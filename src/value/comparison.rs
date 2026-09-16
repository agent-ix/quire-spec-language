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
    /// Quantity operands, or a conversion's source and target, have
    /// incompatible dimensions.
    IncompatibleDimensions,
    /// Arithmetic was applied to a unit whose composed root offset is nonzero.
    AffineUnitArithmetic,
    /// Addition, subtraction or comparison of operands in different units;
    /// conversion never occurs implicitly.
    DistinctUnits,
    /// A `Decimal[lo, hi; smin, smax; mode]` declaration has `lo > hi`,
    /// `smin > smax` or `smax > u32::MAX`.
    MalformedDecimalType,
    /// IEEE operands have different widths and no explicit width conversion
    /// was selected.
    DistinctIeeeWidths,
    /// An IEEE operation or comparison mixes an IEEE operand with an exact
    /// integer, rational or decimal operand.
    IeeeWithExactOperand,
    /// An IEEE value is converted directly to `Decimal`, `Integer` or
    /// `Int[..]`; only a `Rational[..]` target is defined.
    IeeeToNonRationalExact,
    /// Equality operands are of different value kinds, or of option or
    /// collection types with different declared payload/element types.
    DistinctValueTypes,
    /// Record, tuple, variant or reference operands name different
    /// declarations; equal shapes never create a common type.
    DistinctDeclarations,
    /// No explicitly declared lossless conversion reaches the requested
    /// comparison type.
    NoLosslessConversion,
    /// A collection function or query value is not of the source element
    /// type, or a reduction's accumulator is not the element type.
    FunctionParameterType,
    /// A filter predicate does not return `Boolean`.
    PredicateNotBoolean,
    /// A fold identity is not a member of the accumulator type.
    FoldIdentityType,
    /// A set or bag fold or reduction uses a function not declared both
    /// commutative and associative.
    UnorderedFoldRequiresCommutativeAssociative,
    /// A collection conversion discards a property that was not accepted.
    UnacceptedCollectionLoss,
    /// Ordering an unordered collection needs a total element key the element
    /// type does not supply.
    NoTotalElementKey,
    /// The flatten source's elements are not collections, or the
    /// outer/inner kind combination is not defined.
    UnsupportedFlattenSource,
}
