// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1043 (FR-154-AC-6, AC-7, AC-10; FR-105-AC-7), TC-573 (FR-154-AC-4) and
//! TC-572 (FR-154-AC-5, with `decide_frame` over the same package): the `state`/`frame` node
//! body S4 emits for receiver-scoped field entries and scoped deletes, the
//! node ids and package ids those entries decide, and IR's I2 read of the
//! frame step over edited copies. The expected bodies are the vectors of
//! TC-1043's "Expected Results", written here from the test case.

use qsl_foundation::Code;
use quire_contract_model::{CheckedPackageRefusalCause, CheckedPackageRefusalCode};

use super::*;

use crate::checked_v2::V2ReadRefusal;

use super::abstraction::fixture as abstraction_fixture;
use super::child_composite::fixture as ledger_fixture;
use quire_semantic_value::checking::CheckMode;
#[path = "../../../../qsl-semantics/tests/support/receiver_frames.rs"]
pub(super) mod fixture;
#[path = "../../../../qsl-semantics/tests/support/tc235_k2.rs"]
#[allow(
    dead_code,
    reason = "this focused test uses the model-form package records"
)]
mod k2_fixture;

struct Emitted {
    wire: Value,
    evidence: CheckedPackageEvidence,
}

/// What [`unit_of`] builds: the unit text, the package document, the package
/// input keyed by identity.
struct UnitParts {
    unit: String,
    document: Vec<u8>,
    packages: qsl_semantics::model::intake::PackageInput,
}

/// The unit text selecting the package holding `operations` of `Counter`
/// (each named by its own postcondition clause) with the frames of
/// `overrides`, and the package document.
fn unit_of(operations: &[&str], overrides: &[(&str, Value)]) -> UnitParts {
    let document = crate::emit::tests::wire_bytes(&fixture::document(operations, overrides))
        .expect("the fixture serializes");
    let packages = qsl_semantics::model::intake::package_input([document.as_slice()]).unwrap();
    let unit = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n\
         model M = {package:?};\n\
         {posts}",
        package = fixture::PACKAGE,
        posts = fixture::posts(operations),
    );
    UnitParts {
        unit,
        document,
        packages,
    }
}

/// The package holding `operations` of `Counter` (each named by its own
/// postcondition clause), with the frames of `overrides`.
fn emit(operations: &[&str], overrides: &[(&str, Value)]) -> Emitted {
    let document = fixture::document(operations, overrides);
    emit_document(document, operations)
}

fn emit_document(document: Value, operations: &[&str]) -> Emitted {
    let document = crate::emit::tests::wire_bytes(&document).expect("the fixture serializes");
    let packages = qsl_semantics::model::intake::package_input([document.as_slice()]).unwrap();
    let unit = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n\
         model M = {package:?};\n\
         {posts}",
        package = fixture::PACKAGE,
        posts = fixture::posts(operations),
    );
    let graph = assemble_with_models(unit.as_bytes(), &packages)
        .expect("the unit assembles")
        .check(CheckingLimits::default(), CheckMode::Linked)
        .unwrap_or_else(|refusals| panic!("the package checks: {refusals:?}"));
    let emission = emit_checked(&CheckedPackage::link(graph)).expect("the package emits");
    assert_eq!(emission.omitted, []);
    let wire = wire(&emission);
    let mut evidence = CheckedPackageEvidence::new();
    for feature in wire["lock"]["required_features"].as_array().unwrap() {
        evidence.support_feature(feature.as_str().unwrap());
    }
    evidence
        .insert_domain_package_document(package_identity(&document), document.as_slice())
        .expect("a fresh evidence key");
    Emitted { wire, evidence }
}

/// The package holding `operations` of `Counter` with the frames of
/// `overrides`, under a unit whose only clause is a function that names no
/// model declaration.
fn emit_without_operation_posts(operations: &[&str], overrides: &[(&str, Value)]) -> Emitted {
    let document = crate::emit::tests::wire_bytes(&fixture::document(operations, overrides))
        .expect("the fixture serializes");
    let packages = qsl_semantics::model::intake::package_input([document.as_slice()]).unwrap();
    let unit = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n\
         model M = {package:?};\n\
         function marker using v(): Boolean pure {{ true }}",
        package = fixture::PACKAGE,
    );
    let graph = assemble_with_models(unit.as_bytes(), &packages)
        .expect("the unit assembles")
        .check(CheckingLimits::default(), CheckMode::Linked)
        .unwrap_or_else(|refusals| panic!("the package checks: {refusals:?}"));
    let emission = emit_checked(&CheckedPackage::link(graph)).expect("the package emits");
    assert_eq!(emission.omitted, []);
    let wire = wire(&emission);
    let mut evidence = CheckedPackageEvidence::new();
    for feature in wire["lock"]["required_features"].as_array().unwrap() {
        evidence.support_feature(feature.as_str().unwrap());
    }
    evidence
        .insert_domain_package_document(package_identity(&document), document.as_slice())
        .expect("the selected model document is evidence");
    Emitted { wire, evidence }
}

impl Emitted {
    /// QSL's I2 read of `envelope` as the package it names.
    fn read_envelope(&self, envelope: &Value) -> Read {
        read_v2(
            &jcs(envelope),
            library(),
            V2ReadLimits::fixture(),
            &self.evidence,
        )
    }

    /// The I2 read of the package as emitted.
    fn read(&self) -> Read {
        self.read_envelope(&self.wire)
    }

    /// The `Counter` object type's node id, `<Counter>` in the vectors.
    fn counter(&self) -> Value {
        let found: Vec<&Value> = super::owners::node_where(&self.wire, "model", "object_type")
            .into_iter()
            .filter(|node| node["owner"]["node"] == fixture::id("Counter"))
            .collect();
        assert_eq!(found.len(), 1, "Counter is one object_type node");
        found[0]["node_id"].clone()
    }

    /// The frame node id the anchor of `operation` references.
    fn frame_id(&self, operation: &str) -> Value {
        let anchors: Vec<&Value> =
            super::owners::node_where(&self.wire, "state", "operation_anchor")
                .into_iter()
                .filter(|anchor| anchor["body"]["members"][1]["value"]["value"] == operation)
                .collect();
        let [anchor] = anchors[..] else {
            panic!("{operation} has one operation_anchor node: {anchors:#?}");
        };
        let target = &anchor["body"]["members"][2]["value"]["target"];
        assert_eq!(anchor["body"]["members"][2]["name"], "frame");
        target.clone()
    }

    /// The `state`/`frame` node `operation`'s anchor references.
    fn frame(&self, operation: &str) -> &Value {
        let id = self.frame_id(operation);
        super::owners::node_where(&self.wire, "state", "frame")
            .into_iter()
            .find(|node| node["node_id"] == id)
            .unwrap_or_else(|| panic!("{operation}'s frame node is in the package"))
    }
}

/// The NodeRef object `<Counter>` stands for.
fn node_ref(id: &Value) -> Value {
    id.clone()
}

/// The frame body vectors W1 to W6 over `counter` (`<Counter>`).
fn vectors(counter: &Value) -> [Value; 6] {
    let field = |name: &str, scope: &str| json!({"declaration": node_ref(counter), "kind": "field", "name": name, "scope": scope});
    let scoped = |entries: Vec<Value>| Value::Array(entries);
    [
        // W1 (bump)
        json!({"creates": [], "deletes": [], "deletes_scoped": scoped(vec![json!({"kind": "receiver"})]),
               "modifies": [field("step", "receiver"), field("value", "every-object")], "term": "frame"}),
        // W2 (merge)
        json!({"creates": [], "deletes": [node_ref(counter)],
               "deletes_scoped": scoped(vec![
                   json!({"kind": "receiver"}),
                   json!({"kind": "parameter", "name": "a"}),
                   json!({"kind": "parameter", "name": "b"}),
               ]), "modifies": [], "term": "frame"}),
        // W3 (noop)
        json!({"creates": [], "deletes": [], "deletes_scoped": [], "modifies": [], "term": "frame"}),
        // W4 (own)
        json!({"creates": [], "deletes": [], "deletes_scoped": [],
               "modifies": [field("value", "receiver")], "term": "frame"}),
        // W5 (all)
        json!({"creates": [], "deletes": [], "deletes_scoped": [],
               "modifies": [field("value", "every-object")], "term": "frame"}),
        // W6 (drop)
        json!({"creates": [], "deletes": [], "deletes_scoped": [json!({"kind": "receiver"})],
               "modifies": [], "term": "frame"}),
    ]
}

const ALL: [&str; 7] = ["bump", "merge", "noop", "own", "all", "drop", "dispose"];

/// FR-154-AC-6 (TC-1043 step 1): the frame bodies of `bump`, `merge` and
/// `noop` are vectors W1, W2 and W3, the frames of `bump` and `merge` depend
/// on `<Counter>` and the scoped entries add no other dependency, and the
/// package reads back `Verified` (the body of W1 is admitted).
#[trace("TC-1043", "FR-154-AC-6", "FR-154-AC-10")]
#[test]
fn the_frame_bodies_of_scoped_entries_are_the_wire_vectors() {
    let emitted = emit(&ALL, &[]);
    let counter = emitted.counter();
    let [w1, w2, w3, ..] = vectors(&counter);
    assert_eq!(emitted.frame("bump")["body"], w1);
    assert_eq!(emitted.frame("merge")["body"], w2);
    assert_eq!(emitted.frame("noop")["body"], w3);
    for operation in ["bump", "merge"] {
        assert_eq!(
            emitted.frame(operation)["dependencies"],
            json!([counter]),
            "{operation}"
        );
    }
    let read = emitted.read();
    assert!(matches!(read, Read::Verified { .. }), "{read:?}");
}

/// FR-496-AC-1..3, FR-496-AC-5, TC-1286: operation frames flow to the optional checked
/// package carrier with declared identities, complete empty rows, scoped
/// entries, and canonical ordering. The IR v2 reader admits the same rows.
#[trace("FR-496-AC-1", "FR-496-AC-2", "FR-496-AC-3", "FR-496-AC-5", "TC-1286")]
#[test]
fn selected_operation_effects_are_complete_canonical_and_readable() {
    let created = json!({
        "modifies": [fixture::id("Counter/value"), fixture::id("Counter/step")],
        "creates": [fixture::id("Gauge")],
        "deletes": [],
    });
    let operations = [
        "bump", "merge", "noop", "own", "all", "same", "drop", "dispose",
    ];
    let emitted = emit(&operations, &[("same", created)]);
    let rows = emitted.wire["operation_effects"]
        .as_array()
        .expect("the producer emits the carrier");
    assert_eq!(
        rows.len(),
        operations.len(),
        "one row per declared operation"
    );
    let admitted = match emitted.read() {
        Read::Verified { admitted, .. } => admitted,
        other => panic!("IR admits the emitted effect rows: {other:?}"),
    };
    let admitted_rows = admitted
        .operation_effects()
        .expect("IR retains the admitted operation effects")
        .collect::<Vec<_>>();
    assert_eq!(admitted_rows.len(), operations.len());
    assert_eq!(
        admitted_rows[0].operation(),
        rows[0]["operation"].as_str().unwrap()
    );

    let document = package_identity(
        &crate::emit::tests::wire_bytes(&fixture::document(
            &operations,
            &[(
                "same",
                json!({
                    "modifies": [fixture::id("Counter/value"), fixture::id("Counter/step")],
                    "creates": [fixture::id("Gauge")],
                    "deletes": [],
                }),
            )],
        ))
        .unwrap(),
    );
    assert!(rows.iter().all(|row| row["document"] == document));
    assert!(rows.windows(2).all(|pair| {
        pair[0]["declaring"] == pair[1]["declaring"]
            && pair[0]["operation"].as_str() < pair[1]["operation"].as_str()
    }));
    for row in rows {
        let name = row["operation"].as_str().unwrap();
        assert_eq!(
            row["provenance"],
            json!({
                "origin": "model_document_operation_frame",
                "member": fixture::id(&format!("Counter/{name}")),
            })
        );
        assert!(row["effect"].get("modifies").is_some());
        assert!(row["effect"].get("creates").is_some());
        assert!(row["effect"].get("deletes").is_some());
        assert!(row["effect"].get("deletes_scoped").is_some());
    }
    let noop = rows.iter().find(|row| row["operation"] == "noop").unwrap();
    assert_eq!(
        noop["effect"],
        json!({"modifies": [], "creates": [], "deletes": [], "deletes_scoped": []})
    );
    let merge = rows.iter().find(|row| row["operation"] == "merge").unwrap();
    assert_eq!(
        merge["effect"]["deletes_scoped"],
        json!([
            {"kind": "receiver"}, {"kind": "parameter", "name": "a"},
            {"kind": "parameter", "name": "b"}
        ])
    );
    let same = rows.iter().find(|row| row["operation"] == "same").unwrap();
    assert_eq!(same["effect"]["creates"].as_array().unwrap().len(), 1);
    assert_eq!(
        same["effect"]["modifies"]
            .as_array()
            .unwrap()
            .iter()
            .map(|entry| entry["name"].as_str().unwrap())
            .collect::<Vec<_>>(),
        ["step", "value"]
    );
    let mut without_carrier = emitted.wire.clone();
    without_carrier
        .as_object_mut()
        .unwrap()
        .remove("operation_effects");
    assert_eq!(
        without_carrier["semantic_graph"], emitted.wire["semantic_graph"],
        "the optional carrier does not alter graph nodes or identities"
    );

    let reversed = operations.into_iter().rev().collect::<Vec<_>>();
    let reordered = emit(
        &reversed,
        &[(
            "same",
            json!({
                "modifies": [fixture::id("Counter/step"), fixture::id("Counter/value")],
                "creates": [fixture::id("Gauge")],
                "deletes": [],
            }),
        )],
    );
    assert_eq!(
        reordered.wire["operation_effects"], emitted.wire["operation_effects"],
        "model member declaration order does not change the carrier"
    );
}

/// FR-496-AC-1, AC-2, AC-5: package-level rows keep their declaration
/// witnesses in the semantic graph even when no unit clause names the
/// operation or its owner.
#[trace("FR-496-AC-1", "FR-496-AC-2", "FR-496-AC-5", "TC-1286")]
#[test]
fn selected_operation_effects_do_not_require_unit_operation_clauses() {
    let emitted = emit_without_operation_posts(&["noop"], &[]);
    assert_eq!(
        emitted.wire["operation_effects"].as_array().unwrap().len(),
        1
    );
    let read = emitted.read();
    assert!(matches!(read, Read::Verified { .. }), "{read:?}");
}

/// Emits `Counter.noop` with `frame` under a unit with no clause, so only the
/// frame's subject names `Gauge`; asserts that FR-094 roots `Gauge`'s node
/// for the row and that IR's v2 reader admits the package.
fn frame_subject_is_rooted(frame: Value) {
    let emitted = emit_without_operation_posts(&["noop"], &[("noop", frame)]);
    let gauge = super::owners::node_where(&emitted.wire, "model", "object_type")
        .into_iter()
        .filter(|node| node["owner"]["node"] == fixture::id("Gauge"))
        .count();
    assert_eq!(gauge, 1, "Gauge's object_type node is emitted");
    let read = emitted.read();
    assert!(matches!(read, Read::Verified { .. }), "{read:?}");
}

/// FR-496-AC-1, FR-094, TC-1286: a row whose frame modifies the field
/// `Gauge/d`, which no unit clause names, roots the field's declaring node
/// `Gauge`.
#[trace("FR-496-AC-1", "TC-1286")]
#[test]
fn a_rows_written_field_roots_its_declaring_node() {
    frame_subject_is_rooted(json!({
        "modifies": [fixture::id("Gauge/d")], "creates": [], "deletes": [],
    }));
}

/// FR-496-AC-1, FR-094, TC-1286: a row whose frame creates `Gauge`, which no
/// unit clause names, roots `Gauge`.
#[trace("FR-496-AC-1", "TC-1286")]
#[test]
fn a_rows_created_type_is_rooted() {
    frame_subject_is_rooted(json!({
        "modifies": [], "creates": [fixture::id("Gauge")], "deletes": [],
    }));
}

/// FR-496-AC-1, FR-094, TC-1286: a row whose frame deletes `Gauge`
/// type-wide, which no unit clause names, roots `Gauge`.
#[trace("FR-496-AC-1", "TC-1286")]
#[test]
fn a_rows_deleted_type_is_rooted() {
    frame_subject_is_rooted(json!({
        "modifies": [], "creates": [], "deletes": [fixture::id("Gauge")],
    }));
}

/// FR-496-AC-1..5, TC-1286: the Ledger `pay` and `payTaxed` rows survive the
/// emitted-package reader and the selected type projection preserves each
/// operation's own effect, including the all-empty frame.
#[trace("FR-496-AC-1", "FR-496-AC-2", "FR-496-AC-5", "TC-1286")]
#[test]
fn ledger_operation_effects_read_back_with_selected_type_projection() {
    let mut document = ledger_fixture::document();
    document["types"].as_array_mut().unwrap().retain(|ty| {
        ["Money", "TaxedMoney", "Ledger"]
            .iter()
            .any(|name| ty["identity"] == ledger_fixture::id(name))
    });
    document["constructs"]
        .as_array_mut()
        .unwrap()
        .retain(|construct| matches!(construct["kind"]["name"].as_str(), Some("money" | "thing")));
    document["populations"] = json!([]);
    let ledger = document["types"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|ty| ty["identity"] == ledger_fixture::id("Ledger"))
        .expect("the selected document declares Ledger");
    let pay = ledger["operations"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|operation| operation["name"] == "pay")
        .expect("Ledger declares pay");
    pay["frame"]["modifies"] = json!([ledger_fixture::id("Ledger/ledger_id")]);
    let document = crate::emit::tests::wire_bytes(&document).expect("the fixture serializes");
    let packages = qsl_semantics::model::intake::package_input([document.as_slice()]).unwrap();
    let unit = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n\
         model M = {package:?};\n\
         function marker using v(): Boolean pure {{ true }}",
        package = ledger_fixture::PACKAGE,
    );
    let graph = assemble_with_models(unit.as_bytes(), &packages)
        .expect("the unit assembles")
        .check(CheckingLimits::default(), CheckMode::Linked)
        .unwrap_or_else(|refusals| panic!("the package checks: {refusals:?}"));
    let checked = CheckedPackage::link(graph);
    let emission = emit_checked(&checked).expect("the package emits");
    assert_eq!(emission.omitted(), []);
    let mut evidence = super::read_evidence(&emission);
    evidence
        .insert_domain_package_document(package_identity(&document), document.as_slice())
        .expect("the selected model document is evidence");
    let admitted = match read_v2(
        emission.package().bytes(),
        library(),
        V2ReadLimits::fixture(),
        &evidence,
    ) {
        Read::Verified { admitted, .. } => admitted,
        other => panic!("IR admits the emitted Ledger operation effects: {other:?}"),
    };
    let rows = admitted
        .operation_effects()
        .expect("the reader retains the operation-effect carrier")
        .collect::<Vec<_>>();
    assert_eq!(
        rows.iter().map(|row| row.operation()).collect::<Vec<_>>(),
        ["pay", "payTaxed"]
    );
    assert_eq!(rows[0].effect().modifies.len(), 1);
    assert!(rows[1].effect().modifies.is_empty());
    assert!(rows[1].effect().creates.is_empty());
    assert!(rows[1].effect().deletes.is_empty());
    assert!(rows[1].effect().deletes_scoped.is_empty());

    let identities = checked
        .graph()
        .selected_exact_identities(
            &admitted,
            &mut quire_contract_model::NativeJoinBudget::new(10_000_000, 10_000_000),
        )
        .expect("selected declarations join the admitted model");
    let projected = admitted
        .project_selected_type_declarations(
            &identities,
            emission.selected_tuple_declaration_rows(),
            emission.selected_source_unit_rows(),
            &mut quire_contract_model::NativeJoinBudget::new(10_000_000, 10_000_000),
        )
        .expect("the selected type declaration projection succeeds");
    let ledger = projected
        .objects()
        .iter()
        .find(|object| object.original_identity() == ledger_fixture::id("Ledger"))
        .expect("Ledger is projected");
    assert_eq!(
        ledger
            .operations()
            .iter()
            .map(|operation| operation.name())
            .collect::<Vec<_>>(),
        ["pay", "payTaxed"]
    );
    assert_eq!(ledger.operations()[0].effect().modifies.len(), 1);
    assert!(ledger.operations()[1].effect().modifies.is_empty());
}

/// FR-496-AC-1: relationship frames retain a relationship declaration as a
/// relationship modification in the carrier and pass the strict IR join.
#[trace("FR-496-AC-1", "TC-1286")]
#[test]
fn relationship_effects_reference_the_declared_relationship_node() {
    let mut document = fixture::document(
        &["all"],
        &[(
            "all",
            json!({
                "modifies": [fixture::id("relationship/owns")],
                "creates": [],
                "deletes": [],
            }),
        )],
    );
    document["types"][0]["relationships"] = json!([{
        "identity": fixture::id("relationship/owns"),
        "category": "structural",
        "composite": false,
        "direction": "source-to-target",
        "origin": {"source": {
            "sourceIdentity": fixture::id("spec"),
            "path": "model.qspec",
            "startLine": 1,
            "startColumn": 1,
        }},
        "sourceEnd": {
            "type": fixture::id("Counter"),
            "role": "owner",
            "multiplicity": {"lower": 0, "upper": 1, "ordered": false, "unique": true},
        },
        "targetEnd": {
            "type": fixture::id("Gauge"),
            "role": "owned",
            "multiplicity": {"lower": 0, "upper": 1, "ordered": false, "unique": true},
        },
    }]);
    let emitted = emit_document(document, &["all"]);
    let row = &emitted.wire["operation_effects"][0];
    assert_eq!(row["operation"], "all");
    let declaration = row["effect"]["modifies"][0]["declaration"].clone();
    assert_eq!(
        row["effect"]["modifies"],
        json!([{"kind": "relationship", "declaration": declaration}])
    );
    assert!(nodes(&emitted.wire)
        .iter()
        .any(|node| node["node_id"] == declaration));
    assert!(matches!(emitted.read(), Read::Verified { .. }));
}

/// FR-496-AC-2: a member with the same declared name under two owners and
/// in a second selected document remains three separately keyed rows.
#[trace("FR-496-AC-2", "TC-1286")]
#[test]
fn same_operation_name_keeps_its_document_and_declaring_identity() {
    fn relabel(value: &mut Value, from: &str, to: &str) {
        match value {
            Value::String(text) => {
                if let Some(suffix) = text.strip_prefix(&format!("ix://{from}/")) {
                    *text = format!("ix://{to}/{suffix}");
                }
            }
            Value::Array(values) => values.iter_mut().for_each(|value| relabel(value, from, to)),
            Value::Object(values) => values
                .values_mut()
                .for_each(|value| relabel(value, from, to)),
            _ => {}
        }
    }

    let mut first = fixture::document(&["noop"], &[]);
    first["types"][1]["operations"] = json!([{
        "identity": fixture::id("Gauge/noop"),
        "name": "noop",
        "params": [],
        "pre": [],
        "post": [],
        "frame": {"modifies": [], "creates": [], "deletes": []},
        "origin": {"generated": {
            "generatorIdentity": fixture::id("Gauge/noop"),
            "generatorVersion": "1.0.0",
            "inputIdentities": [fixture::id("Gauge/noop")],
        }},
    }]);
    let mut second = fixture::document(&["noop"], &[]);
    relabel(&mut second, fixture::PACKAGE, "test/orders-secondary");
    second["package"]["identity"] = json!("test/orders-secondary");
    let first_bytes = crate::emit::tests::wire_bytes(&first).unwrap();
    let second_bytes = crate::emit::tests::wire_bytes(&second).unwrap();
    let packages = qsl_semantics::model::intake::package_input([
        first_bytes.as_slice(),
        second_bytes.as_slice(),
    ])
    .unwrap();
    let unit = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n\
         model A = {first:?};\n\
         model B = \"test/orders-secondary\";\n\
         post CounterDone using v on A::Counter::noop {{ true }}\n\
         post GaugeDone using v on A::Gauge::noop {{ true }}\n\
         post SecondaryDone using v on B::Counter::noop {{ true }}\n",
        first = fixture::PACKAGE,
    );
    let graph = assemble_with_models(unit.as_bytes(), &packages)
        .expect("both selected packages assemble")
        .check(CheckingLimits::default(), CheckMode::Linked)
        .unwrap_or_else(|refusals| panic!("the package checks: {refusals:?}"));
    let emission = emit_checked(&CheckedPackage::link(graph)).expect("the package emits");
    let wire = wire(&emission);
    let rows = wire["operation_effects"].as_array().unwrap();
    assert_eq!(rows.len(), 3);
    assert!(rows.iter().all(|row| row["operation"] == "noop"));
    assert_eq!(
        rows.iter()
            .map(|row| row["document"].as_str().unwrap())
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        2
    );
    assert_eq!(
        rows.iter()
            .filter(|row| row["document"] == fixture::PACKAGE)
            .map(|row| row["declaring"].to_string())
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        2
    );

    let mut evidence = CheckedPackageEvidence::new();
    for feature in wire["lock"]["required_features"].as_array().unwrap() {
        evidence.support_feature(feature.as_str().unwrap());
    }
    evidence
        .insert_domain_package_document(package_identity(&first_bytes), first_bytes.as_slice())
        .unwrap();
    evidence
        .insert_domain_package_document(package_identity(&second_bytes), second_bytes.as_slice())
        .unwrap();
    let read = read_v2(
        emission.package.bytes(),
        library(),
        V2ReadLimits::fixture(),
        &evidence,
    );
    assert!(matches!(read, Read::Verified { .. }), "{read:?}");
}

/// FR-496-AC-4: directly declared process and state-machine operations get
/// rows, while persistence-interface operations are not frame declarations.
#[trace("FR-496-AC-4", "TC-1286")]
#[test]
fn process_and_state_machine_operations_are_carried_without_persistence_rows() {
    let mut document = k2_fixture::k2();
    k2_fixture::type_named_mut(&mut document, "Fulfilment")["operations"] =
        json!([k2_fixture::operation("Fulfilment", "start")]);
    let document = crate::emit::tests::wire_bytes(&document).expect("the fixture serializes");
    let packages = qsl_semantics::model::intake::package_input([document.as_slice()]).unwrap();
    let unit = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n\
         model M = {package:?};\n\
         function marker using v(): Boolean pure {{ true }}\n",
        package = k2_fixture::PACKAGE,
    );
    let graph = assemble_with_models(unit.as_bytes(), &packages)
        .expect("the unit assembles")
        .check(CheckingLimits::default(), CheckMode::Linked)
        .unwrap_or_else(|refusals| panic!("the package checks: {refusals:?}"));
    let emission = emit_checked(&CheckedPackage::link(graph)).expect("the package emits");
    let wire = wire(&emission);
    let rows = wire["operation_effects"].as_array().unwrap();
    let operations = rows
        .iter()
        .map(|row| row["operation"].as_str().unwrap())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(operations, ["cancel", "place", "start"].into());
}

/// FR-496-AC-4: an operation inherited by a subtype keeps the declaring
/// base type's row and does not create a second row for the subtype.
#[trace("FR-496-AC-4", "TC-1286")]
#[test]
fn inherited_operations_have_no_subtype_effect_row() {
    let mut document = fixture::document(&["noop"], &[]);
    document["types"][1]["supertypes"] = json!([fixture::id("Counter")]);
    document["populations"][0]["members"] = json!([fixture::id("Counter")]);
    let emitted = emit_document(document, &["noop"]);
    let rows = emitted.wire["operation_effects"].as_array().unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0]["operation"], "noop");
    let counter = super::owners::node_where(&emitted.wire, "model", "object_type")
        .into_iter()
        .find(|node| node["owner"]["node"] == fixture::id("Counter"))
        .expect("the base object type is emitted");
    assert_eq!(rows[0]["declaring"], counter["node_id"]);
    assert!(matches!(emitted.read(), Read::Verified { .. }));
}

/// FR-496-AC-4: an inherited member and a directly declared systems-interface
/// operation do not create rows; ordinary object declarations still do.
#[trace("FR-496-AC-4", "TC-1286")]
#[test]
fn systems_interface_and_inherited_operations_are_not_effect_declarations() {
    let document = abstraction_fixture::document();
    let document = crate::emit::tests::wire_bytes(&document).unwrap();
    let packages = qsl_semantics::model::intake::package_input([document.as_slice()]).unwrap();
    let unit = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n\
         model M = {package:?};\n\
         post BaseDone using v on M::Base::op {{ true }}\n\
         post VersionDone using v on M::ConfigVersion::attemptUpdate {{ true }}\n",
        package = abstraction_fixture::PACKAGE,
    );
    let graph = assemble_with_models(unit.as_bytes(), &packages)
        .expect("the unit assembles")
        .check(CheckingLimits::default(), CheckMode::Linked)
        .unwrap_or_else(|refusals| panic!("the package checks: {refusals:?}"));
    let emission = emit_checked(&CheckedPackage::link(graph)).expect("the package emits");
    let wire = wire(&emission);
    let rows = wire["operation_effects"].as_array().unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(
        rows.iter()
            .map(|row| row["operation"].as_str().unwrap())
            .collect::<std::collections::BTreeSet<_>>(),
        ["attemptUpdate", "op"].into()
    );
}

/// FR-154-AC-7, FR-105 (TC-1043 steps 1 and 2): `own`, `all` and `drop` are
/// vectors W4, W5 and W6; frames differing only by one entry's scope, or only
/// by `deletes self`, have different node ids; a frame of type-wide entries
/// carries `deletes_scoped: []` and every field entry `every-object`.
#[trace("TC-1043", "FR-154-AC-7")]
#[test]
fn a_scope_or_a_scoped_delete_changes_the_node_id() {
    let emitted = emit(&ALL, &[]);
    let counter = emitted.counter();
    let [_, _, w3, w4, w5, w6] = vectors(&counter);
    assert_eq!(emitted.frame("own")["body"], w4);
    assert_eq!(emitted.frame("all")["body"], w5);
    assert_eq!(emitted.frame("drop")["body"], w6);
    assert_ne!(emitted.frame_id("own"), emitted.frame_id("all"));
    assert_ne!(emitted.frame_id("drop"), emitted.frame_id("noop"));
    assert_eq!(emitted.frame("noop")["body"], w3);
    assert_eq!(emitted.frame("all")["body"]["deletes_scoped"], json!([]));
}

/// FR-154-AC-4 (TC-573): the frame node of an operation under `modifies self.value`
/// carries the scope `receiver` and under `modifies value` the scope
/// `every-object`, and the two nodes' contents differ; a frame naming `value`
/// in both `modifies_receiver` and `modifies` is refused at intake with
/// `invalid_model_binding`/`conflicting-binding` naming both entries and the
/// operation, so no package and no frame node is built.
#[trace("TC-573", "FR-154-AC-4")]
#[test]
fn the_two_scopes_of_one_field_differ_in_the_node_and_both_at_once_refuse_at_intake() {
    use qsl_semantics::model::intake::{admit_unit, UnitIntakeCause};
    use qsl_semantics::model::refusal::ModelRefusalCause;
    use quire_checked::key::DeclarationKey;

    let emitted = emit(&["own", "all"], &[]);
    let own = emitted.frame("own");
    let all = emitted.frame("all");
    assert_eq!(own["body"]["modifies"][0]["scope"], "receiver");
    assert_eq!(all["body"]["modifies"][0]["scope"], "every-object");
    assert_ne!(own["body"], all["body"]);

    let value = fixture::id("Counter/value");
    let both = json!({
        "modifies": [value], "modifies_receiver": [value], "creates": [], "deletes": [],
    });
    let UnitParts { unit, packages, .. } = unit_of(&["own"], &[("own", both)]);
    let parsed = qsl_cst::parse(
        qsl_foundation::SourceIdentity::new("agent-ix", "test:tc-573"),
        "program.native",
        unit.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 admits the unit");
    let built = qsl_forms::build_unit(&parsed).expect("S2 builds the unit");
    let refusal = admit_unit(
        &built.selections().models,
        &packages,
        qsl_semantics::model::accounting::ModelNormalizationLimits::default(),
    )
    .expect_err("both scopes of one field refuse at intake");
    let UnitIntakeCause::Refused(refusals) = refusal.cause else {
        panic!("{refusal:?}: expected a Refused cause");
    };
    assert_eq!(refusals.len(), 1, "one refusal: {refusals:?}");
    let refusal = &refusals[0];
    assert_eq!(refusal.code.as_str(), "invalid_model_binding");
    let ModelRefusalCause::ConflictingBinding { key, .. } = &refusal.cause else {
        panic!("{:?}: expected ConflictingBinding", refusal.cause);
    };
    assert_eq!(
        key,
        &DeclarationKey {
            package: fixture::PACKAGE.to_owned(),
            node: fixture::id("Counter/own"),
        }
    );
    assert!(
        refusal.detail.contains("modifies[0]") && refusal.detail.contains("modifies_receiver[0]"),
        "names both entries: {}",
        refusal.detail
    );
}

/// FR-105-AC-7 (TC-1043 step 2b): `all` and `same` have equal frames, so the
/// package holds one `state`/`frame` node for both, whose body is W5 and
/// which has a `generated` occurrence for `all` and then one for `same`, and
/// two `operation_anchor` nodes both referencing it; the I2 read admits it.
#[trace("TC-1043", "FR-105-AC-7")]
#[test]
fn equal_frames_share_one_node_with_an_occurrence_per_operation() {
    let emitted = emit(&["noop", "all", "same"], &[]);
    let counter = emitted.counter();
    let [_, _, _, _, w5, _] = vectors(&counter);
    let frames: Vec<&Value> = super::owners::node_where(&emitted.wire, "state", "frame")
        .into_iter()
        .filter(|node| node["body"] == w5)
        .collect();
    let [frame] = frames[..] else {
        panic!("one frame node with body W5: {frames:#?}");
    };
    assert_eq!(emitted.frame_id("all"), frame["node_id"]);
    assert_eq!(emitted.frame_id("same"), frame["node_id"]);
    let occurrences = frame["occurrences"].as_array().expect("occurrences");
    assert_eq!(occurrences.len(), 2, "{occurrences:#?}");
    assert!(occurrences.iter().all(|o| o["role"] == "generated"));
    assert_eq!(
        occurrences
            .iter()
            .map(|o| o["ordinal"].clone())
            .collect::<Vec<_>>(),
        [json!(0), json!(1)]
    );
    let anchors = super::owners::node_where(&emitted.wire, "state", "operation_anchor");
    assert_eq!(anchors.len(), 3, "noop, all and same each have an anchor");
    assert!(matches!(emitted.read(), Read::Verified { .. }));
}

/// Edits the frame node of `operation` in a copy of the emitted package by
/// `edit`, re-keys the node, its anchor and their dependents, then reads the
/// copy (TC-1043 step 4).
fn read_edited(emitted: &Emitted, operation: &str, edit: impl FnOnce(&mut Value)) -> Read {
    let id = emitted.frame_id(operation);
    let mut envelope = emitted.wire.clone();
    let node = envelope["semantic_graph"]["nodes"]
        .as_array_mut()
        .expect("nodes")
        .iter_mut()
        .find(|node| node["node_id"] == id)
        .expect("the frame node");
    edit(&mut node["body"]);
    emitted.read_envelope(&envelope)
}

fn refusal(
    read: &Read,
) -> (
    CheckedPackageRefusalCode,
    Option<CheckedPackageRefusalCause>,
    Code,
) {
    match read {
        Read::Refused(outer @ V2ReadRefusal::Envelope { refusal, .. }) => {
            (refusal.code, refusal.cause, outer.code())
        }
        other => panic!("expected an envelope refusal, got {other:?}"),
    }
}

/// FR-154-AC-10 (TC-1043 step 4, X1 to X3): a field entry with no `scope`, a
/// `scope` other than `receiver` or `every-object`, and a frame body with no
/// `deletes_scoped` each refuse `invalid_semantic_graph` with no cause.
#[trace("TC-1043", "FR-154-AC-10")]
#[test]
fn a_missing_or_unknown_scope_and_a_missing_deletes_scoped_refuse_as_schema_failures() {
    let emitted = emit(&ALL, &[]);
    let x1 = read_edited(&emitted, "bump", |body| {
        body["modifies"][0]
            .as_object_mut()
            .expect("entry")
            .remove("scope");
    });
    let x2 = read_edited(&emitted, "bump", |body| {
        body["modifies"][0]["scope"] = json!("all");
    });
    let x3 = read_edited(&emitted, "bump", |body| {
        body.as_object_mut().expect("body").remove("deletes_scoped");
    });
    for (row, read) in [("X1", x1), ("X2", x2), ("X3", x3)] {
        assert_eq!(
            refusal(&read),
            (
                CheckedPackageRefusalCode::InvalidSemanticGraph,
                None,
                Code::InvalidPackage
            ),
            "{row}"
        );
    }
}

/// FR-154-AC-10 (X4 to X6): `deletes_scoped` with a parameter before the
/// receiver, or `b` before `a`, refuses `invalid_semantic_graph`; a
/// `parameter` entry `c` that `merge` has no parameter for refuses
/// `missing_declaration`/`missing-name`.
#[trace("TC-1043", "FR-154-AC-10")]
#[test]
fn a_misordered_deletes_scoped_and_an_unknown_parameter_refuse() {
    let emitted = emit(&ALL, &[]);
    let receiver = || json!({"kind": "receiver"});
    let parameter = |name: &str| json!({"kind": "parameter", "name": name});
    let x4 = read_edited(&emitted, "merge", |body| {
        body["deletes_scoped"] = json!([parameter("a"), receiver(), parameter("b")]);
    });
    let x5 = read_edited(&emitted, "merge", |body| {
        body["deletes_scoped"] = json!([receiver(), parameter("b"), parameter("a")]);
    });
    for (row, read) in [("X4", x4), ("X5", x5)] {
        assert_eq!(
            refusal(&read),
            (
                CheckedPackageRefusalCode::InvalidSemanticGraph,
                None,
                Code::InvalidPackage
            ),
            "{row}"
        );
    }
    let x6 = read_edited(&emitted, "merge", |body| {
        body["deletes_scoped"] =
            json!([receiver(), parameter("a"), parameter("b"), parameter("c")]);
    });
    assert_eq!(
        refusal(&x6),
        (
            CheckedPackageRefusalCode::MissingDeclaration,
            Some(CheckedPackageRefusalCause::MissingName),
            Code::MissingDeclaration
        )
    );
    // Located at the frame node, in the state step (FR-342 check 5).
    let Read::Refused(V2ReadRefusal::Envelope { refusal, .. }) = &x6 else {
        panic!("{x6:?}");
    };
    assert_eq!(
        refusal
            .locus
            .as_ref()
            .map(|node| serde_json::to_value(node).expect("a node id serialises")),
        Some(emitted.frame_id("merge"))
    );
}

/// FR-154-AC-10 (X7 to X9): the body of W1 re-keyed is admitted; two entries
/// for `step` of both scopes, in either listed order, refuse
/// `invalid_model_binding`/`conflicting-binding` and not
/// `invalid_semantic_graph`.
#[trace("TC-1043", "FR-154-AC-10")]
#[test]
fn both_scopes_of_one_field_refuse_conflicting_binding_in_either_order() {
    let emitted = emit(&ALL, &[]);
    let counter = emitted.counter();
    let step = |scope: &str| json!({"declaration": node_ref(&counter), "kind": "field", "name": "step", "scope": scope});
    let value = json!({"declaration": node_ref(&counter), "kind": "field", "name": "value", "scope": "every-object"});
    let x7 = read_edited(&emitted, "bump", |_| {});
    assert!(matches!(x7, Read::Verified { .. }), "X7: {x7:?}");
    let x8 = read_edited(&emitted, "bump", |body| {
        body["modifies"] = json!([step("receiver"), step("every-object"), value.clone()]);
    });
    let x9 = read_edited(&emitted, "bump", |body| {
        body["modifies"] = json!([step("every-object"), step("receiver"), value.clone()]);
    });
    for (row, read) in [("X8", x8), ("X9", x9)] {
        assert_eq!(
            refusal(&read),
            (
                CheckedPackageRefusalCode::InvalidModelBinding,
                Some(CheckedPackageRefusalCause::ConflictingBinding),
                Code::InvalidModelBinding
            ),
            "{row}"
        );
    }
}

/// FR-154-AC-5 (TC-572 step 3): `drop()` framed `deletes self`, then
/// `dropall()` framed `deletes Counter`, both read from the package the unit
/// selects. `decide_frame` with receiver `c1` over `c1` and `c2`: under
/// `deletes self` the post without `c1` (declared deleted `c1`) is admitted
/// with `c1` as its only deletion, and the post without `c2` is
/// `frame_violation`/`unauthorized-change` naming `c2`; under `deletes
/// Counter` the posts without `c1`, without `c2` and without either are all
/// admitted. The two frames' emitted `state`/`frame` nodes differ.
#[trace("TC-572", "FR-154-AC-5")]
#[test]
fn a_deletes_self_frame_grants_the_receiver_only_and_differs_from_deletes_type() {
    use qsl_semantics::model::domain_package::{
        DomainPackageRecord, DomainPackageRef, OperationEffect,
    };
    use qsl_semantics::model::index::ModelIndex;
    use qsl_semantics::model::intake::{admit, read_domain_package};
    use qsl_semantics::model::normalize::ModelRefusalCause;
    use qsl_semantics::model::population::{
        decide_frame, FrameCharge, FrameDecision, FrameObject, FrameStop,
    };
    use quire_checked::key::DeclarationKey;

    let dropall = json!({"modifies": [], "creates": [], "deletes": [fixture::id("Counter")]});
    let overrides = [("dropall", dropall)];
    let operations = ["drop", "dropall"];

    let emitted = emit(&operations, &overrides);
    assert_ne!(emitted.frame_id("drop"), emitted.frame_id("dropall"));
    assert_ne!(
        emitted.frame("drop")["body"],
        emitted.frame("dropall")["body"]
    );

    let parts = unit_of(&operations, &overrides);
    assert_eq!(parts.packages.len(), 1, "one supplied document");
    let selection = DomainPackageRef {
        identity: fixture::PACKAGE.to_owned(),
    };
    let (selection, document) = admit(
        selection.identity.as_str(),
        &parts.packages,
        qsl_foundation::IntakeLimits::default(),
    )
    .expect("the package is admitted");
    let package = read_domain_package(selection, &document).expect("the package reads");
    let effect_of = |operation: &str| -> OperationEffect {
        let wanted = fixture::id(&format!("Counter/{operation}"));
        package
            .records
            .iter()
            .find_map(|record| match record {
                DomainPackageRecord::OperationMember(member) if member.key.node == wanted => {
                    Some(member.effect.clone())
                }
                _ => None,
            })
            .unwrap_or_else(|| panic!("{operation} is declared"))
    };
    let drop_effect = effect_of("drop");
    let dropall_effect = effect_of("dropall");
    let counter = DeclarationKey {
        package: fixture::PACKAGE.to_owned(),
        node: fixture::id("Counter"),
    };
    assert!(drop_effect.deletes_receiver && drop_effect.deletes.is_empty());
    assert!(!dropall_effect.deletes_receiver && dropall_effect.deletes == [counter.clone()]);

    let index = ModelIndex::build(package);
    let objects = |keys: &[&'static str]| -> Vec<FrameObject<'static, i64>> {
        keys.iter()
            .map(|key| FrameObject {
                object: key,
                type_identity: counter.clone(),
                fields: std::collections::BTreeMap::new(),
            })
            .collect()
    };
    let decide = |effect: &OperationEffect, remaining: &[&'static str], deleted: &[String]| {
        decide_frame(
            &FrameDecision {
                index: &index,
                ancestor_steps: 64,
                effect,
                declared_created: &[],
                declared_deleted: deleted,
                receiver: Some("c1"),
                arguments: &[],
            },
            &objects(&["c1", "c2"]),
            &objects(remaining),
            |_, before, after| before == after,
            &mut |_: FrameCharge| Ok::<(), std::convert::Infallible>(()),
        )
        .map_err(|stop| match stop {
            FrameStop::Refused(refusal) => Box::new(refusal),
            FrameStop::Denied(never) => match never {},
        })
    };
    let gone = |keys: &[&str]| keys.iter().map(|key| (*key).to_owned()).collect::<Vec<_>>();

    let Ok(delta) = decide(&drop_effect, &["c2"], &gone(&["c1"])) else {
        panic!("deletes self grants the receiver");
    };
    assert_eq!(delta.deleted, ["c1".to_owned()].into());
    assert!(delta.created.is_empty());
    let Err(refusal) = decide(&drop_effect, &["c1"], &gone(&["c2"])) else {
        panic!("deletes self does not grant c2");
    };
    assert_eq!(refusal.code, Code::FrameViolation);
    assert_eq!(refusal.cause.as_str(), "unauthorized-change");
    assert_eq!(
        refusal.cause,
        ModelRefusalCause::FrameDeleteOutsideGrant {
            object: "c2".to_owned(),
            type_name: counter.clone(),
        }
    );

    for (remaining, deleted) in [
        (&["c2"][..], &["c1"][..]),
        (&["c1"], &["c2"]),
        (&[], &["c1", "c2"]),
    ] {
        let Ok(delta) = decide(&dropall_effect, remaining, &gone(deleted)) else {
            panic!("deletes Counter grants {deleted:?}");
        };
        assert_eq!(
            delta.deleted,
            deleted.iter().map(|key| (*key).to_owned()).collect()
        );
    }
}

/// FR-105-AC-12 / TC-463: both identity-bound execution-point carriers are
/// admitted together by the current v2 reader and read back unchanged.
#[trace("TC-463", "FR-105-AC-12")]
#[test]
fn execution_point_on_clause_and_anchor_reads_back() {
    let emitted = emit(&["own"], &[]);
    assert!(
        matches!(emitted.read(), Read::Verified { .. }),
        "the point-free package reads back"
    );
    let point = json!({"kind": "post", "operation": "own"});
    let mut envelope = emitted.wire.clone();
    let mut clause = 0;
    let mut anchor = 0;
    for node in envelope["semantic_graph"]["nodes"].as_array_mut().unwrap() {
        match node["semantic_form"].as_str() {
            Some("state_clause") => {
                node["body"]["operation"]["member"]["execution_point"] = point.clone();
                clause += 1;
            }
            Some("operation_anchor") => {
                node["body"]["members"]
                    .as_array_mut()
                    .unwrap()
                    .push(json!({"term": "binding", "name": "execution_point", "value": point}));
                anchor += 1;
            }
            _ => {}
        }
    }
    assert_eq!((clause, anchor), (1, 1));
    assert!(
        matches!(emitted.read_envelope(&envelope), Read::Verified { .. }),
        "the exact clause and anchor carriers read back together"
    );
}

/// FR-105-AC-10 / TC-462: `constraint` is a distinct state-clause member and
/// remains admissible through QSL's V2 reader.
#[trace("TC-462", "FR-105-AC-10")]
#[test]
fn constraint_state_clause_reads_back_as_a_distinct_kind() {
    let mut parts = unit_of(&[], &[]);
    parts
        .unit
        .push_str("constraint Low using v on M::Counter at current { true }\n");
    let graph = assemble_with_models(parts.unit.as_bytes(), &parts.packages)
        .expect("the unit assembles")
        .check(CheckingLimits::default(), CheckMode::Linked)
        .unwrap_or_else(|refusals| panic!("the constraint checks: {refusals:?}"));
    let emission = emit_checked(&CheckedPackage::link(graph)).expect("the package emits");
    let wire = wire(&emission);
    let clause = super::owners::node_where(&wire, "state", "state_clause")
        .into_iter()
        .find(|node| node["body"]["operation"]["member"]["clause"] == "constraint")
        .expect("the distinct constraint clause node");
    assert_eq!(
        clause["body"]["operation"]["member"]["clause"],
        "constraint"
    );

    let mut evidence = CheckedPackageEvidence::new();
    for feature in wire["lock"]["required_features"].as_array().unwrap() {
        evidence.support_feature(feature.as_str().unwrap());
    }
    evidence
        .insert_domain_package_document(
            package_identity(&parts.document),
            parts.document.as_slice(),
        )
        .expect("a fresh evidence key");
    let read = read_v2(&jcs(&wire), library(), V2ReadLimits::fixture(), &evidence);
    assert!(matches!(read, Read::Verified { .. }), "{read:?}");
}
