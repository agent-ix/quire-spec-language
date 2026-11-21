// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-170's state-graph evidence checks over the admitted `ModelSystem`.

use std::collections::{BTreeMap, VecDeque};

use filament_exact::ObjectReference;
use filament_semantic_value::catalog::Category;
use filament_semantic_value::catalog::InternalFault;
use fsl_eval::simulation::model::{
    ModelSimState, ModelSimulationRequest, ModelSystem, PredicateReading, TerminalClass,
    TimedSystem,
};
use fsl_eval::simulation::{StateKey, TransitionSystem};
use fsl_semantics::check::{
    CheckedStateGraphClaim, CheckedStatePredicate, PredicateRole, StateGraphForm,
};

use crate::bounds::ReplayLimits;
use crate::certificate::zone_certificate::{
    CertificateItem, CertifiedItem, ZoneCertificateRequest, ZoneCertificateVerdict,
};
use crate::execute::{recompile_with_temporal_selections, request_limits, ReplayRefusal};
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
use filament_exact::Cancel;

/// Replay a model-graph evidence request through FR-120's `ModelSystem`.
/// Claim identity comes from the model-graph envelope, not a generic
/// `ReplayTarget` member.
pub fn replay_model_graph(
    wire: ReplayRequestWire,
    subject: &ModelTraceSubject,
    replay_limits: ReplayLimits,
) -> Result<crate::result::ReplayResult, ReplayRefusal> {
    replay_model_graph_with_temporal_selections(wire, subject, &[], replay_limits)
}

/// Replay a model-graph witness while recompiling with explicit temporal clock
/// selections for every configured clause in the checked package.
pub fn replay_model_graph_with_temporal_selections(
    wire: ReplayRequestWire,
    subject: &ModelTraceSubject,
    temporal_clock_selections: &[fsl_package::TemporalClockSelection],
    replay_limits: ReplayLimits,
) -> Result<crate::result::ReplayResult, ReplayRefusal> {
    replay_model_graph_with_cancel_and_temporal_selections(
        wire,
        subject,
        temporal_clock_selections,
        replay_limits,
        &Cancel::new(),
    )
}

/// [`replay_model_graph`] with the caller's cancellation handle.
pub fn replay_model_graph_with_cancel(
    wire: ReplayRequestWire,
    subject: &ModelTraceSubject,
    replay_limits: ReplayLimits,
    cancel: &Cancel,
) -> Result<crate::result::ReplayResult, ReplayRefusal> {
    replay_model_graph_with_cancel_and_temporal_selections(
        wire,
        subject,
        &[],
        replay_limits,
        cancel,
    )
}

/// [`replay_model_graph_with_temporal_selections`] with the caller's
/// cancellation handle.
pub fn replay_model_graph_with_cancel_and_temporal_selections(
    wire: ReplayRequestWire,
    subject: &ModelTraceSubject,
    temporal_clock_selections: &[fsl_package::TemporalClockSelection],
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
    let compiled =
        recompile_with_temporal_selections(&request, &caller, temporal_clock_selections)?;
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
        let identity = StateGraphIdentity::resolve(
            package,
            &compiled.packages,
            claim,
            temporal_clock_selections,
            cancel,
            caller.spine.package.emit(),
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
    } else if let Some(clause) = hyper_clause {
        let identity = crate::model_check::identity::HyperIdentity::resolve(
            package,
            &compiled.packages,
            clause,
            temporal_clock_selections,
            cancel,
            caller.spine.package.emit(),
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
        input: fsl_eval::simulation::model::ModelCheckInput::source(package),
        packages: &compiled.packages,
        normalization: caller.spine.model,
        snapshots: &documents,
        initial: subject.initial.clone(),
        universes: subject.universes.clone(),
        observation: caller.observation,
        expansion: subject.expansion,
        meter: request.accounting_limits(),
        emit: caller.spine.package.emit(),
    };
    let system = ModelSystem::new(simulation.clone()).map_err(ReplayRefusal::Admission)?;
    let timed = subject
        .timed
        .as_ref()
        .map(|reading| {
            TimedSystem::with_max_clocks(&system, &reading.model, reading.max_clocks)
                .map_err(|refusal| refused(ModelTraceRefusal::TimedSubject(refusal)))
        })
        .transpose()?;
    let model_limits = caller.engines.model_check;
    let state_graph_truth =
        if let (Some(claim), Some(certificate)) = (claim, envelope.zone_certificate()) {
            let reading = subject
                .timed
                .as_ref()
                .ok_or(ReplayRefusal::ModelGraphMismatch {
                    member: "timed_subject",
                })?;
            let over = graph_over(&system, claim, envelope.family_payload())?;
            let verdict = crate::check_zone_certificate(
                ZoneCertificateRequest {
                    simulation,
                    model: &reading.model,
                    max_clocks: reading.max_clocks,
                    item: CertifiedItem::StateGraph {
                        claim,
                        over: over.as_ref(),
                    },
                },
                &certificate.obligation,
                |obligation| match &obligation.kind {
                    fsl_route::deadlock::ObligationKind::StateGraphAuthored(occurrence) => {
                        CertificateItem::StateGraph(occurrence)
                    }
                    _ => CertificateItem::Other,
                },
                certificate,
                crate::CertificateCheckLimits::default(),
                cancel,
            )
            .map_err(|_| ReplayRefusal::ModelGraphMismatch {
                member: "zone_certificate",
            })?;
            match verdict {
                ZoneCertificateVerdict::Accepted => Some(true),
                ZoneCertificateVerdict::Rejected(rejection)
                    if matches!(
                        rejection.rule,
                        crate::CertificateRule::InitialMissing
                            | crate::CertificateRule::SuccessorMissing
                            | crate::CertificateRule::CountBroken
                    ) =>
                {
                    Some(false)
                }
                _ => {
                    return Err(ReplayRefusal::ModelGraphMismatch {
                        member: "zone_certificate",
                    });
                }
            }
        } else if let (Some(claim), Some(timed_replay), Some(timed)) =
            (claim, subject.timed.as_ref(), timed.as_ref())
        {
            if timed_replay.digital {
                Some(digital_state_graph_truth(
                    &system,
                    timed,
                    claim,
                    envelope.family_payload(),
                    u64::try_from(model_limits.limits.max_states).unwrap_or(u64::MAX),
                    u64::try_from(model_limits.limits.max_transitions).unwrap_or(u64::MAX),
                    cancel,
                )?)
            } else {
                None
            }
        } else {
            None
        };
    let result = if let Some(claim) = claim {
        replay_evidence(
            &system,
            timed.as_ref(),
            state_graph_truth,
            Loci::Source(package),
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

/// FR-850, FR-940: [`replay_model_graph`] over the package route. The replay
/// reads the request's package bytes and the handoff `supply`, never a source
/// provision: the strict reader admits the bytes against the request's domain
/// packages, the model system is built from the package route's model-check
/// input, and the envelope's claim is the package form of the state-graph
/// claim at its occurrence ([`fsl_eval::simulation::model::PackageModelInput::state_graph_claim`]),
/// whose predicates are read through the evaluator port. A claim whose
/// evidence names an undefined state clause, guard or terminal locus has no
/// package form here yet and is refused, never replayed from source.
///
/// # Errors
///
/// [`ReplayRefusal`] as [`replay_model_graph`] refuses, and a fault when the
/// bytes or supply the analysis admitted do not admit again.
pub fn replay_model_graph_from_package(
    wire: ReplayRequestWire,
    supply: crate::HandoffSupply<'_>,
    subject: &ModelTraceSubject,
    replay_limits: ReplayLimits,
) -> Result<crate::result::ReplayResult, ReplayRefusal> {
    let fault = |cause: &'static str| ReplayRefusal::Fault(InternalFault::new("FR-850", cause));
    let request = ReplayRequest::decode_package_route(wire, replay_limits)?;
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
    if envelope.claim_node() != envelope.occurrence_key().node() {
        return Err(ReplayRefusal::ModelGraphMismatch {
            member: "occurrence_key",
        });
    }
    let caller = request_limits(
        request.stage_limits(),
        request.accounting_limits(),
        replay_limits,
    )?;
    let packages =
        fsl_semantics::model::intake::package_input(request.byte_provision().package_documents())
            .map_err(|refusal| ReplayRefusal::DomainPackages(Box::new(refusal)))?;
    let read_limits =
        fsl_package::V2ReadLimits::bounded(crate::spine::I2_NATIVE_JOIN_STORAGE_DEFAULT);
    let mut budget = read_limits
        .native_join_limits()
        .budget()
        .with_read_storage_limit(u64::MAX);
    let admitted = supply.admit_package(
        request.package().bytes(),
        &packages,
        read_limits,
        &mut budget,
    )?;
    let identities = fsl_package::ReadIdentityRows::read(supply.identities, &admitted, &mut budget)
        .map_err(|_| fault("handoff-identities-admit"))?;
    let equality = supply
        .equality
        .map(|bytes| fsl_package::ReadEqualityRows::read(bytes, &admitted))
        .transpose()
        .map_err(|_| fault("handoff-equality-admit"))?;
    let bare = fsl_package::BarePackage::new(&admitted)
        .with_dependency_artifacts(supply.dependencies, &mut budget)
        .and_then(|package| {
            package.with_supply(
                fsl_package::BareSupply {
                    identities: &identities,
                    equality: equality.as_ref(),
                },
                &mut budget,
            )
        })
        .map_err(|_| fault("handoff-supply-joins"))?;
    let input =
        fsl_eval::simulation::model::PackageModelInput::of(&bare, &packages, caller.spine.model)
            .map_err(|_| fault("package-route-input"))?;
    let claim = input
        .state_graph_claim(
            envelope.occurrence_key(),
            filament_semantic_value::checking::CheckingLimits::default().nodes(),
        )
        .map_err(|_| ReplayRefusal::ModelGraphMismatch {
            member: "claim_identity",
        })?;
    let documents = request.byte_provision().snapshots();
    let system = ModelSystem::new(ModelSimulationRequest {
        input: input.input(),
        packages: &packages,
        normalization: caller.spine.model,
        snapshots: &documents,
        initial: subject.initial.clone(),
        universes: subject.universes.clone(),
        observation: caller.observation,
        expansion: subject.expansion,
        meter: request.accounting_limits(),
        emit: caller.spine.package.emit(),
    })
    .map_err(ReplayRefusal::Admission)?;
    let model_limits = caller.engines.model_check;
    Ok(crate::result::ReplayResult::ModelGraph(replay_evidence(
        &system,
        None,
        None,
        Loci::Package(&input),
        &claim,
        envelope.family_payload(),
        model_limits.limits.max_states,
        model_limits.limits.max_transitions,
    )?))
}

pub(super) type Position = (ModelSimState, StateKey);

struct GraphPathReplay {
    positions: Vec<Position>,
    final_timed: Option<fsl_eval::simulation::model::TimedState>,
}

/// Replay typed graph evidence against the recompiled claim and its admitted
/// model system. The facade performs package and identity admission first.
#[allow(
    clippy::too_many_arguments,
    reason = "replay binds the subject, evidence, cancellation, and independent state and transition bounds"
)]
pub(super) fn replay_evidence(
    system: &ModelSystem<'_>,
    timed: Option<&TimedSystem<'_, '_>>,
    state_graph_truth: Option<bool>,
    loci: Loci<'_>,
    claim: &CheckedStateGraphClaim,
    evidence: &GraphEvidence,
    max_states: usize,
    max_transitions: usize,
) -> Result<GraphReplayResult, ReplayRefusal> {
    let unique_path = matches!(claim.form(), StateGraphForm::UniquePath { .. });
    replay_claim_evidence(
        system,
        timed,
        state_graph_truth,
        loci,
        claim,
        evidence,
        max_states,
        max_transitions,
    )
    .map(|replayed| GraphReplayResult {
        unique_path,
        ..replayed
    })
}

#[allow(
    clippy::too_many_arguments,
    reason = "replay binds the subject, evidence, cancellation, and independent state and transition bounds"
)]
fn replay_claim_evidence(
    system: &ModelSystem<'_>,
    timed: Option<&TimedSystem<'_, '_>>,
    state_graph_truth: Option<bool>,
    loci: Loci<'_>,
    claim: &CheckedStateGraphClaim,
    evidence: &GraphEvidence,
    max_states: usize,
    max_transitions: usize,
) -> Result<GraphReplayResult, ReplayRefusal> {
    let over = graph_over(system, claim, evidence)?;

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
            if state_graph_truth == Some(false) {
                agrees = false;
            }
            for (index, (path, _source)) in paths.iter().enumerate() {
                match path {
                    WitnessPath::Path(path) => {
                        let replayed = replay_graph_path(system, timed, path)?;
                        let last = replayed
                            .positions
                            .last()
                            .ok_or_else(|| fault("path-has-initial"))?;
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
            let replayed = replay_graph_path(system, timed, stem)?;
            let positions = replayed.positions;
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
            if timed.is_some() && state_graph_truth == Some(false) {
                let reading = read_predicate(system, target, &last.0, over.as_ref())?;
                let reproduced = matches!(reading, PredicateReading::False);
                let settlement = if reproduced {
                    GraphReplaySettlement::Reproduced
                } else {
                    inconclusive(Category::Violation, Category::Success)
                };
                return Ok(result(settlement, evaluated, None, None));
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
            let replayed_stem = replay_graph_path(system, timed, stem)?;
            let stem_positions = replayed_stem.positions;
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
            let first_states = replay_graph_steps(
                system,
                timed,
                base.clone(),
                replayed_stem.final_timed.clone(),
                first,
            )?;
            let second_states = replay_graph_steps(
                system,
                timed,
                base.clone(),
                replayed_stem.final_timed,
                second,
            )?;
            let (first_ok, second_ok) = if matches!(claim.form(), StateGraphForm::UniquePath { .. })
            {
                (
                    check_target_endpoint(
                        system,
                        target,
                        &first_states,
                        over.as_ref(),
                        &mut evaluated,
                    )?,
                    check_target_endpoint(
                        system,
                        target,
                        &second_states,
                        over.as_ref(),
                        &mut evaluated,
                    )?,
                )
            } else {
                (
                    check_first_target(
                        system,
                        target,
                        &first_states,
                        over.as_ref(),
                        &mut evaluated,
                    )?,
                    check_first_target(
                        system,
                        target,
                        &second_states,
                        over.as_ref(),
                        &mut evaluated,
                    )?,
                )
            };
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
            let replayed = replay_graph_path(system, timed, stem)?;
            let matches = replay_claim_undefined(
                system,
                loci,
                claim,
                &replayed.positions,
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
    loci: Loci<'_>,
    claim: &CheckedStateGraphClaim,
    positions: &[Position],
    over: Option<&ObjectReference>,
    undefined: &crate::model_check::state_graph::GraphUndefined,
) -> Result<bool, ReplayRefusal> {
    let Some((_, state_key)) = positions.last() else {
        return Err(fault("undefined-has-initial"));
    };
    if *state_key != undefined.state_key {
        return Ok(false);
    }
    if let Some(transition) = undefined.transition.as_ref() {
        if undefined.predicate.is_some() || undefined.locus.is_none() {
            return Ok(false);
        }
        let Some(locus) = undefined.locus.as_ref() else {
            return Ok(false);
        };
        let Some(clause) = loci.state_clause_at(locus, false) else {
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
                    contract.clause == clause
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
            let terminal = loci.terminal_at(locus)?;
            if let Some(terminal) = terminal {
                locus_resolved = true;
                if let TerminalClass::Undefined { model, cause } = system
                    .terminal_class(state, has_successor)
                    .map_err(|_| fault("state-graph-undefined-terminal"))?
                {
                    if model == terminal && cause == undefined.cause {
                        return Ok(index + 1 == positions.len());
                    }
                }
            }
            if let Some(constraint) = loci.state_clause_at(locus, true) {
                locus_resolved = true;
                match system.constraint_value(state, &constraint) {
                    Ok(fsl_eval::simulation::ConstraintValue::Undefined { cause, .. })
                        if cause == undefined.cause =>
                    {
                        return Ok(index + 1 == positions.len());
                    }
                    Ok(fsl_eval::simulation::ConstraintValue::Undefined { .. })
                    | Ok(fsl_eval::simulation::ConstraintValue::Boundary) => return Ok(false),
                    Ok(fsl_eval::simulation::ConstraintValue::Holds) => {}
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

fn evidence_over(evidence: &GraphEvidence) -> Option<&fsl_eval::model_subject::ModelInstance> {
    match evidence {
        GraphEvidence::Witness { over, .. }
        | GraphEvidence::Trap { over, .. }
        | GraphEvidence::PathPair { over, .. }
        | GraphEvidence::Undefined { over, .. } => over.as_ref(),
    }
}

fn graph_over(
    system: &ModelSystem<'_>,
    claim: &CheckedStateGraphClaim,
    evidence: &GraphEvidence,
) -> Result<Option<ObjectReference>, ReplayRefusal> {
    match (claim.over(), evidence_over(evidence)) {
        (None, None) => Ok(None),
        (Some(parameter), Some(instance)) => {
            let model_instance = system
                .instances(&filament_exact::ValueType::Reference(
                    parameter.object.clone(),
                ))
                .map_err(|_| refused(ModelTraceRefusal::OverBinding))?
                .into_iter()
                .find(|candidate| {
                    candidate.population == instance.population && candidate.key == instance.key
                })
                .ok_or_else(|| refused(ModelTraceRefusal::OverBinding))?;
            system
                .bound_reference(parameter.object.clone(), &model_instance)
                .map(Some)
                .map_err(|_| fault("state-graph-over-reference"))
        }
        _ => Err(refused(ModelTraceRefusal::OverBinding)),
    }
}

fn digital_state_graph_truth(
    system: &ModelSystem<'_>,
    timed: &TimedSystem<'_, '_>,
    claim: &CheckedStateGraphClaim,
    evidence: &GraphEvidence,
    max_states: u64,
    max_transitions: u64,
    cancel: &Cancel,
) -> Result<bool, ReplayRefusal> {
    let graph = crate::explore_digital_state_graph(timed, max_states, max_transitions, cancel)
        .map_err(|_| ReplayRefusal::ModelGraphMismatch {
            member: "digital_state_graph",
        })?;
    let over = graph_over(system, claim, evidence)?;
    let read_values = |predicate: &fsl_semantics::check::CheckedStatePredicate| {
        if predicate.is_constant_true() {
            return Ok(vec![true; graph.nodes.len()]);
        }
        let body = predicate
            .predicate_body()
            .ok_or(ReplayRefusal::ModelGraphMismatch {
                member: "digital_state_graph_predicate",
            })?;
        graph
            .nodes
            .iter()
            .map(|node| {
                let readings = system
                    .state_predicates_with_cancel(node.discrete(), &[(body, over.as_ref())], cancel)
                    .map_err(|_| ReplayRefusal::ModelGraphMismatch {
                        member: "digital_state_graph_predicate",
                    })?;
                match readings.as_slice() {
                    [PredicateReading::True] => Ok(true),
                    [PredicateReading::False] => Ok(false),
                    _ => Err(ReplayRefusal::ModelGraphMismatch {
                        member: "digital_state_graph_predicate",
                    }),
                }
            })
            .collect::<Result<Vec<_>, _>>()
    };
    let (source, target) = match claim.form() {
        StateGraphForm::Possible { target } => (None, target),
        StateGraphForm::AlwaysPossible { from, target } => (Some(from), target),
        StateGraphForm::UniquePath { from, to } => (Some(from), to),
    };
    let target_values = read_values(target)?;
    let reaches_target = digital_predecessors(&graph, &target_values);
    if let Some(source) = source {
        let source_values = read_values(source)?;
        Ok(source_values
            .iter()
            .zip(&reaches_target)
            .all(|(is_source, reaches)| !is_source || *reaches))
    } else {
        Ok(graph
            .initials
            .iter()
            .all(|initial| reaches_target[*initial]))
    }
}

fn digital_predecessors(graph: &crate::DigitalStateGraph, target: &[bool]) -> Vec<bool> {
    let mut reaches = target.to_vec();
    let mut queue: VecDeque<usize> = reaches
        .iter()
        .enumerate()
        .filter_map(|(index, yes)| yes.then_some(index))
        .collect();
    while let Some(node) = queue.pop_front() {
        for edge in graph.edges.iter().filter(|edge| edge.to == node) {
            if !reaches[edge.from] {
                reaches[edge.from] = true;
                queue.push_back(edge.from);
            }
        }
    }
    reaches
}

fn path_initial(path: &WitnessPath) -> usize {
    match path {
        WitnessPath::Path(path) => path.initial as usize,
        WitnessPath::Lasso(lasso) => lasso.path.initial as usize,
    }
}

/// One predicate of a claim, read through the evaluator port by its role.
#[derive(Clone, Copy)]
struct Slot<'a> {
    claim: &'a CheckedStateGraphClaim,
    role: PredicateRole,
}

fn target_role(_claim: &CheckedStateGraphClaim) -> PredicateRole {
    PredicateRole::Target
}

fn source_role(claim: &CheckedStateGraphClaim) -> PredicateRole {
    match claim.form() {
        StateGraphForm::Possible { .. } => PredicateRole::Target,
        StateGraphForm::AlwaysPossible { .. } => PredicateRole::From,
        StateGraphForm::UniquePath { .. } => PredicateRole::From,
    }
}

fn from_target_roles(claim: &CheckedStateGraphClaim) -> (Option<Slot<'_>>, Slot<'_>) {
    let slot = |role| Slot { claim, role };
    match claim.form() {
        StateGraphForm::Possible { .. } => (None, slot(PredicateRole::Target)),
        StateGraphForm::AlwaysPossible { .. } | StateGraphForm::UniquePath { .. } => {
            (Some(slot(PredicateRole::From)), slot(PredicateRole::Target))
        }
    }
}

fn claim_predicates(claim: &CheckedStateGraphClaim) -> Vec<(PredicateRole, Slot<'_>)> {
    let (from, target) = from_target_roles(claim);
    from.into_iter()
        .chain([target])
        .map(|slot| (slot.role, slot))
        .collect()
}

fn read_predicate(
    system: &ModelSystem<'_>,
    slot: Slot<'_>,
    state: &ModelSimState,
    over: Option<&ObjectReference>,
) -> Result<PredicateReading, ReplayRefusal> {
    let (from, target) = slot.claim.node_predicates();
    let predicate = match slot.role {
        PredicateRole::From => from,
        PredicateRole::Target => Some(target),
    };
    if predicate.is_none_or(CheckedStatePredicate::is_constant_true) {
        return Ok(PredicateReading::True);
    }
    system
        .state_graph_predicates(state, &[(slot.claim, slot.role, over)])
        .map_err(|_| fault("state-graph-predicate-reading"))
        .and_then(|mut readings| {
            readings
                .pop()
                .ok_or_else(|| fault("state-graph-predicate-result"))
        })
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

fn replay_graph_path(
    system: &ModelSystem<'_>,
    timed: Option<&TimedSystem<'_, '_>>,
    path: &ModelPath,
) -> Result<GraphPathReplay, ReplayRefusal> {
    let Some(timed) = timed else {
        return Ok(GraphPathReplay {
            positions: replay_path(system, path)?,
            final_timed: None,
        });
    };
    let initials = timed.initial();
    let state = initials
        .get(path.initial as usize)
        .cloned()
        .ok_or_else(|| {
            refused(ModelTraceRefusal::InitialState {
                index: path.initial as usize,
                count: initials.len(),
            })
        })?;
    let (positions, final_timed) = replay_timed_steps(timed, state, &path.steps)?;
    Ok(GraphPathReplay {
        positions,
        final_timed: Some(final_timed),
    })
}

fn replay_graph_steps(
    system: &ModelSystem<'_>,
    timed: Option<&TimedSystem<'_, '_>>,
    start: ModelSimState,
    timed_start: Option<fsl_eval::simulation::model::TimedState>,
    steps: &[ModelStep],
) -> Result<Vec<Position>, ReplayRefusal> {
    match (timed, timed_start) {
        (Some(timed), Some(start)) => replay_timed_steps(timed, start, steps).map(|(path, _)| path),
        (Some(_), None) => Err(fault("state-graph-timed-path-start")),
        (None, None) => replay_steps(system, start, steps),
        (None, Some(_)) => Err(fault("state-graph-untimed-path-start")),
    }
}

fn replay_timed_steps(
    timed: &TimedSystem<'_, '_>,
    start: fsl_eval::simulation::model::TimedState,
    steps: &[ModelStep],
) -> Result<(Vec<Position>, fsl_eval::simulation::model::TimedState), ReplayRefusal> {
    let key = timed
        .state_key(&start)
        .map_err(|_| fault("state-graph-timed-state-key"))?;
    let mut positions = vec![(start.discrete().clone(), key)];
    let mut current = start;
    for (index, step) in steps.iter().enumerate() {
        let delay = step
            .delay
            .as_ref()
            .ok_or_else(|| refused(ModelTraceRefusal::MissingDelay { step: Some(index) }))?;
        let delayed = timed
            .delay(&current, delay)
            .map_err(|_| fault("state-graph-timed-delay"))?
            .ok_or_else(|| {
                refused(ModelTraceRefusal::DelayNotAdmissible {
                    step: Some(index),
                    delay: delay.clone(),
                    bar: if delay.numerator().is_negative() {
                        super::replay::DelayBar::Negative
                    } else if timed.tick_period().is_some() {
                        super::replay::DelayBar::TickSource
                    } else {
                        super::replay::DelayBar::TimeInvariant("timed state invariant".to_owned())
                    },
                })
            })?;
        let successors = timed
            .steps(&delayed)
            .map_err(|_| fault("state-graph-timed-expansion"))?;
        let mut enabled = false;
        let mut reached = Vec::new();
        let mut found = None;
        for (transition, state) in successors {
            if transition.transition() != &step.transition {
                continue;
            }
            enabled = true;
            let key = timed
                .state_key(&state)
                .map_err(|_| fault("state-graph-timed-state-key"))?;
            reached.push(key.clone());
            if key == step.post_state {
                found = Some((state, key));
                break;
            }
        }
        let Some((state, key)) = found else {
            if !enabled {
                return Err(refused(ModelTraceRefusal::NotEnabled {
                    step: index,
                    transition: Box::new(step.transition.clone()),
                }));
            }
            return Err(refused(ModelTraceRefusal::PostState {
                step: index,
                recorded: step.post_state.clone(),
                reached,
            }));
        };
        positions.push((state.discrete().clone(), key));
        current = state;
    }
    Ok((positions, current))
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
        if step.delay.is_some() {
            return Err(refused(ModelTraceRefusal::UntimedDelay {
                step: Some(index),
            }));
        }
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

fn check_target_endpoint(
    system: &ModelSystem<'_>,
    target: Slot<'_>,
    states: &[Position],
    over: Option<&ObjectReference>,
    evaluated: &mut Vec<(PredicateRole, bool)>,
) -> Result<bool, ReplayRefusal> {
    let Some((state, _)) = states.last() else {
        return Err(fault("target-path-has-no-endpoint"));
    };
    let reading = read_predicate(system, target, state, over)?;
    record_reading(target.role, reading, evaluated)
}

fn check_first_target(
    system: &ModelSystem<'_>,
    target: Slot<'_>,
    states: &[Position],
    over: Option<&ObjectReference>,
    evaluated: &mut Vec<(PredicateRole, bool)>,
) -> Result<bool, ReplayRefusal> {
    for (index, (state, _)) in states.iter().enumerate() {
        let reading = read_predicate(system, target, state, over)?;
        match reading {
            PredicateReading::True => {
                evaluated.push((target.role, true));
                return Ok(index + 1 == states.len());
            }
            PredicateReading::False => {
                evaluated.push((target.role, false));
            }
            PredicateReading::Undefined { .. } | PredicateReading::Refused(_) => return Ok(false),
        }
    }
    Ok(false)
}

fn explore_closure(
    system: &ModelSystem<'_>,
    initial: &ModelSimState,
    target: Slot<'_>,
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
                evaluated.push((target.role, true));
                return Ok((seen.len() as u64, Some(key), None));
            }
            PredicateReading::False => evaluated.push((target.role, false)),
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
        unique_path: false,
    }
}

/// Where an undefined locus is resolved: the source route's checked package,
/// or the package route's state clauses read from the admitted bytes
/// ([`PackageModelInput::state_clauses`]).
#[derive(Clone, Copy)]
pub(super) enum Loci<'a> {
    /// The recompiled source package.
    Source(&'a fsl_package::CheckedPackage),
    /// The package route's model-check input.
    Package(&'a fsl_eval::simulation::model::PackageModelInput<'a>),
}

impl Loci<'_> {
    /// The name of the state clause whose body is at `locus`; only an
    /// `invariant` when `invariant` is set.
    fn state_clause_at(
        self,
        locus: &filament_semantic_value::location::ExpressionLocation,
        invariant: bool,
    ) -> Option<String> {
        match self {
            Self::Source(package) => package
                .graph()
                .state_clauses()
                .iter()
                .find(|clause| {
                    clause.body().location() == locus
                        && (!invariant || clause.kind() == fsl_forms::StateClauseKind::Invariant)
                })
                .map(|clause| clause.name().to_owned()),
            Self::Package(input) => input.state_clauses().iter().find_map(|entry| {
                if invariant && !entry.invariant {
                    return None;
                }
                let body = input
                    .package()
                    .project_state_clause(&entry.identity, &entry.name)
                    .ok()?;
                let projection = body.projection();
                (projection.location(projection.root()) == Some(locus)).then(|| entry.name.clone())
            }),
        }
    }

    /// The model of the `terminal when` predicate whose body is at `locus`,
    /// by selection identity, on either route (FR-940).
    fn terminal_at(
        self,
        locus: &filament_semantic_value::location::ExpressionLocation,
    ) -> Result<Option<fsl_eval::simulation::model::TerminalModel>, ReplayRefusal> {
        let source;
        let input = match self {
            Self::Source(package) => {
                source = fsl_eval::simulation::model::ModelCheckInput::source(package);
                &source
            }
            Self::Package(input) => &input.input(),
        };
        input
            .terminal_at(locus)
            .map_err(|_| fault("state-graph-undefined-terminal-resolution"))
    }
}
