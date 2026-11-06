// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-092-AC-22 and FR-092-AC-23: an emitted source-owned tuple carries one
//! IR `SelectedSourceTupleRow` whose positions are in declaration order with
//! the final type node and checked type of each, and an emitted source enum
//! carries one `SelectedSourceEnumShapeRow` holding the checked variant shape
//! joined to the final declaration and case nodes. IR joins the tuple rows
//! and refuses a row that permutes, drops, adds or re-points a position.

use quire_contract_model::{
    CheckedPackageReadLimits, CheckedPackageV2, CheckedPackageV2ReadResult,
    ExactTypeProjectionErrorCode, SelectedExactIdentities, SelectedSourceTupleRow,
};

use super::source_enum_rows::node_with_id;
use super::*;

const TUPLES: &str = "tuple Pair(Integer, Boolean);\n\
    tuple Triple(Pair, Option<Integer>, Boolean);\n\
    function f using v(t: Triple): Boolean pure { true }\n";

fn emitted(declarations: &str) -> (CheckedPackage, Emission, Value) {
    let (_, package) = package_from_text(declarations);
    let emission = emit_checked(&package).expect("the package emits");
    assert_eq!(emission.omitted, []);
    let wire = wire(&emission);
    (package, emission, wire)
}

fn declaration<'p>(package: &'p CheckedPackage, name: &str) -> &'p CompositeDeclaration {
    package
        .graph()
        .scope()
        .types()
        .composites()
        .find(|declaration| declaration.name() == name)
        .unwrap_or_else(|| panic!("{name} is declared"))
}

fn row_of<'e>(
    package: &CheckedPackage,
    emission: &'e Emission,
    name: &str,
) -> &'e SelectedSourceTupleRow {
    let key = declaration(package, name).key();
    emission
        .selected_source_tuple_rows()
        .iter()
        .find(|row| row.type_handle == key)
        .unwrap_or_else(|| panic!("{name} has a row"))
}

fn id_json(id: &CheckedNodeId) -> Value {
    serde_json::to_value(id).expect("a node id serializes")
}

fn admit(emission: &Emission, wire: &Value) -> CheckedPackageV2 {
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

#[trace("FR-092-AC-22", "TC-1021")]
#[test]
fn a_source_tuple_emits_its_declaration_and_ordered_position_rows() {
    let (package, emission, wire) = emitted(TUPLES);
    assert_eq!(emission.selected_source_tuple_rows().len(), 2);
    let row = row_of(&package, &emission, "Triple");

    let node = node_with_id(&wire, &row.structural_node);
    assert_eq!(
        (&node["node_tag"], &node["semantic_form"]),
        (&json!("composite_type"), &json!("tuple"))
    );
    assert_eq!(
        node["owner"],
        json!({"kind": "source", "authority": "a", "identity": "u"})
    );
    assert_eq!(
        (row.source_authority.as_ref(), row.source_identity.as_ref()),
        ("a", "u")
    );
    assert_eq!(row.qualified_declaration, [Box::from("Triple")]);
    assert_eq!(
        row.type_handle,
        declaration(&package, "Triple").key(),
        "the type handle is the checked declaration's key"
    );

    let ordinals: Vec<u64> = row.positions.iter().map(|p| p.ordinal).collect();
    assert_eq!(ordinals, [0, 1, 2]);
    let pair = declaration(&package, "Pair").key();
    let types: Vec<ValueType> = row.positions.iter().map(|p| p.value_type.clone()).collect();
    assert_eq!(
        types,
        [
            ValueType::Composite(pair),
            ValueType::Option(std::sync::Arc::new(ValueType::Integer)),
            ValueType::Boolean
        ]
    );
    let bound: Vec<Value> = node["body"]["members"]
        .as_array()
        .expect("the tuple body lists its references")
        .iter()
        .map(|member| member["target"].clone())
        .collect();
    let rowed: Vec<Value> = row
        .positions
        .iter()
        .map(|p| id_json(&p.type_node))
        .collect();
    assert_eq!(
        rowed, bound,
        "type nodes are the body's references in order"
    );
    assert_eq!(
        row.positions[0].type_node,
        row_of(&package, &emission, "Pair").structural_node
    );

    let keys: Vec<&NodeKey> = emission
        .selected_source_tuple_rows()
        .iter()
        .map(|row| {
            package
                .graph()
                .semantic_graph()
                .composite_node(&row.type_handle)
                .expect("a tuple node")
        })
        .collect();
    assert!(keys.windows(2).all(|pair| pair[0] < pair[1]));

    let (_, records, _) = emitted(super::source_record_rows::RECORDS);
    assert_eq!(records.selected_source_tuple_rows(), []);
}

#[trace("FR-092-AC-22", "TC-1021")]
#[test]
fn ir_joins_the_emitted_tuple_rows_and_refuses_a_misdescribed_position_list() {
    let (package, emission, wire) = emitted(TUPLES);
    let admitted = admit(&emission, &wire);
    let row = row_of(&package, &emission, "Triple").clone();

    let joined = SelectedExactIdentities::empty()
        .with_source_tuple_rows(
            &admitted,
            emission.selected_source_tuple_rows().to_vec(),
            10_000,
        )
        .expect("IR joins the emitted tuple rows");
    let projected = admitted
        .project_exact_type(&row.structural_node, &joined, 10_000)
        .expect("the tuple projects");
    assert_eq!(
        projected.value_type(),
        &ValueType::Composite(row.type_handle.clone())
    );
    assert_eq!(projected.source_tuple_view(), Some(&row));

    let refusal = admitted
        .project_exact_type(
            &row.structural_node,
            &SelectedExactIdentities::empty(),
            10_000,
        )
        .expect_err("a tuple cannot project without its row");
    assert_eq!(
        refusal.code,
        ExactTypeProjectionErrorCode::UnavailableIdentityWitness
    );

    let mut misordered = row.clone();
    misordered.positions.swap(0, 2);
    let mut short = row.clone();
    short.positions.pop();
    let mut extra = row.clone();
    extra.positions.push(row.positions[2].clone());
    let mut wrong_node = row.clone();
    wrong_node.positions[1].type_node = row.positions[2].type_node.clone();
    let mut renumbered = row.clone();
    renumbered.positions[1].ordinal = 2;
    for bad in [misordered, short, extra, wrong_node, renumbered] {
        let refusal = SelectedExactIdentities::empty()
            .with_source_tuple_rows(&admitted, [bad], 10_000)
            .expect_err("a row that disagrees with the admitted tuple is refused");
        assert_eq!(
            refusal.code,
            ExactTypeProjectionErrorCode::MismatchedIdentityWitness
        );
    }
}

fn guard_inputs(
    package: &CheckedPackage,
) -> (
    Vec<ValueType>,
    Vec<NodeKey>,
    HashMap<NodeKey, CheckedNodeId>,
) {
    let CompositeShape::Tuple(declared) = declaration(package, "Triple").shape() else {
        panic!("Triple is a tuple");
    };
    let graph = package.graph().semantic_graph();
    let node_key = graph
        .composite_node(&declaration(package, "Triple").key())
        .expect("the tuple node");
    let nodes = graph
        .node(node_key.clone())
        .expect("the tuple node")
        .tuple_position_nodes()
        .expect("a tuple body");
    let emitted = graph
        .nodes()
        .map(|node| (node.key(), node_id(node.key())))
        .collect();
    (declared.to_vec(), nodes, emitted)
}

#[trace("FR-092-AC-22", "TC-1021")]
#[test]
fn positions_that_disagree_with_the_declaration_are_a_writer_fault() {
    let (package, emission, _) = emitted(TUPLES);
    let (declared, nodes, emitted) = guard_inputs(&package);
    let rows =
        crate::emit::tuple_position_rows(&declared, nodes.clone(), &emitted).expect("they agree");
    assert_eq!(rows.len(), 3);

    let mut fewer = nodes.clone();
    fewer.pop();
    assert_eq!(
        crate::emit::tuple_position_rows(&declared, fewer, &emitted),
        Err(EmitRefusal::SourceMapInvariant)
    );
    assert_eq!(
        crate::emit::tuple_position_rows(&declared, nodes, &HashMap::new()),
        Err(EmitRefusal::SourceMapInvariant),
        "a type node that was not emitted"
    );

    let emitted_all: HashMap<NodeKey, CheckedNodeId> = package
        .graph()
        .semantic_graph()
        .nodes()
        .map(|node| (node.key(), node_id(node.key())))
        .collect();
    let graph = package.graph();
    let without = graph
        .scope()
        .types()
        .composites()
        .filter(|declaration| declaration.name() != "Triple");
    assert_eq!(
        crate::emit::source_tuple_rows_over(graph, &emitted_all, without),
        Err(EmitRefusal::SourceMapInvariant),
        "an emitted tuple node no declaration joins to is a fault, not no row"
    );
    assert_eq!(emission.selected_source_tuple_rows().len(), 2);
}

#[trace("FR-092-AC-22", "TC-1021")]
#[test]
fn the_final_tuple_node_decides_each_position_type_node() {
    let (package, emission, _) = emitted(TUPLES);
    let row = row_of(&package, &emission, "Triple").clone();
    let mut rows = vec![row.clone()];
    crate::emit::monitor_plan::follow_final_positions(&mut rows, &emission.wire_nodes)
        .expect("an unchanged node follows");
    assert_eq!(rows, [row.clone()]);

    // A writer that re-points a position changes the row with it.
    let edit = |change: &dyn Fn(&mut Vec<CheckedTerm>)| {
        let mut nodes = emission.wire_nodes.clone();
        let tuple = nodes
            .iter_mut()
            .find(|node| node.node_id == row.structural_node)
            .expect("the tuple node");
        let CheckedTerm::Aggregate { members } = &mut tuple.body else {
            panic!("an aggregate body");
        };
        change(members);
        nodes
    };
    let repointed = edit(&|members| {
        members[1] = CheckedTerm::Reference {
            target: row.positions[2].type_node.clone(),
        };
    });
    let mut stale = vec![row.clone()];
    crate::emit::monitor_plan::follow_final_positions(&mut stale, &repointed)
        .expect("a re-pointed position follows");
    assert_eq!(stale[0].positions[1].type_node, row.positions[2].type_node);
    assert_eq!(stale[0].positions[0], row.positions[0]);

    let shortened = edit(&|members| {
        members.pop();
    });
    let mut short = vec![row];
    assert!(crate::emit::monitor_plan::follow_final_positions(&mut short, &shortened).is_err());
}

const ENUMS: &str = "enum Status { READY, DONE }\n\
    ordered enum Level { LOW, MID, HIGH }\n\
    function f using v(s: Status, l: Level): Boolean pure { s = Status::READY }\n";

#[trace("FR-092-AC-23", "TC-1021")]
#[test]
fn a_source_enum_emits_its_checked_variant_shape_with_final_node_ids() {
    let (package, emission, wire) = emitted(ENUMS);
    let rows = emission.selected_source_enum_shape_rows();
    assert_eq!(rows.len(), 2);
    for (name, ordered, cases) in [
        ("Status", false, ["DONE", "READY"].as_slice()),
        ("Level", true, ["LOW", "MID", "HIGH"].as_slice()),
    ] {
        let binding = package
            .graph()
            .scope()
            .enums()
            .iter()
            .find(|binding| binding.name == name)
            .expect("the binding");
        let row = rows
            .iter()
            .find(|row| row.declaration_key == binding.declaration.key())
            .unwrap_or_else(|| panic!("{name} has a row"));
        assert_eq!(row.shape, binding.shape());
        assert_eq!(row.shape.is_ordered(), ordered);
        let names: Vec<&str> = row.cases.iter().map(|c| c.case.as_ref()).collect();
        assert_eq!(names, cases, "canonical order");
        let variants: Vec<_> = row.cases.iter().map(|c| c.variant.clone()).collect();
        let shaped: Vec<_> = row.shape.variants().cloned().collect();
        assert_eq!(variants, shaped);
        let declaration = node_with_id(&wire, &row.structural_node);
        assert_eq!(declaration["semantic_form"], json!("enum"));
        for case in &row.cases {
            let node = node_with_id(&wire, &case.structural_node);
            assert_eq!(node["body"]["value"], json!(case.case.as_ref()));
            assert_eq!(node["semantic_type"], id_json(&row.structural_node));
            assert_eq!(
                node_id_of(&wire, &case.member_key),
                id_json(&case.structural_node)
            );
        }
        let declaration_row = emission
            .selected_enum_declaration_rows()
            .iter()
            .find(|d| d.node_key == row.declaration_key)
            .expect("the declaration row");
        assert_eq!(declaration_row.structural_node, row.structural_node);
        let member_ids: Vec<_> = emission
            .selected_enum_member_rows()
            .iter()
            .filter(|m| m.declaration_node == row.structural_node)
            .map(|m| (m.case.clone(), m.structural_node.clone()))
            .collect();
        for case in &row.cases {
            assert!(member_ids.contains(&(case.case.clone(), case.structural_node.clone())));
        }
    }
}

#[trace("FR-092-AC-23", "TC-1021")]
#[test]
fn a_reordered_or_missing_variant_is_not_the_checked_shape() {
    let (package, emission, _) = emitted(ENUMS);
    let row = emission
        .selected_source_enum_shape_rows()
        .iter()
        .find(|row| row.cases.len() == 3)
        .expect("Level");
    let variants: Vec<_> = row.cases.iter().map(|c| c.variant.clone()).collect();
    let rebuilt = |order: &[usize]| {
        quire_exact::EnumShape::new(true, order.iter().map(|i| variants[*i].clone()))
    };
    assert_eq!(rebuilt(&[0, 1, 2]), row.shape);
    assert_ne!(rebuilt(&[1, 0, 2]), row.shape, "a reordered variant");
    assert_ne!(rebuilt(&[0, 2]), row.shape, "a missing variant");
    assert_ne!(
        quire_exact::EnumShape::new(false, variants.clone()),
        row.shape,
        "the ordered flag is part of the shape"
    );

    // A shape whose variant count disagrees with the cases is a writer fault.
    let emitted: HashMap<NodeKey, CheckedNodeId> = package
        .graph()
        .semantic_graph()
        .nodes()
        .map(|node| (node.key(), node_id(node.key())))
        .collect();
    let all =
        crate::emit::source_enum_shape_rows(package.graph(), &emitted).expect("every enum joins");
    assert_eq!(all.len(), 2);
    let mut missing_case = emitted.clone();
    missing_case.remove(&row.cases[1].member_key);
    assert_eq!(
        crate::emit::source_enum_shape_rows(package.graph(), &missing_case),
        Err(EmitRefusal::SourceMapInvariant),
        "a case node that was not emitted"
    );
}
