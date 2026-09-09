// SPDX-License-Identifier: AGPL-3.0-only
//! FR-032: the concrete named model and actual file-driven workflow.

#[path = "../examples/config-version/fixtures.rs"]
mod fixtures;

use fixtures::Case;
use ix_trace_rs::trace;
use quire_contract_ir as ir;
use quire_spec_language::ByteDigest;
#[cfg(feature = "quire-extraction")]
use quire_spec_language::Span;
use serde_json::{json, Value};
use std::{collections::BTreeSet, path::Path, process::Command};

fn run(directory: &Path, request: &str) -> (i32, Value) {
    let output = Command::new(env!("CARGO_BIN_EXE_quire-spec"))
        .arg("run")
        .arg(directory.join(request))
        .current_dir(directory.parent().unwrap())
        .output()
        .unwrap();
    let bytes = if output.stdout.is_empty() {
        &output.stderr
    } else {
        &output.stdout
    };
    (
        output.status.code().unwrap(),
        serde_json::from_slice(bytes).unwrap(),
    )
}

fn has_code(value: &Value, code: &str) -> bool {
    value["code"] == code
        || value["diagnostic"]["code"] == code
        || value["diagnostics"]
            .as_array()
            .is_some_and(|values| values.iter().any(|value| value["code"] == code))
}

#[test]
#[trace("TC-110", "FR-032-AC-1")]
fn concrete_model_retains_exact_bounds_roles_and_original_source() {
    let model = fixtures::model();
    assert_eq!(
        model.environment().owner().package().as_str(),
        "example/config-version"
    );
    assert_eq!(
        model.source().source().digest(),
        ByteDigest::of(fixtures::MODEL.as_bytes())
    );
    let record = model
        .environment()
        .types()
        .iter()
        .find_map(|ty| match ty {
            ir::TypeDeclaration::Record { declaration }
                if declaration.name().as_str() == "ConfigVersion" =>
            {
                Some(declaration)
            }
            _ => None,
        })
        .unwrap();
    let version = record
        .fields()
        .iter()
        .find(|field| field.name().as_str() == "versionNumber")
        .unwrap();
    assert_eq!(
        version.value_type(),
        &ir::ValueType::integer(
            ir::IntegerType::new(
                ir::IntegerDomain::Signed,
                0,
                1000,
                ir::OverflowPolicy::Reject
            )
            .unwrap()
        )
    );
    let object = &model.roles().objects[0];
    assert_eq!(object.record.as_str(), "ConfigVersion");
    assert_eq!(object.reference.as_str(), "ConfigVersionRef");
    assert_eq!(object.universe.as_str(), "config_history");
    assert_eq!(object.identity_field.as_str(), "id");
    let parent = record
        .fields()
        .iter()
        .find(|field| field.name().as_str() == "parent")
        .unwrap();
    assert_eq!(
        parent.value_type(),
        &ir::ValueType::option(ir::ValueType::Record {
            name: ir::SymbolName::new("ConfigVersionRef").unwrap()
        })
    );
    let operation = &model.roles().operations[0];
    assert_eq!(operation.name.as_str(), "attemptUpdate");
    assert_eq!(
        operation.result.as_ref().unwrap().as_str(),
        "attempt_result"
    );
    assert_eq!(operation.frame.fields.len(), 1);
    assert_eq!(operation.frame.fields[0].0.as_str(), "ConfigVersion");
    assert_eq!(operation.frame.fields[0].1.as_str(), "versionNumber");
    assert!(operation.frame.created.is_empty() && operation.frame.deleted.is_empty());
    for (source, key, expected) in [
        (version.source(), "name", "versionNumber"),
        (parent.source(), "name", "parent"),
        (&object.source, "record", "ConfigVersion"),
        (&operation.source, "name", "attemptUpdate"),
    ] {
        let span = model.source().to_native(source).unwrap();
        let declaration: Value =
            serde_json::from_str(model.source().source().slice(span).unwrap()).unwrap();
        assert_eq!(declaration[key], expected);
    }
}

#[test]
#[trace("TC-110", "FR-032-AC-1", "FR-032-AC-2", "FR-032-AC-3")]
fn actual_native_commands_execute_named_semantics_and_distinct_case_identities() {
    let directory = tempfile::tempdir().unwrap();
    let model = fixtures::model();
    let mut snapshot_ids = BTreeSet::new();
    let mut invocation_ids = BTreeSet::new();
    let mut observed_cases = BTreeSet::new();
    for (case, exit, truth, code) in [
        (Case::Healthy, 0, Some(true), None),
        (Case::Violating, 1, Some(false), None),
        (Case::Absent, 0, Some(true), None),
        (Case::Cycle, 1, Some(false), None),
        (Case::SelfLoop, 1, Some(false), None),
        (Case::Distinct, 1, Some(false), None),
        (Case::Dangling, 1, None, Some("dangling_reference")),
        (Case::Incomplete, 3, None, Some("incomplete_population")),
        (Case::MissingModel, 1, None, Some("missing_import")),
        (Case::Exhausted, 3, None, Some("resource_exhausted")),
        (Case::Unchanged, 0, Some(true), None),
        (Case::Changed, 1, Some(false), None),
        (Case::ForbiddenParent, 1, None, Some("frame_violation")),
    ] {
        let path = directory.path().join(case.id());
        assert!(observed_cases.insert(case.id()));
        fixtures::write(&path, &model, case).unwrap();
        let (actual, result) = run(&path, "request.json");
        assert_eq!(actual, exit, "{}: {result}", case.id());
        assert_eq!(
            result.get("truth").and_then(Value::as_bool),
            truth,
            "{}: {result}",
            case.id()
        );
        if let Some(code) = code {
            assert!(has_code(&result, code), "{}: {result}", case.id());
        }
        let job: Value =
            serde_json::from_slice(&std::fs::read(path.join("request.json")).unwrap()).unwrap();
        if result["source"].is_object() {
            assert_eq!(
                result["source"]["digest"],
                job["request"]["program"]["source"]["digest"]
            );
            assert_eq!(result["selection"], job["request"]["selection"]);
            assert_eq!(result["models"][0]["digest"], model.digest().to_string());
        }
        for snapshot in job["request"]["snapshots"].as_array().unwrap() {
            assert!(snapshot_ids.insert(snapshot["reference"]["identity"].to_string()));
        }
        for invocation in job["request"]["invocations"].as_array().unwrap() {
            assert!(invocation_ids.insert(invocation["reference"]["identity"].to_string()));
        }
    }
    assert_eq!(
        observed_cases,
        fixtures::CASES.iter().map(|case| case.id()).collect()
    );
    assert_eq!(
        run(&directory.path().join(Case::Healthy.id()), "request.json").1["truth"],
        true
    );
}

#[cfg(feature = "quire-extraction")]
#[test]
#[trace("TC-110", "FR-032-AC-4")]
fn actual_markdown_and_native_cases_agree_without_reusing_source_identity() {
    let directory = tempfile::tempdir().unwrap();
    let model = fixtures::model();
    for case in fixtures::CASES {
        let path = directory.path().join(case.id());
        fixtures::write(&path, &model, case).unwrap();
        let (native_exit, native) = run(&path, "request.json");
        let (markdown_exit, markdown) = run(&path, "markdown-run.json");
        assert_eq!(markdown_exit, native_exit, "{}: {markdown}", case.id());
        assert_eq!(markdown["status"], native["status"]);
        assert_eq!(markdown.get("truth"), native.get("truth"));
        if markdown["extraction"].is_object() {
            assert_eq!(markdown["selection"], native["selection"]);
            assert_eq!(markdown["inputs"], native["inputs"]);
            assert_ne!(markdown["source"]["identity"], native["source"]["identity"]);
            assert_ne!(markdown["source"]["digest"], native["source"]["digest"]);
            assert_ne!(markdown["package"]["digest"], native["package"]["digest"]);
            assert_eq!(
                markdown["extraction"]["outcome"]["availability"]["state"],
                "available"
            );
            let original = std::fs::read_to_string(path.join("rules.md")).unwrap();
            let segment = &markdown["extraction"]["mapping"]["segments"][0];
            let span = Span {
                start: usize::try_from(segment["original"]["start"].as_u64().unwrap()).unwrap(),
                end: usize::try_from(segment["original"]["end"].as_u64().unwrap()).unwrap(),
            };
            assert_eq!(
                ByteDigest::of(&original.as_bytes()[span.start..span.end]).to_string(),
                markdown["source"]["digest"]
            );
            assert_eq!(
                ByteDigest::of(original.as_bytes()).to_string(),
                markdown["extraction"]["original"]["digest"]
            );
        }
    }
}

#[test]
#[trace("TC-110", "FR-032-AC-2", "FR-032-AC-4")]
fn exported_config_version_packages_reconstruct_the_same_native_outcomes() {
    let directory = tempfile::tempdir().unwrap();
    let model = fixtures::model();
    for case in [Case::Healthy, Case::Changed, Case::ForbiddenParent] {
        let path = directory.path().join(case.id());
        fixtures::write(&path, &model, case).unwrap();
        let expected = run(&path, "request.json");
        let export = Command::new(env!("CARGO_BIN_EXE_quire-spec"))
            .arg("compile")
            .arg(path.join("compile.json"))
            .output()
            .unwrap();
        assert!(
            export.status.success(),
            "{}",
            String::from_utf8_lossy(&export.stderr)
        );
        let digest = ByteDigest::of(&export.stdout).to_string();
        assert_eq!(expected.1["package"]["digest"], digest);
        std::fs::write(path.join("package.json"), &export.stdout).unwrap();
        let mut job: Value =
            serde_json::from_slice(&std::fs::read(path.join("request.json")).unwrap()).unwrap();
        job["request"]["package"] = json!({"file":"package.json","digest":digest});
        std::fs::write(
            path.join("package-run.json"),
            serde_json::to_vec_pretty(&job).unwrap(),
        )
        .unwrap();
        let actual = run(&path, "package-run.json");
        assert_eq!(actual.0, expected.0);
        for field in [
            "status",
            "truth",
            "package",
            "source",
            "models",
            "inputs",
            "selection",
            "diagnostics",
            "events",
        ] {
            assert_eq!(
                actual.1.get(field),
                expected.1.get(field),
                "{field}: {actual:?}"
            );
        }
    }
}
