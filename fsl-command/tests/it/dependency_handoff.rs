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
                "record A { level: Integer; } function answer using v(): Integer pure { if 1 = 1 then (let reading = A{level: 42} in reading.level) else 41 }".to_owned(),
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
    assert_eq!(result["code"], "missing_import", "{result}");
    assert_eq!(result["cause"], "missing-selection", "{result}");
    assert_eq!(
        result["details"]["path"], "/request/package/lock/dependency_selections/0",
        "{result}"
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

fn replay_boundary(
    directory: &Path,
    request: &Value,
    unlimited: &Value,
    root_bytes: u64,
    external_bytes: u64,
) -> (u64, Value) {
    // The existing decoder runs first. At the root file length it measures
    // the unchanged encoded request, independently of the later artifact charge.
    let (_, measured) = analyze(directory, request.clone(), root_bytes);
    let cause = &measured["items"][0]["record"]["cause"];
    assert_eq!(cause["code"], "stage_limit_exceeded", "{measured}");
    let message = cause["message"]
        .as_str()
        .expect("the original byte refusal is retained");
    let baseline = message
        .split_once("encoded size ")
        .and_then(|(_, rest)| rest.split_whitespace().next())
        .and_then(|value| value.parse::<u64>().ok())
        .expect("the typed byte refusal renders its attempted encoded size");
    assert!(
        baseline > root_bytes,
        "target and source metadata remain charged"
    );
    let total = baseline.checked_add(external_bytes).unwrap();
    let (exit, refused) = analyze(directory, request.clone(), baseline);
    assert_eq!(
        exit, 22,
        "the item-local replay refusal keeps FR-621's inconclusive exit: {refused}"
    );
    assert_eq!(refused["category"], "inconclusive", "{refused}");
    assert_eq!(refused["items"].as_array().unwrap().len(), 2, "{refused}");
    assert_eq!(
        refused["items"][0]["record"]["result"], "inconclusive",
        "{refused}"
    );
    assert_eq!(
        refused["items"][0]["record"]["cause"],
        json!({
            "kind": "replay-refused", "code": "stage_limit_exceeded", "cause": "input-bytes-exceeded",
            "setting": "replay.input_bytes", "bound": baseline.to_string(),
            "message": fsl_replay::BoundExceeded { bound: baseline, actual: u128::from(total) }.to_string(),
        }),
        "the encoded request fits; only unique external artifacts exceed the allowance: {refused}"
    );
    assert_eq!(
        refused["items"][1]["record"], unlimited["items"][1]["record"],
        "the proved sibling's full record is unaffected"
    );
    let (exit, exact) = analyze(directory, request.clone(), total);
    assert_eq!(
        exit, 10,
        "exact root-plus-external allowance reproduces the refutation: {exact}"
    );
    assert_eq!(
        exact["items"], unlimited["items"],
        "no root is charged a second time: {exact}"
    );
    (baseline, exact)
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
    let (source_exit, source_result) = analyze(directory, source_request.clone(), 1 << 20);
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
    let root_bytes = std::fs::metadata(directory.join("handoff/checked-package.json"))
        .unwrap()
        .len();
    let external_bytes = (0..2)
        .map(|index| {
            std::fs::metadata(
                directory.join(format!("handoff/dependencies/{index}/checked-package.json")),
            )
            .unwrap()
            .len()
        })
        .sum();
    assert!(external_bytes > 0);
    let (source_baseline, source_exact) = replay_boundary(
        directory,
        &source_request,
        &source_result,
        root_bytes,
        external_bytes,
    );
    let package_request = json!({"package": {"directory": "handoff"}, "models": models, "documents": [{"file": "start.json"}], "items": [{"claim": "LibraryBound"}, {"claim": "Bounded"}]});
    std::fs::remove_dir_all(directory.join("handoff/sources")).unwrap();
    for file in ["unit.native", "a.native", "z.native"] {
        std::fs::remove_file(directory.join(file)).unwrap();
    }
    let (exit, result) = analyze(directory, package_request.clone(), 1 << 20);
    assert_eq!(exit, source_exit, "{result}");
    assert_eq!(result["items"], source_result["items"], "{result}");
    let (package_baseline, package_exact) = replay_boundary(
        directory,
        &package_request,
        &result,
        root_bytes,
        external_bytes,
    );
    assert!(
        source_baseline > package_baseline,
        "source references and provisions keep their legitimate byte charges"
    );
    assert_eq!(source_exact["items"], package_exact["items"]);
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

fn tree(directory: &Path) -> std::collections::BTreeMap<std::path::PathBuf, Vec<u8>> {
    let mut files = std::collections::BTreeMap::new();
    let mut pending = vec![directory.to_path_buf()];
    while let Some(path) = pending.pop() {
        for entry in std::fs::read_dir(path).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                pending.push(path);
            } else {
                files.insert(
                    path.strip_prefix(directory).unwrap().to_path_buf(),
                    std::fs::read(path).unwrap(),
                );
            }
        }
    }
    files
}

#[test]
#[trace("FR-517-AC-27", "FR-517-AC-28", "TC-1312")]
fn dependency_handoff_is_deterministic_under_provision_order_and_refuses_conflicts_atomically() {
    let directory = tempfile::tempdir().unwrap();
    let mut request = prepare(directory.path(), false);
    let original = tree(&directory.path().join("handoff"));
    request["libraries"].as_array_mut().unwrap().reverse();
    request["handoff"]["directory"] = json!("reordered");
    let path = directory.path().join("compile.json");
    std::fs::write(
        &path,
        serde_json::to_vec(&json!({"format": "native-compile/1", "request": request})).unwrap(),
    )
    .unwrap();
    fsl_command::command::compile(&path).expect("reordered library provision remains valid");
    assert_eq!(tree(&directory.path().join("reordered")), original);
    request["libraries"]
        .as_array_mut()
        .unwrap()
        .push(json!({"identity": "test/a", "source": source("z.native")}));
    request["handoff"]["directory"] = json!("conflicting");
    std::fs::write(
        &path,
        serde_json::to_vec(&json!({"format": "native-compile/1", "request": request})).unwrap(),
    )
    .unwrap();
    assert!(
        fsl_command::command::compile(&path).is_err(),
        "conflicting provisions cannot choose the first artifact"
    );
    assert!(
        !directory.path().join("conflicting").exists(),
        "a refused write leaves no partial directory"
    );
    std::fs::write(
        directory.path().join("unit.native"),
        format!("{HEADER}function answer using v(): Integer pure {{ 42 }}\n"),
    )
    .unwrap();
    request["libraries"] = json!([]);
    request["handoff"]["directory"] = json!("empty");
    std::fs::write(
        &path,
        serde_json::to_vec(&json!({"format": "native-compile/1", "request": request})).unwrap(),
    )
    .unwrap();
    fsl_command::command::compile(&path)
        .expect("import-free handoff writes an explicit empty closure");
    assert_eq!(std::fs::read(directory.path().join("empty/dependencies.json")).unwrap(), b"{\n  \"format\": \"filament.checked-handoff.dependencies/1\",\n  \"dependencies\": []\n}\n");
    assert!(!directory.path().join("empty/dependencies").exists());
}

#[test]
#[trace("FR-850-AC-13", "TC-2410")]
fn closure_cycles_keep_the_first_recorded_back_edge_and_manifest_order_is_canonical() {
    let directory = tempfile::tempdir().unwrap();
    prepare(directory.path(), false);
    let handoff = directory.path().join("handoff");
    let manifest_file = handoff.join("dependencies.json");
    let original = std::fs::read(&manifest_file).unwrap();
    let mut manifest: Value = serde_json::from_slice(&original).unwrap();
    manifest["dependencies"].as_array_mut().unwrap().reverse();
    std::fs::write(&manifest_file, serde_json::to_vec(&manifest).unwrap()).unwrap();
    let (exit, result) = run(directory.path(), json!({"directory": "handoff"}));
    assert_eq!(exit, 20, "{result}");
    assert_eq!(result["cause"], "invalid-value", "{result}");
    std::fs::write(&manifest_file, original).unwrap();
    let package = handoff.join("dependencies/0/checked-package.json");
    let package_bytes = std::fs::read(&package).unwrap();
    let mut value: Value = serde_json::from_slice(&package_bytes).unwrap();
    value["lock"]["dependency_selections"] = json!([{"identity": "test/z"}]);
    std::fs::write(&package, serde_json::to_vec(&value).unwrap()).unwrap();
    let (exit, result) = run(directory.path(), json!({"directory": "handoff"}));
    assert_eq!(exit, 20, "{result}");
    assert_eq!(result["code"], "invalid_package", "{result}");
    assert_eq!(result["cause"], "definition-cycle", "{result}");
    assert_eq!(
        result["details"]["path"], "/lock/dependency_selections/0",
        "{result}"
    );
    assert!(
        result
            .to_string()
            .contains("dependencies/0/checked-package.json"),
        "{result}"
    );
    assert!(
        result.to_string().contains("test/a") && result.to_string().contains("test/z"),
        "{result}"
    );
    std::fs::write(&package, package_bytes).unwrap();
    let root = handoff.join("checked-package.json");
    let mut value: Value = serde_json::from_slice(&std::fs::read(&root).unwrap()).unwrap();
    value["lock"]["dependency_selections"] = json!([{"identity": value["name"]}]);
    std::fs::write(root, serde_json::to_vec(&value).unwrap()).unwrap();
    let (exit, result) = run(directory.path(), json!({"directory": "handoff"}));
    assert_eq!(exit, 20, "{result}");
    assert_eq!(result["cause"], "definition-cycle", "{result}");
    assert_eq!(
        result["details"]["path"], "/lock/dependency_selections/0",
        "{result}"
    );
    assert!(
        result.to_string().contains("checked-package.json"),
        "{result}"
    );
}

#[test]
#[trace("FR-850-AC-15", "TC-2410")]
fn closure_expansion_shares_the_sixty_four_selected_file_limit_with_runtime_inputs() {
    let directory = tempfile::tempdir().unwrap();
    let mut imports = String::new();
    let mut libraries = Vec::new();
    for index in 0..20 {
        let identity = format!("test/lib{index:02}");
        let file = format!("lib{index:02}.native");
        imports.push_str(&format!("import \"{identity}\" as l{index:02};\n"));
        std::fs::write(
            directory.path().join(&file),
            format!("{HEADER}function answer using v(): Integer pure {{ {index} }}\n"),
        )
        .unwrap();
        libraries.push(json!({"identity": identity, "source": source(&file)}));
    }
    std::fs::write(
        directory.path().join("unit.native"),
        format!("{HEADER}{imports}function answer using v(): Integer pure {{ l00::answer() }}\n"),
    )
    .unwrap();
    let request = json!({"program": {"source": source("unit.native")}, "models": [], "libraries": libraries, "handoff": {"directory": "handoff", "package_name": "root", "native_join": {"work_units": 1_u64 << 40, "storage": 1_u64 << 26}}});
    let path = directory.path().join("compile.json");
    std::fs::write(
        &path,
        serde_json::to_vec(&json!({"format": "native-compile/1", "request": request})).unwrap(),
    )
    .unwrap();
    fsl_command::command::compile(&path).expect("twenty unique libraries fit the source intake");
    let artifacts = tree(&directory.path().join("handoff"));
    assert_eq!(
        artifacts
            .keys()
            .filter(|path| !path.starts_with("sources")
                && !matches!(path.to_str(), Some("selection.json" | "inventory.json")))
            .count(),
        64
    );
    let (exit, result) = run(directory.path(), json!({"directory": "handoff"}));
    assert_eq!(
        exit, 0,
        "exactly 64 selected artifact files complete: {result}"
    );
    // A runtime selection consumes the same shared file allowance. Its
    // content is deliberately absent: the file-count refusal precedes opening it.
    let request = json!({"package": {"directory": "handoff"}, "models": [], "documents": [{"file": "not-opened.json"}], "items": []});
    let (exit, result) = analyze(directory.path(), request, 1 << 20);
    assert_eq!(exit, 22, "a 65th selected file stops intake: {result}");
    assert_eq!(result["category"], "incomplete", "{result}");
    assert_eq!(
        result["diagnostics"][0]["code"], "resource_exhausted",
        "{result}"
    );
    assert!(
        result.to_string().contains("selected files limit 64"),
        "{result}"
    );
}

#[test]
#[trace("FR-850-AC-13", "TC-2410")]
fn every_raw_manifest_member_refusal_keeps_its_exact_pointer_before_analysis() {
    let directory = tempfile::tempdir().unwrap();
    prepare(directory.path(), true);
    let manifest = directory.path().join("handoff/dependencies.json");
    let original = std::fs::read(&manifest).unwrap();
    let format = "\"format\":\"filament.checked-handoff.dependencies/1\"";
    let cases = [
            ("not json".to_owned(), ""),
            ("{\"dependencies\":[]}".to_owned(), "/format"),
            (format!("{{{format},{format},\"dependencies\":[]}}"), "/format"),
            (format!("{{{format}}}"), "/dependencies"),
            (format!("{{{format},\"dependencies\":[],\"dependencies\":[]}}"), "/dependencies"),
            (format!("{{{format},\"dependencies\":[],\"extra\":0}}"), "/extra"),
            ("{\"format\":7,\"dependencies\":[]}".to_owned(), "/format"),
            (format!("{{{format},\"dependencies\":{{}}}}"), "/dependencies"),
            (format!("{{{format},\"dependencies\":[null]}}"), "/dependencies/0"),
            (format!("{{{format},\"dependencies\":[{{\"directory\":\"dependencies/0\"}}]}}"), "/dependencies/0/identity"),
            (format!("{{{format},\"dependencies\":[{{\"identity\":\"test/a\"}}]}}"), "/dependencies/0/directory"),
            (format!("{{{format},\"dependencies\":[{{\"identity\":\"test/a\",\"identity\":\"test/a\",\"directory\":\"dependencies/0\"}}]}}"), "/dependencies/0/identity"),
            (format!("{{{format},\"dependencies\":[{{\"identity\":\"test/a\",\"directory\":\"dependencies/0\",\"directory\":\"dependencies/0\"}}]}}"), "/dependencies/0/directory"),
            (format!("{{{format},\"dependencies\":[{{\"identity\":\"test/a\",\"directory\":\"dependencies/0\",\"extra\":0}}]}}"), "/dependencies/0/extra"),
            (format!("{{{format},\"dependencies\":[{{\"identity\":7,\"directory\":\"dependencies/0\"}}]}}"), "/dependencies/0/identity"),
            (format!("{{{format},\"dependencies\":[{{\"identity\":\"test/a\",\"directory\":7}}]}}"), "/dependencies/0/directory"),
        ];
    let unknown_format = String::from_utf8(original.clone()).unwrap().replace(
        "filament.checked-handoff.dependencies/1",
        "filament.checked-handoff.dependencies/2",
    );
    for (bytes, pointer, cause) in cases
        .into_iter()
        .map(|(bytes, pointer)| (bytes, pointer, "malformed-file"))
        .chain(std::iter::once((
            unknown_format,
            "/format",
            "unknown-format",
        )))
    {
        std::fs::write(&manifest, &bytes).unwrap();
        let request = json!({"package": {"directory": "handoff"}, "models": [], "items": [{"claim": "Missing"}]});
        let (exit, outcome) = analyze(directory.path(), request, 1 << 20);
        assert_eq!(exit, 20, "{bytes}: {outcome}");
        assert_eq!(outcome["category"], "refusal", "{outcome}");
        assert_eq!(outcome["last_stage"], Value::Null, "{outcome}");
        assert_eq!(outcome["items"], json!([]), "{outcome}");
        let diagnostic = &outcome["diagnostics"][0];
        assert_eq!(diagnostic["code"], "invalid_package", "{bytes}: {outcome}");
        assert_eq!(diagnostic["cause"], cause, "{bytes}: {outcome}");
        assert_eq!(diagnostic["details"]["path"], pointer, "{bytes}: {outcome}");
        assert!(
            diagnostic["message"]
                .as_str()
                .unwrap()
                .contains("dependencies.json"),
            "{outcome}"
        );
    }
    std::fs::write(manifest, original).unwrap();
    let (exit, outcome) = run(directory.path(), json!({"directory": "handoff"}));
    assert_eq!(
        exit, 0,
        "the unchanged positive control completes: {outcome}"
    );
}

#[test]
#[trace("FR-517-AC-2", "FR-850-AC-12", "TC-1312", "TC-2410")]
fn source_protocol_projection_uses_each_imported_librarys_retained_witnesses() {
    use fsl_command::{checked_handoff, emit_checked_v2};
    let directory = tempfile::tempdir().unwrap();
    let recipe = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("examples/checked-handoff");
    let request = checked_handoff::CheckedHandoffRequest {
        emit: emit_checked_v2::Request {
            source: recipe.join("unit-event-position.native"),
            package_name: "recipe".to_owned(),
            models: vec![recipe.join("model.semantic-ir.json")],
            libraries: vec![(
                "agent-ix/checked-handoff-library".to_owned(),
                recipe.join("library.native"),
            )],
            execution_points: None,
            temporal_selections: ["Full", "Partial", "Due", "DueLate"]
                .into_iter()
                .map(|clause| emit_checked_v2::ClockSelection {
                    clause: clause.to_owned(),
                    clock: filament_contract_model::ClockConfiguration::EventPosition {
                        sequence_authority: "events-authority".into(),
                    },
                })
                .collect(),
            out: None,
        },
        corpus: checked_handoff::Corpus::None,
        native_join: fsl_package::NativeJoinLimits::new(1 << 40, 1 << 28),
    };
    let handoff = directory.path().join("handoff");
    checked_handoff::write(&request, &handoff)
        .expect("the real recipe writes its dependency artifacts");
    let compiled = checked_handoff::compile(&handoff)
        .expect("the explicit source entry recompiles the same recipe");
    let emission = compiled
        .checked()
        .remembered_emission()
        .expect("source acquisition retains its final emission");
    let read = fsl_package::read_evaluation_emission(
        compiled.checked(),
        emission,
        fsl_semantics::library::LibraryName::of_package(compiled.admitted()),
        compiled.packages(),
        fsl_package::V2ReadLimits::bounded(1 << 28),
        compiled.emit_limits(),
        fsl_package::SourceWitnessSupplier::new(emission),
    )
    .expect("the same admitted closure supplies the source evaluation view");
    let view = read
        .evaluation_view()
        .expect("imported predicates in effect and finish slots project through their own package");
    assert!(view.is_reader_backed());
    assert!(view.is_for_package(compiled.checked()));
}
