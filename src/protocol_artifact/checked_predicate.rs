// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-051 checked native Boolean predicate handoff.

use super::{checked_handoff as handoff, v2, wire as w};
use crate::ByteDigest;

pub use handoff::{Error, ErrorCode, Limits, Report, Usage};

/// Canonical immutable JSON Schema bytes for `quire.checked-predicate/v1`.
pub const SCHEMA_BYTES: &[u8] = include_bytes!("../../schemas/checked-predicate-v1.schema.json");
/// SHA-256 digest of [`SCHEMA_BYTES`].
pub const SCHEMA_SHA256: &str = "459b72a948ddc17be824412b04929fb5795ea033b0bf2aa3f60cf378bc42a531";

#[derive(Clone, Debug, Eq, PartialEq)]
/// Exact admitted declaration and Boolean leaf selected for derivation.
pub struct ClauseSelection {
    declaration: u32,
    expression: w::Handle,
}

impl ClauseSelection {
    /// Selects `expression` within the admitted declaration at `declaration`.
    pub const fn new(declaration: u32, expression: w::Handle) -> Self {
        Self {
            declaration,
            expression,
        }
    }

    /// Returns the selected declaration-table index.
    pub const fn declaration(&self) -> u32 {
        self.declaration
    }

    /// Returns the selected admitted expression handle.
    pub const fn expression(&self) -> &w::Handle {
        &self.expression
    }
}

#[derive(Debug)]
/// Canonical derived checked-predicate bytes and identities.
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
/// Constructor-private authority produced only by strict checked-predicate admission.
///
/// ```compile_fail
/// use quire_spec_language::protocol_artifact::checked_predicate::ValidatedCheckedPredicate;
/// let _ = ValidatedCheckedPredicate;
/// ```
pub struct ValidatedCheckedPredicate(Document);

impl ValidatedCheckedPredicate {
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

    /// Returns the selected Boolean expression leaf.
    pub fn leaf(&self) -> &w::Handle {
        self.0 .0.root()
    }

    /// Returns the owner declaration kind containing the selected leaf.
    pub fn parent_kind(&self) -> Option<&str> {
        self.0 .0.predicate_subject().map(|subject| subject.0)
    }

    /// Returns the declaration parameters that bind the leaf.
    pub fn parameters(&self) -> Option<&[w::Handle]> {
        self.0 .0.predicate_subject().map(|subject| subject.2)
    }

    /// Returns the exact authored source and source span.
    pub fn source(&self) -> (&w::Source, &w::Span) {
        self.0 .0.source()
    }

    /// Returns the requirement, clause identity, and execution contract.
    pub fn clause(&self) -> (&w::Requirement, &str, &w::Execution) {
        self.0 .0.clause()
    }

    /// Iterates the reachable value-expression graph in index order.
    pub fn values(&self) -> impl ExactSizeIterator<Item = (u32, &w::Value)> {
        self.0 .0.values()
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
}

/// Derives the unique canonical document from admitted static owner authority.
pub fn derive(
    package: &v2::AdmittedPackage,
    selection: ClauseSelection,
    limits: Limits,
) -> Report<Document> {
    handoff::derive(
        package,
        handoff::Selection::Predicate {
            declaration: selection.declaration,
            expression: selection.expression,
        },
        limits,
    )
    .map(Document)
}

/// Strictly admits `bytes` by independently deriving and comparing the canonical document.
pub fn read(
    bytes: &[u8],
    package: &v2::AdmittedPackage,
    selection: ClauseSelection,
    limits: Limits,
) -> Report<ValidatedCheckedPredicate> {
    handoff::read(
        bytes,
        package,
        handoff::Selection::Predicate {
            declaration: selection.declaration,
            expression: selection.expression,
        },
        limits,
    )
    .map(|document| ValidatedCheckedPredicate(Document(document)))
}
