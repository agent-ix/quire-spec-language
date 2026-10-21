// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-064's consumer edge rules: the allow-list a caller supplies, as a typed
//! value or as the strict JSON object the command reads.
use std::path::{Component, Path};

use serde::Deserialize;

use crate::error::{Error, Result};

/// One allow-list entry: a location and why it is not a dispatch decision
/// (FR-064's own rule: a debug-only consistency check, a logged message, a
/// diagnostic payload or a test assertion -- never a branch-gating
/// comparison).
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct AllowListEntry {
    /// The source file the allow-listed occurrence is in, relative to the
    /// base directory.
    pub file: String,
    /// The enclosing item's name (PR #262 review, finding F8). An earlier
    /// version of this struct keyed on `(file, line)`, the exact brittleness
    /// `xtask::seam_probe::SeamLocation`'s own doc already fixed: a line
    /// number moves whenever anything *above* the comparison in the same
    /// file changes -- a new doc comment, an added item, a `cargo fmt` reflow
    /// -- none of which touch the comparison itself. Keying on the enclosing
    /// item's name (stable against all of that) matches `SeamLocation`
    /// exactly. The trade-off is coarser than a line number: an entry admits
    /// every occurrence in the same item, not one specific comparison.
    pub item: String,
    /// Why this occurrence is not a dispatch decision. Never empty.
    pub reason: String,
}

/// The edge rules a scan applies.
#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct EdgeRules {
    /// The checked-in allow-list; an empty list is an explicit empty
    /// allow-list.
    pub allow_list: Vec<AllowListEntry>,
}

impl EdgeRules {
    /// Parse the rules JSON object: `{"allow_list": [{"file", "item",
    /// "reason"}, ...]}`. An unknown or missing field, a wrong type or
    /// malformed JSON is [`Error::RulesMalformed`]; paths and reasons are
    /// checked when the rules are applied to a base directory.
    pub fn from_json(text: &str) -> Result<Self> {
        serde_json::from_str(text).map_err(|source| Error::RulesMalformed { source })
    }
}

/// The rules checked against `base`: every entry's reason is non-empty and
/// its file is a path inside `base`. Returns the `(file, item)` keys with
/// the file normalised to the base-relative `/` form reports use, in entry
/// order.
pub(crate) fn validated_keys(rules: &EdgeRules, base: &Path) -> Result<Vec<(String, String)>> {
    rules
        .allow_list
        .iter()
        .map(|entry| {
            if entry.reason.trim().is_empty() {
                return Err(Error::RuleReasonEmpty {
                    file: entry.file.clone(),
                    item: entry.item.clone(),
                });
            }
            let file =
                relative_to_base(base, &entry.file).ok_or_else(|| Error::RulePathOutsideBase {
                    file: entry.file.clone(),
                })?;
            Ok((file, entry.item.clone()))
        })
        .collect()
}

/// `file` as a base-relative path in `/` form, lexically normalised, or
/// `None` when it leaves `base` or names no file.
fn relative_to_base(base: &Path, file: &str) -> Option<String> {
    let path = Path::new(file);
    let path = if path.is_absolute() {
        path.strip_prefix(base).ok()?
    } else {
        path
    };
    let mut parts: Vec<String> = Vec::new();
    for component in path.components() {
        match component {
            Component::Normal(part) => parts.push(part.to_string_lossy().into_owned()),
            Component::CurDir => {}
            Component::ParentDir => {
                parts.pop()?;
            }
            Component::RootDir | Component::Prefix(_) => return None,
        }
    }
    if parts.is_empty() {
        return None;
    }
    Some(parts.join("/"))
}
