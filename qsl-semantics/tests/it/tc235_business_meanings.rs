// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-235 M01's literal K2 intake control. This is intentionally a red
//! acceptance vector until the published native type contract and the
//! QSL reader agree; the fixture must not be edited to hide that gap.

use qsl_foundation::{ByteDigest, IntakeLimits};
use qsl_semantics::model::accounting::{ChargePoint, ModelNormalizationLimits};
use qsl_semantics::model::domain_package::DomainPackageRef;
use qsl_semantics::model::intake::{admit, meaning, package_input, read_domain_package};
use qsl_semantics::model::key::SHA256_JCS_DIGEST_DOMAIN;
use qsl_semantics::model::normalize::{NormalizeOutcome, normalize_with_meter};
use serde_json::{Value, json};

const PACKAGE: &str = "test/orders";
const BUSINESS: &str = "test/business";

fn id(node: &str) -> String {
    format!("ix://{PACKAGE}/{node}")
}

fn origin(node: &str) -> Value {
    let identity = id(node);
    json!({"generated": {
        "generatorIdentity": identity,
        "generatorVersion": "1.0.0",
        "inputIdentities": [identity],
    }})
}

fn construct(name: &str, semantic_meaning: &str) -> Value {
    let members = match semantic_meaning {
        meaning::OBJECT_TYPE => json!({"identityFields": "required"}),
        meaning::RECORD_VALUE_TYPE => json!({"identityFields": "optional"}),
        meaning::EVENT_TYPE => json!({"identityFields": "optional", "occurrenceField": "required"}),
        meaning::STATE_MACHINE => json!({"states": "required", "transitions": "required"}),
        meaning::PROCESS => json!({"identityFields": "required", "steps": "required"}),
        meaning::PERSISTENCE_INTERFACE => json!({"persists": "required"}),
        meaning::NAMESPACE => json!({"members": "required", "vocabulary": "required"}),
        _ => json!({}),
    };
    json!({
        "kind": {"module": BUSINESS, "name": name},
        "moduleVersion": "1.0.0",
        "construct": {
            "identity": "none", "shape": "record", "members": members,
            "meaning": semantic_meaning,
        },
    })
}

fn declaration(name: &str, kind: &str, members: Value) -> Value {
    let mut value = json!({
        "identity": id(name), "displayName": name,
        "kind": {"module": BUSINESS, "name": kind},
        "roles": [], "origin": origin(name),
        "constraints": [], "extensions": [], "unknownPolicy": "reject",
    });
    value
        .as_object_mut()
        .unwrap()
        .extend(members.as_object().unwrap().clone());
    value
}

fn field(owner: &str, name: &str, type_ref: &str) -> Value {
    let node = format!("{owner}/{name}");
    json!({
        "identity": id(&node), "name": name, "typeRef": type_ref,
        "presence": "required", "nullable": false, "defaultKind": "none",
        "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
        "origin": origin(&node),
    })
}

fn operation(owner: &str, name: &str) -> Value {
    let node = format!("{owner}/{name}");
    json!({
        "identity": id(&node), "name": name, "params": [], "pre": [], "post": [],
        "frame": {"modifies": [], "creates": [], "deletes": []},
        "origin": origin(&node),
    })
}

fn clause(owner: &str, name: &str, text: &str) -> Value {
    let node = format!("{owner}/{name}");
    json!({
        "identity": id(&node), "language": "quire", "clauseId": name,
        "text": text, "origin": origin(&node),
    })
}

fn k2() -> Value {
    let state = |name: &str| {
        let node = format!("OrderLifecycle/{name}");
        json!({"identity": id(&node), "name": name, "origin": origin(&node)})
    };
    let transition = |name: &str, from: &str, to: &str, trigger: &str, emits: Vec<String>| {
        let node = format!("OrderLifecycle/{name}");
        json!({
            "identity": id(&node), "from": id(&format!("OrderLifecycle/{from}")),
            "to": id(&format!("OrderLifecycle/{to}")),
            "trigger": id(&format!("OrderLifecycle/{trigger}")),
            "emits": emits, "origin": origin(&node),
        })
    };
    let step = |name: &str, kind: &str, consumes: Vec<String>| {
        let node = format!("Fulfilment/{name}");
        json!({
            "identity": id(&node), "name": name, "stepKind": kind,
            "consumes": consumes, "emits": [], "origin": origin(&node),
        })
    };
    let mut first = transition(
        "transition0",
        "Draft",
        "Placed",
        "place",
        vec![id("OrderPlaced")],
    );
    first["guard"] = json!(id("OrderLifecycle/NotYetPlaced"));

    let types = vec![
        declaration(
            "Order",
            "thing",
            json!({
                "identityFields": [id("Order/order_id")],
                "fields": [field("Order", "order_id", "ix://quire/native/UUID"),
                           field("Order", "total", &id("Money"))],
            }),
        ),
        declaration("Instant", "value", json!({"scalar": "Timestamp"})),
        declaration(
            "Money",
            "money",
            json!({
                "fields": [field("Money", "amount_minor", "ix://quire/native/Int"),
                           field("Money", "currency", "ix://quire/native/String")],
            }),
        ),
        declaration(
            "OrderPlaced",
            "happened",
            json!({
                "occurrenceField": id("OrderPlaced/occurred_at"),
                "fields": [field("OrderPlaced", "occurred_at", &id("Instant")),
                           field("OrderPlaced", "order_id", "ix://quire/native/UUID")],
            }),
        ),
        declaration(
            "OrderLifecycle",
            "lifecycle",
            json!({
                "fields": [field("OrderLifecycle", "placed", "ix://quire/native/Boolean")],
                "operations": [operation("OrderLifecycle", "place"), operation("OrderLifecycle", "cancel")],
                "clauses": [clause("OrderLifecycle", "NotYetPlaced", "not self.placed")],
                "states": [state("Draft"), state("Placed"), state("Cancelled")],
                "transitions": [first,
                    transition("transition1", "Draft", "Cancelled", "cancel", vec![]),
                    transition("transition2", "Placed", "Cancelled", "cancel", vec![])],
            }),
        ),
        declaration(
            "Fulfilment",
            "saga",
            json!({
                "identityFields": [id("Fulfilment/correlation_id")],
                "fields": [field("Fulfilment", "correlation_id", "ix://quire/native/UUID")],
                "steps": [step("receive_order", "event", vec![id("OrderPlaced")]),
                          step("reserve_stock", "command", vec![]),
                          step("await_shipment", "wait", vec![])],
            }),
        ),
        declaration(
            "OrderStore",
            "store",
            json!({
                "fields": [],
                "operations": [operation("OrderStore", "get"), operation("OrderStore", "save")],
                "persists": [id("Order")],
            }),
        ),
        declaration(
            "OrderManagement",
            "context",
            json!({
                "members": [id("Order"), id("Money"), id("OrderPlaced")],
                "vocabulary": [{"term": "Place", "doc": "Commit a draft order",
                                "origin": origin("OrderManagement")}],
            }),
        ),
    ];
    let mut constructs = vec![
        construct("thing", meaning::OBJECT_TYPE),
        construct("value", meaning::VALUE_TYPE),
        construct("money", meaning::RECORD_VALUE_TYPE),
        construct("happened", meaning::EVENT_TYPE),
        construct("lifecycle", meaning::STATE_MACHINE),
        construct("saga", meaning::PROCESS),
        construct("store", meaning::PERSISTENCE_INTERFACE),
        construct("context", meaning::NAMESPACE),
        construct("population", meaning::POPULATION),
    ];
    let digest = format!(
        "sha256:{}",
        qsl_semantics::model::key::hex(
            &ByteDigest::of(&serde_json::to_vec(&types).unwrap()).as_bytes()
        )
    );
    for item in &mut constructs {
        item["manifestDigest"] = json!(digest);
    }
    json!({
        "contractVersion": "2.0.0",
        "source": {"identity": id("spec"), "version": "1.0.0", "dialect": "spec-bundle", "digest": digest},
        "package": {"identity": PACKAGE, "version": "1", "manifestDigest": digest,
                    "mappingVersions": [], "profileVersions": [], "lockDigest": digest},
        "constructs": constructs, "types": types, "occurrences": [], "extensions": [],
    })
}

/// M01: literal TC-235 K2 must admit as 21 declarations and normalize with
/// exactly 21 `normalize.record` charges. Current FR-056 AC-17 and the
/// `read_value_type_ref` R5 rule disagree on K2's native UUID/String/Timestamp
/// references; keep this as an admission expectation rather than changing K2.
#[test]
fn tc235_m01_literal_k2_is_admitted_as_declared() {
    let document = k2();
    let bytes = serde_json::to_vec(&document).expect("K2 serializes");
    let offered = package_input([bytes.as_slice()]);
    let digest = *offered.keys().next().expect("one offered package");
    let selection = DomainPackageRef {
        identity: PACKAGE.to_owned(),
        version: "1".to_owned(),
        digest,
    };
    let (selection, admitted) = admit(
        &selection,
        SHA256_JCS_DIGEST_DOMAIN,
        &offered,
        IntakeLimits::default(),
    )
    .expect("S1 admits the authored package bytes");
    let package = read_domain_package(selection, &admitted)
        .expect("TC-235 K2 must reach the semantic model reader");
    assert_eq!(package.records.len(), 21, "K2's declared source count");
    let (outcome, meter) = normalize_with_meter(&package, ModelNormalizationLimits::default());
    let NormalizeOutcome::Completed(view) = outcome else {
        panic!("M01 K2 normalization must admit, got {outcome:?}");
    };
    assert_eq!(
        meter
            .admitted_charges()
            .iter()
            .filter(|charge| **charge == ChargePoint::NormalizeRecord)
            .count(),
        21,
        "M01 charges normalize.record once per original declaration",
    );
    assert!(
        view.type_identities()
            .contains_key(&qsl_semantics::model::key::DeclarationKey {
                package: PACKAGE.to_owned(),
                node: id("Money"),
            })
    );
    assert!(
        !view
            .type_identities()
            .contains_key(&qsl_semantics::model::key::DeclarationKey {
                package: PACKAGE.to_owned(),
                node: id("OrderManagement"),
            })
    );
}
