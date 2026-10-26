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
//! vacuity test accepted (FR-240's test). FR-240's component and
//! divergence parts belong to the liveness and time-lock-freedom proofs
//! (QSL-583) and are not built here.

use std::collections::BTreeMap;

use num_bigint::BigInt;
use qsl_foundation::digest::DigestRecord;
use qsl_route::deadlock::ItemObligation;
use quire_semantic_value::model_transition::ModelTransition;

use super::divergence::{EscapeTarget, EscapeWitness};
use super::search::{Halt, Run};
use super::{Dbm, ZoneCheckRefusal};

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
    /// canonical transition order: one per convex part of an enabled
    /// step's guard, since one target zone cannot cover the union of a
    /// non-convex guard's successors (spec gap STD-493: FR-244 names one
    /// edge per transition identity).
    pub edges: Vec<CertEdge>,
    /// The node that holds each initial symbolic state, one per initial
    /// state that admits a valuation (spec gap STD-493: FR-244 names one).
    pub initial: Vec<usize>,
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
    /// Under a dense source, a quiescent node, from which time diverges by
    /// the idle tail.
    Quiescent(usize),
    /// Under a tick source, a terminal node, whose stutter step counts as
    /// a tick (FR-231 "Digital time").
    Terminal(usize),
    /// A cycle of edges from and back to the path's last node, each
    /// repetition of which lets time advance: it repeats a cycle of
    /// FR-240's test that holds a `z >= 1` reset under a dense source, or a
    /// tick under a tick source.
    Cycle(Vec<usize>),
}

/// A certificate before it names the obligation it proves.
#[derive(Clone, Debug)]
pub(super) struct Proof {
    scale: BigInt,
    lu_bounds: Vec<LuBounds>,
    nodes: Vec<CertNode>,
    edges: Vec<CertEdge>,
    initial: Vec<usize>,
    admitted: DivergenceWitness,
}

impl Proof {
    /// The certificate of `obligation`.
    pub(super) fn proving<S>(self, obligation: ItemObligation<S>) -> ZoneCertificate<S> {
        ZoneCertificate {
            obligation,
            scale: self.scale,
            lu_bounds: self.lu_bounds,
            nodes: self.nodes,
            edges: self.edges,
            initial: self.initial,
            admitted: self.admitted,
        }
    }
}

impl Run<'_, '_, '_> {
    /// The certificate of a completed run with no admitted violation, its
    /// `admitted` part mapped from `witness`, the evidence FR-240's test
    /// accepted from the initial node `root`.
    pub(super) fn certificate(
        &mut self,
        roots: &[usize],
        root: usize,
        witness: &EscapeWitness,
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
                automaton_state: 0,
                zone: node.zone.clone(),
                locked: node.locked,
            });
        }
        // Edges by source in discovery order; from one source the search
        // retains them in canonical step and part order.
        let mut order: Vec<usize> = (0..self.edges.len()).collect();
        order.sort_by_key(|&edge| self.edges[edge].from);
        let mut edges = Vec::with_capacity(order.len());
        let mut steps = Vec::with_capacity(order.len());
        for edge in order {
            let edge = self.edges[edge].clone();
            let transition = self
                .steps_of(edge.from)?
                .get(edge.step)
                .ok_or(Halt::Refusal(ZoneCheckRefusal::Fault("edge-step")))?
                .transition()
                .clone();
            steps.push(edge.step);
            edges.push(CertEdge {
                from: edge.from,
                transition,
                part: edge.part,
                to: edge.to,
            });
        }
        let admitted = admitted(&edges, &steps, root, witness)
            .ok_or(Halt::Refusal(ZoneCheckRefusal::Fault("divergence-witness")))?;
        Ok(Proof {
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
            admitted,
        })
    }
}

/// TD-6's witness over the certificate's edges: `witness`'s moves followed
/// from `root`, each model step taken by the certificate edge of the same
/// step and part. A node that covers a successor under the aLU simulation
/// enables every step and part the successor enables, so each move has its
/// edge. A cycle is repeated until a certificate node recurs at its start,
/// which closes it on the certificate's finitely many nodes; each
/// repetition lets time advance, so the closed cycle does too.
fn admitted(
    edges: &[CertEdge],
    steps: &[usize],
    root: usize,
    witness: &EscapeWitness,
) -> Option<DivergenceWitness> {
    let follow = |at: &mut usize, step: Option<(usize, usize)>, taken: &mut Vec<usize>| {
        let Some((step, part)) = step else {
            // Only time passes.
            return Some(());
        };
        let edge = (0..edges.len()).find(|&edge| {
            edges[edge].from == *at && steps[edge] == step && edges[edge].part == part
        })?;
        taken.push(edge);
        *at = edges[edge].to;
        Some(())
    };
    let mut at = root;
    let mut path = Vec::new();
    for &step in &witness.path {
        follow(&mut at, step, &mut path)?;
    }
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
                let mut round = Vec::new();
                for &step in moves {
                    follow(&mut at, step, &mut round)?;
                }
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
