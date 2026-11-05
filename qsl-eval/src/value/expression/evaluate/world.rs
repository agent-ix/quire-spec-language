// SPDX-License-Identifier: AGPL-3.0-or-later
//! The package a [`super::Machine`] evaluates against (FR-533 "What the bare
//! view reads from the package").
//!
//! A checked package gives its scope and function table. A package read from
//! its bytes alone gives a [`qsl_package::BareWorld`]: the record
//! declarations the identity rows and nodes give, and nothing from a source
//! scope. The machine reads declarations only through this type.

use qsl_package::BareWorld;
use qsl_semantics::check::{CheckedGraph, DispatchOperation, Scope};
use quire_checked::node::RecordField;
use quire_exact::{EffectiveId, NodeKey};
use quire_semantic_value::declaration::{CompositeShape, TypeEnvironment};
use quire_semantic_value::enumeration::EnumMemberIndex;

/// The declarations one body evaluates against.
#[derive(Clone, Copy)]
pub(crate) enum World<'a> {
    /// A checked package: its scope and function table.
    Checked {
        scope: &'a Scope,
        graph: &'a CheckedGraph,
    },
    /// A package read from its bytes alone.
    Bytes(&'a BareWorld),
}

impl<'a> World<'a> {
    /// The record, tuple and object-type declarations.
    pub(crate) fn types(self) -> &'a TypeEnvironment {
        match self {
            Self::Checked { scope, .. } => scope.types(),
            Self::Bytes(world) => world.types(),
        }
    }

    /// The position of `field` among the fields of the record `declaration`.
    pub(crate) fn record_slot(self, declaration: NodeKey, field: &RecordField) -> Option<usize> {
        match self {
            Self::Checked { scope, .. } => scope.record_slot(declaration, field),
            Self::Bytes(world) => {
                let CompositeShape::Record(fields) = world.types().composite(declaration)?.shape()
                else {
                    return None;
                };
                fields
                    .iter()
                    .position(|candidate| candidate.name() == field.name())
            }
        }
    }

    /// Whether the package selects the IEEE profile.
    pub(crate) fn has_ieee_profile(self) -> bool {
        match self {
            Self::Checked { scope, .. } => scope.ieee_profile().is_some(),
            Self::Bytes(world) => world.has_ieee_profile(),
        }
    }

    /// The checked enumeration members.
    pub(crate) fn enum_member_index(self) -> &'a EnumMemberIndex {
        match self {
            Self::Checked { scope, .. } => scope.enum_member_index(),
            Self::Bytes(world) => world.enum_members(),
        }
    }

    /// The function table of a checked package; a bytes-only package has
    /// none, its functions being nodes of the projection.
    pub(crate) fn graph(self) -> Option<&'a CheckedGraph> {
        match self {
            Self::Checked { graph, .. } => Some(graph),
            Self::Bytes(_) => None,
        }
    }

    /// The package's FR-151 dispatch operations; none for a bytes-only
    /// package, whose projection holds no dispatch.
    pub(crate) fn dispatch_operations(self) -> &'a [DispatchOperation] {
        match self {
            Self::Checked { scope, .. } => scope.dispatch_operations(),
            Self::Bytes(_) => &[],
        }
    }

    /// The declaration node of an object type.
    pub(crate) fn object_type_node(self, object_type: EffectiveId) -> Option<String> {
        match self {
            Self::Checked { graph, .. } => graph
                .object_type_key(object_type)
                .map(|key| key.node.clone()),
            Self::Bytes(_) => None,
        }
    }

    /// The checked graph of the import exporting `callee`.
    pub(crate) fn imported(self, callee: &NodeKey) -> Option<&'a CheckedGraph> {
        match self {
            Self::Checked { graph, .. } => graph.scope().imported_graph(callee),
            Self::Bytes(_) => None,
        }
    }
}
