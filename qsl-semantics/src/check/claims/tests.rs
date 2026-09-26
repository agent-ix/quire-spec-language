// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-160 step 10: the requirement records of FR-062's fixture units, read
//! from real complete-V1 source through S1, S2 and `check`, and the keying
//! faults of [`key_claims`] over hand-built occurrences.

use ix_trace_rs::trace;
use qsl_forms::{build_unit, FormsLimits};
use qsl_foundation::SourceIdentity;
use quire_exact::{Integer, RationalDomain, Role};

use super::*;
use crate::check::node_key::NodeTag;
use crate::check::{CheckedGraph, CheckingLimits, PackageDeclarations};

const HEADER: &str = "language \"ix:native\" edition \"1-draft\";\n\
    profile v = \"quire.value.complete/v1\" version \"1\" digest \
    \"sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\";\n";

/// A unit checked through S1, S2 and `check`, with its text.
struct Checked {
    text: String,
    graph: CheckedGraph,
}

fn check(declarations: &str) -> Checked {
    let text = format!("{HEADER}{declarations}\n");
    let parsed = qsl_cst::parse(
        SourceIdentity::new("a", "u", "git", "1"),
        "unit.native",
        text.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let unit = build_unit(&parsed, FormsLimits::default()).expect("S2 builds the unit");
    let graph = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        unit,
        Vec::new(),
        Vec::new(),
    )
    .expect("the unit assembles")
    .check(CheckingLimits::default())
    .unwrap_or_else(|refusals| panic!("{declarations}: {refusals:?}"));
    Checked { text, graph }
}

/// Where a result bound's type node is found, independently of the record.
#[derive(Clone, Copy)]
enum TypeAt {
    /// The declared result type of the named function.
    Result(&'static str),
    /// The declared type of the named function's parameter at this index.
    Parameter(&'static str, usize),
    /// The builtin scalar type node of this form.
    Scalar(&'static str),
}

/// One expected record: its application's operation identity, its site as
/// the `nth` occurrence of `site` in the unit text, its ordinal, its
/// unbounded `Integer` roots as (function, parameter index), its result
/// bound and its guards as (text, nth, required outcome).
struct Expect {
    operation: &'static str,
    site: (&'static str, usize),
    ordinal: u64,
    roots: &'static [(&'static str, usize)],
    bound: (ValueType, TypeAt),
    guards: &'static [(&'static str, usize, bool)],
}

impl Checked {
    /// The byte range of the `nth` occurrence of `needle` in the unit.
    fn span(&self, needle: &str, nth: usize) -> (u64, u64) {
        let (start, _) = self
            .text
            .match_indices(needle)
            .nth(nth)
            .unwrap_or_else(|| panic!("`{needle}` #{nth} is in the unit"));
        let start = u64::try_from(start).unwrap();
        (start, start + u64::try_from(needle.len()).unwrap())
    }

    /// Every `expression` occurrence whose location's region is `span`.
    fn expressions_at(&self, span: (u64, u64)) -> Vec<(NodeKey, Origin)> {
        self.graph
            .occurrences()
            .filter(|(_, origin, location)| {
                origin.role() == &Role::new("expression")
                    && self
                        .graph
                        .region(location)
                        .is_some_and(|region| (region.start(), region.end()) == span)
            })
            .map(|(node, origin, _)| (node, origin))
            .collect()
    }

    /// The one `expression` occurrence at the `nth` `needle` whose node
    /// applies `operation`, or, with no operation, the one occurrence there.
    fn occurrence(&self, needle: &str, nth: usize, operation: Option<&str>) -> OccurrenceKey {
        let found: Vec<(NodeKey, Origin)> = self
            .expressions_at(self.span(needle, nth))
            .into_iter()
            .filter(|(node, _)| {
                operation.is_none_or(|operation| {
                    matches!(
                        self.graph.semantic_graph().node(*node).map(|node| node.body()),
                        Some(SemanticTerm::Application { operation: applied, .. })
                            if applied.identity() == operation
                    )
                })
            })
            .collect();
        let [(node, origin)] = found.as_slice() else {
            panic!("one occurrence of {operation:?} at `{needle}` #{nth}: {found:?}");
        };
        OccurrenceKey::new(wire(*node), origin.clone())
    }

    fn function(&self, name: &str) -> &crate::check::SemanticNode {
        let key = self
            .graph
            .function_identity(name)
            .expect("the function is declared");
        self.graph
            .semantic_graph()
            .node(key)
            .expect("its node is lowered")
    }

    fn parameter(&self, function: &str, index: usize) -> NodeKey {
        self.function(function)
            .function_parameters()
            .expect("a function node")[index]
    }

    fn type_node(&self, at: TypeAt) -> WireNodeId {
        let key = match at {
            TypeAt::Result(function) => self.function(function).semantic_type(),
            TypeAt::Parameter(function, index) => self
                .graph
                .semantic_graph()
                .node(self.parameter(function, index))
                .and_then(|node| node.semantic_type()),
            TypeAt::Scalar(form) => self
                .graph
                .semantic_graph()
                .nodes()
                .find(|node| node.node_tag() == NodeTag::ScalarType && node.semantic_form() == form)
                .map(|node| node.key()),
        };
        wire(key.expect("the type node is lowered"))
    }

    /// Assert the package holds exactly the `expected` records.
    fn assert_records(&self, unit: &str, expected: &[Expect]) {
        let records = self.graph.requirements();
        assert_eq!(records.len(), expected.len(), "{unit}: {records:#?}");
        for expect in expected {
            let (needle, nth) = expect.site;
            let key = self.occurrence(needle, nth, Some(expect.operation));
            let context = format!("{unit}: {} at `{needle}` #{nth}", expect.operation);
            assert_eq!(key.origin().role().as_str(), "expression", "{context}");
            assert_eq!(key.origin().ordinal(), expect.ordinal, "{context}");
            let record = records
                .get(&key)
                .unwrap_or_else(|| panic!("{context}: no record in {records:#?}"));
            assert_eq!(
                record.requirements().kind(),
                Capability::ValueValidity,
                "{context}"
            );
            let domains = expect
                .roots
                .iter()
                .map(|(function, index)| {
                    (
                        DomainKey::new(wire(self.parameter(function, *index)), Vec::new()),
                        DomainKind::Integer,
                    )
                })
                .collect();
            assert_eq!(
                record.requirements().extent(),
                &ClaimExtent::from_domains(domains),
                "{context}"
            );
            let (bound, at) = &expect.bound;
            assert_eq!(record.result_bound().value_type(), bound, "{context}");
            assert_eq!(
                record.result_bound().node(),
                self.type_node(*at),
                "{context}"
            );
            let guards: Vec<(OccurrenceKey, bool)> = expect
                .guards
                .iter()
                .map(|(needle, nth, holds)| (self.occurrence(needle, *nth, None), *holds))
                .collect();
            let recorded: Vec<(OccurrenceKey, bool)> = record
                .path_condition()
                .iter()
                .map(|guard| (guard.occurrence().clone(), guard.holds()))
                .collect();
            assert_eq!(recorded, guards, "{context}");
        }
    }
}

fn interval(lower: i64, upper: i64) -> IntegerInterval {
    IntegerInterval::new(Integer::from(lower), Integer::from(upper)).unwrap()
}

fn int(lower: i64, upper: i64) -> ValueType {
    ValueType::Int(interval(lower, upper))
}

const ADD: &str = "quire.op.integer.add";
const MUL: &str = "quire.op.integer.mul";

/// FR-062's fixtures RR-1 to RR-13 and RR-15 to RR-17, each with exactly
/// the records the fixture table lists.
fn fixtures() -> Vec<(&'static str, Vec<Expect>)> {
    vec![
        (
            "function neg using v(z: Int[0, 9]): Int[-9, 0] pure { -z }",
            vec![Expect {
                operation: "quire.op.integer.negate",
                site: ("-z", 0),
                ordinal: 0,
                roots: &[],
                bound: (int(-9, 0), TypeAt::Result("neg")),
                guards: &[],
            }],
        ),
        (
            "function inc using v(x: Int[0, 9]): Int[0, 10] pure { x + 1 }",
            vec![Expect {
                operation: ADD,
                site: ("x + 1", 0),
                ordinal: 0,
                roots: &[],
                bound: (int(0, 10), TypeAt::Result("inc")),
                guards: &[],
            }],
        ),
        (
            "function add using v(x: Int[0, 9], y: Int[0, 9]): Int[0, 18] pure { x + y }",
            vec![Expect {
                operation: ADD,
                site: ("x + y", 0),
                ordinal: 0,
                roots: &[],
                bound: (int(0, 18), TypeAt::Result("add")),
                guards: &[],
            }],
        ),
        (
            "function eq using v(x: Int[0, 9], y: Int[0, 9]): Boolean pure { x = y }",
            vec![Expect {
                operation: "quire.op.integer.eq",
                site: ("x = y", 0),
                ordinal: 0,
                roots: &[],
                bound: (ValueType::Boolean, TypeAt::Result("eq")),
                guards: &[],
            }],
        ),
        (
            "function sq using v(x: Int[0, 9]): Integer pure { (x + 1) * (x + 1) }",
            vec![
                Expect {
                    operation: ADD,
                    site: ("x + 1", 0),
                    ordinal: 0,
                    roots: &[],
                    bound: (ValueType::Integer, TypeAt::Result("sq")),
                    guards: &[],
                },
                Expect {
                    operation: ADD,
                    site: ("x + 1", 1),
                    ordinal: 1,
                    roots: &[],
                    bound: (ValueType::Integer, TypeAt::Result("sq")),
                    guards: &[],
                },
                Expect {
                    operation: MUL,
                    site: ("(x + 1) * (x + 1)", 0),
                    ordinal: 0,
                    roots: &[],
                    bound: (ValueType::Integer, TypeAt::Result("sq")),
                    guards: &[],
                },
            ],
        ),
        (
            "function big using v(n: Integer): Integer pure { n + 1 }",
            vec![Expect {
                operation: ADD,
                site: ("n + 1", 0),
                ordinal: 0,
                roots: &[("big", 0)],
                bound: (ValueType::Integer, TypeAt::Result("big")),
                guards: &[],
            }],
        ),
        (
            "function lt using v(x: Int[0, 9]): Integer pure { let t = x + 1 in t * 2 }",
            vec![
                Expect {
                    operation: ADD,
                    site: ("x + 1", 0),
                    ordinal: 0,
                    roots: &[],
                    bound: (ValueType::Integer, TypeAt::Result("lt")),
                    guards: &[],
                },
                Expect {
                    operation: MUL,
                    site: ("t * 2", 0),
                    ordinal: 0,
                    roots: &[],
                    bound: (ValueType::Integer, TypeAt::Result("lt")),
                    guards: &[],
                },
            ],
        ),
        (
            "function two using v(): Integer pure { 1 + 1 }",
            vec![Expect {
                operation: ADD,
                site: ("1 + 1", 0),
                ordinal: 0,
                roots: &[],
                bound: (ValueType::Integer, TypeAt::Result("two")),
                guards: &[],
            }],
        ),
        (
            "function both using v(b: Boolean, c: Boolean): Boolean pure { b and c }",
            Vec::new(),
        ),
        ("function c2 using v(): Int[0, 9] pure { 3 }", Vec::new()),
        (
            "function clamp using v(n: Integer): Int[0, 10] pure { if n >= 0 and n <= 10 then n else 0 }",
            vec![
                Expect {
                    operation: "quire.op.integer.ge",
                    site: ("n >= 0", 0),
                    ordinal: 0,
                    roots: &[("clamp", 0)],
                    bound: (ValueType::Boolean, TypeAt::Scalar("boolean")),
                    guards: &[],
                },
                Expect {
                    operation: "quire.op.integer.le",
                    site: ("n <= 10", 0),
                    ordinal: 0,
                    roots: &[("clamp", 0)],
                    bound: (ValueType::Boolean, TypeAt::Scalar("boolean")),
                    guards: &[("n >= 0", 0, true)],
                },
            ],
        ),
        (
            "function g using v(p: Int[0, 10]): Boolean pure { true }\n\
             function f using v(n: Integer): Boolean pure { if n >= 0 and n < 10 then g(n + 1) else true }",
            vec![
                Expect {
                    operation: "quire.op.integer.ge",
                    site: ("n >= 0", 0),
                    ordinal: 0,
                    roots: &[("f", 0)],
                    bound: (ValueType::Boolean, TypeAt::Result("f")),
                    guards: &[],
                },
                Expect {
                    operation: "quire.op.integer.lt",
                    site: ("n < 10", 0),
                    ordinal: 0,
                    roots: &[("f", 0)],
                    bound: (ValueType::Boolean, TypeAt::Result("f")),
                    guards: &[("n >= 0", 0, true)],
                },
                Expect {
                    operation: ADD,
                    site: ("n + 1", 0),
                    ordinal: 0,
                    roots: &[("f", 0)],
                    bound: (int(0, 10), TypeAt::Parameter("g", 0)),
                    guards: &[("n >= 0 and n < 10", 0, true)],
                },
            ],
        ),
        (
            "function q using v(x: Int[0, 9], y: Int[-9, 9]): Rational[-9, 9; 1, 9] pure { if y != 0 then x / y else rational(0, 1) }",
            vec![
                Expect {
                    operation: "quire.op.integer.ne",
                    site: ("y != 0", 0),
                    ordinal: 0,
                    roots: &[],
                    bound: (ValueType::Boolean, TypeAt::Scalar("boolean")),
                    guards: &[],
                },
                Expect {
                    operation: "quire.op.rational.div",
                    site: ("x / y", 0),
                    ordinal: 0,
                    roots: &[],
                    bound: (
                        ValueType::Rational(
                            RationalDomain::new(interval(-9, 9), interval(1, 9)).unwrap(),
                        ),
                        TypeAt::Result("q"),
                    ),
                    guards: &[("y != 0", 0, true)],
                },
            ],
        ),
        (
            "function sib using v(x: Int[0, 9], n: Integer): Integer pure { (let t = x + 1 in t * 2) + (let t = n + 1 in t * 2) }",
            sibling_records(0, 1),
        ),
        (
            "function g using v(p: Int[0, 10]): Boolean pure { true }\n\
             function h using v(q: Int[0, 20]): Boolean pure { true }\n\
             function f using v(x: Int[0, 9]): Boolean pure { g(x + 1) and h(x + 1) }",
            vec![
                Expect {
                    operation: ADD,
                    site: ("x + 1", 0),
                    ordinal: 0,
                    roots: &[],
                    bound: (int(0, 10), TypeAt::Parameter("g", 0)),
                    guards: &[],
                },
                Expect {
                    operation: ADD,
                    site: ("x + 1", 1),
                    ordinal: 1,
                    roots: &[],
                    bound: (int(0, 20), TypeAt::Parameter("h", 0)),
                    guards: &[("g(x + 1)", 0, true)],
                },
            ],
        ),
        (
            // The measure's `x + 1` is the first in the text, the body's the
            // second.
            "function m using v(x: Int[0, 9]): Integer pure decreases(x + 1) { x + 1 }",
            vec![Expect {
                operation: ADD,
                site: ("x + 1", 1),
                ordinal: 0,
                roots: &[],
                bound: (ValueType::Integer, TypeAt::Result("m")),
                guards: &[],
            }],
        ),
    ]
}

/// RR-15's five records, with the `let` binding `x + 1` at operand
/// position `bounded` and the one binding `n + 1` at `unbounded`: each
/// `t * 2` carries its own binding's extent at its own occurrence.
fn sibling_records(bounded: usize, unbounded: usize) -> Vec<Expect> {
    let integer = || (ValueType::Integer, TypeAt::Result("sib"));
    let whole = if bounded == 0 {
        "(let t = x + 1 in t * 2) + (let t = n + 1 in t * 2)"
    } else {
        "(let t = n + 1 in t * 2) + (let t = x + 1 in t * 2)"
    };
    vec![
        Expect {
            operation: ADD,
            site: ("x + 1", 0),
            ordinal: 0,
            roots: &[],
            bound: integer(),
            guards: &[],
        },
        Expect {
            operation: ADD,
            site: ("n + 1", 0),
            ordinal: 0,
            roots: &[("sib", 1)],
            bound: integer(),
            guards: &[],
        },
        Expect {
            operation: MUL,
            site: ("t * 2", bounded),
            ordinal: u64::try_from(bounded).unwrap(),
            roots: &[],
            bound: integer(),
            guards: &[],
        },
        Expect {
            operation: MUL,
            site: ("t * 2", unbounded),
            ordinal: u64::try_from(unbounded).unwrap(),
            roots: &[("sib", 1)],
            bound: integer(),
            guards: &[],
        },
        Expect {
            operation: ADD,
            site: (whole, 0),
            ordinal: 0,
            roots: &[("sib", 1)],
            bound: integer(),
            guards: &[],
        },
    ]
}

/// TC-160 step 10 (FR-062-AC-13; FR-057-AC-10's value-function rows):
/// each fixture unit's map holds exactly the records FR-062's table lists,
/// each `value-validity`, keyed by its application's `expression`
/// occurrence at its own site, with its own extent, result bound (as its
/// type node) and path condition. A narrow and a narrowed literal or
/// parameter carry none; a measure's application carries none.
#[trace("TC-160", "FR-062-AC-13", "FR-057-AC-10")]
#[test]
fn tc_160_each_fixture_unit_holds_exactly_its_requirement_records() {
    for (unit, expected) in fixtures() {
        check(unit).assert_records(unit, &expected);
    }
}

/// TC-160 step 10 (FR-062-AC-13): RR-15 with its two `let` operands
/// swapped. Each `t * 2` still carries its own binding's extent, now at
/// the other ordinal: the extent follows the site, not the order.
#[trace("TC-160", "FR-062-AC-13")]
#[test]
fn tc_160_sibling_lets_keep_their_extents_in_both_orders() {
    let unit = "function sib using v(x: Int[0, 9], n: Integer): Integer pure { (let t = n + 1 in t * 2) + (let t = x + 1 in t * 2) }";
    check(unit).assert_records(unit, &sibling_records(1, 0));
}

/// TC-160 step 10 (FR-062-AC-13): checking RR-5 twice gives equal maps.
#[trace("TC-160", "FR-062-AC-13")]
#[test]
fn tc_160_checking_a_unit_twice_gives_equal_records() {
    let unit = "function sq using v(x: Int[0, 9]): Integer pure { (x + 1) * (x + 1) }";
    assert_eq!(
        check(unit).graph.requirements(),
        check(unit).graph.requirements()
    );
}

fn body(index: usize, path: &[usize]) -> Location {
    Location {
        origin: CheckOrigin::Body {
            function: "f".into(),
            index,
        },
        path: path.to_vec(),
    }
}

fn claim_at(location: Location) -> ValueClaim {
    ValueClaim {
        site: ClaimSite {
            location,
            bound: SiteBound::Own(ValueType::Integer),
            path_condition: Vec::new(),
        },
        domains: BTreeMap::new(),
    }
}

/// TC-160 step 11 (FR-062-AC-13): over RR-8's lowered graph and
/// hand-built occurrence maps, a site pairs only with a scalar application
/// occurrence at its own location. A site whose application has only a
/// `generated` occurrence, a site at a location holding none, and a
/// scalar application occurrence in a body that no claim names are each
/// `KeyFault::UnkeyableRequirements`, never an omission.
#[trace("TC-160", "FR-062-AC-13")]
#[test]
fn tc_160_an_unpaired_site_or_application_faults_instead_of_dropping() {
    let checked = check("function two using v(): Integer pure { 1 + 1 }");
    let graph = checked.graph.semantic_graph();
    let key = checked.occurrence("1 + 1", 0, Some(ADD));
    let add = NodeKey::from_digest(*key.node().as_bytes());
    let site = checked
        .graph
        .occurrence(add, key.origin())
        .expect("the application's occurrence")
        .clone();
    let binders = BTreeMap::new();

    let mut paired = OccurrenceMap::default();
    paired.record(add, "expression", site.clone());
    let records = key_claims(vec![claim_at(site.clone())], &paired, graph, &binders)
        .expect("the site pairs with its occurrence");
    assert_eq!(records.keys().collect::<Vec<_>>(), [&key]);

    let mut generated = OccurrenceMap::default();
    generated.record(add, "generated", site.clone());
    assert_eq!(
        key_claims(vec![claim_at(site.clone())], &generated, graph, &binders),
        Err(KeyFault::UnkeyableRequirements)
    );

    let elsewhere = body(0, &[9]);
    assert_eq!(
        key_claims(vec![claim_at(elsewhere)], &paired, graph, &binders),
        Err(KeyFault::UnkeyableRequirements)
    );

    assert_eq!(
        key_claims(Vec::new(), &paired, graph, &binders),
        Err(KeyFault::UnkeyableRequirements)
    );
}

/// TC-160 step 11 (FR-062-AC-13): two sites of one application node, at
/// two locations, pair with that node's two occurrences by location: the
/// keys differ only in ordinal, whichever order the claims arrive in.
#[trace("TC-160", "FR-062-AC-13")]
#[test]
fn tc_160_two_sites_of_one_node_pair_by_location_not_order() {
    let checked = check("function two using v(): Integer pure { 1 + 1 }");
    let graph = checked.graph.semantic_graph();
    let add = NodeKey::from_digest(*checked.occurrence("1 + 1", 0, Some(ADD)).node().as_bytes());
    let (first, second) = (body(0, &[0]), body(0, &[1]));
    let mut occurrences = OccurrenceMap::default();
    let first_origin = occurrences.record(add, "expression", first.clone());
    let second_origin = occurrences.record(add, "expression", second.clone());
    let binders = BTreeMap::new();
    for claims in [
        vec![claim_at(first.clone()), claim_at(second.clone())],
        vec![claim_at(second.clone()), claim_at(first.clone())],
    ] {
        let records = key_claims(claims, &occurrences, graph, &binders)
            .expect("each site pairs with its own occurrence");
        assert_eq!(
            records.keys().cloned().collect::<Vec<_>>(),
            [
                OccurrenceKey::new(wire(add), first_origin.clone()),
                OccurrenceKey::new(wire(add), second_origin.clone()),
            ]
        );
    }
}

/// The one function of `declarations`, checked through the contract's own
/// `check` with the stage node-count limit `node_count`.
fn family_check(
    declarations: &str,
    node_count: u64,
) -> crate::family::CheckOutcome<crate::check::CheckedDeclaration, crate::check::CheckRefusal> {
    use crate::check::family::fixtures::{check_context, declarations_for, empty_scope, limits};
    use crate::check::family::SCALAR_LIMITS_UNLIMITED;
    use crate::check::{Signature, Signatures, ValueFunctionFamily};
    use crate::family::{DiagnosticSink, FamilyContract, ScopeStack, StageLimits};

    let text = format!("{HEADER}{declarations}\n");
    let parsed = qsl_cst::parse(
        SourceIdentity::new("a", "u", "git", "1"),
        "unit.native",
        text.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    let unit = build_unit(&parsed, FormsLimits::default()).expect("S2 builds the unit");
    let package = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        unit,
        Vec::new(),
        Vec::new(),
    )
    .expect("the unit assembles");
    let form = &package.functions[0];
    let (parameters, result) = package
        .resolved_signatures
        .get(0)
        .expect("the assembler resolves the signature")
        .clone();
    let own_signature = Signature {
        name: form.name.clone(),
        parameters,
        result,
        callable_by_name: true,
    };
    let scope = empty_scope();
    let signatures = Signatures::default();
    let location = body(0, &[]);
    let declarations = declarations_for(
        &scope,
        &signatures,
        &own_signature,
        &[],
        CheckingLimits::default(),
        &location,
    );
    let mut meter = quire_exact::Meter::new(SCALAR_LIMITS_UNLIMITED);
    let mut diagnostics = DiagnosticSink::default();
    let mut scopes = ScopeStack::default();
    let mut cx = check_context(
        &declarations,
        StageLimits {
            node_count,
            ..limits()
        },
        &mut meter,
        &mut diagnostics,
        &mut scopes,
    );
    ValueFunctionFamily::check(form, &mut cx)
}

/// The claims `ValueFunctionFamily`'s pure requirements function returns
/// for the one function of `declarations`, twice from one checked node.
fn family_claims(declarations: &str) -> (Vec<ValueClaim>, Vec<ValueClaim>) {
    use crate::check::ValueFunctionFamily;
    use crate::family::FamilyContract;
    let checked = family_check(declarations, u64::MAX)
        .expect("the declaration checks")
        .into_value();
    (
        ValueFunctionFamily::requirements(&checked),
        ValueFunctionFamily::requirements(&checked),
    )
}

/// TC-160 step 5 (FR-062-AC-4): the pure requirements function yields no
/// claim for `b and c`, whose form has no FR-057 kind, and exactly one
/// `value-validity` claim for `x + y`, at its `+` application (the body's
/// root, which the narrow into `Int[0, 18]` shares). Two calls on one
/// checked node are equal.
#[trace("TC-160", "FR-062-AC-4")]
#[test]
fn tc_160_the_requirements_function_yields_one_claim_per_scalar_application() {
    let (none, again) =
        family_claims("function f using v(b: Boolean, c: Boolean): Boolean pure { b and c }");
    assert_eq!(none, []);
    assert_eq!(again, []);

    let (claims, again) =
        family_claims("function f using v(x: Int[0, 9], y: Int[0, 9]): Int[0, 18] pure { x + y }");
    let [claim] = claims.as_slice() else {
        panic!("one claim: {claims:?}");
    };
    assert_eq!(claim.kind(), Capability::ValueValidity);
    assert_eq!(claim.site().location(), &body(0, &[]));
    assert_eq!(claim.site().result_bound(), int(0, 18));
    assert!(claim.is_bounded());
    assert_eq!(claims, again);
}

/// FR-062 (classification under the declaration's stage limits): a claim
/// whose extent walk passes the node-count limit stops `check` with the
/// declaration's `StageFailure::Limit` of kind node count, while the body's
/// own node count is within it. One position more admits it.
#[trace("TC-160", "FR-062-AC-13")]
#[test]
fn tc_160_a_classification_past_the_node_count_limit_is_a_stage_limit() {
    // Four nested sequences and their `Integer`: five type positions for
    // the `+`'s one root, in a body of three expression nodes.
    let unit = "function f using v(s: Sequence<Sequence<Sequence<Sequence<Integer>[0, 1]>[0, 1]>[0, 1]>[0, 1]): Integer pure { size(s) + 1 }";
    let Err(qsl_foundation::diagnostic::StageFailure::Limit(exceeded)) = family_check(unit, 4)
    else {
        panic!("a node-count limit");
    };
    assert_eq!(
        exceeded.kind(),
        qsl_foundation::diagnostic::LimitKind::NodeCount
    );
    assert_eq!((exceeded.configured_bound(), exceeded.actual()), (4, 5));
    assert!(family_check(unit, 5).is_ok());
}

/// FR-062 (FR-097-AC-2): a root typed by a composite the type environment
/// does not hold is a broken check invariant, never a bounded extent.
#[trace("TC-160", "FR-062-AC-13")]
#[test]
fn tc_160_a_root_of_an_undeclared_composite_is_a_fault() {
    let location = body(0, &[]);
    let node = |kind, value_type| Node {
        kind,
        value_type,
        location: location.clone(),
    };
    let add = node(
        NodeKind::Arithmetic(
            crate::check::Arithmetic::Add,
            Box::new(node(
                NodeKind::Field {
                    operand: Box::new(node(
                        NodeKind::Local(0),
                        ValueType::Composite(NodeKey::from_digest([9; 32])),
                    )),
                    index: 0,
                    optional: false,
                },
                ValueType::Integer,
            )),
            Box::new(node(
                NodeKind::Literal(quire_exact::Value::Integer(Integer::from(1_i64))),
                ValueType::Integer,
            )),
        ),
        ValueType::Integer,
    );
    let parameters = [(
        "r".to_owned(),
        ValueType::Composite(NodeKey::from_digest([9; 32])),
    )];
    let Err(ClassifyFailure::Fault(fault)) = claims_of(
        &add,
        &parameters,
        &location,
        &TypeEnvironment::default(),
        u64::MAX,
    ) else {
        panic!("a fault");
    };
    assert_eq!(
        fault.invariant(),
        "checked-composite-type-not-in-environment"
    );
}

/// The path-condition fixtures: an `if`'s `otherwise` guard (false), `or`
/// guarding its right operand false, `implies` guarding its right operand
/// true, and a nested `if` giving two guards, outermost first.
fn guard_fixtures() -> Vec<(&'static str, Vec<Expect>)> {
    let boolean = |function| (ValueType::Boolean, TypeAt::Result(function));
    let integer = || (ValueType::Integer, TypeAt::Scalar("integer"));
    vec![
        (
            "function nest using v(n: Integer, flag: Boolean): Integer pure { if flag then 0 else if n >= 0 then n + 1 else 0 }",
            vec![
                Expect {
                    operation: "quire.op.integer.ge",
                    site: ("n >= 0", 0),
                    ordinal: 0,
                    roots: &[("nest", 0)],
                    bound: (ValueType::Boolean, TypeAt::Scalar("boolean")),
                    guards: &[("flag", 1, false)],
                },
                Expect {
                    operation: ADD,
                    site: ("n + 1", 0),
                    ordinal: 0,
                    roots: &[("nest", 0)],
                    bound: (ValueType::Integer, TypeAt::Result("nest")),
                    guards: &[("flag", 1, false), ("n >= 0", 0, true)],
                },
            ],
        ),
        (
            "function either using v(n: Integer): Boolean pure { n < 0 or n - 1 >= 0 }",
            vec![
                Expect {
                    operation: "quire.op.integer.lt",
                    site: ("n < 0", 0),
                    ordinal: 0,
                    roots: &[("either", 0)],
                    bound: boolean("either"),
                    guards: &[],
                },
                Expect {
                    operation: "quire.op.integer.sub",
                    site: ("n - 1", 0),
                    ordinal: 0,
                    roots: &[("either", 0)],
                    bound: integer(),
                    guards: &[("n < 0", 0, false)],
                },
                Expect {
                    operation: "quire.op.integer.ge",
                    site: ("n - 1 >= 0", 0),
                    ordinal: 0,
                    roots: &[("either", 0)],
                    bound: boolean("either"),
                    guards: &[("n < 0", 0, false)],
                },
            ],
        ),
        (
            "function onlyif using v(n: Integer): Boolean pure { n > 0 implies n - 1 >= 0 }",
            vec![
                Expect {
                    operation: "quire.op.integer.gt",
                    site: ("n > 0", 0),
                    ordinal: 0,
                    roots: &[("onlyif", 0)],
                    bound: boolean("onlyif"),
                    guards: &[],
                },
                Expect {
                    operation: "quire.op.integer.sub",
                    site: ("n - 1", 0),
                    ordinal: 0,
                    roots: &[("onlyif", 0)],
                    bound: integer(),
                    guards: &[("n > 0", 0, true)],
                },
                Expect {
                    operation: "quire.op.integer.ge",
                    site: ("n - 1 >= 0", 0),
                    ordinal: 0,
                    roots: &[("onlyif", 0)],
                    bound: boolean("onlyif"),
                    guards: &[("n > 0", 0, true)],
                },
            ],
        ),
    ]
}

/// TC-160 step 10 (FR-062-AC-13): a path condition records an `if`'s
/// `otherwise` guard as false, `or`'s left operand as false and
/// `implies`'s as true, and two nested guards outermost first.
#[trace("TC-160", "FR-062-AC-13")]
#[test]
fn tc_160_guards_carry_their_outcome_outermost_first() {
    for (unit, expected) in guard_fixtures() {
        check(unit).assert_records(unit, &expected);
    }
}

impl Checked {
    /// The record at the one `operation` application at the `nth` `needle`.
    fn record(&self, needle: &str, nth: usize, operation: &str) -> &RequirementRecord {
        let key = self.occurrence(needle, nth, Some(operation));
        self.graph
            .requirements()
            .get(&key)
            .unwrap_or_else(|| panic!("a record at `{needle}` #{nth}"))
    }

    /// The node the `argument`th operand of the one `operation`
    /// application at the `nth` `needle` references: a binder's parameter
    /// node, for a binder read.
    fn operand(&self, needle: &str, nth: usize, operation: &str, argument: usize) -> WireNodeId {
        let key = self.occurrence(needle, nth, Some(operation));
        let node = self
            .graph
            .semantic_graph()
            .node(NodeKey::from_digest(*key.node().as_bytes()))
            .expect("the application is lowered");
        let SemanticTerm::Application { arguments, .. } = node.body() else {
            panic!("an application at `{needle}`");
        };
        let Some(SemanticTerm::Reference { target }) = arguments.get(argument) else {
            panic!("operand {argument} at `{needle}` is a reference: {arguments:?}");
        };
        wire(target.0)
    }
}

/// `domains` as an extent.
fn extent(domains: &[(WireNodeId, &[u32], DomainKind)]) -> ClaimExtent {
    ClaimExtent::from_domains(
        domains
            .iter()
            .map(|(node, path, kind)| (DomainKey::new(*node, path.to_vec()), *kind))
            .collect(),
    )
}

/// TC-160 step 10 (FR-062-AC-13): a binder read is a root only of the
/// applications inside the binder's scope. In `size(filter(v in s: v > 0))
/// + 1`, the `>` is unbounded at `v`, and the outer `+` only at `s`'s
/// element: `v` is bound inside it.
#[trace("TC-160", "FR-062-AC-13")]
#[test]
fn tc_160_a_binder_is_a_root_only_inside_its_scope() {
    let checked = check(
        "function sz using v(s: Sequence<Integer>[0, 5]): Integer pure { size(filter(v in s: v > 0)) + 1 }",
    );
    assert_eq!(checked.graph.requirements().len(), 2);
    let v = checked.operand("v > 0", 0, "quire.op.integer.gt", 0);
    assert_eq!(
        checked
            .record("v > 0", 0, "quire.op.integer.gt")
            .requirements()
            .extent(),
        &extent(&[(v, &[], DomainKind::Integer)])
    );
    let s = wire(checked.parameter("sz", 0));
    assert_eq!(
        checked
            .record("size(filter(v in s: v > 0)) + 1", 0, ADD)
            .requirements()
            .extent(),
        &extent(&[(s, &[0], DomainKind::Integer)])
    );
}

/// TC-160 step 10 (FR-062-AC-13): a `fold`'s accumulator and element, a
/// `reduce`'s, and a `flatMap`'s binder are each an extent root keyed by
/// its own parameter node.
#[trace("TC-160", "FR-062-AC-13")]
#[test]
fn tc_160_fold_reduce_and_flat_map_binders_are_roots() {
    for (unit, needle, operation) in [
        (
            "type I = Integer;\nfunction fo using v(s: Sequence<Integer>[0, 5]): Integer pure { fold<I>(acc, x in s: acc + x, identity: 0) }",
            "acc + x",
            ADD,
        ),
        (
            "type I = Integer;\nfunction re using v(s: Sequence<Integer>[1, 5]): Integer pure { reduce<I>(acc, x in s: acc * x) }",
            "acc * x",
            MUL,
        ),
    ] {
        let checked = check(unit);
        let accumulator = checked.operand(needle, 0, operation, 0);
        let element = checked.operand(needle, 0, operation, 1);
        assert_ne!(accumulator, element, "{unit}");
        assert_eq!(
            checked.record(needle, 0, operation).requirements().extent(),
            &extent(&[
                (accumulator, &[], DomainKind::Integer),
                (element, &[], DomainKind::Integer),
            ]),
            "{unit}"
        );
    }

    let checked = check(
        "function fm using v(s: Sequence<Integer>[0, 3]): Sequence<Integer>[0, 9] pure { flatMap(x in s: map(y in s: x + y)) }",
    );
    let x = checked.operand("x + y", 0, ADD, 0);
    let y = checked.operand("x + y", 0, ADD, 1);
    assert_ne!(x, y);
    assert_ne!(x, wire(checked.parameter("fm", 0)));
    assert_eq!(
        checked.record("x + y", 0, ADD).requirements().extent(),
        &extent(&[(x, &[], DomainKind::Integer), (y, &[], DomainKind::Integer)])
    );
}
