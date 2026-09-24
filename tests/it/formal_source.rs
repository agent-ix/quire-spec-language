// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-014: exact native/IR source correspondence through actual public types.

use ix_trace_rs::trace;
use qsl_foundation::{
    source::MAX_SOURCE_BYTES, ByteDigest, Code, Phase, Source, SourceIdentity, Span,
};
use quire_contract_ir as ir;
use quire_spec_language::formal_source::{FormalSource, FormalSourceError};

fn source(text: &str) -> Source {
    Source::read_verified(
        SourceIdentity {
            authority: "test".into(),
            identity: "ix://example/source".into(),
            revision_namespace: "test".into(),
            revision: "draft:alpha".into(),
        },
        "native/source",
        text.as_bytes(),
        ByteDigest::of(text.as_bytes()),
        MAX_SOURCE_BYTES,
    )
    .unwrap()
}

fn identity(document: &str, revision: u64) -> ir::SourceIdentity {
    ir::SourceIdentity::new(
        ir::SourceDocumentId::new(document).unwrap(),
        ir::SourceRevision::new(revision).unwrap(),
    )
}

fn binding(source: &Source) -> FormalSource {
    FormalSource::new(source.clone(), identity("NativeClause", 91))
}

fn coordinates(location: &ir::SourceLocation) -> (u64, u32, u32) {
    (location.byte_offset(), location.line(), location.column())
}

fn assert_refusal<T: std::fmt::Debug>(result: Result<T, Box<FormalSourceError>>, source: &Source) {
    let diagnostic = result.unwrap_err();
    assert_eq!(diagnostic.diagnostic.code, Code::InvalidSourceMap);
    assert_eq!(diagnostic.diagnostic.phase, Phase::SourceMap);
    assert_eq!(&diagnostic.diagnostic.source, source.identity());
    assert_eq!(diagnostic.diagnostic.path, source.path());
    assert_eq!(
        diagnostic.diagnostic.span,
        source.locate(Span { start: 0, end: 0 }).unwrap()
    );
    assert!(diagnostic.upstream.is_none());
    assert!(!diagnostic.diagnostic.is_incomplete());
}

#[test]
#[trace("TC-035", "FR-014-AC-1")]
fn tc_035_retains_explicit_source_assignment() {
    for revision in [1, 91, u64::MAX] {
        let source = source("invariant");
        let expected = identity("NativeClause", revision);
        let binding = FormalSource::new(source, expected.clone());
        assert_eq!(binding.identity(), &expected);
        assert_eq!(binding.source().identity().identity, "ix://example/source");
        assert_eq!(binding.source().identity().revision, "draft:alpha");
        assert_eq!(binding.source().text(), "invariant");
        assert_eq!(binding.source().path(), "native/source");
        assert_eq!(binding.source().digest(), ByteDigest::of(b"invariant"));
    }
    assert!(ir::SourceRevision::new(0).is_err());
    assert!(ir::SourceDocumentId::new("ix://invalid/ir-document").is_err());
}

#[test]
#[trace("TC-036", "FR-014-AC-2")]
fn tc_036_matches_independent_coordinates_and_source_boundary() {
    let source = source("a\r\né😀\nz");
    let binding = binding(&source);
    let expected = [
        (0, 1, 1),
        (1, 1, 2),
        (2, 1, 3),
        (3, 2, 1),
        (5, 2, 2),
        (9, 2, 3),
        (10, 3, 1),
        (11, 3, 2),
    ];
    for &(byte, line, column) in &expected {
        let at = usize::try_from(byte).unwrap();
        let mapped = binding.to_ir(&source, Span { start: at, end: at }).unwrap();
        assert_eq!(mapped.source(), binding.identity());
        assert_eq!(coordinates(mapped.start()), (byte, line, column));
        assert_eq!(coordinates(mapped.end()), (byte, line, column));
    }
    for pair in expected.windows(2) {
        let mapped = binding
            .to_ir(
                &source,
                Span {
                    start: usize::try_from(pair[0].0).unwrap(),
                    end: usize::try_from(pair[1].0).unwrap(),
                },
            )
            .unwrap();
        assert_eq!(coordinates(mapped.start()), pair[0]);
        assert_eq!(coordinates(mapped.end()), pair[1]);
    }
    let empty = self::source("");
    let mapped = self::binding(&empty)
        .to_ir(&empty, Span { start: 0, end: 0 })
        .unwrap();
    assert_eq!(coordinates(mapped.start()), (0, 1, 1));
    assert_eq!(coordinates(mapped.end()), (0, 1, 1));
    let full = self::source(&"a".repeat(MAX_SOURCE_BYTES));
    let mapped = self::binding(&full)
        .to_ir(
            &full,
            Span {
                start: MAX_SOURCE_BYTES,
                end: MAX_SOURCE_BYTES,
            },
        )
        .unwrap();
    assert_eq!(coordinates(mapped.end()), (1_048_576, 1, 1_048_577));
    let oversized = vec![b'a'; MAX_SOURCE_BYTES + 1];
    let error = Source::read(
        full.identity().clone(),
        full.path(),
        &oversized,
        MAX_SOURCE_BYTES,
    )
    .unwrap_err();
    // QSL-236: the source's own byte ceiling is a `SyntaxLimit` kind the
    // catalog admits, so it now reports `stage_limit_exceeded`.
    assert_eq!(error.code, Code::StageLimitExceeded);
    assert_eq!(error.phase, Phase::Source);
}

#[test]
#[trace("TC-037", "FR-014-AC-3")]
fn tc_037_refuses_each_foreign_native_binding_dimension() {
    let source = source("abc");
    let binding = binding(&source);
    let span = Span { start: 1, end: 2 };
    let expected = binding.to_ir(&source, span).unwrap();
    for changed in 0..4 {
        let mut labels = source.identity().clone();
        let mut path = source.path();
        let mut bytes = source.text().as_bytes();
        match changed {
            0 => labels.identity.push('x'),
            1 => labels.revision.push('x'),
            2 => path = "foreign/path",
            3 => bytes = b"axc",
            _ => unreachable!(),
        }
        let foreign = Source::read(labels, path, bytes, MAX_SOURCE_BYTES).unwrap();
        assert_refusal(binding.to_ir(&foreign, span), &source);
        assert_eq!(binding.to_ir(&source, span).unwrap(), expected);
        let reloaded = self::source("abc");
        assert_eq!(binding.to_ir(&reloaded, span).unwrap(), expected);
    }
}

fn formal_span(
    identity: &ir::SourceIdentity,
    start: (u64, u32, u32),
    end: (u64, u32, u32),
) -> ir::SourceSpan {
    let location = |(byte, line, column)| {
        ir::SourceLocation::new(identity.clone(), line, column, byte).unwrap()
    };
    ir::SourceSpan::new(location(start), location(end)).unwrap()
}

#[test]
#[trace("TC-038", "FR-014-AC-4")]
fn tc_038_refuses_false_constructor_valid_formal_coordinates() {
    let source = source("a\r\né😀\nz");
    let binding = binding(&source);
    let id = binding.identity();
    let start = (0, 1, 1);
    let end = (11, 3, 2);
    let mut invalid = vec![
        formal_span(&identity("Foreign", 91), start, end),
        formal_span(&identity("NativeClause", 92), start, end),
        formal_span(id, (0, 2, 1), end),
        formal_span(id, (0, 1, 2), end),
        formal_span(id, start, (11, 4, 2)),
        formal_span(id, start, (11, 3, 3)),
    ];
    for point in [(4, 2, 2), (12, 3, 3), (u64::MAX, 3, 3)] {
        invalid.push(formal_span(id, point, point));
    }
    for wrong in invalid {
        assert_refusal(binding.to_native(&wrong), &source);
        assert_eq!(
            binding.to_native(&formal_span(id, start, end)).unwrap(),
            Span { start: 0, end: 11 }
        );
        assert_eq!(
            binding
                .to_native(&formal_span(id, (3, 2, 1), (3, 2, 1)))
                .unwrap(),
            Span { start: 3, end: 3 }
        );
    }
}

// Independent linear oracle: never calls the native source index or bridge.
fn oracle(text: &str, at: usize) -> (u64, u32, u32) {
    let (line, column) = text[..at].chars().fold((1, 1), |(line, column), ch| {
        if ch == '\n' {
            (line + 1, 1)
        } else {
            (line, column + 1)
        }
    });
    (u64::try_from(at).unwrap(), line, column)
}

#[test]
#[trace("TC-039", "FR-014-AC-5")]
fn tc_039_generated_spans_match_oracle_and_round_trip() {
    let mut family = vec![String::new()];
    let mut level = vec![String::new()];
    for _ in 0..3 {
        level = level
            .iter()
            .flat_map(|prefix| ['a', '\r', '\n', 'é', '😀'].map(|ch| format!("{prefix}{ch}")))
            .collect();
        family.extend(level.iter().cloned());
    }
    assert_eq!(family.len(), 156);
    for text in family {
        let source = source(&text);
        let binding = binding(&source);
        let mut spans: Vec<_> = (0..=text.len() + 1)
            .flat_map(|start| (0..=text.len() + 1).map(move |end| Span { start, end }))
            .collect();
        for _ in 0..2 {
            for &span in &spans {
                if span.start <= span.end
                    && text.is_char_boundary(span.start)
                    && text.is_char_boundary(span.end)
                {
                    let mapped = binding.to_ir(&source, span).unwrap();
                    assert_eq!(mapped.source(), binding.identity());
                    assert_eq!(coordinates(mapped.start()), oracle(&text, span.start));
                    assert_eq!(coordinates(mapped.end()), oracle(&text, span.end));
                    assert_eq!(binding.to_native(&mapped).unwrap(), span);
                } else {
                    assert_refusal(binding.to_ir(&source, span), &source);
                }
            }
            spans.reverse();
        }
    }
}
