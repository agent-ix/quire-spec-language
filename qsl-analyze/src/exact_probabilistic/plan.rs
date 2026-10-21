// SPDX-License-Identifier: AGPL-3.0-or-later
//! What a claim adds to a model state, and how a step advances it: FR-126's
//! monitor for a bounded event, the accumulator of a bounded measure, and no
//! control at all for an unbounded form (FR-196 "Behavior").

use qsl_eval::automaton::{
    translate_body, Atom, AutomatonError, Letter, Monitor, PropertyAutomaton, StateId,
};
use qsl_semantics::check::{
    Bound, BoundedMeasure, CheckedPredicate, CheckedProbabilisticClaim, Event, ProbForm,
    ReachTarget, TemporalProfile,
};
use quire_exact::{Identifier, Integer, Rational, Value};

use crate::model_check::false_extension::false_extension_rejects;

use super::{
    Accumulator, Amount, FormulaState, MeasurePhase, MonitorState, MonitorVerdict, PredicateRole,
    UndefinedRead,
};

/// Why a plan could not advance a state.
#[derive(Debug)]
pub(super) enum PlanFault {
    /// The monitor's automaton refused.
    Automaton(AutomatonError),
    /// A broken invariant of the product.
    Broken(&'static str),
}

impl From<AutomatonError> for PlanFault {
    fn from(error: AutomatonError) -> Self {
        Self::Automaton(error)
    }
}

/// The state predicates a claim reads at each model state, in the order
/// their truths are kept.
pub(super) type Reads<'c> = Vec<(UndefinedRead, &'c CheckedPredicate)>;

/// How the claim's control advances.
pub(super) enum Plan<'c> {
    /// A bounded event: FR-126's deterministic monitor, reading one letter
    /// per position.
    Formula(Box<Monitor<PropertyAutomaton>>),
    /// An `accumulate` or `steps` measure.
    Measure {
        /// The reward summed; none for `steps`, which sums 1 per step.
        reward: Option<&'c Identifier>,
        /// The comparison's threshold, above which an amount is one value.
        threshold: Rational,
        /// The window `h`.
        within: u64,
    },
    /// A fraction of the first `h + 1` positions.
    Fraction {
        /// The window `h`.
        window: u64,
        /// The reward each position is weighted by, when written.
        weight: Option<&'c Identifier>,
    },
    /// An unbounded form: no control, the predicates label the states.
    Labels,
}

impl<'c> Plan<'c> {
    /// The plan of `claim` and the predicates it reads, in order.
    ///
    /// # Errors
    ///
    /// The automaton's refusal when a bounded event does not translate.
    pub(super) fn of(
        claim: &'c CheckedProbabilisticClaim,
        max_automaton_states: u64,
    ) -> Result<(Self, Reads<'c>), PlanFault> {
        let role = |role, predicate| (UndefinedRead::Predicate(role), predicate);
        Ok(match &claim.form {
            ProbForm::Probability {
                event: Event::Formula { formula, .. },
                ..
            } => {
                let profile = TemporalProfile::from_identity(claim.profile.identity())
                    .ok_or(PlanFault::Broken("claim-profile-is-temporal"))?;
                let automaton = translate_body(formula.formula(), profile, max_automaton_states)?;
                let reads = automaton
                    .atoms()
                    .iter()
                    .map(|atom| {
                        formula
                            .condition(atom.index())
                            .map(|predicate| (UndefinedRead::Atom(*atom), predicate))
                            .ok_or(PlanFault::Broken("every-atom-has-a-predicate"))
                    })
                    .collect::<Result<_, _>>()?;
                (
                    Self::Formula(Box::new(Monitor::bad_prefix(
                        automaton,
                        max_automaton_states,
                    )?)),
                    reads,
                )
            }
            ProbForm::Probability {
                event: Event::Compare { measure, bound },
                ..
            } => Self::of_measure(measure, threshold(bound)),
            ProbForm::Quantile {
                measure,
                threshold: bound,
                ..
            } => Self::of_measure(measure, threshold(bound)),
            ProbForm::MeanOfFraction { fraction, .. } => (
                Self::Fraction {
                    window: fraction.window,
                    weight: fraction.weight.as_ref(),
                },
                vec![role(PredicateRole::Target, &fraction.predicate)],
            ),
            ProbForm::LongRunFraction { predicate, .. } => {
                (Self::Labels, vec![role(PredicateRole::Target, predicate)])
            }
            ProbForm::Reach { target, .. } => (
                Self::Labels,
                match target {
                    ReachTarget::Eventually(target) | ReachTarget::Always(target) => {
                        vec![role(PredicateRole::Target, target)]
                    }
                    ReachTarget::Until(from, until) => vec![
                        role(PredicateRole::From, from),
                        role(PredicateRole::Until, until),
                    ],
                },
            ),
            ProbForm::ExpectedReward { until, .. } => {
                (Self::Labels, vec![role(PredicateRole::Until, until)])
            }
        })
    }

    fn of_measure(measure: &'c BoundedMeasure, threshold: &Rational) -> (Self, Reads<'c>) {
        let role = |role, predicate| (UndefinedRead::Predicate(role), predicate);
        match measure {
            BoundedMeasure::Accumulate {
                reward,
                from,
                until,
                within,
            } => (
                Self::Measure {
                    reward: Some(reward),
                    threshold: threshold.clone(),
                    within: *within,
                },
                vec![
                    role(PredicateRole::From, from),
                    role(PredicateRole::Until, until),
                ],
            ),
            BoundedMeasure::Steps {
                from,
                until,
                within,
            } => (
                Self::Measure {
                    reward: None,
                    threshold: threshold.clone(),
                    within: *within,
                },
                vec![
                    role(PredicateRole::From, from),
                    role(PredicateRole::Until, until),
                ],
            ),
            BoundedMeasure::Fraction(fraction) => (
                Self::Fraction {
                    window: fraction.window,
                    weight: fraction.weight.as_ref(),
                },
                vec![role(PredicateRole::Target, &fraction.predicate)],
            ),
        }
    }

    /// The control and amounts at an initial model state, whose predicates
    /// read `truths`.
    pub(super) fn start(
        &mut self,
        truths: &[bool],
    ) -> Result<(Option<MonitorState>, Option<Accumulator>), PlanFault> {
        match self {
            Self::Formula(monitor) => {
                let initial = monitor.initial()?;
                let state = read_letter(monitor, initial, truths)?;
                Ok((Some(MonitorState::Formula(state)), None))
            }
            Self::Measure {
                reward,
                threshold,
                within,
            } => {
                let [from, until] = truths else {
                    return Err(PlanFault::Broken("a-measure-reads-two-predicates"));
                };
                let phase = match (*from, *until) {
                    (false, _) => Activation::Inactive,
                    (true, false) => Activation::Active,
                    (true, true) => Activation::Complete,
                };
                Ok(measure_state(
                    reward.is_none(),
                    *within,
                    phase,
                    &Amount::Exact(zero()),
                    threshold,
                ))
            }
            Self::Fraction { window, weight } => {
                let remaining = window.saturating_add(1);
                Ok((
                    Some(MonitorState::Fraction { remaining }),
                    Some(if weight.is_some() {
                        Accumulator::WeightedFraction {
                            true_sum: zero(),
                            total_sum: zero(),
                        }
                    } else {
                        Accumulator::CountFraction {
                            true_count: 0,
                            position_count: 0,
                        }
                    }),
                ))
            }
            Self::Labels => Ok((None, None)),
        }
    }

    /// The control and amounts after a model step that leaves a state whose
    /// predicates read `source` and reaches one whose predicates read `post`,
    /// with the step's `rewards`.
    pub(super) fn step(
        &mut self,
        monitor: Option<MonitorState>,
        accumulator: Option<&Accumulator>,
        source: &[bool],
        post: &[bool],
        rewards: &[(Identifier, Value)],
    ) -> Result<(Option<MonitorState>, Option<Accumulator>), PlanFault> {
        match (self, monitor, accumulator) {
            (Self::Formula(monitor), Some(MonitorState::Formula(state)), None) => {
                let FormulaState::Running(running) = state else {
                    return Err(PlanFault::Broken("a-decided-state-is-not-stepped"));
                };
                let state = read_letter(monitor, running, post)?;
                Ok((Some(MonitorState::Formula(state)), None))
            }
            (
                Self::Measure {
                    reward,
                    threshold,
                    within: _,
                },
                Some(MonitorState::Measure { remaining, phase }),
                Some(held),
            ) => {
                let [from, until] = post else {
                    return Err(PlanFault::Broken("a-measure-reads-two-predicates"));
                };
                let remaining = remaining
                    .checked_sub(1)
                    .ok_or(PlanFault::Broken("a-decided-state-is-not-stepped"))?;
                let amount = match held {
                    Accumulator::Steps { amount, .. } | Accumulator::Reward { amount, .. } => {
                        amount
                    }
                    _ => return Err(PlanFault::Broken("a-measure-holds-an-amount")),
                };
                let (phase, amount) = match phase {
                    MeasurePhase::Inactive => (
                        match (*from, *until) {
                            (false, _) => Activation::Inactive,
                            (true, false) => Activation::Active,
                            (true, true) => Activation::Complete,
                        },
                        amount.clone(),
                    ),
                    MeasurePhase::Active => {
                        let step = match reward {
                            Some(name) => magnitude(rewards, name)?,
                            None => Rational::from_integer(Integer::one()),
                        };
                        (
                            if *until {
                                Activation::Complete
                            } else {
                                Activation::Active
                            },
                            add(amount, &step),
                        )
                    }
                    _ => return Err(PlanFault::Broken("a-decided-state-is-not-stepped")),
                };
                Ok(measure_state(
                    reward.is_none(),
                    remaining,
                    phase,
                    &amount,
                    threshold,
                ))
            }
            (
                Self::Fraction { weight, .. },
                Some(MonitorState::Fraction { remaining }),
                Some(held),
            ) => {
                let remaining = remaining
                    .checked_sub(1)
                    .ok_or(PlanFault::Broken("a-decided-state-is-not-stepped"))?;
                let [holds] = source else {
                    return Err(PlanFault::Broken("a-fraction-reads-one-predicate"));
                };
                let accumulator = match (held, weight) {
                    (
                        Accumulator::CountFraction {
                            true_count,
                            position_count,
                        },
                        None,
                    ) => Accumulator::CountFraction {
                        true_count: true_count + u64::from(*holds),
                        position_count: position_count + 1,
                    },
                    (
                        Accumulator::WeightedFraction {
                            true_sum,
                            total_sum,
                        },
                        Some(name),
                    ) => {
                        let weight = magnitude(rewards, name)?;
                        Accumulator::WeightedFraction {
                            true_sum: if *holds {
                                true_sum.add(&weight)
                            } else {
                                true_sum.clone()
                            },
                            total_sum: total_sum.add(&weight),
                        }
                    }
                    _ => return Err(PlanFault::Broken("a-fraction-holds-its-sums")),
                };
                Ok((
                    Some(MonitorState::Fraction { remaining }),
                    Some(accumulator),
                ))
            }
            (Self::Labels, None, None) => Ok((None, None)),
            _ => Err(PlanFault::Broken("control-matches-plan")),
        }
    }

    /// The verdict of a bounded event at a terminal model state, under the
    /// profile's false extension (FR-126's closure): the monitor reads the
    /// all-false letter until it rejects or repeats.
    pub(super) fn close(&mut self, running: StateId) -> Result<MonitorVerdict, PlanFault> {
        let Self::Formula(monitor) = self else {
            return Err(PlanFault::Broken("only-an-event-closes"));
        };
        Ok(if false_extension_rejects(monitor, running)? {
            MonitorVerdict::Reject
        } else {
            MonitorVerdict::Accept
        })
    }

    /// Whether a terminal model state closes the event instead of
    /// stuttering: a bounded event's monitor does, a measure reads the
    /// zero-reward stutter steps through its window.
    pub(super) fn closes_at_terminal(&self) -> bool {
        matches!(self, Self::Formula(_))
    }
}

fn threshold(bound: &Bound) -> &Rational {
    match bound {
        Bound::AtLeast(value) | Bound::AtMost(value) => value,
    }
}

fn zero() -> Rational {
    Rational::from_integer(Integer::zero())
}

/// The phase of a measure before its horizon decides it.
#[derive(Clone, Copy)]
enum Activation {
    Inactive,
    Active,
    Complete,
}

/// The monitor state after reading one letter: rejecting or settled states
/// are decided.
fn read_letter(
    monitor: &mut Monitor<PropertyAutomaton>,
    from: StateId,
    truths: &[bool],
) -> Result<FormulaState, PlanFault> {
    let mut letter = Letter::default();
    for (index, holds) in truths.iter().enumerate() {
        if *holds {
            letter = letter.with(Atom::new(index)?);
        }
    }
    let next = monitor.step(from, letter)?;
    decided(monitor, next)
}

/// `state` as the monitor's decided verdict, when it has one. A rejecting
/// state is the empty subset. A subset with a settled member has nothing left
/// to check on that run, so no letter sequence can reject it.
pub(super) fn decided(
    monitor: &Monitor<PropertyAutomaton>,
    state: StateId,
) -> Result<FormulaState, PlanFault> {
    if monitor.is_rejecting(state)? {
        return Ok(FormulaState::Decided(MonitorVerdict::Reject));
    }
    let members = monitor
        .members(state)
        .ok_or(AutomatonError::UnknownState { state })?;
    for member in members {
        if monitor.automaton().is_settled(*member)? {
            return Ok(FormulaState::Decided(MonitorVerdict::Accept));
        }
    }
    Ok(FormulaState::Running(state))
}

/// The reward `name` of a step, as a magnitude in the reward's own unit.
fn magnitude(rewards: &[(Identifier, Value)], name: &Identifier) -> Result<Rational, PlanFault> {
    let (_, value) = rewards
        .iter()
        .find(|(reward, _)| reward == name)
        .ok_or(PlanFault::Broken("a-step-carries-every-reward"))?;
    match value {
        Value::Integer(integer) => Ok(Rational::from_integer(integer.clone())),
        Value::Quantity(quantity) => Ok(quantity.magnitude().clone()),
        _ => Err(PlanFault::Broken("a-reward-is-an-integer-or-a-quantity")),
    }
}

/// `amount` plus `step`; an amount above the threshold stays above it.
fn add(amount: &Amount, step: &Rational) -> Amount {
    match amount {
        Amount::Exact(sum) => Amount::Exact(sum.add(step)),
        other => other.clone(),
    }
}

/// An exact amount above the threshold is the one value above it.
fn normalized(amount: Amount, threshold: &Rational) -> Amount {
    match amount {
        Amount::Exact(value) if value > *threshold => Amount::AboveBound,
        other => other,
    }
}

/// The control and amounts of a measure at a position with `remaining`
/// positions after it. Reaching the horizon decides it: an activated measure
/// that has not ended is censored, one never activated is unactivated.
fn measure_state(
    steps: bool,
    remaining: u64,
    phase: Activation,
    amount: &Amount,
    threshold: &Rational,
) -> (Option<MonitorState>, Option<Accumulator>) {
    let activated = !matches!(phase, Activation::Inactive);
    // An activated measure's amount, zero when it starts, can already be
    // above a negative threshold.
    let amount = if activated {
        normalized(amount.clone(), threshold)
    } else {
        amount.clone()
    };
    let (phase, amount) = match (phase, remaining) {
        (Activation::Inactive, 0) => (MeasurePhase::Unactivated, amount),
        (Activation::Inactive, _) => (MeasurePhase::Inactive, amount),
        (Activation::Active, 0) => (MeasurePhase::Censored, Amount::Censored),
        (Activation::Active, _) => (MeasurePhase::Active, amount),
        (Activation::Complete, _) => (MeasurePhase::Complete, amount),
    };
    let held = if steps {
        Accumulator::Steps { activated, amount }
    } else {
        Accumulator::Reward { activated, amount }
    };
    (Some(MonitorState::Measure { remaining, phase }), Some(held))
}
