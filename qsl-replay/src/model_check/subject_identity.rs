// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-336 Outputs (ADR-013 O-09): the identity projection of a model
//! subject.

use std::collections::hash_map::DefaultHasher;
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::hash::{Hash, Hasher};

use qsl_eval::model_subject::ModelSubject;
use qsl_foundation::bound::ProofBound;
use qsl_package::{emit_checked, EmitRefusal};
use qsl_semantics::library::PackageId;
use quire_checked::key::DeclarationKey;

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
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SubjectIdentity {
    package: PackageId,
    initial: BTreeSet<InitialState>,
    universes: BTreeMap<DeclarationKey, BTreeSet<String>>,
    packages: BTreeSet<[u8; 32]>,
    proof_bounds: HashSet<ProofBound>,
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
            proof_bounds: subject.proof_bounds.iter().cloned().collect(),
        })
    }

    /// The key set of `population`'s universe; `None` when the subject
    /// supplies none.
    pub fn universe(&self, population: &DeclarationKey) -> Option<&BTreeSet<String>> {
        self.universes.get(population)
    }
}

impl Hash for SubjectIdentity {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.package.hash(state);
        self.initial.hash(state);
        self.universes.hash(state);
        self.packages.hash(state);
        // A set's iteration order is unspecified, so fold the members' hashes
        // with an operation that does not depend on the order.
        let folded = self
            .proof_bounds
            .iter()
            .map(|bound| {
                let mut member = DefaultHasher::new();
                bound.hash(&mut member);
                member.finish()
            })
            .fold(0u64, u64::wrapping_add);
        self.proof_bounds.len().hash(state);
        folded.hash(state);
    }
}
