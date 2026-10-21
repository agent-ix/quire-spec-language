// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-064 through the `qsl-string-edge` command: exit codes, reports and
//! path resolution.
use std::path::Path;
use std::process::{Command, Output};

use ix_trace_rs::trace;

use crate::{Tree, CLEAN, VIOLATION};

const RULES: &str = r#"{"allow_list": []}"#;

fn run(current_dir: &Path, arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_qsl-string-edge"))
        .current_dir(current_dir)
        .args(arguments)
        .output()
        .expect("the command runs")
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).expect("utf-8 stdout")
}

fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("utf-8 stderr")
}

/// FR-064-AC-4 and AC-7: exit 0 on a clean report, 1 on violations naming
/// the file and line, the same ordered report from another working
/// directory with absolute paths, and an unselected crate unscanned.
#[trace("TC-162", "FR-064-AC-4", "FR-064-AC-7")]
#[test]
fn exit_zero_when_clean_one_with_violations_naming_file_and_line() {
    let tree = Tree::new();
    tree.write("rules.json", RULES);
    tree.write("reader/src/lib.rs", CLEAN);
    tree.write("emitter/src/lib.rs", CLEAN);
    tree.write("unselected/src/lib.rs", VIOLATION);
    let clean = run(
        tree.path(),
        &[
            "--base-dir",
            ".",
            "--crate-root",
            "reader/src",
            "--crate-root",
            "emitter/src",
            "--rules",
            "rules.json",
        ],
    );
    assert_eq!(clean.status.code(), Some(0));
    assert_eq!(
        stdout(&clean),
        "string-edge: no unmarked, unlisted string comparison or string match found\n"
    );
    assert_eq!(stderr(&clean), "");

    tree.write("emitter/src/lib.rs", VIOLATION);
    tree.write("reader/src/lib.rs", &format!("\n\n{VIOLATION}"));
    let violating = run(
        tree.path(),
        &[
            "--base-dir",
            ".",
            "--crate-root",
            "reader/src",
            "--crate-root",
            "emitter/src",
            "--rules",
            "rules.json",
        ],
    );
    assert_eq!(violating.status.code(), Some(1));
    let expected = "string-edge: 2 unmarked, unlisted occurrence(s):\n  \
                    emitter/src/lib.rs:1 in accepted (branch-gating)\n  \
                    reader/src/lib.rs:3 in accepted (branch-gating)\n";
    assert_eq!(stdout(&violating), expected);

    let elsewhere = Tree::new();
    let base = tree.path().to_str().expect("utf-8");
    let reader = tree.path().join("reader/src");
    let emitter = tree.path().join("emitter/src");
    let rules = tree.path().join("rules.json");
    let absolute = run(
        elsewhere.path(),
        &[
            "--base-dir",
            base,
            "--crate-root",
            reader.to_str().expect("utf-8"),
            "--crate-root",
            emitter.to_str().expect("utf-8"),
            "--rules",
            rules.to_str().expect("utf-8"),
        ],
    );
    assert_eq!(absolute.status.code(), Some(1));
    assert_eq!(stdout(&absolute), expected);
}

/// FR-064-AC-4/AC-7: `--base-dir` resolves relative to the working
/// directory, and crate roots and rules relative to that base.
#[trace("TC-162", "FR-064-AC-4", "FR-064-AC-7")]
#[test]
fn a_relative_base_dir_resolves_against_the_working_directory() {
    let tree = Tree::new();
    tree.write("ws/rules.json", RULES);
    tree.write("ws/crate/src/lib.rs", VIOLATION);
    let output = run(
        tree.path(),
        &[
            "--base-dir",
            "ws",
            "--crate-root",
            "crate/src",
            "--rules",
            "rules.json",
        ],
    );
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        stdout(&output),
        "string-edge: 1 unmarked, unlisted occurrence(s):\n  crate/src/lib.rs:1 in accepted (branch-gating)\n"
    );
}

/// FR-064-AC-8: the command applies the consumer rules the library does: a
/// display-only entry hides its occurrence, and an entry at a branch-gating
/// item (marked or not) is a rule error, exit 2.
#[trace("TC-162", "FR-064-AC-8")]
#[test]
fn the_command_applies_allow_list_entries_and_refuses_branch_gating_ones() {
    let tree = Tree::new();
    tree.write(
        "src/display.rs",
        "pub fn display(text: &str) { let same = text == \"shown\"; log(same); }\n",
    );
    tree.write(
        "src/gate.rs",
        "#[string_edge]\npub fn gate(text: &str) -> bool { if text == \"a\" { true } else { false } }\n",
    );
    let args = |rules: &'static str| ["--base-dir", ".", "--crate-root", "src", "--rules", rules];
    tree.write(
        "display.json",
        r#"{"allow_list": [{"file": "src/display.rs", "item": "display", "reason": "Display-only"}]}"#,
    );
    tree.write("none.json", RULES);
    tree.write(
        "gate.json",
        r#"{"allow_list": [{"file": "src/gate.rs", "item": "gate", "reason": "wrong"}]}"#,
    );
    let hidden = run(tree.path(), &args("display.json"));
    assert_eq!(hidden.status.code(), Some(0));
    tree.write("src/gate.rs", "");
    let restored = run(tree.path(), &args("none.json"));
    assert_eq!(restored.status.code(), Some(1));
    assert!(stdout(&restored).contains("src/display.rs:1"));

    tree.write(
        "src/gate.rs",
        "#[string_edge]\npub fn gate(text: &str) -> bool { if text == \"a\" { true } else { false } }\n",
    );
    tree.write("src/display.rs", "");
    let refused = run(tree.path(), &args("gate.json"));
    assert_eq!(refused.status.code(), Some(2));
    assert_eq!(stdout(&refused), "");
    assert!(
        stderr(&refused).contains("src/gate.rs:2 (gate) gates a branch"),
        "{}",
        stderr(&refused)
    );
}

/// FR-064-AC-9: every request, rule, read and parse failure exits 2 with a
/// message naming its input on stderr and nothing on stdout.
#[trace("TC-162", "FR-064-AC-9")]
#[test]
fn every_failure_exits_two_and_never_prints_a_clean_report() {
    let tree = Tree::new();
    tree.write("rules.json", RULES);
    tree.write("bad-rules.json", "{");
    tree.write("good/src/lib.rs", CLEAN);
    tree.write("broken/src/lib.rs", "fn broken( {\n");
    tree.write("empty/.keep", "");
    for (arguments, needle) in [
        (
            vec!["--base-dir", ".", "--rules", "rules.json"],
            "no crate source root",
        ),
        (
            vec![
                "--base-dir",
                ".",
                "--crate-root",
                "good/src",
                "--rules",
                "absent.json",
            ],
            "absent.json",
        ),
        (
            vec![
                "--base-dir",
                ".",
                "--crate-root",
                "good/src",
                "--rules",
                "bad-rules.json",
            ],
            "malformed rules",
        ),
        (
            vec![
                "--base-dir",
                ".",
                "--crate-root",
                "missing",
                "--rules",
                "rules.json",
            ],
            "missing",
        ),
        (
            vec![
                "--base-dir",
                ".",
                "--crate-root",
                "../escape",
                "--rules",
                "rules.json",
            ],
            "escape",
        ),
        (
            vec![
                "--base-dir",
                ".",
                "--crate-root",
                "good/src",
                "--crate-root",
                "good/src",
                "--rules",
                "rules.json",
            ],
            "named twice",
        ),
        (
            vec![
                "--base-dir",
                ".",
                "--crate-root",
                "empty",
                "--rules",
                "rules.json",
            ],
            "no production Rust file",
        ),
        (
            vec![
                "--base-dir",
                ".",
                "--crate-root",
                "broken/src",
                "--rules",
                "rules.json",
            ],
            "broken/src/lib.rs",
        ),
        (
            vec![
                "--base-dir",
                "absent",
                "--crate-root",
                "good/src",
                "--rules",
                "rules.json",
            ],
            "absent",
        ),
    ] {
        let output = run(tree.path(), &arguments);
        assert_eq!(
            output.status.code(),
            Some(2),
            "{arguments:?}: {}",
            stderr(&output)
        );
        assert_eq!(stdout(&output), "", "{arguments:?}");
        assert!(
            stderr(&output).contains(needle),
            "{arguments:?}: {}",
            stderr(&output)
        );
    }
}

/// FR-064-AC-9: a malformed command line is a usage failure, exit 2.
#[trace("TC-162", "FR-064-AC-9")]
#[test]
fn a_malformed_command_line_is_a_usage_failure() {
    let tree = Tree::new();
    for arguments in [
        vec![],
        vec!["--base-dir", ".", "--crate-root", "src"],
        vec!["--crate-root", "src", "--rules", "rules.json"],
        vec![
            "--base-dir",
            ".",
            "--base-dir",
            ".",
            "--crate-root",
            "src",
            "--rules",
            "r.json",
        ],
        vec![
            "--base-dir",
            ".",
            "--crate-root",
            "src",
            "--rules",
            "a.json",
            "--rules",
            "b.json",
        ],
        vec![
            "--base-dir",
            ".",
            "--crate-root",
            "src",
            "--rules",
            "r.json",
            "--unknown",
            "x",
        ],
        vec!["--base-dir", ".", "--crate-root", "src", "--rules"],
    ] {
        let output = run(tree.path(), &arguments);
        assert_eq!(output.status.code(), Some(2), "{arguments:?}");
        assert!(
            stderr(&output).starts_with("usage: qsl-string-edge"),
            "{arguments:?}"
        );
        assert_eq!(stdout(&output), "");
    }
}

/// FR-064-AC-9: an existing root outside the base, overlapping roots and a
/// non-directory root each exit 2 naming the root.
#[trace("TC-162", "FR-064-AC-9")]
#[test]
fn outside_overlapping_and_non_directory_roots_exit_two() {
    let tree = Tree::new();
    let outside = Tree::new();
    outside.write("src/lib.rs", CLEAN);
    tree.write("ws/rules.json", RULES);
    tree.write("ws/a/src/lib.rs", CLEAN);
    tree.write("ws/a/src/inner/lib.rs", CLEAN);
    tree.write("ws/a/Cargo.toml", "");
    let outside_root = outside.path().join("src");
    let outside_root = outside_root.to_str().expect("utf-8");
    for (roots, needle) in [
        (vec![outside_root], "is outside the base directory"),
        (
            vec!["a/src", "a/src/inner"],
            "lies inside crate source root",
        ),
        (vec!["a/Cargo.toml"], "is not a directory"),
    ] {
        let mut arguments = vec!["--base-dir", "ws"];
        for root in &roots {
            arguments.extend(["--crate-root", root]);
        }
        arguments.extend(["--rules", "rules.json"]);
        let output = run(tree.path(), &arguments);
        assert_eq!(output.status.code(), Some(2), "{roots:?}");
        assert_eq!(stdout(&output), "");
        assert!(
            stderr(&output).contains(needle),
            "{roots:?}: {}",
            stderr(&output)
        );
    }
}

/// FR-064-AC-7: the binary and the library report the same file, line, item
/// and branch classification, and the binary prints the library's report.
#[trace("TC-162", "FR-064-AC-7")]
#[test]
fn the_binary_and_the_library_report_the_same_item() {
    let tree = Tree::new();
    tree.write("rules.json", RULES);
    tree.write(
        "src/lib.rs",
        "pub struct T;\nimpl T {\n    pub fn kind(v: &str) -> bool { if v == \"a\" { true } else { false } }\n}\n",
    );
    let output = run(
        tree.path(),
        &[
            "--base-dir",
            ".",
            "--crate-root",
            "src",
            "--rules",
            "rules.json",
        ],
    );
    let report = qsl_string_edge::scan(&crate::request(tree.path(), &["src"])).expect("scan");
    assert_eq!(report.occurrences.len(), 1);
    assert_eq!(report.occurrences[0].item, "T::kind");
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stdout(&output), report.render());
    assert!(stdout(&output).contains("src/lib.rs:3 in T::kind (branch-gating)"));
}
