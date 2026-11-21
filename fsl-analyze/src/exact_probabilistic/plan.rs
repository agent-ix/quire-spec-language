// SPDX-License-Identifier: AGPL-3.0-or-later
//! What a claim adds to a model state, and how a step advances it: FR-126's
//! monitor for a bounded event, the accumulator of a bounded measure, and no
//! control at all for an unbounded form (FR-196 "Behavior").

use filament_exact::{Identifier, Rational, Value};
use fsl_eval::automaton::{
    translate_body, Atom, AutomatonError, Letter, Monitor, PropertyAutomaton, StateId,
};
use fsl_foundation::Span;
use fsl_semantics::check::{
    Bound, BoundedMeasure, CheckedPredicate, CheckedProbabilisticClaim, Event, ProbForm,
    ReachTarget, TemporalProfile, Weighting,
};

use crate::model_check::false_extension::false_extension_rejects;

use super::{
    Accumulator, FormulaState, MonitorState, MonitorVerdict, PredicateRole, UndefinedRead,
};

/// Why a plan could not advance a state.
#[derive(Debug)]
pub(crate) enum PlanFault {
    /// The monitor's automaton refused.
    Automaton(AutomatonError),
    /// FR-204 does not admit this timed bounded measure shape.
    TimedFormShape { span: Span },
    /// The shared measure owner rejected the supplied control.
    Measure(fsl_eval::probabilistic::MeasureError),
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

fn reward_weight(
    weighting: Option<&Weighting>,
    span: Span,
) -> Result<Option<&Identifier>, PlanFault> {
    match weighting {
        None => Ok(None),
        Some(Weighting::Reward(name)) => Ok(Some(name)),
        Some(Weighting::Delay) => Err(PlanFault::TimedFormShape { span }),
    }
}

/// How the claim's control advances.
pub(super) enum Plan<'c> {
    /// A bounded event: FR-126's deterministic monitor, reading one letter
    /// per position.
    Formula(Box<Monitor<PropertyAutomaton>>),
    /// Shared measure/fraction control owned by the evaluator.
    Bounded(fsl_eval::probabilistic::MeasurePlan<'c>),
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
        let role =
            |role, predicate: &'c CheckedPredicate| (UndefinedRead::Predicate(role), predicate);
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
            } => Self::of_measure(measure, threshold(bound), claim.span)?,
            ProbForm::Quantile {
                measure,
                threshold: bound,
                ..
            } => Self::of_measure(measure, threshold(bound), claim.span)?,
            ProbForm::MeanOfFraction { fraction, .. } => (
                Self::Bounded(fsl_eval::probabilistic::MeasurePlan::Fraction {
                    window: fraction.window,
                    weight: reward_weight(fraction.weight.as_ref(), claim.span)?,
                }),
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
            ProbForm::MeanTime { until, .. } => {
                (Self::Labels, vec![role(PredicateRole::Until, until)])
            }
        })
    }

    fn of_measure(
        measure: &'c BoundedMeasure,
        threshold: &Rational,
        span: Span,
    ) -> Result<(Self, Reads<'c>), PlanFault> {
        let role =
            |role, predicate: &'c CheckedPredicate| (UndefinedRead::Predicate(role), predicate);
        Ok(match measure {
            BoundedMeasure::Accumulate {
                reward,
                from,
                until,
                within,
                ..
            } => (
                Self::Bounded(fsl_eval::probabilistic::MeasurePlan::Measure {
                    reward: Some(reward),
                    threshold: threshold.clone(),
                    within: *within,
                }),
                vec![
                    role(PredicateRole::From, from),
                    role(PredicateRole::Until, until),
                ],
            ),
            BoundedMeasure::Steps {
                from,
                until,
                within,
                ..
            } => (
                Self::Bounded(fsl_eval::probabilistic::MeasurePlan::Measure {
                    reward: None,
                    threshold: threshold.clone(),
                    within: *within,
                }),
                vec![
                    role(PredicateRole::From, from),
                    role(PredicateRole::Until, until),
                ],
            ),
            BoundedMeasure::Elapsed { span, .. } => {
                return Err(PlanFault::TimedFormShape { span: *span });
            }
            BoundedMeasure::Fraction(fraction) => (
                Self::Bounded(fsl_eval::probabilistic::MeasurePlan::Fraction {
                    window: fraction.window,
                    weight: reward_weight(fraction.weight.as_ref(), span)?,
                }),
                vec![role(PredicateRole::Target, &fraction.predicate)],
            ),
        })
    }

    /// The control and amounts at an initial model state, whose predicates
    /// read `truths`.
    pub(super) fn start(
        &mut self,
        truths: &[bool],
        meter: &mut filament_exact::Meter,
    ) -> Result<(Option<MonitorState>, Option<Accumulator>), PlanFault> {
        match self {
            Self::Formula(monitor) => {
                let initial = monitor.initial()?;
                let state = read_letter(monitor, initial, truths)?;
                Ok((Some(MonitorState::Formula(state)), None))
            }
            Self::Bounded(plan) => plan.start(truths, meter).map_err(PlanFault::Measure),
            Self::Labels => Ok((None, None)),
        }
    }

    /// The claim-initialized control before position zero is read. An
    /// undefined position-zero predicate reports this state without applying
    /// a letter or activating a measure.
    pub(super) fn initial_control(
        &mut self,
        meter: &mut filament_exact::Meter,
    ) -> Result<(Option<MonitorState>, Option<Accumulator>), PlanFault> {
        match self {
            Self::Formula(monitor) => Ok((
                Some(MonitorState::Formula(FormulaState::Running(
                    monitor.initial()?,
                ))),
                None,
            )),
            Self::Bounded(plan) => plan.initial_control(meter).map_err(PlanFault::Measure),
            Self::Labels => Ok((None, None)),
        }
    }

    pub(super) fn reads_post(&self, control: Option<MonitorState>) -> bool {
        match self {
            Self::Bounded(plan) => plan.reads_post(control),
            Self::Formula(_) | Self::Labels => true,
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
        meter: &mut filament_exact::Meter,
    ) -> Result<(Option<MonitorState>, Option<Accumulator>), PlanFault> {
        match (self, monitor, accumulator) {
            (Self::Formula(monitor), Some(MonitorState::Formula(state)), None) => {
                let FormulaState::Running(running) = state else {
                    return Err(PlanFault::Broken("a-decided-state-is-not-stepped"));
                };
                let state = read_letter(monitor, running, post)?;
                Ok((Some(MonitorState::Formula(state)), None))
            }
            (Self::Bounded(plan), monitor, accumulator) => plan
                .step(monitor, accumulator, source, post, rewards, meter)
                .map_err(PlanFault::Measure),
            (Self::Labels, None, None) => Ok((None, None)),
            _ => Err(PlanFault::Broken("control-matches-plan")),
        }
    }

    /// The verdict of a bounded event at a terminal model state, under the
    /// profile's false extension (FR-126's closure). The owner closes a
    /// single bounded eventuality directly; other forms read the all-false
    /// letter until rejection or repetition.
    pub(super) fn close(&mut self, running: StateId) -> Result<MonitorVerdict, PlanFault> {
        let Self::Formula(monitor) = self else {
            return Err(PlanFault::Broken("only-an-event-closes"));
        };
        if let Some(closed) = monitor.bounded_eventually_terminal(running)? {
            return Ok(match closed {
                fsl_eval::automaton::BoundedState::Accept => MonitorVerdict::Accept,
                fsl_eval::automaton::BoundedState::Reject => MonitorVerdict::Reject,
                fsl_eval::automaton::BoundedState::Open { .. }
                | fsl_eval::automaton::BoundedState::Idle => {
                    return Err(PlanFault::Broken("terminal-has-no-open-debt"));
                }
            });
        }
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

/// The monitor state after reading one letter: rejecting or settled states
/// are decided.
pub(crate) fn read_letter(
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
