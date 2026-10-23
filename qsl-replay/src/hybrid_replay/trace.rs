// SPDX-License-Identifier: AGPL-3.0-or-later
//! The `HybridTrace` FR-450 replays, in FR-250's shape (QSpec FR-193
//! "Hybrid trace"), and the refusals a reader applies before replaying it.
//!
//! The types carry the model's mode and variable indices rather than the wire
//! form's names: `enclosure[i]` encloses `variables[i]`, and `transition` is
//! the position among the previous state's mode's transitions.

use qsl_foundation::diagnostic::JsonPointer;
use qsl_semantics::check::{CheckedHybridModel, ModeIndex};
use quire_exact::Rational;

/// An FR-331 `Interval` of exact rationals, `lower <= upper` once the trace
/// is admitted ([`admit`]).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExactInterval {
    /// The lower end, inclusive.
    pub lower: Rational,
    /// The upper end, inclusive.
    pub upper: Rational,
}

impl ExactInterval {
    /// The interval `[lower, upper]`.
    pub fn new(lower: Rational, upper: Rational) -> Self {
        Self { lower, upper }
    }

    /// The one-point interval `[value, value]`.
    pub fn point(value: Rational) -> Self {
        Self {
            lower: value.clone(),
            upper: value,
        }
    }

    /// The point, when `lower` equals `upper`.
    pub fn as_point(&self) -> Option<&Rational> {
        (self.lower == self.upper).then_some(&self.lower)
    }
}

/// One state of a trace: its mode, the time at which it is reached and its
/// enclosure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TraceState {
    /// The mode.
    pub mode: ModeIndex,
    /// Encloses the time, in seconds, at which the state is reached.
    pub time: ExactInterval,
    /// `enclosure[i]` encloses the model's variable `i`.
    pub enclosure: Vec<ExactInterval>,
}

/// The state just before a jump; its mode is the previous state's.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TraceBefore {
    /// Encloses the time of the jump.
    pub time: ExactInterval,
    /// `enclosure[i]` encloses the model's variable `i`.
    pub enclosure: Vec<ExactInterval>,
}

/// One jump of a trace.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TraceJump {
    /// The position of the transition taken among the previous state's
    /// mode's transitions.
    pub transition: u64,
    /// The state just before the jump, when the solver reports it.
    pub before: Option<TraceBefore>,
    /// The state the jump reaches.
    pub state: TraceState,
}

/// A path to a possible violation (FR-250 `HybridTrace`, QSpec FR-193
/// "Hybrid trace").
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HybridTrace {
    /// The first state.
    pub start: TraceState,
    /// The jumps, in order.
    pub jumps: Vec<TraceJump>,
    /// The solver's tolerance; `None` reads 0.
    pub delta: Option<Rational>,
    /// The state the last state returns to: 0 for `start`, `k` for the state
    /// of `jumps[k - 1]`.
    pub loop_to: Option<u64>,
}

impl HybridTrace {
    /// The state at position `k`: `start` for 0, the state of `jumps[k - 1]`
    /// otherwise.
    pub fn state(&self, k: usize) -> Option<&TraceState> {
        match k.checked_sub(1) {
            None => Some(&self.start),
            Some(jump) => self.jumps.get(jump).map(|jump| &jump.state),
        }
    }

    /// The number of states: `start` and one per jump.
    pub fn state_count(&self) -> usize {
        self.jumps.len() + 1
    }

    /// The tolerance, 0 when absent.
    pub(super) fn tolerance(&self) -> Rational {
        self.delta.clone().unwrap_or_else(zero)
    }
}

/// Why a reader refuses a trace before replaying it: QSpec FR-193 "Hybrid
/// trace". Each is `invalid_result`/`invalid-value` at the member's pointer.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum TraceFault {
    /// `delta` is negative.
    #[error("the tolerance is negative")]
    NegativeDelta,
    /// `loop` names the last state or a state past it.
    #[error("the loop names the last state or one past it")]
    LoopNotBeforeLast,
    /// `mode` names no mode of the model.
    #[error("the mode names no mode of the model")]
    UnknownMode,
    /// `start` is in no initial mode.
    #[error("the start state is in no initial mode")]
    NotInitialMode,
    /// `transition` names no transition of the previous state's mode.
    #[error("the transition names no transition of the previous state's mode")]
    UnknownTransition,
    /// The jump reaches a mode other than its transition's target.
    #[error("the jump reaches a mode other than its transition's target")]
    WrongTarget,
    /// The enclosure does not hold one interval per continuous variable.
    #[error("the enclosure does not hold one interval per continuous variable")]
    EnclosureArity,
    /// An interval's `lower` exceeds its `upper`.
    #[error("the interval's lower end exceeds its upper end")]
    InvertedInterval,
}

/// A trace the reader refuses: `invalid_result`/`invalid-value` at
/// `pointer`.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[error("invalid_result/invalid-value at `{pointer}`: {fault}")]
pub struct TraceRefusal {
    /// The RFC 6901 pointer of the member, into the trace.
    pub pointer: JsonPointer,
    /// Why.
    pub fault: TraceFault,
}

impl TraceRefusal {
    /// The FR-272 code: `invalid_result`.
    pub const fn code(&self) -> &'static str {
        "invalid_result"
    }

    /// The FR-272 cause: `invalid-value`.
    pub const fn cause(&self) -> &'static str {
        "invalid-value"
    }
}

/// The pointer of state `k`: `/start` or `/jumps/{k-1}/state`.
pub(super) fn state_pointer(k: usize) -> JsonPointer {
    match k.checked_sub(1) {
        None => JsonPointer::root().key("start"),
        Some(jump) => jump_pointer(jump).key("state"),
    }
}

/// The pointer of `jumps[j]`.
pub(super) fn jump_pointer(j: usize) -> JsonPointer {
    JsonPointer::root().key("jumps").key(&j.to_string())
}

/// The pointer of `jumps[j].before`.
pub(super) fn before_pointer(j: usize) -> JsonPointer {
    jump_pointer(j).key("before")
}

/// `/loop`.
pub(super) fn loop_pointer() -> JsonPointer {
    JsonPointer::root().key("loop")
}

pub(super) fn zero() -> Rational {
    Rational::from_integer(0_i64.into())
}

/// The refusals of QSpec FR-193 "Hybrid trace" a reader applies before
/// replay: `delta`, then `loop`, then each state and jump in trace order.
pub fn admit(model: &CheckedHybridModel, trace: &HybridTrace) -> Result<(), TraceRefusal> {
    let refuse = |pointer: JsonPointer, fault: TraceFault| Err(TraceRefusal { pointer, fault });
    if trace
        .delta
        .as_ref()
        .is_some_and(|delta| delta.numerator().is_negative())
    {
        return refuse(JsonPointer::root().key("delta"), TraceFault::NegativeDelta);
    }
    if let Some(loop_to) = trace.loop_to {
        let last = trace.state_count() - 1;
        if usize::try_from(loop_to).map_or(true, |loop_to| loop_to >= last) {
            return refuse(loop_pointer(), TraceFault::LoopNotBeforeLast);
        }
    }
    let width = model.variables.len();
    let intervals = |pointer: JsonPointer,
                     time: &ExactInterval,
                     enclosure: &[ExactInterval]|
     -> Result<(), TraceRefusal> {
        if time.lower > time.upper {
            return refuse(pointer.key("time"), TraceFault::InvertedInterval);
        }
        if enclosure.len() != width {
            return refuse(pointer.key("enclosure"), TraceFault::EnclosureArity);
        }
        for (variable, interval) in enclosure.iter().enumerate() {
            if interval.lower > interval.upper {
                return refuse(
                    pointer
                        .key("enclosure")
                        .key(model.variables[variable].name.as_str()),
                    TraceFault::InvertedInterval,
                );
            }
        }
        Ok(())
    };
    let start = state_pointer(0);
    if model.modes.get(trace.start.mode.get()).is_none() {
        return refuse(start.key("mode"), TraceFault::UnknownMode);
    }
    if !model
        .initial
        .iter()
        .any(|entry| entry.mode == trace.start.mode)
    {
        return refuse(start.key("mode"), TraceFault::NotInitialMode);
    }
    intervals(start, &trace.start.time, &trace.start.enclosure)?;
    let mut previous = trace.start.mode;
    for (j, jump) in trace.jumps.iter().enumerate() {
        let transition = usize::try_from(jump.transition)
            .ok()
            .and_then(|t| model.modes[previous.get()].transitions.get(t));
        let Some(transition) = transition else {
            return refuse(
                jump_pointer(j).key("transition"),
                TraceFault::UnknownTransition,
            );
        };
        if let Some(before) = &jump.before {
            intervals(before_pointer(j), &before.time, &before.enclosure)?;
        }
        let state = jump_pointer(j).key("state");
        if model.modes.get(jump.state.mode.get()).is_none() {
            return refuse(state.key("mode"), TraceFault::UnknownMode);
        }
        if jump.state.mode != transition.target {
            return refuse(state.key("mode"), TraceFault::WrongTarget);
        }
        intervals(state, &jump.state.time, &jump.state.enclosure)?;
        previous = jump.state.mode;
    }
    Ok(())
}
