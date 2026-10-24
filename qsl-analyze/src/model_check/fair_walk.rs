// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-018 CX-5 and ADR-019 FS-7: the canonical lasso of a fair accepting
//! component, shared by every EN-1 second phase.
//!
//! A [`FairModel`] is one retained product as the fairness kernels read it.
//! The liveness phase (FR-126) and the hyper phase (FR-176) each build their
//! own and take their stem from [`tree_path`] and their loop from
//! [`FairModel::greedy_loop`], so the two return the same canonical lasso for
//! the same graph.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use qsl_eval::fairness::{
    Constraint, Edge, EdgeLabel, Graph, Identity, Obligation, Strength, Vertex,
};
use qsl_eval::simulation::model::ModelSystem;
use quire_semantic_value::fairness::{FairnessConstraint, FairnessGranularity};
use quire_semantic_value::model_transition::ModelTransition;

use super::check::{fault, ModelCheckRefusal};
use super::graph::{ProductStateId, RetainedEdge, RetainedGraph};
use super::liveness::LivenessRefusal;
use super::quotient::LoopEdge;

/// The kernels' constraints for `fairness` over `transitions`, the
/// transition identities the graph holds, numbered by `identity`: a `whole`
/// constraint covers every transition of its operation, an `each` constraint
/// is one constraint per such transition (ADR-018 FA-1).
pub(super) fn fairness_constraints(
    system: &ModelSystem<'_>,
    fairness: &[FairnessConstraint],
    transitions: &[ModelTransition],
    identity: impl Fn(&ModelTransition) -> Result<Identity, ModelCheckRefusal>,
) -> Result<Vec<Constraint>, ModelCheckRefusal> {
    let mut constraints = Vec::new();
    for constraint in fairness {
        let strength = Strength::from(constraint.kind());
        let operation = constraint.operation();
        let text = system
            .operation_text(operation.declaring_type(), operation.name())
            .ok_or_else(|| LivenessRefusal::UnknownOperation {
                name: operation.name().to_owned(),
            })?;
        let covered = transitions
            .iter()
            .filter(|transition| transition.operation() == text);
        match constraint.granularity() {
            FairnessGranularity::Whole => {
                let members = covered
                    .map(&identity)
                    .collect::<Result<Vec<_>, ModelCheckRefusal>>()?;
                constraints.push(Constraint::new(strength, members));
            }
            FairnessGranularity::Each => {
                for transition in covered {
                    constraints.push(Constraint::new(strength, [identity(transition)?]));
                }
            }
        }
    }
    Ok(constraints)
}

/// The breadth-first tree path to `at`: the initial state it starts from and
/// the edges that first discovered each state on the way.
pub(super) fn tree_path<S, T, F>(
    graph: &RetainedGraph<S, T, F>,
    at: ProductStateId,
) -> (ProductStateId, Vec<&RetainedEdge<T>>) {
    let mut discovered_by: BTreeMap<ProductStateId, &RetainedEdge<T>> = BTreeMap::new();
    let initial: BTreeSet<ProductStateId> = graph.initial().iter().copied().collect();
    for edge in graph.edges() {
        if !initial.contains(&edge.target) {
            discovered_by.entry(edge.target).or_insert(edge);
        }
    }
    let mut path = Vec::new();
    let mut cursor = at;
    while let Some(edge) = discovered_by.get(&cursor) {
        path.push(*edge);
        cursor = edge.source;
    }
    path.reverse();
    (cursor, path)
}

/// One graph as the fairness kernels read it: vertices and edges of the
/// retained states, the model states' enabled identities, the automaton's
/// acceptance sets and the clause's constraints.
pub(super) struct FairModel {
    pub(super) graph: Graph,
    pub(super) constraints: Vec<Constraint>,
}

impl FairModel {
    /// The retained edges of a walk over the graph, each as its source and
    /// its position among the source's edges (the graph's edges are added in
    /// the retained graph's canonical successor order). A parallel edge with
    /// the same label and target is the first such.
    pub(super) fn loop_edges(
        &self,
        walk: &[(Vertex, Edge)],
    ) -> Result<Vec<LoopEdge>, ModelCheckRefusal> {
        walk.iter()
            .map(|(source, edge)| {
                self.graph
                    .edges_from(*source)
                    .iter()
                    .position(|held| held == edge)
                    .map(|position| (ProductStateId(source.index()), position))
                    .ok_or_else(|| fault("a-walk-edge-is-a-graph-edge"))
            })
            .collect()
    }

    /// ADR-018 CX-5's greedy walk inside `component` from its entry `entry`:
    /// the loop's edges, each with the vertex it leaves. The walk takes no
    /// edge whose identity is in `removed`, the edges the filter's
    /// visibility refinements left out of the passing component (ADR-025
    /// MF-2).
    pub(super) fn greedy_loop(
        &self,
        component: &BTreeSet<Vertex>,
        removed: &BTreeSet<Identity>,
        entry: Vertex,
    ) -> Result<Vec<(Vertex, Edge)>, ModelCheckRefusal> {
        let inside = Inside { component, removed };
        let mut open = Open::of(self, &inside);
        open.discharge_vertex(self, entry);
        let mut walk = Vec::new();
        let mut current = entry;
        while !open.is_empty() {
            let path = self
                .nearest(&inside, current, &open)
                .ok_or_else(|| fault("a-passing-component-discharges-every-obligation"))?;
            for (from, edge) in &path {
                open.discharge_edge(self, edge.label);
                open.discharge_vertex(self, edge.target);
                current = edge.target;
                walk.push((*from, *edge));
            }
        }
        if current != entry || walk.is_empty() {
            let back = self
                .shortest_return(&inside, current, entry)
                .ok_or_else(|| fault("a-component-vertex-lies-on-a-cycle-through-the-entry"))?;
            walk.extend(back);
        }
        Ok(walk)
    }

    /// The shortest path inside `component` from `from` to the nearest
    /// vertex or edge that discharges an open obligation, ties broken by the
    /// canonical order of the path's transitions (breadth-first order over
    /// edges in canonical successor order).
    fn nearest(
        &self,
        inside: &Inside<'_>,
        from: Vertex,
        open: &Open,
    ) -> Option<Vec<(Vertex, Edge)>> {
        let mut parent: BTreeMap<Vertex, (Vertex, usize)> = BTreeMap::new();
        let mut distance: BTreeMap<Vertex, usize> = BTreeMap::from([(from, 0)]);
        let mut queue = VecDeque::from([from]);
        // Candidates of the least length so far: the vertex the path reaches
        // and, for an edge obligation, the position of its final edge.
        let mut best: Option<usize> = None;
        let mut candidates: Vec<(Vertex, Option<usize>)> = Vec::new();
        while let Some(vertex) = queue.pop_front() {
            let reached = *distance.get(&vertex)?;
            if best.is_some_and(|length| reached > length) {
                break;
            }
            let mut consider = |length: usize, candidate: (Vertex, Option<usize>)| {
                match best {
                    Some(least) if length > least => return,
                    Some(least) if length < least => candidates.clear(),
                    _ => {}
                }
                best = Some(length);
                candidates.push(candidate);
            };
            if vertex != from && open.discharged_by_vertex(self, vertex) {
                consider(reached, (vertex, None));
            }
            for (position, edge) in self.graph.edges_from(vertex).iter().enumerate() {
                if !inside.keeps(edge) {
                    continue;
                }
                if open.discharged_by_edge(self, edge.label) {
                    consider(reached + 1, (vertex, Some(position)));
                }
                if let std::collections::btree_map::Entry::Vacant(slot) =
                    distance.entry(edge.target)
                {
                    slot.insert(reached + 1);
                    parent.insert(edge.target, (vertex, position));
                    queue.push_back(edge.target);
                }
            }
        }
        let mut chosen: Option<Vec<(Vertex, usize)>> = None;
        for (end, last) in candidates {
            let mut path = self.climb(&parent, from, end)?;
            if let Some(position) = last {
                path.push((end, position));
            }
            let ordinals = |path: &[(Vertex, usize)]| -> Vec<usize> {
                path.iter().map(|(_, position)| *position).collect()
            };
            if chosen
                .as_deref()
                .is_none_or(|kept| ordinals(&path) < ordinals(kept))
            {
                chosen = Some(path);
            }
        }
        self.edges_of(&chosen?)
    }

    /// The tree path from `from` to `end` as (vertex, edge position) pairs.
    fn climb(
        &self,
        parent: &BTreeMap<Vertex, (Vertex, usize)>,
        from: Vertex,
        end: Vertex,
    ) -> Option<Vec<(Vertex, usize)>> {
        let mut path = Vec::new();
        let mut cursor = end;
        while cursor != from {
            let (before, position) = *parent.get(&cursor)?;
            path.push((before, position));
            cursor = before;
        }
        path.reverse();
        Some(path)
    }

    fn edges_of(&self, path: &[(Vertex, usize)]) -> Option<Vec<(Vertex, Edge)>> {
        path.iter()
            .map(|(vertex, position)| {
                self.graph
                    .edges_from(*vertex)
                    .get(*position)
                    .map(|edge| (*vertex, *edge))
            })
            .collect()
    }

    /// The shortest path inside `component` that leaves `from` along at
    /// least one edge and ends at `entry`: the walk's way back, or the
    /// shortest cycle through `entry` when `from` is `entry`.
    fn shortest_return(
        &self,
        inside: &Inside<'_>,
        from: Vertex,
        entry: Vertex,
    ) -> Option<Vec<(Vertex, Edge)>> {
        let mut parent: BTreeMap<Vertex, (Vertex, usize)> = BTreeMap::new();
        let mut seen: BTreeSet<Vertex> = BTreeSet::from([from]);
        let mut queue = VecDeque::from([from]);
        while let Some(vertex) = queue.pop_front() {
            for (position, edge) in self.graph.edges_from(vertex).iter().enumerate() {
                if !inside.keeps(edge) {
                    continue;
                }
                if edge.target == entry {
                    let mut path = self.climb(&parent, from, vertex)?;
                    path.push((vertex, position));
                    return self.edges_of(&path);
                }
                if seen.insert(edge.target) {
                    parent.insert(edge.target, (vertex, position));
                    queue.push_back(edge.target);
                }
            }
        }
        None
    }
}

/// A passing component's subgraph: its vertices, and the edge identities
/// the filter removed from it.
struct Inside<'a> {
    component: &'a BTreeSet<Vertex>,
    removed: &'a BTreeSet<Identity>,
}

impl Inside<'_> {
    /// Whether the walk may take `edge` out of a vertex of the component.
    fn keeps(&self, edge: &Edge) -> bool {
        self.component.contains(&edge.target)
            && match edge.label {
                EdgeLabel::Transition(identity) => !self.removed.contains(&identity),
                EdgeLabel::Stutter => true,
            }
    }
}

/// The obligations of a component not yet discharged (CX-5, ADR-019 FS-7):
/// one state of each acceptance set, for each weak constraint an edge that
/// takes it or a state that disables it, for each strong constraint enabled
/// at some state of the component an edge that takes it, and for each
/// visibility constraint a kept edge of the component loads (ADR-025 MF-2)
/// an edge that takes it.
struct Open {
    sets: BTreeSet<usize>,
    constraints: BTreeSet<usize>,
}

impl Open {
    /// The obligations of the component `inside`: a strong constraint
    /// enabled at no state of it, and a visibility constraint whose loads no
    /// kept edge of it takes, have none.
    fn of(model: &FairModel, inside: &Inside<'_>) -> Self {
        let owed = |constraint: &Constraint| match constraint.obligation() {
            Obligation::Enablement(Strength::Weak) => true,
            Obligation::Enablement(Strength::Strong) => inside.component.iter().any(|vertex| {
                model
                    .graph
                    .enabled(*vertex)
                    .and_then(|enabled| constraint.is_enabled_in(enabled))
                    == Some(true)
            }),
            Obligation::Visibility { .. } => inside.component.iter().any(|vertex| {
                model
                    .graph
                    .edges_from(*vertex)
                    .iter()
                    .any(|edge| inside.keeps(edge) && constraint.is_owed_by(edge.label))
            }),
        };
        Self {
            sets: (0..model.graph.acceptance_sets().len()).collect(),
            constraints: model
                .constraints
                .iter()
                .enumerate()
                .filter(|(_, constraint)| owed(constraint))
                .map(|(index, _)| index)
                .collect(),
        }
    }

    fn is_empty(&self) -> bool {
        self.sets.is_empty() && self.constraints.is_empty()
    }

    /// Whether `vertex` discharges the obligation of `constraint`: a weak
    /// constraint is disabled there. A strong constraint is never discharged
    /// by a state.
    fn disabled_at(model: &FairModel, constraint: usize, vertex: Vertex) -> bool {
        match (
            model.constraints.get(constraint),
            model.graph.enabled(vertex),
        ) {
            // Only an enablement weak constraint is discharged by a state.
            (Some(constraint), Some(enabled)) => {
                constraint.obligation() == &Obligation::Enablement(Strength::Weak)
                    && constraint.is_enabled_in(enabled) == Some(false)
            }
            _ => false,
        }
    }

    fn discharged_by_vertex(&self, model: &FairModel, vertex: Vertex) -> bool {
        self.sets.iter().any(|set| {
            model
                .graph
                .acceptance_sets()
                .get(*set)
                .is_some_and(|members| members.contains(&vertex))
        }) || self
            .constraints
            .iter()
            .any(|constraint| Self::disabled_at(model, *constraint, vertex))
    }

    fn discharged_by_edge(&self, model: &FairModel, label: EdgeLabel) -> bool {
        self.constraints.iter().any(|constraint| {
            model
                .constraints
                .get(*constraint)
                .is_some_and(|constraint| constraint.is_taken_by(label))
        })
    }

    fn discharge_vertex(&mut self, model: &FairModel, vertex: Vertex) {
        self.sets.retain(|set| {
            !model
                .graph
                .acceptance_sets()
                .get(*set)
                .is_some_and(|members| members.contains(&vertex))
        });
        self.constraints
            .retain(|constraint| !Self::disabled_at(model, *constraint, vertex));
    }

    fn discharge_edge(&mut self, model: &FairModel, label: EdgeLabel) {
        self.constraints.retain(|constraint| {
            !model
                .constraints
                .get(*constraint)
                .is_some_and(|constraint| constraint.is_taken_by(label))
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;
    use qsl_eval::fairness::{analyze, Analysis};
    use quire_exact::Cancel;

    /// CX-5 tie: from the entry v0, edge 0 reaches v1 (an acceptance-set
    /// state) and edge 1 takes the weak constraint; both obligations are at
    /// distance 1 and edge 0 is canonically first, so the walk is
    /// v0 v1 v0 v2 v0, not the edge obligation first.
    #[trace("TC-521")]
    #[test]
    fn a_vertex_obligation_ties_an_edge_obligation_at_the_same_distance() {
        let id0 = Identity::new(0);
        let id1 = Identity::new(1);
        let mut graph = Graph::new();
        let v0 = graph.add_vertex([id0, id1]);
        let v1 = graph.add_vertex([id0, id1]);
        let v2 = graph.add_vertex([id0, id1]);
        for (from, to, label) in [(v0, v1, id0), (v0, v2, id1), (v1, v0, id0), (v2, v0, id0)] {
            graph
                .add_edge(from, to, EdgeLabel::Transition(label))
                .expect("both vertices exist");
        }
        graph.add_acceptance_set([v1]).expect("v1 exists");
        let model = FairModel {
            graph,
            constraints: vec![Constraint::weak([id1])],
        };
        let component: BTreeSet<Vertex> = [v0, v1, v2].into_iter().collect();
        let walk = model
            .greedy_loop(&component, &BTreeSet::new(), v0)
            .expect("a walk exists");
        let targets: Vec<usize> = walk.iter().map(|(_, edge)| edge.target.index()).collect();
        assert_eq!(targets, [1, 0, 2, 0]);
    }

    /// ADR-025 MF-2 in CX-5: thread `b`'s loads of `l` are identities 10
    /// (stale) and 11 (mo-last); 20 is another step. The accepting `v0` has a
    /// stale self-loop, canonically its first edge, and the cycle
    /// `v0 -> v1 -> v0` loads nothing. The filter removes the stale load and
    /// passes the rest, and the walk takes only the kept edges: never the
    /// stale self-loop, whose loop would be unfair.
    #[trace("TC-673", "FR-228-AC-6")]
    #[test]
    fn the_walk_takes_no_edge_a_visibility_refinement_removed() {
        let (stale, fresh, other) = (Identity::new(10), Identity::new(11), Identity::new(20));
        let mut graph = Graph::new();
        let v0 = graph.add_vertex([stale, fresh, other]);
        let v1 = graph.add_vertex([other]);
        for (from, to, label) in [(v0, v0, stale), (v0, v1, other), (v1, v0, other)] {
            graph
                .add_edge(from, to, EdgeLabel::Transition(label))
                .expect("both vertices exist");
        }
        graph.add_acceptance_set([v0]).expect("v0 exists");
        let model = FairModel {
            graph,
            constraints: vec![Constraint::visibility([stale, fresh], [fresh])],
        };
        let all: BTreeSet<Vertex> = model.graph.vertices().collect();
        let analyses =
            analyze(&model.graph, &all, &model.constraints, &Cancel::new()).expect("the filter");
        let passing = Analysis::first_fair_in(&analyses).expect("a fair component");
        assert!(passing.removed().contains(&stale));
        let component: BTreeSet<Vertex> = passing.vertices().iter().copied().collect();
        let walk = model
            .greedy_loop(&component, passing.removed(), v0)
            .expect("a walk exists");
        let labels: Vec<EdgeLabel> = walk.iter().map(|(_, edge)| edge.label).collect();
        assert_eq!(
            labels,
            [EdgeLabel::Transition(other), EdgeLabel::Transition(other)]
        );
    }

    /// ADR-025 MF-2: a stale self-loop on an accepting state, with nothing
    /// else, holds no fair cycle, so the filter yields no counterexample.
    #[trace("TC-673", "FR-228-AC-3")]
    #[test]
    fn a_stale_self_loop_on_an_accepting_state_is_no_counterexample() {
        let (stale, fresh) = (Identity::new(10), Identity::new(11));
        let mut graph = Graph::new();
        let v0 = graph.add_vertex([stale, fresh]);
        graph
            .add_edge(v0, v0, EdgeLabel::Transition(stale))
            .expect("v0 exists");
        graph.add_acceptance_set([v0]).expect("v0 exists");
        let constraints = [Constraint::visibility([stale, fresh], [fresh])];
        let analyses = analyze(&graph, &BTreeSet::from([v0]), &constraints, &Cancel::new())
            .expect("the filter");
        assert!(Analysis::first_fair_in(&analyses).is_none());
    }

    /// FR-228-AC-8: a strong constraint (identity 9, enabled only at state
    /// 0) and a failing pair (stale `1 -> 2`) refine in one step; the second
    /// pair (stale `3 -> 3`, fresh `3 -> 4`) still loads `{3, 4}`, so the
    /// walk from the accepting state 3 discharges it by the fresh load and
    /// the loop is `3 -> 4 -> 3`.
    #[trace("TC-673", "FR-228-AC-8")]
    #[test]
    fn the_walk_discharges_a_pair_the_passing_component_still_loads() {
        let id = Identity::new;
        let mut graph = Graph::new();
        let vertices: Vec<Vertex> = (0..5)
            .map(|index| graph.add_vertex(if index == 0 { vec![id(9)] } else { vec![] }))
            .collect();
        for (from, to, label) in [
            (0, 1, 20),
            (1, 0, 21),
            (1, 2, 10),
            (2, 3, 22),
            (3, 1, 23),
            (3, 3, 12),
            (3, 4, 13),
            (4, 3, 24),
        ] {
            graph
                .add_edge(
                    vertices[from],
                    vertices[to],
                    EdgeLabel::Transition(id(label)),
                )
                .expect("both vertices exist");
        }
        graph
            .add_acceptance_set([vertices[3]])
            .expect("vertex 3 exists");
        let model = FairModel {
            graph,
            constraints: vec![
                Constraint::strong([id(9)]),
                Constraint::visibility([id(10)], [id(11)]),
                Constraint::visibility([id(12), id(13)], [id(13)]),
            ],
        };
        let all: BTreeSet<Vertex> = model.graph.vertices().collect();
        let analyses =
            analyze(&model.graph, &all, &model.constraints, &Cancel::new()).expect("the filter");
        let passing = Analysis::first_fair_in(&analyses).expect("a fair component");
        let component: BTreeSet<Vertex> = passing.vertices().iter().copied().collect();
        assert_eq!(component, BTreeSet::from([vertices[3], vertices[4]]));
        let walk = model
            .greedy_loop(&component, passing.removed(), vertices[3])
            .expect("a walk exists");
        let path: Vec<(usize, usize)> = walk
            .iter()
            .map(|(from, edge)| (from.index(), edge.target.index()))
            .collect();
        assert_eq!(path, [(3, 4), (4, 3)]);
    }
}
