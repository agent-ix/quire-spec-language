// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-771: the root package builds no executable (FR-287-AC-1).
use ix_trace_rs::trace;
use serde_json::Value;
use std::process::Command;

/// FR-287-AC-1: a `bin` target exists only in the developer-tool packages.
#[trace("TC-771", "FR-287-AC-1")]
#[test]
fn only_developer_tool_packages_list_a_bin_target() {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
    let output = Command::new(cargo)
        .args(["metadata", "--no-deps", "--format-version", "1"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("run cargo metadata");
    assert!(output.status.success());
    let metadata: Value = serde_json::from_slice(&output.stdout).expect("metadata JSON");
    for package in metadata["packages"].as_array().expect("packages") {
        let name = package["name"].as_str().expect("name");
        let dir = package["manifest_path"].as_str().expect("manifest");
        let allowed =
            matches!(name, "xtask" | "qsl-string-edge" | "qsl-bench") || dir.contains("/tools/");
        let bins = package["targets"]
            .as_array()
            .expect("targets")
            .iter()
            .any(|target| {
                target["kind"]
                    .as_array()
                    .is_some_and(|kinds| kinds.iter().any(|kind| kind == "bin"))
            });
        assert!(allowed || !bins, "{name} must not list a bin target");
        if name == "quire-spec-language" {
            assert!(!bins);
        }
    }
}
