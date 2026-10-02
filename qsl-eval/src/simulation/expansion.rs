// SPDX-License-Identifier: AGPL-3.0-or-later
//! What one expansion of one state returns, and the findings the engine
//! records from it (FR-101).

use qsl_foundation::digest::DigestRecord;
use qsl_foundation::CatalogCode;

/// One expanded state's successors and findings, as a `TransitionSystem`
/// returns them. The engine orders `successors` itself (FR-101-AC-1) and
/// keeps `findings` in the order given.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Expansion<T, S, F> {
    /// Every enabled transition and its post-state, in any order.
    pub successors: Vec<(T, S)>,
    /// What the system found at this state, in its own order.
    pub findings: Vec<F>,
}

/// A `TransitionSystem` could not expand a state: exploration ends with
/// `Outcome::Stopped` and sampling with `StopReason::Stopped` at that state
/// (FR-101-AC-12, FR-101-AC-14).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ExpansionStop {
    /// Why the expansion stopped, e.g.
    /// `resource_exhausted`/`insufficient-next-charge`.
    pub cause: CatalogCode,
}

/// The findings of one expanded state (FR-101-AC-13).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StateFindings<F> {
    /// The state's `quire.simulation.state-key/v1` digest.
    pub state: DigestRecord,
    /// The state's breadth-first depth in an exploration, or its step index
    /// in a sampled trace.
    pub depth: usize,
    /// The findings, in the order the system returned them; never empty.
    pub findings: Vec<F>,
}
