// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-515-AC-9 (TC-1310 row 9): `protocol_artifact::checked_predicate` and
//! `protocol_artifact::temporal_subject` do not resolve.
//!
//! The proof is the `compile_fail` doctests in `protocol_artifact`, which
//! `cargo test --doc` runs. This test binds them: it fails when a doctest
//! loses its pinned error code or its compiling control, so the retired paths
//! cannot return without a failing test here.

use ix_trace_rs::trace;

fn doc_blocks(text: &str) -> Vec<(String, String)> {
    let mut blocks = Vec::new();
    let mut open: Option<(String, String)> = None;
    for line in text.lines() {
        let Some(doc) = line.strip_prefix("//!") else {
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

#[trace("FR-515-AC-9", "TC-1310")]
#[test]
fn the_retired_module_paths_are_pinned_as_unresolvable_with_a_compiling_control() {
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/protocol_artifact/mod.rs");
    let blocks = doc_blocks(&std::fs::read_to_string(path).expect("the module source"));
    for retired in [
        "use quire_spec_language::protocol_artifact::checked_predicate;",
        "use quire_spec_language::protocol_artifact::temporal_subject;",
    ] {
        assert_eq!(
            blocks
                .iter()
                .filter(|(info, body)| info == "compile_fail,E0432" && body.trim() == retired)
                .count(),
            1,
            "{retired} is pinned as unresolvable"
        );
    }
    assert!(
        blocks.iter().any(|(info, body)| info.is_empty()
            && body.trim() == "use quire_spec_language::protocol_artifact::wire;"),
        "the control path compiles"
    );
}
