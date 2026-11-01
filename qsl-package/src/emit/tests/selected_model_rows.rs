// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-1398 acceptance evidence. No current QSL spec criterion covers
//! selected model tuple witness row production.

use super::*;
use qsl_semantics::model::intake::meaning;

#[path = "../../../../qsl-semantics/tests/support/tc235_k2.rs"]
#[allow(
    dead_code,
    reason = "this test uses only part of the shared K2 fixture"
)]
#[allow(
    clippy::duplicate_mod,
    reason = "a sibling test module also loads the K2 builder"
)]
mod fixture;

fn tuple_document(include_tuple: bool) -> Vec<u8> {
    if !include_tuple {
        return serde_json::to_vec(&fixture::k2())
            .expect("the package without the tuple serializes");
    }
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

fn emit_with_tuple_source(tuple_is_referenced: bool) -> (CheckedPackage, Emission, Vec<u8>) {
    let document = tuple_document(tuple_is_referenced);
    let packages = qsl_semantics::model::intake::package_input([document.as_slice()])
        .expect("the tuple package is admitted");
    let function = if tuple_is_referenced {
        "function keep using v(x: M::Pair): M::Pair pure { x }"
    } else {
        "function keep using v(): Boolean pure { true }"
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
    let checked = CheckedPackage::link(graph);
    let emission = emit_checked(&checked).expect("the package emits");
    (checked, emission, document)
}

/// A selected tuple row names the full structural id of the emitted model
/// tuple; a package without that tuple has no row or node.
#[test]
fn selected_tuple_rows_match_emitted_model_nodes_and_skip_absent_nodes() {
    let (_, emission, _) = emit_with_tuple_source(true);
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

    let (_, absent, _) = emit_with_tuple_source(false);
    assert!(
        absent.selected_tuple_declaration_rows().is_empty(),
        "a row is omitted when its selected tuple node is not emitted"
    );
    assert!(!nodes(&wire(&absent)).iter().any(|node| {
        node["node_tag"] == "model" && node["semantic_form"] == "tuple_value_type"
    }));
}

/// FR-352/FR-354: QSL's model-intake rows join the admitted runtime type view;
/// IR refuses the same tuple when its producer witness is omitted.
#[test]
fn selected_tuple_row_projects_the_admitted_type_and_missing_row_refuses() {
    use crate::checked_v2::{read_checked_package_v2, V2ReadLimits};
    use quire_contract_model::ExactTypeProjectionErrorCode;

    let (checked, emission, document) = emit_with_tuple_source(true);
    let mut evidence = emission.evidence().clone();
    evidence
        .insert_domain_package_document(package_identity(&document), document.clone())
        .expect("the selected model document is evidence for admission");
    let read = read_checked_package_v2(
        emission.package().bytes(),
        library(),
        V2ReadLimits::default(),
        &evidence,
    )
    .expect("IR admits the producer rows' package");
    let admitted = read.into_value().admitted;
    let identities = checked
        .graph()
        .selected_exact_identities(&admitted)
        .expect("the selected declaration and field rows join admission");

    let projected = admitted
        .project_selected_type_declarations(
            &identities,
            emission.selected_tuple_declaration_rows(),
            emission.selected_source_unit_rows(),
            10_000,
        )
        .expect("the complete selected model type view joins");
    let [tuple] = projected.tuples() else {
        panic!("the selected tuple is projected exactly once");
    };
    assert_eq!(tuple.original_identity(), fixture::id("Pair"));
    assert_eq!(
        tuple.node_key(),
        &emission.selected_tuple_declaration_rows()[0].node_key
    );
    let refusal = admitted
        .project_selected_type_declarations(
            &identities,
            &[],
            emission.selected_source_unit_rows(),
            10_000,
        )
        .expect_err("a missing selected tuple row refuses the complete projection");
    assert_eq!(
        refusal.code,
        ExactTypeProjectionErrorCode::UnavailableIdentityWitness
    );
}
