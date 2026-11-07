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
pub mod located_json;
pub mod model_source;
pub mod native_model;
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

#[cfg(test)]
#[path = "../tests/support/native_rule_model.rs"]
pub mod native_rule_model_test_support;

pub use linking::{link, link_native, LinkLimits, LinkedPackage};
pub use parser::{parse, parse_native, parse_native_source, parse_source};
pub use syntax::{Limits, ParsedUnit};
