// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-237's fresh exploration of a replayed timed state's forward closure,
//! unreduced, looking for a time-divergent continuation: a quiescent state,
//! whose idle tail lets time diverge, or a cycle with positive total delay.
//!
//! Under a dense source it explores FR-238's zone successor relation from
//! the state's point zone with the core's own DBMs
//! ([`crate::certificate::zone`], the certificate checker's), never EN-6's. A fresh clock `z`, never compared by the model, is reset by a
//! silent step whenever `z >= 1`, and a cycle of the closure through such a
//! reset is a cycle along which time advances by at least 1 on every
//! repetition (Tripakis's non-Zeno construction, as FR-240 states it).
//! Zones are extrapolated by `Extra_M`, so the closure is finite.
//!
//! Under a tick source it explores the explicit timed states FR-231's
//! steps reach: a terminal state's stutter step is a tick, and a cycle
//! through a tick step advances time.

use std::collections::{BTreeMap, BTreeSet};

use num_bigint::BigInt;
use num_integer::Integer as _;
use qsl_eval::fairness::{components, EdgeLabel, Graph as Digraph, Identity};
use qsl_eval::simulation::model::{
    ModelSimState, PlaceConstraint, TimedState, TimedStop, TimedSystem,
};
use qsl_eval::simulation::Limits;
use qsl_semantics::check::ClockOp;
use quire_exact::{Cancel, Integer, Rational};

use crate::certificate::zone::{Zone, ZoneError};

/// What the exploration finds.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum Closure {
    /// A time-divergent continuation exists.
    Divergent,
    /// The closure completed with none: every continuation is time-locked.
    Locked,
    /// A limit stopped it: `max_states` (`true`) or `max_transitions`
    /// (`false`), with the count that passed it.
    Stopped {
        /// Whether `max_states` stopped it.
        states: bool,
        /// The count, one past the limit.
        count: u64,
    },
}

/// Why the exploration completed no value.
#[derive(Debug)]
pub(crate) enum ExploreStop {
    /// The timed subject could not decide a step, invariant or urgency.
    Timed,
    /// A state has no canonical key.
    StateKey,
    /// A zone operation named a clock the zone does not have.
    Zone,
    /// The explored graph named a vertex it does not hold.
    Graph,
}

impl From<ZoneError> for ExploreStop {
    fn from(_: ZoneError) -> Self {
        Self::Zone
    }
}

impl From<TimedStop> for ExploreStop {
    fn from(_: TimedStop) -> Self {
        Self::Timed
    }
}

/// FR-237: explore `start`'s forward closure under `limits`.
pub(crate) fn explore(
    timed: &TimedSystem<'_, '_>,
    start: &TimedState,
    limits: &Limits,
) -> Result<Closure, ExploreStop> {
    match timed.tick_period() {
        Some(_) => explore_ticks(timed, start, limits),
        None => explore_zones(timed, start, limits),
    }
}

/// A node of the explored graph: its model state and its out-edges, each
/// the target and whether time advances along it.
struct Graph<K> {
    keys: BTreeMap<K, usize>,
    out: Vec<Vec<(usize, bool)>>,
    transitions: usize,
}

impl<K: Ord> Graph<K> {
    fn new() -> Self {
        Self {
            keys: BTreeMap::new(),
            out: Vec::new(),
            transitions: 0,
        }
    }

    /// The node of `key`, and whether it is new; `Err` past `max_states`.
    fn node(&mut self, key: K, limits: &Limits) -> Result<(usize, bool), Closure> {
        if let Some(&node) = self.keys.get(&key) {
            return Ok((node, false));
        }
        let node = self.out.len();
        if node >= limits.max_states {
            return Err(Closure::Stopped {
                states: true,
                count: widen(node + 1),
            });
        }
        self.keys.insert(key, node);
        self.out.push(Vec::new());
        Ok((node, true))
    }

    /// Record an edge; `Err` past `max_transitions`.
    fn edge(
        &mut self,
        from: usize,
        to: usize,
        advances: bool,
        limits: &Limits,
    ) -> Result<(), Closure> {
        self.transitions += 1;
        if self.transitions > limits.max_transitions {
            return Err(Closure::Stopped {
                states: false,
                count: widen(self.transitions),
            });
        }
        self.out[from].push((to, advances));
        Ok(())
    }

    /// Whether some cycle passes through an edge along which time advances:
    /// one whose ends lie in one strongly connected component.
    fn advancing_cycle(&self) -> Result<bool, ExploreStop> {
        let mut graph = Digraph::new();
        let vertices: Vec<_> = self
            .out
            .iter()
            .map(|_| graph.add_vertex(std::iter::empty::<Identity>()))
            .collect();
        for (from, edges) in self.out.iter().enumerate() {
            for (to, _) in edges {
                graph
                    .add_edge(vertices[from], vertices[*to], EdgeLabel::Stutter)
                    .map_err(|_| ExploreStop::Graph)?;
            }
        }
        let within: BTreeSet<_> = vertices.iter().copied().collect();
        let found = components(&graph, &within, &Cancel::new()).map_err(|_| ExploreStop::Graph)?;
        let mut component = vec![usize::MAX; self.out.len()];
        for (index, members) in found.iter().enumerate() {
            for member in members.vertices() {
                component[member.index()] = index;
            }
        }
        Ok(self.out.iter().enumerate().any(|(from, edges)| {
            edges
                .iter()
                .any(|(to, advances)| *advances && component[from] == component[*to])
        }))
    }
}

pub(crate) fn widen(value: usize) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
}

fn key_of(timed: &TimedSystem<'_, '_>, state: &ModelSimState) -> Result<Vec<u8>, ExploreStop> {
    timed
        .system()
        .state_key(state)
        .map(|(bytes, _)| bytes)
        .map_err(|_| ExploreStop::StateKey)
}

/// Under a tick source: explicit timed states.
fn explore_ticks(
    timed: &TimedSystem<'_, '_>,
    start: &TimedState,
    limits: &Limits,
) -> Result<Closure, ExploreStop> {
    let identity = |state: &TimedState| {
        timed
            .state_key(state)
            .map(|key| key.as_bytes().to_vec())
            .map_err(|_| ExploreStop::StateKey)
    };
    let mut graph = Graph::new();
    let mut stack = vec![start.clone()];
    if let Err(stopped) = graph.node(identity(start)?, limits) {
        return Ok(stopped);
    }
    while let Some(state) = stack.pop() {
        let (from, _) = match graph.node(identity(&state)?, limits) {
            Ok(found) => found,
            Err(stopped) => return Ok(stopped),
        };
        let steps = timed.steps(&state)?;
        if steps.is_empty() {
            // A terminal state's stutter step is a tick (FR-231).
            return Ok(Closure::Divergent);
        }
        for (step, next) in steps {
            let ticks = timed.tick_of(step.operation()) > zero();
            let (to, new) = match graph.node(identity(&next)?, limits) {
                Ok(found) => found,
                Err(stopped) => return Ok(stopped),
            };
            if let Err(stopped) = graph.edge(from, to, ticks, limits) {
                return Ok(stopped);
            }
            if new {
                stack.push(next);
            }
        }
    }
    Ok(if graph.advancing_cycle()? {
        Closure::Divergent
    } else {
        Closure::Locked
    })
}

fn zero() -> Rational {
    Rational::from_integer(quire_exact::Integer::zero())
}

/// The scale of a dense exploration: one factor that makes every constant
/// of the model and every clock value of `start` an integer, and each
/// clock's largest scaled constant.
struct Scale {
    factor: BigInt,
    /// One per clock place, then `z`'s.
    max: Vec<Option<Integer>>,
}

impl Scale {
    fn of(timed: &TimedSystem<'_, '_>, start: &TimedState) -> Self {
        let model = timed.model();
        let constants: Vec<Rational> = model
            .guards()
            .iter()
            .flat_map(|guard| guard.parts())
            .flat_map(|part| part.clocks())
            .map(|constraint| constraint.bound().clone())
            .chain(
                model
                    .invariants()
                    .iter()
                    .flat_map(|invariant| invariant.bounds())
                    .map(|constraint| constraint.bound().clone()),
            )
            .chain(
                model
                    .resets()
                    .iter()
                    .flat_map(|resets| resets.resets())
                    .map(|reset| reset.value().clone()),
            )
            .collect();
        let factor = constants
            .iter()
            .chain(start.clocks().values())
            .fold(BigInt::from(1), |factor, value| {
                factor.lcm(value.denominator().as_big())
            });
        let scaled_max = constants
            .iter()
            .chain(start.clocks().values())
            .map(|value| scaled(value, &factor))
            .max()
            .unwrap_or_else(Integer::zero);
        let mut max = vec![Some(scaled_max); timed.places().len()];
        // `z` is compared with 1 only.
        max.push(Some(Integer::from_big(factor.clone())));
        Self { factor, max }
    }
}

/// `value` in units of `1 / factor`, which `factor` makes exact.
fn scaled(value: &Rational, factor: &BigInt) -> Integer {
    Integer::from_big(value.numerator().as_big() * factor / value.denominator().as_big())
}

/// Intersect `zone` with `constraints`, place `p` the zone's clock `p + 1`.
fn constrain(
    zone: &mut Zone,
    constraints: &[PlaceConstraint],
    factor: &BigInt,
) -> Result<(), ZoneError> {
    for constraint in constraints {
        let clock = constraint.place() + 1;
        let bound = scaled(constraint.bound(), factor);
        let below = bound.neg();
        match constraint.op() {
            ClockOp::Less => zone.constrain(clock, 0, &bound, true)?,
            ClockOp::LessOrEqual => zone.constrain(clock, 0, &bound, false)?,
            ClockOp::Equal => {
                zone.constrain(clock, 0, &bound, false)?;
                zone.constrain(0, clock, &below, false)?;
            }
            ClockOp::GreaterOrEqual => zone.constrain(0, clock, &below, false)?,
            ClockOp::Greater => zone.constrain(0, clock, &below, true)?,
        }
    }
    Ok(())
}

/// Under a dense source: zones with the `z` clock.
fn explore_zones(
    timed: &TimedSystem<'_, '_>,
    start: &TimedState,
    limits: &Limits,
) -> Result<Closure, ExploreStop> {
    let scale = Scale::of(timed, start);
    let factor = &scale.factor;
    let z = timed.places().len() + 1;
    // Let time pass at `state` within its invariants, then extrapolate.
    let settle = |state: &ModelSimState, zone: &mut Zone| -> Result<(), ExploreStop> {
        let invariants = timed.invariants(state)?;
        constrain(zone, &invariants, factor)?;
        if !timed.is_urgent(state)? {
            zone.up();
            constrain(zone, &invariants, factor)?;
        }
        zone.extrapolate(&scale.max)?;
        Ok(())
    };
    // The point zone of `start`'s clocks, `z` at 0.
    let mut zone = Zone::zero(z + 1);
    for (place, value) in start.clocks().values().iter().enumerate() {
        zone.reset(place + 1, &scaled(value, factor))?;
    }
    settle(start.discrete(), &mut zone)?;
    let mut graph: Graph<(Vec<u8>, Zone)> = Graph::new();
    let mut nodes: Vec<(ModelSimState, Zone)> = Vec::new();
    match graph.node((key_of(timed, start.discrete())?, zone.clone()), limits) {
        Ok(_) => nodes.push((start.discrete().clone(), zone)),
        Err(stopped) => return Ok(stopped),
    }
    let mut next = 0;
    while let Some((state, zone)) = nodes.get(next).cloned() {
        let from = next;
        next += 1;
        if timed.is_quiescent(&state)? {
            return Ok(Closure::Divergent);
        }
        let mut targets: Vec<(ModelSimState, Zone, bool)> = Vec::new();
        for step in timed.guarded_steps(&state)? {
            let target = timed.invariants(step.post())?;
            for part in step.parts() {
                let mut after = zone.clone();
                constrain(&mut after, part, factor)?;
                if after.is_empty() {
                    continue;
                }
                let carries: Vec<_> = step
                    .carries()
                    .iter()
                    .map(|(target, source)| (target + 1, source + 1))
                    .collect();
                let resets: Vec<_> = step
                    .resets()
                    .iter()
                    .map(|(place, value)| (place + 1, scaled(value, factor)))
                    .collect();
                after.assign_clocks(&carries, &resets)?;
                constrain(&mut after, &target, factor)?;
                if after.is_empty() {
                    continue;
                }
                settle(step.post(), &mut after)?;
                targets.push((step.post().clone(), after, false));
            }
        }
        // The silent step that resets `z` once a unit of time has passed.
        let mut reset = zone.clone();
        reset.constrain(0, z, &Integer::from_big(-factor.clone()), false)?;
        if !reset.is_empty() {
            reset.reset(z, &Integer::zero())?;
            settle(&state, &mut reset)?;
            targets.push((state.clone(), reset, true));
        }
        for (target, after, advances) in targets {
            if after.is_empty() {
                continue;
            }
            let (to, new) = match graph.node((key_of(timed, &target)?, after.clone()), limits) {
                Ok(found) => found,
                Err(stopped) => return Ok(stopped),
            };
            if let Err(stopped) = graph.edge(from, to, advances, limits) {
                return Ok(stopped);
            }
            if new {
                nodes.push((target, after));
            }
        }
    }
    Ok(if graph.advancing_cycle()? {
        Closure::Divergent
    } else {
        Closure::Locked
    })
}
