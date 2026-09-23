// SPDX-License-Identifier: AGPL-3.0-or-later
//! The unexpanded remainder of a stopped exploration run.

/// The full canonical state key a `TransitionSystem` computes for one
/// state. Two states are the same state to the engine exactly when their
/// keys are equal.
#[derive(Clone, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct StateKey(Vec<u8>);

impl StateKey {
    /// Wrap the full canonical key bytes a `TransitionSystem` computed.
    pub fn new(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }

    /// The wrapped canonical key bytes.
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

impl From<Vec<u8>> for StateKey {
    fn from(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }
}

/// Canonical state keys not yet expanded when a run stopped, in the exact
/// order the engine would expand them next.
pub type Frontier = Vec<StateKey>;

/// Which explicit bound stopped a run before its frontier went empty.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Limit {
    /// The distinct-state ceiling (`Limits::max_states`).
    States,
    /// The breadth-first depth ceiling (`Limits::max_depth`).
    Depth,
    /// The explored-transition ceiling (`Limits::max_transitions`).
    Transitions,
}
