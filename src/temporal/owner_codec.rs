// SPDX-License-Identifier: AGPL-3.0-or-later
//! Ordered, length-delimited owner report framing.

const FORMAT: &[u8; 4] = b"QSLR";
const VERSION: u8 = 1;

/// The binding member that differs from the reader's supplied authority.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BindingDimension {
    /// The checked package identity.
    PackageIdentity,
    /// The checked package version.
    PackageVersion,
    /// The checked clause node.
    Clause,
    /// The declaration role and ordinal.
    Occurrence,
    /// The clause's source-map entry.
    SourceMap,
    /// The opaque observation reference.
    Observation,
    /// The opaque observation revision reference.
    Revision,
}

/// A strict owner report admission failure. No failure carries a partial report.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ReportRefusal {
    /// The format identifier or version is unsupported.
    #[error("unsupported owner report format or version")]
    UnsupportedVersion,
    /// The input ends before a required member is complete.
    #[error("truncated owner report")]
    Truncated,
    /// Bytes follow the final member.
    #[error("bytes follow the owner report")]
    TrailingBytes,
    /// A closed enumeration carries an unknown tag.
    #[error("unknown owner report enumeration tag")]
    UnknownTag,
    /// A member is repeated or appears outside its fixed position.
    #[error("owner report member out of order")]
    OutOfOrder,
    /// A binding differs from the supplied authority.
    #[error("owner report binding mismatch: {0:?}")]
    BindingMismatch(BindingDimension),
    /// A support entry names an unadmitted position or valuation row.
    #[error("owner report support is not in the admitted trace")]
    SupportNotInTrace,
    /// A correction does not name its valid direct predecessor.
    #[error("invalid owner report predecessor")]
    InvalidPredecessor,
}

/// Writes a format header followed by tagged, length-delimited fields.
pub(super) struct Writer {
    bytes: Vec<u8>,
}

impl Writer {
    pub(super) fn new() -> Self {
        let mut bytes = FORMAT.to_vec();
        bytes.push(VERSION);
        Self { bytes }
    }

    pub(super) fn field(&mut self, tag: u8, value: &[u8]) {
        self.bytes.push(tag);
        // A Rust allocation is bounded by usize; supported targets have at most
        // 64-bit addresses. Encode the length in eight bytes on every target.
        let mut length = [0_u8; 8];
        let native = value.len().to_le_bytes();
        for (destination, source) in length.iter_mut().zip(native) {
            *destination = source;
        }
        self.bytes.extend_from_slice(&length);
        self.bytes.extend_from_slice(value);
    }

    pub(super) fn finish(self) -> Vec<u8> {
        self.bytes
    }
}

/// Borrows fields only after their complete framing has been admitted.
pub(super) struct Reader<'a> {
    remaining: &'a [u8],
}

impl<'a> Reader<'a> {
    pub(super) fn new(bytes: &'a [u8]) -> Result<Self, ReportRefusal> {
        let header = bytes.get(..5).ok_or(ReportRefusal::Truncated)?;
        if header.get(..4) != Some(FORMAT.as_slice()) || header.get(4) != Some(&VERSION) {
            return Err(ReportRefusal::UnsupportedVersion);
        }
        Ok(Self {
            remaining: bytes.get(5..).ok_or(ReportRefusal::Truncated)?,
        })
    }

    pub(super) fn field(&mut self, expected: u8) -> Result<&'a [u8], ReportRefusal> {
        let (&tag, rest) = self
            .remaining
            .split_first()
            .ok_or(ReportRefusal::Truncated)?;
        if tag != expected {
            return Err(ReportRefusal::OutOfOrder);
        }
        let length_bytes: [u8; 8] = rest
            .get(..8)
            .ok_or(ReportRefusal::Truncated)?
            .try_into()
            .map_err(|_| ReportRefusal::Truncated)?;
        let length = usize::try_from(u64::from_le_bytes(length_bytes))
            .map_err(|_| ReportRefusal::Truncated)?;
        let payload = rest.get(8..).ok_or(ReportRefusal::Truncated)?;
        let value = payload.get(..length).ok_or(ReportRefusal::Truncated)?;
        self.remaining = payload.get(length..).ok_or(ReportRefusal::Truncated)?;
        Ok(value)
    }

    pub(super) fn finish(self) -> Result<(), ReportRefusal> {
        if self.remaining.is_empty() {
            Ok(())
        } else {
            Err(ReportRefusal::TrailingBytes)
        }
    }
}
