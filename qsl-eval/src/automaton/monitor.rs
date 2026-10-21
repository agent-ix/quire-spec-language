// SPDX-License-Identifier: AGPL-3.0-or-later
//! The deterministic bad-prefix monitor of a safety automaton (ADR-018
//! SM-6), materialized as the caller reaches it.
//!
//! The monitor reads its automaton through [`SafetyAutomaton`]: an explicit
//! [`Gba`], or a lazy [`PropertyAutomaton`] whose states are numbered only as
//! they are reached (ADR-018 IV-6). Neither is explored eagerly. A monitor
//! state is the subset of automaton states the prefix read can be in, keeping
//! only the *viable* ones, the states from which some infinite run exists.
//! Viability is decided per state when a step first meets it, by a
//! depth-first search over its satisfiable transitions that stops at the
//! first cycle or already viable state. So the one rejecting state, the empty
//! subset, is reached exactly when the prefix read is a bad prefix, at the
//! first position where no continuation is left.

use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

use crate::automaton::error::AutomatonError;
use crate::automaton::gba::{Gba, Transition};
use crate::automaton::letter::Letter;
use crate::automaton::numbering::Numbering;
use crate::automaton::state::StateId;
use crate::automaton::translate::PropertyAutomaton;

/// An automaton a [`Monitor`] reads: its initial state, the transitions out
/// of a state, and how many acceptance sets it carries.
pub trait SafetyAutomaton {
    /// The number of acceptance sets; a safety automaton carries none.
    fn acceptance_sets(&self) -> usize;

    /// The number of automaton states materialized so far.
    fn state_count(&self) -> usize;

    /// The state before position 0's letter.
    ///
    /// # Errors
    ///
    /// [`AutomatonError::StateLimit`] when the automaton's limit admits no
    /// state.
    fn initial_state(&mut self) -> Result<StateId, AutomatonError>;

    /// Every transition out of `from`.
    ///
    /// # Errors
    ///
    /// [`AutomatonError::UnknownState`] when `from` is not a state;
    /// [`AutomatonError::StateLimit`] when reaching a target would exceed the
    /// automaton's limit.
    fn row(&mut self, from: StateId) -> Result<Vec<Transition>, AutomatonError>;

    /// Whether `state` owes an interval eventuality that only a counted
    /// step can close (ADR-027 PB-3): a pending `eventually` or `until`
    /// deadline, or one waiting out its lower bound. An explicit automaton
    /// keeps no counters.
    fn owes_eventuality(&self, _state: StateId) -> bool {
        false
    }
}

impl SafetyAutomaton for &Gba {
    fn acceptance_sets(&self) -> usize {
        Gba::acceptance_sets(self).len()
    }

    fn state_count(&self) -> usize {
        Gba::state_count(self)
    }

    fn initial_state(&mut self) -> Result<StateId, AutomatonError> {
        Ok(Gba::INITIAL)
    }

    fn row(&mut self, from: StateId) -> Result<Vec<Transition>, AutomatonError> {
        Ok(self.transitions(from)?.to_vec())
    }
}

impl SafetyAutomaton for Gba {
    fn acceptance_sets(&self) -> usize {
        Self::acceptance_sets(self).len()
    }

    fn state_count(&self) -> usize {
        Self::state_count(self)
    }

    fn initial_state(&mut self) -> Result<StateId, AutomatonError> {
        Ok(Self::INITIAL)
    }

    fn row(&mut self, from: StateId) -> Result<Vec<Transition>, AutomatonError> {
        Ok(self.transitions(from)?.to_vec())
    }
}

impl SafetyAutomaton for PropertyAutomaton {
    fn acceptance_sets(&self) -> usize {
        self.acceptance_set_count()
    }

    fn state_count(&self) -> usize {
        self.reached_states()
    }

    fn initial_state(&mut self) -> Result<StateId, AutomatonError> {
        self.initial()
    }

    fn row(&mut self, from: StateId) -> Result<Vec<Transition>, AutomatonError> {
        self.transitions(from)
    }

    fn owes_eventuality(&self, state: StateId) -> bool {
        PropertyAutomaton::owes_eventuality(self, state)
    }
}

impl<A: SafetyAutomaton + ?Sized> SafetyAutomaton for Box<A> {
    fn acceptance_sets(&self) -> usize {
        (**self).acceptance_sets()
    }

    fn state_count(&self) -> usize {
        (**self).state_count()
    }

    fn initial_state(&mut self) -> Result<StateId, AutomatonError> {
        (**self).initial_state()
    }

    fn row(&mut self, from: StateId) -> Result<Vec<Transition>, AutomatonError> {
        (**self).row(from)
    }

    fn owes_eventuality(&self, state: StateId) -> bool {
        (**self).owes_eventuality(state)
    }
}

/// What the monitor knows of one automaton state.
#[derive(Clone, Debug, Default)]
struct Known {
    /// Its satisfiable transitions, once read, shared so a step does not
    /// copy them.
    row: Option<Rc<[Transition]>>,
    /// Whether some infinite run starts there, once decided.
    viable: Option<bool>,
}

/// The deterministic monitor of a safety automaton: its states are the
/// subsets of viable automaton states the prefix read so far can be in,
/// numbered as the caller reaches them. The one rejecting state is the empty
/// subset, reached exactly when the prefix read is a bad prefix.
#[derive(Clone, Debug)]
pub struct Monitor<A> {
    automaton: A,
    initial: Option<StateId>,
    known: BTreeMap<StateId, Known>,
    subsets: Numbering<BTreeSet<StateId>>,
}

impl<A: SafetyAutomaton> Monitor<A> {
    /// The monitor of `automaton`, numbering at most `max_states` monitor
    /// states. Nothing of the automaton is read yet.
    ///
    /// # Errors
    ///
    /// [`AutomatonError::NotSafetyAutomaton`] when `automaton` carries an
    /// acceptance set.
    pub fn bad_prefix(automaton: A, max_states: u64) -> Result<Self, AutomatonError> {
        let sets = automaton.acceptance_sets();
        if sets != 0 {
            return Err(AutomatonError::NotSafetyAutomaton { sets });
        }
        Ok(Self {
            automaton,
            initial: None,
            known: BTreeMap::new(),
            subsets: Numbering::new(max_states),
        })
    }

    /// The automaton the monitor reads.
    pub fn automaton(&self) -> &A {
        &self.automaton
    }

    /// The monitor state before any position is read.
    ///
    /// # Errors
    ///
    /// [`AutomatonError::StateLimit`] when the limit admits no state, the
    /// monitor's or the automaton's.
    pub fn initial(&mut self) -> Result<StateId, AutomatonError> {
        let subset = self.viable_initial()?.into_iter().collect();
        Ok(self.subsets.intern(subset)?.state)
    }

    /// The automaton's initial state when some infinite run starts there.
    ///
    /// # Errors
    ///
    /// As [`Self::initial`].
    pub fn viable_initial(&mut self) -> Result<Option<StateId>, AutomatonError> {
        let start = match self.initial {
            Some(start) => start,
            None => {
                let start = self.automaton.initial_state()?;
                self.initial = Some(start);
                start
            }
        };
        Ok(self.viable(start)?.then_some(start))
    }

    /// The monitor state reached from `from` by reading `letter`.
    ///
    /// # Errors
    ///
    /// [`AutomatonError::UnknownState`] when `from` is not a monitor state;
    /// [`AutomatonError::StateLimit`] when the reached state, or an automaton
    /// state the step reads, would exceed its limit.
    pub fn step(&mut self, from: StateId, letter: Letter) -> Result<StateId, AutomatonError> {
        let members = self.members_of(from)?;
        let mut next = BTreeSet::new();
        for member in members {
            next.extend(self.viable_successors(member, letter)?);
        }
        Ok(self.subsets.intern(next)?.state)
    }

    /// The automaton states reached from the automaton state `from` by
    /// reading `letter` that still have an infinite run, ascending: the
    /// states of ADR-023 HC-2's safety automaton with every state of empty
    /// language removed.
    ///
    /// # Errors
    ///
    /// [`AutomatonError::UnknownState`] when `from` is not an automaton
    /// state; [`AutomatonError::StateLimit`] when a state the step reads
    /// would exceed the automaton's limit.
    pub fn viable_successors(
        &mut self,
        from: StateId,
        letter: Letter,
    ) -> Result<Vec<StateId>, AutomatonError> {
        let mut targets = BTreeSet::new();
        for transition in self.row(from)?.iter() {
            if transition.guard.admits(letter) && self.viable(transition.target)? {
                targets.insert(transition.target);
            }
        }
        Ok(targets.into_iter().collect())
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

    fn members_of(&self, state: StateId) -> Result<Vec<StateId>, AutomatonError> {
        self.members(state)
            .map(|members| members.iter().copied().collect())
            .ok_or(AutomatonError::UnknownState { state })
    }

    fn slot(&mut self, state: StateId) -> &mut Known {
        self.known.entry(state).or_default()
    }

    fn known_viable(&self, state: StateId) -> Option<bool> {
        self.known.get(&state).and_then(|known| known.viable)
    }

    /// `state`'s satisfiable transitions, read from the automaton once.
    fn row(&mut self, state: StateId) -> Result<Rc<[Transition]>, AutomatonError> {
        if let Some(row) = self.known.get(&state).and_then(|known| known.row.as_ref()) {
            return Ok(Rc::clone(row));
        }
        let row: Rc<[Transition]> = self
            .automaton
            .row(state)?
            .into_iter()
            .filter(|transition| transition.guard.is_satisfiable())
            .collect();
        self.slot(state).row = Some(Rc::clone(&row));
        Ok(row)
    }

    /// Whether some infinite run starts at `state`: a depth-first search
    /// over satisfiable transitions that succeeds at the first state on its
    /// own path (a cycle) or already known viable, which makes every state on
    /// the path viable, and marks a state dead once every target is dead.
    fn viable(&mut self, state: StateId) -> Result<bool, AutomatonError> {
        if let Some(known) = self.known_viable(state) {
            return Ok(known);
        }
        let mut path: Vec<(StateId, usize)> = vec![(state, 0)];
        let mut on_path = BTreeSet::from([state]);
        while let Some(&(top, cursor)) = path.last() {
            let row = self.row(top)?;
            // A frame is tested once, when it is pushed: a later visit
            // follows a dead child, which neither adds a state to the path
            // nor makes a target viable.
            let closes = cursor == 0
                && row.iter().any(|transition| {
                    on_path.contains(&transition.target)
                        || self.known_viable(transition.target) == Some(true)
                });
            if closes {
                for (member, _) in path {
                    self.slot(member).viable = Some(true);
                }
                return Ok(true);
            }
            let next = row.iter().enumerate().skip(cursor).find(|(_, transition)| {
                self.known_viable(transition.target).is_none()
                    && !on_path.contains(&transition.target)
            });
            match next {
                Some((index, transition)) => {
                    if let Some(frame) = path.last_mut() {
                        frame.1 = index + 1;
                    }
                    path.push((transition.target, 0));
                    on_path.insert(transition.target);
                }
                None => {
                    self.slot(top).viable = Some(false);
                    path.pop();
                    on_path.remove(&top);
                }
            }
        }
        Ok(false)
    }
}
