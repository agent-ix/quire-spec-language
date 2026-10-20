// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-138 (TC-543, TC-554): the S3 check of `history` rows and the history
//! values `initial_history` and `step_history` compute along a concrete
//! behaviour, over `RegisterHistory`.

use ix_trace_rs::trace;
use qsl_eval::simulation::model::{ModelSimState, ModelSystem};
use qsl_foundation::Code;
use qsl_replay::{
    check_initial, check_step, initial_history, map_state, step_history, ConcreteStep, EvalFailure,
    HiddenSlot, HistoryValues, MappingFailure, MappingRowRef, ObjectKey, RefinementPosition,
    StepVerdict, Undetermined,
};
use qsl_semantics::check::CheckedGraph;
use qsl_semantics::model::state::ModelState;
use quire_exact::{Undefined, Value};
use quire_semantic_value::model_transition::ModelTransition;

use crate::refinement_aux::{
    int, register_history, Refused, Shape, Unit, World, INV, LAST, UNLIMITED, WRITES,
};

/// The concrete transition `write(r, x)` takes `pre` to the state with
/// `value` `x`, and the state it reaches, found in the concrete system's own
/// expansion.
fn write(world: &World<'_>, pre: &ModelState, x: i64) -> (ModelTransition, ModelSimState) {
    let system = world.concrete_system(UNLIMITED);
    let wanted = system
        .state_key_of(&world.concrete_state(&[("value", int(x))]))
        .expect("a key")
        .0;
    system
        .expand_state(pre)
        .expect("the state expands")
        .successors
        .into_iter()
        .find(|(_, reached)| system.state_key_of(reached.state()).expect("a key").0 == wanted)
        .unwrap_or_else(|| panic!("write(r, {x}) does not reach the state"))
}

fn register(unit: &Unit) -> ObjectKey {
    ObjectKey {
        population: unit.concrete_key("regs"),
        key: "r".to_owned(),
    }
}

fn integer(value: Option<&Value>) -> String {
    match value {
        Some(Value::Integer(integer)) => integer.to_string(),
        other => panic!("an integer history value, got {other:?}"),
    }
}

/// The concrete states along `writes`, each with the history the step gave,
/// from the initial state: `Ok` while the step defines every update.
fn along(
    world: &World<'_>,
    system: &ModelSystem<'_>,
    writes: &[i64],
) -> Vec<Result<HistoryValues, MappingFailure>> {
    let mut pre = world.concrete_state(&[("value", int(0))]);
    let mut history = initial_history(world.refinement(), system, &pre);
    let mut found = Vec::new();
    for x in writes {
        let (transition, reached) = write(world, &pre, *x);
        let next = step_history(
            world.refinement(),
            system,
            &history,
            &pre,
            &transition,
            &reached,
        );
        if let Ok(values) = &next {
            history = values.clone();
        }
        found.push(next);
        pre = reached.state().clone();
    }
    found
}

fn codes(found: &[Refused]) -> Vec<(Code, Option<&'static str>)> {
    found
        .iter()
        .map(|refused| (refused.refusal.cause.code(), refused.refusal.cause.cause()))
        .collect()
}

/// The clause identities of `graph`'s state clauses.
fn identities(graph: &CheckedGraph) -> Vec<quire_exact::NodeKey> {
    graph
        .state_clauses()
        .iter()
        .map(qsl_semantics::check::CheckedStateClause::identity)
        .collect()
}

const BOUNDED: &str =
    "invariant Bounded using v on Impl::Register at current { self.value >= 0 }\n";

/// FR-138-AC-1 (TC-543 step 1): `RegisterHistory` checks with one
/// `HistoryRow` for `last`, whose type is `Int[0, 1]`, initial value 0 and
/// one update, on `write`.
#[trace("TC-543", "FR-138-AC-1")]
#[test]
fn a_register_with_a_history_row_checks_to_one_history_row() {
    let unit = Unit::register();
    let world = unit.world(&register_history(LAST));
    let [row] = world.refinement().history() else {
        panic!("one history row");
    };
    assert_eq!(row.object_type, unit.concrete_key("Register"));
    assert_eq!(row.field, "last");
    assert_eq!(
        row.declared,
        quire_exact::ValueType::Int(
            quire_exact::IntegerInterval::new(0_i64.into(), 1_i64.into()).expect("a range")
        )
    );
    assert_eq!(integer(Some(&row.initial)), "0");
    let [update] = row.updates.as_slice() else {
        panic!("one update");
    };
    assert_eq!(update.operation, unit.concrete_key("Register/write"));
}

/// FR-138-AC-1 (TC-543 step 1): the concrete model's clause node
/// identities, its initial states and the successors `ModelSystem` gives
/// from its initial state are equal whether or not the refinement declares
/// the history row.
#[trace("TC-543", "FR-138-AC-1")]
#[test]
fn a_history_row_leaves_the_concrete_model_as_it_was() {
    let unit = Unit::register();
    let with = format!("{BOUNDED}{}", register_history(LAST));
    let without = format!(
        "{BOUNDED}refinement RegisterPlain using inf abstract Spec concrete Impl {{\n\
         population Spec::regs from Impl::regs;\n\
         object Spec::Register from Impl::Register {{ value = self.value; }}\n\
         step Impl::Register::write -> Spec::Register::write(self, x);\n\
         }}"
    );
    let (with, without) = (unit.world(&with), unit.world(&without));
    assert_eq!(
        identities(with.package().graph()),
        identities(without.package().graph())
    );
    let successors = |world: &World<'_>| {
        let system = world.concrete_system(UNLIMITED);
        let initial = world.concrete_state(&[("value", int(0))]);
        let keys: Vec<_> = system
            .initial_states()
            .iter()
            .map(|state| system.state_key_of(state.state()).expect("a key").0)
            .collect();
        let reached: Vec<_> = system
            .expand_state(&initial)
            .expect("the state expands")
            .successors
            .iter()
            .map(|(transition, state)| {
                (
                    system.transition_key(transition).expect("a key"),
                    system.state_key_of(state.state()).expect("a key").0,
                )
            })
            .collect();
        (keys, reached)
    };
    assert_eq!(successors(&with), successors(&without));
}

/// FR-138-AC-2 (TC-543 step 2): a history field named like a declared field
/// of the type refuses `invalid_model_binding`/`conflicting-binding`.
#[trace("TC-543", "FR-138-AC-2")]
#[test]
fn a_history_field_named_like_a_declared_field_is_refused() {
    let unit = Unit::register();
    let bad = "history Impl::Register.value : Int[0, 1] = 0 { }";
    let found = unit.refused(&register_history(&format!("{LAST}\n{bad}")));
    assert_eq!(
        codes(&found),
        [(Code::InvalidModelBinding, Some("conflicting-binding"))]
    );
}

/// FR-138-AC-2 (TC-543 step 2): an unbounded declared type refuses
/// `ill_typed`/`type-mismatch` at the type.
#[trace("TC-543", "FR-138-AC-2")]
#[test]
fn an_unbounded_history_type_is_refused_at_the_type() {
    let unit = Unit::register();
    let bad = "history Impl::Register.big : Integer = 0 { }";
    let found = unit.refused(&register_history(&format!("{LAST}\n{bad}")));
    assert_eq!(codes(&found), [(Code::IllTyped, Some("type-mismatch"))]);
    assert_eq!(found[0].at, "Integer");
}

/// FR-138-AC-2 (TC-543 step 2): an initial literal that is not a member of
/// the declared type's value set refuses `ill_typed`/`type-mismatch` at the
/// literal, decided statically.
#[trace("TC-543", "FR-138-AC-2")]
#[test]
fn an_initial_literal_outside_the_value_set_is_refused_at_the_literal() {
    let unit = Unit::register();
    for literal in ["true", "5"] {
        let bad = format!("history Impl::Register.other : Int[0, 1] = {literal} {{ }}");
        let found = unit.refused(&register_history(&format!("{LAST}\n{bad}")));
        assert_eq!(codes(&found), [(Code::IllTyped, Some("type-mismatch"))]);
        assert_eq!(found[0].at, literal);
    }
}

/// FR-138-AC-2 (TC-543 step 2): two updates for one operation refuse
/// `invalid_model_binding`/`conflicting-binding`.
#[trace("TC-543", "FR-138-AC-2")]
#[test]
fn two_updates_for_one_operation_are_refused() {
    let unit = Unit::register();
    let bad = "history Impl::Register.last : Int[0, 1] = 0 { on Impl::Register::write : pre(self.value); on Impl::Register::write : x; }";
    let found = unit.refused(&register_history(bad));
    assert_eq!(
        codes(&found),
        [(Code::InvalidModelBinding, Some("conflicting-binding"))]
    );
}

/// FR-138-AC-2 (TC-543 step 2): an update on an operation the type does not
/// declare refuses `missing_declaration`/`missing-name`.
#[trace("TC-543", "FR-138-AC-2")]
#[test]
fn an_update_on_an_unknown_operation_is_refused() {
    let unit = Unit::register();
    let bad = "history Impl::Register.last : Int[0, 1] = 0 { on Impl::Register::erase : 0; }";
    let found = unit.refused(&register_history(bad));
    assert_eq!(
        codes(&found),
        [(Code::MissingDeclaration, Some("missing-name"))]
    );
}

/// FR-138-AC-2 (TC-543 step 2): a history field is readable only inside the
/// declaring refinement, so an invariant of the concrete type reading
/// `self.last` refuses `missing_declaration`/`missing-name`.
#[trace("TC-543", "FR-138-AC-2")]
#[test]
fn a_concrete_clause_cannot_read_a_history_field() {
    let unit = Unit::register();
    let reads = "invariant Reads using v on Impl::Register at current { self.last = 0 }\n";
    let found = unit.refused(&format!("{reads}{}", register_history(LAST)));
    assert_eq!(
        codes(&found),
        [(Code::MissingDeclaration, Some("missing-name"))]
    );
}

/// FR-138-AC-3 (TC-543 step 3): along `write(r, 1)`, `write(r, 0)`,
/// `write(r, 1)` from the initial state, `step_history` gives `last` = 0, 1,
/// 0 after each step.
#[trace("TC-543", "FR-138-AC-3")]
#[test]
fn the_previous_value_is_kept_along_a_behaviour() {
    let unit = Unit::register();
    let world = unit.world(&register_history(LAST));
    let system = world.concrete_system(UNLIMITED);
    let initial = initial_history(
        world.refinement(),
        &system,
        &world.concrete_state(&[("value", int(0))]),
    );
    assert_eq!(integer(initial.get(&register(&unit), "last")), "0");
    let lasts: Vec<String> = along(&world, &system, &[1, 0, 1])
        .into_iter()
        .map(|history| {
            integer(
                history
                    .expect("every update is defined")
                    .get(&register(&unit), "last"),
            )
        })
        .collect();
    assert_eq!(lasts, ["0", "1", "0"]);
}

/// FR-138-AC-4 (TC-543 step 4): with the `writes` field added, `writes` is 1
/// after the first `write(r, 0)`, and the second returns
/// `MappingFailure::Undetermined` naming `writes`, `r` and the leaving of
/// the declared type, since the step stays one of the behaviour.
#[trace("TC-543", "FR-138-AC-4")]
#[test]
fn an_update_that_leaves_its_type_is_undetermined() {
    let unit = Unit::register();
    let world = unit.world(&register_history(&format!("{LAST}\n{WRITES}")));
    let system = world.concrete_system(UNLIMITED);
    let [first, second] = &along(&world, &system, &[0, 0])[..] else {
        panic!("two steps");
    };
    assert_eq!(
        integer(
            first
                .as_ref()
                .expect("defined")
                .get(&register(&unit), "writes")
        ),
        "1"
    );
    let Err(MappingFailure::Undetermined(undetermined)) = second else {
        panic!("the second update is undetermined: {second:?}");
    };
    assert_eq!(
        undetermined.row,
        MappingRowRef::History {
            object_type: unit.concrete_key("Register"),
            field: "writes".to_owned(),
            operation: unit.concrete_key("Register/write"),
        }
    );
    assert_eq!(undetermined.object, Some(register(&unit)));
    assert_eq!(undetermined.cause, EvalFailure::OutOfType);
}

/// FR-138-AC-5 (TC-554 step 1): with the `inv` field added, `write(r, 1)`
/// gives `inv` 1 and `write(r, 0)` returns `MappingFailure::Undefined`
/// naming `inv`, `r` and the cause `division-by-zero`.
#[trace("TC-554", "FR-138-AC-5")]
#[test]
fn an_undefined_update_is_undefined() {
    let unit = Unit::register();
    let world = unit.world(&register_history(&format!("{LAST}\n{INV}")));
    let system = world.concrete_system(UNLIMITED);
    let defined = along(&world, &system, &[1]);
    assert_eq!(
        integer(
            defined[0]
                .as_ref()
                .expect("defined")
                .get(&register(&unit), "inv")
        ),
        "1"
    );
    let undefined = along(&world, &system, &[0]);
    let Err(MappingFailure::Undefined { row, object, cause }) = &undefined[0] else {
        panic!("the update is undefined: {:?}", undefined[0]);
    };
    assert_eq!(
        *row,
        MappingRowRef::History {
            object_type: unit.concrete_key("Register"),
            field: "inv".to_owned(),
            operation: unit.concrete_key("Register/write"),
        }
    );
    assert_eq!(*object, Some(register(&unit)));
    assert_eq!(
        **cause,
        qsl_eval::temporal::UndefinedCause::Kernel(Undefined::DivisionByZero)
    );
}

/// FR-140-AC-4 (TC-545 step 4): over `RegisterHistory`, the concrete state
/// `value` 1 with `last` 0 maps to `value` 1, `prev` 0, whose key equals the
/// key of that state built directly in the abstract system. With the `prev`
/// row removed the same state maps to a visible state with `value` 1 and
/// `hidden` the slot `(r, prev)`.
#[trace("TC-545", "FR-140-AC-4")]
#[test]
fn a_history_field_feeds_the_mapping_and_an_unmapped_field_is_hidden() {
    let unit = Unit::register();
    let state = |world: &World<'_>| world.concrete_state(&[("value", int(1))]);
    let world = unit.world(&register_history(LAST));
    let (concrete, abstracted) = (
        world.concrete_system(UNLIMITED),
        world.abstract_system(UNLIMITED),
    );
    let history = initial_history(world.refinement(), &concrete, &state(&world));
    let mapped = map_state(
        world.refinement(),
        &concrete,
        &abstracted,
        &state(&world),
        &history,
    )
    .expect("the state maps");
    assert!(mapped.hidden.is_empty());
    let key = |world: &World<'_>, fields: &[(&str, Value)]| {
        abstracted
            .state_key_of(&world.abstract_state(fields))
            .expect("a key")
            .0
    };
    assert_eq!(
        abstracted.state_key_of(&mapped.visible).expect("a key").0,
        key(&world, &[("value", int(1)), ("prev", int(0))])
    );

    let without = "refinement RegisterHidden using inf abstract Spec concrete Impl {\n\
         population Spec::regs from Impl::regs;\n\
         object Spec::Register from Impl::Register { value = self.value; }\n\
         history Impl::Register.last : Int[0, 1] = 0 { on Impl::Register::write : pre(self.value); }\n\
         step Impl::Register::write -> Spec::Register::write(self, x);\n\
         }";
    let world = unit.world(without);
    let (concrete, abstracted) = (
        world.concrete_system(UNLIMITED),
        world.abstract_system(UNLIMITED),
    );
    let history = initial_history(world.refinement(), &concrete, &state(&world));
    let mapped = map_state(
        world.refinement(),
        &concrete,
        &abstracted,
        &state(&world),
        &history,
    )
    .expect("the state maps");
    assert_eq!(
        mapped.hidden,
        [HiddenSlot {
            object: ObjectKey {
                population: unit.abstract_key("regs"),
                key: "r".to_owned(),
            },
            field: unit.abstract_key("Register/prev"),
        }]
    );
    let visible = &mapped.visible.populations[&unit.abstract_key("regs")]["r"];
    assert_eq!(
        visible.fields.keys().cloned().collect::<Vec<_>>(),
        [unit.abstract_key("Register/value")]
    );
}

/// `check_step` on `write(r, x)` from `pre` at position `at`.
fn check_write(world: &World<'_>, pre: &RefinementPosition, x: i64, at: u64) -> StepVerdict {
    let (transition, reached) = write(world, &pre.concrete, x);
    check_step(
        world.refinement(),
        &world.concrete_system(UNLIMITED),
        &world.abstract_system(UNLIMITED),
        pre,
        &ConcreteStep {
            position: at,
            transition: Some(&transition),
            post: &reached,
        },
    )
    .verdict
}

/// The position of the initial state of `world`.
fn start(world: &World<'_>) -> RefinementPosition {
    let system = world.concrete_system(UNLIMITED);
    let StepVerdict::Passes { post, .. } = check_initial(
        world.refinement(),
        &system,
        &world.abstract_system(UNLIMITED),
        0,
        system.initial_states()[0].state(),
    ) else {
        panic!("the initial state passes");
    };
    post
}

/// FR-141-AC-6 (TC-554 step 2): over `RegisterHistory` with the `inv`
/// update, `check_step` on `write(r, 0)` from the initial state is
/// `Undefined` at position 1, naming the `inv` update, the object `r` and
/// the cause `division-by-zero`.
#[trace("TC-554", "FR-141-AC-6")]
#[test]
fn an_undefined_update_makes_the_step_undefined() {
    let unit = Unit::register();
    let world = unit.world(&register_history(&format!("{LAST}\n{INV}")));
    let StepVerdict::Undefined(evaluation) = check_write(&world, &start(&world), 0, 1) else {
        panic!("the update is undefined");
    };
    assert_eq!(evaluation.site.position, 1);
    assert_eq!(
        evaluation.site.locus,
        qsl_replay::UndefinedLocus::Mapping(Box::new(qsl_replay::MappingSite {
            row: MappingRowRef::History {
                object_type: unit.concrete_key("Register"),
                field: "inv".to_owned(),
                operation: unit.concrete_key("Register/write"),
            },
            object: Some(register(&unit)),
        }))
    );
    assert_eq!(
        evaluation.cause,
        qsl_eval::temporal::UndefinedCause::Kernel(Undefined::DivisionByZero)
    );
}

/// FR-141-AC-6 (TC-554 step 2): with the `writes` update instead, the first
/// `write(r, 0)` passes and the second returns
/// `Undetermined(MappingUndetermined)`.
#[trace("TC-554", "FR-141-AC-6")]
#[test]
fn an_update_that_leaves_its_type_makes_the_step_undetermined() {
    let unit = Unit::register();
    let world = unit.world(&register_history(&format!("{LAST}\n{WRITES}")));
    let StepVerdict::Passes { post, .. } = check_write(&world, &start(&world), 0, 1) else {
        panic!("the first write passes");
    };
    let StepVerdict::Undetermined(Undetermined::Mapping(undetermined)) =
        check_write(&world, &post, 0, 2)
    else {
        panic!("the second update is undetermined");
    };
    assert_eq!(undetermined.cause, EvalFailure::OutOfType);
}

/// TC-543 step 2: a history row on a type that does not exist refuses
/// `missing_declaration`/`missing-name`; an update whose value is no
/// `Int[0, 1]` refuses `ill_typed`/`type-mismatch` at the update.
#[trace("TC-543")]
#[test]
fn an_unknown_type_and_an_ill_typed_update_are_refused() {
    let unit = Unit::register();
    let bad = "history Impl::Nothing.f : Int[0, 1] = 0 { }";
    let found = unit.refused(&register_history(&format!("{LAST}\n{bad}")));
    assert_eq!(
        codes(&found),
        [(Code::MissingDeclaration, Some("missing-name"))]
    );
    let bad = "history Impl::Register.f : Int[0, 1] = 0 { on Impl::Register::write : true; }";
    let found = unit.refused(&register_history(&format!("{LAST}\n{bad}")));
    assert_eq!(codes(&found), [(Code::IllTyped, Some("type-mismatch"))]);
    assert_eq!(found[0].at, "true");
}

/// TC-543 step 2: `none` is the initial value of an `Option` field only, and
/// an `Option` field starts as an option value that holds nothing.
#[trace("TC-543")]
#[test]
fn none_initialises_an_option_field_and_nothing_else() {
    let unit = Unit::register();
    let option = "history Impl::Register.maybe : Option<Int[0, 1]> = none { }";
    let world = unit.world(&register_history(&format!("{LAST}\n{option}")));
    let rows = world.refinement().history();
    assert_eq!(rows.len(), 2);
    assert!(matches!(&rows[1].initial, Value::Option(held) if held.payload().is_none()));
    let bad = "history Impl::Register.plain : Int[0, 1] = none { }";
    let found = unit.refused(&register_history(&format!("{LAST}\n{bad}")));
    assert_eq!(codes(&found), [(Code::IllTyped, Some("type-mismatch"))]);
    assert_eq!(found[0].at, "none");
}

/// TC-551 step 2: a refinement with a history field writes no
/// `StepSimulationRecord`, since `map` is no function of the concrete step.
#[trace("TC-551")]
#[test]
fn a_history_field_writes_no_simulation_record() {
    let unit = Unit::register();
    let world = unit.world(&register_history(LAST));
    let graph = world.package().graph();
    let records = qsl_semantics::check::step_simulation_records(
        quire_exact::NodeKey::from_digest([7; 32]),
        world.refinement(),
        graph.state_clauses(),
    )
    .expect("the rows fit a step row reference");
    assert_eq!(records, []);
}

/// FR-138 Layer 5 (TC-554): a history update whose value leaves its declared
/// type still computed a value, and the rows read it; one with no value
/// settles the step as its own outcome. Here `writes` is 2 after the first
/// `write(r, 1)`, where the `prev` row is defined, so the step is the update's
/// `Undetermined`, not a refutation read off the value before the step.
#[trace("TC-554")]
#[test]
fn a_row_over_a_computed_out_of_type_value_does_not_refute() {
    let unit = Unit::register();
    let text = "refinement RegisterHistory using inf abstract Spec concrete Impl {\n\
         population Spec::regs from Impl::regs;\n\
         object Spec::Register from Impl::Register { prev = if 1 mod (1 - self.value + self.writes) = 0 then self.last else self.last; value = self.value; }\n\
         history Impl::Register.last : Int[0, 1] = 0 { on Impl::Register::write : pre(self.value); }\n\
         history Impl::Register.writes : Int[0, 1] = 0 { on Impl::Register::write : pre(self.writes) + 2; }\n\
         step Impl::Register::write -> Spec::Register::write(self, x);\n\
         }";
    let world = unit.world(text);
    let verdict = check_write(&world, &start(&world), 1, 1);
    let StepVerdict::Undetermined(Undetermined::Mapping(undetermined)) = verdict else {
        panic!("the update is undetermined: {verdict:?}");
    };
    assert_eq!(undetermined.cause, EvalFailure::OutOfType);
}

/// FR-138 Layer 5 (TC-554): an update with no value leaves no state to read
/// the post-state rows over, so the step is the update's `Undefined` even
/// when a row would be undefined too, and `check_step` reads the update for a
/// `stutter` or `any` row as for an abstract one.
#[trace("TC-554")]
#[test]
fn an_update_is_read_whatever_the_row_of_its_operation_says() {
    let unit = Unit::register();
    for target in ["any", "stutter"] {
        let text = register_history(&format!("{LAST}\n{INV}")).replace(
            "step Impl::Register::write -> Spec::Register::write(self, x);",
            &format!("step Impl::Register::write -> {target};"),
        );
        let world = unit.world(&text);
        let verdict = check_write(&world, &start(&world), 0, 1);
        assert!(
            matches!(verdict, StepVerdict::Undefined(_)),
            "{target}: {verdict:?}"
        );
    }
}

/// The concrete state holding the registers `values`, by key.
fn registers(world: &World<'_>, values: &[(&str, i64)]) -> ModelState {
    let fields: Vec<Vec<(&str, Value)>> = values
        .iter()
        .map(|(_, value)| vec![("value", int(*value))])
        .collect();
    let objects: Vec<(&str, &[(&str, Value)])> = values
        .iter()
        .zip(&fields)
        .map(|((key, _), fields)| (*key, fields.as_slice()))
        .collect();
    world.concrete_objects(&objects)
}

/// The concrete transition `operation` takes `pre` to `post`, with the state
/// it reaches.
fn transition(
    world: &World<'_>,
    unit: &Unit,
    pre: &ModelState,
    operation: &str,
    post: &ModelState,
) -> (ModelTransition, ModelSimState) {
    let system = world.concrete_system(UNLIMITED);
    let text = qsl_eval::simulation::model::operation_identity_text(
        &unit.concrete_key(&format!("Register/{operation}")),
    )
    .expect("a text");
    let wanted = system.state_key_of(post).expect("a key").0;
    system
        .expand_state(pre)
        .expect("the state expands")
        .successors
        .into_iter()
        .find(|(found, reached)| {
            found.operation() == text
                && system.state_key_of(reached.state()).expect("a key").0 == wanted
        })
        .unwrap_or_else(|| panic!("{operation} does not reach the state"))
}

const LIFE: &str = "refinement RegisterLife using inf abstract Spec concrete Impl {\n\
     population Spec::regs from Impl::regs;\n\
     object Spec::Register from Impl::Register { value = self.value; }\n\
     history Impl::Register.last : Int[0, 1] = 0 { on Impl::Register::write : pre(self.value); }\n\
     step Impl::Register::write -> any;\n\
     step Impl::Register::spawn -> any;\n\
     step Impl::Register::remove -> any;\n\
     }";

fn key_of(unit: &Unit, key: &str) -> ObjectKey {
    ObjectKey {
        population: unit.concrete_key("regs"),
        key: key.to_owned(),
    }
}

/// FR-138 Layer 5 (TC-543): a step writes the receiver's history field only
/// and keeps every other value. `write(r, 0)` over registers `r` = 1 and `s` =
/// 0 sets `r`'s `last` to 1 and leaves `s`'s at 0.
#[trace("TC-543")]
#[test]
fn an_update_writes_the_receiver_only() {
    let unit = Unit::registers(&["r", "s"], &["r", "s"], Shape::Lifecycle);
    let world = unit.world(LIFE);
    let system = world.concrete_system(UNLIMITED);
    let pre = registers(&world, &[("r", 1), ("s", 0)]);
    let history = initial_history(world.refinement(), &system, &pre);
    let (written, reached) = transition(
        &world,
        &unit,
        &pre,
        "write",
        &registers(&world, &[("r", 0), ("s", 0)]),
    );
    let after = step_history(
        world.refinement(),
        &system,
        &history,
        &pre,
        &written,
        &reached,
    )
    .expect("defined");
    assert_eq!(integer(after.get(&key_of(&unit, "r"), "last")), "1");
    assert_eq!(integer(after.get(&key_of(&unit, "s"), "last")), "0");
}

/// FR-138 Layer 5 (TC-543): an object a step creates takes the initial
/// literal and keeps nobody else's value; an object it deletes loses its
/// values.
#[trace("TC-543")]
#[test]
fn a_created_object_starts_at_the_initial_literal_and_a_deleted_one_loses_its_values() {
    let unit = Unit::registers(&["r", "s"], &["r"], Shape::Lifecycle);
    let world = unit.world(LIFE);
    let system = world.concrete_system(UNLIMITED);
    let pre = registers(&world, &[("r", 1)]);
    let history = initial_history(world.refinement(), &system, &pre);
    let (written, reached) = transition(
        &world,
        &unit,
        &pre,
        "write",
        &registers(&world, &[("r", 0)]),
    );
    let history = step_history(
        world.refinement(),
        &system,
        &history,
        &pre,
        &written,
        &reached,
    )
    .expect("defined");
    assert_eq!(integer(history.get(&key_of(&unit, "r"), "last")), "1");

    let pre = reached.state().clone();
    let both = registers(&world, &[("r", 0), ("s", 0)]);
    let (spawned, reached) = transition(&world, &unit, &pre, "spawn", &both);
    let history = step_history(
        world.refinement(),
        &system,
        &history,
        &pre,
        &spawned,
        &reached,
    )
    .expect("defined");
    assert_eq!(integer(history.get(&key_of(&unit, "r"), "last")), "1");
    assert_eq!(integer(history.get(&key_of(&unit, "s"), "last")), "0");

    let pre = reached.state().clone();
    let (removed, reached) = transition(
        &world,
        &unit,
        &pre,
        "remove",
        &registers(&world, &[("r", 0)]),
    );
    let history = step_history(
        world.refinement(),
        &system,
        &history,
        &pre,
        &removed,
        &reached,
    )
    .expect("defined");
    assert_eq!(integer(history.get(&key_of(&unit, "r"), "last")), "1");
    assert!(history.get(&key_of(&unit, "s"), "last").is_none());
}

/// FR-138 S3 (TC-543): a history field named like a field a subtype declares
/// is refused, since it would shadow that field in every row.
#[trace("TC-543")]
#[test]
fn a_history_field_named_like_a_subtype_field_is_refused() {
    let unit = Unit::registers(&["r"], &["r"], Shape::Subtype);
    let found = unit.refused(&register_history(LAST));
    assert!(
        codes(&found).contains(&(Code::InvalidModelBinding, Some("conflicting-binding"))),
        "{:?}",
        codes(&found)
    );
}

/// FR-138 S3 (TC-543): a history type whose values FR-181 gives no key form,
/// a decimal here, is refused `unsupported_construct`/`not-yet-implemented`
/// at the type, not admitted to fault in a product.
#[trace("TC-543")]
#[test]
fn a_history_type_with_no_key_form_is_unsupported() {
    let unit = Unit::register();
    let bad = "history Impl::Register.d : Decimal[0, 100; 0, 2; nearest-even] = decimal(0, 0) { }";
    let found = unit.refused(&register_history(&format!("{LAST}\n{bad}")));
    assert_eq!(
        codes(&found),
        [(Code::UnsupportedConstruct, Some("not-yet-implemented"))]
    );
    assert!(found[0].at.starts_with("Decimal"));
}
