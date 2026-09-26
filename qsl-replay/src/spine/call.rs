// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-100: `qsl_replay::spine::run`, the spine `run` entry (ADR-011 §5).
//!
//! Compiles a `1-draft` program through the spine ([`super::compile`]) and
//! calls the one function the caller names through S6a
//! (`CheckedPackage::call`), with the arguments the caller supplies. This
//! module's public items name no `qsl_eval` path (FR-100-AC-8): the S6a
//! call itself, and the conversion of its `qsl_eval`-shaped result into the
//! `qsl_foundation`/`qsl_semantics`/`quire_exact` vocabulary below, are both
//! private to this module.
//!
//! **Deferred (QSL-271 scope amendment, 2026-09-26):** FR-096 (QSL-245,
//! PR #465, not yet merged) will change which catalog code and cause
//! spelling a kernel [`quire_exact::Refusal`] gets, and will make
//! `Refusal::CheckedInvariant` an internal fault rather than a refusal
//! record. Converging [`RunRefusal`] with that amendment is out of this
//! change's scope. Until then, every kernel `Outcome::Refused(_)` this
//! module meets -- including `CheckedInvariant` -- is reported through
//! [`RunRefusal::Fault`], the existing internal-fault path (`InternalFault`,
//! `runtime_invariant`) `qsl-replay` already uses for a broken S6a
//! invariant (mirroring `execute::ReplayRefusal::Fault`), not through the
//! `spine-run-result/1` `refused` outcome FR-100 specifies for it. FR-100's
//! kernel-refusal outcome rows (every row keyed on `Outcome::Refused`
//! in the mapping table) are not implemented by this module.

use qsl_foundation::diagnostic::{Code, InternalFault};
use qsl_package::CheckedPackage;
use qsl_semantics::family::{FamilyOutcome, FamilyResult};
use qsl_semantics::library::PackageId;
use qsl_semantics::model::object_environment::ObjectEnvironment;
use quire_exact::{Integer, Meter, Outcome, ScalarLimits, Undefined, Value, ValueType};

use super::{compile, CompileRefusal, DependencyInput, SpineLimits};
use qsl_foundation::SourceIdentity;
use std::collections::BTreeMap;

/// FR-100's default `work_units` (1,000,000) with every other accounting
/// counter at `u64::MAX`.
pub fn default_accounting(work_units: u64) -> ScalarLimits {
    ScalarLimits {
        integer_bits: u64::MAX,
        decimal_digits: u64::MAX,
        scale_expansion: u64::MAX,
        text_input_bytes: u64::MAX,
        text_scalars: u64::MAX,
        normalized_scalars: u64::MAX,
        unit_edges: u64::MAX,
        value_occurrences: u64::MAX,
        work_units,
        result_units: u64::MAX,
    }
}

/// One `{parameter, value}` argument of a [`Call`]: a canonical integer
/// assignment (FR-098's rule), keyed by the parameter's declared name.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CallArgument {
    /// The parameter's declared name.
    pub parameter: String,
    /// The canonical integer assignment: an integer for an integer
    /// parameter, `0`/`1` for a `Boolean` one.
    pub value: i64,
}

/// The `call` member of a 1-draft native-run/1 request (FR-100 Inputs):
/// which function to call, its arguments, and the S6a accounting limits the
/// call runs under.
#[derive(Clone, Debug)]
pub struct Call {
    /// The called function's name: `.`-separated segments, each an
    /// identifier.
    pub function: String,
    /// The call's arguments, one per parameter, in any order.
    pub arguments: Vec<CallArgument>,
    /// The S6a `quire.value.accounting/v1` limits the call charges under.
    pub accounting: ScalarLimits,
}

/// A value [`run`] completed. Always `Boolean` or `Integer`: [`run`] refuses
/// `unsupported_construct` before any call for a function whose declared
/// result is neither.
#[derive(Clone, Debug)]
pub enum CallValue {
    /// `{"kind": "boolean", "value": ...}`.
    Boolean(bool),
    /// `{"kind": "integer", "decimal": ...}` (FR-038's spelling is
    /// `Integer`'s own `Display`).
    Integer(Integer),
}

/// FR-100's outcome mapping (ADR-013 O-16), for every category this module
/// converts. A kernel refusal is not converted here (see the module's
/// deferred note); it is reported through [`RunRefusal::Fault`] instead, so
/// [`run`]'s `Ok` never carries one.
#[derive(Debug)]
pub enum CallOutcome {
    /// `Outcome::Completed(v)`: `completed`, exit 0.
    Completed(CallValue),
    /// `FamilyResult::Refused(c)`: `refused`, `code` `c`'s catalog code, no
    /// `cause`, exit `Code::exit_code()`.
    Refused {
        /// The family refusal's catalog code.
        code: Code,
    },
    /// `Outcome::Undefined(u)` (kebab case) or `FamilyResult::Undefined(u)`
    /// (the catalog's own `UndefinedReason` spelling): `undefined`, exit 20.
    Undefined {
        /// The reason's tabled spelling.
        reason: &'static str,
    },
    /// `Outcome::Incomplete(i)`: `incomplete`, exit 22.
    Incomplete {
        /// The exhausted counter's `quire.value.accounting/v1` member name.
        limit: &'static str,
    },
}

/// Why [`run`] returned no outcome. Every stage-`call` refusal FR-100 names,
/// the wrapped spine compile refusal, and the deferred kernel-refusal/fault
/// path (see the module's deferred note).
#[derive(Debug, thiserror::Error)]
pub enum RunRefusal {
    /// The spine compile refused, at its own stage and cause code.
    #[error("{0}")]
    Compile(Box<CompileRefusal>),
    /// `function` is empty, holds an empty segment or a non-identifier
    /// segment, has more than one segment, or names no function of the
    /// compiled package (`missing_declaration`).
    #[error("no function named {function} is declared")]
    MissingDeclaration {
        /// The request's `function` string.
        function: String,
    },
    /// The named function's declared result is neither `Boolean` nor an
    /// integer type (`unsupported_construct`).
    #[error("the function {function} does not declare a Boolean or integer result")]
    UnsupportedResult {
        /// The request's `function` string.
        function: String,
    },
    /// An argument names no parameter of the called function
    /// (`invalid_runtime_input`).
    #[error("no parameter is named {parameter}")]
    UnknownParameter {
        /// The argument's `parameter` string.
        parameter: String,
    },
    /// Two arguments name the same parameter (`invalid_runtime_input`).
    #[error("the parameter {parameter} is bound twice")]
    DuplicateArgument {
        /// The parameter name bound twice.
        parameter: String,
    },
    /// A parameter of the called function has no argument
    /// (`invalid_runtime_input`).
    #[error("the parameter {parameter} has no argument")]
    UnboundParameter {
        /// The unbound parameter's declared name.
        parameter: String,
    },
    /// An argument's value is not of its parameter's declared type, found
    /// either before the call or by S6a admission (`invalid_runtime_input`).
    #[error("the argument at position {position} is not a value of its parameter's declared type")]
    WrongValueKind {
        /// The parameter's zero-based declared position.
        position: usize,
    },
    /// An S6a invariant broke, or a kernel refusal reached S6a admission
    /// (see the module's deferred note; `runtime_invariant`).
    #[error("internal fault in {}: {}", .0.stage(), .0.invariant())]
    Fault(InternalFault),
}

impl RunRefusal {
    /// The stage FR-100's refusal table names: the wrapped compile
    /// refusal's own stage, or `call` for every refusal this module raises
    /// itself.
    pub fn stage(&self) -> &'static str {
        match self {
            Self::Compile(refusal) => refusal.stage().as_str(),
            Self::MissingDeclaration { .. }
            | Self::UnsupportedResult { .. }
            | Self::UnknownParameter { .. }
            | Self::DuplicateArgument { .. }
            | Self::UnboundParameter { .. }
            | Self::WrongValueKind { .. }
            | Self::Fault(_) => "call",
        }
    }

    /// The catalog code.
    pub fn code(&self) -> Code {
        match self {
            Self::Compile(refusal) => refusal.code(),
            Self::MissingDeclaration { .. } => Code::MissingDeclaration,
            Self::UnsupportedResult { .. } => Code::UnsupportedConstruct,
            Self::UnknownParameter { .. }
            | Self::DuplicateArgument { .. }
            | Self::UnboundParameter { .. }
            | Self::WrongValueKind { .. } => Code::InvalidRuntimeInput,
            Self::Fault(_) => Code::RuntimeInvariant,
        }
    }
}

/// Compile `bytes` through the spine and call `call`'s named function with
/// its arguments, under `call`'s accounting limits. Returns the compiled
/// package's `package_id` and the call's outcome (FR-100).
#[allow(clippy::too_many_arguments)]
pub fn run(
    source: SourceIdentity,
    path: &str,
    bytes: &[u8],
    packages: &BTreeMap<[u8; 32], Vec<u8>>,
    dependencies: &DependencyInput,
    limits: SpineLimits,
    call: &Call,
) -> Result<(PackageId, CallOutcome), Box<RunRefusal>> {
    let compiled = compile(source, path, bytes, packages, dependencies, limits)
        .map_err(|refusal| Box::new(RunRefusal::Compile(refusal)))?;
    let package_id = compiled.emitted.package_id();
    let package = &compiled.package;
    let selected = select(package, &call.function)?;
    let arguments = bind_arguments(selected.parameters, &call.arguments)?;
    let mut meter = Meter::new(call.accounting);
    use qsl_eval::value::CheckedPackageEvaluation;
    let evaluation = package
        .call(
            &selected.name,
            arguments,
            &ObjectEnvironment::default(),
            &mut meter,
        )
        .map_err(convert_call_failure)?;
    Ok((package_id, convert_outcome(evaluation.outcome)?))
}

/// The selected function: its S6a name and declared parameters.
struct Selected<'a> {
    name: qsl_eval::value::QualifiedName,
    parameters: &'a [(String, ValueType)],
}

/// Resolve `function` by one-segment name lookup in `package`'s
/// declarations (OQ-5, as `qsl_replay::replay` does), refusing shapes FR-100
/// names before ever looking one up.
fn select<'a>(
    package: &'a CheckedPackage,
    function: &str,
) -> Result<Selected<'a>, Box<RunRefusal>> {
    let missing = || {
        Box::new(RunRefusal::MissingDeclaration {
            function: function.to_owned(),
        })
    };
    let mut segments = function.split('.');
    let Some(first) = segments.next() else {
        return Err(missing());
    };
    if segments.next().is_some() {
        return Err(missing());
    }
    if first.is_empty() || !is_identifier(first) {
        return Err(missing());
    }
    let callable = package.graph().callable(first).ok_or_else(missing)?;
    if !matches!(
        callable.result,
        ValueType::Boolean | ValueType::Integer | ValueType::Int(_)
    ) {
        return Err(Box::new(RunRefusal::UnsupportedResult {
            function: function.to_owned(),
        }));
    }
    let name = qsl_eval::value::QualifiedName::unqualified(first).map_err(|_| missing())?;
    Ok(Selected {
        name,
        parameters: callable.parameters,
    })
}

/// An ASCII identifier: a letter or `_`, then letters, digits or `_`.
fn is_identifier(segment: &str) -> bool {
    let mut bytes = segment.bytes();
    match bytes.next() {
        Some(byte) if byte.is_ascii_alphabetic() || byte == b'_' => {}
        _ => return false,
    }
    bytes.all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

/// Join each argument to the parameter of the same declared name, ordering
/// values by declared parameter position (FR-100), and convert each to a
/// value of its parameter's declared type.
fn bind_arguments(
    parameters: &[(String, ValueType)],
    arguments: &[CallArgument],
) -> Result<Vec<Value>, Box<RunRefusal>> {
    let mut slots: Vec<Option<i64>> = vec![None; parameters.len()];
    for argument in arguments {
        let position = parameters
            .iter()
            .position(|(name, _)| *name == argument.parameter)
            .ok_or_else(|| {
                Box::new(RunRefusal::UnknownParameter {
                    parameter: argument.parameter.clone(),
                })
            })?;
        let slot = slots.get_mut(position).ok_or_else(|| {
            Box::new(RunRefusal::UnknownParameter {
                parameter: argument.parameter.clone(),
            })
        })?;
        if slot.replace(argument.value).is_some() {
            return Err(Box::new(RunRefusal::DuplicateArgument {
                parameter: argument.parameter.clone(),
            }));
        }
    }
    slots
        .into_iter()
        .zip(parameters)
        .enumerate()
        .map(|(position, (value, (name, value_type)))| {
            let value = value.ok_or_else(|| {
                Box::new(RunRefusal::UnboundParameter {
                    parameter: name.clone(),
                })
            })?;
            argument_value(position, value, value_type)
        })
        .collect()
}

/// A canonical integer assignment as a value of its parameter's declared
/// type (FR-098's rule): an integer for an integer type, `0`/`1` for
/// `Boolean`. Any other value, or a parameter of a kind neither `Boolean`
/// nor an integer type, refuses `WrongValueKind` before the call.
fn argument_value(
    position: usize,
    value: i64,
    value_type: &ValueType,
) -> Result<Value, Box<RunRefusal>> {
    let wrong = || Box::new(RunRefusal::WrongValueKind { position });
    match value_type {
        ValueType::Boolean => match value {
            0 => Ok(Value::Boolean(false)),
            1 => Ok(Value::Boolean(true)),
            _ => Err(wrong()),
        },
        ValueType::Integer | ValueType::Int(_) => Ok(Value::Integer(Integer::from(value))),
        ValueType::Rational(_)
        | ValueType::Decimal(_)
        | ValueType::Float(_)
        | ValueType::Quantity(_)
        | ValueType::Text(_)
        | ValueType::Enum(_)
        | ValueType::Option(_)
        | ValueType::Composite(_)
        | ValueType::Collection(_)
        | ValueType::Reference(_)
        | ValueType::Population(_) => Err(wrong()),
    }
}

/// `CheckedPackage::call`'s admission failure, converted with no `qsl_eval`
/// path in the conversion's own signature (only in this function body).
fn convert_call_failure(failure: qsl_eval::value::CallFailure) -> Box<RunRefusal> {
    use qsl_eval::value::{CallFailure, InputRefusal};
    match failure {
        CallFailure::Input(InputRefusal::WrongValueKind { parameter }) => {
            Box::new(RunRefusal::WrongValueKind {
                position: parameter,
            })
        }
        // `select`/`bind_arguments` already admit the name and arity this
        // module supplies, so no other `InputRefusal` is reachable; treated
        // as an internal fault rather than papered over with a fabricated
        // catalog code.
        CallFailure::Input(_) => Box::new(RunRefusal::Fault(InternalFault::new(
            "call",
            "spine-run-supplies-admitted-name-and-arity",
        ))),
        CallFailure::Fault(fault) => Box::new(RunRefusal::Fault(fault)),
    }
}

/// The kernel's own kebab-case `Undefined` spelling (FR-100).
fn kernel_undefined_reason(reason: Undefined) -> &'static str {
    match reason {
        Undefined::DivisionByZero => "division-by-zero",
        Undefined::IeeeNotFinite => "ieee-not-finite",
        Undefined::EmptyReduction => "empty-reduction",
        Undefined::NoneValue => "none-value",
    }
}

/// FR-100's outcome mapping, for every category this module converts (see
/// the module's deferred note for the kernel-refusal rows it does not).
///
/// FR-063 seam: adding a `FamilyOutcome` variant with no arm here fails
/// `--cfg seam_probe` with `E0004`.
#[deny(clippy::wildcard_enum_match_arm)]
fn convert_outcome(outcome: FamilyOutcome<Value>) -> Result<CallOutcome, Box<RunRefusal>> {
    match outcome {
        FamilyOutcome::Evaluated(Outcome::Completed(Value::Boolean(value))) => {
            Ok(CallOutcome::Completed(CallValue::Boolean(value)))
        }
        FamilyOutcome::Evaluated(Outcome::Completed(Value::Integer(value))) => {
            Ok(CallOutcome::Completed(CallValue::Integer(value)))
        }
        // `select` admits only a function declared `Boolean` or an integer
        // type.
        FamilyOutcome::Evaluated(Outcome::Completed(_)) => Err(Box::new(RunRefusal::Fault(
            InternalFault::new("call", "boolean-or-integer-function-completes-that-kind"),
        ))),
        // Deferred: FR-096 (QSL-245) will define this outcome's catalog code
        // and cause spelling (see the module's deferred note).
        FamilyOutcome::Evaluated(Outcome::Refused(_)) => Err(Box::new(RunRefusal::Fault(
            InternalFault::new("call", "kernel-refusal-pending-fr-096"),
        ))),
        FamilyOutcome::Evaluated(Outcome::Undefined(reason)) => Ok(CallOutcome::Undefined {
            reason: kernel_undefined_reason(reason),
        }),
        FamilyOutcome::Evaluated(Outcome::Incomplete(incomplete)) => Ok(CallOutcome::Incomplete {
            limit: incomplete.limit_kind.as_str(),
        }),
        FamilyOutcome::FamilyEvaluated(FamilyResult::Refused(cause)) => {
            let spelling = cause.catalog_code().code();
            let code = Code::from_code(spelling).ok_or_else(|| {
                Box::new(RunRefusal::Fault(InternalFault::new(
                    "call",
                    "family-refusal-names-a-catalog-code",
                )))
            })?;
            Ok(CallOutcome::Refused { code })
        }
        FamilyOutcome::FamilyEvaluated(FamilyResult::Undefined(cause)) => {
            Ok(CallOutcome::Undefined {
                reason: cause.undefined_record().reason.as_str(),
            })
        }
        // FR-063: no arm for the probe variant under `--cfg seam_probe`
        // alone -- this crate's own seam over `FamilyOutcome` (mirroring
        // `execute::replay`'s). Do not add a catch-all to make it compile.
        #[cfg(seam_probe_replay_downstream)]
        FamilyOutcome::__SeamProbe => {
            unreachable!("never constructed outside the probe build")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spine::DependencyInput;
    use ix_trace_rs::trace;
    use qsl_foundation::diagnostic::{
        CatalogCoded, UndefinedCoded, UndefinedReason, UndefinedRecord,
    };
    use qsl_foundation::SourceIdentity;
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

    const FIXTURE: &str = include_str!("../../../tests/fixtures/spine-run.native");

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
    /// before the call (`flag`, `px`) or at S6a admission (`id`).
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
        assert!(
            matches!(*unknown, RunRefusal::UnknownParameter { ref parameter } if parameter == "y")
        );
        let duplicate =
            run_fixture(FIXTURE, &call("id", vec![arg("x", 1), arg("x", 2)])).unwrap_err();
        assert!(
            matches!(*duplicate, RunRefusal::DuplicateArgument { ref parameter } if parameter == "x")
        );
        let unbound = run_fixture(FIXTURE, &call("id", Vec::new())).unwrap_err();
        assert!(
            matches!(*unbound, RunRefusal::UnboundParameter { ref parameter } if parameter == "x")
        );
        for refusal in [unknown, duplicate, unbound] {
            assert_eq!(refusal.stage(), "call");
            assert_eq!(refusal.code(), Code::InvalidRuntimeInput);
        }
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
    struct StubRefusal(qsl_foundation::diagnostic::CatalogCode);

    impl CatalogCoded for StubRefusal {
        fn catalog_code(&self) -> qsl_foundation::diagnostic::CatalogCode {
            self.0
        }
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

    /// FR-100-AC-9 (TC-452 step 4): the outcome mapping this module owns --
    /// completed, kernel undefined, incomplete, and family refused/undefined
    /// -- converts to the tabled `outcome`/exit. Kernel-refusal rows are out
    /// of scope for this change (QSL-271 amendment); see the module's
    /// deferred note.
    #[trace("TC-452", "FR-100-AC-9")]
    #[test]
    fn tc_452_step_4_outcome_mapping_covers_every_non_kernel_refused_category() {
        use quire_exact::Incomplete;

        for (value, expected) in [(true, "true"), (false, "false")] {
            match convert_outcome(FamilyOutcome::Evaluated(Outcome::Completed(
                Value::Boolean(value),
            )))
            .unwrap()
            {
                CallOutcome::Completed(CallValue::Boolean(rendered)) => {
                    assert_eq!(rendered.to_string(), expected);
                }
                other => panic!("{other:?}"),
            }
        }
        for value in [0i64, -17] {
            match convert_outcome(FamilyOutcome::Evaluated(Outcome::Completed(
                Value::Integer(Integer::from(value)),
            )))
            .unwrap()
            {
                CallOutcome::Completed(CallValue::Integer(rendered)) => {
                    assert_eq!(rendered.to_string(), value.to_string());
                }
                other => panic!("{other:?}"),
            }
        }
        // 2^70, exercised as a magnitude beyond i64.
        let big = Integer::from(1i64).shifted_left(70);
        match convert_outcome(FamilyOutcome::Evaluated(Outcome::Completed(
            Value::Integer(big),
        )))
        .unwrap()
        {
            CallOutcome::Completed(CallValue::Integer(rendered)) => {
                assert_eq!(rendered.to_string(), "1180591620717411303424");
            }
            other => panic!("{other:?}"),
        }

        for (reason, spelling) in [
            (Undefined::DivisionByZero, "division-by-zero"),
            (Undefined::IeeeNotFinite, "ieee-not-finite"),
            (Undefined::EmptyReduction, "empty-reduction"),
            (Undefined::NoneValue, "none-value"),
        ] {
            match convert_outcome(FamilyOutcome::Evaluated(Outcome::Undefined(reason))).unwrap() {
                CallOutcome::Undefined { reason } => assert_eq!(reason, spelling),
                other => panic!("{other:?}"),
            }
        }

        let incomplete = Incomplete {
            limit_kind: quire_exact::LimitKind::WorkUnits,
            limit: 0,
            consumed: 0,
            next_charge: Integer::from(1i64),
            charge_point: quire_exact::ChargePoint::IntegerArithmeticOperands,
        };
        match convert_outcome(FamilyOutcome::Evaluated(Outcome::Incomplete(incomplete))).unwrap() {
            CallOutcome::Incomplete { limit } => assert_eq!(limit, "work_units"),
            other => panic!("{other:?}"),
        }

        let refused_runtime = FamilyResult::Refused(Box::new(StubRefusal(
            qsl_foundation::diagnostic::CatalogCode::new(
                "invalid_runtime_input",
                "wrong-value-kind",
            ),
        )));
        match convert_outcome(FamilyOutcome::FamilyEvaluated(refused_runtime)).unwrap() {
            CallOutcome::Refused { code } => assert_eq!(code, Code::InvalidRuntimeInput),
            other => panic!("{other:?}"),
        }
        let refused_unsupported = FamilyResult::Refused(Box::new(StubRefusal(
            qsl_foundation::diagnostic::CatalogCode::new("unsupported_construct", "some-cause"),
        )));
        match convert_outcome(FamilyOutcome::FamilyEvaluated(refused_unsupported)).unwrap() {
            CallOutcome::Refused { code } => assert_eq!(code, Code::UnsupportedConstruct),
            other => panic!("{other:?}"),
        }

        for reason in [
            UndefinedReason::PreconditionFalse,
            UndefinedReason::AbsentKey,
        ] {
            let undefined = FamilyResult::Undefined(Box::new(StubUndefined(reason)));
            match convert_outcome(FamilyOutcome::FamilyEvaluated(undefined)).unwrap() {
                CallOutcome::Undefined { reason: rendered } => {
                    assert_eq!(rendered, reason.as_str());
                }
                other => panic!("{other:?}"),
            }
        }
    }
}
