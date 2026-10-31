// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-245 over the time-lock-freedom item's divergence part (FR-244).
//!
//! Each node's witness is checked as `admitted` is, from the node's own
//! zone: its path follows valid edges and ends at a quiescent node, a
//! terminal one under a tick source, or a cycle that lets time advance and
//! repeats. A path shows that some valuation of a node diverges; the item
//! needs every valuation to. So the checker also computes, by its own
//! fixpoint over the model states the certificate reaches, the valuations
//! of each state with a time-divergent continuation, and requires every
//! node's zone to lie within them.
//!
//! Under a dense source the fixpoint reads a divergence clock `z`:
//! `νY. μX.` the valuations of a quiescent state, those from which a delay
//! makes `z` reach 1 and, with `z` reset, lie in `Y`, and those from which
//! a delay and a step reach `X`. Under a tick source, `X` holds a terminal
//! state's valuations, whose stutter ticks, and the predecessors by a tick
//! step of `Y` and by any other step of `X`. A move to a model state no
//! node reaches is left out, which can only shrink the sets. Each round is
//! charged to `max_certificate_edges`.

use std::collections::BTreeMap;

use qsl_eval::simulation::model::{GuardedStep, PlaceConstraint};
use quire_exact::{Integer, Rational};

use super::{
    at_node, malformed, reject, undecided, CertificateMember, CertificateRule, Checker,
    DivergenceWitness, Step,
};
use crate::certificate::zone::{Zone, ZoneError};

/// A union of zones.
#[derive(Clone, Default)]
struct Federation(Vec<Zone>);

impl Federation {
    /// Whether the union holds every valuation of `zone`.
    fn covers(&self, zone: &Zone) -> Result<bool, ZoneError> {
        let mut rest = vec![zone.clone()];
        for held in &self.0 {
            let mut next = Vec::new();
            for piece in &rest {
                next.extend(piece.minus(held)?);
            }
            rest = next;
            if rest.is_empty() {
                return Ok(true);
            }
        }
        Ok(rest.is_empty())
    }

    /// Add `zone`; whether the union grew.
    fn add(&mut self, zone: Zone) -> Result<bool, ZoneError> {
        if zone.is_empty() || self.covers(&zone)? {
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

    /// Whether the union covers every zone of `other`.
    fn covers_all(&self, other: &Self) -> Result<bool, ZoneError> {
        for zone in &other.0 {
            if !self.covers(zone)? {
                return Ok(false);
            }
        }
        Ok(true)
    }
}

/// One step of a model state, as the fixpoint reads it.
struct Move {
    to: usize,
    part: Vec<PlaceConstraint>,
    resets: Vec<(usize, Rational)>,
    carries: Vec<(usize, usize)>,
    target: Vec<PlaceConstraint>,
    tick: bool,
}

/// A model state the certificate reaches.
struct Place {
    invariants: Vec<PlaceConstraint>,
    elapses: bool,
    /// A behaviour may idle here forever.
    idles: bool,
    moves: Vec<Move>,
}

impl<O> Checker<'_, '_, O> {
    /// FR-244's divergence part, each node's zone within the exact
    /// divergent valuations of its state.
    pub(super) fn divergence_part(&self) -> Step<(), O> {
        let Some(witnesses) = &self.certificate.divergence else {
            return malformed(CertificateMember::Divergence);
        };
        let divergent = self.divergent_valuations()?;
        for node in 0..self.certificate.nodes.len() {
            self.poll()?;
            let Some(state) = self.states[node].clone() else {
                continue;
            };
            let fails = || reject(CertificateRule::WitnessFails, at_node(node));
            let Some(witness) = witnesses.get(node) else {
                return fails();
            };
            let mut zone = self.certificate.nodes[node].zone.clone();
            self.constrain(&mut zone, &self.invariants(&state)?)?;
            if witness.start != node || self.witness_ends(witness, zone.clone())?.is_err() {
                return fails();
            }
            let state_key = self.state_key(&state)?;
            let point = if self.timed.tick_period().is_none() {
                with_z(&zone)?
            } else {
                zone
            };
            let holds = match divergent.get(&state_key) {
                Some(held) => held.covers(&point)?,
                None => false,
            };
            if !holds {
                return fails();
            }
        }
        Ok(())
    }

    /// Whether `witness`, from the valuations `zone` of its start, follows
    /// valid edges to a divergent target: `Err` with the node it fails at.
    pub(super) fn witness_ends(
        &self,
        witness: &DivergenceWitness,
        mut zone: Zone,
    ) -> Step<Result<(), usize>, O> {
        let mut at = witness.start;
        for &index in &witness.path {
            let edge = &self.certificate.edges[index];
            if edge.from != at || self.covers[index].is_none() {
                return Ok(Err(at));
            }
            zone = self.post(index, &zone)?;
            at = edge.to;
            if zone.is_empty() {
                return Ok(Err(at));
            }
        }
        let holds = match &witness.target {
            super::DivergentTarget::Quiescent(node) => {
                *node == at && self.timed.tick_period().is_none() && self.idles(at)?
            }
            super::DivergentTarget::Terminal(node) => {
                *node == at && self.timed.tick_period().is_some() && self.idles(at)?
            }
            super::DivergentTarget::Cycle(cycle) => {
                self.advancing_cycle(at, cycle)? && self.repeats(cycle, &zone)?
            }
        };
        Ok(if holds { Ok(()) } else { Err(at) })
    }

    /// The valuations of each model state the certificate reaches with a
    /// time-divergent continuation, by the state's state key; under a dense
    /// source over the clock places and `z`.
    fn divergent_valuations(
        &self,
    ) -> Step<BTreeMap<qsl_eval::simulation::StateKey, Federation>, O> {
        let dense = self.timed.tick_period().is_none();
        let mut index: BTreeMap<qsl_eval::simulation::StateKey, usize> = BTreeMap::new();
        let mut states = Vec::new();
        for state in self.states.iter().flatten() {
            let state_key = self.state_key(state)?;
            if let std::collections::btree_map::Entry::Vacant(entry) =
                index.entry(state_key.clone())
            {
                entry.insert(states.len());
                states.push((state_key.clone(), state.clone()));
            }
        }
        let mut places = Vec::with_capacity(states.len());
        let mut moves_count = 0_u64;
        for (_, state) in &states {
            let mut moves = Vec::new();
            for step in self.timed.guarded_steps(state).or_else(undecided)? {
                let Some(&to) = index.get(&self.state_key(step.post())?) else {
                    continue;
                };
                let target = self.invariants(step.post())?;
                let tick = self.ticks_step(&step);
                for part in step.parts() {
                    moves.push(Move {
                        to,
                        part: part.clone(),
                        resets: step.resets().to_vec(),
                        carries: step.carries().to_vec(),
                        target: target.clone(),
                        tick,
                    });
                }
            }
            moves_count =
                moves_count.saturating_add(u64::try_from(moves.len()).unwrap_or(u64::MAX));
            let idles = match self.timed.tick_period() {
                Some(_) => self
                    .timed
                    .guarded_steps(state)
                    .or_else(undecided)?
                    .is_empty(),
                None => {
                    self.timed.is_quiescent(state).or_else(undecided)? && self.elapses(state)?
                }
            };
            places.push(Place {
                invariants: self.invariants(state)?,
                elapses: self.elapses(state)?,
                idles,
                moves,
            });
        }
        let dim = self.timed.places().len() + 1 + usize::from(dense);
        let z = dim - 1;
        let within = |place: &Place| -> Step<Zone, O> {
            let mut zone = Zone::universe(dim);
            self.constrain(&mut zone, &place.invariants)?;
            Ok(zone)
        };
        let before = |place: &Place, mut zone: Zone| -> Step<Zone, O> {
            self.constrain(&mut zone, &place.invariants)?;
            if place.elapses && !zone.is_empty() {
                zone.down();
                self.constrain(&mut zone, &place.invariants)?;
            }
            Ok(zone)
        };
        let pre = |step: &Move, zone: &Zone| -> Step<Zone, O> {
            let mut zone = zone.clone();
            self.constrain(&mut zone, &step.target)?;
            let carries: Vec<_> = step
                .carries
                .iter()
                .map(|(target, source)| (target + 1, source + 1))
                .collect();
            let resets: Vec<_> = step
                .resets
                .iter()
                .map(|(clock, value)| Ok((clock + 1, self.scaled(value)?)))
                .collect::<Step<_, O>>()?;
            zone = zone.preimage_clocks(&carries, &resets)?;
            self.constrain(&mut zone, &step.part)?;
            Ok(zone)
        };
        let mut outer: Vec<Federation> = places
            .iter()
            .map(|place| within(place).map(|zone| Federation(vec![zone])))
            .collect::<Result<_, _>>()?;
        loop {
            self.charge(moves_count.max(1))?;
            let mut inner = vec![Federation::default(); places.len()];
            for (at, place) in places.iter().enumerate() {
                if place.idles {
                    inner[at].add(within(place)?)?;
                }
                if dense {
                    for zone in &outer[at].0 {
                        let mut reset = zone.clone();
                        reset.constrain(z, 0, &Integer::zero(), false)?;
                        reset.free(z)?;
                        reset.constrain(0, z, &self.certificate.scale.neg(), false)?;
                        inner[at].add(before(place, reset)?)?;
                    }
                }
            }
            loop {
                self.charge(moves_count.max(1))?;
                let mut grew = false;
                for (at, place) in places.iter().enumerate() {
                    for step in &place.moves {
                        let source = if !dense && step.tick {
                            outer[step.to].0.clone()
                        } else {
                            inner[step.to].0.clone()
                        };
                        for zone in source {
                            let back = before(place, pre(step, &zone)?)?;
                            grew |= inner[at].add(back)?;
                        }
                    }
                }
                if !grew {
                    break;
                }
            }
            let mut stable = true;
            for (held, earlier) in inner.iter().zip(&outer) {
                stable &= held.covers_all(earlier)?;
            }
            outer = inner;
            if stable {
                break;
            }
        }
        Ok(states
            .into_iter()
            .map(|(state_key, _)| state_key)
            .zip(outer)
            .collect())
    }

    /// Whether `step` is a tick source's tick operation.
    fn ticks_step(&self, step: &GuardedStep) -> bool {
        self.timed.tick_of(step.operation()) > Rational::from_integer(Integer::zero())
    }
}

/// `zone` with one more clock, `z`, at 0.
fn with_z(zone: &Zone) -> Result<Zone, ZoneError> {
    let dim = zone.dim();
    let wide = dim + 1;
    let mut bounds = Vec::with_capacity(wide * wide);
    let zero = Some((Integer::zero(), false));
    for i in 0..wide {
        for j in 0..wide {
            // `z` reads as the reference clock.
            let (ri, rj) = (if i == dim { 0 } else { i }, if j == dim { 0 } else { j });
            bounds.push(if ri == rj {
                zero.clone()
            } else {
                zone.bound(ri, rj)?
            });
        }
    }
    Zone::from_matrix(wide, bounds)
}
