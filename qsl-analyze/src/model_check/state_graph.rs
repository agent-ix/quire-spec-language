// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-166, FR-167 and FR-168 (ADR-022 GE-1, GE-2): [`check_state_graph`]
//! decides a subject's state-graph items and its deadlock-freedom item.
//!
//! A run has three phases:
//!
//! 1. **Phase 0** ([`sample`], FR-166): for each `possible` item instance and
//!    each initial state, seeded FR-101 walks look for a witness before any
//!    exploration. A walk only finds a witness sooner; it settles nothing.
//! 2. **Phase 1** ([`explore`], FR-167): one canonical breadth-first
//!    exploration of the subject's `ModelSystem`, with no property
//!    automaton, retains every edge, labels every node with each predicate
//!    of each item instance and marks every node it did not fully expand
//!    open, with the reason. The deadlock-freedom item is decided on the
//!    same exploration.
//! 3. **Decision** ([`decide`], FR-168): one linear pass over the explored
//!    graph per item instance, returning evidence only where open nodes
//!    cannot change it.
//!
//! A `Witnessed` or `Holds` outcome carries the run's FR-169 certificate,
//! which `qsl-replay`'s `check_state_graph` checks. The settlement of the
//! outcomes (FR-169) and the replay of the evidence (FR-170) are not part of
//! this module.

mod decide;
mod explore;
mod sample;

use std::collections::{BTreeMap, BTreeSet};

use qsl_eval::model_subject::ModelInstance;
use qsl_eval::model_subject::ModelSubject;
use qsl_eval::simulation::model::{check_model_bounds, ModelSystem, PredicateReading};
use qsl_eval::simulation::{
    ExpansionLimit, ExpansionStop, NotSimulated, Permutation, SymmetryGroup,
};
use qsl_foundation::digest::DigestRecord;
use qsl_foundation::selection::DefinitionRef;
use qsl_replay::ReplayRefusal;
use qsl_replay::{
    settle_state_graph, AppliedReduction, GraphReplayResult, IncompleteCause, ModelCheckLimit,
    ModelCheckLimits, ModelCheckOutcome, OpenCause, Proviso, ReachedLimit, RunEnd,
    StateGraphInstanceOutcome, StateGraphMethod, StateGraphOutcome, StateGraphRecord,
    DEFAULT_WITNESS_SEED,
};
use qsl_route::reductions::Reductions;
use qsl_semantics::check::footprint::SymbolicLocation;
use qsl_semantics::check::{
    CheckedPredicateBody, CheckedStateGraphClaim, CheckedStatePredicate, PredicateRole,
    StateGraphForm,
};
use quire_exact::{Cancel, ObjectReference, ValueType};
use quire_semantic_value::model_transition::ModelTransition;

use super::check::{fault, BOUND_POSITIONS};
use super::ModelCheckRefusal;

/// One state-graph item of a request: a checked claim. Its instances are
/// derived from the subject's universes (FR-125 Instancing): one per key of
/// each population that admits its `over` type, in canonical order; a claim
/// with no `over` parameter has the one instance with no binding.
pub struct StateGraphItem<'a> {
    /// The checked claim.
    pub claim: &'a CheckedStateGraphClaim,
    /// An optional `over` binding to decide for this request item. When
    /// absent, FR-125 instancing supplies every compatible key in canonical
    /// order. When present, the request runs just that binding while keeping
    /// the subject's full state space.
    pub binding: Option<ModelInstance>,
}

impl<'a> StateGraphItem<'a> {
    /// Decide every instance of `claim` selected by the subject's universes.
    pub fn new(claim: &'a CheckedStateGraphClaim) -> Self {
        Self {
            claim,
            binding: None,
        }
    }

    /// Decide only `binding` of an `over` claim, retaining the full subject
    /// and its transitions for state-graph exploration.
    pub fn for_binding(claim: &'a CheckedStateGraphClaim, binding: ModelInstance) -> Self {
        Self {
            claim,
            binding: Some(binding),
        }
    }
}

/// One EN-1 run over a model subject's state-graph items (FR-166, FR-167).
pub struct StateGraphRequest<'a> {
    /// The model subject.
    pub subject: ModelSubject<'a>,
    /// The subject's state-graph items.
    pub items: Vec<StateGraphItem<'a>>,
    /// State-space reductions selected by the request (FR-159, ADR-022 GR-1).
    pub reductions: Reductions,
    /// Whether the run also decides the subject's derived deadlock-freedom
    /// item (FR-124).
    pub deadlock_freedom: bool,
    /// The search horizon; `None` sets none. A method parameter, never a
    /// limit.
    pub max_depth: Option<usize>,
    /// The run's caller-raisable budgets.
    pub limits: ModelCheckLimits,
    /// Phase 0's seed; `None` runs under [`DEFAULT_WITNESS_SEED`].
    pub seed: Option<u64>,
}

/// A node handle of an [`ExploredStateGraph`], in discovery order.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NodeIndex(usize);

impl NodeIndex {
    /// The zero-based discovery index.
    pub const fn index(self) -> usize {
        self.0
    }
}

/// How one predicate of one item instance read at a node.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PredicateLabel {
    /// The item's index in the request.
    pub item: usize,
    /// The instance's index in the item.
    pub instance: usize,
    /// The predicate.
    pub role: PredicateRole,
    /// Its reading at the node.
    pub reading: PredicateReading,
}

/// One explored node.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GraphNode {
    /// The node's `quire.simulation.state-key/v1` digest.
    pub key: DigestRecord,
    /// Its breadth-first depth.
    pub depth: u64,
    /// Every predicate's reading, by item, instance and predicate; empty for
    /// a node the run discovered and never took from its frontier.
    pub labels: Vec<PredicateLabel>,
}

/// One retained edge.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GraphEdge {
    /// The source node.
    pub source: NodeIndex,
    /// The FR-120 transition identity.
    pub transition: ModelTransition,
    /// The post-state's node.
    pub target: NodeIndex,
    /// The permutation that maps the concrete successor to its canonical
    /// target; the identity when symmetry is not selected.
    pub permutation: Permutation,
}

/// The counts of one exploration.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct GraphStats {
    /// Distinct nodes discovered.
    pub nodes: u64,
    /// Edges retained.
    pub edges: u64,
    /// The greatest depth of a discovered node.
    pub depth: u64,
}

/// FR-167 Outputs: the explored state graph.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExploredStateGraph {
    /// The nodes, in FR-101 canonical discovery order.
    pub nodes: Vec<GraphNode>,
    /// The retained edges, each node's in canonical order: transition
    /// identity bytes, then post-state key bytes.
    pub edges: Vec<GraphEdge>,
    /// The nodes the run did not fully expand, with the reason.
    pub open: BTreeMap<NodeIndex, OpenCause>,
    /// How the run ended.
    pub end: RunEnd,
    /// The run's counts.
    pub stats: GraphStats,
    /// Each node's outgoing edges, by index into `edges`, in canonical order.
    outgoing: Vec<Vec<usize>>,
    /// The edge that first reached each node: its breadth-first tree.
    parent: Vec<Option<usize>>,
    /// Each node's initial-state index; for a node that is no initial state,
    /// its tree root's.
    root: Vec<usize>,
    /// The node of each initial state of the subject, by its index in the
    /// subject's initial list.
    initial: Vec<NodeIndex>,
}

impl ExploredStateGraph {
    /// The retained edges out of `node`, in canonical order.
    pub fn outgoing(&self, node: NodeIndex) -> impl Iterator<Item = &GraphEdge> {
        self.outgoing
            .get(node.0)
            .into_iter()
            .flatten()
            .filter_map(|edge| self.edges.get(*edge))
    }

    /// The node of each initial state of the subject, in the subject's order.
    pub fn initial(&self) -> &[NodeIndex] {
        &self.initial
    }

    /// The reading of `role` for `item`'s `instance` at `node`; `None` for a
    /// node with no labels.
    pub fn reading(
        &self,
        node: NodeIndex,
        item: usize,
        instance: usize,
        role: PredicateRole,
    ) -> Option<&PredicateReading> {
        self.nodes.get(node.0)?.labels.iter().find_map(|label| {
            (label.item == item && label.instance == instance && label.role == role)
                .then_some(&label.reading)
        })
    }
}

/// What one state-graph run returns.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StateGraphRun {
    /// The explored graph; empty when the run stopped before exploring or
    /// every requested claim had no instances and deadlock freedom was not
    /// requested.
    pub graph: ExploredStateGraph,
    /// Each item's decisions in request order. Instances have one decision
    /// each; an item with no instances has one item-level `NoInstance`
    /// marker, not a per-instance decision (FR-125).
    pub items: Vec<Vec<StateGraphInstanceOutcome>>,
    /// The deadlock-freedom item's outcome, when the request asked for it.
    pub deadlock: Option<ModelCheckOutcome>,
    /// The seed phase 0 ran under.
    pub seed: u64,
}

/// The predicates of a checked form, each with its role; the constant
/// `true` an omitted `from` stands for has no body and is read as true.
fn predicates(form: &StateGraphForm) -> Vec<(PredicateRole, &CheckedStatePredicate)> {
    match form {
        StateGraphForm::Possible { target } => vec![(PredicateRole::Target, target)],
        StateGraphForm::AlwaysPossible { from, target } => {
            vec![(PredicateRole::From, from), (PredicateRole::Target, target)]
        }
        StateGraphForm::UniquePath { from, to } => {
            vec![(PredicateRole::From, from), (PredicateRole::To, to)]
        }
    }
}

/// One instance of one item as the run reads it.
struct InstanceView<'a> {
    item: usize,
    instance: usize,
    over: Option<ObjectReference>,
    model_instance: Option<ModelInstance>,
    /// The instance's orbit under the declared symmetry group.
    orbit: Vec<String>,
    predicates: Vec<(PredicateRole, Option<&'a CheckedPredicateBody>)>,
}

/// The instances of `items`, item by item, derived from `system`'s
/// universes (FR-125 Instancing).
fn instances<'a>(
    items: &[StateGraphItem<'a>],
    system: &ModelSystem<'_>,
) -> Result<Vec<Vec<InstanceView<'a>>>, ModelCheckRefusal> {
    items
        .iter()
        .enumerate()
        .map(|(item, entry)| {
            let bodies: Vec<_> = predicates(entry.claim.form())
                .into_iter()
                .map(|(role, predicate)| {
                    let body = match predicate {
                        CheckedStatePredicate::ConstantTrue => None,
                        CheckedStatePredicate::Checked(body) => Some(body),
                    };
                    (role, body)
                })
                .collect();
            let mut bindings: Vec<(Option<ObjectReference>, Option<ModelInstance>)> =
                match entry.claim.over() {
                    None => vec![(None, None)],
                    Some(over) => system
                        .instances(&ValueType::Reference(over.object))
                        .map_err(bounds_refusal)?
                        .iter()
                        .map(|instance| {
                            system
                                .bound_reference(over.object, instance)
                                .map(|reference| (Some(reference), Some(instance.clone())))
                                .map_err(|_| fault("state-graph-instance-references"))
                        })
                        .collect::<Result<_, _>>()?,
                };
            if let Some(selected) = &entry.binding {
                bindings.retain(|(_, binding)| binding.as_ref() == Some(selected));
            }
            Ok(bindings
                .into_iter()
                .enumerate()
                .map(|(instance, (over, model_instance))| InstanceView {
                    item,
                    instance,
                    over,
                    model_instance,
                    orbit: Vec::new(),
                    predicates: bodies.clone(),
                })
                .collect())
        })
        .collect()
}

/// Collapse symmetry-equivalent claim instances to their least binding.
fn symmetry_instances<'a>(
    views: Vec<Vec<InstanceView<'a>>>,
    declared: &SymmetryGroup,
) -> Result<Vec<Vec<InstanceView<'a>>>, ModelCheckRefusal> {
    views
        .into_iter()
        .map(|instances| {
            let mut result = Vec::new();
            for view in instances {
                let Some(binding) = view.model_instance.clone() else {
                    let mut view = view;
                    view.instance = result.len();
                    result.push(view);
                    continue;
                };
                let mut orbit = declared
                    .class_of(&binding.population, &binding.key)
                    .map_or_else(|| vec![binding.key.clone()], |class| class.keys.clone());
                orbit.sort();
                let representative = orbit
                    .first()
                    .cloned()
                    .ok_or_else(|| fault("a-symmetry-orbit-has-a-representative"))?;
                if binding.key != representative {
                    continue;
                }
                let mut view = view;
                view.instance = result.len();
                view.orbit = orbit;
                result.push(view);
            }
            Ok(result)
        })
        .collect()
}

/// The common subgroup that preserves every retained instance label.
fn common_stabiliser(views: &[Vec<InstanceView<'_>>], declared: &SymmetryGroup) -> SymmetryGroup {
    let fixed: BTreeSet<_> = views
        .iter()
        .flatten()
        .filter_map(|view| {
            view.model_instance
                .as_ref()
                .map(|binding| (binding.population.clone(), binding.key.clone()))
        })
        .collect();
    SymmetryGroup {
        classes: declared
            .classes
            .iter()
            .filter_map(|class| {
                let mut class = class.clone();
                class
                    .keys
                    .retain(|key| !fixed.contains(&(class.population.clone(), key.clone())));
                (class.keys.len() >= 2).then_some(class)
            })
            .collect(),
    }
}

/// The refusal of an unbounded or unclassifiable subject.
fn bounds_refusal(refusal: NotSimulated) -> ModelCheckRefusal {
    match refusal {
        NotSimulated::RequiresBound(roots) => ModelCheckRefusal::RequiresBound(roots),
        NotSimulated::Extent(failure) => ModelCheckRefusal::Extent(Box::new(failure)),
        _ => fault("bounds-precheck-returns-only-requires-bound-or-extent"),
    }
}

/// A stop that ended a run before its frontier emptied: a named limit, a
/// cancellation, or a stop no named limit caused.
#[derive(Clone, Copy, Debug)]
pub(super) struct Halt {
    pub(super) cause: IncompleteCause,
    pub(super) limit: Option<ReachedLimit>,
}

impl Halt {
    pub(super) fn end(self) -> RunEnd {
        RunEnd::Stopped {
            cause: self.cause,
            limit: self.limit,
        }
    }
}

/// The stop at the limit `limit`, set to `value`.
pub(super) fn stopped(limit: ModelCheckLimit, value: u64) -> Halt {
    Halt {
        cause: IncompleteCause::ResourceExhausted,
        limit: Some(ReachedLimit { limit, value }),
    }
}

/// The one generator phase 0 samples with (FR-101).
pub(super) fn sampler() -> Result<DefinitionRef, ModelCheckRefusal> {
    DefinitionRef::new("quire", "quire.simulation.sampler/v1").map_err(|_| fault("sampler-ref"))
}

/// Whether the greatest trace index of phase 0, `(count - 1) × samples +
/// samples - 1` for `count` initial states, fits `u64` (FR-166), computed
/// with checked arithmetic.
pub(super) fn trace_indices_fit(count: usize, samples: u64) -> bool {
    u64::try_from(count.saturating_sub(1))
        .ok()
        .and_then(|initial| initial.checked_mul(samples))
        .and_then(|base| base.checked_add(samples - 1))
        .is_some()
}

/// The trace index of walk `walk` from initial state `initial`: `initial ×
/// samples + walk` (FR-166). `trace_indices_fit` has held for the run.
pub(super) fn trace_index(
    initial: usize,
    samples: u64,
    walk: u64,
) -> Result<u64, ModelCheckRefusal> {
    u64::try_from(initial)
        .ok()
        .and_then(|initial| initial.checked_mul(samples))
        .and_then(|base| base.checked_add(walk))
        .ok_or_else(|| fault("trace-index-fits"))
}

/// The steps one phase 0 walk takes at most: the lesser of `max_walk_steps`
/// and the horizon (FR-166).
pub(super) fn step_ceiling(limits: &ModelCheckLimits, horizon: Option<usize>) -> usize {
    let budget = usize::try_from(limits.max_walk_steps).unwrap_or(usize::MAX);
    horizon.map_or(budget, |horizon| budget.min(horizon))
}

/// Whether `max_walk_steps`, not the horizon, is the walk's ceiling: only a
/// walk it stopped is counted (FR-166).
pub(super) fn budget_is_ceiling(limits: &ModelCheckLimits, horizon: Option<usize>) -> bool {
    let budget = usize::try_from(limits.max_walk_steps).unwrap_or(usize::MAX);
    horizon.is_none_or(|horizon| budget <= horizon)
}

/// The budgets the run's stops name.
#[derive(Clone, Copy)]
pub(super) struct Budgets {
    pub(super) meter: quire_exact::ScalarLimits,
    pub(super) max_candidates: u64,
}

/// The stop for an expansion or predicate stop: a named budget stops the
/// run naming the budget, a broken invariant refuses, any other stop stops
/// it with no named limit.
pub(super) fn expansion_stop(
    stop: &ExpansionStop,
    budgets: Budgets,
) -> Result<Halt, ModelCheckRefusal> {
    match stop.limit {
        Some(ExpansionLimit::EvaluationMeter(counter)) => Ok(stopped(
            ModelCheckLimit::EvaluationMeter { counter },
            qsl_replay::meter_bound(&budgets.meter, counter),
        )),
        Some(ExpansionLimit::MaxCandidates) => Ok(stopped(
            ModelCheckLimit::MaxCandidates,
            budgets.max_candidates,
        )),
        None if stop.is_internal_failure() => Err(fault("state-graph-expansion-invariant")),
        None => Ok(Halt {
            cause: IncompleteCause::ResourceExhausted,
            limit: None,
        }),
    }
}

/// The stop of a run `cancel` stopped, when it is cancelled.
pub(super) fn cancelled(cancel: &Cancel) -> Option<Halt> {
    if cancel.poll() {
        return cancel.cause().map(|source| Halt {
            cause: IncompleteCause::Cancelled { source },
            limit: None,
        });
    }
    None
}

/// FR-166 to FR-168: run phase 0, explore the subject once and decide each
/// item instance, polling `cancel` before every walk, at every state a walk
/// visits and before every node is labelled and expanded.
///
/// # Errors
///
/// [`ModelCheckRefusal`] when the subject does not admit, a root is
/// unbounded, or an identity cannot be encoded; nothing is decided then.
pub fn check_state_graph(
    request: StateGraphRequest<'_>,
    cancel: &Cancel,
) -> Result<StateGraphRun, ModelCheckRefusal> {
    let seed = request.seed.unwrap_or(DEFAULT_WITNESS_SEED);
    let budgets = Budgets {
        meter: request.subject.simulation.meter,
        max_candidates: request.subject.simulation.expansion.max_candidates,
    };
    let no_initial = request.subject.simulation.initial.is_empty();
    let package = request.subject.simulation.package;
    let system = ModelSystem::new(request.subject.simulation)
        .map_err(|failure| ModelCheckRefusal::Admission(Box::new(failure)))?;
    check_model_bounds(&system, BOUND_POSITIONS).map_err(bounds_refusal)?;
    let views = instances(&request.items, &system)?;
    if views.iter().all(Vec::is_empty) && !request.deadlock_freedom {
        return Ok(StateGraphRun {
            graph: explore::empty(RunEnd::Completed),
            items: views.iter().map(|_| vec![no_instance()]).collect(),
            deadlock: None,
            seed,
        });
    }
    if no_initial {
        return Ok(StateGraphRun {
            graph: explore::empty(RunEnd::Completed),
            items: views
                .iter()
                .map(|item| {
                    if item.is_empty() {
                        return vec![no_instance()];
                    }
                    item.iter()
                        .map(|view| StateGraphInstanceOutcome {
                            over: view.over.clone(),
                            outcome: StateGraphOutcome::NoInitialState,
                            sampled: Vec::new(),
                            walks_stopped: 0,
                        })
                        .collect()
                })
                .collect(),
            deadlock: request
                .deadlock_freedom
                .then_some(ModelCheckOutcome::Undecided {
                    cause: qsl_replay::UndecidedCause::NoInitialState,
                    stats: qsl_replay::RunStats::default(),
                }),
            seed,
        });
    }
    let reductions = request
        .items
        .iter()
        .map(|item| non_preserving(&request.reductions, item.claim.form()))
        .collect::<Vec<_>>();
    if !request.deadlock_freedom
        && reductions.iter().all(Option::is_some)
        && views.iter().all(|instances| !instances.is_empty())
    {
        return Ok(StateGraphRun {
            graph: explore::empty(RunEnd::Completed),
            items: views
                .iter()
                .zip(&request.items)
                .zip(reductions)
                .map(|((instances, item), reduction)| {
                    let reduction = reduction.expect("all populated items reject a reduction");
                    instances
                        .iter()
                        .map(|view| StateGraphInstanceOutcome {
                            over: view.over.clone(),
                            outcome: StateGraphOutcome::ReductionNotPreserving {
                                reduction,
                                form: item.claim.form().into(),
                            },
                            sampled: Vec::new(),
                            walks_stopped: 0,
                        })
                        .collect()
                })
                .collect(),
            deadlock: None,
            seed,
        });
    }
    let declared_symmetry = super::reduce::admit(&system, &request.reductions.symmetry)
        .map_err(ModelCheckRefusal::Symmetry)?;
    if !declared_symmetry.is_trivial() {
        let closed =
            super::reduce::closure(&system, &declared_symmetry).map_err(
                |refusal| match refusal {
                    super::reduce::ClosureRefusal::Encoding(refusal) => {
                        ModelCheckRefusal::Encoding(refusal)
                    }
                    super::reduce::ClosureRefusal::Unordered => {
                        fault("a-permuted-collection-is-ordered")
                    }
                },
            )?;
        if closed.is_some() {
            return Err(fault("state-graph-symmetry-initial-closure"));
        }
    }
    let views = symmetry_instances(views, &declared_symmetry)?;
    let symmetry_group = common_stabiliser(&views, &declared_symmetry);
    let possible: Vec<&InstanceView<'_>> = views
        .iter()
        .zip(&request.items)
        .filter(|(_, item)| matches!(item.claim.form(), StateGraphForm::Possible { .. }))
        .flat_map(|(item, _)| item.iter())
        .collect();
    let phase_zero = sample::Phase {
        system: &system,
        limits: &request.limits,
        horizon: request.max_depth,
        seed,
        cancel,
        budgets,
    };
    let sampled = match phase_zero.run(&possible)? {
        Ok(sampled) => sampled,
        Err(halt) => {
            return Ok(stopped_before_exploring(
                &views,
                request.deadlock_freedom,
                halt,
                seed,
            ));
        }
    };
    let por_instances: Vec<ModelInstance> = views
        .iter()
        .flatten()
        .filter_map(|view| view.model_instance.clone())
        .collect();
    let por = if request.reductions.partial_order {
        let mut reads = BTreeSet::<SymbolicLocation>::new();
        for item in &request.items {
            let Some(over) = item.claim.over() else {
                continue;
            };
            let parameter = (over.name.clone(), ValueType::Reference(over.object));
            for (_, predicate) in predicates(item.claim.form()) {
                let CheckedStatePredicate::Checked(body) = predicate else {
                    continue;
                };
                reads.extend(
                    system
                        .atom_reads(&parameter, body.body())
                        .map_err(|refusal| ModelCheckRefusal::Footprint(refusal.into()))?,
                );
            }
        }
        Some(super::ample::Por::with_atom_reads(
            &system,
            reads,
            &por_instances,
            &[],
            None,
        )?)
    } else {
        None
    };
    let explored = explore::Exploration {
        system: &system,
        package,
        limits: &request.limits,
        horizon: request.max_depth,
        cancel,
        budgets,
        views: &views,
        deadlock_freedom: request.deadlock_freedom,
        por: por.as_ref(),
        declared_symmetry: &declared_symmetry,
        symmetry: &symmetry_group,
    }
    .run()?;
    let items = views
        .iter()
        .zip(&request.items)
        .enumerate()
        .map(|(index, (item, entry))| {
            if item.is_empty() {
                return Ok(vec![no_instance()]);
            }
            if let Some(reduction) = reductions[index] {
                return Ok(item
                    .iter()
                    .map(|view| StateGraphInstanceOutcome {
                        over: view.over.clone(),
                        outcome: StateGraphOutcome::ReductionNotPreserving {
                            reduction,
                            form: entry.claim.form().into(),
                        },
                        sampled: Vec::new(),
                        walks_stopped: 0,
                    })
                    .collect());
            }
            item.iter()
                .map(|view| {
                    let outcome = decide::decide(
                        &explored.graph,
                        entry.claim.form(),
                        view,
                        sampled.witnesses(view),
                    )?;
                    let outcome = if explored.reduced {
                        let reductions = applied_reductions(
                            explored.partial_order_reduced,
                            &declared_symmetry,
                            &symmetry_group,
                            &view.orbit,
                        );
                        match outcome {
                            StateGraphOutcome::Holds { .. } => StateGraphOutcome::HoldsReduced {
                                reductions,
                                witnesses: None,
                            },
                            StateGraphOutcome::Witnessed { paths, .. } => {
                                StateGraphOutcome::HoldsReduced {
                                    reductions,
                                    witnesses: Some(paths),
                                }
                            }
                            other => other,
                        }
                    } else {
                        outcome
                    };
                    Ok(StateGraphInstanceOutcome {
                        over: view.over.clone(),
                        outcome,
                        sampled: sampled.witnesses(view).to_vec(),
                        walks_stopped: sampled.stopped(view),
                    })
                })
                .collect::<Result<Vec<_>, ModelCheckRefusal>>()
        })
        .collect::<Result<Vec<_>, ModelCheckRefusal>>()?;
    Ok(StateGraphRun {
        graph: explored.graph,
        items,
        deadlock: explored.deadlock,
        seed,
    })
}

pub(super) fn applied_reductions(
    partial_order: bool,
    declared: &SymmetryGroup,
    applied: &SymmetryGroup,
    orbit: &[String],
) -> Vec<AppliedReduction> {
    let mut reductions = Vec::new();
    if !applied.is_trivial() {
        reductions.push(AppliedReduction::Symmetry {
            groups: declared.classes.clone(),
            orbit: orbit.to_vec(),
            stabiliser: reported_stabiliser(declared, applied),
        });
    }
    if partial_order {
        reductions.push(AppliedReduction::PartialOrder {
            proviso: Proviso::BreadthFirstRevisit,
        });
    }
    reductions
}

/// Preserve fixed keys as singleton blocks in the reported stabilizer
/// partition, while the execution group contains only non-trivial blocks.
fn reported_stabiliser(
    declared: &SymmetryGroup,
    applied: &SymmetryGroup,
) -> Vec<qsl_eval::simulation::SymmetryClass> {
    let mut result = Vec::new();
    for group in &declared.classes {
        let start = result.len();
        let moving = applied
            .classes
            .iter()
            .find(|class| class.population == group.population)
            .map(|class| class.keys.as_slice())
            .unwrap_or_default();
        for key in group.keys.iter().filter(|key| !moving.contains(key)) {
            result.push(qsl_eval::simulation::SymmetryClass {
                population: group.population.clone(),
                keys: vec![key.clone()],
            });
        }
        if !moving.is_empty() {
            result.push(qsl_eval::simulation::SymmetryClass {
                population: group.population.clone(),
                keys: moving.to_vec(),
            });
        }
        result[start..].sort_by(|left, right| left.keys[0].cmp(&right.keys[0]));
    }
    result
}

/// ADR-022 GR-1: selected state-space reductions are rejected by the form's
/// preservation row before any expansion.
fn non_preserving(
    reductions: &Reductions,
    form: &StateGraphForm,
) -> Option<qsl_foundation::ReductionKind> {
    reductions
        .selected()
        .into_iter()
        .find(|reduction| match reduction {
            qsl_foundation::ReductionKind::Symmetry => {
                matches!(form, StateGraphForm::UniquePath { .. })
            }
            qsl_foundation::ReductionKind::PartialOrder => {
                !matches!(form, StateGraphForm::Possible { .. })
            }
            qsl_foundation::ReductionKind::StateConstraint => false,
        })
}

/// A state-graph run and the FR-169 settlement record for each claim instance.
#[derive(Debug)]
pub struct SettledStateGraphRun {
    /// The explored graph and per-instance evidence produced by FR-168.
    pub run: StateGraphRun,
    /// FR-169 records in item and instance order, aligned with `run.items`.
    pub records: Vec<Vec<StateGraphRecord>>,
}

/// Why the combined state-graph check and settlement could not run.
#[derive(Debug, thiserror::Error)]
pub enum StateGraphPipelineRefusal {
    /// The engine refused before producing a run.
    #[error(transparent)]
    Engine(#[from] ModelCheckRefusal),
    /// The request horizon cannot be represented in FR-169's `u64` record.
    #[error("the state-graph search horizon is outside the terminal record range")]
    HorizonOutOfRange,
}

/// Run FR-166–FR-168 and settle every state-graph item instance under FR-169.
///
/// `replay` is called only for evidence rows whose verdict requires replay;
/// certificate checks remain in the core settlement map.
/// The optional deadlock-freedom result remains on `run` for its own FR-127
/// settlement path.
pub fn check_and_settle_state_graph(
    request: StateGraphRequest<'_>,
    cancel: &Cancel,
    mut replay: impl FnMut(&StateGraphInstanceOutcome) -> Result<GraphReplayResult, ReplayRefusal>,
) -> Result<SettledStateGraphRun, StateGraphPipelineRefusal> {
    let max_depth = request
        .max_depth
        .map(u64::try_from)
        .transpose()
        .map_err(|_| StateGraphPipelineRefusal::HorizonOutOfRange)?;
    let claims = request
        .items
        .iter()
        .map(|item| item.claim)
        .collect::<Vec<_>>();
    let simulation = request.subject.simulation.clone();
    let proof_bounds = request.subject.proof_bounds.clone();
    let limits = request.limits;
    let run = check_state_graph(request, cancel)?;
    let subject = ModelSubject {
        simulation,
        proof_bounds,
    };
    let method = StateGraphMethod {
        limits,
        max_depth,
        seed: run.seed,
    };
    let records = claims
        .into_iter()
        .zip(&run.items)
        .map(|(claim, instances)| {
            instances
                .iter()
                .map(|instance| {
                    settle_state_graph(instance, &method, &subject, claim, cancel, |_| {
                        replay(instance)
                    })
                })
                .collect()
        })
        .collect();
    Ok(SettledStateGraphRun { run, records })
}

/// The run phase 0 stopped before any exploration: every instance and the
/// deadlock-freedom item decide nothing.
fn stopped_before_exploring(
    views: &[Vec<InstanceView<'_>>],
    deadlock_freedom: bool,
    halt: Halt,
    seed: u64,
) -> StateGraphRun {
    StateGraphRun {
        items: views
            .iter()
            .map(|item| {
                if item.is_empty() {
                    return vec![no_instance()];
                }
                item.iter()
                    .map(|view| StateGraphInstanceOutcome {
                        over: view.over.clone(),
                        outcome: StateGraphOutcome::NoDecision {
                            end: halt.end(),
                            open: std::collections::BTreeSet::new(),
                        },
                        sampled: Vec::new(),
                        walks_stopped: 0,
                    })
                    .collect()
            })
            .collect(),
        graph: explore::empty(halt.end()),
        deadlock: deadlock_freedom.then_some(ModelCheckOutcome::Stopped {
            cause: halt.cause,
            limit: halt.limit,
            stats: qsl_replay::RunStats::default(),
        }),
        seed,
    }
}

/// An item-level FR-125 outcome when no universe key instantiates its claim.
fn no_instance() -> StateGraphInstanceOutcome {
    StateGraphInstanceOutcome {
        over: None,
        outcome: StateGraphOutcome::NoInstance,
        sampled: Vec::new(),
        walks_stopped: 0,
    }
}
