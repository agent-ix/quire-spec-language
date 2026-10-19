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

use std::collections::BTreeMap;

use qsl_eval::model_subject::ModelSubject;
use qsl_eval::simulation::model::{check_model_bounds, ModelSystem, PredicateReading};
use qsl_eval::simulation::{ExpansionLimit, ExpansionStop, NotSimulated};
use qsl_foundation::digest::DigestRecord;
use qsl_replay::{
    IncompleteCause, ModelCheckLimit, ModelCheckLimits, ModelCheckOutcome, OpenCause, ReachedLimit,
    RunEnd, StateGraphInstanceOutcome, StateGraphOutcome, DEFAULT_WITNESS_SEED,
};
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
}

/// One EN-1 run over a model subject's state-graph items (FR-166, FR-167).
pub struct StateGraphRequest<'a> {
    /// The model subject.
    pub subject: ModelSubject<'a>,
    /// The subject's state-graph items.
    pub items: Vec<StateGraphItem<'a>>,
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
    /// The explored graph; empty when the run stopped before exploring.
    pub graph: ExploredStateGraph,
    /// Each item's decision, per instance, in request order. An item with no
    /// instance has none (FR-125 `NoInstance`).
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
            let bindings: Vec<Option<ObjectReference>> = match entry.claim.over() {
                None => vec![None],
                Some(over) => system
                    .instances(&ValueType::Reference(over.object))
                    .map_err(bounds_refusal)?
                    .iter()
                    .map(|instance| {
                        system
                            .bound_reference(over.object, instance)
                            .map(Some)
                            .map_err(|_| fault("state-graph-instance-references"))
                    })
                    .collect::<Result<_, _>>()?,
            };
            Ok(bindings
                .into_iter()
                .enumerate()
                .map(|(instance, over)| InstanceView {
                    item,
                    instance,
                    over,
                    predicates: bodies.clone(),
                })
                .collect())
        })
        .collect()
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
struct Halt {
    cause: IncompleteCause,
    limit: Option<ReachedLimit>,
}

impl Halt {
    fn end(self) -> RunEnd {
        RunEnd::Stopped {
            cause: self.cause,
            limit: self.limit,
        }
    }
}

/// The stop at the limit `limit`, set to `value`.
fn stopped(limit: ModelCheckLimit, value: u64) -> Halt {
    Halt {
        cause: IncompleteCause::ResourceExhausted,
        limit: Some(ReachedLimit { limit, value }),
    }
}

/// The budgets the run's stops name.
#[derive(Clone, Copy)]
struct Budgets {
    meter: u64,
    max_candidates: u64,
}

/// The stop for an expansion or predicate stop: a named budget stops the
/// run naming the budget, a broken invariant refuses, any other stop stops
/// it with no named limit.
fn expansion_stop(stop: &ExpansionStop, budgets: Budgets) -> Result<Halt, ModelCheckRefusal> {
    match stop.limit {
        Some(ExpansionLimit::EvaluationMeter) => {
            Ok(stopped(ModelCheckLimit::EvaluationMeter, budgets.meter))
        }
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
fn cancelled(cancel: &Cancel) -> Option<Halt> {
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
        meter: request.subject.simulation.meter.work_units,
        max_candidates: request.subject.simulation.expansion.max_candidates,
    };
    let no_initial = request.subject.simulation.initial.is_empty();
    let system = ModelSystem::new(request.subject.simulation)
        .map_err(|failure| ModelCheckRefusal::Admission(Box::new(failure)))?;
    check_model_bounds(&system, BOUND_POSITIONS).map_err(bounds_refusal)?;
    let views = instances(&request.items, &system)?;
    if no_initial {
        return Ok(StateGraphRun {
            graph: explore::empty(RunEnd::Completed),
            items: views
                .iter()
                .map(|item| {
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
    let explored = explore::Exploration {
        system: &system,
        limits: &request.limits,
        horizon: request.max_depth,
        cancel,
        budgets,
        views: &views,
        deadlock_freedom: request.deadlock_freedom,
    }
    .run()?;
    let items = views
        .iter()
        .zip(&request.items)
        .map(|(item, entry)| {
            item.iter()
                .map(|view| {
                    let outcome = decide::decide(
                        &explored.graph,
                        entry.claim.form(),
                        view,
                        sampled.witnesses(view),
                    )?;
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
