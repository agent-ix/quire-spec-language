// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-011 §6.1 layer 5, S6a (ADR-011 §7.3 X-8): the evaluator
//! `value::expression` -- `CheckedPackage::call` and `evaluate` through
//! [`value::CheckedPackageEvaluation`], the S6a seam and its
//! `ReferenceEvaluation` hook, the family evaluators, and the layer-5
//! prototype v2 function-package codec -- the finite exploration engine
//! [`simulation`], and the property-automaton kernels [`automaton`] and
//! fairness kernels [`fairness`] that the model checker and the certificate
//! checkers share, and the self-composition product [`hyper`] they explore.
//!
//! This crate depends on layer 4 (`qsl-package`), layer 3 (`qsl-semantics`),
//! F (`qsl-foundation`) and K (`quire-exact`). It never names the root crate
//! or a SEAM module (the old `state` and `temporal` evaluators stay in the
//! root crate until M-6c deletes them): Cargo refuses the first, and the root
//! crate is the only home of the second. Every later layer depends on this
//! crate, never the other way.
//!
//! # Where the evaluation interface lives (FR-401-AC-3)
//!
//! The interface types are defined in `quire-eval` and `quire-semantic-value`,
//! and resolve at their new paths:
//!
//! ```
//! use quire_eval::{CallFailure, EvalOutcome, Evaluation, FamilyOutcome, FamilyResult};
//! use quire_semantic_value::call::WrongSnapshotCause;
//! use quire_semantic_value::catalog::{
//!     CatalogCode, CatalogCoded, Category, InternalFault, UndefinedCoded, UndefinedReason,
//!     UndefinedRecord,
//! };
//! use quire_semantic_value::supplied_stop::{
//!     SuppliedFailure, SuppliedStop, UnsupportedHelperStop,
//! };
//! ```
//!
//! No former path resolves from another crate:
//!
//! ```compile_fail,E0432
//! use qsl_eval::value::Evaluation;
//! ```
//!
//! ```compile_fail,E0432
//! use qsl_eval::value::CallFailure;
//! ```
//!
//! ```compile_fail,E0432
//! use qsl_semantics::family::FamilyOutcome;
//! ```
//!
//! ```compile_fail,E0432
//! use qsl_semantics::family::FamilyResult;
//! ```
//!
//! ```compile_fail,E0432
//! use qsl_semantics::family::EvalOutcome;
//! ```
//!
//! ```compile_fail,E0432
//! use qsl_foundation::diagnostic::InternalFault;
//! ```
//!
//! ```compile_fail,E0432
//! use qsl_foundation::InternalFault;
//! ```
//!
//! ```compile_fail,E0432
//! use qsl_semantics::value::supplied::SuppliedStop;
//! ```
//!
//! ```compile_fail,E0432
//! use qsl_semantics::check::WrongSnapshotCause;
//! ```
//!

pub mod automaton;
pub mod fairness;
pub mod hyper;
pub mod model_subject;
pub mod simulation;
pub mod temporal;
pub mod value;
