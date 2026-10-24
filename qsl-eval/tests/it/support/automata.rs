// SPDX-License-Identifier: AGPL-3.0-or-later
//! Automaton fixtures shared by the kernel tests.

use qsl_eval::automaton::{Atom, Gba, GbaBuilder, Guard, Letter, StateId};

/// The atom at `index`.
pub fn atom(index: usize) -> Atom {
    Atom::new(index).expect("the index is below the letter width")
}

/// The letter in which exactly the atoms at `atoms` hold.
pub fn letter(atoms: &[usize]) -> Letter {
    atoms
        .iter()
        .fold(Letter::default(), |letter, index| letter.with(atom(*index)))
}

/// ADR-018 section 6's generalized Büchi automaton for the negation of
/// `always eventually P`: `eventually always not P`, with atom 0 the literal
/// `not P`. Position 0's states are the successors of [`Gba::INITIAL`]:
/// `q0` always, and the accepting `q1` when `not P` holds. Returns the
/// automaton, `q0` and `q1`.
pub fn negated_liveness() -> (Gba, StateId, StateId) {
    let not_p = Guard::always().requiring(atom(0));
    let mut builder = GbaBuilder::new();
    let q0 = builder.add_state();
    let q1 = builder.add_state();
    for (from, guard, to) in [
        (Gba::INITIAL, Guard::always(), q0),
        (Gba::INITIAL, not_p, q1),
        (q0, Guard::always(), q0),
        (q0, not_p, q1),
        (q1, not_p, q1),
    ] {
        builder
            .add_transition(from, guard, to)
            .expect("both states were added");
    }
    builder.add_acceptance_set([q1]).expect("q1 was added");
    (builder.build(), q0, q1)
}
