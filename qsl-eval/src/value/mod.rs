// SPDX-License-Identifier: AGPL-3.0-or-later
//! The S6a value evaluator (ADR-011 §6.1 layer 5).
//!
//! The layer-3 value semantics this evaluator runs over -- the §6.2
//! `semantic_value` submodules and `value::model_query` -- live in the `qsl-semantics` crate's own
//! `value` module (X-6b). This module re-exports none of them
//! (ADR-011 §7.2): callers name them at `qsl_semantics::value::...`. It holds
//! `value::expression`, which `qsl-eval` takes at X-8, and re-exports only
//! its own items. The layer-4 `CheckedPackage` it evaluates is
//! `qsl_package::CheckedPackage` (X-7), re-exported by none of this
//! crate.

mod expression;

// `CheckedPackageEvaluation` is the trait that carries `call` and `evaluate`
// over the foreign-to-`value` `CheckedPackage` typestate, since an inherent
// impl here is E0116: `CheckedPackage` is `qsl-package`'s own type (X-7).
pub use expression::{
    admit_arguments, evaluate_closed, evaluate_refinement_row, evaluate_reward,
    evaluate_state_predicate, evaluate_terminal_predicate, input_refusal_code, supplied_stop_cause,
    supplied_stop_code, CallFailure, CheckedPackageEvaluation, ClauseEvaluation, ClauseRead,
    ClauseSetEntry, ClauseSetEvaluation, ClauseSetOutcome, Evaluation, FrameEvaluation,
    InvalidQualifiedName, InvocationBinding, ObservationIdentity, QualifiedName, RuntimeValuePath,
    Separation, SeparationStep, StopReport, ValueInvariantEvaluation, ValueInvariantOutcome,
    ValueInvariantResult, ValuePathStep, ValuePathSubject, WitnessClaim,
};
