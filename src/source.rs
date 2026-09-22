// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-001: immutable exact source bytes, integrity and checked scalar coordinates.
use crate::ByteDigest;
use std::sync::Arc;

/// Authored identity/revision, separate from path and any later semantic digest.
/// These are opaque caller labels, not validated shared artifact references.
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
pub struct SourceIdentity {
    /// Caller-selected source label, preserved without normalization.
    pub identity: String,
    /// Caller-selected revision label, distinct from the byte digest.
    pub revision: String,
}

/// Original byte offset and one-based line/Unicode scalar column.
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize)]
pub struct Position {
    /// Zero-based offset into the exact original UTF-8 bytes.
    pub byte: usize,
    /// One-based line; LF advances the line, including within CRLF.
    pub line: usize,
    /// One-based Unicode scalar column, not a display-cell or UTF-16 offset.
    pub column: usize,
}

/// Half-open original UTF-8 bytes. Coordinates are derived only when requested.
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize)]
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
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize)]
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

/// Why [`Source::read`] refused its input.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SourceReadCause {
    /// The identity, revision or path is empty.
    UnnamedSource,
    /// The bytes exceed the byte budget.
    ByteBudget,
    /// The bytes are not UTF-8.
    InvalidUtf8,
    /// The text contains NUL.
    Nul,
}

/// Cause-specific location and message for a [`Source::read`] refusal, before
/// `diagnostic` (later in this layer's order) maps it onto a stable `Code`.
/// `source` does not construct a `Diagnostic` (ADR-011 §6.1).
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SourceReadError {
    pub(crate) source: SourceIdentity,
    pub(crate) path: String,
    pub(crate) span: LocatedSpan,
    pub(crate) message: String,
}

/// A typed [`Source::read`] refusal.
pub(crate) struct SourceReadRefusal {
    pub(crate) cause: SourceReadCause,
    pub(crate) error: SourceReadError,
}

/// Hard source-content ceiling; callers may select a lower value.
pub const MAX_SOURCE_BYTES: usize = 1_048_576;

impl Source {
    /// [`Source::read`] with the refusal's typed cause retained, before
    /// `diagnostic` maps it onto a stable code (`Source::read` itself is
    /// implemented there; see `diagnostic.rs`).
    pub(crate) fn read_typed(
        identity: SourceIdentity,
        path: impl Into<String>,
        bytes: &[u8],
        byte_limit: usize,
    ) -> Result<Self, SourceReadRefusal> {
        let path = path.into();
        let byte_limit = byte_limit.min(MAX_SOURCE_BYTES);
        let point = Position {
            byte: 0,
            line: 1,
            column: 1,
        };
        let refusal = |cause, source: SourceIdentity, path: String, message: &str| SourceReadRefusal {
            cause,
            error: SourceReadError {
                source,
                path,
                span: LocatedSpan {
                    start: point,
                    end: point,
                },
                message: message.into(),
            },
        };
        if identity.identity.trim().is_empty()
            || identity.revision.trim().is_empty()
            || path.is_empty()
        {
            return Err(refusal(
                SourceReadCause::UnnamedSource,
                identity,
                path,
                "source identity, revision and path must be explicit",
            ));
        }
        if bytes.len() > byte_limit {
            return Err(refusal(
                SourceReadCause::ByteBudget,
                identity,
                path,
                "source byte budget exhausted",
            ));
        }
        let text = match std::str::from_utf8(bytes) {
            Ok(text) => text,
            Err(error) => {
                let mut refused = refusal(
                    SourceReadCause::InvalidUtf8,
                    identity.clone(),
                    path.clone(),
                    "source must be valid UTF-8",
                );
                let prefix =
                    std::str::from_utf8(&bytes[..error.valid_up_to()]).expect("UTF-8 valid prefix");
                let source = Source::new(identity.clone(), path.clone(), prefix);
                refused.error.span = source
                    .locate(Span {
                        start: prefix.len(),
                        end: prefix.len(),
                    })
                    .expect("prefix EOF");
                return Err(refused);
            }
        };
        let source = Source::new(identity, path, text);
        if let Some(at) = text.find('\0') {
            let span = source
                .locate(Span {
                    start: at,
                    end: at + 1,
                })
                .expect("internal offsets are UTF-8 boundaries");
            return Err(SourceReadRefusal {
                cause: SourceReadCause::Nul,
                error: SourceReadError {
                    source: source.identity().clone(),
                    path: source.path().into(),
                    span,
                    message: "NUL is forbidden in source bytes".into(),
                },
            });
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
    // Quire line loci reuse this source's existing byte index.
    #[cfg(feature = "quire-extraction")]
    pub(crate) fn line_start(&self, line: usize) -> Option<usize> {
        self.0.line_starts.get(line.checked_sub(1)?).copied()
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
