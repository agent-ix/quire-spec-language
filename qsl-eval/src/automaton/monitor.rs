// SPDX-License-Identifier: AGPL-3.0-or-later
//! The deterministic bad-prefix monitor of a safety automaton (ADR-018
//! SM-6).

use std::collections::BTreeSet;

use crate::automaton::error::AutomatonError;
use crate::automaton::gba::{Gba, Transition};
use crate::automaton::letter::Letter;
use crate::automaton::numbering::Numbering;
use crate::automaton::state::StateId;

/// The deterministic monitor of a safety automaton: its states are the
/// subsets of automaton states the prefix read so far can be in, numbered as
/// the caller reaches them. The one rejecting state is the empty subset,
/// reached exactly when the prefix read is a bad prefix, because the
/// automaton was first trimmed to the states from which some infinite run
/// exists.
#[derive(Clone, Debug)]
pub struct Monitor {
    rows: Vec<Vec<Transition>>,
    initial_is_live: bool,
    subsets: Numbering<BTreeSet<StateId>>,
}

impl Monitor {
    /// The monitor of `automaton`, numbering at most `max_states` monitor
    /// states.
    ///
    /// # Errors
    ///
    /// [`AutomatonError::NotSafetyAutomaton`] when `automaton` carries an
    /// acceptance set.
    pub fn bad_prefix(automaton: &Gba, max_states: u64) -> Result<Self, AutomatonError> {
        let sets = automaton.acceptance_sets().len();
        if sets != 0 {
            return Err(AutomatonError::NotSafetyAutomaton { sets });
        }
        let (rows, initial_is_live) = trim(automaton);
        Ok(Self {
            rows,
            initial_is_live,
            subsets: Numbering::new(max_states),
        })
    }

    /// The monitor state before any position is read.
    ///
    /// # Errors
    ///
    /// [`AutomatonError::StateLimit`] when the limit admits no state.
    pub fn initial(&mut self) -> Result<StateId, AutomatonError> {
        let mut start = BTreeSet::new();
        if self.initial_is_live {
            start.insert(Gba::INITIAL);
        }
        Ok(self.subsets.intern(start)?.state)
    }

    /// The monitor state reached from `from` by reading `letter`.
    ///
    /// # Errors
    ///
    /// [`AutomatonError::UnknownState`] when `from` is not a monitor state;
    /// [`AutomatonError::StateLimit`] when the reached state would exceed
    /// the limit.
    pub fn step(&mut self, from: StateId, letter: Letter) -> Result<StateId, AutomatonError> {
        let mut next = BTreeSet::new();
        {
            let subset = self
                .subsets
                .key(from)
                .ok_or(AutomatonError::UnknownState { state: from })?;
            for state in subset {
                if let Some(row) = self.rows.get(state.index()) {
                    next.extend(
                        row.iter()
                            .filter(|transition| transition.guard.admits(letter))
                            .map(|transition| transition.target),
                    );
                }
            }
        }
        Ok(self.subsets.intern(next)?.state)
    }

    /// Whether `state` is the rejecting state: the prefix read is a bad
    /// prefix.
    ///
    /// # Errors
    ///
    /// [`AutomatonError::UnknownState`] when `state` is not a monitor state.
    pub fn is_rejecting(&self, state: StateId) -> Result<bool, AutomatonError> {
        self.members(state)
            .map(BTreeSet::is_empty)
            .ok_or(AutomatonError::UnknownState { state })
    }

    /// The automaton states `state` stands for.
    pub fn members(&self, state: StateId) -> Option<&BTreeSet<StateId>> {
        self.subsets.key(state)
    }

    /// The number of monitor states numbered so far.
    pub fn state_count(&self) -> usize {
        self.subsets.len()
    }
}

/// `automaton`'s rows trimmed to the states from which some infinite run
/// exists, and whether the initial state is one of them. A transition
/// survives when its guard is satisfiable and its target is live.
fn trim(automaton: &Gba) -> (Vec<Vec<Transition>>, bool) {
    let rows = automaton.rows();
    let mut live = vec![true; rows.len()];
    loop {
        let next: Vec<bool> = rows
            .iter()
            .enumerate()
            .map(|(state, row)| is_live(&live, state) && row.iter().any(|t| keeps(&live, t)))
            .collect();
        if next == live {
            break;
        }
        live = next;
    }
    let trimmed = rows
        .iter()
        .map(|row| row.iter().filter(|t| keeps(&live, t)).copied().collect())
        .collect();
    (trimmed, is_live(&live, Gba::INITIAL.index()))
}

fn is_live(live: &[bool], state: usize) -> bool {
    live.get(state).copied().unwrap_or(false)
}

fn keeps(live: &[bool], transition: &Transition) -> bool {
    transition.guard.is_satisfiable() && is_live(live, transition.target.index())
}
