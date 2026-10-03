// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-001: immutable exact source bytes, integrity, the caller-named source
//! reference and checked scalar coordinates.
//!
//! [`provenance`] holds ADR-013 O-07/O-12's occurrence key, source region
//! and package source map.
use crate::digest::{DigestDomain, DigestRecord};
use crate::ByteDigest;
use provenance::{RawSourceRef, SourceRegion};
use std::sync::Arc;

pub mod provenance;

/// FR-001: the four labels a caller names a source by, separate from the
/// path and from any digest. QSL defaults none of them and derives none
/// from the path, the bytes or another label. Ordering is lexical over the
/// four labels in declaration order.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, serde::Serialize)]
pub struct SourceIdentity {
    /// The authority that issues the source's identity.
    pub authority: String,
    /// The source's identity within that authority, preserved without
    /// normalization.
    pub identity: String,
    /// The revision system the revision value belongs to, such as `git`.
    pub revision_namespace: String,
    /// The revision value within that namespace, distinct from the byte
    /// digest.
    pub revision: String,
}

/// One of the four labels of a [`SourceIdentity`], in FR-001's check order.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SourceLabel {
    /// The authority that issues the identity.
    Authority,
    /// The identity within the authority.
    Identity,
    /// The revision namespace.
    RevisionNamespace,
    /// The revision value.
    Revision,
}

impl SourceLabel {
    /// The labels in the order admission checks them.
    pub const ALL: [Self; 4] = [
        Self::Authority,
        Self::Identity,
        Self::RevisionNamespace,
        Self::Revision,
    ];

    /// The catalog spelling of the `label` payload
    /// (`quire.native.diagnostics/v1`, `blank-label`).
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Authority => "authority",
            Self::Identity => "identity",
            Self::RevisionNamespace => "revision_namespace",
            Self::Revision => "revision",
        }
    }
}

impl SourceIdentity {
    /// The four labels exactly as given. Nothing is checked here: admission
    /// ([`Source::read_typed`]) refuses a blank label.
    pub fn new(
        authority: impl Into<String>,
        identity: impl Into<String>,
        revision_namespace: impl Into<String>,
        revision: impl Into<String>,
    ) -> Self {
        Self {
            authority: authority.into(),
            identity: identity.into(),
            revision_namespace: revision_namespace.into(),
            revision: revision.into(),
        }
    }

    /// FR-001: the first blank label in the order authority, identity,
    /// revision namespace, revision. A label is blank when it is empty or
    /// only Unicode `White_Space` scalars (`str::trim` strips exactly
    /// those; U+3000 is blank, U+200B is not).
    pub fn first_blank_label(&self) -> Option<SourceLabel> {
        SourceLabel::ALL
            .into_iter()
            .find(|label| self.label(*label).trim().is_empty())
    }

    /// FR-001: whether every label is non-blank.
    pub fn is_named(&self) -> bool {
        self.first_blank_label().is_none()
    }

    /// The value of one label.
    pub fn label(&self, label: SourceLabel) -> &str {
        match label {
            SourceLabel::Authority => &self.authority,
            SourceLabel::Identity => &self.identity,
            SourceLabel::RevisionNamespace => &self.revision_namespace,
            SourceLabel::Revision => &self.revision,
        }
    }

    /// The `RawSourceRef` of bytes with `digest` under these labels.
    /// Provenance refuses only an empty label, which is a blank one, so a
    /// blank label is named first and a provenance refusal after it is an
    /// established-invariant break ([`ReferenceError::Provenance`]).
    fn reference(&self, digest: ByteDigest) -> Result<RawSourceRef, ReferenceError> {
        if let Some(label) = self.first_blank_label() {
            return Err(ReferenceError::Blank(label));
        }
        RawSourceRef::new(
            self.authority.clone(),
            self.identity.clone(),
            DigestRecord::mint(DigestDomain::SourceBytesV1, digest.as_bytes()),
        )
        .map_err(ReferenceError::Provenance)
    }
}

/// Why [`SourceIdentity::reference`] minted no reference.
enum ReferenceError {
    /// A label is blank: `blank-label` naming it.
    Blank(SourceLabel),
    /// Provenance refused labels that are all non-blank: not reachable, so
    /// it is reported as [`SourceReadCause::ReferenceInvariant`], never as a
    /// label.
    Provenance(provenance::InvalidProvenance),
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
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, serde::Serialize)]
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

/// Half-open range after resolving both byte offsets to source coordinates:
/// a rendering, derived from the source bytes when it is asked for (FR-001).
/// The canonical S0 and S1 diagnostics name their position by a
/// [`SourceRegion`], which holds bytes only; the native-v1 `Diagnostic`
/// keeps this rendering as its span (ADR-013 §6, lane-private).
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
    reference: RawSourceRef,
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

/// Why [`Source::read_typed`] or [`Source::read_verified_typed`] refused
/// its input. `pub`: the `qsl-cst` crate's `read_source` maps this cause
/// onto its own `CompleteCode`, a real cross-crate call site.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SourceReadCause {
    /// A label is blank (`blank-label`): the first blank one, in FR-001's
    /// order.
    BlankLabel {
        /// The first blank label.
        label: SourceLabel,
    },
    /// Every label is non-blank and the path is empty (`empty-path`).
    EmptyPath,
    /// Provenance refused labels and a path that passed the blank checks:
    /// an established invariant broke (`runtime_invariant`); not reachable
    /// from any input today.
    ReferenceInvariant,
    /// The bytes exceed the byte budget.
    ByteBudget,
    /// The bytes are not UTF-8.
    InvalidUtf8,
    /// The text begins with a byte-order mark.
    Bom,
    /// The text contains NUL.
    Nul,
    /// Verified intake: the bytes differ from the selected digest.
    DigestMismatch,
}

/// The catalog cause tag of a blank label
/// (`quire.native.diagnostics/v1`, `invalid_source_identity`); the one
/// spelling every layer above reads.
pub const BLANK_LABEL_TAG: &str = "blank-label";
/// The catalog cause tag of an empty path; see [`BLANK_LABEL_TAG`].
pub const EMPTY_PATH_TAG: &str = "empty-path";

/// The wire fields an `invalid_source_identity` refusal adds to a refusal
/// record: `cause`, and `label` on `blank-label` only. Flattened into the
/// CLI refusal line and the run output.
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize)]
pub struct IdentityCauseFields {
    /// `blank-label` or `empty-path`.
    pub cause: &'static str,
    /// The first blank label's spelling.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<&'static str>,
}

impl SourceReadCause {
    /// The wire fields of an `invalid_source_identity` cause; `None` for
    /// every other cause.
    pub const fn identity_fields(self) -> Option<IdentityCauseFields> {
        match self {
            Self::BlankLabel { label } => Some(IdentityCauseFields {
                cause: BLANK_LABEL_TAG,
                label: Some(label.as_str()),
            }),
            Self::EmptyPath => Some(IdentityCauseFields {
                cause: EMPTY_PATH_TAG,
                label: None,
            }),
            Self::ReferenceInvariant
            | Self::ByteBudget
            | Self::InvalidUtf8
            | Self::Bom
            | Self::Nul
            | Self::DigestMismatch => None,
        }
    }
}

/// Cause-specific location and message for a source refusal, before
/// `diagnostic` (later in this layer's order) maps it onto a stable `Code`.
/// `source` does not construct a `Diagnostic` (ADR-011 §6.1). `pub`: see
/// [`SourceReadCause`]'s own doc for the cross-crate call site.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceReadError {
    /// The labels the refusal occurred against, exactly as offered.
    pub source: SourceIdentity,
    /// The display path the refusal occurred against.
    pub path: String,
    /// FR-001: the refusal's region, under a `RawSourceRef` minted over the
    /// offered bytes, for invalid UTF-8, a BOM and NUL; `None` for an
    /// unnamed source, a byte-budget refusal and a digest mismatch.
    pub region: Option<SourceRegion>,
    /// Human-readable detail.
    pub message: String,
}

/// A typed source refusal. `pub`: see [`SourceReadCause`]'s own doc for the
/// cross-crate call site.
pub struct SourceReadRefusal {
    /// Why the read refused.
    pub cause: SourceReadCause,
    /// The refusal's location and message.
    pub error: SourceReadError,
}

/// Default source-content ceiling for callers that configure none. It is
/// not a clamp: [`Source::read_typed`] enforces the caller's own
/// `byte_limit` as given (NFR-001).
pub const MAX_SOURCE_BYTES: usize = 1_048_576;

const BOM: &str = "\u{feff}";

/// FR-001: render `region` over `bytes`, the bytes its `RawSourceRef` names,
/// deriving one-based lines and Unicode scalar columns from the longest
/// valid UTF-8 prefix of those bytes. Returns `None` when the bytes are not
/// the ones the region's reference names, or when an end is not a scalar
/// boundary inside that prefix. This is how a refusal region over offered
/// bytes that were never admitted is rendered.
pub fn render_offered(bytes: &[u8], region: &SourceRegion) -> Option<LocatedSpan> {
    if region.source().digest().as_bytes() != &ByteDigest::of(bytes).as_bytes() {
        return None;
    }
    let text = match std::str::from_utf8(bytes) {
        Ok(text) => text,
        Err(error) => std::str::from_utf8(bytes.get(..error.valid_up_to())?).ok()?,
    };
    let position = |byte: u64| -> Option<Position> {
        let byte = usize::try_from(byte).ok()?;
        let before = text.get(..byte)?;
        let line_start = before.rfind('\n').map_or(0, |at| at + 1);
        Some(Position {
            byte,
            line: before.matches('\n').count() + 1,
            column: before[line_start..].chars().count() + 1,
        })
    };
    Some(LocatedSpan {
        start: position(region.start())?,
        end: position(region.end())?,
    })
}

impl Source {
    /// [`Source::read`] with the refusal's typed cause retained, before
    /// `diagnostic` maps it onto a stable code (`Source::read` itself is
    /// implemented there; see `diagnostic.rs`).
    ///
    /// FR-001's order: a blank label or empty path, then the byte ceiling,
    /// refuse with no region and without hashing the bytes; invalid UTF-8,
    /// a BOM and NUL refuse at a region under the `RawSourceRef` of the
    /// offered bytes. An admitted source carries its `RawSourceRef`.
    #[qsl_attrs::string_edge]
    pub fn read_typed(
        identity: SourceIdentity,
        path: impl Into<String>,
        bytes: &[u8],
        byte_limit: usize,
    ) -> Result<Self, Box<SourceReadRefusal>> {
        let path = path.into();
        let refuse = |cause, region, message: &str| {
            Box::new(SourceReadRefusal {
                cause,
                error: SourceReadError {
                    source: identity.clone(),
                    path: path.clone(),
                    region,
                    message: message.into(),
                },
            })
        };
        if let Some(label) = identity.first_blank_label() {
            return Err(refuse(
                SourceReadCause::BlankLabel { label },
                None,
                &format!("source {} must not be blank", label.as_str()),
            ));
        }
        if path.is_empty() {
            return Err(refuse(
                SourceReadCause::EmptyPath,
                None,
                "source path must not be empty",
            ));
        }
        if bytes.len() > byte_limit {
            return Err(refuse(
                SourceReadCause::ByteBudget,
                None,
                &format!("source byte budget exhausted (limit {byte_limit})"),
            ));
        }
        let digest = ByteDigest::of(bytes);
        // Every label is non-blank here, so the reference is minted unless
        // provenance itself refuses the values.
        let reference = match identity.reference(digest) {
            Ok(reference) => reference,
            Err(ReferenceError::Blank(label)) => {
                return Err(refuse(
                    SourceReadCause::BlankLabel { label },
                    None,
                    &format!("source {} must not be blank", label.as_str()),
                ))
            }
            Err(ReferenceError::Provenance(error)) => {
                return Err(refuse(
                    SourceReadCause::ReferenceInvariant,
                    None,
                    &format!("source reference refused non-blank labels: {error}"),
                ))
            }
        };
        let at = |start: usize, end: usize| {
            let (start, end) = (u64::try_from(start).ok()?, u64::try_from(end).ok()?);
            SourceRegion::new(reference.clone(), start, end).ok()
        };
        let text = match std::str::from_utf8(bytes) {
            Ok(text) => text,
            Err(error) => {
                let end = error.valid_up_to();
                return Err(refuse(
                    SourceReadCause::InvalidUtf8,
                    at(end, end),
                    "source must be valid UTF-8",
                ));
            }
        };
        if text.starts_with(BOM) {
            return Err(refuse(
                SourceReadCause::Bom,
                at(0, BOM.len()),
                "a byte-order mark is forbidden in source bytes",
            ));
        }
        if let Some(nul) = text.find('\0') {
            return Err(refuse(
                SourceReadCause::Nul,
                at(nul, nul + 1),
                "NUL is forbidden in source bytes",
            ));
        }
        Ok(Self::admit(identity, reference, path, text, digest))
    }

    /// Verified intake: [`Source::read_typed`], then the admitted bytes'
    /// digest against the independently selected one. A mismatch concerns
    /// the bytes as a whole and refuses with no region (FR-001).
    pub fn read_verified_typed(
        identity: SourceIdentity,
        path: impl Into<String>,
        bytes: &[u8],
        expected: ByteDigest,
        byte_limit: usize,
    ) -> Result<Self, Box<SourceReadRefusal>> {
        let source = Self::read_typed(identity, path, bytes, byte_limit)?;
        if source.digest() != expected {
            return Err(Box::new(SourceReadRefusal {
                cause: SourceReadCause::DigestMismatch,
                error: SourceReadError {
                    source: source.identity().clone(),
                    path: source.path().into(),
                    region: None,
                    message: "source bytes differ from the selected digest".into(),
                },
            }));
        }
        Ok(source)
    }

    /// SHA-256 of the exact admitted bytes, without normalization.
    pub fn digest(&self) -> ByteDigest {
        self.0.digest
    }

    /// FR-001: the `RawSourceRef` admission minted, the caller's four labels
    /// exactly as supplied and the `quire.source.bytes/v1` digest of the
    /// admitted bytes. Every later stage names this source by it.
    pub fn reference(&self) -> &RawSourceRef {
        &self.0.reference
    }

    /// The region `span` of this source under its `RawSourceRef`, or `None`
    /// for a reversed span, one past the end or one splitting a scalar.
    pub fn region(&self, span: Span) -> Option<SourceRegion> {
        self.slice(span)?;
        let (start, end) = (
            u64::try_from(span.start).ok()?,
            u64::try_from(span.end).ok()?,
        );
        SourceRegion::new(self.0.reference.clone(), start, end).ok()
    }

    /// FR-001: render a region of this source, deriving the one-based line
    /// and Unicode scalar column of each end from the admitted bytes. Returns
    /// `None` for a region under another source's bytes, or one whose ends
    /// are not scalar boundaries of this source.
    pub fn render(&self, region: &SourceRegion) -> Option<LocatedSpan> {
        if region.source().digest() != self.0.reference.digest() {
            return None;
        }
        self.locate(Span {
            start: usize::try_from(region.start()).ok()?,
            end: usize::try_from(region.end()).ok()?,
        })
    }

    pub(crate) fn indentation_end(&self, byte: usize) -> usize {
        let line = self.0.line_starts.partition_point(|&start| start <= byte) - 1;
        self.0.indentation_ends[line]
    }

    fn admit(
        identity: SourceIdentity,
        reference: RawSourceRef,
        path: String,
        text: &str,
        digest: ByteDigest,
    ) -> Self {
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
            reference,
            path,
            text: text.into(),
            line_starts,
            indentation_ends,
            digest,
            wide_ends,
        }))
    }
    /// The caller's four labels exactly as supplied.
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
    /// The byte offset of the start of a one-based line, or `None` past the
    /// last line. `pub`: the `qsl-source` crate (I3, feature
    /// `quire-extraction`) reuses this source's existing byte index for
    /// Quire line loci, a real cross-crate call site. Always compiled here
    /// (this crate has no `quire-extraction` feature of its own);
    /// `qsl-source`'s own feature gate decides whether its caller is compiled.
    pub fn line_start(&self, line: usize) -> Option<usize> {
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

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;

    use super::*;

    fn labels(authority: &str, identity: &str, namespace: &str, revision: &str) -> SourceIdentity {
        SourceIdentity::new(authority, identity, namespace, revision)
    }

    fn refusal(labels: SourceIdentity, bytes: &[u8], limit: usize) -> Box<SourceReadRefusal> {
        match Source::read_typed(labels, "a.quire", bytes, limit) {
            Ok(_) => panic!("{bytes:?} was admitted"),
            Err(refusal) => refusal,
        }
    }

    /// FR-001-AC-5: admission mints a `RawSourceRef` of the authority and
    /// identity labels and the `quire.source.bytes/v1` digest of the bytes
    /// (the type has no other member); other bytes under the same labels
    /// change only the digest.
    #[trace("TC-424", "FR-001-AC-5")]
    #[test]
    fn admission_mints_the_caller_named_source_reference() {
        let admit = |bytes: &[u8]| {
            Source::read_typed(
                labels("agent-ix", "specs/a.quire", "git", "3f2a"),
                "a.quire",
                bytes,
                MAX_SOURCE_BYTES,
            )
            .unwrap_or_else(|_| panic!("admitted"))
        };
        let first = admit(b"b");
        let reference = first.reference();
        assert_eq!(reference.authority(), "agent-ix");
        assert_eq!(reference.identity(), "specs/a.quire");
        assert_eq!(reference.digest().domain(), DigestDomain::SourceBytesV1);
        assert_eq!(
            reference.digest().as_bytes(),
            &ByteDigest::of(b"b").as_bytes()
        );

        let second = admit(b"b'");
        let other = second.reference();
        assert_ne!(reference, other);
        assert_eq!(
            (other.authority(), other.identity()),
            (reference.authority(), reference.identity())
        );
        assert_eq!(other.digest().as_bytes(), &ByteDigest::of(b"b'").as_bytes());
    }

    /// FR-001-AC-6: exactly one empty, single-space or U+3000 label refuses
    /// with `blank-label` naming that label, and no region; U+200B is not
    /// `White_Space`, so it is not blank.
    #[trace("TC-424", "FR-001-AC-6")]
    #[test]
    fn a_blank_label_refuses_naming_it_and_admits_nothing() {
        for blank in ["", " ", "\u{3000}"] {
            for label in SourceLabel::ALL {
                let mut values = ["agent-ix", "specs/a.quire", "git", "3f2a"];
                let position = SourceLabel::ALL.iter().position(|l| *l == label).unwrap();
                values[position] = blank;
                let refused = refusal(
                    labels(values[0], values[1], values[2], values[3]),
                    b"b",
                    MAX_SOURCE_BYTES,
                );
                assert_eq!(
                    refused.cause,
                    SourceReadCause::BlankLabel { label },
                    "{label:?} = {blank:?}"
                );
                assert_eq!(refused.error.region, None);
            }
        }
        let zero_width = labels("\u{200b}", "u", "git", "1");
        assert!(Source::read_typed(zero_width, "a", b"b", MAX_SOURCE_BYTES).is_ok());
    }

    /// FR-001-AC-11: the first blank label in order wins, and a blank label
    /// beats an empty path; only with four non-blank labels is the path
    /// refused, as `empty-path`.
    #[trace("TC-424", "FR-001-AC-11")]
    #[test]
    fn label_order_precedes_the_path() {
        let cause = |identity, path: &str| {
            Source::read_typed(identity, path, b"b", MAX_SOURCE_BYTES)
                .expect_err("refused")
                .cause
        };
        assert_eq!(
            cause(labels(" ", "u", "", "1"), "a"),
            SourceReadCause::BlankLabel {
                label: SourceLabel::Authority
            }
        );
        assert_eq!(
            cause(labels("a", "", "git", "1"), ""),
            SourceReadCause::BlankLabel {
                label: SourceLabel::Identity
            }
        );
        assert_eq!(
            cause(labels("a", "u", "git", "1"), ""),
            SourceReadCause::EmptyPath
        );
        assert_eq!(
            Source::read_typed(labels("a", "u", "git", "1"), "", b"b", MAX_SOURCE_BYTES)
                .unwrap_err()
                .error
                .region,
            None
        );
    }

    /// FR-001-AC-8: invalid UTF-8 refuses at the empty region at the end of
    /// the longest valid prefix, NUL at its one-byte region, and a BOM at
    /// its three bytes; each region is under the `RawSourceRef` of the
    /// offered bytes.
    #[trace("TC-424", "FR-001-AC-8")]
    #[test]
    fn content_refusals_are_located_under_the_offered_bytes() {
        let named = || labels("a", "u", "git", "1");
        for (bytes, cause, start, end) in [
            (&b"a\xffb"[..], SourceReadCause::InvalidUtf8, 1, 1),
            (&b"ab\0c"[..], SourceReadCause::Nul, 2, 3),
            (&b"\xef\xbb\xbfab"[..], SourceReadCause::Bom, 0, 3),
        ] {
            let refused = refusal(named(), bytes, MAX_SOURCE_BYTES);
            assert_eq!(refused.cause, cause);
            let region = refused.error.region.expect("a located refusal");
            assert_eq!((region.start(), region.end()), (start, end));
            let reference = region.source();
            assert_eq!((reference.authority(), reference.identity()), ("a", "u"));
            assert_eq!(
                reference.digest().as_bytes(),
                &ByteDigest::of(bytes).as_bytes()
            );
        }
    }

    /// FR-001-AC-10: an empty revision namespace, five bytes under a
    /// four-byte ceiling and a digest mismatch under verified intake each
    /// refuse with no region.
    #[trace("TC-424", "FR-001-AC-10")]
    #[test]
    fn unnamed_over_budget_and_mismatched_refusals_have_no_region() {
        let unnamed = refusal(labels("a", "u", "", "1"), b"b", MAX_SOURCE_BYTES);
        assert_eq!(
            unnamed.cause,
            SourceReadCause::BlankLabel {
                label: SourceLabel::RevisionNamespace
            }
        );
        assert_eq!(unnamed.error.region, None);

        let over = refusal(labels("a", "u", "git", "1"), b"abcde", 4);
        assert_eq!(over.cause, SourceReadCause::ByteBudget);
        assert_eq!(over.error.region, None);

        let Err(mismatch) = Source::read_verified_typed(
            labels("a", "u", "git", "1"),
            "a.quire",
            b"b",
            ByteDigest::of(b"c"),
            MAX_SOURCE_BYTES,
        ) else {
            panic!("a mismatched digest was admitted");
        };
        assert_eq!(mismatch.cause, SourceReadCause::DigestMismatch);
        assert_eq!(mismatch.error.region, None);
    }

    /// FR-001-AC-9: a region holds its reference and two bytes; the
    /// renderer derives line and scalar column from the admitted bytes.
    #[trace("TC-424", "FR-001-AC-9")]
    #[test]
    fn line_and_column_are_derived_when_rendered() {
        let text = "ab\ncdéf";
        let source = Source::read_typed(
            labels("a", "u", "git", "1"),
            "a.quire",
            text.as_bytes(),
            MAX_SOURCE_BYTES,
        )
        .unwrap_or_else(|_| panic!("admitted"));
        let region = source.region(Span { start: 4, end: 7 }).expect("a region");
        assert_eq!(region.source(), source.reference());
        assert_eq!((region.start(), region.end()), (4, 7));
        let rendered = source.render(&region).expect("rendered");
        assert_eq!(
            (rendered.start.line, rendered.start.column),
            (2, 2),
            "start"
        );
        assert_eq!((rendered.end.line, rendered.end.column), (2, 4), "end");
        assert_eq!(render_offered(text.as_bytes(), &region), Some(rendered));

        // A refusal region over bytes that were never admitted renders over
        // their longest valid prefix.
        let refused = refusal(labels("a", "u", "git", "1"), b"a\n\xffb", MAX_SOURCE_BYTES);
        let region = refused.error.region.expect("a located refusal");
        let rendered = render_offered(b"a\n\xffb", &region).expect("rendered");
        assert_eq!((rendered.start.line, rendered.start.column), (2, 1));
        assert_eq!(render_offered(b"other", &region), None);
    }
}
