// SPDX-License-Identifier: AGPL-3.0-or-later
//! Actual Make recipe routing, separate from real extraction qualification.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use ix_trace_rs::trace;

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
    &["test", "--locked", "--workspace"],
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
    &["test", "--locked", "-p", "qsl-semantics"],
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
    &["test", "--locked", "-p", "qsl-cst"],
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
    &["test", "--locked", "--workspace", "--all-features"],
];

fn workspace() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap()
}

fn cargo_double() -> PathBuf {
    static DIRECTORY: std::sync::OnceLock<tempfile::TempDir> = std::sync::OnceLock::new();
    let directory = DIRECTORY.get_or_init(|| {
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
    });
    directory.path().join("cargo")
}

struct Recipe {
    directory: tempfile::TempDir,
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
        std::os::unix::fs::symlink(cargo_double(), directory.path().join("bin/cargo")).unwrap();
        Self { directory }
    }

    fn run(&self, targets: &[&str], root: Option<&str>, overrides: &[&str]) -> String {
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
        let output = make.output().unwrap();
        assert!(
            output.status.success(),
            "actual Make recipes failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        fs::read_to_string(log).unwrap()
    }
}

fn expected(root: &str, commands: &[&[&str]]) -> String {
    commands
        .iter()
        .map(|args| format!("{root}\t{}\n", args.join("\t")))
        .collect()
}

#[test]
fn every_feature_recipe_uses_its_child_and_preserves_complete_argv() {
    let recipe = Recipe::new();
    for root in [
        None,
        Some(""),
        Some("132-target"),
        Some("owned target"),
        Some("/tmp/owned-target"),
        Some("/tmp/owned target"),
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
}

#[test]
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
}

#[test]
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
fn aggregate_reaches_default_then_all_features_without_changing_recipe_order() {
    let recipe = Recipe::new();
    let other_checks = [
        "check-no-committed-binaries",
        "check-index-completeness",
        "ci-clean-build",
        "seam-probe",
        "string-edge",
        "route-lint",
        "checked-input",
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
        recipe.run(&["ci"], Some("caller target"), &overrides),
        expected("caller target/ci-default-features", DEFAULT_COMMANDS)
            + &expected("caller target/ci-all-features", ALL_COMMANDS)
    );
}

#[test]
#[trace("FR-042-AC-15", "FR-050-AC-8")]
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
