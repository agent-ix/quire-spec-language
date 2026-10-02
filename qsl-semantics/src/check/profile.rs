// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-110: E3 resolves a unit's header profile selections against the
//! `Value` family's catalog, [`DefinitionLock::pinned`].
//!
//! The one row a header selects is `root` (`quire.value.complete/v1`); the
//! catalog's other rows come from its own package-selection rules. A clause
//! profile resolves in its clause family's catalog (ADR-012 §2), which no
//! spine registers yet, so every header profile reaches this table.

use qsl_foundation::selection::ProfileSelection;
use qsl_foundation::{Code, Span};

use crate::value::definition::{CatalogEntry, CatalogRole, DefinitionLock};

/// Why a header profile does not resolve (FR-110's table).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProfileCause {
    /// The identity is in no catalog: `unknown_profile`/`unsupported-selection`.
    UnsupportedSelection,
    /// The identity is a catalog row other than `root`:
    /// `unknown_profile`/`wrong-selection-role`.
    WrongSelectionRole,
}

/// One header profile E3 refused, retaining the supplied selection and the
/// row it was compared with.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProfileRefusal {
    /// The profile's local alias.
    pub alias: String,
    /// The span of the selection's identity literal.
    pub identity_span: Span,
    /// The definition identity the header selected.
    pub selected: String,
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
        }
    }

    /// The `quire.native.diagnostics/v1` cause tag.
    pub fn cause(&self) -> &'static str {
        match self.cause {
            ProfileCause::UnsupportedSelection => "unsupported-selection",
            ProfileCause::WrongSelectionRole => "wrong-selection-role",
        }
    }
}

/// Resolve each header profile by identity against the `root` row of
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
            let selected = profile.identity.as_str();
            let cause = if selected == root.identity {
                return None;
            } else if lock
                .catalog()
                .iter()
                .any(|entry| entry.identity == selected)
            {
                ProfileCause::WrongSelectionRole
            } else {
                ProfileCause::UnsupportedSelection
            };
            Some(ProfileRefusal {
                alias: profile.alias.clone(),
                identity_span: profile.identity_span,
                selected: selected.to_owned(),
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
