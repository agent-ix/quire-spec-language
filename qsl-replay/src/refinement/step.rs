// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-141 (ADR-020 RS-2 to RS-6): [`check_initial`] and [`check_step`], the
//! two functions that decide a refinement's initial states and steps.
//!
//! The model checker (FR-142) and replay (FR-145) call these and no other
//! step rule, as every temporal engine answers to the trace evaluator
//! (ADR-018 SM-1). A step is decided against the abstract model with the
//! calls the abstract `ModelSystem` makes when it expands the mapped
//! pre-state, applied to the one mapped post-state.
//!
//! A refinement's history values (FR-138) ride in the position beside the
//! concrete state. When it has a hidden abstract field (AX-2) a position
//! holds the candidate set: every abstract state whose visible part is the
//! mapped one and that is consistent with the behaviour so far. A step is
//! decided against each member, over every value the hidden fields of the
//! mapped post-state may take, and the set after it is the union of the
//! accepted ones.

use qsl_eval::simulation::model::{
    operation_identity_text, AbstractApplication, Judgement, ModelFinding, ModelSimState,
    ModelSystem, Rejection, ResolvedStep, RowReading,
};
use qsl_eval::simulation::{ExpansionLimit, ExpansionStop};
use qsl_semantics::check::{CheckedRefinement, CheckedRow, StepClass, StepRow, StepTarget};
use qsl_semantics::model::state::ModelState;
use quire_checked::key::DeclarationKey;
use quire_exact::{ObjectReference, Value};
use quire_semantic_value::catalog::InternalFault;
use quire_semantic_value::model_transition::ModelTransition;

use crate::model_check::counterexample::{
    MappingSite, UndefinedEvaluation, UndefinedLocus, UndefinedSite,
};
use crate::model_check::limits::{ModelCheckLimit, ReachedLimit};
use crate::refinement::failure::{
    ProtocolTrace, RefinementFailure, StepRejection, StepResult, StepVerdict, Taken, Undetermined,
};
use crate::refinement::history::{advance, has_update, initial_history, HistoryValues};
use crate::refinement::map::{
    lift_value, map_state, object_of, EvalFailure, Lifting, MappedState, MappingFailure,
    MappingRowRef, MappingUndetermined, StepArgument,
};
use crate::refinement::protocol_system::{AbstractProtocolSystem, ProtocolCandidate};

/// A concrete state with its history values and the abstract states it
/// stands for (FR-141 `RefinementPosition`). Without hidden fields the
/// abstract side is the one state the mapping gives; with them it is the
/// AX-2 candidate set.
#[derive(Clone, Debug)]
pub struct RefinementPosition {
    /// The concrete state.
    pub concrete: ModelState,
    /// The refinement's history values at it (FR-138).
    pub history: HistoryValues,
    /// The abstract states, sorted by state key.
    pub abstract_states: Vec<ModelState>,
    /// The protocol control states of an abstract protocol subject (FR-148).
    /// Empty for a model subject.
    pub abstract_protocol_states: Vec<super::protocol_system::ProtocolCandidate>,
}

/// One concrete step to decide: where it ends and what it did.
#[derive(Clone, Copy, Debug)]
pub struct ConcreteStep<'s> {
    /// The index of the step's post-state in the behaviour.
    pub position: u64,
    /// The transition identity; `None` is the concrete terminal stutter step
    /// (ADR-018 SM-4), which is a `stutter` step whatever rows the
    /// refinement has (RS-6).
    pub transition: Option<&'s ModelTransition>,
    /// The post-state, with the effect of the step that reached it.
    pub post: &'s ModelSimState,
}

fn fault(invariant: &'static str) -> StepVerdict {
    StepVerdict::Fault(InternalFault::new("refinement-step", invariant))
}

/// `state` without its hidden fields: the part of an abstract state a
/// mapping gives (AX-2), which every member of a candidate set shares.
fn visible_part(refinement: &CheckedRefinement, state: &ModelState) -> ModelState {
    let mut visible = state.clone();
    for objects in visible.populations.values_mut() {
        for object in objects.values_mut() {
            let hidden = refinement
                .objects()
                .iter()
                .find(|row| row.abstract_type == object.type_identity)
                .map(|row| row.hidden.as_slice())
                .unwrap_or_default();
            for field in hidden {
                object.fields.remove(field);
            }
        }
    }
    visible
}

/// `states` in state-key order, each state once.
#[allow(
    clippy::result_large_err,
    reason = "FR-141 fixes StepVerdict's shape: the verdict is also the early return of an inner step"
)]
fn sorted(
    system: &ModelSystem<'_>,
    states: impl IntoIterator<Item = ModelState>,
) -> Result<Vec<ModelState>, StepVerdict> {
    let mut keyed = Vec::new();
    for state in states {
        let (key, _) = system
            .state_key_of(&state)
            .map_err(|_| fault("abstract-state-encodes"))?;
        keyed.push((key, state));
    }
    keyed.sort_by(|left, right| left.0.cmp(&right.0));
    keyed.dedup_by(|later, earlier| later.0 == earlier.0);
    Ok(keyed.into_iter().map(|(_, state)| state).collect())
}

/// FR-141 "Initial states" (RS-2): `concrete_initial`, the initial state at
/// `index` of the concrete subject, maps to the abstract subject's own
/// initial states, compared by state key on the visible part (the whole
/// state when no field is hidden).
///
/// `Passes` carries `Taken::Stutter`: an initial state takes no step.
pub fn check_initial(
    refinement: &CheckedRefinement,
    concrete_system: &ModelSystem<'_>,
    abstract_system: &ModelSystem<'_>,
    index: usize,
    concrete_initial: &ModelState,
) -> StepVerdict {
    let history = initial_history(refinement, concrete_system, concrete_initial);
    let mapped = match map_state(
        refinement,
        concrete_system,
        abstract_system,
        concrete_initial,
        &history,
    ) {
        Ok(mapped) => mapped,
        Err(failure) => {
            return mapping_verdict(|| state_key(concrete_system, concrete_initial), 0, failure)
        }
    };
    let Ok((mapped_key, _)) = abstract_system.state_key_of(&mapped.visible) else {
        return fault("mapped-state-encodes");
    };
    let mut matching = Vec::new();
    for initial in abstract_system.initial_states() {
        let visible = visible_part(refinement, initial.state());
        let Ok((key, _)) = abstract_system.state_key_of(&visible) else {
            return fault("initial-state-encodes");
        };
        if key == mapped_key {
            matching.push(initial.state().clone());
        }
    }
    let candidates = match sorted(abstract_system, matching) {
        Ok(candidates) => candidates,
        Err(verdict) => return verdict,
    };
    if candidates.is_empty() {
        return StepVerdict::Fails(RefinementFailure::InitialNotAbstract { initial: index });
    }
    StepVerdict::Passes {
        post: RefinementPosition {
            concrete: concrete_initial.clone(),
            history,
            abstract_states: candidates,
            abstract_protocol_states: Vec::new(),
        },
        taken: Taken::Stutter,
    }
}

/// FR-141 "Deciding the step rules" (RS-3 to RS-6): `step` taken from `pre`.
///
/// The outcome also counts every abstract transition identity the check
/// judged or expanded, which FR-142 adds to the run's `max_transitions`.
pub fn check_step(
    refinement: &CheckedRefinement,
    concrete_system: &ModelSystem<'_>,
    abstract_system: &ModelSystem<'_>,
    pre: &RefinementPosition,
    step: &ConcreteStep<'_>,
) -> StepResult {
    check_step_inner(
        refinement,
        concrete_system,
        abstract_system,
        pre,
        step,
        false,
    )
}

fn check_step_inner(
    refinement: &CheckedRefinement,
    concrete_system: &ModelSystem<'_>,
    abstract_system: &ModelSystem<'_>,
    pre: &RefinementPosition,
    step: &ConcreteStep<'_>,
    allow_protocol_node: bool,
) -> StepResult {
    let mut abstract_successors = 0;
    let verdict = decide_step(
        refinement,
        concrete_system,
        abstract_system,
        pre,
        step,
        &mut abstract_successors,
        allow_protocol_node,
    );
    StepResult {
        verdict,
        abstract_successors,
    }
}

/// FR-148 PR-3: map a concrete initial state to the abstract protocol's
/// initial roots, then close those roots over its admitted internal classes.
pub fn check_protocol_initial(
    refinement: &CheckedRefinement,
    concrete_system: &ModelSystem<'_>,
    abstract_system: &ModelSystem<'_>,
    protocol: &AbstractProtocolSystem<'_>,
    index: usize,
    concrete_initial: &ModelState,
) -> StepResult {
    let verdict = check_initial(
        refinement,
        concrete_system,
        abstract_system,
        index,
        concrete_initial,
    );
    let StepVerdict::Passes { post, taken } = verdict else {
        return StepResult {
            verdict,
            abstract_successors: 0,
        };
    };
    let visible_key =
        match abstract_system.state_key_of(&visible_part(refinement, &post.abstract_states[0])) {
            Ok((key, _)) => key,
            Err(_) => {
                return StepResult {
                    verdict: fault("mapped-state-encodes"),
                    abstract_successors: 0,
                }
            }
        };
    let mut roots = Vec::new();
    for candidate in protocol.initial() {
        let Some(state) = protocol.model_state(&candidate) else {
            return StepResult {
                verdict: fault("candidate-belongs-to-admitted-system"),
                abstract_successors: 0,
            };
        };
        let Ok((key, _)) = abstract_system.state_key_of(&visible_part(refinement, state.state()))
        else {
            return StepResult {
                verdict: fault("initial-state-encodes"),
                abstract_successors: 0,
            };
        };
        if key == visible_key {
            roots.push(candidate);
        }
    }
    let (candidates, abstract_successors) = match protocol_closure(refinement, protocol, roots) {
        Ok(closure) => closure,
        Err(verdict) => {
            return StepResult {
                verdict,
                abstract_successors: 0,
            }
        }
    };
    if candidates.is_empty() {
        return StepResult {
            verdict: StepVerdict::Fails(RefinementFailure::InitialNotAbstract { initial: index }),
            abstract_successors,
        };
    }
    let mut model_states = Vec::with_capacity(candidates.len());
    for candidate in &candidates {
        let Some(state) = protocol.model_state(candidate) else {
            return StepResult {
                verdict: fault("candidate-belongs-to-admitted-system"),
                abstract_successors,
            };
        };
        model_states.push(state.state().clone());
    }
    let mut post = post;
    post.abstract_states = model_states;
    post.abstract_protocol_states = candidates;
    StepResult {
        verdict: StepVerdict::Passes { post, taken },
        abstract_successors,
    }
}

/// FR-148: decide a concrete model edge against abstract protocol states.
/// Model operation matching reuses FR-141's argument evaluation and contract
/// decisions; the protocol expansion then retains matching protocol steps.
pub fn check_protocol_step(
    refinement: &CheckedRefinement,
    concrete_system: &ModelSystem<'_>,
    abstract_system: &ModelSystem<'_>,
    protocol: &AbstractProtocolSystem<'_>,
    pre: &RefinementPosition,
    step: &ConcreteStep<'_>,
) -> StepResult {
    let row = step
        .transition
        .and_then(|transition| step_row(refinement, transition));
    let protocol_node = row.is_some_and(|row| matches!(&row.target, StepTarget::Node { .. }));
    let model_result = check_step_inner(
        refinement,
        concrete_system,
        abstract_system,
        pre,
        step,
        protocol_node,
    );
    let StepVerdict::Passes { mut post, taken } = model_result.verdict else {
        if let StepVerdict::Fails(RefinementFailure::AbstractStepRejected { .. }) =
            model_result.verdict
        {
            return StepResult {
                verdict: StepVerdict::Fails(RefinementFailure::NoAbstractMatch {
                    position: step.position,
                }),
                abstract_successors: model_result.abstract_successors,
            };
        }
        return model_result;
    };
    let row = step
        .transition
        .and_then(|transition| step_row(refinement, transition));
    if row.is_some_and(|row| matches!(&row.target, StepTarget::Any)) {
        return check_protocol_any_step(
            refinement,
            abstract_system,
            protocol,
            pre,
            step,
            post,
            taken,
            model_result.abstract_successors,
        );
    }
    if protocol_node {
        return check_protocol_node_step(
            refinement,
            concrete_system,
            abstract_system,
            protocol,
            pre,
            step,
            post,
            model_result.abstract_successors,
        );
    }
    if matches!(taken, Taken::Stutter) {
        post.abstract_protocol_states = pre.abstract_protocol_states.clone();
        return StepResult {
            verdict: StepVerdict::Passes {
                post,
                taken: Taken::Stutter,
            },
            abstract_successors: model_result.abstract_successors,
        };
    }
    let Taken::Abstract(accepted) = taken else {
        return StepResult {
            verdict: fault("a-model-operation-row-yields-model-transitions"),
            abstract_successors: model_result.abstract_successors,
        };
    };
    let visible_key =
        match abstract_system.state_key_of(&visible_part(refinement, &post.abstract_states[0])) {
            Ok((key, _)) => key,
            Err(_) => {
                return StepResult {
                    verdict: fault("mapped-state-encodes"),
                    abstract_successors: model_result.abstract_successors,
                }
            }
        };
    let mut matching = Vec::new();
    let mut count = model_result.abstract_successors;
    for before in &pre.abstract_protocol_states {
        let expansion = match protocol.successors(before) {
            Ok(expansion) => expansion,
            Err(_) => {
                return StepResult {
                    verdict: fault("protocol-expansion"),
                    abstract_successors: count,
                }
            }
        };
        count = count.saturating_add(u64::try_from(expansion.successors.len()).unwrap_or(u64::MAX));
        for (transition, candidate) in expansion.successors {
            let Some(application) = transition.application() else {
                continue;
            };
            if !accepted.contains(application) {
                continue;
            }
            let Some(state) = protocol.model_state(&candidate) else {
                return StepResult {
                    verdict: fault("candidate-belongs-to-admitted-system"),
                    abstract_successors: count,
                };
            };
            let Ok((key, _)) =
                abstract_system.state_key_of(&visible_part(refinement, state.state()))
            else {
                return StepResult {
                    verdict: fault("protocol-model-state-encodes"),
                    abstract_successors: count,
                };
            };
            if key == visible_key {
                match make_protocol_trace(
                    protocol,
                    before.clone(),
                    vec![transition],
                    vec![candidate],
                ) {
                    Ok(trace) => matching.push(trace),
                    Err(verdict) => {
                        return StepResult {
                            verdict,
                            abstract_successors: count,
                        }
                    }
                }
            }
        }
    }
    let (paths, closure_count) = match protocol_closure_traces(refinement, protocol, matching) {
        Ok(closure) => closure,
        Err(verdict) => {
            return StepResult {
                verdict,
                abstract_successors: count,
            }
        }
    };
    count = count.saturating_add(closure_count);
    let mut candidates = paths
        .iter()
        .map(|path| path.candidates.last().unwrap_or(&path.start).clone())
        .collect::<Vec<_>>();
    candidates.sort_by_key(|candidate| protocol.state_key(candidate).ok().map(|(key, _)| key));
    candidates.dedup_by(|later, earlier| {
        protocol.state_key(later).ok().map(|(key, _)| key)
            == protocol.state_key(earlier).ok().map(|(key, _)| key)
    });
    if candidates.is_empty() {
        return StepResult {
            verdict: StepVerdict::Fails(RefinementFailure::NoAbstractMatch {
                position: step.position,
            }),
            abstract_successors: count,
        };
    }
    let mut model_states = Vec::with_capacity(candidates.len());
    for candidate in &candidates {
        let Some(state) = protocol.model_state(candidate) else {
            return StepResult {
                verdict: fault("candidate-belongs-to-admitted-system"),
                abstract_successors: count,
            };
        };
        model_states.push(state.state().clone());
    }
    post.abstract_states = model_states;
    post.abstract_protocol_states = candidates;
    StepResult {
        verdict: StepVerdict::Passes {
            post,
            taken: Taken::ProtocolTraces(paths),
        },
        abstract_successors: count,
    }
}

#[allow(
    clippy::too_many_arguments,
    reason = "this is one FR-148 step decision over the shared protocol product context"
)]
fn check_protocol_any_step(
    refinement: &CheckedRefinement,
    abstract_system: &ModelSystem<'_>,
    protocol: &AbstractProtocolSystem<'_>,
    pre: &RefinementPosition,
    step: &ConcreteStep<'_>,
    mut post: RefinementPosition,
    taken: Taken,
    mut count: u64,
) -> StepResult {
    let mapped = match post.abstract_states.first() {
        Some(state) => state,
        None => {
            return StepResult {
                verdict: fault("a-mapped-post-state-exists"),
                abstract_successors: count,
            }
        }
    };
    let visible = visible_part(refinement, mapped);
    let Ok((visible_key, _)) = abstract_system.state_key_of(&visible) else {
        return StepResult {
            verdict: fault("mapped-state-encodes"),
            abstract_successors: count,
        };
    };
    let pre_visible = pre
        .abstract_states
        .first()
        .map(|state| visible_part(refinement, state));
    let stuttered = pre_visible
        .and_then(|state| abstract_system.state_key_of(&state).ok())
        .is_some_and(|(key, _)| key == visible_key);
    let mut seeds: Vec<ProtocolTrace> = if stuttered {
        let mut traces = Vec::new();
        for state in &pre.abstract_protocol_states {
            match make_protocol_trace(protocol, state.clone(), Vec::new(), Vec::new()) {
                Ok(trace) => traces.push(trace),
                Err(verdict) => {
                    return StepResult {
                        verdict,
                        abstract_successors: count,
                    }
                }
            }
        }
        traces
    } else {
        Vec::new()
    };
    match taken {
        Taken::Abstract(_) | Taken::Stutter => {}
        Taken::Protocol(_) | Taken::ProtocolTraces(_) => {
            return StepResult {
                verdict: fault("any-row-is-decided-before-protocol-matching"),
                abstract_successors: count,
            }
        }
    }
    let internal: std::collections::BTreeSet<&str> = refinement
        .abstract_internal()
        .iter()
        .map(|class| class.spelling())
        .collect();
    for before in &pre.abstract_protocol_states {
        let expansion = match protocol.successors(before) {
            Ok(expansion) => expansion,
            Err(_) => {
                return StepResult {
                    verdict: fault("protocol-expansion"),
                    abstract_successors: count,
                }
            }
        };
        count = count.saturating_add(u64::try_from(expansion.successors.len()).unwrap_or(u64::MAX));
        for (transition, candidate) in expansion.successors {
            let Some(class) = protocol_class(protocol, &transition) else {
                continue;
            };
            if internal.contains(class.as_str()) {
                continue;
            }
            let Some(state) = protocol.model_state(&candidate) else {
                return StepResult {
                    verdict: fault("candidate-belongs-to-admitted-system"),
                    abstract_successors: count,
                };
            };
            let Ok((key, _)) =
                abstract_system.state_key_of(&visible_part(refinement, state.state()))
            else {
                return StepResult {
                    verdict: fault("protocol-model-state-encodes"),
                    abstract_successors: count,
                };
            };
            if key == visible_key {
                match make_protocol_trace(
                    protocol,
                    before.clone(),
                    vec![transition],
                    vec![candidate],
                ) {
                    Ok(trace) => seeds.push(trace),
                    Err(verdict) => {
                        return StepResult {
                            verdict,
                            abstract_successors: count,
                        }
                    }
                }
            }
        }
    }
    let (paths, closure_count) = match protocol_closure_traces(refinement, protocol, seeds) {
        Ok(closure) => closure,
        Err(verdict) => {
            return StepResult {
                verdict,
                abstract_successors: count,
            }
        }
    };
    count = count.saturating_add(closure_count);
    let mut candidates = paths
        .iter()
        .map(|path| path.candidates.last().unwrap_or(&path.start).clone())
        .collect::<Vec<_>>();
    candidates.sort_by_key(|candidate| protocol.state_key(candidate).ok().map(|(key, _)| key));
    candidates.dedup_by(|later, earlier| {
        protocol.state_key(later).ok().map(|(key, _)| key)
            == protocol.state_key(earlier).ok().map(|(key, _)| key)
    });
    if candidates.is_empty() {
        return StepResult {
            verdict: StepVerdict::Fails(RefinementFailure::NoAbstractMatch {
                position: step.position,
            }),
            abstract_successors: count,
        };
    }
    let mut model_states = Vec::with_capacity(candidates.len());
    for candidate in &candidates {
        let Some(state) = protocol.model_state(candidate) else {
            return StepResult {
                verdict: fault("candidate-belongs-to-admitted-system"),
                abstract_successors: count,
            };
        };
        model_states.push(state.state().clone());
    }
    post.abstract_states = model_states;
    post.abstract_protocol_states = candidates;
    let traces: Vec<_> = paths
        .into_iter()
        .filter(|path| !path.transitions.is_empty())
        .collect();
    if stuttered && traces.is_empty() {
        return StepResult {
            verdict: StepVerdict::Passes {
                post,
                taken: Taken::Stutter,
            },
            abstract_successors: count,
        };
    }
    StepResult {
        verdict: StepVerdict::Passes {
            post,
            taken: Taken::ProtocolTraces(traces),
        },
        abstract_successors: count,
    }
}

#[allow(
    clippy::too_many_arguments,
    reason = "the node row needs both admitted systems and the concrete step context"
)]
fn check_protocol_node_step(
    refinement: &CheckedRefinement,
    concrete_system: &ModelSystem<'_>,
    abstract_system: &ModelSystem<'_>,
    protocol: &AbstractProtocolSystem<'_>,
    pre: &RefinementPosition,
    step: &ConcreteStep<'_>,
    mut post: RefinementPosition,
    mut count: u64,
) -> StepResult {
    let Some(transition) = step.transition else {
        return StepResult {
            verdict: fault("a-protocol-node-row-has-a-concrete-transition"),
            abstract_successors: count,
        };
    };
    let Some(row) = step_row(refinement, transition) else {
        return StepResult {
            verdict: fault("a-protocol-node-row-has-a-concrete-transition"),
            abstract_successors: count,
        };
    };
    let StepTarget::Node {
        protocol: target_protocol,
        node,
        arguments: _,
    } = &row.target
    else {
        return StepResult {
            verdict: fault("a-protocol-node-row-names-a-node"),
            abstract_successors: count,
        };
    };
    if target_protocol.rsplit("::").next() != Some(protocol.protocol().name.as_str()) {
        return StepResult {
            verdict: fault("a-node-row-names-the-selected-protocol"),
            abstract_successors: count,
        };
    }
    let Ok(Some(resolved)) = concrete_system.resolve_step(&pre.concrete, transition, step.post)
    else {
        return StepResult {
            verdict: fault("a-protocol-node-row-resolves-its-concrete-step"),
            abstract_successors: count,
        };
    };
    let (evaluated, soft) = match evaluate_node_arguments(
        refinement,
        concrete_system,
        abstract_system,
        pre,
        step,
        row,
        &resolved,
    ) {
        Ok(result) => result,
        Err(verdict) => {
            return StepResult {
                verdict,
                abstract_successors: count,
            }
        }
    };
    if let Some(verdict) = soft.first() {
        return StepResult {
            verdict,
            abstract_successors: count,
        };
    }
    let Some(evaluated) = evaluated else {
        return StepResult {
            verdict: fault("a-protocol-node-row-arguments-resolve"),
            abstract_successors: count,
        };
    };
    let visible = post
        .abstract_states
        .first()
        .map(|state| visible_part(refinement, state));
    let Some(visible) = visible else {
        return StepResult {
            verdict: fault("a-mapped-post-state-exists"),
            abstract_successors: count,
        };
    };
    let Ok((visible_key, _)) = abstract_system.state_key_of(&visible) else {
        return StepResult {
            verdict: fault("mapped-state-encodes"),
            abstract_successors: count,
        };
    };
    let mut matching = Vec::new();
    for before in &pre.abstract_protocol_states {
        let expansion = match protocol.successors(before) {
            Ok(expansion) => expansion,
            Err(_) => {
                return StepResult {
                    verdict: fault("protocol-expansion"),
                    abstract_successors: count,
                }
            }
        };
        count = count.saturating_add(u64::try_from(expansion.successors.len()).unwrap_or(u64::MAX));
        for (candidate_transition, candidate) in expansion.successors {
            if candidate_transition.node().is_none_or(|candidate_node| {
                candidate_node != node
                    && candidate_node.rsplit('/').next() != node.rsplit('/').next()
            }) || !protocol_node_arguments_match(
                abstract_system,
                &candidate_transition,
                &evaluated,
            ) {
                continue;
            }
            let Some(state) = protocol.model_state(&candidate) else {
                return StepResult {
                    verdict: fault("candidate-belongs-to-admitted-system"),
                    abstract_successors: count,
                };
            };
            let Ok((key, _)) =
                abstract_system.state_key_of(&visible_part(refinement, state.state()))
            else {
                return StepResult {
                    verdict: fault("protocol-model-state-encodes"),
                    abstract_successors: count,
                };
            };
            if key == visible_key {
                match make_protocol_trace(
                    protocol,
                    before.clone(),
                    vec![candidate_transition],
                    vec![candidate],
                ) {
                    Ok(trace) => matching.push(trace),
                    Err(verdict) => {
                        return StepResult {
                            verdict,
                            abstract_successors: count,
                        }
                    }
                }
            }
        }
    }
    let (paths, closure_count) = match protocol_closure_traces(refinement, protocol, matching) {
        Ok(closure) => closure,
        Err(verdict) => {
            return StepResult {
                verdict,
                abstract_successors: count,
            }
        }
    };
    count = count.saturating_add(closure_count);
    let mut candidates = Vec::with_capacity(paths.len());
    for path in &paths {
        candidates.push(path.candidates.last().unwrap_or(&path.start).clone());
    }
    let mut keyed = Vec::new();
    for (path, candidate) in paths.into_iter().zip(candidates.iter()) {
        if let Ok((key, _)) = protocol.state_key(candidate) {
            keyed.push((key, path));
        }
    }
    keyed.sort_by(|left, right| left.0.cmp(&right.0));
    let paths: Vec<_> = keyed.into_iter().map(|(_, path)| path).collect();
    candidates.sort_by_key(|candidate| protocol.state_key(candidate).ok().map(|(key, _)| key));
    candidates.dedup_by(|later, earlier| {
        protocol.state_key(later).ok().map(|(key, _)| key)
            == protocol.state_key(earlier).ok().map(|(key, _)| key)
    });
    if candidates.is_empty() {
        return StepResult {
            verdict: StepVerdict::Fails(RefinementFailure::NoAbstractMatch {
                position: step.position,
            }),
            abstract_successors: count,
        };
    }
    let mut model_states = Vec::with_capacity(candidates.len());
    for candidate in &candidates {
        let Some(state) = protocol.model_state(candidate) else {
            return StepResult {
                verdict: fault("candidate-belongs-to-admitted-system"),
                abstract_successors: count,
            };
        };
        model_states.push(state.state().clone());
    }
    post.abstract_states = model_states;
    post.abstract_protocol_states = candidates;
    StepResult {
        verdict: StepVerdict::Passes {
            post,
            taken: Taken::ProtocolTraces(paths),
        },
        abstract_successors: count,
    }
}

fn protocol_node_arguments_match(
    model: &ModelSystem<'_>,
    transition: &qsl_eval::simulation::protocol::ProtocolTransition,
    expected: &[Option<Value>],
) -> bool {
    if let Some(record) = transition.binder_record() {
        let actual: Vec<Option<&Value>> = match record {
            Value::Composite(composite) => composite
                .slots()
                .iter()
                .map(|slot| match slot {
                    quire_exact::FieldValue::Present(value) => Some(value),
                    quire_exact::FieldValue::Absent | quire_exact::FieldValue::Null => None,
                })
                .collect(),
            other => vec![Some(other)],
        };
        return actual.len() == expected.len()
            && actual.iter().zip(expected).all(|(actual, expected)| {
                expected.as_ref().is_none_or(|expected| {
                    actual.is_some_and(|actual| {
                        model.value_key(expected).is_ok_and(|expected| {
                            model
                                .value_key(actual)
                                .is_ok_and(|actual| expected == actual)
                        })
                    })
                })
            });
    }
    if let Some(application) = transition.application() {
        let (Some(receiver), arguments) = (application.receiver(), application.arguments()) else {
            return false;
        };
        if expected.len() != arguments.len().saturating_add(1) {
            return false;
        }
        let receiver_matches = expected[0].as_ref().is_none_or(|value| {
            model
                .value_key(value)
                .is_ok_and(|key| key.as_slice() == receiver.as_bytes())
        });
        return receiver_matches
            && arguments
                .iter()
                .zip(&expected[1..])
                .all(|(argument, expected)| {
                    expected.as_ref().is_none_or(|value| {
                        model
                            .value_key(value)
                            .is_ok_and(|key| key.as_slice() == argument.as_bytes())
                    })
                });
    }
    expected.is_empty()
}

#[allow(
    clippy::result_large_err,
    reason = "FR-141 fixes StepVerdict's shape, which this closure returns directly"
)]
fn protocol_closure(
    refinement: &CheckedRefinement,
    system: &AbstractProtocolSystem<'_>,
    seeds: Vec<ProtocolCandidate>,
) -> Result<(Vec<ProtocolCandidate>, u64), StepVerdict> {
    let traces = seeds
        .into_iter()
        .map(|start| make_protocol_trace(system, start, Vec::new(), Vec::new()))
        .collect::<Result<Vec<_>, _>>()?;
    let (traces, expanded) = protocol_closure_traces(refinement, system, traces)?;
    let mut keyed = Vec::new();
    for trace in traces {
        let candidate = trace.candidates.last().unwrap_or(&trace.start).clone();
        let (key, _) = system
            .state_key(&candidate)
            .map_err(|_| fault("protocol-state-encodes"))?;
        keyed.push((key, candidate));
    }
    keyed.sort_by(|left, right| left.0.cmp(&right.0));
    keyed.dedup_by(|later, earlier| later.0 == earlier.0);
    Ok((
        keyed.into_iter().map(|(_, candidate)| candidate).collect(),
        expanded,
    ))
}

fn make_protocol_trace(
    system: &AbstractProtocolSystem<'_>,
    start: ProtocolCandidate,
    transitions: Vec<qsl_eval::simulation::protocol::ProtocolTransition>,
    candidates: Vec<ProtocolCandidate>,
) -> Result<ProtocolTrace, StepVerdict> {
    let start_key = system
        .state_key(&start)
        .map_err(|_| fault("protocol-state-encodes"))?
        .0;
    let candidate_keys = candidates
        .iter()
        .map(|candidate| {
            system
                .state_key(candidate)
                .map(|(key, _)| key)
                .map_err(|_| fault("protocol-state-encodes"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(ProtocolTrace {
        start,
        start_key,
        transitions,
        candidates,
        candidate_keys,
    })
}

fn protocol_closure_traces(
    refinement: &CheckedRefinement,
    system: &AbstractProtocolSystem<'_>,
    seeds: Vec<ProtocolTrace>,
) -> Result<(Vec<ProtocolTrace>, u64), StepVerdict> {
    let internal: std::collections::BTreeSet<&str> = refinement
        .abstract_internal()
        .iter()
        .map(|class| class.spelling())
        .collect();
    let mut expanded = 0_u64;
    let mut paths = Vec::new();
    for seed in seeds {
        let mut known = std::collections::BTreeMap::<Vec<u8>, ProtocolTrace>::new();
        let mut queue = std::collections::VecDeque::new();
        let mut closure_edges = std::collections::BTreeSet::new();
        let mut closure_paths = Vec::new();
        let seed_candidate = seed.candidates.last().unwrap_or(&seed.start);
        let seed_key = system
            .state_key(seed_candidate)
            .map_err(|_| fault("protocol-state-encodes"))?
            .0;
        known.insert(seed_key.clone(), seed.clone());
        queue.push_back(seed_key);
        while let Some(key) = queue.pop_front() {
            let Some(path) = known.get(&key).cloned() else {
                return Err(fault("protocol-closure-queue-has-candidate"));
            };
            let state = path.candidates.last().unwrap_or(&path.start);
            let expansion = system
                .successors(state)
                .map_err(|stop| stop_verdict(system.model(), stop))?;
            if let Some((transition, clause)) = expansion.findings.iter().find_map(|finding| {
                let ModelFinding::ContractUndetermined {
                    transition, clause, ..
                } = finding
                else {
                    return None;
                };
                Some((transition.clone(), clause.clone()))
            }) {
                return Err(StepVerdict::ProtocolContractUndetermined {
                    candidate: state.clone(),
                    transition,
                    clause,
                    trace: path,
                });
            }
            for (transition, post) in expansion.successors {
                expanded = expanded.saturating_add(1);
                let Some(class) = protocol_class(system, &transition) else {
                    continue;
                };
                if !internal.contains(class.as_str()) {
                    continue;
                }
                let (post_key, _) = system
                    .state_key(&post)
                    .map_err(|_| fault("protocol-state-encodes"))?;
                if let Some(target_path) = known.get(&post_key) {
                    let transition_key = system
                        .transition_key(&transition)
                        .map_err(|_| fault("protocol-transition-encodes"))?;
                    if !closure_edges.insert((key.clone(), transition_key, post_key.clone())) {
                        continue;
                    }
                    let mut cycle = path.clone();
                    cycle.transitions.push(transition);
                    cycle.candidate_keys.push(post_key);
                    cycle.candidates.push(
                        target_path
                            .candidates
                            .last()
                            .unwrap_or(&target_path.start)
                            .clone(),
                    );
                    closure_paths.push(cycle);
                    continue;
                }
                let mut next = path.clone();
                next.transitions.push(transition);
                next.candidate_keys.push(post_key.clone());
                next.candidates.push(post);
                known.insert(post_key.clone(), next);
                if u64::try_from(known.len()).unwrap_or(u64::MAX) > system.model().max_candidates()
                {
                    return Err(StepVerdict::Stopped(ReachedLimit {
                        limit: ModelCheckLimit::MaxCandidates,
                        value: system.model().max_candidates(),
                    }));
                }
                queue.push_back(post_key);
            }
        }
        paths.extend(known.into_values());
        paths.extend(closure_paths);
    }
    paths.sort_by(|left, right| {
        let left = left.candidates.last().unwrap_or(&left.start);
        let right = right.candidates.last().unwrap_or(&right.start);
        let left_key = system.state_key(left).ok().map(|(key, _)| key);
        let right_key = system.state_key(right).ok().map(|(key, _)| key);
        left_key.cmp(&right_key)
    });
    Ok((paths, expanded))
}

pub(crate) fn protocol_class(
    system: &AbstractProtocolSystem<'_>,
    transition: &qsl_eval::simulation::protocol::ProtocolTransition,
) -> Option<String> {
    use qsl_eval::simulation::protocol::StepKind;
    let protocol = &system.protocol().name;
    let member = transition
        .node()
        .and_then(|node| node.rsplit('/').next())
        .map(str::to_owned);
    Some(match transition.kind() {
        StepKind::Attempt
        | StepKind::Event
        | StepKind::Send
        | StepKind::Receive
        | StepKind::Timeout
        | StepKind::Finish
        | StepKind::Fence => {
            format!("{protocol}::{}", member?)
        }
        StepKind::Fork => format!("fork {protocol}::{}", member?),
        StepKind::Join => format!("join {protocol}::{}", member?),
        StepKind::CAttempt => format!("{protocol}::{}", transition.template()?),
        StepKind::CEnd => format!("cend {protocol}::{}", transition.template()?),
        StepKind::Duplicate => format!("duplicate {protocol}::{}", transition.channel_queue()?.0),
        StepKind::Lose => format!("lose {protocol}::{}", transition.channel_queue()?.0),
        StepKind::Spawn => format!("spawn {protocol}::{}", transition.role_instance()?.0),
        StepKind::Retire => format!("retire {protocol}::{}", transition.role_instance()?.0),
        StepKind::Activate => format!("activate {protocol}"),
        StepKind::Memory => format!("memory {}", transition.memory_step()?.step),
    })
}

/// The first undefined and the first undetermined outcome of the step-row
/// arguments, the history updates and the post-state's rows, in reading
/// order. Of the row outcomes the first undefined one outranks the first
/// undetermined one (QSpec FR-377 "Failure precedence").
#[derive(Default)]
struct Soft {
    undefined: Option<StepVerdict>,
    undetermined: Option<StepVerdict>,
}

impl Soft {
    /// Keep `verdict` if it is a row outcome; hand back any other verdict,
    /// which ends the check at once.
    #[allow(
        clippy::result_large_err,
        reason = "FR-141 fixes StepVerdict's shape: the verdict is also the early return of an inner step"
    )]
    fn record(&mut self, verdict: StepVerdict) -> Result<(), StepVerdict> {
        let slot = match verdict {
            StepVerdict::Undefined(_) => &mut self.undefined,
            StepVerdict::Undetermined(_) => &mut self.undetermined,
            other => return Err(other),
        };
        slot.get_or_insert(verdict);
        Ok(())
    }

    fn first(self) -> Option<StepVerdict> {
        self.undefined.or(self.undetermined)
    }
}

/// A verdict for an `ExpansionStop` of `system`: a budget stop is a
/// [`StepVerdict::Stopped`] naming the limit and its value (a result domain
/// too large to order counts against the candidate bound), a broken
/// invariant a fault.
pub(crate) fn stop_verdict(system: &ModelSystem<'_>, stop: ExpansionStop) -> StepVerdict {
    if stop.is_internal_failure() {
        return fault("system-invariant");
    }
    StepVerdict::Stopped(match stop.limit {
        Some(ExpansionLimit::EvaluationMeter(counter)) => ReachedLimit {
            limit: ModelCheckLimit::EvaluationMeter { counter },
            value: crate::model_check::limits::meter_bound(&system.meter(), counter),
        },
        Some(ExpansionLimit::MaxCandidates) | None => ReachedLimit {
            limit: ModelCheckLimit::MaxCandidates,
            value: system.max_candidates(),
        },
    })
}

#[allow(
    clippy::result_large_err,
    reason = "FR-141 fixes StepVerdict's shape: the verdict is also the early return of an inner step"
)]
fn decide_step(
    refinement: &CheckedRefinement,
    concrete_system: &ModelSystem<'_>,
    abstract_system: &ModelSystem<'_>,
    pre: &RefinementPosition,
    step: &ConcreteStep<'_>,
    abstract_successors: &mut u64,
    allow_protocol_node: bool,
) -> StepVerdict {
    let Some(before) = pre.abstract_states.first() else {
        return fault("a-position-holds-an-abstract-state");
    };
    let row = match step.transition {
        None => None,
        Some(transition) => match step_row(refinement, transition) {
            Some(row) => Some(row),
            None => return fault("every-concrete-operation-has-a-step-row"),
        },
    };
    let applies = matches!(
        row.map(|row| &row.target),
        Some(StepTarget::Abstract { .. })
    ) || (allow_protocol_node
        && matches!(row.map(|row| &row.target), Some(StepTarget::Node { .. })));
    // The step's receiver and arguments, found when a row or an update reads
    // them.
    let mut resolved = None;
    if let Some(transition) = step.transition {
        let Ok(updates) = has_update(refinement, transition) else {
            return fault("an-operation-has-an-identity-text");
        };
        if applies || updates {
            resolved = match concrete_system.resolve_step(&pre.concrete, transition, step.post) {
                Ok(Some(found)) => Some(found),
                Ok(None) => return fault("concrete-step-names-an-argument-vector"),
                Err(stop) => return stop_verdict(concrete_system, stop),
            };
        }
    }
    // Every receiver and argument of the row is read at the pre-state, the
    // history updates are evaluated and every row of the post-state is
    // mapped, in that order; the first undefined outcome of them outranks
    // the first undetermined one.
    let mut soft = Soft::default();
    let mut arguments = None;
    if applies && row.is_some_and(|row| matches!(&row.target, StepTarget::Abstract { .. })) {
        let (Some(row), Some(resolved)) = (row, resolved.as_ref()) else {
            return fault("an-abstract-row-and-resolved-step-exist");
        };
        match evaluate_arguments(
            refinement,
            (concrete_system, abstract_system),
            pre,
            step,
            (row, resolved),
        ) {
            Ok((evaluated, found)) => {
                arguments = evaluated;
                soft = found;
            }
            Err(verdict) => return verdict,
        }
    }
    let advanced = match advance(
        refinement,
        concrete_system,
        &pre.history,
        (&pre.concrete, step.post),
        step.transition.zip(resolved.as_ref()),
    ) {
        Ok(advanced) => advanced,
        Err(failure) => {
            return mapping_verdict(
                || state_key(concrete_system, step.post.state()),
                step.position,
                failure,
            )
        }
    };
    for failure in advanced.outcomes.into_failures() {
        let verdict = mapping_verdict(
            || state_key(concrete_system, step.post.state()),
            step.position,
            failure,
        );
        if let Err(ended) = soft.record(verdict) {
            return ended;
        }
    }
    let history = advanced.values;
    // A row that reads a field whose update failed finds no value there, so
    // it is undetermined and the update's own outcome decides in the read
    // order.
    let mapped = match map_state(
        refinement,
        concrete_system,
        abstract_system,
        step.post.state(),
        &history,
    ) {
        Ok(mapped) => Some(mapped),
        Err(failure) => {
            let verdict = mapping_verdict(
                || state_key(concrete_system, step.post.state()),
                step.position,
                failure,
            );
            if let Err(ended) = soft.record(verdict) {
                return ended;
            }
            None
        }
    };
    if let Some(verdict) = soft.first() {
        return verdict;
    }
    let Some(mapped) = mapped else {
        return fault("a-mapping-gives-a-state-or-a-verdict");
    };
    let Ok((after_key, _)) = abstract_system.state_key_of(&mapped.visible) else {
        return fault("mapped-state-encodes");
    };
    let Ok((before_key, _)) = abstract_system.state_key_of(&visible_part(refinement, before))
    else {
        return fault("mapped-state-encodes");
    };
    let stuttered = after_key == before_key;
    let hidden = !mapped.hidden.is_empty();
    let passes = |taken, states: Vec<ModelState>| StepVerdict::Passes {
        post: RefinementPosition {
            concrete: step.post.state().clone(),
            history: history.clone(),
            abstract_states: states,
            abstract_protocol_states: Vec::new(),
        },
        taken,
    };
    let Some(row) = row else {
        return stutter(step, stuttered, || {
            passes(Taken::Stutter, pre.abstract_states.clone())
        });
    };
    match &row.target {
        StepTarget::Stutter => stutter(step, stuttered, || {
            passes(Taken::Stutter, pre.abstract_states.clone())
        }),
        StepTarget::Any => {
            if stuttered && !hidden {
                return passes(Taken::Stutter, pre.abstract_states.clone());
            }
            // The members after the step: the set itself for a stutter, and
            // every successor of a member whose visible part is the mapped
            // one.
            let mut next: Vec<ModelState> = if stuttered {
                pre.abstract_states.clone()
            } else {
                Vec::new()
            };
            let mut matched = Vec::new();
            let mut undecided = false;
            let mapped_keys: Vec<qsl_eval::simulation::StateKey> =
                match completions(abstract_system, &mapped) {
                    Ok(states) => states
                        .iter()
                        .filter_map(|state| abstract_system.state_key_of(state).ok())
                        .map(|(bytes, _)| {
                            qsl_eval::simulation::StateKey::from_canonical_bytes(bytes)
                        })
                        .collect(),
                    Err(verdict) => return verdict,
                };
            for candidate in &pre.abstract_states {
                let expansion = match abstract_system.expand_state(candidate) {
                    Ok(expansion) => expansion,
                    Err(stop) => return stop_verdict(abstract_system, stop),
                };
                *abstract_successors = abstract_successors
                    .saturating_add(u64::try_from(expansion.successors.len()).unwrap_or(u64::MAX));
                for (transition, successor) in &expansion.successors {
                    let visible = visible_part(refinement, successor.state());
                    match abstract_system.state_key_of(&visible) {
                        Ok((key, _)) if key == after_key => {
                            matched.push(transition.clone());
                            next.push(successor.state().clone());
                        }
                        Ok(_) => {}
                        Err(_) => return fault("successor-encodes"),
                    }
                }
                // An undecided precondition enables its successors in doubt;
                // an undecided postcondition matters only for a candidate
                // whose visible part is the mapped one.
                undecided |= expansion.findings.iter().any(|finding| {
                    matches!(
                        finding,
                        ModelFinding::ContractUndetermined { candidate, .. }
                            if candidate.as_ref().is_none_or(|key| mapped_keys.contains(key))
                    )
                });
            }
            // A successor whose contracts did not decide may be a member of
            // the set, so the set is not known (AX-2).
            if hidden && undecided {
                return StepVerdict::Undetermined(Undetermined::UndecidedSuccessor);
            }
            if next.is_empty() {
                if undecided {
                    return StepVerdict::Undetermined(Undetermined::UndecidedSuccessor);
                }
                return StepVerdict::Fails(RefinementFailure::NoAbstractMatch {
                    position: step.position,
                });
            }
            let next = match sorted(abstract_system, next) {
                Ok(next) => next,
                Err(verdict) => return verdict,
            };
            if stuttered {
                return passes(Taken::Stutter, next);
            }
            match canonical(abstract_system, matched) {
                Ok(matched) => passes(Taken::Abstract(matched), next),
                Err(verdict) => verdict,
            }
        }
        StepTarget::Node { .. } => {
            if !allow_protocol_node {
                return fault("a-protocol-node-row-needs-a-protocol-abstract-subject");
            }
            let candidates = match completions(abstract_system, &mapped) {
                Ok(candidates) => candidates,
                Err(verdict) => return verdict,
            };
            match sorted(abstract_system, candidates) {
                Ok(candidates) => passes(Taken::Abstract(Vec::new()), candidates),
                Err(verdict) => verdict,
            }
        }
        StepTarget::Abstract { operation, .. } => {
            let Some(arguments) = arguments else {
                return fault("an-abstract-row-evaluates-its-arguments");
            };
            judge_operation(
                abstract_system,
                (&pre.abstract_states, &mapped),
                operation,
                arguments,
                step,
                abstract_successors,
                passes,
            )
        }
    }
}

/// RS-3: the step passes when the mapped state is unchanged.
fn stutter(
    step: &ConcreteStep<'_>,
    stuttered: bool,
    passes: impl FnOnce() -> StepVerdict,
) -> StepVerdict {
    if stuttered {
        passes()
    } else {
        StepVerdict::Fails(RefinementFailure::StutterChanged {
            position: step.position,
        })
    }
}

/// The step row of the operation `transition` applies (FR-136).
pub(crate) fn step_row<'r>(
    refinement: &'r CheckedRefinement,
    transition: &ModelTransition,
) -> Option<&'r StepRow> {
    refinement.steps().iter().find(|row| {
        // A protocol step row maps no operation transition.
        let StepClass::Operation(operation) = &row.class else {
            return false;
        };
        operation_identity_text(operation).is_ok_and(|text| text == transition.operation())
    })
}

/// FR-379: build the exact visible observation label of an admitted protocol
/// transition, including its complete typed binder record.
pub fn protocol_observation_label(
    protocol: &AbstractProtocolSystem<'_>,
    transition: &qsl_eval::simulation::protocol::ProtocolTransition,
) -> Result<super::failure::ObservationLabel, InternalFault> {
    let step = super::protocol_transition_class(protocol, transition)
        .ok_or_else(|| InternalFault::new("refinement-relation", "protocol-class-exists"))?;
    let binder = protocol
        .binder_record_json(transition)
        .map_err(|_| InternalFault::new("refinement-relation", "protocol-binder-record-encodes"))?;
    let fields = match binder {
        None => Vec::new(),
        Some(value) => match value.get("fields").and_then(serde_json::Value::as_array) {
            Some(entries) => entries
                .iter()
                .map(|entry| {
                    let field = entry
                        .get("name")
                        .and_then(serde_json::Value::as_str)
                        .ok_or_else(|| {
                            InternalFault::new("refinement-relation", "binder-field-has-name")
                        })?;
                    let value = entry.get("value").cloned().ok_or_else(|| {
                        InternalFault::new("refinement-relation", "binder-field-has-value")
                    })?;
                    Ok(super::failure::ObservationField {
                        field: field.to_owned(),
                        value,
                    })
                })
                .collect::<Result<Vec<_>, InternalFault>>()?,
            None => {
                let names = protocol.binder_names(transition).ok_or_else(|| {
                    InternalFault::new("refinement-relation", "scalar-binder-has-checked-node")
                })?;
                let [field] = names.as_slice() else {
                    return Err(InternalFault::new(
                        "refinement-relation",
                        "scalar-binder-has-one-checked-name",
                    ));
                };
                vec![super::failure::ObservationField {
                    field: field.clone(),
                    value,
                }]
            }
        },
    };
    Ok(super::failure::ObservationLabel {
        step,
        binder: fields,
    })
}

/// FR-148/FR-379's refusal-set and terminal-success checks at one reachable
/// product state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProtocolStateRelation {
    /// Both relation checks were decided; `failure` is the first failure in
    /// FR-379 precedence order, if any.
    Decided {
        /// The first failure in FR-379 precedence order, if any.
        failure: Option<RefinementFailure>,
        /// Abstract successors expanded while checking the relation.
        expanded: u64,
    },
    /// An admitted subject reported `ContractUndetermined` while the checks
    /// were deriving enabledness or terminal success.
    Undetermined {
        /// Abstract successors expanded before the undecided finding.
        expanded: u64,
    },
}

/// A product relation check stopped on an evaluator stop or an established
/// invariant failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProtocolRelationError {
    /// A checked predicate or bounded protocol expansion stopped.
    Expansion(ExpansionStop),
    /// An admitted identity or encoding invariant failed.
    Fault(InternalFault),
}

/// Check the protocol-only refusal-set and terminal-success relations at
/// `position`. `edges` are the already-checked outgoing concrete edges of
/// that product state; a `None` transition is the synthetic terminal stutter
/// and is not a concrete enabled action.
pub fn check_protocol_state_relations(
    refinement: &CheckedRefinement,
    concrete: &ModelSystem<'_>,
    protocol: &AbstractProtocolSystem<'_>,
    position: &RefinementPosition,
    concrete_state: &ModelSimState,
    edges: &[crate::certificate::simulation::CertifiedEdge],
    position_index: u64,
) -> Result<ProtocolStateRelation, ProtocolRelationError> {
    use qsl_eval::simulation::model::TerminalClass;
    use qsl_semantics::check::StepTarget;
    use std::collections::{BTreeMap, BTreeSet};

    let internal: BTreeSet<&str> = refinement
        .abstract_internal()
        .iter()
        .map(|class| class.spelling())
        .collect();
    let mut concrete_stable = true;
    let mut concrete_visible = BTreeSet::<Vec<u8>>::new();
    for edge in edges {
        let Some(transition) = &edge.transition else {
            continue;
        };
        let row = step_row(refinement, transition);
        if row.is_some_and(|row| matches!(&row.target, StepTarget::Stutter | StepTarget::Any)) {
            concrete_stable = false;
        }
        let mut record_visible =
            |transition: &qsl_eval::simulation::protocol::ProtocolTransition| {
                let class =
                    super::protocol_transition_class(protocol, transition).ok_or_else(|| {
                        ProtocolRelationError::Fault(InternalFault::new(
                            "refinement-relation",
                            "protocol-transition-has-class",
                        ))
                    })?;
                if internal.contains(class.as_str())
                    || transition.kind() == qsl_eval::simulation::protocol::StepKind::Finish
                {
                    return Ok(());
                }
                let label = protocol_observation_label(protocol, transition)
                    .map_err(ProtocolRelationError::Fault)?;
                let encoded = serde_json::to_vec(&label).map_err(|_| {
                    ProtocolRelationError::Fault(InternalFault::new(
                        "refinement-relation",
                        "protocol-observation-label-encodes",
                    ))
                })?;
                concrete_visible.insert(encoded);
                Ok(())
            };
        match &edge.taken {
            Taken::Protocol(transitions) => {
                for transition in transitions {
                    record_visible(transition)?;
                }
            }
            Taken::ProtocolTraces(traces) => {
                for trace in traces {
                    for transition in &trace.transitions {
                        record_visible(transition)?;
                    }
                }
            }
            Taken::Stutter | Taken::Abstract(_) => {}
        }
    }
    let mut stable_candidates = Vec::new();
    let mut candidate_labels = Vec::new();
    let mut candidate_success = Vec::new();
    let mut expanded = 0_u64;
    for candidate in &position.abstract_protocol_states {
        let Some((expansion, terminal)) = protocol
            .successors_and_terminal(candidate)
            .map_err(ProtocolRelationError::Expansion)?
        else {
            return Err(ProtocolRelationError::Fault(InternalFault::new(
                "refinement-relation",
                "candidate-belongs-to-admitted-system",
            )));
        };
        expanded =
            expanded.saturating_add(u64::try_from(expansion.successors.len()).unwrap_or(u64::MAX));
        if expansion
            .findings
            .iter()
            .any(|finding| matches!(finding, ModelFinding::ContractUndetermined { .. }))
        {
            return Ok(ProtocolStateRelation::Undetermined { expanded });
        }
        let mut labels = BTreeMap::<Vec<u8>, super::failure::ObservationLabel>::new();
        let mut stable = true;
        for (transition, _) in &expansion.successors {
            // Finishing establishes the protocol's terminal classification;
            // it is not an observable action offered against a concrete step.
            if transition.kind() == qsl_eval::simulation::protocol::StepKind::Finish {
                continue;
            }
            let Some(class) = super::protocol_transition_class(protocol, transition) else {
                return Err(ProtocolRelationError::Fault(InternalFault::new(
                    "refinement-relation",
                    "protocol-transition-has-class",
                )));
            };
            if internal.contains(class.as_str()) {
                stable = false;
                continue;
            }
            let label = protocol_observation_label(protocol, transition)
                .map_err(ProtocolRelationError::Fault)?;
            let encoded = serde_json::to_vec(&label).map_err(|_| {
                ProtocolRelationError::Fault(InternalFault::new(
                    "refinement-relation",
                    "protocol-observation-label-encodes",
                ))
            })?;
            labels.entry(encoded).or_insert(label);
        }
        stable_candidates.push(stable);
        candidate_labels.push(labels);
        if stable {
            let offers_successful_finish = expansion.successors.iter().any(|(transition, _)| {
                transition.kind() == qsl_eval::simulation::protocol::StepKind::Finish
            });
            candidate_success
                .push(matches!(terminal, TerminalClass::Intended) || offers_successful_finish);
        }
    }
    if concrete_stable && stable_candidates.iter().any(|stable| *stable) {
        let stable_indices = stable_candidates
            .iter()
            .enumerate()
            .filter_map(|(index, stable)| stable.then_some(index))
            .collect::<Vec<_>>();
        let mut qualifying = Vec::<(Vec<u8>, super::failure::ObservationLabel)>::new();
        if let Some(first) = stable_indices.first().copied() {
            for (encoded, label) in &candidate_labels[first] {
                let all_enable = stable_indices
                    .iter()
                    .all(|index| candidate_labels[*index].contains_key(encoded));
                if all_enable && !concrete_visible.contains(encoded) {
                    qualifying.push((encoded.clone(), label.clone()));
                }
            }
        }
        qualifying.sort_by(|left, right| left.0.cmp(&right.0));
        if let Some((_, label)) = qualifying.into_iter().next() {
            return Ok(ProtocolStateRelation::Decided {
                failure: Some(RefinementFailure::RefusalNotPermitted {
                    position: position_index,
                    label,
                }),
                expanded,
            });
        }
    }
    let has_successor = edges.iter().any(|edge| edge.transition.is_some());
    let concrete_terminal = !has_successor;
    if concrete_terminal {
        let class = concrete
            .terminal_class(concrete_state, false)
            .map_err(ProtocolRelationError::Expansion)?;
        let concrete_intended = match class {
            TerminalClass::Intended => true,
            TerminalClass::Deadlocked => false,
            TerminalClass::NotTerminal => {
                return Err(ProtocolRelationError::Fault(InternalFault::new(
                    "refinement-relation",
                    "a-terminal-state-is-terminal",
                )))
            }
            TerminalClass::Undefined { .. } | TerminalClass::Undecided { .. } => {
                return Ok(ProtocolStateRelation::Undetermined { expanded })
            }
        };
        let all_abstract_intended =
            !candidate_success.is_empty() && candidate_success.iter().all(|success| *success);
        let any_abstract_intended = candidate_success.iter().any(|success| *success);
        let direction = if concrete_intended && !any_abstract_intended {
            Some(super::failure::TerminalSuccessDirection::ConcreteOnly)
        } else if !concrete_intended && all_abstract_intended {
            Some(super::failure::TerminalSuccessDirection::AbstractOnly)
        } else {
            None
        };
        if let Some(direction) = direction {
            return Ok(ProtocolStateRelation::Decided {
                failure: Some(RefinementFailure::TerminalSuccessMismatch {
                    position: position_index,
                    direction,
                }),
                expanded,
            });
        }
    }
    Ok(ProtocolStateRelation::Decided {
        failure: None,
        expanded,
    })
}

/// `identities` in canonical order (FR-101): ascending transition-identity
/// bytes.
#[allow(
    clippy::result_large_err,
    reason = "FR-141 fixes StepVerdict's shape: the verdict is also the early return of an inner step"
)]
fn canonical(
    system: &ModelSystem<'_>,
    identities: Vec<ModelTransition>,
) -> Result<Vec<ModelTransition>, StepVerdict> {
    let mut keyed = Vec::with_capacity(identities.len());
    for identity in identities {
        let key = system
            .transition_key(&identity)
            .map_err(|_| fault("transition-encodes"))?;
        keyed.push((key, identity));
    }
    keyed.sort_by(|left, right| left.0.cmp(&right.0));
    keyed.dedup_by(|later, earlier| later.0 == earlier.0);
    Ok(keyed.into_iter().map(|(_, identity)| identity).collect())
}

/// What an abstract-operation row reads at the pre-state: the abstract
/// receiver, one value for each argument (`None` is `_`) and the abstract
/// result the concrete step's result binds, when the row binds one.
struct Arguments {
    receiver: ObjectReference,
    supplied: Vec<Option<Value>>,
    result: Option<Value>,
}

/// RS-4: the receiver and arguments of the abstract-operation row `row`,
/// evaluated at the pre-state over the concrete step's receiver and
/// arguments (FR-141 "Abstract-operation evaluation"), with the history
/// values the pre-state holds.
#[allow(
    clippy::result_large_err,
    reason = "FR-141 fixes StepVerdict's shape: the verdict is also the early return of an inner step"
)]
fn evaluate_arguments(
    refinement: &CheckedRefinement,
    (concrete_system, abstract_system): (&ModelSystem<'_>, &ModelSystem<'_>),
    pre: &RefinementPosition,
    step: &ConcreteStep<'_>,
    (row, resolved): (&StepRow, &ResolvedStep),
) -> Result<(Option<Arguments>, Soft), StepVerdict> {
    let StepRow {
        class: StepClass::Operation(concrete_operation),
        target:
            StepTarget::Abstract {
                receiver,
                arguments,
                binds_result,
                ..
            },
        ..
    } = row
    else {
        return Err(fault("an-abstract-row-names-an-operation"));
    };
    let observed = pre
        .history
        .environment(concrete_system, &pre.concrete)
        .map_err(|_| fault("pre-history-names-its-objects"))
        .and_then(|history| {
            concrete_system
                .observe_with_history(&pre.concrete, history)
                .map_err(|_| fault("pre-state-observes"))
        })?;
    let mut bindings = vec![Value::Reference(resolved.receiver.clone())];
    bindings.extend(resolved.arguments.iter().cloned());
    let object = object_of(concrete_system, &pre.concrete, &resolved.receiver);
    let evaluate =
        |expression: &CheckedRow, argument: StepArgument| -> Result<Value, StepVerdict> {
            let at = MappingRowRef::Argument {
                operation: concrete_operation.clone(),
                argument,
            };
            match concrete_system.read_row(expression, &observed, bindings.clone()) {
                Err(_) => Err(fault("argument-row-evaluates")),
                Ok(RowReading::Value(value)) => {
                    lift_value(refinement, concrete_system, abstract_system, value).map_err(
                        |lifting| match lifting {
                            Lifting::Unmapped(concrete_type) => StepVerdict::Undetermined(
                                Undetermined::Mapping(Box::new(MappingUndetermined {
                                    row: at.clone(),
                                    object: object.clone(),
                                    cause: EvalFailure::UnmappedType(concrete_type),
                                })),
                            ),
                            Lifting::Fault => fault("argument-lifts"),
                        },
                    )
                }
                Ok(RowReading::Undefined(cause)) => {
                    let Ok((bytes, _)) = concrete_system.state_key_of(step.post.state()) else {
                        return Err(fault("post-state-encodes"));
                    };
                    Err(StepVerdict::Undefined(UndefinedEvaluation {
                        site: UndefinedSite {
                            position: step.position,
                            state_key: qsl_eval::simulation::StateKey::from_canonical_bytes(bytes),
                            locus: UndefinedLocus::Mapping(Box::new(MappingSite {
                                row: at,
                                object: object.clone(),
                            })),
                            transition: None,
                            time: None,
                        },
                        cause,
                    }))
                }
                Ok(RowReading::Undetermined(outcome)) => Err(StepVerdict::Undetermined(
                    Undetermined::Mapping(Box::new(MappingUndetermined {
                        row: at,
                        object: object.clone(),
                        cause: EvalFailure::Evaluation(Box::new(outcome)),
                    })),
                )),
            }
        };
    let mut soft = Soft::default();
    let mut read =
        |expression: &CheckedRow, argument: StepArgument| match evaluate(expression, argument) {
            Ok(value) => Ok(Some(value)),
            Err(verdict) => soft.record(verdict).map(|()| None),
        };
    let receiver = read(receiver, StepArgument::Receiver)?;
    let mut supplied = Vec::with_capacity(arguments.len());
    let mut complete = receiver.is_some();
    for (index, argument) in arguments.iter().enumerate() {
        match argument {
            None => supplied.push(None),
            Some(expression) => match read(expression, StepArgument::Parameter(index))? {
                Some(value) => supplied.push(Some(value)),
                None => complete = false,
            },
        }
    }
    let evaluated = match receiver {
        Some(Value::Reference(receiver)) if complete => Some(Arguments {
            receiver,
            supplied,
            result: if *binds_result {
                resolved.result.clone()
            } else {
                None
            },
        }),
        Some(Value::Reference(_)) | None => None,
        Some(_) => return Err(fault("a-receiver-row-is-a-reference")),
    };
    Ok((evaluated, soft))
}

#[allow(clippy::result_large_err, reason = "FR-141 fixes StepVerdict's shape")]
fn evaluate_node_arguments(
    refinement: &CheckedRefinement,
    concrete_system: &ModelSystem<'_>,
    abstract_system: &ModelSystem<'_>,
    pre: &RefinementPosition,
    step: &ConcreteStep<'_>,
    row: &StepRow,
    resolved: &ResolvedStep,
) -> Result<NodeArgumentValues, StepVerdict> {
    let StepClass::Operation(concrete_operation) = &row.class else {
        return Err(fault("a-protocol-node-row-is-a-concrete-operation-row"));
    };
    let StepTarget::Node { arguments, .. } = &row.target else {
        return Err(fault("a-protocol-node-row-targets-a-node"));
    };
    let observed = pre
        .history
        .environment(concrete_system, &pre.concrete)
        .map_err(|_| fault("pre-history-names-its-objects"))
        .and_then(|history| {
            concrete_system
                .observe_with_history(&pre.concrete, history)
                .map_err(|_| fault("pre-state-observes"))
        })?;
    let mut bindings = vec![Value::Reference(resolved.receiver.clone())];
    bindings.extend(resolved.arguments.iter().cloned());
    let object = object_of(concrete_system, &pre.concrete, &resolved.receiver);
    let mut soft = Soft::default();
    let mut values = Vec::with_capacity(arguments.len());
    let mut complete = true;
    for (index, expression) in arguments.iter().enumerate() {
        let Some(expression) = expression else {
            values.push(None);
            continue;
        };
        let site = MappingRowRef::Argument {
            operation: concrete_operation.clone(),
            argument: StepArgument::Parameter(index),
        };
        let evaluated = match concrete_system.read_row(expression, &observed, bindings.clone()) {
            Err(_) => return Err(fault("node-row-argument-evaluates")),
            Ok(RowReading::Value(value)) => {
                match lift_value(refinement, concrete_system, abstract_system, value) {
                    Ok(value) => Ok(value),
                    Err(Lifting::Unmapped(concrete_type)) => Err(StepVerdict::Undetermined(
                        Undetermined::Mapping(Box::new(MappingUndetermined {
                            row: site.clone(),
                            object: object.clone(),
                            cause: EvalFailure::UnmappedType(concrete_type),
                        })),
                    )),
                    Err(Lifting::Fault) => Err(fault("node-row-argument-lifts")),
                }
            }
            Ok(RowReading::Undefined(cause)) => {
                let Ok((bytes, _)) = concrete_system.state_key_of(step.post.state()) else {
                    return Err(fault("post-state-encodes"));
                };
                Err(StepVerdict::Undefined(UndefinedEvaluation {
                    site: UndefinedSite {
                        position: step.position,
                        state_key: qsl_eval::simulation::StateKey::from_canonical_bytes(bytes),
                        locus: UndefinedLocus::Mapping(Box::new(MappingSite {
                            row: site,
                            object: object.clone(),
                        })),
                        transition: None,
                        time: None,
                    },
                    cause,
                }))
            }
            Ok(RowReading::Undetermined(outcome)) => Err(StepVerdict::Undetermined(
                Undetermined::Mapping(Box::new(MappingUndetermined {
                    row: site,
                    object: object.clone(),
                    cause: EvalFailure::Evaluation(Box::new(outcome)),
                })),
            )),
        };
        match evaluated {
            Ok(value) => values.push(Some(value)),
            Err(verdict) => {
                complete = false;
                soft.record(verdict)?;
                values.push(None);
            }
        }
    }
    Ok((complete.then_some(values), soft))
}

type NodeArgumentValues = (Option<Vec<Option<Value>>>, Soft);

/// The abstract states a mapped post-state stands for once its hidden fields
/// take each value of their finite domains (AX-2): the visible state itself
/// when no field is hidden.
#[allow(
    clippy::result_large_err,
    reason = "FR-141 fixes StepVerdict's shape: the verdict is also the early return of an inner step"
)]
fn completions(
    system: &ModelSystem<'_>,
    mapped: &MappedState,
) -> Result<Vec<ModelState>, StepVerdict> {
    if mapped.hidden.is_empty() {
        return Ok(vec![mapped.visible.clone()]);
    }
    // A hidden reference ranges over the objects of the mapped state.
    let objects: Vec<ObjectReference> = mapped
        .visible
        .populations
        .values()
        .flatten()
        .filter_map(|(key, object)| system.object_reference(&object.type_identity, key).ok())
        .collect();
    let mut domains: Vec<Vec<Option<Value>>> = Vec::with_capacity(mapped.hidden.len());
    for slot in &mapped.hidden {
        let Some(object) = mapped
            .visible
            .populations
            .get(&slot.object.population)
            .and_then(|objects| objects.get(&slot.object.key))
        else {
            return Err(fault("a-hidden-slot-names-an-object"));
        };
        let Some(field) = system
            .fields_of(&object.type_identity)
            .and_then(|fields| fields.iter().find(|field| field.key == slot.field))
        else {
            return Err(fault("a-hidden-slot-names-a-field"));
        };
        let mut values: Vec<Option<Value>> = system
            .domain_values(&field.value_type, &objects)
            .map_err(|stop| stop_verdict(system, stop))?
            .into_iter()
            .map(Some)
            .collect();
        if field.optional {
            values.insert(0, None);
        }
        domains.push(values);
    }
    let total = domains.iter().try_fold(1u64, |total, domain| {
        total.checked_mul(u64::try_from(domain.len()).ok()?)
    });
    if total.is_none_or(|total| total > system.max_candidates()) {
        return Err(StepVerdict::Stopped(ReachedLimit {
            limit: ModelCheckLimit::MaxCandidates,
            value: system.max_candidates(),
        }));
    }
    let mut digits = vec![0_usize; domains.len()];
    let mut states = Vec::new();
    'assignments: loop {
        if domains.iter().any(Vec::is_empty) {
            break;
        }
        let mut state = mapped.visible.clone();
        for ((slot, domain), digit) in mapped.hidden.iter().zip(&domains).zip(&digits) {
            let Some(Some(value)) = domain.get(*digit) else {
                continue;
            };
            if let Some(object) = state
                .populations
                .get_mut(&slot.object.population)
                .and_then(|objects| objects.get_mut(&slot.object.key))
            {
                object.fields.insert(slot.field.clone(), value.clone());
            }
        }
        states.push(state);
        // The last slot moves fastest.
        let mut at = digits.len();
        loop {
            let Some(previous) = at.checked_sub(1) else {
                break 'assignments;
            };
            at = previous;
            digits[at] += 1;
            if digits[at] < domains[at].len() {
                break;
            }
            digits[at] = 0;
        }
    }
    Ok(states)
}

/// RS-4: the abstract operation `operation` over the evaluated `arguments`,
/// judged against each completion of the mapped post-state from each member
/// of `pre_states`, with the calls the abstract system makes when it expands
/// the member. The members after the step are the accepted completions.
fn judge_operation(
    abstract_system: &ModelSystem<'_>,
    (pre_states, mapped): (&[ModelState], &MappedState),
    operation: &DeclarationKey,
    arguments: Arguments,
    step: &ConcreteStep<'_>,
    abstract_successors: &mut u64,
    passes: impl FnOnce(Taken, Vec<ModelState>) -> StepVerdict,
) -> StepVerdict {
    let completions = match completions(abstract_system, mapped) {
        Ok(completions) => completions,
        Err(verdict) => return verdict,
    };
    let mut next = Vec::new();
    let mut accepted = Vec::new();
    let mut rejected = Vec::new();
    let mut undecided = false;
    let mut order = 0_usize;
    for before in pre_states {
        for post in &completions {
            let judged = match abstract_system.judge_application(&AbstractApplication {
                pre: before,
                receiver: &arguments.receiver,
                operation,
                arguments: &arguments.supplied,
                post,
                result: arguments.result.as_ref(),
            }) {
                Ok(judged) => judged,
                Err(stop) => return stop_verdict(abstract_system, stop),
            };
            *abstract_successors =
                abstract_successors.saturating_add(u64::try_from(judged.len()).unwrap_or(u64::MAX));
            for found in judged {
                let Ok(key) = abstract_system.transition_key(&found.transition) else {
                    return fault("transition-encodes");
                };
                match found.judgement {
                    Judgement::Accepted => {
                        next.push(post.clone());
                        accepted.push(found.transition);
                    }
                    Judgement::Undecided(_) => undecided = true,
                    Judgement::Rejected(rejection) => {
                        rejected.push((key, order, found.transition, rejection));
                    }
                }
                order += 1;
            }
        }
    }
    // With a hidden field a completion that did not decide may be a member of
    // the set after the step, so the set is not known (AX-2).
    if !mapped.hidden.is_empty() && undecided {
        return StepVerdict::Undetermined(Undetermined::UndecidedSuccessor);
    }
    if !accepted.is_empty() {
        let (taken, next) = match (
            canonical(abstract_system, accepted),
            sorted(abstract_system, next),
        ) {
            (Ok(taken), Ok(next)) => (taken, next),
            (Err(verdict), _) | (_, Err(verdict)) => return verdict,
        };
        return passes(Taken::Abstract(taken), next);
    }
    if undecided {
        return StepVerdict::Undetermined(Undetermined::UndecidedSuccessor);
    }
    // A set that comes out empty has no abstract match, whichever candidate
    // and completion rejected (QSpec FR-378).
    if !mapped.hidden.is_empty() {
        return StepVerdict::Fails(RefinementFailure::NoAbstractMatch {
            position: step.position,
        });
    }
    rejected.sort_by(|left, right| left.0.cmp(&right.0).then(left.1.cmp(&right.1)));
    match rejected.into_iter().next() {
        Some((_, _, transition, rejection)) => {
            StepVerdict::Fails(RefinementFailure::AbstractStepRejected {
                position: step.position,
                transition,
                cause: match rejection {
                    Rejection::Precondition { clause } => StepRejection::Precondition { clause },
                    Rejection::Frame { code } => StepRejection::Frame { code },
                    Rejection::Postcondition => StepRejection::Postcondition,
                },
            })
        }
        None => StepVerdict::Fails(RefinementFailure::NoAbstractMatch {
            position: step.position,
        }),
    }
}

/// The canonical state key of `state` in `system`.
fn state_key(
    system: &ModelSystem<'_>,
    state: &ModelState,
) -> Option<qsl_eval::simulation::StateKey> {
    system
        .state_key_of(state)
        .ok()
        .map(|(bytes, _)| qsl_eval::simulation::StateKey::from_canonical_bytes(bytes))
}

/// The verdict of a mapping that gave no state, for the state at `position`,
/// whose state key the evaluation reads.
fn mapping_verdict(
    state_key: impl FnOnce() -> Option<qsl_eval::simulation::StateKey>,
    position: u64,
    failure: MappingFailure,
) -> StepVerdict {
    match failure {
        MappingFailure::Undefined { row, object, cause } => {
            let Some(state_key) = state_key() else {
                return fault("concrete-state-encodes");
            };
            StepVerdict::Undefined(UndefinedEvaluation {
                site: UndefinedSite {
                    position,
                    state_key,
                    locus: UndefinedLocus::Mapping(Box::new(MappingSite { row, object })),
                    transition: None,
                    time: None,
                },
                cause: *cause,
            })
        }
        MappingFailure::Undetermined(undetermined) => {
            StepVerdict::Undetermined(Undetermined::Mapping(Box::new(undetermined)))
        }
        MappingFailure::WrongSystem(_) => fault("the-engine-passed-the-refinements-own-systems"),
        MappingFailure::Fault(_) => fault("mapping-evaluation"),
    }
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;
    use quire_semantic_value::catalog::CatalogCode;

    use super::*;

    /// FR-141 (TC-555): an undetermined row outcome is kept for the
    /// precedence, and any other verdict ends the check at once.
    #[trace("TC-555")]
    #[test]
    fn an_undetermined_row_is_kept_and_any_other_verdict_ends_the_check() {
        let mut soft = Soft::default();
        assert!(soft
            .record(StepVerdict::Undetermined(Undetermined::UndecidedSuccessor))
            .is_ok());
        assert!(soft
            .record(StepVerdict::Fails(RefinementFailure::NoAbstractMatch {
                position: 1
            }))
            .is_err());
        assert!(matches!(
            soft.first(),
            Some(StepVerdict::Undetermined(Undetermined::UndecidedSuccessor))
        ));
    }

    /// FR-141 (TC-555): of the row outcomes recorded the undefined one
    /// outranks an undetermined one recorded before it.
    #[trace("TC-555")]
    #[test]
    fn an_undefined_row_outranks_an_undetermined_one_recorded_first() {
        let undefined = StepVerdict::Undefined(UndefinedEvaluation {
            site: UndefinedSite {
                position: 2,
                state_key: qsl_eval::simulation::StateKey::from_canonical_bytes(b"null".to_vec()),
                locus: UndefinedLocus::Constraint("c".to_owned()),
                transition: None,
                time: None,
            },
            cause: qsl_eval::temporal::UndefinedCause::InsufficientData(
                qsl_eval::temporal::InsufficientData {
                    count: 0,
                    min_count: 1,
                },
            ),
        });
        let mut soft = Soft::default();
        assert!(soft
            .record(StepVerdict::Undetermined(Undetermined::UndecidedSuccessor))
            .is_ok());
        assert!(soft.record(undefined).is_ok());
        assert!(matches!(soft.first(), Some(StepVerdict::Undefined(_))));
    }

    /// FR-141-AC-8 (TC-555 step 4): a `Fault` and a `WrongSystem` from
    /// FR-140 are each a `StepVerdict::Fault`, never `Undefined`,
    /// `Undetermined` or `Fails`, and the state is not even keyed for them.
    #[trace("TC-555", "FR-141-AC-8")]
    #[test]
    fn a_mapping_fault_and_a_wrong_system_are_faults() {
        let code = CatalogCode::new("runtime_invariant", "established-invariant-broken");
        for failure in [
            MappingFailure::Fault(code),
            MappingFailure::WrongSystem(code),
        ] {
            let verdict = mapping_verdict(|| unreachable!("a fault keys no state"), 3, failure);
            assert!(matches!(verdict, StepVerdict::Fault(_)), "{verdict:?}");
        }
    }
}
