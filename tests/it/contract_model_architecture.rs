// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-139: QSL's production graph uses the cycle-free Contract Model substrate.

use std::{collections::BTreeMap, process::Command};

use ix_trace_rs::trace;
use serde_json::Value;

const MODEL_REVISION: &str = "53cc03c639e2e26528132d34d96dc56449df78e8";
const HISTORICAL_IR_REVISION: &str = "04eb6f849c03be23177d373549c6c272551f957d";

#[trace("TC-139", "FR-051-AC-6")]
#[test]
fn production_graph_is_cycle_free_and_historical_ir_is_test_only() {
    let output = Command::new(env!("CARGO"))
        .args(["metadata", "--format-version", "1", "--locked", "--offline"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("run Cargo's locked offline metadata resolver");
    assert!(
        output.status.success(),
        "cargo metadata failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let metadata: Value = serde_json::from_slice(&output.stdout).expect("decode cargo metadata");
    let packages = metadata["packages"]
        .as_array()
        .expect("metadata packages array");
    let by_id: BTreeMap<_, _> = packages
        .iter()
        .map(|package| {
            (
                package["id"].as_str().expect("package id"),
                (
                    package["name"].as_str().expect("package name"),
                    package["source"].as_str(),
                ),
            )
        })
        .collect();
    let resolve = metadata["resolve"].as_object().expect("metadata resolve");
    let root = resolve["root"].as_str().expect("workspace root package id");
    let nodes = resolve["nodes"].as_array().expect("metadata resolve nodes");
    let by_node: BTreeMap<_, _> = nodes
        .iter()
        .map(|node| (node["id"].as_str().expect("resolve node id"), node))
        .collect();

    let root_node = by_node.get(root).expect("workspace root resolve node");
    let direct = root_node["deps"].as_array().expect("root dependencies");
    let model = direct
        .iter()
        .find(|dependency| dependency["name"] == "quire_contract_ir")
        .expect("production Contract Model alias");
    let model_id = model["pkg"].as_str().expect("Contract Model package id");
    assert_eq!(by_id[model_id].0, "quire-contract-model");
    assert!(
        by_id[model_id]
            .1
            .is_some_and(|source| source.contains(MODEL_REVISION)),
        "production alias must select the reviewed cycle-free revision"
    );
    assert!(
        model["dep_kinds"]
            .as_array()
            .expect("model dependency kinds")
            .iter()
            .any(|kind| kind["kind"].is_null()),
        "Contract Model must be a normal production dependency"
    );

    let mut pending = vec![root];
    let mut production = std::collections::BTreeSet::new();
    while let Some(id) = pending.pop() {
        if !production.insert(id) {
            continue;
        }
        let node = by_node.get(id).expect("resolved production node");
        for dependency in node["deps"].as_array().expect("resolved dependencies") {
            let is_normal = dependency["dep_kinds"]
                .as_array()
                .expect("dependency kinds")
                .iter()
                .any(|kind| kind["kind"].is_null());
            if is_normal {
                pending.push(dependency["pkg"].as_str().expect("dependency package id"));
            }
        }
    }
    assert!(
        production
            .iter()
            .all(|id| by_id[id].0 != "quire-contract-ir"),
        "the compatibility Contract-IR package must not enter QSL's production graph"
    );

    let historical = packages
        .iter()
        .find(|package| {
            package["name"] == "quire-contract-ir"
                && package["source"]
                    .as_str()
                    .is_some_and(|source| source.contains(HISTORICAL_IR_REVISION))
        })
        .expect("explicit historical test-only Contract-IR revision");
    assert!(
        !production.contains(historical["id"].as_str().expect("historical package id")),
        "historical Contract-IR must remain outside the production graph"
    );
}
