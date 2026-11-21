// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-1040-AC-3 (TC-3084): the population forms over `ref(c.x)` in a
//! refinement row evaluate as FSpec FR-829-AC-2 states, over the timed
//! `Call` of `tests/support/ring_queue.rs` with `calls = {c1, c2}`.

use std::collections::BTreeMap;

use filament_eval::FamilyOutcome;
use filament_exact::{Integer, Meter, ObjectId, ObjectReference, Outcome, ScalarLimits, Value};
use fsl_eval::value::evaluate_refinement_row;
use fsl_package::CheckedPackage;
use fsl_semantics::model::accounting::ModelNormalizationLimits;
use fsl_semantics::model::observation::{declared_type_universe_for, DocumentRef};
use fsl_semantics::model::state::{ModelState, StateModel, StateObject};
use ix_trace_rs::trace;

use crate::ring_queue::{check_clock_refinement, clock_refinement, key, CLOCKS};

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

/// The `Queue` field rows, in field order: `used` then `busy`.
const USED: usize = 0;
const BUSY: usize = 1;

/// The state with `c1` (`n = 1`) and `c2` (`n = 2`). A clock has no data
/// slot, so a state that differs only in clock values is this same state.
fn state() -> ModelState {
    let call = |n: i64| StateObject {
        type_identity: key(CLOCKS, "Call"),
        fields: BTreeMap::from([
            (key(CLOCKS, "Call/n"), Value::Integer(Integer::from(n))),
            (key(CLOCKS, "Call/d"), Value::Integer(Integer::from(0_i64))),
        ]),
    };
    ModelState {
        populations: BTreeMap::from([(
            key(CLOCKS, "calls"),
            BTreeMap::from([("c1".to_owned(), call(1)), ("c2".to_owned(), call(2))]),
        )]),
        extents: Vec::new(),
    }
}

/// The row `index` of the `Queue` mapping evaluated with `self` the call
/// `receiver`, rendered.
fn evaluate(row: usize, receiver: &str) -> String {
    let checked = check_clock_refinement(&clock_refinement(
        "used = count(c in C::calls where ref(c.x) = ref(self.x));\n\
         busy = only(c in C::calls where c.n = 2: ref(c.x)) = ref(self.x);",
    ))
    .unwrap_or_else(|refusals| {
        panic!(
            "the rows check: {:?}",
            refusals
                .iter()
                .map(|found| &found.refusal)
                .collect::<Vec<_>>()
        )
    });
    let package = CheckedPackage::link(checked.graph);
    format!(
        "{:?}",
        evaluate_row(&package, &checked.packages, row, receiver, &state())
    )
}

/// Evaluate a row against supplied state, keeping the checked package fixed
/// while a reference field is updated.
fn evaluate_row(
    package: &CheckedPackage,
    packages: &fsl_semantics::model::intake::PackageInput,
    row: usize,
    receiver: &str,
    state: &ModelState,
) -> Value {
    let graph = package.graph();
    let (effective, universe) = declared_type_universe_for(
        graph.model_selections(),
        packages,
        ModelNormalizationLimits::default(),
        &key(CLOCKS, "Call"),
    )
    .expect("the universe derives")
    .expect("the type is declared");
    let receiver =
        ObjectReference::new(universe, effective, ObjectId::new(receiver).expect("an id"));
    let observed = StateModel::new(
        graph.model_selections(),
        graph.scope().types(),
        packages,
        ModelNormalizationLimits::default(),
    )
    .expect("model views")
    .observation(
        state,
        DocumentRef {
            authority: "test".to_owned(),
            identity: "state".to_owned(),
        },
    )
    .expect("the state observes")
    .environment;
    let [refinement] = graph.refinements() else {
        panic!("one refinement");
    };
    let [queue, _cell] = refinement.objects() else {
        panic!("the queue and the cell rows");
    };
    let mut meter = Meter::new(UNLIMITED);
    let evaluation = evaluate_refinement_row(
        package,
        &queue.fields[row].expression,
        &observed,
        vec![Value::Reference(receiver)],
        &mut meter,
    )
    .expect("the row evaluates");
    match evaluation.outcome {
        FamilyOutcome::Evaluated(Outcome::Completed(value)) => value,
        other => panic!("row {row} completes: {other:?}"),
    }
}

/// FR-1040-AC-3 (FSpec FR-829-AC-2): over `calls = {c1, c2}`, `only(c in
/// M::calls where c.n = 2: ref(c.x))` is the place id of `c2.x`, so it equals
/// `ref(self.x)` for `c2` and not for `c1`; `count(c in M::calls where
/// ref(c.x) = ref(self.x))` is `1` for either receiver, as each call holds
/// one place `x`.
#[trace("TC-3084", "FR-1040-AC-3")]
#[test]
fn the_refinement_row_place_forms_evaluate_to_the_place_ids() {
    let (true_, false_, one) = (
        format!("{:?}", Value::Boolean(true)),
        format!("{:?}", Value::Boolean(false)),
        format!("{:?}", Value::Integer(Integer::from(1_i64))),
    );
    assert_eq!(evaluate(BUSY, "c2"), true_);
    assert_eq!(evaluate(BUSY, "c1"), false_);
    assert_eq!(evaluate(USED, "c1"), one);
    assert_eq!(evaluate(USED, "c2"), one);
}

/// A stored clock reference is ordinary data: the population count reads the
/// stored place id, and changing that field changes which place is selected.
#[trace("TC-3084", "FR-1040-AC-3", "FR-1040-AC-8")]
#[test]
fn a_refinement_count_reads_the_stored_clock_reference() {
    use crate::ring_queue::{self, QUEUE};
    use serde_json::{json, Value as Json};

    let with_reference = |bytes: Vec<u8>, domain: &str, owner: &str, field: &str| {
        let mut document: Json = serde_json::from_slice(&bytes).expect("JSON");
        let reference = ring_queue::node(domain, "ClockRef");
        let mut definition = ring_queue::declaration(
            domain,
            "ClockRef",
            "unused",
            json!({"target": "ix://filament/native/Clock"}),
        );
        definition["kind"] = json!("reference");
        document["types"]
            .as_array_mut()
            .expect("types")
            .push(definition);
        let ty = document["types"]
            .as_array_mut()
            .expect("types")
            .iter_mut()
            .find(|ty| ty["identity"] == ring_queue::node(domain, owner))
            .expect("owner");
        ty["fields"]
            .as_array_mut()
            .expect("fields")
            .push(ring_queue::single(domain, owner, field, &reference));
        serde_json::to_vec(&document).expect("JSON")
    };
    let queue = with_reference(ring_queue::queue_spec(), QUEUE, "Queue", "chosen");
    let calls = with_reference(ring_queue::clock_calls(), CLOCKS, "Call", "r");
    let checked = ring_queue::check_models(
        &queue,
        ("C", CLOCKS, &calls),
        true,
        &clock_refinement(
            "used = count(c in C::calls where ref(c.x) = self.r);\n\
         busy = only(c in C::calls where c.n = 2: ref(c.x)) = self.r;\n\
         chosen = ref(self.x);",
        ),
    )
    .unwrap_or_else(|refusals| {
        panic!(
            "reference-field rows check: {:?}",
            refusals
                .iter()
                .map(|found| &found.refusal)
                .collect::<Vec<_>>()
        )
    });
    let package = CheckedPackage::link(checked.graph);
    let graph = package.graph();
    let (_, universe) = declared_type_universe_for(
        graph.model_selections(),
        &checked.packages,
        ModelNormalizationLimits::default(),
        &key(CLOCKS, "Call"),
    )
    .expect("universe")
    .expect("Call");
    let clock_type = filament_exact::EffectiveId::new(
        None,
        filament_exact::OriginalDeclarationKey::new("filament/native", "Clock"),
    );
    let placeholder = Value::Reference(ObjectReference::new(
        universe,
        clock_type,
        ObjectId::new("unselected").expect("id"),
    ));
    let mut state = state();
    for call in state
        .populations
        .get_mut(&key(CLOCKS, "calls"))
        .expect("calls")
        .values_mut()
    {
        call.fields
            .insert(key(CLOCKS, "Call/r"), placeholder.clone());
    }
    let chosen = 2;
    let c1 = evaluate_row(&package, &checked.packages, chosen, "c1", &state);
    let c2 = evaluate_row(&package, &checked.packages, chosen, "c2", &state);
    assert_ne!(c1, c2, "the owner distinguishes clock place ids");
    let set_reference = |state: &mut ModelState, reference: Value| {
        for call in state
            .populations
            .get_mut(&key(CLOCKS, "calls"))
            .expect("calls")
            .values_mut()
        {
            call.fields.insert(key(CLOCKS, "Call/r"), reference.clone());
        }
    };
    set_reference(&mut state, c1);
    assert_eq!(
        evaluate_row(&package, &checked.packages, USED, "c2", &state),
        Value::Integer(Integer::from(1_i64))
    );
    assert_eq!(
        evaluate_row(&package, &checked.packages, BUSY, "c2", &state),
        Value::Boolean(false)
    );
    set_reference(&mut state, c2);
    assert_eq!(
        evaluate_row(&package, &checked.packages, USED, "c1", &state),
        Value::Integer(Integer::from(1_i64))
    );
    assert_eq!(
        evaluate_row(&package, &checked.packages, BUSY, "c1", &state),
        Value::Boolean(true)
    );
}
