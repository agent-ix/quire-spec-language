// SPDX-License-Identifier: AGPL-3.0-or-later
//! `qsl-semantics`' integration tests: the ones that exercise only ADR-011
//! §6.1 layer 3, moved from the root crate's `tests/it/`. One
//! binary, one link step, the same shape as the root crate's `it` target.
//!
//! The modules gated on `feature = "test-support"` call this crate's
//! test-only fixture constructors (`DeclarationKey::fixture`,
//! `DomainPackageRef::fixture` and friends), which exist only under that
//! feature; `cargo test --all-features` turns it on. Gating the `mod` line
//! keeps a default-feature build from compiling them at all.

#[cfg(feature = "test-support")]
mod business_meanings;
mod child_composite;
mod complete_value_lock;
mod dispatch_linking;
mod identity_golden_vectors;
mod ieee_profiles;
mod import_view_names;
mod integer_division;
mod library_resolution;
#[cfg(feature = "test-support")]
mod model_conformance;
#[cfg(feature = "test-support")]
mod model_conformance_boundary;
#[cfg(feature = "test-support")]
mod model_dispatch;
#[cfg(feature = "test-support")]
mod model_identity_cost;
#[cfg(feature = "test-support")]
mod model_identity_full_size;
#[cfg(feature = "test-support")]
mod model_identity_links;
#[cfg(feature = "test-support")]
mod model_identity_vectors;
#[cfg(feature = "test-support")]
mod model_intake;
#[cfg(feature = "test-support")]
mod model_normalization;
mod model_operations;
#[cfg(feature = "test-support")]
mod model_population;
#[cfg(feature = "test-support")]
mod model_relationship;
#[cfg(feature = "test-support")]
mod model_systems;
mod native_scalar_equality;
mod observation_composites;
mod operation_bodies;
mod operation_clauses;
mod quantities;
mod state_clauses;
#[cfg(feature = "test-support")]
mod tc235_business_meanings;
#[cfg(feature = "test-support")]
mod type_environment_model;
mod variant_fields;
mod variant_type;

#[path = "../../../tests/support/admitted_charge.rs"]
mod admitted_charge;
pub(crate) use admitted_charge::admitted_charge_points;
