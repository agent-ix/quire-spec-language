// SPDX-License-Identifier: AGPL-3.0-or-later
#![cfg_attr(not(feature = "std"), no_std)]
//! ADR-011 §6.1 layer 4, `package` (ADR-011 §7.3 X-7): the S4
//! in-process [`CheckedPackage`]/[`EmittedPackage`] typestate (ADR-013 T-1,
//! FR-087, the `checked` module), the
//! `filament.checked-package/v2` I2 byte reader (`checked_v2`, ADR-011 §4) and
//! the v2 emitter (`emit`, ADR-011 T-8, M-4).
//!
//! This crate depends on layer 3 (`fsl-semantics`), F (`fsl-foundation`), K
//! (`filament-exact`) and `filament-contract-model` for the v2 wire vocabulary. It never names the root crate or a
//! SEAM-1 module (the root crate's `checking` or `linking`): Cargo refuses the
//! first, and the root crate is the only home of the second. The layer-5 evaluator and every later layer
//! depend on this crate, never the other way.

// NFR-015: with `std` disabled the crate is `no_std` + `alloc` and keeps the
// in-memory checked-IR path; the `filament.checked-package/v2` emitter and reader
// (`emit`, `checked_v2`) and the dependency-bearing `link_with` need
// `filament-contract-model` and are behind `std`.
extern crate alloc;

/// FR-286: the typed refusal pairs produced by package admission and emission.
pub const CATALOG_PAIRS: &[fsl_foundation::diagnostic::CatalogCode] = &[
    fsl_foundation::diagnostic::CatalogCode::new("unknown_wire", "unsupported-wire"),
    fsl_foundation::diagnostic::CatalogCode::new("unknown_required_feature", "unsupported-feature"),
    fsl_foundation::diagnostic::CatalogCode::new("invalid_model_binding", "wrong-model-selection"),
];

#[cfg(feature = "std")]
mod analysis_view;
#[cfg(feature = "std")]
pub use analysis_view::{AnalysisView, ClaimOccurrence, DependencyClosure};
#[cfg(feature = "std")]
pub use model_input_form::UnreadModelForm;
#[cfg(feature = "std")]
mod bare;
#[cfg(feature = "std")]
mod bare_slot;
#[cfg(feature = "std")]
mod bare_state_graph;
#[cfg(feature = "std")]
mod bare_synthesis;
#[cfg(feature = "std")]
mod bare_term;
#[cfg(feature = "std")]
mod bare_terminal;
#[cfg(feature = "std")]
mod bare_units;
#[cfg(feature = "std")]
mod bare_world;
#[cfg(feature = "std")]
pub use bare_synthesis::{is_plan_synthesis, BareSynthesis, BareSynthesisObjective};
mod checked;
#[cfg(feature = "std")]
mod checked_v2;
#[cfg(feature = "std")]
pub mod checked_views;
#[cfg(feature = "std")]
#[cfg(feature = "std")]
mod emit;
#[cfg(feature = "std")]
mod equality_file;
pub mod evaluation_view;
mod export;
#[cfg(feature = "std")]
mod introduced_binders;
#[cfg(feature = "std")]
mod model_input_form;
#[cfg(feature = "std")]
mod native_row;
mod obligation_identity;
#[cfg(feature = "std")]
mod probabilistic_read;
#[cfg(feature = "std")]
mod source_model_read;
#[cfg(feature = "std")]
pub use probabilistic_read::ProbabilisticReadFault;
#[cfg(feature = "std")]
pub use source_model_read::{SourceModel, SourceModelFault, UnreadSourceParameter};
#[cfg(feature = "std")]
pub mod number;
#[cfg(feature = "std")]
mod protocol_activation;
pub mod protocol_artifact_v1;
#[cfg(feature = "std")]
pub mod protocol_artifact_v2;
#[cfg(feature = "std")]
mod protocol_occurrence;
#[cfg(feature = "std")]
mod protocol_occurrence_key;
mod protocol_selection;
#[cfg(feature = "std")]
pub mod protocol_view;
#[cfg(feature = "std")]
mod reader_protocol_projection;
#[cfg(feature = "std")]
pub mod verification_plan;
mod wire_form;

#[cfg(feature = "std")]
pub use bare::{
    BarePackage, BareSupply, IdentityFileRefusal, IdentityJoinFailure, IdentityMember,
    IdentityWitnessRefusal, ReadIdentityRows, IDENTITIES_FORMAT,
};
#[cfg(feature = "std")]
pub use bare_slot::{BareSelectionFailure, BareSlot, BareSource};
#[cfg(feature = "std")]
#[cfg(feature = "std")]
pub use bare_state_graph::BareStatePredicate;
#[cfg(feature = "std")]
pub use bare_term::{
    BareClauseRefusal, BareClauseTerm, BareFrameOperation, BareStateClause,
    BareStateClauseDeclaration, ClauseBinder, DeclaredComposite, DeclaredShape,
};
#[cfg(feature = "std")]
pub use bare_terminal::{BareTerminal, BareTerminalDeclaration, BareTerminalPredicate};
#[cfg(feature = "std")]
pub use bare_world::BareWorld;
pub use checked::{CheckedPackage, ResolvedDependency};
#[cfg(feature = "std")]
pub use checked::{EmitLimits, EmittedPackage, Import, LinkRefusal};
#[cfg(feature = "std")]
pub use checked_v2::{
    checked_package_refusal_code, clause_roots, hyper_roots, occurrence_role,
    read_evaluation_emission, read_evaluation_package, read_hybrids, read_import_view,
    read_terminal, read_terminals, resolve_clause_root, resolve_hyper_root,
    resolve_schedulability_root, resolve_state_graph_root, resolve_terminals, schedulability_roots,
    state_graph_roots, supply_closure, AdmittedPackageRead, AdmittedPackages,
    CheckedClockConstraint, CheckedClockGuard, CheckedClockPlace, CheckedClockReset,
    CheckedConstraintTarget, CheckedGuardPart, CheckedResetTarget, CheckedTimeInvariant,
    CheckedTimeSource, CheckedTimeSourceKind, CheckedTimedModel, CheckedUrgency, ClauseRoot,
    ClauseRootUnresolved, ClockComparison, ClockPathStep, EvaluationWitnessSupplier,
    EvaluationWitnesses, HybridRead, HybridReadError, HyperRoot, HyperRootUnresolved,
    ImportViewRefusal, NativeJoinLimits, PackageTerminal, PackageTerminalDeclaration,
    ReadHybridClaim, ReadHybridModel, ReadTerminal, ReadTerminalDeclaration, ResolvedClockPathStep,
    SchedulabilityRoot, SourceMapDefect, SourceWitnessSupplier, StateGraphKind, StateGraphRoot,
    TerminalResolveRefusal, TimedLiteralTypes, TimedNodeMetadata, UnsupportedWire, V2ReadLimits,
    V2ReadRefusal, WitnessSupplyFailure,
};
#[cfg(feature = "std")]
pub use emit::write_operand_slots;
#[cfg(feature = "std")]
pub use emit::{
    checked_occurrence, clock_configuration_invalid_field, clock_kind,
    declaration_less_function_id, emit_checked, emit_checked_with_cancel,
    emit_checked_with_protocol_results,
    emit_checked_with_protocol_results_and_temporal_selections_and_cancel,
    emit_checked_with_temporal_selections, emit_checked_with_temporal_selections_and_cancel,
    emit_dependency_with_temporal_selections_and_cancel, emit_monitor_plan_over, emit_package,
    emit_package_with_temporal_selections, emit_timed_models, hyper_clock_name, hyper_wrapper,
    is_model_binding_name, profile_clock_kind, ClockSelectionCause, ClockSelectionDeclaration,
    ClockSelectionRefusal, Emission, EmitRefusal, MonitorPlanWire, OmissionCause, OmittedNode,
    ProtocolEmission, ProtocolEmissionAggregate, ProtocolEmissionResult,
    SelectedSourceCollectionRow, SelectedSourceRecordMemberRow, SelectedSourceRecordRow,
    SourceObjectMemberFacts, SourceObjectOperationFacts, TemporalBindingRows, TemporalClauseRef,
    TemporalClockSelection, TemporalRows, TimedOrderFault,
};
#[cfg(feature = "std")]
pub use equality_file::{
    EqualityFileRefusal, EqualityWitnessRefusal, ReadEqualityRows, EQUALITY_FORMAT,
};
#[cfg(feature = "std")]
pub use evaluation_view::NativeJoinLimitError;
pub use evaluation_view::{
    AdmittedEvaluationView, AdmittedProtocolObservation, ProtocolSource, SelectedProtocolTerm,
    TermHandle, ViewFailure,
};
pub use export::{export_bindings, BoundItem, ExportRefusal, UnboundElement};
#[cfg(feature = "std")]
pub use filament_contract_model::{CheckedBindingNameKind, CheckedLiteralValue, CheckedTerm};
#[cfg(feature = "std")]
pub use native_row::{NativeRowError, NativeRowErrorCode};
pub use obligation_identity::{ModelIdentity, ObligationIdentity};
#[cfg(feature = "std")]
pub use protocol_occurrence::{
    protocol_occurrence_schemas_from_read, CompensationEntry, ControlEntry, ControlOperator,
    ControlOwner, OccurrenceRefusal, PopulationName, ProtocolOccurrence, ProtocolOccurrenceSchema,
    RepeatDimension, RoleLifetime, RoleModelMember, RoleReplication, RoleSlot,
    RuntimeBindingRequirement, SubjectKind,
};
#[cfg(feature = "std")]
pub use protocol_occurrence_key::{
    OccurrenceKeyError, ProtocolOccurrenceKey, WorkflowInstanceIdentity,
};
pub use protocol_selection::{ProtocolSlotDescriptor, SlotSelectionFailure};
#[cfg(feature = "std")]
pub use reader_protocol_projection::{
    MissingLocation, ReaderProtocolFunction, ReaderProtocolParameter, ReaderProtocolProjection,
    ReaderSlot, UnsupportedReason, UnsupportedView, ViewLocus,
};
#[cfg(feature = "std")]
pub use verification_plan::{
    emit_verification_plans, read_verification_plan, VerificationPlan, VerificationPlanClaim,
    VerificationPlanDeclarationKey, VerificationPlanEdge, VerificationPlanIdentity,
    VerificationPlanMethod, VerificationPlanNode, VerificationPlanNodeIdentity,
    VerificationPlanPathSegment, VerificationPlanRefusal, VerificationPlanSource,
    VerificationPlanSourceLocation, VERIFICATION_PLAN_FORMAT,
};

#[cfg(feature = "std")]
pub use filament_contract_model::{
    CheckedArtifactRef, CheckedIntervalEnd, CheckedNodeId, CheckedOccurrence,
    CheckedOccurrenceRole, CheckedOperationInterval, CheckedOperationRational, CheckedPackageV2,
    CheckedSelection, CheckedSourceMapEntry, ClockConfiguration, ClockExactNumber,
    ExactTypeProjection, ExactTypeProjectionError, ExactTypeProjectionErrorCode, NativeJoinBudget,
    SelectedExactIdentities, TemporalBinding, CHECKED_PACKAGE_V2,
};

// The FR-105-AC-6 fault-injection seam. Public only under
// `test-support` (see `emit.rs`'s doc comments and this crate's
// `Cargo.toml`), so a dependent crate's own tests can reach it while it
// compiles into no shipped build.
#[cfg(all(feature = "std", feature = "test-support"))]
pub use emit::{checked_node_id_of, emit_checked_with_fault};

// ADR-011 §4 I2: `read_checked_package_v2` and its staged outcome remain
// crate-local. The public `read_import_view` returns a structural import
// view after Contract IR admits the package bytes.

/// NFR-015-AC-5, TC-1111: with `std` disabled, the exports of the `filament.checked-package/v2`
/// wire do not resolve. One case per export.
///
/// ```compile_fail,E0599
/// fn named(graph: fsl_semantics::check::CheckedGraph) {
///     let _ = fsl_package::CheckedPackage::link_with(graph, Vec::new());
/// }
/// ```
///
/// ```compile_fail,E0599
/// fn named(package: &fsl_package::CheckedPackage) {
///     let _ = package.emitted_package_name();
/// }
/// ```
///
/// ```compile_fail,E0432
/// use fsl_package::Import;
/// ```
///
/// ```compile_fail,E0432
/// use fsl_package::LinkRefusal;
/// ```
///
/// ```compile_fail,E0432
/// use fsl_package::EmittedPackage;
/// ```
///
/// ```compile_fail,E0432
/// use fsl_package::emit_checked;
/// ```
///
/// ```compile_fail,E0432
/// use fsl_package::emit_checked_with_cancel;
/// ```
///
/// ```compile_fail,E0432
/// use fsl_package::emit_package;
/// ```
///
/// ```compile_fail,E0432
/// use fsl_package::checked_node_id;
/// ```
///
/// ```compile_fail,E0432
/// use fsl_package::checked_occurrence;
/// ```
///
/// ```compile_fail,E0432
/// use fsl_package::Emission;
/// ```
///
/// ```compile_fail,E0432
/// use fsl_package::EmitRefusal;
/// ```
///
/// ```compile_fail,E0432
/// use fsl_package::OmissionCause;
/// ```
///
/// ```compile_fail,E0432
/// use fsl_package::OmittedNode;
/// ```
///
/// ```compile_fail,E0432
/// use fsl_package::read_import_view;
/// ```
///
/// ```compile_fail,E0432
/// use fsl_package::supply_closure;
/// ```
///
/// ```compile_fail,E0432
/// use fsl_package::clause_roots;
/// ```
///
/// ```compile_fail,E0432
/// use fsl_package::resolve_clause_root;
/// ```
///
/// ```compile_fail,E0432
/// use fsl_package::hyper_roots;
/// ```
///
/// ```compile_fail,E0432
/// use fsl_package::resolve_hyper_root;
/// ```
///
/// ```compile_fail,E0432
/// use fsl_package::HyperRoot;
/// ```
///
/// ```compile_fail,E0432
/// use fsl_package::HyperRootUnresolved;
/// ```
///
/// ```compile_fail,E0432
/// use fsl_package::state_graph_roots;
/// ```
///
/// ```compile_fail,E0432
/// use fsl_package::resolve_state_graph_root;
/// ```
///
/// ```compile_fail,E0432
/// use fsl_package::StateGraphRoot;
/// ```
///
/// ```compile_fail,E0432
/// use fsl_package::StateGraphKind;
/// ```
///
/// ```compile_fail,E0432
/// use fsl_package::AdmittedPackages;
/// ```
///
/// ```compile_fail,E0432
/// use fsl_package::ClauseRoot;
/// ```
///
/// ```compile_fail,E0432
/// use fsl_package::ClauseRootUnresolved;
/// ```
///
/// ```compile_fail,E0432
/// use fsl_package::ImportViewRefusal;
/// ```
///
/// ```compile_fail,E0432
/// use fsl_package::SourceMapDefect;
/// ```
///
/// ```compile_fail,E0432
/// use fsl_package::UnsupportedWire;
/// ```
///
/// ```compile_fail,E0432
/// use fsl_package::V2ReadLimits;
/// ```
///
/// ```compile_fail,E0432
/// use fsl_package::V2ReadRefusal;
/// ```
///
#[cfg(not(feature = "std"))]
mod behind_std {}

#[cfg(test)]
const TEST_PACKAGE_NAME: &str = "pkg";

#[cfg(feature = "std")]
mod dependency_artifact;
#[cfg(feature = "std")]
mod package_admission;
#[cfg(feature = "std")]
pub use dependency_artifact::DependencyArtifact;
#[cfg(feature = "std")]
pub use package_admission::{
    admit_checked_package, admit_checked_package_with_budget,
    admit_checked_package_with_dependencies, PackageAdmissionRefusal,
};
#[cfg(feature = "std")]
mod native_join_failure;
#[cfg(feature = "std")]
pub use native_join_failure::NativeJoinFailure;
