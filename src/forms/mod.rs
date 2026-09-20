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
//! ## Declared layer violation (QSL-146, narrowed; QSL-131 owns the rest)
//!
//! `forms` is layer 2 (ADR-011 §6.1), whose allow-list is layer 1 (`cst`)
//! and F only. This module does not conform. QSL-146 resolved three of the
//! four carrier types named when this violation was first declared:
//! `syntax`'s `AbsenceMode` field now imports `crate::absence::AbsenceMode`
//! (F, allow-listed — QSL-146 moved it out of `model::population`, layer 3),
//! and its `CollectionKind` and `Integer` fields now import
//! `quire_exact::{CollectionKind, Integer}` (layer K, allow-listed — a
//! sibling crate, not a QSL layer at all). One carrier type remains,
//! independently in two places:
//!
//! - [`Expression`]: its `AllInstances`, `Lookup` and `Convert`
//!   variants each carry a `crate::value::ValueType` field (layer 5).
//! - [`FunctionDeclaration`]: `parameters: Vec<(String, ValueType)>`
//!   (`syntax.rs:406`) and `result: ValueType` (`syntax.rs:408`) each carry
//!   `crate::value::ValueType` (layer 5) directly, independent of
//!   `Expression`.
//!
//! `ParsedForm`'s `expression` field is unconditional production code (not
//! test-only), so this remains a hard dependency of the `forms` core on
//! layer 5, not an incidental one.
//!
//! This is a contradiction between two decision records, not an
//! implementation mistake this module can fix alone: ADR-012 §4.3 places
//! the one shared `Expression` enum inside the `forms` core by design, and
//! ADR-011 §6.1 assigns `forms` a layer-2 allow-list that `Expression`'s and
//! `FunctionDeclaration`'s own fields cannot satisfy while `ValueType` lives
//! where it lives today. Unlike `CollectionKind` and `Integer`,
//! `quire-exact`'s `ValueType`/`Value` are not a byte-identical duplicate of
//! the QSL copy — their `Enum`, `Reference` and `Quantity` payloads are a
//! redesigned target shape, and `quire-exact`'s `Value` has no `Population`
//! variant at all — so retargeting `ValueType` is not the mechanical repoint
//! QSL-146 did for the other three. QSL-131 (#213 S-1b) owns that
//! replacement; once it lands, `ValueType` in `syntax` retargets to
//! `quire_exact`'s copy (layer K, allow-listed) and `forms` becomes
//! layer-2-legal without a further change to this module's shape.

mod dispatch;
mod syntax;

pub use dispatch::{build_form, FormsCause, FormsRefusal, LeadingTokenKind, ParsedForm};
pub use syntax::{
    Accumulation, BinaryOperator, BinderQuery, ClauseKind, DeclaredClauseKind, Expression,
    FieldInitializer, FunctionDeclaration,
};
