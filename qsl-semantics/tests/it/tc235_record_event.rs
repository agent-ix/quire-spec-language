// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-235 record-value and event vectors at the source/normalization boundary.
//!
//! Runtime clause truth and FR-207 path outcomes belong to the downstream
//! evaluator/replay boundary. `qsl-semantics` retains the authored clauses
//! and binds their owners; it does not parse or evaluate their text.

use super::*;

use qsl_semantics::model::domain_package::{
    ClauseRecord, DomainPackageRecord, FieldMemberRecord, RecordValueTypeRecord, ValueTypeRef,
};
use qsl_semantics::model::key::DeclarationKey;
use serde_json::json;

fn key(node: &str) -> DeclarationKey {
    DeclarationKey {
        package: PACKAGE.to_owned(),
        node: id(node),
    }
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

fn m10_completed_package(
    document: &serde_json::Value,
    expected_charges: usize,
) -> Result<DomainPackage, String> {
    let package = read_k2(document).map_err(|refusals| format!("reader refusal: {refusals:?}"))?;
    let (outcome, meter) = normalize_with_meter(&package, ModelNormalizationLimits::default());
    let actual_charges = meter
        .admitted_charges()
        .iter()
        .filter(|charge| **charge == ChargePoint::NormalizeRecord)
        .count();
    if actual_charges != expected_charges {
        return Err(format!(
            "normalize.record charges {actual_charges}, expected {expected_charges}"
        ));
    }
    match outcome {
        NormalizeOutcome::Completed(_) => Ok(package),
        other => Err(format!("expected completed normalization, got {other:?}")),
    }
}

fn m10_refusal_mismatches(
    document: &serde_json::Value,
    expected_charges: usize,
    expected_node: &str,
    expected_path: &str,
) -> Vec<String> {
    let package = match read_k2(document) {
        Ok(package) => package,
        Err(refusals) => return vec![format!("reader refused before normalization: {refusals:?}")],
    };
    let (outcome, meter) = normalize_with_meter(&package, ModelNormalizationLimits::default());
    let actual_charges = meter
        .admitted_charges()
        .iter()
        .filter(|charge| **charge == ChargePoint::NormalizeRecord)
        .count();
    let mut mismatches = Vec::new();
    if actual_charges != expected_charges {
        mismatches.push(format!(
            "normalize.record charges {actual_charges}, expected {expected_charges}"
        ));
    }
    let NormalizeOutcome::Refused(refusals) = outcome else {
        mismatches.push(
            "expected one malformed-declaration refusal, got completed normalization".to_owned(),
        );
        return mismatches;
    };
    if refusals.len() != 1 {
        mismatches.push(format!("expected one refusal, got {refusals:?}"));
        return mismatches;
    }
    let refusal = &refusals[0];
    if refusal.code != Code::InvalidModelBinding
        || refusal.cause.as_str() != "malformed-declaration"
    {
        mismatches.push(format!(
            "expected invalid_model_binding/malformed-declaration, got {refusal:?}"
        ));
    }
    // TC-235 specifies the public code/cause/locus. Different typed cause
    // variants may carry that same malformed-declaration result; the
    // detail path is the stable locus for causes without a node field.
    let node_matches = match &refusal.cause {
        ModelRefusalCause::IntakeMalformedDeclaration { node, .. }
        | ModelRefusalCause::FrameEntryMalformed { node, .. } => node == expected_node,
        ModelRefusalCause::MalformedDeclaration => refusal.detail.contains(expected_node),
        _ => false,
    };
    if !node_matches || !refusal.detail.contains(expected_path) {
        mismatches.push(format!(
            "expected node {expected_node} at {expected_path}, got {refusal:?}"
        ));
    }
    mismatches
}

/// TC-235 M16's semantic-model half: the process field, invariant, operation
/// parameter/effect, and population membership survive real intake and
/// normalization. Runtime truth for the two runs and operation outcomes are
/// asserted by the evaluator/replay layer.
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
/// TC-235 M02 adds a record-valued field and an object invariant that compares
/// two Money values. This crate checks the exact field and clause bindings;
/// the downstream evaluation vector distinguishes matching from differing
/// currency fields.
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
            "typeRef": id("Money"),
            "multiplicity": {
                "lower": 1,
                "upper": 1,
                "ordered": false,
                "unique": true,
            },
        }]);
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
            "typeRef": id("Money"),
            "multiplicity": {
                "lower": 1,
                "upper": 1,
                "ordered": false,
                "unique": true,
            },
        }]);
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
    let wrong_meaning_refusals = read_refusals(&wrong_meaning);
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
    assert_eq!(cycle_refusals.len(), 1);
    assert_eq!(cycle_refusals[0].code, Code::InvalidModelBinding);
    assert_eq!(cycle_refusals[0].cause.as_str(), "specialization-cycle");
    assert!(cycle_refusals[0]
        .detail
        .contains("[ix://test/orders/Money, ix://test/orders/TaxedMoney]"));
}
