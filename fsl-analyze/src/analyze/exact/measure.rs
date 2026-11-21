// SPDX-License-Identifier: AGPL-3.0-or-later
//! Shared-control finite-horizon induction and independently accepted proofs.
use super::*;
use crate::exact_probabilistic::{decide_measure_exact, FhObjective, QualifiedFhError};
use fsl_replay::{
    conditional_measure_probability_certificate, measure_probability_certificate_document,
    measure_probability_objective, measure_probability_product_key,
    write_measure_probability_certificate, MeasurePayoffKind,
};

pub(super) fn run(
    request: ProductRequest<'_>,
    cancel: &Cancel,
    tamper: Option<&dyn Fn(&mut ExactProbabilityCertificate)>,
    tamper_witness: Option<&dyn Fn(&mut ProbabilisticCounterexample)>,
) -> EngineReport {
    let claim = request.claim;
    let simulation = request.subject.simulation.clone();
    let limits = request.limits;
    let scalar = simulation.meter;
    let subject = SubjectLimits {
        expansion: simulation.expansion,
        meter: &scalar,
    };
    let admitted = ModelSystem::new_with_cancel(simulation.clone(), cancel);
    if let Some(cause) = cancel.tripped() {
        return settled(&ExactProbBuildIssue::Cancelled(cause), subject);
    }
    let system = match admitted {
        Ok(system) => system,
        Err(error) => return build_refusal(ProductRefusal::Admission(error), subject),
    };
    let Some(package) = system.source_package() else {
        return not_yet();
    };
    let claims = package.graph().probabilistic_model().claims();
    let item = match fsl_replay::probabilistic_claim_identity(claim) {
        Ok(item) => item,
        Err(_) => return not_yet(),
    };
    let binding = match claim_binding(&system, claim) {
        Ok(binding) => binding,
        Err(error) => return document_failure(error.into(), subject),
    };
    let reference = binding.as_ref().map(|binding| &binding.reference);
    let document_limits = ProbabilityDocumentLimits::for_exact(&limits, scalar);
    let mut meter = filament_exact::Meter::new(scalar).with_cancel(cancel.clone());
    let mut storage = 0;
    let kind = match &claim.form {
        ProbForm::Probability {
            event: Event::Compare { .. },
            ..
        } => MeasurePayoffKind::Event,
        ProbForm::MeanOfFraction { .. } => MeasurePayoffKind::Fraction,
        ProbForm::Quantile { .. } if matches!(claim.scheduler, ProbScheduler::Workload(_)) => {
            MeasurePayoffKind::Satisfied
        }
        ProbForm::Quantile { .. } => MeasurePayoffKind::Shifted,
        _ => return not_yet(),
    };
    let (payoff, reward, optimization) = match measure_probability_objective(
        &system,
        claim,
        kind,
        document_limits,
        cancel,
        &mut meter,
        &mut storage,
    ) {
        Ok(selected) => selected,
        Err(error) => return document_failure(error, subject),
    };
    let (mut product, _) = match build_retaining_monitor(request, cancel) {
        Ok((ProbBuild::Built(product), monitor)) => (product, monitor),
        Ok((ProbBuild::Undefined(path), monitor)) => {
            return undefined::run(
                simulation,
                claim,
                path,
                monitor,
                &limits,
                cancel,
                subject,
                tamper_witness,
            )
        }
        Err(error) => return build_refusal(error, subject),
    };
    let Some((_, _, initial)) = product.initial.first() else {
        return broken();
    };
    let initial = *initial;
    if matches!(claim.form, ProbForm::Quantile { .. })
        && !product.states.iter().any(|state| {
            matches!(
                state.accumulator.as_ref(),
                Some(
                    crate::exact_probabilistic::Accumulator::Steps {
                        activated: true,
                        ..
                    } | crate::exact_probabilistic::Accumulator::Reward {
                        activated: true,
                        ..
                    }
                )
            )
        })
    {
        let empty = ExactProbabilityCertificate { values: Vec::new() };
        let checked = (|| {
            let document = fsl_replay::vacuous_measure_probability_certificate_document(
                &system,
                claims,
                &item,
                document_limits,
                cancel,
            )?;
            let bytes = write_measure_probability_certificate(&document, document_limits, cancel)?;
            check_probability_certificate_document(
                &system,
                claims,
                &item,
                &bytes,
                document_limits,
                cancel,
            )
        })();
        return checked_report(checked, empty, binding, None, subject);
    }
    let terminal = match product
        .states
        .iter()
        .map(|state| {
            let control = state.monitor.ok_or(ProbabilityDocumentError::Malformed)?;
            let sums = state
                .accumulator
                .as_ref()
                .ok_or(ProbabilityDocumentError::Malformed)?;
            payoff
                .terminal(control, sums, &mut meter)
                .map_err(ProbabilityDocumentError::from)
        })
        .collect::<Result<Vec<_>, _>>()
    {
        Ok(terminal) => terminal,
        Err(error) => return document_failure(error, subject),
    };
    let objective = match optimization {
        ProbabilityObjective::Maximum => FhObjective::Max,
        _ => FhObjective::Min,
    };
    let (values, policy) = match decide_measure_exact(
        &product,
        &terminal,
        objective,
        limits.max_rational_bits,
        &mut meter,
        &mut storage,
        document_limits.storage_bytes,
    ) {
        Ok(FhResult::Exact { values, policy }) => (values, policy),
        Ok(_) => return broken(),
        Err(error) => return document_failure(induction_failure(error, cancel), subject),
    };
    for state in &mut product.states {
        let Some(control) = state.monitor else {
            return broken();
        };
        let Some(sums) = state.accumulator.as_ref() else {
            return broken();
        };
        let pending = state
            .pending
            .as_ref()
            .map(|pending| {
                let mut admission = fsl_replay::ProbabilityKeyAdmission::new(
                    &mut meter,
                    &mut storage,
                    document_limits.storage_bytes,
                    cancel,
                );
                system
                    .pending_key_bytes_with_budget(
                        &pending.identity,
                        &pending.random,
                        fsl_foundation::IdentityLimits::default(),
                        &mut admission,
                        cancel,
                    )
                    .map_err(|error| admission.refusal(error))
            })
            .transpose();
        let pending = match pending {
            Ok(pending) => pending,
            Err(error) => return document_failure(error, subject),
        };
        let key = match measure_probability_product_key(
            reference,
            &state.model,
            control,
            sums,
            pending.as_deref(),
            reward.as_ref(),
            document_limits,
            cancel,
            &mut meter,
            &mut storage,
        ) {
            Ok(key) => key,
            Err(error) => return document_failure(error, subject),
        };
        state.certificate_state = Some(key);
    }
    let rows =
        |values: &[Rational]| -> Result<ExactProbabilityCertificate, ProbabilityDocumentError> {
            let mut rows = product
                .states
                .iter()
                .zip(values)
                .map(|(state, value)| {
                    Ok(ExactProbabilityValue {
                        state: state
                            .certificate_state
                            .clone()
                            .ok_or(ProbabilityDocumentError::Malformed)?,
                        value: value.clone(),
                    })
                })
                .collect::<Result<Vec<_>, ProbabilityDocumentError>>()?;
            rows.sort_by(|a, b| a.state.cmp(&b.state));
            Ok(ExactProbabilityCertificate { values: rows })
        };
    let mut certificate = match rows(&values) {
        Ok(certificate) => certificate,
        Err(error) => return document_failure(error, subject),
    };
    if let Some(tamper) = tamper {
        tamper(&mut certificate);
    }
    let checked = (|| {
        let numerator = measure_probability_certificate_document(
            &system,
            claims,
            &item,
            &certificate,
            kind,
            document_limits,
            cancel,
        )?;
        let document = if kind == MeasurePayoffKind::Satisfied {
            let (activated, _, _) = measure_probability_objective(
                &system,
                claim,
                MeasurePayoffKind::Activated,
                document_limits,
                cancel,
                &mut meter,
                &mut storage,
            )?;
            let terminal = product
                .states
                .iter()
                .map(|state| {
                    activated
                        .terminal(
                            state.monitor.ok_or(ProbabilityDocumentError::Malformed)?,
                            state
                                .accumulator
                                .as_ref()
                                .ok_or(ProbabilityDocumentError::Malformed)?,
                            &mut meter,
                        )
                        .map_err(ProbabilityDocumentError::from)
                })
                .collect::<Result<Vec<_>, _>>()?;
            let denominator = match decide_measure_exact(
                &product,
                &terminal,
                objective,
                limits.max_rational_bits,
                &mut meter,
                &mut storage,
                document_limits.storage_bytes,
            )
            .map_err(|error| induction_failure(error, cancel))?
            {
                FhResult::Exact { values, .. } => values,
                _ => return Err(ProbabilityDocumentError::Malformed),
            };
            let denominator = measure_probability_certificate_document(
                &system,
                claims,
                &item,
                &rows(&denominator)?,
                MeasurePayoffKind::Activated,
                document_limits,
                cancel,
            )?;
            conditional_measure_probability_certificate(numerator, denominator)?
        } else {
            numerator
        };
        let bytes = write_measure_probability_certificate(&document, document_limits, cancel)?;
        check_probability_certificate_document(
            &system,
            claims,
            &item,
            &bytes,
            document_limits,
            cancel,
        )
    })();
    let failed_bound = matches!(&checked, Ok(ExactProbabilityCertificateCheck::Rejected(rejection)) if rejection.rule == ProbabilityCertificateRule::ThresholdMissed || matches!(&claim.form,ProbForm::Quantile { threshold:Bound::AtLeast(_),.. }) && values.get(initial.index()).is_some_and(Rational::is_zero));
    if failed_bound && tamper.is_none() {
        if matches!(claim.form, ProbForm::MeanOfFraction { .. }) {
            return refute_mean(
                &system,
                claim,
                &product,
                &policy,
                initial,
                reference,
                &certificate,
                &limits,
                cancel,
                subject,
                tamper_witness,
            );
        }
        return refute(
            &system,
            claim,
            &product,
            &policy,
            initial,
            reference,
            &limits,
            cancel,
            subject,
            tamper_witness,
            meter,
            storage,
        );
    }
    let extremum = match optimization {
        ProbabilityObjective::Minimum => Some(Extremum::Min),
        ProbabilityObjective::Maximum => Some(Extremum::Max),
        ProbabilityObjective::Workload => None,
    };
    checked_report(checked, certificate, binding, extremum, subject)
}
#[allow(clippy::too_many_arguments)]
fn refute(
    system: &ModelSystem<'_>,
    claim: &fsl_semantics::check::CheckedProbabilisticClaim,
    product: &ProbProduct,
    policy: &[Option<usize>],
    initial: crate::exact_probabilistic::ProbStateId,
    binding: Option<&ObjectReference>,
    limits: &crate::exact_probabilistic::ExactProbLimits,
    cancel: &Cancel,
    subject: SubjectLimits<'_>,
    tamper: Option<&dyn Fn(&mut ProbabilisticCounterexample)>,
    mut meter: filament_exact::Meter,
    mut storage: u64,
) -> EngineReport {
    use crate::exact_probabilistic::{
        produce_quantile_witness, produce_witness, WitnessError, WitnessScope, WitnessWork,
    };
    let document_limits = ProbabilityDocumentLimits::for_exact(limits, system.meter_limits());
    let kind = if matches!(claim.form, ProbForm::Quantile { .. }) {
        MeasurePayoffKind::Satisfied
    } else {
        MeasurePayoffKind::Event
    };
    let (payoff, _, _) = match measure_probability_objective(
        system,
        claim,
        kind,
        document_limits,
        cancel,
        &mut meter,
        &mut storage,
    ) {
        Ok(selected) => selected,
        Err(error) => return document_failure(error, subject),
    };
    let terminal = match product
        .states
        .iter()
        .map(|state| {
            payoff
                .terminal(
                    state.monitor.ok_or(ProbabilityDocumentError::Malformed)?,
                    state
                        .accumulator
                        .as_ref()
                        .ok_or(ProbabilityDocumentError::Malformed)?,
                    &mut meter,
                )
                .map_err(ProbabilityDocumentError::from)
        })
        .collect::<Result<Vec<_>, _>>()
    {
        Ok(terminal) => terminal,
        Err(error) => return document_failure(error, subject),
    };
    let scope = match claim.scheduler {
        ProbScheduler::Every => WitnessScope::EveryScheduler,
        ProbScheduler::Workload(_) => WitnessScope::Workload,
    };
    let mut issue = None;
    let mut control = |work: WitnessWork<'_>| {
        witness_payload_bytes(work, system)
            .and_then(|bytes| witness_charge(&mut meter, &mut storage, bytes, limits, cancel))
            .map_err(|failure| {
                issue = Some(failure);
                WitnessError::InvalidState
            })
    };
    let produced = match &claim.form {
        ProbForm::Quantile { q, threshold, .. } => {
            let strict = matches!(threshold, Bound::AtLeast(_));
            let never = product
                .states
                .iter()
                .map(|state| {
                    matches!(
                        state.accumulator,
                        Some(
                            fsl_eval::probabilistic::Accumulator::Steps {
                                activated: false,
                                ..
                            } | fsl_eval::probabilistic::Accumulator::Reward {
                                activated: false,
                                ..
                            }
                        )
                    ) && state
                        .monitor
                        .is_some_and(fsl_eval::probabilistic::MonitorState::is_decided)
                })
                .collect::<Vec<_>>();
            let accepting = terminal
                .iter()
                .enumerate()
                .map(|(i, value)| {
                    never[i]
                        || value.as_ref().is_some_and(|value| {
                            if strict {
                                value == &Rational::from_integer(1_i64.into())
                            } else {
                                value.is_zero() && !never[i]
                            }
                        })
                })
                .collect::<Vec<_>>();
            produce_quantile_witness(
                product,
                policy,
                scope,
                initial,
                &accepting,
                &never,
                q,
                strict,
                limits,
                &mut control,
            )
        }
        ProbForm::Probability { bound, .. } => {
            let accepting = terminal
                .iter()
                .map(|value| {
                    value.as_ref().is_some_and(|value| match bound {
                        Bound::AtLeast(_) => value.is_zero(),
                        Bound::AtMost(_) => value == &Rational::from_integer(1_i64.into()),
                    })
                })
                .collect::<Vec<_>>();
            let past = match bound {
                Bound::AtLeast(value) => Rational::from_integer(1_i64.into()).sub(value),
                Bound::AtMost(value) => value.clone(),
            };
            produce_witness(product, policy, scope, initial, &accepting, &past, limits)
        }
        _ => return not_yet(),
    };
    if let Some(issue) = issue {
        return settled(&issue, subject);
    }
    match produced {
        Ok(Some(produced)) => replay_produced_witness(
            system, claim, product, produced, initial, binding, None, limits, cancel, subject,
            tamper, meter, storage,
        ),
        Ok(None) => not_yet(),
        Err(WitnessError::Limit { limit, value }) => {
            settled(&ExactProbBuildIssue::Limit { limit, value }, subject)
        }
        Err(_) => broken(),
    }
}

fn induction_failure(error: QualifiedFhError, cancel: &Cancel) -> ProbabilityDocumentError {
    if let Some(cause) = cancel.tripped() {
        return ProbabilityDocumentError::Cancelled(cause);
    }
    match error {
        QualifiedFhError::Accounting(error) => ProbabilityDocumentError::Accounting(error),
        QualifiedFhError::Storage { bound, actual } => ProbabilityDocumentError::Limit {
            limit: fsl_replay::ProbabilityDocumentLimit::StorageBytes,
            bound,
            actual,
        },
        QualifiedFhError::RationalBits { bound, actual } => ProbabilityDocumentError::Limit {
            limit: fsl_replay::ProbabilityDocumentLimit::RationalBits,
            bound,
            actual,
        },
        QualifiedFhError::Graph(error) => match error {
            FhError::Cycle => ProbabilityDocumentError::Graph(ExactProbabilityGraphError::Cycle),
            FhError::InvalidState
            | FhError::MissingAction
            | FhError::InvalidProbability
            | FhError::InvalidPrecision => ProbabilityDocumentError::Malformed,
        },
    }
}

#[allow(clippy::too_many_arguments)]
fn refute_mean(
    system: &ModelSystem<'_>,
    claim: &fsl_semantics::check::CheckedProbabilisticClaim,
    product: &ProbProduct,
    policy: &[Option<usize>],
    initial: crate::exact_probabilistic::ProbStateId,
    binding: Option<&ObjectReference>,
    certificate: &ExactProbabilityCertificate,
    limits: &crate::exact_probabilistic::ExactProbLimits,
    cancel: &Cancel,
    subject: SubjectLimits<'_>,
    tamper: Option<&dyn Fn(&mut ProbabilisticCounterexample)>,
) -> EngineReport {
    let item = match fsl_replay::probabilistic_claim_identity(claim) {
        Ok(item) => item,
        Err(_) => return broken(),
    };
    let Some(package) = system.source_package() else {
        return not_yet();
    };
    let document_limits = ProbabilityDocumentLimits::for_exact(limits, system.meter_limits());
    let document = match measure_probability_certificate_document(
        system,
        package.graph().probabilistic_model().claims(),
        &item,
        certificate,
        MeasurePayoffKind::Fraction,
        document_limits,
        cancel,
    ) {
        Ok(document) => document,
        Err(error) => return document_failure(error, subject),
    };
    let subsystem = match fsl_replay::ProbabilitySubsystemCertificate::from_document(
        &document,
        document_limits,
        cancel,
    ) {
        Ok(subsystem) => subsystem,
        Err(error) => return document_failure(error, subject),
    };
    let key_of = |state: &crate::exact_probabilistic::ProbKey| {
        state
            .certificate_state
            .clone()
            .ok_or(ProbabilityDocumentError::Malformed)
    };
    let mut scheduler = Vec::new();
    if matches!(claim.scheduler, ProbScheduler::Every) {
        for state in product.ids() {
            let Some(action) = crate::exact_probabilistic::policy_action(product, state, policy)
            else {
                continue;
            };
            let Some(node) = product.states.get(state.index()) else {
                return broken();
            };
            let key = match key_of(node) {
                Ok(key) => key,
                Err(error) => return document_failure(error, subject),
            };
            let action = match canonical_action(system, product, &action, &key_of) {
                Ok(action) => action,
                Err(_) => return not_yet(),
            };
            scheduler.push((key, action));
        }
    }
    scheduler.sort_by(|a, b| a.0.cmp(&b.0));
    let initials = system.initial();
    let [first] = initials.as_slice() else {
        return not_yet();
    };
    let initial_key = match product
        .states
        .get(initial.index())
        .and_then(|state| state.certificate_state.clone())
    {
        Some(key) => key,
        None => return broken(),
    };
    let mut witness = ProbabilisticCounterexample {
        item: item.clone(),
        binding: binding.cloned(),
        initial: first.clone(),
        initial_key,
        scheduler,
        paths: Vec::new(),
        subsystem: Some(Box::new(subsystem)),
    };
    if let Some(tamper) = tamper {
        tamper(&mut witness);
    }
    match replay_probabilistic_witness(package, system, &item, &witness, limits, cancel) {
        Ok(ProbabilityWitnessReplay::Reproduced(token)) => {
            let ProbForm::MeanOfFraction { bound, .. } = &claim.form else {
                return broken();
            };
            let mut spelled = Vec::new();
            for (key, action) in &token.evidence().scheduler {
                let fsl_replay::ProductAction::Identity(schedule) = action else {
                    continue;
                };
                let Some(state) = product.ids().find(|state| {
                    product
                        .states
                        .get(state.index())
                        .and_then(|state| state.certificate_state.as_ref())
                        == Some(key)
                }) else {
                    return broken();
                };
                let identity = product.outgoing(state).iter().find_map(|edge| {
                    let crate::exact_probabilistic::ProductAction::Identity(identity) =
                        &edge.choice
                    else {
                        return None;
                    };
                    (fsl_replay::Schedule::of(system, identity).as_ref() == Ok(schedule))
                        .then_some(identity)
                });
                let Some(identity) = identity else {
                    return broken();
                };
                spelled.push((
                    fsl_eval::simulation::StateKey::from_canonical_bytes(key.as_bytes().to_vec()),
                    identity.clone(),
                ));
            }
            let scope = package.graph().scope();
            let wire = fsl_replay::probabilistic_wire(
                token.evidence(),
                bound,
                scope.types(),
                scope.variant_types(),
                &|state| system.state_key(state).ok().map(|(_, key)| key),
                &spelled,
            );
            EngineReport::Exact(Box::new(ExactReport::Refuted(token, Box::new(wire))))
        }
        Ok(ProbabilityWitnessReplay::Parity) => {
            EngineReport::Exact(Box::new(ExactReport::ReplayParity))
        }
        Err(error) => undefined::failure(error, subject, limits),
    }
}
