// SPDX-License-Identifier: AGPL-3.0-or-later
//! The unexplored remainder of a stopped exploration run (FR-101, ADR-014
//! TR-7).

use qsl_foundation::digest::DigestRecord;
use qsl_foundation::Setting;

/// Canonical states not yet expanded when a run stopped, in the exact order
/// the engine would expand them next: each a `quire.simulation.state-key/v1`
/// digest, never the full key bytes (FR-101).
pub type Frontier = Vec<DigestRecord>;

/// Which resource limit stopped a run before its frontier went empty, with
/// the value it was set to. The search horizon `max_depth` is not a limit
/// (FR-101-AC-7): reaching it is `Outcome::BoundReached`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Limit {
    /// The distinct-state limit (`Limits::max_states`) and its value.
    States(usize),
    /// The explored-transition limit (`Limits::max_transitions`) and its
    /// value.
    Transitions(usize),
}

impl Limit {
    /// The setting that raises this limit (FR-255): `explore.states` or
    /// `explore.transitions`. The library raises it by setting the matching
    /// `Limits` member.
    pub const fn setting(self) -> Setting {
        match self {
            Self::States(_) => Setting::ExploreStates,
            Self::Transitions(_) => Setting::ExploreTransitions,
        }
    }

    /// The value the limit was set to when the run reached it.
    pub const fn value(self) -> usize {
        match self {
            Self::States(value) | Self::Transitions(value) => value,
        }
    }
}
