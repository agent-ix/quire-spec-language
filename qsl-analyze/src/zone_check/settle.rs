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
    TerminalValue, Truth, UnavailabilityCause, ZoneCertificate, ZoneCertificateRefusal,
    ZoneCertificateVerdict,
};
use qsl_replay::{Certification, Execution};
use qsl_route::deadlock::ItemObligation;
use qsl_semantics::family::UnsupportedItem;
use quire_exact::Cancel;

use super::refine::{
    TimedLivenessHalf, TimedLivenessOutcome, TimedRefinementOutcome, TimedRefinementStop,
    TimedRefinementUndecided, TimedSafetyOutcome,
};
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
/// - V-6: a rejected certificate is `Inconclusive(CertificateRejected)`,
///   and one the checker refuses as naming another item or identity is
///   `Inconclusive(ReplayRefused(stale_dependency))`, as FR-127's row for a
///   certificate of another package; any other checker refusal is `Failed`;
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
    check: impl FnOnce(
        &ZoneCertificate<ItemObligation<S>>,
    ) -> Result<ZoneCertificateVerdict, ZoneCertificateRefusal<ItemObligation<S>>>,
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
        ZoneCheckOutcome::Stopped { cause, .. } => stopped(*cause, cancel),
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

/// The V-6 cause of an undecided EN-6 run.
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

/// The V-7 record of an EN-6 run that `cause` stopped.
fn stopped(cause: ZoneStop, cancel: &Cancel) -> ZoneCheckRecord {
    match cause {
        ZoneStop::LimitReached(limit) => {
            let (limit, value) = match limit {
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

/// FR-235's rows for a `Holds` by what FR-245's checker returned for its
/// certificate: `Proved{ZoneCertified, Certified}` only when it accepts.
fn certified<O>(
    checked: Result<ZoneCertificateVerdict, ZoneCertificateRefusal<O>>,
) -> ZoneCheckRecord {
    match checked {
        Ok(ZoneCertificateVerdict::Accepted) => ZoneCheckRecord::row(
            TerminalValue::Proved {
                basis: ProofBasis::ZoneCertified,
                certification: Certification::Certified,
            },
            ResultLabel::Proved,
            SettlementBasis::ClosedScope,
            Execution::Completed,
            Truth::Satisfied,
        ),
        Ok(ZoneCertificateVerdict::Rejected(CertificateRejection { rule, at })) => {
            ZoneCheckRecord::inconclusive(InconclusiveCause::CertificateRejected { rule, at })
        }
        Ok(ZoneCertificateVerdict::Stopped(cause)) => ZoneCheckRecord::incomplete(cause),
        Err(
            ZoneCertificateRefusal::ObligationMismatch { .. }
            | ZoneCertificateRefusal::IdentityMismatch { .. }
            | ZoneCertificateRefusal::RefinementMismatch { .. },
        ) => ZoneCheckRecord::inconclusive(InconclusiveCause::ReplayRefused(
            qsl_foundation::diagnostic::Code::StaleDependency,
        )),
        // A checker that cannot read the certificate's item is a tool
        // failure (ADR-013 C-09), never a verdict.
        Err(
            ZoneCertificateRefusal::Admission(_)
            | ZoneCertificateRefusal::Package(_)
            | ZoneCertificateRefusal::TimedSubject(_)
            | ZoneCertificateRefusal::UnsupportedItem
            | ZoneCertificateRefusal::Unbounded
            | ZoneCertificateRefusal::Undecided
            | ZoneCertificateRefusal::Fault(_),
        ) => ZoneCheckRecord::failed(),
    }
}

/// FR-144 and QSpec FR-430: settle EN-6's timed refinement item as one
/// terminal record. Its safety half settles by FR-144's rows over EN-6's
/// causes:
///
/// - V-1: `Holds` settles `Proved{ZoneCertified, Certified}` only when
///   `check` (FR-245 over FR-244's refinement form) accepts its
///   certificate, by [`settle_zone_check`]'s certificate rows;
/// - V-4: `Violated` settles `refuted`, `decisive-counterexample`, only
///   when `replay` (FR-145 over FR-241's delays) reproduces it; a replay
///   that disagrees is `ReplayParity`, one that a budget stops is V-7
///   naming the limit, and a refusal is `ReplayRefused` (a fault `failed`);
/// - V-6: `Undecided` is `inconclusive` with the zone search's cause, or
///   with the mapping's (`mapping-undetermined`, `undecided-successor`);
/// - V-7: `Stopped` is `incomplete` naming the EN-6 or step budget, or the
///   cancellation;
/// - an evaluator fault is `failed`.
///
/// The item's value is the safety half's unless the safety half is
/// `proved`, and then it is the liveness half's (FR-144, as
/// `settle_refinement` settles an untimed item): a liveness half that holds
/// is `Proved{Exhaustive, Uncertified}`, since no core checker covers it
/// (ADR-020 CT-4); a `Violated` one is `refuted` only through `replay` of its
/// lasso; an `Unsupported` one is V-8 with its cause (ADR-020 AX-5); and
/// otherwise it takes the safety half's rows.
pub fn settle_timed_refinement<S>(
    outcome: &TimedRefinementOutcome<S>,
    cancel: &Cancel,
    check: impl FnOnce(
        &ZoneCertificate<ItemObligation<S>>,
    ) -> Result<ZoneCertificateVerdict, ZoneCertificateRefusal<ItemObligation<S>>>,
    replay: impl FnOnce(&TemporalCounterexample) -> Result<RefinementTraceSettlement, ReplayRefusal>,
) -> ZoneCheckRecord {
    let liveness = match (&outcome.safety, &outcome.liveness) {
        (TimedSafetyOutcome::Holds { certificate, .. }, Some(liveness)) => {
            let safety = certified(check(certificate));
            if !matches!(safety.value, TerminalValue::Proved { .. }) {
                return safety;
            }
            liveness
        }
        (safety, _) => return timed_safety(safety, cancel, check, replay),
    };
    match liveness {
        // ADR-020 AX-5: the second phase did not run.
        TimedLivenessHalf::Unsupported(cause) => ZoneCheckRecord::row(
            TerminalValue::Unsupported(cause.clone()),
            ResultLabel::Unsupported,
            SettlementBasis::Unavailable,
            Execution::Completed,
            Truth::Unavailable,
        ),
        // No core checker covers the liveness half (ADR-020 CT-4).
        TimedLivenessHalf::Checked(TimedLivenessOutcome::Holds { .. }) => ZoneCheckRecord::row(
            TerminalValue::Proved {
                basis: ProofBasis::Exhaustive,
                certification: Certification::Uncertified,
            },
            ResultLabel::Proved,
            SettlementBasis::ClosedScope,
            Execution::Completed,
            Truth::Satisfied,
        ),
        TimedLivenessHalf::Checked(TimedLivenessOutcome::Violated { counterexample, .. }) => {
            violated(counterexample, replay)
        }
        TimedLivenessHalf::Checked(TimedLivenessOutcome::Undecided { cause, .. }) => {
            undecided(cause)
        }
        TimedLivenessHalf::Checked(TimedLivenessOutcome::Stopped { cause, .. }) => {
            timed_stopped(*cause, cancel)
        }
        TimedLivenessHalf::Checked(TimedLivenessOutcome::Fault { .. }) => ZoneCheckRecord::failed(),
    }
}

/// The safety half's record, by FR-144's rows over EN-6's causes.
fn timed_safety<S>(
    outcome: &TimedSafetyOutcome<S>,
    cancel: &Cancel,
    check: impl FnOnce(
        &ZoneCertificate<ItemObligation<S>>,
    ) -> Result<ZoneCertificateVerdict, ZoneCertificateRefusal<ItemObligation<S>>>,
    replay: impl FnOnce(&TemporalCounterexample) -> Result<RefinementTraceSettlement, ReplayRefusal>,
) -> ZoneCheckRecord {
    match outcome {
        TimedSafetyOutcome::Holds { certificate, .. } => certified(check(certificate)),
        TimedSafetyOutcome::Violated { counterexample, .. } => violated(counterexample, replay),
        TimedSafetyOutcome::Undecided { cause, .. } => undecided(cause),
        TimedSafetyOutcome::Stopped { cause, .. } => timed_stopped(*cause, cancel),
        TimedSafetyOutcome::Fault { .. } => ZoneCheckRecord::failed(),
    }
}

/// A half's counterexample: `refuted`, `decisive-counterexample`, only when
/// `replay` (FR-145 over FR-241's delays) reproduces it.
fn violated(
    counterexample: &TemporalCounterexample,
    replay: impl FnOnce(&TemporalCounterexample) -> Result<RefinementTraceSettlement, ReplayRefusal>,
) -> ZoneCheckRecord {
    match replay(counterexample) {
        Ok(RefinementTraceSettlement::Reproduced { .. }) => ZoneCheckRecord {
            counterexample: Some(counterexample.clone()),
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
        // A fault stays a tool failure (ADR-013 C-09).
        Err(refusal) => match TerminalValue::from_replay_refusal(&refusal) {
            TerminalValue::Inconclusive(cause) => ZoneCheckRecord::inconclusive(cause),
            _ => ZoneCheckRecord::failed(),
        },
    }
}

/// A half that decided nothing: `inconclusive` with the zone search's
/// cause or the mapping's.
fn undecided(cause: &TimedRefinementUndecided) -> ZoneCheckRecord {
    ZoneCheckRecord::inconclusive(match cause {
        TimedRefinementUndecided::Zone(zone) => undecided_cause(zone),
        TimedRefinementUndecided::Undetermined(undetermined) => undetermined.cause(),
    })
}

/// A half that stopped: `incomplete` naming the EN-6 or step budget, or
/// the cancellation.
fn timed_stopped(cause: TimedRefinementStop, cancel: &Cancel) -> ZoneCheckRecord {
    match cause {
        TimedRefinementStop::Zone(zone) => stopped(zone, cancel),
        TimedRefinementStop::Step(reached) => {
            ZoneCheckRecord::incomplete(IncompleteCause::LimitReached {
                limit: reached.limit,
                value: reached.value,
            })
        }
    }
}
