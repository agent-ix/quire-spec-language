// SPDX-License-Identifier: AGPL-3.0-or-later
//! `qsl-foundation`: the ADR-011 §6.1 layer **F** foundation crate (QSL-177,
//! ADR-011 §7.3 X-2).
//!
//! Module order, per ADR-011 §6.1's F row: [`absence`] < [`json_number`] <
//! [`serde_object`] < [`digest`] < [`wire_format`] < [`source`] (with
//! [`source_map`]) < [`selection`] < [`diagnostic`]. Every module here depends only on
//! `quire-exact` (K) and external crates -- no module imports the QSL root
//! crate or a SEAM module (ADR-011 §6.1's leaf-of-QSL-workspace rule for
//! layer F).
//!
//! **`located_json` stays in the root crate for this PR.** ADR-011 §6.1
//! also places `located_json` in layer F, but its `formal_source` import
//! (SEAM-2, `located_json.rs:4`) has not retired yet -- that is M-6e's job.
//! `located_json` moves here in a follow-up once M-6e lands (see this
//! crate's own ADR-011 §7.1/§7.3 extraction note).

#![forbid(unsafe_code)]

pub mod absence;
pub mod diagnostic;
pub mod digest;
pub mod json_number;
pub mod selection;
pub mod serde_object;
pub mod source;
pub mod source_map;
pub mod wire_format;

pub use diagnostic::{
    CatalogCode, CatalogCoded, Category, Code, Diagnostic, InternalFault, Phase, SyntaxLimit,
    UndefinedCoded, UndefinedReason, UndefinedRecord,
};
pub use digest::ByteDigest;
pub use source::{LocatedSpan, Position, Source, SourceIdentity, Span, Spanned};
