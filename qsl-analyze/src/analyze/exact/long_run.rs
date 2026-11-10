// SPDX-License-Identifier: AGPL-3.0-or-later
//! Candidate production for the independently checked DTMC long-run profile.

use super::*;
use crate::exact_probabilistic::{decide_long_run_workload, LongRunResult, ProductKind};
use qsl_replay::{
    long_run_probability_certificate_document, write_long_run_probability_certificate,
    LongRunProbabilityComponent, ProbabilityDocumentIoLimits,
};

pub(super) fn run(
    request: ProductRequest<'_>,
    cancel: &Cancel,
    tamper: Option<&dyn Fn(&mut ExactProbabilityCertificate)>,
) -> EngineReport {
    let claim = request.claim;
    let ProbForm::LongRunFraction {
        bound,
        weight: None,
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
    // Independently admit the full graph, predicate and workload before solving.
    let claims = package.graph().probabilistic_model().claims();
    let mut document = match long_run_probability_certificate_document(
        &system,
        claims,
        &item,
        Vec::new(),
        Vec::new(),
        document_limits,
        cancel,
    ) {
        Ok(document) => document,
        Err(error) => return document_failure(error, subject),
    };
    let product = match build_retaining_monitor(request, cancel) {
        Ok((ProbBuild::Built(product), _)) => product,
        Ok((ProbBuild::Undefined(_), _)) => return not_yet(),
        Err(refusal) => return build_refusal(refusal, subject),
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
    let solution = match decide_long_run_workload(&product, &predicate, None, &limits) {
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
    let mut retained = 0u64;
    for (state, value) in product.states.iter().zip(&solution.values) {
        let ExtRational::Finite(value) = value else {
            return broken();
        };
        let bytes = state
            .model
            .len()
            .saturating_mul(8)
            .saturating_add(std::mem::size_of::<ExactProbabilityValue>())
            .saturating_add(
                usize::try_from(
                    value
                        .numerator()
                        .magnitude_bits()
                        .saturating_add(value.denominator().magnitude_bits())
                        .div_ceil(8),
                )
                .unwrap_or(usize::MAX),
            );
        if let Err(error) = reserve(&mut retained, bytes, io, cancel) {
            return document_failure(error, subject);
        }
    }
    let mut certificate = match certificate_of(&product, reference, &solution.values, io, cancel) {
        Ok(certificate) => certificate,
        Err(error) => return document_failure(error, subject),
    };
    document.components = match components(
        &product,
        reference,
        &solution,
        io,
        document_limits,
        &mut retained,
        cancel,
    ) {
        Ok(components) => components,
        Err(error) => return document_failure(error, subject),
    };
    if let Some(tamper) = tamper {
        tamper(&mut certificate);
    }
    document.values = certificate.values.clone();
    let checked = write_long_run_probability_certificate(&document, document_limits, cancel)
        .and_then(|bytes| {
            check_probability_certificate_document(
                &system,
                claims,
                &item,
                &bytes,
                document_limits,
                cancel,
            )
        });
    checked_report(checked, certificate, binding, None, subject)
}

fn components(
    product: &ProbProduct,
    binding: Option<&ObjectReference>,
    solution: &LongRunResult,
    io: ProbabilityDocumentIoLimits,
    limits: ProbabilityDocumentLimits,
    retained: &mut u64,
    cancel: &Cancel,
) -> Result<Vec<LongRunProbabilityComponent>, ProbabilityDocumentError> {
    if solution.components.len() != solution.gain_bias.len() {
        return Err(ProbabilityDocumentError::Malformed);
    }
    let mut meter = quire_exact::Meter::new(limits.accounting).with_cancel(cancel.clone());
    reserve(
        retained,
        solution
            .components
            .len()
            .saturating_mul(std::mem::size_of::<LongRunProbabilityComponent>()),
        io,
        cancel,
    )?;
    let mut components = Vec::with_capacity(solution.components.len());
    for (component, evidence) in solution.components.iter().zip(&solution.gain_bias) {
        if let Some(cause) = cancel.tripped() {
            return Err(ProbabilityDocumentError::Cancelled(cause));
        }
        if component.states.len() != evidence.bias.len() {
            return Err(ProbabilityDocumentError::Malformed);
        }
        reserve(
            retained,
            evidence
                .bias
                .len()
                .saturating_mul(std::mem::size_of::<ExactProbabilityValue>()),
            io,
            cancel,
        )?;
        let mut bias = Vec::with_capacity(evidence.bias.len());
        for (state, value) in &evidence.bias {
            let state = product
                .states
                .get(state.index())
                .ok_or(ProbabilityDocumentError::Malformed)?;
            reserve(
                retained,
                state.model.len().saturating_mul(8).saturating_add(
                    usize::try_from(
                        value
                            .numerator()
                            .magnitude_bits()
                            .saturating_add(value.denominator().magnitude_bits())
                            .div_ceil(8),
                    )
                    .unwrap_or(usize::MAX),
                ),
                io,
                cancel,
            )?;
            bias.push(ExactProbabilityValue {
                state: ProbabilityProductKey::for_model_state_with_limits(
                    binding,
                    &state.model,
                    io,
                    cancel,
                )?,
                value: value.clone(),
            });
        }
        sort(&mut bias, |row| &row.state, &mut meter, cancel)?;
        let offset = bias
            .first()
            .ok_or(ProbabilityDocumentError::Malformed)?
            .value
            .clone();
        for row in &mut bias {
            row.value = match quire_exact::evaluate_rational_arithmetic(
                quire_exact::RationalArithmetic::Subtract(&row.value, &offset),
                None,
                &mut meter,
            ) {
                quire_exact::Outcome::Completed(value) => value,
                quire_exact::Outcome::Incomplete(incomplete) => {
                    return Err(match cancel.tripped() {
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
        &mut meter,
        cancel,
    )?;
    Ok(components)
}

fn reserve(
    retained: &mut u64,
    bytes: usize,
    limits: ProbabilityDocumentIoLimits,
    cancel: &Cancel,
) -> Result<(), ProbabilityDocumentError> {
    if let Some(cause) = cancel.tripped() {
        return Err(ProbabilityDocumentError::Cancelled(cause));
    }
    let actual = retained.saturating_add(bytes as u64);
    if actual > limits.storage_bytes {
        return Err(ProbabilityDocumentError::Limit {
            limit: qsl_replay::ProbabilityDocumentLimit::StorageBytes,
            bound: limits.storage_bytes,
            actual,
        });
    }
    *retained = actual;
    Ok(())
}

// In-place insertion gives a fallible, allocation-free comparison boundary.
fn sort<T>(
    rows: &mut [T],
    key: impl Fn(&T) -> &ProbabilityProductKey,
    meter: &mut quire_exact::Meter,
    cancel: &Cancel,
) -> Result<(), ProbabilityDocumentError> {
    for index in 1..rows.len() {
        let mut position = index;
        while position > 0 {
            if let Some(cause) = cancel.tripped() {
                return Err(ProbabilityDocumentError::Cancelled(cause));
            }
            let bytes = key(&rows[position])
                .as_bytes()
                .len()
                .min(key(&rows[position - 1]).as_bytes().len())
                .saturating_add(1);
            meter
                .charge(
                    quire_exact::Charge::new(quire_exact::ChargePoint::GraphEdge)
                        .work(quire_exact::Integer::from(bytes as u64)),
                )
                .map_err(|error| match cancel.tripped() {
                    Some(cause) => ProbabilityDocumentError::Cancelled(cause),
                    None => ProbabilityDocumentError::Accounting(Box::new(error)),
                })?;
            if key(&rows[position - 1]) <= key(&rows[position]) {
                break;
            }
            rows.swap(position - 1, position);
            position -= 1;
        }
    }
    Ok(())
}
