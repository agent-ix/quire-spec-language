// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-064-AC-13: the `ix-trace-rs` marker leaves a marked function unchanged
//! and rejects an argument with one error that retains the function.
use std::path::{Path, PathBuf};
use std::process::Command;

use ix_trace_rs::trace;

use crate::Tree;

/// The directory of the `ix-trace-rs` package the workspace resolved.
fn marker_crate() -> PathBuf {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned());
    let output = Command::new(cargo)
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .args(["metadata", "--format-version", "1", "--offline"])
        .output()
        .expect("cargo metadata runs");
    assert!(output.status.success(), "cargo metadata failed");
    let metadata: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("cargo metadata emits JSON");
    let manifest = metadata["packages"]
        .as_array()
        .expect("a package list")
        .iter()
        .find(|package| package["name"] == "ix-trace-rs")
        .expect("ix-trace-rs is resolved")["manifest_path"]
        .as_str()
        .expect("a manifest path")
        .to_owned();
    Path::new(&manifest)
        .parent()
        .expect("a package directory")
        .to_path_buf()
}

/// A consumer crate whose `main` is `body`, built with `cargo` offline
/// against the resolved `ix-trace-rs`.
fn build_consumer(tree: &Tree, subcommand: &str, body: &str) -> std::process::Output {
    tree.write(
        "Cargo.toml",
        &format!(
            "[package]\nname = \"marker-consumer\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n\
             [dependencies]\nix-trace-rs = {{ path = {:?} }}\n\n[workspace]\n",
            marker_crate()
        ),
    );
    tree.write("src/main.rs", body);
    Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned()))
        .current_dir(tree.path())
        .env("CARGO_TARGET_DIR", tree.path().join("target"))
        .args([subcommand, "--offline", "--quiet"])
        .output()
        .expect("cargo runs")
}

#[trace("TC-162", "FR-064-AC-13")]
#[test]
fn a_marked_function_behaves_as_the_unmarked_function() {
    let tree = Tree::new();
    let output = build_consumer(
        &tree,
        "run",
        "use ix_trace_rs::string_edge;\n\
         #[string_edge]\n\
         fn marked(kind: &str) -> bool { kind == \"a\" }\n\
         fn unmarked(kind: &str) -> bool { kind == \"a\" }\n\
         fn main() {\n\
         assert_eq!(marked(\"a\"), unmarked(\"a\"));\n\
         assert_eq!(marked(\"b\"), unmarked(\"b\"));\n\
         println!(\"{}{}\", marked(\"a\"), marked(\"b\"));\n\
         }\n",
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "truefalse");
}

#[trace("TC-162", "FR-064-AC-13")]
#[test]
fn an_argument_is_one_error_and_the_item_is_retained() {
    let tree = Tree::new();
    let output = build_consumer(
        &tree,
        "check",
        "#[ix_trace_rs::string_edge(anything)]\n\
         fn marked() -> u8 { 1 }\n\
         fn main() { assert_eq!(marked(), 1); }\n",
    );
    assert!(!output.status.success(), "an argument must not compile");
    let stderr = String::from_utf8_lossy(&output.stderr);
    let errors: Vec<_> = stderr
        .lines()
        .filter(|line| line.starts_with("error") && !line.starts_with("error: could not compile"))
        .collect();
    assert_eq!(
        errors,
        ["error: #[string_edge] takes no arguments"],
        "{stderr}"
    );
}

#[trace("TC-162", "FR-064-AC-13")]
#[test]
fn the_annotated_item_is_kept_so_its_own_errors_still_show() {
    let tree = Tree::new();
    let output = build_consumer(
        &tree,
        "check",
        "#[ix_trace_rs::string_edge(anything)]\n\
         fn marked() -> u8 { \"not a u8\" }\n\
         fn main() { let _ = marked(); }\n",
    );
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("error: #[string_edge] takes no arguments"),
        "{stderr}"
    );
    assert!(
        stderr.contains("error[E0308]"),
        "the marker dropped the item, so its body was never checked: {stderr}"
    );
}
