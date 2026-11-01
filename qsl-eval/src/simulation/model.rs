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
mod cells;
mod constraint;
mod contract;
mod contracts;
mod domain;
pub(crate) mod encode;
mod footprints;
mod frames;
mod letters;
pub(crate) mod probabilistic;
mod refine;
mod state_predicate;
mod symbolic;
mod symmetry;
mod terminal;
mod timed;
mod walk;

use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex, Weak};

use qsl_foundation::diagnostic::Code;
use qsl_foundation::selection::DefinitionRef;
use qsl_foundation::IdentityLimits;
use qsl_package::CheckedPackage;
use qsl_semantics::check::OperationKey;
use qsl_semantics::family::ClassifyFailure;
use qsl_semantics::model::accounting::ModelNormalizationLimits;
use qsl_semantics::model::clause_observation::Observation;
use qsl_semantics::model::domain_package::OperationEffect;
use qsl_semantics::model::observation::{
    AdmissionFailure, AdmissionRecord, AnchorKind, DocumentRef, ObservationLimits, SelectedAnchor,
};
use qsl_semantics::model::operation::OperationDeclaration;
use qsl_semantics::model::state::{ModelState, StateField, StateModel};
use quire_canonical::Limits as CanonicalLimits;
use quire_checked::key::DeclarationKey;
use quire_exact::{
    EffectiveId, Incomplete, NodeKey, ObjectReference, Refusal, ScalarLimits, Undefined, ValueType,
};
use quire_semantic_value::catalog::{CatalogCode, InternalFault};
use quire_semantic_value::declaration::TypeEnvironment;
use quire_semantic_value::model_transition::ModelTransition;

use crate::model_subject::ModelInstance;
use crate::simulation::expansion::{Expansion, ExpansionStop};
use crate::simulation::explore::{explore_request, Exploration, Limits, TransitionSystem};
use crate::simulation::footprint::{Footprint, Location};
use crate::simulation::not_simulated::NotSimulated;
use crate::simulation::order::{expand, sorted_initial, Expanded};
use crate::simulation::protocol::WeakAccessShape;
use crate::simulation::reduction::{
    ConstraintValue, OfferedReductions, Permutation, SymmetryGroup,
};
use crate::simulation::sample::{sample_request, sample_request_until, SampleRequest};
use crate::simulation::trace::{replay, ReplayError, Trace};
use crate::value::QualifiedName;

pub(crate) use application::{FirstApplication, OperationRef};
pub use cells::Cell;
pub use contract::{
    ContractClause, ContractClauseKind, DeclaredStates, DeclaredStatesRefusal, OperationCases,
    OperationStep, PreconditionCase, UndecidedContract,
};
pub(crate) use contracts::{verdict, Verdict};
pub(crate) use domain::enumerable;
use encode::declaration_text;
pub use encode::ModelKey;
pub use footprints::{FootprintError, FootprintViolation, WithinStop};
pub use frames::judge::{
    operation_identity_text, AbstractApplication, Judged, Judgement, Rejection, ResolvedStep,
};
mod hyper_letters;
pub use hyper_letters::CheckedHyperLetters;
pub use letters::{ClauseLetters, LettersRefusal};
pub use probabilistic::{
    same_rewards, Action, Draw, NotMarkov, ScheduledIdentity, TransitionId, UndecidedSuccessor,
    Unsteppable, WeightedStep,
};
pub use refine::{RowReading, StateObservation};
pub use state_predicate::PredicateReading;
pub(crate) use symbolic::{ParameterValues, ResultValues, SymbolicOperation, SymbolicSubject};
pub use terminal::TerminalClass;
pub use timed::{
    replay_timed_sample, sample_timed, sample_timed_race, BehaviourLetters, BehaviourReading,
    ClockPlace, ClockValuation, DelayInterval, GuardedStep, MoveRefusal, PlaceConstraint,
    ScheduledWindow, TimeLock, TimedBehaviour, TimedPosition, TimedRace, TimedRaceDraw,
    TimedRaceStep, TimedRaceStop, TimedSample, TimedSampleDisagreement, TimedSampleReplay,
    TimedSampleStop, TimedSchedulerChoice, TimedSite, TimedState, TimedStop, TimedSubjectRefusal,
    TimedSystem, TimedTerminalClass, DEFAULT_MAX_CLOCKS,
};
pub(crate) use walk::{Applied, Budget};

/// `resource_exhausted`/`insufficient-next-charge`: a candidate count, or an
/// evaluation meter, ran out (FR-120 "Post-states", "Contract clauses").
const EXHAUSTED: CatalogCode = CatalogCode::new("resource_exhausted", "insufficient-next-charge");
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
    /// The S4 package.
    pub package: &'a CheckedPackage,
    /// FR-056's package input: domain package bytes by the identity each declares.
    pub packages: &'a qsl_semantics::model::intake::PackageInput,
    /// The normalization limits FR-106 admission takes.
    pub normalization: ModelNormalizationLimits,
    /// An FR-106 snapshot provision.
    pub snapshots: &'a qsl_semantics::model::observation::DocumentProvision,
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
}

/// A model state with its anchor (FR-120 "State, state key and anchor"). The
/// anchor, and the effect of the step that reached the state, are not part of
/// its key.
#[derive(Clone, Debug)]
pub struct ModelSimState {
    state: ModelState,
    anchor: SelectedAnchor,
    reached_by: Option<StepEffect>,
    /// The typed semantic key shared with clones whose model state is
    /// unchanged. Mutations replace this cache before changing a cell.
    semantic_cache: Arc<Mutex<Option<SemanticCacheEntry>>>,
}

#[derive(Debug)]
struct SemanticCacheEntry {
    context: Weak<()>,
    semantic: Arc<encode::Enc>,
    canonical_model_key: Option<Arc<Vec<u8>>>,
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
    FamilyUndefined(quire_semantic_value::catalog::UndefinedReason),
    /// The family's evaluation-time result was a refusal with this code.
    FamilyRefused(CatalogCode),
    /// A charge was unavailable.
    Incomplete(Incomplete),
    /// The clause completed a value that is no Boolean.
    NotBoolean,
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
                quire_semantic_value::catalog::UndefinedRecord {
                    reason: *reason,
                    fields: BTreeMap::new(),
                },
            )),
            Self::KernelRefused(_)
            | Self::FamilyRefused(_)
            | Self::Incomplete(_)
            | Self::NotBoolean => None,
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
    /// TS-4, QSpec FR-426 "Undefined protocol expressions"), when it is one:
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
    operations: &'a qsl_semantics::model::operation::OperationTable,
    package: &'a CheckedPackage,
    /// FR-411: the package's `terminal` declarations, read from its emitted
    /// `temporal`/`terminal` nodes.
    terminals: Vec<qsl_package::PackageTerminal<'a>>,
    model: StateModel,
    /// Distinguishes semantic-key caches for systems with different typing
    /// contexts, even when they encode the same `ModelSimState`.
    semantic_context: Arc<()>,
    universes: BTreeMap<DeclarationKey, Vec<String>>,
    initial: Vec<ModelSimState>,
    clauses: Clauses,
    applications: BTreeMap<EffectiveId, Vec<Application>>,
    fields: BTreeMap<DeclarationKey, Vec<StateField>>,
    populations: Vec<qsl_semantics::model::state::StatePopulation>,
    limits: ExpansionLimits,
    meter: ScalarLimits,
    footprints: footprints::FootprintCache,
    /// [`Self::universe_declaring`] per field, computed once.
    declaring: std::sync::Mutex<BTreeMap<DeclarationKey, Option<Vec<ObjectReference>>>>,
    /// The attempts' applications already computed, by observed state and
    /// attempt: see [`application::AppliedMemo`].
    applied: application::AppliedMemo,
    /// FR-185's random parameters and rewards, as FR-187 reads them.
    probabilistic: probabilistic::Tables,
    /// QSpec FR-154's unit bounds on fields, each an invariant of every
    /// state, in ascending constraint order.
    member_bounds: Vec<qsl_semantics::model::state::MemberBound>,
    /// Each record's slots in declaration order, for its enumerated domain
    /// (FR-120, FR-215).
    pub(crate) record_orders: domain::RecordOrders,
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
        catalog: quire_semantic_value::catalog::CatalogCode::new(code.as_str(), cause),
        fields: fields.iter().cloned().collect(),
        path: Vec::new(),
        kernel: None,
    }
}

fn fault(invariant: &'static str) -> AdmissionFailure {
    AdmissionFailure::Fault(quire_semantic_value::catalog::InternalFault::new(
        "model-simulation",
        invariant,
    ))
}

impl<'a> ModelSystem<'a> {
    /// The checked package this system is over: what a settlement joins a
    /// run's verdict on, such as its terminal declarations (FR-411).
    pub fn package(&self) -> &'a CheckedPackage {
        self.package
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
        domain::Domain::of(
            &field.value_type,
            references,
            self.types,
            &self.record_orders,
        )
    }

    pub(crate) fn typing(&self) -> encode::Typing<'_> {
        self.value_typing()
            .with_clock_members(&self.clock_counts, &self.clock_bits)
    }

    /// [`Self::typing`] for values alone, which hold no element-count
    /// member: it lives as long as the package.
    pub(crate) fn value_typing(&self) -> encode::Typing<'a> {
        encode::Typing::with_variants(self.types, self.package.graph().scope().variant_types())
    }

    /// Check the universes, then admit each initial document in the given
    /// order, returning the first failure (FR-120 "Initial states").
    ///
    /// # Errors
    ///
    /// FR-106's [`AdmissionFailure`]: `Refused` or `Incomplete`, one record.
    pub fn new(request: ModelSimulationRequest<'a>) -> Result<Self, AdmissionFailure> {
        let selections = request.package.graph().model_selections();
        Self::over(request, selections)
    }

    /// The model system's selected populations in ascending declaration-key
    /// order. This preserves a checked clause's population ordinal when a
    /// replay reconstructs an object-parameter binding.
    pub fn populations(&self) -> &[qsl_semantics::model::state::StatePopulation] {
        &self.populations
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
        let selected: Vec<_> = request
            .package
            .graph()
            .model_selections()
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
        Self::over(request, &selected)
    }

    fn over(
        request: ModelSimulationRequest<'a>,
        selections: &[qsl_semantics::model::domain_package::DomainPackageRef],
    ) -> Result<Self, AdmissionFailure> {
        let graph = request.package.graph();
        let types = graph.scope().types();
        let model = StateModel::new(selections, types, request.packages, request.normalization)?;
        let populations = model.populations();

        let mut universes = BTreeMap::new();
        for universe in request.universes {
            if !populations
                .iter()
                .any(|population| population.key == universe.population)
            {
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
                if quire_exact::ObjectId::new(key.as_str()).is_err() {
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
                semantic_cache: Default::default(),
            });
        }

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
                Self::applications_of(graph, &model, identity)?,
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
        for timed in graph.timed_models() {
            for clock in timed.clocks() {
                let [qsl_semantics::check::PlaceStep::Field(name), qsl_semantics::check::PlaceStep::Slot(shape)] =
                    clock.path()
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
                        ValueType::collection(quire_exact::CollectionType::new(
                            quire_exact::CollectionKind::Sequence,
                            ValueType::Boolean,
                            quire_exact::CardinalityBound::new(shape.min.min(max), max).ok(),
                        ))
                    }
                    Some(max) => quire_exact::IntegerInterval::new(
                        quire_exact::Integer::from(shape.min.min(max)),
                        quire_exact::Integer::from(max),
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
        for timed in graph.timed_models() {
            for clock in timed.clocks() {
                let (true, [qsl_semantics::check::PlaceStep::Field(name)]) =
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
                    quire_exact::Value::Boolean(false),
                    quire_exact::Value::Boolean(true),
                ],
            );
        }

        let record_orders = domain::record_orders(types, &model.record_orders());
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
        let system = Self {
            over: match selections {
                [only] => Some(only.identity.clone()),
                _ => None,
            },
            types,
            operations: graph.scope().operations(),
            package: request.package,
            terminals: qsl_package::resolve_terminals(request.package)
                .map_err(|_| fault("terminal-nodes-resolve"))?,
            clauses: Self::clauses_of(request.package)?,
            model,
            semantic_context: Arc::new(()),
            universes,
            initial,
            applications,
            fields,
            populations,
            limits: request.expansion,
            meter: request.meter,
            footprints: footprints::FootprintCache::default(),
            declaring: std::sync::Mutex::default(),
            applied: application::AppliedMemo::default(),
            probabilistic: probabilistic::Tables::of(graph.probabilistic_model())?,
            member_bounds,
            record_orders,
            clock_counts: counts,
            clock_bits: bits,
            bounded: std::sync::OnceLock::new(),
        };
        system.check_rewards()?;
        Ok(system)
    }

    /// The RFC 8785 key bytes and `quire.simulation.state-key/v1` state key of
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
        let bytes = self.canonical_model_key_bytes(state)?;
        Ok((
            bytes.as_ref().clone(),
            crate::simulation::StateKey::from_canonical_bytes(bytes.as_ref().clone()),
        ))
    }

    /// The full canonical state key used by the FR-101 simulator.
    pub(crate) fn canonical_state_key(
        &self,
        state: &ModelSimState,
    ) -> Result<crate::simulation::StateKey, crate::simulation::EncodingRefusal> {
        self.canonical_model_key_bytes(state)
            .map(|bytes| crate::simulation::StateKey::from_canonical_bytes(bytes.as_ref().clone()))
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
        graph: &qsl_semantics::check::CheckedGraph,
        model: &StateModel,
        identity: EffectiveId,
    ) -> Result<Vec<Application>, AdmissionFailure> {
        let operations = graph.scope().operations();
        let types = graph.scope().types();
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
            let qsl_semantics::model::operation::OperationLookup::Declared {
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
    fn clauses_of(package: &CheckedPackage) -> Result<Clauses, AdmissionFailure> {
        let mut clauses = Clauses::default();
        for clause in package.graph().state_clauses() {
            let info = ClauseInfo {
                name: clause.name().to_owned(),
                qualified: QualifiedName::unqualified(clause.name())
                    .map_err(|_| fault("clause-name"))?,
                identity: clause.identity(),
                context: clause.context(),
                operation: clause.operation().map(|operation| {
                    (
                        operation.declaring.clone(),
                        operation.declaration.name().to_owned(),
                    )
                }),
            };
            match clause.observation() {
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

    /// [`Self::domains`], each with the child-index path below its node at
    /// which the root sits (QSpec FR-331): empty for a population, `[0, f]`
    /// for a field, `[1, o, p]` for a parameter, `[2, o]` for a result.
    pub fn rooted_domains(&self) -> Vec<(NodeKey, Vec<u32>, ValueType)> {
        let mut roots = Vec::new();
        let mut root = |node: Option<NodeKey>, base: Vec<u32>, value_type: &ValueType| {
            let Some(node) = node else {
                return;
            };
            let bounded = if matches!(value_type, ValueType::Population(_))
                || domain::enumerable(value_type)
            {
                value_type.clone()
            } else if domain::finite(value_type, self.types) {
                // A finite enum, record or tuple (STD-508), its extent
                // read whole.
                ValueType::Boolean
            } else {
                ValueType::Integer
            };
            roots.push((node, base, bounded));
        };
        // QSpec FR-331 "Model member domain keys": a member's domain is the
        // model declaration node of its declaring type and the member's path
        // (field `[0, f]`, parameter `[1, o, p]`, result `[2, o]`), `f` and
        // `o` the ordinal among the type's own fields and operations in
        // UTF-8 name order and `p` the declared parameter position.
        let type_node = |id: &EffectiveId| {
            let original = id.original();
            qsl_semantics::model::key::model_node_key(
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
        let graph = self.package.graph();
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
                let Some(ordinal) = graph
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
        for timed in self.package.graph().timed_models() {
            for clock in timed.clocks() {
                let Some(qsl_semantics::check::PlaceStep::Field(name)) = clock.path().first()
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
                if let Some(ordinal) = graph
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
        for key in &granted {
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
                qsl_semantics::model::key::population_node_key(population).ok(),
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
                    qsl_semantics::model::key::population_node_key(population).map_err(|_| {
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
        population: &qsl_semantics::model::state::StatePopulation,
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

impl TransitionSystem for ModelSystem<'_> {
    type State = ModelSimState;
    type TransitionId = ModelTransition;
    type Key = ModelKey;
    type Finding = ModelFinding;

    fn initial(&self) -> Vec<Self::State> {
        self.initial.clone()
    }

    fn key(&self, state: &Self::State) -> Self::Key {
        encode::model_key_with_semantic(encode::Enc::Shared(self.semantic_enc_shared(state)))
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
    let domains = system.rooted_domains();
    let roots: Vec<(NodeKey, Vec<u32>, &ValueType)> = domains
        .iter()
        .map(|(id, base, value_type)| (id.clone(), base.clone(), value_type))
        .collect();
    let trace = sample_request(
        system,
        &roots,
        system.types,
        position_limit,
        sampler,
        seed,
        trace,
        max_steps,
        identity_limits(),
    )?;
    replay(system, &trace, identity_limits()).map_err(ModelSampleError::Replay)?;
    let effects = effects_of(system, &trace)?;
    Ok(ModelTrace { trace, effects })
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
fn effects_of(
    system: &ModelSystem<'_>,
    trace: &Trace<ModelTransition, ModelFinding>,
) -> Result<Vec<StepEffect>, ModelSampleError> {
    let identity = identity_limits();
    let encoding = |error| ModelSampleError::NotSimulated(NotSimulated::KeyEncoding(error));
    let mut current = sorted_initial(system, identity)
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
            expand(system, &current, identity).map_err(encoding)?
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
        authority: "quire".to_owned(),
        identity: "quire.simulation.state".to_owned(),
        revision_namespace: "quire.simulation.state-key".to_owned(),
        revision: "runtime".to_owned(),
    }
}

/// Each of `initial` with every combination of the clock-collection counts
/// `counts` on each object of the count's type: every count from `min` to
/// `max`, none when `min` exceeds `max`, or `min` alone with no maximum
/// (the timed subject ranges it further).
fn with_clock_counts(
    initial: Vec<ModelSimState>,
    counts: &[ClockCount],
    types: &quire_semantic_value::declaration::TypeEnvironment,
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
        let values: Vec<quire_exact::Value> = match bits {
            Some(domain) => (0..domain.len()).map_while(|at| domain.nth(at)).collect(),
            None => (*min..=max.unwrap_or(*min))
                .map(|count| quire_exact::Value::Integer(quire_exact::Integer::from(count)))
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
    values: &[quire_exact::Value],
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
}
