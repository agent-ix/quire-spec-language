// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-162 steps 15 and 16 (FR-064-AC-14, FR-064-AC-15): the `#[string_edge]`
//! marker is `ix-trace-rs`'s. The workspace defines no attribute macro, no
//! re-export or alias of the marker and no marker-only crate, and every
//! package whose source carries the marker declares `ix-trace-rs` as a normal
//! dependency. `cargo check` of each package then resolves each marker to
//! `ix_trace_rs::string_edge`.

use std::fs;
use std::path::{Path, PathBuf};

use ix_trace_rs::trace;

struct Member {
    name: String,
    directory: PathBuf,
    normal: Vec<String>,
    proc_macro: bool,
}

fn members() -> Vec<Member> {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned());
    let output = std::process::Command::new(cargo)
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .args([
            "metadata",
            "--format-version",
            "1",
            "--no-deps",
            "--offline",
        ])
        .output()
        .expect("cargo metadata runs");
    assert!(
        output.status.success(),
        "cargo metadata failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let metadata: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("cargo metadata emits JSON");
    metadata["packages"]
        .as_array()
        .expect("a package list")
        .iter()
        .map(|package| Member {
            name: package["name"].as_str().expect("a name").to_owned(),
            directory: Path::new(package["manifest_path"].as_str().expect("a manifest"))
                .parent()
                .expect("a package directory")
                .to_path_buf(),
            normal: package["dependencies"]
                .as_array()
                .expect("a dependency list")
                .iter()
                .filter(|dependency| dependency["kind"].is_null())
                .map(|dependency| dependency["name"].as_str().expect("a name").to_owned())
                .collect(),
            proc_macro: package["targets"]
                .as_array()
                .expect("a target list")
                .iter()
                .any(|target| {
                    target["kind"]
                        .as_array()
                        .is_some_and(|kinds| kinds.iter().any(|kind| kind == "proc-macro"))
                }),
        })
        .collect()
}

/// Every `.rs` file under the package's `src/`, with its text.
fn sources(member: &Member) -> Vec<(PathBuf, String)> {
    fn walk(directory: &Path, found: &mut Vec<(PathBuf, String)>) {
        let Ok(entries) = fs::read_dir(directory) else {
            return;
        };
        for entry in entries {
            let path = entry.expect("a directory entry").path();
            if path.is_dir() {
                walk(&path, found);
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                let text = fs::read_to_string(&path).expect("a readable source file");
                found.push((path, text));
            }
        }
    }
    let mut found = Vec::new();
    walk(&member.directory.join("src"), &mut found);
    found
}

fn carries_marker(text: &str) -> bool {
    text.contains("#[string_edge]")
        || text.contains("ix_trace_rs::string_edge]")
        || text.contains("use ix_trace_rs::string_edge;")
}

/// FR-064-AC-14: no attribute macro, no marker-only crate, no re-export or
/// alias of the marker anywhere in the workspace.
#[trace("TC-162", "FR-064-AC-14")]
#[test]
fn the_workspace_defines_no_string_edge_marker_of_its_own() {
    let members = members();
    for member in &members {
        assert!(
            !member.proc_macro,
            "{} is a proc-macro crate; the marker is ix-trace-rs's",
            member.name
        );
        for (path, text) in sources(member) {
            for forbidden in [
                "proc_macro_attribute",
                "pub use ix_trace_rs::string_edge",
                "pub use ::ix_trace_rs::string_edge",
                "string_edge as ",
                "qsl_attrs",
            ] {
                // The scanner's own test fixtures spell paths inside string
                // literals; a definition or alias is never written there as
                // a whole-line item.
                let defined = text
                    .lines()
                    .map(str::trim_start)
                    .any(|line| line.starts_with(forbidden));
                assert!(!defined, "{} holds `{forbidden}`", path.display());
            }
        }
    }
}

/// FR-064-AC-15: every package that carries the marker names `ix-trace-rs`
/// among its normal dependencies.
#[trace("TC-162", "FR-064-AC-15")]
#[test]
fn every_package_that_uses_the_marker_depends_on_ix_trace_rs() {
    let mut users = 0;
    for member in members() {
        if sources(&member)
            .iter()
            .any(|(_, text)| carries_marker(text))
        {
            users += 1;
            assert!(
                member.normal.iter().any(|name| name == "ix-trace-rs"),
                "{} uses the marker without a normal ix-trace-rs dependency",
                member.name
            );
        }
    }
    assert!(users > 0, "no package carries the marker");
}
