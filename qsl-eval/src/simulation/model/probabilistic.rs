// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-187: the probabilistic structure of a model state's successors.
//!
//! [`ModelSystem::actions`] groups FR-120's successors by **scheduled
//! identity** (ADR-024 PM-2: a transition identity without its random
//! arguments) and gives each scheduled identity its draws: every vector of
//! its random parameters' supports, with the product of their PM-1
//! probabilities, the post-states it reaches and its rewards (PM-5).
//! [`ModelSystem::weighted_steps`] resolves the scheduler by a workload
//! (PM-2) and gives each transition its exact step probability, or the first
//! scheduled identity and drawn vector that breaks the Markov condition
//! (PM-3). EN-4 (FR-189) reads this structure and does not compute it.
//!
//! A random parameter's support is its finite domain for enumeration, and a
//! random argument is part of the transition identity, so the support graph
//! is FR-120's state graph: every transition `weighted_steps` gives positive
//! probability is one of FR-120's successors and every such successor gets
//! one (PM-6).

use std::collections::BTreeMap;

use qsl_foundation::diagnostic::Code;
use qsl_foundation::digest::DigestRecord;
use qsl_semantics::check::{CheckedProbabilisticModel, Reward, Workload};
use qsl_semantics::model::observation::AdmissionFailure;
use quire_exact::{
    EffectiveId, Identifier, Integer, Meter, ObjectReference, Outcome, Quantity, Rational, Value,
    ValueType,
};
use quire_semantic_value::model_transition::ModelTransition;

use super::contracts::{verdict, Verdict};
use super::domain::{Domain, Odometer};
use super::encode::{value_enc, values_equal};
use super::frames::{identity_bytes, text_of};
use super::walk::{exhausted, expand_enabled, stop};
use super::{refused, ModelFinding, ModelSimState, ModelSystem, BROKEN};
use crate::simulation::expansion::{ExpansionLimit, ExpansionStop};
use crate::value::evaluate_reward;

/// A transition identity without its random arguments (ADR-024 PM-2): the
/// operation, the receiver and the non-random arguments.
#[derive(Clone, Debug)]
pub struct ScheduledIdentity {
    /// The operation's declaration identity text, as
    /// [`ModelTransition::operation`] carries it.
    pub operation: String,
    /// The receiver.
    pub receiver: ObjectReference,
    /// One value per non-random parameter, in declared order.
    pub arguments: Vec<Value>,
}

impl PartialEq for ScheduledIdentity {
    fn eq(&self, other: &Self) -> bool {
        self.operation == other.operation
            && self.receiver == other.receiver
            && same_values(&self.arguments, &other.arguments)
    }
}

/// One scheduled identity enabled at a state, with its draws (FR-187).
#[derive(Clone, Debug)]
pub struct Action {
    /// The scheduled identity.
    pub identity: ScheduledIdentity,
    /// One draw per vector of its random parameters' supports, in canonical
    /// order of the vectors; one draw with no random value for an operation
    /// with no random parameter.
    pub draws: Vec<Draw>,
}

/// One vector of a scheduled identity's random arguments (FR-187).
#[derive(Clone, Debug)]
pub struct Draw {
    /// One value per random parameter, in declared order.
    pub random: Vec<Value>,
    /// The product of the values' PM-1 probabilities: the draws of an
    /// action sum to exactly 1.
    pub probability: Rational,
    /// FR-120's post-states of the transition, in canonical order: exactly
    /// one when the Markov condition holds (PM-3).
    pub successors: Vec<(ModelTransition, ModelSimState)>,
    /// Every reward of the model, in ascending name order: the operation's
    /// own reward evaluated on the transition's arguments, or 0.
    pub rewards: Vec<(Identifier, Value)>,
}

/// The transition a weighted or sampled step takes: an operation's, or the
/// terminal stutter step (ADR-018 SM-4), which no operation takes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TransitionId {
    /// An application of an operation, by its FR-181 transition identity.
    Operation(ModelTransition),
    /// The terminal stutter step: the state repeats.
    Stutter,
}

/// One transition of a state under a workload, with its exact step
/// probability (FR-187, ADR-024 PM-3).
#[derive(Clone, Debug)]
pub struct WeightedStep {
    /// The transition.
    pub transition: TransitionId,
    /// The post-state.
    pub post: ModelSimState,
    /// The step probability: the operation's weight share, times one over
    /// its enabled scheduled identities, times the draw's probability.
    pub probability: Rational,
    /// The step's rewards, as [`Draw::rewards`]; every reward 0 on the
    /// stutter step.
    pub rewards: Vec<(Identifier, Value)>,
}

/// A scheduled identity and drawn vector with no post-state, or with
/// several under a workload: the subject is not a Markov chain at the state
/// (ADR-024 PM-3).
#[derive(Clone, Debug)]
pub struct NotMarkov {
    /// The state's `quire.simulation.state-key/v1` digest.
    pub state: DigestRecord,
    /// The scheduled identity.
    pub transition: ScheduledIdentity,
    /// The drawn random values, in declared order.
    pub random: Vec<Value>,
    /// How many post-states the drawn vector gives: 0, or more than 1.
    pub post_states: u32,
}

impl PartialEq for NotMarkov {
    fn eq(&self, other: &Self) -> bool {
        self.state == other.state
            && self.transition == other.transition
            && same_values(&self.random, &other.random)
            && self.post_states == other.post_states
    }
}

/// A contract conjunction of an application or a postcondition candidate
/// was undecided at the state (FR-120's `ContractUndetermined` finding), so
/// its successors, and its step probabilities, are not exactly known
/// (FR-187).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UndecidedSuccessor {
    /// The state's `quire.simulation.state-key/v1` digest.
    pub state: DigestRecord,
}

/// Why [`ModelSystem::weighted_steps`] gives a state no step probabilities
/// (FR-187).
#[derive(Clone, Debug, PartialEq)]
pub enum Unsteppable {
    /// A scheduled identity and drawn vector has no post-state or several.
    NotMarkov(NotMarkov),
    /// A contract conjunction was undecided at the state.
    UndecidedSuccessor(UndecidedSuccessor),
}

/// Whether two values are the same: equal typed canonical forms, or equal
/// quantities, which have no such form.
pub(crate) fn same_value(left: &Value, right: &Value) -> bool {
    match (left, right) {
        (Value::Quantity(left), Value::Quantity(right)) => {
            left.unit() == right.unit() && left.magnitude() == right.magnitude()
        }
        _ => values_equal(left, right),
    }
}

/// [`same_value`] over two lists, pairwise.
pub(crate) fn same_values(left: &[Value], right: &[Value]) -> bool {
    left.len() == right.len()
        && left
            .iter()
            .zip(right)
            .all(|(left, right)| same_value(left, right))
}

/// Two reward lists are the same: equal names and values, in order.
pub(crate) fn same_rewards(left: &[(Identifier, Value)], right: &[(Identifier, Value)]) -> bool {
    left.len() == right.len()
        && left
            .iter()
            .zip(right)
            .all(|((left, left_value), (right, right_value))| {
                left == right && same_value(left_value, right_value)
            })
}

/// A value's place in canonical value order: Booleans `false` before
/// `true`; integers, rationals, decimals and quantities (one parameter's
/// quantities share a unit) numerically; enum members by their canonical
/// rank; every other value by its typed canonical form's bytes. Values of
/// one type always have keys of one variant.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum OrderKey {
    Boolean(bool),
    Number(Rational),
    Rank(u32),
    Bytes(Vec<u8>),
}

/// `value`'s [`OrderKey`].
///
/// # Errors
///
/// `runtime_invariant`/`established-invariant-broken` when a value ordered
/// by its bytes has no typed canonical form.
pub(crate) fn order_key(value: &Value) -> Result<OrderKey, ExpansionStop> {
    Ok(match value {
        Value::Boolean(flag) => OrderKey::Boolean(*flag),
        Value::Integer(integer) => OrderKey::Number(Rational::from_integer(integer.clone())),
        Value::Rational(rational) => OrderKey::Number(rational.clone()),
        Value::Decimal(decimal) => OrderKey::Number(decimal.normalized().to_rational()),
        Value::Quantity(quantity) => OrderKey::Number(quantity.magnitude().clone()),
        Value::Enum(member) => OrderKey::Rank(member.rank()),
        _ => OrderKey::Bytes(
            quire_canonical::to_vec(&value_enc(value), quire_canonical::Limits::new(u64::MAX))
                .map_err(|_| stop(BROKEN))?,
        ),
    })
}

/// `items` sorted by `key`, every key computed first so the sort compares
/// total, infallible keys.
pub(crate) fn sorted_by_key<T, K: Ord>(
    items: Vec<T>,
    key: impl Fn(&T) -> Result<K, ExpansionStop>,
) -> Result<Vec<T>, ExpansionStop> {
    let mut keyed = items
        .into_iter()
        .map(|item| key(&item).map(|key| (key, item)))
        .collect::<Result<Vec<_>, _>>()?;
    keyed.sort_by(|left, right| left.0.cmp(&right.0));
    Ok(keyed.into_iter().map(|(_, item)| item).collect())
}

/// One value of a random parameter's support, in declared order.
pub(crate) struct SupportItem {
    /// The value.
    pub(crate) value: Value,
    /// Its declared weight.
    pub(crate) weight: Rational,
    /// Its PM-1 probability.
    pub(crate) probability: Rational,
}

/// A random parameter of an operation: its position among the operation's
/// parameters and its support.
pub(crate) struct RandomSupport {
    /// The parameter's 0-based position.
    pub(crate) position: usize,
    /// Its support, in declared order.
    pub(crate) items: Vec<SupportItem>,
}

impl RandomSupport {
    /// The support's values, in declared order.
    pub(super) fn values(&self) -> Vec<Value> {
        self.items.iter().map(|item| item.value.clone()).collect()
    }

    /// The probability of `value`, a value of the support.
    fn probability(&self, value: &Value) -> Option<&Rational> {
        self.items
            .iter()
            .find(|item| same_value(&item.value, value))
            .map(|item| &item.probability)
    }
}

/// What [`ModelSystem`] reads from FR-185's checked declarations.
#[derive(Default)]
pub(super) struct Tables {
    /// Each operation's random parameters, by (declaring type, name), in
    /// ascending position.
    random: BTreeMap<(EffectiveId, String), Vec<RandomSupport>>,
    /// Every reward name of the model, ascending, with the zero of its type
    /// (the type of its first declaration).
    names: Vec<(Identifier, Value)>,
}

impl Tables {
    /// The tables of `model`.
    pub(super) fn of(model: &CheckedProbabilisticModel) -> Result<Self, AdmissionFailure> {
        let mut random: BTreeMap<(EffectiveId, String), Vec<RandomSupport>> = BTreeMap::new();
        for parameter in model.random_parameters() {
            let operation = parameter.operation();
            random
                .entry((operation.declaring(), operation.name().to_owned()))
                .or_default()
                .push(RandomSupport {
                    position: parameter.position(),
                    items: parameter
                        .support()
                        .iter()
                        .map(|value| SupportItem {
                            value: value.value.0.clone(),
                            weight: value.weight.clone(),
                            probability: value.probability.clone(),
                        })
                        .collect(),
                });
        }
        for supports in random.values_mut() {
            supports.sort_by_key(|support| support.position);
        }
        let mut names: BTreeMap<String, Value> = BTreeMap::new();
        for reward in model.rewards() {
            if !names.contains_key(reward.name()) {
                let zero =
                    zero_of(reward.value_type()).ok_or_else(|| super::fault("reward-type"))?;
                names.insert(reward.name().to_owned(), zero);
            }
        }
        let names = names
            .into_iter()
            .map(|(name, zero)| {
                Identifier::new(name)
                    .map(|name| (name, zero))
                    .map_err(|_| super::fault("reward-name"))
            })
            .collect::<Result<_, _>>()?;
        Ok(Self { random, names })
    }

    /// The random parameter at `position` of the operation `name` that
    /// `declaring` declares, when it is one.
    pub(super) fn support(
        &self,
        declaring: EffectiveId,
        name: &str,
        position: usize,
    ) -> Option<&RandomSupport> {
        self.random_of(declaring, name)
            .iter()
            .find(|support| support.position == position)
    }

    /// Every random parameter of the operation, in ascending position.
    pub(crate) fn random_of(&self, declaring: EffectiveId, name: &str) -> &[RandomSupport] {
        self.random
            .get(&(declaring, name.to_owned()))
            .map_or(&[], Vec::as_slice)
    }
}

/// The zero of a reward type: `0` of an integer type, `0` of a quantity's
/// unit.
fn zero_of(value_type: &ValueType) -> Option<Value> {
    match value_type {
        ValueType::Integer | ValueType::Int(_) => Some(Value::Integer(Integer::zero())),
        ValueType::Quantity(unit) => Some(Value::Quantity(Quantity::new(
            Rational::from_integer(Integer::zero()),
            *unit,
        ))),
        _ => None,
    }
}

/// `value` as a record field renders it: an integer in decimal, a Boolean
/// as `true` or `false`, anything else by its typed canonical text.
fn rendered_value(value: &Value) -> Result<String, ExpansionStop> {
    match value {
        Value::Integer(integer) => Ok(integer.to_string()),
        Value::Boolean(flag) => Ok(flag.to_string()),
        _ => text_of(value),
    }
}

/// Whether a reward value is below zero.
fn is_negative(value: &Value) -> bool {
    match value {
        Value::Integer(integer) => integer.is_negative(),
        Value::Quantity(quantity) => quantity.magnitude().numerator().is_negative(),
        _ => false,
    }
}

impl ModelSystem<'_> {
    /// The checked declarations this system reads.
    fn declarations(&self) -> &CheckedProbabilisticModel {
        self.package.graph().probabilistic_model()
    }

    /// The operation whose declaration identity text is `text`: its
    /// declaring type and name.
    pub(crate) fn operation_of_text(&self, text: &str) -> Option<(EffectiveId, &str)> {
        self.applications
            .values()
            .flatten()
            .find(|application| application.text == text)
            .map(|application| (application.declaring, application.name.as_str()))
    }

    /// The random parameters of the operation whose identity text is `text`,
    /// in ascending position.
    pub(crate) fn random_of_text(&self, text: &str) -> &[RandomSupport] {
        match self.operation_of_text(text) {
            Some((declaring, name)) => self.probabilistic.random_of(declaring, name),
            None => &[],
        }
    }

    /// The reward `reward` evaluated on `arguments`.
    fn reward_value(&self, reward: &Reward, arguments: Vec<Value>) -> Result<Value, ExpansionStop> {
        let evaluation =
            evaluate_reward(self.package, reward, arguments, &mut Meter::new(self.meter))
                .map_err(|_| stop(BROKEN))?;
        if let qsl_semantics::family::FamilyOutcome::Evaluated(Outcome::Completed(value)) =
            &evaluation.outcome
        {
            return Ok(value.clone());
        }
        match verdict(&evaluation.outcome) {
            Verdict::Undecided(outcome) if outcome.incomplete_code().is_some() => {
                Err(exhausted(ExpansionLimit::EvaluationMeter))
            }
            _ => Err(stop(BROKEN)),
        }
    }

    /// Every reward of the model for a transition of the operation `name`
    /// that `declaring` declares with `arguments`, in ascending name order:
    /// the operation's reward of that name, or 0 (FR-187).
    fn rewards_of(
        &self,
        declaring: EffectiveId,
        name: &str,
        arguments: &[(String, Value)],
    ) -> Result<Vec<(Identifier, Value)>, ExpansionStop> {
        self.probabilistic
            .names
            .iter()
            .map(|(reward_name, zero)| {
                let declared = self.declarations().rewards().iter().find(|reward| {
                    reward.operation().declaring() == declaring
                        && reward.operation().name() == name
                        && reward.name() == reward_name.as_str()
                });
                let value = match declared {
                    Some(reward) => self.reward_value(
                        reward,
                        arguments.iter().map(|(_, value)| value.clone()).collect(),
                    )?,
                    None => zero.clone(),
                };
                Ok((reward_name.clone(), value))
            })
            .collect()
    }

    /// Every reward of the model at 0: the stutter step's rewards.
    pub(crate) fn zero_rewards(&self) -> Vec<(Identifier, Value)> {
        self.probabilistic.names.clone()
    }

    /// FR-187 admission: every reward evaluated on every argument vector of
    /// its operation's finite parameter domains, in canonical order (the
    /// first parameter most significant), refusing the first negative value
    /// with `invalid_probabilistic_model`/`negative-reward` naming the
    /// operation, the reward and the arguments. A random parameter ranges
    /// over its support; a parameter of a type with no finite domain here,
    /// or of an object type (a reward reads no state, so no object), leaves
    /// its reward unchecked. The vectors count against the expansion's
    /// candidate bound.
    pub(super) fn check_rewards(&self) -> Result<(), AdmissionFailure> {
        let mut budget = 0u64;
        for reward in self.declarations().rewards() {
            let operation = reward.operation();
            let domains: Option<Vec<Domain>> = reward
                .parameters()
                .iter()
                .enumerate()
                .map(|(position, (_, value_type))| {
                    match self.probabilistic.support(
                        operation.declaring(),
                        operation.name(),
                        position,
                    ) {
                        Some(support) => Some(Domain::Listed(support.values())),
                        None => Domain::of(value_type, &[], self.types),
                    }
                })
                .collect();
            let Some(domains) = domains else { continue };
            let mut vectors = Odometer::new(domains.iter().map(Domain::len).collect());
            while let Some(digits) = vectors.next_digits() {
                budget = budget.saturating_add(1);
                if budget > self.limits.max_candidates {
                    return Err(AdmissionFailure::Incomplete(super::record(
                        Code::ResourceExhausted,
                        "insufficient-next-charge",
                        &[("limit", "max_candidates".to_owned())],
                    )));
                }
                let arguments = domains
                    .iter()
                    .zip(&digits)
                    .map(|(domain, digit)| domain.nth(*digit))
                    .collect::<Option<Vec<Value>>>()
                    .ok_or_else(|| super::fault("reward-domain"))?;
                // A reward that does not complete on a vector refuses
                // admission: its sign there is unknown.
                let value =
                    self.reward_value(reward, arguments.clone())
                        .map_err(|stop| match stop.limit {
                            Some(ExpansionLimit::EvaluationMeter) => {
                                AdmissionFailure::Incomplete(super::record(
                                    Code::ResourceExhausted,
                                    "insufficient-next-charge",
                                    &[("limit", "evaluation_meter".to_owned())],
                                ))
                            }
                            _ => super::fault("reward-evaluation"),
                        })?;
                if is_negative(&value) {
                    let rendered = reward
                        .parameters()
                        .iter()
                        .zip(&arguments)
                        .map(|((name, _), value)| {
                            rendered_value(value).map(|text| format!("{name} = {text}"))
                        })
                        .collect::<Result<Vec<_>, _>>()
                        .map_err(|_| super::fault("reward-argument-text"))?;
                    let (code, cause) = NEGATIVE_REWARD;
                    return Err(refused(
                        code,
                        cause,
                        &[
                            ("operation", operation.key().node.clone()),
                            ("reward", reward.name().to_owned()),
                            ("arguments", rendered.join(", ")),
                        ],
                    ));
                }
            }
        }
        Ok(())
    }

    /// FR-187: each scheduled identity whose effective precondition holds at
    /// `state` and that has a post-state for some drawn vector, in
    /// canonical transition order of the scheduled identities, with every
    /// draw of its random parameters' supports in canonical order. A draw
    /// with no post-state carries none, for the caller to report as
    /// [`NotMarkov`]; one with several carries them all, for a scheduler to
    /// choose among.
    ///
    /// A random parameter is read by no precondition (FR-185), so a
    /// scheduled identity's precondition holds for every draw or for none.
    ///
    /// `Ok(Err(UndecidedSuccessor))` when a precondition or a postcondition
    /// candidate of the state's expansion does not decide: the expansion
    /// runs before any draw is judged, so this comes before [`NotMarkov`].
    ///
    /// # Errors
    ///
    /// The [`ExpansionStop`] FR-120's expansion stops with;
    /// `resource_exhausted`/`insufficient-next-charge` when a reward
    /// evaluation exhausts the meter.
    pub fn actions(
        &self,
        state: &ModelSimState,
    ) -> Result<Result<Vec<Action>, UndecidedSuccessor>, ExpansionStop> {
        Ok(self.enabled_actions(state)?.map(|mut actions| {
            actions.retain(|action| action.draws.iter().any(|draw| !draw.successors.is_empty()));
            actions
        }))
    }

    /// [`Self::actions`] with every enabled scheduled identity, those with no
    /// post-state for any draw included: a state with none is terminal
    /// (FR-124), and a drawn vector with no post-state is [`NotMarkov`].
    pub(crate) fn enabled_actions(
        &self,
        state: &ModelSimState,
    ) -> Result<Result<Vec<Action>, UndecidedSuccessor>, ExpansionStop> {
        let (expansion, enabled) = expand_enabled(self, state)?;
        if expansion
            .findings
            .iter()
            .any(|finding| matches!(finding, ModelFinding::ContractUndetermined { .. }))
        {
            let (_, digest) = self.state_key(state).map_err(|_| stop(BROKEN))?;
            return Ok(Err(UndecidedSuccessor { state: digest }));
        }
        let mut successors: Vec<Option<(ModelTransition, ModelSimState)>> =
            expansion.successors.into_iter().map(Some).collect();
        let mut groups: Vec<(Vec<u8>, Action)> = Vec::new();
        for vector in enabled {
            let supports = self.probabilistic.random_of(vector.declaring, &vector.name);
            let mut random = Vec::with_capacity(supports.len());
            let mut probability = Rational::from_integer(Integer::one());
            for support in supports {
                let (_, value) = vector
                    .args
                    .get(support.position)
                    .ok_or_else(|| stop(BROKEN))?;
                probability =
                    probability.mul(support.probability(value).ok_or_else(|| stop(BROKEN))?);
                random.push(value.clone());
            }
            let arguments: Vec<Value> = vector
                .args
                .iter()
                .enumerate()
                .filter(|(position, _)| {
                    supports.iter().all(|support| support.position != *position)
                })
                .map(|(_, (_, value))| value.clone())
                .collect();
            let order = identity_bytes(&vector.operation, &vector.receiver, &arguments)?;
            let mut reached = Vec::with_capacity(vector.successors.len());
            for index in vector.successors.clone() {
                let successor = successors
                    .get_mut(index)
                    .and_then(Option::take)
                    .ok_or_else(|| stop(BROKEN))?;
                let (key, _) = self.state_key(&successor.1).map_err(|_| stop(BROKEN))?;
                reached.push((key, successor));
            }
            reached.sort_by(|left, right| left.0.cmp(&right.0));
            let draw = Draw {
                random,
                probability,
                successors: reached
                    .into_iter()
                    .map(|(_, successor)| successor)
                    .collect(),
                rewards: self.rewards_of(vector.declaring, &vector.name, &vector.args)?,
            };
            match groups.iter_mut().find(|(key, _)| *key == order) {
                Some((_, action)) => action.draws.push(draw),
                None => groups.push((
                    order,
                    Action {
                        identity: ScheduledIdentity {
                            operation: vector.operation,
                            receiver: vector.receiver,
                            arguments,
                        },
                        draws: vec![draw],
                    },
                )),
            }
        }
        groups.sort_by(|left, right| left.0.cmp(&right.0));
        groups
            .into_iter()
            .map(|(_, mut action)| {
                // A vector's key is its values' keys, the first most
                // significant.
                action.draws = sorted_by_key(action.draws, |draw| {
                    draw.random
                        .iter()
                        .map(order_key)
                        .collect::<Result<Vec<_>, _>>()
                })?;
                Ok(action)
            })
            .collect::<Result<_, _>>()
            .map(Ok)
    }

    /// FR-187: every transition of `state` with its exact step probability
    /// under `workload` (ADR-024 PM-2, PM-3): operation `o` with probability
    /// `w(o) / Σ w(O(s))` over the enabled operations, each of its enabled
    /// scheduled identities `1 / |E_o(s)|`, and each draw its probability.
    /// The probabilities sum to exactly 1, in canonical transition order. At
    /// a terminal state (no enabled scheduled identity) the only step is the
    /// stutter step, with probability 1 and every reward 0.
    ///
    /// Distinct draws and distinct scheduled identities have distinct
    /// transition identities, so no two steps share a transition and a
    /// post-state.
    ///
    /// # Errors
    ///
    /// `Ok(Err(Unsteppable::UndecidedSuccessor))` as [`Self::actions`];
    /// otherwise `Ok(Err(Unsteppable::NotMarkov))` for the first enabled
    /// scheduled identity and drawn vector, in canonical order, with no
    /// post-state or several. As
    /// [`Self::actions`]; `runtime_invariant`/`established-invariant-broken`
    /// when `workload` gives no weight to an enabled operation, which a
    /// workload of another model's subject does.
    pub fn weighted_steps(
        &self,
        state: &ModelSimState,
        workload: &Workload,
    ) -> Result<Result<Vec<WeightedStep>, Unsteppable>, ExpansionStop> {
        let actions = match self.enabled_actions(state)? {
            Ok(actions) => actions,
            Err(undecided) => return Ok(Err(Unsteppable::UndecidedSuccessor(undecided))),
        };
        if actions.is_empty() {
            return Ok(Ok(vec![WeightedStep {
                transition: TransitionId::Stutter,
                post: state.clone(),
                probability: Rational::from_integer(Integer::one()),
                rewards: self.zero_rewards(),
            }]));
        }
        let (_, digest) = self.state_key(state).map_err(|_| stop(BROKEN))?;
        for action in &actions {
            for draw in &action.draws {
                if draw.successors.len() != 1 {
                    return Ok(Err(Unsteppable::NotMarkov(NotMarkov {
                        state: digest,
                        transition: action.identity.clone(),
                        random: draw.random.clone(),
                        post_states: u32::try_from(draw.successors.len()).unwrap_or(u32::MAX),
                    })));
                }
            }
        }
        let mut operations: BTreeMap<String, (Rational, u64)> = BTreeMap::new();
        for action in &actions {
            let operation = &action.identity.operation;
            let entry = match operations.get_mut(operation) {
                Some(entry) => entry,
                None => {
                    let weight = self.weight_of(workload, operation)?;
                    operations.entry(operation.clone()).or_insert((weight, 0))
                }
            };
            entry.1 += 1;
        }
        let total = operations.values().fold(
            Rational::from_integer(Integer::zero()),
            |total, (weight, _)| total.add(weight),
        );
        let mut steps: Vec<(Vec<u8>, WeightedStep)> = Vec::new();
        for action in actions {
            let (weight, identities) = operations
                .get(action.identity.operation.as_str())
                .ok_or_else(|| stop(BROKEN))?;
            let share = weight
                .div(&total)
                .and_then(|share| share.div(&Rational::from_integer(Integer::from(*identities))))
                .ok_or_else(|| stop(BROKEN))?;
            for draw in action.draws {
                let [(transition, post)] =
                    <[_; 1]>::try_from(draw.successors).map_err(|_| stop(BROKEN))?;
                let bytes = self.transition_key(&transition).map_err(|_| stop(BROKEN))?;
                steps.push((
                    bytes,
                    WeightedStep {
                        transition: TransitionId::Operation(transition),
                        post,
                        probability: share.mul(&draw.probability),
                        rewards: draw.rewards,
                    },
                ));
            }
        }
        steps.sort_by(|left, right| left.0.cmp(&right.0));
        Ok(Ok(steps.into_iter().map(|(_, step)| step).collect()))
    }

    /// `workload`'s weight for the operation whose identity text is
    /// `operation`.
    pub(crate) fn weight_of(
        &self,
        workload: &Workload,
        operation: &str,
    ) -> Result<Rational, ExpansionStop> {
        workload
            .weights()
            .iter()
            .find(|(weighted, _)| {
                self.operation_text(weighted.declaring(), weighted.name()) == Some(operation)
            })
            .map(|(_, weight)| weight.clone())
            .ok_or_else(|| stop(BROKEN))
    }
}

/// The catalog code and cause of a negative reward (FR-187): a model whose
/// reward is negative is an invalid probabilistic model.
pub(crate) const NEGATIVE_REWARD: (Code, &str) =
    (Code::InvalidProbabilisticModel, "negative-reward");

#[cfg(test)]
mod tests;
