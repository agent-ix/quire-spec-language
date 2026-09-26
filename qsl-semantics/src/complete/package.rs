// SPDX-License-Identifier: AGPL-3.0-or-later
//! Closed complete-V1 package selection and canonical identity domains.
use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;
use std::sync::Arc;

use qsl_foundation::selection::{
    DefinitionDigest, DefinitionRef, InvalidDefinitionComponent, InvalidModelComponent,
    ModelDigest, ModelRef, SourceSelections, MAX_SELECTED_DEFINITIONS,
};
use qsl_foundation::{ByteDigest, Code, Source, SourceIdentity, Span};

use crate::value::semantic_node::IDENTITY_LIMITS as LIMITS;

/// Unforgeable crate-issued proof that typed definition/model parts came from
/// the owning reader boundary.
///
/// There is deliberately no production constructor. Task-054 will connect the
/// concrete checked reader; raw source/editor callers cannot mint this proof.
#[derive(Debug)]
pub struct ReaderAuthority {
    _private: (),
}

impl ReaderAuthority {
    /// Test-only fixture authority (`test-support`), so the integration
    /// tests that parse a source and then resolve it
    /// (`tests/it/complete_package.rs`) can build definitions and models.
    /// It forges the capability this type exists to withhold, so
    /// `test-support` must never be enabled by a shipped dependent: only a
    /// `[dev-dependencies]` entry may turn it on, which
    /// `no_shipped_dependency_enables_test_support` checks.
    #[cfg(any(test, feature = "test-support"))]
    pub const fn fixture() -> Self {
        Self { _private: () }
    }
}

// `DefinitionDigest`, `DefinitionRef`, `ModelDigest`, `ModelRef` and the
// selection lists (`SourceSelections`) are layer-F values
// (`qsl_foundation::selection`, QSL-181): the parser produces them and this
// module resolves them, and neither layer owns them. This module maps their
// validation errors onto `PackageError`.
impl From<InvalidDefinitionComponent> for PackageError {
    fn from(component: InvalidDefinitionComponent) -> Self {
        match component {
            InvalidDefinitionComponent::Identity => Self::InvalidDefinitionIdentity,
            InvalidDefinitionComponent::Version => Self::InvalidDefinitionVersion,
        }
    }
}

impl From<InvalidModelComponent> for PackageError {
    fn from(component: InvalidModelComponent) -> Self {
        match component {
            InvalidModelComponent::Identity => Self::InvalidModelIdentity,
            InvalidModelComponent::Version => Self::InvalidModelVersion,
        }
    }
}

/// Exact compiled-model artifact admitted by its owning model reader.
#[derive(Clone, Debug)]
pub struct ModelArtifact {
    exact: ModelRef,
    exact_bytes: Box<[u8]>,
}

impl ModelArtifact {
    /// Bind an owning reader's typed compiled-model interpretation to the exact
    /// supplied model document bytes selected by native source.
    pub fn from_exact_bytes(
        _authority: &ReaderAuthority,
        identity: impl Into<String>,
        version: impl Into<String>,
        exact_bytes: &[u8],
    ) -> Result<Self, PackageError> {
        if exact_bytes.is_empty() {
            return Err(PackageError::InvalidModelArtifactBytes);
        }
        let exact = ModelRef::new(
            identity,
            version,
            ModelDigest::artifact(ByteDigest::of(exact_bytes)),
        )?;
        Ok(Self {
            exact,
            exact_bytes: exact_bytes.into(),
        })
    }

    /// Exact computed compiled-model reference.
    pub fn exact(&self) -> &ModelRef {
        &self.exact
    }

    /// Exact canonical compiled-model document bytes whose digest appears in
    /// [`Self::exact`].
    pub fn exact_bytes(&self) -> &[u8] {
        &self.exact_bytes
    }
}

/// Role supplied by one exact profile definition.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum DefinitionRole {
    /// Supplies source, package, identity and evolution contracts.
    Source,
    /// Supplies value, model-type and expression contracts.
    ValueModelExpression,
    /// Supplies temporal contracts.
    Temporal,
    /// Supplies observation contracts.
    Observation,
    /// Supplies protocol and choreography contracts.
    Protocol,
    /// Supplies runtime contracts.
    Runtime,
    /// Supplies analysis and method-plan contracts.
    MethodPlan,
    /// Supplies portable IR and backend contracts.
    Backend,
    /// Supplies tooling, mapping and evidence contracts.
    ToolingEvidence,
}

impl DefinitionRole {
    fn facet(self) -> Option<Facet> {
        match self {
            Self::Source => Some(Facet::Source),
            Self::ValueModelExpression => Some(Facet::ValueModelExpression),
            Self::Temporal => Some(Facet::Temporal),
            Self::Observation => Some(Facet::Observation),
            Self::Protocol => Some(Facet::Protocol),
            Self::Runtime => Some(Facet::Runtime),
            Self::MethodPlan => Some(Facet::MethodPlan),
            Self::Backend => Some(Facet::Backend),
            Self::ToolingEvidence => Some(Facet::ToolingEvidence),
        }
    }

    fn identity_name(self) -> &'static str {
        match self {
            Self::Source => "source",
            Self::ValueModelExpression => "value-model-expression",
            Self::Temporal => "temporal",
            Self::Observation => "observation",
            Self::Protocol => "protocol",
            Self::Runtime => "runtime",
            Self::MethodPlan => "method-plan",
            Self::Backend => "backend",
            Self::ToolingEvidence => "tooling-evidence",
        }
    }
}

/// Immutable catalog definition used for exact package-graph resolution.
#[derive(Clone, Debug)]
pub struct Definition {
    exact: DefinitionRef,
    role: DefinitionRole,
    dependencies: BTreeSet<DefinitionRef>,
    capabilities: BTreeSet<CapabilityId>,
    exact_bytes: Box<[u8]>,
}

/// Which [`PackageLimits`] ceiling a [`PackageError::ResourceLimit`] names.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PackageLimitKind {
    /// [`PackageLimits::definitions`].
    Definitions,
    /// [`PackageLimits::dependency_edges`].
    DependencyEdges,
    /// [`PackageLimits::depth`].
    Depth,
    /// [`PackageLimits::artifact_bytes`].
    ArtifactBytes,
    /// [`PackageLimits::single_artifact_bytes`].
    SingleArtifactBytes,
}

impl std::fmt::Display for PackageLimitKind {
    /// The [`PackageLimits`] field name this kind bounds.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Definitions => "definitions",
            Self::DependencyEdges => "dependency_edges",
            Self::Depth => "depth",
            Self::ArtifactBytes => "artifact_bytes",
            Self::SingleArtifactBytes => "single_artifact_bytes",
        })
    }
}

/// Explicit package-graph accounting limits. Every field is enforced
/// exactly as the caller supplies it, above or below [`Self::default`]: an
/// implementation ceiling is not a domain bound (NFR-001), and the default
/// is only the starting point for a caller who configures none.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PackageLimits {
    /// Maximum number of exact definitions (or model artifacts) admitted into
    /// one resolved package graph or catalog.
    pub definitions: usize,
    /// Maximum number of dependency edges traversed while resolving the
    /// package graph.
    pub dependency_edges: usize,
    /// Maximum dependency-chain depth carried on the active resolution stack
    /// before resolution is refused.
    pub depth: usize,
    /// Maximum total artifact byte count (definitions plus models) admitted
    /// into one resolved package or catalog.
    pub artifact_bytes: usize,
    /// Maximum byte count of any one definition or compiled-model artifact
    /// admitted into one resolved package or catalog.
    pub single_artifact_bytes: usize,
}

impl PackageLimits {
    /// Refuses one artifact of `len` bytes past
    /// [`Self::single_artifact_bytes`].
    fn check_single_artifact(&self, len: usize) -> Result<(), PackageError> {
        if len > self.single_artifact_bytes {
            return Err(PackageError::ResourceLimit {
                kind: PackageLimitKind::SingleArtifactBytes,
                limit: self.single_artifact_bytes,
                actual: None,
            });
        }
        Ok(())
    }
}

impl Default for PackageLimits {
    fn default() -> Self {
        Self {
            definitions: MAX_SELECTED_DEFINITIONS,
            dependency_edges: 16_384,
            depth: 256,
            artifact_bytes: 16 * qsl_foundation::source::MAX_SOURCE_BYTES,
            single_artifact_bytes: qsl_foundation::source::MAX_SOURCE_BYTES,
        }
    }
}

impl Definition {
    /// Bind an owning reader's typed definition interpretation to the exact
    /// supplied definition artifact bytes selected by native source.
    pub fn from_exact_bytes(
        _authority: &ReaderAuthority,
        identity: impl Into<String>,
        version: impl Into<String>,
        role: DefinitionRole,
        dependencies: BTreeSet<DefinitionRef>,
        capabilities: BTreeSet<CapabilityId>,
        exact_bytes: &[u8],
    ) -> Result<Self, PackageError> {
        let identity = identity.into();
        let version = version.into();
        if exact_bytes.is_empty() {
            return Err(PackageError::InvalidDefinitionArtifactBytes);
        }
        let exact = DefinitionRef::new(
            identity,
            version,
            DefinitionDigest::from_digest(ByteDigest::of(exact_bytes)),
        )?;
        Ok(Self {
            exact,
            role,
            dependencies,
            capabilities,
            exact_bytes: exact_bytes.into(),
        })
    }

    /// Exact computed definition reference.
    pub fn exact(&self) -> &DefinitionRef {
        &self.exact
    }

    /// Exact canonical definition bytes whose digest appears in [`Self::exact`].
    pub fn exact_bytes(&self) -> &[u8] {
        &self.exact_bytes
    }
}

/// Exact known definition catalog used for profile/package graph validation.
#[derive(Clone, Debug, Default)]
pub struct DefinitionCatalog {
    definitions: BTreeMap<DefinitionRef, Arc<Definition>>,
}

impl DefinitionCatalog {
    /// Build a catalog and refuse duplicate exact records. Multiple versions of
    /// one logical identity may coexist so stale selections can be diagnosed.
    pub fn new(definitions: Vec<Definition>) -> Result<Self, PackageError> {
        Self::with_limits(definitions, PackageLimits::default())
    }

    /// Build a catalog under explicit accounting limits.
    pub fn with_limits(
        definitions: Vec<Definition>,
        limits: PackageLimits,
    ) -> Result<Self, PackageError> {
        if definitions.len() > limits.definitions {
            return Err(PackageError::ResourceLimit {
                kind: PackageLimitKind::Definitions,
                limit: limits.definitions,
                actual: None,
            });
        }
        let mut edges = 0_usize;
        let mut bytes = 0_usize;
        let mut catalog = Self::default();
        for definition in definitions {
            limits.check_single_artifact(definition.exact_bytes.len())?;
            edges = edges.checked_add(definition.dependencies.len()).ok_or(
                PackageError::ResourceLimit {
                    kind: PackageLimitKind::DependencyEdges,
                    limit: limits.dependency_edges,
                    actual: None,
                },
            )?;
            bytes = bytes.checked_add(definition.exact_bytes.len()).ok_or(
                PackageError::ResourceLimit {
                    kind: PackageLimitKind::ArtifactBytes,
                    limit: limits.artifact_bytes,
                    actual: None,
                },
            )?;
            if edges > limits.dependency_edges {
                return Err(PackageError::ResourceLimit {
                    kind: PackageLimitKind::DependencyEdges,
                    limit: limits.dependency_edges,
                    actual: None,
                });
            }
            if bytes > limits.artifact_bytes {
                return Err(PackageError::ResourceLimit {
                    kind: PackageLimitKind::ArtifactBytes,
                    limit: limits.artifact_bytes,
                    actual: None,
                });
            }
            if catalog
                .definitions
                .insert(definition.exact.clone(), Arc::new(definition))
                .is_some()
            {
                return Err(PackageError::DuplicateDefinition);
            }
        }
        Ok(catalog)
    }

    fn exact(&self, selected: &DefinitionRef) -> Option<&Arc<Definition>> {
        self.definitions.get(selected)
    }

    fn contains_identity(&self, identity: &str) -> bool {
        self.definitions
            .keys()
            .any(|definition| definition.identity() == identity)
    }

    fn contains_version(&self, identity: &str, version: &str) -> bool {
        self.definitions
            .keys()
            .any(|definition| definition.identity() == identity && definition.version() == version)
    }
}

/// Exact compiled-model inventory, separate from semantic definitions.
#[derive(Clone, Debug, Default)]
pub struct ModelCatalog {
    models: BTreeMap<ModelRef, Arc<ModelArtifact>>,
}

impl ModelCatalog {
    /// Build a catalog and refuse duplicate exact compiled-model artifacts.
    pub fn new(models: Vec<ModelArtifact>) -> Result<Self, PackageError> {
        Self::with_limits(models, PackageLimits::default())
    }

    /// Build a compiled-model catalog under explicit accounting limits.
    pub fn with_limits(
        models: Vec<ModelArtifact>,
        limits: PackageLimits,
    ) -> Result<Self, PackageError> {
        if models.len() > limits.definitions {
            return Err(PackageError::ResourceLimit {
                kind: PackageLimitKind::Definitions,
                limit: limits.definitions,
                actual: None,
            });
        }
        let mut bytes = 0_usize;
        let mut catalog = Self::default();
        for model in models {
            limits.check_single_artifact(model.exact_bytes.len())?;
            bytes =
                bytes
                    .checked_add(model.exact_bytes.len())
                    .ok_or(PackageError::ResourceLimit {
                        kind: PackageLimitKind::ArtifactBytes,
                        limit: limits.artifact_bytes,
                        actual: None,
                    })?;
            if bytes > limits.artifact_bytes {
                return Err(PackageError::ResourceLimit {
                    kind: PackageLimitKind::ArtifactBytes,
                    limit: limits.artifact_bytes,
                    actual: None,
                });
            }
            if catalog
                .models
                .insert(model.exact.clone(), Arc::new(model))
                .is_some()
            {
                return Err(PackageError::DuplicateModel);
            }
        }
        Ok(catalog)
    }

    fn exact(&self, selected: &ModelRef) -> Option<&Arc<ModelArtifact>> {
        self.models.get(selected)
    }

    fn contains_identity(&self, identity: &str) -> bool {
        self.models.keys().any(|model| model.identity() == identity)
    }
}

/// Original source authority retained through package graph resolution/lowering.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceAuthority {
    /// Caller-selected source identity and revision labels.
    pub identity: SourceIdentity,
    /// Exact raw-byte digest of the source that produced this package graph.
    pub digest: SourceDigest,
    /// Source path recorded for diagnostics and located refusal messages.
    pub path: String,
}

impl SourceAuthority {
    /// The authority of `source`: its identity, exact raw-byte digest and
    /// path.
    pub fn of(source: &Source) -> Self {
        Self {
            identity: source.identity().clone(),
            digest: SourceDigest(source.digest()),
            path: source.path().into(),
        }
    }
}

/// Syntax/package-graph checked result; not the later type-checked package.
#[derive(Clone, Debug)]
pub struct ResolvedSourcePackage {
    authority: SourceAuthority,
    bundle: CompleteBundle,
    definitions: BTreeMap<DefinitionRef, Arc<Definition>>,
    models: BTreeMap<ModelRef, Arc<ModelArtifact>>,
    effective_limits: PackageLimits,
}

impl ResolvedSourcePackage {
    /// Original source authority (identity, digest, path) this package was
    /// resolved from.
    pub fn authority(&self) -> &SourceAuthority {
        &self.authority
    }
    /// The dependency-closed complete bundle selected for this package.
    pub fn bundle(&self) -> &CompleteBundle {
        &self.bundle
    }
    /// Exact dependency closure selected by source.
    pub fn definitions(&self) -> &BTreeMap<DefinitionRef, Arc<Definition>> {
        &self.definitions
    }
    /// Exact compiled-model artifacts selected by source, kept outside the
    /// semantic-definition closure.
    pub fn models(&self) -> &BTreeMap<ModelRef, Arc<ModelArtifact>> {
        &self.models
    }
    /// The [`PackageLimits`] this resolution was checked against, exactly as
    /// the caller supplied them: every field is enforced as given, so a
    /// caller-raised ceiling is visible with the result it produced.
    pub fn effective_limits(&self) -> PackageLimits {
        self.effective_limits
    }
}

macro_rules! digest_domain {
    ($name:ident, $description:literal) => {
        #[doc = $description]
        #[derive(Clone, Copy, Debug, Eq, PartialEq)]
        pub struct $name(ByteDigest);

        impl $name {
            /// Construct this identity in its declared domain from exact bytes.
            pub fn of(bytes: &[u8]) -> Self {
                let mut separated = concat!(stringify!($name), "\0").as_bytes().to_vec();
                separated.extend_from_slice(bytes);
                Self(ByteDigest::of(&separated))
            }

            /// Borrow the underlying SHA-256 value after domain selection.
            pub fn digest(self) -> ByteDigest {
                self.0
            }
        }
    };
}

/// Exact SHA-256 identity of authored source bytes.
///
/// ```compile_fail
/// use qsl_semantics::complete::{SemanticDigest, SourceDigest};
/// let source: SourceDigest = SemanticDigest::of(b"same bytes");
/// ```
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SourceDigest(ByteDigest);

impl SourceDigest {
    /// Hash the exact source bytes without a semantic-domain prefix.
    pub fn of(bytes: &[u8]) -> Self {
        Self(ByteDigest::of(bytes))
    }

    /// Borrow the underlying exact-byte SHA-256 value.
    pub fn digest(self) -> ByteDigest {
        self.0
    }
}

digest_domain!(SemanticDigest, "Linked semantic-definition identity.");

/// One exact atomic capability spelling.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct CapabilityId(String);

impl CapabilityId {
    /// Resolve one of the closed 176 complete-V1 inventory identifiers.
    pub fn complete(value: &str) -> Result<Self, PackageError> {
        if is_complete_capability(value) {
            Ok(Self(value.into()))
        } else {
            Err(PackageError::UnknownCapability(value.into()))
        }
    }

    /// Stable package spelling.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Closed complete-V1 inventory in normative family and ordinal order.
    pub fn complete_inventory() -> Vec<Self> {
        COMPLETE_CAPABILITY_FAMILIES
            .into_iter()
            .flat_map(|(family, count)| {
                (1..=count).map(move |ordinal| Self(format!("V1-{family}-{ordinal:03}")))
            })
            .collect()
    }
}

fn is_complete_capability(value: &str) -> bool {
    let Some(rest) = value.strip_prefix("V1-") else {
        return false;
    };
    let Some((family, ordinal)) = rest.rsplit_once('-') else {
        return false;
    };
    if ordinal.len() != 3 || !ordinal.bytes().all(|byte| byte.is_ascii_digit()) {
        return false;
    }
    let Ok(ordinal) = ordinal.parse::<u16>() else {
        return false;
    };
    let Some((_, maximum)) = COMPLETE_CAPABILITY_FAMILIES
        .iter()
        .find(|(candidate, _)| *candidate == family)
    else {
        return false;
    };
    (1..=*maximum).contains(&ordinal)
}

const COMPLETE_CAPABILITY_FAMILIES: [(&str, u16); 9] = [
    ("SRC", 15),
    ("TYPE", 31),
    ("EXPR", 26),
    ("TEMP", 23),
    ("PROT", 31),
    ("RUN", 20),
    ("BACK", 14),
    ("EVID", 6),
    ("TOOL", 10),
];

/// Dependency-closed semantic facets of the complete package.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Facet {
    /// Source, package, identity and evolution contracts.
    Source,
    /// Values, model types and expression contracts.
    ValueModelExpression,
    /// Temporal contracts.
    Temporal,
    /// Observation contracts.
    Observation,
    /// Protocol and choreography contracts.
    Protocol,
    /// Runtime contracts.
    Runtime,
    /// Analysis and method-plan contracts.
    MethodPlan,
    /// Portable IR and backend contracts.
    Backend,
    /// Tooling, mapping and evidence contracts.
    ToolingEvidence,
}

impl Facet {
    /// Every required complete-V1 facet.
    pub const fn all() -> &'static [Self] {
        &[
            Self::Source,
            Self::ValueModelExpression,
            Self::Temporal,
            Self::Observation,
            Self::Protocol,
            Self::Runtime,
            Self::MethodPlan,
            Self::Backend,
            Self::ToolingEvidence,
        ]
    }
}

/// Immutable result of selecting the dependency-closed complete profile.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompleteBundle {
    facets: BTreeSet<Facet>,
    capabilities: BTreeSet<CapabilityId>,
    identity: SemanticDigest,
}

impl CompleteBundle {
    /// Selected facets, independent of installed execution backends.
    pub fn facets(&self) -> &BTreeSet<Facet> {
        &self.facets
    }

    /// All 176 atomic required capabilities.
    pub fn capabilities(&self) -> &BTreeSet<CapabilityId> {
        &self.capabilities
    }

    /// Semantic identity of the closed selection.
    pub fn identity(&self) -> SemanticDigest {
        self.identity
    }

    /// Verify that a consumer understands every atomic selected capability.
    pub fn validate_known_capabilities(
        &self,
        known: &BTreeSet<CapabilityId>,
    ) -> Result<(), PackageError> {
        if let Some(unknown) = self.capabilities.difference(known).next() {
            Err(PackageError::UnknownCapability(unknown.as_str().into()))
        } else {
            Ok(())
        }
    }
}

/// Link the complete profile. Backend support is deliberately not an input.
fn link_complete_bundle(selected: BTreeSet<Facet>) -> Result<CompleteBundle, PackageError> {
    let required: BTreeSet<_> = Facet::all().iter().copied().collect();
    if let Some(missing) = required.difference(&selected).next() {
        return Err(PackageError::MissingFacet(*missing));
    }
    let capabilities: BTreeSet<_> = CapabilityId::complete_inventory().into_iter().collect();
    let identity = SemanticDigest::of(&encode_set(capabilities.iter().map(CapabilityId::as_str))?);
    Ok(CompleteBundle {
        facets: selected,
        capabilities,
        identity,
    })
}

/// Located package-graph refusal with a stable complete producer code.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[error("{code:?} in {authority:?} at {span:?}: {cause}")]
pub struct PackageRefusal {
    /// Stable producer classification.
    pub code: Code,
    /// Exact source identity, path and raw-byte digest being refused.
    pub authority: Box<SourceAuthority>,
    /// Exact selecting source range.
    pub span: Span,
    /// Typed corrective cause.
    pub cause: PackageError,
    /// Closed catalogued cause tag, admitted by the catalog for `code`.
    pub cause_tag: ResolutionCause,
}

/// The closed catalogued cause tag of a package-graph refusal, under
/// `quire.native.diagnostics/v1` revision `1-draft.7`. Layer 3's own subset of
/// the cause vocabulary: the parser's `qsl_cst::CompleteCause` is layer 1's,
/// and resolution never sees a syntax cause.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ResolutionCause {
    /// `unknown_profile`: the selection is not supported by this consumer.
    UnsupportedSelection,
    /// `stale_dependency`: the selected version differs from the known one.
    RevisionMismatch,
    /// `stale_dependency`: the selected version is known with another digest.
    ByteDigestMismatch,
    /// `resource_exhausted`: the next charge exceeds its limit.
    InsufficientNextCharge,
    /// `missing_import`: no known definition has the selected identity.
    MissingSelection,
    /// `ambiguous_declaration`: one alias is declared twice in a namespace.
    AmbiguousName,
    /// `ambiguous_declaration`: one definition identity is selected at two
    /// distinct exact selections.
    ConflictingAuthority,
    /// `invalid_package`: a catalog holds one exact selection twice.
    DuplicateMember,
    /// `invalid_package`: a definition or model member is malformed.
    InvalidValue,
    /// `invalid_package`: the definition dependency graph has a cycle.
    DefinitionCycle,
    /// `invalid_package`: the resolved closure lacks a required facet.
    FeatureSetMismatch,
    /// `unknown_required_feature`: a capability name outside the inventory.
    UnknownFeature,
    /// `unknown_required_feature`: a known capability the closure does not
    /// provide.
    UnsupportedFeature,
    /// `invalid_model_binding`: the selected compiled model is absent or stale.
    WrongModelSelection,
    /// `invalid_model_binding`: one model identity is selected twice with
    /// distinct exact selections.
    ConflictingBinding,
    /// `stage_limit_exceeded` (QSL-236, revision `1-draft.6`): the package
    /// graph's own dependency-chain depth ceiling
    /// ([`PackageLimitKind::Depth`]), the one [`PackageLimitKind`] that maps
    /// cleanly onto a T-4 [`qsl_foundation::diagnostic::LimitKind`]. The
    /// other ceilings (`Definitions`, `DependencyEdges`, `ArtifactBytes`,
    /// `SingleArtifactBytes`) stay `InsufficientNextCharge`.
    StageLimit(qsl_foundation::diagnostic::LimitKind),
}

impl ResolutionCause {
    /// The stable catalog tag.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::UnsupportedSelection => "unsupported-selection",
            Self::RevisionMismatch => "revision-mismatch",
            Self::ByteDigestMismatch => "byte-digest-mismatch",
            Self::InsufficientNextCharge => "insufficient-next-charge",
            Self::MissingSelection => "missing-selection",
            Self::AmbiguousName => "ambiguous-name",
            Self::ConflictingAuthority => "conflicting-authority",
            Self::DuplicateMember => "duplicate-member",
            Self::InvalidValue => "invalid-value",
            Self::DefinitionCycle => "definition-cycle",
            Self::FeatureSetMismatch => "feature-set-mismatch",
            Self::UnknownFeature => "unknown-feature",
            Self::UnsupportedFeature => "unsupported-feature",
            Self::WrongModelSelection => "wrong-model-selection",
            Self::ConflictingBinding => "conflicting-binding",
            Self::StageLimit(kind) => kind.catalog_cause(),
        }
    }

    /// Whether the catalog admits this cause for `code`.
    pub fn is_cause_of(self, code: Code) -> bool {
        match self {
            Self::UnsupportedSelection => matches!(
                code,
                Code::UnknownLanguage | Code::UnknownEdition | Code::UnknownProfile
            ),
            Self::RevisionMismatch | Self::ByteDigestMismatch => {
                matches!(code, Code::StaleDependency | Code::SourceDigestMismatch)
            }
            Self::InsufficientNextCharge => code == Code::ResourceExhausted,
            Self::MissingSelection => code == Code::MissingImport,
            Self::AmbiguousName | Self::ConflictingAuthority => code == Code::AmbiguousDeclaration,
            Self::DuplicateMember
            | Self::InvalidValue
            | Self::DefinitionCycle
            | Self::FeatureSetMismatch => code == Code::InvalidPackage,
            Self::UnknownFeature | Self::UnsupportedFeature => code == Code::UnknownRequiredFeature,
            Self::WrongModelSelection | Self::ConflictingBinding => {
                code == Code::InvalidModelBinding
            }
            Self::StageLimit(_) => code == Code::StageLimitExceeded,
        }
    }
}

/// Resolve exact source selections into a dependency-closed complete bundle.
///
/// Obligation on the caller: `authority` and `selections` come from one and
/// the same admitted parse. This layer sees no syntax (QSL-181), so it can
/// check neither; both are plain values a caller could pair wrongly, and a
/// wrong pair resolves one source's selections under another's authority.
/// The shipped caller step, `command::resolve_parsed_source` (layer 6),
/// refuses an inadmissible parse and derives both from the parse it is
/// given; call that rather than this function.
pub fn resolve_source_package(
    authority: SourceAuthority,
    selections: &SourceSelections,
    catalog: &DefinitionCatalog,
    models: &ModelCatalog,
    limits: PackageLimits,
) -> Result<ResolvedSourcePackage, PackageRefusal> {
    let refusal = |code, span, cause| refusal(&authority, code, span, cause);
    for (namespace, aliases) in [
        (
            "profile",
            selections
                .profiles
                .iter()
                .map(|selection| (selection.alias.as_str(), selection.span))
                .collect::<Vec<_>>(),
        ),
        (
            "import",
            selections
                .imports
                .iter()
                .map(|selection| {
                    (
                        selection
                            .alias
                            .as_deref()
                            .unwrap_or(selection.identity.as_str()),
                        selection.span,
                    )
                })
                .collect(),
        ),
        (
            "model",
            selections
                .models
                .iter()
                .map(|selection| (selection.alias.as_str(), selection.span))
                .collect(),
        ),
    ] {
        let mut seen = BTreeMap::new();
        for (alias, span) in aliases {
            if let Some(first_span) = seen.insert(alias, span) {
                return Err(refusal(
                    Code::AmbiguousDeclaration,
                    span,
                    PackageError::DuplicateAlias {
                        namespace: namespace.into(),
                        alias: alias.into(),
                        first_span,
                    },
                ));
            }
        }
    }
    // Only profiles select catalog definitions. An `import` names a library
    // that the S4 source resolution compiles from source (ADR-015 D-1), so
    // it is not a definition root here.
    let roots: Vec<_> = selections
        .profiles
        .iter()
        .map(|selection| (&selection.definition, selection.identity_span))
        .collect();

    let mut resolved_models = BTreeMap::new();
    let mut logical_models = BTreeMap::<&str, (&ModelRef, Span)>::new();
    for selection in &selections.models {
        let selected = &selection.model;
        if let Some((previous, previous_span)) =
            logical_models.insert(selected.identity(), (selected, selection.span))
        {
            if previous != selected {
                return Err(refusal(
                    Code::InvalidModelBinding,
                    selection.span,
                    PackageError::ConflictingModels(Box::new(ModelConflict {
                        first: previous.clone(),
                        first_span: previous_span,
                        second: selected.clone(),
                    })),
                ));
            }
        }
        if let ModelDigest::DomainPackage(_) = selected.digest() {
            return Err(refusal(
                Code::InvalidModelBinding,
                selection.span,
                PackageError::DomainPackageModel(Box::new(selected.clone())),
            ));
        }
        let Some(model) = models.exact(selected) else {
            let cause = if models.contains_identity(selected.identity()) {
                PackageError::StaleModel(Box::new(selected.clone()))
            } else {
                PackageError::MissingModel(Box::new(selected.clone()))
            };
            return Err(refusal(Code::InvalidModelBinding, selection.span, cause));
        };
        resolved_models.insert(selected.clone(), model.clone());
    }
    if roots
        .len()
        .checked_add(resolved_models.len())
        .is_none_or(|count| count > limits.definitions)
    {
        return Err(refusal(
            Code::ResourceExhausted,
            Span { start: 0, end: 0 },
            PackageError::ResourceLimit {
                kind: PackageLimitKind::Definitions,
                limit: limits.definitions,
                actual: None,
            },
        ));
    }

    let mut logical = BTreeMap::<&str, (&DefinitionRef, Span)>::new();
    for (selected, span) in &roots {
        if let Some((previous, previous_span)) =
            logical.insert(selected.identity(), (selected, *span))
        {
            if previous != *selected {
                return Err(refusal(
                    Code::AmbiguousDeclaration,
                    *span,
                    PackageError::ConflictingDefinitions(Box::new(DefinitionConflict {
                        first: previous.clone(),
                        first_span: previous_span,
                        second: (*selected).clone(),
                    })),
                ));
            }
        }
    }

    for (selected, span) in &roots {
        if catalog.exact(selected).is_none() {
            let (code, cause) = if catalog.contains_identity(selected.identity()) {
                if catalog.contains_version(selected.identity(), selected.version()) {
                    let mut stale = refusal(
                        Code::StaleDependency,
                        *span,
                        PackageError::StaleDefinition((*selected).clone()),
                    );
                    stale.cause_tag = ResolutionCause::ByteDigestMismatch;
                    return Err(stale);
                }
                (
                    Code::StaleDependency,
                    PackageError::StaleDefinition((*selected).clone()),
                )
            } else {
                (
                    Code::UnknownProfile,
                    PackageError::MissingDefinition((*selected).clone()),
                )
            };
            return Err(refusal(code, *span, cause));
        }
    }

    let mut resolved = BTreeMap::new();
    let mut provenance = BTreeMap::<DefinitionRef, Span>::new();
    let mut provenance_order = BTreeMap::<DefinitionRef, usize>::new();
    let mut next_provenance_order = 0_usize;
    let mut active = Vec::new();
    let mut done = BTreeSet::new();
    let mut traversed_edges = 0_usize;
    for (root, span) in roots {
        let mut work = vec![(root.clone(), false)];
        while let Some((selected, expanded)) = work.pop() {
            if done.contains(&selected) {
                continue;
            }
            if expanded {
                active.pop();
                done.insert(selected.clone());
                if resolved.len() >= limits.definitions {
                    return Err(refusal(
                        Code::ResourceExhausted,
                        span,
                        PackageError::ResourceLimit {
                            kind: PackageLimitKind::Definitions,
                            limit: limits.definitions,
                            actual: None,
                        },
                    ));
                }
                resolved.insert(selected.clone(), catalog.definitions[&selected].clone());
                continue;
            }
            if let std::collections::btree_map::Entry::Vacant(entry) =
                provenance.entry(selected.clone())
            {
                entry.insert(span);
                provenance_order.insert(selected.clone(), next_provenance_order);
                next_provenance_order = next_provenance_order.saturating_add(1);
            }
            if let Some(start) = active.iter().position(|definition| definition == &selected) {
                return Err(refusal(
                    Code::InvalidPackage,
                    span,
                    PackageError::DefinitionCycle(
                        active[start..].iter().cloned().chain([selected]).collect(),
                    ),
                ));
            }
            let Some(definition) = catalog.exact(&selected) else {
                return Err(refusal(
                    Code::MissingImport,
                    span,
                    PackageError::MissingDefinition(selected),
                ));
            };
            if active.len() >= limits.depth {
                return Err(refusal(
                    // QSL-236: the graph's dependency-chain depth ceiling
                    // maps onto `stage_limit_exceeded`/`nesting-depth-exceeded`
                    // (`cause_tag` picks the tag from the `PackageError`).
                    Code::StageLimitExceeded,
                    span,
                    PackageError::ResourceLimit {
                        kind: PackageLimitKind::Depth,
                        limit: limits.depth,
                        actual: Some(active.len() + 1),
                    },
                ));
            }
            traversed_edges = traversed_edges
                .checked_add(definition.dependencies.len())
                .ok_or_else(|| {
                    refusal(
                        Code::ResourceExhausted,
                        span,
                        PackageError::ResourceLimit {
                            kind: PackageLimitKind::DependencyEdges,
                            limit: limits.dependency_edges,
                            actual: None,
                        },
                    )
                })?;
            if traversed_edges > limits.dependency_edges {
                return Err(refusal(
                    Code::ResourceExhausted,
                    span,
                    PackageError::ResourceLimit {
                        kind: PackageLimitKind::DependencyEdges,
                        limit: limits.dependency_edges,
                        actual: None,
                    },
                ));
            }
            active.push(selected.clone());
            work.push((selected, true));
            for dependency in definition.dependencies.iter().rev() {
                work.push((dependency.clone(), false));
            }
        }
    }

    let mut closed_logical = BTreeMap::<&str, (&DefinitionRef, Span)>::new();
    let mut discovered: Vec<_> = resolved.keys().collect();
    discovered.sort_by_key(|selected| provenance_order[*selected]);
    for selected in discovered {
        let selected_span = provenance[selected];
        if let Some((previous, previous_span)) =
            closed_logical.insert(selected.identity(), (selected, selected_span))
        {
            if previous != selected {
                return Err(refusal(
                    Code::AmbiguousDeclaration,
                    selected_span,
                    PackageError::ConflictingDefinitions(Box::new(DefinitionConflict {
                        first: previous.clone(),
                        first_span: previous_span,
                        second: selected.clone(),
                    })),
                ));
            }
        }
    }

    let artifact_span = selections
        .profiles
        .first()
        .map(|selection| selection.span)
        .or_else(|| selections.imports.first().map(|selection| selection.span))
        .or_else(|| selections.models.first().map(|selection| selection.span))
        .unwrap_or(Span { start: 0, end: 0 });
    for len in resolved
        .values()
        .map(|definition| definition.exact_bytes.len())
        .chain(
            resolved_models
                .values()
                .map(|model| model.exact_bytes.len()),
        )
    {
        limits
            .check_single_artifact(len)
            .map_err(|cause| refusal(Code::ResourceExhausted, artifact_span, cause))?;
    }
    let resolved_artifact_bytes = resolved
        .values()
        .try_fold(0_usize, |total, definition| {
            total.checked_add(definition.exact_bytes.len())
        })
        .and_then(|definition_bytes| {
            resolved_models
                .values()
                .try_fold(definition_bytes, |total, model| {
                    total.checked_add(model.exact_bytes.len())
                })
        });
    if resolved_artifact_bytes.is_none_or(|bytes| bytes > limits.artifact_bytes) {
        return Err(refusal(
            Code::ResourceExhausted,
            artifact_span,
            PackageError::ResourceLimit {
                kind: PackageLimitKind::ArtifactBytes,
                limit: limits.artifact_bytes,
                actual: None,
            },
        ));
    }

    let facets: BTreeSet<_> = resolved
        .values()
        .filter_map(|definition| definition.role.facet())
        .collect();
    let capabilities: BTreeSet<_> = resolved
        .values()
        .flat_map(|definition| definition.capabilities.iter().cloned())
        .collect();
    let required_capabilities: BTreeSet<_> =
        CapabilityId::complete_inventory().into_iter().collect();
    if let Some(missing) = required_capabilities.difference(&capabilities).next() {
        return Err(refusal(
            Code::UnknownRequiredFeature,
            selections
                .profiles
                .first()
                .map_or(Span { start: 0, end: 0 }, |selection| selection.span),
            PackageError::MissingCapability(missing.clone()),
        ));
    }
    let mut bundle = link_complete_bundle(facets).map_err(|cause| {
        identity_refusal(
            &authority,
            selections
                .profiles
                .first()
                .map_or(Span { start: 0, end: 0 }, |selection| selection.span),
            cause,
        )
    })?;
    bundle.capabilities = capabilities;
    bundle.identity = resolved_graph_identity(&resolved, &resolved_models, &bundle.capabilities)
        .map_err(|cause| identity_refusal(&authority, Span { start: 0, end: 0 }, cause))?;
    Ok(ResolvedSourcePackage {
        authority,
        bundle,
        definitions: resolved,
        models: resolved_models,
        effective_limits: limits,
    })
}

fn refusal(
    authority: &SourceAuthority,
    code: Code,
    span: Span,
    cause: PackageError,
) -> PackageRefusal {
    PackageRefusal {
        code,
        authority: Box::new(authority.clone()),
        span,
        cause_tag: cause_tag(code, &cause),
        cause,
    }
}

/// The catalogued cause tag of a package-graph refusal. A stale definition is
/// tagged at its call site, which knows whether the version or only the digest
/// differs; here it is the version.
fn cause_tag(code: Code, cause: &PackageError) -> ResolutionCause {
    use ResolutionCause as Tag;
    match cause {
        PackageError::InvalidDefinitionIdentity
        | PackageError::InvalidDefinitionVersion
        | PackageError::InvalidDefinitionArtifactBytes
        | PackageError::InvalidModelIdentity
        | PackageError::InvalidModelVersion
        | PackageError::InvalidModelArtifactBytes => Tag::InvalidValue,
        PackageError::DuplicateDefinition | PackageError::DuplicateModel => Tag::DuplicateMember,
        PackageError::MissingDefinition(_) if code == Code::UnknownProfile => {
            Tag::UnsupportedSelection
        }
        PackageError::MissingDefinition(_) => Tag::MissingSelection,
        PackageError::StaleDefinition(_) => Tag::RevisionMismatch,
        PackageError::MissingModel(_)
        | PackageError::StaleModel(_)
        | PackageError::DomainPackageModel(_) => Tag::WrongModelSelection,
        PackageError::ConflictingModels(_) => Tag::ConflictingBinding,
        PackageError::ConflictingDefinitions(_) => Tag::ConflictingAuthority,
        PackageError::DuplicateAlias { .. } => Tag::AmbiguousName,
        PackageError::DefinitionCycle(_) => Tag::DefinitionCycle,
        PackageError::MissingCapability(_) => Tag::UnsupportedFeature,
        PackageError::UnknownCapability(_) => Tag::UnknownFeature,
        // QSL-236: the graph's own dependency-chain depth is the one
        // `PackageLimitKind` that maps cleanly onto a T-4 `LimitKind`
        // (`NestingDepth`); the byte and count ceilings do not and stay
        // `InsufficientNextCharge`.
        PackageError::ResourceLimit {
            kind: PackageLimitKind::Depth,
            ..
        } => Tag::StageLimit(qsl_foundation::diagnostic::LimitKind::NestingDepth),
        PackageError::CanonicalSize | PackageError::ResourceLimit { .. } => {
            Tag::InsufficientNextCharge
        }
        PackageError::MissingFacet(_) => Tag::FeatureSetMismatch,
    }
}

fn identity_refusal(
    authority: &SourceAuthority,
    span: Span,
    cause: PackageError,
) -> PackageRefusal {
    let code = match &cause {
        PackageError::CanonicalSize | PackageError::ResourceLimit { .. } => Code::ResourceExhausted,
        _ => Code::InvalidPackage,
    };
    refusal(authority, code, span, cause)
}

fn encode_set<'a>(values: impl Iterator<Item = &'a str>) -> Result<Vec<u8>, PackageError> {
    let mut output = Vec::new();
    for value in values {
        field(&mut output, "item", value.as_bytes())?;
    }
    Ok(output)
}

/// Appends one length-prefixed field. The only bound is the wire format's
/// own: each length must fit its `u64` prefix ([`PackageError::CanonicalSize`]).
/// [`encode_set`]'s one caller encodes the closed, compile-time capability
/// inventory, so no caller-supplied size reaches here to need a ceiling.
fn field(output: &mut Vec<u8>, name: &str, value: &[u8]) -> Result<(), PackageError> {
    let name_len = u64::try_from(name.len()).map_err(|_| PackageError::CanonicalSize)?;
    let value_len = u64::try_from(value.len()).map_err(|_| PackageError::CanonicalSize)?;
    output.extend_from_slice(&name_len.to_be_bytes());
    output.extend_from_slice(name.as_bytes());
    output.extend_from_slice(&value_len.to_be_bytes());
    output.extend_from_slice(value);
    Ok(())
}

/// The digest-domain label of a resolved package graph's identity.
const RESOLVED_GRAPH_DOMAIN: &[u8] = b"quire.complete.resolved-graph/2";

/// A resolved package graph's identity preimage: every resolved definition
/// (its exact reference, role, dependencies and capabilities) and model, in
/// their maps' key order, then the linked bundle's capabilities.
#[derive(Serialize)]
struct ResolvedGraphPreimage<'a> {
    definitions: Vec<ResolvedDefinitionPreimage<'a>>,
    models: Vec<ExactRefPreimage<'a>>,
    capabilities: Vec<&'a str>,
}

#[derive(Serialize)]
struct ResolvedDefinitionPreimage<'a> {
    exact: ExactRefPreimage<'a>,
    role: &'static str,
    dependencies: Vec<ExactRefPreimage<'a>>,
    capabilities: Vec<&'a str>,
}

/// An exact `{identity, version, digest}` reference; the digest is spelled
/// `sha256:<hex>`, as `ByteDigest` displays it.
#[derive(Serialize)]
struct ExactRefPreimage<'a> {
    identity: &'a str,
    version: &'a str,
    digest: String,
}

impl<'a> From<&'a DefinitionRef> for ExactRefPreimage<'a> {
    fn from(exact: &'a DefinitionRef) -> Self {
        Self {
            identity: exact.identity(),
            version: exact.version(),
            digest: exact.digest().digest().to_string(),
        }
    }
}

impl<'a> From<&'a ModelRef> for ExactRefPreimage<'a> {
    fn from(exact: &'a ModelRef) -> Self {
        Self {
            identity: exact.identity(),
            version: exact.version(),
            digest: exact.digest().to_string(),
        }
    }
}

/// The [`SemanticDigest`] of a resolved package graph: SHA-256 over the
/// `quire.complete.resolved-graph/2` domain label and the RFC 8785 bytes of
/// its [`ResolvedGraphPreimage`], through `quire-canonical`'s named-domain
/// digest (ADR-013 §2, ADR-013:113: the one RFC 8785 implementation).
///
/// A refusal of the encoder (only a failed heap reservation is reachable
/// for a preimage of strings, which [`LIMITS`] bounds in depth alone) is
/// [`PackageError::CanonicalSize`]: the graph's canonical form cannot be
/// produced at its size.
fn resolved_graph_identity(
    definitions: &BTreeMap<DefinitionRef, Arc<Definition>>,
    models: &BTreeMap<ModelRef, Arc<ModelArtifact>>,
    capabilities: &BTreeSet<CapabilityId>,
) -> Result<SemanticDigest, PackageError> {
    let preimage = ResolvedGraphPreimage {
        definitions: definitions
            .values()
            .map(|definition| ResolvedDefinitionPreimage {
                exact: ExactRefPreimage::from(&definition.exact),
                role: definition.role.identity_name(),
                dependencies: definition
                    .dependencies
                    .iter()
                    .map(ExactRefPreimage::from)
                    .collect(),
                capabilities: definition
                    .capabilities
                    .iter()
                    .map(CapabilityId::as_str)
                    .collect(),
            })
            .collect(),
        models: models
            .values()
            .map(|model| ExactRefPreimage::from(&model.exact))
            .collect(),
        capabilities: capabilities.iter().map(CapabilityId::as_str).collect(),
    };
    let digest = quire_canonical::sha256_with_domain(RESOLVED_GRAPH_DOMAIN, &preimage, LIMITS)
        .map_err(|_| PackageError::CanonicalSize)?;
    ByteDigest::from_hex(&digest.to_string())
        .map(SemanticDigest)
        .map_err(|_| PackageError::CanonicalSize)
}

/// Typed complete-package refusal.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum PackageError {
    /// Definition identity was empty or outside its bound.
    #[error("invalid definition identity")]
    InvalidDefinitionIdentity,
    /// Definition version was empty or outside its bound.
    #[error("invalid definition version")]
    InvalidDefinitionVersion,
    /// Supplied definition artifact bytes were empty. Size is a caller
    /// limit ([`PackageLimits::single_artifact_bytes`]), checked where the
    /// artifact is admitted into a catalog or resolved package.
    #[error("invalid exact definition artifact bytes")]
    InvalidDefinitionArtifactBytes,
    /// Compiled-model identity was empty or outside its bound.
    #[error("invalid compiled-model identity")]
    InvalidModelIdentity,
    /// Compiled-model version was empty or outside its bound.
    #[error("invalid compiled-model version")]
    InvalidModelVersion,
    /// Supplied compiled-model document bytes were empty. Size is a caller
    /// limit ([`PackageLimits::single_artifact_bytes`]), checked where the
    /// artifact is admitted into a catalog or resolved package.
    #[error("invalid exact compiled-model document bytes")]
    InvalidModelArtifactBytes,
    /// An exact definition appeared more than once in a catalog.
    #[error("duplicate exact definition")]
    DuplicateDefinition,
    /// An exact compiled-model artifact appeared more than once.
    #[error("duplicate exact compiled-model artifact")]
    DuplicateModel,
    /// A selected exact definition was absent.
    #[error("missing definition {0:?}")]
    MissingDefinition(DefinitionRef),
    /// A known logical definition was selected with stale version/digest.
    #[error("stale definition {0:?}")]
    StaleDefinition(DefinitionRef),
    /// A selected compiled-model artifact was absent.
    #[error("missing compiled-model artifact {0:?}")]
    MissingModel(Box<ModelRef>),
    /// A known compiled model was selected with stale version/digest.
    #[error("stale compiled-model artifact {0:?}")]
    StaleModel(Box<ModelRef>),
    /// A `model` declaration selects a domain package (`sha256-jcs:`); this
    /// resolver binds compiled-model artifacts only, and spine intake
    /// admits domain packages (FR-056).
    #[error("model {} selects a domain package by {}; only spine intake admits one", .0.identity(), .0.digest())]
    DomainPackageModel(Box<ModelRef>),
    /// Two source model selections conflict on one logical identity.
    #[error("conflicting compiled-model selections")]
    ConflictingModels(Box<ModelConflict>),
    /// Two source selections conflict on one logical identity.
    #[error("conflicting definition selections {0:?}")]
    ConflictingDefinitions(Box<DefinitionConflict>),
    /// One source namespace reused a local alias.
    #[error("duplicate {namespace} alias {alias}; first declared at {first_span:?}")]
    DuplicateAlias {
        /// The namespace (`profile`, `import`, or `model`) that reused the alias.
        namespace: String,
        /// The alias spelling that was declared more than once.
        alias: String,
        /// Where the alias was first declared.
        first_span: Span,
    },
    /// Exact definition dependency graph is cyclic.
    #[error("definition dependency cycle {0:?}")]
    DefinitionCycle(Vec<DefinitionRef>),
    /// Dependency closure omitted one atomic required capability.
    #[error("missing required capability {0:?}")]
    MissingCapability(CapabilityId),
    /// Canonical length cannot be represented by the versioned wire encoding.
    #[error("canonical field exceeds the u64 wire length domain")]
    CanonicalSize,
    /// Package input reached a configured [`PackageLimits`] ceiling.
    #[error("package resource limit exceeded: {kind} (limit {limit})")]
    ResourceLimit {
        /// Which ceiling was reached.
        kind: PackageLimitKind,
        /// The configured bound in force when the ceiling was reached.
        limit: usize,
        /// The counter value the refused step would have reached (QSL-236,
        /// M2). Only [`PackageLimitKind::Depth`] fills this in: it is the
        /// one kind that maps onto a catalogued stage limit. The byte and
        /// count ceilings stay `resource_exhausted` (S1 `SyntaxLimit`
        /// itself stays bound-only, with no matching `actual` field), so
        /// they carry no counter here either.
        actual: Option<usize>,
    },
    /// One required complete facet was omitted.
    #[error("complete V1 is missing facet {0:?}")]
    MissingFacet(Facet),
    /// Required capability is not understood and cannot be dropped.
    #[error("unknown required capability {0}")]
    UnknownCapability(String),
}

/// All source evidence for one logical-definition selection conflict.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DefinitionConflict {
    /// Earlier exact selection.
    pub first: DefinitionRef,
    /// Earlier selection locus.
    pub first_span: Span,
    /// Conflicting later exact selection.
    pub second: DefinitionRef,
}

/// All source evidence for one logical-model selection conflict.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelConflict {
    /// Earlier exact selection.
    pub first: ModelRef,
    /// Earlier selection locus.
    pub first_span: Span,
    /// Conflicting later exact selection.
    pub second: ModelRef,
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;
    use sha2::{Digest as _, Sha256};

    use super::*;

    /// QSL-194 golden vector for the resolved-graph identity's named
    /// `quire-canonical` domain: SHA-256 over the label's big-endian `u64`
    /// length, the label `quire.complete.resolved-graph/2`, and the
    /// preimage's RFC 8785 text, all written out by hand. This domain
    /// replaces the length-prefixed `SemanticIdentityBuilder` encoding
    /// (label `quire.complete.resolved-graph/1`), so its digests are new.
    #[trace("TC-180", "FR-131-AC-2")]
    #[test]
    fn resolved_graph_identity_matches_its_golden_vector() {
        const TEXT: &str =
            r#"{"capabilities":["V1-SRC-001","V1-TYPE-002"],"definitions":[],"models":[]}"#;
        const DIGEST: &str = "0cf46736b80d36aafb61f77297f1d41debb0182c6d9c4b689912661f94282209";
        let mut hasher = Sha256::new();
        hasher.update(
            u64::try_from(RESOLVED_GRAPH_DOMAIN.len())
                .unwrap()
                .to_be_bytes(),
        );
        hasher.update(RESOLVED_GRAPH_DOMAIN);
        hasher.update(TEXT.as_bytes());
        assert_eq!(format!("{:x}", hasher.finalize()), DIGEST);

        let capabilities = ["V1-SRC-001", "V1-TYPE-002"]
            .into_iter()
            .map(|id| CapabilityId::complete(id).unwrap())
            .collect();
        let identity =
            resolved_graph_identity(&BTreeMap::new(), &BTreeMap::new(), &capabilities).unwrap();
        assert_eq!(format!("{:x}", identity.digest()), DIGEST);
    }
}
