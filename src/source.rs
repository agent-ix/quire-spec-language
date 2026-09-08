// SPDX-License-Identifier: AGPL-3.0-only
use std::sync::Arc;

/// Authored identity/revision, separate from path and any later semantic digest.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceIdentity {
    pub identity: String,
    pub revision: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Position {
    pub byte: usize,
    pub line: usize,
    pub column: usize,
}

/// Half-open original UTF-8 bytes. Coordinates are derived only when requested.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

/// A decoded syntax value with the exact original token region.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Spanned<T> {
    pub value: T,
    pub span: Span,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LocatedSpan {
    pub start: Position,
    pub end: Position,
}

#[derive(Debug)]
struct Document {
    identity: SourceIdentity,
    path: String,
    text: Box<str>,
    line_starts: Vec<usize>,
    // End byte and cumulative excess bytes over Unicode scalar count.
    wide_ends: Vec<(usize, usize)>,
}

/// Immutable document shared by syntax and diagnostics; clones are constant cost.
#[derive(Clone, Debug)]
pub struct Source(Arc<Document>);

impl Source {
    pub(crate) fn new(identity: SourceIdentity, path: String, text: &str) -> Self {
        let mut line_starts = vec![0];
        let mut wide_ends = Vec::new();
        let mut excess = 0;
        for (at, ch) in text.char_indices() {
            if ch == '\n' {
                line_starts.push(at + 1);
            }
            if ch.len_utf8() > 1 {
                excess += ch.len_utf8() - 1;
                wide_ends.push((at + ch.len_utf8(), excess));
            }
        }
        Self(Arc::new(Document {
            identity,
            path,
            text: text.into(),
            line_starts,
            wide_ends,
        }))
    }
    pub fn identity(&self) -> &SourceIdentity {
        &self.0.identity
    }
    pub fn path(&self) -> &str {
        &self.0.path
    }
    pub fn text(&self) -> &str {
        &self.0.text
    }
    fn excess(&self, byte: usize) -> usize {
        let index = self.0.wide_ends.partition_point(|&(end, _)| end <= byte);
        index.checked_sub(1).map_or(0, |i| self.0.wide_ends[i].1)
    }
    pub fn position(&self, byte: usize) -> Option<Position> {
        if !self.text().is_char_boundary(byte) {
            return None;
        }
        let line = self.0.line_starts.partition_point(|&offset| offset <= byte) - 1;
        let start = self.0.line_starts[line];
        let column = byte - start - (self.excess(byte) - self.excess(start)) + 1;
        Some(Position {
            byte,
            line: line + 1,
            column,
        })
    }
    pub fn locate(&self, span: Span) -> Option<LocatedSpan> {
        if span.start > span.end {
            return None;
        }
        Some(LocatedSpan {
            start: self.position(span.start)?,
            end: self.position(span.end)?,
        })
    }
    pub fn slice(&self, span: Span) -> Option<&str> {
        self.text().get(span.start..span.end)
    }
}
