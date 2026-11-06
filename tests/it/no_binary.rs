// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-771: the root package builds no executable (FR-287-AC-1) and its
//! library `command` API returns the typed outcomes the driver maps to exit
//! codes without writing to a stream (FR-287-AC-2).
use crate::command_harness::{compile_file, run_file};
use ix_trace_rs::trace;
use serde_json::{json, Value};
use std::path::Path;
use std::process::Command;

const FIXTURE: &str = "tests/fixtures/spine-compile.native";

/// Whether `package` is a developer-tool package FR-287-AC-1 admits a `bin`
/// target in: `xtask`, `qsl-string-edge`, `qsl-bench`, or a package whose
/// manifest directory is under the workspace's `tools/`.
fn is_developer_tool(package: &Value, root: &Path) -> bool {
    let name = package["name"].as_str().expect("name");
    let manifest = Path::new(package["manifest_path"].as_str().expect("manifest"));
    matches!(name, "xtask" | "qsl-string-edge" | "qsl-bench")
        || manifest
            .strip_prefix(root)
            .is_ok_and(|relative| relative.starts_with("tools"))
}

/// FR-287-AC-1: a `bin` target exists only in the developer-tool packages,
/// the root package has no `default_run`, and no other workspace member lists
/// a developer-tool package as a normal or build dependency.
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
    let root = Path::new(metadata["workspace_root"].as_str().expect("workspace root"));
    let packages = metadata["packages"].as_array().expect("packages");
    let tools: Vec<&str> = packages
        .iter()
        .filter(|package| is_developer_tool(package, root))
        .map(|package| package["name"].as_str().expect("name"))
        .collect();
    assert!(tools.contains(&"xtask"), "{tools:?}");
    for package in packages {
        let name = package["name"].as_str().expect("name");
        let has_bin = package["targets"]
            .as_array()
            .expect("targets")
            .iter()
            .any(|target| {
                target["kind"]
                    .as_array()
                    .is_some_and(|kinds| kinds.iter().any(|kind| kind == "bin"))
            });
        let tool = is_developer_tool(package, root);
        assert!(tool || !has_bin, "{name} must not list a bin target");
        if name == "quire-spec-language" {
            assert!(!has_bin);
            assert!(package["default_run"].is_null(), "{name} sets default_run");
        }
        if !tool {
            for dependency in package["dependencies"].as_array().expect("dependencies") {
                // `kind` is null for a normal dependency; a dev-dependency is
                // admitted.
                let kind = dependency["kind"].as_str();
                let dependency_name = dependency["name"].as_str().expect("dependency name");
                assert!(
                    kind == Some("dev") || !tools.contains(&dependency_name),
                    "{name} depends on developer tool {dependency_name} ({kind:?})"
                );
            }
        }
    }
}

/// FR-287-AC-2: `command` called in process with FR-027's compile request over
/// the fixture succeeds with exit code 0, and with FR-100-AC-6's zero
/// `work_units` run request is incomplete with exit code 22. The calls return
/// their outcomes; they hold no handle on stdout or stderr, so none receives
/// a byte.
#[trace("TC-771", "FR-287-AC-2")]
#[test]
fn command_returns_typed_outcomes_without_stream_writes() {
    let directory = tempfile::tempdir().unwrap();
    std::fs::copy(FIXTURE, directory.path().join("program.native")).unwrap();
    let source = json!({"file":"program.native","authority":"agent-ix","identity":"test:spine",
        "document":"Spine","formal_revision":1});
    let compile = json!({"format":"native-compile/1","request":{
        "models":[],"program":{"source":source}}});
    let compile_path = directory.path().join("compile.json");
    std::fs::write(&compile_path, serde_json::to_vec(&compile).unwrap()).unwrap();
    let compiled = compile_file(&compile_path);
    assert_eq!(compiled.status.code(), Some(0));
    assert!(!compiled.stdout.is_empty());
    assert!(compiled.stderr.is_empty());

    let run = json!({"format":"native-run/1","request":{
        "models":[],"program":{"source":source},
        "call":{"function":"seven","arguments":[],"accounting":{"work_units":0}}}});
    let run_path = directory.path().join("run.json");
    std::fs::write(&run_path, serde_json::to_vec(&run).unwrap()).unwrap();
    let ran = run_file(&run_path);
    assert_eq!(ran.status.code(), Some(22));
    let document: Value = serde_json::from_slice(&ran.stdout).unwrap();
    assert_eq!(
        document["outcome"],
        json!({"kind": "incomplete", "limit": "work_units"})
    );
    assert!(ran.stderr.is_empty());
}
