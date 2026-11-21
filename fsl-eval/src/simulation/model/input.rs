// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-531, FR-940: [`ModelCheckInput`], what a model check reads of a
//! checked package, and the two routes that build it.
//!
//! A model check reads the package's declarations as tables (types,
//! operations, model selections, timed and probabilistic models, state
//! clauses, terminals) and evaluates its checked forms through one
//! [`ModelEvaluator`]. The source route builds the input from the source
//! package's `CheckedGraph` ([`ModelCheckInput::source`], FR-531). The package
//! route builds it once from a [`BarePackage`]'s admitted nodes
//! ([`PackageModelInput::of`], FR-940); it never builds a `CheckedGraph`, and
//! a node whose package form is not read yet refuses the input with a typed
//! [`PendingPackageForm`] rather than being left out.

use std::collections::BTreeSet;

use filament_eval::{CallFailure, Evaluation};
use filament_exact::{
    EffectiveId, Meter, NodeKey, ObjectReference, ScalarLimits, Value, ValueType,
};
use filament_semantic_value::catalog::InternalFault;
use filament_semantic_value::declaration::TypeEnvironment;
use fsl_package::{
    BarePackage, CheckedNodeId, CheckedPackage, ClauseBinder, PackageTerminalDeclaration,
    TerminalResolveRefusal, UnreadModelForm,
};
use fsl_semantics::check::footprint::{
    derive_atom_reads, derive_attempt_footprint_fallback_with_reference_literals,
    derive_constraint_reads, derive_footprint, derive_guard_reads, derive_node_reads,
    derive_selected_footprint_with_reference_literals, derive_slot_reads, reference_literals_of,
    FootprintRefusal, ReferenceLiterals, SymbolicFootprint, SymbolicLocation,
};
use fsl_semantics::check::{
    CheckedBody, CheckedHyperClause, CheckedNode, CheckedProbabilisticModel, CheckedProtocol,
    CheckedProtocolNodeKeys, CheckedRow, CheckedStateClause, CheckedTemporalClause,
    CheckedTerminalPredicate, EnumBinding, FormulaNodeId, ModelTimeSource, Reward, StatePredicate,
    TimedModel, VariantTypeDeclaration, Written,
};
use fsl_semantics::model::clause_observation::Observation;
use fsl_semantics::model::domain_package::{DomainPackage, DomainPackageRef};
use fsl_semantics::model::object_environment::ObjectEnvironment;
use fsl_semantics::model::observation::{AdmittedObservations, ModelSelections};
use fsl_semantics::model::operation::{OperationDeclaration, OperationTable};
use fsl_semantics::model::state::StateModel;

use crate::value::{
    CheckedPackageEvaluation, ClauseEvaluation, ClauseSetEvaluation, PostTargets, ProtocolSlot,
    ProtocolSlotEvaluation, ProtocolSupply, QualifiedName, WrittenCollection,
};

/// The reads a body derivation reports, and the reference literals the body
/// names, which the subject's bindings turn into membership reads.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BodyReads {
    /// The symbolic locations read.
    pub reads: BTreeSet<SymbolicLocation>,
    /// The (object type, identity) of each reference literal of the body.
    pub literals: Vec<(EffectiveId, Vec<u8>)>,
}

/// One state clause as a model check reads it: its name, node, context,
/// operation and observation (FR-107, FR-120 "Contract clauses").
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StateClauseEntry {
    /// The clause's declared name.
    pub name: String,
    /// The clause's node identity.
    pub identity: NodeKey,
    /// The declaration's `claim` occurrence of the node.
    pub claim: filament_exact::Origin,
    /// The object type the clause constrains.
    pub context: EffectiveId,
    /// The declaring type and name of the operation the clause belongs to;
    /// `None` for an invariant.
    pub operation: Option<(EffectiveId, String)>,
    /// Which state the clause observes: an invariant the current state, a
    /// precondition the pre-state, a postcondition the post-state.
    pub observation: Observation,
    /// Whether the clause is an `invariant` (and no `constraint`).
    pub invariant: bool,
}

/// One `terminal` declaration as a model check reads it (FR-411, FR-940).
/// The model is named by its source alias on the source route and by the
/// package identity of its selection on the package route.
#[derive(Clone, Debug)]
pub struct ModelTerminal<'a> {
    /// The model, by selection identity.
    pub model: super::TerminalModel,
    /// The declaration's `temporal`/`terminal` node.
    pub node: filament_exact::NodeKey,
    /// The declaration.
    pub declaration: ModelTerminalDeclaration<'a>,
}

/// A [`ModelTerminal`]'s declaration.
#[derive(Clone, Copy, Debug)]
pub enum ModelTerminalDeclaration<'a> {
    /// Every terminal state is intended.
    Any,
    /// A terminal state is intended when the predicate holds at it.
    When(TerminalWhen<'a>),
}

/// A `terminal when` predicate in the form its route reads.
#[derive(Clone, Copy)]
pub enum TerminalWhen<'a> {
    /// The source route's checked body.
    Source(&'a CheckedTerminalPredicate),
    /// The package route's projected condition.
    Package(&'a fsl_package::BareTerminalPredicate),
}

impl std::fmt::Debug for TerminalWhen<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Source(_) => f.write_str("TerminalWhen::Source"),
            Self::Package(_) => f.write_str("TerminalWhen::Package"),
        }
    }
}

impl TerminalWhen<'_> {
    /// The object type `self` ranges over, when the predicate names one.
    pub fn context(&self) -> Option<&EffectiveId> {
        match self {
            Self::Source(predicate) => match predicate.parameters().first() {
                Some((_, ValueType::Reference(context))) => Some(context),
                _ => None,
            },
            Self::Package(predicate) => predicate.reads_self().then(|| predicate.context()),
        }
    }
}

/// What a model check reads of a checked package (FR-531, FR-940): the
/// declaration tables and the evaluator of its checked forms.
#[derive(Clone)]
pub struct ModelCheckInput<'a> {
    /// The package's type environment.
    pub types: &'a TypeEnvironment,
    /// The package's variant types, which a value's canonical form names.
    pub variant_types: &'a [VariantTypeDeclaration],
    /// The package's enumerations, which a value's canonical form names.
    pub enums: &'a [EnumBinding],
    /// The operations each object type declares.
    pub operations: &'a OperationTable,
    /// The domain packages the package selects, ascending by identity
    /// (FR-533).
    pub model_selections: &'a [DomainPackageRef],
    /// The source packages the unit's own `object` declarations build
    /// (FR-569), which have no document.
    pub source_packages: &'a [DomainPackage],
    /// The timed model of each time source, in time-source order (FR-230).
    pub timed_models: &'a [TimedModel],
    /// The model time sources (FR-230).
    pub time_sources: &'a [ModelTimeSource],
    /// The `random`, `reward` and `workload` declarations (FR-185).
    pub probabilistic: &'a CheckedProbabilisticModel,
    /// The state clauses, in the package's order.
    pub state_clauses: Vec<StateClauseEntry>,
    /// The `terminal` declarations (FR-411), or the refusal of their
    /// resolution, which the model system reports when it is built.
    pub terminals: Result<Vec<ModelTerminal<'a>>, TerminalResolveRefusal>,
    /// The rewards of the package route's `reward` nodes, projected once;
    /// `None` on the source route, which evaluates its checked rewards.
    pub read_rewards: Option<&'a PackageRewards>,
    /// The evaluator of the package's checked forms.
    pub evaluator: &'a dyn ModelEvaluator,
    /// The source package the tables were read from; `None` on the package
    /// route.
    pub source: Option<&'a CheckedPackage>,
    /// The admitted package the tables were read from on the package route;
    /// `None` on the source route (FR-940).
    pub admitted: Option<&'a fsl_package::CheckedPackageV2>,
}

/// A protocol definition in the form the selected analysis route reads.
/// Package definitions borrow only the admitted V2 nodes; they do not
/// reconstruct a source checked graph.
pub enum ProtocolDefinition<'a> {
    /// The source route's checked semantic form.
    Source(&'a CheckedProtocol),
    /// The package route's typed protocol-node form.
    Package(Box<fsl_package::protocol_view::ProtocolExecutionView<'a>>),
}

impl<'a> ModelCheckInput<'a> {
    /// The source route's input (FR-531): every table read from the source
    /// package's `CheckedGraph`, and the package itself as the evaluator.
    pub fn source(package: &'a CheckedPackage) -> Self {
        let graph = package.graph();
        let scope = graph.scope();
        Self {
            types: scope.types(),
            variant_types: scope.variant_types(),
            enums: scope.enums(),
            operations: scope.operations(),
            model_selections: graph.model_selections(),
            source_packages: graph.source_packages(),
            timed_models: graph.timed_models(),
            time_sources: graph.time_sources(),
            probabilistic: graph.probabilistic_model(),
            state_clauses: graph
                .state_clauses()
                .iter()
                .map(|clause| StateClauseEntry {
                    name: clause.name().to_owned(),
                    identity: clause.identity(),
                    claim: clause.claim().clone(),
                    context: clause.context(),
                    operation: clause.operation().map(|operation| {
                        (
                            operation.declaring.clone(),
                            operation.declaration.name().to_owned(),
                        )
                    }),
                    observation: clause.observation(),
                    invariant: clause.kind() == fsl_forms::StateClauseKind::Invariant,
                })
                .collect(),
            terminals: fsl_package::resolve_terminals(package).map(|terminals| {
                terminals
                    .into_iter()
                    .map(|terminal| ModelTerminal {
                        model: super::TerminalModel::new(terminal.package, terminal.model)
                            .with_ordinal(terminal.ordinal),
                        node: terminal.node.clone(),
                        declaration: match terminal.declaration {
                            PackageTerminalDeclaration::Any => ModelTerminalDeclaration::Any,
                            PackageTerminalDeclaration::When(predicate) => {
                                ModelTerminalDeclaration::When(TerminalWhen::Source(predicate))
                            }
                        },
                    })
                    .collect()
            }),
            read_rewards: None,
            evaluator: package,
            source: Some(package),
            admitted: None,
        }
    }

    /// FR-411, FR-940: the model of the `terminal when` predicate whose body
    /// is at `locus`, on either route: the source route's checked body, the
    /// package route's projection of the condition node, both located under
    /// the declaration's `terminal N predicate` site.
    ///
    /// # Errors
    ///
    /// [`TerminalResolveRefusal`] when the terminal nodes did not resolve.
    pub fn terminal_at(
        &self,
        locus: &filament_semantic_value::location::ExpressionLocation,
    ) -> Result<Option<super::TerminalModel>, TerminalResolveRefusal> {
        let terminals = self.terminals.as_ref().map_err(Clone::clone)?;
        Ok(terminals.iter().find_map(|terminal| {
            let ModelTerminalDeclaration::When(when) = terminal.declaration else {
                return None;
            };
            let at = match when {
                TerminalWhen::Source(predicate) => Some(predicate.body().location()),
                TerminalWhen::Package(predicate) => {
                    let projection = predicate.projection();
                    projection.location(projection.root())
                }
            };
            (at == Some(locus)).then(|| terminal.model.clone())
        }))
    }

    /// FR-411 "Deadlock-freedom verdict join": each declared terminal's node
    /// key, by its model's selection identity, on either route.
    pub fn terminal_keys(&self) -> fsl_semantics::check::TerminalKeys {
        fsl_semantics::check::TerminalKeys::of(
            self.terminals
                .as_ref()
                .map(Vec::as_slice)
                .unwrap_or_default()
                .iter()
                .map(|terminal| {
                    (
                        (
                            terminal.model.identity().to_owned(),
                            terminal.model.ordinal(),
                        ),
                        terminal.node.clone(),
                    )
                }),
        )
    }

    /// FR-940: resolve a protocol through this input's selected route, keeping
    /// source checked forms and package node forms distinct.
    pub fn protocol_definition(&self, name: &str) -> Option<ProtocolDefinition<'_>> {
        self.evaluator
            .protocol(name)
            .map(ProtocolDefinition::Source)
            .or_else(|| {
                self.evaluator
                    .package_protocol(name)
                    .map(|form| ProtocolDefinition::Package(Box::new(form)))
            })
    }

    /// The packages an admission of this package's observations
    /// re-normalizes: [`Self::model_selections`] and [`Self::source_packages`].
    pub fn model_inputs(&self) -> ModelSelections<'a> {
        ModelSelections::new(self.model_selections, self.source_packages)
            .with_clock_places(self.timed_models)
    }

    /// The timed model of the model alias `model`.
    pub fn timed_model(&self, model: &str) -> Option<&'a TimedModel> {
        self.timed_models
            .iter()
            .find(|timed| timed.model() == model)
    }

    /// The source package, for an engine that has no package form yet
    /// (the SMT encoding); `None` on the package route.
    pub fn source_package(&self) -> Option<&'a CheckedPackage> {
        self.source
    }

    /// The admitted package of the package route (FR-940); `None` on the
    /// source route.
    pub fn admitted_package(&self) -> Option<&'a fsl_package::CheckedPackageV2> {
        self.admitted
    }
}

/// Why the package route built no model-check input (FR-940).
#[derive(Debug, thiserror::Error)]
pub enum PendingPackageForm {
    /// The package holds `node`, a form the package route does not read
    /// yet. No input that leaves the node out is built.
    #[error("the package route does not read the {} form of node {node:?} yet", form.name())]
    Form {
        /// The form.
        form: UnreadModelForm,
        /// The first node of that form, in the package's node order.
        node: CheckedNodeId,
    },
    /// The package's declarations could not be read from its nodes and
    /// identity rows.
    #[error("the package's declarations are not readable: {0:?}")]
    World(fsl_package::BareSelectionFailure),
    /// The supplied domain packages the package selects do not re-admit
    /// against its types (FR-106).
    #[error("the selected domain packages do not admit: {0:?}")]
    Model(fsl_semantics::model::observation::AdmissionFailure),
    /// The selected domain packages declare an operation assembly does not
    /// type (FR-120 "Operations").
    #[error("the selected domain packages' operations are not typed: {0:?}")]
    Operations(fsl_semantics::check::AssemblyRefusal),
    /// The package's source-object rows do not describe source packages the
    /// `probabilistic` nodes join to (FSL FR-740 "Source objects"), at the
    /// row's node.
    #[error("the package's source objects are not readable: {0}")]
    SourceModel(fsl_package::SourceModelFault),
    /// The package's `probabilistic` nodes do not read (FSL FR-740).
    #[error("the package's probabilistic nodes are not readable: {0}")]
    ProbabilisticNodes(fsl_package::ProbabilisticReadFault),
    /// The clause function of a `reward` node does not project: its types or
    /// equality are not readable from the package, or no function is at the
    /// node.
    #[error("the package's reward does not project: {0:?}")]
    Reward(fsl_package::BareSelectionFailure),
    /// The package's random parameters and workloads do not rebuild against
    /// the selected domain packages (FSL FR-740).
    #[error("the package's probabilistic model does not rebuild: {0}")]
    Probabilistic(fsl_semantics::check::PackageProbabilisticRefusal),
}

/// Why the package route built no package form of a temporal clause (FR-940).
#[derive(Debug, thiserror::Error)]
pub enum PendingClauseForm {
    /// The clause's facts were not read from the package.
    #[error("the package form of the clause was not read: {0:?}")]
    Read(fsl_package::BareClauseRefusal),
    /// The facts read do not rebuild the clause.
    #[error("the package form of the clause was not built: {0}")]
    Build(fsl_semantics::check::PackageClauseRefusal),
}

impl PendingClauseForm {
    /// The fact the clause carries that its package form does not read yet,
    /// with the node that carries it, when that is why no form was built
    /// (FR-940): a fairness set, an interval, an aggregate atom, a shared
    /// formula node, a profile other than the infinite-trace one or a
    /// capture.
    pub fn pending_fact(&self) -> Option<(&'static str, &fsl_package::CheckedNodeId)> {
        match self {
            Self::Read(fsl_package::BareClauseRefusal::Pending { fact, node }) => {
                Some((fact, node))
            }
            Self::Read(fsl_package::BareClauseRefusal::Selection(_)) | Self::Build(_) => None,
        }
    }
}

/// What the package route reads to rebuild a package's probabilistic model: the
/// declarations, the selected domain packages' model and operations, and the
/// model itself.
struct ProbabilisticParts {
    world: fsl_package::BareWorld,
    model: StateModel,
    model_selections: Vec<DomainPackageRef>,
    operations: OperationTable,
    probabilistic: CheckedProbabilisticModel,
}

impl ProbabilisticParts {
    /// With `source_objects`, the unit's own object types are declared in the
    /// type environment and their rebuilt source packages are selected beside
    /// the domain packages (FSL FR-740 "Source objects").
    fn read(
        package: &BarePackage<'_>,
        packages: &fsl_semantics::model::intake::PackageInput,
        normalization: fsl_semantics::model::accounting::ModelNormalizationLimits,
        source_objects: bool,
    ) -> Result<Self, PendingPackageForm> {
        let world = if source_objects {
            package.world_with_source_objects()
        } else {
            package.world()
        }
        .map_err(PendingPackageForm::World)?;
        let model_selections = fsl_package::AnalysisView::model_selections(package);
        let source = if source_objects {
            package
                .source_model()
                .map_err(PendingPackageForm::SourceModel)?
        } else {
            fsl_package::SourceModel::default()
        };
        let mut selections = model_selections.clone();
        selections.extend(
            source
                .packages
                .iter()
                .map(|source| source.model_selection.clone()),
        );
        let model = StateModel::new(
            ModelSelections::new(&selections, &source.packages),
            world.types(),
            packages,
            normalization,
        )
        .map_err(PendingPackageForm::Model)?;
        let operations = model
            .operation_table(&std::collections::BTreeMap::new())
            .map_err(PendingPackageForm::Operations)?;
        let nodes = package
            .probabilistic()
            .map_err(PendingPackageForm::ProbabilisticNodes)?;
        // A probabilistic node is rebuilt only over an operation whose
        // parameter types the rows spell readably.
        if let Some(unread) = source.unread_named(&nodes) {
            return Err(PendingPackageForm::SourceModel(
                fsl_package::SourceModelFault::UnreadParameter {
                    operation: format!("{}::{}", unread.object, unread.operation),
                    parameter: unread.parameter.clone(),
                    node: unread.node.clone(),
                },
            ));
        }
        let probabilistic = model
            .probabilistic_model(&operations, &nodes)
            .map_err(PendingPackageForm::Probabilistic)?;
        Ok(Self {
            world,
            model,
            model_selections,
            operations,
            probabilistic,
        })
    }
}

/// The rewards of a package's `reward` nodes, each projected once, in the
/// order of [`CheckedProbabilisticModel::read_rewards`] (FR-187).
pub struct PackageRewards {
    world: fsl_package::BareWorld,
    projected: Vec<crate::value::ProjectedReward>,
}

impl PackageRewards {
    fn of(
        package: &BarePackage<'_>,
        world: fsl_package::BareWorld,
        model: &CheckedProbabilisticModel,
    ) -> Result<Self, PendingPackageForm> {
        let projected = model
            .read_rewards()
            .iter()
            .map(|reward| {
                crate::value::ProjectedReward::of(package, reward)
                    .map_err(PendingPackageForm::Reward)
            })
            .collect::<Result<_, _>>()?;
        Ok(Self { world, projected })
    }

    /// How many rewards are projected.
    pub(crate) fn len(&self) -> usize {
        self.projected.len()
    }

    /// The projection of the reward at `index` of the model's read rewards.
    pub(crate) fn reward(&self, index: usize) -> Option<&crate::value::ProjectedReward> {
        self.projected.get(index)
    }

    /// The package's declarations the rewards evaluate against.
    pub(crate) fn world(&self) -> &fsl_package::BareWorld {
        &self.world
    }
}

/// The package route's model-check input, built once from a
/// [`BarePackage`]'s admitted nodes (FR-940): it owns the tables it read and
/// lends them as a [`ModelCheckInput`].
pub struct PackageModelInput<'p> {
    package: &'p BarePackage<'p>,
    world: fsl_package::BareWorld,
    model: StateModel,
    model_selections: Vec<DomainPackageRef>,
    operations: OperationTable,
    probabilistic: CheckedProbabilisticModel,
    rewards: PackageRewards,
    terminals: Vec<fsl_package::BareTerminal>,
    state_clauses: Vec<StateClauseEntry>,
}

impl<'p> PackageModelInput<'p> {
    /// The package route's input over `package`, whose supply was attached
    /// (FR-517 handoff identity rows, FR-531), with the domain package
    /// documents `packages` bound to its model selections (FR-850
    /// "Admission"). Its operation table is the one S3's assembly declares for
    /// those packages ([`fsl_semantics::check::model_operation_table`]), so a
    /// step applies the operations and effects the source route applies.
    ///
    /// Its random parameters and workloads are the ones the package's
    /// `probabilistic` nodes carry, rebuilt as S3 records them against the
    /// selected domain packages (FSL FR-740), and its rewards are the ones
    /// its `reward` nodes carry, evaluated through their clause functions. It
    /// carries no claim of the model, and its delay tables are empty.
    ///
    /// # Errors
    ///
    /// [`PendingPackageForm::Form`] at the first node, in the package's node
    /// order, of a form the package route does not read yet;
    /// [`PendingPackageForm::World`] when the declarations are not readable
    /// from the nodes and identity rows;
    /// [`PendingPackageForm::Model`] when the supplied packages do not re-admit
    /// against the package's types; [`PendingPackageForm::Operations`] when
    /// their operations are not typed. A type that names a unit is typed
    /// against no unit names: the package route reads none yet, so such an
    /// operation refuses rather than being left out.
    pub fn of(
        package: &'p BarePackage<'p>,
        packages: &fsl_semantics::model::intake::PackageInput,
        normalization: fsl_semantics::model::accounting::ModelNormalizationLimits,
    ) -> Result<Self, PendingPackageForm> {
        Self::of_trace_input(package, packages, normalization, false)
    }

    /// Build the model world for supplied timed observations, without simulating time.
    /// Timed roots remain read through their admitted model-time joins by trace authentication.
    ///
    /// # Errors
    ///
    /// Refuses the same unsupported forms as [`Self::of`], except timed model roots.
    pub fn of_timed_trace(
        package: &'p BarePackage<'p>,
        packages: &fsl_semantics::model::intake::PackageInput,
        normalization: fsl_semantics::model::accounting::ModelNormalizationLimits,
    ) -> Result<Self, PendingPackageForm> {
        Self::of_trace_input(package, packages, normalization, true)
    }

    fn of_trace_input(
        package: &'p BarePackage<'p>,
        packages: &fsl_semantics::model::intake::PackageInput,
        normalization: fsl_semantics::model::accounting::ModelNormalizationLimits,
        timed_trace: bool,
    ) -> Result<Self, PendingPackageForm> {
        let unread = if timed_trace {
            package.first_unread_timed_trace_form()
        } else {
            package.first_unread_model_form()
        };
        if let Some((form, node)) = unread {
            return Err(PendingPackageForm::Form {
                form,
                node: node.clone(),
            });
        }
        let ProbabilisticParts {
            world,
            model,
            model_selections,
            operations,
            probabilistic,
        } = ProbabilisticParts::read(package, packages, normalization, false)?;
        let rewards = PackageRewards::of(package, world.clone(), &probabilistic)?;
        // FR-940: the state clauses by their `claim_names` rows (FSpec
        // FR-322), in source order, as the source route lists them.
        let state_clauses = package
            .state_clause_declarations()
            .map_err(PendingPackageForm::World)?
            .into_iter()
            .map(|declaration| StateClauseEntry {
                name: declaration.name,
                identity: declaration.identity,
                claim: declaration.claim,
                context: declaration.context,
                operation: declaration
                    .operation
                    .map(|operation| (operation.declaring, operation.operation)),
                observation: declaration.observation,
                invariant: declaration.invariant,
            })
            .collect();
        // FR-411, FR-940: the `terminal` declarations by their admitted rows.
        let terminals = package
            .terminal_declarations()
            .map_err(PendingPackageForm::World)?;
        Ok(Self {
            package,
            world,
            model,
            model_selections,
            operations,
            probabilistic,
            rewards,
            state_clauses,
            terminals,
        })
    }

    /// FSL FR-740 "Source objects": the probabilistic model of `package` when
    /// its random parameters and workloads sit on source object types, which
    /// [`Self::of`] refuses. The source packages are rebuilt from the
    /// package's source-object rows ([`BarePackage::source_model`]), one per
    /// declaring unit, and the `probabilistic` nodes join to them as S3's
    /// records do. The rebuilt packages hold the types, supertypes,
    /// operations and parameters of those objects, and no field, population
    /// or effect frame: they serve this join only, so no model-check input
    /// is built from them.
    ///
    /// # Errors
    ///
    /// [`PendingPackageForm::Form`] at the first node of a form the package
    /// route does not read yet (a source object is not one);
    /// [`PendingPackageForm::SourceModel`], at the row's node, when the rows
    /// do not describe source packages or a `probabilistic` node names an
    /// operation one of whose parameter types the rows do not let a reader
    /// rebuild; otherwise as [`Self::of`].
    pub fn source_object_probabilistic_model(
        package: &BarePackage<'_>,
        packages: &fsl_semantics::model::intake::PackageInput,
        normalization: fsl_semantics::model::accounting::ModelNormalizationLimits,
    ) -> Result<CheckedProbabilisticModel, PendingPackageForm> {
        if let Some((form, node)) = package.first_unread_node_form() {
            return Err(PendingPackageForm::Form {
                form,
                node: node.clone(),
            });
        }
        Ok(ProbabilisticParts::read(package, packages, normalization, true)?.probabilistic)
    }

    /// FR-940: the package form of the temporal clause declared at the
    /// `claim` occurrence `claim`, the model checker's clause on the package route: read from
    /// the package's nodes ([`BarePackage::temporal_clause_form`]) with the
    /// population domains of the supplied domain packages, and rebuilt as S3
    /// builds it ([`CheckedTemporalClause::of_package_form`]) within S3's
    /// domain `position_limit`.
    ///
    /// # Errors
    ///
    /// [`PendingClauseForm`] for a clause the package does not select or
    /// whose package form reads a fact the form does not read yet. A clause
    /// under a profile other than the infinite-trace one, and a clause with
    /// captures, whose package-route letters do not read them yet, are
    /// refused [`fsl_package::BareClauseRefusal::Pending`] at the clause node
    /// (`profile`, `capture`) before any engine runs.
    pub fn temporal_clause(
        &self,
        claim: &fsl_foundation::source::provenance::OccurrenceKey,
        position_limit: u64,
    ) -> Result<CheckedTemporalClause, PendingClauseForm> {
        let clause = claim.node();
        let mut form = self
            .package
            .temporal_clause_form(claim)
            .map_err(PendingClauseForm::Read)?;
        let pending = |fact| -> Result<CheckedTemporalClause, PendingClauseForm> {
            let node = self
                .package
                .temporal_clause_node(clause)
                .map_err(PendingClauseForm::Read)?;
            Err(PendingClauseForm::Read(
                fsl_package::BareClauseRefusal::Pending {
                    fact,
                    node: node.clone(),
                },
            ))
        };
        if form.profile != fsl_semantics::check::TemporalProfile::InfiniteTrace {
            return pending("profile");
        }
        if !form.captures.is_empty() {
            return pending("capture");
        }
        if let ValueType::Reference(object) = &form.over.1 {
            form.populations = self.model.overlapping_populations(object.clone());
        }
        CheckedTemporalClause::of_package_form(form, self.world.types(), position_limit)
            .map_err(PendingClauseForm::Build)
    }

    /// [`Self::temporal_clause`] for a run that reads the clause over a trace
    /// or a model's behaviours rather than checking it: every profile
    /// and the clause's step intervals are read, since the evaluator reads
    /// each from the clause itself. A clause with captures is still refused,
    /// as the package-route letters do not read them yet.
    ///
    /// # Errors
    ///
    /// [`PendingClauseForm`] as [`Self::temporal_clause`], less the profile and
    /// interval facts.
    pub fn temporal_run_clause(
        &self,
        claim: &fsl_foundation::source::provenance::OccurrenceKey,
        position_limit: u64,
    ) -> Result<CheckedTemporalClause, PendingClauseForm> {
        let clause = claim.node();
        let mut form = self
            .package
            .temporal_clause_run_form(claim)
            .map_err(PendingClauseForm::Read)?;
        if !form.captures.is_empty() {
            let node = self
                .package
                .temporal_clause_node(clause)
                .map_err(PendingClauseForm::Read)?;
            return Err(PendingClauseForm::Read(
                fsl_package::BareClauseRefusal::Pending {
                    fact: "capture",
                    node: node.clone(),
                },
            ));
        }
        if let ValueType::Reference(object) = &form.over.1 {
            form.populations = self.model.overlapping_populations(object.clone());
        }
        CheckedTemporalClause::of_package_run_form(form, self.world.types(), position_limit)
            .map_err(PendingClauseForm::Build)
    }

    /// FR-940 "Package form of a state-graph claim": the state-graph claim
    /// declared at the `claim` occurrence `claim`, read from the package's
    /// nodes ([`BarePackage::state_graph_claim_form`]) and rebuilt as S3
    /// builds it ([`CheckedStateGraphClaim::of_package_form`]) within S3's
    /// domain `position_limit`. Its predicates are evaluated through the
    /// evaluator port ([`ModelEvaluator::evaluate_state_graph_predicate`]).
    ///
    /// # Errors
    ///
    /// [`PendingClauseForm`] for a claim the package does not select or
    /// whose facts do not rebuild it.
    pub fn state_graph_claim(
        &self,
        claim: &fsl_foundation::source::provenance::OccurrenceKey,
        position_limit: u64,
    ) -> Result<fsl_semantics::check::CheckedStateGraphClaim, PendingClauseForm> {
        let form = self
            .package
            .state_graph_claim_form(claim)
            .map_err(PendingClauseForm::Read)?;
        fsl_semantics::check::CheckedStateGraphClaim::of_package_form(
            form,
            self.world.types(),
            position_limit,
        )
        .map_err(PendingClauseForm::Build)
    }

    /// The state clause declared at the `claim` occurrence `claim` of the
    /// node `clause`, as its `claim_names` row names it (FR-940).
    pub fn state_clause(
        &self,
        clause: &NodeKey,
        claim: &filament_exact::Origin,
    ) -> Option<&StateClauseEntry> {
        self.state_clauses
            .iter()
            .find(|entry| entry.identity == *clause && entry.claim == *claim)
    }

    /// Every state clause declaration, in source order (FR-940).
    pub fn state_clauses(&self) -> &[StateClauseEntry] {
        &self.state_clauses
    }

    /// The state clause whose node key is `clause`, declared at its own
    /// `claim_names` row (FR-940).
    pub fn state_clause_at(&self, clause: &NodeKey) -> Option<&StateClauseEntry> {
        self.state_clauses
            .iter()
            .find(|entry| entry.identity == *clause)
    }

    /// FR-104 "Requirements", FR-940: the domain kinds of the
    /// `operation-contract` record of the `invariant` state clause `entry`,
    /// as S3 classifies them: the domains of its `self` parameter's type
    /// within `position_limit`, then its context's population. `None` when
    /// no single population of the selected domain packages covers the
    /// context, so the record's other populations (those its body walks or
    /// names) would decide the extent, and for a type that does not
    /// classify.
    pub fn invariant_domain_kinds(
        &self,
        entry: &StateClauseEntry,
        position_limit: u64,
    ) -> Option<Vec<fsl_foundation::bound::DomainKind>> {
        let receiver = ValueType::Reference(entry.context.clone());
        let mut kinds: Vec<fsl_foundation::bound::DomainKind> =
            match fsl_semantics::family::classify_extent(
                &[(entry.identity.clone(), &receiver)],
                self.world.types(),
                position_limit,
            )
            .ok()?
            {
                fsl_semantics::family::ClaimExtent::Bounded => Vec::new(),
                fsl_semantics::family::ClaimExtent::Unbounded(domains) => {
                    domains.iter().map(|(_, kind)| kind).collect()
                }
            };
        // A domain package population declares no maximum, and the
        // package route reads no authored bound, as for a temporal clause's
        // populations ([`StateModel::overlapping_populations`]).
        self.model
            .covering_population(entry.context.clone())
            .ok()??;
        kinds.push(fsl_foundation::bound::DomainKind::Population);
        Some(kinds)
    }

    /// The package this input was built from.
    pub fn package(&self) -> &'p BarePackage<'p> {
        self.package
    }

    /// The input this package lends.
    pub fn input(&self) -> ModelCheckInput<'_> {
        ModelCheckInput {
            types: self.world.types(),
            variant_types: &[],
            enums: &[],
            operations: &self.operations,
            model_selections: &self.model_selections,
            source_packages: &[],
            timed_models: &[],
            time_sources: &[],
            probabilistic: &self.probabilistic,
            state_clauses: self.state_clauses.clone(),
            terminals: Ok(self
                .terminals
                .iter()
                .map(|terminal| ModelTerminal {
                    model: super::TerminalModel::new(
                        terminal.model.clone(),
                        terminal.model.clone(),
                    )
                    .with_ordinal(terminal.ordinal),
                    node: terminal.node.clone(),
                    declaration: match &terminal.declaration {
                        fsl_package::BareTerminalDeclaration::Any => ModelTerminalDeclaration::Any,
                        fsl_package::BareTerminalDeclaration::When(predicate) => {
                            ModelTerminalDeclaration::When(TerminalWhen::Package(predicate))
                        }
                    },
                })
                .collect()),
            read_rewards: Some(&self.rewards),
            evaluator: self.package,
            source: None,
            admitted: Some(self.package.package()),
        }
    }
}

mod sealed {
    /// Private to this module: no other type implements the evaluator.
    pub trait Sealed {}
    impl Sealed for fsl_package::CheckedPackage {}
    impl Sealed for fsl_package::BarePackage<'_> {}
}

/// The fault a source checked form handed to the package route's evaluator
/// raises: a package-route input holds no source form, so none reaches it.
const SOURCE_FORM: InternalFault = InternalFault::new("FR-940", "source-form-on-package-route");

/// The fault a package-route form handed to the source package's evaluator
/// raises: a source-route input holds no package form, so none reaches it.
const PACKAGE_FORM: InternalFault = InternalFault::new("FR-940", "package-form-on-source-route");

/// The evaluator of a package's checked forms a model check reads through
/// its [`ModelCheckInput`] (FR-531, FR-940).
///
/// The trait is sealed: the source package and the bare package are its only
/// implementors. A method that takes a source checked form (a body, a row, a
/// predicate) is reached on the source route only, because the package
/// route's input holds no such form; the package route answers it with an
/// internal fault. A method keyed by a name or node evaluates on either
/// route, the package route through [`CheckedPackageEvaluation`] over the
/// package's forms.
pub trait ModelEvaluator: sealed::Sealed + Sync {
    /// FR-107: one state clause, by name, over its admitted observations.
    ///
    /// # Errors
    ///
    /// As [`CheckedPackageEvaluation::evaluate_clause`].
    fn evaluate_clause(
        &self,
        clause: &QualifiedName,
        observations: &AdmittedObservations,
        meter: &mut Meter,
    ) -> Result<ClauseEvaluation, CallFailure>;

    /// FR-107 "Clause sets": the state clauses of one invocation, object or
    /// state, each with a fresh meter of `limits`, polling `cancel`.
    ///
    /// # Errors
    ///
    /// As [`CheckedPackageEvaluation::evaluate_clause_set`].
    fn evaluate_clause_set(
        &self,
        clauses: &[(QualifiedName, &AdmittedObservations)],
        limits: ScalarLimits,
        cancel: Option<&filament_exact::Cancel>,
    ) -> Result<ClauseSetEvaluation, CallFailure>;

    /// FR-940: the `holds` atom `atom` of a temporal clause, keyed by the
    /// clause node and the atom's formula position, over `objects`. Both
    /// routes evaluate it: the source route from its checked graph, the
    /// package route through the atom's package form.
    ///
    /// # Errors
    ///
    /// As [`CheckedPackageEvaluation::evaluate_holds`].
    fn evaluate_holds(
        &self,
        atom: &crate::value::HoldsAtom,
        over: Value,
        captures: &[Value],
        objects: &ObjectEnvironment,
        meter: &mut Meter,
    ) -> Result<Evaluation, CallFailure>;

    /// Evaluate a model behaviour's holds condition with FR-107 model-read accounting.
    fn evaluate_model_holds(
        &self,
        atom: &crate::value::HoldsAtom,
        over: Value,
        captures: &[Value],
        objects: &ObjectEnvironment,
        reference_bindings: &crate::simulation::model::ReferenceBindings,
        meter: &mut Meter,
    ) -> Result<Evaluation, CallFailure>;

    /// FR-940: the `when` guard of the temporal clause `clause`, keyed by
    /// its node, over `objects`, on either route.
    ///
    /// # Errors
    ///
    /// As [`CheckedPackageEvaluation::evaluate_guard`].
    fn evaluate_guard(
        &self,
        clause: &NodeKey,
        over: Value,
        objects: &ObjectEnvironment,
        meter: &mut Meter,
    ) -> Result<Evaluation, CallFailure>;

    /// Evaluate a model behaviour's activation guard with FR-107 model-read accounting.
    fn evaluate_model_guard(
        &self,
        clause: &NodeKey,
        over: Value,
        objects: &ObjectEnvironment,
        reference_bindings: &crate::simulation::model::ReferenceBindings,
        meter: &mut Meter,
    ) -> Result<Evaluation, CallFailure>;

    /// Reference literals reachable from the clause's projected conditions and guard.
    /// An unread condition retains its evaluator refusal when evaluated.
    fn temporal_reference_literals(
        &self,
        clause: &CheckedTemporalClause,
        key: &NodeKey,
    ) -> Vec<(EffectiveId, Vec<u8>)>;

    /// FR-940: `invocation` against the operation frame `frame`, keyed by
    /// its node, on either route. The package route reads the frame's
    /// declared effect from `operations`, the input's operation table.
    ///
    /// # Errors
    ///
    /// As [`CheckedPackageEvaluation::evaluate_frame`].
    fn evaluate_frame(
        &self,
        frame: &NodeKey,
        invocation: &fsl_semantics::model::observation::AdmittedInvocation<'_>,
        operations: &OperationTable,
        meter: &mut Meter,
    ) -> Result<crate::value::FrameEvaluation, CallFailure>;

    /// FR-230 "Collection-valued writes".
    ///
    /// # Errors
    ///
    /// As [`CheckedPackageEvaluation::evaluate_collection_write`].
    fn evaluate_collection_write(
        &self,
        clause: &QualifiedName,
        observations: &AdmittedObservations,
        field: &str,
        written: &Written,
        meter: &mut Meter,
    ) -> Result<Option<WrittenCollection>, CallFailure>;

    /// FR-120 "Directly solved assignments", FR-851 "Eligible conjuncts":
    /// the equation and bound conjuncts of the postcondition `clause` over
    /// `observations`, each conjunct's `e` evaluated once with a fresh meter
    /// from `meter`. Both routes read the clause's checked form.
    ///
    /// # Errors
    ///
    /// As [`CheckedPackageEvaluation::evaluate_clause`].
    fn evaluate_post_targets(
        &self,
        clause: &QualifiedName,
        observations: &AdmittedObservations,
        meter: &mut dyn FnMut() -> Meter,
    ) -> Result<PostTargets, CallFailure>;

    /// FR-487 "Evaluating an atom".
    ///
    /// # Errors
    ///
    /// As [`CheckedPackageEvaluation::evaluate_hyper_holds`].
    fn evaluate_hyper_holds(
        &self,
        clause: &CheckedHyperClause,
        atom: FormulaNodeId,
        slots: Vec<crate::hyper::HyperSlot>,
        components: &[&ObjectEnvironment],
        meter: &mut Meter,
    ) -> Result<Evaluation, CallFailure>;

    /// [`crate::value::evaluate_state_predicate_with`].
    ///
    /// # Errors
    ///
    /// An internal fault, as the evaluator reports it.
    fn evaluate_state_predicate(
        &self,
        predicate: &dyn StatePredicate,
        current: &ObjectEnvironment,
        over: Option<ObjectReference>,
        further: Vec<Value>,
        meter: &mut Meter,
    ) -> Result<Evaluation, InternalFault>;

    /// [`crate::value::evaluate_state_predicate_at_absent_key`].
    ///
    /// # Errors
    ///
    /// An internal fault, as the evaluator reports it.
    fn evaluate_state_predicate_at_absent_key(
        &self,
        predicate: &dyn StatePredicate,
        current: &ObjectEnvironment,
        key: ObjectReference,
        binding: String,
        meter: &mut Meter,
    ) -> Result<Evaluation, InternalFault>;

    /// FR-940 "Package form of a state-graph claim": the predicate of
    /// `role` of the state-graph claim declared at `claim`, with its `over`
    /// parameter bound to `over` (`None` for a claim with no `over`) and
    /// the slots after it to `further`, at the state whose objects are
    /// `current`. Both routes evaluate it: the source route its checked
    /// body, the package route the predicate node's reader projection, so a
    /// predicate has one value on both.
    ///
    /// # Errors
    ///
    /// An internal fault when the claim has no written predicate of `role`.
    fn evaluate_state_graph_predicate(
        &self,
        claim: &fsl_foundation::source::provenance::OccurrenceKey,
        role: fsl_semantics::check::PredicateRole,
        current: &ObjectEnvironment,
        over: Option<ObjectReference>,
        further: Vec<Value>,
        meter: &mut Meter,
    ) -> Result<Evaluation, InternalFault>;

    /// FR-156, FR-940 "Package form of a state-graph claim": the symbolic
    /// locations the predicate of `role` of the claim at `claim` reads, as the
    /// partial-order reduction takes them; none for the constant `true`.
    ///
    /// # Errors
    ///
    /// [`FootprintRefusal`] as the derivation refuses a form with no
    /// footprint rule.
    fn state_graph_reads(
        &self,
        model: &StateModel,
        claim: &fsl_foundation::source::provenance::OccurrenceKey,
        role: fsl_semantics::check::PredicateRole,
    ) -> Result<BodyReads, FootprintRefusal>;

    /// [`Self::evaluate_state_graph_predicate`] for an instance whose key
    /// holds no object conforming to the `over` parameter's type, as
    /// [`crate::value::evaluate_state_predicate_at_absent_key`] reads it.
    ///
    /// # Errors
    ///
    /// As [`Self::evaluate_state_graph_predicate`].
    fn evaluate_state_graph_predicate_at_absent_key(
        &self,
        claim: &fsl_foundation::source::provenance::OccurrenceKey,
        role: fsl_semantics::check::PredicateRole,
        current: &ObjectEnvironment,
        key: ObjectReference,
        binding: String,
        meter: &mut Meter,
    ) -> Result<Evaluation, InternalFault>;

    /// [`crate::value::evaluate_activation_guard`].
    ///
    /// # Errors
    ///
    /// An internal fault, as the evaluator reports it.
    fn evaluate_activation_guard(
        &self,
        guard: &dyn StatePredicate,
        current: &ObjectEnvironment,
        bound: ObjectReference,
        meter: &mut Meter,
    ) -> Result<Evaluation, InternalFault>;

    /// [`crate::value::evaluate_activation_guard_at_absent_key`].
    ///
    /// # Errors
    ///
    /// An internal fault, as the evaluator reports it.
    fn evaluate_activation_guard_at_absent_key(
        &self,
        guard: &dyn StatePredicate,
        current: &ObjectEnvironment,
        key: ObjectReference,
        binding: String,
        meter: &mut Meter,
    ) -> Result<Evaluation, InternalFault>;

    /// [`crate::value::evaluate_capture`].
    ///
    /// # Errors
    ///
    /// An internal fault, as the evaluator reports it.
    fn evaluate_capture(
        &self,
        capture: &dyn StatePredicate,
        current: &ObjectEnvironment,
        bindings: Vec<Value>,
        absent: Option<(Vec<usize>, String)>,
        meter: &mut Meter,
    ) -> Result<Evaluation, InternalFault>;

    /// [`crate::value::evaluate_terminal_predicate`].
    ///
    /// # Errors
    ///
    /// An internal fault, as the evaluator reports it.
    fn evaluate_terminal_predicate(
        &self,
        predicate: TerminalWhen<'_>,
        current: &ObjectEnvironment,
        self_object: Option<ObjectReference>,
        meter: &mut Meter,
    ) -> Result<Evaluation, InternalFault>;

    /// [`crate::value::evaluate_refinement_row`].
    ///
    /// # Errors
    ///
    /// An internal fault, as the evaluator reports it.
    fn evaluate_refinement_row(
        &self,
        row: &CheckedRow,
        current: &ObjectEnvironment,
        bindings: Vec<Value>,
        meter: &mut Meter,
    ) -> Result<Evaluation, InternalFault>;

    /// [`crate::value::evaluate_history_update`].
    ///
    /// # Errors
    ///
    /// An internal fault, as the evaluator reports it.
    fn evaluate_history_update(
        &self,
        row: &CheckedRow,
        states: (&ObjectEnvironment, &ObjectEnvironment),
        bindings: Vec<Value>,
        meter: &mut Meter,
    ) -> Result<Evaluation, InternalFault>;

    /// [`crate::value::evaluate_protocol_slot`].
    ///
    /// # Errors
    ///
    /// An internal fault, as the evaluator reports it.
    fn evaluate_protocol_slot(
        &self,
        body: &CheckedBody,
        frame: Vec<Option<Value>>,
        current: &ObjectEnvironment,
        meter: &mut Meter,
    ) -> Result<Evaluation, InternalFault>;

    /// FR-940: evaluate a package protocol slot from its admitted node
    /// descriptor and supplied package binders. This is separate from the
    /// source `CheckedBody` path so package execution never synthesizes one.
    fn evaluate_package_protocol_slot(
        &self,
        _slot: &ProtocolSlot,
        _supply: &ProtocolSupply<'_>,
        _meter: &mut Meter,
    ) -> Result<ProtocolSlotEvaluation, CallFailure> {
        Err(CallFailure::Fault(SOURCE_FORM))
    }

    /// Admit a simulated state as this package evaluator's current protocol
    /// observation, retaining the bare package's admission authority.
    fn package_protocol_observation(
        &self,
        _model: &DomainPackageRef,
        _packages: &fsl_semantics::model::intake::PackageInput,
        _limits: fsl_semantics::model::accounting::ModelNormalizationLimits,
        _state: &fsl_semantics::model::state::ModelState,
        _identity: fsl_semantics::model::observation::DocumentRef,
    ) -> Result<
        filament_semantic_value::admission::ObservationAdmission<
            fsl_package::AdmittedProtocolObservation<'static>,
        >,
        InternalFault,
    > {
        Err(SOURCE_FORM)
    }

    /// [`crate::value::evaluate_join_predicate`].
    ///
    /// # Errors
    ///
    /// An internal fault, as the evaluator reports it.
    fn evaluate_join_predicate(
        &self,
        function: &NodeKey,
        arguments: Vec<Value>,
        meter: &mut Meter,
    ) -> Result<Evaluation, InternalFault>;

    /// [`crate::value::evaluate_reward`].
    ///
    /// # Errors
    ///
    /// An internal fault, as the evaluator reports it.
    fn evaluate_reward(
        &self,
        reward: &Reward,
        arguments: Vec<Value>,
        meter: &mut Meter,
    ) -> Result<Evaluation, InternalFault>;

    /// The checked state clause `name`, for a reading of its body.
    fn state_clause(&self, name: &str) -> Option<&CheckedStateClause>;

    /// The checked protocol `name`.
    fn protocol(&self, name: &str) -> Option<&CheckedProtocol>;

    /// FR-940: the package-native protocol structure read through the
    /// admitted package view. This is available on the bytes route without
    /// rebuilding a source `CheckedProtocol`.
    fn package_protocol(
        &self,
        _name: &str,
    ) -> Option<fsl_package::protocol_view::ProtocolExecutionView<'_>> {
        None
    }

    /// FR-940: form a package protocol-slot descriptor from its admitted
    /// structural identity. `None` means this evaluator has no package-side
    /// identity row for `node`.
    fn package_protocol_slot(
        &self,
        _node: &CheckedNodeId,
        _member: &str,
        _index: Option<usize>,
    ) -> Option<ProtocolSlot> {
        None
    }

    /// FR-390: the exact binder identities read by an admitted package
    /// protocol slot, in its evaluator-defined order.
    fn package_protocol_slot_binders(&self, _slot: &ProtocolSlot) -> Option<Vec<NodeKey>> {
        None
    }

    /// FR-531: the exact identity row for one admitted package node.
    fn package_node_key(&self, _node: &CheckedNodeId) -> Option<NodeKey> {
        None
    }

    /// FR-531: the declaring type and operation selected by a package
    /// protocol attempt's admitted operation anchor.
    fn package_protocol_operation(&self, _anchor: &CheckedNodeId) -> Option<(EffectiveId, String)> {
        None
    }

    /// FR-531: the exact object type selected by a package protocol role.
    fn package_protocol_role_model(&self, _model: &CheckedNodeId) -> Option<EffectiveId> {
        None
    }

    /// FR-531: the admitted value type of a package protocol binder.
    fn package_protocol_parameter_type(&self, _binder: &CheckedNodeId) -> Option<ValueType> {
        None
    }

    /// The admitted node keys of the protocol `name`.
    fn protocol_node_keys(&self, name: &str) -> Option<&CheckedProtocolNodeKeys>;

    /// FSpec FR-331: the ordinal of the own field `name` of `owner` among
    /// its own fields in name order.
    fn own_field_ordinal(&self, owner: &EffectiveId, name: &str) -> Option<usize>;

    /// FSpec FR-383: the footprint of `operation`, reading only the clauses
    /// `select` admits.
    ///
    /// # Errors
    ///
    /// [`FootprintRefusal`] as the derivation reports it.
    #[allow(
        clippy::too_many_arguments,
        reason = "the footprint derivation takes the operation, its receiver identities, arguments, literals and selector"
    )]
    fn selected_footprint(
        &self,
        model: &StateModel,
        receiver_type: EffectiveId,
        declaring: EffectiveId,
        operation: &OperationDeclaration,
        arguments: &[(String, EffectiveId)],
        reference_literals: &ReferenceLiterals,
        select: &dyn Fn(NodeKey) -> bool,
    ) -> Result<SymbolicFootprint, FootprintRefusal>;

    /// ADR-027 FT-1: [`Self::selected_footprint`] for an attempt whose
    /// clauses hold a form with no footprint rule.
    ///
    /// # Errors
    ///
    /// [`FootprintRefusal`] as the derivation reports it.
    #[allow(
        clippy::too_many_arguments,
        reason = "the footprint derivation takes the operation, its receiver identities, arguments, literals and selector"
    )]
    fn attempt_footprint_fallback(
        &self,
        model: &StateModel,
        receiver_type: EffectiveId,
        declaring: EffectiveId,
        operation: &OperationDeclaration,
        arguments: &[(String, EffectiveId)],
        reference_literals: &ReferenceLiterals,
        select: &dyn Fn(NodeKey) -> bool,
    ) -> Result<SymbolicFootprint, FootprintRefusal>;

    /// FSpec FR-383: the footprint of `operation` over every clause.
    ///
    /// # Errors
    ///
    /// [`FootprintRefusal`] as the derivation reports it.
    fn footprint(
        &self,
        model: &StateModel,
        receiver_type: EffectiveId,
        declaring: EffectiveId,
        operation: &OperationDeclaration,
        arguments: &[(String, EffectiveId)],
    ) -> Result<SymbolicFootprint, FootprintRefusal>;

    /// FR-156 "Visibility": the reads of the state constraint `name`, `None`
    /// for a name the package declares no state clause under.
    ///
    /// # Errors
    ///
    /// [`FootprintRefusal`] as the derivation reports it.
    fn constraint_reads(
        &self,
        model: &StateModel,
        name: &str,
    ) -> Option<Result<BodyReads, FootprintRefusal>>;

    /// FR-156 "Atom footprints" over the package route: the reads of the
    /// `holds` atom at use `position` of the temporal clause `clause`,
    /// read from the package's own nodes. The source route reads its atoms
    /// from checked bodies and refuses this.
    ///
    /// # Errors
    ///
    /// [`FootprintRefusal`] as the derivation reports it.
    fn holds_reads(
        &self,
        model: &StateModel,
        clause: &NodeKey,
        position: usize,
    ) -> Result<BodyReads, FootprintRefusal>;

    /// FR-337 over the package route: the reads of the `when` guard of the
    /// `on each` activation of the temporal clause `clause`, over `binders`,
    /// `None` for a clause with no guard.
    ///
    /// # Errors
    ///
    /// [`FootprintRefusal`] as the derivation reports it.
    fn activation_guard_reads(
        &self,
        model: &StateModel,
        clause: &NodeKey,
    ) -> Result<Option<BodyReads>, FootprintRefusal>;

    /// ADR-027 FT-3: the reads of a protocol expression.
    ///
    /// # Errors
    ///
    /// [`FootprintRefusal`] as the derivation reports it.
    fn slot_reads(
        &self,
        model: &StateModel,
        name: &str,
        parameters: &[(String, ValueType)],
        body: CheckedNode<'_>,
    ) -> Result<BTreeSet<SymbolicLocation>, FootprintRefusal>;

    /// ADR-025 MA-1: the reads of a protocol node's constraint.
    ///
    /// # Errors
    ///
    /// [`FootprintRefusal`] as the derivation reports it.
    fn node_reads(
        &self,
        model: &StateModel,
        name: &str,
        parameters: &[(String, ValueType)],
        body: CheckedNode<'_>,
    ) -> Result<BTreeSet<SymbolicLocation>, FootprintRefusal>;

    /// FR-156 "Atom footprints": the reads of a `holds` atom's condition.
    ///
    /// # Errors
    ///
    /// [`FootprintRefusal`] as the derivation reports it.
    fn atom_reads(
        &self,
        model: &StateModel,
        over: &(String, ValueType),
        condition: CheckedNode<'_>,
    ) -> Result<BTreeSet<SymbolicLocation>, FootprintRefusal>;

    /// FR-337: the reads of an activation guard or capture.
    ///
    /// # Errors
    ///
    /// [`FootprintRefusal`] as the derivation reports it.
    fn guard_reads(
        &self,
        model: &StateModel,
        binders: &[(String, ValueType)],
        condition: CheckedNode<'_>,
    ) -> Result<BTreeSet<SymbolicLocation>, FootprintRefusal>;
}

impl ModelEvaluator for CheckedPackage {
    fn evaluate_clause(
        &self,
        clause: &QualifiedName,
        observations: &AdmittedObservations,
        meter: &mut Meter,
    ) -> Result<ClauseEvaluation, CallFailure> {
        CheckedPackageEvaluation::evaluate_clause(self, clause, observations, meter)
    }

    fn evaluate_clause_set(
        &self,
        clauses: &[(QualifiedName, &AdmittedObservations)],
        limits: ScalarLimits,
        cancel: Option<&filament_exact::Cancel>,
    ) -> Result<ClauseSetEvaluation, CallFailure> {
        crate::value::evaluate_clause_set_with_cancel(self, clauses, limits, cancel)
    }

    fn evaluate_holds(
        &self,
        atom: &crate::value::HoldsAtom,
        over: Value,
        captures: &[Value],
        objects: &ObjectEnvironment,
        meter: &mut Meter,
    ) -> Result<Evaluation, CallFailure> {
        CheckedPackageEvaluation::evaluate_holds(self, atom, over, captures, objects, meter)
    }

    fn evaluate_model_holds(
        &self,
        atom: &crate::value::HoldsAtom,
        over: Value,
        captures: &[Value],
        objects: &ObjectEnvironment,
        reference_bindings: &crate::simulation::model::ReferenceBindings,
        meter: &mut Meter,
    ) -> Result<Evaluation, CallFailure> {
        CheckedPackageEvaluation::evaluate_model_holds(
            self,
            atom,
            over,
            captures,
            objects,
            reference_bindings,
            meter,
        )
    }

    fn evaluate_guard(
        &self,
        clause: &NodeKey,
        over: Value,
        objects: &ObjectEnvironment,
        meter: &mut Meter,
    ) -> Result<Evaluation, CallFailure> {
        CheckedPackageEvaluation::evaluate_guard(self, clause, over, objects, meter)
    }

    fn evaluate_model_guard(
        &self,
        clause: &NodeKey,
        over: Value,
        objects: &ObjectEnvironment,
        reference_bindings: &crate::simulation::model::ReferenceBindings,
        meter: &mut Meter,
    ) -> Result<Evaluation, CallFailure> {
        CheckedPackageEvaluation::evaluate_model_guard(
            self,
            clause,
            over,
            objects,
            reference_bindings,
            meter,
        )
    }

    fn temporal_reference_literals(
        &self,
        clause: &CheckedTemporalClause,
        key: &NodeKey,
    ) -> Vec<(EffectiveId, Vec<u8>)> {
        let mut literals = BTreeSet::new();
        if let Some(node) = self
            .graph()
            .temporal_clauses()
            .iter()
            .find(|node| node.identity() == *key)
        {
            for (index, formula) in clause.formula().nodes().iter().enumerate() {
                if matches!(formula, fsl_semantics::check::FormulaNode::Holds { .. }) {
                    if let Some((condition, _)) =
                        node.condition(FormulaNodeId::from_arena_index(index))
                    {
                        super::reference_bindings::collect_literals(
                            self.graph(),
                            condition,
                            &mut literals,
                        );
                    }
                }
            }
            if let Some((guard, _)) = node.guard() {
                super::reference_bindings::collect_literals(self.graph(), guard, &mut literals);
            }
        }
        literals
            .into_iter()
            .map(|literal| (literal.object_type, literal.identity))
            .collect()
    }

    fn evaluate_frame(
        &self,
        frame: &NodeKey,
        invocation: &fsl_semantics::model::observation::AdmittedInvocation<'_>,
        _operations: &OperationTable,
        meter: &mut Meter,
    ) -> Result<crate::value::FrameEvaluation, CallFailure> {
        // The source route reads the declared effect from its checked frame.
        CheckedPackageEvaluation::evaluate_frame(self, frame.clone(), invocation, meter)
    }

    fn evaluate_collection_write(
        &self,
        clause: &QualifiedName,
        observations: &AdmittedObservations,
        field: &str,
        written: &Written,
        meter: &mut Meter,
    ) -> Result<Option<WrittenCollection>, CallFailure> {
        CheckedPackageEvaluation::evaluate_collection_write(
            self,
            clause,
            observations,
            field,
            written,
            meter,
        )
    }

    fn evaluate_post_targets(
        &self,
        clause: &QualifiedName,
        observations: &AdmittedObservations,
        meter: &mut dyn FnMut() -> Meter,
    ) -> Result<PostTargets, CallFailure> {
        crate::value::source_post_targets(self, clause, observations, meter)
    }

    fn evaluate_hyper_holds(
        &self,
        clause: &CheckedHyperClause,
        atom: FormulaNodeId,
        slots: Vec<crate::hyper::HyperSlot>,
        components: &[&ObjectEnvironment],
        meter: &mut Meter,
    ) -> Result<Evaluation, CallFailure> {
        CheckedPackageEvaluation::evaluate_hyper_holds(self, clause, atom, slots, components, meter)
    }

    fn evaluate_state_predicate(
        &self,
        predicate: &dyn StatePredicate,
        current: &ObjectEnvironment,
        over: Option<ObjectReference>,
        further: Vec<Value>,
        meter: &mut Meter,
    ) -> Result<Evaluation, InternalFault> {
        crate::value::evaluate_state_predicate_with(self, predicate, current, over, further, meter)
    }

    fn evaluate_state_predicate_at_absent_key(
        &self,
        predicate: &dyn StatePredicate,
        current: &ObjectEnvironment,
        key: ObjectReference,
        binding: String,
        meter: &mut Meter,
    ) -> Result<Evaluation, InternalFault> {
        crate::value::evaluate_state_predicate_at_absent_key(
            self, predicate, current, key, binding, meter,
        )
    }

    fn state_graph_reads(
        &self,
        model: &StateModel,
        claim: &fsl_foundation::source::provenance::OccurrenceKey,
        role: fsl_semantics::check::PredicateRole,
    ) -> Result<BodyReads, FootprintRefusal> {
        let body =
            source_predicate(self, claim, role).map_err(|_| FootprintRefusal::Unresolved {
                what: "state-graph predicate",
            })?;
        let reads = derive_atom_reads(
            self.graph(),
            model,
            &(String::new(), ValueType::Boolean),
            body.body(),
        )?;
        Ok(BodyReads {
            reads,
            literals: reference_literals_of(body.body()),
        })
    }

    fn evaluate_state_graph_predicate(
        &self,
        claim: &fsl_foundation::source::provenance::OccurrenceKey,
        role: fsl_semantics::check::PredicateRole,
        current: &ObjectEnvironment,
        over: Option<ObjectReference>,
        further: Vec<Value>,
        meter: &mut Meter,
    ) -> Result<Evaluation, InternalFault> {
        let body = source_predicate(self, claim, role)?;
        crate::value::evaluate_state_predicate_with(self, body, current, over, further, meter)
    }

    fn evaluate_state_graph_predicate_at_absent_key(
        &self,
        claim: &fsl_foundation::source::provenance::OccurrenceKey,
        role: fsl_semantics::check::PredicateRole,
        current: &ObjectEnvironment,
        key: ObjectReference,
        binding: String,
        meter: &mut Meter,
    ) -> Result<Evaluation, InternalFault> {
        let body = source_predicate(self, claim, role)?;
        crate::value::evaluate_state_predicate_at_absent_key(
            self, body, current, key, binding, meter,
        )
    }

    fn evaluate_activation_guard(
        &self,
        guard: &dyn StatePredicate,
        current: &ObjectEnvironment,
        bound: ObjectReference,
        meter: &mut Meter,
    ) -> Result<Evaluation, InternalFault> {
        crate::value::evaluate_activation_guard(self, guard, current, bound, meter)
    }

    fn evaluate_activation_guard_at_absent_key(
        &self,
        guard: &dyn StatePredicate,
        current: &ObjectEnvironment,
        key: ObjectReference,
        binding: String,
        meter: &mut Meter,
    ) -> Result<Evaluation, InternalFault> {
        crate::value::evaluate_activation_guard_at_absent_key(
            self, guard, current, key, binding, meter,
        )
    }

    fn evaluate_capture(
        &self,
        capture: &dyn StatePredicate,
        current: &ObjectEnvironment,
        bindings: Vec<Value>,
        absent: Option<(Vec<usize>, String)>,
        meter: &mut Meter,
    ) -> Result<Evaluation, InternalFault> {
        crate::value::evaluate_capture(self, capture, current, bindings, absent, meter)
    }

    fn evaluate_terminal_predicate(
        &self,
        predicate: TerminalWhen<'_>,
        current: &ObjectEnvironment,
        self_object: Option<ObjectReference>,
        meter: &mut Meter,
    ) -> Result<Evaluation, InternalFault> {
        match predicate {
            TerminalWhen::Source(predicate) => crate::value::evaluate_terminal_predicate(
                self,
                predicate,
                current,
                self_object,
                meter,
            ),
            TerminalWhen::Package(_) => Err(PACKAGE_FORM),
        }
    }

    fn evaluate_refinement_row(
        &self,
        row: &CheckedRow,
        current: &ObjectEnvironment,
        bindings: Vec<Value>,
        meter: &mut Meter,
    ) -> Result<Evaluation, InternalFault> {
        crate::value::evaluate_refinement_row(self, row, current, bindings, meter)
    }

    fn evaluate_history_update(
        &self,
        row: &CheckedRow,
        states: (&ObjectEnvironment, &ObjectEnvironment),
        bindings: Vec<Value>,
        meter: &mut Meter,
    ) -> Result<Evaluation, InternalFault> {
        crate::value::evaluate_history_update(self, row, states, bindings, meter)
    }

    fn evaluate_protocol_slot(
        &self,
        body: &CheckedBody,
        frame: Vec<Option<Value>>,
        current: &ObjectEnvironment,
        meter: &mut Meter,
    ) -> Result<Evaluation, InternalFault> {
        crate::value::evaluate_protocol_slot(self, body, frame, current, meter)
    }

    fn evaluate_join_predicate(
        &self,
        function: &NodeKey,
        arguments: Vec<Value>,
        meter: &mut Meter,
    ) -> Result<Evaluation, InternalFault> {
        crate::value::evaluate_join_predicate(self, function, arguments, meter)
    }

    fn evaluate_reward(
        &self,
        reward: &Reward,
        arguments: Vec<Value>,
        meter: &mut Meter,
    ) -> Result<Evaluation, InternalFault> {
        crate::value::evaluate_reward(self, reward, arguments, meter)
    }

    fn state_clause(&self, name: &str) -> Option<&CheckedStateClause> {
        self.graph().state_clause(name)
    }

    fn protocol(&self, name: &str) -> Option<&CheckedProtocol> {
        self.graph().protocol(name)
    }

    fn protocol_node_keys(&self, name: &str) -> Option<&CheckedProtocolNodeKeys> {
        self.graph().protocol_node_keys(name)
    }

    fn own_field_ordinal(&self, owner: &EffectiveId, name: &str) -> Option<usize> {
        self.graph().own_field_ordinal(owner, name)
    }

    fn selected_footprint(
        &self,
        model: &StateModel,
        receiver_type: EffectiveId,
        declaring: EffectiveId,
        operation: &OperationDeclaration,
        arguments: &[(String, EffectiveId)],
        reference_literals: &ReferenceLiterals,
        select: &dyn Fn(NodeKey) -> bool,
    ) -> Result<SymbolicFootprint, FootprintRefusal> {
        derive_selected_footprint_with_reference_literals(
            self.graph(),
            model,
            receiver_type,
            declaring,
            operation,
            arguments,
            reference_literals,
            select,
        )
    }

    fn attempt_footprint_fallback(
        &self,
        model: &StateModel,
        receiver_type: EffectiveId,
        declaring: EffectiveId,
        operation: &OperationDeclaration,
        arguments: &[(String, EffectiveId)],
        reference_literals: &ReferenceLiterals,
        select: &dyn Fn(NodeKey) -> bool,
    ) -> Result<SymbolicFootprint, FootprintRefusal> {
        derive_attempt_footprint_fallback_with_reference_literals(
            self.graph(),
            model,
            receiver_type,
            declaring,
            operation,
            arguments,
            reference_literals,
            select,
        )
    }

    fn footprint(
        &self,
        model: &StateModel,
        receiver_type: EffectiveId,
        declaring: EffectiveId,
        operation: &OperationDeclaration,
        arguments: &[(String, EffectiveId)],
    ) -> Result<SymbolicFootprint, FootprintRefusal> {
        derive_footprint(
            self.graph(),
            model,
            receiver_type,
            declaring,
            operation,
            arguments,
        )
    }

    fn constraint_reads(
        &self,
        model: &StateModel,
        name: &str,
    ) -> Option<Result<BodyReads, FootprintRefusal>> {
        let clause = self.state_clause(name)?;
        Some(
            derive_constraint_reads(self.graph(), model, clause).map(|reads| BodyReads {
                reads,
                literals: reference_literals_of(clause.body()),
            }),
        )
    }

    fn holds_reads(
        &self,
        _model: &StateModel,
        _clause: &NodeKey,
        _position: usize,
    ) -> Result<BodyReads, FootprintRefusal> {
        Err(SOURCE_ROUTE_FOOTPRINT)
    }

    fn activation_guard_reads(
        &self,
        _model: &StateModel,
        _clause: &NodeKey,
    ) -> Result<Option<BodyReads>, FootprintRefusal> {
        Err(SOURCE_ROUTE_FOOTPRINT)
    }

    fn slot_reads(
        &self,
        model: &StateModel,
        name: &str,
        parameters: &[(String, ValueType)],
        body: CheckedNode<'_>,
    ) -> Result<BTreeSet<SymbolicLocation>, FootprintRefusal> {
        derive_slot_reads(self.graph(), model, name, parameters, body)
    }

    fn node_reads(
        &self,
        model: &StateModel,
        name: &str,
        parameters: &[(String, ValueType)],
        body: CheckedNode<'_>,
    ) -> Result<BTreeSet<SymbolicLocation>, FootprintRefusal> {
        derive_node_reads(self.graph(), model, name, parameters, body)
    }

    fn atom_reads(
        &self,
        model: &StateModel,
        over: &(String, ValueType),
        condition: CheckedNode<'_>,
    ) -> Result<BTreeSet<SymbolicLocation>, FootprintRefusal> {
        derive_atom_reads(self.graph(), model, over, condition)
    }

    fn guard_reads(
        &self,
        model: &StateModel,
        binders: &[(String, ValueType)],
        condition: CheckedNode<'_>,
    ) -> Result<BTreeSet<SymbolicLocation>, FootprintRefusal> {
        derive_guard_reads(self.graph(), model, binders, condition)
    }
}

/// The refusal a package-route derivation raises on the source route, whose
/// input holds checked bodies and no package nodes.
const SOURCE_ROUTE_FOOTPRINT: FootprintRefusal = FootprintRefusal::Unresolved {
    what: "package form on the source route",
};

/// The refusal a footprint derivation over a source checked body raises on
/// the package route, whose input holds no such body.
const NO_SOURCE_FOOTPRINT: FootprintRefusal = FootprintRefusal::Unresolved {
    what: "source checked form on the package route",
};

impl ModelEvaluator for BarePackage<'_> {
    fn package_protocol_observation(
        &self,
        model: &DomainPackageRef,
        packages: &fsl_semantics::model::intake::PackageInput,
        limits: fsl_semantics::model::accounting::ModelNormalizationLimits,
        state: &fsl_semantics::model::state::ModelState,
        identity: fsl_semantics::model::observation::DocumentRef,
    ) -> Result<
        filament_semantic_value::admission::ObservationAdmission<
            fsl_package::AdmittedProtocolObservation<'static>,
        >,
        InternalFault,
    > {
        Ok(self.admit_protocol_state(model, packages, limits, state, identity))
    }

    fn evaluate_clause(
        &self,
        clause: &QualifiedName,
        observations: &AdmittedObservations,
        meter: &mut Meter,
    ) -> Result<ClauseEvaluation, CallFailure> {
        CheckedPackageEvaluation::evaluate_clause(self, clause, observations, meter)
    }

    fn evaluate_clause_set(
        &self,
        clauses: &[(QualifiedName, &AdmittedObservations)],
        limits: ScalarLimits,
        _cancel: Option<&filament_exact::Cancel>,
    ) -> Result<ClauseSetEvaluation, CallFailure> {
        CheckedPackageEvaluation::evaluate_clause_set(self, clauses, limits)
    }

    fn evaluate_holds(
        &self,
        atom: &crate::value::HoldsAtom,
        over: Value,
        captures: &[Value],
        objects: &ObjectEnvironment,
        meter: &mut Meter,
    ) -> Result<Evaluation, CallFailure> {
        CheckedPackageEvaluation::evaluate_holds(self, atom, over, captures, objects, meter)
    }

    fn evaluate_model_holds(
        &self,
        atom: &crate::value::HoldsAtom,
        over: Value,
        captures: &[Value],
        objects: &ObjectEnvironment,
        reference_bindings: &crate::simulation::model::ReferenceBindings,
        meter: &mut Meter,
    ) -> Result<Evaluation, CallFailure> {
        CheckedPackageEvaluation::evaluate_model_holds(
            self,
            atom,
            over,
            captures,
            objects,
            reference_bindings,
            meter,
        )
    }

    fn evaluate_guard(
        &self,
        clause: &NodeKey,
        over: Value,
        objects: &ObjectEnvironment,
        meter: &mut Meter,
    ) -> Result<Evaluation, CallFailure> {
        CheckedPackageEvaluation::evaluate_guard(self, clause, over, objects, meter)
    }

    fn evaluate_model_guard(
        &self,
        clause: &NodeKey,
        over: Value,
        objects: &ObjectEnvironment,
        reference_bindings: &crate::simulation::model::ReferenceBindings,
        meter: &mut Meter,
    ) -> Result<Evaluation, CallFailure> {
        CheckedPackageEvaluation::evaluate_model_guard(
            self,
            clause,
            over,
            objects,
            reference_bindings,
            meter,
        )
    }

    fn temporal_reference_literals(
        &self,
        clause: &CheckedTemporalClause,
        key: &NodeKey,
    ) -> Vec<(EffectiveId, Vec<u8>)> {
        let mut literals = BTreeSet::new();
        let mut collect = |projection: &fsl_package::ReaderProtocolProjection| {
            let mut index = 0;
            while let Some(kind) = projection.kind(index) {
                if let filament_checked::node::NodeKind::ReferenceLiteral {
                    object_type,
                    identity,
                } = kind
                {
                    literals.insert((object_type.clone(), identity.clone()));
                }
                index += 1;
            }
        };
        for (index, formula) in clause.formula().nodes().iter().enumerate() {
            if matches!(formula, fsl_semantics::check::FormulaNode::Holds { .. }) {
                if let Ok(term) = self.project_holds(key, index) {
                    collect(term.projection());
                }
            }
        }
        if let Ok(Some(term)) = self.project_guard(key) {
            collect(term.projection());
        }
        literals.into_iter().collect()
    }

    fn evaluate_frame(
        &self,
        frame: &NodeKey,
        invocation: &fsl_semantics::model::observation::AdmittedInvocation<'_>,
        operations: &OperationTable,
        _meter: &mut Meter,
    ) -> Result<crate::value::FrameEvaluation, CallFailure> {
        crate::bare::evaluate_frame(self, operations, frame, invocation)
    }

    fn evaluate_collection_write(
        &self,
        clause: &QualifiedName,
        observations: &AdmittedObservations,
        field: &str,
        written: &Written,
        meter: &mut Meter,
    ) -> Result<Option<WrittenCollection>, CallFailure> {
        CheckedPackageEvaluation::evaluate_collection_write(
            self,
            clause,
            observations,
            field,
            written,
            meter,
        )
    }

    fn evaluate_post_targets(
        &self,
        clause: &QualifiedName,
        observations: &AdmittedObservations,
        meter: &mut dyn FnMut() -> Meter,
    ) -> Result<PostTargets, CallFailure> {
        crate::value::bare_post_targets(self, clause, observations, meter)
    }

    fn evaluate_hyper_holds(
        &self,
        clause: &CheckedHyperClause,
        atom: FormulaNodeId,
        slots: Vec<crate::hyper::HyperSlot>,
        components: &[&ObjectEnvironment],
        meter: &mut Meter,
    ) -> Result<Evaluation, CallFailure> {
        CheckedPackageEvaluation::evaluate_hyper_holds(self, clause, atom, slots, components, meter)
    }

    fn evaluate_state_predicate(
        &self,
        _predicate: &dyn StatePredicate,
        _current: &ObjectEnvironment,
        _over: Option<ObjectReference>,
        _further: Vec<Value>,
        _meter: &mut Meter,
    ) -> Result<Evaluation, InternalFault> {
        Err(SOURCE_FORM)
    }

    fn evaluate_state_predicate_at_absent_key(
        &self,
        _predicate: &dyn StatePredicate,
        _current: &ObjectEnvironment,
        _key: ObjectReference,
        _binding: String,
        _meter: &mut Meter,
    ) -> Result<Evaluation, InternalFault> {
        Err(SOURCE_FORM)
    }

    fn state_graph_reads(
        &self,
        model: &StateModel,
        claim: &fsl_foundation::source::provenance::OccurrenceKey,
        role: fsl_semantics::check::PredicateRole,
    ) -> Result<BodyReads, FootprintRefusal> {
        let unresolved = || FootprintRefusal::Unresolved {
            what: "state-graph predicate",
        };
        let predicate = self
            .project_state_graph_predicate(claim, role)
            .map_err(|_| unresolved())?;
        let world = self.world().map_err(|_| unresolved())?;
        let over = predicate
            .parameters()
            .first()
            .map(|(_, slot)| *slot)
            .into_iter()
            .collect::<Vec<_>>();
        crate::value::package_term_reads(predicate.projection(), world.types(), model, &over)
    }

    fn evaluate_state_graph_predicate(
        &self,
        claim: &fsl_foundation::source::provenance::OccurrenceKey,
        role: fsl_semantics::check::PredicateRole,
        current: &ObjectEnvironment,
        over: Option<ObjectReference>,
        further: Vec<Value>,
        meter: &mut Meter,
    ) -> Result<Evaluation, InternalFault> {
        let over = over.map_or(
            crate::value::StateGraphOver::None,
            crate::value::StateGraphOver::Bound,
        );
        crate::value::evaluate_bare_state_graph_predicate(
            self, claim, role, current, over, further, meter,
        )
    }

    fn evaluate_state_graph_predicate_at_absent_key(
        &self,
        claim: &fsl_foundation::source::provenance::OccurrenceKey,
        role: fsl_semantics::check::PredicateRole,
        current: &ObjectEnvironment,
        key: ObjectReference,
        binding: String,
        meter: &mut Meter,
    ) -> Result<Evaluation, InternalFault> {
        crate::value::evaluate_bare_state_graph_predicate(
            self,
            claim,
            role,
            current,
            crate::value::StateGraphOver::Absent { key, binding },
            Vec::new(),
            meter,
        )
    }

    fn evaluate_activation_guard(
        &self,
        _guard: &dyn StatePredicate,
        _current: &ObjectEnvironment,
        _bound: ObjectReference,
        _meter: &mut Meter,
    ) -> Result<Evaluation, InternalFault> {
        Err(SOURCE_FORM)
    }

    fn evaluate_activation_guard_at_absent_key(
        &self,
        _guard: &dyn StatePredicate,
        _current: &ObjectEnvironment,
        _key: ObjectReference,
        _binding: String,
        _meter: &mut Meter,
    ) -> Result<Evaluation, InternalFault> {
        Err(SOURCE_FORM)
    }

    fn evaluate_capture(
        &self,
        _capture: &dyn StatePredicate,
        _current: &ObjectEnvironment,
        _bindings: Vec<Value>,
        _absent: Option<(Vec<usize>, String)>,
        _meter: &mut Meter,
    ) -> Result<Evaluation, InternalFault> {
        Err(SOURCE_FORM)
    }

    fn evaluate_terminal_predicate(
        &self,
        predicate: TerminalWhen<'_>,
        current: &ObjectEnvironment,
        self_object: Option<ObjectReference>,
        meter: &mut Meter,
    ) -> Result<Evaluation, InternalFault> {
        match predicate {
            TerminalWhen::Package(predicate) => crate::value::evaluate_bare_terminal_predicate(
                self,
                predicate,
                current,
                self_object,
                meter,
            ),
            TerminalWhen::Source(_) => Err(SOURCE_FORM),
        }
    }

    fn evaluate_refinement_row(
        &self,
        _row: &CheckedRow,
        _current: &ObjectEnvironment,
        _bindings: Vec<Value>,
        _meter: &mut Meter,
    ) -> Result<Evaluation, InternalFault> {
        Err(SOURCE_FORM)
    }

    fn evaluate_history_update(
        &self,
        _row: &CheckedRow,
        _states: (&ObjectEnvironment, &ObjectEnvironment),
        _bindings: Vec<Value>,
        _meter: &mut Meter,
    ) -> Result<Evaluation, InternalFault> {
        Err(SOURCE_FORM)
    }

    fn evaluate_protocol_slot(
        &self,
        _body: &CheckedBody,
        _frame: Vec<Option<Value>>,
        _current: &ObjectEnvironment,
        _meter: &mut Meter,
    ) -> Result<Evaluation, InternalFault> {
        Err(SOURCE_FORM)
    }

    fn evaluate_package_protocol_slot(
        &self,
        slot: &ProtocolSlot,
        supply: &ProtocolSupply<'_>,
        meter: &mut Meter,
    ) -> Result<ProtocolSlotEvaluation, CallFailure> {
        CheckedPackageEvaluation::evaluate_protocol_slot(self, slot, supply, meter)
    }

    fn evaluate_join_predicate(
        &self,
        _function: &NodeKey,
        _arguments: Vec<Value>,
        _meter: &mut Meter,
    ) -> Result<Evaluation, InternalFault> {
        Err(SOURCE_FORM)
    }

    fn evaluate_reward(
        &self,
        _reward: &Reward,
        _arguments: Vec<Value>,
        _meter: &mut Meter,
    ) -> Result<Evaluation, InternalFault> {
        Err(SOURCE_FORM)
    }

    fn state_clause(&self, _name: &str) -> Option<&CheckedStateClause> {
        None
    }

    fn protocol(&self, _name: &str) -> Option<&CheckedProtocol> {
        None
    }

    fn package_protocol(
        &self,
        name: &str,
    ) -> Option<fsl_package::protocol_view::ProtocolExecutionView<'_>> {
        fsl_package::AnalysisView::protocol_execution_named(self, name)
            .ok()
            .flatten()
    }

    fn package_protocol_slot(
        &self,
        node: &CheckedNodeId,
        member: &str,
        index: Option<usize>,
    ) -> Option<ProtocolSlot> {
        Some(ProtocolSlot {
            node: self.node_key(node)?,
            member: member.to_owned(),
            index,
        })
    }

    fn package_protocol_slot_binders(&self, slot: &ProtocolSlot) -> Option<Vec<NodeKey>> {
        let selected = self
            .select_protocol_slot(&fsl_package::ProtocolSlotDescriptor::new(
                slot.node.clone(),
                slot.member.clone(),
                slot.index,
            ))
            .ok()?;
        Some(
            selected
                .sources()
                .iter()
                .map(|source| source.binder().clone())
                .collect(),
        )
    }

    fn package_node_key(&self, node: &CheckedNodeId) -> Option<NodeKey> {
        self.node_key(node)
    }

    fn package_protocol_operation(&self, anchor: &CheckedNodeId) -> Option<(EffectiveId, String)> {
        self.protocol_operation(anchor)
            .ok()
            .map(|operation| (operation.declaring, operation.operation))
    }

    fn package_protocol_role_model(&self, model: &CheckedNodeId) -> Option<EffectiveId> {
        self.protocol_role_model(model).ok()
    }

    fn package_protocol_parameter_type(&self, binder: &CheckedNodeId) -> Option<ValueType> {
        self.protocol_parameter_type(binder).ok()
    }

    fn protocol_node_keys(&self, _name: &str) -> Option<&CheckedProtocolNodeKeys> {
        None
    }

    fn own_field_ordinal(&self, _owner: &EffectiveId, _name: &str) -> Option<usize> {
        None
    }

    fn selected_footprint(
        &self,
        model: &StateModel,
        receiver_type: EffectiveId,
        declaring: EffectiveId,
        operation: &OperationDeclaration,
        arguments: &[(String, EffectiveId)],
        reference_literals: &ReferenceLiterals,
        select: &dyn Fn(NodeKey) -> bool,
    ) -> Result<SymbolicFootprint, FootprintRefusal> {
        package_footprint(
            self,
            model,
            receiver_type,
            declaring,
            operation,
            arguments,
            reference_literals,
            select,
        )
    }

    fn attempt_footprint_fallback(
        &self,
        model: &StateModel,
        receiver_type: EffectiveId,
        declaring: EffectiveId,
        operation: &OperationDeclaration,
        arguments: &[(String, EffectiveId)],
        reference_literals: &ReferenceLiterals,
        select: &dyn Fn(NodeKey) -> bool,
    ) -> Result<SymbolicFootprint, FootprintRefusal> {
        package_footprint(
            self,
            model,
            receiver_type,
            declaring,
            operation,
            arguments,
            reference_literals,
            select,
        )
    }

    fn footprint(
        &self,
        model: &StateModel,
        receiver_type: EffectiveId,
        declaring: EffectiveId,
        operation: &OperationDeclaration,
        arguments: &[(String, EffectiveId)],
    ) -> Result<SymbolicFootprint, FootprintRefusal> {
        package_footprint(
            self,
            model,
            receiver_type,
            declaring,
            operation,
            arguments,
            &ReferenceLiterals::new(),
            &|_| true,
        )
    }

    fn constraint_reads(
        &self,
        model: &StateModel,
        name: &str,
    ) -> Option<Result<BodyReads, FootprintRefusal>> {
        let declarations = match self.state_clause_declarations() {
            Ok(declarations) => declarations,
            Err(failure) => return Some(Err(selection_refusal(name, failure))),
        };
        let declaration = declarations
            .into_iter()
            .find(|declaration| declaration.name == name)?;
        Some(projected_clause(self, declaration).and_then(|clause| {
            let world = self
                .world()
                .map_err(|_| unresolved_package("package world"))?;
            crate::value::package_constraint_reads(world.types(), model, &clause)
        }))
    }

    fn holds_reads(
        &self,
        model: &StateModel,
        clause: &NodeKey,
        position: usize,
    ) -> Result<BodyReads, FootprintRefusal> {
        let term = self
            .project_holds(clause, position)
            .map_err(|failure| selection_refusal("holds", failure))?;
        let world = self
            .world()
            .map_err(|_| unresolved_package("package world"))?;
        let over = receiver_slots(term.binders(), |binder| binder == ClauseBinder::Over);
        crate::value::package_term_reads(term.projection(), world.types(), model, &over)
    }

    fn activation_guard_reads(
        &self,
        model: &StateModel,
        clause: &NodeKey,
    ) -> Result<Option<BodyReads>, FootprintRefusal> {
        let Some(term) = self
            .project_guard(clause)
            .map_err(|failure| selection_refusal("guard", failure))?
        else {
            return Ok(None);
        };
        let world = self
            .world()
            .map_err(|_| unresolved_package("package world"))?;
        let receivers = receiver_slots(term.binders(), |binder| {
            matches!(binder, ClauseBinder::Over | ClauseBinder::Trigger)
        });
        crate::value::package_guard_reads(term.projection(), world.types(), model, &receivers)
            .map(Some)
    }

    fn slot_reads(
        &self,
        _model: &StateModel,
        _name: &str,
        _parameters: &[(String, ValueType)],
        _body: CheckedNode<'_>,
    ) -> Result<BTreeSet<SymbolicLocation>, FootprintRefusal> {
        Err(NO_SOURCE_FOOTPRINT)
    }

    fn node_reads(
        &self,
        _model: &StateModel,
        _name: &str,
        _parameters: &[(String, ValueType)],
        _body: CheckedNode<'_>,
    ) -> Result<BTreeSet<SymbolicLocation>, FootprintRefusal> {
        Err(NO_SOURCE_FOOTPRINT)
    }

    fn atom_reads(
        &self,
        _model: &StateModel,
        _over: &(String, ValueType),
        _condition: CheckedNode<'_>,
    ) -> Result<BTreeSet<SymbolicLocation>, FootprintRefusal> {
        Err(NO_SOURCE_FOOTPRINT)
    }

    fn guard_reads(
        &self,
        _model: &StateModel,
        _binders: &[(String, ValueType)],
        _condition: CheckedNode<'_>,
    ) -> Result<BTreeSet<SymbolicLocation>, FootprintRefusal> {
        Err(NO_SOURCE_FOOTPRINT)
    }
}

/// The local slots of a projected clause term whose binder `reads_as_receiver`
/// selects. A package term gives frame slots only to the binders it reads, so
/// the receiver is found by the binder, never by a fixed slot.
fn receiver_slots(
    binders: &[(usize, ClauseBinder)],
    reads_as_receiver: impl Fn(ClauseBinder) -> bool,
) -> Vec<usize> {
    binders
        .iter()
        .filter(|(_, binder)| reads_as_receiver(*binder))
        .map(|(slot, _)| *slot)
        .collect()
}

/// A package-route footprint refusal for a fact the package does not hold.
fn unresolved_package(what: &'static str) -> FootprintRefusal {
    FootprintRefusal::Unresolved { what }
}

/// The refusal of a package node that does not project: a form with no
/// footprint rule is unsupported as the source route reports it; any other
/// selection failure leaves the node unresolved.
fn selection_refusal(clause: &str, failure: fsl_package::BareSelectionFailure) -> FootprintRefusal {
    match failure {
        fsl_package::BareSelectionFailure::Unsupported(_) => FootprintRefusal::UnsupportedForm {
            clause: clause.to_owned(),
            form: "package expression form",
        },
        _ => unresolved_package("package node"),
    }
}

/// `declaration`'s body projected from the package's nodes.
fn projected_clause(
    package: &BarePackage<'_>,
    declaration: fsl_package::BareStateClauseDeclaration,
) -> Result<crate::value::ProjectedClause, FootprintRefusal> {
    let clause = package
        .project_state_clause(&declaration.identity, &declaration.name)
        .map_err(|failure| selection_refusal(&declaration.name, failure))?;
    Ok(crate::value::ProjectedClause {
        declaration,
        clause,
    })
}

/// FR-155 "Derivation" over the package route: the footprint of `operation`
/// from the state clauses the package names for it, `select` admitting each
/// by its node key, by the derivation the source route runs.
#[allow(
    clippy::too_many_arguments,
    reason = "the footprint derivation takes the operation, its receiver identities, arguments, literals and selector"
)]
fn package_footprint(
    package: &BarePackage<'_>,
    model: &StateModel,
    receiver_type: EffectiveId,
    declaring: EffectiveId,
    operation: &OperationDeclaration,
    arguments: &[(String, EffectiveId)],
    reference_literals: &ReferenceLiterals,
    select: &dyn Fn(NodeKey) -> bool,
) -> Result<SymbolicFootprint, FootprintRefusal> {
    let world = package
        .world()
        .map_err(|_| unresolved_package("package world"))?;
    let mut clauses = Vec::new();
    for declaration in package
        .state_clause_declarations()
        .map_err(|_| unresolved_package("state clause declarations"))?
    {
        let anchored = declaration.operation.as_ref().is_some_and(|owner| {
            owner.declaring == declaring && owner.operation == operation.name()
        });
        if anchored && select(declaration.identity.clone()) {
            clauses.push(projected_clause(package, declaration)?);
        }
    }
    crate::value::package_operation_footprint(
        world.types(),
        model,
        &clauses,
        reference_literals,
        fsl_semantics::check::footprint::PackageOperation {
            receiver_type,
            declaring,
            operation,
            arguments,
        },
        select,
    )
}

/// The checked predicate of `role` of the source package's state-graph claim
/// at `claim`: a written one, which the port evaluates.
fn source_predicate<'p>(
    package: &'p CheckedPackage,
    claim: &fsl_foundation::source::provenance::OccurrenceKey,
    role: fsl_semantics::check::PredicateRole,
) -> Result<&'p fsl_semantics::check::CheckedPredicateBody, InternalFault> {
    use fsl_semantics::check::{CheckedStatePredicate, PredicateRole, StateGraphForm};
    let fault = |cause: &'static str| InternalFault::new("FR-940", cause);
    let claim = package
        .graph()
        .state_graph_claims()
        .iter()
        .find(|held| held.occurrence() == Some(claim))
        .ok_or_else(|| fault("state-graph-claim-is-in-the-package"))?;
    let (from, target) = match claim.form() {
        StateGraphForm::Possible { target } => (None, target),
        StateGraphForm::AlwaysPossible { from, target } => (Some(from), target),
        StateGraphForm::UniquePath { from, to } => (Some(from), to),
    };
    let predicate = match role {
        PredicateRole::From => from,
        PredicateRole::Target => Some(target),
    };
    match predicate {
        Some(CheckedStatePredicate::Checked(body)) => Ok(body),
        _ => Err(fault("state-graph-predicate-is-written")),
    }
}

#[cfg(test)]
mod tests {
    use super::{receiver_slots, ClauseBinder};
    use ix_trace_rs::trace;

    /// FR-940-AC-11, FR-156: a package term gives frame slots to the binders
    /// it reads and the next slots to the variables it introduces, so the
    /// receiver is the slot of the `over` binder (and, for a guard, of the
    /// trigger binder), whatever number that is.
    #[trace("TC-3034", "FR-940-AC-11")]
    #[test]
    fn the_receiver_slots_follow_the_binder_a_term_reads() {
        let binders = [(0, ClauseBinder::Trigger), (1, ClauseBinder::Capture(0))];
        assert_eq!(
            receiver_slots(&binders, |b| b == ClauseBinder::Over),
            Vec::<usize>::new()
        );
        assert_eq!(
            receiver_slots(&binders, |b| matches!(
                b,
                ClauseBinder::Over | ClauseBinder::Trigger
            )),
            [0]
        );
        let binders = [(2, ClauseBinder::Over), (3, ClauseBinder::Trigger)];
        assert_eq!(receiver_slots(&binders, |b| b == ClauseBinder::Over), [2]);
        assert_eq!(
            receiver_slots(&binders, |b| matches!(
                b,
                ClauseBinder::Over | ClauseBinder::Trigger
            )),
            [2, 3]
        );
    }
}
