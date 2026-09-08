// SPDX-License-Identifier: AGPL-3.0-only
//! Native syntax, formatting and source diagnostics for the finite-state profile.
//!
//! Parsing does not establish model binding, type correctness or execution support.
pub mod diagnostic;
pub mod format;
mod lexer;
mod parser;
pub mod source;
pub mod syntax;
mod token;

pub use diagnostic::{Code, Diagnostic, Phase};
pub use parser::parse;
pub use source::{LocatedSpan, Position, Source, SourceIdentity, Span};
pub use syntax::{Limits, ParsedUnit};
