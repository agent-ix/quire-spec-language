// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-235 business-meaning vectors at the real QSL intake/normalization boundary.

use qsl_foundation::diagnostic::Code;
use qsl_foundation::{ByteDigest, IntakeLimits};
use qsl_semantics::model::accounting::{ChargePoint, Meter, ModelNormalizationLimits};
use qsl_semantics::model::domain_package::{DomainPackage, DomainPackageRecord, DomainPackageRef};
use qsl_semantics::model::intake::{admit, meaning, package_input, read_domain_package};
use qsl_semantics::model::key::SHA256_JCS_DIGEST_DOMAIN;
use qsl_semantics::model::normalize::{
    normalize_with_meter, ModelRefusal, ModelRefusalCause, NormalizeOutcome,
};
use serde_json::{json, Value};

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
    let mut constructs = vec![
        construct("thing", meaning::OBJECT_TYPE),
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
        "canonicalization": {"algorithm": "rfc8785-v1", "digest": "sha256-jcs"},
        "source": {"identity": id("spec"), "version": "1.0.0", "dialect": "spec-bundle", "digest": digest},
        "package": {"identity": PACKAGE, "version": "1", "manifestDigest": digest,
                    "mappingVersions": [], "profileVersions": [], "lockDigest": digest},
        "constructs": constructs, "types": types, "occurrences": [], "extensions": [],
    })
}

fn mutate_k2(mutate: impl FnOnce(&mut Value)) -> Value {
    let mut document = k2();
    mutate(&mut document);
    let digest = format!(
        "sha256:{}",
        qsl_semantics::model::key::hex(
            &ByteDigest::of(&serde_json::to_vec(&document["types"]).unwrap()).as_bytes()
        )
    );
    document["source"]["digest"] = json!(digest);
    document["package"]["manifestDigest"] = json!(digest);
    document["package"]["lockDigest"] = json!(digest);
    for item in document["constructs"].as_array_mut().unwrap() {
        item["manifestDigest"] = json!(digest);
    }
    document
}

fn type_named_mut<'a>(document: &'a mut Value, node: &str) -> &'a mut Value {
    document["types"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|ty| ty["identity"] == id(node))
        .expect("K2 type exists")
}

fn read_k2(document: &Value) -> Result<DomainPackage, Vec<ModelRefusal>> {
    let bytes = serde_json::to_vec(document).expect("K2 serializes");
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
    read_domain_package(selection, &admitted)
}

fn assert_record_charge_count(meter: &Meter, expected: usize) {
    assert_eq!(
        meter
            .admitted_charges()
            .iter()
            .filter(|charge| **charge == ChargePoint::NormalizeRecord)
            .count(),
        expected,
        "TC-235 normalize.record charge count",
    );
}

fn record_charges(
    document: &DomainPackage,
    expected: usize,
) -> qsl_semantics::model::normalize::EffectiveView {
    let (outcome, meter) = normalize_with_meter(document, ModelNormalizationLimits::default());
    let NormalizeOutcome::Completed(view) = outcome else {
        panic!("normalization must complete, got {outcome:?}");
    };
    assert_record_charge_count(&meter, expected);
    view
}

/// M01: literal TC-235 K2 must admit as 21 declarations and normalize with
/// exactly 21 `normalize.record` charges. Instant uses FCD's native Timestamp
/// alias form, Money uses the canonical native Integer name, and the state
/// transition guard names the declared clauseId.
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
    assert!(view
        .type_identities()
        .contains_key(&qsl_semantics::model::key::DeclarationKey {
            package: PACKAGE.to_owned(),
            node: id("Money"),
        }));
    assert!(!view
        .type_identities()
        .contains_key(&qsl_semantics::model::key::DeclarationKey {
            package: PACKAGE.to_owned(),
            node: id("OrderManagement"),
        }));
}

/// M03: record-value meanings reject identity and operations, while retaining
/// both native `quire` invariants and foreign-language clauses as originals.
#[test]
fn tc235_m03_record_shape_and_clause_meanings() {
    let identity_document = mutate_k2(|document| {
        type_named_mut(document, "Money")["identityFields"] = json!([id("Money/currency")]);
    });
    let package = read_k2(&identity_document).expect("reader retains the identity field");
    let (outcome, meter) = normalize_with_meter(&package, ModelNormalizationLimits::default());
    assert_record_charge_count(&meter, 21);
    let NormalizeOutcome::Refused(refusals) = outcome else {
        panic!("M03 run 1 must refuse during normalization, got {outcome:?}");
    };
    assert_eq!(refusals.len(), 1);
    assert_eq!(refusals[0].code, Code::InvalidModelBinding);
    let ModelRefusalCause::IntakeMalformedDeclaration { node, .. } = &refusals[0].cause else {
        panic!("M03 run 1 cause differs: {:?}", refusals[0].cause);
    };
    assert_eq!(node, &id("Money"));
    assert!(refusals[0].detail.contains("identityFields"));

    let operation_document = mutate_k2(|document| {
        type_named_mut(document, "Money")["operations"] = json!([operation("Money", "add")]);
    });
    let package = read_k2(&operation_document).expect("reader retains the operation");
    let (outcome, meter) = normalize_with_meter(&package, ModelNormalizationLimits::default());
    assert_record_charge_count(&meter, 22);
    let NormalizeOutcome::Refused(refusals) = outcome else {
        panic!("M03 run 2 must refuse during normalization, got {outcome:?}");
    };
    assert_eq!(refusals.len(), 1);
    assert_eq!(refusals[0].code, Code::UnsupportedConstruct);
    let ModelRefusalCause::UnsupportedDeclarationForm { node, what } = &refusals[0].cause else {
        panic!(
            "M03 run 2 must be declaration-form, got {:?}",
            refusals[0].cause
        );
    };
    assert_eq!(node, &id("Money/add"));
    assert_eq!(what, &format!("{}:operations", meaning::RECORD_VALUE_TYPE));
    assert!(refusals[0].detail.contains("operations[0]"));

    let invariant_document = mutate_k2(|document| {
        type_named_mut(document, "Money")["clauses"] =
            json!([clause("Money", "NonNegative", "self.amount_minor >= 0")]);
    });
    let package = read_k2(&invariant_document).expect("quire clause is admitted");
    let invariant_key = qsl_semantics::model::key::DeclarationKey {
        package: PACKAGE.to_owned(),
        node: id("Money/NonNegative"),
    };
    assert!(package.original(&invariant_key).is_some());
    let view = record_charges(&package, 22);
    let owner = view.type_identities()[&qsl_semantics::model::key::DeclarationKey {
        package: PACKAGE.to_owned(),
        node: id("Money"),
    }];
    assert!(view.declarations().iter().any(|declaration| {
        declaration.visible
            && declaration.preimage.original == invariant_key
            && declaration.preimage.owner_effective_type == Some(owner)
    }));

    let foreign_document = mutate_k2(|document| {
        let mut foreign = clause("Money", "NonNegative", "self.amount_minor >= 0");
        foreign["language"] = json!("ocl");
        type_named_mut(document, "Money")["clauses"] = json!([foreign]);
    });
    let package = read_k2(&foreign_document).expect("foreign-language clause is retained");
    let foreign_key = qsl_semantics::model::key::DeclarationKey {
        package: PACKAGE.to_owned(),
        node: id("Money/NonNegative"),
    };
    let DomainPackageRecord::Clause(foreign) = package
        .records
        .iter()
        .find(|record| record.key() == &foreign_key)
        .expect("foreign clause remains a typed original")
    else {
        panic!("M03 run 4 must retain a clause record");
    };
    assert_eq!(foreign.language, "ocl");
    assert_eq!(foreign.text, "self.amount_minor >= 0");
    let view = record_charges(&package, 22);
    assert!(!view
        .declarations()
        .iter()
        .any(|declaration| declaration.preimage.original == foreign_key));
}
