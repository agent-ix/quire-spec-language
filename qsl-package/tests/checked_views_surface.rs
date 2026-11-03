// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-515-AC-8 (TC-1310 row 8): the two views have no public constructor and
//! none of `digest`, `bytes`, `identity`, `package_digest` or a closure-taking
//! method.
//!
//! The proof is the `compile_fail` doctests on `TemporalSubjectView` and
//! `CheckedPredicateView`, which `cargo test --doc` runs. This test binds
//! them: it fails when a doctest loses its pinned error code or its compiling
//! control, so a doctest cannot be dropped or weakened without a failing test
//! here.

use ix_trace_rs::trace;

fn source(relative: &str) -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(relative);
    std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

/// The fenced blocks of the `///` documentation of `text`: info string and body.
fn doc_blocks(text: &str) -> Vec<(String, String)> {
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

#[trace("FR-515-AC-8", "TC-1310")]
#[test]
fn each_view_pins_its_missing_methods_and_constructor_as_compile_fail() {
    for (file, view, accessor) in [
        (
            "src/checked_views/subject.rs",
            "TemporalSubjectView",
            ".declaration()",
        ),
        (
            "src/checked_views/predicate.rs",
            "CheckedPredicateView",
            ".declaration()",
        ),
    ] {
        let blocks = doc_blocks(&source(file));
        let failing = |code: &str, needle: &str| {
            blocks
                .iter()
                .filter(|(info, body)| info == code && body.contains(needle))
                .count()
        };
        assert_eq!(
            failing("compile_fail,E0451", &format!("{view} {{")),
            1,
            "{file}: the private-field construction"
        );
        for method in [
            ".digest()",
            ".bytes()",
            ".identity()",
            ".package_digest()",
            ".evaluate(|_|",
        ] {
            assert_eq!(
                failing("compile_fail,E0599", method),
                1,
                "{file}: {method} must be pinned as a missing method"
            );
        }
        assert!(
            blocks
                .iter()
                .any(|(info, body)| info.is_empty() && body.contains(accessor)),
            "{file}: the compiling control with a real accessor"
        );
    }
}
