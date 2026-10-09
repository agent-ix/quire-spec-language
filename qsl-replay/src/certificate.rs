// SPDX-License-Identifier: AGPL-3.0-or-later
//! The in-core certificate checkers' own arithmetic (ADR-029 CB-2).
//!
//! [`zone`] holds the zone certificate checker's difference-bound matrices
//! (ADR-026 CF-3, CF-6; FR-245): separate code from the zone engine's DBM in
//! `qsl-analyze`, so a fault in one cannot hide a fault in the other.
//! FR-338 and FR-314's rejection vocabulary retains the failing rule and
//! query or proof-step locus independently of any checker implementation.

use serde::{Serialize, Serializer};

/// FR-314's part of an SMT proof obligation.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum QueryPart {
    /// The bounded-complete unrolling query.
    Unrolling,
    /// The induction base query.
    Base,
    /// The induction step query.
    Step,
}

/// FR-338 and FR-314's finite certificate rejection rule vocabulary.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum CertificateRule {
    /// An initial product state is missing.
    InitialMissing,
    /// A reached successor is missing.
    SuccessorMissing,
    /// A reached state is bad for the checked item.
    BadState,
    /// The components do not partition the reached closure.
    NotPartition,
    /// An edge goes to an earlier component.
    BackwardEdge,
    /// A component witness does not hold.
    WitnessFails,
    /// A carried query differs from the core's expected query.
    QueryMismatch,
    /// The certificate shape differs from the proof basis.
    ShapeMismatch,
    /// A checked proof step does not follow from its premises.
    ProofStepInvalid,
    /// The proof does not conclude a refutation.
    NotRefutation,
}

/// FR-338 and FR-314: where a certificate rejection occurred.
///
/// Query loci serialize with QSpec FR-331's query-part spelling. A proof-step
/// locus retains its part and index, but serialization refuses it because
/// FR-331 defines no lossless string spelling for that payload.
#[non_exhaustive]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CertificateLocus {
    /// The query whose encoding or certificate shape failed.
    Query {
        /// The query's proof obligation part.
        part: QueryPart,
    },
    /// The proof step whose derivation or conclusion failed.
    ProofStep {
        /// The query containing the proof step.
        part: QueryPart,
        /// The step's index in that query's proof.
        index: u64,
    },
}

impl Serialize for CertificateLocus {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Query { part } => part.serialize(serializer),
            Self::ProofStep { .. } => Err(serde::ser::Error::custom(
                "QSpec FR-331 defines no wire spelling for a proof-step certificate locus",
            )),
        }
    }
}

pub mod zone;
