// SPDX-License-Identifier: AGPL-3.0-or-later
//! Process-boundary checks for the local conformance recipes, not vector evidence.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const CHECKS: [(&str, &str); 12] = [
    ("check::node_key::tests::conformance_fr322_application_keys_match_qspec_operation_vectors", "conformance: 1 of 1 QSpec operation vectors match (process double)"),
    ("check::node_key::tests::conformance_fr092_nominal_enum_keys_match_qspec_vectors", "conformance: 2 nominal enum vectors match (process double)"),
    ("quantities::tc_411_compound_unit_ids_match_qspec_vectors", "conformance: 1 compound-unit vectors"),
    ("checked_v2::tests::conformance_c14_source_map_lookup_over_qspec_positive_fixtures", "conformance: 1 source-map entries over 1 positive fixtures"),
    ("checked_v2::tests::conformance_i2_read_over_qspec_checked_package_v2_fixtures", "conformance: 1 positive fixtures through QSL's full I2 read\nconformance: 1 adverse mutations refused"),
    ("checked_v2::tests::conformance_fr340_frame_mutations_match_qspec_vectors", "conformance: 1 frame-body mutation vectors matched"),
    ("checked_v2::tests::conformance_dependency_selection_vectors", "conformance: 1 dependency-selection entry mutations and 1 order vectors"),
    ("emit::tests::golden::conformance_emitted_application_nodes_match_qspec_positive_fixtures", "conformance: 1 emitted application nodes match QSpec's positive fixtures"),
    ("check::profile::tests::conformance_fr110_profile_causes_are_listed_by_qspec_native_diagnostics", "conformance: 1 profile (code, cause) pairs listed by QSpec"),
    ("library::bundle_tests::conformance_fr111_resolution_causes_are_listed_by_qspec_native_diagnostics", "conformance: 1 bundle (code, cause) pairs listed by QSpec"),
    ("complete_value_lock::conformance_catalog_matches_qspec_complete_value_lock", "conformance: 1 catalog rows match QSpec's lock"),
    ("complete_value_lock::conformance_admit_selection_matches_qspec_selection_vectors", "conformance: 1 accepted and 1 refused selection vectors"),
];

// The launcher only forwards process context. All double behavior lives in Rust.
/// Trace: FR-092-AC-8
#[test]
fn cargo_double() {
    let Ok(log) = std::env::var("CONFORMANCE_DOUBLE_LOG") else {
        return;
    };
    let args = std::env::var("CONFORMANCE_DOUBLE_ARGS").unwrap();
    let mut words = args.split_whitespace();
    words.find(|word| *word == "--exact").expect("exact flag");
    let selection = words.next().expect("exact selection");
    let index = CHECKS
        .iter()
        .position(|(name, _)| *name == selection)
        .expect("required selection");
    assert!(args.starts_with("test --locked -p "));
    assert!(args.contains(" -- --exact "));
    assert!(args.ends_with(" --nocapture"));
    let mut calls = fs::read_to_string(&log).unwrap_or_default();
    calls.push_str(&format!(
        "{}\t{}\n",
        selection,
        std::env::var("QSPEC_DIR").unwrap()
    ));
    fs::write(log, calls).unwrap();
    println!("selected: {selection}");
    let fail_at = std::env::var("CONFORMANCE_DOUBLE_FAIL_AT").unwrap();
    let mode = if fail_at == index.to_string() {
        std::env::var("CONFORMANCE_DOUBLE_MODE").unwrap()
    } else {
        "positive".to_owned()
    };
    match mode.as_str() {
        "no-summary" => {}
        "zero-vectors" => {
            println!("conformance: 0 of 0 QSpec operation vectors match (process double)")
        }
        _ => println!("{}", CHECKS[index].1),
    }
    match mode.as_str() {
        "zero-tests" => println!(
            "test result: ok. 0 passed; 0 failed; 0 ignored; 1 filtered out; finished in 0.00s"
        ),
        "ignored" => println!(
            "test result: ok. 0 passed; 0 failed; 1 ignored; 0 filtered out; finished in 0.00s"
        ),
        _ => println!(
            "test result: ok. 1 passed; 0 failed; 0 ignored; 0 filtered out; finished in 0.00s"
        ),
    }
    if mode == "skip" {
        println!("skipped: QSPEC_DIR not set");
    }
    std::process::exit(if mode == "nonzero" { 23 } else { 0 });
}

struct Recipe {
    root: tempfile::TempDir,
    checkout: PathBuf,
    log: PathBuf,
}

impl Recipe {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        fs::copy(workspace.join("Makefile"), root.path().join("Makefile")).unwrap();
        // A linked worktree's common Git directory points back to the main clone.
        let common = root.path().join("main-clone/.git");
        fs::create_dir_all(&common).unwrap();
        let git = Command::new("git")
            .args(["init", "--quiet"])
            .arg(common.parent().unwrap())
            .output()
            .unwrap();
        assert!(git.status.success());
        let worktree_git = common.join("worktrees/recipe");
        fs::create_dir_all(&worktree_git).unwrap();
        fs::write(worktree_git.join("commondir"), "../..\n").unwrap();
        fs::write(worktree_git.join("HEAD"), "ref: refs/heads/recipe\n").unwrap();
        fs::write(
            root.path().join(".git"),
            format!("gitdir: {}\n", worktree_git.display()),
        )
        .unwrap();
        let bin = root.path().join("bin");
        fs::create_dir(&bin).unwrap();
        let executable = std::env::current_exe().unwrap();
        let quoted_executable = executable.to_str().unwrap().replace('\'', "'\\''");
        fs::write(bin.join("cargo"), format!("#!/bin/sh\nexport CONFORMANCE_DOUBLE_ARGS=\"$*\"\nexec '{quoted_executable}' --exact ci_conformance_tests::cargo_double --nocapture\n")).unwrap();
        fs::set_permissions(bin.join("cargo"), fs::Permissions::from_mode(0o755)).unwrap();
        let checkout = root.path().join("quire-specification");
        Self::checkout(&checkout);
        let log = root.path().join("calls");
        Self {
            root,
            checkout,
            log,
        }
    }

    fn checkout(path: &Path) {
        let vectors = path.join("proposals/checked-package-v2");
        fs::create_dir_all(&vectors).unwrap();
        // A path sentinel only: the process double does not read private vectors.
        fs::write(vectors.join("node-identity-vectors.json"), "").unwrap();
    }

    fn run(&self, target: &str, override_dir: Option<&Path>, mode: &str, fail_at: usize) -> Output {
        let mut make = Command::new("make");
        make.current_dir(self.root.path())
            .args(["--no-print-directory", "-rR", target])
            .env_remove("QSPEC_DIR")
            .env_remove("SIBLINGS")
            .env_remove("MAKEFLAGS")
            .env_remove("MFLAGS")
            .env_remove("MAKEFILES")
            .env(
                "PATH",
                std::env::join_paths(
                    std::iter::once(self.root.path().join("bin"))
                        .chain(std::env::split_paths(&std::env::var_os("PATH").unwrap())),
                )
                .unwrap(),
            )
            .env("CONFORMANCE_DOUBLE_LOG", &self.log)
            .env("CONFORMANCE_DOUBLE_MODE", mode)
            .env("CONFORMANCE_DOUBLE_FAIL_AT", fail_at.to_string());
        if let Some(path) = override_dir {
            make.env("QSPEC_DIR", path);
        }
        // Run the real ci dependency graph, confining other lanes to their own tests.
        if target == "ci" {
            for prerequisite in [
                "check-no-committed-binaries",
                "check-index-completeness",
                "ci-default-features",
                "ci-all-features",
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
            ] {
                make.args(["-o", prerequisite]);
            }
        }
        make.output().unwrap()
    }

    fn assert_calls(&self, checkout: &Path) {
        let expected: String = CHECKS
            .iter()
            .map(|(name, _)| format!("{name}\t{}\n", checkout.display()))
            .collect();
        assert_eq!(fs::read_to_string(&self.log).unwrap_or_default(), expected, "local ci must execute all twelve exact conformance selections with the resolved checkout");
    }
}

/// Trace: FR-092-AC-8
#[test]
fn local_ci_runs_all_conformance_checks_from_a_linked_worktree() {
    let recipe = Recipe::new();
    let output = recipe.run("ci", None, "positive", 0);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    recipe.assert_calls(&recipe.checkout);
}

/// Trace: FR-092-AC-8
#[test]
fn caller_checkout_override_is_used_and_exported() {
    let recipe = Recipe::new();
    fs::remove_dir_all(&recipe.checkout).unwrap();
    let override_dir = recipe.root.path().join("caller checkout");
    Recipe::checkout(&override_dir);
    for path in [override_dir.as_path(), Path::new("caller checkout")] {
        let _ = fs::remove_file(&recipe.log);
        let output = recipe.run("ci", Some(path), "positive", 0);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        recipe.assert_calls(&override_dir);
    }
    let exported = Command::new("make")
        .current_dir(recipe.root.path())
        .args([
            "--no-print-directory",
            "-f",
            "Makefile",
            "-f",
            "-",
            "export-probe",
        ])
        .arg(format!("QSPEC_DIR={}", override_dir.display()))
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    use std::io::Write;
    let mut exported = exported;
    exported
        .stdin
        .take()
        .unwrap()
        .write_all(b"export-probe:\n\t@printenv QSPEC_DIR\n")
        .unwrap();
    let output = exported.wait_with_output().unwrap();
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap().trim(),
        override_dir.to_str().unwrap()
    );
}

/// Trace: FR-092-AC-8
#[test]
fn missing_empty_and_invalid_checkouts_fail_before_cargo() {
    for kind in ["missing-default", "empty", "missing-override", "invalid"] {
        let recipe = Recipe::new();
        let invalid = recipe.root.path().join("invalid");
        fs::create_dir(&invalid).unwrap();
        let override_dir = match kind {
            "missing-default" => {
                fs::remove_dir_all(&recipe.checkout).unwrap();
                None
            }
            "empty" => Some(Path::new("")),
            "missing-override" => Some(Path::new("absent")),
            "invalid" => Some(invalid.as_path()),
            _ => unreachable!(),
        };
        let output = recipe.run("ci", override_dir, "positive", 0);
        assert!(!output.status.success(), "{kind} must fail local ci");
        assert!(String::from_utf8_lossy(&output.stderr).contains("conformance:"));
        assert!(!recipe.log.exists(), "{kind} must fail before Cargo runs");
    }
}

/// Trace: FR-092-AC-8
#[test]
fn every_required_selection_rejects_zero_ignored_skip_missing_summary_and_failure() {
    for (index, (selection, _)) in CHECKS.iter().enumerate() {
        for mode in ["zero-tests", "ignored", "skip", "no-summary", "nonzero"] {
            let recipe = Recipe::new();
            let output = recipe.run("conformance", None, mode, index);
            assert!(
                !output.status.success(),
                "selection {index} must reject {mode}"
            );
            let calls = fs::read_to_string(&recipe.log).unwrap();
            assert_eq!(
                calls.lines().count(),
                index + 1,
                "must stop at the refusing selection"
            );
            assert!(
                String::from_utf8_lossy(&output.stdout).contains(selection),
                "the required test must actually have been selected"
            );
        }
    }
}

/// Trace: FR-092-AC-8
#[test]
fn operation_summary_requires_positive_vector_counts() {
    let recipe = Recipe::new();
    let output = recipe.run("conformance", None, "zero-vectors", 0);
    assert!(
        !output.status.success(),
        "a zero-vector operation summary must fail"
    );
    assert!(String::from_utf8_lossy(&output.stderr).contains("vector check did not run"));
}
