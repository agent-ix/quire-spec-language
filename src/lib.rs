// SPDX-License-Identifier: AGPL-3.0-or-later
//! Native syntax, formatting and source diagnostics for the finite-state profile.
//!
//! Parsing does not establish model binding, type correctness or execution support.
// Private unit tests reuse the public fixture producer without exposing a test bypass.
#[cfg(test)]
extern crate self as quire_spec_language;
// Compile the public fixture setup once for private runtime and package tests.
#[cfg(test)]
#[path = "../tests/support/runtime_setup.rs"]
mod runtime_test_setup;

pub mod check;
pub mod checked_package;
pub mod checking;
pub mod command;
pub mod complete;
pub(crate) mod family;
pub mod formal_source;
pub mod format;
pub mod library;
pub mod linking;
pub mod located_json;
pub mod lowering;
pub mod mapped;
pub mod model;
pub mod model_source;
pub mod native_model;
pub mod package;
mod parser;
pub mod protocol_artifact;
pub mod route;
pub mod runtime;
pub mod simulation;
pub mod state;
pub mod syntax;
pub mod temporal;
pub mod value;

pub use linking::{link, link_native, LinkLimits, LinkedPackage};
pub use parser::{parse, parse_native, parse_native_source, parse_source};
pub use syntax::{Limits, ParsedUnit};
