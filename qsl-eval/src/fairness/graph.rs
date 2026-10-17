// SPDX-License-Identifier: AGPL-3.0-or-later
//! The graph the fairness kernels read.

use std::collections::BTreeSet;

use crate::fairness::error::FairnessError;

/// One vertex of a [`Graph`], by its index in discovery order.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Vertex(usize);

impl Vertex {
    /// The vertex at `index`.
    pub(crate) const fn new(index: usize) -> Self {
        Self(index)
    }

    /// The vertex's index.
    pub const fn index(self) -> usize {
        self.0
    }
}

/// One transition identity, as the caller numbers them: the operation, its
/// receiver and its argument vector, which a `whole` constraint groups and an
/// `each` constraint separates.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Identity(u64);

impl Identity {
    /// The identity numbered `value`.
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// The identity's number.
    pub const fn value(self) -> u64 {
        self.0
    }
}

/// What an edge takes.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum EdgeLabel {
    /// A transition identity.
    Transition(Identity),
    /// The terminal stutter step (ADR-018 SM-4): it belongs to no
    /// constraint and enables none.
    Stutter,
}

/// One edge out of a vertex.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Edge {
    /// The vertex the edge leads to.
    pub target: Vertex,
    /// What the edge takes.
    pub label: EdgeLabel,
}

/// A directed graph with the annotations fairness reads. Vertices are
/// numbered in the order they are added, and each vertex's edges are kept in
/// the order they are added, which the caller makes the canonical successor
/// order: every order the kernels report follows these two orders, never a
/// hash map's.
#[derive(Clone, Debug, Default)]
pub struct Graph {
    edges: Vec<Vec<Edge>>,
    enabled: Vec<BTreeSet<Identity>>,
    acceptance: Vec<BTreeSet<Vertex>>,
}

impl Graph {
    /// An empty graph.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a vertex at which exactly the identities in `enabled` are enabled
    /// (ADR-019 FS-1: the labels of the model state's transitions, whether
    /// or not the automaton gives one a move).
    pub fn add_vertex(&mut self, enabled: impl IntoIterator<Item = Identity>) -> Vertex {
        let vertex = Vertex::new(self.edges.len());
        self.edges.push(Vec::new());
        self.enabled.push(enabled.into_iter().collect());
        vertex
    }

    /// Add an edge from `from` to `to`, after the edges already out of
    /// `from`.
    ///
    /// # Errors
    ///
    /// [`FairnessError::UnknownVertex`] when either vertex has not been
    /// added.
    pub fn add_edge(
        &mut self,
        from: Vertex,
        to: Vertex,
        label: EdgeLabel,
    ) -> Result<(), FairnessError> {
        self.check(to)?;
        self.check(from)?;
        if let Some(out) = self.edges.get_mut(from.index()) {
            out.push(Edge { target: to, label });
        }
        Ok(())
    }

    /// Add an acceptance set of the property automaton.
    ///
    /// # Errors
    ///
    /// [`FairnessError::UnknownVertex`] when a member has not been added.
    pub fn add_acceptance_set(
        &mut self,
        members: impl IntoIterator<Item = Vertex>,
    ) -> Result<(), FairnessError> {
        let set: BTreeSet<Vertex> = members.into_iter().collect();
        for vertex in &set {
            self.check(*vertex)?;
        }
        self.acceptance.push(set);
        Ok(())
    }

    /// The number of vertices.
    pub fn vertex_count(&self) -> usize {
        self.edges.len()
    }

    /// Every vertex, in discovery order.
    pub fn vertices(&self) -> impl Iterator<Item = Vertex> {
        (0..self.edges.len()).map(Vertex::new)
    }

    /// The edges out of `vertex`, in the order they were added; none for a
    /// vertex that is not in the graph.
    pub fn edges_from(&self, vertex: Vertex) -> &[Edge] {
        match self.edges.get(vertex.index()) {
            Some(out) => out.as_slice(),
            None => &[],
        }
    }

    /// The identities enabled at `vertex`.
    pub fn enabled(&self, vertex: Vertex) -> Option<&BTreeSet<Identity>> {
        self.enabled.get(vertex.index())
    }

    /// The acceptance sets, in the order they were added.
    pub fn acceptance_sets(&self) -> &[BTreeSet<Vertex>] {
        &self.acceptance
    }

    /// Refuse a vertex that is not in the graph.
    pub(crate) fn check(&self, vertex: Vertex) -> Result<(), FairnessError> {
        if vertex.index() < self.edges.len() {
            Ok(())
        } else {
            Err(FairnessError::UnknownVertex { vertex })
        }
    }
}
