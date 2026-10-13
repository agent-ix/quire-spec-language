// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-056-AC-33: a normalize-stage declaration refusal retains the artifact
//! and source position of the refused node's FCD origin. One typed vector per
//! refusal site, over a K2 package whose every node carries a source origin.
//! Assertions compare the typed code, cause, node, member path, artifact and
//! position, never detail text.

use super::*;

use ix_trace_rs::trace;

const ARTIFACT_COLUMN: u64 = 5;

fn spec_artifact() -> String {
    id("spec")
}

/// Gives every node of `value` a source origin at its own line (numbered in
/// document order) and records that line by node identity.
fn stamp_sources(value: &mut Value, lines: &mut BTreeMap<String, u64>) {
    match value {
        Value::Object(map) => {
            if let Some(Value::String(identity)) = map.get("identity").cloned() {
                if map.contains_key("origin") {
                    let line = lines.len() as u64 + 1;
                    lines.insert(identity, line);
                    let source = json!({
                        "sourceIdentity": spec_artifact(),
                        "path": "spec.qspec",
                        "startLine": line,
                        "startColumn": ARTIFACT_COLUMN,
                    });
                    // The IR reader requires a source clause to carry its span.
                    if map.contains_key("clauseId") {
                        map.insert("sourceSpan".to_owned(), source.clone());
                    }
                    map.insert("origin".to_owned(), json!({ "source": source }));
                }
            }
            for child in map.values_mut() {
                stamp_sources(child, lines);
            }
        }
        Value::Array(items) => {
            for child in items {
                stamp_sources(child, lines);
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {}
    }
}

/// The K2 package mutated by `mutate`, every node sourced; the line of each
/// node by identity.
fn sourced(mutate: impl FnOnce(&mut Value)) -> (Value, BTreeMap<String, u64>) {
    let mut lines = BTreeMap::new();
    let document = mutate_k2(|document| {
        mutate(document);
        stamp_sources(document, &mut lines);
    });
    (document, lines)
}

fn refused(document: &Value) -> Vec<ModelRefusal> {
    let package = read_k2(document).expect("the reader retains the declaration for normalization");
    match normalize_with_meter(&package, ModelNormalizationLimits::default()).0 {
        NormalizeOutcome::Refused(refusals) => refusals.iter().cloned().collect(),
        other => panic!("the vector must refuse, got {other:?}"),
    }
}

/// `refusal` is a `malformed-declaration` at `node`/`member` that retains the
/// node's source artifact and position.
fn assert_retains_origin(
    refusal: &ModelRefusal,
    lines: &BTreeMap<String, u64>,
    node: &str,
    member: &str,
) {
    assert_eq!(refusal.code, Code::InvalidModelBinding);
    let identity = if node.starts_with("ix://") {
        node.to_owned()
    } else {
        id(node)
    };
    let line = *lines
        .get(&identity)
        .unwrap_or_else(|| panic!("{identity} is a sourced node"));
    let ModelRefusalCause::IntakeMalformedDeclaration {
        node: actual,
        artifact,
        span,
        member: actual_member,
    } = &refusal.cause
    else {
        panic!("expected a malformed declaration, got {refusal:?}");
    };
    assert_eq!(actual, &identity);
    assert_eq!(actual_member, &super::member(member));
    assert_eq!(
        artifact.as_deref(),
        Some(spec_artifact().as_str()),
        "{refusal:?}"
    );
    let span = span.expect("the node's source position is retained");
    assert_eq!(
        (span.start.line as u64, span.start.column as u64),
        (line, ARTIFACT_COLUMN)
    );
}

fn only_refusal(
    mutate: impl FnOnce(&mut Value),
    node: &str,
    member: &str,
) -> (ModelRefusal, BTreeMap<String, u64>) {
    let (document, lines) = sourced(mutate);
    let mut found = refused(&document);
    assert_eq!(found.len(), 1, "{found:?}");
    let refusal = found.remove(0);
    assert_retains_origin(&refusal, &lines, node, member);
    (refusal, lines)
}

/// A record value type that declares identity fields.
#[trace("TC-1005", "FR-056-AC-33")]
#[test]
fn a_record_value_type_with_identity_fields_keeps_its_origin() {
    only_refusal(
        |document| {
            type_named_mut(document, "Money")["identityFields"] = json!([id("Money/currency")]);
        },
        "Money",
        "TypeDefinition.identityFields",
    );
}

/// An event type that declares identity fields.
#[trace("TC-1005", "FR-056-AC-33")]
#[test]
fn an_event_type_with_identity_fields_keeps_its_origin() {
    only_refusal(
        |document| {
            type_named_mut(document, "OrderPlaced")["identityFields"] =
                json!([id("OrderPlaced/order_id")]);
        },
        "OrderPlaced",
        "TypeDefinition.identityFields",
    );
}

/// An event type whose occurrence field resolves to a non-Timestamp scalar.
#[trace("TC-1005", "FR-056-AC-33")]
#[test]
fn an_event_occurrence_that_is_not_a_timestamp_keeps_its_origin() {
    only_refusal(
        |document| {
            type_named_mut(document, "Instant")["target"] = json!("ix://quire/native/Integer");
        },
        "OrderPlaced",
        "TypeDefinition.occurrenceField",
    );
}

/// An operation an event type declares is refused at the operation's own node
/// and origin, not the event type's.
#[trace("TC-1005", "FR-056-AC-33")]
#[test]
fn an_operation_declared_by_an_event_type_keeps_its_own_origin() {
    only_refusal(
        |document| {
            type_named_mut(document, "OrderPlaced")["operations"] =
                json!([operation("OrderPlaced", "replay")]);
        },
        "OrderPlaced/replay",
        "TypeDefinition.operations[0]",
    );
}

/// A state machine transition that emits a type of another meaning.
#[trace("TC-1005", "FR-056-AC-33")]
#[test]
fn a_state_machine_transition_emitting_a_non_event_keeps_its_origin() {
    only_refusal(
        |document| {
            type_named_mut(document, "OrderLifecycle")["transitions"][0]["emits"] =
                json!([id("Order")]);
        },
        "OrderLifecycle",
        "TypeDefinition.transitions[0].emits[0]",
    );
}

/// A state machine that declares no states, and one that declares no
/// transitions.
#[trace("TC-1005", "FR-056-AC-33")]
#[test]
fn a_state_machine_missing_required_members_keeps_its_origin() {
    let (document, lines) = sourced(|document| {
        type_named_mut(document, "OrderLifecycle")["transitions"] = json!([]);
    });
    let found = refused(&document);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_retains_origin(
        &found[0],
        &lines,
        "OrderLifecycle",
        "TypeDefinition.transitions",
    );

    let (document, lines) = sourced(|document| {
        type_named_mut(document, "OrderLifecycle")["operations"] = json!([]);
    });
    let found = refused(&document);
    assert_retains_origin(
        &found[0],
        &lines,
        "OrderLifecycle",
        "TypeDefinition.operations",
    );
}

/// A process subtype of a state machine that supplies no identity fields:
/// the identity refusal and then the supertype refusal, each at the process.
#[trace("TC-1005", "FR-056-AC-33")]
#[test]
fn a_process_without_identity_or_process_supertype_keeps_its_origin() {
    let (document, lines) = sourced(|document| {
        document["types"].as_array_mut().unwrap().push(declaration(
            "ExpressFulfilment",
            "saga",
            json!({
                "identityFields": [],
                "supertypes": [id("OrderLifecycle")],
                "operations": [],
                "steps": [{
                    "identity": id("ExpressFulfilment/expedite"),
                    "name": "expedite",
                    "stepKind": "command",
                    "consumes": [],
                    "emits": [],
                    "origin": origin("ExpressFulfilment/expedite"),
                }],
            }),
        ));
    });
    let found = refused(&document);
    assert_eq!(found.len(), 2, "{found:?}");
    assert_retains_origin(
        &found[0],
        &lines,
        "ExpressFulfilment",
        "TypeDefinition.identityFields",
    );
    assert_retains_origin(
        &found[1],
        &lines,
        "ExpressFulfilment",
        "TypeDefinition.supertypes[0]",
    );
}

/// A field typed by a variant type.
#[trace("TC-1005", "FR-056-AC-33")]
#[test]
fn a_field_typed_by_a_variant_type_keeps_its_origin() {
    only_refusal(
        |document| {
            document["constructs"].as_array_mut().unwrap().push(json!({
                "kind": {"module": BUSINESS, "name": "variant_type"},
                "moduleVersion": "1.0.0",
                "construct": {
                    "identity": "none", "shape": "enumeration",
                    "members": {"variants": "required"},
                    "meaning": meaning::VARIANT_TYPE,
                },
            }));
            document["types"].as_array_mut().unwrap().push(declaration(
                "Tier",
                "variant_type",
                json!({"variants": [{
                    "identity": id("variant/Tier-Gold"),
                    "name": "Gold",
                    "origin": origin("variant/Tier-Gold"),
                }]}),
            ));
            type_named_mut(document, "Order")["fields"]
                .as_array_mut()
                .unwrap()
                .push(field("Order", "tier", &id("Tier")));
        },
        "Order/tier",
        "Field.typeRef",
    );
}

fn relationship(owner: &str, name: &str, target: &str) -> Value {
    let node = format!("relationship/{owner}-{name}-{target}");
    json!({
        "identity": id(&node),
        "category": "structural",
        "composite": false,
        "direction": "source-to-target",
        "origin": origin(&node),
        "sourceEnd": {
            "role": name,
            "type": id(owner),
            "multiplicity": {"lower": 0, "ordered": false, "unique": true},
        },
        "targetEnd": {
            "type": id(target),
            "multiplicity": {"lower": 0, "ordered": false, "unique": true},
        },
    })
}

/// A relationship whose source end is not an object type.
#[trace("TC-1005", "FR-056-AC-33")]
#[test]
fn a_relationship_source_of_another_meaning_keeps_its_origin() {
    only_refusal(
        |document| {
            type_named_mut(document, "Money")["relationships"] =
                json!([relationship("Money", "owner", "Order")]);
        },
        "relationship/Money-owner-Order",
        "TypeDefinition.relationships[0]",
    );
}

/// A relationship whose target end is not an object type.
#[trace("TC-1005", "FR-056-AC-33")]
#[test]
fn a_relationship_target_of_another_meaning_keeps_its_origin() {
    only_refusal(
        |document| {
            type_named_mut(document, "Fulfilment")["relationships"] =
                json!([relationship("Fulfilment", "store", "OrderStore")]);
        },
        "relationship/Fulfilment-store-OrderStore",
        "Relationship.target",
    );
}

/// A node whose origin is `generated` has no source: artifact and span are
/// `None`, the one case the field documents.
#[trace("TC-1005", "FR-056-AC-33")]
#[test]
fn a_generated_origin_leaves_artifact_and_span_none() {
    let document = mutate_k2(|document| {
        type_named_mut(document, "Money")["identityFields"] = json!([id("Money/currency")]);
    });
    let found = refused(&document);
    assert_eq!(found.len(), 1, "{found:?}");
    let ModelRefusalCause::IntakeMalformedDeclaration { artifact, span, .. } = &found[0].cause
    else {
        panic!("expected a malformed declaration: {found:?}");
    };
    assert_eq!((artifact, span), (&None, &None));
}

/// TC-235 M10 run 10: a population member naming a declaration of the wrong
/// meaning refuses `malformed-declaration` at the population node, path
/// `PopulationDefinition.members[i]`, keeping the population's origin. A
/// member naming no declaration stays `missing_declaration`.
#[trace("QSpec-TC-235", "TC-1005", "FR-056-AC-33")]
#[test]
fn a_population_member_of_another_meaning_refuses_malformed_with_origin() {
    for wrong in [
        "Money",
        "OrderPlaced",
        "Instant",
        "OrderStore",
        "OrderManagement",
    ] {
        let (document, lines) = sourced(|document| {
            document["populations"] = json!([{
                "identity": id("PO"),
                "displayName": id("PO"),
                "kind": {"module": BUSINESS, "name": "population"},
                "members": [id("Order"), id(wrong)],
                "extent": "closed",
                "origin": origin("PO"),
            }]);
        });
        let found = refused(&document);
        assert_eq!(found.len(), 1, "{wrong}: {found:?}");
        assert_retains_origin(&found[0], &lines, "PO", "PopulationDefinition.members[1]");
    }

    let (document, _) = sourced(|document| {
        document["populations"] = json!([{
            "identity": id("PO"),
            "displayName": id("PO"),
            "kind": {"module": BUSINESS, "name": "population"},
            "members": [id("Missing")],
            "extent": "closed",
            "origin": origin("PO"),
        }]);
    });
    let found = refused(&document);
    assert_eq!(found.len(), 1, "{found:?}");
    assert_eq!(found[0].code, Code::MissingDeclaration);
}
