// SPDX-License-Identifier: AGPL-3.0-or-later
//! Source-map layout and byte/segment lookup over parsed source text.
use ix_trace_rs::trace;
use qsl_foundation::source::MAX_SOURCE_BYTES;
use qsl_foundation::source_map::{Layout, Segment, SourceMap};
use qsl_foundation::{Code, Source, SourceIdentity, Span};

fn source(id: &str, text: &str) -> Source {
    Source::read(
        SourceIdentity {
            authority: "test".into(),
            identity: id.into(),
        },
        format!("{id}.txt"),
        text.as_bytes(),
        MAX_SOURCE_BYTES,
    )
    .unwrap()
}
fn segment(body_start: usize, original_start: usize, length: usize) -> Segment {
    Segment {
        body: Span {
            start: body_start,
            end: body_start + length,
        },
        original: Span {
            start: original_start,
            end: original_start + length,
        },
    }
}
fn layout() -> Layout {
    Layout {
        strip_indentation: true,
        normalize_crlf: true,
        drop_final_newline: true,
    }
}
fn bytes(source: &Source, locations: &[qsl_foundation::LocatedSpan]) -> String {
    locations
        .iter()
        .map(|s| {
            source
                .slice(Span {
                    start: s.start.byte,
                    end: s.end.byte,
                })
                .unwrap()
        })
        .collect()
}

#[trace("TC-011", "FR-001-AC-4")]
#[test]
fn source_byte_ceiling_is_a_stage_limit() {
    let id = SourceIdentity {
        authority: "test".into(),
        identity: "original".into(),
    };
    Source::read(id.clone(), "x", b"abc", 100).unwrap();
    // The source's own byte ceiling is a stage limit
    // (`stage_limit_exceeded`), incomplete work (ADR-029 CB-4).
    assert!(Source::read(id, "x", b"abc", 2)
        .unwrap_err()
        .is_incomplete());
}

/// `Source::read` enforces the caller's byte limit as given, with
/// no hidden 1 MiB ceiling underneath it. A limit raised past the 1 MiB
/// default admits a source the default refuses, and one byte past the raised
/// limit refuses `resource_exhausted` naming that limit.
#[trace("TC-011", "FR-001-AC-4")]
#[test]
fn a_caller_raised_byte_limit_is_enforced_as_given() {
    let id = SourceIdentity {
        authority: "test".into(),
        identity: "raised".into(),
    };
    let raised = MAX_SOURCE_BYTES + 16;
    let at_raised = vec![b'a'; raised];
    assert_eq!(
        Source::read(id.clone(), "x", &at_raised, MAX_SOURCE_BYTES)
            .unwrap_err()
            .code,
        // The source's own byte ceiling is a `SyntaxLimit` kind
        // the catalog admits, so it now reports `stage_limit_exceeded`.
        Code::StageLimitExceeded,
        "the default limit must refuse a source past it"
    );
    let admitted = Source::read(id.clone(), "x", &at_raised, raised)
        .expect("a caller-raised limit must admit a source at it");
    assert_eq!(admitted.text().len(), raised);

    let past_raised = vec![b'a'; raised + 1];
    let refusal = Source::read(id, "x", &past_raised, raised).unwrap_err();
    assert_eq!(refusal.code, Code::StageLimitExceeded);
    assert!(
        refusal.message.contains(&raised.to_string()),
        "the refusal must name the caller's limit, got {refusal:?}"
    );
}

#[trace("TC-014", "QSpec-TC-184", "FR-004-AC-1", "QSpec-FR-134-AC-1")]
#[test]
fn verbatim_mapping_retains_original_utf8_crlf_locations() {
    let original = source("original", "header😀\r\nα + β\r\nfooter");
    let body = source("body", "α + β");
    let start = original.text().find('α').unwrap();
    let map = SourceMap::verify(
        original.clone(),
        body.clone(),
        Span {
            start,
            end: start + body.text().len(),
        },
        vec![segment(0, start, body.text().len())],
        Layout::default(),
        1,
    )
    .unwrap();
    let beta = body.text().find('β').unwrap();
    let mapped = map
        .map_span(
            &body,
            Span {
                start: beta,
                end: beta + 2,
            },
        )
        .unwrap();
    assert_eq!(bytes(&original, &mapped), "β");
    assert_eq!((mapped[0].start.line, mapped[0].start.column), (2, 5));
    assert_eq!(mapped[0].start.byte, original.text().find('β').unwrap());
}

#[trace(
    "TC-014",
    "QSpec-TC-184",
    "FR-004-AC-1",
    "FR-004-AC-4",
    "QSpec-FR-134-AC-2"
)]
#[test]
fn layout_mapping_returns_discontiguous_exact_regions() {
    let original = source("original", "head😀\r\n  α <= 2\r\n\tβ\r\nend");
    let body = source("body", "α <= 2\nβ");
    let start = original.text().find("  α").unwrap();
    let alpha = original.text().find('α').unwrap();
    let beta = original.text().find('β').unwrap();
    let line = body.text().find('\n').unwrap();
    let region = Span {
        start,
        end: original.text().find("end").unwrap(),
    };
    let segments = vec![
        segment(0, alpha, line),
        segment(line, alpha + line + 1, 1),
        segment(line + 1, beta, 2),
    ];
    let map = SourceMap::verify(
        original.clone(),
        body.clone(),
        region,
        segments.clone(),
        layout(),
        3,
    )
    .unwrap();
    let mapped = map
        .map_span(
            &body,
            Span {
                start: 0,
                end: body.text().len(),
            },
        )
        .unwrap();
    assert_eq!(mapped.len(), 3);
    assert_eq!(bytes(&original, &mapped), body.text());
    assert_eq!((mapped[2].start.line, mapped[2].start.column), (3, 2));
    assert_eq!(map.region(), region);
    assert_eq!(map.segments(), segments);
    assert_eq!(map.layout(), layout());
    assert!(SourceMap::verify(
        original.clone(),
        body.clone(),
        region,
        segments.clone(),
        Layout::default(),
        3
    )
    .is_err());
    let error = SourceMap::verify(original, body, region, segments, layout(), 2).unwrap_err();
    assert!(error.is_incomplete());
}

/// ADR-013 C-21: an embedded-body span maps to document regions
/// that keep the document's own `RawSourceRef`, never the body's, and the
/// discontiguous parts stay separate.
#[trace("TC-014")]
#[test]
fn embedded_body_spans_map_to_regions_under_the_document_reference() {
    let original = source("original", "head😀\r\n  α <= 2\r\n\tβ\r\nend");
    let body = source("body", "α <= 2\nβ");
    let start = original.text().find("  α").unwrap();
    let alpha = original.text().find('α').unwrap();
    let beta = original.text().find('β').unwrap();
    let line = body.text().find('\n').unwrap();
    let region = Span {
        start,
        end: original.text().find("end").unwrap(),
    };
    let map = SourceMap::verify(
        original.clone(),
        body.clone(),
        region,
        vec![
            segment(0, alpha, line),
            segment(line, alpha + line + 1, 1),
            segment(line + 1, beta, 2),
        ],
        layout(),
        3,
    )
    .unwrap();
    let whole = Span {
        start: 0,
        end: body.text().len(),
    };
    let regions = map.map_regions(&body, whole).unwrap();
    assert_eq!(regions.len(), 3);
    for region in &regions {
        assert_eq!(region.source(), original.reference());
        assert_ne!(region.source(), body.reference());
    }
    let located: Vec<_> = regions
        .iter()
        .map(|region| original.render(region).unwrap())
        .collect();
    assert_eq!(located, map.map_span(&body, whole).unwrap());
    assert_eq!(bytes(&original, &located), body.text());
    // A span of another source maps to nothing.
    assert!(map.map_regions(&original, whole).is_none());
}

#[trace("TC-014", "QSpec-TC-184", "FR-004-AC-2", "QSpec-FR-134-AC-3")]
#[test]
fn omitted_keywords_internal_whitespace_and_newlines_refuse() {
    for (original, body, segments) in [
        (
            "true and false",
            "true false",
            vec![segment(0, 0, 5), segment(5, 9, 5)],
        ),
        (
            "not true",
            "nottrue",
            vec![segment(0, 0, 3), segment(3, 4, 4)],
        ),
        ("a\nb", "ab", vec![segment(0, 0, 1), segment(1, 2, 1)]),
        ("a\rb", "ab", vec![segment(0, 0, 1), segment(1, 2, 1)]),
    ] {
        let error = SourceMap::verify(
            source("original", original),
            source("body", body),
            Span {
                start: 0,
                end: original.len(),
            },
            segments,
            layout(),
            10,
        )
        .unwrap_err();
        assert_eq!(error.code, Code::InvalidSourceMap);
    }
}

#[trace("TC-014", "QSpec-TC-184", "FR-004-AC-3", "QSpec-FR-134-AC-3")]
#[test]
fn original_and_body_identity_and_bytes_must_match() {
    let body = source("body", "abc");
    let map = SourceMap::verify(
        source("original", "abc"),
        body.clone(),
        Span { start: 0, end: 3 },
        vec![segment(0, 0, 3)],
        Layout::default(),
        1,
    )
    .unwrap();
    let altered = source("body", "abd");
    assert_eq!(
        map.map_span(&altered, Span { start: 0, end: 1 })
            .unwrap_err()
            .code,
        Code::InvalidSourceMap
    );
    let foreign = source("other", "abc");
    assert!(map.map_span(&foreign, Span { start: 0, end: 1 }).is_err());
    let path = Source::read(body.identity().clone(), "other.txt", b"abc", 100).unwrap();
    assert!(map.map_span(&path, Span { start: 0, end: 1 }).is_err());
}

#[trace("TC-014")]
#[test]
fn boundaries_are_right_biased_and_eof_precedes_trimmed_newline() {
    let original = source("original", "a\n  b\n");
    let body = source("body", "a\nb");
    let map = SourceMap::verify(
        original,
        body.clone(),
        Span { start: 0, end: 6 },
        vec![segment(0, 0, 2), segment(2, 4, 1)],
        layout(),
        2,
    )
    .unwrap();
    assert_eq!(
        map.map_span(&body, Span { start: 2, end: 2 }).unwrap()[0]
            .start
            .byte,
        4
    );
    assert_eq!(
        map.map_span(&body, Span { start: 3, end: 3 }).unwrap()[0]
            .start
            .byte,
        5
    );
    let empty = source("empty", "");
    let map = SourceMap::verify(
        source("original", ""),
        empty.clone(),
        Span { start: 0, end: 0 },
        vec![],
        Layout::default(),
        0,
    )
    .unwrap();
    assert_eq!(
        map.map_span(&empty, Span { start: 0, end: 0 }).unwrap()[0]
            .start
            .byte,
        0
    );
}

#[trace("TC-014")]
#[test]
fn malformed_segment_and_query_ranges_never_panic_or_partially_succeed() {
    let original = source("original", "aé\nb");
    let body = source("body", "aé\nb");
    let length = body.text().len();
    let endpoints = [0, 1, 2, 3, 4, 5, 6, usize::MAX];
    for &a in &endpoints {
        for &b in &endpoints {
            for &c in &endpoints {
                for &d in &endpoints {
                    let segments = vec![Segment {
                        body: Span { start: a, end: b },
                        original: Span { start: c, end: d },
                    }];
                    let result = SourceMap::verify(
                        original.clone(),
                        body.clone(),
                        Span {
                            start: 0,
                            end: length,
                        },
                        segments,
                        Layout::default(),
                        1,
                    );
                    if let Ok(map) = result {
                        assert_eq!(
                            bytes(
                                &original,
                                &map.map_span(
                                    &body,
                                    Span {
                                        start: 0,
                                        end: length
                                    }
                                )
                                .unwrap()
                            ),
                            body.text()
                        );
                    }
                }
            }
        }
    }
    let map = SourceMap::verify(
        original,
        body.clone(),
        Span {
            start: 0,
            end: length,
        },
        vec![segment(0, 0, length)],
        Layout::default(),
        1,
    )
    .unwrap();
    for &start in &endpoints {
        for &end in &endpoints {
            let _ = map.map_span(&body, Span { start, end });
        }
    }
    for segments in [
        vec![segment(0, 0, 3), segment(2, 3, 2)],
        vec![segment(0, 0, 3), segment(3, 0, 2)],
        vec![segment(0, 0, 1), segment(2, 2, 3)],
        vec![segment(0, 0, 0)],
    ] {
        assert!(SourceMap::verify(
            map.original().clone(),
            body.clone(),
            map.region(),
            segments,
            layout(),
            10
        )
        .is_err());
    }
}

#[trace("TC-014")]
#[test]
fn extracted_bytes_need_a_distinct_identity() {
    let original = source("original", "ab");
    let body = source("original", "b");
    assert!(SourceMap::verify(
        original,
        body,
        Span { start: 1, end: 2 },
        vec![segment(0, 1, 1)],
        Layout::default(),
        1
    )
    .is_err());
}
