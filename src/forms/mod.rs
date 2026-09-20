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
//! defined exactly once, in [`syntax`], and re-exported at this module's
//! top level. `value::expression`'s own submodules, and `crate::value`'s
//! aggregating re-export, import them from here; neither is a second
//! definition (FR-067-AC-9).

mod dispatch;
mod syntax;

pub use dispatch::{build_form, FormsCause, FormsRefusal, LeadingTokenKind, ParsedForm};
pub use syntax::{
    Accumulation, BinaryOperator, BinderQuery, ClauseKind, DeclaredClauseKind, Expression,
    FieldInitializer, FunctionDeclaration,
};
