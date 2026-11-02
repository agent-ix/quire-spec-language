// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-454 "Outputs" and "Behavior" (TC-1202): the I2 lookup of a
//! schedulability claim from an admitted `quire.checked-package/v2`.
//!
//! Each `claim` occurrence of a `claim`/`schedulability` node is one
//! declaration of the claim: its identity triple is `[node, "claim",
//! ordinal]` (an O-07 occurrence key), and IR's typed view of the node (QSpec
//! FR-482) gives its kind, policy, `response` task and bound, and task set.
//! The lookup is separate from [`super::resolve_clause_root`] and
//! [`super::resolve_state_graph_root`]: each accepts only its own node form.
//!
//! A triple naming a node the package does not hold, a node that is not a
//! `claim`/`schedulability` node, or an occurrence the node does not carry
//! (another role, another ordinal) is [`ClauseRootUnresolved`], a lookup
//! result and not a package refusal.

use quire_contract_model::{
    CheckedNodeKind, CheckedNodeTag, CheckedOccurrenceRole, CheckedPackageV2,
    CheckedSchedulabilityClaimRef, CheckedSemanticNodeV2, ClaimForm,
};

use qsl_foundation::source::provenance::OccurrenceKey;
use quire_exact::{Origin, Role};
use quire_semantic_value::catalog::InternalFault;

use super::ClauseRootUnresolved;

/// One declaration of an admitted schedulability claim: its identity triple
/// and IR's typed view of the claim and its task set (FR-454 "Outputs").
#[derive(Clone, Debug)]
pub struct SchedulabilityRoot<'a> {
    occurrence: OccurrenceKey,
    claim: CheckedSchedulabilityClaimRef<'a>,
}

impl<'a> SchedulabilityRoot<'a> {
    /// The identity triple `[node, "claim", ordinal]`.
    pub fn occurrence(&self) -> &OccurrenceKey {
        &self.occurrence
    }

    /// The admitted claim: kind, policy, `response` task and bound, and the
    /// task set.
    pub fn claim(&self) -> &CheckedSchedulabilityClaimRef<'a> {
        &self.claim
    }
}

/// A broken invariant between IR's admission and this lookup.
const NODE_ID_FAULT: InternalFault =
    InternalFault::new("I2", "admitted-node-id-is-a-structural-node-id");

/// Whether `node` is a `claim`/`schedulability` node.
fn is_schedulability(node: &CheckedSemanticNodeV2) -> bool {
    CheckedNodeTag::from_wire(&node.node_tag)
        .and_then(|tag| CheckedNodeKind::decode(tag, &node.semantic_form))
        == Some(CheckedNodeKind::Claim(ClaimForm::Schedulability))
}

fn triple(key: &quire_exact::NodeKey, ordinal: u64) -> OccurrenceKey {
    OccurrenceKey::new(
        key.clone(),
        Origin::new(
            Role::new(super::role_spelling(&CheckedOccurrenceRole::Claim)),
            ordinal,
        ),
    )
}

/// Every declaration of every schedulability claim of `package`: one
/// [`SchedulabilityRoot`] per `claim` occurrence, in IR's canonical order.
pub fn schedulability_roots(
    package: &CheckedPackageV2,
) -> Result<Vec<SchedulabilityRoot<'_>>, InternalFault> {
    let graph = super::wire_graph(package).map_err(|_| NODE_ID_FAULT)?;
    package
        .schedulability_claims()
        .map(|claim| {
            let key = graph.key(claim.declaration()).map_err(|_| NODE_ID_FAULT)?;
            Ok(SchedulabilityRoot {
                occurrence: triple(&key, claim.occurrence().ordinal),
                claim,
            })
        })
        .collect()
}

/// Resolve the schedulability claim `triple` in `package`: the declaration
/// it names, or [`ClauseRootUnresolved`] at `triple` when it names no
/// `claim` occurrence of a `claim`/`schedulability` node of the package
/// (FR-454 "Behavior").
pub fn resolve_schedulability_root<'a>(
    package: &'a CheckedPackageV2,
    triple: &OccurrenceKey,
) -> Result<Result<SchedulabilityRoot<'a>, ClauseRootUnresolved>, InternalFault> {
    let unresolved = || ClauseRootUnresolved {
        root: triple.clone(),
    };
    let graph = super::wire_graph(package).map_err(|_| NODE_ID_FAULT)?;
    let Some(node) = graph
        .index_of(triple.node())
        .and_then(|at| package.graph().nodes.get(usize::try_from(at).ok()?))
        .filter(|node| is_schedulability(node))
    else {
        return Ok(Err(unresolved()));
    };
    if super::occurrence_role(triple.origin().role().as_str()) != Some(CheckedOccurrenceRole::Claim)
    {
        return Ok(Err(unresolved()));
    }
    let found = package.schedulability_claims().find(|claim| {
        claim.declaration() == &node.node_id
            && claim.occurrence().ordinal == triple.origin().ordinal()
    });
    Ok(found
        .ok_or_else(unresolved)
        .map(|claim| SchedulabilityRoot {
            occurrence: triple.clone(),
            claim,
        }))
}
