// SPDX-License-Identifier: AGPL-3.0-or-later
//! Exact definition and compiled-model selections: the identity, version and
//! digest triples a source's `profile`, `import` and `model` declarations
//! select, the source-located selection lists a parse recovers, and the exact
//! profile inventory authoring tools check a selection against.
//!
//! These are plain values. Layer 1 (`qsl-cst`) produces them from source,
//! layer 3 (`complete::package`) resolves them against definition and model
//! catalogs, and the tool modules check profiles against a
//! [`ProfileCatalog`]; none of those layers owns them, so they sit in F
//! (ADR-011 §6.1, QSL-181).

use std::collections::BTreeSet;

use crate::digest::InvalidDigest;
use crate::{ByteDigest, Span};

/// Exact versioned definition digest in the profile/import domain.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct DefinitionDigest(ByteDigest);

impl DefinitionDigest {
    /// Parse the canonical SHA-256 spelling selected by source.
    pub fn parse(value: &str) -> Result<Self, InvalidDigest> {
        value.parse().map(Self)
    }

    /// Wrap an already-computed raw-byte digest.
    pub fn from_digest(digest: ByteDigest) -> Self {
        Self(digest)
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

/// Why an identity/version pair failed [`DefinitionRef::new`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InvalidDefinitionComponent {
    /// Identity was empty or exceeded 512 bytes.
    Identity,
    /// Version was empty or exceeded 256 bytes.
    Version,
}

impl DefinitionRef {
    /// Validate a non-empty exact definition selection.
    pub fn new(
        identity: impl Into<String>,
        version: impl Into<String>,
        digest: DefinitionDigest,
    ) -> Result<Self, InvalidDefinitionComponent> {
        let (identity, version) = (identity.into(), version.into());
        Self::validate_components(&identity, &version)?;
        Ok(Self::from_validated(identity, version, digest))
    }

    /// Check an identity/version pair against [`Self::new`]'s bounds without
    /// building a reference, so a parser can locate the failing component
    /// before it reads the digest.
    pub fn validate_components(
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

    fn from_validated(identity: String, version: String, digest: DefinitionDigest) -> Self {
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

/// The digest a source `model` declaration selects its model by. The
/// prefix names the slot: `sha256:` is the SHA-256 of a compiled-model
/// artifact's raw bytes, and `sha256-jcs:` the SHA-256 of a domain
/// package's RFC 8785 document (QSL FR-056-CON-4). Neither spelling selects
/// the other kind of model.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ModelDigest {
    /// `sha256:<hex>`: a compiled-model artifact's raw bytes.
    Artifact(ByteDigest),
    /// `sha256-jcs:<hex>`: a domain package's RFC 8785 document.
    DomainPackage(ByteDigest),
}

impl ModelDigest {
    /// Parse the canonical `sha256:` or `sha256-jcs:` spelling selected by
    /// source.
    pub fn parse(value: &str) -> Result<Self, InvalidDigest> {
        match value.strip_prefix("sha256-jcs:") {
            Some(hex) => ByteDigest::from_hex(hex).map(Self::DomainPackage),
            None => value.parse().map(Self::Artifact),
        }
    }

    /// An already-computed compiled-model artifact's raw-byte digest.
    pub fn artifact(digest: ByteDigest) -> Self {
        Self::Artifact(digest)
    }

    /// The SHA-256 value, in whichever slot it selects.
    pub fn digest(self) -> ByteDigest {
        match self {
            Self::Artifact(digest) | Self::DomainPackage(digest) => digest,
        }
    }
}

impl std::fmt::Display for ModelDigest {
    /// The source spelling: `sha256:<hex>` or `sha256-jcs:<hex>`.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Artifact(digest) => write!(f, "{digest}"),
            Self::DomainPackage(digest) => write!(f, "sha256-jcs:{digest:x}"),
        }
    }
}

/// Exact compiled-model identity/version/digest triple.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ModelRef {
    identity: String,
    version: String,
    digest: ModelDigest,
}

impl ModelRef {
    /// Validate a non-empty exact compiled-model selection.
    pub fn new(
        identity: impl Into<String>,
        version: impl Into<String>,
        digest: ModelDigest,
    ) -> Result<Self, InvalidModelComponent> {
        let (identity, version) = (identity.into(), version.into());
        Self::validate_components(&identity, &version)?;
        Ok(Self::from_validated(identity, version, digest))
    }

    /// Check an identity/version pair against [`Self::new`]'s bounds without
    /// building a reference, so a parser can locate the failing component
    /// before it reads the digest.
    pub fn validate_components(identity: &str, version: &str) -> Result<(), InvalidModelComponent> {
        DefinitionRef::validate_components(identity, version).map_err(|component| match component {
            InvalidDefinitionComponent::Identity => InvalidModelComponent::Identity,
            InvalidDefinitionComponent::Version => InvalidModelComponent::Version,
        })
    }

    fn from_validated(identity: String, version: String, digest: ModelDigest) -> Self {
        Self {
            identity,
            version,
            digest,
        }
    }

    /// Opaque compiled-model identity.
    pub fn identity(&self) -> &str {
        &self.identity
    }

    /// Exact selected version.
    pub fn version(&self) -> &str {
        &self.version
    }

    /// Exact selected model digest.
    pub fn digest(&self) -> ModelDigest {
        self.digest
    }
}

/// Why an identity/version pair failed [`ModelRef::new`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InvalidModelComponent {
    /// Identity was empty or exceeded 512 bytes.
    Identity,
    /// Version was empty or exceeded 256 bytes.
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

/// The most exact references one catalog or resolved package graph admits.
/// `complete::package`'s `PackageLimits` hard ceiling and [`ProfileCatalog`]
/// share it.
pub const MAX_SELECTED_DEFINITIONS: usize = 4_096;

/// Exact profile-selection inventory used by public syntax and editor APIs.
///
/// This catalog carries only immutable source-selected references. It grants no
/// semantic-definition, capability, checked-package, model-reader, or runtime
/// authority.
#[derive(Clone, Debug, Default)]
pub struct ProfileCatalog {
    profiles: BTreeSet<DefinitionRef>,
}

/// Why [`ProfileCatalog::new`] refused its inventory.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ProfileCatalogError {
    /// More than [`MAX_SELECTED_DEFINITIONS`] profiles.
    #[error("profile catalog resource limit exceeded")]
    ResourceLimit,
    /// One exact profile appeared twice.
    #[error("duplicate exact profile")]
    DuplicateProfile,
}

impl ProfileCatalog {
    /// Build a bounded exact profile inventory.
    pub fn new(profiles: Vec<DefinitionRef>) -> Result<Self, ProfileCatalogError> {
        if profiles.len() > MAX_SELECTED_DEFINITIONS {
            return Err(ProfileCatalogError::ResourceLimit);
        }
        let mut catalog = Self::default();
        for profile in profiles {
            if !catalog.profiles.insert(profile) {
                return Err(ProfileCatalogError::DuplicateProfile);
            }
        }
        Ok(catalog)
    }

    /// How `selected` relates to this inventory: exactly known, a known
    /// identity at another version or digest, or unknown.
    pub fn profile_status(&self, selected: &DefinitionRef) -> ProfileStatus {
        if self.profiles.contains(selected) {
            ProfileStatus::Exact
        } else if self.profiles.iter().any(|profile| {
            profile.identity() == selected.identity() && profile.version() == selected.version()
        }) {
            ProfileStatus::Stale(StaleProfile::ByteDigest)
        } else if self
            .profiles
            .iter()
            .any(|profile| profile.identity() == selected.identity())
        {
            ProfileStatus::Stale(StaleProfile::Revision)
        } else {
            ProfileStatus::Unknown
        }
    }
}

/// A profile selection's standing against a [`ProfileCatalog`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProfileStatus {
    /// The exact selection is known.
    Exact,
    /// The identity is known, but not at this selection.
    Stale(StaleProfile),
    /// No known profile has this identity.
    Unknown,
}

/// How a known profile identity differs from its selection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StaleProfile {
    /// No known profile has the selected version.
    Revision,
    /// A known profile has the selected version with another digest.
    ByteDigest,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile(index: usize) -> DefinitionRef {
        DefinitionRef::new(
            format!("acme.profile.{index}"),
            "1",
            DefinitionDigest::from_digest(ByteDigest::of(&index.to_be_bytes())),
        )
        .unwrap()
    }

    /// TC-180 (FR-131-AC-2): the profile inventory admits exactly
    /// `MAX_SELECTED_DEFINITIONS` distinct profiles, refuses one more as a
    /// resource limit, and refuses a repeated exact profile as a duplicate.
    #[test]
    fn profile_catalog_bounds_and_duplicates_are_refused_by_their_own_error() {
        let full: Vec<_> = (0..MAX_SELECTED_DEFINITIONS).map(profile).collect();
        let catalog = ProfileCatalog::new(full.clone()).unwrap();
        assert_eq!(
            catalog.profile_status(&full[MAX_SELECTED_DEFINITIONS - 1]),
            ProfileStatus::Exact
        );

        let mut over = full.clone();
        over.push(profile(MAX_SELECTED_DEFINITIONS));
        assert_eq!(
            ProfileCatalog::new(over).unwrap_err(),
            ProfileCatalogError::ResourceLimit
        );

        let duplicate = vec![profile(0), profile(1), profile(0)];
        assert_eq!(
            ProfileCatalog::new(duplicate).unwrap_err(),
            ProfileCatalogError::DuplicateProfile
        );

        // At the limit, a repeat is a duplicate, not a resource refusal.
        let mut repeated = full;
        repeated[MAX_SELECTED_DEFINITIONS - 1] = profile(0);
        assert_eq!(
            ProfileCatalog::new(repeated).unwrap_err(),
            ProfileCatalogError::DuplicateProfile
        );
    }
}
