// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-031 SW-5 and SW-13: the separating-witness vocabulary the S6a
//! evaluator, the replay facade and its consumers share -- QSpec FR-207's
//! runtime value path, for the subjects and steps a state clause's domain
//! uses, and FR-268's separation-check steps.

use crate::source::provenance::OccurrenceKey;
use quire_exact::ObjectReference;

/// QSpec FR-207's selected observation identity: the authority-qualified
/// identity and revision of the snapshot a value is read under.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ObservationIdentity {
    /// The `authority` label.
    pub authority: String,
    /// The `identity` label.
    pub identity: String,
    /// The `revision_namespace` label.
    pub revision_namespace: String,
    /// The `revision` label.
    pub revision: String,
}

/// QSpec FR-207's root subject, for the subjects a state clause's domain
/// collection can have.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ValuePathSubject {
    /// A declared object identity (QSpec FR-204): a collection stored in a
    /// member of that object.
    Object(ObjectReference),
    /// A built collection: the occurrence key of the expression that builds
    /// the collection inside the evaluated claim (QSpec FR-207-AC-9).
    Built(OccurrenceKey),
}

/// QSpec FR-207's traversal steps a state clause's domain path uses.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ValuePathStep {
    /// A declared model member, by its exact spelling.
    Member(String),
    /// The element at this zero-based position of a collection.
    Index(u64),
}

/// QSpec FR-207's runtime value path: a root (the observation and the
/// subject) and an ordered sequence of traversal steps.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RuntimeValuePath {
    /// The observation the value is read under.
    pub observation: ObservationIdentity,
    /// The root subject.
    pub subject: ValuePathSubject,
    /// The traversal from the subject.
    pub steps: Vec<ValuePathStep>,
}

/// FR-268's separation-check steps (ADR-031 SW-13).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SeparationStep {
    /// Step 1: the deciding quantifier is a `forall` or `exists`
    /// occurrence on the clause's claim.
    Quantifier,
    /// Step 2: the quantifier's domain evaluates.
    Domain,
    /// Step 3: the element at the index is at the value path and equals
    /// the deciding element.
    Element,
    /// Step 4: the body, bound to the element, separates.
    Body,
}
