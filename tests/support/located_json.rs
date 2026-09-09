// SPDX-License-Identifier: AGPL-3.0-only
//! FR-017: typed JSON values with checked provenance from Serde's original borrow.

use quire_contract_ir::SourceSpan;
use quire_spec_language::{formal_source::FormalSource, Diagnostic, Span};
use serde::Deserialize;
use serde_json::value::RawValue;

/// Fixture decoding or original-source correspondence failure.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Serde rejected the original JSON or its requested typed shape.
    #[error("invalid JSON fixture: {0}")]
    Json(#[from] serde_json::Error),
    /// The selected value belongs to a different backing allocation.
    #[error("JSON occurrence was not borrowed from the selected source")]
    ForeignOccurrence,
    /// The native-to-formal source mapping rejected the occurrence.
    #[error("invalid JSON source correspondence: {0}")]
    Source(#[from] Box<Diagnostic>),
}

/// A decoded fixture value and its complete original JSON occurrence.
#[derive(Debug)]
pub struct Located<T> {
    /// Value decoded by Serde without reconstructing its spelling.
    pub value: T,
    /// Checked span of the original complete JSON value.
    pub source: SourceSpan,
}

/// Decode the bounded immutable source, preserving borrowed raw JSON values.
pub fn read<'de, T: Deserialize<'de>>(source: &'de FormalSource) -> Result<T, Error> {
    Ok(serde_json::from_str(source.source().text())?)
}

/// Decode a selected original occurrence and retain its exact full-value span.
pub fn decode<'de, T: Deserialize<'de>>(
    source: &FormalSource,
    raw: &'de RawValue,
) -> Result<Located<T>, Error> {
    let original = source.source().text();
    let selected = raw.get();
    // Address arithmetic only: no pointer is dereferenced or used to construct
    // a slice. Checked source slicing establishes the original borrowed region.
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
