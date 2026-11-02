// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-145 (ADR-020 RC-2, RC-3, ADR-018 UE-5): [`replay_refinement_trace`],
//! the replay of a refinement counterexample.
//!
//! It re-executes the concrete steps through the concrete `ModelSystem` as
//! FR-128 does, recomputes the mapped abstract state at every position, and
//! reruns [`check_initial`] and [`check_step`] along the prefix, stopping at
//! the first failure. A failure equal to the recorded one, or an undefined
//! mapping equal to the recorded undefined evaluation, is
//! `reproduced-with-evaluated-witness`; a failure of another kind or at
//! another position, an undetermined check, or no failure at all settles
//! `inconclusive`, `ReplayParity`. Replay needs no model checker and reads no
//! path, environment variable or clock.
//!
//! A lasso (`Divergence` or `AbstractUnfair`, FR-143) is replayed over the
//! prefix and one pass of its loop. Its loop must close in the product, with
//! the history values and candidates it entered with, and be fair under
//! `F_C` as FR-128 reads fairness from the concrete system. The recorded
//! `F_A` constraint must then be enabled at the mapped state of every loop
//! state and taken by no loop step, and the loop is `Divergence` when every
//! step is a `stutter`, `AbstractUnfair` otherwise.
//!
//! What this does not do yet: the facade around it that recompiles the
//! packages and resolves the refinement node by its identity. The recompile is
//! blocked on S4 emitting a refinement, which today is refused as not
//! emitted, so a refinement unit does not compile through the replay request;
//! the caller passes the systems of an already-checked package.

use qsl_eval::simulation::model::{ModelFinding, TimedSystem};
use qsl_eval::simulation::{Limits, TransitionSystem};
use qsl_semantics::model::state::ModelState;
use quire_semantic_value::catalog::Category;
use quire_semantic_value::fairness::FairnessKind;
use quire_semantic_value::model_transition::ModelTransition;

use crate::certificate::simulation::CertifiedEdge;
use crate::execute::ReplayRefusal;
use crate::model_check::counterexample::{
    CounterexampleKind, Lasso, TemporalCounterexample, UndefinedEvaluation, UndefinedLocus,
};
use crate::model_check::limits::{ModelCheckLimit, ReachedLimit};
use crate::model_check::replay::{
    close_loop, disagreement, fault, loop_states_of, no_value_cause, reexecute, refused,
    timed_prefix, unfair_constraint, ModelTraceRefusal, Position,
};
use crate::refinement::{
    abstract_classes, check_initial, check_protocol_initial, check_protocol_state_relations,
    check_protocol_step, check_step, initial_history, map_state, step_history, stop_verdict,
    AssumptionSide, AssumptionTarget, ConcreteStep, ProtocolAssumptionReason,
    ProtocolAssumptionWitness, ProtocolRelationError, ProtocolStateRelation, RefinementFailure,
    RefinementPosition, RefinementSystems, StepVerdict, Taken,
};
use crate::result::DisagreementCause;
use crate::timed::explore::{explore, widen, Closure};
use qsl_semantics::check::CheckedRefinement;

/// FR-145's settlement of a refinement counterexample's replay.
#[derive(Clone, Debug)]
pub enum RefinementTraceSettlement {
    /// `reproduced-with-evaluated-witness`: replay reached the recorded
    /// failure.
    Reproduced {
        /// The failure it reproduced.
        failure: RefinementFailure,
        /// The abstract state each position maps to, from position 0 to the
        /// failing position, the failing one included when it maps.
        mapped: Vec<ModelState>,
    },
    /// `inconclusive`, `ReplayParity`: replay disagrees with the engine.
    Inconclusive(DisagreementCause),
    /// A run budget stopped the replay: V-7 with the limit and its value
    /// (FR-145-AC-6), never `ReplayParity`.
    Stopped(ReachedLimit),
}

/// FR-145's settlement of an assumption-relation witness.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProtocolAssumptionReplaySettlement {
    /// The paired prefix or lasso and its source-side predicates reproduced.
    Reproduced,
    /// Replay disagreed with the typed witness.
    Inconclusive(DisagreementCause),
    /// A replay expansion reached a caller limit.
    Stopped(ReachedLimit),
}

/// FR-145: replay one typed FR-148 assumption-relation witness over the same
/// admitted concrete and protocol systems used by the checker. The replay
/// re-executes both sides of every paired edge, checks the recorded loop
/// closes, and rechecks the checked predicate locus for a finite
/// undetermined-relation witness.
pub fn replay_protocol_assumption_witness(
    refinement: &CheckedRefinement,
    systems: &RefinementSystems<'_>,
    witness: &ProtocolAssumptionWitness,
) -> Result<ProtocolAssumptionReplaySettlement, ReplayRefusal> {
    let Some(protocol) = systems.abstract_protocol.as_ref() else {
        return Ok(ProtocolAssumptionReplaySettlement::Inconclusive(
            disagreement(Category::Success),
        ));
    };
    if !assumption_sources_match(refinement, protocol, &witness.identities)? {
        return Ok(ProtocolAssumptionReplaySettlement::Inconclusive(
            disagreement(Category::Success),
        ));
    }
    let Some(mut concrete) = systems.concrete.initial().into_iter().find(|state| {
        systems
            .concrete
            .state_key(state)
            .is_ok_and(|(_, key)| key == witness.path.concrete_initial)
    }) else {
        return Ok(ProtocolAssumptionReplaySettlement::Inconclusive(
            disagreement(Category::Success),
        ));
    };
    let Some((mut candidate, initial_closure)) = protocol_initial_closure(
        refinement,
        systems,
        protocol,
        &concrete,
        &witness.path.protocol_initial,
    )?
    else {
        return Ok(ProtocolAssumptionReplaySettlement::Inconclusive(
            disagreement(Category::Success),
        ));
    };

    let mut paired_keys = vec![(
        systems
            .concrete
            .state_key(&concrete)
            .map_err(|_| fault("assumption-replay-concrete-state-has-a-key"))?
            .1,
        protocol
            .state_key(&candidate)
            .map_err(|_| fault("assumption-replay-protocol-state-has-a-key"))?
            .0,
    )];
    let mut concrete_states = vec![concrete.clone()];
    let mut protocol_states = vec![candidate.clone()];
    let mut paired_transitions =
        Vec::with_capacity(witness.path.steps.len() + initial_closure.len());
    for (transition, next) in initial_closure {
        paired_transitions.push((None, Some(transition)));
        concrete_states.push(concrete.clone());
        candidate = next;
        protocol_states.push(candidate.clone());
        paired_keys.push((
            systems
                .concrete
                .state_key(&concrete)
                .map_err(|_| fault("assumption-replay-concrete-state-has-a-key"))?
                .1,
            protocol
                .state_key(&candidate)
                .map_err(|_| fault("assumption-replay-protocol-state-has-a-key"))?
                .0,
        ));
    }
    let initial_closure_len = paired_transitions.len();
    for step in &witness.path.steps {
        let concrete_transition = step.concrete.clone();
        if let Some(transition) = &step.concrete {
            let expansion = systems
                .concrete
                .successors(&concrete)
                .map_err(|_| fault("assumption-replay-concrete-expansion-succeeds"))?;
            let Some((_, next)) = expansion.successors.into_iter().find(|(enabled, next)| {
                enabled == transition
                    && systems
                        .concrete
                        .state_key(next)
                        .is_ok_and(|(_, key)| key == step.concrete_post)
            }) else {
                return Ok(ProtocolAssumptionReplaySettlement::Inconclusive(
                    disagreement(Category::Success),
                ));
            };
            concrete = next;
        } else if systems
            .concrete
            .state_key(&concrete)
            .map_or(true, |(_, key)| key != step.concrete_post)
        {
            return Ok(ProtocolAssumptionReplaySettlement::Inconclusive(
                disagreement(Category::Success),
            ));
        }

        if let Some(identity) = &step.protocol {
            let expansion = protocol
                .successors(&candidate)
                .map_err(|_| fault("assumption-replay-protocol-expansion-succeeds"))?;
            let Some((_, next)) = expansion.successors.into_iter().find(|(transition, next)| {
                protocol
                    .transition_key(transition)
                    .is_ok_and(|key| key == *identity)
                    && protocol
                        .state_key(next)
                        .is_ok_and(|(key, _)| key == step.protocol_post)
            }) else {
                return Ok(ProtocolAssumptionReplaySettlement::Inconclusive(
                    disagreement(Category::Success),
                ));
            };
            candidate = next;
        } else if protocol
            .state_key(&candidate)
            .map_or(true, |(key, _)| key != step.protocol_post)
        {
            return Ok(ProtocolAssumptionReplaySettlement::Inconclusive(
                disagreement(Category::Success),
            ));
        }
        let protocol_transition = step.protocol.as_ref().and_then(|identity| {
            // The transition has already been selected by identity above.
            protocol_states
                .last()
                .and_then(|before| protocol.successors(before).ok())
                .and_then(|expansion| {
                    expansion
                        .successors
                        .into_iter()
                        .find_map(|(transition, next)| {
                            (protocol.transition_key(&transition).ok().as_ref() == Some(identity)
                                && protocol
                                    .state_key(&next)
                                    .is_ok_and(|(key, _)| key == step.protocol_post))
                            .then_some(transition)
                        })
                })
        });
        if step.protocol.is_some() && protocol_transition.is_none() {
            return Ok(ProtocolAssumptionReplaySettlement::Inconclusive(
                disagreement(Category::Success),
            ));
        }
        paired_transitions.push((concrete_transition, protocol_transition));
        concrete_states.push(concrete.clone());
        protocol_states.push(candidate.clone());
        paired_keys.push((
            systems
                .concrete
                .state_key(&concrete)
                .map_err(|_| fault("assumption-replay-concrete-state-has-a-key"))?
                .1,
            protocol
                .state_key(&candidate)
                .map_err(|_| fault("assumption-replay-protocol-state-has-a-key"))?
                .0,
        ));
    }

    match witness.reason {
        ProtocolAssumptionReason::Strengthened => {
            let Some(loop_entry) = witness.path.loop_entry else {
                return Ok(ProtocolAssumptionReplaySettlement::Inconclusive(
                    disagreement(Category::Success),
                ));
            };
            let loop_entry = loop_entry.saturating_add(initial_closure_len);
            if paired_keys.get(loop_entry) != paired_keys.last() {
                return Ok(ProtocolAssumptionReplaySettlement::Inconclusive(
                    disagreement(Category::Success),
                ));
            }
            let loop_end = paired_transitions.len();
            if loop_entry >= loop_end
                || !assumption_lasso_matches(
                    refinement,
                    systems,
                    protocol,
                    &witness.identities,
                    &concrete_states,
                    &protocol_states,
                    &paired_transitions,
                    loop_entry,
                    loop_end,
                )?
            {
                return Ok(ProtocolAssumptionReplaySettlement::Inconclusive(
                    disagreement(Category::Success),
                ));
            }
            Ok(ProtocolAssumptionReplaySettlement::Reproduced)
        }
        ProtocolAssumptionReason::RelationUndetermined => {
            if witness.path.loop_entry.is_some() {
                return Ok(ProtocolAssumptionReplaySettlement::Inconclusive(
                    disagreement(Category::Success),
                ));
            }
            let Some(locus) = witness.undecided_locus.as_deref() else {
                return Ok(ProtocolAssumptionReplaySettlement::Inconclusive(
                    disagreement(Category::Success),
                ));
            };
            let expansion = systems
                .concrete
                .successors(&concrete)
                .map_err(|_| fault("assumption-replay-undetermined-expansion-succeeds"))?;
            let reproduced = expansion.findings.iter().any(|finding| {
                matches!(finding, ModelFinding::ContractUndetermined { clause, .. }
                    if locus == format!("ContractUndetermined:{clause}"))
            });
            if reproduced {
                Ok(ProtocolAssumptionReplaySettlement::Reproduced)
            } else {
                Ok(ProtocolAssumptionReplaySettlement::Inconclusive(
                    disagreement(Category::Success),
                ))
            }
        }
    }
}

fn assumption_sources_match(
    refinement: &CheckedRefinement,
    protocol: &crate::refinement::AbstractProtocolSystem<'_>,
    identities: &[crate::refinement::AssumptionIdentity],
) -> Result<bool, ReplayRefusal> {
    for identity in identities {
        if identity.granularity == quire_semantic_value::fairness::FairnessGranularity::Each
            && identity.selected.is_none()
        {
            return Ok(false);
        }
        let matches = match (&identity.side, &identity.target) {
            (AssumptionSide::Implementation, AssumptionTarget::Model(assumption)) => {
                identity.origin == crate::refinement::AssumptionOrigin::Authored
                    && identity.kind == assumption.kind()
                    && identity.granularity == assumption.granularity()
                    && refinement
                        .concrete_operation_fairness()
                        .any(|checked| checked == assumption)
            }
            (AssumptionSide::Specification, AssumptionTarget::Protocol(assumption)) => {
                let expected_origin = match assumption.origin() {
                    qsl_eval::simulation::protocol::FairnessOrigin::Authored => {
                        crate::refinement::AssumptionOrigin::Authored
                    }
                    qsl_eval::simulation::protocol::FairnessOrigin::Scheduler => {
                        crate::refinement::AssumptionOrigin::Scheduler
                    }
                    qsl_eval::simulation::protocol::FairnessOrigin::Memory => {
                        crate::refinement::AssumptionOrigin::Memory
                    }
                };
                identity.origin == expected_origin
                    && identity.kind == assumption.kind()
                    && identity.granularity == assumption.granularity()
                    && protocol
                        .environment_fairness()
                        .map_err(|_| fault("assumption-replay-protocol-fairness-resolves"))?
                        .contains(assumption)
            }
            (
                AssumptionSide::Specification,
                AssumptionTarget::RequiredTemporal {
                    clause,
                    constraint,
                    fairness,
                },
            ) => {
                identity.origin == crate::refinement::AssumptionOrigin::Authored
                    && identity.kind == constraint.kind()
                    && identity.granularity == constraint.granularity()
                    && fairness.origin() == qsl_eval::simulation::protocol::FairnessOrigin::Authored
                    && protocol
                        .required_temporal_fairness()
                        .iter()
                        .any(|required| {
                            required.clause == *clause && required.constraint == *constraint
                        })
                    && protocol
                        .assumption_fairness()
                        .map_err(|_| fault("assumption-replay-protocol-fairness-resolves"))?
                        .iter()
                        .any(|member| {
                            member.required_temporal.as_ref().is_some_and(|required| {
                                required.clause == *clause
                                    && required.constraint == *constraint
                                    && member.fairness == *fairness
                            })
                        })
            }
            _ => false,
        };
        if !matches {
            return Ok(false);
        }
    }
    Ok(!identities.is_empty())
}

fn protocol_initial_closure(
    refinement: &CheckedRefinement,
    systems: &RefinementSystems<'_>,
    protocol: &crate::refinement::AbstractProtocolSystem<'_>,
    concrete: &qsl_eval::simulation::model::ModelSimState,
    target: &[u8],
) -> Result<
    Option<(
        crate::refinement::ProtocolCandidate,
        Vec<(
            qsl_eval::simulation::protocol::ProtocolTransition,
            crate::refinement::ProtocolCandidate,
        )>,
    )>,
    ReplayRefusal,
> {
    use std::collections::{BTreeSet, VecDeque};

    let history = initial_history(refinement, &systems.concrete, concrete.state());
    let Ok(mapped) = map_state(
        refinement,
        &systems.concrete,
        &systems.abstract_system,
        concrete.state(),
        &history,
    ) else {
        return Ok(None);
    };
    let mut visible = mapped.visible;
    for objects in visible.populations.values_mut() {
        for object in objects.values_mut() {
            if let Some(row) = refinement
                .objects()
                .iter()
                .find(|row| row.abstract_type == object.type_identity)
            {
                for field in &row.hidden {
                    object.fields.remove(field);
                }
            }
        }
    }
    let (mapped_key, _) = systems
        .abstract_system
        .state_key_of(&visible)
        .map_err(|_| fault("assumption-replay-mapped-initial-has-a-key"))?;
    let internal: BTreeSet<_> = refinement
        .abstract_internal()
        .iter()
        .map(|class| class.spelling())
        .collect();
    let mut queue = VecDeque::new();
    let mut seen = BTreeSet::new();
    for root in protocol.initial() {
        let Some(model) = protocol.model_state(&root) else {
            continue;
        };
        let mut root_visible = model.state().clone();
        for objects in root_visible.populations.values_mut() {
            for object in objects.values_mut() {
                if let Some(row) = refinement
                    .objects()
                    .iter()
                    .find(|row| row.abstract_type == object.type_identity)
                {
                    for field in &row.hidden {
                        object.fields.remove(field);
                    }
                }
            }
        }
        let Ok((key, _)) = systems.abstract_system.state_key_of(&root_visible) else {
            continue;
        };
        if key != mapped_key {
            continue;
        }
        let Ok((protocol_key, _)) = protocol.state_key(&root) else {
            continue;
        };
        if !seen.insert(protocol_key) {
            continue;
        }
        queue.push_back((root.clone(), root, Vec::new()));
    }
    let mut expanded = 0_u64;
    while let Some((root, state, path)) = queue.pop_front() {
        let (key, _) = protocol
            .state_key(&state)
            .map_err(|_| fault("assumption-replay-protocol-state-has-a-key"))?;
        if key == target {
            return Ok(Some((root, path)));
        }
        let expansion = protocol
            .successors(&state)
            .map_err(|_| fault("assumption-replay-protocol-initial-expands"))?;
        for (transition, next) in expansion.successors {
            let Some(class) = crate::refinement::protocol_transition_class(protocol, &transition)
            else {
                continue;
            };
            if !internal.contains(class.as_str()) {
                continue;
            }
            let (next_key, _) = protocol
                .state_key(&next)
                .map_err(|_| fault("assumption-replay-protocol-state-has-a-key"))?;
            if !seen.insert(next_key) {
                continue;
            }
            expanded = expanded.saturating_add(1);
            if expanded > protocol.model().max_candidates() {
                return Ok(None);
            }
            let mut next_path = path.clone();
            next_path.push((transition, next.clone()));
            queue.push_back((root.clone(), next, next_path));
        }
    }
    Ok(None)
}

fn assumption_lasso_matches(
    refinement: &CheckedRefinement,
    systems: &RefinementSystems<'_>,
    protocol: &crate::refinement::AbstractProtocolSystem<'_>,
    identities: &[crate::refinement::AssumptionIdentity],
    concrete_states: &[qsl_eval::simulation::model::ModelSimState],
    protocol_states: &[crate::refinement::ProtocolCandidate],
    transitions: &[(
        Option<ModelTransition>,
        Option<qsl_eval::simulation::protocol::ProtocolTransition>,
    )],
    loop_entry: usize,
    loop_end: usize,
) -> Result<bool, ReplayRefusal> {
    let mut implementation = false;
    // An empty protocol environment-assumption set is vacuously satisfied.
    let mut specification = true;
    for identity in identities {
        let (enabled, taken) = match (&identity.side, &identity.target) {
            (AssumptionSide::Implementation, AssumptionTarget::Model(assumption)) => {
                if !refinement
                    .concrete_operation_fairness()
                    .any(|checked| checked == assumption)
                {
                    return Ok(false);
                }
                let operation = assumption.operation();
                let Some(text) = systems
                    .concrete
                    .operation_text(operation.declaring_type(), operation.name())
                else {
                    return Ok(false);
                };
                let mut enabled = Vec::new();
                let mut taken = false;
                for index in loop_entry..loop_end {
                    let Some(state) = concrete_states.get(index) else {
                        return Ok(false);
                    };
                    let expansion = systems
                        .concrete
                        .successors(state)
                        .map_err(|_| fault("assumption-replay-concrete-lasso-expands"))?;
                    enabled.push(expansion.successors.iter().any(|(transition, _)| {
                        transition.operation() == text
                            && (identity.granularity
                                == quire_semantic_value::fairness::FairnessGranularity::Whole
                                || matches!(identity.selected,
                                    Some(crate::refinement::AssumptionSelectedMember::Model(ref selected))
                                        if selected == transition))
                    }));
                    taken |= transitions.get(index).is_some_and(|(edge, _)| {
                        edge.as_ref().is_some_and(|transition| {
                            transition.operation() == text
                                && (identity.granularity
                                    == quire_semantic_value::fairness::FairnessGranularity::Whole
                                    || matches!(identity.selected,
                                        Some(crate::refinement::AssumptionSelectedMember::Model(ref selected))
                                            if selected == transition))
                        })
                    });
                }
                (enabled, taken)
            }
            (
                AssumptionSide::Specification,
                AssumptionTarget::Protocol(assumption)
                | AssumptionTarget::RequiredTemporal {
                    fairness: assumption,
                    ..
                },
            ) => {
                if !protocol
                    .assumption_fairness()
                    .map_err(|_| fault("assumption-replay-protocol-fairness-resolves"))?
                    .iter()
                    .any(|member| member.fairness == *assumption)
                {
                    return Ok(false);
                }
                let mut enabled = Vec::new();
                let mut taken = false;
                for index in loop_entry..loop_end {
                    let Some(state) = protocol_states.get(index) else {
                        return Ok(false);
                    };
                    let expansion = protocol
                        .successors(state)
                        .map_err(|_| fault("assumption-replay-protocol-lasso-expands"))?;
                    enabled.push(expansion.successors.iter().any(|(transition, _)| {
                        assumption.class_of(transition).is_some_and(|class| {
                            identity.granularity
                                == quire_semantic_value::fairness::FairnessGranularity::Whole
                                || matches!(identity.selected,
                                    Some(crate::refinement::AssumptionSelectedMember::Protocol(ref selected))
                                        if selected == &class)
                        })
                    }));
                    taken |= transitions.get(index).is_some_and(|(_, edge)| {
                        edge.as_ref().is_some_and(|transition| {
                            assumption.class_of(transition).is_some_and(|class| {
                                identity.granularity
                                    == quire_semantic_value::fairness::FairnessGranularity::Whole
                                    || matches!(identity.selected,
                                        Some(crate::refinement::AssumptionSelectedMember::Protocol(ref selected))
                                            if selected == &class)
                            })
                        })
                    });
                }
                (enabled, taken)
            }
            _ => return Ok(false),
        };
        let satisfied = match identity.kind {
            FairnessKind::Weak => !enabled.iter().all(|enabled| *enabled) || taken,
            FairnessKind::Strong => !enabled.iter().any(|enabled| *enabled) || taken,
        };
        match identity.side {
            AssumptionSide::Implementation => {
                implementation = true;
                if satisfied {
                    return Ok(false);
                }
            }
            AssumptionSide::Specification => {
                specification = true;
                if !satisfied {
                    return Ok(false);
                }
            }
        }
    }
    Ok(implementation && specification)
}

/// What running the checks along the prefix found.
enum Found {
    /// Every position passed.
    Nothing,
    /// A check failed at `position`.
    Failed(RefinementFailure),
    /// A mapping row or argument was undefined.
    Undefined(Box<UndefinedEvaluation>),
    /// A check could not decide.
    Undetermined,
    /// A budget stopped a check.
    Stopped(ReachedLimit),
}

/// FR-145: replay `payload`, a refinement counterexample of `refinement`
/// over `systems`.
///
/// # Errors
///
/// [`ReplayRefusal`] when the prefix does not re-execute (FR-128's refusals:
/// an `initial_state` out of range, a step not enabled, a post-state no
/// successor has, a loop that does not close), the payload names no
/// refinement failure, a lasso's history values or candidates differ at the
/// end of its loop from its entry, a lasso is unfair under `F_C`, or an
/// internal fault occurs.
pub fn replay_refinement_trace(
    refinement: &CheckedRefinement,
    systems: &RefinementSystems<'_>,
    payload: &TemporalCounterexample,
) -> Result<RefinementTraceSettlement, ReplayRefusal> {
    let Some(recorded) = payload.refinement.as_ref() else {
        return Err(refused(ModelTraceRefusal::NoRefinementFailure));
    };
    let Some(positions) = reexecute(&systems.concrete, payload)? else {
        return Ok(RefinementTraceSettlement::Inconclusive(no_value_cause()));
    };
    if let Some(lasso) = payload.lasso {
        close_loop(&systems.concrete, &positions, lasso)?;
    }
    settle_positions(refinement, systems, payload, recorded, &positions)
}

/// QSpec FR-430: replay `payload`, a refinement counterexample of
/// `refinement` whose concrete model is timed, over `systems`. The prefix
/// is re-executed over time by FR-237's rules, each step after its exact
/// delay; the checks then run along it as [`replay_refinement_trace`]
/// runs them, a delay taking no abstract step. A lasso (FR-143) is replayed
/// as [`replay_refinement_trace`] replays one, its loop closed over time
/// with a positive total delay (ADR-026 TS-4). A reproduced failing step
/// stands only where the behaviour goes on from it with time diverging:
/// the valuation it reaches has a time-divergent continuation (ADR-026
/// TV-1 and TS-4), which a fresh exploration bounded by `exploration`
/// finds; one with none settles `inconclusive`.
///
/// # Errors
///
/// [`ReplayRefusal`] when the concrete subject is not timed, the prefix
/// does not re-execute over time or a lasso's loop does not close over
/// time (FR-237's refusals), the payload names no refinement failure, a
/// lasso's history or candidates drift, it is unfair under `F_C`, or an
/// internal fault occurs.
pub fn replay_timed_refinement_trace(
    refinement: &CheckedRefinement,
    systems: &RefinementSystems<'_>,
    payload: &TemporalCounterexample,
    exploration: &Limits,
) -> Result<RefinementTraceSettlement, ReplayRefusal> {
    let Some(recorded) = payload.refinement.as_ref() else {
        return Err(refused(ModelTraceRefusal::NoRefinementFailure));
    };
    let timed = TimedSystem::new(&systems.concrete, refinement.concrete_model())
        .map_err(|refusal| refused(ModelTraceRefusal::TimedSubject(refusal)))?;
    let Some((positions, states)) = timed_prefix(&timed, payload)? else {
        return Ok(RefinementTraceSettlement::Inconclusive(no_value_cause()));
    };
    let settlement = settle_positions(refinement, systems, payload, recorded, &positions)?;
    let RefinementTraceSettlement::Reproduced { failure, .. } = &settlement else {
        return Ok(settlement);
    };
    // A failing initial state is the failing step of the empty prefix: its
    // post-state is the initial state (QSpec FR-430).
    let Some(post) = failure
        .position()
        .and_then(|position| usize::try_from(position).ok())
    else {
        return Ok(settlement);
    };
    let Some(point) = states.get(post) else {
        return Err(fault("a-failing-step-has-a-post-state"));
    };
    Ok(match explore(&timed, point, exploration) {
        Ok(Closure::Divergent) => settlement,
        Ok(Closure::Locked) => {
            RefinementTraceSettlement::Inconclusive(disagreement(Category::Success))
        }
        Ok(Closure::Stopped { states, .. }) => RefinementTraceSettlement::Stopped(if states {
            ReachedLimit {
                limit: ModelCheckLimit::MaxStates,
                value: widen(exploration.max_states),
            }
        } else {
            ReachedLimit {
                limit: ModelCheckLimit::MaxTransitions,
                value: widen(exploration.max_transitions),
            }
        }),
        Err(_) => RefinementTraceSettlement::Inconclusive(no_value_cause()),
    })
}

/// The settlement of `payload` along its re-executed `positions`: the
/// checks run at each position, stopping at the first failure.
fn settle_positions(
    refinement: &CheckedRefinement,
    systems: &RefinementSystems<'_>,
    payload: &TemporalCounterexample,
    recorded: &RefinementFailure,
    positions: &[Position],
) -> Result<RefinementTraceSettlement, ReplayRefusal> {
    let mut mapped = Vec::new();
    let mut previous: Option<RefinementPosition> = None;
    // The position and the abstract steps taken at each step, for a lasso.
    let mut trail = Vec::new();
    let mut taken_by = Vec::new();
    let mut found = Found::Nothing;
    for (index, position) in positions.iter().enumerate() {
        let verdict = match (&previous, index.checked_sub(1)) {
            (Some(pre), Some(step)) => {
                let Some(taken) = payload.prefix.get(step) else {
                    return Err(fault("a-position-has-a-step"));
                };
                let step = ConcreteStep {
                    position: u64::try_from(index).unwrap_or(u64::MAX),
                    transition: Some(&taken.transition),
                    post: &position.state,
                };
                match systems.abstract_protocol.as_ref() {
                    Some(protocol) => {
                        check_protocol_step(
                            refinement,
                            &systems.concrete,
                            &systems.abstract_system,
                            protocol,
                            pre,
                            &step,
                        )
                        .verdict
                    }
                    None => {
                        check_step(
                            refinement,
                            &systems.concrete,
                            &systems.abstract_system,
                            pre,
                            &step,
                        )
                        .verdict
                    }
                }
            }
            _ => match systems.abstract_protocol.as_ref() {
                Some(protocol) => {
                    check_protocol_initial(
                        refinement,
                        &systems.concrete,
                        &systems.abstract_system,
                        protocol,
                        payload.initial_state,
                        position.state.state(),
                    )
                    .verdict
                }
                None => check_initial(
                    refinement,
                    &systems.concrete,
                    &systems.abstract_system,
                    payload.initial_state,
                    position.state.state(),
                ),
            },
        };
        match verdict {
            StepVerdict::Passes { post, taken } => {
                mapped.extend(post.abstract_states.first().cloned());
                trail.push(post.clone());
                if index > 0 {
                    taken_by.push(taken);
                }
                previous = Some(post.clone());
                if let Some(protocol) = systems.abstract_protocol.as_ref() {
                    let state_edges = match protocol_relation_edges(
                        refinement,
                        systems,
                        &post,
                        &position.state,
                        u64::try_from(index).unwrap_or(u64::MAX),
                    )? {
                        Ok(edges) => edges,
                        Err(relation_found) => {
                            found = relation_found;
                            break;
                        }
                    };
                    match check_protocol_state_relations(
                        refinement,
                        &systems.concrete,
                        protocol,
                        &post,
                        &position.state,
                        &state_edges,
                        u64::try_from(index).unwrap_or(u64::MAX),
                    ) {
                        Ok(ProtocolStateRelation::Decided {
                            failure: Some(failure),
                            ..
                        }) => {
                            found = Found::Failed(failure);
                            break;
                        }
                        Ok(ProtocolStateRelation::Undetermined { .. }) => {
                            found = Found::Undetermined;
                            break;
                        }
                        Ok(ProtocolStateRelation::Decided { failure: None, .. }) => {}
                        Err(ProtocolRelationError::Expansion(stop)) => {
                            match stop_verdict(protocol.model(), stop) {
                                StepVerdict::Stopped(reached) => found = Found::Stopped(reached),
                                StepVerdict::Undetermined(_) => found = Found::Undetermined,
                                _ => return Err(fault("protocol-relation-expansion")),
                            }
                            break;
                        }
                        Err(ProtocolRelationError::Fault(_)) => {
                            return Err(fault("protocol-state-relation"));
                        }
                    }
                }
            }
            StepVerdict::Fails(failure) => {
                found = Found::Failed(failure);
                // The failing position's history is the one its step gives.
                let history = match (&previous, payload.prefix.get(index.saturating_sub(1))) {
                    (Some(pre), Some(taken)) if index > 0 => step_history(
                        refinement,
                        &systems.concrete,
                        &pre.history,
                        &pre.concrete,
                        &taken.transition,
                        &position.state,
                    )
                    .ok(),
                    _ => Some(initial_history(
                        refinement,
                        &systems.concrete,
                        position.state.state(),
                    )),
                };
                mapped.extend(history.and_then(|history| {
                    map_state(
                        refinement,
                        &systems.concrete,
                        &systems.abstract_system,
                        position.state.state(),
                        &history,
                    )
                    .ok()
                    .map(|state| state.visible)
                }));
                break;
            }
            StepVerdict::Undefined(evaluation) => {
                found = Found::Undefined(Box::new(evaluation));
                break;
            }
            StepVerdict::Undetermined(_) => {
                found = Found::Undetermined;
                break;
            }
            StepVerdict::Stopped(reached) => {
                found = Found::Stopped(reached);
                break;
            }
            StepVerdict::Fault(_) => return Err(fault("refinement-step")),
        }
    }
    if let Some(lasso) = payload.lasso {
        if let Some(RefinementFailure::Divergence {
            protocol_class: Some(class),
            ..
        }) = payload.refinement.as_ref()
        {
            return replay_protocol_divergence(
                refinement, systems, payload, lasso, positions, mapped, class,
            );
        }
        let walk = Walk {
            positions,
            trail,
            taken_by,
        };
        return replay_lasso(refinement, systems, payload, lasso, walk, found, mapped);
    }
    Ok(match found {
        Found::Failed(failure)
            if payload.kind == CounterexampleKind::Formula && *recorded == failure =>
        {
            RefinementTraceSettlement::Reproduced { failure, mapped }
        }
        Found::Failed(_) => {
            RefinementTraceSettlement::Inconclusive(disagreement(Category::Violation))
        }
        Found::Undefined(evaluation) => match (&payload.kind, &evaluation.site.locus) {
            (CounterexampleKind::UndefinedEvaluation(claimed), UndefinedLocus::Mapping(site))
                if claimed == evaluation.as_ref()
                    && *recorded
                        == (RefinementFailure::Undefined {
                            position: evaluation.site.position,
                            row: site.row.clone(),
                        }) =>
            {
                RefinementTraceSettlement::Reproduced {
                    failure: recorded.clone(),
                    mapped,
                }
            }
            _ => RefinementTraceSettlement::Inconclusive(disagreement(Category::Violation)),
        },
        Found::Undetermined => RefinementTraceSettlement::Inconclusive(no_value_cause()),
        Found::Stopped(reached) => RefinementTraceSettlement::Stopped(reached),
        Found::Nothing => RefinementTraceSettlement::Inconclusive(disagreement(Category::Success)),
    })
}

/// Recompute the outgoing edges needed by FR-148's state-level relation at
/// one replay position. The product checker only runs these relations after
/// every outgoing step rule has passed, so replay applies the same ordering.
fn protocol_relation_edges(
    refinement: &CheckedRefinement,
    systems: &RefinementSystems<'_>,
    position: &RefinementPosition,
    concrete_state: &qsl_eval::simulation::model::ModelSimState,
    position_index: u64,
) -> Result<Result<Vec<CertifiedEdge>, Found>, ReplayRefusal> {
    let Some(protocol) = systems.abstract_protocol.as_ref() else {
        return Ok(Ok(Vec::new()));
    };
    let expansion = match systems.concrete.successors(concrete_state) {
        Ok(expansion) => expansion,
        Err(stop) => {
            return Ok(match stop_verdict(&systems.concrete, stop) {
                StepVerdict::Stopped(reached) => Err(Found::Stopped(reached)),
                StepVerdict::Undetermined(_) => Err(Found::Undetermined),
                _ => return Err(fault("protocol-replay-concrete-expansion")),
            });
        }
    };
    if expansion
        .findings
        .iter()
        .any(|finding| matches!(finding, ModelFinding::ContractUndetermined { .. }))
    {
        return Ok(Err(Found::Undetermined));
    }
    let mut edges = Vec::with_capacity(expansion.successors.len().max(1));
    for (transition, post) in expansion.successors {
        let verdict = check_protocol_step(
            refinement,
            &systems.concrete,
            &systems.abstract_system,
            protocol,
            position,
            &ConcreteStep {
                position: position_index.saturating_add(1),
                transition: Some(&transition),
                post: &post,
            },
        )
        .verdict;
        match verdict {
            StepVerdict::Passes { taken, .. } => edges.push(CertifiedEdge {
                transition: Some(transition),
                post: 0,
                taken,
            }),
            StepVerdict::Fails(failure) => return Ok(Err(Found::Failed(failure))),
            StepVerdict::Undefined(evaluation) => {
                return Ok(Err(Found::Undefined(Box::new(evaluation))));
            }
            StepVerdict::Undetermined(_) => return Ok(Err(Found::Undetermined)),
            StepVerdict::Stopped(reached) => return Ok(Err(Found::Stopped(reached))),
            StepVerdict::Fault(_) => return Err(fault("protocol-replay-step")),
        }
    }
    if edges.is_empty() {
        edges.push(CertifiedEdge {
            transition: None,
            post: 0,
            taken: Taken::Stutter,
        });
    }
    Ok(Ok(edges))
}

/// What replay recomputed along a lasso's prefix and one pass of its loop.
struct Walk<'p> {
    /// The concrete positions, re-executed.
    positions: &'p [Position],
    /// The refinement position at each concrete position.
    trail: Vec<RefinementPosition>,
    /// The abstract steps the step leaving each position took.
    taken_by: Vec<Taken>,
}

/// The settlement of a lasso whose prefix and loop `check_step` has run along:
/// `found` is what the checks found, `walk` what they recomputed.
fn replay_lasso(
    refinement: &CheckedRefinement,
    systems: &RefinementSystems<'_>,
    payload: &TemporalCounterexample,
    lasso: Lasso,
    mut walk: Walk<'_>,
    found: Found,
    mapped: Vec<ModelState>,
) -> Result<RefinementTraceSettlement, ReplayRefusal> {
    match found {
        Found::Nothing => {}
        Found::Failed(_) | Found::Undefined(_) => {
            return Ok(RefinementTraceSettlement::Inconclusive(disagreement(
                Category::Violation,
            )))
        }
        Found::Undetermined => {
            return Ok(RefinementTraceSettlement::Inconclusive(no_value_cause()))
        }
        Found::Stopped(reached) => return Ok(RefinementTraceSettlement::Stopped(reached)),
    }
    let (Some(RefinementFailure::Divergence {
        constraint: Some(constraint),
        protocol_class: None,
    })
    | Some(RefinementFailure::AbstractUnfair { constraint })) = payload.refinement.as_ref()
    else {
        return Err(refused(ModelTraceRefusal::NoRefinementFailure));
    };
    let steps = walk.positions.len() - 1;
    // The terminal stutter step is one more step of the product.
    if lasso.terminal_stutter {
        let (Some(last), Some(position)) = (walk.trail.last(), walk.positions.last()) else {
            return Err(fault("a-lasso-has-a-position"));
        };
        let verdict = check_step(
            refinement,
            &systems.concrete,
            &systems.abstract_system,
            last,
            &ConcreteStep {
                position: u64::try_from(steps + 1).unwrap_or(u64::MAX),
                transition: None,
                post: &position.state,
            },
        )
        .verdict;
        match verdict {
            StepVerdict::Passes { post, taken } => {
                walk.trail.push(post);
                walk.taken_by.push(taken);
            }
            StepVerdict::Stopped(reached) => {
                return Ok(RefinementTraceSettlement::Stopped(reached))
            }
            StepVerdict::Fault(_) => return Err(fault("refinement-step")),
            StepVerdict::Fails(_) | StepVerdict::Undefined(_) | StepVerdict::Undetermined(_) => {
                return Ok(RefinementTraceSettlement::Inconclusive(disagreement(
                    Category::Violation,
                )))
            }
        }
    }
    let (Some(entry), Some(end)) = (walk.trail.get(lasso.entry), walk.trail.last()) else {
        return Err(fault("a-lasso-has-an-entry-and-an-end"));
    };
    if !same_product_state(systems, entry, end)? {
        return Err(refused(ModelTraceRefusal::LoopDrifts {
            entry: lasso.entry,
        }));
    }
    let Some(loop_states) = loop_states_of(&systems.concrete, payload, walk.positions, lasso)?
    else {
        return Ok(RefinementTraceSettlement::Inconclusive(no_value_cause()));
    };
    if let Some(constraint) = unfair_constraint(
        &systems.concrete,
        &loop_states,
        lasso,
        &refinement
            .concrete_operation_fairness()
            .cloned()
            .collect::<Vec<_>>(),
    )? {
        return Err(ReplayRefusal::UnfairLasso { constraint });
    }
    // A strong constraint is outside the weak reading the engine writes
    // lassos by, so no lasso of one is replayed as a reproduction.
    let Some(index) = refinement
        .abstract_operation_fairness()
        .position(|abstract_constraint| {
            abstract_constraint == constraint && abstract_constraint.kind() == FairnessKind::Weak
        })
    else {
        return Ok(RefinementTraceSettlement::Inconclusive(disagreement(
            Category::Success,
        )));
    };
    // The loop's states are the positions from its entry; a terminal stutter
    // loop is the one state it stands at.
    let from = lasso.entry;
    let (Some(loop_trail), Some(loop_taken)) = (walk.trail.get(from..), walk.taken_by.get(from..))
    else {
        return Err(fault("a-lasso-loop-is-in-the-trail"));
    };
    let loop_trail = &loop_trail[..loop_taken.len()];
    let mut enabled = Vec::with_capacity(loop_trail.len());
    for position in loop_trail {
        match abstract_enabled(systems, position)? {
            Enabledness::Enabled(here) => enabled.push(here),
            Enabledness::Undecided => {
                return Ok(RefinementTraceSettlement::Inconclusive(no_value_cause()))
            }
            Enabledness::Stopped(reached) => {
                return Ok(RefinementTraceSettlement::Stopped(reached))
            }
        }
    }
    let identities: Vec<ModelTransition> = enabled
        .iter()
        .flatten()
        .chain(loop_taken.iter().flat_map(|taken| match taken {
            Taken::Abstract(transitions) => transitions.as_slice(),
            Taken::Stutter => &[],
            Taken::Protocol(_) => &[],
            Taken::ProtocolTraces(_) => &[],
        }))
        .cloned()
        .collect();
    let abstract_fairness: Vec<_> = refinement.abstract_operation_fairness().cloned().collect();
    let classes = abstract_classes(&systems.abstract_system, &abstract_fairness, &identities)
        .map_err(ReplayRefusal::Fault)?;
    let violated = classes.iter().any(|class| {
        class.constraint() == index
            && enabled.iter().all(|here| class.is_enabled_in(here))
            && !loop_taken.iter().any(|taken| class.is_taken_by(taken))
    });
    let divergent = loop_taken.iter().all(|taken| *taken == Taken::Stutter);
    Ok(match (payload.refinement.as_ref(), violated, divergent) {
        (Some(RefinementFailure::Divergence { .. }), true, true)
        | (Some(RefinementFailure::AbstractUnfair { .. }), true, false) => {
            RefinementTraceSettlement::Reproduced {
                failure: payload
                    .refinement
                    .clone()
                    .ok_or_else(|| fault("a-lasso-names-a-failure"))?,
                mapped,
            }
        }
        (_, true, _) => RefinementTraceSettlement::Inconclusive(disagreement(Category::Violation)),
        (_, false, _) => RefinementTraceSettlement::Inconclusive(disagreement(Category::Success)),
    })
}

/// Replay FR-148's fairness-independent observable divergence check over the
/// same protocol candidate sets and the concrete lasso.
fn replay_protocol_divergence(
    _refinement: &CheckedRefinement,
    systems: &RefinementSystems<'_>,
    payload: &TemporalCounterexample,
    lasso: Lasso,
    positions: &[Position],
    mapped: Vec<ModelState>,
    class: &str,
) -> Result<RefinementTraceSettlement, ReplayRefusal> {
    close_loop(&systems.concrete, positions, lasso)?;
    let (
        Some(protocol),
        Some(RefinementFailure::Divergence {
            protocol_class: Some(_),
            ..
        }),
    ) = (
        systems.abstract_protocol.as_ref(),
        payload.refinement.as_ref(),
    )
    else {
        return Err(fault("a-protocol-divergence-has-an-admitted-protocol"));
    };
    let start = lasso.entry;
    let end = positions.len();
    if start >= end {
        return Err(fault("a-protocol-divergence-loop-has-a-step"));
    }
    let loop_taken = payload
        .prefix
        .get(start..)
        .ok_or_else(|| fault("a-lasso-loop-is-in-the-prefix"))?;
    let terminal = usize::from(lasso.terminal_stutter);
    if loop_taken.len().saturating_add(terminal)
        != end.saturating_sub(start + 1).saturating_add(terminal)
    {
        return Err(fault("a-protocol-divergence-loop-aligns-with-positions"));
    }
    // Replay has already recomputed every protocol candidate in `trail`.
    // Reconstruct them once more from the concrete prefix for explicit class
    // enabledness and require the loop to contain only internal stutters.
    let mut trail = Vec::new();
    let mut previous: Option<RefinementPosition> = None;
    let mut taken_by = Vec::new();
    for (index, position) in positions.iter().enumerate() {
        let checked = if let Some(pre) = &previous {
            let Some(step) = payload.prefix.get(index - 1) else {
                return Err(fault("a-protocol-divergence-position-has-a-step"));
            };
            check_protocol_step(
                _refinement,
                &systems.concrete,
                &systems.abstract_system,
                protocol,
                pre,
                &ConcreteStep {
                    position: u64::try_from(index).unwrap_or(u64::MAX),
                    transition: Some(&step.transition),
                    post: &position.state,
                },
            )
        } else {
            check_protocol_initial(
                _refinement,
                &systems.concrete,
                &systems.abstract_system,
                protocol,
                payload.initial_state,
                position.state.state(),
            )
        };
        match checked.verdict {
            StepVerdict::Passes { post, taken } => {
                previous = Some(post.clone());
                trail.push(post);
                if index > 0 {
                    taken_by.push(taken);
                }
            }
            StepVerdict::Stopped(reached) => {
                return Ok(RefinementTraceSettlement::Stopped(reached))
            }
            StepVerdict::Fails(_) | StepVerdict::Undefined(_) | StepVerdict::Undetermined(_) => {
                return Ok(RefinementTraceSettlement::Inconclusive(disagreement(
                    Category::Violation,
                )))
            }
            StepVerdict::Fault(_) => return Err(fault("protocol-divergence-replay-step")),
        }
    }
    if lasso.terminal_stutter {
        let (Some(pre), Some(position)) = (trail.last(), positions.last()) else {
            return Err(fault("a-terminal-stutter-has-a-position"));
        };
        let checked = check_protocol_step(
            _refinement,
            &systems.concrete,
            &systems.abstract_system,
            protocol,
            pre,
            &ConcreteStep {
                position: u64::try_from(positions.len()).unwrap_or(u64::MAX),
                transition: None,
                post: &position.state,
            },
        );
        match checked.verdict {
            StepVerdict::Passes { post, taken } => {
                trail.push(post);
                taken_by.push(taken);
            }
            StepVerdict::Stopped(reached) => {
                return Ok(RefinementTraceSettlement::Stopped(reached))
            }
            StepVerdict::Fault(_) => return Err(fault("protocol-terminal-stutter-replay")),
            StepVerdict::Fails(_) | StepVerdict::Undefined(_) | StepVerdict::Undetermined(_) => {
                return Ok(RefinementTraceSettlement::Inconclusive(disagreement(
                    Category::Violation,
                )))
            }
        }
    }
    let loop_end = start
        .saturating_add(loop_taken.len())
        .saturating_add(terminal);
    let loop_positions = trail
        .get(start..loop_end)
        .ok_or_else(|| fault("a-protocol-divergence-loop-is-in-the-trail"))?;
    if taken_by
        .get(start..)
        .is_none_or(|steps| steps.iter().any(|taken| *taken != Taken::Stutter))
    {
        return Ok(RefinementTraceSettlement::Inconclusive(disagreement(
            Category::Violation,
        )));
    }
    for position in loop_positions {
        for candidate in &position.abstract_protocol_states {
            let expansion = protocol
                .successors(candidate)
                .map_err(|_| fault("protocol-divergence-expansion"))?;
            if !expansion.successors.iter().any(|(transition, _)| {
                crate::refinement::protocol_transition_class(protocol, transition).as_deref()
                    == Some(class)
                    && !_refinement
                        .abstract_internal()
                        .iter()
                        .any(|internal| internal.spelling() == class)
            }) {
                return Ok(RefinementTraceSettlement::Inconclusive(disagreement(
                    Category::Success,
                )));
            }
        }
    }
    Ok(RefinementTraceSettlement::Reproduced {
        failure: payload
            .refinement
            .clone()
            .ok_or_else(|| fault("a-protocol-divergence-has-a-failure"))?,
        mapped,
    })
}

/// Whether two refinement positions of one concrete state are one product
/// state: equal history values and equal candidate sets.
fn same_product_state(
    systems: &RefinementSystems<'_>,
    left: &RefinementPosition,
    right: &RefinementPosition,
) -> Result<bool, ReplayRefusal> {
    let encodes = |_| fault("a-product-state-encodes");
    let candidates = |position: &RefinementPosition| {
        position
            .abstract_states
            .iter()
            .map(|state| {
                systems
                    .abstract_system
                    .state_key_of(state)
                    .map(|(key, _)| key)
            })
            .collect::<Result<Vec<_>, _>>()
    };
    let protocol_candidates = |position: &RefinementPosition| {
        let Some(protocol) = &systems.abstract_protocol else {
            return Ok(Vec::new());
        };
        position
            .abstract_protocol_states
            .iter()
            .map(|state| protocol.state_key(state).map(|(key, _)| key))
            .collect::<Result<Vec<_>, _>>()
    };
    Ok(left.history.key(&systems.concrete).map_err(encodes)?
        == right.history.key(&systems.concrete).map_err(encodes)?
        && candidates(left).map_err(encodes)? == candidates(right).map_err(encodes)?
        && protocol_candidates(left).map_err(encodes)?
            == protocol_candidates(right).map_err(encodes)?)
}

/// What enabledness at a refinement position is.
enum Enabledness {
    /// The abstract transition identities enabled from the position's
    /// mapped states.
    Enabled(Vec<ModelTransition>),
    /// A contract conjunction of the abstract model was undecided.
    Undecided,
    /// A budget stopped the expansion.
    Stopped(ReachedLimit),
}

/// The abstract transition identities enabled from the mapped state of
/// `position` (ADR-020 CO-2).
fn abstract_enabled(
    systems: &RefinementSystems<'_>,
    position: &RefinementPosition,
) -> Result<Enabledness, ReplayRefusal> {
    let mut enabled = Vec::new();
    for candidate in &position.abstract_states {
        let expansion = match systems.abstract_system.expand_state(candidate) {
            Ok(expansion) => expansion,
            Err(stop) => {
                return match stop_verdict(&systems.abstract_system, stop) {
                    StepVerdict::Stopped(reached) => Ok(Enabledness::Stopped(reached)),
                    _ => Err(fault("abstract-expansion")),
                }
            }
        };
        if expansion
            .findings
            .iter()
            .any(|finding| matches!(finding, ModelFinding::ContractUndetermined { .. }))
        {
            return Ok(Enabledness::Undecided);
        }
        enabled.extend(
            expansion
                .successors
                .into_iter()
                .map(|(transition, _)| transition),
        );
    }
    Ok(Enabledness::Enabled(enabled))
}
