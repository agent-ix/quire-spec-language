// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-390 (FR-090-AC-9) and TC-386 step 3 (FR-090-AC-5): `FamilyOutcome`,
//! `FamilyResult` and `EvalOutcome` are defined once, in the layer-3 `check`
//! core (`src/family/`); no module below the core names them; the core
//! names no family cause type; and no crate below layer 3 depends on this
//! crate. Uses the resolved-import and definition scans of TC-256, TC-170 and
//! TC-176 (`xtask::import_graph`, `xtask::definition_scan`), so a
//! fully-qualified inline path is caught as well as a `use` line.

use std::path::{Path, PathBuf};

use ix_trace_rs::trace;

const CORE_TYPES: [&str; 3] = ["FamilyOutcome", "FamilyResult", "EvalOutcome"];

/// The family cause types, which the `check` core holds only through
/// `CatalogCoded`/`UndefinedCoded`.
const CAUSE_TYPES: [&str; 4] = [
    "ProtocolClauseSnapshot",
    "ModelRefusal",
    "ModelQueryRefusal",
    "StateModelUndefined",
];

/// The modules ordered below the `check` core (ADR-011 §6.1, §6.2): layer 1
/// `qsl-cst`, layer 2 `forms`, and the layer-3 `semantic_value`, `model` and
/// `library` modules, plus the layer-K and layer-3 `value` modules that sit
/// beside the evaluator (`value::outcome`, `value::model_query`).
const BELOW_CORE: [&str; 12] = [
    "qsl-cst/src",
    "src/forms",
    "src/model",
    "src/library",
    "src/value/definition.rs",
    "src/value/enumeration.rs",
    "src/value/unit.rs",
    "src/value/quantity.rs",
    "src/value/key.rs",
    "src/value/reference.rs",
    "src/value/model_query.rs",
    "src/value/outcome.rs",
];

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

/// Step 1: exactly one definition of each type, under `src/family/`.
#[trace("FR-090-AC-9", "TC-390")]
#[test]
fn family_outcome_types_are_defined_once_in_the_check_core() {
    let definitions = xtask::definition_scan::scan_dirs(
        &workspace_root(),
        &["src", "quire-exact/src", "qsl-foundation/src"],
    )
    .expect("the definition scan runs cleanly");
    for name in CORE_TYPES {
        let locations = definitions.items.get(name).cloned().unwrap_or_default();
        assert_eq!(
            locations.len(),
            1,
            "{name} must be defined exactly once: {locations:?}"
        );
        assert!(
            locations[0].file.starts_with("src/family/"),
            "{name} must be defined under src/family/, found {:?}",
            locations[0].file
        );
    }
}

/// Step 2: no `use` edge or inline path below the core names one of the
/// three types, or globs the `family` module.
#[trace("FR-090-AC-9", "TC-390")]
#[test]
fn no_module_below_the_check_core_names_a_family_outcome_type() {
    for root in BELOW_CORE {
        assert!(
            workspace_root().join(root).exists(),
            "{root} does not exist"
        );
    }
    let paths = xtask::import_graph::resolved_paths(&workspace_root(), &BELOW_CORE)
        .expect("the resolved-import scan runs cleanly");
    assert!(!paths.is_empty(), "the scan found no paths at all");
    let offending: Vec<_> = paths
        .iter()
        .filter(|path| {
            path.segments
                .iter()
                .any(|segment| CORE_TYPES.contains(&segment.as_str()))
                || (path.is_glob && path.segments.last().is_some_and(|last| last == "family"))
        })
        .collect();
    assert!(
        offending.is_empty(),
        "a module below the check core names a family outcome type: {offending:#?}"
    );
}

/// Step 3: no `use` edge or inline path under the core names a family
/// cause type.
#[trace("FR-090-AC-9", "TC-390")]
#[test]
fn the_check_core_names_no_family_cause_type() {
    let paths = xtask::import_graph::resolved_paths(&workspace_root(), &["src/family"])
        .expect("the resolved-import scan runs cleanly");
    assert!(
        !paths.is_empty(),
        "the scan found no paths under src/family"
    );
    let offending: Vec<_> = paths
        .iter()
        .filter(|path| {
            path.segments
                .iter()
                .any(|segment| CAUSE_TYPES.contains(&segment.as_str()))
        })
        .collect();
    assert!(
        offending.is_empty(),
        "the check core names a family cause type: {offending:#?}"
    );
}

/// The crate names a manifest's `[dependencies]` table lists.
fn dependencies(manifest: &str) -> Vec<String> {
    let path = workspace_root().join(manifest);
    let text = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    let mut in_dependencies = false;
    let mut names = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_dependencies = line == "[dependencies]";
            continue;
        }
        if in_dependencies && !line.is_empty() && !line.starts_with('#') {
            if let Some((name, _)) = line.split_once('=') {
                names.push(name.trim().to_owned());
            }
        }
    }
    names
}

/// Step 4 of TC-390, and step 3 of TC-386: each crate below layer 3 names
/// only the workspace crates below it. `quire-exact` names none,
/// `qsl-foundation` may name `quire-exact`, and `qsl-cst` may name
/// `qsl-foundation` and `quire-exact` (ADR-011 §6.1).
#[trace("FR-090-AC-9", "TC-390", "FR-090-AC-5", "TC-386")]
#[test]
fn no_crate_below_layer_three_depends_on_the_check_core() {
    let workspace_crates = [
        "quire-spec-language",
        "quire-exact",
        "qsl-foundation",
        "qsl-cst",
        "qsl-attrs",
        "qsl-replay",
        "xtask",
        "arch-lint",
    ];
    for (manifest, allowed) in [
        ("quire-exact/Cargo.toml", &[][..]),
        ("qsl-foundation/Cargo.toml", &["quire-exact"][..]),
        ("qsl-cst/Cargo.toml", &["qsl-foundation", "quire-exact"][..]),
    ] {
        let names = dependencies(manifest);
        assert!(!names.is_empty(), "{manifest} lists no dependency");
        for name in names {
            assert!(
                !workspace_crates.contains(&name.as_str()) || allowed.contains(&name.as_str()),
                "{manifest} depends on workspace crate {name}"
            );
        }
    }
}
