// SPDX-License-Identifier: AGPL-3.0-or-later
//! Exact algorithms for the unbounded probabilistic forms (FR-198, FR-199).

use super::{ProbEdge, ProbProduct, ProbStateId, ProductAction, ProductKind};
use num_bigint::BigInt;
use num_integer::Integer as BigInteger;
use qsl_replay::ExactProbLimits;
use quire_exact::{Integer, Rational, Value};

type PolicySolution = (Vec<Rational>, Vec<Option<usize>>, u64);
type RewardPolicySolution = (Vec<ExtRational>, Vec<Option<usize>>, u64);
type IntervalStep = (Vec<Rational>, Vec<Rational>, Vec<Option<usize>>);
type MecOptimization = (Rational, Vec<Option<usize>>, Vec<Rational>, u64);

#[derive(Clone, Copy)]
struct PolicyLimits {
    objective: UnboundedObjective,
    max_iterations: u64,
    max_rational_bits: u64,
}

/// A finite exact value or positive infinity (FR-198).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ExtRational {
    /// An exact rational value.
    Finite(Rational),
    /// An unbounded expected reward.
    PlusInfinity,
}

/// The minimum or maximum scheduler objective.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UnboundedObjective {
    /// Minimize over schedulers.
    Min,
    /// Maximize over schedulers.
    Max,
}

/// Exact reachability values and the selected action at each state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnboundedResult {
    /// One exact value per product state.
    pub values: Vec<ExtRational>,
    /// The action-group index selected at each state, when it has choices.
    pub policy: Vec<Option<usize>>,
    /// Exact policy improvements performed.
    pub policy_iterations: u64,
}

/// Reachability output from interval iteration and exact policy iteration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UnboundedOutcome {
    /// Exact policy values, or one exact workload solve.
    Exact(UnboundedResult),
    /// Sound outward-rounded bounds after interval iteration.
    Bounds {
        /// Lower endpoint for each product state.
        lower: Vec<super::Dyadic>,
        /// Upper endpoint for each product state.
        upper: Vec<super::Dyadic>,
        /// Dyadic precision used.
        precision_bits: u32,
        /// Bellman sweeps performed.
        sweeps: u64,
        /// Selected action-group index per state.
        policy: Vec<Option<usize>>,
    },
    /// The interval straddled the threshold at maximum precision.
    PrecisionBudget {
        /// Lower endpoint for the least-favorable initial state.
        lower: super::Dyadic,
        /// Upper endpoint for the least-favorable initial state.
        upper: super::Dyadic,
    },
    /// Exact policy iteration stopped at a configured limit.
    Stopped(qsl_replay::ExactProbLimit),
}

/// Exact long-run result, with one value and gain–bias pair per recurrent
/// component (FR-199, FR-201 candidate data).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LongRunResult {
    /// Recurrent components in canonical state order.
    pub components: Vec<ComponentValue>,
    /// Exact value for each product state, including its eventual component.
    pub values: Vec<ExtRational>,
    /// Selected action per state for MDP products; absent for workloads.
    pub policy: Option<Vec<Option<usize>>>,
    /// Gain–bias equations per recurrent component.
    pub gain_bias: Vec<GainBias>,
}

/// Exact long-run value for one bottom component or maximal end component.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComponentValue {
    /// Product states in the component, in discovery order.
    pub states: Vec<ProbStateId>,
    /// The component's long-run fraction.
    pub rho: Rational,
}

/// Gain and state bias for a long-run component.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GainBias {
    /// Mean reward or fraction of the component.
    pub gain: Rational,
    /// Bias values in the same order as the component's states.
    pub bias: Vec<(ProbStateId, Rational)>,
}

/// Why an unbounded product could not be solved.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum UnboundedError {
    /// A state or edge refers outside the product.
    #[error("the probabilistic product contains an invalid state reference")]
    InvalidState,
    /// A nonterminal product state has no available action.
    #[error("an undecided probabilistic product state has no action")]
    MissingAction,
    /// An action has invalid probability mass.
    #[error("a probabilistic product choice has invalid probability mass")]
    InvalidProbability,
    /// A linear system was singular after qualitative states were removed.
    #[error("the exact policy equations are singular")]
    Singular,
    /// The policy improvement budget was reached.
    #[error("maximum exact policy iterations reached")]
    MaxPolicyIterations,
    /// An exact rational exceeded the configured bit budget.
    #[error("maximum exact rational bit length reached")]
    MaxRationalBits,
    /// The reward is absent or not an exact numeric value on an edge.
    #[error("the requested reward is missing or nonnumeric")]
    MissingReward,
    /// A long-run component has no positive total weight.
    #[error("a long-run component has zero total weight")]
    ZeroWeightComponent,
}

/// Solve unbounded reachability for one target set by exact policy iteration.
///
/// The caller supplies a state-label vector and the extremum. Qualitative
/// target and target-avoiding states are fixed before the linear equations are
/// solved, so singular recurrent classes do not enter policy evaluation.
pub fn decide_reachability(
    product: &ProbProduct,
    target: &[bool],
    objective: UnboundedObjective,
    limits: &ExactProbLimits,
) -> Result<UnboundedResult, UnboundedError> {
    decide_reachability_with_blocked(product, target, None, objective, limits)
}

/// Decide `A until B`, fixing states where neither predicate holds to zero.
pub fn decide_until(
    product: &ProbProduct,
    from: &[bool],
    target: &[bool],
    objective: UnboundedObjective,
    limits: &ExactProbLimits,
) -> Result<UnboundedResult, UnboundedError> {
    if from.len() != target.len() {
        return Err(UnboundedError::InvalidState);
    }
    let blocked = from
        .iter()
        .zip(target)
        .map(|(from, target)| !from && !target)
        .collect::<Vec<_>>();
    decide_reachability_with_blocked(product, target, Some(&blocked), objective, limits)
}

/// Decide `always P` as the complement of eventual `not P`, exchanging the
/// minimum and maximum objective.
pub fn decide_always(
    product: &ProbProduct,
    predicate: &[bool],
    objective: UnboundedObjective,
    limits: &ExactProbLimits,
) -> Result<UnboundedResult, UnboundedError> {
    if predicate.len() != product.states.len() {
        return Err(UnboundedError::InvalidState);
    }
    let complement = predicate.iter().map(|value| !value).collect::<Vec<_>>();
    let complement_objective = match objective {
        UnboundedObjective::Min => UnboundedObjective::Max,
        UnboundedObjective::Max => UnboundedObjective::Min,
    };
    let mut result = decide_reachability(product, &complement, complement_objective, limits)?;
    for value in &mut result.values {
        *value = match value {
            ExtRational::Finite(value) => ExtRational::Finite(one().sub(value)),
            ExtRational::PlusInfinity => return Err(UnboundedError::InvalidState),
        };
    }
    Ok(result)
}

fn product_with_blocked(
    product: &ProbProduct,
    blocked: &[bool],
) -> Result<ProbProduct, UnboundedError> {
    if blocked.len() != product.states.len()
        || product.edges.iter().any(|edge| {
            edge.from.index() >= blocked.len() || edge.to.index() >= product.states.len()
        })
    {
        return Err(UnboundedError::InvalidState);
    }
    let mut product = product.clone();
    product.edges.retain(|edge| !blocked[edge.from.index()]);
    for (index, is_blocked) in blocked.iter().copied().enumerate() {
        if is_blocked {
            let state = ProbStateId(index);
            if !product.decided.iter().any(|(decided, _)| *decided == state) {
                product.decided.push((state, super::MonitorVerdict::Reject));
            }
        }
    }
    Ok(product)
}

fn decide_reachability_with_blocked(
    product: &ProbProduct,
    target: &[bool],
    blocked: Option<&[bool]>,
    objective: UnboundedObjective,
    limits: &ExactProbLimits,
) -> Result<UnboundedResult, UnboundedError> {
    if target.len() != product.states.len() {
        return Err(UnboundedError::InvalidState);
    }
    if blocked.is_some_and(|blocked| blocked.len() != target.len()) {
        return Err(UnboundedError::InvalidState);
    }
    let product_with_terminals = match blocked {
        Some(blocked) => Some(product_with_blocked(product, blocked)?),
        None => None,
    };
    let product = product_with_terminals.as_ref().unwrap_or(product);
    let graph = Graph::new(product)?;
    let zero = zero();
    let one = one();
    let fixed_zero = match (product.kind, objective) {
        (ProductKind::Dtmc, _) => graph.dtmc_prob0(target)?,
        (ProductKind::Mdp, UnboundedObjective::Max) => graph.max_prob0(target)?,
        (ProductKind::Mdp, UnboundedObjective::Min) => graph.min_prob0(target)?,
    };
    let min_prob_one = if product.kind == ProductKind::Mdp && objective == UnboundedObjective::Min {
        Some(
            graph
                .can_avoid_target(target)?
                .iter()
                .map(|can_avoid| !can_avoid)
                .collect::<Vec<_>>(),
        )
    } else {
        None
    };
    let mut fixed = vec![None; target.len()];
    for i in 0..target.len() {
        if target[i] {
            fixed[i] = Some(one.clone());
        } else if fixed_zero[i] || blocked.is_some_and(|blocked| blocked[i]) {
            fixed[i] = Some(zero.clone());
        } else if min_prob_one.as_ref().is_some_and(|set| set[i]) {
            fixed[i] = Some(one.clone());
        }
    }
    let initial_policy = initial_policy(&graph, &fixed, objective)?;
    if fixed.iter().all(Option::is_some) {
        return Ok(UnboundedResult {
            values: fixed
                .into_iter()
                .map(|value| {
                    ExtRational::Finite(
                        value.unwrap_or_else(|| Rational::from_integer(Integer::zero())),
                    )
                })
                .collect(),
            policy: initial_policy,
            policy_iterations: 0,
        });
    }
    let (values, policy, iterations) = policy_iteration(
        &graph,
        &fixed,
        objective,
        limits.max_policy_iterations,
        initial_policy,
        limits.max_rational_bits,
    )?;
    Ok(UnboundedResult {
        values: values.into_iter().map(ExtRational::Finite).collect(),
        policy,
        policy_iterations: iterations,
    })
}

/// Run outward-rounded Bellman iteration and use exact policy iteration when
/// the interval cannot decide `threshold` within the configured budgets.
pub fn decide_reachability_threshold(
    product: &ProbProduct,
    target: &[bool],
    objective: UnboundedObjective,
    threshold: &Rational,
    limits: &ExactProbLimits,
) -> Result<UnboundedOutcome, UnboundedError> {
    decide_reachability_threshold_with_blocked(product, target, None, objective, threshold, limits)
}

/// Threshold form of [`decide_until`].
pub fn decide_until_threshold(
    product: &ProbProduct,
    from: &[bool],
    target: &[bool],
    objective: UnboundedObjective,
    threshold: &Rational,
    limits: &ExactProbLimits,
) -> Result<UnboundedOutcome, UnboundedError> {
    if from.len() != target.len() {
        return Err(UnboundedError::InvalidState);
    }
    let blocked = from
        .iter()
        .zip(target)
        .map(|(from, target)| !from && !target)
        .collect::<Vec<_>>();
    decide_reachability_threshold_with_blocked(
        product,
        target,
        Some(&blocked),
        objective,
        threshold,
        limits,
    )
}

fn decide_reachability_threshold_with_blocked(
    product: &ProbProduct,
    target: &[bool],
    blocked: Option<&[bool]>,
    objective: UnboundedObjective,
    threshold: &Rational,
    limits: &ExactProbLimits,
) -> Result<UnboundedOutcome, UnboundedError> {
    if target.len() != product.states.len()
        || limits.precision_bits == 0
        || limits.max_precision_bits < limits.precision_bits
    {
        return Err(UnboundedError::InvalidState);
    }
    if blocked.is_some_and(|blocked| blocked.len() != target.len()) {
        return Err(UnboundedError::InvalidState);
    }
    let product_with_terminals = match blocked {
        Some(blocked) => Some(product_with_blocked(product, blocked)?),
        None => None,
    };
    let product = product_with_terminals.as_ref().unwrap_or(product);
    let graph = Graph::new(product)?;
    let fixed_zero = match (product.kind, objective) {
        (ProductKind::Dtmc, _) => graph.dtmc_prob0(target)?,
        (ProductKind::Mdp, UnboundedObjective::Max) => graph.max_prob0(target)?,
        (ProductKind::Mdp, UnboundedObjective::Min) => graph.min_prob0(target)?,
    };
    let min_prob_one = if product.kind == ProductKind::Mdp && objective == UnboundedObjective::Min {
        Some(
            graph
                .can_avoid_target(target)?
                .iter()
                .map(|can_avoid| !can_avoid)
                .collect::<Vec<_>>(),
        )
    } else {
        None
    };
    let mecs = if product.kind == ProductKind::Mdp && objective == UnboundedObjective::Max {
        maximal_end_components(&graph)?
            .into_iter()
            .filter(|component| component.iter().all(|state| !target[*state]))
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    let mut fixed = vec![None; target.len()];
    for index in 0..target.len() {
        if target[index] {
            fixed[index] = Some(one());
        } else if fixed_zero[index] || blocked.is_some_and(|blocked| blocked[index]) {
            fixed[index] = Some(zero());
        } else if min_prob_one.as_ref().is_some_and(|set| set[index]) {
            fixed[index] = Some(one());
        }
    }

    let mut precision = limits.precision_bits;
    let mut sweeps = 0_u64;
    loop {
        let mut lower = vec![zero(); target.len()];
        let mut upper = vec![one(); target.len()];
        for (index, value) in fixed.iter().enumerate() {
            if let Some(value) = value {
                lower[index] = value.clone();
                upper[index] = value.clone();
            }
        }
        let mut policy = vec![None; target.len()];
        for _ in 0..limits.max_iterations {
            let (next_lower, next_upper, next_policy) =
                interval_sweep(&graph, &fixed, &lower, &upper, objective, precision, &mecs)?;
            lower = next_lower;
            upper = next_upper;
            policy = next_policy;
            sweeps += 1;
            if initial_intervals_decide(product, &lower, &upper, threshold) {
                return Ok(UnboundedOutcome::Bounds {
                    lower: lower
                        .iter()
                        .map(|value| round_dyadic(value, precision, false))
                        .collect(),
                    upper: upper
                        .iter()
                        .map(|value| round_dyadic(value, precision, true))
                        .collect(),
                    precision_bits: precision,
                    sweeps,
                    policy,
                });
            }
        }
        match decide_reachability_with_blocked(product, target, blocked, objective, limits) {
            Ok(result) => {
                return Ok(UnboundedOutcome::Exact(result));
            }
            Err(UnboundedError::MaxPolicyIterations) => {
                return Ok(UnboundedOutcome::Stopped(
                    qsl_replay::ExactProbLimit::MaxPolicyIterations,
                ));
            }
            Err(UnboundedError::MaxRationalBits) => {
                if precision < limits.max_precision_bits {
                    precision = precision.saturating_mul(2).min(limits.max_precision_bits);
                    continue;
                }
                let index = product
                    .initial
                    .first()
                    .map(|(_, _, state)| state.index())
                    .unwrap_or(0);
                return Ok(UnboundedOutcome::PrecisionBudget {
                    lower: round_dyadic(lower.get(index).unwrap_or(&zero()), precision, false),
                    upper: round_dyadic(upper.get(index).unwrap_or(&one()), precision, true),
                });
            }
            Err(error) => return Err(error),
        }
    }
}

/// Decide an expected reward to the first target visit.
///
/// States from which the objective permits a positive probability of never
/// reaching the target receive positive infinity. Remaining states are
/// evaluated by exact policy iteration.
pub fn decide_expected_reward(
    product: &ProbProduct,
    target: &[bool],
    reward: &str,
    objective: UnboundedObjective,
    limits: &ExactProbLimits,
) -> Result<UnboundedResult, UnboundedError> {
    if target.len() != product.states.len() {
        return Err(UnboundedError::InvalidState);
    }
    let graph = Graph::new(product)?;
    let can_avoid = graph.can_avoid_target(target)?;
    let almost_sure_some_policy = graph.almost_sure_reach(target)?;
    let infinite = match objective {
        UnboundedObjective::Max => can_avoid,
        UnboundedObjective::Min => almost_sure_some_policy.iter().map(|value| !value).collect(),
    };
    let mut initial_policy = graph
        .choices
        .iter()
        .map(|choices| (!choices.is_empty()).then_some(0))
        .collect::<Vec<_>>();
    if objective == UnboundedObjective::Min {
        let proper_policy = graph.almost_sure_policy(target)?;
        for (state, action) in proper_policy.into_iter().enumerate() {
            if action.is_some() {
                initial_policy[state] = action;
            }
        }
    }
    let mut fixed = vec![None; target.len()];
    for i in 0..target.len() {
        if target[i] || infinite[i] {
            fixed[i] = Some(zero());
        }
    }
    if fixed.iter().all(Option::is_some) {
        return Ok(UnboundedResult {
            values: fixed
                .into_iter()
                .enumerate()
                .map(|(index, value)| {
                    if infinite[index] {
                        ExtRational::PlusInfinity
                    } else {
                        ExtRational::Finite(
                            value.unwrap_or_else(|| Rational::from_integer(Integer::zero())),
                        )
                    }
                })
                .collect(),
            policy: initial_policy,
            policy_iterations: 0,
        });
    }
    let (values, policy, iterations) = policy_iteration_reward(
        &graph,
        &fixed,
        target,
        &infinite,
        reward,
        PolicyLimits {
            objective,
            max_iterations: limits.max_policy_iterations,
            max_rational_bits: limits.max_rational_bits,
        },
        initial_policy,
    )?;
    Ok(UnboundedResult {
        values,
        policy,
        policy_iterations: iterations,
    })
}

/// Decide a workload's unweighted or reward-weighted long-run fraction
/// exactly from its bottom strongly connected components (FR-199).
pub fn decide_long_run_workload(
    product: &ProbProduct,
    predicate: &[bool],
    weight: Option<&str>,
    limits: &ExactProbLimits,
) -> Result<LongRunResult, UnboundedError> {
    if predicate.len() != product.states.len() || product.kind != ProductKind::Dtmc {
        return Err(UnboundedError::InvalidState);
    }
    let graph = Graph::new(product)?;
    let sccs = strongly_connected_components(&graph)?;
    let mut fixed = vec![None; predicate.len()];
    let mut components = Vec::new();
    let mut gain_bias = Vec::new();
    for mut states in sccs {
        states.sort_unstable();
        let member = states
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>();
        let bottom = states.iter().all(|state| {
            graph.choices[*state].iter().all(|choice| {
                choice
                    .edges
                    .iter()
                    .all(|edge| member.contains(&edge.to.index()))
            })
        });
        if !bottom {
            continue;
        }
        let stationary = stationary_distribution(&graph, &states)?;
        if stationary
            .iter()
            .any(|value| value.max_part_bits() > limits.max_rational_bits)
        {
            return Err(UnboundedError::MaxRationalBits);
        }
        let mut numerator = zero();
        let mut denominator = zero();
        let mut state_reward = Vec::with_capacity(states.len());
        let mut state_weight = Vec::with_capacity(states.len());
        for (local, state) in states.iter().copied().enumerate() {
            let choice = graph.choices[state]
                .first()
                .ok_or(UnboundedError::MissingAction)?;
            let mut expected_weight = zero();
            for edge in &choice.edges {
                let step_weight = match weight {
                    Some(name) => reward_value(edge, name)?,
                    None => one(),
                };
                expected_weight = expected_weight.add(&edge.probability.mul(&step_weight));
            }
            let local_reward = if predicate[state] {
                expected_weight.clone()
            } else {
                zero()
            };
            numerator = numerator.add(&stationary[local].mul(&local_reward));
            denominator = denominator.add(&stationary[local].mul(&expected_weight));
            state_reward.push(local_reward);
            state_weight.push(expected_weight);
        }
        if denominator.is_zero() {
            return Err(UnboundedError::ZeroWeightComponent);
        }
        let rho = numerator
            .div(&denominator)
            .ok_or(UnboundedError::ZeroWeightComponent)?;
        let policy = graph
            .choices
            .iter()
            .map(|choices| (!choices.is_empty()).then_some(0))
            .collect::<Vec<_>>();
        let bias = component_bias(
            &graph,
            &states,
            &state_reward,
            &state_weight,
            &rho,
            &policy,
            states[0],
        )?;
        if rho.max_part_bits() > limits.max_rational_bits
            || bias
                .iter()
                .any(|value| value.max_part_bits() > limits.max_rational_bits)
        {
            return Err(UnboundedError::MaxRationalBits);
        }
        for state in &states {
            fixed[*state] = Some(rho.clone());
        }
        components.push(ComponentValue {
            states: states.iter().copied().map(ProbStateId).collect(),
            rho: rho.clone(),
        });
        gain_bias.push(GainBias {
            gain: rho,
            bias: states
                .iter()
                .copied()
                .zip(bias)
                .map(|(state, value)| (ProbStateId(state), value))
                .collect(),
        });
    }
    if components.is_empty() {
        return Err(UnboundedError::InvalidState);
    }
    let policy = graph
        .choices
        .iter()
        .map(|choices| (!choices.is_empty()).then_some(0))
        .collect::<Vec<_>>();
    let values = evaluate_policy(&graph, &fixed, &policy)?;
    Ok(LongRunResult {
        components,
        values: values.into_iter().map(ExtRational::Finite).collect(),
        policy: None,
        gain_bias,
    })
}

/// Decide a long-run fraction over every scheduler by MEC optima and exact
/// Dinkelbach updates (FR-199).
pub fn decide_long_run_mdp(
    product: &ProbProduct,
    predicate: &[bool],
    weight: Option<&str>,
    objective: UnboundedObjective,
    limits: &ExactProbLimits,
) -> Result<LongRunResult, UnboundedError> {
    if predicate.len() != product.states.len() || product.kind != ProductKind::Mdp {
        return Err(UnboundedError::InvalidState);
    }
    let graph = Graph::new(product)?;
    let mecs = maximal_end_components(&graph)?;
    if mecs.is_empty() {
        return Err(UnboundedError::InvalidState);
    }
    let mut fixed = vec![None; predicate.len()];
    let mut selected_policy = vec![None; predicate.len()];
    let mut components = Vec::new();
    let mut gain_bias = Vec::new();
    let mut policy_iterations = 0_u64;
    for component in mecs {
        let (rho, component_policy, bias, rounds) = optimize_mec_fraction(
            &graph,
            &component,
            predicate,
            weight,
            objective,
            limits.max_policy_iterations,
            limits.max_rational_bits,
        )?;
        policy_iterations = policy_iterations.saturating_add(rounds);
        for state in &component {
            fixed[*state] = Some(rho.clone());
            selected_policy[*state] = component_policy[*state];
        }
        components.push(ComponentValue {
            states: component.iter().copied().map(ProbStateId).collect(),
            rho: rho.clone(),
        });
        gain_bias.push(GainBias {
            gain: rho,
            bias: component
                .iter()
                .copied()
                .map(|state| (ProbStateId(state), bias[state].clone()))
                .collect(),
        });
    }
    let mut initial = selected_policy.clone();
    for (state, choices) in graph.choices.iter().enumerate() {
        if initial[state].is_none() && !choices.is_empty() {
            initial[state] = Some(0);
        }
    }
    let (values, policy, _) = policy_iteration(
        &graph,
        &fixed,
        objective,
        limits.max_policy_iterations,
        initial,
        limits.max_rational_bits,
    )?;
    Ok(LongRunResult {
        components,
        values: values.into_iter().map(ExtRational::Finite).collect(),
        policy: Some(policy),
        gain_bias,
    })
}

#[derive(Clone)]
struct Choice<'a> {
    edges: Vec<&'a ProbEdge>,
}

struct Graph<'a> {
    product: &'a ProbProduct,
    choices: Vec<Vec<Choice<'a>>>,
}

impl<'a> Graph<'a> {
    fn new(product: &'a ProbProduct) -> Result<Self, UnboundedError> {
        let mut choices = Vec::with_capacity(product.states.len());
        for state in product.ids() {
            let edges = product.outgoing(state);
            let mut groups: Vec<Choice<'a>> = Vec::new();
            for edge in edges {
                if edge.from.index() >= product.states.len()
                    || edge.to.index() >= product.states.len()
                {
                    return Err(UnboundedError::InvalidState);
                }
                if edge.probability < zero() {
                    return Err(UnboundedError::InvalidProbability);
                }
                if edge.probability.is_zero() {
                    continue;
                }
                if groups.last().is_some_and(|group| {
                    group
                        .edges
                        .first()
                        .is_some_and(|first| same_action(&first.choice, &edge.choice))
                }) {
                    if let Some(group) = groups.last_mut() {
                        group.edges.push(edge);
                    }
                } else {
                    groups.push(Choice { edges: vec![edge] });
                }
            }
            for group in &groups {
                let mass = group
                    .edges
                    .iter()
                    .fold(zero(), |sum, edge| sum.add(&edge.probability));
                if mass != one() {
                    return Err(UnboundedError::InvalidProbability);
                }
            }
            if !product.decided.iter().any(|(id, _)| *id == state) && groups.is_empty() {
                return Err(UnboundedError::MissingAction);
            }
            choices.push(groups);
        }
        Ok(Self { product, choices })
    }

    fn dtmc_prob0(&self, target: &[bool]) -> Result<Vec<bool>, UnboundedError> {
        let mut reaches_target = target.to_vec();
        loop {
            let mut changed = false;
            for state in self.product.ids() {
                let i = state.index();
                if reaches_target[i] || self.product.decided.iter().any(|(id, _)| *id == state) {
                    continue;
                }
                if self.choices[i][0]
                    .edges
                    .iter()
                    .any(|edge| reaches_target[edge.to.index()])
                {
                    reaches_target[i] = true;
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
        Ok(reaches_target.iter().map(|reaches| !reaches).collect())
    }

    /// Greatest set in which every action stays away from the target.
    fn max_prob0(&self, target: &[bool]) -> Result<Vec<bool>, UnboundedError> {
        let mut set = target
            .iter()
            .map(|is_target| !is_target)
            .collect::<Vec<_>>();
        loop {
            let mut next = set.clone();
            for state in self.product.ids() {
                let i = state.index();
                if !set[i] {
                    continue;
                }
                next[i] = self.choices[i]
                    .iter()
                    .all(|choice| choice.edges.iter().all(|edge| set[edge.to.index()]));
            }
            if next == set {
                return Ok(set);
            }
            set = next;
        }
    }

    /// States from which some scheduler can avoid the target with probability 1.
    fn min_prob0(&self, target: &[bool]) -> Result<Vec<bool>, UnboundedError> {
        // A safe end component is found by iteratively deleting states with
        // no action whose positive-probability successors remain in the set.
        let mut safe = target
            .iter()
            .map(|is_target| !is_target)
            .collect::<Vec<_>>();
        loop {
            let mut next = safe.clone();
            for state in self.product.ids() {
                let i = state.index();
                if safe[i]
                    && !self.choices[i]
                        .iter()
                        .any(|choice| choice.edges.iter().all(|edge| safe[edge.to.index()]))
                {
                    next[i] = false;
                }
            }
            if next == safe {
                break;
            }
            safe = next;
        }
        let mut reachable = safe.clone();
        loop {
            let mut next = reachable.clone();
            for state in self.product.ids() {
                let i = state.index();
                if !reachable[i]
                    && self.choices[i]
                        .iter()
                        .any(|choice| choice.edges.iter().all(|edge| reachable[edge.to.index()]))
                {
                    next[i] = true;
                }
            }
            if next == reachable {
                return Ok(reachable);
            }
            reachable = next;
        }
    }

    /// States from which a scheduler can fail to reach the target with
    /// positive probability.
    fn can_avoid_target(&self, target: &[bool]) -> Result<Vec<bool>, UnboundedError> {
        let mut safe = target
            .iter()
            .map(|is_target| !is_target)
            .collect::<Vec<_>>();
        loop {
            let mut next = safe.clone();
            for state in self.product.ids() {
                let index = state.index();
                if safe[index]
                    && !self.choices[index]
                        .iter()
                        .any(|choice| choice.edges.iter().all(|edge| safe[edge.to.index()]))
                {
                    next[index] = false;
                }
            }
            if next == safe {
                break;
            }
            safe = next;
        }
        let mut reachable = safe.clone();
        loop {
            let mut next = reachable.clone();
            for state in self.product.ids() {
                let index = state.index();
                if !reachable[index]
                    && self.choices[index].iter().any(|choice| {
                        choice
                            .edges
                            .iter()
                            .all(|edge| target[edge.to.index()] || reachable[edge.to.index()])
                            && choice.edges.iter().any(|edge| reachable[edge.to.index()])
                    })
                {
                    next[index] = true;
                }
            }
            if next == reachable {
                return Ok(reachable);
            }
            reachable = next;
        }
    }

    /// States from which some deterministic memoryless policy reaches target
    /// almost surely, via the standard nested attractor computation.
    fn almost_sure_reach(&self, target: &[bool]) -> Result<Vec<bool>, UnboundedError> {
        let mut candidate = vec![true; target.len()];
        loop {
            let mut attractor = target.to_vec();
            loop {
                let mut next = attractor.clone();
                for state in self.product.ids() {
                    let index = state.index();
                    if !candidate[index] || attractor[index] {
                        continue;
                    }
                    let can_progress = self.choices[index].iter().any(|choice| {
                        choice.edges.iter().all(|edge| candidate[edge.to.index()])
                            && choice.edges.iter().any(|edge| attractor[edge.to.index()])
                    });
                    if can_progress {
                        next[index] = true;
                    }
                }
                if next == attractor {
                    break;
                }
                attractor = next;
            }
            if attractor == candidate {
                return Ok(candidate);
            }
            candidate = attractor;
        }
    }

    fn almost_sure_policy(&self, target: &[bool]) -> Result<Vec<Option<usize>>, UnboundedError> {
        let count = target.len();
        let mut candidate = vec![true; count];
        loop {
            let mut rank = vec![None; count];
            for (index, is_target) in target.iter().copied().enumerate() {
                if is_target {
                    rank[index] = Some(0_usize);
                }
            }
            let mut policy = vec![None; count];
            loop {
                let mut changed = false;
                for state in self.product.ids() {
                    let index = state.index();
                    if !candidate[index] || rank[index].is_some() {
                        continue;
                    }
                    let selected = self.choices[index].iter().enumerate().find_map(
                        |(choice_index, choice)| {
                            let stays_inside =
                                choice.edges.iter().all(|edge| candidate[edge.to.index()]);
                            let minimum_successor_rank = choice
                                .edges
                                .iter()
                                .filter_map(|edge| rank[edge.to.index()])
                                .min();
                            (stays_inside && minimum_successor_rank.is_some())
                                .then_some((choice_index, minimum_successor_rank))
                        },
                    );
                    if let Some((choice_index, minimum_successor_rank)) = selected {
                        let next_rank = minimum_successor_rank
                            .and_then(|value| value.checked_add(1))
                            .ok_or(UnboundedError::InvalidState)?;
                        rank[index] = Some(next_rank);
                        policy[index] = Some(choice_index);
                        changed = true;
                    }
                }
                if !changed {
                    break;
                }
            }
            let region = rank.iter().map(Option::is_some).collect::<Vec<_>>();
            if region == candidate {
                return Ok(policy);
            }
            candidate = region;
        }
    }
}

fn same_action(left: &ProductAction, right: &ProductAction) -> bool {
    match (left, right) {
        (ProductAction::Identity(a), ProductAction::Identity(b)) => a == b,
        (ProductAction::PostState(a), ProductAction::PostState(b)) => a == b,
        (ProductAction::TerminalStutter, ProductAction::TerminalStutter) => true,
        _ => false,
    }
}

fn policy_iteration(
    graph: &Graph<'_>,
    fixed: &[Option<Rational>],
    objective: UnboundedObjective,
    max_iterations: u64,
    mut policy: Vec<Option<usize>>,
    max_rational_bits: u64,
) -> Result<PolicySolution, UnboundedError> {
    let count = graph.product.states.len();
    if graph.product.kind == ProductKind::Mdp && max_iterations == 0 {
        return Err(UnboundedError::MaxPolicyIterations);
    }
    let mut iterations = 0;
    loop {
        let values = evaluate_policy(graph, fixed, &policy)?;
        if values
            .iter()
            .any(|value| value.max_part_bits() > max_rational_bits)
        {
            return Err(UnboundedError::MaxRationalBits);
        }
        let mut improved = false;
        for state in graph.product.ids() {
            let i = state.index();
            if fixed[i].is_some() {
                continue;
            }
            let choices = &graph.choices[i];
            let mut best = policy[i].ok_or(UnboundedError::MissingAction)?;
            let mut best_value = choice_value(&choices[best], &values)?;
            if best_value.max_part_bits() > max_rational_bits {
                return Err(UnboundedError::MaxRationalBits);
            }
            for (choice_index, choice) in choices.iter().enumerate() {
                let value = choice_value(choice, &values)?;
                if value.max_part_bits() > max_rational_bits {
                    return Err(UnboundedError::MaxRationalBits);
                }
                let better = match objective {
                    UnboundedObjective::Min => value < best_value,
                    UnboundedObjective::Max => value > best_value,
                };
                if better {
                    best = choice_index;
                    best_value = value;
                }
            }
            if policy[i] != Some(best) {
                policy[i] = Some(best);
                improved = true;
            }
        }
        if !improved {
            return Ok((values, policy, iterations));
        }
        if iterations >= max_iterations {
            return Err(UnboundedError::MaxPolicyIterations);
        }
        iterations += 1;
        if iterations
            > u64::try_from(count)
                .unwrap_or(u64::MAX)
                .saturating_mul(1_000_000)
        {
            return Err(UnboundedError::MaxPolicyIterations);
        }
    }
}

fn policy_iteration_reward(
    graph: &Graph<'_>,
    fixed: &[Option<Rational>],
    target: &[bool],
    infinite: &[bool],
    reward: &str,
    limits: PolicyLimits,
    mut policy: Vec<Option<usize>>,
) -> Result<RewardPolicySolution, UnboundedError> {
    let count = graph.product.states.len();
    if graph.product.kind == ProductKind::Mdp && limits.max_iterations == 0 {
        return Err(UnboundedError::MaxPolicyIterations);
    }
    let mut iterations = 0;
    loop {
        let values = evaluate_reward_policy(graph, fixed, target, infinite, reward, &policy)?;
        if values.iter().any(|value| match value {
            ExtRational::Finite(value) => value.max_part_bits() > limits.max_rational_bits,
            ExtRational::PlusInfinity => false,
        }) {
            return Err(UnboundedError::MaxRationalBits);
        }
        let mut improved = false;
        for state in graph.product.ids() {
            let i = state.index();
            if fixed[i].is_some() {
                continue;
            }
            let choices = &graph.choices[i];
            let mut best = policy[i].ok_or(UnboundedError::MissingAction)?;
            let mut best_value = reward_choice_value(&choices[best], &values, target, reward)?;
            if let ExtRational::Finite(value) = &best_value {
                if value.max_part_bits() > limits.max_rational_bits {
                    return Err(UnboundedError::MaxRationalBits);
                }
            }
            for (choice_index, choice) in choices.iter().enumerate() {
                let value = reward_choice_value(choice, &values, target, reward)?;
                if let ExtRational::Finite(value) = &value {
                    if value.max_part_bits() > limits.max_rational_bits {
                        return Err(UnboundedError::MaxRationalBits);
                    }
                }
                let ordering = compare_ext_rational(&value, &best_value);
                let better = match limits.objective {
                    UnboundedObjective::Min => ordering.is_lt(),
                    UnboundedObjective::Max => ordering.is_gt(),
                };
                if better {
                    best = choice_index;
                    best_value = value;
                }
            }
            if policy[i] != Some(best) {
                policy[i] = Some(best);
                improved = true;
            }
        }
        if !improved {
            return Ok((values, policy, iterations));
        }
        if iterations >= limits.max_iterations {
            return Err(UnboundedError::MaxPolicyIterations);
        }
        iterations += 1;
        if iterations
            > u64::try_from(count)
                .unwrap_or(u64::MAX)
                .saturating_mul(1_000_000)
        {
            return Err(UnboundedError::MaxPolicyIterations);
        }
    }
}

fn evaluate_reward_policy(
    graph: &Graph<'_>,
    fixed: &[Option<Rational>],
    target: &[bool],
    infinite: &[bool],
    reward: &str,
    policy: &[Option<usize>],
) -> Result<Vec<ExtRational>, UnboundedError> {
    let policy_infinite = policy_infinite_states(graph, target, infinite, policy)?;
    let transient = (0..fixed.len())
        .filter(|index| fixed[*index].is_none() && !policy_infinite[*index])
        .collect::<Vec<_>>();
    let mut positions = vec![None; fixed.len()];
    for (position, state) in transient.iter().copied().enumerate() {
        positions[state] = Some(position);
    }
    let mut matrix = vec![vec![zero(); transient.len() + 1]; transient.len()];
    for (row, state) in transient.iter().copied().enumerate() {
        matrix[row][row] = one();
        let choice_index = policy[state].ok_or(UnboundedError::MissingAction)?;
        let choice = graph.choices[state]
            .get(choice_index)
            .ok_or(UnboundedError::MissingAction)?;
        for edge in &choice.edges {
            matrix[row][transient.len()] = matrix[row][transient.len()]
                .add(&edge.probability.mul(&reward_value(edge, reward)?));
            if !target[edge.to.index()] && !policy_infinite[edge.to.index()] {
                if let Some(column) = positions[edge.to.index()] {
                    matrix[row][column] = matrix[row][column].sub(&edge.probability);
                }
            }
        }
    }
    let solved = solve_linear(matrix)?;
    let mut values = fixed
        .iter()
        .enumerate()
        .map(|(state, value)| {
            if policy_infinite[state] {
                ExtRational::PlusInfinity
            } else {
                ExtRational::Finite(value.clone().unwrap_or_else(zero))
            }
        })
        .collect::<Vec<_>>();
    for (position, state) in transient.into_iter().enumerate() {
        values[state] = ExtRational::Finite(solved[position].clone());
    }
    Ok(values)
}

fn policy_infinite_states(
    graph: &Graph<'_>,
    target: &[bool],
    globally_infinite: &[bool],
    policy: &[Option<usize>],
) -> Result<Vec<bool>, UnboundedError> {
    let count = graph.product.states.len();
    let members = (0..count)
        .filter(|state| !target[*state])
        .collect::<std::collections::BTreeSet<_>>();
    let mut adjacency = vec![Vec::new(); count];
    let mut reverse = vec![Vec::new(); count];
    for state in members.iter().copied() {
        let Some(choice_index) = policy[state] else {
            continue;
        };
        let choice = graph.choices[state]
            .get(choice_index)
            .ok_or(UnboundedError::MissingAction)?;
        for edge in &choice.edges {
            if !target[edge.to.index()] {
                adjacency[state].push(edge.to.index());
                reverse[edge.to.index()].push(state);
            }
        }
    }

    let mut infinite = globally_infinite.to_vec();
    for component in restricted_scc(&members, &adjacency, &reverse) {
        let component_members = component
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>();
        let is_non_target_bottom = component.iter().all(|state| {
            let Some(choice_index) = policy[*state] else {
                return false;
            };
            graph.choices[*state][choice_index]
                .edges
                .iter()
                .all(|edge| component_members.contains(&edge.to.index()))
        });
        if is_non_target_bottom {
            for state in component {
                infinite[state] = true;
            }
        }
    }

    loop {
        let mut changed = false;
        for state in members.iter().copied() {
            if infinite[state] {
                continue;
            }
            let Some(choice_index) = policy[state] else {
                continue;
            };
            if graph.choices[state][choice_index]
                .edges
                .iter()
                .any(|edge| infinite[edge.to.index()])
            {
                infinite[state] = true;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    Ok(infinite)
}

fn reward_choice_value(
    choice: &Choice<'_>,
    values: &[ExtRational],
    target: &[bool],
    reward: &str,
) -> Result<ExtRational, UnboundedError> {
    let mut total = zero();
    for edge in &choice.edges {
        let continuation = if target[edge.to.index()] {
            ExtRational::Finite(zero())
        } else {
            values[edge.to.index()].clone()
        };
        match continuation {
            ExtRational::Finite(continuation) => {
                total = total.add(
                    &edge
                        .probability
                        .mul(&reward_value(edge, reward)?.add(&continuation)),
                );
            }
            ExtRational::PlusInfinity => return Ok(ExtRational::PlusInfinity),
        }
    }
    Ok(ExtRational::Finite(total))
}

fn compare_ext_rational(left: &ExtRational, right: &ExtRational) -> std::cmp::Ordering {
    match (left, right) {
        (ExtRational::Finite(left), ExtRational::Finite(right)) => left.cmp(right),
        (ExtRational::Finite(_), ExtRational::PlusInfinity) => std::cmp::Ordering::Less,
        (ExtRational::PlusInfinity, ExtRational::Finite(_)) => std::cmp::Ordering::Greater,
        (ExtRational::PlusInfinity, ExtRational::PlusInfinity) => std::cmp::Ordering::Equal,
    }
}

fn evaluate_policy(
    graph: &Graph<'_>,
    fixed: &[Option<Rational>],
    policy: &[Option<usize>],
) -> Result<Vec<Rational>, UnboundedError> {
    let count = fixed.len();
    let transient = (0..count)
        .filter(|index| fixed[*index].is_none())
        .collect::<Vec<_>>();
    let mut positions = vec![None; count];
    for (position, state) in transient.iter().copied().enumerate() {
        positions[state] = Some(position);
    }
    let mut matrix = vec![vec![zero(); transient.len() + 1]; transient.len()];
    for (row, state) in transient.iter().copied().enumerate() {
        matrix[row][row] = one();
        let choice_index = policy[state].ok_or(UnboundedError::MissingAction)?;
        let choice = graph.choices[state]
            .get(choice_index)
            .ok_or(UnboundedError::MissingAction)?;
        for edge in &choice.edges {
            match fixed[edge.to.index()].as_ref() {
                Some(value) => {
                    matrix[row][transient.len()] =
                        matrix[row][transient.len()].add(&edge.probability.mul(value))
                }
                None => {
                    let column = positions[edge.to.index()].ok_or(UnboundedError::InvalidState)?;
                    matrix[row][column] = matrix[row][column].sub(&edge.probability);
                }
            }
        }
    }
    let solved = solve_linear(matrix)?;
    let mut values = fixed
        .iter()
        .map(|value| value.clone().unwrap_or_else(zero))
        .collect::<Vec<_>>();
    for (position, state) in transient.into_iter().enumerate() {
        values[state] = solved[position].clone();
    }
    Ok(values)
}

fn solve_linear(matrix: Vec<Vec<Rational>>) -> Result<Vec<Rational>, UnboundedError> {
    let size = matrix.len();
    if matrix.iter().any(|row| row.len() != size + 1) {
        return Err(UnboundedError::Singular);
    }
    let mut integer_matrix = Vec::with_capacity(size);
    for row in matrix {
        let common_denominator = row.iter().fold(BigInt::from(1_u8), |common, value| {
            let denominator = value.denominator().as_big();
            common.lcm(denominator)
        });
        let integer_row: Vec<BigInt> = row
            .iter()
            .map(|value| {
                value.numerator().as_big() * (&common_denominator / value.denominator().as_big())
            })
            .collect();
        integer_matrix.push(integer_row);
    }

    // Bareiss elimination keeps every intermediate an integer and limits
    // coefficient growth compared with rational Gaussian elimination.
    let mut previous_pivot = BigInt::from(1_u8);
    for column in 0..size.saturating_sub(1) {
        let pivot_row = (column..size)
            .find(|row| !integer_matrix[*row][column].eq(&BigInt::from(0_u8)))
            .ok_or(UnboundedError::Singular)?;
        integer_matrix.swap(column, pivot_row);
        let pivot = integer_matrix[column][column].clone();
        for row in column + 1..size {
            for entry in column + 1..=size {
                let numerator = &pivot * &integer_matrix[row][entry]
                    - &integer_matrix[row][column] * &integer_matrix[column][entry];
                let (quotient, remainder) = numerator.div_rem(&previous_pivot);
                if !remainder.eq(&BigInt::from(0_u8)) {
                    return Err(UnboundedError::Singular);
                }
                integer_matrix[row][entry] = quotient;
            }
            integer_matrix[row][column] = BigInt::from(0_u8);
        }
        previous_pivot = pivot;
    }
    if size > 0 && integer_matrix[size - 1][size - 1].eq(&BigInt::from(0_u8)) {
        return Err(UnboundedError::Singular);
    }
    let mut solution = vec![zero(); size];
    for row in (0..size).rev() {
        let mut value = rational_from_big_integer(integer_matrix[row][size].clone());
        for column in row + 1..size {
            value = value.sub(
                &rational_from_big_integer(integer_matrix[row][column].clone())
                    .mul(&solution[column]),
            );
        }
        let diagonal = rational_from_big_integer(integer_matrix[row][row].clone());
        solution[row] = value.div(&diagonal).ok_or(UnboundedError::Singular)?;
    }
    Ok(solution)
}

fn rational_from_big_integer(value: BigInt) -> Rational {
    Rational::from_integer(Integer::from_big(value))
}

fn choice_value(choice: &Choice<'_>, values: &[Rational]) -> Result<Rational, UnboundedError> {
    choice.edges.iter().try_fold(zero(), |sum, edge| {
        let value = values
            .get(edge.to.index())
            .ok_or(UnboundedError::InvalidState)?;
        Ok(sum.add(&edge.probability.mul(value)))
    })
}

fn interval_sweep(
    graph: &Graph<'_>,
    fixed: &[Option<Rational>],
    lower: &[Rational],
    upper: &[Rational],
    objective: UnboundedObjective,
    precision: u32,
    mecs: &[Vec<usize>],
) -> Result<IntervalStep, UnboundedError> {
    let mut next_lower = lower.to_vec();
    let mut next_upper = upper.to_vec();
    let mut policy = vec![None; lower.len()];
    for state in graph.product.ids() {
        let index = state.index();
        if fixed[index].is_some() || mecs.iter().any(|component| component.contains(&index)) {
            continue;
        }
        let choices = &graph.choices[index];
        if choices.is_empty() {
            return Err(UnboundedError::MissingAction);
        }
        let mut lower_values = Vec::with_capacity(choices.len());
        let mut upper_values = Vec::with_capacity(choices.len());
        for choice in choices {
            lower_values.push(choice_value(choice, lower)?);
            upper_values.push(choice_value(choice, upper)?);
        }
        let selected_lower = select_value(&lower_values, objective);
        let selected_upper = select_value(&upper_values, objective);
        next_lower[index] = round_rational(&selected_lower, precision, false);
        next_upper[index] = round_rational(&selected_upper, precision, true);
        let witness_values = match objective {
            UnboundedObjective::Max => &upper_values,
            UnboundedObjective::Min => &lower_values,
        };
        policy[index] = select_index(witness_values, objective);
    }
    if objective == UnboundedObjective::Max {
        for component in mecs {
            let member = component
                .iter()
                .copied()
                .collect::<std::collections::BTreeSet<_>>();
            let macro_value = component
                .iter()
                .filter_map(|state| upper.get(*state))
                .max()
                .cloned()
                .unwrap_or_else(one);
            let mut best: Option<(usize, usize, Rational)> = None;
            for state in component {
                let state_choices = &graph.choices[*state];
                let lower_action_values = state_choices
                    .iter()
                    .map(|choice| choice_value(choice, lower))
                    .collect::<Result<Vec<_>, _>>()?;
                next_lower[*state] = round_rational(
                    &select_value(&lower_action_values, objective),
                    precision,
                    false,
                );
                for (choice_index, choice) in graph.choices[*state].iter().enumerate() {
                    if choice
                        .edges
                        .iter()
                        .all(|edge| member.contains(&edge.to.index()))
                    {
                        continue;
                    }
                    let value = choice.edges.iter().try_fold(zero(), |sum, edge| {
                        let continuation = if member.contains(&edge.to.index()) {
                            macro_value.clone()
                        } else {
                            upper
                                .get(edge.to.index())
                                .cloned()
                                .ok_or(UnboundedError::InvalidState)?
                        };
                        Ok::<Rational, UnboundedError>(
                            sum.add(&edge.probability.mul(&continuation)),
                        )
                    })?;
                    let replace = best.as_ref().is_none_or(|(_, _, current)| value > *current);
                    if replace {
                        best = Some((*state, choice_index, value));
                    }
                }
            }
            if let Some((owner, owner_choice, best_value)) = best {
                let collapsed = round_rational(&best_value, precision, true);
                for state in component {
                    next_upper[*state] = collapsed.clone();
                }
                expand_one_mec_policy(graph, component, owner, owner_choice, &mut policy)?;
            }
        }
    }
    Ok((next_lower, next_upper, policy))
}

fn select_value(values: &[Rational], objective: UnboundedObjective) -> Rational {
    values
        .iter()
        .min_by(|left, right| match objective {
            UnboundedObjective::Min => left.cmp(right),
            UnboundedObjective::Max => right.cmp(left),
        })
        .cloned()
        .unwrap_or_else(zero)
}

fn select_index(values: &[Rational], objective: UnboundedObjective) -> Option<usize> {
    let mut selected = None;
    for (index, value) in values.iter().enumerate() {
        let replace = selected.is_none_or(|current| match objective {
            UnboundedObjective::Min => value < &values[current],
            UnboundedObjective::Max => value > &values[current],
        });
        if replace {
            selected = Some(index);
        }
    }
    selected
}

fn round_rational(value: &Rational, precision: u32, upward: bool) -> Rational {
    round_dyadic(value, precision, upward).rational()
}

fn round_dyadic(value: &Rational, precision: u32, upward: bool) -> super::Dyadic {
    let scale = BigInt::from(1_u8) << precision;
    let numerator = value.numerator().as_big() * &scale;
    let denominator = value.denominator().as_big();
    let quotient = if upward {
        numerator.div_ceil(denominator)
    } else {
        numerator.div_floor(denominator)
    };
    super::Dyadic {
        mantissa: quotient,
        exponent: precision,
    }
}

fn initial_intervals_decide(
    product: &ProbProduct,
    lower: &[Rational],
    upper: &[Rational],
    threshold: &Rational,
) -> bool {
    let initial = product
        .initial
        .iter()
        .map(|(_, _, state)| state.index())
        .collect::<Vec<_>>();
    let indices: Vec<usize> = if initial.is_empty() {
        (0..lower.len()).collect()
    } else {
        initial
    };
    indices.into_iter().all(|index| {
        lower
            .get(index)
            .zip(upper.get(index))
            .is_some_and(|(low, high)| threshold < low || threshold > high)
    })
}

fn reward_value(edge: &ProbEdge, reward: &str) -> Result<Rational, UnboundedError> {
    let value = edge
        .rewards
        .iter()
        .find(|(name, _)| name.as_str() == reward)
        .map(|(_, value)| value);
    let Some(value) = value else {
        return Ok(zero());
    };
    match value {
        Value::Integer(value) => Ok(Rational::from_integer(value.clone())),
        Value::Quantity(value) => Ok(value.magnitude().clone()),
        _ => Err(UnboundedError::MissingReward),
    }
}

fn strongly_connected_components(graph: &Graph<'_>) -> Result<Vec<Vec<usize>>, UnboundedError> {
    let count = graph.product.states.len();
    let mut adjacency = vec![Vec::new(); count];
    let mut reverse = vec![Vec::new(); count];
    for state in graph.product.ids() {
        let from = state.index();
        for choice in &graph.choices[from] {
            for edge in &choice.edges {
                adjacency[from].push(edge.to.index());
                reverse[edge.to.index()].push(from);
            }
        }
    }
    let mut visited = vec![false; count];
    let mut order = Vec::with_capacity(count);
    for root in 0..count {
        if visited[root] {
            continue;
        }
        visited[root] = true;
        let mut stack = vec![(root, 0_usize)];
        while let Some((state, next)) = stack.pop() {
            if next < adjacency[state].len() {
                stack.push((state, next + 1));
                let successor = adjacency[state][next];
                if !visited[successor] {
                    visited[successor] = true;
                    stack.push((successor, 0));
                }
            } else {
                order.push(state);
            }
        }
    }
    visited.fill(false);
    let mut components = Vec::new();
    for root in order.into_iter().rev() {
        if visited[root] {
            continue;
        }
        let mut component = Vec::new();
        let mut stack = vec![root];
        visited[root] = true;
        while let Some(state) = stack.pop() {
            component.push(state);
            for predecessor in &reverse[state] {
                if !visited[*predecessor] {
                    visited[*predecessor] = true;
                    stack.push(*predecessor);
                }
            }
        }
        components.push(component);
    }
    Ok(components)
}

fn maximal_end_components(graph: &Graph<'_>) -> Result<Vec<Vec<usize>>, UnboundedError> {
    let mut pending =
        vec![(0..graph.product.states.len()).collect::<std::collections::BTreeSet<_>>()];
    let mut result = Vec::new();
    while let Some(mut candidate) = pending.pop() {
        loop {
            let retained = candidate
                .iter()
                .copied()
                .filter(|state| {
                    graph.choices[*state].iter().any(|choice| {
                        choice
                            .edges
                            .iter()
                            .all(|edge| candidate.contains(&edge.to.index()))
                    })
                })
                .collect::<std::collections::BTreeSet<_>>();
            if retained == candidate {
                break;
            }
            candidate = retained;
            if candidate.is_empty() {
                break;
            }
        }
        if candidate.is_empty() {
            continue;
        }
        let mut adjacency = vec![Vec::new(); graph.product.states.len()];
        let mut reverse = vec![Vec::new(); graph.product.states.len()];
        for state in candidate.iter().copied() {
            for choice in &graph.choices[state] {
                if choice
                    .edges
                    .iter()
                    .all(|edge| candidate.contains(&edge.to.index()))
                {
                    for edge in &choice.edges {
                        let target = edge.to.index();
                        adjacency[state].push(target);
                        reverse[target].push(state);
                    }
                }
            }
        }
        let components = restricted_scc(&candidate, &adjacency, &reverse);
        if components.len() == 1 && components[0].len() == candidate.len() {
            result.push(
                components
                    .into_iter()
                    .next()
                    .ok_or(UnboundedError::InvalidState)?,
            );
        } else {
            pending.extend(
                components
                    .into_iter()
                    .map(|component| component.into_iter().collect()),
            );
        }
    }
    result.sort_by_key(|component| component.iter().min().copied().unwrap_or(usize::MAX));
    Ok(result)
}

struct FractionPolicy {
    rho: Rational,
    numerator: Rational,
    denominator: Rational,
    choices: Vec<Option<usize>>,
    recurrent: Vec<usize>,
    rewards: Vec<Rational>,
    weights: Vec<Rational>,
}

fn optimize_mec_fraction(
    graph: &Graph<'_>,
    component: &[usize],
    predicate: &[bool],
    weight: Option<&str>,
    objective: UnboundedObjective,
    limit: u64,
    max_rational_bits: u64,
) -> Result<MecOptimization, UnboundedError> {
    if limit == 0 {
        return Err(UnboundedError::MaxPolicyIterations);
    }
    let members = component
        .iter()
        .copied()
        .collect::<std::collections::BTreeSet<_>>();
    let internal = component
        .iter()
        .map(|state| {
            let actions = graph.choices[*state]
                .iter()
                .enumerate()
                .filter_map(|(index, choice)| {
                    choice
                        .edges
                        .iter()
                        .all(|edge| members.contains(&edge.to.index()))
                        .then_some(index)
                })
                .collect::<Vec<_>>();
            (!actions.is_empty())
                .then_some(actions)
                .ok_or(UnboundedError::MissingAction)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut indices = vec![0_usize; component.len()];
    let mut candidates = Vec::new();
    let mut examined = 0_u64;
    loop {
        examined = examined.saturating_add(1);
        if examined > limit {
            return Err(UnboundedError::MaxPolicyIterations);
        }
        let mut policy = vec![None; graph.product.states.len()];
        for (local, state) in component.iter().copied().enumerate() {
            policy[state] = Some(internal[local][indices[local]]);
        }
        // FR-199 assigns one extremal rho to a communicating MEC. A policy
        // with one bottom class and transient states realizes that scalar
        // from every starting state: the MEC's internal choices can route
        // each state almost surely to the selected recurrent sub-MEC. A
        // policy with multiple bottom classes instead has state-dependent
        // values, so it is not itself a candidate scalar; the enumeration
        // also considers the policies that route all states to either
        // extremal class.
        let bottom_components = policy_bottom_components(graph, component, &policy)?;
        if bottom_components.len() == 1 {
            let recurrent = &bottom_components[0];
            let stationary = stationary_distribution_with_policy(graph, recurrent, &policy)?;
            if stationary
                .iter()
                .any(|value| value.max_part_bits() > max_rational_bits)
            {
                return Err(UnboundedError::MaxRationalBits);
            }
            let mut numerator = zero();
            let mut denominator = zero();
            let mut rewards = vec![zero(); component.len()];
            let mut weights = vec![zero(); component.len()];
            for (component_local, state) in component.iter().copied().enumerate() {
                let action = policy[state].ok_or(UnboundedError::MissingAction)?;
                let choice = graph.choices[state]
                    .get(action)
                    .ok_or(UnboundedError::MissingAction)?;
                let mut step_weight = zero();
                for edge in &choice.edges {
                    let edge_weight = match weight {
                        Some(name) => reward_value(edge, name)?,
                        None => one(),
                    };
                    step_weight = step_weight.add(&edge.probability.mul(&edge_weight));
                }
                let step_reward = if predicate[state] {
                    step_weight.clone()
                } else {
                    zero()
                };
                if let Some(local) = recurrent.iter().position(|candidate| *candidate == state) {
                    numerator = numerator.add(&stationary[local].mul(&step_reward));
                    denominator = denominator.add(&stationary[local].mul(&step_weight));
                }
                rewards[component_local] = step_reward;
                weights[component_local] = step_weight;
            }
            if !denominator.is_zero() {
                let rho = numerator
                    .div(&denominator)
                    .ok_or(UnboundedError::ZeroWeightComponent)?;
                if numerator.max_part_bits() > max_rational_bits
                    || denominator.max_part_bits() > max_rational_bits
                    || rho.max_part_bits() > max_rational_bits
                {
                    return Err(UnboundedError::MaxRationalBits);
                }
                candidates.push(FractionPolicy {
                    rho,
                    numerator,
                    denominator,
                    choices: policy,
                    recurrent: recurrent.clone(),
                    rewards,
                    weights,
                });
            }
        }
        let mut cursor = component.len();
        while cursor > 0 {
            cursor -= 1;
            indices[cursor] += 1;
            if indices[cursor] < internal[cursor].len() {
                break;
            }
            indices[cursor] = 0;
        }
        if cursor == 0 && indices[0] == 0 {
            break;
        }
    }
    if candidates.is_empty() {
        return Err(UnboundedError::ZeroWeightComponent);
    }
    let mut rho = candidates[0].rho.clone();
    let mut rounds = 0_u64;
    loop {
        let mut best: Option<(&FractionPolicy, Rational)> = None;
        for candidate in &candidates {
            let gain = candidate.numerator.sub(&rho.mul(&candidate.denominator));
            let replace = best.as_ref().is_none_or(|(_, current)| match objective {
                UnboundedObjective::Min => gain < *current,
                UnboundedObjective::Max => gain > *current,
            });
            if replace {
                best = Some((candidate, gain));
            }
        }
        let (candidate, gain) = best.ok_or(UnboundedError::ZeroWeightComponent)?;
        if gain.is_zero() {
            let bias = component_bias(
                graph,
                component,
                &candidate.rewards,
                &candidate.weights,
                &candidate.rho,
                &candidate.choices,
                candidate.recurrent[0],
            )?;
            return Ok((
                candidate.rho.clone(),
                candidate.choices.clone(),
                bias,
                rounds,
            ));
        }
        if rounds.saturating_add(examined) >= limit {
            return Err(UnboundedError::MaxPolicyIterations);
        }
        rho = candidate.rho.clone();
        rounds += 1;
    }
}

fn policy_bottom_components(
    graph: &Graph<'_>,
    component: &[usize],
    policy: &[Option<usize>],
) -> Result<Vec<Vec<usize>>, UnboundedError> {
    if component.is_empty() {
        return Ok(Vec::new());
    }
    let members = component
        .iter()
        .copied()
        .collect::<std::collections::BTreeSet<_>>();
    let mut adjacency = vec![Vec::new(); graph.product.states.len()];
    let mut transpose = vec![Vec::new(); graph.product.states.len()];
    for state in component {
        let choice_index = policy[*state].ok_or(UnboundedError::MissingAction)?;
        let choice = graph.choices[*state]
            .get(choice_index)
            .ok_or(UnboundedError::MissingAction)?;
        for edge in &choice.edges {
            adjacency[*state].push(edge.to.index());
            transpose[edge.to.index()].push(*state);
        }
    }
    let mut bottom = restricted_scc(&members, &adjacency, &transpose)
        .into_iter()
        .filter(|scc| {
            let scc_members = scc
                .iter()
                .copied()
                .collect::<std::collections::BTreeSet<_>>();
            scc.iter().all(|state| {
                adjacency[*state]
                    .iter()
                    .all(|successor| scc_members.contains(successor))
            })
        })
        .collect::<Vec<_>>();
    for scc in &mut bottom {
        scc.sort_unstable();
    }
    bottom.sort_by_key(|scc| scc[0]);
    Ok(bottom)
}

fn restricted_scc(
    members: &std::collections::BTreeSet<usize>,
    adjacency: &[Vec<usize>],
    reverse: &[Vec<usize>],
) -> Vec<Vec<usize>> {
    let mut visited = vec![false; adjacency.len()];
    let mut order = Vec::with_capacity(members.len());
    for root in members.iter().copied() {
        if visited[root] {
            continue;
        }
        visited[root] = true;
        let mut stack = vec![(root, 0_usize)];
        while let Some((state, next)) = stack.pop() {
            if next < adjacency[state].len() {
                stack.push((state, next + 1));
                let successor = adjacency[state][next];
                if members.contains(&successor) && !visited[successor] {
                    visited[successor] = true;
                    stack.push((successor, 0));
                }
            } else {
                order.push(state);
            }
        }
    }
    visited.fill(false);
    let mut result = Vec::new();
    for root in order.into_iter().rev() {
        if visited[root] {
            continue;
        }
        let mut component = Vec::new();
        let mut stack = vec![root];
        visited[root] = true;
        while let Some(state) = stack.pop() {
            component.push(state);
            for predecessor in &reverse[state] {
                if members.contains(predecessor) && !visited[*predecessor] {
                    visited[*predecessor] = true;
                    stack.push(*predecessor);
                }
            }
        }
        result.push(component);
    }
    result
}

fn initial_policy(
    graph: &Graph<'_>,
    fixed: &[Option<Rational>],
    objective: UnboundedObjective,
) -> Result<Vec<Option<usize>>, UnboundedError> {
    let mut policy = graph
        .choices
        .iter()
        .map(|choices| (!choices.is_empty()).then_some(0))
        .collect::<Vec<_>>();
    if graph.product.kind != ProductKind::Mdp || objective != UnboundedObjective::Max {
        return Ok(policy);
    }
    for component in maximal_end_components(graph)? {
        if component.iter().any(|state| fixed[*state].is_some()) {
            continue;
        }
        let member = component
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>();
        if let Some((owner, action)) = component.iter().find_map(|state| {
            graph.choices[*state]
                .iter()
                .position(|choice| {
                    choice
                        .edges
                        .iter()
                        .any(|edge| !member.contains(&edge.to.index()))
                })
                .map(|action| (*state, action))
        }) {
            expand_one_mec_policy(graph, &component, owner, action, &mut policy)?;
        }
    }
    Ok(policy)
}

fn expand_one_mec_policy(
    graph: &Graph<'_>,
    component: &[usize],
    owner: usize,
    owner_action: usize,
    policy: &mut [Option<usize>],
) -> Result<(), UnboundedError> {
    let members = component
        .iter()
        .copied()
        .collect::<std::collections::BTreeSet<_>>();
    policy[owner] = Some(owner_action);
    let mut distance = vec![None; graph.product.states.len()];
    distance[owner] = Some(0_usize);
    loop {
        let mut changed = false;
        for state in component {
            if distance[*state].is_some() {
                continue;
            }
            let nearest = graph.choices[*state]
                .iter()
                .filter(|choice| {
                    choice
                        .edges
                        .iter()
                        .all(|edge| members.contains(&edge.to.index()))
                })
                .flat_map(|choice| choice.edges.iter().map(|edge| edge.to.index()))
                .filter_map(|target| distance[target].map(|value| value + 1))
                .min();
            if let Some(nearest) = nearest {
                distance[*state] = Some(nearest);
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    for state in component {
        if *state == owner {
            continue;
        }
        let current_distance = distance[*state].ok_or(UnboundedError::InvalidState)?;
        let action = graph.choices[*state]
            .iter()
            .position(|choice| {
                choice
                    .edges
                    .iter()
                    .all(|edge| members.contains(&edge.to.index()))
                    && choice.edges.iter().any(|edge| {
                        distance[edge.to.index()].is_some_and(|next| next < current_distance)
                    })
            })
            .ok_or(UnboundedError::MissingAction)?;
        policy[*state] = Some(action);
    }
    Ok(())
}

fn stationary_distribution(
    graph: &Graph<'_>,
    states: &[usize],
) -> Result<Vec<Rational>, UnboundedError> {
    let policy = graph
        .choices
        .iter()
        .map(|choices| (!choices.is_empty()).then_some(0))
        .collect::<Vec<_>>();
    stationary_distribution_with_policy(graph, states, &policy)
}

fn stationary_distribution_with_policy(
    graph: &Graph<'_>,
    states: &[usize],
    policy: &[Option<usize>],
) -> Result<Vec<Rational>, UnboundedError> {
    let count = states.len();
    if count == 0 {
        return Err(UnboundedError::InvalidState);
    }
    let mut local = vec![None; graph.product.states.len()];
    for (index, state) in states.iter().copied().enumerate() {
        local[state] = Some(index);
    }
    let mut matrix = vec![vec![zero(); count + 1]; count];
    for (destination, row) in matrix.iter_mut().enumerate().take(count.saturating_sub(1)) {
        row[destination] = one();
        for (source_local, source) in states.iter().copied().enumerate() {
            let choice_index = policy[source].ok_or(UnboundedError::MissingAction)?;
            let choice = graph.choices[source]
                .get(choice_index)
                .ok_or(UnboundedError::MissingAction)?;
            for edge in &choice.edges {
                if local[edge.to.index()] == Some(destination) {
                    row[source_local] = row[source_local].sub(&edge.probability);
                }
            }
        }
    }
    matrix[count - 1][..count].fill(one());
    matrix[count - 1][count] = one();
    solve_linear(matrix)
}

fn component_bias(
    graph: &Graph<'_>,
    states: &[usize],
    state_reward: &[Rational],
    state_weight: &[Rational],
    gain: &Rational,
    policy: &[Option<usize>],
    reference: usize,
) -> Result<Vec<Rational>, UnboundedError> {
    let count = states.len();
    if count <= 1 {
        return Ok(vec![zero(); count]);
    }
    let mut local = vec![None; graph.product.states.len()];
    for (index, state) in states.iter().copied().enumerate() {
        local[state] = Some(index);
    }
    let reference_index = local[reference].ok_or(UnboundedError::InvalidState)?;
    let mut matrix = vec![vec![zero(); count + 1]; count];
    let mut row = 0;
    for (state_index, state) in states.iter().copied().enumerate() {
        if state == reference {
            continue;
        }
        let coefficients = &mut matrix[row];
        coefficients[state_index] = one();
        let action = policy[state].ok_or(UnboundedError::MissingAction)?;
        let choice = graph.choices[state]
            .get(action)
            .ok_or(UnboundedError::MissingAction)?;
        for edge in &choice.edges {
            if let Some(column) = local[edge.to.index()] {
                coefficients[column] = coefficients[column].sub(&edge.probability);
            }
        }
        coefficients[count] = state_reward[state_index].sub(&gain.mul(&state_weight[state_index]));
        row += 1;
    }
    matrix[count - 1][reference_index] = one();
    solve_linear(matrix)
}

fn zero() -> Rational {
    Rational::from_integer(Integer::zero())
}
fn one() -> Rational {
    Rational::from_integer(Integer::one())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exact_probabilistic::ProbKey;
    use ix_trace_rs::trace;

    fn ratio(numerator: i64, denominator: i64) -> Rational {
        Rational::new(Integer::from(numerator), Integer::from(denominator))
            .expect("nonzero denominator")
    }

    fn graph(kind: ProductKind, edges: Vec<ProbEdge>) -> ProbProduct {
        let state_count = edges
            .iter()
            .map(|edge| edge.from.index().max(edge.to.index()) + 1)
            .max()
            .unwrap_or(0);
        ProbProduct {
            states: (0..state_count)
                .map(|state| ProbKey {
                    binding: None,
                    model: vec![u8::try_from(state).expect("small state id")],
                    monitor: None,
                    accumulator: None,
                    pending: None,
                })
                .collect(),
            kind,
            edges,
            initial: Vec::new(),
            decided: Vec::new(),
            labels: Vec::new(),
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

    fn choice_edge(from: usize, to: usize, probability: Rational, action: u8) -> ProbEdge {
        let mut edge = edge(from, to, probability);
        edge.choice = ProductAction::PostState(Box::new(ProbKey {
            binding: None,
            model: vec![action],
            monitor: None,
            accumulator: None,
            pending: None,
        }));
        edge
    }

    /// Trace: FR-198-AC-3.
    #[trace("TC-633", "FR-198-AC-3")]
    #[test]
    fn unbounded_reachability_solves_a_self_looping_markov_chain_exactly() {
        let product = graph(
            ProductKind::Dtmc,
            vec![
                edge(0, 0, ratio(1, 3)),
                edge(0, 1, ratio(1, 3)),
                edge(0, 2, ratio(1, 3)),
                edge(1, 1, one()),
                edge(2, 2, one()),
            ],
        );
        let result = decide_reachability(
            &product,
            &[false, true, false],
            UnboundedObjective::Max,
            &ExactProbLimits::default(),
        )
        .expect("a finite DTMC reachability problem");
        assert_eq!(result.values[0], ExtRational::Finite(ratio(1, 2)));
        assert_eq!(result.values[1], ExtRational::Finite(one()));
        assert_eq!(result.values[2], ExtRational::Finite(zero()));
    }

    /// Trace: FR-198's unbounded temporal forms.
    #[trace("TC-633", "FR-198-AC-3")]
    #[test]
    fn until_stops_when_the_left_predicate_fails() {
        let product = graph(
            ProductKind::Dtmc,
            vec![edge(0, 1, one()), edge(1, 2, one()), edge(2, 2, one())],
        );
        let result = decide_until(
            &product,
            &[true, false, true],
            &[false, false, true],
            UnboundedObjective::Max,
            &ExactProbLimits::default(),
        )
        .expect("until treats a failed left predicate as a terminal failure");
        assert_eq!(result.values[0], ExtRational::Finite(zero()));
        assert_eq!(result.values[1], ExtRational::Finite(zero()));
        assert_eq!(result.values[2], ExtRational::Finite(one()));
    }

    #[test]
    fn until_reports_invalid_source_state_instead_of_panicking() {
        let mut product = graph(ProductKind::Dtmc, vec![edge(0, 0, one())]);
        product.edges[0].from = ProbStateId(1);
        assert_eq!(
            decide_until(
                &product,
                &[true],
                &[false],
                UnboundedObjective::Max,
                &ExactProbLimits::default(),
            ),
            Err(UnboundedError::InvalidState)
        );
    }

    /// Trace: FR-198's unbounded temporal forms.
    #[trace("TC-633", "FR-198-AC-3")]
    #[test]
    fn always_is_the_complement_of_eventually_not() {
        let product = graph(
            ProductKind::Dtmc,
            vec![
                edge(0, 1, ratio(1, 2)),
                edge(0, 2, ratio(1, 2)),
                edge(1, 1, one()),
                edge(2, 2, one()),
            ],
        );
        let result = decide_always(
            &product,
            &[true, false, true],
            UnboundedObjective::Min,
            &ExactProbLimits::default(),
        )
        .expect("always complements eventual failure");
        assert_eq!(result.values[0], ExtRational::Finite(ratio(1, 2)));
        assert_eq!(result.values[1], ExtRational::Finite(zero()));
        assert_eq!(result.values[2], ExtRational::Finite(one()));
    }

    /// Trace: FR-198-AC-3.
    #[trace("TC-633", "FR-198-AC-3")]
    #[test]
    fn interval_bounds_round_outward_and_separate_a_threshold() {
        let product = graph(
            ProductKind::Dtmc,
            vec![
                edge(0, 0, ratio(1, 3)),
                edge(0, 1, ratio(1, 3)),
                edge(0, 2, ratio(1, 3)),
                edge(1, 1, one()),
                edge(2, 2, one()),
            ],
        );
        let result = decide_reachability_threshold(
            &product,
            &[false, true, false],
            UnboundedObjective::Max,
            &ratio(2, 5),
            &ExactProbLimits::default(),
        )
        .expect("intervals separate the threshold");
        let UnboundedOutcome::Bounds {
            lower,
            upper,
            sweeps,
            ..
        } = result
        else {
            panic!("the interval should decide: {result:?}");
        };
        assert_eq!(sweeps, 2);
        assert!(lower[0].rational() > ratio(2, 5));
        assert!(lower[0].rational() < ratio(1, 2));
        assert!(upper[0].rational() > ratio(1, 2));
        assert!(upper[0].rational() < ratio(3, 5));
    }

    /// Trace: FR-198-AC-1.
    #[trace("TC-633", "FR-198-AC-1")]
    #[test]
    fn scheduler_minimum_and_maximum_apply_qualitative_sets() {
        let product = graph(
            ProductKind::Mdp,
            vec![
                choice_edge(0, 0, one(), 4),
                choice_edge(0, 1, one(), 5),
                edge(1, 1, one()),
            ],
        );
        let minimum = decide_reachability(
            &product,
            &[false, true],
            UnboundedObjective::Min,
            &ExactProbLimits::default(),
        )
        .expect("wait is a zero-probability scheduler");
        let maximum = decide_reachability(
            &product,
            &[false, true],
            UnboundedObjective::Max,
            &ExactProbLimits::default(),
        )
        .expect("take reaches the target");
        assert_eq!(minimum.values[0], ExtRational::Finite(zero()));
        assert_eq!(maximum.values[0], ExtRational::Finite(one()));
        assert_eq!(minimum.policy[0], Some(0));
        assert_eq!(maximum.policy[0], Some(1));

        let link = graph(
            ProductKind::Mdp,
            vec![
                edge(0, 1, ratio(24, 25)),
                edge(0, 2, ratio(1, 25)),
                edge(1, 1, one()),
                edge(2, 2, one()),
            ],
        );
        let delivery = decide_reachability(
            &link,
            &[false, true, false],
            UnboundedObjective::Min,
            &ExactProbLimits::default(),
        )
        .expect("the Link delivery probability is exact");
        assert_eq!(delivery.values[0], ExtRational::Finite(ratio(24, 25)));
    }

    /// Trace: FR-198-AC-4.
    #[trace("TC-633", "FR-198-AC-4")]
    #[test]
    fn equal_threshold_falls_back_to_policy_iteration_and_observes_its_budget() {
        let product = graph(
            ProductKind::Mdp,
            vec![
                edge(0, 0, ratio(1, 3)),
                edge(0, 1, ratio(1, 3)),
                edge(0, 2, ratio(1, 3)),
                edge(1, 1, one()),
                edge(2, 2, one()),
            ],
        );
        let limits = ExactProbLimits {
            max_iterations: 3,
            max_policy_iterations: 0,
            ..ExactProbLimits::default()
        };
        let stopped = decide_reachability_threshold(
            &product,
            &[false, true, false],
            UnboundedObjective::Max,
            &ratio(1, 2),
            &limits,
        )
        .expect("budget is a typed solver result");
        assert_eq!(
            stopped,
            UnboundedOutcome::Stopped(qsl_replay::ExactProbLimit::MaxPolicyIterations)
        );
        let solved_limits = ExactProbLimits {
            max_iterations: 3,
            max_policy_iterations: 10,
            ..ExactProbLimits::default()
        };
        let first = decide_reachability_threshold(
            &product,
            &[false, true, false],
            UnboundedObjective::Max,
            &ratio(1, 2),
            &solved_limits,
        )
        .expect("policy iteration decides at equality");
        let second = decide_reachability_threshold(
            &product,
            &[false, true, false],
            UnboundedObjective::Max,
            &ratio(1, 2),
            &solved_limits,
        )
        .expect("the exact result is deterministic");
        assert_eq!(first, second);
        let UnboundedOutcome::Exact(exact) = first else {
            panic!("equality is settled exactly: {first:?}");
        };
        assert_eq!(exact.values[0], ExtRational::Finite(ratio(1, 2)));
    }

    /// Trace: FR-198-AC-4.
    #[trace("TC-633", "FR-198-AC-4")]
    #[test]
    fn rational_budget_returns_the_final_sound_interval() {
        let product = graph(
            ProductKind::Dtmc,
            vec![
                edge(0, 0, ratio(1, 3)),
                edge(0, 1, ratio(1, 3)),
                edge(0, 2, ratio(1, 3)),
                edge(1, 1, one()),
                edge(2, 2, one()),
            ],
        );
        let limits = ExactProbLimits {
            max_iterations: 3,
            max_rational_bits: 0,
            ..ExactProbLimits::default()
        };
        let result = decide_reachability_threshold(
            &product,
            &[false, true, false],
            UnboundedObjective::Max,
            &ratio(1, 2),
            &limits,
        )
        .expect("the rational ceiling returns sound bounds");
        let UnboundedOutcome::PrecisionBudget { lower, upper } = result else {
            panic!("the rational ceiling is explicit: {result:?}");
        };
        assert!(lower.rational() <= ratio(1, 2));
        assert!(upper.rational() >= ratio(1, 2));
    }

    /// Trace: FR-198-AC-5.
    #[trace("TC-633", "FR-198-AC-5")]
    #[test]
    fn max_reachability_expands_a_collapsed_mec_through_its_exit_action() {
        let product = graph(
            ProductKind::Mdp,
            vec![
                choice_edge(0, 0, one(), 4),
                choice_edge(0, 1, ratio(1, 2), 5),
                choice_edge(0, 2, ratio(1, 2), 5),
                edge(1, 1, one()),
                edge(2, 2, one()),
            ],
        );
        let result = decide_reachability_threshold(
            &product,
            &[false, true, false],
            UnboundedObjective::Max,
            &ratio(1, 4),
            &ExactProbLimits::default(),
        )
        .expect("MEC collapse decides the threshold");
        let UnboundedOutcome::Bounds {
            lower,
            upper,
            policy,
            ..
        } = result
        else {
            panic!("the one-sweep interval decides: {result:?}");
        };
        assert_eq!(lower[0].rational(), ratio(1, 2));
        assert_eq!(upper[0].rational(), ratio(1, 2));
        assert_eq!(policy[0], Some(1));
    }

    /// Trace: FR-198-AC-2.
    #[trace("TC-633", "FR-198-AC-2")]
    #[test]
    fn expected_reward_is_infinite_when_the_target_cannot_be_reached() {
        let mut loop_edge = edge(0, 0, one());
        loop_edge.rewards.push((
            quire_exact::Identifier::new("cost").expect("identifier"),
            Value::Integer(Integer::one()),
        ));
        let product = graph(ProductKind::Dtmc, vec![loop_edge, edge(1, 1, one())]);
        for objective in [UnboundedObjective::Min, UnboundedObjective::Max] {
            let result = decide_expected_reward(
                &product,
                &[false, true],
                "cost",
                objective,
                &ExactProbLimits::default(),
            )
            .expect("unbounded reward is decided by graph structure");
            assert_eq!(result.values[0], ExtRational::PlusInfinity);
        }
    }

    /// Trace: FR-198-AC-2.
    #[trace("TC-633", "FR-198-AC-2")]
    #[test]
    fn expected_reward_is_solved_exactly_when_the_target_is_almost_sure() {
        let mut loop_edge = edge(0, 0, ratio(1, 2));
        loop_edge.rewards.push((
            quire_exact::Identifier::new("cost").expect("identifier"),
            Value::Integer(Integer::one()),
        ));
        let mut target_edge = edge(0, 1, ratio(1, 2));
        target_edge.rewards.push((
            quire_exact::Identifier::new("cost").expect("identifier"),
            Value::Integer(Integer::one()),
        ));
        let product = graph(
            ProductKind::Dtmc,
            vec![loop_edge, target_edge, edge(1, 1, one())],
        );
        let result = decide_expected_reward(
            &product,
            &[false, true],
            "cost",
            UnboundedObjective::Min,
            &ExactProbLimits::default(),
        )
        .expect("the target is reached almost surely");
        assert_eq!(result.values[0], ExtRational::Finite(ratio(2, 1)));
    }

    /// Trace: FR-198-AC-2.
    #[trace("TC-633", "FR-198-AC-2")]
    #[test]
    fn expected_reward_minimum_and_maximum_choose_exact_policies() {
        let action_a = 7;
        let action_b = 8;
        let mut a_loop = choice_edge(0, 0, ratio(1, 11), action_a);
        a_loop.rewards.push((
            quire_exact::Identifier::new("cost").expect("identifier"),
            Value::Integer(Integer::one()),
        ));
        let mut a_goal = choice_edge(0, 1, ratio(10, 11), action_a);
        a_goal.rewards.push((
            quire_exact::Identifier::new("cost").expect("identifier"),
            Value::Integer(Integer::one()),
        ));
        let mut b_loop = choice_edge(0, 0, ratio(1, 6), action_b);
        b_loop.rewards.push((
            quire_exact::Identifier::new("cost").expect("identifier"),
            Value::Integer(Integer::one()),
        ));
        let mut b_goal = choice_edge(0, 1, ratio(5, 6), action_b);
        b_goal.rewards.push((
            quire_exact::Identifier::new("cost").expect("identifier"),
            Value::Integer(Integer::one()),
        ));
        let product = graph(
            ProductKind::Mdp,
            vec![a_loop, a_goal, b_loop, b_goal, edge(1, 1, one())],
        );
        let minimum = decide_expected_reward(
            &product,
            &[false, true],
            "cost",
            UnboundedObjective::Min,
            &ExactProbLimits::default(),
        )
        .expect("both actions reach the target almost surely");
        let maximum = decide_expected_reward(
            &product,
            &[false, true],
            "cost",
            UnboundedObjective::Max,
            &ExactProbLimits::default(),
        )
        .expect("both actions reach the target almost surely");
        assert_eq!(minimum.values[0], ExtRational::Finite(ratio(11, 10)));
        assert_eq!(maximum.values[0], ExtRational::Finite(ratio(6, 5)));
        assert_eq!(minimum.policy[0], Some(0));
        assert_eq!(maximum.policy[0], Some(1));
    }

    /// Trace: FR-198-AC-2.
    #[trace("TC-633", "FR-198-AC-2")]
    #[test]
    fn minimum_expected_reward_rejects_an_action_into_an_infinite_child() {
        let mut good = choice_edge(0, 1, one(), 1);
        good.rewards.push((
            quire_exact::Identifier::new("cost").expect("identifier"),
            Value::Integer(Integer::from(10_i64)),
        ));
        let mut bad_target = choice_edge(0, 1, ratio(1, 2), 2);
        bad_target.rewards.push((
            quire_exact::Identifier::new("cost").expect("identifier"),
            Value::Integer(Integer::zero()),
        ));
        let mut bad_recurrent = choice_edge(0, 2, ratio(1, 2), 2);
        bad_recurrent.rewards.push((
            quire_exact::Identifier::new("cost").expect("identifier"),
            Value::Integer(Integer::zero()),
        ));
        let mut recurrent = choice_edge(2, 2, one(), 3);
        recurrent.rewards.push((
            quire_exact::Identifier::new("cost").expect("identifier"),
            Value::Integer(Integer::one()),
        ));
        let product = graph(
            ProductKind::Mdp,
            vec![
                good,
                bad_target,
                bad_recurrent,
                edge(1, 1, one()),
                recurrent,
            ],
        );
        let result = decide_expected_reward(
            &product,
            &[false, true, false],
            "cost",
            UnboundedObjective::Min,
            &ExactProbLimits::default(),
        )
        .expect("the good action is the finite minimum");
        assert_eq!(result.values[0], ExtRational::Finite(ratio(10, 1)));
        assert_eq!(result.values[2], ExtRational::PlusInfinity);
        assert_eq!(result.policy[0], Some(0));
    }

    /// Trace: FR-199-AC-1.
    #[trace("TC-634", "FR-199-AC-1")]
    #[test]
    fn workload_long_run_fraction_uses_an_exact_stationary_distribution() {
        let product = graph(
            ProductKind::Dtmc,
            vec![
                edge(0, 0, ratio(3, 4)),
                edge(0, 1, ratio(1, 4)),
                edge(1, 0, one()),
            ],
        );
        let result =
            decide_long_run_workload(&product, &[true, false], None, &ExactProbLimits::default())
                .expect("the finite DTMC has a BSCC");
        assert_eq!(result.components.len(), 1);
        assert_eq!(result.components[0].rho, ratio(4, 5));
        assert_eq!(result.gain_bias[0].gain, ratio(4, 5));
        assert_eq!(
            result.gain_bias[0].bias,
            vec![(ProbStateId(0), zero()), (ProbStateId(1), ratio(-4, 5)),]
        );

        let authored = graph(
            ProductKind::Dtmc,
            vec![
                edge(0, 0, ratio(1999, 2000)),
                edge(0, 1, ratio(1, 2000)),
                edge(1, 0, ratio(9, 10)),
                edge(1, 1, ratio(1, 10)),
            ],
        );
        let authored_result =
            decide_long_run_workload(&authored, &[true, false], None, &ExactProbLimits::default())
                .expect("the authored workload BSCC is irreducible");
        assert_eq!(authored_result.components[0].rho, ratio(1800, 1801));
        assert_eq!(authored_result.gain_bias[0].gain, ratio(1800, 1801));
        assert_eq!(
            authored_result.gain_bias[0].bias,
            vec![
                (ProbStateId(0), zero()),
                (ProbStateId(1), ratio(-2000, 1801)),
            ]
        );
        assert_eq!(result.values[0], ExtRational::Finite(ratio(4, 5)));
        assert_eq!(result.values[1], ExtRational::Finite(ratio(4, 5)));

        let duration = quire_exact::Identifier::new("duration").expect("identifier");
        let mut up_self = edge(0, 0, ratio(3, 4));
        up_self
            .rewards
            .push((duration.clone(), Value::Integer(Integer::one())));
        let mut up_down = edge(0, 1, ratio(1, 4));
        up_down
            .rewards
            .push((duration.clone(), Value::Integer(Integer::one())));
        let mut down_up = edge(1, 0, one());
        down_up
            .rewards
            .push((duration.clone(), Value::Integer(Integer::from(3_i64))));
        let weighted = graph(ProductKind::Dtmc, vec![up_self, up_down, down_up]);
        let weighted_result = decide_long_run_workload(
            &weighted,
            &[true, false],
            Some("duration"),
            &ExactProbLimits::default(),
        )
        .expect("duration gives each position its leaving-step weight");
        assert_eq!(weighted_result.components[0].rho, ratio(4, 7));

        let mut zero_weight = edge(0, 0, one());
        zero_weight
            .rewards
            .push((duration, Value::Integer(Integer::zero())));
        let zero_component = graph(ProductKind::Dtmc, vec![zero_weight]);
        assert_eq!(
            decide_long_run_workload(
                &zero_component,
                &[true],
                Some("duration"),
                &ExactProbLimits::default(),
            ),
            Err(UnboundedError::ZeroWeightComponent)
        );
    }

    /// Trace: FR-199-AC-2.
    #[trace("TC-634", "FR-199-AC-2")]
    #[test]
    fn mdp_long_run_optimizes_exact_fractions_per_mec() {
        let product = graph(
            ProductKind::Mdp,
            vec![
                choice_edge(0, 0, ratio(1999, 2000), 1),
                choice_edge(0, 1, ratio(1, 2000), 1),
                choice_edge(1, 0, ratio(9, 10), 2),
                choice_edge(1, 1, ratio(1, 10), 2),
                choice_edge(1, 0, ratio(1, 2), 3),
                choice_edge(1, 1, ratio(1, 2), 3),
            ],
        );
        let limits = ExactProbLimits::default();
        let minimum = decide_long_run_mdp(
            &product,
            &[true, false],
            None,
            UnboundedObjective::Min,
            &limits,
        )
        .expect("the MDP MEC has two exact policies");
        let maximum = decide_long_run_mdp(
            &product,
            &[true, false],
            None,
            UnboundedObjective::Max,
            &limits,
        )
        .expect("the MDP MEC has two exact policies");
        assert_eq!(minimum.components[0].rho, ratio(1000, 1001));
        assert_eq!(maximum.components[0].rho, ratio(1800, 1801));
        assert_eq!(minimum.policy.as_ref().and_then(|p| p[1]), Some(1));
        assert_eq!(maximum.policy.as_ref().and_then(|p| p[1]), Some(0));

        let workload = graph(
            ProductKind::Dtmc,
            vec![
                edge(0, 0, ratio(1999, 2000)),
                edge(0, 1, ratio(1, 2000)),
                edge(1, 0, ratio(7, 10)),
                edge(1, 1, ratio(3, 10)),
            ],
        );
        let expected = decide_long_run_workload(&workload, &[true, false], None, &limits)
            .expect("the workload chooses the two repairs equally");
        assert_eq!(expected.components[0].rho, ratio(1400, 1401));
    }

    /// Trace: FR-199-AC-2.
    #[trace("TC-634", "FR-199-AC-2")]
    #[test]
    fn mdp_long_run_uses_a_proper_sub_mec_reachable_from_the_maximal_mec() {
        let product = graph(
            ProductKind::Mdp,
            vec![
                choice_edge(0, 0, one(), 1),
                choice_edge(0, 1, one(), 2),
                choice_edge(1, 0, one(), 3),
            ],
        );
        let result = decide_long_run_mdp(
            &product,
            &[true, false],
            None,
            UnboundedObjective::Max,
            &ExactProbLimits::default(),
        )
        .expect("the scheduler can reach and remain in the proper end component");
        assert_eq!(result.components.len(), 1);
        assert_eq!(
            result.components[0].states,
            vec![ProbStateId(0), ProbStateId(1)]
        );
        assert_eq!(result.components[0].rho, one());
        assert_eq!(
            result.values,
            vec![ExtRational::Finite(one()), ExtRational::Finite(one())]
        );
        assert_eq!(result.policy.as_ref().and_then(|policy| policy[0]), Some(0));
        assert_eq!(result.policy.as_ref().and_then(|policy| policy[1]), Some(0));
    }

    /// A policy that stays at each state has two bottom SCCs with different
    /// values. An alternate policy can route every starting state to either
    /// optimal recurrent class, so the extremum is uniform over the MEC.
    #[trace("TC-634", "FR-199-AC-2")]
    #[test]
    fn mdp_long_run_routes_the_whole_mec_to_the_optimal_of_multiple_bsccs() {
        let product = graph(
            ProductKind::Mdp,
            vec![
                choice_edge(0, 0, one(), 1),
                choice_edge(0, 1, one(), 2),
                choice_edge(1, 1, one(), 3),
                choice_edge(1, 0, one(), 4),
            ],
        );
        let minimum = decide_long_run_mdp(
            &product,
            &[true, false],
            None,
            UnboundedObjective::Min,
            &ExactProbLimits::default(),
        )
        .expect("the scheduler can route both states to B");
        let maximum = decide_long_run_mdp(
            &product,
            &[true, false],
            None,
            UnboundedObjective::Max,
            &ExactProbLimits::default(),
        )
        .expect("the scheduler can route both states to A");
        assert_eq!(minimum.components[0].rho, zero());
        assert_eq!(minimum.values, vec![ExtRational::Finite(zero()); 2]);
        assert_eq!(
            minimum.policy.as_ref().and_then(|policy| policy[0]),
            Some(1)
        );
        assert_eq!(
            minimum.policy.as_ref().and_then(|policy| policy[1]),
            Some(0)
        );
        assert_eq!(
            minimum.gain_bias[0].bias,
            vec![(ProbStateId(0), one()), (ProbStateId(1), zero())]
        );
        assert_eq!(maximum.components[0].rho, one());
        assert_eq!(maximum.values, vec![ExtRational::Finite(one()); 2]);
        assert_eq!(
            maximum.policy.as_ref().and_then(|policy| policy[0]),
            Some(0)
        );
        assert_eq!(
            maximum.policy.as_ref().and_then(|policy| policy[1]),
            Some(1)
        );
        assert_eq!(
            maximum.gain_bias[0].bias,
            vec![(ProbStateId(0), zero()), (ProbStateId(1), ratio(-1, 1))]
        );
    }

    /// A stochastic transient action at S reaches A or B; B then reaches A.
    /// The policy can reach the optimal proper end component {A} almost
    /// surely even though the first step branches probabilistically.
    #[trace("TC-634", "FR-199-AC-2")]
    #[test]
    fn mdp_long_run_routes_probabilistic_transient_successors_to_a_sub_mec() {
        let product = graph(
            ProductKind::Mdp,
            vec![
                choice_edge(0, 1, ratio(1, 2), 1),
                choice_edge(0, 2, ratio(1, 2), 1),
                choice_edge(1, 1, one(), 2),
                choice_edge(1, 0, one(), 3),
                choice_edge(2, 1, one(), 4),
            ],
        );
        let predicate = [false, true, false];
        let maximum = decide_long_run_mdp(
            &product,
            &predicate,
            None,
            UnboundedObjective::Max,
            &ExactProbLimits::default(),
        )
        .expect("both probabilistic route branches reach the optimal sub-MEC");
        assert_eq!(maximum.components.len(), 1);
        assert_eq!(maximum.components[0].states.len(), 3);
        assert_eq!(maximum.components[0].rho, one());
        assert_eq!(maximum.values, vec![ExtRational::Finite(one()); 3]);
        assert_eq!(
            maximum.policy.as_ref().and_then(|policy| policy[0]),
            Some(0)
        );
        assert_eq!(
            maximum.policy.as_ref().and_then(|policy| policy[1]),
            Some(0)
        );
        assert_eq!(
            maximum.policy.as_ref().and_then(|policy| policy[2]),
            Some(0)
        );

        let minimum = decide_long_run_mdp(
            &product,
            &predicate,
            None,
            UnboundedObjective::Min,
            &ExactProbLimits::default(),
        )
        .expect("the minimum can use a recurrent policy that includes the route");
        assert_eq!(minimum.components.len(), 1);
        assert_eq!(minimum.components[0].rho, ratio(2, 5));
        assert_eq!(minimum.values, vec![ExtRational::Finite(ratio(2, 5)); 3]);
        assert_eq!(
            minimum.policy.as_ref().and_then(|policy| policy[0]),
            Some(0)
        );
        assert_eq!(
            minimum.policy.as_ref().and_then(|policy| policy[1]),
            Some(1)
        );
        assert_eq!(
            minimum.policy.as_ref().and_then(|policy| policy[2]),
            Some(0)
        );
    }
}
