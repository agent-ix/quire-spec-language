// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-110: E3 resolves a unit's header profile selections against the
//! `Value` family's catalog, [`DefinitionLock::pinned`].
//!
//! The one row a header selects is `root` (`quire.value.complete/v1`); the
//! catalog's other rows come from its own package-selection rules. A clause
//! profile resolves in its clause family's catalog (ADR-012 §2), which no
//! spine registers yet, so every header profile reaches this table.

use qsl_foundation::selection::{DefinitionRef, ProfileSelection};
use qsl_foundation::{Code, Span};

use crate::value::{CatalogEntry, CatalogRole, DefinitionLock};

/// Why a header profile does not resolve (FR-110's table).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProfileCause {
    /// The identity is in no catalog: `unknown_profile`/`unsupported-selection`.
    UnsupportedSelection,
    /// The identity is a catalog row other than `root`:
    /// `unknown_profile`/`wrong-selection-role`.
    WrongSelectionRole,
    /// `root`'s identity at another version:
    /// `stale_dependency`/`revision-mismatch`.
    RevisionMismatch,
    /// `root`'s identity and version with another digest:
    /// `stale_dependency`/`byte-digest-mismatch`.
    ByteDigestMismatch,
}

/// One header profile E3 refused, retaining the supplied selection and the
/// row it was compared with.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProfileRefusal {
    /// The profile's local alias.
    pub alias: String,
    /// The span of the selection's identity literal.
    pub identity_span: Span,
    /// The selection the header supplied.
    pub selected: DefinitionRef,
    /// The role a header profile must select.
    pub required_role: CatalogRole,
    /// The `root` row the selection was compared with.
    pub root: CatalogEntry,
    /// Why it refused.
    pub cause: ProfileCause,
}

impl ProfileRefusal {
    /// The `quire.native.diagnostics/v1` code.
    pub fn code(&self) -> Code {
        match self.cause {
            ProfileCause::UnsupportedSelection | ProfileCause::WrongSelectionRole => {
                Code::UnknownProfile
            }
            ProfileCause::RevisionMismatch | ProfileCause::ByteDigestMismatch => {
                Code::StaleDependency
            }
        }
    }

    /// The `quire.native.diagnostics/v1` cause tag.
    pub fn cause(&self) -> &'static str {
        match self.cause {
            ProfileCause::UnsupportedSelection => "unsupported-selection",
            ProfileCause::WrongSelectionRole => "wrong-selection-role",
            ProfileCause::RevisionMismatch => "revision-mismatch",
            ProfileCause::ByteDigestMismatch => "byte-digest-mismatch",
        }
    }
}

/// Resolve each header profile against the `root` row of
/// [`DefinitionLock::pinned`], in source order (FR-110). Every refusing
/// profile is returned, in source order.
pub fn resolve_profiles(profiles: &[ProfileSelection]) -> Result<(), Vec<ProfileRefusal>> {
    let lock = DefinitionLock::pinned();
    let root = *lock
        .entry(CatalogRole::Root)
        .expect("the closed catalog holds a `root` row");
    let refusals: Vec<ProfileRefusal> = profiles
        .iter()
        .filter_map(|profile| {
            let selected = &profile.definition;
            let cause = if selected.identity() == root.identity {
                if selected.version() != root.revision_value {
                    ProfileCause::RevisionMismatch
                } else if selected.digest().digest().to_string()
                    != format!("sha256:{}", root.digest)
                {
                    ProfileCause::ByteDigestMismatch
                } else {
                    return None;
                }
            } else if lock
                .catalog()
                .iter()
                .any(|entry| entry.identity == selected.identity())
            {
                ProfileCause::WrongSelectionRole
            } else {
                ProfileCause::UnsupportedSelection
            };
            Some(ProfileRefusal {
                alias: profile.alias.clone(),
                identity_span: profile.identity_span,
                selected: selected.clone(),
                required_role: CatalogRole::Root,
                root,
                cause,
            })
        })
        .collect();
    if refusals.is_empty() {
        Ok(())
    } else {
        Err(refusals)
    }
}
