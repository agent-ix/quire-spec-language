// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-092-AC-20: an emitted source-owned union carries one IR
//! `SelectedSourceUnionRow`, joined to the emitted union node, with the
//! members the check sealed and each member's payload nodes in position
//! order; IR admits the rows against the emitted package and projects the
//! union type through them.

use quire_contract_model::{
    CheckedPackageReadLimits, CheckedPackageV2, CheckedPackageV2ReadResult,
    ExactTypeProjectionErrorCode, SelectedExactIdentities,
};

use super::source_enum_rows::node_with_id;
use super::variant_types::{emitted_enums, UNIT};
use super::*;

pub(super) const SHAPE: &str = "union Shape { Empty, Circle(Integer), Rect(Integer, Integer) }\n\
    function rect using v(): Shape pure { Shape::Rect(2, 3) }\n";

/// The package of `declarations`, its emission and the emission's wire.
fn emitted(declarations: &str) -> (CheckedPackage, Emission, Value) {
    let (_, package) = package_from_text(declarations);
    let emission = emit_checked(&package).expect("the package emits");
    assert_eq!(emission.omitted, []);
    let wire = wire(&emission);
    (package, emission, wire)
}

/// The composite declaration `name` of `package`'s type environment.
fn declaration<'p>(package: &'p CheckedPackage, name: &str) -> &'p CompositeDeclaration {
    package
        .graph()
        .scope()
        .types()
        .composites()
        .find(|declaration| declaration.name() == name)
        .unwrap_or_else(|| panic!("{name} is declared"))
}

/// IR's admission of the emitted bytes.
fn admit_bytes(emission: &Emission, wire: &Value) -> CheckedPackageV2 {
    let mut evidence = CheckedPackageEvidence::new();
    for feature in wire["lock"]["required_features"].as_array().unwrap() {
        evidence.support_feature(feature.as_str().unwrap());
    }
    match CheckedPackageV2::read(
        emission.package.bytes(),
        CheckedPackageReadLimits::bounded(),
        &evidence,
    ) {
        CheckedPackageV2ReadResult::Admitted(package) => *package,
        other => panic!("IR admits the emitted package: {other:?}"),
    }
}

#[trace("FR-092-AC-20", "TC-1021")]
#[test]
fn a_source_union_emits_its_declaration_member_and_payload_rows() {
    let (package, emission, wire) = emitted(SHAPE);
    let [row] = emission.selected_source_union_rows() else {
        panic!(
            "one source union row: {:?}",
            emission.selected_source_union_rows()
        );
    };

    let node = node_with_id(&wire, &row.structural_node);
    assert_eq!(
        (&node["node_tag"], &node["semantic_form"]),
        (&json!("composite_type"), &json!("union"))
    );
    assert_eq!(
        node["owner"],
        json!({"kind": "source", "authority": "a", "identity": "u"})
    );
    assert_eq!(
        (row.source_authority.as_ref(), row.source_identity.as_ref()),
        ("a", "u")
    );
    assert_eq!(row.qualified_declaration, [Box::from("Shape")]);

    // The type handle is the checked declaration's key; the settled key is
    // the union's node, the one every sealed member names.
    let shape = declaration(&package, "Shape");
    assert_eq!(row.type_handle, shape.key());
    assert_eq!(
        node_id_of(&wire, &row.settled_key),
        serde_json::to_value(&row.structural_node).expect("a node id serializes")
    );

    let CompositeShape::Union(declared) = shape.shape() else {
        panic!("Shape is a union");
    };
    let identifiers: Vec<&str> = row
        .members
        .iter()
        .map(|member| member.member.identifier())
        .collect();
    assert_eq!(identifiers, ["Empty", "Circle", "Rect"]);
    let payloads: Vec<&[ValueType]> = row.members.iter().map(|m| m.payload.as_slice()).collect();
    assert_eq!(
        payloads,
        [
            &[][..],
            &[ValueType::Integer][..],
            &[ValueType::Integer, ValueType::Integer][..]
        ]
    );
    for (member, declared) in row.members.iter().zip(declared) {
        assert_eq!(
            Some(&member.member),
            declared.member(),
            "{} is the sealed member the check minted",
            member.member.identifier()
        );
        assert_eq!(member.member.declaration(), &row.settled_key);
        assert_eq!(member.payload_nodes.len(), member.payload.len());
        for payload in &member.payload_nodes {
            let payload = node_with_id(&wire, payload);
            assert_eq!(
                (&payload["node_tag"], &payload["semantic_form"]),
                (&json!("scalar_type"), &json!("integer"))
            );
        }
    }

    // The payload ids are the references the union node's member bindings
    // hold, in position order.
    let bound: Vec<Vec<Value>> = node["body"]["members"]
        .as_array()
        .expect("the union body lists its members")
        .iter()
        .map(|member| {
            member["value"]["members"]
                .as_array()
                .expect("a member binds its payload group")
                .iter()
                .map(|reference| reference["target"].clone())
                .collect()
        })
        .collect();
    let rowed: Vec<Vec<Value>> = row
        .members
        .iter()
        .map(|member| {
            member
                .payload_nodes
                .iter()
                .map(|id| serde_json::to_value(id).expect("a node id serializes"))
                .collect()
        })
        .collect();
    assert_eq!(rowed, bound);
}

#[trace("FR-092-AC-20", "TC-1021")]
#[test]
fn a_recursive_union_row_names_the_payload_node_that_reaches_its_own_node() {
    let (_, emission, wire) = emitted("union Chain { End, Link(Integer, Option<Chain>) }\n");
    let [row] = emission.selected_source_union_rows() else {
        panic!("one source union row");
    };
    let link = &row.members[1];
    assert_eq!(link.member.identifier(), "Link");
    assert_eq!(link.payload_nodes.len(), 2);
    let optional = node_with_id(&wire, &link.payload_nodes[1]);
    assert_eq!(optional["semantic_form"], json!("option"));
    assert_eq!(
        optional["body"]["members"][0]["target"],
        serde_json::to_value(&row.structural_node).expect("a node id serializes"),
        "the optional payload names the union node itself"
    );
    assert_eq!(
        link.payload[1],
        ValueType::Option(std::sync::Arc::new(ValueType::Composite(
            row.type_handle.clone()
        )))
    );
}

#[trace("FR-092-AC-20", "TC-1021")]
#[test]
fn mutually_recursive_unions_each_get_a_row_under_their_type_handle_and_settled_key() {
    let (package, emission, wire) = emitted(
        "union Left { Stop, Go(Option<Right>) }\n\
         union Right { Halt, Move(Option<Left>) }\n\
         function f using v(l: Left): Boolean pure { true }\n",
    );
    let rows = emission.selected_source_union_rows();
    assert_eq!(rows.len(), 2);
    let types = package.graph().scope().types();
    for name in ["Left", "Right"] {
        let handle = declaration(&package, name).key();
        let row = rows
            .iter()
            .find(|row| row.type_handle == handle)
            .unwrap_or_else(|| panic!("{name} has a row under its type handle"));
        assert_eq!(
            types.union_key(handle.clone()),
            Some(row.settled_key.clone())
        );
        // QSL issues the same key as handle and settled key for every checked
        // union, so swapping the two fields in the row is an accepted
        // equivalent mutant; the check above pins the relation.
        assert_eq!(
            node_id_of(&wire, &row.settled_key),
            serde_json::to_value(&row.structural_node).expect("a node id serializes")
        );
        assert!(row
            .members
            .iter()
            .all(|member| member.member.declaration() == &row.settled_key));
    }
}

#[trace("FR-092-AC-20", "TC-1021")]
#[test]
fn every_emitted_union_node_has_one_row_and_other_units_have_none() {
    let (_, emission, wire) = emitted(
        "union Shape { Empty, Circle(Integer) }\n\
         union Pick { A, B(Boolean) }\n\
         function f using v(s: Shape, p: Pick): Boolean pure { true }\n",
    );
    let mut emitted_unions: Vec<Value> = nodes(&wire)
        .iter()
        .filter(|node| node["semantic_form"] == "union")
        .map(|node| node["node_id"].clone())
        .collect();
    assert_eq!(emitted_unions.len(), 2);
    let mut rowed: Vec<Value> = emission
        .selected_source_union_rows()
        .iter()
        .map(|row| serde_json::to_value(&row.structural_node).expect("a node id serializes"))
        .collect();
    rowed.sort_by_key(Value::to_string);
    emitted_unions.sort_by_key(Value::to_string);
    assert_eq!(rowed, emitted_unions);
    assert!(
        emission
            .selected_source_union_rows()
            .windows(2)
            .all(|pair| pair[0].settled_key < pair[1].settled_key),
        "rows are in ascending order of their union nodes' keys"
    );

    let (_, plain, _) = emitted("function f using v(): Boolean pure { true }\n");
    assert_eq!(plain.selected_source_union_rows(), []);
    let enums = emitted_enums(UNIT);
    assert_eq!(enums.selected_source_union_rows(), []);
}

#[trace("FR-092-AC-20", "TC-1021")]
#[test]
fn ir_projects_a_union_type_through_the_emitted_rows_and_refuses_without_them() {
    let (_, emission, wire) = emitted(SHAPE);
    let admitted = admit_bytes(&emission, &wire);
    let [row] = emission.selected_source_union_rows() else {
        panic!("one source union row");
    };

    let identities = SelectedExactIdentities::empty()
        .with_source_union_rows(&admitted, [row.clone()])
        .expect("IR joins the emitted union row");
    let projected = admitted
        .project_exact_type(&row.structural_node, &identities, 64)
        .expect("a union with its rows projects");
    assert_eq!(
        projected.value_type(),
        &ValueType::Composite(row.type_handle.clone())
    );

    // A bare union key is no witness.
    let refusal = admitted
        .project_exact_type(&row.structural_node, &SelectedExactIdentities::empty(), 64)
        .expect_err("a union cannot project without its rows");
    assert_eq!(
        refusal.code,
        ExactTypeProjectionErrorCode::UnavailableIdentityWitness
    );

    // A row that misorders the members, or drops a payload node, is refused
    // before any projection.
    let mut reordered = row.clone();
    reordered.members.swap(1, 2);
    let mut truncated = row.clone();
    truncated.members[2].payload_nodes.pop();
    for bad in [reordered, truncated] {
        let refusal = SelectedExactIdentities::empty()
            .with_source_union_rows(&admitted, [bad])
            .expect_err("a row that disagrees with the admitted union is refused");
        assert_eq!(
            refusal.code,
            ExactTypeProjectionErrorCode::MismatchedIdentityWitness
        );
    }
}

/// The declared members, emitted ids and payload lists of `Shape`.
type GuardInputs = (
    Vec<quire_semantic_value::declaration::UnionMemberDeclaration>,
    HashMap<NodeKey, CheckedNodeId>,
    Vec<(String, Vec<NodeKey>)>,
);

/// The declared members of `Shape` with the payload lists its union node
/// holds, for driving `union_member_rows` with lists that disagree.
fn guard_inputs(package: &CheckedPackage, emission: &Emission) -> GuardInputs {
    let CompositeShape::Union(declared) = declaration(package, "Shape").shape() else {
        panic!("Shape is a union");
    };
    let node = package
        .graph()
        .semantic_graph()
        .node(emission.selected_source_union_rows()[0].settled_key.clone())
        .expect("the union node");
    let emitted: HashMap<NodeKey, CheckedNodeId> = package
        .graph()
        .semantic_graph()
        .nodes()
        .map(|node| (node.key(), node_id(node.key())))
        .collect();
    let payloads = node
        .union_member_payload_nodes()
        .expect("a union body")
        .into_iter()
        .map(|(name, nodes)| (name.to_owned(), nodes))
        .collect();
    (declared.to_vec(), emitted, payloads)
}

#[trace("FR-092-AC-20", "TC-1021")]
#[test]
fn a_payload_list_that_disagrees_with_the_declaration_is_a_writer_fault() {
    let (package, emission, _) = emitted(SHAPE);
    let (declared, emitted, payloads) = guard_inputs(&package, &emission);
    let sealed: Vec<&quire_exact::UnionMember> = declared
        .iter()
        .map(|m| m.member().expect("sealed"))
        .collect();
    let call = |payloads: &[(String, Vec<NodeKey>)], emitted: &HashMap<NodeKey, CheckedNodeId>| {
        crate::emit::union_member_rows(
            &declared,
            &sealed,
            payloads
                .iter()
                .map(|(n, p)| (n.as_str(), p.clone()))
                .collect(),
            emitted,
        )
    };
    assert_eq!(
        call(&payloads, &emitted)
            .expect("the agreeing list joins")
            .len(),
        3
    );

    let mut fewer_members = payloads.clone();
    fewer_members.pop();
    assert_eq!(
        call(&fewer_members, &emitted),
        Err(EmitRefusal::SourceMapInvariant)
    );

    let mut wrong_name = payloads.clone();
    wrong_name[1].0 = "Other".to_owned();
    assert_eq!(
        call(&wrong_name, &emitted),
        Err(EmitRefusal::SourceMapInvariant)
    );

    let mut short_payload = payloads.clone();
    short_payload[2].1.pop();
    assert_eq!(
        call(&short_payload, &emitted),
        Err(EmitRefusal::SourceMapInvariant)
    );

    let mut long_payload = payloads.clone();
    long_payload[0].1.push(payloads[1].1[0].clone());
    assert_eq!(
        call(&long_payload, &emitted),
        Err(EmitRefusal::SourceMapInvariant)
    );

    assert_eq!(
        call(&payloads, &HashMap::new()),
        Err(EmitRefusal::SourceMapInvariant),
        "a payload node that was not emitted"
    );
}
