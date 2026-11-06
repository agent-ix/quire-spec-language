// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-092-AC-21: an emitted source-owned record carries one
//! `SelectedSourceRecordRow` joined to the emitted record node, with every
//! member in declaration order: the declared name, the emitted binding name,
//! the declared presence, the emitted type node and the checked field type.

use quire_contract_model::{
    CheckedPackageReadLimits, CheckedPackageV2, CheckedPackageV2ReadResult,
};
use quire_exact::Presence;

use super::source_enum_rows::node_with_id;
use super::*;

pub(super) const RECORDS: &str = "record Point { x: Integer; y: Integer; }\n\
    record Tagged { p: Point; note: Boolean?; maybe: Option<Integer>; }\n\
    function f using v(t: Tagged): Boolean pure { true }\n";

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

/// The row of the record named `name`.
pub(super) fn row_of<'e>(
    package: &CheckedPackage,
    emission: &'e Emission,
    name: &str,
) -> &'e crate::SelectedSourceRecordRow {
    let key = declaration(package, name).key();
    emission
        .selected_source_record_rows()
        .iter()
        .find(|row| row.record_key == key)
        .unwrap_or_else(|| panic!("{name} has a row"))
}

/// The first `target` a wire value holds.
fn target_of(value: &Value) -> Value {
    match value {
        Value::Object(map) => map.get("target").cloned().unwrap_or_else(|| {
            map.values()
                .map(target_of)
                .find(|found| !found.is_null())
                .unwrap_or(Value::Null)
        }),
        Value::Array(items) => items
            .iter()
            .map(target_of)
            .find(|found| !found.is_null())
            .unwrap_or(Value::Null),
        _ => Value::Null,
    }
}

fn id_json(id: &CheckedNodeId) -> Value {
    serde_json::to_value(id).expect("a node id serializes")
}

#[trace("FR-092-AC-21", "TC-1021")]
#[test]
fn a_source_record_emits_its_declaration_and_ordered_member_rows() {
    let (package, emission, wire) = emitted(RECORDS);
    assert_eq!(emission.selected_source_record_rows().len(), 2);
    let row = row_of(&package, &emission, "Tagged");

    let node = node_with_id(&wire, &row.structural_node);
    assert_eq!(
        (&node["node_tag"], &node["semantic_form"]),
        (&json!("composite_type"), &json!("record"))
    );
    assert_eq!(
        node["owner"],
        json!({"kind": "source", "authority": "a", "identity": "u"})
    );
    assert_eq!(
        (row.source_authority.as_ref(), row.source_identity.as_ref()),
        ("a", "u")
    );
    assert_eq!(row.qualified_declaration, [Box::from("Tagged")]);
    assert_eq!(
        node_id_of(&wire, &declaration(&package, "Tagged").key()),
        id_json(&row.structural_node),
        "the record key is the checked declaration's key and names the node"
    );

    let identities: Vec<(&str, &str)> = row
        .members
        .iter()
        .map(|m| {
            (
                m.original_member_identity.as_ref(),
                m.admitted_name.as_ref(),
            )
        })
        .collect();
    assert_eq!(
        identities,
        [("p", "p"), ("note", "note"), ("maybe", "maybe")]
    );
    let presences: Vec<Presence> = row.members.iter().map(|m| m.presence).collect();
    assert_eq!(
        presences,
        [Presence::Required, Presence::Optional, Presence::Required]
    );
    // An optional field keeps its field type; a required `Option<T>` field
    // keeps `Option<T>`.
    assert_eq!(
        row.members[0].value_type,
        ValueType::Composite(declaration(&package, "Point").key())
    );
    assert_eq!(row.members[1].value_type, ValueType::Boolean);
    assert_eq!(
        row.members[2].value_type,
        ValueType::Option(std::sync::Arc::new(ValueType::Integer))
    );

    // The type nodes are the targets of the emitted bindings, in order.
    let bound: Vec<(Value, Value)> = node["body"]["members"]
        .as_array()
        .expect("the record body lists its bindings")
        .iter()
        .map(|member| (member["name"].clone(), target_of(&member["value"])))
        .collect();
    let rowed: Vec<(Value, Value)> = row
        .members
        .iter()
        .map(|m| (json!(m.admitted_name.as_ref()), id_json(&m.type_node)))
        .collect();
    assert_eq!(rowed, bound);
    let point = row_of(&package, &emission, "Point");
    assert_eq!(row.members[0].type_node, point.structural_node);
}

#[trace("FR-092-AC-21", "TC-1021")]
#[test]
fn a_recursive_record_row_names_type_nodes_in_its_own_recursion_group() {
    let (package, emission, wire) = emitted(
        "record Tree { weight: Integer; kids: Sequence<Tree>[0, 3]; rest: Option<Tree>; }\n",
    );
    let row = row_of(&package, &emission, "Tree");
    assert_eq!(row.members.len(), 3);
    for member in &row.members[1..] {
        let type_node = node_with_id(&wire, &member.type_node);
        assert!(
            !type_node["recursion_group"].is_null(),
            "{} is in a recursion group",
            member.admitted_name
        );
        assert_eq!(
            type_node["recursion_group"],
            node_with_id(&wire, &row.structural_node)["recursion_group"],
            "{} shares the record node's recursion group",
            member.admitted_name
        );
    }
    assert!(
        node_with_id(&wire, &row.members[0].type_node)["recursion_group"].is_null(),
        "a scalar member is outside the group"
    );
    let tree = ValueType::Composite(row.record_key.clone());
    assert_eq!(
        row.members[2].value_type,
        ValueType::Option(std::sync::Arc::new(tree))
    );
}

#[trace("FR-092-AC-21", "TC-1021")]
#[test]
fn rows_are_ascending_and_other_units_have_none() {
    let (package, emission, wire) = emitted(RECORDS);
    let emitted_records: Vec<Value> = nodes(&wire)
        .iter()
        .filter(|node| node["semantic_form"] == "record")
        .map(|node| node["node_id"].clone())
        .collect();
    assert_eq!(emitted_records.len(), 2);
    let rows = emission.selected_source_record_rows();
    let mut rowed: Vec<Value> = rows.iter().map(|r| id_json(&r.structural_node)).collect();
    let mut want = emitted_records;
    rowed.sort_by_key(Value::to_string);
    want.sort_by_key(Value::to_string);
    assert_eq!(rowed, want, "one row per emitted record node");
    let node_keys: Vec<&NodeKey> = rows
        .iter()
        .map(|row| {
            package
                .graph()
                .semantic_graph()
                .composite_node(&row.record_key)
                .expect("a record node")
        })
        .collect();
    assert_eq!(node_keys.len(), 2);
    assert!(
        node_keys.windows(2).all(|pair| pair[0] < pair[1]),
        "rows are in ascending order of their record nodes' keys"
    );

    let (package, tuple, _) = emitted("tuple Pair(Boolean, Boolean);\nunion U { A, B(Integer) }\n");
    assert_eq!(tuple.selected_source_record_rows(), []);
    drop(package);
}

#[trace("FR-092-AC-21", "TC-1021")]
#[test]
fn a_model_owned_record_has_no_source_record_row() {
    let (emission, wire, _) = super::child_composite::emit_unit(super::child_composite::UNIT);
    let model_records = nodes(&wire)
        .iter()
        .filter(|node| node["semantic_form"] == "record_value_type")
        .count();
    assert!(model_records >= 1, "the unit emits model-owned records");
    assert_eq!(
        emission.selected_source_record_rows(),
        [],
        "a model-owned record is not a source record"
    );
}

#[trace("FR-092-AC-21", "TC-1021")]
#[test]
fn ir_admits_the_row_nodes_as_record_declarations_and_their_type_nodes() {
    let (_, emission, wire) = emitted(RECORDS);
    let mut evidence = CheckedPackageEvidence::new();
    for feature in wire["lock"]["required_features"].as_array().unwrap() {
        evidence.support_feature(feature.as_str().unwrap());
    }
    let CheckedPackageV2ReadResult::Admitted(admitted) = CheckedPackageV2::read(
        emission.package.bytes(),
        CheckedPackageReadLimits::bounded(),
        &evidence,
    ) else {
        panic!("IR admits the emitted package");
    };
    for row in emission.selected_source_record_rows() {
        let named = |id: &CheckedNodeId| admitted.graph().nodes.iter().find(|n| &n.node_id == id);
        let record = named(&row.structural_node).expect("the record node is admitted");
        assert_eq!(record.semantic_form.as_ref(), "record");
        for member in &row.members {
            assert!(
                named(&member.type_node).is_some(),
                "{} names an admitted node",
                member.admitted_name
            );
        }
    }
}

#[trace("TC-1331", "FR-531-AC-19", "FR-517-AC-23")]
#[test]
fn required_identity_handoff_preserves_original_record_keys_and_native_views() {
    let (_, emission, wire) = emitted(RECORDS);
    let mut evidence = CheckedPackageEvidence::new();
    for feature in wire["lock"]["required_features"].as_array().unwrap() {
        evidence.support_feature(feature.as_str().unwrap());
    }
    let CheckedPackageV2ReadResult::Admitted(package) = CheckedPackageV2::read(
        emission.package().bytes(),
        CheckedPackageReadLimits::bounded(),
        &evidence,
    ) else {
        panic!("the genuine record package admits");
    };
    let rows = emission
        .identity_rows(&package)
        .expect("final producer rows");
    assert_eq!(rows.source_records().len(), 2);
    let bytes = rows
        .to_bytes(&package)
        .expect("the required carrier encodes");
    let read =
        crate::ReadIdentityRows::read(&bytes, &package).expect("native joins accept the rows");
    assert_eq!(read, rows);
    assert!(
        read.selected_populations().is_empty(),
        "a genuinely model-free emission needs no catalog"
    );
    assert_eq!(read.to_bytes(&package).unwrap(), bytes);
    drop(rows);
    drop(emission);
    let selected = read.selected(&package).expect("the native record witness");
    for row in read.source_records() {
        let projection = package
            .project_exact_type(&row.structural_node, &selected, 1_000_000)
            .expect("native source record projects");
        assert_eq!(projection.source_record_view(), Some(row));
        assert_eq!(
            projection.value_type(),
            &ValueType::Composite(row.record_key.clone())
        );
    }
    let encoded: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(encoded["format"], "quire.checked-handoff.identities/2");
    assert!(encoded["source_records"][0]["record_key"]["path"].is_array());
    let tagged = read
        .source_records()
        .iter()
        .find(|row| {
            row.qualified_declaration
                .last()
                .is_some_and(|name| name.as_ref() == "Tagged")
        })
        .unwrap();
    let point = read
        .source_records()
        .iter()
        .find(|row| {
            row.qualified_declaration
                .last()
                .is_some_and(|name| name.as_ref() == "Point")
        })
        .unwrap();
    assert_eq!(
        tagged.members[0].value_type,
        ValueType::Composite(point.record_key.clone())
    );
}

#[trace("TC-1331", "FR-531-AC-20")]
#[test]
fn required_source_record_rows_refuse_missing_fields_and_stale_native_bindings() {
    let (_, emission, wire) = emitted(RECORDS);
    let mut evidence = CheckedPackageEvidence::new();
    for feature in wire["lock"]["required_features"].as_array().unwrap() {
        evidence.support_feature(feature.as_str().unwrap());
    }
    let CheckedPackageV2ReadResult::Admitted(package) = CheckedPackageV2::read(
        emission.package().bytes(),
        CheckedPackageReadLimits::bounded(),
        &evidence,
    ) else {
        panic!("the genuine package admits");
    };
    let rows = emission.identity_rows(&package).unwrap();
    let bytes = rows.to_bytes(&package).unwrap();
    let good: Value = serde_json::from_slice(&bytes).unwrap();
    for (field, member) in [
        ("source_records", crate::IdentityMember::SourceRecords),
        (
            "selected_populations",
            crate::IdentityMember::SelectedPopulations,
        ),
    ] {
        let mut missing = good.clone();
        missing.as_object_mut().unwrap().remove(field);
        assert_eq!(
            crate::ReadIdentityRows::read(&serde_json::to_vec(&missing).unwrap(), &package),
            Err(crate::IdentityFileRefusal::Malformed { member })
        );
    }
    let mut stale = good.clone();
    stale["source_records"][0]["members"][0]["admitted_name"] = json!("not-the-final-name");
    let Err(crate::IdentityFileRefusal::Native(error)) =
        crate::ReadIdentityRows::read(&serde_json::to_vec(&stale).unwrap(), &package)
    else {
        panic!("a stale binding must refuse natively");
    };
    assert_eq!(
        error.code,
        quire_contract_model::ExactTypeProjectionErrorCode::MismatchedIdentityWitness
    );
    let mut foreign = good.clone();
    foreign["source_records"][0]["source_identity"] = json!("another-selected-source");
    let Err(crate::IdentityFileRefusal::Native(error)) =
        crate::ReadIdentityRows::read(&serde_json::to_vec(&foreign).unwrap(), &package)
    else {
        panic!("a foreign source owner must refuse natively");
    };
    assert_eq!(
        error.code,
        quire_contract_model::ExactTypeProjectionErrorCode::MismatchedIdentityWitness
    );
    let mut conflicting = good.clone();
    let repeated = conflicting["source_records"][0].clone();
    conflicting["source_records"]
        .as_array_mut()
        .unwrap()
        .push(repeated);
    let Err(crate::IdentityFileRefusal::Native(error)) =
        crate::ReadIdentityRows::read(&serde_json::to_vec(&conflicting).unwrap(), &package)
    else {
        panic!("duplicate source rows must refuse natively");
    };
    assert_eq!(
        error.code,
        quire_contract_model::ExactTypeProjectionErrorCode::ConflictingIdentityWitness
    );
    let mut missing = good.clone();
    missing["source_records"] = json!([]);
    let missing = crate::ReadIdentityRows::read(&serde_json::to_vec(&missing).unwrap(), &package)
        .expect("a carrier read is not a whole-witness validator");
    let incomplete = missing.selected(&package).unwrap();
    let requested = &rows.source_records()[0].structural_node;
    let refusal = package
        .project_exact_type(requested, &incomplete, 1_000_000)
        .expect_err("the actual missing record request refuses natively");
    assert_eq!(
        refusal.code,
        quire_contract_model::ExactTypeProjectionErrorCode::UnavailableIdentityWitness
    );
}

#[trace("TC-1331", "FR-531-AC-20")]
#[test]
fn required_identity_handoff_refuses_deep_original_keys_without_stack_growth() {
    const CHILD: &str = "QSL1760_DEEP_IDENTITY_CHILD";
    const TEST: &str = "emit::tests::source_record_rows::required_identity_handoff_refuses_deep_original_keys_without_stack_growth";
    if std::env::var_os(CHILD).is_none() {
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", TEST, "--nocapture"])
            .env(CHILD, "1")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "isolated deep-key control failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        return;
    }
    let (_, emission, wire) = emitted(RECORDS);
    let mut evidence = CheckedPackageEvidence::new();
    for feature in wire["lock"]["required_features"].as_array().unwrap() {
        evidence.support_feature(feature.as_str().unwrap());
    }
    let CheckedPackageV2ReadResult::Admitted(package) = CheckedPackageV2::read(
        emission.package().bytes(),
        CheckedPackageReadLimits::bounded(),
        &evidence,
    ) else {
        panic!("the genuine record package admits");
    };
    let rows = emission.identity_rows(&package).unwrap();
    let mut key = NodeKey::new(
        "deep-original",
        vec![quire_exact::PathSegment::Text("leaf".into())],
        0,
    )
    .unwrap();
    for ordinal in 1..2000 {
        key = NodeKey::new(
            "deep-original",
            vec![quire_exact::PathSegment::Node(key)],
            ordinal,
        )
        .unwrap();
    }
    let good = String::from_utf8(rows.to_bytes(&package).unwrap()).unwrap();
    let original =
        qsl_foundation::node_wire::node_key_to_text(&rows.source_records()[0].record_key).unwrap();
    let deep = qsl_foundation::node_wire::node_key_to_text(&key).unwrap();
    assert!(good.contains(&original));
    let input = good.replace(&original, &deep);
    assert!(
        matches!(
            crate::ReadIdentityRows::read(input.as_bytes(), &package),
            Err(crate::IdentityFileRefusal::Malformed { .. })
        ),
        "a deeply nested input returns a typed refusal"
    );
    let mut source_records = rows.source_records().to_vec();
    source_records[0].record_key = key;
    let carried = crate::ReadIdentityRows::of_rows(
        &package,
        rows.declarations().to_vec(),
        rows.fields().to_vec(),
        source_records,
        rows.selected_populations().to_vec(),
    );
    assert_eq!(
        carried.to_bytes(&package),
        Err(crate::IdentityFileRefusal::Malformed {
            member: crate::IdentityMember::SourceRecords
        }),
        "the public writer refuses before recursive serialization or drop"
    );
}

#[trace("TC-1331", "FR-531-AC-21", "FR-517-AC-23")]
#[test]
fn required_identity_handoff_refuses_missing_or_foreign_final_root_catalogs() {
    let (emission, _, read) = super::child_composite::emit_unit(super::child_composite::UNIT);
    let Read::Verified { admitted, .. } = read else {
        panic!("the genuine model fixture admits");
    };
    assert!(!emission.selected_population_catalogs().is_empty());
    emission
        .identity_rows(&admitted)
        .expect("normal emission retains its exact root catalogs");
    let mut foreign = emission.clone();
    foreign.selected_population_catalogs[0].model_root.package = "different-final-package".into();
    assert_eq!(
        foreign.identity_rows(&admitted),
        Err(crate::IdentityFileRefusal::Malformed {
            member: crate::IdentityMember::SelectedPopulations
        })
    );
    let special = crate::emit::emit_timed_models(&admitted, &[])
        .expect("the genuine special writer can still emit its temporal-only output");
    assert!(!special.package().bytes().is_empty());
    assert_eq!(special.identity_rows(&admitted), Err(crate::IdentityFileRefusal::Malformed { member: crate::IdentityMember::SelectedPopulations }), "requesting a required carrier must not turn absent producer catalogs into an empty supplier");
}

/// The declared fields, node bindings and emitted ids of `Tagged`.
type GuardInputs = (
    Vec<quire_semantic_value::declaration::FieldDeclaration>,
    Vec<(String, NodeKey)>,
    HashMap<NodeKey, CheckedNodeId>,
);

/// The declared fields of `Tagged` and the bindings its node holds.
fn guard_inputs(package: &CheckedPackage) -> GuardInputs {
    let CompositeShape::Record(declared) = declaration(package, "Tagged").shape() else {
        panic!("Tagged is a record");
    };
    let graph = package.graph().semantic_graph();
    let node_key = graph
        .composite_node(&declaration(package, "Tagged").key())
        .expect("the record node");
    let bindings = graph
        .node(node_key.clone())
        .expect("the record node")
        .record_member_nodes()
        .expect("a record body")
        .into_iter()
        .map(|(name, key)| (name.to_owned(), key))
        .collect();
    let emitted = graph
        .nodes()
        .map(|node| (node.key(), node_id(node.key())))
        .collect();
    (declared.to_vec(), bindings, emitted)
}

#[trace("FR-092-AC-21", "TC-1021")]
#[test]
fn bindings_that_disagree_with_the_declaration_are_a_writer_fault() {
    let (package, _, _) = emitted(RECORDS);
    let (declared, bindings, emitted) = guard_inputs(&package);
    let call = |bindings: &[(String, NodeKey)], emitted: &HashMap<NodeKey, CheckedNodeId>| {
        crate::emit::record_member_rows(
            &declared,
            bindings
                .iter()
                .map(|(n, k)| (n.as_str(), k.clone()))
                .collect(),
            emitted,
        )
    };
    assert_eq!(call(&bindings, &emitted).expect("they agree").len(), 3);

    let mut fewer = bindings.clone();
    fewer.pop();
    assert_eq!(call(&fewer, &emitted), Err(EmitRefusal::SourceMapInvariant));

    // A writer that renames a binding changes only the admitted name; the
    // declared name stays the original member identity.
    let mut renamed = bindings.clone();
    renamed[1].0 = "annotation".to_owned();
    let rows = call(&renamed, &emitted).expect("a renamed binding still joins");
    let names: Vec<(&str, &str)> = rows
        .iter()
        .map(|m| {
            (
                m.original_member_identity.as_ref(),
                m.admitted_name.as_ref(),
            )
        })
        .collect();
    assert_eq!(
        names,
        [("p", "p"), ("note", "annotation"), ("maybe", "maybe")]
    );

    let mut permuted = bindings.clone();
    permuted.swap(0, 1);
    assert_eq!(
        call(&permuted, &emitted),
        Err(EmitRefusal::SourceMapInvariant),
        "the declared names in another order would pair fields with the wrong bindings"
    );

    assert_eq!(
        call(&bindings, &HashMap::new()),
        Err(EmitRefusal::SourceMapInvariant),
        "a type node that was not emitted"
    );
}

#[trace("FR-092-AC-21", "TC-1021")]
#[test]
fn an_emitted_source_record_node_no_declaration_joins_to_is_a_writer_fault() {
    let (package, emission, _) = emitted(RECORDS);
    let emitted: HashMap<NodeKey, CheckedNodeId> = package
        .graph()
        .semantic_graph()
        .nodes()
        .map(|node| (node.key(), node_id(node.key())))
        .collect();
    let graph = package.graph();
    let all =
        crate::emit::source_record_rows_over(graph, &emitted, graph.scope().types().composites())
            .expect("every declared record joins");
    assert_eq!(all.len(), emission.selected_source_record_rows().len());

    let without_tagged = graph
        .scope()
        .types()
        .composites()
        .filter(|declaration| declaration.name() != "Tagged");
    assert_eq!(
        crate::emit::source_record_rows_over(graph, &emitted, without_tagged),
        Err(EmitRefusal::SourceMapInvariant),
        "an emitted record node with no matching declaration gets a fault, not no row"
    );
}
