// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-555 step 6: `check_step` reads the receiver and every
//! argument of the step row and the rows of the post-state, and ranks what it
//! found: undefined outranks undetermined, the first of each kind is kept in
//! reading order, and a fault wins.
//!
//! The `Reads` fixture is the ring buffer with an abstract `enq(x, y)`, the
//! row `put -> enq(R, A1, A2)`, and for each of `R`, `A1`, `A2` and the
//! post-state row `P` an expression that is cheap, over the meter budget `B`,
//! or undefined. The post-history update `H` and a history out-of-type case
//! need history fields (QSL-545) and are not built here.

use ix_trace_rs::trace;
use qsl_eval::simulation::model::{operation_identity_text, ModelSimState};
use qsl_replay::{
    check_step, open_systems, ConcreteStep, MappingRowRef, RefinementPosition, RefinementSystems,
    StepArgument, StepVerdict, Undetermined,
};
use qsl_semantics::model::state::ModelState;
use quire_exact::ScalarLimits;
use quire_semantic_value::model_transition::ModelTransition;

use crate::ring_world::{
    custom_state, key, ring_is_queue, ring_state, RingSpec, RingWorld, Variant, QUEUE, RING,
    UNLIMITED,
};

const ITEMS: &str = "items = seq(i in 0 .. self.size - 1: only(s in R::slots where s.ring = self and s.index + 0 = (self.head + i) mod 2: s.value));";
const ITEMS_UNDEFINED: &str =
    "items = seq(i in 0 .. self.size - 1: only(s in R::slots where s.ring = self: s.value));";
const CHEAP_RECEIVER: &str = "self.ring";
const CHEAP_ARGUMENT: &str = "x";
const UNDEFINED_ARGUMENT: &str = "only(s in R::slots where s.ring = self.ring: s.value)";

/// A condition that costs far more than any other expression of the fixture.
fn heavy() -> String {
    format!("s.index {} = self.index + 0", " + 0".repeat(400))
}

fn over_budget_receiver() -> String {
    format!("only(s in R::slots where {}: s.ring)", heavy())
}

fn over_budget_argument() -> String {
    format!("only(s in R::slots where {}: s.value)", heavy())
}

/// The per-evaluation budget `B` under which the cheap rows fit: that of the
/// light ring model.
fn budget() -> u64 {
    let light = RingWorld::new();
    let (mut low, mut high) = (1, 1 << 20);
    while low < high {
        let middle = low + (high - low) / 2;
        let work = ScalarLimits {
            work_units: middle,
            ..UNLIMITED
        };
        let systems = open_systems(
            light.refinement(),
            &light.concrete_request(work),
            light.package(),
            vec![light.abstract_initial()],
        )
        .expect("the subjects open");
        let state = ring_state(&systems.concrete, 2 % 2, 2, [0, 1]);
        if systems.concrete.expand_state(&state).is_ok()
            && qsl_replay::map_state(
                light.refinement(),
                &systems.concrete,
                &systems.abstract_system,
                &state,
            )
            .is_ok()
        {
            high = middle;
        } else {
            low = middle + 1;
        }
    }
    low
}

fn reads(variant: Variant, receiver: &str, first: &str, second: &str, items: &str) -> RingWorld {
    let refinement = ring_is_queue(items).replace(
        "enq(self.ring, x)",
        &format!("enq({receiver}, {first}, {second})"),
    );
    RingWorld::with(
        Variant {
            two_args: true,
            ..variant
        },
        &refinement,
    )
}

/// `put(1)` from `pre` to `post`: its transition and the state it reaches.
fn step_between(
    systems: &RefinementSystems<'_>,
    pre: &ModelState,
    post: &ModelState,
) -> (ModelTransition, ModelSimState) {
    let text = operation_identity_text(&key(RING, "Slot/put")).expect("a text");
    let wanted = systems.concrete.state_key_of(post).expect("a key").0;
    systems
        .concrete
        .expand_state(pre)
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
        .expect("put takes the state to the other")
}

/// `check_step` on the step from `pre` to `post`, in a world metered at `B`
/// (or at `swapped` systems).
fn verdict(
    world: &RingWorld,
    work: u64,
    pre: impl Fn(&qsl_eval::simulation::model::ModelSystem<'_>) -> ModelState,
    post: impl Fn(&qsl_eval::simulation::model::ModelSystem<'_>) -> ModelState,
    swap: bool,
) -> StepVerdict {
    let meter = ScalarLimits {
        work_units: work,
        ..UNLIMITED
    };
    let systems = open_systems(
        world.refinement(),
        &world.concrete_request(meter),
        world.package(),
        vec![world.abstract_initial()],
    )
    .expect("the subjects open");
    let (pre, post) = (pre(&systems.concrete), post(&systems.concrete));
    let (transition, reached) = step_between(&systems, &pre, &post);
    let position = RefinementPosition {
        concrete: pre,
        abstract_states: vec![systems.abstract_system.initial_states()[0].state().clone()],
    };
    let (concrete, abstracted) = if swap {
        (&systems.abstract_system, &systems.concrete)
    } else {
        (&systems.concrete, &systems.abstract_system)
    };
    check_step(
        world.refinement(),
        concrete,
        abstracted,
        &position,
        &ConcreteStep {
            position: 1,
            transition: Some(&transition),
            post: &reached,
        },
    )
    .verdict
}

fn empty(system: &qsl_eval::simulation::model::ModelSystem<'_>) -> ModelState {
    ring_state(system, 0, 0, [0, 0])
}

fn one(system: &qsl_eval::simulation::model::ModelSystem<'_>) -> ModelState {
    ring_state(system, 0, 1, [1, 0])
}

fn locus_row(verdict: &StepVerdict) -> MappingRowRef {
    match verdict {
        StepVerdict::Undefined(evaluation) => match &evaluation.site.locus {
            qsl_replay::UndefinedLocus::Mapping(site) => site.row.clone(),
            other => panic!("not a mapping locus: {other:?}"),
        },
        StepVerdict::Undetermined(Undetermined::Mapping(undetermined)) => undetermined.row.clone(),
        other => panic!("neither undefined nor undetermined: {other:?}"),
    }
}

fn parameter(index: usize) -> MappingRowRef {
    MappingRowRef::Argument {
        operation: key(RING, "Slot/put"),
        argument: StepArgument::Parameter(index),
    }
}

fn items_row() -> MappingRowRef {
    MappingRowRef::Field {
        object_type: key(QUEUE, "Queue"),
        field: key(QUEUE, "Queue/items"),
    }
}

/// TC-555 step 6: an over-budget receiver and an undefined `A1` give
/// `Undefined` at `A1`; an over-budget `A1` and an undefined post-state row
/// `P` give `Undefined` at `P`; two undefined arguments give `Undefined` at
/// `A1`; a swapped system with `A1` undefined gives a fault.
#[trace("TC-555")]
#[test]
fn an_undefined_row_outranks_an_over_budget_one_read_before_it() {
    let b = budget();
    let world = reads(
        Variant::default(),
        &over_budget_receiver(),
        UNDEFINED_ARGUMENT,
        CHEAP_ARGUMENT,
        ITEMS,
    );
    let found = verdict(&world, b, empty, one, false);
    assert!(matches!(found, StepVerdict::Undefined(_)), "{found:?}");
    assert_eq!(locus_row(&found), parameter(0));

    let world = reads(
        Variant::default(),
        CHEAP_RECEIVER,
        &over_budget_argument(),
        CHEAP_ARGUMENT,
        ITEMS_UNDEFINED,
    );
    let found = verdict(&world, b, empty, one, false);
    assert!(matches!(found, StepVerdict::Undefined(_)), "{found:?}");
    assert_eq!(locus_row(&found), items_row());

    let world = reads(
        Variant::default(),
        CHEAP_RECEIVER,
        UNDEFINED_ARGUMENT,
        UNDEFINED_ARGUMENT,
        ITEMS,
    );
    let found = verdict(&world, b, empty, one, false);
    assert!(matches!(found, StepVerdict::Undefined(_)), "{found:?}");
    assert_eq!(locus_row(&found), parameter(0));

    let found = verdict(&world, b, empty, one, true);
    assert!(matches!(found, StepVerdict::Fault(_)), "{found:?}");
}

/// TC-555 step 6: an over-budget argument with nothing undefined is
/// `Undetermined` at that argument, and a post-state row of an unmapped type
/// with nothing undefined is `Undetermined` at that row.
#[trace("TC-555")]
#[test]
fn an_over_budget_or_unmapped_row_with_nothing_undefined_is_undetermined() {
    let b = budget();
    let world = reads(
        Variant::default(),
        CHEAP_RECEIVER,
        &over_budget_argument(),
        CHEAP_ARGUMENT,
        ITEMS,
    );
    let found = verdict(&world, b, empty, one, false);
    assert!(matches!(found, StepVerdict::Undetermined(_)), "{found:?}");
    assert_eq!(locus_row(&found), parameter(0));

    let world = reads(
        Variant {
            next: true,
            ..Variant::default()
        },
        CHEAP_RECEIVER,
        CHEAP_ARGUMENT,
        CHEAP_ARGUMENT,
        &format!("{ITEMS} next = self.next;"),
    );
    let ring = |size: i64| RingSpec {
        key: "a",
        type_name: "Ring",
        head: 0,
        size,
        next: Some(("q", "SubRing")),
    };
    let sub = RingSpec {
        key: "q",
        type_name: "SubRing",
        head: 0,
        size: 0,
        next: Some(("q", "SubRing")),
    };
    let slots = |value: i64| [("a0", "a", 0, value), ("a1", "a", 1, 0)];
    let found = verdict(
        &world,
        b,
        |system| custom_state(system, &[ring(0), sub_of(&sub)], &slots(0)),
        |system| custom_state(system, &[ring(1), sub_of(&sub)], &slots(1)),
        false,
    );
    assert!(matches!(found, StepVerdict::Undetermined(_)), "{found:?}");
    assert_eq!(
        locus_row(&found),
        MappingRowRef::Field {
            object_type: key(QUEUE, "Queue"),
            field: key(QUEUE, "Queue/next"),
        }
    );
}

fn sub_of<'a>(sub: &RingSpec<'a>) -> RingSpec<'a> {
    RingSpec {
        key: sub.key,
        type_name: sub.type_name,
        head: sub.head,
        size: sub.size,
        next: sub.next,
    }
}
