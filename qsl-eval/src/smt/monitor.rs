// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-315 Behavior: the safety monitor of a TP-3 item or a TP-2 `on each`
//! item (ADR-018 EN-3, LA-2), read from `qsl-eval`'s property-automaton
//! translation and tabulated for printing.
//!
//! The monitor state at a position `m<i>` is the state after reading that
//! position's letter, as EN-1 creates a product state (ADR-018 UE-4): `m0` is
//! the initial monitor state stepped over the letter at position 0. The
//! property at a position is the monitor state there not rejecting.

use qsl_semantics::check::{
    Activation, CheckedFormula, CheckedTemporalClause, FormulaNodeId, TemporalProfile,
};

use crate::automaton::{
    translate, translate_body, AutomatonError, Letter, Monitor, PropertyAutomaton, StateId,
};
use crate::smt::{Measure, Refusal, SmtEncodingRefusal};

/// One monitor transition: from a state over a letter to a state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Move {
    /// The state read from.
    pub from: usize,
    /// The truth of each atom, atom `i` being bit `i`.
    pub letter: u64,
    /// The state reached.
    pub to: usize,
}

/// A monitor tabulated over every letter of its atoms.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MonitorTable {
    atoms: Vec<FormulaNodeId>,
    initial: usize,
    rejecting: Option<usize>,
    states: usize,
    moves: Vec<Move>,
}

impl MonitorTable {
    /// The refusal for an automaton error: the state limit is a size limit.
    fn translation(error: AutomatonError) -> SmtEncodingRefusal {
        match error {
            AutomatonError::StateLimit { limit } => Refusal::TooLarge {
                measure: Measure::MonitorStates,
                value: limit.saturating_add(1),
                limit,
                setting: Measure::MonitorStates.setting(),
            }
            .into(),
            other => Refusal::ItemTranslation(other).into(),
        }
    }

    /// Tabulate the bad-prefix monitor of `clause`.
    ///
    /// # Errors
    ///
    /// [`Refusal::ItemTranslation`] when the translation does not
    /// build and [`Refusal::TooLarge`] when the monitor has more
    /// atoms or states than the encoding tabulates.
    pub fn of(
        clause: &CheckedTemporalClause,
        max_atoms: u64,
        max_states: u64,
    ) -> Result<Self, SmtEncodingRefusal> {
        if clause.activation() == Activation::Origin {
            return Self::over(clause.formula(), clause.profile(), max_atoms, max_states);
        }
        let automaton = translate(clause, max_states).map_err(Self::translation)?;
        Self::tabulate(automaton, max_atoms, max_states)
    }

    /// Tabulate the bad-prefix monitor of `body` read from the origin under
    /// `profile`, as [`MonitorTable::of`] tabulates an `on origin` clause.
    ///
    /// # Errors
    ///
    /// As [`MonitorTable::of`].
    pub fn over(
        body: &CheckedFormula,
        profile: TemporalProfile,
        max_atoms: u64,
        max_states: u64,
    ) -> Result<Self, SmtEncodingRefusal> {
        let automaton = translate_body(body, profile, max_states).map_err(Self::translation)?;
        Self::tabulate(automaton, max_atoms, max_states)
    }

    fn tabulate(
        automaton: PropertyAutomaton,
        max_atoms: u64,
        max_states: u64,
    ) -> Result<Self, SmtEncodingRefusal> {
        let atoms = automaton.atoms().to_vec();
        if u64::try_from(atoms.len()).unwrap_or(u64::MAX) > max_atoms {
            return Err(Refusal::TooLarge {
                measure: Measure::MonitorAtoms,
                value: atoms.len() as u64,
                limit: max_atoms,
                setting: Measure::MonitorAtoms.setting(),
            }
            .into());
        }
        let bits = atoms.len();
        let mut monitor = Monitor::bad_prefix(automaton, max_states).map_err(Self::translation)?;
        let initial = monitor.initial().map_err(Self::translation)?;
        let mut reached: Vec<StateId> = vec![initial];
        let mut moves = Vec::new();
        let mut next = 0;
        while let Some(&from) = reached.get(next) {
            next += 1;
            for letter in 0..(1u64 << bits) {
                let to = monitor
                    .step(from, Letter::from_bits(letter))
                    .map_err(Self::translation)?;
                let target = match reached.iter().position(|seen| *seen == to) {
                    Some(position) => position,
                    None => {
                        reached.push(to);
                        reached.len() - 1
                    }
                };
                moves.push(Move {
                    from: next - 1,
                    letter,
                    to: target,
                });
            }
        }
        let mut rejecting = None;
        for (position, state) in reached.iter().enumerate() {
            if monitor.is_rejecting(*state).map_err(Self::translation)? {
                rejecting = Some(position);
            }
        }
        Ok(Self {
            atoms,
            initial: 0,
            rejecting,
            states: reached.len(),
            moves,
        })
    }

    /// The `holds` atoms of the letter, atom `i` being bit `i`.
    pub fn atoms(&self) -> &[FormulaNodeId] {
        &self.atoms
    }

    /// The initial monitor state, read before position 0.
    pub const fn initial(&self) -> usize {
        self.initial
    }

    /// The rejecting state, when some letter sequence reaches it.
    pub const fn rejecting(&self) -> Option<usize> {
        self.rejecting
    }

    /// The number of states, numbered `0..states` in the order the table
    /// reached them.
    pub const fn states(&self) -> usize {
        self.states
    }

    /// Every transition, in ascending (from, letter) order.
    pub fn moves(&self) -> &[Move] {
        &self.moves
    }
}
