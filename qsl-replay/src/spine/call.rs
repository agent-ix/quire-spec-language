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
//! FR-096 (QSL-245, PR #465/#469) governs how a refused outcome renders: a
//! refusal with an FR-096 record renders its `code`, `cause`, `fields` and
//! `locus`; a family refusal with no record renders `code`/`cause` alone; a
//! kernel refusal with no record renders bare; `CheckedInvariant` and a
//! broken S6a invariant are internal failures ([`RunRefusal::Fault`]), never
//! a `refused` outcome.

use std::collections::BTreeMap;

use qsl_foundation::diagnostic::{CatalogCode, Code, InternalFault, Locus, RefusalRecord};
use qsl_foundation::source::Source;
use qsl_package::CheckedPackage;
use qsl_semantics::check::{CheckedGraph, Location};
use qsl_semantics::family::{FamilyOutcome, FamilyResult};
use qsl_semantics::library::PackageId;
use qsl_semantics::model::object_environment::ObjectEnvironment;
use quire_exact::{Integer, Meter, Outcome, Refusal, ScalarLimits, Undefined, Value, ValueType};

use super::{compile, CompileRefusal, DependencyInput, SpineLimits};
use qsl_foundation::SourceIdentity;

/// FR-100: the default `work_units` limit when a request omits it.
pub const DEFAULT_WORK_UNITS: u64 = 1_000_000;

/// FR-100's default `work_units` with every other accounting counter at
/// `u64::MAX`.
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

/// A refusal's [`Locus::Region`], resolved to the `sha256:` digest of the
/// source it points into and its span rendered over that source's bytes
/// (FR-096, FR-100).
#[derive(Clone, Debug)]
pub struct CallLocus {
    /// The `sha256:` digest of the source the region belongs to.
    pub source_digest: String,
    /// The region's span, rendered over that source.
    pub span: qsl_foundation::LocatedSpan,
}

/// FR-100's `refused` outcome, one variant per refusal-table row (ADR-013
/// O-17, FR-096).
#[derive(Debug)]
pub enum CallRefusal {
    /// FR-096 builds a `RefusalRecord` for this refusal, kernel or family
    /// (`kernel_refusal_record`/`CatalogCoded::refusal_record`).
    Record {
        /// The record's catalog code and cause.
        code: CatalogCode,
        /// The record's catalog fields, keyed by the catalog's payload
        /// names.
        fields: BTreeMap<&'static str, String>,
        /// The record's locus, resolved over a supplied source, when it
        /// carries one.
        locus: Option<CallLocus>,
        /// Where the outcome arose, when known.
        location: Option<Location>,
    },
    /// A family cause with no FR-096 key-table row: its catalog code and
    /// cause, no fields.
    Family {
        /// The cause's catalog code and cause.
        code: CatalogCode,
        /// Where the outcome arose, when known.
        location: Option<Location>,
    },
    /// A kernel refusal other than `CardinalityOutOfBound` and
    /// `CheckedInvariant`: no code, cause or fields.
    Kernel {
        /// Where the outcome arose, when known.
        location: Option<Location>,
    },
}

/// FR-100's outcome mapping (ADR-013 O-16), for every category
/// `qsl_replay::spine::run` returns as `Ok`. `CheckedInvariant` and a broken
/// S6a invariant are never carried here (see [`RunRefusal::Fault`]).
#[derive(Debug)]
pub enum CallOutcome {
    /// `Outcome::Completed(v)`: `completed`, exit 0.
    Completed(CallValue),
    /// `refused`, one of the refusal table's three rows, exit by the
    /// mapping's own rule.
    Refused(CallRefusal),
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

/// Why [`run`] returned no outcome: a spine compile refusal, every
/// stage-`call` refusal FR-100 names, or an internal failure (a broken S6a
/// invariant, the kernel `CheckedInvariant` refusal, or a record's locus
/// naming no supplied source).
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
    /// An internal failure at S6a (FR-100 "Internal failure at S6a"): the
    /// kernel `CheckedInvariant` refusal, `CallFailure::Fault`, or a
    /// record's locus naming no supplied source. Never `Code::exit_code()`;
    /// the caller exits 30 directly.
    #[error("internal fault in {}: {}", .0.stage(), .0.invariant())]
    Fault(InternalFault),
}

impl RunRefusal {
    /// The stage FR-100's refusal table names: the wrapped compile
    /// refusal's own stage, or `call` for every refusal this module raises
    /// itself, including [`Self::Fault`] (the internal-failure envelope's
    /// own stage is `call`, FR-100).
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

    /// The catalog code. For [`Self::Fault`] this is always
    /// `runtime_invariant`; its exit status is not `Code::exit_code()`
    /// (FR-100), which the caller must apply directly rather than through
    /// this code.
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
pub fn run(
    source: SourceIdentity,
    path: &str,
    bytes: &[u8],
    packages: &BTreeMap<[u8; 32], Vec<u8>>,
    dependencies: &DependencyInput,
    limits: SpineLimits,
    call: &Call,
) -> Result<(PackageId, CallOutcome), Box<RunRefusal>> {
    let compiled = compile(source.clone(), path, bytes, packages, dependencies, limits)
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
    let sources = supplied_sources(source, path, bytes, dependencies, limits)?;
    let outcome = convert_outcome(evaluation, package.graph(), &sources)?;
    Ok((package_id, outcome))
}

/// The program's source and every supplied library's source, each read
/// exactly as [`compile`] read it (FR-100: a locus is resolved over "the
/// `Source` whose reference equals the region's reference"). Never fails in
/// practice, since `compile` already admitted these same bytes under the
/// same limits; a read failure here is defensive and reported the same way
/// as an unresolvable locus (FR-100 "Internal failure at S6a").
fn supplied_sources(
    source: SourceIdentity,
    path: &str,
    bytes: &[u8],
    dependencies: &DependencyInput,
    limits: SpineLimits,
) -> Result<Vec<Source>, Box<RunRefusal>> {
    let byte_limit = limits.source.source_bytes;
    let unresolved = || Box::new(RunRefusal::Fault(InternalFault::new(
        "spine-run",
        "locus-source-supplied",
    )));
    let mut sources = Vec::with_capacity(1 + dependencies.libraries.len());
    sources.push(Source::read(source, path.to_owned(), bytes, byte_limit).map_err(|_| unresolved())?);
    for library in dependencies.libraries.values() {
        sources.push(
            Source::read(
                library.source.clone(),
                library.path.clone(),
                &library.bytes,
                byte_limit,
            )
            .map_err(|_| unresolved())?,
        );
    }
    Ok(sources)
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

/// Join each argument to the parameter of the same declared name (refusing
/// an unknown or twice-bound parameter, and an unbound one, before any value
/// is converted -- FND-010: binding order is checked in full before any
/// value's kind is), then convert each bound value to its parameter's
/// declared type, ordered by declared parameter position.
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
    // Every parameter is bound before any is converted: an unbound
    // parameter at a later position refuses before a wrong-kind value at an
    // earlier one is ever inspected (FND-010).
    let bound: Vec<i64> = slots
        .into_iter()
        .zip(parameters)
        .map(|(value, (name, _))| {
            value.ok_or_else(|| {
                Box::new(RunRefusal::UnboundParameter {
                    parameter: name.clone(),
                })
            })
        })
        .collect::<Result<_, _>>()?;
    bound
        .into_iter()
        .zip(parameters)
        .enumerate()
        .map(|(position, (value, (_, value_type)))| argument_value(position, value, value_type))
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

/// Resolve an FR-096 record's [`Locus`] over `sources`: a region locus
/// renders as the digest and span of the one supplied source whose
/// reference equals the region's (FR-100). Every other locus shape, and a
/// region matching no supplied source, is an internal failure ("a locus
/// naming no supplied source is an internal failure", FR-100).
fn resolve_locus(locus: &Locus, sources: &[Source]) -> Result<CallLocus, Box<RunRefusal>> {
    let unresolved = || {
        Box::new(RunRefusal::Fault(InternalFault::new(
            "spine-run",
            "locus-source-supplied",
        )))
    };
    let Locus::Region(region) = locus else {
        return Err(unresolved());
    };
    let source = sources
        .iter()
        .find(|source| source.reference() == region.source())
        .ok_or_else(unresolved)?;
    let span = source.render(region).ok_or_else(unresolved)?;
    Ok(CallLocus {
        source_digest: format!("sha256:{}", region.source().digest().hex()),
        span,
    })
}

/// Build [`CallRefusal`] from a refusal's FR-096 record (when FR-096 built
/// one), its fallback catalog code (family causes with no record; `None` for
/// a kernel refusal), and its location.
fn convert_refusal(
    record: Option<RefusalRecord>,
    fallback: Option<CatalogCode>,
    location: Option<Location>,
    sources: &[Source],
) -> Result<CallRefusal, Box<RunRefusal>> {
    if let Some(record) = record {
        let locus = record
            .locus()
            .map(|locus| resolve_locus(locus, sources))
            .transpose()?;
        return Ok(CallRefusal::Record {
            code: record.code(),
            fields: record.fields().clone(),
            locus,
            location,
        });
    }
    Ok(match fallback {
        Some(code) => CallRefusal::Family { code, location },
        None => CallRefusal::Kernel { location },
    })
}

/// FR-100's outcome mapping, for every category `run` returns as `Ok`
/// (ADR-013 O-16). `CheckedInvariant` and a broken S6a invariant return
/// `Err(RunRefusal::Fault(_))` instead (FR-100 "Internal failure at S6a").
///
/// FR-063 seam: adding a `FamilyOutcome` variant with no arm here fails
/// `--cfg seam_probe` with `E0004`.
#[deny(clippy::wildcard_enum_match_arm)]
fn convert_outcome(
    evaluation: qsl_eval::value::Evaluation,
    graph: &CheckedGraph,
    sources: &[Source],
) -> Result<CallOutcome, Box<RunRefusal>> {
    let record = evaluation.refusal_record(graph);
    let qsl_eval::value::Evaluation {
        outcome, location, ..
    } = evaluation;
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
        FamilyOutcome::Evaluated(Outcome::Refused(Refusal::CheckedInvariant)) => {
            Err(Box::new(RunRefusal::Fault(InternalFault::new(
                "S6a",
                "checked-program-invariant",
            ))))
        }
        FamilyOutcome::Evaluated(Outcome::Refused(_)) => Ok(CallOutcome::Refused(
            convert_refusal(record, None, location, sources)?,
        )),
        FamilyOutcome::Evaluated(Outcome::Undefined(reason)) => Ok(CallOutcome::Undefined {
            reason: kernel_undefined_reason(reason),
        }),
        FamilyOutcome::Evaluated(Outcome::Incomplete(incomplete)) => Ok(CallOutcome::Incomplete {
            limit: incomplete.limit_kind.as_str(),
        }),
        FamilyOutcome::FamilyEvaluated(FamilyResult::Refused(cause)) => {
            let fallback = cause.catalog_code();
            Ok(CallOutcome::Refused(convert_refusal(
                record,
                Some(fallback),
                location,
                sources,
            )?))
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
mod tests;
