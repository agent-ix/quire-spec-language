// SPDX-License-Identifier: AGPL-3.0-or-later
//! The `operation-contract` check arm (QSpec FR-483, FR-485): decide a
//! claim over a checked package's model by evaluating every case of the
//! subject's finite universes through the native evaluator.
//!
//! * An `invariant` holds in every initial state and is preserved by every
//!   operation of its context from every pre-state of the declared types
//!   that satisfies the pre-state premises.
//! * A `pre` or `post` clause of an operation with no implementation binding
//!   is defined at every case that satisfies its premises.
//! * A `simulation` item's initial-state rule holds at every initial state,
//!   and its step row's rule at every step from every pre-state of the
//!   declared types, through the refinement check's own step decision.
//! * A bound item, and a `frame` item, are `unsupported`: the arm holds no
//!   implementation engine, and an unbound frame has no contract-level
//!   reading (FR-485 "The oracle", "Frame").
//! * A proof is basis `exhaustive`, `uncertified`; a violation is `refuted`
//!   once the native replay reproduces it, else `inconclusive`
//!   `replay-parity`; a case set no premise admits is `inconclusive`
//!   `kani_vacuous_proof`.
//! * One case is one initial state or pre-state evaluated. The count is
//!   `check.limits.max_cases` or the descriptor default; reaching it before
//!   the cases are exhausted is `incomplete` `limit-reached`.
//!
//! [`contract_item`] is the only constructor of the arm's work.

pub mod manifest;

use std::num::NonZeroU64;

use qsl_eval::simulation::model::{ModelSimulationRequest, ModelSystem};
use qsl_eval::simulation::{ExpansionLimit, ExpansionStop};
use qsl_foundation::RequestIndex;
use qsl_package::CheckedPackage;
use qsl_replay::contract_check::{
    CaseVerdict, ContractCase, ContractCheckUnsupported, ContractClaim, ContractCounterexample,
    ContractForm, ContractSelection, ContractTarget, ReproducedContractViolation,
};
use qsl_replay::{open_systems, Cancel, DeclineCode, ProofRefusalCause, SubjectRefusal};
use qsl_route::Mode;
use qsl_semantics::check::Capability;
use qsl_semantics::model::observation::DocumentRef;
use quire_exact::{CancelCause, ScalarLimits};
use quire_semantic_value::catalog::{CatalogCode, InternalFault};

use crate::analyze::{AnalyzeItem, EngineReport, Refusal};

/// What the arm found, before settlement.
#[derive(Debug)]
pub enum ContractOutcome {
    /// The goal holds at every check of all `cases`.
    Proved {
        /// The cases evaluated.
        cases: u64,
    },
    /// A violation the native replay reproduced.
    Refuted(Box<ReproducedContractViolation>),
    /// A candidate violation the native replay did not reproduce.
    ReplayParity,
    /// No case satisfied the premises: zero checks.
    Vacuous,
    /// A premise or the goal decided no Boolean, and no violation was found.
    Undecided,
    /// The case budget was reached before the cases were exhausted.
    MaxCases(u64),
    /// The caller cancelled, or its deadline ended the run.
    Cancelled(CancelCause),
    /// A budget of the evaluation stopped it: the stop, and the meter and
    /// candidate bound it ran under.
    Stopped {
        /// The stop.
        stop: ExpansionStop,
        /// The evaluation meter's limits.
        meter: ScalarLimits,
        /// The per-expansion candidate bound.
        max_candidates: u64,
    },
}

/// Why the arm settles without a check.
#[derive(Debug)]
pub enum ContractRefusal {
    /// The form or an element it reads lies outside what the arm decides.
    Unsupported(ContractForm, ContractCheckUnsupported),
    /// The subject's model system does not admit.
    Admission,
    /// A broken invariant.
    Fault(InternalFault),
}

/// The abstract side of a simulation item: the package that holds the
/// abstract model and the abstract subject's initial snapshots.
pub struct AbstractSubject<'a> {
    /// The package that holds the abstract model.
    pub package: &'a CheckedPackage,
    /// The abstract subject's FR-106 initial snapshots.
    pub initial: Vec<DocumentRef>,
}

/// One operation-contract request item: the claim, its subject and its case
/// budget.
pub struct ContractRequest<'a> {
    /// The checked package the claim is over.
    pub package: &'a CheckedPackage,
    /// The claim's form and target.
    pub claim: ContractClaim,
    /// The concrete subject: the model, its initial states and the
    /// universes that bound its populations.
    pub subject: ModelSimulationRequest<'a>,
    /// The abstract subject of a simulation item; `None` otherwise.
    pub abstract_subject: Option<AbstractSubject<'a>>,
    /// The item's `check.limits.max_cases`; absent takes the descriptor
    /// default ([`manifest::DEFAULT_MAX_CASES`]).
    pub max_cases: Option<NonZeroU64>,
}

/// Decide `request` (QSpec FR-483, FR-485).
///
/// # Errors
///
/// [`ContractRefusal`] when the item settles without a check.
pub fn check_contract(
    request: ContractRequest<'_>,
    cancel: &Cancel,
) -> Result<ContractOutcome, ContractRefusal> {
    let form = request.claim.form();
    let meter = request.subject.meter;
    let max_candidates = request.subject.expansion.max_candidates;
    let budget = request
        .max_cases
        .unwrap_or(manifest::DEFAULT_MAX_CASES)
        .get();
    let (concrete, abstract_system) = open(&request)?;
    let target = match ContractTarget::new(
        request.claim,
        request.package,
        &concrete,
        abstract_system.as_ref(),
    )
    .map_err(ContractRefusal::Fault)?
    {
        Ok(target) => target,
        Err(reason) => return Err(ContractRefusal::Unsupported(form, reason)),
    };
    let declared = target
        .declared_cases()
        .map_err(|reason| ContractRefusal::Unsupported(form, reason))?;
    let initial = target.initial_cases().into_iter().map(Ok);
    let declared = declared
        .into_iter()
        .flatten()
        .map(|state| state.map(ContractCase::Declared));
    let stopped = |stop| ContractOutcome::Stopped {
        stop,
        meter,
        max_candidates,
    };
    let mut cases = 0_u64;
    let mut checked = false;
    let mut undecided = false;
    for case in initial.chain(declared) {
        if let Some(cause) = cancel.tripped() {
            return Ok(ContractOutcome::Cancelled(cause));
        }
        if cases >= budget {
            return Ok(ContractOutcome::MaxCases(budget));
        }
        cases += 1;
        let case = match case {
            Ok(case) => case,
            Err(stop) if stop.is_internal_failure() => {
                return Err(ContractRefusal::Fault(InternalFault::new(
                    "contract-check",
                    "declared-states-enumerate",
                )))
            }
            Err(stop) => return Ok(stopped(stop)),
        };
        match target.evaluate(&case).map_err(ContractRefusal::Fault)? {
            CaseVerdict::Holds => checked = true,
            CaseVerdict::Inapplicable => {}
            CaseVerdict::Undecided => undecided = true,
            CaseVerdict::Stopped(stop) => return Ok(stopped(stop)),
            CaseVerdict::Violated(counterexample) => {
                return Ok(
                    match target
                        .replay(&counterexample)
                        .map_err(ContractRefusal::Fault)?
                    {
                        Some(reproduced) => ContractOutcome::Refuted(Box::new(reproduced)),
                        None => ContractOutcome::ReplayParity,
                    },
                );
            }
        }
    }
    Ok(if undecided {
        ContractOutcome::Undecided
    } else if checked {
        ContractOutcome::Proved { cases }
    } else {
        ContractOutcome::Vacuous
    })
}

/// The model systems one item reads: the subject's, and for a simulation
/// item the refinement's abstract system beside it.
type Systems<'a> = (ModelSystem<'a>, Option<ModelSystem<'a>>);

/// Open the systems of `request`.
fn open<'a>(request: &ContractRequest<'a>) -> Result<Systems<'a>, ContractRefusal> {
    let Some(abstract_subject) = &request.abstract_subject else {
        return ModelSystem::new(request.subject.clone())
            .map(|system| (system, None))
            .map_err(|_| ContractRefusal::Admission);
    };
    let refinement = request
        .package
        .graph()
        .refinements()
        .iter()
        .find(|refinement| {
            refinement
                .step_simulations()
                .iter()
                .any(|entry| entry.occurrence() == request.claim.claim())
        })
        .ok_or_else(|| {
            ContractRefusal::Fault(InternalFault::new(
                "contract-check",
                "a-simulation-claim-names-its-refinement",
            ))
        })?;
    open_systems(
        refinement,
        &request.subject,
        abstract_subject.package,
        abstract_subject.initial.clone(),
    )
    .map(|systems| (systems.concrete, Some(systems.abstract_system)))
    .map_err(|refusal| match refusal {
        SubjectRefusal::Admission(_) => ContractRefusal::Admission,
        _ => ContractRefusal::Fault(InternalFault::new(
            "contract-check",
            "the-subjects-of-a-refinement-open",
        )),
    })
}

/// What the arm reports for one item.
pub(crate) enum ContractReport {
    /// The arm's finding.
    Found(ContractOutcome),
    /// An arm `unsupported` outcome.
    Unsupported(ContractForm, ContractCheckUnsupported),
}

fn invalid_input() -> Refusal {
    Refusal::Declined(
        ProofRefusalCause::InvalidInput,
        DeclineCode::Qsl(qsl_foundation::Code::InvalidRuntimeInput),
        CatalogCode::new("invalid_runtime_input", "invalid-value"),
    )
}

fn incomplete_input() -> Refusal {
    Refusal::Declined(
        ProofRefusalCause::IncompleteInput,
        DeclineCode::Qsl(qsl_foundation::Code::MissingDeclaration),
        CatalogCode::new("missing_declaration", "missing-name"),
    )
}

/// The `operation-contract` item that runs `selection` on the arm: a claim
/// that selects no form is `declined` `invalid-input`; one whose package
/// lacks a required node, or a bound item that reads an element with no
/// abstraction binding, is `declined` `incomplete-input`; none of them runs
/// a check (QSpec FR-483 "The check input", FR-485 "The oracle").
pub fn contract_item<'a>(
    index: RequestIndex,
    selection: Result<ContractRequest<'a>, ContractSelection>,
) -> AnalyzeItem<'a> {
    AnalyzeItem::new(
        index,
        Capability::OperationContract,
        Mode::Bounded,
        manifest::BACKEND_ID,
        move |cancel| {
            let request = match selection {
                Ok(request) => request,
                Err(ContractSelection::NoForm) => return EngineReport::Refused(invalid_input()),
                Err(ContractSelection::MissingNode | ContractSelection::Unbound(_)) => {
                    return EngineReport::Refused(incomplete_input())
                }
                Err(ContractSelection::Fault(fault)) => {
                    return EngineReport::Refused(Refusal::Failed(fault.catalog_code()))
                }
            };
            match check_contract(request, cancel) {
                Ok(outcome) => EngineReport::Contract(Box::new(ContractReport::Found(outcome))),
                Err(ContractRefusal::Unsupported(form, reason)) => {
                    EngineReport::Contract(Box::new(ContractReport::Unsupported(form, reason)))
                }
                Err(ContractRefusal::Admission) => EngineReport::Refused(Refusal::Failed(
                    CatalogCode::new("invalid_runtime_input", "invalid-value"),
                )),
                Err(ContractRefusal::Fault(fault)) => {
                    EngineReport::Refused(Refusal::Failed(fault.catalog_code()))
                }
            }
        },
    )
}

/// Settle the arm's report as the item's FR-331 terminal record (QSpec
/// FR-483 "Outcomes and their mapping"), with the counterexample a
/// `refuted` item keeps.
pub(crate) fn settle(
    report: ContractReport,
    index: RequestIndex,
    backend: qsl_foundation::BackendId,
) -> (qsl_replay::TerminalRecord, Option<ContractCounterexample>) {
    use qsl_replay::{
        IncompleteCause, InconclusiveCause, ModelCheckLimit, TerminalRecord, UnavailabilityCause,
    };

    let outcome = match report {
        ContractReport::Unsupported(form, reason) => {
            return (
                TerminalRecord::unsupported(
                    index,
                    backend,
                    UnavailabilityCause::ContractCheck { form, reason },
                ),
                None,
            )
        }
        ContractReport::Found(outcome) => outcome,
    };
    let incomplete = |cause| {
        (
            TerminalRecord::incomplete(index, backend.clone(), cause),
            None,
        )
    };
    match outcome {
        ContractOutcome::Proved { .. } => (TerminalRecord::exhaustive(index, backend), None),
        ContractOutcome::Refuted(reproduced) => (
            TerminalRecord::refuted_by_contract(index, backend, &reproduced),
            Some(reproduced.counterexample().clone()),
        ),
        ContractOutcome::ReplayParity => (
            TerminalRecord::inconclusive(
                index,
                backend,
                qsl_replay::contract_check::ReadCounterexample::NotReproduced
                    .inconclusive_cause()
                    .unwrap_or(InconclusiveCause::MappingUndetermined),
            ),
            None,
        ),
        // A zero-check proof is the vacuity cause `kani_vacuous_proof`.
        ContractOutcome::Vacuous => (TerminalRecord::proved(index, backend, 0), None),
        ContractOutcome::Undecided => (
            TerminalRecord::inconclusive(
                index,
                backend,
                InconclusiveCause::SolverInconclusive {
                    trace: None,
                    replay: None,
                },
            ),
            None,
        ),
        ContractOutcome::MaxCases(value) => incomplete(IncompleteCause::LimitReached {
            limit: ModelCheckLimit::ContractCheckMaxCases,
            value,
        }),
        ContractOutcome::Cancelled(source) => incomplete(IncompleteCause::Cancelled { source }),
        ContractOutcome::Stopped {
            stop,
            meter,
            max_candidates,
        } => incomplete(match stop.limit {
            Some(ExpansionLimit::EvaluationMeter(counter)) => IncompleteCause::LimitReached {
                limit: ModelCheckLimit::EvaluationMeter { counter },
                value: qsl_replay::meter_bound(&meter, counter),
            },
            Some(ExpansionLimit::MaxCandidates) => IncompleteCause::LimitReached {
                limit: ModelCheckLimit::MaxCandidates,
                value: max_candidates,
            },
            None => IncompleteCause::ResourceExhausted,
        }),
    }
}
