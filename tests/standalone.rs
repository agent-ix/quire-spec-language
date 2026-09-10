// SPDX-License-Identifier: AGPL-3.0-only
//! FR-026: real file-driven native command outcomes and adverse intake.

#[path = "support/standalone_setup.rs"]
mod setup;

use ix_trace_rs::trace;
use quire_spec_language::ByteDigest;
use serde_json::{json, Value};
use std::{path::Path, process::Command};

fn invoke(directory: &Path) -> (i32, Value, bool) {
    let result = Command::new(env!("CARGO_BIN_EXE_quire-spec"))
        .arg("run")
        .arg(directory.join("request.json"))
        .current_dir(directory.parent().unwrap())
        .output()
        .unwrap();
    let from_stdout = !result.stdout.is_empty();
    let bytes = if from_stdout {
        &result.stdout
    } else {
        &result.stderr
    };
    let value: Value = serde_json::from_slice(bytes).unwrap();
    assert!(
        result_schema().is_valid(&value),
        "result violates its schema: {value}"
    );
    if let Some(code) = value.get("code") {
        assert!(
            quire_spec_language::Code::from_code(code.as_str().unwrap()).is_some(),
            "uncatalogued command code: {code}"
        );
    }
    (result.status.code().unwrap(), value, from_stdout)
}

fn result_schema() -> &'static jsonschema::JSONSchema {
    static SCHEMA: std::sync::OnceLock<jsonschema::JSONSchema> = std::sync::OnceLock::new();
    SCHEMA.get_or_init(|| {
        let schema: Value =
            serde_json::from_str(include_str!("../schemas/native-run-result-1.schema.json"))
                .unwrap();
        jsonschema::JSONSchema::compile(&schema).unwrap()
    })
}

fn save(directory: &Path, value: &Value) {
    std::fs::write(
        directory.join("request.json"),
        serde_json::to_vec_pretty(value).unwrap(),
    )
    .unwrap();
}

#[test]
#[trace("TC-103", "FR-026-AC-1", "FR-026-AC-5")]
fn aggregate_files_produce_actual_truth_identities_work_and_events() {
    for (number, expected) in [(1, false), (2, true)] {
        let directory = tempfile::tempdir().unwrap();
        let (job, package_digest) = setup::write(directory.path(), setup::Case::Aggregate(number));
        let (code, result, stdout) = invoke(directory.path());
        assert_eq!(code, i32::from(!expected), "{result}");
        assert!(stdout);
        assert_eq!(result["status"], "completed");
        assert_eq!(result["truth"], expected);
        assert_eq!(result["package"]["digest"], package_digest);
        assert_eq!(
            result["source"]["digest"],
            job["request"]["program"]["source"]["digest"]
        );
        assert_eq!(
            result["inputs"]["snapshots"][0],
            job["request"]["snapshots"][0]["reference"]
        );
        assert_eq!(result["selection"], job["request"]["selection"]);
        assert_eq!(
            result["models"][0]["source"]["digest"],
            job["request"]["models"][0]["source"]["digest"]
        );
        assert_eq!(
            result["request_digest"],
            ByteDigest::of(&std::fs::read(directory.path().join("request.json")).unwrap())
                .to_string()
        );
        assert!(result["validation_usage"]["work"].as_u64().unwrap() > 0);
        assert!(
            result["evaluation_usage"]["expression_steps"]
                .as_u64()
                .unwrap()
                > 0
        );
        assert_eq!(result["events"].as_array().unwrap().len(), 3);
        assert_eq!(result["events"][1]["kind"], "antecedent_completed");
        assert_eq!(result["events"][1]["truth"], true);
        assert_eq!(result["cost_model"], "native-ref-cost/1-draft");
    }
}

#[test]
#[trace("TC-103", "FR-026-AC-2")]
fn recorded_operation_files_preserve_captures_and_frame_refusal() {
    for bad_frame in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let (job, _) = setup::write(directory.path(), setup::Case::Operation(bad_frame));
        let (code, result, stdout) = invoke(directory.path());
        assert!(stdout);
        assert_eq!(
            result["inputs"]["invocations"][0],
            job["request"]["invocations"][0]["reference"]
        );
        assert_eq!(result["inputs"]["snapshots"].as_array().unwrap().len(), 2);
        if bad_frame {
            assert_eq!(code, 1);
            assert_eq!(result["stage"], "validate");
            assert!(result.get("truth").is_none());
            let error = result["diagnostics"]
                .as_array()
                .unwrap()
                .iter()
                .find(|d| d["code"] == "frame_violation")
                .unwrap();
            assert_eq!(error["runtime"]["clause"], "population_rule");
            assert_eq!(error["runtime"]["artifact"]["kind"], "snapshot");
        } else {
            assert_eq!(code, 0, "{result}");
            assert_eq!(result["truth"], true);
        }
    }
}

#[test]
#[trace("TC-104", "FR-026-AC-3")]
fn stale_bytes_closed_requests_and_missing_files_keep_actual_failure_stage() {
    let directory = tempfile::tempdir().unwrap();
    let (job, _) = setup::write(directory.path(), setup::Case::Aggregate(2));
    for (pointer, replacement, expected_exit, stage, code) in [
        ("/format", json!("unknown"), 1, "envelope", "unknown_wire"),
        (
            "/request/program/source/digest",
            json!(ByteDigest::of(b"foreign").to_string()),
            1,
            "source",
            "source_digest_mismatch",
        ),
        (
            "/request/snapshots/0/reference/digest",
            json!("0".repeat(64)),
            1,
            "input",
            "stale_dependency",
        ),
        (
            "/request/program/source/file",
            json!("missing.native"),
            2,
            "file",
            "io-error",
        ),
        (
            "/request/selection/owner/revision",
            json!(0),
            2,
            "request",
            "invalid-identifier",
        ),
        (
            "/request/models/0/source",
            json!([]),
            2,
            "request",
            "invalid-request",
        ),
    ] {
        let mut changed = job.clone();
        *changed.pointer_mut(pointer).unwrap() = replacement;
        save(directory.path(), &changed);
        let (actual, result, stdout) = invoke(directory.path());
        assert_eq!(
            (actual, result["stage"].as_str(), result["code"].as_str()),
            (expected_exit, Some(stage), Some(code)),
            "{result}"
        );
        assert!(!stdout);
        assert!(result.get("truth").is_none());
        assert_eq!(
            result["request_digest"],
            ByteDigest::of(&std::fs::read(directory.path().join("request.json")).unwrap())
                .to_string()
        );
        if stage == "source" {
            assert_eq!(result["details"]["source"]["identity"], "test:runtime-rule");
        }
        if stage == "input" {
            assert_eq!(
                result["details"]["expected"]["reference"]["digest"],
                "0".repeat(64)
            );
        }
    }
    let text = serde_json::to_string(&job).unwrap();
    for malformed in [
        text.replacen("\"request\":", "\"extra\":0,\"request\":", 1),
        text.replacen(
            "\"format\":\"native-run/1\"",
            "\"format\":\"native-run/1\",\"format\":\"native-run/1\"",
            1,
        ),
    ] {
        std::fs::write(directory.path().join("request.json"), malformed).unwrap();
        let (code, result, _) = invoke(directory.path());
        assert_eq!(code, 2);
        assert_eq!(result["code"], "invalid-request");
    }
}

#[test]
#[trace("TC-104", "FR-026-AC-4")]
fn bounded_intake_and_runtime_stops_allow_fresh_default_execution() {
    let directory = tempfile::tempdir().unwrap();
    let (job, _) = setup::write(directory.path(), setup::Case::Aggregate(2));
    for limits in [
        json!({"validation_work":0}),
        json!({"expression_steps":0}),
        json!({"expression_steps":3}),
    ] {
        let mut stopped = job.clone();
        stopped["request"]["limits"] = limits;
        save(directory.path(), &stopped);
        let (code, result, stdout) = invoke(directory.path());
        assert!(stdout);
        assert_eq!(code, 3, "{result}");
        assert_eq!(result["status"], "incomplete");
        assert!(result.get("truth").is_none());
        if result["stage"] == "evaluate" {
            assert_eq!(result["diagnostic"]["code"], "resource_exhausted");
        }
    }
    let mut too_many = job.clone();
    too_many["request"]["models"] = json!(vec![job["request"]["models"][0].clone(); 64]);
    save(directory.path(), &too_many);
    assert_eq!(
        invoke(directory.path()).1["details"]["limit"],
        "selected files"
    );
    std::fs::write(directory.path().join("request.json"), vec![b' '; 1_048_577]).unwrap();
    let (code, result, _) = invoke(directory.path());
    assert_eq!(code, 3);
    assert_eq!(result["request_digest"], Value::Null);
    save(directory.path(), &job);
    let original = std::fs::read(directory.path().join("program.native")).unwrap();
    std::fs::write(
        directory.path().join("program.native"),
        vec![b' '; 1_048_577],
    )
    .unwrap();
    assert_eq!(invoke(directory.path()).0, 3);
    std::fs::write(directory.path().join("program.native"), original).unwrap();
    let snapshot_path = directory.path().join("snapshot-0.json");
    let snapshot = std::fs::read(&snapshot_path).unwrap();
    let mut padded = snapshot.clone();
    padded.resize(200_000, b' ');
    std::fs::write(&snapshot_path, &padded).unwrap();
    let mut aggregate = job.clone();
    let mut selected = aggregate["request"]["snapshots"][0].clone();
    selected["reference"]["digest"] = json!(ByteDigest::of(&padded)
        .to_string()
        .strip_prefix("sha256:")
        .unwrap());
    aggregate["request"]["snapshots"] = json!(vec![selected; 42]);
    save(directory.path(), &aggregate);
    let (code, result, _) = invoke(directory.path());
    assert_eq!(code, 3, "{result}");
    assert_eq!(result["stage"], "intake");
    assert_eq!(result["details"]["limit"], "file bytes");
    std::fs::write(&snapshot_path, snapshot).unwrap();
    save(directory.path(), &job);
    let (code, result, _) = invoke(directory.path());
    assert_eq!(code, 0);
    assert_eq!(result["truth"], true);
}

#[test]
#[trace("TC-104", "FR-026-AC-3", "FR-026-AC-4")]
fn typed_package_failure_retains_incomplete_classification() {
    use quire_spec_language::command::{RunCause, RunError};
    use quire_spec_language::package::{PackageError, PackageStage, PackageUsage};
    use quire_spec_language::Code;
    for (code, expected, status) in [
        (Code::Cancelled, 3, "incomplete"),
        (Code::InvalidPackage, 1, "refused"),
    ] {
        let error = RunError {
            request_digest: Some(ByteDigest::of(b"selected request")),
            cause: RunCause::Package(Box::new(PackageError {
                code,
                stage: PackageStage::Rebind,
                path: Vec::new(),
                usage: PackageUsage::default(),
                message: "identical prose for distinct causes",
                cause: None,
            })),
        };
        assert_eq!(error.exit_code(), expected);
        let value = error.value().unwrap();
        assert!(result_schema().is_valid(&value));
        assert_eq!(value["status"], status);
        assert_eq!(value["stage"], "package");
        assert_eq!(value["code"], code.as_str());
        assert!(value.get("truth").is_none());
    }
}

#[test]
#[trace("TC-104", "FR-026-AC-3")]
fn result_schema_rejects_missing_fields_and_truth_on_refusals() {
    for case in [setup::Case::Aggregate(2), setup::Case::Operation(true)] {
        let directory = tempfile::tempdir().unwrap();
        setup::write(directory.path(), case);
        let (_, result, _) = invoke(directory.path());
        for field in ["source", "package", "status", "stage", "validation_usage"] {
            let mut incomplete = result.clone();
            incomplete.as_object_mut().unwrap().remove(field);
            assert!(!result_schema().is_valid(&incomplete), "missing {field}");
        }
        let mut invalid = result.clone();
        if result["status"] == "completed" {
            invalid.as_object_mut().unwrap().remove("truth");
        } else {
            assert_eq!(result["status"], "refused");
            invalid["truth"] = json!(false);
        }
        assert!(!result_schema().is_valid(&invalid));
    }
}

#[test]
#[trace("TC-103", "FR-026-AC-5")]
fn absolute_and_parent_relative_operands_keep_selected_bytes() {
    let directory = tempfile::tempdir().unwrap();
    let (job, _) = setup::write(directory.path(), setup::Case::Aggregate(2));
    let child = directory.path().join("requests");
    std::fs::create_dir(&child).unwrap();
    for absolute in [true, false] {
        let mut changed = job.clone();
        for pointer in [
            "/request/models/0/source/file",
            "/request/program/source/file",
            "/request/snapshots/0/file",
        ] {
            let selected = changed.pointer(pointer).unwrap().as_str().unwrap();
            let path = if absolute {
                directory.path().join(selected)
            } else {
                Path::new("..").join(selected)
            };
            *changed.pointer_mut(pointer).unwrap() = json!(path.to_str().unwrap());
        }
        save(&child, &changed);
        let (code, result, stdout) = invoke(&child);
        assert_eq!(code, 0, "{result}");
        assert!(stdout);
        assert_eq!(result["truth"], true);
        assert_eq!(
            result["source"]["digest"],
            job["request"]["program"]["source"]["digest"]
        );
        assert_eq!(
            result["source"]["path"],
            changed["request"]["program"]["source"]["file"]
        );
    }
}
