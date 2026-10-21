// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-144 (ADR-020 RE-4, RE-5, ADR-018 V-1, V-4 to V-7): the settlement of a
//! refinement's safety half as an FR-331 terminal record, by FR-127's rows.
//!
//! A `Violated` half settles `refuted` only through its replay (FR-145)
//! reproducing it; a replay that settles `inconclusive` settles V-6 with
//! `ReplayParity`, a refusal V-6 with `ReplayRefused` and a fault `failed`.
//! An undecided half settles V-6 with `MappingUndetermined` or
//! `UndecidedSuccessor`, and a fault of the engine or the mapping settles
//! `failed` (QSpec FR-383).
//!
//! A `Holds` half settles `proved` (`Exhaustive`, `Certified`, `closed-scope`)
//! only after the core's own [`check_simulation_certificate`] (FR-149)
//! accepts its certificate. A rejected certificate settles `inconclusive`
//! with `CertificateRejected{rule, at}` (ADR-018 PC-2), a check that a limit
//! or a cancellation stops settles V-7 naming the limit, and a certificate
//! that names another refinement settles `ReplayRefused(stale_dependency)`.
//!
//! [`settle_refinement`] settles the whole item from its two halves. A
//! liveness half that holds settles `proved` `Uncertified`, since the core
//! has no checker for it (ADR-020 CT-4); a `Violated` one settles `refuted`
//! only through the replay of its lasso, as a safety half does. The item's
//! value is the safety half's unless the safety half is `proved`, and then it
//! is the liveness half's: `Unsupported` carries the proved safety half
//! beside it (ADR-020 AX-5).
//!
//! What this does not do yet: the requirement record and its `Refinement`
//! property form and the obligation identity.

use qsl_foundation::InternalFault;
use quire_exact::Cancel;

use crate::certificate::simulation::{
    check_simulation_certificate, SimulationCertificateRequest, SimulationRefusal,
};
use crate::execute::ReplayRefusal;
use crate::model_check::counterexample::TemporalCounterexample;
use crate::model_check::limits::ModelCheckLimits;
use crate::model_check::outcome::{ModelCheckOutcome, UndecidedCause};
use crate::model_check::refinement_replay::RefinementTraceSettlement;
use crate::model_check::settle::{ModelCheckRecord, Row, EXPLICIT_STATE};
use crate::proof_result::{
    Certification, IncompleteCause, InconclusiveCause, ProofBasis, SettlementBasis, TerminalValue,
};
use crate::refinement::{LivenessHalf, RefinementOutcome, SafetyOutcome};

/// FR-144's terminal record of one refinement item: the item's own record
/// and the value each half settled.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RefinementRecord {
    /// The item's record, with the counterexample a `refuted` item carries.
    pub record: ModelCheckRecord,
    /// The safety half's value; `None` when `check_refinement` refused and
    /// no half was decided.
    pub safety: Option<TerminalValue>,
    /// The liveness half's value; `None` when the refinement has none, and
    /// when `check_refinement` refused.
    pub liveness: Option<TerminalValue>,
    /// The internal fault `check_refinement` refused with, which the item
    /// settles `failed` with; `None` for an item that has an outcome.
    pub fault: Option<InternalFault>,
}

/// FR-144: settle the refusal `check_refinement` returned for an internal
/// fault of its second phase (FR-143): the item is `failed` with that fault
/// and no half value, the safety half and its certificate having been
/// discarded with the outcome.
pub fn settle_refinement_refusal(
    fault: &InternalFault,
    limits: &ModelCheckLimits,
) -> RefinementRecord {
    RefinementRecord {
        record: record_of(Row::failed(), limits, None, None),
        safety: None,
        liveness: None,
        fault: Some(fault.clone()),
    }
}

/// FR-144: settle `outcome` as one terminal record, each half by its own
/// rows. The safety half's certificate is checked by
/// [`check_simulation_certificate`] over `certified`, and each `Violated`
/// half's counterexample is handed to `replay` (FR-145).
pub fn settle_refinement(
    outcome: &RefinementOutcome,
    limits: &ModelCheckLimits,
    certified: &SimulationCertificateRequest<'_>,
    cancel: &Cancel,
    mut replay: impl FnMut(&TemporalCounterexample) -> Result<RefinementTraceSettlement, ReplayRefusal>,
) -> RefinementRecord {
    let safety = settle_refinement_safety(&outcome.safety, limits, certified, cancel, &mut replay);
    let liveness = match &outcome.liveness {
        None => None,
        Some(LivenessHalf::Unsupported(cause)) => Some(record_of(
            Row::unsupported(cause.clone()),
            limits,
            None,
            None,
        )),
        Some(LivenessHalf::Checked(checked)) => Some(settle_liveness(checked, limits, replay)),
    };
    let proved = matches!(safety.value, TerminalValue::Proved { .. });
    RefinementRecord {
        safety: Some(safety.value.clone()),
        liveness: liveness.as_ref().map(|record| record.value.clone()),
        fault: None,
        record: match liveness {
            Some(liveness) if proved => liveness,
            _ => safety,
        },
    }
}

/// The liveness half's record: `proved` `Uncertified` when it holds, `refuted`
/// when its lasso replays, and the same rows as the safety half's otherwise.
fn settle_liveness(
    outcome: &ModelCheckOutcome,
    limits: &ModelCheckLimits,
    replay: impl FnOnce(&TemporalCounterexample) -> Result<RefinementTraceSettlement, ReplayRefusal>,
) -> ModelCheckRecord {
    let mut counterexample = None;
    let row = match outcome {
        // No core checker covers the liveness half (ADR-020 CT-4).
        ModelCheckOutcome::HoldsReduced { .. } => Row::proof(
            ProofBasis::Exhaustive,
            Certification::Uncertified,
            SettlementBasis::ClosedScope,
        ),
        ModelCheckOutcome::Violated {
            counterexample: payload,
            ..
        } => match replay(payload) {
            Ok(RefinementTraceSettlement::Reproduced { .. }) => {
                counterexample = Some(payload.as_ref().clone());
                Row::refuted(SettlementBasis::DecisiveCounterexample)
            }
            Ok(RefinementTraceSettlement::Inconclusive(cause)) => {
                Row::inconclusive(InconclusiveCause::ReplayParity(cause))
            }
            Ok(RefinementTraceSettlement::Stopped(reached)) => {
                Row::stopped_run(IncompleteCause::ResourceExhausted, Some(reached))
            }
            Err(refusal) => Row::refused(&refusal),
        },
        ModelCheckOutcome::Undecided { cause, .. } => Row::inconclusive(match cause {
            UndecidedCause::UndecidedSuccessor => InconclusiveCause::UndecidedSuccessor,
            UndecidedCause::NoInitialState => InconclusiveCause::NoInitialState,
            UndecidedCause::NoInstance => InconclusiveCause::NoInstance,
            UndecidedCause::MatchUndetermined => InconclusiveCause::MatchUndetermined,
            UndecidedCause::VacuousMatch => InconclusiveCause::VacuousMatch,
            UndecidedCause::InstanceBoundReached { bound, states } => {
                InconclusiveCause::InstanceBoundReached {
                    bound: *bound,
                    states: *states,
                }
            }
        }),
        ModelCheckOutcome::Stopped { cause, limit, .. } => Row::stopped_run(*cause, *limit),
        // The second phase of a refinement returns no other outcome.
        ModelCheckOutcome::Holds { .. }
        | ModelCheckOutcome::Reduction { .. }
        | ModelCheckOutcome::Memory { .. }
        | ModelCheckOutcome::BoundReached { .. } => Row::failed(),
    };
    record_of(row, limits, None, counterexample)
}

/// The record of `row` for a run under `limits`.
fn record_of(
    row: Row,
    limits: &ModelCheckLimits,
    depth: Option<u64>,
    counterexample: Option<TemporalCounterexample>,
) -> ModelCheckRecord {
    ModelCheckRecord {
        value: row.value,
        label: row.label,
        basis: row.basis,
        execution: row.execution,
        truth: row.truth,
        method: EXPLICIT_STATE,
        depth,
        limits: limits.used(),
        limit_reached: row.limit_reached,
        counterexample,
        hints: Vec::new(),
        memory_bound: None,
        instance_bound: None,
    }
}

/// FR-144: settle `outcome`, a safety run under `limits`, as one terminal
/// record. A `Holds` outcome's certificate is checked by
/// [`check_simulation_certificate`] over `certified`, under `limits` and
/// `cancel`. A `Violated` outcome's counterexample is handed to `replay`
/// (FR-145) and settles by its result.
pub fn settle_refinement_safety(
    outcome: &SafetyOutcome,
    limits: &ModelCheckLimits,
    certified: &SimulationCertificateRequest<'_>,
    cancel: &Cancel,
    replay: impl FnOnce(&TemporalCounterexample) -> Result<RefinementTraceSettlement, ReplayRefusal>,
) -> ModelCheckRecord {
    let mut counterexample = None;
    let mut depth = None;
    let row = match outcome {
        SafetyOutcome::Holds { certificate, .. } => {
            match check_simulation_certificate(certified, certificate, limits, cancel) {
                Err(SimulationRefusal::Stale { .. }) => Row::stale(),
                Err(SimulationRefusal::Subject(refusal)) => Row::of_check(Err(refusal), || {
                    Row::proved(ProofBasis::Exhaustive, SettlementBasis::ClosedScope)
                }),
                Ok(check) => Row::of_check(Ok(check), || {
                    Row::proved(ProofBasis::Exhaustive, SettlementBasis::ClosedScope)
                }),
            }
        }
        SafetyOutcome::Violated {
            counterexample: payload,
            ..
        } => match replay(payload) {
            Ok(RefinementTraceSettlement::Reproduced { .. }) => {
                counterexample = Some(payload.as_ref().clone());
                Row::refuted(SettlementBasis::DecisiveCounterexample)
            }
            Ok(RefinementTraceSettlement::Inconclusive(cause)) => {
                Row::inconclusive(InconclusiveCause::ReplayParity(cause))
            }
            Ok(RefinementTraceSettlement::Stopped(reached)) => {
                Row::stopped_run(IncompleteCause::ResourceExhausted, Some(reached))
            }
            Err(refusal) => Row::refused(&refusal),
        },
        SafetyOutcome::BoundReached { depth: reached, .. } => {
            depth = Some(*reached);
            Row::inconclusive(InconclusiveCause::BoundReached { depth: *reached })
        }
        SafetyOutcome::Undecided { cause, .. } => Row::inconclusive(cause.cause()),
        SafetyOutcome::Stopped { cause, limit, .. } => Row::stopped_run(*cause, *limit),
        SafetyOutcome::Fault { .. } => Row::failed(),
    };
    record_of(row, limits, depth, counterexample)
}
