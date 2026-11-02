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
use qsl_eval::simulation::StateKey;
use qsl_replay::{ModelStep, TaskAction, TaskSearchTransition, TaskTransitionIdentity};
use qsl_semantics::check::ClockOp;
use quire_exact::{Integer, Rational};
use std::collections::BTreeMap;

use super::search::scaled;
use super::{Bound, Dbm, DbmError};

/// One position of a symbolic path, with the members its delay obeys.
#[derive(Clone)]
pub(super) struct Stage {
    /// The time invariants applying before the position.
    pub invariants: Vec<PlaceConstraint>,
    /// Whether time may elapse before the position.
    pub elapses: bool,
    /// The time invariants applying after it.
    pub target: Vec<PlaceConstraint>,
    /// The step and the convex part of its guard taken; `None` for an
    /// idle-tail stutter position, which comes after a positive delay.
    pub step: Option<(GuardedStep, usize)>,
    /// What a claim automaton requires of its own clocks at the position
    /// (FR-242), read after the step's resets.
    pub position: Vec<PlaceConstraint>,
    /// The claim automaton's clocks reset at the position, after its
    /// guard is read: indices past the clock places.
    pub position_resets: Vec<usize>,
    /// The zone over clock places, in units of the concretisation's scale,
    /// the valuation before the position's step, after its delay, lies in:
    /// the cell of the liveness product's symbolic state the step leaves
    /// (FR-240).
    pub before: Option<Dbm>,
    /// The zone over clock places, in units of the concretisation's scale,
    /// the valuation after the position lies in: the cell of the symbolic
    /// state a liveness product enters there (FR-240).
    pub piece: Option<Dbm>,
    /// Whether the stage is no position but an instant a delay passes
    /// through, from one cell of a liveness product into the next: it has
    /// no step, and its delay is part of the next step's.
    pub silent: bool,
    /// The delay before the position, when it is fixed: a lasso's loop
    /// copies repeat the loop's solved delays (FR-241), in units of the
    /// concretisation's scale.
    pub fixed: Option<BigInt>,
    /// The exact source/task transition and joint post-state key when this
    /// stage belongs to a task-product witness.
    pub task_product_step: Option<(TaskSearchTransition, StateKey)>,
    /// Exact task-service clock decrements at a task completion edge.
    pub task_subtractions: Vec<(usize, Rational)>,
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
    /// Its steps, each with its delay and post-state key; a stutter
    /// position adds none.
    pub prefix: Vec<ModelStep>,
    /// The same path with task scheduler edges retained explicitly.
    pub task_product_prefix: Vec<ModelStep<TaskSearchTransition>>,
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
    /// A lasso's loop needed more constraints than the run's budget allows
    /// (FR-241's elimination, charged to `max_symbolic_transitions`).
    Exhausted,
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

impl Resets {
    /// The bounds of `zone`, over clock places, on the valuation at step
    /// `at`, where clock `k >= 1` reads `T_at - T_r + c`.
    fn within(
        &self,
        times: &mut Dbm,
        at: usize,
        zone: &Dbm,
        scale: &BigInt,
    ) -> Result<(), NotConcretised> {
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
                let read = |k: usize| -> Result<(usize, BigInt), NotConcretised> {
                    if k == 0 {
                        Ok((at, BigInt::from(0)))
                    } else {
                        let (since, value) = self.0.get(k - 1).ok_or(NotConcretised::Infeasible)?;
                        Ok((*since, scaled(value, scale)))
                    }
                };
                let ((since_i, c_i), (since_j, c_j)) = (read(i)?, read(j)?);
                // (T_at - T_si + c_i) - (T_at - T_sj + c_j) = T_sj - T_si + c_i - c_j,
                // with the reference clock read as `T_at - T_at`.
                let (plus, minus) = match (i, j) {
                    (0, _) => (since_j, at),
                    (_, 0) => (at, since_i),
                    _ => (since_j, since_i),
                };
                apply(times, plus, minus, op, value - c_i + c_j)?;
            }
        }
        Ok(())
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
/// scale factor and `clocks` its number of clocks: the clock places, then
/// any clock a claim automaton adds, which no step resets.
pub(super) fn concretise(
    timed: &TimedSystem<'_, '_>,
    clocks: usize,
    stages: &[Stage],
    target: &Target,
    scale: &BigInt,
) -> Result<Concrete, NotConcretised> {
    let places = clocks;
    let zero = Rational::from_integer(Integer::zero());
    let mut resets = Resets(vec![(0, zero.clone()); places]);
    let mut times = Dbm::universe(stages.len() + 1);
    for (index, stage) in stages.iter().enumerate() {
        let at = index + 1;
        times.constrain(at - 1, at, Bound::le(BigInt::from(0)))?;
        if !stage.elapses {
            times.constrain(at, at - 1, Bound::le(BigInt::from(0)))?;
        }
        let part: &[PlaceConstraint] = match &stage.step {
            Some((step, part)) => step.parts().get(*part).ok_or(NotConcretised::Part)?,
            // A stutter position reads the state again. One at zero delay
            // re-reads the same state at the same stamp, so it only marks
            // the idle tail and is taken as is: no positive delay is
            // required of it (SR-1631001 FND-004).
            None => &[],
        };
        for constraint in stage.invariants.iter().chain(part) {
            resets.constrain(&mut times, at, constraint, scale)?;
        }
        if let Some(before) = &stage.before {
            resets.within(&mut times, at, before, scale)?;
        }
        if let Some((step, _)) = &stage.step {
            let before = resets.0.clone();
            for &(target, source) in step.carries() {
                if let (Some(slot), Some(value)) = (resets.0.get_mut(target), before.get(source)) {
                    *slot = value.clone();
                }
            }
            for (place, value) in step.resets() {
                if let Some(slot) = resets.0.get_mut(*place) {
                    *slot = (at, value.clone());
                }
            }
        }
        for constraint in stage.target.iter().chain(&stage.position) {
            resets.constrain(&mut times, at, constraint, scale)?;
        }
        for &clock in &stage.position_resets {
            if let Some(slot) = resets.0.get_mut(clock) {
                *slot = (at, zero.clone());
            }
        }
        for (clock, amount) in &stage.task_subtractions {
            let Some((_, value)) = resets.0.get_mut(*clock) else {
                return Err(NotConcretised::Infeasible);
            };
            *value = value.sub(amount);
        }
        if let Some(piece) = &stage.piece {
            resets.within(&mut times, at, piece, scale)?;
        }
        if let Some(delay) = &stage.fixed {
            times.constrain(at, at - 1, Bound::le(delay.clone()))?;
            times.constrain(at - 1, at, Bound::le(-delay.clone()))?;
        }
    }
    if let Target::Within(zone) = target {
        resets.within(&mut times, stages.len(), zone, scale)?;
    }
    if times.is_empty() {
        return Err(NotConcretised::Infeasible);
    }
    let mut unit = scale.clone();
    let mut stamps = vec![zero.clone()];
    let mut prefix = Vec::with_capacity(stages.len());
    let mut task_product_prefix = Vec::new();
    let mut task_identities = TaskWitnessIdentityState::default();
    let mut values = vec![zero.clone(); places];
    let mut reset_values = vec![zero.clone(); places];
    let mut last_reset = vec![0_usize; places];
    // The delays of the silent stages since the last step.
    let mut waited = zero.clone();
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
        let task_delay = delay.clone();
        let stamp = stamps[index].add(&delay);
        stamps.push(stamp.clone());
        for (place, value) in values.iter_mut().enumerate() {
            let since = last_reset[place];
            *value = stamp.sub(&stamps[since]).add(&reset_values[place]);
        }
        for &clock in &stage.position_resets {
            if let Some(slot) = values.get_mut(clock) {
                *slot = zero.clone();
                last_reset[clock] = at;
            }
        }
        for (clock, amount) in &stage.task_subtractions {
            let Some(value) = values.get_mut(*clock) else {
                return Err(NotConcretised::Infeasible);
            };
            *value = value.sub(amount);
            if let Some(reset_value) = reset_values.get_mut(*clock) {
                reset_value.clone_from(value);
            }
            if let Some(reset) = last_reset.get_mut(*clock) {
                *reset = at;
            }
        }
        let Some((step, _)) = &stage.step else {
            if let Some((transition, post_state)) = &stage.task_product_step {
                task_product_prefix.push(ModelStep {
                    transition: concrete_task_transition(transition, &stamp, &mut task_identities)?,
                    post_state: post_state.clone(),
                    delay: Some(task_delay),
                });
            }
            if stage.silent {
                waited = waited.add(&delay);
            }
            continue;
        };
        let delay = std::mem::replace(&mut waited, zero.clone()).add(&delay);
        let before = values.clone();
        for &(target, source) in step.carries() {
            if let (Some(value), Some(source_value)) = (values.get_mut(target), before.get(source))
            {
                value.clone_from(source_value);
                if let Some(slot) = reset_values.get_mut(target) {
                    slot.clone_from(source_value);
                }
                if let Some(slot) = last_reset.get_mut(target) {
                    *slot = at;
                }
            }
        }
        for (place, reset) in step.resets() {
            if let Some(slot) = values.get_mut(*place) {
                slot.clone_from(reset);
                if let Some(reset_value) = reset_values.get_mut(*place) {
                    reset_value.clone_from(reset);
                }
                last_reset[*place] = at;
            }
        }
        // The post-state key names the model's clock places only.
        let model_clocks = values.get(..timed.places().len()).unwrap_or(&values);
        let state = TimedState::new(
            step.post().clone(),
            ClockValuation::new(model_clocks.to_vec()),
        );
        let post_state = timed
            .state_key(&state)
            .map_err(|_| NotConcretised::Digest)?;
        prefix.push(ModelStep {
            transition: step.transition().clone(),
            post_state: post_state.clone(),
            delay: Some(delay.clone()),
        });
        if let Some((transition, task_post_state)) = &stage.task_product_step {
            task_product_prefix.push(ModelStep {
                transition: concrete_task_transition(transition, &stamp, &mut task_identities)?,
                post_state: task_post_state.clone(),
                delay: Some(task_delay),
            });
        }
    }
    Ok(Concrete {
        prefix,
        task_product_prefix,
        last_clocks: values,
        last_stamp: stamps.last().cloned().unwrap_or(zero),
    })
}

#[derive(Default)]
struct TaskWitnessIdentityState {
    next_job: BTreeMap<(String, String), u64>,
    next_generation: BTreeMap<(String, String, u64), u64>,
    active: Vec<(TaskTransitionIdentity, TaskTransitionIdentity)>,
}

fn concrete_task_transition(
    transition: &TaskSearchTransition,
    stamp: &Rational,
    identities: &mut TaskWitnessIdentityState,
) -> Result<TaskSearchTransition, NotConcretised> {
    let TaskSearchTransition::Task(identity) = transition else {
        return Ok(transition.clone());
    };
    let mut concrete = identity.as_ref().clone();
    if concrete.action == TaskAction::Activate {
        let task = (concrete.taskset.clone(), concrete.task.clone());
        let slot = (
            concrete.taskset.clone(),
            concrete.task.clone(),
            concrete.slot,
        );
        concrete.job = *identities.next_job.entry(task).or_default();
        let next_job = identities
            .next_job
            .entry((concrete.taskset.clone(), concrete.task.clone()))
            .or_default();
        *next_job = next_job.checked_add(1).ok_or(NotConcretised::Infeasible)?;
        concrete.generation = *identities.next_generation.entry(slot).or_default();
        // The search key quotients concrete activations by capture-avoiding
        // slot renaming. Restore the concrete job/generation for this path
        // before exposing the witness identity.
        concrete.deadline = stamp.add(&identity.deadline);
        identities
            .active
            .push((identity.as_ref().clone(), concrete.clone()));
    } else {
        let active = identities
            .active
            .iter()
            .position(|(symbolic, _)| same_task_job(symbolic, &concrete))
            .or_else(|| {
                identities.active.iter().position(|(_, actual)| {
                    actual.taskset == concrete.taskset
                        && actual.task == concrete.task
                        && actual.slot == concrete.slot
                        && actual.release == concrete.release
                })
            });
        let Some(active) = active else {
            return Err(NotConcretised::Infeasible);
        };
        let (_, actual) = identities
            .active
            .get(active)
            .ok_or(NotConcretised::Infeasible)?;
        concrete.job = actual.job;
        concrete.generation = actual.generation;
        concrete.deadline = actual.deadline.clone();
        if matches!(
            concrete.action,
            TaskAction::Complete | TaskAction::DeadlineMiss
        ) {
            let actual = identities.active.remove(active).1;
            identities.next_generation.insert(
                (actual.taskset.clone(), actual.task.clone(), actual.slot),
                actual
                    .generation
                    .checked_add(1)
                    .ok_or(NotConcretised::Infeasible)?,
            );
        }
    }
    Ok(TaskSearchTransition::Task(Box::new(concrete)))
}

fn same_task_job(left: &TaskTransitionIdentity, right: &TaskTransitionIdentity) -> bool {
    left.taskset == right.taskset
        && left.task == right.task
        && left.job == right.job
        && left.slot == right.slot
        && left.generation == right.generation
        && left.release == right.release
}
