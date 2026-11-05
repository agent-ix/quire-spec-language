// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-515 "The checked predicate": the view of one Boolean leaf of a
//! temporal clause of an admitted checked package.

use quire_contract_model::{
    CheckedNodeId, CheckedNodeKind, CheckedPackageV2, CheckedSourceMapEntry, CheckedSourceRegion,
    ValueForm,
};

use super::graph::Graph;
use super::subject::{subject_in, Activation, TemporalSubjectView, FORMULA_VISITS};
use super::ViewRefusal;

/// Which clause leaf a [`PredicateSelection`] names.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PredicateKind {
    /// The Boolean operand of a `holds` atom.
    HoldsLeaf,
    /// The guard of an `on each ... when (g)` activation.
    ActivationGuard,
    /// Another complete Boolean leaf of the clause's formula.
    ClauseLeaf,
}

/// A clause and one term of it to read as a predicate.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PredicateSelection {
    /// The `temporal`/`temporal_clause` node.
    pub clause: CheckedNodeId,
    /// The Boolean leaf.
    pub leaf: CheckedNodeId,
}

/// One Boolean leaf of a temporal clause of one admitted checked package.
///
/// The view borrows the package, is built only by [`checked_predicate`] and
/// evaluates nothing. It carries no digest, document bytes or identity label.
///
/// A view cannot be built outside this module:
///
/// ```compile_fail,E0451
/// use qsl_package::checked_views::CheckedPredicateView;
/// fn construct(package: &quire_contract_model::CheckedPackageV2) -> CheckedPredicateView<'_> {
///     CheckedPredicateView { package }
/// }
/// ```
///
/// It has none of `digest`, `bytes`, `identity` or `package_digest`, and no
/// method that takes a closure. Each call below fails for the missing method
/// alone; the control call shows the same shape compiles with a real accessor.
///
/// ```
/// fn call(view: &qsl_package::checked_views::CheckedPredicateView<'_>) {
///     let _ = (view.declaration(), view.leaf(), view.kind(), view.binders());
/// }
/// ```
///
/// ```compile_fail,E0599
/// fn call(view: &qsl_package::checked_views::CheckedPredicateView<'_>) {
///     let _ = view.digest();
/// }
/// ```
///
/// ```compile_fail,E0599
/// fn call(view: &qsl_package::checked_views::CheckedPredicateView<'_>) {
///     let _ = view.bytes();
/// }
/// ```
///
/// ```compile_fail,E0599
/// fn call(view: &qsl_package::checked_views::CheckedPredicateView<'_>) {
///     let _ = view.identity();
/// }
/// ```
///
/// ```compile_fail,E0599
/// fn call(view: &qsl_package::checked_views::CheckedPredicateView<'_>) {
///     let _ = view.package_digest();
/// }
/// ```
///
/// ```compile_fail,E0599
/// fn call(view: &qsl_package::checked_views::CheckedPredicateView<'_>) {
///     let _ = view.evaluate(|_| true);
/// }
/// ```
#[derive(Clone, Debug)]
pub struct CheckedPredicateView<'p> {
    subject: TemporalSubjectView<'p>,
    leaf: &'p CheckedNodeId,
    kind: PredicateKind,
    parent_operation: Option<&'p str>,
    binders: Vec<CheckedNodeId>,
    source_span: &'p CheckedSourceMapEntry,
}

impl<'p> CheckedPredicateView<'p> {
    /// The clause node.
    pub fn declaration(&self) -> &'p CheckedNodeId {
        self.subject.declaration()
    }

    /// The Boolean leaf.
    pub fn leaf(&self) -> &'p CheckedNodeId {
        self.leaf
    }

    /// Which clause leaf this is.
    pub fn kind(&self) -> PredicateKind {
        self.kind
    }

    /// The operation identity of the application that first reads the leaf
    /// in the clause.
    pub fn parent_operation(&self) -> Option<&'p str> {
        self.parent_operation
    }

    /// The `over`, trigger and capture binder nodes the leaf term reads, in
    /// first-reference order.
    pub fn binders(&self) -> &[CheckedNodeId] {
        &self.binders
    }

    /// The package source map entry of the leaf, the one inside the clause's
    /// span when the leaf is shared with another clause.
    pub fn source_span(&self) -> &'p CheckedSourceMapEntry {
        self.source_span
    }

    /// The clause's selected temporal profile.
    pub fn profile(&self) -> &'p quire_contract_model::CheckedSelection {
        self.subject.profile()
    }

    /// Whether `package` is the package this view was read from.
    pub fn belongs_to(&self, package: &CheckedPackageV2) -> bool {
        self.subject.belongs_to(package)
    }
}

/// Read the Boolean leaf `selection.leaf` of the clause `selection.clause`.
///
/// Refuses [`ViewRefusal::UnresolvedClause`] for a clause that
/// [`super::temporal_subject`] refuses, [`ViewRefusal::LeafNotInClause`] for a leaf
/// no term of the clause reads, [`ViewRefusal::LeafHasNoSource`] for a clause
/// term with no source map entry and [`ViewRefusal::LeafNotBoolean`] for a leaf
/// the package does not type `boolean`; no refusal leaves a partial view.
pub fn checked_predicate<'p>(
    package: &'p CheckedPackageV2,
    selection: &PredicateSelection,
) -> Result<CheckedPredicateView<'p>, ViewRefusal> {
    read_predicate(package, package.source_map(), selection)
}

/// [`checked_predicate`] over an explicit source map. An admitted package
/// holds an entry for every occurrence, so the map is a parameter in order
/// that the [`ViewRefusal::LeafHasNoSource`] path can be read with a map that
/// lacks one.
pub(crate) fn read_predicate<'p>(
    package: &'p CheckedPackageV2,
    sources: &'p [CheckedSourceMapEntry],
    selection: &PredicateSelection,
) -> Result<CheckedPredicateView<'p>, ViewRefusal> {
    let graph = Graph::new(package);
    let subject = subject_in(&graph, &selection.clause, FORMULA_VISITS)?;
    let leaf_at = graph
        .position(&selection.leaf)
        .ok_or(ViewRefusal::LeafNotInClause)?;
    let leaf = graph
        .node(leaf_at)
        .map(|node| &node.node_id)
        .ok_or(ViewRefusal::LeafNotInClause)?;
    let clause_at = graph
        .position(subject.declaration())
        .ok_or(ViewRefusal::UnresolvedClause)?;

    let guard = match subject.activation() {
        Activation::Each { guard, .. } => guard.as_ref(),
        Activation::Origin => None,
    };
    let formula_root = graph
        .arguments(clause_at)
        .and_then(<[_]>::last)
        .and_then(|term| match term {
            quire_contract_model::CheckedTerm::Reference { target } => graph.position(target),
            _ => None,
        })
        .ok_or(ViewRefusal::UnresolvedClause)?;
    let roots: Vec<usize> = guard
        .and_then(|guard| graph.position(guard))
        .into_iter()
        .chain([formula_root])
        .collect();
    let reads = graph.reachable_terms(&roots);

    let kind = if guard == Some(&selection.leaf) {
        PredicateKind::ActivationGuard
    } else if subject.predicate_leaves().contains(&selection.leaf) {
        PredicateKind::HoldsLeaf
    } else if reads.contains(&leaf_at) {
        PredicateKind::ClauseLeaf
    } else {
        return Err(ViewRefusal::LeafNotInClause);
    };
    if !graph.is_boolean(leaf_at) {
        return Err(ViewRefusal::LeafNotBoolean);
    }

    let parent_operation = std::iter::once(clause_at)
        .chain(reads.iter().copied())
        .find(|&at| graph.references(at).contains(&leaf_at))
        .and_then(|at| graph.operation(at));
    let binders = graph
        .reachable_terms(&[leaf_at])
        .into_iter()
        .filter(|&at| graph.kind(at) == Some(CheckedNodeKind::Value(ValueForm::Parameter)))
        .filter_map(|at| graph.node(at).map(|node| node.node_id.clone()))
        .collect();
    let formula_id = graph
        .node(formula_root)
        .map(|node| &node.node_id)
        .ok_or(ViewRefusal::UnresolvedClause)?;
    let source_span = leaf_source(sources, leaf, formula_id).ok_or(ViewRefusal::LeafHasNoSource)?;
    Ok(CheckedPredicateView {
        subject,
        leaf,
        kind,
        parent_operation,
        binders,
        source_span,
    })
}

/// The source map entry of `leaf`: the first inside the source of the
/// clause's formula root, else the first. A leaf shared with another clause
/// has an entry per reading, and this picks the one in this clause's formula.
fn leaf_source<'p>(
    sources: &'p [CheckedSourceMapEntry],
    leaf: &CheckedNodeId,
    formula_root: &CheckedNodeId,
) -> Option<&'p CheckedSourceMapEntry> {
    let formula: Vec<&CheckedSourceRegion> = sources
        .iter()
        .filter(|entry| entry.node_id == *formula_root)
        .flat_map(|entry| entry.regions.iter())
        .collect();
    let inside = |region: &CheckedSourceRegion| {
        formula.iter().any(|outer| {
            outer.source == region.source && outer.start <= region.start && region.end <= outer.end
        })
    };
    let mut entries = sources.iter().filter(|entry| entry.node_id == *leaf);
    let first = entries.clone().next();
    entries
        .find(|entry| entry.regions.iter().all(inside))
        .or(first)
}

/// Whether the operand of each `holds` atom of `subject`'s formula reads an
/// instance binder, one entry per atom in formula pre-order (so entry `k`
/// belongs to the `k`-th `holds` node and to `predicate_leaves()[k]`).
///
/// The instance binders of a clause are the binders of its captures, each the
/// `parameter` node named like the capture (the captures argument holds the
/// initializers, not the binders); the `over` binder is not one, and a `holds`
/// condition cannot read the trigger binder. The activation guard is no
/// `holds` atom and has no entry. An atom is
/// instance-bound when [`CheckedPredicateView::binders`] of its operand
/// includes a capture binder.
///
/// Refuses [`ViewRefusal::UnresolvedClause`] when the clause's
/// `predicate_leaves()` and the `holds` nodes of its `formula_graph()` differ
/// in count, because the mapping of an atom to its operand is by formula
/// order.
pub fn instance_bound_atoms(subject: &TemporalSubjectView<'_>) -> Result<Vec<bool>, ViewRefusal> {
    let holds = subject
        .formula_graph()
        .iter()
        .filter(|node| node.kind() == Some(super::subject::FormulaOperation::Holds))
        .count();
    if holds != subject.predicate_leaves().len() {
        return Err(ViewRefusal::UnresolvedClause);
    }
    let graph = Graph::new(subject.package());
    let mut leaves = subject.predicate_leaves().iter();
    subject
        .formula_graph()
        .iter()
        .filter(|node| node.kind().is_some_and(|operation| operation.is_atom()))
        .map(|node| match node.kind() {
            Some(super::subject::FormulaOperation::Holds) => {
                let leaf = leaves.next().ok_or(ViewRefusal::UnresolvedClause)?;
                let view = read_predicate(
                    subject.package(),
                    subject.package().source_map(),
                    &PredicateSelection {
                        clause: subject.declaration().clone(),
                        leaf: leaf.clone(),
                    },
                )?;
                Ok(view.binders().iter().any(|binder| {
                    subject
                        .package()
                        .term(binder)
                        .and_then(super::subject::parameter_name)
                        .is_some_and(|name| {
                            subject
                                .capture_names()
                                .iter()
                                .any(|capture| capture == name)
                        })
                }))
            }
            Some(super::subject::FormulaOperation::Aggregate) => {
                let aggregate = node.aggregate.ok_or(ViewRefusal::UnresolvedClause)?;
                let roots: Vec<_> = aggregate
                    .reference_targets()
                    .into_iter()
                    .filter_map(|target| graph.position(target))
                    .collect();
                let bound = graph.reachable_terms(&roots).into_iter().any(|at| {
                    graph.kind(at) == Some(CheckedNodeKind::Value(ValueForm::Parameter))
                        && graph
                            .term(at)
                            .and_then(super::subject::parameter_name)
                            .is_some_and(|name| {
                                subject
                                    .capture_names()
                                    .iter()
                                    .any(|capture| capture == name)
                            })
                });
                Ok(bound)
            }
            _ => Err(ViewRefusal::UnresolvedClause),
        })
        .collect()
}
