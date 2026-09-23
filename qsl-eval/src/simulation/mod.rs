// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-181: finite exploration of a caller-supplied transition system.
//!
//! This module is the exploration engine only. It knows nothing about QSL's
//! own models, formal semantics or admitted packages: a caller implements
//! `TransitionSystem` over whatever state it has, and the engine walks it —
//! exhaustively by deterministic breadth-first search (`explore`, producing
//! an `Outcome`), or by seeded sampling over the same successor relation
//! (`sample`, producing a `Trace`). `replay` re-runs a `Trace` against a
//! `TransitionSystem`, independently of the engine.
//!
//! The state key a `TransitionSystem::key` returns is that implementer's
//! own canonical encoding (QSpec#129 Q1); this module never interprets it
//! beyond equality. `CounterSampler` is a deterministic reference/test
//! sampler only, not the pinned production generator
//! (`quire.simulation.sampler/v1`, QSpec#129 Q2).

mod explore;
mod frontier;
mod sample;
mod trace;

pub use explore::{explore, Limits, Outcome, Stats, TransitionSystem};
pub use frontier::{Frontier, Limit, StateKey};
pub use sample::{sample, CounterSampler, EmptyInitial, Sampler};
pub use trace::{replay, ReplayError, SampleProvenance, Step, StopReason, Trace};
