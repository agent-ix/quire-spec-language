// SPDX-License-Identifier: AGPL-3.0-or-later
//! The root surfaces quire-protocol uses:
//! complete-V1 editor tooling over `qsl-cst`, and the
//! checked-package v2 emitter and handoff corpus. The language stages live in
//! the `qsl-*` layer crates (ADR-011 §6.1); this crate re-exports none of them.
/// The version of this package as built, which `cargo xtask qualify` reports
/// as the QSL version (FR-567-AC-5).
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(feature = "checked-handoff")]
pub mod checked_handoff;
pub mod complete;
pub mod emit_checked_v2;
#[cfg(feature = "population-fixtures")]
pub mod population_fixtures;

// The shared integration-test support below names this crate by its package
// name; the alias lets the same file compile as a unit-test module. The module
// is public under `cfg(test)` only because most of its helpers serve the
// integration tests that include the same file, not this crate's unit tests.
#[cfg(test)]
extern crate self as quire_spec_language;
