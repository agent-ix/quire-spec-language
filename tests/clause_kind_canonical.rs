// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-257 (FR-088-AC-1): exactly one closed checked clause-kind enum is
//! defined, in the layer-3 `check` core, and the lane-private
//! `syntax::ClauseKind` (native-v1) is unchanged by FR-088 -- it gains no
//! new variant, and no consumer added by this requirement uses it as if it
//! were canonical (ADR-013 R-09).
//!
//! Reuses `xtask::definition_scan::scan_crate`'s existing whole-crate `syn`
//! scanner (built for TC-170/TC-171/TC-173's own "exactly one defining
//! location" claims) rather than a second, byte-identical copy of it.
//!
//! **Scope of what step 4 (no new consumer) actually checks**, following
//! `definition_scan`'s own "Scope of what this catches" convention: this
//! test asserts `syntax::ClauseKind`'s own variant list is byte-for-byte
//! the fixed list below (step 3, rigorous: any variant added, renamed or
//! reordered fails it) and that the literal fully-qualified spelling
//! `syntax::ClauseKind` (the only spelling a consumer outside `syntax.rs`
//! itself would need to name it, since it is not re-exported through any
//! aggregate) appears in exactly as many `.rs` files as it does today (six,
//! all pre-existing and unrelated to this requirement -- measured, see the
//! test's own comment, not assumed). It
//! does not resolve a bare, unqualified `ClauseKind` back to which of this
//! crate's three same-named enums a given use site names (that needs full
//! type resolution, out of scope for a `syn`-only scan, same as
//! `definition_scan`'s own documented limits) -- a change that adds a
//! consumer through a bare `use crate::syntax::ClauseKind;` import without
//! ever spelling the qualified path is outside this test's own bound, and
//! is exactly what a source-diff at PR review time still needs to check.

use std::path::Path;

use ix_trace_rs::trace;

fn workspace_root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

/// Steps 1-2: exactly one canonical clause-kind enum, in `check`.
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
        locations[0].file.starts_with("src/check/"),
        "CheckedClauseKind's one definition must live under src/check/, found {:?}",
        locations[0].file
    );
}

/// Step 3: `syntax::ClauseKind`'s variant list, read from the live source
/// file, is byte-for-byte the fixed list below -- any addition, removal,
/// rename or reorder fails this step and names the mismatch.
#[trace("TC-257", "FR-088-AC-1")]
#[test]
fn syntax_clause_kind_variant_list_is_unchanged() {
    let path = workspace_root().join("src/syntax.rs");
    let source = std::fs::read_to_string(&path).expect("src/syntax.rs reads");
    let parsed = syn::parse_file(&source).expect("src/syntax.rs parses as Rust");
    let clause_kind = parsed
        .items
        .iter()
        .find_map(|item| match item {
            syn::Item::Enum(item_enum) if item_enum.ident == "ClauseKind" => Some(item_enum),
            _ => None,
        })
        .expect("src/syntax.rs defines a top-level ClauseKind enum");
    let variants: Vec<String> = clause_kind
        .variants
        .iter()
        .map(|variant| variant.ident.to_string())
        .collect();
    assert_eq!(
        variants,
        vec![
            "Invariant".to_owned(),
            "Precondition".to_owned(),
            "Postcondition".to_owned(),
        ],
        "syntax::ClauseKind gained, lost or reordered a variant -- FR-088-CON-5 forbids this"
    );
}

/// Step 4 (bounded, see this file's own doc): the literal fully-qualified
/// spelling `syntax::ClauseKind` occurs in exactly as many `.rs` files
/// under `src/` as it did when this test was written -- a plain count, so
/// a new file spelling it out (a new consumer treating the lane-private
/// enum as canonical) is caught, within this scan's own documented bound.
#[trace("TC-257", "FR-088-AC-1")]
#[test]
fn syntax_clause_kind_qualified_spelling_gains_no_new_consumer_file() {
    let root = workspace_root().join("src");
    let mut files_naming_it = Vec::new();
    let mut pending = vec![root];
    while let Some(dir) = pending.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                pending.push(path);
                continue;
            }
            if path.extension().and_then(|extension| extension.to_str()) != Some("rs") {
                continue;
            }
            let Ok(contents) = std::fs::read_to_string(&path) else {
                continue;
            };
            if contents.contains("syntax::ClauseKind") {
                files_naming_it.push(path);
            }
        }
    }
    // Six pre-existing, already-reviewed occurrences, measured against the
    // live tree rather than assumed: `runtime/validation/inventory.rs` and
    // `checking/bindings.rs` (`use crate::syntax::ClauseKind;`),
    // `runtime/validation/values.rs` (same), `package/view.rs` and
    // `protocol_artifact/native/layout.rs` (both match on its variants
    // directly), and `check/identity.rs` (this requirement's own module
    // doc, explaining why it does *not* touch this type). None of these
    // was added by this requirement -- FR-088 adds no new one.
    assert_eq!(
        files_naming_it.len(),
        6,
        "unexpected change in files naming syntax::ClauseKind by its qualified path: {files_naming_it:?}"
    );
}
