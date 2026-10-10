// SPDX-License-Identifier: AGPL-3.0-or-later
//! Process-boundary checks for clean-build recipes, not compiler qualification.

use ix_trace_rs::trace;
use std::path::Path;
use std::process::Command;

struct Recipe {
    directory: tempfile::TempDir,
}

impl Recipe {
    fn new() -> Self {
        let directory = tempfile::tempdir().expect("own the process recorder directory");
        let output = Command::new("rustc")
            .arg(
                Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("src/ci_clean_build_tests/argv_recorder.rs"),
            )
            .args(["--edition=2021", "-o"])
            .arg(directory.path().join("cargo"))
            .output()
            .expect("compile the Rust argv recorder");
        assert!(
            output.status.success(),
            "recorder compile: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        Self { directory }
    }

    fn assert_arguments(&self, root: Option<&str>, command_line: bool) {
        let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("workspace parent");
        let log = self.directory.path().join("arguments");
        std::fs::write(&log, []).expect("clear the argument receipt");
        let path = std::env::join_paths(std::iter::once(self.directory.path().to_owned()).chain(
            std::env::split_paths(&std::env::var_os("PATH").expect("process search path")),
        ))
        .expect("prepend the owned argument recorder");
        let mut make = Command::new("make");
        make.current_dir(workspace)
            .args(["--no-print-directory", "-rR", "ci-clean-build"])
            .env_remove("MAKEFLAGS")
            .env_remove("MFLAGS")
            .env_remove("MAKEFILES")
            .env_remove("CARGO_TARGET_DIR")
            .env("PATH", path)
            .env("QSL_ARGV_LOG", &log);
        if let Some(root) = root {
            if command_line {
                make.arg(format!("CARGO_TARGET_DIR={root}"));
            } else {
                make.env("CARGO_TARGET_DIR", root);
            }
        }
        let output = make.output().expect("execute the real clean-build recipes");
        assert!(
            output.status.success(),
            "recipe process: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let bytes = std::fs::read(log).expect("read argument and environment receipts");
        let mut remaining = bytes.as_slice();
        let mut calls = Vec::new();
        while !remaining.is_empty() {
            let (presence, rest) = remaining.split_first().expect("environment presence flag");
            remaining = rest;
            let observed_root = match presence {
                0 => None,
                1 => Some(read_text(&mut remaining)),
                _ => panic!("invalid environment presence flag"),
            };
            assert_eq!(
                observed_root.as_deref(),
                root,
                "every Cargo-boundary call preserves absent, empty or literal caller environment"
            );
            let count = read_length(&mut remaining);
            let arguments: Vec<_> = (0..count).map(|_| read_text(&mut remaining)).collect();
            calls.push(arguments);
        }
        let clean = format!(
            "{}/clean",
            root.filter(|root| !root.is_empty()).unwrap_or("target")
        );
        assert_eq!(
            calls,
            vec![
                vec![
                    "build",
                    "--locked",
                    "--workspace",
                    "--no-default-features",
                    "--target-dir",
                    &clean
                ],
                vec![
                    "check",
                    "--locked",
                    "-p",
                    "quire-spec-language",
                    "--lib",
                    "--no-default-features",
                    "--features",
                    "handoff-writer",
                    "--target-dir",
                    &clean
                ],
                vec![
                    "run",
                    "--locked",
                    "--no-default-features",
                    "--",
                    "parse",
                    "agent-ix",
                    "test:parent",
                    "fixture",
                    "fixture:1",
                    "tests/fixtures/parent.native"
                ],
            ],
            "literal caller path and all three recipe argument lists are preserved"
        );
    }
}

fn read_text(bytes: &mut &[u8]) -> String {
    let len = read_length(bytes);
    let (text, rest) = bytes.split_at(len);
    *bytes = rest;
    std::str::from_utf8(text)
        .expect("UTF-8 test inputs")
        .to_owned()
}

fn read_length(bytes: &mut &[u8]) -> usize {
    let (length, rest) = bytes.split_at(8);
    *bytes = rest;
    usize::try_from(u64::from_le_bytes(
        length.try_into().expect("eight length bytes"),
    ))
    .expect("argument length fits usize")
}

/// Recipe wiring supports the writer checks; real compilation is separate.
#[trace("TC-121", "TC-138", "FR-042-AC-15", "FR-050-AC-8")]
#[test]
fn clean_build_defaults_to_target_clean() {
    let recipe = Recipe::new();
    recipe.assert_arguments(None, false);
    recipe.assert_arguments(Some(""), false);
    recipe.assert_arguments(Some(""), true);
}

/// The actual shell-delivered argv must preserve literal caller data.
#[trace("TC-121", "TC-138", "FR-042-AC-15", "FR-050-AC-8")]
#[test]
fn clean_build_uses_the_callers_target_root() {
    let recipe = Recipe::new();
    let marker = recipe.directory.path().join("must-not-execute");
    let roots = [
        "/tmp/qsl-owned-target".to_owned(),
        "209-target".to_owned(),
        "/tmp/qsl owned target".to_owned(),
        "owned target".to_owned(),
        "owned\"quoted'root".to_owned(),
        "owned$HOME${HOME}root".to_owned(),
        format!("owned`touch {}`root", marker.display()),
        format!("owned$(touch {})root", marker.display()),
        format!("owned$(shell touch {})root", marker.display()),
    ];
    for root in roots {
        for command_line in [false, true] {
            recipe.assert_arguments(Some(&root), command_line);
            assert!(!marker.exists(), "path contents are never executed");
        }
    }
}
