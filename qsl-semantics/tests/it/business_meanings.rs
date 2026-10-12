// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSpec FR-208 business meanings through the public Semantic IR intake and
//! normalization boundary. K2 retains TC-235's natives and all declarations;
//! the separately named fixtures isolate rules that need no native alignment.
//! These authored wire inputs do not qualify artifact extraction or runtime.

use ix_trace_rs::trace;
use qsl_foundation::{ByteDigest, IntakeLimits};
use qsl_semantics::model::accounting::{ChargePoint, ModelNormalizationLimits};
use qsl_semantics::model::domain_package::{DomainPackage, DomainPackageRecord};
use qsl_semantics::model::intake::{admit, meaning, package_input, read_domain_package};
use qsl_semantics::model::key::{hex, DeclarationKey, SHA256_JCS_DIGEST_DOMAIN};
use qsl_semantics::model::normalize::{normalize_with_meter, EffectiveView, NormalizeOutcome};
use serde_json::{json, Value};

const BUSINESS_MODULE: &str = "test/business";

fn identity(package: &str, node: &str) -> String {
    format!("ix://{package}/{node}")
}

fn key(package: &str, node: &str) -> DeclarationKey {
    DeclarationKey {
        package: package.to_owned(),
        node: identity(package, node),
    }
}

fn origin(package: &str, node: &str) -> Value {
    let identity = identity(package, node);
    json!({"generated": {
        "generatorIdentity": identity,
        "generatorVersion": "1.0.0",
        "inputIdentities": [identity],
    }})
}

fn construct(name: &str, meaning: &str) -> Value {
    let members = match meaning {
        meaning::OBJECT_TYPE => json!({"identityFields": "required"}),
        meaning::RECORD_VALUE_TYPE => json!({"identityFields": "optional"}),
        meaning::EVENT_TYPE => json!({
            "identityFields": "optional", "occurrenceField": "required",
        }),
        meaning::STATE_MACHINE => json!({"states": "required", "transitions": "required"}),
        meaning::PROCESS => json!({"identityFields": "required", "steps": "required"}),
        meaning::PERSISTENCE_INTERFACE => json!({"persists": "required"}),
        meaning::NAMESPACE => json!({"members": "required", "vocabulary": "required"}),
        _ => json!({}),
    };
    json!({
        "kind": {"module": BUSINESS_MODULE, "name": name},
        "moduleVersion": "1.0.0",
        "construct": {
            "identity": "none", "shape": "record", "members": members,
            "meaning": meaning,
        },
    })
}

fn definition(package: &str, name: &str, kind: &str, members: Value) -> Value {
    let mut definition = json!({
        "identity": identity(package, name), "displayName": name,
        "kind": {"module": BUSINESS_MODULE, "name": kind},
        "roles": [], "origin": origin(package, name),
        "constraints": [], "extensions": [], "unknownPolicy": "reject",
    });
    definition
        .as_object_mut()
        .expect("definition is an object")
        .extend(members.as_object().expect("members are an object").clone());
    definition
}

fn field(package: &str, owner: &str, name: &str, type_ref: &str) -> Value {
    json!({
        "identity": identity(package, &format!("{owner}/{name}")), "name": name,
        "typeRef": type_ref, "presence": "required", "nullable": false,
        "defaultKind": "none",
        "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
        "origin": origin(package, &format!("{owner}/{name}")),
    })
}

fn operation(package: &str, owner: &str, name: &str) -> Value {
    json!({
        "identity": identity(package, &format!("{owner}/{name}")), "name": name,
        "params": [], "pre": [], "post": [],
        "frame": {"modifies": [], "creates": [], "deletes": []},
        "origin": origin(package, &format!("{owner}/{name}")),
    })
}

fn clause(package: &str, owner: &str, name: &str, text: &str) -> Value {
    json!({
        "identity": identity(package, &format!("{owner}/{name}")),
        "language": "quire", "clauseId": name, "text": text,
        "origin": origin(package, &format!("{owner}/{name}")),
    })
}

fn document(package: &str, version: &str, mut constructs: Vec<Value>, types: Vec<Value>) -> Value {
    // The wire requires content digests even for generated test inputs. Bind
    // them to these authored declarations; do not use a fixed digest catalog.
    let source = serde_json::to_vec(&types).expect("authored declarations serialize");
    let digest = format!("sha256:{}", hex(&ByteDigest::of(&source).as_bytes()));
    for construct in &mut constructs {
        construct["manifestDigest"] = json!(digest);
    }
    json!({
        "contractVersion": "2.0.0",
        "canonicalization": {"algorithm": "rfc8785-v1", "digest": "sha256-jcs"},
        "source": {
            "identity": identity(package, "spec"), "version": "1.0.0",
            "dialect": "spec-bundle", "digest": digest,
        },
        "package": {
            "identity": package, "version": version, "manifestDigest": digest,
            "mappingVersions": [], "profileVersions": [], "lockDigest": digest,
        },
        "constructs": constructs, "types": types, "occurrences": [], "extensions": [],
    })
}

fn read_authored(document: &Value) -> DomainPackage {
    let bytes = serde_json::to_vec(document).expect("authored document serializes");
    let packages = package_input([bytes.as_slice()]);
    let digest = *packages.keys().next().expect("one authored package");
    let selection = qsl_semantics::model::domain_package::DomainPackageRef {
        identity: document["package"]["identity"]
            .as_str()
            .expect("package identity")
            .to_owned(),
        version: document["package"]["version"]
            .as_str()
            .expect("package version")
            .to_owned(),
        digest,
    };
    let (selection, admitted) = admit(
        &selection,
        SHA256_JCS_DIGEST_DOMAIN,
        &packages,
        IntakeLimits::default(),
    )
    .expect("the selected authored package passes byte admission");
    read_domain_package(selection, &admitted)
        .expect("business declarations must be admitted through the public reader")
}

fn normalized(package: &DomainPackage, declarations: usize) -> EffectiveView {
    let (outcome, meter) = normalize_with_meter(package, ModelNormalizationLimits::UNLIMITED);
    let NormalizeOutcome::Completed(view) = outcome else {
        panic!("business declarations must normalize: {outcome:?}");
    };
    assert_eq!(
        meter
            .admitted_charges()
            .iter()
            .filter(|point| **point == ChargePoint::NormalizeRecord)
            .count(),
        declarations,
        "one normalize.record charge per original declaration",
    );
    view
}

fn original_keys(package: &DomainPackage) -> Vec<DeclarationKey> {
    let mut keys: Vec<_> = package
        .records
        .iter()
        .map(|record| record.key().clone())
        .collect();
    keys.sort();
    keys
}

fn expected_keys(package: &str, nodes: &[&str]) -> Vec<DeclarationKey> {
    let mut keys: Vec<_> = nodes.iter().map(|node| key(package, node)).collect();
    keys.sort();
    keys
}

fn k2() -> Value {
    let p = "test/orders";
    let state = |name: &str| {
        json!({
            "identity": identity(p, &format!("OrderLifecycle/{name}")), "name": name,
            "origin": origin(p, &format!("OrderLifecycle/{name}")),
        })
    };
    let transition = |name: &str, from: &str, to: &str, trigger: &str, emits: Vec<String>| {
        json!({
            "identity": identity(p, &format!("OrderLifecycle/{name}")),
            "from": identity(p, &format!("OrderLifecycle/{from}")),
            "to": identity(p, &format!("OrderLifecycle/{to}")),
            "trigger": identity(p, &format!("OrderLifecycle/{trigger}")),
            "emits": emits, "origin": origin(p, &format!("OrderLifecycle/{name}")),
        })
    };
    let step = |name: &str, kind: &str, consumes: Vec<String>| {
        json!({
            "identity": identity(p, &format!("Fulfilment/{name}")), "name": name,
            "stepKind": kind, "consumes": consumes, "emits": [],
            "origin": origin(p, &format!("Fulfilment/{name}")),
        })
    };
    let mut placed = transition(
        "transition0",
        "Draft",
        "Placed",
        "place",
        vec![identity(p, "OrderPlaced")],
    );
    placed["guard"] = json!(identity(p, "OrderLifecycle/NotYetPlaced"));
    document(
        p,
        "1",
        vec![
            construct("thing", meaning::OBJECT_TYPE),
            construct("value", meaning::VALUE_TYPE),
            construct("money", meaning::RECORD_VALUE_TYPE),
            construct("happened", meaning::EVENT_TYPE),
            construct("lifecycle", meaning::STATE_MACHINE),
            construct("saga", meaning::PROCESS),
            construct("store", meaning::PERSISTENCE_INTERFACE),
            construct("context", meaning::NAMESPACE),
            construct("population", meaning::POPULATION),
        ],
        vec![
            definition(
                p,
                "Order",
                "thing",
                json!({
                    "identityFields": [identity(p, "Order/order_id")],
                    "fields": [field(p, "Order", "order_id", "ix://quire/native/UUID"),
                               field(p, "Order", "total", &identity(p, "Money"))],
                }),
            ),
            // The IR scalar tag datetime retains the declared native Timestamp
            // binding; its wire spelling is distinct from the native type name.
            definition(p, "Instant", "value", json!({"scalar": "datetime"})),
            definition(
                p,
                "Money",
                "money",
                json!({
                    "fields": [field(p, "Money", "amount_minor", "ix://quire/native/Int"),
                               field(p, "Money", "currency", "ix://quire/native/String")],
                }),
            ),
            definition(
                p,
                "OrderPlaced",
                "happened",
                json!({
                    "occurrenceField": identity(p, "OrderPlaced/occurred_at"),
                    "fields": [field(p, "OrderPlaced", "occurred_at", &identity(p, "Instant")),
                               field(p, "OrderPlaced", "order_id", "ix://quire/native/UUID")],
                }),
            ),
            definition(
                p,
                "OrderLifecycle",
                "lifecycle",
                json!({
                    "fields": [field(p, "OrderLifecycle", "placed", "ix://quire/native/Boolean")],
                    "operations": [operation(p, "OrderLifecycle", "place"), operation(p, "OrderLifecycle", "cancel")],
                    "clauses": [clause(p, "OrderLifecycle", "NotYetPlaced", "not self.placed")],
                    "states": [state("Draft"), state("Placed"), state("Cancelled")],
                    "transitions": [placed, transition("transition1", "Draft", "Cancelled", "cancel", vec![]),
                                    transition("transition2", "Placed", "Cancelled", "cancel", vec![])],
                }),
            ),
            definition(
                p,
                "Fulfilment",
                "saga",
                json!({
                    "identityFields": [identity(p, "Fulfilment/correlation_id")],
                    "fields": [field(p, "Fulfilment", "correlation_id", "ix://quire/native/UUID")],
                    "steps": [step("receive_order", "event", vec![identity(p, "OrderPlaced")]),
                              step("reserve_stock", "command", vec![]), step("await_shipment", "wait", vec![])],
                }),
            ),
            definition(
                p,
                "OrderStore",
                "store",
                json!({
                    "fields": [], "operations": [operation(p, "OrderStore", "get"), operation(p, "OrderStore", "save")],
                    "persists": [identity(p, "Order")],
                }),
            ),
            definition(
                p,
                "OrderManagement",
                "context",
                json!({
                    "members": [identity(p, "Order"), identity(p, "Money"), identity(p, "OrderPlaced")],
                    "vocabulary": [{"term": "Place", "doc": "Commit a draft order", "origin": origin(p, "OrderManagement")}],
                }),
            ),
        ],
    )
}

/// The full authored K2 must reach its 21 original declarations and charges.
/// Trace: QSpec-FR-208-AC-4, QSpec-FR-154-AC-1
#[trace("QSpec-TC-235")]
#[test]
fn full_k2_admission_preserves_twenty_one_declarations() {
    let package = read_authored(&k2());
    assert_eq!(
        original_keys(&package),
        expected_keys(
            "test/orders",
            &[
                "Order",
                "Order/order_id",
                "Order/total",
                "Instant",
                "Money",
                "Money/amount_minor",
                "Money/currency",
                "OrderPlaced",
                "OrderPlaced/occurred_at",
                "OrderPlaced/order_id",
                "OrderLifecycle",
                "OrderLifecycle/placed",
                "OrderLifecycle/place",
                "OrderLifecycle/cancel",
                "OrderLifecycle/NotYetPlaced",
                "Fulfilment",
                "Fulfilment/correlation_id",
                "OrderStore",
                "OrderStore/get",
                "OrderStore/save",
                "OrderManagement",
            ]
        )
    );
    assert!(
        package.records.iter().any(|record| matches!(record,
        DomainPackageRecord::RecordValueType(record) if record.key == key("test/orders", "Money")))
    );
    let view = normalized(&package, 21);
    assert!(!view
        .type_identities()
        .contains_key(&key("test/orders", "OrderManagement")));
}

/// A record invariant is one original declaration, independent of native alignment.
/// Trace: QSpec-FR-208-AC-10, QSpec-FR-154-AC-1
#[trace("QSpec-TC-235")]
#[test]
fn record_invariant_is_preserved_as_a_charged_declaration() {
    let p = "test/record-invariant";
    let package = read_authored(&document(
        p,
        "1.0.0",
        vec![construct("value_record", meaning::RECORD_VALUE_TYPE)],
        vec![definition(
            p,
            "Amount",
            "value_record",
            json!({
                "fields": [field(p, "Amount", "amount_minor", "ix://quire/native/Integer")],
                "clauses": [clause(p, "Amount", "NonNegative", "self.amount_minor >= 0")],
            }),
        )],
    ));
    assert_eq!(
        original_keys(&package),
        expected_keys(p, &["Amount", "Amount/amount_minor", "Amount/NonNegative"])
    );
    let original = package
        .original(&key(p, "Amount/NonNegative"))
        .expect("the invariant retains original provenance");
    assert_eq!(original.member_name.as_deref(), Some("NonNegative"));
    assert_eq!(original.origin, origin(p, "Amount/NonNegative"));
    normalized(&package, 3);
}

fn object_clause_document(clauses: Vec<Value>) -> Value {
    let p = "test/object-invariant";
    document(
        p,
        "1.0.0",
        vec![construct("entity", meaning::OBJECT_TYPE)],
        vec![definition(
            p,
            "Order",
            "entity",
            json!({
                "identityFields": [identity(p, "Order/id")],
                "fields": [field(p, "Order", "id", "ix://quire/native/Integer")],
                "clauses": clauses,
            }),
        )],
    )
}

fn isolated_owner_document(owner: &str, owner_meaning: &str, clauses: Vec<Value>) -> Value {
    let p = "test/owner-boundary";
    let members = match owner_meaning {
        meaning::PROCESS => json!({
            "identityFields": [identity(p, &format!("{owner}/id"))],
            "fields": [field(p, owner, "id", "ix://quire/native/Integer")],
            "steps": [{"identity": identity(p, &format!("{owner}/work")), "name": "work",
                "stepKind": "command", "consumes": [], "emits": [], "origin": origin(p, &format!("{owner}/work"))}],
            "clauses": clauses,
        }),
        meaning::PERSISTENCE_INTERFACE => json!({
            "persists": [identity(p, "Order")], "operations": [operation(p, owner, "get")], "clauses": clauses,
        }),
        meaning::NAMESPACE => json!({
            "members": [identity(p, "Order")], "vocabulary": [{"term": "Order", "doc": "Order scope", "origin": origin(p, owner)}],
            "clauses": clauses,
        }),
        _ => panic!("unsupported isolated owner fixture"),
    };
    document(
        p,
        "1.0.0",
        vec![
            construct("entity", meaning::OBJECT_TYPE),
            construct("owner", owner_meaning),
        ],
        vec![
            definition(
                p,
                "Order",
                "entity",
                json!({
                    "identityFields": [identity(p, "Order/id")], "fields": [field(p, "Order", "id", "ix://quire/native/Integer")],
                }),
            ),
            definition(p, owner, "owner", members),
        ],
    )
}

/// Typed inputs isolate normalization's owner and signature rules from wire admission.
fn normalization_owner_package() -> DomainPackage {
    use qsl_semantics::model::domain_package::{
        DomainPackageRef, EventTypeRecord, FieldMemberRecord, Multiplicity, NamespaceRecord,
        NativeValueType, ObjectTypeRecord, OperationEffect, OperationMemberRecord,
        PersistenceInterfaceRecord, ProcessRecord, ProcessStepKind, ProcessStepRecord,
        RecordValueTypeRecord, ScalarTypeRecord, ScalarValueDomain, StateMachineRecord,
        StateMachineStateRecord, StateMachineTransitionRecord, ValueTypeRef,
    };
    let p = "test/owner-boundary";
    let native_field = |owner: &str, name: &str, native| {
        DomainPackageRecord::FieldMember(FieldMemberRecord {
            key: key(p, &format!("{owner}/{name}")),
            owner: key(p, owner),
            value_type: ValueTypeRef::Native(native),
            multiplicity: Multiplicity {
                lower: 1,
                upper: Some(1),
                ordered: false,
                unique: true,
            },
            presence: quire_exact::Presence::Required,
            subsets: Vec::new(),
            redefines: None,
        })
    };
    let op = |owner: &str, name: &str| {
        DomainPackageRecord::OperationMember(OperationMemberRecord {
            key: key(p, &format!("{owner}/{name}")),
            owner: key(p, owner),
            parameters: Vec::new(),
            result: None,
            effect: OperationEffect::default(),
            own_postcondition_clauses: Vec::new(),
            has_body: false,
            redefines: None,
        })
    };
    DomainPackage::new(
        DomainPackageRef::fixture(p),
        vec![
            DomainPackageRecord::ObjectType(ObjectTypeRecord {
                key: key(p, "Order"),
                interface_features: None,
                abstract_type: false,
                supertypes: Vec::new(),
            }),
            native_field("Order", "id", NativeValueType::Integer),
            DomainPackageRecord::RecordValueType(RecordValueTypeRecord {
                key: key(p, "Money"),
                identity_fields: Vec::new(),
                supertypes: Vec::new(),
            }),
            native_field("Money", "amount_minor", NativeValueType::Integer),
            DomainPackageRecord::ScalarType(ScalarTypeRecord {
                key: key(p, "Count"),
                domain: ScalarValueDomain::Integer {
                    lower: 0,
                    upper: 100,
                },
            }),
            DomainPackageRecord::EventType(EventTypeRecord {
                key: key(p, "OrderPlaced"),
                identity_fields: Vec::new(),
                occurrence_field: Some(key(p, "OrderPlaced/occurred_at")),
                supertypes: Vec::new(),
            }),
            native_field("OrderPlaced", "occurred_at", NativeValueType::Timestamp),
            DomainPackageRecord::StateMachine(StateMachineRecord {
                key: key(p, "OrderLifecycle"),
                supertypes: Vec::new(),
                states: vec![StateMachineStateRecord {
                    key: key(p, "OrderLifecycle/Ready"),
                    name: "Ready".to_owned(),
                }],
                transitions: vec![StateMachineTransitionRecord {
                    key: key(p, "OrderLifecycle/place"),
                    from: key(p, "OrderLifecycle/Ready"),
                    to: key(p, "OrderLifecycle/Ready"),
                    trigger: key(p, "OrderLifecycle/advance"),
                    guard: None,
                    emits: Vec::new(),
                    path: "/types/0/transitions/0".to_owned(),
                    origin: origin(p, "OrderLifecycle/place"),
                }],
            }),
            op("OrderLifecycle", "advance"),
            DomainPackageRecord::Process(ProcessRecord {
                key: key(p, "Fulfilment"),
                identity_fields: vec![key(p, "Fulfilment/id")],
                identity_fields_declared: true,
                supertypes: Vec::new(),
                steps: vec![ProcessStepRecord {
                    key: key(p, "Fulfilment/work"),
                    name: "work".to_owned(),
                    step_kind: ProcessStepKind::Command,
                    consumes: Vec::new(),
                    emits: Vec::new(),
                }],
            }),
            native_field("Fulfilment", "id", NativeValueType::Integer),
            DomainPackageRecord::PersistenceInterface(PersistenceInterfaceRecord {
                key: key(p, "OrderStore"),
                persists: vec![key(p, "Order")],
            }),
            op("OrderStore", "get"),
            DomainPackageRecord::Namespace(NamespaceRecord {
                key: key(p, "OrderManagement"),
                members: vec![key(p, "Order")],
                vocabulary: Vec::new(),
            }),
        ],
    )
}

/// Object invariants retain authored order and provenance and reach the effective view.
/// Trace: FR-056-AC-1, QSpec-FR-208-AC-10
#[trace("FR-056-AC-1", "QSpec-TC-235")]
#[test]
fn object_clauses_preserve_order_identity_and_effective_declarations() {
    let p = "test/object-invariant";
    let package = read_authored(&object_clause_document(vec![
        clause(p, "Order", "ZPositive", "self.id > 0"),
        clause(p, "Order", "ABounded", "self.id < 100"),
    ]));
    let clauses: Vec<_> = package
        .records
        .iter()
        .filter_map(|record| {
            if let DomainPackageRecord::Clause(clause) = record {
                Some(clause)
            } else {
                None
            }
        })
        .collect();
    assert_eq!(
        clauses.len(),
        2,
        "both authored object clauses must be read"
    );
    for (clause, (name, text)) in clauses
        .iter()
        .zip([("ZPositive", "self.id > 0"), ("ABounded", "self.id < 100")])
    {
        assert_eq!(clause.key, key(p, &format!("Order/{name}")));
        assert_eq!(clause.owner, key(p, "Order"));
        assert_eq!(clause.language, "quire");
        assert_eq!(clause.text, text);
        let original = package
            .original(&clause.key)
            .expect("clause has original provenance");
        assert_eq!(original.member_name.as_deref(), Some(name));
        assert_eq!(original.origin, origin(p, &format!("Order/{name}")));
    }
    let view = normalized(&package, 4);
    let owner = view.type_identities()[&key(p, "Order")];
    for clause in clauses {
        assert_eq!(
            view.declarations()
                .iter()
                .filter(|entry| {
                    entry.visible
                        && entry.preimage.original == clause.key
                        && entry.preimage.owner_effective_type == Some(owner)
                })
                .count(),
            1,
            "each invariant reaches its owner's effective declaration exactly once"
        );
    }
}

/// Object clauses apply the same identity/name check as record clauses.
/// Trace: FR-056-AC-1, QSpec-FR-208-AC-10
#[trace("FR-056-AC-1", "QSpec-TC-235")]
#[test]
fn malformed_object_clause_is_refused_like_a_record_clause() {
    use qsl_foundation::Code;
    use qsl_semantics::model::normalize::ModelRefusalCause;
    let p = "test/object-invariant";
    let mut malformed = clause(p, "Order", "Positive", "self.id > 0");
    malformed["clauseId"] = json!("Different");
    let object = object_clause_document(vec![malformed]);
    let mut record = object.clone();
    record["constructs"][0]["kind"]["name"] = json!("value_record");
    record["constructs"][0]["construct"]["meaning"] = json!(meaning::RECORD_VALUE_TYPE);
    record["constructs"][0]["construct"]["members"]["identityFields"] = json!("optional");
    record["types"][0]["kind"]["name"] = json!("value_record");
    record["types"][0]
        .as_object_mut()
        .unwrap()
        .remove("identityFields");
    let mut expected_cause = None;
    for fixture in [record, object] {
        let refusals = authored_refusals(&fixture);
        assert_eq!(refusals.len(), 1);
        assert_eq!(refusals[0].code, Code::InvalidModelBinding);
        assert!(
            matches!(&refusals[0].cause, ModelRefusalCause::IntakeMalformedDeclaration { node, .. }
            if node == &identity(p, "Order/Positive"))
        );
        if let Some(expected) = &expected_cause {
            assert_eq!(&refusals[0].cause, expected);
        } else {
            expected_cause = Some(refusals[0].cause.clone());
        }
        assert!(refusals[0].detail.contains("clauses[0]"));
        assert!(refusals[0]
            .detail
            .contains("clauseId: differs from its identity's member name"));
    }
}

/// The process reader shares object member reading and retains clauses only once.
/// Trace: QSpec-FR-208-AC-10
#[trace("QSpec-TC-235")]
#[test]
fn process_clause_is_read_and_normalized_exactly_once() {
    let p = "test/owner-boundary";
    let fixture = isolated_owner_document(
        "Fulfilment",
        meaning::PROCESS,
        vec![clause(p, "Fulfilment", "HasCorrelation", "true")],
    );
    let package = read_authored(&fixture);
    let clause_key = key(p, "Fulfilment/HasCorrelation");
    assert_eq!(
        package
            .records
            .iter()
            .filter(|record| record.key() == &clause_key)
            .count(),
        1
    );
    let view = normalized(&package, 5);
    assert_eq!(
        view.declarations()
            .iter()
            .filter(|entry| { entry.visible && entry.preimage.original == clause_key })
            .count(),
        1
    );
}

/// Business containers refuse quire invariants at their authored clause.
/// Trace: QSpec-FR-208-AC-10
#[trace("QSpec-TC-235")]
#[test]
fn business_container_quire_clauses_refuse_and_foreign_clauses_are_retained() {
    use qsl_foundation::Code;
    use qsl_semantics::model::normalize::ModelRefusalCause;
    let p = "test/owner-boundary";
    for (owner, owner_meaning, declarations) in [
        ("OrderStore", meaning::PERSISTENCE_INTERFACE, 5),
        ("OrderManagement", meaning::NAMESPACE, 4),
    ] {
        let mut fixture = isolated_owner_document(
            owner,
            owner_meaning,
            vec![clause(p, owner, "Forbidden", "true")],
        );
        let refusals = authored_refusals(&fixture);
        assert_eq!(refusals.len(), 1);
        assert_eq!(refusals[0].code, Code::UnsupportedConstruct);
        assert!(matches!(
            &refusals[0].cause,
            ModelRefusalCause::UnsupportedDeclarationForm { .. }
        ));
        assert!(refusals[0].detail.contains("clauses[0]"));
        fixture["types"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|node| node["displayName"] == owner)
            .unwrap()["clauses"][0]["language"] = json!("ocl");
        let package = read_authored(&fixture);
        let clause_key = key(p, &format!("{owner}/Forbidden"));
        assert_eq!(
            package.original(&clause_key).unwrap().origin,
            origin(p, &format!("{owner}/Forbidden"))
        );
        let view = normalized(&package, declarations);
        assert!(!view
            .declarations()
            .iter()
            .any(|entry| entry.preimage.original == clause_key));
    }
}

/// Sharing the object record shape does not grant systems interfaces invariant ownership.
#[trace("FR-056-AC-1")]
#[test]
fn systems_interface_clause_owner_remains_refused() {
    use qsl_foundation::Code;
    use qsl_semantics::model::normalize::ModelRefusalCause;
    let p = "test/object-invariant";
    let mut fixture = object_clause_document(vec![clause(p, "Order", "Forbidden", "true")]);
    fixture["constructs"][0]["kind"]["name"] = json!("interface");
    fixture["types"][0]["kind"]["name"] = json!("interface");
    fixture["constructs"][0]["construct"]["meaning"] = json!(meaning::SYSTEMS_INTERFACE);
    fixture["constructs"][0]["construct"]["members"] = json!({"featureOrder": "required"});
    fixture["types"][0]
        .as_object_mut()
        .unwrap()
        .remove("identityFields");
    fixture["types"][0]["featureOrder"] = json!([identity(p, "Order/id")]);
    let refusals = authored_refusals(&fixture);
    assert!(
        refusals.iter().any(|refusal| {
            refusal.code == Code::DanglingReference
                && matches!(&refusal.cause,
            ModelRefusalCause::UnknownOwner { member, owner }
            if member == &key(p, "Order/Forbidden") && owner == &key(p, "Order"))
        }),
        "{refusals:?}"
    );
}

/// Each invariant-bearing business type admits its clause as an effective member.
#[trace("FR-056-AC-1", "QSpec-TC-235")]
#[test]
fn business_type_invariant_owner_matrix_is_admitted() {
    use qsl_semantics::model::domain_package::ClauseRecord;
    let p = "test/owner-boundary";
    for owner in [
        "Order",
        "Money",
        "OrderPlaced",
        "OrderLifecycle",
        "Fulfilment",
    ] {
        let base = normalization_owner_package();
        let clause_key = key(p, &format!("{owner}/Invariant"));
        let mut records = base.records;
        records.push(DomainPackageRecord::Clause(ClauseRecord {
            key: clause_key.clone(),
            owner: key(p, owner),
            language: "quire".to_owned(),
            text: "true".to_owned(),
        }));
        let package = DomainPackage::new(base.model_selection, records);
        let view = normalized(&package, 15);
        assert_eq!(
            view.declarations()
                .iter()
                .filter(|entry| entry.visible && entry.preimage.original == clause_key)
                .count(),
            1
        );
    }
}

/// Signature types exclude business containers, event values, state machines and process runs.
#[trace("FR-103-AC-6", "QSpec-TC-235")]
#[test]
fn operation_signature_type_meaning_matrix() {
    use qsl_foundation::Code;
    use qsl_semantics::model::domain_package::{
        Multiplicity, OperationEffect, OperationMemberRecord, OperationParameterRecord,
        OperationResult, ValueTypeRef,
    };
    use qsl_semantics::model::normalize::ModelRefusalCause;
    let p = "test/owner-boundary";
    let base = normalization_owner_package();
    for (name, allowed) in [
        ("Order", true),
        ("Money", true),
        ("Count", true),
        ("OrderPlaced", false),
        ("OrderLifecycle", false),
        ("Fulfilment", false),
        ("OrderStore", false),
        ("OrderManagement", false),
    ] {
        let operation_key = key(p, "Order/change");
        let parameter_key = key(p, "Order/change/value");
        let value_type = ValueTypeRef::Package(key(p, name));
        let multiplicity = Multiplicity {
            lower: 1,
            upper: Some(1),
            ordered: false,
            unique: true,
        };
        let mut records = base.records.clone();
        records.push(DomainPackageRecord::OperationMember(
            OperationMemberRecord {
                key: operation_key.clone(),
                owner: key(p, "Order"),
                parameters: vec![OperationParameterRecord {
                    key: parameter_key.clone(),
                    value_type: value_type.clone(),
                    multiplicity,
                }],
                result: Some(OperationResult {
                    value_type: value_type.clone(),
                    multiplicity,
                }),
                effect: OperationEffect::default(),
                own_postcondition_clauses: Vec::new(),
                has_body: false,
                redefines: None,
            },
        ));
        let package = DomainPackage::new(base.model_selection.clone(), records);
        let (outcome, _) = normalize_with_meter(&package, ModelNormalizationLimits::UNLIMITED);
        if allowed {
            let NormalizeOutcome::Completed(view) = outcome else {
                panic!("declared signature type {name} must admit: {outcome:?}");
            };
            assert!(view
                .domain_package()
                .records
                .iter()
                .any(|record| record.key() == &operation_key));
        } else {
            let NormalizeOutcome::Refused(refusals) = outcome else {
                panic!("signature type {name} must refuse");
            };
            assert_eq!(refusals.len(), 2, "parameter and result must each refuse");
            for refusal in refusals.iter() {
                assert_eq!(refusal.code, Code::InvalidModelBinding);
                assert_eq!(refusal.cause, ModelRefusalCause::MalformedDeclaration);
                assert!(refusal.detail.contains(&value_type.to_string()));
                assert!(refusal.detail.contains(&operation_key.node));
            }
            for position in [
                format!("parameter {}", parameter_key.node),
                "result".to_owned(),
            ] {
                assert!(refusals
                    .iter()
                    .any(|refusal| { refusal.detail.contains(&position) }));
            }
        }
    }
}

/// A record may specialize its own meaning and inherit every ancestor field.
/// Trace: QSpec-FR-208-AC-12
#[trace("QSpec-TC-235")]
#[test]
fn same_meaning_record_specialization_inherits_effective_fields() {
    let p = "test/record-specialization";
    let package = read_authored(&document(
        p,
        "1.0.0",
        vec![construct("value_record", meaning::RECORD_VALUE_TYPE)],
        vec![
            definition(
                p,
                "Amount",
                "value_record",
                json!({
                    "fields": [field(p, "Amount", "amount_minor", "ix://quire/native/Integer")],
                }),
            ),
            definition(
                p,
                "TaxedAmount",
                "value_record",
                json!({
                    "supertypes": [identity(p, "Amount")],
                    "fields": [field(p, "TaxedAmount", "tax_minor", "ix://quire/native/Integer")],
                }),
            ),
        ],
    ));
    assert_eq!(
        original_keys(&package),
        expected_keys(
            p,
            &[
                "Amount",
                "Amount/amount_minor",
                "TaxedAmount",
                "TaxedAmount/tax_minor"
            ]
        )
    );
    let view = normalized(&package, 4);
    let derived = view
        .type_identities()
        .get(&key(p, "TaxedAmount"))
        .expect("record subtype has an effective type");
    let mut fields: Vec<_> = view
        .declarations()
        .iter()
        .filter(|entry| {
            entry.visible && entry.preimage.owner_effective_type.as_ref() == Some(derived)
        })
        .map(|entry| entry.preimage.original.clone())
        .collect();
    fields.sort();
    assert_eq!(
        fields,
        expected_keys(p, &["Amount/amount_minor", "TaxedAmount/tax_minor"])
    );
}

/// A namespace is an original declaration and contributes no effective type.
/// Trace: QSpec-FR-208-AC-8
#[trace("QSpec-TC-235")]
#[test]
fn namespace_admission_preserves_original_without_an_effective_type() {
    let p = "test/namespace-admission";
    let package = read_authored(&document(
        p,
        "1.0.0",
        vec![
            construct("entity", meaning::OBJECT_TYPE),
            construct("context", meaning::NAMESPACE),
        ],
        vec![
            definition(
                p,
                "Entry",
                "entity",
                json!({
                    "fields": [field(p, "Entry", "id", "ix://quire/native/Integer")],
                    "identityFields": [identity(p, "Entry/id")],
                }),
            ),
            definition(
                p,
                "Scope",
                "context",
                json!({
                    "members": [identity(p, "Entry")],
                    "vocabulary": [{"term": "Entry", "doc": "An identified entry", "origin": origin(p, "Scope")}],
                }),
            ),
        ],
    ));
    assert_eq!(
        original_keys(&package),
        expected_keys(p, &["Entry", "Entry/id", "Scope"])
    );
    let original = package
        .original(&key(p, "Scope"))
        .expect("namespace has one original declaration");
    assert_eq!(original.meaning.as_deref(), Some(meaning::NAMESPACE));
    let DomainPackageRecord::Namespace(namespace) = package
        .records
        .iter()
        .find(|record| record.key() == &key(p, "Scope"))
        .expect("namespace original")
    else {
        panic!("expected namespace payload");
    };
    assert_eq!(namespace.members, expected_keys(p, &["Entry"]));
    assert_eq!(namespace.vocabulary.len(), 1);
    assert_eq!(namespace.vocabulary[0].term, "Entry");
    assert_eq!(namespace.vocabulary[0].doc, "An identified entry");
    assert_eq!(namespace.vocabulary[0].origin, origin(p, "Scope"));
    let view = normalized(&package, 3);
    assert!(!view.type_identities().contains_key(&key(p, "Scope")));
    assert!(view.type_identities().contains_key(&key(p, "Entry")));
}

/// Own and ancestor invariant members share the existing qualification walk;
/// foreign-language clauses stay original-only and records have no universe.
/// Trace: QSpec-FR-208-AC-10, QSpec-FR-208-AC-12
#[trace("QSpec-TC-235")]
#[test]
fn record_subtype_inherits_invariant_without_an_object_universe() {
    let p = "test/record-inheritance";
    let mut foreign = clause(p, "Amount", "External", "self.amount_minor >= 0");
    foreign["language"] = json!("ocl");
    let package = read_authored(&document(
        p,
        "1.0.0",
        vec![construct("value_record", meaning::RECORD_VALUE_TYPE)],
        vec![
            definition(
                p,
                "Amount",
                "value_record",
                json!({
                    "fields": [field(p, "Amount", "amount_minor", "ix://quire/native/Integer")],
                    "clauses": [clause(p, "Amount", "NonNegative", "self.amount_minor >= 0"), foreign],
                }),
            ),
            definition(
                p,
                "SpecialAmount",
                "value_record",
                json!({"supertypes": [identity(p, "Amount")]}),
            ),
        ],
    ));
    assert_eq!(
        original_keys(&package),
        expected_keys(
            p,
            &[
                "Amount",
                "Amount/amount_minor",
                "Amount/NonNegative",
                "Amount/External",
                "SpecialAmount"
            ]
        )
    );
    let DomainPackageRecord::Clause(clause) = package
        .records
        .iter()
        .find(|record| record.key() == &key(p, "Amount/External"))
        .expect("foreign clause is a real original")
    else {
        panic!("expected clause payload");
    };
    assert_eq!(clause.language, "ocl");
    assert_eq!(clause.text, "self.amount_minor >= 0");
    let view = normalized(&package, 5);
    let subtype = view.type_identities()[&key(p, "SpecialAmount")];
    let mut inherited: Vec<_> = view
        .declarations()
        .iter()
        .filter(|entry| entry.visible && entry.preimage.owner_effective_type == Some(subtype))
        .map(|entry| entry.preimage.original.clone())
        .collect();
    inherited.sort();
    assert_eq!(
        inherited,
        expected_keys(p, &["Amount/amount_minor", "Amount/NonNegative"])
    );
    assert!(view.object_universe_of(&key(p, "Amount")).is_none());
    assert!(view.object_universe_of(&key(p, "SpecialAmount")).is_none());
    assert!(view
        .object_universes()
        .all(|universe| universe.root_types.is_empty()));
}

/// FR-154 ends intake on a cycle refusal; FR-208 Generalization / AC-12
/// requires specialization-cycle, not a generic malformed declaration.
#[trace("QSpec-TC-235")]
#[test]
fn record_specialization_cycle_refuses_before_effective_view() {
    let p = "test/record-cycle";
    let refusals = authored_refusals(&document(
        p,
        "1.0.0",
        vec![construct("value_record", meaning::RECORD_VALUE_TYPE)],
        vec![
            definition(
                p,
                "Amount",
                "value_record",
                json!({"supertypes": [identity(p, "TaxedAmount")], "fields": [field(p, "Amount", "amount_minor", "ix://quire/native/Integer")]}),
            ),
            definition(
                p,
                "TaxedAmount",
                "value_record",
                json!({"supertypes": [identity(p, "Amount")], "fields": [field(p, "TaxedAmount", "tax_minor", "ix://quire/native/Integer")]}),
            ),
        ],
    ));
    assert_eq!(
        refusals.len(),
        2,
        "each declaration on the specialization cycle refuses: {refusals:?}"
    );
    for (refusal, (ancestor, via)) in refusals
        .iter()
        .zip([("Amount", "TaxedAmount"), ("TaxedAmount", "Amount")])
    {
        assert_eq!(
            refusal.code,
            qsl_foundation::diagnostic::Code::InvalidModelBinding
        );
        let qsl_semantics::model::normalize::ModelRefusalCause::SpecializationCycle {
            ancestor: actual,
            via: actual_via,
        } = &refusal.cause
        else {
            panic!("expected typed intake cycle cause: {refusal:?}");
        };
        assert_eq!(actual, &key(p, ancestor));
        assert_eq!(actual_via, &key(p, via));
    }
}

fn authored_refusals(document: &Value) -> Vec<qsl_semantics::model::normalize::ModelRefusal> {
    let bytes = serde_json::to_vec(document).expect("authored restriction fixture serializes");
    let packages = package_input([bytes.as_slice()]);
    let digest = *packages.keys().next().expect("one authored package");
    let selection = qsl_semantics::model::domain_package::DomainPackageRef {
        identity: document["package"]["identity"]
            .as_str()
            .expect("package identity")
            .to_owned(),
        version: document["package"]["version"]
            .as_str()
            .expect("package version")
            .to_owned(),
        digest,
    };
    let (selection, admitted) = admit(
        &selection,
        SHA256_JCS_DIGEST_DOMAIN,
        &packages,
        IntakeLimits::default(),
    )
    .expect("byte admission precedes business restriction checks");
    let package = match read_domain_package(selection, &admitted) {
        Ok(package) => package,
        Err(refusals) => return refusals,
    };
    let (outcome, _) = normalize_with_meter(&package, ModelNormalizationLimits::default());
    match outcome {
        NormalizeOutcome::Refused(refusals) => refusals.iter().cloned().collect(),
        NormalizeOutcome::Completed(_) => panic!("invalid business form must refuse"),
        NormalizeOutcome::Incomplete(incomplete) => {
            panic!("unlimited business fixture is incomplete: {incomplete:?}")
        }
    }
}

/// Record restrictions survive admitting their ancestry and clause members.
/// Trace: QSpec-FR-208-AC-4, QSpec-FR-208-AC-12
#[trace("QSpec-TC-235")]
#[test]
fn record_identity_operation_and_wrong_meaning_parent_remain_refused() {
    use qsl_foundation::diagnostic::Code;
    use qsl_semantics::model::normalize::ModelRefusalCause;
    let p = "test/record-restrictions";
    for form in 0..3 {
        let mut amount = definition(
            p,
            "Amount",
            "value_record",
            json!({"fields": [field(p, "Amount", "amount_minor", "ix://quire/native/Integer")]}),
        );
        match form {
            0 => amount["identityFields"] = json!([identity(p, "Amount/amount_minor")]),
            1 => amount["operations"] = json!([operation(p, "Amount", "add")]),
            _ => amount["supertypes"] = json!([identity(p, "Entry")]),
        }
        let fixture = document(
            p,
            "1.0.0",
            vec![
                construct("value_record", meaning::RECORD_VALUE_TYPE),
                construct("entity", meaning::OBJECT_TYPE),
            ],
            vec![
                amount,
                definition(
                    p,
                    "Entry",
                    "entity",
                    json!({"identityFields": [identity(p, "Entry/id")], "fields": [field(p, "Entry", "id", "ix://quire/native/Integer")]}),
                ),
            ],
        );
        let refusals = authored_refusals(&fixture);
        assert_eq!(refusals.len(), 1, "one restriction refusal for form {form}");
        if form == 1 {
            assert_eq!(refusals[0].code, Code::UnsupportedConstruct);
            let ModelRefusalCause::UnsupportedDeclarationForm { node, what } = &refusals[0].cause
            else {
                panic!("expected operation declaration-form refusal");
            };
            assert_eq!(node, &identity(p, "Amount/add"));
            assert_eq!(what, &format!("{}:operations", meaning::RECORD_VALUE_TYPE));
        } else {
            assert_eq!(refusals[0].code, Code::InvalidModelBinding);
            let ModelRefusalCause::IntakeMalformedDeclaration { node, .. } = &refusals[0].cause
            else {
                panic!("expected located malformed record");
            };
            assert_eq!(node, &identity(p, "Amount"));
        }
    }
}

/// Namespace membership is original data and the later namespace owns the
/// duplicate-member refusal, regardless of input-array order.
/// Trace: QSpec-FR-208-AC-8
#[trace("QSpec-TC-235")]
#[test]
fn namespace_later_membership_and_namespace_target_remain_refused() {
    use qsl_semantics::model::normalize::ModelRefusalCause;
    let p = "test/namespace-restrictions";
    let scope = |name: &str, member: &str| {
        definition(
            p,
            name,
            "context",
            json!({
                "members": [identity(p, member)], "vocabulary": [{"term": "Entry", "doc": "An entry", "origin": origin(p, name)}],
            }),
        )
    };
    let entry = definition(
        p,
        "Entry",
        "entity",
        json!({"identityFields": [identity(p, "Entry/id")], "fields": [field(p, "Entry", "id", "ix://quire/native/Integer")]}),
    );
    for namespaces in [
        vec![scope("Zulu", "Entry"), scope("Alpha", "Entry")],
        vec![scope("Zulu", "Alpha"), scope("Alpha", "Entry")],
    ] {
        let mut nodes = vec![entry.clone()];
        nodes.extend(namespaces);
        let refusals = authored_refusals(&document(
            p,
            "1.0.0",
            vec![
                construct("entity", meaning::OBJECT_TYPE),
                construct("context", meaning::NAMESPACE),
            ],
            nodes,
        ));
        assert_eq!(refusals.len(), 1);
        assert_eq!(
            refusals[0].code,
            qsl_foundation::diagnostic::Code::InvalidModelBinding
        );
        let ModelRefusalCause::IntakeMalformedDeclaration { node, .. } = &refusals[0].cause else {
            panic!("expected located namespace refusal");
        };
        assert_eq!(node, &identity(p, "Zulu"));
    }
}

/// Namespace forbidden members do not gain semantics from the new payload.
/// Trace: QSpec-FR-208-AC-8, QSpec-FR-208-AC-12
#[trace("QSpec-TC-235")]
#[test]
fn namespace_field_operation_quire_clause_and_supertypes_remain_refused() {
    use qsl_foundation::diagnostic::Code;
    use qsl_semantics::model::normalize::ModelRefusalCause;
    let p = "test/namespace-forms";
    for form in 0..4 {
        let mut scope = definition(
            p,
            "Scope",
            "context",
            json!({"members": [], "vocabulary": []}),
        );
        match form {
            0 => scope["fields"] = json!([field(p, "Scope", "name", "ix://quire/native/Integer")]),
            1 => scope["operations"] = json!([operation(p, "Scope", "close")]),
            2 => scope["clauses"] = json!([clause(p, "Scope", "Scoped", "true")]),
            _ => scope["supertypes"] = json!([identity(p, "Ghost")]),
        }
        let refusals = authored_refusals(&document(
            p,
            "1.0.0",
            vec![construct("context", meaning::NAMESPACE)],
            vec![scope],
        ));
        assert_eq!(refusals.len(), 1);
        if form < 2 {
            assert_eq!(refusals[0].code, Code::InvalidModelBinding);
            let ModelRefusalCause::IntakeMalformedDeclaration { node, .. } = &refusals[0].cause
            else {
                panic!("expected located malformed namespace member");
            };
            assert_eq!(
                node,
                &identity(
                    p,
                    if form == 0 {
                        "Scope/name"
                    } else {
                        "Scope/close"
                    }
                )
            );
        } else {
            assert_eq!(
                refusals[0].code,
                Code::UnsupportedConstruct,
                "form {form}: {:?}",
                refusals[0]
            );
            let ModelRefusalCause::UnsupportedDeclarationForm { node, what } = &refusals[0].cause
            else {
                panic!("expected namespace declaration-form refusal");
            };
            assert_eq!(
                node,
                &identity(p, if form == 2 { "Scope/Scoped" } else { "Scope" })
            );
            assert_eq!(
                what,
                &format!(
                    "{}:{}",
                    meaning::NAMESPACE,
                    if form == 2 { "clauses" } else { "supertypes" }
                )
            );
        }
    }
}
