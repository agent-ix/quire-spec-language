// SPDX-License-Identifier: AGPL-3.0-or-later
//! Definition and compiled-model selections: the definition identities a
//! source's `profile` declarations select, the identity
//! a source's `import` and `model` declarations select, the source-located
//! selection lists a parse recovers, and the profile inventory authoring
//! tools check a selection against.
//!
//! These are plain values. Layer 1 (`qsl-cst`) produces them from source,
//! the spine resolves them (E3, I1), and the tool modules check profiles against a
//! [`ProfileCatalog`]; none of those layers owns them, so they sit in F
//! (ADR-011 §6.1).

use alloc::collections::BTreeSet;
use alloc::{string::String, vec::Vec};

use crate::Span;

/// A definition selection by identity: the publishing authority and the
/// definition identity. A definition resolves by identity alone; no version
/// or byte digest is part of a selection.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct DefinitionRef {
    /// Publishing authority.
    authority: String,
    /// Opaque definition identity.
    identity: String,
}

/// Why an authority/identity pair failed [`DefinitionRef::new`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InvalidDefinitionComponent {
    /// Authority was empty or exceeded 512 bytes.
    Authority,
    /// Identity was empty or exceeded 512 bytes.
    Identity,
}

/// The most bytes a selected identity or authority holds.
pub const MAX_SELECTION_IDENTITY_BYTES: usize = 512;

/// Whether `identity` is a non-empty selection identity within
/// [`MAX_SELECTION_IDENTITY_BYTES`].
pub fn valid_selection_identity(identity: &str) -> bool {
    !identity.is_empty() && identity.len() <= MAX_SELECTION_IDENTITY_BYTES
}

impl DefinitionRef {
    /// Validate a non-empty definition selection.
    pub fn new(
        authority: impl Into<String>,
        identity: impl Into<String>,
    ) -> Result<Self, InvalidDefinitionComponent> {
        let (authority, identity) = (authority.into(), identity.into());
        if !valid_selection_identity(&authority) {
            return Err(InvalidDefinitionComponent::Authority);
        }
        if !valid_selection_identity(&identity) {
            return Err(InvalidDefinitionComponent::Identity);
        }
        Ok(Self {
            authority,
            identity,
        })
    }

    /// Publishing authority.
    pub fn authority(&self) -> &str {
        &self.authority
    }

    /// Opaque definition identity.
    pub fn identity(&self) -> &str {
        &self.identity
    }
}

/// Source-located header profile selection: `profile <alias> = "<identity>";`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProfileSelection {
    /// Local alias used by declarations.
    pub alias: String,
    /// The selected definition identity, within
    /// [`MAX_SELECTION_IDENTITY_BYTES`].
    pub identity: String,
    /// Full profile declaration range.
    pub span: Span,
    /// Exact identity literal range used for located refusals.
    pub identity_span: Span,
}

/// Source-located library import selection (ADR-015 D-2, D-3): the
/// library identity an `import` names. The library is selected by identity;
/// its content is bound by `package_id` in the lock.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ImportSelection {
    /// Optional local alias.
    pub alias: Option<String>,
    /// Library identity, within the parser's selection bound.
    pub identity: String,
    /// Full import declaration range.
    pub span: Span,
    /// Exact identity literal range used for located refusals.
    pub identity_span: Span,
}

/// Source-located formal model selection: `model <alias> = "<identity>";`.
/// The domain package is selected by its identity alone.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelSelection {
    /// Local model alias.
    pub alias: String,
    /// Domain package identity, within [`MAX_SELECTION_IDENTITY_BYTES`].
    pub identity: String,
    /// Full model declaration range.
    pub span: Span,
    /// Exact identity literal range used for located refusals.
    pub identity_span: Span,
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
/// `library::bundle`'s `PackageLimits` default ceiling and [`ProfileCatalog`]
/// share it.
pub const MAX_SELECTED_DEFINITIONS: usize = 4_096;

/// Profile-identity inventory used by public syntax and editor APIs.
///
/// This catalog carries only source-selectable profile identities. It grants
/// no semantic-definition, capability, checked-package, model-reader, or
/// runtime authority.
#[derive(Clone, Debug, Default)]
pub struct ProfileCatalog {
    profiles: BTreeSet<String>,
}

/// Why [`ProfileCatalog::new`] refused its inventory.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ProfileCatalogError {
    /// More than [`MAX_SELECTED_DEFINITIONS`] profiles.
    #[error("profile catalog resource limit exceeded")]
    ResourceLimit,
    /// One profile identity appeared twice.
    #[error("duplicate profile identity")]
    DuplicateProfile,
}

impl ProfileCatalog {
    /// Build a bounded profile-identity inventory.
    pub fn new(
        profiles: impl IntoIterator<Item = impl Into<String>>,
    ) -> Result<Self, ProfileCatalogError> {
        let mut catalog = Self::default();
        for profile in profiles {
            if catalog.profiles.len() == MAX_SELECTED_DEFINITIONS {
                return Err(ProfileCatalogError::ResourceLimit);
            }
            if !catalog.profiles.insert(profile.into()) {
                return Err(ProfileCatalogError::DuplicateProfile);
            }
        }
        Ok(catalog)
    }

    /// Whether this inventory holds the profile `identity`.
    pub fn contains(&self, identity: &str) -> bool {
        self.profiles.contains(identity)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::{format, string::String, vec, vec::Vec};

    fn profile(index: usize) -> String {
        format!("acme.profile.{index}")
    }

    /// TC-180 (FR-131-AC-2): the profile inventory admits exactly
    /// `MAX_SELECTED_DEFINITIONS` distinct profiles, refuses one more as a
    /// resource limit, and refuses a repeated profile as a duplicate.
    #[test]
    fn profile_catalog_bounds_and_duplicates_are_refused_by_their_own_error() {
        let full: Vec<_> = (0..MAX_SELECTED_DEFINITIONS).map(profile).collect();
        let catalog = ProfileCatalog::new(full.clone()).unwrap();
        assert!(catalog.contains(&full[MAX_SELECTED_DEFINITIONS - 1]));
        assert!(!catalog.contains(&profile(MAX_SELECTED_DEFINITIONS)));

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
