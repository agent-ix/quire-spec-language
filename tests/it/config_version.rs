// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-032: the concrete named model and actual file-driven workflow.

use crate::support::config_version as fixtures;

use fixtures::Case;
use ix_trace_rs::trace;
use quire_contract_ir as ir;
use quire_spec_language::ByteDigest;
#[cfg(feature = "quire-extraction")]
use quire_spec_language::Span;
use serde_json::{json, Value};
use std::{collections::BTreeSet, path::Path, process::Command};

struct Run {
    exit: i32,
    value: Value,
    stdout: bool,
}

fn run(directory: &Path, request: &str) -> Run {
    let output = Command::new(env!("CARGO_BIN_EXE_quire-spec"))
        .arg("run")
        .arg(directory.join(request))
        .current_dir(directory.parent().unwrap())
        .output()
        .unwrap();
    assert_ne!(
        output.stdout.is_empty(),
        output.stderr.is_empty(),
        "exactly one result stream"
    );
    let stdout = !output.stdout.is_empty();
    let bytes = if stdout {
        &output.stdout
    } else {
        &output.stderr
    };
    Run {
        exit: output.status.code().unwrap(),
        value: serde_json::from_slice(bytes).unwrap(),
        stdout,
    }
}

// Independent of the generator's CaseSpec: adding a catalog case requires an
// explicit expected semantic result here, enforced by this exhaustive match.
enum Expected {
    Completed(bool),
    Validation {
        code: &'static str,
        incomplete: bool,
    },
    MissingModel,
    Exhausted,
}

fn expected(case: Case) -> Expected {
    match case {
        Case::Healthy | Case::Absent | Case::Unchanged => Expected::Completed(true),
        Case::Violating | Case::Cycle | Case::SelfLoop | Case::Distinct | Case::Changed => {
            Expected::Completed(false)
        }
        Case::Dangling => Expected::Validation {
            code: "dangling_reference",
            incomplete: false,
        },
        Case::Incomplete => Expected::Validation {
            code: "incomplete_population",
            incomplete: true,
        },
        Case::ForbiddenParent => Expected::Validation {
            code: "frame_violation",
            incomplete: false,
        },
        Case::MissingModel => Expected::MissingModel,
        Case::Exhausted => Expected::Exhausted,
    }
}

fn assert_outcome(case: Case, actual: &Run, extracted: bool) {
    let value = &actual.value;
    match expected(case) {
        Expected::Completed(truth) => {
            assert_eq!(
                actual.exit,
                if truth { 0 } else { 10 },
                "{}: {value}",
                case.id()
            );
            assert!(actual.stdout);
            assert_eq!(value["status"], "completed");
            assert_eq!(value["stage"], "evaluate");
            assert_eq!(value["truth"], truth);
            assert!(value.get("code").is_none());
            assert!(value.get("diagnostic").is_none());
        }
        Expected::Validation { code, incomplete } => {
            assert_eq!(actual.exit, if incomplete { 22 } else { 20 });
            assert!(actual.stdout);
            assert_eq!(
                value["status"],
                if incomplete { "incomplete" } else { "refused" }
            );
            assert_eq!(value["stage"], "validate");
            assert!(value.get("truth").is_none());
            assert!(value.get("code").is_none());
            assert!(value.get("diagnostic").is_none());
            let codes: BTreeSet<_> = value["diagnostics"]
                .as_array()
                .unwrap()
                .iter()
                .map(|value| value["code"].as_str().unwrap())
                .collect();
            // The same refusal can locate several affected input paths.
            assert_eq!(codes, BTreeSet::from([code]), "{}: {value}", case.id());
        }
        Expected::MissingModel => {
            assert_eq!(actual.exit, 20);
            assert!(!actual.stdout);
            assert_eq!(value["status"], "refused");
            assert_eq!(value["stage"], if extracted { "quire" } else { "link" });
            assert_eq!(value["code"], "missing_import");
            assert!(value.get("truth").is_none());
            assert!(value.get("source").is_none());
            assert!(value.get("extraction").is_none());
        }
        Expected::Exhausted => {
            assert_eq!(actual.exit, 22);
            assert!(actual.stdout);
            assert_eq!(value["status"], "incomplete");
            assert_eq!(value["stage"], "evaluate");
            assert_eq!(value["diagnostic"]["code"], "resource_exhausted");
            assert!(value.get("truth").is_none());
            assert!(value.get("code").is_none());
            assert!(value.get("diagnostics").is_none());
        }
    }
}

#[test]
#[trace("TC-110", "FR-032-AC-1")]
fn concrete_model_retains_exact_bounds_roles_and_original_source() {
    let model = fixtures::model().unwrap();
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
#[trace(
    "TC-110",
    "TC-131",
    "FR-032-AC-1",
    "FR-032-AC-2",
    "FR-032-AC-3",
    "FR-047-AC-8"
)]
fn actual_native_commands_execute_named_semantics_and_distinct_case_identities() {
    let directory = tempfile::tempdir().unwrap();
    let model = fixtures::model().unwrap();
    let mut snapshot_ids = BTreeSet::new();
    let mut invocation_ids = BTreeSet::new();
    let mut observed_cases = BTreeSet::new();
    for &case in fixtures::CASES {
        let path = directory.path().join(case.id());
        assert!(observed_cases.insert(case.id()));
        fixtures::write(&path, &model, case).unwrap();
        let actual = run(&path, "request.json");
        assert_outcome(case, &actual, false);
        let result = &actual.value;
        let job: Value =
            serde_json::from_slice(&std::fs::read(path.join("request.json")).unwrap()).unwrap();
        if matches!(case, Case::MissingModel) {
            assert_eq!(
                result["details"]["source"]["identity"],
                job["request"]["program"]["source"]["identity"]
            );
        } else {
            assert!(result["source"].is_object(), "{}: {result}", case.id());
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
        run(&directory.path().join(Case::Healthy.id()), "request.json").value["truth"],
        true
    );
}

#[cfg(feature = "quire-extraction")]
#[test]
#[trace("TC-110", "FR-032-AC-4")]
fn actual_markdown_and_native_cases_agree_without_reusing_source_identity() {
    let directory = tempfile::tempdir().unwrap();
    let model = fixtures::model().unwrap();
    for &case in fixtures::CASES {
        let path = directory.path().join(case.id());
        fixtures::write(&path, &model, case).unwrap();
        let native_run = run(&path, "request.json");
        let markdown_run = run(&path, "markdown-run.json");
        assert_outcome(case, &native_run, false);
        assert_outcome(case, &markdown_run, true);
        let native = &native_run.value;
        let markdown = &markdown_run.value;
        assert_eq!(
            markdown_run.exit,
            native_run.exit,
            "{}: {markdown}",
            case.id()
        );
        assert_eq!(markdown["status"], native["status"]);
        assert_eq!(markdown.get("truth"), native.get("truth"));
        if matches!(case, Case::MissingModel) {
            let job: Value =
                serde_json::from_slice(&std::fs::read(path.join("markdown-run.json")).unwrap())
                    .unwrap();
            assert_eq!(
                markdown["details"]["original"]["identity"],
                job["request"]["program"]["source"]["identity"]
            );
            assert_eq!(
                markdown["details"]["original"]["digest"],
                job["request"]["program"]["source"]["digest"]
            );
            assert_eq!(
                markdown["details"]["cause"]["diagnostic"]["code"],
                "missing_import"
            );
            assert!(!markdown["details"]["cause"]["original_spans"]
                .as_array()
                .unwrap()
                .is_empty());
        } else {
            assert!(
                markdown["extraction"].is_object(),
                "{}: {markdown}",
                case.id()
            );
            assert!(markdown["source"].is_object());
            assert!(native["source"].is_object());
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
#[trace("TC-110", "TC-131", "FR-032-AC-2", "FR-032-AC-4", "FR-047-AC-8")]
fn exported_config_version_packages_reconstruct_the_same_native_outcomes() {
    let directory = tempfile::tempdir().unwrap();
    let model = fixtures::model().unwrap();
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
        assert_eq!(expected.value["package"]["digest"], digest);
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
        assert_eq!(actual.exit, expected.exit);
        assert_outcome(case, &actual, false);
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
                actual.value.get(field),
                expected.value.get(field),
                "{field}: {}",
                actual.value
            );
        }
    }
}

#[test]
#[trace("TC-110", "FR-032-AC-1", "FR-032-AC-3")]
fn fixture_output_is_repeatable_and_filesystem_failure_is_returned() {
    let directory = tempfile::tempdir().unwrap();
    let model = fixtures::model().unwrap();
    let first = directory.path().join("first");
    let second = directory.path().join("second");
    fixtures::write(&first, &model, Case::Healthy).unwrap();
    fixtures::write(&second, &model, Case::Healthy).unwrap();
    for entry in std::fs::read_dir(&first).unwrap() {
        let entry = entry.unwrap();
        assert_eq!(
            std::fs::read(entry.path()).unwrap(),
            std::fs::read(second.join(entry.file_name())).unwrap()
        );
    }
    let file = directory.path().join("occupied");
    std::fs::write(&file, b"existing file").unwrap();
    assert!(fixtures::write(&file, &model, Case::Healthy).is_err());
    assert_eq!(std::fs::read(file).unwrap(), b"existing file");
}
