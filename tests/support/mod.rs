// SPDX-License-Identifier: AGPL-3.0-or-later
//! Shared helpers for the root integration-test binary.

pub mod composed_types;
#[path = "../../examples/config-version/spine_corpus.rs"]
pub mod config_version_spine;
pub mod front_end;
pub mod located_json;
pub mod native_protocol;
pub mod native_rule_model;
pub mod protocol_artifact;
pub mod spine_model;
pub mod temporal;
pub mod workspace;

#[path = "admitted_charge.rs"]
mod admitted_charge;
pub(crate) use admitted_charge::admitted_charge_points;
