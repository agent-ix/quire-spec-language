// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-101 (implementing QSpec FR-181): finite exploration and seeded
//! sampling of a caller-supplied transition system, in canonical order, with
//! typed state keys and the pinned `quire.simulation.sampler/v1` generator.
//!
//! This module is the exploration engine only. It knows nothing about QSL's
//! own models, formal semantics or admitted packages: a caller implements
//! `TransitionSystem` over whatever state it has, and the engine walks it --
//! exhaustively by deterministic breadth-first search (`explore_request`,
//! producing an `Outcome`), or by seeded sampling over the same successor
//! relation (`sample_request`, producing a `Trace`). `replay` re-runs a
//! `Trace` against a `TransitionSystem`, independently of the engine.
//!
//! `explore_request` and `sample_request` are the only public entries that
//! explore or sample: `explore`, `sample` and `Sampler` are `pub(crate)`, so
//! no caller skips the requires-bound check or the sampler's
//! `GeneratorMismatch` check.

mod explore;
mod frontier;
mod key;
mod not_simulated;
mod order;
mod sample;
mod trace;

pub use explore::{explore_request, Limits, Outcome, Stats, TransitionSystem};
pub use frontier::{Frontier, Limit};
pub use key::EncodingRefusal;
pub use not_simulated::{NotSimulated, RequiresBound};
pub use sample::sample_request;
pub use trace::{replay, ReplayError, SampleProvenance, Step, StopReason, Trace};
