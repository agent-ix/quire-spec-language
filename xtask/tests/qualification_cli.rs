// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL qualification runner behavior (FR-350..FR-353).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use ix_trace_rs::trace;
use serde_json::Value;
use tempfile::TempDir;

fn qspec_dir() -> PathBuf {
    std::env::var_os("QSPEC_DIR")
        .map(PathBuf::from)
        .or_else(|| {
            let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .expect("xtask is under the workspace root");
            Some(workspace.parent()?.join("quire-specification"))
        })
        .expect("QSpec checkout path")
}

fn fixture_dir() -> PathBuf {
    qspec_dir().join("proposals/conformance-corpus-v1/fixtures")
}

fn run(arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_xtask"))
        .args(arguments)
        .env("QSPEC_DIR", qspec_dir())
        .output()
        .expect("xtask starts")
}

fn utf8(bytes: &[u8]) -> &str {
    std::str::from_utf8(bytes).expect("process output is UTF-8")
}

fn copy_fixture(temp: &TempDir) -> PathBuf {
    let root = temp.path();
    fs::create_dir_all(root.join("inputs")).unwrap();
    fs::copy(
        fixture_dir().join("complete-v1.json"),
        root.join("complete-v1.json"),
    )
    .unwrap();
    fs::copy(
        fixture_dir().join("inputs/native.native"),
        root.join("inputs/native.native"),
    )
    .unwrap();
    root.to_path_buf()
}

fn write_canonical(path: &Path, value: &Value) {
    let bytes = serde_json_canonicalizer::to_vec(value).expect("canonical corpus");
    fs::write(path, bytes).expect("write canonical corpus");
}

#[test]
#[trace("TC-880", "FR-353-AC-1")]
fn qualify_runs_qspec_formatter_fixture_and_reports_a_qualified_scope() {
    let fixture = fixture_dir();
    let output = run(&["qualify", "--corpus", fixture.to_str().unwrap()]);

    assert_eq!(output.status.code(), Some(0), "{}", utf8(&output.stderr));
    let report: Value = serde_json::from_slice(&output.stdout).expect("JSON report");
    assert_eq!(report["verdict"], "qualified");
    assert_eq!(report["scope"], serde_json::json!(["V1-TOOL-004"]));
    assert_eq!(report["counts"]["passed"], 1);
    assert_eq!(report["coverage"]["passed"], 1);
    assert_eq!(report["coverage"]["in_scope"], 1);
    assert_eq!(
        report["capabilities"][0]["outcome"], "passed",
        "the fixture's formatter vector is executed through QSL"
    );
}

#[test]
#[trace("TC-875", "FR-350-AC-1")]
fn qualify_compiles_a_language_vector_through_the_public_entry_point() {
    let temp = tempfile::tempdir().unwrap();
    let corpus = copy_fixture(&temp);
    let source_path = corpus.join("inputs/native.native");
    let source_bytes = fs::read(&source_path).unwrap();
    let identity = qsl_foundation::SourceIdentity::new(
        "agent-ix",
        "inputs/native.native",
        "qualification",
        "1",
    );
    let compiled = qsl_replay::compile_package(
        identity,
        "inputs/native.native",
        &source_bytes,
        std::iter::empty::<&[u8]>(),
        &qsl_replay::DependencyInput::default(),
        qsl_replay::StageLimits::default(),
        qsl_replay::ReplayLimits::default(),
    )
    .expect("the compile fixture is admitted");
    let package_id = compiled.package_id();
    let mut value: Value =
        serde_json::from_slice(&fs::read(corpus.join("complete-v1.json")).unwrap()).unwrap();
    value["coverage"]["primary"][0]["applicability"]["consumer_types"] =
        serde_json::json!(["language"]);
    value["vectors"][0]["applicability"]["consumer_types"] = serde_json::json!(["language"]);
    value["vectors"][0]["subject"]["consumer_type"] = Value::String("language".into());
    value["vectors"][0]["subject"]["operation"] = Value::String("compile".into());
    value["vectors"][0]["expected"]["value"] =
        serde_json::json!({"domain":package_id.domain().as_str(),"hex":package_id.hex()});
    write_canonical(&corpus.join("complete-v1.json"), &value);

    let output = run(&["qualify", "--corpus", corpus.to_str().unwrap()]);

    assert_eq!(output.status.code(), Some(0), "{}", utf8(&output.stderr));
    let report: Value = serde_json::from_slice(&output.stdout).expect("JSON report");
    assert_eq!(report["counts"]["passed"], 1);
    assert_eq!(report["capabilities"][0]["outcome"], "passed");
}

#[test]
#[trace("TC-881", "FR-353-AC-4")]
fn qualify_refuses_a_schema_invalid_corpus_before_execution() {
    let temp = tempfile::tempdir().unwrap();
    let corpus = copy_fixture(&temp);
    let corpus_file = corpus.join("complete-v1.json");
    let mut value: Value = serde_json::from_slice(&fs::read(&corpus_file).unwrap()).unwrap();
    value["vectors"][0]
        .as_object_mut()
        .unwrap()
        .remove("expected");
    fs::write(&corpus_file, serde_json::to_vec(&value).unwrap()).unwrap();

    let output = run(&["qualify", "--corpus", corpus.to_str().unwrap()]);

    assert_eq!(output.status.code(), Some(2));
    assert!(
        output.stdout.is_empty(),
        "a refusal has no verdict or report"
    );
    assert!(utf8(&output.stderr).contains("expected"));
}

#[test]
#[trace("TC-876", "FR-350-AC-4", "FR-353-AC-2")]
fn missing_vector_input_is_reported_as_tool_failure_and_exit_one() {
    let temp = tempfile::tempdir().unwrap();
    let corpus = copy_fixture(&temp);
    let corpus_file = corpus.join("complete-v1.json");
    let mut value: Value = serde_json::from_slice(&fs::read(&corpus_file).unwrap()).unwrap();
    value["vectors"][0]["input"]["files"][0]["path"] =
        Value::String("inputs/missing.native".into());
    write_canonical(&corpus_file, &value);

    let output = run(&["qualify", "--corpus", corpus.to_str().unwrap()]);

    assert_eq!(output.status.code(), Some(1));
    let report: Value = serde_json::from_slice(&output.stdout).expect("completed report");
    assert_eq!(report["verdict"], "not-qualified");
    assert_eq!(report["counts"]["tool-failure"], 1);
    assert_eq!(report["capabilities"][0]["outcome"], "tool-failure");
    assert_eq!(
        report["capabilities"][0]["vectors"][0]["outcome"],
        "tool-failure"
    );
}

#[test]
#[trace("TC-881", "FR-353-AC-4")]
fn duplicate_object_keys_refuse_before_a_report() {
    let temp = tempfile::tempdir().unwrap();
    let corpus = copy_fixture(&temp);
    let corpus_file = corpus.join("complete-v1.json");
    let bytes = fs::read(&corpus_file).unwrap();
    let source = String::from_utf8(bytes).unwrap();
    let duplicate = source.replacen(
        "\"contract_version\":",
        "\"contract_version\":\"quire.conformance-corpus/v1\",\"contract_version\":",
        1,
    );
    fs::write(&corpus_file, duplicate).unwrap();

    let output = run(&["qualify", "--corpus", corpus.to_str().unwrap()]);

    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    assert!(utf8(&output.stderr).contains("duplicate object key contract_version"));
}
