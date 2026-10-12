// SPDX-License-Identifier: AGPL-3.0-or-later
//! Shared TC-235 K2 fixture builder used by QSL semantics and evaluator tests.

use qsl_foundation::ByteDigest;
use qsl_semantics::model::intake::meaning;
use serde_json::{json, Value};

pub(crate) const PACKAGE: &str = "test/orders";
pub(crate) const BUSINESS: &str = "test/business";

pub(crate) fn id(node: &str) -> String {
    format!("ix://{PACKAGE}/{node}")
}

pub(crate) fn origin(node: &str) -> Value {
    let identity = id(node);
    json!({"generated": {
        "generatorIdentity": identity,
        "generatorVersion": "1.0.0",
        "inputIdentities": [identity],
    }})
}

pub(crate) fn construct(name: &str, semantic_meaning: &str) -> Value {
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

pub(crate) fn declaration(name: &str, kind: &str, members: Value) -> Value {
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

pub(crate) fn field(owner: &str, name: &str, type_ref: &str) -> Value {
    let node = format!("{owner}/{name}");
    json!({
        "identity": id(&node), "name": name, "typeRef": type_ref,
        "presence": "required", "nullable": false, "defaultKind": "none",
        "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
        "origin": origin(&node),
    })
}

pub(crate) fn operation(owner: &str, name: &str) -> Value {
    let node = format!("{owner}/{name}");
    json!({
        "identity": id(&node), "name": name, "params": [], "pre": [], "post": [],
        "frame": {"modifies": [], "creates": [], "deletes": []},
        "origin": origin(&node),
    })
}

pub(crate) fn clause(owner: &str, name: &str, text: &str) -> Value {
    let node = format!("{owner}/{name}");
    json!({
        "identity": id(&node), "language": "quire", "clauseId": name,
        "text": text, "origin": origin(&node),
    })
}

pub(crate) fn jcs_digest(value: &Value) -> String {
    let input = serde_json::to_vec(value).expect("fixture value serializes");
    let document = quire_canonical::read(&input, u64::MAX).expect("fixture value is JSON");
    let canonical = quire_canonical::to_vec(&document, quire_canonical::Limits::new(u64::MAX))
        .expect("fixture value has a JCS encoding");
    format!(
        "sha256:{}",
        qsl_semantics::model::key::hex(&ByteDigest::of(&canonical).as_bytes())
    )
}

pub(crate) fn domain_document(types: Vec<Value>, mut constructs: Vec<Value>) -> Value {
    // Construct manifests identify this QSL-owned fixture's authored types.
    // The selected package digest is independently computed by package_input
    // from the complete bytes through quire-canonical's JCS reader.
    let digest = jcs_digest(&Value::Array(types.clone()));
    for item in &mut constructs {
        item["manifestDigest"] = json!(digest);
    }
    json!({
        "contractVersion": "2.0.0",
        "canonicalization": {"algorithm": "rfc8785-v1", "digest": "sha256-jcs"},
        "source": {"identity": id("spec"), "version": "1.0.0", "dialect": "spec-bundle", "digest": digest},
        "package": {"identity": PACKAGE, "version": "1", "manifestDigest": digest,
                    "mappingVersions": [], "profileVersions": [], "lockDigest": digest},
        "constructs": constructs, "types": types, "occurrences": [], "extensions": [],
    })
}

#[test]
fn qsl_owned_digest_fixture_uses_sha256_jcs() {
    assert_eq!(
        jcs_digest(&json!({"a": 1})),
        "sha256:015abd7f5cc57a2dd94b7590f04ad8084273905ee33ec5cebeae62276a97f862"
    );
}

pub(crate) fn k2() -> Value {
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
    // FCD's transition guard is keyed by the local clauseId, not by the
    // clause's package identity.
    first["guard"] = json!("NotYetPlaced");

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
        {
            let mut instant = declaration("Instant", "value", json!({}));
            instant["kind"] = json!("alias");
            instant["target"] = json!("ix://quire/native/Timestamp");
            instant
        },
        declaration(
            "Money",
            "money",
            json!({
                "fields": [field("Money", "amount_minor", "ix://quire/native/Integer"),
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
    let constructs = vec![
        construct("thing", meaning::OBJECT_TYPE),
        construct("money", meaning::RECORD_VALUE_TYPE),
        construct("happened", meaning::EVENT_TYPE),
        construct("lifecycle", meaning::STATE_MACHINE),
        construct("saga", meaning::PROCESS),
        construct("store", meaning::PERSISTENCE_INTERFACE),
        construct("context", meaning::NAMESPACE),
        construct("population", meaning::POPULATION),
    ];
    domain_document(types, constructs)
}

pub(crate) fn mutate_k2(mutate: impl FnOnce(&mut Value)) -> Value {
    let mut document = k2();
    mutate(&mut document);
    let digest = jcs_digest(&document["types"]);
    document["source"]["digest"] = json!(digest);
    document["package"]["manifestDigest"] = json!(digest);
    document["package"]["lockDigest"] = json!(digest);
    for item in document["constructs"].as_array_mut().unwrap() {
        item["manifestDigest"] = json!(digest);
    }
    document
}

pub(crate) fn type_named_mut<'a>(document: &'a mut Value, node: &str) -> &'a mut Value {
    document["types"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|ty| ty["identity"] == id(node))
        .expect("K2 type exists")
}

#[test]
fn tc235_shared_k2_fixture_preserves_fcd_alias_and_digest_contract() {
    let document = k2();
    assert_eq!(document["types"].as_array().unwrap().len(), 8);
    assert_eq!(document["types"][1]["kind"], "alias");
    assert_eq!(
        document["types"][1]["target"],
        "ix://quire/native/Timestamp"
    );
    let digest = jcs_digest(&document["types"]);
    assert_eq!(document["source"]["digest"], digest);
    assert_eq!(document["package"]["manifestDigest"], digest);
}
