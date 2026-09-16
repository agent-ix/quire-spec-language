// SPDX-License-Identifier: AGPL-3.0-or-later
//! Scalar comparison operators and the type-checking refusals that precede
//! evaluation.
//!
//! An [`IllTyped`] comparison is `refused { code: ill_typed }` at type checking;
//! it is not a peer [`Outcome`](super::Outcome) and consumes no charge.

use std::cmp::Ordering;

/// A comparison request.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ComparisonOperator {
    /// `==`.
    Equal,
    /// `!=`.
    NotEqual,
    /// `<`.
    Less,
    /// `<=`.
    LessOrEqual,
    /// `>`.
    Greater,
    /// `>=`.
    GreaterOrEqual,
}

impl ComparisonOperator {
    /// Every operator.
    pub const ALL: [Self; 6] = [
        Self::Equal,
        Self::NotEqual,
        Self::Less,
        Self::LessOrEqual,
        Self::Greater,
        Self::GreaterOrEqual,
    ];

    /// Whether the operator needs an ordering rather than only identity.
    pub fn is_ordering(self) -> bool {
        match self {
            Self::Equal | Self::NotEqual => false,
            Self::Less | Self::LessOrEqual | Self::Greater | Self::GreaterOrEqual => true,
        }
    }

    /// Decide the operator over an exact ordering of its operands.
    pub(crate) fn holds(self, ordering: Ordering) -> bool {
        match self {
            Self::Equal => ordering.is_eq(),
            Self::NotEqual => ordering.is_ne(),
            Self::Less => ordering.is_lt(),
            Self::LessOrEqual => ordering.is_le(),
            Self::Greater => ordering.is_gt(),
            Self::GreaterOrEqual => ordering.is_ge(),
        }
    }
}

/// The type-checking refusal `refused { code: ill_typed }`.
///
/// It is also the refusal of a malformed declared type.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, thiserror::Error)]
#[error("ill_typed: {cause:?}")]
pub struct IllTyped {
    /// Why the comparison has no common type.
    pub cause: IllTypedCause,
}

impl IllTyped {
    /// Stable refusal code.
    pub const CODE: &'static str = "ill_typed";
}

/// The typed reason for an [`IllTyped`] refusal.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum IllTypedCause {
    /// The operands use different text profiles and no explicit common-profile
    /// conversion was applied.
    DistinctTextProfiles,
    /// The operands are members of different enum declaration nodes.
    DistinctEnumDeclarations,
    /// An ordering operator was applied to an unordered enumeration.
    UnorderedEnumOrdering,
    /// A `Decimal[lo, hi; smin, smax; mode]` declaration has `lo > hi`,
    /// `smin > smax` or `smax > u32::MAX`.
    MalformedDecimalType,
}
