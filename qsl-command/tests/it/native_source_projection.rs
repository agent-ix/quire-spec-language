// SPDX-License-Identifier: AGPL-3.0-or-later
//! Two projections of one typed refusal without repeating source admission.

use ix_trace_rs::trace;
use qsl_foundation::source::provenance::{RawSourceRef, SourceRegion};
use qsl_foundation::source::{SourceLabel, SourceReadCause, SourceReadError, SourceReadRefusal};
use qsl_foundation::{Code, Phase, Source, SourceIdentity, SyntaxLimit};

#[test]
#[trace("TC-424", "FR-001-AC-16")]
fn native_projection_borrows_all_typed_causes_and_preserves_the_original_for_cst() {
    for ceiling in [4, 2_097_152] {
        for (cause, code) in [
            (
                SourceReadCause::BlankLabel {
                    label: SourceLabel::Authority,
                },
                Code::InvalidSourceIdentity,
            ),
            (
                SourceReadCause::BlankLabel {
                    label: SourceLabel::Identity,
                },
                Code::InvalidSourceIdentity,
            ),
            (SourceReadCause::EmptyPath, Code::InvalidSourceIdentity),
            (SourceReadCause::ReferenceInvariant, Code::RuntimeInvariant),
            (SourceReadCause::ByteBudget, Code::StageLimitExceeded),
            (SourceReadCause::InvalidUtf8, Code::InvalidUtf8),
            (SourceReadCause::Bom, Code::InvalidSyntax),
            (SourceReadCause::Nul, Code::InvalidSyntax),
        ] {
            let source = SourceIdentity::new("offered issuer", "offered identity");
            let message = format!("original detail {cause:?}");
            let refusal = SourceReadRefusal {
                cause,
                error: SourceReadError {
                    source: source.clone(),
                    path: "original.native".into(),
                    region: None,
                    message: message.clone(),
                },
            };
            let native = qsl_foundation::diagnostic::source_refusal(&refusal, b"ok", ceiling);
            assert_eq!(native.code, code);
            assert_eq!(native.phase, Phase::Source);
            assert_eq!(native.source, source);
            assert_eq!(native.path, "original.native");
            assert_eq!(native.message, message);
            assert_eq!(native.span.start.byte, 0);
            assert_eq!(native.span.start.line, 1);
            assert_eq!(native.span.start.column, 1);
            assert_eq!(native.span.start, native.span.end);
            assert_eq!(
                native.identity_cause(),
                match cause {
                    SourceReadCause::BlankLabel { .. } | SourceReadCause::EmptyPath => Some(cause),
                    _ => None,
                }
            );
            assert_eq!(
                native.limit(),
                if cause == SourceReadCause::ByteBudget {
                    Some(SyntaxLimit::SourceBytes { bound: ceiling })
                } else {
                    None
                }
            );
            assert_eq!(refusal.cause, cause);
            assert_eq!(refusal.error.region, None);
            let complete = qsl_cst::diagnostic::source_refusal(refusal, ceiling);
            assert_eq!(complete.code, code);
            assert_eq!(complete.region, None);
            assert_eq!(complete.message, message);
            assert_eq!(complete.limit(), native.limit());
        }
    }
}

#[test]
#[trace("TC-424", "FR-001-AC-16")]
fn offered_byte_coordinates_and_unrenderable_fallback_do_not_change_the_typed_region() {
    let identity = SourceIdentity::new("issuer", "unit");
    for (bytes, start, end, column, end_column) in [
        (b"a\xffb".as_slice(), 1, 1, 2, 2),
        (b"\xef\xbb\xbftext".as_slice(), 0, 3, 1, 2),
        (b"ab\0c".as_slice(), 2, 3, 3, 4),
    ] {
        let refusal =
            Source::read_typed(identity.clone(), "offered.native", bytes, 128).unwrap_err();
        let original = refusal.error.region.clone();
        let native = qsl_foundation::diagnostic::source_refusal(&refusal, bytes, 128);
        assert_eq!(native.span.start.byte, start);
        assert_eq!(native.span.end.byte, end);
        assert_eq!(native.span.start.line, 1);
        assert_eq!(native.span.start.column, column);
        assert_eq!(native.span.end.line, 1);
        assert_eq!(native.span.end.column, end_column);
        assert_eq!(refusal.error.region, original);
        let complete = qsl_cst::diagnostic::source_refusal(*refusal, 128);
        assert_eq!(complete.region, original);
    }
    let region = SourceRegion::new(RawSourceRef::new("issuer", "unit").unwrap(), 90, 99).unwrap();
    let refusal = SourceReadRefusal {
        cause: SourceReadCause::Nul,
        error: SourceReadError {
            source: identity,
            path: "offered.native".into(),
            region: Some(region.clone()),
            message: "unrenderable region".into(),
        },
    };
    let native = qsl_foundation::diagnostic::source_refusal(&refusal, b"ok", 128);
    assert_eq!(native.span.start.byte, 0);
    assert_eq!(native.span.start.line, 1);
    assert_eq!(native.span.start.column, 1);
    assert_eq!(native.span.start, native.span.end);
    assert_eq!(refusal.error.region, Some(region.clone()));
    assert_eq!(
        qsl_cst::diagnostic::source_refusal(refusal, 128).region,
        Some(region)
    );
}

#[test]
#[trace("TC-424", "FR-001-AC-16")]
fn source_read_and_direct_native_projection_have_identical_fields() {
    for ceiling in [4, 2_097_152] {
        let too_large = vec![b'x'; ceiling + 1];
        for (authority, identity, path, bytes) in [
            ("", "unit", "offered.native", b"ok".as_slice()),
            ("issuer", "", "offered.native", b"ok".as_slice()),
            ("issuer", "unit", "", b"ok".as_slice()),
            ("issuer", "unit", "offered.native", too_large.as_slice()),
            ("issuer", "unit", "offered.native", b"a\xffb".as_slice()),
            (
                "issuer",
                "unit",
                "offered.native",
                b"\xef\xbb\xbf".as_slice(),
            ),
            ("issuer", "unit", "offered.native", b"ab\0c".as_slice()),
        ] {
            let source = SourceIdentity::new(authority, identity);
            let typed = Source::read_typed(source.clone(), path, bytes, ceiling).unwrap_err();
            let projected = qsl_foundation::diagnostic::source_refusal(&typed, bytes, ceiling);
            let existing = Source::read(source, path, bytes, ceiling).unwrap_err();
            assert_eq!(projected, existing);
        }
    }
}
