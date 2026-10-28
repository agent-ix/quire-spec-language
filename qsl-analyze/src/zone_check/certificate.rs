// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-244: the zone certificate every `Holds` outcome of the zone search
//! carries (ADR-026 CF-1, CF-5), built from the retained symbolic graph.
//!
//! A safety or deadlock-freedom proof carries the reachability part: each
//! stored node (its model-state key, its claim-automaton state, its
//! canonical zone and whether it is `locked`) and each retained edge, a
//! step's convex part to the node that holds or covers its successor. Every
//! certificate carries `admitted`, a path of its own edges from an initial
//! node to a divergent target (ADR-026 TD-6), taken from the evidence the
//! vacuity test accepted (FR-240's test). A time-lock-freedom proof adds
//! the divergence part: each node's own path to a divergent target, from
//! FR-240's test at that node. The liveness proof's product and component
//! parts are built in `live_certificate`.

use std::collections::{BTreeMap, BTreeSet};

use num_bigint::BigInt;
use qsl_replay::{
    CertEdge, CertNode, CertificateItem, Components, DivergenceWitness, DivergentTarget, LuBounds,
    Zone, ZoneCertificate,
};
use qsl_route::deadlock::{ItemObligation, ObligationKind};
use quire_exact::Integer;

use super::divergence::{EscapeTarget, EscapeWitness, Move};
use super::search::{Halt, Run, SymbolicEdge};
use super::{Bound, Dbm, ZoneCheckRefusal};

/// The item `obligation` names, as FR-245's checker compares it with the
/// recompiled item: its argument `item_of` for an EN-6 certificate.
pub fn certificate_item<S>(obligation: &ItemObligation<S>) -> CertificateItem<'_> {
    match &obligation.kind {
        ObligationKind::Authored(occurrence) => CertificateItem::Clause(occurrence),
        ObligationKind::DeadlockFreedom => CertificateItem::DeadlockFreedom,
        ObligationKind::TimeLockFreedom => CertificateItem::TimeLockFreedom,
        ObligationKind::RaceFreedom { .. } | ObligationKind::Refinement { .. } => {
            CertificateItem::Other
        }
    }
}

/// A certificate before it names the obligation it proves.
#[derive(Clone, Debug)]
pub(super) struct Proof {
    pub(super) scale: Integer,
    pub(super) lu_bounds: LuBounds,
    pub(super) nodes: Vec<CertNode>,
    pub(super) edges: Vec<CertEdge>,
    pub(super) initial: Vec<usize>,
    pub(super) components: Option<Components>,
    pub(super) divergence: Option<Vec<DivergenceWitness>>,
    pub(super) admitted: DivergenceWitness,
}

impl Proof {
    /// The certificate of `obligation`.
    pub(super) fn proving<S>(
        self,
        obligation: ItemObligation<S>,
    ) -> ZoneCertificate<ItemObligation<S>> {
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
    /// The certificate of a completed run with no admitted violation, its
    /// `admitted` part mapped from `witness`, the evidence FR-240's test
    /// accepted from the initial node `root`; with `divergence`, each
    /// node's own witness of that test too (the time-lock-freedom item).
    pub(super) fn certificate(
        &mut self,
        roots: &[usize],
        root: usize,
        witness: &EscapeWitness,
        divergence: bool,
    ) -> Result<Proof, Halt> {
        let mut nodes = Vec::with_capacity(self.nodes.len());
        for node in &self.nodes {
            let (_, state_key) = self
                .timed
                .system()
                .state_key(&node.state)
                .map_err(|_| Halt::Refusal(ZoneCheckRefusal::Fault("state-key")))?;
            nodes.push(CertNode {
                state_key,
                automaton_state: node.q,
                zone: core_zone(&node.zone)?,
                locked: node.locked,
                product: None,
            });
        }
        // Edges by source in discovery order; from one source the search
        // retains them in canonical step and part order.
        let mut order: Vec<usize> = (0..self.edges.len()).collect();
        order.sort_by_key(|&edge| self.edges[edge].from);
        let mut edges = Vec::with_capacity(order.len());
        let mut steps = Vec::with_capacity(order.len());
        for edge in order {
            let SymbolicEdge {
                from,
                to,
                step,
                piece,
                ..
            } = self.edges[edge].clone();
            let (transition, part) = match step {
                Some((step, part)) => {
                    let transition = self
                        .steps_of(from)?
                        .get(step)
                        .ok_or(Halt::Refusal(ZoneCheckRefusal::Fault("edge-step")))?
                        .transition()
                        .clone();
                    (Some(transition), Some(part))
                }
                None => (None, None),
            };
            let horizon = match piece {
                Some(piece) => Some(
                    self.pieces
                        .get(piece)
                        .map(|(constraints, _)| constraints.clone())
                        .ok_or(Halt::Refusal(ZoneCheckRefusal::Fault("edge-piece")))?,
                ),
                None => None,
            };
            steps.push(step);
            edges.push(CertEdge {
                from,
                transition,
                part,
                horizon,
                to,
                silent: None,
            });
        }
        let witnessed = admitted(&edges, &steps, root, witness)
            .ok_or(Halt::Refusal(ZoneCheckRefusal::Fault("divergence-witness")))?;
        let divergence = if divergence {
            let mut witnesses = Vec::with_capacity(self.nodes.len());
            for node in 0..self.nodes.len() {
                self.poll()?;
                // FR-232: the item holds, so every valuation of every node
                // has a time-divergent continuation.
                let escape = self.node_escape(node)?;
                let found = escape
                    .witness
                    .filter(|_| escape.diverges)
                    .and_then(|witness| admitted(&edges, &steps, node, &witness))
                    .ok_or(ZoneCheckRefusal::Fault("node-divergence-witness"))?;
                witnesses.push(found);
            }
            Some(witnesses)
        } else {
            None
        };
        let integers = |bounds: &[Option<BigInt>]| {
            bounds
                .iter()
                .map(|bound| bound.clone().map(Integer::from_big))
                .collect()
        };
        Ok(Proof {
            scale: Integer::from_big(self.scale.clone()),
            lu_bounds: LuBounds {
                lower: integers(&self.lower),
                upper: integers(&self.upper),
            },
            nodes,
            edges,
            initial: roots.to_vec(),
            components: None,
            divergence,
            admitted: witnessed,
        })
    }
}

/// `zone` as the core's zone, which the certificate carries: the same
/// matrix, read by the core's own code (ADR-026 CF-6).
pub(super) fn core_zone(zone: &Dbm) -> Result<Zone, Halt> {
    let dim = zone.dim();
    let mut bounds = Vec::with_capacity(dim * dim);
    for i in 0..dim {
        for j in 0..dim {
            bounds.push(match zone.bound(i, j)? {
                Bound::Finite { value, strict } => {
                    Some((Integer::from_big(value.clone()), *strict))
                }
                Bound::Infinite => None,
            });
        }
    }
    Zone::from_matrix(dim, bounds).map_err(|_| Halt::Refusal(ZoneCheckRefusal::Fault("core-zone")))
}

/// TD-6's witness over the certificate's edges: `witness`'s moves followed
/// from `root`, each model step taken by a certificate edge of the same
/// step and part. A node that covers a successor under the aLU simulation
/// enables every step and part the successor enables. Under a TT-2
/// clause's automaton one step has an edge per piece of the horizon
/// clock's axis, and the moves are followed through whichever pieces carry
/// them on. A cycle is repeated until a certificate node recurs at its
/// start, which closes it on the certificate's finitely many nodes; each
/// repetition lets time advance, so the closed cycle does too.
fn admitted(
    edges: &[CertEdge],
    steps: &[Option<(usize, usize)>],
    root: usize,
    witness: &EscapeWitness,
) -> Option<DivergenceWitness> {
    let follow = |at: usize, moves: &[Move]| follow(edges, steps, at, moves);
    let (mut at, mut path) = follow(root, &witness.path)?;
    let target = match &witness.target {
        EscapeTarget::Quiescent => DivergentTarget::Quiescent(at),
        EscapeTarget::Terminal => DivergentTarget::Terminal(at),
        EscapeTarget::Cycle(moves) => {
            let mut started: BTreeMap<usize, usize> = BTreeMap::new();
            let mut rounds: Vec<Vec<usize>> = Vec::new();
            loop {
                if let Some(&first) = started.get(&at) {
                    path.extend(rounds[..first].iter().flatten());
                    let cycle: Vec<usize> = rounds[first..].iter().flatten().copied().collect();
                    if cycle.is_empty() {
                        return None;
                    }
                    break DivergentTarget::Cycle(cycle);
                }
                started.insert(at, rounds.len());
                let (end, round) = follow(at, moves)?;
                at = end;
                rounds.push(round);
            }
        }
    };
    Some(DivergenceWitness {
        start: root,
        path,
        target,
    })
}

/// The first sequence of certificate edges from `at` that takes `moves`'
/// model steps in order, depth first over the edges each step has, with
/// the node it ends at. The search is iterative, and a `(node, move)` pair
/// from which the rest cannot be taken is tried once.
fn follow(
    edges: &[CertEdge],
    steps: &[Option<(usize, usize)>],
    at: usize,
    moves: &[Move],
) -> Option<(usize, Vec<usize>)> {
    // The model steps only: a move with none lets time pass.
    let moves: Vec<(usize, usize)> = moves.iter().filter_map(|step| *step).collect();
    let mut leaving: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for (index, edge) in edges.iter().enumerate() {
        leaving.entry(edge.from).or_default().push(index);
    }
    let mut dead: BTreeSet<(usize, usize)> = BTreeSet::new();
    // Each frame: the node, the move taken next, and the next candidate
    // edge to try.
    let mut taken: Vec<usize> = Vec::new();
    let mut frames: Vec<(usize, usize)> = vec![(at, 0)];
    loop {
        let depth = frames.len() - 1;
        let (node, next) = *frames.last()?;
        if depth == moves.len() {
            return Some((node, taken));
        }
        let candidates = leaving.get(&node).map_or(&[][..], Vec::as_slice);
        let found = candidates
            .iter()
            .enumerate()
            .skip(next)
            .find(|(_, &edge)| {
                steps[edge] == Some(moves[depth]) && !dead.contains(&(edges[edge].to, depth + 1))
            })
            .map(|(index, &edge)| (index, edge));
        match found {
            Some((index, edge)) => {
                if let Some(frame) = frames.last_mut() {
                    frame.1 = index + 1;
                }
                taken.push(edge);
                frames.push((edges[edge].to, 0));
            }
            None => {
                dead.insert((node, depth));
                frames.pop();
                taken.pop();
                if frames.is_empty() {
                    return None;
                }
            }
        }
    }
}
