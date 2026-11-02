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
mod predicate;
mod subject;

pub use predicate::{checked_predicate, CheckedPredicateView, PredicateKind, PredicateSelection};
pub use subject::{
    temporal_subject, Activation, ClauseOccurrence, OperatorUse, TemporalSubjectView,
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
    /// The selected leaf is no term the clause reads.
    #[error("the leaf is not a term of the clause")]
    LeafNotInClause,
    /// The selected leaf's semantic type is not the `boolean` node.
    #[error("the leaf is not a Boolean term")]
    LeafNotBoolean,
}
