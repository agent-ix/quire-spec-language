// SPDX-License-Identifier: AGPL-3.0-or-later
//! By-absence checks for the checked-package spine's dependencies on the
//! retained root compiler and protocol-artifact surfaces.

use ix_trace_rs::trace;
use std::path::{Path, PathBuf};

fn rust_sources(root: &Path) -> Vec<PathBuf> {
    let mut pending = vec![root.to_path_buf()];
    let mut files = Vec::new();
    while let Some(path) = pending.pop() {
        let entries = std::fs::read_dir(&path)
            .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
        for entry in entries {
            let path = entry.expect("directory entry").path();
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                files.push(path);
            }
        }
    }
    files.sort();
    files
}

fn mentions_symbol(source: &str, symbol: &str) -> bool {
    source.match_indices(symbol).any(|(start, _)| {
        start == 0
            || !source.as_bytes()[start - 1].is_ascii_alphanumeric()
                && source.as_bytes()[start - 1] != b'_'
    })
}

fn assert_absent(root: &Path, forbidden: &[&str]) {
    for path in rust_sources(root) {
        let source = std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("read {}: {error}", path.display()));
        for symbol in forbidden {
            assert!(
                !mentions_symbol(&source, symbol),
                "{} names retired boundary {symbol}",
                path.display()
            );
        }
    }
}

/// The spine producer/type crates do not route through the retained composed
/// parser, and the checked-package crates do not reach into root-owned
/// front-end or protocol-artifact modules. The root keeps those modules until
/// their separately gated deletion slices.
#[trace("FR-362-AC-6", "FR-493-AC-3", "FR-493-AC-4", "FR-499")]
#[test]
fn spine_crates_do_not_import_the_retained_root_frontend_or_protocol_reader() {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"));
    assert_absent(
        &workspace.join("qsl-replay/src/spine"),
        &[
            "parse_native",
            "parse_native_source",
            "syntax::composed",
            "linking::composed",
            "checking::composed",
        ],
    );
    for crate_name in ["qsl-package", "qsl-eval", "qsl-replay"] {
        assert_absent(
            &workspace.join(crate_name).join("src"),
            &[
                "DeclarationKind::Protocol",
                "Family::Protocol",
                "RegisteredDefinition::Protocol",
                "quire_spec_language::parser",
                "quire_spec_language::syntax",
                "quire_spec_language::linking::native",
                "quire_spec_language::checking",
                "quire_spec_language::model_source",
                "quire_spec_language::located_json",
                "quire_spec_language::protocol_artifact",
            ],
        );
    }
}

/// TC-1027 steps 2 and 3: after the composed lane is deleted, no source under
/// the root crate or any layer crate names its entry points or types, and none
/// of its source directories exist. The composed parser was the only parser
/// besides `qsl_cst::parse` that admitted a profile line with a version or
/// digest, so its absence is FR-362-AC-2's by-absence check.
#[trace("FR-362-AC-2", "FR-362-AC-6", "FR-499-AC-5")]
#[test]
fn no_source_names_the_composed_lane() {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut roots = vec![workspace.join("src"), workspace.join("xtask/src")];
    for entry in std::fs::read_dir(workspace).expect("read the workspace") {
        let path = entry.expect("directory entry").path();
        let is_layer = path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with("qsl-"));
        if is_layer && path.join("src").is_dir() {
            roots.push(path.join("src"));
        }
    }
    // Assembled so that this file does not name the symbols it forbids.
    let forbidden = [
        ["parse", "_native"].concat(),
        ["parse", "_native_source"].concat(),
        ["syntax::", "composed"].concat(),
        ["linking::", "composed"].concat(),
        ["checking::", "composed"].concat(),
    ];
    let forbidden: Vec<&str> = forbidden.iter().map(String::as_str).collect();
    for root in &roots {
        assert_absent(root, &forbidden);
    }
    for directory in ["parser", "syntax", "linking", "checking"] {
        let path = workspace.join("src").join(directory).join("composed");
        assert!(!path.exists(), "{} still exists", path.display());
        let file = workspace
            .join("src")
            .join(format!("{directory}/composed.rs"));
        assert!(!file.exists(), "{} still exists", file.display());
    }
}

/// FR-499-AC-4 (TC-1293): after QSL-499, `src/native_model` is absent and no
/// workspace source names the root crate's `native_model` module. TC-469's
/// spine corpus (`config_version_spine.rs`) is the other half of the criterion.
#[trace("FR-499-AC-4", "TC-1293")]
#[test]
fn the_native_model_module_is_absent() {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"));
    for path in ["src/native_model", "src/native_model.rs"] {
        assert!(
            !workspace.join(path).exists(),
            "{path} still exists after QSL-499"
        );
    }
    let mut roots = vec![workspace.join("src"), workspace.join("tests")];
    for entry in std::fs::read_dir(workspace).expect("read the workspace") {
        let path = entry.expect("directory entry").path();
        let is_layer = path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with("qsl-"));
        if is_layer && path.join("src").is_dir() {
            roots.push(path.join("src"));
        }
    }
    // Assembled so that this file does not name the paths it forbids.
    let forbidden = [
        ["crate::", "native_model"].concat(),
        ["quire_spec_language::", "native_model"].concat(),
    ];
    let forbidden: Vec<&str> = forbidden.iter().map(String::as_str).collect();
    for root in &roots {
        assert_absent(root, &forbidden);
    }
}

/// FR-499-AC-2, AC-3 and FR-515-AC-9: the compiled-protocol readers are
/// deleted, so no root source declares or names `protocol_artifact` (its
/// `wire`, `checked_predicate` and `temporal_subject` paths included), and no root source
/// names `DeclarationKind::Protocol`, `Family::Protocol`,
/// `RegisteredDefinition::Protocol` or `crate::temporal`. The registry that
/// listed the finite-global protocol definition is itself deleted (QSL-1810).
#[trace("FR-499-AC-2", "FR-499-AC-3", "FR-515-AC-9", "TC-1293", "TC-1310")]
#[test]
fn no_root_source_names_the_deleted_protocol_reader_or_protocol_definition() {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"));
    assert!(!workspace.join("src/protocol_artifact").exists());
    assert!(!workspace.join("src/protocol_artifact.rs").exists());
    // Assembled so that this file does not name the symbols it forbids.
    let forbidden = [
        ["protocol", "_artifact"].concat(),
        ["DeclarationKind::", "Protocol"].concat(),
        ["Family::", "Protocol"].concat(),
        ["RegisteredDefinition::", "Protocol"].concat(),
        ["crate::", "temporal"].concat(),
    ];
    let forbidden: Vec<&str> = forbidden.iter().map(String::as_str).collect();
    assert_absent(&workspace.join("src"), &forbidden);
    for entry in std::fs::read_dir(workspace).expect("read the workspace") {
        let path = entry.expect("directory entry").path();
        let is_layer = path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with("qsl-"));
        if is_layer && path.join("src").is_dir() {
            assert_absent(
                &path.join("src"),
                &["quire_spec_language::protocol_artifact"],
            );
        }
    }
}

/// Names in `directory` whose first dot-separated component is `stem`
/// (`parser.rs` and `parser/` for `parser`).
fn entries_named(directory: &Path, stem: &str) -> Vec<String> {
    std::fs::read_dir(directory)
        .unwrap_or_else(|error| panic!("read {}: {error}", directory.display()))
        .map(|entry| entry.expect("directory entry").file_name())
        .filter_map(|name| name.to_str().map(str::to_owned))
        .filter(|name| name.split('.').next() == Some(stem))
        .collect()
}

/// FR-493-AC-3, FR-499-AC-5 (QSL-1810): the root `0-draft` front end is
/// deleted. `src/parser*`, `src/syntax*`, `src/linking*` and `src/checking*`
/// do not exist and no root source or test names its entry points, types or
/// module paths. The positive control proves that the scan sees the retained
/// root modules and that the symbol check fires on a name that is present.
#[trace("FR-493-AC-3", "FR-499-AC-5", "TC-1276")]
#[test]
fn the_zero_draft_front_end_is_absent() {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"));
    let src = workspace.join("src");

    // Positive control: the same directory scan finds the retained modules.
    for retained in ["command", "complete", "formal_source"] {
        assert!(
            !entries_named(&src, retained).is_empty(),
            "the scan finds no retained src/{retained}: the by-absence check is vacuous"
        );
    }
    let lib = std::fs::read_to_string(src.join("lib.rs")).expect("read src/lib.rs");
    assert!(
        mentions_symbol(&lib, "formal_source"),
        "the symbol scan misses a name that src/lib.rs declares"
    );

    for stem in ["parser", "syntax", "linking", "checking"] {
        let found = entries_named(&src, stem);
        assert!(found.is_empty(), "src still has {found:?}");
    }

    // Assembled so that this file does not name the symbols it forbids.
    let forbidden = [
        ["Link", "Limits"].concat(),
        ["Linked", "Package"].concat(),
        ["Native", "Type"].concat(),
        ["Domain", "Type"].concat(),
        ["quire_spec_language::", "parse"].concat(),
        ["quire_spec_language::", "link"].concat(),
        ["quire_spec_language::", "syntax"].concat(),
        ["quire_spec_language::", "linking"].concat(),
        ["quire_spec_language::", "checking"].concat(),
        ["crate::", "syntax"].concat(),
        ["crate::", "linking"].concat(),
        ["crate::", "checking"].concat(),
        ["crate::", "parser"].concat(),
    ];
    let forbidden: Vec<&str> = forbidden.iter().map(String::as_str).collect();
    assert_absent(&src, &forbidden);
    assert_absent(&workspace.join("xtask/src"), &forbidden);
    for path in rust_sources(&workspace.join("tests")) {
        if path.ends_with("tests/it/spine_deletion_boundaries.rs") {
            continue;
        }
        let source = std::fs::read_to_string(&path).expect("read test source");
        for symbol in &forbidden {
            assert!(
                !mentions_symbol(&source, symbol),
                "{} names the deleted front end {symbol}",
                path.display()
            );
        }
    }
}
