// SPDX-License-Identifier: AGPL-3.0-or-later
//! End-to-end CLI behavior: usage, I/O and resource exit-code outcomes.
use ix_trace_rs::trace;
use std::process::Command;

/// The lowercase-hex `quire.source.bytes/v1` digest of `bytes`.
fn hex(bytes: &[u8]) -> String {
    qsl_foundation::ByteDigest::of(bytes)
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

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
        vec![b' '; quire_spec_language::Limits::default().source_bytes + 1],
    )
    .unwrap();
    let output = Command::new(executable)
        .args(["parse", "agent-ix", "id", "git", "revision"])
        .arg(&oversized)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(22));
    assert!(output.stdout.is_empty());
    let value: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(value["status"], "incomplete");
    assert_eq!(value["code"], "resource_exhausted");
    assert_eq!(value["phase"], "source");
    assert_eq!(value["source"]["authority"], "agent-ix");
    assert_eq!(value["source"]["identity"], "id");
    assert_eq!(value["source"]["revision_namespace"], "git");
    assert_eq!(value["source"]["revision"], "revision");
    // The native-v1 diagnostic still renders a region-less S0 refusal at
    // byte 0: retained lane debt (FR-001 "Where an S0 refusal is located").
    assert_eq!(value["span"]["start"]["byte"], 0);
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
    let path = temp
        .path()
        .join(OsString::from_vec(b"source-\xff.native".to_vec()));
    let bytes = include_bytes!("../fixtures/parent.native");
    std::fs::write(&path, bytes).unwrap();
    let output = Command::new(executable)
        .args(["parse", "agent-ix", "exact:é", "git", "revision:😀"])
        .arg(&path)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0));
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["status"], "parsed");
    assert_eq!(value["source"]["identity"], "exact:é");
    assert_eq!(value["source"]["revision"]["value"], "revision:😀");
    assert_eq!(value["source"]["digest"], hex(bytes));
    assert_eq!(value["path"], path.to_string_lossy().as_ref());
    assert!(value["path"].as_str().unwrap().contains('\u{fffd}'));
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
            "tests/fixtures/parent.native",
        ])
        .output()
        .unwrap();
    assert!(parsed.status.success());
    let value: serde_json::Value = serde_json::from_slice(&parsed.stdout).unwrap();
    assert_eq!(value["status"], "parsed");
    assert_eq!(value["clauses"], 4);
    assert_eq!(value["source"]["revision"]["value"], "fixture:1");
    assert_eq!(value["source"]["identity"], "test:parent");
    assert_eq!(value["path"], "tests/fixtures/parent.native");
    assert_eq!(
        value["source"]["digest"],
        hex(include_bytes!("../fixtures/parent.native"))
    );
    assert!(value.get("value").is_none());
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
            "tests/fixtures/parent.native",
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
            "tests/fixtures/parent.native",
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
    std::fs::write(
        &path,
        "language \"ix:native\" edition \"0-draft\";\n\
         profile \"state-finite/0-draft\";\n\
         model M = \"test/model\" version \"1\" digest \"unresolved\";\n\
         invariant Test on M::Thing at current { 1 / 2 }\n",
    )
    .unwrap();
    let refused = Command::new(env!("CARGO_BIN_EXE_quire-spec"))
        .args([
            "parse",
            "agent-ix",
            "test:unsupported",
            "fixture",
            "fixture:1",
        ])
        .arg(&path)
        .output()
        .unwrap();
    assert_eq!(refused.status.code(), Some(21));
    assert!(refused.stdout.is_empty());
    let value: serde_json::Value = serde_json::from_slice(&refused.stderr).unwrap();
    assert_eq!(value["code"], "unsupported_construct");
    assert_eq!(value["status"], "refused");
}

// Sibling to the 21 case above: a recognized-but-unsupported token (`/`)
// takes the `unsupported` half of `Parser::unexpected`'s split, but an
// expression position holding a token the grammar never admits at all (a
// stray `}`) must still take the `invalid syntax` half and exit 20. If the
// unsupported-token set at src/parser.rs's `unexpected` were ever widened to
// include `CloseBrace`, this assertion would fail against 21 instead.
#[trace("TC-015", "FR-010-AC-2", "FR-010-AC-9")]
#[test]
fn parse_of_genuinely_malformed_syntax_exits_20_not_21() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("malformed.native");
    std::fs::write(
        &path,
        "language \"ix:native\" edition \"0-draft\";\n\
         profile \"state-finite/0-draft\";\n\
         model M = \"test/model\" version \"1\" digest \"unresolved\";\n\
         invariant Test on M::Thing at current { }\n",
    )
    .unwrap();
    let refused = Command::new(env!("CARGO_BIN_EXE_quire-spec"))
        .args([
            "parse",
            "agent-ix",
            "test:malformed",
            "fixture",
            "fixture:1",
        ])
        .arg(&path)
        .output()
        .unwrap();
    assert_eq!(refused.status.code(), Some(20));
    assert!(refused.stdout.is_empty());
    let value: serde_json::Value = serde_json::from_slice(&refused.stderr).unwrap();
    assert_eq!(value["code"], "invalid_syntax");
    assert_eq!(value["status"], "refused");
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
/// before the file; `parse` reports the source as its `RawSourceRef`.
#[trace("TC-425", "FR-010-AC-11")]
#[test]
fn parse_and_format_take_the_four_source_labels() {
    let executable = env!("CARGO_BIN_EXE_quire-spec");
    let labels = ["agent-ix", "specs/a.quire", "git", "3f2a"];
    let file = "tests/fixtures/parent.native";
    let parsed = Command::new(executable)
        .arg("parse")
        .args(labels)
        .arg(file)
        .output()
        .unwrap();
    assert_eq!(parsed.status.code(), Some(0));
    let value: serde_json::Value = serde_json::from_slice(&parsed.stdout).unwrap();
    assert_eq!(
        value["source"],
        serde_json::json!({
            "authority": "agent-ix",
            "identity": "specs/a.quire",
            "revision": {"namespace": "git", "value": "3f2a"},
            "digest_domain": "quire.source.bytes/v1",
            "digest": hex(include_bytes!("../fixtures/parent.native")),
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
    assert!(value["span"].is_null());

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
