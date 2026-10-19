// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-167 (ADR-022 GE-1, GM-6): phase 1, one canonical breadth-first
//! exploration of the subject for every state-graph item.
//!
//! Initial states are taken in ascending state-key order, equal keys
//! coalescing into one node, and successors in transition-identity bytes,
//! then post-state key bytes, order (FR-101). A node is labelled when it is
//! taken from the frontier, then expanded. Every edge is retained. A node is
//! open when the run did not learn all of its outgoing edges:
//!
//! - `MaxDepth`: it is at the horizon and has a successor, which the run does
//!   not follow;
//! - `UndecidedSuccessor`: its expansion evaluated a contract conjunction
//!   undecided, or a claim predicate read refused at it; the successors the
//!   expansion did return are retained and explored;
//! - `NotExpanded`: it was discovered and not expanded, or partly expanded,
//!   when a limit or a cancellation stopped the run.
//!
//! The deadlock-freedom item is decided on the same exploration, as FR-126
//! decides it: the first node in discovery order whose expansion evaluates a
//! contract conjunction undefined, or that is deadlocked, is its violation,
//! with the canonical breadth-first path to it.

use std::collections::{BTreeMap, VecDeque};

use qsl_eval::automaton::{SafetyState, SafetyTranslation};
use qsl_eval::simulation::model::{
    ModelFinding, ModelSimState, ModelSystem, PredicateReading, TerminalClass,
};
use qsl_eval::simulation::TransitionSystem;
use qsl_replay::{
    ClosureCertificate, CounterexampleKind, ItemRefusal, ModelCheckLimit, ModelCheckLimits,
    ModelCheckOutcome, ModelStep, OpenCause, ProductStateRef, RunEnd, RunStats,
    TemporalCounterexample, UndecidedCause, UndefinedEvaluation, UndefinedLocus, UndefinedSite,
};
use quire_exact::Cancel;

use super::{
    cancelled, expansion_stop, stopped, Budgets, ExploredStateGraph, GraphEdge, GraphNode,
    GraphStats, Halt, InstanceView, NodeIndex, PredicateLabel,
};
use crate::model_check::check::{fault, widen};
use crate::model_check::ModelCheckRefusal;

/// The explored graph of a run that explored nothing.
pub(super) fn empty(end: RunEnd) -> ExploredStateGraph {
    ExploredStateGraph {
        nodes: Vec::new(),
        edges: Vec::new(),
        open: BTreeMap::new(),
        end,
        stats: GraphStats::default(),
        outgoing: Vec::new(),
        parent: Vec::new(),
        root: Vec::new(),
        initial: Vec::new(),
    }
}

/// What the deadlock-freedom item found at the first node that decided it.
enum DeadlockEvent {
    Violated(CounterexampleKind, usize),
    Undecided,
}

/// Phase 1's result: the graph and the deadlock-freedom item's outcome.
pub(super) struct Explored {
    pub(super) graph: ExploredStateGraph,
    pub(super) deadlock: Option<ModelCheckOutcome>,
}

/// Phase 1 of one run.
pub(super) struct Exploration<'r, 'a, 'v> {
    pub(super) system: &'r ModelSystem<'a>,
    pub(super) limits: &'r ModelCheckLimits,
    pub(super) horizon: Option<usize>,
    pub(super) cancel: &'r Cancel,
    pub(super) budgets: Budgets,
    pub(super) views: &'r [Vec<InstanceView<'v>>],
    pub(super) deadlock_freedom: bool,
}

/// The run's working state.
struct Walk {
    graph: ExploredStateGraph,
    states: Vec<ModelSimState>,
    queue: VecDeque<usize>,
    deadlock: Option<DeadlockEvent>,
    horizon_cut: bool,
}

impl Walk {
    /// Mark `at` and every queued node `NotExpanded`.
    fn halt(&mut self, at: usize) {
        for node in std::iter::once(at).chain(self.queue.iter().copied()) {
            self.graph
                .open
                .insert(NodeIndex(node), OpenCause::NotExpanded);
        }
    }

    fn insert(
        &mut self,
        state: ModelSimState,
        digest: qsl_foundation::digest::DigestRecord,
        depth: usize,
        parent: Option<usize>,
        root: usize,
    ) -> usize {
        let node = self.graph.nodes.len();
        self.graph.nodes.push(GraphNode {
            key: digest,
            depth: widen(depth),
            labels: Vec::new(),
        });
        self.graph.outgoing.push(Vec::new());
        self.graph.parent.push(parent);
        self.graph.root.push(root);
        self.states.push(state);
        self.queue.push_back(node);
        node
    }
}

impl Exploration<'_, '_, '_> {
    /// Explore the subject.
    pub(super) fn run(&self) -> Result<Explored, ModelCheckRefusal> {
        let mut walk = Walk {
            graph: empty(RunEnd::Completed),
            states: Vec::new(),
            queue: VecDeque::new(),
            deadlock: None,
            horizon_cut: false,
        };
        let mut initial = Vec::new();
        for (index, state) in self.system.initial().into_iter().enumerate() {
            let (key, digest) = self.system.state_key(&state)?;
            initial.push((key, index, digest, state));
        }
        let count = initial.len();
        initial.sort_by(|left, right| left.0.cmp(&right.0).then(left.1.cmp(&right.1)));
        let mut known = BTreeMap::new();
        let mut of_initial = vec![None; count];
        let distinct = {
            let mut keys: Vec<&Vec<u8>> = initial.iter().map(|entry| &entry.0).collect();
            keys.dedup();
            keys.len()
        };
        if distinct > self.limits.limits.max_states {
            walk.graph.end = stopped(
                ModelCheckLimit::MaxStates,
                widen(self.limits.limits.max_states),
            )
            .end();
            return self.finish(walk);
        }
        for (key, index, digest, state) in initial {
            // Equal keys coalesce into the first initial state that has the key.
            let node = match known.get(&key) {
                Some(node) => *node,
                None => {
                    let node = walk.insert(state, digest, 0, None, index);
                    known.insert(key, node);
                    node
                }
            };
            if let Some(slot) = of_initial.get_mut(index) {
                *slot = Some(NodeIndex(node));
            }
        }
        walk.graph.initial = of_initial
            .into_iter()
            .collect::<Option<Vec<_>>>()
            .ok_or_else(|| fault("every-initial-state-has-a-node"))?;
        while let Some(at) = walk.queue.pop_front() {
            match self.visit(&mut walk, at, &mut known)? {
                Ok(()) => {}
                Err(halt) => {
                    walk.halt(at);
                    walk.graph.end = halt.end();
                    break;
                }
            }
        }
        self.finish(walk)
    }

    /// Label and expand node `at`.
    fn visit(
        &self,
        walk: &mut Walk,
        at: usize,
        known: &mut BTreeMap<Vec<u8>, usize>,
    ) -> Result<Result<(), Halt>, ModelCheckRefusal> {
        if let Some(halt) = cancelled(self.cancel) {
            return Ok(Err(halt));
        }
        let mut keys = Vec::new();
        let mut predicates = Vec::new();
        for view in self.views.iter().flatten() {
            for (role, body) in &view.predicates {
                if let Some(body) = body {
                    keys.push((view.item, view.instance, *role));
                    predicates.push((*body, view.over.as_ref()));
                }
            }
        }
        let readings = match self.system.state_predicates(&walk.states[at], &predicates) {
            Ok(readings) => readings,
            Err(stop) => return expansion_stop(&stop, self.budgets).map(Err),
        };
        let refused = readings
            .iter()
            .any(|reading| matches!(reading, PredicateReading::Refused(_)));
        let labels = keys
            .into_iter()
            .zip(readings)
            .map(|((item, instance, role), reading)| PredicateLabel {
                item,
                instance,
                role,
                reading,
            })
            .collect();
        walk.graph.nodes[at].labels = labels;
        let expansion = match self.system.successors(&walk.states[at]) {
            Ok(expansion) => expansion,
            Err(stop) => return expansion_stop(&stop, self.budgets).map(Err),
        };
        let undefined = expansion
            .findings
            .iter()
            .find_map(ModelFinding::undefined_contract);
        let undecided = expansion
            .findings
            .iter()
            .any(ModelFinding::is_refused_contract);
        if refused || undefined.is_some() || undecided {
            walk.graph
                .open
                .insert(NodeIndex(at), OpenCause::UndecidedSuccessor);
        }
        if self.deadlock_freedom && walk.deadlock.is_none() {
            match self.deadlock_event(
                walk,
                at,
                undefined,
                undecided,
                !expansion.successors.is_empty(),
            )? {
                Ok(event) => walk.deadlock = event,
                Err(halt) => return Ok(Err(halt)),
            }
        }
        let depth = self.depth_of(walk, at)?;
        if self.horizon.is_some_and(|horizon| depth >= horizon) {
            if !expansion.successors.is_empty() {
                walk.horizon_cut = true;
                walk.graph
                    .open
                    .entry(NodeIndex(at))
                    .or_insert(OpenCause::MaxDepth);
            }
            return Ok(Ok(()));
        }
        let mut successors = Vec::with_capacity(expansion.successors.len());
        for (transition, state) in expansion.successors {
            let order = self.system.transition_key(&transition)?;
            let (key, digest) = self.system.state_key(&state)?;
            successors.push((order, key, digest, transition, state));
        }
        successors.sort_by(|left, right| left.0.cmp(&right.0).then(left.1.cmp(&right.1)));
        let root = walk.graph.root[at];
        for (_, key, digest, transition, state) in successors {
            if walk.graph.edges.len() >= self.limits.limits.max_transitions {
                return Ok(Err(stopped(
                    ModelCheckLimit::MaxTransitions,
                    widen(self.limits.limits.max_transitions),
                )));
            }
            let edge = walk.graph.edges.len();
            let target = match known.get(&key) {
                Some(node) => *node,
                None => {
                    if walk.graph.nodes.len() >= self.limits.limits.max_states {
                        return Ok(Err(stopped(
                            ModelCheckLimit::MaxStates,
                            widen(self.limits.limits.max_states),
                        )));
                    }
                    let node =
                        walk.insert(state, digest, depth.saturating_add(1), Some(edge), root);
                    known.insert(key, node);
                    node
                }
            };
            walk.graph.edges.push(GraphEdge {
                source: NodeIndex(at),
                transition,
                target: NodeIndex(target),
            });
            walk.graph.outgoing[at].push(edge);
        }
        Ok(Ok(()))
    }

    fn depth_of(&self, walk: &Walk, at: usize) -> Result<usize, ModelCheckRefusal> {
        usize::try_from(walk.graph.nodes[at].depth).map_err(|_| fault("node-depth-fits-usize"))
    }

    /// The deadlock-freedom item's event at node `at`, if `at` decides it:
    /// an undefined contract conjunction, then a refused one, then the
    /// state's FR-124 class (ADR-018 DL-7, UE-6, V-6).
    fn deadlock_event(
        &self,
        walk: &Walk,
        at: usize,
        undefined: Option<qsl_eval::simulation::model::UndefinedContract>,
        undecided: bool,
        has_successor: bool,
    ) -> Result<Result<Option<DeadlockEvent>, Halt>, ModelCheckRefusal> {
        let site = |locus, transition| UndefinedSite {
            position: walk.graph.nodes[at].depth,
            state_key: walk.graph.nodes[at].key,
            locus,
            transition,
        };
        if let Some(contract) = undefined {
            return Ok(Ok(Some(DeadlockEvent::Violated(
                CounterexampleKind::UndefinedEvaluation(UndefinedEvaluation {
                    site: site(UndefinedLocus::Contract(contract.clause), contract.guard_of),
                    cause: contract.cause,
                }),
                at,
            ))));
        }
        if undecided {
            return Ok(Ok(Some(DeadlockEvent::Undecided)));
        }
        let class = match self.system.terminal_class(&walk.states[at], has_successor) {
            Ok(class) => class,
            Err(stop) => return expansion_stop(&stop, self.budgets).map(Err),
        };
        Ok(Ok(match class {
            TerminalClass::NotTerminal | TerminalClass::Intended => None,
            TerminalClass::Deadlocked => {
                Some(DeadlockEvent::Violated(CounterexampleKind::Deadlock, at))
            }
            TerminalClass::Undefined { model, cause } => Some(DeadlockEvent::Violated(
                CounterexampleKind::UndefinedEvaluation(UndefinedEvaluation {
                    site: site(UndefinedLocus::Terminal(model), None),
                    cause,
                }),
                at,
            )),
            TerminalClass::Undecided { .. } => Some(DeadlockEvent::Undecided),
        }))
    }

    /// The graph's statistics and the deadlock-freedom item's outcome.
    fn finish(&self, mut walk: Walk) -> Result<Explored, ModelCheckRefusal> {
        walk.graph.stats = GraphStats {
            nodes: widen(walk.graph.nodes.len()),
            edges: widen(walk.graph.edges.len()),
            depth: walk
                .graph
                .nodes
                .iter()
                .map(|node| node.depth)
                .max()
                .unwrap_or(0),
        };
        let deadlock = if self.deadlock_freedom {
            Some(self.deadlock(&walk)?)
        } else {
            None
        };
        Ok(Explored {
            graph: walk.graph,
            deadlock,
        })
    }

    /// The deadlock-freedom item's outcome, as FR-126 decides it.
    fn deadlock(&self, walk: &Walk) -> Result<ModelCheckOutcome, ModelCheckRefusal> {
        let graph = &walk.graph;
        let stats = RunStats {
            product_states: graph.stats.nodes,
            product_edges: graph.stats.edges,
            automaton_states: 0,
            depth: graph.stats.depth,
        };
        Ok(match &walk.deadlock {
            Some(DeadlockEvent::Violated(kind, at)) => ModelCheckOutcome::Violated {
                counterexample: Box::new(TemporalCounterexample {
                    initial_state: graph.root[*at],
                    prefix: graph.path_to(NodeIndex(*at)),
                    over_binding: None,
                    fairness: Vec::new(),
                    kind: kind.clone(),
                }),
                stats,
            },
            Some(DeadlockEvent::Undecided) => ModelCheckOutcome::Undecided {
                cause: UndecidedCause::UndecidedSuccessor,
                stats,
            },
            None => match &graph.end {
                RunEnd::Stopped { cause, limit } => ModelCheckOutcome::Stopped {
                    cause: *cause,
                    limit: *limit,
                    stats,
                },
                RunEnd::Completed if walk.horizon_cut => ModelCheckOutcome::BoundReached {
                    depth: widen(self.horizon.unwrap_or(0)),
                    stats,
                },
                RunEnd::Completed => ModelCheckOutcome::Holds {
                    certificate: deadlock_certificate(graph)?,
                    stats,
                },
            },
        })
    }
}

/// FR-126's closure certificate of a deadlock-freedom proof: every explored
/// model state under the item's carried root, sorted.
fn deadlock_certificate(
    graph: &ExploredStateGraph,
) -> Result<ClosureCertificate, ModelCheckRefusal> {
    let key = SafetyTranslation::deadlock_freedom()
        .map_err(|_| ModelCheckRefusal::Item(ItemRefusal::DeadlockContext))?
        .key(SafetyState::Carried)?;
    let mut states: Vec<ProductStateRef> = graph
        .nodes
        .iter()
        .map(|node| ProductStateRef {
            model_state: node.key,
            automaton_state: key.clone(),
        })
        .collect();
    states.sort();
    states.dedup();
    Ok(ClosureCertificate { states })
}

impl ExploredStateGraph {
    /// The canonical breadth-first path's steps to `node`.
    pub(super) fn path_to(&self, node: NodeIndex) -> Vec<ModelStep> {
        let mut steps = Vec::new();
        let mut cursor = node.0;
        while let Some(edge) = self.parent.get(cursor).copied().flatten() {
            let Some(edge) = self.edges.get(edge) else {
                break;
            };
            steps.push(ModelStep {
                transition: edge.transition.clone(),
                post_state: self.nodes[cursor].key,
            });
            cursor = edge.source.0;
        }
        steps.reverse();
        steps
    }

    /// The initial-state index of `node`'s breadth-first tree root.
    pub(super) fn root_of(&self, node: NodeIndex) -> usize {
        self.root.get(node.0).copied().unwrap_or(0)
    }
}
