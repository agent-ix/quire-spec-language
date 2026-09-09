// SPDX-License-Identifier: AGPL-3.0-only
//! Native syntax, formatting and source diagnostics for the finite-state profile.
//!
//! Parsing does not establish model binding, type correctness or execution support.
pub mod checking;
pub mod diagnostic;
pub mod digest;
pub mod formal_source;
pub mod format;
mod lexer;
pub mod linking;
pub mod native_model;
mod parser;
pub mod runtime;
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
