// SPDX-License-Identifier: AGPL-3.0-or-later
//! The position index and the closed term reads both views share.

use std::collections::{BTreeMap, BTreeSet};

use quire_contract_model::{
    CheckedNodeId, CheckedNodeKind, CheckedOperationMember, CheckedPackageV2,
    CheckedSemanticNodeV2, CheckedTerm, ScalarTypeForm, TemporalForm,
};

/// An admitted package with its nodes indexed by id.
pub(super) struct Graph<'p> {
    package: &'p CheckedPackageV2,
    positions: BTreeMap<&'p CheckedNodeId, usize>,
}

impl<'p> Graph<'p> {
    pub(super) fn new(package: &'p CheckedPackageV2) -> Self {
        let positions = package
            .resolved_graph()
            .nodes
            .iter()
            .enumerate()
            .map(|(at, node)| (&node.node_id, at))
            .collect();
        Self { package, positions }
    }

    pub(super) fn package(&self) -> &'p CheckedPackageV2 {
        self.package
    }

    pub(super) fn position(&self, id: &CheckedNodeId) -> Option<usize> {
        self.positions.get(id).copied()
    }

    pub(super) fn node(&self, at: usize) -> Option<&'p CheckedSemanticNodeV2> {
        self.package.resolved_graph().nodes.get(at)
    }

    pub(super) fn kind(&self, at: usize) -> Option<CheckedNodeKind> {
        self.package.node_kinds().get(at).copied()
    }

    /// The node with `id` and its kind.
    pub(super) fn resolve(
        &self,
        id: &CheckedNodeId,
    ) -> Option<(&'p CheckedSemanticNodeV2, CheckedNodeKind)> {
        let at = self.position(id)?;
        Some((self.node(at)?, self.kind(at)?))
    }

    pub(super) fn term(&self, at: usize) -> Option<&'p CheckedTerm> {
        self.package.term(&self.node(at)?.node_id)
    }

    /// Whether the node `at` is typed by the package's `boolean` scalar type.
    pub(super) fn is_boolean(&self, at: usize) -> bool {
        self.node(at)
            .and_then(|node| self.resolve(&node.semantic_type))
            .is_some_and(|(_, kind)| kind == CheckedNodeKind::ScalarType(ScalarTypeForm::Boolean))
    }

    /// The nodes the body of node `at` references, in term order.
    pub(super) fn references(&self, at: usize) -> Vec<usize> {
        self.term(at)
            .map(|term| {
                term.reference_targets()
                    .into_iter()
                    .filter_map(|target| self.position(target))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// The formula, expression and value nodes reachable from `roots` through
    /// body references, in depth-first pre-order, each node once. Formula and
    /// expression nodes are expanded; a value node ends its branch, and every
    /// other family is a boundary the walk records nothing past.
    pub(super) fn reachable_terms(&self, roots: &[usize]) -> Vec<usize> {
        let mut seen = BTreeSet::new();
        let mut order = Vec::new();
        let mut pending: Vec<usize> = roots.iter().rev().copied().collect();
        while let Some(at) = pending.pop() {
            let Some(kind) = self.kind(at) else { continue };
            let expands = matches!(
                kind,
                CheckedNodeKind::Expression(_) | CheckedNodeKind::Temporal(TemporalForm::Formula)
            );
            if !(expands || matches!(kind, CheckedNodeKind::Value(_))) || !seen.insert(at) {
                continue;
            }
            order.push(at);
            if expands {
                pending.extend(self.references(at).into_iter().rev());
            }
        }
        order
    }

    /// The operation identity of the application at the body root of node `at`.
    pub(super) fn operation(&self, at: usize) -> Option<&'p str> {
        match self.term(at)? {
            CheckedTerm::Application { operation, .. } => Some(operation.read().ok()?.identity),
            _ => None,
        }
    }

    /// The typed member of the application at the body root of node `at`.
    pub(super) fn member(&self, at: usize) -> Option<&'p CheckedOperationMember> {
        match self.term(at)? {
            CheckedTerm::Application { operation, .. } => operation.read().ok()?.member,
            _ => None,
        }
    }

    /// The arguments of the application at the body root of node `at`.
    pub(super) fn arguments(&self, at: usize) -> Option<&'p [CheckedTerm]> {
        match self.term(at)? {
            CheckedTerm::Application { arguments, .. } => Some(arguments),
            _ => None,
        }
    }
}
