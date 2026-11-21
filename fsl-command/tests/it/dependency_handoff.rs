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
                "record A { level: Integer; } function answer using v(): Integer pure { if 1 = 1 then 42 else 41 }".to_owned(),
            ),
            (
                "test/z",
                "z.native",
                "import \"test/a\" as a; record Z { level: Integer; } function answer using v(): Integer pure { if 2 = 2 then a::answer() else 41 }"
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
    run_request(
        directory,
        json!({"models": [], "package": package, "call": {"function": "answer", "arguments": []}}),
        Default::default(),
    )
}

fn run_request(
    directory: &Path,
    request: Value,
    limits: fsl_command::command::RunLimits,
) -> (u8, Value) {
    let path = directory.join("run.json");
    std::fs::write(
        &path,
        serde_json::to_vec(&json!({"format": "native-run/1", "request": request})).unwrap(),
    )
    .unwrap();
    match fsl_command::command::run_with_limits(&path, limits, &crate::negotiation::Fr290) {
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

fn analyze(directory: &Path, request: Value, replay_input_bytes: u64) -> (u8, Value) {
    let path = directory.join("analyze.json");
    std::fs::write(
        &path,
        serde_json::to_vec(&json!({"format": "native-analyze/1", "request": request})).unwrap(),
    )
    .unwrap();
    let limits = fsl_command::command::RunLimits {
        replay_input_bytes,
        ..Default::default()
    };
    match fsl_command::command::analyze_with_limits(
        &path,
        limits,
        &crate::negotiation::Fr290,
        &filament_exact::Cancel::new(),
    ) {
        Ok(result) => (result.exit_code, result.value.as_value().clone()),
        Err(error) => (
            error.category().exit_code(),
            serde_json::from_slice(
                &error
                    .outcome_document(fsl_replay::Operation::Analyze)
                    .to_bytes()
                    .unwrap(),
            )
            .unwrap(),
        ),
    }
}

#[test]
#[trace("FR-850-AC-12", "FR-850-AC-15", "TC-2410")]
fn transitive_library_claim_analysis_and_replay_equal_source_after_source_deletion() {
    let directory = tempfile::tempdir().unwrap();
    let directory = directory.path();
    let mut compile = prepare(directory, false);
    std::fs::remove_dir_all(directory.join("handoff")).unwrap();
    let unit = format!("{HEADER}\
        profile paths = \"filament.temporal.infinite-trace/v1\";\n\
        import \"test/z\" as z;\n\
        model M = \"test/counter\";\n\
        pre Space using v on M::Counter::inc {{ self.value < 3 }}\n\
        post Inc using v on M::Counter::inc {{ self.value - pre(self.value) = 1 }}\n\
        temporal Bounded using paths over (c: M::Counter) clock \"model-steps\" on origin {{ always holds(c.value <= 3) }}\n\
        temporal LibraryBound using paths over (c: M::Counter) clock \"model-steps\" on origin {{ always holds(c.value <= z::answer() - 40) }}\n");
    std::fs::write(directory.join("unit.native"), unit).unwrap();
    std::fs::write(
        directory.join("counter.json"),
        include_bytes!("../fixtures/cli-contract/counter.semantic-ir.json"),
    )
    .unwrap();
    let models = json!([{"format": "semantic-ir/2.0.0", "source": source("counter.json")}]);
    compile["models"] = models.clone();
    compile["handoff"]["package_name"] = source("unit.native")["identity"].clone();
    let compile_path = directory.join("compile.json");
    std::fs::write(
        &compile_path,
        serde_json::to_vec(&json!({"format": "native-compile/1", "request": compile})).unwrap(),
    )
    .unwrap();
    fsl_command::command::compile(&compile_path).expect("the library-dependent Counter compiles");
    std::fs::write(directory.join("start.json"), serde_json::to_vec(&json!({"format": "filament.state.snapshot/v1", "identity": {"authority": "contracts", "identity": "counter-start"}, "observation": "current", "model": {"identity": "test/counter"}, "anchor": {"kind": "initialization", "name": "counter-start"}, "populations": [{"population": "ix://test/counter/counters", "complete": true, "objects": [{"key": "counter", "type": "ix://test/counter/Counter", "fields": {"value": {"integer": "0"}}}]}]})).unwrap()).unwrap();
    let source_request = json!({"program": {"source": source("unit.native")}, "libraries": compile["libraries"], "models": models, "documents": [{"file": "start.json"}], "items": [{"claim": "LibraryBound"}, {"claim": "Bounded"}]});
    let (source_exit, source_result) = analyze(directory, source_request, 1 << 20);
    assert_eq!(
        source_result["items"][0]["record"]["result"], "refuted",
        "{source_result}"
    );
    assert_eq!(
        source_result["items"][1]["record"]["result"], "proved",
        "{source_result}"
    );
    assert_eq!(
        source_result["items"][0]["replay"]["reproduced"], true,
        "{source_result}"
    );
    let package_request = json!({"package": {"directory": "handoff"}, "models": models, "documents": [{"file": "start.json"}], "items": [{"claim": "LibraryBound"}, {"claim": "Bounded"}]});
    std::fs::remove_dir_all(directory.join("handoff/sources")).unwrap();
    for file in ["unit.native", "a.native", "z.native"] {
        std::fs::remove_file(directory.join(file)).unwrap();
    }
    let (exit, result) = analyze(directory, package_request, 1 << 20);
    assert_eq!(exit, source_exit, "{result}");
    assert_eq!(result["items"], source_result["items"], "{result}");
}

#[test]
#[trace("FR-850-AC-15", "TC-2410")]
fn artifact_intake_counts_the_unique_closure_bytes_at_exact_and_one_short_bounds() {
    let directory = tempfile::tempdir().unwrap();
    prepare(directory.path(), true);
    let handoff = directory.path().join("handoff");
    let manifest: Value =
        serde_json::from_slice(&std::fs::read(handoff.join("dependencies.json")).unwrap()).unwrap();
    let mut files = vec![
        "checked-package.json".to_owned(),
        "dependencies.json".to_owned(),
        "identities.json".to_owned(),
        "equality.json".to_owned(),
    ];
    for row in manifest["dependencies"].as_array().unwrap() {
        for file in ["checked-package.json", "identities.json", "equality.json"] {
            files.push(format!("{}/{file}", row["directory"].as_str().unwrap()));
        }
    }
    assert_eq!(files.len(), 13, "the diamond's shared triple counts once");
    let bytes = files
        .iter()
        .map(|file| std::fs::read(handoff.join(file)).unwrap().len())
        .sum::<usize>();
    let request = json!({"models": [], "package": {"directory": "handoff"}, "call": {"function": "answer", "arguments": []}});
    for (bound, expected) in [(bytes, 0), (bytes - 1, 22)] {
        let (exit, result) = run_request(
            directory.path(),
            request.clone(),
            fsl_command::command::RunLimits {
                dependent_bytes: bound,
                ..Default::default()
            },
        );
        assert_eq!(exit, expected, "bound {bound}: {result}");
        if expected == 22 {
            assert_eq!(result["code"], "resource_exhausted", "{result}");
            assert!(result.to_string().contains("dependent_bytes"), "{result}");
            assert!(result.to_string().contains(&bound.to_string()), "{result}");
            assert!(result.to_string().contains("--dependent-bytes"), "{result}");
        }
    }
}

#[test]
#[trace("FR-850-AC-16", "TC-2410")]
fn a_domain_document_selected_only_by_a_library_is_bound_to_that_library() {
    let directory = tempfile::tempdir().unwrap();
    let directory = directory.path();
    let mut compile = prepare(directory, false);
    std::fs::remove_dir_all(directory.join("handoff")).unwrap();
    std::fs::write(directory.join("a.native"), format!("{HEADER}model M = \"test/counter\"; function answer using v(): Integer pure {{ 42 }}\n")).unwrap();
    std::fs::write(
        directory.join("counter.json"),
        include_bytes!("../fixtures/cli-contract/counter.semantic-ir.json"),
    )
    .unwrap();
    let models = json!([{"format": "semantic-ir/2.0.0", "source": source("counter.json")}]);
    compile["models"] = models.clone();
    let path = directory.join("compile.json");
    std::fs::write(
        &path,
        serde_json::to_vec(&json!({"format": "native-compile/1", "request": compile})).unwrap(),
    )
    .unwrap();
    fsl_command::command::compile(&path).expect("a library-only model selection compiles");
    let request = json!({"models": models, "package": {"directory": "handoff"}, "call": {"function": "answer", "arguments": []}});
    let (exit, result) = run_request(directory, request.clone(), Default::default());
    assert_eq!(
        exit, 0,
        "library-only selected model is not extraneous: {result}"
    );
    let (exit, result) = run(directory, json!({"directory": "handoff"}));
    assert_eq!(exit, 20, "{result}");
    assert_eq!(result["code"], "missing_import", "{result}");
    assert_eq!(result["cause"], "missing-selection", "{result}");
    assert!(result.to_string().contains("test/counter"), "{result}");
    assert!(
        result
            .to_string()
            .contains("dependencies/0/checked-package.json"),
        "{result}"
    );
    let mut changed: Value =
        serde_json::from_slice(&std::fs::read(directory.join("counter.json")).unwrap()).unwrap();
    changed["types"][1]["fields"][0]["typeRef"] = json!("ix://test/counter/Absent");
    std::fs::write(
        directory.join("counter.json"),
        serde_json::to_vec(&changed).unwrap(),
    )
    .unwrap();
    let (exit, result) = run_request(directory, request, Default::default());
    assert_eq!(exit, 20, "{result}");
    assert!(
        result
            .to_string()
            .contains("dependencies/0/checked-package.json"),
        "{result}"
    );
}

#[test]
#[trace("FR-850-AC-13", "FR-850-AC-14", "TC-2410")]
fn foreign_artifacts_and_dependency_witnesses_and_unsafe_files_are_refused() {
    let directory = tempfile::tempdir().unwrap();
    prepare(directory.path(), false);
    let handoff = directory.path().join("handoff");
    let package = handoff.join("dependencies/1/checked-package.json");
    let original = std::fs::read(&package).unwrap();
    std::fs::copy(
        handoff.join("dependencies/0/checked-package.json"),
        &package,
    )
    .unwrap();
    let (exit, result) = run(directory.path(), json!({"directory": "handoff"}));
    assert_eq!(exit, 20, "{result}");
    assert_eq!(result["cause"], "conflicting-definition", "{result}");
    assert!(
        result
            .to_string()
            .contains("dependencies/1/checked-package.json"),
        "{result}"
    );
    std::fs::write(&package, original).unwrap();
    let witness = handoff.join("dependencies/1/identities.json");
    let original = std::fs::read(&witness).unwrap();
    std::fs::copy(handoff.join("dependencies/0/identities.json"), &witness).unwrap();
    let (exit, result) = run(directory.path(), json!({"directory": "handoff"}));
    assert_eq!(exit, 20, "{result}");
    assert!(
        result
            .to_string()
            .contains("dependencies/1/identities.json"),
        "{result}"
    );
    std::fs::write(&witness, original).unwrap();
    let equality = handoff.join("dependencies/1/equality.json");
    let original = std::fs::read(&equality).unwrap();
    std::fs::copy(handoff.join("dependencies/0/equality.json"), &equality).unwrap();
    let (exit, result) = run(directory.path(), json!({"directory": "handoff"}));
    assert_eq!(exit, 20, "{result}");
    assert_eq!(result["cause"], "refused-file", "{result}");
    assert!(
        result.to_string().contains("dependencies/1/equality.json"),
        "{result}"
    );
    std::fs::write(&equality, original).unwrap();
    std::fs::remove_file(&package).unwrap();
    std::fs::create_dir(&package).unwrap();
    let (exit, result) = run(directory.path(), json!({"directory": "handoff"}));
    assert_eq!(exit, 20, "{result}");
    assert_eq!(result["cause"], "invalid-value", "{result}");
    #[cfg(unix)]
    {
        std::fs::remove_dir(&package).unwrap();
        std::os::unix::fs::symlink(
            handoff.join("dependencies/0/checked-package.json"),
            &package,
        )
        .unwrap();
        let (exit, result) = run(directory.path(), json!({"directory": "handoff"}));
        assert_eq!(exit, 20, "{result}");
        assert_eq!(result["cause"], "invalid-value", "{result}");
    }
}
