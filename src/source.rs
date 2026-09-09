// SPDX-License-Identifier: AGPL-3.0-only
//! FR-001: immutable exact source bytes, integrity and checked scalar coordinates.
use crate::{ByteDigest, Code, Diagnostic, Phase};
use std::sync::Arc;

/// Authored identity/revision, separate from path and any later semantic digest.
/// These are opaque caller labels, not validated shared artifact references.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceIdentity {
    /// Caller-selected source label, preserved without normalization.
    pub identity: String,
    /// Caller-selected revision label, distinct from the byte digest.
    pub revision: String,
}

/// Original byte offset and one-based line/Unicode scalar column.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Position {
    /// Zero-based offset into the exact original UTF-8 bytes.
    pub byte: usize,
    /// One-based line; LF advances the line, including within CRLF.
    pub line: usize,
    /// One-based Unicode scalar column, not a display-cell or UTF-16 offset.
    pub column: usize,
}

/// Half-open original UTF-8 bytes. Coordinates are derived only when requested.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Span {
    /// Inclusive starting byte offset.
    pub start: usize,
    /// Exclusive ending byte offset; may equal start for an empty range.
    pub end: usize,
}

/// A decoded syntax value with the exact original token region.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Spanned<T> {
    /// Decoded value, which need not equal the raw token spelling.
    pub value: T,
    /// Original token byte range, including literal delimiters.
    pub span: Span,
}

/// Half-open range after resolving both byte offsets to source coordinates.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LocatedSpan {
    /// Inclusive range start.
    pub start: Position,
    /// Exclusive range end.
    pub end: Position,
}

#[derive(Debug)]
struct Document {
    identity: SourceIdentity,
    path: String,
    text: Box<str>,
    line_starts: Vec<usize>,
    indentation_ends: Vec<usize>,
    digest: ByteDigest,
    // End byte and cumulative excess bytes over Unicode scalar count.
    wide_ends: Vec<(usize, usize)>,
}

/// Immutable document shared by syntax and diagnostics; clones are constant cost.
#[derive(Clone, Debug)]
pub struct Source(Arc<Document>);

/// Hard source-content ceiling; callers may select a lower value.
pub const MAX_SOURCE_BYTES: usize = 1_048_576;

impl Source {
    /// Read original UTF-8 bytes without normalization. Caller may lower the 1 MiB ceiling.
    pub fn read(
        identity: SourceIdentity,
        path: impl Into<String>,
        bytes: &[u8],
        byte_limit: usize,
    ) -> Result<Self, Box<Diagnostic>> {
        let path = path.into();
        let byte_limit = byte_limit.min(MAX_SOURCE_BYTES);
        let point = Position {
            byte: 0,
            line: 1,
            column: 1,
        };
        let refusal = |code, message: &str| {
            Box::new(Diagnostic {
                phase: Phase::Source,
                code,
                source: identity.clone(),
                path: path.clone(),
                span: LocatedSpan {
                    start: point,
                    end: point,
                },
                message: message.into(),
                related: Vec::new(),
                upstream: None,
            })
        };
        if identity.identity.trim().is_empty()
            || identity.revision.trim().is_empty()
            || path.is_empty()
        {
            return Err(refusal(
                Code::InvalidSourceIdentity,
                "source identity, revision and path must be explicit",
            ));
        }
        if bytes.len() > byte_limit {
            return Err(refusal(
                Code::ResourceExhausted,
                "source byte budget exhausted",
            ));
        }
        let text = match std::str::from_utf8(bytes) {
            Ok(text) => text,
            Err(error) => {
                let mut diagnostic = refusal(Code::InvalidUtf8, "source must be valid UTF-8");
                let prefix =
                    std::str::from_utf8(&bytes[..error.valid_up_to()]).expect("UTF-8 valid prefix");
                let source = Source::new(identity.clone(), path.clone(), prefix);
                diagnostic.span = source
                    .locate(Span {
                        start: prefix.len(),
                        end: prefix.len(),
                    })
                    .expect("prefix EOF");
                return Err(diagnostic);
            }
        };
        let source = Source::new(identity, path, text);
        if let Some(at) = text.find('\0') {
            return Err(crate::diagnostic::error(
                &source,
                Code::InvalidSyntax,
                Phase::Source,
                at,
                at + 1,
                "NUL is forbidden in source bytes",
            ));
        }
        Ok(source)
    }

    /// Verify an independently supplied byte digest before constructing a mapped subject.
    pub fn read_verified(
        identity: SourceIdentity,
        path: impl Into<String>,
        bytes: &[u8],
        expected: ByteDigest,
        byte_limit: usize,
    ) -> Result<Self, Box<Diagnostic>> {
        let source = Self::read(identity, path, bytes, byte_limit)?;
        if source.digest() != expected {
            return Err(crate::diagnostic::error(
                &source,
                Code::SourceDigestMismatch,
                Phase::Source,
                0,
                0,
                "source bytes differ from the selected digest",
            ));
        }
        Ok(source)
    }
    /// SHA-256 of the exact admitted bytes, without normalization.
    pub fn digest(&self) -> ByteDigest {
        self.0.digest
    }
    pub(crate) fn indentation_end(&self, byte: usize) -> usize {
        let line = self.0.line_starts.partition_point(|&start| start <= byte) - 1;
        self.0.indentation_ends[line]
    }

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
        let indentation_ends = line_starts
            .iter()
            .map(|&start| {
                start
                    + text.as_bytes()[start..]
                        .iter()
                        .take_while(|&&b| matches!(b, b' ' | b'\t'))
                        .count()
            })
            .collect();
        Self(Arc::new(Document {
            identity,
            path,
            text: text.into(),
            line_starts,
            indentation_ends,
            digest: ByteDigest::of(text.as_bytes()),
            wide_ends,
        }))
    }
    /// Original opaque caller-selected labels.
    pub fn identity(&self) -> &SourceIdentity {
        &self.0.identity
    }
    /// Display path; not a portable artifact reference or lossless OS path.
    pub fn path(&self) -> &str {
        &self.0.path
    }
    /// Original UTF-8 content with whitespace and line endings intact.
    pub fn text(&self) -> &str {
        &self.0.text
    }
    fn excess(&self, byte: usize) -> usize {
        let index = self.0.wide_ends.partition_point(|&(end, _)| end <= byte);
        index.checked_sub(1).map_or(0, |i| self.0.wide_ends[i].1)
    }
    /// Resolve a UTF-8 boundary, including EOF; reject split scalars/out-of-range offsets.
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
    /// Resolve both boundaries; reject reversed, split-scalar or out-of-range spans.
    pub fn locate(&self, span: Span) -> Option<LocatedSpan> {
        if span.start > span.end {
            return None;
        }
        Some(LocatedSpan {
            start: self.position(span.start)?,
            end: self.position(span.end)?,
        })
    }
    /// Borrow exact bytes at a valid UTF-8 range; invalid ranges return None.
    pub fn slice(&self, span: Span) -> Option<&str> {
        self.text().get(span.start..span.end)
    }
}
