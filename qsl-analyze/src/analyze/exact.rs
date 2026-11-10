// SPDX-License-Identifier: AGPL-3.0-or-later
//! EN-5's item adapter (FR-281, FR-203) builds and solves the supported
//! probabilistic products, writes the corresponding FR-201 document, and
//! settles `proved` only after independent source-subject checking accepts it.
//!
//! Each closed document profile owns its graph, binding and workload admission.
//! Unsupported forms remain explicit unsupported dispositions. Missed bounds
//! settle `refuted` only when the profile has native witness production and
//! independent FR-202 replay; long-run document evidence authorizes proofs.

mod candidate;
mod long_run;

use qsl_eval::simulation::model::ModelSystem;
use qsl_eval::simulation::TransitionSystem;
use qsl_foundation::{BackendId, RequestIndex};
use qsl_replay::statistical::SubjectLimits;
use qsl_replay::{
    check_probability_certificate_document, claim_binding, probability_certificate_document,
    write_probability_certificate, ExactBinding, ExactProbBuildIssue, ExactProbBuildRecord,
    ExactProbabilityCertificate, ExactProbabilityCertificateCheck, ExactProbabilityGraphError,
    ExactProbabilityValue, Extremum, ModelCheckLimit, ProbabilityBoundSide,
    ProbabilityDocumentError, ProbabilityDocumentLimits, ProbabilityProductKey, TerminalRecord,
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
    Bound, Capability, DeclarationFamily, Event, ProbForm, ProbScheduler, ReachTarget,
};
use quire_exact::{Cancel, ObjectReference, Rational};

use super::items::{admission, BROKEN, NOT_YET};
use super::{AnalyzeItem, EngineReport, Refusal};
use crate::exact_probabilistic::{
    build_retaining_monitor, decide_always, decide_bounded_event, decide_expected_reward,
    decide_reachability, decide_until, manifest as en5, settle_product_refusal, ExtRational,
    FhResult, ProbBuild, ProbProduct, ProductRefusal, ProductRequest, UnboundedError,
    UnboundedObjective,
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
        at: CertificateLocus,
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
    /// A construction overflow or actual allocator refusal exhausted resources.
    ResourceExhausted,
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
                    InconclusiveCause::CertificateRejected { rule, at },
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
            Self::ResourceExhausted => (
                TerminalRecord::incomplete(
                    index,
                    backend,
                    qsl_replay::IncompleteCause::ResourceExhausted,
                ),
                None,
            ),
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
    if matches!(request.claim.form, ProbForm::LongRunFraction { .. }) {
        return long_run::run(request, cancel, tamper);
    }
    let claim = request.claim;
    let simulation = request.subject.simulation.clone();
    let package = simulation.package;
    let limits = request.limits;
    let meter = simulation.meter;
    let subject = SubjectLimits {
        expansion: simulation.expansion,
        meter: &meter,
    };
    let (bound, bounded_formula) = match &claim.form {
        ProbForm::Reach {
            bound,
            target: ReachTarget::Eventually(_) | ReachTarget::Until(..) | ReachTarget::Always(_),
        } => (bound, None),
        ProbForm::ExpectedReward { bound, .. } => (bound, None),
        ProbForm::Probability {
            bound,
            event:
                Event::Formula {
                    formula,
                    timed_horizon: None,
                    ..
                },
        } => (bound, Some(formula)),
        _ => return not_yet(),
    };
    let state_predicate = matches!(
        &claim.form,
        ProbForm::Reach {
            target: ReachTarget::Until(..) | ReachTarget::Always(_),
            ..
        }
    );
    let expected_reward = matches!(&claim.form, ProbForm::ExpectedReward { .. });
    if (state_predicate || expected_reward)
        && qsl_semantics::check::TemporalProfile::from_identity(claim.profile.identity())
            != Some(qsl_semantics::check::TemporalProfile::InfiniteTrace)
    {
        return not_yet();
    }
    if !claim.fairness.is_empty() {
        return not_yet();
    }
    let document_limits = ProbabilityDocumentLimits {
        bytes: limits.document.bytes,
        storage_bytes: limits.document.storage_bytes,
        states: limits.model.limits.max_states,
        edges: limits.model.limits.max_transitions,
        scalar_bits: limits.max_rational_bits,
        accounting: meter,
    };
    let reward_system = if expected_reward {
        let Ok(item) = qsl_replay::probabilistic_claim_identity(claim) else {
            return not_yet();
        };
        let admitted = ModelSystem::new_with_cancel(simulation.clone(), cancel);
        if let Some(cause) = cancel.tripped() {
            return settled(&ExactProbBuildIssue::Cancelled(cause), subject);
        }
        let system = match admitted {
            Ok(system) => system,
            Err(error) => return build_refusal(ProductRefusal::Admission(error), subject),
        };
        if let Err(error) = qsl_replay::admit_expected_reward_certificate_subject(
            &system,
            package.graph().probabilistic_model().claims(),
            &item,
            document_limits,
            cancel,
        ) {
            return document_failure(error, subject);
        }
        Some(system)
    } else {
        None
    };
    if let Some(formula) = bounded_formula {
        let Some(profile) =
            qsl_semantics::check::TemporalProfile::from_identity(claim.profile.identity())
        else {
            return not_yet();
        };
        if !qsl_eval::automaton::QualifiedBoundedMonitor::supports(formula.formula(), profile) {
            return not_yet();
        }
    }
    let (objective, extremum) = match (&claim.scheduler, bound) {
        (ProbScheduler::Every, Bound::AtLeast(_)) => (UnboundedObjective::Min, Some(Extremum::Min)),
        (ProbScheduler::Every, Bound::AtMost(_)) => (UnboundedObjective::Max, Some(Extremum::Max)),
        (ProbScheduler::Workload(_), _) => (UnboundedObjective::Min, None),
    };

    let mut candidate_budget =
        expected_reward.then(|| candidate::CandidateBudget::new(document_limits, cancel));
    let built = if let Some(budget) = candidate_budget.as_mut() {
        use crate::exact_probabilistic::{
            build_retaining_monitor_with_budget, BudgetedProductError,
        };
        let identity = budget.identity;
        let mut admission = budget.admission();
        match build_retaining_monitor_with_budget(request, identity, &mut admission, cancel) {
            Ok(built) => built,
            Err(BudgetedProductError::Product(error)) => return build_refusal(error, subject),
            Err(BudgetedProductError::Key(error)) => {
                return document_failure(admission.refusal(error), subject)
            }
            Err(BudgetedProductError::Undefined(cause)) => {
                return document_failure(
                    ProbabilityDocumentError::UndefinedPredicate(cause),
                    subject,
                )
            }
        }
    } else {
        match build_retaining_monitor(request, cancel) {
            Ok(built) => built,
            Err(error) => return build_refusal(error, subject),
        }
    };
    let (product, monitor) = match built {
        (ProbBuild::Built(product), monitor) => (product, monitor),
        (ProbBuild::Undefined(_), _) => return not_yet(),
    };
    let [(_, _, start)] = product.initial.as_slice() else {
        return not_yet();
    };
    if bounded_formula.is_some()
        && product
            .states
            .iter()
            .any(|state| state.pending.is_some() || state.accumulator.is_some())
    {
        return not_yet();
    }
    let target: Vec<bool> = product
        .labels
        .iter()
        .map(|labels| labels.first().copied().unwrap_or(false))
        .collect();
    let (values, policy) = if bounded_formula.is_some() {
        match decide_bounded_event(&product, bound, &limits) {
            Ok(FhResult::Exact { values, policy }) => (
                values.into_iter().map(ExtRational::Finite).collect(),
                policy,
            ),
            Ok(FhResult::Bounds { .. }) => return not_yet(),
            Ok(FhResult::PrecisionBudget { lower, upper, .. }) => {
                return settled(
                    &ExactProbBuildIssue::PrecisionBudget {
                        lower: lower.rational(),
                        upper: upper.rational(),
                    },
                    subject,
                )
            }
            Err(_) => return broken(),
        }
    } else {
        let result = match &claim.form {
            ProbForm::Reach {
                target: ReachTarget::Until(..),
                ..
            } => {
                let until = product
                    .labels
                    .iter()
                    .map(|labels| labels.get(1).copied().unwrap_or(false))
                    .collect::<Vec<_>>();
                decide_until(&product, &target, &until, objective, &limits)
            }
            ProbForm::Reach {
                target: ReachTarget::Always(_),
                ..
            } => decide_always(&product, &target, objective, &limits),
            ProbForm::ExpectedReward { reward, .. } => {
                decide_expected_reward(&product, &target, reward.as_str(), objective, &limits)
            }
            _ => decide_reachability(&product, &target, objective, &limits),
        };
        let solved = match result {
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
        (solved.values, solved.policy)
    };
    let Some(ExtRational::Finite(value)) = values.get(start.index()) else {
        return if expected_reward { not_yet() } else { broken() };
    };
    let holds = match bound {
        Bound::AtLeast(threshold) => value >= threshold,
        Bound::AtMost(threshold) => value <= threshold,
    };
    let Ok(item) = qsl_replay::probabilistic_claim_identity(claim) else {
        return not_yet();
    };
    let admitted = match reward_system {
        Some(system) => Ok(system),
        None => ModelSystem::new_with_cancel(simulation, cancel),
    };
    if let Some(cause) = cancel.tripped() {
        return settled(&ExactProbBuildIssue::Cancelled(cause), subject);
    }
    let Ok(system) = admitted else {
        return broken();
    };
    let binding = match claim_binding(&system, claim) {
        Ok(binding) => binding,
        Err(ExactProbabilityGraphError::UnsupportedBinding) => return not_yet(),
        Err(_) => return broken(),
    };
    let reference = binding.as_ref().map(|bound| &bound.reference);
    if !holds {
        if expected_reward {
            return not_yet();
        }
        if matches!(
            claim.form,
            ProbForm::Reach {
                target: ReachTarget::Eventually(_),
                ..
            }
        ) {
            match qsl_replay::admit_acyclic_probabilistic_refutation_subject(
                &system, &limits, cancel,
            ) {
                Ok(true) => {}
                Ok(false) => return not_yet(),
                Err(issue) => return settled(&issue, subject),
            }
        }
        let bounded_monitor = match (bounded_formula, monitor) {
            (Some(formula), Some(monitor)) => {
                let context =
                    match qsl_replay::probabilistic_formula_monitor_context(claim, reference) {
                        Ok(context) => context,
                        Err(_) => return not_yet(),
                    };
                match qsl_eval::automaton::QualifiedBoundedMonitor::from_monitor(
                    formula.formula(),
                    &context,
                    monitor,
                    limits.model.max_automaton_states,
                ) {
                    Ok(monitor) => Some(monitor),
                    Err(_) => return not_yet(),
                }
            }
            (None, _) => None,
            (Some(_), None) => return broken(),
        };
        return replay_refutation(
            &system,
            claim,
            &product,
            &policy,
            *start,
            &target,
            reference,
            bounded_monitor.as_ref(),
            &limits,
            cancel,
            subject,
            tamper_witness,
        );
    }
    let certificate = match (bounded_formula, monitor) {
        (Some(formula), Some(monitor)) => {
            let context = match qsl_replay::probabilistic_formula_monitor_context(claim, reference)
            {
                Ok(context) => context,
                Err(_) => return not_yet(),
            };
            let monitor = match qsl_eval::automaton::QualifiedBoundedMonitor::from_monitor(
                formula.formula(),
                &context,
                monitor,
                limits.model.max_automaton_states,
            ) {
                Ok(monitor) => monitor,
                Err(qsl_eval::automaton::QualifiedBoundedError::Translation(_)) => {
                    return not_yet()
                }
                Err(_) => return broken(),
            };
            bounded_certificate_of(
                &product,
                reference,
                &values,
                &monitor,
                qsl_replay::ProbabilityDocumentIoLimits {
                    bytes: document_limits.bytes,
                    storage_bytes: document_limits.storage_bytes,
                },
                cancel,
            )
        }
        (None, _) if candidate_budget.is_some() => candidate::values(
            &product,
            reference,
            &values,
            candidate_budget.as_mut().expect("reward candidate budget"),
        ),
        (None, _) => certificate_of(
            &product,
            reference,
            &values,
            if state_predicate || expected_reward {
                qsl_replay::ProbabilityDocumentIoLimits {
                    bytes: document_limits.bytes,
                    storage_bytes: document_limits.storage_bytes,
                }
            } else {
                qsl_replay::ProbabilityDocumentIoLimits::default()
            },
            cancel,
        ),
        (Some(_), None) => return broken(),
    };
    let mut certificate = match certificate {
        Ok(certificate) => certificate,
        Err(error) => return document_failure(error, subject),
    };
    if let Some(tamper) = tamper {
        tamper(&mut certificate);
    }
    let checked = (|| {
        let claims = package.graph().probabilistic_model().claims();
        let document = probability_certificate_document(
            &system,
            claims,
            &item,
            &certificate,
            document_limits,
            cancel,
        )?;
        certificate.values = document.values.clone();
        let bytes = write_probability_certificate(&document, document_limits, cancel)?;
        let check = check_probability_certificate_document(
            &system,
            claims,
            &item,
            &bytes,
            document_limits,
            cancel,
        )?;
        Ok::<_, ProbabilityDocumentError>(check)
    })();
    checked_report(checked, certificate, binding, extremum, subject)
}

fn checked_report(
    checked: Result<ExactProbabilityCertificateCheck, ProbabilityDocumentError>,
    certificate: ExactProbabilityCertificate,
    binding: Option<qsl_replay::ClaimBinding>,
    extremum: Option<Extremum>,
    subject: SubjectLimits<'_>,
) -> EngineReport {
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
                    ProbabilityCertificateRule::Malformed => CertificateRule::Malformed,
                    ProbabilityCertificateRule::RankMissing => CertificateRule::RankMissing,
                    ProbabilityCertificateRule::RankBroken => CertificateRule::RankBroken,
                };
                match &rejection.at {
                    ProbabilityCertificateLocus::ProductState(at) => {
                        EngineReport::Exact(Box::new(ExactReport::Rejected {
                            rule,
                            at: CertificateLocus::ProbabilityProductState(at.clone()),
                        }))
                    }
                    ProbabilityCertificateLocus::Member(name) => {
                        let Some(member) = qsl_replay::CertificateMember::ALL
                            .iter()
                            .find(|member| member.as_str() == *name)
                        else {
                            return broken();
                        };
                        EngineReport::Exact(Box::new(ExactReport::Rejected {
                            rule,
                            at: CertificateLocus::CertificateMember(*member),
                        }))
                    }
                }
            }
        },
        Err(error) => document_failure(error, subject),
    }
}

fn document_failure(error: ProbabilityDocumentError, subject: SubjectLimits<'_>) -> EngineReport {
    match error {
        ProbabilityDocumentError::Cancelled(cause) => {
            settled(&ExactProbBuildIssue::Cancelled(cause), subject)
        }
        ProbabilityDocumentError::Limit { limit, bound, .. } => settled(
            &ExactProbBuildIssue::Limit {
                limit: limit.model_limit(),
                value: bound,
            },
            subject,
        ),
        ProbabilityDocumentError::Accounting(incomplete) => settled(
            &ExactProbBuildIssue::Limit {
                limit: ModelCheckLimit::EvaluationMeter {
                    counter: incomplete.limit_kind,
                },
                value: incomplete.limit,
            },
            subject,
        ),
        ProbabilityDocumentError::IntegerText { bit_bound, .. } => settled(
            &ExactProbBuildIssue::Limit {
                limit: ModelCheckLimit::ExactProbMaxRationalBits,
                value: bit_bound,
            },
            subject,
        ),
        ProbabilityDocumentError::KeyEncoding(qsl_eval::simulation::EncodingRefusal::Limit(
            limit,
        )) => settled(
            &ExactProbBuildIssue::Limit {
                limit: ModelCheckLimit::IdentityInputBytes,
                value: limit.configured_bound(),
            },
            subject,
        ),
        ProbabilityDocumentError::KeyEncoding(
            qsl_eval::simulation::EncodingRefusal::Allocation { .. },
        )
        | ProbabilityDocumentError::KeySizeOverflow => {
            EngineReport::Exact(Box::new(ExactReport::ResourceExhausted))
        }
        ProbabilityDocumentError::KeyEncoding(
            qsl_eval::simulation::EncodingRefusal::NoEncoding(_),
        ) => EngineReport::Refused(Refusal::Declined(
            qsl_replay::ProofRefusalCause::InvalidInput,
            qsl_replay::DeclineCode::Qsl(qsl_foundation::diagnostic::Code::InvalidRuntimeInput),
            quire_semantic_value::catalog::CatalogCode::new(
                "invalid_runtime_input",
                "invalid-value",
            ),
        )),
        ProbabilityDocumentError::KeyDenied => broken(),
        ProbabilityDocumentError::Build(issue) => settled(&issue, subject),
        ProbabilityDocumentError::Unsupported | ProbabilityDocumentError::UndefinedPredicate(_) => {
            not_yet()
        }
        ProbabilityDocumentError::MalformedMember(_)
        | ProbabilityDocumentError::Malformed
        | ProbabilityDocumentError::MissingMember(_)
        | ProbabilityDocumentError::Noncanonical
        | ProbabilityDocumentError::IdentityMismatch
        | ProbabilityDocumentError::ObjectiveMismatch
        | ProbabilityDocumentError::Graph(_) => broken(),
    }
}

fn bounded_certificate_of(
    product: &ProbProduct,
    binding: Option<&ObjectReference>,
    values: &[ExtRational],
    monitor: &qsl_eval::automaton::QualifiedBoundedMonitor,
    io: qsl_replay::ProbabilityDocumentIoLimits,
    cancel: &Cancel,
) -> Result<ExactProbabilityCertificate, ProbabilityDocumentError> {
    if values.len() != product.states.len() {
        return Err(ExactProbabilityGraphError::MissingSuccessor.into());
    }
    let mut entries = Vec::with_capacity(product.states.len());
    for (state, value) in product.states.iter().zip(values) {
        if state.accumulator.is_some() || state.pending.is_some() {
            return Err(ExactProbabilityGraphError::UnsupportedDraw.into());
        }
        let ExtRational::Finite(value) = value else {
            return Err(ExactProbabilityGraphError::ExpansionRefused.into());
        };
        entries.push(ExactProbabilityValue {
            state: bounded_product_key(state, binding, monitor, io, cancel)?,
            value: value.clone(),
        });
    }
    entries.sort_by(|left, right| left.state.cmp(&right.state));
    if entries
        .windows(2)
        .any(|pair| pair[0].state == pair[1].state)
    {
        return Err(ExactProbabilityGraphError::ExpansionRefused.into());
    }
    Ok(ExactProbabilityCertificate { values: entries })
}

fn bounded_product_key(
    state: &crate::exact_probabilistic::ProbKey,
    binding: Option<&ObjectReference>,
    monitor: &qsl_eval::automaton::QualifiedBoundedMonitor,
    io: qsl_replay::ProbabilityDocumentIoLimits,
    cancel: &Cancel,
) -> Result<ProbabilityProductKey, ProbabilityDocumentError> {
    use crate::exact_probabilistic::{FormulaState, MonitorState, MonitorVerdict};
    let key = match state.monitor {
        Some(MonitorState::Formula(FormulaState::Running(state))) => monitor.key(state),
        Some(MonitorState::Formula(FormulaState::Decided(verdict))) => {
            monitor.decision_key(verdict == MonitorVerdict::Accept)
        }
        _ => return Err(ExactProbabilityGraphError::UnsupportedClaimForm.into()),
    }
    .map_err(|_| ExactProbabilityGraphError::ExpansionRefused)?;
    ProbabilityProductKey::for_formula_state(binding, &state.model, &key, io, cancel)
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
    io: qsl_replay::ProbabilityDocumentIoLimits,
    cancel: &Cancel,
) -> Result<ExactProbabilityCertificate, ProbabilityDocumentError> {
    if values.len() != product.states.len() {
        return Err(ExactProbabilityGraphError::MissingSuccessor.into());
    }
    let mut storage = 0_u64;
    let rows = u64::try_from(product.states.len()).unwrap_or(u64::MAX);
    reserve_certificate_storage(
        &mut storage,
        io.storage_bytes,
        rows.saturating_mul(std::mem::size_of::<ExactProbabilityValue>() as u64),
    )?;
    for value in values {
        let ExtRational::Finite(value) = value else {
            return Err(ExactProbabilityGraphError::ExpansionRefused.into());
        };
        reserve_certificate_storage(
            &mut storage,
            io.storage_bytes,
            value
                .numerator()
                .magnitude_bits()
                .div_ceil(8)
                .saturating_add(value.denominator().magnitude_bits().div_ceil(8)),
        )?;
    }
    let mut entries = Vec::with_capacity(product.states.len());
    for (state, value) in product.states.iter().zip(values) {
        let ExtRational::Finite(value) = value else {
            return Err(ExactProbabilityGraphError::ExpansionRefused.into());
        };
        let key = ProbabilityProductKey::for_model_state_with_limits(
            binding,
            &state.model,
            qsl_replay::ProbabilityDocumentIoLimits {
                storage_bytes: io.storage_bytes.saturating_sub(storage),
                ..io
            },
            cancel,
        )?;
        reserve_certificate_storage(&mut storage, io.storage_bytes, key.as_bytes().len() as u64)?;
        entries.push(ExactProbabilityValue {
            state: key,
            value: value.clone(),
        });
    }
    entries.sort_unstable_by(|a, b| a.state.cmp(&b.state));
    if entries
        .windows(2)
        .any(|pair| pair[0].state == pair[1].state)
    {
        return Err(ExactProbabilityGraphError::ExpansionRefused.into());
    }
    Ok(ExactProbabilityCertificate { values: entries })
}

fn reserve_certificate_storage(
    used: &mut u64,
    bound: u64,
    bytes: u64,
) -> Result<(), ProbabilityDocumentError> {
    let actual = used.saturating_add(bytes);
    if actual > bound {
        return Err(ProbabilityDocumentError::Limit {
            limit: qsl_replay::ProbabilityDocumentLimit::StorageBytes,
            bound,
            actual,
        });
    }
    *used = actual;
    Ok(())
}

fn witness_payload_bytes(
    work: crate::exact_probabilistic::WitnessWork<'_>,
    system: &ModelSystem<'_>,
) -> Result<usize, Box<ExactProbBuildIssue>> {
    use crate::exact_probabilistic::{ProductAction, WitnessWork};
    let action_bytes = |action: &ProductAction| -> Result<usize, Box<ExactProbBuildIssue>> {
        match action {
            ProductAction::Identity(identity) => system
                .scheduled_identity_key(identity)
                .map(|bytes| bytes.len())
                .map_err(|stop| Box::new(ExactProbBuildIssue::Expansion(stop))),
            ProductAction::TerminalStutter => Ok(0),
            _ => Err(Box::new(ExactProbBuildIssue::BrokenInvariant)),
        }
    };
    match work {
        WitnessWork::Visit | WitnessWork::MaterializedStep => Ok(0),
        WitnessWork::Rational(probability) => {
            Ok([probability.numerator(), probability.denominator()]
                .iter()
                .map(|integer| {
                    usize::try_from(integer.as_big().bits().div_ceil(8)).unwrap_or(usize::MAX)
                })
                .fold(0usize, usize::saturating_add))
        }
        WitnessWork::Slots(bytes) => Ok(bytes),
        WitnessWork::Action(action) => action_bytes(action),
        WitnessWork::Step {
            action,
            draw,
            probability,
            rewards,
        } => {
            let mut bytes = action_bytes(action)?;
            let mut value_bytes =
                |value: &quire_exact::Value| -> Result<(), Box<ExactProbBuildIssue>> {
                    let key = system
                        .value_key(value)
                        .map_err(|_| Box::new(ExactProbBuildIssue::BrokenInvariant))?;
                    bytes = bytes.saturating_add(key.len());
                    Ok(())
                };
            for value in draw.into_iter().flatten() {
                value_bytes(value)?;
            }
            for (_, value) in rewards {
                value_bytes(value)?;
            }
            bytes = bytes.saturating_add(draw.map_or(0, |draw| {
                draw.len()
                    .saturating_mul(std::mem::size_of::<quire_exact::Value>())
            }));
            for (name, _) in rewards {
                bytes =
                    bytes
                        .saturating_add(name.as_str().len())
                        .saturating_add(std::mem::size_of::<(
                            quire_exact::Identifier,
                            quire_exact::Value,
                        )>());
            }
            for integer in [probability.numerator(), probability.denominator()] {
                bytes = bytes.saturating_add(
                    usize::try_from(integer.as_big().bits().div_ceil(8)).unwrap_or(usize::MAX),
                );
            }
            Ok(bytes)
        }
    }
}

fn witness_charge(
    meter: &mut quire_exact::Meter,
    storage: &mut u64,
    bytes: usize,
    limits: &crate::exact_probabilistic::ExactProbLimits,
    cancel: &Cancel,
) -> Result<(), Box<ExactProbBuildIssue>> {
    if cancel.poll() {
        if let Some(cause) = cancel.tripped() {
            return Err(Box::new(ExactProbBuildIssue::Cancelled(cause)));
        }
    }
    *storage = storage.saturating_add(u64::try_from(bytes).unwrap_or(u64::MAX));
    if *storage > limits.document.storage_bytes {
        return Err(Box::new(ExactProbBuildIssue::Limit {
            limit: ModelCheckLimit::CertificateMaxWorkingBytes,
            value: limits.document.storage_bytes,
        }));
    }
    meter
        .charge(quire_exact::Charge::new(
            quire_exact::ChargePoint::CollectionVisit,
        ))
        .map_err(|incomplete| {
            if let Some(cause) = cancel.tripped() {
                return Box::new(ExactProbBuildIssue::Cancelled(cause));
            }
            Box::new(ExactProbBuildIssue::Predicate(
                qsl_eval::simulation::model::ClauseOutcome::Incomplete(incomplete),
            ))
        })
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
    monitor: Option<&qsl_eval::automaton::QualifiedBoundedMonitor>,
    limits: &crate::exact_probabilistic::ExactProbLimits,
    cancel: &Cancel,
    subject: SubjectLimits<'_>,
    tamper: Option<&dyn Fn(&mut ProbabilisticCounterexample)>,
) -> EngineReport {
    use crate::exact_probabilistic::{
        produce_witness, FormulaState, MonitorState, MonitorVerdict, ProductAction, WitnessScope,
    };
    let mut producer_meter =
        quire_exact::Meter::new(system.meter_limits()).with_cancel(cancel.clone());
    let mut producer_storage = 0u64;
    // Classify direct state boundaries in one pass over the source-labelled
    // product. The replay core independently derives the same outcomes.
    let state_outcomes = match &claim.form {
        ProbForm::Reach {
            target: target @ (ReachTarget::Until(..) | ReachTarget::Always(_)),
            ..
        } => {
            if let Err(issue) = witness_charge(
                &mut producer_meter,
                &mut producer_storage,
                product.states.len().saturating_mul(
                    std::mem::size_of::<bool>() + std::mem::size_of::<Option<bool>>(),
                ),
                limits,
                cancel,
            ) {
                return settled(&issue, subject);
            }
            let mut terminal = Vec::new();
            let mut outcomes = Vec::new();
            if terminal.try_reserve(product.states.len()).is_err()
                || outcomes.try_reserve(product.states.len()).is_err()
            {
                return settled(
                    &ExactProbBuildIssue::Limit {
                        limit: ModelCheckLimit::CertificateMaxWorkingBytes,
                        value: limits.document.storage_bytes,
                    },
                    subject,
                );
            }
            terminal.resize(product.states.len(), true);
            for edge in &product.edges {
                if let Err(issue) = witness_charge(
                    &mut producer_meter,
                    &mut producer_storage,
                    0,
                    limits,
                    cancel,
                ) {
                    return settled(&issue, subject);
                }
                if !matches!(
                    edge.choice,
                    ProductAction::Identity(_) | ProductAction::TerminalStutter
                ) {
                    return not_yet();
                }
                if !matches!(edge.choice, ProductAction::TerminalStutter) {
                    let Some(terminal) = terminal.get_mut(edge.from.index()) else {
                        return broken();
                    };
                    *terminal = false;
                }
            }
            for (index, labels) in product.labels.iter().enumerate() {
                if let Err(issue) = witness_charge(
                    &mut producer_meter,
                    &mut producer_storage,
                    0,
                    limits,
                    cancel,
                ) {
                    return settled(&issue, subject);
                }
                let Some(terminal) = terminal.get(index).copied() else {
                    return broken();
                };
                let first = labels.first().copied().unwrap_or(false);
                let verdict = match target {
                    ReachTarget::Until(..) => {
                        if labels.get(1).copied().unwrap_or(false) {
                            Some(true)
                        } else if !first || terminal {
                            Some(false)
                        } else {
                            None
                        }
                    }
                    ReachTarget::Always(_) => {
                        if !first {
                            Some(false)
                        } else if terminal {
                            Some(true)
                        } else {
                            None
                        }
                    }
                    ReachTarget::Eventually(_) => return broken(),
                };
                outcomes.push(verdict);
            }
            Some(outcomes)
        }
        _ => None,
    };
    let (accepting, past) = match (&claim.form, state_outcomes.as_deref()) {
        (ProbForm::Reach { bound, .. }, Some(outcomes)) => (
            outcomes
                .iter()
                .map(|outcome| *outcome == Some(matches!(bound, Bound::AtMost(_))))
                .collect(),
            match bound {
                Bound::AtMost(threshold) => threshold.clone(),
                Bound::AtLeast(threshold) => Rational::from_integer(1_i64.into()).sub(threshold),
            },
        ),
        (ProbForm::Probability { bound, .. }, _) if monitor.is_some() => (
            product
                .states
                .iter()
                .map(|state| {
                    matches!(
                        (bound, state.monitor),
                        (
                            Bound::AtMost(_),
                            Some(MonitorState::Formula(FormulaState::Decided(
                                MonitorVerdict::Accept
                            )))
                        ) | (
                            Bound::AtLeast(_),
                            Some(MonitorState::Formula(FormulaState::Decided(
                                MonitorVerdict::Reject
                            )))
                        )
                    )
                })
                .collect(),
            match bound {
                Bound::AtMost(threshold) => threshold.clone(),
                Bound::AtLeast(threshold) => Rational::from_integer(1_i64.into()).sub(threshold),
            },
        ),
        (
            ProbForm::Reach {
                bound: Bound::AtMost(threshold),
                ..
            },
            _,
        ) => (target.to_vec(), threshold.clone()),
        (
            ProbForm::Reach {
                bound: Bound::AtLeast(threshold),
                ..
            },
            _,
        ) => (
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
    let stopped = state_outcomes
        .as_ref()
        .map(|outcomes| outcomes.iter().map(Option::is_some).collect::<Vec<_>>());
    let mut producer_issue = None;
    let mut control = |work: crate::exact_probabilistic::WitnessWork<'_>| {
        let result = witness_payload_bytes(work, system).and_then(|bytes| {
            witness_charge(
                &mut producer_meter,
                &mut producer_storage,
                bytes,
                limits,
                cancel,
            )
        });
        result.map_err(|issue| {
            producer_issue = Some(issue);
            crate::exact_probabilistic::WitnessError::InvalidState
        })
    };
    let produced = if let Some(stopped) = stopped.as_deref() {
        crate::exact_probabilistic::produce_witness_stopping(
            product,
            policy,
            scope,
            initial,
            &accepting,
            Some(stopped),
            &past,
            limits,
            &mut control,
        )
    } else {
        produce_witness(product, policy, scope, initial, &accepting, &past, limits)
    };
    if cancel.poll() {
        if let Some(cause) = cancel.tripped() {
            return settled(&ExactProbBuildIssue::Cancelled(cause), subject);
        }
    }
    if let Some(issue) = producer_issue {
        return settled(&issue, subject);
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
    let key_of = |state: &crate::exact_probabilistic::ProbKey| {
        if let Some(monitor) = monitor {
            bounded_product_key(state, binding, monitor, limits.document, cancel)
        } else {
            ProbabilityProductKey::for_model_state_with_limits(
                binding,
                &state.model,
                limits.document,
                cancel,
            )
        }
    };
    let Some(initial_state) = product.states.get(initial.index()) else {
        return broken();
    };
    let initial_key = match key_of(initial_state) {
        Ok(key) => key,
        Err(error) => return document_failure(error, subject),
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
        let key = match key_of(state) {
            Ok(key) => key,
            Err(error) => return document_failure(error, subject),
        };
        scheduler.push((key, action));
    }
    scheduler.sort_by(|a, b| a.0.cmp(&b.0));
    let mut paths = Vec::new();
    let mut conversion_work = 0usize;
    for path in produced.paths {
        let mut partials = vec![(first.clone(), Vec::new())];
        for step in path.steps {
            if cancel.poll() {
                if let Some(cause) = cancel.tripped() {
                    return settled(&ExactProbBuildIssue::Cancelled(cause), subject);
                }
            }
            if monitor.is_some() && matches!(step.action, ProductAction::TerminalStutter) {
                // Qualified replay derives false extension at an actual leaf;
                // closure consumes no model step or draw.
                continue;
            }
            let ProductAction::Identity(identity) = step.action else {
                return not_yet();
            };
            let Some(to) = product.states.get(step.to.index()) else {
                return broken();
            };
            let post_key = match key_of(to) {
                Ok(key) => key,
                Err(error) => return document_failure(error, subject),
            };
            let mut next = Vec::new();
            for (state, steps) in partials {
                conversion_work = conversion_work.saturating_add(1);
                if conversion_work > limits.model.limits.max_transitions {
                    return settled(
                        &ExactProbBuildIssue::Limit {
                            limit: ModelCheckLimit::ExactProbMaxTransitions,
                            value: u64::try_from(limits.model.limits.max_transitions)
                                .unwrap_or(u64::MAX),
                        },
                        subject,
                    );
                }
                if let Err(issue) = witness_charge(
                    &mut producer_meter,
                    &mut producer_storage,
                    std::mem::size_of::<ProbabilityWitnessStep>(),
                    limits,
                    cancel,
                ) {
                    return settled(&issue, subject);
                }
                let result = system.actions_with_cancel(&state, cancel);
                if let Some(cause) = cancel.tripped() {
                    return settled(&ExactProbBuildIssue::Cancelled(cause), subject);
                }
                let actions = match result {
                    Ok(Ok(actions)) => actions,
                    Err(stop) => return settled(&ExactProbBuildIssue::Expansion(stop), subject),
                    Ok(Err(_)) => return broken(),
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
                            if system.state_key(&post).ok().map(|key| key.0).as_ref()
                                == Some(&to.model)
                            {
                                if let Err(issue) = witness_charge(
                                    &mut producer_meter,
                                    &mut producer_storage,
                                    post_key.as_bytes().len().saturating_mul(2),
                                    limits,
                                    cancel,
                                ) {
                                    return settled(&issue, subject);
                                }
                                if candidates.try_reserve(1).is_err() {
                                    return settled(
                                        &ExactProbBuildIssue::Limit {
                                            limit: ModelCheckLimit::CertificateMaxWorkingBytes,
                                            value: limits.document.storage_bytes,
                                        },
                                        subject,
                                    );
                                }
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
                    if let Err(issue) = witness_charge(
                        &mut producer_meter,
                        &mut producer_storage,
                        steps
                            .len()
                            .saturating_add(1)
                            .saturating_mul(std::mem::size_of::<ProbabilityWitnessStep>())
                            .saturating_add(post_key.as_bytes().len()),
                        limits,
                        cancel,
                    ) {
                        return settled(&issue, subject);
                    }
                    if next.try_reserve(1).is_err() {
                        return settled(
                            &ExactProbBuildIssue::Limit {
                                limit: ModelCheckLimit::CertificateMaxWorkingBytes,
                                value: limits.document.storage_bytes,
                            },
                            subject,
                        );
                    }
                    let mut extended = Vec::new();
                    if extended.try_reserve(steps.len().saturating_add(1)).is_err() {
                        return settled(
                            &ExactProbBuildIssue::Limit {
                                limit: ModelCheckLimit::CertificateMaxWorkingBytes,
                                value: limits.document.storage_bytes,
                            },
                            subject,
                        );
                    }
                    extended.extend(steps.iter().cloned());
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
    if monitor.is_some() {
        // A finite-horizon policy only needs entries for represented choices.
        // The unbounded producer retains its continuation policy contract.
        let mut used = std::collections::BTreeSet::new();
        for path in &paths {
            let mut key = &initial_key;
            for step in &path.steps {
                used.insert(key.clone());
                key = &step.post_key;
            }
        }
        scheduler.retain(|(key, _)| used.contains(key));
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
        let key = ProbabilityProductKey::for_model_state(
            None,
            include_bytes!(
                "../../../tools/qsl-probability-check/tests/fixtures/link.model-key-0.json"
            ),
        )
        .expect("the key encodes");
        let (terminal, evidence, counterexample) = ExactReport::Rejected {
            rule: CertificateRule::ThresholdMissed,
            at: CertificateLocus::ProbabilityProductState(key.clone()),
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
