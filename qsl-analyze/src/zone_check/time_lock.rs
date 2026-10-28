// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-240 "Time-lock search" (ADR-026 EZ-7): the time-lock-freedom item
//! decided on the completed symbolic graph.
//!
//! Nodes are taken in breadth-first order, the search's canonical order.
//! At each, the local time-locks are the valuations of its zone at which no
//! positive delay is admissible and no step is enabled, computed by DBM
//! operations. Then the trapped valuations, those with no time-divergent
//! continuation: a timed state among them is concretised (FR-241) and its
//! forward closure explored afresh from its point zone, which confirms the
//! trap. The trapped valuations are exact, so FR-240's unconfirmed trap,
//! which it passes over, cannot arise here: a piece that does not
//! concretise or is not confirmed is a fault of the engine, never a proof.
//! At one node a local time-lock comes before a non-local one.
//!
//! EZ-7 marks whole symbolic states that reach no divergent target, which
//! misses a trap that holds only part of a zone: a state whose early
//! valuations step to a quiescent state and whose late ones only loop
//! Zeno-wise is marked by neither test (spec gap STD-493). Under a dense
//! source the trapped valuations are therefore computed exactly, as the
//! complement of the valuations with a time-divergent continuation:
//! `νY. μX. Q ∪ Pre(X) ∪ Pre_z(Y)` over the model states of the graph,
//! each a union of zones with the clock `z` of FR-240's test added, where
//! `Q` is a quiescent state's valuations, `Pre` the time and step
//! predecessors and `Pre_z` the silent `z >= 1` reset (Tripakis). Stored
//! zones are covered under the aM simulation (`L = U = M` per clock), a
//! bisimulation, so every reachable valuation has a bisimilar stored one
//! and a trap is never hidden behind a covering edge. Under a tick source
//! every zone is a point, so FR-240's test of a node decides its one
//! valuation.
//!
//! The counterexample is FR-236's `TimeLock`: the steps to the node, each
//! with its exact delay, and `final_delay`, the delay at the node that
//! reaches the time-locked valuation.

use std::collections::BTreeMap;

use qsl_eval::simulation::model::PlaceConstraint;
use qsl_replay::{CounterexampleKind, TemporalCounterexample};
use qsl_semantics::check::ClockOp;
use quire_exact::{Integer, Rational};

use super::concretise::{concretise, Concrete, Stage, Target};
use super::search::{constrain_at, uncovered, zone_bytes, Halt, Run};
use super::{Bound, Dbm, DbmError, ZoneCheckRefusal};

/// A union of zones.
#[derive(Clone, Debug, Default)]
struct Federation(Vec<Dbm>);

impl Federation {
    /// Add `zone`; whether the union grew.
    fn add(&mut self, zone: Dbm) -> Result<bool, DbmError> {
        if zone.is_empty() || uncovered(&zone, &self.0)?.is_empty() {
            return Ok(false);
        }
        let mut kept = Vec::with_capacity(self.0.len() + 1);
        for held in self.0.drain(..) {
            if !zone.includes(&held)? {
                kept.push(held);
            }
        }
        kept.push(zone);
        self.0 = kept;
        Ok(true)
    }

    /// Whether every valuation of `other` is in this union.
    fn covers(&self, other: &Self) -> Result<bool, DbmError> {
        for zone in &other.0 {
            if !uncovered(zone, &self.0)?.is_empty() {
                return Ok(false);
            }
        }
        Ok(true)
    }
}

/// The valuations of each model state of the graph with a time-divergent
/// continuation.
struct Divergence {
    /// Each model state's index, by its state key.
    keys: BTreeMap<Vec<u8>, usize>,
    /// Each model state's divergent valuations, as zones whose union they
    /// are.
    held: Vec<Vec<Dbm>>,
}

/// A path to a time-locked valuation: the initial state it starts from and
/// its concretisation.
struct LockPath {
    initial: usize,
    concrete: Concrete,
}

/// A model state of the graph, as the trap computation reads it.
struct Place {
    invariants: Vec<PlaceConstraint>,
    elapses: bool,
    quiescent: bool,
    moves: Vec<Move>,
}

/// One convex part of a step between two model states of the graph.
struct Move {
    to: usize,
    part: Vec<PlaceConstraint>,
    resets: Vec<(usize, Rational)>,
    carries: Vec<(usize, usize)>,
    target: Vec<PlaceConstraint>,
}

impl Run<'_, '_, '_> {
    /// FR-240: the first time-lock of the completed graph in canonical
    /// order, local before non-local at one node, as a counterexample.
    pub(super) fn first_time_lock(&mut self) -> Result<Option<TemporalCounterexample>, Halt> {
        let traps = self.traps()?;
        for (node, trapped) in traps.into_iter().enumerate() {
            self.poll()?;
            let locks = self.local_locks(node)?;
            if !locks.is_empty() {
                // Every valuation of a stored zone is reached along its real
                // edges, so some piece concretises.
                for lock in locks {
                    if let Some(concrete) = self.lock_path(node, &Target::Within(lock))? {
                        return Ok(Some(time_lock(concrete)?));
                    }
                }
                return Err(ZoneCheckRefusal::Fault("time-lock-concretisation").into());
            }
            let state = self.nodes[node].state.clone();
            // The trapped valuations are exact and reached along the node's
            // real edges: a piece that does not concretise, or whose point
            // the fresh exploration finds divergent, is the engine's own
            // disagreement, never a proof of time-lock freedom.
            if let Some(piece) = trapped.into_iter().next() {
                let concrete = self
                    .lock_path(node, &Target::Within(piece))?
                    .ok_or(ZoneCheckRefusal::Fault("time-lock-concretisation"))?;
                if self.point_diverges(&state, &concrete.concrete)? {
                    return Err(ZoneCheckRefusal::Fault("time-lock-confirmation").into());
                }
                return Ok(Some(time_lock(concrete)?));
            }
        }
        Ok(None)
    }

    /// Each node's trapped valuations, as zones whose union is that set.
    fn traps(&mut self) -> Result<Vec<Vec<Dbm>>, Halt> {
        if self.timed.tick_period().is_some() {
            let diverges = self.divergent_nodes()?;
            return Ok(diverges
                .into_iter()
                .zip(&self.nodes)
                .map(|(divergent, node)| {
                    if divergent {
                        Vec::new()
                    } else {
                        vec![node.zone.clone()]
                    }
                })
                .collect());
        }
        let Divergence {
            keys,
            held: divergent,
        } = self.divergent_valuations()?;
        let mut traps = Vec::with_capacity(self.nodes.len());
        for node in 0..self.nodes.len() {
            let held = keys
                .get(&self.key(&self.nodes[node].state)?)
                .and_then(|&place| divergent.get(place))
                .ok_or(ZoneCheckRefusal::Fault("trap-state"))?;
            traps.push(uncovered(&self.nodes[node].zone, held)?);
        }
        Ok(traps)
    }

    /// Under a dense source, the valuations of each model state of the
    /// graph with a time-divergent continuation.
    ///
    /// Every zone a set gains is charged to `max_zone_bytes`, which stops
    /// the run (V-7): the sets range over every valuation and the `z` clock,
    /// so they can outgrow the search's own zones. Each ν round computes
    /// its μ set afresh: the μ set of a smaller ν set is smaller, so the
    /// previous round's is no start from below.
    fn divergent_valuations(&mut self) -> Result<Divergence, Halt> {
        let mut keys = BTreeMap::new();
        let mut states = Vec::new();
        for node in 0..self.nodes.len() {
            let state = self.nodes[node].state.clone();
            let key = self.key(&state)?;
            if let std::collections::btree_map::Entry::Vacant(entry) = keys.entry(key) {
                entry.insert(states.len());
                states.push(state);
            }
        }
        let mut places = Vec::with_capacity(states.len());
        for state in &states {
            let mut moves = Vec::new();
            for step in self.steps_for(state)? {
                let Some(&to) = keys.get(&self.key(step.post())?) else {
                    // No reachable valuation takes the step.
                    continue;
                };
                let target = self.timed(self.timed.invariants(step.post()))?;
                for part in step.parts() {
                    moves.push(Move {
                        to,
                        part: part.clone(),
                        resets: step.resets().to_vec(),
                        carries: step.carries().to_vec(),
                        target: target.clone(),
                    });
                }
            }
            places.push(Place {
                invariants: self.timed(self.timed.invariants(state))?,
                elapses: self.elapses(state)?,
                quiescent: self.timed(self.timed.is_quiescent(state))?,
                moves,
            });
        }
        let dim = self.dim() + 1;
        let z = dim - 1;
        let scale = self.scale.clone();
        let within = |place: &Place| -> Result<Dbm, DbmError> {
            let mut zone = Dbm::universe(dim);
            constrain_at(&mut zone, &place.invariants, &scale)?;
            Ok(zone)
        };
        // The time predecessors of `zone` within `place`'s invariants.
        let before = |place: &Place, mut zone: Dbm| -> Result<Dbm, DbmError> {
            constrain_at(&mut zone, &place.invariants, &scale)?;
            if place.elapses && !zone.is_empty() {
                zone.down();
                constrain_at(&mut zone, &place.invariants, &scale)?;
            }
            Ok(zone)
        };
        let mut outer: Vec<Federation> = places
            .iter()
            .map(|place| within(place).map(|zone| Federation(vec![zone])))
            .collect::<Result<_, _>>()?;
        loop {
            self.poll()?;
            let mut inner = vec![Federation::default(); places.len()];
            for (at, place) in places.iter().enumerate() {
                if place.quiescent {
                    self.hold(&mut inner[at], within(place)?)?;
                }
                // The silent step that resets `z` once a unit has passed,
                // into the outer set.
                for zone in &outer[at].0 {
                    let mut reset = zone.clone();
                    reset.constrain(z, 0, Bound::le(0.into()))?;
                    reset.free(z)?;
                    reset.constrain(0, z, Bound::le(-scale.clone()))?;
                    self.hold(&mut inner[at], before(place, reset)?)?;
                }
            }
            loop {
                self.poll()?;
                let mut grew = false;
                for (at, place) in places.iter().enumerate() {
                    for step in &place.moves {
                        for zone in inner[step.to].0.clone() {
                            let mut pre = zone;
                            constrain_at(&mut pre, &step.target, &scale)?;
                            let carries: Vec<_> = step
                                .carries
                                .iter()
                                .map(|(target, source)| (target + 1, source + 1))
                                .collect();
                            let resets: Vec<_> = step
                                .resets
                                .iter()
                                .map(|(clock, value)| (clock + 1, self.scaled(value)))
                                .collect();
                            pre = pre.preimage_clocks(&carries, &resets)?;
                            constrain_at(&mut pre, &step.part, &scale)?;
                            grew |= self.hold(&mut inner[at], before(place, pre)?)?;
                        }
                    }
                }
                if !grew {
                    break;
                }
            }
            let stable = inner
                .iter()
                .zip(&outer)
                .try_fold(true, |stable, (inner, outer)| {
                    Ok::<_, DbmError>(stable && inner.covers(outer)?)
                })?;
            outer = inner;
            if stable {
                break;
            }
        }
        let divergent = outer
            .into_iter()
            .map(|held| {
                held.0
                    .into_iter()
                    .map(|mut zone| {
                        zone.constrain(z, 0, Bound::le(0.into()))?;
                        Ok(zone.without_last_clock())
                    })
                    .filter(|zone| zone.as_ref().map_or(true, |zone| !zone.is_empty()))
                    .collect::<Result<Vec<_>, DbmError>>()
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Divergence {
            keys,
            held: divergent,
        })
    }

    /// Add `zone` to `set`, charging it to `max_zone_bytes` when the set
    /// grows; whether it grew.
    fn hold(&mut self, set: &mut Federation, zone: Dbm) -> Result<bool, Halt> {
        let bytes = zone_bytes(&zone);
        if !set.add(zone)? {
            return Ok(false);
        }
        self.charge_bytes(bytes)?;
        Ok(true)
    }

    /// Whether each node has a valuation with a time-divergent continuation
    /// (FR-240's test). A node created by a real edge holds exactly the
    /// successors of its parent's valuations, so a parent with a divergent
    /// child diverges and needs no test of its own; children come after
    /// their parent, so the nodes are taken last to first.
    fn divergent_nodes(&mut self) -> Result<Vec<bool>, Halt> {
        let count = self.nodes.len();
        let mut diverges = vec![false; count];
        let mut real_child = vec![false; count];
        for node in (0..count).rev() {
            self.poll()?;
            diverges[node] = real_child[node] || self.node_escape(node)?.diverges;
            if diverges[node] {
                if let Some(parent) = self.parent_of(node) {
                    real_child[parent] = true;
                }
            }
        }
        Ok(diverges)
    }

    /// The valuations of `node`'s zone at which no positive delay is
    /// admissible and no step is enabled, as zones whose union is that set.
    /// Under a tick source a time-lock is never local: the tick is a step.
    fn local_locks(&mut self, node: usize) -> Result<Vec<Dbm>, Halt> {
        if self.timed.tick_period().is_some() {
            return Ok(Vec::new());
        }
        let state = self.nodes[node].state.clone();
        let zone = self.nodes[node].zone.clone();
        let dim = zone.dim();
        let mut covers = self.enabled(node, dim)?;
        if self.elapses(&state)? {
            // A positive delay is admissible exactly where every upper bound
            // of the time invariants holds strictly: a lower bound holds of
            // the zone and of every later instant.
            let mut delaying = Dbm::universe(dim);
            for invariant in self.timed(self.timed.invariants(&state))? {
                if matches!(
                    invariant.op(),
                    ClockOp::Less | ClockOp::LessOrEqual | ClockOp::Equal
                ) {
                    let bound = self.scaled(invariant.bound());
                    delaying.constrain(invariant.place() + 1, 0, Bound::lt(bound))?;
                }
            }
            covers.push(delaying);
        }
        Ok(uncovered(&zone, &covers)?)
    }

    /// FR-241 for a time-lock: the real edges to `node` and then a delay at
    /// it, the last valuation in `target`; the initial state's index and
    /// the path, or `None` when it has no concretisation.
    fn lock_path(&mut self, node: usize, target: &Target) -> Result<Option<LockPath>, Halt> {
        let (initial, mut stages, _) = self.stages_to(node)?;
        let state = self.nodes[node].state.clone();
        let invariants = self.timed(self.timed.invariants(&state))?;
        stages.push(Stage {
            invariants: invariants.clone(),
            elapses: self.elapses(&state)?,
            target: invariants,
            step: None,
            position: Vec::new(),
            position_resets: Vec::new(),
            before: None,
            piece: None,
            silent: false,
            fixed: None,
        });
        Ok(
            concretise(self.timed, self.dim() - 1, &stages, target, &self.scale)
                .ok()
                .map(|concrete| LockPath { initial, concrete }),
        )
    }
}

/// FR-236's `TimeLock` counterexample of the path `concrete` from the
/// initial state `initial`.
fn time_lock(LockPath { initial, concrete }: LockPath) -> Result<TemporalCounterexample, Halt> {
    let stem = concrete
        .prefix
        .iter()
        .try_fold(Rational::from_integer(Integer::zero()), |sum, step| {
            step.delay.as_ref().map(|delay| sum.add(delay))
        })
        .ok_or(ZoneCheckRefusal::Fault("time-lock-delay"))?;
    Ok(TemporalCounterexample {
        initial_state: initial,
        prefix: concrete.prefix,
        over_binding: None,
        fairness: Vec::new(),
        kind: CounterexampleKind::TimeLock,
        lasso: None,
        blocked: Vec::new(),
        refinement: None,
        final_delay: Some(concrete.last_stamp.sub(&stem)),
    })
}
