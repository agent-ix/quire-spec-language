// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-278, FR-286: a driver serializes actual parse results through the
//! public replay facade, preserving the producer's diagnostics.

use ix_trace_rs::trace;
use qsl_foundation::diagnostic::StageFailure;
use qsl_replay::spine::{parse, CompileRefusal, ParseRequest, SpineLimits, SpineStage};
use qsl_replay::{Category, InternalFault, OutcomeDocument, SourceIdentity};
use quire_exact::{Cancel, CancelCause};
use serde_json::{json, Value};

const FIXTURE: &[u8] = include_bytes!("../../../tests/fixtures/spine-compile.native");
const HEADER: &str =
    "language \"ix:native\" edition \"1-draft\";\nprofile v = \"quire.value.complete/v1\";\n";

fn identity() -> SourceIdentity {
    SourceIdentity::new("agent-ix", "test:parse-outcome", "fixture", "fixture:1")
}

fn document_json(document: &OutcomeDocument) -> Value {
    let bytes = document.to_bytes().expect("the document encodes");
    assert_eq!(bytes, document.to_bytes().expect("encoding is deterministic"));
    serde_json::from_slice(&bytes).expect("the document is JSON")
}

fn expected(stage: Value, category: &str, diagnostics: Value) -> Value {
    json!({
        "format": "quire-outcome/1",
        "operation": "parse",
        "last_stage": stage,
        "category": category,
        "items": [],
        "diagnostics": diagnostics,
        "artifacts": [],
        "result": null
    })
}

fn diagnostic(refusal: &CompileRefusal) -> Value {
    let locus = refusal.region().map(|region| {
        json!({
            "kind": "region",
            "source": region.source(),
            "start": region.start(),
            "end": region.end()
        })
    });
    json!({
        "cause": refusal.cause(),
        "code": refusal.code().as_str(),
        "locus": locus,
        "message": refusal.to_string()
    })
}

/// Parsing the admitted fixture produces syntax only, without a package or
/// evaluation payload.
#[trace("FR-286-AC-1", "FR-286-AC-3", "FR-278-AC-1")]
#[test]
fn a_driver_serializes_parse_success_at_s2_without_artifacts() {
    let source = identity();
    let result = parse(
        &ParseRequest {
            source: &source,
            path: "unit.native",
            bytes: FIXTURE,
        },
        SpineLimits::default().source,
        &Cancel::new(),
    );
    let parsed = result.as_ref().expect("the admitted fixture parses");
    assert_eq!(parsed.value().source().text().as_bytes(), FIXTURE);
    let document = OutcomeDocument::from_parse(&result);
    assert_eq!(document.category(), Category::Success);
    assert_eq!(document.category().exit_code(), 0);
    assert_eq!(
        document_json(&document),
        expected(json!("S2"), "success", json!([]))
    );
}

/// Both S1 syntax refusal and S2 unsupported forms retain the actual
/// producer's code, cause, message and located source region.
#[trace("FR-278-AC-2", "FR-286-AC-2", "FR-286-AC-3")]
#[test]
fn a_driver_serializes_parse_refusals_with_the_original_locus() {
    let source = identity();
    let cases = [
        (
            "function f using v(: Boolean pure { true }",
            "invalid_syntax",
            SpineStage::Source,
            "S1",
            Category::Refusal,
            "refusal",
            20,
        ),
        (
            "function f using v(): Boolean pure { null }",
            "unsupported_construct",
            SpineStage::Forms,
            "S2",
            Category::Unsupported,
            "unsupported",
            21,
        ),
    ];
    for (declaration, code, stage, wire_stage, category, wire_category, exit) in cases {
        let text = format!("{HEADER}{declaration}\n");
        let result = parse(
            &ParseRequest {
                source: &source,
                path: "unit.native",
                bytes: text.as_bytes(),
            },
            SpineLimits::default().source,
            &Cancel::new(),
        );
        let StageFailure::Refused(refusal) = result.as_ref().expect_err("the source refuses") else {
            panic!("the source must produce a typed refusal");
        };
        assert_eq!(refusal.code().as_str(), code);
        assert_eq!(refusal.stage(), stage);
        let region = refusal.region().expect("the actual refusal is located");
        assert!(region.start() < region.end());
        assert!(region.end() <= u64::try_from(text.len()).expect("small source"));
        let document = OutcomeDocument::from_parse(&result);
        assert_eq!(document.category(), category);
        assert_eq!(document.category().exit_code(), exit);
        assert_eq!(
            document_json(&document),
            expected(
                json!(wire_stage),
                wire_category,
                json!([diagnostic(refusal)])
            )
        );
    }
}

/// Each real source limit settles as incomplete at S1 with the reached
/// limit's own diagnostic, and mints no output identity.
#[trace("FR-277-AC-1", "FR-286-AC-3")]
#[test]
fn a_driver_serializes_each_reached_parse_limit() {
    let source = identity();
    let defaults = SpineLimits::default().source;
    let cases = [
        ("source_bytes", qsl_cst::Limits { source_bytes: 0, ..defaults }),
        ("tokens", qsl_cst::Limits { tokens: 0, ..defaults }),
        ("nodes", qsl_cst::Limits { nodes: 0, ..defaults }),
        ("work_units", qsl_cst::Limits { work_units: 0, ..defaults }),
    ];
    for (field, limits) in cases {
        let result = parse(
            &ParseRequest {
                source: &source,
                path: "unit.native",
                bytes: FIXTURE,
            },
            limits,
            &Cancel::new(),
        );
        let StageFailure::Limit(limit) = result.as_ref().expect_err("zero denies a charge") else {
            panic!("{field} must produce a reached limit");
        };
        assert_eq!(limit.configured_bound(), 0, "{field}");
        assert!(limit.actual() > 0, "{field}");
        let refusal = CompileRefusal::Limit(limit.clone());
        let document = OutcomeDocument::from_parse(&result);
        assert_eq!(document.category(), Category::Incomplete);
        assert_eq!(document.category().exit_code(), 22);
        assert_eq!(
            document_json(&document),
            expected(json!("S1"), "incomplete", json!([diagnostic(&refusal)])),
            "{field}"
        );
    }
}

/// Actual parse calls with requested or deadline cancellation have no
/// reached stage and preserve the cancel cause.
#[trace("FR-276-AC-1", "FR-286-AC-5", "FR-286-AC-3")]
#[test]
fn a_driver_serializes_requested_and_deadline_parse_cancellation() {
    let source = identity();
    for (cause, wire_cause) in [
        (CancelCause::Requested, "requested"),
        (CancelCause::Deadline, "deadline"),
    ] {
        let cancel = Cancel::new();
        cancel.cancel(cause);
        let result = parse(
            &ParseRequest {
                source: &source,
                path: "unit.native",
                bytes: FIXTURE,
            },
            SpineLimits::default().source,
            &cancel,
        );
        assert!(matches!(result, Err(StageFailure::Cancelled(actual)) if actual == cause));
        let document = OutcomeDocument::from_parse(&result);
        assert_eq!(document.category(), Category::Incomplete);
        assert_eq!(document.category().exit_code(), 22);
        assert_eq!(
            document_json(&document),
            expected(
                Value::Null,
                "incomplete",
                json!([{
                    "cause": wire_cause,
                    "code": "cancelled",
                    "locus": null,
                    "message": "the operation was cancelled"
                }])
            )
        );
    }
}

/// This constructed fault exercises the adapter's fault arm; it does not
/// claim that a source triggers an internal parse invariant failure.
#[trace("FR-285-AC-3", "FR-286-AC-3")]
#[test]
fn a_driver_serializes_a_constructed_parse_fault_at_no_stage() {
    let fault = InternalFault::new("S2", "constructed-test-invariant");
    let document = OutcomeDocument::from_parse(&Err(StageFailure::Fault(fault)));
    assert_eq!(document.category(), Category::InternalFailure);
    assert_eq!(document.category().exit_code(), 30);
    assert_eq!(
        document_json(&document),
        expected(
            Value::Null,
            "internal-failure",
            json!([{
                "cause": fault.catalog_code().cause(),
                "code": fault.catalog_code().code(),
                "locus": null,
                "message": "internal invariant constructed-test-invariant broken in S2"
            }])
        )
    );
}
