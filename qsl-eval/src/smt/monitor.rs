// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-315 Behavior: the safety monitor of a TP-3 item or a TP-2 `on each`
//! item (ADR-018 EN-3, LA-2), read from `qsl-eval`'s property-automaton
//! translation and tabulated for printing.
//!
//! The monitor state at a position `m<i>` is the state after reading that
//! position's letter, as EN-1 creates a product state (ADR-018 UE-4): `m0` is
//! the initial monitor state stepped over the letter at position 0. The
//! property at a position is the monitor state there not rejecting.

use qsl_semantics::check::{CheckedTemporalClause, FormulaNodeId};

use crate::automaton::{translate, Atom, Letter, Monitor, StateId};
use crate::smt::SmtEncodingRefusal;

/// The most `holds` atoms of a letter the encoding tabulates.
pub const MAX_ATOMS: usize = 8;
/// The most monitor states the encoding tabulates.
pub const MAX_STATES: usize = 64;

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
#[derive(Clone, Debug)]
pub struct MonitorTable {
    atoms: Vec<FormulaNodeId>,
    initial: usize,
    rejecting: Option<usize>,
    states: usize,
    moves: Vec<Move>,
}

impl MonitorTable {
    /// Tabulate the bad-prefix monitor of `clause`.
    ///
    /// # Errors
    ///
    /// [`SmtEncodingRefusal::Automaton`] when the translation does not
    /// build, [`SmtEncodingRefusal::ItemForm`] for a guarded activation, and
    /// [`SmtEncodingRefusal::MonitorBound`] when the monitor has more atoms
    /// or states than the encoding tabulates.
    pub fn of(clause: &CheckedTemporalClause) -> Result<Self, SmtEncodingRefusal> {
        let automaton = translate(clause, MAX_STATES as u64)?;
        if automaton.when().is_some() {
            return Err(SmtEncodingRefusal::ItemForm {
                form: "guarded by a `when` condition",
            });
        }
        let atoms = automaton.atoms().to_vec();
        if atoms.len() > MAX_ATOMS {
            return Err(SmtEncodingRefusal::MonitorBound {
                atoms: atoms.len(),
                states: 0,
                max_atoms: MAX_ATOMS,
                max_states: MAX_STATES,
            });
        }
        // The positions of an unrolling are all represented, so the atom
        // that marks the represented positions always holds.
        let live = automaton
            .live()
            .map_or(0, |atom: Atom| 1u64 << atom.index());
        let mut monitor = Monitor::bad_prefix(automaton, MAX_STATES as u64)?;
        let initial = monitor.initial()?;
        let mut reached: Vec<StateId> = vec![initial];
        let mut moves = Vec::new();
        let mut next = 0;
        while let Some(&from) = reached.get(next) {
            next += 1;
            for letter in 0..(1u64 << atoms.len()) {
                let to = monitor.step(from, Letter::from_bits(letter | live))?;
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
            if monitor.is_rejecting(*state)? {
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
