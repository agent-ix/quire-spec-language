// SPDX-License-Identifier: AGPL-3.0-or-later
//! Named-reward admission, first-target graphs and independently evaluated costs.

use super::*;
use qsl_semantics::check::CheckedPredicate;
use quire_exact::{Identifier, Value as ExactValue, ValueType};

#[derive(Clone, Copy, Eq, PartialEq)]
pub(super) enum RewardProfile {
    Acyclic,
    CyclicDtmc,
}

pub(super) struct RewardGraph {
    pub(super) objective: ProbabilityRewardObjective,
    pub(super) costs: BTreeMap<ProbabilityProductKey, Vec<Vec<Rational>>>,
}

/// Resolve the actual checked model reward and selected numeric unit.
pub(super) fn objective(
    system: &ModelSystem<'_>,
    claim: &CheckedProbabilisticClaim,
    name: &Identifier,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
    storage: &mut u64,
) -> Result<ProbabilityRewardObjective, ProbabilityDocumentError> {
    let declarations = system.package().graph().probabilistic_model();
    let declared = declarations
        .rewards()
        .iter()
        .find(|reward| {
            reward.name() == name.as_str() && reward.operation().model() == claim.subject.alias
        })
        .ok_or(ProbabilityDocumentError::Unsupported)?;
    charge(storage, limits.storage_bytes, name.as_str().len() as u64)?;
    let unit = match declared.value_type() {
        ValueType::Integer | ValueType::Int(_) => None,
        ValueType::Quantity(unit) => {
            if let Some(terms) = unit.compound_terms() {
                for (_, exponent) in terms {
                    poll_cancel(cancel)?;
                    let bits = exponent.magnitude_bits();
                    if bits > limits.scalar_bits {
                        return Err(ProbabilityDocumentError::Limit {
                            limit: ProbabilityDocumentLimit::RationalBits,
                            bound: limits.scalar_bits,
                            actual: bits,
                        });
                    }
                    charge(
                        storage,
                        limits.storage_bytes,
                        (std::mem::size_of::<(quire_exact::NodeKey, Integer)>() as u64)
                            .saturating_add(bits.div_ceil(8)),
                    )?;
                }
            }
            Some(unit.clone())
        }
        _ => return Err(ProbabilityDocumentError::Unsupported),
    };
    let model = system
        .package()
        .graph()
        .model_roots()
        .find(|(selection, _)| **selection == claim.subject.package)
        .map(|(_, node)| node)
        .ok_or(ProbabilityDocumentError::Unsupported)?;
    Ok(ProbabilityRewardObjective {
        model,
        name: name.clone(),
        unit,
    })
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub(super) fn enumerate(
    system: &ModelSystem<'_>,
    claim: &CheckedProbabilisticClaim,
    name: &Identifier,
    until: &CheckedPredicate,
    binding: Option<&ObjectReference>,
    workload: Option<&Workload>,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
    identity: qsl_foundation::IdentityLimits,
    profile: RewardProfile,
    meter: &mut quire_exact::Meter,
    used: &mut u64,
) -> Result<
    (
        CheckerGraph,
        BTreeMap<ProbabilityProductKey, u64>,
        RewardGraph,
    ),
    ProbabilityDocumentError,
> {
    poll_cancel(cancel)?;
    let storage = std::cell::RefCell::new(used);
    let objective = objective(
        system,
        claim,
        name,
        limits,
        cancel,
        &mut storage.borrow_mut(),
    )?;
    let costs = std::cell::RefCell::new(BTreeMap::new());
    let failure = std::cell::RefCell::new(None);
    let issue = std::cell::RefCell::new(None);
    let mut remaining_edges = limits.edges;
    let meter = std::cell::RefCell::new(meter);
    let key_of = |state: &ModelSimState| {
        let mut meter = meter.borrow_mut();
        let mut storage = storage.borrow_mut();
        let mut admission =
            ProbabilityKeyAdmission::new(&mut meter, &mut storage, limits.storage_bytes, cancel);
        let encoded = (|| {
            let bytes =
                system.state_key_bytes_with_budget(state, identity, &mut admission, cancel)?;
            ProbabilityProductKey::for_model_state_with_budget(
                binding,
                &bytes,
                identity,
                &mut admission,
                cancel,
            )
        })();
        encoded
            .map_err(|error| admission.refusal(error))
            .map_err(|error| {
                *failure.borrow_mut() = Some(error);
                ExactProbabilityGraphError::ExpansionRefused
            })
    };
    let collect_costs = std::cell::Cell::new(profile == RewardProfile::Acyclic);
    let explore = |target: Option<&CheckedPredicate>, stop_at_target: bool| {
        enumerate_model_graph_visiting(
            system,
            target,
            binding,
            limits.states,
            limits.edges,
            cancel,
            stop_at_target,
            &key_of,
            &mut |bytes| {
                charge(
                    &mut storage.borrow_mut(),
                    limits.storage_bytes,
                    bytes as u64,
                )
                .map_err(|error| {
                    *failure.borrow_mut() = Some(error);
                    ExactProbabilityGraphError::ExpansionRefused
                })
            },
            &mut |state, _| {
                let mut meter = meter.borrow_mut();
                let mut storage = storage.borrow_mut();
                let mut admission = ProbabilityKeyAdmission::new(
                    &mut meter,
                    &mut storage,
                    limits.storage_bytes,
                    cancel,
                );
                model_distributions_with_budget(
                    system,
                    state,
                    workload,
                    identity,
                    &mut admission,
                    cancel,
                )
                .map_err(|error| {
                    *failure.borrow_mut() = Some(error);
                    ExactProbabilityGraphError::ExpansionRefused
                })
            },
            if profile == RewardProfile::Acyclic {
                ModelVisitPhase::BeforeTarget
            } else {
                ModelVisitPhase::AfterTarget
            },
            &mut |key, state, terminal| {
                if !collect_costs.get()
                    || profile == RewardProfile::CyclicDtmc && terminal == Some(true)
                {
                    return Ok(());
                }
                let mut storage = storage.borrow_mut();
                let result = charge(
                    &mut storage,
                    limits.storage_bytes,
                    (key.as_bytes().len() as u64)
                        .saturating_add(std::mem::size_of::<Vec<Vec<Rational>>>() as u64),
                )
                .and_then(|()| {
                    step_costs(
                        system,
                        state,
                        &objective,
                        workload,
                        limits,
                        cancel,
                        identity,
                        &mut storage,
                        &mut meter.borrow_mut(),
                        &mut remaining_edges,
                    )
                });
                match result {
                    Ok(rows) => {
                        costs.borrow_mut().insert(key.clone(), rows);
                        Ok(())
                    }
                    Err(error) => {
                        *failure.borrow_mut() = Some(error);
                        Err(ExactProbabilityGraphError::ExpansionRefused)
                    }
                }
            },
            &mut |failure| *issue.borrow_mut() = Some(failure),
        )
    };
    if profile == RewardProfile::CyclicDtmc {
        if workload.is_none() {
            return Err(ProbabilityDocumentError::Unsupported);
        }
        // Original model admission does not evaluate the query predicate or
        // accumulated costs. Its full exploration remains caller-controlled.
        let original = finish_graph(explore(None, false), &failure, &issue)?;
        for state in original.states.values() {
            poll_cancel(cancel)?;
            if !state.choices.is_empty() && state.choices.len() != 1 {
                return Err(ProbabilityDocumentError::Unsupported);
            }
        }
    }
    collect_costs.set(true);
    let mut graph = finish_graph(
        explore(Some(until), profile == RewardProfile::CyclicDtmc),
        &failure,
        &issue,
    )?;
    let mut costs = costs.into_inner();
    if profile == RewardProfile::CyclicDtmc {
        let ranks = progress_ranks(
            &graph,
            limits,
            cancel,
            &mut storage.borrow_mut(),
            &mut meter.borrow_mut(),
        )?;
        return Ok((graph, ranks, RewardGraph { objective, costs }));
    }
    // The original graph must terminate even beyond an early target.
    reserve_order(
        &graph,
        &mut storage.borrow_mut(),
        limits.storage_bytes,
        cancel,
    )?;
    topological_order(&graph, cancel)?;
    if graph
        .states
        .values()
        .any(|state| state.choices.is_empty() && state.terminal != Some(true))
    {
        return Err(ProbabilityDocumentError::Unsupported);
    }
    for key in graph.states.keys() {
        poll_cancel(cancel)?;
        let bytes = (key.as_bytes().len() as u64)
            .saturating_mul(2)
            .saturating_add(std::mem::size_of::<ProbabilityProductKey>() as u64)
            .saturating_add(std::mem::size_of::<(ProbabilityProductKey, CheckerState)>() as u64);
        charge(&mut storage.borrow_mut(), limits.storage_bytes, bytes)?;
    }
    let mut pending = VecDeque::from([graph.initial.clone()]);
    let mut queued = BTreeSet::from([graph.initial.clone()]);
    let mut stopped = BTreeMap::new();
    while let Some(key) = pending.pop_front() {
        poll_cancel(cancel)?;
        if stopped.contains_key(&key) {
            continue;
        }
        let mut state = graph
            .states
            .remove(&key)
            .ok_or(ProbabilityDocumentError::Malformed)?;
        if state.terminal == Some(true) {
            state.choices.clear();
            costs.remove(&key);
        } else {
            for edge in state.choices.iter().flatten() {
                if queued.insert(edge.target.clone()) {
                    pending.push_back(edge.target.clone());
                }
            }
        }
        stopped.insert(key, state);
    }
    graph.states = stopped;
    costs.retain(|key, _| graph.states.contains_key(key));
    reserve_order(
        &graph,
        &mut storage.borrow_mut(),
        limits.storage_bytes,
        cancel,
    )?;
    for key in graph.states.keys() {
        poll_cancel(cancel)?;
        charge(
            &mut storage.borrow_mut(),
            limits.storage_bytes,
            (key.as_bytes().len() as u64)
                .saturating_add(std::mem::size_of::<(ProbabilityProductKey, u64)>() as u64),
        )?;
    }
    let mut ranks = BTreeMap::new();
    for key in topological_order(&graph, cancel)?.into_iter().rev() {
        poll_cancel(cancel)?;
        let state = graph
            .states
            .get(&key)
            .ok_or(ProbabilityDocumentError::Malformed)?;
        let mut rank = 0_u64;
        for edge in state.choices.iter().flatten() {
            let successor: &u64 = ranks
                .get(&edge.target)
                .ok_or(ProbabilityDocumentError::Malformed)?;
            rank = rank.max(
                successor
                    .checked_add(1)
                    .filter(|rank| *rank <= 9_007_199_254_740_991)
                    .ok_or(ProbabilityDocumentError::Limit {
                        limit: ProbabilityDocumentLimit::Rank,
                        bound: 9_007_199_254_740_991,
                        actual: u64::MAX,
                    })?,
            );
        }
        ranks.insert(key, rank);
    }
    Ok((graph, ranks, RewardGraph { objective, costs }))
}

fn finish_graph(
    result: Result<CheckerGraph, ExactProbabilityGraphError>,
    failure: &std::cell::RefCell<Option<ProbabilityDocumentError>>,
    issue: &std::cell::RefCell<Option<crate::ExactProbBuildIssue>>,
) -> Result<CheckerGraph, ProbabilityDocumentError> {
    if let Some(error) = failure.borrow_mut().take() {
        return Err(error);
    }
    if let Some(issue) = issue.borrow_mut().take() {
        return Err(ProbabilityDocumentError::Build(Box::new(issue)));
    }
    Ok(result?)
}

/// Derive one finite progress witness without changing graph boundaries or
/// assigning probability-zero values to non-reaching reward states.
fn progress_ranks(
    graph: &CheckerGraph,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
    storage: &mut u64,
    meter: &mut quire_exact::Meter,
) -> Result<BTreeMap<ProbabilityProductKey, u64>, ProbabilityDocumentError> {
    let mut reverse = BTreeMap::<&ProbabilityProductKey, Vec<&ProbabilityProductKey>>::new();
    let mut ranks = BTreeMap::new();
    let mut pending = VecDeque::new();
    for (key, state) in &graph.states {
        visit(cancel, meter)?;
        charge(
            storage,
            limits.storage_bytes,
            (key.as_bytes().len() as u64)
                .saturating_add(std::mem::size_of::<(ProbabilityProductKey, u64)>() as u64),
        )?;
        ranks.insert(key.clone(), 0);
        if state.terminal == Some(true) {
            charge(
                storage,
                limits.storage_bytes,
                std::mem::size_of::<&ProbabilityProductKey>() as u64,
            )?;
            pending
                .try_reserve_exact(1)
                .map_err(|_| ProbabilityDocumentError::Malformed)?;
            pending.push_back(key);
        }
        for edge in state.choices.iter().flatten() {
            visit(cancel, meter)?;
            if edge.probability > rational(0) {
                charge(
                    storage,
                    limits.storage_bytes,
                    (std::mem::size_of::<(&ProbabilityProductKey, Vec<&ProbabilityProductKey>)>()
                        + std::mem::size_of::<&ProbabilityProductKey>()) as u64,
                )?;
                let previous = reverse.entry(&edge.target).or_default();
                previous
                    .try_reserve_exact(1)
                    .map_err(|_| ProbabilityDocumentError::Malformed)?;
                previous.push(key);
            }
        }
    }
    let mut seen = BTreeSet::new();
    while let Some(key) = pending.pop_front() {
        visit(cancel, meter)?;
        let next = ranks
            .get(key)
            .copied()
            .ok_or(ProbabilityDocumentError::Malformed)?
            .checked_add(1)
            .filter(|rank| *rank <= 9_007_199_254_740_991)
            .ok_or(ProbabilityDocumentError::Limit {
                limit: ProbabilityDocumentLimit::Rank,
                bound: 9_007_199_254_740_991,
                actual: u64::MAX,
            })?;
        for previous in reverse.get(key).into_iter().flatten() {
            visit(cancel, meter)?;
            if graph
                .states
                .get(*previous)
                .is_some_and(|state| state.terminal != Some(true))
                && !seen.contains(previous)
            {
                charge(
                    storage,
                    limits.storage_bytes,
                    (2 * std::mem::size_of::<&ProbabilityProductKey>()) as u64,
                )?;
                seen.insert(*previous);
                ranks.insert((*previous).clone(), next);
                pending
                    .try_reserve_exact(1)
                    .map_err(|_| ProbabilityDocumentError::Malformed)?;
                pending.push_back(*previous);
            }
        }
    }
    // Unreachable nodes retain rank zero. The checker rejects their offered
    // finite evidence as RankBroken, while a producer retains core infinity.
    Ok(ranks)
}

fn visit(cancel: &Cancel, meter: &mut quire_exact::Meter) -> Result<(), ProbabilityDocumentError> {
    poll_cancel(cancel)?;
    meter
        .charge(quire_exact::Charge::new(
            quire_exact::ChargePoint::CollectionMemberTest,
        ))
        .map_err(|incomplete| match cancel.tripped() {
            Some(cause) => ProbabilityDocumentError::Cancelled(cause),
            None => ProbabilityDocumentError::Accounting(Box::new(incomplete)),
        })
}

fn reserve_order(
    graph: &CheckerGraph,
    storage: &mut u64,
    bound: u64,
    cancel: &Cancel,
) -> Result<(), ProbabilityDocumentError> {
    for key in graph.states.keys() {
        poll_cancel(cancel)?;
        // Topological traversal retains one indegree, ready-set and output key.
        let bytes = (key.as_bytes().len() as u64)
            .saturating_mul(3)
            .saturating_add(std::mem::size_of::<(ProbabilityProductKey, usize)>() as u64)
            .saturating_add(
                (std::mem::size_of::<ProbabilityProductKey>() as u64).saturating_mul(2),
            );
        charge(storage, bound, bytes)?;
    }
    Ok(())
}

fn charge(storage: &mut u64, bound: u64, bytes: u64) -> Result<(), ProbabilityDocumentError> {
    let actual = storage.saturating_add(bytes);
    if actual > bound {
        return Err(ProbabilityDocumentError::Limit {
            limit: ProbabilityDocumentLimit::StorageBytes,
            bound,
            actual,
        });
    }
    *storage = actual;
    Ok(())
}

/// Evaluate one actual draw's checked numeric magnitude; missing reward is zero.
pub(super) fn magnitude(
    rewards: &[(Identifier, ExactValue)],
    objective: &ProbabilityRewardObjective,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
    storage: &mut u64,
    remaining: &mut usize,
    meter: &mut quire_exact::Meter,
) -> Result<Rational, ProbabilityDocumentError> {
    poll_cancel(cancel)?;
    if *remaining == 0 {
        return Err(ProbabilityDocumentError::Limit {
            limit: ProbabilityDocumentLimit::Edges,
            bound: limits.edges as u64,
            actual: (limits.edges as u64).saturating_add(1),
        });
    }
    *remaining -= 1;
    charge(
        storage,
        limits.storage_bytes,
        std::mem::size_of::<Rational>() as u64,
    )?;
    let Some((_, value)) = rewards.iter().find(|(name, _)| name == &objective.name) else {
        return Ok(rational(0));
    };
    if let ExactValue::Integer(integer) = value {
        meter
            .charge(
                quire_exact::Charge::new(quire_exact::ChargePoint::GraphResultRetain).size(
                    quire_exact::LimitKind::IntegerBits,
                    integer.magnitude_bits(),
                ),
            )
            .map_err(|error| match cancel.tripped() {
                Some(cause) => ProbabilityDocumentError::Cancelled(cause),
                None => ProbabilityDocumentError::Accounting(Box::new(error)),
            })?;
    }
    let (numerator, denominator) = match value {
        ExactValue::Integer(integer) => (integer.magnitude_bits(), 1),
        ExactValue::Quantity(quantity) => (
            quantity.magnitude().numerator().magnitude_bits(),
            quantity.magnitude().denominator().magnitude_bits(),
        ),
        _ => return Err(ProbabilityDocumentError::Unsupported),
    };
    if numerator.max(denominator) > limits.scalar_bits {
        return Err(ProbabilityDocumentError::Limit {
            limit: ProbabilityDocumentLimit::RationalBits,
            bound: limits.scalar_bits,
            actual: numerator.max(denominator),
        });
    }
    charge(
        storage,
        limits.storage_bytes,
        numerator
            .div_ceil(8)
            .saturating_add(denominator.div_ceil(8)),
    )?;
    let value = match (value, &objective.unit) {
        (ExactValue::Integer(integer), None) => Rational::from_integer(integer.clone()),
        (ExactValue::Quantity(quantity), Some(unit)) if quantity.unit() == unit => {
            quantity.magnitude().clone()
        }
        _ => return Err(ProbabilityDocumentError::Unsupported),
    };
    if value < rational(0) {
        return Err(ProbabilityDocumentError::Unsupported);
    }
    super::document::check_scalar(&value, limits)?;
    Ok(value)
}

#[allow(clippy::too_many_arguments)]
fn step_costs(
    system: &ModelSystem<'_>,
    state: &ModelSimState,
    objective: &ProbabilityRewardObjective,
    workload: Option<&Workload>,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
    identity: qsl_foundation::IdentityLimits,
    storage: &mut u64,
    meter: &mut quire_exact::Meter,
    remaining_edges: &mut usize,
) -> Result<Vec<Vec<Rational>>, ProbabilityDocumentError> {
    use qsl_eval::simulation::model::{TransitionId, Unsteppable};
    let unsteppable = |stop| {
        ProbabilityDocumentError::Build(Box::new(match stop {
            Unsteppable::NotMarkov(_) => crate::ExactProbBuildIssue::NotMarkov,
            Unsteppable::UndecidedSuccessor(_) => crate::ExactProbBuildIssue::UndecidedSuccessor,
        }))
    };
    if let Some(workload) = workload {
        let mut admission =
            ProbabilityKeyAdmission::new(meter, storage, limits.storage_bytes, cancel);
        let steps = system
            .weighted_steps_with_budget(state, workload, identity, &mut admission, cancel)
            .map_err(|error| admission.expansion_refusal(error))?
            .map_err(unsteppable)?;
        if steps
            .iter()
            .any(|step| step.transition == TransitionId::Stutter)
        {
            return if steps.len() == 1 {
                Ok(Vec::new())
            } else {
                Err(ProbabilityDocumentError::Unsupported)
            };
        }
        let mut costs = Vec::new();
        charge(
            storage,
            limits.storage_bytes,
            std::mem::size_of::<Vec<Rational>>() as u64,
        )?;
        for step in steps {
            poll_cancel(cancel)?;
            if step.probability != rational(0) {
                let value = magnitude(
                    &step.rewards,
                    objective,
                    limits,
                    cancel,
                    storage,
                    remaining_edges,
                    meter,
                )?;
                costs
                    .try_reserve_exact(1)
                    .map_err(|_| ProbabilityDocumentError::Malformed)?;
                costs.push(value);
            }
        }
        return Ok(vec![costs]);
    }
    let mut admission = ProbabilityKeyAdmission::new(meter, storage, limits.storage_bytes, cancel);
    let actions = system
        .actions_with_budget(state, identity, &mut admission, cancel)
        .map_err(|error| admission.expansion_refusal(error))?
        .map_err(|_| {
            ProbabilityDocumentError::Build(Box::new(
                crate::ExactProbBuildIssue::UndecidedSuccessor,
            ))
        })?;
    let mut rows = Vec::new();
    for action in actions {
        poll_cancel(cancel)?;
        let mut costs = Vec::new();
        charge(
            storage,
            limits.storage_bytes,
            std::mem::size_of::<Vec<Rational>>() as u64,
        )?;
        for draw in action.draws {
            poll_cancel(cancel)?;
            if draw.probability == rational(0) {
                continue;
            }
            if draw.successors.len() != 1 {
                return Err(ProbabilityDocumentError::Unsupported);
            }
            let value = magnitude(
                &draw.rewards,
                objective,
                limits,
                cancel,
                storage,
                remaining_edges,
                meter,
            )?;
            costs
                .try_reserve_exact(1)
                .map_err(|_| ProbabilityDocumentError::Malformed)?;
            costs.push(value);
        }
        rows.try_reserve_exact(1)
            .map_err(|_| ProbabilityDocumentError::Malformed)?;
        rows.push(costs);
    }
    Ok(rows)
}
