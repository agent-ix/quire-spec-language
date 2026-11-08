// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-092-AC-22 to AC-24: an emitted source-owned tuple carries one IR
//! `SelectedSourceTupleRow` with its ordered positions, an emitted source enum
//! carries the checked shape and per-case variants in IR's enum rows, and a
//! collection type node is paired with the checker's collection type. IR joins
//! the tuple and enum rows and refuses rows that disagree.

use quire_contract_model::{
    ExactTypeProjectionErrorCode, SelectedExactIdentities, SelectedSourceTupleRow,
};

use super::source_enum_rows::node_with_id;
use super::source_union_rows::admit_bytes;
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

    let (_, records, _) = emitted(super::source_record_rows::RECORDS);
    assert_eq!(records.selected_source_tuple_rows(), []);
}

#[trace("FR-092-AC-22", "TC-1021")]
#[test]
fn ir_joins_the_emitted_tuple_rows_and_refuses_a_misdescribed_position_list() {
    let (package, emission, wire) = emitted(TUPLES);
    let admitted = admit_bytes(&emission, &wire);
    let row = row_of(&package, &emission, "Triple").clone();

    let joined = SelectedExactIdentities::empty()
        .with_source_tuple_rows(
            &admitted,
            emission.selected_source_tuple_rows().to_vec(),
            &mut quire_contract_model::NativeJoinBudget::new(10_000_000, 10_000_000),
        )
        .expect("IR joins the emitted tuple rows");
    let projected = admitted
        .project_exact_type(
            &row.structural_node,
            &joined,
            &mut crate::V2ReadLimits::fixture().native_join_budget(),
        )
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
            &mut crate::V2ReadLimits::fixture().native_join_budget(),
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
            .with_source_tuple_rows(
                &admitted,
                [bad],
                &mut quire_contract_model::NativeJoinBudget::new(10_000_000, 10_000_000),
            )
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
    let (package, _, _) = emitted(TUPLES);
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

    let graph = package.graph();
    let without = graph
        .scope()
        .types()
        .composites()
        .filter(|declaration| declaration.name() != "Triple");
    assert_eq!(
        crate::emit::source_tuple_rows_over(graph, &emitted, without),
        Err(EmitRefusal::SourceMapInvariant),
        "an emitted tuple node no declaration joins to is a fault, not no row"
    );
}

#[trace("FR-092-AC-22", "TC-1021")]
#[test]
fn the_final_tuple_node_decides_each_position_type_node() {
    let (package, emission, _) = emitted(TUPLES);
    let row = row_of(&package, &emission, "Triple").clone();
    let mut rows = vec![row.clone()];
    crate::emit::monitor_plan::follow_final_positions(&mut rows, &emission.wire_nodes)
        .expect("an unchanged node follows");
    assert_eq!(rows, std::slice::from_ref(&row));

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
fn a_source_enum_carries_its_checked_shape_and_ir_joins_it() {
    let (package, emission, wire) = emitted(ENUMS);
    let admitted = admit_bytes(&emission, &wire);
    let rows = emission
        .selected_source_enum_rows(&mut quire_contract_model::NativeJoinBudget::new(
            10_000_000, 10_000_000,
        ))
        .expect("budgeted source enum rows");
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
            .find(|row| row.declaration.node_key == binding.declaration.key())
            .unwrap_or_else(|| panic!("{name} has a row"));
        let shape = &row.declaration.shape;
        assert_eq!(*shape, binding.shape());
        assert_eq!(shape.is_ordered(), ordered);
        let names: Vec<&str> = row.members.iter().map(|m| m.case.as_ref()).collect();
        assert_eq!(names, cases, "canonical order");
        let variants: Vec<_> = row.members.iter().map(|m| m.variant.clone()).collect();
        assert_eq!(variants, shape.variants().cloned().collect::<Vec<_>>());
        for member in &row.members {
            let node = node_with_id(&wire, &member.structural_node);
            assert_eq!(node["body"]["value"], json!(member.case.as_ref()));
            assert_eq!(member.declaration_node, row.declaration.structural_node);
        }

        let joined = SelectedExactIdentities::empty()
            .with_source_enum_rows(
                &admitted,
                rows.clone(),
                &mut quire_contract_model::NativeJoinBudget::new(10_000_000, 10_000_000),
            )
            .expect("IR joins the emitted enum rows");
        let projected = admitted
            .project_exact_type(
                &row.declaration.structural_node,
                &joined,
                &mut crate::V2ReadLimits::fixture().native_join_budget(),
            )
            .expect("the enum projects");
        assert_eq!(projected.value_type(), &ValueType::Enum(shape.clone()));
        assert_eq!(projected.source_enum_view(), Some(row));
    }
}

#[trace("FR-092-AC-23", "TC-1021")]
#[test]
fn ir_refuses_an_enum_row_with_a_reordered_or_missing_variant_or_flipped_flag() {
    let (_, emission, wire) = emitted(ENUMS);
    let admitted = admit_bytes(&emission, &wire);
    let level = emission
        .selected_source_enum_rows(&mut quire_contract_model::NativeJoinBudget::new(
            10_000_000, 10_000_000,
        ))
        .expect("budgeted source enum rows")
        .into_iter()
        .find(|row| row.members.len() == 3)
        .expect("Level");
    let mut reordered = level.clone();
    reordered.members.swap(0, 1);
    let mut missing = level.clone();
    missing.members.pop();
    let mut swapped_variants = level.clone();
    let first = swapped_variants.members[0].variant.clone();
    swapped_variants.members[0].variant = swapped_variants.members[1].variant.clone();
    swapped_variants.members[1].variant = first;
    let mut flipped = level.clone();
    let variants = level.declaration.shape.variants().cloned();
    flipped.declaration.shape = quire_exact::EnumShape::new(false, variants);
    let mut short_shape = level.clone();
    short_shape.declaration.shape =
        quire_exact::EnumShape::new(true, level.declaration.shape.variants().take(2).cloned());
    for bad in [reordered, missing, swapped_variants, flipped, short_shape] {
        SelectedExactIdentities::empty()
            .with_source_enum_rows(
                &admitted,
                [bad],
                &mut quire_contract_model::NativeJoinBudget::new(10_000_000, 10_000_000),
            )
            .expect_err("a row that disagrees with the admitted enum is refused");
    }
    let refusal = admitted
        .project_exact_type(
            &level.declaration.structural_node,
            &SelectedExactIdentities::empty(),
            &mut crate::V2ReadLimits::fixture().native_join_budget(),
        )
        .expect_err("no row, no projection");
    assert_eq!(
        refusal.code,
        ExactTypeProjectionErrorCode::UnavailableIdentityWitness
    );
}

#[trace("FR-092-AC-23", "TC-1021")]
#[test]
fn an_enum_case_node_that_was_not_emitted_is_a_writer_fault() {
    let (package, emission, _) = emitted(ENUMS);
    let emitted_ids: HashMap<NodeKey, CheckedNodeId> = package
        .graph()
        .semantic_graph()
        .nodes()
        .map(|node| (node.key(), node_id(node.key())))
        .collect();
    let (declarations, members) =
        crate::emit::source_enum_rows(package.graph(), &emitted_ids).expect("every enum joins");
    assert_eq!(
        declarations.len(),
        emission.selected_enum_declaration_rows().len()
    );
    assert_eq!(members.len(), emission.selected_enum_member_rows().len());
    let mut missing = emitted_ids;
    missing.remove(&members[0].member_key);
    assert_eq!(
        crate::emit::source_enum_rows(package.graph(), &missing),
        Err(EmitRefusal::SourceMapInvariant)
    );
}

const NESTED: &str = "enum Color { RED, BLUE }\n\
    tuple Inner(Color, Integer);\n\
    record Rec { kids: Sequence<Rec>[0, 2]; pair: Inner; grid: Sequence<Sequence<Color>>; }\n\
    function f using v(r: Rec): Boolean pure { true }\n";

#[trace("FR-092-AC-22", "FR-092-AC-23", "FR-092-AC-24", "TC-1021")]
#[test]
fn nested_enum_tuple_and_collection_elements_are_paired_with_their_final_nodes() {
    let (package, emission, wire) = emitted(NESTED);
    let enum_row = &emission
        .selected_source_enum_rows(&mut quire_contract_model::NativeJoinBudget::new(
            10_000_000, 10_000_000,
        ))
        .expect("budgeted source enum rows")[0];
    let color = enum_row.declaration.structural_node.clone();
    let inner = row_of(&package, &emission, "Inner");
    assert_eq!(inner.positions[0].type_node, color);
    assert_eq!(
        inner.positions[0].value_type,
        ValueType::Enum(enum_row.declaration.shape.clone())
    );
    let rec = super::source_record_rows::row_of(&package, &emission, "Rec");
    assert_eq!(rec.members[1].type_node, inner.structural_node);

    let rows = emission.selected_source_collection_rows();
    assert_eq!(rows.len(), 3);
    let element_of = |row: &crate::SelectedSourceCollectionRow| match &row.value_type {
        ValueType::Collection(collection) => collection.element().clone(),
        other => panic!("a collection row holds {other:?}"),
    };
    let find = |want: &dyn Fn(&ValueType) -> bool| {
        rows.iter()
            .find(|row| want(&element_of(row)))
            .expect("the collection row")
    };
    let kids = find(
        &|e| matches!(e, ValueType::Composite(key) if *key == declaration(&package, "Rec").key()),
    );
    let inner_grid = find(&|e| matches!(e, ValueType::Enum(_)));
    let outer_grid = find(&|e| matches!(e, ValueType::Collection(_)));
    assert_eq!(
        kids.element_node, rec.structural_node,
        "a recursive element"
    );
    assert_eq!(inner_grid.element_node, color, "an enum element");
    assert_eq!(outer_grid.element_node, inner_grid.structural_node);
    assert_eq!(rec.members[0].type_node, kids.structural_node);
    assert_eq!(rec.members[2].type_node, outer_grid.structural_node);
    for row in rows {
        let base = node_with_id(&wire, &row.base_node);
        assert_eq!(
            base["body"]["members"][0]["target"],
            id_json(&row.element_node)
        );
    }
}

const COLLECTIONS: &str = "record R { xs: Sequence<Integer>[0, 3]; s: Set<Boolean>; }\n\
    function f using v(r: R): Boolean pure { true }\n";

#[trace("FR-092-AC-24", "TC-1021")]
#[test]
fn a_collection_type_node_is_paired_with_the_checkers_collection_type() {
    let (package, emission, wire) = emitted(COLLECTIONS);
    let CompositeShape::Record(fields) = declaration(&package, "R").shape() else {
        panic!("R is a record");
    };
    let rows = emission.selected_source_collection_rows();
    assert_eq!(rows.len(), 2);
    let record = super::source_record_rows::row_of(&package, &emission, "R");
    for (index, form, bounded, element_form) in [
        (0, "sequence", true, "integer"),
        (1, "set", false, "boolean"),
    ] {
        let row = rows
            .iter()
            .find(|row| row.value_type == *fields[index].value_type())
            .expect("a row for the field's checked type");
        assert_eq!(
            row.structural_node, record.members[index].type_node,
            "the record binds the collection row's node"
        );
        assert_eq!(row.structural_node != row.base_node, bounded);
        let base = node_with_id(&wire, &row.base_node);
        assert_eq!(base["semantic_form"], json!(form));
        assert_eq!(
            base["body"]["members"][0]["target"],
            id_json(&row.element_node)
        );
        assert_eq!(
            node_with_id(&wire, &row.element_node)["semantic_form"],
            json!(element_form)
        );
        if bounded {
            let domain = node_with_id(&wire, &row.structural_node);
            assert_eq!(domain["semantic_form"], json!("collection_bounds"));
            assert_eq!(domain["semantic_type"], id_json(&row.base_node));
        }
    }
    assert_eq!(
        emitted(super::source_record_rows::RECORDS)
            .1
            .selected_source_collection_rows(),
        []
    );
}

#[trace("FR-092-AC-24", "TC-1021")]
#[test]
fn a_collection_row_that_disagrees_with_its_nodes_is_a_writer_fault() {
    let (package, emission, _) = emitted(COLLECTIONS);
    let graph = package.graph().semantic_graph();
    let (key, collection) = graph.source_collections().next().expect("a collection");
    let emitted_ids: HashMap<NodeKey, CheckedNodeId> = graph
        .nodes()
        .map(|node| (node.key(), node_id(node.key())))
        .collect();
    let structural = node_id(key.clone());
    let references = Some(vec![collection.element.clone()]);
    let join = |c: &qsl_semantics::check::SourceCollection,
                refs: Option<Vec<NodeKey>>,
                ids: &HashMap<NodeKey, CheckedNodeId>| {
        crate::emit::collection_row(&structural, c, refs, ids)
    };
    assert!(join(collection, references.clone(), &emitted_ids).is_ok());

    // A wrong element reference.
    let other = graph
        .source_collections()
        .map(|(_, c)| c.element.clone())
        .find(|element| *element != collection.element)
        .expect("another element");
    assert_eq!(
        join(collection, Some(vec![other]), &emitted_ids),
        Err(EmitRefusal::SourceMapInvariant)
    );
    // A map row with no key node is not a collection row.
    let mut mixed = collection.clone();
    mixed.value_type = ValueType::map(
        quire_exact::MapType::new(ValueType::String, ValueType::Boolean, None).expect("a leaf key"),
    );
    assert_eq!(
        join(&mixed, references.clone(), &emitted_ids),
        Err(EmitRefusal::SourceMapInvariant)
    );
    // An element that was not emitted.
    let mut without = emitted_ids.clone();
    without.remove(&collection.element);
    assert_eq!(
        join(collection, references, &without),
        Err(EmitRefusal::SourceMapInvariant)
    );

    // The final nodes decide the element and the bound's base.
    let bounded = emission
        .selected_source_collection_rows()
        .iter()
        .find(|row| row.structural_node != row.base_node)
        .expect("the bounded sequence")
        .clone();
    let mut rows = vec![bounded.clone()];
    crate::emit::monitor_plan::follow_final_collections(&mut rows, &emission.wire_nodes)
        .expect("the emitted nodes agree");
    assert_eq!(rows, std::slice::from_ref(&bounded));
    let mut nodes = emission.wire_nodes.clone();
    for node in &mut nodes {
        if node.node_id == bounded.base_node {
            node.body = CheckedTerm::Aggregate {
                members: vec![CheckedTerm::Reference {
                    target: bounded.structural_node.clone(),
                }],
            };
        }
    }
    let mut moved = vec![bounded.clone()];
    crate::emit::monitor_plan::follow_final_collections(&mut moved, &nodes)
        .expect("a re-pointed element follows");
    assert_eq!(moved[0].element_node, bounded.structural_node);
    for node in &mut nodes {
        if node.node_id == bounded.structural_node {
            node.semantic_type = bounded.element_node.clone();
        }
    }
    assert!(
        crate::emit::monitor_plan::follow_final_collections(&mut [bounded], &nodes).is_err(),
        "a bound over another node than the collection"
    );
}

/// A `composite_type`/`map` body: the `key` binding, then the `value` binding.
fn map_body(key: &CheckedNodeId, value: &CheckedNodeId) -> CheckedTerm {
    serde_json::from_value(json!({
        "term": "aggregate",
        "members": [
            {"term": "binding", "name": "key",
             "value": {"term": "reference", "target": id_json(key)}},
            {"term": "binding", "name": "value",
             "value": {"term": "reference", "target": id_json(value)}},
        ],
    }))
    .expect("a map body")
}

/// FR-092-AC-28: a map's row joins its key and value nodes, and every
/// disagreement between the row, the checked type and the base node's operands
/// is a writer fault.
#[trace("FR-092-AC-28", "TC-1021")]
#[test]
fn a_map_row_pairs_the_key_and_value_nodes_and_refuses_disagreement() {
    let (package, _, _) = emitted(COLLECTIONS);
    let graph = package.graph().semantic_graph();
    let emitted_ids: HashMap<NodeKey, CheckedNodeId> = graph
        .nodes()
        .map(|node| (node.key(), node_id(node.key())))
        .collect();
    let mut keys = graph.source_collections().map(|(_, c)| c.element.clone());
    let (key, value) = (
        keys.next().expect("a key node"),
        keys.next().expect("a value node"),
    );
    let (structural, collection) = graph.source_collections().next().expect("a collection");
    let map_type = ValueType::map(
        quire_exact::MapType::new(ValueType::String, ValueType::Boolean, None).expect("a leaf key"),
    );
    let map = qsl_semantics::check::SourceCollection {
        value_type: map_type.clone(),
        base: collection.base.clone(),
        element: value.clone(),
        key: Some(key.clone()),
    };
    let structural = node_id(structural.clone());
    let join = |c: &qsl_semantics::check::SourceCollection,
                refs: Option<Vec<NodeKey>>,
                ids: &HashMap<NodeKey, CheckedNodeId>| {
        crate::emit::collection_row(&structural, c, refs, ids)
    };
    let both = Some(vec![key.clone(), value.clone()]);
    let row = join(&map, both.clone(), &emitted_ids).expect("a map row");
    assert_eq!(row.key_node, Some(node_id(key.clone())));
    assert_eq!(row.element_node, node_id(value.clone()));
    assert_eq!(row.value_type, map_type);

    let fault = Err(EmitRefusal::SourceMapInvariant);
    // The base names the value before the key, or only the value.
    assert_eq!(
        join(&map, Some(vec![value.clone(), key.clone()]), &emitted_ids),
        fault
    );
    assert_eq!(join(&map, Some(vec![value.clone()]), &emitted_ids), fault);
    // A map with no key node, and a collection with one.
    let mut keyless = map.clone();
    keyless.key = None;
    assert_eq!(join(&keyless, both.clone(), &emitted_ids), fault);
    assert_eq!(
        join(&keyless, Some(vec![value.clone()]), &emitted_ids),
        fault
    );
    let mut keyed = collection.clone();
    keyed.key = Some(key.clone());
    assert_eq!(
        join(
            &keyed,
            Some(vec![key.clone(), collection.element.clone()]),
            &emitted_ids
        ),
        fault
    );
    // A key that was not emitted.
    let mut without = emitted_ids.clone();
    without.remove(&key);
    assert_eq!(join(&map, both, &without), fault);
}

/// FR-092-AC-28: the final nodes decide a map row's key and value, and a base
/// that is not the `key` then `value` bindings is a writer fault.
#[trace("FR-092-AC-28", "TC-1021")]
#[test]
fn a_map_row_follows_its_final_key_and_value_nodes() {
    let (_, emission, _) = emitted(COLLECTIONS);
    let bounded = emission
        .selected_source_collection_rows()
        .iter()
        .find(|row| row.structural_node != row.base_node)
        .expect("a bounded collection")
        .clone();
    let (key, value) = (
        bounded.element_node.clone(),
        bounded.structural_node.clone(),
    );
    let mut row = bounded.clone();
    row.key_node = Some(bounded.base_node.clone());
    let mut nodes = emission.wire_nodes.clone();
    for node in &mut nodes {
        if node.node_id == bounded.base_node {
            node.body = map_body(&key, &value);
        }
    }
    let mut followed = vec![row.clone()];
    crate::emit::monitor_plan::follow_final_collections(&mut followed, &nodes)
        .expect("a map base follows");
    assert_eq!(followed[0].key_node, Some(key.clone()));
    assert_eq!(followed[0].element_node, value);

    // A map row over a collection-shaped base, and a collection row over a
    // map-shaped base, are writer faults.
    let mut unchanged = emission.wire_nodes.clone();
    assert!(
        crate::emit::monitor_plan::follow_final_collections(&mut [row.clone()], &unchanged)
            .is_err()
    );
    for node in &mut unchanged {
        if node.node_id == bounded.base_node {
            node.body = map_body(&key, &value);
        }
    }
    let mut collection_row = row;
    collection_row.key_node = None;
    assert!(
        crate::emit::monitor_plan::follow_final_collections(&mut [collection_row], &unchanged)
            .is_err()
    );
}

/// A checked package whose record `R` holds `m: Map<Text, Boolean>[1, 4]` and
/// `n: Map<Text, Int[0, 9]>`, and a function over `R`. Source text spells no
/// map, so the record is declared as the checker declares it, over the
/// declaration API the emitter tests share.
pub(super) fn map_record_package() -> CheckedPackage {
    use qsl_forms::TypeForm;
    use quire_exact::{CardinalityBound, MapType, Presence};
    use quire_semantic_value::declaration::{
        CompositeDeclaration, CompositeShape, FieldDeclaration, TypeEnvironment,
    };

    let handle = NodeKey::new("pkg", vec![quire_exact::PathSegment::Text("R".into())], 0)
        .expect("a well formed key");
    let map = |values, bound: Option<(u64, u64)>| {
        ValueType::map(
            MapType::new(
                ValueType::String,
                values,
                bound.map(|(lower, upper)| CardinalityBound::new(lower, upper).expect("a bound")),
            )
            .expect("a leaf key"),
        )
    };
    let digit = ValueType::Int(
        quire_exact::IntegerInterval::new(
            quire_exact::Integer::from(0_i64),
            quire_exact::Integer::from(9_i64),
        )
        .expect("an interval"),
    );
    let record = CompositeDeclaration::new(
        handle,
        "R",
        CompositeShape::Record(vec![
            FieldDeclaration::new(
                "m",
                map(ValueType::Boolean, Some((1, 4))),
                Presence::Required,
            ),
            FieldDeclaration::new("n", map(digit, None), Presence::Required),
        ]),
    );
    let span = qsl_foundation::Span { start: 0, end: 0 };
    let mut declarations = qsl_semantics::check::depth_forms::declarations(
        qsl_semantics::check::depth_forms::Form::And,
        1,
    );
    declarations.functions = vec![qsl_semantics::check::depth_forms::function(
        "f",
        &[("r", TypeForm::name("R", span))],
        qsl_semantics::check::depth_forms::boolean(),
        None,
        qsl_forms::Expression::boolean(true),
    )];
    declarations.package_name = "pkg".to_owned();
    declarations.types = TypeEnvironment::new([record], []).expect("R is admitted");
    let graph = declarations
        .check(CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("the package checks: {refusals:?}"));
    CheckedPackage::link(graph)
}

/// FR-092-AC-28 end to end: a record with map fields emits one row per map,
/// with the key node a `text` scalar, the value node the field's value type
/// and the base node binding the key then the value. `carry_rows` re-keys the
/// key node with the other nodes of the row.
#[trace("FR-092-AC-28", "TC-1021")]
#[test]
fn a_record_with_map_fields_emits_map_rows_and_carries_their_key_nodes() {
    let package = map_record_package();
    // The record is built in code, not read from a source unit, so every
    // occurrence is placed at the fixture unit.
    let emission = crate::emit::emit_package(&package, whole_unit, crate::EmitLimits::fixture())
        .expect("the package emits");
    assert_eq!(emission.omitted, []);
    let wire = wire(&emission);
    let rows = emission.selected_source_collection_rows();
    assert_eq!(rows.len(), 2);
    let record = super::source_record_rows::row_of(&package, &emission, "R");
    for (index, value_form, bounded) in [(0, "boolean", true), (1, "integer_range", false)] {
        let row = rows
            .iter()
            .find(|row| row.structural_node == record.members[index].type_node)
            .expect("the record binds the map row's node");
        assert!(matches!(row.value_type, ValueType::Map(_)));
        assert_eq!(row.structural_node != row.base_node, bounded);
        let key = row.key_node.clone().expect("a map row holds its key node");
        assert_eq!(node_with_id(&wire, &key)["semantic_form"], json!("text"));
        assert_eq!(
            node_with_id(&wire, &row.element_node)["semantic_form"],
            json!(value_form)
        );
        let base = node_with_id(&wire, &row.base_node);
        assert_eq!(base["semantic_form"], json!("map"));
        assert_eq!(base["body"]["members"][0]["name"], json!("key"));
        assert_eq!(base["body"]["members"][0]["value"]["target"], id_json(&key));
        assert_eq!(base["body"]["members"][1]["name"], json!("value"));
        assert_eq!(
            base["body"]["members"][1]["value"]["target"],
            id_json(&row.element_node)
        );
    }

    // The retained rows follow a renamed key node.
    let key = rows[0].key_node.clone().expect("a key node");
    let renamed = |id: &CheckedNodeId| CheckedNodeId {
        package: format!("renamed:{}", id.package).into(),
        path: id.path.clone(),
        ordinal: id.ordinal,
    };
    let carried = crate::emit::monitor_plan::carry_rows(&emission, &[(key.clone(), renamed(&key))]);
    for (got, was) in carried.selected_source_collection_rows.iter().zip(rows) {
        let expected = was.key_node.as_ref().map(|node| {
            if *node == key {
                renamed(node)
            } else {
                node.clone()
            }
        });
        assert_eq!(got.key_node, expected);
        assert_eq!(got.element_node, was.element_node);
    }
    assert!(carried
        .selected_source_collection_rows
        .iter()
        .any(|row| row.key_node == Some(renamed(&key))));
}
