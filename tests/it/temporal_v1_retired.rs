// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-499-AC-1 / TC-1293: the v1 temporal evaluator, its progress ledger, its
//! activation and budget machinery and its protocol-ledger driver are deleted
//! from `src/temporal`, with no shim, alias or relocated copy left in the root
//! crate's `src/` or `tests/`.
//!
//! The kept surface is the FR-521 row-5 set (`Closure`, `Completeness`,
//! `Execution`, `Evidence`, `Position`, `OrderKey`, `Classification`,
//! `mapping_support_v2`, `evaluate_v2`, `Trace`, `Report`); this scan does not
//! name any of them. Each retired name is built from two separately-quoted
//! halves so this file's own source needs no exemption from its own scan.
use ix_trace_rs::trace;
use std::path::{Path, PathBuf};

/// The retired v1 entry points and result types.
fn banned_names() -> Vec<String> {
    vec![
        ["evaluate_", "checked"].concat(),
        ["evaluate_", "artifact_v2"].concat(),
        ["evaluate_", "with_progress"].concat(),
        ["mapping_support", "_checked"].concat(),
        ["Protocol", "Ledger"].concat(),
        ["Artifact", "Report"].concat(),
        ["temporal::", "Ledger"].concat(),
        ["temporal::", "Limits"].concat(),
        ["temporal::", "Obligation"].concat(),
    ]
}

/// The retired files of `src/temporal`.
fn banned_files() -> [&'static str; 5] {
    [
        "src/temporal/activation.rs",
        "src/temporal/budget.rs",
        "src/temporal/formula.rs",
        "src/temporal/progress.rs",
        "src/temporal/protocol.rs",
    ]
}

/// Every regular `.rs` file under `root`, recursively, appended to `out`.
fn files_under(root: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if let Ok(file_type) = entry.file_type() {
            if file_type.is_dir() {
                files_under(&path, out);
            } else if file_type.is_file()
                && path.extension().and_then(|extension| extension.to_str()) == Some("rs")
            {
                out.push(path);
            }
        }
    }
}

#[trace("TC-1293", "FR-499-AC-1")]
#[test]
fn the_retired_v1_temporal_evaluator_files_are_absent() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let present: Vec<&str> = banned_files()
        .into_iter()
        .filter(|file| manifest_dir.join(file).exists())
        .collect();
    assert!(present.is_empty(), "retired files still exist: {present:?}");
    assert!(
        manifest_dir.join("src/temporal/owner.rs").is_file(),
        "the kept FR-521 row-5 owner report is the file this scan must not reach"
    );
}

#[trace("TC-1293", "FR-499-AC-1")]
#[test]
fn no_root_source_or_test_names_a_retired_v1_temporal_entry_point() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let names = banned_names();
    let mut offending = Vec::new();
    let mut scanned = 0_usize;
    for root in ["src", "tests"] {
        let mut files = Vec::new();
        files_under(&manifest_dir.join(root), &mut files);
        for path in files {
            let Ok(contents) = std::fs::read_to_string(&path) else {
                continue;
            };
            scanned += 1;
            for name in &names {
                if contents.contains(name.as_str()) {
                    offending.push((path.display().to_string(), name.clone()));
                }
            }
        }
    }
    assert!(
        scanned > 0,
        "the scan found no `.rs` files under src/ or tests/; a root was renamed or moved"
    );
    assert!(
        offending.is_empty(),
        "retired v1 temporal names still have source references: {offending:?}"
    );
}
