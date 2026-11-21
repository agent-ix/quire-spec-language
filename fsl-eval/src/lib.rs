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

//! This crate depends on layer 4 (`fsl-package`), layer 3 (`fsl-semantics`),
//! F (`fsl-foundation`) and K (`filament-exact`). It never names the root crate
//! or a SEAM module (the old `state` and `temporal` evaluators stay in the
//! root crate until M-6c deletes them): Cargo refuses the first, and the root
//! crate is the only home of the second. Every later layer depends on this
//! crate, never the other way.
//!
//! # Where the evaluation interface lives (FR-401-AC-3)
//!
//! The interface types are defined in `filament-eval` and `filament-semantic-value`,
//! and resolve at their new paths:
//!
//! ```
//! use filament_eval::{CallFailure, EvalOutcome, Evaluation, FamilyOutcome, FamilyResult};
//! use filament_semantic_value::call::WrongSnapshotCause;
//! use filament_semantic_value::catalog::{
//!     CatalogCode, CatalogCoded, Category, InternalFault, UndefinedCoded, UndefinedReason,
//!     UndefinedRecord,
//! };
//! use filament_semantic_value::supplied_stop::{
//!     SuppliedFailure, SuppliedStop, UnsupportedHelperStop,
//! };
//! ```
//!
//! No former path resolves from another crate:
//!
//! ```compile_fail,E0432
//! use fsl_eval::value::Evaluation;
//! ```
//!
//! ```compile_fail,E0432
//! use fsl_eval::value::CallFailure;
//! ```
//!
//! ```compile_fail,E0432
//! use fsl_semantics::family::FamilyOutcome;
//! ```
//!
//! ```compile_fail,E0432
//! use fsl_semantics::family::FamilyResult;
//! ```
//!
//! ```compile_fail,E0432
//! use fsl_semantics::family::EvalOutcome;
//! ```
//!
//! ```compile_fail,E0432
//! use fsl_foundation::diagnostic::InternalFault;
//! ```
//!
//! ```compile_fail,E0432
//! use fsl_foundation::InternalFault;
//! ```
//!
//! ```compile_fail,E0432
//! use fsl_semantics::value::supplied::SuppliedStop;
//! ```
//!
//! ```compile_fail,E0432
//! use fsl_semantics::value::supplied::SuppliedRun;
//! ```
//!
//! ```compile_fail,E0432
//! use fsl_semantics::model::refusal::render_identity;
//! ```
//!
//! ```compile_fail,E0432
//! use fsl_semantics::value::modulo;
//! ```
//!
//! ```compile_fail,E0432
//! use fsl_semantics::check::WrongSnapshotCause;
//! ```
//!

//! # Checked-term admission (TC-136)
//!
//! These public-API fixtures pair accepted typed handles with rejected raw
//! bytes and freely constructed wire records. Each rejected call has its own
//! fixture so an earlier type error cannot hide a later admission attempt.
//!
//! ```no_run
//! use fsl_eval::value::{CheckedPackageEvaluation, QualifiedName};
//! use fsl_package::CheckedPackage;
//! use fsl_semantics::{check::CheckedExpression, model::object_environment::ObjectEnvironment};
//! use filament_exact::Meter;
//! fn accepted(package: &CheckedPackage, name: &QualifiedName, expression: &CheckedExpression,
//!             objects: &ObjectEnvironment, meter: &mut Meter) {
//!     let _ = package.call(name, vec![], objects, meter);
//!     let _ = package.evaluate(expression, vec![], objects, meter);
//! }
//! ```
//!
//! ```compile_fail,E0308
//! use fsl_eval::value::CheckedPackageEvaluation;
//! use fsl_package::CheckedPackage;
//! use fsl_semantics::model::object_environment::ObjectEnvironment;
//! use filament_exact::Meter;
//! struct WireExpression { name: &'static str }
//! fn rejected(package: &CheckedPackage, objects: &ObjectEnvironment, meter: &mut Meter) {
//!     let _ = package.call(b"unchecked".as_slice(), vec![], objects, meter);
//! }
//! ```
//!
//! # Canonical checked-node descriptors (FR-400-AC-12)
//!
//! The canonical operation vocabulary and borrowed node descriptor are
//! available from `filament-checked`:
//!
//! ```no_run
//! use filament_checked::{NodeKind, NodeRead};
//! use filament_exact::Value;
//! use filament_semantic_value::location::{ExpressionLocation as Location, ExpressionOwner as Origin};
//! use filament_exact::ValueType;
//! fn canonical(value_type: &ValueType, location: &Location) {
//!     let operation: NodeKind<usize, &Value> = NodeKind::Local(0);
//!     let _descriptor = NodeRead {
//!         operation,
//!         value_type,
//!         location,
//!     };
//! }
//! let _ = canonical;
//! let _ = (Location::root(Origin::Expression), ValueType::Boolean);
//! ```
//!
//! A descriptor does not admit an expression to the public evaluator:
//!
//! ```compile_fail,E0308
//! use fsl_eval::value::CheckedPackageEvaluation;
//! use fsl_package::CheckedPackage;
//! use fsl_semantics::model::object_environment::ObjectEnvironment;
//! use filament_checked::{NodeKind, NodeRead};
//! use filament_exact::{Meter, Value, ValueType};
//! use filament_semantic_value::location::{ExpressionLocation as Location, ExpressionOwner as Origin};
//! fn rejected(package: &CheckedPackage, objects: &ObjectEnvironment, meter: &mut Meter) {
//!     let value_type = ValueType::Boolean;
//!     let location = Location::root(Origin::Expression);
//!     let descriptor = NodeRead {
//!         operation: NodeKind::<usize, &Value>::Local(0),
//!         value_type: &value_type,
//!         location: &location,
//!     };
//!     let _ = package.evaluate(&descriptor, vec![], objects, meter);
//! }
//! ```
//!
//! The operation alone is rejected for the same reason:
//!
//! ```compile_fail,E0308
//! use fsl_eval::value::CheckedPackageEvaluation;
//! use fsl_package::CheckedPackage;
//! use fsl_semantics::model::object_environment::ObjectEnvironment;
//! use filament_checked::NodeKind;
//! use filament_exact::{Meter, Value};
//! fn rejected(package: &CheckedPackage, objects: &ObjectEnvironment, meter: &mut Meter) {
//!     let operation = NodeKind::<usize, &Value>::Local(0);
//!     let _ = package.evaluate(&operation, vec![], objects, meter);
//! }
//! ```
//!
//! ```compile_fail,E0308
//! use fsl_eval::value::CheckedPackageEvaluation;
//! use fsl_package::CheckedPackage;
//! use fsl_semantics::model::object_environment::ObjectEnvironment;
//! use filament_exact::Meter;
//! struct WireExpression { name: &'static str }
//! fn rejected(package: &CheckedPackage, objects: &ObjectEnvironment, meter: &mut Meter) {
//!     let _ = package.call(&WireExpression { name: "unchecked" }, vec![], objects, meter);
//! }
//! ```
//!
//! ```compile_fail,E0308
//! use fsl_eval::value::CheckedPackageEvaluation;
//! use fsl_package::CheckedPackage;
//! use fsl_semantics::model::object_environment::ObjectEnvironment;
//! use filament_exact::Meter;
//! struct WireExpression { name: &'static str }
//! fn rejected(package: &CheckedPackage, objects: &ObjectEnvironment, meter: &mut Meter) {
//!     let _ = package.evaluate(b"unchecked".as_slice(), vec![], objects, meter);
//! }
//! ```
//!
//! ```compile_fail,E0308
//! use fsl_eval::value::CheckedPackageEvaluation;
//! use fsl_package::CheckedPackage;
//! use fsl_semantics::model::object_environment::ObjectEnvironment;
//! use filament_exact::Meter;
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
//! use fsl_semantics::check::CheckedExpression;
//! use filament_semantic_value::checking::CheckingLimits;
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
//! use fsl_eval::value::{CheckedPackageEvaluation, QualifiedName};
//! use fsl_package::CheckedPackage;
//! use fsl_semantics::model::object_environment::ObjectEnvironment;
//! use filament_exact::Meter;
//! fn rejected(package: &CheckedPackage, name: &QualifiedName,
//!             objects: &ObjectEnvironment, meter: &mut Meter) {
//!     let authority = ("legacy", r#"{ "version": 1, "name": "orders" }"#);
//!     let _ = package.call(name, vec![], objects, meter, authority);
//! }
//! ```
//!
//! ```compile_fail,E0061
//! use fsl_eval::value::{CheckedPackageEvaluation, QualifiedName};
//! use fsl_package::CheckedPackage;
//! use fsl_semantics::model::object_environment::ObjectEnvironment;
//! use filament_exact::Meter;
//! fn rejected(package: &CheckedPackage, name: &QualifiedName,
//!             objects: &ObjectEnvironment, meter: &mut Meter) {
//!     let authority = ("legacy", r#"{ "objects": ["Order", "Line"], "id": "orders-model" }"#);
//!     let _ = package.call(name, vec![], objects, meter, authority);
//! }
//! ```
//!
//! ```compile_fail,E0061
//! use fsl_eval::value::{CheckedPackageEvaluation, QualifiedName};
//! use fsl_package::CheckedPackage;
//! use fsl_semantics::model::object_environment::ObjectEnvironment;
//! use filament_exact::Meter;
//! fn rejected(package: &CheckedPackage, name: &QualifiedName,
//!             objects: &ObjectEnvironment, meter: &mut Meter) {
//!     let authority = ("legacy", r#"{ "limits": {"depth": 8}, "profile": "strict" }"#);
//!     let _ = package.call(name, vec![], objects, meter, authority);
//! }
//! ```
//!
//! ```compile_fail,E0061
//! use fsl_eval::value::{CheckedPackageEvaluation, QualifiedName};
//! use fsl_package::CheckedPackage;
//! use fsl_semantics::model::object_environment::ObjectEnvironment;
//! use filament_exact::Meter;
//! fn rejected(package: &CheckedPackage, name: &QualifiedName,
//!             objects: &ObjectEnvironment, meter: &mut Meter) {
//!     let authority = ("legacy", r#"{ "enabled": true, "mode": "batch" }"#);
//!     let _ = package.call(name, vec![], objects, meter, authority);
//! }
//! ```
//!

/// FR-286: the typed refusal pairs produced by evaluation.
pub const CATALOG_PAIRS: &[fsl_foundation::diagnostic::CatalogCode] = &[
    fsl_foundation::diagnostic::CatalogCode::new("cancelled", "caller-cancelled"),
    fsl_foundation::diagnostic::CatalogCode::new("cardinality_out_of_bound", "above-maximum"),
    fsl_foundation::diagnostic::CatalogCode::new(
        "dangling_reference",
        "absent-target-in-complete-population",
    ),
    fsl_foundation::diagnostic::CatalogCode::new("foreign_reference", "foreign-universe"),
    fsl_foundation::diagnostic::CatalogCode::new("incomplete_population", "incomplete-scope"),
    fsl_foundation::diagnostic::CatalogCode::new("invalid_runtime_input", "invalid-value"),
    fsl_foundation::diagnostic::CatalogCode::new("invalid_runtime_input", "wrong-value-kind"),
    fsl_foundation::diagnostic::CatalogCode::new("undefined_expression", "unproved-nonzero"),
    fsl_foundation::diagnostic::CatalogCode::new("undefined_expression", "unproved-presence"),
    fsl_foundation::diagnostic::CatalogCode::new("undefined_expression", "unproved-range"),
    fsl_foundation::diagnostic::CatalogCode::new(
        "unsupported_construct",
        "collection-write-not-computed",
    ),
    fsl_foundation::diagnostic::CatalogCode::new("unsupported_construct", "expression-form"),
    fsl_foundation::diagnostic::CatalogCode::new("unsupported_construct", "not-yet-implemented"),
    fsl_foundation::diagnostic::CatalogCode::new("stage_limit_exceeded", "work-budget-exceeded"),
    fsl_foundation::diagnostic::CatalogCode::new("missing_declaration", "missing-name"),
    fsl_foundation::diagnostic::CatalogCode::new("resource_exhausted", "allocation-failed"),
    fsl_foundation::diagnostic::CatalogCode::new("resource_exhausted", "insufficient-next-charge"),
    fsl_foundation::diagnostic::CatalogCode::new(
        "runtime_invariant",
        "established-invariant-broken",
    ),
    fsl_foundation::diagnostic::CatalogCode::new("wrong_snapshot", "forbidden-pre-read"),
    fsl_foundation::diagnostic::CatalogCode::new("wrong_snapshot", "wrong-anchor"),
];

pub mod automaton;
pub mod bare;
pub mod completeness;
pub mod execution;
pub mod fairness;
pub mod hyper;
pub mod model_subject;
pub mod probabilistic;
pub mod simulation;
pub mod smt;
pub mod temporal;
pub mod value;

#[cfg(test)]
pub(crate) mod test_ids;
