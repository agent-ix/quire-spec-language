// SPDX-License-Identifier: AGPL-3.0-or-later
//! The S6a value evaluator (ADR-011 §6.1 layer 5).
//!
//! The layer-3 value semantics this evaluator runs over -- the §6.2
//! `semantic_value` submodules, `value::model_query` and
//! `value::application_key` -- live in the `qsl-semantics` crate's own
//! `value` module (QSL-181 X-6b). This module re-exports none of them
//! (ADR-011 §7.2): callers name them at `qsl_semantics::value::...`. It holds
//! `value::expression`, which `qsl-eval` takes at X-8, and re-exports only
//! its own items. The layer-4 `CheckedPackage` it evaluates is
//! `qsl_package::CheckedPackage` (QSL-182 X-7), re-exported by none of this
//! crate.

mod expression;

// `CheckedPackageEvaluation` is the trait that carries `call`, `evaluate`
// and `emit_function_package_v2` over the foreign-to-`value`
// `CheckedPackage` typestate, since an inherent impl here is E0116:
// `CheckedPackage` is `qsl-package`'s own type (X-7).
// `decode_function_package_v2` takes no package, so it is a free function
// beside the v2 codec it wraps.
pub use expression::{
    decode_function_package_v2, CallFailure, CheckedPackageEvaluation, DecodeV2Error, Evaluation,
    InputRefusal, InvalidQualifiedName, LocatedLoss, QualifiedName, ValueLoss,
};
