// SPDX-License-Identifier: AGPL-3.0-or-later
//! The one refusal type of the automaton kernels and of the property-automaton
//! translation built on them.

use qsl_semantics::check::TemporalProfile;
use thiserror::Error;

use crate::automaton::state::StateId;

/// Why an automaton kernel refused its input or stopped.
#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
pub enum AutomatonError {
    /// An atom index is not below the number of atoms a letter holds.
    #[error("atom index {index} is not below the {max} atoms a letter holds")]
    AtomOutOfRange {
        /// The index asked for.
        index: usize,
        /// The number of atoms a letter holds.
        max: usize,
    },
    /// A state does not exist in the automaton or numbering asked.
    #[error("automaton state {state:?} does not exist")]
    UnknownState {
        /// The state asked for.
        state: StateId,
    },
    /// A monitor was asked for over an automaton that carries acceptance
    /// sets; only a safety automaton has a bad-prefix monitor.
    #[error("a safety automaton carries no acceptance sets; this one has {sets}")]
    NotSafetyAutomaton {
        /// The number of acceptance sets the automaton carries.
        sets: usize,
    },
    /// Numbering one more state would exceed the state limit
    /// (`max_automaton_states`, ADR-018 IV-6).
    #[error("the automaton reached its limit of {limit} states")]
    StateLimit {
        /// The limit that was reached.
        limit: u64,
    },
    /// An interval's lower bound exceeds its upper bound.
    #[error("interval lower bound {lower} exceeds upper bound {upper}")]
    InvertedInterval {
        /// The lower bound.
        lower: u64,
        /// The upper bound.
        upper: u64,
    },
    /// A past operator's operand holds a future operator: its truth at the
    /// previous position is not a function of the history, so the automaton
    /// cannot track it as state (ADR-018 SM-6).
    #[error("a past operator's operand holds a future operator")]
    PastOverFuture,
    /// A past operator with an interval, over a subject whose steps count
    /// by kind: the memory of a past operator keeps its ages in positions,
    /// and is not read in counted steps yet (ADR-027 PB-3).
    #[error("a past operator with an interval is not read in counted steps yet")]
    PastIntervalInCountedSteps,
    /// The clause's profile has no property automaton.
    #[error("the {profile:?} profile has no property automaton")]
    ProfileNotTranslated {
        /// The profile the clause was checked under.
        profile: TemporalProfile,
    },
    /// The translation's own invariant broke.
    #[error("the property-automaton translation broke its invariant `{invariant}`")]
    Invariant {
        /// The stable name of the invariant.
        invariant: &'static str,
    },
}
