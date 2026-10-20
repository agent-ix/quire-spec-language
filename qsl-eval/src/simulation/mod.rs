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
//! `GeneratorMismatch` check. `weighted_choice` is the generator's one
//! selection function (FR-188): it reads draws at a choice point and runs no
//! transition system. `sample_probabilistic` samples a probabilistic model
//! under a workload (FR-188), over FR-187's step structure.

mod choice;
mod expansion;
mod explore;
mod footprint;
mod frontier;
mod key;
pub mod model;
mod not_simulated;
mod order;
pub mod protocol;
mod reduction;
mod sample;
mod sampled;
mod subject;
mod trace;

pub use choice::{weighted_choice, ChoicePoint, TotalAboveDrawRange, Weights, WeightsRefusal};
pub use expansion::{Expansion, ExpansionLimit, ExpansionStop, StateFindings};
pub use explore::{explore_request, Exploration, Limits, Outcome, Stats, TransitionSystem};
pub use footprint::{any_affects, independent, Footprint, Location};
pub use frontier::{Frontier, Limit};
pub use key::EncodingRefusal;
pub use not_simulated::{NotSimulated, RequiresBound, BOUND_POSITIONS};
pub use reduction::{
    ConstraintValue, OfferedReductions, Permutation, SymmetryClass, SymmetryGroup,
};
pub use sample::{sample_request, sample_request_until, SampleRequest};
pub use sampled::{
    replay_sampled, sample_probabilistic, SampleStop, SampledBehaviour, SampledDisagreement,
    SampledRecord, SampledReplay, SampledReplayRefusal, SampledStep,
};
pub use subject::SubjectSystem;
pub use trace::{replay, ReplayError, SampleProvenance, Step, StopReason, Trace};
