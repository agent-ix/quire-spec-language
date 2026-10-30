// SPDX-License-Identifier: AGPL-3.0-or-later
//! `qsl-replay`: the ADR-011 §6.1 layer **6** crate (ADR-011 §7.3
//! X-10) -- the CG-facing replay facade (ADR-011 §6.1, ADR-013 O-24 to
//! O-27, C-13). It depends on layers 1 to 5, F and K, as §6.1 allows.
//!
//! Its public API is the compile entry [`compile_package`] and the replay executor entry [`replay`] (ADR-013 TK-01,
//! C-13; FR-098), which recompiles a request's digest-addressed source
//! through the spine ([`spine::parse`], [`spine::select`], [`spine::check`]
//! and [`spine::package`], S1 to E4) and calls the selected
//! function through S6a (`CheckedPackageEvaluation::call`), and the four
//! typed envelopes it reads and settles -- the proof-result envelope
//! (`proof_result`), the counterexample/witness envelope (`witness`), the
//! replay request (`request`) and the replay result (`result`) -- with
//! their round trips and their redacted rendering (FR-069 through FR-073).
//! It builds no backend invocation and no Kani harness. CG reaches this
//! crate's public API and nothing else in QSL (ADR-011 FB-05); `command`
//! composes those four operations for the CLI's `compile`.
//!
//! # Identity and provenance types
//!
//! O-07's occurrence key and O-12's source region are
//! `qsl_foundation::source::provenance`'s (#213 S-4), and this
//! crate re-exports them for CG. O-09's obligation identity is this crate's
//! own, the canonical type ADR-013 OQ-H assigns here, which CG names
//! directly; IR names no QSL type. O-11's `QualifiedName` is still `identity`'s own minimal
//! version, with the shape ADR-013 specifies; `identity` names the ticket
//! that absorbs it.
//!
//! # Former paths (FR-401-AC-3)
//!
//! The ADR-013 O-16 `Category` a `Verdict` carries is
//! `quire_semantic_value::catalog::Category`. This crate no longer re-exports
//! it:
//!
//! ```
//! use quire_semantic_value::catalog::Category;
//! # let _ = Category::Success;
//! ```
//!
//! ```compile_fail,E0432
//! use qsl_replay::Category;
//! ```

#![forbid(unsafe_code)]

mod admitted;
mod application;
mod bounds;
mod call_replay_context;
mod call_requirements;
mod call_site;
mod certificate;
mod clause_evaluation;
mod compile;
mod composite;
pub mod contract_check;
mod execute;
mod hybrid;
mod hybrid_replay;
mod id_text;
mod identity;
mod limits;
mod model_check;
mod monitor_plan;
mod object_environment;
mod outcome;
mod proof_result;
mod refinement;
mod refinement_query;
mod request;
mod result;
mod scalar;
pub mod spine;
pub mod statistical;
mod supplied_limits;
mod target;
mod timed;
pub mod value_check;
mod witness;

pub use admitted::{
    compile_admitted, AdmittedCompilation, AdmittedComposite, AdmittedCompositeForm,
    AdmittedContext, AdmittedDimension, AdmittedEnum, AdmittedEnumCase, AdmittedLookupRefusal,
    AdmittedRefusal, AdmittedResource, AdmittedUnionMember, AdmittedUnit,
};
pub use application::{
    acquire_checked_closure, ApplicationBeginRefusal, ApplicationEnvironment, ApplicationRegistry,
    CallableLookupRefusal, CallableSelection, CheckedApplicationProgress,
    CheckedApplicationSession, CheckedClosureCompilation, CheckedClosureHandle,
    NestedCallOccurrence, NestedCallSite, NestedCallSiteRefusal, NominalDeclaration,
    NominalDeclarationForm, NominalDeclarationMember, NominalDeclarationRefusal, SelectionRefusal,
};
pub use bounds::{BoundExceeded, ReplayLimits, DEFAULT_REPLAY_INPUT_BYTES};
pub use call_replay_context::{run_with_replay_context, CallReplayContext, RunWithReplayContext};
pub use call_requirements::{
    run_with_requirements, CallRequirement, CallRequirements, RunWithRequirements,
};
pub use call_site::{
    call_site, CallSite, CallSiteRefusal, CallSiteSelection, ClauseName, ClauseSite, FieldName,
    FieldSite, FunctionSite, OperationSite, PopulationName, PopulationSite, ValueParitySite,
};
pub use certificate::closed_form::outcome::{
    AmcEvidence, ClosedFormAnalysis, ClosedFormEvidence, ClosedFormOutcome, DemandWitness,
    HighModeMiss, JobIterates, MissEvidence, ScheduleLimit, TaskFixpoint,
};
pub use certificate::closed_form::recompute::{recompute, ClosedFormCheck, ClosedFormDisagreement};
pub use certificate::closed_form::settle::settle;
pub use certificate::closed_form::task_set::{
    Arrival, Criticality, ScheduleClaim, ScheduleSubject, SchedulingPolicy, Task, TaskSet,
    TaskSetRefusal, TimeMember, Wcet,
};
pub use certificate::closure::{
    check_closure, check_components, check_protocol_closure, check_protocol_components,
    CertificateRejection, CertificateRequest, ClosureCertificate, ClosureRefusal, ProductStateRef,
    ProtocolCertificateRequest, TemporalCertificateCheck,
};
pub use certificate::scc::{Component, ComponentCertificate, ComponentWitness, ProofCertificate};
pub use certificate::simulation::{
    check_simulation_certificate, CertifiedEdge, CertifiedPosition, EvaluationOutcome, FailedStep,
    IncompleteLimit, SimulationCertificate, SimulationCertificateRequest, SimulationRefusal,
    UndeterminedStep,
};
pub use certificate::smt::{
    check_refinement_smt_proof, ProofFormat, RefinementSmtCertificate, RefutedQuery, SmtProofCheck,
};
pub use certificate::state_graph::{
    check_state_graph, StateGraphCertificate, StateGraphCertificateRequest,
};
pub use certificate::zone::{LuBounds, Zone, ZoneError};
pub use certificate::zone_certificate::{
    check_zone_certificate, AcceptanceSet, CertEdge, CertNode, CertificateCheckLimits,
    CertificateItem, CertifiedItem, CheckerFault, ComponentReason, Components, DivergenceWitness,
    DivergentTarget, FairnessMember, ItemIdentity, ProductMark, RefinementIdentity, RefinementPart,
    SilentMove, ZoneCertificate, ZoneCertificateRefusal, ZoneCertificateRequest,
    ZoneCertificateVerdict,
};
pub use certificate::{CertificateLocus, CertificateMember, CertificateRule, QueryPart};
// FR-491: the state-clause evaluation entry and its typed result, at the root
// so CG names them without naming `spine` (ADR-011 FB-05, T-12 rule (a)).
pub use clause_evaluation::{
    evaluate_clause, ClauseEvaluation, ClauseEvaluationRequest, ClauseIncomplete,
    ClauseObservation, ClauseRefusal,
};
pub use compile::{
    compile_package, compile_package_with_execution_points, CompileUnit, CompiledPackage,
};
pub use composite::{
    CompositeEvidence, CompositeIdentity, CompositeParityClaim, CompositeParityReport,
    CompositeParityResult, EqualityOutcome, FalsifiedParity, IncompleteStage, NativeCause,
    NativeParityObservation, Refinement, VerifiedShadow, VerifiedShadowReport,
    VerifiedShadowResult,
};
pub use execute::{
    parity_obligation, replay, replay_composite_parity, replay_frame, replay_operator_parity,
    replay_value_parity, replay_with_cancel, settle_verified_shadow, value_parity_obligation,
    BoundEntries, DependencySelectionsCause, Domain, FrameIdentityMismatch, FrameReplayResult,
    IdentityEncodeError, LimitAboveReader, MalformedOperationReason, ParityArgument,
    ParityBoundRefusal, ParityPreimage, ReplayRefusal, ScalarIdentityMismatch, ValueParityArgument,
    ValueParityPreimage, ValueParityReport, ValueParityResult,
};
pub use hybrid::{
    settle_hybrid_candidates, AdmittedHybridProcessItem, AdmittedHybridProcessRequest,
    BoundDimension, BudgetRefusal, ClaimHorizon, HybridProcessDependencyPackage, HybridProcessItem,
    HybridProcessPackageContext, HybridProcessRequest, HybridProcessRequestRefusal,
    HybridSettlement, NegativeHorizon, SolverBudget, SolverCoverage, SolverHorizon, SolverMethod,
    SolverOutcome, SolverResult, SolverStop,
};
pub use hybrid_replay::{
    admit as admit_hybrid_trace, exact_replay, static_replay, ExactCheck, ExactGap, ExactInterval,
    ExactReal, ExactRecord, ExactReplay, ExpansionGap, ExpressionLocus, ExpressionSite,
    HybridCounterexample, HybridCounterexampleKind, HybridReplayMeter, HybridReplayRefusal,
    HybridTrace, JsonPointer, ReplayRecord, Root, StaticCheck, StaticReplay, TermLocus,
    TraceBefore, TraceFault, TraceJump, TraceRefusal, TraceState, UndefinedCause, UndefinedInstant,
};
pub use identity::{
    DeclaredDomain, EmptyQualifiedName, ModelIdentity, ObligationIdentity, ProfileSelection,
    QualifiedName, RawSourceRef, TracePosition,
};
pub use limits::{AccountingLimits, CallerLimits, EngineLimits, PrepareLimits, RenderLimits};
pub use model_check::counterexample::{
    undefined_finding, ClaimPredicate, CounterexampleKind, Lasso, MappingSite, ModelStep,
    ObservedTemporalCounterexample, ObservedTemporalKind, ObservedTemporalStep, Race,
    TemporalCounterexample, TimedPoint, UndefinedEvaluation, UndefinedLocus, UndefinedSite,
};
pub use model_check::existential_outcome::{ExistentialInstanceOutcome, ExistentialOutcome};
pub use model_check::graph_replay::{replay_model_graph, replay_model_graph_with_cancel};
pub use model_check::graph_settle::{
    settle_state_graph, GraphReplayResult, GraphReplaySettlement, StateGraphMethod,
    StateGraphRecord,
};
pub use model_check::hyper_counterexample::{
    HyperCounterexample, HyperCounterexampleKind, HyperLocus, HyperStep, HyperTrace,
    HyperUndefined, MatchWitness, MatchWitnessCheck, StepMark,
};
pub use model_check::hyper_outcome::{BoundCovers, HyperOutcome, HyperReductionCause};
pub use model_check::hyper_replay::{check_match_witness, replay_hyper_trace_tuple, HyperReplay};
pub use model_check::hyper_settle::{
    settle_hyper_check, settle_hyper_form, settle_hyper_smt, HyperRecord, HyperSettleRefusal,
    LimitOrigin, ProductMember, UsedLimit,
};
pub use model_check::hyper_smt_outcome::{HyperSmtOutcome, HyperSmtResult, SmtMethod};
pub use model_check::hyper_smt_query::{
    encode_hyper_query, HyperProductIdentity, HyperQuery, HyperQueryRequest,
};
pub use model_check::identity::{
    node_occurrence, ClauseIdentity, ClauseRootRefusal, StateGraphIdentity,
};
pub use model_check::item::{
    Instance, ItemRefusal, LivenessTranslation, ModelCheckItem, Translation,
};
pub use model_check::limits::{
    meter_bound, ModelCheckLimit, ModelCheckLimits, ReachedLimit, DEFAULT_MAX_AUTOMATON_STATES,
    DEFAULT_MAX_RELATION_TUPLES, DEFAULT_MAX_WALK_STEPS, DEFAULT_MAX_WITNESS_SET,
    DEFAULT_WITNESS_SAMPLES, DEFAULT_WITNESS_SEED,
};
pub use model_check::outcome::{
    MemoryCause, ModelCheckOutcome, ReductionCause, RunStats, UndecidedCause,
};
pub use model_check::refinement_replay::{
    replay_refinement_trace, replay_timed_refinement_trace, RefinementTraceSettlement,
};
pub use model_check::refinement_settle::{
    settle_refinement, settle_refinement_refusal, settle_refinement_safety, RefinementRecord,
};
#[cfg(feature = "test-support")]
#[doc(hidden)]
pub use model_check::replay::replay_timed_counterexample;
pub use model_check::replay::{
    replay_model_trace, replay_protocol_trace, DelayBar, ModelTraceRefusal, ModelTraceReplayResult,
    ModelTraceSettlement, ModelTraceSubject, ModelTraceValue, ProtocolTraceSubject,
    StrongFairnessHint, TimedReplay,
};
pub use model_check::settle::{
    settle_model_check, settle_protocol_check, Execution, ModelCheckRecord, RecordWarning,
    ResultLabel, Truth, EXPLICIT_STATE, K_INDUCTION,
};
pub use model_check::smt_query::{
    encode_smt_query, Refusal, SmtEncodingLimits, SmtEncodingRefusal, SmtQueryKind,
};
pub use model_check::state_graph::{
    GraphEvidence, GraphEvidenceEnvelope, GraphUndefined, GraphWitness, LassoWitness, ModelLasso,
    ModelPath, OpenCause, RunEnd, StateGraphInstanceOutcome, StateGraphOutcome,
    StateGraphPropertyForm, TrapClosure, WitnessPath, WitnessSource,
};
pub use model_check::step_safety::{
    certify_step_records, settle_step_records, StepCertification, StepRecordVerdict,
    StepRecordsSettlement,
};
pub use model_check::step_settle::{
    settle_step_simulation, BackendCertificate, OtherBackendResult, StepBackendOutcome,
    StepSimulationSettlement, STEP_INDUCTION_DEPTH,
};
pub use model_check::subject_identity::SubjectIdentity;
pub use monitor_plan::{
    plan_obligations, AtomId, ClockBindingKey, MonitorAtom, MonitorPlanRefusal, MonitorProgram,
    MonitorSubject, MonitorTarget, PlanId, SubformulaId, TickInterval, TimedMonitorPlan,
    TimingObligation, TimingObligationSettings,
};

// FR-492: the refinement simulation query, its request and kind, as
// replay-owned wrappers (FR-100-AC-8), so CG names neither `spine` nor
// `qsl_eval` (ADR-011 FB-05, T-12).
pub use refinement_query::{
    encode_refinement_query, refinement_step_sources, RefinementKeyRefusal, RefinementQueryKind,
    RefinementQueryRequest, RefinementQuerySubject, RefinementUniverse,
};

pub use refinement::{
    abstract_classes, check_initial, check_step, initial_history, map_state, open_systems,
    step_history, AbstractClass, ConcreteStep, EvalFailure, HiddenSlot, HistoryValues,
    LivenessHalf, MappedState, MappingFailure, MappingRowRef, MappingUndetermined, ObjectKey,
    RefinementFailure, RefinementOutcome, RefinementPosition, RefinementSystems, SafetyOutcome,
    SafetyUndecided, StepArgument, StepRejection, StepResult, StepVerdict, SubjectRefusal, Taken,
    Undetermined,
};
// The equality operator a composite parity claim names (FR-358).
pub use quire_semantic_value::declaration::EqualityOperator;
// The typed identity and catalog code a `TerminalRecord` and a
// `TerminalValue` are built from, so the code generator, which depends on
// this crate alone, names them without naming `qsl_foundation`.
pub use outcome::undefined_reason;
pub use outcome::{
    ArtifactKind, ExecuteResult, ItemCause, ItemLabel, Operation, OutcomeArtifact,
    OutcomeDiagnostic, OutcomeDocument, OutcomeItem, OutcomeLocus, OutcomeStage, OutcomeWriteError,
    ParityReason, ResultLimit, ResultValue, OUTCOME_FORMAT,
};
pub use proof_result::{
    read_backend_provider_envelope, read_first_party_solver_basis, read_plugin_assignments,
    read_plugin_basis, read_plugin_cause_kind, read_plugin_certification,
    read_plugin_hybrid_counterexample, read_plugin_inconclusive_cause,
    read_plugin_inconclusive_cause_over, AppliedReduction, BackendProviderSource, Certification,
    DeclineCode, EmptyEnvelopeSet, IncompleteCause, InconclusiveCause, InstanceBound, MemoryBound,
    MemoryBoundMember, PluginReplayContext, PluginReplayRefusal, PluginResultRefusal, ProofBasis,
    ProofRefusalCause, ProofResultEnvelope, ProofResultRefusal, Proviso, QslOnlyCause,
    ReportedInconclusiveCause, SettlementBasis, StaleIdentity, StatedInstanceBound,
    StatedMemoryBound, StatisticalRequestRefusal, SymmetryBrokenCause, SymmetryRefusal,
    TerminalRecord, TerminalRecordRefusal, TerminalValue, UnavailabilityCause,
    ValueCheckUnsupported, ValueFault, WeakAccessFailure,
};
pub use qsl_foundation::{BackendId, Code, RequestIndex};
pub use quire_contract_model::{std001_code, Std001Code};
// The inputs `call_site` takes and the typed operation name it and FR-115
// select by. They are defined in `spine` because the spine compile reads
// them; re-exported at the root so CG names them without naming `spine`,
// which it may not (ADR-011 §3 FB-05, T-12 rule (a)).
pub use spine::{
    DependencyInput, DependencyInputRefusal, OperationName, SourceHolder, SuppliedLibrary,
};
// The O-07 occurrence key and O-12 source region these envelopes carry
// (#213 S-4), re-exported so CG, which reaches QSL only through this crate
// (ADR-011 FB-05), can name them. CG builds an occurrence key from a node id
// and the `Origin` re-exported below; a source region's constructor inputs
// (`qsl_foundation`'s `RawSourceRef`, `Revision` and `InvalidProvenance`)
// are not re-exported, so CG builds no source region.
pub use qsl_foundation::source::provenance::{OccurrenceKey, SourceRegion};
// The kernel identity and digest types a `ReplayRequestWire`'s or a frame
// counterexample's members are built from -- `quire_exact`'s identifier,
// occurrence origin and role, and value-accounting limits, and
// `qsl_foundation`'s digest-record vocabulary -- re-exported so
// CG, which reaches QSL only through this crate (ADR-011 FB-05), builds one
// without its own direct dependency on `quire-exact` or `qsl-foundation`.
// These re-exports add constructors CG can call directly:
// `DigestRecord::mint`, `NodeKey::new`,
// `SourceIdentity::new`, `Identifier::new` and `Origin::new`/`Role::new`
// among them, and with `Origin` the occurrence key's own
// `OccurrenceKey::new(NodeKey, Origin)` becomes callable. None is a
// T-12-governed constructor (ADR-011 §3 FB-05), so no rule is broken; CG
// needs exactly these to build the requests `call_site` and `replay`
// (FR-098) and `replay_frame` (FR-116) read.
pub use qsl_foundation::digest::{ByteDigest, DigestDomain, DigestRecord};
pub use qsl_foundation::SourceIdentity;
pub use quire_exact::{Identifier, NodeKey, Origin, Role, ScalarLimits, Value};
// FR-395: the types of `run`'s signature and result that the root did not
// export, so a consumer calls `run_with_requirements` and reads its result
// without naming `qsl_replay::spine` or `qsl_semantics`.
pub use qsl_semantics::library::LibraryName;
pub use qsl_semantics::model::object_environment::ObjectEnvironment;
pub use quire_exact::{Cancel, CancelCause};
pub use spine::{
    Call, CallArgument, CallIncomplete, CallLocus, CallOutcome, CallRefusal, CallValue,
    ClauseRunStage, CompileRefusal, RunRefusal, SpineLimits,
};
// The input refusal a `ReplayRefusal::Input` and a value-parity
// `ValueParityResult::RefusedInput` carry.
pub use quire_semantic_value::call::InputRefusal;
// The ADR-014 B-4 proof bound a `DeclaredDomain` wraps, its domain key and
// finite domain, and the kernel integer and interval an integer range is
// built from, re-exported so CG, which reaches QSL only through this crate
// (ADR-011 FB-05), builds a `DeclaredDomain` without its own direct
// dependency on `qsl-foundation` or `quire-exact`. With them come their
// constructors -- `DomainKey`'s `Node` and `Population` variants,
// `FiniteBound::cardinality`/`integer_range`/`depth`, `IntegerInterval::new` and `Integer`'s `From`
// conversions -- and the refusals they return. None is a T-12-governed
// constructor (ADR-011 §3 FB-05).
pub use execute::{ObjectEnvironmentFailure, Unresolved};
pub use object_environment::{
    object_reference_witness, read_object_environment, write_object_environment, EnvironmentLabels,
    EnvironmentModel, EnvironmentObject, EnvironmentPopulation, ObjectEnvironmentDocument,
    ObjectEnvironmentRef, ObjectEnvironmentRefusal, ObjectEnvironmentWriteError,
    ObjectReferenceError,
};
pub use qsl_foundation::bound::{
    DomainKey, DomainKind, EmptyFiniteBound, FiniteBound, FiniteBoundKind, ProofBound,
    ProofBoundRefusal,
};
pub use qsl_semantics::model::accounting::ModelNormalizationLimits;
pub use qsl_semantics::model::domain_package::DomainPackageRef;
pub use quire_exact::{EmptyInterval, Incomplete, Integer, IntegerInterval};
pub use request::{
    ByteProvision, DependencyEntry, DependencyEntryWire, ReplayRequest, ReplayRequestRefusal,
    ReplayRequestWire, StageLimitNameRefusal, StageLimits, StateEnvironment,
};
pub use result::{
    read_bounded, CauseCodecError, DisagreementCause, EvaluatedValue, HybridTraceDisagreement,
    InputArmResult, InputSettlement, ObservedTemporalParity, ObservedTemporalResult,
    ObservedTemporalSettlement, ReplayResult, SeparatingWitnessRecord, SeparationReason,
    SeparationRefusal, Verdict, WitnessArmResult, WitnessCheck, WitnessFailure, WitnessSettlement,
};
pub use supplied_limits::{read_supplied_limits, SuppliedLimitsRefusal};
// QSpec FR-323: the replay target a packet, envelope and request each carry.
pub use target::{
    NamedKind, ReplayEntry, ReplayOverBinding, ReplayTarget, ReplayTargetMembers,
    RequestTargetWire, TargetMember, TargetSelectionRefusal,
};
// FR-265 and FR-268: the separation-check step and the QSpec FR-207 value
// path a separating witness record carries, re-exported so CG names them
// through this crate (ADR-011 FB-05).
pub use qsl_foundation::witness::{
    InvocationBinding, ObservationIdentity, RuntimeValuePath, SeparationStep, ValuePathStep,
    ValuePathSubject,
};
pub use scalar::{
    GeneratedFault, NativeOutcome, OperandIdentity, OperandRefusal, OperatorClaim,
    OperatorIdentity, OperatorParityReport, OperatorParityResult, ScalarAgreement, ScalarClaim,
    ScalarOperand, ScalarOperation, ScalarOperator, ScalarOutcome, ValueClaim, ValueIdentity,
};
pub use witness::{
    CanonicalAssignment, ClaimedChange, DecodeRefusal, EntryFault, EnvelopeSource, FamilyPayload,
    FrameCounterexample, FrameOperation, MalformedTranscript, NoPayload, ObservedTemporalEnvelope,
    QuantityMagnitude, ReplaySource, ValueTextError, Witness, WitnessBinding, WitnessEnvelope,
    WitnessField, WitnessPacket, WitnessRefusal, WitnessSlot, WitnessValue, WitnessValueType,
};
// FR-116: the FR-106 document, object and FR-115 frame witness types a
// frame counterexample and its replay result carry, re-exported so CG can
// name and build them through this crate (ADR-011 FB-05). The node and
// occurrence identities are built from the `NodeKey`, `Origin` and
// `Identifier` re-exported above.
pub use qsl_semantics::model::observation::{
    AdmissionFailure, AdmissionRecord, DocumentRef, FrameChange, FrameWitness, LinkDifference,
    SelectedObject,
};
pub use quire_checked::key::DeclarationKey;

// FR-122: the state-clause counterexample's replay entry, payload and
// result, and the FR-106 observation types a payload is built from and a
// refusal names, re-exported so CG can name and build them through this
// crate (ADR-011 FB-05).
pub use execute::{replay_state_clause, ClauseIdentityMismatch, StateClauseReplayResult};
pub use qsl_forms::StateClauseKind;
pub use qsl_semantics::model::observation::{
    AnchorKind, ClauseSelectionInput, ObservationForm, ObservationLimits, SelectedAnchor,
    SnapshotValue,
};
pub use witness::StateClauseCounterexample;

/// A structural node identity for a test fixture, distinct per `fill`.
#[cfg(test)]
pub(crate) fn fixture_node(fill: u8) -> NodeKey {
    NodeKey::new(
        "fixture",
        vec![quire_exact::PathSegment::Text(fill.to_string())],
        0,
    )
    .expect("a fixture node is admissible")
}

/// A parameter node's transcript name.
#[cfg(test)]
pub(crate) fn pn(node: &NodeKey) -> String {
    crate::witness::parameter_name(node).expect("a node spells")
}

/// A universe identity for a test fixture, distinct per `fill`.
#[cfg(test)]
pub(crate) fn fixture_universe(fill: u8) -> quire_exact::UniverseId {
    quire_exact::UniverseId::new(fixture_node(fill))
}

/// An effective declaration identity for a test fixture, distinct per `fill`.
#[cfg(test)]
pub(crate) fn fixture_effective(fill: u8) -> quire_exact::EffectiveId {
    quire_exact::EffectiveId::new(
        None,
        quire_exact::OriginalDeclarationKey::new("fixture", format!("node{fill}")),
    )
}

#[cfg(test)]
mod redaction_tests {
    use ix_trace_rs::trace;

    use super::*;
    use quire_exact::Identifier;

    fn scalar_limits(seed: u64) -> quire_exact::ScalarLimits {
        quire_exact::ScalarLimits {
            integer_bits: seed,
            decimal_digits: seed,
            scale_expansion: seed,
            text_input_bytes: seed,
            text_scalars: seed,
            normalized_scalars: seed,
            unit_edges: seed,
            value_occurrences: seed,
            work_units: seed,
            result_units: seed,
        }
    }

    /// FR-073-AC-3 (TC-211): a malformed-transcript refusal from the witness
    /// envelope renders with no unredacted content, and a separately,
    /// validly constructed value carrying the same content stays fully
    /// readable through its typed accessor.
    #[trace("TC-211", "FR-073-AC-3")]
    #[test]
    fn tc_211_refusal_causes_redact_while_typed_accessors_stay_readable() {
        // Half 1: a validly constructed request whose byte provision
        // carries entry X's exact bytes.
        let entry_x: Vec<u8> = (0..4096u32).map(|i| (i % 241) as u8).collect();
        // It returns the full content through the typed accessor.
        let valid_wire = ReplayRequestWire {
            profile_selections: vec![],
            package_name: "fixture".to_owned(),
            sources: vec![(
                "registry".to_owned(),
                "pkg-a".to_owned(),
                "git".to_owned(),
                "rev-1".to_owned(),
            )],
            dependencies: Vec::new(),
            replay_target: crate::RequestTargetWire::function(
                QualifiedName::new(vec![Identifier::new("f").unwrap()]).unwrap(),
                crate::target::test_key(),
            ),
            source: ReplaySource::Input(vec![CanonicalAssignment {
                parameter: crate::fixture_node(9),
                value: WitnessValue::Integer(1),
            }]),
            obligation_identity: crate::ObligationIdentity::new(crate::fixture_node(2), None, 0),
            backend: "kani-backend-1".to_owned(),
            state_environment: StateEnvironment::new(vec![]),
            accounting_limits: scalar_limits(1),
            stage_limits: std::collections::BTreeMap::new(),
            supplied_limits: None,
            object_environment: None,
            declared_domains: Vec::new(),
            byte_provision: vec![("registry".to_owned(), "pkg-a".to_owned(), entry_x.clone())],
            documents: vec![],
        };
        let valid_request =
            ReplayRequest::decode(valid_wire, crate::ReplayLimits::default()).unwrap();
        let looked_up = valid_request
            .byte_provision()
            .get(&crate::RawSourceRef::new(
                "registry".to_owned(),
                "pkg-a".to_owned(),
                "git".to_owned(),
                "rev-1".to_owned(),
            ))
            .unwrap();
        assert_eq!(looked_up, entry_x.as_slice());

        // Half 2: witness envelope malformed-transcript refusal.
        let marker = "MARKER-transcript-y-4c2b1a";
        let untrimmed = format!("leading-junk<<<assertion|h|c|{marker}=1>>>");
        let refusal = Witness::parse(untrimmed).unwrap_err();
        let rendered = format!("{refusal:?} {refusal}");
        assert!(!rendered.contains(marker));

        // A separately, validly constructed witness from the trimmed block
        // still returns the marker through its typed accessor.
        let valid_witness = Witness::parse(format!("<<<assertion|h|c|{marker}=1>>>")).unwrap();
        assert!(valid_witness
            .concrete_values()
            .iter()
            .any(|(name, _)| name == marker));
    }
}
