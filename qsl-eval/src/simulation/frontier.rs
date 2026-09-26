// SPDX-License-Identifier: AGPL-3.0-or-later
//! The unexplored remainder of a stopped exploration run (FR-101, ADR-014
//! TR-7).

use qsl_foundation::digest::DigestRecord;

/// Canonical states not yet expanded when a run stopped, in the exact order
/// the engine would expand them next: each a `quire.simulation.state-key/v1`
/// digest, never the full key bytes (FR-101).
pub type Frontier = Vec<DigestRecord>;

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
