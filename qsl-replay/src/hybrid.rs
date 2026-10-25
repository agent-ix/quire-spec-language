// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-250 (ADR-026 HY-1 to HY-3): a hybrid solver result settles as an FR-331
//! terminal value. A result settles `proved` only where its method kind makes
//! it sound for its outcome, and that proof is `uncertified`: a solver QSL
//! runs has no core certificate checker (ADR-018 PC-1). Every other result is
//! `inconclusive` or `incomplete`. A solver result settles `refuted` only
//! through FR-450's exact replay of the trace it reports
//! ([`SolverResult::settle`]), and a provider's own claim about its
//! soundness is never read: [`SolverMethod`] has no such field.

use quire_exact::{Integer, NonZeroInteger, Rational};
use serde::ser::SerializeStruct;
use serde::{Serialize, Serializer};
use std::collections::BTreeSet;

use crate::hybrid_replay::{
    exact_replay, static_replay, ExactRecord, ExactReplay, HybridCounterexample, HybridReplayMeter,
    HybridReplayRefusal, HybridTrace, ReplayRecord,
};
use crate::model_check::limits::ModelCheckLimit;
use crate::proof_result::{
    Certification, IncompleteCause, InconclusiveCause, ProofBasis, SettlementBasis, TerminalValue,
    UnavailabilityCause,
};
use crate::result::{DisagreementCause, HybridTraceDisagreement};
use qsl_semantics::check::{CheckedGraph, CheckedHybridClaim, HybridClock, HybridHorizon};
use quire_exact::LimitKind;

mod process_request;
pub use process_request::{
    AdmittedHybridProcessItem, AdmittedHybridProcessRequest, HybridProcessItem,
    HybridProcessRequest, HybridProcessRequestRefusal,
};

/// The kind of method a provider ran. QSL decides soundness from the kind
/// (QSpec FR-193), never from a provider's say-so.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SolverMethod {
    /// A δ-complete solver: its `unsat` is sound for bounded unreachability.
    DeltaComplete,
    /// A flowpipe computed with outward rounding: its enclosure is sound.
    FlowpipeOutwardRounded,
    /// A flowpipe without outward rounding: no result of it is sound.
    FlowpipeUnrounded,
    /// An inductive enclosure proves every time and jump count.
    InductiveEnclosure,
    /// A method outside QSpec FR-193's classified solver methods.
    Unclassified,
}

impl SolverMethod {
    /// The method's wire spelling in a `bounded-solver` basis.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::DeltaComplete => "delta-complete",
            Self::FlowpipeOutwardRounded => "flowpipe-outward-rounded",
            Self::FlowpipeUnrounded => "flowpipe-unrounded",
            Self::InductiveEnclosure => "inductive-enclosure",
            Self::Unclassified => "unclassified",
        }
    }

    /// Whether the method, outcome and coverage form a sound proof triple.
    pub fn is_sound_for(self, outcome: &SolverOutcome, coverage: &SolverCoverage) -> bool {
        matches!(
            (self, outcome, coverage),
            (
                Self::DeltaComplete,
                SolverOutcome::Unsat,
                SolverCoverage::Bounded { .. }
            ) | (
                Self::FlowpipeOutwardRounded,
                SolverOutcome::EnclosureDisjoint,
                SolverCoverage::Bounded { .. }
            ) | (
                Self::InductiveEnclosure,
                SolverOutcome::EnclosureDisjoint,
                SolverCoverage::Inductive
            )
        )
    }
}

/// The dimension of a bounded solver result established complete.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum BoundDimension {
    /// Every behavior's jumps through the given time are within `jumps`.
    Time,
    /// Every behavior's first `jumps` jumps occur within `horizon`.
    Jumps,
}

impl Serialize for SolverMethod {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

/// The budget that stopped a solver. A time stop carries no seconds, so a
/// stored record never depends on the requested limit.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SolverStop {
    /// The wall-clock budget ran out: settles `incomplete`, `timed-out`.
    Time,
    /// The step budget ran out after this many steps: settles `incomplete`,
    /// `limit-reached` naming `solver-steps`.
    Steps(u64),
}

/// A provider's outcome (FR-250 Inputs). The outcomes that report a possible
/// violation carry the path the solver reports to it, if any.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SolverOutcome {
    /// The unsafe set is unreachable.
    Unsat,
    /// A δ-satisfying point exists: a possible violation, not a witness.
    DeltaSat(Option<Box<HybridTrace>>),
    /// A flowpipe enclosure is disjoint from the unsafe set.
    EnclosureDisjoint,
    /// A flowpipe enclosure meets the unsafe set.
    EnclosureMeetsUnsafe(Option<Box<HybridTrace>>),
    /// The solver could not decide.
    Unknown(Option<Box<HybridTrace>>),
    /// The model has no initial state; no behavior establishes a proof.
    NoInitialState,
    /// A budget stopped the solver.
    Stopped(SolverStop),
}

impl SolverOutcome {
    /// The path to the possible violation the solver reported, if any.
    pub fn trace(&self) -> Option<&HybridTrace> {
        match self {
            Self::DeltaSat(trace) | Self::EnclosureMeetsUnsafe(trace) | Self::Unknown(trace) => {
                trace.as_deref()
            }
            Self::Unsat | Self::EnclosureDisjoint | Self::NoInitialState | Self::Stopped(_) => None,
        }
    }
}

/// A non-negative time horizon, an exact rational number of seconds.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct SolverHorizon(Rational);

/// A negative horizon was offered.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NegativeHorizon;

impl SolverHorizon {
    /// `seconds` as a horizon; a negative number is refused.
    pub fn new(seconds: Rational) -> Result<Self, NegativeHorizon> {
        if seconds.numerator().is_negative() {
            Err(NegativeHorizon)
        } else {
            Ok(Self(seconds))
        }
    }

    /// The horizon in seconds.
    pub const fn seconds(&self) -> &Rational {
        &self.0
    }
}

impl Serialize for SolverHorizon {
    /// FR-250's `ExactRational` wire: decimal-string components, so a
    /// numerator beyond 2^53 keeps its precision for JSON consumers.
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut object = serializer.serialize_struct("SolverHorizon", 2)?;
        object.serialize_field("numerator", &self.0.numerator().to_string())?;
        object.serialize_field("denominator", &self.0.denominator().to_string())?;
        object.end()
    }
}

/// The clock and horizon of the checked hybrid claim whose proof is being
/// settled. A missing horizon names an unbounded claim.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ClaimHorizon {
    /// A duration in exact seconds under `model-time`.
    ModelTime(Option<SolverHorizon>),
    /// A count of positions under `model-steps`, never converted to seconds.
    ModelSteps(Option<u64>),
}

impl ClaimHorizon {
    /// The clock and horizon of `claim`, as S3 recorded them (FR-447
    /// "Horizon"); coverage is decided from nothing else. A horizon whose
    /// kind does not fit the clock, which S3 never records, reads as
    /// unbounded, which no bounded proof covers.
    pub fn of(claim: &CheckedHybridClaim) -> Self {
        match (claim.clock, &claim.horizon) {
            (HybridClock::ModelTime, Some(HybridHorizon::Seconds(seconds))) => {
                Self::ModelTime(SolverHorizon::new(seconds.clone()).ok())
            }
            (HybridClock::ModelSteps, Some(HybridHorizon::Positions(positions))) => {
                Self::ModelSteps(Some(*positions))
            }
            (HybridClock::ModelSteps, _) => Self::ModelSteps(None),
            (HybridClock::ModelTime, _) => Self::ModelTime(None),
        }
    }
}

/// The part of a hybrid behavior a solver result established (FR-250).
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum SolverCoverage {
    /// A finite time and jump bound, with the dimensions established complete.
    Bounded {
        /// The covered duration in exact seconds.
        horizon: SolverHorizon,
        /// The covered jump count.
        jumps: u64,
        /// Completeness established by the provider.
        exhaustive: BTreeSet<BoundDimension>,
    },
    /// Every time and every jump count.
    Inductive,
}

impl SolverCoverage {
    /// Whether this coverage includes every state a claim can read.
    pub fn covers(&self, claim: &ClaimHorizon) -> bool {
        match (self, claim) {
            (Self::Inductive, _) => true,
            (
                Self::Bounded {
                    horizon,
                    exhaustive,
                    ..
                },
                ClaimHorizon::ModelTime(Some(required)),
            ) => {
                horizon.seconds() >= required.seconds()
                    && exhaustive.contains(&BoundDimension::Time)
            }
            (
                Self::Bounded {
                    jumps, exhaustive, ..
                },
                ClaimHorizon::ModelSteps(Some(required)),
            ) => {
                *jumps >= *required
                    && (*required == 0 || exhaustive.contains(&BoundDimension::Jumps))
            }
            (
                Self::Bounded { .. },
                ClaimHorizon::ModelTime(None) | ClaimHorizon::ModelSteps(None),
            ) => false,
        }
    }
}

/// A provider's result (FR-250 Inputs).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SolverResult {
    /// The kind of method that produced the outcome.
    pub method: SolverMethod,
    /// The outcome.
    pub outcome: SolverOutcome,
    /// The behavior range established by the solver.
    pub coverage: SolverCoverage,
}

impl SolverResult {
    /// FR-250's method and coverage map for a checked claim, before any
    /// replay. A sound proof settles `proved` only when its coverage includes
    /// the claim; a sound bounded result that falls short names its actual
    /// bounds. A reported trace is kept unreplayed, so this is private:
    /// [`Self::settle`] admits and replays it first. No provider assertion of
    /// soundness is read.
    fn terminal_value(&self, claim: &ClaimHorizon) -> TerminalValue {
        match &self.outcome {
            SolverOutcome::Stopped(SolverStop::Time) => {
                TerminalValue::Incomplete(IncompleteCause::TimedOut)
            }
            SolverOutcome::Stopped(SolverStop::Steps(value)) => {
                TerminalValue::Incomplete(IncompleteCause::LimitReached {
                    limit: ModelCheckLimit::SolverSteps,
                    value: *value,
                })
            }
            SolverOutcome::NoInitialState => {
                TerminalValue::Inconclusive(InconclusiveCause::NoInitialState)
            }
            outcome if self.method.is_sound_for(outcome, &self.coverage) => match &self.coverage {
                SolverCoverage::Bounded {
                    horizon,
                    jumps,
                    exhaustive,
                } if !self.coverage.covers(claim) => {
                    TerminalValue::Inconclusive(InconclusiveCause::InsufficientHorizon {
                        horizon: horizon.clone(),
                        jumps: *jumps,
                        exhaustive: exhaustive.clone(),
                    })
                }
                SolverCoverage::Bounded {
                    horizon,
                    jumps,
                    exhaustive,
                } => TerminalValue::Proved {
                    basis: ProofBasis::BoundedSolver {
                        method: self.method,
                        horizon: horizon.clone(),
                        jumps: *jumps,
                        exhaustive: exhaustive.clone(),
                    },
                    certification: Certification::Uncertified,
                },
                SolverCoverage::Inductive => TerminalValue::Proved {
                    basis: ProofBasis::InductiveSolver {
                        method: self.method,
                    },
                    certification: Certification::Uncertified,
                },
            },
            outcome => TerminalValue::Inconclusive(InconclusiveCause::SolverInconclusive {
                trace: outcome.trace().cloned().map(Box::new),
                replay: None,
            }),
        }
    }

    /// FR-250 with FR-450: the map of `Self::terminal_value`, and for a
    /// reported trace its replay against the claim's model in `graph`. The
    /// trace is admitted first (QSpec FR-193 "Hybrid trace"); static replay
    /// keeps its outcome with the trace on the `inconclusive` record, and a
    /// trace exact replay reproduces settles `refuted` with its
    /// counterexample, the only path to `refuted`. A replay that exhausts the
    /// caller's `work_units` meter settles `incomplete`, with no verdict or
    /// counterexample.
    ///
    /// # Errors
    ///
    /// [`HybridReplayRefusal`] when the trace breaks the correspondence
    /// rules, or the claim's model or a called function cannot be read.
    pub fn settle(
        &self,
        graph: &CheckedGraph,
        claim: &CheckedHybridClaim,
        meter: &mut HybridReplayMeter,
    ) -> Result<HybridSettlement, HybridReplayRefusal> {
        let value = self.terminal_value(&ClaimHorizon::of(claim));
        let Some(trace) = self.outcome.trace() else {
            return Ok(HybridSettlement::without_counterexample(value));
        };
        if !matches!(
            value,
            TerminalValue::Inconclusive(InconclusiveCause::SolverInconclusive { .. })
        ) {
            return Ok(HybridSettlement::without_counterexample(value));
        }
        let exhausted = |meter: &HybridReplayMeter| {
            HybridSettlement::without_counterexample(TerminalValue::Incomplete(
                IncompleteCause::LimitReached {
                    limit: ModelCheckLimit::EvaluationMeter {
                        counter: LimitKind::WorkUnits,
                    },
                    value: meter.work_units,
                },
            ))
        };
        let static_outcome = match static_replay(graph, claim, trace, meter) {
            Ok(outcome) => outcome,
            Err(HybridReplayRefusal::ResourceExhausted { .. }) => return Ok(exhausted(meter)),
            Err(refusal) => return Err(refusal),
        };
        let exact = match exact_replay(graph, claim, trace, meter) {
            Ok(ExactReplay::Reproduced(counterexample)) => {
                return Ok(HybridSettlement {
                    value: TerminalValue::Refuted,
                    counterexample: Some(*counterexample),
                });
            }
            Ok(ExactReplay::NotReproduced { at, check }) => {
                Some(ExactRecord::NotReproduced { at, check })
            }
            Ok(ExactReplay::NotExact) => Some(ExactRecord::NotExact),
            Err(HybridReplayRefusal::ResourceExhausted { .. }) => return Ok(exhausted(meter)),
            Err(refusal) => return Err(refusal),
        };
        Ok(HybridSettlement::without_counterexample(
            TerminalValue::Inconclusive(InconclusiveCause::SolverInconclusive {
                trace: Some(Box::new(trace.clone())),
                replay: Some(Box::new(ReplayRecord {
                    static_outcome,
                    exact,
                })),
            }),
        ))
    }

    /// The QSpec FR-243 basis the result settles with: `closed-scope` for a
    /// proof within its horizon and jump bound, `unsettled` for an
    /// inconclusive result, `unavailable` for a stopped solver.
    pub fn settlement_basis(&self, claim: &CheckedHybridClaim) -> SettlementBasis {
        self.basis(&ClaimHorizon::of(claim))
    }

    fn basis(&self, claim: &ClaimHorizon) -> SettlementBasis {
        match &self.outcome {
            SolverOutcome::Stopped(_) => SettlementBasis::Unavailable,
            outcome
                if self.method.is_sound_for(outcome, &self.coverage)
                    && self.coverage.covers(claim) =>
            {
                SettlementBasis::ClosedScope
            }
            _ => SettlementBasis::Unsettled,
        }
    }
}

/// FR-250: a hybrid item (its FR-331 `requirements` member holds
/// `hybrid-dynamics`) whose negotiated candidate set (FR-075, with
/// `candidate_count` backends) is empty settles `unsupported`,
/// `unsupported-requested-capability`. `None` for an item without the
/// requirement, or when a candidate exists and the item is the provider's to
/// settle. Negotiation itself is the caller's (FR-057); this is the map from
/// its empty set to the item's terminal value.
pub fn settle_hybrid_candidates(
    requirements: &[&str],
    candidate_count: usize,
) -> Option<TerminalValue> {
    (requirements.contains(&"hybrid-dynamics") && candidate_count == 0).then_some(
        TerminalValue::Unsupported(UnavailabilityCause::HybridProviderAbsent),
    )
}

/// What settling a provider's result produced: the terminal value and, when
/// FR-450's exact replay reproduced the reported trace, the item's
/// counterexample (one `counterexamples` entry).
#[derive(Clone, Debug, PartialEq)]
pub struct HybridSettlement {
    /// The terminal value.
    pub value: TerminalValue,
    /// The reproduced trace; `Some` exactly when `value` is `Refuted`.
    pub counterexample: Option<HybridCounterexample>,
}

impl HybridSettlement {
    fn without_counterexample(value: TerminalValue) -> Self {
        Self {
            value,
            counterexample: None,
        }
    }

    /// FR-450: exact-replay a process provider's `refuted` hybrid counterexample.
    /// The provider payload is retained for a replay-parity disagreement.
    pub fn from_provider_counterexample(
        graph: &CheckedGraph,
        claim: &CheckedHybridClaim,
        counterexample: HybridCounterexample,
        meter: &mut HybridReplayMeter,
    ) -> Result<Self, HybridReplayRefusal> {
        if !matches!(
            counterexample.kind,
            crate::hybrid_replay::HybridCounterexampleKind::Formula
        ) {
            return Err(HybridReplayRefusal::UnsupportedCounterexampleKind);
        }
        let exhausted = |meter: &HybridReplayMeter| {
            Self::without_counterexample(TerminalValue::Incomplete(IncompleteCause::LimitReached {
                limit: ModelCheckLimit::EvaluationMeter {
                    counter: LimitKind::WorkUnits,
                },
                value: meter.work_units,
            }))
        };
        match exact_replay(graph, claim, &counterexample.trace, meter) {
            Ok(ExactReplay::Reproduced(replayed)) if replayed.kind == counterexample.kind => {
                Ok(Self {
                    value: TerminalValue::Refuted,
                    counterexample: Some(counterexample),
                })
            }
            Ok(ExactReplay::Reproduced(_)) | Ok(ExactReplay::NotReproduced { .. }) => {
                Ok(Self::without_counterexample(TerminalValue::Inconclusive(
                    InconclusiveCause::ReplayParity(DisagreementCause::HybridTrace(
                        HybridTraceDisagreement::NotReproduced {
                            counterexample: Box::new(counterexample),
                        },
                    )),
                )))
            }
            Ok(ExactReplay::NotExact) => Ok(Self::without_counterexample(
                TerminalValue::Inconclusive(InconclusiveCause::ReplayParity(
                    DisagreementCause::HybridTrace(HybridTraceDisagreement::NotExact {
                        counterexample: Box::new(counterexample),
                    }),
                )),
            )),
            Err(HybridReplayRefusal::ResourceExhausted { .. }) => Ok(exhausted(meter)),
            Err(refusal) => Err(refusal),
        }
    }
}

/// A request's hybrid solver budget (FR-250, QSpec FR-331 `hybrid.limits`).
///
/// `max_solver_time` is a whole number of seconds: a request value is
/// rounded up, so 1.5 s is 2 s and 0.2 s is 1 s. Each member is positive. A
/// member the request does not set is omitted, and the provider applies the
/// bound its descriptor publishes ([`SolverBudget::resolved`]).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SolverBudget {
    time_seconds: Option<u64>,
    steps: Option<u64>,
}

/// Why a hybrid budget is refused before any solver consumes it
/// (`invalid_runtime_input`/`invalid-value`).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BudgetRefusal {
    /// A budget member that is zero or negative.
    NotPositive(&'static str),
    /// A wire member that is not a whole number.
    NotAWholeNumber(&'static str),
    /// A whole count above the `PositiveCount` maximum, 2^53 - 1.
    OutOfDomain(&'static str),
    /// A member of `hybrid` or `hybrid.limits` the contract does not define.
    UnknownMember(String),
    /// `hybrid`, or its `limits`, is empty or not an object.
    EmptyOrMalformed(&'static str),
}

/// The largest count a budget member holds (FR-331 `PositiveCount`).
const MAX_COUNT: u64 = (1 << 53) - 1;
const TIME_MEMBER: &str = "max_solver_time";
const STEPS_MEMBER: &str = "max_solver_steps";

impl SolverBudget {
    /// The budget a request sets: `time` in seconds (rounded up to whole
    /// seconds) and `steps`; `None` leaves a member unset.
    pub fn from_request(
        time: Option<&Rational>,
        steps: Option<u64>,
    ) -> Result<Self, BudgetRefusal> {
        let time_seconds = time.map(Self::round_up_seconds).transpose()?;
        match steps {
            Some(0) => return Err(BudgetRefusal::NotPositive(STEPS_MEMBER)),
            Some(count) if count > MAX_COUNT => {
                return Err(BudgetRefusal::OutOfDomain(STEPS_MEMBER))
            }
            _ => {}
        }
        Ok(Self {
            time_seconds,
            steps,
        })
    }

    fn round_up_seconds(time: &Rational) -> Result<u64, BudgetRefusal> {
        if time.numerator().is_negative() || time.is_zero() {
            return Err(BudgetRefusal::NotPositive(TIME_MEMBER));
        }
        let divisor = NonZeroInteger::try_from(time.denominator())
            .map_err(|_| BudgetRefusal::NotAWholeNumber(TIME_MEMBER))?;
        let (floor, remainder) = time.numerator().div_mod_floor(divisor);
        let ceiling = if remainder.is_zero() {
            floor
        } else {
            floor.add(&Integer::one())
        };
        ceiling
            .to_u64()
            .filter(|seconds| *seconds <= MAX_COUNT)
            .ok_or(BudgetRefusal::OutOfDomain(TIME_MEMBER))
    }

    /// The whole-second time budget the request sets.
    pub fn time_seconds(&self) -> Option<u64> {
        self.time_seconds
    }

    /// The budget a solver runs under: each unset member takes the bound the
    /// provider's descriptor publishes, as `(seconds, steps)`.
    pub fn resolved(&self, published_seconds: u64, published_steps: u64) -> (u64, u64) {
        (
            self.time_seconds.unwrap_or(published_seconds),
            self.steps.unwrap_or(published_steps),
        )
    }

    /// The request's `hybrid` member, or `None` when the request sets no
    /// budget (both members are then omitted).
    pub fn hybrid_member(&self) -> Option<serde_json::Value> {
        let mut limits = serde_json::Map::new();
        if let Some(seconds) = self.time_seconds {
            limits.insert(TIME_MEMBER.to_owned(), seconds.into());
        }
        if let Some(steps) = self.steps {
            limits.insert(STEPS_MEMBER.to_owned(), steps.into());
        }
        (!limits.is_empty()).then(|| serde_json::json!({ "limits": limits }))
    }

    /// The bytes the budget contributes to a request's canonical key. The
    /// step budget is part of the request identity and the wall-clock budget
    /// is not, so two requests that differ only in `max_solver_time` have
    /// equal bytes.
    pub fn key_bytes(&self) -> Vec<u8> {
        let keyed = Self {
            time_seconds: None,
            steps: self.steps,
        };
        keyed
            .hybrid_member()
            .map(|member| member.to_string().into_bytes())
            .unwrap_or_default()
    }

    /// Read a request's `hybrid` member, refusing a malformed one before any
    /// solver consumes it. The member names are the wire's, so this reader
    /// is a string edge.
    #[ix_trace_rs::string_edge]
    pub fn read_hybrid(hybrid: &serde_json::Value) -> Result<Self, BudgetRefusal> {
        let object = hybrid
            .as_object()
            .filter(|object| !object.is_empty())
            .ok_or(BudgetRefusal::EmptyOrMalformed("hybrid"))?;
        if let Some(unknown) = object.keys().find(|key| key.as_str() != "limits") {
            return Err(BudgetRefusal::UnknownMember(unknown.clone()));
        }
        let limits = object
            .get("limits")
            .and_then(serde_json::Value::as_object)
            .filter(|limits| !limits.is_empty())
            .ok_or(BudgetRefusal::EmptyOrMalformed("hybrid.limits"))?;
        if let Some(unknown) = limits
            .keys()
            .find(|key| ![TIME_MEMBER, STEPS_MEMBER].contains(&key.as_str()))
        {
            return Err(BudgetRefusal::UnknownMember(unknown.clone()));
        }
        let member = |name: &'static str| -> Result<Option<u64>, BudgetRefusal> {
            limits
                .get(name)
                .map(|value| Self::wire_count(name, value))
                .transpose()
        };
        Ok(Self {
            time_seconds: member(TIME_MEMBER)?,
            steps: member(STEPS_MEMBER)?,
        })
    }

    fn wire_count(name: &'static str, value: &serde_json::Value) -> Result<u64, BudgetRefusal> {
        let number = value
            .as_number()
            .ok_or(BudgetRefusal::NotAWholeNumber(name))?;
        if let Some(count) = number.as_u64() {
            return match count {
                0 => Err(BudgetRefusal::NotPositive(name)),
                count if count > MAX_COUNT => Err(BudgetRefusal::OutOfDomain(name)),
                count => Ok(count),
            };
        }
        if number.as_i64().is_some() {
            return Err(BudgetRefusal::NotPositive(name));
        }
        // A literal past 64 bits reads as a float.
        match number.as_f64() {
            Some(float) if float.is_finite() && float.fract() == 0.0 => {
                if float <= 0.0 {
                    Err(BudgetRefusal::NotPositive(name))
                } else if float > MAX_COUNT as f64 {
                    Err(BudgetRefusal::OutOfDomain(name))
                } else {
                    Err(BudgetRefusal::NotAWholeNumber(name))
                }
            }
            _ => Err(BudgetRefusal::NotAWholeNumber(name)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model_check::limits::ModelCheckLimit;
    use crate::outcome::OutcomeItem;
    use crate::proof_result::TerminalRecord;
    use ix_trace_rs::trace;
    use qsl_foundation::{RequestIndex, SourceIdentity};
    use qsl_semantics::check::{CheckedGraph, CheckedHybridClaim, PackageDeclarations};
    use quire_exact::{CancelCause, Integer};
    use quire_semantic_value::checking::CheckingLimits;
    use serde_json::json;

    fn horizon(seconds: u64) -> SolverHorizon {
        SolverHorizon::new(Rational::from_integer(Integer::from(seconds))).expect("non-negative")
    }

    #[trace("TC-705", "FR-250-AC-7", "FR-250-AC-6")]
    #[test]
    fn bounded_coverage_requires_the_claims_dimension_and_full_extent() {
        let time = SolverCoverage::Bounded {
            horizon: horizon(10),
            jumps: 1,
            exhaustive: BTreeSet::from([BoundDimension::Time]),
        };
        assert!(time.covers(&ClaimHorizon::ModelTime(Some(horizon(10)))));
        assert!(!time.covers(&ClaimHorizon::ModelTime(Some(horizon(11)))));
        assert!(!time.covers(&ClaimHorizon::ModelSteps(Some(1))));
        assert!(!time.covers(&ClaimHorizon::ModelTime(None)));

        let jumps = SolverCoverage::Bounded {
            horizon: horizon(20),
            jumps: 1,
            exhaustive: BTreeSet::from([BoundDimension::Jumps]),
        };
        assert!(jumps.covers(&ClaimHorizon::ModelSteps(Some(1))));
        assert!(!jumps.covers(&ClaimHorizon::ModelSteps(Some(2))));
        assert!(!jumps.covers(&ClaimHorizon::ModelTime(Some(horizon(1)))));

        let zero = SolverCoverage::Bounded {
            horizon: horizon(1),
            jumps: 0,
            exhaustive: BTreeSet::new(),
        };
        assert!(zero.covers(&ClaimHorizon::ModelSteps(Some(0))));
        assert!(!zero.covers(&ClaimHorizon::ModelSteps(Some(1))));
        assert!(!zero.covers(&ClaimHorizon::ModelSteps(None)));
        assert!(SolverCoverage::Inductive.covers(&ClaimHorizon::ModelTime(None)));
        assert!(SolverCoverage::Inductive.covers(&ClaimHorizon::ModelSteps(None)));
    }

    #[trace("TC-705", "FR-250-AC-7", "FR-250-AC-6")]
    #[test]
    fn sound_bounded_result_that_misses_a_claim_is_inconclusive() {
        let result = SolverResult {
            method: SolverMethod::DeltaComplete,
            outcome: SolverOutcome::Unsat,
            coverage: SolverCoverage::Bounded {
                horizon: horizon(1),
                jumps: 1,
                exhaustive: BTreeSet::from([BoundDimension::Time]),
            },
        };
        assert_eq!(
            result.terminal_value(&ClaimHorizon::ModelTime(Some(horizon(10)))),
            TerminalValue::Inconclusive(InconclusiveCause::InsufficientHorizon {
                horizon: horizon(1),
                jumps: 1,
                exhaustive: BTreeSet::from([BoundDimension::Time]),
            })
        );
        assert_eq!(
            result.basis(&ClaimHorizon::ModelTime(Some(horizon(10)))),
            SettlementBasis::Unsettled
        );
    }

    #[trace("TC-705", "FR-250-AC-7", "FR-250-AC-6")]
    #[test]
    fn inductive_enclosure_proves_unbounded_claims_only_with_inductive_coverage() {
        let result = SolverResult {
            method: SolverMethod::InductiveEnclosure,
            outcome: SolverOutcome::EnclosureDisjoint,
            coverage: SolverCoverage::Inductive,
        };
        assert_eq!(
            result.terminal_value(&ClaimHorizon::ModelTime(None)),
            TerminalValue::Proved {
                basis: ProofBasis::InductiveSolver {
                    method: SolverMethod::InductiveEnclosure,
                },
                certification: Certification::Uncertified,
            }
        );
        assert_eq!(
            result.basis(&ClaimHorizon::ModelTime(None)),
            SettlementBasis::ClosedScope
        );
    }

    #[trace("TC-705", "FR-250-AC-6")]
    #[test]
    fn an_empty_initial_set_never_proves_a_claim() {
        let result = SolverResult {
            method: SolverMethod::DeltaComplete,
            outcome: SolverOutcome::NoInitialState,
            coverage: SolverCoverage::Inductive,
        };
        assert_eq!(
            result.terminal_value(&ClaimHorizon::ModelTime(None)),
            TerminalValue::Inconclusive(InconclusiveCause::NoInitialState)
        );
        assert_eq!(
            result.basis(&ClaimHorizon::ModelTime(None)),
            SettlementBasis::Unsettled
        );
    }

    #[trace("TC-705", "FR-250-AC-6")]
    #[test]
    fn only_the_three_classified_method_outcome_coverage_triples_are_sound() {
        let bounded = SolverCoverage::Bounded {
            horizon: horizon(10),
            jumps: 1,
            exhaustive: BTreeSet::from([BoundDimension::Time]),
        };
        let inductive = SolverCoverage::Inductive;
        let methods = [
            SolverMethod::DeltaComplete,
            SolverMethod::FlowpipeOutwardRounded,
            SolverMethod::FlowpipeUnrounded,
            SolverMethod::InductiveEnclosure,
            SolverMethod::Unclassified,
        ];
        let outcomes = [SolverOutcome::Unsat, SolverOutcome::EnclosureDisjoint];
        for method in methods {
            for outcome in outcomes.clone() {
                for coverage in [&bounded, &inductive] {
                    let expected = matches!(
                        (method, outcome.clone(), coverage),
                        (
                            SolverMethod::DeltaComplete,
                            SolverOutcome::Unsat,
                            SolverCoverage::Bounded { .. }
                        ) | (
                            SolverMethod::FlowpipeOutwardRounded,
                            SolverOutcome::EnclosureDisjoint,
                            SolverCoverage::Bounded { .. }
                        ) | (
                            SolverMethod::InductiveEnclosure,
                            SolverOutcome::EnclosureDisjoint,
                            SolverCoverage::Inductive
                        )
                    );
                    assert_eq!(
                        method.is_sound_for(&outcome, coverage),
                        expected,
                        "{method:?} {outcome:?} {coverage:?}"
                    );
                }
            }
        }
    }

    fn result(method: SolverMethod, outcome: SolverOutcome) -> SolverResult {
        SolverResult {
            method,
            outcome,
            coverage: SolverCoverage::Bounded {
                horizon: horizon(10),
                jumps: 3,
                exhaustive: BTreeSet::from([BoundDimension::Time]),
            },
        }
    }

    fn claim() -> ClaimHorizon {
        ClaimHorizon::ModelTime(Some(horizon(10)))
    }

    fn item(result: &SolverResult) -> serde_json::Value {
        let record =
            TerminalRecord::from_value(RequestIndex::new(0), result.terminal_value(&claim()));
        serde_json::to_value(OutcomeItem::from_terminal(&record)).expect("serializes")
    }

    /// FR-447's `Braking` and `SlowEnough`, with the declared initial set
    /// empty because the `Moving` condition is outside its invariant.
    fn no_initial_state_fixture() -> (CheckedGraph, CheckedHybridClaim) {
        const SOURCE: &str = "language \"ix:native\" edition \"1-draft\";\n\
             profile p = \"quire.temporal.infinite-trace/v1\";\n\
             dimension Time;\n\
             unit s : Time = rational(1, 1);\n\
             hybrid Braking using p {\n\
               initial { mode Moving when { v = 30 }; }\n\
               mode Moving invariant { v >= 0 and v <= 20 and a >= 1 and a <= 2 }\n\
                 flow { v' = -a; a' = 0; t' = 1; }\n\
                 transition to Stopped when { v = 0 } reset { v = 0 };\n\
               mode Stopped invariant { v = 0 }\n\
                 flow { v' = 0; a' = 0; t' = 1; }\n\
             }\n\
             temporal SlowEnough using p over (h: Braking) clock \"model-time\"\
               on origin { always [0, 10] holds(h.v < 30) }\n";
        let parsed = qsl_cst::parse(
            SourceIdentity::new("agent-ix", "qsl-1035", "fixture", "fixture:1"),
            "program.native",
            SOURCE.as_bytes(),
            qsl_cst::Limits::default(),
        )
        .expect("S1 reads the unit");
        assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
        let unit = qsl_forms::build_unit(&parsed).expect("S2 builds the unit");
        let graph = PackageDeclarations::assemble(
            parsed.source().reference().clone(),
            unit,
            qsl_semantics::model::intake::SelectedModels::default(),
            Vec::new(),
        )
        .expect("the unit assembles")
        .check(CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("the unit checks: {refusals:?}"));
        let claim = match graph.hybrid_claims() {
            [claim] => claim.clone(),
            _ => panic!("one checked hybrid claim"),
        };
        (graph, claim)
    }

    fn process_item(package: &[u8], budget: SolverBudget) -> HybridProcessItem {
        let wire: serde_json::Value = serde_json::from_slice(package).expect("package JSON");
        let nodes = wire["semantic_graph"]["nodes"]
            .as_array()
            .expect("semantic graph nodes");
        let claim = nodes
            .iter()
            .find(|node| {
                node["semantic_form"] == "temporal_clause"
                    && node["occurrences"].as_array().is_some_and(|occurrences| {
                        occurrences
                            .iter()
                            .any(|occurrence| occurrence["role"] == "claim")
                    })
            })
            .expect("emitted hybrid claim");
        let occurrence = claim["occurrences"]
            .as_array()
            .expect("claim occurrences")
            .iter()
            .find(|occurrence| occurrence["role"] == "claim")
            .expect("claim occurrence");
        HybridProcessItem::new(
            RequestIndex::new(0),
            json!({"node": claim["node_id"], "occurrence": occurrence}),
            json!({"classification": "bounded", "bounds": []}),
            json!({"type": "set", "backends": ["hybrid-fixture"]}),
            budget,
        )
    }

    fn process_package_evidence() -> quire_contract_model::CheckedPackageEvidence {
        let mut evidence = quire_contract_model::CheckedPackageEvidence::default();
        evidence.support_feature("quire.value.complete/v1");
        evidence
    }

    /// QSpec FR-193's verdict map, FR-250's table: a sound `Unsat` settles
    /// `proved`, `uncertified`, with its horizon and jump bound.
    #[trace("TC-705", "FR-250-AC-2")]
    #[test]
    fn a_sound_unsat_settles_proved_uncertified_with_horizon_and_jumps() {
        let sound = result(SolverMethod::DeltaComplete, SolverOutcome::Unsat);
        assert_eq!(
            sound.terminal_value(&claim()),
            TerminalValue::Proved {
                basis: ProofBasis::BoundedSolver {
                    method: SolverMethod::DeltaComplete,
                    horizon: horizon(10),
                    jumps: 3,
                    exhaustive: BTreeSet::from([BoundDimension::Time]),
                },
                certification: Certification::Uncertified,
            }
        );
        assert_eq!(
            item(&sound),
            json!({
                "request_index": 0,
                "result": "proved",
                "cause": null,
                "basis": {
                    "type": "bounded-solver",
                    "method": "delta-complete",
                    "horizon": {"numerator": "10", "denominator": "1"},
                    "jumps": 3,
                    "exhaustive": ["time"],
                },
                "certification": "uncertified",
                "category": "success",
            })
        );
        assert_eq!(sound.basis(&claim()), SettlementBasis::ClosedScope);
        let enclosure = result(
            SolverMethod::FlowpipeOutwardRounded,
            SolverOutcome::EnclosureDisjoint,
        );
        assert!(matches!(
            enclosure.terminal_value(&claim()),
            TerminalValue::Proved {
                certification: Certification::Uncertified,
                ..
            }
        ));
    }

    /// Every other result is `inconclusive`, `solver-inconclusive`.
    #[trace("TC-705", "FR-250-AC-2")]
    #[test]
    fn an_unsound_or_undecided_result_settles_inconclusive() {
        let unsound = [
            result(SolverMethod::DeltaComplete, SolverOutcome::DeltaSat(None)),
            result(
                SolverMethod::FlowpipeUnrounded,
                SolverOutcome::EnclosureDisjoint,
            ),
            result(
                SolverMethod::FlowpipeOutwardRounded,
                SolverOutcome::EnclosureMeetsUnsafe(None),
            ),
            result(SolverMethod::DeltaComplete, SolverOutcome::Unknown(None)),
            result(SolverMethod::FlowpipeOutwardRounded, SolverOutcome::Unsat),
        ];
        for result in &unsound {
            assert_eq!(result.basis(&claim()), SettlementBasis::Unsettled);
            assert_eq!(
                result.terminal_value(&claim()),
                TerminalValue::Inconclusive(InconclusiveCause::SolverInconclusive {
                    trace: None,
                    replay: None
                }),
                "{result:?}"
            );
        }
        assert_eq!(
            item(&unsound[0]),
            json!({
                "request_index": 0,
                "result": "inconclusive",
                "cause": {"kind": "solver-inconclusive"},
                "category": "inconclusive",
            })
        );
    }

    /// A solver stopped by its time budget settles `incomplete`,
    /// `timed-out`, carrying no seconds; one stopped by its step budget
    /// settles `incomplete`, `limit-reached` naming `solver-steps`.
    #[trace("TC-705", "FR-250-AC-2", "FR-250-AC-3")]
    #[test]
    fn a_stopped_solver_settles_incomplete_by_the_budget_that_stopped_it() {
        let timed = result(
            SolverMethod::DeltaComplete,
            SolverOutcome::Stopped(SolverStop::Time),
        );
        assert_eq!(timed.basis(&claim()), SettlementBasis::Unavailable);
        assert_eq!(
            item(&timed),
            json!({
                "request_index": 0,
                "result": "incomplete",
                "cause": {"kind": "timed-out"},
                "category": "incomplete",
            })
        );
        let frontend_deadline = TerminalRecord::from_value(
            RequestIndex::new(0),
            TerminalValue::Incomplete(IncompleteCause::Cancelled {
                source: CancelCause::Deadline,
            }),
        );
        let frontend_deadline =
            serde_json::to_value(OutcomeItem::from_terminal(&frontend_deadline))
                .expect("the frontend deadline record serializes");
        assert_eq!(frontend_deadline["cause"]["kind"], "cancelled");
        assert_eq!(frontend_deadline["cause"]["source"], "deadline");
        assert_ne!(item(&timed)["cause"], frontend_deadline["cause"]);
        let stepped = result(
            SolverMethod::DeltaComplete,
            SolverOutcome::Stopped(SolverStop::Steps(1000)),
        );
        assert_eq!(stepped.basis(&claim()), SettlementBasis::Unavailable);
        assert_eq!(
            item(&stepped),
            json!({
                "request_index": 0,
                "result": "incomplete",
                "cause": {
                    "kind": "limit-reached",
                    "limit": "solver-steps",
                    "value": 1000,
                    "setting": "hybrid.max_solver_steps",
                },
                "category": "incomplete",
            })
        );
    }

    /// FR-250's table over every method and outcome, written out cell by
    /// cell: only a delta-complete `Unsat` and an outward-rounded
    /// `EnclosureDisjoint` prove, a stop is incomplete (a time stop `timed-out`, a step stop
    /// `limit-reached`), the rest are
    /// inconclusive, and none is `refuted` or `certified`.
    #[trace("TC-705", "FR-250-AC-2")]
    #[test]
    fn every_method_and_outcome_settles_as_the_table_states() {
        use SolverMethod::{
            DeltaComplete as D, FlowpipeOutwardRounded as O, FlowpipeUnrounded as U,
        };
        use SolverOutcome::{EnclosureDisjoint, Unsat};
        let delta_sat = || SolverOutcome::DeltaSat(None);
        let meets_unsafe = || SolverOutcome::EnclosureMeetsUnsafe(None);
        let unknown = || SolverOutcome::Unknown(None);
        #[derive(Clone, Copy)]
        enum Expect {
            Proved,
            Inconclusive,
            TimedOut,
            StepsReached,
        }
        use Expect::{Inconclusive, Proved, StepsReached, TimedOut};
        let time = SolverOutcome::Stopped(SolverStop::Time);
        let steps = SolverOutcome::Stopped(SolverStop::Steps(1));
        let table = [
            (D, Unsat, Proved),
            (D, delta_sat(), Inconclusive),
            (D, EnclosureDisjoint, Inconclusive),
            (D, meets_unsafe(), Inconclusive),
            (D, unknown(), Inconclusive),
            (D, time.clone(), TimedOut),
            (D, steps.clone(), StepsReached),
            (O, Unsat, Inconclusive),
            (O, delta_sat(), Inconclusive),
            (O, EnclosureDisjoint, Proved),
            (O, meets_unsafe(), Inconclusive),
            (O, unknown(), Inconclusive),
            (O, time.clone(), TimedOut),
            (O, steps.clone(), StepsReached),
            (U, Unsat, Inconclusive),
            (U, delta_sat(), Inconclusive),
            (U, EnclosureDisjoint, Inconclusive),
            (U, meets_unsafe(), Inconclusive),
            (U, unknown(), Inconclusive),
            (U, time, TimedOut),
            (U, steps, StepsReached),
        ];
        for (method, outcome, expect) in table {
            let settled = result(method, outcome.clone());
            let (value, basis) = (settled.terminal_value(&claim()), settled.basis(&claim()));
            match expect {
                Proved => {
                    assert_eq!(
                        value,
                        TerminalValue::Proved {
                            basis: ProofBasis::BoundedSolver {
                                method,
                                horizon: horizon(10),
                                jumps: 3,
                                exhaustive: BTreeSet::from([BoundDimension::Time]),
                            },
                            certification: Certification::Uncertified,
                        },
                        "{method:?} {outcome:?}"
                    );
                    assert_eq!(basis, SettlementBasis::ClosedScope);
                }
                Inconclusive => {
                    assert_eq!(
                        value,
                        TerminalValue::Inconclusive(InconclusiveCause::SolverInconclusive {
                            trace: None,
                            replay: None
                        }),
                        "{method:?} {outcome:?}"
                    );
                    assert_eq!(basis, SettlementBasis::Unsettled);
                }
                TimedOut => {
                    assert_eq!(
                        value,
                        TerminalValue::Incomplete(IncompleteCause::TimedOut),
                        "{method:?} {outcome:?}"
                    );
                    assert_eq!(basis, SettlementBasis::Unavailable);
                }
                StepsReached => {
                    assert_eq!(
                        value,
                        TerminalValue::Incomplete(IncompleteCause::LimitReached {
                            limit: ModelCheckLimit::SolverSteps,
                            value: 1,
                        }),
                        "{method:?} {outcome:?}"
                    );
                    assert_eq!(basis, SettlementBasis::Unavailable);
                }
            }
        }
    }

    #[test]
    fn a_horizon_is_exact_non_negative_and_wires_its_components_as_strings() {
        let negative = Rational::from_integer(Integer::from(-1_i64));
        assert_eq!(SolverHorizon::new(negative), Err(NegativeHorizon));
        let large = Integer::from(u64::MAX);
        let half = Rational::new(large, Integer::from(2_i64)).expect("non-zero");
        assert_eq!(
            serde_json::to_value(SolverHorizon::new(half).expect("non-negative")).expect("wire"),
            json!({"numerator": "18446744073709551615", "denominator": "2"})
        );
    }

    fn ratio(numerator: i64, denominator: i64) -> Rational {
        Rational::new(Integer::from(numerator), Integer::from(denominator)).expect("non-zero")
    }

    #[trace("TC-705", "FR-250-AC-3")]
    #[test]
    fn max_solver_time_rounds_up_to_whole_seconds_and_unset_members_take_the_supplied_bounds() {
        let seconds = |time: Rational| {
            SolverBudget::from_request(Some(&time), None)
                .expect("admitted")
                .time_seconds()
        };
        assert_eq!(seconds(ratio(3, 2)), Some(2));
        assert_eq!(seconds(ratio(2, 1)), Some(2));
        assert_eq!(seconds(ratio(1, 5)), Some(1));

        let unset = SolverBudget::from_request(None, None).expect("admitted");
        assert_eq!(unset.resolved(60, 1_000_000), (60, 1_000_000));
        let set = SolverBudget::from_request(Some(&ratio(3, 2)), Some(1000)).expect("admitted");
        assert_eq!(set.resolved(7, 9), (2, 1000));
    }

    #[trace("TC-705", "FR-250-AC-3")]
    #[test]
    fn the_request_writer_writes_whole_seconds_and_omits_unset_members() {
        let rounded = SolverBudget::from_request(Some(&ratio(3, 2)), None).expect("admitted");
        assert_eq!(
            rounded.hybrid_member(),
            Some(json!({"limits": {"max_solver_time": 2}}))
        );
        let both = SolverBudget::from_request(Some(&ratio(1, 5)), Some(1000)).expect("admitted");
        assert_eq!(
            both.hybrid_member(),
            Some(json!({"limits": {"max_solver_time": 1, "max_solver_steps": 1000}}))
        );
        assert_eq!(
            serde_json::to_vec(&both.hybrid_member().expect("the process request member"))
                .expect("serializes the process request member"),
            br#"{"limits":{"max_solver_steps":1000,"max_solver_time":1}}"#
        );
        let unset = SolverBudget::from_request(None, None).expect("admitted");
        assert_eq!(unset.hybrid_member(), None);
        assert_eq!(
            SolverBudget::read_hybrid(&both.hybrid_member().expect("set")),
            Ok(both)
        );
    }

    /// The process request carries the same budget member and the exact
    /// emitted v2 package bytes; reading it admits the package and binds the
    /// item to the checked hybrid claim.
    #[trace("TC-705", "FR-250-AC-3")]
    #[test]
    fn process_request_composes_and_reads_v2_package_bytes_with_hybrid_limits() {
        let (graph, _) = no_initial_state_fixture();
        let package = qsl_package::CheckedPackage::link(graph);
        let emission = qsl_package::emit_checked(&package).expect("the package emits");
        let package_bytes = emission.package().bytes();
        let budget = SolverBudget::from_request(Some(&ratio(3, 2)), Some(1000))
            .expect("the budget is admitted");
        let item = process_item(package_bytes, budget);
        let evidence = process_package_evidence();
        let limits = quire_contract_model::CheckedPackageReadLimits::bounded();
        let request =
            HybridProcessRequest::compose(package_bytes, &evidence, limits, 20_000, &[item])
                .expect("the process request composes");
        let wire: serde_json::Value =
            serde_json::from_slice(request.bytes()).expect("the request is JSON");
        use base64::Engine as _;
        let carried_package = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(wire["package"].as_str().expect("package base64url"))
            .expect("the package decodes");
        assert_eq!(carried_package, package_bytes);
        assert_eq!(
            wire["items"][0]["hybrid"]["limits"],
            json!({"max_solver_time": 2, "max_solver_steps": 1000})
        );
        let admitted = HybridProcessRequest::read(request.bytes(), &evidence, limits)
            .expect("the process request reads against its package");
        assert_eq!(admitted.hybrids().claims.len(), 1);
        assert_eq!(admitted.items()[0].budget(), budget);

        let mut wrong_occurrence = wire.clone();
        wrong_occurrence["items"][0]["claim"]["occurrence"]["role"] = json!("trigger");
        let wrong_occurrence =
            serde_json::to_vec(&wrong_occurrence).expect("wrong occurrence serializes");
        assert!(matches!(
            HybridProcessRequest::read(&wrong_occurrence, &evidence, limits),
            Err(HybridProcessRequestRefusal::ClaimMismatch)
        ));

        let mut malformed = wire.clone();
        malformed["items"][0]["hybrid"]["limits"]["max_solver_time"] = json!(1.5);
        let malformed = serde_json::to_vec(&malformed).expect("malformed request serializes");
        assert!(matches!(
            HybridProcessRequest::read(&malformed, &evidence, limits),
            Err(HybridProcessRequestRefusal::Budget(
                BudgetRefusal::NotAWholeNumber("max_solver_time")
            ))
        ));
    }

    #[trace("TC-705", "FR-250-AC-3")]
    #[test]
    fn a_malformed_budget_is_refused_before_any_solver_consumes_it() {
        let refused = |hybrid: serde_json::Value| SolverBudget::read_hybrid(&hybrid);
        assert_eq!(
            refused(json!({"limits": {"max_solver_time": 1.5}})),
            Err(BudgetRefusal::NotAWholeNumber("max_solver_time"))
        );
        assert_eq!(
            refused(json!({"limits": {"max_solver_time": 0}})),
            Err(BudgetRefusal::NotPositive("max_solver_time"))
        );
        assert_eq!(
            refused(json!({"limits": {"max_solver_steps": 0}})),
            Err(BudgetRefusal::NotPositive("max_solver_steps"))
        );
        assert_eq!(
            refused(json!({"limits": {"max_solver_depth": 3}})),
            Err(BudgetRefusal::UnknownMember("max_solver_depth".to_owned()))
        );
        assert_eq!(
            refused(json!({})),
            Err(BudgetRefusal::EmptyOrMalformed("hybrid"))
        );
        assert_eq!(
            refused(json!({"limits": {}})),
            Err(BudgetRefusal::EmptyOrMalformed("hybrid.limits"))
        );
        let zero = Rational::from_integer(Integer::from(0_i64));
        assert_eq!(
            SolverBudget::from_request(Some(&zero), None),
            Err(BudgetRefusal::NotPositive("max_solver_time"))
        );
        assert_eq!(
            SolverBudget::from_request(None, Some(0)),
            Err(BudgetRefusal::NotPositive("max_solver_steps"))
        );
    }

    #[trace("TC-705", "FR-250-AC-3")]
    #[test]
    fn requests_differing_only_in_max_solver_time_have_equal_key_bytes() {
        let key = |time: i64, steps: Option<u64>| {
            SolverBudget::from_request(Some(&ratio(time, 1)), steps)
                .expect("admitted")
                .key_bytes()
        };
        assert_eq!(key(1, Some(1000)), key(30, Some(1000)));
        assert_eq!(key(1, None), key(30, None));
        assert_ne!(key(1, Some(1000)), key(1, Some(2000)));
        assert!(!String::from_utf8(key(30, Some(1000)))
            .expect("utf-8")
            .contains("max_solver_time"));

        let timed = result(
            SolverMethod::DeltaComplete,
            SolverOutcome::Stopped(SolverStop::Time),
        );
        let record =
            TerminalRecord::from_value(RequestIndex::new(0), timed.terminal_value(&claim()));
        let wire = serde_json::to_value(OutcomeItem::from_terminal(&record))
            .expect("the timed-out record serializes");
        assert_eq!(wire["cause"]["kind"], "timed-out");
        assert!(!wire.to_string().contains("max_solver_time"));
    }

    /// A provider's empty-initial result is inconclusive through both the
    /// in-process result path and the process provider's plugin-result reader.
    #[trace("TC-705", "FR-250-AC-5")]
    #[test]
    fn no_initial_state_is_inconclusive_for_in_process_and_plugin_results() {
        let (graph, claim) = no_initial_state_fixture();
        let fixture = SolverResult {
            method: SolverMethod::DeltaComplete,
            outcome: SolverOutcome::NoInitialState,
            coverage: SolverCoverage::Inductive,
        };
        let expected = TerminalValue::Inconclusive(InconclusiveCause::NoInitialState);
        let settlement = fixture
            .settle(&graph, &claim, &mut HybridReplayMeter::new(u64::MAX))
            .expect("the no-initial-state result requires no replay");
        assert_eq!(settlement.value, expected);
        assert!(settlement.counterexample.is_none());
        let in_process = serde_json::to_value(OutcomeItem::from_terminal(
            &TerminalRecord::from_value(RequestIndex::new(0), settlement.value.clone()),
        ))
        .expect("the in-process result serializes");
        assert_eq!(in_process["result"], "inconclusive");
        assert_eq!(in_process["cause"]["kind"], "no-initial-state");
        assert!(in_process.get("counterexamples").is_none());

        let package = qsl_package::CheckedPackage::link(graph);
        let emission = qsl_package::emit_checked(&package).expect("the v2 provider package emits");
        let package_bytes = emission.package().bytes();
        let unset = SolverBudget::from_request(None, None).expect("unset budget is valid");
        let item = process_item(package_bytes, unset);
        let evidence = process_package_evidence();
        let limits = quire_contract_model::CheckedPackageReadLimits::bounded();
        let request =
            HybridProcessRequest::compose(package_bytes, &evidence, limits, 20_000, &[item])
                .expect("the process request composes");
        let admitted = HybridProcessRequest::read(request.bytes(), &evidence, limits)
            .expect("the process request admits its v2 package");
        let request_wire: serde_json::Value =
            serde_json::from_slice(request.bytes()).expect("the process request is JSON");
        assert!(request_wire["items"][0].get("hybrid").is_none());
        assert_eq!(
            admitted.items()[0].budget().resolved(60, 1_000_000),
            (60, 1_000_000)
        );
        let process = admitted
            .read_no_initial_state_result(&json!({
                "request_index": 0,
                "backend": "hybrid-fixture",
                "kind": "temporal-satisfaction",
                "result": "inconclusive",
                "cause": {"type": "no-initial-state"},
            }))
            .expect("the process result reads against the admitted hybrid claim");
        assert_eq!(
            admitted.read_no_initial_state_result(&json!({
                "request_index": 1,
                "backend": "hybrid-fixture",
                "kind": "temporal-satisfaction",
                "result": "inconclusive",
                "cause": {"type": "no-initial-state"},
            })),
            Err(HybridProcessRequestRefusal::ResultMismatch)
        );
        let process = serde_json::to_value(OutcomeItem::from_terminal(&process))
            .expect("the process result serializes");
        assert_eq!(process["result"], "inconclusive");
        assert_eq!(process["cause"]["kind"], "no-initial-state");
        assert!(process.get("counterexamples").is_none());

        assert_eq!(admitted.hybrids().models.len(), 1);
        assert_eq!(admitted.hybrids().claims.len(), 1);
    }

    #[trace("TC-705")]
    #[test]
    fn budget_members_are_positive_counts_up_to_two_to_the_53_minus_one() {
        let max = (1_u64 << 53) - 1;
        let read = |value: serde_json::Value| {
            SolverBudget::read_hybrid(&json!({"limits": {"max_solver_steps": value}}))
        };
        let admitted = read(json!(max)).expect("admitted");
        assert_eq!(
            admitted.hybrid_member(),
            Some(json!({"limits": {"max_solver_steps": max}}))
        );
        assert_eq!(
            read(json!(max + 1)),
            Err(BudgetRefusal::OutOfDomain("max_solver_steps"))
        );
        assert_eq!(
            read(json!(-3)),
            Err(BudgetRefusal::NotPositive("max_solver_steps"))
        );
        assert_eq!(
            read(serde_json::from_str("100000000000000000000").expect("json")),
            Err(BudgetRefusal::OutOfDomain("max_solver_steps"))
        );
        for float in ["2.0", "1e2"] {
            assert_eq!(
                read(serde_json::from_str(float).expect("json")),
                Err(BudgetRefusal::NotAWholeNumber("max_solver_steps")),
                "{float}"
            );
        }
        assert_eq!(
            read(serde_json::from_str("1e20").expect("json")),
            Err(BudgetRefusal::OutOfDomain("max_solver_steps"))
        );
        assert_eq!(
            read(json!(1.5)),
            Err(BudgetRefusal::NotAWholeNumber("max_solver_steps"))
        );
        assert_eq!(SolverBudget::from_request(None, Some(max)), Ok(admitted));
        assert_eq!(
            SolverBudget::from_request(None, Some(max + 1)),
            Err(BudgetRefusal::OutOfDomain("max_solver_steps"))
        );
        let seconds = |n: u64| Rational::from_integer(Integer::from(n));
        assert!(SolverBudget::from_request(Some(&seconds(max)), None).is_ok());
        assert_eq!(
            SolverBudget::from_request(Some(&seconds(max + 1)), None),
            Err(BudgetRefusal::OutOfDomain("max_solver_time"))
        );
        // 2^53 - 1 plus a fraction rounds up past the maximum.
        let fraction =
            Rational::new(Integer::from(2 * max + 1), Integer::from(2_i64)).expect("non-zero");
        assert_eq!(
            SolverBudget::from_request(Some(&fraction), None),
            Err(BudgetRefusal::OutOfDomain("max_solver_time"))
        );
    }
}
