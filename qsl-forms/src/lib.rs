// SPDX-License-Identifier: AGPL-3.0-or-later
//! S2: parsed semantic forms, produced from a lossless CST with no error or
//! recovery node (ADR-011 §2.1 E2; ADR-012 §3, §4.3).
//!
//! This crate is the `forms` core (ADR-011 §6.1 layer 2) — the closed,
//! family-keyed dispatch entry table over the closed leading-token-kind
//! enum, and the shared parsed-form contract — and the family form
//! builders that have migrated onto it: the `Value` family's (`value`,
//! FR-091, M-3b for `Value`). Each other family's own migration ticket adds
//! its production function and its variants when it migrates.
//!
//! The `Expression` enum and its sibling parsed-form types are defined
//! exactly once, in `syntax`, and re-exported at this crate's top level
//! (FR-067-AC-9).
//!
//! ## Layering
//!
//! `qsl-forms` is layer 2 (ADR-011 §6.1), which depends on layer 1
//! (`qsl-cst`), F (`qsl-foundation`) and K (`quire-exact`), and on nothing
//! else. It carries kernel types directly as parsed-form payloads
//! (ADR-013 OQ-A): `quire_exact::Integer` for an integer literal and
//! `quire_exact::CollectionKind` for a collection kind, both in an
//! [`Expression`] and in a [`TypeForm`]'s collection head. A parsed form
//! holds no `ValueType` and no `NodeKey` (FR-091-AC-11): [`Expression`]'s
//! `AllInstances`, `Lookup` and `Convert` variants, and
//! [`FunctionDeclaration`]'s `parameters`/`result`, each carry a
//! [`TypeForm`], resolved to the kernel `ValueType` only at check (E3,
//! ADR-013 O-14/C-26). See `syntax::TypeForm`'s own doc.

mod dispatch;
mod spans;
mod syntax;
mod value;

pub use dispatch::{
    build_unit, FormsCause, FormsFailure, FormsLimits, FormsRefusal, LeadingTokenKind, ParsedForm,
    ParsedUnit, DEFAULT_FORMS_NESTING_DEPTH,
};
pub use spans::{DeclarationSpans, ExpressionSpans, SpanId, SpanRefusal, SpansMismatch};
pub use syntax::{
    Accumulation, AliasForm, BinaryOperator, BinderQuery, BuiltinType, ClauseKind, DeclarationForm,
    DeclaredClauseKind, DeclaredName, Expression, FieldInitializer, FunctionDeclaration,
    RecordFieldForm, RecordForm, TupleForm, TypeForm, TypeFormHead, UsingAlias,
};
