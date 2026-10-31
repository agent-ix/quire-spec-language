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
//! use qsl_semantics::value::supplied::SuppliedRun;
//! ```
//!
//! ```compile_fail,E0432
//! use qsl_semantics::model::refusal::render_identity;
//! ```
//!
//! ```compile_fail,E0432
//! use qsl_semantics::value::modulo;
//! ```
//!
//! ```compile_fail,E0432
//! use qsl_semantics::check::WrongSnapshotCause;
//! ```
//!

//! # Checked-term admission (TC-136)
//!
//! These public-API fixtures pair accepted typed handles with rejected raw
//! bytes and freely constructed wire records. Each rejected call has its own
//! fixture so an earlier type error cannot hide a later admission attempt.
//!
//! ```no_run
//! use qsl_eval::value::{CheckedPackageEvaluation, QualifiedName};
//! use qsl_package::CheckedPackage;
//! use qsl_semantics::{check::CheckedExpression, model::object_environment::ObjectEnvironment};
//! use quire_exact::Meter;
//! fn accepted(package: &CheckedPackage, name: &QualifiedName, expression: &CheckedExpression,
//!             objects: &ObjectEnvironment, meter: &mut Meter) {
//!     let _ = package.call(name, vec![], objects, meter);
//!     let _ = package.evaluate(expression, vec![], objects, meter);
//! }
//! ```
//!
//! ```compile_fail,E0308
//! use qsl_eval::value::CheckedPackageEvaluation;
//! use qsl_package::CheckedPackage;
//! use qsl_semantics::model::object_environment::ObjectEnvironment;
//! use quire_exact::Meter;
//! struct WireExpression { name: &'static str }
//! fn rejected(package: &CheckedPackage, objects: &ObjectEnvironment, meter: &mut Meter) {
//!     let _ = package.call(b"unchecked".as_slice(), vec![], objects, meter);
//! }
//! ```
//!
//! ```compile_fail,E0308
//! use qsl_eval::value::CheckedPackageEvaluation;
//! use qsl_package::CheckedPackage;
//! use qsl_semantics::model::object_environment::ObjectEnvironment;
//! use quire_exact::Meter;
//! struct WireExpression { name: &'static str }
//! fn rejected(package: &CheckedPackage, objects: &ObjectEnvironment, meter: &mut Meter) {
//!     let _ = package.call(&WireExpression { name: "unchecked" }, vec![], objects, meter);
//! }
//! ```
//!
//! ```compile_fail,E0308
//! use qsl_eval::value::CheckedPackageEvaluation;
//! use qsl_package::CheckedPackage;
//! use qsl_semantics::model::object_environment::ObjectEnvironment;
//! use quire_exact::Meter;
//! struct WireExpression { name: &'static str }
//! fn rejected(package: &CheckedPackage, objects: &ObjectEnvironment, meter: &mut Meter) {
//!     let _ = package.evaluate(b"unchecked".as_slice(), vec![], objects, meter);
//! }
//! ```
//!
//! ```compile_fail,E0308
//! use qsl_eval::value::CheckedPackageEvaluation;
//! use qsl_package::CheckedPackage;
//! use qsl_semantics::model::object_environment::ObjectEnvironment;
//! use quire_exact::Meter;
//! struct WireExpression { name: &'static str }
//! fn rejected(package: &CheckedPackage, objects: &ObjectEnvironment, meter: &mut Meter) {
//!     let _ = package.evaluate(&WireExpression { name: "unchecked" }, vec![], objects, meter);
//! }
//! ```
//!
//! A checked expression cannot be freely constructed, even when its result
//! is intended to be a simple Boolean.
//!
//! ```compile_fail,E0616
//! use qsl_semantics::check::CheckedExpression;
//! use quire_semantic_value::checking::CheckingLimits;
//! fn hand_built(expression: CheckedExpression) -> CheckedExpression {
//!     CheckedExpression {
//!         parameters: vec![],
//!         root: expression.root,
//!         slots: 0,
//!         effective_limits: CheckingLimits::default(),
//!     }
//! }
//! ```
//!
//! The checked call has no additional runtime authority arguments. The same
//! typed setup above compiles without the extra pair.
//!
//! ```compile_fail,E0061
//! use qsl_eval::value::{CheckedPackageEvaluation, QualifiedName};
//! use qsl_package::CheckedPackage;
//! use qsl_semantics::model::object_environment::ObjectEnvironment;
//! use quire_exact::Meter;
//! fn rejected(package: &CheckedPackage, name: &QualifiedName,
//!             objects: &ObjectEnvironment, meter: &mut Meter) {
//!     let authority = ("legacy", r#"{ "version": 1, "name": "orders" }"#);
//!     let _ = package.call(name, vec![], objects, meter, authority);
//! }
//! ```
//!
//! ```compile_fail,E0061
//! use qsl_eval::value::{CheckedPackageEvaluation, QualifiedName};
//! use qsl_package::CheckedPackage;
//! use qsl_semantics::model::object_environment::ObjectEnvironment;
//! use quire_exact::Meter;
//! fn rejected(package: &CheckedPackage, name: &QualifiedName,
//!             objects: &ObjectEnvironment, meter: &mut Meter) {
//!     let authority = ("legacy", r#"{ "objects": ["Order", "Line"], "id": "orders-model" }"#);
//!     let _ = package.call(name, vec![], objects, meter, authority);
//! }
//! ```
//!
//! ```compile_fail,E0061
//! use qsl_eval::value::{CheckedPackageEvaluation, QualifiedName};
//! use qsl_package::CheckedPackage;
//! use qsl_semantics::model::object_environment::ObjectEnvironment;
//! use quire_exact::Meter;
//! fn rejected(package: &CheckedPackage, name: &QualifiedName,
//!             objects: &ObjectEnvironment, meter: &mut Meter) {
//!     let authority = ("legacy", r#"{ "limits": {"depth": 8}, "profile": "strict" }"#);
//!     let _ = package.call(name, vec![], objects, meter, authority);
//! }
//! ```
//!
//! ```compile_fail,E0061
//! use qsl_eval::value::{CheckedPackageEvaluation, QualifiedName};
//! use qsl_package::CheckedPackage;
//! use qsl_semantics::model::object_environment::ObjectEnvironment;
//! use quire_exact::Meter;
//! fn rejected(package: &CheckedPackage, name: &QualifiedName,
//!             objects: &ObjectEnvironment, meter: &mut Meter) {
//!     let authority = ("legacy", r#"{ "enabled": true, "mode": "batch" }"#);
//!     let _ = package.call(name, vec![], objects, meter, authority);
//! }
//! ```
//!

pub mod automaton;
pub mod fairness;
pub mod hyper;
pub mod model_subject;
pub mod simulation;
pub mod smt;
pub mod temporal;
pub mod value;

#[cfg(test)]
pub(crate) mod test_ids;
