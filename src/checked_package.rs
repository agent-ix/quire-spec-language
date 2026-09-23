// SPDX-License-Identifier: AGPL-3.0-or-later
//! Layer-4 `package` (ADR-011 §6.1), the future `qsl-package` crate
//! (QSL-182, ADR-011 §7.3 X-7): the S4 in-process
//! [`CheckedPackage`]/[`EmittedPackage`] typestate (ADR-013 T-1, FR-087,
//! QSL-158 S-3a, this module's own `checked` submodule), the
//! `quire.checked-package/v2` I2 byte reader (`checked_v2`, ADR-011 §4) and
//! the v2 emitter (`emit`, ADR-011 T-8, M-4).
//!
//! Split out of `src/package.rs` (QSL-182 prep): this module tree is meant
//! to become `qsl-package`'s own crate root unchanged, so its own imports
//! are exactly what that future crate may depend on -- layer 3, F, K, and
//! `quire-contract-model` and other external crates (ADR-011 §6.1's
//! exhaustive allow-list for layer 4) -- and never SEAM-1 (`checking`,
//! `linking`, `native_model`, or `package`'s own staying submodules).
//! Nothing here may depend back on `package`.
//!
//! Two test-only imports predate this doc and do not yet satisfy that
//! allow-list: `emit`'s tests reach `qsl_forms` (layer 2) and the root
//! `crate::value` (K's copy is pending QSL-131). Both must change when this
//! tree becomes a real crate root (QSL-182 X-7); recorded here as a plain
//! fact, not a plan.

mod checked;
mod checked_v2;
mod emit;

pub use checked::{CheckedPackage, EmittedPackage};

// ADR-011 §4 I2: `read_checked_package_v2`, its outcome type
// (`V2ReadOutcome`) and its refusal/incomplete types (`V2ReadRefusal`,
// `V2ReadIncomplete`) are all `pub(crate)` on `checked_v2` itself (QSL-6
// review) and not re-exported here: `checked_v2` now returns a real
// `qsl_semantics::library::VerifiedPackage` (FR-087-AC-1, AC-3; QSL-6 slice A1), but
// this module still has no caller of its own (S3, ADR-011 §4's round trip,
// QSL-6 slice A7, has not landed) beyond `checked_v2`'s own tests.
