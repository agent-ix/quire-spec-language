// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-067-AC-6 / TC-168: SEAM-5 (`complete::package::lower_source_graph` /
//! `LoweredSourceGraph`) is deleted in the same change as the S2 `forms`
//! core, with no compatibility shim, disabled test or relocated copy left
//! behind anywhere in the scanned code directories.
//!
//! FR-067-AC-5's own text calls the compiled-symbol scan a weak oracle on
//! its own ("rustc emits no symbol for an unreferenced non-generic struct,
//! so this scan can pass while `LoweredDeclaration` still exists in
//! source") and names the source-wide reference scan (AC-6, this test) as
//! the load-bearing check; this test backs that load-bearing check, not
//! the weak one.
use ix_trace_rs::trace;
use std::path::Path;

/// The three retired names, built so this file's own source never spells
/// any of them out as one literal substring (it would otherwise match its
/// own scan).
fn banned_names() -> Vec<String> {
    vec![
        ["Lowered", "SourceGraph"].concat(),
        ["Lowered", "Declaration"].concat(),
        ["lower_source", "_graph"].concat(),
    ]
}

/// Every regular file under `root`, recursively; `root` itself is skipped
/// (not an error) when it does not exist, matching the FR-067-AC-6 scan
/// over `benches/`, which this repository does not have.
fn files_under(root: &Path, out: &mut Vec<std::path::PathBuf>) {
    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if let Ok(file_type) = entry.file_type() {
            if file_type.is_dir() {
                files_under(&path, out);
            } else if file_type.is_file() {
                out.push(path);
            }
        }
    }
}

#[trace("TC-168", "FR-067-AC-6")]
#[test]
fn no_source_or_build_input_file_references_the_retired_seam() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let names = banned_names();
    let mut offending = Vec::new();
    // `src/`, `tests/`, `xtask/`, `examples/`, not `src/` alone; `benches/`
    // does not exist in this repository, and `files_under` treats that as
    // zero files, not an error. This scan does not reach `spec/` or
    // `docs/`, which name these three symbols by design (FR-067-AC-6).
    for root in ["src", "tests", "benches", "xtask", "examples"] {
        let mut files = Vec::new();
        files_under(&manifest_dir.join(root), &mut files);
        for path in files {
            if path.extension().and_then(|extension| extension.to_str()) != Some("rs") {
                continue;
            }
            // This test's own source names the three retired symbols only
            // through `banned_names`' split-and-concat construction; skip
            // it so that construction is never mistaken for a source
            // reference to what it is built to detect.
            if path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name == "seam5_retired.rs")
            {
                continue;
            }
            let Ok(contents) = std::fs::read_to_string(&path) else {
                continue;
            };
            for name in &names {
                if contents.contains(name.as_str()) {
                    offending.push((path.display().to_string(), name.clone()));
                }
            }
        }
    }
    assert!(
        offending.is_empty(),
        "SEAM-5's retired symbols still have source references: {offending:?}"
    );
}
