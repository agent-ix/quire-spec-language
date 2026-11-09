// SPDX-License-Identifier: AGPL-3.0-or-later
//! Direct infinite-trace labels on a checker-owned acyclic model graph.

use super::*;
use qsl_eval::simulation::model::PredicateReading;

#[allow(clippy::too_many_arguments)]
pub(super) fn enumerate(
    system: &ModelSystem<'_>,
    target: &ReachTarget,
    binding: Option<&ObjectReference>,
    workload: Option<&Workload>,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
    failure: &mut impl FnMut(crate::ExactProbBuildIssue),
) -> Result<
    (
        ProbabilityStateEvent,
        CheckerGraph,
        Vec<ProbabilityProductKey>,
    ),
    ProbabilityDocumentError,
> {
    let (event, predicate, from) = match target {
        ReachTarget::Until(from, target) => (ProbabilityStateEvent::Until, target, Some(from)),
        ReachTarget::Always(predicate) => (ProbabilityStateEvent::Always, predicate, None),
        ReachTarget::Eventually(_) => return Err(ProbabilityDocumentError::Unsupported),
    };
    let mut from_labels = BTreeMap::new();
    let mut predicate_error = None;
    let mut graph = enumerate_model_graph(
        system,
        predicate,
        binding,
        workload,
        limits.states,
        limits.edges,
        cancel,
        false,
        &mut |key, state| {
            if let Some(from) = from {
                let reading = system
                    .state_predicates(state, &[(from, binding)])
                    .map_err(crate::ExactProbBuildIssue::Expansion)
                    .and_then(|readings| match readings.into_iter().next() {
                        Some(PredicateReading::True) => Ok(true),
                        Some(PredicateReading::False) => Ok(false),
                        Some(PredicateReading::Refused(outcome)) => {
                            Err(crate::ExactProbBuildIssue::Predicate(outcome))
                        }
                        _ => Err(crate::ExactProbBuildIssue::UndecidedSuccessor),
                    });
                match reading {
                    Ok(label) => {
                        from_labels.insert(key.clone(), label);
                    }
                    Err(error) => {
                        predicate_error = Some(error);
                    }
                }
            }
        },
        failure,
    )?;
    if let Some(error) = predicate_error {
        return Err(ProbabilityDocumentError::Build(Box::new(error)));
    }
    // Admission is about the original graph, even beyond an initial boundary.
    topological_order(&graph, cancel)?;
    let mut pending = VecDeque::from([graph.initial.clone()]);
    let mut stopped = BTreeMap::new();
    let mut failures = Vec::new();
    while let Some(key) = pending.pop_front() {
        poll_cancel(cancel)?;
        if stopped.contains_key(&key) {
            continue;
        }
        let mut state = graph
            .states
            .remove(&key)
            .ok_or(ProbabilityDocumentError::Malformed)?;
        let predicate = state.terminal == Some(true);
        let terminal = state.choices.is_empty();
        let (success, failure) = match event {
            ProbabilityStateEvent::Until => (
                predicate,
                !predicate
                    && !from_labels
                        .get(&key)
                        .copied()
                        .ok_or(ProbabilityDocumentError::Malformed)?,
            ),
            ProbabilityStateEvent::Always => (predicate && terminal, !predicate),
        };
        state.terminal = if success {
            Some(true)
        } else if failure || terminal {
            Some(false)
        } else {
            None
        };
        if failure {
            failures.push(key.clone());
        }
        if state.terminal.is_some() {
            state.choices.clear();
        } else {
            for edge in state.choices.iter().flatten() {
                pending.push_back(edge.target.clone());
            }
        }
        stopped.insert(key, state);
    }
    failures.sort();
    graph.states = stopped;
    Ok((event, graph, failures))
}
