// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-1398 acceptance evidence. No current QSL spec criterion covers
//! selected model tuple witness row production.

use super::*;
use qsl_semantics::model::intake::meaning;

#[path = "../../../../qsl-semantics/tests/support/tc235_k2.rs"]
mod fixture;

fn tuple_document() -> Vec<u8> {
    let tuple = fixture::declaration(
        "Pair",
        "tuple",
        json!({
            "positions": [
                {
                    "identity": fixture::id("Pair/p0"),
                    "typeRef": "ix://quire/native/Boolean",
                    "origin": fixture::origin("Pair/p0"),
                },
                {
                    "identity": fixture::id("Pair/p1"),
                    "typeRef": "ix://quire/native/Boolean",
                    "origin": fixture::origin("Pair/p1"),
                },
            ],
        }),
    );
    let tuple_construct = json!({
        "kind": {"module": fixture::BUSINESS, "name": "tuple"},
        "moduleVersion": "1.0.0",
        "construct": {
            "identity": "none",
            "shape": "tuple",
            "members": {"positions": "required"},
            "meaning": meaning::TUPLE_VALUE_TYPE,
        },
    });
    serde_json::to_vec(&fixture::domain_document(
        vec![tuple],
        vec![tuple_construct],
    ))
    .expect("the tuple model document serializes")
}

fn emit_with_tuple_source(tuple_is_referenced: bool) -> Emission {
    let document = tuple_document();
    let packages = qsl_semantics::model::intake::package_input([document.as_slice()])
        .expect("the tuple package is admitted");
    let function = if tuple_is_referenced {
        "function keep using v(pair: M::Pair): M::Pair pure { pair }"
    } else {
        "function keep using v(value: Boolean): Boolean pure { value }"
    };
    let unit = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.model.complete/v1\";\n\
         model M = {:?};\n{function}\n",
        fixture::PACKAGE,
    );
    let graph = assemble_with_models(unit.as_bytes(), &packages)
        .expect("the unit assembles")
        .check(CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("the unit checks: {refusals:?}"));
    emit_checked(&CheckedPackage::link(graph)).expect("the package emits")
}

/// A selected tuple row names the full structural id of the emitted model
/// tuple; an unreferenced tuple has no row or node.
#[test]
fn selected_tuple_rows_match_emitted_model_nodes_and_skip_absent_nodes() {
    let emission = emit_with_tuple_source(true);
    let emitted_wire = wire(&emission);
    let tuple_nodes: Vec<&Value> = nodes(&emitted_wire)
        .iter()
        .filter(|node| node["node_tag"] == "model" && node["semantic_form"] == "tuple_value_type")
        .collect();
    let [tuple_node] = tuple_nodes.as_slice() else {
        panic!("one selected model tuple node is emitted")
    };
    let [row] = emission.selected_tuple_declaration_rows() else {
        panic!("one tuple witness accompanies the emitted tuple")
    };
    assert_eq!(row.selected_document.as_ref(), fixture::PACKAGE);
    assert_eq!(row.original_identity.as_ref(), fixture::id("Pair").as_str());
    assert_eq!(
        row.node_key,
        qsl_semantics::model::key::model_node_key(
            fixture::PACKAGE,
            "model",
            "tuple_value_type",
            &fixture::id("Pair"),
        )
        .expect("the selected model tuple key is valid")
    );
    assert_eq!(
        serde_json::to_value(&row.structural_node).expect("the row id serializes"),
        tuple_node["node_id"]
    );

    let absent = emit_with_tuple_source(false);
    assert!(
        absent.selected_tuple_declaration_rows().is_empty(),
        "a row is omitted when its selected tuple node is not emitted"
    );
    assert!(!nodes(&wire(&absent)).iter().any(|node| {
        node["node_tag"] == "model" && node["semantic_form"] == "tuple_value_type"
    }));
}
