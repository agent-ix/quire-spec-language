// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-139 (TC-544): the population-valued forms of a refinement row,
//! evaluated through the one clause evaluator over a concrete state of
//! ADR-020 §8's ring buffer (`tests/support/ring_queue.rs`).

use std::collections::BTreeMap;

use ix_trace_rs::trace;
use qsl_eval::value::{evaluate_row, Evaluation};
use qsl_foundation::diagnostic::UndefinedReason;
use qsl_semantics::check::CheckedRefinement;
use qsl_semantics::family::{FamilyOutcome, FamilyResult};
use qsl_semantics::model::accounting::ModelNormalizationLimits;
use qsl_semantics::model::object_environment::{ObjectEnvironment, ObservedRead, ReadLog};
use qsl_semantics::model::observation::{declared_type_universe_for, DocumentRef};
use qsl_semantics::model::state::{ModelState, StateModel, StateObject};
use quire_exact::{
    ChargePoint, Integer, Meter, ObjectId, ObjectReference, Outcome, ScalarLimits, Value,
};

#[path = "../../../tests/support/ring_queue.rs"]
mod ring_queue;

use ring_queue::{all_rows, checked, key, ring_is_queue, Checked, RING};

const UNLIMITED: ScalarLimits = ScalarLimits {
    integer_bits: 1 << 20,
    decimal_digits: 1 << 20,
    scale_expansion: 1 << 20,
    text_input_bytes: 1 << 20,
    text_scalars: 1 << 20,
    normalized_scalars: 1 << 20,
    unit_edges: 1 << 20,
    value_occurrences: 1 << 20,
    work_units: 1 << 30,
    result_units: 1 << 30,
};

/// The index of each `Queue` field row, in `all_rows` order.
const ITEMS: usize = 0;
const USED: usize = 1;
const CELLS: usize = 3;
const LONE: usize = 4;
const GHOST: usize = 5;
const SOLID: usize = 6;

fn integer(value: i64) -> Value {
    Value::Integer(Integer::from(value))
}

struct World {
    checked: Checked,
}

impl World {
    fn new() -> Self {
        Self {
            checked: checked(&ring_is_queue(&all_rows())),
        }
    }

    fn refinement(&self) -> &CheckedRefinement {
        let [refinement] = self.checked.graph.refinements() else {
            panic!("one refinement");
        };
        refinement
    }

    fn reference(&self, object_type: &str, object: &str) -> ObjectReference {
        let (effective, universe) = declared_type_universe_for(
            self.checked.graph.model_selections(),
            &self.checked.packages,
            ModelNormalizationLimits::default(),
            &key(RING, object_type),
        )
        .expect("the universe derives")
        .expect("the type is declared");
        ObjectReference::new(universe, effective, ObjectId::new(object).expect("an id"))
    }

    /// The state with the ring `r` at `head` holding `size` items, and the
    /// slots `s0` and `s1` (indices 0 and 1) holding `values`.
    fn state(&self, head: i64, size: i64, values: [i64; 2]) -> ModelState {
        self.state_keyed(head, size, values, ["s0", "s1"])
    }

    /// [`Self::state`] with the slots' keys `keys`.
    fn state_keyed(&self, head: i64, size: i64, values: [i64; 2], keys: [&str; 2]) -> ModelState {
        let ring = |field: &str| key(RING, &format!("Ring/{field}"));
        let slot = |field: &str| key(RING, &format!("Slot/{field}"));
        let rings = BTreeMap::from([(
            "r".to_owned(),
            StateObject {
                type_identity: key(RING, "Ring"),
                fields: BTreeMap::from([
                    (ring("head"), integer(head)),
                    (ring("size"), integer(size)),
                ]),
            },
        )]);
        let slots = values
            .iter()
            .enumerate()
            .map(|(index, value)| {
                (
                    keys[index].to_owned(),
                    StateObject {
                        type_identity: key(RING, "Slot"),
                        fields: BTreeMap::from([
                            (slot("ring"), Value::Reference(self.reference("Ring", "r"))),
                            (
                                slot("index"),
                                integer(i64::try_from(index).expect("an index")),
                            ),
                            (slot("value"), integer(*value)),
                        ]),
                    },
                )
            })
            .collect();
        ModelState {
            populations: BTreeMap::from([(key(RING, "rings"), rings), (key(RING, "slots"), slots)]),
            extents: Vec::new(),
        }
    }

    fn observe(&self, state: &ModelState) -> ObjectEnvironment {
        let graph = &self.checked.graph;
        StateModel::new(
            graph.model_selections(),
            graph.scope().types(),
            &self.checked.packages,
            ModelNormalizationLimits::default(),
        )
        .expect("model views")
        .observation(
            state,
            DocumentRef {
                authority: "test".to_owned(),
                identity: "state".to_owned(),
                revision_namespace: "fixture".to_owned(),
                revision: "fixture:1".to_owned(),
                digest: [0; 32],
            },
        )
        .expect("the state observes")
        .environment
    }

    /// The `Queue` field row `index` evaluated at `state` for the ring `r`.
    fn evaluate(&self, state: &ModelState, index: usize, meter: &mut Meter) -> Evaluation {
        let [queue, _cell] = self.refinement().objects() else {
            panic!("the queue and the cell rows");
        };
        evaluate_row(
            &self.checked.graph,
            &queue.fields[index].expression,
            &self.observe(state),
            vec![Value::Reference(self.reference("Ring", "r"))],
            meter,
        )
        .expect("the row evaluates")
    }

    fn value(&self, state: &ModelState, index: usize) -> Value {
        let mut meter = Meter::new(UNLIMITED);
        match self.evaluate(state, index, &mut meter).outcome {
            FamilyOutcome::Evaluated(Outcome::Completed(value)) => value,
            other => panic!("row {index} completes: {other:?}"),
        }
    }
}

/// The elements of the collection `value`, rendered.
fn elements(value: &Value) -> Vec<String> {
    let Value::Collection(collection) = value else {
        panic!("a collection: {value:?}");
    };
    collection.elements().iter().map(render).collect()
}

/// `value`, rendered: `Value` has no equality of its own.
fn render(value: &Value) -> String {
    format!("{value:?}")
}

/// FR-139-AC-3 (TC-544 step 3): at the state `head = 1`, `size = 2`,
/// `s0.value = 0`, `s1.value = 1`, `items` evaluates to `[1, 0]`; at `head =
/// 0`, `size = 0` it evaluates to `[]`; `count(s in R::slots where s.ring =
/// self)` evaluates to 2 and `all(s in R::slots where s.value = 1)` to
/// `false`.
#[trace(
    "TC-544",
    "FR-139-AC-3",
    "QSpec-TC-311",
    "QSpec-FR-376-AC-1",
    "QSpec-FR-376-AC-2",
    "QSpec-FR-376-AC-3"
)]
#[test]
fn the_ring_rows_evaluate_over_the_slots_in_the_observed_state() {
    let world = World::new();
    let full = world.state(1, 2, [0, 1]);
    assert_eq!(
        elements(&world.value(&full, ITEMS)),
        [render(&integer(1)), render(&integer(0))]
    );
    assert!(elements(&world.value(&world.state(0, 0, [0, 1]), ITEMS)).is_empty());
    assert_eq!(render(&world.value(&full, USED)), render(&integer(2)));
    assert_eq!(
        render(&world.value(&full, SOLID)),
        render(&Value::Boolean(false))
    );
    assert_eq!(
        render(&world.value(&world.state(0, 0, [1, 1]), SOLID)),
        render(&Value::Boolean(true))
    );
}

/// FR-139-AC-3, QSpec FR-376-AC-5: renaming the slot keys leaves every
/// mapped value unchanged, since no form orders the slots by identity.
#[trace("TC-544", "FR-139-AC-3", "QSpec-TC-311", "QSpec-FR-376-AC-5")]
#[test]
fn renaming_the_slot_keys_leaves_every_mapped_value_unchanged() {
    let world = World::new();
    let renamed = world.state_keyed(1, 2, [0, 1], ["zeta", "alpha"]);
    // `alpha` now holds index 1 and value 1, `zeta` index 0 and value 0.
    let original = world.state(1, 2, [0, 1]);
    for row in [ITEMS, USED, SOLID] {
        assert_eq!(
            render(&world.value(&renamed, row)),
            render(&world.value(&original, row))
        );
    }
}

/// FR-139 behavior: a population name evaluates to the objects of that
/// population in the observed state, in ascending key order.
#[trace("TC-544", "FR-139-AC-3")]
#[test]
fn a_population_name_evaluates_to_its_objects_in_ascending_key_order() {
    let world = World::new();
    let value = world.value(&world.state(0, 0, [0, 1]), CELLS);
    assert_eq!(
        elements(&value),
        [
            render(&Value::Reference(world.reference("Slot", "s0"))),
            render(&Value::Reference(world.reference("Slot", "s1"))),
        ]
    );
}

/// FR-139 behavior: each form charges one work unit per object visited, and
/// the evaluation records the membership it read.
#[trace("TC-544", "FR-139-AC-3")]
#[test]
fn a_form_charges_one_work_unit_per_object_visited() {
    let world = World::new();
    let state = world.state(1, 2, [0, 1]);
    let mut meter = Meter::new(UNLIMITED);
    world.evaluate(&state, USED, &mut meter);
    let visits = meter
        .admitted_charges()
        .iter()
        .filter(|charge| charge.point == ChargePoint::CollectionVisit)
        .count();
    assert_eq!(visits, 2, "one per slot");

    let log = ReadLog::new();
    let environment = world.observe(&state).with_read_log(log.clone());
    let [queue, _cell] = world.refinement().objects() else {
        panic!("the queue and the cell rows");
    };
    evaluate_row(
        &world.checked.graph,
        &queue.fields[USED].expression,
        &environment,
        vec![Value::Reference(world.reference("Ring", "r"))],
        &mut Meter::new(UNLIMITED),
    )
    .expect("the row evaluates");
    assert!(
        log.reads().iter().any(|read| matches!(
            read,
            ObservedRead::Membership { population, object: None, .. }
                if population == &key(RING, "slots")
        )),
        "{:?}",
        log.reads()
    );
}

/// The undefined reason and payload of `row` at `state`.
fn undefined(world: &World, state: &ModelState, row: usize) -> (UndefinedReason, Vec<String>) {
    let mut meter = Meter::new(UNLIMITED);
    match world.evaluate(state, row, &mut meter).outcome {
        FamilyOutcome::FamilyEvaluated(FamilyResult::Undefined(cause)) => {
            let record = cause.undefined_record();
            (
                record.reason,
                record
                    .fields
                    .iter()
                    .map(|(name, value)| format!("{name}={value}"))
                    .collect(),
            )
        }
        other => panic!("row {row} is undefined: {other:?}"),
    }
}

/// FR-139-AC-4 (TC-544 step 4): `only(s in R::slots where s.ring = self:
/// s.value)` is undefined at every state of the fixture (two slots satisfy
/// it), and `only(s in R::slots where s.ring = self and s.index = self.head +
/// 2: s.value)` is undefined (no slot satisfies it).
#[trace("TC-544", "FR-139-AC-4", "QSpec-TC-311", "QSpec-FR-376-AC-3")]
#[test]
fn only_is_undefined_for_no_object_and_for_several() {
    let world = World::new();
    for state in [world.state(1, 2, [0, 1]), world.state(0, 0, [1, 1])] {
        assert_eq!(
            undefined(&world, &state, LONE),
            (
                UndefinedReason::OnlyNotOne,
                vec![
                    "found=2".to_owned(),
                    format!("population={}", key(RING, "slots").node)
                ]
            )
        );
        assert_eq!(
            undefined(&world, &state, GHOST).0,
            UndefinedReason::OnlyNotOne
        );
        assert_eq!(undefined(&world, &state, GHOST).1[0], "found=0");
    }
}
