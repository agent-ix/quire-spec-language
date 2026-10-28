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
    CertificateRejection, CounterexampleKind, IncompleteCause, InconclusiveCause, ModelCheckLimit,
    ModelTraceReplayResult, ModelTraceSettlement, ProofBasis, ReachedLimit,
    RefinementTraceSettlement, ReplayRefusal, ResultLabel, SettlementBasis, TemporalCounterexample,
    TerminalValue, Truth, UnavailabilityCause, ZoneCertificate, ZoneCertificateVerdict,
};
use qsl_replay::{Certification, Execution};
use qsl_route::deadlock::ItemObligation;
use qsl_semantics::family::UnsupportedItem;
use quire_exact::Cancel;

use super::refine::{TimedRefinementStop, TimedRefinementUndecided, TimedSafetyOutcome};
use super::search::{ZoneCheckLimit, ZoneCheckOutcome, ZoneReduction, ZoneStop, ZoneUndecided};

/// The settlement method of an EN-6 record (FR-235).
pub const ZONE_CERTIFIED: &str = "zone-certified";

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
///   `Undecided` is `Inconclusive` with the cause FR-127 or FR-235 names
///   (a deadlock only after a delay as `LassoNotConcretized`, a window
///   short of data as `UndecidedSuccessor`, both spec gap STD-493); a
///   replay that settles
///   `inconclusive` is `ReplayParity`, and one that refuses is
///   `ReplayRefused` (a fault is `Failed`);
/// - V-4: `Violated` whose replay reproduces is `Refuted`,
///   `decisive-counterexample`, an undefined evaluation's carrying its
///   cause on the counterexample;
/// - V-10: a reproduced time-lock is `Refuted`, `closed-scope`;
/// - V-7: `Stopped`, or a certificate check or replay a budget stopped, is
///   `Incomplete` naming the limit and its value, or `Cancelled` with
///   `cancel`'s cause;
/// - V-8: an `Unsupported` disposition is `Unsupported` with its cause,
///   and a reduction EN-6 does not apply is `UnsupportedReduction`;
/// - FR-160: a symmetry-reduced `Holds` is `Proved{Reduced, Uncertified}`,
///   and `SymmetryBroken` is `Inconclusive`.
pub fn settle_zone_check<S>(
    item: ZoneItem<'_, S>,
    package_id: &DigestRecord,
    cancel: &Cancel,
    check: impl FnOnce(&ZoneCertificate<ItemObligation<S>>) -> ZoneCertificateVerdict,
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
        ZoneCheckOutcome::Holds { certificate, .. } => certified(check(certificate)),
        // FR-160 (QSpec FR-430): a symmetry-reduced proof, which no core
        // checker reads. FR-235's map has no reduced row (spec gap
        // STD-493); FR-160's is used.
        ZoneCheckOutcome::HoldsReduced { reductions, .. } => ZoneCheckRecord::row(
            TerminalValue::Proved {
                basis: ProofBasis::Reduced {
                    reductions: reductions.clone(),
                },
                certification: Certification::Uncertified,
            },
            ResultLabel::Proved,
            SettlementBasis::ClosedScope,
            Execution::Completed,
            Truth::Satisfied,
        ),
        ZoneCheckOutcome::Reduction { cause, .. } => match cause {
            // V-8 (QSpec FR-430): a reduction EN-6 does not apply.
            ZoneReduction::Unsupported { reduction } => ZoneCheckRecord::row(
                TerminalValue::Unsupported(UnavailabilityCause::UnsupportedReduction {
                    reduction: *reduction,
                }),
                ResultLabel::Unsupported,
                SettlementBasis::Unavailable,
                Execution::Completed,
                Truth::Unavailable,
            ),
            // FR-160's V-6 row.
            ZoneReduction::SymmetryBroken { population, cause } => {
                ZoneCheckRecord::inconclusive(InconclusiveCause::SymmetryBroken {
                    population: population.clone(),
                    cause: cause.clone(),
                })
            }
        },
        ZoneCheckOutcome::Undecided { cause, .. } => {
            ZoneCheckRecord::inconclusive(undecided_cause(cause))
        }
        ZoneCheckOutcome::Stopped { cause, .. } => stopped(cause, cancel),
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

/// FR-235's V-6 cause of an EN-6 run that decided nothing.
fn undecided_cause(cause: &ZoneUndecided) -> InconclusiveCause {
    match cause {
        // FR-127's V-6 rows, which FR-235 extends.
        ZoneUndecided::NoInitialState => InconclusiveCause::NoInitialState,
        ZoneUndecided::UndecidedSuccessor(_) => InconclusiveCause::UndecidedSuccessor,
        // FR-235's V-6 rows.
        ZoneUndecided::NoAdmittedBehaviour => InconclusiveCause::NoAdmittedBehaviour,
        ZoneUndecided::LassoNotConcretized => InconclusiveCause::LassoNotConcretized,
        // Spec gap STD-493: a violation found with no counterexample
        // form, as FR-241's lasso that does not concretise.
        ZoneUndecided::DeadlockAfterDelay => InconclusiveCause::LassoNotConcretized,
        // Spec gap STD-493: a reading with no value (ADR-024 WA-4), as
        // FR-127's undecided successor reading.
        ZoneUndecided::InsufficientData(_) => InconclusiveCause::UndecidedSuccessor,
    }
}

/// FR-235's V-7 record of an EN-6 run that stopped.
fn stopped(cause: &ZoneStop, cancel: &Cancel) -> ZoneCheckRecord {
    match cause {
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
                ZoneCheckLimit::MaxZoneBytes(value) => (ModelCheckLimit::ZoneMaxZoneBytes, value),
                ZoneCheckLimit::MaxClocks(value) => (ModelCheckLimit::MaxClocks, value),
            };
            ZoneCheckRecord::incomplete(IncompleteCause::LimitReached { limit, value })
        }
        ZoneStop::Cancelled => match cancel.cause() {
            Some(source) => ZoneCheckRecord::incomplete(IncompleteCause::Cancelled { source }),
            None => ZoneCheckRecord::failed(),
        },
    }
}

/// The record of a holding run whose certificate `check` judged: V-1 only
/// when FR-245 accepts it (ADR-026 CF-4).
fn certified(verdict: ZoneCertificateVerdict) -> ZoneCheckRecord {
    match verdict {
        ZoneCertificateVerdict::Accepted => ZoneCheckRecord::row(
            TerminalValue::Proved {
                basis: ProofBasis::ZoneCertified,
                certification: Certification::Certified,
            },
            ResultLabel::Proved,
            SettlementBasis::ClosedScope,
            Execution::Completed,
            Truth::Satisfied,
        ),
        ZoneCertificateVerdict::Rejected(CertificateRejection { rule, at }) => {
            ZoneCheckRecord::inconclusive(InconclusiveCause::CertificateRejected { rule, at })
        }
        ZoneCertificateVerdict::Stopped(cause) => ZoneCheckRecord::incomplete(cause),
    }
}

/// QSpec FR-430 over FR-235: settle a timed refinement's safety half. A
/// `Holds` settles `Proved{ZoneCertified, Certified}` only once `check`,
/// FR-245's checker of its refinement-form certificate, accepts it; a
/// `Violated` settles `refuted` only once `replay`, FR-145's timed replay,
/// reproduces it. Every other outcome settles as FR-235's map settles
/// EN-6's, and a `check_step` budget as V-7 naming it.
pub fn settle_timed_safety<O>(
    outcome: &TimedSafetyOutcome<O>,
    cancel: &Cancel,
    check: impl FnOnce(&ZoneCertificate<O>) -> ZoneCertificateVerdict,
    replay: impl FnOnce(&TemporalCounterexample) -> Result<RefinementTraceSettlement, ReplayRefusal>,
) -> ZoneCheckRecord {
    match outcome {
        TimedSafetyOutcome::Holds { certificate, .. } => certified(check(certificate)),
        TimedSafetyOutcome::Violated { counterexample, .. } => match replay(counterexample) {
            Ok(RefinementTraceSettlement::Reproduced { .. }) => ZoneCheckRecord {
                counterexample: Some(counterexample.as_ref().clone()),
                ..ZoneCheckRecord::row(
                    TerminalValue::Refuted,
                    ResultLabel::Refuted,
                    SettlementBasis::DecisiveCounterexample,
                    Execution::Completed,
                    Truth::Violated,
                )
            },
            Ok(RefinementTraceSettlement::Inconclusive(cause)) => {
                ZoneCheckRecord::inconclusive(InconclusiveCause::ReplayParity(cause))
            }
            Ok(RefinementTraceSettlement::Stopped(reached)) => {
                ZoneCheckRecord::incomplete(IncompleteCause::LimitReached {
                    limit: reached.limit,
                    value: reached.value,
                })
            }
            Err(refusal) => match TerminalValue::from_replay_refusal(&refusal) {
                TerminalValue::Inconclusive(cause) => ZoneCheckRecord::inconclusive(cause),
                _ => ZoneCheckRecord::failed(),
            },
        },
        TimedSafetyOutcome::Undecided { cause, .. } => ZoneCheckRecord::inconclusive(match cause {
            TimedRefinementUndecided::Zone(cause) => undecided_cause(cause),
            TimedRefinementUndecided::Undetermined(undetermined) => undetermined.cause(),
        }),
        TimedSafetyOutcome::Stopped { cause, .. } => match cause {
            TimedRefinementStop::Zone(cause) => stopped(cause, cancel),
            TimedRefinementStop::Step(reached) => {
                ZoneCheckRecord::incomplete(IncompleteCause::LimitReached {
                    limit: reached.limit,
                    value: reached.value,
                })
            }
        },
        TimedSafetyOutcome::Fault { .. } => ZoneCheckRecord::failed(),
    }
}
