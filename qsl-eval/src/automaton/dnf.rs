// SPDX-License-Identifier: AGPL-3.0-or-later
//! A condition on one letter as a disjunction of guards: what the
//! translation reads a state formula as, so a transition's condition stays a
//! set of [`Guard`]s.

use crate::automaton::letter::{Atom, Guard, Letter};

/// A disjunction of satisfiable guards, sorted, none covered by another.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct Dnf(Vec<Guard>);

impl Dnf {
    /// The condition no letter meets.
    pub(super) const fn never() -> Self {
        Self(Vec::new())
    }

    /// The condition every letter meets.
    pub(super) fn always() -> Self {
        Self(vec![Guard::always()])
    }

    /// The condition that `atom` holds.
    pub(super) fn atom(atom: Atom) -> Self {
        Self(vec![Guard::always().requiring(atom)])
    }

    /// The guards whose disjunction this is.
    pub(super) fn guards(&self) -> &[Guard] {
        &self.0
    }

    /// Whether no letter meets the condition.
    pub(super) fn is_never(&self) -> bool {
        self.0.is_empty()
    }

    /// Whether `letter` meets the condition.
    pub(super) fn admits(&self, letter: Letter) -> bool {
        self.0.iter().any(|guard| guard.admits(letter))
    }

    /// The letters both conditions admit.
    pub(super) fn and(&self, other: &Self) -> Self {
        Self::reduced(
            self.0
                .iter()
                .flat_map(|left| other.0.iter().map(|right| left.conjoined(*right)))
                .collect(),
        )
    }

    /// The letters either condition admits.
    pub(super) fn or(&self, other: &Self) -> Self {
        Self::reduced(self.0.iter().chain(&other.0).copied().collect())
    }

    /// The letters this condition does not admit.
    pub(super) fn negate(&self) -> Self {
        self.0.iter().fold(Self::always(), |negation, guard| {
            let literals = Self::reduced(guard.negated_literals().collect());
            negation.and(&literals)
        })
    }

    fn reduced(mut guards: Vec<Guard>) -> Self {
        guards.retain(|guard| guard.is_satisfiable());
        guards.sort_unstable();
        guards.dedup();
        let kept = guards
            .iter()
            .filter(|guard| {
                !guards
                    .iter()
                    .any(|other| other != *guard && other.covers(**guard))
            })
            .copied()
            .collect();
        Self(kept)
    }
}
