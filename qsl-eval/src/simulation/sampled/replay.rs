// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-193 "Replay" (ADR-024 SV-8): the draw checks of a recorded sampled
//! behaviour. Each recorded random value must lie in its parameter's
//! support; each choice, recomputed from the seed, the trace index, the step
//! and the choice index, must select the recorded operation, scheduled
//! identity and values; and the path probability, recomputed exactly, must
//! equal the recorded one.
//!
//! The recomputation is [`sample_probabilistic`] itself, over as many
//! positions as the record has, so the replayed behaviour is drawn by the
//! one sampler that drew the record: a step that is not enabled, a post
//! state no successor has, or a draw that selects another item all show as
//! a recomputed step that differs from the recorded one.

use qsl_foundation::selection::DefinitionRef;
use qsl_semantics::check::Workload;
use quire_exact::{Cancel, Integer, Rational};
use quire_semantic_value::catalog::CatalogCode;

use super::{sample_probabilistic, SampleStop, SampledStep};
use crate::simulation::model::probabilistic::same_value;
use crate::simulation::model::{ModelSimState, ModelSystem, TransitionId};

/// `invalid_runtime_input`/`invalid-value`: a recorded value outside its
/// support, or a recorded step no sample takes (FR-193).
const INVALID_VALUE: CatalogCode = CatalogCode::new("invalid_runtime_input", "invalid-value");

/// A recorded sampled behaviour, as a witness keeps it (FR-193).
#[derive(Clone, Copy, Debug)]
pub struct SampledRecord<'a> {
    /// The trace index it was drawn at.
    pub trace: u64,
    /// Its steps, one per position after the first.
    pub steps: &'a [SampledStep],
    /// The recorded product of the steps' probabilities.
    pub path_probability: &'a Rational,
}

/// Where a recomputed behaviour departs from its record (FR-193).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SampledDisagreement {
    /// The recomputed step at this 0-based index differs from the recorded
    /// one: its transition, post-state digest, selections, drawn values,
    /// probability or rewards.
    Step {
        /// The step's index.
        step: u64,
    },
    /// Every step agrees, and the recorded path probability is not their
    /// product.
    PathProbability {
        /// The product of the recomputed step probabilities.
        recomputed: Rational,
    },
}

/// What a recorded sampled behaviour replays to (FR-193).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SampledReplay {
    /// Every check agrees.
    Reproduced,
    /// A recomputed draw or the path probability disagrees.
    Disagrees(SampledDisagreement),
}

/// Why a recorded sampled behaviour is refused before it replays (FR-193).
#[derive(Clone, Debug, PartialEq, thiserror::Error)]
pub enum SampledReplayRefusal {
    /// A recorded random value lies outside its parameter's support.
    #[error("step {step} drew a value outside the support of random parameter {parameter}")]
    OutsideSupport {
        /// The step's 0-based index.
        step: u64,
        /// The random parameter's index, in declared order.
        parameter: u32,
    },
    /// The recomputation stopped: a `TotalAboveDrawRange` among them, since
    /// no sample takes such a step.
    #[error(transparent)]
    Sample(Box<SampleStop>),
}

impl SampledReplayRefusal {
    /// The catalog code of a refusal FR-193 names: `invalid_runtime_input`/`invalid-value` for a
    /// value outside its support and for a `TotalAboveDrawRange`; `None` for
    /// a stop whose code is its own.
    pub fn catalog_code(&self) -> Option<CatalogCode> {
        match self {
            Self::OutsideSupport { .. } => Some(INVALID_VALUE),
            Self::Sample(stop) => match **stop {
                SampleStop::TotalAboveDrawRange(_) => Some(INVALID_VALUE),
                SampleStop::GeneratorMismatch { .. }
                | SampleStop::Expansion(_)
                | SampleStop::NotMarkov(_)
                | SampleStop::UndecidedSuccessor
                | SampleStop::Cancelled(_) => None,
            },
        }
    }
}

/// The first recorded random value outside its parameter's support.
fn outside_support(system: &ModelSystem<'_>, steps: &[SampledStep]) -> Option<(u64, u32)> {
    (0_u64..).zip(steps).find_map(|(index, step)| {
        let TransitionId::Operation(transition) = &step.transition else {
            return None;
        };
        let supports = system.random_of_text(transition.operation());
        (0_u32..)
            .zip(supports.iter().zip(&step.random))
            .find(|(_, (support, value))| {
                !support
                    .items
                    .iter()
                    .any(|item| same_value(&item.value, value))
            })
            .map(|(parameter, _)| (index, parameter))
    })
}

/// FR-193: replay `record` over `system` under `workload` from `start`,
/// with `sampler` and `seed`: refuse a recorded value outside its support,
/// then recompute the behaviour with [`sample_probabilistic`] over the
/// record's positions and compare each step, then the path probability.
///
/// # Errors
///
/// [`SampledReplayRefusal::OutsideSupport`] for the first recorded random
/// value outside its support, before any draw;
/// [`SampledReplayRefusal::Sample`] when the recomputation stops.
pub fn replay_sampled(
    system: &ModelSystem<'_>,
    workload: &Workload,
    start: &ModelSimState,
    sampler: &DefinitionRef,
    seed: u64,
    record: SampledRecord<'_>,
    cancel: &Cancel,
) -> Result<SampledReplay, SampledReplayRefusal> {
    if let Some((step, parameter)) = outside_support(system, record.steps) {
        return Err(SampledReplayRefusal::OutsideSupport { step, parameter });
    }
    let positions = u64::try_from(record.steps.len())
        .unwrap_or(u64::MAX)
        .saturating_add(1);
    let replayed = sample_probabilistic(
        system,
        workload,
        start,
        sampler,
        seed,
        record.trace,
        positions,
        cancel,
    )
    .map_err(|stop| SampledReplayRefusal::Sample(Box::new(stop)))?;
    let differs = (0_u64..)
        .zip(replayed.steps.iter().zip(record.steps))
        .find(|(_, (replayed, recorded))| replayed != recorded);
    if let Some((step, _)) = differs {
        return Ok(SampledReplay::Disagrees(SampledDisagreement::Step { step }));
    }
    let recomputed = replayed
        .steps
        .iter()
        .fold(Rational::from_integer(Integer::one()), |product, step| {
            product.mul(&step.probability)
        });
    if recomputed != *record.path_probability {
        return Ok(SampledReplay::Disagrees(
            SampledDisagreement::PathProbability { recomputed },
        ));
    }
    Ok(SampledReplay::Reproduced)
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;
    use qsl_foundation::selection::DefinitionRef;
    use quire_exact::{Cancel, Integer, Rational, Value};

    use super::*;
    use crate::simulation::sampled::world::{self, initial, World};
    use crate::simulation::sampled::SampledBehaviour;

    fn sampler() -> DefinitionRef {
        DefinitionRef::new("agent-ix", "quire.simulation.sampler/v1").expect("sampler")
    }

    fn product(steps: &[SampledStep]) -> Rational {
        steps
            .iter()
            .fold(Rational::from_integer(Integer::one()), |product, step| {
                product.mul(&step.probability)
            })
    }

    /// `Service` under `Steady` from its initial state, seed 7, trace 12, 11
    /// positions.
    fn service_sample(world: &World) -> SampledBehaviour {
        sample_with(world, 7)
    }

    /// `Service` under `Steady` from its initial state at trace 12 over 11
    /// positions, under `seed`.
    fn sample_with(world: &World, seed: u64) -> SampledBehaviour {
        let system = world.system();
        let start = initial(&system);
        sample_probabilistic(
            &system,
            world.workload("Steady"),
            &start,
            &sampler(),
            seed,
            12,
            11,
            &Cancel::new(),
        )
        .expect("Service samples")
    }

    fn replay(
        world: &World,
        seed: u64,
        steps: &[SampledStep],
        path_probability: &Rational,
    ) -> Result<SampledReplay, SampledReplayRefusal> {
        let system = world.system();
        let start = initial(&system);
        replay_sampled(
            &system,
            world.workload("Steady"),
            &start,
            &sampler(),
            seed,
            SampledRecord {
                trace: 12,
                steps,
                path_probability,
            },
            &Cancel::new(),
        )
    }

    /// FR-193 "Replay" over ADR-024 §7.2's `Service`: a recorded behaviour
    /// replays `Reproduced`; with an `attempt` step's `d` changed to 2 ms,
    /// outside `{1, 3, 12}`, it refuses `invalid_runtime_input`/
    /// `invalid-value` before any draw; under another seed a recomputed
    /// draw disagrees; with its path probability changed the path
    /// probability disagrees.
    #[trace("TC-628")]
    #[test]
    fn a_recorded_service_behaviour_replays_and_each_alteration_is_caught() {
        let world = World::new(&world::service(""));
        let behaviour = service_sample(&world);
        let path = product(&behaviour.steps);
        assert_eq!(
            replay(&world, 7, &behaviour.steps, &path),
            Ok(SampledReplay::Reproduced)
        );

        let attempt = behaviour
            .steps
            .iter()
            .position(|step| step.random.len() == 2)
            .expect("the sample takes an attempt step");
        let mut outside = behaviour.steps.clone();
        outside[attempt].random[0] = Value::Integer(Integer::from(2_i64));
        let refusal = replay(&world, 7, &outside, &path).expect_err("2 ms is outside the support");
        assert_eq!(
            refusal,
            SampledReplayRefusal::OutsideSupport {
                step: u64::try_from(attempt).expect("small"),
                parameter: 0,
            }
        );
        assert_eq!(refusal.catalog_code(), Some(INVALID_VALUE));

        // The first seed after 7 whose behaviour at trace 12 differs, and the
        // first step where it does.
        let (seed, step) = (8_u64..)
            .take(256)
            .find_map(|seed| {
                let other = sample_with(&world, seed);
                (0_u64..)
                    .zip(other.steps.iter().zip(&behaviour.steps))
                    .find(|(_, (other, recorded))| other != recorded)
                    .map(|(step, _)| (seed, step))
            })
            .expect("some seed draws another behaviour");
        assert_eq!(
            replay(&world, seed, &behaviour.steps, &path),
            Ok(SampledReplay::Disagrees(SampledDisagreement::Step { step }))
        );

        let doubled = path.add(&path);
        assert_eq!(
            replay(&world, 7, &behaviour.steps, &doubled),
            Ok(SampledReplay::Disagrees(
                SampledDisagreement::PathProbability { recomputed: path }
            ))
        );
    }

    /// A recorded step whose post-state digest is not the recomputed one
    /// disagrees at that step.
    #[trace("TC-628")]
    #[test]
    fn a_changed_post_state_disagrees_at_its_step() {
        let world = World::new(&world::service(""));
        let behaviour = service_sample(&world);
        let path = product(&behaviour.steps);
        let mut moved = behaviour.steps.clone();
        moved[1].post_digest = moved[0].post_digest;
        assert_ne!(
            moved[1], behaviour.steps[1],
            "steps 0 and 1 reach distinct states"
        );
        assert_eq!(
            replay(&world, 7, &moved, &path),
            Ok(SampledReplay::Disagrees(SampledDisagreement::Step {
                step: 1
            }))
        );
    }
}
