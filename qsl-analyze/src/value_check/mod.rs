// SPDX-License-Identifier: AGPL-3.0-or-later
//! The `value-validity` check arm (QSpec FR-483, FR-484): decide a claim over
//! a checked package by evaluating every assignment of its function's
//! parameters in their finite domains through the native evaluator.
//!
//! * A `clause` (a `Boolean` function's own claim) is `proved` with basis
//!   `exhaustive` and certification `uncertified` when it holds at every
//!   assignment, and `refuted` at the first assignment where it is false or
//!   undefined, once the native replay reproduces that violation.
//! * A `scalar-operation` is `refuted` where the evaluation stops undefined
//!   at its own occurrence. The evaluator reports neither the occurrences it
//!   passed through nor those a stop elsewhere masked, so the arm never
//!   proves one: with no violation found it is `unknown`.
//! * One case is one assignment evaluated. The count is `check.limits.max_cases`
//!   or the descriptor default; reaching it before the domain is exhausted is
//!   `incomplete` `limit-reached`.
//!
//! [`value_item`] is the only constructor of the arm's work.

pub mod manifest;

use std::num::NonZeroU64;

use qsl_foundation::RequestIndex;
use qsl_replay::value_check::{
    integers, ClaimForm, ParameterDomain, ReproducedViolation, Selection, Stop, ValueTarget,
    Verdict,
};
use qsl_replay::{Cancel, DeclineCode, ProofRefusalCause, Value, ValueCheckUnsupported};
use qsl_route::Mode;
use qsl_semantics::check::Capability;
use quire_semantic_value::catalog::{CatalogCode, InternalFault};

use crate::analyze::{AnalyzeItem, EngineReport, Refusal};

/// What the arm found, before settlement.
#[derive(Debug)]
pub enum ValueOutcome {
    /// The goal holds at all `cases` assignments of the finite domain.
    Proved {
        /// The assignments evaluated.
        cases: u64,
    },
    /// A violation the native replay reproduced.
    Refuted(Box<ReproducedViolation>),
    /// A candidate violation the native replay did not reproduce.
    ReplayParity,
    /// No assignment: a bound left a parameter without a value.
    Vacuous,
    /// The arm neither established the goal nor found a violation.
    Undecided,
    /// The case budget was reached before the domain was exhausted.
    MaxCases(u64),
    /// The evaluation was stopped.
    Stopped(Stop),
}

/// Why the arm settles without a check.
#[derive(Debug)]
pub enum ValueRefusal {
    /// A parameter is the mathematical `Integer` with no bound.
    RequiresBound,
    /// The form or an element it reads lies outside what the arm decides.
    Unsupported(ClaimForm, ValueCheckUnsupported),
    /// A broken invariant.
    Fault(InternalFault),
}

/// One value-validity request item: the claim's target and its case budget.
pub struct ValueRequest<'p> {
    /// The claim's target, with the item's bounds applied.
    pub target: ValueTarget<'p>,
    /// The item's `check.limits.max_cases`; absent takes the descriptor
    /// default ([`manifest::DEFAULT_MAX_CASES`]).
    pub max_cases: Option<NonZeroU64>,
}

/// The values of `domain` in order, at most `cap` of them.
fn values(domain: &ParameterDomain, cap: u64) -> Result<Vec<Value>, ValueCheckUnsupported> {
    let take = usize::try_from(cap).unwrap_or(usize::MAX);
    match domain {
        ParameterDomain::Boolean => Ok(vec![Value::Boolean(false), Value::Boolean(true)]),
        ParameterDomain::Integers(Some(interval)) => {
            Ok(integers(interval).take(take).map(Value::Integer).collect())
        }
        ParameterDomain::Integers(None) => Ok(Vec::new()),
        ParameterDomain::Unbounded | ParameterDomain::Unsupported(_) => {
            Err(ValueCheckUnsupported::ParameterDomain)
        }
    }
}

/// Decide `request` (QSpec FR-483, FR-484).
///
/// # Errors
///
/// [`ValueRefusal`] when the item settles without a check.
pub fn check_value(
    request: &ValueRequest<'_>,
    cancel: &Cancel,
) -> Result<ValueOutcome, ValueRefusal> {
    let target = &request.target;
    let form = target.form();
    let budget = request
        .max_cases
        .unwrap_or(manifest::DEFAULT_MAX_CASES)
        .get();
    if target
        .parameters()
        .iter()
        .any(|parameter| *parameter.domain() == ParameterDomain::Unbounded)
    {
        return Err(ValueRefusal::RequiresBound);
    }
    let domains = target
        .parameters()
        .iter()
        .map(|parameter| values(parameter.domain(), budget.saturating_add(1)))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|reason| ValueRefusal::Unsupported(form, reason))?;
    if domains.iter().any(Vec::is_empty) {
        return Ok(ValueOutcome::Vacuous);
    }
    let mut position = vec![0_usize; domains.len()];
    let mut cases = 0_u64;
    loop {
        if cases >= budget {
            return Ok(ValueOutcome::MaxCases(budget));
        }
        cases += 1;
        let assignment: Vec<Value> = position
            .iter()
            .zip(&domains)
            .map(|(at, domain)| domain[*at].clone())
            .collect();
        match target
            .evaluate(&assignment, cancel)
            .map_err(ValueRefusal::Fault)?
        {
            Verdict::Holds | Verdict::NotDecisive => {}
            Verdict::Violated(violation) => {
                return Ok(
                    match target
                        .replay(&assignment, &violation, cancel)
                        .map_err(ValueRefusal::Fault)?
                    {
                        Some(reproduced) => ValueOutcome::Refuted(Box::new(reproduced)),
                        None => ValueOutcome::ReplayParity,
                    },
                );
            }
            Verdict::Stopped(stop) => return Ok(ValueOutcome::Stopped(stop)),
            Verdict::Refused => {
                return Err(ValueRefusal::Unsupported(
                    form,
                    ValueCheckUnsupported::EvaluationRefused,
                ))
            }
        }
        // Advance the odometer, the last parameter fastest.
        let mut digit = position.len();
        loop {
            if digit == 0 {
                return Ok(match form {
                    ClaimForm::Clause => ValueOutcome::Proved { cases },
                    ClaimForm::ScalarOperation => ValueOutcome::Undecided,
                });
            }
            digit -= 1;
            position[digit] += 1;
            if position[digit] < domains[digit].len() {
                break;
            }
            position[digit] = 0;
        }
    }
}

/// The record a refuted item keeps: the reproduced violation.
#[derive(Debug)]
pub struct ValueCounterexample {
    /// The claim's `{node, occurrence}`.
    pub claim: qsl_replay::OccurrenceKey,
    /// The parameters' wire node ids, in the order of the assignment.
    pub parameters: Vec<qsl_replay::WireNodeId>,
    /// The reproduced violation.
    pub reproduced: ReproducedViolation,
}

/// What the arm reports for one item.
pub(crate) enum ValueReport {
    /// The arm's finding, with the counterexample's identity.
    Found(
        ValueOutcome,
        Vec<qsl_replay::WireNodeId>,
        qsl_replay::OccurrenceKey,
    ),
    /// An arm `unsupported` outcome.
    Unsupported(ClaimForm, ValueCheckUnsupported),
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

/// The `value-validity` item that runs `selection` on the arm: a claim that
/// selects no form is `declined` `invalid-input`, one whose package lacks a
/// required node `incomplete-input`, and neither runs a check (QSpec FR-483
/// "The check input").
pub fn value_item<'a>(
    index: RequestIndex,
    selection: Result<ValueRequest<'a>, Selection>,
) -> AnalyzeItem<'a> {
    AnalyzeItem::new(
        index,
        Capability::ValueValidity,
        Mode::Bounded,
        manifest::BACKEND_ID,
        move |cancel| {
            let request = match selection {
                Ok(request) => request,
                Err(Selection::NoForm) => return EngineReport::Refused(invalid_input()),
                Err(Selection::MissingNode) => return EngineReport::Refused(incomplete_input()),
                Err(Selection::Fault(fault)) => {
                    return EngineReport::Refused(Refusal::Failed(fault.catalog_code()))
                }
            };
            let parameters = request
                .target
                .parameters()
                .iter()
                .map(|parameter| parameter.wire())
                .collect();
            let claim = request.target.claim().clone();
            match check_value(&request, cancel) {
                Ok(outcome) => {
                    EngineReport::Value(Box::new(ValueReport::Found(outcome, parameters, claim)))
                }
                Err(ValueRefusal::RequiresBound) => EngineReport::Refused(Refusal::RequiresBound),
                Err(ValueRefusal::Unsupported(form, reason)) => {
                    EngineReport::Value(Box::new(ValueReport::Unsupported(form, reason)))
                }
                Err(ValueRefusal::Fault(fault)) => {
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
    report: ValueReport,
    index: RequestIndex,
    backend: qsl_foundation::BackendId,
) -> (qsl_replay::TerminalRecord, Option<ValueCounterexample>) {
    use qsl_replay::{
        DisagreementCause, IncompleteCause, InconclusiveCause, ModelCheckLimit, TerminalRecord,
        UnavailabilityCause, Verdict,
    };
    use quire_semantic_value::catalog::Category;

    let (outcome, parameters, claim) = match report {
        ValueReport::Unsupported(form, reason) => {
            return (
                TerminalRecord::unsupported(
                    index,
                    backend,
                    UnavailabilityCause::ValueCheck { form, reason },
                ),
                None,
            )
        }
        ValueReport::Found(outcome, parameters, claim) => (outcome, parameters, claim),
    };
    match outcome {
        ValueOutcome::Proved { .. } => (TerminalRecord::exhaustive(index, backend), None),
        ValueOutcome::Refuted(reproduced) => (
            TerminalRecord::refuted_by_assignment(index, backend, &reproduced),
            Some(ValueCounterexample {
                claim,
                parameters,
                reproduced: *reproduced,
            }),
        ),
        ValueOutcome::ReplayParity => (
            TerminalRecord::inconclusive(
                index,
                backend,
                InconclusiveCause::ReplayParity(DisagreementCause::Verdicts {
                    proved: Verdict::from_category(Category::Violation),
                    replayed: Verdict::from_category(Category::Success),
                }),
            ),
            None,
        ),
        // A zero-check proof is the vacuity cause `kani_vacuous_proof`.
        ValueOutcome::Vacuous => (TerminalRecord::proved(index, backend, 0), None),
        ValueOutcome::Undecided => (
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
        ValueOutcome::MaxCases(value) => (
            TerminalRecord::incomplete(
                index,
                backend,
                IncompleteCause::LimitReached {
                    limit: ModelCheckLimit::ValueCheckMaxCases,
                    value,
                },
            ),
            None,
        ),
        ValueOutcome::Stopped(Stop::Cancelled(source)) => (
            TerminalRecord::incomplete(index, backend, IncompleteCause::Cancelled { source }),
            None,
        ),
        ValueOutcome::Stopped(Stop::Meter(incomplete)) => (
            TerminalRecord::incomplete(
                index,
                backend,
                IncompleteCause::LimitReached {
                    limit: ModelCheckLimit::EvaluationMeter {
                        counter: incomplete.limit_kind,
                    },
                    value: incomplete.limit,
                },
            ),
            None,
        ),
    }
}
