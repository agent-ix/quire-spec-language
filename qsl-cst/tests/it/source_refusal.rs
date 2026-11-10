// SPDX-License-Identifier: AGPL-3.0-or-later
//! Public projection of source refusals before source admission.

use ix_trace_rs::trace;
use qsl_cst::diagnostic::{self, CompleteCause, CompleteCode, HostCause};
use qsl_foundation::diagnostic::LimitKind;
use qsl_foundation::source::{SourceLabel, SourceReadCause, SourceReadError, SourceReadRefusal};
use qsl_foundation::{Phase, Source, SourceIdentity, SyntaxLimit};

#[test]
#[trace("TC-424", "FR-001-AC-13")]
fn preflight_identity_refusals_need_no_bytes_or_admitted_source() {
    for blank in ["", " ", "\u{3000}"] {
        for (authority, identity, label) in [
            (blank, "document", SourceLabel::Authority),
            ("issuer", blank, SourceLabel::Identity),
            (blank, blank, SourceLabel::Authority),
        ] {
            let source = SourceIdentity::new(authority, identity);
            let found = source.first_blank_label().expect("blank label");
            assert_eq!(found, label);
            let refusal = diagnostic::source_refusal(
                SourceReadRefusal {
                    cause: SourceReadCause::BlankLabel { label: found },
                    error: SourceReadError {
                        source: source.clone(),
                        path: "unopened.native".into(),
                        region: None,
                        message: "preflight detail".into(),
                    },
                },
                17,
            );
            assert_eq!(refusal.source, source);
            assert_eq!(refusal.region, None);
            let fields = refusal
                .cause
                .identity_cause()
                .expect("identity cause")
                .identity_fields()
                .expect("identity fields");
            assert_eq!(fields.cause, "blank-label");
            assert_eq!(fields.label, Some(label.as_str()));
        }
    }
    assert_eq!(
        SourceIdentity::new("\u{200b}", "document").first_blank_label(),
        None
    );
    let refusal = diagnostic::source_refusal(
        SourceReadRefusal {
            cause: SourceReadCause::EmptyPath,
            error: SourceReadError {
                source: SourceIdentity::new("issuer", "document"),
                path: String::new(),
                region: None,
                message: "empty locator".into(),
            },
        },
        17,
    );
    let fields = refusal
        .cause
        .identity_cause()
        .expect("identity cause")
        .identity_fields()
        .expect("identity fields");
    assert_eq!(fields.cause, "empty-path");
    assert_eq!(fields.label, None);
    assert_eq!(refusal.region, None);
}

#[test]
#[trace("TC-424", "FR-001-AC-14", "FR-001-AC-15")]
fn projection_preserves_each_cause_and_its_original_context() {
    let source = SourceIdentity::new("distinct issuer", "distinct document");
    let cases = [
        (
            SourceReadCause::BlankLabel {
                label: SourceLabel::Identity,
            },
            CompleteCode::InvalidSourceIdentity,
            CompleteCause::Host(HostCause::BlankLabel {
                label: SourceLabel::Identity,
            }),
            None,
        ),
        (
            SourceReadCause::EmptyPath,
            CompleteCode::InvalidSourceIdentity,
            CompleteCause::Host(HostCause::EmptyPath),
            None,
        ),
        (
            SourceReadCause::ReferenceInvariant,
            CompleteCode::RuntimeInvariant,
            CompleteCause::EstablishedInvariantBroken,
            None,
        ),
        (
            SourceReadCause::ByteBudget,
            CompleteCode::StageLimitExceeded,
            CompleteCause::StageLimit(LimitKind::InputBytes),
            None,
        ),
        (
            SourceReadCause::InvalidUtf8,
            CompleteCode::InvalidUtf8,
            CompleteCause::Host(HostCause::InvalidUtf8),
            Some(b"a\xffb".as_slice()),
        ),
        (
            SourceReadCause::Bom,
            CompleteCode::InvalidSyntax,
            CompleteCause::InvalidToken,
            Some(b"\xef\xbb\xbftext".as_slice()),
        ),
        (
            SourceReadCause::Nul,
            CompleteCode::InvalidSyntax,
            CompleteCause::InvalidToken,
            Some(b"ab\0c".as_slice()),
        ),
    ];
    for ceiling in [4, 2_097_152] {
        for (cause, code, complete_cause, bytes) in cases {
            let region = bytes.map(|bytes| {
                Source::read_typed(source.clone(), "original.native", bytes, usize::MAX)
                    .expect_err("invalid bytes")
                    .error
                    .region
                    .expect("located refusal")
            });
            let detail = format!("distinct human detail for {cause:?}");
            let projected = diagnostic::source_refusal(
                SourceReadRefusal {
                    cause,
                    error: SourceReadError {
                        source: source.clone(),
                        path: "distinct display path".into(),
                        region: region.clone(),
                        message: detail.clone(),
                    },
                },
                ceiling,
            );
            assert_eq!(projected.code, code);
            assert_eq!(projected.cause, complete_cause);
            assert_eq!(projected.source, source);
            assert_eq!(projected.path, "distinct display path");
            assert_eq!(projected.message, detail);
            assert_eq!(projected.region, region);
            assert_eq!(projected.phase, Phase::Source);
            assert!(projected.related.is_empty());
            assert_eq!(
                projected.limit(),
                if cause == SourceReadCause::ByteBudget {
                    Some(SyntaxLimit::SourceBytes { bound: ceiling })
                } else {
                    None
                }
            );
        }
    }
}

#[test]
#[trace("TC-424", "FR-001-AC-15")]
fn read_source_uses_the_public_projection_for_actual_refusals() {
    for ceiling in [4, 2_097_152] {
        let oversized = vec![b'x'; ceiling + 1];
        for (authority, identity, path, bytes) in [
            ("", "document", "input.native", b"ok".as_slice()),
            ("issuer", "document", "", b"ok".as_slice()),
            ("issuer", "document", "input.native", oversized.as_slice()),
            ("issuer", "document", "input.native", b"a\xffb".as_slice()),
            (
                "issuer",
                "document",
                "input.native",
                b"\xef\xbb\xbf".as_slice(),
            ),
            ("issuer", "document", "input.native", b"ab\0c".as_slice()),
        ] {
            let source = SourceIdentity::new(authority, identity);
            let typed = Source::read_typed(source.clone(), path, bytes, ceiling)
                .expect_err("source refusal");
            let direct = diagnostic::source_refusal(*typed, ceiling);
            let through_reader =
                diagnostic::read_source(source, path, bytes, ceiling).expect_err("source refusal");
            assert_eq!(through_reader, direct);
        }
    }
}
