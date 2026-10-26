// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-239 "Admitted behaviour" and "Vacuity", by FR-240's test: whether
//! some valuation of a symbolic state has a time-divergent continuation.
//!
//! Under a dense source, time diverges along a run exactly when a fresh
//! clock `z`, compared with 1 only and reset by a silent `z >= 1` step that
//! changes nothing else, can be reset infinitely often (Tripakis). The test
//! explores the zone graph of the subject with `z` and that step added,
//! from the symbolic state with `z = 0`, and asks whether some reachable
//! strongly connected component holds a `z` reset. Nodes are identified by
//! their model state and their aLU abstraction (mutual aLU inclusion, with
//! `L_z = U_z = 1`), never merged by one-sided coverage, so a component of
//! this graph is one of the abstract zone graph, which preserves the Büchi
//! condition (Herbreteau, Srivathsan and Walukiewicz).
//!
//! Under a tick source time is the count of tick steps (FR-231 "Digital
//! time"): a run diverges when it takes infinitely many, or ends at a
//! terminal state, whose stutter step counts as a tick. The test asks for a
//! reachable component with a tick step, or a reachable terminal node.

use std::collections::{BTreeMap, VecDeque};

use num_bigint::BigInt;
use qsl_eval::simulation::model::ModelSimState;

use super::search::{Halt, Run};
use super::{Bound, Dbm, ZoneCheckLimit, ZoneStop};

/// A node of the divergence graph.
struct Node {
    state: ModelSimState,
    zone: Dbm,
}

/// An edge of the divergence graph.
struct Edge {
    to: usize,
    /// Whether the edge counts towards divergence: a `z` reset, or a tick.
    accepting: bool,
}

/// What the test finds.
pub(super) struct Escape {
    /// Whether some valuation of the symbolic state diverges.
    pub diverges: bool,
    /// Under a dense source, the model steps from the symbolic state to the
    /// first quiescent state it reaches, each as the source's model state,
    /// the step's index and the part's index; every valuation of a
    /// quiescent state diverges by its idle tail.
    pub to_quiescent: Option<Vec<(ModelSimState, usize, usize)>>,
}

impl Run<'_, '_, '_> {
    /// FR-240's test from `(state, zone)`, bounds in units of `1 / factor`
    /// with `lower` and `upper` the places' `L` and `U` in those units.
    pub(super) fn escape(
        &mut self,
        state: &ModelSimState,
        zone: &Dbm,
        factor: &BigInt,
        lower: &[Option<BigInt>],
        upper: &[Option<BigInt>],
    ) -> Result<Escape, Halt> {
        let dense = self.timed.tick_period().is_none();
        let (start, lower, upper) = if dense {
            let mut lower = lower.to_vec();
            let mut upper = upper.to_vec();
            lower.push(Some(factor.clone()));
            upper.push(Some(factor.clone()));
            (zone.with_clock_at_zero(), lower, upper)
        } else {
            (zone.clone(), lower.to_vec(), upper.to_vec())
        };
        let mut start = start;
        // `z` counts from the symbolic state's entry; its own delays are
        // then taken.
        self.settle_at(state, &mut start, factor)?;
        let z = start.dim() - 1;
        let mut nodes = vec![Node {
            state: state.clone(),
            zone: start,
        }];
        let mut keys: BTreeMap<Vec<u8>, Vec<usize>> = BTreeMap::new();
        keys.entry(self.key(state)?).or_default().push(0);
        let mut out: Vec<Vec<Edge>> = vec![Vec::new()];
        // Each node's first edge in: its source and the model step and part,
        // `None` for the `z` reset.
        #[allow(clippy::type_complexity, reason = "a source and an optional step")]
        let mut parent: Vec<Option<(usize, Option<(usize, usize)>)>> = vec![None];
        let mut quiescent = None;
        let mut queue = VecDeque::from([0_usize]);
        while let Some(at) = queue.pop_front() {
            self.poll()?;
            let node_state = nodes[at].state.clone();
            let node_zone = nodes[at].zone.clone();
            if dense && quiescent.is_none() && self.timed(self.timed.is_quiescent(&node_state))? {
                quiescent = Some(at);
            }
            let mut targets = Vec::new();
            for successor in self.successors(&node_state, &node_zone, factor)? {
                targets.push((
                    successor.state,
                    successor.zone,
                    !dense && successor.tick,
                    Some((successor.step, successor.part)),
                ));
            }
            if dense {
                // The silent `z >= 1` step resets `z`.
                let mut reset = node_zone.clone();
                reset.constrain(0, z, Bound::le(-factor.clone()))?;
                if !reset.is_empty() {
                    reset.reset(z, &BigInt::from(0))?;
                    self.settle_at(&node_state, &mut reset, factor)?;
                    if !reset.is_empty() {
                        targets.push((node_state.clone(), reset, true, None));
                    }
                }
            } else if targets.is_empty() {
                // A terminal state's stutter step is a tick.
                targets.push((node_state.clone(), node_zone.clone(), true, None));
            }
            for (target, target_zone, accepting, step) in targets {
                let key = self.key(&target)?;
                let mut found = None;
                for &candidate in keys.get(&key).map_or(&[][..], Vec::as_slice) {
                    let other = &nodes[candidate].zone;
                    if target_zone.alu_covered_by(other, &lower, &upper)?
                        && other.alu_covered_by(&target_zone, &lower, &upper)?
                    {
                        found = Some(candidate);
                        break;
                    }
                }
                let to = match found {
                    Some(to) => to,
                    None => {
                        if u64::try_from(nodes.len()).unwrap_or(u64::MAX)
                            >= self.limits.max_symbolic_states
                        {
                            return Err(self.stop(ZoneStop::LimitReached(
                                ZoneCheckLimit::MaxSymbolicStates(self.limits.max_symbolic_states),
                            )));
                        }
                        let index = nodes.len();
                        nodes.push(Node {
                            state: target,
                            zone: target_zone,
                        });
                        keys.entry(key).or_default().push(index);
                        out.push(Vec::new());
                        parent.push(Some((at, step)));
                        queue.push_back(index);
                        index
                    }
                };
                out[at].push(Edge { to, accepting });
            }
        }
        let adjacency: Vec<Vec<usize>> = out
            .iter()
            .map(|edges| edges.iter().map(|edge| edge.to).collect())
            .collect();
        let mut component_of = vec![usize::MAX; nodes.len()];
        for (index, component) in sccs(&adjacency).into_iter().enumerate() {
            for member in component {
                component_of[member] = index;
            }
        }
        let diverges = out.iter().enumerate().any(|(from, edges)| {
            edges
                .iter()
                .any(|edge| edge.accepting && component_of[from] == component_of[edge.to])
        });
        let to_quiescent = quiescent.map(|mut at| {
            let mut path = Vec::new();
            while let Some((from, step)) = parent[at] {
                // A `z` reset only lets time pass.
                if let Some((step, part)) = step {
                    path.push((nodes[from].state.clone(), step, part));
                }
                at = from;
            }
            path.reverse();
            path
        });
        Ok(Escape {
            diverges,
            to_quiescent,
        })
    }
}

/// The strongly connected components of the graph `out` (node `i`'s edges
/// lead to `out[i]`), by Tarjan's algorithm run without recursion.
pub(super) fn sccs(out: &[Vec<usize>]) -> Vec<Vec<usize>> {
    let size = out.len();
    let mut index = vec![usize::MAX; size];
    let mut low = vec![0_usize; size];
    let mut on_stack = vec![false; size];
    let mut stack = Vec::new();
    let mut next = 0_usize;
    let mut found = Vec::new();
    for root in 0..size {
        if index[root] != usize::MAX {
            continue;
        }
        let mut frames = vec![(root, 0_usize)];
        index[root] = next;
        low[root] = next;
        next += 1;
        stack.push(root);
        on_stack[root] = true;
        while let Some(&mut (at, ref mut visited)) = frames.last_mut() {
            if let Some(&to) = out[at].get(*visited) {
                *visited += 1;
                if index[to] == usize::MAX {
                    index[to] = next;
                    low[to] = next;
                    next += 1;
                    stack.push(to);
                    on_stack[to] = true;
                    frames.push((to, 0));
                } else if on_stack[to] {
                    low[at] = low[at].min(index[to]);
                }
                continue;
            }
            frames.pop();
            if let Some(&(parent, _)) = frames.last() {
                low[parent] = low[parent].min(low[at]);
            }
            if low[at] == index[at] {
                let mut component = Vec::new();
                while let Some(member) = stack.pop() {
                    on_stack[member] = false;
                    component.push(member);
                    if member == at {
                        break;
                    }
                }
                found.push(component);
            }
        }
    }
    found
}
