// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-014 TR-2: the position of a represented temporal trace, and its wire
//! form `qsl_replay::TracePosition`, decimal ASCII with no leading zero.
//!
//! This crate sits below `qsl-replay`, so it owns the typed position and the
//! decimal descriptor; wrapping the descriptor in `TracePosition` is the
//! replay hook's step. Only the TemporalTrace evaluate hook decodes a
//! descriptor (FR-327 "Positions").

use qsl_foundation::CatalogCode;

/// A zero-based index into the represented trace, prefix first, then loop
/// (ADR-014 TR-2).
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct TemporalPosition(u64);

impl TemporalPosition {
    /// The position at `index`.
    pub const fn new(index: u64) -> Self {
        Self(index)
    }

    /// The zero-based index.
    pub const fn index(self) -> u64 {
        self.0
    }

    /// The wire descriptor: decimal ASCII with no leading zero, `0` for
    /// position zero.
    pub fn descriptor(self) -> String {
        self.0.to_string()
    }

    /// The position a wire `descriptor` names (FR-327 "Positions").
    #[qsl_attrs::string_edge]
    pub fn decode(descriptor: &str) -> Result<Self, PositionRefusal> {
        let bytes = descriptor.as_bytes();
        if bytes.is_empty() || !bytes.iter().all(u8::is_ascii_digit) {
            return Err(PositionRefusal::NotDecimal);
        }
        if bytes.len() > 1 && bytes.first() == Some(&b'0') {
            return Err(PositionRefusal::LeadingZero);
        }
        descriptor
            .parse::<u64>()
            .map(Self)
            .map_err(|_| PositionRefusal::Overflow)
    }
}

/// Why a position descriptor, or a decoded position, is refused. Each is
/// `invalid_runtime_input`/`invalid-value` (FR-327 "Positions").
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub enum PositionRefusal {
    /// The descriptor is empty or holds a byte that is not an ASCII digit.
    #[error("a trace position is decimal ASCII digits")]
    NotDecimal,
    /// The descriptor has a leading zero.
    #[error("a trace position has no leading zero")]
    LeadingZero,
    /// The descriptor exceeds `u64`.
    #[error("a trace position exceeds u64")]
    Overflow,
    /// The decoded position lies outside the represented trace.
    #[error("trace position {position:?} lies outside the represented trace")]
    OutsideTrace {
        /// The refused position.
        position: TemporalPosition,
    },
}

impl PositionRefusal {
    /// The catalog code, `invalid_runtime_input`/`invalid-value` for each
    /// cause.
    pub fn catalog_code(&self) -> CatalogCode {
        match self {
            Self::NotDecimal | Self::LeadingZero | Self::Overflow | Self::OutsideTrace { .. } => {
                CatalogCode::new("invalid_runtime_input", "invalid-value")
            }
        }
    }
}
