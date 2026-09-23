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

/// Every workspace package's name and its normal (`[dependencies]`)
/// dependency names, from `cargo metadata`, so `x.workspace = true` keys and
/// `[dependencies.x]` tables resolve like any other entry.
fn workspace_dependencies() -> Vec<(String, Vec<String>)> {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned());
    let output = std::process::Command::new(cargo)
        .current_dir(workspace_root())
        .args([
            "metadata",
            "--format-version",
            "1",
            "--no-deps",
            "--offline",
        ])
        .output()
        .expect("cargo metadata runs");
    assert!(
        output.status.success(),
        "cargo metadata failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let metadata: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("cargo metadata emits JSON");
    metadata["packages"]
        .as_array()
        .expect("cargo metadata lists packages")
        .iter()
        .map(|package| {
            let name = package["name"].as_str().expect("a package name").to_owned();
            let dependencies = package["dependencies"]
                .as_array()
                .expect("a dependency list")
                .iter()
                .filter(|dependency| dependency["kind"].is_null())
                .map(|dependency| {
                    dependency["name"]
                        .as_str()
                        .expect("a dependency name")
                        .to_owned()
                })
                .collect();
            (name, dependencies)
        })
        .collect()
}

/// Step 4 of TC-390, and step 3 of TC-386: each crate below layer 3 depends
/// only on the workspace crates below it. `quire-exact` names none,
/// `qsl-foundation` may name `quire-exact`, and `qsl-cst` may name
/// `qsl-foundation` and `quire-exact` (ADR-011 §6.1). Every other workspace
/// crate, `qsl-source` and this crate included, is refused.
#[trace("FR-090-AC-9", "TC-390", "FR-090-AC-5", "TC-386")]
#[test]
fn no_crate_below_layer_three_depends_on_the_check_core() {
    let packages = workspace_dependencies();
    let workspace_crates: Vec<&str> = packages.iter().map(|(name, _)| name.as_str()).collect();
    for required in ["quire-spec-language", "qsl-source", "qsl-cst"] {
        assert!(
            workspace_crates.contains(&required),
            "cargo metadata lists no {required}: {workspace_crates:?}"
        );
    }
    for (crate_name, allowed) in [
        ("quire-exact", &[][..]),
        ("qsl-foundation", &["quire-exact"][..]),
        ("qsl-cst", &["qsl-foundation", "quire-exact"][..]),
    ] {
        let (_, dependencies) = packages
            .iter()
            .find(|(name, _)| name == crate_name)
            .unwrap_or_else(|| panic!("cargo metadata lists no {crate_name}"));
        assert!(!dependencies.is_empty(), "{crate_name} lists no dependency");
        for dependency in dependencies {
            assert!(
                !workspace_crates.contains(&dependency.as_str())
                    || allowed.contains(&dependency.as_str()),
                "{crate_name} depends on workspace crate {dependency}"
            );
        }
    }
}
