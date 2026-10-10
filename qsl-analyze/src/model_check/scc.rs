// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-018 EN-1: iterative SCC decomposition over the actual retained edges.

use super::{ProductStateId, RetainedGraph};

/// One strongly connected component of a retained graph.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Component {
    /// Member handles in ascending discovery order.
    pub states: Vec<ProductStateId>,
    /// More than one member or a retained self edge (parallel edges allowed).
    pub cyclic: bool,
}

impl<S, T, F> RetainedGraph<S, T, F> {
    /// Decompose all retained states, including disconnected and frontier states.
    ///
    /// Two iterative Kosaraju passes use O(V + E) time and storage and no
    /// recursive call stack. Components are enumerated by their first discovered
    /// member, as EN-1 requires; this is not certificate topological ordering.
    /// A partial graph's components describe only its retained edges.
    pub fn components(&self) -> Vec<Component> {
        let mut visited = vec![false; self.states.len()];
        let mut finished = Vec::with_capacity(self.states.len());
        for root in 0..self.states.len() {
            if visited[root] {
                continue;
            }
            visited[root] = true;
            let mut stack = vec![(root, 0)];
            while let Some((node, next)) = stack.last_mut() {
                // All indices are constructed privately from admitted endpoints.
                if let Some(edge) = self.outgoing[*node].get(*next) {
                    *next += 1;
                    let target = self.edges[*edge].target.0;
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
        let mut incoming = vec![Vec::new(); self.states.len()];
        for edge in &self.edges {
            incoming[edge.target.0].push(edge.source.0);
        }
        visited.fill(false);
        let mut membership = vec![0; self.states.len()];
        let mut count = 0;
        for root in finished.into_iter().rev() {
            if visited[root] {
                continue;
            }
            let mut stack = vec![root];
            visited[root] = true;
            while let Some(node) = stack.pop() {
                membership[node] = count;
                for &source in &incoming[node] {
                    if !visited[source] {
                        visited[source] = true;
                        stack.push(source);
                    }
                }
            }
            count += 1;
        }
        // A discovery-order scan orders both components and their members
        // without a comparison sort, preserving the linear SCC bound.
        let mut slots = vec![None; count];
        let mut components: Vec<Component> = Vec::with_capacity(count);
        for (node, group) in membership.into_iter().enumerate() {
            let slot = *slots[group].get_or_insert_with(|| {
                components.push(Component {
                    states: Vec::new(),
                    cyclic: false,
                });
                components.len() - 1
            });
            components[slot].states.push(ProductStateId(node));
            components[slot].cyclic |= components[slot].states.len() > 1
                || self.outgoing[node]
                    .iter()
                    .any(|edge| self.edges[*edge].target.0 == node);
        }
        components
    }
}
