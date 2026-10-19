// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-127 (ADR-018 V-1, V-4 to V-7, LA-3): the one settlement map from an
//! EN-1 outcome to a QSpec FR-331 terminal record.
//!
//! A `Violated` outcome settles `refuted` only on a replay result (ADR-018
//! SM-7) that reproduced it against the package the run checked: the
//! caller's `replay` (in production [`replay_model_trace`]) supplies the
//! result, and the map checks its `package_id`. A `Holds` outcome settles
//! only through the core's own closure checker, [`check_closure`], which the
//! map runs itself on the outcome's certificate (ADR-018 PC-3, PC-5):
//! accepted is `Proved{Exhaustive, Certified}`, rejected is
//! `Inconclusive(CertificateRejected{rule, at})`, and a stopped check is
//! `Incomplete` with its exact limit or cancellation source.

use qsl_foundation::diagnostic::Code;
use qsl_foundation::digest::DigestRecord;

use quire_exact::Cancel;

use crate::certificate::closure::{
    check_closure, CertificateRejection, CertificateRequest, ClosureRefusal,
    TemporalCertificateCheck,
};
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

/// FR-127: settle `outcome`, a run under `limits` over the package
/// `package_id`, as one terminal record. A `Holds` outcome's certificate is
/// checked by [`check_closure`] for `certificate`, under `limits` and
/// `cancel`; a refused check settles `inconclusive` by `NoInstance` for an
/// item with no instance and `failed` otherwise. A `Violated` outcome's
/// counterexample is handed to `replay` (FR-128) and settles by its result:
/// reproduced against `package_id` is `Refuted`; a result settled against
/// any other package is `Inconclusive(ReplayRefused(stale_dependency))`; a
/// disagreement is `Inconclusive(ReplayParity)`; a refusal is
/// `Inconclusive(ReplayRefused)`, except an internal fault, which is
/// `Failed`.
///
/// [`replay_model_trace`]: crate::model_check::replay::replay_model_trace
pub fn settle_model_check(
    outcome: &ModelCheckOutcome,
    limits: &ModelCheckLimits,
    package_id: &DigestRecord,
    certificate: &CertificateRequest<'_>,
    cancel: &Cancel,
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
    let incomplete = |cause| {
        record(
            TerminalValue::Incomplete(cause),
            ResultLabel::Failed,
            SettlementBasis::Unavailable,
            Execution::ResourceIncomplete,
            Truth::Unavailable,
        )
    };
    let failed = || {
        record(
            TerminalValue::Failed,
            ResultLabel::Failed,
            SettlementBasis::Unavailable,
            Execution::Failed,
            Truth::Unavailable,
        )
    };
    match outcome {
        ModelCheckOutcome::Holds {
            certificate: closure,
            ..
        } => match check_closure(certificate, closure, limits, cancel) {
            Ok(TemporalCertificateCheck::Accepted) => record(
                TerminalValue::Proved {
                    basis: ProofBasis::Exhaustive,
                    certification: Certification::Certified,
                },
                ResultLabel::Proved,
                SettlementBasis::ClosedScope,
                Execution::Completed,
                Truth::Satisfied,
            ),
            Ok(TemporalCertificateCheck::Rejected(CertificateRejection { rule, at })) => {
                inconclusive(InconclusiveCause::CertificateRejected { rule, at })
            }
            Ok(TemporalCertificateCheck::Stopped(cause)) => {
                let limit_reached = match cause {
                    IncompleteCause::LimitReached { limit, value } => {
                        Some(ReachedLimit { limit, value })
                    }
                    IncompleteCause::TimedOut
                    | IncompleteCause::Cancelled { .. }
                    | IncompleteCause::ResourceExhausted => None,
                };
                ModelCheckRecord {
                    limit_reached,
                    ..incomplete(cause)
                }
            }
            Err(ClosureRefusal::NoInstance) => inconclusive(InconclusiveCause::NoInstance),
            Err(_) => failed(),
        },
        ModelCheckOutcome::BoundReached { depth, .. } => ModelCheckRecord {
            depth: Some(*depth),
            ..inconclusive(InconclusiveCause::BoundReached { depth: *depth })
        },
        ModelCheckOutcome::Undecided { cause, .. } => inconclusive(match cause {
            UndecidedCause::UndecidedSuccessor => InconclusiveCause::UndecidedSuccessor,
            UndecidedCause::NoInitialState => InconclusiveCause::NoInitialState,
            UndecidedCause::NoInstance => InconclusiveCause::NoInstance,
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
                ..incomplete(cause)
            }
        }
        ModelCheckOutcome::Violated { counterexample, .. } => match replay(counterexample) {
            Ok(ModelTraceReplayResult {
                package_id: replayed,
                ..
            }) if !replayed.matches(package_id) => {
                inconclusive(InconclusiveCause::ReplayRefused(Code::StaleDependency))
            }
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
            Err(ReplayRefusal::Fault(_)) => failed(),
            Err(refusal) => inconclusive(InconclusiveCause::ReplayRefused(refusal.code())),
        },
    }
}
