// SPDX-License-Identifier: AGPL-3.0-only
//! FR-031: actual standalone Markdown execution and feature/mode refusals.

#[path = "support/standalone_setup.rs"]
mod setup;

use ix_trace_rs::trace;
use serde_json::Value;
use std::{path::Path, process::Command};

fn invoke(directory: &Path, command: &str, job: &Value) -> (i32, Value, bool) {
    invoke_bytes(directory, command, &serde_json::to_vec_pretty(job).unwrap())
}

fn invoke_bytes(directory: &Path, command: &str, bytes: &[u8]) -> (i32, Value, bool) {
    std::fs::write(directory.join("request.json"), bytes).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_quire-spec"))
        .arg(command)
        .arg(directory.join("request.json"))
        .current_dir(directory.parent().unwrap())
        .output()
        .unwrap();
    let stdout = !output.stdout.is_empty();
    let bytes = if stdout {
        &output.stdout
    } else {
        &output.stderr
    };
    (
        output.status.code().unwrap(),
        serde_json::from_slice(bytes).unwrap(),
        stdout,
    )
}

#[test]
#[trace("TC-109", "FR-031-AC-3")]
fn extraction_request_obeys_the_build_feature() {
    let directory = tempfile::tempdir().unwrap();
    let job = setup::write_extracted(directory.path(), setup::Case::Aggregate(2), false);
    let (code, result, stdout) = invoke(directory.path(), "run", &job);
    if cfg!(feature = "quire-extraction") {
        assert_eq!(code, 0, "{result}");
        assert!(stdout);
        assert_eq!(result["truth"], true);
    } else {
        assert_eq!(code, 2, "{result}");
        assert!(!stdout);
        assert_eq!(result["code"], "invalid-request");
        assert!(result.get("truth").is_none());
    }
}

#[cfg(feature = "quire-extraction")]
mod enabled {
    use super::*;
    use quire_spec_language::ByteDigest;
    use serde_json::json;

    fn replace(directory: &Path, job: &mut Value, text: &str) {
        std::fs::write(directory.join("rules.md"), text).unwrap();
        job["request"]["program"]["source"]["digest"] =
            json!(ByteDigest::of(text.as_bytes()).to_string());
    }

    #[test]
    #[trace("TC-109", "FR-031-AC-1", "FR-031-AC-4")]
    fn actual_markdown_cases_keep_native_results_and_exact_source_maps() {
        for crlf in [false, true] {
            for (case, code, truth) in [
                (setup::Case::Aggregate(2), 0, Some(true)),
                (setup::Case::Aggregate(1), 1, Some(false)),
                (setup::Case::Operation(false), 0, Some(true)),
                (setup::Case::Operation(true), 1, None),
            ] {
                let directory = tempfile::tempdir().unwrap();
                let job = setup::write_extracted(directory.path(), case, crlf);
                let original = std::fs::read_to_string(directory.path().join("rules.md")).unwrap();
                let (actual, result, stdout) = invoke(directory.path(), "run", &job);
                assert_eq!(actual, code, "{result}");
                assert!(stdout);
                assert_eq!(result.get("truth").and_then(Value::as_bool), truth);
                assert_eq!(result["selection"], job["request"]["selection"]);
                assert_eq!(
                    result["extraction"]["original"]["digest"],
                    job["request"]["program"]["source"]["digest"]
                );
                assert_eq!(
                    result["source"]["identity"],
                    job["request"]["program"]["extraction"]["body"]["identity"]
                );
                assert_eq!(
                    result["extraction"]["outcome"]["availability"]["state"],
                    "available"
                );
                assert_eq!(
                    result["extraction"]["outcome"]["availability"]["lossy"],
                    true
                );
                let outcome = &result["extraction"]["outcome"];
                assert!(outcome["diagnostics"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|item| item["code"] == "semantic.clause-language-unchecked"));
                let clause = job["request"]["program"]["clauses"][0]["clause"]
                    .as_str()
                    .unwrap();
                let raw = outcome["clause_text"][clause].as_str().unwrap();
                let body = if crlf {
                    raw.strip_suffix('\r').unwrap()
                } else {
                    raw
                };
                let segments = result["extraction"]["mapping"]["segments"]
                    .as_array()
                    .unwrap();
                assert_eq!(segments.len(), 1);
                let start =
                    usize::try_from(segments[0]["original"]["start"].as_u64().unwrap()).unwrap();
                let end =
                    usize::try_from(segments[0]["original"]["end"].as_u64().unwrap()).unwrap();
                assert_eq!(&original[start..end], body);
                assert_eq!(
                    result["source"]["digest"],
                    ByteDigest::of(body.as_bytes()).to_string()
                );
                if truth.is_none() {
                    assert_eq!(result["stage"], "validate");
                    assert!(result["diagnostics"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|item| item["code"] == "frame_violation"));
                }
            }
        }
    }

    #[test]
    #[trace("TC-109", "FR-031-AC-2")]
    fn extraction_and_native_failures_retain_original_selection() {
        for (replacement, code) in [
            ("@", "invalid_syntax"),
            ("collect(self.items)", "unsupported_construct"),
        ] {
            let directory = tempfile::tempdir().unwrap();
            let mut job = setup::write_extracted(directory.path(), setup::Case::Aggregate(2), true);
            let original = std::fs::read_to_string(directory.path().join("rules.md")).unwrap();
            let text = original.replace(
                "true implies forall(item in self.items: item < self.n)",
                replacement,
            );
            replace(directory.path(), &mut job, &text);
            let (actual, result, stdout) = invoke(directory.path(), "run", &job);
            assert_eq!(actual, 1, "{result}");
            assert!(!stdout);
            assert_eq!(result["code"], code);
            assert_eq!(
                result["details"]["original"]["digest"],
                job["request"]["program"]["source"]["digest"]
            );
            assert_eq!(
                result["details"]["selection"]["clause"],
                job["request"]["program"]["clauses"][0]["clause"]
            );
            assert_eq!(
                result["details"]["cause"]["original_spans"][0]["start"]["byte"],
                text.find(replacement).unwrap()
            );
            assert_eq!(
                result["details"]["outcome"]["availability"]["state"],
                "available"
            );
            assert!(result.get("truth").is_none());
            let missing = original.replace("## Invariants", "## Unselected");
            replace(directory.path(), &mut job, &missing);
            let (_, result, _) = invoke(directory.path(), "run", &job);
            assert_eq!(result["code"], "invalid_model_binding");
            assert_ne!(
                result["details"]["outcome"]["availability"]["state"],
                "available"
            );
        }
    }

    #[test]
    #[trace("TC-109", "FR-031-AC-3", "FR-031-AC-4")]
    fn descriptors_modes_stale_bytes_and_limits_refuse_with_fresh_retry() {
        let directory = tempfile::tempdir().unwrap();
        let job = setup::write_extracted(directory.path(), setup::Case::Aggregate(2), false);
        for bad in [
            Value::Null,
            json!([]),
            json!({"body":[]}),
            json!({"body":null}),
            json!({"body":{},"extra":0}),
        ] {
            let mut malformed = job.clone();
            malformed["request"]["program"]["extraction"] = bad;
            let (code, result, stdout) = invoke(directory.path(), "run", &malformed);
            assert_eq!(code, 2, "{result}");
            assert!(!stdout);
            assert_eq!(result["code"], "invalid-request");
        }
        let mut selected = job.clone();
        selected["request"]["models"][0]["source"]["file"] = json!("missing-model.json");
        selected["request"]["package"] = json!({"file":"missing-package.json","digest":"bad"});
        let (_, result, _) = invoke(directory.path(), "run", &selected);
        assert_eq!(result["code"], "unsupported-extraction-mode");
        selected["request"]
            .as_object_mut()
            .unwrap()
            .remove("package");
        selected["request"]["program"]["clauses"] = json!([]);
        let (_, result, _) = invoke(directory.path(), "run", &selected);
        assert_eq!(result["code"], "unsupported-extraction-mode");
        selected["request"]["program"]["clauses"] = json!([
            job["request"]["program"]["clauses"][0],
            job["request"]["program"]["clauses"][0]
        ]);
        let (_, result, _) = invoke(directory.path(), "run", &selected);
        assert_eq!(result["code"], "unsupported-extraction-mode");
        let compilation = json!({"format":"native-compile/1","request":{"models":selected["request"]["models"],"program":job["request"]["program"]}});
        for command in ["compile", "lower"] {
            let (code, result, stdout) = invoke(directory.path(), command, &compilation);
            assert_eq!(code, 1);
            assert!(!stdout);
            assert_eq!(result["code"], "unsupported-extraction-mode");
        }
        let descriptor = job["request"]["program"]["extraction"].to_string();
        let duplicate = descriptor.replacen(
            "\"identity\":",
            "\"identity\":\"duplicate\",\"identity\":",
            1,
        );
        let bytes = job.to_string().replacen(&descriptor, &duplicate, 1);
        let (code, result, _) = invoke_bytes(directory.path(), "run", bytes.as_bytes());
        assert_eq!(code, 2, "{result}");
        assert_eq!(result["code"], "invalid-request");
        let original = std::fs::read_to_string(directory.path().join("rules.md")).unwrap();
        std::fs::write(
            directory.path().join("rules.md"),
            format!("{original}stale\n"),
        )
        .unwrap();
        let (_, result, _) = invoke(directory.path(), "run", &job);
        assert_eq!(result["code"], "source_digest_mismatch");
        let mut limited = job.clone();
        replace(
            directory.path(),
            &mut limited,
            &format!("{}{original}", "\n".repeat(4096)),
        );
        let (code, result, _) = invoke(directory.path(), "run", &limited);
        assert_eq!(code, 3, "{result}");
        assert_eq!(result["code"], "resource_exhausted");
        assert!(result["details"]["outcome"].is_null());
        replace(directory.path(), &mut limited, &original);
        for limit in ["expression_steps", "validation_work"] {
            limited["request"]["limits"] = json!({limit:0});
            let (code, result, stdout) = invoke(directory.path(), "run", &limited);
            assert_eq!(code, 3, "{result}");
            assert!(stdout);
            assert_eq!(result["status"], "incomplete");
            assert!(result["extraction"].is_object());
        }
        let (code, result, _) = invoke(directory.path(), "run", &job);
        assert_eq!(code, 0, "{result}");
        assert_eq!(result["truth"], true);
    }
}
