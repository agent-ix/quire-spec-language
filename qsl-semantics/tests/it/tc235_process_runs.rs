// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-235 process and cross-cutting runs at the source/normalization
//! boundary: M07 (process members), M10 (type, member, frame and population
//! references), M16 and M20 (process semantic halves) and M21 (relationships).
//!
//! Every run compares the typed result: refusal code, cause tag, refused node,
//! member path (or the reader's document pointer where the cause carries one)
//! and the exact `normalize.record` charge count. Detail text is never read.

use super::*;

use ix_trace_rs::trace;
use qsl_semantics::model::dispatch::GeneralizationClosure;
use qsl_semantics::model::domain_package::{
    DomainPackageRecord, OperationMemberRecord, ValueTypeRef,
};
use qsl_semantics::model::key::DeclarationKey;
use qsl_semantics::model::population::{
    admit_binding, all_instances, AdmissionMeter, AdmissionOutcome, AllInstancesOutcome,
    PopulationAdmissionLimits, PopulationDocument, PopulationMember, ReferenceKey,
};
use serde_json::json;

fn key(node: &str) -> DeclarationKey {
    package_key(node)
}

/// One refusal reduced to the typed fields a TC-235 row states.
#[derive(Debug, PartialEq)]
struct Seen {
    code: Code,
    cause: &'static str,
    node: String,
    /// The member path, where the cause carries one.
    member: Option<MemberPath>,
    /// The absent node (`missing-name`) or the frame entry that names the
    /// wrong meaning.
    other: Option<String>,
    /// The reader's document pointer, where the cause carries one.
    pointer: Option<String>,
}

fn seen(refusal: &ModelRefusal) -> Seen {
    let (node, member, other, pointer) = match &refusal.cause {
        ModelRefusalCause::IntakeMalformedDeclaration { node, member, .. } => {
            (node.clone(), Some(member.clone()), None, None)
        }
        ModelRefusalCause::UnsupportedDeclarationForm { node, member, .. } => {
            (node.clone(), Some(member.clone()), None, None)
        }
        ModelRefusalCause::ConflictingBinding { key, member } => {
            (key.node.clone(), member.clone(), None, None)
        }
        ModelRefusalCause::IntakeMissingDeclaration {
            node,
            reference,
            path,
            member,
            ..
        } => (
            node.clone(),
            Some(member.clone()),
            Some(reference.clone()),
            Some(path.as_str().to_owned()),
        ),
        ModelRefusalCause::FrameEntryMalformed {
            node,
            entry,
            member,
            ..
        } => (
            node.clone(),
            Some(member.clone()),
            Some(entry.clone()),
            None,
        ),
        other => panic!("a cause outside the TC-235 process/cross-cutting rows: {other:?}"),
    };
    Seen {
        code: refusal.code,
        cause: refusal.cause.as_str(),
        node,
        member,
        other,
        pointer,
    }
}

fn malformed(node: &str, path: &str) -> Seen {
    Seen {
        code: Code::InvalidModelBinding,
        cause: "malformed-declaration",
        node: node.to_owned(),
        member: Some(member(path)),
        other: None,
        pointer: None,
    }
}

fn conflicting(node: &str, path: &str) -> Seen {
    Seen {
        code: Code::InvalidModelBinding,
        cause: "conflicting-binding",
        node: node.to_owned(),
        member: Some(member(path)),
        other: None,
        pointer: None,
    }
}

fn missing(node: &str, absent: &str, pointer: &str, member_path: &str) -> Seen {
    Seen {
        code: Code::MissingDeclaration,
        cause: "missing-name",
        node: node.to_owned(),
        member: Some(member(member_path)),
        other: Some(absent.to_owned()),
        pointer: Some(pointer.to_owned()),
    }
}

/// `document` refuses after exactly `charges` `normalize.record` charges with
/// exactly `expected`, in order.
fn assert_refused(document: &Value, charges: usize, expected: &[Seen]) {
    let actual: Vec<_> = normalized_refusals(document, charges)
        .iter()
        .map(seen)
        .collect();
    assert_eq!(actual, expected);
}

/// `document` admits and normalizes after exactly `charges` charges.
fn admitted(document: &Value, charges: usize) -> DomainPackage {
    let package = read_k2(document).expect("the run's package reads");
    record_charges(&package, charges);
    package
}

fn operation_of<'a>(package: &'a DomainPackage, node: &str) -> &'a OperationMemberRecord {
    package
        .records
        .iter()
        .find_map(|record| match record {
            DomainPackageRecord::OperationMember(operation) if operation.key == key(node) => {
                Some(operation)
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("missing operation {node}"))
}

fn position_of_type(document: &Value, node: &str) -> usize {
    document["types"]
        .as_array()
        .unwrap()
        .iter()
        .position(|ty| ty["identity"] == id(node))
        .expect("the type is declared")
}

/// A closed population of `members`.
fn population(name: &str, members: &[&str]) -> Value {
    json!({
        "identity": id(name),
        "displayName": id(name),
        "kind": {"module": BUSINESS, "name": "population"},
        "members": members.iter().map(|member| id(member)).collect::<Vec<_>>(),
        "extent": "closed",
        "origin": origin(name),
    })
}

/// A relationship `owner/name` from `owner` to `target`, keyed as FR-154
/// keys a relationship: `relationship/<Owner>-<name>-<Target>`.
fn relationship(owner: &str, name: &str, target: &str) -> Value {
    let node = format!("relationship/{owner}-{name}-{target}");
    json!({
        "identity": id(&node),
        "category": "structural",
        "composite": false,
        "direction": "source-to-target",
        "origin": {"source": {
            "sourceIdentity": id(&node),
            "path": "tc235_process_runs.rs",
            "startLine": 1,
            "startColumn": 1,
            "endLine": 1,
            "endColumn": 1,
        }},
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

/// M07, every run: the refused process names its typed member path (the
/// existing binder test pins the node, charge count and the two runs whose
/// cause carries the path).
///
/// Mutation used (derived by reading `model/normalize.rs`, not executed), one
/// per run: removing the process `identity_fields` emptiness refusal admits
/// run 1 so `normalized_refusals` panics on the `Refused` let-else; moving the
/// empty-`steps` refusal off `TypeDefinition.steps` flips run 2's member
/// path; removing the duplicate-step refusal admits run 3; removing the
/// non-event `consumes` refusal admits run 4 and removing the `emits` event
/// check admits run 5; dropping the reader's missing-name mapping flips run
/// 6's code to `invalid_model_binding`.
#[trace("QSpec-TC-235", "QSpec-FR-208-AC-6", "QSpec-FR-208-AC-10")]
#[test]
fn tc235_m07_every_process_refusal_names_its_typed_member_path() {
    let fulfilment = id("Fulfilment");

    let mut document = k2();
    type_named_mut(&mut document, "Fulfilment")["identityFields"] = json!([]);
    assert_refused(
        &document,
        21,
        &[malformed(&fulfilment, "TypeDefinition.identityFields")],
    );

    let mut document = k2();
    type_named_mut(&mut document, "Fulfilment")["steps"] = json!([]);
    assert_refused(
        &document,
        21,
        &[malformed(&fulfilment, "TypeDefinition.steps")],
    );

    let mut document = k2();
    type_named_mut(&mut document, "Fulfilment")["steps"]
        .as_array_mut()
        .unwrap()
        .push(json!({
            "identity": id("Fulfilment/reserve_stock"),
            "name": "reserve_stock",
            "stepKind": "command",
            "consumes": [],
            "emits": [],
            "origin": origin("Fulfilment/reserve_stock"),
        }));
    assert_refused(
        &document,
        21,
        &[conflicting(&fulfilment, "TypeDefinition.steps[3]")],
    );

    let mut document = k2();
    type_named_mut(&mut document, "Fulfilment")["steps"][0]["consumes"] = json!([id("Money")]);
    assert_refused(
        &document,
        21,
        &[malformed(
            &fulfilment,
            "TypeDefinition.steps[0].consumes[0]",
        )],
    );

    let mut document = k2();
    type_named_mut(&mut document, "Fulfilment")["steps"][1]["emits"] = json!([id("Order")]);
    assert_refused(
        &document,
        21,
        &[malformed(&fulfilment, "TypeDefinition.steps[1].emits[0]")],
    );

    // The missing event is reported at the reader's document pointer of the
    // reference, beside the absent node.
    let mut document = k2();
    type_named_mut(&mut document, "Fulfilment")["steps"][1]["emits"] = json!([id("StockReserved")]);
    let at = position_of_type(&document, "Fulfilment");
    assert_refused(
        &document,
        21,
        &[missing(
            &fulfilment,
            &id("StockReserved"),
            &format!("/types/{at}/steps/1/emits/0"),
            "TypeDefinition.steps[1].emits[0]",
        )],
    );
}

/// M07 runs 7 and 8: an operation and an invariant of a process are bound to
/// the process as their owner, each after 22 charges.
#[trace("QSpec-TC-235", "QSpec-FR-208-AC-6", "QSpec-FR-208-AC-10")]
#[test]
fn tc235_m07_process_operation_and_invariant_bind_to_the_process() {
    let mut document = k2();
    type_named_mut(&mut document, "Fulfilment")["operations"]
        .as_array_mut()
        .unwrap()
        .push(operation("Fulfilment", "restart"));
    let package = admitted(&document, 22);
    assert_eq!(
        operation_of(&package, "Fulfilment/restart").owner,
        key("Fulfilment")
    );

    let mut document = k2();
    type_named_mut(&mut document, "Fulfilment")["clauses"] = json!([clause(
        "Fulfilment",
        "Bounded",
        "self.correlation_id = self.correlation_id"
    )]);
    let package = admitted(&document, 22);
    let bounded = package
        .records
        .iter()
        .find_map(|record| match record {
            DomainPackageRecord::Clause(clause) if clause.key == key("Fulfilment/Bounded") => {
                Some(clause)
            }
            _ => None,
        })
        .expect("Bounded is a retained clause");
    assert_eq!(bounded.owner, key("Fulfilment"));
    assert_eq!(bounded.text, "self.correlation_id = self.correlation_id");
}

/// M10 runs 1-6: a field typed by a namespace, persistence interface, event
/// type, state machine or process, and an object type specialising a
/// namespace, refuse at the reference; a record value type is admitted.
///
/// Mutation used (derived by reading the object-type field arm of
/// `model/normalize.rs`, not executed): admitting an event, state-machine,
/// process, persistence or namespace `typeRef` flips each of runs 1-5 to an
/// admitted package, so `normalized_refusals` panics; treating a record value
/// type as a wrong meaning flips the control to a refusal. Dropping the
/// object-type `supertypes` meaning check admits run 6.
#[trace("QSpec-TC-235", "QSpec-FR-208-AC-9")]
#[test]
fn tc235_m10_field_and_supertype_references_name_object_or_value_types_only() {
    for target in [
        "OrderManagement",
        "OrderStore",
        "OrderPlaced",
        "OrderLifecycle",
        "Fulfilment",
    ] {
        let document = mutate_k2(|document| {
            type_named_mut(document, "Order")["fields"]
                .as_array_mut()
                .unwrap()
                .push(field("Order", "ref", &id(target)));
        });
        assert_refused(
            &document,
            22,
            &[malformed(&id("Order/ref"), "Field.typeRef")],
        );
    }

    let document = mutate_k2(|document| {
        type_named_mut(document, "Order")["fields"]
            .as_array_mut()
            .unwrap()
            .push(field("Order", "ref", &id("Money")));
    });
    let package = admitted(&document, 22);
    let field = package
        .records
        .iter()
        .find_map(|record| match record {
            DomainPackageRecord::FieldMember(field) if field.key == key("Order/ref") => Some(field),
            _ => None,
        })
        .expect("Order/ref is a retained field");
    assert_eq!(field.value_type, ValueTypeRef::Package(key("Money")));

    let document = mutate_k2(|document| {
        type_named_mut(document, "Order")["supertypes"] = json!([id("OrderManagement")]);
    });
    assert_refused(
        &document,
        21,
        &[malformed(&id("Order"), "TypeDefinition.supertypes[0]")],
    );
}

/// M10 runs 7 and 8: a relationship whose target end names a namespace, and
/// a relationship owned by a state machine, refuse at their typed loci.
///
/// Mutation used (derived by reading the relationship arm of
/// `model/normalize.rs`, not executed): dropping the target-meaning check
/// admits run 7; dropping the owner-meaning check admits run 8.
#[trace("QSpec-TC-235", "QSpec-FR-208-AC-9")]
#[test]
fn tc235_m10_relationships_reach_object_types_only() {
    let document = mutate_k2(|document| {
        type_named_mut(document, "Order")["relationships"] =
            json!([relationship("Order", "scope", "OrderManagement")]);
    });
    assert_refused(
        &document,
        22,
        &[malformed(
            &id("relationship/Order-scope-OrderManagement"),
            "Relationship.target",
        )],
    );

    let document = mutate_k2(|document| {
        type_named_mut(document, "OrderLifecycle")["relationships"] =
            json!([relationship("OrderLifecycle", "next", "Order")]);
    });
    assert_refused(
        &document,
        22,
        &[malformed(
            &id("relationship/OrderLifecycle-next-Order"),
            "TypeDefinition.relationships[0]",
        )],
    );
}

/// M10 runs 9-12: a population admits a process as a member type and refuses
/// a namespace; an operation frame admits a process in `creates` and refuses a
/// persistence interface.
///
/// Mutation used (derived by reading the population and frame arms of
/// `model/normalize.rs`, not executed): rejecting a process member type flips
/// run 9 to a refusal; admitting a namespace member type flips run 10; the
/// frame `creates` meaning check dropped flips run 12 to an admission.
#[trace("QSpec-TC-235", "QSpec-FR-208-AC-9")]
#[test]
fn tc235_m10_population_members_and_frame_creates_name_object_types_only() {
    let document = mutate_k2(|document| {
        document["populations"] = json!([population("PA", &["Fulfilment"])]);
    });
    let package = admitted(&document, 22);
    let record = package
        .records
        .iter()
        .find_map(|record| match record {
            DomainPackageRecord::Population(population) if population.key == key("PA") => {
                Some(population)
            }
            _ => None,
        })
        .expect("PA is retained");
    assert_eq!(record.member_types, [key("Fulfilment")]);

    let document = mutate_k2(|document| {
        document["populations"] = json!([population("PA", &["OrderManagement"])]);
    });
    assert_refused(
        &document,
        22,
        &[malformed(&id("PA"), "PopulationDefinition.members[0]")],
    );

    let creating = |created: &str| {
        mutate_k2(|document| {
            let mut open = operation("Order", "open");
            open["frame"]["creates"] = json!([id(created)]);
            type_named_mut(document, "Order")["operations"] = json!([open]);
        })
    };
    let package = admitted(&creating("Fulfilment"), 22);
    assert_eq!(
        operation_of(&package, "Order/open").effect.creates,
        [key("Fulfilment")]
    );

    // The frame entry carries the operation, the entry and its locus.
    let refusals = normalized_refusals(&creating("OrderStore"), 22);
    assert_eq!(
        refusals.iter().map(seen).collect::<Vec<_>>(),
        [Seen {
            code: Code::InvalidModelBinding,
            cause: "malformed-declaration",
            node: id("Order/open"),
            member: Some(member("Operation.frame.creates[0]")),
            other: Some(id("OrderStore")),
            pointer: None,
        }]
    );
}

/// M20 run 1, population half: a run of the process subtype is a member of a
/// population of the supertype, in the universe of `Fulfilment`, and is
/// selected by `allInstances<Fulfilment>`.
///
/// Mutation used (derived by reading `admit_binding` and `all_instances` in
/// `model/population.rs`, not executed): dropping the subtype walk from the
/// member-type check refuses `r3` at admission, so the `Admitted` let-else
/// panics; selecting only members whose own type equals the queried type
/// drops `r3` from the selected set, flipping the final assertion.
#[trace("QSpec-TC-235", "QSpec-FR-208-AC-12")]
#[test]
fn tc235_m20_run1_a_subtype_run_is_a_member_of_the_supertype_population() {
    let document = mutate_k2(|document| {
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
        document["populations"] = json!([population("PF", &["Fulfilment"])]);
    });
    let package = read_k2(&document).expect("M20 run 1 package reads");
    let view = record_charges(&package, 23);
    let universe = view
        .object_universe_of(&key("Fulfilment"))
        .expect("Fulfilment has an object universe")
        .identity();
    assert_eq!(
        view.object_universe_of(&key("ExpressFulfilment"))
            .map(|universe| universe.identity()),
        Some(universe),
        "the subtype shares the supertype's universe"
    );
    let express = view.type_identities()[&key("ExpressFulfilment")];

    let mut admission = AdmissionMeter::new(PopulationAdmissionLimits::UNLIMITED);
    let binding = match admit_binding(
        &view,
        &PopulationDocument {
            model_identity: PACKAGE.to_owned(),
            members: vec![PopulationMember {
                object: "r3".to_owned(),
                type_identity: key("ExpressFulfilment"),
                field_values: Vec::new(),
            }],
        },
        &key("PF"),
        GeneralizationClosure::Closed,
        Some(1),
        &mut admission,
    ) {
        AdmissionOutcome::Admitted(binding) => binding,
        other => panic!("r3 is admitted into PF, got {other:?}"),
    };
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
    let AllInstancesOutcome::Completed(selected) = all_instances(
        &binding,
        &key("Fulfilment"),
        &mut quire_exact::Meter::new(unlimited),
    ) else {
        panic!("allInstances<Fulfilment> completes");
    };
    assert_eq!(
        selected.members().iter().collect::<Vec<_>>(),
        [&ReferenceKey {
            universe,
            type_identity: express,
            object: "r3".to_owned(),
        }]
    );
}
