// SPDX-License-Identifier: AGPL-3.0-or-later
//! Fair end components for exact probabilistic scheduler products (FR-200).

use std::collections::{BTreeSet, VecDeque};

use qsl_replay::ExactProbLimits;
use qsl_semantics::check::Bound;
use quire_exact::{Integer, Rational};
use quire_semantic_value::fairness::{FairnessConstraint, FairnessGranularity, FairnessKind};

use super::unbounded::{
    decide_expected_reward, decide_reachability, decide_until, maximal_end_components,
    maximal_end_components_from, optimize_mec_fraction, policy_iteration, Choice, ComponentValue,
    ExtRational, GainBias, Graph, LongRunResult, UnboundedError, UnboundedObjective,
};
use super::{decide_bounded_event, FhError, FhResult, ProbProduct, ProbStateId, ProductKind};

/// One maximal fair end component and the scheduled identities that keep it
/// closed. The action list is empty for a terminal-stutter-only component,
/// because terminal stutter belongs to no fairness constraint.
#[derive(Clone, Debug)]
pub struct FairEndComponent {
    /// Product states in canonical discovery order.
    pub states: Vec<ProbStateId>,
    /// Scheduled identities available internally in the component.
    pub actions: Vec<(ProbStateId, super::ScheduledIdentity)>,
}

/// Why a fair-component computation could not inspect its product.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum FairProductError {
    /// The product graph or its exact action probabilities are malformed.
    #[error("invalid probabilistic product: {0}")]
    Product(#[from] UnboundedError),
    /// A checked fairness operation has no matching operation identity in the product.
    #[error("a checked fairness operation has no product identity")]
    MissingOperation,
    /// The selected scheduler induces a bottom component that violates fairness.
    #[error("the generated scheduler witness is not fair")]
    UnfairWitness,
    /// The fair reachability operation needs an MDP product and matching labels.
    #[error("fair reachability requires an MDP product and valid state labels")]
    InvalidForm,
}

/// A scheduler action group, by its index among the state's enabled actions.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ActionChoice(pub usize);

/// A deterministic or exact randomized scheduler choice.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SchedulerChoice {
    /// Take one action group.
    Deterministic(ActionChoice),
    /// Randomize over action groups with exact probabilities.
    Randomized(Vec<(ActionChoice, Rational)>),
}

/// Exact reachability values and a fairness-aware scheduler.
#[derive(Clone, Debug)]
pub struct FairUnboundedResult {
    /// Exact value for each product state.
    pub values: Vec<ExtRational>,
    /// One fair scheduler choice for each state that has available actions.
    pub policy: Vec<Option<SchedulerChoice>>,
    /// Fair end components used by the transformation.
    pub components: Vec<FairEndComponent>,
    /// The checked fairness set carried by the product.
    pub fairness: Vec<FairnessConstraint>,
}

/// A finite-horizon result paired with its checked fairness set. The value is
/// exactly FR-197's result because fairness does not change bounded forms.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FairBoundedResult {
    /// FR-197's exact value or sound precision result.
    pub outcome: FhResult,
    /// The checked fairness set recorded with the decision.
    pub fairness: Vec<FairnessConstraint>,
}

/// Exact fair long-run values and the randomized witness, when the bound is
/// refuted.
#[derive(Clone, Debug)]
pub struct FairLongRunResult {
    /// Optimal fair end-component values and gain–bias data.
    pub components: Vec<ComponentValue>,
    /// Exact value for each product state, including transient routing.
    pub values: Vec<ExtRational>,
    /// Gain–bias certificates for each fair end component.
    pub gain_bias: Vec<GainBias>,
    /// Randomized scheduler returned for a refutation, if any.
    pub policy: Option<Vec<Option<SchedulerChoice>>>,
    /// Fair randomized scheduler used to refute the bound, when one is needed.
    pub witness: Option<FairRandomizedWitness>,
    /// The checked fairness set recorded with the result.
    pub fairness: Vec<FairnessConstraint>,
}

/// Randomized fair scheduler used as exact refutation evidence.
#[derive(Clone, Debug)]
pub struct FairRandomizedWitness {
    /// The exploration exponent in `2^-k`.
    pub k: u64,
    /// One randomized scheduler choice at each state that has actions.
    pub policy: Vec<Option<SchedulerChoice>>,
    /// The exact values of the induced Markov chain.
    pub values: Vec<ExtRational>,
}

/// Exact expected-reward values and their checked fairness set.
#[derive(Clone, Debug)]
pub struct FairExpectedRewardResult {
    /// Exact finite value or positive infinity for each product state.
    pub values: Vec<ExtRational>,
    /// Fair deterministic or randomized scheduler choice per state.
    pub policy: Option<Vec<Option<SchedulerChoice>>>,
    /// The checked fairness set recorded with the result.
    pub fairness: Vec<FairnessConstraint>,
}

/// Decide a bounded event unchanged and carry the product's fairness set.
pub fn decide_fair_bounded_event(
    product: &ProbProduct,
    bound: &Bound,
    limits: &ExactProbLimits,
) -> Result<FairBoundedResult, FhError> {
    Ok(FairBoundedResult {
        outcome: decide_bounded_event(product, bound, limits)?,
        fairness: product.fairness.clone(),
    })
}

#[derive(Clone)]
struct FairAction<'a> {
    whole: bool,
    identity: Option<&'a super::ScheduledIdentity>,
    operation: &'a str,
    kind: FairnessKind,
}

/// Decompose an MDP into maximal fair end components by the FR-200 refinement.
///
/// A whole-operation constraint covers all identities with the resolved
/// declaration identity. An each-invocation constraint is refined into one
/// obligation per enabled scheduled identity (receiver and arguments).
pub fn maximal_fair_end_components(
    product: &ProbProduct,
    fairness: &[FairnessConstraint],
) -> Result<Vec<FairEndComponent>, FairProductError> {
    let graph = Graph::new(product)?;
    fair_components(product, &graph, &graph, fairness)
}

fn fair_components(
    product: &ProbProduct,
    graph: &Graph<'_>,
    enabled_graph: &Graph<'_>,
    fairness: &[FairnessConstraint],
) -> Result<Vec<FairEndComponent>, FairProductError> {
    let actions = fairness_actions(product, fairness)?;

    let mut pending = VecDeque::from(maximal_end_components(graph)?);
    let mut components = Vec::new();
    while let Some(states) = pending.pop_front() {
        let candidate = states.iter().copied().collect::<BTreeSet<_>>();
        let mut split = None;
        let mut discarded = false;
        for fairness_action in &actions {
            let enabled = candidate
                .iter()
                .map(|state| {
                    enabled_graph.choices[*state]
                        .iter()
                        .any(|choice| choice_matches(choice, fairness_action))
                })
                .collect::<Vec<_>>();
            let taken = candidate.iter().any(|state| {
                graph.choices[*state].iter().any(|choice| {
                    choice_matches(choice, fairness_action)
                        && choice
                            .edges
                            .iter()
                            .all(|edge| candidate.contains(&edge.to.index()))
                })
            });
            if taken {
                continue;
            }
            match fairness_action.kind {
                FairnessKind::Weak if enabled.iter().all(|value| *value) => {
                    discarded = true;
                    break;
                }
                FairnessKind::Strong if enabled.iter().any(|value| *value) => {
                    let retained = candidate
                        .iter()
                        .copied()
                        .filter(|state| !enabled[candidate_position(&candidate, *state)])
                        .collect::<BTreeSet<_>>();
                    if retained.is_empty() {
                        discarded = true;
                    } else {
                        split = Some(mecs_in(graph, retained)?);
                    }
                    break;
                }
                _ => {}
            }
        }
        if discarded {
            continue;
        }
        if let Some(children) = split {
            pending.extend(children);
            continue;
        }
        let mut fair_actions = Vec::new();
        for state in candidate.iter().copied() {
            for choice in &graph.choices[state] {
                if choice
                    .edges
                    .iter()
                    .all(|edge| candidate.contains(&edge.to.index()))
                {
                    if let Some(identity) =
                        choice.edges.first().and_then(|edge| edge.action.as_ref())
                    {
                        if !fair_actions.iter().any(
                            |(at, seen): &(ProbStateId, super::ScheduledIdentity)| {
                                *at == ProbStateId(state) && seen == identity
                            },
                        ) {
                            fair_actions.push((ProbStateId(state), identity.clone()));
                        }
                    }
                }
            }
        }
        components.push(FairEndComponent {
            states: candidate.into_iter().map(ProbStateId).collect(),
            actions: fair_actions,
        });
    }
    components.sort_by_key(|component| component.states.first().copied());
    Ok(components)
}

fn fairness_actions<'a>(
    product: &'a ProbProduct,
    fairness: &'a [FairnessConstraint],
) -> Result<Vec<FairAction<'a>>, FairProductError> {
    let mut actions = Vec::new();
    for constraint in fairness {
        let mapping = product
            .fairness_operations
            .iter()
            .find(|entry| entry.operation == *constraint.operation())
            .ok_or(FairProductError::MissingOperation)?;
        if constraint.granularity() == FairnessGranularity::Whole {
            actions.push(FairAction {
                whole: true,
                identity: None,
                operation: &mapping.text,
                kind: constraint.kind(),
            });
        } else {
            let mut identities = Vec::<&super::ScheduledIdentity>::new();
            for edge in &product.edges {
                if let Some(identity) = edge.action.as_ref() {
                    if identity.operation == mapping.text && !identities.contains(&identity) {
                        identities.push(identity);
                    }
                }
            }
            for identity in identities {
                actions.push(FairAction {
                    whole: false,
                    identity: Some(identity),
                    operation: &mapping.text,
                    kind: constraint.kind(),
                });
            }
        }
    }
    Ok(actions)
}

/// Decide unbounded reachability over fair schedulers (FR-200).
///
/// For a minimum, this applies FR-200's fair-end-component complement
/// transformation. A maximum uses FR-198's optimum and fills zero-value
/// states with a uniform randomized scheduler.
pub fn decide_fair_reachability(
    product: &ProbProduct,
    from: Option<&[bool]>,
    target: &[bool],
    objective: UnboundedObjective,
    limits: &ExactProbLimits,
) -> Result<FairUnboundedResult, FairProductError> {
    if product.kind != ProductKind::Mdp
        || target.len() != product.states.len()
        || from.is_some_and(|labels| labels.len() != product.states.len())
    {
        return Err(FairProductError::InvalidForm);
    }
    let graph = Graph::new(product)?;
    let all_non_target = target.iter().map(|value| !value).collect::<Vec<_>>();
    let until_domain = from
        .map(|labels| {
            labels
                .iter()
                .zip(target)
                .map(|(from, target)| *from && !*target)
                .collect()
        })
        .unwrap_or_else(|| all_non_target.clone());
    let allowed = &until_domain;
    let components = if objective == UnboundedObjective::Min {
        let restricted = graph_restricted_to(&graph, allowed);
        fair_components(product, &restricted, &graph, &product.fairness)?
    } else {
        maximal_fair_end_components(product, &product.fairness)?
    };

    let (values, policy_values, optimizing_policy) = match objective {
        UnboundedObjective::Max => {
            let result = if let Some(from) = from {
                decide_until(product, from, target, objective, limits)?
            } else {
                decide_reachability(product, target, objective, limits)?
            };
            (result.values.clone(), result.values, result.policy)
        }
        UnboundedObjective::Min => {
            let mut fair_states = vec![false; product.states.len()];
            for component in &components {
                for state in &component.states {
                    fair_states[state.index()] = true;
                }
            }
            let transformed = if let Some(from) = from {
                decide_until(product, from, &fair_states, UnboundedObjective::Max, limits)?
            } else {
                decide_until(
                    product,
                    &all_non_target,
                    &fair_states,
                    UnboundedObjective::Max,
                    limits,
                )?
            };
            let values = transformed
                .values
                .clone()
                .into_iter()
                .map(|value| match value {
                    ExtRational::Finite(value) => ExtRational::Finite(one().sub(&value)),
                    ExtRational::PlusInfinity => ExtRational::Finite(zero()),
                })
                .collect();
            (values, transformed.values, transformed.policy)
        }
    };

    let policy = fair_witness_policy(
        product,
        &graph,
        &components,
        &policy_values,
        optimizing_policy,
        objective,
    );
    validate_fair_witness(product, &graph, &policy, &product.fairness)?;
    Ok(FairUnboundedResult {
        values,
        policy,
        components,
        fairness: product.fairness.clone(),
    })
}

/// Decide a long-run fraction over fair schedulers, optimizing each refined
/// fair MEC exactly and constructing a randomized refutation witness when the
/// extremal value crosses the checked bound.
pub fn decide_fair_long_run_mdp(
    product: &ProbProduct,
    predicate: &[bool],
    weight: Option<&str>,
    bound: &Bound,
    limits: &ExactProbLimits,
) -> Result<FairLongRunResult, FairProductError> {
    if product.kind != ProductKind::Mdp || predicate.len() != product.states.len() {
        return Err(FairProductError::InvalidForm);
    }
    let objective = match bound {
        Bound::AtLeast(_) => UnboundedObjective::Min,
        Bound::AtMost(_) => UnboundedObjective::Max,
    };
    let graph = Graph::new(product)?;
    let fair_components = maximal_fair_end_components(product, &product.fairness)?;
    let normal_mecs = maximal_end_components(&graph)?;
    let mut fixed = vec![None; product.states.len()];
    let mut selected_policy = vec![None; product.states.len()];
    let mut components = Vec::new();
    let mut gain_bias = Vec::new();
    for fair_component in &fair_components {
        let states = fair_component
            .states
            .iter()
            .map(|state| state.index())
            .collect::<Vec<_>>();
        let (rho, policy, bias, _) = optimize_mec_fraction(
            &graph,
            &states,
            predicate,
            weight,
            objective,
            limits.max_policy_iterations,
            limits.max_rational_bits,
        )?;
        for state in &states {
            fixed[*state] = Some(rho.clone());
            selected_policy[*state] = policy[*state];
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
                .enumerate()
                .map(|(local, state)| (ProbStateId(state), bias[local].clone()))
                .collect(),
        });
    }

    // Terminal stutter components are vacuously fair continuations, but are
    // not emitted as scheduler-action fair components. They still need a
    // terminal long-run value when other states route to them.
    for component in &normal_mecs {
        let terminal = component.iter().all(|state| {
            graph.choices[*state].iter().all(|choice| {
                choice
                    .edges
                    .iter()
                    .all(|edge| matches!(edge.choice, super::ProductAction::TerminalStutter))
            })
        });
        if !terminal {
            continue;
        }
        let (rho, policy, _, _) = optimize_mec_fraction(
            &graph,
            component,
            predicate,
            weight,
            objective,
            limits.max_policy_iterations,
            limits.max_rational_bits,
        )?;
        for state in component {
            fixed[*state] = Some(rho.clone());
            selected_policy[*state] = policy[*state];
        }
    }
    if fixed.iter().all(Option::is_none) {
        return Err(FairProductError::InvalidForm);
    }
    let terminal_or_fair = fixed.iter().map(Option::is_some).collect::<Vec<_>>();
    let absorbing_product = absorbing_target_product(product, &terminal_or_fair);
    let absorbing_graph = Graph::new(&absorbing_product)?;
    let collapsed_mecs = maximal_end_components(&absorbing_graph)?;
    let (collapsed_product, collapsed_target, state_map, collapsed) = collapse_zero_reward_mecs(
        &absorbing_product,
        &absorbing_graph,
        &terminal_or_fair,
        &collapsed_mecs,
    )?;
    let collapsed_graph = Graph::new(&collapsed_product)?;
    let mut collapsed_fixed = vec![None; product.states.len()];
    for state in 0..product.states.len() {
        if let Some(value) = fixed[state].as_ref() {
            collapsed_fixed[state_map[state]] = Some(value.clone());
        }
    }
    for state in 0..product.states.len() {
        if collapsed_target[state] && collapsed_fixed[state].is_none() {
            collapsed_fixed[state] = Some(zero());
        }
    }
    let mut initial_policy = vec![None; product.states.len()];
    for (state, choices) in collapsed_graph.choices.iter().enumerate() {
        if !choices.is_empty() {
            initial_policy[state] = Some(0);
        }
    }
    let (values, deterministic_policy, _) = policy_iteration(
        &collapsed_graph,
        &collapsed_fixed,
        objective,
        limits.max_policy_iterations,
        initial_policy,
        limits.max_rational_bits,
    )?;
    let original_policy =
        expand_collapsed_policy(&graph, &deterministic_policy, &state_map, &collapsed)?;
    let mut deterministic_policy = original_policy;
    for (state, selected) in selected_policy.into_iter().enumerate() {
        if fixed[state].is_some() {
            deterministic_policy[state] = selected;
        }
    }
    let values = (0..product.states.len())
        .map(|state| ExtRational::Finite(values[state_map[state]].clone()))
        .collect::<Vec<_>>();
    let base = LongRunResult {
        components,
        values: values.clone(),
        policy: Some(deterministic_policy.clone()),
        gain_bias,
    };
    let threshold = match bound {
        Bound::AtLeast(value) | Bound::AtMost(value) => value,
    };
    let witness = if bound_refuted(product, &values, objective, threshold) {
        let mut found = None;
        for k in 1..=limits.max_policy_iterations {
            let policy = randomized_fair_policy(&graph, &fair_components, &deterministic_policy, k);
            let induced = induced_chain(product, &graph, &policy)?;
            let evaluated =
                super::unbounded::decide_long_run_workload(&induced, predicate, weight, limits)?;
            if bound_refuted(product, &evaluated.values, objective, threshold) {
                found = Some(FairRandomizedWitness {
                    k,
                    policy,
                    values: evaluated.values,
                });
                break;
            }
        }
        Some(found.ok_or(UnboundedError::MaxPolicyIterations)?)
    } else {
        None
    };
    if let Some(witness) = &witness {
        validate_fair_witness(product, &graph, &witness.policy, &product.fairness)?;
    }
    // `base` is intentionally unpacked here: only its public proof values are
    // returned; the deterministic optimizer policy is not emitted as evidence.
    Ok(FairLongRunResult {
        components: base.components,
        values: base.values,
        gain_bias: base.gain_bias,
        policy: witness.as_ref().map(|witness| witness.policy.clone()),
        witness,
        fairness: product.fairness.clone(),
    })
}

/// Decide an expected accumulated reward under the FR-200 fairness rules.
/// Minimum values use FR-198 directly. A maximum first applies the fair
/// probability-one and positive-reward end-component tests, then collapses
/// zero-reward recurrent components before the exact FR-198 solve.
pub fn decide_fair_expected_reward(
    product: &ProbProduct,
    target: &[bool],
    reward: &str,
    objective: UnboundedObjective,
    limits: &ExactProbLimits,
) -> Result<FairExpectedRewardResult, FairProductError> {
    if product.kind != ProductKind::Mdp || target.len() != product.states.len() {
        return Err(FairProductError::InvalidForm);
    }
    if objective == UnboundedObjective::Min {
        let result = decide_expected_reward(product, target, reward, objective, limits)?;
        let graph = Graph::new(product)?;
        let policy = deterministic_choices(&result.policy);
        validate_fair_witness(product, &graph, &policy, &product.fairness)?;
        return Ok(FairExpectedRewardResult {
            values: result.values,
            policy: Some(policy),
            fairness: product.fairness.clone(),
        });
    }

    let graph = Graph::new(product)?;
    let fair_reach =
        decide_fair_reachability(product, None, target, UnboundedObjective::Min, limits)?;
    let mecs = maximal_end_components(&graph)?;
    let mut positive_components = Vec::new();
    for component in &mecs {
        let members = component.iter().copied().collect::<BTreeSet<_>>();
        let positive = component.iter().any(|state| {
            graph.choices[*state].iter().any(|choice| {
                choice
                    .edges
                    .iter()
                    .all(|edge| members.contains(&edge.to.index()))
                    && choice
                        .edges
                        .iter()
                        .any(|edge| edge_reward_positive(edge, reward))
            })
        });
        if positive {
            positive_components.push(component.clone());
        }
    }
    let mut positive_states = vec![false; product.states.len()];
    for component in &positive_components {
        for state in component {
            positive_states[*state] = true;
        }
    }
    let positive_reach = if positive_states.iter().any(|value| *value) {
        Some(decide_reachability(
            product,
            &positive_states,
            UnboundedObjective::Max,
            limits,
        )?)
    } else {
        None
    };
    let mut infinite = vec![false; product.states.len()];
    for (state, infinite_state) in infinite.iter_mut().enumerate() {
        let fair_probability_one =
            matches!(&fair_reach.values[state], ExtRational::Finite(value) if value == &one());
        let can_reach_positive = positive_reach.as_ref().is_some_and(
            |result| matches!(&result.values[state], ExtRational::Finite(value) if value > &zero()),
        );
        *infinite_state = !fair_probability_one || can_reach_positive;
    }
    let mut collapsed_target = target.to_vec();
    for state in 0..product.states.len() {
        collapsed_target[state] |= infinite[state];
    }
    let (collapsed_product, collapsed_target, collapse_map, collapsed_mecs) =
        collapse_zero_reward_mecs(product, &graph, &collapsed_target, &mecs)?;
    let finite = decide_expected_reward(
        &collapsed_product,
        &collapsed_target,
        reward,
        UnboundedObjective::Max,
        limits,
    )?;
    let mut values = (0..product.states.len())
        .map(|state| finite.values[collapse_map[state]].clone())
        .collect::<Vec<_>>();
    for (state, value) in values.iter_mut().enumerate() {
        if infinite[state] {
            *value = ExtRational::PlusInfinity;
        }
    }
    let mut base_policy =
        expand_collapsed_policy(&graph, &finite.policy, &collapse_map, &collapsed_mecs)?;
    let fair_components = maximal_fair_end_components(product, &product.fairness)?;
    // In positive-reward fair MECs the unbounded value is realized by taking
    // a positive-reward internal action. Randomization adds every omitted
    // action with positive probability, preserving fairness.
    for component in &fair_components {
        for (state, identity) in &component.actions {
            let state = state.index();
            if let Some((index, _)) = graph.choices[state].iter().enumerate().find(|(_, choice)| {
                choice.edges.first().and_then(|edge| edge.action.as_ref()) == Some(identity)
                    && choice
                        .edges
                        .iter()
                        .any(|edge| edge_reward_positive(edge, reward))
            }) {
                base_policy[state] = Some(index);
            }
        }
    }
    let mut policy = randomized_fair_policy(&graph, &fair_components, &base_policy, 1);
    for (state, choice) in policy.iter_mut().enumerate() {
        if matches!(&fair_reach.values[state], ExtRational::Finite(value) if value < &one()) {
            *choice = fair_reach.policy[state].clone();
        }
    }
    validate_fair_witness(product, &graph, &policy, &product.fairness)?;
    Ok(FairExpectedRewardResult {
        values,
        policy: Some(policy),
        fairness: product.fairness.clone(),
    })
}

struct CollapsedMec {
    members: BTreeSet<usize>,
    representative: usize,
    exits: Vec<(usize, usize)>,
}

type ZeroRewardCollapse = (ProbProduct, Vec<bool>, Vec<usize>, Vec<CollapsedMec>);

fn collapse_zero_reward_mecs<'a>(
    product: &'a ProbProduct,
    graph: &Graph<'a>,
    target: &[bool],
    mecs: &[Vec<usize>],
) -> Result<ZeroRewardCollapse, UnboundedError> {
    let mut state_map = (0..product.states.len()).collect::<Vec<_>>();
    let mut collapsed_mecs = Vec::new();
    for component in mecs {
        if component.iter().any(|state| target[*state]) {
            continue;
        }
        let members = component.iter().copied().collect::<BTreeSet<_>>();
        let representative = *members.iter().next().ok_or(UnboundedError::InvalidState)?;
        for state in &members {
            state_map[*state] = representative;
        }
        collapsed_mecs.push(CollapsedMec {
            members,
            representative,
            exits: Vec::new(),
        });
    }
    let by_representative = collapsed_mecs
        .iter()
        .enumerate()
        .map(|(index, collapsed)| (collapsed.representative, index))
        .collect::<std::collections::BTreeMap<_, _>>();
    let mut collapsed_target = target.to_vec();
    let mut edges = Vec::new();
    for state in 0..product.states.len() {
        if state_map[state] != state {
            collapsed_target[state] = true;
            edges.push(stutter_edge(ProbStateId(state)));
            continue;
        }
        if let Some(collapsed_index) = by_representative.get(&state).copied() {
            let members = collapsed_mecs[collapsed_index].members.clone();
            let mut row_index = 0_u64;
            for owner in members.iter().copied() {
                for (choice_index, choice) in graph.choices[owner].iter().enumerate() {
                    if choice
                        .edges
                        .iter()
                        .all(|edge| members.contains(&edge.to.index()))
                    {
                        continue;
                    }
                    let mut key = product.states[state].clone();
                    key.model.extend_from_slice(b"\0FR200-collapse-exit");
                    key.model.extend_from_slice(&row_index.to_be_bytes());
                    row_index = row_index.saturating_add(1);
                    let action = super::ProductAction::PostState(Box::new(key));
                    collapsed_mecs[collapsed_index]
                        .exits
                        .push((owner, choice_index));
                    for edge in &choice.edges {
                        let mut copy = (*edge).clone();
                        copy.from = ProbStateId(state);
                        copy.to = ProbStateId(state_map[edge.to.index()]);
                        copy.choice = action.clone();
                        copy.action = None;
                        edges.push(copy);
                    }
                }
            }
            if collapsed_mecs[collapsed_index].exits.is_empty() {
                collapsed_target[state] = true;
                edges.push(stutter_edge(ProbStateId(state)));
            }
            continue;
        }
        for edge in product.outgoing(ProbStateId(state)) {
            let mut copy = edge.clone();
            copy.to = ProbStateId(state_map[edge.to.index()]);
            edges.push(copy);
        }
    }
    let initial = product
        .initial
        .iter()
        .map(|(binding, instance, state)| {
            (
                *binding,
                instance.clone(),
                ProbStateId(state_map[state.index()]),
            )
        })
        .collect();
    Ok((
        ProbProduct {
            states: product.states.clone(),
            kind: product.kind,
            edges,
            initial,
            decided: product.decided.clone(),
            labels: product.labels.clone(),
            fairness_operations: product.fairness_operations.clone(),
            fairness: product.fairness.clone(),
        },
        collapsed_target,
        state_map,
        collapsed_mecs,
    ))
}

fn stutter_edge(state: ProbStateId) -> super::ProbEdge {
    super::ProbEdge {
        from: state,
        choice: super::ProductAction::TerminalStutter,
        action: None,
        draw: None,
        to: state,
        probability: one(),
        rewards: Vec::new(),
    }
}

fn absorbing_target_product(product: &ProbProduct, target: &[bool]) -> ProbProduct {
    let mut edges = Vec::new();
    for state in product.ids() {
        if target[state.index()] {
            edges.push(stutter_edge(state));
        } else {
            edges.extend(product.outgoing(state).iter().cloned());
        }
    }
    ProbProduct {
        states: product.states.clone(),
        kind: product.kind,
        edges,
        initial: product.initial.clone(),
        decided: product.decided.clone(),
        labels: product.labels.clone(),
        fairness_operations: product.fairness_operations.clone(),
        fairness: product.fairness.clone(),
    }
}

fn expand_collapsed_policy(
    graph: &Graph<'_>,
    collapsed_policy: &[Option<usize>],
    state_map: &[usize],
    collapsed_mecs: &[CollapsedMec],
) -> Result<Vec<Option<usize>>, UnboundedError> {
    let mut policy = vec![None; graph.product.states.len()];
    let reps = collapsed_mecs
        .iter()
        .map(|collapsed| collapsed.representative)
        .collect::<BTreeSet<_>>();
    for state in 0..policy.len() {
        if state_map[state] == state && !reps.contains(&state) {
            policy[state] = collapsed_policy[state];
        }
    }
    for collapsed in collapsed_mecs {
        if collapsed.exits.is_empty() {
            for state in &collapsed.members {
                policy[*state] = graph.choices[*state].first().map(|_| 0);
            }
            continue;
        }
        let action =
            collapsed_policy[collapsed.representative].ok_or(UnboundedError::MissingAction)?;
        let (owner, owner_action) = *collapsed
            .exits
            .get(action)
            .ok_or(UnboundedError::InvalidState)?;
        policy_route_to_exit(graph, collapsed, owner, owner_action, &mut policy)?;
    }
    Ok(policy)
}

fn policy_route_to_exit(
    graph: &Graph<'_>,
    collapsed: &CollapsedMec,
    owner: usize,
    exit_action: usize,
    policy: &mut [Option<usize>],
) -> Result<(), UnboundedError> {
    let mut distance = vec![None; graph.product.states.len()];
    distance[owner] = Some(0_usize);
    loop {
        let mut changed = false;
        for state in collapsed.members.iter().copied() {
            if distance[state].is_some() {
                continue;
            }
            let nearest = graph.choices[state]
                .iter()
                .filter(|choice| {
                    choice
                        .edges
                        .iter()
                        .all(|edge| collapsed.members.contains(&edge.to.index()))
                })
                .flat_map(|choice| choice.edges.iter())
                .filter_map(|edge| distance[edge.to.index()])
                .min();
            if let Some(nearest) = nearest {
                distance[state] = Some(nearest.saturating_add(1));
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    if collapsed
        .members
        .iter()
        .any(|state| distance[*state].is_none())
    {
        return Err(UnboundedError::InvalidState);
    }
    policy[owner] = Some(exit_action);
    for state in collapsed
        .members
        .iter()
        .copied()
        .filter(|state| *state != owner)
    {
        let rank = distance[state].ok_or(UnboundedError::InvalidState)?;
        let action = graph.choices[state]
            .iter()
            .enumerate()
            .find(|(_, choice)| {
                choice
                    .edges
                    .iter()
                    .all(|edge| collapsed.members.contains(&edge.to.index()))
                    && choice
                        .edges
                        .iter()
                        .any(|edge| distance[edge.to.index()].is_some_and(|next| next < rank))
            })
            .map(|(index, _)| index)
            .ok_or(UnboundedError::MissingAction)?;
        policy[state] = Some(action);
    }
    Ok(())
}

fn deterministic_choices(policy: &[Option<usize>]) -> Vec<Option<SchedulerChoice>> {
    policy
        .iter()
        .map(|choice| choice.map(|index| SchedulerChoice::Deterministic(ActionChoice(index))))
        .collect()
}

fn edge_reward_positive(edge: &super::ProbEdge, reward: &str) -> bool {
    edge.rewards
        .iter()
        .find(|(name, _)| name.as_str() == reward)
        .is_some_and(|(_, value)| match value {
            quire_exact::Value::Integer(value) => value > &Integer::from(0_i64),
            quire_exact::Value::Quantity(value) => value.magnitude() > &zero(),
            _ => false,
        })
}

fn randomized_fair_policy(
    graph: &Graph<'_>,
    components: &[FairEndComponent],
    deterministic: &[Option<usize>],
    k: u64,
) -> Vec<Option<SchedulerChoice>> {
    let epsilon = one().divided_by_power_of_two(k);
    let mut policy = deterministic
        .iter()
        .map(|choice| choice.map(|index| SchedulerChoice::Deterministic(ActionChoice(index))))
        .collect::<Vec<_>>();
    for component in components {
        for state in &component.states {
            let at = state.index();
            let members = component.states.iter().copied().collect::<BTreeSet<_>>();
            let internal = graph.choices[at]
                .iter()
                .enumerate()
                .filter(|(_, choice)| choice.edges.iter().all(|edge| members.contains(&edge.to)))
                .map(|(index, _)| index)
                .collect::<Vec<_>>();
            if internal.len() <= 1 {
                continue;
            }
            let preferred = deterministic[at]
                .filter(|choice| internal.contains(choice))
                .unwrap_or(internal[0]);
            let others = internal
                .iter()
                .copied()
                .filter(|choice| *choice != preferred)
                .collect::<Vec<_>>();
            let other_probability = epsilon
                .clone()
                .div(&Rational::from_integer(Integer::from(others.len() as u64)))
                .expect("a nonempty omitted-action set");
            let preferred_probability = one().sub(&epsilon);
            let mut randomized = vec![(ActionChoice(preferred), preferred_probability)];
            randomized.extend(
                others
                    .into_iter()
                    .map(|choice| (ActionChoice(choice), other_probability.clone())),
            );
            policy[at] = Some(SchedulerChoice::Randomized(randomized));
        }
    }
    policy
}

fn induced_chain(
    product: &ProbProduct,
    graph: &Graph<'_>,
    policy: &[Option<SchedulerChoice>],
) -> Result<ProbProduct, UnboundedError> {
    let mut edges = Vec::new();
    for (state, choices) in graph.choices.iter().enumerate() {
        let selected = policy
            .get(state)
            .and_then(Option::as_ref)
            .ok_or(UnboundedError::MissingAction)?;
        let distributions: Vec<(usize, Rational)> = match selected {
            SchedulerChoice::Deterministic(ActionChoice(choice)) => vec![(*choice, one())],
            SchedulerChoice::Randomized(distribution) => distribution
                .iter()
                .map(|(choice, probability)| (choice.0, probability.clone()))
                .collect(),
        };
        for (choice_index, scheduler_probability) in distributions {
            let choice = choices
                .get(choice_index)
                .ok_or(UnboundedError::InvalidState)?;
            for edge in &choice.edges {
                let mut edge = (*edge).clone();
                edge.choice = super::ProductAction::TerminalStutter;
                edge.action = None;
                edge.probability = edge.probability.mul(&scheduler_probability);
                edges.push(edge);
            }
        }
    }
    Ok(ProbProduct {
        states: product.states.clone(),
        kind: ProductKind::Dtmc,
        edges,
        initial: product.initial.clone(),
        decided: product.decided.clone(),
        labels: product.labels.clone(),
        fairness_operations: Vec::new(),
        fairness: Vec::new(),
    })
}

fn bound_refuted(
    product: &ProbProduct,
    values: &[ExtRational],
    objective: UnboundedObjective,
    threshold: &Rational,
) -> bool {
    let mut initial = product
        .initial
        .iter()
        .map(|(_, _, state)| state.index())
        .collect::<Vec<_>>();
    if initial.is_empty() {
        initial.extend(0..values.len());
    }
    initial.into_iter().any(|state| {
        matches!(values.get(state), Some(ExtRational::Finite(value)) if match objective {
            UnboundedObjective::Min => value < threshold,
            UnboundedObjective::Max => value > threshold,
        })
    })
}

fn graph_restricted_to<'a>(graph: &Graph<'a>, allowed: &[bool]) -> Graph<'a> {
    let choices = graph
        .choices
        .iter()
        .enumerate()
        .map(|(state, choices)| {
            if !allowed[state] {
                return Vec::new();
            }
            choices
                .iter()
                .filter(|choice| choice.edges.iter().all(|edge| allowed[edge.to.index()]))
                .cloned()
                .collect()
        })
        .collect();
    Graph {
        product: graph.product,
        choices,
    }
}

fn fair_witness_policy(
    product: &ProbProduct,
    graph: &Graph<'_>,
    components: &[FairEndComponent],
    values: &[ExtRational],
    optimizing_policy: Vec<Option<usize>>,
    objective: UnboundedObjective,
) -> Vec<Option<SchedulerChoice>> {
    let mut result = vec![None; product.states.len()];
    let mut in_fair_component = vec![None; product.states.len()];
    for (component_index, component) in components.iter().enumerate() {
        for state in &component.states {
            in_fair_component[state.index()] = Some(component_index);
        }
    }
    for (state, choices) in graph.choices.iter().enumerate() {
        if choices.is_empty() {
            continue;
        }
        if objective == UnboundedObjective::Max
            && matches!(values.get(state), Some(ExtRational::Finite(value)) if value > &zero())
        {
            if let Some(choice) = optimizing_policy[state] {
                result[state] = Some(SchedulerChoice::Deterministic(ActionChoice(choice)));
                continue;
            }
        }
        if let Some(component_index) = in_fair_component[state] {
            let component = &components[component_index];
            let internal = choices
                .iter()
                .enumerate()
                .filter(|(_, choice)| {
                    choice
                        .edges
                        .iter()
                        .all(|edge| component.states.contains(&edge.to))
                })
                .map(|(index, _)| index)
                .collect::<Vec<_>>();
            if !internal.is_empty() {
                result[state] = Some(uniform(internal));
                continue;
            }
        }
        if matches!(values.get(state), Some(ExtRational::Finite(value)) if value > &zero()) {
            if let Some(choice) = optimizing_policy[state] {
                result[state] = Some(SchedulerChoice::Deterministic(ActionChoice(choice)));
                continue;
            }
        }
        result[state] = Some(uniform((0..choices.len()).collect()));
    }
    result
}

fn validate_fair_witness(
    product: &ProbProduct,
    graph: &Graph<'_>,
    policy: &[Option<SchedulerChoice>],
    fairness: &[FairnessConstraint],
) -> Result<(), FairProductError> {
    let actions = fairness_actions(product, fairness)?;
    let mut adjacency = vec![Vec::new(); product.states.len()];
    let mut selected_by_state = vec![Vec::<usize>::new(); product.states.len()];
    for state in 0..product.states.len() {
        let choices = &graph.choices[state];
        if choices.is_empty() {
            continue;
        }
        let selected = policy
            .get(state)
            .and_then(Option::as_ref)
            .ok_or(FairProductError::UnfairWitness)?;
        for choice in scheduler_indices(selected) {
            let Some(group) = choices.get(choice) else {
                return Err(FairProductError::UnfairWitness);
            };
            selected_by_state[state].push(choice);
            adjacency[state].extend(group.edges.iter().map(|edge| edge.to.index()));
        }
        adjacency[state].sort_unstable();
        adjacency[state].dedup();
    }
    for component in bottom_components(&adjacency) {
        let members = component.iter().copied().collect::<BTreeSet<_>>();
        for constraint in &actions {
            let enabled = component
                .iter()
                .map(|state| {
                    graph.choices[*state]
                        .iter()
                        .any(|choice| choice_matches(choice, constraint))
                })
                .collect::<Vec<_>>();
            let required = match constraint.kind {
                FairnessKind::Weak => enabled.iter().all(|value| *value),
                FairnessKind::Strong => enabled.iter().any(|value| *value),
            };
            if !required {
                continue;
            }
            let taken = component.iter().any(|state| {
                selected_by_state[*state].iter().any(|index| {
                    graph.choices[*state].get(*index).is_some_and(|choice| {
                        choice_matches(choice, constraint)
                            && choice
                                .edges
                                .iter()
                                .all(|edge| members.contains(&edge.to.index()))
                    })
                })
            });
            if !taken {
                return Err(FairProductError::UnfairWitness);
            }
        }
    }
    Ok(())
}

fn scheduler_indices(choice: &SchedulerChoice) -> Vec<usize> {
    match choice {
        SchedulerChoice::Deterministic(ActionChoice(index)) => vec![*index],
        SchedulerChoice::Randomized(choices) => choices
            .iter()
            .filter(|(_, probability)| probability > &zero())
            .map(|(ActionChoice(index), _)| *index)
            .collect(),
    }
}

fn bottom_components(adjacency: &[Vec<usize>]) -> Vec<Vec<usize>> {
    let count = adjacency.len();
    let mut reverse = vec![Vec::new(); count];
    for (from, successors) in adjacency.iter().enumerate() {
        for to in successors {
            reverse[*to].push(from);
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
        visited[root] = true;
        let mut component = Vec::new();
        let mut stack = vec![root];
        while let Some(state) = stack.pop() {
            component.push(state);
            for predecessor in &reverse[state] {
                if !visited[*predecessor] {
                    visited[*predecessor] = true;
                    stack.push(*predecessor);
                }
            }
        }
        if component.iter().all(|state| {
            adjacency[*state]
                .iter()
                .all(|target| component.contains(target))
        }) {
            components.push(component);
        }
    }
    components
}

fn uniform(choices: Vec<usize>) -> SchedulerChoice {
    let probability = Rational::new(Integer::from(1_i64), Integer::from(choices.len() as u64))
        .expect("a nonempty action set has a positive denominator");
    SchedulerChoice::Randomized(
        choices
            .into_iter()
            .map(|choice| (ActionChoice(choice), probability.clone()))
            .collect(),
    )
}

fn zero() -> Rational {
    Rational::from_integer(Integer::from(0_i64))
}

fn one() -> Rational {
    Rational::from_integer(Integer::from(1_i64))
}

fn choice_matches(choice: &Choice<'_>, constraint: &FairAction<'_>) -> bool {
    choice
        .edges
        .first()
        .and_then(|edge| edge.action.as_ref())
        .is_some_and(|identity| {
            identity.operation == constraint.operation
                && (constraint.whole || constraint.identity == Some(identity))
        })
}

fn candidate_position(candidate: &BTreeSet<usize>, state: usize) -> usize {
    candidate.range(..state).count()
}

fn mecs_in(graph: &Graph<'_>, states: BTreeSet<usize>) -> Result<Vec<Vec<usize>>, UnboundedError> {
    let count = graph.product.states.len();
    let choices = graph
        .choices
        .iter()
        .enumerate()
        .map(|(state, choices)| {
            if !states.contains(&state) {
                return Vec::new();
            }
            choices
                .iter()
                .filter(|choice| {
                    choice
                        .edges
                        .iter()
                        .all(|edge| states.contains(&edge.to.index()))
                })
                .cloned()
                .collect()
        })
        .collect::<Vec<Vec<Choice<'_>>>>();
    let induced = Graph {
        product: graph.product,
        choices,
    };
    debug_assert_eq!(induced.choices.len(), count);
    maximal_end_components_from(&induced, states)
}
