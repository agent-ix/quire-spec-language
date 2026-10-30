// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1263 (FR-485-AC-13): a downstream consumer obtains owner-checked nominal
//! declarations through the public replay facade and shared kernel/value APIs.

use ix_trace_rs::trace;
use qsl_replay::{
    acquire_checked_closure, DependencyInput, NominalDeclarationForm, NominalDeclarationMember,
    ReplayLimits, SourceIdentity, StageLimits, NodeKey,
};
use quire_exact::ValueType;
use serde_json::Value as JsonValue;

const SOURCE: &str = "language \"ix:native\" edition \"1-draft\";\n\
profile v = \"quire.value.complete/v1\";\n\
tuple Pair(Boolean, Integer);\n\
union Shape { Empty, Circle(Integer) }\n\
record Holder { datum: Integer; pair: Pair; shape: Shape; }\n\
function consume using v(x: Holder): Boolean pure { true }\n";

fn node_id(package: &[u8], tag: &str, name: &str) -> NodeKey {
    let package: JsonValue = serde_json::from_slice(package).expect("checked package JSON");
    package["semantic_graph"]["nodes"]
        .as_array()
        .expect("semantic graph nodes")
        .iter()
        .find(|node| {
            node["node_tag"] == tag
                && node["declaration"]["qualified_name"]
                    .as_array()
                    .and_then(|segments| segments.last())
                    .and_then(JsonValue::as_str)
                    == Some(name)
        })
        .and_then(|node| node["node_id"]["digest"].as_str())
        .and_then(NodeKey::from_hex)
        .expect("matching node id")
}

#[trace("TC-1263", "FR-485-AC-13")]
#[test]
fn public_consumer_reads_complete_ordered_nominal_members_from_one_selection() {
    let acquisition = acquire_checked_closure(
        SourceIdentity::new("a", "unit", "git", "1"),
        "unit.native",
        SOURCE.as_bytes(),
        [],
        &DependencyInput::default(),
        StageLimits::default(),
        ReplayLimits::default(),
    )
    .expect("checked closure");
    let function = node_id(acquisition.package().bytes(), "function", "consume");
    let holder_id = node_id(acquisition.package().bytes(), "composite_type", "Holder");
    let pair_id = node_id(acquisition.package().bytes(), "composite_type", "Pair");
    let shape_id = node_id(acquisition.package().bytes(), "composite_type", "Shape");
    let handle = acquisition.handle();
    let selection = handle
        .lookup_callable(handle.package_id(), function)
        .expect("checked callable selection");

    let holder = handle
        .nominal_declaration(&selection, holder_id)
        .expect("signature's record declaration");
    assert_eq!(holder.package_id(), handle.package_id());
    assert_eq!(
        selection.signature().parameters(),
        &[ValueType::Composite(holder.key())]
    );
    assert_eq!(holder.node().as_bytes(), holder_id.as_bytes());
    assert_eq!(holder.form(), NominalDeclarationForm::Record);
    assert!(matches!(
        holder.members(),
        [
            NominalDeclarationMember::RecordField { identifier, value_type: ValueType::Integer },
            NominalDeclarationMember::RecordField { identifier: second, value_type: ValueType::Composite(_) },
            NominalDeclarationMember::RecordField { identifier: third, value_type: ValueType::Composite(_) },
        ] if identifier.as_str() == "datum" && second.as_str() == "pair" && third.as_str() == "shape"
    ));

    let pair = handle
        .nominal_declaration(&selection, pair_id)
        .expect("nested tuple declaration");
    assert_eq!(pair.form(), NominalDeclarationForm::Tuple);
    assert!(matches!(
        pair.members(),
        [
            NominalDeclarationMember::TuplePosition {
                position: 0,
                value_type: ValueType::Boolean
            },
            NominalDeclarationMember::TuplePosition {
                position: 1,
                value_type: ValueType::Integer
            },
        ]
    ));

    let shape = handle
        .nominal_declaration(&selection, shape_id)
        .expect("nested union declaration");
    assert_eq!(shape.form(), NominalDeclarationForm::Union);
    let [NominalDeclarationMember::UnionMember {
        member: empty,
        variant: empty_variant,
        payload: empty_payload,
    }, NominalDeclarationMember::UnionMember {
        member: circle,
        variant: circle_variant,
        payload: circle_payload,
    }] = shape.members()
    else {
        panic!("all declared union variants are available in order");
    };
    assert_eq!(empty.identifier().as_str(), "Empty");
    assert_eq!(empty.variant(), *empty_variant);
    assert!(empty_payload.is_empty());
    assert_eq!(circle.identifier().as_str(), "Circle");
    assert_eq!(circle.variant(), *circle_variant);
    assert_eq!(circle_payload, &[ValueType::Integer]);
}
