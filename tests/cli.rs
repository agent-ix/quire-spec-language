// SPDX-License-Identifier: AGPL-3.0-only
use std::process::Command;

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
    assert_eq!(invalid.status.code(), Some(1));
    let value: serde_json::Value = serde_json::from_slice(&invalid.stderr).unwrap();
    assert_eq!(value["code"], "invalid_source_identity");
    assert_eq!(value["status"], "refused");
}
