// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-098-AC-8 to AC-10 (TC-906): composite and leaf-family arguments
//! replay from an `Input` assignment and from a witness value text, and
//! refuse by kind, by domain and at the request's limits.

use ix_trace_rs::trace;
use qsl_semantics::check::NominalNode;
use quire_exact::{Incomplete, Integer, LimitKind, ValueType};
use quire_semantic_value::call::InputRefusal;

use super::*;
use crate::witness::{QuantityMagnitude, WitnessField, WitnessSlot};

/// A compiled unit under test.
struct Fixture {
    source: String,
    compiled: ComposedUnit,
    /// Byte provision entries beyond the source: the model package.
    provision: Vec<(Option<String>, String, Vec<u8>)>,
}

fn fixture(body: &str) -> Fixture {
    let source = format!("language \"ix:native\" edition \"1-draft\";\n{PROFILE}{body}");
    let compiled = spine(&source, &BTreeMap::new());
    Fixture {
        source,
        compiled,
        provision: Vec::new(),
    }
}

/// A fixture compiled under the raised S1 and S3 limits a very large source
/// needs, as the proving run would have run it.
fn fixture_with_raised_limits(body: &str) -> Fixture {
    let source = format!("language \"ix:native\" edition \"1-draft\";\n{PROFILE}{body}");
    let raised = SpineLimits {
        source: qsl_cst::Limits {
            source_bytes: 1 << 28,
            tokens: 1 << 28,
            nodes: 1 << 28,
            work_units: 1 << 20,
        },
        checking: SpineLimits::default()
            .checking
            .with_nodes(1 << 40)
            .with_input_bytes(1 << 40)
            .with_work_budget(1 << 40),
        ..SpineLimits::default()
    };
    let compiled = compose(
        SourceIdentity::new(AUTHORITY, IDENTITY, NAMESPACE, REVISION),
        IDENTITY,
        source.as_bytes(),
        &BTreeMap::new(),
        &crate::spine::DependencyInput::default(),
        raised,
    )
    .expect("the unit compiles under the raised limits");
    Fixture {
        source,
        compiled,
        provision: Vec::new(),
    }
}

/// A fixture over the `acme/orders` model package (`M::Widget`,
/// `M::Rock`), which the request's byte provision carries.
fn model_fixture(body: &str) -> Fixture {
    let document = std::fs::read(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../tests/fixtures/spine-model.semantic-ir.json"
    ))
    .unwrap();
    let digest = PackageDocument::parse(&document, qsl_foundation::IntakeLimits::default())
        .unwrap()
        .jcs_digest();
    let hex = qsl_semantics::model::key::hex(&digest);
    let source = format!(
        "language \"ix:native\" edition \"1-draft\";\n{PROFILE}\
         model M = \"acme/orders\" version \"1.0.0\" digest \"sha256-jcs:{hex}\";\n{body}"
    );
    let packages = package_input([document.as_slice()]);
    let compiled = spine(&source, &packages);
    let record = DigestRecord::mint(DigestDomain::Sha256Jcs, digest);
    Fixture {
        source,
        compiled,
        provision: vec![(
            Some(DigestDomain::Sha256Jcs.as_str().to_owned()),
            record.hex(),
            document,
        )],
    }
}

impl Fixture {
    fn parameter(&self, function: &str) -> WireNodeId {
        parameter(&self.compiled, function, 0)
    }

    /// The declared type of `function`'s one parameter.
    fn declared(&self, function: &str) -> ValueType {
        let graph = self.compiled.package.graph();
        let callable = graph.callable(function).expect("declared");
        callable.parameters[0].1.clone()
    }

    fn record_id(&self, name: &str) -> WireNodeId {
        let types = self.compiled.package.graph().scope().types();
        let record = types
            .composites()
            .find(|declaration| declaration.name() == name)
            .expect("a declared record");
        WireNodeId::from_digest(*record.key().as_bytes())
    }

    fn enum_id(&self, name: &str) -> WireNodeId {
        let graph = self.compiled.package.graph().semantic_graph();
        let node = graph
            .nodes()
            .find(|node| {
                matches!(
                    node.nominal(),
                    Some(NominalNode::EnumDeclaration(preimage))
                        if preimage.qualified_declaration().last().map(String::as_str) == Some(name)
                )
            })
            .expect("a declared enum");
        WireNodeId::from_digest(*node.key().as_bytes())
    }

    fn request(&self, function: &str, source: ReplaySource) -> ReplayRequestWire {
        let mut wire = request(
            self.source.as_bytes(),
            self.compiled.emitted.package_id(),
            name(&[function]),
            source,
        );
        wire.byte_provision.extend(self.provision.iter().cloned());
        wire
    }

    fn input(&self, function: &str, value: WitnessValue) -> ReplayRequestWire {
        self.request(function, typed_input(self.parameter(function), value))
    }

    fn witness(&self, function: &str, value: &WitnessValue) -> ReplayRequestWire {
        let parameter = self.parameter(function);
        let text = value.to_value_text().unwrap();
        let transcript = format!("<<<assertion|{function}|c|{parameter}={text}>>>");
        self.request(
            function,
            ReplaySource::Witness(Witness::parse(transcript).unwrap()),
        )
    }

    /// `value` replayed on both arms.
    fn both(
        &self,
        function: &str,
        value: WitnessValue,
    ) -> [Result<ReplayResult, ReplayRefusal>; 2] {
        [
            replay(self.witness(function, &value), ReplayLimits::default()),
            replay(self.input(function, value), ReplayLimits::default()),
        ]
    }
}

fn int(value: i64) -> WitnessValue {
    WitnessValue::ExactInteger(Integer::from(value))
}

fn field(name: &str, slot: WitnessSlot) -> WitnessField {
    WitnessField {
        name: name.to_owned(),
        slot,
    }
}

fn present(name: &str, value: WitnessValue) -> WitnessField {
    field(name, WitnessSlot::Present(value))
}

/// An agreeing replay: the call's verdict is the refuted one.
fn assert_reproduced(result: Result<ReplayResult, ReplayRefusal>) -> ReplayResult {
    let result = result.expect("the replay runs");
    match &result {
        ReplayResult::Witness(arm) => assert_eq!(
            arm.settlement(),
            WitnessSettlement::ReproducedWithEvaluatedWitness
        ),
        ReplayResult::Input(arm) => {
            assert_eq!(arm.settlement(), InputSettlement::ReproducedWithoutWitness);
        }
    }
    assert_eq!(result.category(), Category::Violation);
    result
}

fn assert_wrong_kind(result: Result<ReplayResult, ReplayRefusal>, what: &str) {
    assert!(
        matches!(
            result,
            Err(ReplayRefusal::Input(InputRefusal::WrongValueKind {
                parameter: 0
            }))
        ),
        "{what}: expected WrongValueKind at position 0, got {result:?}"
    );
}

const NESTED: &str = "record Inner { a: Int[0, 9]; b: Boolean?; }\n\
     record Outer { i: Inner; o: Option<Int[0, 9]>; s: Sequence<Int[0, 9]>[0, 3]; }\n\
     function p using v(x: Outer): Boolean pure { x.i.a < 5 }\n";

/// The AC-8 counterexample: `a` is 7, so `p` is false.
fn outer(fixture: &Fixture, a: WitnessValue, s: Vec<WitnessValue>) -> WitnessValue {
    WitnessValue::Record {
        declaration: fixture.record_id("Outer"),
        fields: vec![
            present(
                "i",
                WitnessValue::Record {
                    declaration: fixture.record_id("Inner"),
                    fields: vec![present("a", a), present("b", WitnessValue::Boolean(true))],
                },
            ),
            present("o", WitnessValue::Option(Some(Box::new(int(5))))),
            present("s", WitnessValue::Sequence(s)),
        ],
    }
}

/// FR-098-AC-8 (TC-906): a predicate over nested composite values replays
/// its counterexample from an `Input` assignment and from a `Witness` entry,
/// and each settles as AC-2 states, with the replayed call evaluating the
/// nested values (a smaller `a` makes the predicate hold, so the replay
/// disagrees instead).
#[trace("TC-906", "FR-098-AC-8")]
#[test]
fn tc_906_a_nested_composite_counterexample_replays_on_both_arms() {
    let fixture = fixture(NESTED);
    for result in fixture.both("p", outer(&fixture, int(7), vec![int(1), int(2)])) {
        let result = assert_reproduced(result);
        assert!(result.charges().work_units > 0);
    }
    for result in fixture.both("p", outer(&fixture, int(3), vec![int(1), int(2)])) {
        let result = result.expect("the replay runs");
        assert_eq!(result.category(), Category::Success);
        assert!(matches!(
            result.disagreement(),
            Some(crate::result::DisagreementCause::Verdicts { .. })
        ));
    }
}

/// FR-098-AC-8 (TC-906): each malformed argument refuses `WrongValueKind`
/// naming position 0, before the call, on both arms.
#[trace("TC-906", "FR-098-AC-8")]
#[test]
fn tc_906_a_value_the_declared_type_does_not_admit_refuses_by_kind() {
    let fixture = fixture(NESTED);
    let (inner, outer_id) = (fixture.record_id("Inner"), fixture.record_id("Outer"));
    let good = |a: WitnessValue| vec![present("a", a), present("b", WitnessValue::Boolean(true))];
    let with_inner = |inner_value: WitnessValue| WitnessValue::Record {
        declaration: outer_id,
        fields: vec![
            present("i", inner_value),
            present("o", WitnessValue::Option(None)),
            present("s", WitnessValue::Sequence(vec![])),
        ],
    };
    let inner_record = |fields: Vec<WitnessField>| WitnessValue::Record {
        declaration: inner,
        fields,
    };
    let cases: Vec<(&str, WitnessValue)> = vec![
        (
            "a sequence given for the option",
            WitnessValue::Record {
                declaration: outer_id,
                fields: vec![
                    present("i", inner_record(good(int(7)))),
                    present("o", WitnessValue::Sequence(vec![])),
                    present("s", WitnessValue::Sequence(vec![])),
                ],
            },
        ),
        (
            "an Inner named for Outer's declaration",
            with_inner(WitnessValue::Record {
                declaration: outer_id,
                fields: good(int(7)),
            }),
        ),
        (
            "a record missing a",
            with_inner(inner_record(vec![present(
                "b",
                WitnessValue::Boolean(true),
            )])),
        ),
        (
            "a record with an undeclared field",
            with_inner(inner_record(vec![
                present("a", int(7)),
                present("b", WitnessValue::Boolean(true)),
                present("c", int(1)),
            ])),
        ),
        (
            "an absent required field",
            with_inner(inner_record(vec![
                field("a", WitnessSlot::Absent),
                present("b", WitnessValue::Boolean(true)),
            ])),
        ),
        (
            "a null slot for a required field",
            with_inner(inner_record(vec![
                field("a", WitnessSlot::Null),
                present("b", WitnessValue::Boolean(true)),
            ])),
        ),
        (
            "a leaf outside its range",
            with_inner(inner_record(good(int(12)))),
        ),
        (
            "four elements for a three-element sequence",
            outer(&fixture, int(7), vec![int(1), int(2), int(3), int(4)]),
        ),
        (
            "an element outside the element range",
            outer(&fixture, int(7), vec![int(10)]),
        ),
    ];
    for (what, value) in cases {
        for result in fixture.both("p", value.clone()) {
            assert_wrong_kind(result, what);
        }
    }
}

/// FR-098-AC-8 (TC-906): a `set` holding `1.0` and `1.00`, which have
/// distinct value texts and are numerically equal, refuses `WrongValueKind`.
#[trace("TC-906", "FR-098-AC-8")]
#[test]
fn tc_906_a_set_holding_numerically_equal_decimals_refuses() {
    let fixture = fixture(
        "function g using v(v: Set<Decimal[0, 100; 0, 2; exact]>[0, 3]): Boolean pure { false }\n",
    );
    let decimal = |coefficient: i64, scale: i64| WitnessValue::Decimal {
        coefficient: Integer::from(coefficient),
        scale: Integer::from(scale),
    };
    let texts = |set: WitnessValue| set.to_value_text().unwrap();
    let duplicated = WitnessValue::Set(vec![decimal(10, 1), decimal(100, 2)]);
    assert_ne!(
        texts(WitnessValue::Set(vec![decimal(10, 1)])),
        texts(WitnessValue::Set(vec![decimal(100, 2)])),
        "distinct value texts"
    );
    for result in fixture.both("g", duplicated) {
        assert_wrong_kind(result, "equal decimals in a set");
    }
    // Distinct numeric values replay.
    let distinct = WitnessValue::Set(vec![decimal(10, 1), decimal(25, 1)]);
    for result in fixture.both("g", distinct) {
        let result = result.expect("the replay runs");
        assert_eq!(result.category(), Category::Violation);
    }
}

/// The smallest `value_occurrences` limit under which `value`'s replay makes
/// its call, and the result it settles at one below.
fn occurrence_boundary(fixture: &Fixture, value: &WitnessValue) -> (u64, ReplayResult) {
    let mut below = None;
    for limit in 0..200_u64 {
        let mut wire = fixture.input("p", value.clone());
        wire.accounting_limits.value_occurrences = limit;
        let result = replay(wire, ReplayLimits::default()).expect("the replay runs");
        let pre_call = result.limit().is_some_and(|incomplete| {
            incomplete.limit_kind == LimitKind::ValueOccurrences && incomplete.consumed == 0
        });
        if !pre_call {
            return (limit, below.expect("the limit starts too small"));
        }
        below = Some(result);
    }
    panic!("no limit under 200 admits the value");
}

/// FR-098-AC-9 (TC-906): a request whose `value_occurrences` is one below
/// the argument's occurrence count makes no call and settles `inconclusive`
/// with cause `NoValue`, carrying the outcome `Incomplete` that names the
/// counter, its configured value and the count reached; raised to fit, the
/// replay runs and its charges equal those of an unlimited request.
#[trace("TC-906", "FR-098-AC-9")]
#[test]
fn tc_906_the_occurrence_limit_stops_the_replay_before_the_call() {
    let fixture = fixture(NESTED);
    let value = outer(&fixture, int(7), vec![int(1), int(2)]);
    let (fits, below) = occurrence_boundary(&fixture, &value);
    assert!(fits > 1);
    let Some(Incomplete {
        limit_kind,
        limit,
        next_charge,
        ..
    }) = below.limit().cloned()
    else {
        panic!("the replay below the count carries its Incomplete");
    };
    assert_eq!(limit_kind, LimitKind::ValueOccurrences);
    assert_eq!(limit, fits - 1);
    assert_eq!(next_charge, Integer::from(fits), "the count reached");
    assert_eq!(below.category(), Category::Incomplete);
    assert!(matches!(
        below.disagreement(),
        Some(crate::result::DisagreementCause::NoValue { .. })
    ));
    assert_eq!(below.charges().work_units, 0, "no call, no charge");

    let mut raised = fixture.input("p", value.clone());
    raised.accounting_limits.value_occurrences = fits;
    let raised = assert_reproduced(replay(raised, ReplayLimits::default()));
    let unlimited = assert_reproduced(replay(fixture.input("p", value), ReplayLimits::default()));
    assert_eq!(raised.charges(), unlimited.charges());
}

/// FR-098-AC-9 (TC-906): a request whose `work_units` is below the
/// argument's node count settles the same way, naming `work_units`.
#[trace("TC-906", "FR-098-AC-9")]
#[test]
fn tc_906_the_node_limit_stops_the_replay_before_the_call() {
    let fixture = fixture(NESTED);
    let value = outer(&fixture, int(7), vec![int(1), int(2)]);
    // Outer, i, a, b, o and its payload, s and its two elements.
    let nodes = 9_u64;
    let mut wire = fixture.input("p", value.clone());
    wire.accounting_limits.work_units = nodes - 1;
    let result = replay(wire, ReplayLimits::default()).expect("the replay runs");
    let incomplete = result.limit().expect("the Incomplete outcome");
    assert_eq!(incomplete.limit_kind, LimitKind::WorkUnits);
    assert_eq!(incomplete.limit, nodes - 1);
    assert_eq!(incomplete.next_charge, Integer::from(nodes));
    assert_eq!(result.category(), Category::Incomplete);
    assert!(matches!(
        result.disagreement(),
        Some(crate::result::DisagreementCause::NoValue { .. })
    ));
    assert_eq!(result.charges().work_units, 0);

    let mut raised = fixture.input("p", value.clone());
    raised.accounting_limits.work_units = 10_000;
    let raised = assert_reproduced(replay(raised, ReplayLimits::default()));
    let unlimited = assert_reproduced(replay(fixture.input("p", value), ReplayLimits::default()));
    assert_eq!(raised.charges(), unlimited.charges());
}

/// FR-255-AC-10 (TC-914): a `stage_limits` entry `work_units` is the
/// call's `work_units` whatever the request's accounting limits give it,
/// and every other counter keeps the request's value; for FR-098-AC-9's
/// counterexample the entry one below the node count settles `NoValue`
/// naming `work_units`, and raised to fit the replay runs.
#[trace("TC-914", "FR-255-AC-10")]
#[test]
fn tc_914_a_stage_limits_counter_entry_sets_the_budget() {
    let fixture = fixture(NESTED);
    let value = outer(&fixture, int(7), vec![int(1), int(2)]);
    let nodes = 9_u64;
    let mut wire = fixture.input("p", value.clone());
    wire.accounting_limits.work_units = 10_000;
    wire.stage_limits.insert("work_units".to_owned(), nodes - 1);
    let request = ReplayRequest::decode(wire.clone(), ReplayLimits::default()).expect("decodes");
    assert_eq!(request.accounting_limits().work_units, nodes - 1);
    assert_eq!(
        request.accounting_limits().value_occurrences,
        wire.accounting_limits.value_occurrences
    );
    let result = replay(wire, ReplayLimits::default()).expect("the replay runs");
    let incomplete = result.limit().expect("the Incomplete outcome");
    assert_eq!(incomplete.limit_kind, LimitKind::WorkUnits);
    assert_eq!(incomplete.limit, nodes - 1);
    assert!(matches!(
        result.disagreement(),
        Some(crate::result::DisagreementCause::NoValue { .. })
    ));

    let mut raised = fixture.input("p", value.clone());
    raised.accounting_limits.work_units = 0;
    raised.stage_limits.insert("work_units".to_owned(), 10_000);
    let raised = assert_reproduced(replay(raised, ReplayLimits::default()));
    let unlimited = assert_reproduced(replay(fixture.input("p", value), ReplayLimits::default()));
    assert_eq!(raised.charges(), unlimited.charges());
}

const LEAVES: &str = "enum Color { BLUE, GREEN, RED }\n\
     enum Shade { DARK, LIGHT }\n\
     function ec using v(c: Color): Boolean pure { false }\n\
     function es using v(c: Shade): Boolean pure { false }\n\
     function tx using v(t: Text[0, 8; nfc]): Boolean pure { false }\n\
     function ra using v(r: Rational[0, 9; 1, 9]): Boolean pure { false }\n\
     function dc using v(d: Decimal[0, 1000; 0, 2; exact]): Boolean pure { false }\n\
     function f64 using v(f: Float64): Boolean pure { false }\n\
     function f32 using v(f: Float32): Boolean pure { false }\n";

/// FR-098-AC-10 (TC-906): one predicate per leaf family replays its
/// counterexample from a witness value text and settles as AC-2 states, and
/// the refused forms refuse `WrongValueKind` at position 0.
#[trace("TC-906", "FR-098-AC-10")]
#[test]
fn tc_906_leaf_families_replay_and_refuse_by_kind_and_domain() {
    let fixture = fixture(LEAVES);
    let color = |declaration: WireNodeId, member: &str| WitnessValue::Enum {
        declaration,
        member: member.to_owned(),
    };
    let text = |value: &str| WitnessValue::Text(value.to_owned());
    let rational = |numerator: i64, denominator: i64| WitnessValue::Rational {
        numerator: Integer::from(numerator),
        denominator: Integer::from(denominator),
    };
    let decimal = |coefficient: i64, scale: i64| WitnessValue::Decimal {
        coefficient: Integer::from(coefficient),
        scale: Integer::from(scale),
    };

    let witness = |function: &str, value: WitnessValue| {
        replay(fixture.witness(function, &value), ReplayLimits::default())
    };
    let color_id = fixture.enum_id("Color");
    let shade_id = fixture.enum_id("Shade");
    assert_reproduced(witness("ec", color(color_id, "RED")));
    assert_reproduced(witness("tx", text("a;b>c")));
    assert_reproduced(witness("ra", rational(3, 2)));
    assert_reproduced(witness("dc", decimal(1050, 2)));
    assert_reproduced(witness("f64", WitnessValue::Float64(0x4009_21fb_5444_2d18)));
    assert_reproduced(witness("f32", WitnessValue::Float32(0x7fc0_0001)));

    assert_wrong_kind(
        witness("ec", color(shade_id, "DARK")),
        "another enum's declaration",
    );
    assert_wrong_kind(witness("ec", color(color_id, "PURPLE")), "a non-member");
    assert_wrong_kind(witness("tx", text("nine scalar")), "a nine-scalar text");
    assert_wrong_kind(
        witness("ra", rational(1, 10)),
        "a rational outside its domain",
    );
    assert_wrong_kind(witness("dc", decimal(1, 3)), "a decimal with scale 3");
    assert_wrong_kind(
        witness("dc", decimal(2000, 0)),
        "a decimal outside its range",
    );
    assert_wrong_kind(
        witness("f64", WitnessValue::Float32(0)),
        "a float32 for a Float64",
    );
    assert_wrong_kind(witness("ec", text("RED")), "a text for an enum");
    assert_wrong_kind(witness("ra", decimal(15, 1)), "a decimal for a Rational");
}

/// The converter's one argument as a value of `declared`, over a compiled
/// package whose own declarations are not consulted: a quantity's unit and a
/// reference's object type are identities, compared by their bytes.
fn convert_one(
    declared: &ValueType,
    value: &WitnessValue,
    limits: &ScalarLimits,
) -> Result<quire_exact::Value, Box<ReplayRefusal>> {
    let fixture = fixture("function t using v(b: Boolean): Boolean pure { b }\n");
    let converted = argument::convert_arguments(
        &fixture.compiled.package,
        std::slice::from_ref(declared),
        std::slice::from_ref(value),
        limits,
    );
    match converted {
        Ok(mut converted) => Ok(converted.values.remove(0)),
        Err(argument::Stopped::Refusal(refusal)) => Err(refusal),
        Err(argument::Stopped::Limit(incomplete)) => {
            panic!("an accounting limit stopped the conversion: {incomplete:?}")
        }
    }
}

/// FR-098-AC-10 (TC-906): a quantity converts in its declared unit, and one
/// in another unit refuses `WrongValueKind`. A source cannot yet declare a
/// quantity-typed parameter (the assembler resolves no unit name as a type),
/// so this reaches the conversion directly.
#[trace("TC-906", "FR-098-AC-10")]
#[test]
fn tc_906_a_quantity_converts_in_its_declared_unit_only() {
    let declared_unit = quire_exact::UnitId::declared(quire_exact::NodeKey::from_digest([7; 32]));
    let declared = ValueType::Quantity(declared_unit);
    let quantity = |unit: [u8; 32]| WitnessValue::Quantity {
        magnitude: QuantityMagnitude::Decimal {
            coefficient: Integer::from(15_i64),
            scale: Integer::from(1_i64),
        },
        unit,
    };
    let converted = convert_one(&declared, &quantity([7; 32]), &UNLIMITED).expect("converts");
    let quire_exact::Value::Quantity(converted) = converted else {
        panic!("a quantity");
    };
    assert_eq!(
        converted.magnitude(),
        &quire_exact::Rational::new(Integer::from(3_i64), Integer::from(2_i64)).unwrap(),
        "1.5 converts exactly"
    );
    assert_eq!(converted.unit(), declared_unit);
    let refused = convert_one(&declared, &quantity([8; 32]), &UNLIMITED);
    assert!(
        matches!(
            &refused,
            Err(refusal) if matches!(
                **refusal,
                ReplayRefusal::Input(InputRefusal::WrongValueKind { parameter: 0 })
            )
        ),
        "a quantity in another unit: {refused:?}"
    );
}

/// FR-098: a decimal quantity whose scale would expand past the request's
/// `scale_expansion` limit stops at that limit instead of expanding a power
/// of ten, and settles `Incomplete`.
#[trace("TC-906", "FR-098-AC-9")]
#[test]
fn tc_906_a_decimal_magnitude_scale_stops_at_the_request_limit() {
    let fixture = fixture("function t using v(b: Boolean): Boolean pure { b }\n");
    let declared_unit = quire_exact::UnitId::declared(quire_exact::NodeKey::from_digest([7; 32]));
    let value = WitnessValue::Quantity {
        magnitude: QuantityMagnitude::Decimal {
            coefficient: Integer::from(1_i64),
            scale: Integer::from(1_000_000_000_i64),
        },
        unit: *declared_unit.as_bytes(),
    };
    let limits = ScalarLimits {
        scale_expansion: 64,
        ..UNLIMITED
    };
    let stopped = argument::convert_arguments(
        &fixture.compiled.package,
        &[ValueType::Quantity(declared_unit)],
        &[value],
        &limits,
    );
    let Err(argument::Stopped::Limit(incomplete)) = stopped else {
        panic!("the scale stops at the limit");
    };
    assert_eq!(incomplete.limit_kind, LimitKind::ScaleExpansion);
    assert_eq!(incomplete.limit, 64);
    assert_eq!(incomplete.next_charge, Integer::from(1_000_000_000_i64));
}

/// Run `body` on a thread with a 512 KiB stack.
fn on_small_stack(body: impl FnOnce() + Send + 'static) {
    std::thread::Builder::new()
        .stack_size(512 * 1024)
        .spawn(body)
        .unwrap()
        .join()
        .unwrap();
}

/// `record List { head: Int[0, 9]; tail: List?; }` nested `depth` deep.
fn list(declaration: WireNodeId, depth: usize) -> WitnessValue {
    let mut value = WitnessValue::Record {
        declaration,
        fields: vec![present("head", int(1)), field("tail", WitnessSlot::Absent)],
    };
    for _ in 1..depth {
        value = WitnessValue::Record {
            declaration,
            fields: vec![present("head", int(1)), present("tail", value)],
        };
    }
    value
}

/// FR-263-AC-1 (TC-736 steps 1 and 2): on a thread with a 512 KiB stack, a
/// request whose source holds a function with a 100,000-term sum over a
/// recursive list parameter, whose assignment (and witness entry) carries a
/// 100,000-long recursive list, whose `stage_limits` carries raised `s1.*`
/// and `s3.*` settings, and whose `replay.input_bytes` is raised to fit,
/// replays to the proving run's verdict, with its recompiled `package_id`
/// equal to the request's. Under the default limits it refuses by the first
/// limit it reaches.
#[trace("TC-736", "FR-263-AC-1")]
#[test]
fn tc_736_a_100000_term_source_and_list_replay_to_the_proving_verdict() {
    on_small_stack(|| {
        const TERMS: usize = 100_000;
        let sum = vec!["l.head"; TERMS].join(" + ");
        let fixture = fixture_with_raised_limits(&format!(
            "record List {{ head: Int[0, 9]; tail: List?; }}\n\
             function small using v(l: List): Boolean pure {{ {sum} < 5 }}\n"
        ));
        let value = list(fixture.record_id("List"), TERMS);
        let raised = ReplayLimits::default().with_input_bytes(1 << 28);
        let limits: BTreeMap<String, u64> = BTreeMap::from(
            [
                "s1.input_bytes",
                "s1.tokens",
                "s1.nodes",
                "s1.work_units",
                "s3.nodes",
                "s3.input_bytes",
                "s3.work_units",
            ]
            .map(|name| {
                let bound = if name == "s1.input_bytes" {
                    1_u64 << 27
                } else {
                    1_u64 << 40
                };
                (name.to_owned(), bound)
            }),
        );
        let witness = |fixture: &Fixture, limits: &BTreeMap<String, u64>| {
            let mut wire = fixture.witness("small", &value);
            wire.stage_limits = limits.clone();
            wire
        };
        // Step 2: the default limits stop the recompile at a named stage.
        let refused = replay(witness(&fixture, &BTreeMap::new()), raised);
        assert!(
            matches!(refused, Err(ReplayRefusal::Recompile(_))),
            "{refused:?}"
        );
        // The raised settings replay, from the witness and from the input.
        let wire = witness(&fixture, &limits);
        assert_reproduced(replay(wire, raised));
        let mut input = fixture.input("small", value.clone());
        input.stage_limits = limits;
        assert_reproduced(replay(input, raised));
    });
}

/// FR-263-AC-1 (TC-736), the value half: a 100,000-long recursive list
/// under the default `replay.input_bytes` refuses `BoundExceeded`, and
/// replays once it is raised.
#[trace("TC-736", "FR-263-AC-1")]
#[test]
fn tc_736_a_100000_long_list_replays_under_the_raised_input_bound() {
    on_small_stack(|| {
        let fixture = fixture(
            "record List { head: Int[0, 9]; tail: List?; }\n\
             function big using v(l: List): Boolean pure { l.head > 5 }\n",
        );
        let raised = ReplayLimits::default().with_input_bytes(1 << 28);
        let value = list(fixture.record_id("List"), 100_000);
        let witness = fixture.witness("big", &value);
        assert!(matches!(
            replay(witness.clone(), ReplayLimits::default()),
            Err(ReplayRefusal::Request(ReplayRequestRefusal::BoundExceeded(exceeded)))
                if exceeded.bound == crate::DEFAULT_REPLAY_INPUT_BYTES
        ));
        assert_reproduced(replay(witness, raised));
        let input = fixture.input("big", value);
        assert!(matches!(
            replay(input.clone(), ReplayLimits::default()),
            Err(ReplayRefusal::Request(ReplayRequestRefusal::BoundExceeded(
                _
            )))
        ));
        assert_reproduced(replay(input, raised));
    });
}

/// FR-098-AC-10 (TC-906): a reference replays over the replay's object
/// environment, which holds no object. A predicate that tests the reference
/// for equality with itself replays; one that dereferences it completes no
/// value and settles `inconclusive` with cause `NoValue`; a reference whose
/// `object_type` is another type's, or whose identity is no authored
/// identity, refuses `WrongValueKind`.
#[trace("TC-906", "FR-098-AC-10")]
#[test]
fn tc_906_a_reference_replays_by_identity_and_a_dereference_completes_no_value() {
    let fixture = model_fixture(
        "function rw using v(w: Reference<M::Widget>): Boolean pure { not (w = w) }\n\
         function rr using v(w: Reference<M::Rock>): Boolean pure { false }\n\
         function rd using v(w: Reference<M::Widget>): Boolean pure { deref(w).code > 0 }\n",
    );
    let type_of = |function: &str| {
        let ValueType::Reference(object_type) = fixture.declared(function) else {
            panic!("{function} takes a reference");
        };
        *object_type.as_bytes()
    };
    let reference = |object_type: [u8; 32], identity: &[u8]| WitnessValue::Reference {
        universe: [1; 32],
        object_type,
        identity: identity.to_vec(),
    };
    for result in fixture.both("rw", reference(type_of("rw"), b"w-1")) {
        assert_reproduced(result);
    }
    for result in fixture.both("rd", reference(type_of("rw"), b"w-1")) {
        let result = result.expect("the replay runs");
        assert_eq!(result.category(), Category::Inconclusive);
        assert!(matches!(
            result.disagreement(),
            Some(crate::result::DisagreementCause::NoValue { .. })
        ));
    }
    assert_wrong_kind(
        replay(
            fixture.witness("rw", &reference(type_of("rr"), b"w-1")),
            ReplayLimits::default(),
        ),
        "a reference to another object type",
    );
    assert_wrong_kind(
        replay(
            fixture.witness("rw", &reference(type_of("rw"), &[0xff, 0xfe])),
            ReplayLimits::default(),
        ),
        "an identity that is not UTF-8",
    );
    assert_wrong_kind(
        replay(
            fixture.witness("rw", &reference(type_of("rw"), b"")),
            ReplayLimits::default(),
        ),
        "an empty identity",
    );
}
