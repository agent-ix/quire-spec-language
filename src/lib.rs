// SPDX-License-Identifier: AGPL-3.0-only
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

pub mod checking;
pub mod command;
pub mod diagnostic;
pub mod digest;
pub mod formal_source;
pub mod format;
mod lexer;
pub mod linking;
pub mod located_json;
pub mod lowering;
pub mod mapped;
pub mod model_source;
pub mod native_model;
pub mod package;
mod parser;
#[cfg(feature = "quire-extraction")]
pub mod quire_source;
pub mod runtime;
mod serde_object;
pub mod source;
pub mod source_map;
pub mod syntax;
mod token;

pub use diagnostic::{Code, Diagnostic, Phase};
pub use digest::ByteDigest;
pub use linking::{link, link_native, LinkLimits, LinkedPackage};
pub use parser::{parse, parse_source};
pub use source::{LocatedSpan, Position, Source, SourceIdentity, Span, Spanned};
pub use syntax::{Limits, ParsedUnit};
