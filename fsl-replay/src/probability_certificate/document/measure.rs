// SPDX-License-Identifier: AGPL-3.0-or-later
//! Finite-horizon measure proofs, independently reconstructed from checked input.

use super::*;
use filament_exact::OrderingOperator as Order;
use fsl_eval::probabilistic::{
    Accumulator, MeasureComparison, MeasurePayoff, MeasurePlan, MonitorState,
};
use fsl_eval::simulation::model::{ModelKeyBudget, ModelKeyError, PredicateReading};
use fsl_semantics::check::{BoundedMeasure, CheckedPredicate, Event, Weighting};
use quire_canonical::{Encode, Error, Node, Sink, Writer};
use std::cell::RefCell;

pub(super) const PROFILE: &str = "filament.probability-objective/finite-horizon-measure/v1";
pub(in crate::probability_certificate) mod bounds;

/// Terminal-payoff role of a finite-horizon measure proof.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MeasurePayoffKind {
    /// A probability comparison event.
    Event,
    /// A quantile's satisfied and activated numerator.
    Satisfied,
    /// A quantile's activation denominator.
    Activated,
    /// An every-scheduler quantile's signed transform.
    Shifted,
    /// The mean of a completed fraction.
    Fraction,
}
impl MeasurePayoffKind {
    fn text(self) -> &'static str {
        match self {
            Self::Event => "event",
            Self::Satisfied => "satisfied",
            Self::Activated => "activated",
            Self::Shifted => "shifted",
            Self::Fraction => "fraction",
        }
    }
    fn read(text: &str) -> Result<Self, ProbabilityDocumentError> {
        match text {
            "event" => Ok(Self::Event),
            "satisfied" => Ok(Self::Satisfied),
            "activated" => Ok(Self::Activated),
            "shifted" => Ok(Self::Shifted),
            "fraction" => Ok(Self::Fraction),
            _ => Err(ProbabilityDocumentError::ObjectiveMismatch),
        }
    }
}

/// Unchecked finite-horizon measure evidence. Construction supplies no acceptance.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MeasureProbabilityDocument {
    /// Exact checked obligation identity.
    pub identity: ObligationIdentity,
    /// Terminal-payoff selection, independently checked against the claim.
    pub payoff: MeasurePayoffKind,
    /// Complete full-key values in canonical order.
    pub values: Vec<ExactProbabilityValue>,
    /// Remaining-horizon ranks in canonical order.
    pub ranking: Vec<(ProbabilityProductKey, u64)>,
    objective: Objective,
    conditional: Option<Box<Conditional>>,
}
#[derive(Clone, Debug, Eq, PartialEq)]
enum Conditional {
    Ratio(MeasureProbabilityDocument, MeasureProbabilityDocument),
    Vacuous,
}
#[derive(Clone, Debug, Eq, PartialEq)]
struct Objective {
    kind: &'static str,
    window: u64,
    reward: Option<ProbabilityRewardObjective>,
    comparison: Option<MeasureComparison>,
    threshold: Option<Rational>,
    level: Option<Rational>,
    optimization: ProbabilityObjective,
    fairness: Vec<fairness::Record>,
}

struct Plan<'a> {
    control: MeasurePlan<'a>,
    predicates: Vec<&'a CheckedPredicate>,
    objective: Objective,
    payoff: MeasurePayoff,
    bound: Bound,
    strict: bool,
}

pub(in crate::probability_certificate) fn ordered(
    operator: filament_exact::OrderingOperator,
    left: &Rational,
    right: &Rational,
    meter: &mut filament_exact::Meter,
    cancel: &Cancel,
) -> Result<bool, ProbabilityDocumentError> {
    poll(cancel)?;
    match filament_exact::order_numbers(
        operator,
        filament_exact::OrderedOperands::Rationals(left, right),
        meter,
    ) {
        filament_exact::Outcome::Completed(value) => Ok(value),
        filament_exact::Outcome::Incomplete(error) => match cancel.tripped() {
            Some(cause) => Err(ProbabilityDocumentError::Cancelled(cause)),
            None => Err(ProbabilityDocumentError::Accounting(Box::new(error))),
        },
        filament_exact::Outcome::Undefined(_) | filament_exact::Outcome::Refused(_) => {
            Err(ProbabilityDocumentError::Malformed)
        }
    }
}

fn plan<'a>(
    system: &ModelSystem<'_>,
    claim: &'a CheckedProbabilisticClaim,
    payoff: MeasurePayoffKind,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
    meter: &mut filament_exact::Meter,
    storage: &mut u64,
) -> Result<Plan<'a>, ProbabilityDocumentError> {
    use fsl_eval::simulation::model::{TimedSubjectRefusal, TimedSystem};
    if fsl_semantics::check::TemporalProfile::from_identity(claim.profile.identity())
        != Some(fsl_semantics::check::TemporalProfile::EventPositionFalseExtension)
        || !matches!(
            TimedSystem::new(system, &claim.subject.alias),
            Err(TimedSubjectRefusal::NoTimedModel { .. })
        )
    {
        return Err(ProbabilityDocumentError::Unsupported);
    }
    let (measure, fraction, bound, q, comparison) = match (&claim.form, payoff) {
        (
            ProbForm::Probability {
                event:
                    Event::Compare {
                        measure,
                        bound: comparison,
                    },
                bound,
            },
            MeasurePayoffKind::Event,
        ) => (Some(measure), None, bound.clone(), None, Some(comparison)),
        (
            ProbForm::Quantile {
                measure,
                threshold,
                q,
            },
            MeasurePayoffKind::Satisfied
            | MeasurePayoffKind::Activated
            | MeasurePayoffKind::Shifted,
        ) => (
            Some(measure),
            None,
            Bound::AtLeast(q.clone()),
            Some(q),
            Some(threshold),
        ),
        (ProbForm::MeanOfFraction { fraction, bound }, MeasurePayoffKind::Fraction) => {
            (None, Some(fraction), bound.clone(), None, None)
        }
        _ => return Err(ProbabilityDocumentError::ObjectiveMismatch),
    };
    let strict = matches!(
        &claim.form,
        ProbForm::Quantile {
            threshold: Bound::AtLeast(_),
            ..
        }
    );
    let comparison_kind = comparison.map(|bound| match bound {
        Bound::AtLeast(_) if strict => MeasureComparison::Lt,
        Bound::AtLeast(_) => MeasureComparison::Ge,
        Bound::AtMost(_) => MeasureComparison::Le,
    });
    let threshold = comparison.map(|bound| match bound {
        Bound::AtLeast(value) | Bound::AtMost(value) => value.clone(),
    });
    let weight = |weight: &'a Option<Weighting>| match weight {
        None => Ok(None),
        Some(Weighting::Reward(name)) => Ok(Some(name)),
        Some(Weighting::Delay) => Err(ProbabilityDocumentError::Unsupported),
    };
    let (control, predicates, kind, window, reward_name) = match (measure, fraction) {
        (
            Some(BoundedMeasure::Steps {
                from,
                until,
                within,
                ..
            }),
            _,
        ) => (
            MeasurePlan::Measure {
                reward: None,
                threshold: threshold
                    .clone()
                    .ok_or(ProbabilityDocumentError::Malformed)?,
                within: *within,
            },
            vec![from.as_ref(), until.as_ref()],
            "steps",
            *within,
            None,
        ),
        (
            Some(BoundedMeasure::Accumulate {
                reward,
                from,
                until,
                within,
                ..
            }),
            _,
        ) => (
            MeasurePlan::Measure {
                reward: Some(reward),
                threshold: threshold
                    .clone()
                    .ok_or(ProbabilityDocumentError::Malformed)?,
                within: *within,
            },
            vec![from.as_ref(), until.as_ref()],
            "accumulate",
            *within,
            Some(reward),
        ),
        (Some(BoundedMeasure::Fraction(fraction)), _) | (None, Some(fraction)) => {
            let weight = weight(&fraction.weight)?;
            (
                MeasurePlan::Fraction {
                    window: fraction.window,
                    weight,
                },
                vec![&fraction.predicate],
                if weight.is_some() {
                    "weighted-fraction"
                } else {
                    "fraction"
                },
                fraction.window,
                weight,
            )
        }
        _ => return Err(ProbabilityDocumentError::Unsupported),
    };
    let reward = reward_name
        .map(|name| super::super::reward::objective(system, claim, name, limits, cancel, storage))
        .transpose()?;
    let optimization = match claim.scheduler {
        ProbScheduler::Workload(_) => ProbabilityObjective::Workload,
        ProbScheduler::Every => match (&claim.form, &bound) {
            (ProbForm::Quantile { .. }, _) if strict => ProbabilityObjective::Maximum,
            (_, Bound::AtLeast(_)) => ProbabilityObjective::Minimum,
            (_, Bound::AtMost(_)) => ProbabilityObjective::Maximum,
        },
    };
    let fairness = if claim.fairness.is_empty() {
        Vec::new()
    } else {
        fairness::checked(system, claim, limits, cancel, meter, storage)?
    };
    let comparison = if matches!(
        payoff,
        MeasurePayoffKind::Activated | MeasurePayoffKind::Fraction
    ) {
        None
    } else {
        comparison_kind
    };
    let objective = Objective {
        kind,
        window,
        reward,
        comparison,
        threshold: comparison.and(threshold.clone()),
        level: if payoff == MeasurePayoffKind::Shifted {
            q.cloned()
        } else {
            None
        },
        optimization,
        fairness,
    };
    let compared = || -> Result<_, ProbabilityDocumentError> {
        Ok((
            comparison_kind.ok_or(ProbabilityDocumentError::Malformed)?,
            threshold
                .clone()
                .ok_or(ProbabilityDocumentError::Malformed)?,
        ))
    };
    let terminal = match payoff {
        MeasurePayoffKind::Event => {
            let (op, value) = compared()?;
            MeasurePayoff::Event(op, value)
        }
        MeasurePayoffKind::Satisfied => {
            let (op, value) = compared()?;
            MeasurePayoff::Satisfied(op, value)
        }
        MeasurePayoffKind::Activated => MeasurePayoff::Activated,
        MeasurePayoffKind::Shifted => {
            let (op, value) = compared()?;
            MeasurePayoff::Shifted(
                op,
                value,
                q.cloned().ok_or(ProbabilityDocumentError::Malformed)?,
            )
        }
        MeasurePayoffKind::Fraction => MeasurePayoff::Fraction,
    };
    Ok(Plan {
        control,
        predicates,
        objective,
        payoff: terminal,
        bound,
        strict,
    })
}

/// Construct the complete shared-owner measure key under the actual caller.
/// No model, monitor, amount or binding component is projected away.
#[allow(clippy::too_many_arguments)]
pub fn measure_probability_product_key(
    binding: Option<&ObjectReference>,
    raw: &[u8],
    control: MonitorState,
    accumulator: &Accumulator,
    pending: Option<&[u8]>,
    reward: Option<&ProbabilityRewardObjective>,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
    meter: &mut filament_exact::Meter,
    storage: &mut u64,
) -> Result<ProbabilityProductKey, ProbabilityDocumentError> {
    poll(cancel)?;
    let mut admission = ProbabilityKeyAdmission::new(meter, storage, limits.storage_bytes, cancel);
    let scope = RefCell::new(&mut admission as &mut dyn ModelKeyBudget);
    struct Reserve<'a, 'b>(&'a RefCell<&'b mut dyn ModelKeyBudget>);
    impl quire_canonical::Reservation for Reserve<'_, '_> {
        fn reserve(&mut self, bytes: usize) -> Result<(), quire_canonical::ReservationDenied> {
            self.0
                .borrow_mut()
                .reserve(bytes)
                .map_err(|_| quire_canonical::ReservationDenied)
        }
    }
    let identity = fsl_foundation::IdentityLimits::default();
    let result = (|| {
        scope
            .borrow_mut()
            .work(u64::try_from(raw.len()).unwrap_or(u64::MAX))
            .map_err(|_| ModelKeyError::Denied)?;
        let model =
            quire_canonical::read_with_reservation(raw, identity.input_bytes, &mut Reserve(&scope))
                .map_err(|error| key_parse_error(error, identity))?;
        if !key::model_form(model.root()) {
            return Err(ModelKeyError::Unsupported("canonical model body"));
        }
        let original = writer::encode_admitted(&model, identity.input_bytes, &mut Reserve(&scope))
            .map_err(|_| ModelKeyError::Denied)?;
        if original != raw {
            return Err(ModelKeyError::Unsupported("canonical model body"));
        }
        let pending = pending
            .map(|bytes| {
                quire_canonical::read_with_reservation(
                    bytes,
                    identity.input_bytes,
                    &mut Reserve(&scope),
                )
                .map_err(|error| key_parse_error(error, identity))
            })
            .transpose()?;
        let key = fsl_eval::probabilistic::key::MeasureProductKey {
            binding,
            model: model.root(),
            control,
            accumulator,
            pending: pending.as_ref().map(|pending| pending.root()),
            reward: reward.map(|reward| fsl_eval::probabilistic::key::RewardKey {
                model: &reward.model,
                name: &reward.name,
                unit: reward.unit.as_ref(),
            }),
            budget: &scope,
            max_bytes: identity.input_bytes,
        };
        key.validate().map_err(ModelKeyError::Unsupported)?;
        writer::encode_admitted(&key, identity.input_bytes, &mut Reserve(&scope))
            .map(ProbabilityProductKey::from_canonical_bytes)
            .map_err(|error| {
                ModelKeyError::Encoding(fsl_eval::simulation::EncodingRefusal::of(&error, identity))
            })
    })();
    drop(scope);
    result.map_err(|error| admission.refusal(error))
}

type Resolution = (
    Vec<ModelSimState>,
    Vec<(filament_exact::Identifier, filament_exact::Value)>,
);

enum Successor {
    Position(
        Rational,
        ModelSimState,
        Vec<(filament_exact::Identifier, filament_exact::Value)>,
    ),
    Draw(
        Rational,
        fsl_eval::simulation::model::ScheduledIdentity,
        Vec<filament_exact::Value>,
        Vec<ModelSimState>,
        Vec<(filament_exact::Identifier, filament_exact::Value)>,
    ),
}
struct State {
    terminal: Option<Rational>,
    rank: u64,
    choices: Vec<Vec<ProbabilityEdge>>,
    actions: Vec<Option<crate::ProductAction>>,
    activated: bool,
}
struct Graph {
    initial: ProbabilityProductKey,
    initials: Vec<(u32, ProbabilityProductKey)>,
    states: BTreeMap<ProbabilityProductKey, State>,
}

#[allow(clippy::too_many_arguments)]
fn enumerate(
    system: &ModelSystem<'_>,
    claim: &CheckedProbabilisticClaim,
    plan: &Plan<'_>,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
    meter: &mut filament_exact::Meter,
    storage: &mut u64,
    named: Option<&BTreeSet<ProbabilityProductKey>>,
) -> Result<Graph, ProbabilityDocumentError> {
    let binding = claim_binding(system, claim)?;
    let binding = binding.as_ref().map(|binding| &binding.reference);
    let workload = claim_workload(system, claim)?;
    if limits.states == 0 {
        return Err(exceeded(ProbabilityDocumentLimit::States, 0, 1));
    }
    let initial = system.initial();
    if initial.is_empty() {
        return Err(ProbabilityDocumentError::Malformed);
    }
    let read = |state: &ModelSimState| -> Result<Vec<bool>, ProbabilityDocumentError> {
        let predicates = plan
            .predicates
            .iter()
            .map(|predicate| (*predicate, binding))
            .collect::<Vec<_>>();
        system
            .state_predicates_with_cancel(state, &predicates, cancel)
            .map_err(|error| match cancel.tripped() {
                Some(cause) => ProbabilityDocumentError::Cancelled(cause),
                None => ProbabilityDocumentError::Build(Box::new(
                    crate::ExactProbBuildIssue::Expansion(error),
                )),
            })?
            .into_iter()
            .map(|reading| match reading {
                PredicateReading::True => Ok(true),
                PredicateReading::False => Ok(false),
                PredicateReading::Undefined { cause, .. } => {
                    Err(ProbabilityDocumentError::UndefinedPredicate(cause))
                }
                PredicateReading::Refused(outcome) => Err(ProbabilityDocumentError::Build(
                    Box::new(crate::ExactProbBuildIssue::Predicate(outcome)),
                )),
            })
            .collect()
    };
    let key_of = |state: &ModelSimState,
                  control: MonitorState,
                  accumulator: &Accumulator,
                  meter: &mut filament_exact::Meter,
                  storage: &mut u64| {
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
                .map_err(|error| admission.refusal(error))?
        };
        measure_probability_product_key(
            binding,
            &raw,
            control,
            accumulator,
            None,
            plan.objective.reward.as_ref(),
            limits,
            cancel,
            meter,
            storage,
        )
    };
    let mut queue = VecDeque::new();
    let mut seen = BTreeSet::new();
    let mut initials = Vec::new();
    for (index, initial) in initial.iter().enumerate() {
        poll(cancel)?;
        let truths = read(initial)?;
        let (control, accumulator) = plan
            .control
            .start(&truths, meter)
            .map_err(ProbabilityDocumentError::from)?;
        let control = control.ok_or(ProbabilityDocumentError::Malformed)?;
        let accumulator = accumulator.ok_or(ProbabilityDocumentError::Malformed)?;
        let root = key_of(initial, control, &accumulator, meter, storage)?;
        charge(
            storage,
            limits.storage_bytes,
            u64::try_from(
                root.as_bytes()
                    .len()
                    .saturating_add(std::mem::size_of::<(u32, ProbabilityProductKey)>()),
            )
            .unwrap_or(u64::MAX),
        )?;
        initials.push((
            u32::try_from(index).map_err(|_| ProbabilityDocumentError::Malformed)?,
            root.clone(),
        ));
        if seen.insert(root.clone()) {
            if seen.len() > limits.states {
                return Err(exceeded(
                    ProbabilityDocumentLimit::States,
                    u64::try_from(limits.states).unwrap_or(u64::MAX),
                    u64::try_from(seen.len()).unwrap_or(u64::MAX),
                ));
            }
            queue.push_back((
                root,
                initial.clone(),
                control,
                accumulator,
                truths,
                None::<Resolution>,
            ));
        }
    }
    let root = seen
        .first()
        .cloned()
        .ok_or(ProbabilityDocumentError::Malformed)?;
    let mut states = BTreeMap::new();
    let mut edges = 0usize;
    while let Some((key, model, control, accumulator, truths, resolution)) = queue.pop_front() {
        poll(cancel)?;
        charge(
            storage,
            limits.storage_bytes,
            u64::try_from(
                key.as_bytes()
                    .len()
                    .saturating_mul(3)
                    .saturating_add(std::mem::size_of::<State>()),
            )
            .unwrap_or(u64::MAX),
        )?;
        if named.is_some_and(|named| !named.contains(&key)) {
            // A lower certificate assigns zero to this frontier key. Its
            // descendants and terminal payoff are outside the named proof.
            states.insert(
                key,
                State {
                    terminal: None,
                    rank: 0,
                    choices: Vec::new(),
                    actions: Vec::new(),
                    activated: false,
                },
            );
            continue;
        }
        let terminal = plan
            .payoff
            .terminal(control, &accumulator, meter)
            .map_err(ProbabilityDocumentError::from)?;
        let rank = if control.is_decided() {
            0
        } else {
            match control {
                MonitorState::Measure { remaining, .. } | MonitorState::Fraction { remaining } => {
                    remaining
                }
                _ => return Err(ProbabilityDocumentError::Malformed),
            }
        };
        let activated = matches!(
            accumulator,
            Accumulator::Steps {
                activated: true,
                ..
            } | Accumulator::Reward {
                activated: true,
                ..
            }
        );
        let mut choices = Vec::new();
        let mut action_labels = Vec::new();
        let resolving = resolution.is_some();
        if terminal.is_none() {
            let mut admission =
                ProbabilityKeyAdmission::new(meter, storage, limits.storage_bytes, cancel);
            let distributions = if let Some((posts, rewards)) = resolution {
                posts
                    .into_iter()
                    .map(|post| {
                        action_labels.push(None);
                        vec![Successor::Position(rational(1), post, rewards.clone())]
                    })
                    .collect()
            } else if let Some(workload) = workload {
                let steps = system
                    .weighted_steps_with_budget(
                        &model,
                        workload,
                        fsl_foundation::IdentityLimits::default(),
                        &mut admission,
                        cancel,
                    )
                    .map_err(|error| distribution_error(error, &mut admission))?
                    .map_err(|_| {
                        ProbabilityDocumentError::Build(Box::new(
                            crate::ExactProbBuildIssue::NotMarkov,
                        ))
                    })?;
                action_labels.push(None);
                vec![steps
                    .into_iter()
                    .map(|step| Successor::Position(step.probability, step.post, step.rewards))
                    .collect::<Vec<_>>()]
            } else {
                let actions = system
                    .actions_with_budget(
                        &model,
                        fsl_foundation::IdentityLimits::default(),
                        &mut admission,
                        cancel,
                    )
                    .map_err(|error| distribution_error(error, &mut admission))?
                    .map_err(|_| {
                        ProbabilityDocumentError::Build(Box::new(
                            crate::ExactProbBuildIssue::UndecidedSuccessor,
                        ))
                    })?;
                if actions.is_empty() {
                    let rewards = system.zero_rewards();
                    action_labels.push(Some(crate::ProductAction::TerminalStutter));
                    vec![vec![Successor::Position(
                        rational(1),
                        model.clone(),
                        rewards,
                    )]]
                } else {
                    let mut distributions = Vec::new();
                    for action in actions {
                        action_labels.push(Some(crate::ProductAction::Identity(
                            crate::Schedule::of(system, &action.identity)
                                .map_err(|_| ProbabilityDocumentError::Malformed)?,
                        )));
                        let mut distribution = Vec::new();
                        for draw in action.draws {
                            if draw.successors.is_empty() {
                                return Err(ProbabilityDocumentError::Build(Box::new(
                                    crate::ExactProbBuildIssue::NotMarkov,
                                )));
                            }
                            if draw.successors.len() == 1 {
                                let (_, post) = draw
                                    .successors
                                    .into_iter()
                                    .next()
                                    .ok_or(ProbabilityDocumentError::Malformed)?;
                                distribution.push(Successor::Position(
                                    draw.probability,
                                    post,
                                    draw.rewards,
                                ));
                            } else {
                                distribution.push(Successor::Draw(
                                    draw.probability,
                                    action.identity.clone(),
                                    draw.random,
                                    draw.successors.into_iter().map(|(_, post)| post).collect(),
                                    draw.rewards,
                                ));
                            }
                        }
                        distributions.push(distribution);
                    }
                    distributions
                }
            };
            drop(admission);
            for distribution in distributions {
                let mut choice = Vec::new();
                for successor in distribution {
                    if let Successor::Draw(probability, scheduled, random, posts, rewards) =
                        successor
                    {
                        if probability.is_zero() {
                            continue;
                        }
                        edges = edges
                            .checked_add(1)
                            .ok_or(ProbabilityDocumentError::KeySizeOverflow)?;
                        if edges > limits.edges {
                            return Err(exceeded(
                                ProbabilityDocumentLimit::Edges,
                                u64::try_from(limits.edges).unwrap_or(u64::MAX),
                                u64::try_from(edges).unwrap_or(u64::MAX),
                            ));
                        }
                        let (raw, pending) = {
                            let mut admission = ProbabilityKeyAdmission::new(
                                meter,
                                storage,
                                limits.storage_bytes,
                                cancel,
                            );
                            let identity = fsl_foundation::IdentityLimits::default();
                            let raw = system
                                .state_key_bytes_with_budget(
                                    &model,
                                    identity,
                                    &mut admission,
                                    cancel,
                                )
                                .map_err(|error| admission.refusal(error))?;
                            let pending = system
                                .pending_key_bytes_with_budget(
                                    &scheduled,
                                    &random,
                                    identity,
                                    &mut admission,
                                    cancel,
                                )
                                .map_err(|error| admission.refusal(error))?;
                            (raw, pending)
                        };
                        let target = measure_probability_product_key(
                            binding,
                            &raw,
                            control,
                            &accumulator,
                            Some(&pending),
                            plan.objective.reward.as_ref(),
                            limits,
                            cancel,
                            meter,
                            storage,
                        )?;
                        charge(
                            storage,
                            limits.storage_bytes,
                            u64::try_from(
                                target
                                    .as_bytes()
                                    .len()
                                    .saturating_mul(3)
                                    .saturating_add(std::mem::size_of::<ProbabilityEdge>()),
                            )
                            .unwrap_or(u64::MAX),
                        )?;
                        if seen.insert(target.clone()) {
                            if seen.len() > limits.states {
                                return Err(exceeded(
                                    ProbabilityDocumentLimit::States,
                                    u64::try_from(limits.states).unwrap_or(u64::MAX),
                                    u64::try_from(seen.len()).unwrap_or(u64::MAX),
                                ));
                            }
                            queue.push_back((
                                target.clone(),
                                model.clone(),
                                control,
                                accumulator.clone(),
                                truths.clone(),
                                Some((posts, rewards)),
                            ));
                        }
                        choice.push(ProbabilityEdge {
                            target,
                            probability,
                        });
                        continue;
                    }
                    let Successor::Position(probability, post, rewards) = successor else {
                        return Err(ProbabilityDocumentError::Malformed);
                    };
                    poll(cancel)?;
                    if probability.is_zero() {
                        continue;
                    }
                    edges = edges
                        .checked_add(1)
                        .ok_or(ProbabilityDocumentError::KeySizeOverflow)?;
                    if edges > limits.edges {
                        return Err(exceeded(
                            ProbabilityDocumentLimit::Edges,
                            u64::try_from(limits.edges).unwrap_or(u64::MAX),
                            u64::try_from(edges).unwrap_or(u64::MAX),
                        ));
                    }
                    let post_truths = if plan.control.reads_post(Some(control)) {
                        read(&post)?
                    } else {
                        Vec::new()
                    };
                    let (next, sums) = plan
                        .control
                        .step(
                            Some(control),
                            Some(&accumulator),
                            &truths,
                            &post_truths,
                            &rewards,
                            meter,
                        )
                        .map_err(ProbabilityDocumentError::from)?;
                    let next = next.ok_or(ProbabilityDocumentError::Malformed)?;
                    let sums = sums.ok_or(ProbabilityDocumentError::Malformed)?;
                    let target = key_of(&post, next, &sums, meter, storage)?;
                    charge(
                        storage,
                        limits.storage_bytes,
                        u64::try_from(
                            target
                                .as_bytes()
                                .len()
                                .saturating_mul(3)
                                .saturating_add(std::mem::size_of::<ProbabilityEdge>()),
                        )
                        .unwrap_or(u64::MAX),
                    )?;
                    if seen.insert(target.clone()) {
                        if seen.len() > limits.states {
                            return Err(exceeded(
                                ProbabilityDocumentLimit::States,
                                u64::try_from(limits.states).unwrap_or(u64::MAX),
                                u64::try_from(seen.len()).unwrap_or(u64::MAX),
                            ));
                        }
                        queue.push_back((target.clone(), post, next, sums, post_truths, None));
                    }
                    choice.push(ProbabilityEdge {
                        target,
                        probability,
                    });
                }
                if resolving {
                    let target = choice
                        .first()
                        .ok_or(ProbabilityDocumentError::Malformed)?
                        .target
                        .clone();
                    *action_labels
                        .get_mut(choices.len())
                        .ok_or(ProbabilityDocumentError::Malformed)? =
                        Some(crate::ProductAction::PostState(target));
                }
                choices.push(choice);
            }
        }
        states.insert(
            key,
            State {
                terminal,
                rank,
                choices,
                actions: action_labels,
                activated,
            },
        );
    }
    Ok(Graph {
        initial: root,
        initials,
        states,
    })
}

fn distribution_error(
    error: fsl_eval::simulation::model::ModelKeyExpansionError,
    admission: &mut ProbabilityKeyAdmission<'_>,
) -> ProbabilityDocumentError {
    match error {
        fsl_eval::simulation::model::ModelKeyExpansionError::Key(error) => admission.refusal(error),
        fsl_eval::simulation::model::ModelKeyExpansionError::Expansion(error) => {
            ProbabilityDocumentError::Build(Box::new(crate::ExactProbBuildIssue::Expansion(error)))
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WireMeasure {
    kind: String,
    window: u64,
    reward: Option<(Box<serde_json::value::RawValue>, String)>,
    unit: Box<serde_json::value::RawValue>,
    comparison: Option<String>,
    threshold: Option<WireRational>,
    level: Option<WireRational>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WireObjective {
    profile: String,
    payoff: String,
    measure: WireMeasure,
    optimization: String,
    fairness: Vec<Box<serde_json::value::RawValue>>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WireDocument {
    identity: WireIdentity,
    objective: WireObjective,
    kind: String,
    values: Vec<WireValue>,
    ranking: Vec<WireRank>,
    conditional: Option<WireConditional>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct WireConditional {
    mode: String,
    numerator: Option<Box<WireDocument>>,
    denominator: Option<Box<WireDocument>>,
}

fn decode_state(
    text: &str,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
    storage: &mut u64,
) -> Result<ProbabilityProductKey, ProbabilityDocumentError> {
    poll(cancel)?;
    charge(
        storage,
        limits.storage_bytes,
        u64::try_from(text.len()).unwrap_or(u64::MAX),
    )?;
    let bytes = URL_SAFE_NO_PAD
        .decode(text)
        .map_err(|_| ProbabilityDocumentError::MalformedMember("values"))?;
    if URL_SAFE_NO_PAD.encode(&bytes) != text {
        return Err(ProbabilityDocumentError::Noncanonical);
    }
    let admission = RefCell::new(writer::Admission::with_used(
        limits.storage_bytes,
        cancel,
        *storage,
    ));
    let parsed = quire_canonical::read_with_reservation(
        &bytes,
        limits.bytes,
        &mut writer::Reserve(&admission),
    );
    *storage = admission.borrow().used();
    if let Some(error) = admission.borrow_mut().take_stop() {
        return Err(error);
    }
    let parsed = parsed.map_err(|_| ProbabilityDocumentError::MalformedMember("values"))?;
    let canonical = writer::encode_reserved(&parsed, limits.bytes, &admission)?;
    *storage = admission.borrow().used();
    if canonical != bytes {
        return Err(ProbabilityDocumentError::Noncanonical);
    }
    let Node::Array(parts) = parsed.root().node() else {
        return Err(ProbabilityDocumentError::MalformedMember("values"));
    };
    let parts = parts.collect::<Vec<_>>();
    if parts.len() != 8
        || !matches!(
            parts[0].node(),
            Node::String("filament.probability-product-key/v1")
        )
        || !matches!(parts[2].node(), Node::Null)
        || !matches!(parts[3].node(), Node::Null)
    {
        return Err(ProbabilityDocumentError::MalformedMember("values"));
    }
    if !matches!(parts[5].node(), Node::Array(monitor) if monitor.len() == 3)
        || !matches!(parts[6].node(), Node::Array(_))
    {
        return Err(ProbabilityDocumentError::MalformedMember("values"));
    }
    Ok(ProbabilityProductKey::from_canonical_bytes(bytes))
}

fn read_wire(
    wire: WireDocument,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
    meter: &mut filament_exact::Meter,
    storage: &mut u64,
    depth: u8,
) -> Result<MeasureProbabilityDocument, ProbabilityDocumentError> {
    poll(cancel)?;
    if depth > 1 || wire.objective.profile != PROFILE {
        return Err(ProbabilityDocumentError::Unsupported);
    }
    let payoff = MeasurePayoffKind::read(&wire.objective.payoff)?;
    let identity = ObligationIdentity::new(
        fsl_foundation::node_wire::node_key_from_text(wire.identity.node_id.get())
            .map_err(|_| ProbabilityDocumentError::MalformedMember("identity"))?,
        wire.identity.model.map(crate::ModelIdentity::new),
        wire.identity.ordinal,
    );
    let objective = wire.objective;
    let measure = objective.measure;
    let reward = measure
        .reward
        .map(|(model, name)| {
            Ok::<_, ProbabilityDocumentError>(ProbabilityRewardObjective {
                model: fsl_foundation::node_wire::node_key_from_text(model.get())
                    .map_err(|_| ProbabilityDocumentError::MalformedMember("objective"))?,
                name: filament_exact::Identifier::new(&name)
                    .map_err(|_| ProbabilityDocumentError::MalformedMember("objective"))?,
                unit: unit::read(measure.unit.get().as_bytes(), limits, cancel)?,
            })
        })
        .transpose()?;
    if reward.is_none() && measure.unit.get() != "null" {
        return Err(ProbabilityDocumentError::ObjectiveMismatch);
    }
    let parse_rational = |wire, meter: &mut filament_exact::Meter| {
        ratio_checked(wire, limits.scalar_bits, meter, cancel, false)
    };
    let comparison = measure
        .comparison
        .as_deref()
        .map(|text| match text {
            "le" => Ok(MeasureComparison::Le),
            "lt" => Ok(MeasureComparison::Lt),
            "ge" => Ok(MeasureComparison::Ge),
            _ => Err(ProbabilityDocumentError::ObjectiveMismatch),
        })
        .transpose()?;
    let kind = match measure.kind.as_str() {
        "steps" => "steps",
        "accumulate" => "accumulate",
        "fraction" => "fraction",
        "weighted-fraction" => "weighted-fraction",
        _ => return Err(ProbabilityDocumentError::ObjectiveMismatch),
    };
    let objective = Objective {
        kind,
        window: measure.window,
        reward,
        comparison,
        threshold: measure
            .threshold
            .map(|wire| parse_rational(wire, meter))
            .transpose()?,
        level: measure
            .level
            .map(|wire| parse_rational(wire, meter))
            .transpose()?,
        optimization: match objective.optimization.as_str() {
            "minimum" => ProbabilityObjective::Minimum,
            "maximum" => ProbabilityObjective::Maximum,
            "workload" => ProbabilityObjective::Workload,
            _ => return Err(ProbabilityDocumentError::ObjectiveMismatch),
        },
        fairness: fairness::read(&objective.fairness, limits, cancel, meter, storage)?,
    };
    if wire.values.len() > limits.states || wire.ranking.len() > limits.states {
        return Err(exceeded(
            ProbabilityDocumentLimit::States,
            u64::try_from(limits.states).unwrap_or(u64::MAX),
            u64::try_from(wire.values.len().max(wire.ranking.len())).unwrap_or(u64::MAX),
        ));
    }
    let mut values = Vec::new();
    for row in wire.values {
        let state = decode_state(&row.state, limits, cancel, storage)?;
        let value = parse_rational(row.value, meter)?;
        if values
            .last()
            .is_some_and(|previous: &ExactProbabilityValue| previous.state >= state)
        {
            return Err(ProbabilityDocumentError::MalformedMember("values"));
        }
        charge(
            storage,
            limits.storage_bytes,
            u64::try_from(
                std::mem::size_of::<ExactProbabilityValue>().saturating_add(state.as_bytes().len()),
            )
            .unwrap_or(u64::MAX),
        )?;
        values.push(ExactProbabilityValue { state, value });
    }
    let mut ranking = Vec::new();
    for row in wire.ranking {
        let state = decode_state(&row.state, limits, cancel, storage)?;
        if row.rank > MAX_COUNT
            || ranking
                .last()
                .is_some_and(|previous: &(ProbabilityProductKey, u64)| previous.0 >= state)
        {
            return Err(ProbabilityDocumentError::MalformedMember("ranking"));
        }
        ranking.push((state, row.rank));
    }
    let conditional = match (wire.kind.as_str(), wire.conditional) {
        ("exact", None) => None,
        ("conditional", Some(conditional))
            if depth == 0 && values.is_empty() && ranking.is_empty() =>
        {
            Some(Box::new(match conditional.mode.as_str() {
                "ratio" if objective.optimization == ProbabilityObjective::Workload => {
                    Conditional::Ratio(
                        read_wire(
                            *conditional
                                .numerator
                                .ok_or(ProbabilityDocumentError::MalformedMember("numerator"))?,
                            limits,
                            cancel,
                            meter,
                            storage,
                            depth + 1,
                        )?,
                        read_wire(
                            *conditional
                                .denominator
                                .ok_or(ProbabilityDocumentError::MalformedMember("denominator"))?,
                            limits,
                            cancel,
                            meter,
                            storage,
                            depth + 1,
                        )?,
                    )
                }
                "vacuous"
                    if conditional.numerator.is_none() && conditional.denominator.is_none() =>
                {
                    Conditional::Vacuous
                }
                _ => return Err(ProbabilityDocumentError::MalformedMember("mode")),
            }))
        }
        ("conditional", None) => {
            return Err(ProbabilityDocumentError::MalformedMember("conditional"))
        }
        _ => return Err(ProbabilityDocumentError::MalformedMember("kind")),
    };
    Ok(MeasureProbabilityDocument {
        identity,
        payoff,
        values,
        ranking,
        objective,
        conditional,
    })
}

/// Decode strict canonical finite-horizon measure evidence, without accepting it.
fn decode_measure_probability_certificate(
    bytes: &[u8],
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
    meter: &mut filament_exact::Meter,
    storage: &mut u64,
) -> Result<MeasureProbabilityDocument, ProbabilityDocumentError> {
    poll(cancel)?;
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > limits.bytes {
        return Err(exceeded(
            ProbabilityDocumentLimit::Bytes,
            limits.bytes,
            u64::try_from(bytes.len()).unwrap_or(u64::MAX),
        ));
    }
    charge(
        storage,
        limits.storage_bytes,
        u64::try_from(bytes.len())
            .unwrap_or(u64::MAX)
            .saturating_mul(4),
    )?;
    let wire: WireDocument =
        serde_json::from_slice(bytes).map_err(|_| ProbabilityDocumentError::Malformed)?;
    let document = read_wire(wire, limits, cancel, meter, storage, 0)?;
    let canonical = encode_with_storage(&document, limits, cancel, storage)?;
    if canonical != bytes {
        return Err(ProbabilityDocumentError::Noncanonical);
    }
    Ok(document)
}

/// Decode the strict closed profile and validate its terminal-payoff value range.
/// Contextual checking compares the checked objective before this range test.
pub fn read_measure_probability_certificate(
    bytes: &[u8],
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
) -> Result<MeasureProbabilityDocument, ProbabilityDocumentError> {
    let mut meter = filament_exact::Meter::new(limits.accounting).with_cancel(cancel.clone());
    let document =
        decode_measure_probability_certificate(bytes, limits, cancel, &mut meter, &mut 0)?;
    validate_ranges(&document, limits, cancel, &mut meter)?;
    Ok(document)
}
pub(super) fn identity(
    bytes: &[u8],
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
) -> Result<ObligationIdentity, ProbabilityDocumentError> {
    Ok(decode_measure_probability_certificate(
        bytes,
        limits,
        cancel,
        &mut filament_exact::Meter::new(limits.accounting).with_cancel(cancel.clone()),
        &mut 0,
    )?
    .identity)
}
fn validate_ranges(
    document: &MeasureProbabilityDocument,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
    meter: &mut filament_exact::Meter,
) -> Result<(), ProbabilityDocumentError> {
    let (lower, upper) = if document.payoff == MeasurePayoffKind::Shifted {
        let level = document
            .objective
            .level
            .as_ref()
            .ok_or(ProbabilityDocumentError::ObjectiveMismatch)?;
        (
            arithmetic(
                filament_exact::RationalArithmetic::Negate(level),
                meter,
                cancel,
            )?,
            arithmetic(
                filament_exact::RationalArithmetic::Subtract(&super::rational(1), level),
                meter,
                cancel,
            )?,
        )
    } else {
        (super::rational(0), super::rational(1))
    };
    for row in &document.values {
        poll(cancel)?;
        if ordered(Order::Less, &row.value, &lower, meter, cancel)?
            || ordered(Order::Greater, &row.value, &upper, meter, cancel)?
        {
            return Err(ProbabilityDocumentError::MalformedMember("values"));
        }
    }
    if let Some(conditional) = &document.conditional {
        if let Conditional::Ratio(numerator, denominator) = conditional.as_ref() {
            validate_ranges(numerator, limits, cancel, meter)?;
            validate_ranges(denominator, limits, cancel, meter)?;
        }
    }
    Ok(())
}

struct Events<'a> {
    document: &'a MeasureProbabilityDocument,
    text: writer::Text<'a>,
}
impl Events<'_> {
    fn rational<S: Sink + ?Sized>(
        &self,
        value: &Rational,
        writer: &mut Writer<'_, S>,
    ) -> Result<(), Error> {
        writer.begin_object()?;
        writer.name("numerator")?;
        self.text.integer(value.numerator(), writer)?;
        writer.name("denominator")?;
        self.text.integer(value.denominator(), writer)?;
        writer.end_object()
    }
}
impl Encode for Events<'_> {
    fn encode_into<S: Sink + ?Sized>(&self, writer: &mut Writer<'_, S>) -> Result<(), Error> {
        self.text.poll()?;
        let document = self.document;
        writer.begin_object()?;
        writer.name("identity")?;
        writer.begin_object()?;
        writer.name("node_id")?;
        fsl_foundation::node_wire::encode_node_key(document.identity.node(), writer)?;
        writer.name("model")?;
        match document.identity.model() {
            Some(model) => writer.string(model.as_str())?,
            None => writer.null()?,
        }
        writer.name("ordinal")?;
        writer.integer(i128::from(document.identity.ordinal()))?;
        writer.end_object()?;
        writer.name("objective")?;
        writer.begin_object()?;
        writer.name("profile")?;
        writer.string(PROFILE)?;
        writer.name("payoff")?;
        writer.string(document.payoff.text())?;
        writer.name("measure")?;
        writer.begin_object()?;
        writer.name("kind")?;
        writer.string(document.objective.kind)?;
        writer.name("window")?;
        writer.integer(i128::from(document.objective.window))?;
        writer.name("reward")?;
        match &document.objective.reward {
            None => writer.null()?,
            Some(reward) => {
                writer.begin_array()?;
                fsl_foundation::node_wire::encode_node_key(&reward.model, writer)?;
                writer.string(reward.name.as_str())?;
                writer.end_array()?;
            }
        }
        writer.name("unit")?;
        self.text.unit(
            document
                .objective
                .reward
                .as_ref()
                .and_then(|reward| reward.unit.as_ref()),
            writer,
        )?;
        writer.name("comparison")?;
        match document.objective.comparison {
            None => writer.null()?,
            Some(comparison) => writer.string(match comparison {
                MeasureComparison::Le => "le",
                MeasureComparison::Lt => "lt",
                MeasureComparison::Ge => "ge",
            })?,
        }
        writer.name("threshold")?;
        match &document.objective.threshold {
            None => writer.null()?,
            Some(value) => self.rational(value, writer)?,
        }
        writer.name("level")?;
        match &document.objective.level {
            None => writer.null()?,
            Some(value) => self.rational(value, writer)?,
        }
        writer.end_object()?;
        writer.name("optimization")?;
        writer.string(match document.objective.optimization {
            ProbabilityObjective::Minimum => "minimum",
            ProbabilityObjective::Maximum => "maximum",
            ProbabilityObjective::Workload => "workload",
        })?;
        writer.name("fairness")?;
        writer.begin_array()?;
        for constraint in &document.objective.fairness {
            self.text.poll()?;
            constraint.encode_into(writer)?;
        }
        writer.end_array()?;
        writer.end_object()?;
        writer.name("kind")?;
        writer.string(if document.conditional.is_some() {
            "conditional"
        } else {
            "exact"
        })?;
        writer.name("values")?;
        writer.begin_array()?;
        for (index, row) in document.values.iter().enumerate() {
            writer.begin_object()?;
            writer.name("state")?;
            self.text.key(&row.state, writer, index != 0)?;
            writer.name("value")?;
            self.rational(&row.value, writer)?;
            writer.end_object()?;
        }
        writer.end_array()?;
        writer.name("ranking")?;
        writer.begin_array()?;
        for (index, (state, rank)) in document.ranking.iter().enumerate() {
            writer.begin_object()?;
            writer.name("state")?;
            self.text.key(state, writer, index != 0)?;
            writer.name("rank")?;
            writer.integer(i128::from(*rank))?;
            writer.end_object()?;
        }
        writer.end_array()?;
        if let Some(conditional) = &document.conditional {
            writer.name("conditional")?;
            writer.begin_object()?;
            writer.name("mode")?;
            match conditional.as_ref() {
                Conditional::Vacuous => writer.string("vacuous")?,
                Conditional::Ratio(numerator, denominator) => {
                    writer.string("ratio")?;
                    writer.name("numerator")?;
                    Events {
                        document: numerator,
                        text: writer::Text::new(self.text.limits(), self.text.cancel()),
                    }
                    .encode_into(writer)?;
                    writer.name("denominator")?;
                    Events {
                        document: denominator,
                        text: writer::Text::new(self.text.limits(), self.text.cancel()),
                    }
                    .encode_into(writer)?;
                }
            }
            writer.end_object()?;
        }
        writer.end_object()
    }
}
fn encode(
    document: &MeasureProbabilityDocument,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
) -> Result<Vec<u8>, ProbabilityDocumentError> {
    encode_with_storage(document, limits, cancel, &mut 0)
}
fn encode_with_storage(
    document: &MeasureProbabilityDocument,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
    storage: &mut u64,
) -> Result<Vec<u8>, ProbabilityDocumentError> {
    let admission = RefCell::new(writer::Admission::with_used(
        limits.storage_bytes,
        cancel,
        *storage,
    ));
    let events = Events {
        document,
        text: writer::Text::new(limits, cancel),
    };
    let result = writer::encode_reserved(&events, limits.bytes, &admission);
    *storage = admission.borrow().used();
    if let Some(error) = events
        .text
        .take_stop()
        .or_else(|| admission.borrow_mut().take_stop())
    {
        return Err(error);
    }
    result
}
/// Write canonical unchecked finite-horizon measure proof bytes.
pub fn write_measure_probability_certificate(
    document: &MeasureProbabilityDocument,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
) -> Result<Vec<u8>, ProbabilityDocumentError> {
    let bytes = encode(document, limits, cancel)?;
    read_measure_probability_certificate(&bytes, limits, cancel)?;
    Ok(bytes)
}

/// Build an unchecked exact measure document from the producer's values.
/// Ranks, objective and reachable coverage come from independent reconstruction;
/// the offered values are copied without solving any equation.
#[allow(clippy::too_many_arguments)]
pub fn measure_probability_certificate_document(
    system: &ModelSystem<'_>,
    claims: &[CheckedProbabilisticClaim],
    item: &ObligationIdentity,
    certificate: &ExactProbabilityCertificate,
    payoff: MeasurePayoffKind,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
) -> Result<MeasureProbabilityDocument, ProbabilityDocumentError> {
    let claim = bind_claim(claims, item, cancel)?;
    let mut meter = filament_exact::Meter::new(limits.accounting).with_cancel(cancel.clone());
    let mut storage = 0;
    let plan = plan(
        system,
        claim,
        payoff,
        limits,
        cancel,
        &mut meter,
        &mut storage,
    )?;
    let graph = enumerate(
        system,
        claim,
        &plan,
        limits,
        cancel,
        &mut meter,
        &mut storage,
        None,
    )?;
    Ok(MeasureProbabilityDocument {
        identity: item.clone(),
        payoff,
        values: certificate.values.clone(),
        ranking: graph
            .states
            .into_iter()
            .map(|(key, state)| (key, state.rank))
            .collect(),
        objective: plan.objective,
        conditional: None,
    })
}

/// Build unchecked value-free quantile evidence without computing discarded
/// numeric values or ranks. Only the independent checker proves vacuity.
pub fn vacuous_measure_probability_certificate_document(
    system: &ModelSystem<'_>,
    claims: &[CheckedProbabilisticClaim],
    item: &ObligationIdentity,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
) -> Result<MeasureProbabilityDocument, ProbabilityDocumentError> {
    let claim = bind_claim(claims, item, cancel)?;
    if !matches!(claim.form, ProbForm::Quantile { .. }) {
        return Err(ProbabilityDocumentError::MalformedMember("mode"));
    }
    let mut meter = filament_exact::Meter::new(limits.accounting).with_cancel(cancel.clone());
    let mut storage = 0;
    let selected = plan(
        system,
        claim,
        MeasurePayoffKind::Satisfied,
        limits,
        cancel,
        &mut meter,
        &mut storage,
    )?;
    Ok(MeasureProbabilityDocument {
        identity: item.clone(),
        payoff: MeasurePayoffKind::Satisfied,
        values: Vec::new(),
        ranking: Vec::new(),
        objective: selected.objective,
        conditional: Some(Box::new(Conditional::Vacuous)),
    })
}

/// Combine exact numerator and denominator documents into a workload ratio.
/// Both parts remain independently checked before any ratio is accepted.
pub fn conditional_measure_probability_certificate(
    numerator: MeasureProbabilityDocument,
    denominator: MeasureProbabilityDocument,
) -> Result<MeasureProbabilityDocument, ProbabilityDocumentError> {
    if numerator.identity != denominator.identity
        || numerator.payoff != MeasurePayoffKind::Satisfied
        || denominator.payoff != MeasurePayoffKind::Activated
        || numerator.conditional.is_some()
        || denominator.conditional.is_some()
        || numerator.objective.optimization != ProbabilityObjective::Workload
        || denominator.objective.optimization != ProbabilityObjective::Workload
    {
        return Err(ProbabilityDocumentError::MalformedMember("mode"));
    }
    Ok(MeasureProbabilityDocument {
        identity: numerator.identity.clone(),
        payoff: numerator.payoff,
        values: Vec::new(),
        ranking: Vec::new(),
        objective: numerator.objective.clone(),
        conditional: Some(Box::new(Conditional::Ratio(numerator, denominator))),
    })
}

fn checked_values(
    document: &MeasureProbabilityDocument,
    graph: &Graph,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
    meter: &mut filament_exact::Meter,
) -> Result<
    Result<BTreeMap<ProbabilityProductKey, Rational>, ExactProbabilityCertificateCheck>,
    ProbabilityDocumentError,
> {
    let shape = |member| {
        ExactProbabilityCertificateCheck::Rejected(ProbabilityCertificateRejection {
            rule: ProbabilityCertificateRule::Malformed,
            at: ProbabilityCertificateLocus::Member(member),
        })
    };
    if document.values.len() != graph.states.len()
        || document
            .values
            .iter()
            .map(|row| &row.state)
            .ne(graph.states.keys())
    {
        return Ok(Err(shape("values")));
    }
    if document.ranking.len() != graph.states.len()
        || document
            .ranking
            .iter()
            .map(|row| &row.0)
            .ne(graph.states.keys())
    {
        return Ok(Err(shape("ranking")));
    }
    let values = document
        .values
        .iter()
        .map(|row| (row.state.clone(), row.value.clone()))
        .collect::<BTreeMap<_, _>>();
    for (key, state) in &graph.states {
        poll(cancel)?;
        if document
            .ranking
            .binary_search_by(|row| row.0.cmp(key))
            .ok()
            .map(|index| document.ranking[index].1)
            != Some(state.rank)
        {
            return Ok(Err(rejected(
                ProbabilityCertificateRule::RankBroken,
                key.clone(),
            )));
        }
        let expected = match &state.terminal {
            Some(value) => value.clone(),
            None => {
                let mut optimum: Option<Rational> = None;
                for choice in &state.choices {
                    let sum = action_sum(choice, &values, meter, cancel)?;
                    optimum = Some(match optimum {
                        None => sum,
                        Some(old) => match document.objective.optimization {
                            ProbabilityObjective::Minimum => {
                                if ordered(Order::Less, &old, &sum, meter, cancel)? {
                                    old
                                } else {
                                    sum
                                }
                            }
                            ProbabilityObjective::Maximum => {
                                if ordered(Order::Greater, &old, &sum, meter, cancel)? {
                                    old
                                } else {
                                    sum
                                }
                            }
                            ProbabilityObjective::Workload => {
                                return Err(ProbabilityDocumentError::Malformed)
                            }
                        },
                    });
                }
                optimum.ok_or(ProbabilityDocumentError::Malformed)?
            }
        };
        check_value_bits(&expected, limits.scalar_bits)?;
        if values.get(key) != Some(&expected) {
            return Ok(Err(rejected(
                ProbabilityCertificateRule::InequalityFails,
                key.clone(),
            )));
        }
    }
    Ok(Ok(values))
}
fn check_value_bits(value: &Rational, bits: u64) -> Result<(), ProbabilityDocumentError> {
    let actual = value
        .numerator()
        .magnitude_bits()
        .max(value.denominator().magnitude_bits());
    if actual > bits {
        return Err(exceeded(
            ProbabilityDocumentLimit::RationalBits,
            bits,
            actual,
        ));
    }
    Ok(())
}
fn action_sum(
    choice: &[ProbabilityEdge],
    values: &BTreeMap<ProbabilityProductKey, Rational>,
    meter: &mut filament_exact::Meter,
    cancel: &Cancel,
) -> Result<Rational, ProbabilityDocumentError> {
    let mut sum = super::rational(0);
    let mut mass = super::rational(0);
    for edge in choice {
        poll(cancel)?;
        if edge.probability < super::rational(0) {
            return Err(ProbabilityDocumentError::Malformed);
        }
        let value = values
            .get(&edge.target)
            .ok_or(ProbabilityDocumentError::MalformedMember("values"))?;
        let term = arithmetic(
            filament_exact::RationalArithmetic::Multiply(&edge.probability, value),
            meter,
            cancel,
        )?;
        sum = arithmetic(
            filament_exact::RationalArithmetic::Add(&sum, &term),
            meter,
            cancel,
        )?;
        mass = arithmetic(
            filament_exact::RationalArithmetic::Add(&mass, &edge.probability),
            meter,
            cancel,
        )?;
    }
    if mass != super::rational(1) {
        return Err(ProbabilityDocumentError::Malformed);
    }
    Ok(sum)
}
fn tight_activation(
    graph: &Graph,
    values: &BTreeMap<ProbabilityProductKey, Rational>,
    meter: &mut filament_exact::Meter,
    cancel: &Cancel,
) -> Result<Option<ProbabilityProductKey>, ProbabilityDocumentError> {
    let mut seen = BTreeSet::from([graph.initial.clone()]);
    let mut queue = VecDeque::from([graph.initial.clone()]);
    let mut activated = BTreeSet::new();
    while let Some(key) = queue.pop_front() {
        poll(cancel)?;
        let state = graph
            .states
            .get(&key)
            .ok_or(ProbabilityDocumentError::Malformed)?;
        if state.activated {
            activated.insert(key.clone());
        }
        for choice in &state.choices {
            if values.get(&key) != Some(&action_sum(choice, values, meter, cancel)?) {
                continue;
            }
            for edge in choice {
                if !edge.probability.is_zero() && seen.insert(edge.target.clone()) {
                    queue.push_back(edge.target.clone());
                }
            }
        }
    }
    Ok(activated.into_iter().next())
}
fn acceptance(
    document: &MeasureProbabilityDocument,
    value: Rational,
    side: ProbabilityBoundSide,
    bytes: &[u8],
) -> ExactProbabilityCertificateCheck {
    ExactProbabilityCertificateCheck::Accepted(ExactProbabilityAcceptance {
        initial_values: None,
        value,
        side,
        certificate: ExactProbabilityCertificate {
            values: document.values.clone(),
        },
        document: Some(bytes.to_vec().into_boxed_slice()),
    })
}

pub(super) fn check(
    system: &ModelSystem<'_>,
    claims: &[CheckedProbabilisticClaim],
    item: &ObligationIdentity,
    bytes: &[u8],
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
) -> Result<ExactProbabilityCertificateCheck, ProbabilityDocumentError> {
    let mut meter = filament_exact::Meter::new(limits.accounting).with_cancel(cancel.clone());
    let mut storage = 0;
    let document = match decode_measure_probability_certificate(
        bytes,
        limits,
        cancel,
        &mut meter,
        &mut storage,
    ) {
        Ok(document) => document,
        Err(error) => return document_admission_error(error),
    };
    if &document.identity != item {
        return Err(ProbabilityDocumentError::IdentityMismatch);
    }
    if !std::ptr::eq(system.input().probabilistic.claims(), claims) {
        return Err(ProbabilityDocumentError::IdentityMismatch);
    }
    let claim = bind_claim(claims, item, cancel)?;
    let plan = plan(
        system,
        claim,
        document.payoff,
        limits,
        cancel,
        &mut meter,
        &mut storage,
    )?;
    if plan.objective != document.objective {
        return Err(ProbabilityDocumentError::ObjectiveMismatch);
    }
    if document.conditional.is_none() {
        if let Err(error) = validate_ranges(&document, limits, cancel, &mut meter) {
            return document_admission_error(error);
        }
    }
    if let Some(conditional) = &document.conditional {
        if matches!(conditional.as_ref(), Conditional::Vacuous) {
            if !matches!(claim.form, ProbForm::Quantile { .. })
                || document.payoff != MeasurePayoffKind::Satisfied
            {
                return Err(ProbabilityDocumentError::MalformedMember("mode"));
            }
            let graph = enumerate(
                system,
                claim,
                &plan,
                limits,
                cancel,
                &mut meter,
                &mut storage,
                None,
            )?;
            if graph.states.values().any(|state| state.activated) {
                return Ok(rejected(
                    ProbabilityCertificateRule::InequalityFails,
                    graph.initial,
                ));
            }
            poll(cancel)?;
            return Ok(ExactProbabilityCertificateCheck::AcceptedVacuous(
                ProbabilityVacuityAcceptance {
                    document: bytes.to_vec().into_boxed_slice(),
                    side: if plan.strict {
                        ProbabilityBoundSide::AtLeast
                    } else {
                        ProbabilityBoundSide::AtMost
                    },
                },
            ));
        }
        let Conditional::Ratio(numerator, denominator) = conditional.as_ref() else {
            return Err(ProbabilityDocumentError::MalformedMember("mode"));
        };
        if !matches!(claim.scheduler, ProbScheduler::Workload(_))
            || !matches!(claim.form, ProbForm::Quantile { .. })
            || numerator.payoff != MeasurePayoffKind::Satisfied
            || denominator.payoff != MeasurePayoffKind::Activated
            || numerator.identity != document.identity
            || denominator.identity != document.identity
        {
            return Err(ProbabilityDocumentError::MalformedMember("conditional"));
        }
        let mut checked_parts = Vec::new();
        for (part, part_name) in [
            (numerator, ProbabilityCertificatePart::Numerator),
            (denominator, ProbabilityCertificatePart::Denominator),
        ] {
            let part_plan = plan_for_part(
                system,
                claim,
                part,
                limits,
                cancel,
                &mut meter,
                &mut storage,
            )?;
            if let Err(error) = validate_ranges(part, limits, cancel, &mut meter) {
                return document_admission_error(error);
            }
            let graph_part = enumerate(
                system,
                claim,
                &part_plan,
                limits,
                cancel,
                &mut meter,
                &mut storage,
                None,
            )?;
            let values = match checked_values(part, &graph_part, limits, cancel, &mut meter)? {
                Ok(values) => values,
                Err(mut rejection) => {
                    if let ExactProbabilityCertificateCheck::Rejected(failure) = &mut rejection {
                        if let ProbabilityCertificateLocus::ProductState(key) = &failure.at {
                            failure.at = ProbabilityCertificateLocus::ProductPart {
                                key: key.clone(),
                                part: part_name,
                            };
                        }
                    }
                    return Ok(rejection);
                }
            };
            checked_parts.push((graph_part, values));
        }
        let [(numerator_graph, numerator_values), (denominator_graph, denominator_values)] =
            checked_parts.as_slice()
        else {
            return Err(ProbabilityDocumentError::Malformed);
        };
        if numerator_graph.initials != denominator_graph.initials {
            return Err(ProbabilityDocumentError::Malformed);
        }
        let threshold = match &plan.bound {
            Bound::AtLeast(value) | Bound::AtMost(value) => value,
        };
        let mut coverage = Vec::new();
        let mut least: Option<Rational> = None;
        for (index, key) in &denominator_graph.initials {
            charge(
                &mut storage,
                limits.storage_bytes,
                u64::try_from(std::mem::size_of::<(u32, Option<Rational>)>()).unwrap_or(u64::MAX),
            )?;
            let numerator = numerator_values
                .get(key)
                .ok_or(ProbabilityDocumentError::Malformed)?;
            let denominator = denominator_values
                .get(key)
                .ok_or(ProbabilityDocumentError::Malformed)?;
            if denominator.is_zero() {
                // checked_values proves the full nonnegative activation Bellman
                // system, hence zero at this initial proves no positive activation.
                if !numerator.is_zero() {
                    return Err(ProbabilityDocumentError::Malformed);
                }
                coverage.push((*index, None));
                continue;
            }
            let value = arithmetic(
                filament_exact::RationalArithmetic::Divide(numerator, denominator),
                &mut meter,
                cancel,
            )?;
            check_value_bits(&value, limits.scalar_bits)?;
            charge(
                &mut storage,
                limits.storage_bytes,
                value
                    .numerator()
                    .magnitude_bits()
                    .saturating_add(value.denominator().magnitude_bits())
                    .div_ceil(8),
            )?;
            if !ordered(
                if plan.strict {
                    Order::Less
                } else {
                    Order::GreaterOrEqual
                },
                &value,
                threshold,
                &mut meter,
                cancel,
            )? {
                return Ok(rejected(
                    ProbabilityCertificateRule::ThresholdMissed,
                    key.clone(),
                ));
            }
            let replace = match &least {
                None => true,
                Some(old) => ordered(
                    if plan.strict {
                        Order::Greater
                    } else {
                        Order::Less
                    },
                    &value,
                    old,
                    &mut meter,
                    cancel,
                )?,
            };
            if replace {
                least = Some(value.clone());
            }
            coverage.push((*index, Some(value)));
        }
        let Some(value) = least else {
            return Ok(rejected(
                ProbabilityCertificateRule::InequalityFails,
                denominator_graph.initial.clone(),
            ));
        };
        let mut accepted = acceptance(
            &document,
            value,
            if plan.strict {
                ProbabilityBoundSide::AtLeast
            } else {
                ProbabilityBoundSide::AtMost
            },
            bytes,
        );
        if let ExactProbabilityCertificateCheck::Accepted(token) = &mut accepted {
            let binding = claim_binding(system, claim)?.map(|binding| crate::ExactBinding {
                population: binding.instance.population.node,
                key: binding.instance.key,
            });
            token.initial_values = Some(CheckedInitialValues {
                values: coverage,
                binding,
            });
        }
        return Ok(accepted);
    }
    if matches!(claim.form, ProbForm::Quantile { .. })
        && document.payoff != MeasurePayoffKind::Shifted
    {
        return Err(ProbabilityDocumentError::MalformedMember("conditional"));
    }
    let graph = enumerate(
        system,
        claim,
        &plan,
        limits,
        cancel,
        &mut meter,
        &mut storage,
        None,
    )?;
    if graph.initials.len() != 1 {
        return Err(ProbabilityDocumentError::Unsupported);
    }
    if matches!(claim.form, ProbForm::Quantile { .. })
        && !graph.states.values().any(|state| state.activated)
    {
        return Ok(rejected(
            ProbabilityCertificateRule::InequalityFails,
            graph.initial,
        ));
    }
    let values = match checked_values(&document, &graph, limits, cancel, &mut meter)? {
        Ok(values) => values,
        Err(rejection) => return Ok(rejection),
    };
    let value = values
        .get(&graph.initial)
        .ok_or(ProbabilityDocumentError::Malformed)?
        .clone();
    let (holds, side) = if document.payoff == MeasurePayoffKind::Shifted {
        if plan.strict && value.is_zero() {
            if let Some(key) = tight_activation(&graph, &values, &mut meter, cancel)? {
                return Ok(rejected(ProbabilityCertificateRule::InequalityFails, key));
            }
        }
        (
            if plan.strict {
                ordered(
                    Order::LessOrEqual,
                    &value,
                    &super::rational(0),
                    &mut meter,
                    cancel,
                )?
            } else {
                ordered(
                    Order::GreaterOrEqual,
                    &value,
                    &super::rational(0),
                    &mut meter,
                    cancel,
                )?
            },
            if plan.strict {
                ProbabilityBoundSide::AtLeast
            } else {
                ProbabilityBoundSide::AtMost
            },
        )
    } else {
        match &plan.bound {
            Bound::AtLeast(threshold) => (
                ordered(Order::GreaterOrEqual, &value, threshold, &mut meter, cancel)?,
                ProbabilityBoundSide::AtLeast,
            ),
            Bound::AtMost(threshold) => (
                ordered(Order::LessOrEqual, &value, threshold, &mut meter, cancel)?,
                ProbabilityBoundSide::AtMost,
            ),
        }
    };
    if !holds {
        return Ok(rejected(
            ProbabilityCertificateRule::ThresholdMissed,
            graph.initial,
        ));
    }
    Ok(acceptance(&document, value, side, bytes))
}
#[allow(clippy::too_many_arguments)]
fn plan_for_part<'a>(
    system: &ModelSystem<'_>,
    claim: &'a CheckedProbabilisticClaim,
    document: &MeasureProbabilityDocument,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
    meter: &mut filament_exact::Meter,
    storage: &mut u64,
) -> Result<Plan<'a>, ProbabilityDocumentError> {
    let plan = plan(
        system,
        claim,
        document.payoff,
        limits,
        cancel,
        meter,
        storage,
    )?;
    if plan.objective != document.objective || document.conditional.is_some() {
        return Err(ProbabilityDocumentError::ObjectiveMismatch);
    }
    Ok(plan)
}

fn charge(storage: &mut u64, bound: u64, bytes: u64) -> Result<(), ProbabilityDocumentError> {
    let actual = storage
        .checked_add(bytes)
        .ok_or(ProbabilityDocumentError::KeySizeOverflow)?;
    if actual > bound {
        return Err(exceeded(
            ProbabilityDocumentLimit::StorageBytes,
            bound,
            actual,
        ));
    }
    *storage = actual;
    Ok(())
}

/// Resolve the shared terminal payoff, actual reward descriptor and operator
/// from the checked claim. Producers use this same selection as the checker;
/// only the graph construction and backward induction belong to the engine.
#[allow(clippy::too_many_arguments)]
pub fn measure_probability_objective(
    system: &ModelSystem<'_>,
    claim: &CheckedProbabilisticClaim,
    payoff: MeasurePayoffKind,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
    meter: &mut filament_exact::Meter,
    storage: &mut u64,
) -> Result<
    (
        MeasurePayoff,
        Option<ProbabilityRewardObjective>,
        ProbabilityObjective,
    ),
    ProbabilityDocumentError,
> {
    let selected = plan(system, claim, payoff, limits, cancel, meter, storage)?;
    Ok((
        selected.payoff,
        selected.objective.reward,
        selected.objective.optimization,
    ))
}

pub(in crate::probability_certificate) struct ReplayControl<'a> {
    pub(in crate::probability_certificate) control: MeasurePlan<'a>,
    pub(in crate::probability_certificate) predicates: Vec<&'a CheckedPredicate>,
    pub(in crate::probability_certificate) payoff: MeasurePayoff,
    pub(in crate::probability_certificate) reward: Option<ProbabilityRewardObjective>,
}
#[allow(clippy::too_many_arguments)]
pub(in crate::probability_certificate) fn replay_control<'a>(
    system: &ModelSystem<'_>,
    claim: &'a CheckedProbabilisticClaim,
    kind: MeasurePayoffKind,
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
    meter: &mut filament_exact::Meter,
    storage: &mut u64,
) -> Result<ReplayControl<'a>, ProbabilityDocumentError> {
    let plan = plan(system, claim, kind, limits, cancel, meter, storage)?;
    Ok(ReplayControl {
        control: plan.control,
        predicates: plan.predicates,
        payoff: plan.payoff,
        reward: plan.objective.reward,
    })
}

impl MeasurePayoffKind {
    /// Initialize the shared measure control against the admitted subject.
    /// `unread` preserves position-zero control before any predicate is read.
    ///
    /// # Errors
    /// Unsupported shape, undefined/refused predicates, or actual caller stops.
    #[allow(clippy::too_many_arguments)]
    pub fn initial_control(
        self,
        system: &ModelSystem<'_>,
        claim: &CheckedProbabilisticClaim,
        unread: bool,
        limits: ProbabilityDocumentLimits,
        cancel: &Cancel,
        meter: &mut filament_exact::Meter,
        storage: &mut u64,
    ) -> Result<(Option<MonitorState>, Option<Accumulator>), ProbabilityDocumentError> {
        let selected = replay_control(system, claim, self, limits, cancel, meter, storage)?;
        if unread {
            return selected
                .control
                .initial_control(meter)
                .map_err(ProbabilityDocumentError::from);
        }
        let binding = claim_binding(system, claim)?;
        let reference = binding.as_ref().map(|binding| &binding.reference);
        let initial = system.initial();
        let [initial] = initial.as_slice() else {
            return Err(ProbabilityDocumentError::Unsupported);
        };
        let inputs = selected
            .predicates
            .iter()
            .map(|predicate| (*predicate, reference))
            .collect::<Vec<_>>();
        let truths = system
            .state_predicates_with_cancel(initial, &inputs, cancel)
            .map_err(|error| {
                ProbabilityDocumentError::Build(Box::new(crate::ExactProbBuildIssue::Expansion(
                    error,
                )))
            })?
            .into_iter()
            .map(|reading| match reading {
                PredicateReading::True => Ok(true),
                PredicateReading::False => Ok(false),
                PredicateReading::Refused(outcome) => Err(ProbabilityDocumentError::Build(
                    Box::new(crate::ExactProbBuildIssue::Predicate(outcome)),
                )),
                PredicateReading::Undefined { cause, .. } => {
                    Err(ProbabilityDocumentError::UndefinedPredicate(cause))
                }
            })
            .collect::<Result<Vec<_>, _>>()?;
        selected
            .control
            .start(&truths, meter)
            .map_err(ProbabilityDocumentError::from)
    }
}

fn key_parse_error(
    error: quire_canonical::ReadError,
    identity: fsl_foundation::IdentityLimits,
) -> ModelKeyError {
    match error {
        quire_canonical::ReadError::ReservationDenied { .. } => ModelKeyError::Denied,
        quire_canonical::ReadError::Allocation { requested } => {
            ModelKeyError::Encoding(fsl_eval::simulation::EncodingRefusal::Allocation { requested })
        }
        quire_canonical::ReadError::Limit(limit) => {
            ModelKeyError::Encoding(fsl_eval::simulation::EncodingRefusal::Limit(
                fsl_foundation::diagnostic::LimitExceeded::new(
                    fsl_foundation::Setting::IdentityInputBytes,
                    identity.input_bytes,
                    u128::from(limit.required),
                ),
            ))
        }
        other => ModelKeyError::Encoding(fsl_eval::simulation::EncodingRefusal::NoEncoding(
            other.to_string(),
        )),
    }
}

/// Internal role result; it cannot be used as a proof acceptance token.
pub(in crate::probability_certificate) enum MeanRefutationCheck {
    Accepted,
    AcceptedBounds(Vec<(ProbabilityProductKey, crate::ProductAction)>),
    Rejected(ProbabilityCertificateRejection),
    PolicyParity,
}
#[allow(clippy::too_many_arguments)]
pub(in crate::probability_certificate) fn check_mean_refutation(
    system: &ModelSystem<'_>,
    claim: &CheckedProbabilisticClaim,
    item: &ObligationIdentity,
    bytes: &[u8],
    scheduler: &[(ProbabilityProductKey, crate::ProductAction)],
    limits: ProbabilityDocumentLimits,
    cancel: &Cancel,
) -> Result<MeanRefutationCheck, ProbabilityDocumentError> {
    poll(cancel)?;
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > limits.bytes {
        return Err(exceeded(
            ProbabilityDocumentLimit::Bytes,
            limits.bytes,
            u64::try_from(bytes.len()).unwrap_or(u64::MAX),
        ));
    }
    if bounds::is_bound(bytes)? {
        return bounds::check(system, claim, item, bytes, scheduler, limits, cancel);
    }
    let mut meter = filament_exact::Meter::new(limits.accounting).with_cancel(cancel.clone());
    let mut storage = 0;
    let document =
        decode_measure_probability_certificate(bytes, limits, cancel, &mut meter, &mut storage)?;
    if document.identity != *item {
        return Err(ProbabilityDocumentError::IdentityMismatch);
    }
    if document.conditional.is_some() || document.payoff != MeasurePayoffKind::Fraction {
        return Err(ProbabilityDocumentError::MalformedMember("kind"));
    }
    let selected = plan(
        system,
        claim,
        MeasurePayoffKind::Fraction,
        limits,
        cancel,
        &mut meter,
        &mut storage,
    )?;
    if selected.objective != document.objective {
        return Err(ProbabilityDocumentError::ObjectiveMismatch);
    }
    validate_ranges(&document, limits, cancel, &mut meter)?;
    let graph = enumerate(
        system,
        claim,
        &selected,
        limits,
        cancel,
        &mut meter,
        &mut storage,
        None,
    )?;
    let values = match checked_values(&document, &graph, limits, cancel, &mut meter)? {
        Ok(values) => values,
        Err(ExactProbabilityCertificateCheck::Rejected(rejection)) => {
            return Ok(MeanRefutationCheck::Rejected(rejection))
        }
        Err(_) => return Err(ProbabilityDocumentError::Malformed),
    };
    if selected.objective.optimization == ProbabilityObjective::Workload {
        if !scheduler.is_empty() {
            return Ok(MeanRefutationCheck::PolicyParity);
        }
    } else {
        let policy = scheduler
            .iter()
            .map(|(key, action)| (key, action))
            .collect::<BTreeMap<_, _>>();
        for (key, action) in scheduler {
            poll(cancel)?;
            let Some(state) = graph.states.get(key) else {
                return Ok(MeanRefutationCheck::PolicyParity);
            };
            let Some(index) = state
                .actions
                .iter()
                .position(|candidate| candidate.as_ref() == Some(action))
            else {
                return Ok(MeanRefutationCheck::PolicyParity);
            };
            if state.terminal.is_some()
                || action_sum(
                    state
                        .choices
                        .get(index)
                        .ok_or(ProbabilityDocumentError::Malformed)?,
                    &values,
                    &mut meter,
                    cancel,
                )? != *values.get(key).ok_or(ProbabilityDocumentError::Malformed)?
            {
                return Ok(MeanRefutationCheck::PolicyParity);
            }
        }
        let mut queue = VecDeque::from([graph.initial.clone()]);
        let mut seen = BTreeSet::from([graph.initial.clone()]);
        while let Some(key) = queue.pop_front() {
            poll(cancel)?;
            let state = graph
                .states
                .get(&key)
                .ok_or(ProbabilityDocumentError::Malformed)?;
            if state.terminal.is_some() {
                continue;
            }
            let index = if let Some(action) = policy.get(&key) {
                state
                    .actions
                    .iter()
                    .position(|candidate| candidate.as_ref() == Some(*action))
                    .ok_or(ProbabilityDocumentError::Malformed)?
            } else if state.choices.len() == 1 {
                0
            } else {
                return Ok(MeanRefutationCheck::PolicyParity);
            };
            for edge in state
                .choices
                .get(index)
                .ok_or(ProbabilityDocumentError::Malformed)?
            {
                if !edge.probability.is_zero() && seen.insert(edge.target.clone()) {
                    charge(
                        &mut storage,
                        limits.storage_bytes,
                        u64::try_from(edge.target.as_bytes().len().saturating_mul(2))
                            .unwrap_or(u64::MAX),
                    )?;
                    queue.push_back(edge.target.clone());
                }
            }
        }
    }
    let value = values
        .get(&graph.initial)
        .ok_or(ProbabilityDocumentError::Malformed)?;
    let refutes = match &selected.bound {
        Bound::AtLeast(threshold) => ordered(Order::Less, value, threshold, &mut meter, cancel)?,
        Bound::AtMost(threshold) => ordered(Order::Greater, value, threshold, &mut meter, cancel)?,
    };
    if !refutes {
        return Ok(MeanRefutationCheck::Rejected(
            ProbabilityCertificateRejection {
                rule: ProbabilityCertificateRule::ThresholdMissed,
                at: ProbabilityCertificateLocus::ProductState(graph.initial),
            },
        ));
    }
    poll(cancel)?;
    Ok(MeanRefutationCheck::Accepted)
}
