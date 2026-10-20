// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-141-AC-9, FR-145-AC-6 (TC-555, TC-550): an abstract contract that costs
//! more than the per-evaluation meter stops the run with the limit and its
//! value; it is neither a fault nor a parity disagreement.

use ix_trace_rs::trace;
use qsl_analyze::model_check::{check_refinement, RefinementCheckRequest};
use qsl_eval::model_subject::ModelSubject;
use qsl_eval::simulation::model::{operation_identity_text, ModelSimState};
use qsl_replay::{
    check_step, map_state, open_systems, replay_refinement_trace, settle_refinement_safety,
    ConcreteStep, CounterexampleKind, Execution, IncompleteCause, ModelCheckLimit,
    ModelCheckLimits, ModelStep, ReachedLimit, RefinementFailure, RefinementPosition,
    RefinementSystems, RefinementTraceSettlement, SafetyOutcome, StepRejection, StepVerdict,
    TemporalCounterexample, TerminalValue,
};
use quire_exact::{Cancel, ScalarLimits};
use quire_semantic_value::model_transition::ModelTransition;

use crate::ring_world::{key, ring_state, RingWorld, Variant, QUEUE, RING, UNLIMITED};

/// `UNLIMITED` with the work-unit budget `work`.
fn meter(work: u64) -> ScalarLimits {
    ScalarLimits {
        work_units: work,
        ..UNLIMITED
    }
}

fn run(world: &RingWorld, work: u64) -> SafetyOutcome {
    check_refinement(
        RefinementCheckRequest {
            refinement: world.refinement(),
            concrete: ModelSubject {
                simulation: world.concrete_request(meter(work)),
                proof_bounds: Vec::new(),
            },
            abstract_package: world.package(),
            abstract_initial: vec![world.abstract_initial()],
            max_depth: None,
            limits: ModelCheckLimits::default(),
        },
        &Cancel::new(),
    )
    .expect("the refinement is checked")
    .safety
}

/// The least per-evaluation budget under which the light ring model, whose
/// concrete contracts, rows and arguments are `HeavyEnq`'s, holds: `B`.
fn budget() -> u64 {
    let light = RingWorld::new();
    let (mut low, mut high) = (1, 1 << 20);
    while low < high {
        let middle = low + (high - low) / 2;
        if matches!(run(&light, middle), SafetyOutcome::Holds { .. }) {
            high = middle;
        } else {
            low = middle + 1;
        }
    }
    low
}

fn heavy() -> RingWorld {
    RingWorld::variant(Variant {
        heavy: 150,
        ..Variant::default()
    })
}

fn systems<'w>(world: &'w RingWorld, work: u64) -> RefinementSystems<'w> {
    open_systems(
        world.refinement(),
        &world.concrete_request(meter(work)),
        world.package(),
        vec![world.abstract_initial()],
    )
    .expect("the subjects open")
}

/// The `put(1)` step from the empty queue: its transition, the state it
/// reaches, and the position before it.
fn put(
    world: &RingWorld,
    systems: &RefinementSystems<'_>,
) -> (ModelTransition, ModelSimState, RefinementPosition) {
    let pre = ring_state(&systems.concrete, 0, 0, [0, 0]);
    let post = ring_state(&systems.concrete, 0, 1, [1, 0]);
    let text = operation_identity_text(&key(RING, "Slot/put")).expect("a text");
    let wanted = systems.concrete.state_key_of(&post).expect("a key").0;
    let (transition, reached) = systems
        .concrete
        .expand_state(&pre)
        .expect("the state expands")
        .successors
        .into_iter()
        .find(|(transition, reached)| {
            transition.operation() == text
                && systems
                    .concrete
                    .state_key_of(reached.state())
                    .expect("a key")
                    .0
                    == wanted
        })
        .expect("put(1) takes the empty ring to one item");
    let mapped = map_state(
        world.refinement(),
        &systems.concrete,
        &systems.abstract_system,
        &pre,
    )
    .expect("the state maps");
    (
        transition,
        reached,
        RefinementPosition {
            concrete: pre,
            abstract_states: vec![mapped.visible],
        },
    )
}

/// FR-141-AC-9 (TC-555 step 5): with the meter at `B`, `check_step` on
/// `put(1)` from the empty queue stops with `EvaluationMeter` and value `B`,
/// not a fault.
#[trace("TC-555", "FR-141-AC-9")]
#[test]
fn an_abstract_contract_over_the_budget_stops_the_step_check() {
    let b = budget();
    let world = heavy();
    let systems = systems(&world, b);
    let (transition, reached, pre) = put(&world, &systems);
    let result = check_step(
        world.refinement(),
        &systems.concrete,
        &systems.abstract_system,
        &pre,
        &ConcreteStep {
            position: 1,
            transition: Some(&transition),
            post: &reached,
        },
    );
    let StepVerdict::Stopped(limit) = result.verdict else {
        panic!("the check stops: {:?}", result.verdict);
    };
    assert_eq!(
        limit,
        ReachedLimit {
            limit: ModelCheckLimit::EvaluationMeter,
            value: b
        }
    );
}

/// FR-141-AC-9 (TC-555 step 5): FR-142 over `HeavyEnq` with that meter ends
/// `Stopped{ResourceExhausted, {EvaluationMeter, B}}`, and FR-144 settles it
/// as V-7: `resource-incomplete`, `Incomplete(LimitReached{..})`, never
/// `Failed`.
#[trace("TC-555", "FR-141-AC-9")]
#[test]
fn a_run_over_the_budget_stops_and_settles_incomplete() {
    let b = budget();
    let world = heavy();
    let outcome = run(&world, b);
    let SafetyOutcome::Stopped { cause, limit, .. } = &outcome else {
        panic!("the run stops: {outcome:?}");
    };
    assert_eq!(*cause, IncompleteCause::ResourceExhausted);
    let reached = ReachedLimit {
        limit: ModelCheckLimit::EvaluationMeter,
        value: b,
    };
    assert_eq!(*limit, Some(reached));
    let record = settle_refinement_safety(&outcome, &ModelCheckLimits::default(), |_| {
        panic!("a stopped run has no counterexample")
    });
    assert_eq!(
        record.value,
        TerminalValue::Incomplete(IncompleteCause::LimitReached {
            limit: ModelCheckLimit::EvaluationMeter,
            value: b
        })
    );
    assert_eq!(record.execution, Execution::ResourceIncomplete);
    assert_eq!(record.limit_reached, Some(reached));
}

/// FR-145-AC-6 (TC-550 step 5): replaying a hand-built payload, a recorded
/// `AbstractStepRejected` over `HeavyEnq`'s `put(1)` step, under the meter
/// `B` settles V-7 with the limit and value, never `ReplayParity`.
#[trace("TC-550", "FR-145-AC-6")]
#[test]
fn a_replay_over_the_budget_settles_incomplete_not_parity() {
    let b = budget();
    let world = heavy();
    let systems = systems(&world, b);
    let (transition, reached, _) = put(&world, &systems);
    let digest = systems.concrete.state_key(&reached).expect("a key").1;
    let recorded = RefinementFailure::AbstractStepRejected {
        position: 1,
        transition: ModelTransition::new(
            operation_identity_text(&key(QUEUE, "Queue/enq")).expect("a text"),
            transition.receiver().map(str::to_owned),
            Vec::new(),
            quire_canonical::Limits::new(u64::MAX),
        )
        .expect("an identity"),
        cause: StepRejection::Postcondition,
    };
    let payload = TemporalCounterexample {
        initial_state: 0,
        prefix: vec![ModelStep {
            transition,
            post_state: digest,
        }],
        over_binding: None,
        fairness: Vec::new(),
        kind: CounterexampleKind::Formula,
        lasso: None,
        refinement: Some(recorded),
    };
    let settlement =
        replay_refinement_trace(world.refinement(), &systems, &payload).expect("the trace replays");
    let RefinementTraceSettlement::Stopped(limit) = settlement else {
        panic!("the replay stops: {settlement:?}");
    };
    assert_eq!(
        limit,
        ReachedLimit {
            limit: ModelCheckLimit::EvaluationMeter,
            value: b
        }
    );
}
