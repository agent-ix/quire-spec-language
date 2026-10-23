// SPDX-License-Identifier: AGPL-3.0-or-later
//! The plumbing the model-state certificate checkers share: FR-338's
//! [`check_closure`](super::closure::check_closure), FR-169's
//! [`check_state_graph`](super::state_graph::check_state_graph) and FR-149's
//! [`check_simulation_certificate`](super::simulation::check_simulation_certificate)
//! (which shares the bound pre-check and the fault vocabulary). Each
//! re-admits the subject the same way, stops at the same budgets with the
//! same causes and polls cancellation the same way, so the checkers cannot
//! drift from each other.

use qsl_eval::simulation::model::{check_model_bounds, ModelSimulationRequest, ModelSystem};
use qsl_eval::simulation::{EncodingRefusal, ExpansionLimit, ExpansionStop, BOUND_POSITIONS};
use quire_exact::Cancel;
use quire_semantic_value::catalog::InternalFault;

use crate::certificate::closure::{ClosureRefusal, TemporalCertificateCheck};
use crate::model_check::limits::ModelCheckLimit;
use crate::proof_result::IncompleteCause;

/// The fault component of the checker that broke an invariant.
#[derive(Clone, Copy)]
pub(super) enum Checker {
    /// FR-338's closure checker.
    Closure,
    /// FR-169's state-graph checker.
    StateGraph,
    /// FR-149's simulation checker.
    Simulation,
}

impl Checker {
    const fn component(self) -> &'static str {
        match self {
            Self::Closure => "closure-check",
            Self::StateGraph => "state-graph-check",
            Self::Simulation => "simulation-check",
        }
    }

    /// The fault of this checker's broken `invariant`.
    pub(super) fn fault(self, invariant: &'static str) -> ClosureRefusal {
        ClosureRefusal::Fault(InternalFault::new(self.component(), invariant))
    }
}

/// The budgets a check's expansion and predicate stops name.
#[derive(Clone, Copy)]
pub(super) struct Budgets {
    /// The evaluation meter's work units.
    pub(super) meter: u64,
    /// The expansion's candidate bound.
    pub(super) max_candidates: u64,
}

impl Budgets {
    /// The budgets of `simulation`.
    pub(super) fn of(simulation: &ModelSimulationRequest<'_>) -> Self {
        Self {
            meter: simulation.meter.work_units,
            max_candidates: simulation.expansion.max_candidates,
        }
    }
}

/// FR-120's admission of `simulation`'s `ModelSystem`, with EN-1's
/// requires-bound pre-check.
pub(super) fn admit(
    simulation: ModelSimulationRequest<'_>,
) -> Result<ModelSystem<'_>, ClosureRefusal> {
    let system = ModelSystem::new(simulation)
        .map_err(|failure| ClosureRefusal::Admission(Box::new(failure)))?;
    check_model_bounds(&system, BOUND_POSITIONS)
        .map_err(|refusal| ClosureRefusal::NotSimulated(Box::new(refusal)))?;
    Ok(system)
}

/// A finished check: a rejection or a stop, or a refusal.
pub(super) enum Halt {
    /// A rejection or a stop.
    Check(TemporalCertificateCheck),
    /// The request, not the certificate, is at fault.
    Refusal(ClosureRefusal),
}

impl From<ClosureRefusal> for Halt {
    fn from(refusal: ClosureRefusal) -> Self {
        Self::Refusal(refusal)
    }
}

impl From<EncodingRefusal> for Halt {
    fn from(refusal: EncodingRefusal) -> Self {
        Self::Refusal(refusal.into())
    }
}

/// One step of a check.
pub(super) type Step<T> = Result<T, Halt>;

/// The checker's result from a finished run.
pub(super) fn finish(run: Step<()>) -> Result<TemporalCertificateCheck, ClosureRefusal> {
    match run {
        Ok(()) => Ok(TemporalCertificateCheck::Accepted),
        Err(Halt::Check(result)) => Ok(result),
        Err(Halt::Refusal(refusal)) => Err(refusal),
    }
}

/// The stop at `limit`, set to `value`.
pub(super) fn stopped(limit: ModelCheckLimit, value: u64) -> Halt {
    Halt::Check(TemporalCertificateCheck::Stopped(
        IncompleteCause::LimitReached { limit, value },
    ))
}

/// Stop `Cancelled` when `cancel` is cancelled.
pub(super) fn poll(cancel: &Cancel) -> Step<()> {
    if cancel.poll() {
        if let Some(source) = cancel.cause() {
            return Err(Halt::Check(TemporalCertificateCheck::Stopped(
                IncompleteCause::Cancelled { source },
            )));
        }
    }
    Ok(())
}

/// The halt of an expansion or predicate stop: a named budget stops the
/// check naming the budget's value, any other stop stops it with no named
/// limit, and a broken invariant is `checker`'s fault.
pub(super) fn expansion_halt(stop: &ExpansionStop, budgets: Budgets, checker: Checker) -> Halt {
    match stop.limit {
        Some(ExpansionLimit::EvaluationMeter) => {
            stopped(ModelCheckLimit::EvaluationMeter, budgets.meter)
        }
        Some(ExpansionLimit::MaxCandidates) => {
            stopped(ModelCheckLimit::MaxCandidates, budgets.max_candidates)
        }
        None if stop.is_internal_failure() => {
            Halt::Refusal(checker.fault("model-expansion-invariant"))
        }
        None => Halt::Check(TemporalCertificateCheck::Stopped(
            IncompleteCause::ResourceExhausted,
        )),
    }
}

/// `value` as a `u64`, saturating.
pub(super) fn widen(value: usize) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
}
