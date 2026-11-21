// SPDX-License-Identifier: AGPL-3.0-or-later
//! `fsl-foundation`: the ADR-011 §6.1 layer **F** foundation crate
//! (ADR-011 §7.3 X-2).
//!
//! Module order, per ADR-011 §6.1's F row: [`backend`] < [`json_number`] < [`serde_object`] <
//! [`setting`] < [`wire_format`] < [`source`] (with
//! [`source_map`]) < [`selection`] < [`bound`] < [`diagnostic`]. Every module here depends only on
//! `filament-exact` (K) and external crates -- no module imports the FSL root
//! crate or a SEAM module (ADR-011 §6.1's leaf-of-FSL-workspace rule for
//! layer F).
//!
//! **`located_json` stays in the root crate for this PR.** ADR-011 §6.1
//! also places `located_json` in layer F, but its `formal_source` import
//! (SEAM-2, `located_json.rs:4`) has not retired yet -- that is M-6e's job.
//! `located_json` moves here in a follow-up once M-6e lands (see this
//! crate's own ADR-011 §7.1/§7.3 extraction note).

#![forbid(unsafe_code)]
#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub mod backend;
pub mod bound;
pub mod canonical_json;
pub mod diagnostic;
pub mod identity_limits;
pub mod intake_limits;
pub mod json_number;
pub mod memory;
mod node_identity;
pub mod node_wire;
pub mod reduction;
pub mod request_index;
pub mod scalar_wire;
pub mod selection;
pub mod serde_object;
pub mod setting;
pub mod source;
pub mod source_map;
/// Gated on 64-bit atomics: the total is a `u64` that a shared reference adds
/// to, which targets without them (such as `thumbv7em-none-eabi`) cannot hold.
#[cfg(target_has_atomic = "64")]
pub mod tally;
pub mod wire_format;
pub mod witness;

pub use backend::{BackendId, EmptyBackendIdentity};
pub use diagnostic::{Code, Diagnostic, Phase, SyntaxLimit};
/// FR-286: catalog pairs produced by this crate's typed diagnostic causes.
pub const CATALOG_PAIRS: &[diagnostic::CatalogCode] = &[
    diagnostic::CatalogCode::new("resource_exhausted", "allocation-failed"),
    diagnostic::CatalogCode::new("resource_exhausted", "reservation-size-overflow"),
    diagnostic::CatalogCode::new("runtime_invariant", "established-invariant-broken"),
    diagnostic::CatalogCode::new("stage_limit_exceeded", "input-bytes-exceeded"),
    diagnostic::CatalogCode::new("stage_limit_exceeded", "token-count-exceeded"),
    diagnostic::CatalogCode::new("stage_limit_exceeded", "node-count-exceeded"),
    diagnostic::CatalogCode::new("stage_limit_exceeded", "edge-count-exceeded"),
    diagnostic::CatalogCode::new("stage_limit_exceeded", "occurrence-count-exceeded"),
    diagnostic::CatalogCode::new("stage_limit_exceeded", "diagnostic-count-exceeded"),
    diagnostic::CatalogCode::new("stage_limit_exceeded", "work-budget-exceeded"),
    diagnostic::CatalogCode::new("stage_limit_exceeded", "logical-storage-exceeded"),
    diagnostic::CatalogCode::new("stage_limit_exceeded", "rational-bits-exceeded"),
    diagnostic::CatalogCode::new("wrong_snapshot", "wrong-anchor"),
];
pub use identity_limits::{IdentityLimits, DEFAULT_IDENTITY_INPUT_BYTES};
pub use intake_limits::{IntakeLimits, DEFAULT_INTAKE_INPUT_BYTES, DEFAULT_SEQUENCE_MAXIMUM};
pub use memory::MemoryModelName;
pub use reduction::ReductionKind;
pub use request_index::RequestIndex;
pub use setting::{
    AccountingSetting, Setting, SettingLimits, SettingName, UsageCause, UsageRefusal,
};
pub use source::{LocatedSpan, Position, Source, SourceIdentity, SourceLabel, Span, Spanned};
