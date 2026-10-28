// SPDX-License-Identifier: AGPL-3.0-or-later
//! The in-core certificate checkers' own arithmetic (ADR-029 CB-2).
//!
//! [`zone`] holds the zone certificate checker's difference-bound matrices
//! (ADR-026 CF-3, CF-6; FR-245): separate code from the zone engine's DBM in
//! `qsl-analyze`, so a fault in one cannot hide a fault in the other.
//! [`zone_certificate`] holds FR-244's zone certificate and FR-245's check
//! of it.
//! [`closure`] holds FR-338's closure certificate and its checker (ADR-018
//! PC-3); [`scc`] holds FR-339's component certificate witnesses and the
//! partition check of them (ADR-018 PC-4); [`simulation`] holds FR-149's
//! simulation certificate of a refinement and its checker. FR-338 and FR-314's rejection vocabulary retains the failing rule
//! and product-state, query or proof-step locus independently of any checker
//! implementation.

use serde::{ser::SerializeStruct, Serialize, Serializer};

use qsl_foundation::digest::DigestRecord;

use crate::certificate::closure::{DigestWire, ProductStateRef};
use crate::certificate::simulation::FailedStep;

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
    /// A certificate's entries are unsorted, duplicated, out of range or of
    /// the wrong domain (FR-163, FR-169), found before any recomputation.
    Malformed,
    /// A claim predicate is undefined or refused at a closure state
    /// (FR-169).
    PredicateUndefined,
    /// A state that needs a rank has none (FR-169).
    RankMissing,
    /// A rank is not the distance it claims (FR-169).
    RankBroken,
    /// An edge of `H` does not go to a node earlier in the order (FR-169).
    OrderBroken,
    /// A node's path count is not the one its successors give (FR-169).
    CountBroken,
    /// A concrete initial state does not pass `check_initial` with the
    /// position the simulation certificate lists for it (FR-149).
    InitialMismatch,
    /// A position's recomputed concrete successors are not the listed
    /// edges' transitions, in canonical order (FR-149).
    SuccessorsDiffer,
    /// `check_step` does not pass a listed edge (FR-149).
    StepFails,
    /// A passing edge's post position or `Taken` differs from the listed
    /// edge's (FR-149).
    PostDiffers,
}

/// FR-338 and FR-314: where a certificate rejection occurred.
///
/// A product-state locus serializes as [`ProductStateRef`]'s object. Query
/// loci serialize with QSpec FR-331's query-part spelling. A proof-step
/// locus serializes as an object containing its part and its canonical decimal
/// `u64` index string.
#[non_exhaustive]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CertificateLocus {
    /// The product state of a closure certificate at which a rule failed.
    ProductState(ProductStateRef),
    /// The query whose encoding or certificate shape failed.
    Query {
        /// The query's proof obligation part.
        part: QueryPart,
    },
    /// The model state of a state-graph certificate at which a rule failed
    /// (FR-169): its `quire.simulation.state-key/v1` digest.
    ModelState(DigestRecord),
    /// A state-graph certificate entry whose closure index names no closure
    /// state (FR-169).
    Entry {
        /// The closure index the entry names.
        index: u64,
    },
    /// The position, and the edge out of it, of a simulation certificate at
    /// which a rule failed (FR-149).
    SimulationStep {
        /// The certificate position the rule concerns.
        position: u64,
        /// The index of the listed edge when the rule concerns one.
        edge: Option<u64>,
        /// What `check_step` returned, for `StepFails`.
        verdict: Option<Box<FailedStep>>,
    },
    /// The proof step whose derivation or conclusion failed.
    ProofStep {
        /// The query containing the proof step.
        part: QueryPart,
        /// The step's index in that query's proof.
        index: u64,
    },
    /// A malformed member of a certificate (FR-163, FR-245).
    CertificateMember(CertificateMember),
    /// A zone certificate node, by its position in `nodes` (FR-245).
    ZoneNode {
        /// The node's position.
        node: u64,
    },
    /// A zone certificate edge, by its position in `edges` (FR-245).
    ZoneEdge {
        /// The edge's position.
        edge: u64,
    },
    /// A zone certificate component, by its index (FR-245).
    ZoneComponent {
        /// The component's index.
        component: u64,
    },
}

/// A certificate member QSpec FR-331's certificate-member locus names.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CertificateMember {
    /// `identity`.
    Identity,
    /// `objective`.
    Objective,
    /// `kind`.
    Kind,
    /// `values`.
    Values,
    /// `ranking`.
    Ranking,
    /// `policy`.
    Policy,
    /// `components`.
    Components,
    /// `scale`.
    Scale,
    /// `bounds`, a zone certificate's LU bounds.
    Bounds,
    /// `nodes`.
    Nodes,
    /// `edges`.
    Edges,
    /// `reasons`.
    Reasons,
    /// `divergence`.
    Divergence,
    /// `refinement`, a zone certificate's refinement part (FR-244
    /// "Refinement form").
    Refinement,
    /// `positions`, the refinement part's positions.
    Positions,
}

impl CertificateMember {
    /// The member's FR-331 spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Identity => "identity",
            Self::Objective => "objective",
            Self::Kind => "kind",
            Self::Values => "values",
            Self::Ranking => "ranking",
            Self::Policy => "policy",
            Self::Components => "components",
            Self::Scale => "scale",
            Self::Bounds => "bounds",
            Self::Nodes => "nodes",
            Self::Edges => "edges",
            Self::Reasons => "reasons",
            Self::Divergence => "divergence",
            Self::Refinement => "refinement",
            Self::Positions => "positions",
        }
    }
}

impl Serialize for CertificateLocus {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::ProductState(state) => state.serialize(serializer),
            Self::ModelState(state) => DigestWire::of(state).serialize(serializer),
            Self::Entry { index } => {
                let mut object = serializer.serialize_struct("CertificateLocus", 1)?;
                let index = index.to_string();
                object.serialize_field("entry", &index)?;
                object.end()
            }
            Self::SimulationStep {
                position,
                edge,
                verdict,
            } => {
                let mut object = serializer.serialize_struct("CertificateLocus", 3)?;
                let position = position.to_string();
                object.serialize_field("position", &position)?;
                let edge = edge.map(|edge| edge.to_string());
                object.serialize_field("edge", &edge)?;
                object.serialize_field("verdict", &verdict.as_deref())?;
                object.end()
            }
            Self::Query { part } => part.serialize(serializer),
            Self::CertificateMember(member) => {
                let mut object = serializer.serialize_struct("CertificateLocus", 1)?;
                object.serialize_field("member", member.as_str())?;
                object.end()
            }
            Self::ZoneNode { node } => {
                let mut object = serializer.serialize_struct("CertificateLocus", 1)?;
                object.serialize_field("node", &node.to_string())?;
                object.end()
            }
            Self::ZoneEdge { edge } => {
                let mut object = serializer.serialize_struct("CertificateLocus", 1)?;
                object.serialize_field("edge", &edge.to_string())?;
                object.end()
            }
            Self::ZoneComponent { component } => {
                let mut object = serializer.serialize_struct("CertificateLocus", 1)?;
                object.serialize_field("component", &component.to_string())?;
                object.end()
            }
            Self::ProofStep { part, index } => {
                let mut object = serializer.serialize_struct("CertificateLocus", 2)?;
                object.serialize_field("part", part)?;
                let index = index.to_string();
                object.serialize_field("index", &index)?;
                object.end()
            }
        }
    }
}

mod check;
pub mod closed_form;
pub mod closure;
mod recording;
pub mod scc;
pub mod simulation;
pub mod state_graph;
pub mod zone;
pub mod zone_certificate;
