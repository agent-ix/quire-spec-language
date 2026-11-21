// SPDX-License-Identifier: AGPL-3.0-or-later
//! Shared bounded measure and fraction controls (FR-196, FR-1043).
//! Initialization reads position zero; each resolved model step advances once.
//! Terminal stutters use the same step operation with zero model rewards.

use crate::automaton::StateId;
/// Canonical shared measure and fraction key encoding.
pub mod key;
use filament_exact::{
    Identifier, Incomplete, Integer, Meter, Outcome, Rational, RationalArithmetic, Value,
};

/// What a decided bounded-event monitor concluded.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum MonitorVerdict {
    /// Every continuation satisfies the event.
    Accept,
    /// No continuation satisfies the event.
    Reject,
}

/// The state of a bounded event's monitor after the position read.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum FormulaState {
    /// Undecided: the monitor's state, local to the product's monitor.
    Running(StateId),
    /// Decided and absorbing.
    Decided(MonitorVerdict),
}

/// Where a bounded measure stands (FR-196 "Canonical probability product
/// key", measure control).
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum MeasurePhase {
    /// No position has satisfied `A` yet.
    Inactive,
    /// `A` held; `B` has not yet.
    Active,
    /// `B` ended an activated measure. Decided.
    Complete,
    /// The horizon ended an activated measure before `B`: the measure
    /// exceeds every finite threshold. Decided.
    Censored,
    /// The horizon ended with `A` never held. Decided.
    Unactivated,
}

/// The control a bounded event or measure adds to a model state.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum MonitorState {
    /// A bounded event's monitor.
    Formula(FormulaState),
    /// An `accumulate` or `steps` measure: the model positions still to be
    /// read, `0..=h`, and its phase.
    Measure {
        /// The positions after this one that the window still reads.
        remaining: u64,
        /// Where the measure stands.
        phase: MeasurePhase,
    },
    /// A fraction: the included positions whose contribution is still
    /// unread, initially `h + 1`. It is complete, and decided, at 0.
    Fraction {
        /// The positions still to contribute.
        remaining: u64,
    },
    /// A timed deadline event (FR-196 canonical key, FR-204): the event's
    /// kind and phase. `D` and the exact elapsed clock are in the model
    /// member of the key.
    Deadline {
        /// `eventually`, `always` or `until`.
        kind: DeadlineKind,
        /// Where the event stands.
        phase: DeadlinePhase,
    },
}

/// The shape of a timed deadline event (FR-204 "Admission").
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum DeadlineKind {
    /// `eventually[0, D] holds(P)`.
    Eventually,
    /// `always[0, D] holds(P)`.
    Always,
    /// `holds(A) until[0, D] holds(B)`.
    Until,
}

/// Where a timed deadline event stands.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum DeadlinePhase {
    /// Undecided.
    Pending,
    /// The event held at or before `D`. Decided and absorbing.
    Accept,
    /// A delay crossed `D` first, or `A` failed before `B`. Decided and
    /// absorbing.
    Reject,
}

impl MonitorState {
    /// Whether the control is decided: nothing it reads can change it, so
    /// the product does not expand the state.
    pub fn is_decided(self) -> bool {
        match self {
            Self::Formula(state) => matches!(state, FormulaState::Decided(_)),
            Self::Measure { phase, .. } => matches!(
                phase,
                MeasurePhase::Complete | MeasurePhase::Censored | MeasurePhase::Unactivated
            ),
            Self::Fraction { remaining } => remaining == 0,
            Self::Deadline { phase, .. } => !matches!(phase, DeadlinePhase::Pending),
        }
    }
}

/// An accumulated `steps` or `accumulate` amount, normalized so that equal
/// futures have equal keys (FR-196 "Canonical probability product key").
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Amount {
    /// The amount, at most the claim's threshold; equality to the threshold
    /// is kept exact.
    Exact(Rational),
    /// Greater than the claim's threshold: every such amount is one value.
    AboveBound,
    /// The horizon ended the measure before its `B`: greater than every
    /// finite threshold.
    Censored,
}

/// The sums a bounded measure carries.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Accumulator {
    /// `steps from A until B`.
    Steps {
        /// Whether `A` has held; false entails amount zero.
        activated: bool,
        /// The steps taken since `A`.
        amount: Amount,
    },
    /// `accumulate R from A until B`.
    Reward {
        /// Whether `A` has held; false entails amount zero.
        activated: bool,
        /// The reward summed since `A`, in the reward's own unit.
        amount: Amount,
    },
    /// An unweighted fraction.
    CountFraction {
        /// The included positions at which `P` held.
        true_count: u64,
        /// The included positions read.
        position_count: u64,
    },
    /// A fraction weighted by a reward. Both sums stay exact: `(1, 2)` and
    /// `(2, 4)` are different states.
    WeightedFraction {
        /// The weight of the positions at which `P` held.
        true_sum: Rational,
        /// The weight of the positions read.
        total_sum: Rational,
    },
}

/// A malformed measure control or reward supplied to its semantic owner.
#[derive(Debug, thiserror::Error)]
pub enum MeasureError {
    /// A scalar operation of the claim has no value, with the kernel cause.
    #[error("undefined measure value: {0:?}")]
    Undefined(Box<filament_exact::Undefined>),
    /// The checked control, predicates or reward does not match the plan.
    #[error("invalid measure control: {0}")]
    Broken(&'static str),
    /// The actual caller arithmetic or cancellation charge was denied.
    #[error("measure arithmetic stopped: {0:?}")]
    Accounting(Box<Incomplete>),
}

/// The finite control selected from an untimed checked claim.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MeasurePlan<'c> {
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
}

impl MeasurePlan<'_> {
    /// Read the initial position's predicate truths.
    ///
    /// # Errors
    /// Returns a typed error when the predicate count does not match.
    pub fn start(
        &self,
        truths: &[bool],
        meter: &mut Meter,
    ) -> Result<(Option<MonitorState>, Option<Accumulator>), MeasureError> {
        tick(meter)?;
        match self {
            Self::Measure {
                reward,
                threshold,
                within,
            } => {
                let [from, until] = truths else {
                    return Err(MeasureError::Broken("a-measure-reads-two-predicates"));
                };
                let phase = match (*from, *until) {
                    (false, _) => Activation::Inactive,
                    (true, false) => Activation::Active,
                    (true, true) => Activation::Complete,
                };
                measure_state(
                    reward.is_none(),
                    *within,
                    phase,
                    &Amount::Exact(zero()),
                    threshold,
                    meter,
                )
            }
            Self::Fraction { window, weight } => {
                let remaining = window
                    .checked_add(1)
                    .ok_or(MeasureError::Broken("fraction-window-overflow"))?;
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
        }
    }

    /// Initialize control without evaluating position-zero predicates.
    ///
    /// # Errors
    /// Returns a typed error for a control that cannot be represented.
    pub fn initial_control(
        &self,
        meter: &mut Meter,
    ) -> Result<(Option<MonitorState>, Option<Accumulator>), MeasureError> {
        tick(meter)?;
        match self {
            Self::Measure { reward, within, .. } => Ok((
                Some(MonitorState::Measure {
                    remaining: *within,
                    phase: MeasurePhase::Inactive,
                }),
                Some(if reward.is_none() {
                    Accumulator::Steps {
                        activated: false,
                        amount: Amount::Exact(zero()),
                    }
                } else {
                    Accumulator::Reward {
                        activated: false,
                        amount: Amount::Exact(zero()),
                    }
                }),
            )),
            Self::Fraction { window, weight } => Ok((
                Some(MonitorState::Fraction {
                    remaining: window
                        .checked_add(1)
                        .ok_or(MeasureError::Broken("fraction-window-overflow"))?,
                }),
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
            )),
        }
    }

    /// Whether the resolved step's destination contributes a predicate read.
    /// A fraction's last departing contribution completes its window before
    /// any predicate outside that window is evaluated.
    pub fn reads_post(&self, control: Option<MonitorState>) -> bool {
        !matches!(
            (self, control),
            (
                Self::Fraction { .. },
                Some(MonitorState::Fraction { remaining: 1 })
            )
        )
    }

    /// Advance once after a model step resolves and its predicates are defined.
    /// For a fraction the predicate belongs to the position the step leaves.
    ///
    /// # Errors
    /// Returns a typed error for decided/mismatched control or a missing reward.
    pub fn step(
        &self,
        monitor: Option<MonitorState>,
        accumulator: Option<&Accumulator>,
        source: &[bool],
        post: &[bool],
        rewards: &[(Identifier, Value)],
        meter: &mut Meter,
    ) -> Result<(Option<MonitorState>, Option<Accumulator>), MeasureError> {
        tick(meter)?;
        match (self, monitor, accumulator) {
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
                    return Err(MeasureError::Broken("a-measure-reads-two-predicates"));
                };
                let remaining = remaining
                    .checked_sub(1)
                    .ok_or(MeasureError::Broken("a-decided-state-is-not-stepped"))?;
                let amount = match held {
                    Accumulator::Steps { amount, .. } | Accumulator::Reward { amount, .. } => {
                        amount
                    }
                    _ => return Err(MeasureError::Broken("a-measure-holds-an-amount")),
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
                            add(amount, &step, meter)?,
                        )
                    }
                    _ => return Err(MeasureError::Broken("a-decided-state-is-not-stepped")),
                };
                measure_state(
                    reward.is_none(),
                    remaining,
                    phase,
                    &amount,
                    threshold,
                    meter,
                )
            }
            (
                Self::Fraction { weight, .. },
                Some(MonitorState::Fraction { remaining }),
                Some(held),
            ) => {
                let remaining = remaining
                    .checked_sub(1)
                    .ok_or(MeasureError::Broken("a-decided-state-is-not-stepped"))?;
                let [holds] = source else {
                    return Err(MeasureError::Broken("a-fraction-reads-one-predicate"));
                };
                let accumulator = match (held, weight) {
                    (
                        Accumulator::CountFraction {
                            true_count,
                            position_count,
                        },
                        None,
                    ) => Accumulator::CountFraction {
                        true_count: true_count
                            .checked_add(u64::from(*holds))
                            .ok_or(MeasureError::Broken("fraction-count-overflow"))?,
                        position_count: position_count
                            .checked_add(1)
                            .ok_or(MeasureError::Broken("fraction-count-overflow"))?,
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
                                arithmetic(RationalArithmetic::Add(true_sum, &weight), meter)?
                            } else {
                                true_sum.clone()
                            },
                            total_sum: arithmetic(
                                RationalArithmetic::Add(total_sum, &weight),
                                meter,
                            )?,
                        }
                    }
                    _ => return Err(MeasureError::Broken("a-fraction-holds-its-sums")),
                };
                if remaining == 0 {
                    if let Accumulator::WeightedFraction {
                        true_sum,
                        total_sum,
                    } = &accumulator
                    {
                        if total_sum.is_zero() {
                            arithmetic(RationalArithmetic::Divide(true_sum, total_sum), meter)?;
                        }
                    }
                }
                Ok((
                    Some(MonitorState::Fraction { remaining }),
                    Some(accumulator),
                ))
            }
            _ => Err(MeasureError::Broken("control-matches-plan")),
        }
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

/// The reward `name` of a step, as a magnitude in the reward's own unit.
fn magnitude(rewards: &[(Identifier, Value)], name: &Identifier) -> Result<Rational, MeasureError> {
    let (_, value) = rewards
        .iter()
        .find(|(reward, _)| reward == name)
        .ok_or(MeasureError::Broken("a-step-carries-every-reward"))?;
    match value {
        Value::Integer(integer) => Ok(Rational::from_integer(integer.clone())),
        Value::Quantity(quantity) => Ok(quantity.magnitude().clone()),
        _ => Err(MeasureError::Broken("a-reward-is-an-integer-or-a-quantity")),
    }
}

/// `amount` plus `step`; an amount above the threshold stays above it.
fn add(amount: &Amount, step: &Rational, meter: &mut Meter) -> Result<Amount, MeasureError> {
    Ok(match amount {
        Amount::Exact(sum) => Amount::Exact(arithmetic(RationalArithmetic::Add(sum, step), meter)?),
        other => other.clone(),
    })
}

/// An exact amount above the threshold is the one value above it.
fn normalized(
    amount: Amount,
    threshold: &Rational,
    meter: &mut Meter,
) -> Result<Amount, MeasureError> {
    match amount {
        Amount::Exact(value) => {
            let above = completed(filament_exact::order_numbers(
                filament_exact::OrderingOperator::Greater,
                filament_exact::OrderedOperands::Rationals(&value, threshold),
                meter,
            ))?;
            Ok(if above {
                Amount::AboveBound
            } else {
                Amount::Exact(value)
            })
        }
        other => Ok(other),
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
    meter: &mut Meter,
) -> Result<(Option<MonitorState>, Option<Accumulator>), MeasureError> {
    let activated = !matches!(phase, Activation::Inactive);
    // An activated measure's amount, zero when it starts, can already be
    // above a negative threshold.
    let amount = if activated {
        normalized(amount.clone(), threshold, meter)?
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
    Ok((Some(MonitorState::Measure { remaining, phase }), Some(held)))
}

fn tick(meter: &mut Meter) -> Result<(), MeasureError> {
    meter
        .charge(
            filament_exact::Charge::new(filament_exact::ChargePoint::GraphEdge)
                .work(Integer::one()),
        )
        .map_err(|error| MeasureError::Accounting(Box::new(error)))
}

fn completed<T>(outcome: Outcome<T>) -> Result<T, MeasureError> {
    match outcome {
        Outcome::Completed(value) => Ok(value),
        Outcome::Incomplete(error) => Err(MeasureError::Accounting(Box::new(error))),
        Outcome::Undefined(cause) => Err(MeasureError::Undefined(Box::new(cause))),
        Outcome::Refused(_) => Err(MeasureError::Broken("measure-arithmetic-defined")),
    }
}

fn arithmetic(
    operation: RationalArithmetic<'_>,
    meter: &mut Meter,
) -> Result<Rational, MeasureError> {
    completed(filament_exact::evaluate_rational_arithmetic(
        operation, None, meter,
    ))
}

/// The comparison encoded by a checked bounded measure claim.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MeasureComparison {
    /// Amount at most the finite threshold.
    Le,
    /// Amount strictly below the finite threshold.
    Lt,
    /// Amount at least the finite threshold.
    Ge,
}

/// A terminal payoff over the shared measure product.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MeasurePayoff {
    /// A measure comparison event, with unactivated paths valued zero.
    Event(MeasureComparison, Rational),
    /// The numerator of an activation-conditioned quantile.
    Satisfied(MeasureComparison, Rational),
    /// The denominator of an activation-conditioned quantile.
    Activated,
    /// The every-scheduler quantile transform.
    Shifted(MeasureComparison, Rational, Rational),
    /// The completed fraction.
    Fraction,
}

impl MeasurePayoff {
    /// Compute a decided state's payoff from its complete control and sums.
    /// An undecided state returns `None`.
    ///
    /// # Errors
    /// Mismatched control, zero fraction denominator, or caller arithmetic denial.
    pub fn terminal(
        &self,
        control: MonitorState,
        accumulator: &Accumulator,
        meter: &mut Meter,
    ) -> Result<Option<Rational>, MeasureError> {
        tick(meter)?;
        if !control.is_decided() {
            return Ok(None);
        }
        let one = Rational::from_integer(Integer::one());
        let fraction = || match accumulator {
            Accumulator::CountFraction {
                true_count,
                position_count,
            } => Ok((
                Rational::from_integer(Integer::from(*true_count)),
                Rational::from_integer(Integer::from(*position_count)),
            )),
            Accumulator::WeightedFraction {
                true_sum,
                total_sum,
            } => Ok((true_sum.clone(), total_sum.clone())),
            _ => Err(MeasureError::Broken("fraction-payoff-holds-fraction")),
        };
        if matches!(self, Self::Fraction) {
            let (numerator, denominator) = fraction()?;
            return arithmetic(RationalArithmetic::Divide(&numerator, &denominator), meter)
                .map(Some);
        }
        let (activated, amount) = match accumulator {
            Accumulator::Steps { activated, amount }
            | Accumulator::Reward { activated, amount } => (*activated, Some(amount)),
            Accumulator::CountFraction { .. } | Accumulator::WeightedFraction { .. } => {
                (true, None)
            }
        };
        if matches!(self, Self::Activated) {
            return Ok(Some(if activated { one } else { zero() }));
        }
        if !activated {
            return Ok(Some(zero()));
        }
        let (comparison, threshold) = match self {
            Self::Event(comparison, threshold)
            | Self::Satisfied(comparison, threshold)
            | Self::Shifted(comparison, threshold, _) => (*comparison, threshold),
            Self::Activated | Self::Fraction => {
                return Err(MeasureError::Broken("comparison-payoff"))
            }
        };
        let operator = match comparison {
            MeasureComparison::Le => filament_exact::OrderingOperator::LessOrEqual,
            MeasureComparison::Lt => filament_exact::OrderingOperator::Less,
            MeasureComparison::Ge => filament_exact::OrderingOperator::GreaterOrEqual,
        };
        let satisfied = match amount {
            Some(Amount::Exact(value)) => completed(filament_exact::order_numbers(
                operator,
                filament_exact::OrderedOperands::Rationals(value, threshold),
                meter,
            ))?,
            Some(Amount::AboveBound | Amount::Censored) => comparison == MeasureComparison::Ge,
            None => {
                let (numerator, denominator) = fraction()?;
                let value =
                    arithmetic(RationalArithmetic::Divide(&numerator, &denominator), meter)?;
                completed(filament_exact::order_numbers(
                    operator,
                    filament_exact::OrderedOperands::Rationals(&value, threshold),
                    meter,
                ))?
            }
        };
        let indicator = if satisfied { one } else { zero() };
        Ok(Some(match self {
            Self::Shifted(_, _, level) => {
                arithmetic(RationalArithmetic::Subtract(&indicator, level), meter)?
            }
            _ => indicator,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;

    fn scalar_limits() -> filament_exact::ScalarLimits {
        filament_exact::ScalarLimits {
            integer_bits: 256,
            decimal_digits: 256,
            scale_expansion: 256,
            text_input_bytes: 4096,
            text_scalars: 4096,
            normalized_scalars: 4096,
            unit_edges: 256,
            value_occurrences: 4096,
            work_units: 4096,
            result_units: 4096,
        }
    }
    fn meter() -> Meter {
        Meter::new(scalar_limits())
    }

    fn steps(within: u64) -> MeasurePlan<'static> {
        MeasurePlan::Measure {
            reward: None,
            threshold: Rational::from_integer(Integer::from(2_u64)),
            within,
        }
    }

    #[trace("TC-631", "FR-196-AC-7", "FR-196-AC-11")]
    #[test]
    fn terminal_steps_count_and_keep_censoring_distinct_from_above_bound() {
        let plan = steps(4);
        let mut meter = meter();
        let (mut control, mut amount) = plan.start(&[true, false], &mut meter).unwrap();
        for remaining in (0..4).rev() {
            (control, amount) = plan
                .step(
                    control,
                    amount.as_ref(),
                    &[true, false],
                    &[true, false],
                    &[],
                    &mut meter,
                )
                .unwrap();
            assert_eq!(
                control,
                Some(MonitorState::Measure {
                    remaining,
                    phase: if remaining == 0 {
                        MeasurePhase::Censored
                    } else {
                        MeasurePhase::Active
                    }
                })
            );
            let expected = if remaining == 0 {
                Amount::Censored
            } else if remaining == 1 {
                Amount::AboveBound
            } else {
                Amount::Exact(Rational::from_integer(Integer::from(4 - remaining)))
            };
            assert_eq!(
                amount,
                Some(Accumulator::Steps {
                    activated: true,
                    amount: expected
                })
            );
        }
        assert!(plan
            .step(
                control,
                amount.as_ref(),
                &[true, false],
                &[true, false],
                &[],
                &mut meter
            )
            .is_err());
    }

    #[trace("TC-631", "FR-196-AC-7", "FR-196-AC-11")]
    #[test]
    fn unactivation_and_zero_completion_have_different_controls() {
        let plan = steps(0);
        let mut meter = meter();
        let absent = plan.start(&[false, true], &mut meter).unwrap();
        let complete = plan.start(&[true, true], &mut meter).unwrap();
        assert_eq!(
            absent.0,
            Some(MonitorState::Measure {
                remaining: 0,
                phase: MeasurePhase::Unactivated
            })
        );
        assert_eq!(
            complete.0,
            Some(MonitorState::Measure {
                remaining: 0,
                phase: MeasurePhase::Complete
            })
        );
        assert_eq!(
            absent.1,
            Some(Accumulator::Steps {
                activated: false,
                amount: Amount::Exact(zero())
            })
        );
        assert_eq!(
            complete.1,
            Some(Accumulator::Steps {
                activated: true,
                amount: Amount::Exact(zero())
            })
        );
    }

    #[trace("TC-631", "FR-196-AC-11")]
    #[test]
    fn fraction_counts_departing_positions_including_horizon() {
        let plan = MeasurePlan::Fraction {
            window: 2,
            weight: None,
        };
        let mut meter = meter();
        let (mut control, mut sums) = plan.start(&[false], &mut meter).unwrap();
        for holds in [false, true, true] {
            (control, sums) = plan
                .step(control, sums.as_ref(), &[holds], &[!holds], &[], &mut meter)
                .unwrap();
        }
        assert_eq!(control, Some(MonitorState::Fraction { remaining: 0 }));
        assert_eq!(
            sums,
            Some(Accumulator::CountFraction {
                true_count: 2,
                position_count: 3
            })
        );
    }

    #[trace("TC-3152", "FR-1043-AC-7")]
    #[test]
    fn shared_control_retains_the_denied_caller_charge() {
        let mut limits = scalar_limits();
        limits.work_units = 0;
        let error = steps(4)
            .initial_control(&mut Meter::new(limits))
            .unwrap_err();
        assert!(matches!(error, MeasureError::Accounting(_)), "{error:?}");
    }
}
