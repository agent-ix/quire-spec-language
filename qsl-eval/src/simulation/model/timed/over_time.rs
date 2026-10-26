// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-232 (ADR-026 TD-3 to TD-5): enabledness, terminal states, time-locks
//! and weak fairness read over time, at a timed state and across the delays
//! that leave it.
//!
//! Every clock advances at rate 1 during a delay, so an atomic constraint
//! over a clock that is not reset holds on a half-line of delays; a convex
//! part or an applicable time invariant holds on the intersection of its
//! half-lines, an interval; a step is enabled on the union of its parts'
//! intervals. The reading is exact: each end point is a rational and keeps
//! whether it is included.

use quire_exact::Rational;

use qsl_semantics::check::ClockOp;

use super::{zero, GuardedStep, PlaceConstraint, TimedState, TimedStop, TimedSystem};
use crate::simulation::model::TerminalClass;

/// An interval of delays, `lower` to `upper`, each end with whether it is
/// included; no `upper` is unbounded above.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DelayInterval {
    lower: (Rational, bool),
    upper: Option<(Rational, bool)>,
}

impl DelayInterval {
    /// Every delay `>= 0`.
    fn all() -> Self {
        Self {
            lower: (zero(), true),
            upper: None,
        }
    }

    /// The least delay, and whether it is included.
    pub fn lower(&self) -> (&Rational, bool) {
        (&self.lower.0, self.lower.1)
    }

    /// The greatest delay and whether it is included; `None` when the
    /// interval is unbounded above.
    pub fn upper(&self) -> Option<(&Rational, bool)> {
        self.upper.as_ref().map(|(value, closed)| (value, *closed))
    }

    /// Whether the interval holds no delay.
    pub fn is_empty(&self) -> bool {
        match &self.upper {
            None => false,
            Some((upper, closed)) => {
                upper < &self.lower.0 || (upper == &self.lower.0 && !(*closed && self.lower.1))
            }
        }
    }

    /// Whether `delay` is in the interval.
    pub fn contains(&self, delay: &Rational) -> bool {
        let above = delay > &self.lower.0 || (delay == &self.lower.0 && self.lower.1);
        let below = self
            .upper
            .as_ref()
            .is_none_or(|(upper, closed)| delay < upper || (delay == upper && *closed));
        above && below
    }

    /// The intersection with `other`.
    #[must_use]
    fn meet(&self, other: &Self) -> Self {
        let lower = match self.lower.0.cmp(&other.lower.0) {
            std::cmp::Ordering::Greater => self.lower.clone(),
            std::cmp::Ordering::Less => other.lower.clone(),
            std::cmp::Ordering::Equal => (self.lower.0.clone(), self.lower.1 && other.lower.1),
        };
        let upper = match (&self.upper, &other.upper) {
            (None, upper) | (upper, None) => upper.clone(),
            (Some(left), Some(right)) => Some(match left.0.cmp(&right.0) {
                std::cmp::Ordering::Less => left.clone(),
                std::cmp::Ordering::Greater => right.clone(),
                std::cmp::Ordering::Equal => (left.0.clone(), left.1 && right.1),
            }),
        };
        Self { lower, upper }
    }

    /// The delays `t` at which `value + t op bound` holds.
    fn of(value: &Rational, op: ClockOp, bound: &Rational) -> Self {
        let edge = bound.sub(value);
        match op {
            ClockOp::Less => Self::below(edge, false),
            ClockOp::LessOrEqual => Self::below(edge, true),
            ClockOp::Equal => Self {
                lower: (edge.clone(), true),
                upper: Some((edge, true)),
            },
            ClockOp::GreaterOrEqual => Self::above(edge, true),
            ClockOp::Greater => Self::above(edge, false),
        }
    }

    fn below(edge: Rational, closed: bool) -> Self {
        Self {
            lower: (zero(), true),
            upper: Some((edge, closed)),
        }
    }

    fn above(edge: Rational, closed: bool) -> Self {
        Self {
            lower: (edge, closed),
            upper: None,
        }
    }

    /// The empty interval.
    fn none() -> Self {
        Self {
            lower: (zero(), false),
            upper: Some((zero(), false)),
        }
    }
}

/// Whether the union of `intervals` holds every delay in `[0, delay]`.
fn covers(intervals: &[DelayInterval], delay: &Rational) -> bool {
    // `[0, reach]` is covered when `included`, `[0, reach)` otherwise.
    let mut reach = zero();
    let mut included = false;
    loop {
        let mut grew = false;
        for interval in intervals.iter().filter(|interval| !interval.is_empty()) {
            let (lower, lower_closed) = &interval.lower;
            let joins = lower < &reach || (lower == &reach && (*lower_closed || included));
            if !joins {
                continue;
            }
            match &interval.upper {
                None => return true,
                Some((upper, closed)) => {
                    if upper > &reach || (upper == &reach && *closed && !included) {
                        reach.clone_from(upper);
                        included = *closed;
                        grew = true;
                    }
                }
            }
        }
        if !grew {
            return &reach > delay || (&reach == delay && included);
        }
    }
}

/// What [`TimedSystem::time_lock`] finds at a timed state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TimeLock {
    /// No positive delay is admissible and no discrete step is enabled.
    Local,
    /// A discrete step is enabled, but no timed state that discrete steps
    /// alone reach from it admits a positive delay: every continuation is
    /// Zeno.
    NonLocal,
}

/// The class of a timed state over time (FR-232 "Deadlocks over time").
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TimedTerminalClass {
    /// A discrete step is enabled at the state or after an admissible
    /// delay.
    NotTerminal,
    /// The state is terminal and its delay is bounded: a local time-lock
    /// is reached at the bound, which the time-lock-freedom item reports,
    /// not the deadlock-freedom item.
    TimeLock,
    /// The state is terminal and quiescent: FR-124's class of its discrete
    /// state.
    Quiescent(TerminalClass),
}

impl TimedSystem<'_, '_> {
    /// The delays admissible at `state` (FR-231 "Delay"): `[0, 0]` under a
    /// tick source or at an urgent state, and otherwise every delay at
    /// which each applicable time invariant still holds.
    ///
    /// # Errors
    ///
    /// As [`Self::invariants`] and [`Self::is_urgent`].
    pub fn admissible_delays(&self, state: &TimedState) -> Result<DelayInterval, TimedStop> {
        if self.tick.is_some() || self.is_urgent(&state.discrete)? {
            return Ok(DelayInterval {
                lower: (zero(), true),
                upper: Some((zero(), true)),
            });
        }
        Ok(self.within(&state.clocks, &self.invariants(&state.discrete)?, &[]))
    }

    /// The delays at which every constraint of `constraints` holds of
    /// `clocks + t`, a place of `reset` keeping its reset value.
    fn within(
        &self,
        clocks: &super::ClockValuation,
        constraints: &[PlaceConstraint],
        reset: &[(usize, Rational)],
    ) -> DelayInterval {
        let mut interval = DelayInterval::all();
        for constraint in constraints {
            let fixed = reset
                .iter()
                .rev()
                .find(|(place, _)| *place == constraint.place)
                .map(|(_, value)| value);
            let Some(value) = clocks.get(constraint.place) else {
                return DelayInterval::none();
            };
            let holding = match fixed {
                Some(value) if constraint.op.holds(value, &constraint.bound) => {
                    DelayInterval::all()
                }
                Some(_) => DelayInterval::none(),
                None => DelayInterval::of(value, constraint.op, &constraint.bound),
            };
            interval = interval.meet(&holding);
        }
        interval
    }

    /// The delays after which `step` is enabled from `state` (FR-232
    /// "Fairness over time"): a convex part of its guard holds, and the
    /// timed state it reaches satisfies the time invariants applying there.
    ///
    /// # Errors
    ///
    /// As [`Self::invariants`].
    pub fn enabled_delays(
        &self,
        state: &TimedState,
        step: &GuardedStep,
    ) -> Result<Vec<DelayInterval>, TimedStop> {
        let target = self.within(&state.clocks, &self.invariants(&step.post)?, &step.resets);
        Ok(step
            .parts
            .iter()
            .map(|part| self.within(&state.clocks, part, &[]).meet(&target))
            .filter(|interval| !interval.is_empty())
            .collect())
    }

    /// The class of `state` over time (FR-232 "Deadlocks over time").
    ///
    /// # Errors
    ///
    /// As [`Self::guarded_steps`], [`Self::admissible_delays`] and
    /// [`ModelSystem::terminal_class`].
    pub fn terminal_class(&self, state: &TimedState) -> Result<TimedTerminalClass, TimedStop> {
        let admissible = self.admissible_delays(state)?;
        for step in self.guarded_steps(&state.discrete)? {
            if self
                .enabled_delays(state, &step)?
                .iter()
                .any(|interval| !interval.meet(&admissible).is_empty())
            {
                return Ok(TimedTerminalClass::NotTerminal);
            }
        }
        if admissible.upper.is_some() && self.tick.is_none() {
            return Ok(TimedTerminalClass::TimeLock);
        }
        Ok(TimedTerminalClass::Quiescent(
            self.system.terminal_class(&state.discrete, false)?,
        ))
    }

    /// FR-124's `deadlocked` over time: `state` is terminal after every
    /// admissible delay, is quiescent, and no `terminal` declaration
    /// intends it.
    ///
    /// # Errors
    ///
    /// As [`Self::terminal_class`].
    pub fn deadlocked(&self, state: &TimedState) -> Result<bool, TimedStop> {
        Ok(matches!(
            self.terminal_class(state)?,
            TimedTerminalClass::Quiescent(TerminalClass::Deadlocked)
        ))
    }

    /// Whether `state` admits a positive delay.
    fn delays(&self, state: &TimedState) -> Result<bool, TimedStop> {
        let admissible = self.admissible_delays(state)?;
        Ok(admissible.upper.is_none_or(|(upper, _)| upper > zero()))
    }

    /// Whether `state` is a time-lock that time cannot leave: `Local` when no
    /// positive delay is admissible and no step is enabled, `NonLocal` when
    /// a step is enabled but no timed state that discrete steps alone reach
    /// admits a positive delay, at most `budget` such states being visited.
    /// `None` when such a state admits a positive delay, or when `budget`
    /// is spent: a non-local time-lock whose continuations are Zeno through
    /// ever smaller positive delays is the zone engine's to find (FR-239).
    ///
    /// # Errors
    ///
    /// As [`Self::steps`] and [`Self::admissible_delays`].
    pub fn time_lock(
        &self,
        state: &TimedState,
        budget: usize,
    ) -> Result<Option<TimeLock>, TimedStop> {
        if self.delays(state)? {
            return Ok(None);
        }
        let first = self.steps(state)?;
        if first.is_empty() {
            return Ok(Some(TimeLock::Local));
        }
        // `Rational` caches, so the visited timed states are a list, which
        // `budget` keeps short.
        let mut seen = vec![self.key(state)?];
        let mut frontier: Vec<TimedState> = first.into_iter().map(|(_, next)| next).collect();
        while let Some(next) = frontier.pop() {
            let key = self.key(&next)?;
            if seen.contains(&key) {
                continue;
            }
            seen.push(key);
            if seen.len() > budget || self.delays(&next)? {
                return Ok(None);
            }
            frontier.extend(self.steps(&next)?.into_iter().map(|(_, after)| after));
        }
        Ok(Some(TimeLock::NonLocal))
    }

    /// A timed state's identity: its state key and clock valuation.
    fn key(&self, state: &TimedState) -> Result<(Vec<u8>, super::ClockValuation), TimedStop> {
        let (bytes, _) = self
            .system
            .state_key(&state.discrete)
            .map_err(|_| crate::simulation::model::walk::stop(crate::simulation::model::BROKEN))?;
        Ok((bytes, state.clocks.clone()))
    }

    /// Whether some step that `constrained` names is enabled at every
    /// instant from `state` through a delay of `delay`.
    fn enabled_throughout(
        &self,
        state: &TimedState,
        delay: &Rational,
        constrained: &impl Fn(&GuardedStep) -> bool,
    ) -> Result<bool, TimedStop> {
        let mut intervals = Vec::new();
        for step in self.guarded_steps(&state.discrete)? {
            if constrained(&step) {
                intervals.extend(self.enabled_delays(state, &step)?);
            }
        }
        Ok(covers(&intervals, delay))
    }

    /// The largest constant any guard, time invariant or reset of the model
    /// compares a clock with.
    fn max_constant(&self) -> Rational {
        let model = self.model;
        let guards = model
            .guards()
            .iter()
            .flat_map(|guard| guard.parts())
            .flat_map(|part| part.clocks())
            .map(|constraint| constraint.bound().clone());
        let invariants = model
            .invariants()
            .iter()
            .flat_map(|invariant| invariant.bounds())
            .map(|constraint| constraint.bound().clone());
        let resets = model
            .resets()
            .iter()
            .flat_map(|resets| resets.resets())
            .map(|reset| reset.value().clone());
        guards.chain(invariants).chain(resets).fold(
            zero(),
            |max, value| {
                if value > max {
                    value
                } else {
                    max
                }
            },
        )
    }

    /// FR-232 "Fairness over time": whether the loop of `behaviour` from
    /// position `entry` to its last position, repeated forever, satisfies
    /// weak fairness for the constraint naming the steps `constrained`
    /// accepts. It does unless, from some repetition on, the constraint is
    /// enabled at every instant of the loop, positions and the delays
    /// between them, while the loop takes none of its steps.
    ///
    /// A loop of two or more positions ends where it re-enters `entry`'s
    /// discrete state. A loop of one position is FR-231's idle tail: its
    /// stutter step after `stutter_delay`. A clock the loop never resets
    /// grows by the loop's total delay at each repetition, so the loop is
    /// read at the first repetition where every such clock is above every
    /// constant of the model, after which each repetition reads alike.
    ///
    /// # Errors
    ///
    /// As [`Self::guarded_steps`] and [`Self::enabled_delays`].
    pub fn weakly_fair(
        &self,
        behaviour: &super::TimedBehaviour,
        entry: usize,
        stutter_delay: &Rational,
        constrained: impl Fn(&GuardedStep) -> bool,
    ) -> Result<bool, TimedStop> {
        let positions = behaviour.positions();
        let Some(looped) = positions.get(entry..).filter(|looped| !looped.is_empty()) else {
            return Ok(true);
        };
        let steps: Vec<&GuardedStep> = looped
            .iter()
            .skip(1)
            .filter_map(super::TimedPosition::step)
            .collect();
        if steps.iter().any(|step| constrained(step)) {
            return Ok(true);
        }
        // Each window of one repetition: the delay before it and the step
        // that ends it, the idle tail's being the stutter step.
        let windows: Vec<(Rational, Option<&GuardedStep>)> = if looped.len() == 1 {
            vec![(stutter_delay.clone(), None)]
        } else {
            looped
                .windows(2)
                .map(|pair| {
                    let tick = pair[1]
                        .step
                        .as_ref()
                        .map_or_else(zero, |step| self.tick_of(&step.operation));
                    (
                        pair[1].stamp.sub(&pair[0].stamp).sub(&tick),
                        pair[1].step.as_ref(),
                    )
                })
                .collect()
        };
        let total = windows
            .iter()
            .fold(zero(), |sum, (delay, _)| sum.add(delay));
        // Shift each clock the loop never resets past every constant.
        let mut clocks = looped[0].state.clocks.clone();
        if total > zero() {
            let ceiling = self.max_constant();
            let reset = |place: usize| {
                steps
                    .iter()
                    .any(|step| step.resets.iter().any(|(reset, _)| *reset == place))
            };
            let mut shifted = clocks.0.clone();
            for (place, value) in shifted.iter_mut().enumerate() {
                if reset(place) {
                    continue;
                }
                while *value <= ceiling {
                    *value = value.add(&total);
                }
            }
            clocks = super::ClockValuation(shifted);
        }
        let mut state = TimedState {
            discrete: looped[0].state.discrete.clone(),
            clocks,
        };
        for (delay, step) in windows {
            if !self.enabled_throughout(&state, &delay, &constrained)? {
                return Ok(true);
            }
            if let Some(step) = step {
                state = TimedState {
                    discrete: step.post.clone(),
                    clocks: step.reset(&state.clocks.delayed(&delay)),
                };
            } else {
                state.clocks = state.clocks.delayed(&delay);
            }
        }
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use quire_exact::Integer;

    use super::*;

    fn exact(value: i64) -> Rational {
        Rational::from_integer(Integer::from(value))
    }

    #[test]
    fn half_open_intervals_cover_only_with_their_shared_end() {
        let closed = DelayInterval::of(&exact(0), ClockOp::LessOrEqual, &exact(1));
        let open_above = DelayInterval::of(&exact(0), ClockOp::Greater, &exact(1));
        let closed_above = DelayInterval::of(&exact(0), ClockOp::GreaterOrEqual, &exact(1));
        let open = DelayInterval::of(&exact(0), ClockOp::Less, &exact(1));
        assert!(covers(&[closed.clone(), open_above], &exact(5)));
        assert!(covers(&[open.clone(), closed_above], &exact(5)));
        assert!(!covers(
            &[
                open,
                DelayInterval::of(&exact(0), ClockOp::Greater, &exact(1))
            ],
            &exact(5)
        ));
        assert!(covers(std::slice::from_ref(&closed), &exact(1)));
        assert!(!covers(&[closed], &exact(2)));
        assert!(!covers(
            &[DelayInterval::of(&exact(0), ClockOp::Greater, &exact(0))],
            &exact(1)
        ));
    }

    #[test]
    fn an_equality_holds_at_one_delay() {
        let point = DelayInterval::of(&exact(1), ClockOp::Equal, &exact(3));
        assert!(point.contains(&exact(2)));
        assert!(!point.contains(&exact(1)));
        assert!(!point.is_empty());
        assert!(DelayInterval::of(&exact(4), ClockOp::Equal, &exact(3))
            .meet(&DelayInterval::all())
            .is_empty());
    }
}
