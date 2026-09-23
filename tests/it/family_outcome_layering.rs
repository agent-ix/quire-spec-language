// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-390 (FR-090-AC-9) and TC-386 step 3 (FR-090-AC-5): `FamilyOutcome`,
//! `FamilyResult` and `EvalOutcome` are defined once, in the layer-3 `check`
//! core (`qsl-semantics/src/family/`); no module below the core names them;
//! the core names no family cause type; no crate below layer 3 depends on
//! this crate; and `qsl-semantics`, layer 3 itself, depends on layers 2, F
//! and K only. Uses the resolved-import and definition scans of TC-256, TC-170 and
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
/// `qsl-cst`, layer 2 `qsl-forms`, and the layer-3 `semantic_value`, `model` and
/// `library` modules, plus the layer-3 `value` module that sits beside the
/// evaluator (`value::model_query`). `value::outcome` left this list under
/// QSL-131 O2, which deleted the module (it was layer K, a byte-identical
/// `quire_exact` copy). `value::reference` left it under QSL-181 X-6a, which
/// moved its `ObjectEnvironment` into `model` (`qsl-semantics/src/model`
/// covers it). QSL-181 (X-6b) moved every layer-3 module into
/// `qsl-semantics`.
const BELOW_CORE: [&str; 10] = [
    "qsl-cst/src",
    "qsl-forms/src",
    "qsl-semantics/src/model",
    "qsl-semantics/src/library",
    "qsl-semantics/src/value/definition.rs",
    "qsl-semantics/src/value/enumeration.rs",
    "qsl-semantics/src/value/unit.rs",
    "qsl-semantics/src/value/quantity.rs",
    "qsl-semantics/src/value/declaration.rs",
    "qsl-semantics/src/value/model_query.rs",
];

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

/// Step 1: exactly one definition of each type, under
/// `qsl-semantics/src/family/`.
#[trace("FR-090-AC-9", "TC-390")]
#[test]
fn family_outcome_types_are_defined_once_in_the_check_core() {
    let definitions = xtask::definition_scan::scan_dirs(
        &workspace_root(),
        &[
            "src",
            "quire-exact/src",
            "qsl-foundation/src",
            "qsl-cst/src",
            "qsl-forms/src",
            "qsl-semantics/src",
        ],
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
            locations[0].file.starts_with("qsl-semantics/src/family/"),
            "{name} must be defined under qsl-semantics/src/family/, found {:?}",
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
    let paths =
        xtask::import_graph::resolved_paths(&workspace_root(), &["qsl-semantics/src/family"])
            .expect("the resolved-import scan runs cleanly");
    assert!(
        !paths.is_empty(),
        "the scan found no paths under qsl-semantics/src/family"
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

/// One workspace package's normal (`[dependencies]`) and dev
/// (`[dev-dependencies]`) dependency names.
struct PackageDependencies {
    name: String,
    normal: Vec<String>,
    dev: Vec<String>,
    /// Each normal (and build) dependency with the features it enables.
    shipped_features: Vec<(String, Vec<String>)>,
    /// The package's own `default` feature list.
    default_features: Vec<String>,
    /// Every one of the package's own `[features]` entries, `default`
    /// included, with the values it enables.
    features: Vec<(String, Vec<String>)>,
}

/// Every workspace package's dependency names, from `cargo metadata`, so
/// `x.workspace = true` keys, `[dependencies.x]` tables and renamed
/// dependencies resolve to the real package name like any other entry.
fn workspace_dependencies() -> Vec<PackageDependencies> {
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
            let dependencies = package["dependencies"]
                .as_array()
                .expect("a dependency list");
            let names_of_kind = |kind: Option<&str>| -> Vec<String> {
                dependencies
                    .iter()
                    .filter(|dependency| dependency["kind"].as_str() == kind)
                    .map(|dependency| {
                        dependency["name"]
                            .as_str()
                            .expect("a dependency name")
                            .to_owned()
                    })
                    .collect()
            };
            let strings = |value: &serde_json::Value| -> Vec<String> {
                value
                    .as_array()
                    .map(|items| {
                        items
                            .iter()
                            .map(|item| item.as_str().expect("a feature name").to_owned())
                            .collect()
                    })
                    .unwrap_or_default()
            };
            PackageDependencies {
                name: package["name"].as_str().expect("a package name").to_owned(),
                normal: names_of_kind(None),
                dev: names_of_kind(Some("dev")),
                shipped_features: dependencies
                    .iter()
                    .filter(|dependency| dependency["kind"].as_str() != Some("dev"))
                    .map(|dependency| {
                        (
                            dependency["name"]
                                .as_str()
                                .expect("a dependency name")
                                .to_owned(),
                            strings(&dependency["features"]),
                        )
                    })
                    .collect(),
                default_features: strings(&package["features"]["default"]),
                features: package["features"]
                    .as_object()
                    .expect("a feature table")
                    .iter()
                    .map(|(name, values)| (name.clone(), strings(values)))
                    .collect(),
            }
        })
        .collect()
}

/// Step 4 of TC-390, step 3 of TC-386 and TC-398's crate edges: each crate
/// below layer 3 depends only on the workspace crates below it, in its
/// `[dependencies]` and its `[dev-dependencies]` alike. `qsl-semantics`
/// (layer 3, QSL-181) names exactly `qsl-forms`, `qsl-foundation` and
/// `quire-exact` among the workspace crates, never `qsl-cst`, and only
/// `model::intake` names its FCD dependencies. `quire-exact` names
/// none, `qsl-foundation` may name `quire-exact`, `qsl-cst` may name
/// `qsl-foundation` and `quire-exact`, and `qsl-forms` names exactly
/// `qsl-cst`, `qsl-foundation` and `quire-exact` in `[dependencies]` (ADR-011
/// §6.1; layer 2's cell names no external crate). `qsl-package` (layer 4,
/// QSL-182) names only `qsl-semantics`, `qsl-foundation` and `quire-exact`
/// among the workspace crates in `[dependencies]`, and `quire-contract-model`
/// as its one quire-ecosystem crate; its `[dev-dependencies]` may also name
/// the lower layer `qsl-forms`. Every other workspace crate, `qsl-source` and
/// this crate included, is refused. Cargo already
/// refuses a normal-dependency cycle back to this crate, but accepts a
/// dev-dependency one, so the dev table is checked here.
#[trace(
    "FR-090-AC-9",
    "TC-390",
    "FR-090-AC-5",
    "TC-386",
    "FR-091-AC-11",
    "TC-398"
)]
#[test]
fn no_crate_below_layer_three_depends_on_the_check_core() {
    let packages = workspace_dependencies();
    let workspace_crates: Vec<&str> = packages
        .iter()
        .map(|package| package.name.as_str())
        .collect();
    for required in [
        "quire-spec-language",
        "qsl-source",
        "qsl-cst",
        "qsl-forms",
        "qsl-semantics",
        "qsl-package",
    ] {
        assert!(
            workspace_crates.contains(&required),
            "cargo metadata lists no {required}: {workspace_crates:?}"
        );
    }
    for (crate_name, allowed, dev_only) in [
        ("quire-exact", &[][..], &[][..]),
        ("qsl-foundation", &["quire-exact"][..], &[][..]),
        ("qsl-cst", &["qsl-foundation", "quire-exact"][..], &[][..]),
        (
            "qsl-forms",
            &["qsl-cst", "qsl-foundation", "quire-exact"][..],
            &[][..],
        ),
        (
            "qsl-semantics",
            &["qsl-forms", "qsl-foundation", "quire-exact"][..],
            &[][..],
        ),
        (
            "qsl-package",
            &["qsl-foundation", "qsl-semantics", "quire-exact"][..],
            &["qsl-forms"][..],
        ),
    ] {
        let package = packages
            .iter()
            .find(|package| package.name == crate_name)
            .unwrap_or_else(|| panic!("cargo metadata lists no {crate_name}"));
        assert!(
            !package.normal.is_empty(),
            "{crate_name} lists no dependency"
        );
        if crate_name == "qsl-forms" {
            // Layer 2's "Depends on" cell names no external crate, so its
            // `[dependencies]` are exactly the three layer crates.
            let mut normal: Vec<&str> = package.normal.iter().map(String::as_str).collect();
            normal.sort_unstable();
            assert_eq!(normal, allowed, "{crate_name}'s [dependencies]");
        }
        if crate_name == "qsl-semantics" {
            // Layer 3 (QSL-181): its workspace-crate `[dependencies]` are
            // exactly layers 2, F and K -- no `qsl-cst` (layer 1) and no
            // root crate. Its other entries are third-party crates and the
            // FCD crates, which `fcd_is_named_by_model_intake_only` below
            // confines to `model::intake`.
            let mut workspace_normal: Vec<&str> = package
                .normal
                .iter()
                .map(String::as_str)
                .filter(|dependency| workspace_crates.contains(dependency))
                .collect();
            workspace_normal.sort_unstable();
            assert_eq!(workspace_normal, allowed, "{crate_name}'s [dependencies]");
            assert!(
                !package
                    .normal
                    .iter()
                    .any(|dependency| dependency == "qsl-cst"),
                "{crate_name} depends on layer-1 qsl-cst"
            );
            fcd_is_named_by_model_intake_only(&package.normal);
        }
        if crate_name == "qsl-package" {
            // Layer 4 (QSL-182): "3, F, K; `quire-contract-model` for v2 wire
            // constants and round-trip tests only". Layer 3 is required, and
            // the one quire-ecosystem crate outside the workspace is
            // `quire-contract-model`.
            assert!(
                package
                    .normal
                    .iter()
                    .any(|dependency| dependency == "qsl-semantics"),
                "{crate_name} does not depend on layer-3 qsl-semantics"
            );
            let ecosystem: Vec<&str> = package
                .normal
                .iter()
                .map(String::as_str)
                .filter(|dependency| {
                    !workspace_crates.contains(dependency)
                        && (dependency.starts_with("quire-") || dependency.starts_with("agent-ix-"))
                })
                .collect();
            assert_eq!(
                ecosystem,
                ["quire-contract-model"],
                "{crate_name}'s ecosystem [dependencies]"
            );
        }
        for (kind, dependencies, extra) in [
            ("normal", &package.normal, &[][..]),
            ("dev", &package.dev, dev_only),
        ] {
            for dependency in dependencies {
                assert!(
                    !workspace_crates.contains(&dependency.as_str())
                        || allowed.contains(&dependency.as_str())
                        || extra.contains(&dependency.as_str()),
                    "{crate_name} has a {kind} dependency on workspace crate {dependency}"
                );
            }
        }
    }
}

/// The FCD crates `qsl-semantics` depends on are named by
/// `qsl-semantics/src/model/intake.rs` and by no other file of that crate
/// (ADR-011 §6.1: FCD from `model::intake` only). A plain text search for
/// each crate's Rust name, so a `use`, an inline path and a macro argument
/// are all caught.
fn fcd_is_named_by_model_intake_only(dependencies: &[String]) {
    let fcd: Vec<String> = dependencies
        .iter()
        .filter(|dependency| dependency.starts_with("agent-ix-"))
        .map(|dependency| dependency.replace('-', "_"))
        .collect();
    assert!(!fcd.is_empty(), "qsl-semantics names no FCD crate");
    let root = workspace_root().join("qsl-semantics/src");
    let mut pending = vec![root.clone()];
    let mut naming = std::collections::BTreeSet::new();
    while let Some(dir) = pending.pop() {
        for entry in std::fs::read_dir(&dir).expect("the source tree lists") {
            let path = entry.expect("a directory entry").path();
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                let source = std::fs::read_to_string(&path).expect("a source file reads");
                if fcd.iter().any(|name| source.contains(name.as_str())) {
                    let relative = path.strip_prefix(&root).expect("under the root");
                    naming.insert(relative.to_string_lossy().replace('\\', "/"));
                }
            }
        }
    }
    assert_eq!(
        naming.into_iter().collect::<Vec<_>>(),
        ["model/intake.rs"],
        "files of qsl-semantics naming an FCD crate"
    );
}

/// Whether a `[features]` value forwards to a workspace crate's
/// `test-support`: `<crate>/test-support` or `<crate>?/test-support`.
fn forwards_to_test_support(value: &str, workspace_crates: &[&str]) -> bool {
    value.split_once('/').is_some_and(|(dependency, feature)| {
        feature == "test-support"
            && workspace_crates.contains(&dependency.strip_suffix('?').unwrap_or(dependency))
    })
}

/// QSL-181 (#371 review L8, #372 review M1): `test-support` turns on fixture
/// constructors that forge crate-issued capabilities
/// (`ReaderAuthority::fixture`) and test-only views of `pub(crate)` items.
/// It is for tests only:
///
/// - no workspace package enables its own `test-support` by default;
/// - no normal or build dependency edge enables it on a workspace crate (a
///   dev-dependency may, which is how a crate's own tests reach it);
/// - no `[features]` entry other than a package's own `test-support`
///   forwards to a workspace crate's `test-support` (`default =
///   ["qsl-semantics/test-support"]`, or any feature a shipped dependent
///   could enable). Forwarding `test-support` to `test-support` is allowed:
///   only a test build can turn the outer one on, by the two rules above.
///
/// Cargo feature unification would otherwise switch it on in a shipped
/// build. Measured: a root `default = ["qsl-semantics/test-support"]` fails
/// this test.
#[trace("FR-087-AC-1", "TC-243")]
#[test]
fn no_shipped_dependency_enables_test_support() {
    let packages = workspace_dependencies();
    let workspace_crates: Vec<&str> = packages
        .iter()
        .map(|package| package.name.as_str())
        .collect();
    assert!(workspace_crates.contains(&"quire-spec-language"));
    // The matcher sees both spellings, including the `?` one Cargo accepts
    // only for an optional dependency, and nothing else.
    for (value, forwards) in [
        ("qsl-semantics/test-support", true),
        ("qsl-semantics?/test-support", true),
        ("qsl-semantics/other", false),
        ("serde/test-support", false),
        ("dep:qsl-semantics", false),
    ] {
        assert_eq!(
            forwards_to_test_support(value, &workspace_crates),
            forwards,
            "{value}"
        );
    }
    for package in &packages {
        assert!(
            !package
                .default_features
                .iter()
                .any(|feature| feature == "test-support"),
            "{} enables test-support by default",
            package.name
        );
        for (feature, values) in &package.features {
            if feature == "test-support" {
                continue;
            }
            for value in values {
                assert!(
                    !forwards_to_test_support(value, &workspace_crates),
                    "{}'s feature `{feature}` enables {value}, which a shipped build can reach",
                    package.name
                );
            }
        }
        for (dependency, features) in &package.shipped_features {
            assert!(
                !(workspace_crates.contains(&dependency.as_str())
                    && features.iter().any(|feature| feature == "test-support")),
                "{} enables {dependency}/test-support through a normal or build dependency",
                package.name
            );
        }
    }
}
