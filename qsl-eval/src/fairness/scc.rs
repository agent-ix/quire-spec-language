// SPDX-License-Identifier: AGPL-3.0-or-later
//! Iterative strongly connected components of a subgraph.

use std::collections::BTreeSet;

use quire_exact::Cancel;

use crate::fairness::error::{charge, FairnessError};
use crate::fairness::graph::{Graph, Vertex};

/// One strongly connected component.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Component {
    vertices: Vec<Vertex>,
    has_edge: bool,
}

impl Component {
    /// The component's vertices, ascending.
    pub fn vertices(&self) -> &[Vertex] {
        &self.vertices
    }

    /// Whether some edge of the graph runs between two vertices of the
    /// component.
    pub const fn has_edge(&self) -> bool {
        self.has_edge
    }

    /// Whether the component has no edge: a single vertex with no
    /// self-loop, which holds no cycle.
    pub const fn is_trivial(&self) -> bool {
        !self.has_edge
    }
}

/// The depth-first walk's bookkeeping, indexed by vertex index. Every index
/// is below the graph's vertex count: `components` checks `within` against
/// it and `Graph::add_edge` admits only existing vertices.
struct Walk {
    discovery: Vec<Option<usize>>,
    low: Vec<usize>,
    on_stack: Vec<bool>,
    stack: Vec<usize>,
    next: usize,
}

impl Walk {
    fn new(count: usize) -> Self {
        Self {
            discovery: vec![None; count],
            low: vec![0; count],
            on_stack: vec![false; count],
            stack: Vec::new(),
            next: 0,
        }
    }

    fn seen(&self, vertex: usize) -> bool {
        self.discovery[vertex].is_some()
    }

    fn open(&mut self, vertex: usize) {
        self.discovery[vertex] = Some(self.next);
        self.low[vertex] = self.next;
        self.next = self.next.saturating_add(1);
        self.stack.push(vertex);
        self.on_stack[vertex] = true;
    }

    /// An edge from `vertex` to a vertex the walk has already seen.
    fn back_edge(&mut self, vertex: usize, target: usize) {
        if self.on_stack[target] {
            if let Some(order) = self.discovery[target] {
                self.low[vertex] = self.low[vertex].min(order);
            }
        }
    }

    /// `child` finished: its low link reaches its parent.
    fn finish_child(&mut self, parent: usize, child: usize) {
        self.low[parent] = self.low[parent].min(self.low[child]);
    }

    /// `vertex` finished: when it is the root of a component, that
    /// component's vertices, ascending.
    fn pop_component(&mut self, vertex: usize) -> Option<Vec<usize>> {
        if self.discovery[vertex] != Some(self.low[vertex]) {
            return None;
        }
        let mut members = Vec::new();
        while let Some(member) = self.stack.pop() {
            self.on_stack[member] = false;
            members.push(member);
            if member == vertex {
                break;
            }
        }
        members.sort_unstable();
        Some(members)
    }
}

/// The strongly connected components of the subgraph of `graph` induced by
/// `within`, by Tarjan's algorithm with an explicit stack, so a long chain
/// of vertices cannot overflow the call stack.
///
/// The components are in discovery order: ascending by their least vertex,
/// and so by the order the caller discovered them. Vertices outside `within`
/// and edges that leave it are ignored. Each vertex entered charges `cancel`
/// first, as does the call.
///
/// # Errors
///
/// [`FairnessError::UnknownVertex`] when `within` names a vertex not in the
/// graph; [`FairnessError::Cancelled`] when `cancel` is cancelled.
pub fn components(
    graph: &Graph,
    within: &BTreeSet<Vertex>,
    cancel: &Cancel,
) -> Result<Vec<Component>, FairnessError> {
    charge(cancel)?;
    for vertex in within {
        graph.require_vertex(*vertex)?;
    }
    let mut walk = Walk::new(graph.vertex_count());
    let mut found: Vec<Vec<usize>> = Vec::new();
    for root in within {
        if walk.seen(root.index()) {
            continue;
        }
        charge(cancel)?;
        walk.open(root.index());
        let mut frames: Vec<(usize, usize)> = vec![(root.index(), 0)];
        while let Some(&(vertex, position)) = frames.last() {
            if let Some(edge) = graph.edges_from(Vertex::new(vertex)).get(position) {
                if let Some(frame) = frames.last_mut() {
                    frame.1 = position.saturating_add(1);
                }
                if !within.contains(&edge.target) {
                    continue;
                }
                let target = edge.target.index();
                if walk.seen(target) {
                    walk.back_edge(vertex, target);
                } else {
                    charge(cancel)?;
                    walk.open(target);
                    frames.push((target, 0));
                }
            } else {
                frames.pop();
                if let Some(&(parent, _)) = frames.last() {
                    walk.finish_child(parent, vertex);
                }
                if let Some(members) = walk.pop_component(vertex) {
                    found.push(members);
                }
            }
        }
    }
    found.sort_unstable();
    Ok(found
        .into_iter()
        .map(|members| component_of(graph, &members))
        .collect())
}

/// The component holding `members`, ascending.
fn component_of(graph: &Graph, members: &[usize]) -> Component {
    let set: BTreeSet<usize> = members.iter().copied().collect();
    let has_edge = members.iter().any(|member| {
        graph
            .edges_from(Vertex::new(*member))
            .iter()
            .any(|edge| set.contains(&edge.target.index()))
    });
    Component {
        vertices: members.iter().copied().map(Vertex::new).collect(),
        has_edge,
    }
}
