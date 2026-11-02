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
use num_integer::Integer as NumInteger;
use qsl_replay::{
    CertEdge, CertNode, CertificateItem, CertificateTransition, Components, DivergenceWitness,
    DivergentTarget, LuBounds, TaskArrivalKind, TaskClosureMethod, TaskDiagonalConstraint,
    TaskDiagonalOperator, TaskProductAbstraction, TaskProductCertificate, TaskProductStateKey,
    TaskQueueBound, TaskTimeProgress, TaskTimeProgressMethod, Zone, ZoneCertificate,
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
        ObligationKind::TaskSchedulability(occurrence) => {
            CertificateItem::TaskSchedulability(occurrence)
        }
        ObligationKind::DeadlockFreedom => CertificateItem::DeadlockFreedom,
        ObligationKind::TimeLockFreedom => CertificateItem::TimeLockFreedom,
        ObligationKind::Refinement {
            occurrence,
            abstract_initial,
        } => CertificateItem::Refinement {
            occurrence,
            abstract_initial,
        },
        // A protocol subject is never timed, so EN-6 decides neither.
        ObligationKind::ProtocolAuthored { .. }
        | ObligationKind::ProtocolDeadlockFreedom { .. }
        | ObligationKind::RaceFreedom { .. } => CertificateItem::Other,
    }
}

/// A certificate before it names the obligation it proves.
#[derive(Clone, Debug)]
pub(super) struct Proof {
    pub(super) scale: Integer,
    pub(super) lu_bounds: LuBounds,
    pub(super) nodes: Vec<CertNode>,
    pub(super) edges: Vec<CertEdge>,
    pub(super) initials: Vec<usize>,
    pub(super) components: Option<Components>,
    pub(super) divergence: Option<Vec<DivergenceWitness>>,
    pub(super) admitted: DivergenceWitness,
    pub(super) task_product: Option<TaskProductCertificate>,
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
            initials: self.initials,
            components: self.components,
            divergence: self.divergence,
            admitted: self.admitted,
            refinement: None,
            task_product: self.task_product,
        }
    }
}

impl Run<'_, '_, '_> {
    /// The retained edges in certificate order, by index: by source in
    /// discovery order; from one source the search retains them in
    /// canonical step and part order.
    pub(super) fn edge_order(&self) -> Vec<usize> {
        let mut order: Vec<usize> = (0..self.edges.len()).collect();
        order.sort_by_key(|&edge| self.edges[edge].from);
        order
    }

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
            let (_, source_state_key) = self
                .timed
                .system()
                .state_key(&node.state)
                .map_err(|_| Halt::Refusal(ZoneCheckRefusal::Fault("state-key")))?;
            let state_key = match &node.task_state {
                Some(scheduler) => TaskProductStateKey::new(
                    source_state_key,
                    scheduler
                        .canonical_control_key()
                        .map_err(|_| Halt::Refusal(ZoneCheckRefusal::Fault("task-state-key")))?,
                )
                .into_state_key()
                .map_err(|_| Halt::Refusal(ZoneCheckRefusal::Fault("task-state-key")))?,
                None => source_state_key,
            };
            nodes.push(CertNode {
                state_key,
                automaton_state: node.q,
                zone: core_zone(&node.zone)?,
                locked: node.locked,
                product: None,
            });
        }
        let order = self.edge_order();
        let mut edges = Vec::with_capacity(order.len());
        let mut steps = Vec::with_capacity(order.len());
        for edge in order {
            let SymbolicEdge {
                from,
                to,
                step,
                piece,
                task_template,
                ..
            } = self.edges[edge].clone();
            let (transition, part) = match (step, task_template) {
                (Some((step, part)), _) => {
                    let transition = self
                        .steps_of(from)?
                        .get(step)
                        .ok_or(Halt::Refusal(ZoneCheckRefusal::Fault("edge-step")))?
                        .transition()
                        .clone();
                    (Some(CertificateTransition::Source(transition)), Some(part))
                }
                (None, Some(task)) => (Some(CertificateTransition::Task(Box::new(task))), None),
                (None, None) => (None, None),
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
        let task_product = self.task_product_certificate()?;
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
            initials: roots.to_vec(),
            components: None,
            divergence,
            admitted: witnessed,
            task_product,
        })
    }

    fn task_product_certificate(&self) -> Result<Option<TaskProductCertificate>, Halt> {
        let Some(subject) = &self.task_subject else {
            return Ok(None);
        };
        let product = match subject.route() {
            super::task_product::TaskProductRoute::Exact(product) => product,
            super::task_product::TaskProductRoute::Bounded(_)
            | super::task_product::TaskProductRoute::NoRelease => {
                let no_release = self.nodes.iter().all(|node| {
                    node.task_state.as_ref().is_some_and(|state| {
                        state.jobs().is_empty() && state.pending_releases().is_empty()
                    })
                }) && self.edges.iter().all(|edge| edge.task_transition.is_none());
                if !no_release {
                    return Err(Halt::Refusal(ZoneCheckRefusal::Fault(
                        "bounded-task-product-proof",
                    )));
                }
                let queue_bounds = subject
                    .task_set()
                    .tasks
                    .iter()
                    .map(|task| {
                        let (arrival, interarrival) = match &task.arrival {
                            qsl_semantics::check::Arrival::Periodic { period } => {
                                (TaskArrivalKind::Periodic, period.clone())
                            }
                            qsl_semantics::check::Arrival::Sporadic { min_interarrival } => {
                                (TaskArrivalKind::Sporadic, min_interarrival.clone())
                            }
                        };
                        let work_upper = match &task.wcet {
                            qsl_semantics::check::Wcet::Single(value) => value.clone(),
                            qsl_semantics::check::Wcet::Dual { hi, .. } => hi.clone(),
                        };
                        let ratio = task.deadline.div(&interarrival).ok_or(Halt::Refusal(
                            ZoneCheckRefusal::Fault("task-queue-bound-ratio"),
                        ))?;
                        let jobs = ratio
                            .numerator()
                            .as_big()
                            .div_ceil(ratio.denominator().as_big());
                        let max_jobs = Integer::from_big(jobs)
                            .add(&Integer::one())
                            .to_u64()
                            .filter(|value| *value > 0)
                            .ok_or(Halt::Refusal(ZoneCheckRefusal::Fault(
                                "task-queue-bound-cap",
                            )))?;
                        Ok(TaskQueueBound {
                            taskset: subject.task_set().name.clone(),
                            task: task.name.clone(),
                            arrival,
                            interarrival,
                            relative_deadline: task.deadline.clone(),
                            max_jitter: task.jitter.clone(),
                            work_lower: task.bcet.clone(),
                            work_upper,
                            max_jobs,
                        })
                    })
                    .collect::<Result<Vec<_>, Halt>>()?;
                return Ok(Some(TaskProductCertificate {
                    abstraction: TaskProductAbstraction::BoundedSubtractionDiagonalV1,
                    queue_bounds,
                    clock_ceilings: Vec::new(),
                    diagonal_constraints: Vec::new(),
                    subtractions: Vec::new(),
                    time_progress: TaskTimeProgress {
                        method: TaskTimeProgressMethod::ClosedNoReleaseProductV1,
                        live_job_cap: 0,
                        closure: TaskClosureMethod::RecomputeAllSourceSuccessorsNoTaskReleaseV1,
                    },
                }));
            }
        };
        let layout = self
            .task_clock_layout
            .as_ref()
            .ok_or(Halt::Refusal(ZoneCheckRefusal::Fault("task-clock-layout")))?;
        let queue_bounds = product
            .tasks
            .iter()
            .map(|task| TaskQueueBound {
                taskset: task.task_set.clone(),
                task: task.task.clone(),
                arrival: match task.arrival {
                    super::task_product::FixedWorkArrival::Periodic => TaskArrivalKind::Periodic,
                    super::task_product::FixedWorkArrival::Sporadic => TaskArrivalKind::Sporadic,
                },
                interarrival: task.interarrival.clone(),
                relative_deadline: task.relative_deadline.clone(),
                max_jitter: task.max_jitter.clone(),
                work_lower: task.work.clone(),
                work_upper: task.work.clone(),
                max_jobs: task.max_jobs,
            })
            .collect();
        let mut diagonal_constraints = Vec::new();
        for left_task in &product.tasks {
            for left_slot in 0..left_task.max_jobs {
                let left = format!(
                    "task:{}:{}:slot{}:service",
                    left_task.task_set, left_task.task, left_slot
                );
                for right_task in &product.tasks {
                    for right_slot in 0..right_task.max_jobs {
                        let right = format!(
                            "task:{}:{}:slot{}:service",
                            right_task.task_set, right_task.task, right_slot
                        );
                        if left != right {
                            diagonal_constraints.push(TaskDiagonalConstraint {
                                left: left.clone(),
                                operator: TaskDiagonalOperator::Less,
                                right,
                                constant: Integer::from_big(self.scaled(&left_task.work)),
                            });
                        }
                    }
                }
            }
        }
        let mut subtractions = Vec::new();
        for edge in &self.edges {
            for subtraction in &edge.task_subtractions {
                if !subtractions.contains(subtraction) {
                    subtractions.push(subtraction.clone());
                }
            }
        }
        Ok(Some(TaskProductCertificate {
            abstraction: TaskProductAbstraction::BoundedSubtractionDiagonalV1,
            queue_bounds,
            clock_ceilings: layout.ceilings().to_vec(),
            diagonal_constraints,
            subtractions,
            time_progress: TaskTimeProgress {
                method: TaskTimeProgressMethod::ClosedFiniteProductV1,
                live_job_cap: product.live_job_cap,
                closure: TaskClosureMethod::RecomputeAllReachableSuccessorsV1,
            },
        }))
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
