// SPDX-License-Identifier: AGPL-3.0-or-later
//! The S6a value evaluator (ADR-011 §6.1 layer 5).
//!
//! The layer-3 value semantics this evaluator runs over -- the §6.2
//! `semantic_value` submodules, `value::model_query` and
//! `value::application_key` -- live in the `qsl-semantics` crate's own
//! `value` module (QSL-181 X-6b). This module re-exports none of them
//! (ADR-011 §7.2): callers name them at `qsl_semantics::value::...`. It holds
//! `value::expression`, which `qsl-eval` takes at X-8, and re-exports only
//! its own items and the layer-4 `CheckedPackage` it evaluates.

mod expression;

// ADR-013 T-1 (FR-087, QSL-158 S-3a): `CheckedPackage` (S4 in-process, the
// S3 `CheckedGraph` plus the checked dependency closure) is defined in
// layer-4 `checked_package`. Re-exported through `expression` (which already
// re-exports it from `crate::checked_package` as the S6a entry point,
// FR-087-AC-9/TC-256) rather than a second, independent
// `pub use crate::checked_package::CheckedPackage;` line, so there is
// exactly one re-export source for `value` to track.
pub use expression::CheckedPackage;
// `CheckedPackageEvaluation` is the trait that carries `call`, `evaluate`
// and `emit_function_package_v2` over the foreign-to-`value`
// `CheckedPackage` typestate, since an inherent impl here would be E0116
// once `CheckedPackage` is `qsl-package`'s own type (X-7).
// `decode_function_package_v2` takes no package, so it is a free function
// beside the v2 codec it wraps.
pub use expression::{
    decode_function_package_v2, CallFailure, CheckedPackageEvaluation, DecodeV2Error, Evaluation,
    InputRefusal, InvalidQualifiedName, LocatedLoss, QualifiedName, ValueLoss,
};
