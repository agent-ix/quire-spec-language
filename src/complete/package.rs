// SPDX-License-Identifier: AGPL-3.0-or-later
//! Closed complete-V1 package selection and canonical identity domains.
use std::collections::{BTreeMap, BTreeSet};

use sha2::{Digest as _, Sha256};
use std::sync::Arc;

use crate::{ByteDigest, SourceIdentity, Span};

/// Unforgeable crate-issued proof that typed definition/model parts came from
/// the owning reader boundary.
///
/// There is deliberately no public constructor. Task-054 will connect the
/// concrete checked reader; raw source/editor callers cannot mint this proof.
#[derive(Debug)]
pub struct ReaderAuthority {
    _private: (),
}

impl ReaderAuthority {
    #[cfg(test)]
    pub(crate) const fn crate_owned() -> Self {
        Self { _private: () }
    }
}

/// Exact versioned definition digest in the profile/import domain.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct DefinitionDigest(ByteDigest);

impl DefinitionDigest {
    /// Parse the canonical SHA-256 spelling selected by source.
    pub fn parse(value: &str) -> Result<Self, PackageError> {
        value
            .parse()
            .map(Self)
            .map_err(|_| PackageError::InvalidDefinitionDigest(value.into()))
    }

    /// Canonical selected value.
    pub fn digest(self) -> ByteDigest {
        self.0
    }
}

/// Exact definition identity/version/digest triple.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct DefinitionRef {
    /// Opaque definition identity.
    identity: String,
    /// Exact selected version.
    version: String,
    /// Exact content digest.
    digest: DefinitionDigest,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum InvalidDefinitionComponent {
    Identity,
    Version,
}

impl DefinitionRef {
    /// Validate a non-empty exact definition selection.
    pub fn new(
        identity: impl Into<String>,
        version: impl Into<String>,
        digest: DefinitionDigest,
    ) -> Result<Self, PackageError> {
        let (identity, version) = (identity.into(), version.into());
        Self::validate_components(&identity, &version).map_err(|component| match component {
            InvalidDefinitionComponent::Identity => PackageError::InvalidDefinitionIdentity,
            InvalidDefinitionComponent::Version => PackageError::InvalidDefinitionVersion,
        })?;
        Ok(Self::from_validated(identity, version, digest))
    }

    pub(crate) fn validate_components(
        identity: &str,
        version: &str,
    ) -> Result<(), InvalidDefinitionComponent> {
        if identity.is_empty() || identity.len() > 512 {
            return Err(InvalidDefinitionComponent::Identity);
        }
        if version.is_empty() || version.len() > 256 {
            return Err(InvalidDefinitionComponent::Version);
        }
        Ok(())
    }

    pub(crate) fn from_validated(
        identity: String,
        version: String,
        digest: DefinitionDigest,
    ) -> Self {
        Self {
            identity,
            version,
            digest,
        }
    }

    /// Opaque definition identity.
    pub fn identity(&self) -> &str {
        &self.identity
    }

    /// Exact selected version.
    pub fn version(&self) -> &str {
        &self.version
    }

    /// Exact raw-byte definition digest.
    pub fn digest(&self) -> DefinitionDigest {
        self.digest
    }
}

/// Raw-byte SHA-256 digest of one compiled-model document.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ModelDigest(ByteDigest);

impl ModelDigest {
    /// Parse the canonical SHA-256 spelling selected by source.
    pub fn parse(value: &str) -> Result<Self, PackageError> {
        value
            .parse()
            .map(Self)
            .map_err(|_| PackageError::InvalidModelDigest(value.into()))
    }

    /// Canonical selected value.
    pub fn digest(self) -> ByteDigest {
        self.0
    }
}

/// Exact compiled-model identity/version/raw-byte-digest triple.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ModelRef {
    identity: String,
    version: String,
    digest: ModelDigest,
}

impl ModelRef {
    pub fn new(
        identity: impl Into<String>,
        version: impl Into<String>,
        digest: ModelDigest,
    ) -> Result<Self, PackageError> {
        let (identity, version) = (identity.into(), version.into());
        Self::validate_components(&identity, &version).map_err(|component| match component {
            InvalidModelComponent::Identity => PackageError::InvalidModelIdentity,
            InvalidModelComponent::Version => PackageError::InvalidModelVersion,
        })?;
        Ok(Self::from_validated(identity, version, digest))
    }

    pub(crate) fn validate_components(
        identity: &str,
        version: &str,
    ) -> Result<(), InvalidModelComponent> {
        DefinitionRef::validate_components(identity, version).map_err(|component| match component {
            InvalidDefinitionComponent::Identity => InvalidModelComponent::Identity,
            InvalidDefinitionComponent::Version => InvalidModelComponent::Version,
        })
    }

    pub(crate) fn from_validated(identity: String, version: String, digest: ModelDigest) -> Self {
        Self {
            identity,
            version,
            digest,
        }
    }

    pub fn identity(&self) -> &str {
        &self.identity
    }

    pub fn version(&self) -> &str {
        &self.version
    }

    pub fn digest(&self) -> ModelDigest {
        self.digest
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum InvalidModelComponent {
    Identity,
    Version,
}

/// Source-located profile definition selection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProfileSelection {
    /// Local alias used by declarations.
    pub alias: String,
    /// Exact definition triple.
    pub definition: DefinitionRef,
    /// Full profile declaration range.
    pub span: Span,
    /// Exact identity literal range used for located refusals.
    pub identity_span: Span,
}

/// Source-located import definition selection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ImportSelection {
    /// Optional local alias.
    pub alias: Option<String>,
    /// Exact definition triple.
    pub definition: DefinitionRef,
    /// Full import declaration range.
    pub span: Span,
}

/// Source-located formal model selection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelSelection {
    /// Local model alias.
    pub alias: String,
    /// Exact compiled-model document selection.
    pub model: ModelRef,
    /// Full model declaration range.
    pub span: Span,
}

/// Exact compiled-model artifact admitted by its owning model reader.
#[derive(Clone, Debug)]
pub struct ModelArtifact {
    exact: ModelRef,
    exact_bytes: Box<[u8]>,
}

impl ModelArtifact {
    pub fn from_exact_bytes(
        _authority: &ReaderAuthority,
        identity: impl Into<String>,
        version: impl Into<String>,
        exact_bytes: &[u8],
    ) -> Result<Self, PackageError> {
        if exact_bytes.is_empty() || exact_bytes.len() > crate::source::MAX_SOURCE_BYTES {
            return Err(PackageError::InvalidModelArtifactBytes);
        }
        let exact = ModelRef::new(identity, version, ModelDigest(ByteDigest::of(exact_bytes)))?;
        Ok(Self {
            exact,
            exact_bytes: exact_bytes.into(),
        })
    }

    pub fn exact(&self) -> &ModelRef {
        &self.exact
    }

    pub fn exact_bytes(&self) -> &[u8] {
        &self.exact_bytes
    }
}

/// Exact package-relevant selections recovered from the admitted syntax.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct SourceSelections {
    /// Selected profile definitions in source order.
    pub profiles: Vec<ProfileSelection>,
    /// Selected package imports in source order.
    pub imports: Vec<ImportSelection>,
    /// Selected model exports in source order.
    pub models: Vec<ModelSelection>,
}

/// Role supplied by one exact profile definition.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum DefinitionRole {
    Source,
    ValueModelExpression,
    Temporal,
    Observation,
    Protocol,
    Runtime,
    MethodPlan,
    Backend,
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

/// Explicit package-graph accounting limits, independently bounded by hard
/// implementation ceilings.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PackageLimits {
    pub definitions: usize,
    pub dependency_edges: usize,
    pub depth: usize,
    pub artifact_bytes: usize,
}

impl Default for PackageLimits {
    fn default() -> Self {
        Self {
            definitions: 4_096,
            dependency_edges: 16_384,
            depth: 256,
            artifact_bytes: 16 * crate::source::MAX_SOURCE_BYTES,
        }
    }
}

impl PackageLimits {
    fn bounded(self) -> Self {
        let hard = Self::default();
        Self {
            definitions: self.definitions.min(hard.definitions),
            dependency_edges: self.dependency_edges.min(hard.dependency_edges),
            depth: self.depth.min(hard.depth),
            artifact_bytes: self.artifact_bytes.min(hard.artifact_bytes),
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
        if exact_bytes.is_empty() || exact_bytes.len() > crate::source::MAX_SOURCE_BYTES {
            return Err(PackageError::InvalidDefinitionArtifactBytes);
        }
        let exact = DefinitionRef::new(
            identity,
            version,
            DefinitionDigest(ByteDigest::of(exact_bytes)),
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

    /// Build a catalog under explicit lower accounting limits.
    pub fn with_limits(
        definitions: Vec<Definition>,
        limits: PackageLimits,
    ) -> Result<Self, PackageError> {
        let limits = limits.bounded();
        if definitions.len() > limits.definitions {
            return Err(PackageError::ResourceLimit);
        }
        let mut edges = 0_usize;
        let mut bytes = 0_usize;
        let mut catalog = Self::default();
        for definition in definitions {
            edges = edges
                .checked_add(definition.dependencies.len())
                .ok_or(PackageError::ResourceLimit)?;
            bytes = bytes
                .checked_add(definition.exact_bytes.len())
                .ok_or(PackageError::ResourceLimit)?;
            if edges > limits.dependency_edges || bytes > limits.artifact_bytes {
                return Err(PackageError::ResourceLimit);
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
            .any(|definition| definition.identity == identity)
    }
}

/// Exact profile-selection inventory used by public syntax and editor APIs.
///
/// This catalog carries only immutable source-selected references. It grants no
/// semantic-definition, capability, checked-package, model-reader, or runtime
/// authority.
#[derive(Clone, Debug, Default)]
pub struct ProfileCatalog {
    profiles: BTreeSet<DefinitionRef>,
}

impl ProfileCatalog {
    /// Build a bounded exact profile inventory.
    pub fn new(profiles: Vec<DefinitionRef>) -> Result<Self, PackageError> {
        let limits = PackageLimits::default();
        if profiles.len() > limits.definitions {
            return Err(PackageError::ResourceLimit);
        }
        let mut catalog = Self::default();
        for profile in profiles {
            if !catalog.profiles.insert(profile) {
                return Err(PackageError::DuplicateDefinition);
            }
        }
        Ok(catalog)
    }

    pub(crate) fn profile_status(&self, selected: &DefinitionRef) -> ProfileStatus {
        if self.profiles.contains(selected) {
            ProfileStatus::Exact
        } else if self.profiles.iter().any(|profile| {
            profile.identity == selected.identity && profile.version == selected.version
        }) {
            ProfileStatus::Stale(StaleProfile::ByteDigest)
        } else if self
            .profiles
            .iter()
            .any(|profile| profile.identity == selected.identity)
        {
            ProfileStatus::Stale(StaleProfile::Revision)
        } else {
            ProfileStatus::Unknown
        }
    }
}

/// Exact compiled-model inventory, separate from semantic definitions.
#[derive(Clone, Debug, Default)]
pub struct ModelCatalog {
    models: BTreeMap<ModelRef, Arc<ModelArtifact>>,
}

impl ModelCatalog {
    pub fn new(models: Vec<ModelArtifact>) -> Result<Self, PackageError> {
        Self::with_limits(models, PackageLimits::default())
    }

    pub fn with_limits(
        models: Vec<ModelArtifact>,
        limits: PackageLimits,
    ) -> Result<Self, PackageError> {
        let limits = limits.bounded();
        if models.len() > limits.definitions {
            return Err(PackageError::ResourceLimit);
        }
        let mut bytes = 0_usize;
        let mut catalog = Self::default();
        for model in models {
            bytes = bytes
                .checked_add(model.exact_bytes.len())
                .ok_or(PackageError::ResourceLimit)?;
            if bytes > limits.artifact_bytes {
                return Err(PackageError::ResourceLimit);
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ProfileStatus {
    Exact,
    Stale(StaleProfile),
    Unknown,
}

/// How a known profile identity differs from its selection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum StaleProfile {
    /// No known profile has the selected version.
    Revision,
    /// A known profile has the selected version with another digest.
    ByteDigest,
}

/// Original source authority retained through package graph resolution/lowering.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceAuthority {
    pub identity: SourceIdentity,
    pub digest: SourceDigest,
    pub path: String,
}

/// Syntax/package-graph checked result; not the later type-checked package.
#[derive(Clone, Debug)]
pub struct ResolvedSourcePackage {
    parsed: Arc<super::ParsedSource>,
    authority: SourceAuthority,
    bundle: CompleteBundle,
    definitions: BTreeMap<DefinitionRef, Arc<Definition>>,
    models: BTreeMap<ModelRef, Arc<ModelArtifact>>,
}

impl ResolvedSourcePackage {
    pub fn parsed(&self) -> &Arc<super::ParsedSource> {
        &self.parsed
    }
    pub fn authority(&self) -> &SourceAuthority {
        &self.authority
    }
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
}

/// One declaration occurrence in the identity-preserving source graph.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LoweredDeclaration {
    pub production: super::Production,
    pub span: Span,
}

/// Structural lowering established by #117, before semantic checking.
#[derive(Clone, Debug)]
pub struct LoweredSourceGraph {
    authority: SourceAuthority,
    bundle_identity: SemanticDigest,
    declarations: Vec<LoweredDeclaration>,
}

impl LoweredSourceGraph {
    pub fn authority(&self) -> &SourceAuthority {
        &self.authority
    }
    pub fn bundle_identity(&self) -> SemanticDigest {
        self.bundle_identity
    }
    pub fn declarations(&self) -> &[LoweredDeclaration] {
        &self.declarations
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
/// use quire_spec_language::complete::{SemanticDigest, SourceDigest};
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
    pub code: super::CompleteCode,
    /// Exact source identity, path and raw-byte digest being refused.
    pub authority: Box<SourceAuthority>,
    /// Exact selecting source range.
    pub span: Span,
    /// Typed corrective cause.
    pub cause: PackageError,
}

/// Resolve exact source selections into a dependency-closed complete bundle.
pub fn resolve_source_package(
    parsed: Arc<super::ParsedSource>,
    catalog: &DefinitionCatalog,
    models: &ModelCatalog,
    limits: PackageLimits,
) -> Result<ResolvedSourcePackage, PackageRefusal> {
    let limits = limits.bounded();
    let refusal = |code, span, cause| refusal(parsed.as_ref(), code, span, cause);
    if !parsed.is_admissible() {
        return Err(refusal(
            super::CompleteCode::InvalidSyntax,
            Span {
                start: 0,
                end: parsed.source().text().len(),
            },
            PackageError::InvalidSource,
        ));
    }
    let selections = parsed.selections();
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
                            .unwrap_or(&selection.definition.identity),
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
                    super::CompleteCode::AmbiguousDeclaration,
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
    let roots: Vec<_> = selections
        .profiles
        .iter()
        .map(|selection| (&selection.definition, selection.identity_span, true))
        .chain(
            selections
                .imports
                .iter()
                .map(|selection| (&selection.definition, selection.span, false)),
        )
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
                    super::CompleteCode::InvalidModelBinding,
                    selection.span,
                    PackageError::ConflictingModels(Box::new(ModelConflict {
                        first: previous.clone(),
                        first_span: previous_span,
                        second: selected.clone(),
                    })),
                ));
            }
        }
        let Some(model) = models.exact(selected) else {
            let cause = if models.contains_identity(selected.identity()) {
                PackageError::StaleModel(selected.clone())
            } else {
                PackageError::MissingModel(selected.clone())
            };
            return Err(refusal(
                super::CompleteCode::InvalidModelBinding,
                selection.span,
                cause,
            ));
        };
        resolved_models.insert(selected.clone(), model.clone());
    }
    if roots
        .len()
        .checked_add(resolved_models.len())
        .is_none_or(|count| count > limits.definitions)
    {
        return Err(refusal(
            super::CompleteCode::ResourceExhausted,
            Span { start: 0, end: 0 },
            PackageError::ResourceLimit,
        ));
    }

    let mut logical = BTreeMap::<&str, (&DefinitionRef, Span)>::new();
    for (selected, span, _) in &roots {
        if let Some((previous, previous_span)) =
            logical.insert(&selected.identity, (selected, *span))
        {
            if previous != *selected {
                return Err(refusal(
                    super::CompleteCode::AmbiguousDeclaration,
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

    for (selected, span, profile) in &roots {
        if catalog.exact(selected).is_none() {
            let (code, cause) = if catalog.contains_identity(&selected.identity) {
                (
                    super::CompleteCode::StaleDependency,
                    PackageError::StaleDefinition((*selected).clone()),
                )
            } else if *profile {
                (
                    super::CompleteCode::UnknownProfile,
                    PackageError::MissingDefinition((*selected).clone()),
                )
            } else {
                (
                    super::CompleteCode::MissingImport,
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
    for (root, span, _) in roots {
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
                        super::CompleteCode::ResourceExhausted,
                        span,
                        PackageError::ResourceLimit,
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
                    super::CompleteCode::InvalidPackage,
                    span,
                    PackageError::DefinitionCycle(
                        active[start..].iter().cloned().chain([selected]).collect(),
                    ),
                ));
            }
            let Some(definition) = catalog.exact(&selected) else {
                return Err(refusal(
                    super::CompleteCode::MissingImport,
                    span,
                    PackageError::MissingDefinition(selected),
                ));
            };
            if active.len() >= limits.depth {
                return Err(refusal(
                    super::CompleteCode::ResourceExhausted,
                    span,
                    PackageError::ResourceLimit,
                ));
            }
            traversed_edges = traversed_edges
                .checked_add(definition.dependencies.len())
                .ok_or_else(|| {
                    refusal(
                        super::CompleteCode::ResourceExhausted,
                        span,
                        PackageError::ResourceLimit,
                    )
                })?;
            if traversed_edges > limits.dependency_edges {
                return Err(refusal(
                    super::CompleteCode::ResourceExhausted,
                    span,
                    PackageError::ResourceLimit,
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
            closed_logical.insert(&selected.identity, (selected, selected_span))
        {
            if previous != selected {
                return Err(refusal(
                    super::CompleteCode::AmbiguousDeclaration,
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
            super::CompleteCode::ResourceExhausted,
            artifact_span,
            PackageError::ResourceLimit,
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
            super::CompleteCode::UnknownRequiredFeature,
            selections
                .profiles
                .first()
                .map_or(Span { start: 0, end: 0 }, |selection| selection.span),
            PackageError::MissingCapability(missing.clone()),
        ));
    }
    let mut bundle = link_complete_bundle(facets).map_err(|cause| {
        identity_refusal(
            parsed.as_ref(),
            selections
                .profiles
                .first()
                .map_or(Span { start: 0, end: 0 }, |selection| selection.span),
            cause,
        )
    })?;
    bundle.capabilities = capabilities;
    let mut identity = SemanticIdentityBuilder::new(b"quire.complete.resolved-graph/1\0");
    for definition in resolved.values() {
        let digest = definition.exact.digest.digest().to_string();
        for (name, value) in [
            ("definition-identity", definition.exact.identity.as_bytes()),
            ("definition-version", definition.exact.version.as_bytes()),
            ("definition-digest", digest.as_bytes()),
            (
                "definition-role",
                definition.role.identity_name().as_bytes(),
            ),
        ] {
            identity.field(name, value).map_err(|cause| {
                identity_refusal(parsed.as_ref(), Span { start: 0, end: 0 }, cause)
            })?;
        }
        for dependency in &definition.dependencies {
            let digest = dependency.digest.digest().to_string();
            for (name, value) in [
                ("dependency-identity", dependency.identity.as_bytes()),
                ("dependency-version", dependency.version.as_bytes()),
                ("dependency-digest", digest.as_bytes()),
            ] {
                identity.field(name, value).map_err(|cause| {
                    identity_refusal(parsed.as_ref(), Span { start: 0, end: 0 }, cause)
                })?;
            }
        }
        for capability in &definition.capabilities {
            identity
                .field("definition-capability", capability.as_str().as_bytes())
                .map_err(|cause| {
                    identity_refusal(parsed.as_ref(), Span { start: 0, end: 0 }, cause)
                })?;
        }
    }
    for model in resolved_models.values() {
        let digest = model.exact.digest.digest().to_string();
        for (name, value) in [
            ("model-identity", model.exact.identity.as_bytes()),
            ("model-version", model.exact.version.as_bytes()),
            ("model-digest", digest.as_bytes()),
        ] {
            identity.field(name, value).map_err(|cause| {
                identity_refusal(parsed.as_ref(), Span { start: 0, end: 0 }, cause)
            })?;
        }
    }
    for capability in &bundle.capabilities {
        identity
            .field("capability", capability.as_str().as_bytes())
            .map_err(|cause| identity_refusal(parsed.as_ref(), Span { start: 0, end: 0 }, cause))?;
    }
    bundle.identity = identity
        .finish()
        .map_err(|cause| identity_refusal(parsed.as_ref(), Span { start: 0, end: 0 }, cause))?;
    Ok(ResolvedSourcePackage {
        authority: source_authority(parsed.as_ref()),
        parsed,
        bundle,
        definitions: resolved,
        models: resolved_models,
    })
}

fn source_authority(parsed: &super::ParsedSource) -> SourceAuthority {
    SourceAuthority {
        identity: parsed.source().identity().clone(),
        digest: SourceDigest(parsed.source().digest()),
        path: parsed.source().path().into(),
    }
}

fn refusal(
    parsed: &super::ParsedSource,
    code: super::CompleteCode,
    span: Span,
    cause: PackageError,
) -> PackageRefusal {
    PackageRefusal {
        code,
        authority: Box::new(source_authority(parsed)),
        span,
        cause,
    }
}

fn identity_refusal(
    parsed: &super::ParsedSource,
    span: Span,
    cause: PackageError,
) -> PackageRefusal {
    let code = match &cause {
        PackageError::CanonicalSize | PackageError::ResourceLimit => {
            super::CompleteCode::ResourceExhausted
        }
        _ => super::CompleteCode::InvalidPackage,
    };
    refusal(parsed, code, span, cause)
}

/// Lower the resolved syntax/package graph while preserving source authority.
pub fn lower_source_graph(package: &ResolvedSourcePackage) -> LoweredSourceGraph {
    let declarations = package
        .parsed
        .cst()
        .root()
        .children()
        .iter()
        .filter_map(|element| match element {
            super::CstElement::Node(index) => package.parsed.cst().nodes().get(*index),
            super::CstElement::Token(_) => None,
        })
        .filter(|node| node.production() == super::Production::Declaration)
        .filter_map(|wrapper| {
            wrapper.children().iter().find_map(|element| match element {
                super::CstElement::Node(index) => {
                    package
                        .parsed
                        .cst()
                        .nodes()
                        .get(*index)
                        .map(|node| LoweredDeclaration {
                            production: node.production(),
                            span: node.span(),
                        })
                }
                super::CstElement::Token(_) => None,
            })
        })
        .collect();
    LoweredSourceGraph {
        authority: package.authority.clone(),
        bundle_identity: package.bundle.identity(),
        declarations,
    }
}

fn encode_set<'a>(values: impl Iterator<Item = &'a str>) -> Result<Vec<u8>, PackageError> {
    let mut output = Vec::new();
    for value in values {
        field(&mut output, "item", value.as_bytes())?;
    }
    Ok(output)
}

fn field(output: &mut Vec<u8>, name: &str, value: &[u8]) -> Result<(), PackageError> {
    let name_len = u64::try_from(name.len()).map_err(|_| PackageError::CanonicalSize)?;
    let value_len = u64::try_from(value.len()).map_err(|_| PackageError::CanonicalSize)?;
    let added = 16_usize
        .checked_add(name.len())
        .and_then(|size| size.checked_add(value.len()))
        .ok_or(PackageError::ResourceLimit)?;
    if output
        .len()
        .checked_add(added)
        .is_none_or(|size| size > crate::source::MAX_SOURCE_BYTES)
    {
        return Err(PackageError::ResourceLimit);
    }
    output.extend_from_slice(&name_len.to_be_bytes());
    output.extend_from_slice(name.as_bytes());
    output.extend_from_slice(&value_len.to_be_bytes());
    output.extend_from_slice(value);
    Ok(())
}

struct SemanticIdentityBuilder(Sha256);

impl SemanticIdentityBuilder {
    fn new(domain: &[u8]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(b"SemanticDigest\0");
        hasher.update(domain);
        Self(hasher)
    }

    fn field(&mut self, name: &str, value: &[u8]) -> Result<(), PackageError> {
        let name_len = u64::try_from(name.len()).map_err(|_| PackageError::CanonicalSize)?;
        let value_len = u64::try_from(value.len()).map_err(|_| PackageError::CanonicalSize)?;
        self.0.update(name_len.to_be_bytes());
        self.0.update(name.as_bytes());
        self.0.update(value_len.to_be_bytes());
        self.0.update(value);
        Ok(())
    }

    fn finish(self) -> Result<SemanticDigest, PackageError> {
        let hex = format!("{:x}", self.0.finalize());
        ByteDigest::from_hex(&hex)
            .map(SemanticDigest)
            .map_err(|_| PackageError::CanonicalSize)
    }
}

/// Typed complete-package refusal.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum PackageError {
    /// Source was not syntactically admitted.
    #[error("source is not admissible")]
    InvalidSource,
    /// Definition identity was empty or outside its bound.
    #[error("invalid definition identity")]
    InvalidDefinitionIdentity,
    /// Definition version was empty or outside its bound.
    #[error("invalid definition version")]
    InvalidDefinitionVersion,
    /// Definition digest was not canonical SHA-256.
    #[error("invalid definition digest {0}")]
    InvalidDefinitionDigest(String),
    /// Supplied definition artifact bytes were empty or exceeded the hard ceiling.
    #[error("invalid exact definition artifact bytes")]
    InvalidDefinitionArtifactBytes,
    /// Compiled-model identity was empty or outside its bound.
    #[error("invalid compiled-model identity")]
    InvalidModelIdentity,
    /// Compiled-model version was empty or outside its bound.
    #[error("invalid compiled-model version")]
    InvalidModelVersion,
    /// Compiled-model digest was not canonical SHA-256.
    #[error("invalid compiled-model digest {0}")]
    InvalidModelDigest(String),
    /// Supplied compiled-model document bytes were empty or too large.
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
    MissingModel(ModelRef),
    /// A known compiled model was selected with stale version/digest.
    #[error("stale compiled-model artifact {0:?}")]
    StaleModel(ModelRef),
    /// Two source model selections conflict on one logical identity.
    #[error("conflicting compiled-model selections")]
    ConflictingModels(Box<ModelConflict>),
    /// Two source selections conflict on one logical identity.
    #[error("conflicting definition selections {0:?}")]
    ConflictingDefinitions(Box<DefinitionConflict>),
    /// One source namespace reused a local alias.
    #[error("duplicate {namespace} alias {alias}; first declared at {first_span:?}")]
    DuplicateAlias {
        namespace: String,
        alias: String,
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
    /// Package input exceeds the closed implementation ceiling.
    #[error("complete package resource limit exceeded")]
    ResourceLimit,
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
