// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-555 step 6: `check_step` reads the receiver and every
//! argument of the step row and the rows of the post-state, and ranks what it
//! found: undefined outranks undetermined, the first of each kind is kept in
//! reading order, and a fault wins.
//!
//! The `Reads` fixture is the ring buffer with an abstract `enq(x, y)`, the
//! row `put -> enq(R, A1, A2)`, and for each of `R`, `A1`, `A2`, the
//! post-history update `H` (a history field of `Slot`) and the post-state row
//! `P` an expression that is cheap, over the meter budget `B`, undefined, or,
//! for `H`, out of its declared type.

use ix_trace_rs::trace;
use qsl_eval::simulation::model::{operation_identity_text, ModelSimState};
use qsl_replay::{
    check_step, initial_history, open_systems, ConcreteStep, HistoryValues, MappingRowRef,
    RefinementPosition, RefinementSystems, StepArgument, StepVerdict, Undetermined,
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
const UNDEFINED_RECEIVER: &str = "only(s in R::slots where s.ring = self.ring: s.ring)";
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
                &HistoryValues::default(),
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
        history: initial_history(world.refinement(), &systems.concrete, &pre),
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
#[trace("TC-555", "FR-141-AC-10")]
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
#[trace("TC-555", "FR-141-AC-10")]
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

/// The update of the history field `h` of `Slot` by `put` that is `update`,
/// added to `reads`' refinement.
fn with_history(world_text: &str, update: &str) -> String {
    world_text.replace(
        "step R::Slot::put",
        &format!("history R::Slot.h : Int[0, 1] = 0 {{ on R::Slot::put : {update}; }}\nstep R::Slot::put"),
    )
}

/// The `Reads` world whose history field `h` is updated by `update`, with
/// `enq(R, A1, A2)` and the `items` row `items`.
fn reads_history(update: &str, first: &str, items: &str) -> RingWorld {
    let refinement = with_history(
        &ring_is_queue(items).replace(
            "enq(self.ring, x)",
            &format!("enq({CHEAP_RECEIVER}, {first}, {CHEAP_ARGUMENT})"),
        ),
        update,
    );
    RingWorld::with(
        Variant {
            two_args: true,
            ..Variant::default()
        },
        &refinement,
    )
}

/// An update whose value is 2, outside `Int[0, 1]`, for `put(1)` on `s0`.
const OUT_OF_TYPE: &str = "self.index + x + 1";
/// An update that is undefined: two slots satisfy the condition.
const UNDEFINED_UPDATE: &str = "only(s in R::slots where s.ring = self.ring: s.value)";

fn history_row() -> MappingRowRef {
    MappingRowRef::History {
        object_type: key(RING, "Slot"),
        field: "h".to_owned(),
        operation: key(RING, "Slot/put"),
    }
}

/// `items` for a post-state row `P` over the meter budget, which costs far
/// more than the rest of the fixture wherever the ring holds an item.
fn items_over_budget() -> String {
    format!(
        "items = seq(i in 0 .. self.size - 1: only(s in R::slots where s.ring = self and s.index{} = (self.head + i) mod 2: s.value));",
        " + 0".repeat(400)
    )
}

/// FR-141-AC-10 (TC-555 step 6): an out-of-type post-history update `H` and
/// an over-budget post-state row `P` with nothing undefined is
/// `Undetermined` naming `H`; an out-of-type `H` and an over-budget `A1` is
/// `Undetermined` naming `A1`, which is read first.
#[trace("TC-555", "FR-141-AC-10")]
#[test]
fn an_out_of_type_update_is_undetermined_at_the_first_undetermined_read() {
    let b = budget();
    let world = reads_history(OUT_OF_TYPE, CHEAP_ARGUMENT, &items_over_budget());
    let found = verdict(&world, b, empty, one, false);
    assert!(matches!(found, StepVerdict::Undetermined(_)), "{found:?}");
    assert_eq!(locus_row(&found), history_row());

    let world = reads_history(OUT_OF_TYPE, &over_budget_argument(), ITEMS);
    let found = verdict(&world, b, empty, one, false);
    assert!(matches!(found, StepVerdict::Undetermined(_)), "{found:?}");
    assert_eq!(locus_row(&found), parameter(0));
}

/// FR-141-AC-10 (TC-555 step 6): an undefined `H` with an undefined `P` is
/// `Undefined` at `H`, which is read first; an out-of-type `H` with an
/// undefined `P` is `Undefined` at `P`, since an undefined read outranks an
/// undetermined one read before it.
#[trace("TC-555", "FR-141-AC-10")]
#[test]
fn an_undefined_update_is_read_before_the_post_state_rows() {
    let b = budget();
    let world = reads_history(UNDEFINED_UPDATE, CHEAP_ARGUMENT, ITEMS_UNDEFINED);
    let found = verdict(&world, b, empty, one, false);
    assert!(matches!(found, StepVerdict::Undefined(_)), "{found:?}");
    assert_eq!(locus_row(&found), history_row());

    let world = reads_history(OUT_OF_TYPE, CHEAP_ARGUMENT, ITEMS_UNDEFINED);
    let found = verdict(&world, b, empty, one, false);
    assert!(matches!(found, StepVerdict::Undefined(_)), "{found:?}");
    assert_eq!(locus_row(&found), items_row());
}

/// TC-555 step 6: an undefined receiver and an undefined `A1` give
/// `Undefined` at the receiver, which is read first; an undefined `A1` and
/// an undefined post-state row give `Undefined` at `A1`.
#[trace("TC-555", "FR-141-AC-10")]
#[test]
fn the_first_undefined_read_is_reported() {
    let b = budget();
    let world = reads(
        Variant::default(),
        UNDEFINED_RECEIVER,
        UNDEFINED_ARGUMENT,
        CHEAP_ARGUMENT,
        ITEMS,
    );
    let found = verdict(&world, b, empty, one, false);
    assert!(matches!(found, StepVerdict::Undefined(_)), "{found:?}");
    assert_eq!(
        locus_row(&found),
        MappingRowRef::Argument {
            operation: key(RING, "Slot/put"),
            argument: StepArgument::Receiver,
        }
    );

    let world = reads(
        Variant::default(),
        CHEAP_RECEIVER,
        UNDEFINED_ARGUMENT,
        CHEAP_ARGUMENT,
        ITEMS_UNDEFINED,
    );
    let found = verdict(&world, b, empty, one, false);
    assert!(matches!(found, StepVerdict::Undefined(_)), "{found:?}");
    assert_eq!(locus_row(&found), parameter(0));
}

/// FR-141-AC-10 (TC-555 step 6): an over-budget `H` is not evaluated into the
/// rows, but a post-state row that does not read it and is undefined is
/// `Undefined` at that row: an undefined read anywhere wins over the update's
/// `Undetermined`.
#[trace("TC-555", "FR-141-AC-10")]
#[test]
fn an_undefined_row_wins_over_an_over_budget_update() {
    let b = budget();
    let world = reads_history(&over_budget_argument(), CHEAP_ARGUMENT, ITEMS_UNDEFINED);
    let found = verdict(&world, b, empty, one, false);
    assert!(matches!(found, StepVerdict::Undefined(_)), "{found:?}");
    assert_eq!(locus_row(&found), items_row());
}

/// FR-138 / FR-141 (TC-555 step 6): a row of the ring that reaches `h` of
/// another object through a population read is not evaluated at a stand-in
/// either. `h` of `s0` leaves its type, and the `items` row reads `s.h` as
/// `1 mod (0 - s.h)`, which is undefined at the value the step started from
/// and defined at the computed 2: the step is `Undetermined` naming `h`.
#[trace("TC-555", "FR-141-AC-10")]
#[test]
fn a_row_reaching_a_failed_field_through_a_population_is_undetermined() {
    let b = budget();
    let items = "items = seq(i in 0 .. self.size - 1: only(s in R::slots where s.ring = self and s.index + 0 = (self.head + i) mod 2: if 1 mod (0 - s.h) = 0 then s.value else s.value));";
    let world = reads_history(OUT_OF_TYPE, CHEAP_ARGUMENT, items);
    let found = verdict(&world, b, empty, one, false);
    assert!(matches!(found, StepVerdict::Undetermined(_)), "{found:?}");
    assert_eq!(locus_row(&found), history_row());
}
