// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-257 (FR-088-AC-1): exactly one closed checked clause-kind enum is
//! defined, in the layer-3 `check` core.
//!
//! Reuses `xtask::definition_scan::scan_crate`'s existing whole-crate `syn`
//! scanner (built for TC-170/TC-171/TC-173's own "exactly one defining
//! location" claims) rather than a second, byte-identical copy of it.

use std::path::Path;

use ix_trace_rs::trace;

fn workspace_root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

/// Exactly one canonical clause-kind enum, in `check`.
#[trace("TC-257", "FR-088-AC-1")]
#[test]
fn exactly_one_checked_clause_kind_enum_is_defined_under_check() {
    let definitions = xtask::definition_scan::scan_crate(&workspace_root())
        .expect("the whole-crate definition scan runs cleanly");
    let locations = definitions
        .items
        .get("CheckedClauseKind")
        .cloned()
        .unwrap_or_default();
    assert_eq!(
        locations.len(),
        1,
        "CheckedClauseKind must be defined exactly once: {locations:?}"
    );
    assert!(
        locations[0].file.starts_with("qsl-semantics/src/check/"),
        "CheckedClauseKind's one definition must live under qsl-semantics/src/check/, found {:?}",
        locations[0].file
    );
}
