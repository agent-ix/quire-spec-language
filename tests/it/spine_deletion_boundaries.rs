// SPDX-License-Identifier: AGPL-3.0-or-later
//! By-absence checks for the checked-package spine's dependencies on the
//! retained root compiler and protocol-artifact surfaces.

use ix_trace_rs::trace;
use std::path::{Path, PathBuf};

fn rust_sources(root: &Path) -> Vec<PathBuf> {
    let mut pending = vec![root.to_path_buf()];
    let mut files = Vec::new();
    while let Some(path) = pending.pop() {
        let entries = std::fs::read_dir(&path)
            .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
        for entry in entries {
            let path = entry.expect("directory entry").path();
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                files.push(path);
            }
        }
    }
    files.sort();
    files
}

fn mentions_symbol(source: &str, symbol: &str) -> bool {
    source.match_indices(symbol).any(|(start, _)| {
        start == 0
            || !source.as_bytes()[start - 1].is_ascii_alphanumeric()
                && source.as_bytes()[start - 1] != b'_'
    })
}

fn assert_absent(root: &Path, forbidden: &[&str]) {
    for path in rust_sources(root) {
        let source = std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
        for symbol in forbidden {
            assert!(
                !mentions_symbol(&source, symbol),
                "{} names retired boundary {symbol}",
                path.display()
            );
        }
    }
}

/// The spine producer/type crates do not route through the retained composed
/// parser, and the checked-package crates do not reach into root-owned
/// front-end or protocol-artifact modules. The root keeps those modules until
/// their separately gated deletion slices.
#[trace("FR-362-AC-6", "FR-493-AC-3", "FR-493-AC-4", "FR-499")]
#[test]
fn spine_crates_do_not_import_the_retained_root_frontend_or_protocol_reader() {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"));
    assert_absent(
        &workspace.join("qsl-replay/src/spine"),
        &[
            "parse_native",
            "parse_native_source",
            "syntax::composed",
            "linking::composed",
            "checking::composed",
        ],
    );
    for crate_name in ["qsl-package", "qsl-eval", "qsl-replay"] {
        assert_absent(
            &workspace.join(crate_name).join("src"),
            &[
                "DeclarationKind::Protocol",
                "Family::Protocol",
                "RegisteredDefinition::Protocol",
                "quire_spec_language::parser",
                "quire_spec_language::syntax",
                "quire_spec_language::linking::native",
                "quire_spec_language::checking",
                "quire_spec_language::model_source",
                "quire_spec_language::located_json",
                "quire_spec_language::protocol_artifact",
            ],
        );
    }
}
