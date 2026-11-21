// SPDX-License-Identifier: AGPL-3.0-or-later
//! Recursive package artifacts are sufficient after every source is removed.

use ix_trace_rs::trace;
use serde_json::{json, Value};
use std::path::Path;

const HEADER: &str =
    "language \"ix:native\" edition \"1-draft\";\nprofile v = \"filament.value.complete/v1\";\n";

fn source(file: &str) -> Value {
    json!({"file": file, "authority": "agent-ix", "identity": format!("test:dependency:{file}"), "document": "DependencyHandoff", "formal_revision": 1})
}

fn prepare(directory: &Path, diamond: bool) -> Value {
    let libraries = if diamond {
        vec![("test/shared", "shared.native", "function answer using v(): Integer pure { 21 }".to_owned()),
            ("test/left", "left.native", "import \"test/shared\" as s; function answer using v(): Integer pure { s::answer() }".to_owned()),
            ("test/right", "right.native", "import \"test/shared\" as s; function answer using v(): Integer pure { s::answer() }".to_owned())]
    } else {
        vec![
            (
                "test/a",
                "a.native",
                "function answer using v(): Integer pure { 42 }".to_owned(),
            ),
            (
                "test/z",
                "z.native",
                "import \"test/a\" as a; function answer using v(): Integer pure { a::answer() }"
                    .to_owned(),
            ),
        ]
    };
    let unit = if diamond {
        "import \"test/left\" as l; import \"test/right\" as r; function answer using v(): Integer pure { l::answer() + r::answer() }"
    } else {
        "import \"test/z\" as z; function answer using v(): Integer pure { z::answer() }"
    };
    std::fs::write(directory.join("unit.native"), format!("{HEADER}{unit}\n")).unwrap();
    for (_, file, body) in &libraries {
        std::fs::write(directory.join(file), format!("{HEADER}{body}\n")).unwrap();
    }
    let libraries: Vec<_> = libraries
        .into_iter()
        .map(|(identity, file, _)| json!({"identity": identity, "source": source(file)}))
        .collect();
    let request = json!({"models": [], "program": {"source": source("unit.native")}, "libraries": libraries,
        "handoff": {"directory": "handoff", "package_name": "dependency-root", "native_join": {"work_units": 1_u64 << 40, "storage": 1_u64 << 24}}});
    let path = directory.join("compile.json");
    std::fs::write(
        &path,
        serde_json::to_vec(&json!({"format": "native-compile/1", "request": request})).unwrap(),
    )
    .unwrap();
    fsl_command::command::compile(&path)
        .expect("the transitive unit compiles and writes its artifact closure");
    request
}

fn run(directory: &Path, package: Value) -> (u8, Value) {
    let path = directory.join("run.json");
    std::fs::write(&path, serde_json::to_vec(&json!({"format": "native-run/1", "request": {"models": [], "package": package, "call": {"function": "answer", "arguments": []}}})).unwrap()).unwrap();
    match fsl_command::command::run(&path, &crate::negotiation::Fr290) {
        Ok(result) => (result.exit_code, result.value.as_value().clone()),
        Err(error) => (error.category().exit_code(), error.value().unwrap()),
    }
}

#[test]
#[trace("FR-517-AC-27", "FR-850-AC-12", "TC-1312", "TC-2410")]
fn transitive_library_calls_run_with_only_handoff_artifacts() {
    let directory = tempfile::tempdir().unwrap();
    let request = prepare(directory.path(), false);
    let manifest: Value = serde_json::from_slice(
        &std::fs::read(directory.path().join("handoff/dependencies.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        manifest,
        json!({"format": "filament.checked-handoff.dependencies/1", "dependencies": [{"identity": "test/a", "directory": "dependencies/0"}, {"identity": "test/z", "directory": "dependencies/1"}]})
    );
    std::fs::remove_dir_all(directory.path().join("handoff/sources")).unwrap();
    std::fs::remove_file(directory.path().join("unit.native")).unwrap();
    for library in request["libraries"].as_array().unwrap() {
        std::fs::remove_file(
            directory
                .path()
                .join(library["source"]["file"].as_str().unwrap()),
        )
        .unwrap();
    }
    let (exit, result) = run(directory.path(), json!({"directory": "handoff"}));
    assert_eq!(exit, 0, "{result}");
    assert_eq!(
        result["outcome"]["value"],
        json!({"integer": "42"}),
        "{result}"
    );
    let (exit, result) = run(
        directory.path(),
        json!({"file": "handoff/checked-package.json"}),
    );
    assert_eq!(
        exit, 20,
        "bare files cannot discover neighboring dependency evidence: {result}"
    );
}

#[test]
#[trace("FR-517-AC-28", "FR-850-AC-13", "TC-1312", "TC-2410")]
fn diamond_closure_shares_one_artifact_and_missing_transitive_supply_refuses() {
    let directory = tempfile::tempdir().unwrap();
    prepare(directory.path(), true);
    let manifest_file = directory.path().join("handoff/dependencies.json");
    let manifest: Value = serde_json::from_slice(&std::fs::read(&manifest_file).unwrap()).unwrap();
    assert_eq!(manifest["dependencies"].as_array().unwrap().len(), 3);
    let (exit, result) = run(directory.path(), json!({"directory": "handoff"}));
    assert_eq!(exit, 0, "{result}");
    let mut omitted = manifest;
    omitted["dependencies"].as_array_mut().unwrap().pop();
    std::fs::write(manifest_file, serde_json::to_vec(&omitted).unwrap()).unwrap();
    let (exit, result) = run(directory.path(), json!({"directory": "handoff"}));
    assert_eq!(exit, 20, "{result}");
    assert!(result.to_string().contains("missing-selection"), "{result}");
    assert!(
        result
            .to_string()
            .contains("dependencies/0/checked-package.json"),
        "{result}"
    );
    assert!(result.to_string().contains("test/shared"), "{result}");
}

#[test]
#[trace("FR-850-AC-13", "FR-850-AC-14", "TC-2410")]
fn malformed_manifests_and_missing_indexed_witnesses_refuse_before_a_call() {
    let directory = tempfile::tempdir().unwrap();
    prepare(directory.path(), false);
    let handoff = directory.path().join("handoff");
    let manifest_file = handoff.join("dependencies.json");
    let original = std::fs::read(&manifest_file).unwrap();
    for (raw, cause, pointer) in [
        ("not json", "malformed-file", ""),
        ("{\"format\":7,\"dependencies\":[]}", "malformed-file", "/format"),
        ("{\"format\":\"filament.checked-handoff.dependencies/2\",\"dependencies\":[]}", "unknown-format", "/format"),
        ("{\"format\":\"filament.checked-handoff.dependencies/1\",\"format\":\"filament.checked-handoff.dependencies/1\",\"dependencies\":[]}", "malformed-file", "/format"),
        ("{\"format\":\"filament.checked-handoff.dependencies/1\",\"dependencies\":[{\"identity\":\"test/a\",\"directory\":\"../escape\"}]}", "invalid-value", "/dependencies/0/directory"),
    ] {
        std::fs::write(&manifest_file, raw).unwrap();
        let (exit, result) = run(directory.path(), json!({"directory": "handoff"}));
        assert_eq!(exit, 20, "{raw}: {result}");
        assert_eq!(result["stage"], "request", "{result}");
        assert_eq!(result["code"], "invalid_package", "{result}");
        assert_eq!(result["cause"], cause, "{result}");
        assert_eq!(result["details"]["path"], pointer, "{result}");
        assert!(result.to_string().contains("dependencies.json"), "{result}");
    }
    std::fs::write(&manifest_file, original).unwrap();
    for file in [
        "dependencies.json",
        "dependencies/0/checked-package.json",
        "dependencies/0/identities.json",
        "dependencies/0/equality.json",
        "dependencies/1/identities.json",
        "dependencies/1/equality.json",
    ] {
        let path = handoff.join(file);
        let bytes = std::fs::read(&path).unwrap();
        std::fs::remove_file(&path).unwrap();
        let (exit, result) = run(directory.path(), json!({"directory": "handoff"}));
        assert_eq!(exit, 20, "{file}: {result}");
        assert_eq!(result["code"], "invalid_package", "{result}");
        assert_eq!(result["cause"], "missing-file", "{result}");
        assert!(result.to_string().contains(file), "{result}");
        std::fs::write(path, bytes).unwrap();
    }
    let (exit, result) = run(directory.path(), json!({"directory": "handoff"}));
    assert_eq!(exit, 0, "unchanged positive control: {result}");
}
