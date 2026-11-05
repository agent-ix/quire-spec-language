// SPDX-License-Identifier: AGPL-3.0-or-later
//! The node-to-family reader (QSL FR-507 "Where the family is read", QSpec
//! FR-290 "Reading the family from a request item"): the declaration family
//! of the claim a request item names, read from the item's `item_kind` and
//! from the checked package's node, by the node's tag, `semantic_form` and
//! occurrence role.
//!
//! The family is a property of the claim, never of the item's kind, of the
//! kind's FR-290 kind group or of QSL's `FamilyKind` (FR-507): this module
//! takes no [`Capability`](qsl_semantics::check::Capability).

use qsl_foundation::source::provenance::OccurrenceKey;
use qsl_package::CheckedPackage;
use qsl_route::request::ItemKind;
use qsl_semantics::check::{Capability, CheckedGraph, DeclarationFamily, NodeTag};
use quire_exact::ValueType;

/// The `claim` occurrence role.
const CLAIM: &str = "claim";
/// The `declaration` occurrence role.
const DECLARATION: &str = "declaration";
/// The `expression` occurrence role.
const EXPRESSION: &str = "expression";
/// The `anchor` occurrence role.
const ANCHOR: &str = "anchor";

/// The declaration family of the claim `claim` of an item whose `item_kind`
/// is `item_kind`, over `package`; `None` when the claim has no family, so
/// that every kind is inapplicable to it (FR-507-AC-9).
///
/// An item whose `item_kind` is not `formula` takes its family from the
/// `item_kind`, whatever node it is anchored on; every other item takes the
/// family of its claim's node.
pub fn claim_family(
    package: &CheckedPackage,
    item_kind: ItemKind,
    claim: &OccurrenceKey,
) -> Option<DeclarationFamily> {
    match item_kind {
        ItemKind::DeadlockFreedom | ItemKind::TimeLockFreedom | ItemKind::RaceFreedom => {
            Some(DeclarationFamily::TemporalTrace)
        }
        ItemKind::Refinement => Some(DeclarationFamily::RelationClaim),
        ItemKind::Formula => node_family(package.graph(), claim),
    }
}

/// The family of the node `claim` names, by QSpec FR-290's node-to-family
/// table.
fn node_family(graph: &CheckedGraph, claim: &OccurrenceKey) -> Option<DeclarationFamily> {
    let role = claim.origin().role().as_str();
    let (tag, form) = node_form(graph, claim)?;
    match (tag, form, role) {
        // A Boolean value clause: the function's own claim (FR-484 form
        // `clause`).
        (NodeTag::Function, _, CLAIM | DECLARATION) => graph
            .callable_by_identity(claim.node().clone())
            .filter(|callable| *callable.result == ValueType::Boolean)
            .map(|_| DeclarationFamily::Value),
        // A scalar operation application in a `Value` function body (FR-484
        // form `scalar-operation`): an occurrence a `value-validity`
        // requirement record keys, owned by the function that holds it.
        (NodeTag::Expression, _, EXPRESSION) => graph
            .requirements()
            .get(claim)
            .filter(|record| {
                record.requirements().kind() == Capability::ValueValidity
                    && record.owner().is_some()
            })
            .map(|_| DeclarationFamily::Value),
        (NodeTag::State, "state_clause", CLAIM) | (NodeTag::State, "operation_anchor", ANCHOR) => {
            Some(DeclarationFamily::StateModel)
        }
        (NodeTag::Temporal, "temporal_clause", _)
        | (NodeTag::Claim, "state_graph" | "hyperproperty", _) => {
            Some(DeclarationFamily::TemporalTrace)
        }
        (NodeTag::Claim, "step_relation", _) | (NodeTag::Relation, "refinement", _) => {
            Some(DeclarationFamily::RelationClaim)
        }
        (NodeTag::Claim, "probabilistic_claim", _) => Some(DeclarationFamily::Probabilistic),
        (NodeTag::Claim, "schedulability", _) => Some(DeclarationFamily::RealTime),
        (NodeTag::Protocol, _, _) => Some(DeclarationFamily::Protocol),
        _ => None,
    }
}

/// The `node_tag` and `semantic_form` of the node `claim` names, or `None`
/// for a node the package does not hold. A refinement and a task-set claim
/// node are minted with the checked package (QSpec FR-375, FR-482) beside
/// its semantic graph, so the graph's own records name them.
fn node_form(graph: &CheckedGraph, claim: &OccurrenceKey) -> Option<(NodeTag, &'static str)> {
    let key = claim.node();
    if let Some(node) = graph.semantic_graph().node(key.clone()) {
        return Some((node.node_tag(), node.semantic_form()));
    }
    let refinement = graph
        .refinements()
        .iter()
        .any(|refinement| {
            refinement
                .v2_emission()
                .is_ok_and(|emission| emission.identity == *key)
        })
        .then_some((NodeTag::Relation, "refinement"));
    refinement.or_else(|| {
        graph
            .task_sets()
            .claims()
            .iter()
            .any(|schedule| schedule.node_id() == Some(key))
            .then_some((NodeTag::Claim, "schedulability"))
    })
}
