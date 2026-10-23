// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-337 "Outputs" and "Behavior", FR-337-AC-5 (TC-1079): the I2 lookup of
//! a temporal clause's root from an admitted `quire.checked-package/v2`.
//!
//! Each `claim` occurrence of a `temporal`/`temporal_clause` node is one
//! declaration of the clause: its identity triple is `[node, "claim",
//! ordinal]` (an O-07 occurrence key), and its `over` node is the
//! `value`/`parameter` node the first argument of its
//! `quire.op.temporal.clause` application references. A consumer takes
//! both from the package alone (FR-337 "Behavior"), never from its caller.
//!
//! A triple naming a node the package does not hold, a node that is not a
//! `temporal`/`temporal_clause` node, or an occurrence the node does not
//! carry (another role, another ordinal) is [`ClauseRootUnresolved`]: a
//! lookup result, not an FR-370 package refusal, since the package itself
//! is well formed. The caller decides what it means (FR-338).

use quire_contract_model::{
    CheckedNodeKind, CheckedNodeTag, CheckedOccurrence, CheckedOccurrenceRole, CheckedPackageV2,
    CheckedSemanticNodeV2, TemporalForm,
};

use qsl_foundation::digest::WireNodeId;
use qsl_foundation::source::provenance::OccurrenceKey;
use quire_exact::{Origin, Role};
use quire_semantic_value::catalog::InternalFault;

/// One declaration of an admitted temporal clause: its identity triple and
/// its `over` node (FR-337 "Outputs").
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ClauseRoot {
    occurrence: OccurrenceKey,
    over: Option<WireNodeId>,
}

impl ClauseRoot {
    /// The identity triple `[node, "claim", ordinal]`.
    pub fn occurrence(&self) -> &OccurrenceKey {
        &self.occurrence
    }

    /// The `value`/`parameter` node of the clause's `over` parameter, or
    /// `None` for a clause with no `over` parameter (a valid result, not a
    /// refusal).
    pub fn over(&self) -> Option<WireNodeId> {
        self.over
    }
}

/// `clause-root-unresolved`: a looked-up clause root names no `claim`
/// occurrence of a `temporal`/`temporal_clause` node of the package
/// (FR-337 "Behavior").
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[error("clause root {root} names no temporal clause of the package")]
pub struct ClauseRootUnresolved {
    /// The looked-up triple.
    pub root: OccurrenceKey,
}

impl ClauseRootUnresolved {
    /// The lookup refusal's tag.
    pub const TAG: &'static str = "clause-root-unresolved";
}

/// A broken invariant between IR's admission and this lookup.
const NODE_ID_FAULT: InternalFault = InternalFault::new("I2", "admitted-node-id-is-a-wire-node-id");

/// Whether `node` is a `temporal`/`temporal_clause` node.
fn is_clause(node: &CheckedSemanticNodeV2) -> bool {
    CheckedNodeTag::from_wire(&node.node_tag)
        .and_then(|tag| CheckedNodeKind::decode(tag, &node.semantic_form))
        == Some(CheckedNodeKind::Temporal(TemporalForm::TemporalClause))
}

/// The `over` node of the clause node `node`: the `target` of its body
/// application's first argument, which IR's reader admitted as a reference
/// to a `value`/`parameter` node (QSpec FR-370 reader order). `None` when
/// that argument names no target.
fn over_of(node: &CheckedSemanticNodeV2) -> Option<WireNodeId> {
    let over = node.body.get("arguments")?.get(0)?;
    WireNodeId::from_hex(over.get("target")?.get("digest")?.as_str()?)
}

/// The clause root of `node`'s `claim` occurrence `occurrence`.
fn root(node: WireNodeId, occurrence: &CheckedOccurrence, over: Option<WireNodeId>) -> ClauseRoot {
    ClauseRoot {
        occurrence: OccurrenceKey::new(
            node,
            Origin::new(
                Role::new(super::role_spelling(&CheckedOccurrenceRole::Claim)),
                occurrence.ordinal,
            ),
        ),
        over,
    }
}

/// Every declaration of every temporal clause of `package`: one
/// [`ClauseRoot`] per `claim` occurrence, in graph order and then
/// occurrence order.
pub fn clause_roots(package: &CheckedPackageV2) -> Result<Vec<ClauseRoot>, InternalFault> {
    let mut roots = Vec::new();
    for node in package.graph().nodes.iter().filter(|node| is_clause(node)) {
        let id = WireNodeId::from_hex(&node.node_id.digest).ok_or(NODE_ID_FAULT)?;
        let over = over_of(node);
        roots.extend(
            node.occurrences
                .iter()
                .filter(|occurrence| occurrence.role == CheckedOccurrenceRole::Claim)
                .map(|occurrence| root(id, occurrence, over)),
        );
    }
    Ok(roots)
}

/// Resolve the clause root `triple` in `package`: the declaration it names,
/// or [`ClauseRootUnresolved`] at `triple`.
pub fn resolve_clause_root(
    package: &CheckedPackageV2,
    triple: &OccurrenceKey,
) -> Result<ClauseRoot, ClauseRootUnresolved> {
    let unresolved = || ClauseRootUnresolved {
        root: triple.clone(),
    };
    let digest = triple.node().to_string();
    let node = package
        .graph()
        .nodes
        .iter()
        .find(|node| *node.node_id.digest == *digest)
        .filter(|node| is_clause(node))
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
    Ok(root(triple.node(), occurrence, over_of(node)))
}
