// SPDX-License-Identifier: AGPL-3.0-only
//! FR-029: lowering failure context resolved before the compiled source is released.

use crate::formal_source::FormalSource;
use crate::{ByteDigest, LocatedSpan, Source, SourceIdentity, Span};
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
                identity: "test:lowering".into(),
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
}
