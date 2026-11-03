// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-527-AC-11 and FR-527-AC-14 (TC-1322 rows 11 and 14): `mapping_support`
//! takes no decision-scope closure, and `AuthenticatedSelection` cannot be
//! constructed outside the classifier.
//!
//! The proof is the `compile_fail` doctests in `src/checked_views/mapping.rs`,
//! which `cargo test --doc` runs. These tests bind them: each fails when a
//! doctest loses its pinned error code or its subject, so a doctest cannot be
//! dropped or weakened without a failing test here.

use ix_trace_rs::trace;

/// The fenced blocks of the `///` documentation of `src/checked_views/mapping.rs`:
/// info string and body.
fn doc_blocks() -> Vec<(String, String)> {
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/checked_views/mapping.rs");
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    let mut blocks = Vec::new();
    let mut open: Option<(String, String)> = None;
    for line in text.lines() {
        let Some(doc) = line.trim_start().strip_prefix("///") else {
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

fn count(code: &str, needle: &str) -> usize {
    doc_blocks()
        .iter()
        .filter(|(info, body)| info == code && body.contains(needle))
        .count()
}

/// FR-527-AC-11: the doctest that passes a local decision-scope enum to
/// `mapping_support` is pinned as a type mismatch.
#[trace("FR-527-AC-11", "TC-1322")]
#[test]
fn a_decision_scope_closure_is_pinned_as_a_compile_fail() {
    assert_eq!(
        count(
            "compile_fail,E0308",
            "mapping_support(view, DecisionScope::Open)"
        ),
        1
    );
    assert_eq!(count("compile_fail,E0308", "enum DecisionScope"), 1);
}

/// FR-527-AC-14: the doctests that build `AuthenticatedSelection` by its
/// fields and through a constructor are pinned as private-field and missing
/// function errors.
#[trace("FR-527-AC-14", "TC-1322")]
#[test]
fn constructing_the_selection_is_pinned_as_a_compile_fail() {
    assert_eq!(
        count("compile_fail,E0451", "AuthenticatedSelection {"),
        1,
        "the private-field construction"
    );
    assert_eq!(
        count("compile_fail,E0599", "AuthenticatedSelection::new("),
        1,
        "the constructor call"
    );
}
