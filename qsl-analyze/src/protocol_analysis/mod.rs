// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-036 PA-1: the protocol analysis engine, layer A, beside `model_check`.
//! It projects a global protocol onto one role (FR-570; PA-9 to PA-12),
//! reading the checked protocol through `qsl_package::protocol_view` alone
//! (PA-3), and decides realizability over the arena of the one `qsl-eval`
//! protocol transition system (FR-571; PA-16 to PA-19). It builds no
//! transition system of its own (PA-22). The engine is served through its
//! provider manifest ([`manifest`]) and its `analyze` item adapters, which
//! settle each result by the FR-573 map (PA-4, PA-20).

mod item;
pub mod manifest;
mod profile;
mod project;
mod realize;

pub(crate) use item::settle;
pub use item::{projection_item, realizability_item, ProtocolAnalysisArtifact};
pub use profile::{unsupported_construct, Profile};
pub use project::{project, ProjectionOutcome, ProjectionRefusal, ProjectionRequest};
#[cfg(feature = "test-support")]
pub use realize::realize_tampered;
pub use realize::{
    realize, replay_counterexample, replay_strategy, RealizabilityOutcome, RealizabilityRefusal,
    RealizabilityRequest, ANALYZER,
};
