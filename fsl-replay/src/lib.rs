// SPDX-License-Identifier: AGPL-3.0-or-later
//! `fsl-replay`: the ADR-011 §6.1 layer **6** crate (ADR-011 §7.3
//! X-10) -- the CG-facing replay facade (ADR-011 §6.1, ADR-013 O-24 to
//! O-27, C-13). It depends on layers 1 to 5, F and K, as §6.1 allows.
//!
//! Its public API is the compile entry [`compile_package`] and the replay executor entry [`replay`] (ADR-013 TK-01,
//! C-13; FR-098), which recompiles a request's source-selected inputs
//! through the spine ([`spine::parse`], [`spine::select`], [`spine::check`]
//! and [`spine::package`], S1 to E4) and calls the selected
//! function through S6a (`CheckedPackageEvaluation::call`), and the four
//! typed envelopes it reads and settles -- the proof-result envelope
//! (`proof_result`), the counterexample/witness envelope (`witness`), the
//! replay request (`request`) and the replay result (`result`) -- with
//! their round trips and their redacted rendering (FR-069 through FR-073).
//! It builds no backend invocation and no Kani harness. CG reaches this
//! crate's public API and nothing else in FSL (ADR-011 FB-05); `command`
//! composes those four operations for the CLI's `compile`.
//!
//! # Identity and provenance types
//!
//! O-07's occurrence key and O-12's source region are
//! `fsl_foundation::source::provenance`'s (#213 S-4), and this
//! crate re-exports them for CG. O-09's obligation identity is this crate's
//! own, the canonical type ADR-013 OQ-H assigns here, which CG names
//! directly; IR names no FSL type. O-11's `QualifiedName` is still `identity`'s own minimal
//! version, with the shape ADR-013 specifies; `identity` names the ticket
//! that absorbs it.
//!
//! # Former paths (FR-401-AC-3)
//!
//! The ADR-013 O-16 `Category` a `Verdict` carries is
//! `filament_semantic_value::catalog::Category`. This crate no longer re-exports
//! it:
//!
//! ```
//! use filament_semantic_value::catalog::Category;
//! # let _ = Category::Success;
//! ```
//!
//! ```compile_fail,E0432
//! use fsl_replay::Category;
//! ```

#![forbid(unsafe_code)]

mod admitted;
mod application;
mod axes;
pub mod backends;
mod bounds;
mod build_inventory;
mod call_replay_context;
mod call_requirements;
mod call_site;
mod certificate;
mod clause_evaluation;
mod compile;
mod composite;
pub mod contract_check;
mod exact_probabilistic;
mod execute;
mod hybrid;
mod hybrid_replay;
mod id_text;
mod identity;
mod limits;
mod model_check;
pub use model_check::digital_state_graph::{
    explore_digital_state_graph, DigitalGraphEdge, DigitalGraphLimit, DigitalStateGraph,
};
mod monitor_plan;
mod object_environment;
pub mod outcome;

/// FR-286: the typed refusal pairs produced by replay and its spine.
pub const CATALOG_PAIRS: &[fsl_foundation::diagnostic::CatalogCode] = &[
    fsl_foundation::diagnostic::CatalogCode::new("cancelled", "caller-cancelled"),
    fsl_foundation::diagnostic::CatalogCode::new("foreign_reference", "foreign-universe"),
    fsl_foundation::diagnostic::CatalogCode::new("ill_typed", "type-mismatch"),
    fsl_foundation::diagnostic::CatalogCode::new("integer_out_of_domain", "outside-domain"),
    fsl_foundation::diagnostic::CatalogCode::new("invalid_capability", "not-statistical"),
    fsl_foundation::diagnostic::CatalogCode::new("invalid_runtime_input", "absent-key"),
    fsl_foundation::diagnostic::CatalogCode::new("invalid_runtime_input", "wrong-role-mapping"),
    fsl_foundation::diagnostic::CatalogCode::new("invalid_runtime_input", "missing-member"),
    fsl_foundation::diagnostic::CatalogCode::new("invalid_runtime_input", "unknown-member"),
    fsl_foundation::diagnostic::CatalogCode::new("invalid_runtime_input", "invalid-value"),
    fsl_foundation::diagnostic::CatalogCode::new("missing_declaration", "missing-name"),
    fsl_foundation::diagnostic::CatalogCode::new("resource_exhausted", "insufficient-next-charge"),
    fsl_foundation::diagnostic::CatalogCode::new("resource_exhausted", "max-automaton-states"),
    fsl_foundation::diagnostic::CatalogCode::new(
        "runtime_invariant",
        "established-invariant-broken",
    ),
    fsl_foundation::diagnostic::CatalogCode::new("unsupported_construct", "not-yet-implemented"),
    fsl_foundation::diagnostic::CatalogCode::new("unsupported_construct", "expression-form"),
    fsl_foundation::diagnostic::CatalogCode::new("unsupported_projection", "not-markov"),
    fsl_foundation::diagnostic::CatalogCode::new("stage_limit_exceeded", "input-bytes-exceeded"),
    fsl_foundation::diagnostic::CatalogCode::new("stage_limit_exceeded", "work-budget-exceeded"),
];
pub mod planner;
mod probability_certificate;
mod product_action;
mod proof_result;
pub mod protocol_analysis;
mod refinement;
mod refinement_query;
mod request;
mod result;
mod scalar;
pub mod source_inventory;
pub mod spine;
mod spine_eval_api;
pub mod statistical;
mod supplied_limits;
mod target;
mod timed;
pub mod value_check;
mod witness;
mod witness_entry_name;

use fsl_package::{ModelIdentity, ObligationIdentity};

pub use admitted::{
    compile_admitted, compile_admitted_with_temporal_selections, AdmittedCompilation,
    AdmittedComposite, AdmittedCompositeForm, AdmittedContext, AdmittedDimension, AdmittedEnum,
    AdmittedEnumCase, AdmittedLookupRefusal, AdmittedRefusal, AdmittedResource,
    AdmittedUnionMember, AdmittedUnit,
};
pub use application::{
    acquire_checked_closure, acquire_checked_closure_with_integer_division,
    ApplicationBeginRefusal, ApplicationEnvironment, ApplicationRegistry, CallableLookupRefusal,
    CallableSelection, CheckedApplicationProgress, CheckedApplicationSession,
    CheckedClosureCompilation, CheckedClosureHandle, ClosureOwnerRefusal, NestedCallOccurrence,
    NestedCallSite, NestedCallSiteRefusal, NominalDeclaration, NominalDeclarationForm,
    NominalDeclarationMember, NominalDeclarationRefusal, SelectionRefusal,
};
pub use bounds::{BoundExceeded, ReplayLimits, DEFAULT_REPLAY_INPUT_BYTES};
pub use build_inventory::{
    build_inventory, Advertisement, Application, BuildInventory, Consumer, Family, Profile,
};
pub use call_replay_context::{run_with_replay_context, CallReplayContext, RunWithReplayContext};
pub use call_requirements::{
    call_requirements_for, run_with_requirements, run_with_requirements_reading,
    run_with_requirements_reading_and_profile_catalog,
    run_with_requirements_reading_and_profile_catalog_and_integer_division, CallRequirement,
    CallRequirements, RequirementsRun, RunWithRequirements,
};
pub use call_site::{
    call_site, CallSite, CallSiteRefusal, CallSiteSelection, ClauseName, ClauseSite, FieldName,
    FieldSite, FunctionSite, ImportedFunctionName, ImportedFunctionSite, OperationSite,
    PopulationName, PopulationSite, ValueParitySite,
};
pub use certificate::closed_form::check::{
    check_closed_form, CertificateRefusal, ClaimTriple, ClosedFormResult,
};
pub use certificate::closed_form::outcome::{
    AmcEvidence, ClosedFormAnalysis, ClosedFormEvidence, ClosedFormOutcome, DemandWitness,
    HighModeMiss, JobIterates, MissEvidence, TaskFixpoint,
};
pub use certificate::closed_form::recompute::{recompute, ClosedFormCheck, ClosedFormDisagreement};
pub use certificate::closed_form::settle::settle;
pub use certificate::closed_form::task_set::{
    Arrival, Criticality, ScheduleClaim, ScheduleSubject, SchedulingPolicy, Task, TaskSet,
    TaskSetRefusal, TimeMember, Wcet,
};
pub use certificate::closure::{
    check_closure, check_components, check_components_over_stutter_closure, check_protocol_closure,
    check_protocol_components, CertificateRejection, CertificateRequest, ClosureCertificate,
    ClosureRefusal, ProductStateRef, ProtocolCertificateRequest, TemporalCertificateCheck,
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
pub use certificate::task_product::{
    same_source_transition, TaskArrivalKind, TaskArrivalPhase, TaskClockCeiling,
    TaskClosureEvidence, TaskClosureMethod, TaskDiagonalConstraint, TaskDiagonalOperator,
    TaskJobPhase, TaskLiveJob, TaskPendingBudgetChange, TaskProductCertificate, TaskProductKey,
    TaskProductKeyRefusal, TaskProductShapeReason, TaskProductShapeRefusal, TaskProductWireError,
    TaskQueueBound, TaskSchedulerControl, TaskSubtraction, TaskTimeProgress,
    TaskTransitionTemplate,
};
pub use certificate::zone::{LuBounds, Zone, ZoneError};
pub use certificate::zone_certificate::{
    check_free_delay_zone_certificate, check_zone_certificate, explore_task_product,
    recomputed_member, AcceptanceSet, CertEdge, CertNode, CertificateCheckLimits, CertificateItem,
    CertifiedItem, CheckerFault, ComponentReason, Components, DivergenceWitness, DivergentTarget,
    FairnessMember, ItemIdentity, NonZeno, ProductMark, QuotientEdge, QuotientIdentity,
    QuotientInitial, QuotientStop, QuotientVertex, RefinementIdentity, RefinementPart, SilentMove,
    StateGraphCertificateForm, StateGraphPart, TaskProductQuotient, ZoneCertificate,
    ZoneCertificateRefusal, ZoneCertificateRequest, ZoneCertificateVerdict, ZoneCertificateWireError,
};
pub use certificate::{CertificateLocus, CertificateMember, CertificateRule, QueryPart};
/// FR-322 "Operand index segments": a package's graph and its wire ids.
pub use fsl_semantics::check::wire_id::{WireGraph, WireIdError};
// FR-491: the state-clause evaluation entry and its typed result, at the root
// so CG names them without naming `spine` (ADR-011 FB-05, T-12 rule (a)).
pub use clause_evaluation::{
    evaluate_clause, ClauseEvaluation, ClauseEvaluationRequest, ClauseIncomplete,
    ClauseObservation, ClauseRefusal,
};
/// The checked owner's opaque admitted integer-division selection.
pub use filament_checked::integer_division::AdmittedIntegerDivision;
/// The semantic owner's division-definition admission and closed definition references.
pub use fsl_semantics::value::{admit_integer_division, DefinitionReference, DefinitionRole};

pub use compile::{
    compile_package, compile_package_with_execution_points, compile_package_with_integer_division,
    compile_package_with_temporal_selections,
    compile_package_with_temporal_selections_and_execution_points, CompileUnit, CompiledPackage,
};
pub use composite::{
    CompositeEvidence, CompositeIdentity, CompositeParityClaim, CompositeParityReport,
    CompositeParityResult, EqualityOutcome, FalsifiedParity, IncompleteStage, NativeCause,
    NativeParityObservation, Refinement, VerifiedShadow, VerifiedShadowReport,
    VerifiedShadowResult,
};
pub use exact_probabilistic::{
    settle_exact_prob_build_issue, ExactProbBuildIssue, ExactProbBuildRecord,
};
pub use execute::{
    parity_obligation, replay, replay_composite_parity, replay_frame, replay_operator_parity,
    replay_value_parity, replay_with_cancel, replay_with_cancel_and_temporal_selections,
    replay_with_temporal_selections, settle_verified_shadow, value_parity_obligation, BoundEntries,
    DependencySelectionsCause, Domain, FrameIdentityMismatch, FrameReplayResult,
    IdentityEncodeError, LimitAboveReader, MalformedOperationReason, PackageReadRefusal,
    ParityArgument, ParityBoundRefusal, ParityPreimage, ReplayRefusal, ScalarIdentityMismatch,
    ValueParityArgument, ValueParityPreimage, ValueParityReport, ValueParityResult,
};
pub use hybrid::{
    hybrid_locus_site, hybrid_locus_wire, settle_hybrid_candidates, AdmittedHybridProcessItem,
    AdmittedHybridProcessRequest, BoundDimension, BudgetRefusal, CertificateRecord,
    CertificateStatus, ClaimHorizon, HybridAnswer, HybridEntryRefusal, HybridProcessDependency,
    HybridProcessDependencyPackage, HybridProcessItem, HybridProcessPackageContext,
    HybridProcessRequest, HybridProcessRequestRefusal, HybridProcessStage, HybridProvider,
    HybridSettlement, HybridSolveRequest, HybridVerdict, NegativeHorizon, OptionRefusal,
    SolverBudget, SolverCoverage, SolverHorizon, SolverMethod, SolverOutcome, SolverResult,
    SolverStop, UnsupportedConstruct, HYBRID_CERTIFICATE_CHECKER,
};
pub use hybrid_replay::{
    admit as admit_hybrid_trace, box_advance, box_hull, box_tube, box_within,
    check_certificate as check_hybrid_certificate, exact_replay, polynomial_class_gap,
    probe_flowpipe_claim as probe_hybrid_flowpipe_claim,
    read_certificate as read_hybrid_certificate, static_replay,
    Acceptance as HybridCertificateAcceptance, CertificateOutcome as HybridCertificateOutcome,
    CertificateRefusal as HybridCertificateRefusal, Decisive as HybridDecisive, ExactCheck,
    ExactGap, ExactInterval, ExactReal, ExactRecord, ExactReplay, ExpansionGap,
    Expect as HybridCertificateExpect, ExpressionLocus, ExpressionSite, Fired as HybridFired,
    HybridBox, HybridCertificate, HybridCertificateRule, HybridCounterexample,
    HybridCounterexampleKind, HybridReplayMeter, HybridReplayRefusal, HybridTrace,
    InitialHull as HybridInitialHull, Interval as HybridInterval, JsonPointer,
    ModelReader as HybridModelReader, ProbeOutcome as HybridProbeOutcome,
    ProofCoverage as HybridProofCoverage, ReadStop as HybridReadStop,
    Rejection as HybridCertificateRejection, ReplayRecord, Root, StaticCheck, StaticReplay,
    TermLocus, TraceBefore, TraceFault, TraceJump, TraceRefusal, TraceState, Tri as HybridTri,
    UndefinedCause, UndefinedInstant,
};
pub use identity::{
    DeclaredDomain, DefinitionSourceRef, EmptyQualifiedName, ProfileSelection, QualifiedName,
    TracePosition,
};
pub use limits::{AccountingLimits, CallerLimits, EngineLimits, RenderLimits};
pub use model_check::counterexample::{
    undefined_finding, ClaimPredicate, CounterexampleKind, Lasso, LoopProofCounterexample,
    LoopProofFailure, LoopProofObligation, MappingSite, ModelStep, ObservedTemporalCounterexample,
    ObservedTemporalKind, ObservedTemporalStep, Race, TemporalCounterexample, TimedPoint,
    UndefinedEvaluation, UndefinedLocus, UndefinedSite,
};
pub use model_check::existential_outcome::{ExistentialInstanceOutcome, ExistentialOutcome};
pub use model_check::finite_hyper_replay::{
    replay_finite_hyper_refutation, reproduce_finite_hyper_refutation,
    ReproducedFiniteHyperRefutation,
};
pub use model_check::graph_replay::{
    replay_model_graph, replay_model_graph_from_package, replay_model_graph_with_cancel,
    replay_model_graph_with_cancel_and_temporal_selections,
    replay_model_graph_with_temporal_selections,
};
pub use model_check::graph_settle::{
    settle_state_graph, GraphReplayResult, GraphReplaySettlement, StateGraphMethod,
    StateGraphRecord,
};
pub use model_check::hyper_counterexample::{
    HyperCounterexample, HyperCounterexampleKind, HyperLocus, HyperStep, HyperTrace,
    HyperUndefined, MatchWitness, MatchWitnessCheck, StepMark,
};
pub use model_check::hyper_outcome::{BoundCovers, HyperOutcome, HyperReductionCause};
pub use model_check::hyper_replay::{
    check_match_witness, replay_hyper_trace_tuple, replay_model_relation_tuple, HyperReplay,
    ModelRelationReplay,
};
pub use model_check::hyper_settle::{
    settle_hyper_check, settle_hyper_form, settle_hyper_smt, HyperRecord, HyperSettleRefusal,
    LimitOrigin, ProductMember, UsedLimit,
};
pub use model_check::hyper_smt_outcome::{HyperSmtOutcome, HyperSmtResult, SmtMethod};
pub use model_check::hyper_smt_query::{
    encode_hyper_query, HyperProductIdentity, HyperQuery, HyperQueryRequest,
};
pub use model_check::identity::{
    node_occurrence, ClauseIdentity, ClauseRootRefusal, HyperIdentity, StateGraphIdentity,
};
pub use model_check::item::{
    Instance, InvariantClause, ItemRefusal, LivenessTranslation, ModelCheckItem, Translation,
};
pub use model_check::limits::{
    meter_bound, ExactProbLimit, ExactProbLimits, ModelCheckLimit, ModelCheckLimits, ReachedLimit,
    DEFAULT_EXACT_PROB_MAX_ITERATIONS, DEFAULT_EXACT_PROB_MAX_POLICY_ITERATIONS,
    DEFAULT_EXACT_PROB_MAX_PRECISION_BITS, DEFAULT_EXACT_PROB_MAX_RATIONAL_BITS,
    DEFAULT_EXACT_PROB_MAX_WITNESS_PATHS, DEFAULT_EXACT_PROB_PRECISION_BITS,
    DEFAULT_MAX_AUTOMATON_STATES, DEFAULT_MAX_RELATION_TUPLES, DEFAULT_MAX_WALK_STEPS,
    DEFAULT_MAX_WITNESS_SET, DEFAULT_WITNESS_SAMPLES, DEFAULT_WITNESS_SEED,
};
pub use model_check::outcome::{
    MemoryCause, ModelCheckOutcome, ReductionCause, RunStats, UndecidedCause,
};
pub use model_check::refinement_replay::{
    replay_free_delay_refinement_trace, replay_protocol_assumption_witness,
    replay_refinement_trace, replay_timed_refinement_trace, ProtocolAssumptionReplaySettlement,
    RefinementTraceSettlement,
};
pub use model_check::refinement_settle::{
    settle_refinement, settle_refinement_refusal, settle_refinement_safety,
    settle_refinement_safety_with_assumption_replay, settle_refinement_with_assumption_replay,
    RefinementRecord,
};
#[cfg(feature = "test-support")]
#[doc(hidden)]
pub use model_check::replay::replay_timed_counterexample;
pub use model_check::replay::{
    behaviour_valuations, evaluate_model_trace, evaluate_protocol_trace, replay_model_trace,
    replay_checked_source_model_trace,
    replay_model_trace_from_package, replay_model_trace_with_temporal_selections,
    replay_protocol_trace, replay_protocol_trace_from_package,
    replay_protocol_trace_with_temporal_selections, BehaviourValuationEnd,
    BehaviourValuations, DelayBar, HandoffSupply, ModelTraceEnd, ModelTraceRefusal,
    ModelTraceReplayResult, ModelTraceRun, ModelTraceSettlement, ModelTraceSubject,
    ModelTraceValue, ProtocolTraceEnd, ProtocolTraceRun, ProtocolTraceSubject, StrongFairnessHint,
    TimedReplay,
};
pub use model_check::settle::{
    settle_model_check, settle_model_check_over_stutter_closure, settle_protocol_check,
    CertificateChecker, Execution, ModelCheckRecord, RecordWarning, ResultLabel, Truth,
    EXPLICIT_STATE, K_INDUCTION,
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
    canonical_rational, plan_obligations, ActivationToken, AtomId, ClockBindingKey, FormulaToken,
    MonitorAtom, MonitorPlanRefusal, MonitorProgram, MonitorSubject, MonitorTarget, PlanId,
    PlanRootPremises, PremiseDefect, SubformulaId, TickInterval, TimedMonitorPlan,
    TimingObligation, TimingObligationSettings, PLAN_ROOT_BINDINGS,
};

// FR-492: the refinement simulation query, its request and kind, as
// replay-owned wrappers (FR-100-AC-8), so CG names neither `spine` nor
// `fsl_eval` (ADR-011 FB-05, T-12).
pub use refinement_query::{
    encode_refinement_query, RefinementHandoffRefusal, RefinementKeyRefusal, RefinementQueryKind,
    RefinementQueryRequest, RefinementQuerySubject, RefinementUniverse,
};

pub use refinement::{
    abstract_classes, candidate_lasso_has_fair_run, check_initial, check_protocol_initial,
    check_protocol_state_relations, check_protocol_step, check_step, initial_history, map_state,
    open_protocol_systems, open_systems, protocol_observation_label, protocol_transition_class,
    step_history, step_view_over, AbstractClass, AbstractInitial, AbstractProtocolSystem,
    AssumptionIdentity, AssumptionOrigin, AssumptionSelectedMember, AssumptionSide,
    AssumptionTarget, CandidateFairState, CandidateFairStep, CandidateLivenessError, ConcreteStep,
    EvalFailure, HiddenSlot, HistoryValues, LivenessHalf, MappedState, MappingFailure,
    MappingRowRef, MappingUndetermined, ObjectKey, ObservationField, ObservationLabel,
    PairedAssumptionPath, PairedAssumptionStep, ProtocolAssumptionFairness,
    ProtocolAssumptionReason, ProtocolAssumptionWitness, ProtocolCandidate, ProtocolRelationError,
    ProtocolStateRelation, ProtocolStep, ProtocolStepView, ProtocolSteps, ProtocolSystemsRefusal,
    ProtocolTrace, RefinementFailure, RefinementOutcome, RefinementPosition, RefinementSystems,
    RefinementWitness, RefinementWitnessError, RequiredTemporalFairness, SafetyOutcome,
    SafetyUndecided, StateIndex, StepArgument, StepRejection, StepResult, StepVerdict,
    StepViewRefusal, SubjectRefusal, Taken, TerminalSuccessDirection, Undetermined,
    UnsteppedConstruct,
};
// The equality operator a composite parity claim names (FR-358).
pub use filament_semantic_value::declaration::EqualityOperator;
// The typed identity and catalog code a `TerminalRecord` and a
// `TerminalValue` are built from, so the code generator, which depends on
// this crate alone, names them without naming `fsl_foundation`.
pub use axes::{ActivationAxis, ExecutionAxis, OutcomeAxes, TruthAxis};
pub use filament_contract_model::{std001_code, Std001Code};
pub use fsl_foundation::{BackendId, Code, RequestIndex};
pub use outcome::undefined_reason;
pub use outcome::{
    assignments_wire, certificate_identity_wire, claim_locus_wire, graph_evidence_wire,
    hyper_counterexample_wire, omitted_witnesses, probabilistic_undefined_wire, probabilistic_wire,
    protocol_temporal_wire, read_task_temporal_counterexample, read_task_temporal_document,
    read_temporal_counterexample, replay_model_trace_document, replay_task_product_trace,
    sampled_witness_wire, state_key_wire, task_product_transition_wire,
    task_source_transition_wire, task_temporal_wire, task_temporal_wire_with_queue_growth,
    task_transition_wire, temporal_wire, ClauseRunNotAnItem, ConformanceClosureOutput,
    ConformanceGlobalOutput, ConformanceMessageBoundOutput, ConformanceObligationOutput,
    ConformanceProjectionOutput, ConformanceRecordingIdentity, ConformanceReplayOutput,
    CounterexampleWireError, ExecuteResult, ItemCause, ItemLabel, OmittedWitness, Operation,
    OutcomeDiagnostic, OutcomeDocument, OutcomeItem, OutcomeItemError, OutcomeLocus, OutcomeStage,
    OutcomeWriteError, ParityReason, ProtocolConformanceOutput, ProtocolWire, ReadCounterexample,
    ResultLimit, ResultValue, TaskProductCounterexampleDocument, TaskProductTraceReplayRefusal,
    TemporalCounterexampleReadRefusal, TraceAlphabet, WitnessOmission, OUTCOME_FORMAT,
};
pub use planner::{
    read_premise_item, read_result_entry, read_settlement_request, Budget, PlannerDeclaration,
    PlannerResult, PremiseItem, PremiseResult, ResultEntry, SettlementRequest,
};
pub use probability_certificate::admit_acyclic_probabilistic_refutation_subject;
#[cfg(feature = "test-support")]
pub use probability_certificate::{
    admit_acyclic_probabilistic_refutation_subject_with_identity,
    admit_cyclic_eventual_certificate_subject_with_identity,
    check_bounded_fair_probability_certificate_observing,
    check_cyclic_mdp_probability_certificate_observing,
    check_cyclic_mdp_probability_certificate_with_identity,
    check_cyclic_reward_probability_certificate_observing, BoundedFairProbabilityPhase,
    CyclicMdpProbabilityPhase, RewardProbabilityPhase,
};
pub use probability_certificate::{
    admit_expected_reward_certificate_subject, check_claim_exact_reachability,
    check_probability_certificate_document, check_probability_certificate_package,
    check_probability_certificate_source, claim_binding,
    cyclic_mdp_probability_certificate_document,
    cyclic_mdp_reward_probability_certificate_document, digital_probability_product_key,
    long_run_probability_certificate_document, probabilistic_claim_identity,
    probabilistic_formula_monitor_context, probability_certificate_document,
    probability_proof_document, read_long_run_probability_certificate,
    read_probability_certificate, read_probability_proof_document,
    read_weighted_long_run_probability_certificate, region_probability_product_key,
    replay_probabilistic_undefined, replay_probabilistic_witness,
    weighted_long_run_probability_certificate_document, write_long_run_probability_certificate,
    write_probability_certificate, write_probability_proof_document,
    write_weighted_long_run_probability_certificate, ClaimBinding, DigitalClockKey,
    DigitalDeadlineKey, DigitalProductKeyInput, DigitalTail, ExactProbabilityAcceptance,
    ExactProbabilityCertificate, ExactProbabilityCertificateCheck, ExactProbabilityGraphError,
    ExactProbabilityValue, LongRunProbabilityComponent, LongRunProbabilityDocument,
    ProbabilisticCounterexample, ProbabilityBoundSide, ProbabilityCertificateDocument,
    ProbabilityCertificateLocus, ProbabilityCertificateRejection, ProbabilityCertificateRule,
    ProbabilityDocumentError, ProbabilityDocumentIoLimits, ProbabilityDocumentLimit,
    ProbabilityDocumentLimits, ProbabilityKeyAdmission, ProbabilityObjective,
    ProbabilityPackageRequest, ProbabilityProductKey, ProbabilityProofDocument,
    ProbabilityRewardObjective, ProbabilitySourceError, ProbabilitySourceLimits,
    ProbabilitySourceRequest, ProbabilityStateEvent, ProbabilityUndefinedWitness,
    ProbabilityWitnessError, ProbabilityWitnessPath, ProbabilityWitnessReplay,
    ProbabilityWitnessStep, RegionClockKey, RegionDeadlineKey, RegionKeyKind,
    RegionProductKeyInput, ReproducedProbabilityWitness, TimedProbabilityForm,
    TimedProbabilityObjective, TimedProbabilityRoute, WeightedLongRunProbabilityDocument,
};
#[cfg(feature = "test-support")]
pub use probability_certificate::{
    admit_expected_reward_certificate_subject_with_identity,
    check_long_run_probability_certificate_observing,
    check_long_run_probability_certificate_with_identity, replay_probabilistic_witness_observing,
    LongRunProbabilityPhase, ProbabilityWitnessSemanticPhase,
};
pub use product_action::{
    FreeDelayActionRow, FreeDelayRow, FreeDelaysRace, FreeRacer, GivenRacer, NotMixedRace,
    ProductAction, ProductActionError, ProductActionRefusal, Schedule,
};
pub use proof_result::{
    read_backend_provider_envelope, read_first_party_solver_basis, read_plugin_assignments,
    read_plugin_basis, read_plugin_cause_kind, read_plugin_certification,
    read_plugin_hybrid_counterexample, read_plugin_hybrid_counterexample_over,
    read_plugin_inconclusive_cause, read_plugin_inconclusive_cause_over, AppliedReduction,
    BackendProviderSource, BoundsMethod, Certification, DeclineCode, EmptyEnvelopeSet,
    ExactBinding, ExactEntry, ExactOrBounds, ExactRational, Extremum, FslOnlyCause,
    IncompleteCause, InconclusiveCause, InstanceBound, MemoryBound, MemoryBoundMember,
    PluginReplayContext, PluginReplayRefusal, PluginResultRefusal, PremiseOutcome, ProofBasis,
    ProofRefusalCause, ProofResultEnvelope, ProofResultRefusal, Proviso, ReportedInconclusiveCause,
    SettlementBasis, StaleIdentity, StatedInstanceBound, StatedMemoryBound,
    StatisticalRequestRefusal, SymmetryBrokenCause, SymmetryRefusal, TerminalRecord,
    TerminalRecordRefusal, TerminalValue, ToolFailure, TransferReason, UnavailabilityCause,
    ValueCheckUnsupported, ValueFault, WeakAccessFailure,
};
// The inputs `call_site` takes and the typed operation name it and FR-115
// select by. They are defined in `spine` because the spine compile reads
// them; re-exported at the root so CG names them without naming `spine`,
// which it may not (ADR-011 §3 FB-05, T-12 rule (a)).
pub use spine::{
    DependencyInput, DependencyInputRefusal, OperationName, SourceHolder, SuppliedLibrary,
};
// The O-07 occurrence key and O-12 source region these envelopes carry
// (#213 S-4), re-exported so CG, which reaches FSL only through this crate
// (ADR-011 FB-05), can name them. CG builds an occurrence key from a node id
// and the `Origin` re-exported below; a source region's constructor inputs
// (`fsl_foundation`'s `RawSourceRef`, `Revision` and `InvalidProvenance`)
// are not re-exported, so CG builds no source region.
pub use fsl_foundation::source::provenance::{OccurrenceKey, SourceRegion};
// The kernel identity types a `ReplayRequestWire`'s or a frame
// counterexample's members are built from -- `filament_exact`'s identifier,
// occurrence origin and role, and value-accounting limits -- re-exported so
// CG, which reaches FSL only through this crate (ADR-011 FB-05), builds one
// without its own direct dependency on `filament-exact` or `fsl-foundation`.
// These re-exports add constructors CG can call directly:
// `NodeKey::new`, `SourceIdentity::new`, `Identifier::new` and `Origin::new`/`Role::new`
// among them, and with `Origin` the occurrence key's own
// `OccurrenceKey::new(NodeKey, Origin)` becomes callable. None is a
// T-12-governed constructor (ADR-011 §3 FB-05), so no rule is broken; CG
// needs exactly these to build the requests `call_site` and `replay`
// (FR-098) and `replay_frame` (FR-116) read.
pub use filament_exact::{Identifier, NodeKey, Origin, Role, ScalarLimits, Value};
pub use fsl_foundation::SourceIdentity;
// FR-395: the types of `run`'s signature and result that the root did not
// export, so a consumer calls `run_with_requirements` and reads its result
// without naming `fsl_replay::spine` or `fsl_semantics`.
pub use filament_exact::{Cancel, CancelCause};
pub use fsl_semantics::library::LibraryName;
pub use fsl_semantics::model::object_environment::ObjectEnvironment;
pub use spine::{
    Call, CallArgument, CallIncomplete, CallLocus, CallOutcome, CallRefusal, CallValue,
    ClauseRunStage, CompileRefusal, RunRefusal, SpineLimits,
};
// FR-535: the bounded `native-run-result/2` consumer boundary, so CG reads and
// writes the exact result bytes without naming `spine` (ADR-011 FB-05).
pub use spine::{
    read_native_run_result_bounded, temporal_disposition_json, write_clause_run_result,
    CallLocationRead, CallLocusRead, CallOriginRead, ClauseDisposition, ClauseRunRecord,
    ClauseRunReport, ClauseRunUsage, CommandErrorRecord, ConsequentSourceEvents, NativeRunLimits,
    NativeRunOutcome, NativeRunReadError, NativeRunResult, NativeRunResultError, RefusalLocusRead,
    SourceRefRead, SpanRead, TemporalRecord,
};
// The input refusal a `ReplayRefusal::Input` and a value-parity
// `ValueParityResult::RefusedInput` carry.
pub use filament_semantic_value::call::InputRefusal;
// The ADR-014 B-4 proof bound a `DeclaredDomain` wraps, its domain key and
// finite domain, and the kernel integer and interval an integer range is
// built from, re-exported so CG, which reaches FSL only through this crate
// (ADR-011 FB-05), builds a `DeclaredDomain` without its own direct
// dependency on `fsl-foundation` or `filament-exact`. With them come their
// constructors -- `DomainKey`'s `Node` and `Population` variants,
// `FiniteBound::cardinality`/`integer_range`/`depth`, `IntegerInterval::new` and `Integer`'s `From`
// conversions -- and the refusals they return. None is a T-12-governed
// constructor (ADR-011 §3 FB-05).
pub use execute::{ObjectEnvironmentFailure, Unresolved};
pub use filament_exact::{EmptyInterval, Incomplete, Integer, IntegerInterval};
pub use fsl_eval::simulation::model::{
    TaskProductTransition, TaskTransitionAction, TaskTransitionIdentity,
};
pub use fsl_foundation::bound::{
    DomainKey, DomainKind, EmptyFiniteBound, FiniteBound, FiniteBoundKind, ProofBound,
    ProofBoundRefusal,
};
pub use fsl_semantics::model::accounting::ModelNormalizationLimits;
pub use fsl_semantics::model::domain_package::DomainPackageRef;
pub use object_environment::{
    object_reference_witness, read_object_environment, write_object_environment, EnvironmentLabels,
    EnvironmentModel, EnvironmentObject, EnvironmentPopulation, ObjectEnvironmentDocument,
    ObjectEnvironmentRef, ObjectEnvironmentRefusal, ObjectEnvironmentWriteError,
    ObjectReferenceError,
};
pub use request::{
    ByteProvision, DependencyEntry, DependencyEntryWire, PackageMemberCause, ReplayPackage,
    ReplayPackageAdmissionRefusal, ReplayRequest, ReplayRequestRefusal, ReplayRequestWire,
    StageLimitNameRefusal, StageLimits, StateEnvironment,
};
pub use result::{
    read_bounded, runtime_value_path_json, CauseCodecError, DecidingElement, DisagreementCause,
    EvaluatedValue, HybridReplayRecord, HybridReplayResult, HybridReplaySettlement,
    HybridTraceDisagreement, InputArmResult, InputSettlement, ObservedTemporalParity,
    ObservedTemporalResult, ObservedTemporalSettlement, ReplayResult, SeparatingWitnessRecord,
    SeparationReason, SeparationRefusal, TemporalDecisive, Verdict, WitnessArmResult, WitnessCheck,
    WitnessFailure, WitnessSettlement,
};
pub use supplied_limits::{read_supplied_limits, SuppliedLimitsRefusal};
// FSpec FR-323: the replay target a packet, envelope and request each carry.
pub use target::{
    NamedKind, ReplayEntry, ReplayOverBinding, ReplayTarget, ReplayTargetMembers,
    RequestTargetWire, TargetMember, TargetSelectionRefusal,
};
// FR-265 and FR-268: the separation-check step and the FSpec FR-207 value
// path a separating witness record carries, re-exported so CG names them
// through this crate (ADR-011 FB-05).
pub use fsl_foundation::node_wire::NodeWireError;
pub use fsl_foundation::witness::{
    InvocationBinding, ObservationIdentity, RuntimeValuePath, SeparationStep, ValuePathStep,
    ValuePathSubject,
};
pub use scalar::{
    GeneratedFault, NativeOutcome, OperandIdentity, OperandRefusal, OperatorClaim,
    OperatorIdentity, OperatorParityReport, OperatorParityResult, ScalarAgreement, ScalarClaim,
    ScalarOperand, ScalarOperation, ScalarOperator, ScalarOutcome, ValueClaim, ValueIdentity,
};
pub use witness::{derive_state_clause_witness, DerivationRefusal};
pub use witness::{
    CanonicalAssignment, ClaimedChange, DecodeRefusal, EntryFault, EnvelopeSource, FamilyPayload,
    FrameCounterexample, FrameOperation, MalformedTranscript, NoPayload, ObservedTemporalEnvelope,
    QuantityMagnitude, ReplaySource, ValueTextError, Witness, WitnessBinding, WitnessEnvelope,
    WitnessField, WitnessPacket, WitnessRefusal, WitnessSlot, WitnessValue, WitnessValueType,
};
pub use witness_entry_name::witness_entry_name;
// FR-116: the FR-106 document, object and FR-115 frame witness types a
// frame counterexample and its replay result carry, re-exported so CG can
// name and build them through this crate (ADR-011 FB-05). The node and
// occurrence identities are built from the `NodeKey`, `Origin` and
// `Identifier` re-exported above.
pub use filament_checked::key::DeclarationKey;
pub use fsl_semantics::model::observation::{
    AdmissionFailure, AdmissionRecord, DocumentRef, FrameChange, FrameWitness, LinkDifference,
    SelectedObject,
};

// FR-122: the state-clause counterexample's replay entry, payload and
// result, and the FR-106 observation types a payload is built from and a
// refusal names, re-exported so CG can name and build them through this
// crate (ADR-011 FB-05).
pub use execute::{replay_state_clause, ClauseIdentityMismatch, StateClauseReplayResult};
pub use fsl_forms::StateClauseKind;
pub use fsl_semantics::model::observation::{
    AnchorKind, ClauseSelectionInput, ObservationForm, ObservationLimits, SelectedAnchor,
    SnapshotValue,
};
pub use witness::StateClauseCounterexample;

/// A structural node identity for a test fixture, distinct per `fill`.
#[cfg(test)]
pub(crate) fn fixture_node(fill: u8) -> NodeKey {
    NodeKey::new(
        "fixture",
        vec![filament_exact::PathSegment::Text(fill.to_string())],
        0,
    )
    .expect("a fixture node is admissible")
}

/// A parameter node's transcript name.
#[cfg(test)]
pub(crate) fn pn(node: &NodeKey) -> String {
    witness_entry_name(node).expect("a node spells")
}

/// The admitted node identity of a checked node reference.
#[cfg(test)]
pub(crate) fn adm(reference: &fsl_semantics::check::NodeRef) -> NodeKey {
    reference
        .admitted()
        .expect("a checked reference is admitted")
        .clone()
}

/// A universe identity for a test fixture, distinct per `fill`.
#[cfg(test)]
pub(crate) fn fixture_universe(fill: u8) -> filament_exact::UniverseId {
    filament_exact::UniverseId::new(fixture_node(fill))
}

/// An effective declaration identity for a test fixture, distinct per `fill`.
#[cfg(test)]
pub(crate) fn fixture_effective(fill: u8) -> filament_exact::EffectiveId {
    filament_exact::EffectiveId::new(
        None,
        filament_exact::OriginalDeclarationKey::new("fixture", format!("node{fill}")),
    )
}

#[cfg(test)]
mod redaction_tests {
    use ix_trace_rs::trace;

    use super::*;
    use filament_exact::Identifier;

    fn scalar_limits(seed: u64) -> filament_exact::ScalarLimits {
        filament_exact::ScalarLimits {
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
            certificate: None,
            profile_selections: vec![],
            package: crate::request::raw_package_descriptor(
                "fixture",
                &[("registry".to_owned(), "pkg-a".to_owned())],
            ),
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
            .get(&crate::DefinitionSourceRef::new(
                "registry".to_owned(),
                "pkg-a".to_owned(),
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
