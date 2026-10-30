// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-492: the facade's refinement-query entry (FR-315 "Refinement queries").
//! A consumer encodes the `Initial` or the `Step { row }` simulation query of
//! a refinement with [`encode_refinement_query`], and reads which source row
//! each `steps` row is with [`refinement_step_sources`], through the
//! `qsl_replay` root without naming [`crate::spine`] or `qsl_eval`
//! (ADR-011 FB-05, T-12 rule (a)).
//!
//! The types here are replay-owned wrappers, not re-exports (FR-100-AC-8: no
//! `qsl_replay` re-export names a `qsl_eval` path). They add no digest, hash
//! or revision and compute none: the request holds the checked package and
//! subjects it was built from, unchanged.
//!
//! The refinement node's `steps` array is the rows in the order the V2 wire
//! sorts them, which is not the order they were written in. A `Step { row }`
//! query names a position of that array; [`refinement_step_sources`] gives,
//! for each position, the zero-based position of the same row among the
//! refinement's rows as written.

use qsl_eval::model_subject::ModelSubject;
use qsl_eval::simulation::model::{ExpansionLimits, ModelSimulationRequest, PopulationUniverse};
use qsl_eval::smt::{self, RefinementCertificateRequest};
use qsl_foundation::bound::ProofBound;
use qsl_semantics::model::accounting::ModelNormalizationLimits;
use qsl_semantics::model::intake::PackageInput;
use qsl_semantics::model::observation::{DocumentProvision, DocumentRef, ObservationLimits};
use quire_checked::key::DeclarationKey;
use quire_exact::{NodeKey, ScalarLimits};

use qsl_package::CheckedPackage;

use crate::application::CheckedClosureHandle;
use crate::SmtEncodingRefusal;

/// One simulated population and its finite key set.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct RefinementUniverse {
    /// The population's declaration key.
    pub population: DeclarationKey,
    /// The object keys the population may hold.
    pub keys: Vec<String>,
}

/// One subject of a refinement query, the concrete or the abstract model
/// (FR-314's subject): the provisions its initial states are read from, the
/// initial states, the finite universes, the per-expansion candidate bound,
/// the limits and the proof bounds it is qualified by. The provision maps and
/// document labels are passed through unchanged (QSL-1282 scope). Build one
/// with [`RefinementQuerySubject::new`].
#[non_exhaustive]
pub struct RefinementQuerySubject<'a> {
    /// FR-056's domain package input.
    pub packages: &'a PackageInput,
    /// The FR-106 snapshot provision.
    pub snapshots: &'a DocumentProvision,
    /// The initial states: documents of `snapshots`.
    pub initial: Vec<DocumentRef>,
    /// For each simulated population, its finite key set.
    pub universes: Vec<RefinementUniverse>,
    /// The most candidates one expansion generates.
    pub max_candidates: u64,
    /// The normalization limits FR-106 admission takes.
    pub normalization: ModelNormalizationLimits,
    /// FR-106's document limits.
    pub observation: ObservationLimits,
    /// The evaluation meter budget; each clause evaluation gets a fresh meter.
    pub meter: ScalarLimits,
    /// The ADR-014 B-4 proof bounds the item is qualified by.
    pub proof_bounds: Vec<ProofBound>,
}

impl RefinementUniverse {
    /// The universe of `keys` for the population `population` declared in
    /// `package`, by declared name.
    pub fn for_population(package: &str, population: &str, keys: Vec<String>) -> Self {
        Self::new(
            DeclarationKey {
                package: package.to_owned(),
                node: format!("ix://{package}/{population}"),
            },
            keys,
        )
    }

    /// The universe of `keys` for the population declared by `population`.
    pub fn new(population: DeclarationKey, keys: Vec<String>) -> Self {
        Self { population, keys }
    }
}

impl<'a> RefinementQuerySubject<'a> {
    /// A subject over the provisions `packages` and `snapshots`, starting at
    /// `initial`, simulating `universes`, with at most `max_candidates` per
    /// expansion and the evaluation budget `meter`. The normalization and
    /// observation limits are their defaults and there are no proof bounds;
    /// set the public fields to change them.
    pub fn new(
        packages: &'a PackageInput,
        snapshots: &'a DocumentProvision,
        initial: Vec<DocumentRef>,
        universes: Vec<RefinementUniverse>,
        max_candidates: u64,
        meter: ScalarLimits,
    ) -> Self {
        Self {
            packages,
            snapshots,
            initial,
            universes,
            max_candidates,
            normalization: ModelNormalizationLimits::default(),
            observation: ObservationLimits::default(),
            meter,
            proof_bounds: Vec::new(),
        }
    }
}

impl RefinementQuerySubject<'_> {
    fn model_subject<'p>(&'p self, package: &'p CheckedPackage) -> ModelSubject<'p> {
        ModelSubject {
            simulation: ModelSimulationRequest {
                package,
                packages: self.packages,
                normalization: self.normalization,
                snapshots: self.snapshots,
                initial: self.initial.clone(),
                universes: self
                    .universes
                    .iter()
                    .map(|universe| PopulationUniverse {
                        population: universe.population.clone(),
                        keys: universe.keys.clone(),
                    })
                    .collect(),
                observation: self.observation,
                expansion: ExpansionLimits {
                    max_candidates: self.max_candidates,
                },
                meter: self.meter,
            },
            proof_bounds: self.proof_bounds.clone(),
        }
    }
}

/// The request of a refinement query: the retained checked closure that
/// declares the refinement, the refinement's node key and the concrete and
/// abstract subjects (FR-314's request of a refinement item).
pub struct RefinementQueryRequest<'a> {
    closure: &'a CheckedClosureHandle,
    refinement: NodeKey,
    concrete: RefinementQuerySubject<'a>,
    abstract_: RefinementQuerySubject<'a>,
}

impl<'a> RefinementQueryRequest<'a> {
    /// The request over the refinement `refinement` of `closure`.
    pub fn new(
        closure: &'a CheckedClosureHandle,
        refinement: NodeKey,
        concrete: RefinementQuerySubject<'a>,
        abstract_: RefinementQuerySubject<'a>,
    ) -> Self {
        Self {
            closure,
            refinement,
            concrete,
            abstract_,
        }
    }

    fn with_certificate_request<T>(
        &self,
        body: impl FnOnce(&RefinementCertificateRequest<'_>) -> T,
    ) -> T {
        let package = self.closure.checked_package();
        let concrete = self.concrete.model_subject(package);
        let abstract_ = self.abstract_.model_subject(package);
        body(&RefinementCertificateRequest {
            package,
            refinement: self.refinement,
            concrete: &concrete,
            abstract_: &abstract_,
        })
    }
}

/// Which refinement query to encode.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum RefinementQueryKind {
    /// The initial query: every concrete initial state maps to an abstract
    /// initial state.
    Initial,
    /// The step query of the row at this zero-based position of the
    /// refinement node's `steps` array.
    Step {
        /// The row's position.
        row: u64,
    },
}

impl From<RefinementQueryKind> for smt::RefinementQueryKind {
    fn from(kind: RefinementQueryKind) -> Self {
        match kind {
            RefinementQueryKind::Initial => Self::Initial,
            RefinementQueryKind::Step { row } => Self::Step { row },
        }
    }
}

/// Encode the refinement query `kind` of `request` as an SMT-LIB 2 script in
/// FR-315's canonical printing.
///
/// # Errors
///
/// [`SmtEncodingRefusal`] as FR-315 names it; no script is written.
pub fn encode_refinement_query(
    request: &RefinementQueryRequest<'_>,
    kind: RefinementQueryKind,
) -> Result<Vec<u8>, SmtEncodingRefusal> {
    request.with_certificate_request(|inner| smt::encode_refinement_query(inner, kind.into()))
}

/// The source row of each row of the refinement `request` names, in `steps`
/// order: entry `k` is the position, among the rows as written, of the row at
/// position `k` of the refinement node's `steps` array. The entries are the
/// numbers `0..n` in some order.
///
/// # Errors
///
/// A fault [`SmtEncodingRefusal`] naming why the refinement has no emitted
/// rows, or that the request names no refinement of its package.
pub fn refinement_step_sources(
    request: &RefinementQueryRequest<'_>,
) -> Result<Vec<u64>, SmtEncodingRefusal> {
    request
        .with_certificate_request(smt::refinement_step_sources)?
        .into_iter()
        .map(|source| {
            u64::try_from(source)
                .map_err(|_| SmtEncodingRefusal::fault("a source row position fits a u64"))
        })
        .collect()
}

/// Why [`CheckedClosureHandle::refinement_node_key`] gave no key.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum RefinementKeyRefusal {
    /// The closure declares nothing named `name`.
    #[error("the closure declares no refinement named {name}")]
    UnknownName {
        /// The requested declared name.
        name: String,
    },
    /// `name` is declared, but it is a state clause and not a refinement.
    #[error("{name} is declared, but not as a refinement")]
    NotARefinement {
        /// The requested declared name.
        name: String,
    },
    /// The named refinement has no emitted node.
    #[error("the refinement does not emit: {invariant}")]
    NotEmitted {
        /// The broken invariant.
        invariant: &'static str,
    },
}

impl CheckedClosureHandle {
    /// The node key of the refinement declared as `name`, resolved from the
    /// closure.
    ///
    /// # Errors
    ///
    /// [`RefinementKeyRefusal`] for a name the closure does not declare, a
    /// name declared as another kind, or a refinement with no emitted node.
    pub fn refinement_node_key(&self, name: &str) -> Result<NodeKey, RefinementKeyRefusal> {
        use qsl_semantics::check::RefinementV2EncodingError;
        let graph = self.checked_package().graph();
        let Some(refinement) = graph
            .refinements()
            .iter()
            .find(|refinement| refinement.name() == name)
        else {
            return Err(if graph.state_clause(name).is_some() {
                RefinementKeyRefusal::NotARefinement {
                    name: name.to_owned(),
                }
            } else {
                RefinementKeyRefusal::UnknownName {
                    name: name.to_owned(),
                }
            });
        };
        refinement
            .v2_emission()
            .map(|emission| emission.identity)
            .map_err(|error| RefinementKeyRefusal::NotEmitted {
                invariant: match error {
                    RefinementV2EncodingError::MissingSubject => {
                        "the refinement selects its model subjects"
                    }
                    RefinementV2EncodingError::MissingRows => {
                        "the refinement rows are lowered to node keys"
                    }
                    _ => "the refinement emits its V2 body",
                },
            })
    }
}
