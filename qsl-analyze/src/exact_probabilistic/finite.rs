// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-197's finite-product Bellman pass.  This module is deliberately about
//! the graph operation: callers provide the value of each decided product
//! state, while the pass checks that every undecided state has an acyclic
//! continuation and computes its exact value.

use std::collections::VecDeque;

use num_bigint::BigInt;
use num_integer::Integer as BigInteger;
use num_traits::{One, Zero};
use qsl_replay::ExactProbLimits;
use quire_exact::{Integer, Rational};

use super::{
    Accumulator, Amount, FormulaState, MeasurePhase, MonitorState, MonitorVerdict, ProbProduct,
    ProbStateId, ProductAction, ProductKind,
};
use qsl_semantics::check::Bound;

/// Which scheduler extremum a finite MDP objective asks for.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Extremum {
    /// The least value over enabled actions.
    Minimum,
    /// The greatest value over enabled actions.
    Maximum,
}

/// An exact value for each state of the finite product.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FiniteValues {
    values: Vec<Rational>,
    /// The first edge of the canonical action selected at each MDP state;
    /// none at DTMC and decided states.
    policy_edges: Vec<Option<usize>>,
}

/// One outward-rounded dyadic interval, with endpoints in units of
/// `2^-precision_bits`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DyadicInterval {
    /// The lower endpoint's integer mantissa.
    pub lower: BigInt,
    /// The upper endpoint's integer mantissa.
    pub upper: BigInt,
    /// The shared endpoint denominator is `2^precision_bits`.
    pub precision_bits: u32,
}

/// An outward-rounded value for each state of a finite product.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FiniteBounds {
    values: Vec<DyadicInterval>,
}

/// An exact finite solve when every rational fits its budget, or an outward
/// interval solve after the exact pass exceeds `max_rational_bits`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FiniteSolution {
    /// Every value's numerator and denominator fit the exact-rational limit.
    Exact(FiniteValues),
    /// Exact arithmetic exceeded its limit; this sound enclosure is the
    /// first dyadic pass and may be refined by the caller.
    Bounds(FiniteBounds),
}

impl FiniteBounds {
    /// The dyadic interval at a product state.
    pub fn get(&self, state: ProbStateId) -> Option<&DyadicInterval> {
        self.values.get(state.index())
    }

    /// Intervals in product discovery order.
    pub fn as_slice(&self) -> &[DyadicInterval] {
        &self.values
    }
}

/// The terminal payoff to propagate over a finite probabilistic product.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FinitePayoff {
    /// A bounded formula's Boolean verdict.
    Formula,
    /// A bounded measure comparison's Boolean event, including the
    /// specified result for a path on which activation never occurs.
    MeasureComparison(Bound),
    /// The value of the finite-window fraction.
    Fraction,
    /// The quantile objective `[M <= c] - q`, zero on unactivated paths.
    QuantileAtMost {
        /// The quantile's probability level.
        level: Rational,
        /// The inclusive measure threshold.
        threshold: Rational,
    },
    /// The strict quantile objective `[M < c] - q`, zero on unactivated
    /// paths.
    QuantileAtLeast {
        /// The quantile's probability level.
        level: Rational,
        /// The strict measure threshold.
        threshold: Rational,
    },
}

impl FiniteValues {
    /// The value at a product state.
    pub fn get(&self, state: ProbStateId) -> Option<&Rational> {
        self.values.get(state.index())
    }

    /// Values in product discovery order.
    pub fn as_slice(&self) -> &[Rational] {
        &self.values
    }

    /// Index of the first edge of the selected action at `state`, when the
    /// product is an MDP and the state is undecided.
    pub fn policy_edge(&self, state: ProbStateId) -> Option<usize> {
        self.policy_edges.get(state.index()).copied().flatten()
    }
}

/// Compute the exact finite-horizon value at every product state (FR-197).
///
/// `terminal` has one entry per product state.  A `Some` value marks a
/// decided state and gives its objective value; every undecided state is
/// evaluated from its successors.  DTMC probabilities are summed.  At an
/// MDP state, probabilities are summed per semantic action and the requested
/// extremum is selected, with ties retaining the first canonical action.
///
/// # Errors
///
/// Returns an error if the terminal vector has the wrong length, an
/// undecided state has no successor, an MDP action's edge probabilities do
/// not sum to one, or the undecided subgraph contains a cycle.
pub fn backward_induction(
    product: &ProbProduct,
    terminal: &[Option<Rational>],
    extremum: Extremum,
) -> Result<FiniteValues, FiniteFault> {
    if terminal.len() != product.states.len() {
        return Err(FiniteFault::TerminalLength {
            expected: product.states.len(),
            actual: terminal.len(),
        });
    }

    let count = product.states.len();
    let mut indegree = vec![0_usize; count];
    for edge in &product.edges {
        if terminal[edge.from.index()].is_none() && terminal[edge.to.index()].is_none() {
            indegree[edge.to.index()] = indegree[edge.to.index()]
                .checked_add(1)
                .ok_or(FiniteFault::GraphSize)?;
        }
    }

    let mut ready = VecDeque::new();
    let mut undecided = 0_usize;
    for (index, value) in terminal.iter().enumerate() {
        if value.is_none() {
            undecided += 1;
            if indegree[index] == 0 {
                ready.push_back(index);
            }
        }
    }

    let mut order = Vec::with_capacity(undecided);
    while let Some(index) = ready.pop_front() {
        order.push(index);
        let state = ProbStateId(index);
        for edge in product.outgoing(state) {
            if terminal[edge.to.index()].is_none() {
                let degree = &mut indegree[edge.to.index()];
                *degree = degree.checked_sub(1).ok_or(FiniteFault::GraphShape)?;
                if *degree == 0 {
                    ready.push_back(edge.to.index());
                }
            }
        }
    }
    if order.len() != undecided {
        return Err(FiniteFault::Cycle);
    }

    let mut values = terminal
        .iter()
        .map(|value| value.clone().unwrap_or_else(zero))
        .collect::<Vec<_>>();
    let mut policy_edges = vec![None; count];
    for index in order.into_iter().rev() {
        let state = ProbStateId(index);
        let edges = product.outgoing(state);
        if edges.is_empty() {
            return Err(FiniteFault::NoSuccessor { state: index });
        }
        values[index] = match product.kind {
            ProductKind::Dtmc => expectation(
                edges
                    .iter()
                    .map(|edge| (edge.probability.clone(), values[edge.to.index()].clone())),
            )?,
            ProductKind::Mdp => {
                let (value, edge) = mdp_value(edges, &values, extremum, index)?;
                policy_edges[index] = Some(edge);
                value
            }
        };
    }
    Ok(FiniteValues {
        values,
        policy_edges,
    })
}

/// Compute a sound dyadic interval at every finite-product state (FR-197
/// AR-1 to AR-3), rounding every arithmetic operation outwards.
pub fn backward_induction_bounds(
    product: &ProbProduct,
    terminal: &[Option<Rational>],
    extremum: Extremum,
    precision_bits: u32,
) -> Result<FiniteBounds, FiniteFault> {
    if terminal.len() != product.states.len() {
        return Err(FiniteFault::TerminalLength {
            expected: product.states.len(),
            actual: terminal.len(),
        });
    }
    let count = product.states.len();
    let mut indegree = vec![0_usize; count];
    for edge in &product.edges {
        if terminal[edge.from.index()].is_none() && terminal[edge.to.index()].is_none() {
            indegree[edge.to.index()] = indegree[edge.to.index()]
                .checked_add(1)
                .ok_or(FiniteFault::GraphSize)?;
        }
    }
    let mut ready = VecDeque::new();
    let mut undecided = 0;
    for (index, value) in terminal.iter().enumerate() {
        if value.is_none() {
            undecided += 1;
            if indegree[index] == 0 {
                ready.push_back(index);
            }
        }
    }
    let mut order = Vec::with_capacity(undecided);
    while let Some(index) = ready.pop_front() {
        order.push(index);
        for edge in product.outgoing(ProbStateId(index)) {
            if terminal[edge.to.index()].is_none() {
                let degree = &mut indegree[edge.to.index()];
                *degree = degree.checked_sub(1).ok_or(FiniteFault::GraphShape)?;
                if *degree == 0 {
                    ready.push_back(edge.to.index());
                }
            }
        }
    }
    if order.len() != undecided {
        return Err(FiniteFault::Cycle);
    }

    let mut values = terminal
        .iter()
        .map(|value| {
            value
                .as_ref()
                .map(|value| outward(value, precision_bits))
                .unwrap_or_else(|| point(BigInt::zero(), precision_bits))
        })
        .collect::<Vec<_>>();
    for index in order.into_iter().rev() {
        let edges = product.outgoing(ProbStateId(index));
        if edges.is_empty() {
            return Err(FiniteFault::NoSuccessor { state: index });
        }
        values[index] = match product.kind {
            ProductKind::Dtmc => interval_expectation(
                edges
                    .iter()
                    .map(|edge| (&edge.probability, &values[edge.to.index()])),
                precision_bits,
            )?,
            ProductKind::Mdp => interval_mdp(edges, &values, extremum, index, precision_bits)?,
        };
    }
    Ok(FiniteBounds { values })
}

/// Run exact finite backward induction first, switching to outward-rounded
/// dyadic arithmetic when any value exceeds FR-203's rational bit budget.
pub fn finite_solution(
    product: &ProbProduct,
    terminal: &[Option<Rational>],
    extremum: Extremum,
    limits: ExactProbLimits,
) -> Result<FiniteSolution, FiniteFault> {
    let exact = backward_induction(product, terminal, extremum)?;
    if exact
        .as_slice()
        .iter()
        .all(|value| within_rational_limit(value, limits.max_rational_bits))
    {
        Ok(FiniteSolution::Exact(exact))
    } else {
        backward_induction_bounds(product, terminal, extremum, limits.precision_bits)
            .map(FiniteSolution::Bounds)
    }
}

fn within_rational_limit(value: &Rational, max_bits: u64) -> bool {
    value.numerator().as_big().bits() <= max_bits && value.denominator().as_big().bits() <= max_bits
}

fn interval_mdp(
    edges: &[super::ProbEdge],
    values: &[DyadicInterval],
    extremum: Extremum,
    state: usize,
    precision_bits: u32,
) -> Result<DyadicInterval, FiniteFault> {
    let mut cursor = 0;
    let mut selected: Option<DyadicInterval> = None;
    while cursor < edges.len() {
        let first = &edges[cursor];
        let mut end = cursor + 1;
        while end < edges.len() && same_action(&first.choice, &edges[end].choice) {
            end += 1;
        }
        let action = interval_expectation(
            edges[cursor..end]
                .iter()
                .map(|edge| (&edge.probability, &values[edge.to.index()])),
            precision_bits,
        )?;
        selected = Some(match selected {
            None => action,
            Some(current) => select_interval(current, action, extremum),
        });
        cursor = end;
    }
    selected.ok_or(FiniteFault::NoSuccessor { state })
}

fn select_interval(
    left: DyadicInterval,
    right: DyadicInterval,
    extremum: Extremum,
) -> DyadicInterval {
    let (lower, upper) = match extremum {
        Extremum::Minimum => (left.lower.min(right.lower), left.upper.min(right.upper)),
        Extremum::Maximum => (left.lower.max(right.lower), left.upper.max(right.upper)),
    };
    DyadicInterval {
        lower,
        upper,
        precision_bits: left.precision_bits,
    }
}

fn interval_expectation<'a>(
    terms: impl IntoIterator<Item = (&'a Rational, &'a DyadicInterval)>,
    precision_bits: u32,
) -> Result<DyadicInterval, FiniteFault> {
    let mut lower = BigInt::zero();
    let mut upper = BigInt::zero();
    let mut mass = zero();
    for (probability, value) in terms {
        if probability < &zero() {
            return Err(FiniteFault::NegativeProbability);
        }
        if value.precision_bits != precision_bits {
            return Err(FiniteFault::GraphShape);
        }
        let (edge_lower, _) = multiply_outward(probability, &value.lower);
        let (_, edge_upper) = multiply_outward(probability, &value.upper);
        lower += edge_lower;
        upper += edge_upper;
        mass = mass.add(probability);
    }
    if mass != one() {
        return Err(FiniteFault::ProbabilityMass);
    }
    Ok(DyadicInterval {
        lower,
        upper,
        precision_bits,
    })
}

fn multiply_outward(probability: &Rational, mantissa: &BigInt) -> (BigInt, BigInt) {
    let numerator = probability.numerator().as_big() * mantissa;
    let denominator = probability.denominator().as_big();
    (
        numerator.div_floor(denominator),
        numerator.div_ceil(denominator),
    )
}

fn outward(value: &Rational, precision_bits: u32) -> DyadicInterval {
    let scale = BigInt::one() << precision_bits;
    let numerator = value.numerator().as_big() * scale;
    let denominator = value.denominator().as_big();
    DyadicInterval {
        lower: numerator.div_floor(denominator),
        upper: numerator.div_ceil(denominator),
        precision_bits,
    }
}

fn point(mantissa: BigInt, precision_bits: u32) -> DyadicInterval {
    DyadicInterval {
        lower: mantissa.clone(),
        upper: mantissa,
        precision_bits,
    }
}

/// Read the objective's payoff at every decided state of `product`.
///
/// The result is suitable for [`backward_induction`]; undecided product
/// states are `None`.  The claim's checked form determines which terminal
/// accumulator shape is accepted, so a mismatch is reported as an internal
/// product fault instead of being interpreted as a default value.
pub fn terminal_values(
    product: &ProbProduct,
    payoff: &FinitePayoff,
) -> Result<Vec<Option<Rational>>, FiniteFault> {
    product
        .states
        .iter()
        .map(|key| match key.monitor {
            Some(MonitorState::Formula(FormulaState::Decided(verdict))) => {
                let value = match (payoff, verdict) {
                    (FinitePayoff::Formula, MonitorVerdict::Accept) => one(),
                    (FinitePayoff::Formula, MonitorVerdict::Reject) => zero(),
                    _ => return Err(FiniteFault::ObjectiveShape),
                };
                Ok(Some(value))
            }
            Some(MonitorState::Measure { phase, .. }) if phase_is_decided(phase) => {
                let accumulator = key
                    .accumulator
                    .as_ref()
                    .ok_or(FiniteFault::ObjectiveShape)?;
                terminal_measure(payoff, phase, accumulator).map(Some)
            }
            Some(MonitorState::Fraction { remaining: 0 }) => {
                let accumulator = key
                    .accumulator
                    .as_ref()
                    .ok_or(FiniteFault::ObjectiveShape)?;
                terminal_fraction(payoff, accumulator).map(Some)
            }
            _ => Ok(None),
        })
        .collect()
}

fn phase_is_decided(phase: MeasurePhase) -> bool {
    matches!(
        phase,
        MeasurePhase::Complete | MeasurePhase::Censored | MeasurePhase::Unactivated
    )
}

fn terminal_measure(
    payoff: &FinitePayoff,
    phase: MeasurePhase,
    accumulator: &Accumulator,
) -> Result<Rational, FiniteFault> {
    let (activated, amount) = match accumulator {
        Accumulator::Steps { activated, amount } | Accumulator::Reward { activated, amount } => {
            (*activated, amount)
        }
        _ => return Err(FiniteFault::ObjectiveShape),
    };
    match payoff {
        FinitePayoff::MeasureComparison(bound) => {
            let holds = match (phase, activated, bound, amount) {
                (MeasurePhase::Unactivated, false, Bound::AtLeast(_), _) => false,
                (MeasurePhase::Unactivated, false, Bound::AtMost(_), _) => true,
                (_, true, Bound::AtLeast(_), Amount::AboveBound | Amount::Censored) => true,
                (_, true, Bound::AtMost(_), Amount::AboveBound | Amount::Censored) => false,
                (_, true, Bound::AtLeast(threshold), Amount::Exact(value)) => value >= threshold,
                (_, true, Bound::AtMost(threshold), Amount::Exact(value)) => value <= threshold,
                _ => return Err(FiniteFault::ObjectiveShape),
            };
            Ok(if holds { one() } else { zero() })
        }
        FinitePayoff::QuantileAtMost { level, threshold } => {
            quantile_payoff(phase, activated, amount, level, threshold, false)
        }
        FinitePayoff::QuantileAtLeast { level, threshold } => {
            quantile_payoff(phase, activated, amount, level, threshold, true)
        }
        FinitePayoff::Formula | FinitePayoff::Fraction => Err(FiniteFault::ObjectiveShape),
    }
}

fn quantile_payoff(
    phase: MeasurePhase,
    activated: bool,
    amount: &Amount,
    level: &Rational,
    threshold: &Rational,
    strict: bool,
) -> Result<Rational, FiniteFault> {
    if phase == MeasurePhase::Unactivated && !activated {
        return Ok(zero());
    }
    if !activated {
        return Err(FiniteFault::ObjectiveShape);
    }
    let holds = match amount {
        Amount::Exact(value) if strict => value < threshold,
        Amount::Exact(value) => value <= threshold,
        Amount::AboveBound | Amount::Censored => false,
    };
    Ok((if holds { one() } else { zero() }).sub(level))
}

fn terminal_fraction(
    payoff: &FinitePayoff,
    accumulator: &Accumulator,
) -> Result<Rational, FiniteFault> {
    match (payoff, accumulator) {
        (
            FinitePayoff::Fraction,
            Accumulator::CountFraction {
                true_count,
                position_count,
            },
        ) if *position_count > 0 => quotient(
            &Rational::from_integer(Integer::from(*true_count)),
            &Rational::from_integer(Integer::from(*position_count)),
        ),
        (
            FinitePayoff::Fraction,
            Accumulator::WeightedFraction {
                true_sum,
                total_sum,
            },
        ) if !total_sum.is_zero() => quotient(true_sum, total_sum),
        (FinitePayoff::Fraction, Accumulator::WeightedFraction { .. }) => {
            Err(FiniteFault::ZeroWeight)
        }
        _ => Err(FiniteFault::ObjectiveShape),
    }
}

fn quotient(numerator: &Rational, denominator: &Rational) -> Result<Rational, FiniteFault> {
    numerator.div(denominator).ok_or(FiniteFault::ZeroWeight)
}

fn mdp_value(
    edges: &[super::ProbEdge],
    values: &[Rational],
    extremum: Extremum,
    state: usize,
) -> Result<(Rational, usize), FiniteFault> {
    let mut cursor = 0;
    let mut selected: Option<(Rational, usize)> = None;
    while cursor < edges.len() {
        let first = &edges[cursor];
        let mut end = cursor + 1;
        while end < edges.len() && same_action(&first.choice, &edges[end].choice) {
            end += 1;
        }
        let action_value = expectation(
            edges[cursor..end]
                .iter()
                .map(|edge| (edge.probability.clone(), values[edge.to.index()].clone())),
        )?;
        selected = Some(match selected {
            None => (action_value, cursor),
            Some((current, edge)) => {
                let better = match extremum {
                    Extremum::Minimum => action_value < current,
                    Extremum::Maximum => action_value > current,
                };
                if better {
                    (action_value, cursor)
                } else {
                    (current, edge)
                }
            }
        });
        cursor = end;
    }
    selected.ok_or(FiniteFault::NoSuccessor { state })
}

fn expectation(
    terms: impl IntoIterator<Item = (Rational, Rational)>,
) -> Result<Rational, FiniteFault> {
    let mut total = zero();
    let mut mass = zero();
    for (probability, value) in terms {
        if probability < zero() {
            return Err(FiniteFault::NegativeProbability);
        }
        mass = mass.add(&probability);
        total = total.add(&probability.mul(&value));
    }
    if mass != one() {
        return Err(FiniteFault::ProbabilityMass);
    }
    Ok(total)
}

fn same_action(left: &ProductAction, right: &ProductAction) -> bool {
    match (left, right) {
        (ProductAction::Identity(left), ProductAction::Identity(right)) => left == right,
        (ProductAction::PostState(left), ProductAction::PostState(right)) => left == right,
        (ProductAction::TerminalStutter, ProductAction::TerminalStutter) => true,
        _ => false,
    }
}

fn zero() -> Rational {
    Rational::from_integer(Integer::zero())
}

fn one() -> Rational {
    Rational::from_integer(Integer::one())
}

/// A malformed finite product or an unexpected unbounded continuation.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum FiniteFault {
    /// The terminal vector does not cover every product state.
    #[error("terminal values have length {actual}, expected {expected}")]
    TerminalLength { expected: usize, actual: usize },
    /// A product exceeds the addressable graph size.
    #[error("product graph size overflow")]
    GraphSize,
    /// Edge endpoints or indegrees are inconsistent.
    #[error("product graph structure is inconsistent")]
    GraphShape,
    /// The undecided subgraph contains a cycle.
    #[error("finite product contains an undecided cycle")]
    Cycle,
    /// An undecided state has no outgoing edge.
    #[error("undecided state {state} has no successor")]
    NoSuccessor { state: usize },
    /// A transition probability is negative.
    #[error("product has a negative probability")]
    NegativeProbability,
    /// A state or action's probabilities do not sum exactly to one.
    #[error("product probabilities do not sum to one")]
    ProbabilityMass,
    /// An objective does not match a decided state's monitor and accumulator.
    #[error("objective and finite-product terminal state have different shapes")]
    ObjectiveShape,
    /// A weighted fraction has zero total weight.
    #[error("weighted fraction has zero total weight")]
    ZeroWeight,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exact_probabilistic::{ProbEdge, ProbKey};

    fn ratio(numerator: i64, denominator: i64) -> Rational {
        Rational::new(Integer::from(numerator), Integer::from(denominator)).expect("ratio")
    }

    fn key(index: usize) -> ProbKey {
        ProbKey {
            binding: None,
            model: vec![u8::try_from(index).expect("small index")],
            monitor: None,
            accumulator: None,
            pending: None,
        }
    }

    fn edge(from: usize, to: usize, probability: Rational) -> ProbEdge {
        ProbEdge {
            from: ProbStateId(from),
            choice: ProductAction::TerminalStutter,
            action: None,
            draw: None,
            to: ProbStateId(to),
            probability,
            rewards: Vec::new(),
        }
    }

    #[test]
    fn finite_chain_values_follow_reverse_topological_order() {
        let product = ProbProduct {
            states: vec![key(0), key(1), key(2)],
            kind: ProductKind::Dtmc,
            edges: vec![edge(0, 1, ratio(1, 2)), edge(0, 2, ratio(1, 2))],
            initial: vec![],
            decided: vec![],
            labels: vec![vec![], vec![], vec![]],
            fairness: vec![],
        };
        let values = backward_induction(
            &product,
            &[None, Some(one()), Some(zero())],
            Extremum::Minimum,
        )
        .expect("acyclic finite product");
        assert_eq!(values.get(ProbStateId(0)), Some(&ratio(1, 2)));
    }

    #[test]
    fn dyadic_pass_rounds_each_successor_contribution_outwards() {
        let product = ProbProduct {
            states: vec![key(0), key(1), key(2)],
            kind: ProductKind::Dtmc,
            edges: vec![edge(0, 1, ratio(1, 3)), edge(0, 2, ratio(2, 3))],
            initial: vec![],
            decided: vec![],
            labels: vec![vec![], vec![], vec![]],
            fairness: vec![],
        };
        let values = backward_induction_bounds(
            &product,
            &[None, Some(one()), Some(zero())],
            Extremum::Minimum,
            4,
        )
        .expect("valid finite product");
        let root = values.get(ProbStateId(0)).expect("initial state");
        assert_eq!(root.precision_bits, 4);
        assert_eq!(root.lower, BigInt::from(5));
        assert_eq!(root.upper, BigInt::from(6));
    }

    #[test]
    fn finite_solution_switches_only_after_the_rational_budget_is_exceeded() {
        let product = ProbProduct {
            states: vec![key(0), key(1), key(2)],
            kind: ProductKind::Dtmc,
            edges: vec![edge(0, 1, ratio(1, 3)), edge(0, 2, ratio(2, 3))],
            initial: vec![],
            decided: vec![],
            labels: vec![vec![], vec![], vec![]],
            fairness: vec![],
        };
        let terminal = [None, Some(one()), Some(zero())];
        let exact = finite_solution(
            &product,
            &terminal,
            Extremum::Minimum,
            ExactProbLimits::default(),
        )
        .expect("default rational limit covers a small fraction");
        assert!(matches!(exact, FiniteSolution::Exact(_)));

        let limits = ExactProbLimits {
            max_rational_bits: 1,
            precision_bits: 4,
            ..ExactProbLimits::default()
        };
        let bounded = finite_solution(&product, &terminal, Extremum::Minimum, limits)
            .expect("small rational budget selects dyadic arithmetic");
        let FiniteSolution::Bounds(values) = bounded else {
            panic!("expected interval fallback")
        };
        let root = values.get(ProbStateId(0)).expect("initial state");
        assert_eq!(
            (root.lower.clone(), root.upper.clone()),
            (5.into(), 6.into())
        );
    }

    #[test]
    fn finite_pass_refuses_an_undecided_cycle() {
        let product = ProbProduct {
            states: vec![key(0)],
            kind: ProductKind::Dtmc,
            edges: vec![edge(0, 0, one())],
            initial: vec![],
            decided: vec![],
            labels: vec![vec![]],
            fairness: vec![],
        };
        assert_eq!(
            backward_induction(&product, &[None], Extremum::Minimum),
            Err(FiniteFault::Cycle)
        );
    }

    #[test]
    fn quantile_terminal_payoff_keeps_strict_and_non_strict_sides_distinct() {
        let threshold = ratio(3, 1);
        let level = ratio(1, 2);
        let exact_three = Amount::Exact(ratio(3, 1));
        let exact_one = Amount::Exact(ratio(1, 1));
        assert_eq!(
            quantile_payoff(
                MeasurePhase::Complete,
                true,
                &exact_three,
                &level,
                &threshold,
                false,
            ),
            Ok(ratio(1, 2))
        );
        assert_eq!(
            quantile_payoff(
                MeasurePhase::Complete,
                true,
                &exact_three,
                &level,
                &threshold,
                true,
            ),
            Ok(ratio(-1, 2))
        );
        assert_eq!(
            quantile_payoff(
                MeasurePhase::Complete,
                true,
                &exact_one,
                &level,
                &threshold,
                true,
            ),
            Ok(ratio(1, 2))
        );
        assert_eq!(
            quantile_payoff(
                MeasurePhase::Unactivated,
                false,
                &exact_one,
                &level,
                &threshold,
                false,
            ),
            Ok(zero())
        );
    }

    #[test]
    fn scheduler_extremum_is_selected_after_each_action_expectation() {
        let left = key(0);
        let right = key(1);
        let mut product = ProbProduct {
            states: vec![key(0), key(1), key(2)],
            kind: ProductKind::Mdp,
            edges: vec![
                ProbEdge {
                    from: ProbStateId(0),
                    choice: ProductAction::PostState(Box::new(left.clone())),
                    action: None,
                    draw: None,
                    to: ProbStateId(1),
                    probability: ratio(1, 2),
                    rewards: Vec::new(),
                },
                ProbEdge {
                    from: ProbStateId(0),
                    choice: ProductAction::PostState(Box::new(left)),
                    action: None,
                    draw: None,
                    to: ProbStateId(2),
                    probability: ratio(1, 2),
                    rewards: Vec::new(),
                },
                ProbEdge {
                    from: ProbStateId(0),
                    choice: ProductAction::PostState(Box::new(right)),
                    action: None,
                    draw: None,
                    to: ProbStateId(2),
                    probability: one(),
                    rewards: Vec::new(),
                },
            ],
            initial: vec![],
            decided: vec![],
            labels: vec![vec![], vec![], vec![]],
            fairness: vec![],
        };
        product
            .edges
            .sort_by(|left, right| match (&left.choice, &right.choice) {
                (ProductAction::PostState(left), ProductAction::PostState(right)) => {
                    left.cmp(right)
                }
                _ => std::cmp::Ordering::Equal,
            });

        let terminal = [None, Some(zero()), Some(one())];
        let minimum =
            backward_induction(&product, &terminal, Extremum::Minimum).expect("valid finite MDP");
        let maximum =
            backward_induction(&product, &terminal, Extremum::Maximum).expect("valid finite MDP");
        assert_eq!(minimum.get(ProbStateId(0)), Some(&ratio(1, 2)));
        assert_eq!(maximum.get(ProbStateId(0)), Some(&one()));
        assert_eq!(minimum.policy_edge(ProbStateId(0)), Some(0));
        assert_eq!(maximum.policy_edge(ProbStateId(0)), Some(2));
    }
}
