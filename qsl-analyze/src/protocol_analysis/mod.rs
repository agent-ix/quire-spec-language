// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-036 PA-1: the protocol analysis engine, layer A, beside `model_check`.
//! It projects a global protocol onto one role (FR-570; PA-9 to PA-12),
//! reading the checked protocol through `qsl_package::protocol_view` alone
//! (PA-3), and decides realizability over the arena of the one `qsl-eval`
//! protocol transition system (FR-571; PA-16 to PA-19). It builds no
//! transition system of its own (PA-22). Nothing here serves a kind: the provider manifest is a
//! later slice (QSL-1897).

mod profile;
mod project;
mod realize;

pub use profile::{unsupported_construct, Profile};
pub use project::{project, ProjectionOutcome, ProjectionRefusal, ProjectionRequest};
pub use realize::{
    realize, replay_strategy, RealizabilityOutcome, RealizabilityRefusal, RealizabilityRequest,
    StrategyReplayFailure, ANALYZER,
};
