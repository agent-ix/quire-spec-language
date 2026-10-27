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

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use num_bigint::BigInt;
use qsl_eval::simulation::model::ModelSimState;

use super::search::{Halt, Run};
use super::{Bound, Dbm};

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
    /// The model step and part it takes; `None` for a `z` reset or a
    /// terminal stutter.
    step: Option<(usize, usize)>,
}

/// One move of a divergence witness: a model step's index and part's
/// index, or `None` where only time passes (a `z` reset, or a terminal
/// state's stutter under a tick source).
pub(super) type Move = Option<(usize, usize)>;

/// The evidence the test accepted: the moves from the symbolic state to a
/// divergent target.
#[derive(Clone, Debug)]
pub(super) struct EscapeWitness {
    /// The moves from the symbolic state to the target.
    pub path: Vec<Move>,
    /// The target.
    pub target: EscapeTarget,
}

/// The divergent target a witness reaches.
#[derive(Clone, Debug)]
pub(super) enum EscapeTarget {
    /// Under a dense source, a quiescent state.
    Quiescent,
    /// Under a tick source, a terminal state, whose stutter step counts as
    /// a tick (FR-231 "Digital time").
    Terminal,
    /// A cycle back to the path's end through an accepting edge of its
    /// strongly connected component: a `z` reset under a dense source, a
    /// tick under a tick source, so each repetition lets time advance.
    Cycle(Vec<Move>),
}

/// What the test finds.
pub(super) struct Escape {
    /// Whether some valuation of the symbolic state diverges.
    pub diverges: bool,
    /// Under a dense source, for each quiescent node the test reaches in
    /// breadth-first order, the model steps from the symbolic state to it,
    /// each as the source's model state, the step's index and the part's
    /// index; every valuation of a quiescent state diverges by its idle
    /// tail.
    #[allow(clippy::type_complexity, reason = "paths of (state, step, part)")]
    pub to_quiescent: Vec<Vec<(ModelSimState, usize, usize)>>,
    /// The evidence of `diverges`; `None` when the test found none.
    pub witness: Option<EscapeWitness>,
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
        self.charge_state(&start)?;
        self.escape_state();
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
        let mut quiescent = Vec::new();
        let mut queue = VecDeque::from([0_usize]);
        while let Some(at) = queue.pop_front() {
            self.poll()?;
            let node_state = nodes[at].state.clone();
            let node_zone = nodes[at].zone.clone();
            if dense && self.timed(self.timed.is_quiescent(&node_state))? {
                quiescent.push(at);
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
                        self.charge_state(&target_zone)?;
                        self.escape_state();
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
                self.charge_edge()?;
                self.escape_edge();
                out[at].push(Edge {
                    to,
                    accepting,
                    step,
                });
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
        let accepting = out.iter().enumerate().find_map(|(from, edges)| {
            edges
                .iter()
                .find(|edge| edge.accepting && component_of[from] == component_of[edge.to])
                .map(|edge| (from, edge))
        });
        let diverges = accepting.is_some();
        let moves_to = |mut at: usize| -> Vec<Move> {
            let mut path = Vec::new();
            while let Some((from, step)) = parent[at] {
                path.push(step);
                at = from;
            }
            path.reverse();
            path
        };
        // The target: a quiescent state under a dense source, a terminal one
        // under a tick source, each first in breadth-first order; or else
        // the first accepting edge's cycle.
        let terminal = (!dense)
            .then(|| {
                (0..nodes.len()).find(|&at| {
                    out[at]
                        .iter()
                        .all(|edge| edge.step.is_none() && edge.to == at)
                        && !out[at].is_empty()
                })
            })
            .flatten();
        let witness = if let Some(&at) = quiescent.first() {
            Some(EscapeWitness {
                path: moves_to(at),
                target: EscapeTarget::Quiescent,
            })
        } else if let Some(at) = terminal {
            Some(EscapeWitness {
                path: moves_to(at),
                target: EscapeTarget::Terminal,
            })
        } else if let Some((from, edge)) = accepting {
            let component = component_of[from];
            // The cycle leaves `from` by the accepting edge and returns to
            // it inside the component.
            let back = moves_within(&out, &component_of, component, edge.to, from);
            back.map(|back| {
                let mut cycle = vec![edge.step];
                cycle.extend(back);
                EscapeWitness {
                    path: moves_to(from),
                    target: EscapeTarget::Cycle(cycle),
                }
            })
        } else {
            None
        };
        let to_quiescent = quiescent
            .into_iter()
            .map(|mut at| {
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
            })
            .collect();
        Ok(Escape {
            diverges,
            to_quiescent,
            witness,
        })
    }
}

/// The moves of a shortest path from `start` to `end` inside `component`.
fn moves_within(
    out: &[Vec<Edge>],
    component_of: &[usize],
    component: usize,
    start: usize,
    end: usize,
) -> Option<Vec<Move>> {
    let mut first: BTreeMap<usize, (usize, Move)> = BTreeMap::new();
    let mut queue = VecDeque::from([start]);
    let mut seen = BTreeSet::from([start]);
    while let Some(at) = queue.pop_front() {
        if at == end {
            let mut moves = Vec::new();
            let mut back = at;
            while back != start {
                let (from, step) = *first.get(&back)?;
                moves.push(step);
                back = from;
            }
            moves.reverse();
            return Some(moves);
        }
        for edge in &out[at] {
            if component_of[edge.to] == component && seen.insert(edge.to) {
                first.insert(edge.to, (at, edge.step));
                queue.push_back(edge.to);
            }
        }
    }
    None
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
