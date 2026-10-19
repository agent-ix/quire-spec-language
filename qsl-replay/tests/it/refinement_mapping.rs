// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-140 (TC-545, TC-555): `map_state` over ADR-020 §8's `CasRefinesCounter`.

use ix_trace_rs::trace;
use qsl_replay::{map_state, EvalFailure, HiddenSlot, MappingFailure, MappingRowRef, ObjectKey};
use qsl_semantics::model::state::ModelState;
use quire_exact::ScalarLimits;

use crate::refinement_fixture::{
    cas_refines_counter, cas_state, key, mutated, spec_state, World, SPEC, UNLIMITED, ZERO,
};

/// The state key of the abstract state `concrete` maps to, under a clause
/// budget of `meter`.
fn mapped_key(world: &World, concrete: &ModelState) -> Vec<u8> {
    let abstract_system = world.abstract_system(UNLIMITED);
    let mapped = map_state(
        world.refinement(),
        &world.concrete_system(UNLIMITED),
        &abstract_system,
        concrete,
    )
    .expect("the state maps");
    assert!(mapped.hidden.is_empty(), "{:?}", mapped.hidden);
    abstract_system
        .state_key_of(&mapped.visible)
        .expect("the mapped state has a key")
        .0
}

fn failure(world: &World, meter: ScalarLimits, concrete: &ModelState) -> MappingFailure {
    map_state(
        world.refinement(),
        &world.concrete_system(meter),
        &world.abstract_system(UNLIMITED),
        concrete,
    )
    .expect_err("the state does not map")
}

/// FR-140-AC-1 (TC-545 step 1): `(1, 0, f, 0, t)` maps to the state with
/// `c.value = 1` whose key equals the key of that state built directly in
/// `Spec`'s system; `(0, 0, t, 0, f)` and `(0, 0, f, 0, f)` map to equal keys.
#[trace("TC-545", "FR-140-AC-1")]
#[test]
fn a_concrete_state_maps_to_the_abstract_state_with_its_value() {
    let world = World::new(&cas_refines_counter());
    let direct = world
        .abstract_system(UNLIMITED)
        .state_key_of(&spec_state(1))
        .expect("a key")
        .0;
    assert_eq!(mapped_key(&world, &cas_state(1, 0, false, 0, true)), direct);

    let busy_a = mapped_key(&world, &cas_state(0, 0, true, 0, false));
    let idle = mapped_key(&world, &cas_state(0, 0, false, 0, false));
    assert_eq!(busy_a, idle);
    assert_ne!(busy_a, direct);
}

/// FR-140 "Behavior": each abstract object has the key of its concrete
/// source and the abstract type its object row names, and the population
/// holds no other object.
#[trace("TC-545")]
#[test]
fn the_image_object_keeps_its_key_and_takes_the_mapped_type() {
    let world = World::new(&cas_refines_counter());
    let mapped = map_state(
        world.refinement(),
        &world.concrete_system(UNLIMITED),
        &world.abstract_system(UNLIMITED),
        &cas_state(2, 1, true, 0, false),
    )
    .expect("the state maps");
    let objects = &mapped.visible.populations[&key(SPEC, "counters")];
    assert_eq!(objects.keys().collect::<Vec<_>>(), ["c"]);
    assert_eq!(objects["c"].type_identity, key(SPEC, "Counter"));
    assert_eq!(mapped.visible.populations.len(), 1);
}

/// FR-140 "Behavior": a row evaluated with a meter that admits no charge is
/// incomplete, so the mapping is undetermined at the row and object and no
/// partial state is returned (FR-140-AC-5's mechanism, over the counter).
#[trace("TC-555")]
#[test]
fn an_incomplete_row_leaves_the_mapping_undetermined() {
    let world = World::new(&cas_refines_counter());
    let MappingFailure::Undetermined(undetermined) =
        failure(&world, ZERO, &cas_state(1, 0, false, 0, true))
    else {
        panic!("the mapping is undetermined");
    };
    assert_eq!(
        undetermined.row,
        MappingRowRef::Field {
            object_type: key(SPEC, "Counter"),
            field: key(SPEC, "Counter/value"),
        }
    );
    assert_eq!(undetermined.object.expect("an object").key, "c");
    assert!(
        matches!(undetermined.cause, EvalFailure::Evaluation(_)),
        "{:?}",
        undetermined.cause
    );
}

/// FR-140 "Behavior": a field no row defines is hidden; `map_state` lists it
/// on the mapped object, leaves it unset in `visible` and still returns the
/// rest of the state (ADR-020 AX-2 completes it, in a later slice).
#[trace("TC-545")]
#[test]
fn a_field_with_no_row_is_listed_as_hidden() {
    let refinement = mutated(&cas_refines_counter(), "{ value = self.value; }", "{ }");
    let world = World::new(&refinement);
    let mapped = map_state(
        world.refinement(),
        &world.concrete_system(UNLIMITED),
        &world.abstract_system(UNLIMITED),
        &cas_state(1, 0, false, 0, true),
    )
    .expect("the state maps");
    assert_eq!(
        mapped.hidden,
        [HiddenSlot {
            object: ObjectKey {
                population: key(SPEC, "counters"),
                key: "c".to_owned(),
            },
            field: key(SPEC, "Counter/value"),
        }]
    );
    let counter = &mapped.visible.populations[&key(SPEC, "counters")]["c"];
    assert!(counter.fields.is_empty(), "{:?}", counter.fields);
}
