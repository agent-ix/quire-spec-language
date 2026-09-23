// SPDX-License-Identifier: AGPL-3.0-or-later
//! The four distinct evaluator outcomes of AD-005.
//!
//! A false Boolean is a completed value, never a refusal. `requires-bound` and
//! `unsupported` are per-item I13 provider dispositions, not evaluator outcomes,
//! so they have no variant here.

use super::reference::ObjectReference;
use quire_exact::{BoundViolation, CardinalityBound, CollectionKind, IeeeFlags, Incomplete};

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
    /// FR-090-AC-1: this QSL kernel-copy outcome as the
    /// `quire_exact::Outcome<T>` that `FamilyOutcome::Evaluated` and
    /// `EvalOutcome::Kernel` carry (ADR-013 O-16). A total, one-to-one
    /// re-tagging: the copy's `Undefined` and `Refusal` have exactly the
    /// kernel's variants. Remaining work: QSL-131 deletes the copy.
    pub(crate) fn into_kernel(self) -> quire_exact::Outcome<T> {
        match self {
            Self::Completed(value) => quire_exact::Outcome::Completed(value),
            Self::Undefined(undefined) => quire_exact::Outcome::Undefined(undefined.into_kernel()),
            Self::Refused(refusal) => quire_exact::Outcome::Refused(refusal.into_kernel()),
            Self::Incomplete(record) => quire_exact::Outcome::Incomplete(record),
        }
    }
}

impl Undefined {
    fn into_kernel(self) -> quire_exact::Undefined {
        match self {
            Self::DivisionByZero => quire_exact::Undefined::DivisionByZero,
            Self::IeeeNotFinite => quire_exact::Undefined::IeeeNotFinite,
            Self::EmptyReduction => quire_exact::Undefined::EmptyReduction,
            Self::NoneValue => quire_exact::Undefined::NoneValue,
        }
    }
}

impl Refusal {
    fn into_kernel(self) -> quire_exact::Refusal {
        match self {
            Self::InexactDecimal => quire_exact::Refusal::InexactDecimal,
            Self::DecimalOutOfDomain => quire_exact::Refusal::DecimalOutOfDomain,
            Self::DivisionPairOutOfDomain {
                quotient_admitted,
                remainder_admitted,
            } => quire_exact::Refusal::DivisionPairOutOfDomain {
                quotient_admitted,
                remainder_admitted,
            },
            Self::ModuloOutOfDomain => quire_exact::Refusal::ModuloOutOfDomain,
            Self::TextLengthOutOfDomain => quire_exact::Refusal::TextLengthOutOfDomain,
            Self::IntegerOutOfDomain => quire_exact::Refusal::IntegerOutOfDomain,
            Self::RationalOutOfDomain => quire_exact::Refusal::RationalOutOfDomain,
            Self::IeeeNotExact { would_be } => quire_exact::Refusal::IeeeNotExact { would_be },
            Self::IeeeNanPayloadNotRepresentable => {
                quire_exact::Refusal::IeeeNanPayloadNotRepresentable
            }
            Self::IeeeRationalOutOfDomain => quire_exact::Refusal::IeeeRationalOutOfDomain,
            Self::ForeignReference => quire_exact::Refusal::ForeignReference,
            Self::CardinalityOutOfBound {
                violation,
                kind,
                bound,
                count,
            } => quire_exact::Refusal::CardinalityOutOfBound {
                violation,
                kind,
                bound,
                count,
            },
            Self::CheckedInvariant => quire_exact::Refusal::CheckedInvariant,
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

/// Why an operation is undefined. Holds kernel reasons only: the
/// `StateModel` undefined results are family-owned (ADR-013 O-16, T-6).
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum Undefined {
    /// A divisor is (normalized) zero.
    DivisionByZero,
    /// An IEEE NaN or infinity has no exact value.
    IeeeNotFinite,
    /// FR-145: `reduce` over an empty collection has no value. Only direct
    /// kernel evaluation of an unlinked expression can meet it.
    EmptyReduction,
    /// `value(e)` of `none`. Only direct kernel evaluation of an unlinked
    /// expression can meet it.
    NoneValue,
}

/// The `precondition-false` payload (`native-diagnostics.md`): the called
/// effective operation, the selected method's effective identity, the
/// receiver reference and the call locus. The call locus is the evaluator's
/// own [`crate::value::Evaluation::location`], not
/// repeated here.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct PreconditionFailure {
    /// The called effective operation's unqualified member name.
    pub operation: String,
    /// The selected method's effective identity: the redefinition candidate
    /// the receiver's most-specific runtime type actually linked to.
    pub selected: String,
    /// The receiver reference the call was made on.
    pub receiver: ObjectReference,
}

/// Why a defined result is refused. Refusals never carry the refused value.
/// Holds kernel causes only: the evaluation-time `wrong_snapshot` and
/// model-query refusals are family-owned (ADR-013 O-16, T-6), and an
/// unresolved or mismatched population argument is refused at admission
/// (`CallFailure::Input`) and is an `InternalFault` inside S6a (ADR-013 T-4).
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

/// Internal early-exit carrier, always converted into an [`Outcome`] by
/// [`Outcome::from_stop`]. An S6a invariant break (FR-090-AC-3/AC-10,
/// ADR-013 T-4) is never a `Stop`: this type is shared by roughly two dozen
/// `value/*.rs` computations that all convert through `from_stop`, so a
/// fault variant here would be reachable from every one of them with no
/// compiler-checked guarantee that `Machine::run` intercepts it first (PR
/// #334 review round 2, finding N1) -- `Machine`'s own crate-private `Halt`
/// (`value::expression::evaluate.rs`) carries a fault instead, and no
/// `From<Halt> for Stop`/`Outcome` conversion exists, so a fault is
/// unrepresentable here by construction, not by convention.
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
