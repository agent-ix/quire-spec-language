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
#[path = "refinement_cas.rs"]
pub mod refinement_cas;
pub mod spine_model;
pub mod temporal;
pub mod workspace;

#[path = "admitted_charge.rs"]
mod admitted_charge;
pub(crate) use admitted_charge::admitted_charge_points;

/// The admitted artifact of `native-package/model-source.json` written before
/// the package producer existed (TC-101): `model-artifact.json`.
pub(crate) fn historical_model_artifact() -> &'static [u8] {
    include_bytes!("../fixtures/native-package/model-artifact.json")
}
