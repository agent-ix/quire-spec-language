// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-108: the spine-only half of the ConfigVersion corpus -- the shared case
//! catalog and the `1-draft` unit, domain package and clause-run request
//! written from it -- with no native model, runtime or lowering import.

#[path = "cases.rs"]
pub mod cases;
pub use cases::Case;

/// FR-108: the spine counterpart of the native fixtures.
#[path = "spine.rs"]
pub mod spine;
