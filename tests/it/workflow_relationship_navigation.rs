// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-2740 PR 2a: source navigation follows the model relationships listed
//! by a native-run snapshot, with concrete workflow object keys.

use crate::command_harness::{run_file, Output};
use crate::workflow_population_aggregates::{rename, source};
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
function shipments using v(order: Reference<Workflow::Order>): Set<Reference<Workflow::Shipment>>[0,3] pure { order.shipments }\n\
function paymentAttempts using v(order: Reference<Workflow::Order>): Reference<Workflow::PaymentAttempt> pure { order.paymentAttempts }\n\
function refunds using v(order: Reference<Workflow::Order>): Option<Reference<Workflow::Refund>> pure { order.refunds }\n\
function provider using v(order: Reference<Workflow::Order>): Option<Reference<Workflow::Provider>> pure { order.usesProvider }\n\
function providerOrders using v(provider: Reference<Workflow::Provider>): Set<Reference<Workflow::Order>>[0,3] pure { provider.orders }\n";

fn relationship(
    identity: &str,
    source_role: &str,
    target_type: &str,
    target_role: &str,
    target_lower: u64,
    target_upper: u64,
    direction: &str,
) -> Value {
    json!({
        "identity": identity,
        "sourceEnd": {
            "type": ORDER,
            "role": source_role,
            "multiplicity": {"lower": 0, "upper": 3, "ordered": false, "unique": true}
        },
        "targetEnd": {
            "type": target_type,
            "role": target_role,
            "multiplicity": {"lower": target_lower, "upper": target_upper, "ordered": false, "unique": true}
        },
        "category": "structural",
        "composite": false,
        "direction": direction,
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
        relationship(
            SHIPMENTS,
            "shipments",
            SHIPMENT,
            "order",
            0,
            3,
            "source-to-target"
        ),
        relationship(
            PAYMENTS,
            "paymentAttempts",
            PAYMENT_ATTEMPT,
            "order",
            1,
            1,
            "source-to-target",
        ),
        relationship(
            REFUNDS,
            "refunds",
            REFUND,
            "order",
            0,
            1,
            "source-to-target"
        ),
        relationship(
            PROVIDERS,
            "usesProvider",
            PROVIDER,
            "orders",
            0,
            1,
            "bidirectional"
        )
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

fn relationship_row(identity: &str, complete: bool, instances: Vec<Value>) -> Value {
    json!({"relationship": identity, "complete": complete, "instances": instances})
}

fn run(function: &str, input_key: &str, relationships: Vec<Value>) -> Output {
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
    let request = json!({"format": "native-run/1", "request": {
        "models": [{"format": "semantic-ir/2.0.0", "source": source("model.semantic-ir.json", "WorkflowModel")}],
        "program": {"source": source("workflow.native", "WorkflowUnit")},
        "call": {"function": function, "snapshot": "snapshot.json", "arguments": [
            {"parameter": if function == "providerOrders" { "provider" } else { "order" },
                "value": {"reference": {"population": RECORDS, "key": input_key}}}
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

fn assert_value(function: &str, input_key: &str, expected: Value) {
    let output = run(function, input_key, relationship_set());
    let document = result(&output);
    assert_eq!(output.status.code(), Some(0), "{document}");
    assert_eq!(document["outcome"]["kind"], "completed", "{document}");
    assert_eq!(document["outcome"]["value"], expected, "{document}");
}

#[test]
#[trace("FR-363-AC-1", "FR-364-AC-7")]
fn source_relationship_members_return_concrete_workflow_references() {
    let reference = |key| json!({"reference": {"population": RECORDS, "key": key}});
    assert_value(
        "shipments",
        "o1",
        json!({"set": [reference("s1"), reference("s2")]}),
    );
    assert_value("shipments", "o2", json!({"set": [reference("s3")]}));
    assert_value("paymentAttempts", "o1", reference("p1"));
    assert_value("paymentAttempts", "o2", reference("p2"));
    assert_value("refunds", "o1", json!({"present": reference("r1")}));
    assert_value("refunds", "o2", json!({"present": reference("r2")}));
    assert_value("provider", "o1", json!({"present": reference("provider0")}));
    assert_value("provider", "o2", json!({"present": reference("provider0")}));
    assert_value(
        "providerOrders",
        "provider0",
        json!({"set": [reference("o1"), reference("o2")]}),
    );
}

#[test]
#[trace("FR-363-AC-1", "FR-364-AC-9")]
fn source_relationship_members_distinguish_empty_and_incomplete_extents() {
    let complete_empty = run(
        "refunds",
        "o1",
        vec![relationship_row(REFUNDS, true, Vec::new())],
    );
    let document = result(&complete_empty);
    assert_eq!(complete_empty.status.code(), Some(0), "{document}");
    assert_eq!(document["outcome"]["kind"], "completed", "{document}");
    assert_eq!(
        document["outcome"]["value"],
        json!({"absent": {}}),
        "{document}"
    );

    for relationships in [
        vec![relationship_row(REFUNDS, false, Vec::new())],
        Vec::new(),
    ] {
        let output = run("refunds", "o1", relationships);
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
