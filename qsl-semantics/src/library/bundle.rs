// SPDX-License-Identifier: AGPL-3.0-or-later
//! Complete-V1 definition bundle linking (FR-111; ADR-011 §6.2, layer-3
//! `library`).
//!
//! [`link_bundle`] closes root definition selections over a caller-supplied
//! [`DefinitionCatalog`] into a dependency-closed [`CompleteBundle`], or
//! refuses with a catalogued code and cause. It computes a bundle only: no
//! stage of the S1 to S4 spine calls it, it builds no checked package and it
//! names no `check`, `package`, `model`, emitter or backend type
//! (FR-111-CON-1). A source's header profile selections resolve at E3
//! (FR-110), its `import` selections in the spine (FR-099) and its `model`
//! selections at I1 (FR-056); none of them reaches this module.
use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;
use std::sync::Arc;

use qsl_foundation::selection::{
    DefinitionDigest, DefinitionRef, InvalidDefinitionComponent, MAX_SELECTED_DEFINITIONS,
};
use qsl_foundation::{ByteDigest, Code};

use crate::value::semantic_node::IDENTITY_LIMITS as LIMITS;

/// Unforgeable crate-issued proof that typed definition parts came from
/// the owning reader boundary.
///
/// There is deliberately no production constructor. Task-054 will connect the
/// concrete checked reader; raw source/editor callers cannot mint this proof.
#[derive(Debug)]
pub struct ReaderAuthority {
    _private: (),
}

impl ReaderAuthority {
    /// Test-only fixture authority (`test-support`), so the tests that build
    /// definitions (`library::bundle_tests`) can. It forges the capability
    /// this type exists to withhold, so
    /// `test-support` must never be enabled by a shipped dependent: only a
    /// `[dev-dependencies]` entry may turn it on, which
    /// `no_shipped_dependency_enables_test_support` checks.
    #[cfg(any(test, feature = "test-support"))]
    pub const fn fixture() -> Self {
        Self { _private: () }
    }
}

// `DefinitionDigest` and `DefinitionRef` are layer-F values
// (`qsl_foundation::selection`, QSL-181): this module maps their validation
// errors onto `PackageError`.
impl From<InvalidDefinitionComponent> for PackageError {
    fn from(component: InvalidDefinitionComponent) -> Self {
        match component {
            InvalidDefinitionComponent::Identity => Self::InvalidDefinitionIdentity,
            InvalidDefinitionComponent::Version => Self::InvalidDefinitionVersion,
        }
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
    /// Maximum number of exact definitions admitted into
    /// one resolved package graph or catalog.
    pub definitions: usize,
    /// Maximum number of dependency edges traversed while resolving the
    /// package graph.
    pub dependency_edges: usize,
    /// Maximum dependency-chain depth carried on the active resolution stack
    /// before resolution is refused.
    pub depth: usize,
    /// Maximum total artifact byte count admitted
    /// into one resolved package or catalog.
    pub artifact_bytes: usize,
    /// Maximum byte count of any one definition
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

#[qsl_attrs::string_edge]
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

/// The linked complete-V1 semantic identity: SHA-256 over the
/// `quire.complete.resolved-graph/2` domain label and the RFC 8785 bytes of
/// the bundle's identity preimage (FR-111).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SemanticDigest(ByteDigest);

impl SemanticDigest {
    /// Borrow the underlying SHA-256 value after domain selection.
    pub fn digest(self) -> ByteDigest {
        self.0
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

/// The result of [`link_bundle`]: the closed definitions, the complete bundle
/// they form and the limits the link was checked against.
#[derive(Clone, Debug)]
pub struct LinkedBundle {
    bundle: CompleteBundle,
    definitions: BTreeMap<DefinitionRef, Arc<Definition>>,
    limits: PackageLimits,
}

impl LinkedBundle {
    /// The dependency-closed complete bundle.
    pub fn bundle(&self) -> &CompleteBundle {
        &self.bundle
    }

    /// Every closed definition, by exact reference.
    pub fn definitions(&self) -> &BTreeMap<DefinitionRef, Arc<Definition>> {
        &self.definitions
    }

    /// The [`PackageLimits`] this link was checked against, exactly as the
    /// caller supplied them: every field is enforced as given, so a
    /// caller-raised ceiling is visible with the result it produced.
    pub fn limits(&self) -> PackageLimits {
        self.limits
    }
}

/// A [`link_bundle`] refusal with a stable complete producer code.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[error("{code:?} (root {root:?}): {cause}")]
pub struct BundleRefusal {
    /// Stable producer classification.
    pub code: Code,
    /// Closed catalogued cause tag, admitted by the catalog for `code`.
    pub cause_tag: ResolutionCause,
    /// Typed corrective cause.
    pub cause: PackageError,
    /// Index into `roots` of the root whose closure refused; `None` for a
    /// refusal of the whole link (a ceiling on the root list, the closed
    /// definitions' bytes, the bundle's capabilities or facets, or its
    /// identity).
    pub root: Option<usize>,
}

/// The closed catalogued cause tag of a bundle refusal, under
/// `quire.native.diagnostics/v1` revision `1-draft.7`. Layer 3's own subset of
/// the cause vocabulary: the parser's `qsl_cst::CompleteCause` is layer 1's,
/// and linking never sees a syntax cause.
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
    /// `ambiguous_declaration`: one definition identity is selected at two
    /// distinct exact selections.
    ConflictingAuthority,
    /// `invalid_package`: a catalog holds one exact selection twice.
    DuplicateMember,
    /// `invalid_package`: a definition member is malformed.
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
            Self::ConflictingAuthority => "conflicting-authority",
            Self::DuplicateMember => "duplicate-member",
            Self::InvalidValue => "invalid-value",
            Self::DefinitionCycle => "definition-cycle",
            Self::FeatureSetMismatch => "feature-set-mismatch",
            Self::UnknownFeature => "unknown-feature",
            Self::UnsupportedFeature => "unsupported-feature",
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
            Self::ConflictingAuthority => code == Code::AmbiguousDeclaration,
            Self::DuplicateMember
            | Self::InvalidValue
            | Self::DefinitionCycle
            | Self::FeatureSetMismatch => code == Code::InvalidPackage,
            Self::UnknownFeature | Self::UnsupportedFeature => code == Code::UnknownRequiredFeature,
            Self::StageLimit(_) => code == Code::StageLimitExceeded,
        }
    }
}

/// Link exact root definition selections into a dependency-closed complete
/// bundle over `catalog` (FR-111).
///
/// The roots close over their dependency edges depth first. Every
/// [`PackageLimits`] field applies exactly as supplied. No backend, installed
/// runtime or support set is an input, so none changes the bundle's identity.
pub fn link_bundle(
    roots: &[DefinitionRef],
    catalog: &DefinitionCatalog,
    limits: PackageLimits,
) -> Result<LinkedBundle, BundleRefusal> {
    if roots.len() > limits.definitions {
        return Err(refusal(
            Code::ResourceExhausted,
            None,
            PackageError::ResourceLimit {
                kind: PackageLimitKind::Definitions,
                limit: limits.definitions,
                actual: None,
            },
        ));
    }

    let mut logical = BTreeMap::<&str, (&DefinitionRef, usize)>::new();
    for (index, selected) in roots.iter().enumerate() {
        if let Some((previous, previous_root)) =
            logical.insert(selected.identity(), (selected, index))
        {
            if previous != selected {
                return Err(conflict(previous, previous_root, selected, index));
            }
        }
    }

    for (index, selected) in roots.iter().enumerate() {
        if catalog.exact(selected).is_none() {
            let (code, cause_tag, cause) = if catalog.contains_identity(selected.identity()) {
                let tag = if catalog.contains_version(selected.identity(), selected.version()) {
                    ResolutionCause::ByteDigestMismatch
                } else {
                    ResolutionCause::RevisionMismatch
                };
                (
                    Code::StaleDependency,
                    tag,
                    PackageError::StaleDefinition(selected.clone()),
                )
            } else {
                (
                    Code::UnknownProfile,
                    ResolutionCause::UnsupportedSelection,
                    PackageError::MissingDefinition(selected.clone()),
                )
            };
            return Err(BundleRefusal {
                code,
                cause_tag,
                cause,
                root: Some(index),
            });
        }
    }

    let mut resolved = BTreeMap::new();
    let mut provenance = BTreeMap::<DefinitionRef, usize>::new();
    let mut provenance_order = BTreeMap::<DefinitionRef, usize>::new();
    let mut next_provenance_order = 0_usize;
    let mut active = Vec::new();
    let mut done = BTreeSet::new();
    let mut traversed_edges = 0_usize;
    for (index, root) in roots.iter().enumerate() {
        let root_index = Some(index);
        let mut work = vec![(root.clone(), false)];
        while let Some((selected, expanded)) = work.pop() {
            if done.contains(&selected) {
                continue;
            }
            let Some(definition) = catalog.exact(&selected) else {
                return Err(refusal(
                    Code::MissingImport,
                    root_index,
                    PackageError::MissingDefinition(selected),
                ));
            };
            if expanded {
                active.pop();
                done.insert(selected.clone());
                if resolved.len() >= limits.definitions {
                    return Err(refusal(
                        Code::ResourceExhausted,
                        root_index,
                        PackageError::ResourceLimit {
                            kind: PackageLimitKind::Definitions,
                            limit: limits.definitions,
                            actual: None,
                        },
                    ));
                }
                resolved.insert(selected, definition.clone());
                continue;
            }
            if let std::collections::btree_map::Entry::Vacant(entry) =
                provenance.entry(selected.clone())
            {
                entry.insert(index);
                provenance_order.insert(selected.clone(), next_provenance_order);
                next_provenance_order = next_provenance_order.saturating_add(1);
            }
            if let Some(start) = active.iter().position(|definition| definition == &selected) {
                return Err(refusal(
                    Code::InvalidPackage,
                    root_index,
                    PackageError::DefinitionCycle(
                        active[start..].iter().cloned().chain([selected]).collect(),
                    ),
                ));
            }
            if active.len() >= limits.depth {
                return Err(refusal(
                    // QSL-236: the graph's dependency-chain depth ceiling
                    // maps onto `stage_limit_exceeded`/`nesting-depth-exceeded`
                    // (`cause_tag` picks the tag from the `PackageError`).
                    Code::StageLimitExceeded,
                    root_index,
                    PackageError::ResourceLimit {
                        kind: PackageLimitKind::Depth,
                        limit: limits.depth,
                        actual: Some(active.len() + 1),
                    },
                ));
            }
            let edge_limit = || {
                refusal(
                    Code::ResourceExhausted,
                    root_index,
                    PackageError::ResourceLimit {
                        kind: PackageLimitKind::DependencyEdges,
                        limit: limits.dependency_edges,
                        actual: None,
                    },
                )
            };
            traversed_edges = traversed_edges
                .checked_add(definition.dependencies.len())
                .ok_or_else(edge_limit)?;
            if traversed_edges > limits.dependency_edges {
                return Err(edge_limit());
            }
            active.push(selected.clone());
            work.push((selected, true));
            for dependency in definition.dependencies.iter().rev() {
                work.push((dependency.clone(), false));
            }
        }
    }

    let mut closed_logical = BTreeMap::<&str, (&DefinitionRef, usize)>::new();
    let mut discovered: Vec<_> = resolved.keys().collect();
    discovered.sort_by_key(|selected| provenance_order[*selected]);
    for selected in discovered {
        let selected_root = provenance[selected];
        if let Some((previous, previous_root)) =
            closed_logical.insert(selected.identity(), (selected, selected_root))
        {
            if previous != selected {
                return Err(conflict(previous, previous_root, selected, selected_root));
            }
        }
    }

    for len in resolved
        .values()
        .map(|definition| definition.exact_bytes.len())
    {
        limits
            .check_single_artifact(len)
            .map_err(|cause| refusal(Code::ResourceExhausted, None, cause))?;
    }
    let resolved_artifact_bytes = resolved.values().try_fold(0_usize, |total, definition| {
        total.checked_add(definition.exact_bytes.len())
    });
    if resolved_artifact_bytes.is_none_or(|bytes| bytes > limits.artifact_bytes) {
        return Err(refusal(
            Code::ResourceExhausted,
            None,
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
    let inventory: BTreeSet<_> = CapabilityId::complete_inventory().into_iter().collect();
    if let Some(missing) = inventory.difference(&capabilities).next() {
        return Err(refusal(
            Code::UnknownRequiredFeature,
            None,
            PackageError::MissingCapability(missing.clone()),
        ));
    }
    if let Some(missing) = Facet::all().iter().find(|facet| !facets.contains(facet)) {
        return Err(refusal(
            Code::InvalidPackage,
            None,
            PackageError::MissingFacet(*missing),
        ));
    }
    let identity = resolved_graph_identity(&resolved, &capabilities).map_err(|cause| {
        let code = match &cause {
            PackageError::CanonicalSize | PackageError::ResourceLimit { .. } => {
                Code::ResourceExhausted
            }
            _ => Code::InvalidPackage,
        };
        refusal(code, None, cause)
    })?;
    Ok(LinkedBundle {
        bundle: CompleteBundle {
            facets,
            capabilities,
            identity,
        },
        definitions: resolved,
        limits,
    })
}

fn refusal(code: Code, root: Option<usize>, cause: PackageError) -> BundleRefusal {
    BundleRefusal {
        code,
        cause_tag: cause_tag(code, &cause),
        cause,
        root,
    }
}

/// Two selections of one identity that differ: `second` (at root `second_root`)
/// contradicts `first` (at root `first_root`).
fn conflict(
    first: &DefinitionRef,
    first_root: usize,
    second: &DefinitionRef,
    second_root: usize,
) -> BundleRefusal {
    refusal(
        Code::AmbiguousDeclaration,
        Some(second_root),
        PackageError::ConflictingDefinitions(Box::new(DefinitionConflict {
            first: first.clone(),
            first_root,
            second: second.clone(),
        })),
    )
}

/// The catalogued cause tag of a bundle refusal. A stale root is tagged at its
/// call site, which knows whether the version or only the digest differs.
fn cause_tag(code: Code, cause: &PackageError) -> ResolutionCause {
    use ResolutionCause as Tag;
    match cause {
        PackageError::InvalidDefinitionIdentity
        | PackageError::InvalidDefinitionVersion
        | PackageError::InvalidDefinitionArtifactBytes => Tag::InvalidValue,
        PackageError::DuplicateDefinition => Tag::DuplicateMember,
        PackageError::MissingDefinition(_) if code == Code::UnknownProfile => {
            Tag::UnsupportedSelection
        }
        PackageError::MissingDefinition(_) => Tag::MissingSelection,
        PackageError::StaleDefinition(_) => Tag::RevisionMismatch,
        PackageError::ConflictingDefinitions(_) => Tag::ConflictingAuthority,
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

/// The digest-domain label of a bundle's identity.
const RESOLVED_GRAPH_DOMAIN: &[u8] = b"quire.complete.resolved-graph/2";

/// A bundle's identity preimage: every closed definition (its exact
/// reference, role, dependencies and capabilities) in key order, `models`,
/// which is always empty because model selections resolve only through I1
/// (FR-111, FR-056), then the bundle's capabilities.
#[derive(Serialize)]
struct ResolvedGraphPreimage<'a> {
    definitions: Vec<ResolvedDefinitionPreimage<'a>>,
    models: [&'a str; 0],
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

/// The [`SemanticDigest`] of a bundle: SHA-256 over the
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
        models: [],
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
    /// artifact is admitted into a catalog or linked bundle.
    #[error("invalid exact definition artifact bytes")]
    InvalidDefinitionArtifactBytes,
    /// An exact definition appeared more than once in a catalog.
    #[error("duplicate exact definition")]
    DuplicateDefinition,
    /// A selected exact definition was absent.
    #[error("missing definition {0:?}")]
    MissingDefinition(DefinitionRef),
    /// A known logical definition was selected with stale version/digest.
    #[error("stale definition {0:?}")]
    StaleDefinition(DefinitionRef),
    /// Two selections conflict on one logical identity.
    #[error("conflicting definition selections {0:?}")]
    ConflictingDefinitions(Box<DefinitionConflict>),
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

/// Both selections of one logical-definition conflict.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DefinitionConflict {
    /// Earlier exact selection.
    pub first: DefinitionRef,
    /// Index into the link's roots of the root whose closure selected
    /// `first`.
    pub first_root: usize,
    /// Conflicting later exact selection.
    pub second: DefinitionRef,
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;
    use sha2::{Digest as _, Sha256};

    use super::*;

    /// QSL-194 golden vector for the bundle identity's named
    /// `quire-canonical` domain: SHA-256 over the label's big-endian `u64`
    /// length, the label `quire.complete.resolved-graph/2`, and the
    /// preimage's RFC 8785 text, all written out by hand.
    #[trace("TC-491", "FR-111-AC-5", "FR-131-AC-2")]
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
        let identity = resolved_graph_identity(&BTreeMap::new(), &capabilities).unwrap();
        assert_eq!(format!("{:x}", identity.digest()), DIGEST);
    }
}
