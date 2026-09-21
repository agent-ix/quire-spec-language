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

pub mod absence;
pub mod checking;
pub mod command;
pub mod complete;
pub mod diagnostic;
pub mod digest;
pub(crate) mod family;
pub mod formal_source;
pub mod format;
pub mod forms;
mod json_number;
mod lexer;
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
#[cfg(feature = "quire-extraction")]
pub mod quire_source;
pub mod runtime;
mod serde_object;
pub mod simulation;
pub mod source;
pub mod source_map;
pub mod state;
pub mod syntax;
pub mod temporal;
mod token;
pub mod value;
pub mod wire_format;

pub use diagnostic::{CatalogCode, Category, Code, Diagnostic, InternalFault, Phase};
pub use digest::ByteDigest;
pub use linking::{link, link_native, LinkLimits, LinkedPackage};
pub use parser::{parse, parse_native, parse_native_source, parse_source};
pub use source::{LocatedSpan, Position, Source, SourceIdentity, Span, Spanned};
pub use syntax::{Limits, ParsedUnit};
