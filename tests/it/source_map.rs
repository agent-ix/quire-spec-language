// SPDX-License-Identifier: AGPL-3.0-or-later
//! Source-map layout and byte/segment lookup over parsed source text.
use ix_trace_rs::trace;
use qsl_foundation::source_map::{Layout, Segment, SourceMap};
use qsl_foundation::{ByteDigest, Code, Source, SourceIdentity, Span};
use quire_spec_language::{parse_source, Limits};

fn source(id: &str, text: &str) -> Source {
    Source::read(
        SourceIdentity {
            authority: "test".into(),
            identity: id.into(),
            revision_namespace: "test".into(),
            revision: "fixture:1".into(),
        },
        format!("{id}.txt"),
        text.as_bytes(),
        Limits::default().source_bytes,
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

#[trace("TC-011", "FR-001-AC-1", "FR-001-AC-2", "FR-001-AC-4")]
#[test]
fn exact_bytes_digest_is_checked_before_correspondence() {
    let digest: ByteDigest =
        "sha256:ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
            .parse()
            .unwrap();
    assert_eq!(ByteDigest::of(b"abc"), digest);
    assert_eq!(
        digest.to_string(),
        "sha256:ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    let id = SourceIdentity {
        authority: "test".into(),
        identity: "original".into(),
        revision_namespace: "test".into(),
        revision: "1".into(),
    };
    let original = Source::read_verified(id.clone(), "x", b"abc", digest, 100).unwrap();
    assert_eq!(original.digest(), digest);
    assert_eq!(
        Source::read_verified(id.clone(), "x", b"abd", digest, 100)
            .unwrap_err()
            .code,
        Code::SourceDigestMismatch
    );
    // QSL-236: the source's own byte ceiling is now a stage limit
    // (`stage_limit_exceeded`), a refusal, not incomplete work.
    assert!(!Source::read_verified(id, "x", b"abc", digest, 2)
        .unwrap_err()
        .is_incomplete());
    for invalid in [
        "",
        "sha256:abc",
        "SHA256:ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
        "sha256:BA7816BF8F01CFEA414140DE5DAE2223B00361A396177A9CB410FF61F20015AD",
    ] {
        assert!(invalid.parse::<ByteDigest>().is_err());
    }
}

/// QSL-199: `Source::read` enforces the caller's byte limit as given, with
/// no hidden 1 MiB ceiling underneath it. A limit raised past the 1 MiB
/// default admits a source the default refuses, and one byte past the raised
/// limit refuses `resource_exhausted` naming that limit.
#[trace("TC-011", "FR-001-AC-4")]
#[test]
fn a_caller_raised_byte_limit_is_enforced_as_given() {
    use qsl_foundation::source::MAX_SOURCE_BYTES;
    let id = SourceIdentity {
        authority: "test".into(),
        identity: "raised".into(),
        revision_namespace: "test".into(),
        revision: "1".into(),
    };
    let raised = MAX_SOURCE_BYTES + 16;
    let at_raised = vec![b'a'; raised];
    assert_eq!(
        Source::read(id.clone(), "x", &at_raised, MAX_SOURCE_BYTES)
            .unwrap_err()
            .code,
        // QSL-236: the source's own byte ceiling is a `SyntaxLimit` kind
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

#[trace("TC-014", "TC-184", "FR-004-AC-1", "FR-134-AC-1")]
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

#[trace("TC-014", "TC-184", "FR-004-AC-1", "FR-004-AC-4", "FR-134-AC-2")]
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

/// ADR-013 C-21 (QSL-233): an embedded-body span maps to document regions
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

#[trace("TC-014", "TC-184", "FR-004-AC-2", "FR-134-AC-3")]
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

#[trace("TC-014", "TC-184", "FR-004-AC-3", "FR-134-AC-3")]
#[test]
fn original_and_body_identity_revision_and_bytes_must_match() {
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
    let revision = Source::read(
        SourceIdentity {
            authority: "test".into(),
            identity: "body".into(),
            revision_namespace: "test".into(),
            revision: "fixture:2".into(),
        },
        "body.txt",
        b"abc",
        100,
    )
    .unwrap();
    assert!(map.map_span(&revision, Span { start: 0, end: 1 }).is_err());
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
fn parser_refusal_maps_back_through_a_real_indented_body() {
    let text = include_str!("../fixtures/parent.native")
        .replace("not reaches(self, self, parent)", "always(true)");
    let body_text = text.trim_end_matches('\n');
    let body = source("body", body_text);
    let mut document = String::from("# Original café\r\n```native\r\n");
    let region_start = document.len();
    let mut offset = 0;
    let mut segments = Vec::new();
    for line in body_text.split_inclusive('\n') {
        document.push_str("  ");
        let original_start = document.len();
        let content = line.trim_end_matches('\n');
        document.push_str(content);
        if !content.is_empty() {
            segments.push(segment(offset, original_start, content.len()));
        }
        offset += content.len();
        document.push('\r');
        if line.ends_with('\n') {
            segments.push(segment(offset, document.len(), 1));
            offset += 1;
        }
        document.push('\n');
    }
    let region_end = document.len();
    document.push_str("```\r\n");
    let original = source("original", &document);
    let map = SourceMap::verify(
        original.clone(),
        body.clone(),
        Span {
            start: region_start,
            end: region_end,
        },
        segments,
        layout(),
        100,
    )
    .unwrap();
    let error = parse_source(body.clone(), Limits::default()).unwrap_err();
    assert_eq!(error.code, Code::UnsupportedConstruct);
    let mapped = map
        .map_span(
            &body,
            Span {
                start: error.span.start.byte,
                end: error.span.end.byte,
            },
        )
        .unwrap();
    assert_eq!(bytes(&original, &mapped), "always");
    assert_eq!(mapped[0].start.byte, document.find("always").unwrap());
    assert_eq!(mapped[0].start.line, error.span.start.line + 2);
    assert_eq!(mapped[0].start.column, error.span.start.column + 2);
}

#[trace("TC-014")]
#[test]
fn extracted_bytes_need_a_distinct_identity_and_respect_parse_limits() {
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
    let body = source("body", "true");
    // QSL-236: the source's own byte ceiling is now a stage limit
    // (`stage_limit_exceeded`), a refusal, not incomplete work.
    assert!(!parse_source(
        body,
        Limits {
            source_bytes: 1,
            ..Limits::default()
        }
    )
    .unwrap_err()
    .is_incomplete());
}
