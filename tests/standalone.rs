// SPDX-License-Identifier: AGPL-3.0-only
//! FR-026/028: real file-driven native command outcomes and adverse intake.

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
    (
        result.status.code().unwrap(),
        serde_json::from_slice(bytes).unwrap(),
        from_stdout,
    )
}

fn save(directory: &Path, value: &Value) {
    std::fs::write(
        directory.join("request.json"),
        serde_json::to_vec_pretty(value).unwrap(),
    )
    .unwrap();
}

fn exported_job(directory: &Path, case: setup::Case) -> Value {
    let (mut job, _) = setup::write(directory, case);
    let export = Command::new(env!("CARGO_BIN_EXE_quire-spec"))
        .arg("compile")
        .arg(directory.join("compile.json"))
        .output()
        .unwrap();
    assert_eq!(
        export.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&export.stderr)
    );
    std::fs::write(directory.join("package.json"), &export.stdout).unwrap();
    job["request"]["package"] =
        json!({"file":"package.json","digest":ByteDigest::of(&export.stdout).to_string()});
    save(directory, &job);
    job
}

#[test]
#[trace("TC-106", "FR-028-AC-1")]
fn selected_exports_reach_real_state_and_operation_outcomes() {
    for (case, code, truth) in [
        (setup::Case::Aggregate(2), 0, Some(true)),
        (setup::Case::Aggregate(1), 1, Some(false)),
        (setup::Case::Operation(false), 0, Some(true)),
        (setup::Case::Operation(true), 1, None),
    ] {
        let directory = tempfile::tempdir().unwrap();
        let job = exported_job(directory.path(), case);
        let (actual, result, stdout) = invoke(directory.path());
        assert_eq!(actual, code, "{result}");
        assert!(stdout);
        assert_eq!(result.get("truth").and_then(Value::as_bool), truth);
        assert_eq!(
            result["package"]["digest"],
            job["request"]["package"]["digest"]
        );
        assert_eq!(result["selection"], job["request"]["selection"]);
        if truth.is_none() {
            assert_eq!(result["stage"], "validate");
            assert!(result["diagnostics"]
                .as_array()
                .unwrap()
                .iter()
                .any(|error| error["code"] == "frame_violation"));
        }
    }
}

#[test]
#[trace("TC-106", "FR-028-AC-2", "FR-028-AC-4")]
fn selected_external_layout_retains_its_digest_and_original_runtime_observations() {
    let directory = tempfile::tempdir().unwrap();
    let mut job = exported_job(directory.path(), setup::Case::Aggregate(2));
    let (_, original, _) = invoke(directory.path());
    let mut alternate = b" \n\t".to_vec();
    alternate.extend(std::fs::read(directory.path().join("package.json")).unwrap());
    std::fs::write(directory.path().join("package.json"), &alternate).unwrap();
    job["request"]["package"]["digest"] = json!(ByteDigest::of(&alternate).to_string());
    save(directory.path(), &job);
    let (code, result, _) = invoke(directory.path());
    assert_eq!(code, 0, "{result}");
    assert_ne!(result["package"]["digest"], original["package"]["digest"]);
    assert_eq!(
        result["package"]["digest"],
        job["request"]["package"]["digest"]
    );
    assert_eq!(
        result["package"]["canonical_identity"],
        original["package"]["canonical_identity"]
    );
    for field in [
        "source",
        "models",
        "inputs",
        "selection",
        "truth",
        "evaluation_usage",
        "events",
    ] {
        assert_eq!(result[field], original[field], "{field}");
    }
    job["request"].as_object_mut().unwrap().remove("package");
    save(directory.path(), &job);
    let (code, compiled, _) = invoke(directory.path());
    assert_eq!(code, 0);
    assert_eq!(compiled["package"], original["package"]);
}

#[test]
#[trace("TC-106", "FR-028-AC-3")]
fn selected_package_failures_preserve_reader_details_without_compilation_fallback() {
    for (mutation, stage, code) in [
        ("stale", "decode", "stale_dependency"),
        ("forged", "compare", "invalid_package"),
        ("owner", "rebind", "invalid_model_binding"),
        ("formal", "rebind", "invalid_model_binding"),
        ("syntax", "rebind", "invalid_syntax"),
    ] {
        let directory = tempfile::tempdir().unwrap();
        let mut job = exported_job(directory.path(), setup::Case::Aggregate(2));
        let mut package: Value =
            serde_json::from_slice(&std::fs::read(directory.path().join("package.json")).unwrap())
                .unwrap();
        match mutation {
            "stale" => {
                job["request"]["package"]["digest"] = json!(ByteDigest::of(b"foreign").to_string())
            }
            "forged" => package["clauses"][0]["runtime"]["universes"] = json!([]),
            "owner" => job["request"]["program"]["clauses"][0]["owner"]["revision"] = json!(8),
            "formal" => {
                job["request"]["program"]["source"]["document"] = json!("ForeignFormalSource")
            }
            "syntax" => {
                let bytes = b"language ?";
                std::fs::write(directory.path().join("program.native"), bytes).unwrap();
                let digest = json!(ByteDigest::of(bytes).to_string());
                job["request"]["program"]["source"]["digest"] = digest.clone();
                package["source"]["digest"] = digest;
            }
            _ => unreachable!(),
        }
        if matches!(mutation, "forged" | "syntax") {
            let bytes = serde_json::to_vec(&package).unwrap();
            std::fs::write(directory.path().join("package.json"), &bytes).unwrap();
            job["request"]["package"]["digest"] = json!(ByteDigest::of(&bytes).to_string());
        }
        save(directory.path(), &job);
        let (exit, result, stdout) = invoke(directory.path());
        assert_eq!(exit, 1, "{result}");
        assert!(!stdout);
        assert_eq!(result["stage"], "package");
        assert_eq!(result["code"], code, "{result}");
        assert_eq!(result["details"]["stage"], stage);
        assert_eq!(result["details"]["file"], "package.json");
        assert_eq!(
            result["details"]["expected"]["digest"],
            job["request"]["package"]["digest"]
        );
        assert!(result.get("truth").is_none());
        if mutation == "forged" {
            assert!(!result["details"]["path"].as_array().unwrap().is_empty());
        }
        if mutation == "syntax" {
            assert_eq!(result["details"]["cause"]["code"], "invalid_syntax");
        }
    }
    let directory = tempfile::tempdir().unwrap();
    let job = exported_job(directory.path(), setup::Case::Aggregate(2));
    for (selection, expected_code) in [
        (Value::Null, "invalid-request"),
        (json!([]), "invalid-request"),
        (json!({"file":"package.json"}), "invalid-request"),
        (
            json!({"file":"package.json","digest":"bad","extra":0}),
            "invalid-request",
        ),
        (
            json!({"file":"package.json","digest":"bad"}),
            "invalid-digest",
        ),
    ] {
        let mut malformed = job.clone();
        malformed["request"]["package"] = selection;
        save(directory.path(), &malformed);
        let (exit, result, _) = invoke(directory.path());
        assert_eq!(exit, 2);
        assert_eq!(result["code"], expected_code);
    }
}

#[test]
#[trace("TC-106", "FR-028-AC-4")]
fn selected_package_intake_and_runtime_limits_stop_with_fresh_retry() {
    let directory = tempfile::tempdir().unwrap();
    let job = exported_job(directory.path(), setup::Case::Aggregate(2));
    for limits in [json!({"validation_work":0}), json!({"expression_steps":0})] {
        let mut stopped = job.clone();
        stopped["request"]["limits"] = limits;
        save(directory.path(), &stopped);
        let (code, result, stdout) = invoke(directory.path());
        assert_eq!(code, 3, "{result}");
        assert!(stdout);
        assert_eq!(result["status"], "incomplete");
        assert!(result.get("truth").is_none());
        assert_eq!(
            result["package"]["digest"],
            job["request"]["package"]["digest"]
        );
    }
    let mut count = job.clone();
    count["request"]["snapshots"] = json!(vec![job["request"]["snapshots"][0].clone(); 62]);
    save(directory.path(), &count);
    let (code, result, _) = invoke(directory.path());
    assert_eq!(code, 3);
    assert_eq!(result["details"]["limit"], "selected files");
    save(directory.path(), &job);
    let path = directory.path().join("package.json");
    let bytes = std::fs::read(&path).unwrap();
    std::fs::write(&path, vec![b' '; 8_388_609]).unwrap();
    let (code, result, _) = invoke(directory.path());
    assert_eq!(code, 3);
    assert_eq!(result["stage"], "intake");
    std::fs::write(path, bytes).unwrap();
    let (code, result, _) = invoke(directory.path());
    assert_eq!(code, 0, "{result}");
    assert_eq!(result["truth"], true);
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
