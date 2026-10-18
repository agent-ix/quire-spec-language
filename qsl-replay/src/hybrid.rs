// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-250 (ADR-026 HY-1 to HY-3): a hybrid solver result settles as an FR-331
//! terminal value. A result settles `proved` only where its method kind makes
//! it sound for its outcome, and that proof is `uncertified`: a solver QSL
//! runs has no core certificate checker (ADR-018 PC-1). Every other result is
//! `inconclusive`, `incomplete` or, with no provider, `unsupported`. No
//! solver result settles `refuted`, and a provider's own claim about its
//! soundness is never read: [`SolverMethod`] has no such field.

use std::num::NonZeroU64;

use serde::{Serialize, Serializer};

use crate::proof_result::{
    Certification, IncompleteCause, InconclusiveCause, ProofBasis, TerminalValue,
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
}

impl SolverMethod {
    /// The method's wire spelling in a `bounded-solver` basis.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::DeltaComplete => "delta-complete",
            Self::FlowpipeOutwardRounded => "flowpipe-outward-rounded",
            Self::FlowpipeUnrounded => "flowpipe-unrounded",
        }
    }

    /// Whether this kind of method makes `outcome` a sound proof.
    pub const fn is_sound_for(self, outcome: &SolverOutcome) -> bool {
        matches!(
            (self, outcome),
            (Self::DeltaComplete, SolverOutcome::Unsat)
                | (
                    Self::FlowpipeOutwardRounded,
                    SolverOutcome::EnclosureDisjoint
                )
        )
    }
}

impl Serialize for SolverMethod {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

/// A solver budget that can stop a run.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SolverBudget {
    /// The solver's time budget.
    Time,
    /// The solver's step budget.
    Steps,
}

impl SolverBudget {
    /// The budget's wire spelling.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Time => "solver-time",
            Self::Steps => "solver-steps",
        }
    }

    /// The request setting that raises this budget.
    pub const fn setting(self) -> &'static str {
        match self {
            Self::Time => "max_solver_time",
            Self::Steps => "max_solver_steps",
        }
    }
}

/// A solver stopped by one of its budgets.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SolverLimit {
    /// The budget that stopped the run.
    pub budget: SolverBudget,
    /// The budget's value for the run.
    pub value: u64,
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
    /// A budget stopped the solver.
    Stopped(SolverLimit),
}

/// A non-negative time horizon, an exact reduced fraction of seconds.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize)]
pub struct SolverHorizon {
    numerator: u64,
    denominator: NonZeroU64,
}

impl SolverHorizon {
    /// `numerator/denominator` seconds, reduced; zero is `0/1`.
    pub fn new(numerator: u64, denominator: NonZeroU64) -> Self {
        let divisor = gcd(numerator, denominator.get());
        // `divisor` is at least 1: `denominator` is non-zero.
        let denominator = NonZeroU64::new(denominator.get() / divisor).unwrap_or(denominator);
        Self {
            numerator: numerator / divisor,
            denominator,
        }
    }

    /// The reduced numerator.
    pub const fn numerator(self) -> u64 {
        self.numerator
    }

    /// The reduced denominator.
    pub const fn denominator(self) -> NonZeroU64 {
        self.denominator
    }
}

fn gcd(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a
}

/// A provider's result (FR-250 Inputs).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct SolverResult {
    /// The kind of method that produced the outcome.
    pub method: SolverMethod,
    /// The outcome.
    pub outcome: SolverOutcome,
    /// The time horizon the result covers.
    pub horizon: SolverHorizon,
    /// The jump bound the result covers.
    pub jumps: u64,
}

impl SolverResult {
    /// FR-250's map. A sound `Unsat` or `EnclosureDisjoint` is `proved`,
    /// `uncertified`, over its horizon and jump bound; `Stopped` is
    /// `incomplete`; everything else is `inconclusive`. Never `refuted`.
    pub fn terminal_value(&self) -> TerminalValue {
        match self.outcome {
            SolverOutcome::Stopped(limit) => {
                TerminalValue::Incomplete(IncompleteCause::LimitReached {
                    limit: limit.budget,
                    value: limit.value,
                })
            }
            outcome if self.method.is_sound_for(&outcome) => TerminalValue::Proved {
                basis: ProofBasis::BoundedSolver {
                    method: self.method,
                    horizon: self.horizon,
                    jumps: self.jumps,
                },
                certification: Certification::Uncertified,
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::outcome::OutcomeItem;
    use crate::proof_result::TerminalRecord;
    use ix_trace_rs::trace;
    use qsl_foundation::RequestIndex;
    use serde_json::json;

    const METHODS: [SolverMethod; 3] = [
        SolverMethod::DeltaComplete,
        SolverMethod::FlowpipeOutwardRounded,
        SolverMethod::FlowpipeUnrounded,
    ];

    fn horizon(seconds: u64) -> SolverHorizon {
        SolverHorizon::new(seconds, NonZeroU64::MIN)
    }

    fn result(method: SolverMethod, outcome: SolverOutcome) -> SolverResult {
        SolverResult {
            method,
            outcome,
            horizon: horizon(10),
            jumps: 3,
        }
    }

    fn item(result: &SolverResult) -> serde_json::Value {
        let record = TerminalRecord::from_value(RequestIndex::new(0), result.terminal_value());
        serde_json::to_value(OutcomeItem::from_terminal(&record)).expect("serializes")
    }

    /// QSpec FR-193's verdict map, FR-250's table: a sound `Unsat` settles
    /// `proved`, `uncertified`, with its horizon and jump bound.
    #[trace("TC-705", "FR-250-AC-2")]
    #[test]
    fn a_sound_unsat_settles_proved_uncertified_with_horizon_and_jumps() {
        let sound = result(SolverMethod::DeltaComplete, SolverOutcome::Unsat);
        assert_eq!(
            sound.terminal_value(),
            TerminalValue::Proved {
                basis: ProofBasis::BoundedSolver {
                    method: SolverMethod::DeltaComplete,
                    horizon: horizon(10),
                    jumps: 3,
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
                    "horizon": {"numerator": 10, "denominator": 1},
                    "jumps": 3,
                },
                "certification": "uncertified",
                "category": "success",
            })
        );
        let enclosure = result(
            SolverMethod::FlowpipeOutwardRounded,
            SolverOutcome::EnclosureDisjoint,
        );
        assert!(matches!(
            enclosure.terminal_value(),
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
        for result in unsound {
            assert_eq!(
                result.terminal_value(),
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

    /// A stopped solver is `incomplete`, `limit-reached` naming the budget,
    /// its value and the setting that raises it.
    #[trace("TC-705", "FR-250-AC-2")]
    #[test]
    fn a_stopped_solver_settles_incomplete_naming_the_budget() {
        let stopped = result(
            SolverMethod::DeltaComplete,
            SolverOutcome::Stopped(SolverLimit {
                budget: SolverBudget::Time,
                value: 30,
            }),
        );
        assert_eq!(
            item(&stopped),
            json!({
                "request_index": 0,
                "result": "incomplete",
                "cause": {
                    "kind": "limit-reached",
                    "limit": "solver-time",
                    "value": 30,
                    "setting": "max_solver_time",
                },
                "category": "incomplete",
            })
        );
    }

    /// No solver result settles `refuted`, and a proof is always
    /// `uncertified`, over every method and outcome.
    #[trace("TC-705", "FR-250-AC-2")]
    #[test]
    fn no_solver_result_settles_refuted_or_certified() {
        let stop = SolverOutcome::Stopped(SolverLimit {
            budget: SolverBudget::Steps,
            value: 1,
        });
        for method in METHODS {
            for outcome in [
                SolverOutcome::Unsat,
                SolverOutcome::DeltaSat,
                SolverOutcome::EnclosureDisjoint,
                SolverOutcome::EnclosureMeetsUnsafe,
                SolverOutcome::Unknown,
                stop,
            ] {
                match result(method, outcome).terminal_value() {
                    TerminalValue::Proved { certification, .. } => {
                        assert_eq!(certification, Certification::Uncertified);
                    }
                    TerminalValue::Inconclusive(_) | TerminalValue::Incomplete(_) => {}
                    other => panic!("{method:?} {outcome:?} settled {other:?}"),
                }
            }
        }
    }

    #[test]
    fn a_horizon_is_reduced() {
        let reduced = SolverHorizon::new(10, NonZeroU64::new(4).expect("non-zero"));
        assert_eq!(reduced.numerator(), 5);
        assert_eq!(reduced.denominator().get(), 2);
        assert_eq!(
            horizon(0),
            SolverHorizon::new(0, NonZeroU64::new(7).expect("non-zero"))
        );
    }
}
