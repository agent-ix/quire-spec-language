// SPDX-License-Identifier: AGPL-3.0-or-later
//! Integration tests that reach `qsl-replay` through its public facade only.

mod category_facade;
mod compile_protocol_profile;
mod format_outcome_facade;
mod outcome_facade;
mod parse_outcome_facade;
#[path = "../../../tests/support/refinement_cas.rs"]
mod refinement_fixture;
mod refinement_mapping;
mod refinement_ring;
mod refinement_step;
mod refinement_unmapped;
#[path = "../../../tests/support/ring_world.rs"]
mod ring_world;
