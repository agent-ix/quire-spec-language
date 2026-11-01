// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-253/FR-254: one exact timed race and the ordered tie steps it takes.

use qsl_semantics::check::{DelayDistribution, Workload};
use quire_exact::{Integer, Rational};
use quire_semantic_value::model_transition::ModelTransition;

use super::{GuardedStep, TimeLock, TimedState, TimedStop, TimedSystem, DEFAULT_MAX_CLOCKS};
use crate::simulation::choice::{weighted_choice_counted, ChoicePoint};
use crate::simulation::model::TransitionId;
use crate::simulation::sampled::{draw_timed_delay, TimedDelayRefusal};
use crate::simulation::sampled::{SampledBehaviour, SampledStep};
use crate::simulation::StateKey;

/// One transition taken by a timed race. The first transition has the
/// minimum sampled delay; later transitions in the same tie have delay 0.
#[derive(Clone, Debug)]
pub struct TimedRaceStep {
    /// The operation transition.
    pub transition: ModelTransition,
    /// The exact delay before this transition.
    pub delay: Rational,
    /// The reached timed state.
    pub state: TimedState,
    /// This ordered race outcome's probability factor.
    pub probability: Rational,
    /// The choice indices used to select its delay and tie position.
    pub choices: Vec<u32>,
}

/// One stochastic race result at a timed state.
#[derive(Clone, Debug)]
pub struct TimedRace {
    /// The ordered operation steps taken at the least sampled delay.
    pub steps: Vec<TimedRaceStep>,
    /// The timed state after the tie sequence.
    pub state: TimedState,
    /// The joint probability of the delay draws and ordered tie outcome.
    pub probability: Rational,
    /// Whether the state had no racing identity and therefore takes the idle
    /// tail.
    pub idle: bool,
    /// The race contains scheduler-owned free delays, so it is a Markov
    /// decision outcome rather than a sampled successor. The recorded draws
    /// show stochastic delay choices made before that scheduler decision.
    pub scheduler_choice: Option<TimedSchedulerChoice>,
}

/// A Markov decision race at a state with one or more free-delay identities.
#[derive(Clone, Debug)]
pub struct TimedSchedulerChoice {
    /// The timed state where the scheduler chooses the free delay.
    pub state: StateKey,
    /// Stochastic identities' exact draws at this state.
    pub draws: Vec<TimedRaceDraw>,
    /// Enabled free-delay identities and their admissible windows.
    pub free_delays: Vec<super::ScheduledWindow>,
    /// The joint probability of the draws in `draws`.
    pub probability: Rational,
}

/// One stochastic delay draw made before a scheduler-owned free delay.
#[derive(Clone, Debug)]
pub struct TimedRaceDraw {
    /// The operation whose delay was sampled.
    pub transition: ModelTransition,
    /// The exact sampled delay.
    pub delay: Rational,
    /// The probability of that delay draw.
    pub probability: Rational,
    /// The choice index used to make the draw, if it consumed one.
    pub choices: Vec<u32>,
}

/// Why a timed race has no sampled successor.
#[derive(Clone, Debug, PartialEq, thiserror::Error)]
pub enum TimedRaceStop {
    /// A free delay races, or a delay law has no defined distribution in its
    /// window. The state key and operation identify the refusal.
    #[error("no stochastic delay is defined for {identity:?} at {state:?}")]
    NotStochastic {
        /// The full timed-state key.
        state: StateKey,
        /// The declaration identity of the scheduled operation.
        identity: ModelTransition,
    },
    /// The timed model stopped while its windows or transitions were read.
    #[error("timed expansion stopped: {0:?}")]
    Timed(TimedStop),
    /// The workload does not assign positive weight to a tied operation.
    #[error("a workload weight is missing for a tied operation")]
    MissingWeight,
    /// The choice total exceeds the sampler's 256-bit range.
    #[error("a race choice exceeds the sampler's draw range")]
    DrawRange,
    /// The timed state could not be assigned its canonical state key.
    #[error("the timed state has no canonical state key")]
    StateKey,
    /// A sampled tie step is no longer enabled at its turn.
    #[error("a tied transition stopped being enabled")]
    TieDisabled,
}

/// A sampled timed prefix, with the delay carried on each transition step.
#[derive(Clone, Debug)]
pub struct TimedSample {
    /// The seed-derived trace index.
    pub trace: u64,
    /// The canonical key of the initial timed state.
    pub start: StateKey,
    /// The sampled steps, including each exact delay.
    pub steps: Vec<SampledStep>,
    /// Timed positions, initial position first, aligned with `stamps`.
    pub states: Vec<TimedState>,
    /// Exact absolute time stamp of each state.
    pub stamps: Vec<Rational>,
    /// The reached timed state.
    pub state: TimedState,
    /// The final time stamp.
    pub stamp: Rational,
    /// Whether the prefix ended at an idle tail.
    pub idle: bool,
}

/// A timed sampling request refused before a sample can be completed.
#[derive(Clone, Debug, PartialEq, thiserror::Error)]
pub enum TimedSampleStop {
    /// A race could not be sampled.
    #[error(transparent)]
    Race(#[from] TimedRaceStop),
    /// The sampled prefix exceeded its request budget before its horizon.
    #[error("the timed sample exceeded its maximum number of steps")]
    MaxSampleSteps,
    /// The sample reached a local time-lock before its horizon.
    #[error("the timed sample reached a local time-lock at {state:?}")]
    TimeLockedSample {
        /// The key of the locally time-locked state.
        state: StateKey,
    },
}

/// A sampled timed witness disagrees with deterministic recomputation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TimedSampleDisagreement {
    /// The recorded step differs from the seed-derived step.
    Step {
        /// The first differing step index.
        step: u64,
    },
    /// All recorded steps agree but the final time or idle disposition does not.
    EndPosition,
}

/// Replay result for a timed sample witness.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TimedSampleReplay {
    /// The sample, including every exact delay, recomputes from its seed.
    Reproduced,
    /// The recomputed sample differs from the witness.
    Disagrees(TimedSampleDisagreement),
}

/// Sample one timed prefix through `horizon`, repeating FR-253's race.
///
/// The first position is at time zero. The sample stops when a race has no
/// scheduled identity, or when the next position would pass the horizon.
/// Tied transitions are retained in their race order; only the first has a
/// positive delay and every later tied transition has delay zero.
pub fn sample_timed(
    system: &TimedSystem<'_, '_>,
    initial: &TimedState,
    workload: &Workload,
    at: ChoicePoint,
    q: u32,
    horizon: &Rational,
    max_sample_steps: u64,
) -> Result<TimedSample, TimedSampleStop> {
    let start = system
        .state_key(initial)
        .map_err(|_| TimedRaceStop::StateKey)?;
    let mut state = initial.clone();
    let mut stamp = Rational::from_integer(Integer::zero());
    let mut steps = Vec::new();
    let mut states = vec![state.clone()];
    let mut stamps = vec![stamp.clone()];
    let mut idle = false;
    while stamp <= *horizon {
        let time_lock = system
            .time_lock(
                &state,
                usize::try_from(DEFAULT_MAX_CLOCKS.get()).unwrap_or(usize::MAX),
            )
            .map_err(TimedRaceStop::Timed)?;
        if time_lock == Some(TimeLock::Local) {
            return Err(TimedSampleStop::TimeLockedSample {
                state: system
                    .state_key(&state)
                    .map_err(|_| TimedRaceStop::StateKey)?,
            });
        }
        let point = ChoicePoint {
            step: u64::try_from(steps.len()).unwrap_or(u64::MAX),
            choice: 0,
            ..at
        };
        let race = sample_timed_race(system, &state, workload, point, q)?;
        if let Some(choice) = &race.scheduler_choice {
            let identity = choice
                .free_delays
                .first()
                .ok_or(TimedRaceStop::StateKey)?
                .transition
                .transition()
                .clone();
            return Err(TimedRaceStop::NotStochastic {
                state: choice.state.clone(),
                identity,
            }
            .into());
        }
        if race.idle || race.steps.is_empty() {
            let remaining = horizon.sub(&stamp);
            if let Some((upper, true)) = system
                .admissible_delays(&state)
                .map_err(TimedRaceStop::Timed)?
                .upper()
                .filter(|(upper, _)| *upper < &remaining)
            {
                // An idle state with a closed finite invariant can still
                // reach a local time-lock before the sample horizon. Advance
                // the clock without adding a sampled transition, then let the
                // next loop iteration classify the boundary state.
                state = system
                    .delay(&state, upper)
                    .map_err(TimedRaceStop::Timed)?
                    .ok_or(TimedRaceStop::StateKey)?;
                stamp = stamp.add(upper);
                continue;
            }
            idle = true;
            break;
        }
        let mut advanced = false;
        for race_step in race.steps {
            let next_stamp = stamp.add(&race_step.delay);
            if next_stamp > *horizon {
                return Ok(TimedSample {
                    trace: at.trace,
                    start,
                    steps,
                    states,
                    stamps,
                    state,
                    stamp,
                    idle: false,
                });
            }
            if u64::try_from(steps.len()).unwrap_or(u64::MAX) >= max_sample_steps {
                return Err(TimedSampleStop::MaxSampleSteps);
            }
            let post_state = system
                .state_key(&race_step.state)
                .map_err(|_| TimedRaceStop::StateKey)?;
            steps.push(SampledStep {
                transition: TransitionId::Operation(race_step.transition),
                delay: Some(race_step.delay.clone()),
                post_state,
                choices: race_step.choices,
                random: Vec::new(),
                probability: race_step.probability,
                rewards: Vec::new(),
            });
            state = race_step.state;
            stamp = next_stamp;
            states.push(state.clone());
            stamps.push(stamp.clone());
            advanced = true;
        }
        if !advanced {
            idle = true;
            break;
        }
    }
    Ok(TimedSample {
        trace: at.trace,
        start,
        steps,
        states,
        stamps,
        state,
        stamp,
        idle,
    })
}

/// Recompute a recorded timed sample from its seed and compare every draw,
/// delay, successor and end position (FR-254 witness replay).
#[allow(clippy::too_many_arguments)] // each value is an explicit replay input
pub fn replay_timed_sample(
    system: &TimedSystem<'_, '_>,
    initial: &TimedState,
    workload: &Workload,
    at: ChoicePoint,
    q: u32,
    horizon: &Rational,
    max_sample_steps: u64,
    record: &TimedSample,
) -> Result<TimedSampleReplay, TimedSampleStop> {
    let replayed = sample_timed(system, initial, workload, at, q, horizon, max_sample_steps)?;
    if let Some((step, _)) = replayed
        .steps
        .iter()
        .zip(&record.steps)
        .enumerate()
        .find(|(_, (actual, expected))| actual != expected)
    {
        return Ok(TimedSampleReplay::Disagrees(
            TimedSampleDisagreement::Step {
                step: u64::try_from(step).unwrap_or(u64::MAX),
            },
        ));
    }
    let same_state = system.state_key(&replayed.state).ok() == system.state_key(&record.state).ok();
    let same_start = replayed.start == record.start;
    if replayed.steps.len() != record.steps.len()
        || replayed.trace != record.trace
        || !same_start
        || replayed.stamp != record.stamp
        || replayed.idle != record.idle
        || !same_state
    {
        return Ok(TimedSampleReplay::Disagrees(
            TimedSampleDisagreement::EndPosition,
        ));
    }
    Ok(TimedSampleReplay::Reproduced)
}

impl TimedSample {
    /// The portion usable by FR-193 witness storage.
    pub fn behaviour(&self) -> SampledBehaviour {
        SampledBehaviour {
            trace: self.trace,
            start: self.start.clone(),
            steps: self.steps.clone(),
        }
    }
}

struct Candidate {
    step: GuardedStep,
    delay: Rational,
    delay_probability: Rational,
    choices: Vec<u32>,
}

/// Sample and take the race at `state` (FR-253 SD-3, FR-254 SS-2).
///
/// Delays draw independently at consecutive choice indices. The workload
/// orders ties by weighted operation, then uniformly among that operation's
/// identities, without replacement.
pub fn sample_timed_race(
    system: &TimedSystem<'_, '_>,
    state: &TimedState,
    workload: &Workload,
    at: ChoicePoint,
    q: u32,
) -> Result<TimedRace, TimedRaceStop> {
    let windows = system
        .scheduled_windows(state)
        .map_err(TimedRaceStop::Timed)?;
    if windows.is_empty() {
        return Ok(TimedRace {
            steps: Vec::new(),
            state: state.clone(),
            probability: one(),
            idle: true,
            scheduler_choice: None,
        });
    }
    let key = system
        .state_key(state)
        .map_err(|_| TimedRaceStop::StateKey)?;
    let mut choice = at.choice;
    let mut candidates = Vec::new();
    let mut stochastic_draws = Vec::new();
    let mut free_delays = Vec::new();
    for window in windows {
        let Some(distribution) = &window.distribution else {
            free_delays.push(window);
            continue;
        };
        if matches!(distribution, DelayDistribution::Uniform)
            && window
                .intervals
                .iter()
                .any(|interval| interval.upper().is_none())
        {
            return Err(TimedRaceStop::NotStochastic {
                state: key,
                identity: window.transition.transition().clone(),
            });
        }
        let point = ChoicePoint { choice, ..at };
        let delay = match draw_timed_delay(distribution, &window.intervals, point, q) {
            Ok(delay) => delay,
            Err(TimedDelayRefusal::NoMass) => continue,
            Err(TimedDelayRefusal::UnboundedUniform | TimedDelayRefusal::GridExponent { .. }) => {
                return Err(TimedRaceStop::NotStochastic {
                    state: key,
                    identity: window.transition.transition().clone(),
                })
            }
        };
        let probability = delay_probability(distribution, &window.intervals, &delay, q);
        let consumes = delay_choice(distribution, &window.intervals, q);
        let choices = if consumes {
            let selected = choice;
            choice = choice.checked_add(1).ok_or(TimedRaceStop::DrawRange)?;
            vec![selected]
        } else {
            Vec::new()
        };
        let candidate = Candidate {
            step: window.transition,
            delay: delay.clone(),
            delay_probability: probability.clone(),
            choices: choices.clone(),
        };
        stochastic_draws.push(TimedRaceDraw {
            transition: candidate.step.transition().clone(),
            delay,
            probability,
            choices,
        });
        candidates.push(candidate);
    }
    if !free_delays.is_empty() {
        let probability = stochastic_draws
            .iter()
            .fold(one(), |product, draw| product.mul(&draw.probability));
        return Ok(TimedRace {
            steps: Vec::new(),
            state: state.clone(),
            probability: probability.clone(),
            idle: false,
            scheduler_choice: Some(TimedSchedulerChoice {
                state: key,
                draws: stochastic_draws,
                free_delays,
                probability,
            }),
        });
    }
    if candidates.is_empty() {
        return Ok(TimedRace {
            steps: Vec::new(),
            state: state.clone(),
            probability: one(),
            idle: true,
            scheduler_choice: None,
        });
    }
    let delay_mass = candidates_probability(&candidates);
    let least = candidates
        .iter()
        .map(|candidate| &candidate.delay)
        .min()
        .expect("a non-empty candidate list")
        .clone();
    let mut tied: Vec<Candidate> = candidates
        .into_iter()
        .filter(|candidate| candidate.delay == least)
        .collect();
    let delay = least;
    let mut ordered = Vec::new();
    let mut tie_factors = Vec::new();
    while !tied.is_empty() {
        let mut operations: Vec<(String, Rational, Vec<usize>)> = Vec::new();
        for (index, candidate) in tied.iter().enumerate() {
            let operation = candidate.step.operation().to_owned();
            if let Some((_, _, identities)) = operations
                .iter_mut()
                .find(|(text, _, _)| text == &operation)
            {
                identities.push(index);
                continue;
            }
            let weight = workload_weight(system, workload, &operation)?;
            operations.push((operation, weight, vec![index]));
        }
        let weights: Vec<_> = operations
            .iter()
            .map(|(_, weight, _)| weight.clone())
            .collect();
        let point = ChoicePoint { choice, ..at };
        let selected = choose(&weights, point)?;
        let (operation, operation_weight, identities) = &operations[selected.index];
        let operation_total = weights.iter().fold(zero(), |sum, weight| sum.add(weight));
        let op_probability = quotient(operation_weight, &operation_total);
        let identity_index = if identities.len() == 1 {
            0
        } else {
            let identity_weights = vec![one(); identities.len()];
            let point = ChoicePoint {
                choice: choice
                    .checked_add(u32::from(selected.consumed))
                    .ok_or(TimedRaceStop::DrawRange)?,
                ..at
            };
            choose(&identity_weights, point)?.index
        };
        let index = *identities
            .get(identity_index)
            .ok_or(TimedRaceStop::DrawRange)?;
        tie_factors.push(
            op_probability
                .div(&Rational::from_integer(Integer::from(
                    identities.len() as u64
                )))
                .expect("positive tie count"),
        );
        let mut candidate = tied.remove(index);
        if selected.consumed {
            candidate.choices.push(choice);
            choice = choice.checked_add(1).ok_or(TimedRaceStop::DrawRange)?;
        }
        if identities.len() > 1 {
            candidate.choices.push(choice);
            choice = choice.checked_add(1).ok_or(TimedRaceStop::DrawRange)?;
        }
        let _ = operation;
        ordered.push(candidate);
    }
    let mut current = state.clone();
    let mut steps = Vec::new();
    let mut remaining_probability = delay_mass;
    for (index, candidate) in ordered.into_iter().enumerate() {
        let after_delay = if index == 0 {
            system
                .delay(&current, &delay)
                .map_err(TimedRaceStop::Timed)?
        } else {
            Some(current.clone())
        }
        .ok_or(TimedRaceStop::TieDisabled)?;
        let reached = system
            .steps(&after_delay)
            .map_err(TimedRaceStop::Timed)?
            .into_iter()
            .find(|(step, _)| step.transition() == candidate.step.transition())
            .map(|(_, next)| next);
        let Some(reached) = reached else {
            continue;
        };
        let factor = if index == 0 {
            remaining_probability.mul(&tie_factors[index])
        } else {
            tie_factors[index].clone()
        };
        if index == 0 {
            remaining_probability = one();
        }
        steps.push(TimedRaceStep {
            transition: candidate.step.transition().clone(),
            delay: if index == 0 { delay.clone() } else { zero() },
            state: reached.clone(),
            probability: factor,
            choices: candidate.choices,
        });
        current = reached;
    }
    let probability = steps.iter().fold(one(), |product, step: &TimedRaceStep| {
        product.mul(&step.probability)
    });
    Ok(TimedRace {
        steps,
        state: current,
        probability,
        idle: false,
        scheduler_choice: None,
    })
}

struct Chosen {
    index: usize,
    consumed: bool,
}

fn choose(weights: &[Rational], at: ChoicePoint) -> Result<Chosen, TimedRaceStop> {
    if weights.len() == 1 {
        return Ok(Chosen {
            index: 0,
            consumed: false,
        });
    }
    let (index, _) = weighted_choice_counted(at, weights);
    Ok(Chosen {
        index: index.map_err(|_| TimedRaceStop::DrawRange)?,
        consumed: true,
    })
}

fn workload_weight(
    system: &TimedSystem<'_, '_>,
    workload: &Workload,
    operation: &str,
) -> Result<Rational, TimedRaceStop> {
    workload
        .weights()
        .iter()
        .find(|(candidate, _)| {
            system
                .system()
                .operation_text(candidate.declaring(), candidate.name())
                == Some(operation)
        })
        .map(|(_, weight)| weight.clone())
        .ok_or(TimedRaceStop::MissingWeight)
}

fn delay_choice(
    distribution: &DelayDistribution,
    intervals: &[super::DelayInterval],
    q: u32,
) -> bool {
    match distribution {
        DelayDistribution::Discrete(values) => {
            values
                .iter()
                .filter(|(delay, _)| intervals.iter().any(|window| window.contains(delay)))
                .count()
                > 1
        }
        DelayDistribution::Uniform | DelayDistribution::UniformRange { .. } => {
            q > 0
                && intervals.iter().any(|window| {
                    window
                        .upper()
                        .is_none_or(|(upper, _)| upper > window.lower().0)
                })
        }
        DelayDistribution::Exponential { .. } => q > 0,
    }
}

fn delay_probability(
    distribution: &DelayDistribution,
    intervals: &[super::DelayInterval],
    delay: &Rational,
    q: u32,
) -> Rational {
    match distribution {
        DelayDistribution::Discrete(values) => {
            let selected = values
                .iter()
                .find(|(value, _)| value == delay)
                .map(|(_, weight)| weight);
            let total = values
                .iter()
                .filter(|(value, _)| intervals.iter().any(|window| window.contains(value)))
                .fold(zero(), |sum, (_, weight)| sum.add(weight));
            selected.map_or_else(zero, |weight| quotient(weight, &total))
        }
        _ if !delay_choice(distribution, intervals, q) => one(),
        _ => Rational::from_integer(Integer::one()).divided_by_power_of_two(u64::from(q)),
    }
}

fn candidates_probability(candidates: &[Candidate]) -> Rational {
    candidates.iter().fold(one(), |product, candidate| {
        product.mul(&candidate.delay_probability)
    })
}

fn quotient(numerator: &Rational, denominator: &Rational) -> Rational {
    numerator.div(denominator).expect("positive workload total")
}

fn one() -> Rational {
    Rational::from_integer(Integer::one())
}

fn zero() -> Rational {
    Rational::from_integer(Integer::zero())
}
