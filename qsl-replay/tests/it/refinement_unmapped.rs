// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-140 (TC-1086): the rules of `map_state` that name an unmapped type
//! reached through a reference or an object, the first failure in reading
//! order, and a population reference mapped element by element.

use ix_trace_rs::trace;
use qsl_eval::temporal::UndefinedCause;
use qsl_foundation::diagnostic::UndefinedReason;
use qsl_replay::{map_state, EvalFailure, HistoryValues, MappingFailure, MappingRowRef};
use quire_exact::Value;

use crate::ring_world::{
    custom_state, key, ring_is_queue, MapWorld, RingSpec, RingWorld, Variant, ITEMS, QUEUE, RING,
    UNLIMITED,
};

const ITEMS_ROW: &str = "items = seq(i in 0 .. self.size - 1: only(s in R::slots where s.ring = self and s.index + 0 = (self.head + i) mod 2: s.value));";
const ONLY_ROW: &str =
    "items = seq(i in 0 .. self.size - 1: only(s in R::slots where s.ring = self: s.value));";

fn ring<'a>(key: &'a str, type_name: &'a str, next: Option<(&'a str, &'a str)>) -> RingSpec<'a> {
    RingSpec {
        key,
        type_name,
        head: 0,
        size: 0,
        next,
    }
}

fn next_world() -> RingWorld {
    RingWorld::with(
        Variant {
            next: true,
            ..Variant::default()
        },
        &ring_is_queue(&format!("{ITEMS_ROW} next = self.next;")),
    )
}

/// FR-140-AC-6 (TC-1086 step 1): a reference to an object of a type no object
/// row names is `Undetermined` at the referencing row and object, with
/// `UnmappedType`; with `q` of type `Ring` it maps to the `Queue` reference
/// with key `q`.
#[trace("TC-1086", "FR-140-AC-6")]
#[test]
fn a_reference_to_an_unmapped_type_is_undetermined_at_the_referencing_row() {
    let world = next_world();
    let concrete = world.concrete_system(UNLIMITED);
    let abstracted = world.abstract_system(UNLIMITED);
    let sub = key(RING, "SubRing");

    let state = custom_state(
        &concrete,
        &[
            ring("a", "Ring", Some(("q", "SubRing"))),
            ring("q", "SubRing", Some(("q", "SubRing"))),
        ],
        &[],
    );
    let Err(MappingFailure::Undetermined(undetermined)) = map_state(
        world.refinement(),
        &concrete,
        &abstracted,
        &state,
        &HistoryValues::default(),
    ) else {
        panic!("the mapping is undetermined");
    };
    assert_eq!(undetermined.cause, EvalFailure::UnmappedType(sub));
    assert_eq!(
        undetermined.row,
        MappingRowRef::Field {
            object_type: key(QUEUE, "Queue"),
            field: key(QUEUE, "Queue/next"),
        }
    );
    assert_eq!(undetermined.object.expect("an object").key, "a");

    let state = custom_state(
        &concrete,
        &[
            ring("a", "Ring", Some(("q", "Ring"))),
            ring("q", "Ring", Some(("q", "Ring"))),
        ],
        &[],
    );
    let mapped = map_state(
        world.refinement(),
        &concrete,
        &abstracted,
        &state,
        &HistoryValues::default(),
    )
    .expect("the state maps");
    let queue = &mapped.visible.populations[&key(QUEUE, "queues")]["a"];
    let Some(Value::Reference(next)) = queue.fields.get(&key(QUEUE, "Queue/next")) else {
        panic!("a.next is a reference");
    };
    assert_eq!(
        *next,
        abstracted
            .object_reference(&key(QUEUE, "Queue"), "q")
            .expect("a reference")
    );
}

/// FR-140-AC-9 (TC-1086 step 4): an undefined row outranks an unmapped type
/// that sorts before it; with a defined `items` row the unmapped type is the
/// failure.
#[trace("TC-1086", "FR-140-AC-9")]
#[test]
fn an_undefined_row_outranks_an_unmapped_type_that_sorts_first() {
    let state = |concrete| {
        let mut ring_r = ring("r", "Ring", None);
        ring_r.head = 1;
        ring_r.size = 2;
        custom_state(
            concrete,
            &[ring("a", "SubRing", None), ring_r],
            &[("s0", "r", 0, 0), ("s1", "r", 1, 1)],
        )
    };
    let sub_ring = Variant {
        sub_ring: true,
        ..Variant::default()
    };

    let world = RingWorld::with(sub_ring, &ring_is_queue(ONLY_ROW));
    let concrete = world.concrete_system(UNLIMITED);
    let abstracted = world.abstract_system(UNLIMITED);
    let Err(MappingFailure::Undefined { row, object, cause }) = map_state(
        world.refinement(),
        &concrete,
        &abstracted,
        &state(&concrete),
        &HistoryValues::default(),
    ) else {
        panic!("the undefined row outranks the unmapped type");
    };
    assert_eq!(object.expect("an object").key, "r");
    assert_eq!(
        row,
        MappingRowRef::Field {
            object_type: key(QUEUE, "Queue"),
            field: key(QUEUE, "Queue/items"),
        }
    );
    let UndefinedCause::Family(record) = *cause else {
        panic!("the cause is a family record");
    };
    assert_eq!(record.reason, UndefinedReason::OnlyNotOne);

    let world = RingWorld::with(sub_ring, &ring_is_queue(ITEMS_ROW));
    let concrete = world.concrete_system(UNLIMITED);
    let abstracted = world.abstract_system(UNLIMITED);
    let Err(MappingFailure::Undetermined(undetermined)) = map_state(
        world.refinement(),
        &concrete,
        &abstracted,
        &state(&concrete),
        &HistoryValues::default(),
    ) else {
        panic!("the unmapped type is the failure");
    };
    assert_eq!(
        undetermined.cause,
        EvalFailure::UnmappedType(key(RING, "SubRing"))
    );
    assert_eq!(undetermined.object.expect("an object").key, "a");
}

/// FR-140-AC-12 (TC-1086 step 7): a whole-population reference row maps to
/// the abstract references with the same keys, in key order, each of the
/// abstract type.
#[trace("TC-1086", "FR-140-AC-12")]
#[test]
fn a_population_reference_row_maps_element_by_element() {
    let world = MapWorld::new(&format!("{ITEMS}\ncells = R::slots;"));
    let concrete = world.concrete_system();
    let abstracted = world.abstract_system();
    let state = crate::ring_world::ring_state(&concrete, 1, 2, [0, 1]);
    let mapped = map_state(
        world.refinement(),
        &concrete,
        &abstracted,
        &state,
        &HistoryValues::default(),
    )
    .expect("the state maps");
    let queue = &mapped.visible.populations[&key(QUEUE, "queues")]["r"];
    let Some(Value::Collection(cells)) = queue.fields.get(&key(QUEUE, "Queue/cells")) else {
        panic!("cells is a collection");
    };
    let expected: Vec<Value> = ["s0", "s1"]
        .iter()
        .map(|slot| {
            Value::Reference(
                abstracted
                    .object_reference(&key(QUEUE, "Cell"), slot)
                    .expect("a reference"),
            )
        })
        .collect();
    let found: Vec<String> = cells.elements().iter().map(|e| format!("{e:?}")).collect();
    let wanted: Vec<String> = expected.iter().map(|e| format!("{e:?}")).collect();
    assert_eq!(found, wanted);
}

/// FR-140-AC-13 (TC-1086 step 4): the failure reported is the first in
/// reading order, which does not depend on the order the rows were written:
/// the undefined `items` row of object `a`, not `b`; and with `items`
/// defined, object `a`'s field `x` before `y`.
#[trace("TC-1086", "FR-140-AC-13")]
#[test]
fn the_reported_failure_does_not_depend_on_the_source_order_of_the_rows() {
    let g = "function g using v(i: Int[0, 1]): Int[0, 1] pure { if 1 mod i = 0 then 0 else 1 }";
    let xy = Variant {
        xy: true,
        ..Variant::default()
    };
    for (items, expected_field) in [(ONLY_ROW, "items"), (ITEMS_ROW, "x")] {
        for rows in [
            format!("{items} x = g(self.head); y = g(self.head);"),
            format!("y = g(self.head); x = g(self.head); {items}"),
        ] {
            let world = RingWorld::with(xy, &format!("{g}\n{}", ring_is_queue(&rows)));
            let concrete = world.concrete_system(UNLIMITED);
            let abstracted = world.abstract_system(UNLIMITED);
            let mut ring_a = ring("a", "Ring", None);
            ring_a.size = 1;
            let mut ring_b = ring("b", "Ring", None);
            ring_b.size = 1;
            let state = custom_state(
                &concrete,
                &[ring_a, ring_b],
                &[
                    ("a0", "a", 0, 0),
                    ("a1", "a", 1, 1),
                    ("b0", "b", 0, 0),
                    ("b1", "b", 1, 1),
                ],
            );
            let Err(MappingFailure::Undefined { row, object, .. }) = map_state(
                world.refinement(),
                &concrete,
                &abstracted,
                &state,
                &HistoryValues::default(),
            ) else {
                panic!("the mapping is undefined");
            };
            assert_eq!(object.expect("an object").key, "a");
            assert_eq!(
                row,
                MappingRowRef::Field {
                    object_type: key(QUEUE, "Queue"),
                    field: key(QUEUE, &format!("Queue/{expected_field}")),
                },
                "{rows}"
            );
        }
    }
}
