// SPDX-License-Identifier: AGPL-3.0-or-later
//! The EN-5 item adapter (FR-281, FR-203): builds FR-196's product for an
//! unbounded `eventually` reachability claim, solves it exactly (FR-198),
//! turns the solution into FR-201's certificate, has the qualified core's
//! checker re-enumerate the subject and check it, and only then settles
//! `proved`. No verdict is built by hand: `proved` is built from the
//! checker's acceptance.
//!
//! This is the first cut. It decides the shape the checker core covers
//! (`check_claim_exact_reachability`: one initial state, at most one `over`
//! instance, an acyclic model whose draws each have one post-state); every other shape
//! settles `unsupported` with `unsupported_construct`/`not-yet-implemented`
//! and is never relabelled as a verdict. Missed bounds use the existing
//! witness producer and the independent typed replay core; only the core's
//! reproduced token can settle `refuted` with its counterexample evidence.

use qsl_eval::simulation::model::ModelSystem;
use qsl_eval::simulation::TransitionSystem;
use qsl_foundation::{BackendId, RequestIndex};
use qsl_replay::statistical::SubjectLimits;
use qsl_replay::{
    check_claim_exact_reachability, claim_binding, ExactBinding, ExactProbBuildIssue,
    ExactProbBuildRecord, ExactProbabilityCertificate, ExactProbabilityCertificateCheck,
    ExactProbabilityGraphError, ExactProbabilityValue, Extremum, ModelCheckLimit,
    ProbabilityBoundSide, ProbabilityProductKey, TerminalRecord,
};
use qsl_replay::{
    replay_probabilistic_witness, ProbabilisticCounterexample, ProbabilityWitnessError,
    ProbabilityWitnessPath, ProbabilityWitnessReplay, ProbabilityWitnessStep,
    ReproducedProbabilityWitness,
};
use qsl_replay::{CertificateLocus, CertificateRule};
use qsl_replay::{InconclusiveCause, ProbabilityCertificateLocus, ProbabilityCertificateRule};
use qsl_route::Mode;
use qsl_semantics::check::{
    Bound, Capability, DeclarationFamily, ProbForm, ProbScheduler, ReachTarget,
};
use quire_exact::{Cancel, ObjectReference, Rational};

use super::items::{admission, BROKEN, NOT_YET};
use super::{AnalyzeItem, EngineReport, Refusal};
use crate::exact_probabilistic::{
    build_product, decide_reachability, manifest as en5, settle_product_refusal, ExtRational,
    ProbBuild, ProbProduct, ProductRefusal, ProductRequest, UnboundedError, UnboundedObjective,
};

/// The certificate EN-5 produced and the core's checker accepted for a
/// `proved` item (FR-281-AC-1: the record carries the certificate).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExactProbabilityEvidence {
    /// One exact value for every state of the checker's graph.
    pub certificate: ExactProbabilityCertificate,
    /// The checked value at the initial state.
    pub value: Rational,
    /// The authored bound side that was checked.
    pub side: ProbabilityBoundSide,
}

/// What EN-5 reports for one item.
pub(crate) enum ExactReport {
    /// Only independently reproduced path evidence can settle refuted.
    Refuted(Box<ReproducedProbabilityWitness>),
    /// A semantic replay disagreed with the produced path evidence.
    ReplayParity,
    /// Replay refused the witness's semantic content.
    ReplayRefused,
    /// The core rejected a probability value at a complete product state.
    Rejected {
        rule: CertificateRule,
        at: ProbabilityProductKey,
    },
    /// The checker accepted the solver's certificate.
    Proved {
        /// The checker's acceptance, the only evidence a `proved` record
        /// is built from.
        check: ExactProbabilityCertificateCheck,
        /// The scheduler extremum the value is, absent for a workload.
        extremum: Option<Extremum>,
        /// The claim's one `over` instance, absent without `over`.
        binding: Option<ExactBinding>,
        /// The certificate the record carries.
        evidence: Box<ExactProbabilityEvidence>,
    },
    /// FR-203's map settled a stop of the build or the check.
    Settled(ExactProbBuildRecord),
}

impl ExactReport {
    /// The terminal record and the evidence it carries.
    pub(crate) fn settle(
        self,
        index: RequestIndex,
        backend: BackendId,
    ) -> (
        TerminalRecord,
        Option<Box<ExactProbabilityEvidence>>,
        Option<Box<ReproducedProbabilityWitness>>,
    ) {
        let (record, evidence) = match self {
            Self::Refuted(reproduced) => {
                return (
                    TerminalRecord::refuted_by_probability_witness(
                        index,
                        backend,
                        &reproduced.evidence().item,
                        &reproduced,
                    )
                    .expect("the token's own claim matches"),
                    None,
                    Some(reproduced),
                )
            }
            Self::ReplayParity => (
                TerminalRecord::inconclusive(
                    index,
                    backend,
                    InconclusiveCause::ReplayParity(qsl_replay::DisagreementCause::NoValue {
                        proved: qsl_replay::Verdict::from_category(
                            quire_semantic_value::catalog::Category::Violation,
                        ),
                        replayed: qsl_replay::Verdict::from_category(
                            quire_semantic_value::catalog::Category::Inconclusive,
                        ),
                    }),
                ),
                None,
            ),
            Self::ReplayRefused => (
                TerminalRecord::inconclusive(
                    index,
                    backend,
                    InconclusiveCause::ReplayRefused(
                        qsl_foundation::diagnostic::Code::InvalidRuntimeInput,
                    ),
                ),
                None,
            ),
            Self::Rejected { rule, at } => (
                TerminalRecord::inconclusive(
                    index,
                    backend,
                    InconclusiveCause::CertificateRejected {
                        rule,
                        at: CertificateLocus::ProbabilityProductState(at),
                    },
                ),
                None,
            ),
            Self::Proved {
                check,
                extremum,
                binding,
                evidence,
            } => match TerminalRecord::proved_by_exact_certificate(
                index,
                backend.clone(),
                &check,
                extremum,
                binding,
            ) {
                Some(record) => (record, Some(evidence)),
                None => (TerminalRecord::failed_with(index, backend, BROKEN), None),
            },
            Self::Settled(record) => (record.into_terminal(index, backend), None),
        };
        (record, evidence, None)
    }
}

/// The `probabilistic-satisfaction` item that runs `request` on EN-5 and
/// settles it by FR-203. It is `Supported(EN-5)` until the caller's
/// negotiation says otherwise ([`AnalyzeItem::disposed`]).
pub fn exact_probabilistic_item<'a>(
    index: RequestIndex,
    request: ProductRequest<'a>,
) -> AnalyzeItem<'a> {
    AnalyzeItem::new(
        index,
        Capability::ProbabilisticSatisfaction,
        Some(DeclarationFamily::Probabilistic),
        Mode::Unbounded,
        en5::BACKEND_ID,
        move |cancel| run(request, cancel, None, None),
    )
}

/// [`exact_probabilistic_item`] whose certificate `tamper` alters after the
/// solver built it and before the core's checker sees it: the way a test
/// shows a rejected certificate never settles `proved`.
#[cfg(feature = "test-support")]
pub fn exact_probabilistic_item_tampered<'a>(
    index: RequestIndex,
    request: ProductRequest<'a>,
    tamper: impl Fn(&mut ExactProbabilityCertificate) + 'a,
) -> AnalyzeItem<'a> {
    AnalyzeItem::new(
        index,
        Capability::ProbabilisticSatisfaction,
        Some(DeclarationFamily::Probabilistic),
        Mode::Unbounded,
        en5::BACKEND_ID,
        move |cancel| run(request, cancel, Some(&tamper), None),
    )
}

fn not_yet() -> EngineReport {
    EngineReport::Refused(Refusal::Unsupported(NOT_YET))
}

fn broken() -> EngineReport {
    EngineReport::Refused(Refusal::Failed(BROKEN))
}

fn settled(issue: &ExactProbBuildIssue, subject: SubjectLimits<'_>) -> EngineReport {
    EngineReport::Exact(Box::new(ExactReport::Settled(
        qsl_replay::settle_exact_prob_build_issue(issue, subject),
    )))
}

fn run(
    request: ProductRequest<'_>,
    cancel: &Cancel,
    tamper: Option<&dyn Fn(&mut ExactProbabilityCertificate)>,
    tamper_witness: Option<&dyn Fn(&mut ProbabilisticCounterexample)>,
) -> EngineReport {
    let claim = request.claim;
    let simulation = request.subject.simulation.clone();
    let package = simulation.package;
    let limits = request.limits;
    let meter = simulation.meter;
    let subject = SubjectLimits {
        expansion: simulation.expansion,
        meter: &meter,
    };
    let ProbForm::Reach {
        bound,
        target: ReachTarget::Eventually(_),
    } = &claim.form
    else {
        return not_yet();
    };
    if !claim.fairness.is_empty() {
        return not_yet();
    }
    let (objective, extremum) = match (&claim.scheduler, bound) {
        (ProbScheduler::Every, Bound::AtLeast(_)) => (UnboundedObjective::Min, Some(Extremum::Min)),
        (ProbScheduler::Every, Bound::AtMost(_)) => (UnboundedObjective::Max, Some(Extremum::Max)),
        (ProbScheduler::Workload(_), _) => (UnboundedObjective::Min, None),
    };

    let product = match build_product(request, cancel) {
        Ok(ProbBuild::Built(product)) => product,
        // An undefined state refutes only after FR-202's replay.
        Ok(ProbBuild::Undefined(_)) => return not_yet(),
        Err(refusal) => return build_refusal(refusal, subject),
    };
    let [(_, _, start)] = product.initial.as_slice() else {
        return not_yet();
    };
    let target: Vec<bool> = product
        .labels
        .iter()
        .map(|labels| labels.first().copied().unwrap_or(false))
        .collect();
    let solved = match decide_reachability(&product, &target, objective, &limits) {
        Ok(solved) => solved,
        Err(UnboundedError::MaxPolicyIterations) => {
            return settled(
                &ExactProbBuildIssue::Limit {
                    limit: ModelCheckLimit::ExactProbMaxPolicyIterations,
                    value: limits.max_policy_iterations,
                },
                subject,
            )
        }
        Err(UnboundedError::MaxRationalBits) => {
            return settled(
                &ExactProbBuildIssue::Limit {
                    limit: ModelCheckLimit::ExactProbMaxRationalBits,
                    value: limits.max_rational_bits,
                },
                subject,
            )
        }
        Err(_) => return broken(),
    };
    let Some(ExtRational::Finite(value)) = solved.values.get(start.index()) else {
        return broken();
    };
    let holds = match bound {
        Bound::AtLeast(threshold) => value >= threshold,
        Bound::AtMost(threshold) => value <= threshold,
    };
    let Ok(item) = qsl_replay::probabilistic_claim_identity(claim) else {
        return not_yet();
    };
    let Ok(system) = ModelSystem::new(simulation) else {
        return broken();
    };
    let binding = match claim_binding(&system, claim) {
        Ok(binding) => binding,
        Err(ExactProbabilityGraphError::UnsupportedBinding) => return not_yet(),
        Err(_) => return broken(),
    };
    let reference = binding.as_ref().map(|bound| &bound.reference);
    if !holds {
        return replay_refutation(
            &system,
            claim,
            &product,
            &solved.policy,
            *start,
            &target,
            reference,
            &limits,
            cancel,
            subject,
            tamper_witness,
        );
    }
    let Ok(mut certificate) = certificate_of(&product, reference, &solved.values) else {
        return broken();
    };
    if let Some(tamper) = tamper {
        tamper(&mut certificate);
    }
    let checked = check_claim_exact_reachability(
        &system,
        package.graph().probabilistic_model().claims(),
        &item,
        &certificate,
        limits.model.limits.max_states,
        limits.model.limits.max_transitions,
        cancel,
    );
    let count = |limit: usize| u64::try_from(limit).unwrap_or(u64::MAX);
    match checked {
        Ok(check) => match &check {
            ExactProbabilityCertificateCheck::Accepted(accepted) => {
                let evidence = Box::new(ExactProbabilityEvidence {
                    certificate,
                    value: accepted.value().clone(),
                    side: accepted.side(),
                });
                let binding = binding.map(|bound| ExactBinding {
                    population: bound.instance.population.node,
                    key: bound.instance.key,
                });
                EngineReport::Exact(Box::new(ExactReport::Proved {
                    check,
                    extremum,
                    binding,
                    evidence,
                }))
            }
            ExactProbabilityCertificateCheck::Rejected(rejection) => {
                let rule = match rejection.rule {
                    ProbabilityCertificateRule::InequalityFails => CertificateRule::InequalityFails,
                    ProbabilityCertificateRule::ThresholdMissed => CertificateRule::ThresholdMissed,
                    // QSpec's probability rejection branch has no malformed
                    // rule/member form; retain the fault until that is specified.
                    ProbabilityCertificateRule::Malformed => return broken(),
                };
                match &rejection.at {
                    ProbabilityCertificateLocus::ProductState(at) => {
                        EngineReport::Exact(Box::new(ExactReport::Rejected {
                            rule,
                            at: at.clone(),
                        }))
                    }
                    ProbabilityCertificateLocus::Member(_) => broken(),
                }
            }
        },
        Err(ExactProbabilityGraphError::Cancelled(cause)) => {
            settled(&ExactProbBuildIssue::Cancelled(cause), subject)
        }
        Err(ExactProbabilityGraphError::LimitReached) => settled(
            &ExactProbBuildIssue::Limit {
                limit: ModelCheckLimit::ExactProbMaxStates,
                value: count(limits.model.limits.max_states),
            },
            subject,
        ),
        Err(
            ExactProbabilityGraphError::EdgeLimitReached | ExactProbabilityGraphError::TooManyEdges,
        ) => settled(
            &ExactProbBuildIssue::Limit {
                limit: ModelCheckLimit::ExactProbMaxTransitions,
                value: count(limits.model.limits.max_transitions),
            },
            subject,
        ),
        // Shapes the checker core does not cover yet.
        Err(
            ExactProbabilityGraphError::Cycle
            | ExactProbabilityGraphError::UnsupportedDraw
            | ExactProbabilityGraphError::UnsupportedInitial
            | ExactProbabilityGraphError::UnsupportedBinding
            | ExactProbabilityGraphError::UnsupportedClaimForm,
        ) => not_yet(),
        // The checker could not rebuild the graph the solver solved, or
        // could not bind the claim the item itself carries: a fault, not a
        // verdict.
        Err(
            ExactProbabilityGraphError::ExpansionRefused
            | ExactProbabilityGraphError::MissingSuccessor
            | ExactProbabilityGraphError::InvalidDigitalKey
            | ExactProbabilityGraphError::ClaimNotBound,
        ) => broken(),
    }
}

fn build_refusal(refusal: ProductRefusal, subject: SubjectLimits<'_>) -> EngineReport {
    match settle_product_refusal(refusal, subject) {
        Ok(record) => EngineReport::Exact(Box::new(ExactReport::Settled(record))),
        Err(ProductRefusal::Admission(failure)) => EngineReport::Refused(admission(&failure)),
        Err(ProductRefusal::NotSimulated(qsl_eval::simulation::NotSimulated::RequiresBound(_))) => {
            EngineReport::Refused(Refusal::RequiresBound)
        }
        Err(ProductRefusal::NotStochastic { .. }) => not_yet(),
        Err(_) => broken(),
    }
}

/// FR-201's certificate for `product`: the solved value of every state, under
/// the key the checker gives the same model state.
fn certificate_of(
    product: &ProbProduct,
    binding: Option<&ObjectReference>,
    values: &[ExtRational],
) -> Result<ExactProbabilityCertificate, ExactProbabilityGraphError> {
    let mut entries = Vec::with_capacity(product.states.len());
    for (state, value) in product.states.iter().zip(values) {
        let ExtRational::Finite(value) = value else {
            return Err(ExactProbabilityGraphError::ExpansionRefused);
        };
        entries.push(ExactProbabilityValue {
            state: ProbabilityProductKey::for_model_state(binding, &state.model)?,
            value: value.clone(),
        });
    }
    entries.sort_by(|a, b| a.state.cmp(&b.state));
    if entries
        .windows(2)
        .any(|pair| pair[0].state == pair[1].state)
    {
        return Err(ExactProbabilityGraphError::ExpansionRefused);
    }
    Ok(ExactProbabilityCertificate { values: entries })
}

#[allow(clippy::too_many_arguments)]
fn replay_refutation(
    system: &ModelSystem<'_>,
    claim: &qsl_semantics::check::CheckedProbabilisticClaim,
    product: &ProbProduct,
    policy: &[Option<usize>],
    initial: crate::exact_probabilistic::ProbStateId,
    target: &[bool],
    binding: Option<&ObjectReference>,
    limits: &crate::exact_probabilistic::ExactProbLimits,
    cancel: &Cancel,
    subject: SubjectLimits<'_>,
    tamper: Option<&dyn Fn(&mut ProbabilisticCounterexample)>,
) -> EngineReport {
    use crate::exact_probabilistic::{produce_witness, ProductAction, WitnessScope};
    let (accepting, past) = match &claim.form {
        ProbForm::Reach {
            bound: Bound::AtMost(threshold),
            ..
        } => (target.to_vec(), threshold.clone()),
        ProbForm::Reach {
            bound: Bound::AtLeast(threshold),
            ..
        } => (
            product
                .states
                .iter()
                .enumerate()
                .map(|(index, _)| {
                    !target.get(index).copied().unwrap_or(false)
                        && product
                            .edges
                            .iter()
                            .filter(|edge| edge.from.index() == index)
                            .all(|edge| matches!(edge.choice, ProductAction::TerminalStutter))
                })
                .collect(),
            Rational::from_integer(1_i64.into()).sub(threshold),
        ),
        _ => return not_yet(),
    };
    let scope = match claim.scheduler {
        ProbScheduler::Every => WitnessScope::EveryScheduler,
        ProbScheduler::Workload(_) => WitnessScope::Workload,
    };
    let produced = produce_witness(product, policy, scope, initial, &accepting, &past, limits);
    if cancel.poll() {
        if let Some(cause) = cancel.tripped() {
            return settled(&ExactProbBuildIssue::Cancelled(cause), subject);
        }
    }
    let produced = match produced {
        Ok(Some(witness)) => witness,
        Ok(None) => return not_yet(),
        Err(crate::exact_probabilistic::WitnessError::Limit { limit, value }) => {
            return settled(&ExactProbBuildIssue::Limit { limit, value }, subject);
        }
        Err(_) => return broken(),
    };
    let Ok(item) = qsl_replay::probabilistic_claim_identity(claim) else {
        return not_yet();
    };
    let initials = system.initial();
    let [first] = initials.as_slice() else {
        return not_yet();
    };
    let key_of = |state: &qsl_eval::simulation::model::ModelSimState| {
        let (bytes, _) = system.state_key(state).map_err(|_| ())?;
        ProbabilityProductKey::for_model_state(binding, &bytes).map_err(|_| ())
    };
    let Ok(initial_key) = key_of(first) else {
        return broken();
    };
    let mut scheduler = Vec::new();
    for (state, action) in produced.entries {
        // Terminal stutter carries no scheduler decision. Evidence paths
        // stop at first hits or independently checked nonreach leaves.
        if matches!(action, ProductAction::TerminalStutter) {
            continue;
        }
        let ProductAction::Identity(action) = action else {
            return not_yet();
        };
        let Some(state) = product.states.get(state.index()) else {
            return broken();
        };
        let Ok(key) = ProbabilityProductKey::for_model_state(binding, &state.model) else {
            return broken();
        };
        scheduler.push((key, action));
    }
    scheduler.sort_by(|a, b| a.0.cmp(&b.0));
    let mut paths = Vec::new();
    for path in produced.paths {
        let mut partials = vec![(first.clone(), Vec::new())];
        for step in path.steps {
            if cancel.poll() {
                if let Some(cause) = cancel.tripped() {
                    return settled(&ExactProbBuildIssue::Cancelled(cause), subject);
                }
            }
            let ProductAction::Identity(identity) = step.action else {
                return not_yet();
            };
            let Some(to) = product.states.get(step.to.index()) else {
                return broken();
            };
            let Ok(post_key) = ProbabilityProductKey::for_model_state(binding, &to.model) else {
                return broken();
            };
            let mut next = Vec::new();
            for (state, steps) in partials {
                let Ok(Ok(actions)) = system.actions(&state) else {
                    return broken();
                };
                let mut candidates = Vec::new();
                for action in actions {
                    if action.identity != identity {
                        continue;
                    }
                    for draw in action.draws {
                        if step.draw.as_ref().is_some_and(|expected| {
                            !qsl_eval::simulation::model::same_values(expected, &draw.random)
                        }) {
                            continue;
                        }
                        for (transition, post) in draw.successors {
                            if key_of(&post).ok().as_ref() == Some(&post_key) {
                                candidates.push(ProbabilityWitnessStep {
                                    action: identity.clone(),
                                    transition,
                                    draw: draw.random.clone(),
                                    post,
                                    post_key: post_key.clone(),
                                    elapsed: Rational::from_integer(0_i64.into()),
                                });
                            }
                        }
                    }
                }
                if candidates.is_empty() {
                    return not_yet();
                }
                for candidate in candidates {
                    if u64::try_from(next.len().saturating_add(paths.len())).unwrap_or(u64::MAX)
                        >= limits.max_witness_paths
                    {
                        return settled(
                            &ExactProbBuildIssue::Limit {
                                limit: ModelCheckLimit::ExactProbMaxWitnessPaths,
                                value: limits.max_witness_paths,
                            },
                            subject,
                        );
                    }
                    let mut extended = steps.clone();
                    let post = candidate.post.clone();
                    extended.push(candidate);
                    next.push((post, extended));
                }
            }
            partials = next;
        }
        paths.extend(
            partials
                .into_iter()
                .map(|(_, steps)| ProbabilityWitnessPath { steps }),
        );
    }
    let mut witness = ProbabilisticCounterexample {
        item: item.clone(),
        binding: binding.cloned(),
        initial: first.clone(),
        initial_key,
        scheduler,
        paths,
    };
    if let Some(tamper) = tamper {
        tamper(&mut witness);
    }
    match replay_probabilistic_witness(system.package(), system, &item, &witness, limits, cancel) {
        Ok(ProbabilityWitnessReplay::Reproduced(token)) => {
            EngineReport::Exact(Box::new(ExactReport::Refuted(token)))
        }
        Ok(ProbabilityWitnessReplay::Parity) => {
            EngineReport::Exact(Box::new(ExactReport::ReplayParity))
        }
        Err(ProbabilityWitnessError::Build(issue)) => settled(&issue, subject),
        Err(ProbabilityWitnessError::Refused) => {
            EngineReport::Exact(Box::new(ExactReport::ReplayRefused))
        }
        Err(ProbabilityWitnessError::Limit { limit, value }) => {
            settled(&ExactProbBuildIssue::Limit { limit, value }, subject)
        }
        Err(ProbabilityWitnessError::Graph(ExactProbabilityGraphError::Cancelled(cause))) => {
            settled(&ExactProbBuildIssue::Cancelled(cause), subject)
        }
        Err(ProbabilityWitnessError::Graph(ExactProbabilityGraphError::LimitReached)) => settled(
            &ExactProbBuildIssue::Limit {
                limit: ModelCheckLimit::ExactProbMaxStates,
                value: u64::try_from(limits.model.limits.max_states).unwrap_or(u64::MAX),
            },
            subject,
        ),
        Err(ProbabilityWitnessError::Graph(
            ExactProbabilityGraphError::EdgeLimitReached | ExactProbabilityGraphError::TooManyEdges,
        )) => settled(
            &ExactProbBuildIssue::Limit {
                limit: ModelCheckLimit::ExactProbMaxTransitions,
                value: u64::try_from(limits.model.limits.max_transitions).unwrap_or(u64::MAX),
            },
            subject,
        ),
        Err(ProbabilityWitnessError::Graph(
            ExactProbabilityGraphError::Cycle
            | ExactProbabilityGraphError::UnsupportedDraw
            | ExactProbabilityGraphError::UnsupportedInitial
            | ExactProbabilityGraphError::UnsupportedBinding
            | ExactProbabilityGraphError::UnsupportedClaimForm,
        )) => not_yet(),
        Err(ProbabilityWitnessError::Graph(
            ExactProbabilityGraphError::ExpansionRefused
            | ExactProbabilityGraphError::MissingSuccessor
            | ExactProbabilityGraphError::InvalidDigitalKey
            | ExactProbabilityGraphError::ClaimNotBound,
        )) => broken(),
    }
}

/// Test seam altering produced typed evidence before independent replay.
#[cfg(feature = "test-support")]
pub fn exact_probabilistic_item_witness_tampered<'a>(
    index: RequestIndex,
    request: ProductRequest<'a>,
    tamper: impl Fn(&mut ProbabilisticCounterexample) + 'a,
) -> AnalyzeItem<'a> {
    AnalyzeItem::new(
        index,
        Capability::ProbabilisticSatisfaction,
        Some(DeclarationFamily::Probabilistic),
        Mode::Unbounded,
        en5::BACKEND_ID,
        move |cancel| run(request, cancel, None, Some(&tamper)),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;
    use qsl_replay::TerminalValue;

    #[trace("TC-1197", "FR-201-AC-6", "FR-203-AC-3")]
    #[test]
    fn threshold_rejection_retains_its_rule_and_complete_product_key() {
        let key = ProbabilityProductKey::for_model_state(None, b"complete-model-state")
            .expect("the key encodes");
        let (terminal, evidence, counterexample) = ExactReport::Rejected {
            rule: CertificateRule::ThresholdMissed,
            at: key.clone(),
        }
        .settle(RequestIndex::new(0), BackendId::new("en-5"));
        assert_eq!(
            terminal.value(),
            &TerminalValue::Inconclusive(InconclusiveCause::CertificateRejected {
                rule: CertificateRule::ThresholdMissed,
                at: CertificateLocus::ProbabilityProductState(key),
            })
        );
        assert!(evidence.is_none());
        assert!(counterexample.is_none());
    }
}
