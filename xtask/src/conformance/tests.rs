// SPDX-License-Identifier: AGPL-3.0-or-later
//! Process-boundary checks with synthetic Cargo output, not QSpec vector evidence.

use super::{resolve, run_ci_with, run_with, CHECKS, VECTOR};
use crate::error::Error;
use std::ffi::OsStr;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::process::Stdio;

fn cargo_double() -> PathBuf {
    static EXECUTABLE: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();
    EXECUTABLE
        .get_or_init(|| {
            let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
            let target = std::env::var_os("CARGO_TARGET_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|| workspace.join("target"));
            let target = if target.is_absolute() {
                target
            } else {
                workspace.join(target)
            };
            let directory = target.join("ci-conformance-process-double");
            fs::create_dir_all(&directory).unwrap();
            let executable = directory.join("cargo");
            let output = Command::new("rustc")
                .args([
                    "--edition=2021",
                    "--crate-name",
                    "conformance_cargo_double",
                    "-D",
                    "warnings",
                ])
                .arg(workspace.join("xtask/tests/fixtures/ci_conformance_cargo_double.rs"))
                .arg("-o")
                .arg(&executable)
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            executable
        })
        .clone()
}

fn checkout(path: &Path) {
    let vector = path.join(VECTOR);
    fs::create_dir_all(vector.parent().unwrap()).unwrap();
    fs::write(vector, "").unwrap();
}

fn run_case(root: &Path, input: &OsStr, mode: &str, fail_at: usize) -> Result<String, Error> {
    let log = root.join("calls");
    let index = fail_at.to_string();
    run_with(
        &cargo_double(),
        root,
        Some(input),
        &[
            ("CONFORMANCE_DOUBLE_LOG", log.as_os_str()),
            ("CONFORMANCE_DOUBLE_MODE", OsStr::new(mode)),
            ("CONFORMANCE_DOUBLE_FAIL_AT", OsStr::new(&index)),
        ],
    )
}

fn assert_calls(root: &Path, checkout: &Path, count: usize) {
    let actual = fs::read_to_string(root.join("calls")).unwrap();
    let expected = CHECKS
        .iter()
        .take(count)
        .map(|check| format!("{}\t{}\n", check.selection, checkout.display()))
        .collect::<String>();
    assert_eq!(actual, expected);
}

#[test]
fn default_and_caller_checkouts_with_spaces_and_dollars_reach_every_child_absolute() {
    let root = crate::test_support::tempdir();
    let workspace = root.path();
    let checkout_path = workspace.join("main clone parent with spaces/$qspec checkout");
    checkout(&checkout_path);
    run_case(workspace, checkout_path.as_os_str(), "positive", usize::MAX).unwrap();
    assert_calls(workspace, &checkout_path, CHECKS.len());
    fs::remove_file(workspace.join("calls")).unwrap();
    let relative = Path::new("main clone parent with spaces/$qspec checkout");
    run_case(workspace, relative.as_os_str(), "positive", usize::MAX).unwrap();
    assert_calls(workspace, &checkout_path, CHECKS.len());
    fs::remove_file(workspace.join("calls")).unwrap();
    let alias = workspace.join("linked $qspec");
    std::os::unix::fs::symlink(&checkout_path, &alias).unwrap();
    run_case(workspace, alias.as_os_str(), "positive", usize::MAX).unwrap();
    assert_calls(workspace, &checkout_path, CHECKS.len());
}

#[test]
fn missing_empty_and_invalid_checkouts_refuse_before_cargo() {
    let root = crate::test_support::tempdir();
    let workspace = root.path();
    let invalid = workspace.join("invalid");
    fs::create_dir(&invalid).unwrap();
    for input in [
        None,
        Some(OsStr::new("")),
        Some(OsStr::new("absent")),
        Some(invalid.as_os_str()),
    ] {
        assert!(matches!(
            resolve(input, workspace),
            Err(Error::ConformanceInput { .. })
        ));
    }
    assert!(!workspace.join("calls").exists());
}

#[test]
fn each_selection_refuses_missing_evidence_or_failure() {
    for (index, _) in CHECKS.iter().enumerate() {
        for mode in ["zero-tests", "ignored", "skip", "no-summary", "nonzero"] {
            let root = crate::test_support::tempdir();
            let checkout_path = root.path().join("quire-specification");
            checkout(&checkout_path);
            assert!(matches!(
                run_case(root.path(), checkout_path.as_os_str(), mode, index),
                Err(Error::ConformanceCheck { .. })
            ));
            assert_calls(root.path(), &checkout_path, index + 1);
        }
    }
}

#[test]
fn zero_operation_vectors_refuse() {
    let root = crate::test_support::tempdir();
    let checkout_path = root.path().join("quire-specification");
    checkout(&checkout_path);
    assert!(matches!(
        run_case(root.path(), checkout_path.as_os_str(), "zero-vectors", 0),
        Err(Error::ConformanceCheck { .. })
    ));
}

#[test]
fn make_exports_raw_default_and_command_line_override() {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    for override_value in [None, Some("caller checkout $qspec with spaces")] {
        let mut command = Command::new("make");
        command
            .current_dir(workspace)
            .args([
                "--no-print-directory",
                "-rR",
                "-f",
                "Makefile",
                "-f",
                "-",
                "export-probe",
            ])
            .env_remove("QSPEC_DIR")
            .env_remove("SIBLINGS")
            .env_remove("MAKEFLAGS")
            .env_remove("MFLAGS")
            .env_remove("MAKEFILES")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped());
        if let Some(value) = override_value {
            command.arg(format!("QSPEC_DIR={value}"));
        }
        let mut child = command.spawn().unwrap();
        child
            .stdin
            .take()
            .unwrap()
            .write_all(b"export-probe:\n\t@printenv QSPEC_DIR\n")
            .unwrap();
        let output = child.wait_with_output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let exported = String::from_utf8(output.stdout).unwrap();
        if let Some(value) = override_value {
            assert_eq!(exported.trim_end(), value);
        } else {
            let path = Path::new(exported.trim_end());
            assert!(path.is_absolute() && path.ends_with("quire-specification"));
        }
    }
}

#[test]
fn make_exports_default_checkout_from_a_main_clone_parent_with_spaces() {
    let root = crate::test_support::tempdir();
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    fs::copy(source.join("Makefile"), root.path().join("Makefile")).unwrap();
    let siblings = root.path().join("main clone parent with spaces");
    let main = siblings.join("main-clone");
    fs::create_dir_all(&main).unwrap();
    let init = Command::new("git")
        .args(["init", "--quiet"])
        .arg(&main)
        .output()
        .unwrap();
    assert!(init.status.success());
    let worktree_git = main.join(".git/worktrees/recipe");
    fs::create_dir_all(&worktree_git).unwrap();
    fs::write(worktree_git.join("commondir"), "../..\n").unwrap();
    fs::write(worktree_git.join("HEAD"), "ref: refs/heads/recipe\n").unwrap();
    fs::write(
        root.path().join(".git"),
        format!("gitdir: {}\n", worktree_git.display()),
    )
    .unwrap();
    let mut child = Command::new("make")
        .current_dir(root.path())
        .args([
            "--no-print-directory",
            "-rR",
            "-f",
            "Makefile",
            "-f",
            "-",
            "export-probe",
        ])
        .env_remove("QSPEC_DIR")
        .env_remove("SIBLINGS")
        .env_remove("MAKEFLAGS")
        .env_remove("MFLAGS")
        .env_remove("MAKEFILES")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"export-probe:\n\t@printenv QSPEC_DIR\n")
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap().trim_end(),
        siblings.join("quire-specification").to_str().unwrap()
    );
}

#[test]
fn ordinary_ci_cargo_lane_inherits_canonical_checkout() {
    let root = crate::test_support::tempdir();
    let workspace = root.path();
    let source = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    fs::copy(source.join("Makefile"), workspace.join("Makefile")).unwrap();
    let checkout_path = workspace.join("real checkout $qspec");
    checkout(&checkout_path);
    let alias = workspace.join("linked checkout");
    std::os::unix::fs::symlink(&checkout_path, &alias).unwrap();

    let bin = workspace.join("bin");
    fs::create_dir(&bin).unwrap();
    let cargo = bin.join("cargo");
    let output = Command::new("rustc")
        .args([
            "--edition=2021",
            "--crate-name",
            "ci_cargo_double",
            "-D",
            "warnings",
        ])
        .arg(source.join("xtask/src/conformance/ci_cargo_double.rs"))
        .arg("-o")
        .arg(&cargo)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let path = std::env::join_paths(
        std::iter::once(bin).chain(std::env::split_paths(&std::env::var_os("PATH").unwrap())),
    )
    .unwrap();
    let log = workspace.join("ordinary-cargo-calls");
    let skip = [
        "-o",
        "check-conformance-input",
        "-o",
        "conformance",
        "-o",
        "check-no-committed-binaries",
        "-o",
        "ci-all-features",
        "-o",
        "ci-clean-build",
        "-o",
        "seam-probe",
        "-o",
        "string-edge",
        "-o",
        "route-lint",
        "-o",
        "checked-input",
        "-o",
        "refinement-versioning",
        "-o",
        "cargo-deny-bans",
        "-o",
        "test-deep-format",
        "-o",
        "ci-docs",
        "-o",
        "arch-lint-canonical-encoder",
        "-o",
        "arch-lint-api-surface-qsl",
        "-o",
        "arch-lint-qualified-core",
    ];
    run_ci_with(
        Path::new("make"),
        workspace,
        Some(Path::new("linked checkout").as_os_str()),
        &skip,
        &[
            ("PATH", path.as_os_str()),
            ("CI_CARGO_DOUBLE_LOG", log.as_os_str()),
        ],
    )
    .unwrap();
    let calls = fs::read_to_string(log).unwrap();
    assert_eq!(
        calls.lines().count(),
        7,
        "all ordinary default-feature Cargo commands must run"
    );
    for line in calls.lines() {
        let (command, exported) = line.split_once('\t').unwrap();
        // The default-feature lane uses ordinary `cargo test` on its own;
        // QSL-157 routes those same test invocations through the
        // `test-suites` xtask guard, so Cargo sees `run` at this boundary.
        assert!(["fmt", "clippy", "test", "run"].contains(&command));
        assert_eq!(exported, checkout_path.to_str().unwrap());
    }
}
