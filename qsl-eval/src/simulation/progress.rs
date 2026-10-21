// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-027 PB-3 and FR-338 "Protocol non-progress acceptance": the loops of a
//! product whose every step leaves interval distance unchanged.
//!
//! Counted bounds do not bound the number of steps that do not count. A
//! product that holds an interval debt open on a cycle of such steps is a
//! behaviour that never meets the interval, and it violates the claim
//! exactly when it is a fair behaviour: the scheduler constraints of
//! FR-212 and the claim's authored constraints, read over the complete
//! subject at each state, must hold on the loop. A counted step that leaves
//! the cycle is enabled and never taken, so a weak constraint of a thread
//! that can still move excludes the loop; an adversarial scheduler does not.
//!
//! [`ZeroProgress`] collects the debt-holding states of a product, the steps
//! between them that do not count and the transitions each state enables,
//! and asks the fairness kernels for a fair component. EN-1 builds the
//! lasso of that component, and the core's closure checker rejects a proof
//! that has one, from the same search.

use std::collections::{BTreeMap, BTreeSet};

use quire_exact::Cancel;
use quire_semantic_value::fairness::FairnessConstraint;

use crate::fairness::{
    analyze, Analysis, Constraint, EdgeLabel, FairnessError, Graph, Identity, Vertex,
};
use crate::simulation::protocol::ProtocolFairnessError;
use crate::simulation::{EncodingRefusal, SubjectSystem};

/// Why the search for a fair loop could not run to an answer.
#[derive(Debug, thiserror::Error)]
pub enum ZeroProgressError {
    /// A transition identity has no canonical encoding.
    #[error(transparent)]
    Encoding(#[from] EncodingRefusal),
    /// A fairness constraint names an operation the subject does not admit.
    #[error(transparent)]
    Fairness(#[from] ProtocolFairnessError),
    /// The caller's `Cancel` handle was cancelled.
    #[error("the run was cancelled")]
    Cancelled,
    /// The graph the search built is malformed.
    #[error("the zero-progress graph names a vertex it does not hold")]
    Graph,
}

/// A component of the zero-progress graph that holds a fair cycle, and what
/// a caller needs to walk it.
#[derive(Debug)]
pub struct FairLoop<T> {
    /// The graph, whose vertices are numbered as [`ZeroProgress::vertex`]
    /// returned them and whose edge identities index [`Self::transitions`].
    pub graph: Graph,
    /// The kernels' constraints the component satisfies.
    pub constraints: Vec<Constraint>,
    /// The component's vertices.
    pub component: BTreeSet<Vertex>,
    /// The edge identities a visibility refinement removed from the
    /// component.
    pub removed: BTreeSet<Identity>,
    /// The component's first vertex, the canonical entry of its loop.
    pub entry: Vertex,
    /// The transitions the graph's identities name, by identity.
    pub transitions: Vec<T>,
}

/// The states of a product that hold an interval debt open and the steps
/// between them that do not count.
#[derive(Debug)]
pub struct ZeroProgress<S: SubjectSystem> {
    transitions: Vec<S::TransitionId>,
    numbers: BTreeMap<Vec<u8>, usize>,
    enabled: Vec<Vec<usize>>,
    edges: Vec<(usize, usize, usize)>,
}

impl<S: SubjectSystem> Default for ZeroProgress<S> {
    fn default() -> Self {
        Self {
            transitions: Vec::new(),
            numbers: BTreeMap::new(),
            enabled: Vec::new(),
            edges: Vec::new(),
        }
    }
}

impl<S: SubjectSystem> ZeroProgress<S> {
    /// An empty graph.
    pub fn new() -> Self {
        Self::default()
    }

    /// The identity of `transition`, numbered on first sight by its canonical
    /// bytes.
    fn number(
        &mut self,
        system: &S,
        transition: &S::TransitionId,
    ) -> Result<usize, EncodingRefusal> {
        let key = system.transition_key(transition)?;
        let next = self.transitions.len();
        let number = *self.numbers.entry(key).or_insert(next);
        if number == next {
            self.transitions.push(transition.clone());
        }
        Ok(number)
    }

    /// Add a state that enables `enabled`, the transitions of its complete
    /// expansion, and return its number.
    ///
    /// # Errors
    ///
    /// [`EncodingRefusal`] when a transition has no canonical encoding.
    pub fn vertex<'t>(
        &mut self,
        system: &S,
        enabled: impl IntoIterator<Item = &'t S::TransitionId>,
    ) -> Result<usize, EncodingRefusal>
    where
        S::TransitionId: 't,
    {
        let mut numbers = BTreeSet::new();
        for transition in enabled {
            numbers.insert(self.number(system, transition)?);
        }
        self.enabled.push(numbers.into_iter().collect());
        Ok(self.enabled.len() - 1)
    }

    /// Add the step `transition` from state `from` to state `to`.
    ///
    /// # Errors
    ///
    /// [`EncodingRefusal`] when the transition has no canonical encoding.
    pub fn edge(
        &mut self,
        system: &S,
        from: usize,
        to: usize,
        transition: &S::TransitionId,
    ) -> Result<(), EncodingRefusal> {
        let number = self.number(system, transition)?;
        self.edges.push((from, to, number));
        Ok(())
    }

    /// Whether any step was added.
    pub fn is_empty(&self) -> bool {
        self.edges.is_empty()
    }

    /// The first component of the graph that holds a cycle fair under the
    /// subject's scheduler constraints and the `authored` ones (ADR-019),
    /// or `None` when no cycle is fair.
    ///
    /// # Errors
    ///
    /// [`ZeroProgressError`] when a constraint names an unknown operation, a
    /// transition cannot be encoded, or `cancel` is cancelled.
    pub fn fair_loop(
        self,
        system: &S,
        authored: &[FairnessConstraint],
        cancel: &Cancel,
    ) -> Result<Option<FairLoop<S::TransitionId>>, ZeroProgressError> {
        if self.edges.is_empty() {
            return Ok(None);
        }
        let identity = |number: usize| Identity::new(u64::try_from(number).unwrap_or(u64::MAX));
        let mut graph = Graph::new();
        let vertices: Vec<Vertex> = self
            .enabled
            .iter()
            .map(|enabled| graph.add_vertex(enabled.iter().map(|number| identity(*number))))
            .collect();
        let vertex = |number: usize| {
            vertices
                .get(number)
                .copied()
                .ok_or(ZeroProgressError::Graph)
        };
        for (from, to, number) in &self.edges {
            graph
                .add_edge(
                    vertex(*from)?,
                    vertex(*to)?,
                    EdgeLabel::Transition(identity(*number)),
                )
                .map_err(|_| ZeroProgressError::Graph)?;
        }
        let numbers = &self.numbers;
        let constraints =
            system.progress_constraints(authored, &self.transitions, &|transition| {
                let key = system.transition_key(transition)?;
                numbers
                    .get(&key)
                    .map(|number| identity(*number))
                    .ok_or(ZeroProgressError::Graph)
            })?;
        let all: BTreeSet<Vertex> = graph.vertices().collect();
        let analyses = match analyze(&graph, &all, &constraints, cancel) {
            Ok(analyses) => analyses,
            Err(FairnessError::Cancelled) => return Err(ZeroProgressError::Cancelled),
            Err(FairnessError::UnknownVertex { .. } | FairnessError::EmptyLoop) => {
                return Err(ZeroProgressError::Graph)
            }
        };
        let Some(passing) = Analysis::first_fair_in(&analyses) else {
            return Ok(None);
        };
        let component: BTreeSet<Vertex> = passing.vertices().iter().copied().collect();
        let entry = passing
            .vertices()
            .first()
            .copied()
            .ok_or(ZeroProgressError::Graph)?;
        let removed = passing.removed().clone();
        Ok(Some(FairLoop {
            graph,
            constraints,
            component,
            removed,
            entry,
            transitions: self.transitions,
        }))
    }
}
