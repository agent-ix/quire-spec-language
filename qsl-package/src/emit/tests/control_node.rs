// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-064: `is_control_node` decodes a node's `(node_tag, semantic_form)`
//! through IR's closed `CheckedNodeKind` vocabulary, so only a real protocol
//! `control` is a control node. Every case is built from typed kinds.

use super::*;

/// A node of `tag` and `form`, with no declaration or occurrences.
fn node_of_wire(tag: CheckedNodeTag, form: &str) -> CheckedSemanticNodeV2 {
    let key = || {
        quire_exact::NodeKey::new(
            "pkg".to_owned(),
            vec![quire_exact::PathSegment::Text("control".to_owned())],
            0,
        )
        .expect("a fixture key is a valid structural identity")
    };
    let id = crate::emit::checked_node_id_of(&key());
    CheckedSemanticNodeV2 {
        owner_model: None,
        node_id: crate::emit::checked_node_id_of(&key()),
        schema_version: GRAPH_V2.into(),
        node_tag: tag.as_wire().into(),
        semantic_form: form.into(),
        semantic_type: id,
        dependencies: Vec::new(),
        occurrences: Vec::new(),
        recursion_group: None,
        declaration: None,
        owner: None,
        body: CheckedTerm::Aggregate {
            members: Vec::new(),
        },
    }
}

#[trace("FR-064-AC-1")]
#[test]
fn a_real_protocol_control_node_is_a_control_node() {
    let control = CheckedNodeKind::Protocol(ProtocolForm::Control);
    assert!(is_control_node(&node_of_wire(
        control.tag(),
        control.form_wire()
    )));
}

#[trace("FR-064-AC-1")]
#[test]
fn a_control_form_under_a_non_protocol_tag_is_not_a_control_node() {
    let control = CheckedNodeKind::Protocol(ProtocolForm::Control);
    let foreign: Vec<CheckedNodeTag> = CheckedNodeTag::ALL
        .iter()
        .copied()
        .filter(|tag| *tag != control.tag())
        .collect();
    assert!(!foreign.is_empty());
    for tag in foreign {
        assert!(
            !is_control_node(&node_of_wire(tag, control.form_wire())),
            "{tag:?}"
        );
    }
}

#[trace("FR-064-AC-1")]
#[test]
fn only_the_protocol_control_kind_is_a_control_node() {
    let control = CheckedNodeKind::Protocol(ProtocolForm::Control);
    for kind in CheckedNodeKind::all() {
        assert_eq!(
            is_control_node(&node_of_wire(kind.tag(), kind.form_wire())),
            kind == control,
            "{kind:?}"
        );
    }
}
