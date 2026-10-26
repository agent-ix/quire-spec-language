// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-244: the zone certificate every `Holds` outcome of the zone search
//! carries (ADR-026 CF-1, CF-5), built from the retained symbolic graph.
//!
//! A safety or deadlock-freedom proof carries the reachability part: each
//! stored node (its model-state key, its claim-automaton state, its
//! canonical zone and whether it is `locked`) and each retained edge, a
//! step's convex part to the node that holds or covers its successor. Every
//! certificate carries `admitted`, a path of its own edges from an initial
//! node to a divergent target (ADR-026 TD-6). The component and divergence
//! parts belong to FR-240's liveness and time-lock-freedom proofs.

use std::collections::{BTreeSet, VecDeque};

use num_bigint::BigInt;
use qsl_eval::simulation::model::ModelSimState;
use qsl_foundation::digest::DigestRecord;
use qsl_route::deadlock::ItemObligation;
use quire_semantic_value::model_transition::ModelTransition;

use super::divergence::sccs;
use super::search::{Halt, Run, SymbolicEdge};
use super::Dbm;

/// FR-244's certificate of a `Holds` outcome.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ZoneCertificate<S> {
    /// The item it proves (ADR-013 O-09): its subject and kind.
    pub obligation: ItemObligation<S>,
    /// The scale factor of every zone bound (FR-238).
    pub scale: BigInt,
    /// Each clock place's `L` and `U`, in scaled units. The search uses one
    /// pair per place for every discrete state.
    pub lu_bounds: Vec<LuBounds>,
    /// The nodes, in discovery order.
    pub nodes: Vec<CertNode>,
    /// The edges, by source node in discovery order and, from each node, in
    /// canonical transition order.
    pub edges: Vec<CertEdge>,
    /// The node that holds each initial symbolic state, one per initial
    /// state that admits a valuation. FR-244 names one initial node; a
    /// subject has one per initial model state (spec gap STD-493).
    pub initial: Vec<usize>,
    /// FR-240's component numbering; `None` for a safety proof.
    pub components: Option<Vec<CertComponent>>,
    /// FR-240's divergence part; `None` but for the time-lock-freedom item.
    pub divergence: Option<Vec<DivergenceWitness>>,
    /// The witness that the subject admits a time-divergent behaviour.
    pub admitted: DivergenceWitness,
}

/// One clock place's extrapolation bounds.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LuBounds {
    /// The largest lower-bound constant the place is compared with.
    pub lower: Option<BigInt>,
    /// The largest upper-bound constant.
    pub upper: Option<BigInt>,
}

/// One certificate node.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CertNode {
    /// The digest of the model state's state key.
    pub state_key: DigestRecord,
    /// The claim-automaton state; 0 for an item that needs no automaton.
    pub automaton_state: usize,
    /// The zone, canonical, in scaled units.
    pub zone: Dbm,
    /// Whether the node violates the item only on time-locked valuations,
    /// which FR-239 continues past.
    pub locked: bool,
}

/// One certificate edge.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CertEdge {
    /// The source node.
    pub from: usize,
    /// The step's transition identity.
    pub transition: ModelTransition,
    /// The convex part of the step's guard taken.
    pub part: usize,
    /// The node whose zone holds, or under the aLU simulation covers, the
    /// successor.
    pub to: usize,
}

/// One component of FR-240's numbering.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CertComponent {
    /// The component's nodes.
    pub nodes: Vec<usize>,
}

/// A path of certificate edges to a divergent target.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DivergenceWitness {
    /// The node the path starts at.
    pub start: usize,
    /// The edges of the path, by index.
    pub path: Vec<usize>,
    /// Where it ends.
    pub target: DivergentTarget,
}

/// A divergent target.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DivergentTarget {
    /// A quiescent node, from which time diverges by the idle tail.
    Quiescent(usize),
    /// A cycle of edges, from and back to the path's last node, along which
    /// some run takes a positive total delay.
    Cycle(Vec<usize>),
}

impl ZoneCertificate<()> {
    /// This certificate naming `obligation`.
    pub(super) fn proving<S>(self, obligation: ItemObligation<S>) -> ZoneCertificate<S> {
        ZoneCertificate {
            obligation,
            scale: self.scale,
            lu_bounds: self.lu_bounds,
            nodes: self.nodes,
            edges: self.edges,
            initial: self.initial,
            components: self.components,
            divergence: self.divergence,
            admitted: self.admitted,
        }
    }
}

impl Run<'_, '_, '_> {
    /// The certificate of a completed run with no admitted violation, or
    /// `None` when no path of retained edges reaches a divergent target.
    pub(super) fn certificate<S: Clone>(
        &mut self,
        obligation: ItemObligation<S>,
        roots: &[usize],
    ) -> Result<Option<ZoneCertificate<S>>, Halt> {
        let mut nodes = Vec::with_capacity(self.nodes.len());
        for node in &self.nodes {
            let (_, state_key) = self
                .timed
                .system()
                .state_key(&node.state)
                .map_err(|_| Halt::Refusal(super::ZoneCheckRefusal::Fault("state-key")))?;
            nodes.push(CertNode {
                state_key,
                automaton_state: 0,
                zone: node.zone.clone(),
                locked: node.locked,
            });
        }
        // Edges by source in discovery order; from one source the search
        // retains them in canonical step and part order.
        let mut order: Vec<usize> = (0..self.edges.len()).collect();
        order.sort_by_key(|&edge| self.edges[edge].ends().0);
        let mut edges = Vec::with_capacity(order.len());
        let mut edge_steps = Vec::with_capacity(order.len());
        for edge in order {
            let (SymbolicEdge::Step {
                from,
                to,
                step,
                part,
            }
            | SymbolicEdge::Covering {
                from,
                to,
                step,
                part,
            }) = self.edges[edge].clone();
            let steps = self.steps_of(from)?;
            let transition = steps
                .get(step)
                .ok_or(Halt::Refusal(super::ZoneCheckRefusal::Fault("edge-step")))?
                .transition()
                .clone();
            edge_steps.push(step);
            edges.push(CertEdge {
                from,
                transition,
                part,
                to,
            });
        }
        let Some(admitted) = self.admitted(&edges, &edge_steps, roots)? else {
            return Ok(None);
        };
        Ok(Some(ZoneCertificate {
            obligation,
            scale: self.scale.clone(),
            lu_bounds: self
                .lower
                .iter()
                .zip(&self.upper)
                .map(|(lower, upper)| LuBounds {
                    lower: lower.clone(),
                    upper: upper.clone(),
                })
                .collect(),
            nodes,
            edges,
            initial: roots.to_vec(),
            components: None,
            divergence: None,
            admitted,
        }))
    }

    /// TD-6's witness over the certificate's edges: the shortest path from
    /// an initial node to a quiescent node, or else to a cycle along which
    /// a run takes a positive total delay.
    fn admitted(
        &mut self,
        edges: &[CertEdge],
        edge_steps: &[usize],
        roots: &[usize],
    ) -> Result<Option<DivergenceWitness>, Halt> {
        let size = self.nodes.len();
        let mut out: Vec<Vec<usize>> = vec![Vec::new(); size];
        for (index, edge) in edges.iter().enumerate() {
            out[edge.from].push(index);
        }
        // Breadth-first from every root at once, remembering each node's
        // first edge in.
        let mut reached: Vec<Option<Option<usize>>> = vec![None; size];
        let mut queue = VecDeque::new();
        for &root in roots {
            if reached[root].is_none() {
                reached[root] = Some(None);
                queue.push_back(root);
            }
        }
        let mut order = Vec::new();
        while let Some(at) = queue.pop_front() {
            self.poll()?;
            order.push(at);
            for &edge in &out[at] {
                let to = edges[edge].to;
                if reached[to].is_none() {
                    reached[to] = Some(Some(edge));
                    queue.push_back(to);
                }
            }
        }
        let path_to = |mut at: usize| -> (usize, Vec<usize>) {
            let mut path = Vec::new();
            while let Some(Some(edge)) = reached[at] {
                path.push(edge);
                at = edges[edge].from;
            }
            path.reverse();
            (at, path)
        };
        for &at in &order {
            let state = self.nodes[at].state.clone();
            if self.timed(self.timed.is_quiescent(&state))? {
                let (start, path) = path_to(at);
                return Ok(Some(DivergenceWitness {
                    start,
                    path,
                    target: DivergentTarget::Quiescent(at),
                }));
            }
        }
        let adjacency: Vec<Vec<usize>> = out
            .iter()
            .map(|leaving| leaving.iter().map(|&edge| edges[edge].to).collect())
            .collect();
        let reachable: BTreeSet<usize> = order.iter().copied().collect();
        let mut components = sccs(&adjacency);
        // Components in discovery order of their first node.
        for component in &mut components {
            component.sort_unstable();
        }
        components.sort_by_key(|component| component.first().copied());
        for component in components {
            let Some(&head) = component.first() else {
                continue;
            };
            if !reachable.contains(&head) {
                continue;
            }
            let members: BTreeSet<usize> = component.iter().copied().collect();
            let Some(cycle) = cycle_through(head, &members, &out, edges) else {
                continue;
            };
            if self.cycle_delays(head, &cycle, edges, edge_steps)? {
                let (start, path) = path_to(head);
                return Ok(Some(DivergenceWitness {
                    start,
                    path,
                    target: DivergentTarget::Cycle(cycle),
                }));
            }
        }
        Ok(None)
    }

    /// Whether some run around `cycle`, from a valuation of `head`'s zone,
    /// takes a positive total delay: the zone with a fresh clock `z` at 0,
    /// carried along the cycle's steps, admits `z > 0` back at `head`.
    fn cycle_delays(
        &mut self,
        head: usize,
        cycle: &[usize],
        edges: &[CertEdge],
        edge_steps: &[usize],
    ) -> Result<bool, Halt> {
        let factor = self.scale.clone();
        let mut state: ModelSimState = self.nodes[head].state.clone();
        let mut zone = self.nodes[head].zone.with_clock_at_zero();
        for &edge in cycle {
            let part = edges[edge].part;
            let step = edge_steps[edge];
            let Some(next) = self
                .successors(&state, &zone, &factor)?
                .into_iter()
                .find(|successor| successor.step == step && successor.part == part)
            else {
                return Ok(false);
            };
            state = next.state;
            zone = next.zone;
        }
        let z = zone.dim() - 1;
        Ok(!zone.is_empty()
            && match zone.bound(z, 0)? {
                super::Bound::Infinite => true,
                super::Bound::Finite { value, .. } => *value > BigInt::from(0),
            })
    }
}

/// A cycle of edges inside `members` from `head` back to `head`: the
/// shortest, by breadth-first search.
fn cycle_through(
    head: usize,
    members: &BTreeSet<usize>,
    out: &[Vec<usize>],
    edges: &[CertEdge],
) -> Option<Vec<usize>> {
    let mut first: std::collections::BTreeMap<usize, usize> = std::collections::BTreeMap::new();
    let mut queue = VecDeque::from([head]);
    while let Some(at) = queue.pop_front() {
        for &edge in &out[at] {
            let to = edges[edge].to;
            if !members.contains(&to) {
                continue;
            }
            if to == head {
                let mut cycle = vec![edge];
                let mut back = at;
                while back != head {
                    let previous = *first.get(&back)?;
                    cycle.push(previous);
                    back = edges[previous].from;
                }
                cycle.reverse();
                return Some(cycle);
            }
            if let std::collections::btree_map::Entry::Vacant(slot) = first.entry(to) {
                slot.insert(edge);
                queue.push_back(to);
            }
        }
    }
    None
}
