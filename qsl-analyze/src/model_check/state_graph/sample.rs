// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-166 (ADR-022 GE-2): phase 0, seeded witness walks for `possible`
//! items before exploration.
//!
//! Walk `r` from initial state `i` is FR-101's sampled trace with the run's
//! seed and trace index `i × witness_samples + r`, over the view of the
//! subject whose only initial state is `i`, taking at most the lesser of
//! `max_walk_steps` and the horizon. A walk is a function of the subject,
//! the seed and its trace index alone, so one walk serves every instance
//! that still has no witness from `i`: the target of each is read at every
//! state the walk visits, position 0 included, and the walk ends at the
//! first state by which every one of them has found its target, or an
//! undefined or refused reading of it. Each instance keeps the witness of
//! the lowest walk index that found one and reads no later walk.

use std::collections::BTreeMap;

use qsl_eval::simulation::model::{sample_model_walk, ModelSystem, PredicateReading};
use qsl_eval::simulation::{NotSimulated, SampleProvenance, StopReason, TransitionSystem};
use qsl_foundation::selection::DefinitionRef;
use qsl_replay::{
    GraphWitness, ModelCheckLimit, ModelCheckLimits, ModelPath, ModelStep, WitnessSource,
};
use qsl_semantics::check::{CheckedPredicateBody, PredicateRole};
use quire_exact::{Cancel, ObjectReference};

use super::{
    budget_is_ceiling, cancelled, expansion_stop, fault, sampler, step_ceiling, stopped,
    trace_index, trace_indices_fit, Budgets, Halt, InstanceView,
};
use crate::model_check::check::BOUND_POSITIONS;
use crate::model_check::ModelCheckRefusal;

/// An instance's identity within the request: item and instance index.
type InstanceKey = (usize, usize);

/// One instance's target as a walk reads it.
type Target<'a> = (&'a CheckedPredicateBody, Option<&'a ObjectReference>);

/// What phase 0 found: per `possible` instance, each initial state's sampled
/// witness, and the count of its walks `max_walk_steps` stopped.
#[derive(Default)]
pub(super) struct Sampled {
    witnesses: BTreeMap<InstanceKey, Vec<Option<GraphWitness>>>,
    stopped: BTreeMap<InstanceKey, u64>,
}

impl Sampled {
    /// `view`'s sampled witness per initial state; empty for an instance
    /// phase 0 drew no walk for.
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

/// What one walk found for one instance: the position of the first target
/// state, or a position where the target had no truth value.
#[derive(Clone, Copy)]
enum Found {
    Target(usize),
    Unreadable,
}

/// One walk: what it found for each instance it read, its steps, whether
/// `max_walk_steps` ended it, and its provenance.
struct Walk {
    found: Vec<Option<Found>>,
    steps: Vec<ModelStep>,
    budget_stopped: bool,
    provenance: SampleProvenance,
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
    /// request: `Ok(Err(halt))` when a cancellation, a meter or an
    /// overflowing trace index stopped it, before the first walk in the last
    /// case.
    pub(super) fn run(
        &self,
        instances: &[&InstanceView<'_>],
    ) -> Result<Result<Sampled, Halt>, ModelCheckRefusal> {
        let samples = self.limits.witness_samples;
        let mut sampled = Sampled::default();
        if samples == 0 || instances.is_empty() {
            return Ok(Ok(sampled));
        }
        let count = self.system.initial().len();
        // The greatest trace index, with checked arithmetic, before any walk.
        if !trace_indices_fit(count, samples) {
            return Ok(Err(stopped(ModelCheckLimit::WitnessSamples, samples)));
        }
        let sampler = sampler()?;
        let mut targets: Vec<Target<'_>> = Vec::with_capacity(instances.len());
        for view in instances {
            let Some((_, Some(target))) = view
                .predicates
                .iter()
                .find(|(role, _)| *role == PredicateRole::Target)
            else {
                return Err(fault("possible-has-a-target"));
            };
            targets.push((*target, view.over.as_ref()));
            sampled
                .witnesses
                .insert((view.item, view.instance), vec![None; count]);
            sampled.stopped.insert((view.item, view.instance), 0);
        }
        for initial in 0..count {
            let mut pending: Vec<usize> = (0..instances.len()).collect();
            let mut walk_index = 0;
            while walk_index < samples && !pending.is_empty() {
                if let Some(halt) = cancelled(self.cancel) {
                    return Ok(Err(halt));
                }
                let asked: Vec<Target<'_>> = pending.iter().map(|index| targets[*index]).collect();
                let walk = match self.walk(
                    &sampler,
                    initial,
                    trace_index(initial, samples, walk_index)?,
                    &asked,
                )? {
                    Ok(walk) => walk,
                    Err(halt) => return Ok(Err(halt)),
                };
                let mut still = Vec::with_capacity(pending.len());
                for (index, found) in pending.into_iter().zip(&walk.found) {
                    let view = instances[index];
                    let key = (view.item, view.instance);
                    match found {
                        Some(Found::Target(position)) => {
                            let path = ModelPath {
                                initial: u32::try_from(initial)
                                    .map_err(|_| fault("initial-index-fits-u32"))?,
                                steps: walk.steps.iter().take(*position).cloned().collect(),
                            };
                            if let Some(slot) = sampled
                                .witnesses
                                .get_mut(&key)
                                .and_then(|slots| slots.get_mut(initial))
                            {
                                // The walk this instance would draw alone
                                // ends at its target: `Reached`, whatever
                                // ended the shared walk.
                                *slot = Some(GraphWitness {
                                    path,
                                    source: WitnessSource::Sampled(SampleProvenance {
                                        stopped: StopReason::Reached,
                                        ..walk.provenance.clone()
                                    }),
                                });
                            }
                        }
                        Some(Found::Unreadable) => still.push(index),
                        None => {
                            if walk.budget_stopped {
                                if let Some(count) = sampled.stopped.get_mut(&key) {
                                    *count = count.saturating_add(1);
                                }
                            }
                            still.push(index);
                        }
                    }
                }
                pending = still;
                walk_index += 1;
            }
        }
        Ok(Ok(sampled))
    }

    /// Walk `trace` from `initial`, reading each of `targets` at every state
    /// it visits until each has found its target or an unreadable value.
    fn walk(
        &self,
        sampler: &DefinitionRef,
        initial: usize,
        trace: u64,
        targets: &[Target<'_>],
    ) -> Result<Result<Walk, Halt>, ModelCheckRefusal> {
        let mut found: Vec<Option<Found>> = vec![None; targets.len()];
        let mut halt: Option<Result<Halt, ModelCheckRefusal>> = None;
        let mut position = 0;
        let walked = sample_model_walk(
            self.system,
            initial,
            BOUND_POSITIONS,
            sampler,
            self.seed,
            trace,
            step_ceiling(self.limits, self.horizon),
            |state| {
                let here = position;
                position += 1;
                if let Some(cancel) = cancelled(self.cancel) {
                    halt = Some(Ok(cancel));
                    return true;
                }
                let open: Vec<usize> = (0..targets.len())
                    .filter(|index| found[*index].is_none())
                    .collect();
                let asked: Vec<Target<'_>> = open.iter().map(|index| targets[*index]).collect();
                let readings = match self.system.state_predicates(state, &asked) {
                    Ok(readings) => readings,
                    Err(stop) => {
                        halt = Some(expansion_stop(&stop, self.budgets));
                        return true;
                    }
                };
                for (index, reading) in open.into_iter().zip(readings) {
                    found[index] = match reading {
                        PredicateReading::True => Some(Found::Target(here)),
                        PredicateReading::False => None,
                        PredicateReading::Undefined { .. } | PredicateReading::Refused(_) => {
                            Some(Found::Unreadable)
                        }
                    };
                }
                found.iter().all(Option::is_some)
            },
        );
        if let Some(halt) = halt {
            return halt.map(Err);
        }
        let trace = match walked {
            Ok(trace) => trace,
            Err(NotSimulated::KeyEncoding(refusal)) => {
                return Err(ModelCheckRefusal::Encoding(refusal))
            }
            Err(_) => return Err(fault("phase-zero-walk-samples")),
        };
        let provenance = trace
            .provenance
            .ok_or_else(|| fault("sampled-trace-has-provenance"))?;
        let budget_stopped = budget_is_ceiling(self.limits, self.horizon)
            && provenance.stopped == StopReason::StepLimit;
        let steps = trace
            .steps
            .into_iter()
            .map(|step| ModelStep {
                transition: step.transition,
                post_state: step.key,
            })
            .collect();
        Ok(Ok(Walk {
            found,
            steps,
            budget_stopped,
            provenance,
        }))
    }
}
