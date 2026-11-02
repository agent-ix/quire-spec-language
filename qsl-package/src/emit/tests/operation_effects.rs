// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1286 (FR-496-AC-1 to AC-5): the optional FR-322 `operation_effects`
//! member carries one row per operation of every `object_type`, `process`
//! and `state_machine` node of the selected documents, each row equal to the
//! operation's frame lift, sorted by `(document, declaring, operation)`.
//! Every package goes through the real compile and emission and is admitted
//! by IR's strict reader, whose typed rows the tests read. The packages are
//! the shapes of QSpec's `operation-effect-vectors.json` cases OEC-01 to
//! OEC-07, written here from the case descriptions.

use quire_contract_model::{
    CheckedFieldScope, CheckedFrameModification, CheckedNodeId, CheckedNodeOwner,
    CheckedOperationEffect, CheckedOperationEffectOrigin, CheckedOperationEffectProvenance,
    CheckedPackageV2, CheckedScopedDeletion,
};

use super::*;

#[path = "../../../../qsl-semantics/tests/support/receiver_frames.rs"]
#[allow(
    clippy::duplicate_mod,
    reason = "a sibling test module also loads the receiver-frame fixture"
)]
mod fixture;

#[path = "../../../../qsl-semantics/tests/support/tc235_k2.rs"]
#[allow(dead_code, reason = "this test uses part of the shared K2 builder")]
#[allow(
    clippy::duplicate_mod,
    reason = "a sibling test module also loads the K2 builder"
)]
mod k2;

type Row = (
    String,
    CheckedNodeId,
    String,
    CheckedOperationEffect,
    CheckedOperationEffectProvenance,
);

/// A package emitted from one model document and the unit naming its
/// operations.
struct Emitted {
    wire: Value,
    admitted: CheckedPackageV2,
    evidence: CheckedPackageEvidence,
}

fn unit(clauses: &str) -> String {
    format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n\
         model M = {package:?};\n\
         {clauses}",
        package = fixture::PACKAGE,
    )
}

fn compile(document: &Value, clauses: &str) -> (CheckedPackage, Vec<u8>) {
    let bytes = serde_json::to_vec(document).expect("the fixture serializes");
    let packages = qsl_semantics::model::intake::package_input([bytes.as_slice()]).unwrap();
    let graph = assemble_with_models(unit(clauses).as_bytes(), &packages)
        .expect("the unit assembles")
        .check(CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("the package checks: {refusals:?}"));
    (CheckedPackage::link(graph), bytes)
}

fn admit(emission: &Emission, document: &[u8]) -> Emitted {
    let mut evidence = read_evidence(emission);
    evidence
        .insert_domain_package_document(package_identity(document), document)
        .expect("the selected document is retained as evidence");
    let admitted = match quire_contract_model::read_checked_package(
        emission.package.bytes(),
        quire_contract_model::CheckedPackageReadLimits::bounded(),
        &evidence,
    ) {
        quire_contract_model::CheckedPackageDispatchResult::AdmittedV2(admitted) => admitted,
        other => panic!("IR admits the emitted package: {other:?}"),
    };
    Emitted {
        wire: wire(emission),
        admitted: *admitted,
        evidence,
    }
}

fn emit(document: &Value, clauses: &str) -> Emitted {
    let (checked, bytes) = compile(document, clauses);
    let emission = emit_checked(&checked).expect("the package emits");
    assert_eq!(emission.omitted, []);
    admit(&emission, &bytes)
}

impl Emitted {
    fn verified(&self) -> bool {
        matches!(
            read_v2(
                &jcs(&self.wire),
                library(),
                V2ReadLimits::default(),
                &self.evidence,
            ),
            Read::Verified { .. }
        )
    }

    /// The structural id of the model declaration `name`.
    fn declaration(&self, name: &str) -> CheckedNodeId {
        let found: Vec<_> = self
            .admitted
            .graph()
            .nodes
            .iter()
            .filter(|node| {
                matches!(
                    &node.owner,
                    Some(CheckedNodeOwner::Model { identity, node })
                        if identity.as_ref() == fixture::PACKAGE
                            && node.as_ref() == fixture::id(name)
                )
            })
            .collect();
        let [node] = found[..] else {
            panic!("{name} is one model node of the package: {found:?}");
        };
        node.node_id.clone()
    }

    /// The admitted `operation_effects` rows, `None` when the member is
    /// absent.
    fn rows(&self) -> Option<Vec<Row>> {
        self.admitted.operation_effects().map(|rows| {
            rows.map(|row| {
                (
                    row.document().to_owned(),
                    row.declaring().clone(),
                    row.operation().to_owned(),
                    row.effect().clone(),
                    row.provenance().clone(),
                )
            })
            .collect()
        })
    }
}

fn effect(
    modifies: Vec<CheckedFrameModification>,
    creates: Vec<CheckedNodeId>,
    deletes: Vec<CheckedNodeId>,
    deletes_scoped: Vec<CheckedScopedDeletion>,
) -> CheckedOperationEffect {
    CheckedOperationEffect {
        modifies,
        creates,
        deletes,
        deletes_scoped,
    }
}

fn field(
    declaration: &CheckedNodeId,
    name: &str,
    scope: CheckedFieldScope,
) -> CheckedFrameModification {
    CheckedFrameModification::Field {
        declaration: declaration.clone(),
        name: name.into(),
        scope,
    }
}

fn parameter(name: &str) -> CheckedScopedDeletion {
    CheckedScopedDeletion::Parameter { name: name.into() }
}

fn provenance(owner: &str, operation: &str) -> CheckedOperationEffectProvenance {
    CheckedOperationEffectProvenance {
        origin: CheckedOperationEffectOrigin::ModelDocumentOperationFrame,
        member: fixture::id(&format!("{owner}/{operation}")).into(),
    }
}

/// The relationship identity `Counter` declares and `link` modifies.
fn relationship() -> String {
    fixture::id("relationship/Counter-supersedes-Counter")
}

/// The Counter fixture with the extra operations `link` (modifies the
/// relationship) and `spawn` (creates a `Gauge`), and `Counter` declaring the
/// relationship.
fn document(operations: &[&str], reverse: bool) -> Value {
    let overrides = vec![
        (
            "link",
            json!({"modifies": [relationship()], "creates": [], "deletes": []}),
        ),
        (
            "spawn",
            json!({"modifies": [], "creates": [fixture::id("Gauge")], "deletes": []}),
        ),
    ];
    let mut document = fixture::document(operations, &overrides);
    let one = json!({"lower": 1, "upper": 1, "ordered": false, "unique": true});
    let types = document["types"].as_array_mut().unwrap();
    types[0]["relationships"] = json!([{
        "identity": relationship(),
        "sourceEnd": {"type": fixture::id("Counter"), "role": "newer", "multiplicity": one},
        "targetEnd": {"type": fixture::id("Counter"), "multiplicity": one},
        "category": "structural",
        "composite": false,
        "direction": "source-to-target",
        "origin": {"source": {
            "sourceIdentity": fixture::id("spec"), "path": "spec.qspec",
            "startLine": 1, "startColumn": 1,
        }},
    }]);
    if reverse {
        types.reverse();
        let operations = types
            .iter_mut()
            .find(|declared| declared["identity"] == fixture::id("Counter"))
            .unwrap()["operations"]
            .as_array_mut()
            .unwrap();
        operations.reverse();
        for operation in operations {
            for member in ["modifies", "modifies_receiver", "deletes_parameters"] {
                if let Some(entries) = operation["frame"]
                    .get_mut(member)
                    .and_then(Value::as_array_mut)
                {
                    entries.reverse();
                }
            }
        }
    }
    document
}

const OPERATIONS: [&str; 9] = [
    "bump", "merge", "noop", "own", "all", "drop", "dispose", "link", "spawn",
];

/// The rows `OPERATIONS` of `Counter` must emit, in key order.
fn counter_rows(emitted: &Emitted) -> Vec<Row> {
    let counter = emitted.declaration("Counter");
    let gauge = emitted.declaration("Gauge");
    let relationship = emitted.declaration("relationship/Counter-supersedes-Counter");
    let receiver = CheckedScopedDeletion::Receiver;
    let mut rows = vec![
        (
            "all",
            effect(
                vec![field(&counter, "value", CheckedFieldScope::EveryObject)],
                vec![],
                vec![],
                vec![],
            ),
        ),
        (
            "bump",
            effect(
                vec![
                    field(&counter, "step", CheckedFieldScope::Receiver),
                    field(&counter, "value", CheckedFieldScope::EveryObject),
                ],
                vec![],
                vec![],
                vec![receiver.clone()],
            ),
        ),
        (
            "dispose",
            effect(vec![], vec![], vec![gauge.clone()], vec![receiver.clone()]),
        ),
        (
            "drop",
            effect(vec![], vec![], vec![], vec![receiver.clone()]),
        ),
        (
            "link",
            effect(
                vec![CheckedFrameModification::Relationship {
                    declaration: relationship,
                }],
                vec![],
                vec![],
                vec![],
            ),
        ),
        (
            "merge",
            effect(
                vec![],
                vec![],
                vec![counter.clone()],
                vec![receiver, parameter("a"), parameter("b")],
            ),
        ),
        ("noop", effect(vec![], vec![], vec![], vec![])),
        (
            "own",
            effect(
                vec![field(&counter, "value", CheckedFieldScope::Receiver)],
                vec![],
                vec![],
                vec![],
            ),
        ),
        ("spawn", effect(vec![], vec![gauge], vec![], vec![])),
    ];
    rows.sort_by(|left, right| left.0.cmp(right.0));
    rows.into_iter()
        .map(|(operation, effect)| {
            (
                fixture::PACKAGE.to_owned(),
                counter.clone(),
                operation.to_owned(),
                effect,
                provenance("Counter", operation),
            )
        })
        .collect()
}

/// FR-496-AC-1, AC-2 (OEC-01 to OEC-07): an object type whose operations
/// modify a field under each scope and a relationship, create and delete an
/// object type and delete the receiver and reference parameters emits one row
/// per operation, each equal to its frame lift member by member; an
/// operation with no frame entry has a row of four empty members.
#[trace("FR-496-AC-1", "FR-496-AC-2", "TC-1286")]
#[test]
fn every_operation_emits_one_row_equal_to_its_frame_lift() {
    let document = document(&OPERATIONS, false);
    let clauses = fixture::posts(&OPERATIONS);
    let emitted = emit(&document, &clauses);
    assert!(emitted.verified());
    assert_eq!(emitted.rows(), Some(counter_rows(&emitted)));
}

/// FR-496-AC-3 (OEC-06, OEC-07): permuting the source declaration order, the
/// member order and the frame entry order of one package emits an equal
/// `operation_effects` member, sorted by `(document, declaring, operation)`
/// with every entry in FR-340 order.
#[trace("FR-496-AC-3", "TC-1286")]
#[test]
fn permuting_source_order_emits_an_equal_member() {
    let straight = emit(&document(&OPERATIONS, false), &fixture::posts(&OPERATIONS));
    let mut reversed_names = OPERATIONS;
    reversed_names.reverse();
    let permuted = emit(
        &document(&reversed_names, true),
        &fixture::posts(&reversed_names),
    );
    let rows = straight.rows().expect("the package carries the member");
    assert_eq!(permuted.rows().as_ref(), Some(&rows));
    assert!(
        rows.windows(2)
            .all(|pair| (&pair[0].0, &pair[0].1, &pair[0].2) < (&pair[1].0, &pair[1].1, &pair[1].2)),
        "rows are strictly ascending by (document, declaring, operation)"
    );
}

/// `frame` as the frame of the operation `owner`/`name` of the k2 document.
fn frame(document: &mut Value, owner: &str, name: &str, frame: Value) {
    let declared = document["types"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|declared| declared["identity"] == k2::id(owner))
        .unwrap();
    let node = format!("{owner}/{name}");
    if declared["operations"].is_null() {
        declared["operations"] = json!([]);
    }
    let operations = declared["operations"].as_array_mut().unwrap();
    match operations
        .iter_mut()
        .find(|operation| operation["identity"] == k2::id(&node))
    {
        Some(operation) => operation["frame"] = frame,
        None => {
            let mut operation = k2::operation(owner, name);
            operation["frame"] = frame;
            operations.push(operation);
        }
    }
}

/// The k2 document without its event type `OrderPlaced` and the members that
/// name it: IR's owner rule gives an event type no `owner`, so a package
/// holding one does not read, whatever its operations.
fn drop_event_type(document: &mut Value) {
    let event = k2::id("OrderPlaced");
    document["constructs"]
        .as_array_mut()
        .unwrap()
        .retain(|construct| construct["kind"]["name"] != "happened");
    let types = document["types"].as_array_mut().unwrap();
    types.retain(|declared| declared["identity"] != event);
    for declared in types {
        match declared["identity"].as_str() {
            Some(identity) if identity == k2::id("OrderLifecycle") => {
                let first = declared["transitions"][0].as_object_mut().unwrap();
                first.insert("emits".into(), json!([]));
                first.remove("guard");
            }
            Some(identity) if identity == k2::id("Fulfilment") => {
                declared["steps"][0]["consumes"] = json!([]);
            }
            Some(identity) if identity == k2::id("OrderManagement") => {
                declared["members"] = json!([k2::id("Order"), k2::id("Money")]);
            }
            _ => {}
        }
    }
}

/// FR-496-AC-4 (TC-1286 step 2): a state machine, a process and an object
/// type each emit rows for the operations they declare; a subtype that only
/// inherits an operation emits none, and no row is declared by a node of
/// another form (a relationship, a persistence interface or a systems
/// interface declares no FR-013 frame).
#[trace("FR-496-AC-4", "TC-1286")]
#[test]
fn state_machines_processes_and_object_types_emit_rows_and_a_subtype_does_not() {
    let mut document = k2::k2();
    drop_event_type(&mut document);
    document["types"]
        .as_array_mut()
        .unwrap()
        .push(k2::declaration(
            "Rush",
            "thing",
            json!({
                "supertypes": [k2::id("Order")],
                "identityFields": [k2::id("Rush/rush_id")],
                "fields": [k2::field("Rush", "rush_id", "ix://quire/native/UUID")],
                "operations": [],
            }),
        ));
    frame(
        &mut document,
        "Order",
        "settle",
        json!({"modifies": [], "creates": [], "deletes": [],
               "modifies_receiver": [k2::id("Order/total")]}),
    );
    frame(
        &mut document,
        "Fulfilment",
        "run",
        json!({"modifies": [k2::id("Fulfilment/correlation_id")],
               "creates": [k2::id("Order")], "deletes": []}),
    );
    frame(
        &mut document,
        "OrderLifecycle",
        "place",
        json!({"modifies": [k2::id("OrderLifecycle/placed")], "creates": [], "deletes": []}),
    );
    let clauses = "\
        function keep using v(r: M::Rush): Boolean pure { true }\n\
        post Settled using v on M::Order::settle { true }\n\
        post Ran using v on M::Fulfilment::run { true }\n\
        post Placed using v on M::OrderLifecycle::place { true }\n\
        post Cancelled using v on M::OrderLifecycle::cancel { true }\n";
    let emitted = emit(&document, clauses);
    assert!(emitted.verified());
    let order = emitted.declaration("Order");
    let fulfilment = emitted.declaration("Fulfilment");
    let lifecycle = emitted.declaration("OrderLifecycle");
    let mut expected: Vec<Row> = [
        (
            &order,
            "Order",
            "settle",
            effect(
                vec![field(&order, "total", CheckedFieldScope::Receiver)],
                vec![],
                vec![],
                vec![],
            ),
        ),
        (
            &fulfilment,
            "Fulfilment",
            "run",
            effect(
                vec![field(
                    &fulfilment,
                    "correlation_id",
                    CheckedFieldScope::EveryObject,
                )],
                vec![order.clone()],
                vec![],
                vec![],
            ),
        ),
        (
            &lifecycle,
            "OrderLifecycle",
            "place",
            effect(
                vec![field(&lifecycle, "placed", CheckedFieldScope::EveryObject)],
                vec![],
                vec![],
                vec![],
            ),
        ),
        (
            &lifecycle,
            "OrderLifecycle",
            "cancel",
            effect(vec![], vec![], vec![], vec![]),
        ),
    ]
    .into_iter()
    .map(|(declaring, owner, operation, effect)| {
        (
            fixture::PACKAGE.to_owned(),
            declaring.clone(),
            operation.to_owned(),
            effect,
            provenance(owner, operation),
        )
    })
    .collect();
    expected
        .sort_by(|left, right| (&left.0, &left.1, &left.2).cmp(&(&right.0, &right.1, &right.2)));
    assert_eq!(emitted.rows(), Some(expected));
    let rush = emitted.declaration("Rush");
    assert!(
        emitted.rows().unwrap().iter().all(|row| row.1 != rush),
        "a subtype that only inherits an operation has no row"
    );
}

/// FR-496-AC-5 (TC-1286 step 4): when an operation's effect names a
/// declaration the package does not carry, the member is omitted whole, never
/// emitted partial, and the remaining package still reads back `Verified`.
#[trace("FR-496-AC-5", "TC-1286")]
#[test]
fn an_unavailable_effect_omits_the_whole_member() {
    let document = document(&OPERATIONS, false);
    let (checked, bytes) = compile(&document, &fixture::posts(&OPERATIONS));
    let cannot_decode_relationship = |tag: CheckedNodeTag, form: &str| {
        !(tag == CheckedNodeTag::Relation && form == "relationship")
            && CheckedNodeKind::decode(tag, form).is_some()
    };
    let emission = emit_package_decoding(
        &checked,
        checked.graph().memoized_regions(),
        &cannot_decode_relationship,
    )
    .expect("the remaining nodes emit");
    assert!(
        !emission.omitted.is_empty(),
        "the relationship and the nodes naming it are omitted"
    );
    let emitted = admit(&emission, &bytes);
    assert!(emitted.wire.get("operation_effects").is_none());
    assert_eq!(emitted.rows(), None);
    assert!(emitted.verified());
    let full = emit(&document, &fixture::posts(&OPERATIONS));
    assert_eq!(
        emitted.declaration("Counter"),
        full.declaration("Counter"),
        "omitting the member rewrites no declaration identity"
    );
}
