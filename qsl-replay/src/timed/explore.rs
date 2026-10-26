// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-237's fresh exploration of a replayed timed state's forward closure,
//! unreduced, looking for a time-divergent continuation: a quiescent state,
//! whose idle tail lets time diverge, or a cycle with positive total delay.
//!
//! Under a dense source it explores FR-238's zone successor relation from
//! the state's point zone with the core's own DBMs ([`super::dbm`]), never
//! EN-6's. A fresh clock `z`, never compared by the model, is reset by a
//! silent step whenever `z >= 1`, and a cycle of the closure through such a
//! reset is a cycle along which time advances by at least 1 on every
//! repetition (Tripakis's non-Zeno construction, as FR-240 states it).
//! Zones are extrapolated by `Extra_M`, so the closure is finite.
//!
//! Under a tick source it explores the explicit timed states FR-231's
//! steps reach: a terminal state's stutter step is a tick, and a cycle
//! through a tick step advances time.

use std::collections::BTreeMap;

use num_bigint::BigInt;
use num_integer::Integer as _;
use qsl_eval::simulation::model::{
    ModelSimState, PlaceConstraint, TimedState, TimedStop, TimedSystem,
};
use qsl_eval::simulation::Limits;
use qsl_semantics::check::ClockOp;
use quire_exact::Rational;

use super::dbm::{Bound, Dbm};

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

    /// Whether some cycle passes through an edge along which time advances.
    fn advancing_cycle(&self) -> bool {
        let adjacency: Vec<Vec<usize>> = self
            .out
            .iter()
            .map(|edges| edges.iter().map(|(to, _)| *to).collect())
            .collect();
        let mut component = vec![usize::MAX; self.out.len()];
        for (index, members) in sccs(&adjacency).into_iter().enumerate() {
            for member in members {
                component[member] = index;
            }
        }
        self.out.iter().enumerate().any(|(from, edges)| {
            edges
                .iter()
                .any(|(to, advances)| *advances && component[from] == component[*to])
        })
    }
}

fn widen(value: usize) -> u64 {
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
            .state_digest(state)
            .map(|digest| *digest.as_bytes())
            .map_err(|_| ExploreStop::StateKey)
    };
    let mut graph = Graph::new();
    let mut queue = vec![start.clone()];
    if let Err(stopped) = graph.node(identity(start)?, limits) {
        return Ok(stopped);
    }
    while let Some(state) = queue.pop() {
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
                queue.push(next);
            }
        }
    }
    Ok(if graph.advancing_cycle() {
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
    max: Vec<Option<BigInt>>,
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
            .unwrap_or_default();
        let mut max = vec![Some(scaled_max); timed.places().len()];
        // `z` is compared with 1 only.
        max.push(Some(factor.clone()));
        Self { factor, max }
    }
}

/// `value` in units of `1 / factor`, which `factor` makes exact.
fn scaled(value: &Rational, factor: &BigInt) -> BigInt {
    value.numerator().as_big() * factor / value.denominator().as_big()
}

/// Intersect `zone` with `constraints`, place `p` the DBM's clock `p + 1`.
fn constrain(zone: &mut Dbm, constraints: &[PlaceConstraint], factor: &BigInt) {
    for constraint in constraints {
        let clock = constraint.place() + 1;
        let bound = scaled(constraint.bound(), factor);
        match constraint.op() {
            ClockOp::Less => zone.constrain(clock, 0, Bound::lt(bound)),
            ClockOp::LessOrEqual => zone.constrain(clock, 0, Bound::le(bound)),
            ClockOp::Equal => {
                zone.constrain(clock, 0, Bound::le(bound.clone()));
                zone.constrain(0, clock, Bound::le(-bound));
            }
            ClockOp::GreaterOrEqual => zone.constrain(0, clock, Bound::le(-bound)),
            ClockOp::Greater => zone.constrain(0, clock, Bound::lt(-bound)),
        }
    }
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
    let settle = |state: &ModelSimState, zone: &mut Dbm| -> Result<(), ExploreStop> {
        let invariants = timed.invariants(state)?;
        constrain(zone, &invariants, factor);
        if !timed.is_urgent(state)? {
            zone.up();
            constrain(zone, &invariants, factor);
        }
        zone.extrapolate(&scale.max);
        Ok(())
    };
    let mut values: Vec<BigInt> = start
        .clocks()
        .values()
        .iter()
        .map(|value| scaled(value, factor))
        .collect();
    values.push(BigInt::from(0));
    let mut zone = Dbm::point(&values);
    settle(start.discrete(), &mut zone)?;
    let mut graph: Graph<(Vec<u8>, Dbm)> = Graph::new();
    let mut nodes: Vec<(ModelSimState, Dbm)> = Vec::new();
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
        let mut targets: Vec<(ModelSimState, Dbm, bool)> = Vec::new();
        for step in timed.guarded_steps(&state)? {
            let target = timed.invariants(step.post())?;
            for part in step.parts() {
                let mut after = zone.clone();
                constrain(&mut after, part, factor);
                if after.is_empty() {
                    continue;
                }
                for (place, value) in step.resets() {
                    after.reset(place + 1, &scaled(value, factor));
                }
                constrain(&mut after, &target, factor);
                if after.is_empty() {
                    continue;
                }
                settle(step.post(), &mut after)?;
                targets.push((step.post().clone(), after, false));
            }
        }
        // The silent step that resets `z` once a unit of time has passed.
        let mut reset = zone.clone();
        reset.constrain(0, z, Bound::le(-factor.clone()));
        if !reset.is_empty() {
            reset.reset(z, &BigInt::from(0));
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
    Ok(if graph.advancing_cycle() {
        Closure::Divergent
    } else {
        Closure::Locked
    })
}

/// The strongly connected components of `out` (node `i`'s successors are
/// `out[i]`), by Tarjan's algorithm without recursion.
fn sccs(out: &[Vec<usize>]) -> Vec<Vec<usize>> {
    let size = out.len();
    let mut index = vec![usize::MAX; size];
    let mut low = vec![0; size];
    let mut on_stack = vec![false; size];
    let mut stack = Vec::new();
    let mut found = Vec::new();
    let mut counter = 0;
    for root in 0..size {
        if index[root] != usize::MAX {
            continue;
        }
        // Each frame: the node and the next successor to visit.
        let mut frames = vec![(root, 0)];
        index[root] = counter;
        low[root] = counter;
        counter += 1;
        stack.push(root);
        on_stack[root] = true;
        while let Some(&mut (node, ref mut next)) = frames.last_mut() {
            if let Some(&successor) = out[node].get(*next) {
                *next += 1;
                if index[successor] == usize::MAX {
                    index[successor] = counter;
                    low[successor] = counter;
                    counter += 1;
                    stack.push(successor);
                    on_stack[successor] = true;
                    frames.push((successor, 0));
                } else if on_stack[successor] {
                    low[node] = low[node].min(index[successor]);
                }
                continue;
            }
            frames.pop();
            if let Some(&(parent, _)) = frames.last() {
                low[parent] = low[parent].min(low[node]);
            }
            if low[node] == index[node] {
                let mut component = Vec::new();
                while let Some(member) = stack.pop() {
                    on_stack[member] = false;
                    component.push(member);
                    if member == node {
                        break;
                    }
                }
                found.push(component);
            }
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::sccs;

    /// A cycle is one component and a tail node its own.
    #[test]
    fn a_cycle_is_one_component() {
        let mut found = sccs(&[vec![1], vec![2], vec![1, 3], vec![]]);
        for component in &mut found {
            component.sort_unstable();
        }
        found.sort();
        assert_eq!(found, [vec![0], vec![1, 2], vec![3]]);
    }
}
