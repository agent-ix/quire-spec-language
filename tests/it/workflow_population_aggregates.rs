// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-2740: execute source aggregates over explicit reservation and refund
//! populations, retaining group identity, repeated values, and completeness.

use crate::command_harness::{Output, run_file};
use ix_trace_rs::trace;
use serde_json::{Value, json};
use std::path::Path;

const PACKAGE: &str = "test/workflow-aggregates";
const ORDER: &str = "ix://test/workflow-aggregates/Order";
const RESERVATION: &str = "ix://test/workflow-aggregates/Reservation";
const REFUND: &str = "ix://test/workflow-aggregates/Refund";
const ORDERS: &str = "ix://test/workflow-aggregates/orders";
const RESERVATIONS: &str = "ix://test/workflow-aggregates/reservations";
const REFUNDS: &str = "ix://test/workflow-aggregates/refunds";
const DELIVERY_RECEIPT: &str = "ix://test/workflow-aggregates/DeliveryReceipt";
const DELIVERY_RECEIPTS: &str = "ix://test/workflow-aggregates/delivery_receipts";

const UNIT: &str = "language \"ix:native\" edition \"1-draft\";\n\
profile v = \"filament.model.complete/v1\";\n\
model Workflow = \"test/workflow-aggregates\";\n\
type Total = Int[0, 10000];\n\
type Count = Int[0, 4];\n\
function refundTotal using v(order: Workflow::Order, refunds: Population<Workflow::Refund>[4],\
    receipts: Population<Workflow::DeliveryReceipt>[4]): Total pure {\
  sum<Total>(r in allInstances<Workflow::Refund>(refunds):\
    if deref(r).order = order and count<Count>(d in allInstances<Workflow::DeliveryReceipt>(receipts):\
      deref(d).effect = r) > 0\
    then deref(r).amount else 0)\
}\n\
function reservationTotal using v(order: Workflow::Order, reservations: Population<Workflow::Reservation>[4]): Total pure {\
  sum<Total>(r in allInstances<Workflow::Reservation>(reservations):\
    if deref(r).order = order then deref(r).amount else 0)\
}\n\
function refundCount using v(order: Workflow::Order, refunds: Population<Workflow::Refund>[4]): Count pure {\
  count<Count>(r in allInstances<Workflow::Refund>(refunds): deref(r).order = order)\
}\n\
function recovered using v(order: Workflow::Order, reservations: Population<Workflow::Reservation>[4],\
    refunds: Population<Workflow::Refund>[4], receipts: Population<Workflow::DeliveryReceipt>[4]): Boolean pure {\
  reservationTotal(order, reservations) = refundTotal(order, refunds, receipts)\
}\n";

pub(super) fn source(file: &str, document: &str) -> Value {
    json!({
        "file": file,
        "authority": "agent-ix",
        "identity": format!("test:qsl-2740:{file}"),
        "document": document,
        "formal_revision": 1
    })
}

pub(super) fn rename(value: &mut Value, from: &str, to: &str) {
    match value {
        Value::String(text) => *text = text.replace(from, to),
        Value::Array(items) => items.iter_mut().for_each(|item| rename(item, from, to)),
        Value::Object(fields) => fields.values_mut().for_each(|item| rename(item, from, to)),
        _ => {}
    }
}

fn workflow_model() -> Value {
    let mut model: Value = serde_json::from_str(include_str!(
        "../../examples/config-version/model.semantic-ir.json"
    ))
    .expect("the example Semantic IR package parses");
    rename(
        &mut model,
        "example/config-version",
        "test/workflow-aggregates",
    );
    rename(&mut model, "ConfigVersion", "Order");
    rename(&mut model, "config_history", "orders");

    let types = model["types"].as_array_mut().expect("types array");
    let order_index = types
        .iter()
        .position(|ty| ty["identity"] == ORDER)
        .expect("the Order object type");
    let mut order = types[order_index].clone();
    order["operations"] = json!([]);

    let workflow_type = |name: &str, identity: &str| {
        let mut ty = order.clone();
        rename(&mut ty, "/Order", &format!("/{name}"));
        ty["identity"] = json!(identity);
        ty["displayName"] = json!(identity);
        let amount_id = format!("{identity}/amount");
        let relation_id = format!("{identity}/order");
        let mut amount = ty["fields"][0].clone();
        amount["identity"] = json!(amount_id);
        amount["name"] = json!("amount");
        amount["origin"]["generated"]["generatorIdentity"] = json!(amount_id);
        amount["origin"]["generated"]["inputIdentities"] = json!([amount_id]);
        let mut related_order = amount.clone();
        related_order["identity"] = json!(relation_id);
        related_order["name"] = json!("order");
        related_order["typeRef"] = json!(ORDER);
        related_order["presence"] = json!("required");
        related_order["origin"]["generated"]["generatorIdentity"] = json!(relation_id);
        related_order["origin"]["generated"]["inputIdentities"] = json!([relation_id]);
        ty["fields"] = json!([amount, related_order]);
        ty
    };
    let reservation = workflow_type("Reservation", RESERVATION);
    let refund = workflow_type("Refund", REFUND);
    let mut delivery_receipt = workflow_type("DeliveryReceipt", DELIVERY_RECEIPT);
    let mut effect = delivery_receipt["fields"][0].clone();
    let effect_id = format!("{DELIVERY_RECEIPT}/effect");
    effect["identity"] = json!(effect_id);
    effect["name"] = json!("effect");
    effect["typeRef"] = json!(REFUND);
    effect["origin"]["generated"]["generatorIdentity"] = json!(effect_id);
    effect["origin"]["generated"]["inputIdentities"] = json!([effect_id]);
    delivery_receipt["fields"] = json!([effect]);
    types.push(reservation);
    types.push(refund);
    types.push(delivery_receipt);

    let populations = model["populations"]
        .as_array_mut()
        .expect("populations array");
    let mut orders = populations[0].clone();
    let mut reservations = orders.clone();
    let mut refunds = orders.clone();
    let mut delivery_receipts = orders.clone();
    reservations["identity"] = json!(RESERVATIONS);
    reservations["displayName"] = json!(RESERVATIONS);
    reservations["members"] = json!([RESERVATION]);
    reservations["origin"]["generated"]["generatorIdentity"] = json!(RESERVATIONS);
    reservations["origin"]["generated"]["inputIdentities"] = json!([RESERVATIONS]);
    refunds["identity"] = json!(REFUNDS);
    refunds["displayName"] = json!(REFUNDS);
    refunds["members"] = json!([REFUND]);
    refunds["origin"]["generated"]["generatorIdentity"] = json!(REFUNDS);
    refunds["origin"]["generated"]["inputIdentities"] = json!([REFUNDS]);
    delivery_receipts["identity"] = json!(DELIVERY_RECEIPTS);
    delivery_receipts["displayName"] = json!(DELIVERY_RECEIPTS);
    delivery_receipts["members"] = json!([DELIVERY_RECEIPT]);
    delivery_receipts["origin"]["generated"]["generatorIdentity"] = json!(DELIVERY_RECEIPTS);
    delivery_receipts["origin"]["generated"]["inputIdentities"] = json!([DELIVERY_RECEIPTS]);
    orders["members"] = json!([ORDER]);
    populations.clear();
    populations.extend([orders, reservations, refunds, delivery_receipts]);
    model
}

fn related_order(key: &str) -> Value {
    json!({"reference": {"population": ORDERS, "key": key}})
}

fn workflow_snapshot(complete_refunds: bool, add_contradictory_refund: bool) -> Value {
    let mut refund_objects = vec![
        json!({"key": "r1", "type": REFUND, "fields": {
            "amount": {"integer": "2"}, "order": related_order("o1")}}),
        json!({"key": "r2", "type": REFUND, "fields": {
            "amount": {"integer": "2"}, "order": related_order("o1")}}),
        json!({"key": "r3", "type": REFUND, "fields": {
            "amount": {"integer": "4"}, "order": related_order("o2")}}),
    ];
    if add_contradictory_refund {
        refund_objects.push(json!({"key": "r4", "type": REFUND, "fields": {
            "amount": {"integer": "1"}, "order": related_order("o1")}}));
    }
    let mut delivery_receipt_objects = vec![
        json!({"key": "d1", "type": DELIVERY_RECEIPT, "fields": {
            "effect": {"reference": {"population": REFUNDS, "key": "r1"}}}}),
        json!({"key": "d2", "type": DELIVERY_RECEIPT, "fields": {
            "effect": {"reference": {"population": REFUNDS, "key": "r1"}}}}),
        json!({"key": "d3", "type": DELIVERY_RECEIPT, "fields": {
            "effect": {"reference": {"population": REFUNDS, "key": "r2"}}}}),
    ];
    if add_contradictory_refund {
        delivery_receipt_objects.push(json!({"key": "d4", "type": DELIVERY_RECEIPT, "fields": {
            "effect": {"reference": {"population": REFUNDS, "key": "r4"}}}}));
    }
    json!({
        "format": "filament.state.snapshot/v1",
        "identity": {"authority": "agent-ix", "identity": "ix://test/workflow-aggregates/snapshot"},
        "observation": "current",
        "anchor": {"kind": "handler", "name": "settle"},
        "model": {"identity": PACKAGE},
        "populations": [
            {"population": ORDERS, "complete": true, "objects": [
                {"key": "o1", "type": ORDER, "fields": {"versionNumber": {"integer": "4"},
                    "parent": {"absent": {}}}},
                {"key": "o2", "type": ORDER, "fields": {"versionNumber": {"integer": "7"},
                    "parent": {"absent": {}}}}
            ]},
            {"population": RESERVATIONS, "complete": true, "objects": [
                {"key": "s1", "type": RESERVATION, "fields": {
                    "amount": {"integer": "2"}, "order": related_order("o1")}},
                {"key": "s2", "type": RESERVATION, "fields": {
                    "amount": {"integer": "2"}, "order": related_order("o1")}},
                {"key": "s3", "type": RESERVATION, "fields": {
                    "amount": {"integer": "7"}, "order": related_order("o2")}}
            ]},
            {"population": REFUNDS, "complete": complete_refunds, "objects": refund_objects},
            {"population": DELIVERY_RECEIPTS, "complete": true, "objects": delivery_receipt_objects}
        ]
    })
}

fn write_run(directory: &Path, complete_refunds: bool, add_contradictory_refund: bool) {
    std::fs::write(
        directory.join("model.semantic-ir.json"),
        workflow_model().to_string(),
    )
    .unwrap();
    std::fs::write(directory.join("workflow.native"), UNIT).unwrap();
    std::fs::write(
        directory.join("snapshot.json"),
        workflow_snapshot(complete_refunds, add_contradictory_refund).to_string(),
    )
    .unwrap();
    let request = json!({"format": "native-run/1", "request": {
        "models": [{"format": "semantic-ir/2.0.0", "source": source("model.semantic-ir.json", "WorkflowModel")}],
        "program": {"source": source("workflow.native", "WorkflowUnit")},
        "call": {"function": "recovered", "snapshot": "snapshot.json", "arguments": [
            {"parameter": "order", "value": {"reference": {"population": ORDERS, "key": "o1"}}},
            {"parameter": "reservations", "value": {"population": RESERVATIONS}},
            {"parameter": "refunds", "value": {"population": REFUNDS}},
            {"parameter": "receipts", "value": {"population": DELIVERY_RECEIPTS}}
        ]}
    }});
    std::fs::write(
        directory.join("request.json"),
        serde_json::to_vec(&request).unwrap(),
    )
    .unwrap();
}

fn run(complete_refunds: bool, add_contradictory_refund: bool) -> Output {
    let directory = tempfile::tempdir().unwrap();
    write_run(directory.path(), complete_refunds, add_contradictory_refund);
    run_file(&directory.path().join("request.json"))
}

fn outcome(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "run produced an outcome document: {error}; stderr={}",
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

// These are the external filament-specification criteria. The FSpec prefix
// distinguishes their identifiers from same-numbered criteria in this repo.
#[test]
#[trace(
    "FSpec-FR-180-AC-5",
    "FSpec-FR-049-AC-2",
    "FSpec-FR-049-AC-8",
    "FSpec-FR-046-AC-6",
    "FSpec-TC-173"
)]
fn source_aggregates_count_effects_not_duplicate_delivery_receipts() {
    // The source joins receipt effects to the refund population by identity.
    // d1 and d2 both reference r1, so r1 contributes once; d3 references
    // distinct r2, which also contributes. Counting receipt rows gives 6,
    // while deduplicating by amount gives 2; the correct effect total is 4.
    let output = run(true, false);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let document = outcome(&output);
    assert_eq!(document["outcome"]["kind"], "completed", "{document}");
    assert_eq!(
        document["outcome"]["value"],
        json!({"boolean": true}),
        "{document}"
    );
}

#[test]
#[trace("FSpec-FR-180-AC-5", "FSpec-FR-049-AC-8", "FSpec-FR-046-AC-6")]
fn an_incomplete_refund_population_cannot_settle_a_recovery_aggregate() {
    let output = run(false, false);
    assert_eq!(output.status.code(), Some(22), "{output:?}");
    let document = outcome(&output);
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
        document["outcome"]["fields"]["population"], REFUNDS,
        "{document}"
    );
    assert!(document["outcome"]["location"].is_object(), "{document}");
    assert!(document["outcome"].get("value").is_none(), "{document}");
}

#[test]
#[trace(
    "FSpec-FR-180-AC-5",
    "FSpec-FR-049-AC-8",
    "FSpec-FR-046-AC-6",
    "FSpec-TC-173"
)]
fn a_contradictory_refund_total_does_not_pass_recovery() {
    // r4 is a distinct effect with its own receipt; unlike the repeated r1
    // receipt, it adds one to the aggregate and makes the order unrecovered.
    let output = run(true, true);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let document = outcome(&output);
    assert_eq!(document["outcome"]["kind"], "completed", "{document}");
    assert_eq!(
        document["outcome"]["value"],
        json!({"boolean": false}),
        "{document}"
    );
}
