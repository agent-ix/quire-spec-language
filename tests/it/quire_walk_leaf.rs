// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-898 step 1 (FR-356-AC-1): the walker toolkit `quire-walk` is a
//! shared leaf with no features and no dependency, so its dependency tree
//! is `core` and `alloc` alone. `make quire-walk-no-std` builds it for a
//! bare-metal target with no `std`; `tools/arch-lint`'s
//! `tc_arch_lint_metadata_009` checks the direction rule over it.

use ix_trace_rs::trace;

/// The resolved `cargo metadata` of the whole workspace.
fn metadata() -> serde_json::Value {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned());
    let output = std::process::Command::new(cargo)
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .args(["metadata", "--format-version", "1", "--offline"])
        .output()
        .expect("cargo metadata runs");
    assert!(
        output.status.success(),
        "cargo metadata failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("cargo metadata emits JSON")
}

#[trace("TC-898", "FR-356-AC-1")]
#[test]
fn quire_walk_has_no_features_and_no_dependency() {
    let metadata = metadata();
    let package = metadata["packages"]
        .as_array()
        .expect("a package list")
        .iter()
        .find(|package| package["name"] == "quire-walk")
        .expect("quire-walk is a workspace package");
    assert_eq!(
        package["features"],
        serde_json::json!({}),
        "quire-walk declares no features"
    );
    let shipped: Vec<&serde_json::Value> = package["dependencies"]
        .as_array()
        .expect("a dependency list")
        .iter()
        .filter(|dependency| dependency["kind"].as_str() != Some("dev"))
        .collect();
    assert!(
        shipped.is_empty(),
        "quire-walk has no normal or build dependency: {shipped:?}"
    );
    let id = package["id"].as_str().expect("a package id");
    let node = metadata["resolve"]["nodes"]
        .as_array()
        .expect("a resolve graph")
        .iter()
        .find(|node| node["id"] == id)
        .expect("quire-walk is resolved");
    let resolved_shipped: Vec<&serde_json::Value> = node["deps"]
        .as_array()
        .expect("resolved dependencies")
        .iter()
        .filter(|dependency| {
            dependency["dep_kinds"]
                .as_array()
                .is_some_and(|kinds| kinds.iter().any(|kind| kind["kind"] != "dev"))
        })
        .collect();
    assert!(
        resolved_shipped.is_empty(),
        "quire-walk resolves no normal or build dependency: {resolved_shipped:?}"
    );
    assert_eq!(
        node["features"],
        serde_json::json!([]),
        "the workspace build enables no feature of quire-walk"
    );
}
