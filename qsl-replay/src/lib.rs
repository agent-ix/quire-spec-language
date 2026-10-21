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

#![forbid(unsafe_code)]

mod admitted;
mod bounds;
mod call_requirements;
mod call_site;
mod compile;
// The facade exports the rejection vocabulary. Zone arithmetic stays
// crate-private until `check_zone_certificate` (FR-245) calls it; its tests
// and Kani harnesses exercise it directly.
#[allow(
    dead_code,
    reason = "the zone certificate checker and the FR-339 component certificate check are its first callers and have not landed yet"
)]
mod certificate;
mod composite;
mod execute;
mod hybrid;
mod identity;
mod limits;
mod model_check;
mod object_environment;
mod outcome;
mod proof_result;
mod refinement;
mod request;
mod result;
mod scalar;
pub mod spine;
pub mod statistical;
mod supplied_limits;
mod witness;

pub use admitted::{
    compile_admitted, AdmittedCompilation, AdmittedContext, AdmittedEnum, AdmittedEnumCase,
    AdmittedLookupRefusal, AdmittedRefusal, AdmittedResource,
};
pub use bounds::{BoundExceeded, ReplayLimits, DEFAULT_REPLAY_INPUT_BYTES};
pub use call_requirements::{
    run_with_requirements, CallRequirement, CallRequirements, RunWithRequirements,
};
pub use call_site::{
    call_site, CallSite, CallSiteRefusal, CallSiteSelection, ClauseName, ClauseSite, FieldName,
    FieldSite, FunctionSite, OperationSite, PopulationName, PopulationSite,
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
    check_closure, check_protocol_closure, CertificateRejection, CertificateRequest,
    ClosureCertificate, ClosureRefusal, ProductStateRef, ProtocolCertificateRequest,
    TemporalCertificateCheck,
};
pub use certificate::scc::{Component, ComponentWitness};
pub use certificate::simulation::{
    check_simulation_certificate, CertifiedEdge, CertifiedPosition, EvaluationOutcome, FailedStep,
    IncompleteLimit, SimulationCertificate, SimulationCertificateRequest, SimulationRefusal,
    UndeterminedStep,
};
pub use certificate::state_graph::{
    check_state_graph, StateGraphCertificate, StateGraphCertificateRequest,
};
pub use certificate::{CertificateLocus, CertificateRule, QueryPart};
pub use compile::{compile_package, CompiledPackage};
pub use composite::{
    CompositeEvidence, CompositeIdentity, CompositeParityClaim, CompositeParityReport,
    CompositeParityResult, EqualityOutcome, FalsifiedParity, IncompleteStage, NativeCause,
    NativeParityObservation, Refinement, VerifiedShadow, VerifiedShadowReport,
    VerifiedShadowResult,
};
pub use execute::{
    parity_obligation, replay, replay_composite_parity, replay_frame, replay_operator_parity,
    replay_value_parity, replay_with_cancel, settle_verified_shadow, BoundEntries,
    DependencySelectionsCause, Domain, FrameIdentityMismatch, FrameReplayResult,
    IdentityEncodeError, LimitAboveReader, ParityArgument, ParityBoundRefusal, ParityPreimage,
    ReplayRefusal, ScalarIdentityMismatch, ValueParityReport, ValueParityResult,
};
pub use hybrid::{
    NegativeHorizon, SolverHorizon, SolverMethod, SolverOutcome, SolverResult, SolverStop,
};
pub use identity::{
    DeclaredDomain, EmptyQualifiedName, ObligationIdentity, ProfileSelection, QualifiedName,
    RawSourceRef, TracePosition,
};
pub use limits::{AccountingLimits, CallerLimits};
pub use model_check::counterexample::{
    deadlock_freedom_clause, undefined_finding, ClaimPredicate, CounterexampleKind, Lasso,
    MappingSite, ModelStep, TemporalCounterexample, UndefinedEvaluation, UndefinedLocus,
    UndefinedSite,
};
pub use model_check::graph_settle::{
    settle_state_graph, GraphReplayResult, GraphReplaySettlement, StateGraphMethod,
    StateGraphRecord,
};
pub use model_check::hyper_counterexample::{
    HyperCounterexample, HyperCounterexampleKind, HyperLocus, HyperStep, HyperTrace,
    HyperUndefined, StepMark,
};
pub use model_check::hyper_outcome::{HyperOutcome, HyperReductionCause};
pub use model_check::hyper_replay::{replay_hyper_trace_tuple, HyperReplay};
pub use model_check::identity::{ClauseIdentity, ClauseRootRefusal};
pub use model_check::item::{Instance, ItemRefusal, ModelCheckItem, Translation};
pub use model_check::limits::{
    ModelCheckLimit, ModelCheckLimits, ReachedLimit, DEFAULT_MAX_AUTOMATON_STATES,
    DEFAULT_MAX_RELATION_TUPLES, DEFAULT_MAX_WALK_STEPS, DEFAULT_MAX_WITNESS_SET,
    DEFAULT_WITNESS_SAMPLES, DEFAULT_WITNESS_SEED,
};
pub use model_check::outcome::{
    MemoryCause, ModelCheckOutcome, ReductionCause, RunStats, UndecidedCause,
};
pub use model_check::refinement_replay::{replay_refinement_trace, RefinementTraceSettlement};
pub use model_check::refinement_settle::{
    settle_refinement, settle_refinement_refusal, settle_refinement_safety, RefinementRecord,
};
pub use model_check::replay::{
    replay_model_trace, replay_protocol_trace, ModelTraceRefusal, ModelTraceReplayResult,
    ModelTraceSettlement, ModelTraceSubject, ModelTraceValue, ProtocolTraceSubject,
    StrongFairnessHint,
};
pub use model_check::settle::{
    settle_model_check, settle_protocol_check, Execution, ModelCheckRecord, ResultLabel, Truth,
    EXPLICIT_STATE,
};
pub use model_check::state_graph::{
    GraphUndefined, GraphWitness, ModelPath, OpenCause, RunEnd, StateGraphInstanceOutcome,
    StateGraphOutcome, WitnessSource,
};
pub use model_check::step_settle::{
    settle_step_simulation, BackendCertificate, StepBackendOutcome, StepSimulationSettlement,
    STEP_INDUCTION_DEPTH,
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
pub use outcome::{
    ArtifactKind, ExecuteResult, ItemCause, ItemLabel, Operation, OutcomeArtifact,
    OutcomeDiagnostic, OutcomeDocument, OutcomeItem, OutcomeLocus, OutcomeStage, OutcomeWriteError,
    ParityReason, ResultLimit, ResultValue, OUTCOME_FORMAT,
};
pub use proof_result::{
    read_backend_provider_envelope, AppliedReduction, BackendProviderSource, Certification,
    DeclineCode, EmptyEnvelopeSet, IncompleteCause, InconclusiveCause, InstanceBound, MemoryBound,
    MemoryBoundMember, ProofBasis, ProofRefusalCause, ProofResultEnvelope, ProofResultRefusal,
    Proviso, ReductionKind, ReportedInconclusiveCause, SettlementBasis, StatedInstanceBound,
    StatedMemoryBound, StatisticalRequestRefusal, SymmetryBrokenCause, TerminalRecord,
    TerminalRecordRefusal, TerminalValue, UnavailabilityCause,
};
pub use qsl_foundation::{Code, InternalFault, RequestIndex};
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
// `qsl_foundation`'s node id and digest-record vocabulary -- re-exported so
// CG, which reaches QSL only through this crate (ADR-011 FB-05), builds one
// without its own direct dependency on `quire-exact` or `qsl-foundation`.
// These re-exports add constructors CG can call directly:
// `DigestRecord::mint`, `WireNodeId::from_digest`/`from_hex`,
// `SourceIdentity::new`, `Identifier::new` and `Origin::new`/`Role::new`
// among them, and with `Origin` the occurrence key's own
// `OccurrenceKey::new(WireNodeId, Origin)` becomes callable. None is a
// T-12-governed constructor (ADR-011 §3 FB-05), so no rule is broken; CG
// needs exactly these to build the requests `call_site` and `replay`
// (FR-098) and `replay_frame` (FR-116) read.
pub use qsl_foundation::digest::{ByteDigest, DigestDomain, DigestRecord, WireNodeId};
// The ADR-013 O-16 category a `Verdict` and each arm result carry, re-exported
// so CG names `Category::Success` and `Category::Violation` through this crate.
pub use qsl_foundation::diagnostic::Category;
pub use qsl_foundation::SourceIdentity;
pub use quire_exact::{Identifier, NodeKey, Origin, Role, ScalarLimits, Value};
// FR-395: the types of `run`'s signature and result that the root did not
// export, so a consumer calls `run_with_requirements` and reads its result
// without naming `qsl_replay::spine` or `qsl_semantics`.
pub use qsl_semantics::library::PackageId;
pub use qsl_semantics::model::object_environment::ObjectEnvironment;
pub use qsl_semantics::value::supplied::SuppliedLimits;
pub use quire_exact::{Cancel, CancelCause};
pub use spine::{
    Call, CallArgument, CallIncomplete, CallLocus, CallOutcome, CallRefusal, CallValue,
    CompileRefusal, RunRefusal, SpineLimits,
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
    ReplayRequestWire, StageLimits, StateEnvironment,
};
pub use result::{
    read_bounded, CauseCodecError, DisagreementCause, EvaluatedValue, InputArmResult,
    InputSettlement, ReplayResult, SeparatingWitnessRecord, SeparationReason, SeparationRefusal,
    Verdict, WitnessArmResult, WitnessCheck, WitnessFailure, WitnessSettlement,
};
pub use supplied_limits::{read_supplied_limits, SuppliedLimitsRefusal};
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
    ScalarOperand, ScalarOperation, ScalarOperator, ScalarOutcome, ValueIdentity,
};
pub use witness::{
    CanonicalAssignment, ClaimedChange, DecodeRefusal, EntryFault, FamilyPayload,
    FrameCounterexample, FrameOperation, MalformedTranscript, NoPayload, QuantityMagnitude,
    ReplaySource, ValueTextError, Witness, WitnessBinding, WitnessEnvelope, WitnessField,
    WitnessPacket, WitnessRefusal, WitnessSlot, WitnessValue, WitnessValueType,
};
// FR-116: the FR-106 document, object and FR-115 frame witness types a
// frame counterexample and its replay result carry, re-exported so CG can
// name and build them through this crate (ADR-011 FB-05). The node and
// occurrence identities are built from the `WireNodeId`, `Origin` and
// `Identifier` re-exported above.
pub use qsl_semantics::model::key::DeclarationKey;
pub use qsl_semantics::model::observation::{
    AdmissionFailure, AdmissionRecord, DocumentRef, FrameChange, FrameWitness, LinkDifference,
    SelectedObject,
};

// FR-122: the state-clause counterexample's replay entry, payload and
// result, and the FR-106 observation types a payload is built from and a
// refusal names, re-exported so CG can name and build them through this
// crate (ADR-011 FB-05).
pub use execute::{replay_state_clause, ClauseIdentityMismatch, StateClauseReplayResult};
pub use qsl_forms::StateClauseKind;
pub use qsl_semantics::model::observation::{
    AnchorKind, ClauseSelectionInput, ObservationForm, SelectedAnchor, SnapshotValue,
};
pub use witness::StateClauseCounterexample;

#[cfg(test)]
mod redaction_tests {
    use ix_trace_rs::trace;

    use super::*;
    use qsl_foundation::digest::{ByteDigest, DigestDomain, DigestRecord, WireNodeId};
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

    /// FR-073-AC-3 (TC-211): a `stale_dependency`/`byte-digest-mismatch`
    /// refusal from the replay request, and a malformed-transcript refusal
    /// from the witness envelope, each render with no unredacted content --
    /// and a separately, validly constructed value carrying the same
    /// content stays fully readable through its typed accessor.
    #[trace("TC-211", "FR-073-AC-3")]
    #[test]
    fn tc_211_refusal_causes_redact_while_typed_accessors_stay_readable() {
        // Half 1: replay request byte-digest mismatch.
        let entry_x: Vec<u8> = (0..4096u32).map(|i| (i % 241) as u8).collect();
        let correct_digest = ByteDigest::of(&entry_x).as_bytes();
        let mut mismatched_bytes = entry_x.clone();
        mismatched_bytes[0] ^= 0xFF;

        let source_digest_record = DigestRecord::mint(DigestDomain::SourceBytesV1, correct_digest);
        let wire = ReplayRequestWire {
            profile_selections: vec![],
            package_id: (
                Some(DigestDomain::PackageSemanticV2.as_str().to_owned()),
                DigestRecord::mint(DigestDomain::PackageSemanticV2, [1; 32]).hex(),
            ),
            source_digests: vec![(
                "registry".to_owned(),
                "pkg-a".to_owned(),
                "git".to_owned(),
                "rev-1".to_owned(),
                Some(DigestDomain::SourceBytesV1.as_str().to_owned()),
                source_digest_record.hex(),
            )],
            dependencies: Vec::new(),
            selected_function: QualifiedName::new(vec![Identifier::new("f").unwrap()]).unwrap(),
            source: ReplaySource::Input(vec![CanonicalAssignment {
                parameter: WireNodeId::from_digest([9; 32]),
                value: WitnessValue::Integer(1),
            }]),
            obligation_identity: [2; 32],
            backend: "kani-backend-1".to_owned(),
            state_environment: StateEnvironment::new(vec![]),
            accounting_limits: scalar_limits(1),
            stage_limits: std::collections::BTreeMap::new(),
            supplied_limits: None,
            object_environment: None,
            declared_domains: Vec::new(),
            byte_provision: vec![(
                Some(DigestDomain::SourceBytesV1.as_str().to_owned()),
                source_digest_record.hex(),
                mismatched_bytes,
            )],
        };
        let refusal = ReplayRequest::decode(wire, crate::ReplayLimits::default()).unwrap_err();
        let rendered = format!("{refusal:?} {refusal}");
        assert!(!contains_bytes(&rendered, &entry_x));

        // A separately, validly constructed request whose byte provision
        // carries entry X's exact bytes under their own correct digest (no
        // mismatch) still returns the full content through the typed
        // accessor.
        let valid_wire = ReplayRequestWire {
            profile_selections: vec![],
            package_id: (
                Some(DigestDomain::PackageSemanticV2.as_str().to_owned()),
                DigestRecord::mint(DigestDomain::PackageSemanticV2, [1; 32]).hex(),
            ),
            source_digests: vec![(
                "registry".to_owned(),
                "pkg-a".to_owned(),
                "git".to_owned(),
                "rev-1".to_owned(),
                Some(DigestDomain::SourceBytesV1.as_str().to_owned()),
                source_digest_record.hex(),
            )],
            dependencies: Vec::new(),
            selected_function: QualifiedName::new(vec![Identifier::new("f").unwrap()]).unwrap(),
            source: ReplaySource::Input(vec![CanonicalAssignment {
                parameter: WireNodeId::from_digest([9; 32]),
                value: WitnessValue::Integer(1),
            }]),
            obligation_identity: [2; 32],
            backend: "kani-backend-1".to_owned(),
            state_environment: StateEnvironment::new(vec![]),
            accounting_limits: scalar_limits(1),
            stage_limits: std::collections::BTreeMap::new(),
            supplied_limits: None,
            object_environment: None,
            declared_domains: Vec::new(),
            byte_provision: vec![(
                Some(DigestDomain::SourceBytesV1.as_str().to_owned()),
                source_digest_record.hex(),
                entry_x.clone(),
            )],
        };
        let valid_request =
            ReplayRequest::decode(valid_wire, crate::ReplayLimits::default()).unwrap();
        let looked_up = valid_request
            .byte_provision()
            .get(source_digest_record)
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

    /// Whether `haystack` contains `needle`'s bytes reinterpreted as a
    /// lossy UTF-8 string (the shape a `{:?}`/`{}` rendering of a `Vec<u8>`
    /// field would produce if it leaked raw bytes through `derive(Debug)`).
    fn contains_bytes(haystack: &str, needle: &[u8]) -> bool {
        let needle_text = String::from_utf8_lossy(needle);
        haystack.contains(needle_text.as_ref())
    }
}
