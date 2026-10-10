// SPDX-License-Identifier: AGPL-3.0-or-later
//! Native structural-validator controls for the local CI gate (NFR-002/NFR-005).

use ix_trace_rs::trace;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

const VALID: &str = "---\nid: NFR-999\ntitle: Fixture validation\ntype: NFR\nquality_attribute: maintainability\n---\n# NFR-999: Fixture validation\n\n## Statement\n\nThe gate shall validate each document.\n\n## Measurement and Evaluation\n\n| Metric | Target | Threshold | Method |\n| --- | --- | --- | --- |\n| Invalid documents | 0 | 0 | Test |\n\n## Verification\n\nRun the native validator.\n";

fn workspace() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask is below the workspace")
        .to_path_buf()
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

#[trace("NFR-002", "NFR-005")]
#[test]
fn spec_validation_executes_native_schema_controls_and_fails_closed() {
    let target = std::env::var_os("CARGO_TARGET_DIR").expect("owned target required");
    let fixture = tempfile::Builder::new()
        .prefix("spec validation controls ")
        .tempdir_in(target)
        .expect("scratch inside the owned target");
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

#[trace("NFR-002", "NFR-005")]
#[test]
fn full_ci_composes_the_unrestricted_scoped_validator() {
    let output = run(make(&workspace()).args(["-n", "ci"]));
    assert!(output.status.success(), "expand the real aggregate gate");
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("validate --scope . \"$SPEC_VALIDATION_DOCUMENTS\""));
    let validator = stdout.find("validate --scope").unwrap();
    let compilation = stdout.find("cargo clippy").unwrap();
    assert!(
        validator < compilation,
        "structural validation precedes compilation"
    );
}
