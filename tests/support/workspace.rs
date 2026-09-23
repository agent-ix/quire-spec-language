// SPDX-License-Identifier: AGPL-3.0-or-later
//! The workspace's member crates, read from `cargo metadata`, so a source
//! scan covers every crate as soon as it is a member instead of only the ones
//! someone remembered to list (QSL-183 review L6, I6).

use std::path::{Path, PathBuf};

/// The workspace root: this crate's own manifest directory.
pub fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

/// `cargo metadata --no-deps` for the workspace, as JSON.
fn metadata() -> serde_json::Value {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned());
    let output = std::process::Command::new(cargo)
        .current_dir(root())
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
    serde_json::from_slice(&output.stdout).expect("cargo metadata emits JSON")
}

/// `path` relative to the workspace root.
fn relative(path: &Path) -> PathBuf {
    path.strip_prefix(root())
        .expect("a path under the workspace root")
        .to_path_buf()
}

/// Every workspace member's directory, relative to the workspace root (the
/// root crate is `""`), sorted.
pub fn member_dirs() -> Vec<PathBuf> {
    let mut members: Vec<PathBuf> = metadata()["packages"]
        .as_array()
        .expect("a package list")
        .iter()
        .map(|package| {
            relative(
                Path::new(package["manifest_path"].as_str().expect("a manifest path"))
                    .parent()
                    .expect("a package directory"),
            )
        })
        .collect();
    members.sort();
    assert!(
        members.len() > 1,
        "cargo metadata lists too few members: {members:?}"
    );
    members
}

/// The source roots of every workspace member's shipped targets (`lib`,
/// `proc-macro` and `bin`): the directory holding each target's root file,
/// relative to the workspace root, sorted and deduplicated. The root crate
/// gives `src` and `tools/fixture-audit`; `tools/arch-lint`, whose sources
/// sit beside its manifest, gives `tools/arch-lint`. Members whose directory
/// is named in `excluded` are left out. Each root must exist.
pub fn member_src_roots(excluded: &[&str]) -> Vec<PathBuf> {
    let mut roots = Vec::new();
    for package in metadata()["packages"].as_array().expect("a package list") {
        let member = relative(
            Path::new(package["manifest_path"].as_str().expect("a manifest path"))
                .parent()
                .expect("a package directory"),
        );
        if excluded.iter().any(|name| member == Path::new(name)) {
            continue;
        }
        let mut shipped = 0;
        for target in package["targets"].as_array().expect("a target list") {
            let kinds = target["kind"].as_array().expect("a kind list");
            if !kinds
                .iter()
                .any(|kind| matches!(kind.as_str(), Some("lib" | "proc-macro" | "bin")))
            {
                continue;
            }
            shipped += 1;
            let dir = relative(
                Path::new(target["src_path"].as_str().expect("a source path"))
                    .parent()
                    .expect("a source directory"),
            );
            assert!(
                root().join(&dir).is_dir(),
                "{}: source root {} does not exist",
                member.display(),
                dir.display()
            );
            roots.push(dir);
        }
        assert!(shipped > 0, "{} has no shipped target", member.display());
    }
    roots.sort();
    roots.dedup();
    roots
}

/// Every regular `.rs` file under each of `roots` (relative to the workspace
/// root), recursively, as paths relative to the workspace root. A root or
/// directory that cannot be read is an error, so a renamed or moved root
/// fails the scan instead of letting it pass with no coverage there.
pub fn rust_files_under(roots: &[PathBuf]) -> Vec<PathBuf> {
    let root = root();
    let mut files = Vec::new();
    let mut pending: Vec<PathBuf> = roots.iter().map(|relative| root.join(relative)).collect();
    while let Some(dir) = pending.pop() {
        let entries = std::fs::read_dir(&dir)
            .unwrap_or_else(|error| panic!("{}: cannot be read: {error}", dir.display()));
        for entry in entries {
            let path = entry
                .unwrap_or_else(|error| panic!("{}: {error}", dir.display()))
                .path();
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                files.push(
                    path.strip_prefix(&root)
                        .expect("a file under the workspace root")
                        .to_path_buf(),
                );
            }
        }
    }
    files.sort();
    files
}
