// SPDX-License-Identifier: AGPL-3.0-or-later
//! HP-5 settlement with independently checked closure and replayed lassos.

use filament_exact::{Cancel, ValueType};
use fsl_eval::simulation::model::ModelSystem;

use super::existential_outcome::{ExistentialInstanceOutcome, ExistentialOutcome};
use super::graph_settle::{GraphReplaySettlement, StateGraphMethod, StateGraphRecord};
use super::settle::Row;
use super::state_graph::{
    GraphEvidence, GraphUndefined, RunEnd, TrapClosure, WitnessPath, WitnessSource,
};
use crate::{
    Certification, IncompleteCause, InconclusiveCause, ProductClosureRefusal,
    ProductClosureRequest, ProofBasis, SettlementBasis, TemporalCertificateCheck, TerminalValue,
};

/// A single-existential settlement and the evidence that its replay reproduced.
pub struct ExistentialRecord {
    /// The FR-169 terminal record.
    pub record: StateGraphRecord,
    /// A reproduced lasso tuple or refuting product trap/undefined prefix.
    pub evidence: Option<GraphEvidence>,
}

/// Settle one HP-5 object binding over the original checked subject.
///
/// # Errors
///
/// Refuses an inconsistent subject, binding or missing proof certificate.
pub fn settle_existential(
    request: &ProductClosureRequest<'_>,
    instance: &ExistentialInstanceOutcome,
    method: &StateGraphMethod,
    cancel: &Cancel,
) -> Result<ExistentialRecord, ProductClosureRefusal> {
    let [subject] = request.subjects else {
        return Err(ProductClosureRefusal::Input("HP-5-subject"));
    };
    let alias = request
        .clause
        .aliases()
        .first()
        .ok_or(ProductClosureRefusal::Input("HP-5-subject"))?;
    let system = ModelSystem::for_alias(subject.simulation.clone(), alias.as_str())
        .map_err(|_| ProductClosureRefusal::Input("HP-5-subject"))?;
    let over = match (request.clause.parameters(), instance.instance.as_slice()) {
        ([], []) => None,
        ([parameter], [(name, key)]) if name == &parameter.name => Some(
            system
                .instances(&ValueType::Reference(parameter.population.object()))
                .map_err(|_| ProductClosureRefusal::Input("HP-5-universe"))?
                .into_iter()
                .find(|instance| &instance.key == key)
                .ok_or(ProductClosureRefusal::Input("HP-5-instance"))?,
        ),
        _ => return Err(ProductClosureRefusal::Input("HP-5-instance")),
    };
    if request.clause.profile() == fsl_semantics::check::TemporalProfile::EventPositionFalseExtension {
        return settle_closed(request, &system, over, instance, method, cancel);
    }
    let mut evidence = None;
    let mut undefined = None;
    let mut end = None;
    let mut replay = |candidate: GraphEvidence, decided: Row| {
        let replayed = super::graph_hyper::replay_evidence(
            &system,
            request.package,
            request.clause,
            &candidate,
            method.limits.limits.max_states,
            method.limits.limits.max_transitions,
            method.limits.max_automaton_states,
            subject.simulation.expansion.max_candidates,
            request.meter,
            cancel,
        );
        match replayed {
            Ok(result) => match result.settlement {
                GraphReplaySettlement::Reproduced => {
                    evidence = Some(candidate);
                    decided
                }
                GraphReplaySettlement::Inconclusive(cause) => {
                    Row::inconclusive(InconclusiveCause::ReplayParity(cause))
                }
                GraphReplaySettlement::Stopped(reached) => {
                    Row::stopped_run(IncompleteCause::ResourceExhausted, Some(reached))
                }
            },
            Err(refusal) => Row::refused(&refusal),
        }
    };
    let row = match &instance.outcome {
        ExistentialOutcome::Witnessed { lassos } => {
            let certificate = instance
                .certificate
                .as_ref()
                .ok_or(ProductClosureRefusal::Input("HP-5-certificate"))?;
            if certificate.instance != instance.instance {
                return Err(ProductClosureRefusal::Input("HP-5-certificate-instance"));
            }
            match crate::check_product_closure(request, certificate, &method.limits, cancel)? {
                TemporalCertificateCheck::Accepted => replay(
                    GraphEvidence::Witness {
                        paths: lassos
                            .iter()
                            .map(|witness| {
                                (
                                    WitnessPath::Lasso(witness.lasso.clone()),
                                    witness.source.clone(),
                                )
                            })
                            .collect(),
                        over: over.clone(),
                    },
                    Row::proof(
                        ProofBasis::Witness {
                            sources: lassos
                                .iter()
                                .map(|witness| witness.source.clone())
                                .collect(),
                        },
                        Certification::Certified,
                        SettlementBasis::DecisiveWitness,
                    ),
                ),
                TemporalCertificateCheck::Rejected(rejection) => {
                    Row::inconclusive(InconclusiveCause::CertificateRejected {
                        rule: rejection.rule,
                        at: rejection.at,
                    })
                }
                TemporalCertificateCheck::Stopped(cause) => Row::stopped_run(cause, None),
                TemporalCertificateCheck::MemoryBoundReached { .. }
                | TemporalCertificateCheck::InstanceBoundReached { .. } => Row::failed(),
            }
        }
        ExistentialOutcome::WitnessedUnchecked {
            lassos,
            end: run_end,
            ..
        } => {
            end = Some(run_end.clone());
            replay(
                GraphEvidence::Witness {
                    paths: lassos
                        .iter()
                        .map(|witness| {
                            (
                                WitnessPath::Lasso(witness.lasso.clone()),
                                witness.source.clone(),
                            )
                        })
                        .collect(),
                    over: over.clone(),
                },
                Row::inconclusive(InconclusiveCause::WellDefinednessUnchecked {
                    sources: lassos
                        .iter()
                        .map(|witness| witness.source.clone())
                        .collect(),
                }),
            )
        }
        ExistentialOutcome::ClosedWitnessed { .. } => {
            return Err(ProductClosureRefusal::Input("HP-5-closed-witness"));
        }
        ExistentialOutcome::Trapped { stem } => replay(
            GraphEvidence::Trap {
                stem: stem.clone(),
                closure: TrapClosure::Product,
                over: over.clone(),
            },
            Row::refuted(SettlementBasis::ClosedScope),
        ),
        ExistentialOutcome::Undefined {
            stem,
            undefined: finding,
        } => {
            let found = GraphUndefined {
                predicate: Some(fsl_semantics::check::PredicateRole::Target),
                transition: finding.site.transition.clone(),
                state_key: finding.site.state_key.clone(),
                locus: None,
                cause: finding.cause.clone(),
            };
            undefined = Some(found.clone());
            replay(
                GraphEvidence::Undefined {
                    stem: stem.clone(),
                    undefined: found,
                    over: over.clone(),
                },
                Row::refuted(SettlementBasis::DecisiveCounterexample),
            )
        }
        ExistentialOutcome::NoInitialState => Row::inconclusive(InconclusiveCause::NoInitialState),
        ExistentialOutcome::NoDecision {
            end: RunEnd::Stopped { cause, limit },
            ..
        } => Row::stopped_run(cause.clone(), *limit),
        ExistentialOutcome::NoDecision {
            end: RunEnd::Completed,
            open,
        } => {
            if open.contains(&super::state_graph::OpenCause::UndecidedSuccessor) {
                Row::inconclusive(InconclusiveCause::UndecidedSuccessor)
            } else if let Some(depth) = method.max_depth {
                Row::inconclusive(InconclusiveCause::BoundReached { depth })
            } else {
                Row::failed()
            }
        }
    };
    if !matches!(row.value, TerminalValue::Refuted) {
        undefined = None;
    }
    Ok(ExistentialRecord {
        record: StateGraphRecord {
            value: row.value,
            label: row.label,
            basis: row.basis,
            execution: row.execution,
            truth: row.truth,
            method: super::settle::EXPLICIT_STATE,
            max_depth: method.max_depth,
            limits: method.limits.used(),
            limit_reached: row.limit_reached,
            seed: method.seed,
            witness_samples: method.limits.witness_samples,
            walks_stopped: instance.walks_stopped,
            end,
            undefined,
        },
        evidence,
    })
}

/// Settle an event-position HP-5 binding over closed finite executions (FR-173
/// "Event-position form"): the closed search is run again and read with the
/// closed-tuple judgement, so a witness counts only when its own execution is
/// found and holds the body, and a trap only when no execution from the initial
/// state does.
fn settle_closed(
    request: &ProductClosureRequest<'_>,
    system: &ModelSystem<'_>,
    over: Option<fsl_eval::model_subject::ModelInstance>,
    instance: &ExistentialInstanceOutcome,
    method: &StateGraphMethod,
    cancel: &Cancel,
) -> Result<ExistentialRecord, ProductClosureRefusal> {
    use fsl_eval::hyper::{
        closed_executions, horizon, judge_closed_tuple, CheckedHyperLetters, CheckedStepMatch,
        ClosedInputs, ClosedStop, ClosedVerdict, ParameterBinding,
    };
    let clause = request.clause;
    let binding: Vec<ParameterBinding> = match (clause.parameters(), &over) {
        ([parameter], Some(found)) => vec![ParameterBinding {
            parameter: parameter.name.clone(),
            instance: found.clone(),
        }],
        _ => Vec::new(),
    };
    let letters = CheckedHyperLetters::new(request.package, clause, vec![system])
        .map_err(|_| ProductClosureRefusal::Input("HP-5-letters"))?;
    let matcher = CheckedStepMatch::new(request.package, clause);
    let depth = horizon(clause.body()).ok_or(ProductClosureRefusal::Input("HP-5-horizon"))?;
    let mut evidence = None;
    let row = match closed_executions(
        system,
        depth,
        method.limits.limits.max_states,
        &std::collections::BTreeSet::new(),
        cancel,
    ) {
        Err(ClosedStop::Encoding(_)) => Row::failed(),
        Err(_) => Row::stopped_run(IncompleteCause::ResourceExhausted, None),
        Ok(executions) => {
            let inputs = ClosedInputs {
                clause,
                instance: &binding,
                letters: &letters,
                matcher: &matcher,
                meter: request.meter,
                cancel,
            };
            let judged = |initial: usize, steps: Option<&[super::counterexample::ModelStep]>| {
                let mut found = false;
                for execution in executions
                    .iter()
                    .filter(|candidate| candidate.initial == initial)
                {
                    match judge_closed_tuple(&inputs, &[execution]) {
                        ClosedVerdict::Holds => {
                            let same = steps.is_none_or(|steps| {
                                steps.len() == execution.steps.len()
                                    && steps.iter().zip(&execution.steps).all(|(step, own)| {
                                        step.transition == own.transition
                                            && step.post_state == own.post_state
                                    })
                            });
                            if same {
                                found = true;
                                break;
                            }
                        }
                        ClosedVerdict::UniversalUnmatched
                        | ClosedVerdict::ExistentialUnmatched
                        | ClosedVerdict::Violated { .. } => {}
                        _ => return Err(()),
                    }
                }
                Ok(found)
            };
            match &instance.outcome {
                ExistentialOutcome::ClosedWitnessed { paths } => {
                    let mut all = Ok(true);
                    for path in paths {
                        let initial = usize::try_from(path.initial).unwrap_or(usize::MAX);
                        all = match judged(initial, Some(&path.steps)) {
                            Ok(true) => all,
                            other => other,
                        };
                        if all != Ok(true) {
                            break;
                        }
                    }
                    match all {
                        Ok(true) if paths.len() == fsl_eval::simulation::TransitionSystem::initial(system).len() => {
                            evidence = Some(GraphEvidence::Witness {
                                paths: paths
                                    .iter()
                                    .map(|path| {
                                        (WitnessPath::Path(path.clone()), WitnessSource::Explored)
                                    })
                                    .collect(),
                                over: over.clone(),
                            });
                            Row::proved(ProofBasis::Exhaustive, SettlementBasis::ClosedScope)
                        }
                        Err(()) => Row::stopped_run(IncompleteCause::ResourceExhausted, None),
                        _ => Row::failed(),
                    }
                }
                ExistentialOutcome::Trapped { stem } => {
                    let initial = usize::try_from(stem.initial).unwrap_or(usize::MAX);
                    match judged(initial, None) {
                        Ok(false) => {
                            evidence = Some(GraphEvidence::Trap {
                                stem: stem.clone(),
                                closure: TrapClosure::Product,
                                over: over.clone(),
                            });
                            Row::refuted(SettlementBasis::ClosedScope)
                        }
                        Ok(true) => Row::failed(),
                        Err(()) => Row::stopped_run(IncompleteCause::ResourceExhausted, None),
                    }
                }
                _ => return Err(ProductClosureRefusal::Input("HP-5-closed-outcome")),
            }
        }
    };
    Ok(ExistentialRecord {
        record: StateGraphRecord {
            value: row.value,
            label: row.label,
            basis: row.basis,
            execution: row.execution,
            truth: row.truth,
            method: super::settle::EXPLICIT_STATE,
            max_depth: method.max_depth,
            limits: method.limits.used(),
            limit_reached: row.limit_reached,
            seed: method.seed,
            witness_samples: method.limits.witness_samples,
            walks_stopped: instance.walks_stopped,
            end: None,
            undefined: None,
        },
        evidence,
    })
}
