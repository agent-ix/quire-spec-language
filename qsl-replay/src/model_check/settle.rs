// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-127 (ADR-018 V-1, V-4 to V-7, LA-3): the one settlement map from an
//! EN-1 outcome to a QSpec FR-331 terminal record.
//!
//! The map replays a counterexample itself, so a `Violated` outcome settles
//! `refuted` only through its replay (ADR-018 SM-7). A `Holds` outcome is an
//! exhaustive proof; EN-1 returns no certificate the core checks yet
//! (ADR-018 PC-3), so it settles `Proved{Exhaustive, Uncertified}`, a
//! first-party engine's proof with no core checker (PC-1), never
//! `Certified`.

use crate::execute::ReplayRefusal;
use crate::model_check::counterexample::TemporalCounterexample;
use crate::model_check::limits::{ModelCheckLimit, ModelCheckLimits, ReachedLimit};
use crate::model_check::outcome::{ModelCheckOutcome, UndecidedCause};
use crate::model_check::replay::{ModelTraceReplayResult, ModelTraceSettlement};
use crate::proof_result::{
    Certification, IncompleteCause, InconclusiveCause, ProofBasis, SettlementBasis, TerminalValue,
};

/// The settlement method every EN-1 record states (FR-127).
pub const EXPLICIT_STATE: &str = "explicit-state";

/// QSpec FR-360's result label.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResultLabel {
    /// `proved`.
    Proved,
    /// `refuted`.
    Refuted,
    /// `inconclusive`.
    Inconclusive,
    /// `failed`.
    Failed,
}

/// The record's execution status.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Execution {
    /// The method completed.
    Completed,
    /// A limit or a cancellation stopped it: `resource-incomplete`.
    ResourceIncomplete,
    /// The tool failed.
    Failed,
}

/// The record's truth.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Truth {
    /// The item holds.
    Satisfied,
    /// The item does not hold.
    Violated,
    /// Execution completed and the truth is still pending.
    Pending,
    /// No truth was settled.
    Unavailable,
}

/// One EN-1 item's terminal record (FR-127 Outputs).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelCheckRecord {
    /// The terminal value.
    pub value: TerminalValue,
    /// Its QSpec FR-360 label.
    pub label: ResultLabel,
    /// Its QSpec FR-243 basis.
    pub basis: SettlementBasis,
    /// The execution status.
    pub execution: Execution,
    /// The truth.
    pub truth: Truth,
    /// The settlement method: [`EXPLICIT_STATE`].
    pub method: &'static str,
    /// The search horizon a V-5 record states.
    pub depth: Option<u64>,
    /// Each limit of the run with the value it used.
    pub limits: [(ModelCheckLimit, u64); 3],
    /// The limit the run reached, when one stopped it.
    pub limit_reached: Option<ReachedLimit>,
    /// The counterexample a `refuted` record carries.
    pub counterexample: Option<TemporalCounterexample>,
}

/// FR-127: settle `outcome`, a run under `limits`, as one terminal record.
/// A `Violated` outcome's counterexample is replayed by `replay` (FR-128)
/// and settles by its result: reproduced is `Refuted`; a disagreement is
/// `Inconclusive(ReplayParity)`; a refusal is `Inconclusive(ReplayRefused)`,
/// except an internal fault, which is `Failed`.
pub fn settle_model_check(
    outcome: &ModelCheckOutcome,
    limits: &ModelCheckLimits,
    replay: impl FnOnce(&TemporalCounterexample) -> Result<ModelTraceReplayResult, ReplayRefusal>,
) -> ModelCheckRecord {
    let record = |value, label, basis, execution, truth| ModelCheckRecord {
        value,
        label,
        basis,
        execution,
        truth,
        method: EXPLICIT_STATE,
        depth: None,
        limits: limits.used(),
        limit_reached: None,
        counterexample: None,
    };
    let inconclusive = |cause| {
        record(
            TerminalValue::Inconclusive(cause),
            ResultLabel::Inconclusive,
            SettlementBasis::Unsettled,
            Execution::Completed,
            Truth::Pending,
        )
    };
    match outcome {
        ModelCheckOutcome::Holds { .. } => record(
            TerminalValue::Proved {
                basis: ProofBasis::Exhaustive,
                certification: Certification::Uncertified,
            },
            ResultLabel::Proved,
            SettlementBasis::ClosedScope,
            Execution::Completed,
            Truth::Satisfied,
        ),
        ModelCheckOutcome::BoundReached { depth, .. } => ModelCheckRecord {
            depth: Some(*depth),
            ..inconclusive(InconclusiveCause::BoundReached { depth: *depth })
        },
        ModelCheckOutcome::Undecided { cause, .. } => inconclusive(match cause {
            UndecidedCause::UndecidedSuccessor => InconclusiveCause::UndecidedSuccessor,
            UndecidedCause::NoInitialState => InconclusiveCause::NoInitialState,
        }),
        ModelCheckOutcome::Stopped { cause, limit, .. } => {
            let cause = match (cause, limit) {
                (IncompleteCause::ResourceExhausted, Some(reached)) => {
                    IncompleteCause::LimitReached {
                        limit: reached.limit,
                        value: reached.value,
                    }
                }
                (cause, _) => *cause,
            };
            ModelCheckRecord {
                limit_reached: *limit,
                ..record(
                    TerminalValue::Incomplete(cause),
                    ResultLabel::Failed,
                    SettlementBasis::Unavailable,
                    Execution::ResourceIncomplete,
                    Truth::Unavailable,
                )
            }
        }
        ModelCheckOutcome::Violated { counterexample, .. } => match replay(counterexample) {
            Ok(ModelTraceReplayResult {
                settlement: ModelTraceSettlement::Reproduced { .. },
                ..
            }) => ModelCheckRecord {
                counterexample: Some(counterexample.as_ref().clone()),
                ..record(
                    TerminalValue::Refuted,
                    ResultLabel::Refuted,
                    SettlementBasis::DecisiveCounterexample,
                    Execution::Completed,
                    Truth::Violated,
                )
            },
            Ok(ModelTraceReplayResult {
                settlement: ModelTraceSettlement::Inconclusive(cause),
                ..
            }) => inconclusive(InconclusiveCause::ReplayParity(cause)),
            Err(ReplayRefusal::Fault(_)) => record(
                TerminalValue::Failed,
                ResultLabel::Failed,
                SettlementBasis::Unavailable,
                Execution::Failed,
                Truth::Unavailable,
            ),
            Err(refusal) => inconclusive(InconclusiveCause::ReplayRefused(refusal.code())),
        },
    }
}
