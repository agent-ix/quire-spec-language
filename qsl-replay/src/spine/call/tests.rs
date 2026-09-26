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
        profile v = \"quire.value.complete/v1\" version \"1\" digest \
        \"sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\";\n\
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

/// TC-452's fixture unit `F`: three LF-terminated lines, 229 bytes, with a
/// `function f` whose body's literal `5` (path `[1]`) is at byte 225 to
/// 226, line 3 column 54 to 55 -- recomputed independently of the spec's
/// own literal digest and span, which it agrees with byte for byte.
const FIXTURE_F: &str = "language \"ix:native\" edition \"1-draft\";\nprofile v = \"quire.value.complete/v1\" version \"1\" digest \"sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\";\nfunction f using v(x: Int[0, 9]): Integer pure { x + 5 }\n";

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
    assert_eq!(FIXTURE_F.len(), 229, "fixture F is 229 bytes");
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
                    "sha256:5f2742391e3eaef04bc5dd7141fd639b1913dc821d14bb2f2ca618ad8598ca26"
                );
                assert_eq!(locus.span.start.byte, 225);
                assert_eq!(locus.span.start.line, 3);
                assert_eq!(locus.span.start.column, 54);
                assert_eq!(locus.span.end.byte, 226);
                assert_eq!(locus.span.end.line, 3);
                assert_eq!(locus.span.end.column, 55);
                assert_location(&got_location);
            }
            other => panic!("{other:?}"),
        }
    }

    // Every other kernel refusal but `CheckedInvariant`: bare, with location.
    let kernel_no_record = [
        Refusal::InexactDecimal,
        Refusal::DecimalOutOfDomain,
        Refusal::DivisionPairOutOfDomain {
            quotient_admitted: false,
            remainder_admitted: false,
        },
        Refusal::ModuloOutOfDomain,
        Refusal::TextLengthOutOfDomain,
        Refusal::IntegerOutOfDomain,
        Refusal::RationalOutOfDomain,
        Refusal::IeeeNotExact {
            would_be: quire_exact::IeeeFlags::EMPTY,
        },
        Refusal::IeeeNanPayloadNotRepresentable,
        Refusal::IeeeRationalOutOfDomain,
        Refusal::ForeignReference,
    ];
    for refusal in kernel_no_record {
        match convert(FamilyOutcome::Evaluated(Outcome::Refused(refusal.clone()))).unwrap() {
            CallOutcome::Refused(CallRefusal::Kernel { location: got }) => {
                assert_location(&got);
            }
            other => panic!("{refusal:?}: {other:?}"),
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

    // Kernel undefined: each of the four reasons.
    for (reason, spelling) in [
        (Undefined::DivisionByZero, "division-by-zero"),
        (Undefined::IeeeNotFinite, "ieee-not-finite"),
        (Undefined::EmptyReduction, "empty-reduction"),
        (Undefined::NoneValue, "none-value"),
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
