// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-197: exact backward induction for the bounded acyclic product.

use std::collections::VecDeque;

use num_bigint::BigInt;
use num_integer::Integer as _;
use num_traits::Zero as _;
use quire_exact::{Integer, Rational};

use super::{ProbEdge, ProbProduct, ProbStateId, ProductAction, ProductKind};

/// The scheduler objective for a bounded claim over every scheduler.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FhObjective {
    /// Choose the least expected value at every state.
    Min,
    /// Choose the greatest expected value at every state.
    Max,
}

/// An integer multiple of `2^-exponent` (FR-197 AR-2).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Dyadic {
    /// The signed integer multiple.
    pub mantissa: BigInt,
    /// The power-of-two denominator.
    pub exponent: u32,
}

impl Dyadic {
    fn rational(&self) -> Rational {
        Rational::from_integer(Integer::from_big(self.mantissa.clone()))
            .divided_by_power_of_two(u64::from(self.exponent))
    }
}

/// A finite-horizon value vector and its memoryless deterministic policy.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FhResult {
    /// Exact rational values, one per product state.
    Exact {
        /// Values in product discovery order.
        values: Vec<Rational>,
        /// The selected action-group index for each state, when it has a choice.
        policy: Vec<Option<usize>>,
    },
    /// Outward-rounded values after the exact rational budget was exceeded.
    Bounds {
        /// Lower bounds in product discovery order.
        lower: Vec<Dyadic>,
        /// Upper bounds in product discovery order.
        upper: Vec<Dyadic>,
        /// The precision used.
        precision_bits: u32,
        /// The selected action-group index for each state, when it has a choice.
        policy: Vec<Option<usize>>,
    },
    /// The initial value still straddles the threshold at maximum precision.
    PrecisionBudget {
        /// The product initial state whose interval straddles the threshold.
        initial: ProbStateId,
        /// The lower endpoint.
        lower: Dyadic,
        /// The upper endpoint.
        upper: Dyadic,
    },
}

/// Why the graph cannot be evaluated as a finite-horizon product.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum FhError {
    /// A product state or edge refers outside the state vector.
    #[error("the probabilistic product contains an invalid state reference")]
    InvalidState,
    /// An undecided state has no outgoing action.
    #[error("an undecided probabilistic product state has no action")]
    MissingAction,
    /// A product edge has a negative probability or a choice does not sum to 1.
    #[error("a probabilistic product choice has invalid probability mass")]
    InvalidProbability,
    /// The purported finite-horizon product contains a cycle.
    #[error("the probabilistic product is cyclic")]
    Cycle,
    /// Precision limits are inconsistent or cannot be represented.
    #[error("the precision limits are invalid")]
    InvalidPrecision,
}

/// Evaluate a finite-horizon product in exact rationals first, then in
/// outward-rounded dyadic intervals if the rational-size budget is exceeded.
///
/// `terminal` contains a value for each decided state and `None` for each
/// undecided state. `threshold` is used only to decide whether precision must
/// be doubled; exact values are returned regardless of which side they fall on.
/// Product edge order is the canonical tie-break order for the witness policy.
///
/// # Errors
/// Returns [`FhError`] for malformed or cyclic products, or inconsistent
/// precision limits.
pub fn backward_induction(
    product: &ProbProduct,
    terminal: &[Option<Rational>],
    objective: FhObjective,
    threshold: &Rational,
    max_rational_bits: u64,
    precision_bits: u32,
    max_precision_bits: u32,
) -> Result<FhResult, FhError> {
    if terminal.len() != product.states.len()
        || precision_bits == 0
        || max_precision_bits < precision_bits
    {
        return Err(FhError::InvalidPrecision);
    }
    let order = topological_order(product)?;
    let exact = exact_pass(product, terminal, objective, &order)?;
    if exact
        .values
        .iter()
        .all(|value| rational_fits(value, max_rational_bits))
    {
        return Ok(FhResult::Exact {
            values: exact.values,
            policy: exact.policy,
        });
    }

    let mut bits = precision_bits;
    loop {
        let intervals = interval_pass(product, terminal, objective, &order, bits, &exact.policy)?;
        let mut ambiguous = None;
        for (_, _, initial) in &product.initial {
            let index = initial.index();
            let lower = intervals.lower.get(index).ok_or(FhError::InvalidState)?;
            let upper = intervals.upper.get(index).ok_or(FhError::InvalidState)?;
            if lower.rational() <= *threshold && *threshold <= upper.rational() {
                ambiguous = Some((*initial, lower.clone(), upper.clone()));
                break;
            }
        }
        if let Some((initial, lower, upper)) = ambiguous {
            if bits == max_precision_bits {
                return Ok(FhResult::PrecisionBudget {
                    initial,
                    lower,
                    upper,
                });
            }
            bits = bits.saturating_mul(2).min(max_precision_bits);
            continue;
        }
        return Ok(FhResult::Bounds {
            lower: intervals.lower,
            upper: intervals.upper,
            precision_bits: bits,
            policy: intervals.policy,
        });
    }
}

struct ExactValues {
    values: Vec<Rational>,
    policy: Vec<Option<usize>>,
}

struct IntervalValues {
    lower: Vec<Dyadic>,
    upper: Vec<Dyadic>,
    policy: Vec<Option<usize>>,
}

fn exact_pass(
    product: &ProbProduct,
    terminal: &[Option<Rational>],
    objective: FhObjective,
    order: &[ProbStateId],
) -> Result<ExactValues, FhError> {
    let zero = rational_zero();
    let mut values = vec![zero; product.states.len()];
    let mut policy = vec![None; product.states.len()];
    for state in order.iter().rev().copied() {
        let index = state.index();
        if let Some(value) = &terminal[index] {
            values[index] = value.clone();
            continue;
        }
        let choices = action_groups(product, state)?;
        if choices.is_empty() {
            return Err(FhError::MissingAction);
        }
        let mut selected: Option<(usize, Rational)> = None;
        for (choice_index, edges) in choices.iter().enumerate() {
            let value = weighted_value(edges, &values)?;
            let replace = selected
                .as_ref()
                .is_none_or(|(_, current)| match objective {
                    FhObjective::Min => value < *current,
                    FhObjective::Max => value > *current,
                });
            if replace {
                selected = Some((choice_index, value));
            }
        }
        let (choice_index, value) = selected.ok_or(FhError::MissingAction)?;
        values[index] = value;
        policy[index] = Some(choice_index);
    }
    Ok(ExactValues { values, policy })
}

fn interval_pass(
    product: &ProbProduct,
    terminal: &[Option<Rational>],
    objective: FhObjective,
    order: &[ProbStateId],
    bits: u32,
    exact_policy: &[Option<usize>],
) -> Result<IntervalValues, FhError> {
    let zero = Dyadic {
        mantissa: BigInt::zero(),
        exponent: bits,
    };
    let mut lower = vec![zero.clone(); product.states.len()];
    let mut upper = vec![zero; product.states.len()];
    if exact_policy.len() != product.states.len() {
        return Err(FhError::InvalidState);
    }
    for state in order.iter().rev().copied() {
        let index = state.index();
        if let Some(value) = &terminal[index] {
            lower[index] = round(value, bits, false)?;
            upper[index] = round(value, bits, true)?;
            continue;
        }
        let choices = action_groups(product, state)?;
        if choices.is_empty() {
            return Err(FhError::MissingAction);
        }
        let mut selected: Option<(Dyadic, Dyadic)> = None;
        for (choice_index, edges) in choices.iter().enumerate() {
            let (choice_lower, choice_upper) = weighted_interval(edges, &lower, &upper, bits)?;
            let _ = choice_index;
            selected = Some(match selected {
                None => (choice_lower, choice_upper),
                Some((current_lower, current_upper)) => match objective {
                    FhObjective::Min => (
                        if choice_lower.rational() < current_lower.rational() {
                            choice_lower
                        } else {
                            current_lower
                        },
                        if choice_upper.rational() < current_upper.rational() {
                            choice_upper
                        } else {
                            current_upper
                        },
                    ),
                    FhObjective::Max => (
                        if choice_lower.rational() > current_lower.rational() {
                            choice_lower
                        } else {
                            current_lower
                        },
                        if choice_upper.rational() > current_upper.rational() {
                            choice_upper
                        } else {
                            current_upper
                        },
                    ),
                },
            });
        }
        let (value_lower, value_upper) = selected.ok_or(FhError::MissingAction)?;
        lower[index] = value_lower;
        upper[index] = value_upper;
    }
    Ok(IntervalValues {
        lower,
        upper,
        policy: exact_policy.to_vec(),
    })
}

fn topological_order(product: &ProbProduct) -> Result<Vec<ProbStateId>, FhError> {
    let state_count = product.states.len();
    let mut indegree = vec![0_usize; state_count];
    for edge in &product.edges {
        if edge.from.index() >= state_count || edge.to.index() >= state_count {
            return Err(FhError::InvalidState);
        }
        indegree[edge.to.index()] = indegree[edge.to.index()]
            .checked_add(1)
            .ok_or(FhError::InvalidState)?;
    }
    let mut ready = VecDeque::new();
    for (index, degree) in indegree.iter().enumerate() {
        if *degree == 0 {
            ready.push_back(ProbStateId(index));
        }
    }
    let mut order = Vec::with_capacity(state_count);
    while let Some(state) = ready.pop_front() {
        order.push(state);
        for edge in product.outgoing(state) {
            let degree = indegree
                .get_mut(edge.to.index())
                .ok_or(FhError::InvalidState)?;
            *degree = degree.checked_sub(1).ok_or(FhError::InvalidState)?;
            if *degree == 0 {
                ready.push_back(edge.to);
            }
        }
    }
    if order.len() != state_count {
        return Err(FhError::Cycle);
    }
    for (_, _, initial) in &product.initial {
        if initial.index() >= state_count {
            return Err(FhError::InvalidState);
        }
    }
    Ok(order)
}

fn action_groups(
    product: &ProbProduct,
    state: ProbStateId,
) -> Result<Vec<Vec<&ProbEdge>>, FhError> {
    let edges = product.outgoing(state);
    if edges.iter().any(|edge| edge.from != state) {
        return Err(FhError::InvalidState);
    }
    if product.kind == ProductKind::Dtmc {
        return if edges.is_empty() {
            Ok(Vec::new())
        } else {
            Ok(vec![edges.iter().collect()])
        };
    }
    let mut groups: Vec<(ProductAction, Vec<&ProbEdge>)> = Vec::new();
    for edge in edges {
        if edge.probability < rational_zero() {
            return Err(FhError::InvalidProbability);
        }
        match groups
            .iter_mut()
            .find(|(action, _)| action_equal(action, &edge.choice))
        {
            Some((_, group)) => group.push(edge),
            None => groups.push((edge.choice.clone(), vec![edge])),
        }
    }
    for (_, group) in &groups {
        validate_probability_mass(group)?;
    }
    Ok(groups.into_iter().map(|(_, edges)| edges).collect())
}

fn action_equal(left: &ProductAction, right: &ProductAction) -> bool {
    match (left, right) {
        (ProductAction::Identity(left), ProductAction::Identity(right)) => left == right,
        (ProductAction::PostState(left), ProductAction::PostState(right)) => left == right,
        (ProductAction::TerminalStutter, ProductAction::TerminalStutter) => true,
        _ => false,
    }
}

fn weighted_value(edges: &[&ProbEdge], values: &[Rational]) -> Result<Rational, FhError> {
    validate_probability_mass(edges)?;
    edges.iter().try_fold(rational_zero(), |sum, edge| {
        let value = values.get(edge.to.index()).ok_or(FhError::InvalidState)?;
        Ok(sum.add(&edge.probability.mul(value)))
    })
}

fn weighted_interval(
    edges: &[&ProbEdge],
    lower: &[Dyadic],
    upper: &[Dyadic],
    bits: u32,
) -> Result<(Dyadic, Dyadic), FhError> {
    validate_probability_mass(edges)?;
    let scale = Rational::from_integer(Integer::one().shifted_left(u64::from(bits)));
    let (lower_sum, upper_sum) = edges.iter().try_fold(
        (rational_zero(), rational_zero()),
        |(lower_sum, upper_sum), edge| {
            let edge_lower = lower.get(edge.to.index()).ok_or(FhError::InvalidState)?;
            let edge_upper = upper.get(edge.to.index()).ok_or(FhError::InvalidState)?;
            Ok::<_, FhError>((
                lower_sum.add(&edge.probability.mul(&edge_lower.rational())),
                upper_sum.add(&edge.probability.mul(&edge_upper.rational())),
            ))
        },
    )?;
    let lower_scaled = lower_sum.mul(&scale);
    let upper_scaled = upper_sum.mul(&scale);
    Ok((
        round_scaled(&lower_scaled, bits, false)?,
        round_scaled(&upper_scaled, bits, true)?,
    ))
}

fn validate_probability_mass(edges: &[&ProbEdge]) -> Result<(), FhError> {
    let mut total = rational_zero();
    for edge in edges {
        if edge.probability < rational_zero() {
            return Err(FhError::InvalidProbability);
        }
        total = total.add(&edge.probability);
    }
    if total != rational_one() {
        return Err(FhError::InvalidProbability);
    }
    Ok(())
}

fn round(value: &Rational, bits: u32, upward: bool) -> Result<Dyadic, FhError> {
    let scale = BigInt::from(1_u8) << bits;
    let numerator = value.numerator().as_big() * &scale;
    let denominator = value.denominator().as_big();
    let mantissa = if upward {
        numerator.div_ceil(denominator)
    } else {
        numerator.div_floor(denominator)
    };
    Ok(Dyadic {
        mantissa,
        exponent: bits,
    })
}

fn round_scaled(value: &Rational, bits: u32, upward: bool) -> Result<Dyadic, FhError> {
    let numerator = value.numerator().as_big();
    let denominator = value.denominator().as_big();
    let mantissa = if upward {
        numerator.div_ceil(denominator)
    } else {
        numerator.div_floor(denominator)
    };
    Ok(Dyadic {
        mantissa,
        exponent: bits,
    })
}

fn rational_fits(value: &Rational, max_bits: u64) -> bool {
    value.numerator().as_big().bits() <= max_bits && value.denominator().as_big().bits() <= max_bits
}

fn rational_zero() -> Rational {
    Rational::from_integer(Integer::zero())
}

fn rational_one() -> Rational {
    Rational::from_integer(Integer::one())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;

    fn rational(numerator: i64, denominator: i64) -> Rational {
        Rational::new(Integer::from(numerator), Integer::from(denominator))
            .expect("nonzero denominator")
    }

    fn product() -> ProbProduct {
        let key = |model| super::super::ProbKey {
            binding: None,
            model: vec![model],
            monitor: None,
            accumulator: None,
            pending: None,
        };
        ProbProduct {
            states: vec![key(0), key(1), key(2)],
            kind: ProductKind::Dtmc,
            edges: vec![
                ProbEdge {
                    from: ProbStateId(0),
                    choice: ProductAction::TerminalStutter,
                    action: None,
                    draw: None,
                    to: ProbStateId(1),
                    probability: rational(1, 4),
                    rewards: Vec::new(),
                },
                ProbEdge {
                    from: ProbStateId(0),
                    choice: ProductAction::TerminalStutter,
                    action: None,
                    draw: None,
                    to: ProbStateId(2),
                    probability: rational(3, 4),
                    rewards: Vec::new(),
                },
            ],
            initial: vec![(0, None, ProbStateId(0))],
            decided: Vec::new(),
            labels: vec![Vec::new(); 3],
        }
    }

    /// Trace: TC-632, FR-197-AC-1
    #[trace("TC-632", "FR-197-AC-1")]
    #[test]
    fn computes_the_exact_weighted_value() {
        let result = backward_induction(
            &product(),
            &[None, Some(rational(1, 1)), Some(rational(0, 1))],
            FhObjective::Min,
            &rational(1, 4),
            128,
            8,
            64,
        )
        .expect("valid finite product");

        let FhResult::Exact { values, .. } = result else {
            panic!("small rational result should remain exact");
        };
        assert_eq!(values[0], rational(1, 4));
    }

    /// Trace: TC-632, FR-197-AC-4
    #[trace("TC-632", "FR-197-AC-4")]
    #[test]
    fn precision_budget_contains_an_exact_threshold() {
        let result = backward_induction(
            &product(),
            &[None, Some(rational(1, 1)), Some(rational(0, 1))],
            FhObjective::Min,
            &rational(1, 4),
            0,
            4,
            8,
        )
        .expect("valid finite product");

        let FhResult::PrecisionBudget { lower, upper, .. } = result else {
            panic!("exact threshold remains inside every outward interval");
        };
        assert!(lower.rational() <= rational(1, 4));
        assert!(rational(1, 4) <= upper.rational());
    }
}
