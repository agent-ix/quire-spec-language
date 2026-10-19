// SPDX-License-Identifier: AGPL-3.0-or-later
//! The generalized Büchi automaton over letters (ADR-018 SM-6).

use std::collections::BTreeSet;

use crate::automaton::error::AutomatonError;
use crate::automaton::letter::{Guard, Letter};
use crate::automaton::state::StateId;

/// One transition: the guard on the letter and the state it leads to.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Transition {
    /// The condition on the letter that enables the transition.
    pub guard: Guard,
    /// The state the transition leads to.
    pub target: StateId,
}

/// A generalized Büchi automaton: states numbered from the initial state 0,
/// guarded transitions, and a list of acceptance sets. A run accepts when it
/// visits every acceptance set infinitely often; with no acceptance set
/// every infinite run accepts, which is a safety automaton.
///
/// The automaton reads a letter at every position, starting from
/// [`Gba::INITIAL`]: the states a behaviour's position 0 can be in are the
/// successors of the initial state on position 0's letter (ADR-018 §6).
/// Each state's transitions are in canonical order: by target, then guard.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Gba {
    rows: Vec<Vec<Transition>>,
    acceptance: Vec<BTreeSet<StateId>>,
}

impl Gba {
    /// The initial state, read before position 0's letter.
    pub const INITIAL: StateId = StateId::new(0);

    /// The number of states.
    pub fn state_count(&self) -> usize {
        self.rows.len()
    }

    /// The transitions out of `state`, in canonical order.
    ///
    /// # Errors
    ///
    /// [`AutomatonError::UnknownState`] when the automaton has no such
    /// state.
    pub fn transitions(&self, state: StateId) -> Result<&[Transition], AutomatonError> {
        self.rows
            .get(state.index())
            .map(Vec::as_slice)
            .ok_or(AutomatonError::UnknownState { state })
    }

    /// The states reached from `from` on `letter`, ascending and without
    /// repeats.
    ///
    /// # Errors
    ///
    /// [`AutomatonError::UnknownState`] when the automaton has no such
    /// state.
    pub fn successors(
        &self,
        from: StateId,
        letter: Letter,
    ) -> Result<Vec<StateId>, AutomatonError> {
        let mut targets: Vec<StateId> = self
            .transitions(from)?
            .iter()
            .filter(|transition| transition.guard.admits(letter))
            .map(|transition| transition.target)
            .collect();
        targets.sort_unstable();
        targets.dedup();
        Ok(targets)
    }

    /// The acceptance sets, in the order they were added.
    pub fn acceptance_sets(&self) -> &[BTreeSet<StateId>] {
        &self.acceptance
    }

    /// Whether `state` belongs to acceptance set `set`.
    pub fn is_accepting_in(&self, set: usize, state: StateId) -> bool {
        self.acceptance
            .get(set)
            .is_some_and(|members| members.contains(&state))
    }
}

/// Builds a [`Gba`]. A new builder holds the initial state alone.
#[derive(Clone, Debug)]
pub struct GbaBuilder {
    rows: Vec<Vec<Transition>>,
    acceptance: Vec<BTreeSet<StateId>>,
}

impl Default for GbaBuilder {
    fn default() -> Self {
        Self {
            rows: vec![Vec::new()],
            acceptance: Vec::new(),
        }
    }
}

impl GbaBuilder {
    /// A builder holding the initial state alone.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add one state and return it.
    pub fn add_state(&mut self) -> StateId {
        let state = StateId::new(self.rows.len());
        self.rows.push(Vec::new());
        state
    }

    /// Add a transition from `from` to `to` enabled by `guard`.
    ///
    /// # Errors
    ///
    /// [`AutomatonError::UnknownState`] when either state has not been
    /// added.
    pub fn add_transition(
        &mut self,
        from: StateId,
        guard: Guard,
        to: StateId,
    ) -> Result<(), AutomatonError> {
        self.require_state(from)?;
        self.require_state(to)?;
        if let Some(row) = self.rows.get_mut(from.index()) {
            row.push(Transition { guard, target: to });
        }
        Ok(())
    }

    /// Add an acceptance set.
    ///
    /// # Errors
    ///
    /// [`AutomatonError::UnknownState`] when a member has not been added.
    pub fn add_acceptance_set(
        &mut self,
        members: impl IntoIterator<Item = StateId>,
    ) -> Result<(), AutomatonError> {
        let set: BTreeSet<StateId> = members.into_iter().collect();
        for state in &set {
            self.require_state(*state)?;
        }
        self.acceptance.push(set);
        Ok(())
    }

    /// The automaton, each state's transitions in canonical order.
    pub fn build(self) -> Gba {
        let mut rows = self.rows;
        for row in &mut rows {
            row.sort_unstable_by_key(|transition| (transition.target, transition.guard));
            row.dedup();
        }
        Gba {
            rows,
            acceptance: self.acceptance,
        }
    }

    fn require_state(&self, state: StateId) -> Result<(), AutomatonError> {
        if state.index() < self.rows.len() {
            Ok(())
        } else {
            Err(AutomatonError::UnknownState { state })
        }
    }
}
