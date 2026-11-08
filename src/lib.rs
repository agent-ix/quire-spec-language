// SPDX-License-Identifier: AGPL-3.0-or-later
//! The root surfaces the driver and quire-protocol use:
//! command orchestration, complete-V1 editor tooling over `qsl-cst`, and the
//! checked-package v2 emitter and handoff corpus. The language stages live in
//! the `qsl-*` layer crates (ADR-011 §6.1); this crate re-exports none of them.
#[cfg(feature = "checked-handoff")]
pub mod checked_handoff;
pub mod command;
pub mod complete;
pub mod emit_checked_v2;
pub mod formal_source;
#[cfg(feature = "population-fixtures")]
pub mod population_fixtures;

// The shared integration-test support below names this crate by its package
// name; the alias lets the same file compile as a unit-test module. The module
// is public under `cfg(test)` only because most of its helpers serve the
// integration tests that include the same file, not this crate's unit tests.
#[cfg(test)]
extern crate self as quire_spec_language;
