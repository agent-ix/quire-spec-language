// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-011 §6.1 layer 4, `package` (QSL-182, ADR-011 §7.3 X-7): the S4
//! in-process [`CheckedPackage`]/[`EmittedPackage`] typestate (ADR-013 T-1,
//! FR-087, QSL-158 S-3a, the `checked` module), the
//! `quire.checked-package/v2` I2 byte reader (`checked_v2`, ADR-011 §4) and
//! the v2 emitter (`emit`, ADR-011 T-8, M-4).
//!
//! This crate depends on layer 3 (`qsl-semantics`), F (`qsl-foundation`), K
//! (`quire-exact`) and `quire-contract-model` for the v2 wire vocabulary. It never names the root crate or a
//! SEAM-1 module (`checking`, `linking`, `native_model`, or the root crate's
//! native-v1 `package`): Cargo refuses the first, and the root crate is the
//! only home of the second. The layer-5 evaluator and every later layer
//! depend on this crate, never the other way.

mod checked;
mod checked_v2;
mod emit;

pub use checked::{CheckedPackage, EmittedPackage};

// ADR-011 §4 I2: `read_checked_package_v2`, its outcome type
// (`V2ReadOutcome`) and its refusal/incomplete types (`V2ReadRefusal`,
// `V2ReadIncomplete`) are all `pub(crate)` on `checked_v2` itself (QSL-6
// review) and not re-exported here: `checked_v2` returns a real
// `qsl_semantics::library::VerifiedPackage` (FR-087-AC-1, AC-3; QSL-6 slice
// A1), but it still has no caller of its own (S3, ADR-011 §4's round trip,
// QSL-6 slice A7, has not landed) beyond `checked_v2`'s own tests.
