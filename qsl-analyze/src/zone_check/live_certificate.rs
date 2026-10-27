// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-244 over FR-240's liveness product: the certificate of a liveness
//! `Holds` (ADR-026 CF-2).
//!
//! The certificate's nodes and edges are every instance's product, in
//! instance order and each in discovery order: a node carries its instance
//! and entry marks, an edge its step and part or its silent move (a delay
//! into the next cell, or the divergence clock's reset). Its `components`
//! number the strongly connected components the fairness kernels found,
//! topologically (every edge goes to an equal or greater index, ties broken
//! by least node), each with the reason it holds no fair accepting cycle:
//! no cycle, a missing acceptance set, an unfair weak constraint, or unfair
//! strong constraints and the nested numbering of the rest. Its `admitted`
//! part is a path in the first instance's product to a quiescent (dense) or
//! terminal (tick) node, or a cycle through a divergence reset or a tick.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use num_bigint::BigInt;
use qsl_eval::fairness::{Analysis, Outcome};
use qsl_replay::{
    AcceptanceSet, CertEdge, CertNode, ComponentReason, Components, DivergenceWitness,
    DivergentTarget, LuBounds, ProductMark, SilentMove,
};
use quire_exact::{Integer, Rational};

use super::certificate::{core_zone, Proof};
use super::divergence::sccs;
use super::liveness::{InstanceProof, Product, Via};
use super::search::{Halt, Run};
use super::ZoneCheckRefusal;

impl Run<'_, '_, '_> {
    /// The certificate of a liveness `Holds` over `proofs`, each
    /// instance's product with no fair accepting cycle.
    pub(super) fn live_certificate(&mut self, proofs: Vec<InstanceProof>) -> Result<Proof, Halt> {
        let total: usize = proofs
            .iter()
            .map(|proof| proof.product.vertices.len())
            .sum();
        let mut nodes = Vec::with_capacity(total);
        let mut edges = Vec::new();
        let mut initial = Vec::new();
        let mut components = Components {
            numbering: vec![None; total],
            reasons: Vec::new(),
        };
        // Each instance's product edges as certificate edges, by index.
        let mut placed: Vec<Vec<usize>> = Vec::with_capacity(proofs.len());
        let mut offset = 0;
        for (instance, proof) in proofs.iter().enumerate() {
            let product = &proof.product;
            for (index, vertex) in product.vertices.iter().enumerate() {
                self.poll()?;
                let (_, state_key) = self
                    .timed
                    .system()
                    .state_key(&vertex.state)
                    .map_err(|_| Halt::Refusal(ZoneCheckRefusal::Fault("state-key")))?;
                nodes.push(CertNode {
                    state_key,
                    automaton_state: vertex.q.index(),
                    zone: core_zone(&vertex.zone)?,
                    locked: false,
                    product: Some(ProductMark {
                        instance,
                        positioned: vertex.positioned,
                        divergent: vertex.divergent,
                        tail: vertex.tail,
                    }),
                });
                if vertex.initial.is_some() {
                    initial.push(offset + index);
                }
            }
            let mut at = vec![usize::MAX; product.edges.len()];
            for out in &product.out {
                for &index in out {
                    at[index] = edges.len();
                    edges.push(self.cert_edge(product, index, offset)?);
                }
            }
            placed.push(at);
            self.number(&proof.analyses, proof, offset, &mut components)?;
            offset += product.vertices.len();
        }
        let first = proofs
            .first()
            .ok_or(ZoneCheckRefusal::Fault("liveness-instance"))?;
        let admitted = self.live_admitted(&first.product, &placed[0])?;
        let integers = |bounds: &[Option<BigInt>]| -> Vec<Option<Integer>> {
            bounds
                .iter()
                .map(|bound| bound.clone().map(Integer::from_big))
                .collect()
        };
        Ok(Proof {
            scale: Integer::from_big(self.scale.clone()),
            lu_bounds: LuBounds {
                lower: integers(&first.bounds),
                upper: integers(&first.bounds),
            },
            nodes,
            edges,
            initial,
            components: Some(components),
            divergence: None,
            admitted,
        })
    }

    /// Product edge `index` as a certificate edge, its nodes shifted by
    /// `offset`.
    fn cert_edge(
        &mut self,
        product: &Product,
        index: usize,
        offset: usize,
    ) -> Result<CertEdge, Halt> {
        let edge = product.edges[index];
        let (transition, part, silent) = match edge.via {
            Via::Step { step, part } => {
                let steps = self.steps_for(&product.vertices[edge.from].state)?;
                let transition = steps
                    .get(step)
                    .ok_or(ZoneCheckRefusal::Fault("edge-step"))?
                    .transition()
                    .clone();
                (Some(transition), Some(part), None)
            }
            Via::Wait => (None, None, Some(SilentMove::Wait)),
            Via::Diverge => (None, None, Some(SilentMove::Diverge)),
            Via::Stutter => (None, None, None),
        };
        Ok(CertEdge {
            from: offset + edge.from,
            transition,
            part,
            horizon: None,
            to: offset + edge.to,
            silent,
        })
    }

    /// Number `analyses`, components of `proof`'s product, into `into`
    /// topologically, ties broken by least vertex, each with its reason.
    fn number(
        &mut self,
        analyses: &[Analysis],
        proof: &InstanceProof,
        offset: usize,
        into: &mut Components,
    ) -> Result<(), Halt> {
        let mut component_of: BTreeMap<usize, usize> = BTreeMap::new();
        for (index, analysis) in analyses.iter().enumerate() {
            for vertex in analysis.vertices() {
                component_of.insert(vertex.index(), index);
            }
        }
        let count = analyses.len();
        let mut later: Vec<BTreeSet<usize>> = vec![BTreeSet::new(); count];
        let mut before = vec![0_usize; count];
        for edge in &proof.product.edges {
            if let (Some(&from), Some(&to)) =
                (component_of.get(&edge.from), component_of.get(&edge.to))
            {
                if from != to && later[from].insert(to) {
                    before[to] += 1;
                }
            }
        }
        let least = |index: usize| {
            analyses[index]
                .vertices()
                .iter()
                .map(|vertex| vertex.index())
                .min()
                .unwrap_or(usize::MAX)
        };
        let mut ready: BTreeSet<(usize, usize)> = (0..count)
            .filter(|&index| before[index] == 0)
            .map(|index| (least(index), index))
            .collect();
        let mut order = Vec::with_capacity(count);
        while let Some(next) = ready.pop_first() {
            let (_, index) = next;
            order.push(index);
            for &to in &later[index] {
                before[to] -= 1;
                if before[to] == 0 {
                    ready.insert((least(to), to));
                }
            }
        }
        if order.len() != count {
            return Err(ZoneCheckRefusal::Fault("component-order").into());
        }
        for index in order {
            self.poll()?;
            let number = into.reasons.len();
            for vertex in analyses[index].vertices() {
                into.numbering[offset + vertex.index()] = Some(number);
            }
            let reason = self.reason(&analyses[index], proof, offset, into.numbering.len())?;
            into.reasons.push(reason);
        }
        Ok(())
    }

    /// Why `analysis`'s component holds no fair accepting cycle.
    fn reason(
        &mut self,
        analysis: &Analysis,
        proof: &InstanceProof,
        offset: usize,
        total: usize,
    ) -> Result<ComponentReason, Halt> {
        let member = |constraint: usize| {
            proof
                .members
                .get(constraint)
                .cloned()
                .ok_or(ZoneCheckRefusal::Fault("fairness-member"))
        };
        Ok(match analysis.outcome() {
            Outcome::Trivial => ComponentReason::Acyclic,
            Outcome::MissingAcceptance { set } => {
                let sets = proof.automaton_sets;
                ComponentReason::Lacks(match set {
                    set if *set < sets => AcceptanceSet::Automaton(*set),
                    set if *set == sets => AcceptanceSet::Divergence,
                    set if *set == sets + 1 => AcceptanceSet::Position,
                    _ => return Err(ZoneCheckRefusal::Fault("acceptance-set").into()),
                })
            }
            Outcome::UnfairWeak { constraint } => ComponentReason::Weak(member(*constraint)?),
            Outcome::UnfairStrong { constraints, sub } => {
                let mut nested = Components {
                    numbering: vec![None; total],
                    reasons: Vec::new(),
                };
                self.number(sub, proof, offset, &mut nested)?;
                ComponentReason::Strong {
                    constraints: constraints
                        .iter()
                        .map(|&constraint| member(constraint))
                        .collect::<Result<_, _>>()?,
                    nested: Box::new(nested),
                }
            }
            // A timed clause's fairness has no visibility constraint, and a
            // `Holds` no fair component.
            Outcome::UnfairVisibility { .. } | Outcome::Fair => {
                return Err(ZoneCheckRefusal::Fault("component-reason").into())
            }
        })
    }

    /// TD-6's witness in the first instance's product: a path from an
    /// initial vertex to a quiescent (dense) or terminal (tick) vertex, or
    /// else to a cycle through a divergence reset or a tick. `placed` maps
    /// the product's edges to the certificate's.
    fn live_admitted(
        &mut self,
        product: &Product,
        placed: &[usize],
    ) -> Result<DivergenceWitness, Halt> {
        let dense = self.timed.tick_period().is_none();
        // Breadth first from the initial vertices: each vertex's first edge
        // in and its initial vertex.
        let mut parent: Vec<Option<usize>> = vec![None; product.vertices.len()];
        let mut seen = vec![false; product.vertices.len()];
        let mut queue = VecDeque::new();
        for (index, vertex) in product.vertices.iter().enumerate() {
            if vertex.initial.is_some() {
                seen[index] = true;
                queue.push_back(index);
            }
        }
        let mut order = Vec::new();
        while let Some(at) = queue.pop_front() {
            order.push(at);
            for &index in &product.out[at] {
                let to = product.edges[index].to;
                if !seen[to] {
                    seen[to] = true;
                    parent[to] = Some(index);
                    queue.push_back(to);
                }
            }
        }
        let path_to = |mut at: usize| -> (usize, Vec<usize>) {
            let mut path = Vec::new();
            while let Some(index) = parent[at] {
                path.push(placed[index]);
                at = product.edges[index].from;
            }
            path.reverse();
            (at, path)
        };
        for &at in &order {
            self.poll()?;
            let vertex = &product.vertices[at];
            if vertex.tail {
                continue;
            }
            let idles = if dense {
                self.timed(self.timed.is_quiescent(&vertex.state))?
                    && self.elapses(&vertex.state)?
            } else {
                self.steps_for(&vertex.state)?.is_empty()
            };
            if idles {
                let (start, path) = path_to(at);
                // The first instance's nodes come first, unshifted.
                let node = at;
                return Ok(DivergenceWitness {
                    start,
                    path,
                    target: if dense {
                        DivergentTarget::Quiescent(node)
                    } else {
                        DivergentTarget::Terminal(node)
                    },
                });
            }
        }
        // A cycle through an edge that lets time advance.
        let adjacency: Vec<Vec<usize>> = product
            .out
            .iter()
            .map(|out| out.iter().map(|&index| product.edges[index].to).collect())
            .collect();
        let mut component_of = vec![usize::MAX; product.vertices.len()];
        for (index, component) in sccs(&adjacency).into_iter().enumerate() {
            for member in component {
                component_of[member] = index;
            }
        }
        for &at in &order {
            for &index in &product.out[at] {
                let edge = product.edges[index];
                if component_of[edge.from] != component_of[edge.to] {
                    continue;
                }
                let advances = match edge.via {
                    Via::Diverge => true,
                    Via::Step { step, .. } if !dense => {
                        let steps = self.steps_for(&product.vertices[edge.from].state)?;
                        let operation = steps
                            .get(step)
                            .ok_or(ZoneCheckRefusal::Fault("edge-step"))?
                            .operation()
                            .to_owned();
                        self.timed.tick_of(&operation) > Rational::from_integer(Integer::zero())
                    }
                    _ => false,
                };
                if !advances {
                    continue;
                }
                let back = back_within(product, &component_of, edge.to, edge.from)
                    .ok_or(ZoneCheckRefusal::Fault("admitted-cycle"))?;
                let (start, path) = path_to(edge.from);
                let mut cycle = vec![placed[index]];
                cycle.extend(back.into_iter().map(|index| placed[index]));
                return Ok(DivergenceWitness {
                    start,
                    path,
                    target: DivergentTarget::Cycle(cycle),
                });
            }
        }
        Err(ZoneCheckRefusal::Fault("admitted-witness").into())
    }
}

/// A shortest path of product edges from `start` to `end` inside their
/// strongly connected component.
fn back_within(
    product: &Product,
    component_of: &[usize],
    start: usize,
    end: usize,
) -> Option<Vec<usize>> {
    let component = component_of[start];
    let mut first: BTreeMap<usize, usize> = BTreeMap::new();
    let mut seen = BTreeSet::from([start]);
    let mut queue = VecDeque::from([start]);
    while let Some(at) = queue.pop_front() {
        if at == end {
            let mut path = Vec::new();
            let mut back = at;
            while back != start {
                let index = *first.get(&back)?;
                path.push(index);
                back = product.edges[index].from;
            }
            path.reverse();
            return Some(path);
        }
        for &index in &product.out[at] {
            let to = product.edges[index].to;
            if component_of[to] == component && seen.insert(to) {
                first.insert(to, index);
                queue.push_back(to);
            }
        }
    }
    None
}
