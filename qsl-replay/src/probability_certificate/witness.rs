// SPDX-License-Identifier: AGPL-3.0-or-later
//! Independent checked-subject replay of finite probabilistic witnesses.

use super::*;
use crate::{ExactProbLimits, ModelCheckLimit};
use qsl_eval::simulation::model::{
    same_values, PredicateReading, ScheduledIdentity, TimedSubjectRefusal, TimedSystem,
    TransitionId,
};
use qsl_package::CheckedPackage;
use quire_exact::Value as SemanticValue;
use quire_semantic_value::model_transition::ModelTransition;

mod bounded;

/// A semantic operation, draw and resulting state in a finite witness.
#[derive(Clone, Debug)]
pub struct ProbabilityWitnessStep {
    /// The scheduled operation, receiver and non-random arguments.
    pub action: ScheduledIdentity,
    /// The actual resolved transition, including its random arguments.
    pub transition: ModelTransition,
    /// The random argument vector in declaration order.
    pub draw: Vec<SemanticValue>,
    /// The actual post-state.
    pub post: ModelSimState,
    /// The complete canonical post-product key.
    pub post_key: ProbabilityProductKey,
    /// Elapsed time; this untimed entry requires zero.
    pub elapsed: Rational,
}

/// One finite path from the witness's initial state.
#[derive(Clone, Debug)]
pub struct ProbabilityWitnessPath {
    /// The semantic steps in execution order.
    pub steps: Vec<ProbabilityWitnessStep>,
}

/// Typed path-set evidence for one exact checked claim and binding.
#[derive(Clone, Debug)]
pub struct ProbabilisticCounterexample {
    /// The checked claim occurrence, including its owning package identity.
    pub item: ObligationIdentity,
    /// The claim's object binding, absent for an unbound claim.
    pub binding: Option<ObjectReference>,
    /// Actual initial state content.
    pub initial: ModelSimState,
    /// Complete initial product key.
    pub initial_key: ProbabilityProductKey,
    /// Deterministic scheduler entries in full-key order; empty for a workload.
    pub scheduler: Vec<(ProbabilityProductKey, ScheduledIdentity)>,
    /// Prefix-free finite paths.
    pub paths: Vec<ProbabilityWitnessPath>,
}

/// A refutation reproduced by the qualified core. No caller can construct it.
///
/// ```compile_fail,E0451
/// use qsl_replay::{ReproducedProbabilityWitness, ProbabilisticCounterexample};
/// use quire_exact::Rational;
/// fn forge(evidence: ProbabilisticCounterexample, mass: Rational) {
///     let _ = ReproducedProbabilityWitness { evidence, mass };
/// }
/// ```
#[derive(Clone, Debug)]
pub struct ReproducedProbabilityWitness {
    evidence: ProbabilisticCounterexample,
    mass: Rational,
}

impl ReproducedProbabilityWitness {
    /// The exact independently recomputed path-set mass.
    pub fn mass(&self) -> &Rational {
        &self.mass
    }
    /// The evidence bound to this reproduction.
    pub fn evidence(&self) -> &ProbabilisticCounterexample {
        &self.evidence
    }
}

/// A completed replay either reproduces a refutation or disagrees with it.
#[derive(Clone, Debug)]
pub enum ProbabilityWitnessReplay {
    /// All semantic and threshold checks passed.
    Reproduced(Box<ReproducedProbabilityWitness>),
    /// The path set does not establish a strict refutation.
    Parity,
}

/// A replay could not complete; none of these outcomes contains a token.
#[derive(Clone, Debug)]
pub enum ProbabilityWitnessError {
    /// A subject expansion stopped with its original accounting or failure cause.
    Build(Box<crate::ExactProbBuildIssue>),
    /// Model admission, graph enumeration, unsupported shape or cancellation.
    Graph(ExactProbabilityGraphError),
    /// The witness names an invalid semantic choice, state or claim binding.
    Refused,
    /// A witness storage or arithmetic budget stopped replay.
    Limit {
        /// The exhausted caller limit.
        limit: ModelCheckLimit,
        /// Its configured bound.
        value: u64,
    },
}

impl From<ExactProbabilityGraphError> for ProbabilityWitnessError {
    fn from(value: ExactProbabilityGraphError) -> Self {
        Self::Graph(value)
    }
}

/// Replay a typed path set against the exact package and admitted system.
/// The claim, binding, predicates, probabilities and terminal leaves are all
/// independently derived. This entry accepts no analyzer graph or wire data.
/// It admits acyclic unbounded reachability, direct infinite-trace `until`
/// and `always` state events, and origin event-position
/// `eventually[0,h] holds(P)`. The latter uses qualified monitor keys and
/// terminal false extension and permits raw model cycles within its horizon.
#[allow(clippy::too_many_arguments)]
pub fn replay_probabilistic_witness(
    package: &CheckedPackage,
    system: &ModelSystem<'_>,
    item: &ObligationIdentity,
    witness: &ProbabilisticCounterexample,
    limits: &ExactProbLimits,
    cancel: &Cancel,
) -> Result<ProbabilityWitnessReplay, ProbabilityWitnessError> {
    poll_cancel(cancel)?;
    if !std::ptr::eq(package, system.package()) || item != &witness.item {
        return Err(ProbabilityWitnessError::Refused);
    }
    let claim = bind_claim(package.graph().probabilistic_model().claims(), item, cancel).map_err(
        |error| match error {
            ExactProbabilityGraphError::ClaimNotBound => ProbabilityWitnessError::Refused,
            _ => ProbabilityWitnessError::Graph(error),
        },
    )?;
    if matches!(claim.form, ProbForm::Probability { .. }) {
        return bounded::replay(system, claim, witness, limits, cancel);
    }
    let ProbForm::Reach { bound, target } = &claim.form else {
        return Err(ExactProbabilityGraphError::UnsupportedClaimForm.into());
    };
    if !matches!(target, ReachTarget::Eventually(_))
        && qsl_semantics::check::TemporalProfile::from_identity(claim.profile.identity())
            != Some(qsl_semantics::check::TemporalProfile::InfiniteTrace)
    {
        return Err(ExactProbabilityGraphError::UnsupportedClaimForm.into());
    }
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
    let mut model_states = BTreeMap::new();
    let mut storage = 0;
    let mut meter = quire_exact::Meter::new(system.meter_limits()).with_cancel(cancel.clone());
    let key_error = std::cell::RefCell::new(None);
    let mut issue = None;
    let graph = enumerate_model_graph_with_keys(
        system,
        None::<&qsl_semantics::check::CheckedPredicate>,
        reference,
        None,
        limits.model.limits.max_states,
        limits.model.limits.max_transitions,
        cancel,
        false,
        &|state| {
            key_of(system, reference, state, limits, cancel).map_err(|error| {
                *key_error.borrow_mut() = Some(error);
                ExactProbabilityGraphError::ExpansionRefused
            })
        },
        &mut |key, state| {
            bounded::visit(&mut meter, cancel).map_err(|error| {
                *key_error.borrow_mut() = Some(error);
                ExactProbabilityGraphError::ExpansionRefused
            })?;
            bounded::retain(key.as_bytes().len().saturating_mul(2), &mut storage, limits).map_err(
                |error| {
                    *key_error.borrow_mut() = Some(error);
                    ExactProbabilityGraphError::ExpansionRefused
                },
            )?;
            model_states.insert(key.clone(), state.clone());
            Ok(())
        },
        &mut |failure| {
            issue = Some(failure);
        },
    )
    .map_err(|error| {
        if let Some(error) = key_error.borrow_mut().take() {
            return error;
        }
        issue.map_or(ProbabilityWitnessError::Graph(error), |issue| {
            ProbabilityWitnessError::Build(Box::new(issue))
        })
    })?;
    topological_order(&graph, cancel)?;
    let initials = system.initial();
    let [initial] = initials.as_slice() else {
        return Err(ExactProbabilityGraphError::UnsupportedInitial.into());
    };
    if key_of(system, reference, initial, limits, cancel)? != witness.initial_key
        || key_of(system, reference, &witness.initial, limits, cancel)? != witness.initial_key
        || initial.anchor() != witness.initial.anchor()
        || initial.reached_by() != witness.initial.reached_by()
    {
        return Err(ProbabilityWitnessError::Refused);
    }
    if u64::try_from(witness.paths.len()).unwrap_or(u64::MAX) > limits.max_witness_paths {
        return Err(ProbabilityWitnessError::Limit {
            limit: ModelCheckLimit::ExactProbMaxWitnessPaths,
            value: limits.max_witness_paths,
        });
    }
    if witness.scheduler.len() > limits.model.limits.max_states {
        return Err(ExactProbabilityGraphError::LimitReached.into());
    }
    let workload = match &claim.scheduler {
        ProbScheduler::Every => None,
        ProbScheduler::Workload(name) => Some(
            package
                .graph()
                .probabilistic_model()
                .workload(name)
                .ok_or(ProbabilityWitnessError::Refused)?,
        ),
    };
    if workload.is_some() && !witness.scheduler.is_empty() {
        return Err(ProbabilityWitnessError::Refused);
    }
    let mut scheduler = BTreeMap::new();
    let mut previous = None;
    for (key, action) in &witness.scheduler {
        poll_cancel(cancel)?;
        if previous.is_some_and(|previous| previous >= key) || !graph.states.contains_key(key) {
            return Err(ProbabilityWitnessError::Refused);
        }
        previous = Some(key);
        let state = model_states
            .get(key)
            .ok_or(ProbabilityWitnessError::Refused)?;
        let enabled = system
            .actions(state)
            .map_err(expansion)?
            .map_err(|_| ExactProbabilityGraphError::ExpansionRefused)?;
        if !enabled.iter().any(|enabled| &enabled.identity == action) {
            return Err(ProbabilityWitnessError::Refused);
        }
        let action_bytes = system.scheduled_identity_key(action).map_err(expansion)?;
        bounded::retain(
            key.as_bytes().len().saturating_add(action_bytes.len()),
            &mut storage,
            limits,
        )?;
        scheduler.insert(key, action);
    }
    let mut transitions = 0usize;
    let mut mass = rational(0);
    let mut replayed: Vec<&ProbabilityWitnessPath> = Vec::new();
    for path in &witness.paths {
        poll_cancel(cancel)?;
        let mut state = initial.clone();
        let mut probability = rational(1);

        for step in &path.steps {
            poll_cancel(cancel)?;
            transitions = transitions
                .checked_add(1)
                .ok_or(ExactProbabilityGraphError::TooManyEdges)?;
            if transitions > limits.model.limits.max_transitions {
                return Err(ExactProbabilityGraphError::EdgeLimitReached.into());
            }
            let key = key_of(system, reference, &state, limits, cancel)?;
            let terminal = graph
                .states
                .get(&key)
                .ok_or(ProbabilityWitnessError::Refused)?
                .choices
                .is_empty();
            if outcome(system, target, reference, &state, terminal)?.is_some() {
                return Ok(ProbabilityWitnessReplay::Parity);
            }
            if step.elapsed != rational(0)
                || workload.is_none() && scheduler.get(&key).copied() != Some(&step.action)
            {
                return Err(ProbabilityWitnessError::Refused);
            }
            let mut found = None;
            if let Some(workload) = workload {
                for candidate in system
                    .weighted_steps(&state, workload)
                    .map_err(expansion)?
                    .map_err(|_| ExactProbabilityGraphError::UnsupportedDraw)?
                {
                    poll_cancel(cancel)?;
                    if candidate.identity.as_ref() == Some(&step.action)
                        && candidate.transition == TransitionId::Operation(step.transition.clone())
                        && same_values(&candidate.random, &step.draw)
                        && key_of(system, reference, &candidate.post, limits, cancel)?
                            == step.post_key
                    {
                        found = Some((candidate.probability, candidate.post));
                        break;
                    }
                }
            } else {
                for action in system
                    .actions(&state)
                    .map_err(expansion)?
                    .map_err(|_| ExactProbabilityGraphError::ExpansionRefused)?
                {
                    poll_cancel(cancel)?;
                    if action.identity != step.action {
                        continue;
                    }
                    for draw in action.draws {
                        poll_cancel(cancel)?;
                        if !same_values(&draw.random, &step.draw) {
                            continue;
                        }
                        if draw.successors.len() != 1 {
                            return Err(ExactProbabilityGraphError::UnsupportedDraw.into());
                        }
                        for (transition, post) in draw.successors {
                            if transition == step.transition
                                && key_of(system, reference, &post, limits, cancel)?
                                    == step.post_key
                            {
                                found = Some((draw.probability.clone(), post));
                            }
                        }
                    }
                }
            }
            let Some((step_mass, post)) = found else {
                return Err(ProbabilityWitnessError::Refused);
            };
            if step_mass <= rational(0)
                || key_of(system, reference, &step.post, limits, cancel)? != step.post_key
                || post.anchor() != step.post.anchor()
                || post.reached_by() != step.post.reached_by()
            {
                return Err(ProbabilityWitnessError::Refused);
            }
            check_bits(&step_mass, limits.max_rational_bits)?;
            probability = bounded::arithmetic(
                quire_exact::RationalArithmetic::Multiply(&probability, &step_mass),
                &mut meter,
                cancel,
            )?;
            check_bits(&probability, limits.max_rational_bits)?;

            state = post;
        }
        let key = key_of(system, reference, &state, limits, cancel)?;
        let terminal = graph
            .states
            .get(&key)
            .ok_or(ProbabilityWitnessError::Refused)?
            .choices
            .is_empty();
        let accepting = matches!(
            (bound, outcome(system, target, reference, &state, terminal)?),
            (Bound::AtMost(_), Some(true)) | (Bound::AtLeast(_), Some(false))
        );

        if !accepting {
            return Ok(ProbabilityWitnessReplay::Parity);
        }
        for previous in &replayed {
            poll_cancel(cancel)?;
            let mut prefix = true;
            for (a, b) in previous.steps.iter().zip(&path.steps) {
                bounded::visit(&mut meter, cancel)?;
                if a.action != b.action
                    || a.transition != b.transition
                    || a.post_key != b.post_key
                    || !same_values(&a.draw, &b.draw)
                {
                    prefix = false;
                    break;
                }
            }
            if prefix {
                return Ok(ProbabilityWitnessReplay::Parity);
            }
        }
        bounded::retain(
            std::mem::size_of::<&ProbabilityWitnessPath>(),
            &mut storage,
            limits,
        )?;
        replayed
            .try_reserve(1)
            .map_err(|_| ProbabilityWitnessError::Limit {
                limit: ModelCheckLimit::CertificateMaxWorkingBytes,
                value: limits.document.storage_bytes,
            })?;
        replayed.push(path);
        mass = bounded::arithmetic(
            quire_exact::RationalArithmetic::Add(&mass, &probability),
            &mut meter,
            cancel,
        )?;
        check_bits(&mass, limits.max_rational_bits)?;
    }
    let threshold = match bound {
        Bound::AtMost(threshold) => threshold.clone(),
        Bound::AtLeast(threshold) => bounded::arithmetic(
            quire_exact::RationalArithmetic::Subtract(&rational(1), threshold),
            &mut meter,
            cancel,
        )?,
    };
    if mass <= threshold {
        return Ok(ProbabilityWitnessReplay::Parity);
    }
    let evidence = witness.clone();
    poll_cancel(cancel)?;
    Ok(ProbabilityWitnessReplay::Reproduced(Box::new(
        ReproducedProbabilityWitness { evidence, mass },
    )))
}

/// First decisive state under infinite stutter at a genuine terminal state.
fn outcome(
    system: &ModelSystem<'_>,
    target: &ReachTarget,
    binding: Option<&ObjectReference>,
    state: &ModelSimState,
    terminal: bool,
) -> Result<Option<bool>, ProbabilityWitnessError> {
    if let ReachTarget::Eventually(predicate) = target {
        if observes(system, predicate, binding, state)? {
            return Ok(Some(true));
        }
    }
    match target {
        ReachTarget::Eventually(_) => Ok(terminal.then_some(false)),
        ReachTarget::Until(..) | ReachTarget::Always(_) => {
            state_predicate::outcome(system, target, binding, state, terminal)
                .map(|(verdict, _)| verdict)
                .map_err(bounded::document_error)
        }
    }
}

fn key_of(
    system: &ModelSystem<'_>,
    binding: Option<&ObjectReference>,
    state: &ModelSimState,
    limits: &ExactProbLimits,
    cancel: &Cancel,
) -> Result<ProbabilityProductKey, ProbabilityWitnessError> {
    let (bytes, _) = system
        .state_key(state)
        .map_err(|_| ExactProbabilityGraphError::ExpansionRefused)?;
    ProbabilityProductKey::for_model_state_with_limits(binding, &bytes, limits.document, cancel)
        .map_err(bounded::document_error)
}
fn observes(
    system: &ModelSystem<'_>,
    predicate: &impl StatePredicate,
    binding: Option<&ObjectReference>,
    state: &ModelSimState,
) -> Result<bool, ProbabilityWitnessError> {
    match system
        .state_predicates(state, &[(predicate, binding)])
        .map_err(expansion)?
        .first()
    {
        Some(PredicateReading::True) => Ok(true),
        Some(PredicateReading::False) => Ok(false),
        Some(PredicateReading::Refused(outcome)) => Err(ProbabilityWitnessError::Build(Box::new(
            crate::ExactProbBuildIssue::Predicate(outcome.clone()),
        ))),
        _ => Err(ExactProbabilityGraphError::ExpansionRefused.into()),
    }
}
fn expansion(stop: qsl_eval::simulation::ExpansionStop) -> ProbabilityWitnessError {
    ProbabilityWitnessError::Build(Box::new(crate::ExactProbBuildIssue::Expansion(stop)))
}
fn check_bits(value: &Rational, max: u64) -> Result<(), ProbabilityWitnessError> {
    if value.numerator().as_big().bits() > max || value.denominator().as_big().bits() > max {
        Err(ProbabilityWitnessError::Limit {
            limit: ModelCheckLimit::ExactProbMaxRationalBits,
            value: max,
        })
    } else {
        Ok(())
    }
}
