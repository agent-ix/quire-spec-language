// SPDX-License-Identifier: AGPL-3.0-or-later
//! `qsl-semantics`' integration tests: the ones that exercise only ADR-011
//! §6.1 layer 3 (QSL-181), moved from the root crate's `tests/it/`. One
//! binary, one link step, the same shape as the root crate's `it` target.
//!
//! The modules gated on `feature = "test-support"` call this crate's
//! test-only fixture constructors (`DeclarationKey::fixture`,
//! `DomainPackageRef::fixture` and friends), which exist only under that
//! feature; `cargo test --all-features` turns it on. Gating the `mod` line
//! keeps a default-feature build from compiling them at all.

mod complete_value_lock;
mod ieee_profiles;
mod integer_division;
mod library_resolution;
#[cfg(feature = "test-support")]
mod model_conformance;
#[cfg(feature = "test-support")]
mod model_dispatch;
mod model_intake;
#[cfg(feature = "test-support")]
mod model_normalization;
#[cfg(feature = "test-support")]
mod model_population;
#[cfg(feature = "test-support")]
mod model_systems;
mod quantities;
