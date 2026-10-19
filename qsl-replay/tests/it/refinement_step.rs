// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-141 (TC-546): `check_initial` and `check_step` over ADR-020 §8's
//! `CasRefinesCounter` and its lost-update model.

use ix_trace_rs::trace;
use qsl_eval::simulation::model::{ModelSimState, ModelSystem};
use qsl_replay::{
    check_initial, check_step, map_state, ConcreteStep, RefinementFailure, RefinementPosition,
    StepRejection, StepVerdict, Taken,
};
use qsl_semantics::model::state::ModelState;
use quire_semantic_value::model_transition::ModelTransition;

use crate::refinement_fixture::{
    cas_refines_counter, cas_state, key, lost_update_refinement, mutated, spec_state, Variant,
    World, CAS, SPEC, UNLIMITED,
};

/// The concrete transition `operation` takes `pre` to `post`, with the state
/// it reaches, found in the concrete system's own expansion.
fn step(
    world: &World,
    pre: &ModelState,
    operation: &str,
    post: &ModelState,
) -> (ModelTransition, ModelSimState) {
    let system = world.concrete_system(UNLIMITED);
    let text = qsl_eval::simulation::model::operation_identity_text(&key(
        CAS,
        &format!("Counter/{operation}"),
    ))
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

/// The position of `concrete`, whose abstract state is its mapping.
fn position(world: &World, concrete: &ModelState) -> RefinementPosition {
    let mapped = map_state(
        world.refinement(),
        &world.concrete_system(UNLIMITED),
        &world.abstract_system(UNLIMITED),
        concrete,
    )
    .expect("the state maps");
    RefinementPosition {
        concrete: concrete.clone(),
        abstract_states: vec![mapped.visible],
    }
}

/// `check_step` on `operation` from `pre` to `post` at position `at`.
fn verdict(
    world: &World,
    pre: &ModelState,
    operation: &str,
    post: &ModelState,
    at: u64,
) -> StepVerdict {
    let (transition, reached) = step(world, pre, operation, post);
    check_step(
        world.refinement(),
        &world.concrete_system(UNLIMITED),
        &world.abstract_system(UNLIMITED),
        &position(world, pre),
        &ConcreteStep {
            position: at,
            transition: Some(&transition),
            post: &reached,
        },
        &mut 0,
    )
}

/// The one abstract transition `inc(c)` from `value`, as the abstract
/// system's own expansion names it.
fn inc(system: &ModelSystem<'_>, value: i64) -> ModelTransition {
    let mut successors = system
        .expand_state(&spec_state(value))
        .expect("the state expands")
        .successors;
    assert_eq!(successors.len(), 1);
    successors.remove(0).0
}

fn passes_with(verdict: StepVerdict) -> Taken {
    let StepVerdict::Passes { taken, .. } = verdict else {
        panic!("the step passes: {verdict:?}");
    };
    taken
}

fn fails_with(verdict: StepVerdict) -> RefinementFailure {
    let StepVerdict::Fails(failure) = verdict else {
        panic!("the step fails: {verdict:?}");
    };
    failure
}

/// FR-141-AC-1 (TC-546 step 1): the initial state and the `beginA` and
/// `commitA` steps of `CasRefinesCounter` pass, `commitA` as `inc(c)`.
#[trace("TC-546", "FR-141-AC-1")]
#[test]
fn the_initial_state_and_the_compare_and_set_steps_pass() {
    let world = World::new(&cas_refines_counter());
    let initial = cas_state(0, 0, false, 0, false);
    let StepVerdict::Passes { post, taken } = check_initial(
        world.refinement(),
        &world.concrete_system(UNLIMITED),
        &world.abstract_system(UNLIMITED),
        0,
        &initial,
    ) else {
        panic!("the initial state passes");
    };
    assert_eq!(taken, Taken::Stutter);
    assert_eq!(post.abstract_states.len(), 1);

    let begun = cas_state(0, 0, true, 0, false);
    assert_eq!(
        passes_with(verdict(&world, &initial, "beginA", &begun, 1)),
        Taken::Stutter
    );
    let committed = cas_state(1, 0, false, 0, false);
    let abstract_system = world.abstract_system(UNLIMITED);
    assert_eq!(
        passes_with(verdict(&world, &begun, "commitA", &committed, 2)),
        Taken::Abstract(vec![inc(&abstract_system, 0)])
    );
}

/// FR-141-AC-1 (TC-546 step 1): over the lost-update model `commitB` from
/// `(1, 0, f, 0, t)` to `(1, 0, f, 0, f)` fails `inc(c)`'s postcondition.
#[trace("TC-546", "FR-141-AC-1")]
#[test]
fn a_lost_update_fails_the_increment_postcondition() {
    let world = World::with(Variant::LOST_UPDATE, &lost_update_refinement());
    let pre = cas_state(1, 0, false, 0, true);
    let post = cas_state(1, 0, false, 0, false);
    let abstract_system = world.abstract_system(UNLIMITED);
    assert_eq!(
        fails_with(verdict(&world, &pre, "commitB", &post, 4)),
        RefinementFailure::AbstractStepRejected {
            position: 4,
            transition: inc(&abstract_system, 1),
            cause: StepRejection::Postcondition,
        }
    );
}

/// FR-141-AC-2 (TC-546 step 2): a concrete initial `value` of 1 maps to no
/// abstract initial state.
#[trace("TC-546", "FR-141-AC-2")]
#[test]
fn an_initial_state_with_another_value_is_not_abstract() {
    let world = World::with(
        Variant {
            concrete_value: 1,
            ..Variant::CAS
        },
        &cas_refines_counter(),
    );
    assert_eq!(
        fails_with(check_initial(
            world.refinement(),
            &world.concrete_system(UNLIMITED),
            &world.abstract_system(UNLIMITED),
            0,
            &cas_state(1, 0, false, 0, false),
        )),
        RefinementFailure::InitialNotAbstract { initial: 0 }
    );
}

/// FR-141-AC-2 (TC-546 step 2): `commitA` mapped `-> stutter` changes the
/// mapped value, so it fails the `stutter` rule.
#[trace("TC-546", "FR-141-AC-2")]
#[test]
fn a_stutter_row_that_changes_the_mapped_state_fails() {
    let refinement = mutated(
        &cas_refines_counter(),
        "step Impl::Counter::commitA -> Spec::Counter::inc(self);",
        "step Impl::Counter::commitA -> stutter;",
    );
    let world = World::new(&refinement);
    assert_eq!(
        fails_with(verdict(
            &world,
            &cas_state(0, 0, true, 0, false),
            "commitA",
            &cas_state(1, 0, false, 0, false),
            2,
        )),
        RefinementFailure::StutterChanged { position: 2 }
    );
}

/// FR-141-AC-2 (TC-546 step 2): with both commit rows `-> any`, the
/// lost-update `commitB` passes as a stutter, and a commit that lowers
/// `value` matches no abstract step.
#[trace("TC-546", "FR-141-AC-2")]
#[test]
fn an_any_row_passes_a_stutter_and_fails_a_step_no_abstract_step_matches() {
    let refinement = lost_update_refinement()
        .replace(
            "step Impl::Counter::commitA -> Spec::Counter::inc(self);",
            "step Impl::Counter::commitA -> any;",
        )
        .replace(
            "step Impl::Counter::commitB -> Spec::Counter::inc(self);",
            "step Impl::Counter::commitB -> any;",
        );
    let world = World::with(Variant::LOST_UPDATE, &refinement);
    assert_eq!(
        passes_with(verdict(
            &world,
            &cas_state(1, 0, false, 0, true),
            "commitB",
            &cas_state(1, 0, false, 0, false),
            4,
        )),
        Taken::Stutter
    );
    assert_eq!(
        fails_with(verdict(
            &world,
            &cas_state(2, 1, false, 0, true),
            "commitB",
            &cas_state(1, 1, false, 0, false),
            6,
        )),
        RefinementFailure::NoAbstractMatch { position: 6 }
    );
}

/// FR-141-AC-2 (TC-546 step 2): `inc(c)` at value 2 is rejected by `CanInc`.
#[trace("TC-546", "FR-141-AC-2")]
#[test]
fn an_increment_at_the_top_is_rejected_by_its_precondition() {
    let world = World::with(Variant::LOST_UPDATE, &lost_update_refinement());
    let abstract_system = world.abstract_system(UNLIMITED);
    let failure = fails_with(verdict(
        &world,
        &cas_state(2, 1, true, 0, false),
        "commitA",
        &cas_state(2, 1, false, 0, false),
        3,
    ));
    let RefinementFailure::AbstractStepRejected {
        position,
        transition,
        cause,
    } = failure
    else {
        panic!("the abstract step is rejected: {failure:?}");
    };
    assert_eq!(position, 3);
    assert_eq!(transition.operation(), inc(&abstract_system, 0).operation());
    assert_eq!(
        cause,
        StepRejection::Precondition {
            clause: "CanInc".to_owned()
        }
    );
    let _ = SPEC;
}

/// FR-141-AC-8 (TC-555 step 4): the two systems passed swapped are a
/// `WrongSystem` of FR-140 inside `check_step` and `check_initial`, which is
/// an internal fault of the caller, never a verdict.
#[trace("TC-555", "FR-141-AC-8")]
#[test]
fn a_swapped_system_is_a_fault_not_a_verdict() {
    let world = World::new(&cas_refines_counter());
    let pre = cas_state(0, 0, false, 0, false);
    let post = cas_state(0, 0, true, 0, false);
    let (transition, reached) = step(&world, &pre, "beginA", &post);
    let concrete = world.concrete_system(UNLIMITED);
    let abstracted = world.abstract_system(UNLIMITED);
    let verdict = check_step(
        world.refinement(),
        &abstracted,
        &concrete,
        &position(&world, &pre),
        &ConcreteStep {
            position: 1,
            transition: Some(&transition),
            post: &reached,
        },
        &mut 0,
    );
    assert!(matches!(verdict, StepVerdict::Fault(_)), "{verdict:?}");
    let verdict = check_initial(world.refinement(), &abstracted, &concrete, 0, &pre);
    assert!(matches!(verdict, StepVerdict::Fault(_)), "{verdict:?}");
}
