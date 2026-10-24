// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-064's worked consumer run: a non-QSL crate outside any QSL checkout
//! scanned by the command, clean and then violating; and the install route
//! from the authoritative Git source.
use std::fs;
use std::path::Path;
use std::process::Command;

use ix_trace_rs::trace;

use crate::{Tree, CLEAN, VIOLATION};

const MANIFEST: &str =
    "[package]\nname = \"consumer-example\"\nversion = \"0.1.0\"\nedition = \"2021\"\n";
const RULES: &str = "{\"allow_list\": []}\n";
const SCAN: [&str; 6] = [
    "--base-dir",
    ".",
    "--crate-root",
    "src",
    "--rules",
    "edge-rules.json",
];

fn consumer(tree: &Tree) {
    tree.write("Cargo.toml", MANIFEST);
    tree.write("edge-rules.json", RULES);
    tree.write("src/lib.rs", CLEAN);
}

/// FR-064-AC-10, built binary: the documented consumer fixture, in a
/// directory with no QSL name, exits 0 when clean and, with its production
/// body replaced by an unmarked literal-string dispatch, exits 1 naming the
/// consumer file and line.
#[trace("TC-162", "FR-064-AC-10")]
#[test]
fn the_worked_consumer_crate_passes_clean_and_fails_on_an_unmarked_dispatch() {
    let tree = Tree::new();
    consumer(&tree);
    assert!(!tree.path().to_string_lossy().to_lowercase().contains("qsl"));
    let binary = env!("CARGO_BIN_EXE_qsl-string-edge");
    let run = || {
        Command::new(binary)
            .current_dir(tree.path())
            .args(SCAN)
            .output()
            .expect("the command runs")
    };
    let clean = run();
    assert_eq!(clean.status.code(), Some(0));
    tree.write("src/lib.rs", VIOLATION);
    let failing = run();
    assert_eq!(failing.status.code(), Some(1));
    assert_eq!(
        String::from_utf8(failing.stdout).expect("utf-8"),
        "string-edge: 1 unmarked, unlisted occurrence(s):\n  src/lib.rs:1 in accepted (branch-gating)\n"
    );
}

/// FR-064-AC-10, marker: a consumer that marks its intake with the
/// qualified `#[ix_trace_rs::string_edge]` attribute passes.
#[trace("TC-162", "FR-064-AC-10")]
#[test]
fn a_consumer_intake_marked_with_the_qualified_attribute_passes() {
    let tree = Tree::new();
    consumer(&tree);
    tree.write(
        "src/lib.rs",
        "#[ix_trace_rs::string_edge]\n\
         pub fn parse(kind: &str) -> Option<u8> { match kind { \"accepted\" => Some(1), _ => None } }\n",
    );
    let output = Command::new(env!("CARGO_BIN_EXE_qsl-string-edge"))
        .current_dir(tree.path())
        .args(SCAN)
        .output()
        .expect("the command runs");
    assert_eq!(output.status.code(), Some(0));
}

/// The authoritative Git route FR-064 documents, and the branch to take it
/// from: `main` once this crate is merged, a pushed branch before then
/// (`QSL_STRING_EDGE_BRANCH`).
const REPOSITORY: &str = "https://github.com/agent-ix/quire-spec-language";

fn cargo(current_dir: &Path, arguments: &[&str]) -> std::process::Output {
    Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned()))
        .current_dir(current_dir)
        .args(arguments)
        .output()
        .expect("cargo runs")
}

/// FR-064-AC-10, install route: `cargo install --git` of `qsl-string-edge`
/// into an empty root, then the installed binary run from outside QSL on the
/// worked consumer crate (clean, then violating); then a separate Rust client
/// with the Git library dependency calls the public API on the same source.
/// Needs network and access to the private repository, so it is run on
/// request: `cargo test -p qsl-string-edge --test it -- --ignored`.
#[ignore = "installs from the private Git source over the network; run with --ignored"]
#[trace("TC-162", "FR-064-AC-10")]
#[test]
fn the_documented_git_install_and_library_dependency_work_outside_qsl() {
    let branch = std::env::var("QSL_STRING_EDGE_BRANCH").unwrap_or_else(|_| "main".to_owned());
    let tree = Tree::new();
    let install_root = tree.path().join("install");
    let work = tree.path().join("work");
    let consumer_dir = work.join("string-edge-consumer");
    fs::create_dir_all(&consumer_dir).expect("mkdir");

    let install = cargo(
        &work,
        &[
            "install",
            "--git",
            REPOSITORY,
            "--branch",
            &branch,
            "--root",
            install_root.to_str().expect("utf-8"),
            "qsl-string-edge",
        ],
    );
    assert!(
        install.status.success(),
        "cargo install failed: {}",
        String::from_utf8_lossy(&install.stderr)
    );
    let installed = install_root.join("bin/qsl-string-edge");

    fs::write(consumer_dir.join("Cargo.toml"), MANIFEST).expect("write");
    fs::write(consumer_dir.join("edge-rules.json"), RULES).expect("write");
    fs::create_dir_all(consumer_dir.join("src")).expect("mkdir");
    fs::write(consumer_dir.join("src/lib.rs"), CLEAN).expect("write");
    let scan = || {
        Command::new(&installed)
            .current_dir(&consumer_dir)
            .args(SCAN)
            .output()
            .expect("the installed command runs")
    };
    assert_eq!(scan().status.code(), Some(0));
    fs::write(consumer_dir.join("src/lib.rs"), VIOLATION).expect("write");
    let failing = scan();
    assert_eq!(failing.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&failing.stdout).contains("src/lib.rs:1"));

    // A separate Rust client: a Git dependency, no copied scanner sources.
    let client = work.join("client");
    fs::create_dir_all(client.join("src")).expect("mkdir");
    fs::write(
        client.join("Cargo.toml"),
        format!(
            "[package]\nname = \"client\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n\
             [dependencies]\nqsl-string-edge = {{ git = \"{REPOSITORY}\", branch = \"{branch}\" }}\n"
        ),
    )
    .expect("write");
    fs::write(
        client.join("src/main.rs"),
        format!(
            "use qsl_string_edge::{{scan, EdgeRules, ScanRequest}};\n\
             fn main() {{\n\
             let report = scan(&ScanRequest {{\n\
             base_dir: \"{base}\".into(),\n\
             crate_roots: vec![\"src\".into()],\n\
             rules: EdgeRules::default(),\n\
             }})\n\
             .expect(\"scan completes\");\n\
             assert_eq!(report.occurrences.len(), 1);\n\
             assert_eq!(report.occurrences[0].file, \"src/lib.rs\");\n\
             assert_eq!(report.occurrences[0].line, 1);\n\
             }}\n",
            base = consumer_dir.to_str().expect("utf-8")
        ),
    )
    .expect("write");
    let run = cargo(&client, &["run", "--quiet"]);
    assert!(
        run.status.success(),
        "client failed: {}",
        String::from_utf8_lossy(&run.stderr)
    );
    let copied = fs::read_dir(client.join("src"))
        .expect("read")
        .map(|entry| entry.expect("entry").file_name())
        .collect::<Vec<_>>();
    assert_eq!(copied, [std::ffi::OsString::from("main.rs")]);
}
