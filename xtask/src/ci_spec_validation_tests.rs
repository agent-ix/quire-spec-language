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
    make_with_file(root, &workspace().join("Makefile"))
}

fn make_with_file(root: &Path, file: &Path) -> Command {
    let mut command = Command::new("/usr/bin/make");
    command
        .current_dir(root)
        .arg("--no-print-directory")
        .arg("-rR")
        .arg("-f")
        .arg(file)
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

#[trace("TC-920", "NFR-002-AC-3")]
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

#[trace("TC-920", "NFR-002-AC-1")]
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

#[trace("TC-920", "NFR-002-AC-4")]
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
#[path = "__CONFORMANCE_CHECKS__"]
mod checks;
fn main() {
    let executable = std::env::args_os().next().unwrap();
    let name = std::path::Path::new(&executable).file_name().unwrap();
    let cargo = name == "cargo";
    let variable = if cargo { "SPEC_GATE_CARGO_CALLS" } else { "SPEC_GATE_PRECHECK_CALLS" };
    let path = std::env::var_os(variable).expect("owned call log");
    let mut log = std::fs::OpenOptions::new().create(true).append(true).open(path).unwrap();
    let call = if cargo {
        std::env::args().skip(1).collect::<Vec<_>>().join("\t")
    } else {
        name.to_str().unwrap().to_owned()
    };
    let call = format!("{call}\n");
    log.write_all(call.as_bytes()).unwrap();
    // Reuse the parent's synthetic completion vocabulary only to let the full
    // invocation control traverse conformance. No private vectors are read.
    if cargo {
        let args: Vec<String> = std::env::args().skip(1).collect();
        if let Some(selection) = args.windows(2).find(|pair| pair[0] == "--exact") {
            let (_, summary) = checks::CHECKS.iter()
                .find(|(name, _)| *name == selection[1])
                .expect("known conformance invocation");
            println!("{summary}");
            println!("test result: ok. 1 passed; 0 failed; 0 ignored; 0 filtered out; finished in 0.00s");
        }
    }
}
"#;

// Independent complete invocation inventory: default (7), all-feature (2),
// clean (3, including the two target-dir calls added below), four xtask probes,
// deny, docs and three architecture checks. The twelve adopted conformance
// calls are additional to this original 19+2 inventory, never replacements.
const AGGREGATE_CARGO_CALLS: &[&str] = &[
    "fmt\t--all\t--\t--check",
    "clippy\t--locked\t--workspace\t--all-targets\t--\t-D\twarnings",
    "test\t--locked\t--workspace",
    "clippy\t--locked\t-p\tqsl-semantics\t--all-targets\t--\t-D\twarnings",
    "test\t--locked\t-p\tqsl-semantics",
    "clippy\t--locked\t-p\tqsl-cst\t--all-targets\t--\t-D\twarnings",
    "test\t--locked\t-p\tqsl-cst",
    "clippy\t--locked\t--workspace\t--all-targets\t--all-features\t--\t-D\twarnings",
    "test\t--locked\t--workspace\t--all-features",
    "run\t--locked\t--no-default-features\t--\tparse\tagent-ix\ttest:parent\tfixture\tfixture:1\ttests/fixtures/parent.native",
    "run\t--package\txtask\t--\tseam-probe",
    "run\t--package\txtask\t--\tstring-edge",
    "run\t--package\txtask\t--\troute-lint",
    "run\t--package\txtask\t--\tchecked-input",
    "deny\t--workspace\tcheck\tbans\t--config\tdeny.toml",
    "doc\t--locked\t--workspace\t--no-deps\t--all-features",
    "run\t--locked\t-p\tarch-lint\t--\tcanonical-encoder\t--qsl\t.",
    "run\t--locked\t-p\tarch-lint\t--\tapi-surface\t--qsl\t.\t--qsl-only",
    "run\t--locked\t-p\tarch-lint\t--\tqualified-core\t--qsl\t.",
];

// Independent complete argv, not extracted from Make or generated from the
// synthetic summary vocabulary used by the invocation-only recorder.
const CONFORMANCE_CARGO_CALLS: &[&str] = &[
    "test\t--locked\t-p\tqsl-semantics\t--lib\t--\t--exact\tcheck::node_key::tests::conformance_fr322_application_keys_match_qspec_operation_vectors\t--format\tterse\t--nocapture",
    "test\t--locked\t-p\tqsl-semantics\t--lib\t--\t--exact\tcheck::node_key::tests::conformance_fr092_nominal_enum_keys_match_qspec_vectors\t--format\tterse\t--nocapture",
    "test\t--locked\t-p\tqsl-semantics\t--test\tit\t--\t--exact\tquantities::tc_411_compound_unit_ids_match_qspec_vectors\t--format\tterse\t--nocapture",
    "test\t--locked\t-p\tqsl-package\t--lib\t--\t--exact\tchecked_v2::tests::conformance_c14_source_map_lookup_over_qspec_positive_fixtures\t--format\tterse\t--nocapture",
    "test\t--locked\t-p\tqsl-package\t--lib\t--\t--exact\tchecked_v2::tests::conformance_i2_read_over_qspec_checked_package_v2_fixtures\t--format\tterse\t--nocapture",
    "test\t--locked\t-p\tqsl-package\t--lib\t--\t--exact\tchecked_v2::tests::conformance_fr340_frame_mutations_match_qspec_vectors\t--format\tterse\t--nocapture",
    "test\t--locked\t-p\tqsl-package\t--lib\t--\t--exact\tchecked_v2::tests::conformance_dependency_selection_vectors\t--format\tterse\t--nocapture",
    "test\t--locked\t-p\tqsl-package\t--lib\t--\t--exact\temit::tests::golden::conformance_emitted_application_nodes_match_qspec_positive_fixtures\t--format\tterse\t--nocapture",
    "test\t--locked\t-p\tqsl-semantics\t--lib\t--\t--exact\tcheck::profile::tests::conformance_fr110_profile_causes_are_listed_by_qspec_native_diagnostics\t--format\tterse\t--nocapture",
    "test\t--locked\t-p\tqsl-semantics\t--lib\t--\t--exact\tlibrary::bundle_tests::conformance_fr111_resolution_causes_are_listed_by_qspec_native_diagnostics\t--format\tterse\t--nocapture",
    "test\t--locked\t-p\tqsl-semantics\t--test\tit\t--\t--exact\tcomplete_value_lock::conformance_catalog_matches_qspec_complete_value_lock\t--format\tterse\t--nocapture",
    "test\t--locked\t-p\tqsl-semantics\t--test\tit\t--\t--exact\tcomplete_value_lock::conformance_admit_selection_matches_qspec_selection_vectors\t--format\tterse\t--nocapture",
];

#[trace("TC-920", "NFR-002-AC-2")]
#[test]
fn invalid_native_spec_prevents_actual_cargo_calls_even_in_parallel() {
    let fixture = owned_fixture();
    let root = fixture.path();
    std::fs::create_dir_all(root.join("spec/nested")).unwrap();
    let document = root.join("spec/nested/fixture with spaces.md");
    let bin = root.join("bin");
    std::fs::create_dir(&bin).unwrap();
    let source = root.join("cargo_recorder.rs");
    let checks = workspace().join("xtask/src/ci_conformance_tests/checks.rs");
    std::fs::write(
        &source,
        CARGO_RECORDER.replace("\"__CONFORMANCE_CHECKS__\"", &format!("{checks:?}")),
    )
    .unwrap();
    let compiler = run(
        Command::new("rustc")
            .args(["--edition=2021", "-D", "warnings"])
            .arg(&source)
            .arg("-o")
            .arg(bin.join("cargo")),
    );
    assert!(compiler.status.success(), "compile native process recorder");
    // cargo-deny is probed for executable presence only; the real recipe's
    // `cargo deny` invocation goes through the Cargo recorder, with no install.
    std::os::unix::fs::symlink(bin.join("cargo"), bin.join("cargo-deny")).unwrap();
    // The two non-Cargo prerequisites use native invocation seams. Their
    // underlying binary/index audits are not qualified by this boundary test.
    let tools = root.join("tools");
    std::fs::create_dir(&tools).unwrap();
    for name in ["check-no-committed-binaries.sh", "check-index-completeness.sh"] {
        std::os::unix::fs::symlink(bin.join("cargo"), tools.join(name)).unwrap();
    }
    let path = std::env::join_paths(
        std::iter::once(bin).chain(std::env::split_paths(&std::env::var_os("PATH").unwrap())),
    )
    .unwrap();
    let calls = root.join("cargo calls");
    let prechecks = root.join("precheck calls");
    let target = root.join("target");
    let checkout = root.join("invocation-only QSpec sentinel");
    let vectors = checkout.join("proposals/checked-package-v2");
    std::fs::create_dir_all(&vectors).unwrap();
    // Presence seam only, as in the parent fixture; never vector evidence.
    std::fs::write(vectors.join("node-identity-vectors.json"), "").unwrap();
    let aggregate = |file: &Path, scheduling: &[&str]| {
        let mut command = make_with_file(root, file);
        command
            .args(scheduling)
            .arg("ci")
            .env("PATH", &path)
            .env("CARGO_TARGET_DIR", &target)
            .env("QSPEC_DIR", &checkout)
            .env_remove("CI_DEFAULT_TARGET_DIR")
            .env_remove("CI_ALL_TARGET_DIR")
            .env_remove("CI_CLEAN_TARGET_DIR")
            .env("SPEC_GATE_CARGO_CALLS", &calls)
            .env("SPEC_GATE_PRECHECK_CALLS", &prechecks);
        command
    };
    let makefile = workspace().join("Makefile");
    for scheduling in [&["-j1"][..], &["-j4"][..], &["-j4", "-k"][..]] {
        std::fs::write(&document, VALID.replace("id: NFR-999\n", "")).unwrap();
        let negative = run(&mut aggregate(&makefile, scheduling));
        refused(&negative, "1 document(s) failed structural validation");
        assert!(
            !calls.exists(),
            "invalid native spec must prevent ALL Cargo calls"
        );
        assert!(
            !prechecks.exists(),
            "validation precedes non-Cargo checks too"
        );

        std::fs::write(&document, VALID).unwrap();
        assert!(
            run(&mut aggregate(&makefile, scheduling)).status.success(),
            "restored aggregate reaches Cargo"
        );
        let recorded = std::fs::read_to_string(&calls).unwrap();
        let mut actual: Vec<_> = recorded.lines().map(str::to_owned).collect();
        let mut expected: Vec<_> = AGGREGATE_CARGO_CALLS
            .iter()
            .chain(CONFORMANCE_CARGO_CALLS)
            .map(|call| (*call).to_owned())
            .collect();
        let clean = target.join("clean");
        expected.push(format!(
            "build\t--locked\t--workspace\t--no-default-features\t--target-dir\t{}",
            clean.display()
        ));
        expected.push(format!(
            "check\t--locked\t-p\tquire-spec-language\t--lib\t--no-default-features\t--features\thandoff-writer\t--target-dir\t{}",
            clean.display()
        ));
        actual.sort();
        expected.sort();
        assert_eq!(
            actual, expected,
            "every aggregate Cargo invocation must be observed"
        );
        let recorded = std::fs::read_to_string(&prechecks).unwrap();
        let mut actual: Vec<_> = recorded.lines().collect();
        actual.sort();
        assert_eq!(
            actual,
            ["check-index-completeness.sh", "check-no-committed-binaries.sh"]
        );
        std::fs::remove_file(&calls).unwrap();
        std::fs::remove_file(&prechecks).unwrap();
    }

    // Mutation sensitivity: alter only the deny target's validation edge in an
    // owned copy. Native Quire still refuses, but deny alone now invokes Cargo;
    // the unchanged zero-call oracle above must reject this graph.
    let source = std::fs::read_to_string(&makefile).unwrap();
    let edge = "$(CI_CHECKS): | check-spec-validation";
    assert_eq!(
        source.matches(edge).count(),
        1,
        "one aggregate barrier declaration"
    );
    let mutant = root.join("deny-edge-only.Makefile");
    std::fs::write(
        &mutant,
        source.replace(
            edge,
            "$(filter-out cargo-deny-bans,$(CI_CHECKS)): | check-spec-validation",
        ),
    )
    .unwrap();
    std::fs::write(&document, VALID.replace("id: NFR-999\n", "")).unwrap();
    let negative = run(&mut aggregate(&mutant, &["-j4", "-k"]));
    refused(&negative, "1 document(s) failed structural validation");
    assert_eq!(
        std::fs::read_to_string(&calls).unwrap(),
        "deny\t--workspace\tcheck\tbans\t--config\tdeny.toml\n",
        "removing only deny's barrier must violate the zero-Cargo oracle"
    );
    assert!(
        !prechecks.exists(),
        "other prerequisite barriers remain intact"
    );
}
