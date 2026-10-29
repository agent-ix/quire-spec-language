// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-042-AC-16/AC-17, FR-050-AC-9..AC-12: no compiled-protocol handoff is
//! committed and the removed path constants are not exported. A consumer
//! generates a handoff with `write_v1`/`write_v2` and finds the authored
//! sources through `SOURCE_ROOT`.

use std::{path::Path, process::Command};

use ix_trace_rs::trace;
use quire_spec_language::protocol_artifact::handoff::SOURCE_ROOT;

const HANDOFF_SOURCE: &str = include_str!("../../src/protocol_artifact/handoff.rs");

/// `git ls-files <pathspec>` in this crate's checkout, or `None` (with a
/// message) when the build is not a git checkout whose top level is
/// `SOURCE_ROOT` (a crate tarball or a vendored copy).
fn tracked(pathspec: &str) -> Option<Vec<String>> {
    let root = Path::new(SOURCE_ROOT);
    let top = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["rev-parse", "--show-toplevel"])
        .output();
    let top = match top {
        Ok(output) if output.status.success() => {
            String::from_utf8_lossy(&output.stdout).trim().to_owned()
        }
        _ => {
            eprintln!("skipped: {SOURCE_ROOT} is not a git checkout (tarball build)");
            return None;
        }
    };
    let same = std::fs::canonicalize(&top).ok() == std::fs::canonicalize(root).ok();
    if !same {
        eprintln!("skipped: {SOURCE_ROOT} is not the top level of a git checkout ({top})");
        return None;
    }
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["ls-files", "--", pathspec])
        .output()
        .expect("git ls-files runs in a git checkout");
    assert!(output.status.success(), "git ls-files succeeds");
    Some(
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .map(str::to_owned)
            .collect(),
    )
}

/// True when the handoff module source declares or re-exports `name`.
fn exports(name: &str) -> bool {
    let declared = format!("const {name}:");
    HANDOFF_SOURCE.lines().any(|line| {
        let line = line.trim();
        (line.starts_with("pub") && line.contains(&declared))
            || (line.starts_with("pub use") && line.contains(name))
    })
}

#[test]
#[trace("TC-121", "FR-042-AC-16")]
fn no_v1_handoff_file_is_tracked() {
    if let Some(files) = tracked("artifacts/compiled-protocol-v1") {
        assert!(files.is_empty(), "tracked /1 handoff files: {files:?}");
    }
}

#[test]
#[trace("TC-121", "FR-042-AC-17")]
fn published_v1_handoff_is_not_exported() {
    assert!(!exports("PUBLISHED_V1_HANDOFF"));
    assert!(
        exports("PUBLISHED_V1_OFFER_FILE"),
        "detector sees a real export"
    );
}

#[test]
#[trace("TC-138", "FR-050-AC-9")]
fn no_v2_handoff_file_is_tracked() {
    if let Some(files) = tracked("artifacts/compiled-protocol-v2") {
        assert!(files.is_empty(), "tracked /2 handoff files: {files:?}");
    }
}

#[test]
#[trace("TC-138", "FR-050-AC-10")]
fn no_handoff_checksum_manifest_is_tracked() {
    let Some(files) = tracked(".") else { return };
    let checksums: Vec<_> = files
        .iter()
        .filter(|file| {
            let name = file.rsplit('/').next().unwrap_or(file).to_ascii_lowercase();
            file.contains("compiled-protocol")
                && (name.contains("checksum")
                    || name.contains("sha256sums")
                    || name.contains("sums"))
        })
        .collect();
    assert!(
        checksums.is_empty(),
        "tracked handoff checksum manifests: {checksums:?}"
    );
}

#[test]
#[trace("TC-138", "FR-050-AC-11")]
fn source_root_names_the_manifest_directory_with_authored_sources() {
    let root = Path::new(SOURCE_ROOT);
    assert!(root.is_absolute());
    assert!(
        root.join("Cargo.toml").is_file(),
        "manifest under SOURCE_ROOT"
    );
    assert!(root.join("docs/compiled-protocol-v1.md").is_file());
    assert!(root.join("docs/compiled-protocol-v2.md").is_file());
    assert!(root.join("examples/protocol-handoff").is_dir());
}

#[test]
#[trace("TC-138", "FR-050-AC-12")]
fn removed_published_path_constants_are_not_exported() {
    for name in [
        "PUBLISHED_HANDOFF",
        "PUBLISHED_V1_HANDOFF",
        "PUBLISHED_CHECKSUMS_FILE",
    ] {
        assert!(!exports(name), "{name} must not be exported");
    }
}
