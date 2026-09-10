// SPDX-License-Identifier: AGPL-3.0-only
//! FR-027: actual CLI artifact bytes, verified consumer intake and stage failures.

#[path = "support/standalone_setup.rs"]
mod fixtures;
use fixtures::runtime;

use ix_trace_rs::trace;
use quire_spec_language::{
    package::{NativePackage, NativePackageRef, PackageReadLimits, PackageSupport},
    ByteDigest,
};
use serde_json::{json, Value};
use std::{
    path::Path,
    process::{Command, Output},
};

fn compile(directory: &Path) -> Output {
    Command::new(env!("CARGO_BIN_EXE_quire-spec"))
        .arg("compile")
        .arg(directory.join("compile.json"))
        .current_dir(directory.parent().unwrap())
        .output()
        .unwrap()
}

#[test]
#[trace("TC-105", "FR-027-AC-1", "FR-027-AC-2")]
fn source_only_compilation_emits_exact_bytes_accepted_by_the_existing_reader() {
    for (case, expression, kind) in [
        (
            fixtures::Case::Aggregate(2),
            "true implies forall(item in self.items: item < self.n)",
            quire_spec_language::syntax::ClauseKind::Invariant,
        ),
        (
            fixtures::Case::Operation {
                violate_frame: false,
            },
            "pre(self.n) = 1 and self.n = 2 and result",
            quire_spec_language::syntax::ClauseKind::Postcondition,
        ),
    ] {
        let generated = tempfile::tempdir().unwrap();
        let (_, digest) = fixtures::write(generated.path(), case);
        let request_bytes = std::fs::read(generated.path().join("compile.json")).unwrap();
        let job: Value = serde_json::from_slice(&request_bytes).unwrap();
        let directory = tempfile::tempdir().unwrap();
        std::fs::write(directory.path().join("compile.json"), request_bytes).unwrap();
        let selected_sources = job["request"]["models"]
            .as_array()
            .unwrap()
            .iter()
            .map(|model| model["source"]["file"].as_str().unwrap())
            .chain(std::iter::once(
                job["request"]["program"]["source"]["file"]
                    .as_str()
                    .unwrap(),
            ))
            .collect::<std::collections::BTreeSet<_>>();
        for name in &selected_sources {
            std::fs::copy(generated.path().join(name), directory.path().join(name)).unwrap();
        }
        assert_eq!(
            std::fs::read_dir(directory.path()).unwrap().count(),
            selected_sources.len() + 1
        );
        let output = compile(directory.path());
        assert_eq!(
            output.status.code(),
            Some(0),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(output.stderr.is_empty());
        assert_eq!(ByteDigest::of(&output.stdout).to_string(), digest);
        assert!(!output.stdout.ends_with(b"\n"));
        let models = [runtime::native_rule_model::parts().model()];
        let checked = runtime::checked_kind(&models, expression, kind);
        let read = NativePackage::read_verified(
            &output.stdout,
            NativePackageRef::new(ByteDigest::of(&output.stdout)),
            checked.bindings().clone(),
            &models,
            &PackageSupport::default(),
            PackageReadLimits::default(),
        )
        .unwrap();
        assert_eq!(read.bytes(), output.stdout);
        assert_eq!(read.checked().clauses().len(), 2);
        assert_eq!(read.digest().to_string(), digest);
    }
}

#[test]
#[trace("TC-105", "FR-027-AC-2", "FR-027-AC-3")]
fn incompatible_request_and_static_failures_emit_no_package_bytes() {
    enum Change {
        Format,
        Runtime,
        Stale,
        Syntax,
    }
    for change in [
        Change::Format,
        Change::Runtime,
        Change::Stale,
        Change::Syntax,
    ] {
        let directory = tempfile::tempdir().unwrap();
        fixtures::write(directory.path(), fixtures::Case::Aggregate(2));
        let original = std::fs::read(directory.path().join("compile.json")).unwrap();
        let mut changed: Value = serde_json::from_slice(&original).unwrap();
        let (expected, code) = match change {
            Change::Format => {
                changed["format"] = json!("native-run/1");
                (1, "unknown_wire")
            }
            Change::Runtime => {
                changed["request"]["snapshots"] = json!([]);
                (2, "invalid-request")
            }
            Change::Stale => {
                changed["request"]["program"]["source"]["digest"] =
                    json!(ByteDigest::of(b"foreign").to_string());
                (1, "source_digest_mismatch")
            }
            Change::Syntax => {
                let broken = b"language ?";
                std::fs::write(directory.path().join("program.native"), broken).unwrap();
                changed["request"]["program"]["source"]["digest"] =
                    json!(ByteDigest::of(broken).to_string());
                (1, "invalid_syntax")
            }
        };
        std::fs::write(
            directory.path().join("compile.json"),
            serde_json::to_vec(&changed).unwrap(),
        )
        .unwrap();
        let output = compile(directory.path());
        assert_eq!(output.status.code(), Some(expected));
        assert!(output.stdout.is_empty());
        let failure: Value = serde_json::from_slice(&output.stderr).unwrap();
        assert_eq!(failure["code"], code, "{failure}");
        assert_eq!(
            failure["request_digest"],
            ByteDigest::of(&std::fs::read(directory.path().join("compile.json")).unwrap())
                .to_string()
        );
    }
}

#[test]
#[trace("TC-105", "FR-027-AC-3")]
fn compile_arity_is_checked_before_opening_any_request() {
    for arguments in [
        vec!["compile"],
        vec!["compile", "missing.json", "extra"],
        vec!["compile", "missing.json", "--target", "integer-ir/v1"],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_quire-spec"))
            .args(arguments)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
        assert_eq!(
            std::str::from_utf8(&output.stderr).unwrap(),
            "usage: quire-spec compile <request-file>\n"
        );
    }
}

#[cfg(target_os = "linux")]
#[test]
#[trace("TC-105", "FR-027-AC-3")]
fn failed_artifact_output_is_an_io_exit() {
    let directory = tempfile::tempdir().unwrap();
    fixtures::write(directory.path(), fixtures::Case::Aggregate(2));
    let full = std::fs::OpenOptions::new()
        .write(true)
        .open("/dev/full")
        .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_quire-spec"))
        .arg("compile")
        .arg(directory.path().join("compile.json"))
        .stdout(std::process::Stdio::from(full))
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(std::str::from_utf8(&output.stderr)
        .unwrap()
        .starts_with("output failed:"));
}

#[test]
#[trace("TC-105", "FR-027-AC-3", "TC-104", "FR-026-AC-4")]
fn file_count_refusals_name_the_exhausted_group_before_file_io() {
    for (field, remaining) in [("models", 63), ("snapshots", 62), ("invocations", 60)] {
        let directory = tempfile::tempdir().unwrap();
        let (mut job, _) = fixtures::write(
            directory.path(),
            fixtures::Case::Operation {
                violate_frame: false,
            },
        );
        let item = job["request"][field][0].clone();
        job["request"][field] = json!(vec![item; 64]);
        job["request"]["program"]["source"]["file"] = json!("missing.native");
        std::fs::write(
            directory.path().join("request.json"),
            serde_json::to_vec(&job).unwrap(),
        )
        .unwrap();
        let output = Command::new(env!("CARGO_BIN_EXE_quire-spec"))
            .arg("run")
            .arg(directory.path().join("request.json"))
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(3), "{field}");
        assert!(output.stdout.is_empty());
        let value: Value = serde_json::from_slice(&output.stderr).unwrap();
        assert_eq!(value["code"], "resource_exhausted");
        assert_eq!(
            value["details"],
            json!({"limit":"selected files", "category":field,
            "requested":64, "remaining":remaining, "maximum":64})
        );
    }
}

#[test]
#[trace("TC-105", "FR-027-AC-1", "FR-027-AC-2")]
fn shared_format_catalog_preserves_existing_wire_spellings() {
    use quire_spec_language::wire_format::WireFormat;
    let names: Vec<_> = WireFormat::ALL
        .iter()
        .map(|format| serde_json::to_value(format).unwrap())
        .collect();
    assert_eq!(
        names,
        vec![
            json!("native-run/1"),
            json!("native-compile/1"),
            json!("native-run-result/1"),
            json!("native-rule-model/1"),
            json!("native-state-input/1"),
            json!("native-linked-package/1")
        ]
    );
    let result_schema: Value =
        serde_json::from_str(include_str!("../schemas/native-run-result-1.schema.json")).unwrap();
    assert_eq!(
        result_schema["$defs"]["report"]["properties"]["format"]["const"],
        WireFormat::RunResult.as_str()
    );
    assert_eq!(
        quire_spec_language::model_source::FORMAT,
        WireFormat::RuleModel.as_str()
    );
}
