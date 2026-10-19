// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-166 (ADR-022 GE-2): phase 0, seeded witness walks for `possible`
//! items before exploration.
//!
//! Walk `r` from initial state `i` is FR-101's sampled trace with the run's
//! seed and trace index `i × witness_samples + r`, over the view of the
//! subject whose only initial state is `i`, taking at most the lesser of
//! `max_walk_steps` and the horizon. A walk is a function of the subject,
//! the seed and its trace index alone, so one walk serves every instance
//! that still has no witness from `i`; each instance keeps the witness of
//! the lowest walk index that found one and reads no later walk.

use std::collections::BTreeMap;

use qsl_eval::simulation::model::{sample_model_walk, ModelSystem, PredicateReading};
use qsl_eval::simulation::NotSimulated;
use qsl_foundation::selection::DefinitionRef;
use qsl_replay::{
    ModelCheckLimit, ModelCheckLimits, ModelPath, ModelStep, GraphWitness, WitnessSource,
};
use qsl_semantics::check::PredicateRole;
use quire_exact::Cancel;

use super::{cancelled, expansion_stop, fault, stopped, Budgets, Halt, InstanceView};
use crate::model_check::check::BOUND_POSITIONS;
use crate::model_check::ModelCheckRefusal;

/// The one generator phase 0 samples with (FR-101).
const SAMPLER: &str = "quire.simulation.sampler/v1";

/// An instance's identity within the request: item and instance index.
type InstanceKey = (usize, usize);

/// What phase 0 found: per `possible` instance, each initial state's sampled
/// witness, and the count of its walks `max_walk_steps` stopped.
#[derive(Default)]
pub(super) struct Sampled {
    witnesses: BTreeMap<InstanceKey, Vec<Option<GraphWitness>>>,
    stopped: BTreeMap<InstanceKey, u64>,
}

impl Sampled {
    /// `view`'s sampled witness per initial state; empty for an instance
    /// phase 0 did not sample.
    pub(super) fn witnesses(&self, view: &InstanceView<'_>) -> &[Option<GraphWitness>] {
        self.witnesses
            .get(&(view.item, view.instance))
            .map_or(&[], Vec::as_slice)
    }

    /// The count of `view`'s walks `max_walk_steps` stopped.
    pub(super) fn stopped(&self, view: &InstanceView<'_>) -> u64 {
        self.stopped
            .get(&(view.item, view.instance))
            .copied()
            .unwrap_or(0)
    }
}

/// Phase 0 of one run.
pub(super) struct Phase<'r, 'a> {
    pub(super) system: &'r ModelSystem<'a>,
    pub(super) limits: &'r ModelCheckLimits,
    pub(super) horizon: Option<usize>,
    pub(super) seed: u64,
    pub(super) cancel: &'r Cancel,
    pub(super) budgets: Budgets,
}

impl Phase<'_, '_> {
    /// Run phase 0 for `instances`, the `possible` item instances of the
    /// request: `Ok(Err(halt))` when a cancellation or an overflowing trace
    /// index stopped it, before the first walk in the latter case.
    pub(super) fn run(
        &self,
        instances: &[&InstanceView<'_>],
    ) -> Result<Result<Sampled, Halt>, ModelCheckRefusal> {
        let samples = self.limits.witness_samples;
        let mut sampled = Sampled::default();
        if samples == 0 || instances.is_empty() {
            return Ok(Ok(sampled));
        }
        let count = self.system_initial_count();
        // The greatest trace index, `(count - 1) × samples + samples - 1`,
        // with checked arithmetic, before any walk.
        let last = u64::try_from(count.saturating_sub(1))
            .ok()
            .and_then(|initial| initial.checked_mul(samples))
            .and_then(|base| base.checked_add(samples - 1));
        if last.is_none() {
            return Ok(Err(stopped(ModelCheckLimit::WitnessSamples, samples)));
        }
        let steps = self.step_ceiling();
        let sampler = DefinitionRef::new("quire", SAMPLER).map_err(|_| fault("sampler-ref"))?;
        for view in instances {
            sampled
                .witnesses
                .insert((view.item, view.instance), vec![None; count]);
            sampled.stopped.insert((view.item, view.instance), 0);
        }
        for initial in 0..count {
            let mut pending: Vec<&InstanceView<'_>> = instances.to_vec();
            // `initial < count` and the greatest index fits, so these do.
            let base = u64::try_from(initial).unwrap_or(u64::MAX) * samples;
            let mut walk_index = 0;
            while walk_index < samples && !pending.is_empty() {
                if let Some(halt) = cancelled(self.cancel) {
                    return Ok(Err(halt));
                }
                let walk = match sample_model_walk(
                    self.system,
                    initial,
                    BOUND_POSITIONS,
                    &sampler,
                    self.seed,
                    base + walk_index,
                    steps,
                ) {
                    Ok(walk) => walk,
                    Err(NotSimulated::KeyEncoding(refusal)) => {
                        return Err(ModelCheckRefusal::Encoding(refusal))
                    }
                    Err(_) => return Err(fault("phase-zero-walk-samples")),
                };
                let mut still = Vec::with_capacity(pending.len());
                for view in pending {
                    match self.witness_in(&walk, view, initial)? {
                        Ok(Found::GraphWitness(path)) => {
                            let provenance = walk
                                .trace
                                .provenance
                                .clone()
                                .ok_or_else(|| fault("sampled-trace-has-provenance"))?;
                            if let Some(slot) = sampled
                                .witnesses
                                .get_mut(&(view.item, view.instance))
                                .and_then(|slots| slots.get_mut(initial))
                            {
                                *slot = Some(GraphWitness {
                                    path,
                                    source: WitnessSource::Sampled(provenance),
                                });
                            }
                        }
                        Ok(Found::Nothing { budget_stopped }) => {
                            if budget_stopped {
                                if let Some(count) =
                                    sampled.stopped.get_mut(&(view.item, view.instance))
                                {
                                    *count = count.saturating_add(1);
                                }
                            }
                            still.push(view);
                        }
                        Err(halt) => return Ok(Err(halt)),
                    }
                }
                pending = still;
                walk_index += 1;
            }
        }
        Ok(Ok(sampled))
    }

    fn system_initial_count(&self) -> usize {
        qsl_eval::simulation::TransitionSystem::initial(self.system).len()
    }

    /// The lesser of `max_walk_steps` and the horizon.
    fn step_ceiling(&self) -> usize {
        let budget = usize::try_from(self.limits.max_walk_steps).unwrap_or(usize::MAX);
        self.horizon.map_or(budget, |horizon| budget.min(horizon))
    }

    /// Whether `max_walk_steps`, not the horizon, is the walk's ceiling.
    fn budget_is_ceiling(&self) -> bool {
        let budget = usize::try_from(self.limits.max_walk_steps).unwrap_or(usize::MAX);
        self.horizon.is_none_or(|horizon| budget <= horizon)
    }

    /// `view`'s target along `walk`, from position 0: the prefix up to the
    /// first position where it holds; nothing when the walk ends, or visits
    /// a position where the target is undefined or refused, first.
    fn witness_in(
        &self,
        walk: &qsl_eval::simulation::model::ModelWalk,
        view: &InstanceView<'_>,
        initial: usize,
    ) -> Result<Result<Found, Halt>, ModelCheckRefusal> {
        let Some((_, Some(target))) = view
            .predicates
            .iter()
            .find(|(role, _)| *role == PredicateRole::Target)
        else {
            return Err(fault("possible-has-a-target"));
        };
        for (position, state) in walk.states.iter().enumerate() {
            if let Some(halt) = cancelled(self.cancel) {
                return Ok(Err(halt));
            }
            let reading = match self.system.state_predicate(state, target, view.over.as_ref()) {
                Ok(reading) => reading,
                Err(stop) => {
                    return expansion_stop(&stop, self.budgets).map(Err);
                }
            };
            match reading {
                PredicateReading::True => {
                    let steps = walk
                        .trace
                        .steps
                        .iter()
                        .take(position)
                        .map(|step| ModelStep {
                            transition: step.transition.clone(),
                            post_state: step.key,
                        })
                        .collect();
                    let initial = u32::try_from(initial).map_err(|_| fault("initial-index-u32"))?;
                    return Ok(Ok(Found::GraphWitness(ModelPath { initial, steps })));
                }
                PredicateReading::False => {}
                PredicateReading::Undefined { .. } | PredicateReading::Refused(_) => {
                    return Ok(Ok(Found::Nothing {
                        budget_stopped: false,
                    }));
                }
            }
        }
        let budget_stopped = self.budget_is_ceiling()
            && walk.trace.provenance.as_ref().is_some_and(|provenance| {
                provenance.stopped == qsl_eval::simulation::StopReason::StepLimit
            });
        Ok(Ok(Found::Nothing { budget_stopped }))
    }
}

/// What one walk found for one instance.
enum Found {
    /// A witness.
    GraphWitness(ModelPath),
    /// Nothing; `budget_stopped` when `max_walk_steps` ended the walk.
    Nothing { budget_stopped: bool },
}
