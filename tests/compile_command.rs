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
            fixtures::Case::Operation(false),
            "pre(self.n) = 1 and self.n = 2 and result",
            quire_spec_language::syntax::ClauseKind::Postcondition,
        ),
    ] {
        let directory = tempfile::tempdir().unwrap();
        let (_, digest) = fixtures::write(directory.path(), case).unwrap();
        for name in [
            "snapshot-0.json",
            "snapshot-1.json",
            "invocation-0.json",
            "request.json",
        ] {
            let path = directory.path().join(name);
            if path.exists() {
                std::fs::remove_file(path).unwrap();
            }
        }
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
    let directory = tempfile::tempdir().unwrap();
    fixtures::write(directory.path(), fixtures::Case::Aggregate(2)).unwrap();
    let original = std::fs::read(directory.path().join("compile.json")).unwrap();
    let job: Value = serde_json::from_slice(&original).unwrap();
    for (change, expected, code) in [
        ("format", 1, "unknown_wire"),
        ("runtime", 2, "invalid-request"),
        ("stale", 1, "source_digest_mismatch"),
        ("syntax", 1, "invalid_syntax"),
    ] {
        let mut changed = job.clone();
        match change {
            "format" => changed["format"] = json!("native-run/1"),
            "runtime" => changed["request"]["snapshots"] = json!([]),
            "stale" => {
                changed["request"]["program"]["source"]["digest"] =
                    json!(ByteDigest::of(b"foreign").to_string())
            }
            "syntax" => {
                let broken = b"language ?";
                std::fs::write(directory.path().join("program.native"), broken).unwrap();
                changed["request"]["program"]["source"]["digest"] =
                    json!(ByteDigest::of(broken).to_string());
            }
            _ => unreachable!(),
        }
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
