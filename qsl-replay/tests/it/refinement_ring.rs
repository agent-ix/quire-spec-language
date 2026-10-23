// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-140 and FR-141 (TC-545, TC-546, TC-555, TC-1086) over ADR-020 §8's
//! ring buffer refined to a queue.

use ix_trace_rs::trace;
use qsl_eval::simulation::model::{operation_identity_text, ModelSimState, ModelSystem};
use qsl_eval::temporal::UndefinedCause;
use qsl_replay::{
    check_step, map_state, ConcreteStep, EvalFailure, HistoryValues, MappingFailure, MappingRowRef,
    RefinementFailure, RefinementPosition, StepArgument, StepRejection, StepVerdict, Taken,
    Undetermined,
};
use qsl_semantics::model::state::ModelState;
use quire_exact::{Integer, Value, ValueType};
use quire_semantic_value::catalog::UndefinedReason;
use quire_semantic_value::model_transition::ModelTransition;

use crate::ring_world::{
    key, ring_is_queue, ring_state, RingWorld, Variant, QUEUE, RING, UNLIMITED, ZERO,
};

fn integer(value: i64) -> Value {
    Value::Integer(Integer::from(value))
}

/// The abstract state with the queue `r` holding `items`.
fn queue_state(system: &ModelSystem<'_>, items: &[i64]) -> ModelState {
    let queue = key(QUEUE, "Queue");
    let field = system
        .fields_of(&queue)
        .and_then(|fields| fields.iter().find(|field| field.name == "items"))
        .expect("the queue declares items");
    let ValueType::Collection(declared) = &field.value_type else {
        panic!("items is a collection");
    };
    let items = quire_exact::from_admitted(
        (**declared).clone(),
        items.iter().map(|item| integer(*item)).collect(),
    );
    let object = qsl_semantics::model::state::StateObject {
        type_identity: queue,
        fields: std::collections::BTreeMap::from([(field.key.clone(), items)]),
    };
    ModelState {
        populations: std::collections::BTreeMap::from([(
            key(QUEUE, "queues"),
            std::collections::BTreeMap::from([("r".to_owned(), object)]),
        )]),
        extents: Vec::new(),
    }
}

/// FR-140-AC-2 (TC-545 step 2): at `head = 1`, `size = 2`, `s0.value = 0`,
/// `s1.value = 1` the mapped state holds one `Queue` with key `r` and
/// `items = [1, 0]`, and no object for either slot. FR-140-AC-10 (TC-1086
/// step 5): it lists no relationship extent.
#[trace("TC-545", "FR-140-AC-2")]
#[trace("TC-1086", "FR-140-AC-10")]
#[test]
fn the_slots_map_to_the_items_of_one_queue() {
    let world = RingWorld::new();
    let concrete = world.concrete_system(UNLIMITED);
    let abstracted = world.abstract_system(UNLIMITED);
    let mapped = map_state(
        world.refinement(),
        &concrete,
        &abstracted,
        &ring_state(&concrete, 1, 2, [0, 1]),
        &HistoryValues::default(),
    )
    .expect("the state maps");
    assert!(mapped.hidden.is_empty());
    assert_eq!(mapped.visible.populations.len(), 1);
    assert_eq!(
        mapped.visible.populations[&key(QUEUE, "queues")]
            .keys()
            .collect::<Vec<_>>(),
        ["r"]
    );
    assert!(mapped.visible.extents.is_empty());
    assert_eq!(
        abstracted.state_key_of(&mapped.visible).expect("a key").0,
        abstracted
            .state_key_of(&queue_state(&abstracted, &[1, 0]))
            .expect("a key")
            .0
    );
}

/// FR-140-AC-3 (TC-545 step 3) and FR-140-AC-9 (TC-1086 step 4): with the
/// `only` condition `s.ring = self` the `items` row is undefined wherever the
/// queue is not empty, with reason `only-not-one`.
#[trace("TC-545", "FR-140-AC-3")]
#[trace("TC-1086", "FR-140-AC-9")]
#[test]
fn an_only_that_finds_two_objects_is_undefined() {
    let world = RingWorld::variant(Variant {
        only_ring: true,
        ..Variant::default()
    });
    let concrete = world.concrete_system(UNLIMITED);
    let abstracted = world.abstract_system(UNLIMITED);
    for (head, size) in [(0, 1), (1, 2), (0, 2), (1, 1)] {
        let Err(MappingFailure::Undefined { row, object, cause }) = map_state(
            world.refinement(),
            &concrete,
            &abstracted,
            &ring_state(&concrete, head, size, [0, 1]),
            &HistoryValues::default(),
        ) else {
            panic!("the mapping is undefined at {head}, {size}");
        };
        assert_eq!(
            row,
            MappingRowRef::Field {
                object_type: key(QUEUE, "Queue"),
                field: key(QUEUE, "Queue/items"),
            }
        );
        assert_eq!(object.expect("an object").key, "r");
        let UndefinedCause::Family(record) = *cause else {
            panic!("the cause is a family record");
        };
        assert_eq!(record.reason, UndefinedReason::OnlyNotOne);
    }
    assert!(map_state(
        world.refinement(),
        &concrete,
        &abstracted,
        &ring_state(&concrete, 1, 0, [0, 1]),
        &HistoryValues::default(),
    )
    .is_ok());
}

/// FR-140-AC-5 (TC-555 step 1): with a per-evaluation meter budget of zero
/// the `items` row is incomplete, so the mapping is undetermined at the row
/// and object `r`.
#[trace("TC-555", "FR-140-AC-5")]
#[test]
fn an_incomplete_items_row_leaves_the_mapping_undetermined() {
    let world = RingWorld::new();
    let concrete = world.concrete_system(ZERO);
    let abstracted = world.abstract_system(UNLIMITED);
    let Err(MappingFailure::Undetermined(undetermined)) = map_state(
        world.refinement(),
        &concrete,
        &abstracted,
        &ring_state(&concrete, 1, 2, [0, 1]),
        &HistoryValues::default(),
    ) else {
        panic!("the mapping is undetermined");
    };
    assert_eq!(
        undetermined.row,
        MappingRowRef::Field {
            object_type: key(QUEUE, "Queue"),
            field: key(QUEUE, "Queue/items"),
        }
    );
    assert_eq!(undetermined.object.expect("an object").key, "r");
    assert!(
        matches!(undetermined.cause, EvalFailure::Evaluation(_)),
        "{:?}",
        undetermined.cause
    );
}

/// The concrete transition `operation` takes `pre` to `post`, with the state
/// it reaches.
fn step(
    system: &ModelSystem<'_>,
    pre: &ModelState,
    operation: &str,
    post: &ModelState,
) -> (ModelTransition, ModelSimState) {
    let text = operation_identity_text(&key(RING, &format!("Slot/{operation}")))
        .expect("an identity text");
    let wanted = system.state_key_of(post).expect("a key").0;
    system
        .expand_state(pre)
        .expect("the state expands")
        .successors
        .into_iter()
        .find(|(transition, reached)| {
            transition.operation() == text
                && system.state_key_of(reached.state()).expect("a key").0 == wanted
        })
        .unwrap_or_else(|| panic!("{operation} does not take the state to the other"))
}

fn position(world: &RingWorld, concrete: &ModelState) -> RefinementPosition {
    let mapped = map_state(
        world.refinement(),
        &world.concrete_system(UNLIMITED),
        &world.abstract_system(UNLIMITED),
        concrete,
        &HistoryValues::default(),
    )
    .expect("the state maps");
    RefinementPosition {
        concrete: concrete.clone(),
        history: HistoryValues::default(),
        abstract_states: vec![mapped.visible],
    }
}

/// `check_step` on `operation` from `pre` to `post` at position 1, with the
/// concrete system `concrete`.
fn verdict_with(
    world: &RingWorld,
    concrete: &ModelSystem<'_>,
    abstracted: &ModelSystem<'_>,
    pre: &ModelState,
    operation: &str,
    post: &ModelState,
) -> StepVerdict {
    let (transition, reached) = step(&world.concrete_system(UNLIMITED), pre, operation, post);
    check_step(
        world.refinement(),
        concrete,
        abstracted,
        &position(world, pre),
        &ConcreteStep {
            position: 1,
            transition: Some(&transition),
            post: &reached,
        },
    )
    .verdict
}

fn verdict(world: &RingWorld, pre: &ModelState, operation: &str, post: &ModelState) -> StepVerdict {
    verdict_with(
        world,
        &world.concrete_system(UNLIMITED),
        &world.abstract_system(UNLIMITED),
        pre,
        operation,
        post,
    )
}

/// The one abstract transition `operation` takes the abstract `pre` to
/// `post`, as the abstract system's own expansion names it.
fn abstract_transition(
    system: &ModelSystem<'_>,
    pre: &[i64],
    operation: &str,
    post: &[i64],
) -> ModelTransition {
    let wanted = system
        .state_key_of(&queue_state(system, post))
        .expect("a key")
        .0;
    let text = operation_identity_text(&key(QUEUE, &format!("Queue/{operation}")))
        .expect("an identity text");
    system
        .expand_state(&queue_state(system, pre))
        .expect("the state expands")
        .successors
        .into_iter()
        .find(|(transition, reached)| {
            transition.operation() == text
                && system.state_key_of(reached.state()).expect("a key").0 == wanted
        })
        .map(|(transition, _)| transition)
        .expect("the abstract transition exists")
}

/// FR-141-AC-3 (TC-546 step 3): `take` from `head = 1`, `size = 2`,
/// `s0.value = 0`, `s1.value = 1`, returning 1, passes RS-4 as `deq(r)` with
/// the result bound.
#[trace("TC-546", "FR-141-AC-3")]
#[test]
fn a_take_that_returns_the_first_item_passes_as_a_dequeue() {
    let world = RingWorld::new();
    let concrete = world.concrete_system(UNLIMITED);
    let abstracted = world.abstract_system(UNLIMITED);
    let StepVerdict::Passes { taken, .. } = verdict(
        &world,
        &ring_state(&concrete, 1, 2, [0, 1]),
        "take",
        &ring_state(&concrete, 0, 1, [0, 1]),
    ) else {
        panic!("the take passes");
    };
    assert_eq!(
        taken,
        Taken::Abstract(vec![abstract_transition(&abstracted, &[1, 0], "deq", &[0])])
    );
}

/// FR-141-AC-3 (TC-546 step 3): the broken `take`, which returns the slot
/// after the head, returns 0 from the same state and fails `deq(r)`'s
/// postcondition.
#[trace("TC-546", "FR-141-AC-3")]
#[test]
fn a_take_that_returns_the_second_item_fails_the_dequeue_postcondition() {
    let world = RingWorld::variant(Variant {
        broken_take: true,
        ..Variant::default()
    });
    let concrete = world.concrete_system(UNLIMITED);
    let abstracted = world.abstract_system(UNLIMITED);
    let StepVerdict::Fails(failure) = verdict(
        &world,
        &ring_state(&concrete, 1, 2, [0, 1]),
        "take",
        &ring_state(&concrete, 0, 1, [0, 1]),
    ) else {
        panic!("the take fails");
    };
    assert_eq!(
        failure,
        RefinementFailure::AbstractStepRejected {
            position: 1,
            transition: abstract_transition(&abstracted, &[1, 0], "deq", &[0]),
            cause: StepRejection::Postcondition,
        }
    );
}

/// FR-141-AC-3 (TC-546 step 3): with `put -> enq(self.ring, _)`, `put(1)`
/// from the empty queue passes with `enq(r, 1)` only, the `_` argument
/// ranging over its domain.
#[trace("TC-546", "FR-141-AC-3")]
#[test]
fn an_any_argument_ranges_over_its_domain_and_only_the_value_put_passes() {
    let refinement = ring_is_queue(
        "items = seq(i in 0 .. self.size - 1: only(s in R::slots where s.ring = self and s.index + 0 = (self.head + i) mod 2: s.value));",
    )
    .replace("enq(self.ring, x)", "enq(self.ring, _)");
    let world = RingWorld::with(Variant::default(), &refinement);
    let concrete = world.concrete_system(UNLIMITED);
    let abstracted = world.abstract_system(UNLIMITED);
    let StepVerdict::Passes { taken, .. } = verdict(
        &world,
        &ring_state(&concrete, 0, 0, [0, 0]),
        "put",
        &ring_state(&concrete, 0, 1, [1, 0]),
    ) else {
        panic!("the put passes");
    };
    assert_eq!(
        taken,
        Taken::Abstract(vec![abstract_transition(&abstracted, &[], "enq", &[1])])
    );
}

/// FR-141-AC-7 (TC-555 step 2): over the row `put -> enq(self.ring,
/// g(self.index))`, `put(1)` with receiver `s0` (index 0) from the empty
/// queue is undefined at the step-row argument, cause `division-by-zero`;
/// with receiver `s1` the argument is `g(1)`, which is defined.
#[trace("TC-555", "FR-141-AC-7")]
#[test]
fn an_undefined_step_row_argument_is_undefined() {
    let items = "items = seq(i in 0 .. self.size - 1: only(s in R::slots where s.ring = self and s.index + 0 = (self.head + i) mod 2: s.value));";
    let refinement = format!(
        "function g using v(i: Int[0, 1]): Int[0, 1] pure {{ if 1 mod i = 0 then 0 else 1 }}\n{}",
        ring_is_queue(items).replace("enq(self.ring, x)", "enq(self.ring, g(self.index))")
    );
    let world = RingWorld::with(Variant::default(), &refinement);
    let concrete = world.concrete_system(UNLIMITED);
    let StepVerdict::Undefined(undefined) = verdict(
        &world,
        &ring_state(&concrete, 0, 0, [0, 0]),
        "put",
        &ring_state(&concrete, 0, 1, [1, 0]),
    ) else {
        panic!("the argument is undefined");
    };
    assert_eq!(undefined.site.position, 1);
    assert_eq!(
        undefined.site.locus,
        qsl_replay::UndefinedLocus::Mapping(Box::new(qsl_replay::MappingSite {
            row: MappingRowRef::Argument {
                operation: key(RING, "Slot/put"),
                argument: StepArgument::Parameter(0),
            },
            object: Some(qsl_replay::ObjectKey {
                population: key(RING, "slots"),
                key: "s0".to_owned(),
            }),
        }))
    );
}

/// FR-141-AC-7 (TC-555 step 2): with receiver `s1` (index 1) the argument is
/// `g(1)` = 0, which is defined; the step then fails the postcondition of
/// `enq(r, 0)`, since `put(1)` wrote 1.
#[trace("TC-555", "FR-141-AC-7")]
#[test]
fn a_defined_step_row_argument_is_judged_against_the_post_state() {
    let items = "items = seq(i in 0 .. self.size - 1: only(s in R::slots where s.ring = self and s.index + 0 = (self.head + i) mod 2: s.value));";
    let refinement = format!(
        "function g using v(i: Int[0, 1]): Int[0, 1] pure {{ if 1 mod i = 0 then 0 else 1 }}\n{}",
        ring_is_queue(items).replace("enq(self.ring, x)", "enq(self.ring, g(self.index))")
    );
    let world = RingWorld::with(Variant::default(), &refinement);
    let concrete = world.concrete_system(UNLIMITED);
    let StepVerdict::Fails(RefinementFailure::AbstractStepRejected { cause, .. }) = verdict(
        &world,
        &ring_state(&concrete, 1, 0, [0, 0]),
        "put",
        &ring_state(&concrete, 1, 1, [0, 1]),
    ) else {
        panic!("the step fails the abstract postcondition");
    };
    assert_eq!(cause, StepRejection::Postcondition);
}

/// FR-141-AC-7 (TC-555 step 2): with the original rows and an evaluation
/// budget that admits no charge, the step-row argument `self.ring` is
/// incomplete, so the step is undetermined at the argument.
#[trace("TC-555", "FR-141-AC-7")]
#[test]
fn an_incomplete_step_row_argument_is_undetermined() {
    let world = RingWorld::new();
    let concrete = world.concrete_system(UNLIMITED);
    let starved = world.concrete_system(ZERO);
    let StepVerdict::Undetermined(Undetermined::Mapping(undetermined)) = verdict_with(
        &world,
        &starved,
        &world.abstract_system(UNLIMITED),
        &ring_state(&concrete, 0, 0, [0, 0]),
        "put",
        &ring_state(&concrete, 0, 1, [1, 0]),
    ) else {
        panic!("the step is undetermined");
    };
    assert_eq!(
        undetermined.row,
        MappingRowRef::Argument {
            operation: key(RING, "Slot/put"),
            argument: StepArgument::Receiver,
        }
    );
}

/// FR-142 (TC-547): an abstract system whose evaluation meter admits no
/// charge stops the step check with `Stopped`, not a fault: `take` from
/// `head = 1`, `size = 2` exhausts the meter at `deq`'s precondition.
#[trace("TC-547")]
#[test]
fn an_exhausted_abstract_meter_stops_the_step_check() {
    let world = RingWorld::new();
    let concrete = world.concrete_system(UNLIMITED);
    let starved = world.abstract_system(ZERO);
    let verdict = verdict_with(
        &world,
        &concrete,
        &starved,
        &ring_state(&concrete, 1, 2, [0, 1]),
        "take",
        &ring_state(&concrete, 0, 1, [0, 1]),
    );
    let StepVerdict::Stopped(reached) = verdict else {
        panic!("the check stops: {verdict:?}");
    };
    assert_eq!(reached.limit, qsl_replay::ModelCheckLimit::EvaluationMeter);
}
