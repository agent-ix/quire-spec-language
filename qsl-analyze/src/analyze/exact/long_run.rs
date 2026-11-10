// SPDX-License-Identifier: AGPL-3.0-or-later
//! Candidate production for the independently checked DTMC long-run profile.

use super::candidate::{rational_bytes, sort, values, CandidateBudget};
use super::*;
use crate::exact_probabilistic::{
    build_retaining_monitor_with_budget, decide_long_run_workload, BudgetedProductError,
    LongRunResult, ProductKind,
};
use qsl_replay::{
    long_run_probability_certificate_document, write_long_run_probability_certificate,
    LongRunProbabilityComponent, ProbabilityDocumentIoLimits,
    weighted_long_run_probability_certificate_document, write_weighted_long_run_probability_certificate,
};

pub(super) fn run(
    request: ProductRequest<'_>,
    cancel: &Cancel,
    tamper: Option<&dyn Fn(&mut ExactProbabilityCertificate)>,
) -> EngineReport {
    let claim = request.claim;
    let ProbForm::LongRunFraction {
        bound,
        weight,
        ..
    } = &claim.form
    else {
        return not_yet();
    };
    if !matches!(claim.scheduler, ProbScheduler::Workload(_))
        || !claim.fairness.is_empty()
        || qsl_semantics::check::TemporalProfile::from_identity(claim.profile.identity())
            != Some(qsl_semantics::check::TemporalProfile::InfiniteTrace)
    {
        return not_yet();
    }
    let simulation = request.subject.simulation.clone();
    let package = simulation.package;
    let limits = request.limits;
    let subject = SubjectLimits {
        expansion: simulation.expansion,
        meter: &simulation.meter,
    };
    let system = match ModelSystem::new_with_cancel(simulation.clone(), cancel) {
        Ok(system) => system,
        Err(_) => {
            return match cancel.tripped() {
                Some(cause) => settled(&ExactProbBuildIssue::Cancelled(cause), subject),
                None => broken(),
            };
        }
    };
    let item = match qsl_replay::probabilistic_claim_identity(claim) {
        Ok(item) => item,
        Err(_) => return not_yet(),
    };
    let binding = match claim_binding(&system, claim) {
        Ok(binding) => binding,
        Err(_) => return not_yet(),
    };
    let io = ProbabilityDocumentIoLimits {
        bytes: limits.document.bytes,
        storage_bytes: limits.document.storage_bytes,
    };
    let document_limits = ProbabilityDocumentLimits {
        bytes: io.bytes,
        storage_bytes: io.storage_bytes,
        states: limits.model.limits.max_states,
        edges: limits.model.limits.max_transitions,
        scalar_bits: limits.max_rational_bits,
        accounting: simulation.meter,
    };
    let claims = package.graph().probabilistic_model().claims();
    let mut candidate = CandidateBudget::new(document_limits, cancel);
    let product = {
        let identity = candidate.identity;
        let mut admission = candidate.admission();
        match build_retaining_monitor_with_budget(request, identity, &mut admission, cancel) {
            Ok((ProbBuild::Built(product), _)) => product,
            Ok((ProbBuild::Undefined(_), _)) => return not_yet(),
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
    };
    if product.kind != ProductKind::Dtmc
        || product.initial.len() != 1
        || product.states.iter().any(|state| {
            state.monitor.is_some() || state.accumulator.is_some() || state.pending.is_some()
        })
    {
        return not_yet();
    }
    let predicate = product
        .labels
        .iter()
        .map(|labels| labels.first().copied().unwrap_or(false))
        .collect::<Vec<_>>();
    if let Some(cause) = cancel.tripped() {
        return settled(&ExactProbBuildIssue::Cancelled(cause), subject);
    }
    let solution = match decide_long_run_workload(&product, &predicate, weight.as_ref().map(|name| name.as_str()), &limits) {
        Ok(solution) => solution,
        Err(UnboundedError::MaxRationalBits) => {
            return settled(
                &ExactProbBuildIssue::Limit {
                    limit: ModelCheckLimit::ExactProbMaxRationalBits,
                    value: limits.max_rational_bits,
                },
                subject,
            )
        }
        Err(UnboundedError::MaxPolicyIterations) => {
            return settled(
                &ExactProbBuildIssue::Limit {
                    limit: ModelCheckLimit::ExactProbMaxPolicyIterations,
                    value: limits.max_policy_iterations,
                },
                subject,
            )
        }
        Err(UnboundedError::ZeroWeightComponent) => return EngineReport::Refused(Refusal::Unsupported("ZeroWeightComponent")),
        Err(UnboundedError::MissingReward) => return not_yet(),
        Err(_) => return broken(),
    };
    if let Some(cause) = cancel.tripped() {
        return settled(&ExactProbBuildIssue::Cancelled(cause), subject);
    }
    let [(_, _, initial)] = product.initial.as_slice() else {
        return not_yet();
    };
    let Some(ExtRational::Finite(value)) = solution.values.get(initial.index()) else {
        return broken();
    };
    if !match bound {
        Bound::AtLeast(threshold) => value >= threshold,
        Bound::AtMost(threshold) => value <= threshold,
    } {
        // This profile authorizes proofs only, never an unverified refutation.
        return not_yet();
    }
    let reference = binding.as_ref().map(|b| &b.reference);
    let mut certificate = match values(&product, reference, &solution.values, &mut candidate) {
        Ok(certificate) => certificate,
        Err(error) => return document_failure(error, subject),
    };
    let components = match components(&product, reference, &solution, &mut candidate) {
        Ok(components) => components,
        Err(error) => return document_failure(error, subject),
    };
    if let Some(tamper) = tamper {
        tamper(&mut certificate);
    }
    for row in &certificate.values {
        if let Err(error) = candidate.retain_row(row) {
            return document_failure(error, subject);
        }
    }
    let bytes = if weight.is_some() {
        weighted_long_run_probability_certificate_document(
            &system, claims, &item, certificate.values.clone(), components, document_limits, cancel,
        ).and_then(|document| write_weighted_long_run_probability_certificate(&document, document_limits, cancel))
    } else {
        long_run_probability_certificate_document(
            &system, claims, &item, certificate.values.clone(), components, document_limits, cancel,
        ).and_then(|document| write_long_run_probability_certificate(&document, document_limits, cancel))
    };
    let checked = bytes.and_then(|bytes| check_probability_certificate_document(
        &system, claims, &item, &bytes, document_limits, cancel,
    ));
    checked_report(checked, certificate, binding, None, subject)
}

fn components(
    product: &ProbProduct,
    binding: Option<&ObjectReference>,
    solution: &LongRunResult,
    budget: &mut CandidateBudget<'_>,
) -> Result<Vec<LongRunProbabilityComponent>, ProbabilityDocumentError> {
    if solution.components.len() != solution.gain_bias.len() {
        return Err(ProbabilityDocumentError::Malformed);
    }
    budget.reserve(
        solution
            .components
            .len()
            .saturating_mul(std::mem::size_of::<LongRunProbabilityComponent>()),
    )?;
    let mut components = Vec::with_capacity(solution.components.len());
    for (component, evidence) in solution.components.iter().zip(&solution.gain_bias) {
        if let Some(cause) = budget.cancel.tripped() {
            return Err(ProbabilityDocumentError::Cancelled(cause));
        }
        if component.states.len() != evidence.bias.len() {
            return Err(ProbabilityDocumentError::Malformed);
        }
        budget.reserve(
            evidence
                .bias
                .len()
                .saturating_mul(std::mem::size_of::<ExactProbabilityValue>()),
        )?;
        let mut bias = Vec::with_capacity(evidence.bias.len());
        for (state, value) in &evidence.bias {
            let state = product
                .states
                .get(state.index())
                .ok_or(ProbabilityDocumentError::Malformed)?;
            budget.reserve(rational_bytes(value))?;
            bias.push(ExactProbabilityValue {
                state: budget.key(binding, &state.model)?,
                value: value.clone(),
            });
        }
        sort(
            &mut bias,
            |row| &row.state,
            &mut budget.meter,
            budget.cancel,
        )?;
        let offset = bias
            .first()
            .ok_or(ProbabilityDocumentError::Malformed)?
            .value
            .clone();
        for row in &mut bias {
            row.value = match quire_exact::evaluate_rational_arithmetic(
                quire_exact::RationalArithmetic::Subtract(&row.value, &offset),
                None,
                &mut budget.meter,
            ) {
                quire_exact::Outcome::Completed(value) => value,
                quire_exact::Outcome::Incomplete(incomplete) => {
                    return Err(match budget.cancel.tripped() {
                        Some(cause) => ProbabilityDocumentError::Cancelled(cause),
                        None => ProbabilityDocumentError::Accounting(Box::new(incomplete)),
                    })
                }
                _ => return Err(ProbabilityDocumentError::Malformed),
            };
        }
        components.push(LongRunProbabilityComponent {
            value: component.rho.clone(),
            gain: evidence.gain.clone(),
            bias,
        });
    }
    sort(
        &mut components,
        |component| &component.bias[0].state,
        &mut budget.meter,
        budget.cancel,
    )?;
    Ok(components)
}
