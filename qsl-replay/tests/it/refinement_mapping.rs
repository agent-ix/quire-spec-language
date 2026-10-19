// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-140 (TC-545, TC-555): `map_state` over ADR-020 §8's `CasRefinesCounter`.

use ix_trace_rs::trace;
use qsl_foundation::diagnostic::Code;
use qsl_foundation::CatalogCode;
use qsl_replay::{map_state, EvalFailure, HiddenSlot, MappingFailure, MappingRowRef, ObjectKey};
use qsl_semantics::model::observation::AdmissionFailure;
use qsl_semantics::model::state::ModelState;
use quire_exact::ScalarLimits;

use crate::refinement_fixture::{
    cas_refines_counter, cas_state, key, mutated, spec_state, Variant, World, CAS, SPEC, THIRD,
    UNLIMITED, ZERO,
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

/// FR-140 "Behavior": a concrete object whose most-specific type no object
/// row names is reported as an unmapped type, never dropped from the image.
#[trace("TC-545")]
#[test]
fn an_object_of_an_unmapped_type_is_reported_not_dropped() {
    let world = World::new(&cas_refines_counter());
    let mut state = cas_state(1, 0, false, 0, true);
    let stray = key(CAS, "Stray");
    for object in state.populations.values_mut().flat_map(|o| o.values_mut()) {
        object.type_identity = stray.clone();
    }
    let MappingFailure::Undetermined(undetermined) = failure(&world, UNLIMITED, &state) else {
        panic!("the mapping is undetermined");
    };
    assert_eq!(undetermined.cause, EvalFailure::UnmappedType(stray.clone()));
    assert_eq!(
        undetermined.row,
        MappingRowRef::Object {
            concrete_type: stray
        }
    );
}

/// FR-120-AC-14 (TC-1086 step 6): `ModelSystem::over_model` refuses a model
/// the unit does not select with `invalid_runtime_input`/`wrong-role-mapping`
/// naming it, and a system over each selected model reports that model.
#[trace("TC-1086", "FR-120-AC-14")]
#[test]
fn a_system_over_an_unselected_model_is_refused() {
    let world = World::new(&cas_refines_counter());
    let Err(AdmissionFailure::Refused(record)) = world.unselected_system("example/not-selected")
    else {
        panic!("the system is refused");
    };
    assert_eq!(record.code, Code::InvalidRuntimeInput);
    assert_eq!(record.cause, "wrong-role-mapping");
    assert_eq!(record.fields["model"], "example/not-selected");
    assert_eq!(world.concrete_system(UNLIMITED).model(), Some(CAS));
    assert_eq!(world.abstract_system(UNLIMITED).model(), Some(SPEC));
    assert_eq!(world.third_system().model(), Some(THIRD));
}

/// FR-140-AC-7 (TC-1086 step 2): an object of a subtype that `counters`
/// admits and no object row names is an unmapped type, never dropped.
#[trace("TC-1086", "FR-140-AC-7")]
#[test]
fn an_object_of_an_admitted_subtype_with_no_object_row_is_undetermined() {
    let world = World::with(
        Variant {
            sub_counter: true,
            ..Variant::CAS
        },
        &cas_refines_counter(),
    );
    let mut state = cas_state(1, 0, false, 0, true);
    let sub = key(CAS, "SubCounter");
    for object in state.populations.values_mut().flat_map(|o| o.values_mut()) {
        object.type_identity = sub.clone();
    }
    let MappingFailure::Undetermined(undetermined) = failure(&world, UNLIMITED, &state) else {
        panic!("the mapping is undetermined");
    };
    assert_eq!(undetermined.cause, EvalFailure::UnmappedType(sub.clone()));
    assert_eq!(
        undetermined.row,
        MappingRowRef::Object { concrete_type: sub }
    );
    assert_eq!(undetermined.object.expect("an object").key, "c");
}

/// FR-140-AC-8 (TC-1086 step 3): the two systems passed swapped, or a system
/// over a third model, are `WrongSystem` with `wrong-role-mapping`.
#[trace("TC-1086", "FR-140-AC-8")]
#[test]
fn a_system_over_another_model_than_the_refinements_is_a_wrong_system() {
    let world = World::new(&cas_refines_counter());
    let state = cas_state(1, 0, false, 0, true);
    let refinement = world.refinement();
    let concrete = world.concrete_system(UNLIMITED);
    let abstracted = world.abstract_system(UNLIMITED);
    let third = world.third_system();
    for (concrete, abstracted) in [
        (&abstracted, &concrete),
        (&concrete, &third),
        (&third, &abstracted),
    ] {
        let Err(MappingFailure::WrongSystem(code)) =
            map_state(refinement, concrete, abstracted, &state)
        else {
            panic!("the systems are not the refinement's");
        };
        assert_eq!(
            code,
            CatalogCode::new("invalid_runtime_input", "wrong-role-mapping")
        );
    }
}

/// FR-140-AC-14 (TC-1086 step 9): a concrete state its own system cannot
/// observe is an internal fault, which is no row's outcome.
#[trace("TC-1086", "FR-140-AC-14")]
#[test]
fn a_state_the_evaluator_cannot_observe_is_a_fault() {
    let world = World::new(&cas_refines_counter());
    let mut state = cas_state(1, 0, false, 0, true);
    for object in state.populations.values_mut().flat_map(|o| o.values_mut()) {
        object.fields.remove(&key(CAS, "Counter/value"));
    }
    let MappingFailure::Fault(code) = failure(&world, UNLIMITED, &state) else {
        panic!("the mapping faults");
    };
    assert_eq!(
        code,
        CatalogCode::new("runtime_invariant", "established-invariant-broken")
    );
}
