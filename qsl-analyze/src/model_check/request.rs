// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-336-AC-4 (ADR-013 O-09): the EN-1 request path's writer of a model
//! subject's temporal items.
//!
//! [`SubjectContext::author_clause`] builds an item of a [`ModelRequest`] from a checked
//! clause and the [`ModelSubject`] EN-1 would check it over. The request is
//! generic over the subject it identifies, and a [`ModelSubject`] is a large
//! live input (a checked package, document bytes, limits) with no equality of
//! its own, so the request holds the subject's [`SubjectIdentity`]: the part
//! of the subject a verdict is about. A verdict over one universe joins only
//! its own request index, never another universe's.

use std::collections::BTreeSet;

use qsl_eval::model_subject::ModelSubject;
use qsl_foundation::source::provenance::OccurrenceKey;
use qsl_foundation::RequestIndex;
use qsl_package::{CheckedPackage, EmitRefusal};
use qsl_replay::{node_occurrence, SubjectIdentity, SubjectIdentityRefusal};
use qsl_route::deadlock::{ModelRefinementItem, ModelTemporalItem};
use qsl_route::model_request::ModelRequest;
use qsl_route::request::AbstractSubject;
use qsl_semantics::check::{CheckedRefinement, TemporalClauseNode};
use qsl_semantics::model::observation::DocumentRef;

/// Why [`SubjectContext::author_clause`] wrote no item.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum RequestRefusal {
    /// The subject's checked package has no requirement record for the
    /// clause's occurrence, so the clause is not one of its claim sites.
    #[error("the subject's package has no requirement record for the clause {occurrence:?}")]
    NoRequirementRecord {
        /// The clause's occurrence key.
        occurrence: OccurrenceKey,
    },
    /// The subject's checked package wrote no requirement record for the
    /// refinement (FR-144), so it has no item.
    #[error("the subject's package has no requirement record for the refinement {name}")]
    NoRefinementRecord {
        /// The refinement's declared name.
        name: String,
    },
    /// The subject's checked package has no canonical identity.
    #[error(transparent)]
    Package(#[from] EmitRefusal),
}

/// A model subject with its identity computed once, from which the items of
/// any number of clauses are written. Building it emits the subject's checked
/// package for its identity; writing a clause's item does not.
pub struct SubjectContext<'s, 'a> {
    subject: &'s ModelSubject<'a>,
    identity: SubjectIdentity,
    /// The `model_selections` rows the subject reads: the domain packages
    /// of the populations its universes cover (QSpec FR-361, FR-368).
    selections: BTreeSet<&'s str>,
}

impl<'s, 'a> SubjectContext<'s, 'a> {
    /// The context of `subject`.
    ///
    /// # Errors
    ///
    /// [`RequestRefusal::Package`] when the subject's checked package has no
    /// canonical identity.
    pub fn new(subject: &'s ModelSubject<'a>) -> Result<Self, RequestRefusal> {
        Ok(Self {
            subject,
            identity: SubjectIdentity::of(subject).map_err(|refusal| match refusal {
                SubjectIdentityRefusal::Package(refusal) => RequestRefusal::Package(refusal),
            })?,
            selections: subject
                .simulation
                .universes
                .iter()
                .map(|universe| universe.population.package.as_str())
                .collect(),
        })
    }

    /// The subject's identity.
    pub fn identity(&self) -> &SubjectIdentity {
        &self.identity
    }

    /// Write the item of the checked `clause` node over this subject into
    /// `request` and return its index.
    ///
    /// The item's requirement record is the clause's own, read from the
    /// subject's checked package, so the subject's universes reach the item
    /// only through its obligation identity (FR-336-AC-1). The obligation is
    /// the subject's [`SubjectIdentity`] and the clause's occurrence, so the
    /// same clause over a different universe is a different item, and
    /// writing the same obligation twice returns the one index.
    ///
    /// # Errors
    ///
    /// [`RequestRefusal::NoRequirementRecord`] when the package has no record
    /// for the clause.
    pub fn author_clause(
        &self,
        request: &mut ModelRequest<SubjectIdentity>,
        clause: &TemporalClauseNode,
    ) -> Result<RequestIndex, RequestRefusal> {
        let package = self.subject.simulation.package.graph();
        let occurrence = node_occurrence(clause);
        let record = package.requirements().get(&occurrence).ok_or_else(|| {
            RequestRefusal::NoRequirementRecord {
                occurrence: occurrence.clone(),
            }
        })?;
        let item = ModelTemporalItem {
            occurrence: &occurrence,
            subject: &self.identity,
            package,
        };
        Ok(request.authored(&item, record.requirements(), &self.selections))
    }

    /// Write the item of the checked `refinement` (FR-144), with this subject
    /// as its concrete subject and the abstract library `abstract_library` with
    /// `abstract_initial` as the abstract subject, into `request` and return
    /// its index.
    ///
    /// The item's record is the refinement's own, read from the subject's
    /// checked package. It carries the abstract subject as the abstract
    /// package's library name, as the caller's request names it, and the initial snapshots (QSpec FR-368-AC-19). Its
    /// obligation is the subject's identity, the refinement's occurrence and
    /// the abstract initial snapshots, so the same refinement against other
    /// abstract initial states is another item.
    ///
    /// # Errors
    ///
    /// [`RequestRefusal::NoRefinementRecord`] when the package has no record
    /// for the refinement.
    pub fn author_refinement(
        &self,
        request: &mut ModelRequest<SubjectIdentity>,
        refinement: &CheckedRefinement,
        abstract_library: qsl_semantics::library::LibraryName,
        abstract_initial: &[DocumentRef],
    ) -> Result<RequestIndex, RequestRefusal> {
        let package = self.subject.simulation.package.graph();
        let missing = || RequestRefusal::NoRefinementRecord {
            name: refinement.name().to_owned(),
        };
        let record = refinement
            .requirement_occurrence()
            .and_then(|occurrence| package.requirements().get(occurrence))
            .ok_or_else(missing)?;
        let item = ModelRefinementItem {
            refinement,
            subject: &self.identity,
            abstract_initial,
            package,
        };
        let abstract_subject = AbstractSubject {
            library: abstract_library,
            initial_states: abstract_initial.to_vec(),
        };
        request
            .refinement(&item, record.requirements(), abstract_subject)
            .ok_or_else(missing)
    }
}
