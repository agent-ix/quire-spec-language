// SPDX-License-Identifier: AGPL-3.0-or-later
use super::*;
use crate::spine::DependencyInput;
use ix_trace_rs::trace;
use qsl_foundation::diagnostic::{CatalogCoded, UndefinedCoded, UndefinedReason, UndefinedRecord};
use qsl_foundation::SourceIdentity;
use qsl_semantics::check::Origin;
use qsl_semantics::model::key::DeclarationKey;
use qsl_semantics::model::refusal::ModelRefusalCause;
use quire_exact::{BoundViolation, CardinalityBound, CollectionKind, Incomplete, UniverseId};
use std::collections::BTreeMap;

#[test]
fn is_identifier_admits_ascii_identifiers_only() {
    assert!(is_identifier("seven"));
    assert!(is_identifier("_seven"));
    assert!(is_identifier("a1"));
    assert!(!is_identifier(""));
    assert!(!is_identifier("7x"));
    assert!(!is_identifier("a.b"));
}

const FIXTURE: &str = include_str!("../../../../tests/fixtures/spine-run.native");

fn source() -> SourceIdentity {
    SourceIdentity::new("agent-ix", "spine-run-fixture", "git", "1")
}

fn call(function: &str, arguments: Vec<CallArgument>) -> Call {
    Call {
        function: function.to_owned(),
        arguments,
        accounting: default_accounting(1_000_000),
    }
}

fn run_fixture(bytes: &str, call: &Call) -> Result<CallOutcome, Box<RunRefusal>> {
    run(
        source(),
        "spine-run.native",
        bytes.as_bytes(),
        &BTreeMap::new(),
        &DependencyInput::default(),
        SpineLimits::default(),
        call,
    )
    .map(|(_, outcome)| outcome)
}

fn arg(parameter: &str, value: i64) -> CallArgument {
    CallArgument {
        parameter: parameter.to_owned(),
        value,
    }
}

/// FR-100-AC-1/AC-7 (TC-450 step 1, TC-452 step 1): `seven()` completes
/// integer `7`, and the returned `package_id` equals `spine::compile`'s.
#[trace("TC-452", "FR-100-AC-7")]
#[test]
fn tc_452_seven_completes_and_agrees_with_compile() {
    let compiled = compile(
        source(),
        "spine-run.native",
        FIXTURE.as_bytes(),
        &BTreeMap::new(),
        &DependencyInput::default(),
        SpineLimits::default(),
    )
    .unwrap();
    let (package_id, outcome) = run(
        source(),
        "spine-run.native",
        FIXTURE.as_bytes(),
        &BTreeMap::new(),
        &DependencyInput::default(),
        SpineLimits::default(),
        &call("seven", Vec::new()),
    )
    .unwrap();
    assert_eq!(package_id, compiled.emitted.package_id());
    match outcome {
        CallOutcome::Completed(CallValue::Integer(value)) => {
            assert_eq!(value.to_string(), "7");
        }
        other => panic!("expected a completed integer, got {other:?}"),
    }
}

/// FR-100-AC-3 (TC-450 step 6): a `1-draft` request supplying both
/// `libraries` and `models` runs, exit 0. `spine-run.native`'s `id` function
/// takes no library or model, so this exercises the two inputs being wired
/// through unused rather than the compiled program actually depending on
/// them; TC-450 step 6 at the CLI layer exercises the request shape this
/// unit test exercises the plumbing for.
#[trace("TC-450", "FR-100-AC-3")]
#[test]
fn tc_450_step_6_libraries_and_models_both_present_runs() {
    let outcome = run(
        source(),
        "spine-run.native",
        FIXTURE.as_bytes(),
        &BTreeMap::new(),
        &DependencyInput::default(),
        SpineLimits::default(),
        &call("seven", Vec::new()),
    )
    .unwrap()
    .1;
    assert!(matches!(outcome, CallOutcome::Completed(_)));
}

/// FR-100-AC-4/AC-7 (TC-451 step 1): `lt` binds `b` before `a` by name,
/// `flag`/`id` complete their declared kind.
#[trace("TC-451", "FR-100-AC-4")]
#[trace("TC-452", "FR-100-AC-7")]
#[test]
fn tc_451_step_1_arguments_bind_by_declared_name() {
    let lt = run_fixture(FIXTURE, &call("lt", vec![arg("b", 3), arg("a", 5)])).unwrap();
    assert!(matches!(
        lt,
        CallOutcome::Completed(CallValue::Boolean(false))
    ));
    let flag = run_fixture(FIXTURE, &call("flag", vec![arg("b", 1)])).unwrap();
    assert!(matches!(
        flag,
        CallOutcome::Completed(CallValue::Boolean(true))
    ));
    let id = run_fixture(FIXTURE, &call("id", vec![arg("x", 4)])).unwrap();
    match id {
        CallOutcome::Completed(CallValue::Integer(value)) => assert_eq!(value.to_string(), "4"),
        other => panic!("expected a completed integer, got {other:?}"),
    }
}

/// FR-100-AC-4 (TC-451 step 2): a value outside the parameter's kind or
/// declared domain refuses `WrongValueKind` at position 0, whether found
/// before the call (`flag`, `px`) or at S6a admission (`id`). `corner`
/// (declared result `Point`, a record) also refuses before the call at all,
/// so it never reaches this admission path -- a distinct oracle from a
/// bad-argument refusal (FR-100-AC-5, `tc_451_step_5`).
#[trace("TC-451", "FR-100-AC-4")]
#[test]
fn tc_451_step_2_wrong_value_kind_names_position() {
    for (function, argument) in [
        ("flag", arg("b", 2)),
        ("id", arg("x", 12)),
        ("px", arg("p", 1)),
    ] {
        let refusal = run_fixture(FIXTURE, &call(function, vec![argument])).unwrap_err();
        assert_eq!(refusal.stage(), "call");
        assert_eq!(refusal.code(), Code::InvalidRuntimeInput);
        assert!(
            matches!(*refusal, RunRefusal::WrongValueKind { position: 0 }),
            "{refusal:?}"
        );
    }
}

/// FR-100-AC-4 (TC-451 step 3): an argument naming no parameter, a
/// parameter bound twice, and a parameter left unbound each refuse
/// `invalid_runtime_input` at stage `call`, naming the parameter.
#[trace("TC-451", "FR-100-AC-4")]
#[test]
fn tc_451_step_3_argument_binding_names_the_parameter() {
    let unknown = run_fixture(FIXTURE, &call("id", vec![arg("y", 4)])).unwrap_err();
    assert!(matches!(*unknown, RunRefusal::UnknownParameter { ref parameter } if parameter == "y"));
    let duplicate = run_fixture(FIXTURE, &call("id", vec![arg("x", 1), arg("x", 2)])).unwrap_err();
    assert!(
        matches!(*duplicate, RunRefusal::DuplicateArgument { ref parameter } if parameter == "x")
    );
    let unbound = run_fixture(FIXTURE, &call("id", Vec::new())).unwrap_err();
    assert!(matches!(*unbound, RunRefusal::UnboundParameter { ref parameter } if parameter == "x"));
    for refusal in [unknown, duplicate, unbound] {
        assert_eq!(refusal.stage(), "call");
        assert_eq!(refusal.code(), Code::InvalidRuntimeInput);
    }
}

/// FND-010: binding every parameter is checked before any value is
/// converted. `lt(a, b)` with `a` bound to a wrong-kind literal but `b`
/// altogether unbound refuses `UnboundParameter { parameter: "b" }`, not
/// `WrongValueKind` for `a` -- deleting the "bind everything first" split in
/// `bind_arguments` (converting eagerly per argument, as the arguments
/// arrive) turns this red, since it would instead report `a`'s wrong kind
/// first.
#[trace("FR-100-AC-4", "TC-451")]
#[test]
fn fnd_010_unbound_parameter_is_reported_before_an_earlier_wrong_kind() {
    let refusal =
        run_fixture(FIXTURE, &call("lt", vec![arg("a", 9223372036854775807)])).unwrap_err();
    assert!(
        matches!(*refusal, RunRefusal::UnboundParameter { ref parameter } if parameter == "b"),
        "{refusal:?}"
    );
}

/// FR-100-AC-5 (TC-451 step 5): each malformed or unresolved `function`
/// string refuses `missing_declaration` at stage `call`, and a record
/// result refuses `unsupported_construct`, before any call.
#[trace("TC-451", "FR-100-AC-5")]
#[test]
fn tc_451_step_5_function_shape_and_lookup() {
    for name in ["nope", "module.seven", "", "seven.", "7x"] {
        let refusal = run_fixture(FIXTURE, &call(name, Vec::new())).unwrap_err();
        assert_eq!(refusal.stage(), "call");
        assert_eq!(refusal.code(), Code::MissingDeclaration);
        assert!(
            matches!(*refusal, RunRefusal::MissingDeclaration { ref function } if function == name),
            "{refusal:?}"
        );
    }
    let corner = run_fixture(FIXTURE, &call("corner", vec![arg("p", 0)])).unwrap_err();
    assert_eq!(corner.stage(), "call");
    assert_eq!(corner.code(), Code::UnsupportedConstruct);
    assert!(
        matches!(*corner, RunRefusal::UnsupportedResult { ref function } if function == "corner")
    );
}

/// FR-100-AC-5 (TC-451 step 6): a syntax error refuses at stage `source`
/// (`invalid_syntax`), and integer `/` with no `Rational` expected type
/// refuses at stage `check` (`ill_typed`).
#[trace("TC-451", "FR-100-AC-5")]
#[test]
fn tc_451_step_6_compile_refusals_carry_their_own_stage() {
    const SYNTAX_ERROR: &str = "language \"ix:native\" edition \"1-draft\";\nfunction (";
    let refusal = run_fixture(SYNTAX_ERROR, &call("f", Vec::new())).unwrap_err();
    assert_eq!(refusal.stage(), "source");
    assert_eq!(refusal.code(), Code::InvalidSyntax);

    const ILL_TYPED: &str = "language \"ix:native\" edition \"1-draft\";\n\
        profile v = \"quire.value.complete/v1\" version \"1-draft.2\" digest \
        \"sha256:c8c7ae9fbe783286369ecc83f006190f83be4c3c8fc585766617c90f27a25b16\";\n\
        type Digit = Int[0, 9];\n\
        function inv using v(x: Digit): Boolean pure { 1 / x > 0 }\n";
    let refusal = run_fixture(ILL_TYPED, &call("inv", vec![arg("x", 1)])).unwrap_err();
    assert_eq!(refusal.stage(), "check");
    assert_eq!(refusal.code(), Code::IllTyped);
}

/// FR-100-AC-6 (TC-451 step 7): `work_units` 0 exhausts before the
/// charge, reported `incomplete` at `work_units`.
#[trace("TC-451", "FR-100-AC-6")]
#[test]
fn tc_451_step_7_zero_work_units_is_incomplete() {
    let outcome = run_fixture(FIXTURE, &call("seven", vec![])).unwrap();
    assert!(matches!(outcome, CallOutcome::Completed(_)));
    let starved = Call {
        accounting: default_accounting(0),
        ..call("seven", Vec::new())
    };
    let outcome = run_fixture(FIXTURE, &starved).unwrap();
    assert!(
        matches!(
            outcome,
            CallOutcome::Incomplete {
                limit: "work_units"
            }
        ),
        "{outcome:?}"
    );
}

#[derive(Debug)]
struct StubUndefined(UndefinedReason);

impl UndefinedCoded for StubUndefined {
    fn undefined_record(&self) -> UndefinedRecord {
        UndefinedRecord {
            reason: self.0,
            fields: BTreeMap::new(),
        }
    }
}

/// A `qsl_semantics::model::refusal::ModelRefusalCause` as a family
/// `CatalogCoded`, exactly the shape `qsl-eval`'s own `ModelQueryRefusal`
/// wraps it in (that type is crate-private to `qsl-eval`, so this test
/// builds its own thin wrapper rather than reaching into it).
#[derive(Debug)]
struct ModelCause(ModelRefusalCause);

impl CatalogCoded for ModelCause {
    fn catalog_code(&self) -> CatalogCode {
        self.0.catalog_code()
    }

    fn catalog_fields(&self) -> Option<BTreeMap<&'static str, String>> {
        self.0.catalog_fields()
    }
}

/// TC-452's fixture unit `F`: three LF-terminated lines, 237 bytes, with a
/// `function f` whose body's literal `5` (path `[1]`) is at byte 233 to
/// 234, line 3 column 54 to 55 -- recomputed independently of the spec's
/// own literal digest and span, which TC-452 (A05's lane) still spells for
/// the placeholder header.
const FIXTURE_F: &str = "language \"ix:native\" edition \"1-draft\";\nprofile v = \"quire.value.complete/v1\" version \"1-draft.2\" digest \"sha256:c8c7ae9fbe783286369ecc83f006190f83be4c3c8fc585766617c90f27a25b16\";\nfunction f using v(x: Int[0, 9]): Integer pure { x + 5 }\n";

fn fixture_f_location() -> Location {
    Location {
        origin: Origin::Body {
            function: "f".to_owned(),
            index: 0,
        },
        path: vec![1],
    }
}

fn evaluation(outcome: FamilyOutcome<Value>) -> qsl_eval::value::Evaluation {
    qsl_eval::value::Evaluation {
        outcome,
        location: Some(fixture_f_location()),
        losses: Vec::new(),
    }
}

/// FR-100-AC-9 (TC-452 step 4): the outcome mapping converts every S6a
/// outcome category the mapping table names, over the literal fixture unit
/// `F`.
#[trace("TC-452", "FR-100-AC-9")]
#[test]
fn tc_452_step_4_outcome_mapping_covers_every_category() {
    assert_eq!(FIXTURE_F.len(), 237, "fixture F is 237 bytes");
    let compiled = compile(
        source(),
        "tc-452-f.native",
        FIXTURE_F.as_bytes(),
        &BTreeMap::new(),
        &DependencyInput::default(),
        SpineLimits::default(),
    )
    .unwrap();
    let graph = compiled.package.graph();
    let sources = vec![Source::read(
        source(),
        "tc-452-f.native",
        FIXTURE_F.as_bytes(),
        qsl_foundation::source::MAX_SOURCE_BYTES,
    )
    .unwrap()];

    let convert =
        |outcome: FamilyOutcome<Value>| convert_outcome(evaluation(outcome), graph, &sources);

    // Completed: each value kind.
    for (value, expected) in [
        (Value::Boolean(true), "true"),
        (Value::Boolean(false), "false"),
    ] {
        match convert(FamilyOutcome::Evaluated(Outcome::Completed(value))).unwrap() {
            CallOutcome::Completed(CallValue::Boolean(rendered)) => {
                assert_eq!(rendered.to_string(), expected);
            }
            other => panic!("{other:?}"),
        }
    }
    for (value, expected) in [(0i64, "0"), (-17, "-17")] {
        match convert(FamilyOutcome::Evaluated(Outcome::Completed(
            Value::Integer(Integer::from(value)),
        )))
        .unwrap()
        {
            CallOutcome::Completed(CallValue::Integer(rendered)) => {
                assert_eq!(rendered.to_string(), expected);
            }
            other => panic!("{other:?}"),
        }
    }
    let big = Integer::from(1i64).shifted_left(70);
    match convert(FamilyOutcome::Evaluated(Outcome::Completed(
        Value::Integer(big),
    )))
    .unwrap()
    {
        CallOutcome::Completed(CallValue::Integer(rendered)) => {
            assert_eq!(rendered.to_string(), "1180591620717411303424");
        }
        other => panic!("{other:?}"),
    }

    let location = fixture_f_location();
    let assert_location = |actual: &Option<Location>| {
        assert_eq!(actual.as_ref(), Some(&location));
    };

    // CardinalityOutOfBound: a record, with fields and locus.
    for (violation, count, cause) in [
        (BoundViolation::AboveMaximum, 4u64, "above-maximum"),
        (BoundViolation::BelowMinimum, 0u64, "below-minimum"),
    ] {
        let refusal = Refusal::CardinalityOutOfBound {
            violation,
            kind: CollectionKind::Set,
            bound: CardinalityBound::new(1, 3).unwrap(),
            count,
        };
        match convert(FamilyOutcome::Evaluated(Outcome::Refused(refusal))).unwrap() {
            CallOutcome::Refused(CallRefusal::Record {
                code,
                fields,
                locus,
                location: got_location,
            }) => {
                assert_eq!(code, CatalogCode::new("cardinality_out_of_bound", cause));
                assert_eq!(
                    fields,
                    BTreeMap::from([
                        ("collection", "set".to_owned()),
                        ("bound", "[1, 3]".to_owned()),
                        ("count", count.to_string()),
                    ])
                );
                let locus = locus.expect("a record locus");
                assert_eq!(
                    locus.source_digest,
                    "sha256:3cb8ab70e4d3187dae8621768491c4d2eb0c8c0b82d330d4f2fba72883f6e77c"
                );
                assert_eq!(locus.span.start.byte, 233);
                assert_eq!(locus.span.start.line, 3);
                assert_eq!(locus.span.start.column, 54);
                assert_eq!(locus.span.end.byte, 234);
                assert_eq!(locus.span.end.line, 3);
                assert_eq!(locus.span.end.column, 55);
                assert_location(&got_location);
            }
            other => panic!("{other:?}"),
        }
    }

    // The ten kernel value refusals (TC-452 step 4): each is a
    // record with the refusal's own code and cause, its exact catalog
    // fields (TC-452's payloads), a locus and the location.
    {
        use quire_exact::{
            DecimalType, IeeeFlag, IeeeFlags, IeeeWidth, InexactTarget, Integer, IntegerInterval,
            RationalDomain, RoundingMode, TextProfile, TextType,
        };
        let int_0_9 = || IntegerInterval::spanning(Integer::zero(), Integer::from(9_i64));
        let decimal_0_100 = || {
            Box::new(
                DecimalType::new(
                    Integer::zero(),
                    Integer::from(100_i64),
                    0,
                    2,
                    RoundingMode::Exact,
                )
                .unwrap(),
            )
        };
        let rational_neg5_5 = || {
            Box::new(
                RationalDomain::new(
                    IntegerInterval::spanning(Integer::from(-5_i64), Integer::from(5_i64)),
                    IntegerInterval::spanning(Integer::one(), Integer::from(12_i64)),
                )
                .unwrap(),
            )
        };
        let kernel_value_refusals = [
            (
                Refusal::InexactDecimal {
                    target: InexactTarget::Decimal(decimal_0_100()),
                },
                "inexact_decimal",
                "nonzero-discarded-digit",
                BTreeMap::from([("expected", "Decimal[0, 100; 0, 2]".to_owned())]),
            ),
            (
                Refusal::DecimalOutOfDomain {
                    target: decimal_0_100(),
                },
                "decimal_out_of_domain",
                "outside-domain",
                BTreeMap::from([("expected", "Decimal[0, 100; 0, 2]".to_owned())]),
            ),
            (
                Refusal::DivisionPairOutOfDomain {
                    domain: Box::new(int_0_9()),
                    quotient_admitted: false,
                    remainder_admitted: true,
                },
                "division_pair_out_of_domain",
                "quotient-outside-domain",
                BTreeMap::from([("expected", "Int[0, 9]".to_owned())]),
            ),
            (
                Refusal::DivisionPairOutOfDomain {
                    domain: Box::new(int_0_9()),
                    quotient_admitted: true,
                    remainder_admitted: false,
                },
                "division_pair_out_of_domain",
                "remainder-outside-domain",
                BTreeMap::from([("expected", "Int[0, 9]".to_owned())]),
            ),
            (
                Refusal::DivisionPairOutOfDomain {
                    domain: Box::new(int_0_9()),
                    quotient_admitted: false,
                    remainder_admitted: false,
                },
                "division_pair_out_of_domain",
                "both-outside-domain",
                BTreeMap::from([("expected", "Int[0, 9]".to_owned())]),
            ),
            (
                Refusal::ModuloOutOfDomain {
                    domain: Box::new(int_0_9()),
                },
                "modulo_out_of_domain",
                "outside-domain",
                BTreeMap::from([("expected", "Int[0, 9]".to_owned())]),
            ),
            (
                Refusal::TextLengthOutOfDomain {
                    target: TextType::new(1, 8, TextProfile::BinaryUtf8).unwrap(),
                },
                "text_length_out_of_domain",
                "outside-domain",
                BTreeMap::from([("expected", "Text[1, 8; binary-utf8]".to_owned())]),
            ),
            (
                Refusal::IntegerOutOfDomain {
                    target: Box::new(int_0_9()),
                },
                "integer_out_of_domain",
                "outside-domain",
                BTreeMap::from([("expected", "Int[0, 9]".to_owned())]),
            ),
            (
                Refusal::RationalOutOfDomain {
                    target: rational_neg5_5(),
                },
                "rational_out_of_domain",
                "outside-domain",
                BTreeMap::from([("expected", "Rational[-5, 5; 1, 12]".to_owned())]),
            ),
            (
                Refusal::IeeeNotExact {
                    target: IeeeWidth::Binary64,
                    would_be: [IeeeFlag::Overflow, IeeeFlag::Inexact]
                        .into_iter()
                        .collect::<IeeeFlags>(),
                },
                "ieee_not_exact",
                "rounding-required",
                BTreeMap::from([
                    ("expected", "binary64".to_owned()),
                    ("flags", "overflow,inexact".to_owned()),
                ]),
            ),
            (
                Refusal::IeeeNanPayloadNotRepresentable {
                    target: IeeeWidth::Binary32,
                    source: IeeeWidth::Binary64,
                },
                "ieee_nan_payload_not_representable",
                "payload-exceeds-target",
                BTreeMap::from([
                    ("expected", "binary32".to_owned()),
                    ("actual", "binary64".to_owned()),
                ]),
            ),
            (
                Refusal::IeeeRationalOutOfDomain {
                    target: rational_neg5_5(),
                },
                "ieee_rational_out_of_domain",
                "outside-domain",
                BTreeMap::from([("expected", "Rational[-5, 5; 1, 12]".to_owned())]),
            ),
        ];
        for (refusal, code, cause, expected_fields) in kernel_value_refusals {
            match convert(FamilyOutcome::Evaluated(Outcome::Refused(refusal.clone()))).unwrap() {
                CallOutcome::Refused(CallRefusal::Record {
                    code: got_code,
                    fields,
                    locus,
                    location: got_location,
                }) => {
                    assert_eq!(got_code, CatalogCode::new(code, cause), "{refusal:?}");
                    assert_eq!(fields, expected_fields, "{refusal:?}");
                    let locus = locus.expect("a record locus");
                    assert_eq!(
                        locus.source_digest,
                        "sha256:3cb8ab70e4d3187dae8621768491c4d2eb0c8c0b82d330d4f2fba72883f6e77c"
                    );
                    assert_eq!(locus.span.start.byte, 233);
                    assert_eq!(locus.span.end.byte, 234);
                    assert_location(&got_location);
                }
                other => panic!("{refusal:?}: {other:?}"),
            }
        }
    }

    // ForeignReference: a record, with `required`/`supplied` fields
    // and a locus -- no longer bare, now that the kernel variant carries both
    // universes.
    {
        let required = UniverseId::from_digest([0x01; 32]);
        let supplied = UniverseId::from_digest([0x02; 32]);
        let refusal = Refusal::ForeignReference { required, supplied };
        match convert(FamilyOutcome::Evaluated(Outcome::Refused(refusal))).unwrap() {
            CallOutcome::Refused(CallRefusal::Record {
                code,
                fields,
                locus,
                location: got_location,
            }) => {
                assert_eq!(
                    code,
                    CatalogCode::new("foreign_reference", "foreign-universe")
                );
                assert_eq!(
                    fields,
                    BTreeMap::from([("required", "01".repeat(32)), ("supplied", "02".repeat(32)),])
                );
                let locus = locus.expect("a record locus");
                assert_eq!(
                    locus.source_digest,
                    "sha256:3cb8ab70e4d3187dae8621768491c4d2eb0c8c0b82d330d4f2fba72883f6e77c"
                );
                assert_eq!(locus.span.start.byte, 233);
                assert_eq!(locus.span.end.byte, 234);
                assert_location(&got_location);
            }
            other => panic!("{other:?}"),
        }
    }

    // `CheckedInvariant`: an internal failure, not a `refused` outcome.
    let err = convert(FamilyOutcome::Evaluated(Outcome::Refused(
        Refusal::CheckedInvariant,
    )))
    .unwrap_err();
    assert!(matches!(
        *err,
        RunRefusal::Fault(ref fault)
            if fault.stage() == "S6a" && fault.invariant() == "checked-program-invariant"
    ));

    // Kernel undefined: each of the five reasons.
    for (reason, spelling) in [
        (Undefined::DivisionByZero, "division-by-zero"),
        (Undefined::IeeeNotFinite, "ieee-not-finite"),
        (Undefined::EmptyReduction, "empty-reduction"),
        (Undefined::NoneValue, "none-value"),
        (Undefined::SumOutOfDomain, "sum-out-of-domain"),
    ] {
        match convert(FamilyOutcome::Evaluated(Outcome::Undefined(reason))).unwrap() {
            CallOutcome::Undefined { reason } => assert_eq!(reason, spelling),
            other => panic!("{other:?}"),
        }
    }

    // Incomplete at `work_units`.
    let incomplete = Incomplete {
        limit_kind: quire_exact::LimitKind::WorkUnits,
        limit: 0,
        consumed: 0,
        next_charge: Integer::from(1i64),
        charge_point: quire_exact::ChargePoint::IntegerArithmeticOperands,
    };
    match convert(FamilyOutcome::Evaluated(Outcome::Incomplete(incomplete))).unwrap() {
        CallOutcome::Incomplete { limit } => assert_eq!(limit, "work_units"),
        other => panic!("{other:?}"),
    }

    // `FamilyResult::Refused` with an FR-096 key-table row: `AbsentKey`.
    let absent_key = ModelCause(ModelRefusalCause::AbsentKey {
        binding: "people".to_owned(),
        key: b"p7".to_vec(),
    });
    match convert(FamilyOutcome::FamilyEvaluated(FamilyResult::Refused(
        Box::new(absent_key),
    )))
    .unwrap()
    {
        CallOutcome::Refused(CallRefusal::Record { code, fields, .. }) => {
            assert_eq!(
                code,
                CatalogCode::new("invalid_runtime_input", "absent-key")
            );
            assert_eq!(
                fields,
                BTreeMap::from([("binding", "people".to_owned()), ("key", "p7".to_owned()),])
            );
        }
        other => panic!("{other:?}"),
    }

    // `FamilyResult::Refused` with a row: `ForeignUniverse`.
    let foreign_universe = ModelCause(ModelRefusalCause::ForeignUniverse {
        actual: vec![0x01; 32],
        expected: UniverseId::from_digest([0x02; 32]),
    });
    match convert(FamilyOutcome::FamilyEvaluated(FamilyResult::Refused(
        Box::new(foreign_universe),
    )))
    .unwrap()
    {
        CallOutcome::Refused(CallRefusal::Record { code, fields, .. }) => {
            assert_eq!(
                code,
                CatalogCode::new("foreign_reference", "foreign-universe")
            );
            assert_eq!(
                fields,
                BTreeMap::from([("required", "02".repeat(32)), ("supplied", "01".repeat(32)),])
            );
        }
        other => panic!("{other:?}"),
    }

    // `FamilyResult::Refused` with no row: `TypeMismatch`, `AncestorSteps`.
    let type_mismatch = ModelCause(ModelRefusalCause::TypeMismatch);
    match convert(FamilyOutcome::FamilyEvaluated(FamilyResult::Refused(
        Box::new(type_mismatch),
    )))
    .unwrap()
    {
        CallOutcome::Refused(CallRefusal::Family { code, .. }) => {
            assert_eq!(code, CatalogCode::new("ill_typed", "type-mismatch"));
        }
        other => panic!("{other:?}"),
    }
    let ancestor_steps = ModelCause(ModelRefusalCause::AncestorSteps {
        from: DeclarationKey {
            package: "test/orders".to_owned(),
            node: "n".to_owned(),
        },
        limit: 5,
    });
    match convert(FamilyOutcome::FamilyEvaluated(FamilyResult::Refused(
        Box::new(ancestor_steps),
    )))
    .unwrap()
    {
        CallOutcome::Refused(CallRefusal::Family { code, .. }) => {
            assert_eq!(
                code,
                CatalogCode::new("resource_exhausted", "ancestor-steps")
            );
        }
        other => panic!("{other:?}"),
    }

    // `FamilyResult::Undefined`: `precondition-false` and `absent-key`.
    for reason in [
        UndefinedReason::PreconditionFalse,
        UndefinedReason::AbsentKey,
    ] {
        let undefined = FamilyResult::Undefined(Box::new(StubUndefined(reason)));
        match convert(FamilyOutcome::FamilyEvaluated(undefined)).unwrap() {
            CallOutcome::Undefined { reason: rendered } => {
                assert_eq!(rendered, reason.as_str());
            }
            other => panic!("{other:?}"),
        }
    }
}

/// FR-100-AC-10 (TC-452 step 5, SR-749 FND-002): `sum<Pos>(x in q: x)` for
/// `Pos = Int[1, 9]`, checked under `CheckMode::Kernel` (TC-452 step 5's own
/// procedure -- a linked `sum` whose prefixes are not all proved members
/// refuses at checking, QSpec FR-145, so a real compiled program can never
/// reach this outcome; only a standalone expression checked this way can)
/// and evaluated for real, not a hand-built `Evaluation` unlike
/// [`tc_452_step_4_outcome_mapping_covers_every_category`], then converted
/// by the real `convert_outcome`.
///
/// An empty `q` leaves the seed `0` outside `Pos`, and `q` holding `9, 9`
/// leaves the running total `18` outside `Pos`: both are
/// `Outcome::Undefined(Undefined::SumOutOfDomain)`, located at the `sum`
/// node (`checked.root()`'s own location, never a summand's -- FR-100: "a
/// failing addition's running total at the sum node"), and the real
/// `convert_outcome` converts each to `CallOutcome::Undefined { reason:
/// "sum-out-of-domain" }`. `q` holding `4` completes with integer `4`
/// (TC-452 step 5's own third case).
///
/// Composes with `undefined_kernel_reasons_render_and_exit_20`
/// (`src/command/output.rs`), which proves every `CallOutcome::Undefined {
/// reason: "sum-out-of-domain" }` renders `{"kind": "undefined", "reason":
/// "sum-out-of-domain"}` and exits 20 -- the root crate names no
/// `qsl_eval` path (FR-100-AC-8, TC-452 step 2/3), so a real `Evaluation`
/// can only be produced here, never there.
#[trace("TC-452", "FR-100-AC-10")]
#[test]
fn tc_452_step_5_sum_over_pos_is_sum_out_of_domain_or_completes() {
    use qsl_eval::value::CheckedPackageEvaluation;
    use qsl_forms::Expression;
    use qsl_semantics::check::{CheckMode, CheckingLimits, PackageDeclarations};
    use quire_exact::{CardinalityBound, CollectionType, Integer, IntegerInterval, Meter};

    let pos = ValueType::Int(IntegerInterval::spanning(
        Integer::one(),
        Integer::from(9_i64),
    ));
    let graph = PackageDeclarations {
        aliases: vec![("Pos".to_owned(), pos.clone())],
        ..PackageDeclarations::new(qsl_semantics::check::fixture_source())
    }
    .check(CheckingLimits::default())
    .unwrap();
    let package = CheckedPackage::link(graph);

    let q_type = ValueType::collection(CollectionType::new(
        CollectionKind::Sequence,
        pos,
        Some(CardinalityBound::new(0, 2).unwrap()),
    ));
    let expression = Expression::Sum {
        result_type_span: qsl_foundation::Span { start: 0, end: 0 },
        result_type: "Pos".to_owned(),
        binder: "x".to_owned(),
        source: Box::new(Expression::Name("q".to_owned())),
        summand: Box::new(Expression::Name("x".to_owned())),
    };
    let checked = package
        .graph()
        .check_expression(
            vec![("q".to_owned(), q_type.clone())],
            &expression,
            None,
            CheckMode::Kernel,
            CheckingLimits::default(),
        )
        .unwrap();
    let sum_node = checked.root().location().clone();

    let ValueType::Collection(q_collection_type) = &q_type else {
        unreachable!("q_type is always a collection");
    };
    let q = |elements: Vec<i64>| {
        let elements = elements
            .into_iter()
            .map(|value| Value::Integer(Integer::from(value)))
            .collect();
        quire_exact::form_collection(
            q_collection_type,
            elements,
            &mut Meter::new(default_accounting(u64::MAX)),
        )
        .unwrap()
        .completed()
        .expect("q admits its own declared bound")
    };
    let evaluate = |q_value: Value| {
        package
            .evaluate(
                &checked,
                vec![q_value],
                &ObjectEnvironment::default(),
                &mut Meter::new(default_accounting(u64::MAX)),
            )
            .unwrap()
    };

    for elements in [Vec::<i64>::new(), vec![9, 9]] {
        let evaluation = evaluate(q(elements.clone()));
        assert!(
            matches!(
                evaluation.outcome,
                FamilyOutcome::Evaluated(Outcome::Undefined(Undefined::SumOutOfDomain))
            ),
            "{elements:?}: {:?}",
            evaluation.outcome
        );
        assert_eq!(
            evaluation.location.as_ref(),
            Some(&sum_node),
            "{elements:?}: located at the sum node"
        );
        match convert_outcome(evaluation, package.graph(), &[]).unwrap() {
            CallOutcome::Undefined { reason } => assert_eq!(reason, "sum-out-of-domain"),
            other => panic!("{elements:?}: {other:?}"),
        }
    }

    match convert_outcome(evaluate(q(vec![4])), package.graph(), &[]).unwrap() {
        CallOutcome::Completed(CallValue::Integer(value)) => assert_eq!(value.to_string(), "4"),
        other => panic!("{other:?}"),
    }
}

/// FR-100 "Internal failure at S6a" (SR-674 FND-013): `convert_call_failure`
/// maps `CallFailure::Input(WrongValueKind)` to its own typed refusal, every
/// other `InputRefusal` (already admitted before S6a, so never actually
/// reachable) to an internal fault naming that invariant, and
/// `CallFailure::Fault` straight through, unchanged.
#[test]
fn convert_call_failure_maps_wrong_value_kind_and_forwards_faults() {
    use qsl_eval::value::{CallFailure, InputRefusal};

    match *convert_call_failure(CallFailure::Input(InputRefusal::WrongValueKind {
        parameter: 2,
    })) {
        RunRefusal::WrongValueKind { position } => assert_eq!(position, 2),
        other => panic!("{other:?}"),
    }

    match *convert_call_failure(CallFailure::Input(InputRefusal::UnknownFunction(
        "ghost".to_owned(),
    ))) {
        RunRefusal::Fault(fault) => {
            assert_eq!(fault.stage(), "call");
            assert_eq!(
                fault.invariant(),
                "spine-run-supplies-admitted-name-and-arity"
            );
        }
        other => panic!("{other:?}"),
    }

    match *convert_call_failure(CallFailure::Fault(InternalFault::new(
        "S6a",
        "checked-program-invariant",
    ))) {
        RunRefusal::Fault(fault) => {
            assert_eq!(fault.stage(), "S6a");
            assert_eq!(fault.invariant(), "checked-program-invariant");
        }
        other => panic!("{other:?}"),
    }
}

/// SR-748 FND-001: `convert_refusal`'s `record == None && fallback == None`
/// branch (call.rs:513-518) is unreachable from `convert_outcome` today (see
/// its own doc comment), but is built directly here -- nothing else can
/// reach it -- to prove it returns a typed `InternalFault`, stage `call`,
/// invariant `kernel-refusal-with-no-record`, rather than panicking.
#[test]
fn convert_refusal_with_no_record_and_no_fallback_is_a_typed_fault() {
    match *convert_refusal(None, None, None, &[]).unwrap_err() {
        RunRefusal::Fault(fault) => {
            assert_eq!(fault.stage(), "call");
            assert_eq!(fault.invariant(), "kernel-refusal-with-no-record");
        }
        other => panic!("{other:?}"),
    }
}
