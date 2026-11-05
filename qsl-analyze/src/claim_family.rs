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

/// The `semantic_form` words the reader selects by, decoded once at
/// [`Form::from_wire`] (ADR-012 section 9).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Form {
    StateClause,
    OperationAnchor,
    TemporalClause,
    StateGraph,
    Hyperproperty,
    StepRelation,
    Refinement,
    ProbabilisticClaim,
    Schedulability,
    /// A form no row selects by.
    Other,
}

impl Form {
    /// The form a `semantic_form` word names.
    #[ix_trace_rs::string_edge]
    fn from_wire(word: &str) -> Self {
        match word {
            "state_clause" => Self::StateClause,
            "operation_anchor" => Self::OperationAnchor,
            "temporal_clause" => Self::TemporalClause,
            "state_graph" => Self::StateGraph,
            "hyperproperty" => Self::Hyperproperty,
            "step_relation" => Self::StepRelation,
            "refinement" => Self::Refinement,
            "probabilistic_claim" => Self::ProbabilisticClaim,
            "schedulability" => Self::Schedulability,
            _ => Self::Other,
        }
    }
}

/// The occurrence roles the reader selects by, decoded once at
/// [`Role::from_wire`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Role {
    Claim,
    Declaration,
    Expression,
    Anchor,
    /// A role no row selects by.
    Other,
}

impl Role {
    /// The role an occurrence role word names.
    #[ix_trace_rs::string_edge]
    fn from_wire(word: &str) -> Self {
        match word {
            "claim" => Self::Claim,
            "declaration" => Self::Declaration,
            "expression" => Self::Expression,
            "anchor" => Self::Anchor,
            _ => Self::Other,
        }
    }
}

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
    let role = Role::from_wire(claim.origin().role().as_str());
    let (tag, form) = node_form(graph, claim)?;
    match (tag, form, role) {
        // A Boolean value clause (FR-484 form `clause`), on its Boolean
        // expression node at the claim or declaration occurrence (QSpec
        // FR-290).
        (NodeTag::Expression, _, Role::Claim | Role::Declaration)
            if boolean_expression(graph, claim) =>
        {
            Some(DeclarationFamily::Value)
        }
        // A scalar operation application in a `Value` function body (FR-484
        // form `scalar-operation`): an occurrence a `value-validity`
        // requirement record keys, owned by the function that holds it.
        (NodeTag::Expression, _, Role::Expression) => graph
            .requirements()
            .get(claim)
            .filter(|record| {
                record.requirements().kind() == Capability::ValueValidity
                    && record.owner().is_some()
            })
            .map(|_| DeclarationFamily::Value),
        (NodeTag::State, Form::StateClause, Role::Claim)
        | (NodeTag::State, Form::OperationAnchor, Role::Anchor) => {
            Some(DeclarationFamily::StateModel)
        }
        (NodeTag::Temporal, Form::TemporalClause, _)
        | (NodeTag::Claim, Form::StateGraph | Form::Hyperproperty, _) => {
            Some(DeclarationFamily::TemporalTrace)
        }
        (NodeTag::Claim, Form::StepRelation, _) | (NodeTag::Relation, Form::Refinement, _) => {
            Some(DeclarationFamily::RelationClaim)
        }
        (NodeTag::Claim, Form::ProbabilisticClaim, _) => Some(DeclarationFamily::Probabilistic),
        (NodeTag::Claim, Form::Schedulability, _) => Some(DeclarationFamily::RealTime),
        (NodeTag::Protocol, _, _) => Some(DeclarationFamily::Protocol),
        _ => None,
    }
}

/// Whether `claim` names a Boolean expression node.
/// The semantic graph exposes the scalar type form as a wire word; this
/// comparison recognizes its canonical Boolean spelling at that boundary.
#[ix_trace_rs::string_edge]
fn boolean_expression(graph: &CheckedGraph, claim: &OccurrenceKey) -> bool {
    let Some(expression) = graph.semantic_graph().node(claim.node().clone()) else {
        return false;
    };
    let Some(semantic_type) = expression.semantic_type() else {
        return false;
    };
    graph
        .semantic_graph()
        .node(semantic_type)
        .is_some_and(|node| {
            node.node_tag() == NodeTag::ScalarType && node.semantic_form() == "boolean"
        })
}

/// The `node_tag` and decoded `semantic_form` of the node `claim` names, or `None`
/// for a node the package does not hold. A refinement and a task-set claim
/// node are minted with the checked package (QSpec FR-375, FR-482) beside
/// its semantic graph, so the graph's own records name them.
fn node_form(graph: &CheckedGraph, claim: &OccurrenceKey) -> Option<(NodeTag, Form)> {
    let key = claim.node();
    if let Some(node) = graph.semantic_graph().node(key.clone()) {
        return Some((node.node_tag(), Form::from_wire(node.semantic_form())));
    }
    let refinement = graph
        .refinements()
        .iter()
        .any(|refinement| {
            refinement
                .v2_emission()
                .is_ok_and(|emission| emission.identity == *key)
        })
        .then_some((NodeTag::Relation, Form::Refinement));
    refinement.or_else(|| {
        graph
            .task_sets()
            .claims()
            .iter()
            .any(|schedule| schedule.node_id() == Some(key))
            .then_some((NodeTag::Claim, Form::Schedulability))
    })
}
