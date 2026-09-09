// SPDX-License-Identifier: AGPL-3.0-only
//! FR-029/FR-033: actual source-only projection export and backend boundaries.

#[path = "support/standalone_setup.rs"]
mod fixtures;

use ix_trace_rs::trace;
use quire_contract_codegen as codegen;
use quire_contract_ir as ir;
use quire_contract_ir_backend as backend_ir;
use quire_spec_language::{
    lowering::{lower, LoweringLimits},
    package::{NativePackage, PackageLimits},
    syntax::ClauseKind,
    ByteDigest,
};
use serde_json::{json, Value};
use std::{
    path::Path,
    process::{Command, Output},
};

fn invoke(directory: &Path, command: &str, file: &str) -> Output {
    Command::new(env!("CARGO_BIN_EXE_quire-spec"))
        .arg(command)
        .arg(directory.join(file))
        .current_dir(directory.parent().unwrap())
        .output()
        .unwrap()
}

fn save(directory: &Path, job: &Value) {
    std::fs::write(
        directory.join("compile.json"),
        serde_json::to_vec(job).unwrap(),
    )
    .unwrap();
}

fn job(directory: &Path) -> Value {
    serde_json::from_slice(&std::fs::read(directory.join("compile.json")).unwrap()).unwrap()
}

#[test]
#[trace("TC-111", "FR-033-AC-5")]
fn fixture_filesystem_failures_propagate_and_fresh_generation_succeeds() {
    let directory = tempfile::tempdir().unwrap();
    let file = directory.path().join("file");
    std::fs::write(&file, "not a directory").unwrap();
    for markdown in [false, true] {
        let invalid = file.join("child");
        let error = if markdown {
            fixtures::write_extracted(&invalid, fixtures::Case::Integer(1), true).unwrap_err()
        } else {
            fixtures::write(&invalid, fixtures::Case::Integer(1)).unwrap_err()
        };
        assert_eq!(error.kind(), std::io::ErrorKind::NotADirectory);

        let later = directory
            .path()
            .join(if markdown { "markdown" } else { "native" });
        std::fs::create_dir_all(&later).unwrap();
        let collision = later.join(if markdown { "rules.md" } else { "model.json" });
        std::fs::create_dir(&collision).unwrap();
        let error = if markdown {
            fixtures::write_extracted(&later, fixtures::Case::Integer(1), true).unwrap_err()
        } else {
            fixtures::write(&later, fixtures::Case::Integer(1)).unwrap_err()
        };
        assert_eq!(error.kind(), std::io::ErrorKind::IsADirectory);
        assert!(collision.is_dir());
    }
    let valid = directory.path().join("retry");
    fixtures::write(&valid, fixtures::Case::Integer(1)).unwrap();
    let output = invoke(&valid, "run", "request.json");
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stdout).unwrap()["truth"],
        true
    );
}

#[test]
#[trace("TC-111", "FR-033-AC-4")]
fn runnable_integer_examples_keep_runtime_truth_separate_from_projection() {
    let directory = tempfile::tempdir().unwrap();
    let mut projections = Vec::new();
    for (name, amount, truth) in [("healthy", 1, true), ("violating", 10, false)] {
        let path = directory.path().join(name);
        fixtures::write(&path, fixtures::Case::Integer(amount)).unwrap();
        let actual = invoke(&path, "run", "request.json");
        assert_eq!(actual.status.code(), Some(if truth { 0 } else { 1 }));
        assert_eq!(
            serde_json::from_slice::<Value>(&actual.stdout).unwrap()["truth"],
            truth
        );
        let output = Command::new(env!("CARGO_BIN_EXE_quire-spec"))
            .arg("lower")
            .arg(path.join("compile.json"))
            .args(["--target", "integer-ir/v1"])
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let bound = ir::BoundPackage::from_json_bytes(&output.stdout).unwrap();
        assert_eq!(bound.clauses().len(), 2);
        projections.push(output.stdout);
    }
    assert_eq!(projections[0], projections[1]);
    let path = directory.path().join("boolean");
    fixtures::write(&path, fixtures::Case::Boolean(true)).unwrap();
    let default = invoke(&path, "lower", "compile.json");
    let explicit = Command::new(env!("CARGO_BIN_EXE_quire-spec"))
        .arg("lower")
        .arg(path.join("compile.json"))
        .args(["--target", "boolean-oracle/v1"])
        .output()
        .unwrap();
    assert!(default.status.success() && explicit.status.success());
    assert_eq!(default.stdout, explicit.stdout);
}

#[test]
#[trace("TC-107", "FR-029-AC-1", "FR-029-AC-3")]
fn exported_boolean_bytes_reach_both_ir_readers_and_the_complete_backend_population() {
    let directory = tempfile::tempdir().unwrap();
    fixtures::write(directory.path(), fixtures::Case::Boolean(true)).unwrap();
    let run = invoke(directory.path(), "run", "request.json");
    assert_eq!(run.status.code(), Some(0));
    assert_eq!(
        serde_json::from_slice::<Value>(&run.stdout).unwrap()["truth"],
        true
    );
    std::fs::remove_file(directory.path().join("snapshot-0.json")).unwrap();
    std::fs::remove_file(directory.path().join("request.json")).unwrap();

    let output = invoke(directory.path(), "lower", "compile.json");
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    assert!(!output.stdout.ends_with(b"\n"));

    let model_text = std::fs::read_to_string(directory.path().join("model.json")).unwrap();
    let models = [
        fixtures::runtime::native_rule_model::from_text(&model_text, "model.json", "1")
            .unwrap()
            .model(),
    ];
    let checked =
        fixtures::runtime::checked_kind(&models, "true implies flag", ClauseKind::Invariant);
    let native = NativePackage::new(checked, PackageLimits::default()).unwrap();
    let expected = lower(&native, LoweringLimits::default()).unwrap();
    assert_eq!(output.stdout, expected.bytes());
    let current = ir::BoundPackage::from_json_bytes(&output.stdout).unwrap();
    let consumer = backend_ir::BoundPackage::from_json_bytes(&output.stdout).unwrap();
    assert_eq!(current.digest().to_string(), consumer.digest().to_string());
    assert_eq!(current.digest(), expected.bound().digest());

    let generated = codegen::generate_bound_oracles(
        &consumer,
        codegen::AttestationContext {
            // Synthetic generator context only; this test makes no attestation claim.
            record_digest: &"0".repeat(64),
            candidate_revision: &"0".repeat(40),
        },
    )
    .unwrap();
    let codegen::BoundOracleGeneration::Generated(population) = generated else {
        panic!("expected complete generated population");
    };
    let names: Vec<_> = population
        .clauses()
        .iter()
        .map(|clause| clause.identity().clause().as_str())
        .collect();
    assert_eq!(names, ["other_rule", "population_rule"]);
    for clause in population.clauses() {
        let source = syn::parse_file(&clause.bundle().rust.contents).unwrap();
        assert_eq!(source.items.iter().filter(|item| matches!(item, syn::Item::Fn(function) if matches!(function.vis, syn::Visibility::Public(_)))).count(), 1);
        let regions: Vec<codegen::SourceRegion> =
            serde_json::from_str(&clause.bundle().source_map.contents).unwrap();
        assert!(!regions.is_empty());
        assert!(regions
            .iter()
            .all(|region| region.package_id == "example/runtime-rules"
                && region.requirement_id == "PopulationRule"
                && region.requirement_revision == 7
                && region.clause_id == clause.identity().clause().as_str()));
    }
    let compiled = invoke(directory.path(), "compile", "compile.json");
    assert_eq!(compiled.status.code(), Some(0));
    assert_ne!(compiled.stdout, output.stdout);
    assert_eq!(ByteDigest::of(&compiled.stdout), native.digest());
}

#[test]
#[trace("TC-107", "FR-029-AC-2")]
fn later_unsupported_clause_preserves_native_authority_and_never_exports_a_prefix() {
    let directory = tempfile::tempdir().unwrap();
    fixtures::write(directory.path(), fixtures::Case::Boolean(true)).unwrap();
    let original_job = job(directory.path());
    let program_path = directory.path().join("program.native");
    let original = std::fs::read_to_string(&program_path).unwrap();
    let changed = original.replace(
        "Other on M::Node at current { true }",
        "Other on M::Node at current { self.n = 2 }",
    );
    assert_ne!(changed, original);
    std::fs::write(&program_path, &changed).unwrap();
    let mut changed_job = original_job.clone();
    changed_job["request"]["program"]["source"]["digest"] =
        json!(ByteDigest::of(changed.as_bytes()).to_string());
    save(directory.path(), &changed_job);
    let native = invoke(directory.path(), "compile", "compile.json");
    assert_eq!(native.status.code(), Some(0));

    let output = invoke(directory.path(), "lower", "compile.json");
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    let error: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(error["stage"], "lower");
    assert_eq!(error["code"], "unsupported_projection");
    assert_eq!(error["details"]["clause"]["clause"], "other_rule");
    assert_eq!(
        error["details"]["package"]["digest"],
        ByteDigest::of(&native.stdout).to_string()
    );
    assert_eq!(
        error["details"]["source"]["digest"],
        changed_job["request"]["program"]["source"]["digest"]
    );
    assert_eq!(error["details"]["source"]["path"], "program.native");
    let span = &error["details"]["span"];
    let start = usize::try_from(span["start"]["byte"].as_u64().unwrap()).unwrap();
    let end = usize::try_from(span["end"]["byte"].as_u64().unwrap()).unwrap();
    assert_eq!(&changed[start..end], "self.n = 2");
    assert_eq!(
        error["request_digest"],
        ByteDigest::of(&std::fs::read(directory.path().join("compile.json")).unwrap()).to_string()
    );
    assert!(error.get("truth").is_none());

    std::fs::write(&program_path, original).unwrap();
    save(directory.path(), &original_job);
    assert_eq!(
        invoke(directory.path(), "lower", "compile.json")
            .status
            .code(),
        Some(0)
    );
}

#[test]
#[trace("TC-107", "FR-029-AC-3")]
fn projection_export_reuses_source_request_refusals_and_intake_limits() {
    let directory = tempfile::tempdir().unwrap();
    fixtures::write(directory.path(), fixtures::Case::Boolean(false)).unwrap();
    let original = job(directory.path());
    for (mutation, exit, code) in [
        ("format", 1, "unknown_wire"),
        ("runtime", 2, "invalid-request"),
        ("stale", 1, "source_digest_mismatch"),
        ("files", 3, "resource_exhausted"),
    ] {
        let mut changed = original.clone();
        match mutation {
            "format" => changed["format"] = json!("native-run/1"),
            "runtime" => changed["request"]["snapshots"] = json!([]),
            "stale" => {
                changed["request"]["program"]["source"]["digest"] =
                    json!(ByteDigest::of(b"foreign").to_string())
            }
            "files" => {
                changed["request"]["models"] =
                    json!(vec![original["request"]["models"][0].clone(); 64])
            }
            _ => unreachable!(),
        }
        save(directory.path(), &changed);
        let output = invoke(directory.path(), "lower", "compile.json");
        assert_eq!(output.status.code(), Some(exit));
        assert!(output.stdout.is_empty());
        let error: Value = serde_json::from_slice(&output.stderr).unwrap();
        assert_eq!(error["code"], code);
        assert_eq!(
            error["request_digest"],
            ByteDigest::of(&std::fs::read(directory.path().join("compile.json")).unwrap())
                .to_string()
        );
    }
}
