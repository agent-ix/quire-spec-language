// SPDX-License-Identifier: AGPL-3.0-or-later
//! Checked-subject replay of a first undefined probability observation.

use super::*;
use fsl_eval::automaton::{BoundedState, QualifiedBoundedMonitor, StateId};
use fsl_semantics::check::Event;

/// Untrusted path and the exact pre-read location it claims is undefined.
#[derive(Clone, Debug)]
pub struct ProbabilityUndefinedWitness {
    /// Exactly one positive-probability path, with its scheduler and binding.
    pub path: ProbabilisticCounterexample,
    /// First undefined evaluation, including predicate locus and position.
    pub evaluation: crate::UndefinedEvaluation,
    /// Complete product key before the failing observation advances control.
    pub product_state: ProbabilityProductKey,
}

/// Re-execute an undefined path against its checked package and admitted subject.
/// The threshold has no role: only the first undefined observation of an
/// undecided claim can issue a reproduced-witness token. This entry admits
/// untimed origin bounded event-position and infinite-trace reach forms.
pub fn replay_probabilistic_undefined(
    package: &CheckedPackage,
    system: &ModelSystem<'_>,
    item: &ObligationIdentity,
    witness: &ProbabilityUndefinedWitness,
    limits: &ExactProbLimits,
    cancel: &Cancel,
) -> Result<ProbabilityWitnessReplay, ProbabilityWitnessError> {
    poll_cancel(cancel)?;
    let evidence = &witness.path;
    if evidence.subsystem.is_some() {
        return Err(ProbabilityWitnessError::Refused);
    }
    if !system
        .source_package()
        .is_some_and(|own| std::ptr::eq(package, own))
        || item != &evidence.item
    {
        return Err(ProbabilityWitnessError::Refused);
    }
    let claim = bind_claim(package.graph().probabilistic_model().claims(), item, cancel)?;
    if claim.bounded_measure() {
        return super::measure::replay_undefined(system, claim, witness, limits, cancel);
    }
    if !matches!(
        TimedSystem::new(system, &claim.subject.alias),
        Err(TimedSubjectRefusal::NoTimedModel { .. })
    ) {
        return Err(ExactProbabilityGraphError::UnsupportedClaimForm.into());
    }
    let binding = claim_binding(system, claim)?;
    let reference = binding.as_ref().map(|binding| &binding.reference);
    if reference != evidence.binding.as_ref() {
        return Err(ProbabilityWitnessError::Refused);
    }
    let workload = claim_workload(system, claim)?;
    if workload.is_some() && !evidence.scheduler.is_empty() {
        return Err(ProbabilityWitnessError::Refused);
    }
    let [path] = evidence.paths.as_slice() else {
        return Err(ProbabilityWitnessError::Refused);
    };
    if limits.max_witness_paths == 0 {
        return Err(ProbabilityWitnessError::Limit {
            limit: ModelCheckLimit::ExactProbMaxWitnessPaths,
            value: 0,
        });
    }
    let mut monitor = match &claim.form {
        ProbForm::Probability {
            event:
                Event::Formula {
                    formula,
                    timed_horizon: None,
                    ..
                },
            ..
        } => {
            let context = probabilistic_formula_monitor_context(claim, reference)?;
            Some(
                QualifiedBoundedMonitor::for_observations(
                    formula.formula(),
                    &context,
                    limits.model.max_automaton_states,
                )
                .map_err(bounded::monitor_error)?,
            )
        }
        ProbForm::Reach { .. }
            if fsl_semantics::check::TemporalProfile::from_identity(claim.profile.identity())
                == Some(fsl_semantics::check::TemporalProfile::InfiniteTrace) =>
        {
            None
        }
        _ => return Err(ExactProbabilityGraphError::UnsupportedClaimForm.into()),
    };
    let mut control: Option<StateId> = None;
    let mut storage = 0;
    let mut meter = filament_exact::Meter::new(system.meter_limits()).with_cancel(cancel.clone());
    let scheduler = step::scheduler(
        &evidence.scheduler,
        limits,
        &mut meter,
        &mut storage,
        cancel,
    )?;
    let mut transition_count = 0;
    let initials = system.initial();
    let [initial] = initials.as_slice() else {
        return Err(ExactProbabilityGraphError::UnsupportedInitial.into());
    };
    if key_of(system, reference, initial, limits, cancel)?
        != key_of(system, reference, &evidence.initial, limits, cancel)?
        || initial.anchor() != evidence.initial.anchor()
        || initial.reached_by() != evidence.initial.reached_by()
    {
        return Err(ProbabilityWitnessError::Refused);
    }
    let mut state = initial.clone();
    let mut mass = rational(1);
    let mut seen = BTreeSet::new();
    let mut used = BTreeSet::new();
    for position in 0..=path.steps.len() {
        bounded::visit(&mut meter, cancel)?;
        if let (Some(monitor), Some(control)) = (monitor.as_ref(), control) {
            if !matches!(monitor.state(control).map_err(bounded::monitor_error)?, BoundedState::Open { remaining } if remaining > 0)
            {
                return Ok(ProbabilityWitnessReplay::Parity);
            }
        }
        let (raw, state_key) = system
            .state_key(&state)
            .map_err(|_| ExactProbabilityGraphError::ExpansionRefused)?;
        let pre_key = match monitor.as_ref() {
            Some(monitor) => {
                let key = match control {
                    Some(control) => monitor.key(control),
                    None => monitor.unread_key(),
                }
                .map_err(bounded::monitor_error)?;
                ProbabilityProductKey::for_formula_state(
                    reference,
                    &raw,
                    &key,
                    limits.document,
                    cancel,
                )
                .map_err(bounded::document_error)?
            }
            None => key_of(system, reference, &state, limits, cancel)?,
        };
        let predicates: Vec<(
            &fsl_semantics::check::CheckedPredicate,
            crate::UndefinedLocus,
        )> = match &claim.form {
            ProbForm::Probability {
                event: Event::Formula { formula, .. },
                ..
            } => {
                let atom = monitor
                    .as_ref()
                    .ok_or(ProbabilityWitnessError::Refused)?
                    .atom();
                vec![(
                    formula
                        .condition(atom.index())
                        .ok_or(ProbabilityWitnessError::Refused)?,
                    crate::UndefinedLocus::Atom(atom),
                )]
            }
            ProbForm::Reach {
                target: ReachTarget::Eventually(p) | ReachTarget::Always(p),
                ..
            } => vec![(
                p,
                crate::UndefinedLocus::ClaimPredicate(crate::ClaimPredicate::Target),
            )],
            ProbForm::Reach {
                target: ReachTarget::Until(a, b),
                ..
            } => vec![
                (
                    a,
                    crate::UndefinedLocus::ClaimPredicate(crate::ClaimPredicate::From),
                ),
                (
                    b,
                    crate::UndefinedLocus::ClaimPredicate(crate::ClaimPredicate::Until),
                ),
            ],
            _ => return Err(ExactProbabilityGraphError::UnsupportedClaimForm.into()),
        };
        let inputs = predicates
            .iter()
            .map(|(p, _)| (*p, reference))
            .collect::<Vec<_>>();
        let readings = system
            .state_predicates_with_cancel(&state, &inputs, cancel)
            .map_err(|stop| cancelled_expansion(stop, cancel))?;
        poll_cancel(cancel)?;
        let mut truths = Vec::new();
        for ((_, locus), reading) in predicates.iter().zip(readings) {
            match reading {
                PredicateReading::True => truths.push(true),
                PredicateReading::False => truths.push(false),
                PredicateReading::Refused(outcome) => {
                    return Err(ProbabilityWitnessError::Build(Box::new(
                        crate::ExactProbBuildIssue::Predicate(outcome),
                    )))
                }
                PredicateReading::Undefined { cause, .. } => {
                    if !seen.contains(&pre_key) && seen.len() >= limits.model.limits.max_states {
                        return Err(ExactProbabilityGraphError::LimitReached.into());
                    }
                    let evaluation = crate::UndefinedEvaluation {
                        site: crate::UndefinedSite {
                            position: u64::try_from(position).unwrap_or(u64::MAX),
                            state_key,
                            locus: locus.clone(),
                            transition: None,
                            time: None,
                        },
                        cause,
                    };
                    let offered_key = if position == 0 {
                        &evidence.initial_key
                    } else {
                        &path.steps[position - 1].post_key
                    };
                    if position != path.steps.len()
                        || evaluation != witness.evaluation
                        || pre_key != witness.product_state
                        || offered_key != &pre_key
                        || scheduler.keys().any(|key| !used.contains(*key))
                    {
                        return Ok(ProbabilityWitnessReplay::Parity);
                    }
                    bounded::retain(pre_key.as_bytes().len(), &mut storage, limits)?;
                    check_bits(&mass, limits.max_rational_bits)?;
                    let token = ReproducedProbabilityWitness {
                        evidence: evidence.clone(),
                        mass,
                        undefined: Some(Box::new(witness.clone())),
                    };
                    poll_cancel(cancel)?;
                    return Ok(ProbabilityWitnessReplay::Reproduced(Box::new(token)));
                }
            }
        }
        let (key, decided) = match monitor.as_mut() {
            Some(monitor) => {
                let next = match control {
                    Some(control) => monitor.step(control, truths[0]),
                    None => monitor.start(truths[0]),
                }
                .map_err(bounded::monitor_error)?;
                control = Some(next);
                let key = monitor.key(next).map_err(bounded::monitor_error)?;
                (
                    ProbabilityProductKey::for_formula_state(
                        reference,
                        &raw,
                        &key,
                        limits.document,
                        cancel,
                    )
                    .map_err(bounded::document_error)?,
                    !matches!(
                        monitor.state(next).map_err(bounded::monitor_error)?,
                        BoundedState::Open { .. }
                    ),
                )
            }
            None => {
                let decided = match &claim.form {
                    ProbForm::Reach {
                        target: ReachTarget::Eventually(_),
                        ..
                    } => truths[0],
                    ProbForm::Reach {
                        target: ReachTarget::Always(_),
                        ..
                    } => !truths[0],
                    ProbForm::Reach {
                        target: ReachTarget::Until(..),
                        ..
                    } => truths[1] || !truths[0],
                    _ => return Err(ExactProbabilityGraphError::UnsupportedClaimForm.into()),
                };
                (pre_key, decided)
            }
        };
        let offered_key = if position == 0 {
            &evidence.initial_key
        } else {
            &path.steps[position - 1].post_key
        };
        if &key != offered_key || decided {
            return Ok(ProbabilityWitnessReplay::Parity);
        }
        if !seen.insert(key.clone()) && workload.is_none() {
            return Ok(ProbabilityWitnessReplay::Parity);
        }
        if seen.len() > limits.model.limits.max_states {
            return Err(ExactProbabilityGraphError::LimitReached.into());
        }
        bounded::retain(key.as_bytes().len(), &mut storage, limits)?;
        let Some(step) = path.steps.get(position) else {
            return Ok(ProbabilityWitnessReplay::Parity);
        };
        if position >= limits.model.limits.max_transitions {
            return Err(ExactProbabilityGraphError::EdgeLimitReached.into());
        }
        if workload.is_none() && scheduler.get(&key).copied() != Some(&step.choice) {
            return Err(ProbabilityWitnessError::Refused);
        }
        used.insert(key);
        let expected = key_of(system, reference, &step.post, limits, cancel)?;
        let (probability, post, ()) = step::replay(
            system,
            &state,
            workload,
            step,
            limits,
            &mut meter,
            &mut storage,
            &mut transition_count,
            cancel,
            step::ActionRefusal::UnsupportedDraw,
            |post, _, _, _| {
                Ok((key_of(system, reference, post, limits, cancel)? == expected).then_some(()))
            },
        )?;
        mass = bounded::arithmetic(
            filament_exact::RationalArithmetic::Multiply(&mass, &probability),
            &mut meter,
            cancel,
        )?;
        check_bits(&mass, limits.max_rational_bits)?;
        state = post;
    }
    Ok(ProbabilityWitnessReplay::Parity)
}
