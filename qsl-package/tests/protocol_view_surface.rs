// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-519-AC-2, FR-519-AC-9 and FR-519-AC-13 (TC-1314 rows 2, 9 and 13): the
//! read surface of `qsl_package::protocol_view` has one causal-relation
//! derivation over a checked package, and exports nothing for the records the
//! map ends or for a window binding.
//!
//! The export proof is the `compile_fail` doctests in the module
//! documentation of `src/protocol_view.rs`, which `cargo test --doc` runs.
//! These tests bind them: each fails when a doctest loses its pinned error
//! code or its subject, so a doctest cannot be dropped or weakened without a
//! failing test here.

use std::path::{Path, PathBuf};

use ix_trace_rs::trace;

fn read(path: &Path) -> String {
    std::fs::read_to_string(path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

/// The fenced blocks of the `//!` module documentation of `text`: info string
/// and body.
fn module_doc_blocks(text: &str) -> Vec<(String, String)> {
    let mut blocks = Vec::new();
    let mut open: Option<(String, String)> = None;
    for line in text.lines() {
        let Some(doc) = line.trim_start().strip_prefix("//!") else {
            continue;
        };
        let doc = doc.trim();
        match (&mut open, doc.strip_prefix("```")) {
            (None, Some(info)) => open = Some((info.to_owned(), String::new())),
            (Some(_), Some(_)) => blocks.extend(open.take()),
            (Some((_, body)), None) => {
                body.push_str(doc);
                body.push('\n');
            }
            (None, None) => {}
        }
    }
    blocks
}

fn protocol_view_blocks() -> Vec<(String, String)> {
    module_doc_blocks(&read(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("src/protocol_view.rs"),
    ))
}

/// The number of `compile_fail,E0432` doctests that `use` exactly `item`.
fn unresolved_uses(blocks: &[(String, String)], item: &str) -> usize {
    let wanted = format!("use qsl_package::protocol_view::{item};");
    blocks
        .iter()
        .filter(|(info, body)| info == "compile_fail,E0432" && body.trim() == wanted)
        .count()
}

/// FR-519-AC-9: each of `ExportRef`, `ArtifactKind`, `Revision`, `Language`
/// and `Definition` has a doctest whose `use` fails with E0432, and a passing
/// control proves the module does export the names it keeps.
#[trace("FR-519-AC-9", "TC-1314")]
#[test]
fn the_ended_records_are_pinned_as_unresolved_imports() {
    let blocks = protocol_view_blocks();
    for ended in [
        "ExportRef",
        "ArtifactKind",
        "Revision",
        "Language",
        "Definition",
    ] {
        assert_eq!(unresolved_uses(&blocks, ended), 1, "{ended}");
    }
    let control = blocks
        .iter()
        .filter(|(info, body)| {
            info.is_empty()
                && body.contains("use qsl_package::protocol_view::{ControlOperation, Delivery")
        })
        .count();
    assert_eq!(control, 1, "the compiling control");
}

/// FR-519-AC-13: the doctest that imports `Window` from `protocol_view` is
/// pinned as an unresolved import, and the module declares no `Window` item.
#[trace("FR-519-AC-13", "TC-1314")]
#[test]
fn a_window_binding_is_pinned_as_an_unresolved_import() {
    assert_eq!(unresolved_uses(&protocol_view_blocks(), "Window"), 1);
    let source = read(&Path::new(env!("CARGO_MANIFEST_DIR")).join("src/protocol_view.rs"));
    for declared in ["struct Window", "enum Window", "type Window"] {
        assert!(!source.contains(declared), "{declared}");
    }
}

/// The production sources of `qsl-*/src` and `src`: every `.rs` file outside
/// a test module.
fn production_sources() -> Vec<PathBuf> {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        let mut entries: Vec<_> = std::fs::read_dir(dir)
            .unwrap_or_else(|error| panic!("{}: {error}", dir.display()))
            .map(|entry| entry.expect("a directory entry").path())
            .collect();
        entries.sort();
        for path in entries {
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if path.is_dir() {
                if name != "tests" {
                    walk(&path, out);
                }
            } else if name.ends_with(".rs") && name != "tests.rs" {
                out.push(path);
            }
        }
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("the workspace root");
    let mut out = Vec::new();
    walk(&root.join("src"), &mut out);
    let mut crates: Vec<_> = std::fs::read_dir(root)
        .expect("the workspace root lists")
        .map(|entry| entry.expect("an entry").path())
        .filter(|path| {
            path.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("qsl-"))
        })
        .collect();
    crates.sort();
    for krate in crates {
        if krate.join("src").is_dir() {
            walk(&krate.join("src"), &mut out);
        }
    }
    out
}

/// The `fn` items of `text` with the signature up to the opening brace: name
/// and the signature text.
fn signatures(text: &str) -> Vec<(String, String)> {
    let mut found = Vec::new();
    let mut rest = text;
    while let Some(at) = rest.find("fn ") {
        let before_ok = rest[..at]
            .chars()
            .next_back()
            .is_none_or(|c| !(c.is_alphanumeric() || c == '_'));
        let after = &rest[at + 3..];
        let name: String = after
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect();
        if before_ok && !name.is_empty() {
            let end = after.find(['{', ';']).unwrap_or(after.len());
            found.push((name, after[..end].to_owned()));
        }
        rest = after;
    }
    found
}

/// FR-519-AC-2: among the production sources of `qsl-*/src` and `src`, the
/// functions that take the nodes of a checked package (a `CheckedPackageV2`
/// or a slice of its nodes) and return a causal relation or its edges are
/// `causal_relation` and the private `relation_over` it delegates to, both in
/// `protocol_view.rs`; the checker's own `CausalEdge` is built from a
/// `CheckedControlTree`, not from package nodes.
#[trace("FR-519-AC-2", "TC-1314")]
#[test]
fn causal_relation_is_the_only_derivation_over_checked_package_nodes() {
    let mut deriving = Vec::new();
    for path in production_sources() {
        let text = read(&path);
        for (name, signature) in signatures(&text) {
            let (parameters, returns) = signature.split_once("->").unwrap_or((&signature, ""));
            let over_nodes = parameters.contains("CheckedPackageV2")
                || parameters.contains("CheckedSemanticNodeV2")
                || parameters.contains("[Node]")
                || parameters.contains("&Node");
            let returns_relation =
                returns.contains("CausalRelation") || returns.contains("CausalEdge");
            if over_nodes && returns_relation {
                let file = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or_default()
                    .to_owned();
                deriving.push((file, name));
            }
        }
    }
    deriving.sort();
    assert_eq!(
        deriving,
        [
            ("protocol_view.rs".to_owned(), "causal_relation".to_owned()),
            ("protocol_view.rs".to_owned(), "relation_over".to_owned()),
        ]
    );
    let checker = read(
        &Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("the workspace root")
            .join("qsl-semantics/src/check/protocol_clause/graph.rs"),
    );
    assert!(
        checker.contains("CheckedControlTree"),
        "the checker starts from the control tree"
    );
    assert!(!checker.contains("CheckedPackageV2"));
}
