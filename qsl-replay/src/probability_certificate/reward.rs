// SPDX-License-Identifier: AGPL-3.0-or-later
//! Finite acyclic named-reward admission and independently evaluated costs.

use super::*;
use qsl_semantics::check::CheckedPredicate;
use quire_exact::{Identifier, Value as ExactValue, ValueType};

pub(super) struct RewardGraph {
    pub(super) objective: ProbabilityRewardObjective,
    pub(super) costs: BTreeMap<ProbabilityProductKey, Vec<Vec<Rational>>>,
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
) -> Result<
    (
        CheckerGraph,
        BTreeMap<ProbabilityProductKey, u64>,
        RewardGraph,
    ),
    ProbabilityDocumentError,
> {
    poll_cancel(cancel)?;
    let declarations = system.package().graph().probabilistic_model();
    let declared = declarations
        .rewards()
        .iter()
        .find(|reward| {
            reward.name() == name.as_str() && reward.operation().model() == claim.subject.alias
        })
        .ok_or(ProbabilityDocumentError::Unsupported)?;
    let storage = std::cell::RefCell::new(0_u64);
    charge(
        &mut storage.borrow_mut(),
        limits.storage_bytes,
        name.as_str().len() as u64,
    )?;
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
                        &mut storage.borrow_mut(),
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
    let objective = ProbabilityRewardObjective {
        model,
        name: name.clone(),
        unit,
    };
    let mut costs = BTreeMap::new();
    let failure = std::cell::RefCell::new(None);
    let mut issue = None;
    let mut remaining_edges = limits.edges;
    let key_of = |state: &ModelSimState| {
        let (bytes, _) = system
            .state_key(state)
            .map_err(|_| ExactProbabilityGraphError::ExpansionRefused)?;
        ProbabilityProductKey::for_model_state_with_limits(
            binding,
            &bytes,
            ProbabilityDocumentIoLimits {
                bytes: limits.bytes,
                storage_bytes: limits.storage_bytes,
            },
            cancel,
        )
        .map_err(|error| {
            *failure.borrow_mut() = Some(error);
            ExactProbabilityGraphError::ExpansionRefused
        })
    };
    let result = enumerate_model_graph_with_keys_and_reservation(
        system,
        Some(until),
        binding,
        workload,
        limits.states,
        limits.edges,
        cancel,
        false,
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
        #[cfg(feature = "test-support")]
        &mut |_| {},
        &mut |key, state| {
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
                    &mut storage,
                    &mut remaining_edges,
                )
            });
            match result {
                Ok(rows) => {
                    costs.insert(key.clone(), rows);
                    Ok(())
                }
                Err(error) => {
                    *failure.borrow_mut() = Some(error);
                    Err(ExactProbabilityGraphError::ExpansionRefused)
                }
            }
        },
        &mut |failure| issue = Some(failure),
    );
    if let Some(error) = failure.into_inner() {
        return Err(error);
    }
    if let Some(issue) = issue {
        return Err(ProbabilityDocumentError::Build(Box::new(issue)));
    }
    let mut graph = result?;
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

#[allow(clippy::too_many_arguments)]
fn step_costs(
    system: &ModelSystem<'_>,
    state: &ModelSimState,
    objective: &ProbabilityRewardObjective,
    workload: Option<&Workload>,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
    storage: &mut u64,
    remaining_edges: &mut usize,
) -> Result<Vec<Vec<Rational>>, ProbabilityDocumentError> {
    use qsl_eval::simulation::model::{TransitionId, Unsteppable};
    let expansion = |stop| {
        ProbabilityDocumentError::Build(Box::new(crate::ExactProbBuildIssue::Expansion(stop)))
    };
    let unsteppable = |stop| {
        ProbabilityDocumentError::Build(Box::new(match stop {
            Unsteppable::NotMarkov(_) => crate::ExactProbBuildIssue::NotMarkov,
            Unsteppable::UndecidedSuccessor(_) => crate::ExactProbBuildIssue::UndecidedSuccessor,
        }))
    };
    let magnitude =
        |rewards: &[(Identifier, ExactValue)], storage: &mut u64, remaining: &mut usize| {
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
        };
    if let Some(workload) = workload {
        let result = system.weighted_steps_with_cancel(state, workload, cancel);
        poll_cancel(cancel)?;
        let steps = result.map_err(expansion)?.map_err(unsteppable)?;
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
                let value = magnitude(&step.rewards, storage, remaining_edges)?;
                costs
                    .try_reserve_exact(1)
                    .map_err(|_| ProbabilityDocumentError::Malformed)?;
                costs.push(value);
            }
        }
        return Ok(vec![costs]);
    }
    let result = system.actions_with_cancel(state, cancel);
    poll_cancel(cancel)?;
    let actions = result.map_err(expansion)?.map_err(|_| {
        ProbabilityDocumentError::Build(Box::new(crate::ExactProbBuildIssue::UndecidedSuccessor))
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
            let value = magnitude(&draw.rewards, storage, remaining_edges)?;
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
