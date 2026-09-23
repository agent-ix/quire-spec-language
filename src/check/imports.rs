// SPDX-License-Identifier: AGPL-3.0-or-later
//! E3's name index over one imported package's [`ImportView`] (FR-087-AC-4,
//! Behavior "`ImportView` never resolves a name"; ADR-013 R-06).
//!
//! `library` exposes a verified dependency's exports only as
//! `(name, PackageNodeKey)` entries keyed by `WireNodeId`. The checker owns
//! name resolution (scoping, visibility, ambiguity), so it builds its own
//! index from those entries here and resolves a name against it. `library`
//! has no function that takes a name and returns a node id
//! (`tests/it/import_view_names.rs`).

use std::collections::BTreeMap;

use crate::library::{ImportView, PackageNodeKey};

use super::refusal::CheckCause;

/// One imported package's exports, indexed by exported name. `pub` for the
/// layer-4 v2 reader's tests across the QSL-181 crate boundary; its
/// production caller is E3 imported-name resolution (FR-087-AC-13, TC-379).
///
/// Each name maps to one node. `library` admits a package only if its
/// identity preimage declares each name at most once (it refuses
/// `ambiguous-name` as a malformed preimage), so a view never carries two
/// entries with one name.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ImportedNames<'a>(BTreeMap<&'a str, PackageNodeKey>);

impl<'a> ImportedNames<'a> {
    /// The name index of `view`'s entries.
    pub fn of(view: &'a ImportView) -> Self {
        Self(view.exports().collect())
    }

    /// The node `name` resolves to among the imported package's exports,
    /// or `missing_declaration` / `missing-name` when it exports no
    /// declaration of that name.
    pub fn resolve(&self, name: &str) -> Result<PackageNodeKey, CheckCause> {
        self.0
            .get(name)
            .copied()
            .ok_or_else(|| CheckCause::MissingName(name.to_owned()))
    }
}
