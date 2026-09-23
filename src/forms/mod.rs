// SPDX-License-Identifier: AGPL-3.0-or-later
//! S2: parsed semantic forms, produced from a lossless CST with no error or
//! recovery node (ADR-011 §2.1 E2; ADR-012 §3, §4.3).
//!
//! This module is the `forms` core (ADR-011 §6.1 layer 2, M-3a): the closed,
//! family-keyed dispatch entry table over the closed leading-token-kind
//! enum, and the shared parsed-form contract. It owns no family's grammar
//! production function and no family's variant of the parsed-form enum
//! (M-3b, FR-067-CON-2); each family's own migration ticket adds both when
//! that family migrates onto S2 forms.
//!
//! `value::expression::syntax` moves here in the same change (ADR-011 §6.2
//! module map, M-3a): the `Expression` enum and its seven sibling types are
//! defined exactly once, in `syntax`, and re-exported at this module's
//! top level. `value::expression`'s own submodules, and `crate::value`'s
//! aggregating re-export, import them from here; neither is a second
//! definition (FR-067-AC-9).
//!
//! `value::expression::syntax` no longer exists as a module (FR-067-AC-9,
//! TC-169 step 1). The `use` below fails to compile, but not as proof of
//! absence by itself: `value::expression` is a private module
//! (`src/value/mod.rs`, `mod expression;`, unchanged by this move), so the
//! same `use` would fail the same way, with the same error, for a module
//! that still existed there but stayed private. `syntax`'s own test
//! module carries the real absence check, directly against the file-per-
//! module convention rather than through this crate's public API:
//!
//! ```compile_fail
//! use quire_spec_language::value::expression::syntax::Expression;
//! ```
//!
//! ## Layering
//!
//! `forms` is layer 2 (ADR-011 §6.1), which depends on layer 1 (`cst`), F
//! and K. It carries kernel types directly as parsed-form payloads
//! (ADR-013 OQ-A): `quire_exact::Integer` for an integer literal and
//! `quire_exact::CollectionKind` for a collection kind, both in an
//! [`Expression`] and in a [`TypeForm`]'s collection head. A parsed form
//! holds no `ValueType` and no `NodeKey` (FR-091-AC-11): [`Expression`]'s
//! `AllInstances`, `Lookup` and `Convert` variants, and
//! [`FunctionDeclaration`]'s `parameters`/`result`, each carry a
//! [`TypeForm`], resolved to the kernel `ValueType` only at check (E3,
//! ADR-013 O-14/C-26). See `syntax::TypeForm`'s own doc.

mod dispatch;
mod syntax;

pub use dispatch::{build_form, FormsCause, FormsRefusal, LeadingTokenKind, ParsedForm};
pub use syntax::{
    Accumulation, BinaryOperator, BinderQuery, BuiltinType, ClauseKind, DeclaredClauseKind,
    Expression, FieldInitializer, FunctionDeclaration, TypeForm, TypeFormHead,
};
