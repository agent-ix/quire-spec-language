// SPDX-License-Identifier: AGPL-3.0-or-later
//! Prepared command inputs and typed outcome projection at the public seam.

use std::path::Path;

use ix_trace_rs::trace;
use qsl_command::command::{self, PreparedRun, RunLimits};
use qsl_foundation::SourceIdentity;
use qsl_replay::spine::{self, DependencyInput, ParseRequest, SpineLimits};
use qsl_replay::{Operation, OutcomeDocument};
use quire_exact::Cancel;
use serde_json::{json, Value};

const PROGRAM: &[u8] = include_bytes!("../../../tests/fixtures/spine-run.native");
const DOMAIN: &[u8] = include_bytes!("../../../examples/config-version/model.semantic-ir.json");
const HEADER: &str =
    "language \"ix:native\" edition \"1-draft\";\nprofile v = \"quire.value.complete/v1\";\n";

fn source() -> Value {
    json!({"file":"program.native", "authority":"offered issuer", "identity":"offered:unit",
        "document":"PreparedCommand", "formal_revision":1})
}

fn write_request(directory: &Path, filename: &str, request: Value) -> std::path::PathBuf {
    let path = directory.join(filename);
    std::fs::write(&path, serde_json::to_vec(&request).unwrap()).unwrap();
    path
}

fn compile_request(directory: &Path, bytes: &[u8]) -> std::path::PathBuf {
    std::fs::write(directory.join("program.native"), bytes).unwrap();
    write_request(
        directory,
        "compile.json",
        json!({"format":"native-compile/1", "request":{
        "models":[], "program":{"source":source()}}}),
    )
}

fn run_request(directory: &Path, work_units: u64) -> std::path::PathBuf {
    std::fs::write(directory.join("program.native"), PROGRAM).unwrap();
    std::fs::write(directory.join("library.native"), PROGRAM).unwrap();
    std::fs::write(directory.join("model.json"), DOMAIN).unwrap();
    write_request(
        directory,
        "run.json",
        json!({"format":"native-run/1", "request":{
        "models":[{"format":"semantic-ir/2.0.0","source":{
            "file":"model.json","authority":"model issuer","identity":"model:source",
            "document":"DomainModel","formal_revision":1}}],
        "libraries":[{"identity":"fixture:library","source":{
            "file":"library.native","authority":"library issuer","identity":"library:source",
            "document":"LibrarySource","formal_revision":1}}],
        "program":{"source":source()}, "call":{
            "function":"lt", "arguments":[{"parameter":"a","value":2},{"parameter":"b","value":5}],
            "accounting":{"work_units":work_units}}}}),
    )
}

fn document(document: &OutcomeDocument) -> Value {
    serde_json::from_slice(&document.to_bytes().unwrap()).unwrap()
}

#[test]
#[trace("TC-771", "FR-287-AC-10")]
fn compilation_retains_the_actual_core_document_and_the_existing_artifact() {
    let directory = tempfile::tempdir().unwrap();
    let path = compile_request(directory.path(), PROGRAM);
    let actual = command::compile_outcome(&path).unwrap();
    assert_eq!(actual.package_bytes(), command::compile(&path).unwrap());

    let identity = SourceIdentity::new("offered issuer", "offered:unit");
    let limits = SpineLimits::default();
    let cancel = Cancel::new();
    let parsed = spine::parse(
        &ParseRequest {
            source: &identity,
            path: "program.native",
            bytes: PROGRAM,
        },
        limits.source,
        &cancel,
    )
    .unwrap()
    .into_value();
    let selected = spine::select(&parsed, &Default::default(), limits.model, &cancel)
        .unwrap()
        .into_value();
    let checked = spine::check(
        &parsed,
        "offered:unit".into(),
        &selected,
        &DependencyInput::default(),
        limits,
        &cancel,
    )
    .unwrap()
    .into_value();
    let packaged = spine::package(&checked, limits.package, &cancel);
    let expected = OutcomeDocument::from_package(&packaged);
    assert_eq!(
        actual.document().to_bytes().unwrap(),
        expected.to_bytes().unwrap()
    );
    assert_eq!(
        actual.package_bytes(),
        packaged.unwrap().into_value().package().bytes()
    );
    assert_eq!(document(actual.document())["last_stage"], "S4");
}

#[test]
#[trace("TC-771", "FR-287-AC-10")]
fn preparation_retains_owned_inputs_and_the_existing_run_outcomes() {
    for (work_units, expected_category, expected_kind) in [
        (100_000, "success", "completed"),
        (0, "incomplete", "incomplete"),
    ] {
        let directory = tempfile::tempdir().unwrap();
        let path = run_request(directory.path(), work_units);
        let native = command::run(&path).unwrap();
        let PreparedRun::Function(prepared) =
            command::prepare_run(&path, RunLimits::default()).unwrap()
        else {
            panic!("a function selection remains a function");
        };
        let input = prepared.into_input();
        assert_eq!(
            input.source.identity(),
            &SourceIdentity::new("offered issuer", "offered:unit")
        );
        assert_eq!(input.source.path(), "program.native");
        assert_eq!(input.source.text().as_bytes(), PROGRAM);
        assert_eq!(input.package_name, "offered:unit");
        assert_eq!(
            input.packages,
            qsl_semantics::model::intake::package_input([DOMAIN]).unwrap()
        );
        assert_eq!(
            input.dependencies,
            DependencyInput::new([spine::SuppliedLibrary {
                identity: "fixture:library".into(),
                source: SourceIdentity::new("library issuer", "library:source"),
                path: "library.native".into(),
                bytes: PROGRAM.to_vec(),
            }])
            .unwrap()
        );
        assert_eq!(input.call.function, "lt");
        assert_eq!(input.call.accounting.work_units, work_units);
        assert_eq!(input.call.arguments.len(), 2);
        assert_eq!(input.call.arguments[0].parameter, "a");
        let quire_exact::Value::Integer(value) = &input.call.arguments[0].value else {
            panic!("the request's integer argument retains its typed value");
        };
        assert_eq!(value, &quire_exact::Integer::from(2));
        for file in ["program.native", "library.native", "model.json", "run.json"] {
            std::fs::remove_file(directory.path().join(file)).unwrap();
        }
        let result = spine::run(
            input.source.identity().clone(),
            input.package_name,
            input.source.path(),
            input.source.text().as_bytes(),
            &input.packages,
            &input.dependencies,
            SpineLimits::default(),
            &input.call,
            &Default::default(),
            &Cancel::new(),
        );
        let outcome = OutcomeDocument::from_run(&result);
        assert_eq!(outcome.category().exit_code(), native.exit_code);
        assert_eq!(document(&outcome)["category"], expected_category);
        assert_eq!(native.value.as_value()["outcome"]["kind"], expected_kind);
    }
}

#[test]
#[trace("TC-771", "FR-287-AC-11")]
fn intake_failures_have_their_original_codes_and_no_invented_stage() {
    let directory = tempfile::tempdir().unwrap();
    let missing = command::compile_outcome(&directory.path().join("missing.json"))
        .err()
        .expect("a missing request refuses");
    let missing = document(&missing.outcome_document(Operation::Package));
    assert_eq!(missing["last_stage"], Value::Null);
    assert_eq!(missing["diagnostics"][0]["code"], "io-error");
    assert_eq!(missing["category"], "refusal");

    let path = directory.path().join("invalid.json");
    std::fs::write(&path, b"{not-json").unwrap();
    let malformed = command::prepare_run(&path, RunLimits::default())
        .err()
        .expect("malformed JSON refuses");
    let malformed = document(&malformed.outcome_document(Operation::Execute));
    assert_eq!(malformed["last_stage"], Value::Null);
    assert_eq!(malformed["diagnostics"][0]["code"], "invalid-request");
    assert_eq!(malformed["category"], "refusal");
}

#[test]
#[trace("TC-771", "FR-287-AC-11")]
fn stage_refusals_preserve_the_reached_stage_and_available_location() {
    for (body, stage, code, located) in [
        (
            "function f using v(): Int[0, 9] pure { 7\n",
            "S1",
            "invalid_syntax",
            true,
        ),
        (
            "function f using v(): Int[0, 9] pure { true }\n",
            "S3",
            "ill_typed",
            true,
        ),
        (
            "type Digit = Int[0, 9];\n",
            "S4",
            "unsupported_projection",
            false,
        ),
    ] {
        let directory = tempfile::tempdir().unwrap();
        let path = compile_request(directory.path(), format!("{HEADER}{body}").as_bytes());
        let error = command::compile_outcome(&path)
            .err()
            .expect("the stage refuses");
        let native = error.artifact_value().unwrap();
        let outcome = document(&error.outcome_document(Operation::Package));
        assert_eq!(outcome["last_stage"], stage, "{outcome}");
        assert_eq!(outcome["diagnostics"][0]["code"], code, "{outcome}");
        assert_eq!(
            outcome["diagnostics"][0]["locus"].is_object(),
            located,
            "{outcome}"
        );
        assert_eq!(
            outcome["category"],
            if code == "unsupported_projection" {
                "unsupported"
            } else {
                "refusal"
            }
        );
        assert_eq!(native["code"], code);
    }
}

#[test]
#[trace("TC-771", "FR-287-AC-11", "TC-424", "FR-001-AC-16")]
fn source_intake_retains_both_native_and_canonical_projections_of_one_failure() {
    for (authority, identity, bytes, code, cause, label, region) in [
        (
            " ",
            "unit",
            b"ok".as_slice(),
            "invalid_source_identity",
            "blank-label",
            Some("authority"),
            None,
        ),
        (
            "issuer",
            "\u{3000}",
            b"ok".as_slice(),
            "invalid_source_identity",
            "blank-label",
            Some("identity"),
            None,
        ),
        (
            "issuer",
            "unit",
            b"a\xffb".as_slice(),
            "invalid_utf8",
            "invalid_utf8",
            None,
            Some((1, 1)),
        ),
        (
            "issuer",
            "unit",
            b"\xef\xbb\xbftext".as_slice(),
            "invalid_syntax",
            "invalid-token",
            None,
            Some((0, 3)),
        ),
        (
            "issuer",
            "unit",
            b"ab\0c".as_slice(),
            "invalid_syntax",
            "invalid-token",
            None,
            Some((2, 3)),
        ),
    ] {
        let directory = tempfile::tempdir().unwrap();
        let path = compile_request(directory.path(), bytes);
        let mut request: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        request["request"]["program"]["source"]["authority"] = json!(authority);
        request["request"]["program"]["source"]["identity"] = json!(identity);
        std::fs::write(&path, serde_json::to_vec(&request).unwrap()).unwrap();
        let error = command::compile_outcome(&path)
            .err()
            .expect("source intake refuses");
        let outcome = document(&error.outcome_document(Operation::Package));
        assert_eq!(outcome["last_stage"], Value::Null);
        assert_eq!(outcome["diagnostics"][0]["code"], code);
        assert_eq!(outcome["diagnostics"][0]["cause"], cause);
        assert_eq!(
            outcome["diagnostics"][0]
                .get("label")
                .and_then(Value::as_str),
            label
        );
        let locus = &outcome["diagnostics"][0]["locus"];
        if let Some((start, end)) = region {
            assert_eq!(locus["kind"], "region");
            assert_eq!(locus["start"], start);
            assert_eq!(locus["end"], end);
        } else {
            assert_eq!(locus, &Value::Null);
        }
        let original = qsl_foundation::Source::read(
            SourceIdentity::new(authority, identity),
            "program.native",
            bytes,
            8_388_608,
        )
        .unwrap_err();
        let native = error.value().unwrap();
        assert_eq!(native["stage"], "source");
        assert_eq!(native["code"], original.code.as_str());
        assert_eq!(native["message"], original.to_string());
        assert_eq!(native["details"]["message"], original.message);
        assert_eq!(
            native["details"]["span"],
            serde_json::to_value(original.span).unwrap()
        );
        assert_eq!(
            native["details"]["source"],
            serde_json::to_value(original.source).unwrap()
        );
    }
}

#[test]
#[trace("TC-771", "FR-287-AC-10")]
fn preparation_does_not_compile_the_source_and_retains_caller_byte_limits() {
    let directory = tempfile::tempdir().unwrap();
    let path = run_request(directory.path(), 1_000);
    std::fs::write(
        directory.path().join("program.native"),
        b"not a native program",
    )
    .unwrap();
    let PreparedRun::Function(prepared) =
        command::prepare_run(&path, RunLimits::default()).unwrap()
    else {
        panic!("the request still selects a function");
    };
    assert_eq!(prepared.into_input().source.text(), "not a native program");
    let request_bytes = std::fs::read(&path).unwrap().len();
    let error = command::prepare_run(
        &path,
        RunLimits {
            request_bytes: request_bytes - 1,
            ..RunLimits::default()
        },
    )
    .err()
    .expect("the caller's request ceiling is used");
    assert!(
        matches!(error.cause, command::RunCause::Limit(command::LimitKind::RequestBytes { maximum })
        if maximum == request_bytes - 1)
    );
    let error = command::prepare_run(
        &path,
        RunLimits {
            dependent_bytes: 2,
            ..RunLimits::default()
        },
    )
    .err()
    .expect("the caller's dependent ceiling is used");
    assert!(matches!(
        error.cause,
        command::RunCause::Limit(command::LimitKind::DependentBytes { maximum: 2 })
    ));
}
