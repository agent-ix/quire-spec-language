// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-188: sampling one behaviour of a probabilistic model under a workload
//! with the `quire.simulation.sampler/v1` generator (QSpec
//! `proposals/quire-v1/definitions/simulation-sampler.md`, "Choices within a
//! step").
//!
//! Each step makes its choices in order: the operation among the enabled
//! operations by the workload's weights, in canonical operation order; the
//! scheduled identity uniformly among the operation's, in canonical
//! transition order; then one weighted choice per random parameter, in
//! declared order, over its support in canonical value order. A choice over
//! one item selects it with no digest and takes no choice index; every other
//! choice takes the step's next choice index, from 0. The step taken is the
//! one FR-187 gives for the drawn identity and vector.

use std::collections::BTreeMap;
use std::rc::Rc;

use qsl_foundation::digest::DigestRecord;
use qsl_foundation::selection::DefinitionRef;
use qsl_semantics::check::Workload;
use quire_exact::{Cancel, CancelCause, Identifier, Integer, Rational, Value};

use crate::simulation::choice::{
    weighted_choice_counted, ChoicePoint, TotalAboveDrawRange, WeightsRefusal,
};
use crate::simulation::expansion::ExpansionStop;
use crate::simulation::model::probabilistic::{
    order_key, same_rewards, same_values, sorted_by_key,
};
use crate::simulation::model::{
    Action, ModelSimState, ModelSystem, NotMarkov, TransitionId, BROKEN,
};
use crate::simulation::sample::is_pinned_sampler;

mod replay;

pub use replay::{
    replay_sampled, SampledDisagreement, SampledRecord, SampledReplay, SampledReplayRefusal,
};

/// One step of a sampled behaviour (FR-188).
#[derive(Clone, Debug)]
pub struct SampledStep {
    /// The transition taken: an operation's, or the terminal stutter step.
    pub transition: TransitionId,
    /// The post-state's `quire.simulation.state-key/v1` digest.
    pub post_digest: DigestRecord,
    /// The selected item index of each choice that drew, in choice-index
    /// order; a one-item choice records none.
    pub choices: Vec<u32>,
    /// The drawn value of each random parameter, one-item supports
    /// included, in declared order.
    pub random: Vec<Value>,
    /// The exact step probability FR-187 gives the transition.
    pub probability: Rational,
    /// The step's rewards, in ascending name order; every reward 0 on the
    /// stutter step.
    pub rewards: Vec<(Identifier, Value)>,
}

impl PartialEq for SampledStep {
    fn eq(&self, other: &Self) -> bool {
        self.transition == other.transition
            && self.post_digest == other.post_digest
            && self.choices == other.choices
            && same_values(&self.random, &other.random)
            && self.probability == other.probability
            && same_rewards(&self.rewards, &other.rewards)
    }
}

impl Eq for SampledStep {}

/// One sampled behaviour: a start state and its steps (FR-188).
#[derive(Clone, Debug, PartialEq)]
pub struct SampledBehaviour {
    /// The trace index the behaviour was drawn at.
    pub trace: u64,
    /// The start state's `quire.simulation.state-key/v1` digest.
    pub start: DigestRecord,
    /// One step per position after the first.
    pub steps: Vec<SampledStep>,
}

/// Why [`sample_probabilistic`] returned no behaviour (FR-188).
#[derive(Clone, Debug, PartialEq, thiserror::Error)]
pub enum SampleStop {
    /// The sampler is not `quire.simulation.sampler/v1`: FR-101's
    /// `NotSimulated::GeneratorMismatch`, before any draw.
    #[error("sampler {supplied:?} is not quire.simulation.sampler/v1")]
    GeneratorMismatch {
        /// The sampler supplied.
        supplied: DefinitionRef,
    },
    /// A state's expansion stopped.
    #[error("the expansion stopped: {0:?}")]
    Expansion(ExpansionStop),
    /// The drawn identity and vector give no post-state or several.
    #[error("the drawn step is not Markov: {0:?}")]
    NotMarkov(NotMarkov),
    /// A contract conjunction was undecided during a sampled step (FR-187);
    /// FR-189 settles it as an undecided successor.
    #[error("a contract conjunction was undecided during a sampled step")]
    UndecidedSuccessor,
    /// A choice's scaled total exceeds `2^256`.
    #[error(transparent)]
    TotalAboveDrawRange(TotalAboveDrawRange),
    /// The cancellation handle was cancelled (FR-276).
    #[error("cancelled: {0:?}")]
    Cancelled(CancelCause),
}

impl From<ExpansionStop> for SampleStop {
    fn from(stop: ExpansionStop) -> Self {
        Self::Expansion(stop)
    }
}

/// `runtime_invariant`/`established-invariant-broken`: the sampler read a
/// structure FR-187 built that broke its own invariant.
fn broken() -> SampleStop {
    SampleStop::Expansion(ExpansionStop {
        cause: BROKEN,
        limit: None,
    })
}

/// The draws of one step: its seed, trace and step, the next choice index,
/// the selections recorded and the digests computed.
struct StepDraws {
    seed: u64,
    trace: u64,
    step: u64,
    choice: u32,
    choices: Vec<u32>,
    digests: u64,
}

impl StepDraws {
    /// The index `weights` selects: 0 with no digest for one item; otherwise
    /// a draw at the next choice index, recorded.
    #[allow(
        clippy::result_large_err,
        reason = "FR-188 pins SampleStop::NotMarkov(NotMarkov), whose scheduled identity carries a receiver reference"
    )]
    fn choose(&mut self, weights: &[Rational]) -> Result<usize, SampleStop> {
        if weights.len() == 1 {
            return Ok(0);
        }
        let at = ChoicePoint {
            seed: self.seed,
            trace: self.trace,
            step: self.step,
            choice: self.choice,
        };
        let (selected, digests) = weighted_choice_counted(at, weights);
        self.digests = self.digests.saturating_add(digests);
        let index = selected.map_err(|refusal| match refusal {
            WeightsRefusal::TotalAboveDrawRange(total) => SampleStop::TotalAboveDrawRange(total),
            // Checked weights are positive and every choice is over an
            // enabled, non-empty set.
            WeightsRefusal::Empty | WeightsRefusal::NotPositive { .. } => broken(),
        })?;
        self.choice = self.choice.checked_add(1).ok_or_else(broken)?;
        self.choices
            .push(u32::try_from(index).map_err(|_| broken())?);
        Ok(index)
    }
}

/// [`sample_probabilistic`]'s inputs other than the system.
pub(crate) struct Sampling<'r> {
    pub(crate) workload: &'r Workload,
    pub(crate) start: &'r ModelSimState,
    pub(crate) sampler: &'r DefinitionRef,
    pub(crate) seed: u64,
    pub(crate) trace: u64,
    pub(crate) positions: u64,
    pub(crate) cancel: &'r Cancel,
}

/// [`sample_probabilistic`], with how many draw digests the run computed.
pub(crate) fn sample_counted(
    system: &ModelSystem<'_>,
    request: &Sampling<'_>,
) -> (Result<SampledBehaviour, SampleStop>, u64) {
    let mut sampler = match ProbabilisticSampler::new(system, request.workload, request.sampler) {
        Ok(sampler) => sampler,
        Err(stop) => return (Err(stop), 0),
    };
    let result = sample(&mut sampler, request);
    (result, sampler.digests)
}

#[allow(
    clippy::result_large_err,
    reason = "FR-188 pins SampleStop::NotMarkov(NotMarkov), whose scheduled identity carries a receiver reference"
)]
fn sample(
    sampler: &mut ProbabilisticSampler<'_, '_>,
    request: &Sampling<'_>,
) -> Result<SampledBehaviour, SampleStop> {
    let mut current = sampler.position(request.start.clone())?;
    let start = current.digest;
    let mut steps = Vec::new();
    for step in 0..request.positions.saturating_sub(1) {
        if let Some(cause) = request.cancel.cause() {
            return Err(SampleStop::Cancelled(cause));
        }
        let at = StepAt {
            seed: request.seed,
            trace: request.trace,
            step,
        };
        let (post, sampled) = sampler.step(&current, at)?;
        current = post;
        steps.push(sampled);
    }
    Ok(SampledBehaviour {
        trace: request.trace,
        start,
        steps,
    })
}

/// Where a step of a sampled behaviour draws (FR-188): the run's seed, the
/// behaviour's trace index and the step's 0-based index.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct StepAt {
    /// The run's seed.
    pub seed: u64,
    /// The behaviour's trace index.
    pub trace: u64,
    /// The step's index: the step from position `step` to `step + 1`.
    pub step: u64,
}

/// One position of a sampled behaviour: its state and that state's
/// `quire.simulation.state-key/v1` key and digest.
#[derive(Clone, Debug)]
pub struct SampledPosition {
    state: ModelSimState,
    key: Vec<u8>,
    digest: DigestRecord,
}

impl SampledPosition {
    /// The position's state.
    pub fn state(&self) -> &ModelSimState {
        &self.state
    }

    /// The state's canonical state-key bytes: two positions hold the same
    /// state exactly when their keys are equal.
    pub fn key(&self) -> &[u8] {
        &self.key
    }

    /// The state's state-key digest.
    pub fn digest(&self) -> DigestRecord {
        self.digest
    }
}

/// FR-188's sampler over one system and workload, one step at a time
/// (FR-189, FR-190: a caller that evaluates a claim position by position
/// draws only the steps it reads). It keeps each state's enabled actions,
/// keyed by state key, so a run that revisits a state expands it once; a
/// step is a function of the system, the workload, the position and its
/// [`StepAt`] alone, as [`sample_probabilistic`]'s are.
pub struct ProbabilisticSampler<'s, 'a> {
    system: &'s ModelSystem<'a>,
    workload: &'s Workload,
    /// Each state's enabled actions, or that a contract conjunction was
    /// undecided there, by state key. It grows with the states a run visits;
    /// no FR-191 budget bounds it, a known memory cost over a large
    /// reachable space.
    actions: BTreeMap<Vec<u8>, Option<Rc<[Action]>>>,
    /// The draw digests computed.
    digests: u64,
}

impl<'s, 'a> ProbabilisticSampler<'s, 'a> {
    /// A sampler of `system` under `workload` with the generator `sampler`
    /// names.
    ///
    /// # Errors
    ///
    /// [`SampleStop::GeneratorMismatch`] for a sampler other than
    /// `quire.simulation.sampler/v1`, before any draw.
    #[allow(
        clippy::result_large_err,
        reason = "FR-188 pins SampleStop::NotMarkov(NotMarkov), whose scheduled identity carries a receiver reference"
    )]
    pub fn new(
        system: &'s ModelSystem<'a>,
        workload: &'s Workload,
        sampler: &DefinitionRef,
    ) -> Result<Self, SampleStop> {
        if !is_pinned_sampler(sampler) {
            return Err(SampleStop::GeneratorMismatch {
                supplied: sampler.clone(),
            });
        }
        Ok(Self {
            system,
            workload,
            actions: BTreeMap::new(),
            digests: 0,
        })
    }

    /// `state` as a position of a behaviour.
    ///
    /// # Errors
    ///
    /// `runtime_invariant`/`established-invariant-broken` when the state's
    /// key does not encode.
    #[allow(
        clippy::result_large_err,
        reason = "FR-188 pins SampleStop::NotMarkov(NotMarkov), whose scheduled identity carries a receiver reference"
    )]
    pub fn position(&self, state: ModelSimState) -> Result<SampledPosition, SampleStop> {
        let (key, digest) = self.system.state_key(&state).map_err(|_| broken())?;
        Ok(SampledPosition { state, key, digest })
    }

    /// `from`'s enabled actions, expanded once per state key.
    #[allow(
        clippy::result_large_err,
        reason = "FR-188 pins SampleStop::NotMarkov(NotMarkov), whose scheduled identity carries a receiver reference"
    )]
    fn actions(&mut self, from: &SampledPosition) -> Result<Rc<[Action]>, SampleStop> {
        let actions = match self.actions.get(&from.key) {
            Some(known) => known.clone(),
            None => {
                let actions = self.system.enabled_actions(&from.state)?.ok().map(Rc::from);
                self.actions.insert(from.key.clone(), actions.clone());
                actions
            }
        };
        actions.ok_or(SampleStop::UndecidedSuccessor)
    }

    /// Whether `from` is terminal: no action is enabled there, so its step
    /// is the stutter step (ADR-018 SM-4).
    ///
    /// # Errors
    ///
    /// As [`Self::step`]: an expansion stop, or `UndecidedSuccessor` when a
    /// contract conjunction is undecided at the state.
    #[allow(
        clippy::result_large_err,
        reason = "FR-188 pins SampleStop::NotMarkov(NotMarkov), whose scheduled identity carries a receiver reference"
    )]
    pub fn is_terminal(&mut self, from: &SampledPosition) -> Result<bool, SampleStop> {
        Ok(self.actions(from)?.is_empty())
    }

    /// The step from `from` that the draws at `at` select, and the position
    /// it reaches. A terminal state takes the stutter step (ADR-018 SM-4).
    ///
    /// # Errors
    ///
    /// As [`sample_probabilistic`], but for `GeneratorMismatch` and
    /// `Cancelled`.
    #[allow(
        clippy::result_large_err,
        reason = "FR-188 pins SampleStop::NotMarkov(NotMarkov), whose scheduled identity carries a receiver reference"
    )]
    pub fn step(
        &mut self,
        from: &SampledPosition,
        at: StepAt,
    ) -> Result<(SampledPosition, SampledStep), SampleStop> {
        let actions = self.actions(from)?;
        if actions.is_empty() {
            return Ok((
                from.clone(),
                SampledStep {
                    transition: TransitionId::Stutter,
                    post_digest: from.digest,
                    choices: Vec::new(),
                    random: Vec::new(),
                    probability: Rational::from_integer(Integer::one()),
                    rewards: self.system.zero_rewards(),
                },
            ));
        }
        let mut draws = StepDraws {
            seed: at.seed,
            trace: at.trace,
            step: at.step,
            choice: 0,
            choices: Vec::new(),
            digests: 0,
        };
        let taken = take_step(
            self.system,
            self.workload,
            &actions,
            &from.digest,
            &mut draws,
        );
        self.digests = self.digests.saturating_add(draws.digests);
        let taken = taken?;
        let position = self.position(taken.post)?;
        let sampled = SampledStep {
            transition: taken.transition,
            post_digest: position.digest,
            choices: draws.choices,
            random: taken.random,
            probability: taken.probability,
            rewards: taken.rewards,
        };
        Ok((position, sampled))
    }
}

/// A step [`take_step`] selected, before its post-state is keyed.
struct Taken {
    post: ModelSimState,
    transition: TransitionId,
    random: Vec<Value>,
    probability: Rational,
    rewards: Vec<(Identifier, Value)>,
}

/// The step `draws` selects among `actions`, the enabled actions of the
/// state whose digest is `state`.
#[allow(
    clippy::result_large_err,
    reason = "FR-188 pins SampleStop::NotMarkov(NotMarkov), whose scheduled identity carries a receiver reference"
)]
fn take_step(
    system: &ModelSystem<'_>,
    workload: &Workload,
    actions: &[Action],
    state: &DigestRecord,
    draws: &mut StepDraws,
) -> Result<Taken, SampleStop> {
    // The enabled operations in canonical operation order (ascending
    // declaration identity text), each with its weight and its actions in
    // canonical transition order.
    let mut texts: Vec<&str> = system
        .operations()
        .into_iter()
        .map(|(_, _, text)| text)
        .collect();
    texts.sort_unstable();
    texts.dedup();
    let mut operations: Vec<(&str, Rational, Vec<&Action>)> = Vec::new();
    for text in texts {
        let enabled: Vec<&Action> = actions
            .iter()
            .filter(|action| action.identity.operation == text)
            .collect();
        if !enabled.is_empty() {
            let weight = system.weight_of(workload, text)?;
            operations.push((text, weight, enabled));
        }
    }
    let weights: Vec<Rational> = operations
        .iter()
        .map(|(_, weight, _)| weight.clone())
        .collect();
    let total = weights
        .iter()
        .fold(Rational::from_integer(Integer::zero()), |total, weight| {
            total.add(weight)
        });
    let (operation, weight, enabled) =
        operations.get(draws.choose(&weights)?).ok_or_else(broken)?;
    let unit = vec![Rational::from_integer(Integer::one()); enabled.len()];
    let action = enabled.get(draws.choose(&unit)?).ok_or_else(broken)?;

    let mut random = Vec::new();
    for support in system.random_of_text(operation) {
        let items = sorted_by_key(support.items.iter().collect(), |item| {
            order_key(&item.value)
        })?;
        let weights: Vec<Rational> = items.iter().map(|item| item.weight.clone()).collect();
        let item = items.get(draws.choose(&weights)?).ok_or_else(broken)?;
        random.push(item.value.clone());
    }
    let draw = action
        .draws
        .iter()
        .find(|draw| same_values(&draw.random, &random))
        .ok_or_else(broken)?;
    let [(transition, post)] =
        <[_; 1]>::try_from(draw.successors.clone()).map_err(|successors: Vec<_>| {
            SampleStop::NotMarkov(NotMarkov {
                state: *state,
                transition: action.identity.clone(),
                random: random.clone(),
                post_states: u32::try_from(successors.len()).unwrap_or(u32::MAX),
            })
        })?;
    let identities = Rational::from_integer(Integer::from(
        u64::try_from(enabled.len()).map_err(|_| broken())?,
    ));
    let probability = weight
        .div(&total)
        .and_then(|share| share.div(&identities))
        .ok_or_else(broken)?
        .mul(&draw.probability);
    Ok(Taken {
        post,
        transition: TransitionId::Operation(transition),
        random: draw.random.clone(),
        probability,
        rewards: draw.rewards.clone(),
    })
}

/// FR-188: sample one behaviour of `system` under `workload` from `start`,
/// with no draw selecting the start, over `positions` positions (so
/// `positions − 1` steps), drawing at trace index `trace` under `seed` with
/// the generator `sampler` names. A terminal state continues on stutter
/// steps (ADR-018 SM-4). The behaviour is a function of the system, the
/// workload, the start, the seed, the trace index and the number of
/// positions.
///
/// # Errors
///
/// [`SampleStop::GeneratorMismatch`] for a sampler other than
/// `quire.simulation.sampler/v1`, before any draw;
/// [`SampleStop::Expansion`] when a state's expansion stops;
/// [`SampleStop::UndecidedSuccessor`] when a contract conjunction of a
/// state's expansion is undecided; [`SampleStop::NotMarkov`] when the
/// drawn identity and vector give no post-state or several; [`SampleStop::TotalAboveDrawRange`] when a
/// choice's scaled total exceeds `2^256`; [`SampleStop::Cancelled`] when
/// `cancel` is cancelled, checked before each step.
#[allow(
    clippy::too_many_arguments,
    clippy::result_large_err,
    reason = "FR-188 pins this exact signature and SampleStop::NotMarkov(NotMarkov)"
)]
pub fn sample_probabilistic(
    system: &ModelSystem<'_>,
    workload: &Workload,
    start: &ModelSimState,
    sampler: &DefinitionRef,
    seed: u64,
    trace: u64,
    positions: u64,
    cancel: &Cancel,
) -> Result<SampledBehaviour, SampleStop> {
    sample_counted(
        system,
        &Sampling {
            workload,
            start,
            sampler,
            seed,
            trace,
            positions,
            cancel,
        },
    )
    .0
}

#[cfg(test)]
pub(crate) mod world;

#[cfg(test)]
mod tests;
