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
                    "identityFields": [id("Fulfilment/correlation_id")],
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

    let wrong_meaning = mutate_k2(|document| {
        document["types"].as_array_mut().unwrap().push(declaration(
            "ExpressFulfilment",
            "saga",
            json!({
                "identityFields": [id("OrderLifecycle/placed")],
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
    let refusals = normalized_refusals(&wrong_meaning, 22);
    assert_eq!(refusals.len(), 1);
    assert_malformed_at(
        &refusals[0],
        &id("ExpressFulfilment"),
        "TypeDefinition.supertypes[0]",
    );

    let cycle = mutate_k2(|document| {
        type_named_mut(document, "Fulfilment")["supertypes"] = json!([id("ExpressFulfilment")]);
        document["types"].as_array_mut().unwrap().push(declaration(
            "ExpressFulfilment",
            "saga",
            json!({
                "identityFields": [id("Fulfilment/correlation_id")],
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
    });
    let refusals = normalized_refusals(&cycle, 22);
    assert_eq!(refusals.len(), 2);
    assert!(
        refusals.iter().all(|refusal| {
            refusal.code == Code::InvalidModelBinding
                && refusal.cause.as_str() == "specialization-cycle"
        }),
        "{refusals:#?}"
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
                "multiplicity": {"lower": 0, "ordered": false, "unique": true},
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
            json!([relationship("Fulfilment", "store", "MissingTarget")]);
    });
    let refusals = read_refusals(&bad_target);
    assert_eq!(refusals.len(), 1);
    assert_eq!(refusals[0].code, Code::MissingDeclaration);
    assert!(
        refusals[0].detail.contains("MissingTarget"),
        "{refusals:#?}"
    );

    let bad_owner = mutate_k2(|document| {
        type_named_mut(document, "Money")["relationships"] =
            json!([relationship("Money", "owner", "Order")]);
    });
    let refusals = normalized_refusals(&bad_owner, 22);
    assert_eq!(refusals.len(), 1);
    assert_malformed_at(
        &refusals[0],
        &id("relationship/Money-owner-Order"),
        "TypeDefinition.relationships[0]",
    );
}

/// TC-235 M22's assembly half: the invocation parameter is a declared record
/// type owned by the operation signature. Runtime-value-path comparison for
/// `self.placed` versus `flags.placed` is an evaluator assertion and remains
/// outside this source/assembly test.
#[ignore = "expected red until QSL-729/#777 record/event arms land"]
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
