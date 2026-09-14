// SPDX-License-Identifier: AGPL-3.0-only
//! FR-051 checked native temporal-subject handoff.

use super::{checked_handoff as handoff, v2, wire as w};
use crate::ByteDigest;

pub use handoff::{Error, ErrorCode, Limits, Report, Usage};

/// Canonical immutable JSON Schema bytes for `quire.checked-temporal-subject/v1`.
pub const SCHEMA_BYTES: &[u8] =
    include_bytes!("../../schemas/checked-temporal-subject-v1.schema.json");
/// SHA-256 digest of [`SCHEMA_BYTES`].
pub const SCHEMA_SHA256: &str = "e72fce683648b18dd7e0f64b438f7dd5d63c48c015b4bcbce9a9f9aef3c096e9";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
/// Exact admitted temporal declaration selected for derivation.
pub struct DeclarationSelection {
    declaration: u32,
}

impl DeclarationSelection {
    /// Selects the admitted temporal declaration at `declaration`.
    pub const fn new(declaration: u32) -> Self {
        Self { declaration }
    }

    /// Returns the selected declaration-table index.
    pub const fn declaration(self) -> u32 {
        self.declaration
    }
}

#[derive(Debug)]
/// Canonical derived temporal-subject bytes and identities.
pub struct Document(handoff::Document);

impl Document {
    /// Returns the complete canonical JSON document bytes.
    pub fn bytes(&self) -> &[u8] {
        self.0.bytes()
    }

    /// Returns the SHA-256 digest of the complete document bytes.
    pub const fn digest(&self) -> ByteDigest {
        self.0.digest()
    }

    /// Returns the domain-separated semantic identity carried by the document.
    pub fn identity(&self) -> &str {
        self.0.identity()
    }
}

#[derive(Debug)]
/// Constructor-private authority produced only by strict temporal-subject admission.
///
/// ```compile_fail
/// use quire_spec_language::protocol_artifact::temporal_subject::ValidatedTemporalSubject;
/// let _ = ValidatedTemporalSubject;
/// ```
pub struct ValidatedTemporalSubject(Document);

impl ValidatedTemporalSubject {
    /// Returns the exact canonical document that was admitted.
    pub fn document(&self) -> &Document {
        &self.0
    }

    /// Returns the selected admitted package byte digest.
    pub fn package_digest(&self) -> ByteDigest {
        self.0 .0.package_digest()
    }

    /// Returns the selected admitted package artifact reference.
    pub fn package_artifact(&self) -> &w::ArtifactRef {
        self.0 .0.package_artifact()
    }

    /// Returns the selected declaration-table index.
    pub fn declaration(&self) -> u32 {
        self.0 .0.declaration()
    }

    /// Returns the selected temporal root handle.
    pub fn root(&self) -> &w::Handle {
        self.0 .0.root()
    }

    /// Returns the exact authored source and source span.
    pub fn source(&self) -> (&w::Source, &w::Span) {
        self.0 .0.source()
    }

    /// Returns the requirement, clause identity, and execution contract.
    pub fn clause(&self) -> (&w::Requirement, &str, &w::Execution) {
        self.0 .0.clause()
    }

    /// Iterates reachable value expressions in index order.
    pub fn values(&self) -> impl ExactSizeIterator<Item = (u32, &w::Value)> {
        self.0 .0.values()
    }

    /// Iterates reachable temporal expressions in index order.
    pub fn temporal_nodes(&self) -> impl ExactSizeIterator<Item = (u32, &w::Temporal)> {
        self.0 .0.temporal()
    }

    /// Iterates sorted owner identity bindings as `(kind, identity)` pairs.
    pub fn bindings(&self) -> impl ExactSizeIterator<Item = (&str, &str)> {
        self.0 .0.bindings()
    }

    /// Returns all retained Boolean type-table indices.
    pub fn type_indices(&self) -> &[u32] {
        self.0 .0.type_indices()
    }

    /// Iterates the exact evaluation and definedness profile identities.
    pub fn profile_identities(&self) -> impl Iterator<Item = &str> {
        self.0 .0.profile_identities()
    }

    /// Returns the declaration activation policy.
    pub fn activation(&self) -> Option<&w::Activation> {
        self.0 .0.temporal_subject().map(|subject| subject.0)
    }

    /// Returns the selected clock index and immutable clock configuration.
    pub fn clock(&self) -> Option<(u32, &v2::wire::ClockConfiguration)> {
        self.0
             .0
            .temporal_subject()
            .map(|subject| (subject.1, subject.2))
    }

    /// Returns the declaration capture handles.
    pub fn captures(&self) -> Option<&[w::Handle]> {
        self.0 .0.temporal_subject().map(|subject| subject.3)
    }

    /// Returns `execution-origin` or `history-cutoff` for the admitted activation.
    pub fn history_boundary(&self) -> Option<&str> {
        self.0 .0.temporal_subject().map(|subject| subject.4)
    }

    /// Returns the sorted temporal operator inventory.
    pub fn operators(&self) -> Option<&[String]> {
        self.0 .0.temporal_subject().map(|subject| subject.5)
    }

    /// Returns the exact retained interval requirements.
    pub fn required_history(&self) -> Option<&[w::Interval]> {
        self.0 .0.temporal_subject().map(|subject| subject.6)
    }

    /// Returns the sorted Boolean value leaves reachable from the temporal root.
    pub fn predicate_leaves(&self) -> Option<&[w::Handle]> {
        self.0 .0.temporal_subject().map(|subject| subject.7)
    }
}

/// Derives the unique canonical document from admitted static owner authority.
pub fn derive(
    package: &v2::AdmittedPackage,
    selection: DeclarationSelection,
    limits: Limits,
) -> Report<Document> {
    handoff::derive(
        package,
        handoff::Selection::Temporal {
            declaration: selection.declaration,
        },
        limits,
    )
    .map(Document)
}

/// Strictly admits `bytes` by independently deriving and comparing the canonical document.
pub fn read(
    bytes: &[u8],
    package: &v2::AdmittedPackage,
    selection: DeclarationSelection,
    limits: Limits,
) -> Report<ValidatedTemporalSubject> {
    handoff::read(
        bytes,
        package,
        handoff::Selection::Temporal {
            declaration: selection.declaration,
        },
        limits,
    )
    .map(|document| ValidatedTemporalSubject(Document(document)))
}
