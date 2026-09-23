// SPDX-License-Identifier: AGPL-3.0-or-later
//! Scalar comparison operators and the type-checking refusals that precede
//! evaluation.
//!
//! An [`IllTyped`] comparison is `refused { code: ill_typed }` at type checking;
//! it is not a peer [`crate::outcome::Outcome`] and consumes no charge.
//!
//! Ported verbatim from QSL `value::comparison` as part of QSL#213 S-1
//! (ADR-011 X-1); no edge needed cutting.
//!
//! [`ComparisonOperator::holds`] is `pub`: it is a pure decision over an
//! already-computed `Ordering`, so exposing it exposes no unmetered
//! computation.

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
    ///
    /// `pub`, not `pub(crate)` (QSL-131 K1): `quire_spec_language::value::
    /// text`'s own `compare`, still local because it returns this crate's
    /// own `Outcome`, calls it directly. It is a pure decision over an
    /// already-computed `Ordering`, so widening charges or exposes nothing.
    pub fn holds(self, ordering: Ordering) -> bool {
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
    /// FR-272 `type-mismatch`: the operands or a value and its expected
    /// position have no common declared type and no admitted conversion.
    TypeMismatch,
    /// FR-272 `operator-ineligible`: the operator is not defined for the
    /// operand type, such as `=` on an IEEE-bearing type.
    OperatorIneligible,
    /// FR-272 `ambiguous-literal`: a collection or `rational` literal has no
    /// unique expected type.
    AmbiguousLiteral,
}

impl IllTypedCause {
    /// The closed FR-272 `cause` tag, for the causes the complete-V1 cause
    /// table names.
    pub fn tag(self) -> Option<&'static str> {
        match self {
            Self::TypeMismatch => Some("type-mismatch"),
            Self::OperatorIneligible => Some("operator-ineligible"),
            Self::AmbiguousLiteral => Some("ambiguous-literal"),
            Self::DistinctTextProfiles
            | Self::DistinctEnumDeclarations
            | Self::UnorderedEnumOrdering
            | Self::IncompatibleDimensions
            | Self::AffineUnitArithmetic
            | Self::DistinctUnits
            | Self::MalformedDecimalType
            | Self::DistinctIeeeWidths
            | Self::IeeeWithExactOperand
            | Self::IeeeToNonRationalExact => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cmp::Ordering;

    use ix_trace_rs::trace;

    use super::*;

    /// TC-335: each comparison operator decides its Boolean from an exact
    /// ordering exactly as its FR-149 name reads.
    #[trace("TC-335")]
    #[test]
    fn tc_335_holds_matches_the_operator_name() {
        assert!(ComparisonOperator::Less.holds(Ordering::Less));
        assert!(!ComparisonOperator::Less.holds(Ordering::Equal));
        assert!(ComparisonOperator::GreaterOrEqual.holds(Ordering::Equal));
        assert!(!ComparisonOperator::Equal.holds(Ordering::Greater));
    }

    /// TC-336: only the three FR-272-catalogued causes have a `tag`; every
    /// other cause is a kernel-internal reason with no closed spelling.
    #[trace("TC-336")]
    #[test]
    fn tc_336_only_catalogued_causes_have_a_tag() {
        assert_eq!(IllTypedCause::TypeMismatch.tag(), Some("type-mismatch"));
        assert_eq!(IllTypedCause::DistinctUnits.tag(), None);
    }
}
