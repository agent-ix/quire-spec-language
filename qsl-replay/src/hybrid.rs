// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-250 (ADR-026 HY-1 to HY-3): a hybrid solver result settles as an FR-331
//! terminal value. A result settles `proved` only where its method kind makes
//! it sound for its outcome, and that proof is `uncertified`: a solver QSL
//! runs has no core certificate checker (ADR-018 PC-1). Every other result is
//! `inconclusive` or `incomplete`. No
//! solver result settles `refuted`, and a provider's own claim about its
//! soundness is never read: [`SolverMethod`] has no such field.

use quire_exact::Rational;
use serde::ser::SerializeStruct;
use serde::{Serialize, Serializer};
use std::collections::BTreeSet;

use crate::model_check::limits::ModelCheckLimit;
use crate::proof_result::{
    Certification, IncompleteCause, InconclusiveCause, ProofBasis, SettlementBasis, TerminalValue,
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

/// A provider's outcome (FR-250 Inputs).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SolverOutcome {
    /// The unsafe set is unreachable.
    Unsat,
    /// A δ-satisfying point exists: a possible violation, not a witness.
    DeltaSat,
    /// A flowpipe enclosure is disjoint from the unsafe set.
    EnclosureDisjoint,
    /// A flowpipe enclosure meets the unsafe set.
    EnclosureMeetsUnsafe,
    /// The solver could not decide.
    Unknown,
    /// The model has no initial state; no behavior establishes a proof.
    NoInitialState,
    /// A budget stopped the solver.
    Stopped(SolverStop),
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
        let mut object = serializer.serialize_struct("SolverHorizon", 3)?;
        object.serialize_field("kind", "rational")?;
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
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct SolverResult {
    /// The kind of method that produced the outcome.
    pub method: SolverMethod,
    /// The outcome.
    pub outcome: SolverOutcome,
    /// The behavior range established by the solver.
    pub coverage: SolverCoverage,
}

impl SolverResult {
    /// FR-250's method and coverage map for a checked claim. A sound proof
    /// settles `proved` only when its coverage includes the claim; a sound
    /// bounded result that falls short names its actual bounds. No provider
    /// assertion of soundness is read.
    pub fn terminal_value(&self, claim: &ClaimHorizon) -> TerminalValue {
        match self.outcome {
            SolverOutcome::Stopped(SolverStop::Time) => {
                TerminalValue::Incomplete(IncompleteCause::TimedOut)
            }
            SolverOutcome::Stopped(SolverStop::Steps(value)) => {
                TerminalValue::Incomplete(IncompleteCause::LimitReached {
                    limit: ModelCheckLimit::SolverSteps,
                    value,
                })
            }
            SolverOutcome::NoInitialState => {
                TerminalValue::Inconclusive(InconclusiveCause::NoInitialState)
            }
            outcome if self.method.is_sound_for(&outcome, &self.coverage) => match &self.coverage {
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
            SolverOutcome::Unsat
            | SolverOutcome::DeltaSat
            | SolverOutcome::EnclosureDisjoint
            | SolverOutcome::EnclosureMeetsUnsafe
            | SolverOutcome::Unknown => {
                TerminalValue::Inconclusive(InconclusiveCause::SolverInconclusive)
            }
        }
    }

    /// The QSpec FR-243 basis the result settles with: `closed-scope` for a
    /// proof within its horizon and jump bound, `unsettled` for an
    /// inconclusive result, `unavailable` for a stopped solver.
    pub fn basis(&self, claim: &ClaimHorizon) -> SettlementBasis {
        match self.outcome {
            SolverOutcome::Stopped(_) => SettlementBasis::Unavailable,
            outcome
                if self.method.is_sound_for(&outcome, &self.coverage)
                    && self.coverage.covers(claim) =>
            {
                SettlementBasis::ClosedScope
            }
            SolverOutcome::Unsat
            | SolverOutcome::DeltaSat
            | SolverOutcome::EnclosureDisjoint
            | SolverOutcome::EnclosureMeetsUnsafe
            | SolverOutcome::Unknown
            | SolverOutcome::NoInitialState => SettlementBasis::Unsettled,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model_check::limits::ModelCheckLimit;
    use crate::outcome::OutcomeItem;
    use crate::proof_result::TerminalRecord;
    use crate::proof_result::UnavailabilityCause;
    use ix_trace_rs::trace;
    use qsl_foundation::RequestIndex;
    use quire_exact::Integer;
    use serde_json::json;

    fn horizon(seconds: u64) -> SolverHorizon {
        SolverHorizon::new(Rational::from_integer(Integer::from(seconds))).expect("non-negative")
    }

    /// Trace: FR-250-AC-6, FR-250-AC-7, TC-705.
    #[trace("TC-705", "FR-250-AC-7")]
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

    /// Trace: FR-250-AC-6, FR-250-AC-7, TC-705.
    #[trace("TC-705", "FR-250-AC-7")]
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

    /// Trace: FR-250-AC-6, FR-250-AC-7, TC-705.
    #[trace("TC-705", "FR-250-AC-7")]
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

    /// Trace: FR-250-AC-6, TC-705.
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

    /// Trace: FR-250-AC-1, TC-705.
    #[trace("TC-705", "FR-250-AC-1")]
    #[test]
    fn absent_hybrid_provider_settles_unsupported_requested_capability() {
        let record = TerminalRecord::from_value(
            RequestIndex::new(0),
            TerminalValue::Unsupported(UnavailabilityCause::HybridProviderAbsent),
        );
        let item = serde_json::to_value(OutcomeItem::from_terminal(&record)).expect("serializes");
        assert_eq!(item["result"], "unsupported");
        assert_eq!(item["category"], "unsupported");
        assert_eq!(item["cause"]["kind"], "unsupported-requested-capability");
        assert_eq!(item["cause"]["code"], "unsupported_projection");
    }

    /// Trace: FR-250-AC-6, TC-705.
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
            for outcome in outcomes {
                for coverage in [&bounded, &inductive] {
                    let expected = matches!(
                        (method, outcome, coverage),
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
                    "horizon": {"kind": "rational", "numerator": "10", "denominator": "1"},
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
            result(SolverMethod::DeltaComplete, SolverOutcome::DeltaSat),
            result(
                SolverMethod::FlowpipeUnrounded,
                SolverOutcome::EnclosureDisjoint,
            ),
            result(
                SolverMethod::FlowpipeOutwardRounded,
                SolverOutcome::EnclosureMeetsUnsafe,
            ),
            result(SolverMethod::DeltaComplete, SolverOutcome::Unknown),
            result(SolverMethod::FlowpipeOutwardRounded, SolverOutcome::Unsat),
        ];
        for result in &unsound {
            assert_eq!(result.basis(&claim()), SettlementBasis::Unsettled);
            assert_eq!(
                result.terminal_value(&claim()),
                TerminalValue::Inconclusive(InconclusiveCause::SolverInconclusive),
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
    #[trace("TC-705", "FR-250-AC-2")]
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
                    "setting": "max_solver_steps",
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
        use SolverOutcome::{DeltaSat, EnclosureDisjoint, EnclosureMeetsUnsafe};
        use SolverOutcome::{Unknown, Unsat};
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
            (D, DeltaSat, Inconclusive),
            (D, EnclosureDisjoint, Inconclusive),
            (D, EnclosureMeetsUnsafe, Inconclusive),
            (D, Unknown, Inconclusive),
            (D, time, TimedOut),
            (D, steps, StepsReached),
            (O, Unsat, Inconclusive),
            (O, DeltaSat, Inconclusive),
            (O, EnclosureDisjoint, Proved),
            (O, EnclosureMeetsUnsafe, Inconclusive),
            (O, Unknown, Inconclusive),
            (O, time, TimedOut),
            (O, steps, StepsReached),
            (U, Unsat, Inconclusive),
            (U, DeltaSat, Inconclusive),
            (U, EnclosureDisjoint, Inconclusive),
            (U, EnclosureMeetsUnsafe, Inconclusive),
            (U, Unknown, Inconclusive),
            (U, time, TimedOut),
            (U, steps, StepsReached),
        ];
        for (method, outcome, expect) in table {
            let settled = result(method, outcome);
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
                        TerminalValue::Inconclusive(InconclusiveCause::SolverInconclusive),
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
            json!({"kind": "rational", "numerator": "18446744073709551615", "denominator": "2"})
        );
    }
}
