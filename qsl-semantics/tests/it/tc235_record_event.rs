// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-235 record-value and event vectors at the source/normalization boundary.
//!
//! Runtime clause truth and FR-207 path outcomes belong to the downstream
//! evaluator/replay boundary. `qsl-semantics` retains the authored clauses
//! and binds their owners; it does not parse or evaluate their text.

use super::*;

use ix_trace_rs::trace;
use qsl_foundation::SourceIdentity;
use qsl_semantics::check::PackageDeclarations;
use qsl_semantics::model::domain_package::{
    ClauseRecord, DomainPackageRecord, FieldMemberRecord, RecordValueTypeRecord, ValueTypeRef,
};
use qsl_semantics::model::intake::admit_unit;
use qsl_semantics::model::key::{hex, DeclarationKey};
use serde_json::json;

fn key(node: &str) -> DeclarationKey {
    DeclarationKey {
        package: PACKAGE.to_owned(),
        node: id(node),
    }
}

fn relationship_origin(node: &str) -> serde_json::Value {
    json!({
        "source": {
            "sourceIdentity": id(node),
            "path": "tc235_record_event.rs",
            "startLine": 1,
            "startColumn": 1,
            "endLine": 1,
            "endColumn": 1,
        }
    })
}

fn clause_for<'a>(package: &'a DomainPackage, node: &str) -> &'a ClauseRecord {
    package
        .records
        .iter()
        .find_map(|record| match record {
            DomainPackageRecord::Clause(clause) if clause.key == key(node) => Some(clause),
            _ => None,
        })
        .unwrap_or_else(|| panic!("missing clause {node}"))
}

fn field_for<'a>(package: &'a DomainPackage, node: &str) -> &'a FieldMemberRecord {
    package
        .records
        .iter()
        .find_map(|record| match record {
            DomainPackageRecord::FieldMember(field) if field.key == key(node) => Some(field),
            _ => None,
        })
        .unwrap_or_else(|| panic!("missing field {node}"))
}

/// TC-235 M16's semantic-model half: the process field, invariant, operation
/// parameter/effect, and population membership survive real intake and
/// normalization. Runtime truth for the two runs and operation outcomes are
/// asserted by the evaluator/replay layer.
#[trace("QSpec-TC-235")]
#[test]
fn tc235_m16_process_field_invariant_and_operation_sources_are_bound() {
    let document = mutate_k2(|document| {
        let process = type_named_mut(document, "Fulfilment");
        process["fields"].as_array_mut().unwrap().push(field(
            "Fulfilment",
            "attempts",
            "ix://quire/native/Integer",
        ));
        process["clauses"] = json!([clause(
            "Fulfilment",
            "NonNegativeAttempts",
            "self.attempts >= 0",
        )]);
        let mut restart = operation("Fulfilment", "restart");
        restart["pre"] = json!([{
            "language": "quire",
            "text": "self.attempts < 3",
            "origin": origin("Fulfilment/restart"),
        }]);
        restart["post"] = json!([{
            "language": "quire",
            "text": "self.attempts = pre(self.attempts) + 1",
            "origin": origin("Fulfilment/restart"),
        }]);
        restart["frame"]["modifies"] = json!([id("Fulfilment/attempts")]);
        process["operations"].as_array_mut().unwrap().push(restart);
        document["populations"] = json!([{
            "identity": id("PF"),
            "displayName": id("PF"),
            "kind": {"module": BUSINESS, "name": "population"},
            "members": [id("Fulfilment")],
            "extent": "closed",
            "origin": origin("PF"),
        }]);
    });

    let package = read_k2(&document).expect("M16 fixture passes the reader");
    let view = record_charges(&package, 25);
    assert_eq!(
        field_for(&package, "Fulfilment/attempts").value_type,
        ValueTypeRef::Native(qsl_semantics::model::domain_package::NativeValueType::Integer),
    );
    let invariant = clause_for(&package, "Fulfilment/NonNegativeAttempts");
    assert_eq!(invariant.owner, key("Fulfilment"));
    assert_eq!(invariant.text, "self.attempts >= 0");

    let restart = package
        .records
        .iter()
        .find_map(|record| match record {
            DomainPackageRecord::OperationMember(operation)
                if operation.key == key("Fulfilment/restart") =>
            {
                Some(operation)
            }
            _ => None,
        })
        .expect("restart is an admitted operation member");
    assert_eq!(restart.owner, key("Fulfilment"));
    assert_eq!(restart.effect.modifies, [key("Fulfilment/attempts")]);

    let population = package.records.iter().find_map(|record| match record {
        DomainPackageRecord::Population(population) if population.key == key("PF") => {
            Some(population)
        }
        _ => None,
    });
    assert_eq!(
        population.expect("PF is retained").member_types,
        [key("Fulfilment")]
    );
    assert!(view.type_identities().contains_key(&key("Fulfilment")));
}
/// TC-235 M20's model-semantics half: a process subtype inherits its
/// correlation identity and member meaning, keeps authored step order, and
/// refuses cross-meaning ancestry and specialization cycles.
#[trace("QSpec-TC-235")]
#[test]
fn tc235_m20_process_subtype_inherits_identity_and_steps() {
    let with_process_subtype = |extra_step: bool| {
        mutate_k2(|document| {
            let mut steps = vec![json!({
                "identity": id("ExpressFulfilment/expedite"),
                "name": "expedite",
                "stepKind": "command",
                "consumes": [],
                "emits": [],
                "origin": origin("ExpressFulfilment/expedite"),
            })];
            if extra_step {
                steps.push(json!({
                    "identity": id("ExpressFulfilment/receive_order"),
                    "name": "receive_order",
                    "stepKind": "event",
                    "consumes": [id("OrderPlaced")],
                    "emits": [],
                    "origin": origin("ExpressFulfilment/receive_order"),
                }));
            }
            document["types"].as_array_mut().unwrap().push(declaration(
                "ExpressFulfilment",
                "saga",
                json!({
                    "identityFields": [],
                    "supertypes": [id("Fulfilment")],
                    "operations": [],
                    "steps": steps,
                }),
            ));
            document["populations"] = json!([{
                "identity": id("PF"),
                "displayName": id("PF"),
                "kind": {"module": BUSINESS, "name": "population"},
                "members": [id("Fulfilment")],
                "extent": "closed",
                "origin": origin("PF"),
            }]);
        })
    };

    let positive = with_process_subtype(false);
    let package = read_k2(&positive).expect("M20 subtype package reads");
    let view = record_charges(&package, 23);
    let subtype = package.records.iter().find_map(|record| match record {
        DomainPackageRecord::Process(process) if process.key == key("ExpressFulfilment") => {
            Some(process)
        }
        _ => None,
    });
    let subtype = subtype.expect("ExpressFulfilment remains a process meaning");
    assert_eq!(subtype.identity_fields, [key("Fulfilment/correlation_id")]);
    assert_eq!(subtype.supertypes, [key("Fulfilment")]);
    assert_eq!(
        subtype
            .steps
            .iter()
            .map(|step| step.name.as_str())
            .collect::<Vec<_>>(),
        ["expedite"]
    );
    assert_eq!(subtype.steps[0].step_kind, ProcessStepKind::Command);
    assert!(view
        .type_identities()
        .contains_key(&key("ExpressFulfilment")));
    assert!(view.type_identities().contains_key(&key("Fulfilment")));
    let subtype_identity = view.type_identities()[&key("ExpressFulfilment")];
    assert!(view.declarations().iter().any(|declaration| {
        declaration.visible
            && declaration.preimage.original == key("Fulfilment/correlation_id")
            && declaration.preimage.owner_effective_type == Some(subtype_identity)
    }));
    let population = package.records.iter().find_map(|record| match record {
        DomainPackageRecord::Population(population) if population.key == key("PF") => {
            Some(population)
        }
        _ => None,
    });
    assert_eq!(
        population.expect("PF is retained").member_types,
        [key("Fulfilment")]
    );

    let with_event_step = with_process_subtype(true);
    let package = read_k2(&with_event_step).expect("M20 event step package reads");
    record_charges(&package, 23);
    let event_subtype = package.records.iter().find_map(|record| match record {
        DomainPackageRecord::Process(process) if process.key == key("ExpressFulfilment") => {
            Some(process)
        }
        _ => None,
    });
    let event_subtype = event_subtype.expect("subtype process remains");
    assert_eq!(event_subtype.steps.len(), 2);
    assert_eq!(event_subtype.steps[1].step_kind, ProcessStepKind::Event);
    assert_eq!(event_subtype.steps[1].consumes, [key("OrderPlaced")]);

    let missing_identity = mutate_k2(|document| {
        document["types"].as_array_mut().unwrap().push(declaration(
            "ExpressFulfilment",
            "saga",
            json!({
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
    let refusals = normalized_refusals(&missing_identity, 22);
    assert_eq!(refusals.len(), 2, "{refusals:#?}");
    assert_malformed_at(
        &refusals[0],
        &id("ExpressFulfilment"),
        "TypeDefinition.identityFields",
    );
    assert_malformed_at(
        &refusals[1],
        &id("ExpressFulfilment"),
        "TypeDefinition.supertypes[0]",
    );

    let wrong_meaning = mutate_k2(|document| {
        document["types"].as_array_mut().unwrap().push(declaration(
            "ExpressFulfilment",
            "saga",
            json!({
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
        document["populations"] = json!([{
            "identity": id("PF"),
            "displayName": id("PF"),
            "kind": {"module": BUSINESS, "name": "population"},
            "members": [id("Fulfilment")],
            "extent": "closed",
            "origin": origin("PF"),
        }]);
    });
    let refusals = normalized_refusals(&wrong_meaning, 23);
    assert_eq!(refusals.len(), 2, "{refusals:#?}");
    assert_malformed_at(
        &refusals[0],
        &id("ExpressFulfilment"),
        "TypeDefinition.identityFields",
    );
    assert_malformed_at(
        &refusals[1],
        &id("ExpressFulfilment"),
        "TypeDefinition.supertypes[0]",
    );

    let cycle = mutate_k2(|document| {
        document["types"].as_array_mut().unwrap().push(declaration(
            "ExpressFulfilment",
            "saga",
            json!({
                "identityFields": [],
                "supertypes": [],
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
        document["populations"] = json!([{
            "identity": id("PF"),
            "displayName": id("PF"),
            "kind": {"module": BUSINESS, "name": "population"},
            "members": [id("Fulfilment")],
            "extent": "closed",
            "origin": origin("PF"),
        }]);
    });
    let package = read_k2(&cycle).expect("cycle base package reads before normalization mutation");
    let mut records = package.records.clone();
    for record in &mut records {
        if let DomainPackageRecord::Process(process) = record {
            match process.key.node.as_str() {
                "ix://test/orders/Fulfilment" => {
                    process.supertypes = vec![key("ExpressFulfilment")]
                }
                "ix://test/orders/ExpressFulfilment" => {
                    process.supertypes = vec![key("Fulfilment")]
                }
                _ => {}
            }
        }
    }
    let cyclic_package = DomainPackage::new(package.model_selection.clone(), records);
    let (outcome, meter) =
        normalize_with_meter(&cyclic_package, ModelNormalizationLimits::default());
    assert_record_charge_count(&meter, 23);
    let NormalizeOutcome::Refused(refusals) = outcome else {
        panic!("cycle must be refused, got {outcome:?}");
    };
    assert_specialization_cycle_refusals(
        refusals.iter(),
        &["ExpressFulfilment", "Fulfilment"],
        &["ExpressFulfilment", "Fulfilment"],
    );
}

/// A subtype's effective identity is the stable union of every process
/// ancestor, including a deep second parent, rather than the first parent
/// encountered during intake.
#[trace("QSpec-TC-235")]
#[test]
fn tc235_m20_process_subtype_unions_deep_multi_parent_identities() {
    let document = mutate_k2(|document| {
        document["types"].as_array_mut().unwrap().extend([
            declaration(
                "AuditRoot",
                "saga",
                json!({
                    "identityFields": [id("AuditRoot/id")],
                    "fields": [field("AuditRoot", "id", "ix://quire/native/UUID")],
                    "operations": [],
                    "steps": [{
                        "identity": id("AuditRoot/record"),
                        "name": "record",
                        "stepKind": "command",
                        "consumes": [],
                        "emits": [],
                        "origin": origin("AuditRoot/record"),
                    }],
                }),
            ),
            declaration(
                "AuditMid",
                "saga",
                json!({
                    "identityFields": [],
                    "supertypes": [id("AuditRoot")],
                    "operations": [],
                    "steps": [{
                        "identity": id("AuditMid/record"),
                        "name": "record",
                        "stepKind": "command",
                        "consumes": [],
                        "emits": [],
                        "origin": origin("AuditMid/record"),
                    }],
                }),
            ),
            declaration(
                "ExpressFulfilment",
                "saga",
                json!({
                    "identityFields": [],
                    "supertypes": [id("Fulfilment"), id("AuditMid")],
                    "operations": [],
                    "steps": [{
                        "identity": id("ExpressFulfilment/record"),
                        "name": "record",
                        "stepKind": "command",
                        "consumes": [],
                        "emits": [],
                        "origin": origin("ExpressFulfilment/record"),
                    }],
                }),
            ),
        ]);
    });
    let package = read_k2(&document).expect("deep multi-parent process reads");
    let process = package.records.iter().find_map(|record| match record {
        DomainPackageRecord::Process(process) if process.key == key("ExpressFulfilment") => {
            Some(process)
        }
        _ => None,
    });
    assert_eq!(
        process
            .expect("ExpressFulfilment remains a process")
            .identity_fields,
        [key("Fulfilment/correlation_id"), key("AuditRoot/id")]
    );
}

/// A missing unrelated process member must remain a schema refusal when the
/// process subtype identity exception is applied.
#[trace("QSpec-TC-235")]
#[test]
fn tc235_m20_unrelated_process_root_schema_error_is_retained() {
    let document = mutate_k2(|document| {
        document["types"].as_array_mut().unwrap().push(declaration(
            "ExpressFulfilment",
            "saga",
            json!({
                "supertypes": [id("Fulfilment")],
                "operations": [],
                "steps": [{
                    "identity": id("ExpressFulfilment/record"),
                    "name": "record",
                    "stepKind": "command",
                    "consumes": [],
                    "emits": [],
                    "origin": origin("ExpressFulfilment/record"),
                }],
                "unexpected": true,
            }),
        ));
    });
    let refusals = read_refusals(&document);
    assert_eq!(refusals.len(), 1, "{refusals:#?}");
    assert!(refusals.iter().all(|refusal| {
        refusal.code == Code::InvalidModelBinding
            && matches!(
                refusal.cause,
                ModelRefusalCause::IntakeMalformedDeclaration { .. }
            )
            && refusal.detail.contains("TypeDefinition")
    }));
    assert!(refusals
        .iter()
        .any(|refusal| refusal.detail.contains("unexpected")));
}

/// The JSON acceptance vector records the two diagnostics that the producer
/// must retain for one malformed process subtype: missing identity fields
/// first, followed by the cross-meaning supertype.
#[trace("QSpec-TC-235")]
#[test]
fn tc235_m20_json_acceptance_reports_missing_identity_then_wrong_supertype() {
    let document = mutate_k2(|document| {
        document["types"].as_array_mut().unwrap().push(declaration(
            "ExpressFulfilment",
            "saga",
            json!({
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
        document["populations"] = json!([{
            "identity": id("PF"),
            "displayName": id("PF"),
            "kind": {"module": BUSINESS, "name": "population"},
            "members": [id("Fulfilment")],
            "extent": "closed",
            "origin": origin("PF"),
        }]);
    });
    let refusals = normalized_refusals(&document, 23);
    assert_eq!(refusals.len(), 2);
    assert_malformed_at(
        &refusals[0],
        &id("ExpressFulfilment"),
        "TypeDefinition.identityFields",
    );
    assert_malformed_at(
        &refusals[1],
        &id("ExpressFulfilment"),
        "TypeDefinition.supertypes[0]",
    );
}

/// The cyclic JSON acceptance vector retains the normalization-level cycle
/// diagnostic and names both process identities in the cycle.
#[trace("QSpec-TC-235")]
#[test]
fn tc235_m20_json_acceptance_reports_specialization_cycle() {
    let document = mutate_k2(|document| {
        type_named_mut(document, "Fulfilment")["supertypes"] = json!([id("ExpressFulfilment")]);
        document["types"].as_array_mut().unwrap().push(declaration(
            "ExpressFulfilment",
            "saga",
            json!({
                "identityFields": [],
                "supertypes": [id("Fulfilment")],
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
        document["populations"] = json!([{
            "identity": id("PF"),
            "displayName": id("PF"),
            "kind": {"module": BUSINESS, "name": "population"},
            "members": [id("Fulfilment")],
            "extent": "closed",
            "origin": origin("PF"),
        }]);
    });
    let refusals = normalized_refusals(&document, 23);
    assert_specialization_cycle_refusals(
        refusals.iter(),
        &["ExpressFulfilment", "Fulfilment"],
        &["ExpressFulfilment", "Fulfilment"],
    );
}

/// TC-235 M21's semantic-model half: relationship ends remain attached to
/// their authored relationship, the clause and populations stay typed, and
/// invalid target/owner meanings retain their FR-208 refusal boundary.
#[trace("QSpec-TC-235")]
#[test]
fn tc235_m21_relationship_sources_and_owner_rules_are_retained() {
    let relationship = |owner: &str, name: &str, target: &str| {
        let owner_name = owner.rsplit('/').next().expect("owner has a node name");
        let target_name = target.rsplit('/').next().expect("target has a node name");
        let node = format!("relationship/{owner_name}-{name}-{target_name}");
        json!({
            "identity": id(&node),
            "category": "structural",
            "composite": false,
            "direction": "source-to-target",
            "origin": relationship_origin(&node),
            "sourceEnd": {
                "role": name,
                "type": id(owner),
                "multiplicity": {"lower": 0, "ordered": false, "unique": true},
            },
            "targetEnd": {
                "type": id(target),
                "multiplicity": {"lower": 0, "upper": 1, "ordered": false, "unique": true},
            },
        })
    };

    let order_link = mutate_k2(|document| {
        type_named_mut(document, "Order")["relationships"] =
            json!([relationship("Order", "fulfilments", "Fulfilment")]);
    });
    let package = read_k2(&order_link).expect("M21 object relationship reads");
    let view = record_charges(&package, 22);
    let link = package.records.iter().find_map(|record| match record {
        DomainPackageRecord::Relationship(link)
            if link.key == key("relationship/Order-fulfilments-Fulfilment") =>
        {
            Some(link)
        }
        _ => None,
    });
    let link = link.expect("Order/fulfilments remains a relationship record");
    assert_eq!(link.source.type_identity, key("Order"));
    assert_eq!(link.source.role.as_deref(), Some("fulfilments"));
    assert_eq!(link.target.type_identity, key("Fulfilment"));
    assert_eq!(
        link.direction,
        qsl_semantics::model::domain_package::RelationshipDirection::SourceToTarget
    );
    assert!(view.type_identities().contains_key(&key("Order")));
    assert!(view.type_identities().contains_key(&key("Fulfilment")));

    let fulfilment_link = mutate_k2(|document| {
        type_named_mut(document, "Fulfilment")["relationships"] =
            json!([relationship("Fulfilment", "order", "Order")]);
        type_named_mut(document, "Fulfilment")["clauses"] =
            json!([clause("Fulfilment", "HasOrder", "present(self.order)")]);
        document["populations"] = json!([
            {
                "identity": id("PF"),
                "displayName": id("PF"),
                "kind": {"module": BUSINESS, "name": "population"},
                "members": [id("Fulfilment")],
                "extent": "closed",
                "origin": origin("PF"),
            },
            {
                "identity": id("PO"),
                "displayName": id("PO"),
                "kind": {"module": BUSINESS, "name": "population"},
                "members": [id("Order")],
                "extent": "closed",
                "origin": origin("PO"),
            },
        ]);
    });
    let package = read_k2(&fulfilment_link).expect("M21 reverse relationship reads");
    record_charges(&package, 25);
    let link = package.records.iter().find_map(|record| match record {
        DomainPackageRecord::Relationship(link)
            if link.key == key("relationship/Fulfilment-order-Order") =>
        {
            Some(link)
        }
        _ => None,
    });
    let link = link.expect("Fulfilment/order remains a relationship record");
    assert_eq!(link.source.type_identity, key("Fulfilment"));
    assert_eq!(link.target.type_identity, key("Order"));
    assert_eq!(
        clause_for(&package, "Fulfilment/HasOrder").text,
        "present(self.order)"
    );
    // FR-363-AC-1: the authored clause type-checks, `self.order` navigating
    // the `[0,1]` target end of `Fulfilment/order` as an `Option<Reference<Order>>`.
    let checked = crate::relationship_navigation::check_invariant(
        package.clone(),
        "Fulfilment",
        &clause_for(&package, "Fulfilment/HasOrder").text,
    )
    .expect("present(self.order) type-checks");
    assert_eq!(
        crate::relationship_navigation::navigations(&checked),
        [(
            quire_exact::ValueType::option(quire_exact::ValueType::Reference(
                crate::relationship_navigation::object(&checked, "Order")
            )),
            key("relationship/Fulfilment-order-Order"),
            "order".to_owned(),
        )]
    );
    assert!(package.records.iter().any(|record| {
        matches!(record, DomainPackageRecord::Population(population)
            if population.key == key("PF") && population.member_types == [key("Fulfilment")])
    }));
    assert!(package.records.iter().any(|record| {
        matches!(record, DomainPackageRecord::Population(population)
            if population.key == key("PO") && population.member_types == [key("Order")])
    }));

    let bad_target = mutate_k2(|document| {
        type_named_mut(document, "Fulfilment")["relationships"] =
            json!([relationship("Fulfilment", "store", "OrderStore")]);
    });
    let refusals = normalized_refusals(&bad_target, 22);
    assert_eq!(refusals.len(), 1);
    assert_malformed_at(
        &refusals[0],
        &id("relationship/Fulfilment-store-OrderStore"),
        "Relationship.target",
    );

    let bad_owner = mutate_k2(|document| {
        type_named_mut(document, "Money")["relationships"] =
            json!([relationship("Money", "owner", "Order")]);
    });
    let refusals = normalized_refusals(&bad_owner, 22);
    assert_eq!(refusals.len(), 1);
    // QSpec M10 runs 7-8 spell their relationship nodes the same way
    // (`Order/scope`, `OrderLifecycle/next`); the divergence is carried by
    // QSpec PR #240, which this test code does not wait on.
    assert_malformed_at(
        &refusals[0],
        &id("relationship/Money-owner-Order"),
        "TypeDefinition.relationships[0]",
    );

    // Each refused relationship names its own position in the owner's array.
    let two_bad = mutate_k2(|document| {
        type_named_mut(document, "Money")["relationships"] = json!([
            relationship("Money", "owner", "Order"),
            relationship("Money", "payer", "Order"),
        ]);
    });
    let refusals = normalized_refusals(&two_bad, 23);
    assert_eq!(refusals.len(), 2, "{refusals:?}");
    for (name, position) in [("Money-owner-Order", 0), ("Money-payer-Order", 1)] {
        let refusal = refusals
            .iter()
            .find(|refusal| refusal.detail.contains(name))
            .expect("one refusal per relationship");
        assert_malformed_at(
            refusal,
            &id(&format!("relationship/{name}")),
            &format!("TypeDefinition.relationships[{position}]"),
        );
    }
}

/// A record value type's or event type's operations are each refused at their
/// own position in the declaration's `operations` array, not at `[0]`.
#[test]
fn tc235_operation_refusals_name_the_declared_position() {
    let record_document = mutate_k2(|document| {
        type_named_mut(document, "Money")["operations"] =
            json!([operation("Money", "zeta"), operation("Money", "alpha")]);
    });
    let refusals = normalized_refusals(&record_document, 23);
    assert_eq!(refusals.len(), 2, "{refusals:?}");
    for (name, position) in [("zeta", 0), ("alpha", 1)] {
        let refusal = refusals
            .iter()
            .find(|refusal| refusal.detail.contains(&id(&format!("Money/{name}"))))
            .expect("one refusal per operation");
        assert_unsupported_at(
            refusal,
            &id(&format!("Money/{name}")),
            &format!("TypeDefinition.operations[{position}]"),
        );
    }

    let event_document = mutate_k2(|document| {
        type_named_mut(document, "OrderPlaced")["operations"] = json!([
            operation("OrderPlaced", "zeta"),
            operation("OrderPlaced", "alpha")
        ]);
    });
    let refusals = normalized_refusals(&event_document, 23);
    assert_eq!(refusals.len(), 2, "{refusals:?}");
    for (name, position) in [("zeta", 0), ("alpha", 1)] {
        let refusal = refusals
            .iter()
            .find(|refusal| refusal.detail.contains(&id(&format!("OrderPlaced/{name}"))))
            .expect("one refusal per operation");
        assert_malformed_at(
            refusal,
            &id(&format!("OrderPlaced/{name}")),
            &format!("TypeDefinition.operations[{position}]"),
        );
    }
}

/// TC-235 M22's assembly half: the invocation parameter is a declared record
/// type owned by the operation signature. Runtime-value-path comparison for
/// `self.placed` versus `flags.placed` is an evaluator assertion and remains
/// outside this source/assembly test.
#[trace("QSpec-TC-235")]
#[test]
fn tc235_m22_flags_parameter_is_assembled_as_a_record() {
    let document = mutate_k2(|document| {
        type_named_mut(document, "OrderLifecycle")["fields"]
            .as_array_mut()
            .unwrap()
            .push(field(
                "OrderLifecycle",
                "cancelled",
                "ix://quire/native/Boolean",
            ));
        type_named_mut(document, "OrderLifecycle")["clauses"]
            .as_array_mut()
            .unwrap()
            .push(clause(
                "OrderLifecycle",
                "Consistent",
                "not (self.placed and self.cancelled)",
            ));
        document["types"].as_array_mut().unwrap().push(declaration(
            "Flags",
            "money",
            json!({
                "fields": [field("Flags", "placed", "ix://quire/native/Boolean")],
                "operations": [],
            }),
        ));
        let place = operation("OrderLifecycle", "place");
        let mut place = place;
        place["params"] = json!([{
            "identity": id("OrderLifecycle/place/flags"),
            "origin": origin("OrderLifecycle/place/flags"),
            "name": "flags",
            "typeRef": id("Flags"),
            "presence": "required",
            "nullable": false,
            "defaultKind": "none",
            "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
        }]);
        place["pre"] = json!([{
            "language": "quire",
            "text": "not self.placed",
            "origin": origin("OrderLifecycle/place"),
        }]);
        place["post"] = json!([{
            "language": "quire",
            "text": "self.placed and not self.cancelled",
            "origin": origin("OrderLifecycle/place"),
        }]);
        place["frame"]["modifies"] = json!([id("OrderLifecycle/placed")]);
        type_named_mut(document, "OrderLifecycle")["operations"][0] = place;
    });

    let package = read_k2(&document).expect("M22 K2 plus Flags passes the reader");
    let view = record_charges(&package, 25);

    let bytes = serde_json::to_vec(&document).expect("M22 package serializes");
    let packages = package_input([bytes.as_slice()]);
    let digest = hex(packages.keys().next().expect("one offered package"));
    let unit = format!(
        concat!(
            "language \"ix:native\" edition \"1-draft\";\n",
            "profile v = \"quire.value.complete/v1\";\n",
            "model K2 = {PACKAGE:?} version \"1\" digest \"sha256-jcs:{digest}\";\n",
            "function noop using v(): Boolean pure {{ true }}\n",
        ),
        PACKAGE = PACKAGE,
        digest = digest,
    );
    let parsed = qsl_cst::parse(
        SourceIdentity::new("test", "tc-235", "m22-flags", "fixture:1"),
        "unit.native",
        unit.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("M22 assembly unit parses");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let built = qsl_forms::build_unit(&parsed).expect("M22 assembly unit builds");
    let models = admit_unit(
        &built.selections().models,
        &packages,
        ModelNormalizationLimits::default(),
    )
    .expect("M22 package admits for assembly");
    let declarations = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        built,
        models,
        Vec::new(),
    )
    .expect("M22 package assembles");
    let lifecycle = view.type_identities()[&key("OrderLifecycle")];
    let place = declarations
        .operations
        .declared_by(lifecycle)
        .iter()
        .find(|operation| operation.name() == "place")
        .expect("place operation is assembled");
    assert_eq!(place.parameters().len(), 1);
    assert_eq!(place.parameters()[0].0, "flags");
    assert!(matches!(
        place.parameters()[0].1,
        quire_exact::ValueType::Composite(_)
    ));
    assert_eq!(
        clause_for(&package, "OrderLifecycle/Consistent").text,
        "not (self.placed and self.cancelled)"
    );
    assert_eq!(place.effect().modifies, [key("OrderLifecycle/placed")]);
}

/// TC-235 M02 adds a record-valued field and an object invariant that compares
/// two Money values. This crate checks the exact field and clause bindings;
/// the downstream evaluation vector distinguishes matching from differing
/// currency fields.
///
/// Mutation used (derived by reading `read_clauses` in `model/intake.rs`, not
/// executed): recording the wrong clause `owner` flips `clause.owner ==
/// Order`; dropping the field's `typeRef` flips the `ValueTypeRef::Package`
/// assertion. The 23-charge count is taken over the document's declared nodes,
/// so it does not change when a record goes unread; the clause and field
/// assertions above are the ones that fail.
#[trace("QSpec-TC-235", "QSpec-FR-208-AC-4")]
#[test]
fn tc235_m02_record_value_field_and_clause_bind_to_order() {
    let document = mutate_k2(|document| {
        let order = type_named_mut(document, "Order");
        order["fields"]
            .as_array_mut()
            .unwrap()
            .push(field("Order", "quoted", &id("Money")));
        order["clauses"] = json!([clause("Order", "QuotedIsTotal", "self.quoted = self.total",)]);
    });

    let package = read_k2(&document).expect("M02 K2 reads with a record-valued field");
    let view = record_charges(&package, 23);

    assert_eq!(
        field_for(&package, "Order/quoted").value_type,
        ValueTypeRef::Package(key("Money")),
    );
    let clause = clause_for(&package, "Order/QuotedIsTotal");
    assert_eq!(clause.owner, key("Order"));
    assert_eq!(clause.text, "self.quoted = self.total");
    let order_type = view.type_identities()[&key("Order")];
    assert!(view.declarations().iter().any(|declaration| {
        declaration.visible
            && declaration.preimage.original == key("Order/QuotedIsTotal")
            && declaration.preimage.owner_effective_type == Some(order_type)
    }));
}

/// TC-235 M13 preserves all six Money-valued access paths used by the runtime
/// vector: two direct fields, a bounded sequence, a present optional, and an
/// invocation parameter, plus the Money invariant itself.
///
/// The runtime half (four violation paths, run 2's `forbidden-pre-read`) is
/// not asserted here. Remaining work: #789.
///
/// Mutation used (derived by reading `multiplicity` and `read_clauses` in
/// `model/intake.rs`, not executed): ignoring `upper` flips
/// `history.multiplicity.upper == Some(3)`; reading `presence` as required
/// flips the `discount` optional assertion; dropping the operation parameter
/// flips `operation.parameters.len() == 1`.
#[test]
fn tc235_m13_record_invariant_binds_all_reachable_value_sources() {
    let document = mutate_k2(|document| {
        type_named_mut(document, "Money")["clauses"] =
            json!([clause("Money", "NonNegative", "self.amount_minor >= 0",)]);

        let order = type_named_mut(document, "Order");
        let history = field("Order", "history", &id("Money"));
        let mut bounded_history = history;
        bounded_history["multiplicity"] = json!({
            "lower": 0,
            "upper": 3,
            "ordered": false,
            "unique": true,
        });
        let mut discount = field("Order", "discount", &id("Money"));
        discount["presence"] = json!("optional");
        let fields = order["fields"].as_array_mut().unwrap();
        fields.push(field("Order", "quoted", &id("Money")));
        fields.push(bounded_history);
        fields.push(discount);

        let mut requote = super::operation("Order", "requote");
        requote["params"] = json!([{
            "identity": id("Order/requote/amount"),
            "name": "amount",
            "origin": origin("Order/requote/amount"),
            "typeRef": id("Money"),
            "presence": "required",
            "nullable": false,
            "defaultKind": "none",
            "multiplicity": {
                "lower": 1,
                "upper": 1,
                "ordered": false,
                "unique": true,
            },
        }]);
        order["operations"] = json!([]);
        order["operations"].as_array_mut().unwrap().push(requote);
    });

    let package = read_k2(&document).expect("M13 K2 source reads");
    let view = record_charges(&package, 26);
    let money_type = view.type_identities()[&key("Money")];
    let invariant = clause_for(&package, "Money/NonNegative");
    assert_eq!(invariant.owner, key("Money"));
    assert_eq!(invariant.text, "self.amount_minor >= 0");
    assert!(view.declarations().iter().any(|declaration| {
        declaration.visible
            && declaration.preimage.original == key("Money/NonNegative")
            && declaration.preimage.owner_effective_type == Some(money_type)
    }));

    assert_eq!(
        field_for(&package, "Order/quoted").value_type,
        ValueTypeRef::Package(key("Money")),
    );
    let history = field_for(&package, "Order/history");
    assert_eq!(history.multiplicity.lower, 0);
    assert_eq!(history.multiplicity.upper, Some(3));
    assert_eq!(
        field_for(&package, "Order/discount").presence,
        quire_exact::Presence::Optional
    );
    let operation = package
        .records
        .iter()
        .find_map(|record| match record {
            DomainPackageRecord::OperationMember(operation)
                if operation.key == key("Order/requote") =>
            {
                Some(operation)
            }
            _ => None,
        })
        .expect("requote operation is retained");
    assert_eq!(operation.parameters.len(), 1);
    assert_eq!(
        operation.parameters[0].value_type,
        ValueTypeRef::Package(key("Money")),
    );

    // TC-235 M13 run 2's text survives the same reader/normalizer boundary;
    // evaluation must refuse its pre-read as wrong_snapshot/forbidden-pre-read.
    let pre_document = mutate_k2(|document| {
        type_named_mut(document, "Money")["clauses"] = json!([clause(
            "Money",
            "NonNegative",
            "pre(self.amount_minor) >= 0",
        )]);
        let order = type_named_mut(document, "Order");
        order["fields"]
            .as_array_mut()
            .unwrap()
            .push(field("Order", "quoted", &id("Money")));
        order["fields"]
            .as_array_mut()
            .unwrap()
            .push(field("Order", "history", &id("Money")));
        let mut requote = super::operation("Order", "requote");
        requote["params"] = json!([{
            "identity": id("Order/requote/amount"),
            "name": "amount",
            "origin": origin("Order/requote/amount"),
            "typeRef": id("Money"),
            "presence": "required",
            "nullable": false,
            "defaultKind": "none",
            "multiplicity": {
                "lower": 1,
                "upper": 1,
                "ordered": false,
                "unique": true,
            },
        }]);
        order["operations"] = json!([]);
        order["operations"].as_array_mut().unwrap().push(requote);
        let mut discount = field("Order", "discount", &id("Money"));
        discount["presence"] = json!("optional");
        order["fields"].as_array_mut().unwrap().push(discount);
    });
    let pre_package = read_k2(&pre_document).expect("M13 pre-read clause is retained as source");
    record_charges(&pre_package, 26);
    assert_eq!(
        clause_for(&pre_package, "Money/NonNegative").text,
        "pre(self.amount_minor) >= 0",
    );
}

/// TC-235 M14 binds an invariant on an event type without inventing an event
/// value from the unrelated Order observation. The pre-read rejection is a
/// downstream evaluation result; this layer retains the exact clause text.
///
/// The evaluation half (no result for `Occurred`; run 2's
/// `wrong_snapshot`/`forbidden-pre-read`) is not asserted here. Remaining
/// work: #789.
///
/// Mutation used (derived by reading `model/event.rs`, not executed):
/// dropping the event's `occurrence_field` flips the
/// `event.occurrence_field == Some(..)` assertion; dropping the clause record
/// flips `clause_for(.., "OrderPlaced/Occurred")`.
#[test]
fn tc235_m14_event_invariant_is_bound_without_an_event_value() {
    for (text, expected) in [
        (
            "self.occurred_at = self.occurred_at",
            "self.occurred_at = self.occurred_at",
        ),
        (
            "pre(self.order_id) = self.order_id",
            "pre(self.order_id) = self.order_id",
        ),
    ] {
        let document = mutate_k2(|document| {
            type_named_mut(document, "OrderPlaced")["clauses"] =
                json!([clause("OrderPlaced", "Occurred", text)]);
        });
        let package = read_k2(&document).expect("M14 event clause source reads");
        let view = record_charges(&package, 22);
        let event = package
            .records
            .iter()
            .find_map(|record| match record {
                DomainPackageRecord::EventType(event) if event.key == key("OrderPlaced") => {
                    Some(event)
                }
                _ => None,
            })
            .expect("OrderPlaced retains its event meaning");
        assert_eq!(event.occurrence_field, Some(key("OrderPlaced/occurred_at")));
        assert_eq!(clause_for(&package, "OrderPlaced/Occurred").text, expected);
        assert!(view.type_identities().contains_key(&key("OrderPlaced")));
    }
}

/// TC-235 M17 keeps the record subtype's inherited fields and invariant under
/// its own effective identity, and reports wrong-meaning ancestry and a
/// specialization cycle with the specified typed outcomes.
///
/// Run 1 here retypes `Order/total` to `TaxedMoney` to check the effective
/// members; the unretyped input is
/// `tc235_m17_run1_taxed_money_is_admitted_at_order_total_and_not_equal_to_money`.
///
#[test]
fn tc235_m17_record_value_specialization_vectors() {
    let positive_document = mutate_k2(|document| {
        type_named_mut(document, "Money")["clauses"] =
            json!([clause("Money", "NonNegative", "self.amount_minor >= 0",)]);
        type_named_mut(document, "Order")["fields"][1]["typeRef"] = json!(id("TaxedMoney"));
        document["types"].as_array_mut().unwrap().push(declaration(
            "TaxedMoney",
            "money",
            json!({
                "supertypes": [id("Money")],
                "fields": [field("TaxedMoney", "tax_minor", "ix://quire/native/Integer")],
            }),
        ));
    });
    let package = read_k2(&positive_document).expect("M17 record subtype reads");
    let view = record_charges(&package, 24);
    let taxed_key = key("TaxedMoney");
    let taxed_type = view.type_identities()[&taxed_key];
    let DomainPackageRecord::RecordValueType(RecordValueTypeRecord { supertypes, .. }) = package
        .records
        .iter()
        .find(|record| record.key() == &taxed_key)
        .expect("TaxedMoney source record exists")
    else {
        panic!("TaxedMoney retains its record-value meaning");
    };
    assert_eq!(supertypes, &[key("Money")]);
    assert_eq!(
        field_for(&package, "Order/total").value_type,
        ValueTypeRef::Package(taxed_key.clone())
    );
    let mut effective_members: Vec<_> = view
        .declarations()
        .iter()
        .filter(|declaration| {
            declaration.visible && declaration.preimage.owner_effective_type == Some(taxed_type)
        })
        .map(|declaration| declaration.preimage.original.node.clone())
        .collect();
    effective_members.sort();
    assert_eq!(
        effective_members,
        [
            id("Money/NonNegative"),
            id("Money/amount_minor"),
            id("Money/currency"),
            id("TaxedMoney/tax_minor"),
        ]
    );
    assert!(view.object_universe_of(&taxed_key).is_none());
    assert_ne!(view.type_identities()[&key("Money")], taxed_type);

    let wrong_meaning = mutate_k2(|document| {
        type_named_mut(document, "Money")["clauses"] =
            json!([clause("Money", "NonNegative", "self.amount_minor >= 0",)]);
        document["types"].as_array_mut().unwrap().push(declaration(
            "TaxedMoney",
            "money",
            json!({
                "supertypes": [id("OrderPlaced")],
                "fields": [field("TaxedMoney", "tax_minor", "ix://quire/native/Integer")],
            }),
        ));
    });
    let wrong_meaning_refusals = normalized_refusals(&wrong_meaning, 24);
    assert_eq!(wrong_meaning_refusals.len(), 1);
    assert_malformed_at(
        &wrong_meaning_refusals[0],
        &id("TaxedMoney"),
        "TypeDefinition.supertypes[0]",
    );

    let cycle = mutate_k2(|document| {
        type_named_mut(document, "Money")["clauses"] =
            json!([clause("Money", "NonNegative", "self.amount_minor >= 0",)]);
        type_named_mut(document, "Money")["supertypes"] = json!([id("TaxedMoney")]);
        document["types"].as_array_mut().unwrap().push(declaration(
            "TaxedMoney",
            "money",
            json!({
                "supertypes": [id("Money")],
                "fields": [field("TaxedMoney", "tax_minor", "ix://quire/native/Integer")],
            }),
        ));
    });
    let cycle_refusals = normalized_refusals(&cycle, 24);
    assert_specialization_cycle_refusals(
        cycle_refusals.iter(),
        &["Money", "TaxedMoney"],
        &["Money", "TaxedMoney"],
    );
}

/// TC-235 M17 run 3 / M20 run 4: one `invalid_model_binding`/
/// `specialization-cycle` refusal per declaration on the cycle, in
/// declaration-key order, each listing the cycle's keys rotated to start at
/// the least key. Typed fields only.
fn assert_specialization_cycle_refusals<'a>(
    refusals: impl Iterator<Item = &'a qsl_semantics::model::normalize::ModelRefusal>,
    declarations: &[&str],
    cycle: &[&str],
) {
    let refusals: Vec<_> = refusals.collect();
    assert_eq!(refusals.len(), declarations.len(), "{refusals:#?}");
    for (refusal, declaration) in refusals.into_iter().zip(declarations) {
        assert_eq!(refusal.code, Code::InvalidModelBinding);
        let ModelRefusalCause::SpecializationCycle {
            declaration: actual,
            cycle: actual_cycle,
        } = &refusal.cause
        else {
            panic!("expected typed specialization cycle cause: {refusal:?}");
        };
        assert_eq!(actual, &key(declaration));
        let expected: Vec<_> = cycle.iter().map(|name| key(name)).collect();
        assert_eq!(actual_cycle, &expected);
    }
}

/// QSL-732: the effective composite of a record or event type named `name`.
fn assembled_composite(
    declarations: &PackageDeclarations,
    name: &str,
) -> quire_semantic_value::declaration::CompositeDeclaration {
    declarations
        .types
        .composites()
        .find(|declaration| declaration.name() == name)
        .unwrap_or_else(|| panic!("{name} is assembled"))
        .clone()
}

/// A `Money` or `TaxedMoney` value over the assembled environment.
fn money_value(
    declarations: &PackageDeclarations,
    name: &str,
    with_tax: bool,
) -> quire_exact::Value {
    use quire_exact::{FieldValue, Integer, Value};
    let mut fields = vec![(
        "amount_minor",
        FieldValue::Present(Value::Integer(Integer::from(100_i64))),
    )];
    if with_tax {
        fields.push((
            "tax_minor",
            FieldValue::Present(Value::Integer(Integer::from(7_i64))),
        ));
    }
    declarations
        .types
        .record(assembled_composite(declarations, name).key(), fields)
        .expect("the record constructs")
}

/// QSL-732 / FR-056: a domain package's record and event `supertypes[]`
/// assemble as composite ancestry. A child value is admitted at its parent's
/// slot and keeps its own identity; the parent value is refused at the
/// child's slot; an unrelated record is refused at either.
#[trace("FR-056-AC-18", "TC-235")]
#[test]
fn qsl_732_composite_subtype_is_admitted_at_its_parent_slot_only() {
    use quire_exact::{Value, ValueType};
    let document = mutate_k2(|document| {
        // Same slice as `k2_record_event_subtype_slice(true)`, plus a record
        // that is related to neither Money nor TaxedMoney.
        *document = k2_record_event_subtype_slice(true);
        document["types"].as_array_mut().unwrap().push(declaration(
            "Discount",
            "money",
            json!({
                "fields": [field("Discount", "amount_minor", "ix://quire/native/Integer")],
            }),
        ));
    });
    let declarations = assemble_k2_unit(
        &document,
        "function noop using v(money: M::Money): Integer pure { 0 }",
    );
    let (money, taxed, discount) = (
        assembled_composite(&declarations, "M::Money").key(),
        assembled_composite(&declarations, "M::TaxedMoney").key(),
        assembled_composite(&declarations, "M::Discount").key(),
    );
    assert_eq!(
        assembled_composite(&declarations, "M::TaxedMoney").supertypes(),
        [money]
    );
    assert!(assembled_composite(&declarations, "M::Money")
        .supertypes()
        .is_empty());

    let types = &declarations.types;
    let child = money_value(&declarations, "M::TaxedMoney", true);
    let parent = money_value(&declarations, "M::Money", false);
    let unrelated = money_value(&declarations, "M::Discount", false);

    assert_eq!(types.admits(&ValueType::Composite(money), &child), Ok(true));
    assert_eq!(types.admits(&ValueType::Composite(taxed), &child), Ok(true));
    // The child keeps its own identity when admitted at the parent's slot.
    let Value::Composite(record) = &child else {
        panic!("a composite value");
    };
    assert_eq!(record.declaration(), taxed);

    assert_eq!(
        types.admits(&ValueType::Composite(taxed), &parent),
        Ok(false)
    );
    assert_eq!(
        types.admits(&ValueType::Composite(money), &unrelated),
        Ok(false)
    );
    assert_eq!(
        types.admits(&ValueType::Composite(taxed), &unrelated),
        Ok(false)
    );
    assert_eq!(
        types.admits(&ValueType::Composite(discount), &child),
        Ok(false)
    );
    assert_eq!(
        types.admits(&ValueType::Composite(discount), &parent),
        Ok(false)
    );
}

/// QSL-732: an event type's `supertypes[]` assemble the same way.
#[trace("FR-056-AC-18", "TC-235")]
#[test]
fn qsl_732_event_subtype_conforms_to_its_parent_event() {
    let document = k2_record_event_subtype_slice(true);
    let declarations = assemble_k2_unit(
        &document,
        "function noop using v(money: M::Money): Integer pure { 0 }",
    );
    let (placed, express) = (
        assembled_composite(&declarations, "M::OrderPlaced").key(),
        assembled_composite(&declarations, "M::ExpressOrderPlaced").key(),
    );
    let types = &declarations.types;
    assert!(types.composite_conforms(express, placed));
    assert!(types.composite_conforms(express, express));
    assert!(!types.composite_conforms(placed, express));
    let money = assembled_composite(&declarations, "M::Money").key();
    assert!(!types.composite_conforms(express, money));
}

/// TC-235 M17 run 1, value half, on the row's input: K2 with `Money` given
/// `NonNegative`, `TaxedMoney` (`supertypes: [Money]`, field `tax_minor`) and
/// `Order/total` still typed `Money`. A `TaxedMoney` value `t`
/// `{amount_minor: 100, currency: "EUR", tax_minor: 20}` is admitted at the
/// declared type of `Order/total`, keeps its `TaxedMoney` declaration, and the
/// `t` is not equal to a `Money` value `{amount_minor: 100, currency: "EUR"}`.
///
/// Mutation used (derived by reading `TypeEnvironment::admits` and
/// `quire_exact::planned_equality`, not executed): removing the composite
/// supertype edge from `TaxedMoney` to `Money` flips the `admits` assertion to
/// false. Any change that makes `planned_equality` return `true` for these operands
/// (for example comparing only the shared fields) flips the `!equal`
/// assertion.
#[trace("FR-056-AC-18", "TC-235")]
#[test]
fn tc235_m17_run1_taxed_money_is_admitted_at_order_total_and_not_equal_to_money() {
    use quire_exact::{FieldValue, Integer, Value, ValueType};
    use quire_semantic_value::declaration::CompositeShape;
    let document = mutate_k2(|document| {
        type_named_mut(document, "Money")["clauses"] =
            json!([clause("Money", "NonNegative", "self.amount_minor >= 0")]);
        document["types"].as_array_mut().unwrap().push(declaration(
            "TaxedMoney",
            "money",
            json!({
                "supertypes": [id("Money")],
                "fields": [field("TaxedMoney", "tax_minor", "ix://quire/native/Integer")],
            }),
        ));
    });
    let declarations = assemble_k2_unit(
        &document,
        "function noop using v(money: M::Money): Integer pure { 0 }",
    );
    let total_type = declarations
        .types
        .object_types()
        .find(|object| object.name() == "M::Order")
        .expect("Order is assembled as an object type")
        .attributes()
        .iter()
        .find(|field| field.name() == "total")
        .expect("Order/total is assembled")
        .value_type()
        .clone();
    let money_key = assembled_composite(&declarations, "M::Money").key();
    assert_eq!(
        total_type,
        ValueType::Composite(money_key),
        "total stays Money"
    );

    let build = |name: &str, tax: Option<i64>| {
        let composite = assembled_composite(&declarations, name);
        let CompositeShape::Record(fields) = composite.shape() else {
            panic!("{name} is a record composite");
        };
        let slots: Vec<(&str, FieldValue)> = fields
            .iter()
            .map(|field| {
                let value = match (field.name(), field.value_type()) {
                    ("amount_minor", _) => Value::Integer(Integer::from(100_i64)),
                    ("tax_minor", _) => Value::Integer(Integer::from(tax.expect("tax_minor"))),
                    ("currency", ValueType::String) => Value::String("EUR".to_owned()),
                    (other, ty) => panic!("unexpected field {other}: {ty:?}"),
                };
                (field.name(), FieldValue::Present(value))
            })
            .collect();
        declarations
            .types
            .record(composite.key(), slots)
            .expect("the record constructs")
    };
    let child = build("M::TaxedMoney", Some(20));
    let parent = build("M::Money", None);

    assert_eq!(declarations.types.admits(&total_type, &child), Ok(true));
    let Value::Composite(record) = &child else {
        panic!("t is a composite value");
    };
    assert_eq!(
        record.declaration(),
        assembled_composite(&declarations, "M::TaxedMoney").key(),
        "t keeps its TaxedMoney type"
    );
    let unlimited = quire_exact::ScalarLimits {
        integer_bits: u64::MAX,
        decimal_digits: u64::MAX,
        scale_expansion: u64::MAX,
        text_input_bytes: u64::MAX,
        text_scalars: u64::MAX,
        normalized_scalars: u64::MAX,
        unit_edges: u64::MAX,
        value_occurrences: u64::MAX,
        work_units: u64::MAX,
        result_units: u64::MAX,
    };
    let equal =
        quire_exact::planned_equality(&child, &parent, &mut quire_exact::Meter::new(unlimited))
            .completed()
            .expect("equality of the two admitted records completes");
    assert!(!equal, "t is not equal to the Money value");
}

/// TC-235 M18 run 1, consumption half: `ExpressOrderPlaced` is accepted
/// wherever `Fulfilment` step `receive_order` consumes `OrderPlaced`.
///
/// Mutation used (derived by reading `TypeEnvironment::composite_conforms`,
/// not executed): removing the event supertype edge flips the
/// `composite_conforms(express, consumed)` assertion to false.
#[trace("FR-056-AC-18", "TC-235")]
#[test]
fn tc235_m18_run1_express_event_conforms_to_the_event_receive_order_consumes() {
    let document = k2_record_event_subtype_slice(true);
    let package = read_k2(&document).expect("M18 slice reads");
    let steps = package
        .records
        .iter()
        .find_map(|record| match record {
            DomainPackageRecord::Process(process) if process.key == key("Fulfilment") => {
                Some(&process.steps)
            }
            _ => None,
        })
        .expect("Fulfilment retains its steps");
    assert_eq!(steps[0].name, "receive_order");
    assert_eq!(steps[0].consumes, [key("OrderPlaced")]);

    let declarations = assemble_k2_unit(
        &document,
        "function noop using v(money: M::Money): Integer pure { 0 }",
    );
    let consumed = assembled_composite(&declarations, "M::OrderPlaced").key();
    let express = assembled_composite(&declarations, "M::ExpressOrderPlaced").key();
    assert!(declarations.types.composite_conforms(express, consumed));
}

fn money_record(name: &str, supertypes: &[&str]) -> serde_json::Value {
    let supertypes: Vec<_> = supertypes.iter().map(|node| id(node)).collect();
    declaration(
        name,
        "money",
        json!({
            "supertypes": supertypes,
            "fields": [field(name, &format!("{name}_minor"), "ix://quire/native/Integer")],
        }),
    )
}

/// Record value types with one declaration on two cycles: `A` is refused once
/// per cycle, `A` and `B` listing `[A, B]`, then `A` and `C` listing `[A, C]`.
#[trace("TC-1020", "FR-082-AC-15")]
#[test]
fn a_record_declaration_on_two_cycles_is_refused_once_per_cycle() {
    let document = mutate_k2(|document| {
        document["types"].as_array_mut().unwrap().extend([
            money_record("RecA", &["RecB", "RecC"]),
            money_record("RecB", &["RecA"]),
            money_record("RecC", &["RecA"]),
        ]);
    });
    assert_specialization_cycles(
        &document,
        &[
            ("RecA", &["RecA", "RecB"]),
            ("RecB", &["RecA", "RecB"]),
            ("RecA", &["RecA", "RecC"]),
            ("RecC", &["RecA", "RecC"]),
        ],
    );
}

/// A record cycle whose path order is not its key order: `B -> D -> C -> B`
/// lists `[B, D, C]`, refused at `B`, `C` and `D` in key order.
#[trace("TC-1020", "FR-082-AC-15")]
#[test]
fn a_record_cycle_lists_path_order_and_refuses_in_key_order() {
    let document = mutate_k2(|document| {
        document["types"].as_array_mut().unwrap().extend([
            money_record("RecB", &["RecD"]),
            money_record("RecC", &["RecB"]),
            money_record("RecD", &["RecC"]),
        ]);
    });
    let cycle = ["RecB", "RecD", "RecC"];
    assert_specialization_cycles(
        &document,
        &[("RecB", &cycle), ("RecC", &cycle), ("RecD", &cycle)],
    );
}

/// An event type cycle is a `specialization-cycle`, at each event type on it.
#[trace("TC-1020", "FR-082-AC-15")]
#[test]
fn an_event_type_cycle_is_a_specialization_cycle() {
    let document = mutate_k2(|document| {
        type_named_mut(document, "OrderPlaced")["supertypes"] = json!([id("ExpressOrderPlaced")]);
        document["types"].as_array_mut().unwrap().push(declaration(
            "ExpressOrderPlaced",
            "happened",
            json!({
                "supertypes": [id("OrderPlaced")],
                "occurrenceField": id("OrderPlaced/occurred_at"),
                "fields": [field("ExpressOrderPlaced", "promised_by", &id("Instant"))],
            }),
        ));
    });
    let cycle = ["ExpressOrderPlaced", "OrderPlaced"];
    assert_specialization_cycles(
        &document,
        &[("ExpressOrderPlaced", &cycle), ("OrderPlaced", &cycle)],
    );
}
