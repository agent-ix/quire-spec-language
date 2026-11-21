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
        graph.model_inputs(),
        packages,
        ModelNormalizationLimits::default(),
        &key(CLOCKS, "Call"),
    )
    .expect("the universe derives")
    .expect("the type is declared");
    let receiver =
        ObjectReference::new(universe, effective, ObjectId::new(receiver).expect("an id"));
    let observed = StateModel::new(
        graph.model_inputs(),
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

/// The TC-3084 clock fixture with a stored reference and a row that returns
/// an actual evaluated place id for snapshot admission tests.
fn clock_reference_fixture(wrappers: bool) -> crate::ring_queue::Checked {
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
        if wrappers && domain == CLOCKS {
            let mut maybe = ring_queue::single(domain, owner, "maybe", &reference);
            maybe["presence"] = json!("optional");
            let mut refs = ring_queue::single(domain, owner, "refs", &reference);
            refs["multiplicity"] =
                json!({"lower": 0, "upper": 3, "ordered": true, "unique": false});
            let mut peer =
                ring_queue::single(domain, owner, "peer", &ring_queue::node(domain, owner));
            peer["presence"] = json!("optional");
            ty["fields"]
                .as_array_mut()
                .expect("fields")
                .extend([maybe, refs, peer]);
        }
        serde_json::to_vec(&document).expect("JSON")
    };
    let queue = with_reference(ring_queue::queue_spec(), QUEUE, "Queue", "chosen");
    let calls = with_reference(ring_queue::clock_calls(), CLOCKS, "Call", "r");
    ring_queue::check_models(
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
    })
}

/// A stored clock reference is ordinary data: the population count reads the
/// stored place id, and changing that field changes which place is selected.
#[trace("TC-3084", "FR-1040-AC-3", "FR-1040-AC-8")]
#[test]
fn a_refinement_count_reads_the_stored_clock_reference() {
    let checked = clock_reference_fixture(false);
    let package = CheckedPackage::link(checked.graph);
    let graph = package.graph();
    let (effective, universe) = declared_type_universe_for(
        graph.model_inputs(),
        &checked.packages,
        ModelNormalizationLimits::default(),
        &key(CLOCKS, "Call"),
    )
    .expect("universe")
    .expect("Call");
    let [refinement] = graph.refinements() else {
        panic!("one refinement")
    };
    let [queue, _] = refinement.objects() else {
        panic!("Queue and Cell")
    };
    let place_id = |owner: &str| {
        let owner = ObjectReference::new(
            universe.clone(),
            effective.clone(),
            ObjectId::new(owner).expect("id"),
        );
        let mut meter = Meter::new(UNLIMITED);
        // This row only reads its owner binder, so it needs no data snapshot.
        let evaluation = evaluate_refinement_row(
            &package,
            &queue.fields[2].expression,
            &fsl_semantics::model::object_environment::ObjectEnvironment::default(),
            vec![Value::Reference(owner)],
            &mut meter,
        )
        .expect("ref(self.x) evaluates");
        match evaluation.outcome {
            FamilyOutcome::Evaluated(Outcome::Completed(value)) => value,
            other => panic!("place id completes: {other:?}"),
        }
    };
    let c1 = place_id("c1");
    let c2 = place_id("c2");
    let mut state = state();
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

/// Run the actual `chosen = ref(self.x)` row with its owner binder alone.
fn generated_place(
    package: &CheckedPackage,
    packages: &fsl_semantics::model::intake::PackageInput,
    owner: &str,
) -> ObjectReference {
    let graph = package.graph();
    let (effective, universe) = declared_type_universe_for(
        graph.model_inputs(),
        packages,
        ModelNormalizationLimits::default(),
        &key(CLOCKS, "Call"),
    )
    .expect("universe")
    .expect("Call");
    let owner = ObjectReference::new(universe, effective, ObjectId::new(owner).expect("id"));
    let [refinement] = graph.refinements() else {
        panic!("one refinement")
    };
    let [queue, _] = refinement.objects() else {
        panic!("Queue and Cell")
    };
    let mut meter = Meter::new(UNLIMITED);
    let evaluation = evaluate_refinement_row(
        package,
        &queue.fields[2].expression,
        &fsl_semantics::model::object_environment::ObjectEnvironment::default(),
        vec![Value::Reference(owner)],
        &mut meter,
    )
    .expect("place row evaluates");
    match evaluation.outcome {
        FamilyOutcome::Evaluated(Outcome::Completed(Value::Reference(reference))) => reference,
        other => panic!("place row returns a reference: {other:?}"),
    }
}

/// The existing raw reference form names a place in a population's owner
/// context, including optional and sequence data fields.
fn clock_snapshot(reference: &ObjectReference) -> (DocumentRef, Vec<u8>) {
    use serde_json::json;
    let identity = DocumentRef {
        authority: "test".to_owned(),
        identity: "clock-snapshot".to_owned(),
    };
    let r = json!({"reference": {"population": crate::ring_queue::node(CLOCKS, "calls"), "key": reference.object().as_str()}});
    let bytes = serde_json::to_vec(&json!({
        "format": "filament.state.snapshot/v1", "identity": {"authority": identity.authority, "identity": identity.identity},
        "observation": "current", "model": {"identity": CLOCKS},
        "anchor": {"kind": "initialization", "name": "start"},
        "populations": [{"population": crate::ring_queue::node(CLOCKS, "calls"), "complete": true,
            "objects": [{"key": "c1", "type": crate::ring_queue::node(CLOCKS, "Call"), "fields": {
                "n": {"integer": "1"}, "d": {"integer": "0"}, "r": r,
                "maybe": {"present": r}, "refs": {"sequence": [r]}, "peer": {"absent": true},
            }}]}],
    })).expect("JSON");
    (identity, bytes)
}

/// Only a declared place instance is admitted as clock-reference data;
/// its owner is not resolved as an object of type Clock.
#[trace("TC-464", "FR-106-AC-72", "TC-3084", "FR-1040-AC-8")]
#[test]
fn clock_reference_snapshot_accepts_a_declared_place() {
    let checked = clock_reference_fixture(true);
    let package = CheckedPackage::link(checked.graph);
    let reference = generated_place(&package, &checked.packages, "c1");
    let (identity, bytes) = clock_snapshot(&reference);
    let graph = package.graph();
    let model = StateModel::new(
        graph.model_inputs(),
        graph.scope().types(),
        &checked.packages,
        ModelNormalizationLimits::default(),
    )
    .expect("model");
    let (state, _) = model
        .admit_initial(
            &BTreeMap::from([(identity.clone(), bytes)]),
            &identity,
            fsl_semantics::model::observation::ObservationLimits::default(),
        )
        .expect("declared clock place admits");
    let call = &state.populations[&key(CLOCKS, "calls")]["c1"];
    assert_eq!(
        call.fields[&key(CLOCKS, "Call/r")],
        Value::Reference(reference)
    );
    assert!(call.fields.contains_key(&key(CLOCKS, "Call/maybe")));
    assert!(call.fields.contains_key(&key(CLOCKS, "Call/refs")));
    model
        .observation(&state, identity)
        .expect("stored references remain data on readback");
}

/// A well-formed place id of another owner instance, and an ordinary
/// dangling object reference, both retain the dangling-reference boundary.
#[trace("TC-464", "FR-106-AC-73")]
#[test]
fn clock_reference_snapshot_refuses_an_undeclared_place() {
    use serde_json::{json, Value as Json};
    let checked = clock_reference_fixture(true);
    let package = CheckedPackage::link(checked.graph);
    let graph = package.graph();
    let model = StateModel::new(
        graph.model_inputs(),
        graph.scope().types(),
        &checked.packages,
        ModelNormalizationLimits::default(),
    )
    .expect("model");
    let valid = generated_place(&package, &checked.packages, "c1");
    let missing_owner = generated_place(&package, &checked.packages, "c2");
    let owner = model
        .object_reference(&key(CLOCKS, "Call"), "c1")
        .expect("owner");
    let undeclared_template = fsl_semantics::model::clock_reference::encode(
        &owner,
        graph.timed_models()[0].model_node(),
        &[],
    )
    .expect("well-formed identity of a non-place node");
    let (identity, bytes) = clock_snapshot(&valid);
    for missing in [missing_owner, undeclared_template] {
        for field in ["r", "maybe", "refs", "peer"] {
            let mut document: Json = serde_json::from_slice(&bytes).expect("JSON");
            let reference = json!({"reference": {"population": crate::ring_queue::node(CLOCKS, "calls"),
            "key": if field == "peer" { "ghost" } else { missing.object().as_str() }}});
            document["populations"][0]["objects"][0]["fields"][field] = match field {
                "maybe" => json!({"present": reference}),
                "refs" => json!({"sequence": [reference]}),
                _ => reference,
            };
            let failure = model
                .admit_initial(
                    &BTreeMap::from([(
                        identity.clone(),
                        serde_json::to_vec(&document).expect("JSON"),
                    )]),
                    &identity,
                    fsl_semantics::model::observation::ObservationLimits::default(),
                )
                .expect_err("undeclared place/object refuses");
            let fsl_semantics::model::observation::AdmissionFailure::Refused(record) = failure
            else {
                panic!("{field}: {failure:?}")
            };
            assert_eq!(
                record.catalog.code(),
                fsl_foundation::diagnostic::Code::DanglingReference.as_str(),
                "{field}: {record:?}"
            );
        }
    }
}
