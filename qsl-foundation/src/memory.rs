// SPDX-License-Identifier: AGPL-3.0-or-later
//! The memory model names of a `parallel`'s `memory` clause (FR-219,
//! ADR-025 MM-1): what S2 parses, S3 records and `route` filters candidates
//! by (FR-226), so it sits below every layer that names it.

/// A `parallel`'s memory model name (FR-219): `sc`, `tso` or `ra`.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum MemoryModelName {
    /// `memory sc`, the interleaving meaning and the default.
    Sc,
    /// `memory tso`.
    Tso,
    /// `memory ra`.
    Ra,
}

impl MemoryModelName {
    /// The model's spelling in source.
    pub fn label(self) -> &'static str {
        match self {
            Self::Sc => "sc",
            Self::Tso => "tso",
            Self::Ra => "ra",
        }
    }

    /// Whether the model is `tso` or `ra`.
    pub fn is_weak(self) -> bool {
        self != Self::Sc
    }
}
