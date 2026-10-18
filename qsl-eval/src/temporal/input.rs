// SPDX-License-Identifier: AGPL-3.0-or-later
//! The shape of the trace a temporal clause is evaluated over (FR-327
//! "Inputs", FR-329 "Lasso shape").
//!
//! The evaluator reads a trace only through its positions: the observation at
//! each position is the caller's (FR-106), reached through
//! [`TemporalLetters`](super::TemporalLetters). The shape is what the
//! evaluator itself needs: how many positions are represented and, for a
//! lasso, where the loop re-enters.

use super::position::{PositionRefusal, TemporalPosition};

/// The represented positions of a trace, prefix first, then loop (ADR-014
/// TR-2).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TemporalTrace {
    /// `len >= 1` observations at positions `0..len`. Under a bounded
    /// profile it is a closed trace; under `quire.temporal.infinite-trace/v1`
    /// it is a finite prefix of an unknown continuation (FR-328).
    Finite {
        /// The number of represented positions.
        len: u64,
    },
    /// A lasso (FR-329): `prefix` positions, then `loop_len >= 1` positions
    /// whose end re-enters position `prefix`.
    Lasso {
        /// The number of positions before the loop.
        prefix: u64,
        /// The number of positions in the loop.
        loop_len: u64,
    },
}

impl TemporalTrace {
    /// The number of represented positions, or `None` when a lasso's
    /// prefix and loop together exceed `u64`.
    pub fn represented(self) -> Option<u64> {
        match self {
            Self::Finite { len } => Some(len),
            Self::Lasso { prefix, loop_len } => prefix.checked_add(loop_len),
        }
    }

    /// The position a wire `descriptor` names on this trace (FR-327
    /// "Positions"): decoded, then reconstructed. A finite trace has no
    /// position past its last; a lasso reaches every position by wrapping
    /// (ADR-018 SM-8).
    pub fn locate(self, descriptor: &str) -> Result<TemporalPosition, PositionRefusal> {
        self.admit(TemporalPosition::decode(descriptor)?)
    }

    /// `position`, when this trace represents it.
    pub fn admit(self, position: TemporalPosition) -> Result<TemporalPosition, PositionRefusal> {
        match self {
            Self::Finite { len } if position.index() >= len => {
                Err(PositionRefusal::OutsideTrace { position })
            }
            Self::Finite { .. } | Self::Lasso { .. } => Ok(position),
        }
    }
}
