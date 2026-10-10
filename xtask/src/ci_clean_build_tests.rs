// SPDX-License-Identifier: AGPL-3.0-or-later
//! Regression checks for the native minimal-feature clean-build recipes.

use std::path::Path;
use std::process::Command;

fn assert_clean_build_recipe(caller_root: Option<&str>, clean_root: &str) {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask lives one level below the workspace");
    let mut make = Command::new("make");
    make.current_dir(workspace)
        .args(["--no-print-directory", "-rR", "-n", "ci-clean-build"])
        .env_remove("MAKEFLAGS")
        .env_remove("MFLAGS")
        .env_remove("MAKEFILES")
        .env_remove("CARGO_TARGET_DIR")
        .env_remove("CI_CLEAN_TARGET_DIR");
    if let Some(root) = caller_root {
        make.env("CARGO_TARGET_DIR", root);
    }
    let output = make.output().expect("run the real Makefile with make -n");
    assert!(
        output.status.success(),
        "make -n failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).expect("UTF-8 recipes");
    let expected = format!(
        "cargo build --locked --workspace --no-default-features --target-dir \"{clean_root}\"\n\
         cargo check --locked -p quire-spec-language --lib --no-default-features --features handoff-writer --target-dir \"{clean_root}\"\n\
         cargo run --locked --no-default-features -- parse agent-ix test:parent fixture fixture:1 tests/fixtures/parent.native\n"
    );
    assert_eq!(
        stdout, expected,
        "clean-build recipes must retain minimal-feature/writer/parse checks and use the selected clean child as one quoted argument"
    );
}

/// Provenance: QSL-209; recipe wiring only, real compilation is checked separately.
/// Trace: FR-042-AC-15, FR-050-AC-8
#[test]
fn clean_build_defaults_to_target_clean() {
    assert_clean_build_recipe(None, "target/clean");
    assert_clean_build_recipe(Some(""), "target/clean");
}

/// Provenance: QSL-209; recipe wiring only, real compilation is checked separately.
/// Trace: FR-042-AC-15, FR-050-AC-8
#[test]
fn clean_build_uses_the_callers_target_root() {
    for (root, clean) in [
        ("/tmp/qsl-owned-target", "/tmp/qsl-owned-target/clean"),
        ("209-target", "209-target/clean"),
        ("/tmp/qsl owned target", "/tmp/qsl owned target/clean"),
        ("owned target", "owned target/clean"),
    ] {
        assert_clean_build_recipe(Some(root), clean);
    }
}
