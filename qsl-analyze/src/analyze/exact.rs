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
//! and is never relabelled as a verdict. A claim whose bound the exact value
//! misses is not `refuted` here either: FR-203 settles `refuted` only after
//! FR-202's replay of a counterexample, which is not implemented.

use qsl_eval::simulation::model::ModelSystem;
use qsl_foundation::{BackendId, RequestIndex};
use qsl_replay::statistical::SubjectLimits;
use qsl_replay::{
    check_claim_exact_reachability, ExactProbBuildIssue, ExactProbBuildRecord,
    ExactProbabilityCertificate, ExactProbabilityCertificateCheck, ExactProbabilityGraphError,
    ExactProbabilityValue, Extremum, ModelCheckLimit, ObligationIdentity, ProbabilityBoundSide,
    ProbabilityProductKey, TerminalRecord,
};
use qsl_route::Mode;
use qsl_semantics::check::{
    Bound, Capability, DeclarationFamily, ProbForm, ProbScheduler, ReachTarget,
};
use quire_exact::{Cancel, Rational};

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
    /// The checker accepted the solver's certificate.
    Proved {
        /// The checker's acceptance, the only evidence a `proved` record
        /// is built from.
        check: ExactProbabilityCertificateCheck,
        /// The scheduler extremum the value is, absent for a workload.
        extremum: Option<Extremum>,
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
    ) -> (TerminalRecord, Option<Box<ExactProbabilityEvidence>>) {
        match self {
            Self::Proved {
                check,
                extremum,
                evidence,
            } => match TerminalRecord::proved_by_exact_certificate(
                index,
                backend.clone(),
                &check,
                extremum,
            ) {
                Some(record) => (record, Some(evidence)),
                None => (TerminalRecord::failed_with(index, backend, BROKEN), None),
            },
            Self::Settled(record) => (record.into_terminal(index, backend), None),
        }
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
        Mode::Bounded,
        en5::BACKEND_ID,
        move |cancel| run(request, cancel),
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

fn run(request: ProductRequest<'_>, cancel: &Cancel) -> EngineReport {
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
    if !holds {
        // The bound is missed; `refuted` waits for FR-202's replay.
        return not_yet();
    }

    let Ok(certificate) = certificate_of(&product, &solved.values) else {
        return broken();
    };
    let Some(node) = claim.occurrence().map(|key| key.node().clone()) else {
        return not_yet();
    };
    let Ok(system) = ModelSystem::new(simulation) else {
        return broken();
    };
    let checked = check_claim_exact_reachability(
        &system,
        package.graph().probabilistic_model().claims(),
        &ObligationIdentity::new(node, None, 0),
        &certificate,
        limits.model.limits.max_states,
        limits.model.limits.max_transitions,
    );
    match checked {
        Ok(check) => match &check {
            ExactProbabilityCertificateCheck::Accepted { value, side } => {
                let evidence = Box::new(ExactProbabilityEvidence {
                    certificate,
                    value: value.clone(),
                    side: *side,
                });
                EngineReport::Exact(Box::new(ExactReport::Proved {
                    check,
                    extremum,
                    evidence,
                }))
            }
            // The solver's own certificate was rejected: an engine fault.
            // The published `CertificateRejected` cause carries QSpec
            // FR-331's rule vocabulary, which QSL does not map yet.
            ExactProbabilityCertificateCheck::Rejected(_) => broken(),
        },
        Err(ExactProbabilityGraphError::LimitReached) => settled(
            &ExactProbBuildIssue::Limit {
                limit: ModelCheckLimit::ExactProbMaxStates,
                value: u64::try_from(limits.model.limits.max_states).unwrap_or(u64::MAX),
            },
            subject,
        ),
        // Cyclic graphs, draws with several post-states and the other
        // shapes the checker core does not cover.
        Err(_) => not_yet(),
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
    values: &[ExtRational],
) -> Result<ExactProbabilityCertificate, ExactProbabilityGraphError> {
    let mut entries = Vec::with_capacity(product.states.len());
    for (state, value) in product.states.iter().zip(values) {
        let ExtRational::Finite(value) = value else {
            return Err(ExactProbabilityGraphError::ExpansionRefused);
        };
        entries.push(ExactProbabilityValue {
            state: ProbabilityProductKey::for_model_state(state.binding.as_ref(), &state.model)?,
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
