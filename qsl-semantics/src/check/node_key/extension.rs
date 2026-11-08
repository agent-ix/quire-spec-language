// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSpec FR-469: the body of an `extension`/`extension_form` node.
//!
//! The body is `{term: "extension_form", definition, node_kind, clauses}`:
//! the extension definition's identity, the node kind of the production's
//! typed-node schema entry, and one clause per production clause in the
//! production's order, each the node of the clause's checked value. The node
//! kind is data, so every extension's forms share this one term and adding an
//! extension adds no Rust type (ADR-012 §5.1 row S3).

use std::collections::BTreeSet;

use qsl_foundation::selection::DefinitionRef;
use quire_exact::NodeKey;

use super::{NodeKeyRefusal, NodeRef};

/// An extension form node's body.
#[derive(Clone, Eq, Hash, PartialEq)]
pub struct ExtensionTerm {
    definition: DefinitionRef,
    node_kind: String,
    clauses: Vec<ExtensionClauseTerm>,
}

qsl_foundation::debug_with_struct!(ExtensionTerm, "ExtensionTerm" { definition: plain, node_kind: plain, clauses: with });

/// One clause: the clause's name and the node of its checked value.
#[derive(Clone, Eq, Hash, PartialEq)]
pub struct ExtensionClauseTerm {
    name: String,
    value: NodeRef,
}

qsl_foundation::debug_with_struct!(ExtensionClauseTerm, "ExtensionClauseTerm" { name: plain, value: with });

impl ExtensionClauseTerm {
    /// The clause `name` whose checked value is the node `value` names.
    pub(crate) fn new(name: impl Into<String>, value: impl Into<NodeRef>) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
        }
    }

    /// The clause name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The node of the clause's checked value.
    pub fn value(&self) -> &NodeRef {
        &self.value
    }
}

impl ExtensionTerm {
    /// The body of a form of `definition`'s production whose typed-node
    /// schema entry has `node_kind`, with `clauses` in the production's
    /// order.
    pub(crate) fn new(
        definition: DefinitionRef,
        node_kind: impl Into<String>,
        clauses: Vec<ExtensionClauseTerm>,
    ) -> Self {
        Self {
            definition,
            node_kind: node_kind.into(),
            clauses,
        }
    }

    /// The extension definition the form belongs to.
    pub fn definition(&self) -> &DefinitionRef {
        &self.definition
    }

    /// The node kind of the production's typed-node schema entry.
    pub fn node_kind(&self) -> &str {
        &self.node_kind
    }

    /// The clauses in the production's order.
    pub fn clauses(&self) -> &[ExtensionClauseTerm] {
        &self.clauses
    }

    /// The unique clause targets in structural order, which are the node's
    /// dependencies (QSpec FR-469 "Type and dependencies"). A pending target
    /// is a lowering fault and cannot be emitted.
    pub fn targets(&self) -> Result<BTreeSet<NodeKey>, NodeKeyRefusal> {
        self.clauses
            .iter()
            .map(|clause| {
                clause
                    .value
                    .admitted()
                    .cloned()
                    .ok_or(NodeKeyRefusal::UnresolvedPlaceholder)
            })
            .collect()
    }

    /// Call `visit` with every node reference this term holds.
    pub(crate) fn for_each_ref(&self, visit: &mut impl FnMut(&NodeRef)) {
        for clause in &self.clauses {
            visit(&clause.value);
        }
    }

    /// Remap every node reference.
    pub(crate) fn map_refs(&self, map: &mut impl FnMut(&NodeRef) -> NodeRef) -> Self {
        Self {
            definition: self.definition.clone(),
            node_kind: self.node_kind.clone(),
            clauses: self
                .clauses
                .iter()
                .map(|clause| ExtensionClauseTerm {
                    name: clause.name.clone(),
                    value: map(&clause.value),
                })
                .collect(),
        }
    }
}
