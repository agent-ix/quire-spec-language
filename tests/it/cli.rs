// SPDX-License-Identifier: AGPL-3.0-or-later
//! End-to-end CLI behavior: usage, I/O and resource exit-code outcomes.
use ix_trace_rs::trace;
use std::process::Command;

#[trace("TC-015", "FR-010-AC-3", "FR-010-AC-4", "FR-010-AC-7")]
#[test]
fn cli_usage_io_and_resource_outcomes_are_distinct() {
    let temp = tempfile::tempdir().unwrap();
    let missing = temp.path().join("missing.native");
    let executable = env!("CARGO_BIN_EXE_quire-spec");
    for args in [
        vec![],
        vec!["parse"],
        vec!["run", "id", "revision", "unused"],
        vec![
            "parse", "agent-ix", "id", "git", "revision", "unused", "extra",
        ],
    ] {
        let output = Command::new(executable).args(args).output().unwrap();
        assert_eq!(output.status.code(), Some(20));
        assert!(output.stdout.is_empty());
        assert!(!output.stderr.is_empty());
    }
    // A directory is not a readable source file, including when tests run as root.
    for path in [missing.as_path(), temp.path()] {
        let output = Command::new(executable)
            .args(["parse", "agent-ix", "id", "git", "revision"])
            .arg(path)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(20));
        assert!(output.stdout.is_empty());
        assert!(!output.stderr.is_empty());
    }
    let oversized = temp.path().join("oversized.native");
    std::fs::write(
        &oversized,
        vec![b' '; qsl_cst::Limits::default().source_bytes + 1],
    )
    .unwrap();
    let output = Command::new(executable)
        .args(["parse", "agent-ix", "id", "git", "revision"])
        .arg(&oversized)
        .output()
        .unwrap();
    // The source's own byte ceiling is a stage limit
    // (`stage_limit_exceeded`), incomplete work: exit 22 (ADR-029 CB-4).
    assert_eq!(output.status.code(), Some(22));
    assert!(output.stdout.is_empty());
    let value: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
    // FR-002/FR-010: the refusal is the `parse` operation's outcome document.
    assert_eq!(value["format"], "quire-outcome/1");
    assert_eq!(value["operation"], "parse");
    assert_eq!(value["category"], "incomplete");
    assert_eq!(value["last_stage"], "S1");
    assert_eq!(value["diagnostics"][0]["code"], "stage_limit_exceeded");
    assert!(value.get("value").is_none());
}

#[cfg(unix)]
#[trace("TC-017", "FR-010-AC-3", "FR-010-AC-6", "FR-010-AC-7")]
#[test]
fn cli_preserves_os_paths_and_refuses_non_utf8_labels_before_io() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;

    let temp = tempfile::tempdir().unwrap();
    let missing = temp.path().join("missing.native");
    let executable = env!("CARGO_BIN_EXE_quire-spec");
    for invalid_at in 0..5 {
        let mut args = vec![
            OsString::from("parse"),
            "agent-ix".into(),
            "id".into(),
            "git".into(),
            "revision".into(),
            missing.clone().into_os_string(),
        ];
        args[invalid_at] = OsString::from_vec(vec![0xff]);
        let output = Command::new(executable).args(args).output().unwrap();
        assert_eq!(output.status.code(), Some(20));
        assert!(output.stdout.is_empty());
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert!(stderr.contains("must be UTF-8"), "{stderr}");
        assert!(!stderr.contains("cannot open"));
    }
}

// APFS and HFS+ reject file names that are not valid UTF-8 (`EILSEQ`), so a
// non-UTF-8 source path cannot exist on macOS; the argv half above covers
// the label refusal there.
#[cfg(all(unix, not(target_os = "macos")))]
#[trace("TC-017", "FR-010-AC-3", "FR-010-AC-6", "FR-010-AC-7")]
#[test]
fn cli_preserves_non_utf8_os_paths() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;

    let temp = tempfile::tempdir().unwrap();
    let executable = env!("CARGO_BIN_EXE_quire-spec");
    let path = temp
        .path()
        .join(OsString::from_vec(b"source-\xff.native".to_vec()));
    let bytes = include_bytes!("../fixtures/spine-compile.native");
    std::fs::write(&path, bytes).unwrap();
    let output = Command::new(executable)
        .args(["parse", "agent-ix", "exact:é", "git", "revision:😀"])
        .arg(&path)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0));
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["operation"], "parse");
    assert_eq!(value["category"], "success");
    // FR-010-AC-6: the same bytes parsed in process carry the exact labels.
    let source = qsl_foundation::SourceIdentity::new("agent-ix", "exact:é", "git", "revision:😀");
    let parsed = qsl_replay::spine::parse(
        &qsl_replay::spine::ParseRequest {
            source: &source,
            path: &path.to_string_lossy(),
            bytes,
        },
        qsl_cst::Limits::default(),
        &quire_exact::Cancel::new(),
    )
    .unwrap();
    let reference = parsed.value().source().reference();
    assert_eq!(reference.authority, "agent-ix");
    assert_eq!(reference.identity, "exact:é");
}

#[trace("TC-012", "FR-002-AC-1")]
#[trace("TC-015", "FR-010-AC-1", "FR-010-AC-2", "FR-010-AC-5")]
#[trace("TC-103", "FR-026-AC-5")]
#[test]
fn cli_parses_and_formats_without_claiming_execution() {
    let parsed = Command::new(env!("CARGO_BIN_EXE_quire-spec"))
        .args([
            "parse",
            "agent-ix",
            "test:parent",
            "fixture",
            "fixture:1",
            "tests/fixtures/spine-compile.native",
        ])
        .output()
        .unwrap();
    assert!(parsed.status.success());
    let value: serde_json::Value = serde_json::from_slice(&parsed.stdout).unwrap();
    // FR-002-AC-7: the document with operation `parse`, last stage S2,
    // category `success` and nothing else.
    assert_eq!(
        value,
        serde_json::json!({
            "format": "quire-outcome/1", "operation": "parse", "last_stage": "S2",
            "category": "success", "items": [], "diagnostics": [], "artifacts": [],
            "result": null
        })
    );
    // FR-003: `format` reads complete-V1 source through the lossless CST.
    let formatted = Command::new(env!("CARGO_BIN_EXE_quire-spec"))
        .args([
            "format",
            "agent-ix",
            "test:value-format",
            "fixture",
            "fixture:1",
            "tests/fixtures/value-format.native",
        ])
        .output()
        .unwrap();
    assert!(
        formatted.status.success(),
        "{}",
        String::from_utf8_lossy(&formatted.stderr)
    );
    assert!(String::from_utf8(formatted.stdout)
        .unwrap()
        .contains("// the larger coordinate"));
    let refused = Command::new(env!("CARGO_BIN_EXE_quire-spec"))
        .args([
            "format",
            "agent-ix",
            "test:parent",
            "fixture",
            "fixture:1",
            "tests/fixtures/parent.native",
        ])
        .output()
        .unwrap();
    assert_eq!(refused.status.code(), Some(20));
    assert!(refused.stdout.is_empty());
    let value: serde_json::Value = serde_json::from_slice(&refused.stderr).unwrap();
    assert_eq!(value["status"], "refused");
    assert_eq!(value["code"], "unknown_edition");
    assert_eq!(value["path"], "tests/fixtures/parent.native");
    let invalid = Command::new(env!("CARGO_BIN_EXE_quire-spec"))
        .args([
            "parse",
            "agent-ix",
            "",
            "fixture",
            "fixture:1",
            "tests/fixtures/spine-compile.native",
        ])
        .output()
        .unwrap();
    assert_eq!(invalid.status.code(), Some(20));
    let value: serde_json::Value = serde_json::from_slice(&invalid.stderr).unwrap();
    assert_eq!(value["code"], "invalid_source_identity");
    assert_eq!(value["status"], "refused");
}

#[cfg(target_os = "linux")]
#[trace("TC-015", "FR-010-AC-10")]
#[test]
fn parse_output_failure_exits_30() {
    let full = std::fs::OpenOptions::new()
        .write(true)
        .open("/dev/full")
        .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_quire-spec"))
        .args([
            "parse",
            "agent-ix",
            "test:parent",
            "fixture",
            "fixture:1",
            "tests/fixtures/spine-compile.native",
        ])
        .stdout(std::process::Stdio::from(full))
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(30));
    assert!(std::str::from_utf8(&output.stderr)
        .unwrap()
        .starts_with("output failed:"));
}

#[trace("TC-015", "FR-010-AC-9")]
#[test]
fn parse_of_a_recognized_but_unsupported_construct_exits_21() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("unsupported.native");
    // S1 recognizes `null`; no S2 form represents it.
    std::fs::write(
        &path,
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n\
         function f using v(): Boolean pure { null }\n",
    )
    .unwrap();
    let refused = parse_file(&path, "test:unsupported");
    assert_eq!(refused.status.code(), Some(21));
    assert!(refused.stdout.is_empty());
    let value: serde_json::Value = serde_json::from_slice(&refused.stderr).unwrap();
    assert_eq!(value["category"], "unsupported");
    assert_eq!(value["last_stage"], "S2");
    assert_eq!(value["diagnostics"][0]["code"], "unsupported_construct");
}

// Sibling to the 21 case above: a source S1 recovers from is `invalid_syntax`
// and exits 20, not 21.
#[trace("TC-015", "FR-010-AC-2", "FR-010-AC-9")]
#[test]
fn parse_of_genuinely_malformed_syntax_exits_20_not_21() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("malformed.native");
    std::fs::write(
        &path,
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n\
         function f using v(: Boolean pure { true }\n",
    )
    .unwrap();
    let refused = parse_file(&path, "test:malformed");
    assert_eq!(refused.status.code(), Some(20));
    assert!(refused.stdout.is_empty());
    let value: serde_json::Value = serde_json::from_slice(&refused.stderr).unwrap();
    assert_eq!(value["category"], "refusal");
    assert_eq!(value["last_stage"], "S1");
    assert_eq!(value["diagnostics"][0]["code"], "invalid_syntax");
}

fn parse_file(path: &std::path::Path, identity: &str) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_quire-spec"))
        .args(["parse", "agent-ix", identity, "fixture", "fixture:1"])
        .arg(path)
        .output()
        .unwrap()
}

#[test]
#[trace("TC-104", "FR-026-AC-5")]
fn malformed_operands_report_the_selected_command_before_io() {
    for (arguments, command) in [
        (vec!["run"], "run"),
        (vec!["run", "missing.json", "extra"], "run"),
        (vec!["parse", "source"], "parse"),
        (
            vec![
                "format",
                "agent-ix",
                "source",
                "git",
                "revision",
                "missing.native",
                "extra",
            ],
            "format",
        ),
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_quire-spec"))
            .args(arguments)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(20));
        assert!(output.stdout.is_empty());
        let message = std::str::from_utf8(&output.stderr).unwrap();
        assert!(
            message.starts_with(&format!("usage: quire-spec {command} ")),
            "{message}"
        );
        assert!(!message.contains("cannot read"));
        assert!(!message.contains("cannot open"));
    }
}

/// TC-425 (FR-010-AC-11): `parse` and `format` take the four source labels
/// before the file; `parse` reports the source as its `RawSourceRef`:
/// authority and identity, and no other member.
#[trace("TC-425", "FR-010-AC-11")]
#[test]
fn parse_and_format_take_the_four_source_labels() {
    let executable = env!("CARGO_BIN_EXE_quire-spec");
    let labels = ["agent-ix", "specs/a.quire", "git", "3f2a"];
    let file = "tests/fixtures/spine-compile.native";
    let parsed = Command::new(executable)
        .arg("parse")
        .args(labels)
        .arg(file)
        .output()
        .unwrap();
    assert_eq!(parsed.status.code(), Some(0));
    let value: serde_json::Value = serde_json::from_slice(&parsed.stdout).unwrap();
    assert_eq!(value["category"], "success");
    // The success document carries no source member; the `ParsedSource` the
    // same operation returns in process names authority and identity only.
    let source = qsl_foundation::SourceIdentity::new(labels[0], labels[1], labels[2], labels[3]);
    let result = qsl_replay::spine::parse(
        &qsl_replay::spine::ParseRequest {
            source: &source,
            path: file,
            bytes: include_bytes!("../fixtures/spine-compile.native"),
        },
        qsl_cst::Limits::default(),
        &quire_exact::Cancel::new(),
    )
    .unwrap();
    assert_eq!(
        serde_json::to_value(result.value().source().reference()).unwrap(),
        serde_json::json!({
            "authority": "agent-ix",
            "identity": "specs/a.quire",
        })
    );

    for trailing in [vec![], vec![file, "extra"]] {
        let output = Command::new(executable)
            .arg("parse")
            .args(labels)
            .args(&trailing)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(20), "{trailing:?}");
        assert!(output.stdout.is_empty());
    }

    let blank = Command::new(executable)
        .args([
            "parse",
            "agent-ix",
            "specs/a.quire",
            "",
            "3f2a",
            "missing.native",
        ])
        .output()
        .unwrap();
    assert_eq!(blank.status.code(), Some(20));
    let value: serde_json::Value = serde_json::from_slice(&blank.stderr).unwrap();
    assert_eq!(value["code"], "invalid_source_identity");
    assert_eq!(value["cause"], "blank-label");
    assert_eq!(value["label"], "revision_namespace");
    assert!(value["span"].is_null());

    // An empty file operand is a file error, not an `empty-path` refusal:
    // the file is opened before any source is admitted, so `empty-path` is
    // reachable only through a library read (FR-001).
    let empty = Command::new(executable)
        .args(["parse", "agent-ix", "specs/a.quire", "git", "3f2a", ""])
        .output()
        .unwrap();
    assert_eq!(empty.status.code(), Some(20));
    assert!(!String::from_utf8_lossy(&empty.stderr).contains("invalid_source_identity"));

    let format = |extra: &[&str]| {
        Command::new(executable)
            .arg("format")
            .args(labels)
            .arg("tests/fixtures/value-format.native")
            .args(extra)
            .output()
            .unwrap()
    };
    let formatted = format(&[]);
    assert_eq!(
        formatted.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&formatted.stderr)
    );
    assert_eq!(format(&["extra"]).status.code(), Some(20));
}
