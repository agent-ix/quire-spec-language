// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-170's state-graph evidence checks over the admitted `ModelSystem`.

use std::collections::{BTreeMap, VecDeque};

use qsl_eval::simulation::model::{
    ModelSimState, ModelSimulationRequest, ModelSystem, PredicateReading, TerminalClass,
};
use qsl_eval::simulation::{StateKey, TransitionSystem};
use qsl_semantics::check::{
    CheckedStateGraphClaim, CheckedStatePredicate, PredicateRole, StateGraphForm,
};
use quire_exact::ObjectReference;
use quire_semantic_value::catalog::Category;

use crate::bounds::ReplayLimits;
use crate::execute::{recompile, request_limits, ReplayRefusal};
use crate::model_check::counterexample::ModelStep;
use crate::model_check::graph_settle::{GraphReplayResult, GraphReplaySettlement};
use crate::model_check::identity::StateGraphIdentity;
use crate::model_check::limits::{ModelCheckLimit, ReachedLimit};
use crate::model_check::replay::ModelTraceSubject;
use crate::model_check::replay::{fault, refused, ModelTraceRefusal};
use crate::model_check::state_graph::{GraphEvidence, ModelPath, TrapClosure, WitnessPath};
use crate::request::{ReplayRequest, ReplayRequestWire};
use crate::result::{DisagreementCause, Verdict};
use crate::witness::ReplaySource;
use quire_exact::Cancel;

/// Replay a model-graph evidence request through FR-120's `ModelSystem`.
/// Claim identity comes from the model-graph envelope, not a generic
/// `ReplayTarget` member.
pub fn replay_model_graph(
    wire: ReplayRequestWire,
    subject: &ModelTraceSubject,
    replay_limits: ReplayLimits,
) -> Result<crate::result::ReplayResult, ReplayRefusal> {
    replay_model_graph_with_cancel(wire, subject, replay_limits, &Cancel::new())
}

/// [`replay_model_graph`] with the caller's cancellation handle.
pub fn replay_model_graph_with_cancel(
    wire: ReplayRequestWire,
    subject: &ModelTraceSubject,
    replay_limits: ReplayLimits,
    cancel: &Cancel,
) -> Result<crate::result::ReplayResult, ReplayRefusal> {
    let request = ReplayRequest::decode(wire, replay_limits)?;
    let ReplaySource::ModelGraph(envelope) = request.source() else {
        return Err(ReplayRefusal::WrongModelGraphSource);
    };
    let envelope = envelope.as_ref();
    if request.obligation_identity() != envelope.obligation_identity() {
        return Err(ReplayRefusal::ModelGraphMismatch {
            member: "obligation_identity",
        });
    }
    if request.profile_selections() != envelope.profile_selections() {
        return Err(ReplayRefusal::ModelGraphMismatch {
            member: "profile_selections",
        });
    }
    if request.declared_domains() != envelope.declared_domains() {
        return Err(ReplayRefusal::ModelGraphMismatch {
            member: "declared_domains",
        });
    }

    let caller = request_limits(
        request.stage_limits(),
        request.accounting_limits(),
        replay_limits,
    )?;
    let compiled = recompile(&request, &caller)?;
    let package = compiled.checked.package();
    if envelope.claim_node() != envelope.occurrence_key().node() {
        return Err(ReplayRefusal::ModelGraphMismatch {
            member: "occurrence_key",
        });
    }
    let claim = package.graph().state_graph_claims().iter().find(|claim| {
        claim.occurrence().is_some_and(|occurrence| {
            occurrence.node() == envelope.claim_node() && occurrence == envelope.occurrence_key()
        })
    });
    let hyper_clause = package
        .graph()
        .hyper_clauses()
        .iter()
        .find(|clause| clause.assembled_occurrence() == Some(envelope.occurrence_key()));
    if claim.is_some() == hyper_clause.is_some() {
        return Err(ReplayRefusal::ModelGraphMismatch {
            member: "claim_identity",
        });
    }
    if let Some(claim) = claim {
        let identity = StateGraphIdentity::resolve(package, &compiled.packages, claim, cancel)
            .map_err(|refusal| match refusal {
                crate::model_check::identity::ClauseRootRefusal::Cancelled(cause) => {
                    ReplayRefusal::Cancelled(cause)
                }
                _ => ReplayRefusal::ModelGraphMismatch {
                    member: "claim_identity",
                },
            })?;
        if identity.occurrence() != envelope.occurrence_key() {
            return Err(ReplayRefusal::ModelGraphMismatch {
                member: "claim_identity",
            });
        }
    } else if let Some(clause) = hyper_clause {
        let identity = crate::model_check::identity::HyperIdentity::resolve(
            package,
            &compiled.packages,
            clause,
            cancel,
        )
        .map_err(|refusal| match refusal {
            crate::model_check::identity::ClauseRootRefusal::Cancelled(cause) => {
                ReplayRefusal::Cancelled(cause)
            }
            _ => ReplayRefusal::ModelGraphMismatch {
                member: "claim_identity",
            },
        })?;
        if identity.occurrence() != envelope.occurrence_key() {
            return Err(ReplayRefusal::ModelGraphMismatch {
                member: "claim_identity",
            });
        }
    }

    let documents = request.byte_provision().snapshots();
    let simulation = ModelSimulationRequest {
        package,
        packages: &compiled.packages,
        normalization: caller.spine.model,
        snapshots: &documents,
        initial: subject.initial.clone(),
        universes: subject.universes.clone(),
        observation: caller.observation,
        expansion: subject.expansion,
        meter: request.accounting_limits(),
    };
    let system = ModelSystem::new(simulation).map_err(ReplayRefusal::Admission)?;
    let model_limits = caller.engines.model_check;
    let result = if let Some(claim) = claim {
        replay_evidence(
            &system,
            package,
            claim,
            envelope.family_payload(),
            model_limits.limits.max_states,
            model_limits.limits.max_transitions,
        )?
    } else if let Some(clause) = hyper_clause {
        super::graph_hyper::replay_evidence(
            &system,
            package,
            clause,
            envelope.family_payload(),
            model_limits.limits.max_states,
            model_limits.limits.max_transitions,
            model_limits.max_automaton_states,
            subject.expansion.max_candidates,
            request.accounting_limits(),
            cancel,
        )?
    } else {
        return Err(ReplayRefusal::ModelGraphMismatch {
            member: "claim_identity",
        });
    };
    Ok(crate::result::ReplayResult::ModelGraph(result))
}

pub(super) type Position = (ModelSimState, StateKey);

/// Replay typed graph evidence against the recompiled claim and its admitted
/// model system. The facade performs package and identity admission first.
pub(super) fn replay_evidence(
    system: &ModelSystem<'_>,
    package: &qsl_package::CheckedPackage,
    claim: &CheckedStateGraphClaim,
    evidence: &GraphEvidence,
    max_states: usize,
    max_transitions: usize,
) -> Result<GraphReplayResult, ReplayRefusal> {
    let over = match (claim.over(), evidence_over(evidence)) {
        (None, None) => None,
        (Some(parameter), Some(instance)) => {
            let model_instance = system
                .instances(&quire_exact::ValueType::Reference(parameter.object.clone()))
                .map_err(|_| refused(ModelTraceRefusal::OverBinding))?
                .into_iter()
                .find(|candidate| {
                    candidate.population == instance.population && candidate.key == instance.key
                })
                .ok_or_else(|| refused(ModelTraceRefusal::OverBinding))?;
            Some(
                system
                    .bound_reference(parameter.object.clone(), &model_instance)
                    .map_err(|_| fault("state-graph-over-reference"))?,
            )
        }
        _ => return Err(refused(ModelTraceRefusal::OverBinding)),
    };

    let mut evaluated = Vec::new();
    let (settlement, closure_nodes, first_target) = match evidence {
        GraphEvidence::Witness { paths, .. } => {
            let initial_count = system.initial().len();
            if paths.len() != initial_count {
                return Err(refused(ModelTraceRefusal::InitialState {
                    index: paths.len(),
                    count: initial_count,
                }));
            }
            let mut agrees = true;
            for (index, (path, _source)) in paths.iter().enumerate() {
                match path {
                    WitnessPath::Path(path) => {
                        let positions = replay_path(system, path)?;
                        let last = positions.last().ok_or_else(|| fault("path-has-initial"))?;
                        let target = target_read(system, claim, &last.0, over.as_ref())?;
                        match target {
                            PredicateReading::True => evaluated.push((target_role(claim), true)),
                            PredicateReading::False => {
                                evaluated.push((target_role(claim), false));
                                agrees = false;
                            }
                            PredicateReading::Undefined { .. } | PredicateReading::Refused(_) => {
                                agrees = false;
                            }
                        }
                    }
                    WitnessPath::Lasso(_) => {
                        // A lasso belongs to FR-181's temporal product replay.
                        // The state-graph core never interprets it as a finite path.
                        return Err(refused(ModelTraceRefusal::HyperShape {
                            reason: "a product lasso requires FR-181 replay",
                        }));
                    }
                }
                if path_initial(path) != index {
                    return Err(refused(ModelTraceRefusal::InitialState {
                        index: path_initial(path),
                        count: initial_count,
                    }));
                }
            }
            (
                if agrees {
                    GraphReplaySettlement::Reproduced
                } else {
                    inconclusive(Category::Success, Category::Violation)
                },
                None,
                None,
            )
        }
        GraphEvidence::Trap { stem, closure, .. } => {
            if *closure == TrapClosure::Product {
                return Err(refused(ModelTraceRefusal::HyperShape {
                    reason: "a product trap requires FR-181 replay",
                }));
            }
            let positions = replay_path(system, stem)?;
            let last = positions.last().ok_or_else(|| fault("trap-has-initial"))?;
            let (source, target) = from_target_roles(claim);
            if let Some(from) = source {
                let reading = read_predicate(system, from, &last.0, over.as_ref())?;
                if !record_reading(source_role(claim), reading, &mut evaluated)? {
                    return Ok(result(
                        inconclusive(Category::Violation, Category::Success),
                        evaluated,
                        None,
                        None,
                    ));
                }
            } else if !stem.steps.is_empty() {
                return Err(refused(ModelTraceRefusal::HyperShape {
                    reason: "a possible trap has an empty stem",
                }));
            }
            let (count, found, stopped) = explore_closure(
                system,
                &last.0,
                target,
                over.as_ref(),
                max_states,
                max_transitions,
                &mut evaluated,
            )?;
            if let Some(limit) = stopped {
                (GraphReplaySettlement::Stopped(limit), Some(count), found)
            } else if found.is_some() {
                (
                    inconclusive(Category::Violation, Category::Success),
                    Some(count),
                    found,
                )
            } else {
                (GraphReplaySettlement::Reproduced, Some(count), None)
            }
        }
        GraphEvidence::PathPair {
            stem,
            first,
            second,
            ..
        } => {
            let stem_positions = replay_path(system, stem)?;
            let base = &stem_positions
                .last()
                .ok_or_else(|| fault("path-pair-has-stem-initial"))?
                .0;
            let (source, target) = from_target_roles(claim);
            if let Some(from) = source {
                let reading = read_predicate(system, from, base, over.as_ref())?;
                if !record_reading(source_role(claim), reading, &mut evaluated)? {
                    return Ok(result(
                        inconclusive(Category::Violation, Category::Success),
                        evaluated,
                        None,
                        None,
                    ));
                }
            }
            let first_states = replay_steps(system, base.clone(), first)?;
            let second_states = replay_steps(system, base.clone(), second)?;
            let first_ok =
                check_first_target(system, target, &first_states, over.as_ref(), &mut evaluated)?;
            let second_ok = check_first_target(
                system,
                target,
                &second_states,
                over.as_ref(),
                &mut evaluated,
            )?;
            let differs = first != second;
            let settlement = if first_ok && second_ok && differs {
                GraphReplaySettlement::Reproduced
            } else {
                inconclusive(Category::Violation, Category::Success)
            };
            (settlement, None, None)
        }
        GraphEvidence::Undefined {
            stem, undefined, ..
        } => {
            let positions = replay_path(system, stem)?;
            let matches = replay_claim_undefined(
                system,
                package,
                claim,
                &positions,
                over.as_ref(),
                undefined,
            )?;
            (
                if matches {
                    GraphReplaySettlement::Reproduced
                } else {
                    inconclusive(Category::Violation, Category::Success)
                },
                None,
                None,
            )
        }
    };
    Ok(result(settlement, evaluated, closure_nodes, first_target))
}

fn replay_claim_undefined(
    system: &ModelSystem<'_>,
    package: &qsl_package::CheckedPackage,
    claim: &CheckedStateGraphClaim,
    positions: &[Position],
    over: Option<&ObjectReference>,
    undefined: &crate::model_check::state_graph::GraphUndefined,
) -> Result<bool, ReplayRefusal> {
    let Some((_, digest)) = positions.last() else {
        return Err(fault("undefined-has-initial"));
    };
    if *digest != undefined.state_key {
        return Ok(false);
    }
    if let Some(transition) = undefined.transition.as_ref() {
        if undefined.predicate.is_some() || undefined.locus.is_none() {
            return Ok(false);
        }
        let Some(locus) = undefined.locus.as_ref() else {
            return Ok(false);
        };
        let Some(clause) = package
            .graph()
            .state_clauses()
            .iter()
            .find(|clause| clause.body().location() == locus)
        else {
            return Err(ReplayRefusal::ModelGraphMismatch {
                member: "undefined_locus",
            });
        };
        for (index, (state, _)) in positions.iter().enumerate() {
            let expansion = system
                .successors(state)
                .map_err(|_| fault("state-graph-undefined-guard-expansion"))?;
            let matched = expansion
                .findings
                .iter()
                .filter_map(|finding| finding.undefined_contract())
                .any(|contract| {
                    contract.clause == clause.name()
                        && contract.guard_of.as_ref() == Some(transition)
                        && contract.cause == undefined.cause
                });
            if matched {
                return Ok(index + 1 == positions.len());
            }
        }
        return Ok(false);
    }
    if undefined.predicate.is_none() {
        let Some(locus) = undefined.locus.as_ref() else {
            return Ok(false);
        };
        let mut locus_resolved = false;
        for (index, (state, _)) in positions.iter().enumerate() {
            let expansion = system
                .successors(state)
                .map_err(|_| fault("state-graph-undefined-subject-expansion"))?;
            let has_successor = !expansion.successors.is_empty();
            let terminals = qsl_package::resolve_terminals(package)
                .map_err(|_| fault("state-graph-undefined-terminal-resolution"))?;
            let terminal = terminals.iter().find(|terminal| {
                let qsl_package::PackageTerminalDeclaration::When(predicate) = terminal.declaration
                else {
                    return false;
                };
                predicate.body().location() == locus
            });
            if let Some(terminal) = terminal {
                locus_resolved = true;
                if let TerminalClass::Undefined { model, cause } = system
                    .terminal_class(state, has_successor)
                    .map_err(|_| fault("state-graph-undefined-terminal"))?
                {
                    if model == terminal.model && cause == undefined.cause {
                        return Ok(index + 1 == positions.len());
                    }
                }
            }
            let constraint = package.graph().state_clauses().iter().find(|clause| {
                clause.body().location() == locus
                    && clause.kind() == qsl_forms::StateClauseKind::Invariant
            });
            if let Some(constraint) = constraint {
                locus_resolved = true;
                match system.constraint_value(state, constraint.name()) {
                    Ok(qsl_eval::simulation::ConstraintValue::Undefined { cause, .. })
                        if cause == undefined.cause =>
                    {
                        return Ok(index + 1 == positions.len())
                    }
                    Ok(qsl_eval::simulation::ConstraintValue::Undefined { .. })
                    | Ok(qsl_eval::simulation::ConstraintValue::Boundary) => return Ok(false),
                    Ok(qsl_eval::simulation::ConstraintValue::Holds) => {}
                    Err(_) => return Err(fault("state-graph-undefined-constraint")),
                }
            }
        }
        if locus_resolved {
            return Ok(false);
        }
        return Err(ReplayRefusal::ModelGraphMismatch {
            member: "undefined_locus",
        });
    }
    for (index, (state, _)) in positions.iter().enumerate() {
        for (role, predicate) in claim_predicates(claim) {
            if Some(role) != undefined.predicate {
                continue;
            }
            if let PredicateReading::Undefined { cause, location } =
                read_predicate(system, predicate, state, over)?
            {
                return Ok(index + 1 == positions.len()
                    && cause == undefined.cause
                    && location == undefined.locus);
            }
        }
    }
    Ok(false)
}

fn evidence_over(evidence: &GraphEvidence) -> Option<&qsl_eval::model_subject::ModelInstance> {
    match evidence {
        GraphEvidence::Witness { over, .. }
        | GraphEvidence::Trap { over, .. }
        | GraphEvidence::PathPair { over, .. }
        | GraphEvidence::Undefined { over, .. } => over.as_ref(),
    }
}

fn path_initial(path: &WitnessPath) -> usize {
    match path {
        WitnessPath::Path(path) => path.initial as usize,
        WitnessPath::Lasso(lasso) => lasso.path.initial as usize,
    }
}

fn target_role(claim: &CheckedStateGraphClaim) -> PredicateRole {
    match claim.form() {
        StateGraphForm::Possible { .. } | StateGraphForm::AlwaysPossible { .. } => {
            PredicateRole::Target
        }
        StateGraphForm::UniquePath { .. } => PredicateRole::To,
    }
}

fn source_role(claim: &CheckedStateGraphClaim) -> PredicateRole {
    match claim.form() {
        StateGraphForm::Possible { .. } => PredicateRole::Target,
        StateGraphForm::AlwaysPossible { .. } => PredicateRole::From,
        StateGraphForm::UniquePath { .. } => PredicateRole::From,
    }
}

fn from_target_roles(
    claim: &CheckedStateGraphClaim,
) -> (Option<&CheckedStatePredicate>, &CheckedStatePredicate) {
    match claim.form() {
        StateGraphForm::Possible { target } => (None, target),
        StateGraphForm::AlwaysPossible { from, target } => (Some(from), target),
        StateGraphForm::UniquePath { from, to } => (Some(from), to),
    }
}

fn claim_predicates(
    claim: &CheckedStateGraphClaim,
) -> Vec<(PredicateRole, &CheckedStatePredicate)> {
    match claim.form() {
        StateGraphForm::Possible { target } => vec![(PredicateRole::Target, target)],
        StateGraphForm::AlwaysPossible { from, target } => {
            vec![(PredicateRole::From, from), (PredicateRole::Target, target)]
        }
        StateGraphForm::UniquePath { from, to } => {
            vec![(PredicateRole::From, from), (PredicateRole::To, to)]
        }
    }
}

fn read_predicate(
    system: &ModelSystem<'_>,
    predicate: &CheckedStatePredicate,
    state: &ModelSimState,
    over: Option<&ObjectReference>,
) -> Result<PredicateReading, ReplayRefusal> {
    match predicate {
        CheckedStatePredicate::ConstantTrue => Ok(PredicateReading::True),
        CheckedStatePredicate::Checked(body) => system
            .state_predicates(state, &[(body, over)])
            .map_err(|_| fault("state-graph-predicate-reading"))
            .and_then(|mut readings| {
                readings
                    .pop()
                    .ok_or_else(|| fault("state-graph-predicate-result"))
            }),
    }
}

fn target_read(
    system: &ModelSystem<'_>,
    claim: &CheckedStateGraphClaim,
    state: &ModelSimState,
    over: Option<&ObjectReference>,
) -> Result<PredicateReading, ReplayRefusal> {
    let (_, target) = from_target_roles(claim);
    read_predicate(system, target, state, over)
}

fn record_reading(
    role: PredicateRole,
    reading: PredicateReading,
    evaluated: &mut Vec<(PredicateRole, bool)>,
) -> Result<bool, ReplayRefusal> {
    match reading {
        PredicateReading::True => {
            evaluated.push((role, true));
            Ok(true)
        }
        PredicateReading::False => {
            evaluated.push((role, false));
            Ok(false)
        }
        PredicateReading::Undefined { .. } | PredicateReading::Refused(_) => Ok(false),
    }
}

pub(super) fn replay_path(
    system: &ModelSystem<'_>,
    path: &ModelPath,
) -> Result<Vec<Position>, ReplayRefusal> {
    let initial = system.initial();
    let state = initial.get(path.initial as usize).cloned().ok_or_else(|| {
        refused(ModelTraceRefusal::InitialState {
            index: path.initial as usize,
            count: initial.len(),
        })
    })?;
    replay_steps(system, state, &path.steps)
}

fn replay_steps(
    system: &ModelSystem<'_>,
    start: ModelSimState,
    steps: &[ModelStep],
) -> Result<Vec<Position>, ReplayRefusal> {
    let (bytes, _) = system
        .state_key(&start)
        .map_err(|_| fault("state-graph-state-key"))?;
    let key = StateKey::from_canonical_bytes(bytes);
    let mut positions = vec![(start, key)];
    for (index, step) in steps.iter().enumerate() {
        let current = positions
            .last()
            .ok_or_else(|| fault("state-graph-path-position"))?;
        let expansion = system
            .successors(&current.0)
            .map_err(|_| fault("state-graph-path-expansion"))?;
        let mut enabled = false;
        let mut reached = Vec::new();
        for (transition, next) in expansion.successors {
            if transition != step.transition {
                continue;
            }
            enabled = true;
            let (bytes, _) = system
                .state_key(&next)
                .map_err(|_| fault("state-graph-state-key"))?;
            let key = StateKey::from_canonical_bytes(bytes);
            reached.push(key.clone());
            if key == step.post_state {
                positions.push((next, key));
                break;
            }
        }
        if !enabled {
            return Err(refused(ModelTraceRefusal::NotEnabled {
                step: index,
                transition: Box::new(step.transition.clone()),
            }));
        }
        if positions
            .last()
            .is_none_or(|position| position.1 != step.post_state)
        {
            return Err(refused(ModelTraceRefusal::PostState {
                step: index,
                recorded: step.post_state.clone(),
                reached,
            }));
        }
    }
    Ok(positions)
}

fn check_first_target(
    system: &ModelSystem<'_>,
    target: &CheckedStatePredicate,
    states: &[Position],
    over: Option<&ObjectReference>,
    evaluated: &mut Vec<(PredicateRole, bool)>,
) -> Result<bool, ReplayRefusal> {
    for (index, (state, _)) in states.iter().enumerate() {
        let reading = read_predicate(system, target, state, over)?;
        match reading {
            PredicateReading::True => {
                evaluated.push((target_role_from_predicate(target), true));
                return Ok(index + 1 == states.len());
            }
            PredicateReading::False => {
                evaluated.push((target_role_from_predicate(target), false));
            }
            PredicateReading::Undefined { .. } | PredicateReading::Refused(_) => return Ok(false),
        }
    }
    Ok(false)
}

fn target_role_from_predicate(_predicate: &CheckedStatePredicate) -> PredicateRole {
    PredicateRole::To
}

fn explore_closure(
    system: &ModelSystem<'_>,
    initial: &ModelSimState,
    target: &CheckedStatePredicate,
    over: Option<&ObjectReference>,
    max_states: usize,
    max_transitions: usize,
    evaluated: &mut Vec<(PredicateRole, bool)>,
) -> Result<(u64, Option<StateKey>, Option<ReachedLimit>), ReplayRefusal> {
    let (bytes, _) = system
        .state_key(initial)
        .map_err(|_| fault("state-graph-state-key"))?;
    let key = StateKey::from_canonical_bytes(bytes);
    let mut seen = BTreeMap::from([(key.clone(), initial.clone())]);
    let mut queue = VecDeque::from([(key, initial.clone())]);
    let mut transitions = 0usize;
    while let Some((key, state)) = queue.pop_front() {
        let reading = read_predicate(system, target, &state, over)?;
        match reading {
            PredicateReading::True => {
                evaluated.push((target_role_from_predicate(target), true));
                return Ok((seen.len() as u64, Some(key), None));
            }
            PredicateReading::False => evaluated.push((target_role_from_predicate(target), false)),
            PredicateReading::Undefined { .. } | PredicateReading::Refused(_) => {
                return Ok((seen.len() as u64, Some(key), None));
            }
        }
        let expansion = system
            .successors(&state)
            .map_err(|_| fault("state-graph-closure-expansion"))?;
        let mut successors = expansion.successors;
        let mut keyed = Vec::with_capacity(successors.len());
        for (transition, next) in successors.drain(..) {
            transitions = transitions.saturating_add(1);
            if transitions > max_transitions {
                return Ok((
                    seen.len() as u64,
                    None,
                    Some(ReachedLimit {
                        limit: ModelCheckLimit::MaxTransitions,
                        value: max_transitions as u64,
                    }),
                ));
            }
            let (bytes, _) = system
                .state_key(&next)
                .map_err(|_| fault("state-graph-state-key"))?;
            let key = StateKey::from_canonical_bytes(bytes);
            let transition_key = system
                .transition_key(&transition)
                .map_err(|_| fault("state-graph-transition-key"))?;
            keyed.push((transition_key, key, next));
        }
        keyed.sort_by(|left, right| (&left.0, &left.1).cmp(&(&right.0, &right.1)));
        for (_, key, next) in keyed {
            if seen.contains_key(&key) {
                continue;
            }
            if seen.len() >= max_states {
                return Ok((
                    seen.len() as u64,
                    None,
                    Some(ReachedLimit {
                        limit: ModelCheckLimit::MaxStates,
                        value: max_states as u64,
                    }),
                ));
            }
            seen.insert(key.clone(), next.clone());
            queue.push_back((key, next));
        }
    }
    Ok((seen.len() as u64, None, None))
}

pub(super) fn inconclusive(proved: Category, replayed: Category) -> GraphReplaySettlement {
    GraphReplaySettlement::Inconclusive(DisagreementCause::Verdicts {
        proved: Verdict::from_category(proved),
        replayed: Verdict::from_category(replayed),
    })
}

pub(super) fn result(
    settlement: GraphReplaySettlement,
    evaluated_predicates: Vec<(PredicateRole, bool)>,
    closure_nodes: Option<u64>,
    first_target: Option<StateKey>,
) -> GraphReplayResult {
    GraphReplayResult {
        settlement,
        evaluated_predicates,
        closure_nodes,
        first_target,
    }
}
