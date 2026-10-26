// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-235 (ADR-026 TV-1, CF-4, OV-14): the one settlement map from an EN-6
//! outcome to a QSpec FR-331 terminal record.
//!
//! A `Holds` outcome settles `proved` only after the core's checker of its
//! zone certificate (FR-245) accepts it, and a `Violated` outcome settles
//! `refuted` only through its replay (FR-237) against the package the run
//! checked. The caller supplies both: `check` runs the certificate checker
//! and `replay` the replay.

use qsl_foundation::digest::DigestRecord;
use qsl_replay::{
    CertificateLocus, CertificateRule, CounterexampleKind, IncompleteCause, InconclusiveCause,
    ModelCheckLimit, ModelTraceReplayResult, ModelTraceSettlement, ProofBasis, ReachedLimit,
    ReplayRefusal, ResultLabel, SettlementBasis, TemporalCounterexample, TerminalValue, Truth,
    UnavailabilityCause,
};
use qsl_replay::{Certification, Execution};
use qsl_semantics::family::UnsupportedItem;
use quire_exact::Cancel;

use super::certificate::ZoneCertificate;
use super::search::{ZoneCheckLimit, ZoneCheckOutcome, ZoneStop, ZoneUndecided};

/// The settlement method of an EN-6 record (FR-235).
pub const ZONE_CERTIFIED: &str = "zone-certified";

/// The result of the core's check of a zone certificate (FR-245), as the
/// map reads it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ZoneCertificateCheck {
    /// The checker accepted the certificate.
    Accepted,
    /// The checker rejected it, at the rule and locus that failed.
    Rejected {
        /// The rule that failed.
        rule: CertificateRule,
        /// Where.
        at: CertificateLocus,
    },
    /// A budget or a cancellation stopped the check.
    Stopped(IncompleteCause),
}

/// What FR-235 settles for one item over a timed subject: the request
/// writer's disposition of an item it routed to no engine, or EN-6's
/// outcome.
#[derive(Clone, Copy, Debug)]
pub enum ZoneItem<'o, S> {
    /// V-8: the request writer disposed the item `Unsupported`, and it ran
    /// on no engine.
    Unsupported(&'o UnsupportedItem),
    /// EN-6's outcome.
    Outcome(&'o ZoneCheckOutcome<S>),
}

/// One EN-6 item's terminal record (FR-235 Outputs).
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct ZoneCheckRecord {
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
    /// The settlement method: [`ZONE_CERTIFIED`].
    pub method: &'static str,
    /// The limit a stopped run or check reached.
    pub limit_reached: Option<ReachedLimit>,
    /// The counterexample a `refuted` record carries, which replay
    /// reproduced.
    pub counterexample: Option<TemporalCounterexample>,
}

impl ZoneCheckRecord {
    fn row(
        value: TerminalValue,
        label: ResultLabel,
        basis: SettlementBasis,
        execution: Execution,
        truth: Truth,
    ) -> Self {
        Self {
            value,
            label,
            basis,
            execution,
            truth,
            method: ZONE_CERTIFIED,
            limit_reached: None,
            counterexample: None,
        }
    }

    fn inconclusive(cause: InconclusiveCause) -> Self {
        Self::row(
            TerminalValue::Inconclusive(cause),
            ResultLabel::Inconclusive,
            SettlementBasis::Unsettled,
            Execution::Completed,
            Truth::Pending,
        )
    }

    fn incomplete(cause: IncompleteCause) -> Self {
        let limit_reached = match cause {
            IncompleteCause::LimitReached { limit, value } => Some(ReachedLimit { limit, value }),
            IncompleteCause::TimedOut
            | IncompleteCause::Cancelled { .. }
            | IncompleteCause::ResourceExhausted => None,
        };
        Self {
            limit_reached,
            ..Self::row(
                TerminalValue::Incomplete(cause),
                ResultLabel::Failed,
                SettlementBasis::Unavailable,
                Execution::ResourceIncomplete,
                Truth::Unavailable,
            )
        }
    }

    fn failed() -> Self {
        Self::row(
            TerminalValue::Failed,
            ResultLabel::Failed,
            SettlementBasis::Unavailable,
            Execution::Failed,
            Truth::Unavailable,
        )
    }
}

/// FR-235: settle `item`, run over the package `package_id`, as one
/// terminal record. The map is exhaustive with no `_` arm:
///
/// - V-1: `Holds` whose certificate `check` accepts is
///   `Proved{ZoneCertified, Certified}`, `closed-scope`;
/// - V-6: a rejected certificate is `Inconclusive(CertificateRejected)`;
///   `Undecided` is `Inconclusive` with its cause; a replay that settles
///   `inconclusive` is `ReplayParity`, and one that refuses is
///   `ReplayRefused` (a fault is `Failed`);
/// - V-4: `Violated` whose replay reproduces is `Refuted`,
///   `decisive-counterexample`, an undefined evaluation's carrying its
///   cause on the counterexample;
/// - V-10: a reproduced time-lock is `Refuted`, `closed-scope`;
/// - V-7: `Stopped`, or a certificate check or replay a budget stopped, is
///   `Incomplete` naming the limit and its value, or `Cancelled` with
///   `cancel`'s cause;
/// - V-8: an `Unsupported` disposition is `Unsupported` with its cause.
pub fn settle_zone_check<S>(
    item: ZoneItem<'_, S>,
    package_id: &DigestRecord,
    cancel: &Cancel,
    check: impl FnOnce(&ZoneCertificate<S>) -> ZoneCertificateCheck,
    replay: impl FnOnce(&TemporalCounterexample) -> Result<ModelTraceReplayResult, ReplayRefusal>,
) -> ZoneCheckRecord {
    let outcome = match item {
        ZoneItem::Unsupported(disposed) => {
            return ZoneCheckRecord::row(
                TerminalValue::Unsupported(UnavailabilityCause::from(disposed)),
                ResultLabel::Unsupported,
                SettlementBasis::Unavailable,
                Execution::Completed,
                Truth::Unavailable,
            )
        }
        ZoneItem::Outcome(outcome) => outcome,
    };
    match outcome {
        ZoneCheckOutcome::Holds { certificate, .. } => match check(certificate) {
            ZoneCertificateCheck::Accepted => ZoneCheckRecord::row(
                TerminalValue::Proved {
                    basis: ProofBasis::ZoneCertified,
                    certification: Certification::Certified,
                },
                ResultLabel::Proved,
                SettlementBasis::ClosedScope,
                Execution::Completed,
                Truth::Satisfied,
            ),
            ZoneCertificateCheck::Rejected { rule, at } => {
                ZoneCheckRecord::inconclusive(InconclusiveCause::CertificateRejected { rule, at })
            }
            ZoneCertificateCheck::Stopped(cause) => ZoneCheckRecord::incomplete(cause),
        },
        ZoneCheckOutcome::Undecided { cause, .. } => ZoneCheckRecord::inconclusive(match cause {
            ZoneUndecided::NoInitialState => InconclusiveCause::NoInitialState,
            ZoneUndecided::UndecidedSuccessor(_) => InconclusiveCause::UndecidedSuccessor,
            ZoneUndecided::NoAdmittedBehaviour => InconclusiveCause::NoAdmittedBehaviour,
            ZoneUndecided::LassoNotConcretized => InconclusiveCause::LassoNotConcretized,
            ZoneUndecided::DeadlockAfterDelay => InconclusiveCause::DeadlockAfterDelay,
        }),
        ZoneCheckOutcome::Stopped { cause, .. } => match cause {
            ZoneStop::LimitReached(limit) => {
                let (limit, value) = match *limit {
                    ZoneCheckLimit::MaxSymbolicStates(value) => {
                        (ModelCheckLimit::ZoneMaxSymbolicStates, value)
                    }
                    ZoneCheckLimit::MaxSymbolicTransitions(value) => {
                        (ModelCheckLimit::ZoneMaxSymbolicTransitions, value)
                    }
                    ZoneCheckLimit::MaxAutomatonStates(value) => {
                        (ModelCheckLimit::ZoneMaxAutomatonStates, value)
                    }
                    ZoneCheckLimit::MaxZoneBytes(value) => {
                        (ModelCheckLimit::ZoneMaxZoneBytes, value)
                    }
                };
                ZoneCheckRecord::incomplete(IncompleteCause::LimitReached { limit, value })
            }
            ZoneStop::Cancelled => match cancel.cause() {
                Some(source) => ZoneCheckRecord::incomplete(IncompleteCause::Cancelled { source }),
                None => ZoneCheckRecord::failed(),
            },
        },
        ZoneCheckOutcome::Violated { counterexample, .. } => match replay(counterexample) {
            Ok(ModelTraceReplayResult {
                package_id: replayed,
                ..
            }) if !replayed.matches(package_id) => ZoneCheckRecord::inconclusive(
                InconclusiveCause::ReplayRefused(qsl_foundation::diagnostic::Code::StaleDependency),
            ),
            Ok(ModelTraceReplayResult {
                settlement: ModelTraceSettlement::Reproduced { .. },
                ..
            }) => {
                let basis = match counterexample.kind {
                    CounterexampleKind::TimeLock => SettlementBasis::ClosedScope,
                    CounterexampleKind::Formula
                    | CounterexampleKind::Deadlock
                    | CounterexampleKind::Race(_)
                    | CounterexampleKind::UndefinedEvaluation(_) => {
                        SettlementBasis::DecisiveCounterexample
                    }
                };
                ZoneCheckRecord {
                    counterexample: Some(counterexample.as_ref().clone()),
                    ..ZoneCheckRecord::row(
                        TerminalValue::Refuted,
                        ResultLabel::Refuted,
                        basis,
                        Execution::Completed,
                        Truth::Violated,
                    )
                }
            }
            Ok(ModelTraceReplayResult {
                settlement: ModelTraceSettlement::Inconclusive(cause),
                ..
            }) => ZoneCheckRecord::inconclusive(InconclusiveCause::ReplayParity(cause)),
            Ok(ModelTraceReplayResult {
                settlement: ModelTraceSettlement::Stopped { reached, .. },
                ..
            }) => ZoneCheckRecord::incomplete(IncompleteCause::LimitReached {
                limit: reached.limit,
                value: reached.value,
            }),
            // A fault stays a tool failure (ADR-013 C-09).
            Err(refusal) => match TerminalValue::from_replay_refusal(&refusal) {
                TerminalValue::Inconclusive(cause) => ZoneCheckRecord::inconclusive(cause),
                _ => ZoneCheckRecord::failed(),
            },
        },
    }
}
