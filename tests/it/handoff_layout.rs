// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-042-AC-16/AC-17, FR-050-AC-9..AC-12: no compiled-protocol handoff is
//! committed and the removed path constants are not exported. A consumer
//! generates a handoff with `write_v1`/`write_v2` and finds the authored
//! sources through `SOURCE_ROOT`.
//!
//! Git preconditions: the tracked-file checks need a git checkout whose top
//! level is `SOURCE_ROOT`. Locally (no `CI` variable) a tarball build skips
//! them with a message. When `CI` is set they FAIL instead, so a CI
//! misconfiguration cannot pass them vacuously.
//!
//! FR-050-AC-10 rule, on the file NAME alone (case-insensitive), over ALL
//! tracked files: a checksum manifest is a file whose name contains
//! `checksum`, `sha256sums`, `sha512sums`, `sha1sums` or `md5sums`, or ends in
//! `.sha256`, `.sha512` or `.sums`. No legitimately tracked file matches.

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use ix_trace_rs::trace;
use quire_spec_language::protocol_artifact::handoff::SOURCE_ROOT;

/// A git precondition does not hold: fail under CI, skip loudly otherwise.
fn unmet(reason: &str) -> Option<Vec<String>> {
    if std::env::var_os("CI").is_some() {
        panic!("CI is set but the git checkout precondition fails: {reason}");
    }
    eprintln!("SKIPPED (not a git checkout, not CI): {reason}");
    None
}

/// `git ls-files <pathspec>` in this crate's checkout, or `None` when the
/// preconditions do not hold outside CI.
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
        _ => return unmet(&format!("{SOURCE_ROOT} is not a git checkout")),
    };
    if fs::canonicalize(&top).ok() != fs::canonicalize(root).ok() {
        return unmet(&format!(
            "{SOURCE_ROOT} is not the top level of a git checkout ({top})"
        ));
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

/// True when `file`'s name is a checksum manifest by the rule in the module doc.
fn is_checksum_manifest(file: &str) -> bool {
    let name = file.rsplit('/').next().unwrap_or(file).to_ascii_lowercase();
    [
        "checksum",
        "sha256sums",
        "sha512sums",
        "sha1sums",
        "md5sums",
    ]
    .iter()
    .any(|part| name.contains(part))
        || [".sha256", ".sha512", ".sums"]
            .iter()
            .any(|suffix| name.ends_with(suffix))
}

fn rust_files(directory: &Path, files: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(directory).expect("read source directory") {
        let path = entry.expect("directory entry").path();
        if path.is_dir() {
            rust_files(&path, files);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            files.push(path);
        }
    }
}

fn use_tree_names(tree: &syn::UseTree, names: &mut Vec<String>) {
    match tree {
        syn::UseTree::Path(path) => use_tree_names(&path.tree, names),
        syn::UseTree::Name(name) => names.push(name.ident.to_string()),
        syn::UseTree::Rename(rename) => {
            names.push(rename.ident.to_string());
            names.push(rename.rename.to_string());
        }
        syn::UseTree::Glob(_) => {}
        syn::UseTree::Group(group) => group.items.iter().for_each(|t| use_tree_names(t, names)),
    }
}

/// Every non-private item or re-export named `name` in `source`, of any kind
/// (`pub`, `pub(crate)`, `pub(super)`, `pub(in ..)`), at any module depth.
fn exported_idents(source: &str) -> Vec<String> {
    struct Finder(Vec<String>);
    impl<'ast> syn::visit::Visit<'ast> for Finder {
        fn visit_item_const(&mut self, item: &'ast syn::ItemConst) {
            if !matches!(item.vis, syn::Visibility::Inherited) {
                self.0.push(item.ident.to_string());
            }
            syn::visit::visit_item_const(self, item);
        }
        fn visit_item_static(&mut self, item: &'ast syn::ItemStatic) {
            if !matches!(item.vis, syn::Visibility::Inherited) {
                self.0.push(item.ident.to_string());
            }
            syn::visit::visit_item_static(self, item);
        }
        fn visit_item_use(&mut self, item: &'ast syn::ItemUse) {
            if !matches!(item.vis, syn::Visibility::Inherited) {
                use_tree_names(&item.tree, &mut self.0);
            }
            syn::visit::visit_item_use(self, item);
        }
        fn visit_item_type(&mut self, item: &'ast syn::ItemType) {
            if !matches!(item.vis, syn::Visibility::Inherited) {
                self.0.push(item.ident.to_string());
            }
            syn::visit::visit_item_type(self, item);
        }
    }
    let file = syn::parse_file(source).expect("source parses");
    let mut finder = Finder(Vec::new());
    syn::visit::Visit::visit_file(&mut finder, &file);
    finder.0
}

/// True when any file under `src/protocol_artifact/` exports `name`.
fn exported_under_protocol_artifact(name: &str) -> bool {
    let mut files = Vec::new();
    rust_files(
        &Path::new(SOURCE_ROOT).join("src/protocol_artifact"),
        &mut files,
    );
    assert!(!files.is_empty(), "protocol_artifact sources found");
    files.iter().any(|file| {
        let source = fs::read_to_string(file).expect("read source");
        exported_idents(&source).iter().any(|ident| ident == name)
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
    assert!(!exported_under_protocol_artifact("PUBLISHED_V1_HANDOFF"));
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
    assert!(!files.is_empty(), "git lists tracked files");
    let checksums: Vec<_> = files.iter().filter(|f| is_checksum_manifest(f)).collect();
    assert!(
        checksums.is_empty(),
        "tracked checksum manifests: {checksums:?}"
    );
}

#[test]
#[trace("TC-138", "FR-050-AC-10")]
fn the_checksum_manifest_rule_catches_each_name_form() {
    for name in [
        "SHA256SUMS",
        "sub/dir/handoff-checksums.txt",
        "CHECKSUMS",
        "x.sha256",
        "x.sha512",
        "x.sums",
        "MD5SUMS",
    ] {
        assert!(is_checksum_manifest(name), "{name}");
    }
    for name in ["Cargo.lock", "src/summary.rs", "docs/consumers.md"] {
        assert!(!is_checksum_manifest(name), "{name}");
    }
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
        assert!(
            !exported_under_protocol_artifact(name),
            "{name} must not be exported"
        );
    }
}

#[test]
#[trace("TC-138", "FR-050-AC-12")]
fn the_export_detector_sees_every_form_it_handles() {
    for source in [
        "pub const NAME: &str = \"\";",
        "pub const NAME : &str = \"\";",
        "pub(crate) const NAME: &str = \"\";",
        "pub static NAME: &str = \"\";",
        "pub use x::NAME;",
        "pub use x::{\n    OTHER,\n    NAME,\n};",
        "pub use x::Y as NAME;",
        "pub use x::NAME as Z;",
        "pub(crate) use x::{a::{NAME}};",
        "mod m { pub const NAME: u8 = 0; }",
        "pub type NAME = u8;",
    ] {
        assert!(
            exported_idents(source).iter().any(|i| i == "NAME"),
            "undetected: {source}"
        );
    }
    for source in [
        "const NAME: u8 = 0;",
        "use x::NAME;",
        "pub const OTHER: u8 = 0;",
    ] {
        assert!(
            !exported_idents(source).iter().any(|i| i == "NAME"),
            "false positive: {source}"
        );
    }
}
