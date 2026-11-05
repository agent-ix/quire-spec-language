// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-515 (TC-1310): the checked predicate and the temporal subject, read as
//! views over an admitted [`quire_contract_model::CheckedPackageV2`].
//!
//! Each view borrows the package it was read from, has no public
//! constructor and no method that evaluates a term. It is a function of the
//! package alone: the clause's identity triple, `over` node, profile, clock,
//! activation, captures, operators and `holds` operands are read from the
//! nodes FR-337 emits through the package's public surface, with no document,
//! digest or schema bytes in between. A caller resolves any returned node id
//! with [`quire_contract_model::CheckedPackageV2::term`].

mod graph;
mod mapping;
pub(crate) mod predicate;
mod subject;

pub use mapping::{
    mapping_support, AuthenticatedSelection, Classification, Closure, MappingRefusal, Retained,
    Support, Target, Unmatched, OUTSTANDING_PREMISES,
};
pub use predicate::{
    checked_predicate, instance_bound_atoms, CheckedPredicateView, PredicateKind,
    PredicateSelection,
};
#[cfg(test)]
pub(crate) use subject::temporal_subject_within;
pub use subject::{
    activation_relation, temporal_subject, Activation, ClauseOccurrence, FormulaNodeUse,
    FormulaOperation, OperatorDirection, OperatorUse, TemporalSubjectView,
};

/// Why a view was not read: a closed set, so a caller branches on the
/// variant. No refusal returns a partial view.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ViewRefusal {
    /// The clause node is absent, is not a `temporal`/`temporal_clause` node
    /// that is a clause root, or does not carry the node facts FR-337 states
    /// (FR-337's `clause-root-unresolved`).
    #[error("the node names no temporal clause of the package")]
    UnresolvedClause,
    /// The selected leaf is reachable from neither the clause's formula root
    /// nor its activation guard.
    #[error("the leaf is not a term of the clause")]
    LeafNotInClause,
    /// The leaf is a term of the clause and the package source map holds no
    /// entry for it.
    #[error("the leaf has no source map entry")]
    LeafHasNoSource,
    /// The clause's formula, read once per use of each shared node, exceeds
    /// the reader's `i2.work_units` default of visits.
    #[error("the clause formula expands past the work budget")]
    FormulaTooLarge,
    /// The selected leaf's semantic type is not the `boolean` node.
    #[error("the leaf is not a Boolean term")]
    LeafNotBoolean,
}
