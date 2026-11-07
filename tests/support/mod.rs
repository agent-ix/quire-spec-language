// SPDX-License-Identifier: AGPL-3.0-or-later
//! Shared helpers for the root integration-test binary.

#[path = "../../examples/config-version/spine_corpus.rs"]
pub mod config_version_spine;
pub mod front_end;
pub mod protocol_artifact;
#[path = "refinement_cas.rs"]
pub mod refinement_cas;
pub mod spine_model;
pub mod workspace;

#[path = "admitted_charge.rs"]
mod admitted_charge;
pub(crate) use admitted_charge::admitted_charge_points;
