// SPDX-License-Identifier: AGPL-3.0-or-later
//! The one refusal type of the automaton kernels.

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
}
