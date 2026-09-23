// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-067-AC-6 / TC-168: the retired structural-lowering graph type, its
//! declaration-occurrence sibling type, and the function that built the
//! graph from a resolved source package are deleted in the same change as
//! the S2 `forms` core, with no compatibility shim, disabled test or
//! relocated copy left behind anywhere in the scanned code directories.
//!
//! FR-067-AC-5's own text calls the compiled-symbol scan a weak oracle on
//! its own ("rustc emits no symbol for an unreferenced non-generic struct,
//! so this scan can pass while [the declaration-occurrence type] still
//! exists in source") and names the source-wide reference scan (AC-6, this
//! test) as the load-bearing check; this test backs that load-bearing
//! check, and backs AC-5 itself along with it: this scan covers every
//! `.rs` file under `src/`, so zero source references anywhere in `src/`
//! entails no definition anywhere in `src/`, which entails no compiled
//! symbol — AC-5's own claim, not only AC-6's. It also backs TC-168 step 2
//! (AC-5): `complete::mod`'s re-export list is itself a `.rs` file under
//! `src/`, so this same scan already reaches it, without a separate pass.
//!
//! This file's own source never spells any of the three retired names out
//! as one literal substring — [`banned_names`] builds each from two
//! separately-quoted halves — so it needs no exemption from its own scan,
//! and none is given: every `.rs` file under the scanned roots, including
//! this one, is checked.
use ix_trace_rs::trace;
use std::path::Path;

/// The three retired names, built so this file's own source never spells
/// any of them out as one literal substring (it would otherwise match its
/// own scan, the exact hole an exemption-by-filename would reopen: any
/// other file sharing this file's basename would then escape unchecked).
fn banned_names() -> Vec<String> {
    vec![
        ["Lowered", "SourceGraph"].concat(),
        ["Lowered", "Declaration"].concat(),
        ["lower_source", "_graph"].concat(),
    ]
}

/// Every regular `.rs` file under `root`, recursively, appended to `out`.
/// `root` itself is skipped (not an error) when it does not exist, matching
/// the FR-067-AC-6 scan over `benches/`, which this repository does not
/// have; the caller asserts a positive total count across every root
/// instead, so a renamed or emptied root cannot pass the scan vacuously.
fn files_under(root: &Path, out: &mut Vec<std::path::PathBuf>) {
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

#[trace("TC-168", "FR-067-AC-5", "FR-067-AC-6")]
#[test]
fn no_source_or_build_input_file_references_the_retired_seam() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let names = banned_names();
    let mut offending = Vec::new();
    let mut scanned = 0_usize;
    // `src/`, `tests/`, `xtask/`, `examples/` and `qsl-forms/` (the S2
    // forms crate, ADR-011 §7.3 X-5), not `src/` alone; `benches/` does not
    // exist in this repository, and `files_under` treats that as zero
    // files, not an error. This scan does not reach `spec/` or `docs/`,
    // which name these three symbols by design (FR-067-AC-6).
    for root in ["src", "tests", "benches", "xtask", "examples", "qsl-forms"] {
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
        "the scan found no `.rs` files at all under src/tests/xtask/examples; \
         a root was likely renamed or moved, which would let this check pass \
         vacuously"
    );
    assert!(
        offending.is_empty(),
        "SEAM-5's retired symbols still have source references: {offending:?}"
    );
}
