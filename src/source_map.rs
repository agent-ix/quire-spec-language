// SPDX-License-Identifier: AGPL-3.0-only
//! FR-004: checked body-to-document byte correspondence; no Markdown or wire decoding.
use crate::{Code, Diagnostic, LocatedSpan, Phase, Source, Span};

/// Hard ceiling on selected correspondence segments.
pub const MAX_SEGMENTS: usize = 50_000;

/// Equal byte regions in the extracted body and its original document.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Segment {
    /// Half-open region of the extracted body.
    pub body: Span,
    /// Corresponding half-open region of the original document.
    pub original: Span,
}

/// Only explicitly enabled layout deletions are accepted. No inserted or rewritten bytes.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Layout {
    /// Permit deletion of leading spaces/tabs at original line boundaries.
    pub strip_indentation: bool,
    /// Permit deletion of CR immediately before a retained LF.
    pub normalize_crlf: bool,
    /// Permit deletion of the selected region's final newline.
    pub drop_final_newline: bool,
}

/// Verified immutable correspondence bound to exact original and body sources.
#[derive(Clone, Debug)]
pub struct SourceMap {
    original: Source,
    body: Source,
    region: Span,
    segments: Vec<Segment>,
    layout: Layout,
}

impl SourceMap {
    /// Validate a complete monotonic map against immutable exact source bytes.
    /// Both sources can be loaded with Source::read_verified when consuming pinned artifacts.
    pub fn verify(
        original: Source,
        body: Source,
        region: Span,
        segments: Vec<Segment>,
        layout: Layout,
        segment_limit: usize,
    ) -> Result<Self, Box<Diagnostic>> {
        let fail = |code, message: &str| {
            crate::diagnostic::error(&original, code, Phase::SourceMap, 0, 0, message)
        };
        if segments.len() > segment_limit.min(MAX_SEGMENTS) {
            return Err(fail(
                Code::ResourceExhausted,
                "source-map segment budget exhausted",
            ));
        }
        if original.slice(region).is_none() {
            return Err(fail(
                Code::InvalidSourceMap,
                "invalid original source region",
            ));
        }
        if original.identity() == body.identity() && original.digest() != body.digest() {
            return Err(fail(
                Code::InvalidSourceMap,
                "original and extracted body reuse one identity/revision for different bytes",
            ));
        }
        let mut body_cursor = 0;
        let mut original_cursor = region.start;
        for segment in &segments {
            let Some(body_bytes) = body.slice(segment.body) else {
                return Err(fail(Code::InvalidSourceMap, "invalid body segment"));
            };
            let Some(original_bytes) = original.slice(segment.original) else {
                return Err(fail(Code::InvalidSourceMap, "invalid original segment"));
            };
            if body_bytes.is_empty()
                || segment.body.start != body_cursor
                || segment.original.start < original_cursor
                || segment.original.end > region.end
            {
                return Err(fail(Code::InvalidSourceMap, "segments must cover the body once in original order inside the selected region"));
            }
            if body_bytes != original_bytes {
                return Err(fail(Code::InvalidSourceMap, "mapped bytes differ"));
            }
            if !allowed_gap(
                &original,
                Span {
                    start: original_cursor,
                    end: segment.original.start,
                },
                region.end,
                false,
                layout,
            ) {
                return Err(fail(
                    Code::InvalidSourceMap,
                    "mapping discards bytes outside the selected layout policy",
                ));
            }
            body_cursor = segment.body.end;
            original_cursor = segment.original.end;
        }
        if body_cursor != body.text().len()
            || !allowed_gap(
                &original,
                Span {
                    start: original_cursor,
                    end: region.end,
                },
                region.end,
                true,
                layout,
            )
        {
            return Err(fail(
                Code::InvalidSourceMap,
                "mapping omits body bytes or discards an unapproved source suffix",
            ));
        }
        Ok(Self {
            original,
            body,
            region,
            segments,
            layout,
        })
    }
    /// Original source whose identity and bytes were checked at construction.
    pub fn original(&self) -> &Source {
        &self.original
    }
    /// Extracted source whose full content the segments cover.
    pub fn body(&self) -> &Source {
        &self.body
    }
    /// Selected original region enclosing the correspondence.
    pub fn region(&self) -> Span {
        self.region
    }
    /// Verified segments in body/original order.
    pub fn segments(&self) -> &[Segment] {
        &self.segments
    }
    /// Explicit layout deletions admitted by this map.
    pub fn layout(&self) -> Layout {
        self.layout
    }

    /// Map a span from the exact body source. Discontiguous regions stay separate;
    /// concatenating their original bytes reproduces the body region exactly.
    /// A zero-width boundary selects the following segment, except EOF uses the last end.
    pub fn map_span(
        &self,
        source: &Source,
        span: Span,
    ) -> Result<Vec<LocatedSpan>, Box<Diagnostic>> {
        let fail = |message: &str| {
            crate::diagnostic::error(
                &self.body,
                Code::InvalidSourceMap,
                Phase::SourceMap,
                0,
                0,
                message,
            )
        };
        if source.identity() != self.body.identity()
            || source.path() != self.body.path()
            || source.digest() != self.body.digest()
        {
            return Err(fail(
                "span belongs to a different source identity, revision, path or bytes",
            ));
        }
        if self.body.slice(span).is_none() {
            return Err(fail("invalid body span"));
        }
        let first = self
            .segments
            .partition_point(|segment| segment.body.end <= span.start);
        if span.start == span.end {
            let byte = self.segments.get(first).map_or_else(
                || {
                    self.segments
                        .last()
                        .map_or(self.region.start, |segment| segment.original.end)
                },
                |segment| segment.original.start + span.start - segment.body.start,
            );
            return Ok(vec![self
                .original
                .locate(Span {
                    start: byte,
                    end: byte,
                })
                .expect("verified map boundary")]);
        }
        let mut regions: Vec<Span> = Vec::new();
        for segment in &self.segments[first..] {
            if segment.body.start >= span.end {
                break;
            }
            let start =
                segment.original.start + span.start.max(segment.body.start) - segment.body.start;
            let end = segment.original.start + span.end.min(segment.body.end) - segment.body.start;
            if let Some(previous) = regions.last_mut().filter(|previous| previous.end == start) {
                previous.end = end;
            } else {
                regions.push(Span { start, end });
            }
        }
        Ok(regions
            .into_iter()
            .map(|region| {
                self.original
                    .locate(region)
                    .expect("verified map subregion")
            })
            .collect())
    }
}

fn allowed_gap(
    source: &Source,
    gap: Span,
    region_end: usize,
    trailing: bool,
    layout: Layout,
) -> bool {
    let bytes = source.text().as_bytes();
    let mut at = gap.start;
    while at < gap.end {
        match bytes[at] {
            b' ' | b'\t' if layout.strip_indentation && at < source.indentation_end(at) => {
                // Skip the entire allowed indentation prefix, keeping validation linear.
                at = gap.end.min(source.indentation_end(at));
            }
            b'\r'
                if layout.normalize_crlf
                    && at + 1 < region_end
                    && bytes.get(at + 1) == Some(&b'\n') =>
            {
                at += 1
            }
            b'\n' if trailing && layout.drop_final_newline && at + 1 == region_end => at += 1,
            // Dropping the final CRLF pair does not require normalization of interior newlines.
            b'\r'
                if trailing
                    && layout.drop_final_newline
                    && at + 2 == region_end
                    && gap.end == region_end
                    && bytes.get(at + 1) == Some(&b'\n') =>
            {
                at += 2
            }
            _ => return false,
        }
    }
    true
}
