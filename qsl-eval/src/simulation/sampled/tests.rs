// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-188 (TC-623): `sample_probabilistic` over the fixtures of
//! [`super::world`], with the digest count [`super::sample_counted`]
//! returns as the test-only digest counter.

use ix_trace_rs::trace;
use qsl_foundation::selection::DefinitionRef;
use quire_exact::{Cancel, CancelCause, Integer, Value};

use super::world::{self, field, initial, World};
use super::{sample_counted, SampleStop, SampledBehaviour, Sampling};
use crate::simulation::choice::{DrawPreimage, TotalAboveDrawRange};
use crate::simulation::key::plain_digest;
use crate::simulation::model::probabilistic::{same_rewards, same_values};
use crate::simulation::model::{ModelSimState, ModelSystem, TransitionId};

fn sampler() -> DefinitionRef {
    DefinitionRef::new("agent-ix", "quire.simulation.sampler/v1").expect("a valid DefinitionRef")
}

/// Sample `world` under `workload` from its initial state, with the
/// digests computed.
fn sample(
    world: &World,
    workload: &str,
    sampler: &DefinitionRef,
    (seed, trace, positions): (u64, u64, u64),
    cancel: &Cancel,
) -> (Result<SampledBehaviour, SampleStop>, u64) {
    let system = world.system();
    let start = initial(&system);
    sample_counted(
        &system,
        &Sampling {
            workload: world.workload(workload),
            start: &start,
            sampler,
            seed,
            trace,
            positions,
            cancel,
        },
    )
}

/// FR-188-AC-1 (TC-623 step 1): a sampler `DefinitionRef` of another
/// identity is refused `GeneratorMismatch` with no digest computed.
#[trace("TC-623", "FR-188-AC-1")]
#[test]
fn another_generator_is_refused_before_any_draw() {
    let world = World::new(&world::coin());
    let other = DefinitionRef::new("agent-ix", "quire.simulation.other/v1").expect("valid");
    let (result, digests) = sample(&world, "Unit", &other, (424_242, 0, 3), &Cancel::new());
    assert_eq!(
        result,
        Err(SampleStop::GeneratorMismatch { supplied: other })
    );
    assert_eq!(digests, 0);
}

/// FR-188-AC-4 (TC-623 step 3): on `Lone` every choice is over one item,
/// so a step computes no digest and records no choice.
#[trace("TC-623", "FR-188-AC-4")]
#[test]
fn one_item_choices_draw_nothing() {
    let world = World::new(&world::lone());
    let (result, digests) = sample(&world, "Unit", &sampler(), (424_242, 0, 3), &Cancel::new());
    let behaviour = result.expect("Lone samples");
    assert_eq!(behaviour.steps.len(), 2);
    for step in &behaviour.steps {
        assert!(step.choices.is_empty());
        assert!(step.random.is_empty());
        assert!(matches!(step.transition, TransitionId::Operation(_)));
    }
    assert_eq!(digests, 0);
}

/// FR-188-AC-4 (TC-623 step 3, QSpec TC-354 SM-07): on `Coin` the operation
/// and identity choices are one-item, so `x`'s choice is choice index 0,
/// whose preimage is QSpec TC-210's step-0 one; its `v mod 4` is 2, below
/// the first weight 3 of `(3, 1)` in canonical value order, so it selects
/// index 0, `false`.
#[trace("TC-623", "FR-188-AC-4")]
#[test]
fn a_random_parameter_takes_the_first_choice_index() {
    let preimage = DrawPreimage::at(
        crate::simulation::ChoicePoint {
            seed: 424_242,
            trace: 0,
            step: 0,
            choice: 0,
        },
        0,
    );
    let bytes = quire_canonical::to_vec(&preimage, quire_canonical::Limits::new(u64::MAX))
        .expect("encodes");
    assert_eq!(
        bytes,
        br#"{"choice":"0","draw":"0","seed":"424242","step":"0","trace":"0"}"#
    );
    let digest = plain_digest(&preimage);
    assert_eq!(digest[31] % 4, 2, "v mod 4 is the last byte's");

    let world = World::new(&world::coin());
    let (result, digests) = sample(&world, "Unit", &sampler(), (424_242, 0, 2), &Cancel::new());
    let behaviour = result.expect("Coin samples");
    let [step] = &behaviour.steps[..] else {
        panic!("one step: {behaviour:?}");
    };
    assert_eq!(step.choices, [0]);
    assert!(same_values(&step.random, &[Value::Boolean(false)]));
    assert_eq!(digests, 1);
}

/// FR-188-AC-5 (TC-623 step 3): on `Wide`, `x`'s weights `(2^256, 1)` in
/// canonical value order stop the sample `TotalAboveDrawRange{2^256 + 1}`
/// at that choice, with no digest computed.
#[trace("TC-623", "FR-188-AC-5")]
#[test]
fn a_total_above_the_draw_range_stops_the_sample() {
    let world = World::new(&world::wide());
    let (result, digests) = sample(&world, "Unit", &sampler(), (424_242, 0, 2), &Cancel::new());
    assert_eq!(
        result,
        Err(SampleStop::TotalAboveDrawRange(TotalAboveDrawRange {
            total: Integer::one().shifted_left(256).add(&Integer::one())
        }))
    );
    assert_eq!(digests, 0);
}

/// The state `step` of `behaviour` reaches from `state`, found among
/// FR-187's weighted steps, with that step's probability and rewards.
fn follow(
    system: &ModelSystem<'_>,
    world: &World,
    state: &ModelSimState,
    step: &super::SampledStep,
) -> ModelSimState {
    let weighted = system
        .weighted_steps(state, world.workload("Steady"))
        .expect("expands")
        .expect("Markov");
    let matched = weighted
        .into_iter()
        .find(|candidate| {
            candidate.transition == step.transition
                && system.state_key(&candidate.post).expect("key").1 == step.post_digest
        })
        .expect("the sampled step is one of FR-187's");
    assert_eq!(matched.probability, step.probability);
    assert!(same_rewards(&matched.rewards, &step.rewards));
    matched.post
}

fn integer_of(value: &Value) -> i64 {
    match value {
        Value::Integer(integer) => integer.to_string().parse().expect("small"),
        other => panic!("not an integer: {other:?}"),
    }
}

/// FR-188-AC-3 (TC-623 step 4): `Service` under `Steady` from its initial
/// state, seed 7, trace 12, 11 positions, sampled twice, gives equal
/// behaviours; each step is one of FR-187's, with its probability and its
/// `duration` (`d` for `attempt`, else 0), its drawn values inside their
/// supports and one selection per choice that drew; the sample passes
/// `Done` and then `Idle` and continues. Untagged `FR-188-AC-3`: `Service`
/// stands in integers and Booleans for its enum and quantity types (see the
/// world's module doc).
#[trace("TC-623")]
#[test]
fn service_samples_reproduce_and_follow_the_step_structure() {
    let world = World::new(&world::service(""));
    let (first, _) = sample(&world, "Steady", &sampler(), (7, 12, 11), &Cancel::new());
    let (second, _) = sample(&world, "Steady", &sampler(), (7, 12, 11), &Cancel::new());
    let behaviour = first.expect("Service samples");
    assert_eq!(Ok(behaviour.clone()), second);
    assert_eq!(behaviour.trace, 12);
    assert_eq!(behaviour.steps.len(), 10);

    let system = world.system();
    let mut state = initial(&system);
    assert_eq!(behaviour.start, system.state_key(&state).expect("key").1);
    let mut phases = Vec::new();
    for step in &behaviour.steps {
        let pre_phase = integer_of(&field(&state, "phase"));
        state = follow(&system, &world, &state, step);
        phases.push(integer_of(&field(&state, "phase")));
        let duration = integer_of(&step.rewards[0].1);
        if pre_phase == 1 {
            // `attempt`: `d` then `ok` drawn, `d` from `{1, 3, 12}`.
            let [d, ok] = &step.random[..] else {
                panic!("two random values: {step:?}");
            };
            let d = integer_of(d);
            assert!([1, 3, 12].contains(&d));
            assert!(matches!(ok, Value::Boolean(_)));
            assert_eq!(duration, d);
            // One enabled operation and identity: the two draws only.
            assert_eq!(step.choices.len(), 2);
        } else {
            assert!(step.random.is_empty());
            assert!(step.choices.is_empty());
            assert_eq!(duration, 0);
        }
    }
    let done = phases
        .iter()
        .position(|phase| *phase == 2)
        .expect("reaches Done");
    assert_eq!(phases.get(done + 1), Some(&0), "Done then Idle: {phases:?}");
    assert!(
        done + 1 < phases.len() - 1,
        "the sample continues: {phases:?}"
    );
}

/// FR-188-AC-3 (TC-623 step 4): the `Health` variant with two post-states
/// per draw stops with `SampleStop::NotMarkov`.
#[trace("TC-623", "FR-188-AC-3")]
#[test]
fn a_non_unique_post_state_stops_the_sample() {
    let world = World::new(&world::loose_health());
    let (result, _) = sample(&world, "Steady", &sampler(), (7, 12, 11), &Cancel::new());
    let Err(SampleStop::NotMarkov(not_markov)) = result else {
        panic!("NotMarkov: {result:?}");
    };
    assert_eq!(not_markov.post_states, 2);
}

/// A terminal state continues on stutter steps (ADR-018 SM-4): `Link`
/// samples to a terminal state within its two attempts, then stutters with
/// probability 1 and no draw.
#[trace("TC-623")]
#[test]
fn a_terminal_state_continues_on_stutter_steps() {
    let world = World::new(&world::link());
    let (result, _) = sample(&world, "Even", &sampler(), (7, 0, 6), &Cancel::new());
    let behaviour = result.expect("Link samples");
    assert_eq!(behaviour.steps.len(), 5);
    let last = behaviour.steps.last().expect("a step");
    assert_eq!(last.transition, TransitionId::Stutter);
    assert!(last.choices.is_empty());
    assert_eq!(
        last.probability,
        quire_exact::Rational::from_integer(Integer::one())
    );
}

/// A cancelled handle stops the sample before its first step.
#[trace("TC-623")]
#[test]
fn a_cancelled_handle_stops_the_sample() {
    let world = World::new(&world::coin());
    let cancel = Cancel::new();
    cancel.cancel(CancelCause::Requested);
    let (result, digests) = sample(&world, "Unit", &sampler(), (424_242, 0, 3), &cancel);
    assert_eq!(result, Err(SampleStop::Cancelled(CancelCause::Requested)));
    assert_eq!(digests, 0);
}

/// FR-188-AC-6 (TC-623 step 3): `SampledStep`, `SampledBehaviour` and
/// `SampleStop` resolve at `qsl_eval::simulation` and are defined in
/// `qsl-eval`, so no `quire-semantic-value` item is one of them (that leaf
/// does not depend on `qsl-eval`).
#[trace("TC-623", "FR-188-AC-6")]
#[test]
fn the_sample_types_are_qsl_eval_simulation_items() {
    for name in [
        std::any::type_name::<crate::simulation::SampledStep>(),
        std::any::type_name::<crate::simulation::SampledBehaviour>(),
        std::any::type_name::<crate::simulation::SampleStop>(),
    ] {
        assert!(name.starts_with("qsl_eval::simulation::"), "{name}");
    }
}
