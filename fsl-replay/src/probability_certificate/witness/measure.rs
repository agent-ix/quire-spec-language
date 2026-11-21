// SPDX-License-Identifier: AGPL-3.0-or-later
//! Native bounded-measure path sets and quantile two-mass replay (FR-202).
use super::super::document::measure::ordered;
use super::super::document::measure::{measure_probability_product_key, replay_control};
use super::*;
use filament_exact::OrderingOperator as Order;
use fsl_eval::probabilistic::{Accumulator, MeasureError, MonitorState};

fn limits_of(system: &ModelSystem<'_>, limits: &ExactProbLimits) -> ProbabilityDocumentLimits {
    ProbabilityDocumentLimits::for_exact(limits, system.meter_limits())
}
fn measure_error(error: MeasureError) -> ProbabilityWitnessError {
    bounded::document_error(ProbabilityDocumentError::from(error))
}
fn read(
    system: &ModelSystem<'_>,
    predicates: &[&fsl_semantics::check::CheckedPredicate],
    binding: Option<&ObjectReference>,
    state: &ModelSimState,
    cancel: &Cancel,
) -> Result<Vec<bool>, ProbabilityWitnessError> {
    let inputs = predicates
        .iter()
        .map(|predicate| (*predicate, binding))
        .collect::<Vec<_>>();
    system
        .state_predicates_with_cancel(state, &inputs, cancel)
        .map_err(|stop| cancelled_expansion(stop, cancel))?
        .into_iter()
        .map(|reading| match reading {
            PredicateReading::True => Ok(true),
            PredicateReading::False => Ok(false),
            PredicateReading::Refused(outcome) => Err(ProbabilityWitnessError::Build(Box::new(
                crate::ExactProbBuildIssue::Predicate(outcome),
            ))),
            PredicateReading::Undefined { .. } => Err(ProbabilityWitnessError::Refused),
        })
        .collect()
}
#[allow(clippy::too_many_arguments)]
fn key(
    system: &ModelSystem<'_>,
    binding: Option<&ObjectReference>,
    state: &ModelSimState,
    control: MonitorState,
    accumulator: &Accumulator,
    reward: Option<&ProbabilityRewardObjective>,
    limits: ProbabilityDocumentLimits,
    meter: &mut filament_exact::Meter,
    storage: &mut u64,
    cancel: &Cancel,
) -> Result<ProbabilityProductKey, ProbabilityWitnessError> {
    let raw = {
        let mut admission =
            ProbabilityKeyAdmission::new(meter, storage, limits.storage_bytes, cancel);
        system
            .state_key_bytes_with_budget(
                state,
                fsl_foundation::IdentityLimits::default(),
                &mut admission,
                cancel,
            )
            .map_err(|error| bounded::document_error(admission.refusal(error)))?
    };
    measure_probability_product_key(
        binding,
        &raw,
        control,
        accumulator,
        None,
        reward,
        limits,
        cancel,
        meter,
        storage,
    )
    .map_err(bounded::document_error)
}

/// Check the second scheduler choice of a physical draw before permitting
/// resolved replay. Its key retains the source control and exact draw.
#[allow(clippy::too_many_arguments)]
fn residual_choice(
    system: &ModelSystem<'_>,
    state: &ModelSimState,
    reference: Option<&ObjectReference>,
    control: MonitorState,
    sums: &Accumulator,
    reward: Option<&ProbabilityRewardObjective>,
    offered: &ProbabilityWitnessStep,
    workload: Option<&Workload>,
    scheduler: &BTreeMap<&ProbabilityProductKey, &ProductAction>,
    used: &mut BTreeSet<ProbabilityProductKey>,
    seen: &mut BTreeSet<ProbabilityProductKey>,
    edges: &mut usize,
    execution: &ExactProbLimits,
    limits: ProbabilityDocumentLimits,
    meter: &mut filament_exact::Meter,
    storage: &mut u64,
    cancel: &Cancel,
) -> Result<step::ActionRefusal, ProbabilityWitnessError> {
    if workload.is_some() || matches!(offered.choice, ProductAction::TerminalStutter) {
        return Ok(step::ActionRefusal::UnsupportedDraw);
    }
    let (Some(identity), Some(draw)) = (&offered.action, &offered.draw) else {
        return Err(ProbabilityWitnessError::Refused);
    };
    let mut admission = ProbabilityKeyAdmission::new(meter, storage, limits.storage_bytes, cancel);
    let actions = system
        .actions_with_budget(
            state,
            fsl_foundation::IdentityLimits::default(),
            &mut admission,
            cancel,
        )
        .map_err(|error| match error {
            fsl_eval::simulation::model::ModelKeyExpansionError::Key(error) => {
                bounded::document_error(admission.refusal(error))
            }
            fsl_eval::simulation::model::ModelKeyExpansionError::Expansion(error) => {
                cancelled_expansion(error, cancel)
            }
        })?
        .map_err(|_| ExactProbabilityGraphError::UnsupportedDraw)?;
    let residual = actions
        .iter()
        .filter(|action| &action.identity == identity)
        .flat_map(|action| &action.draws)
        .any(|candidate| same_values(&candidate.random, draw) && candidate.successors.len() > 1);
    if !residual {
        return Ok(step::ActionRefusal::UnsupportedDraw);
    }
    let raw = system
        .state_key_bytes_with_budget(
            state,
            fsl_foundation::IdentityLimits::default(),
            &mut admission,
            cancel,
        )
        .map_err(|error| bounded::document_error(admission.refusal(error)))?;
    let pending = system
        .pending_key_bytes_with_budget(
            identity,
            draw,
            fsl_foundation::IdentityLimits::default(),
            &mut admission,
            cancel,
        )
        .map_err(|error| bounded::document_error(admission.refusal(error)))?;
    drop(admission);
    let pending = measure_probability_product_key(
        reference,
        &raw,
        control,
        sums,
        Some(&pending),
        reward,
        limits,
        cancel,
        meter,
        storage,
    )
    .map_err(bounded::document_error)?;
    bounded::admit_state(&pending, seen, storage, execution)?;
    *edges = edges
        .checked_add(1)
        .ok_or(ExactProbabilityGraphError::TooManyEdges)?;
    if *edges > execution.model.limits.max_transitions {
        return Err(ExactProbabilityGraphError::EdgeLimitReached.into());
    }
    let expected = ProductAction::PostState(offered.post_key.clone());
    if scheduler.get(&pending).copied() != Some(&expected) {
        return Err(ProbabilityWitnessError::Refused);
    }
    let actual =
        storage.saturating_add(u64::try_from(pending.as_bytes().len()).unwrap_or(u64::MAX));
    if actual > limits.storage_bytes {
        return Err(ProbabilityWitnessError::Limit {
            limit: ModelCheckLimit::CertificateMaxWorkingBytes,
            value: limits.storage_bytes,
        });
    }
    *storage = actual;
    used.insert(pending);
    Ok(step::ActionRefusal::CheckedResidual)
}

pub(super) fn replay(
    system: &ModelSystem<'_>,
    claim: &CheckedProbabilisticClaim,
    witness: &ProbabilisticCounterexample,
    limits: &ExactProbLimits,
    cancel: &Cancel,
) -> Result<ProbabilityWitnessReplay, ProbabilityWitnessError> {
    let kind = match &claim.form {
        ProbForm::Probability { .. } => MeasurePayoffKind::Event,
        ProbForm::Quantile { .. } => MeasurePayoffKind::Satisfied,
        _ => return Err(ExactProbabilityGraphError::UnsupportedClaimForm.into()),
    };
    let document_limits = limits_of(system, limits);
    let mut meter = filament_exact::Meter::new(system.meter_limits()).with_cancel(cancel.clone());
    let mut storage = 0;
    let selected = replay_control(
        system,
        claim,
        kind,
        document_limits,
        cancel,
        &mut meter,
        &mut storage,
    )
    .map_err(bounded::document_error)?;
    let binding = claim_binding(system, claim)?;
    let reference = binding.as_ref().map(|binding| &binding.reference);
    if reference != witness.binding.as_ref() {
        return Err(ProbabilityWitnessError::Refused);
    }
    let workload = claim_workload(system, claim)?;
    if workload.is_some() && !witness.scheduler.is_empty() {
        return Err(ProbabilityWitnessError::Refused);
    }
    if u64::try_from(witness.paths.len()).unwrap_or(u64::MAX) > limits.max_witness_paths {
        return Err(ProbabilityWitnessError::Limit {
            limit: ModelCheckLimit::ExactProbMaxWitnessPaths,
            value: limits.max_witness_paths,
        });
    }
    let initials = system.initial();
    let [initial] = initials.as_slice() else {
        return Err(ExactProbabilityGraphError::UnsupportedInitial.into());
    };
    let root_truths = read(system, &selected.predicates, reference, initial, cancel)?;
    let (root_control, root_sums) = selected
        .control
        .start(&root_truths, &mut meter)
        .map_err(measure_error)?;
    let root_control = root_control.ok_or(ProbabilityWitnessError::Refused)?;
    let root_sums = root_sums.ok_or(ProbabilityWitnessError::Refused)?;
    let root = key(
        system,
        reference,
        initial,
        root_control,
        &root_sums,
        selected.reward.as_ref(),
        document_limits,
        &mut meter,
        &mut storage,
        cancel,
    )?;
    if root != witness.initial_key
        || key(
            system,
            reference,
            &witness.initial,
            root_control,
            &root_sums,
            selected.reward.as_ref(),
            document_limits,
            &mut meter,
            &mut storage,
            cancel,
        )? != root
        || initial.anchor() != witness.initial.anchor()
        || initial.reached_by() != witness.initial.reached_by()
    {
        return Err(ProbabilityWitnessError::Refused);
    }
    let scheduler = step::scheduler(&witness.scheduler, limits, &mut meter, &mut storage, cancel)?;
    let mut used = BTreeSet::new();
    let mut seen = BTreeSet::new();
    let mut edges = 0;
    let mut cylinders: Vec<&ProbabilityWitnessPath> = Vec::new();
    let mut violating = rational(0);
    let mut never = rational(0);
    for path in &witness.paths {
        poll_cancel(cancel)?;
        let mut state = initial.clone();
        let mut control = root_control;
        let mut sums = root_sums.clone();
        let mut truths = root_truths.clone();
        let mut current = root.clone();
        let mut mass = rational(1);
        for offered in &path.steps {
            bounded::admit_state(&current, &mut seen, &mut storage, limits)?;
            if control.is_decided() {
                return Ok(ProbabilityWitnessReplay::Parity);
            }
            if workload.is_none() && !matches!(offered.choice, ProductAction::TerminalStutter) {
                if scheduler.get(&current).copied() != Some(&offered.choice) {
                    return Err(ProbabilityWitnessError::Refused);
                }
                used.insert(current.clone());
            }
            let refusal = residual_choice(
                system,
                &state,
                reference,
                control,
                &sums,
                selected.reward.as_ref(),
                offered,
                workload,
                &scheduler,
                &mut used,
                &mut seen,
                &mut edges,
                limits,
                document_limits,
                &mut meter,
                &mut storage,
                cancel,
            )?;
            let (probability, post, (next, next_sums, post_truths, post_key)) = step::replay(
                system,
                &state,
                workload,
                offered,
                limits,
                &mut meter,
                &mut storage,
                &mut edges,
                cancel,
                refusal,
                |post, rewards, meter, storage| {
                    let post_truths = if selected.control.reads_post(Some(control)) {
                        read(system, &selected.predicates, reference, post, cancel)?
                    } else {
                        Vec::new()
                    };
                    let (next, next_sums) = selected
                        .control
                        .step(
                            Some(control),
                            Some(&sums),
                            &truths,
                            &post_truths,
                            rewards,
                            meter,
                        )
                        .map_err(measure_error)?;
                    let next = next.ok_or(ProbabilityWitnessError::Refused)?;
                    let next_sums = next_sums.ok_or(ProbabilityWitnessError::Refused)?;
                    let post_key = key(
                        system,
                        reference,
                        post,
                        next,
                        &next_sums,
                        selected.reward.as_ref(),
                        document_limits,
                        meter,
                        storage,
                        cancel,
                    )?;
                    let offered_key = key(
                        system,
                        reference,
                        &offered.post,
                        next,
                        &next_sums,
                        selected.reward.as_ref(),
                        document_limits,
                        meter,
                        storage,
                        cancel,
                    )?;
                    Ok(
                        (post_key == offered.post_key && post_key == offered_key).then_some((
                            next,
                            next_sums,
                            post_truths,
                            post_key,
                        )),
                    )
                },
            )?;
            mass = bounded::arithmetic(
                filament_exact::RationalArithmetic::Multiply(&mass, &probability),
                &mut meter,
                cancel,
            )?;
            check_bits(&mass, limits.max_rational_bits)?;
            state = post;
            control = next;
            sums = next_sums;
            truths = post_truths;
            current = post_key;
        }
        bounded::admit_state(&current, &mut seen, &mut storage, limits)?;
        let terminal = selected
            .payoff
            .terminal(control, &sums, &mut meter)
            .map_err(measure_error)?;
        let Some(value) = terminal else {
            return Ok(ProbabilityWitnessReplay::Parity);
        };
        let unactivated = matches!(
            sums,
            Accumulator::Steps {
                activated: false,
                ..
            } | Accumulator::Reward {
                activated: false,
                ..
            }
        );
        let is_violating = match &claim.form {
            ProbForm::Quantile {
                threshold: Bound::AtMost(_),
                ..
            } => !unactivated && value == rational(0),
            ProbForm::Quantile {
                threshold: Bound::AtLeast(_),
                ..
            } => !unactivated && value == rational(1),
            ProbForm::Probability {
                bound: Bound::AtLeast(_),
                ..
            } => value == rational(0),
            ProbForm::Probability {
                bound: Bound::AtMost(_),
                ..
            } => value == rational(1),
            _ => return Err(ProbabilityWitnessError::Refused),
        };
        if !is_violating && !(unactivated && matches!(claim.form, ProbForm::Quantile { .. })) {
            return Ok(ProbabilityWitnessReplay::Parity);
        }
        for previous in &cylinders {
            bounded::visit(&mut meter, cancel)?;
            if previous.steps.iter().zip(&path.steps).all(|(a, b)| {
                a.choice == b.choice
                    && a.transition == b.transition
                    && same_draw(&a.draw, &b.draw)
                    && a.post_key == b.post_key
            }) {
                return Ok(ProbabilityWitnessReplay::Parity);
            }
        }
        bounded::retain(
            std::mem::size_of::<&ProbabilityWitnessPath>(),
            &mut storage,
            limits,
        )?;
        cylinders.push(path);
        if unactivated && matches!(claim.form, ProbForm::Quantile { .. }) {
            never = bounded::arithmetic(
                filament_exact::RationalArithmetic::Add(&never, &mass),
                &mut meter,
                cancel,
            )?;
        } else {
            violating = bounded::arithmetic(
                filament_exact::RationalArithmetic::Add(&violating, &mass),
                &mut meter,
                cancel,
            )?;
        }
    }
    if scheduler.keys().any(|key| !used.contains(*key)) {
        return Err(ProbabilityWitnessError::Refused);
    }
    let one_minus = |value: &Rational, meter: &mut filament_exact::Meter| {
        bounded::arithmetic(
            filament_exact::RationalArithmetic::Subtract(&rational(1), value),
            meter,
            cancel,
        )
    };
    let holds = match &claim.form {
        ProbForm::Quantile { q, threshold, .. } => {
            let remaining = one_minus(&never, &mut meter)?;
            let coefficient = match threshold {
                Bound::AtMost(_) => one_minus(q, &mut meter)?,
                Bound::AtLeast(_) => q.clone(),
            };
            let threshold_mass = bounded::arithmetic(
                filament_exact::RationalArithmetic::Multiply(&coefficient, &remaining),
                &mut meter,
                cancel,
            )?;
            match threshold {
                Bound::AtMost(_) => ordered(
                    Order::Greater,
                    &violating,
                    &threshold_mass,
                    &mut meter,
                    cancel,
                )
                .map_err(bounded::document_error)?,
                Bound::AtLeast(_) => {
                    ordered(Order::Greater, &violating, &rational(0), &mut meter, cancel)
                        .map_err(bounded::document_error)?
                        && ordered(
                            Order::GreaterOrEqual,
                            &violating,
                            &threshold_mass,
                            &mut meter,
                            cancel,
                        )
                        .map_err(bounded::document_error)?
                }
            }
        }
        ProbForm::Probability {
            bound: Bound::AtLeast(threshold),
            ..
        } => {
            let threshold = one_minus(threshold, &mut meter)?;
            ordered(Order::Greater, &violating, &threshold, &mut meter, cancel)
                .map_err(bounded::document_error)?
        }
        ProbForm::Probability {
            bound: Bound::AtMost(threshold),
            ..
        } => ordered(Order::Greater, &violating, threshold, &mut meter, cancel)
            .map_err(bounded::document_error)?,
        _ => false,
    };
    if !holds {
        return Ok(ProbabilityWitnessReplay::Parity);
    }
    let mass = bounded::arithmetic(
        filament_exact::RationalArithmetic::Add(&violating, &never),
        &mut meter,
        cancel,
    )?;
    poll_cancel(cancel)?;
    Ok(ProbabilityWitnessReplay::Reproduced(Box::new(
        ReproducedProbabilityWitness {
            evidence: witness.clone(),
            mass,
            undefined: None,
        },
    )))
}

#[allow(clippy::too_many_arguments)]
fn finish_undefined(
    witness: &ProbabilityUndefinedWitness,
    evaluation: crate::UndefinedEvaluation,
    position: usize,
    pre_key: ProbabilityProductKey,
    scheduler: &BTreeMap<&ProbabilityProductKey, &ProductAction>,
    used: &BTreeSet<ProbabilityProductKey>,
    mass: Rational,
    seen: &mut BTreeSet<ProbabilityProductKey>,
    storage: &mut u64,
    limits: &ExactProbLimits,
    cancel: &Cancel,
) -> Result<ProbabilityWitnessReplay, ProbabilityWitnessError> {
    let evidence = &witness.path;
    let [path] = evidence.paths.as_slice() else {
        return Err(ProbabilityWitnessError::Refused);
    };
    let offered = if position == 0 {
        &evidence.initial_key
    } else {
        &path.steps[position - 1].post_key
    };
    if position != path.steps.len()
        || evaluation != witness.evaluation
        || pre_key != witness.product_state
        || offered != &pre_key
        || scheduler.keys().any(|key| !used.contains(*key))
    {
        return Ok(ProbabilityWitnessReplay::Parity);
    }
    bounded::admit_state(&pre_key, seen, storage, limits)?;
    check_bits(&mass, limits.max_rational_bits)?;
    poll_cancel(cancel)?;
    Ok(ProbabilityWitnessReplay::Reproduced(Box::new(
        ReproducedProbabilityWitness {
            evidence: evidence.clone(),
            mass,
            undefined: Some(Box::new(witness.clone())),
        },
    )))
}

pub(super) fn replay_undefined(
    system: &ModelSystem<'_>,
    claim: &CheckedProbabilisticClaim,
    witness: &ProbabilityUndefinedWitness,
    limits: &ExactProbLimits,
    cancel: &Cancel,
) -> Result<ProbabilityWitnessReplay, ProbabilityWitnessError> {
    let evidence = &witness.path;
    let [path] = evidence.paths.as_slice() else {
        return Err(ProbabilityWitnessError::Refused);
    };
    if limits.max_witness_paths == 0 {
        return Err(ProbabilityWitnessError::Limit {
            limit: ModelCheckLimit::ExactProbMaxWitnessPaths,
            value: 0,
        });
    }
    let document_limits = limits_of(system, limits);
    let mut meter = filament_exact::Meter::new(system.meter_limits()).with_cancel(cancel.clone());
    let mut storage = 0;
    let kind = match &claim.form {
        ProbForm::Quantile { .. } => MeasurePayoffKind::Satisfied,
        ProbForm::MeanOfFraction { .. } => MeasurePayoffKind::Fraction,
        _ => MeasurePayoffKind::Event,
    };
    let selected = replay_control(
        system,
        claim,
        kind,
        document_limits,
        cancel,
        &mut meter,
        &mut storage,
    )
    .map_err(bounded::document_error)?;
    let binding = claim_binding(system, claim)?;
    let reference = binding.as_ref().map(|binding| &binding.reference);
    if reference != evidence.binding.as_ref() {
        return Err(ProbabilityWitnessError::Refused);
    }
    let workload = claim_workload(system, claim)?;
    if workload.is_some() && !evidence.scheduler.is_empty() {
        return Err(ProbabilityWitnessError::Refused);
    }
    let scheduler = step::scheduler(
        &evidence.scheduler,
        limits,
        &mut meter,
        &mut storage,
        cancel,
    )?;
    let initials = system.initial();
    let [initial] = initials.as_slice() else {
        return Err(ExactProbabilityGraphError::UnsupportedInitial.into());
    };
    let (control, sums) = selected
        .control
        .initial_control(&mut meter)
        .map_err(measure_error)?;
    let mut control = control.ok_or(ProbabilityWitnessError::Refused)?;
    let mut sums = sums.ok_or(ProbabilityWitnessError::Refused)?;
    if key(
        system,
        reference,
        initial,
        control,
        &sums,
        selected.reward.as_ref(),
        document_limits,
        &mut meter,
        &mut storage,
        cancel,
    )? != key(
        system,
        reference,
        &evidence.initial,
        control,
        &sums,
        selected.reward.as_ref(),
        document_limits,
        &mut meter,
        &mut storage,
        cancel,
    )? || initial.anchor() != evidence.initial.anchor()
        || initial.reached_by() != evidence.initial.reached_by()
    {
        return Err(ProbabilityWitnessError::Refused);
    }
    let mut state = initial.clone();
    let mut previous_truths = Vec::new();
    let mut rewards = Vec::new();
    let mut mass = rational(1);
    let mut seen = BTreeSet::new();
    let mut used = BTreeSet::new();
    let mut edges = 0;
    for position in 0..=path.steps.len() {
        bounded::visit(&mut meter, cancel)?;
        if position > 0 && control.is_decided() {
            return Ok(ProbabilityWitnessReplay::Parity);
        }
        let pre_key = key(
            system,
            reference,
            &state,
            control,
            &sums,
            selected.reward.as_ref(),
            document_limits,
            &mut meter,
            &mut storage,
            cancel,
        )?;
        let (_, state_key) = system
            .state_key(&state)
            .map_err(|_| ExactProbabilityGraphError::ExpansionRefused)?;
        if position > 0 && !selected.control.reads_post(Some(control)) {
            match selected.control.step(
                Some(control),
                Some(&sums),
                &previous_truths,
                &[],
                &rewards,
                &mut meter,
            ) {
                Err(MeasureError::Undefined(cause)) => {
                    let evaluation = crate::UndefinedEvaluation {
                        site: crate::UndefinedSite {
                            position: u64::try_from(position).unwrap_or(u64::MAX),
                            state_key,
                            locus: crate::UndefinedLocus::FractionValue,
                            transition: None,
                            time: None,
                        },
                        cause: fsl_eval::temporal::UndefinedCause::Kernel(*cause),
                    };
                    return finish_undefined(
                        witness,
                        evaluation,
                        position,
                        pre_key,
                        &scheduler,
                        &used,
                        mass,
                        &mut seen,
                        &mut storage,
                        limits,
                        cancel,
                    );
                }
                Err(error) => return Err(measure_error(error)),
                Ok(_) => return Ok(ProbabilityWitnessReplay::Parity),
            }
        }
        let inputs = selected
            .predicates
            .iter()
            .map(|predicate| (*predicate, reference))
            .collect::<Vec<_>>();
        let readings = system
            .state_predicates_with_cancel(&state, &inputs, cancel)
            .map_err(|stop| cancelled_expansion(stop, cancel))?;
        let mut truths = Vec::new();
        for (index, reading) in readings.into_iter().enumerate() {
            match reading {
                PredicateReading::True => truths.push(true),
                PredicateReading::False => truths.push(false),
                PredicateReading::Refused(outcome) => {
                    return Err(ProbabilityWitnessError::Build(Box::new(
                        crate::ExactProbBuildIssue::Predicate(outcome),
                    )))
                }
                PredicateReading::Undefined { cause, .. } => {
                    let predicate = if selected.predicates.len() == 1 {
                        crate::ClaimPredicate::Target
                    } else if index == 0 {
                        crate::ClaimPredicate::From
                    } else {
                        crate::ClaimPredicate::Until
                    };
                    let evaluation = crate::UndefinedEvaluation {
                        site: crate::UndefinedSite {
                            position: u64::try_from(position).unwrap_or(u64::MAX),
                            state_key,
                            locus: crate::UndefinedLocus::ClaimPredicate(predicate),
                            transition: None,
                            time: None,
                        },
                        cause,
                    };
                    return finish_undefined(
                        witness,
                        evaluation,
                        position,
                        pre_key,
                        &scheduler,
                        &used,
                        mass,
                        &mut seen,
                        &mut storage,
                        limits,
                        cancel,
                    );
                }
            }
        }
        let (next, next_sums) = if position == 0 {
            selected.control.start(&truths, &mut meter)
        } else {
            selected.control.step(
                Some(control),
                Some(&sums),
                &previous_truths,
                &truths,
                &rewards,
                &mut meter,
            )
        }
        .map_err(measure_error)?;
        control = next.ok_or(ProbabilityWitnessError::Refused)?;
        sums = next_sums.ok_or(ProbabilityWitnessError::Refused)?;
        let current = key(
            system,
            reference,
            &state,
            control,
            &sums,
            selected.reward.as_ref(),
            document_limits,
            &mut meter,
            &mut storage,
            cancel,
        )?;
        let offered = if position == 0 {
            &evidence.initial_key
        } else {
            &path.steps[position - 1].post_key
        };
        if offered != &current || control.is_decided() {
            return Ok(ProbabilityWitnessReplay::Parity);
        }
        bounded::admit_state(&current, &mut seen, &mut storage, limits)?;
        let Some(offered_step) = path.steps.get(position) else {
            return Ok(ProbabilityWitnessReplay::Parity);
        };
        if workload.is_none() && !matches!(offered_step.choice, ProductAction::TerminalStutter) {
            if scheduler.get(&current).copied() != Some(&offered_step.choice) {
                return Err(ProbabilityWitnessError::Refused);
            }
            used.insert(current);
        }
        let refusal = residual_choice(
            system,
            &state,
            reference,
            control,
            &sums,
            selected.reward.as_ref(),
            offered_step,
            workload,
            &scheduler,
            &mut used,
            &mut seen,
            &mut edges,
            limits,
            document_limits,
            &mut meter,
            &mut storage,
            cancel,
        )?;
        let (probability, post, next_rewards) = step::replay(
            system,
            &state,
            workload,
            offered_step,
            limits,
            &mut meter,
            &mut storage,
            &mut edges,
            cancel,
            refusal,
            |post, rewards, meter, storage| {
                let actual = key(
                    system,
                    reference,
                    post,
                    control,
                    &sums,
                    selected.reward.as_ref(),
                    document_limits,
                    meter,
                    storage,
                    cancel,
                )?;
                let offered = key(
                    system,
                    reference,
                    &offered_step.post,
                    control,
                    &sums,
                    selected.reward.as_ref(),
                    document_limits,
                    meter,
                    storage,
                    cancel,
                )?;
                bounded::retain(
                    rewards.len().saturating_mul(std::mem::size_of::<(
                        filament_exact::Identifier,
                        SemanticValue,
                    )>()),
                    storage,
                    limits,
                )?;
                Ok((actual == offered).then(|| rewards.to_vec()))
            },
        )?;
        mass = bounded::arithmetic(
            filament_exact::RationalArithmetic::Multiply(&mass, &probability),
            &mut meter,
            cancel,
        )?;
        check_bits(&mass, limits.max_rational_bits)?;
        previous_truths = truths;
        rewards = next_rewards;
        state = post;
    }
    Ok(ProbabilityWitnessReplay::Parity)
}

pub(super) fn replay_subsystem(
    system: &ModelSystem<'_>,
    claim: &CheckedProbabilisticClaim,
    witness: &ProbabilisticCounterexample,
    limits: &ExactProbLimits,
    cancel: &Cancel,
) -> Result<ProbabilityWitnessReplay, ProbabilityWitnessError> {
    if !matches!(claim.form, ProbForm::MeanOfFraction { .. }) || !witness.paths.is_empty() {
        return Err(ProbabilityWitnessError::Refused);
    }
    let subsystem = witness
        .subsystem
        .as_ref()
        .ok_or(ProbabilityWitnessError::Refused)?;
    let document_limits = limits_of(system, limits);
    let mut meter = filament_exact::Meter::new(system.meter_limits()).with_cancel(cancel.clone());
    let mut storage = 0;
    let binding = claim_binding(system, claim)?;
    let reference = binding.as_ref().map(|binding| &binding.reference);
    if reference != witness.binding.as_ref() {
        return Err(ProbabilityWitnessError::Refused);
    }
    let selected = replay_control(
        system,
        claim,
        MeasurePayoffKind::Fraction,
        document_limits,
        cancel,
        &mut meter,
        &mut storage,
    )
    .map_err(bounded::document_error)?;
    let initials = system.initial();
    let [initial] = initials.as_slice() else {
        return Err(ExactProbabilityGraphError::UnsupportedInitial.into());
    };
    let truths = read(system, &selected.predicates, reference, initial, cancel)?;
    let (control, sums) = selected
        .control
        .start(&truths, &mut meter)
        .map_err(measure_error)?;
    let control = control.ok_or(ProbabilityWitnessError::Refused)?;
    let sums = sums.ok_or(ProbabilityWitnessError::Refused)?;
    let initial_key = key(
        system,
        reference,
        initial,
        control,
        &sums,
        selected.reward.as_ref(),
        document_limits,
        &mut meter,
        &mut storage,
        cancel,
    )?;
    if initial_key != witness.initial_key
        || key(
            system,
            reference,
            &witness.initial,
            control,
            &sums,
            selected.reward.as_ref(),
            document_limits,
            &mut meter,
            &mut storage,
            cancel,
        )? != initial_key
        || initial.anchor() != witness.initial.anchor()
        || initial.reached_by() != witness.initial.reached_by()
    {
        return Err(ProbabilityWitnessError::Refused);
    }
    step::scheduler(&witness.scheduler, limits, &mut meter, &mut storage, cancel)?;
    let checked = super::super::document::measure::check_mean_refutation(
        system,
        claim,
        &witness.item,
        subsystem.as_bytes(),
        &witness.scheduler,
        document_limits,
        cancel,
    )
    .map_err(|error| match error {
        ProbabilityDocumentError::MalformedMember(member)
        | ProbabilityDocumentError::MissingMember(member) => {
            ProbabilityWitnessError::CertificateRejected(ProbabilityCertificateRejection {
                rule: ProbabilityCertificateRule::Malformed,
                at: ProbabilityCertificateLocus::Member(member),
            })
        }
        other => bounded::document_error(other),
    })?;
    match checked {
        super::super::document::measure::MeanRefutationCheck::Accepted
        | super::super::document::measure::MeanRefutationCheck::AcceptedBounds(_) => {
            let mut evidence = witness.clone();
            if let super::super::document::measure::MeanRefutationCheck::AcceptedBounds(policy) =
                checked
            {
                evidence.scheduler = policy;
            }
            poll_cancel(cancel)?;
            Ok(ProbabilityWitnessReplay::Reproduced(Box::new(
                ReproducedProbabilityWitness {
                    evidence,
                    mass: rational(1),
                    undefined: None,
                },
            )))
        }
        super::super::document::measure::MeanRefutationCheck::Rejected(rejection) => {
            Err(ProbabilityWitnessError::CertificateRejected(rejection))
        }
        super::super::document::measure::MeanRefutationCheck::PolicyParity => {
            Ok(ProbabilityWitnessReplay::Parity)
        }
    }
}
