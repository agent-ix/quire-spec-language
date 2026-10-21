// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-064's consumer edge rules: the strict JSON object and its checks.
use ix_trace_rs::trace;
use qsl_string_edge::{scan, AllowListEntry, EdgeRules, Error};

use crate::{request, Tree, CLEAN};

/// FR-064-AC-8: the documented rules object parses into the typed value the
/// library also accepts directly; an empty array is an explicit empty
/// allow-list.
#[trace("TC-162", "FR-064-AC-8")]
#[test]
fn the_documented_rules_object_parses() {
    let rules = EdgeRules::from_json(
        r#"{"allow_list": [{"file": "src/display.rs", "item": "display", "reason": "Display-only comparison of supplied text"}]}"#,
    )
    .expect("rules parse");
    assert_eq!(
        rules,
        EdgeRules {
            allow_list: vec![AllowListEntry {
                file: "src/display.rs".to_owned(),
                item: "display".to_owned(),
                reason: "Display-only comparison of supplied text".to_owned(),
            }]
        }
    );
    assert_eq!(
        EdgeRules::from_json(r#"{"allow_list": []}"#).expect("empty list"),
        EdgeRules::default()
    );
}

/// FR-064-AC-9: unknown or missing fields, wrong types and malformed JSON
/// are rule errors.
#[trace("TC-162", "FR-064-AC-9")]
#[test]
fn malformed_rules_are_typed_errors() {
    let entry = r#"{"file": "a.rs", "item": "f", "reason": "r"}"#;
    for (label, text) in [
        ("malformed JSON", "{".to_owned()),
        ("empty input", String::new()),
        ("missing allow_list", "{}".to_owned()),
        (
            "unknown top-level field",
            r#"{"allow_list": [], "extra": 1}"#.to_owned(),
        ),
        (
            "allow_list not an array",
            r#"{"allow_list": {}}"#.to_owned(),
        ),
        (
            "entry not an object",
            r#"{"allow_list": ["a.rs"]}"#.to_owned(),
        ),
        (
            "missing file",
            r#"{"allow_list": [{"item": "f", "reason": "r"}]}"#.to_owned(),
        ),
        (
            "missing item",
            r#"{"allow_list": [{"file": "a.rs", "reason": "r"}]}"#.to_owned(),
        ),
        (
            "missing reason",
            r#"{"allow_list": [{"file": "a.rs", "item": "f"}]}"#.to_owned(),
        ),
        (
            "unknown entry field",
            r#"{"allow_list": [{"file": "a.rs", "item": "f", "reason": "r", "line": 3}]}"#
                .to_owned(),
        ),
        (
            "wrong type",
            r#"{"allow_list": [{"file": 3, "item": "f", "reason": "r"}]}"#.to_owned(),
        ),
        ("trailing text", format!(r#"{{"allow_list": [{entry}]}} x"#)),
    ] {
        assert!(
            matches!(
                EdgeRules::from_json(&text),
                Err(Error::RulesMalformed { .. })
            ),
            "{label}"
        );
    }
}

/// FR-064-AC-9: an empty reason (also whitespace only) is a rule error
/// naming the entry.
#[trace("TC-162", "FR-064-AC-9")]
#[test]
fn an_empty_reason_is_a_rule_error() {
    let tree = Tree::new();
    tree.write("src/lib.rs", CLEAN);
    for reason in ["", "  \t"] {
        let mut request = request(tree.path(), &["src"]);
        request.rules = EdgeRules {
            allow_list: vec![AllowListEntry {
                file: "src/lib.rs".to_owned(),
                item: "f".to_owned(),
                reason: reason.to_owned(),
            }],
        };
        assert!(
            matches!(scan(&request), Err(Error::RuleReasonEmpty { file, item })
                if file == "src/lib.rs" && item == "f"),
            "{reason:?}"
        );
    }
}

/// FR-064-AC-9: an entry path that leaves the base, is absolute outside it,
/// or names no file is a rule error.
#[trace("TC-162", "FR-064-AC-9")]
#[test]
fn an_entry_path_outside_the_base_is_a_rule_error() {
    let tree = Tree::new();
    tree.write("src/lib.rs", CLEAN);
    for file in ["../other.rs", "src/../../other.rs", "/etc/passwd", ".", ""] {
        let mut request = request(tree.path(), &["src"]);
        request.rules = EdgeRules {
            allow_list: vec![AllowListEntry {
                file: file.to_owned(),
                item: "f".to_owned(),
                reason: "r".to_owned(),
            }],
        };
        assert!(
            matches!(scan(&request), Err(Error::RulePathOutsideBase { file: named }) if named == file),
            "{file:?}"
        );
    }
}
