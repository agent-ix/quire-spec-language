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
//! A `Holds` half settles `Proved{basis: Exhaustive}` labelled `Uncertified`:
//! the proof is exhaustive exploration of the concrete subject, and no core
//! checker stands behind it until the simulation certificate (FR-149) exists,
//! which FR-144 requires before it settles `Certified`.
//!
//! What this does not do yet: the requirement record and its `Refinement`
//! property form, the obligation identity, the liveness half and the
//! item-level combination of the two halves.

use crate::execute::ReplayRefusal;
use crate::model_check::counterexample::TemporalCounterexample;
use crate::model_check::limits::ModelCheckLimits;
use crate::model_check::refinement_replay::RefinementTraceSettlement;
use crate::model_check::settle::{ModelCheckRecord, Row, EXPLICIT_STATE};
use crate::proof_result::{Certification, InconclusiveCause, ProofBasis, SettlementBasis};
use crate::refinement::SafetyOutcome;

/// FR-144: settle `outcome`, a safety run under `limits`, as one terminal
/// record. A `Violated` outcome's counterexample is handed to `replay`
/// (FR-145) and settles by its result.
pub fn settle_refinement_safety(
    outcome: &SafetyOutcome,
    limits: &ModelCheckLimits,
    replay: impl FnOnce(&TemporalCounterexample) -> Result<RefinementTraceSettlement, ReplayRefusal>,
) -> ModelCheckRecord {
    let mut counterexample = None;
    let mut depth = None;
    let row = match outcome {
        SafetyOutcome::Holds { .. } => Row::proof(
            ProofBasis::Exhaustive,
            Certification::Uncertified,
            SettlementBasis::ClosedScope,
        ),
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
    }
}
