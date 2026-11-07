// SPDX-License-Identifier: AGPL-3.0-or-later
//! Native syntax, formatting and source diagnostics for the finite-state profile.
//!
//! Parsing does not establish model binding, type correctness or execution support.
#[cfg(feature = "checked-handoff")]
pub mod checked_handoff;
pub mod checking;
pub mod command;
pub mod complete;
pub mod emit_checked_v2;
pub mod formal_source;
pub mod linking;
pub mod number;
mod parser;
#[cfg(feature = "population-fixtures")]
pub mod population_fixtures;
pub mod protocol_artifact;
pub mod syntax;

// The shared integration-test support below names this crate by its package
// name; the alias lets the same file compile as a unit-test module. The module
// is public under `cfg(test)` only because most of its helpers serve the
// integration tests that include the same file, not this crate's unit tests.
#[cfg(test)]
extern crate self as quire_spec_language;

pub use linking::{link, LinkLimits, LinkedPackage};
pub use parser::{parse, parse_source};
pub use syntax::{Limits, ParsedUnit};
