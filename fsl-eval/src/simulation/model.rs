// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-120: [`ModelSystem`], the FR-101 [`TransitionSystem`] of a checked
//! package's state family, and the entries that explore and sample it.
//!
//! A model state is one closed population state. Its successors are every
//! enabled application of an operation (receiver, then one value per declared
//! parameter) times every post-state inside the operation's frame whose
//! postcondition holds. Candidates are generated lazily, one at a time, and
//! counted against [`ExpansionLimits::max_candidates`]. Every effect a model
//! describes is data: a [`StepEffect`] holds the creations, deletions and field
//! changes of one step, and nothing outside the returned values changes.
//!
//! A random parameter (FR-185) ranges over its support, and FR-187's
//! [`ModelSystem::actions`] and [`ModelSystem::weighted_steps`] give the
//! probabilistic structure of a state's successors.

mod application;
mod behaviours;
mod cells;
mod constraint;
mod contract;
mod contracts;
mod domain;
pub(crate) mod encode;
mod footprints;
mod frames;
mod input;
pub use input::{
    BodyReads, ModelCheckInput, ModelEvaluator, ModelTerminal, ModelTerminalDeclaration,
    PackageModelInput, PackageRewards, PendingClauseForm, PendingPackageForm, StateClauseEntry,
    TerminalWhen,
};
mod operation_coverage;
pub use operation_coverage::{
    ModelOperation, ModelOperationCase, ModelOperationCoverage, ModelOperationCoverageEntry,
    ModelOperationCoverageLimit, ModelOperationCoverageLimits, ModelOperationCoverageReason,
    ModelOperationCoverageReasonKind, ModelOperationCoverageStatus,
    ModelOperationCoverageSummaryStatus, model_operation_coverage,
};
mod model_based_test;
pub use model_based_test::{
    ClauseReading, read_clause,
    ModelBasedCaseProvenance, ModelBasedCounterexample, ModelBasedCoverage,
    ModelBasedPartitionCoverage, ModelBasedProvenance, ModelBasedRunLimits,
    ModelBasedSizeBoundedRoot, ModelBasedTestCause, ModelBasedTestChecks, ModelBasedTestCounts,
    ModelBasedTestDefect, ModelBasedTestDiscardCount, ModelBasedTestDiscardReason,
    ModelBasedTestError, ModelBasedTestEvidence, ModelBasedTestRecord, ModelBasedTestRefusal,
    ModelBasedTestRequest, ModelBasedTestResult, ModelBasedTestSubject, ModelBasedValueBoundCut,
    ModelBasedValueBoundCuts, NativeAdmission, RootAdmission, run_model_based_state_invariant,
    run_model_based_state_invariant_with_value_bits, run_model_based_test,
    run_model_based_test_with_system, run_model_based_test_with_value_bits,
};
mod key_budget;
mod letters;
pub use behaviours::{BehaviourState, BehavioursEnd, model_behaviours};
pub use key_budget::{ModelKeyBudget, ModelKeyError, ModelKeyExpansionError};
pub(crate) mod probabilistic;
mod reference_bindings;
mod refine;
mod state_predicate;
mod symbolic;
mod symmetry;
mod task_automaton;
mod task_key;
mod task_product;
mod terminal;
mod timed;
mod walk;

use std::collections::{BTreeMap, BTreeSet};

use filament_checked::key::DeclarationKey;
use filament_exact::{
    EffectiveId, Incomplete, Integer, IntegerInterval, NodeKey, ObjectReference, Refusal,
    ScalarLimits, Undefined, ValueType,
};
use filament_semantic_value::catalog::{CatalogCode, InternalFault};
use filament_semantic_value::declaration::TypeEnvironment;
use filament_semantic_value::model_transition::ModelTransition;
use fsl_foundation::IdentityLimits;
use fsl_foundation::diagnostic::Code;
use fsl_foundation::selection::DefinitionRef;
use fsl_package::CheckedPackage;
use fsl_semantics::check::OperationKey;
use fsl_semantics::family::ClassifyFailure;
use fsl_semantics::model::accounting::ModelNormalizationLimits;
use fsl_semantics::model::clause_observation::Observation;
use fsl_semantics::model::domain_package::OperationEffect;
use fsl_semantics::model::observation::{
    AdmissionFailure, AdmissionRecord, AnchorKind, DocumentRef, ObservationLimits, SelectedAnchor,
};
use fsl_semantics::model::operation::OperationDeclaration;
use fsl_semantics::model::state::{ModelState, StateField, StateModel};
use quire_canonical::Limits as CanonicalLimits;

use crate::model_subject::ModelInstance;
use crate::simulation::expansion::{Expansion, ExpansionStop, StateFindings};
use crate::simulation::explore::{Exploration, Limits, TransitionSystem, explore_request};
use crate::simulation::footprint::{Footprint, Location};
use crate::simulation::not_simulated::NotSimulated;
use crate::simulation::order::{Expanded, expand, sorted_initial};
use crate::simulation::protocol::WeakAccessShape;
use crate::simulation::reduction::{
    ConstraintValue, OfferedReductions, Permutation, SymmetryGroup,
};
use crate::simulation::sample::{SampleRequest, sample_request, sample_request_until};
use crate::simulation::trace::{ReplayError, SampleProvenance, Step, StopReason, Trace, replay};
use crate::value::QualifiedName;

pub(crate) use application::{FirstApplication, Observed, ObservedIdentities, OperationRef};
pub use cells::Cell;
pub use contract::{
    ContractClause, ContractClauseKind, DeclaredStates, DeclaredStatesRefusal, OperationCases,
    OperationStep, PreconditionCase, UndecidedContract,
};
pub(crate) use contracts::{Verdict, verdict};
use encode::declaration_text;
pub use encode::{
    ModelKey, canonical_json, declaration_identity_text, encode_reference,
    reference_canonical_text, typed_value_json, value_json,
};
pub use footprints::{FootprintError, FootprintViolation, WithinStop};
pub use frames::judge::{
    AbstractApplication, Judged, Judgement, Rejection, ResolvedStep, operation_identity_text,
};
pub(crate) use reference_bindings::{ReferenceBinding, ReferenceBindings};
mod hyper_letters;
pub use hyper_letters::CheckedHyperLetters;
pub use letters::{ClauseLetters, DecisiveObservation, LettersRefusal};
pub use probabilistic::{
    Action, Draw, NotMarkov, ScheduledIdentity, TransitionId, UndecidedSuccessor, Unsteppable,
    WeightedStep, same_rewards, same_values,
};
pub use refine::{RowReading, StateObservation};
pub use state_predicate::PredicateReading;
pub(crate) use symbolic::{ParameterValues, ResultValues, SymbolicOperation, SymbolicSubject};
pub use terminal::{TerminalClass, TerminalModel};
pub use task_automaton::{
    AutomatonClock, AutomatonDiagonal, BoundedSubtraction, CertificationRefusal, ClockAtom,
    ClockCarry, ClockReset, ClockRole, EdgeKind, Guard, JobPhase, LiveJob, LiveJobKey, LocationInfo,
    LocationKey, Overflow, Successors, TaskAutomaton, TaskAutomatonRefusal, TaskControl, TaskEdge,
    TaskLocation, TaskTemplate, delay_valuation,
};
pub use task_key::{
    TaskKeyControl, TaskKeyIdentityRefusal, TaskKeyJob, TaskKeyPending, write_task_control,
};
pub use task_product::{
    AllowancePhase, CriticalityMode, TaskProductClock, TaskProductClockKind,
    TaskProductDerivedStep, TaskProductJob, TaskProductModeSwitch, TaskProductMoveRefusal,
    TaskProductPlan, TaskProductPlanRefusal, TaskProductRelease,
    TaskProductReplayRefusal, TaskProductSourceStep, TaskProductState, TaskProductStateKeyRefusal,
    TaskProductSystem, TaskProductTask, TaskProductTrace, TaskProductTraceStep,
    TaskProductTransition, TaskTransitionAction, TaskTransitionIdentity,
};
pub use timed::{
    replay_timed_sample, sample_timed, sample_timed_race, BehaviourLetters, BehaviourReading,
    ClockPlace, ClockValuation, DEFAULT_MAX_CLOCKS, DelayInterval, GuardedStep, MoveRefusal,
    PlaceConstraint, ScheduledWindow, TaskArrivalKind, TaskReleaseBinding, TaskTimingRow, TimeLock,
    TimedBehaviour, TimedPosition, TimedRace, TimedRaceDraw, TimedRaceStep, TimedRaceStop, TimedSample,
    TimedSampleDisagreement, TimedSampleReplay, TimedSampleStop, TimedSchedulerChoice, TimedSite,
    TimedState, TimedStop, TimedSubjectRefusal, TimedSystem, TimedTerminalClass,
};
pub(crate) use walk::{Applied, Budget};

/// `resource_exhausted`/`insufficient-next-charge`: a candidate count, or an
/// evaluation meter, ran out (FR-120 "Post-states", "Contract clauses").
pub(crate) const EXHAUSTED: CatalogCode =
    CatalogCode::new("resource_exhausted", "insufficient-next-charge");
/// `runtime_invariant`/`established-invariant-broken`: the simulator built an
/// input itself, so a refusal of it is a broken internal invariant.
pub(crate) const BROKEN: CatalogCode =
    CatalogCode::new("runtime_invariant", "established-invariant-broken");

/// One simulated population and its finite key set (FR-120 "Inputs").
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PopulationUniverse {
    /// The population's declaration key.
    pub population: DeclarationKey,
    /// The object keys the population may hold.
    pub keys: Vec<String>,
}

/// The work one expansion may do (FR-120 "Post-states").
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ExpansionLimits {
    /// The most candidates, and (candidate, result) pairs, one expansion
    /// generates before it stops with `resource_exhausted`.
    pub max_candidates: u64,
}

/// Everything [`ModelSystem::new`] reads (FR-120 "Inputs").
#[derive(Clone)]
pub struct ModelSimulationRequest<'a> {
    /// What the check reads of the S4 package (FR-531, FR-940).
    pub input: ModelCheckInput<'a>,
    /// FR-056's package input: domain package bytes by the identity each declares.
    pub packages: &'a fsl_semantics::model::intake::PackageInput,
    /// The normalization limits FR-106 admission takes.
    pub normalization: ModelNormalizationLimits,
    /// An FR-106 snapshot provision.
    pub snapshots: &'a fsl_semantics::model::observation::DocumentProvision,
    /// The initial states: documents of `snapshots`.
    pub initial: Vec<DocumentRef>,
    /// For each simulated population, its finite key set.
    pub universes: Vec<PopulationUniverse>,
    /// FR-106's document limits.
    pub observation: ObservationLimits,
    /// The per-expansion candidate bound.
    pub expansion: ExpansionLimits,
    /// The evaluation meter budget; each clause evaluation gets a fresh meter.
    pub meter: ScalarLimits,
    /// The ceiling on the canonical bytes of each package emission the
    /// subject's analysis makes (`emit.output_bytes`, FR-264 Behavior 5).
    pub emit: fsl_package::EmitLimits,
}

impl<'a> ModelSimulationRequest<'a> {
    /// The source package the request is over, for an engine with no
    /// package form yet; `None` on the package route.
    pub fn source_package(&self) -> Option<&'a CheckedPackage> {
        self.input.source_package()
    }

    /// The admitted package the request is over on the package route
    /// (FR-940); `None` on the source route.
    pub fn admitted_package(&self) -> Option<&'a fsl_package::CheckedPackageV2> {
        self.input.admitted_package()
    }
}

/// A model state with its anchor (FR-120 "State, state key and anchor"). The
/// anchor, and the effect of the step that reached the state, are not part of
/// its key.
#[derive(Clone, Debug)]
pub struct ModelSimState {
    state: ModelState,
    anchor: SelectedAnchor,
    reached_by: Option<StepEffect>,
}

impl ModelSimState {
    /// The closed population state.
    pub fn state(&self) -> &ModelState {
        &self.state
    }

    /// The initialization or handler this state is anchored to.
    pub fn anchor(&self) -> &SelectedAnchor {
        &self.anchor
    }

    /// The effect of the step that reached this state; `None` for an initial
    /// state.
    pub fn reached_by(&self) -> Option<&StepEffect> {
        self.reached_by.as_ref()
    }
}

/// One field a step changed (FR-120 "Effects as data"), each value in
/// FR-181's typed canonical form; `None` is an absent `?` field.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FieldChange {
    /// The object whose field changed.
    pub object: ObjectReference,
    /// The field's declared name.
    pub field: String,
    /// The field's canonical text before the step.
    pub pre: Option<String>,
    /// The field's canonical text after the step.
    pub post: Option<String>,
}

/// The effect of one step, as data (FR-120 "Effects as data"): objects in
/// ascending reference-key order and fields in declaration order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StepEffect {
    /// The objects the step created.
    pub created: Vec<ObjectReference>,
    /// The objects the step deleted.
    pub deleted: Vec<ObjectReference>,
    /// The fields the step changed on surviving objects.
    pub changed: Vec<FieldChange>,
    /// The canonical text of the transition's result, or `None`.
    pub result: Option<String>,
    /// The receiver's clock collections the step wrote whole (FR-230
    /// "Collection-valued writes"), each with the provenance of every slot
    /// of its post-state value.
    pub written: Vec<WrittenSlots>,
}

/// A clock collection a step wrote whole, and where each of its post-state
/// slots comes from.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WrittenSlots {
    /// The receiver's collection field.
    pub field: String,
    /// One provenance for each slot of the post-state value, in slot order.
    pub provenance: Vec<crate::value::Provenance>,
}

/// How a clause evaluation that decided no Boolean ended (FR-120 "Contract
/// clauses"), kept as data so a finding is `Clone + Eq`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ClauseOutcome {
    /// A kernel evaluation was undefined.
    KernelUndefined(Undefined),
    /// A kernel evaluation was refused.
    KernelRefused(Refusal),
    /// The family's evaluation-time result was undefined, for this reason.
    FamilyUndefined(filament_semantic_value::catalog::UndefinedReason),
    /// The family's evaluation-time result was a refusal with this code.
    FamilyRefused(CatalogCode),
    /// A charge was unavailable.
    Incomplete(Incomplete),
    /// The clause completed a value that is no Boolean.
    NotBoolean,
    /// The evaluation stopped before deciding, with this cause: a broken
    /// evaluation call (`runtime_invariant`/`established-invariant-broken`)
    /// or the run's cancellation (`cancelled`/`caller-cancelled`). Only a
    /// state evaluated for its invariant findings alone reports it (FR-634
    /// "Invariant findings"); an expansion stops instead.
    Stopped(CatalogCode),
}

impl ClauseOutcome {
    /// `resource_exhausted`/`insufficient-next-charge` for an incomplete
    /// evaluation; `None` otherwise.
    pub fn incomplete_code(&self) -> Option<CatalogCode> {
        matches!(self, Self::Incomplete(_)).then_some(EXHAUSTED)
    }

    /// The undefined value's cause (ADR-018 UE-2) for an undefined
    /// evaluation; `None` for every other outcome.
    pub fn undefined_cause(&self) -> Option<crate::temporal::UndefinedCause> {
        match self {
            Self::KernelUndefined(undefined) => {
                Some(crate::temporal::UndefinedCause::Kernel(undefined.clone()))
            }
            Self::FamilyUndefined(reason) => Some(crate::temporal::UndefinedCause::Family(
                filament_semantic_value::catalog::UndefinedRecord {
                    reason: *reason,
                    fields: BTreeMap::new(),
                },
            )),
            Self::KernelRefused(_)
            | Self::FamilyRefused(_)
            | Self::Incomplete(_)
            | Self::NotBoolean
            | Self::Stopped(_) => None,
        }
    }
}

/// A contract conjunction an expansion evaluated undefined (ADR-018 UE-6):
/// the clause, the transition when it is that transition's guard, and the
/// cause.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UndefinedContract {
    /// The `pre` or `post` clause's name.
    pub clause: String,
    /// The guarded transition, for a precondition; `None` for a
    /// postcondition.
    pub guard_of: Option<ModelTransition>,
    /// The undefined value's cause.
    pub cause: crate::temporal::UndefinedCause,
}

impl ModelFinding {
    /// This finding as an undefined contract conjunction, when it is one.
    pub fn undefined_contract(&self) -> Option<UndefinedContract> {
        let Self::ContractUndetermined {
            transition,
            candidate,
            clause,
            evaluation,
        } = self
        else {
            return None;
        };
        Some(UndefinedContract {
            clause: clause.clone(),
            guard_of: candidate.is_none().then(|| transition.clone()),
            cause: evaluation.undefined_cause()?,
        })
    }

    /// This finding as a protocol expression evaluated undefined (ADR-027
    /// TS-4, FSpec FR-426 "Undefined protocol expressions"), when it is one:
    /// the node's static declaration path and the cause.
    pub fn undefined_protocol_expression(
        &self,
    ) -> Option<(String, crate::temporal::UndefinedCause)> {
        let Self::ProtocolExpressionUndetermined { node, evaluation } = self else {
            return None;
        };
        Some((node.clone(), evaluation.undefined_cause()?))
    }

    /// Whether this finding is a contract conjunction or a protocol
    /// expression that was refused, so the successor relation is not
    /// exactly known (ADR-018 V-6).
    pub fn is_refused_contract(&self) -> bool {
        matches!(
            self,
            Self::ContractUndetermined { evaluation, .. }
                | Self::ProtocolExpressionUndetermined { evaluation, .. }
                if evaluation.undefined_cause().is_none()
        )
    }
}

/// What a state or a successor set found (FR-120 "Contract clauses",
/// "Invariants"); FR-101 records these for expanded states.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ModelFinding {
    /// EN-1 found one or more weak-memory access shapes at this reachable
    /// protocol state (FR-220, ADR-025 MA-4).
    WeakAccessShape(Box<WeakAccessShape>),
    /// EN-1 deferred a local ordered access until reachable-state
    /// classification completes (FR-220).
    WeakAccessDeferred {
        /// The names this attempt transition accesses while classified local.
        names: Vec<Location>,
        /// The local-ordering mismatch to report if no name is shared.
        shape: Box<WeakAccessShape>,
    },
    /// EN-1 classified this name shared in a reachable state (FR-220).
    WeakAccessShared {
        /// The canonical model location shared by enabled footprints.
        location: Location,
    },
    /// An invariant was false for an object of the state.
    InvariantViolated {
        /// The clause's name.
        clause: String,
        /// The object it was false for.
        self_object: ObjectReference,
        /// The state's anchor.
        anchor: SelectedAnchor,
    },
    /// An invariant could not be decided for an object of the state.
    InvariantUndetermined {
        /// The clause's name.
        clause: String,
        /// The object it could not be decided for.
        self_object: ObjectReference,
        /// The state's anchor.
        anchor: SelectedAnchor,
        /// Why it was not decided.
        evaluation: ClauseOutcome,
    },
    /// A protocol step's constraint could not be decided (ADR-027 TS-4).
    ProtocolExpressionUndetermined {
        /// The node's static declaration path.
        node: String,
        /// Why it was not decided.
        evaluation: ClauseOutcome,
    },
    /// A `pre` or `post` clause of an application could not be decided.
    ContractUndetermined {
        /// The application the clause belongs to.
        transition: ModelTransition,
        /// The candidate's canonical state key for a postcondition; `None` for a
        /// precondition.
        candidate: Option<crate::simulation::StateKey>,
        /// The clause's name.
        clause: String,
        /// Why it was not decided.
        evaluation: ClauseOutcome,
    },
}

/// A sampled trace and the effect of each of its steps (FR-120 "Outputs").
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelTrace {
    /// FR-181's transition identities, canonical state keys and findings.
    pub trace: Trace<ModelTransition, ModelFinding>,
    /// One effect per step, derived by replaying the trace.
    pub effects: Vec<StepEffect>,
    /// A partition invocation that was checked but deliberately not applied.
    pub attempted: Option<ModelTransition>,
    /// Boundary partitions for finite integer domains, in declaration order.
    pub boundary_partitions: Vec<ModelBoundaryPartition>,
}

/// Which edge of a finite integer domain a generated partition represents.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ModelBoundaryKind {
    /// The empty text value when the declared minimum length is zero.
    TextEmpty,
    /// The shortest, least-content text at the declared minimum length.
    TextLowerEndpoint,
    /// The least text of length one when that length is admitted.
    TextSingleton,
    /// The longest, least-content text at the declared maximum length.
    TextUpperEndpoint,
    /// A normalization witness retained by the declared text profile.
    TextNormalization,
    /// The least value of a size-bounded domain.
    SizeLeast,
    /// A size-one value in the pinned size order.
    SizeUnit,
    /// The least member of a size-bounded domain in canonical value order.
    SizeExtremeLow,
    /// The greatest member of a size-bounded domain in canonical value order.
    SizeExtremeHigh,
    /// The representable value immediately below the inclusive lower bound.
    LowerRefusal,
    /// The inclusive lower bound.
    LowerEndpoint,
    /// The inclusive upper bound.
    UpperEndpoint,
    /// The representable value immediately above the inclusive upper bound.
    UpperRefusal,
}

/// A generated endpoint or adjacent refusal value for one model input domain.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelBoundaryPartition {
    /// The model declaration node that owns this domain.
    pub node: NodeKey,
    /// The path from the declaration node to this input domain.
    pub path: Vec<u32>,
    /// The declared inclusive integer domain.
    pub domain: IntegerInterval,
    /// The boundary represented by `value`.
    pub kind: ModelBoundaryKind,
    /// The generated endpoint or adjacent refusal value.
    pub value: Integer,
    /// The generated text value for a Text partition; `value` then records
    /// its selected-profile length.
    pub text_value: Option<String>,
    /// The comparison and length profile for a Text partition.
    pub text_profile: Option<filament_exact::TextProfile>,
    /// How this partition was accounted for in a completed model-based run.
    pub status: Option<ModelPartitionStatus>,
}

/// The case-level disposition of one generated root partition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ModelPartitionStatus {
    /// An evaluated passing or failing case held this value at the root.
    Covered {
        /// The first zero-based generated case that covered the partition.
        first_case: u64,
    },
    /// Native admission refused the partition value as required.
    Refused {
        /// The zero-based generated case whose admission refused the value.
        first_case: u64,
    },
    /// No case could establish coverage for this partition.
    Uncovered {
        /// The reason coverage could not be established.
        reason: ModelPartitionUncoveredReason,
        /// The limits that bounded the reachability claim.
        bounds: Vec<String>,
    },
}

/// Why a model-based run could not generate or evaluate a root partition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ModelPartitionUncoveredReason {
    /// No enabled state accepts the operation argument in the bounded graph.
    PreconditionFalse,
    /// Reachability proved no state contains the field value.
    Unreachable,
    /// No state within the requested bound contains the field value.
    UnreachableWithinBound,
    /// A state exists, but the partition did not receive a generated case.
    NotGenerated,
    /// Reachability stopped at the named exploration limit.
    Undetermined {
        /// The FR-461 limit that stopped exploration.
        limit: String,
        /// The value charged against that limit.
        value: u128,
    },
}

/// Why [`sample_model`] returned no trace.
#[derive(Debug, Eq, PartialEq, thiserror::Error)]
pub enum ModelSampleError {
    /// The request was refused before sampling (FR-101).
    #[error(transparent)]
    NotSimulated(#[from] NotSimulated),
    /// The sampled trace does not replay against the system that sampled it.
    #[error(transparent)]
    Replay(ReplayError<ModelTransition>),
}

/// Why [`shrink_model_trace`] could not retain a failing prefix.
#[derive(Debug, Eq, PartialEq, thiserror::Error)]
pub enum ModelShrinkError {
    /// The supplied trace did not reproduce the caller's failure oracle.
    #[error("the supplied trace does not reproduce the failure")]
    NotCounterexample,
    /// An accepted candidate did not replay against the model that generated it.
    #[error(transparent)]
    Replay(ReplayError<ModelTransition>),
    /// The trace's identities could not be encoded during replay.
    #[error(transparent)]
    NotSimulated(NotSimulated),
}

/// One operation of an object type's effective view, with the texts and keys
/// that name it.
struct Application {
    name: String,
    declaring: EffectiveId,
    declaration: OperationDeclaration,
    text: String,
}

/// A checked state clause, resolved for evaluation.
struct ClauseInfo {
    name: String,
    qualified: QualifiedName,
    identity: NodeKey,
    context: EffectiveId,
    operation: Option<(EffectiveId, String)>,
}

/// The package's state clauses by kind, each in ascending name order.
#[derive(Default)]
struct Clauses {
    invariants: Vec<ClauseInfo>,
    preconditions: Vec<ClauseInfo>,
    postconditions: Vec<ClauseInfo>,
}

/// FR-120: the transition system of a checked package's state family.
pub struct ModelSystem<'a> {
    /// The identity of the one domain package this system is over, when it
    /// is over one (FR-140's `WrongSystem` check reads it).
    over: Option<String>,
    types: &'a TypeEnvironment,
    operations: &'a fsl_semantics::model::operation::OperationTable,
    /// What the system reads of its package; its `terminal` declarations
    /// (FR-411) resolved when the system was built.
    input: ModelCheckInput<'a>,
    model: StateModel,
    universes: BTreeMap<DeclarationKey, Vec<String>>,
    initial: Vec<ModelSimState>,
    /// FR-1000: subject-specific resolutions for checked reference literals.
    reference_bindings: ReferenceBindings,
    clauses: Clauses,
    applications: BTreeMap<EffectiveId, Vec<Application>>,
    fields: BTreeMap<DeclarationKey, Vec<StateField>>,
    populations: Vec<fsl_semantics::model::state::StatePopulation>,
    limits: ExpansionLimits,
    meter: ScalarLimits,
    footprints: footprints::FootprintCache,
    /// [`Self::universe_declaring`] per field, computed once.
    declaring: std::sync::Mutex<BTreeMap<DeclarationKey, Option<Vec<ObjectReference>>>>,
    /// The attempts' applications already computed, by observed state and
    /// attempt: see [`application::AppliedMemo`].
    applied: application::AppliedMemo,
    /// The map-key names of the run's state keys: see
    /// [`encode::KeyNames`].
    key_names: encode::KeyNames,
    /// The active FR-190 size bound, zero when no size-bounded roots are
    /// admitted for this system call.
    max_value_bits: std::sync::atomic::AtomicU8,
    /// FR-752: value-bound cuts observed while building this run's successors.
    value_bound_cuts: std::sync::Mutex<Vec<ModelBasedValueBoundCut>>,
    /// The canonical writer of this system's keys, with the identities it
    /// has already encoded.
    canonical: encode::Canonicalizer,
    /// FR-185's random parameters and rewards, as FR-187 reads them.
    probabilistic: probabilistic::Tables,
    /// FSpec FR-154's unit bounds on fields, each an invariant of every
    /// state, in ascending constraint order.
    member_bounds: Vec<fsl_semantics::model::state::MemberBound>,
    /// Each record's slots in declaration order, for its enumerated domain
    /// (FR-120, FR-215).
    pub(crate) record_orders: domain::RecordOrders,
    tuple_positions: BTreeMap<filament_exact::NodeKey, Vec<Option<usize>>>,
    /// The element-count members of the pure-clock collections (FR-230
    /// "Initial element count"): each object type holding one, the member,
    /// and the collection's `min` and `max` (`None` with no declared
    /// maximum).
    pub(crate) clock_counts: Vec<ClockCount>,
    /// The presence-bit members of the `Option<Clock>` fields (FR-230
    /// "Presence bit"): each object type holding one and the member.
    pub(crate) clock_bits: Vec<(DeclarationKey, StateField)>,
    /// Whether FR-120's bound check admits the system ([`Self::bounded`]),
    /// found once.
    bounded: std::sync::OnceLock<bool>,
}

/// One element-count member of a pure-clock collection: the object type
/// holding it, the member, and the count's `min` and `max`.
pub(crate) type ClockCount = (DeclarationKey, StateField, u64, Option<u64>);

fn refused(code: Code, cause: &'static str, fields: &[(&'static str, String)]) -> AdmissionFailure {
    AdmissionFailure::Refused(Box::new(record(code, cause, fields)))
}

fn record(code: Code, cause: &'static str, fields: &[(&'static str, String)]) -> AdmissionRecord {
    AdmissionRecord {
        catalog: filament_semantic_value::catalog::CatalogCode::new(code.as_str(), cause),
        fields: fields.iter().cloned().collect(),
        path: Vec::new(),
        kernel: None,
    }
}

fn fault(invariant: &'static str) -> AdmissionFailure {
    AdmissionFailure::Fault(filament_semantic_value::catalog::InternalFault::new(
        "model-simulation",
        invariant,
    ))
}

impl<'a> ModelSystem<'a> {
    /// What this system reads of its package (FR-531, FR-940).
    pub fn input(&self) -> &ModelCheckInput<'a> {
        &self.input
    }

    /// The source package this system is over, for an engine with no
    /// package form yet; `None` on the package route.
    pub fn source_package(&self) -> Option<&'a CheckedPackage> {
        self.input.source_package()
    }

    /// The evaluator of the package's checked forms.
    pub(crate) fn evaluator(&self) -> &'a dyn ModelEvaluator {
        self.input.evaluator
    }

    /// The package's `terminal` declarations (FR-411).
    pub(crate) fn terminals(&self) -> &[input::ModelTerminal<'a>] {
        self.input.terminals.as_deref().unwrap_or_default()
    }

    /// Resolve a checked operation's declaring type without substituting the
    /// receiver's inheriting type. Owner-chain lookup work belongs to the caller.
    ///
    /// # Errors
    /// Returns the work hook's first refusal unchanged.
    pub fn declaration_key_with_work<E>(
        &self,
        effective: &EffectiveId,
        work: &mut impl FnMut(u64) -> Result<(), E>,
    ) -> Result<Option<&DeclarationKey>, E> {
        self.model.declaration_key_with_work(effective, work)
    }

    /// Every field some operation's frame grants: the fields it writes and
    /// every field of a type it creates.
    fn granted(&self) -> BTreeSet<DeclarationKey> {
        let mut granted = BTreeSet::new();
        for declaration in self.types.object_types() {
            for operation in self.operations.declared_by(declaration.key()) {
                let effect = operation.effect();
                granted.extend(effect.written_fields().cloned());
                for created in self.creatable_types(effect) {
                    if let Some(type_fields) = self.fields.get(&created) {
                        granted.extend(type_fields.iter().map(|field| field.key.clone()));
                    }
                }
            }
        }
        granted
    }

    /// Whether FR-120's requires-bound check ([`check_model_bounds`])
    /// admits this system: every root of [`Self::domains`] is bounded.
    /// Every expansion entry point runs that check before expanding.
    pub(crate) fn bounded(&self) -> bool {
        *self
            .bounded
            .get_or_init(|| check_model_bounds(self, crate::simulation::BOUND_POSITIONS).is_ok())
    }

    /// The declarations every value of this system is encoded under: the
    /// package's record, tuple, union and variant types (FR-181).
    /// The values a candidate gives `field` of an object of `type_key`
    /// over the objects `references`: its FR-120 domain, except that the
    /// element count of a pure-clock `Set` or `OrderedSet` whose `min`
    /// exceeds 1 has no value (FR-230 "Initial element count": a set holds
    /// at most one bare clock), so no object of the type is created.
    pub(in crate::simulation::model) fn field_domain(
        &self,
        type_key: &DeclarationKey,
        field: &StateField,
        references: &[ObjectReference],
    ) -> Option<domain::Domain> {
        let infeasible = self.clock_counts.iter().any(|(key, count, min, max)| {
            key == type_key && count.key == field.key && max.is_some_and(|max| max < *min)
        });
        if infeasible {
            return Some(domain::Domain::Listed(Vec::new()));
        }
        self.value_domain(&field.value_type, references)
    }

    pub(crate) fn typing(&self) -> encode::Typing<'_> {
        encode::Typing::with_variants(self.types, self.input.variant_types)
            .with_enums(self.input.enums)
            .with_tuple_positions(&self.tuple_positions)
            .with_clock_members(&self.clock_counts, &self.clock_bits)
            .with_names(&self.key_names)
            .with_state_fields(&self.fields)
    }

    /// [`Self::typing`] for values alone, which hold no element-count
    /// member: it lives as long as the package.
    pub(crate) fn value_typing(&self) -> encode::Typing<'a> {
        encode::Typing::with_variants(self.types, self.input.variant_types)
            .with_enums(self.input.enums)
    }

    /// Check the universes, then admit each initial document in the given
    /// order, returning the first failure (FR-120 "Initial states").
    ///
    /// # Errors
    ///
    /// FR-106's [`AdmissionFailure`]: `Refused` or `Incomplete`, one record.
    pub fn new(request: ModelSimulationRequest<'a>) -> Result<Self, AdmissionFailure> {
        let selections = request.input.model_inputs();
        Self::over(request, selections, None, &[])
    }

    /// [`Self::new`] over the `model_selections` rows the subject reads: those
    /// of the identities its universes cover (FR-124-AC-12). Only the derived
    /// deadlock-freedom item reads a subject this way: it classifies the
    /// terminal states of the identities the subject reads and no other. When
    /// the universes cover every selection or none, this is [`Self::new`].
    ///
    /// # Errors
    ///
    /// As [`Self::new`].
    pub fn new_over_read_identities(
        request: ModelSimulationRequest<'a>,
    ) -> Result<Self, AdmissionFailure> {
        use fsl_semantics::model::observation::ModelSelections;

        let covered: BTreeSet<&str> = request
            .universes
            .iter()
            .map(|universe| universe.population.package.as_str())
            .collect();
        let read: Vec<_> = request
            .input
            .model_selections
            .iter()
            .filter(|selection| covered.contains(selection.identity.as_str()))
            .cloned()
            .collect();
        if read.is_empty() || read.len() == request.input.model_selections.len() {
            return Self::new(request);
        }
        let sources = request.input.source_packages;
        Self::over(request, ModelSelections::new(&read, sources), None, &[])
    }

    /// [`Self::new`] with the caller's cancellation shared by qualified
    /// reward-expression meters during reward admission.
    ///
    /// # Errors
    /// The same admission failures as [`Self::new`]; cancelled evaluation
    /// is incomplete and retains the cause on the supplied token.
    pub fn new_with_cancel(
        request: ModelSimulationRequest<'a>,
        cancel: &filament_exact::Cancel,
    ) -> Result<Self, AdmissionFailure> {
        let selections = request.input.model_inputs();
        Self::over(request, selections, Some(cancel), &[])
    }

    /// The model system's selected populations in ascending declaration-key
    /// order. This preserves a checked clause's population ordinal when a
    /// replay reconstructs an object-parameter binding.
    pub fn populations(&self) -> &[fsl_semantics::model::state::StatePopulation] {
        &self.populations
    }

    /// The system of trace variable alias `alias` of a hyper clause (FR-173
    /// "Event-position form": each trace is read over its own model). A unit
    /// that selects several models gets [`Self::over_model`] the package the
    /// alias names, so a trace ranges over its own model's executions alone;
    /// a unit with one model, or a package-route input, gets [`Self::new`].
    ///
    /// # Errors
    ///
    /// As [`Self::new`] and [`Self::over_model`].
    pub fn for_alias(
        request: ModelSimulationRequest<'a>,
        alias: &str,
    ) -> Result<Self, AdmissionFailure> {
        let model = match request.source_package() {
            Some(package) if request.input.model_selections.len() > 1 => package
                .graph()
                .model_package_identity(alias)
                .map(str::to_owned),
            _ => None,
        };
        match model {
            Some(model) => Self::over_model_of(request, &model, true),
            None => Self::new(request),
        }
    }

    /// The effective identity of the object type the declaration `key`
    /// names, as the checked type environment knows it.
    pub fn type_identity(&self, key: &DeclarationKey) -> Option<EffectiveId> {
        self.model.type_identity(key)
    }

    /// [`Self::new`] over the one domain package `model` names, for a unit
    /// that selects several models (ADR-020 RM-1: a refinement's abstract
    /// and concrete subjects are two models of one checked package). The
    /// system holds that model's populations only.
    ///
    /// # Errors
    ///
    /// `invalid_runtime_input`/`wrong-role-mapping` when the package selects
    /// no model named `model`; otherwise as [`Self::new`].
    pub fn over_model(
        request: ModelSimulationRequest<'a>,
        model: &str,
    ) -> Result<Self, AdmissionFailure> {
        Self::over_model_of(request, model, false)
    }

    /// [`Self::over_model`], skipping each universe of a population that
    /// another selected model owns when `skip_other_models` (a hyper clause's
    /// universes name the populations of every model its aliases range over).
    /// A universe of no selected model's population is refused either way.
    fn over_model_of(
        request: ModelSimulationRequest<'a>,
        model: &str,
        skip_other_models: bool,
    ) -> Result<Self, AdmissionFailure> {
        let selected: Vec<_> = request
            .input
            .model_selections
            .iter()
            .filter(|selection| selection.identity == model)
            .cloned()
            .collect();
        if selected.is_empty() {
            return Err(refused(
                Code::InvalidRuntimeInput,
                "wrong-role-mapping",
                &[("model", model.to_owned())],
            ));
        }
        let sources = request.input.source_packages;
        let others = if skip_other_models {
            let every = StateModel::new(
                request.input.model_inputs(),
                request.input.types,
                request.packages,
                request.normalization,
            )?;
            every
                .populations()
                .iter()
                .map(|population| population.key.clone())
                .collect()
        } else {
            Vec::new()
        };
        Self::over(
            request,
            fsl_semantics::model::observation::ModelSelections::new(&selected, sources),
            None,
            &others,
        )
    }

    fn over(
        request: ModelSimulationRequest<'a>,
        selections: fsl_semantics::model::observation::ModelSelections<'_>,
        cancel: Option<&filament_exact::Cancel>,
        other_models: &[DeclarationKey],
    ) -> Result<Self, AdmissionFailure> {
        let input = request.input;
        let types = input.types;
        let only = selections.selections();
        let model = StateModel::new(selections, types, request.packages, request.normalization)?;
        let populations = model.populations();

        let mut universes = BTreeMap::new();
        for universe in request.universes {
            if !populations
                .iter()
                .any(|population| population.key == universe.population)
            {
                if other_models.contains(&universe.population) {
                    continue;
                }
                return Err(refused(
                    Code::InvalidRuntimeInput,
                    "wrong-role-mapping",
                    &[("population", universe.population.node.clone())],
                ));
            }
            let mut seen = BTreeSet::new();
            for key in &universe.keys {
                // A key is an object identity (FR-035): the empty key names
                // no object, so it is refused here, not where a key context
                // binds it.
                if filament_exact::ObjectId::new(key.as_str()).is_err() {
                    return Err(refused(
                        Code::InvalidRuntimeInput,
                        "invalid-value",
                        &[
                            ("population", universe.population.node.clone()),
                            ("key", key.clone()),
                        ],
                    ));
                }
                if !seen.insert(key) {
                    return Err(refused(
                        Code::InvalidRuntimeInput,
                        "conflicting-identity",
                        &[
                            ("population", universe.population.node.clone()),
                            ("key", key.clone()),
                        ],
                    ));
                }
            }
            if universes
                .insert(universe.population.clone(), universe.keys)
                .is_some()
            {
                return Err(refused(
                    Code::InvalidRuntimeInput,
                    "conflicting-identity",
                    &[("population", universe.population.node)],
                ));
            }
        }

        let mut initial = Vec::new();
        for document in &request.initial {
            let (state, anchor) =
                model.admit_initial(request.snapshots, document, request.observation)?;
            for population in universes.keys() {
                if !state.populations.contains_key(population) {
                    return Err(AdmissionFailure::Incomplete(Box::new(record(
                        Code::IncompletePopulation,
                        "incomplete-scope",
                        &[("population", population.node.clone())],
                    ))));
                }
            }
            for (population, objects) in &state.populations {
                let Some(keys) = universes.get(population) else {
                    continue;
                };
                if let Some(key) = objects.keys().find(|key| !keys.contains(key)) {
                    return Err(refused(
                        Code::InvalidRuntimeInput,
                        "invalid-value",
                        &[
                            ("population", population.node.clone()),
                            ("object", key.clone()),
                        ],
                    ));
                }
            }
            initial.push(ModelSimState {
                state,
                anchor,
                reached_by: None,
            });
        }

        let reference_bindings = if let Some(package) = input.source_package() {
            ReferenceBindings::bind(
                package.graph(),
                &model,
                types,
                &populations,
                &universes,
                &initial,
            )?
        } else {
            ReferenceBindings::default()
        };

        let mut fields = BTreeMap::new();
        let mut applications = BTreeMap::new();
        for declaration in types.object_types() {
            let identity = declaration.key();
            let Some(type_key) = model.type_key(identity.clone()).cloned() else {
                continue;
            };
            let declared = model
                .fields(&type_key)?
                .ok_or_else(|| fault("type-fields"))?;
            fields.insert(type_key, declared);
            applications.insert(
                identity.clone(),
                Self::applications_of(input.types, input.operations, &model, identity)?,
            );
        }

        // FR-120-AC-26, FR-230 "Initial element count": a collection of bare
        // clocks holds no data, so its element count is an eval-side member
        // of each object's state, `[min, max]` (a `Set` or `OrderedSet` at
        // most 1), rooted, enumerated and read by the slot places like a
        // field. It is in no written or admitted document. An initial state
        // ranges over every count; with no declared maximum, the timed
        // subject ranges it up to `max_clocks` (`TimedSystem::initial`).
        let mut counts: Vec<ClockCount> = Vec::new();
        for timed in input.timed_models {
            for clock in timed.clocks() {
                let [
                    fsl_semantics::check::PlaceStep::Field(name),
                    fsl_semantics::check::PlaceStep::Slot(shape),
                ] = clock.path()
                else {
                    continue;
                };
                // A set's elements differ in their data part, which a bare
                // clock lacks (FR-120's `p` = 1).
                let max = if shape.unique {
                    Some(shape.max.unwrap_or(1).min(1))
                } else {
                    shape.max
                };
                // FR-230 "Presence bit", FR-230-AC-41/42: a
                // `Sequence<Option<Clock>>` holds one presence bit for each
                // element, so its member is the sequence of the bits, whose
                // length is the count.
                let value_type = match max {
                    Some(max) if clock.presence_bit() => {
                        ValueType::collection(filament_exact::CollectionType::new(
                            filament_exact::CollectionKind::Sequence,
                            ValueType::Boolean,
                            filament_exact::CardinalityBound::new(shape.min.min(max), max).ok(),
                        ))
                    }
                    Some(max) => filament_exact::IntegerInterval::new(
                        filament_exact::Integer::from(shape.min.min(max)),
                        filament_exact::Integer::from(max),
                    )
                    .map(ValueType::Int)
                    .unwrap_or(ValueType::Integer),
                    None => ValueType::Integer,
                };
                // The owner and every object type below it holds the count.
                for (type_key, declared) in &fields {
                    let below = model
                        .type_identity(type_key)
                        .is_some_and(|identity| types.conforms(identity, clock.owner()));
                    let held = declared.iter().any(|field| field.key == *clock.field())
                        || counts.iter().any(|(key, field, _, _)| {
                            key == type_key && field.key == *clock.field()
                        });
                    if !below || held {
                        continue;
                    }
                    counts.push((
                        type_key.clone(),
                        StateField {
                            key: clock.field().clone(),
                            name: name.clone(),
                            value_type: value_type.clone(),
                            optional: false,
                        },
                        shape.min,
                        max,
                    ));
                }
            }
        }
        for (type_key, field, _, _) in &counts {
            if let Some(declared) = fields.get_mut(type_key) {
                declared.push(field.clone());
            }
        }
        // FR-230 "Presence bit": an `Option<Clock>` field holds no data, so
        // its presence is an eval-side Boolean member of each object's
        // state, rooted and enumerated like a field; an initial state
        // ranges over absent and present.
        let mut bits: Vec<(DeclarationKey, StateField)> = Vec::new();
        for timed in input.timed_models {
            for clock in timed.clocks() {
                let (true, [fsl_semantics::check::PlaceStep::Field(name)]) =
                    (clock.presence_bit(), clock.path())
                else {
                    continue;
                };
                for (type_key, declared) in &fields {
                    let below = model
                        .type_identity(type_key)
                        .is_some_and(|identity| types.conforms(identity, clock.owner()));
                    let held = declared.iter().any(|field| field.key == *clock.field())
                        || bits
                            .iter()
                            .any(|(key, field)| key == type_key && field.key == *clock.field());
                    if !below || held {
                        continue;
                    }
                    bits.push((
                        type_key.clone(),
                        StateField {
                            key: clock.field().clone(),
                            name: name.clone(),
                            value_type: ValueType::Boolean,
                            optional: false,
                        },
                    ));
                }
            }
        }
        for (type_key, field) in &bits {
            if let Some(declared) = fields.get_mut(type_key) {
                declared.push(field.clone());
            }
        }
        let mut initial = with_clock_counts(initial, &counts, types);
        for (type_key, field) in &bits {
            initial = ranged(
                initial,
                type_key,
                &field.key,
                &[
                    filament_exact::Value::Boolean(false),
                    filament_exact::Value::Boolean(true),
                ],
            );
        }

        let record_orders = domain::record_orders(types, &model.record_orders());
        let tuple_positions = model.tuple_data_positions();
        let mut member_bounds = model.member_bounds();
        member_bounds.sort_by(|left, right| left.constraint.cmp(&right.constraint));
        // A field's bounds, read on each state; a parameter's and a result's
        // are read where an invocation supplies them.
        member_bounds.retain(|bound| {
            fields
                .values()
                .flatten()
                .any(|field| field.key == bound.member)
        });
        input
            .terminals
            .as_ref()
            .map_err(|_| fault("terminal-nodes-resolve"))?;
        let clauses = Self::clauses_of(&input.state_clauses)?;
        let probabilistic = probabilistic::Tables::of(input.probabilistic, input.read_rewards)?;
        let system = Self {
            over: match only {
                [only] => Some(only.identity.clone()),
                _ => None,
            },
            types,
            operations: input.operations,
            input,
            clauses,
            model,
            universes,
            initial,
            reference_bindings,
            applications,
            fields,
            populations,
            limits: request.expansion,
            meter: request.meter,
            footprints: footprints::FootprintCache::default(),
            declaring: std::sync::Mutex::default(),
            applied: application::AppliedMemo::default(),
            key_names: encode::KeyNames::default(),
            max_value_bits: std::sync::atomic::AtomicU8::new(0),
            value_bound_cuts: std::sync::Mutex::new(Vec::new()),
            canonical: encode::Canonicalizer::default(),
            probabilistic,
            member_bounds,
            record_orders,
            tuple_positions,
            clock_counts: counts,
            clock_bits: bits,
            bounded: std::sync::OnceLock::new(),
        };
        system.check_rewards_with_cancel(cancel)?;
        Ok(system)
    }

    /// The RFC 8785 key bytes and `filament.simulation.state-key/v1` state key of
    /// `state`, under the identity limits exploration encodes with.
    ///
    /// # Errors
    ///
    /// [`crate::simulation::EncodingRefusal`] when the key has no RFC 8785
    /// encoding under those limits.
    pub fn state_key(
        &self,
        state: &ModelSimState,
    ) -> Result<(Vec<u8>, crate::simulation::StateKey), crate::simulation::EncodingRefusal> {
        crate::simulation::key::state_key_of(
            &self.canonical,
            &encode::model_key(&self.model, &state.state, self.typing()).0,
            identity_limits(),
        )
    }

    /// Canonical raw model identity bytes under cumulative caller admission.
    ///
    /// `identity` bounds raw identity bytes, independently of any certificate
    /// transport limit. The budget and original cancellation token are shared
    /// across calls; this method makes no duplicate state-key buffer.
    ///
    /// # Errors
    /// [`ModelKeyError`] preserves caller denial, cancellation, identity byte
    /// refusal and actual allocation failure separately.
    pub fn state_key_bytes_with_budget(
        &self,
        state: &ModelSimState,
        identity: IdentityLimits,
        budget: &mut dyn ModelKeyBudget,
        cancel: &filament_exact::Cancel,
    ) -> Result<Vec<u8>, ModelKeyError> {
        encode::bounded::model_bytes(
            &self.model,
            &state.state,
            self.typing(),
            identity,
            budget,
            cancel,
        )
    }

    /// The full canonical state key used by the FR-101 simulator.
    pub(crate) fn canonical_state_key(
        &self,
        state: &ModelSimState,
    ) -> Result<crate::simulation::StateKey, crate::simulation::EncodingRefusal> {
        crate::simulation::key::canonical_state_key_of(
            &self.canonical,
            &encode::model_key(&self.model, &state.state, self.typing()).0,
            identity_limits(),
        )
    }

    /// The canonical writer of this system's keys.
    pub(crate) fn canonical(&self) -> &encode::Canonicalizer {
        &self.canonical
    }

    /// The evaluation meter limits each reading of the system starts from.
    pub fn meter_limits(&self) -> ScalarLimits {
        self.meter
    }

    /// FR-101's canonical bytes of `transition`, which order successors.
    ///
    /// # Errors
    ///
    /// [`crate::simulation::EncodingRefusal`] when the identity has no RFC
    /// 8785 encoding under exploration's identity limits.
    pub fn transition_key(
        &self,
        transition: &ModelTransition,
    ) -> Result<Vec<u8>, crate::simulation::EncodingRefusal> {
        crate::simulation::key::canonical_bytes(transition, identity_limits())
    }

    /// FR-187's canonical order key of a scheduled identity: the canonical
    /// bytes of its transition identity without random arguments, the order
    /// of [`Self::actions`]'s identities.
    ///
    /// # Errors
    ///
    /// An [`ExpansionStop`] when the identity has no canonical encoding.
    pub fn scheduled_identity_key(
        &self,
        identity: &ScheduledIdentity,
    ) -> Result<Vec<u8>, ExpansionStop> {
        frames::identity_bytes(
            &identity.operation,
            &identity.receiver,
            &identity.arguments,
            self.typing(),
        )
    }

    /// The operation text a transition of `operation` carries (FR-181): the
    /// declaration identity text of the operation `name` that `declaring`
    /// declares, which [`ModelTransition::operation`] returns for each of its
    /// applications. `None` when the subject's package declares no such
    /// operation on a type the subject admits. A fairness constraint covers
    /// the transitions that carry its operation's text (ADR-018 FA-1).
    pub fn operation_text(&self, declaring: EffectiveId, name: &str) -> Option<&str> {
        self.applications
            .values()
            .flatten()
            .find(|application| application.declaring == declaring && application.name == name)
            .map(|application| application.text.as_str())
    }

    /// Every transition identity of `state`, enabled or not, in canonical
    /// transition order (FR-156 "Candidates").
    ///
    /// # Errors
    ///
    /// An [`ExpansionStop`] when the state cannot be walked or the identities
    /// exceed the candidate bound.
    pub fn transition_identities(
        &self,
        state: &ModelSimState,
    ) -> Result<Vec<ModelTransition>, ExpansionStop> {
        walk::identities_of(self, state)
    }

    /// The operation text of the operation `key` names: the text
    /// [`Self::operation_text`] gives for the operation its declaring type's
    /// declaration key and its identifier select. `None` when this subject
    /// declares no such operation, as for the operation of another alias's
    /// model.
    pub fn operation_text_of(&self, key: &OperationKey) -> Option<&str> {
        self.applications
            .values()
            .flatten()
            .find(|application| {
                application.name == key.operation.as_str()
                    && self.model.type_key(application.declaring.clone()) == Some(&key.declaring)
            })
            .map(|application| application.text.as_str())
    }

    /// The operations of the subject's types, each once: its declaring type,
    /// name, and the text its transitions carry (see
    /// [`Self::operation_text`]). They come in canonical order: by the
    /// declaring type's effective identity (not the admitted type that
    /// inherits the operation), then by name bytes, so the order depends on
    /// neither declaration order nor which type admits an operation.
    pub fn operations(&self) -> Vec<(EffectiveId, &str, &str)> {
        let mut operations: Vec<(EffectiveId, &str, &str)> = self
            .applications
            .values()
            .flatten()
            .map(|application| {
                (
                    application.declaring.clone(),
                    application.name.as_str(),
                    application.text.as_str(),
                )
            })
            .collect();
        operations.sort_by(|left, right| {
            (left.0.clone(), left.1.as_bytes()).cmp(&(right.0.clone(), right.1.as_bytes()))
        });
        operations.dedup_by(|next, kept| (next.0.clone(), next.1) == (kept.0.clone(), kept.1));
        operations
    }

    /// The operations in `identity`'s effective view (FR-103, FR-081), by name.
    fn applications_of(
        types: &TypeEnvironment,
        operations: &fsl_semantics::model::operation::OperationTable,
        model: &StateModel,
        identity: EffectiveId,
    ) -> Result<Vec<Application>, AdmissionFailure> {
        let mut names = BTreeSet::new();
        for declaring in types.object_types() {
            if types.conforms(identity.clone(), declaring.key()) {
                for operation in operations.declared_by(declaring.key()) {
                    names.insert(operation.name().to_owned());
                }
            }
        }
        let mut applications = Vec::new();
        for name in names {
            let fsl_semantics::model::operation::OperationLookup::Declared {
                declaring,
                operation,
            } = operations.resolve(types, identity.clone(), &name)
            else {
                continue;
            };
            let owner = model
                .type_key(declaring.clone())
                .ok_or_else(|| fault("operation-owner"))?;
            let key = DeclarationKey {
                package: owner.package.clone(),
                node: format!("{}/{name}", owner.node),
            };
            let text = declaration_text(&key, CanonicalLimits::new(u64::MAX))
                .map_err(|_| fault("operation-identity-text"))?;
            applications.push(Application {
                name,
                declaring,
                declaration: operation.clone(),
                text,
            });
        }
        Ok(applications)
    }

    /// The package's state clauses by kind, each kind in ascending name order.
    fn clauses_of(entries: &[StateClauseEntry]) -> Result<Clauses, AdmissionFailure> {
        let mut clauses = Clauses::default();
        for clause in entries {
            let info = ClauseInfo {
                name: clause.name.clone(),
                qualified: QualifiedName::unqualified(&clause.name)
                    .map_err(|_| fault("clause-name"))?,
                identity: clause.identity.clone(),
                context: clause.context.clone(),
                operation: clause.operation.clone(),
            };
            match clause.observation {
                Observation::Current => clauses.invariants.push(info),
                Observation::Pre => clauses.preconditions.push(info),
                Observation::Post => clauses.postconditions.push(info),
            }
        }
        for group in [
            &mut clauses.invariants,
            &mut clauses.preconditions,
            &mut clauses.postconditions,
        ] {
            group.sort_by(|left, right| left.name.cmp(&right.name));
        }
        Ok(clauses)
    }

    /// FR-101's requires-bound roots (FR-120 "Bounds"): each operation
    /// parameter and result, each field a `modifies` entry grants or a
    /// creatable type declares, and each population the initial states hold.
    /// A type this module cannot enumerate is listed as an unbounded domain.
    pub fn domains(&self) -> Vec<(NodeKey, ValueType)> {
        self.rooted_domains()
            .into_iter()
            .map(|(node, _, value_type)| (node, value_type))
            .collect()
    }

    /// The active FR-190 bounded domain for one value type. Without a
    /// `max_value_bits` request, this is the ordinary FR-120 finite domain.
    pub(in crate::simulation::model) fn value_domain(
        &self,
        value_type: &ValueType,
        objects: &[ObjectReference],
    ) -> Option<domain::Domain> {
        let bits = match self
            .max_value_bits
            .load(std::sync::atomic::Ordering::Relaxed)
        {
            0 => None,
            bits => Some(bits),
        };
        domain::Domain::of_bounded(value_type, objects, self.types, &self.record_orders, bits)
    }

    pub(super) fn reset_value_bound_cuts(&self) {
        if let Ok(mut cuts) = self.value_bound_cuts.lock() {
            cuts.clear();
        }
    }

    pub(super) fn record_value_bound_cut(&self, operation: &str, state: &str) {
        let entry = ModelBasedValueBoundCut {
            operation: operation.to_owned(),
            state: state.to_owned(),
        };
        if let Ok(mut cuts) = self.value_bound_cuts.lock() {
            if !cuts.contains(&entry) {
                cuts.push(entry);
            }
        }
    }

    pub(super) fn value_bound_cuts(&self) -> Vec<ModelBasedValueBoundCut> {
        self.value_bound_cuts
            .lock()
            .map(|cuts| cuts.clone())
            .unwrap_or_default()
    }

    /// Generate inclusive endpoints and adjacent refusal values for every
    /// finite integer field and parameter domain, in declaration order.
    pub fn boundary_partitions(&self) -> Vec<ModelBoundaryPartition> {
        let mut partitions = Vec::new();
        let one = Integer::one();
        let mut roots = self.rooted_domains();
        roots.retain(|(_, path, _)| path.first() != Some(&2));
        roots.sort_by(|(left_node, left_path, _), (right_node, right_path, _)| {
            let rank = |path: &[u32]| match path.first() {
                None => 0_u8,
                Some(0) => 1,
                Some(1) => 2,
                Some(_) => 3,
            };
            rank(left_path)
                .cmp(&rank(right_path))
                .then_with(|| format!("{left_node:?}").cmp(&format!("{right_node:?}")))
                .then_with(|| left_path.cmp(right_path))
        });
        for (node, path, value_type) in roots {
            match &value_type {
                ValueType::Int(domain) => {
                    for (kind, value) in [
                        (ModelBoundaryKind::LowerRefusal, domain.lower().sub(&one)),
                        (ModelBoundaryKind::LowerEndpoint, domain.lower().clone()),
                        (ModelBoundaryKind::UpperEndpoint, domain.upper().clone()),
                        (ModelBoundaryKind::UpperRefusal, domain.upper().add(&one)),
                    ] {
                        partitions.push(ModelBoundaryPartition {
                            node: node.clone(),
                            path: path.clone(),
                            domain: domain.clone(),
                            kind,
                            value,
                            text_value: None,
                            text_profile: None,
                            status: None,
                        });
                    }
                }
                ValueType::Text(text_type) => {
                    let domain = IntegerInterval::new(
                        Integer::from(text_type.min()),
                        Integer::from(text_type.max()),
                    )
                    .expect("a checked text type has ordered bounds");
                    for (kind, length, text) in text_boundary_specs(text_type) {
                        let value = Integer::from(length);
                        partitions.push(ModelBoundaryPartition {
                            node: node.clone(),
                            path: path.clone(),
                            domain: domain.clone(),
                            kind,
                            value,
                            text_value: Some(text),
                            text_profile: Some(text_type.profile()),
                            status: None,
                        });
                    }
                }
                _ => {}
            }
        }
        partitions
    }

    /// [`Self::domains`], each with the child-index path below its node at
    /// which the root sits (FSpec FR-331): empty for a population, `[0, f]`
    /// for a field, `[1, o, p]` for a parameter, `[2, o]` for a result.
    pub fn rooted_domains(&self) -> Vec<(NodeKey, Vec<u32>, ValueType)> {
        self.member_domains(false)
    }

    /// FR-190 bounded roots, retaining each declared value type rather than
    /// reducing non-enumerable domains to the requires-bound marker type.
    /// Includes operation results, which are computed rather than generated.
    pub fn generation_domains(&self) -> Vec<(NodeKey, Vec<u32>, ValueType)> {
        self.member_domains(true)
    }

    fn member_domains(&self, preserve_types: bool) -> Vec<(NodeKey, Vec<u32>, ValueType)> {
        let mut roots = Vec::new();
        let mut root = |node: Option<NodeKey>, base: Vec<u32>, value_type: &ValueType| {
            let Some(node) = node else {
                return;
            };
            if preserve_types {
                roots.push((node, base, value_type.clone()));
                return;
            }
            let value_type = root_classification_type(value_type);
            let size_bits = self
                .max_value_bits
                .load(std::sync::atomic::Ordering::Relaxed);
            let bounded = if matches!(&value_type, ValueType::Population(_))
                || domain::enumerable(&value_type)
            {
                value_type
            } else if domain::finite(&value_type, self.types) {
                // A finite enum, record or tuple (STD-508), its extent
                // read whole.
                ValueType::Boolean
            } else if matches!(&value_type, ValueType::Integer) && size_bits > 0 {
                let extent = Integer::one()
                    .shifted_left(u64::from(size_bits))
                    .sub(&Integer::one());
                let interval = filament_exact::IntegerInterval::new(
                    extent.mul(&Integer::from(-1_i64)),
                    extent,
                );
                match interval {
                    Ok(interval) => ValueType::Int(interval),
                    Err(_) => ValueType::Integer,
                }
            } else {
                value_type
            };
            roots.push((node, base, bounded));
        };
        // FSpec FR-331 "Model member domain keys": a member's domain is the
        // model declaration node of its declaring type and the member's path
        // (field `[0, f]`, parameter `[1, o, p]`, result `[2, o]`), `f` and
        // `o` the ordinal among the type's own fields and operations in
        // UTF-8 name order and `p` the declared parameter position.
        let type_node = |id: &EffectiveId| {
            let original = id.original();
            fsl_semantics::model::key::model_node_key(
                original.package(),
                "model",
                "object_type",
                original.node(),
            )
            .ok()
        };
        for declaration in self.types.object_types() {
            let owner = declaration.key();
            let declared = self.operations.declared_by(owner.clone());
            let mut names: Vec<&str> = declared.iter().map(|operation| operation.name()).collect();
            names.sort_unstable();
            for operation in declared {
                let Some(ordinal) = names
                    .binary_search(&operation.name())
                    .ok()
                    .and_then(|ordinal| u32::try_from(ordinal).ok())
                else {
                    continue;
                };
                let clocks: BTreeSet<usize> = operation
                    .clock_parameters()
                    .iter()
                    .map(|(position, _)| *position)
                    .collect();
                let mut declared_positions =
                    (0_usize..).filter(|position| !clocks.contains(position));
                for (position, (_, value_type)) in operation.parameters().iter().enumerate() {
                    let declared_position = declared_positions.next().unwrap_or(position);
                    // A random parameter's finite domain is its support
                    // (ADR-024 PM-1), whatever its declared type.
                    if !domain::enumerable(value_type)
                        && self
                            .probabilistic
                            .support(declaration.key(), operation.name(), position)
                            .is_some()
                    {
                        continue;
                    }
                    let Ok(declared_position) = u32::try_from(declared_position) else {
                        continue;
                    };
                    root(
                        type_node(&owner),
                        vec![1, ordinal, declared_position],
                        value_type,
                    );
                }
                if let Some(result) = operation.result() {
                    root(type_node(&owner), vec![2, ordinal], result);
                }
            }
        }
        let granted = self.granted();
        let field_types: BTreeMap<&DeclarationKey, ValueType> = self
            .fields
            .values()
            .flatten()
            .map(|field| {
                let declared = field.value_type.clone();
                (
                    &field.key,
                    if field.optional {
                        ValueType::option(declared)
                    } else {
                        declared
                    },
                )
            })
            .collect();
        // Each field's declaring type and ordinal among that type's own
        // fields, found through the effective attribute set of every type
        // (which lists a field in the same order as `self.fields`).
        let evaluator = self.evaluator();
        let mut field_homes: BTreeMap<&DeclarationKey, (EffectiveId, u32)> = BTreeMap::new();
        for declaration in self.types.object_types() {
            let Some(type_key) = self.model.type_key(declaration.key()) else {
                continue;
            };
            let (Some(fields), Some(attributes)) = (
                self.fields.get(type_key),
                self.types.attributes(declaration.key()),
            ) else {
                continue;
            };
            let attributes: Vec<_> = attributes.into_iter().collect();
            for field in fields {
                // A field is found by name in the effective set: an eval-side
                // member (a clock count or presence bit) is named for the
                // clock field it stands for.
                let Some(attribute) = attributes
                    .iter()
                    .find(|attribute| attribute.field().name() == field.name)
                else {
                    continue;
                };
                let owner = attribute.owner();
                let Some(ordinal) = evaluator
                    .own_field_ordinal(&owner, attribute.field().name())
                    .and_then(|ordinal| u32::try_from(ordinal).ok())
                else {
                    continue;
                };
                field_homes.insert(&field.key, (owner, ordinal));
            }
        }
        // An eval-side clock member stands for a clock field its owner
        // declares in a `time` declaration, outside the owner's data
        // attributes: it is keyed under that owner by its place in the
        // owner's one name-ordered sequence of own fields.
        let mut clock_fields: BTreeMap<EffectiveId, BTreeMap<String, &DeclarationKey>> =
            BTreeMap::new();
        for timed in self.input.timed_models {
            for clock in timed.clocks() {
                let Some(fsl_semantics::check::PlaceStep::Field(name)) = clock.path().first()
                else {
                    continue;
                };
                if field_homes.contains_key(clock.field()) {
                    continue;
                }
                clock_fields
                    .entry(clock.owner())
                    .or_default()
                    .insert(name.clone(), clock.field());
            }
        }
        for (owner, named) in &clock_fields {
            for (name, key) in named {
                if let Some(ordinal) = evaluator
                    .own_field_ordinal(owner, name)
                    .and_then(|ordinal| u32::try_from(ordinal).ok())
                {
                    field_homes.insert(key, (owner.clone(), ordinal));
                }
            }
        }
        // A pure-clock collection's count is a root only where a frame
        // grants it: with no declared maximum it is then unbounded. An
        // ungranted one ranges its initial count up to `max_clocks` (FR-230
        // "Initial element count").
        let root_fields: Vec<_> = if preserve_types {
            field_types.keys().copied().collect()
        } else {
            granted.iter().collect()
        };
        for key in root_fields {
            // FR-190 classes E/C keep clock-bearing roots under FR-230;
            // their eval-side count/presence encoding is not a size-bound
            // mathematical Integer generation domain.
            if preserve_types
                && self
                    .input
                    .timed_models
                    .iter()
                    .any(|timed| timed.clocks().iter().any(|clock| clock.field() == key))
            {
                continue;
            }
            if let (Some(value_type), Some((owner, ordinal))) =
                (field_types.get(key), field_homes.get(key))
            {
                root(type_node(owner), vec![0, *ordinal], value_type);
            }
        }
        let held: BTreeSet<&DeclarationKey> = self
            .initial
            .iter()
            .flat_map(|state| state.state.populations.keys())
            .collect();
        for population in held {
            let bound = self
                .universes
                .get(population)
                .map(|keys| u64::try_from(keys.len()).unwrap_or(u64::MAX));
            root(
                fsl_semantics::model::key::population_node_key(population).ok(),
                Vec::new(),
                &ValueType::Population(bound),
            );
        }
        roots
    }

    /// FR-125 Instancing: the instances of an `over (x: T)` parameter of
    /// type `over` under this subject's universes, in the canonical order.
    ///
    /// A population admits `T` when one of its member types is `T`, a
    /// subtype or a supertype of `T`; each key of each admitting
    /// population's universe is one instance, however many of its member
    /// types are comparable with `T`. Admitting populations are ordered by
    /// declaration identity text, and the keys of each by its universe list.
    /// A type no population admits, and a type that is not an object
    /// reference, has no instance.
    ///
    /// # Errors
    ///
    /// [`NotSimulated::RequiresBound`] naming every admitting population
    /// with no universe, before any instance is listed (FR-120 "Bounds");
    /// [`NotSimulated::Extent`] when that classification stops or a
    /// population's identity text does not encode.
    pub fn instances(&self, over: &ValueType) -> Result<Vec<ModelInstance>, NotSimulated> {
        let ValueType::Reference(parameter) = over else {
            return Ok(Vec::new());
        };
        let mut admitting = Vec::new();
        for population in &self.populations {
            if self.overlaps(population, parameter.clone()) {
                let text = declaration_text(&population.key, CanonicalLimits::new(u64::MAX))
                    .map_err(|_| {
                        NotSimulated::Extent(ClassifyFailure::Fault(InternalFault::new(
                            "model-simulation",
                            "population-identity-text",
                        )))
                    })?;
                admitting.push((text, &population.key));
            }
        }
        let mut unbounded = Vec::new();
        for (_, population) in &admitting {
            if !self.universes.contains_key(*population) {
                let node =
                    fsl_semantics::model::key::population_node_key(population).map_err(|_| {
                        NotSimulated::Extent(ClassifyFailure::Fault(InternalFault::new(
                            "model-simulation",
                            "population-root-node",
                        )))
                    })?;
                unbounded.push((node, ValueType::Population(None)));
            }
        }
        let roots: Vec<(NodeKey, &ValueType)> = unbounded
            .iter()
            .map(|(id, root)| (id.clone(), root))
            .collect();
        crate::simulation::not_simulated::check_requires_bound(&roots, self.types, u64::MAX)?;
        admitting.sort_by(|left, right| left.0.cmp(&right.0));
        Ok(admitting
            .into_iter()
            .flat_map(|(_, population)| {
                self.universes
                    .get(population)
                    .into_iter()
                    .flatten()
                    .map(|key| ModelInstance {
                        population: population.clone(),
                        key: key.clone(),
                    })
            })
            .collect())
    }

    /// Whether `population` overlaps the object type `parameter` (FR-125
    /// Instancing): one of its member types conforms to `parameter` or
    /// `parameter` conforms to one of them.
    fn overlaps(
        &self,
        population: &fsl_semantics::model::state::StatePopulation,
        parameter: EffectiveId,
    ) -> bool {
        population.member_types.iter().any(|member| {
            self.model.type_identity(member).is_some_and(|member| {
                self.types.conforms(member.clone(), parameter.clone())
                    || self.types.conforms(parameter.clone(), member)
            })
        })
    }

    /// The typed value an FR-338 key context binds for `instance` of a
    /// parameter of object type `over`: the reference to its key in the
    /// universe of `over`'s declared type. It is a function of the instance
    /// and the parameter alone, never of a state's object.
    ///
    /// # Errors
    ///
    /// An [`InternalFault`] when no selected model declares `over`, or the
    /// key, which [`ModelSystem::new`] admitted, has no reference.
    pub fn bound_reference(
        &self,
        over: EffectiveId,
        instance: &ModelInstance,
    ) -> Result<ObjectReference, InternalFault> {
        let type_key = self
            .model
            .type_key(over)
            .ok_or_else(|| InternalFault::new("model-simulation", "over-type-declared"))?;
        self.model
            .object_reference(type_key, &instance.key)
            .map_err(|_| InternalFault::new("model-simulation", "universe-key-references"))
    }

    /// ADR-027 FT-3: every object of this subject's universe whose type
    /// declares `field`, in canonical order, computed once per field.
    ///
    /// `None` when no object type declares it or a population has no
    /// universe to list, including a reference that does not resolve. That
    /// is intentional: the caller then keeps the field on any object, the
    /// wider footprint, so a fault here costs precision and never a
    /// dependence.
    pub(crate) fn universe_declaring(
        &self,
        field: &DeclarationKey,
    ) -> Option<Vec<ObjectReference>> {
        let mut cache = self
            .declaring
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        cache
            .entry(field.clone())
            .or_insert_with(|| self.compute_universe_declaring(field))
            .clone()
    }

    fn compute_universe_declaring(&self, field: &DeclarationKey) -> Option<Vec<ObjectReference>> {
        let mut objects = Vec::new();
        for (type_key, fields) in &self.fields {
            if !fields.iter().any(|declared| &declared.key == field) {
                continue;
            }
            let identity = self.model.type_identity(type_key)?;
            for instance in self
                .instances(&ValueType::Reference(identity.clone()))
                .ok()?
            {
                objects.push(self.bound_reference(identity.clone(), &instance).ok()?);
            }
        }
        objects.sort();
        objects.dedup();
        (!objects.is_empty()).then_some(objects)
    }

    /// The non-abstract object types `effect` creates (FR-013): each conforms
    /// to a `creates` entry.
    fn creatable_types(&self, effect: &OperationEffect) -> Vec<DeclarationKey> {
        let mut creatable = Vec::new();
        for declaration in self.types.object_types() {
            let Some(key) = self.model.type_key(declaration.key()) else {
                continue;
            };
            if self.model.is_abstract(key) {
                continue;
            }
            let conforms = effect.creates.iter().any(|grant| {
                self.model
                    .type_identity(grant)
                    .is_some_and(|target| self.types.conforms(declaration.key(), target))
            });
            if conforms {
                creatable.push(key.clone());
            }
        }
        creatable.sort();
        creatable
    }
}

fn text_nul_value(length: u64) -> String {
    let count = usize::try_from(length).expect("text boundary length fits this platform");
    let mut value = String::new();
    value
        .try_reserve_exact(count)
        .expect("text boundary length can be allocated");
    value.extend(std::iter::repeat_n('\0', count));
    value
}

fn text_boundary_specs(
    text_type: &filament_exact::TextType,
) -> Vec<(ModelBoundaryKind, u64, String)> {
    let min = text_type.min();
    let max = text_type.max();
    let mut boundaries = Vec::new();
    if min == 0 {
        boundaries.push((ModelBoundaryKind::TextEmpty, 0));
    } else {
        boundaries.push((ModelBoundaryKind::TextLowerEndpoint, min));
    }
    if min <= 1 && 1 <= max {
        boundaries.push((ModelBoundaryKind::TextSingleton, 1));
    }
    boundaries.push((ModelBoundaryKind::TextUpperEndpoint, max));
    if min > 0 {
        boundaries.push((ModelBoundaryKind::LowerRefusal, min - 1));
    }
    if let Some(outside) = max.checked_add(1) {
        boundaries.push((ModelBoundaryKind::UpperRefusal, outside));
    }
    boundaries
        .into_iter()
        .map(|(kind, length)| (kind, length, text_nul_value(length)))
        .chain(text_normalization_specs(text_type))
        .collect()
}

fn text_normalization_specs(
    text_type: &filament_exact::TextType,
) -> Vec<(ModelBoundaryKind, u64, String)> {
    const WITNESSES: [&str; 8] = [
        "\u{00e9}",
        "e\u{0301}",
        "d\u{0307}\u{0323}",
        "d\u{0323}\u{0307}",
        "\u{ac00}",
        "\u{1100}\u{1161}",
        "\u{212b}",
        "\u{fb01}",
    ];
    let profile = text_type.profile();
    let mut seen = std::collections::BTreeSet::new();
    WITNESSES
        .into_iter()
        .filter_map(|spelling| {
            let retained = profile
                .normalization()
                .map(|form| form.apply(spelling).collect::<String>())
                .unwrap_or_else(|| spelling.to_owned());
            let length = profile.length(&retained);
            if !(text_type.min()..=text_type.max()).contains(&length)
                || !seen.insert(retained.clone())
            {
                return None;
            }
            Some((ModelBoundaryKind::TextNormalization, length, retained))
        })
        .collect()
}

/// The classify_extent input for a rooted quantity: quantity domains are
/// integral before the classifier decides whether the root is bounded.
pub(crate) fn root_classification_type(value_type: &ValueType) -> ValueType {
    match value_type {
        ValueType::Quantity(_) | ValueType::BoundedQuantity(_) | ValueType::CarriedQuantity(_) => {
            ValueType::Integer
        }
        other => other.clone(),
    }
}

impl TransitionSystem for ModelSystem<'_> {
    type State = ModelSimState;
    type TransitionId = ModelTransition;
    type Key = ModelKey;
    type Finding = ModelFinding;

    fn initial(&self) -> Vec<Self::State> {
        self.initial.clone()
    }

    fn key(&self, state: &Self::State) -> Self::Key {
        encode::model_key(&self.model, &state.state, self.typing())
    }

    fn successors(
        &self,
        state: &Self::State,
    ) -> Result<Expansion<Self::TransitionId, Self::State, Self::Finding>, ExpansionStop> {
        walk::expand_state(self, state, None)
    }

    /// FR-162: symmetry, partial-order reduction and state constraints.
    fn offered_reductions(&self) -> OfferedReductions {
        OfferedReductions {
            symmetry: true,
            partial_order: true,
            constraint: true,
        }
    }

    fn canonical(
        &self,
        state: &Self::State,
        group: &SymmetryGroup,
    ) -> Option<Result<(Self::State, Permutation), ExpansionStop>> {
        Some(self.canonicalise(state, group))
    }

    fn holds_constraint(
        &self,
        state: &Self::State,
        constraint: &str,
    ) -> Option<Result<ConstraintValue, ExpansionStop>> {
        Some(self.constraint_value(state, constraint))
    }

    /// FR-634: the invariant findings of `state` alone, `None` for a package
    /// declaring no invariant and no member bound.
    fn state_findings(
        &self,
        state: &Self::State,
    ) -> Option<Result<Vec<Self::Finding>, ExpansionStop>> {
        (!self.clauses.invariants.is_empty() || !self.member_bounds.is_empty())
            .then(|| self.invariant_findings_alone(state))
    }

    /// FR-634: every invariant of `state` undetermined with `stop`'s cause,
    /// and its member-bound findings; empty when its objects cannot be
    /// referenced, so no object can be named.
    fn stopped_findings(&self, state: &Self::State, stop: ExpansionStop) -> Vec<Self::Finding> {
        contracts::stopped_invariants(self, state, stop.cause).unwrap_or_default()
    }

    /// FR-162: [`ModelSystem::footprint`], which reads the transition alone.
    fn footprint(
        &self,
        transition: &Self::TransitionId,
    ) -> Option<Result<Footprint, ExpansionStop>> {
        Some(
            ModelSystem::footprint(self, transition).map_err(|_| ExpansionStop {
                cause: BROKEN,
                limit: None,
            }),
        )
    }
}

/// A model view used to generate a claim's cases without enforcing the claim
/// itself as a precondition or postcondition of the transition relation.
pub(super) struct ModelRunView<'s, 'a> {
    system: &'s ModelSystem<'a>,
    omitted_clause: Option<NodeKey>,
}

impl<'s, 'a> ModelRunView<'s, 'a> {
    pub(super) fn new(system: &'s ModelSystem<'a>, omitted_clause: Option<NodeKey>) -> Self {
        Self {
            system,
            omitted_clause,
        }
    }

    pub(super) fn transition_key(
        &self,
        transition: &ModelTransition,
    ) -> Result<Vec<u8>, crate::simulation::EncodingRefusal> {
        self.system.transition_key(transition)
    }
}

impl TransitionSystem for ModelRunView<'_, '_> {
    type State = ModelSimState;
    type TransitionId = ModelTransition;
    type Key = ModelKey;
    type Finding = ModelFinding;

    fn initial(&self) -> Vec<Self::State> {
        self.system.initial.clone()
    }

    fn key(&self, state: &Self::State) -> Self::Key {
        self.system.key(state)
    }

    fn successors(
        &self,
        state: &Self::State,
    ) -> Result<Expansion<Self::TransitionId, Self::State, Self::Finding>, ExpansionStop> {
        match &self.omitted_clause {
            Some(clause) => walk::expand_state_omitting_clause(self.system, state, clause),
            None => self.system.successors(state),
        }
    }

    fn state_findings(
        &self,
        state: &Self::State,
    ) -> Option<Result<Vec<Self::Finding>, ExpansionStop>> {
        self.system.state_findings(state)
    }

    fn stopped_findings(&self, state: &Self::State, stop: ExpansionStop) -> Vec<Self::Finding> {
        self.system.stopped_findings(state, stop)
    }
}

/// The identity limits `explore_model` and `sample_model` encode under.
fn identity_limits() -> IdentityLimits {
    IdentityLimits::default()
}

/// FR-101's requires-bound pre-check over `system`'s [`ModelSystem::domains`]
/// alone, with no state explored (ADR-018 "EN-1 pre-check").
///
/// # Errors
///
/// [`NotSimulated::RequiresBound`] when a root is unbounded;
/// [`NotSimulated::Extent`] when classification stops.
pub fn check_model_bounds(
    system: &ModelSystem<'_>,
    position_limit: u64,
) -> Result<(), NotSimulated> {
    let domains = system.rooted_domains();
    let roots: Vec<(NodeKey, Vec<u32>, &ValueType)> = domains
        .iter()
        .map(|(id, base, value_type)| (id.clone(), base.clone(), value_type))
        .collect();
    crate::simulation::not_simulated::check_requires_bound(&roots, system.types, position_limit)
}

/// FR-101's `explore_request` over `system`, its [`ModelSystem::domains`] and
/// the package's `TypeEnvironment` (FR-120 "Outputs").
///
/// # Errors
///
/// [`NotSimulated`] as `explore_request` returns it, before any state is
/// expanded when a root is unbounded.
pub fn explore_model(
    system: &ModelSystem<'_>,
    position_limit: u64,
    limits: Limits,
    poll: impl FnMut() -> bool,
) -> Result<Exploration<ModelFinding>, NotSimulated> {
    let domains = system.rooted_domains();
    let roots: Vec<(NodeKey, Vec<u32>, &ValueType)> = domains
        .iter()
        .map(|(id, base, value_type)| (id.clone(), base.clone(), value_type))
        .collect();
    explore_request(
        system,
        &roots,
        system.types,
        position_limit,
        limits,
        identity_limits(),
        None,
        poll,
    )
}

/// FR-101's `sample_request` over `system`, followed by `replay` of the
/// sampled trace against it, which yields one [`StepEffect`] per step.
///
/// # Errors
///
/// [`ModelSampleError::NotSimulated`] as `sample_request` returns it;
/// [`ModelSampleError::Replay`] when the trace does not replay against the
/// system that sampled it.
#[allow(
    clippy::result_large_err,
    reason = "FR-120 pins ModelSampleError::Replay(ReplayError<ModelTransition>)"
)]
pub fn sample_model(
    system: &ModelSystem<'_>,
    position_limit: u64,
    sampler: &DefinitionRef,
    seed: u64,
    trace: u64,
    max_steps: usize,
) -> Result<ModelTrace, ModelSampleError> {
    sample_model_with_omitted_clause(
        system,
        position_limit,
        sampler,
        seed,
        trace,
        max_steps,
        None,
    )
}

pub(super) fn sample_model_with_omitted_clause(
    system: &ModelSystem<'_>,
    position_limit: u64,
    sampler: &DefinitionRef,
    seed: u64,
    trace: u64,
    max_steps: usize,
    omitted_clause: Option<NodeKey>,
) -> Result<ModelTrace, ModelSampleError> {
    let domains = system.rooted_domains();
    let roots: Vec<(NodeKey, Vec<u32>, &ValueType)> = domains
        .iter()
        .map(|(id, base, value_type)| (id.clone(), base.clone(), value_type))
        .collect();
    let view = ModelRunView::new(system, omitted_clause.clone());
    let trace = sample_request(
        &view,
        &roots,
        system.types,
        position_limit,
        sampler,
        seed,
        trace,
        max_steps,
        identity_limits(),
    )?;
    replay(&view, &trace, identity_limits()).map_err(ModelSampleError::Replay)?;
    let effects = effects_of_with_omitted_clause(system, &trace, omitted_clause.as_ref())?;
    let boundary_partitions = system.boundary_partitions();
    Ok(ModelTrace {
        trace,
        effects,
        attempted: None,
        boundary_partitions,
    })
}

/// Shrink a model trace through single-invocation deletion and immediate
/// decreases in canonical invocation order while preserving a caller's
/// native failure oracle.
///
/// Each candidate is reconstructed from enabled model transitions and
/// replayed against `system` before `fails` is called. Deletion strictly
/// reduces the trace length; invocation decreases use canonical transition
/// identity bytes as a deterministic finite order. The result is 1-minimal
/// under those one-step reductions. The oracle should evaluate the same claim
/// and disposition against the candidate trace using the native evaluator.
///
/// # Errors
///
/// [`ModelShrinkError::NotCounterexample`] when the supplied trace does not
/// satisfy `fails`; replay and model-bound failures retain their typed cause.
pub fn shrink_model_trace(
    system: &ModelSystem<'_>,
    trace: &ModelTrace,
    fails: impl FnMut(&ModelTrace) -> bool,
) -> Result<ModelTrace, ModelShrinkError> {
    shrink_model_trace_with_omitted_clause(system, trace, None, fails)
}

pub(super) fn shrink_model_trace_with_omitted_clause(
    system: &ModelSystem<'_>,
    trace: &ModelTrace,
    omitted_clause: Option<NodeKey>,
    mut fails: impl FnMut(&ModelTrace) -> bool,
) -> Result<ModelTrace, ModelShrinkError> {
    let view = ModelRunView::new(system, omitted_clause.clone());
    replay(&view, &trace.trace, identity_limits()).map_err(|error| match error {
        ReplayError::KeyEncoding(refusal) => {
            ModelShrinkError::NotSimulated(NotSimulated::KeyEncoding(refusal))
        }
        other => ModelShrinkError::Replay(other),
    })?;
    if !fails(trace) {
        return Err(ModelShrinkError::NotCounterexample);
    }

    let mut smallest = trace.clone();
    loop {
        let (replayed, states) = rebuild_model_trace(
            system,
            &smallest.trace.initial,
            &smallest.trace.steps,
            smallest.trace.provenance.as_ref(),
            omitted_clause.as_ref(),
        )?
        .ok_or(ModelShrinkError::NotCounterexample)?;
        let mut accepted = None;

        // First try deleting each invocation, from left to right. Rebuilding
        // the candidate recomputes state keys and findings at every position.
        for index in 0..replayed.trace.steps.len() {
            let mut steps = replayed.trace.steps.clone();
            steps.remove(index);
            let Some((candidate, _)) = rebuild_model_trace(
                system,
                &replayed.trace.initial,
                &steps,
                replayed.trace.provenance.as_ref(),
                omitted_clause.as_ref(),
            )?
            else {
                continue;
            };
            if fails(&candidate) {
                accepted = Some(candidate);
                break;
            }
        }
        if let Some(candidate) = accepted {
            smallest = candidate;
            continue;
        }

        // Then try the immediate lower enabled invocation with the same
        // operation and receiver at each position. Invocation order is the
        // canonical transition-identity byte order used by simulation.
        for (index, current) in replayed.trace.steps.iter().enumerate() {
            let current_key = view
                .transition_key(&current.transition)
                .map_err(|refusal| {
                    ModelShrinkError::NotSimulated(NotSimulated::KeyEncoding(refusal))
                })?;
            let expansion =
                expand(&view, &states[index], identity_limits()).map_err(|refusal| {
                    ModelShrinkError::NotSimulated(NotSimulated::KeyEncoding(refusal))
                })?;
            let Expanded::Successors { successors, .. } = expansion else {
                continue;
            };
            let mut predecessor: Option<(Vec<u8>, ModelTransition)> = None;
            for successor in successors {
                let transition = successor.transition;
                if transition.operation() != current.transition.operation()
                    || transition.receiver() != current.transition.receiver()
                {
                    continue;
                }
                let key = view.transition_key(&transition).map_err(|refusal| {
                    ModelShrinkError::NotSimulated(NotSimulated::KeyEncoding(refusal))
                })?;
                let is_closer = match predecessor.as_ref() {
                    Some((previous, _)) => key.as_slice() > previous.as_slice(),
                    None => true,
                };
                if key.as_slice() < current_key.as_slice() && is_closer {
                    predecessor = Some((key, transition));
                }
            }
            let Some((_, transition)) = predecessor else {
                continue;
            };
            let mut steps = replayed.trace.steps.clone();
            steps[index] = Step {
                transition,
                ordinal: 0,
                key: current.key.clone(),
            };
            let Some((candidate, _)) = rebuild_model_trace(
                system,
                &replayed.trace.initial,
                &steps,
                replayed.trace.provenance.as_ref(),
                omitted_clause.as_ref(),
            )?
            else {
                continue;
            };
            if fails(&candidate) {
                accepted = Some(candidate);
                break;
            }
        }
        if let Some(candidate) = accepted {
            smallest = candidate;
        } else {
            return Ok(replayed);
        }
    }
}

/// Rebuild a candidate from enabled transitions, recomputing canonical keys,
/// findings, effects and its terminal reason. An unavailable transition makes
/// the candidate invalid rather than an error in the shrink run.
fn rebuild_model_trace(
    system: &ModelSystem<'_>,
    initial_key: &crate::simulation::StateKey,
    requested: &[Step<ModelTransition>],
    provenance: Option<&SampleProvenance>,
    omitted_clause: Option<&NodeKey>,
) -> Result<Option<(ModelTrace, Vec<ModelSimState>)>, ModelShrinkError> {
    let view = ModelRunView::new(system, omitted_clause.cloned());
    let mut current = sorted_initial(&view, identity_limits())
        .map_err(|refusal| ModelShrinkError::NotSimulated(NotSimulated::KeyEncoding(refusal)))?
        .into_iter()
        .find(|initial| initial.key == *initial_key)
        .map(|initial| initial.state);
    let Some(mut current) = current.take() else {
        return Err(ModelShrinkError::Replay(ReplayError::UnknownInitial {
            expected: initial_key.clone(),
        }));
    };

    let mut steps = Vec::with_capacity(requested.len());
    let mut findings = Vec::new();
    let mut states = Vec::with_capacity(requested.len() + 1);
    states.push(current.clone());
    let mut stopped = None;

    for index in 0..=requested.len() {
        let (successors, at_state) = match expand(&view, &current, identity_limits())
            .map_err(|refusal| ModelShrinkError::NotSimulated(NotSimulated::KeyEncoding(refusal)))?
        {
            Expanded::Successors {
                successors,
                findings,
            } => (successors, findings),
            Expanded::Stopped(stop) => {
                if index < requested.len() {
                    return Ok(None);
                }
                let state_findings = system.findings_at_stop(&current);
                if !state_findings.is_empty() {
                    let (_, state_key) = system.state_key(&current).map_err(|refusal| {
                        ModelShrinkError::NotSimulated(NotSimulated::KeyEncoding(refusal))
                    })?;
                    findings.push(StateFindings {
                        state: state_key,
                        depth: index,
                        findings: state_findings,
                    });
                }
                stopped = Some(StopReason::Stopped(stop.cause));
                break;
            }
        };
        if !at_state.is_empty() {
            let (_, state_key) = system.state_key(&current).map_err(|refusal| {
                ModelShrinkError::NotSimulated(NotSimulated::KeyEncoding(refusal))
            })?;
            findings.push(StateFindings {
                state: state_key,
                depth: index,
                findings: at_state,
            });
        }
        if index == requested.len() {
            stopped = Some(if successors.is_empty() {
                StopReason::NoSuccessors
            } else {
                StopReason::StepLimit
            });
            break;
        }

        let wanted = &requested[index];
        let Some(chosen) = successors
            .into_iter()
            .filter(|successor| successor.transition == wanted.transition)
            .nth(wanted.ordinal)
        else {
            return Ok(None);
        };
        steps.push(Step {
            transition: wanted.transition.clone(),
            ordinal: wanted.ordinal,
            key: chosen.key,
        });
        current = chosen.state;
        states.push(current.clone());
    }

    let mut provenance = provenance.cloned();
    if let Some(provenance) = provenance.as_mut() {
        provenance.stopped = stopped.unwrap_or(StopReason::StepLimit);
    }
    let trace = Trace {
        initial: initial_key.clone(),
        steps,
        findings,
        provenance,
    };
    replay(&view, &trace, identity_limits()).map_err(|error| match error {
        ReplayError::KeyEncoding(refusal) => {
            ModelShrinkError::NotSimulated(NotSimulated::KeyEncoding(refusal))
        }
        other => ModelShrinkError::Replay(other),
    })?;
    let effects = effects_of_with_omitted_clause(system, &trace, omitted_clause).map_err(
        |error| match error {
            ModelSampleError::NotSimulated(cause) => ModelShrinkError::NotSimulated(cause),
            ModelSampleError::Replay(cause) => ModelShrinkError::Replay(cause),
        },
    )?;
    let boundary_partitions = system.boundary_partitions();
    Ok(Some((
        ModelTrace {
            trace,
            effects,
            attempted: None,
            boundary_partitions,
        },
        states,
    )))
}

/// FSL FR-634's enumeration over `system`: [`enumerate_request`] over its
/// [`ModelSystem::domains`] and the package's `TypeEnvironment`, keeping
/// the discovery tree and charging `meter` for every expansion and every
/// stored state and edge.
///
/// # Errors
///
/// [`NotSimulated`] as `explore_model` returns it.
///
/// [`enumerate_request`]: crate::simulation::enumerate_request
pub fn enumerate_model(
    system: &ModelSystem<'_>,
    position_limit: u64,
    bounds: crate::simulation::EnumerationBounds,
    meter: &mut filament_exact::Meter,
    poll: impl FnMut() -> bool,
) -> Result<crate::simulation::Enumeration<ModelTransition, ModelFinding>, NotSimulated> {
    let domains = system.rooted_domains();
    let roots: Vec<(NodeKey, Vec<u32>, &ValueType)> = domains
        .iter()
        .map(|(id, base, value_type)| (id.clone(), base.clone(), value_type))
        .collect();
    crate::simulation::enumerate_request(
        system,
        crate::simulation::EnumerationRequest {
            domains: &roots,
            types: system.types,
            position_limit,
            bounds,
            identity: identity_limits(),
        },
        meter,
        poll,
    )
}

/// The sampled runs of one FSL FR-634 `sample` item: what to draw.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SampledRuns {
    /// The run's seed.
    pub seed: u64,
    /// How many traces to draw, trace indices `0` to `samples - 1`.
    pub samples: u64,
    /// The most steps one trace takes.
    pub max_steps: usize,
}

/// What [`sample_model_runs`] drew: the traces in trace-index order, the
/// coverage they visited, and the FR-109 refusal that stopped the run, if
/// the meter refused before every requested trace was drawn in full.
pub struct SampledDraw {
    /// The traces drawn, the last one cut short when `stopped` is set.
    pub traces: Vec<Trace<ModelTransition, ModelFinding>>,
    /// What the traces visited.
    pub coverage: crate::simulation::Coverage,
    /// The meter's refusal, when it stopped the run.
    pub stopped: Option<filament_exact::Incomplete>,
}

/// FSL FR-634's sampled runs over `system`: `runs.samples` traces drawn by
/// `sampler` (the `filament.simulation.sampler/v1` generator) under
/// `runs.seed`, in trace-index order, with the coverage they visited.
///
/// Every state a trace reaches is charged to `meter` (`graph.edge`) before
/// the sampler draws from it, and every finished trace's retained keys
/// (`graph.result-retain`, their bytes), so the traces held never outgrow
/// the request's accounting limits: a refused charge ends the trace there
/// and the run, and [`SampledDraw::stopped`] names the counter. `poll` is
/// called once per trace; returning `true` stops before that trace.
///
/// # Errors
///
/// [`NotSimulated`] as `sample_request` returns it.
pub fn sample_model_runs(
    system: &ModelSystem<'_>,
    position_limit: u64,
    sampler: &DefinitionRef,
    runs: SampledRuns,
    meter: &mut filament_exact::Meter,
    mut poll: impl FnMut() -> bool,
) -> Result<SampledDraw, NotSimulated> {
    use filament_exact::{Charge, ChargePoint, length_amount};
    let domains = system.rooted_domains();
    let roots: Vec<(NodeKey, Vec<u32>, &ValueType)> = domains
        .iter()
        .map(|(id, base, value_type)| (id.clone(), base.clone(), value_type))
        .collect();
    let mut traces = Vec::new();
    let mut stopped = None;
    for trace in 0..runs.samples {
        if poll() {
            break;
        }
        let drawn = sample_request_until(
            system,
            SampleRequest {
                domains: &roots,
                types: system.types,
                position_limit,
                sampler,
                seed: runs.seed,
                trace,
                max_steps: runs.max_steps,
                identity: identity_limits(),
            },
            |_| match meter.charge(Charge::new(ChargePoint::GraphEdge)) {
                Ok(()) => false,
                Err(incomplete) => {
                    stopped = Some(incomplete);
                    true
                }
            },
        )?;
        let retained = std::iter::once(&drawn.initial)
            .chain(drawn.steps.iter().map(|step| &step.key))
            .map(|key| length_amount(key.as_bytes().len()))
            .fold(0u64, u64::saturating_add);
        if stopped.is_none() {
            if let Err(incomplete) =
                meter.charge(Charge::new(ChargePoint::GraphResultRetain).results(retained))
            {
                stopped = Some(incomplete);
            }
        }
        traces.push(drawn);
        if stopped.is_some() {
            break;
        }
    }
    let coverage = crate::simulation::sampled_coverage(system, &traces, identity_limits())
        .map_err(NotSimulated::from)?;
    Ok(SampledDraw {
        traces,
        coverage,
        stopped,
    })
}

/// The expansions of one run's sampled walks, kept so that a state the walks
/// reach again is expanded once. An expansion is a function of the state's
/// key and anchor alone (each expansion draws a fresh budget), so a hit is
/// the expansion the system would return. The run that makes the memo owns
/// it; it holds every state its walks reached and is dropped with the run.
#[derive(Default)]
pub struct WalkMemo {
    expansions: std::cell::RefCell<std::collections::HashMap<MemoAt, ModelExpansion>>,
}

/// A state's canonical key bytes and its anchor.
type MemoAt = (Vec<u8>, String);

type ModelExpansion = Expansion<ModelTransition, ModelSimState, ModelFinding>;

impl WalkMemo {
    /// An empty memo.
    pub fn new() -> Self {
        Self::default()
    }

    /// `system`'s expansion of `state`, computed once per state key and
    /// anchor.
    ///
    /// # Errors
    ///
    /// [`ExpansionStop`] as `system` returns it; a stop is not kept.
    pub fn successors(
        &self,
        system: &ModelSystem<'_>,
        state: &ModelSimState,
    ) -> Result<Expansion<ModelTransition, ModelSimState, ModelFinding>, ExpansionStop> {
        let limits = identity_limits();
        let Ok(bytes) = quire_canonical::to_vec(
            &system.key(state),
            quire_canonical::Limits::new(limits.input_bytes),
        ) else {
            return system.successors(state);
        };
        let at = (bytes, format!("{:?}", state.anchor));
        if let Some(found) = self.expansions.borrow().get(&at) {
            return Ok(found.clone());
        }
        let expanded = system.successors(state)?;
        self.expansions.borrow_mut().insert(at, expanded.clone());
        Ok(expanded)
    }
}

/// `system` with one initial state.
struct OneInitial<'s, 'a> {
    system: &'s ModelSystem<'a>,
    start: ModelSimState,
    memo: Option<&'s WalkMemo>,
}

impl TransitionSystem for OneInitial<'_, '_> {
    type State = ModelSimState;
    type TransitionId = ModelTransition;
    type Key = ModelKey;
    type Finding = ModelFinding;

    fn initial(&self) -> Vec<Self::State> {
        vec![self.start.clone()]
    }

    fn key(&self, state: &Self::State) -> Self::Key {
        self.system.key(state)
    }

    fn successors(
        &self,
        state: &Self::State,
    ) -> Result<Expansion<Self::TransitionId, Self::State, Self::Finding>, ExpansionStop> {
        match self.memo {
            Some(memo) => memo.successors(self.system, state),
            None => self.system.successors(state),
        }
    }
}

/// FR-166: FR-101's sampled trace over the view of `system` whose only
/// initial state is the subject's initial state `initial` (its index in the
/// subject's initial list), ending at the first state where `end` holds
/// ([`sample_request_until`]). `end` reads each state the walk visits, in
/// order, position 0 included.
///
/// # Errors
///
/// [`NotSimulated`] as `sample_request` returns it;
/// [`NotSimulated::EmptyInitial`] when `initial` is not an index of the
/// subject's initial states.
#[allow(
    clippy::too_many_arguments,
    reason = "FR-101's sample request members plus the walk's initial state and end condition"
)]
pub fn sample_model_walk(
    system: &ModelSystem<'_>,
    initial: usize,
    position_limit: u64,
    sampler: &DefinitionRef,
    seed: u64,
    trace: u64,
    max_steps: usize,
    memo: Option<&WalkMemo>,
    end: impl FnMut(&ModelSimState) -> bool,
) -> Result<Trace<ModelTransition, ModelFinding>, NotSimulated> {
    let start = system
        .initial
        .get(initial)
        .cloned()
        .ok_or(NotSimulated::EmptyInitial)?;
    let view = OneInitial {
        system,
        start,
        memo,
    };
    let domains = system.rooted_domains();
    let roots: Vec<(NodeKey, Vec<u32>, &ValueType)> = domains
        .iter()
        .map(|(id, base, value_type)| (id.clone(), base.clone(), value_type))
        .collect();
    sample_request_until(
        &view,
        SampleRequest {
            domains: &roots,
            types: system.types,
            position_limit,
            sampler,
            seed,
            trace,
            max_steps,
            identity: identity_limits(),
        },
        end,
    )
}

/// The effect of each step of a trace that replayed: the state each step's
/// transition and canonical key select carries the effect of the step that
/// reached
/// it.
#[allow(
    clippy::result_large_err,
    reason = "FR-120 pins ModelSampleError::Replay(ReplayError<ModelTransition>)"
)]
fn effects_of_with_omitted_clause(
    system: &ModelSystem<'_>,
    trace: &Trace<ModelTransition, ModelFinding>,
    omitted_clause: Option<&NodeKey>,
) -> Result<Vec<StepEffect>, ModelSampleError> {
    let view = ModelRunView::new(system, omitted_clause.cloned());
    let identity = identity_limits();
    let encoding = |error| ModelSampleError::NotSimulated(NotSimulated::KeyEncoding(error));
    let mut current = sorted_initial(&view, identity)
        .map_err(encoding)?
        .into_iter()
        .find(|item| item.key == trace.initial)
        .map(|item| item.state)
        .ok_or(ModelSampleError::Replay(ReplayError::UnknownInitial {
            expected: trace.initial.clone(),
        }))?;
    let mut effects = Vec::with_capacity(trace.steps.len());
    for (index, step) in trace.steps.iter().enumerate() {
        let missing = || {
            ModelSampleError::Replay(ReplayError::MissingTransition {
                step: index,
                transition: step.transition.clone(),
            })
        };
        let Expanded::Successors { successors, .. } =
            expand(&view, &current, identity).map_err(encoding)?
        else {
            return Err(missing());
        };
        let matched = successors
            .into_iter()
            .filter(|successor| successor.transition == step.transition)
            .nth(step.ordinal)
            .ok_or_else(missing)?;
        effects.push(matched.state.reached_by.clone().ok_or_else(missing)?);
        current = matched.state;
    }
    Ok(effects)
}

/// The `{handler, <operation text>}` anchor of a state a transition reached.
fn handler_anchor(operation: &str) -> SelectedAnchor {
    SelectedAnchor {
        kind: AnchorKind::Handler,
        name: operation.to_owned(),
    }
}

/// A synthesized state's observation document reference. The state key is
/// retained by the enclosing finding, so the source label is stable and
/// carries no duplicate content identity.
fn state_document() -> DocumentRef {
    DocumentRef {
        authority: "filament".to_owned(),
        identity: "filament.simulation.state".to_owned(),
    }
}

/// Each of `initial` with every combination of the clock-collection counts
/// `counts` on each object of the count's type: every count from `min` to
/// `max`, none when `min` exceeds `max`, or `min` alone with no maximum
/// (the timed subject ranges it further).
fn with_clock_counts(
    initial: Vec<ModelSimState>,
    counts: &[ClockCount],
    types: &filament_semantic_value::declaration::TypeEnvironment,
) -> Vec<ModelSimState> {
    let mut states = initial;
    for (type_key, field, min, max) in counts {
        // A `Sequence<Option<Clock>>`'s bits: every sequence of `min` to
        // `max` bits, each element's initial presence ranging over both.
        let bits = matches!(field.value_type, ValueType::Collection(_))
            .then(|| {
                domain::Domain::of(&field.value_type, &[], types, &domain::RecordOrders::new())
            })
            .flatten();
        let values: Vec<filament_exact::Value> = match bits {
            Some(domain) => (0..domain.len()).map_while(|at| domain.nth(at)).collect(),
            None => (*min..=max.unwrap_or(*min))
                .map(|count| filament_exact::Value::Integer(filament_exact::Integer::from(count)))
                .collect(),
        };
        states = ranged(states, type_key, &field.key, &values);
    }
    states
}

/// Each of `states` with the member `field` of every object of type
/// `type_key` taking each of `values`, every combination over the objects;
/// none when `values` is empty.
fn ranged(
    states: Vec<ModelSimState>,
    type_key: &DeclarationKey,
    field: &DeclarationKey,
    values: &[filament_exact::Value],
) -> Vec<ModelSimState> {
    let mut next = Vec::new();
    for state in states {
        let holders: Vec<(DeclarationKey, String)> = state
            .state
            .populations
            .iter()
            .flat_map(|(population, objects)| {
                objects
                    .iter()
                    .filter(|(_, object)| object.type_identity == *type_key)
                    .map(move |(key, _)| (population.clone(), key.clone()))
            })
            .collect();
        let mut partial = vec![state];
        for (population, key) in &holders {
            let mut grown = Vec::new();
            for state in partial {
                for value in values {
                    let mut held = state.clone();
                    if let Some(object) = held
                        .state
                        .populations
                        .get_mut(population)
                        .and_then(|objects| objects.get_mut(key))
                    {
                        object.fields.insert(field.clone(), value.clone());
                    }
                    grown.push(held);
                }
            }
            partial = grown;
        }
        next.extend(partial);
    }
    next
}

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;

    /// Quantity roots become integral before `classify_extent` decides
    /// whether the root is bounded.
    /// Provenance: QSL-2209.
    #[trace("TC-660")]
    #[test]
    fn quantity_root_types_classify_as_integer() {
        use filament_exact::{
            BoundedQuantityType, CarriedQuantityType, Integer, IntegerInterval, QuantityCarrier,
            RoundingMode, UnitId,
        };

        let unit = UnitId::declared(crate::test_ids::node_key("metre"));
        let quantity = ValueType::Quantity(unit.clone());
        let bounded = ValueType::BoundedQuantity(
            BoundedQuantityType::new(unit.clone(), None, None)
                .expect("empty bounds are admitted")
                .into(),
        );
        let carried = ValueType::CarriedQuantity(
            CarriedQuantityType::new(
                unit,
                QuantityCarrier::Integer {
                    domain: IntegerInterval::new(Integer::from(0_i64), Integer::from(10_i64))
                        .expect("ordered carrier bounds"),
                    rounding: RoundingMode::Exact,
                },
            )
            .into(),
        );

        for (kind, root) in [
            ("quantity", quantity),
            ("bounded", bounded),
            ("carried", carried),
        ] {
            assert_eq!(
                root_classification_type(&root),
                ValueType::Integer,
                "the {kind} root is classified as Integer",
            );
        }
    }

    /// FR-120 "Post-states": the two stops a model expansion makes carry the
    /// catalog's codes.
    #[trace("TC-472", "FR-120-AC-6")]
    #[test]
    fn the_stop_causes_are_the_catalog_codes() {
        assert_eq!(EXHAUSTED.code(), "resource_exhausted");
        assert_eq!(EXHAUSTED.cause(), "insufficient-next-charge");
        assert_eq!(BROKEN.code(), "runtime_invariant");
        assert_eq!(BROKEN.cause(), "established-invariant-broken");
    }

    /// W75 section 3.3 orders Text partitions before refusal lengths.
    #[trace("TC-2102", "FR-752-AC-7")]
    #[test]
    fn text_boundary_partitions_follow_length_then_content_order() {
        let text_type = filament_exact::TextType::new(0, 2, filament_exact::TextProfile::Nfc)
            .expect("ordered Text bounds");
        let actual: Vec<_> = text_boundary_specs(&text_type)
            .into_iter()
            .map(|(kind, length, text)| (kind, length, text))
            .collect();
        assert_eq!(
            actual,
            [
                (ModelBoundaryKind::TextEmpty, 0, String::new()),
                (ModelBoundaryKind::TextSingleton, 1, "\0".to_owned()),
                (ModelBoundaryKind::TextUpperEndpoint, 2, "\0\0".to_owned()),
                (ModelBoundaryKind::UpperRefusal, 3, "\0\0\0".to_owned()),
                (ModelBoundaryKind::TextNormalization, 1, "\u{e9}".to_owned(),),
                (
                    ModelBoundaryKind::TextNormalization,
                    2,
                    "\u{1e0d}\u{307}".to_owned(),
                ),
                (
                    ModelBoundaryKind::TextNormalization,
                    1,
                    "\u{ac00}".to_owned(),
                ),
                (ModelBoundaryKind::TextNormalization, 1, "\u{c5}".to_owned(),),
                (
                    ModelBoundaryKind::TextNormalization,
                    1,
                    "\u{fb01}".to_owned(),
                ),
            ]
        );
    }

    /// FR-190-AC-19: witness spellings are retained under each profile,
    /// duplicate equivalence-class representatives are emitted once.
    #[trace("FSpec-TC-1600", "FSpec-FR-190-AC-19")]
    #[test]
    fn text_normalization_partitions_follow_profile_retention() {
        let actual = |profile, max| {
            let text_type =
                filament_exact::TextType::new(0, max, profile).expect("ordered Text bounds");
            text_normalization_specs(&text_type)
                .into_iter()
                .map(|(_, _, text)| text)
                .collect::<Vec<_>>()
        };
        assert_eq!(
            actual(filament_exact::TextProfile::UnicodeScalars, 4),
            [
                "\u{e9}",
                "e\u{301}",
                "d\u{307}\u{323}",
                "d\u{323}\u{307}",
                "\u{ac00}",
                "\u{1100}\u{1161}",
                "\u{212b}",
                "\u{fb01}",
            ]
        );
        assert_eq!(
            actual(filament_exact::TextProfile::BinaryUtf8, 4),
            ["\u{e9}", "e\u{301}", "\u{ac00}", "\u{212b}", "\u{fb01}"]
        );
        assert_eq!(
            actual(filament_exact::TextProfile::Nfc, 4),
            [
                "\u{e9}",
                "\u{1e0d}\u{307}",
                "\u{ac00}",
                "\u{c5}",
                "\u{fb01}",
            ]
        );
        assert_eq!(
            actual(filament_exact::TextProfile::Nfd, 4),
            [
                "e\u{301}",
                "d\u{323}\u{307}",
                "\u{1100}\u{1161}",
                "A\u{30a}",
                "\u{fb01}",
            ]
        );
        assert_eq!(
            actual(filament_exact::TextProfile::Nfkc, 4),
            ["\u{e9}", "\u{1e0d}\u{307}", "\u{ac00}", "\u{c5}", "fi"]
        );
        assert_eq!(
            actual(filament_exact::TextProfile::Nfkd, 4),
            [
                "e\u{301}",
                "d\u{323}\u{307}",
                "\u{1100}\u{1161}",
                "A\u{30a}",
                "fi"
            ]
        );
        assert_eq!(actual(filament_exact::TextProfile::Nfd, 1), ["\u{fb01}"]);
    }
}
