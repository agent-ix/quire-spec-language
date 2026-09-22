// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-017/025: typed JSON values with checked provenance from Serde's original borrow.

use crate::formal_source::{FormalSource, FormalSourceError};
use crate::Span;
use quire_contract_ir::SourceSpan;
use serde::Deserialize;
use serde_json::value::RawValue;

/// JSON decoding or original-source correspondence failure.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The source exceeded the effective read ceiling before JSON decoding.
    #[error("JSON source has {actual} bytes, exceeding the {maximum}-byte limit")]
    ByteLimit {
        /// Original source length.
        actual: usize,
        /// Effective caller-lowered ceiling, at most 1 MiB.
        maximum: usize,
    },
    /// Serde rejected the original JSON or its requested typed shape.
    #[error("invalid JSON source: {0}")]
    Json(#[from] serde_json::Error),
    /// The selected value belongs to a different backing allocation.
    #[error("JSON occurrence was not borrowed from the selected source")]
    ForeignOccurrence,
    /// The native-to-formal source mapping rejected the occurrence.
    #[error("invalid JSON source correspondence: {0}")]
    Source(#[from] Box<FormalSourceError>),
}

/// A decoded source value and its complete original JSON occurrence.
#[derive(Debug)]
pub struct Located<T> {
    /// Value decoded by Serde without reconstructing its spelling.
    pub value: T,
    /// Checked span of the original complete JSON value.
    pub source: SourceSpan,
}

/// Decode immutable source under an explicit byte ceiling, capped at 1 MiB.
/// Raw values borrow this exact source allocation; Serde's recursion guard remains enabled.
pub fn read<'de, T: Deserialize<'de>>(
    source: &'de FormalSource,
    maximum_bytes: usize,
) -> Result<T, Error> {
    let actual = source.source().text().len();
    let maximum = maximum_bytes.min(1_048_576);
    if actual > maximum {
        return Err(Error::ByteLimit { actual, maximum });
    }
    Ok(serde_json::from_str(source.source().text())?)
}

/// Decode a selected original occurrence and retain its exact full-value span.
///
/// `raw` must be borrowed directly from this source's text, as returned by
/// [`read`]. An owned `Box<RawValue>`, a reconstructed value, or a borrow from
/// another source returns [`Error::ForeignOccurrence`], even for identical bytes.
pub fn decode<'de, T: Deserialize<'de>>(
    source: &FormalSource,
    raw: &'de RawValue,
) -> Result<Located<T>, Error> {
    let original = source.source().text();
    let selected = raw.get();
    // Address arithmetic only: no pointer is dereferenced or used to construct
    // a slice. Checked source slicing establishes the original borrowed region.
    // Both nonempty allocations are live: a foreign allocation cannot overlap
    // the original address range. The range check therefore excludes foreign
    // allocations on either side; equal text alone does not admit an occurrence.
    let start = selected
        .as_ptr()
        .addr()
        .checked_sub(original.as_ptr().addr())
        .ok_or(Error::ForeignOccurrence)?;
    let end = start
        .checked_add(selected.len())
        .ok_or(Error::ForeignOccurrence)?;
    if original.get(start..end) != Some(selected) {
        return Err(Error::ForeignOccurrence);
    }
    let span = source.to_ir(source.source(), Span { start, end })?;
    Ok(Located {
        value: serde_json::from_str(selected)?,
        source: span,
    })
}
