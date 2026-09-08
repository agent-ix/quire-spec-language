// SPDX-License-Identifier: AGPL-3.0-only
//! FR-012: real audit process boundaries, immutable checkpoints and private packet lane.
use ix_trace_rs::trace;
use quire_spec_language::ByteDigest;
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    ffi::{OsStr, OsString},
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

fn audit(arguments: &[&OsStr]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_fixture-audit"))
        .env_clear()
        .env("PATH", "")
        .args(arguments)
        .output()
        .unwrap()
}

fn mode(mode: &str, root: &Path) -> Output {
    audit(&[OsStr::new(mode), root.as_os_str()])
}

fn passed(output: Output, expected: &str) -> String {
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains(expected), "{stdout}");
    stdout
}

fn refused(output: Output, exit: i32, code: &str) -> String {
    assert_eq!(output.status.code(), Some(exit), "{output:?}");
    assert!(output.stdout.is_empty(), "partial success: {output:?}");
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.starts_with(&format!("{code}:")), "{stderr}");
    stderr
}

fn fixture_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn copy_tree(source: &Path, destination: &Path) {
    fs::create_dir_all(destination).unwrap();
    for entry in fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let target = destination.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            assert!(entry.file_type().unwrap().is_file());
            fs::copy(entry.path(), target).unwrap();
        }
    }
}

fn contents(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn visit(base: &Path, root: &Path, result: &mut BTreeMap<PathBuf, Vec<u8>>) {
        for entry in fs::read_dir(root).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if entry.file_type().unwrap().is_dir() {
                visit(base, &path, result);
            } else {
                result.insert(
                    path.strip_prefix(base).unwrap().to_owned(),
                    fs::read(path).unwrap(),
                );
            }
        }
    }
    let mut result = BTreeMap::new();
    visit(root, root, &mut result);
    result
}

fn read_json(path: &Path) -> Value {
    serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
}
fn write_json(path: &Path, value: &Value) {
    fs::write(path, serde_json::to_vec(value).unwrap()).unwrap();
}

#[trace("TC-009", "FR-012-AC-10", "FR-012-AC-11", "NFR-005-M-2", "NFR-005-M-3")]
#[test]
fn cli_usage_and_unapproved_producer_with_no_external_runtime_path() {
    passed(
        audit(&[OsStr::new("self-test")]),
        "6 content/digest negative controls",
    );
    passed(
        mode("model-bytes", &fixture_root()),
        "5 producer checkpoint byte digests",
    );
    for arguments in [
        vec![],
        vec!["unknown"],
        vec!["model-bytes"],
        vec!["self-test", "extra"],
        vec!["roles", "root", "extra"],
    ] {
        let os: Vec<_> = arguments.iter().map(OsStr::new).collect();
        refused(audit(&os), 2, "usage");
    }
    refused(
        audit(&[OsStr::new("model-producer")]),
        3,
        "producer-language-unapproved",
    );
    let temp = tempfile::tempdir().unwrap();
    refused(mode("model-bytes", temp.path()), 2, "io");
}

#[cfg(unix)]
#[trace("TC-009", "FR-012-AC-10")]
#[test]
fn non_utf8_mode_refuses_and_non_utf8_root_executes() {
    use std::os::unix::ffi::OsStringExt;
    let invalid = OsString::from_vec(vec![0xff]);
    refused(audit(&[invalid.as_os_str()]), 2, "usage");
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join(invalid);
    copy_tree(&fixture_root(), &root);
    passed(
        mode("model-bytes", &root),
        "5 producer checkpoint byte digests",
    );
}

#[trace("TC-004", "FR-012-AC-4", "FR-012-AC-8")]
#[test]
fn model_checkpoint_digest_pin_and_immutable_inputs() {
    let original = fixture_root();
    let before = contents(&original);
    let summary = passed(
        mode("model-bytes", &original),
        "5 producer checkpoint byte digests and exact producer pin",
    );
    assert!(summary.contains("historical evidence only; no producer or evaluator executed"));
    let temp = tempfile::tempdir().unwrap();
    copy_tree(&original, temp.path());
    let provenance_path = temp.path().join("model-output/provenance.json");
    let provenance = read_json(&provenance_path);
    for row in provenance["artifacts"].as_array().unwrap() {
        let path = temp.path().join(row["path"].as_str().unwrap());
        let raw = fs::read(&path).unwrap();
        let mut changed = raw.clone();
        changed.push(b' ');
        fs::write(&path, changed).unwrap();
        refused(mode("model-bytes", temp.path()), 1, "digest-mismatch");
        fs::write(path, raw).unwrap();
    }
    let mut changed = provenance;
    changed["producer"]["revision"] = json!("unselected");
    write_json(&provenance_path, &changed);
    refused(mode("model-bytes", temp.path()), 1, "invalid-fixture");
    assert_eq!(contents(&original), before);
}

#[trace("TC-006", "FR-012-AC-6")]
#[test]
fn model_required_fields_reject_missing_null_and_wrong_types() {
    let temp = tempfile::tempdir().unwrap();
    copy_tree(&fixture_root(), temp.path());
    let path = temp.path().join("model-output/provenance.json");
    let original = read_json(&path);
    for pointer in [
        "/fixtureVersion",
        "/producer",
        "/producer/revision",
        "/artifacts",
        "/artifacts/0/path",
        "/artifacts/0/digest",
    ] {
        for replacement in [Value::Null, json!(0), json!(false), json!([]), json!({})] {
            let mut changed = original.clone();
            *changed.pointer_mut(pointer).unwrap() = replacement;
            write_json(&path, &changed);
            refused(mode("model-bytes", temp.path()), 1, "invalid-fixture");
        }
        let (parent, key) = pointer.rsplit_once('/').unwrap();
        let mut changed = original.clone();
        changed
            .pointer_mut(parent)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .remove(key);
        write_json(&path, &changed);
        refused(mode("model-bytes", temp.path()), 1, "invalid-fixture");
    }
    for raw in [br#"{"x":1,"\u0078":2}"#.as_slice(), b"{} true", b"\xff"] {
        fs::write(&path, raw).unwrap();
        refused(mode("model-bytes", temp.path()), 1, "invalid-json");
    }
}

#[trace("TC-008", "FR-012-AC-9")]
#[test]
fn process_budget_failure_emits_no_success_summary() {
    let temp = tempfile::tempdir().unwrap();
    fs::create_dir(temp.path().join("model-output")).unwrap();
    let path = temp.path().join("model-output/provenance.json");
    fs::File::create(&path)
        .unwrap()
        .set_len(8 * 1024 * 1024 + 1)
        .unwrap();
    refused(mode("model-bytes", temp.path()), 3, "resource-exhausted");
    fs::write(&path, format!("{}0{}", "[".repeat(64), "]".repeat(64))).unwrap();
    refused(mode("model-bytes", temp.path()), 3, "resource-exhausted");
}

fn selected_packet() -> PathBuf {
    PathBuf::from(
        std::env::var_os("QUIRE_STATE_CORE")
            .expect("IT-004 private-packet lane requires explicitly selected QUIRE_STATE_CORE"),
    )
}

#[trace("TC-002", "FR-012-AC-2")]
#[test]
#[ignore = "IT-004 private-packet lane: set QUIRE_STATE_CORE to the selected state-core directory"]
fn selected_review_packet_and_corrupt_invocation() {
    let original = selected_packet().join("fixtures");
    let before = contents(&original);
    let result = passed(
        mode("review", &original),
        "23 exact artifact files, 7 invocation cases; 6 content/digest negative controls",
    );
    assert!(result.contains("no evaluator executed"));
    let temp = tempfile::tempdir().unwrap();
    copy_tree(&original, temp.path());
    let manifest = read_json(&temp.path().join("invocation-cases.json"));
    let path = temp
        .path()
        .join(manifest["cases"][0]["input"]["path"].as_str().unwrap());
    let mut raw = fs::read(&path).unwrap();
    raw.push(b' ');
    fs::write(path, raw).unwrap();
    refused(mode("review", temp.path()), 1, "digest-mismatch");
    assert_eq!(contents(&original), before);
}

fn replace_reference(value: &mut Value, old: &Value, new: &Value) {
    if value == old {
        *value = new.clone();
        return;
    }
    match value {
        Value::Array(values) => {
            for value in values {
                replace_reference(value, old, new);
            }
        }
        Value::Object(values) => {
            for value in values.values_mut() {
                replace_reference(value, old, new);
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {}
    }
}

#[trace("TC-003", "FR-012-AC-3", "FR-012-AC-6")]
#[test]
#[ignore = "IT-004 private-packet lane: set QUIRE_STATE_CORE to the selected state-core directory"]
fn selected_roles_packet_and_independent_correspondence_mutations() {
    let original = selected_packet();
    let before = contents(&original);
    passed(
        mode("roles", &original.join("fixtures")),
        "17 exact artifacts and changed-byte controls; four source regions/coordinates",
    );
    let temp = tempfile::tempdir().unwrap();
    copy_tree(&original, temp.path());
    let root = temp.path().join("fixtures");
    let packet_path = root.join("role-compositions.json");
    let packet = read_json(&packet_path);
    let property_path = root.join(
        packet["artifactLocators"]["property-current"]
            .as_str()
            .unwrap(),
    );
    let property = read_json(&property_path);
    for (pointer, replacement, code, message) in [
        (
            "/source/clauseSpan/startColumn",
            json!(999),
            "invalid-fixture",
            "source coordinates mismatch",
        ),
        (
            "/source/clauseDigest",
            json!("sha256:changed"),
            "digest-mismatch",
            "source region bytes changed",
        ),
        (
            "/source/irRevisionMapping/revision",
            json!("1"),
            "invalid-fixture",
            "expected bounded unsigned integer",
        ),
    ] {
        let mut changed = property.clone();
        *changed.pointer_mut(pointer).unwrap() = replacement;
        write_json(&property_path, &changed);
        let old = &packet["artifactRefs"]["property-current"];
        let mut new = old.clone();
        new["digest"] = json!(ByteDigest::of(&fs::read(&property_path).unwrap()).to_string());
        let mut changed_packet = packet.clone();
        replace_reference(&mut changed_packet, old, &new);
        write_json(&packet_path, &changed_packet);
        assert!(refused(mode("roles", &root), 1, code).contains(message));
    }
    // Restore exact original bytes so role-membership validation is reached independently.
    let selected_path = packet["artifactLocators"]["property-current"]
        .as_str()
        .unwrap();
    fs::copy(
        original.join("fixtures").join(selected_path),
        &property_path,
    )
    .unwrap();
    let mut changed = packet;
    let roles = changed["cases"][0]["roles"].as_object_mut().unwrap();
    *roles.values_mut().next().unwrap() = json!({"identity":"unlisted"});
    write_json(&packet_path, &changed);
    assert!(refused(mode("roles", &root), 1, "invalid-fixture")
        .contains("role selects an unlisted reference"));
    assert_eq!(contents(&original), before);
}

#[trace("TC-005", "FR-012-AC-5", "FR-012-AC-8")]
#[test]
#[ignore = "IT-004 private-packet lane: set QUIRE_STATE_CORE to the selected state-core directory"]
fn selected_rule_syntax_and_malformed_case_metadata() {
    let original = selected_packet();
    let before = contents(&original);
    let summary = passed(
        mode("rule-syntax", &original),
        "50 parsed expressions; 1 unsupported refusal",
    );
    assert!(summary.contains("no typechecker or evaluator executed"));
    let temp = tempfile::tempdir().unwrap();
    copy_tree(&original, temp.path());
    let path = temp.path().join("fixtures/typing-cases.json");
    let fixture = read_json(&path);
    for (pointer, replacement) in [
        ("/cases/0/anchor", json!("unknown")),
        ("/cases/0/expression", json!(false)),
        ("/cases/0/id", Value::Null),
        ("/cases/0/expected/syntax", json!("evaluated")),
        ("/cases/1/id", fixture["cases"][0]["id"].clone()),
    ] {
        let mut changed = fixture.clone();
        *changed.pointer_mut(pointer).unwrap() = replacement;
        write_json(&path, &changed);
        refused(mode("rule-syntax", temp.path()), 1, "invalid-fixture");
    }
    assert_eq!(contents(&original), before);
}
