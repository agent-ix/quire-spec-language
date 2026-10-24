// SPDX-License-Identifier: AGPL-3.0-or-later
//! The index of one automaton state.

/// One state of an automaton, by its index. A [`Gba`](crate::automaton::Gba)
/// and a [`Numbering`](crate::automaton::Numbering) hand these out; the index
/// is storage local to that automaton or graph, never a state's identity
/// (ADR-018 §12).
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct StateId(usize);

impl StateId {
    /// The state at `index`.
    pub(crate) const fn new(index: usize) -> Self {
        Self(index)
    }

    /// The state's index.
    pub const fn index(self) -> usize {
        self.0
    }
}
