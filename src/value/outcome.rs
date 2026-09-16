// SPDX-License-Identifier: AGPL-3.0-or-later
//! The four distinct evaluator outcomes of AD-005.
//!
//! A false Boolean is a completed value, never a refusal. `requires-bound` and
//! `unsupported` are per-item I13 provider dispositions, not evaluator outcomes,
//! so they have no variant here.

use super::accounting::Incomplete;
use super::collection::{CardinalityBound, CollectionKind};
use super::ieee::IeeeFlags;

/// Exactly one of a completed value, undefined, refused or incomplete.
#[derive(Clone, Debug, Eq, PartialEq)]
#[must_use]
pub enum Outcome<T> {
    /// A completed value with its provenance and any typed loss.
    Completed(T),
    /// The operation has no mathematical value.
    Undefined(Undefined),
    /// The operation is defined but its result is not admitted.
    Refused(Refusal),
    /// A named charge was unavailable; no partial value exists.
    Incomplete(Incomplete),
}

impl<T> Outcome<T> {
    /// The completed value, if any.
    pub fn completed(self) -> Option<T> {
        match self {
            Self::Completed(value) => Some(value),
            Self::Undefined(_) | Self::Refused(_) | Self::Incomplete(_) => None,
        }
    }
}

impl<T> Outcome<T> {
    pub(crate) fn from_stop(result: Result<T, Stop>) -> Self {
        match result {
            Ok(value) => Self::Completed(value),
            Err(Stop::Undefined(reason)) => Self::Undefined(reason),
            Err(Stop::Refused(reason)) => Self::Refused(reason),
            Err(Stop::Incomplete(record)) => Self::Incomplete(record),
        }
    }

    pub(crate) fn into_stop(self) -> Result<T, Stop> {
        match self {
            Self::Completed(value) => Ok(value),
            Self::Undefined(reason) => Err(Stop::Undefined(reason)),
            Self::Refused(reason) => Err(Stop::Refused(reason)),
            Self::Incomplete(record) => Err(Stop::Incomplete(record)),
        }
    }
}

/// Why an operation is undefined.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Undefined {
    /// A divisor is (normalized) zero.
    DivisionByZero,
    /// An IEEE NaN or infinity has no exact value.
    IeeeNotFinite,
}

/// Why a defined result is refused. Refusals never carry the refused value.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Refusal {
    /// Strict `exact` rounding would discard a nonzero digit.
    InexactDecimal,
    /// The normalized decimal coefficient is outside the target domain.
    DecimalOutOfDomain,
    /// At least one member of a quotient/remainder pair is outside the consumer
    /// domain; neither member is exposed.
    DivisionPairOutOfDomain {
        /// Whether the quotient is a domain member.
        quotient_admitted: bool,
        /// Whether the remainder is a domain member.
        remainder_admitted: bool,
    },
    /// The Euclidean `mod` remainder is outside the consumer domain.
    ModuloOutOfDomain,
    /// The profile length (scalars, or bytes for `binary-utf8`) is outside the
    /// declared `Text[min,max; profile]` bounds.
    TextLengthOutOfDomain,
    /// An exact conversion or arithmetic result is outside the target integer
    /// domain.
    IntegerOutOfDomain,
    /// An exact rational arithmetic result is outside its `Rational[..]`
    /// result domain.
    RationalOutOfDomain,
    /// Strict IEEE `exact` found an inexact, overflowing or tiny-and-inexact
    /// result; only its would-be flags are reported, never rounded bits.
    IeeeNotExact {
        /// The flags the rounded result would have raised.
        would_be: IeeeFlags,
    },
    /// A NaN payload does not fit the explicit conversion's target width.
    IeeeNanPayloadNotRepresentable,
    /// An exact rational converted from an IEEE value is outside the
    /// `Rational[..]` target domain.
    IeeeRationalOutOfDomain,
    /// An FR-149 comparison met two references of different universes.
    ForeignReference,
    /// A formed collection's bound count is outside its declared bound; no
    /// collection is materialized.
    CardinalityOutOfBound {
        /// Which side of the bound is violated.
        violation: BoundViolation,
        /// The collection kind of the declared type.
        kind: CollectionKind,
        /// The declared inclusive bound.
        bound: CardinalityBound,
        /// The formed bound count: occurrences for a sequence or bag, members
        /// for a set or ordered set.
        count: u64,
    },
    /// A checked-program invariant failed during evaluation; unreachable for
    /// an admitted program.
    CheckedInvariant,
}

impl Refusal {
    /// The closed `refused { code }` spelling, where the language defines one.
    pub fn code(self) -> Option<&'static str> {
        match self {
            Self::IeeeNanPayloadNotRepresentable => Some("ieee_nan_payload_not_representable"),
            Self::IeeeRationalOutOfDomain => Some("ieee_rational_out_of_domain"),
            Self::ForeignReference => Some("foreign_reference"),
            Self::CardinalityOutOfBound { .. } => Some("cardinality_out_of_bound"),
            Self::InexactDecimal
            | Self::DecimalOutOfDomain
            | Self::DivisionPairOutOfDomain { .. }
            | Self::ModuloOutOfDomain
            | Self::TextLengthOutOfDomain
            | Self::IntegerOutOfDomain
            | Self::RationalOutOfDomain
            | Self::IeeeNotExact { .. }
            | Self::CheckedInvariant => None,
        }
    }

    /// The closed FR-272 `cause` tag, where the code has one.
    pub fn cause(self) -> Option<&'static str> {
        match self {
            Self::CardinalityOutOfBound { violation, .. } => Some(violation.as_str()),
            Self::InexactDecimal
            | Self::DecimalOutOfDomain
            | Self::DivisionPairOutOfDomain { .. }
            | Self::ModuloOutOfDomain
            | Self::TextLengthOutOfDomain
            | Self::IntegerOutOfDomain
            | Self::RationalOutOfDomain
            | Self::IeeeNotExact { .. }
            | Self::IeeeNanPayloadNotRepresentable
            | Self::IeeeRationalOutOfDomain
            | Self::ForeignReference
            | Self::CheckedInvariant => None,
        }
    }
}

/// The side of a cardinality bound a formed collection violates.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum BoundViolation {
    /// `below-minimum`.
    BelowMinimum,
    /// `above-maximum`.
    AboveMaximum,
}

impl BoundViolation {
    /// The FR-272 cause tag.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::BelowMinimum => "below-minimum",
            Self::AboveMaximum => "above-maximum",
        }
    }
}

/// Internal early-exit carrier converted into [`Outcome`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum Stop {
    Undefined(Undefined),
    Refused(Refusal),
    Incomplete(Incomplete),
}

impl From<Incomplete> for Stop {
    fn from(record: Incomplete) -> Self {
        Self::Incomplete(record)
    }
}
