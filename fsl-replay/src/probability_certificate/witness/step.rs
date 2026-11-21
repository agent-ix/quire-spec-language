// SPDX-License-Identifier: AGPL-3.0-or-later
//! Semantic step replay shared by finite path sets and undefined paths.
use super::*;

/// Preserve the entry point's established settlement of a semantic action refusal.
#[derive(Clone, Copy)]
pub(super) enum ActionRefusal {
    ExpansionRefused,
    UnsupportedDraw,
    /// Measure replay has independently checked the pending post-state policy.
    CheckedResidual,
}
impl ActionRefusal {
    fn error(self) -> ExactProbabilityGraphError {
        match self {
            Self::ExpansionRefused => ExactProbabilityGraphError::ExpansionRefused,
            Self::UnsupportedDraw | Self::CheckedResidual => {
                ExactProbabilityGraphError::UnsupportedDraw
            }
        }
    }
}

pub(super) fn scheduler<'a>(
    rows: &'a [(ProbabilityProductKey, ProductAction)],
    limits: &ExactProbLimits,
    meter: &mut filament_exact::Meter,
    storage: &mut u64,
    cancel: &Cancel,
) -> Result<BTreeMap<&'a ProbabilityProductKey, &'a ProductAction>, ProbabilityWitnessError> {
    if rows.len() > limits.model.limits.max_states {
        return Err(ExactProbabilityGraphError::LimitReached.into());
    }
    let mut parsed = BTreeMap::new();
    let mut previous = None;
    for (key, action) in rows {
        bounded::visit(meter, cancel)?;
        if previous.is_some_and(|previous| previous >= key) {
            return Err(ProbabilityWitnessError::Refused);
        }
        previous = Some(key);
        bounded::retain(
            key.as_bytes().len().saturating_add(
                action
                    .to_bytes()
                    .map_err(|_| ProbabilityWitnessError::Refused)?
                    .len(),
            ),
            storage,
            limits,
        )?;
        parsed.insert(key, action);
    }
    Ok(parsed)
}

#[allow(clippy::too_many_arguments)]
pub(super) fn replay<T>(
    system: &ModelSystem<'_>,
    state: &ModelSimState,
    workload: Option<&Workload>,
    step: &ProbabilityWitnessStep,
    limits: &ExactProbLimits,
    meter: &mut filament_exact::Meter,
    storage: &mut u64,
    edges: &mut usize,
    cancel: &Cancel,
    action_refusal: ActionRefusal,
    mut project: impl FnMut(
        &ModelSimState,
        &[(filament_exact::Identifier, SemanticValue)],
        &mut filament_exact::Meter,
        &mut u64,
    ) -> Result<Option<T>, ProbabilityWitnessError>,
) -> Result<(Rational, ModelSimState, T), ProbabilityWitnessError> {
    if matches!(step.choice, ProductAction::TerminalStutter) {
        if step.action.is_some()
            || step.transition.is_some()
            || step.draw.is_some()
            || step.elapsed != rational(0)
        {
            return Err(ProbabilityWitnessError::Refused);
        }
        let mut issue = None;
        let distributions = model_distributions(system, state, workload, cancel, &mut |failure| {
            issue = Some(failure)
        })
        .map_err(|error| {
            issue.map_or(ProbabilityWitnessError::Graph(error), |issue| {
                ProbabilityWitnessError::Build(Box::new(issue))
            })
        })?;
        if !distributions.is_empty()
            || system.state_key(state).ok().map(|(bytes, _)| bytes)
                != system.state_key(&step.post).ok().map(|(bytes, _)| bytes)
        {
            return Err(ProbabilityWitnessError::Refused);
        }
        let rewards = system.zero_rewards();
        let Some(projected) = project(state, &rewards, meter, storage)? else {
            return Err(ProbabilityWitnessError::Refused);
        };
        *edges = edges
            .checked_add(1)
            .ok_or(ExactProbabilityGraphError::TooManyEdges)?;
        if *edges > limits.model.limits.max_transitions {
            return Err(ExactProbabilityGraphError::EdgeLimitReached.into());
        }
        return Ok((rational(1), state.clone(), projected));
    }
    let ProductAction::Identity(schedule) = &step.choice else {
        return Err(ProbabilityWitnessError::Refused);
    };
    let (Some(action), Some(transition), Some(draw)) = (&step.action, &step.transition, &step.draw)
    else {
        return Err(ProbabilityWitnessError::Refused);
    };
    if step.elapsed != rational(0) || crate::Schedule::of(system, action).as_ref() != Ok(schedule) {
        return Err(ProbabilityWitnessError::Refused);
    }
    if !matches!(action_refusal, ActionRefusal::CheckedResidual) {
        let mut issue = None;
        model_distributions(system, state, workload, cancel, &mut |failure| {
            issue = Some(failure)
        })
        .map_err(|error| {
            issue.map_or(ProbabilityWitnessError::Graph(error), |issue| {
                ProbabilityWitnessError::Build(Box::new(issue))
            })
        })?;
    }
    let mut candidates = Vec::new();
    if let Some(workload) = workload {
        for candidate in system
            .weighted_steps_with_cancel(state, workload, cancel)
            .map_err(|stop| cancelled_expansion(stop, cancel))?
            .map_err(|_| ExactProbabilityGraphError::UnsupportedDraw)?
        {
            if candidate.identity.as_ref() == Some(action)
                && candidate.transition == TransitionId::Operation(transition.clone())
                && same_values(&candidate.random, draw)
            {
                candidates.push((candidate.probability, candidate.post, candidate.rewards));
            }
        }
    } else {
        for candidate in system
            .actions_with_cancel(state, cancel)
            .map_err(|stop| cancelled_expansion(stop, cancel))?
            .map_err(|_| action_refusal.error())?
        {
            if candidate.identity != *action {
                continue;
            }
            for candidate_draw in candidate.draws {
                if !same_values(&candidate_draw.random, draw) {
                    continue;
                }
                for (candidate_transition, post) in candidate_draw.successors {
                    if candidate_transition == *transition {
                        candidates.push((
                            candidate_draw.probability.clone(),
                            post,
                            candidate_draw.rewards.clone(),
                        ));
                    }
                }
            }
        }
    }
    let mut matched = None;
    for (probability, post, rewards) in candidates {
        bounded::visit(meter, cancel)?;
        if post.anchor() != step.post.anchor() || post.reached_by() != step.post.reached_by() {
            continue;
        }
        if let Some(projected) = project(&post, &rewards, meter, storage)? {
            if matched.is_some() {
                return Err(ExactProbabilityGraphError::UnsupportedDraw.into());
            }
            matched = Some((probability, post, projected));
        }
    }
    let Some((probability, post, projected)) = matched else {
        return Err(ProbabilityWitnessError::Refused);
    };
    if probability <= rational(0) {
        return Err(ProbabilityWitnessError::Refused);
    }
    check_bits(&probability, limits.max_rational_bits)?;
    *edges = edges
        .checked_add(1)
        .ok_or(ExactProbabilityGraphError::TooManyEdges)?;
    if *edges > limits.model.limits.max_transitions {
        return Err(ExactProbabilityGraphError::EdgeLimitReached.into());
    }
    let action_bytes = system
        .scheduled_identity_key(action)
        .map_err(|stop| cancelled_expansion(stop, cancel))?;
    let transition_bytes = system
        .transition_key(transition)
        .map_err(|_| ExactProbabilityGraphError::ExpansionRefused)?;
    bounded::retain(
        action_bytes
            .len()
            .saturating_add(transition_bytes.len().saturating_mul(2))
            .saturating_add(std::mem::size_of::<ProbabilityWitnessStep>()),
        storage,
        limits,
    )?;
    Ok((probability, post, projected))
}
