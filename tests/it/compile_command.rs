// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-027: actual CLI artifact bytes, verified consumer intake and stage failures.

use crate::support::standalone_setup as fixtures;
use fixtures::runtime;

use ix_trace_rs::trace;
use qsl_foundation::ByteDigest;
use quire_spec_language::package::{
    NativePackage, NativePackageRef, PackageReadLimits, PackageSupport,
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

/// FR-027-AC-6 (TC-435 step 3): these programs declare `edition "0-draft"`,
/// so they compile through native compile, byte for byte.
#[test]
#[trace("TC-105", "FR-027-AC-1", "FR-027-AC-2", "TC-435", "FR-027-AC-6")]
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
        let (_, digest) = fixtures::write(generated.path(), case).unwrap();
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
        fixtures::write(directory.path(), fixtures::Case::Aggregate(2)).unwrap();
        let original = std::fs::read(directory.path().join("compile.json")).unwrap();
        let mut changed: Value = serde_json::from_slice(&original).unwrap();
        let (expected, code) = match change {
            Change::Format => {
                changed["format"] = json!("native-run/1");
                (20, "unknown_wire")
            }
            Change::Runtime => {
                changed["request"]["snapshots"] = json!([]);
                (20, "invalid-request")
            }
            Change::Stale => {
                changed["request"]["program"]["source"]["digest"] =
                    json!(ByteDigest::of(b"foreign").to_string());
                (20, "source_digest_mismatch")
            }
            Change::Syntax => {
                let broken = b"language ?";
                std::fs::write(directory.path().join("program.native"), broken).unwrap();
                changed["request"]["program"]["source"]["digest"] =
                    json!(ByteDigest::of(broken).to_string());
                (20, "invalid_syntax")
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
        assert_eq!(output.status.code(), Some(20));
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
fn failed_artifact_output_is_a_tool_failure_exit() {
    let directory = tempfile::tempdir().unwrap();
    fixtures::write(directory.path(), fixtures::Case::Aggregate(2)).unwrap();
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
    assert_eq!(output.status.code(), Some(30));
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
        )
        .unwrap();
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
        assert_eq!(output.status.code(), Some(22), "{field}");
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
    use qsl_foundation::wire_format::WireFormat;
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
            json!("native-rule-model/2"),
            json!("native-state-input/1"),
            json!("native-linked-package/1")
        ]
    );
    let result_schema: Value = serde_json::from_str(include_str!(
        "../../schemas/native-run-result-1.schema.json"
    ))
    .unwrap();
    assert_eq!(
        result_schema["$defs"]["report"]["properties"]["format"]["const"],
        WireFormat::RunResult.as_str()
    );
    assert_eq!(
        quire_spec_language::model_source::FORMAT,
        WireFormat::RuleModel.as_str()
    );
}

/// TC-430 steps 3-4 (FR-027-AC-4): a compile request's program source
/// carries the four labels into the package `source`, which validates
/// against the native-linked-package/1 schema; a request without
/// `revision_namespace` refuses with `invalid-request`.
#[test]
#[trace("TC-430", "FR-027-AC-4")]
fn compiled_packages_name_the_four_source_labels() {
    let directory = tempfile::tempdir().unwrap();
    fixtures::write(directory.path(), fixtures::Case::Aggregate(2)).unwrap();
    let path = directory.path().join("compile.json");
    let mut job: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    let source = &mut job["request"]["program"]["source"];
    source["authority"] = json!("agent-ix");
    source["identity"] = json!("p");
    source["revision_namespace"] = json!("git");
    source["revision"] = json!("1");
    std::fs::write(&path, serde_json::to_vec(&job).unwrap()).unwrap();
    let output = compile(directory.path());
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let package: Value = serde_json::from_slice(&output.stdout).unwrap();
    for (label, value) in [
        ("authority", "agent-ix"),
        ("identity", "p"),
        ("revision_namespace", "git"),
        ("revision", "1"),
    ] {
        assert_eq!(package["source"][label], value, "{label}");
    }
    let schema: Value = serde_json::from_str(include_str!(
        "../../schemas/native-linked-package-1.schema.json"
    ))
    .unwrap();
    let schema = jsonschema::JSONSchema::options()
        .with_draft(jsonschema::Draft::Draft202012)
        .compile(&schema)
        .unwrap();
    assert!(schema.is_valid(&package));

    job["request"]["program"]["source"]
        .as_object_mut()
        .unwrap()
        .remove("revision_namespace");
    std::fs::write(&path, serde_json::to_vec(&job).unwrap()).unwrap();
    let output = compile(directory.path());
    assert_eq!(output.status.code(), Some(20));
    assert!(output.stdout.is_empty());
    let value: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(value["code"], "invalid-request");
}

/// The complete-V1 compile fixture (FR-027-AC-5).
const SPINE_FIXTURE: &str = "tests/fixtures/spine-compile.native";

/// The fixture's header and profile selection, which every stage-refusal
/// body below follows.
const SPINE_HEADER: &str = "language \"ix:native\" edition \"1-draft\";\nprofile v = \"quire.value.complete/v1\" version \"1\" digest \"sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\";\n";

/// Write `program` as `program.native` under `directory` with a
/// native-compile/1 request selecting it, no models and no clause bindings.
fn spine_request(directory: &Path, program: &[u8]) {
    std::fs::write(directory.join("program.native"), program).unwrap();
    let request = json!({"format":"native-compile/1","request":{"models":[],"program":{
        "source":{"file":"program.native","authority":"agent-ix","identity":"test:spine",
        "revision_namespace":"fixture","revision":"fixture:1",
        "digest":ByteDigest::of(program).to_string(),"document":"Spine","formal_revision":1},
        "clauses":[]}}});
    std::fs::write(
        directory.join("compile.json"),
        serde_json::to_vec(&request).unwrap(),
    )
    .unwrap();
}

/// FR-027-AC-5 (TC-435 step 2): a `1-draft` program compiles through the
/// spine; stdout is exactly `qsl_replay::spine::compile`'s bytes, which
/// qsl-package's TC-435 step 1 reads back Verified with nothing omitted.
#[test]
#[trace("TC-435", "FR-027-AC-5")]
fn a_complete_v1_program_compiles_through_the_spine() {
    let program = std::fs::read(SPINE_FIXTURE).unwrap();
    let directory = tempfile::tempdir().unwrap();
    spine_request(directory.path(), &program);
    let output = compile(directory.path());
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    let library = qsl_replay::spine::compile(
        qsl_foundation::SourceIdentity::new("agent-ix", "test:spine", "fixture", "fixture:1"),
        "program.native",
        &program,
        &std::collections::BTreeMap::new(),
        &qsl_replay::spine::DependencyInput::default(),
        qsl_replay::spine::SpineLimits::default(),
    )
    .unwrap();
    assert_eq!(output.stdout, library.emitted.bytes());
    let wire: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(wire["contract_version"], "quire.checked-package/v2");
}

/// FR-029-AC-3 (TC-107): `lower` refuses a `1-draft` program with
/// `unknown_edition`, naming the file and locating the edition literal by
/// span, the same existing intake behavior FR-029 says `lower` retains from
/// `compile`.
///
/// `lower`'s refusal comes from the native parser's own header check
/// (`src/parser.rs`), not `compile`'s `Edition::of` dispatch
/// (`an_edition_neither_compiler_reads_refuses`, above): its `message` is
/// the generic "supported edition is 0-draft", naming neither the file nor
/// the offered edition in prose, so this asserts the file through
/// `details.path` and the edition through the refusal's own span, over the
/// message text those two checks take there.
#[test]
#[trace("TC-107", "FR-029-AC-3")]
fn lower_refuses_a_complete_v1_program_as_unknown_edition() {
    let program = std::fs::read(SPINE_FIXTURE).unwrap();
    let directory = tempfile::tempdir().unwrap();
    spine_request(directory.path(), &program);
    let output = Command::new(env!("CARGO_BIN_EXE_quire-spec"))
        .arg("lower")
        .arg(directory.path().join("compile.json"))
        .current_dir(directory.path().parent().unwrap())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(20));
    assert!(output.stdout.is_empty());
    let failure: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(failure["code"], "unknown_edition", "{failure}");
    assert_eq!(failure["details"]["path"], "program.native", "{failure}");
    let text = String::from_utf8(program).expect("the spine fixture is UTF-8");
    let literal = text
        .find("\"1-draft\"")
        .expect("the fixture declares edition \"1-draft\"");
    assert_eq!(failure["details"]["span"]["start"]["byte"], literal);
    assert_eq!(
        failure["details"]["span"]["end"]["byte"],
        literal + "\"1-draft\"".len()
    );
}

/// FR-027-AC-5 (TC-435 step 2): a `1-draft` compile validates the program's
/// `document` and `formal_revision` but the v2 wire does not record them,
/// so two requests differing only there write identical bytes; an invalid
/// `document` still refuses.
#[test]
#[trace("TC-435", "FR-027-AC-5")]
fn formal_labels_are_validated_but_not_recorded_by_a_spine_compile() {
    let program = std::fs::read(SPINE_FIXTURE).unwrap();
    let directory = tempfile::tempdir().unwrap();
    spine_request(directory.path(), &program);
    let first = compile(directory.path());
    assert_eq!(first.status.code(), Some(0));
    let path = directory.path().join("compile.json");
    let mut job: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    job["request"]["program"]["source"]["document"] = json!("Elsewhere");
    job["request"]["program"]["source"]["formal_revision"] = json!(9);
    std::fs::write(&path, serde_json::to_vec(&job).unwrap()).unwrap();
    let second = compile(directory.path());
    assert_eq!(
        second.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&second.stderr)
    );
    assert!(!first.stdout.is_empty());
    assert_eq!(first.stdout, second.stdout);
    job["request"]["program"]["source"]["document"] = json!("");
    std::fs::write(&path, serde_json::to_vec(&job).unwrap()).unwrap();
    let invalid = compile(directory.path());
    assert_eq!(invalid.status.code(), Some(20));
    assert!(invalid.stdout.is_empty());
    let failure: Value = serde_json::from_slice(&invalid.stderr).unwrap();
    assert_eq!(failure["stage"], "request", "{failure}");
}

/// FR-027-AC-7 (TC-435 step 4): an edition neither compiler reads refuses
/// with `unknown_edition` at its literal, naming the file and the edition.
#[test]
#[trace("TC-435", "FR-027-AC-7")]
fn an_edition_neither_compiler_reads_refuses() {
    let directory = tempfile::tempdir().unwrap();
    fixtures::write(directory.path(), fixtures::Case::Aggregate(2)).unwrap();
    let path = directory.path().join("program.native");
    let program = std::fs::read_to_string(&path).unwrap().replacen(
        "edition \"0-draft\"",
        "edition \"7-draft\"",
        1,
    );
    assert!(program.contains("\"7-draft\""));
    std::fs::write(&path, &program).unwrap();
    let request = directory.path().join("compile.json");
    let mut job: Value = serde_json::from_slice(&std::fs::read(&request).unwrap()).unwrap();
    job["request"]["program"]["source"]["digest"] =
        json!(ByteDigest::of(program.as_bytes()).to_string());
    std::fs::write(&request, serde_json::to_vec(&job).unwrap()).unwrap();
    let output = compile(directory.path());
    assert_eq!(output.status.code(), Some(20));
    assert!(output.stdout.is_empty());
    let failure: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(failure["code"], "unknown_edition", "{failure}");
    assert_eq!(failure["stage"], "profile", "{failure}");
    let message = failure["message"].as_str().unwrap();
    assert!(message.contains("program.native"), "{message}");
    assert!(message.contains("7-draft"), "{message}");
    let literal = program.find("\"7-draft\"").unwrap();
    assert_eq!(failure["details"]["span"]["start"]["byte"], literal);
    assert_eq!(
        failure["details"]["span"]["end"]["byte"],
        literal + "\"7-draft\"".len()
    );
}

/// FR-027-AC-7 (TC-435 step 5): a `1-draft` program compiles alone; a
/// request that selects models or clause bindings for it refuses, whatever
/// state the model files are in.
#[test]
#[trace("TC-435", "FR-027-AC-7")]
fn a_complete_v1_request_selecting_native_inputs_refuses() {
    let directory = tempfile::tempdir().unwrap();
    fixtures::write(directory.path(), fixtures::Case::Aggregate(2)).unwrap();
    let program = std::fs::read(SPINE_FIXTURE).unwrap();
    std::fs::write(directory.path().join("program.native"), &program).unwrap();
    let request = directory.path().join("compile.json");
    let mut job: Value = serde_json::from_slice(&std::fs::read(&request).unwrap()).unwrap();
    job["request"]["program"]["source"]["digest"] = json!(ByteDigest::of(&program).to_string());
    assert!(!job["request"]["models"].as_array().unwrap().is_empty());
    // Malformed model files: the edition is decided before any model is
    // read, so the refusal names the selection, not the files.
    for model in job["request"]["models"].as_array().unwrap() {
        let file = model["source"]["file"].as_str().unwrap();
        std::fs::write(directory.path().join(file), b"not a model {").unwrap();
    }
    assert!(!job["request"]["program"]["clauses"]
        .as_array()
        .unwrap()
        .is_empty());
    for (strip, selected) in [
        (None, "native rule-model sources"),
        (Some("models"), "clause bindings"),
    ] {
        if let Some(field) = strip {
            job["request"][field] = json!([]);
        }
        std::fs::write(&request, serde_json::to_vec(&job).unwrap()).unwrap();
        let output = compile(directory.path());
        assert_eq!(output.status.code(), Some(20), "{selected}");
        assert!(output.stdout.is_empty());
        let failure: Value = serde_json::from_slice(&output.stderr).unwrap();
        assert_eq!(failure["code"], "invalid-request", "{failure}");
        assert_eq!(failure["stage"], "request", "{failure}");
        assert!(
            failure["message"].as_str().unwrap().ends_with(selected),
            "{failure}"
        );
    }
    job["request"]["program"]["clauses"] = json!([]);
    std::fs::write(&request, serde_json::to_vec(&job).unwrap()).unwrap();
    let output = compile(directory.path());
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// FR-027-AC-8 (TC-435 step 6): each spine stage's refusal reports that
/// stage and its cause's code, exits on that code, and writes no bytes.
#[test]
#[trace("TC-435", "FR-027-AC-8")]
fn each_spine_stage_refusal_reports_its_stage_and_code() {
    for (body, stage, code, exit, located) in [
        (
            "function f using v(): Int[0, 9] pure { 7\n",
            "source",
            "invalid_syntax",
            20,
            None,
        ),
        (
            "predicate p using v(): Boolean { true }\n",
            "forms",
            "unsupported_construct",
            21,
            Some("predicate p using v(): Boolean { true }"),
        ),
        (
            "function f using v(p: Nope): Int[0, 9] pure { 1 }\n",
            "assembly",
            "missing_declaration",
            20,
            Some("Nope"),
        ),
        (
            "function f using v(): Int[0, 9] pure { true }\n",
            "check",
            "ill_typed",
            20,
            Some("true"),
        ),
        (
            "type Digit = Int[0, 9];\n",
            "emit",
            "unsupported_projection",
            21,
            None,
        ),
    ] {
        let text = format!("{SPINE_HEADER}{body}");
        let directory = tempfile::tempdir().unwrap();
        spine_request(directory.path(), text.as_bytes());
        let output = compile(directory.path());
        assert_eq!(output.status.code(), Some(exit), "{stage}");
        assert!(output.stdout.is_empty(), "{stage}");
        let failure: Value = serde_json::from_slice(&output.stderr).unwrap();
        assert_eq!(failure["stage"], stage, "{failure}");
        assert_eq!(failure["code"], code, "{failure}");
        assert_eq!(failure["status"], "refused", "{failure}");
        assert_eq!(failure["details"]["path"], "program.native", "{failure}");
        let message = failure["message"].as_str().unwrap();
        assert!(!message.contains('{'), "a Debug dump: {message}");
        let span = &failure["details"]["span"];
        match located {
            Some(located) => {
                let start = SPINE_HEADER.len() + body.find(located).unwrap();
                assert_eq!(span["start"]["byte"], start, "{failure}");
                assert_eq!(span["end"]["byte"], start + located.len(), "{failure}");
            }
            None if stage == "emit" => assert!(span.is_null(), "{failure}"),
            None => assert!(span.is_object(), "{failure}"),
        }
    }
}

/// FR-027-AC-5 (TC-435 step 7): a source that does not open with an
/// `ix:native` header declares no edition and goes to native compile, whose
/// parser reports its header.
#[test]
#[trace("TC-435", "FR-027-AC-5")]
fn a_source_without_an_ix_native_header_goes_to_native() {
    for (program, code) in [
        (
            "function f using v(): Int[0, 9] pure { 7 }\n".to_owned(),
            "invalid_syntax",
        ),
        (
            SPINE_HEADER.replacen("ix:native", "ix:other", 1)
                + "function f using v(): Int[0, 9] pure { 7 }\n",
            "unknown_language",
        ),
    ] {
        let directory = tempfile::tempdir().unwrap();
        spine_request(directory.path(), program.as_bytes());
        let output = compile(directory.path());
        assert_eq!(output.status.code(), Some(20), "{program}");
        assert!(output.stdout.is_empty());
        let failure: Value = serde_json::from_slice(&output.stderr).unwrap();
        assert_eq!(failure["code"], code, "{failure}");
        // The native parser's diagnostic carries its phase; no spine
        // refusal does.
        assert!(failure["details"]["phase"].is_string(), "{failure}");
        assert!(
            !["source", "forms", "assembly", "check", "emit"]
                .contains(&failure["stage"].as_str().unwrap()),
            "{failure}"
        );
    }
}

/// The complete-V1 compile fixture with a domain package (FR-027-AC-9).
const SPINE_MODEL_FIXTURE: &str = "tests/fixtures/spine-model.native";

/// The domain package document `SPINE_MODEL_FIXTURE`'s `model M` selects.
const SPINE_MODEL_DOCUMENT: &str = "tests/fixtures/spine-model.semantic-ir.json";

/// [`spine_request`], with `document` written as `orders.json` and selected
/// as a `semantic-ir/2.0.0` model.
fn spine_model_request(directory: &Path, program: &[u8], document: &[u8]) {
    spine_request(directory, program);
    std::fs::write(directory.join("orders.json"), document).unwrap();
    let path = directory.join("compile.json");
    let mut job: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    job["request"]["models"] = json!([{"format":"semantic-ir/2.0.0","source":{
        "file":"orders.json","authority":"agent-ix","identity":"acme/orders",
        "revision_namespace":"fixture","revision":"fixture:1",
        "digest":ByteDigest::of(document).to_string(),"document":"Orders","formal_revision":1}}]);
    std::fs::write(&path, serde_json::to_vec(&job).unwrap()).unwrap();
}

/// FR-027-AC-9 (TC-442 step 2): a `1-draft` request selecting a domain
/// package document compiles its program through the spine. Stdout is
/// exactly `qsl_replay::spine::compile`'s bytes over the same source and
/// package input, and the lock and identity preimage select the domain
/// package by the `sha256-jcs` digest the program's `model` declaration
/// names.
#[test]
#[trace("TC-442", "FR-027-AC-9")]
fn a_complete_v1_request_with_a_domain_package_locks_its_model_selection() {
    let program = std::fs::read(SPINE_MODEL_FIXTURE).unwrap();
    let document = std::fs::read(SPINE_MODEL_DOCUMENT).unwrap();
    let directory = tempfile::tempdir().unwrap();
    spine_model_request(directory.path(), &program, &document);
    let output = compile(directory.path());
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let library = qsl_replay::spine::compile(
        qsl_foundation::SourceIdentity::new("agent-ix", "test:spine", "fixture", "fixture:1"),
        "program.native",
        &program,
        &qsl_semantics::model::intake::package_input([document.as_slice()]),
        &qsl_replay::spine::DependencyInput::default(),
        qsl_replay::spine::SpineLimits::default(),
    )
    .unwrap();
    assert_eq!(output.stdout, library.emitted.bytes());
    let wire: Value = serde_json::from_slice(&output.stdout).unwrap();
    let selected = std::str::from_utf8(&program)
        .unwrap()
        .split("digest \"sha256-jcs:")
        .nth(1)
        .and_then(|rest| rest.split('"').next())
        .unwrap()
        .to_owned();
    let selection = json!([{
        "identity": "acme/orders",
        "version": "1.0.0",
        "digest_domain": "sha256-jcs",
        "digest": selected,
    }]);
    assert_eq!(wire["lock"]["model_selections"], selection);
    assert_eq!(wire["identity_preimage"]["model_selections"], selection);
}

/// FR-027-AC-9, FR-056-AC-9 (TC-442 step 4): the model fixture refuses at
/// the stage that owns each defect, at its region: `M::Nope`, which the
/// admitted package does not declare, at `assembly`; `deref(g).nope`, a
/// field neither `Gadget` nor its supertype declares, and `g = w` over a
/// `Gadget` and an unrelated `Rock` (FR-082 compares references of one
/// type only), at `check`; the `model` declaration, when the request
/// supplies no domain package or a document whose digest differs, at
/// `intake`; and a `sha256:` model digest at `intake`.
#[test]
#[trace("TC-442", "FR-027-AC-9", "FR-056-AC-9")]
fn a_model_bearing_request_refuses_at_the_owning_stage() {
    let program = std::fs::read_to_string(SPINE_MODEL_FIXTURE).unwrap();
    let document = std::fs::read(SPINE_MODEL_DOCUMENT).unwrap();
    let declaration = program
        .lines()
        .find(|line| line.starts_with("model M = "))
        .unwrap();
    let artifact = program.replacen("sha256-jcs:", "sha256:", 1);
    // Another document of the same package: its `sha256-jcs` digest is not
    // the one the declaration selects.
    let mut changed_document: Value = serde_json::from_slice(&document).unwrap();
    changed_document["package"]["lockDigest"] = json!(format!("sha256:{}", "1".repeat(64)));
    let changed_document = serde_json::to_vec(&changed_document).unwrap();
    for (text, supplied, stage, code, located) in [
        (
            program.replacen("deref(g).code", "deref(g).nope", 1),
            Some(document.clone()),
            "check",
            "ill_typed",
            "deref(g).nope".to_owned(),
        ),
        (
            program.replacen("w: M::Widget): Boolean", "w: M::Rock): Boolean", 1),
            Some(document.clone()),
            "check",
            "ill_typed",
            "g = w".to_owned(),
        ),
        (
            program.replacen("Reference<M::Widget>", "Reference<M::Nope>", 1),
            Some(document.clone()),
            "assembly",
            "missing_declaration",
            "M::Nope".to_owned(),
        ),
        (
            program.clone(),
            None,
            "intake",
            "missing_import",
            declaration.to_owned(),
        ),
        (
            program.clone(),
            Some(changed_document),
            "intake",
            "missing_import",
            declaration.to_owned(),
        ),
        (
            artifact.clone(),
            Some(document.clone()),
            "intake",
            "invalid_model_binding",
            artifact
                .lines()
                .find(|line| line.starts_with("model M = "))
                .unwrap()
                .to_owned(),
        ),
    ] {
        let directory = tempfile::tempdir().unwrap();
        match &supplied {
            Some(supplied) => spine_model_request(directory.path(), text.as_bytes(), supplied),
            None => spine_request(directory.path(), text.as_bytes()),
        }
        let output = compile(directory.path());
        assert_eq!(output.status.code(), Some(20), "{stage} {code}");
        assert!(output.stdout.is_empty());
        let failure: Value = serde_json::from_slice(&output.stderr).unwrap();
        assert_eq!(failure["stage"], stage, "{failure}");
        assert_eq!(failure["code"], code, "{failure}");
        let message = failure["message"].as_str().unwrap();
        assert!(!message.contains('{'), "a Debug dump: {message}");
        let start = text.find(&located).unwrap();
        let span = &failure["details"]["span"];
        assert_eq!(span["start"]["byte"], start, "{failure}");
        assert_eq!(span["end"]["byte"], start + located.len(), "{failure}");
    }
}

/// `test/geometry`'s source, a library `f` the importing program calls.
const GEOMETRY: &str = "function f using v(x: Int[0, 9]): Boolean pure { x < 5 }\n";

/// The source identity `geometry.native` is selected under.
fn geometry_identity() -> qsl_foundation::SourceIdentity {
    qsl_foundation::SourceIdentity::new("agent-ix", "test:geometry", "fixture", "fixture:1")
}

/// [`spine_request`] for a program importing `test/geometry` version `1`,
/// with `library` written as `geometry.native` and selected as that
/// library under `identity` and `version`. The import's digest is the
/// `package_id` `library` compiles to from source when it compiles, and an
/// arbitrary digest otherwise.
fn spine_library_request(
    directory: &Path,
    library: &[u8],
    identity: &str,
    version: &str,
) -> Vec<u8> {
    let digest = qsl_replay::spine::compile(
        geometry_identity(),
        "geometry.native",
        library,
        &std::collections::BTreeMap::new(),
        &qsl_replay::spine::DependencyInput::default(),
        qsl_replay::spine::SpineLimits::default(),
    )
    .map_or_else(|_| "e".repeat(64), |compiled| compiled.emitted.package_id().hex());
    let program = format!(
        "{SPINE_HEADER}import \"test/geometry\" version \"1\" digest \"{digest}\" as g;\n\
         function u using v(x: Int[0, 9]): Boolean pure {{ g::f(x) }}\n"
    )
    .into_bytes();
    spine_request(directory, &program);
    std::fs::write(directory.join("geometry.native"), library).unwrap();
    let path = directory.join("compile.json");
    let mut job: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    job["request"]["libraries"] = json!([{"identity":identity,"version":version,"source":{
        "file":"geometry.native","authority":"agent-ix","identity":"test:geometry",
        "revision_namespace":"fixture","revision":"fixture:1",
        "digest":ByteDigest::of(library).to_string(),"document":"Geometry","formal_revision":1}}]);
    std::fs::write(&path, serde_json::to_vec(&job).unwrap()).unwrap();
    program
}

/// FR-027-AC-10 (TC-446 step 7): a `1-draft` request's `libraries` supply
/// the program's import. Stdout is exactly `qsl_replay::spine::compile`'s
/// bytes over the same program and dependency input.
#[test]
#[trace("TC-446", "FR-027-AC-10")]
fn a_complete_v1_request_supplies_its_libraries_to_the_spine() {
    let library = format!("{SPINE_HEADER}{GEOMETRY}").into_bytes();
    let directory = tempfile::tempdir().unwrap();
    let program = spine_library_request(directory.path(), &library, "test/geometry", "1");
    let output = compile(directory.path());
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    let dependencies =
        qsl_replay::spine::DependencyInput::new([qsl_replay::spine::SuppliedLibrary {
            identity: "test/geometry".to_owned(),
            version: "1".to_owned(),
            source: geometry_identity(),
            path: "geometry.native".to_owned(),
            bytes: library,
        }])
        .unwrap();
    let compiled = qsl_replay::spine::compile(
        qsl_foundation::SourceIdentity::new("agent-ix", "test:spine", "fixture", "fixture:1"),
        "program.native",
        &program,
        &std::collections::BTreeMap::new(),
        &dependencies,
        qsl_replay::spine::SpineLimits::default(),
    )
    .unwrap();
    assert_eq!(output.stdout, compiled.emitted.bytes());
}

/// FR-027-AC-10 (TC-446 step 7): a library with an empty identity or
/// version, and any library beside a `0-draft` program, refuse
/// `invalid-request` with nothing written.
#[test]
#[trace("TC-446", "FR-027-AC-10")]
fn malformed_libraries_refuse_as_invalid_request() {
    let library = format!("{SPINE_HEADER}{GEOMETRY}").into_bytes();
    for (identity, version) in [("", "1"), ("test/geometry", "")] {
        let directory = tempfile::tempdir().unwrap();
        spine_library_request(directory.path(), &library, identity, version);
        let output = compile(directory.path());
        assert_eq!(output.status.code(), Some(20), "{identity:?} {version:?}");
        assert!(output.stdout.is_empty());
        let failure: Value = serde_json::from_slice(&output.stderr).unwrap();
        assert_eq!(failure["code"], "invalid-request", "{failure}");
    }
    let generated = tempfile::tempdir().unwrap();
    fixtures::write(generated.path(), fixtures::Case::Aggregate(2)).unwrap();
    let path = generated.path().join("compile.json");
    let mut job: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    job["request"]["libraries"] = json!([{"identity":"test/geometry","version":"1","source":
        job["request"]["program"]["source"].clone()}]);
    std::fs::write(&path, serde_json::to_vec(&job).unwrap()).unwrap();
    let output = compile(generated.path());
    assert_eq!(output.status.code(), Some(20));
    assert!(output.stdout.is_empty());
    let failure: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(failure["code"], "invalid-request", "{failure}");
}

/// FR-027-AC-10: a library's own refusal is reported through the program's
/// import and rendered over the library's source, not the program's.
#[test]
#[trace("TC-446", "FR-027-AC-10")]
fn a_library_refusal_renders_over_the_library_source() {
    let body = "function f using v(x: Int[0, 9]): Boolean pure { x < true }\n";
    let library = format!("{SPINE_HEADER}{body}").into_bytes();
    let directory = tempfile::tempdir().unwrap();
    spine_library_request(directory.path(), &library, "test/geometry", "1");
    let output = compile(directory.path());
    assert_eq!(output.status.code(), Some(20));
    assert!(output.stdout.is_empty());
    let failure: Value = serde_json::from_slice(&output.stderr).unwrap();
    assert_eq!(failure["details"]["path"], "geometry.native", "{failure}");
    let start = SPINE_HEADER.len() + body.find("x < true").unwrap();
    let end = (start + "x < true".len()) as u64;
    let span = &failure["details"]["span"];
    let (from, to) = (
        span["start"]["byte"].as_u64().unwrap(),
        span["end"]["byte"].as_u64().unwrap(),
    );
    assert!(from >= start as u64 && to <= end, "{failure}");
}
