// SPDX-License-Identifier: AGPL-3.0-or-later
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
    assert_ne!(
        output.stdout.is_empty(),
        output.stderr.is_empty(),
        "exactly one result stream"
    );
    let stdout = !output.stdout.is_empty();
    let bytes = if stdout {
        &output.stdout
    } else {
        &output.stderr
    };
    let value: Value = serde_json::from_slice(bytes).unwrap();
    assert!(
        result_schema().is_valid(&value),
        "result schema rejected {value}"
    );
    if let Some(code) = value.get("code") {
        assert!(quire_spec_language::Code::from_code(code.as_str().unwrap()).is_some());
    }
    (output.status.code().unwrap(), value, stdout)
}

#[test]
#[trace("TC-109", "FR-031-AC-3")]
fn extraction_request_obeys_the_build_feature() {
    let directory = tempfile::tempdir().unwrap();
    let job = setup::write_extracted(directory.path(), setup::Case::Aggregate(2), false).unwrap();
    let (code, result, stdout) = invoke(directory.path(), "run", &job);
    if cfg!(feature = "quire-extraction") {
        assert_eq!(code, 0, "{result}");
        assert!(stdout);
        assert_eq!(result["truth"], true);
    } else {
        assert_eq!(code, 20, "{result}");
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
                (setup::Case::Aggregate(1), 10, Some(false)),
                (
                    setup::Case::Operation {
                        violate_frame: false,
                    },
                    0,
                    Some(true),
                ),
                (
                    setup::Case::Operation {
                        violate_frame: true,
                    },
                    20,
                    None,
                ),
            ] {
                let directory = tempfile::tempdir().unwrap();
                let job = setup::write_extracted(directory.path(), case, crlf).unwrap();
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
            let mut job =
                setup::write_extracted(directory.path(), setup::Case::Aggregate(2), true).unwrap();
            let original = std::fs::read_to_string(directory.path().join("rules.md")).unwrap();
            let text = original.replace(
                "true implies forall(item in self.items: item < self.n)",
                replacement,
            );
            replace(directory.path(), &mut job, &text);
            let (actual, result, stdout) = invoke(directory.path(), "run", &job);
            assert_eq!(actual, 20, "{result}");
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
    #[trace("TC-109", "FR-031-AC-3")]
    fn closed_descriptors_and_shared_identity_fields_refuse_malformed_input() {
        let directory = tempfile::tempdir().unwrap();
        let job =
            setup::write_extracted(directory.path(), setup::Case::Aggregate(2), false).unwrap();
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
            assert_eq!(code, 20, "{result}");
            assert!(!stdout);
            assert_eq!(result["code"], "invalid-request");
        }
        for pointer in [
            "/request/program/source",
            "/request/program/extraction/body",
        ] {
            for (field, bad) in [
                ("identity", json!(1)),
                ("revision", json!(false)),
                ("document", json!([])),
                ("formal_revision", json!("seven")),
                ("extra", json!(0)),
            ] {
                let mut malformed = job.clone();
                malformed.pointer_mut(pointer).unwrap()[field] = bad;
                let (code, result, stdout) = invoke(directory.path(), "run", &malformed);
                assert_eq!(code, 20, "{pointer}/{field}: {result}");
                assert!(!stdout);
                assert_eq!(result["code"], "invalid-request");
            }
            let descriptor = job.pointer(pointer).unwrap().to_string();
            let duplicate = descriptor.replacen(
                "\"identity\":",
                "\"identity\":\"duplicate\",\"identity\":",
                1,
            );
            let bytes = job.to_string().replacen(&descriptor, &duplicate, 1);
            let (code, result, stdout) = invoke_bytes(directory.path(), "run", bytes.as_bytes());
            assert_eq!(code, 20, "{pointer}: {result}");
            assert!(!stdout);
            assert_eq!(result["code"], "invalid-request");
        }
    }

    fn missing_model_job(directory: &Path) -> Value {
        let mut job = setup::write_extracted(directory, setup::Case::Aggregate(2), false).unwrap();
        job["request"]["models"][0]["source"]["file"] = json!("missing-model.json");
        job
    }

    #[test]
    #[trace("TC-109", "FR-031-AC-3")]
    fn selected_package_conflict_is_distinct_and_precedes_file_io() {
        let directory = tempfile::tempdir().unwrap();
        let mut job = missing_model_job(directory.path());
        job["request"]["package"] = json!({"file":"missing-package.json","digest":"bad"});
        let (code, result, stdout) = invoke(directory.path(), "run", &job);
        assert_eq!(code, 20);
        assert!(!stdout);
        assert_eq!(result["stage"], "extraction-selection");
        assert_eq!(result["code"], "extraction-package-conflict");
        assert_eq!(result["details"], json!({"kind":"package_selected"}));
    }

    #[test]
    #[trace("TC-109", "FR-031-AC-3")]
    fn clause_count_refusals_retain_the_actual_count_before_file_io() {
        let directory = tempfile::tempdir().unwrap();
        let job = missing_model_job(directory.path());
        for count in [0, 2] {
            let mut changed = job.clone();
            changed["request"]["program"]["clauses"] =
                json!(vec![job["request"]["program"]["clauses"][0].clone(); count]);
            let (code, result, stdout) = invoke(directory.path(), "run", &changed);
            assert_eq!(code, 20);
            assert!(!stdout);
            assert_eq!(result["stage"], "extraction-selection");
            assert_eq!(result["code"], "extraction-clause-count");
            assert_eq!(
                result["details"],
                json!({"kind":"wrong_clause_count","actual":count})
            );
        }
    }

    #[test]
    #[trace("TC-109", "FR-031-AC-3")]
    fn source_only_exports_reject_extraction_before_file_io() {
        let directory = tempfile::tempdir().unwrap();
        let job = missing_model_job(directory.path());
        let compilation = json!({"format":"native-compile/1","request":{"models":job["request"]["models"],"program":job["request"]["program"]}});
        for command in ["compile", "lower"] {
            let (code, result, stdout) = invoke(directory.path(), command, &compilation);
            assert_eq!(code, 20);
            assert!(!stdout);
            assert_eq!(result["code"], "extraction-requires-run");
            assert_eq!(result["details"], json!({"kind":"compile_command"}));
        }
    }

    #[test]
    #[trace("TC-109", "FR-031-AC-4")]
    fn stale_source_and_extraction_line_ceiling_preserve_the_actual_stage() {
        let directory = tempfile::tempdir().unwrap();
        let job =
            setup::write_extracted(directory.path(), setup::Case::Aggregate(2), false).unwrap();
        let original = std::fs::read_to_string(directory.path().join("rules.md")).unwrap();
        std::fs::write(
            directory.path().join("rules.md"),
            format!("{original}stale\n"),
        )
        .unwrap();
        let (code, result, stdout) = invoke(directory.path(), "run", &job);
        assert_eq!(code, 20);
        assert!(!stdout);
        assert_eq!(result["code"], "source_digest_mismatch");
        let mut limited = job.clone();
        replace(
            directory.path(),
            &mut limited,
            &format!("{}{original}", "\n".repeat(4096)),
        );
        let (code, result, stdout) = invoke(directory.path(), "run", &limited);
        assert_eq!(code, 22, "{result}");
        assert!(!stdout);
        assert_eq!(result["stage"], "quire");
        assert_eq!(result["code"], "resource_exhausted");
        assert!(result["details"]["outcome"].is_null());
        assert_eq!(
            result["details"]["cause"]["preflight"]["kind"],
            "source_lines"
        );
        assert_eq!(result["details"]["cause"]["preflight"]["maximum"], 4096);
    }

    #[test]
    #[trace("TC-109", "FR-031-AC-4")]
    fn runtime_stops_retain_extraction_and_allow_a_fresh_success() {
        let directory = tempfile::tempdir().unwrap();
        let job =
            setup::write_extracted(directory.path(), setup::Case::Aggregate(2), false).unwrap();
        for limit in ["expression_steps", "validation_work"] {
            let mut limited = job.clone();
            limited["request"]["limits"] = json!({limit:0});
            let (code, result, stdout) = invoke(directory.path(), "run", &limited);
            assert_eq!(code, 22, "{result}");
            assert!(stdout);
            assert_eq!(result["status"], "incomplete");
            assert!(result["extraction"].is_object());
            assert!(result.get("truth").is_none());
        }
        let (code, result, stdout) = invoke(directory.path(), "run", &job);
        assert_eq!(code, 0, "{result}");
        assert!(stdout);
        assert_eq!(result["truth"], true);
    }

    #[test]
    #[trace("TC-109", "FR-031-AC-1", "FR-031-AC-4")]
    fn extraction_schema_rejects_incomplete_provenance_and_mapping() {
        let directory = tempfile::tempdir().unwrap();
        let job =
            setup::write_extracted(directory.path(), setup::Case::Aggregate(2), true).unwrap();
        let (code, result, _) = invoke(directory.path(), "run", &job);
        assert_eq!(code, 0);
        for (pointer, field) in [
            ("/extraction", "original"),
            ("/extraction", "outcome"),
            ("/extraction", "mapping"),
            ("/extraction/mapping", "segments"),
            ("/extraction/mapping/segments/0", "original"),
            ("/extraction/outcome", "clause_text"),
        ] {
            let mut invalid = result.clone();
            assert!(invalid
                .pointer_mut(pointer)
                .unwrap()
                .as_object_mut()
                .unwrap()
                .remove(field)
                .is_some());
            assert!(!result_schema().is_valid(&invalid), "{pointer}/{field}");
        }
    }
}

fn result_schema() -> &'static jsonschema::JSONSchema {
    static SCHEMA: std::sync::OnceLock<jsonschema::JSONSchema> = std::sync::OnceLock::new();
    SCHEMA.get_or_init(|| {
        jsonschema::JSONSchema::compile(
            &serde_json::from_str::<Value>(include_str!(
                "../schemas/native-run-result-1.schema.json"
            ))
            .unwrap(),
        )
        .unwrap()
    })
}
