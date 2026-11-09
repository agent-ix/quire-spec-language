// SPDX-License-Identifier: AGPL-3.0-or-later
//! Independent bounded product enumeration. Only the monitor's semantic
//! primitives are shared with EN-5; graph construction and arithmetic are not.

use super::*;
use qsl_eval::automaton::{
    AutomatonStateKey, BoundedState, ContextActivation, ContextBinding, ContextRoot,
    QualifiedBoundedMonitor, StateContext, StateId,
};
use qsl_eval::simulation::model::PredicateReading;
use qsl_semantics::check::CheckedTemporalFormula;

/// Derive FR-338's monitor context from the assembled claim and its actual
/// subject binding, retaining the real S3 parameter node and instance key.
pub fn probabilistic_formula_monitor_context(
    claim: &CheckedProbabilisticClaim,
    binding: Option<&ObjectReference>,
) -> Result<StateContext, ExactProbabilityGraphError> {
    let profile = qsl_semantics::check::TemporalProfile::from_identity(claim.profile.identity())
        .ok_or(ExactProbabilityGraphError::UnsupportedClaimForm)?;
    let occurrence = claim
        .occurrence()
        .ok_or(ExactProbabilityGraphError::ClaimNotBound)?;
    let binding = match (&claim.over, binding) {
        (None, None) => Vec::new(),
        (Some(over), Some(reference)) => vec![ContextBinding::instance(
            over.node()
                .ok_or(ExactProbabilityGraphError::ClaimNotBound)?
                .clone(),
            reference.object().as_str(),
        )],
        _ => return Err(ExactProbabilityGraphError::UnsupportedBinding),
    };
    Ok(StateContext {
        root: ContextRoot::Clause(occurrence.clone()),
        profile,
        activation: ContextActivation::Origin,
        binding,
    })
}

/// Encode FR-196's complete formula-bearing product identity. Model and
/// monitor keys are embedded as parsed canonical values, never local IDs.
pub fn bounded_probability_product_key(
    binding: Option<&ObjectReference>,
    model: &[u8],
    monitor: &AutomatonStateKey,
) -> Result<ProbabilityProductKey, ExactProbabilityGraphError> {
    let model: Value =
        serde_json::from_slice(model).map_err(|_| ExactProbabilityGraphError::ExpansionRefused)?;
    let monitor: Value = serde_json::from_slice(monitor.as_bytes())
        .map_err(|_| ExactProbabilityGraphError::ExpansionRefused)?;
    let binding = binding
        .map(|reference| {
            qsl_eval::simulation::model::reference_canonical_text(reference)
                .map_err(|_| ExactProbabilityGraphError::ExpansionRefused)
                .and_then(|text| {
                    serde_json::from_str::<Value>(&text)
                        .map_err(|_| ExactProbabilityGraphError::ExpansionRefused)
                })
        })
        .transpose()?;
    canonical_json(&json!([
        "quire.probability-product-key/v1",
        binding,
        ["quire.simulation.state-key/v1", model],
        ["formula", monitor],
        null,
        null
    ]))
    .map(ProbabilityProductKey)
}

#[derive(Clone, Copy)]
enum Control {
    Running(StateId),
    Decided(bool),
}

impl Control {
    fn read(
        monitor: &QualifiedBoundedMonitor,
        state: StateId,
    ) -> Result<Self, ExactProbabilityGraphError> {
        match monitor
            .state(state)
            .map_err(|_| ExactProbabilityGraphError::ExpansionRefused)?
        {
            BoundedState::Open { .. } => Ok(Self::Running(state)),
            BoundedState::Accept => Ok(Self::Decided(true)),
            BoundedState::Reject => Ok(Self::Decided(false)),
        }
    }

    fn key(
        self,
        monitor: &QualifiedBoundedMonitor,
    ) -> Result<AutomatonStateKey, ExactProbabilityGraphError> {
        match self {
            Self::Running(state) => monitor.key(state),
            Self::Decided(accepted) => monitor.decision_key(accepted),
        }
        .map_err(|_| ExactProbabilityGraphError::ExpansionRefused)
    }

    fn rank(self, monitor: &QualifiedBoundedMonitor) -> Result<u64, ExactProbabilityGraphError> {
        match self {
            Self::Running(state) => monitor
                .rank(state)
                .map_err(|_| ExactProbabilityGraphError::ExpansionRefused),
            Self::Decided(_) => Ok(0),
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn enumerate(
    system: &ModelSystem<'_>,
    formula: &CheckedTemporalFormula,
    context: &StateContext,
    binding: Option<&ObjectReference>,
    workload: Option<&Workload>,
    max_states: usize,
    max_edges: usize,
    cancel: &Cancel,
    failure: &mut impl FnMut(crate::ExactProbBuildIssue),
) -> Result<(CheckerGraph, BTreeMap<ProbabilityProductKey, u64>), ExactProbabilityGraphError> {
    poll_cancel(cancel)?;
    if max_states == 0 {
        return Err(ExactProbabilityGraphError::LimitReached);
    }
    let mut monitor = QualifiedBoundedMonitor::new(
        // The document ceiling counts product states, not internal NFA
        // states. Each newly queued product is checked against it below.
        formula.formula(),
        context,
        u64::MAX,
    )
    .map_err(|_| ExactProbabilityGraphError::UnsupportedClaimForm)?;
    let predicate = formula
        .condition(monitor.atom().index())
        .ok_or(ExactProbabilityGraphError::UnsupportedClaimForm)?;
    let read = |state: &ModelSimState, failure: &mut dyn FnMut(crate::ExactProbBuildIssue)| {
        poll_cancel(cancel)?;
        let reading = system
            .state_predicates(state, &[(predicate, binding)])
            .map_err(|stop| {
                failure(crate::ExactProbBuildIssue::Expansion(stop));
                ExactProbabilityGraphError::ExpansionRefused
            })?
            .into_iter()
            .next()
            .ok_or(ExactProbabilityGraphError::ExpansionRefused)?;
        match reading {
            PredicateReading::True => Ok(true),
            PredicateReading::False => Ok(false),
            PredicateReading::Refused(outcome) => {
                failure(crate::ExactProbBuildIssue::Predicate(outcome));
                Err(ExactProbabilityGraphError::ExpansionRefused)
            }
            _ => Err(ExactProbabilityGraphError::ExpansionRefused),
        }
    };
    let initial = system.initial();
    let [first] = initial.as_slice() else {
        return Err(ExactProbabilityGraphError::UnsupportedInitial);
    };
    let first_control = monitor
        .start(read(first, failure)?)
        .map_err(|_| ExactProbabilityGraphError::ExpansionRefused)?;
    let first_control = Control::read(&monitor, first_control)?;
    let key_of = |model: &ModelSimState, control: Control, monitor: &QualifiedBoundedMonitor| {
        let (bytes, _) = system
            .state_key(model)
            .map_err(|_| ExactProbabilityGraphError::ExpansionRefused)?;
        bounded_probability_product_key(binding, &bytes, &control.key(monitor)?)
    };
    let root = key_of(first, first_control, &monitor)?;
    let mut queue = VecDeque::from([(root.clone(), first.clone(), first_control)]);
    let mut seen = BTreeSet::from([root.clone()]);
    let mut states = BTreeMap::new();
    let mut ranks = BTreeMap::new();
    let mut edge_count = 0usize;
    while let Some((key, model, control)) = queue.pop_front() {
        poll_cancel(cancel)?;
        ranks.insert(key.clone(), control.rank(&monitor)?);
        if let Control::Decided(accepted) = control {
            states.insert(
                key,
                CheckerState {
                    terminal: Some(accepted),
                    choices: Vec::new(),
                },
            );
            continue;
        }
        let Control::Running(running) = control else {
            return Err(ExactProbabilityGraphError::ExpansionRefused);
        };
        let distributions = model_distributions(system, &model, workload, cancel, failure)?;
        let mut choices = Vec::new();
        let mut successors = Vec::new();
        if distributions.is_empty() {
            let terminal = monitor
                .terminal(running)
                .map_err(|_| ExactProbabilityGraphError::ExpansionRefused)?;
            let closed = match terminal {
                BoundedState::Accept => true,
                BoundedState::Reject => false,
                BoundedState::Open { .. } => {
                    return Err(ExactProbabilityGraphError::ExpansionRefused)
                }
            };
            successors.push(vec![(rational(1), model, Control::Decided(closed))]);
        } else {
            for distribution in distributions {
                let mut next = Vec::new();
                for (probability, post) in distribution {
                    poll_cancel(cancel)?;
                    if probability == rational(0) {
                        continue;
                    }
                    let state = monitor
                        .step(running, read(&post, failure)?)
                        .map_err(|_| ExactProbabilityGraphError::ExpansionRefused)?;
                    next.push((probability, post, Control::read(&monitor, state)?));
                }
                successors.push(next);
            }
        }
        for distribution in successors {
            let mut choice = Vec::new();
            for (probability, post, control) in distribution {
                poll_cancel(cancel)?;
                let target = key_of(&post, control, &monitor)?;
                edge_count = edge_count
                    .checked_add(1)
                    .ok_or(ExactProbabilityGraphError::TooManyEdges)?;
                if edge_count > max_edges {
                    return Err(ExactProbabilityGraphError::EdgeLimitReached);
                }
                if !seen.contains(&target) {
                    if seen.len() >= max_states {
                        return Err(ExactProbabilityGraphError::LimitReached);
                    }
                    seen.insert(target.clone());
                    queue.push_back((target.clone(), post, control));
                }
                choice.push(ProbabilityEdge {
                    target,
                    probability,
                });
            }
            choices.push(choice);
        }
        states.insert(
            key,
            CheckerState {
                terminal: None,
                choices,
            },
        );
    }
    Ok((
        CheckerGraph {
            initial: root,
            states,
        },
        ranks,
    ))
}
