// SPDX-License-Identifier: AGPL-3.0-or-later
//! Source refusals keep their owner-provided fields through public outcomes.

use ix_trace_rs::trace;
use qsl_cst::diagnostic::{self, CompleteDiagnostic};
use qsl_cst::format::FormatRefusal;
use qsl_foundation::diagnostic::StageFailure;
use qsl_foundation::source::{SourceLabel, SourceReadCause, SourceReadError, SourceReadRefusal};
use qsl_foundation::{Source, SourceIdentity};
use qsl_replay::spine::{self, CompileRefusal, ParseRequest, SpineLimits};
use qsl_replay::{Operation, OutcomeDiagnostic, OutcomeDocument};
use quire_exact::Cancel;
use quire_semantic_value::catalog::Category;
use serde_json::{json, Value};

fn document(diagnostic: OutcomeDiagnostic) -> OutcomeDocument {
    OutcomeDocument::new(Operation::Parse, None, Category::Refusal)
        .with_diagnostics(vec![diagnostic])
}

fn diagnostic_value(document: &OutcomeDocument) -> Value {
    let value: Value =
        serde_json::from_slice(&document.to_bytes().expect("typed document encodes"))
            .expect("outcome is JSON");
    assert_eq!(value["diagnostics"].as_array().unwrap().len(), 1);
    value["diagnostics"][0].clone()
}

fn assert_projections(diagnostic: &CompleteDiagnostic, expected: &Value) {
    let direct = OutcomeDiagnostic::from_source_diagnostic(diagnostic);
    assert_eq!(diagnostic_value(&document(direct.clone())), *expected);
    let refusal = CompileRefusal::Source(Box::new(diagnostic.clone()));
    assert_eq!(refusal.cause(), Some(diagnostic.cause.as_str()));
    assert_eq!(OutcomeDiagnostic::from_compile_refusal(&refusal), direct);
    let parsed = OutcomeDocument::from_parse(&Err(StageFailure::Refused(Box::new(refusal))));
    assert_eq!(diagnostic_value(&parsed), *expected);
    let formatted = OutcomeDocument::from_format(&Err(FormatRefusal::DiagnosedSource(Box::new(
        diagnostic.clone(),
    ))));
    assert_eq!(diagnostic_value(&formatted), *expected);
}

#[test]
#[trace("TC-770", "FR-286-AC-15")]
fn pre_admission_blank_labels_retain_the_typed_label_without_a_locus() {
    for blank in ["", " \t", "\u{3000}"] {
        for (authority, identity, label, spelling) in [
            (blank, "document", SourceLabel::Authority, "authority"),
            ("issuer", blank, SourceLabel::Identity, "identity"),
            (blank, blank, SourceLabel::Authority, "authority"),
        ] {
            let source = SourceIdentity::new(authority, identity);
            assert_eq!(source.first_blank_label(), Some(label));
            let diagnostic = diagnostic::source_refusal(
                SourceReadRefusal {
                    cause: SourceReadCause::BlankLabel { label },
                    error: SourceReadError {
                        source,
                        path: "unopened.native".into(),
                        region: None,
                        message: "exact preflight detail: λ".into(),
                    },
                },
                17,
            );
            assert_projections(
                &diagnostic,
                &json!({
                    "cause": "blank-label",
                    "code": "invalid_source_identity",
                    "locus": null,
                    "message": "exact preflight detail: λ",
                    "label": spelling
                }),
            );
        }
    }
    assert_eq!(
        SourceIdentity::new("\u{200b}", "document").first_blank_label(),
        None
    );
}

#[test]
#[trace("TC-770", "FR-286-AC-15")]
fn other_source_causes_omit_label_and_preserve_the_original_region_and_detail() {
    let source = SourceIdentity::new("issuer", "document");
    let cases = [
        (
            SourceReadCause::EmptyPath,
            "invalid_source_identity",
            "empty-path",
            None,
        ),
        (
            SourceReadCause::ByteBudget,
            "stage_limit_exceeded",
            "input-bytes-exceeded",
            None,
        ),
        (
            SourceReadCause::ReferenceInvariant,
            "runtime_invariant",
            "established-invariant-broken",
            None,
        ),
        (
            SourceReadCause::InvalidUtf8,
            "invalid_utf8",
            "invalid_utf8",
            Some(b"ab\xffc".as_slice()),
        ),
        (
            SourceReadCause::Bom,
            "invalid_syntax",
            "invalid-token",
            Some(b"\xef\xbb\xbfunit".as_slice()),
        ),
        (
            SourceReadCause::Nul,
            "invalid_syntax",
            "invalid-token",
            Some(b"ab\0c".as_slice()),
        ),
    ];
    for (cause, code, tag, bytes) in cases {
        let region = bytes.map(|bytes| {
            Source::read_typed(source.clone(), "located.native", bytes, usize::MAX)
                .expect_err("source bytes refuse")
                .error
                .region
                .expect("byte refusal is located")
        });
        let expected_locus = region.as_ref().map(|region| {
            json!({
                "kind": "region",
                "source": region.source(),
                "start": region.start(),
                "end": region.end()
            })
        });
        let detail = format!("retained detail for {cause:?}");
        let diagnostic = diagnostic::source_refusal(
            SourceReadRefusal {
                cause,
                error: SourceReadError {
                    source: source.clone(),
                    path: "original display path".into(),
                    region,
                    message: detail.clone(),
                },
            },
            19,
        );
        let expected = json!({
            "cause": tag,
            "code": code,
            "locus": expected_locus,
            "message": detail
        });
        assert_projections(&diagnostic, &expected);
        assert!(!expected.as_object().unwrap().contains_key("label"));
    }
    let ordinary = OutcomeDiagnostic::new(None, "cancelled", None, "ordinary diagnostic");
    assert_eq!(
        diagnostic_value(&document(ordinary)),
        json!({"cause": null, "code": "cancelled", "locus": null, "message": "ordinary diagnostic"})
    );
}

#[test]
#[trace("TC-770", "FR-286-AC-15")]
fn actual_source_reads_and_parse_refusals_use_the_same_projection_bytes() {
    for (authority, identity, path, bytes) in [
        ("", "document", "unit.native", b"unread".as_slice()),
        ("issuer", "\u{3000}", "unit.native", b"unread".as_slice()),
        ("issuer", "document", "", b"unread".as_slice()),
        ("issuer", "document", "unit.native", b"ab\xffc".as_slice()),
    ] {
        let source = SourceIdentity::new(authority, identity);
        let limits = SpineLimits::default().source;
        let read = diagnostic::read_source(source.clone(), path, bytes, limits.source_bytes)
            .expect_err("source read refuses");
        let expected = OutcomeDocument::from_parse(&Err(StageFailure::Refused(Box::new(
            CompileRefusal::Source(read),
        ))));
        let result = spine::parse(
            &ParseRequest {
                source: &source,
                path,
                bytes,
            },
            limits,
            &Cancel::new(),
        );
        assert!(
            matches!(&result, Err(StageFailure::Refused(refusal)) if matches!(refusal.as_ref(), CompileRefusal::Source(_)))
        );
        let actual = OutcomeDocument::from_parse(&result);
        assert_eq!(actual.to_bytes().unwrap(), expected.to_bytes().unwrap());
        assert_eq!(actual.category(), Category::Refusal);
        assert_eq!(actual.category().exit_code(), 20);
    }
}
