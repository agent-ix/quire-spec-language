// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-011 §6.1 layer 3: QSL's semantic core (QSL-181, ADR-011 §7.3 X-6).
//!
//! In §6.1's order, `semantic_value < model < library < check core`:
//!
//! - [`value`]: the §6.2 `semantic_value` modules (definitions, enumerations,
//!   units, quantities, declarations, containment, semantic nodes), plus
//!   `value::model_query` (§6.2 `model`) and `value::application_key` (§6.2
//!   `check`);
//! - [`model`]: domain packages, keys, normalization, dispatch, population
//!   and conformance, with `model::intake`, the only module that names the
//!   FCD crates;
//! - [`library`] and [`complete`]: library resolution, package identity and
//!   complete-V1 package selection;
//! - [`check`] and [`family`]: the S3 check core, its family checkers and the
//!   check-core family contract.
//!
//! This crate depends on layer 2 (`qsl-forms`), F (`qsl-foundation`) and K
//! (`quire-exact`) only. It never parses: callers pass the values they pulled
//! out of their own parse. Later layers (the checked package, the S6a
//! evaluator, SEAM and native tooling) live in `quire-spec-language` and
//! depend on this crate, never the other way.

pub mod check;
pub mod complete;
pub mod family;
pub mod library;
pub mod model;
pub mod value;
