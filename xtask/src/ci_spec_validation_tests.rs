// SPDX-License-Identifier: AGPL-3.0-or-later
//! Native structural-validator controls for the local CI gate (NFR-002/NFR-005).

use ix_trace_rs::trace;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const VALID: &str = "---\nid: NFR-999\ntitle: Fixture validation\ntype: NFR\nquality_attribute: maintainability\n---\n# NFR-999: Fixture validation\n\n## Statement\n\nThe gate shall validate each document.\n\n## Measurement and Evaluation\n\n| Metric | Target | Threshold | Method |\n| --- | --- | --- | --- |\n| Invalid documents | 0 | 0 | Test |\n\n## Verification\n\nRun the native validator.\n";

fn workspace() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask is below the workspace")
        .to_path_buf()
}

fn target_directory(root: &Path, target: Option<&OsStr>) -> PathBuf {
    root.join(
        target
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("target")),
    )
}

fn fixture_in(root: &Path, target: Option<&OsStr>) -> tempfile::TempDir {
    let target = target_directory(root, target);
    std::fs::create_dir_all(&target).expect("create workspace-resolved target");
    tempfile::Builder::new()
        .prefix("spec validation controls ")
        .tempdir_in(target)
        .expect("scratch inside the workspace-resolved target")
}

fn owned_fixture() -> tempfile::TempDir {
    fixture_in(&workspace(), std::env::var_os("CARGO_TARGET_DIR").as_deref())
}

fn make(root: &Path) -> Command {
    let mut command = Command::new("/usr/bin/make");
    command
        .current_dir(root)
        .arg("--no-print-directory")
        .arg("-rR")
        .arg("-f")
        .arg(workspace().join("Makefile"))
        .env_remove("MAKEFLAGS")
        .env_remove("MFLAGS")
        .env_remove("MAKEFILES")
        .env_remove("SPEC_VALIDATION_DOCUMENTS");
    command
}

fn run(command: &mut Command) -> Output {
    let output = command.output().expect("execute the real Make target");
    eprintln!(
        "command={command:?}\nexit={}\nstdout={}\nstderr={}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

fn refused(output: &Output, diagnostic: &str) {
    assert_eq!(output.status.code(), Some(2), "Make must propagate failure");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains(diagnostic),
        "expected {diagnostic}: {stderr}"
    );
}

#[trace("TC-914", "NFR-002-AC-3")]
#[test]
fn spec_validation_executes_native_schema_controls_and_fails_closed() {
    let fixture = owned_fixture();
    let root = fixture.path();
    std::fs::create_dir(root.join("spec")).unwrap();
    let document = root.join("spec/fixture with spaces.md");
    std::fs::write(&document, VALID).unwrap();
    let positive = run(make(root).arg("check-spec-validation"));
    assert!(positive.status.success(), "valid native schema fixture");

    for (invalid, diagnostic) in [
        (VALID.replace("id: NFR-999\n", ""), "id"),
        (VALID.replace("## Statement", "## Omitted"), "Statement"),
        (VALID.replace("| Method |", "| Wrong |"), "Method"),
    ] {
        std::fs::write(&document, invalid).unwrap();
        let negative = run(make(root).arg("check-spec-validation"));
        refused(&negative, diagnostic);
        assert!(
            String::from_utf8_lossy(&negative.stderr)
                .contains("1 document(s) failed structural validation"),
            "invalid fixtures must fail native document validation"
        );
        let native = run(Command::new("quire").current_dir(root).args([
            "validate",
            "--scope",
            ".",
            "spec/**/*.md",
        ]));
        assert!(!native.status.success(), "native validator must refuse");
        let native_exit = native.status.code().expect("native validator exit code");
        assert!(
            String::from_utf8_lossy(&negative.stderr).contains(&format!("Error {native_exit}")),
            "Make must retain the actual native validator exit"
        );
        assert!(
            String::from_utf8_lossy(&negative.stderr).contains("fixture with spaces.md"),
            "document failure must name the owned fixture"
        );
    }
    std::fs::write(&document, VALID).unwrap();
    assert!(run(make(root).arg("check-spec-validation"))
        .status
        .success());

    let empty = run(make(root)
        .arg("check-spec-validation")
        .env("SPEC_VALIDATION_DOCUMENTS", "spec/missing/**/*.md"));
    refused(&empty, "document glob matched no files");
    let selected = run(make(root)
        .arg("check-spec-validation")
        .env("SPEC_VALIDATION_DOCUMENTS", "spec/fixture with spaces.md"));
    assert!(
        selected.status.success(),
        "forward document paths as one argument"
    );

    let binary_missing = run(make(root).arg("check-spec-validation").env("PATH", ""));
    refused(&binary_missing, "native quire is required on PATH");
    assert!(String::from_utf8_lossy(&binary_missing.stderr).contains("Error 127"));

    let isolated_home = root.join("empty home");
    std::fs::create_dir(&isolated_home).unwrap();
    let schema_missing = run(make(root)
        .arg("check-spec-validation")
        .env("HOME", &isolated_home)
        .env_remove("IX_FILAMENT_MODULES_PATH")
        .env_remove("IX_SCHEMA_PATH"));
    refused(&schema_missing, "no modules found for scoped validation");
    assert!(String::from_utf8_lossy(&schema_missing.stderr).contains("quoin not found on PATH"));
    assert!(
        !isolated_home.join(".ix").exists(),
        "gate must not install modules"
    );
}

#[trace("TC-914", "NFR-002-AC-1")]
#[test]
fn full_ci_composes_the_unrestricted_scoped_validator() {
    let output = run(make(&workspace()).args(["-n", "ci"]));
    assert!(output.status.success(), "expand the real aggregate gate");
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("validate --scope . \"$SPEC_VALIDATION_DOCUMENTS\""));
    assert!(
        stdout.contains("cargo clippy"),
        "aggregate includes compilation"
    );
    let baseline = run(make(&workspace()).arg("check-spec-validation"));
    assert!(
        baseline.status.success(),
        "full repository scoped validation"
    );
}

#[trace("TC-914", "NFR-002-AC-4")]
#[test]
fn native_fixtures_use_workspace_default_relative_and_absolute_targets() {
    let owner = owned_fixture();
    let root = owner.path().join("caller workspace");
    std::fs::create_dir(&root).unwrap();
    let absolute = owner.path().join("absolute target");
    for (target, expected) in [
        (None, root.join("target")),
        (
            Some(OsStr::new("relative target")),
            root.join("relative target"),
        ),
        (Some(absolute.as_os_str()), absolute.clone()),
    ] {
        let fixture = fixture_in(&root, target);
        assert_eq!(fixture.path().parent(), Some(expected.as_path()));
        let path = fixture.path().to_path_buf();
        std::fs::create_dir_all(path.join("spec/nested")).unwrap();
        std::fs::write(path.join("spec/nested/fixture with spaces.md"), VALID).unwrap();
        assert!(run(make(&path).arg("check-spec-validation"))
            .status
            .success());
        drop(fixture);
        assert!(!path.exists(), "Rust owner must remove its fixture");
    }
    assert_eq!(
        target_directory(&workspace(), None),
        workspace().join("target")
    );
    assert_eq!(
        target_directory(&workspace(), Some(OsStr::new("136-target/relative"))),
        workspace().join("136-target/relative")
    );
    assert_eq!(
        target_directory(&workspace(), Some(absolute.as_os_str())),
        absolute
    );
}

// Observes Make-to-Cargo invocation only. Returning success here is not evidence
// of compilation, feature qualification, schema validation or tool correctness.
const CARGO_RECORDER: &str = r#"
use std::io::Write;
fn main() {
    let path = std::env::var_os("SPEC_GATE_CARGO_CALLS").expect("owned call log");
    let mut log = std::fs::OpenOptions::new().create(true).append(true).open(path).unwrap();
    let call = format!("{}\n", std::env::args().skip(1).collect::<Vec<_>>().join("\t"));
    log.write_all(call.as_bytes()).unwrap();
}
"#;

#[trace("TC-914", "NFR-002-AC-2")]
#[test]
fn invalid_native_spec_prevents_actual_cargo_calls_even_in_parallel() {
    let fixture = owned_fixture();
    let root = fixture.path();
    std::fs::create_dir_all(root.join("spec/nested")).unwrap();
    let document = root.join("spec/nested/fixture with spaces.md");
    let bin = root.join("bin");
    std::fs::create_dir(&bin).unwrap();
    let source = root.join("cargo_recorder.rs");
    std::fs::write(&source, CARGO_RECORDER).unwrap();
    let compiler = run(
        Command::new("rustc")
            .args(["--edition=2021", "-D", "warnings"])
            .arg(&source)
            .arg("-o")
            .arg(bin.join("cargo")),
    );
    assert!(compiler.status.success(), "compile native process recorder");
    let path = std::env::join_paths(
        std::iter::once(bin).chain(std::env::split_paths(&std::env::var_os("PATH").unwrap())),
    )
    .unwrap();
    let calls = root.join("cargo calls");
    for scheduling in [&["-j1"][..], &["-j4"][..], &["-j4", "-k"][..]] {
        let aggregate = || {
            let mut command = make(root);
            command
                .args(scheduling)
                .args([
                    "-o",
                    "check-no-committed-binaries",
                    "-o",
                    "check-index-completeness",
                    "-o",
                    "cargo-deny-bans",
                    "ci",
                ])
                .env("PATH", &path)
                .env("SPEC_GATE_CARGO_CALLS", &calls);
            command
        };
        std::fs::write(&document, VALID.replace("id: NFR-999\n", "")).unwrap();
        let negative = run(&mut aggregate());
        refused(&negative, "1 document(s) failed structural validation");
        assert!(
            !calls.exists(),
            "invalid native spec must prevent ALL Cargo calls"
        );

        std::fs::write(&document, VALID).unwrap();
        assert!(
            run(&mut aggregate()).status.success(),
            "restored aggregate reaches Cargo"
        );
        let recorded = std::fs::read_to_string(&calls).unwrap();
        for operation in [
            "fmt\t", "clippy\t", "test\t", "build\t", "check\t", "doc\t", "run\t",
        ] {
            assert!(
                recorded.lines().any(|line| line.starts_with(operation)),
                "missing {operation}: {recorded}"
            );
        }
        std::fs::remove_file(&calls).unwrap();
    }
}
