// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-315 Inputs: which proof query to encode.

/// The proof query to encode: V-2's bounded unrolling or one of EN-3's
/// `k`-induction cases (ADR-018 PC-6).
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SmtQueryKind {
    /// `depth` steps from the initial states, for a violation or a
    /// definedness failure at some position `0..=depth`.
    Unrolling {
        /// The number of steps.
        depth: u64,
    },
    /// EN-3's base case: as [`Self::Unrolling`], for a safety item.
    InductionBase {
        /// The number of steps.
        depth: u64,
    },
    /// EN-3's step case: `depth` steps from no initial state, with the
    /// property holding before the last.
    InductionStep {
        /// The number of steps.
        depth: u64,
    },
}

impl SmtQueryKind {
    /// The number of steps, so the query declares `depth + 1` state copies.
    pub const fn depth(self) -> u64 {
        match self {
            Self::Unrolling { depth }
            | Self::InductionBase { depth }
            | Self::InductionStep { depth } => depth,
        }
    }

    /// Whether the query asserts the initial-state predicate on copy 0
    /// (`Unrolling` and `InductionBase`).
    pub const fn starts_initial(self) -> bool {
        !matches!(self, Self::InductionStep { .. })
    }

    /// The kind's spelling in a refusal.
    pub const fn name(self) -> &'static str {
        match self {
            Self::Unrolling { .. } => "unrolling",
            Self::InductionBase { .. } => "induction-base",
            Self::InductionStep { .. } => "induction-step",
        }
    }
}
