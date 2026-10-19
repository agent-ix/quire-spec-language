// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-168 (ADR-022 §3 "Phase 1 algorithms", GV-1 to GV-7): one pass over the
//! explored graph per item instance.
//!
//! - `possible` and `always possible`: a backward breadth-first search from
//!   every target node and every open node gives `E⁺`; a closed node outside
//!   it is a trap. For `possible`, a second search from the target nodes
//!   alone, through closed nodes, gives each node its distance `d_P`; an
//!   explored witness descends it by the first edge in canonical order.
//! - `unique path`: `H` is the retained graph restricted to the nodes that
//!   reach a `to` node or an open node, with every edge out of a `to` node
//!   removed; paths to `to` are counted per node in reverse topological order
//!   of `H`'s condensation, saturating at 2.
//!
//! A node where a predicate of the instance read refused is open for that
//! instance alone, `UndecidedSuccessor`: its truth there is not known, and
//! no other instance's reading depends on it.
//!
//! Refuting evidence (a trap, a path pair, an undefined evaluation) counts
//! whatever else is open; between a trap or path pair and an undefined
//! evaluation the earlier node in discovery order decides, and a node that is
//! both is undefined. `Witnessed` and `Holds` need a run that completed with
//! no open node.

use std::collections::{BTreeSet, VecDeque};

use qsl_eval::simulation::model::PredicateReading;
use qsl_foundation::digest::DigestRecord;
use qsl_replay::{
    GraphUndefined, GraphWitness, ModelPath, ModelStep, OpenCause, RunEnd, StateGraphCertificate,
    StateGraphOutcome, WitnessSource,
};
use qsl_semantics::check::{PredicateRole, StateGraphForm};

use super::{ExploredStateGraph, InstanceView, NodeIndex};
use crate::model_check::check::fault;
use crate::model_check::ModelCheckRefusal;

/// Decide `form` for `view` over `graph`, with `sampled`, phase 0's witness
/// per initial state (empty when phase 0 drew none for the instance).
pub(super) fn decide(
    graph: &ExploredStateGraph,
    form: &StateGraphForm,
    view: &InstanceView<'_>,
    sampled: &[Option<GraphWitness>],
) -> Result<StateGraphOutcome, ModelCheckRefusal> {
    let reader = Reader::new(graph, view);
    match form {
        StateGraphForm::Possible { .. } => reader.possible(sampled),
        StateGraphForm::AlwaysPossible { .. } => reader.always_possible(),
        StateGraphForm::UniquePath { .. } => reader.unique_path(),
    }
}

/// The node with no distance.
const FAR: usize = usize::MAX;

/// One instance's reading of the graph.
struct Reader<'g> {
    graph: &'g ExploredStateGraph,
    view: &'g InstanceView<'g>,
    /// Each node's retained predecessors, one entry per edge.
    incoming: Vec<Vec<usize>>,
    /// Whether a predicate of the instance read refused at each node.
    refused: Vec<bool>,
}

impl<'g> Reader<'g> {
    fn new(graph: &'g ExploredStateGraph, view: &'g InstanceView<'g>) -> Self {
        let mut incoming = vec![Vec::new(); graph.nodes.len()];
        for edge in &graph.edges {
            if let Some(sources) = incoming.get_mut(edge.target.0) {
                sources.push(edge.source.0);
            }
        }
        let refused = graph
            .nodes
            .iter()
            .map(|node| {
                node.labels.iter().any(|label| {
                    label.item == view.item
                        && label.instance == view.instance
                        && matches!(label.reading, PredicateReading::Refused(_))
                })
            })
            .collect();
        Self {
            graph,
            view,
            incoming,
            refused,
        }
    }

    fn count(&self) -> usize {
        self.graph.nodes.len()
    }

    /// Whether `node` is open for the instance: open in the graph, or a
    /// predicate of the instance read refused there.
    fn is_open(&self, node: usize) -> bool {
        self.refused[node] || self.graph.open.contains_key(&NodeIndex(node))
    }

    /// Whether `role` holds at `node`: `None` when it is undefined, refused
    /// or unread there. An omitted `from` holds everywhere.
    fn holds(&self, node: usize, role: PredicateRole) -> Option<bool> {
        let written = self
            .view
            .predicates
            .iter()
            .any(|(predicate, body)| *predicate == role && body.is_some());
        if !written {
            return Some(true);
        }
        match self
            .graph
            .reading(NodeIndex(node), self.view.item, self.view.instance, role)?
        {
            PredicateReading::True => Some(true),
            PredicateReading::False => Some(false),
            PredicateReading::Undefined { .. } | PredicateReading::Refused(_) => None,
        }
    }

    /// The first node in discovery order where a predicate of the claim is
    /// undefined, with where and why.
    fn first_undefined(&self) -> Option<(usize, GraphUndefined)> {
        (0..self.count()).find_map(|node| {
            self.graph.nodes[node].labels.iter().find_map(|label| {
                if label.item != self.view.item || label.instance != self.view.instance {
                    return None;
                }
                let PredicateReading::Undefined { cause, location } = &label.reading else {
                    return None;
                };
                Some((
                    node,
                    GraphUndefined {
                        predicate: label.role,
                        state_key: self.graph.nodes[node].key,
                        locus: location.clone(),
                        cause: cause.clone(),
                    },
                ))
            })
        })
    }

    fn nodes_where(&self, role: PredicateRole) -> Vec<usize> {
        (0..self.count())
            .filter(|node| self.holds(*node, role) == Some(true))
            .collect()
    }

    /// The breadth-first distance from every node to the nearest of
    /// `sources`, backwards over the retained edges whose source `admit`
    /// accepts.
    fn backward(&self, sources: &[usize], admit: impl Fn(usize) -> bool) -> Vec<usize> {
        let mut distance = vec![FAR; self.count()];
        let mut queue = VecDeque::new();
        for source in sources {
            if distance[*source] == FAR {
                distance[*source] = 0;
                queue.push_back(*source);
            }
        }
        while let Some(node) = queue.pop_front() {
            for &predecessor in &self.incoming[node] {
                if distance[predecessor] == FAR && admit(predecessor) {
                    distance[predecessor] = distance[node].saturating_add(1);
                    queue.push_back(predecessor);
                }
            }
        }
        distance
    }

    fn open_nodes(&self) -> Vec<usize> {
        (0..self.count())
            .filter(|node| self.is_open(*node))
            .collect()
    }

    fn open_causes(&self) -> BTreeSet<OpenCause> {
        let mut causes: BTreeSet<OpenCause> = self.graph.open.values().copied().collect();
        if self.refused.contains(&true) {
            causes.insert(OpenCause::UndecidedSuccessor);
        }
        causes
    }

    fn complete(&self) -> bool {
        self.graph.end == RunEnd::Completed
            && self.graph.open.is_empty()
            && !self.refused.contains(&true)
    }

    fn no_decision(&self) -> StateGraphOutcome {
        StateGraphOutcome::NoDecision {
            end: self.graph.end.clone(),
            open: self.open_causes(),
        }
    }

    fn step(&self, edge: usize) -> ModelStep {
        let edge = &self.graph.edges[edge];
        ModelStep {
            transition: edge.transition.clone(),
            post_state: self.graph.nodes[edge.target.0].key,
        }
    }

    /// The canonical breadth-first path to `node`.
    fn stem(&self, node: usize) -> Result<ModelPath, ModelCheckRefusal> {
        Ok(ModelPath {
            initial: initial_index(self.graph.root_of(NodeIndex(node)))?,
            steps: self.graph.path_to(NodeIndex(node)),
        })
    }

    fn undefined(
        &self,
        node: usize,
        undefined: GraphUndefined,
    ) -> Result<StateGraphOutcome, ModelCheckRefusal> {
        Ok(StateGraphOutcome::Undefined {
            stem: self.stem(node)?,
            undefined,
        })
    }

    /// `possible`: a witness for every initial state, else a trap initial
    /// state, else an undefined evaluation.
    fn possible(
        &self,
        sampled: &[Option<GraphWitness>],
    ) -> Result<StateGraphOutcome, ModelCheckRefusal> {
        // A subject with no initial state never reaches the decision, so a
        // graph with none is a run that stopped before inserting them: it
        // has no node to hold a witness for each initial state.
        if self.graph.initial.is_empty() {
            return Ok(self.no_decision());
        }
        let targets = self.nodes_where(PredicateRole::Target);
        let mut reaching = targets.clone();
        reaching.extend(self.open_nodes());
        let e_plus = self.backward(&reaching, |_| true);
        let d_p = self.backward(&targets, |node| !self.is_open(node));
        let undefined = self.first_undefined();
        let mut witnesses = Vec::with_capacity(self.graph.initial.len());
        let mut trap: Option<(usize, usize)> = None;
        for (index, node) in self.graph.initial.iter().enumerate() {
            let node = node.0;
            if let Some(Some(witness)) = sampled.get(index) {
                witnesses.push(witness.clone());
                continue;
            }
            if d_p[node] != FAR {
                witnesses.push(GraphWitness {
                    path: ModelPath {
                        initial: initial_index(index)?,
                        steps: self.descend(node, &d_p, |_| true),
                    },
                    source: WitnessSource::Explored,
                });
                continue;
            }
            let is_trap = !self.is_open(node) && e_plus[node] == FAR;
            if is_trap && trap.is_none_or(|(first, _)| node < first) {
                trap = Some((node, index));
            }
        }
        if witnesses.len() == self.graph.initial.len() {
            return match undefined {
                Some((node, undefined)) => self.undefined(node, undefined),
                None if self.complete() => Ok(StateGraphOutcome::Witnessed {
                    paths: witnesses,
                    certificate: self.ranked(&d_p)?,
                }),
                None => Ok(StateGraphOutcome::WitnessedUnchecked {
                    paths: witnesses,
                    end: self.graph.end.clone(),
                    open: self.open_causes(),
                }),
            };
        }
        match (trap, undefined) {
            (Some((node, index)), found) if found.as_ref().is_none_or(|(at, _)| node < *at) => {
                Ok(StateGraphOutcome::Trapped {
                    stem: ModelPath {
                        initial: initial_index(index)?,
                        steps: Vec::new(),
                    },
                })
            }
            (_, Some((node, undefined))) => self.undefined(node, undefined),
            (_, None) => Ok(self.no_decision()),
        }
    }

    /// `always possible`: a reachable trap where `from` holds, else an
    /// undefined evaluation, else `Holds` on a complete run.
    fn always_possible(&self) -> Result<StateGraphOutcome, ModelCheckRefusal> {
        let mut reaching = self.nodes_where(PredicateRole::Target);
        reaching.extend(self.open_nodes());
        let e_plus = self.backward(&reaching, |_| true);
        let trap = (0..self.count()).find(|node| {
            !self.is_open(*node)
                && e_plus[*node] == FAR
                && self.holds(*node, PredicateRole::From) == Some(true)
        });
        let targets = self.nodes_where(PredicateRole::Target);
        self.settle(trap.map(|node| (node, Evidence::Trap)), || {
            self.ranked(&self.backward(&targets, |_| true))
        })
    }

    /// `unique path`: a known node where `from` holds with two paths, else a
    /// known one with none, else an undefined evaluation, else `Holds` on a
    /// complete run.
    fn unique_path(&self) -> Result<StateGraphOutcome, ModelCheckRefusal> {
        let to: Vec<usize> = self.nodes_where(PredicateRole::To);
        let is_to = {
            let mut flags = vec![false; self.count()];
            for node in &to {
                flags[*node] = true;
            }
            flags
        };
        let mut reaching = to.clone();
        reaching.extend(self.open_nodes());
        let in_h: Vec<bool> = self
            .backward(&reaching, |node| !is_to[node])
            .into_iter()
            .map(|distance| distance != FAR)
            .collect();
        let h_edge = |edge: usize| {
            let edge = &self.graph.edges[edge];
            in_h[edge.source.0] && in_h[edge.target.0] && !is_to[edge.source.0]
        };
        let reaches_open = self.backward_over(&self.open_nodes(), &h_edge);
        let (counts, order) = self.path_counts(&in_h, &is_to, &h_edge);
        let from = |node: usize| self.holds(node, PredicateRole::From) == Some(true);
        let pair = (0..self.count())
            .find(|node| reaches_open[*node] == FAR && from(*node) && counts[*node] == 2);
        // A known node reaches no open node in `H`, so it is closed and
        // what it reaches is settled.
        let trap = (0..self.count())
            .find(|node| reaches_open[*node] == FAR && from(*node) && counts[*node] == 0)
            .map(|node| (node, Evidence::Trap));
        match pair {
            Some(node) => {
                let d_y = self.backward_over(&to, &h_edge);
                let evidence = self.path_pair(node, &d_y, &h_edge)?;
                self.settle_with(node, evidence)
            }
            None => self.settle(trap, || self.counted(&in_h, &counts, &order)),
        }
    }

    /// The breadth-first distance from every node to the nearest of
    /// `sources`, backwards over the edges `admit` accepts.
    fn backward_over(&self, sources: &[usize], admit: &impl Fn(usize) -> bool) -> Vec<usize> {
        let mut incoming = vec![Vec::new(); self.count()];
        for (index, edge) in self.graph.edges.iter().enumerate() {
            if admit(index) {
                incoming[edge.target.0].push(edge.source.0);
            }
        }
        let mut distance = vec![FAR; self.count()];
        let mut queue = VecDeque::new();
        for source in sources {
            if distance[*source] == FAR {
                distance[*source] = 0;
                queue.push_back(*source);
            }
        }
        while let Some(node) = queue.pop_front() {
            for &predecessor in &incoming[node] {
                if distance[predecessor] == FAR {
                    distance[predecessor] = distance[node].saturating_add(1);
                    queue.push_back(predecessor);
                }
            }
        }
        distance
    }

    /// Each node's count of paths to a `to` node in `H`, saturating at 2,
    /// and its position in a reverse topological order of `H`'s
    /// condensation.
    fn path_counts(
        &self,
        in_h: &[bool],
        is_to: &[bool],
        h_edge: &impl Fn(usize) -> bool,
    ) -> (Vec<u8>, Vec<u64>) {
        let outgoing: Vec<Vec<usize>> = (0..self.count())
            .map(|node| {
                self.graph.outgoing[node]
                    .iter()
                    .copied()
                    .filter(|edge| h_edge(*edge))
                    .map(|edge| self.graph.edges[edge].target.0)
                    .collect()
            })
            .collect();
        let components = condensation(&outgoing);
        let mut counts = vec![0u8; self.count()];
        let mut order = vec![0u64; self.count()];
        let mut position = 0u64;
        // `condensation` lists components in topological order; counting
        // runs in reverse, so each successor is counted first.
        for component in components.iter().rev() {
            let cyclic =
                component.len() > 1 || component.iter().any(|node| outgoing[*node].contains(node));
            for &node in component {
                order[node] = position;
                position = position.saturating_add(1);
                counts[node] = if !in_h[node] {
                    0
                } else if is_to[node] {
                    1
                } else if cyclic {
                    2
                } else {
                    outgoing[node].iter().fold(0u8, |sum, target| {
                        sum.saturating_add(counts[*target]).min(2)
                    })
                };
            }
        }
        (counts, order)
    }

    /// The certificate's closure: every explored node's digest, sorted, and
    /// each node's index in it.
    fn closure(&self) -> Result<(Vec<DigestRecord>, Vec<u64>), ModelCheckRefusal> {
        let mut closure: Vec<DigestRecord> = self.graph.nodes.iter().map(|node| node.key).collect();
        closure.sort();
        closure.dedup();
        let index = self
            .graph
            .nodes
            .iter()
            .map(|node| {
                closure
                    .binary_search(&node.key)
                    .ok()
                    .and_then(|at| u64::try_from(at).ok())
                    .ok_or_else(|| fault("closure-holds-every-node"))
            })
            .collect::<Result<_, _>>()?;
        Ok((closure, index))
    }

    /// The `possible` and `always possible` certificate (FR-169, ADR-022
    /// GC-1): the closure and each node's `distance` to the nearest target
    /// node, for every node that reaches one.
    fn ranked(&self, distance: &[usize]) -> Result<StateGraphCertificate, ModelCheckRefusal> {
        let (closure, index) = self.closure()?;
        let mut ranks = Vec::new();
        for (node, d) in distance.iter().enumerate() {
            if *d != FAR {
                let d = u32::try_from(*d).map_err(|_| fault("distance-fits-u32"))?;
                ranks.push((index[node], d));
            }
        }
        ranks.sort_unstable();
        Ok(StateGraphCertificate {
            closure,
            ranks,
            counts: Vec::new(),
        })
    }

    /// The `unique path` certificate (FR-169, ADR-022 GC-1): the closure and
    /// each node of `H`'s path count and position in reverse topological
    /// order.
    fn counted(
        &self,
        in_h: &[bool],
        counts: &[u8],
        order: &[u64],
    ) -> Result<StateGraphCertificate, ModelCheckRefusal> {
        let (closure, index) = self.closure()?;
        let mut entries: Vec<(u64, u8, u64)> = (0..self.count())
            .filter(|node| in_h[*node])
            .map(|node| (index[node], counts[node], order[node]))
            .collect();
        entries.sort_unstable();
        Ok(StateGraphCertificate {
            closure,
            ranks: Vec::new(),
            counts: entries,
        })
    }

    /// The shortest path from `node` to the nearest node at distance 0,
    /// taking at each node the first edge in canonical order that `admit`
    /// accepts and whose target is one nearer.
    fn descend(
        &self,
        node: usize,
        distance: &[usize],
        admit: impl Fn(usize) -> bool,
    ) -> Vec<ModelStep> {
        self.descend_edges(node, distance, &admit)
            .into_iter()
            .map(|edge| self.step(edge))
            .collect()
    }

    fn descend_edges(
        &self,
        mut node: usize,
        distance: &[usize],
        admit: &impl Fn(usize) -> bool,
    ) -> Vec<usize> {
        let mut path = Vec::new();
        while distance[node] != 0 && distance[node] != FAR {
            let Some(edge) = self.graph.outgoing[node].iter().copied().find(|edge| {
                admit(*edge) && distance[self.graph.edges[*edge].target.0] == distance[node] - 1
            }) else {
                break;
            };
            path.push(edge);
            node = self.graph.edges[edge].target.0;
        }
        path
    }

    /// The path pair from `node`: the shortest path to `to`, ties broken by
    /// canonical edge order, and the least, by length then canonical edge
    /// order, of the paths that first deviate from it.
    fn path_pair(
        &self,
        node: usize,
        d_y: &[usize],
        h_edge: &impl Fn(usize) -> bool,
    ) -> Result<Evidence, ModelCheckRefusal> {
        let first = self.descend_edges(node, d_y, h_edge);
        let second = self
            .second_path(&first, d_y, h_edge)
            .ok_or_else(|| fault("count-2-node-has-a-second-path"))?;
        Ok(Evidence::PairOf {
            first: first.iter().map(|edge| self.step(*edge)).collect(),
            second: second.iter().map(|edge| self.step(*edge)).collect(),
        })
    }

    /// The edges of the least path that first deviates from `first`, by
    /// length then canonical edge order; `None` when no edge out of a node
    /// of `first` other than its own leads into `H` toward `to`.
    ///
    /// A deviation at a position of `first` by one of its node's other edges
    /// has the length of the prefix, the edge and the target's distance, so
    /// no candidate is built to compare it. Two of one length first differ
    /// at the earlier deviation position, where the earlier one takes its
    /// own edge and the later one takes `first`'s; edges out of one node
    /// have distinct canonical ranks, so the earlier is the lesser exactly
    /// when its edge ranks below `first`'s there. One scan of the edges out
    /// of `first`'s nodes, in position then canonical order, finds it,
    /// linear in nodes and edges.
    fn second_path(
        &self,
        first: &[usize],
        d_y: &[usize],
        h_edge: &impl Fn(usize) -> bool,
    ) -> Option<Vec<usize>> {
        let mut best: Option<Deviation> = None;
        for (position, taken) in first.iter().enumerate() {
            let source = self.graph.edges[*taken].source.0;
            let mut below = true;
            for &edge in &self.graph.outgoing[source] {
                if edge == *taken {
                    below = false;
                    continue;
                }
                let target = self.graph.edges[edge].target.0;
                if !h_edge(edge) || d_y[target] == FAR {
                    continue;
                }
                let candidate = Deviation {
                    length: position + 1 + d_y[target],
                    position,
                    edge,
                    below,
                };
                // Scanning in position then canonical order, an equal-length
                // candidate at the same position ranks above the best, and
                // one at a later position is less unless the best is below.
                let better = best.as_ref().is_none_or(|current| {
                    candidate.length < current.length
                        || (candidate.length == current.length
                            && candidate.position > current.position
                            && !current.below)
                });
                if better {
                    best = Some(candidate);
                }
            }
        }
        let deviation = best?;
        let mut second = first[..deviation.position].to_vec();
        second.push(deviation.edge);
        second.extend(self.descend_edges(self.graph.edges[deviation.edge].target.0, d_y, h_edge));
        Some(second)
    }

    /// Settle with refuting `evidence` at its node, unless an undefined
    /// evaluation is at an earlier node or the same one.
    fn settle_with(
        &self,
        node: usize,
        evidence: Evidence,
    ) -> Result<StateGraphOutcome, ModelCheckRefusal> {
        if let Some((at, undefined)) = self.first_undefined() {
            if at <= node {
                return self.undefined(at, undefined);
            }
        }
        Ok(match evidence {
            Evidence::Trap => StateGraphOutcome::Trapped {
                stem: self.stem(node)?,
            },
            Evidence::PairOf { first, second } => StateGraphOutcome::PathPair {
                stem: self.stem(node)?,
                first,
                second,
            },
        })
    }

    /// Settle with the trap at `trap`, if any, else an undefined evaluation,
    /// else `Holds` with `certificate`'s certificate on a complete run.
    fn settle(
        &self,
        trap: Option<(usize, Evidence)>,
        certificate: impl FnOnce() -> Result<StateGraphCertificate, ModelCheckRefusal>,
    ) -> Result<StateGraphOutcome, ModelCheckRefusal> {
        if let Some((node, evidence)) = trap {
            return self.settle_with(node, evidence);
        }
        if let Some((at, undefined)) = self.first_undefined() {
            return self.undefined(at, undefined);
        }
        Ok(if self.complete() {
            StateGraphOutcome::Holds {
                certificate: certificate()?,
            }
        } else {
            self.no_decision()
        })
    }
}

/// Refuting evidence at a node.
enum Evidence {
    /// The node is a trap.
    Trap,
    /// The node's path pair.
    PairOf {
        first: Vec<ModelStep>,
        second: Vec<ModelStep>,
    },
}

/// A path that first deviates from a path pair's first path.
struct Deviation {
    /// Its length in steps.
    length: usize,
    /// The position of the first path where it deviates.
    position: usize,
    /// The edge it takes there.
    edge: usize,
    /// Whether that edge ranks below the first path's edge there.
    below: bool,
}

/// `index`, an initial-state index, as a [`ModelPath`] stores it.
fn initial_index(index: usize) -> Result<u32, ModelCheckRefusal> {
    u32::try_from(index).map_err(|_| fault("initial-index-fits-u32"))
}

/// The strongly connected components of the graph whose successors are
/// `outgoing`, in topological order of the condensation: every edge goes
/// from a component to itself or to a later one. Iterative Kosaraju, linear
/// in nodes and edges.
fn condensation(outgoing: &[Vec<usize>]) -> Vec<Vec<usize>> {
    let count = outgoing.len();
    let mut visited = vec![false; count];
    let mut finished = Vec::with_capacity(count);
    for root in 0..count {
        if visited[root] {
            continue;
        }
        visited[root] = true;
        let mut stack = vec![(root, 0)];
        while let Some((node, next)) = stack.last_mut() {
            if let Some(&target) = outgoing[*node].get(*next) {
                *next += 1;
                if !visited[target] {
                    visited[target] = true;
                    stack.push((target, 0));
                }
            } else {
                finished.push(*node);
                stack.pop();
            }
        }
    }
    let mut incoming = vec![Vec::new(); count];
    for (source, targets) in outgoing.iter().enumerate() {
        for &target in targets {
            incoming[target].push(source);
        }
    }
    visited.fill(false);
    let mut components = Vec::new();
    // Taking roots by decreasing finish time over the reversed graph yields
    // the components in topological order of the original graph.
    for root in finished.into_iter().rev() {
        if visited[root] {
            continue;
        }
        visited[root] = true;
        let mut component = Vec::new();
        let mut stack = vec![root];
        while let Some(node) = stack.pop() {
            component.push(node);
            for &source in &incoming[node] {
                if !visited[source] {
                    visited[source] = true;
                    stack.push(source);
                }
            }
        }
        components.push(component);
    }
    components
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use qsl_eval::simulation::model::{ClauseOutcome, PredicateReading};
    use qsl_foundation::digest::{DigestDomain, DigestRecord};
    use qsl_replay::{
        GraphWitness, ModelPath, OpenCause, RunEnd, StateGraphOutcome, WitnessSource,
    };
    use qsl_semantics::check::{CheckedStatePredicate, PredicateRole, StateGraphForm};
    use quire_semantic_value::model_transition::ModelTransition;

    use super::super::{
        ExploredStateGraph, GraphEdge, GraphNode, GraphStats, InstanceView, NodeIndex,
        PredicateLabel,
    };
    use super::{decide, Reader, FAR};

    /// Item `item`'s one instance, `possible` with an unwritten target.
    fn view(item: usize) -> InstanceView<'static> {
        InstanceView {
            item,
            instance: 0,
            over: None,
            predicates: vec![(PredicateRole::Target, None)],
        }
    }

    /// SR-12183 FND-006: a predicate of item 0 that read refused at the one
    /// node of a completed run opens that node for item 0 alone,
    /// `UndecidedSuccessor`; item 1 over the same graph is `Witnessed`.
    #[test]
    fn a_refused_reading_opens_its_node_for_its_own_instance_only() {
        let graph = ExploredStateGraph {
            nodes: vec![GraphNode {
                key: DigestRecord::mint(DigestDomain::SimulationStateKeyV1, [0; 32]),
                depth: 0,
                labels: vec![PredicateLabel {
                    item: 0,
                    instance: 0,
                    role: PredicateRole::Target,
                    reading: PredicateReading::Refused(ClauseOutcome::NotBoolean),
                }],
            }],
            edges: Vec::new(),
            open: BTreeMap::new(),
            end: RunEnd::Completed,
            stats: GraphStats::default(),
            outgoing: vec![Vec::new()],
            parent: vec![None],
            root: vec![0],
            initial: vec![NodeIndex(0)],
        };
        let form = StateGraphForm::Possible {
            target: CheckedStatePredicate::ConstantTrue,
        };
        let paths = vec![GraphWitness {
            path: ModelPath {
                initial: 0,
                steps: Vec::new(),
            },
            source: WitnessSource::Explored,
        }];
        assert_eq!(
            decide(&graph, &form, &view(0), &[]).expect("decides"),
            StateGraphOutcome::WitnessedUnchecked {
                paths: paths.clone(),
                end: RunEnd::Completed,
                open: BTreeSet::from([OpenCause::UndecidedSuccessor]),
            }
        );
        let StateGraphOutcome::Witnessed {
            paths: witnessed, ..
        } = decide(&graph, &form, &view(1), &[]).expect("decides")
        else {
            panic!("item 1 is witnessed");
        };
        assert_eq!(witnessed, paths);
    }

    /// A closed, completed graph over `count` nodes with `edges`, each
    /// `(source, target)`; each node's outgoing edges rank in list order.
    fn graph_of(count: usize, edges: &[(usize, usize)]) -> ExploredStateGraph {
        let transition = |index: usize| {
            ModelTransition::new(
                format!("\"e{index}\""),
                None,
                Vec::new(),
                quire_canonical::Limits::new(1 << 10),
            )
            .expect("a transition")
        };
        let mut outgoing = vec![Vec::new(); count];
        for (index, (source, _)) in edges.iter().enumerate() {
            outgoing[*source].push(index);
        }
        ExploredStateGraph {
            nodes: (0..count)
                .map(|node| GraphNode {
                    key: DigestRecord::mint(
                        DigestDomain::SimulationStateKeyV1,
                        [u8::try_from(node).expect("a small graph"); 32],
                    ),
                    depth: 0,
                    labels: Vec::new(),
                })
                .collect(),
            edges: edges
                .iter()
                .enumerate()
                .map(|(index, (source, target))| GraphEdge {
                    source: NodeIndex(*source),
                    transition: transition(index),
                    target: NodeIndex(*target),
                })
                .collect(),
            open: BTreeMap::new(),
            end: RunEnd::Completed,
            stats: GraphStats::default(),
            outgoing,
            parent: vec![None; count],
            root: vec![0; count],
            initial: vec![NodeIndex(0)],
        }
    }

    /// The second path as the scan replaced in QSL-537 chose it: build every
    /// path that first deviates from `first` and keep the least by length
    /// then the canonical ranks of its edges.
    fn by_candidates(
        reader: &Reader<'_>,
        first: &[usize],
        d_y: &[usize],
        h_edge: &impl Fn(usize) -> bool,
    ) -> Option<Vec<usize>> {
        let graph = reader.graph;
        let ranks = |path: &[usize]| -> Vec<usize> {
            path.iter()
                .map(|edge| {
                    let source = graph.edges[*edge].source.0;
                    graph.outgoing[source]
                        .iter()
                        .position(|candidate| candidate == edge)
                        .expect("an edge is out of its source")
                })
                .collect()
        };
        let mut best: Option<Vec<usize>> = None;
        for (position, taken) in first.iter().enumerate() {
            let source = graph.edges[*taken].source.0;
            for &edge in &graph.outgoing[source] {
                let target = graph.edges[edge].target.0;
                if edge == *taken || !h_edge(edge) || d_y[target] == FAR {
                    continue;
                }
                let mut candidate = first[..position].to_vec();
                candidate.push(edge);
                candidate.extend(reader.descend_edges(target, d_y, h_edge));
                let better = best.as_ref().is_none_or(|current| {
                    (candidate.len(), ranks(&candidate)) < (current.len(), ranks(current))
                });
                if better {
                    best = Some(candidate);
                }
            }
        }
        best
    }

    /// The scan's second path from every node of `graph` with a path to
    /// `to`, against [`by_candidates`]; `H`'s edges are those out of a node
    /// that is not a `to` node. Returns the number of nodes compared.
    fn agrees(graph: &ExploredStateGraph, to: &[usize]) -> usize {
        let view = view(0);
        let reader = Reader::new(graph, &view);
        let h_edge = |edge: usize| !to.contains(&graph.edges[edge].source.0);
        let d_y = reader.backward_over(to, &h_edge);
        let mut compared = 0;
        for node in (0..graph.nodes.len()).filter(|node| d_y[*node] != FAR && d_y[*node] > 0) {
            let first = reader.descend_edges(node, &d_y, &h_edge);
            assert_eq!(
                reader.second_path(&first, &d_y, &h_edge),
                by_candidates(&reader, &first, &d_y, &h_edge),
                "node {node} of {:?} to {to:?}",
                graph
                    .edges
                    .iter()
                    .map(|edge| (edge.source.0, edge.target.0))
                    .collect::<Vec<_>>()
            );
            compared += 1;
        }
        compared
    }

    /// FR-168's second-path choice: the one scan picks the path the old
    /// build-every-candidate comparison picks, over the shapes of §7.3's
    /// jobs, `Ladder` and `Detour` and over 2,000 seeded random graphs of up
    /// to seven nodes, cycles, self-loops and parallel edges included.
    #[test]
    fn the_second_path_scan_agrees_with_building_every_candidate() {
        // §7.3's job: 0 -stepA-> 1, 0 -stepB-> 2, both -> 3, 3 -finish-> 4.
        assert!(
            agrees(
                &graph_of(5, &[(0, 1), (0, 2), (1, 3), (2, 3), (3, 4)]),
                &[4]
            ) > 0
        );
        // The sequenced job with `reset`: 0 -stepA-> 1, 1 -stepB-> 2,
        // 1 -reset-> 0, 2 -finish-> 3.
        assert!(agrees(&graph_of(4, &[(0, 1), (1, 2), (1, 0), (2, 3)]), &[3]) > 0);
        // `Ladder`: a, b, c, d, e.
        assert!(
            agrees(
                &graph_of(4, &[(0, 1), (0, 2), (1, 3), (1, 3), (2, 3)]),
                &[3]
            ) > 0
        );
        // `Detour`: s, t, u, w, y, x.
        let detour = graph_of(6, &[(0, 1), (0, 2), (1, 4), (2, 5), (2, 4), (4, 5)]);
        assert!(agrees(&detour, &[5]) > 0);
        let mut state: u64 = 0x9E37_79B9_7F4A_7C15;
        let mut next = |bound: usize| {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            usize::try_from(state % u64::try_from(bound).expect("fits")).expect("fits")
        };
        let mut compared = 0;
        for _ in 0..2_000 {
            let count = 2 + next(6);
            let edges: Vec<(usize, usize)> = (0..next(3 * count))
                .map(|_| (next(count), next(count)))
                .collect();
            let to: Vec<usize> = (0..1 + next(2)).map(|_| next(count)).collect();
            compared += agrees(&graph_of(count, &edges), &to);
        }
        assert!(
            compared > 1_000,
            "the random graphs compared {compared} nodes"
        );
    }
}
