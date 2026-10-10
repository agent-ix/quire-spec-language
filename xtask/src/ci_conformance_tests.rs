// SPDX-License-Identifier: AGPL-3.0-or-later
//! Process-boundary checks for the local conformance recipes, not vector evidence.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use ix_trace_rs::trace;

mod checks;
use checks::CHECKS;

fn cargo_double() -> PathBuf {
    static EXECUTABLE: std::sync::OnceLock<tempfile::TempDir> = std::sync::OnceLock::new();
    let directory = EXECUTABLE.get_or_init(|| {
        let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        let target = std::env::var_os("CARGO_TARGET_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| workspace.join("target"));
        let target = if target.is_absolute() {
            target
        } else {
            workspace.join(target)
        };
        fs::create_dir_all(&target).unwrap();
        let directory = tempfile::tempdir_in(target).unwrap();
        let output = Command::new("rustc")
            .args(["--edition=2021", "--crate-name", "conformance_cargo_double", "-D", "warnings"])
            .arg(workspace.join("xtask/src/ci_conformance_tests/cargo_double.rs"))
            .arg("-o")
            .arg(directory.path().join("cargo"))
            .output()
            .unwrap();
        assert!(output.status.success(), "compile Rust Cargo process double: {}", String::from_utf8_lossy(&output.stderr));
        directory
    });
    directory.path().join("cargo")
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
        std::os::unix::fs::symlink(cargo_double(), bin.join("cargo")).unwrap();
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

#[trace("FR-092-AC-8", "TC-413")]
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

#[trace("FR-092-AC-8", "TC-413")]
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
        .arg("QSPEC_DIR=caller checkout")
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

#[trace("FR-092-AC-8", "TC-413")]
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

#[trace("FR-092-AC-8", "TC-413")]
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

#[trace("FR-092-AC-8", "TC-413")]
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
