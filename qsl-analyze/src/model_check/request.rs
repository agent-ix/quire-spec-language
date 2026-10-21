// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-336-AC-4 (ADR-013 O-09): the EN-1 request path's writer of a model
//! subject's temporal items.
//!
//! [`author_clause`] builds an item of a [`ModelRequest`] from a checked
//! clause and the [`ModelSubject`] EN-1 would check it over. The request is
//! generic over the subject it identifies, and a [`ModelSubject`] is a large
//! live input (a checked package, document bytes, limits) with no equality of
//! its own, so the request holds the subject's [`SubjectIdentity`]: the part
//! of the subject a verdict is about. A verdict over one universe joins only
//! its own request index, never another universe's.

use std::collections::{BTreeMap, BTreeSet, HashSet};

use qsl_eval::model_subject::ModelSubject;
use qsl_foundation::bound::ProofBound;
use qsl_foundation::digest::WireNodeId;
use qsl_foundation::source::provenance::OccurrenceKey;
use qsl_foundation::RequestIndex;
use qsl_route::deadlock::ModelTemporalItem;
use qsl_route::model_request::ModelRequest;
use qsl_semantics::check::TemporalClauseNode;
use qsl_semantics::model::key::DeclarationKey;

/// An initial state's document, as the identity reads it: the four source
/// labels and the digest.
type InitialState = (String, String, String, String, [u8; 32]);

/// The identity projection of a [`ModelSubject`] (ADR-013 O-09, FR-336): what
/// a verdict over the subject holds for.
///
/// It is the initial states, each population's universe, the digests of the
/// domain packages the subject was admitted against, and the proof bounds.
/// Order and repetition of any of them are immaterial. The checked package is
/// named by the item's occurrence (the other half of the obligation
/// identity), and the expansion limits, meter and observation limits decide
/// whether a run finishes, never what a finished run holds, so they are no
/// part of it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SubjectIdentity {
    initial: BTreeSet<InitialState>,
    universes: BTreeMap<DeclarationKey, BTreeSet<String>>,
    packages: BTreeSet<[u8; 32]>,
    proof_bounds: HashSet<ProofBound>,
}

impl SubjectIdentity {
    /// The identity of `subject`.
    pub fn of(subject: &ModelSubject<'_>) -> Self {
        let simulation = &subject.simulation;
        let mut universes: BTreeMap<DeclarationKey, BTreeSet<String>> = BTreeMap::new();
        for universe in &simulation.universes {
            universes
                .entry(universe.population.clone())
                .or_default()
                .extend(universe.keys.iter().cloned());
        }
        Self {
            initial: simulation
                .initial
                .iter()
                .map(|document| {
                    (
                        document.authority.clone(),
                        document.identity.clone(),
                        document.revision_namespace.clone(),
                        document.revision.clone(),
                        document.digest,
                    )
                })
                .collect(),
            universes,
            packages: simulation.packages.keys().copied().collect(),
            proof_bounds: subject.proof_bounds.iter().cloned().collect(),
        }
    }

    /// The key set of `population`'s universe; `None` when the subject
    /// supplies none.
    pub fn universe(&self, population: &DeclarationKey) -> Option<&BTreeSet<String>> {
        self.universes.get(population)
    }
}

/// Why [`author_clause`] wrote no item.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum RequestRefusal {
    /// The subject's checked package has no requirement record for the
    /// clause's occurrence, so the clause is not one of its claim sites.
    #[error("the subject's package has no requirement record for the clause {occurrence:?}")]
    NoRequirementRecord {
        /// The clause's occurrence key.
        occurrence: OccurrenceKey,
    },
}

/// The occurrence key of `clause`: its checked node and its claim origin.
pub fn clause_occurrence(clause: &TemporalClauseNode) -> OccurrenceKey {
    OccurrenceKey::new(
        WireNodeId::from_digest(*clause.identity().as_bytes()),
        clause.claim().clone(),
    )
}

/// Write the item of the checked `clause` node over `subject` into `request` and return its
/// index.
///
/// The item's requirement record is the clause's own, read from the
/// subject's checked package, so the subject's universes reach the item only
/// through its obligation identity (FR-336-AC-1). The obligation is the
/// subject's [`SubjectIdentity`] and the clause's occurrence, so the same
/// clause over a different universe is a different item, and writing the
/// same obligation twice returns the one index.
///
/// # Errors
///
/// [`RequestRefusal::NoRequirementRecord`] when the package has no record for
/// the clause.
pub fn author_clause(
    request: &mut ModelRequest<SubjectIdentity>,
    subject: &ModelSubject<'_>,
    clause: &TemporalClauseNode,
) -> Result<RequestIndex, RequestRefusal> {
    let package = subject.simulation.package.graph();
    let occurrence = clause_occurrence(clause);
    let record = package.requirements().get(&occurrence).ok_or_else(|| {
        RequestRefusal::NoRequirementRecord {
            occurrence: occurrence.clone(),
        }
    })?;
    let identity = SubjectIdentity::of(subject);
    let item = ModelTemporalItem {
        occurrence: &occurrence,
        subject: &identity,
        package,
    };
    Ok(request.authored(&item, record.requirements()))
}
