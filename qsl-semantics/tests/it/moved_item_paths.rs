// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-400-AC-5 for the moved `DeclarationKey` and `AdmittedIntegerDivision`:
//! the old `qsl_semantics::` paths do not resolve, and the new owner's do.
//!
//! The proof that an old path fails to compile is the `compile_fail`
//! doctests in `model::key` and `value::definition`, which `cargo test --doc`
//! runs. This test binds them: it fails when a doctest loses its pinned
//! error code or its compiling `no_run` pair, so the old path cannot be
//! re-exported without a failing test here.

use ix_trace_rs::trace;
use quire_checked::integer_division::AdmittedIntegerDivision;
use quire_checked::key::DeclarationKey;

fn source(relative: &str) -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(relative);
    std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

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

#[test]
#[trace("FR-400-AC-5")]
fn the_moved_item_paths_are_pinned_as_unresolvable_with_a_compiling_new_owner() {
    let cases = [
        (
            "src/model/key.rs",
            "E0603",
            vec!["use qsl_semantics::model::key::DeclarationKey;"],
            "use quire_checked::key::DeclarationKey;",
        ),
        (
            "src/value/definition.rs",
            "E0432",
            vec![
                "use qsl_semantics::value::AdmittedIntegerDivision;",
                "use qsl_semantics::value::definition::AdmittedIntegerDivision;",
            ],
            "use quire_checked::integer_division::AdmittedIntegerDivision;",
        ),
    ];
    for (file, code, old_paths, new_path) in cases {
        let blocks = doc_blocks(&source(file));
        for old in old_paths {
            assert!(
                blocks.iter().any(|(info, body)| {
                    info == &format!("compile_fail,{code}") && body.trim() == old
                }),
                "{file}: no `compile_fail,{code}` doctest for `{old}`"
            );
        }
        assert!(
            blocks
                .iter()
                .any(|(info, body)| info == "no_run" && body.contains(new_path)),
            "{file}: no compiling `no_run` pair for `{new_path}`"
        );
    }
    let _ = (
        std::mem::size_of::<DeclarationKey>(),
        std::mem::size_of::<AdmittedIntegerDivision>(),
    );
}
