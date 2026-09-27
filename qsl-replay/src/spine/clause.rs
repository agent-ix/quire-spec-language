// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-109 (QSL-278): `qsl_replay::spine::run_clause`, the spine's clause-run
//! entry beside [`super::call::run`]'s function-run entry (ADR-011 §5).
//!
//! Compiles a `1-draft` unit through the spine ([`super::compile`]),
//! resolves one state clause or one Boolean function run as a claim by
//! name, admits the supplied observations (FR-106,
//! `qsl_semantics::model::observation`), evaluates through S6a (FR-107's
//! `evaluate_clause`, or `CheckedPackage::call` for a `Function`
//! selection) and returns one [`ClauseRunReport`]: a typed disposition.
//! Reuses FR-100's outcome mapping ([`super::call::convert_outcome`]) by
//! reference, rather than restating it.

use std::collections::BTreeMap;

use qsl_foundation::diagnostic::InternalFault;
use qsl_foundation::source::Source;
use qsl_semantics::library::PackageId;
use qsl_semantics::model::accounting::ModelNormalizationLimits;
use qsl_semantics::model::observation::{
    admit_current_snapshot, admit_observations, population_universe, AdmissionFailure,
    AdmissionRecord, ClauseFacts, ClauseSelection, DocumentRef, ObservationLimits, OperationFacts,
    Provisions,
};
use quire_exact::{Meter, ScalarLimits, Value, ValueType};

use super::call::{convert_call_failure, convert_outcome, select};
pub use super::call::{CallOutcome, CallValue, RunRefusal};
use super::{compile, CompileRefusal, DependencyInput, SpineLimits};

/// FR-109's `Function` selection argument value: FR-100's canonical
/// integer, or an object reference resolved in the selection's current
/// snapshot.
#[derive(Clone, Debug)]
pub enum ClauseArgumentValue {
    /// A canonical integer (FR-100's rule): the integer itself, or `0`/`1`
    /// for a `Boolean` parameter.
    Integer(i64),
    /// `{"reference": {population, key}}`.
    Reference {
        /// The population identity.
        population: String,
        /// The object's declared key.
        key: String,
    },
}

/// One `{parameter, value}` argument of a `Function` selection.
#[derive(Clone, Debug)]
pub struct ClauseArgument {
    /// The parameter's declared name.
    pub parameter: String,
    /// The argument's value.
    pub value: ClauseArgumentValue,
}

/// FR-109's selection: a state clause (FR-106's [`ClauseSelection`]), or a
/// Boolean function run as a claim.
pub enum ClauseRunSelection {
    /// A state clause, admitted per FR-106.
    Clause(ClauseSelection),
    /// A function, called as a claim (FR-109's own `Function` selection).
    Function {
        /// The function's name (FR-100's `function` rule; one segment,
        /// resolved by [`super::call::select`]).
        name: String,
        /// The call's arguments, one per parameter, in any order.
        arguments: Vec<ClauseArgument>,
        /// The current snapshot each object argument resolves against.
        snapshot: DocumentRef,
    },
}

/// FR-109 Inputs: one clause-run request.
pub struct ClauseRunRequest {
    /// The unit's source identity.
    pub source: qsl_foundation::SourceIdentity,
    /// The source's display path.
    pub path: String,
    /// The unit's source bytes.
    pub bytes: Vec<u8>,
    /// FR-056's package input.
    pub packages: BTreeMap<[u8; 32], Vec<u8>>,
    /// FR-099's dependency input.
    pub dependencies: DependencyInput,
    /// FR-106's snapshot provision.
    pub snapshots: BTreeMap<[u8; 32], Vec<u8>>,
    /// FR-106's invocation provision.
    pub invocations: BTreeMap<[u8; 32], Vec<u8>>,
    /// The selection.
    pub selection: ClauseRunSelection,
    /// An optional expected `package_id` (FR-098's stale package rule).
    pub expected_package_id: Option<PackageId>,
    /// The spine's stage limits.
    pub limits: SpineLimits,
    /// FR-106's observation limits.
    pub observation_limits: ObservationLimits,
    /// The limits admission re-normalizes the unit's domain packages under
    /// (see `qsl_semantics::model::observation`'s own module doc).
    pub model_limits: ModelNormalizationLimits,
    /// The evaluation meter's accounting limits (FR-100's `work_units`).
    pub accounting: ScalarLimits,
}

/// Why [`run_clause`] returned no report: a request that cannot be formed
/// (FR-109 Outputs). Every compile, selection, admission or evaluation
/// result is a report, never this.
#[derive(Debug, thiserror::Error)]
pub enum ClauseRunRefusal {
    /// The unit's source is empty.
    #[error("the unit source is empty")]
    EmptySource,
}

/// FR-109's disposition stage.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClauseRunStage {
    /// Compile (S1 to S4).
    Compile,
    /// Name resolution.
    Select,
    /// FR-106 admission.
    Admit,
    /// S6a evaluation.
    Evaluate,
}

/// FR-109's disposition: one record per non-`Evaluate` stage, or the S6a
/// outcome (FR-100's mapping, reused unchanged) for `Evaluate`.
#[derive(Debug)]
pub enum ClauseDisposition {
    /// Stage `compile`: the spine compile refused, naming its own cause
    /// code.
    Compile(Box<CompileRefusal>),
    /// Stage `compile`, category `refusal`, `stale_dependency`: the
    /// recompiled `package_id` differs from the request's expected one.
    StalePackage {
        /// The expected `package_id`.
        expected: PackageId,
        /// The recompiled `package_id`.
        actual: PackageId,
    },
    /// Stage `select`, `missing_declaration`/`missing-name`.
    MissingName {
        /// The selected name.
        name: String,
    },
    /// Stage `select`, `ill_typed`/`type-mismatch`: a `Function` selection
    /// names a function whose declared result is not `Boolean`.
    NotAPredicate {
        /// The selected name.
        name: String,
    },
    /// Stage `admit`: FR-106 admission's own failure, or (for a `Function`
    /// selection) the analogous snapshot-only admission failure.
    Admit(AdmissionFailure),
    /// Stage `admit`: a `Function` selection's argument refused before any
    /// call (FR-100's own refusal, reused).
    ArgumentRefusal(Box<RunRefusal>),
    /// Stage `evaluate`: the mapped S6a outcome.
    Evaluate(CallOutcome),
    /// Stage `evaluate`, category `internal-failure`: a broken S6a
    /// invariant reached outside the outcome mapping (FR-100's own
    /// internal-failure exit status).
    EvaluateFault(qsl_foundation::diagnostic::InternalFault),
}

impl ClauseDisposition {
    /// The FR-109 stage this disposition reports at.
    pub fn stage(&self) -> ClauseRunStage {
        match self {
            Self::Compile(_) | Self::StalePackage { .. } => ClauseRunStage::Compile,
            Self::MissingName { .. } | Self::NotAPredicate { .. } => ClauseRunStage::Select,
            Self::Admit(_) | Self::ArgumentRefusal(_) => ClauseRunStage::Admit,
            Self::Evaluate(_) | Self::EvaluateFault(_) => ClauseRunStage::Evaluate,
        }
    }
}

/// FR-109's `ClauseRunReport`: the disposition, with the source's own
/// digest and, where compile reached it, the compiled `package_id`.
#[derive(Debug)]
pub struct ClauseRunReport {
    /// The source's `sha256:` digest.
    pub source_digest: String,
    /// The compiled `package_id`, when compile completed.
    pub package_id: Option<PackageId>,
    /// The disposition.
    pub disposition: ClauseDisposition,
}

impl ClauseRunReport {
    /// FR-109: one total match over the stage and category, no `_` arm.
    pub fn exit_code(&self) -> u8 {
        use qsl_foundation::diagnostic::Code;
        let code_of = |code: &str| Code::all().iter().find(|c| c.as_str() == code).copied();
        match &self.disposition {
            ClauseDisposition::Compile(refusal) => refusal.code().exit_code(),
            ClauseDisposition::StalePackage { .. } => Code::StaleDependency.exit_code(),
            ClauseDisposition::MissingName { .. } => Code::MissingDeclaration.exit_code(),
            ClauseDisposition::NotAPredicate { .. } => Code::IllTyped.exit_code(),
            ClauseDisposition::Admit(AdmissionFailure::Fault(_)) => 30,
            ClauseDisposition::Admit(
                AdmissionFailure::Refused(record) | AdmissionFailure::Incomplete(record),
            ) => code_of(record.code).map_or(20, Code::exit_code),
            ClauseDisposition::ArgumentRefusal(refusal) => match refusal.as_ref() {
                RunRefusal::Fault(_) => 30,
                other => other.code().exit_code(),
            },
            ClauseDisposition::Evaluate(outcome) => evaluate_exit_code(outcome),
            ClauseDisposition::EvaluateFault(_) => 30,
        }
    }
}

fn evaluate_exit_code(outcome: &CallOutcome) -> u8 {
    use qsl_foundation::diagnostic::Code;
    let code_of = |code: &str| Code::all().iter().find(|c| c.as_str() == code).copied();
    match outcome {
        CallOutcome::Completed(CallValue::Boolean(true)) => 0,
        CallOutcome::Completed(CallValue::Boolean(false)) => 10,
        CallOutcome::Completed(CallValue::Integer(_)) => 0,
        CallOutcome::Refused(super::call::CallRefusal::Record { code, .. }) => {
            code_of(code.code()).map_or(20, |c| c.exit_code())
        }
        CallOutcome::Refused(super::call::CallRefusal::Family { code, .. }) => {
            code_of(code.code()).map_or(20, |c| c.exit_code())
        }
        CallOutcome::Undefined { .. } => 20,
        CallOutcome::Incomplete { .. } => 22,
    }
}

/// FR-109: `qsl_replay::spine::run_clause`.
pub fn run_clause(request: ClauseRunRequest) -> Result<ClauseRunReport, ClauseRunRefusal> {
    if request.bytes.is_empty() {
        return Err(ClauseRunRefusal::EmptySource);
    }
    let source_digest = qsl_foundation::ByteDigest::of(&request.bytes).to_string();
    let report = |package_id, disposition| ClauseRunReport {
        source_digest: source_digest.clone(),
        package_id,
        disposition,
    };

    let compiled = match compile(
        request.source.clone(),
        &request.path,
        &request.bytes,
        &request.packages,
        &request.dependencies,
        request.limits,
    ) {
        Ok(compiled) => compiled,
        Err(refusal) => return Ok(report(None, ClauseDisposition::Compile(refusal))),
    };
    let package_id = compiled.emitted.package_id();
    if let Some(expected) = request.expected_package_id {
        if package_id != expected {
            return Ok(report(
                Some(package_id),
                ClauseDisposition::StalePackage {
                    expected,
                    actual: package_id,
                },
            ));
        }
    }
    let package = &compiled.package;
    // FND-016: the unit's and every resolved library's source, reused
    // exactly as `compile` already read them (mirroring `call::run`'s own
    // `sources` build), so a locus resolves without a second read of
    // already-admitted bytes.
    let mut sources = Vec::with_capacity(1 + compiled.libraries.len());
    sources.push(compiled.source.clone());
    sources.extend(compiled.libraries.iter().cloned());

    match request.selection {
        ClauseRunSelection::Clause(selection) => {
            let Some(clause) = package.graph().state_clause(&selection.name) else {
                return Ok(report(
                    Some(package_id),
                    ClauseDisposition::MissingName {
                        name: selection.name,
                    },
                ));
            };
            let provisions = Provisions {
                snapshots: &request.snapshots,
                invocations: &request.invocations,
            };
            // The model -> check edge must stay empty (FR-074-AC-3):
            // `admit_observations` (FR-106) is a `model`-layer function, so
            // this caller reads the checked clause's own facts out of
            // `check::CheckedStateClause` here, into model-level
            // `ClauseFacts`/`OperationFacts`, rather than that module
            // importing `check` itself.
            let clause_facts = ClauseFacts {
                identity: clause.identity(),
                kind: clause.kind(),
                context: clause.context(),
                operation: clause.operation().map(|operation| OperationFacts {
                    declaring: operation.declaring,
                    declaration: operation.declaration.clone(),
                }),
            };
            let observations = match admit_observations(
                package.graph().model_selections(),
                package.graph().scope().types(),
                &clause_facts,
                &request.packages,
                request.model_limits,
                &provisions,
                &selection,
                request.observation_limits,
            ) {
                Ok(observations) => observations,
                Err(failure) => {
                    return Ok(report(Some(package_id), ClauseDisposition::Admit(failure)))
                }
            };
            use qsl_eval::value::{CheckedPackageEvaluation, QualifiedName};
            let Ok(qualified) = QualifiedName::unqualified(&selection.name) else {
                return Ok(report(
                    Some(package_id),
                    ClauseDisposition::MissingName {
                        name: selection.name,
                    },
                ));
            };
            let mut meter = Meter::new(request.accounting);
            match package.evaluate_clause(&qualified, &observations, &mut meter) {
                Ok(evaluation) => match convert_outcome(evaluation, package.graph(), &sources) {
                    Ok(outcome) => Ok(report(
                        Some(package_id),
                        ClauseDisposition::Evaluate(outcome),
                    )),
                    Err(refusal) => Ok(report(
                        Some(package_id),
                        match *refusal {
                            RunRefusal::Fault(fault) => ClauseDisposition::EvaluateFault(fault),
                            other => ClauseDisposition::ArgumentRefusal(Box::new(other)),
                        },
                    )),
                },
                Err(qsl_eval::value::CallFailure::Input(input)) => {
                    use qsl_eval::value::InputRefusal;
                    Ok(report(
                        Some(package_id),
                        match input {
                            InputRefusal::UnknownClause(name) => {
                                ClauseDisposition::MissingName { name }
                            }
                            // Unreachable in this flow: this module resolves the
                            // named clause once (above) and admits `observations`
                            // against that same clause's identity, so
                            // `evaluate_clause`'s own name/identity checks can
                            // never disagree with it. Treated as a broken
                            // invariant rather than a document defect, never a
                            // wildcard arm (FR-090).
                            InputRefusal::ObservationsMismatch => ClauseDisposition::EvaluateFault(
                                InternalFault::new("call", "clause-observations-mismatch"),
                            ),
                            InputRefusal::UnknownFunction(_)
                            | InputRefusal::Arity { .. }
                            | InputRefusal::WrongValueKind { .. }
                            | InputRefusal::DanglingReference { .. } => {
                                ClauseDisposition::EvaluateFault(InternalFault::new(
                                    "call",
                                    "evaluate-clause-supplies-no-function-style-input",
                                ))
                            }
                        },
                    ))
                }
                Err(qsl_eval::value::CallFailure::Fault(fault)) => Ok(report(
                    Some(package_id),
                    ClauseDisposition::EvaluateFault(fault),
                )),
            }
        }
        ClauseRunSelection::Function {
            name,
            arguments,
            snapshot,
        } => run_function(
            &request.packages,
            request.model_limits,
            &request.snapshots,
            request.observation_limits,
            request.accounting,
            package,
            package_id,
            &name,
            &arguments,
            &snapshot,
            &source_digest,
            &sources,
        ),
    }
}

#[allow(clippy::too_many_arguments)]
fn run_function(
    packages: &BTreeMap<[u8; 32], Vec<u8>>,
    model_limits: ModelNormalizationLimits,
    snapshots: &BTreeMap<[u8; 32], Vec<u8>>,
    observation_limits: ObservationLimits,
    accounting: ScalarLimits,
    package: &qsl_package::CheckedPackage,
    package_id: PackageId,
    name: &str,
    arguments: &[ClauseArgument],
    snapshot: &DocumentRef,
    source_digest: &str,
    sources: &[Source],
) -> Result<ClauseRunReport, ClauseRunRefusal> {
    use qsl_eval::value::CheckedPackageEvaluation;
    let report = |disposition| ClauseRunReport {
        source_digest: source_digest.to_owned(),
        package_id: Some(package_id),
        disposition,
    };

    let selected = match select(package, name) {
        Ok(selected) => selected,
        Err(refusal) => {
            return Ok(match *refusal {
                RunRefusal::MissingDeclaration { function } => {
                    report(ClauseDisposition::MissingName { name: function })
                }
                RunRefusal::UnsupportedResult { function } => {
                    report(ClauseDisposition::NotAPredicate { name: function })
                }
                other => report(ClauseDisposition::ArgumentRefusal(Box::new(other))),
            })
        }
    };
    if !matches!(
        package
            .graph()
            .callable(name)
            .map(|callable| callable.result),
        Some(ValueType::Boolean)
    ) {
        return Ok(report(ClauseDisposition::NotAPredicate {
            name: name.to_owned(),
        }));
    }

    let environment = match admit_current_snapshot(
        package.graph().model_selections(),
        package.graph().scope().types(),
        packages,
        model_limits,
        snapshots,
        snapshot,
        observation_limits,
    ) {
        Ok(environment) => environment,
        Err(failure) => return Ok(report(ClauseDisposition::Admit(failure))),
    };

    // Bind each argument to its parameter (FR-100's own name-binding
    // refusals), then convert to a value: a canonical integer as FR-100
    // does, or an object reference resolved in `environment`.
    let mut slots: Vec<Option<&ClauseArgumentValue>> = vec![None; selected.parameters.len()];
    for argument in arguments {
        let Some(position) = selected
            .parameters
            .iter()
            .position(|(parameter, _)| *parameter == argument.parameter)
        else {
            return Ok(report(ClauseDisposition::ArgumentRefusal(Box::new(
                RunRefusal::UnknownParameter {
                    parameter: argument.parameter.clone(),
                },
            ))));
        };
        if slots[position].replace(&argument.value).is_some() {
            return Ok(report(ClauseDisposition::ArgumentRefusal(Box::new(
                RunRefusal::DuplicateArgument {
                    parameter: argument.parameter.clone(),
                },
            ))));
        }
    }
    let mut bound = Vec::with_capacity(selected.parameters.len());
    for (position, (parameter, value_type)) in selected.parameters.iter().enumerate() {
        let Some(value) = slots[position] else {
            return Ok(report(ClauseDisposition::ArgumentRefusal(Box::new(
                RunRefusal::UnboundParameter {
                    parameter: parameter.clone(),
                },
            ))));
        };
        let converted = match (value, value_type) {
            (ClauseArgumentValue::Integer(0), ValueType::Boolean) => Value::Boolean(false),
            (ClauseArgumentValue::Integer(1), ValueType::Boolean) => Value::Boolean(true),
            (ClauseArgumentValue::Integer(value), ValueType::Integer | ValueType::Int(_)) => {
                Value::Integer(quire_exact::Integer::from(*value))
            }
            (ClauseArgumentValue::Reference { population, key }, ValueType::Reference(_)) => {
                let universe = population_universe(population);
                let Some(reference) = environment.find(universe, key) else {
                    let mut fields = BTreeMap::new();
                    fields.insert("population", population.clone());
                    fields.insert("object", key.clone());
                    return Ok(report(ClauseDisposition::Admit(AdmissionFailure::Refused(
                        AdmissionRecord {
                            code: "invalid_runtime_input",
                            cause: "wrong-role-mapping",
                            fields,
                        },
                    ))));
                };
                Value::Reference(reference.clone())
            }
            _ => {
                return Ok(report(ClauseDisposition::ArgumentRefusal(Box::new(
                    RunRefusal::WrongValueKind { position },
                ))))
            }
        };
        bound.push(converted);
    }

    let mut meter = Meter::new(accounting);
    match package.call(&selected.name, bound, &environment, &mut meter) {
        Ok(evaluation) => match convert_outcome(evaluation, package.graph(), sources) {
            Ok(outcome) => Ok(report(ClauseDisposition::Evaluate(outcome))),
            Err(refusal) => Ok(report(match *refusal {
                RunRefusal::Fault(fault) => ClauseDisposition::EvaluateFault(fault),
                other => ClauseDisposition::ArgumentRefusal(Box::new(other)),
            })),
        },
        Err(failure) => Ok(report(match *convert_call_failure(failure) {
            RunRefusal::Fault(fault) => ClauseDisposition::EvaluateFault(fault),
            other => ClauseDisposition::ArgumentRefusal(Box::new(other)),
        })),
    }
}

#[cfg(test)]
mod tests;
