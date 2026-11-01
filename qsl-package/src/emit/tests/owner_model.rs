// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-383-AC-1, AC-14, AC-26 (QSpec FR-464 "owner_model", FR-464-AC-16;
//! TC-1061, TC-1062, TC-1063): a protocol clause's node-level `owner_model`
//! edge, the exact local model declaration node the clause's `over` input
//! references. Every shape is emitted by `emit_checked` and read back by
//! IR's strict reader (QSL's whole I2 read and IR's own); each mutation's
//! verdict is the reader's, not one the test computes.

use qsl_semantics::check::{CheckRefusal, ProtocolConstruct};
use quire_contract_model::{
    read_checked_package, CheckedPackageDispatchResult, CheckedPackageEvidence,
    CheckedPackageReadLimits, ExactTypeProjectionErrorCode, SelectedExactIdentities,
};

use super::population_bound::document;
use super::protocol_graph::{checked, flow_over, PROFILES};
use super::protocol_reference::{node_by_id, node_of, not_emitted};
use super::reachability::{lower_with, model_node, Lowered};
use super::*;

/// A clause over `over` with a static role on each of the two object types, so
/// both model declaration nodes are in the graph.
fn flow(over: &str) -> String {
    flow_over(
        over,
        "on origin",
        "role Buyer on M::Order;\nrole Seller on M::Person;",
    )
}

/// `text` emitted over the `acme/orders` document, read back by IR's strict
/// reader and by QSL's whole I2 read.
fn emitted(text: &str) -> Lowered {
    let lowered = lower_with(document(), PROFILES, text);
    assert!(
        lowered.ir.is_ok(),
        "{:?}\n{}",
        lowered.ir.err(),
        lowered.text
    );
    assert!(lowered.verified, "{}", lowered.text);
    lowered
}

/// The reference targets of `term`, in document order.
pub(super) fn targets(term: &Value, found: &mut Vec<Value>) {
    match term {
        Value::Object(members) => {
            if members.get("term") == Some(&json!("reference")) {
                found.push(members["target"].clone());
            }
            members.values().for_each(|member| targets(member, found));
        }
        Value::Array(items) => items.iter().for_each(|item| targets(item, found)),
        _ => {}
    }
}

/// The clause node's position in the wire's `nodes`.
fn clause_position(wire: &Value) -> usize {
    let clause = node_of(wire, "protocol", "protocol_clause");
    nodes(wire)
        .iter()
        .position(|node| node["node_id"] == clause["node_id"])
        .expect("the clause is in the graph")
}

/// IR's verdict on `envelope`: `admitted`, or `<code> at <path>`, with the
/// cause when the reader gives one.
pub(super) fn verdict(lowered: &Lowered, envelope: &Value) -> String {
    let mut evidence = CheckedPackageEvidence::new();
    for feature in lowered.wire["lock"]["required_features"]
        .as_array()
        .unwrap()
    {
        evidence.support_feature(feature.as_str().unwrap());
    }
    let document = document();
    evidence
        .insert_domain_package_document(package_identity(&document[..]), &document[..])
        .expect("a fresh evidence key");
    match read_checked_package(
        &jcs(envelope),
        CheckedPackageReadLimits::bounded(),
        &evidence,
    ) {
        CheckedPackageDispatchResult::AdmittedV2(_) => "admitted".to_owned(),
        CheckedPackageDispatchResult::Refused(refusal) => format!(
            "{:?} at {} ({:?})",
            refusal.code,
            refusal.loci[0]
                .path
                .as_ref()
                .map_or("-", |path| path.as_str()),
            refusal.cause
        ),
        other => panic!("not a V2 verdict: {other:?}"),
    }
}

/// Replaces the clause's exact `dependencies` with the body's reference
/// targets plus `owner`, once each in ascending structural-key order (QSpec FR-464
/// "dependencies").
pub(super) fn depend_on(envelope: &mut Value, position: usize, owner: &Value) {
    let graph = qsl_semantics::check::wire_id::WireGraph::of_package_value(envelope).unwrap();
    let node = &mut envelope["semantic_graph"]["nodes"][position];
    let mut found = vec![owner.clone()];
    targets(&node["body"], &mut found);
    found.sort_by_key(|value| graph.key_of_value(value).unwrap());
    found.dedup();
    node["dependencies"] = Value::Array(found);
}

/// FR-383-AC-1, AC-26: the clause names the object type its `over` input
/// references. The edge is a local node reference, a dependency of the clause
/// (once, in ascending order, with the body's targets), the only `owner_model`
/// of the graph, read back unchanged by IR's typed reader, and part of the
/// clause's structural owner edge.
#[trace("FR-383-AC-1", "FR-383-AC-26", "TC-1061")]
#[test]
fn a_clause_names_the_object_type_its_over_input_references() {
    let lowered = emitted(&flow("M::Order"));
    let wire = &lowered.wire;
    let clause = node_of(wire, "protocol", "protocol_clause");
    let owner = model_node(wire, "Order");
    assert_eq!(clause["owner_model"], owner["node_id"]);
    assert_eq!(owner["node_tag"], "model");

    let mut expected = vec![owner["node_id"].clone()];
    targets(&clause["body"], &mut expected);
    let graph = qsl_semantics::check::wire_id::WireGraph::of_package_value(wire).unwrap();
    expected.sort_by_key(|id| graph.key_of_value(id).expect("a structural dependency"));
    expected.dedup();
    assert_eq!(clause["dependencies"], Value::Array(expected));

    let carriers: Vec<&Value> = nodes(wire)
        .iter()
        .filter(|node| node.get("owner_model").is_some())
        .collect();
    assert_eq!(carriers, [clause], "only the clause carries the edge");

    let admitted = lowered.ir.unwrap();
    let typed = admitted
        .graph()
        .nodes
        .iter()
        .filter(|node| node.owner_model.is_some())
        .collect::<Vec<_>>();
    let [typed] = typed[..] else {
        panic!("one typed clause carries the edge: {typed:?}");
    };
    assert_eq!(
        serde_json::to_value(typed.owner_model.as_ref().unwrap()).unwrap(),
        owner["node_id"]
    );
    assert_eq!(
        serde_json::to_value(&typed.node_id).unwrap(),
        clause["node_id"]
    );
}

/// QSL-1377: model-intake identity rows are carried into the admitted IR
/// projection, and an identity-bearing reference refuses when the rows are
/// absent.
#[trace("FR-390-AC-17")]
#[test]
fn selected_model_identity_rows_back_reference_projection_and_missing_rows_refuse() {
    let lowered = emitted(&flow("M::Order"));
    let admitted = lowered.ir.as_ref().expect("IR admits the package");
    assert!(
        lowered.selected_exact_declarations.iter().any(|row| {
            row.selected_document.as_ref() == "acme/orders"
                && row.original_identity.as_ref() == "ix://acme/orders/Order"
                && row.form == quire_contract_model::SelectedExactDeclarationForm::ObjectType
        }),
        "the model intake stage minted the selected Order identity row"
    );
    assert!(lowered.selected_exact_fields.is_empty());
    let identities = lowered
        .selected_exact_identities
        .as_ref()
        .expect("the QSL checked graph validates intake rows against IR admission");

    let mut witnessed = false;
    let mut refused_without_rows = false;
    for node in &admitted.graph().nodes {
        if let Ok(projected) = admitted.project_exact_type(&node.node_id, identities, 64) {
            if matches!(projected.value_type(), quire_exact::ValueType::Reference(_)) {
                witnessed = true;
                let refusal = admitted
                    .project_exact_type(&node.node_id, &SelectedExactIdentities::empty(), 64)
                    .expect_err("a reference cannot project without selected identity rows");
                assert_eq!(
                    refusal.code,
                    ExactTypeProjectionErrorCode::UnavailableIdentityWitness
                );
                refused_without_rows = true;
                break;
            }
        }
    }
    assert!(
        witnessed,
        "the package has a reference projection backed by Order"
    );
    assert!(refused_without_rows, "the empty witness was refused by IR");
}

/// FR-383-AC-26: two clauses over different object types have distinct owner
/// edges, and the edge is not reconstructed from the role models: `Flow` over
/// `Order` and over `Person` have the same roles and differ in the edge, in
/// the clause's semantic body.
#[trace("FR-383-AC-14", "FR-383-AC-26", "TC-1062")]
#[test]
fn clauses_over_different_object_types_have_distinct_owner_edges() {
    let order = emitted(&flow("M::Order"));
    let person = emitted(&flow("M::Person"));
    let clause = |lowered: &Lowered| node_of(&lowered.wire, "protocol", "protocol_clause").clone();
    assert_eq!(
        clause(&person)["owner_model"],
        model_node(&person.wire, "Person")["node_id"]
    );
    assert_ne!(
        clause(&order)["owner_model"],
        clause(&person)["owner_model"]
    );
}

/// FR-383-AC-26, FR-464-AC-16 (TC-1063): each defect of the edge, over the
/// emitted package, is refused by IR's reader with the verdict QSpec fixes,
/// and each is refused where the reader first reaches it. The table compares
/// the reader's verdict, so a defect the reader admits fails the case.
#[trace("FR-383-AC-26", "TC-1063")]
#[test]
fn each_defect_of_the_edge_is_refused_where_the_reader_reaches_it() {
    let lowered = emitted(&flow("M::Order"));
    let wire = &lowered.wire;
    let position = clause_position(wire);
    let clause_path = format!("/semantic_graph/nodes/{position}");
    let owner = wire["semantic_graph"]["nodes"][position]["owner_model"].clone();
    let role = nodes(wire)
        .iter()
        .position(|node| node["semantic_form"] == "role")
        .expect("a role");
    let boolean = nodes(wire)
        .iter()
        .find(|node| node["node_tag"] == "scalar_type" && node["semantic_form"] == "boolean")
        .expect("the Boolean type")["node_id"]
        .clone();
    let absent = qsl_foundation::node_wire::node_key_to_json(
        &NodeKey::new("pkg", vec![PathSegment::Text("missing".into())], 0).unwrap(),
    )
    .unwrap();
    let external = json!({"package": "other", "node": owner});

    type Edit = Box<dyn Fn(&mut Value)>;
    let cases: Vec<(&str, Edit, String)> = vec![
        (
            "the edge is missing",
            Box::new(move |envelope| {
                envelope["semantic_graph"]["nodes"][position]
                    .as_object_mut()
                    .unwrap()
                    .remove("owner_model");
            }),
            format!("MalformedWire at {clause_path} (None)"),
        ),
        (
            "another node carries an edge",
            Box::new({
                let owner = owner.clone();
                move |envelope| {
                    envelope["semantic_graph"]["nodes"][role]["owner_model"] = owner.clone();
                }
            }),
            format!("MalformedWire at /semantic_graph/nodes/{role}/owner_model (None)"),
        ),
        (
            "the target is an external package key",
            Box::new({
                let external = external.clone();
                move |envelope| {
                    envelope["semantic_graph"]["nodes"][position]["owner_model"] = external.clone();
                }
            }),
            format!("MalformedWire at {clause_path}/owner_model (None)"),
        ),
        (
            "the target is not among the dependencies",
            Box::new({
                let owner = owner.clone();
                move |envelope| {
                    let dependencies = envelope["semantic_graph"]["nodes"][position]
                        ["dependencies"]
                        .as_array_mut()
                        .unwrap();
                    dependencies.retain(|dependency| *dependency != owner);
                }
            }),
            format!("InvalidSemanticGraph at {clause_path}/dependencies (None)"),
        ),
        (
            "the target is a node of a kind that is not a model",
            Box::new({
                let boolean = boolean.clone();
                move |envelope| {
                    envelope["semantic_graph"]["nodes"][position]["owner_model"] = boolean.clone();
                    depend_on(envelope, position, &boolean);
                }
            }),
            format!(
                "InvalidModelBinding at {clause_path}/owner_model (Some(MalformedDeclaration))"
            ),
        ),
        (
            "the target is no node of the graph",
            Box::new({
                let absent = absent.clone();
                move |envelope| {
                    envelope["semantic_graph"]["nodes"][position]["owner_model"] = absent.clone();
                    depend_on(envelope, position, &absent);
                }
            }),
            format!("MissingDeclaration at {clause_path}/owner_model (Some(MissingName))"),
        ),
    ];
    for (name, edit, expected) in cases {
        let mut envelope = wire.clone();
        edit(&mut envelope);
        let found = verdict(&lowered, &envelope);
        assert_ne!(found, "admitted", "{name}");
        assert_eq!(found, expected, "{name}");
    }
}

/// FR-383-AC-26: a clause whose `over` input is not a reference to an object
/// type of a selected model has no model declaration node to name, so the
/// protocol is refused typed (`unsupported_construct`/`not-yet-implemented`,
/// QSL-1147), once, and with the emission gate open writes no clause. The same
/// protocol over an object type checks and writes the clause.
#[trace("FR-383-AC-27", "TC-1063")]
#[test]
fn an_over_input_that_is_no_object_type_refuses_typed_and_writes_no_clause() {
    let clauses = |graph: &qsl_semantics::check::CheckedGraph| {
        graph
            .semantic_graph()
            .nodes()
            .filter(|node| node.semantic_form() == "protocol_clause")
            .count()
    };
    let written =
        checked(&flow("M::Order"), false).unwrap_or_else(|refusals| panic!("{refusals:?}"));
    assert_eq!(clauses(&written), 1);
    assert_eq!(ProtocolConstruct::NoOwnerModel.tracking(), "QSL-1147");
    for over in [
        "Boolean",
        "Integer",
        "Option<M::Order>",
        "Sequence<M::Order>",
    ] {
        let text = flow(over);
        let refusals: Vec<CheckRefusal> = checked(&text, false).expect_err(&text);
        assert_eq!(
            refusals
                .iter()
                .map(|refusal| &refusal.cause)
                .collect::<Vec<_>>(),
            [&not_emitted(ProtocolConstruct::NoOwnerModel)],
            "{over}"
        );
        let open = checked(&text, true).unwrap_or_else(|refusals| panic!("{over}: {refusals:?}"));
        assert_eq!(clauses(&open), 0, "{over} writes no clause node");
    }
}

/// FR-383-AC-26: the edge is the model declaration the clause's `over` names,
/// not one found from the roles: a clause over `Order` whose only role is on
/// `Person` still names `Order`, and `Order`'s node is the clause's
/// dependency though no role names it.
#[trace("FR-383-AC-26", "TC-1062")]
#[test]
fn the_edge_follows_over_and_not_the_roles() {
    let lowered = emitted(&flow_over(
        "M::Order",
        "on origin",
        "role Seller on M::Person;",
    ));
    let wire = &lowered.wire;
    let clause = node_of(wire, "protocol", "protocol_clause");
    let owner = model_node(wire, "Order");
    assert_eq!(clause["owner_model"], owner["node_id"]);
    assert!(clause["dependencies"]
        .as_array()
        .unwrap()
        .contains(&owner["node_id"]));
    let role = node_of(wire, "protocol", "role");
    let mut reached = Vec::new();
    targets(&role["body"], &mut reached);
    assert!(!reached.contains(&owner["node_id"]));
    assert_eq!(
        node_by_id(wire, &clause["owner_model"])["declaration"],
        owner["declaration"]
    );
}
