// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-253 (FR-087-AC-3), ADR-011 §4 condition 1: the witness
//! `library::SupportedV2Wire` that `verify_binding` requires is minted only
//! by the layer-4 v2 reader, after IR admitted the bytes.
//!
//! `library` is layer 3 and may not name IR's admitted package type, so the
//! compiler alone cannot confine the witness's `pub(crate)` constructor to
//! one module. This `syn` scan does: every reference to
//! `attest_ir_admitted_v2` outside its own definition must sit in
//! `src/checked_package/checked_v2.rs`, or in `library`'s own test-only
//! `binding_tests.rs`. Code outside the crate cannot reach the constructor
//! or `verify_binding` at all; `VerifiedPackage`'s `compile_fail,E0603`
//! doctest shows that.
use std::path::{Path, PathBuf};

use ix_trace_rs::trace;
use syn::visit::Visit;

const CONSTRUCTOR: &str = "attest_ir_admitted_v2";
const READER: &str = "src/checked_package/checked_v2.rs";
const ALLOWED: [&str; 2] = [READER, "src/library/binding_tests.rs"];

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

fn source_files() -> Vec<String> {
    let root = root();
    let mut files = Vec::new();
    let mut pending = vec![root.join("src")];
    while let Some(dir) = pending.pop() {
        let entries = std::fs::read_dir(&dir)
            .unwrap_or_else(|error| panic!("{}: failed to list: {error}", dir.display()));
        for entry in entries {
            let path = entry.expect("directory entry reads").path();
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().and_then(|extension| extension.to_str()) == Some("rs") {
                let relative = path
                    .strip_prefix(&root)
                    .expect("under the manifest dir")
                    .to_string_lossy()
                    .replace('\\', "/");
                files.push(relative);
            }
        }
    }
    files.sort();
    files
}

/// Lines where `source` refers to the witness constructor by path.
fn references(source: &str) -> Vec<usize> {
    struct Finder(Vec<usize>);
    impl<'ast> Visit<'ast> for Finder {
        fn visit_path_segment(&mut self, segment: &'ast syn::PathSegment) {
            if segment.ident == CONSTRUCTOR {
                self.0.push(segment.ident.span().start().line);
            }
            syn::visit::visit_path_segment(self, segment);
        }
    }
    let parsed = syn::parse_file(source).expect("source parses as Rust");
    let mut finder = Finder(Vec::new());
    finder.visit_file(&parsed);
    finder.0
}

#[trace("TC-253", "FR-087-AC-3")]
#[test]
fn only_the_v2_reader_mints_the_condition_1_witness() {
    let root = root();
    let mut outside = Vec::new();
    let mut in_reader = 0;
    for file in source_files() {
        let source = std::fs::read_to_string(root.join(&file))
            .unwrap_or_else(|error| panic!("{file}: failed to read: {error}"));
        let lines = references(&source);
        if file == READER {
            in_reader = lines.len();
        } else if !ALLOWED.contains(&file.as_str()) {
            outside.extend(lines.into_iter().map(|line| format!("{file}:{line}")));
        }
    }
    assert!(
        outside.is_empty(),
        "the condition-1 witness is minted outside the v2 reader: {outside:?}"
    );
    assert_eq!(
        in_reader, 1,
        "{READER} must mint the witness exactly once, in IR's AdmittedV2 arm"
    );
}

/// The finder sees a call by path, so the live-tree pass is not vacuous.
#[trace("TC-253", "FR-087-AC-3")]
#[test]
fn the_scan_sees_a_synthetic_mint() {
    assert_eq!(
        references("fn forge() { let _ = SupportedV2Wire::attest_ir_admitted_v2(); }"),
        vec![1]
    );
}
