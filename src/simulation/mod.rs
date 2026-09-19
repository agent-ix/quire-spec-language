// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-181: finite exploration of a caller-supplied transition system.
//!
//! This module is the exploration engine only. It knows nothing about QSL's
//! own models, formal semantics or admitted packages: a caller implements
//! `TransitionSystem` over whatever state it has, and the engine walks it —
//! exhaustively by deterministic breadth-first search (`explore`), or by
//! seeded sampling over the same successor relation (`sample`). Every run
//! produces a `Trace` that `replay` can re-run without the engine.
//!
//! ## Open questions (not decided by this module)
//!
//! - Outcome disposition naming: FR-181 lines 27-28 and 34-35 describe
//!   `requires-bound`/`incomplete`/`refused` outcomes. This module's
//!   `Outcome` (`Exhaustive`/`Bounded`/`Cancelled`) names what the engine
//!   itself observed; whether and how that maps onto those dispositions,
//!   or onto FR-323, is unresolved here.
//! - Pinned sampler identity: FR-181 lines 43-44 call for sampling with "a
//!   pinned counter-based generator". `CounterSampler` is a deterministic
//!   reference/test implementation only; which generator is actually
//!   pinned for production sampling is open.
//! - State key composition: FR-181 line 39 describes the state key as a
//!   canonical digest over typed semantic state, control positions,
//!   queues, role lifetimes, observation progress and remaining bounds.
//!   `TransitionSystem::key` is supplied by the implementer; this module
//!   defines no encoding.

mod explore;
mod frontier;
mod sample;
mod trace;

pub use explore::{explore, Limits, Outcome, TransitionSystem};
pub use frontier::{Frontier, Limit};
pub use sample::{sample, CounterSampler, Sampler};
pub use trace::{replay, ReplayError, SampleProvenance, Step, Trace};
