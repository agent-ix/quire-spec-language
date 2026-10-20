// SPDX-License-Identifier: AGPL-3.0-or-later
//! The nodes of an abstraction relation (QSpec FR-451, QSL FR-306) that no
//! lowering run keys: the relation node itself, and the model nodes it names
//! that the unit's own functions and clauses do not.
//!
//! They are built from a checked relation after `check`, so they hold no
//! entry of the model correspondence and no occurrence of the checked graph;
//! the emitter places them.

use qsl_foundation::IdentityLimits;
use quire_exact::NodeKey;

use super::model::model_node_content;
use super::{NodeContent, SemanticNode};
use crate::check::node_key::{
    abstraction_node_key, node_key, AbstractionTerm, BodyTerm, LeafTerm, LiteralValue, MemberTerm,
    ModelOwner, NodeKeyRefusal, NodeTag,
};
use crate::model::key::DeclarationKey;

/// `content` keyed as lowering keys any node outside a recursion group.
fn detached(
    content: NodeContent,
    identity: IdentityLimits,
) -> Result<SemanticNode, NodeKeyRefusal> {
    let keyed = node_key(&content.input(), identity)?;
    Ok(SemanticNode {
        key: keyed.key,
        preimage: keyed.preimage,
        recursion: None,
        content,
        nominal: None,
    })
}

/// The builtin `scalar_type`/`text` node: its own semantic type, empty body
/// (FR-092 T3). A population node's identity literal is typed at it.
pub(crate) fn text_scalar_node(identity: IdentityLimits) -> Result<SemanticNode, NodeKeyRefusal> {
    detached(
        NodeContent {
            node_tag: NodeTag::ScalarType,
            semantic_form: "text",
            semantic_type: None,
            declaration: None,
            owner: None,
            body: BodyTerm::aggregate(Vec::new()),
        },
        identity,
    )
}

/// The `model`/`object_type` node of `declaration`, as lowering keys it: its
/// `ModelOwner`, a `null` `declaration` and an empty body. `None` when the
/// declaration's package identity or node is empty.
pub(crate) fn object_type_node(
    declaration: &DeclarationKey,
    identity: IdentityLimits,
) -> Result<Option<SemanticNode>, NodeKeyRefusal> {
    let Ok(owner) = ModelOwner::new(declaration.package.clone(), declaration.node.clone()) else {
        return Ok(None);
    };
    detached(
        model_node_content(owner, NodeTag::Model, "object_type"),
        identity,
    )
    .map(Some)
}

/// The `relation`/`population` node of `declaration`: no owner (the IR reader
/// admits none on a population), and a body that binds `identity` to the
/// population's IR node identity as a text literal typed at `text`, so each
/// population has a key of its own.
pub(crate) fn population_node(
    declaration: &DeclarationKey,
    text: NodeKey,
    identity: IdentityLimits,
) -> Result<SemanticNode, NodeKeyRefusal> {
    let literal = LeafTerm::literal(text, LiteralValue::Text(declaration.node.clone()));
    detached(
        NodeContent {
            node_tag: NodeTag::Relation,
            semantic_form: "population",
            semantic_type: None,
            declaration: None,
            owner: None,
            body: BodyTerm::aggregate(vec![MemberTerm::bound("identity", literal)]),
        },
        identity,
    )
}

/// The `correspondence`/`abstraction_relation` node holding `term`, keyed by
/// QSpec FR-451's own preimage and typed by itself.
pub(crate) fn relation_node(
    term: AbstractionTerm,
    identity: IdentityLimits,
) -> Result<SemanticNode, NodeKeyRefusal> {
    let keyed = abstraction_node_key(&term, identity)?;
    Ok(SemanticNode {
        key: keyed.key,
        preimage: keyed.preimage,
        recursion: None,
        content: NodeContent {
            node_tag: NodeTag::Correspondence,
            semantic_form: "abstraction_relation",
            semantic_type: None,
            declaration: None,
            owner: None,
            body: BodyTerm::Abstraction(term),
        },
        nominal: None,
    })
}
