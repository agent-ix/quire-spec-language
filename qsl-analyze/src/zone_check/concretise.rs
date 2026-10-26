// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-241 for a finite symbolic path: exact rational delays.
//!
//! The times of a path's steps `T_1 <= ... <= T_n` (with `T_0 = 0`) obey
//! difference constraints only: a clock last reset at step `r` to `c` reads
//! `T_k - T_r + c` at step `k`, so every guard, time invariant and urgency
//! of the path is a bound on `T_k - T_r`. Those constraints form a zone over
//! the step times, kept in the engine's [`Dbm`]. Each delay is then fixed in
//! order, the least admissible one when the delay set has a least element,
//! otherwise the midpoint of a bounded set, otherwise its lower end plus one
//! unit; a closed zone guarantees that every earlier choice extends.

use num_bigint::BigInt;
use num_integer::Integer as _;
use qsl_eval::simulation::model::{
    ClockValuation, GuardedStep, PlaceConstraint, TimedState, TimedSystem,
};
use qsl_replay::ModelStep;
use qsl_semantics::check::ClockOp;
use quire_exact::{Integer, Rational};

use super::search::scaled;
use super::{Bound, Dbm, DbmError};

/// One step of a symbolic path, with the members its delay obeys.
pub(super) struct Stage {
    /// The time invariants applying before the step.
    pub invariants: Vec<PlaceConstraint>,
    /// Whether time may elapse before the step.
    pub elapses: bool,
    /// The time invariants applying after it.
    pub target: Vec<PlaceConstraint>,
    /// The convex part of its guard taken.
    pub part: usize,
    /// The step.
    pub step: GuardedStep,
}

/// Where the path's last valuation must lie.
pub(super) enum Target {
    /// Anywhere the path reaches.
    Any,
    /// Within this zone over clock places, in scaled units.
    Within(Dbm),
}

/// A concretised path.
pub(super) struct Concrete {
    /// Its steps, each with its delay and post-state digest.
    pub prefix: Vec<ModelStep>,
    /// Its last clock valuation, every place in order.
    pub last_clocks: Vec<Rational>,
    /// The time stamp of its last position.
    pub last_stamp: Rational,
}

/// Why a path has no concretisation.
#[derive(Debug)]
pub(super) enum NotConcretised {
    /// Its constraints have no solution.
    Infeasible,
    /// A part index names no part of its step.
    Part,
    /// A matrix operation failed.
    Dbm,
    /// A timed state did not encode.
    Digest,
}

impl From<DbmError> for NotConcretised {
    fn from(_: DbmError) -> Self {
        Self::Dbm
    }
}

/// The last reset of each place: the step that reset it and the value.
struct Resets(Vec<(usize, Rational)>);

impl Resets {
    /// The bound on `T_at - T_r` that `place op bound` gives at step `at`.
    fn constrain(
        &self,
        times: &mut Dbm,
        at: usize,
        constraint: &PlaceConstraint,
        scale: &BigInt,
    ) -> Result<(), NotConcretised> {
        let (since, value) = self
            .0
            .get(constraint.place())
            .ok_or(NotConcretised::Infeasible)?;
        let edge = scaled(constraint.bound(), scale) - scaled(value, scale);
        apply(times, at, *since, constraint.op(), edge)
    }
}

/// `T_at - T_since op edge`.
fn apply(
    times: &mut Dbm,
    at: usize,
    since: usize,
    op: ClockOp,
    edge: BigInt,
) -> Result<(), NotConcretised> {
    match op {
        ClockOp::Less => times.constrain(at, since, Bound::lt(edge))?,
        ClockOp::LessOrEqual => times.constrain(at, since, Bound::le(edge))?,
        ClockOp::Equal => {
            times.constrain(at, since, Bound::le(edge.clone()))?;
            times.constrain(since, at, Bound::le(-edge))?;
        }
        ClockOp::GreaterOrEqual => times.constrain(since, at, Bound::le(-edge))?,
        ClockOp::Greater => times.constrain(since, at, Bound::lt(-edge))?,
    }
    Ok(())
}

/// FR-241: the path from the initial zero valuation through `stages`, its
/// last valuation in `target`, with exact delays; `scale` is the zone's
/// scale factor.
pub(super) fn concretise(
    timed: &TimedSystem<'_, '_>,
    stages: &[Stage],
    target: &Target,
    scale: &BigInt,
) -> Result<Concrete, NotConcretised> {
    let places = timed.places().len();
    let zero = Rational::from_integer(Integer::zero());
    let mut resets = Resets(vec![(0, zero.clone()); places]);
    let mut times = Dbm::universe(stages.len() + 1);
    for (index, stage) in stages.iter().enumerate() {
        let at = index + 1;
        times.constrain(at - 1, at, Bound::le(BigInt::from(0)))?;
        if !stage.elapses {
            times.constrain(at, at - 1, Bound::le(BigInt::from(0)))?;
        }
        let part = stage
            .step
            .parts()
            .get(stage.part)
            .ok_or(NotConcretised::Part)?;
        for constraint in stage.invariants.iter().chain(part) {
            resets.constrain(&mut times, at, constraint, scale)?;
        }
        for (place, value) in stage.step.resets() {
            if let Some(slot) = resets.0.get_mut(*place) {
                *slot = (at, value.clone());
            }
        }
        for constraint in &stage.target {
            resets.constrain(&mut times, at, constraint, scale)?;
        }
    }
    if let Target::Within(zone) = target {
        let last = stages.len();
        for i in 0..zone.dim() {
            for j in (0..zone.dim()).filter(|&j| j != i) {
                let Bound::Finite { value, strict } = zone.bound(i, j)? else {
                    continue;
                };
                let op = if *strict {
                    ClockOp::Less
                } else {
                    ClockOp::LessOrEqual
                };
                // `x_i - x_j` where clock `k >= 1` reads `T_n - T_r + c`.
                let read = |k: usize| -> (usize, BigInt) {
                    if k == 0 {
                        (last, BigInt::from(0))
                    } else {
                        let (since, value) = &resets.0[k - 1];
                        (*since, scaled(value, scale))
                    }
                };
                let ((since_i, c_i), (since_j, c_j)) = (read(i), read(j));
                // (T_n - T_si + c_i) - (T_n - T_sj + c_j) = T_sj - T_si + c_i - c_j,
                // with the reference clock read as `T_n - T_n`.
                let (plus, minus) = match (i, j) {
                    (0, _) => (since_j, last),
                    (_, 0) => (last, since_i),
                    _ => (since_j, since_i),
                };
                apply(&mut times, plus, minus, op, value - c_i + c_j)?;
            }
        }
    }
    if times.is_empty() {
        return Err(NotConcretised::Infeasible);
    }
    let mut unit = scale.clone();
    let mut stamps = vec![zero.clone()];
    let mut prefix = Vec::with_capacity(stages.len());
    let mut values = vec![zero.clone(); places];
    let mut last_reset = vec![0_usize; places];
    for (index, stage) in stages.iter().enumerate() {
        let at = index + 1;
        let delay = loop {
            let Bound::Finite {
                value: low,
                strict: open,
            } = times.bound(at - 1, at)?.clone()
            else {
                return Err(NotConcretised::Infeasible);
            };
            let low = -low;
            if !open {
                break low;
            }
            match times.bound(at, at - 1)?.clone() {
                Bound::Infinite => break low + &unit,
                Bound::Finite { value: high, .. } => {
                    let sum = low + high;
                    if sum.is_even() {
                        break sum / 2;
                    }
                    times.scale_by(&BigInt::from(2));
                    unit *= 2;
                }
            }
        };
        times.constrain(at, at - 1, Bound::le(delay.clone()))?;
        times.constrain(at - 1, at, Bound::le(-delay.clone()))?;
        if times.is_empty() {
            return Err(NotConcretised::Infeasible);
        }
        let delay = Rational::new(Integer::from_big(delay), Integer::from_big(unit.clone()))
            .map_err(|_| NotConcretised::Infeasible)?;
        let stamp = stamps[index].add(&delay);
        stamps.push(stamp.clone());
        for (place, value) in values.iter_mut().enumerate() {
            let since = last_reset[place];
            *value = stamp
                .sub(&stamps[since])
                .add(&resets_value(stages, place, since));
        }
        for (place, reset) in stage.step.resets() {
            if let Some(slot) = values.get_mut(*place) {
                slot.clone_from(reset);
                last_reset[*place] = at;
            }
        }
        let state = TimedState::new(
            stage.step.post().clone(),
            ClockValuation::new(values.clone()),
        );
        let post_state = timed
            .state_digest(&state)
            .map_err(|_| NotConcretised::Digest)?;
        prefix.push(ModelStep {
            transition: stage.step.transition().clone(),
            post_state,
            delay: Some(delay),
        });
    }
    Ok(Concrete {
        prefix,
        last_clocks: values,
        last_stamp: stamps.last().cloned().unwrap_or(zero),
    })
}

/// The value place `place` was reset to at step `since`, 0 at step 0.
fn resets_value(stages: &[Stage], place: usize, since: usize) -> Rational {
    since
        .checked_sub(1)
        .and_then(|index| stages.get(index))
        .and_then(|stage| {
            stage
                .step
                .resets()
                .iter()
                .rev()
                .find(|(reset, _)| *reset == place)
                .map(|(_, value)| value.clone())
        })
        .unwrap_or_else(|| Rational::from_integer(Integer::zero()))
}
