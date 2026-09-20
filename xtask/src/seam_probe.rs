// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL#214 (FR-063): `cargo xtask seam-probe` demonstrates ADR-012 §5.1's S1
//! seam by building the QSL crate under `RUSTFLAGS=--cfg seam_probe` and
//! comparing the `E0004` (non-exhaustive match) locations rustc reports
//! against a checked-in list, exactly as FR-063 requires.
//!
//! **Scope: S1 only.** FR-063-AC-6 names five categories (S1's two matches,
//! S2, S3, S4). S2 (the parser's leading-token-kind entry table and the
//! parsed-form-enum check seam) and S3 (the checked-node-enum's evaluator,
//! v2 emitter and requirement-derivation matches) are seams over
//! `token::Kind`, `Expression` and `NodeKind` -- pre-existing, crate-wide
//! enums every `Value` form uses (literals, operators, `let`, `if`,
//! records, collections), not just the two forms this ticket migrates
//! (function declaration and application) -- so covering them means adding
//! a probe arm to every match site across `parser.rs`, `check.rs`,
//! `evaluate.rs`, `facts.rs` and `termination.rs`, correctly, for forms
//! this ticket does not own. That is tracked as its own piece of work
//! (Linear QSL-143), not attempted here. S4 (each family `Cause` enum's
//! `catalog_code()`) has no cause-bearing family to demonstrate it yet --
//! see `crate::family::outcome`'s own doc in the QSL crate. This tool's
//! checked-in list therefore covers exactly the two S1 locations #214
//! itself adds, and reports that scope honestly rather than as "AC-6
//! satisfied."

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

use crate::error::{Error, Result};

/// One seam-probe location: a file path (relative to the workspace root, as
/// rustc reports it) and the 1-based line of its `E0004`.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct SeamLocation {
    pub file: String,
    pub line: u32,
}

/// FR-063-AC-6's S1 category, checked in: the two `match`es over
/// `FamilyKind` in `src/family/mod.rs` -- `catalog_code_prefix`'s prefix
/// arm and `stage_hooks`'s stage-participation table (which also carries
/// the explicit `Relation`-at-`Evaluate` arm FR-063-AC-6 names). Exactly
/// two locations because that module has exactly two `match`es over
/// `FamilyKind` today; adding a third real seam over `FamilyKind` means
/// updating this list in the same change (FR-063-AC-1/AC-2), or
/// `xtask seam-probe` fails.
pub fn checked_in_locations() -> BTreeSet<SeamLocation> {
    [
        SeamLocation {
            file: "src/family/mod.rs".to_owned(),
            line: 107,
        },
        SeamLocation {
            file: "src/family/mod.rs".to_owned(),
            line: 201,
        },
    ]
    .into_iter()
    .collect()
}

/// Build the QSL crate's `--lib` target with `rustflags` appended to
/// `RUSTFLAGS` (empty for a normal build) and collect every `E0004`
/// diagnostic's primary span as `(file, line)`, plus whether the build
/// itself succeeded.
fn build_and_collect_e0004(workspace_root: &Path, rustflags: &str) -> Result<(bool, BTreeSet<SeamLocation>)> {
    let mut command = Command::new("cargo");
    command.current_dir(workspace_root).args([
        "build",
        "--offline",
        "-p",
        "quire-spec-language",
        "--lib",
        "--message-format=json",
    ]);
    if !rustflags.is_empty() {
        command.env("RUSTFLAGS", rustflags);
    }
    let output = command
        .output()
        .map_err(|source| Error::SeamProbeSpawn { source })?;
    let succeeded = output.status.success();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut locations = BTreeSet::new();
    for line in stdout.lines() {
        let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };
        if value.get("reason").and_then(serde_json::Value::as_str) != Some("compiler-message") {
            continue;
        }
        let Some(message) = value.get("message") else {
            continue;
        };
        let is_e0004 = message
            .get("code")
            .and_then(|code| code.get("code"))
            .and_then(serde_json::Value::as_str)
            == Some("E0004");
        if !is_e0004 {
            continue;
        }
        let Some(spans) = message.get("spans").and_then(serde_json::Value::as_array) else {
            continue;
        };
        for span in spans {
            if span.get("is_primary").and_then(serde_json::Value::as_bool) != Some(true) {
                continue;
            }
            let file = span.get("file_name").and_then(serde_json::Value::as_str);
            let line_start = span.get("line_start").and_then(serde_json::Value::as_u64);
            if let (Some(file), Some(line_start)) = (file, line_start) {
                locations.insert(SeamLocation {
                    file: file.to_owned(),
                    line: line_start as u32,
                });
            }
        }
    }
    Ok((succeeded, locations))
}

/// `cargo xtask seam-probe`: FR-063's two required assertions (a probe
/// build that fails with exactly the checked-in `E0004` locations, and a
/// normal build that succeeds with none), then the checked-in-list
/// comparison itself.
pub fn run(workspace_root: &Path) -> Result<String> {
    // FR-063 constraint: assert the probe build fails AND the normal build
    // succeeds -- one without the other is half a test. If `--cfg
    // seam_probe` silently stopped being applied, only checking the normal
    // build would never notice.
    let (normal_succeeded, normal_locations) = build_and_collect_e0004(workspace_root, "")?;
    if !normal_succeeded {
        return Err(Error::SeamProbeNormalBuildFailed);
    }
    if !normal_locations.is_empty() {
        return Err(Error::SeamProbeNormalBuildHasE0004 {
            locations: format!("{normal_locations:?}"),
        });
    }
    let (probe_succeeded, probe_locations) = build_and_collect_e0004(workspace_root, "--cfg seam_probe")?;
    if probe_succeeded {
        return Err(Error::SeamProbeBuildUnexpectedlySucceeded);
    }
    let checked_in = checked_in_locations();
    let unexpected_but_present: Vec<_> = probe_locations.difference(&checked_in).cloned().collect();
    let expected_but_missing: Vec<_> = checked_in.difference(&probe_locations).cloned().collect();
    if !unexpected_but_present.is_empty() || !expected_but_missing.is_empty() {
        return Err(Error::SeamProbeMismatch {
            unexpected_but_present: format!("{unexpected_but_present:?}"),
            expected_but_missing: format!("{expected_but_missing:?}"),
        });
    }
    Ok(format!(
        "seam-probe: {} checked-in S1 location(s) confirmed under RUSTFLAGS=--cfg seam_probe; \
         normal build has none. S2/S3 (QSL-143) and S4 (no cause-bearing family yet) are not \
         covered by this checked-in list.\n",
        checked_in.len()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;

    /// FR-063-AC-6, S1 portion only (see this module's own scope note).
    #[trace("TC-161", "FR-063-AC-6")]
    #[test]
    fn checked_in_locations_are_the_two_family_kind_matches() {
        let locations = checked_in_locations();
        assert_eq!(locations.len(), 2);
        assert!(locations.iter().all(|location| location.file == "src/family/mod.rs"));
    }
}
