// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-450 (ADR-026 HY-2): a `HybridTrace` replayed against the claim's
//! checked hybrid model (FR-447), by QSpec FR-193 "Trace replay".
//!
//! [`static_replay`] checks every trace in exact rational interval
//! arithmetic within the trace's tolerance and settles no value.
//! [`exact_replay`] decides a point trace through modes with polynomial
//! flows exactly, with no floating-point step: each dwell's solution is a
//! vector of polynomials in time with exact rational coefficients, every atom
//! along a dwell a rational function of time whose sign QSL decides by exact
//! real root isolation, and the claim is read over the replayed behaviour by
//! QSpec FR-193 "Hybrid claims". A trace it reproduces is the item's one
//! refutation path (ADR-026 HY-2).
//!
//! Both read the checked package only: the claim's `CheckedHybridClaim`, its
//! `CheckedHybridModel` and the checked bodies of the functions its terms
//! call, by the indices the trace carries. Neither reads source.

mod eval;
mod exact;
mod expand;
mod flow;
mod interval;
mod poly;
mod real;
mod signal;
mod static_replay;
mod trace;

#[cfg(test)]
mod tests;

use qsl_semantics::check::{
    CheckedFunctionRef, CheckedGraph, CheckedHybridClaim, CheckedHybridModel, HybridModelIndex,
};
use quire_exact::LimitKind;

pub use expand::{ExpansionGap, TermLocus};
pub use qsl_foundation::diagnostic::JsonPointer;
pub use real::{ExactReal, Root};
pub use trace::{
    admit, ExactInterval, HybridTrace, TraceBefore, TraceFault, TraceJump, TraceRefusal, TraceState,
};

/// The outcome of static replay (FR-450 "Outputs").
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StaticReplay {
    /// No check is definitely false.
    Consistent,
    /// The first check, in trace order, that is definitely false.
    Inconsistent {
        /// The RFC 6901 pointer of the member the check reads.
        at: JsonPointer,
        /// Which check.
        check: StaticCheck,
    },
}

/// QSpec FR-193's static checks 1 to 5.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum StaticCheck {
    /// 1: `start` meets an initial entry of its mode.
    Initial,
    /// 2: a state meets its mode's invariant.
    Invariant,
    /// 3: no time runs backwards and a jump takes no time.
    Time,
    /// 4: the source invariant, the guard and the reset across a jump.
    Jump,
    /// 5: a lasso's last state meets the state it returns to.
    Loop,
}

impl StaticCheck {
    /// The check's number on the stored record's `replay` member.
    pub const fn number(self) -> u8 {
        match self {
            Self::Initial => 1,
            Self::Invariant => 2,
            Self::Time => 3,
            Self::Jump => 4,
            Self::Loop => 5,
        }
    }
}

/// The replay outcomes kept with a trace on an `inconclusive` record: QSpec
/// FR-193's `replay` member, `static` and, when exact replay ran and did not
/// reproduce, `exact`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReplayRecord {
    /// The static replay outcome.
    pub static_outcome: StaticReplay,
    /// The exact replay outcome, when exact replay ran and did not
    /// reproduce the trace.
    pub exact: Option<ExactRecord>,
}

/// An exact replay outcome that settles no value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExactRecord {
    /// The trace is not eligible for exact replay.
    NotExact,
    /// The first exact check that fails.
    NotReproduced {
        /// The RFC 6901 pointer of the member the check reads.
        at: JsonPointer,
        /// Which check.
        check: ExactCheck,
    },
}

/// The outcome of exact replay (FR-450 "Outputs").
#[derive(Clone, Debug, PartialEq)]
pub enum ExactReplay {
    /// Not a point trace, a mode the trace dwells in without a polynomial
    /// flow, or an expression the replay evaluates that calls a recursive
    /// function.
    NotExact,
    /// The trace reproduces a violation.
    Reproduced(Box<HybridCounterexample>),
    /// The first exact check, in trace order, that fails.
    NotReproduced {
        /// The RFC 6901 pointer of the member the check reads; the empty
        /// pointer, the whole trace, for check 5.
        at: JsonPointer,
        /// Which check.
        check: ExactCheck,
    },
}

/// QSpec FR-193's exact checks 1 to 5.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ExactCheck {
    /// 1: `start` satisfies an initial entry.
    Initial,
    /// 2: a dwell's solution reaches the jump's `before` and keeps the
    /// mode's invariant.
    Dwell,
    /// 3: the guard, the jump's time, the reset and the target invariant.
    Jump,
    /// 4: a lasso returns to the state it names after a positive dwell.
    Loop,
    /// 5: the claim is false on the replayed behaviour.
    Claim,
}

impl ExactCheck {
    /// The check's number on the stored record's `replay` member.
    pub const fn number(self) -> u8 {
        match self {
            Self::Initial => 1,
            Self::Dwell => 2,
            Self::Jump => 3,
            Self::Loop => 4,
            Self::Claim => 5,
        }
    }
}

/// A reproduced hybrid trace (QSpec FR-331 `hybrid-counterexample`).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HybridCounterexample {
    /// What the trace reproduces.
    pub kind: HybridCounterexampleKind,
    /// The trace.
    pub trace: HybridTrace,
}

impl HybridCounterexample {
    pub(crate) fn measured_bytes(&self) -> usize {
        fn rational(value: &quire_exact::Rational) -> usize {
            value.numerator().to_string().len() + value.denominator().to_string().len()
        }
        fn interval(value: &ExactInterval) -> usize {
            rational(&value.lower) + rational(&value.upper)
        }
        fn state(value: &TraceState) -> usize {
            interval(&value.time) + value.enclosure.iter().map(interval).sum::<usize>() + 8
        }
        let trace = &self.trace;
        state(&trace.start)
            + trace
                .jumps
                .iter()
                .map(|jump| {
                    8 + jump.before.as_ref().map_or(0, |before| {
                        interval(&before.time)
                            + before.enclosure.iter().map(interval).sum::<usize>()
                    }) + state(&jump.state)
                })
                .sum::<usize>()
            + trace.delta.as_ref().map_or(0, rational)
            + trace.loop_to.map_or(0, |_| 8)
    }
}

/// What a reproduced trace shows.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HybridCounterexampleKind {
    /// The claim is false on the replayed behaviour.
    Formula,
    /// An expression the replay evaluates has no value at the decisive
    /// instant.
    UndefinedEvaluation {
        /// The instant and the expression: QSpec FR-193's `where`.
        at: Box<UndefinedInstant>,
        /// Why it has no value.
        cause: UndefinedCause,
    },
}

/// The instant of a behaviour at which an expression has no value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UndefinedInstant {
    /// The trace state whose dwell, or whose own point, holds the instant:
    /// 0 for `start`, `k` for the state of `jumps[k - 1]`.
    pub state: usize,
    /// The instant's time stamp, in seconds. When the expression has no
    /// value on an open stretch of instants only, this is the stretch's
    /// start, the infimum of those instants.
    pub time: ExactReal,
    /// The expression.
    pub locus: ExpressionLocus,
}

/// An expression of the model or the claim, and a place inside it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExpressionLocus {
    /// Which expression.
    pub site: ExpressionSite,
    /// Where in it.
    pub term: TermLocus,
}

/// One expression of a checked hybrid model or claim.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ExpressionSite {
    /// The condition of `initial[entry]`.
    Initial {
        /// The entry's position.
        entry: usize,
    },
    /// A mode's invariant.
    Invariant {
        /// The mode's position.
        mode: usize,
    },
    /// A mode's rate of a variable.
    Rate {
        /// The mode's position.
        mode: usize,
        /// The variable's position.
        variable: usize,
    },
    /// A transition's guard.
    Guard {
        /// The source mode's position.
        mode: usize,
        /// The transition's position in the mode.
        transition: usize,
    },
    /// A transition's reset constraint.
    Reset {
        /// The source mode's position.
        mode: usize,
        /// The transition's position in the mode.
        transition: usize,
        /// The constraint's position in the reset.
        constraint: usize,
    },
    /// The claim's `on each ... when` guard.
    Activation,
    /// A capture's initializer.
    Capture {
        /// The capture's position.
        capture: usize,
    },
    /// A `holds(...)` atom of the claim's formula, by its position in the
    /// formula's pre-order.
    ClaimAtom {
        /// The atom's position.
        atom: usize,
    },
}

/// Why an expression has no value (QSpec FR-193 "Continuous values and
/// expressions").
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum UndefinedCause {
    /// A division whose divisor is 0.
    DivisionByZero,
    /// A call's argument outside its parameter's value range.
    OutsideParameterRange {
        /// The called function.
        function: CheckedFunctionRef,
        /// The parameter's position.
        parameter: usize,
    },
    /// A call's result outside its result type's value range.
    OutsideResultRange {
        /// The called function.
        function: CheckedFunctionRef,
    },
    /// A capture's value outside its type's value range.
    OutsideCaptureRange {
        /// The capture's position.
        capture: usize,
    },
}

/// A claim form exact replay does not decide.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ExactGap {
    /// An `on each` claim under `model-time` whose body or captures read the
    /// activation position's state: a continuum of obligations, each with
    /// its own captured values.
    #[error("an `on each` claim under model-time reads its activation position's state")]
    ActivationState,
    /// A fairness constraint whose enabledness depends on a reset that does
    /// not determine the state after the jump.
    #[error("fairness reads the enabledness of a transition with a nondeterministic reset")]
    NondeterministicReset {
        /// The source mode's position.
        mode: usize,
        /// The transition's position in the mode.
        transition: usize,
    },
    /// A recursive call reached exact evaluation.
    #[error("a recursive call reached exact evaluation")]
    RecursiveCall,
    /// A Boolean where a number is read, or a number where a Boolean is.
    #[error("a term of the wrong kind reached exact evaluation")]
    IllKindedTerm,
}

/// Why a reader cannot replay a trace.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum HybridReplayRefusal {
    /// The trace breaks QSpec FR-193 "Hybrid trace":
    /// `invalid_result`/`invalid-value` at its member.
    #[error(transparent)]
    Trace(#[from] TraceRefusal),
    /// The claim names no model of the package.
    #[error("the claim names no hybrid model of the package: {0:?}")]
    UnknownModel(HybridModelIndex),
    /// A called function's checked body cannot be expanded.
    #[error(transparent)]
    Expansion(#[from] ExpansionGap),
    /// Exact replay does not decide this claim form.
    #[error(transparent)]
    Exact(#[from] ExactGap),
    /// The caller's accounting work limit denied the next replay step.
    #[error("hybrid replay exhausted the caller's accounting counter {counter:?}")]
    ResourceExhausted {
        /// The accounting counter whose bound was reached.
        counter: LimitKind,
    },
}

/// Caller-supplied work budget shared by static and exact hybrid replay.
/// One admitted step consumes one `accounting.work_units` unit.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HybridReplayMeter {
    /// The caller's configured accounting bound.
    pub work_units: u64,
    /// Successfully charged work units.
    pub consumed: u64,
}

impl HybridReplayMeter {
    /// Create a replay meter from the caller's `accounting.work_units` bound.
    pub const fn new(work_units: u64) -> Self {
        Self {
            work_units,
            consumed: 0,
        }
    }

    /// The number of admitted work units.
    pub const fn consumed(&self) -> u64 {
        self.consumed
    }

    fn charge(&mut self) -> Result<(), HybridReplayRefusal> {
        match self.consumed.checked_add(1) {
            Some(next) if next <= self.work_units => {
                self.consumed = next;
                Ok(())
            }
            _ => Err(HybridReplayRefusal::ResourceExhausted {
                counter: LimitKind::WorkUnits,
            }),
        }
    }
}

fn model_of<'g>(
    graph: &'g CheckedGraph,
    claim: &CheckedHybridClaim,
) -> Result<&'g CheckedHybridModel, HybridReplayRefusal> {
    graph
        .hybrid_models()
        .get(claim.model.get())
        .ok_or(HybridReplayRefusal::UnknownModel(claim.model))
}

/// FR-450 "Static replay": `trace` against `claim`'s model in `graph`, after
/// the QSpec FR-193 "Hybrid trace" refusals ([`admit`]).
pub fn static_replay(
    graph: &CheckedGraph,
    claim: &CheckedHybridClaim,
    trace: &HybridTrace,
    meter: &mut HybridReplayMeter,
) -> Result<StaticReplay, HybridReplayRefusal> {
    let model = model_of(graph, claim)?;
    admit(model, trace)?;
    let mut expander = expand::Expander::new(graph);
    static_replay::replay(model, &mut expander, trace, meter)
}

/// FR-450 "Exact replay": `trace` against `claim` and its model in `graph`,
/// after the QSpec FR-193 "Hybrid trace" refusals ([`admit`]).
pub fn exact_replay(
    graph: &CheckedGraph,
    claim: &CheckedHybridClaim,
    trace: &HybridTrace,
    meter: &mut HybridReplayMeter,
) -> Result<ExactReplay, HybridReplayRefusal> {
    let model = model_of(graph, claim)?;
    admit(model, trace)?;
    let mut expander = expand::Expander::new(graph);
    exact::replay(model, claim, &mut expander, trace, meter)
}
