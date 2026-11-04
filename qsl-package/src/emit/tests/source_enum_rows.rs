// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-092-AC-19: an emitted source-owned enum carries one IR
//! `SelectedEnumDeclarationRow` and one `SelectedEnumMemberRow` per case,
//! each joined to the emitted node it names; a model variant type carries no
//! source enum row.

use super::variant_types::{emitted_enums, UNIT};
use super::*;

/// A source enum read by a member literal.
pub(super) const STATUS: &str = "enum Status { READY, DONE }\n\
    function ready using v(s: Status): Boolean pure { s = Status::READY }\n";

/// The emitted wire node whose id is `id`.
fn node_with_id<'w>(wire: &'w Value, id: &quire_contract_model::CheckedNodeId) -> &'w Value {
    let id = serde_json::to_value(id).expect("a node id serializes");
    nodes(wire)
        .iter()
        .find(|node| node["node_id"] == id)
        .unwrap_or_else(|| panic!("{id} is an emitted node"))
}

#[trace("FR-092-AC-19")]
#[test]
fn a_source_enum_emits_its_declaration_and_member_rows() {
    let (_, emission) = emit_from_text(STATUS);
    let wire = wire(&emission);

    let [declaration] = emission.selected_enum_declaration_rows() else {
        panic!(
            "one source enum declaration row: {:?}",
            emission.selected_enum_declaration_rows()
        );
    };
    let declared = node_with_id(&wire, &declaration.structural_node);
    assert_eq!(
        (&declared["node_tag"], &declared["semantic_form"]),
        (&json!("scalar_type"), &json!("enum"))
    );
    assert_eq!(
        declared["owner"],
        json!({"kind": "source", "authority": "a", "identity": "u"})
    );
    assert_eq!(
        (
            declaration.source_authority.as_ref(),
            declaration.source_identity.as_ref()
        ),
        ("a", "u")
    );
    assert_eq!(declaration.qualified_declaration, [Box::from("Status")]);
    assert_eq!(
        emission
            .wire_nodes
            .iter()
            .find(|node| node.node_id == declaration.structural_node)
            .and_then(|node| node.declaration.as_ref())
            .map(|declaration| declaration.qualified_name.clone()),
        Some(vec![Box::from("Status")])
    );

    let members = emission.selected_enum_member_rows();
    assert_eq!(members.len(), 2, "{members:?}");
    let mut cases: Vec<&str> = members.iter().map(|row| row.case.as_ref()).collect();
    cases.sort_unstable();
    assert_eq!(cases, ["DONE", "READY"]);
    let declaration_id =
        serde_json::to_value(&declaration.structural_node).expect("a node id serializes");
    for member in members {
        let node = node_with_id(&wire, &member.structural_node);
        assert_eq!(
            (&node["node_tag"], &node["semantic_form"]),
            (&json!("value"), &json!("enum_value"))
        );
        assert_eq!(node["semantic_type"], declaration_id);
        assert_eq!(node["body"]["value"], json!(member.case.as_ref()));
        assert_eq!(member.declaration_node, declaration.structural_node);
        assert_eq!(member.declaration_qualified_name, [Box::from("Status")]);
        assert_eq!(
            (
                member.source_authority.as_ref(),
                member.source_identity.as_ref()
            ),
            ("a", "u")
        );
        assert_ne!(member.member_key, declaration.node_key);
    }
}

#[trace("FR-092-AC-19")]
#[test]
fn a_model_variant_type_has_no_source_enum_row() {
    let emission = emitted_enums(UNIT);
    assert!(!emission.selected_model_enum_declaration_rows().is_empty());
    assert_eq!(emission.selected_enum_declaration_rows(), []);
    assert_eq!(emission.selected_enum_member_rows(), []);
}
