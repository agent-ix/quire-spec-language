// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-390 "Source-declared operation calls from the reader", FR-531: the
//! source-operation identity rows and their admission.
//!
//! An operation anchor whose declaring type the unit's own source declares
//! names no model operation member, so the reader view cannot recover it from
//! the wire alone. One [`SourceOperationRow`] per such anchor travels in the
//! `source_operations` member of `identities.json`. The rows are read off the
//! final emission's graph by [`source_operation_rows`], and a consumer admits
//! them against the package's own nodes with [`admit`].

use std::collections::{BTreeMap, BTreeSet};

use quire_contract_model::{
    CheckedNodeId, CheckedNodeOwner, CheckedSemanticNodeV2, CheckedTerm, LiteralKind,
};

use crate::bare::IdentityWitnessRefusal;
use crate::wire_form::{has_form, has_tag, WireForm, WireTag};

/// The identity row of one source-declared `state`/`operation_anchor` node.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceOperationRow {
    /// The structural node of the anchor.
    pub structural_node: CheckedNodeId,
    /// The source authority of the declaring source unit.
    pub source_authority: String,
    /// The original source operation identity: the operation name as the
    /// source declares it, qualified by its declaring source type.
    pub source_identity: String,
    /// The full anchor id of the node, never a prefix or a display name.
    pub anchor_id: CheckedNodeId,
}

/// A source-declared operation anchor of a graph.
struct SourceAnchor<'a> {
    node: &'a CheckedNodeId,
    authority: &'a str,
    identity: String,
    reached: bool,
}

type Nodes<'a> = BTreeMap<&'a CheckedNodeId, &'a CheckedSemanticNodeV2>;

fn index(nodes: &[CheckedSemanticNodeV2]) -> Nodes<'_> {
    nodes.iter().map(|node| (&node.node_id, node)).collect()
}

fn is_anchor(node: &CheckedSemanticNodeV2) -> bool {
    has_form(node, WireTag::State, WireForm::OperationAnchor)
}

fn anchor_members(node: &CheckedSemanticNodeV2) -> Option<&[CheckedTerm]> {
    match &node.body {
        CheckedTerm::Aggregate { members } => Some(members.as_slice()),
        CheckedTerm::OperationAnchorBody(body) => Some(body.members.as_slice()),
        _ => None,
    }
}

/// The source authority and identity of the anchor's declaring type, when the
/// unit's own source declares it, with the type's qualified name.
fn declaring<'a>(nodes: &Nodes<'a>, anchor: &CheckedSemanticNodeV2) -> Option<(&'a str, String)> {
    let members = anchor_members(anchor)?;
    let context = members.iter().find_map(|member| match member {
        CheckedTerm::Binding { name, value } if name.as_ref() == "context" => Some(value.as_ref()),
        _ => None,
    })?;
    let CheckedTerm::Reference { target } = context else {
        return None;
    };
    let declaring = nodes.get(target)?;
    let Some(CheckedNodeOwner::Source { authority, .. }) = &declaring.owner else {
        return None;
    };
    let qualified = declaring.declaration.as_ref()?.qualified_name.join("::");
    Some((authority, qualified))
}

fn operation_name(anchor: &CheckedSemanticNodeV2) -> Option<&str> {
    anchor_members(anchor)?
        .iter()
        .find_map(|member| match member {
            CheckedTerm::Binding { name, value } if name.as_ref() == "operation" => {
                match &**value {
                    CheckedTerm::Literal {
                        value_kind: LiteralKind::Text,
                        value,
                        ..
                    } => value.as_str(),
                    _ => None,
                }
            }
            _ => None,
        })
}

/// The nodes a protocol node reaches through its dependencies, itself included.
fn reached_by_protocols<'a>(nodes: &Nodes<'a>) -> BTreeSet<&'a CheckedNodeId> {
    let mut reached = BTreeSet::new();
    let mut pending: Vec<&CheckedNodeId> = nodes
        .values()
        .filter(|node| has_tag(node, WireTag::Protocol))
        .map(|node| &node.node_id)
        .collect();
    while let Some(id) = pending.pop() {
        if !reached.insert(id) {
            continue;
        }
        if let Some(node) = nodes.get(id) {
            pending.extend(node.dependencies.iter());
        }
    }
    reached
}

fn source_anchors<'a>(nodes: &Nodes<'a>) -> Vec<SourceAnchor<'a>> {
    let reached = reached_by_protocols(nodes);
    nodes
        .values()
        .filter(|node| is_anchor(node))
        .filter_map(|node| {
            let (authority, qualified) = declaring(nodes, node)?;
            let operation = operation_name(node)?;
            Some(SourceAnchor {
                node: &node.node_id,
                authority,
                identity: format!("{qualified}::{operation}"),
                reached: reached.contains(&node.node_id),
            })
        })
        .collect()
}

/// The source-operation rows of a final emission's graph, in graph order
/// (FR-517): one per source-declared operation anchor.
pub(crate) fn source_operation_rows(nodes: &[CheckedSemanticNodeV2]) -> Vec<SourceOperationRow> {
    let held = index(nodes);
    let anchors = source_anchors(&held);
    nodes
        .iter()
        .filter_map(|node| {
            let anchor = anchors.iter().find(|anchor| anchor.node == &node.node_id)?;
            Some(SourceOperationRow {
                structural_node: node.node_id.clone(),
                source_authority: anchor.authority.to_owned(),
                source_identity: anchor.identity.clone(),
                anchor_id: node.node_id.clone(),
            })
        })
        .collect()
}

/// Admit `rows` against the package's `nodes` (FR-390 "View creation and
/// order of refusals"). Foreign wins over Unknown for the same row (FR-531-AC-5);
/// a source-declared anchor a protocol node reaches without a row is Missing.
///
/// Returns the first source-declared anchor a protocol node reaches, which
/// IR's FR-352 witness kind does not yet accept.
///
/// # Errors
///
/// [`IdentityWitnessRefusal`] for the first refusal in the order Foreign,
/// Unknown, Missing.
pub(crate) fn admit<'a>(
    nodes: &'a [CheckedSemanticNodeV2],
    rows: &[SourceOperationRow],
) -> Result<Option<&'a CheckedNodeId>, IdentityWitnessRefusal> {
    let held = index(nodes);
    let authorities: BTreeSet<&str> = nodes
        .iter()
        .filter_map(|node| match &node.owner {
            Some(CheckedNodeOwner::Source { authority, .. }) => Some(&**authority),
            _ => None,
        })
        .collect();
    if rows
        .iter()
        .any(|row| !authorities.contains(row.source_authority.as_str()))
    {
        return Err(IdentityWitnessRefusal::Foreign);
    }
    let anchors = source_anchors(&held);
    let mut seen = BTreeSet::new();
    for row in rows {
        let agrees = row.anchor_id == row.structural_node
            && seen.insert(&row.structural_node)
            && anchors.iter().any(|anchor| {
                anchor.node == &row.structural_node
                    && anchor.authority == row.source_authority
                    && anchor.identity == row.source_identity
            });
        if !agrees {
            return Err(IdentityWitnessRefusal::Unknown);
        }
    }
    let mut first = None;
    for anchor in anchors.iter().filter(|anchor| anchor.reached) {
        if !seen.contains(anchor.node) {
            return Err(IdentityWitnessRefusal::Missing);
        }
        first.get_or_insert(anchor.node);
    }
    Ok(first)
}
