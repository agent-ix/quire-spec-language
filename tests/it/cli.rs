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
        vec!["parse", "id", "revision", "unused", "extra"],
    ] {
        let output = Command::new(executable).args(args).output().unwrap();
        assert_eq!(output.status.code(), Some(20));
        assert!(output.stdout.is_empty());
        assert!(!output.stderr.is_empty());
    }
    // A directory is not a readable source file, including when tests run as root.
    for path in [missing.as_path(), temp.path()] {
        let output = Command::new(executable)
            .args(["parse", "id", "revision"])
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
        .args(["parse", "id", "revision"])
        .arg(&oversized)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(22));
    assert!(output.stdout.is_empty());
    let value: serde_json::Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(value["status"], "incomplete");
    assert_eq!(value["code"], "resource_exhausted");
    assert_eq!(value["phase"], "source");
    assert_eq!(value["source"]["identity"], "id");
    assert_eq!(value["source"]["revision"], "revision");
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
    for invalid_at in 0..3 {
        let mut args = vec![
            OsString::from("parse"),
            "id".into(),
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
        .args(["parse", "exact:é", "revision:😀"])
        .arg(&path)
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0));
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["status"], "parsed");
    assert_eq!(value["source"]["identity"], "exact:é");
    assert_eq!(value["source"]["revision"], "revision:😀");
    assert_eq!(
        value["source"]["digest"],
        quire_spec_language::ByteDigest::of(bytes).to_string()
    );
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
            "test:parent",
            "fixture:1",
            "tests/fixtures/parent.native",
        ])
        .output()
        .unwrap();
    assert!(parsed.status.success());
    let value: serde_json::Value = serde_json::from_slice(&parsed.stdout).unwrap();
    assert_eq!(value["status"], "parsed");
    assert_eq!(value["clauses"], 4);
    assert_eq!(value["source"]["revision"], "fixture:1");
    assert_eq!(value["source"]["identity"], "test:parent");
    assert_eq!(value["path"], "tests/fixtures/parent.native");
    assert_eq!(
        value["source"]["digest"],
        quire_spec_language::ByteDigest::of(include_bytes!("../fixtures/parent.native")).to_string()
    );
    assert!(value.get("value").is_none());
    let formatted = Command::new(env!("CARGO_BIN_EXE_quire-spec"))
        .args([
            "format",
            "test:parent",
            "fixture:1",
            "tests/fixtures/parent.native",
        ])
        .output()
        .unwrap();
    assert!(formatted.status.success());
    assert!(String::from_utf8(formatted.stdout)
        .unwrap()
        .contains("// Newly authored syntax fixture"));
    let invalid = Command::new(env!("CARGO_BIN_EXE_quire-spec"))
        .args(["parse", "", "fixture:1", "tests/fixtures/parent.native"])
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
            "test:parent",
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
        .args(["parse", "test:unsupported", "fixture:1"])
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
        .args(["parse", "test:malformed", "fixture:1"])
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
            vec!["format", "source", "revision", "missing.native", "extra"],
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
