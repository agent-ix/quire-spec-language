// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-336 Outputs (ADR-013 O-09): the identity projection of a model
//! subject.

use std::collections::{BTreeMap, BTreeSet};

use qsl_eval::model_subject::ModelSubject;
use qsl_foundation::bound::{DomainKey, DomainKind, FiniteBound, ProofBound};
use qsl_package::{emit_checked, EmitRefusal};
use qsl_semantics::library::PackageId;
use quire_checked::key::DeclarationKey;
use quire_exact::Integer;

/// An initial state's document, as the identity reads it: the four source
/// labels and the digest.
type InitialState = (String, String, String, String, [u8; 32]);

/// The identity projection of a [`ModelSubject`] (ADR-013 O-09, FR-336): what
/// a verdict over the subject holds for.
///
/// It is the subject's checked package (its canonical `package_id`, which
/// covers every contract the successor relation reads, not only the clause's
/// own nodes), the initial states, each population's universe, the digests of
/// the domain packages the package's model selections name, and the proof
/// bounds. The order of the initial states, of a universe's keys and of the
/// proof bounds is immaterial. The expansion limits, the evaluation meter,
/// the observation and normalization limits and the snapshot provision decide
/// whether a run finishes or reads, not what a finished run holds, so they
/// are no part of it.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct SubjectIdentity {
    package: PackageId,
    initial: BTreeSet<InitialState>,
    universes: BTreeMap<DeclarationKey, BTreeSet<String>>,
    packages: BTreeSet<[u8; 32]>,
    proof_bounds: BTreeSet<BoundKey>,
}

/// A finite bound in an orderable form, so the identity's set of proof
/// bounds is ordered and hashes without depending on listing order.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
enum Bound {
    Cardinality(u64),
    IntegerRange(Integer, Integer),
    Depth(u64),
    Variants(Vec<String>),
}

/// One proof bound: its domain, its kind and its finite bound.
type BoundKey = (DomainKey, Option<DomainKind>, Bound);

fn bound_key(bound: &ProofBound) -> BoundKey {
    let finite = match bound.bound() {
        FiniteBound::Cardinality { maximum } => Bound::Cardinality(*maximum),
        FiniteBound::IntegerRange(range) => {
            Bound::IntegerRange(range.lower().clone(), range.upper().clone())
        }
        FiniteBound::Depth { maximum } => Bound::Depth(maximum.get()),
        FiniteBound::Variants { members } => Bound::Variants(members.clone()),
    };
    (bound.domain().clone(), bound.kind(), finite)
}

/// Why no [`SubjectIdentity`] was made.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum SubjectIdentityRefusal {
    /// The subject's checked package has no emission, so no canonical
    /// identity.
    #[error(transparent)]
    Package(#[from] EmitRefusal),
}

impl SubjectIdentity {
    /// The identity of `subject`.
    ///
    /// # Errors
    ///
    /// [`SubjectIdentityRefusal::Package`] when the subject's checked package
    /// has no emission, so no canonical identity.
    pub fn of(subject: &ModelSubject<'_>) -> Result<Self, SubjectIdentityRefusal> {
        let simulation = &subject.simulation;
        let mut universes: BTreeMap<DeclarationKey, BTreeSet<String>> = BTreeMap::new();
        for universe in &simulation.universes {
            universes
                .entry(universe.population.clone())
                .or_default()
                .extend(universe.keys.iter().cloned());
        }
        let package = emit_checked(simulation.package)?.package().package_id();
        Ok(Self {
            package,
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
            packages: simulation
                .package
                .graph()
                .model_selections()
                .iter()
                .map(|selection| selection.digest)
                .collect(),
            proof_bounds: subject.proof_bounds.iter().map(bound_key).collect(),
        })
    }

    /// The key set of `population`'s universe; `None` when the subject
    /// supplies none.
    pub fn universe(&self, population: &DeclarationKey) -> Option<&BTreeSet<String>> {
        self.universes.get(population)
    }
}
