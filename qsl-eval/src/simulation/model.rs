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

mod contracts;
mod domain;
pub(crate) mod encode;
mod footprints;
mod frames;
mod terminal;
mod walk;

use std::collections::{BTreeMap, BTreeSet};

use qsl_foundation::diagnostic::Code;
use qsl_foundation::digest::{DigestRecord, WireNodeId};
use qsl_foundation::selection::DefinitionRef;
use qsl_foundation::{CatalogCode, IdentityLimits, InternalFault};
use qsl_package::CheckedPackage;
use qsl_semantics::family::ClassifyFailure;
use qsl_semantics::model::accounting::ModelNormalizationLimits;
use qsl_semantics::model::clause_observation::Observation;
use qsl_semantics::model::domain_package::OperationEffect;
use qsl_semantics::model::key::DeclarationKey;
use qsl_semantics::model::observation::{
    AdmissionFailure, AdmissionRecord, AnchorKind, DocumentRef, ObservationLimits, SelectedAnchor,
};
use qsl_semantics::model::operation::OperationDeclaration;
use qsl_semantics::model::state::{ModelState, StateField, StateModel};
use quire_canonical::Limits as CanonicalLimits;
use quire_exact::{
    EffectiveId, Incomplete, NodeKey, ObjectReference, Refusal, ScalarLimits, Undefined, ValueType,
};
use quire_semantic_value::declaration::TypeEnvironment;
use quire_semantic_value::model_transition::ModelTransition;

use crate::model_subject::ModelInstance;
use crate::simulation::expansion::{Expansion, ExpansionStop};
use crate::simulation::explore::{explore_request, Exploration, Limits, TransitionSystem};
use crate::simulation::not_simulated::NotSimulated;
use crate::simulation::order::{expand, sorted_initial, Expanded};
use crate::simulation::sample::sample_request;
use crate::simulation::trace::{replay, ReplayError, Trace};
use crate::value::QualifiedName;

pub use encode::ModelKey;
use encode::{declaration_enc, declaration_text, Enc};
pub use footprints::{FootprintError, FootprintViolation, WithinStop};
pub use terminal::TerminalClass;

/// `resource_exhausted`/`insufficient-next-charge`: a candidate count, or an
/// evaluation meter, ran out (FR-120 "Post-states", "Contract clauses").
const EXHAUSTED: CatalogCode = CatalogCode::new("resource_exhausted", "insufficient-next-charge");
/// `runtime_invariant`/`established-invariant-broken`: the simulator built an
/// input itself, so a refusal of it is a broken internal invariant.
const BROKEN: CatalogCode = CatalogCode::new("runtime_invariant", "established-invariant-broken");

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
    /// FR-056's package input: domain package bytes by `sha256-jcs` digest.
    pub packages: &'a BTreeMap<[u8; 32], Vec<u8>>,
    /// The normalization limits FR-106 admission takes.
    pub normalization: ModelNormalizationLimits,
    /// An FR-106 snapshot provision.
    pub snapshots: &'a BTreeMap<[u8; 32], Vec<u8>>,
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
    FamilyUndefined(qsl_foundation::diagnostic::UndefinedReason),
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
                qsl_foundation::diagnostic::UndefinedRecord {
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

    /// Whether this finding is a contract conjunction that was refused, so
    /// the successor relation is not exactly known (ADR-018 V-6).
    pub fn is_refused_contract(&self) -> bool {
        matches!(
            self,
            Self::ContractUndetermined { evaluation, .. }
                if evaluation.undefined_cause().is_none()
        )
    }
}

/// What a state or a successor set found (FR-120 "Contract clauses",
/// "Invariants"); FR-101 records these for expanded states.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ModelFinding {
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
    /// A `pre` or `post` clause of an application could not be decided.
    ContractUndetermined {
        /// The application the clause belongs to.
        transition: ModelTransition,
        /// The candidate's state-key digest for a postcondition; `None` for a
        /// precondition.
        candidate: Option<DigestRecord>,
        /// The clause's name.
        clause: String,
        /// Why it was not decided.
        evaluation: ClauseOutcome,
    },
}

/// A sampled trace and the effect of each of its steps (FR-120 "Outputs").
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelTrace {
    /// FR-181's transition identities, state-key digests and findings.
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
    types: &'a TypeEnvironment,
    operations: &'a qsl_semantics::model::operation::OperationTable,
    package: &'a CheckedPackage,
    model: StateModel,
    universes: BTreeMap<DeclarationKey, Vec<String>>,
    initial: Vec<ModelSimState>,
    clauses: Clauses,
    applications: BTreeMap<EffectiveId, Vec<Application>>,
    fields: BTreeMap<DeclarationKey, Vec<StateField>>,
    populations: Vec<qsl_semantics::model::state::StatePopulation>,
    limits: ExpansionLimits,
    meter: ScalarLimits,
    footprints: footprints::FootprintCache,
}

fn refused(code: Code, cause: &'static str, fields: &[(&'static str, String)]) -> AdmissionFailure {
    AdmissionFailure::Refused(record(code, cause, fields))
}

fn record(code: Code, cause: &'static str, fields: &[(&'static str, String)]) -> AdmissionRecord {
    AdmissionRecord {
        code,
        cause,
        fields: fields.iter().cloned().collect(),
        path: Vec::new(),
        kernel: None,
    }
}

fn fault(invariant: &'static str) -> AdmissionFailure {
    AdmissionFailure::Fault(qsl_foundation::diagnostic::InternalFault::new(
        "model-simulation",
        invariant,
    ))
}

impl<'a> ModelSystem<'a> {
    /// Check the universes, then admit each initial document in the given
    /// order, returning the first failure (FR-120 "Initial states").
    ///
    /// # Errors
    ///
    /// FR-106's [`AdmissionFailure`]: `Refused` or `Incomplete`, one record.
    pub fn new(request: ModelSimulationRequest<'a>) -> Result<Self, AdmissionFailure> {
        let graph = request.package.graph();
        let types = graph.scope().types();
        let model = StateModel::new(
            graph.model_selections(),
            types,
            request.packages,
            request.normalization,
        )?;
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
                    return Err(AdmissionFailure::Incomplete(record(
                        Code::IncompletePopulation,
                        "incomplete-scope",
                        &[("population", population.node.clone())],
                    )));
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

        let mut fields = BTreeMap::new();
        let mut applications = BTreeMap::new();
        for declaration in types.object_types() {
            let identity = declaration.key();
            let Some(type_key) = model.type_key(identity).cloned() else {
                continue;
            };
            let declared = model
                .fields(&type_key)?
                .ok_or_else(|| fault("type-fields"))?;
            fields.insert(type_key, declared);
            applications.insert(identity, Self::applications_of(graph, &model, identity)?);
        }

        Ok(Self {
            types,
            operations: graph.scope().operations(),
            package: request.package,
            clauses: Self::clauses_of(request.package)?,
            model,
            universes,
            initial,
            applications,
            fields,
            populations,
            limits: request.expansion,
            meter: request.meter,
            footprints: footprints::FootprintCache::default(),
        })
    }

    /// FR-101's `quire.simulation.state-key/v1` digest of `state`, and its
    /// full key bytes, under the identity limits exploration encodes with.
    ///
    /// # Errors
    ///
    /// [`crate::simulation::EncodingRefusal`] when the key has no RFC 8785
    /// encoding under those limits.
    pub fn state_key(
        &self,
        state: &ModelSimState,
    ) -> Result<(Vec<u8>, DigestRecord), crate::simulation::EncodingRefusal> {
        let (key, digest) = crate::simulation::key::state_key(
            &encode::model_key(&self.model, &state.state),
            identity_limits(),
        )?;
        Ok((key.as_bytes().to_vec(), digest))
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
            if types.conforms(identity, declaring.key()) {
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
            } = operations.resolve(types, identity, &name)
            else {
                continue;
            };
            let owner = model
                .type_key(declaring)
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
                    (operation.declaring, operation.declaration.name().to_owned())
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
    pub fn domains(&self) -> Vec<(WireNodeId, ValueType)> {
        let mut roots = Vec::new();
        let mut root = |naming: Enc, value_type: &ValueType| {
            let Ok(digest) = quire_canonical::sha256(&naming, CanonicalLimits::new(u64::MAX))
            else {
                return;
            };
            let bounded = if matches!(value_type, ValueType::Population(_))
                || domain::enumerable(value_type)
            {
                value_type.clone()
            } else {
                ValueType::Integer
            };
            roots.push((WireNodeId::from_digest(*digest.as_bytes()), bounded));
        };
        let mut granted: BTreeSet<DeclarationKey> = BTreeSet::new();
        for declaration in self.types.object_types() {
            for operation in self.operations.declared_by(declaration.key()) {
                let Some(owner) = self.model.type_key(declaration.key()) else {
                    continue;
                };
                let key = DeclarationKey {
                    package: owner.package.clone(),
                    node: format!("{}/{}", owner.node, operation.name()),
                };
                for (name, value_type) in operation.parameters() {
                    root(
                        Enc::Object(vec![
                            ("operation", declaration_enc(&key)),
                            ("parameter", Enc::text(name.clone())),
                        ]),
                        value_type,
                    );
                }
                if let Some(result) = operation.result() {
                    root(
                        Enc::Object(vec![
                            ("operation", declaration_enc(&key)),
                            ("result", Enc::text("result")),
                        ]),
                        result,
                    );
                }
                let effect = operation.effect();
                granted.extend(effect.written_fields().cloned());
                for created in self.creatable_types(effect) {
                    if let Some(type_fields) = self.fields.get(&created) {
                        granted.extend(type_fields.iter().map(|field| field.key.clone()));
                    }
                }
            }
        }
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
        for key in &granted {
            if let Some(value_type) = field_types.get(key) {
                root(declaration_enc(key), value_type);
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
            root(declaration_enc(population), &ValueType::Population(bound));
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
            let admits = population.member_types.iter().any(|member| {
                self.model.type_identity(member).is_some_and(|member| {
                    self.types.conforms(member, *parameter)
                        || self.types.conforms(*parameter, member)
                })
            });
            if admits {
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
                let digest = quire_canonical::sha256(
                    &declaration_enc(population),
                    CanonicalLimits::new(u64::MAX),
                )
                .map_err(|_| {
                    NotSimulated::Extent(ClassifyFailure::Fault(InternalFault::new(
                        "model-simulation",
                        "population-root-digest",
                    )))
                })?;
                unbounded.push((
                    WireNodeId::from_digest(*digest.as_bytes()),
                    ValueType::Population(None),
                ));
            }
        }
        let roots: Vec<(WireNodeId, &ValueType)> =
            unbounded.iter().map(|(id, root)| (*id, root)).collect();
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
        encode::model_key(&self.model, &state.state)
    }

    fn successors(
        &self,
        state: &Self::State,
    ) -> Result<Expansion<Self::TransitionId, Self::State, Self::Finding>, ExpansionStop> {
        walk::expand_state(self, state, None)
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
    let domains = system.domains();
    let roots: Vec<(WireNodeId, &ValueType)> = domains
        .iter()
        .map(|(id, value_type)| (*id, value_type))
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
    let domains = system.domains();
    let roots: Vec<(WireNodeId, &ValueType)> = domains
        .iter()
        .map(|(id, value_type)| (*id, value_type))
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
    let domains = system.domains();
    let roots: Vec<(WireNodeId, &ValueType)> = domains
        .iter()
        .map(|(id, value_type)| (*id, value_type))
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

/// The effect of each step of a trace that replayed: the state each step's
/// transition and digest select carries the effect of the step that reached
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
        .find(|item| item.digest == trace.initial)
        .map(|item| item.state)
        .ok_or(ModelSampleError::Replay(ReplayError::UnknownInitial {
            expected: trace.initial,
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
            .find(|successor| {
                successor.transition == step.transition && successor.digest == step.key
            })
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

/// A synthesized state's document reference (FR-120 "Frame and observation
/// seam"): authority `quire`, identity `quire.simulation.state/<digest>`, and
/// the state-key digest as its digest.
fn state_document(digest: &DigestRecord) -> DocumentRef {
    DocumentRef {
        authority: "quire".to_owned(),
        identity: format!("quire.simulation.state/{}", digest.hex()),
        revision_namespace: "quire.simulation.state-key".to_owned(),
        revision: digest.hex(),
        digest: *digest.as_bytes(),
    }
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
