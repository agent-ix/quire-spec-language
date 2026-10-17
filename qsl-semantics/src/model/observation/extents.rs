// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-364: the relationship instances an observation holds, the population
//! manifest that decides `resolve`, and the bundle that binds them.
//!
//! The extent store, the manifest and `resolve` are `quire-semantic-value`'s
//! (FR-119). This module is the model layer's side of that boundary: the one
//! entry that builds an observation's extents from full references (FR-106's
//! snapshot admission goes through the same entry), the typed refusals
//! mapped to the admission catalog, and the bundle key of FR-364 "One bundle
//! binds the manifest, the observation and the type environment".

use qsl_foundation::diagnostic::Code;
use qsl_foundation::source::provenance::RawSourceRef;
use quire_exact::{NodeKey, ObjectReference};
use quire_semantic_value::declaration::TypeEnvironment;
use quire_semantic_value::object_closure::ObjectClosure;
use quire_semantic_value::relationship_extent::{
    ClosedPopulationManifest, EndSide, ExtentRefusal, ExtentState, ManifestFailure,
    ManifestOutcome, NavigationRelationship, NavigationRelationshipTable, PopulationEntry,
    PopulationEnumerator, RelationshipExtent, RelationshipExtentStore, Resolution, StopReason,
    TableRefusal,
};

use super::document::ObservationRole;
use super::{fault, refuse, AdmissionFailure, AdmissionRecord};
use crate::model::key::DeclarationKey;
use crate::model::object_environment::ObjectEnvironment;

/// Why the population enumerator stopped before normal exhaustion: it could
/// not read the population declaration `declaration`. This is the only way a
/// manifest is partial (FR-364 "The population manifest").
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PopulationEnumerationFailure {
    /// The population declaration the enumerator could not read.
    pub declaration: DeclarationKey,
}

/// What the traversal of the package's population declarations left behind:
/// the rows it read and, when it stopped early, the failure. The QSV
/// manifest state is rebuilt from this evidence on demand, because the QSV
/// manifest borrows the type environment of the reader.
#[derive(Clone, Debug)]
struct RetainedTraversal {
    rows: Vec<PopulationEntry<DeclarationKey>>,
    failure: Option<PopulationEnumerationFailure>,
    row_limit: usize,
}

/// The enumerator over the retained traversal: its rows, then its failure or
/// normal exhaustion.
struct Replay {
    rows: std::vec::IntoIter<PopulationEntry<DeclarationKey>>,
    failure: Option<PopulationEnumerationFailure>,
}

impl PopulationEnumerator<DeclarationKey> for Replay {
    type Error = PopulationEnumerationFailure;

    fn next_row(
        &mut self,
    ) -> Result<Option<PopulationEntry<DeclarationKey>>, PopulationEnumerationFailure> {
        match self.rows.next() {
            Some(row) => Ok(Some(row)),
            None => match self.failure.take() {
                Some(failure) => Err(failure),
                None => Ok(None),
            },
        }
    }
}

/// A population manifest as `resolve` reads it: closed by an exhausted
/// traversal, or partial with the failure.
pub type PopulationManifest<'t> = ManifestOutcome<'t, DeclarationKey, PopulationEnumerationFailure>;

/// FR-364: the extents of every navigation relationship an observation lists,
/// and the evidence of the traversal of the package's populations that
/// `resolve` classifies a missing object by.
#[derive(Clone, Debug)]
pub struct RelationshipExtents {
    store: RelationshipExtentStore<DeclarationKey>,
    traversal: RetainedTraversal,
}

impl RelationshipExtents {
    /// FR-364 "Extent admission is the same for every way of building an
    /// extent": admit `extents` (full references, in the order supplied)
    /// against the navigation relationships `navigation`, and traverse the
    /// package's population declarations with `enumerator`, whose row limit
    /// is `row_limit`, the number of population declarations of the package.
    ///
    /// # Errors
    ///
    /// The first failing condition of FR-364 conditions 1 to 4 as
    /// `Refused`, with the relationship, the zero-based instance and the end
    /// it names; a navigation table or a manifest the store cannot hold is a
    /// `Fault`, because both come from the authenticated declarations.
    pub fn admit<E>(
        types: &TypeEnvironment,
        navigation: impl IntoIterator<Item = NavigationRelationship<DeclarationKey>>,
        extents: impl IntoIterator<Item = RelationshipExtent<DeclarationKey>>,
        enumerator: &mut E,
        row_limit: usize,
    ) -> Result<Self, AdmissionFailure>
    where
        E: PopulationEnumerator<DeclarationKey, Error = PopulationEnumerationFailure>,
    {
        let table = NavigationRelationshipTable::new(types, navigation).map_err(table_fault)?;
        let store = RelationshipExtentStore::new(types, &table, extents)
            .map_err(|refusal| extent_refusal(types, refusal))?;
        let outcome = ClosedPopulationManifest::from_enumerator(types, enumerator, row_limit)
            .map_err(manifest_fault)?;
        let failure = match outcome.stop() {
            None => None,
            Some(StopReason::EnumeratorError(failure)) => Some(failure.clone()),
            Some(StopReason::RowLimit { .. }) => return Err(fault("population-row-limit-reached")),
        };
        Ok(Self {
            store,
            traversal: RetainedTraversal {
                rows: outcome.rows().cloned().collect(),
                failure,
                row_limit,
            },
        })
    }

    /// The listed state of `relationship`: unlisted, incomplete or complete.
    pub fn state(&self, relationship: &DeclarationKey) -> ExtentState {
        self.store.state(relationship)
    }

    /// FR-364 `complete(R)`: `true` exactly when a complete extent of
    /// `relationship` is held.
    pub fn complete(&self, relationship: &DeclarationKey) -> bool {
        self.store.complete(relationship)
    }

    /// FR-364 `targets(R, r)`: the target of every instance of
    /// `relationship` whose source is `source`, in ascending reference-key
    /// order.
    pub fn targets<'a>(
        &'a self,
        relationship: &DeclarationKey,
        source: &ObjectReference,
    ) -> impl Iterator<Item = &'a ObjectReference> {
        self.store.targets(relationship, source)
    }

    /// FR-364 `sources(R, r)`: the source of every instance of
    /// `relationship` whose target is `target`, in ascending reference-key
    /// order.
    pub fn sources<'a>(
        &'a self,
        relationship: &DeclarationKey,
        target: &ObjectReference,
    ) -> impl Iterator<Item = &'a ObjectReference> {
        self.store.sources(relationship, target)
    }

    /// The population manifest, rebuilt from the retained traversal over
    /// `types`: closed exactly when the traversal reached normal exhaustion.
    ///
    /// # Errors
    ///
    /// A `Fault` when the store cannot hold the retained rows, which the
    /// admitted traversal already held.
    pub fn manifest<'t>(
        &self,
        types: &'t TypeEnvironment,
    ) -> Result<PopulationManifest<'t>, AdmissionFailure> {
        let mut replay = Replay {
            rows: self.traversal.rows.clone().into_iter(),
            failure: self.traversal.failure.clone(),
        };
        ClosedPopulationManifest::from_enumerator(types, &mut replay, self.traversal.row_limit)
            .map_err(manifest_fault)
    }
}

fn table_fault(refusal: TableRefusal<DeclarationKey>) -> AdmissionFailure {
    match refusal {
        TableRefusal::DuplicateRelationshipKey(_) => fault("navigation-table-duplicate-key"),
        TableRefusal::UnknownEndType { .. } => fault("navigation-table-unknown-end-type"),
        TableRefusal::Allocation(_) | TableRefusal::Capacity(_) => {
            fault("navigation-table-storage")
        }
    }
}

fn manifest_fault(failure: ManifestFailure<DeclarationKey>) -> AdmissionFailure {
    match failure {
        ManifestFailure::DuplicatePopulation(_) => fault("population-manifest-duplicate-row"),
        ManifestFailure::UnknownCoveredType { .. } => {
            fault("population-manifest-unknown-covered-type")
        }
        ManifestFailure::Allocation(_) | ManifestFailure::Capacity(_) => {
            fault("population-manifest-storage")
        }
    }
}

/// The catalog name of `end`.
pub(super) fn end_name(end: EndSide) -> &'static str {
    match end {
        EndSide::Source => "source",
        EndSide::Target => "target",
    }
}

/// The name a type is reported under: its declared name, or its identity
/// when the model declares no object type of it.
fn type_name(types: &TypeEnvironment, id: quire_exact::EffectiveId) -> String {
    types
        .object_type(id)
        .map_or_else(|| format!("{id:?}"), |declared| declared.name().to_owned())
}

/// FR-364 "Where each refusal fires": the admission record of one failed
/// extent condition.
fn extent_refusal(
    types: &TypeEnvironment,
    refusal: ExtentRefusal<DeclarationKey>,
) -> AdmissionFailure {
    match refusal {
        ExtentRefusal::NotNavigationRelationship { relationship, .. } => refuse(
            AdmissionRecord::new(Code::InvalidRuntimeInput, "wrong-role-mapping")
                .with("relationship", relationship.node),
        ),
        ExtentRefusal::DuplicateExtent { relationship, .. } => refuse(
            AdmissionRecord::new(Code::InvalidRuntimeInput, "conflicting-identity")
                .with("relationship", relationship.node),
        ),
        ExtentRefusal::EndTypeMismatch {
            relationship,
            instance_index,
            side,
            actual_type,
            declared_type,
            ..
        } => refuse(
            AdmissionRecord::new(Code::InvalidRuntimeInput, "wrong-value-kind")
                .with("relationship", relationship.node)
                .with("instance", instance_index.to_string())
                .with("end", end_name(side))
                .with("type", type_name(types, actual_type))
                .with("declared", type_name(types, declared_type)),
        ),
        ExtentRefusal::DuplicateTriple {
            relationship,
            instance_index,
            ..
        } => refuse(
            AdmissionRecord::new(Code::InvalidRuntimeInput, "conflicting-identity")
                .with("relationship", relationship.node)
                .with("instance", instance_index.to_string()),
        ),
        ExtentRefusal::Allocation(_) | ExtentRefusal::Capacity(_) => fault("extent-store-storage"),
    }
}

// ---------------------------------------------------------------------------
// The bundle
// ---------------------------------------------------------------------------

/// Where an observation's identity comes from: the digest of the snapshot
/// document it was built from, or the label the model-layer producer issued.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ObservationOrigin {
    /// A snapshot-built observation: its own document's `sha256-jcs` digest.
    Document([u8; 32]),
    /// A model-layer observation: the producer's label.
    Label(String),
}

/// FR-364 "One bundle binds the manifest, the observation and the type
/// environment": the checked unit's source reference as a whole, the
/// selection, and the observation's own identity. Two keys are equal only
/// when every component is equal.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BundleKey {
    /// The checked unit's source reference (authority, identity and digest).
    pub source: RawSourceRef,
    /// The selected clause.
    pub clause: NodeKey,
    /// The digest of each selected document, for a snapshot-built selection.
    pub documents: Vec<[u8; 32]>,
    /// The observation's role.
    pub role: ObservationRole,
    /// The observation's own identity.
    pub origin: ObservationOrigin,
}

/// A part of a bundle, in the order a constructor checks them.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BundlePart {
    /// The population manifest.
    Manifest,
    /// The observation's object closure.
    Closure,
    /// The type environment.
    Types,
}

impl BundlePart {
    fn name(self) -> &'static str {
        match self {
            Self::Manifest => "manifest",
            Self::Closure => "closure",
            Self::Types => "types",
        }
    }
}

/// A part issued under the key it was built for.
#[derive(Clone, Debug)]
pub struct Keyed<T> {
    /// The key the producer built this part for.
    pub key: BundleKey,
    /// The part.
    pub part: T,
}

/// FR-364-AC-14: the first part whose key differs from the selected key.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BundleRefusal {
    /// The first differing part, in the order manifest, closure, types.
    pub part: BundlePart,
    /// The selected key.
    pub selected: BundleKey,
    /// The key the differing part carries.
    pub differing: BundleKey,
}

impl BundleRefusal {
    /// The admission record: `invalid_runtime_input`/`conflicting-identity`
    /// naming the part.
    pub fn record(&self) -> AdmissionRecord {
        AdmissionRecord::new(Code::InvalidRuntimeInput, "conflicting-identity")
            .with("part", self.part.name())
    }
}

/// The manifest, the closure and the type environment of one selected key:
/// the only way to `resolve` a missing reference.
#[derive(Debug)]
pub struct ResolveBundle<'a, 't> {
    manifest: PopulationManifest<'t>,
    closure: &'a ObjectClosure,
    types: &'t TypeEnvironment,
}

impl<'a, 't> ResolveBundle<'a, 't> {
    /// Bind the three parts to `selected`.
    ///
    /// # Errors
    ///
    /// The first part, in the order `manifest`, `closure`, `types`, whose key
    /// differs from `selected`.
    pub fn new(
        selected: &BundleKey,
        manifest: Keyed<PopulationManifest<'t>>,
        closure: Keyed<&'a ObjectClosure>,
        types: Keyed<&'t TypeEnvironment>,
    ) -> Result<Self, Box<BundleRefusal>> {
        let differing = [
            (BundlePart::Manifest, &manifest.key),
            (BundlePart::Closure, &closure.key),
            (BundlePart::Types, &types.key),
        ]
        .into_iter()
        .find(|(_, key)| *key != selected);
        if let Some((part, key)) = differing {
            return Err(Box::new(BundleRefusal {
                part,
                selected: selected.clone(),
                differing: key.clone(),
            }));
        }
        Ok(Self {
            manifest: manifest.part,
            closure: closure.part,
            types: types.part,
        })
    }

    /// The bundle of `environment`'s own manifest, closure and `types`, all
    /// bound to `key`.
    ///
    /// # Errors
    ///
    /// A `Fault` when the environment holds no extents or the manifest
    /// cannot be rebuilt.
    pub fn over(
        environment: &'a ObjectEnvironment,
        key: &BundleKey,
        types: &'t TypeEnvironment,
    ) -> Result<Self, AdmissionFailure> {
        let extents = environment
            .extents()
            .ok_or_else(|| fault("bundle-extents-missing"))?;
        let manifest = Keyed {
            key: key.clone(),
            part: extents.manifest(types)?,
        };
        let closure = Keyed {
            key: key.clone(),
            part: environment.objects(),
        };
        let held_types = Keyed {
            key: key.clone(),
            part: types,
        };
        Self::new(key, manifest, closure, held_types)
            .map_err(|_| fault("bundle-key-disagrees-with-itself"))
    }

    /// FR-364 `resolve(x)`: `present` when the observation holds the object;
    /// otherwise `dangling` or `undecidable` by the manifest alone.
    pub fn resolve(&self, reference: &ObjectReference) -> Resolution {
        self.closure.resolve(reference, self.types, &self.manifest)
    }
}

impl super::AdmittedObservations {
    /// The bundle of the `role` observation of this admission over `types`,
    /// keyed by `source` and the clause this admission was made for.
    ///
    /// # Errors
    ///
    /// A `Fault` when this admission holds no such observation or no
    /// extents, or the manifest cannot be rebuilt.
    pub fn bundle<'a, 't>(
        &'a self,
        role: ObservationRole,
        source: &RawSourceRef,
        types: &'t TypeEnvironment,
    ) -> Result<ResolveBundle<'a, 't>, AdmissionFailure> {
        let observation = match role {
            ObservationRole::Current => self.current.as_ref(),
            ObservationRole::Pre => self.pre.as_ref(),
            ObservationRole::Post => self.post.as_ref(),
        }
        .ok_or_else(|| fault("bundle-observation-missing"))?;
        let key = BundleKey {
            source: source.clone(),
            clause: self.clause,
            documents: [&self.current, &self.pre, &self.post]
                .into_iter()
                .flatten()
                .map(|held| held.identity.digest)
                .collect(),
            role,
            origin: ObservationOrigin::Document(observation.identity.digest),
        };
        ResolveBundle::over(&observation.environment, &key, types)
    }
}
