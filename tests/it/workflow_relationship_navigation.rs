// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-2740 PR 2a: source navigation follows the model relationships listed
//! by a native-run snapshot, with concrete workflow object keys.

use crate::command_harness::{run_file, Output};
use ix_trace_rs::trace;
use serde_json::{json, Value};
use std::path::Path;

const PACKAGE: &str = "test/workflow-relationships";
const ORDER: &str = "ix://test/workflow-relationships/Order";
const SHIPMENT: &str = "ix://test/workflow-relationships/Shipment";
const PAYMENT_ATTEMPT: &str = "ix://test/workflow-relationships/PaymentAttempt";
const REFUND: &str = "ix://test/workflow-relationships/Refund";
const PROVIDER: &str = "ix://test/workflow-relationships/Provider";
const RECORDS: &str = "ix://test/workflow-relationships/records";
const SHIPMENTS: &str = "ix://test/workflow-relationships/relationship/Order-ships-Shipment";
const PAYMENTS: &str =
    "ix://test/workflow-relationships/relationship/Order-hasPaymentAttempt-PaymentAttempt";
const REFUNDS: &str = "ix://test/workflow-relationships/relationship/Order-hasRefund-Refund";
const PROVIDERS: &str = "ix://test/workflow-relationships/relationship/Order-usesProvider-Provider";

const UNIT: &str = "language \"ix:native\" edition \"1-draft\";\n\
profile v = \"filament.model.complete/v1\";\n\
model Workflow = \"test/workflow-relationships\";\n\
function hasShipment using v(order: Reference<Workflow::Order>, shipment: Reference<Workflow::Shipment>): Boolean pure {\
  reaches(order, shipment, shipments)\
}\n\
function hasPayment using v(order: Reference<Workflow::Order>, payment: Reference<Workflow::PaymentAttempt>): Boolean pure {\
  reaches(order, payment, paymentAttempts)\
}\n\
function hasRefund using v(order: Reference<Workflow::Order>, refund: Reference<Workflow::Refund>): Boolean pure {\
  reaches(order, refund, refunds)\
}\n\
function hasProvider using v(order: Reference<Workflow::Order>, provider: Reference<Workflow::Provider>): Boolean pure {\
  reaches(order, provider, usesProvider)\
}\n";

fn rename(value: &mut Value, from: &str, to: &str) {
    match value {
        Value::String(text) => *text = text.replace(from, to),
        Value::Array(items) => items.iter_mut().for_each(|item| rename(item, from, to)),
        Value::Object(fields) => fields.values_mut().for_each(|item| rename(item, from, to)),
        _ => {}
    }
}

fn relationship(identity: &str, source_role: &str, target_type: &str, target_role: &str) -> Value {
    json!({
        "identity": identity,
        "sourceEnd": {
            "type": ORDER,
            "role": source_role,
            "multiplicity": {"lower": 0, "upper": null, "ordered": false, "unique": true}
        },
        "targetEnd": {
            "type": target_type,
            "role": target_role,
            "multiplicity": {"lower": 0, "upper": 1, "ordered": false, "unique": true}
        },
        "category": "structural",
        "composite": false,
        "direction": "source-to-target",
        "origin": {"source": {
            "sourceIdentity": "ix://test/workflow-relationships/spec",
            "path": "workflow.fspec",
            "startLine": 1,
            "startColumn": 1
        }}
    })
}

fn workflow_model() -> Value {
    let mut model: Value = serde_json::from_str(include_str!(
        "../../examples/config-version/model.semantic-ir.json"
    ))
    .expect("the example Semantic IR package parses");
    rename(&mut model, "example/config-version", PACKAGE);
    rename(&mut model, "ConfigVersion", "Order");
    rename(&mut model, "config_history", "records");

    let types = model["types"].as_array_mut().expect("types array");
    let order_index = types
        .iter()
        .position(|ty| ty["identity"] == ORDER)
        .expect("the Order object type");
    let mut order = types[order_index].clone();
    order["fields"] = json!([]);
    order["operations"] = json!([]);
    order["relationships"] = json!([
        relationship(SHIPMENTS, "shipments", SHIPMENT, "order"),
        relationship(PAYMENTS, "paymentAttempts", PAYMENT_ATTEMPT, "order"),
        relationship(REFUNDS, "refunds", REFUND, "order"),
        relationship(PROVIDERS, "usesProvider", PROVIDER, "orders")
    ]);

    let workflow_type = |name: &str, identity: &str| {
        let mut ty = order.clone();
        rename(&mut ty, "/Order", &format!("/{name}"));
        ty["identity"] = json!(identity);
        ty["displayName"] = json!(identity);
        ty["origin"]["generated"]["generatorIdentity"] = json!(identity);
        ty["origin"]["generated"]["inputIdentities"] = json!([identity]);
        ty["relationships"] = json!([]);
        ty
    };
    types[order_index] = order.clone();
    types.extend([
        workflow_type("Shipment", SHIPMENT),
        workflow_type("PaymentAttempt", PAYMENT_ATTEMPT),
        workflow_type("Refund", REFUND),
        workflow_type("Provider", PROVIDER),
    ]);

    let populations = model["populations"]
        .as_array_mut()
        .expect("populations array");
    let mut records = populations[0].clone();
    records["identity"] = json!(RECORDS);
    records["displayName"] = json!(RECORDS);
    records["members"] = json!([ORDER, SHIPMENT, PAYMENT_ATTEMPT, REFUND, PROVIDER]);
    records["origin"]["generated"]["generatorIdentity"] = json!(RECORDS);
    records["origin"]["generated"]["inputIdentities"] = json!([RECORDS]);
    populations.clear();
    populations.push(records);
    model
}

fn reference(key: &str) -> Value {
    json!({"population": RECORDS, "key": key})
}

fn instance(source: &str, target: &str) -> Value {
    json!({"source": reference(source), "target": reference(target)})
}

fn workflow_snapshot(relationships: Vec<Value>) -> Value {
    json!({
        "format": "filament.state.snapshot/v1",
        "identity": {"authority": "agent-ix", "identity": "ix://test/workflow-relationships/snapshot"},
        "observation": "current",
        "anchor": {"kind": "handler", "name": "settle"},
        "model": {"identity": PACKAGE},
        "populations": [{
            "population": RECORDS,
            "complete": true,
            "objects": [
                {"key": "o1", "type": ORDER, "fields": {}},
                {"key": "o2", "type": ORDER, "fields": {}},
                {"key": "s1", "type": SHIPMENT, "fields": {}},
                {"key": "s2", "type": SHIPMENT, "fields": {}},
                {"key": "s3", "type": SHIPMENT, "fields": {}},
                {"key": "p1", "type": PAYMENT_ATTEMPT, "fields": {}},
                {"key": "p2", "type": PAYMENT_ATTEMPT, "fields": {}},
                {"key": "r1", "type": REFUND, "fields": {}},
                {"key": "r2", "type": REFUND, "fields": {}},
                {"key": "provider0", "type": PROVIDER, "fields": {}}
            ]
        }],
        "relationships": relationships
    })
}

fn source(file: &str, document: &str) -> Value {
    json!({
        "file": file,
        "authority": "agent-ix",
        "identity": format!("test:qsl-2740:{file}"),
        "document": document,
        "formal_revision": 1
    })
}

fn relationship_row(identity: &str, complete: bool, instances: Vec<Value>) -> Value {
    json!({"relationship": identity, "complete": complete, "instances": instances})
}

fn run(function: &str, first_key: &str, second_key: &str, relationships: Vec<Value>) -> Output {
    let directory = tempfile::tempdir().unwrap();
    let path: &Path = directory.path();
    std::fs::write(
        path.join("model.semantic-ir.json"),
        workflow_model().to_string(),
    )
    .unwrap();
    std::fs::write(path.join("workflow.native"), UNIT).unwrap();
    std::fs::write(
        path.join("snapshot.json"),
        workflow_snapshot(relationships).to_string(),
    )
    .unwrap();
    let second_parameter = match function {
        "hasShipment" => "shipment",
        "hasPayment" => "payment",
        "hasRefund" => "refund",
        _ => "provider",
    };
    let request = json!({"format": "native-run/1", "request": {
        "models": [{"format": "semantic-ir/2.0.0", "source": source("model.semantic-ir.json", "WorkflowModel")}],
        "program": {"source": source("workflow.native", "WorkflowUnit")},
        "call": {"function": function, "snapshot": "snapshot.json", "arguments": [
            {"parameter": "order", "value": {"reference": {"population": RECORDS, "key": first_key}}},
            {"parameter": second_parameter,
                "value": {"reference": {"population": RECORDS, "key": second_key}}}
        ]}
    }});
    std::fs::write(
        path.join("request.json"),
        serde_json::to_vec(&request).unwrap(),
    )
    .unwrap();
    run_file(&path.join("request.json"))
}

fn result(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "run produced an outcome document: {error}; stderr={}",
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

fn relationship_set() -> Vec<Value> {
    vec![
        relationship_row(
            SHIPMENTS,
            true,
            vec![
                instance("o1", "s1"),
                instance("o1", "s2"),
                instance("o2", "s3"),
            ],
        ),
        relationship_row(
            PAYMENTS,
            true,
            vec![instance("o1", "p1"), instance("o2", "p2")],
        ),
        relationship_row(
            REFUNDS,
            true,
            vec![instance("o1", "r1"), instance("o2", "r2")],
        ),
        relationship_row(
            PROVIDERS,
            true,
            vec![instance("o1", "provider0"), instance("o2", "provider0")],
        ),
    ]
}

fn assert_boolean(function: &str, first_key: &str, second_key: &str, expected: bool) {
    let output = run(function, first_key, second_key, relationship_set());
    let document = result(&output);
    assert_eq!(output.status.code(), Some(0), "{document}");
    assert_eq!(document["outcome"]["kind"], "completed", "{document}");
    assert_eq!(
        document["outcome"]["value"],
        json!({"boolean": expected}),
        "{document}"
    );
}

#[test]
#[trace(
    "FR-106-AC-23",
    "FR-364-AC-7",
    "FSpec-FR-049-AC-7",
    "FSpec-FR-050-AC-3"
)]
fn source_navigation_keeps_concrete_related_workflow_instances_distinct() {
    assert_boolean("hasShipment", "o1", "s1", true);
    assert_boolean("hasShipment", "o1", "s2", true);
    assert_boolean("hasShipment", "o1", "s3", false);
    assert_boolean("hasShipment", "o2", "s3", true);
    assert_boolean("hasPayment", "o1", "p1", true);
    assert_boolean("hasPayment", "o1", "p2", false);
    assert_boolean("hasPayment", "o2", "p2", true);
    assert_boolean("hasRefund", "o1", "r1", true);
    assert_boolean("hasRefund", "o1", "r2", false);
    assert_boolean("hasRefund", "o2", "r2", true);
    assert_boolean("hasProvider", "o1", "provider0", true);
    assert_boolean("hasProvider", "o2", "provider0", true);
}

#[test]
#[trace(
    "FR-106-AC-23",
    "FR-364-AC-9",
    "FSpec-FR-049-AC-7",
    "FSpec-FR-049-AC-8"
)]
fn source_navigation_keeps_complete_empty_distinct_from_missing_completeness() {
    let complete_empty = run(
        "hasRefund",
        "o1",
        "r1",
        vec![relationship_row(REFUNDS, true, Vec::new())],
    );
    let document = result(&complete_empty);
    assert_eq!(complete_empty.status.code(), Some(0), "{document}");
    assert_eq!(document["outcome"]["kind"], "completed", "{document}");
    assert_eq!(
        document["outcome"]["value"],
        json!({"boolean": false}),
        "{document}"
    );

    for relationships in [
        vec![relationship_row(REFUNDS, false, Vec::new())],
        Vec::new(),
    ] {
        let output = run("hasRefund", "o1", "r1", relationships);
        let document = result(&output);
        assert_eq!(output.status.code(), Some(22), "{document}");
        assert_eq!(document["outcome"]["kind"], "refused", "{document}");
        assert_eq!(
            document["outcome"]["code"], "incomplete_population",
            "{document}"
        );
        assert_eq!(
            document["outcome"]["cause"], "incomplete-scope",
            "{document}"
        );
        assert_eq!(
            document["outcome"]["fields"]["relationship"], REFUNDS,
            "{document}"
        );
        assert!(document["outcome"].get("value").is_none(), "{document}");
    }
}
