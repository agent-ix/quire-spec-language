// SPDX-License-Identifier: AGPL-3.0-or-later
//! HP-5 settlement with independently checked closure and replayed lassos.

use filament_exact::{Cancel, ValueType};
use fsl_eval::simulation::model::ModelSystem;

use super::existential_outcome::{ExistentialInstanceOutcome, ExistentialOutcome};
use super::graph_settle::{GraphReplaySettlement, StateGraphMethod, StateGraphRecord};
use super::settle::Row;
use super::state_graph::{GraphEvidence, GraphUndefined, RunEnd, TrapClosure, WitnessPath};
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
    let system = ModelSystem::new(subject.simulation.clone())
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
