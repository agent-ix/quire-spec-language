// SPDX-License-Identifier: AGPL-3.0-or-later
//! The unexpanded remainder of a stopped exploration run.

/// Canonical state keys not yet expanded when a run stopped, in the exact
/// order the engine would expand them next.
pub type Frontier = Vec<Vec<u8>>;

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
