// SPDX-License-Identifier: AGPL-3.0-or-later
//! The checked nodes the S6a [`super::Machine`] walks (FR-390 "One admitted
//! checked-term view"): one machine over any source of checked nodes.

use qsl_semantics::check::{CheckedGraph, CheckedNode, NodeId};
use quire_checked::node::NodeKind;
use quire_exact::ValueType;
use quire_semantic_value::location::Location;

/// A handle to one checked node, from which its operands and the bodies of
/// the functions it calls are reached.
pub(crate) trait NodeSource<'a>: Copy + 'a {
    /// What a [`NodeKind`] of this source names an operand by.
    type Child: Copy + 'a;

    /// What the node computes.
    fn kind(self) -> &'a NodeKind<Self::Child>;

    /// The operand `child` names, in this node's source.
    fn at(self, child: Self::Child) -> Self;

    /// The node's checked static type.
    fn value_type(self) -> &'a ValueType;

    /// The node's source location.
    fn location(self) -> &'a Location;

    /// Whether the node is a numeric-equality part of a float ordering's
    /// composition (FR-397).
    fn is_ordering_part(self) -> bool;

    /// Direct operands in evaluation order.
    fn children(self) -> Vec<Self>;

    /// Function `function` of `graph`, the package a call at this node
    /// evaluates against: a `NodeKind::Call` index, an imported library's
    /// function, or a dispatch candidate's body or precondition.
    fn callee(self, graph: &'a CheckedGraph, function: usize) -> Option<Callee<'a, Self>>;
}

/// A called function's body, as a node of the caller's source.
pub(crate) struct Callee<'a, N> {
    /// The declared name.
    pub(crate) name: &'a str,
    /// The body's root.
    pub(crate) body: N,
    /// The evaluation slot count.
    pub(crate) slots: usize,
}

impl<'a> NodeSource<'a> for CheckedNode<'a> {
    type Child = NodeId;

    fn kind(self) -> &'a NodeKind<NodeId> {
        CheckedNode::kind(self)
    }

    fn at(self, child: NodeId) -> Self {
        CheckedNode::at(self, child)
    }

    fn value_type(self) -> &'a ValueType {
        CheckedNode::value_type(self)
    }

    fn location(self) -> &'a Location {
        CheckedNode::location(self)
    }

    fn is_ordering_part(self) -> bool {
        CheckedNode::is_ordering_part(self)
    }

    fn children(self) -> Vec<Self> {
        CheckedNode::children(self)
    }

    fn callee(self, graph: &'a CheckedGraph, function: usize) -> Option<Callee<'a, Self>> {
        graph.function_state(function).map(|state| Callee {
            name: state.name,
            body: state.body,
            slots: state.slots,
        })
    }
}
