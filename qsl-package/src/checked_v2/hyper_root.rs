// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-488 "Outputs" (TC-1266): the I2 lookup of a hyper clause's root from an
//! admitted `quire.checked-package/v2`.
//!
//! Each `claim` occurrence of a `claim`/`hyperproperty` node is one
//! declaration of the clause: its identity triple is `[node, "claim",
//! ordinal]` (an O-07 occurrence key), the `clause` member of FR-315's
//! `HyperProductIdentity`. Its body wrapper is the `temporal`/
//! `temporal_clause` node the node's `body` binding references, which is
//! no clause root of its own ([`super::clause_roots`] skips it).
//!
//! A triple naming a node the package does not hold, a node that is not a
//! `claim`/`hyperproperty` node, or an occurrence the node does not carry
//! is [`HyperRootUnresolved`]: a lookup result, not an FR-370 refusal.

use std::collections::BTreeSet;

use quire_contract_model::{
    CheckedNodeKind, CheckedNodeTag, CheckedOccurrenceRole, CheckedPackageV2,
    CheckedSemanticNodeV2, CheckedTerm, ClaimForm,
};

use qsl_foundation::source::provenance::OccurrenceKey;
use quire_contract_model::CheckedNodeId;
use quire_exact::{NodeKey, Origin, Role};

use quire_semantic_value::catalog::InternalFault;

/// One declaration of an admitted hyper clause: its identity triple and its
/// body wrapper node (FR-488 "Outputs").
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HyperRoot {
    occurrence: OccurrenceKey,
    body: Option<NodeKey>,
}

impl HyperRoot {
    /// The identity triple `[node, "claim", ordinal]`.
    pub fn occurrence(&self) -> &OccurrenceKey {
        &self.occurrence
    }

    /// The `temporal`/`temporal_clause` node the clause's `body` binding
    /// references, or `None` for a node whose body names none.
    pub fn body(&self) -> Option<&NodeKey> {
        self.body.as_ref()
    }
}

/// `hyper-root-unresolved`: a looked-up hyper root names no `claim`
/// occurrence of a `claim`/`hyperproperty` node of the package (FR-488
/// "Outputs").
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[error("hyper root {root} names no hyper clause of the package")]
pub struct HyperRootUnresolved {
    /// The looked-up triple.
    pub root: OccurrenceKey,
}

impl HyperRootUnresolved {
    /// The lookup refusal's tag.
    pub const TAG: &'static str = "hyper-root-unresolved";
}

/// A broken invariant between IR's admission and this lookup.
const NODE_ID_FAULT: InternalFault =
    InternalFault::new("I2", "admitted-node-id-is-a-structural-node-id");

/// Whether `node` is a `claim`/`hyperproperty` node.
fn is_hyper(node: &CheckedSemanticNodeV2) -> bool {
    CheckedNodeTag::from_wire(&node.node_tag)
        .and_then(|tag| CheckedNodeKind::decode(tag, &node.semantic_form))
        == Some(CheckedNodeKind::Claim(ClaimForm::Hyperproperty))
}

/// The node id the `body` binding of the hyper node `node` references. The
/// binding's wire name is the one place the written text is read (FR-064's
/// string-edge): FR-402 names the member `body`.
#[ix_trace_rs::string_edge]
fn body_id(node: &CheckedSemanticNodeV2) -> Option<&CheckedNodeId> {
    let CheckedTerm::Aggregate { members } = &node.body else {
        return None;
    };
    members.iter().find_map(|member| match member {
        CheckedTerm::Binding { name, value } if &**name == "body" => match &**value {
            CheckedTerm::Reference { target } => Some(target),
            _ => None,
        },
        _ => None,
    })
}

/// The node ids of the body wrappers of every hyper clause of `package`:
/// `temporal`/`temporal_clause` nodes that are no clause of their own.
pub(super) fn wrappers(package: &CheckedPackageV2) -> BTreeSet<CheckedNodeId> {
    package
        .graph()
        .nodes
        .iter()
        .filter(|node| is_hyper(node))
        .filter_map(|node| body_id(node).cloned())
        .collect()
}

fn root(node: &NodeKey, ordinal: u64, body: Option<NodeKey>) -> HyperRoot {
    HyperRoot {
        occurrence: OccurrenceKey::new(
            node.clone(),
            Origin::new(
                Role::new(super::role_spelling(&CheckedOccurrenceRole::Claim)),
                ordinal,
            ),
        ),
        body,
    }
}

/// Every declaration of every hyper clause of `package`: one [`HyperRoot`]
/// per `claim` occurrence, in graph order and then occurrence order.
pub fn hyper_roots(package: &CheckedPackageV2) -> Result<Vec<HyperRoot>, InternalFault> {
    let mut roots = Vec::new();
    let graph = super::wire_graph(package).map_err(|_| NODE_ID_FAULT)?;
    for (node, id) in package
        .graph()
        .nodes
        .iter()
        .zip(graph.keys())
        .filter(|(node, _)| is_hyper(node))
    {
        let body = body_id(node).and_then(|body| graph.key(body).ok());
        roots.extend(
            node.occurrences
                .iter()
                .filter(|occurrence| occurrence.role == CheckedOccurrenceRole::Claim)
                .map(|occurrence| root(id, occurrence.ordinal, body.clone())),
        );
    }
    Ok(roots)
}

/// Resolve the hyper root `triple` in `package`: the declaration it names,
/// or [`HyperRootUnresolved`] at `triple`.
pub fn resolve_hyper_root(
    package: &CheckedPackageV2,
    triple: &OccurrenceKey,
) -> Result<HyperRoot, HyperRootUnresolved> {
    let unresolved = || HyperRootUnresolved {
        root: triple.clone(),
    };
    let graph = super::wire_graph(package).map_err(|_| unresolved())?;
    let node = graph
        .index_of(triple.node())
        .and_then(|at| package.graph().nodes.get(usize::try_from(at).ok()?))
        .filter(|node| is_hyper(node))
        .ok_or_else(unresolved)?;
    if super::occurrence_role(triple.origin().role().as_str()) != Some(CheckedOccurrenceRole::Claim)
    {
        return Err(unresolved());
    }
    let occurrence = node
        .occurrences
        .iter()
        .find(|occurrence| {
            occurrence.role == CheckedOccurrenceRole::Claim
                && occurrence.ordinal == triple.origin().ordinal()
        })
        .ok_or_else(unresolved)?;
    Ok(root(
        triple.node(),
        occurrence.ordinal,
        body_id(node).and_then(|body| graph.key(body).ok()),
    ))
}
