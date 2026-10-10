// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-003, FR-286, ADR-029 OP-1/CB-4: the public format outcome adapter
//! preserves typed diagnostics and leaves formatted UTF-8 text in its value.

use ix_trace_rs::trace;
use qsl_cst::format::{format_with_limits, FormatLimits, FormatRefusal};
use qsl_cst::{CompleteCause, CompleteDiagnostic, ParsedSource};
use qsl_foundation::diagnostic::{LimitKind, Staged};
use qsl_foundation::{Code, Phase, SourceIdentity, SyntaxLimit};
use qsl_replay::{Category, OutcomeDocument};
use serde_json::{json, Value};

const HEADER: &str =
    "language \"ix:native\" edition \"1-draft\";\nprofile v = \"quire.value.complete/v1\";\n";
const FIXTURE: &str = include_str!("../../../tests/fixtures/spine-compile.native");

fn parsed(text: &str) -> ParsedSource {
    qsl_cst::parse(
        SourceIdentity::new("agent-ix", "test:format-outcome", "fixture", "fixture:1"),
        "unit.native",
        text.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("the source reaches a CST")
}

fn document_json(document: &OutcomeDocument) -> Value {
    let bytes = document.to_bytes().expect("the document encodes");
    assert_eq!(
        bytes,
        document.to_bytes().expect("encoding is deterministic")
    );
    serde_json::from_slice(&bytes).expect("the document is JSON")
}

fn expected(category: &str, diagnostics: Value) -> Value {
    json!({
        "format": "quire-outcome/1",
        "operation": "format",
        "last_stage": "S2",
        "category": category,
        "items": [],
        "diagnostics": diagnostics,
        "artifacts": [],
        "result": null
    })
}

fn diagnostic_json(diagnostic: &CompleteDiagnostic) -> Value {
    let locus = diagnostic.region.as_ref().map(|region| {
        json!({
            "kind": "region",
            "source": region.source(),
            "start": region.start(),
            "end": region.end()
        })
    });
    json!({
        "cause": diagnostic.cause.as_str(),
        "code": diagnostic.code.as_str(),
        "locus": locus,
        "message": diagnostic.message
    })
}

fn assert_refusal(
    result: &Result<Staged<String>, FormatRefusal>,
    category: Category,
    wire_category: &str,
    exit: u8,
) {
    let diagnostic = result
        .as_ref()
        .expect_err("no formatted output")
        .diagnostic();
    assert!(!diagnostic.message.is_empty());
    let region = diagnostic
        .region
        .as_ref()
        .expect("the producer supplied a region");
    assert!(region.start() < region.end());
    let before = diagnostic.clone();
    let document = OutcomeDocument::from_format(result);
    assert_eq!(document.category(), category);
    assert_eq!(document.category().exit_code(), exit);
    assert_eq!(
        document_json(&document),
        expected(wire_category, json!([diagnostic_json(diagnostic)]))
    );
    assert_eq!(result.as_ref().unwrap_err().diagnostic(), &before);
}

#[trace("FR-003-AC-1", "FR-003-AC-2", "FR-003-AC-3", "FR-286-AC-3")]
#[test]
fn a_driver_serializes_format_success_at_s2_without_text_payload() {
    let text = format!("{FIXTURE}\n// café λ\n");
    let source = parsed(&text);
    assert!(source.is_admissible());
    let limits = FormatLimits::default();
    let result = format_with_limits(&source, limits).map(Staged::new);
    let output = result
        .as_ref()
        .expect("the admitted fixture formats")
        .value();
    assert_eq!(output, &format_with_limits(&source, limits).unwrap());
    assert!(output.contains("// café λ"));
    let reparsed = parsed(output);
    assert!(reparsed.is_admissible());
    let spellings = |source: &ParsedSource| -> Vec<Vec<u8>> {
        source
            .cst()
            .tokens()
            .iter()
            .filter(|token| token.class() != qsl_cst::TokenClass::Whitespace)
            .map(|token| token.spelling().to_vec())
            .collect()
    };
    assert_eq!(spellings(&source), spellings(&reparsed));
    assert_eq!(&format_with_limits(&reparsed, limits).unwrap(), output);
    let document = OutcomeDocument::from_format(&result);
    assert_eq!(document.category(), Category::Success);
    assert_eq!(document.category().exit_code(), 0);
    let json = document_json(&document);
    assert_eq!(json.as_object().unwrap().len(), 8);
    assert_eq!(json, expected("success", json!([])));
    let borrowed_text = output.clone();
    assert_eq!(result.unwrap().into_value(), borrowed_text);
}

#[trace("FR-003-AC-8", "FR-286-AC-3")]
#[test]
fn a_driver_serializes_recovering_format_refusal_with_original_locus() {
    let source = parsed(&format!("{HEADER}record Broken {{ value: Integer }}"));
    assert!(!source.cst().recoveries().is_empty());
    let first = source.diagnostics()[0].clone();
    let result = format_with_limits(&source, FormatLimits::default()).map(Staged::new);
    let FormatRefusal::RecoveringCst(diagnostic) = result.as_ref().unwrap_err() else {
        panic!("the recovering CST must retain its first diagnostic");
    };
    assert_eq!(diagnostic.as_ref(), &first);
    assert_eq!(first.code, Code::InvalidSyntax);
    assert_refusal(&result, Category::Refusal, "refusal", 20);
}

/// Selection diagnostics are supplied explicitly to an admitted CST; these
/// controls do not claim that the ordinary parse invents feature refusals.
#[trace("FR-003-AC-8", "FR-286-AC-3")]
#[test]
fn a_driver_serializes_diagnosed_format_refusals_by_catalogue_category() {
    for (code, cause, category, wire_category, exit) in [
        (
            Code::UnknownProfile,
            CompleteCause::UnsupportedSelection,
            Category::Refusal,
            "refusal",
            20,
        ),
        (
            Code::UnknownRequiredFeature,
            CompleteCause::UnsupportedFeature,
            Category::Unsupported,
            "unsupported",
            21,
        ),
    ] {
        let mut source = parsed(FIXTURE);
        assert!(source.is_admissible());
        let diagnostic = qsl_cst::diagnostic::error(
            source.source(),
            code,
            cause,
            Phase::Profile,
            0,
            "language".len(),
            "the selected catalog does not admit this profile or feature",
        );
        source.prepend_diagnostic(*diagnostic.clone());
        assert!(source.cst().recoveries().is_empty());
        let result = format_with_limits(&source, FormatLimits::default()).map(Staged::new);
        let FormatRefusal::DiagnosedSource(actual) = result.as_ref().unwrap_err() else {
            panic!("the diagnosed CST must retain its first diagnostic");
        };
        assert_eq!(actual, &diagnostic);
        assert_refusal(&result, category, wire_category, exit);
    }
}

#[trace("FR-003-AC-4", "FR-003-AC-5", "FR-286-AC-3")]
#[test]
fn a_driver_serializes_format_output_limit_with_original_cause() {
    let source = parsed(FIXTURE);
    assert!(source.is_admissible());
    let result = format_with_limits(&source, FormatLimits::default().with_output_bytes(0))
        .map(Staged::new);
    let FormatRefusal::OutputBudgetExhausted(diagnostic) = result.as_ref().unwrap_err() else {
        panic!("zero output bytes must exhaust the output budget");
    };
    assert_eq!(diagnostic.code, Code::StageLimitExceeded);
    assert_eq!(
        diagnostic.cause,
        CompleteCause::StageLimit(LimitKind::InputBytes)
    );
    assert_eq!(
        diagnostic.limit(),
        Some(SyntaxLimit::SourceBytes { bound: 0 })
    );
    assert!(diagnostic.message.contains("0 bytes"));
    assert!(diagnostic.message.contains("format_with_limit"));
    assert_refusal(&result, Category::Incomplete, "incomplete", 22);
}

/// This constructed record tests conversion, not a naturally reachable fault
/// in the formatter's immutable admitted CST.
#[trace("FR-286-AC-3")]
#[test]
fn a_driver_serializes_a_constructed_format_invariant_failure() {
    let source = parsed(FIXTURE);
    let diagnostic = qsl_cst::diagnostic::error(
        source.source(),
        Code::RuntimeInvariant,
        CompleteCause::EstablishedInvariantBroken,
        Phase::Format,
        0,
        "language".len(),
        "constructed format invariant detail",
    );
    assert_eq!(diagnostic.code, Code::RuntimeInvariant);
    assert_eq!(diagnostic.cause, CompleteCause::EstablishedInvariantBroken);
    assert!(!diagnostic.message.is_empty());
    let region = diagnostic.region.as_ref().expect("the actual source span");
    assert!(region.start() < region.end());
    let original = *diagnostic.clone();
    let result = Err(FormatRefusal::BrokenInvariant(diagnostic));
    let document = OutcomeDocument::from_format(&result);
    assert_eq!(document.category(), Category::InternalFailure);
    assert_eq!(document.category().exit_code(), 30);
    let json = document_json(&document);
    assert_eq!(json["last_stage"], Value::Null);
    assert_eq!(json.as_object().unwrap().len(), 8);
    assert_eq!(
        json,
        json!({
            "format": "quire-outcome/1",
            "operation": "format",
            "last_stage": null,
            "category": "internal-failure",
            "items": [],
            "diagnostics": [diagnostic_json(&original)],
            "artifacts": [],
            "result": null
        })
    );
    assert_eq!(result.as_ref().unwrap_err().diagnostic(), &original);
}
