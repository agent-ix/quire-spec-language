// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-339 "Behavior": the facts FR-338's closure check of a liveness (TP-4)
//! proof computes, kept so the component work reads only them: the reached
//! product states in the order the check reached them, the edges it computed
//! between them, each state's enabled transitions and the acceptance sets of
//! each instance's automaton.
//!
//! A [`Recording`] numbers each transition by its FR-101 canonical bytes, in
//! the order the check first sees one, so [`Finished::identity`] gives a
//! transition the number its edges and enabled sets carry. Instances whose
//! key contexts are equal (two admitting populations that hold the same
//! key) reach the same product states: the later instance reuses them and
//! adds none of their enabled sets or edges again.

use std::collections::{BTreeMap, BTreeSet};

use qsl_eval::fairness::{EdgeLabel, Identity, Vertex};
use qsl_eval::simulation::{EncodingRefusal, SubjectSystem};

use crate::certificate::closure::ProductStateRef;
use crate::certificate::scc::{ProductGraph, ProductGraphError};

/// One recorded product state.
struct Recorded {
    at: ProductStateRef,
    enabled: BTreeSet<Identity>,
    edges: Vec<(usize, EdgeLabel)>,
}

/// The product graph a closure check of a TP-4 item has computed so far.
pub(super) struct Recording<S: SubjectSystem> {
    states: Vec<Recorded>,
    index: BTreeMap<ProductStateRef, usize>,
    numbers: BTreeMap<Vec<u8>, (Identity, S::TransitionId)>,
    /// Per acceptance set, the recorded states that belong to it.
    acceptance: Vec<BTreeSet<usize>>,
    /// The first state this instance recorded.
    first: usize,
}

impl<S: SubjectSystem> Recording<S> {
    pub(super) const fn new() -> Self {
        Self {
            states: Vec::new(),
            index: BTreeMap::new(),
            numbers: BTreeMap::new(),
            acceptance: Vec::new(),
            first: 0,
        }
    }

    /// The next instance begins: states it reaches that an earlier instance
    /// recorded keep that instance's enabled sets and edges.
    pub(super) fn begin_instance(&mut self) {
        self.first = self.states.len();
    }

    /// Record the product state `at`, which belongs to the acceptance sets
    /// `accepting` marks, and return its index. `None` when an instance's
    /// automaton has another number of acceptance sets than an earlier one's.
    pub(super) fn reach(&mut self, at: &ProductStateRef, accepting: &[bool]) -> Option<usize> {
        if self.states.is_empty() {
            self.acceptance = vec![BTreeSet::new(); accepting.len()];
        } else if self.acceptance.len() != accepting.len() {
            return None;
        }
        if let Some(known) = self.index.get(at) {
            return Some(*known);
        }
        let index = self.states.len();
        self.states.push(Recorded {
            at: at.clone(),
            enabled: BTreeSet::new(),
            edges: Vec::new(),
        });
        self.index.insert(at.clone(), index);
        for (set, member) in self.acceptance.iter_mut().zip(accepting) {
            if *member {
                set.insert(index);
            }
        }
        Some(index)
    }

    /// The identity number of `transition`, numbered on first sight.
    fn number(
        &mut self,
        system: &S,
        transition: &S::TransitionId,
    ) -> Result<Identity, EncodingRefusal> {
        let key = system.transition_key(transition)?;
        let next = Identity::new(u64::try_from(self.numbers.len()).unwrap_or(u64::MAX));
        let (identity, _) = self
            .numbers
            .entry(key)
            .or_insert_with(|| (next, transition.clone()));
        Ok(*identity)
    }

    /// The state `at` enables `transitions`, the transitions of its
    /// expansion (ADR-019 FS-1).
    pub(super) fn enable(
        &mut self,
        system: &S,
        at: usize,
        transitions: &[&S::TransitionId],
    ) -> Result<(), EncodingRefusal> {
        let mut enabled = BTreeSet::new();
        for transition in transitions {
            enabled.insert(self.number(system, transition)?);
        }
        if at >= self.first {
            if let Some(state) = self.states.get_mut(at) {
                state.enabled = enabled;
            }
        }
        Ok(())
    }

    /// The edge from `from` to `to` taking `transition`, or the terminal
    /// stutter step when `None`.
    pub(super) fn edge(
        &mut self,
        system: &S,
        from: usize,
        to: usize,
        transition: Option<&S::TransitionId>,
    ) -> Result<(), EncodingRefusal> {
        let label = match transition {
            Some(transition) => EdgeLabel::Transition(self.number(system, transition)?),
            None => EdgeLabel::Stutter,
        };
        if from >= self.first {
            if let Some(state) = self.states.get_mut(from) {
                state.edges.push((to, label));
            }
        }
        Ok(())
    }

    /// The recorded product graph, and the transitions it holds.
    pub(super) fn finish(self) -> Result<Finished<S>, ProductGraphError> {
        let mut graph = ProductGraph::new();
        let mut vertices: Vec<Vertex> = Vec::with_capacity(self.states.len());
        for state in &self.states {
            vertices.push(graph.add_state(state.at.clone(), state.enabled.iter().copied())?);
        }
        for (state, from) in self.states.iter().zip(&vertices) {
            for (to, label) in &state.edges {
                let to = vertices
                    .get(*to)
                    .copied()
                    .ok_or(ProductGraphError::UnknownState)?;
                graph.add_edge(*from, to, *label)?;
            }
        }
        for members in &self.acceptance {
            let members = members
                .iter()
                .map(|index| {
                    vertices
                        .get(*index)
                        .copied()
                        .ok_or(ProductGraphError::UnknownState)
                })
                .collect::<Result<Vec<_>, _>>()?;
            graph.add_acceptance_set(members)?;
        }
        let mut identities = BTreeMap::new();
        let mut transitions = Vec::with_capacity(self.numbers.len());
        for (key, (identity, transition)) in self.numbers {
            identities.insert(key, identity);
            transitions.push(transition);
        }
        Ok(Finished {
            graph,
            transitions,
            identities,
        })
    }
}

/// A finished recording.
pub(super) struct Finished<S: SubjectSystem> {
    pub(super) graph: ProductGraph,
    /// The transitions of the graph's edges and enabled sets, ascending by
    /// their FR-101 canonical bytes.
    pub(super) transitions: Vec<S::TransitionId>,
    identities: BTreeMap<Vec<u8>, Identity>,
}

impl<S: SubjectSystem> Finished<S> {
    /// The identity number `transition` carries in the graph; `None` for a
    /// transition the graph does not hold.
    pub(super) fn identity(&self, system: &S, transition: &S::TransitionId) -> Option<Identity> {
        let key = system.transition_key(transition).ok()?;
        self.identities.get(&key).copied()
    }
}
