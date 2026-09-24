// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-029: lowering failure context resolved before the compiled source is released.

use crate::formal_source::FormalSource;
use qsl_foundation::{ByteDigest, LocatedSpan, Source, SourceIdentity, Span};
use quire_contract_ir as ir;

/// Exact program identity retained by a command failure, without the program text.
#[derive(Debug)]
pub struct ProjectionSource {
    /// Original authored labels.
    pub identity: SourceIdentity,
    /// Exact selected source bytes.
    pub digest: ByteDigest,
    /// Original file operand.
    pub path: String,
    /// Explicit formal document identity.
    pub formal: ir::SourceIdentity,
}

impl From<&FormalSource> for ProjectionSource {
    fn from(source: &FormalSource) -> Self {
        Self {
            identity: source.source().identity().clone(),
            digest: source.source().digest(),
            path: source.source().path().into(),
            formal: source.identity().clone(),
        }
    }
}

/// Source location of a lowering failure; invalid coordinates are not absent coordinates.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum ProjectionLocation {
    /// The lowering failure did not select a source span.
    Absent,
    /// Exact coordinates resolved against the selected source.
    Located(LocatedSpan),
    /// The lowering failure supplied a span that the selected source cannot locate.
    Invalid(Span),
}

impl ProjectionLocation {
    pub(super) fn resolve(source: &Source, span: Option<Span>) -> Self {
        match span {
            None => Self::Absent,
            Some(span) => match source.locate(span) {
                Some(located) => Self::Located(located),
                None => Self::Invalid(span),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;

    #[test]
    #[trace("TC-107", "FR-029-AC-2")]
    fn missing_and_invalid_lowering_spans_remain_distinct() {
        let source = Source::read(
            SourceIdentity {
                authority: "test".into(),
                identity: "test:lowering".into(),
                revision_namespace: "test".into(),
                revision: "1".into(),
            },
            "program.native",
            "é\ntrue".as_bytes(),
            1024,
        )
        .unwrap();
        assert_eq!(
            ProjectionLocation::resolve(&source, None),
            ProjectionLocation::Absent
        );
        for span in [Span { start: 1, end: 2 }, Span { start: 3, end: 99 }] {
            assert_eq!(
                ProjectionLocation::resolve(&source, Some(span)),
                ProjectionLocation::Invalid(span)
            );
        }
        let ProjectionLocation::Located(span) =
            ProjectionLocation::resolve(&source, Some(Span { start: 3, end: 7 }))
        else {
            panic!("valid exact source span must locate");
        };
        assert_eq!(
            (span.start.byte, span.start.line, span.start.column),
            (3, 2, 1)
        );
        assert_eq!((span.end.byte, span.end.line, span.end.column), (7, 2, 5));
    }

    #[test]
    #[trace("TC-107", "FR-029-AC-2")]
    fn lowering_error_wire_keeps_catalog_codes_and_location_status() {
        use crate::command::{RunCause, RunError};
        use crate::lowering::{LoweringCode, LoweringError};
        use crate::package::NativePackageRef;
        let source = Source::read(
            SourceIdentity {
                authority: "test".into(),
                identity: "test:lowering".into(),
                revision_namespace: "test".into(),
                revision: "1".into(),
            },
            "program.native",
            b"true",
            1024,
        )
        .unwrap();
        let formal = FormalSource::new(
            source,
            ir::SourceIdentity::new(
                ir::SourceDocumentId::new("Program").unwrap(),
                ir::SourceRevision::new(1).unwrap(),
            ),
        );
        for (code, expected_code, exit) in [
            (LoweringCode::Unsupported, "unsupported_projection", 21),
            (LoweringCode::ResourceExhausted, "resource_exhausted", 22),
            (LoweringCode::Binding, "projection_binding", 20),
            (
                LoweringCode::InvalidCorrespondence,
                "invalid_projection_correspondence",
                20,
            ),
        ] {
            for (span, expected_status) in [
                (None, "absent"),
                (Some(Span { start: 0, end: 9 }), "invalid"),
            ] {
                let error = RunError {
                    request_digest: Some(ByteDigest::of(b"request")),
                    cause: RunCause::Lowering {
                        target: crate::lowering::ProjectionTarget::BooleanOracleV1,
                        package: NativePackageRef::new(ByteDigest::of(b"package")),
                        program: Box::new((&formal).into()),
                        location: ProjectionLocation::resolve(formal.source(), span),
                        error: Box::new(LoweringError {
                            code,
                            clause: None,
                            source: span,
                            message: "controlled lowering adapter failure".into(),
                            upstream: vec![],
                        }),
                    },
                };
                assert_eq!(error.exit_code(), exit);
                let value = error.value().unwrap();
                assert_eq!(value["stage"], "lower");
                assert_eq!(value["code"], expected_code);
                assert_eq!(
                    qsl_foundation::Code::from_code(expected_code),
                    Some(code.code())
                );
                assert_eq!(value["details"]["source"]["identity"], "test:lowering");
                assert_eq!(
                    value["details"]["source"]["digest"],
                    ByteDigest::of(b"true").to_string()
                );
                assert_eq!(value["details"]["span_status"], expected_status);
                assert!(value["details"]["span"].is_null());
                match span {
                    Some(span) => assert_eq!(
                        value["details"]["unmapped_span"],
                        serde_json::json!({"start":span.start,"end":span.end})
                    ),
                    None => assert!(value["details"].get("unmapped_span").is_none()),
                }
                assert!(value.get("truth").is_none());
            }
        }
    }
}
