// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-202 finite event-position witness replay, independent of solver products.

use super::*;
use fsl_eval::automaton::{
    AutomatonError, BoundedState, QualifiedBoundedError, QualifiedBoundedMonitor, StateId,
};
use fsl_semantics::check::Event;

pub(super) fn replay(
    system: &ModelSystem<'_>,
    claim: &CheckedProbabilisticClaim,
    witness: &ProbabilisticCounterexample,
    limits: &ExactProbLimits,
    cancel: &Cancel,
) -> Result<ProbabilityWitnessReplay, ProbabilityWitnessError> {
    let ProbForm::Probability {
        bound,
        event:
            Event::Formula {
                formula,
                timed_horizon: None,
                ..
            },
    } = &claim.form
    else {
        return Err(ExactProbabilityGraphError::UnsupportedClaimForm.into());
    };
    match bound {
        Bound::AtLeast(threshold) | Bound::AtMost(threshold) => {
            check_bits(threshold, limits.max_rational_bits)?
        }
    }
    if !claim.fairness.is_empty()
        || !matches!(
            TimedSystem::new(system, &claim.subject.alias),
            Err(TimedSubjectRefusal::NoTimedModel { .. })
        )
    {
        return Err(ExactProbabilityGraphError::UnsupportedClaimForm.into());
    }
    let binding = claim_binding(system, claim)?;
    let reference = binding.as_ref().map(|binding| &binding.reference);
    if reference != witness.binding.as_ref() {
        return Err(ProbabilityWitnessError::Refused);
    }
    let context = probabilistic_formula_monitor_context(claim, reference)?;
    let mut monitor = QualifiedBoundedMonitor::new(
        formula.formula(),
        &context,
        limits.model.max_automaton_states,
    )
    .map_err(monitor_error)?;
    let predicate = formula
        .condition(monitor.atom().index())
        .ok_or(ExactProbabilityGraphError::UnsupportedClaimForm)?;
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
    let initial = system.initial();
    let [initial] = initial.as_slice() else {
        return Err(ExactProbabilityGraphError::UnsupportedInitial.into());
    };
    let root = monitor
        .start(observes(system, predicate, reference, initial, cancel)?)
        .map_err(monitor_error)?;
    let key_of = |state: &ModelSimState, control: StateId, monitor: &QualifiedBoundedMonitor| {
        let (bytes, _) = system
            .state_key(state)
            .map_err(|_| ExactProbabilityGraphError::ExpansionRefused)?;
        let monitor_key = monitor.key(control).map_err(monitor_error)?;
        ProbabilityProductKey::for_formula_state(
            reference,
            &bytes,
            &monitor_key,
            limits.document,
            cancel,
        )
        .map_err(document_error)
    };
    let root_key = key_of(initial, root, &monitor)?;
    if root_key != witness.initial_key
        || key_of(&witness.initial, root, &monitor)? != root_key
        || initial.anchor() != witness.initial.anchor()
        || initial.reached_by() != witness.initial.reached_by()
    {
        return Err(ProbabilityWitnessError::Refused);
    }
    let mut seen = BTreeSet::new();
    let mut visited_choices = BTreeSet::new();
    let mut transition_count = 0usize;
    let mut storage = 0u64;
    let mut meter = filament_exact::Meter::new(system.meter_limits()).with_cancel(cancel.clone());
    let mut mass = rational(0);
    // A cylinder includes its semantic choice and draw, even when several
    // disjoint draws produce exactly the same model and monitor state.
    let mut cylinders: Vec<&ProbabilityWitnessPath> = Vec::new();
    let scheduler = step::scheduler(&witness.scheduler, limits, &mut meter, &mut storage, cancel)?;
    for path in &witness.paths {
        poll_cancel(cancel)?;
        let mut state = initial.clone();
        let mut control = root;
        let mut key = root_key.clone();
        let mut probability = rational(1);
        for step in &path.steps {
            poll_cancel(cancel)?;
            admit_state(&key, &mut seen, &mut storage, limits)?;
            if !matches!(
                monitor.state(control).map_err(monitor_error)?,
                BoundedState::Open { .. }
            ) {
                return Ok(ProbabilityWitnessReplay::Parity);
            }
            if workload.is_none() && scheduler.get(&key).copied() != Some(&step.choice) {
                return Err(ProbabilityWitnessError::Refused);
            }
            visited_choices.insert(key.clone());
            let (step_mass, post, (next, post_key)) = step::replay(
                system,
                &state,
                workload,
                step,
                limits,
                &mut meter,
                &mut storage,
                &mut transition_count,
                cancel,
                step::ActionRefusal::ExpansionRefused,
                |post, _, _, _| {
                    let next = monitor
                        .step(
                            control,
                            observes(system, predicate, reference, post, cancel)?,
                        )
                        .map_err(monitor_error)?;
                    let post_key = key_of(post, next, &monitor)?;
                    Ok((post_key == step.post_key
                        && key_of(&step.post, next, &monitor)? == post_key)
                        .then_some((next, post_key)))
                },
            )?;
            probability = arithmetic(
                filament_exact::RationalArithmetic::Multiply(&probability, &step_mass),
                &mut meter,
                cancel,
            )?;
            check_bits(&probability, limits.max_rational_bits)?;
            retain(post_key.as_bytes().len(), &mut storage, limits)?;
            state = post;
            control = next;
            key = post_key;
        }
        admit_state(&key, &mut seen, &mut storage, limits)?;
        let mut verdict = monitor.state(control).map_err(monitor_error)?;
        if matches!(verdict, BoundedState::Open { .. }) {
            let mut issue = None;
            let distributions =
                model_distributions(system, &state, workload, cancel, &mut |failure| {
                    issue = Some(failure)
                })
                .map_err(|error| {
                    issue.map_or(ProbabilityWitnessError::Graph(error), |issue| {
                        ProbabilityWitnessError::Build(Box::new(issue))
                    })
                })?;
            if !distributions.is_empty() {
                return Ok(ProbabilityWitnessReplay::Parity);
            }
            verdict = monitor.terminal(control).map_err(monitor_error)?;
        }
        if !matches!(
            (bound, verdict),
            (Bound::AtMost(_), BoundedState::Accept) | (Bound::AtLeast(_), BoundedState::Reject)
        ) {
            return Ok(ProbabilityWitnessReplay::Parity);
        }
        for previous in &cylinders {
            poll_cancel(cancel)?;
            let mut prefix = true;
            for (a, b) in previous.steps.iter().zip(&path.steps) {
                poll_cancel(cancel)?;
                if a.action != b.action
                    || a.transition != b.transition
                    || !super::same_draw(&a.draw, &b.draw)
                    || a.post_key != b.post_key
                {
                    prefix = false;
                    break;
                }
            }
            if prefix {
                return Ok(ProbabilityWitnessReplay::Parity);
            }
        }
        retain(
            std::mem::size_of::<&ProbabilityWitnessPath>(),
            &mut storage,
            limits,
        )?;
        cylinders.push(path);
        mass = arithmetic(
            filament_exact::RationalArithmetic::Add(&mass, &probability),
            &mut meter,
            cancel,
        )?;
        check_bits(&mass, limits.max_rational_bits)?;
    }
    if scheduler.keys().any(|key| !visited_choices.contains(*key)) {
        return Err(ProbabilityWitnessError::Refused);
    }
    let threshold = match bound {
        Bound::AtMost(threshold) => threshold.clone(),
        Bound::AtLeast(threshold) => arithmetic(
            filament_exact::RationalArithmetic::Subtract(&rational(1), threshold),
            &mut meter,
            cancel,
        )?,
    };
    check_bits(&threshold, limits.max_rational_bits)?;
    if mass <= threshold {
        return Ok(ProbabilityWitnessReplay::Parity);
    }
    let evidence = witness.clone();
    poll_cancel(cancel)?;
    Ok(ProbabilityWitnessReplay::Reproduced(Box::new(
        ReproducedProbabilityWitness {
            evidence,
            mass,
            undefined: None,
        },
    )))
}

pub(super) fn admit_state(
    key: &ProbabilityProductKey,
    seen: &mut BTreeSet<ProbabilityProductKey>,
    storage: &mut u64,
    limits: &ExactProbLimits,
) -> Result<(), ProbabilityWitnessError> {
    if seen.contains(key) {
        return Ok(());
    }
    if seen.len() >= limits.model.limits.max_states {
        return Err(ExactProbabilityGraphError::LimitReached.into());
    }
    retain(key.as_bytes().len(), storage, limits)?;
    seen.insert(key.clone());
    Ok(())
}

pub(super) fn retain(
    bytes: usize,
    storage: &mut u64,
    limits: &ExactProbLimits,
) -> Result<(), ProbabilityWitnessError> {
    *storage = storage.saturating_add(u64::try_from(bytes).unwrap_or(u64::MAX));
    if *storage > limits.document.storage_bytes {
        return Err(ProbabilityWitnessError::Limit {
            limit: ModelCheckLimit::CertificateMaxWorkingBytes,
            value: limits.document.storage_bytes,
        });
    }
    Ok(())
}
pub(super) fn visit(
    meter: &mut filament_exact::Meter,
    cancel: &Cancel,
) -> Result<(), ProbabilityWitnessError> {
    poll_cancel(cancel)?;
    if let Err(incomplete) = meter.charge(filament_exact::Charge::new(
        filament_exact::ChargePoint::CollectionMemberTest,
    )) {
        poll_cancel(cancel)?;
        return Err(ProbabilityWitnessError::Build(Box::new(
            crate::ExactProbBuildIssue::Predicate(
                fsl_eval::simulation::model::ClauseOutcome::Incomplete(incomplete),
            ),
        )));
    }
    Ok(())
}
pub(super) fn arithmetic(
    operation: filament_exact::RationalArithmetic<'_>,
    meter: &mut filament_exact::Meter,
    cancel: &Cancel,
) -> Result<Rational, ProbabilityWitnessError> {
    poll_cancel(cancel)?;
    match filament_exact::evaluate_rational_arithmetic(operation, None, meter) {
        filament_exact::Outcome::Completed(value) => Ok(value),
        filament_exact::Outcome::Incomplete(incomplete) => {
            poll_cancel(cancel)?;
            Err(ProbabilityWitnessError::Build(Box::new(
                crate::ExactProbBuildIssue::Predicate(
                    fsl_eval::simulation::model::ClauseOutcome::Incomplete(incomplete),
                ),
            )))
        }
        _ => Err(ProbabilityWitnessError::Refused),
    }
}
pub(super) fn document_error(error: ProbabilityDocumentError) -> ProbabilityWitnessError {
    error.into()
}

pub(super) fn monitor_error(error: QualifiedBoundedError) -> ProbabilityWitnessError {
    match error {
        QualifiedBoundedError::Translation(_) => {
            ExactProbabilityGraphError::UnsupportedClaimForm.into()
        }
        QualifiedBoundedError::Automaton(AutomatonError::StateLimit { limit }) => {
            ProbabilityWitnessError::Limit {
                limit: ModelCheckLimit::ExactProbMaxAutomatonStates,
                value: limit,
            }
        }
        QualifiedBoundedError::Automaton(_) | QualifiedBoundedError::Key(_) => {
            ExactProbabilityGraphError::ExpansionRefused.into()
        }
    }
}
