// SPDX-License-Identifier: AGPL-3.0-or-later
//! NFR-013 / TC-915: actual Make routing, separate from real extraction qualification.

use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use ix_trace_rs::trace;

use crate::feature_scope::{skipped, KeepReason, Verdict};
use crate::test_suites::{all_features_decisions, read_tree_features};

// Independent complete argv expectations, including standalone default checks.
const DEFAULT_COMMANDS: &[&[&str]] = &[
    &["fmt", "--all", "--", "--check"],
    &[
        "clippy",
        "--locked",
        "--workspace",
        "--all-targets",
        "--",
        "-D",
        "warnings",
    ],
    &[
        "run",
        "--locked",
        "--package",
        "xtask",
        "--",
        "test-suites",
        "default-workspace",
        "--locked",
        "--workspace",
    ],
    &[
        "clippy",
        "--locked",
        "-p",
        "qsl-semantics",
        "--all-targets",
        "--",
        "-D",
        "warnings",
    ],
    &[
        "run",
        "--locked",
        "--package",
        "xtask",
        "--",
        "test-suites",
        "default-qsl-semantics",
        "--locked",
        "-p",
        "qsl-semantics",
    ],
    &[
        "clippy",
        "--locked",
        "-p",
        "qsl-cst",
        "--all-targets",
        "--",
        "-D",
        "warnings",
    ],
    &[
        "run",
        "--locked",
        "--package",
        "xtask",
        "--",
        "test-suites",
        "default-qsl-cst",
        "--locked",
        "-p",
        "qsl-cst",
    ],
];
const ALL_COMMANDS: &[&[&str]] = &[
    &[
        "clippy",
        "--locked",
        "--workspace",
        "--all-targets",
        "--all-features",
        "--",
        "-D",
        "warnings",
    ],
    &[
        "run",
        "--locked",
        "--package",
        "xtask",
        "--",
        "test-suites",
        "all-features-workspace",
        "--locked",
        "--workspace",
        "--all-features",
    ],
];

fn workspace() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap()
}

fn cargo_double() -> tempfile::TempDir {
    let target = std::env::var_os("CARGO_TARGET_DIR")
        .filter(|root| !root.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| workspace().join("target"));
    let target = workspace().join(target);
    fs::create_dir_all(&target).unwrap();
    let directory = tempfile::tempdir_in(target).unwrap();
    let output = Command::new("rustc")
        .args([
            "--edition=2021",
            "--crate-name",
            "feature_lane_cargo_double",
            "-D",
            "warnings",
        ])
        .arg(workspace().join("xtask/src/ci_feature_lane_tests/cargo_double.rs"))
        .arg("-o")
        .arg(directory.path().join("cargo"))
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "compile native Cargo process double: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    directory
}

struct Recipe {
    directory: tempfile::TempDir,
    // All child processes have been joined before this owner is dropped.
    cargo_directory: tempfile::TempDir,
}

impl Recipe {
    fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        fs::copy(
            workspace().join("Makefile"),
            directory.path().join("Makefile"),
        )
        .unwrap();
        fs::create_dir(directory.path().join("bin")).unwrap();
        let cargo_directory = cargo_double();
        std::os::unix::fs::symlink(
            cargo_directory.path().join("cargo"),
            directory.path().join("bin/cargo"),
        )
        .unwrap();
        Self {
            directory,
            cargo_directory,
        }
    }

    fn run(&self, targets: &[&str], root: Option<&str>, overrides: &[&str]) -> String {
        let output = self.run_raw(targets, root, overrides, None);
        assert!(
            output.status.success(),
            "actual Make recipes failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        fs::read_to_string(self.directory.path().join("calls")).unwrap()
    }

    fn run_raw(
        &self,
        targets: &[&str],
        root: Option<&str>,
        overrides: &[&str],
        list_count: Option<&str>,
    ) -> std::process::Output {
        let log = self.directory.path().join("calls");
        if log.exists() {
            fs::remove_file(&log).unwrap();
        }
        let mut make = Command::new("make");
        make.current_dir(self.directory.path())
            .args(["--no-print-directory", "-rR"])
            .args(targets)
            .args(overrides)
            .env_remove("MAKEFLAGS")
            .env_remove("MFLAGS")
            .env_remove("MAKEFILES")
            .env_remove("CARGO_TARGET_DIR")
            .env_remove("CI_DEFAULT_TARGET_DIR")
            .env_remove("CI_ALL_TARGET_DIR")
            .env_remove("CI_CLEAN_TARGET_DIR")
            .env("FEATURE_LANE_PATH_VALUE", "expanded-canary")
            .env("FEATURE_LANE_CALL_LOG", &log)
            .env(
                "PATH",
                std::env::join_paths(
                    std::iter::once(self.directory.path().join("bin"))
                        .chain(std::env::split_paths(&std::env::var_os("PATH").unwrap())),
                )
                .unwrap(),
            );
        if let Some(root) = root {
            make.env("CARGO_TARGET_DIR", root);
        }
        if let Some(count) = list_count {
            make.env("FEATURE_LANE_LIST_COUNT", count);
        }
        make.output().unwrap()
    }
}

fn expected(root: &str, commands: &[&[&str]]) -> String {
    commands
        .iter()
        .map(|args| format!("{root}\t{}\n", args.join("\t")))
        .collect()
}

const DEEP_THREADS: &str = "--test-threads=3";

const DEEP_CASES: [&str; 5] = [
    "formatter_identity::deep_sum_source_generator_keeps_format_identity",
    "formatter_identity::deep_else_if_source_generator_keeps_format_identity",
    "formatter_identity::deep_let_source_generator_keeps_format_identity",
    "formatter_identity::replay_large_source_generator_keeps_format_identity",
    "formatter_identity::dependency_chain_and_order_variants_keep_format_identity",
];

/// The deep-format calls in recipe order: one exact listing per case, then the
/// five cases in one run at `DEEP_THREADS`, all in the default lane's directory.
fn deep_calls(directory: &str) -> String {
    let listings: Vec<Vec<&str>> = DEEP_CASES
        .iter()
        .map(|&case| {
            vec![
                "test",
                "--locked",
                "--workspace",
                "--tests",
                "--",
                "--ignored",
                "--exact",
                case,
                "--list",
            ]
        })
        .collect();
    let mut run = vec![
        "test",
        "--locked",
        "--workspace",
        "--tests",
        "--",
        "--ignored",
        "--exact",
        DEEP_THREADS,
    ];
    run.extend(DEEP_CASES);
    let mut commands: Vec<&[&str]> = listings.iter().map(Vec::as_slice).collect();
    commands.push(&run);
    expected(directory, &commands)
}

#[test]
#[trace(
    "TC-915",
    "NFR-013-AC-1",
    "NFR-013-AC-2",
    "NFR-013-AC-3",
    "NFR-013-M-1"
)]
fn every_feature_recipe_uses_its_child_and_preserves_complete_argv() {
    let recipe = Recipe::new();
    for root in [
        None,
        Some(""),
        Some("132-target"),
        Some("owned target"),
        Some("/tmp/owned-target"),
        Some("/tmp/owned target"),
        Some("owned \"target\""),
        Some("/tmp/owned`literal-backtick`"),
        Some("owned ${FEATURE_LANE_PATH_VALUE}"),
        Some("owned $(FEATURE_LANE_PATH_VALUE)"),
        Some("/tmp/owned $FEATURE_LANE_PATH_VALUE"),
        Some("owned 'single quote' \\ slash; * ? [x]"),
    ] {
        let parent = root.filter(|root| !root.is_empty()).unwrap_or("target");
        let calls = recipe.run(&["ci-default-features", "ci-all-features"], root, &[]);
        assert_eq!(
            calls,
            expected(&format!("{parent}/ci-default-features"), DEFAULT_COMMANDS)
                + &expected(&format!("{parent}/ci-all-features"), ALL_COMMANDS),
            "each lane must use its isolated child for every command"
        );
    }
    let helper_directory = recipe.cargo_directory.path().to_path_buf();
    assert!(helper_directory.join("cargo").is_file());
    drop(recipe);
    assert!(
        !helper_directory.exists(),
        "fixture teardown removes its compiled helper"
    );
}

#[test]
#[trace("TC-915", "NFR-013-AC-2", "NFR-013-AC-3")]
fn explicit_roots_and_lane_overrides_are_one_quoted_argument() {
    let recipe = Recipe::new();
    assert_eq!(
        recipe.run(
            &["ci-default-features", "ci-all-features"],
            Some("ignored"),
            &["CARGO_TARGET_DIR=caller target"]
        ),
        expected("caller target/ci-default-features", DEFAULT_COMMANDS)
            + &expected("caller target/ci-all-features", ALL_COMMANDS)
    );
    assert_eq!(
        recipe.run(
            &["ci-default-features", "ci-all-features"],
            Some("caller"),
            &[
                "CI_DEFAULT_TARGET_DIR=default override",
                "CI_ALL_TARGET_DIR=/tmp/all override"
            ]
        ),
        expected("default override", DEFAULT_COMMANDS)
            + &expected("/tmp/all override", ALL_COMMANDS)
    );
    for path in [
        "caller target",
        "caller \"quote\"",
        "caller`literal-backtick`",
        "caller ${FEATURE_LANE_PATH_VALUE}",
        "caller $(FEATURE_LANE_PATH_VALUE)",
        "caller $FEATURE_LANE_PATH_VALUE",
        "caller 'quote' \\ slash; * ? [x]",
    ] {
        // Command::args passes the literal path directly to Make. The recipe
        // exports it as shell data, so neither Make nor the shell expands it.
        let root = format!("CARGO_TARGET_DIR={path}");
        assert_eq!(
            recipe.run(
                &["ci-default-features", "ci-all-features"],
                Some("ignored"),
                &[&root]
            ),
            expected(&format!("{path}/ci-default-features"), DEFAULT_COMMANDS)
                + &expected(&format!("{path}/ci-all-features"), ALL_COMMANDS)
        );
        let default = format!("CI_DEFAULT_TARGET_DIR={path}");
        let all_path = format!("/tmp/{path}");
        let all = format!("CI_ALL_TARGET_DIR={all_path}");
        assert_eq!(
            recipe.run(
                &["ci-default-features", "ci-all-features"],
                Some("ignored"),
                &[&default, &all]
            ),
            expected(path, DEFAULT_COMMANDS) + &expected(&all_path, ALL_COMMANDS)
        );
    }
}

#[test]
#[trace("TC-915", "NFR-013-AC-1", "NFR-013-AC-3")]
fn deep_formatter_target_runs_each_full_size_case_and_is_in_ci() {
    let recipe = Recipe::new();
    assert_eq!(
        recipe.run(&["test-deep-format"], None, &[]),
        deep_calls("target/ci-default-features"),
        "the full-size depth and volume regressions run once, in the default lane's directory"
    );
    assert_eq!(
        recipe.run(&["test-deep-format"], Some("caller target"), &[]),
        deep_calls("caller target/ci-default-features"),
        "a caller root selects the default lane's child, not a directory of its own"
    );
    assert_eq!(
        recipe.run(
            &["test-deep-format"],
            Some("caller target"),
            &["CI_DEFAULT_TARGET_DIR=default override"]
        ),
        deep_calls("default override"),
        "the default lane override moves the deep target with its lane"
    );

    let makefile = fs::read_to_string(recipe.directory.path().join("Makefile")).unwrap();
    let ci_checks = makefile
        .lines()
        .find(|line| line.starts_with("CI_CHECKS :="))
        .expect("the aggregate CI check list exists");
    assert!(
        ci_checks
            .split_whitespace()
            .any(|name| name == "test-deep-format"),
        "the named full-size target is included in make ci"
    );

    for count in ["0", "2"] {
        let output = recipe.run_raw(&["test-deep-format"], None, &[], Some(count));
        assert!(
            !output.status.success(),
            "the deep target must reject {count} selected tests"
        );
        assert!(
            String::from_utf8_lossy(&output.stderr).contains(&format!("found {count}")),
            "the failure reports the selected count"
        );
    }
}

#[test]
#[trace("TC-915", "NFR-013-AC-1", "NFR-013-AC-2", "NFR-013-AC-3")]
fn routing_oracle_rejects_safe_path_and_command_mutations() {
    let recipe = Recipe::new();
    let makefile = recipe.directory.path().join("Makefile");
    let original = fs::read_to_string(&makefile).unwrap();
    let root = "caller \"quote\" ${FEATURE_LANE_PATH_VALUE}";
    let oracle = expected(&format!("{root}/ci-default-features"), DEFAULT_COMMANDS)
        + &expected(&format!("{root}/ci-all-features"), ALL_COMMANDS);
    assert_eq!(
        recipe.run(&["ci-default-features", "ci-all-features"], Some(root), &[]),
        oracle
    );
    for (before, after) in [
        (
            "CARGO_TARGET_DIR=\"$${CI_DEFAULT_TARGET_DIR:-$${CARGO_TARGET_DIR:-target}/ci-default-features}\" cargo fmt --all -- --check",
            "CARGO_TARGET_DIR=\"$${CI_DEFAULT_TARGET_DIR:-$${CARGO_TARGET_DIR:-target}}\" cargo fmt --all -- --check",
        ),
        (
            "CARGO_TARGET_DIR=\"$${CI_DEFAULT_TARGET_DIR:-$${CARGO_TARGET_DIR:-target}/ci-default-features}\" cargo fmt --all -- --check",
            "CARGO_TARGET_DIR=\"$${CI_DEFAULT_TARGET_DIR:-$${CARGO_TARGET_DIR:-target}/ci-default-features-mutant}\" cargo fmt --all -- --check",
        ),
        (
            "CARGO_TARGET_DIR=\"$${CI_DEFAULT_TARGET_DIR:-$${CARGO_TARGET_DIR:-target}/ci-default-features}\" cargo run --locked --package xtask -- test-suites default-workspace --locked --workspace\n",
            "CARGO_TARGET_DIR=\"$${CI_ALL_TARGET_DIR:-$${CARGO_TARGET_DIR:-target}/ci-all-features}\" cargo run --locked --package xtask -- test-suites all-features-workspace --locked --workspace --all-features\n",
        ),
    ] {
        // Mutate only the fixture copy, retaining safe shell-variable expansion.
        // No control ever runs path contents through the unsafe reviewed recipes.
        assert_eq!(
            original.matches(before).count(),
            1,
            "one intended mutation site"
        );
        fs::write(&makefile, original.replacen(before, after, 1)).unwrap();
        let observed = recipe.run(&["ci-default-features", "ci-all-features"], Some(root), &[]);
        assert_ne!(
            observed, oracle,
            "complete literal-path/argv oracle rejects the mutant"
        );
        fs::write(&makefile, &original).unwrap();
        assert_eq!(
            fs::read_to_string(&makefile).unwrap(),
            original,
            "exact fixture restore"
        );
        assert_eq!(
            recipe.run(&["ci-default-features", "ci-all-features"], Some(root), &[]),
            oracle
        );
    }
}

#[test]
#[trace("TC-915", "NFR-013-AC-1", "NFR-013-AC-3")]
fn repeated_switches_in_both_directions_keep_each_lane_directory() {
    let recipe = Recipe::new();
    for targets in [
        ["ci-default-features", "ci-all-features"],
        ["ci-all-features", "ci-default-features"],
    ] {
        for _ in 0..2 {
            for lane in targets {
                let (child, commands) = match lane {
                    "ci-default-features" => ("ci-default-features", DEFAULT_COMMANDS),
                    "ci-all-features" => ("ci-all-features", ALL_COMMANDS),
                    _ => unreachable!(),
                };
                assert_eq!(
                    recipe.run(&[lane], Some("warm caller root"), &[]),
                    expected(&format!("warm caller root/{child}"), commands)
                );
            }
        }
    }
}

#[test]
#[trace("TC-915", "NFR-013-AC-1", "NFR-013-AC-3")]
fn aggregate_reaches_default_then_all_features_without_changing_recipe_order() {
    let recipe = Recipe::new();
    let other_checks = [
        "check-conformance-input",
        "conformance",
        "check-no-committed-binaries",
        "ci-clean-build",
        "seam-probe",
        "string-edge",
        "route-lint",
        "checked-input",
        "refinement-versioning",
        "cargo-deny-bans",
        "ci-docs",
        "arch-lint-canonical-encoder",
        "arch-lint-api-surface-qsl",
        "arch-lint-qualified-core",
    ];
    let overrides: Vec<&str> = other_checks
        .iter()
        .flat_map(|target| ["-o", *target])
        .collect();
    assert_eq!(
        recipe.run(&["ci-internal"], Some("caller target"), &overrides),
        expected("caller target/ci-default-features", DEFAULT_COMMANDS)
            + &expected("caller target/ci-all-features", ALL_COMMANDS)
            + &deep_calls("caller target/ci-default-features")
    );
}

#[test]
#[trace("TC-915", "NFR-013-AC-3", "FR-042-AC-15", "FR-050-AC-8")]
fn clean_and_core_tooling_retain_their_own_target_and_features() {
    let recipe = Recipe::new();
    assert_eq!(
        recipe.run(
            &[
                "ci-clean-build",
                "string-edge",
                "route-lint",
                "checked-input"
            ],
            Some("caller target"),
            &[]
        ),
        expected(
            "caller target",
            &[
                &[
                    "build",
                    "--locked",
                    "--workspace",
                    "--no-default-features",
                    "--target-dir",
                    "caller target/clean"
                ],
                &[
                    "check",
                    "--locked",
                    "-p",
                    "quire-spec-language",
                    "--lib",
                    "--no-default-features",
                    "--features",
                    "handoff-writer",
                    "--target-dir",
                    "caller target/clean"
                ],
                &[
                    "run",
                    "--locked",
                    "--no-default-features",
                    "--",
                    "parse",
                    "agent-ix",
                    "test:parent",
                    "fixture",
                    "fixture:1",
                    "tests/fixtures/parent.native"
                ],
                &["run", "--package", "xtask", "--", "string-edge"],
                &["run", "--package", "xtask", "--", "route-lint"],
                &["run", "--package", "xtask", "--", "checked-input"],
            ]
        )
    );
}

/// QSL-825: the all-features lane's package scope, from the real workspace's
/// `cargo metadata`. Prints the decision table (`--nocapture`).
#[test]
#[trace("TC-915", "NFR-013-AC-4", "NFR-013-AC-5")]
fn real_workspace_decisions_keep_every_package_whose_compile_changes() {
    let decisions = all_features_decisions(workspace(), &[OsString::from("--locked")])
        .expect("the workspace resolves in both feature selections");
    for decision in &decisions {
        println!("{decision}");
    }
    let skip = skipped(&decisions);
    println!("--exclude {}", skip.join(" --exclude "));
    // The invariant the exclusions rest on: with them left out, every package
    // that stays resolves to exactly the feature sets of the full
    // `--all-features` workspace, host and target units apart.
    let arguments = [OsString::from("--locked")];
    let full = read_tree_features(workspace(), &arguments, true, &[]).expect("full tree");
    let without =
        read_tree_features(workspace(), &arguments, true, &skip).expect("tree without skips");
    assert_eq!(without.differences(&full), Vec::<String>::new());
    // And each skipped package compiles on the same units in the default run.
    let default = read_tree_features(workspace(), &arguments, false, &[]).expect("default tree");
    for name in &skip {
        assert!(default.compiles_same_as(&full, name), "{name}");
    }
    let verdict = |name: &str| {
        &decisions
            .iter()
            .find(|decision| decision.package == name)
            .unwrap_or_else(|| panic!("{name} is a workspace package"))
            .verdict
    };
    // Optional features no default build turns on.
    assert_eq!(
        verdict("quire-spec-language"),
        &Verdict::Run(KeepReason::OwnFeatures)
    );
    assert_eq!(
        verdict("qsl-source"),
        &Verdict::Run(KeepReason::OwnFeatures)
    );
    // No optional features of its own, but a dependency's features change.
    assert_eq!(
        verdict("qsl-bench"),
        &Verdict::Run(KeepReason::DependencyResolvesDifferently {
            dependency: "qsl-replay".to_owned()
        })
    );
}
