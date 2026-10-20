// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-142 (TC-547): the safety half of a refinement on the explicit-state
//! product, over ADR-020 §8's `CasRefinesCounter` and its lost-update model.

use ix_trace_rs::trace;
use qsl_analyze::model_check::{check_refinement, RefinementCheckRequest};
use qsl_eval::model_subject::ModelSubject;
use qsl_eval::simulation::model::operation_identity_text;
use qsl_eval::simulation::Limits;
use qsl_replay::{
    CounterexampleKind, IncompleteCause, MappingRowRef, MappingSite, ModelCheckLimit,
    ModelCheckLimits, ObjectKey, ReachedLimit, RefinementFailure, RefinementOutcome, SafetyOutcome,
    StepRejection, TemporalCounterexample, UndefinedLocus,
};
use quire_exact::{Cancel, CancelCause};

use crate::refinement_fixture::{
    cas_refines_counter, key, lost_update_refinement, mutated, Variant, World, CAS, UNLIMITED,
};
use crate::ring_world::{
    key as ring_key, RingWorld, Variant as RingVariant, QUEUE, RING, UNLIMITED as RING_UNLIMITED,
};

fn run_with(
    world: &World,
    limits: ModelCheckLimits,
    max_depth: Option<usize>,
    cancel: &Cancel,
) -> SafetyOutcome {
    let RefinementOutcome { safety, .. } = check_refinement(
        RefinementCheckRequest {
            refinement: world.refinement(),
            concrete: ModelSubject {
                simulation: world.concrete_request(UNLIMITED),
                proof_bounds: Vec::new(),
            },
            abstract_package: world.package(),
            abstract_initial: vec![world.abstract_initial()],
            max_depth,
            limits,
        },
        cancel,
    )
    .expect("the refinement is checked");
    safety
}

fn run(world: &World) -> SafetyOutcome {
    run_with(world, ModelCheckLimits::default(), None, &Cancel::new())
}

fn violated(outcome: SafetyOutcome) -> TemporalCounterexample {
    let SafetyOutcome::Violated { counterexample, .. } = outcome else {
        panic!("the refinement is violated: {outcome:?}");
    };
    *counterexample
}

/// The operations of `counterexample`'s prefix, by name.
fn operations(counterexample: &TemporalCounterexample) -> Vec<String> {
    let names = [
        "beginA", "beginB", "commitA", "commitB", "retryA", "retryB", "peek",
    ];
    counterexample
        .prefix
        .iter()
        .map(|step| {
            names
                .iter()
                .find(|name| {
                    operation_identity_text(&key(CAS, &format!("Counter/{name}")))
                        .is_ok_and(|text| text == step.transition.operation())
                })
                .map(|name| (*name).to_owned())
                .expect("a step of the concrete model")
        })
        .collect()
}

/// FR-142-AC-1 (TC-547 step 1): `CasRefinesCounter` holds over 28 product
/// states.
#[trace("TC-547", "FR-142-AC-1")]
#[test]
fn the_compare_and_set_counter_holds_over_twenty_eight_states() {
    let SafetyOutcome::Holds { stats, .. } = run(&World::new(&cas_refines_counter())) else {
        panic!("the refinement holds");
    };
    assert_eq!(stats.product_states, 28);
}

/// FR-142-AC-2 (TC-547 step 2): the lost-update model is refuted at the
/// fourth step, and with both commit rows `-> any` at the sixth.
#[trace("TC-547", "FR-142-AC-2")]
#[test]
fn the_lost_update_is_refuted_at_its_first_failing_edge() {
    let world = World::with(Variant::LOST_UPDATE, &lost_update_refinement());
    let counterexample = violated(run(&world));
    assert_eq!(
        operations(&counterexample),
        ["beginA", "beginB", "commitA", "commitB"]
    );
    assert_eq!(counterexample.kind, CounterexampleKind::Formula);
    let Some(RefinementFailure::AbstractStepRejected {
        position,
        transition,
        cause,
    }) = counterexample.refinement
    else {
        panic!("an abstract step is rejected");
    };
    assert_eq!(position, 4);
    assert_eq!(cause, StepRejection::Postcondition);
    assert!(transition.receiver().is_some());

    let any = lost_update_refinement()
        .replace(
            "step Impl::Counter::commitA -> Spec::Counter::inc(self);",
            "step Impl::Counter::commitA -> any;",
        )
        .replace(
            "step Impl::Counter::commitB -> Spec::Counter::inc(self);",
            "step Impl::Counter::commitB -> any;",
        );
    let counterexample = violated(run(&World::with(Variant::LOST_UPDATE, &any)));
    assert_eq!(counterexample.prefix.len(), 6);
    assert_eq!(
        counterexample.refinement,
        Some(RefinementFailure::NoAbstractMatch { position: 6 })
    );
}

/// FR-142-AC-3 (TC-547 step 3): a concrete initial `value` of 1 is not an
/// abstract initial state, and `commitA -> stutter` changes the mapped value.
#[trace("TC-547", "FR-142-AC-3")]
#[test]
fn a_wrong_initial_state_and_a_changing_stutter_are_refuted() {
    let initial = World::with(
        Variant {
            concrete_value: 1,
            ..Variant::CAS
        },
        &cas_refines_counter(),
    );
    let counterexample = violated(run(&initial));
    assert!(counterexample.prefix.is_empty());
    assert_eq!(
        counterexample.refinement,
        Some(RefinementFailure::InitialNotAbstract { initial: 0 })
    );

    let stutter = mutated(
        &cas_refines_counter(),
        "step Impl::Counter::commitA -> Spec::Counter::inc(self);",
        "step Impl::Counter::commitA -> stutter;",
    );
    let counterexample = violated(run(&World::new(&stutter)));
    assert_eq!(operations(&counterexample), ["beginA", "commitA"]);
    assert_eq!(
        counterexample.refinement,
        Some(RefinementFailure::StutterChanged { position: 2 })
    );
}

/// FR-142-AC-7 (TC-547 step 7): `max_states` 5 stops the run, `max_depth` 2
/// reaches its horizon, and a cancelled handle stops it.
#[trace("TC-547", "FR-142-AC-7")]
#[test]
fn a_limit_a_horizon_and_a_cancellation_end_the_run() {
    let world = World::new(&cas_refines_counter());
    let few = ModelCheckLimits {
        limits: Limits {
            max_states: 5,
            ..Limits::default()
        },
        ..ModelCheckLimits::default()
    };
    let SafetyOutcome::Stopped { cause, limit, .. } = run_with(&world, few, None, &Cancel::new())
    else {
        panic!("the run is stopped");
    };
    assert_eq!(cause, IncompleteCause::ResourceExhausted);
    assert_eq!(
        limit,
        Some(ReachedLimit {
            limit: ModelCheckLimit::MaxStates,
            value: 5
        })
    );

    let SafetyOutcome::BoundReached { depth, .. } =
        run_with(&world, ModelCheckLimits::default(), Some(2), &Cancel::new())
    else {
        panic!("the horizon is reached");
    };
    assert_eq!(depth, 2);

    let cancel = Cancel::new();
    cancel.cancel(CancelCause::Requested);
    let SafetyOutcome::Stopped { cause, limit, .. } =
        run_with(&world, ModelCheckLimits::default(), None, &cancel)
    else {
        panic!("the run is cancelled");
    };
    assert_eq!(
        cause,
        IncompleteCause::Cancelled {
            source: CancelCause::Requested
        }
    );
    assert_eq!(limit, None);
}

/// FR-142-AC-7 (TC-547 step 7): the outcome and the counterexample are a
/// function of the request.
#[trace("TC-547", "FR-142-AC-7")]
#[test]
fn the_same_request_gives_the_same_outcome() {
    let world = World::with(Variant::LOST_UPDATE, &lost_update_refinement());
    assert_eq!(run(&world), run(&world));
}

/// The safety half of the ring world `world`, under the default limits.
fn run_ring(world: &RingWorld) -> SafetyOutcome {
    let RefinementOutcome { safety, .. } = check_refinement(
        RefinementCheckRequest {
            refinement: world.refinement(),
            concrete: ModelSubject {
                simulation: world.concrete_request(RING_UNLIMITED),
                proof_bounds: Vec::new(),
            },
            abstract_package: world.package(),
            abstract_initial: vec![world.abstract_initial()],
            max_depth: None,
            limits: ModelCheckLimits::default(),
        },
        &Cancel::new(),
    )
    .expect("the refinement is checked");
    safety
}

/// FR-142-AC-6 (TC-547 step 6): `RingIsQueue` holds.
#[trace("TC-547", "FR-142-AC-6")]
#[test]
fn the_ring_buffer_refines_the_queue() {
    let SafetyOutcome::Holds { stats, .. } = run_ring(&RingWorld::new()) else {
        panic!("the refinement holds");
    };
    assert!(stats.product_states > 1);
}

/// FR-142-AC-6 (TC-547 step 6): with the broken `take` it is refuted at
/// `deq(r)`'s postcondition.
#[trace("TC-547", "FR-142-AC-6")]
#[test]
fn the_broken_take_is_refuted_at_the_dequeue_postcondition() {
    let world = RingWorld::variant(RingVariant {
        broken_take: true,
        ..RingVariant::default()
    });
    let counterexample = violated(run_ring(&world));
    let Some(RefinementFailure::AbstractStepRejected {
        transition, cause, ..
    }) = counterexample.refinement
    else {
        panic!("an abstract step is rejected");
    };
    assert_eq!(cause, StepRejection::Postcondition);
    assert_eq!(
        transition.operation(),
        operation_identity_text(&ring_key(QUEUE, "Queue/deq")).expect("a text")
    );
}

/// FR-142-AC-6 (TC-547 step 6): with FR-140-AC-3's `only` the refinement is
/// refuted at the first state with `size >= 1` in canonical order, as an
/// undefined evaluation naming that position, the `items` row and object `r`.
#[trace("TC-547", "FR-142-AC-6")]
#[test]
fn an_undefined_items_row_refutes_at_the_first_non_empty_queue() {
    let world = RingWorld::variant(RingVariant {
        only_ring: true,
        ..RingVariant::default()
    });
    let counterexample = violated(run_ring(&world));
    assert_eq!(counterexample.prefix.len(), 1);
    let row = MappingRowRef::Field {
        object_type: ring_key(QUEUE, "Queue"),
        field: ring_key(QUEUE, "Queue/items"),
    };
    assert_eq!(
        counterexample.refinement,
        Some(RefinementFailure::Undefined {
            position: 1,
            row: row.clone()
        })
    );
    let CounterexampleKind::UndefinedEvaluation(evaluation) = counterexample.kind else {
        panic!("the kind is an undefined evaluation");
    };
    assert_eq!(evaluation.site.position, 1);
    assert_eq!(
        evaluation.site.locus,
        UndefinedLocus::Mapping(Box::new(MappingSite {
            row,
            object: Some(ObjectKey {
                population: ring_key(RING, "rings"),
                key: "r".to_owned(),
            }),
        }))
    );
}
