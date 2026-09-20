// SPDX-License-Identifier: AGPL-3.0-or-later
//! O-16 kernel outcomes: `Outcome<T>`, `Undefined`, `Refusal`, `Stop`.
//!
//! A false Boolean is a completed value, never a refusal. `requires-bound`
//! and `unsupported` are per-item provider dispositions, not evaluator
//! outcomes, so they have no variant here.
//!
//! Cut from QSL `value::outcome` per ADR-013 O-16/O-17: the kernel `Refusal`
//! carries only its own typed cause. Two variants are dropped against the
//! original because their payload is not a kernel type:
//!
//! - `Refusal::WrongSnapshot(WrongSnapshotCause)`: `WrongSnapshotCause` is
//!   QSL `value::expression`'s own closed cause set for a `pre(..)` anchor
//!   mismatch, a QSL `check`/evaluator concept, not a kernel one.
//! - `Refusal::Model(ModelQueryRefusal)`: `ModelQueryRefusal` carries
//!   `crate::diagnostic::Code`, QSL's `diagnostic` catalog (ADR-013 O-17
//!   explicitly keeps `CatalogCode`/category out of the kernel; that is
//!   S-5's job, not S-1's).
//!
//! The category-mapping table and `FamilyOutcome`/`FamilyRefusal` that union
//! several evaluators' outcomes into one reported shape are QSL concepts
//! layered on top of this and are not kernel (ADR-013 O-16).
//!
//! **M-4: `Undefined::PreconditionFalse(PreconditionFailure)` is dropped**
//! against the original (it carried `{ operation, selected, receiver }`: the
//! called member name, the selected redefinition candidate's identity as a
//! bare `String`, and the receiver reference). Choosing among several
//! redefinition candidates by the receiver's most-specific runtime type is
//! family dispatch, and T-6 is explicit that family-dispatch causes are
//! never kernel causes: resolving *which* candidate linked, and reporting
//! that its precondition evaluated false, is QSL `model`/`check`'s own
//! concept, layered on top of this module the same way the category-mapping
//! table above it is. Retyping `selected` to `EffectiveId` would still leave
//! a dispatch-resolution cause sitting in the kernel's closed `Undefined`
//! set, so removing the variant, not retyping its payload, is the fix.

use crate::accounting::Incomplete;
use crate::collection::{CardinalityBound, CollectionKind};
use crate::ieee::IeeeFlags;

/// Exactly one of a completed value, undefined, refused or incomplete.
#[derive(Clone, Debug, Eq, PartialEq)]
#[must_use]
pub enum Outcome<T> {
    /// A completed value. `T` itself carries any typed loss where the
    /// operation has one (e.g. `DecimalResult::loss`); `Outcome` does not
    /// separately carry [`crate::Location`]/[`crate::Origin`] provenance --
    /// M-3, correcting a previous, false claim here. Nothing in this crate
    /// wires the two together: a caller that wants a completed value's
    /// occurrence provenance holds it itself, the way a call locus is
    /// already the caller's own concept (see `PreconditionFailure`'s former
    /// doc comment, now removed as M-4).
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
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum Undefined {
    /// A divisor is (normalized) zero.
    DivisionByZero,
    /// An IEEE NaN or infinity has no exact value.
    IeeeNotFinite,
    /// `reduce` over an empty collection has no value. Only direct kernel
    /// evaluation of an unlinked expression can meet it.
    EmptyReduction,
    /// `value(e)` of `none`. Only direct kernel evaluation of an unlinked
    /// expression can meet it.
    NoneValue,
    /// A population-lookup query's `absent-key` reason: the queried
    /// reference is not a member of the bound population, and the query
    /// names no mathematical value for that case.
    AbsentKey,
}

/// Why a defined result is refused. Refusals never carry the refused value.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Refusal {
    /// Strict `exact` rounding would discard a nonzero digit.
    InexactDecimal,
    /// The normalized decimal coefficient is outside the target domain.
    DecimalOutOfDomain,
    /// At least one member of a quotient/remainder pair is outside the
    /// consumer domain; neither member is exposed.
    DivisionPairOutOfDomain {
        /// Whether the quotient is a domain member.
        quotient_admitted: bool,
        /// Whether the remainder is a domain member.
        remainder_admitted: bool,
    },
    /// The Euclidean `mod` remainder is outside the consumer domain.
    ModuloOutOfDomain,
    /// The profile length (scalars, or bytes for `binary-utf8`) is outside
    /// the declared `Text[min,max; profile]` bounds.
    TextLengthOutOfDomain,
    /// An exact conversion or arithmetic result is outside the target
    /// integer domain.
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
    /// A comparison met two references of different universes.
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
        /// The formed bound count: occurrences for a sequence or bag,
        /// members for a set or ordered set.
        count: u64,
    },
    /// A checked-program invariant failed during evaluation; unreachable
    /// for an admitted program.
    CheckedInvariant,
}

impl Refusal {
    /// The closed `refused { code }` spelling, where the language defines
    /// one. The kernel names only its own codes; a caller mapping this to a
    /// wider catalog (QSL `diagnostic`, ADR-013 O-17) does so above the
    /// kernel.
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

    /// The closed cause tag, where the code has one.
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
    /// The cause tag.
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

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;

    use super::*;

    /// TC-317 (H-7/H-8, strengthened): `Outcome::completed` returns the
    /// value for `Completed` and `None` for every other variant --
    /// `Undefined`, `Refused` and `Incomplete` are each exercised, not just
    /// `Undefined` as before.
    #[trace("TC-317")]
    #[test]
    fn tc_317_completed_extracts_only_the_completed_variant() {
        use crate::accounting::{ChargePoint, Incomplete, LimitKind};
        use crate::integer::Integer;

        assert_eq!(Outcome::Completed(1).completed(), Some(1));
        assert_eq!(
            Outcome::<i32>::Undefined(Undefined::DivisionByZero).completed(),
            None
        );
        assert_eq!(
            Outcome::<i32>::Refused(Refusal::CheckedInvariant).completed(),
            None
        );
        assert_eq!(
            Outcome::<i32>::Incomplete(Incomplete {
                limit_kind: LimitKind::IntegerBits,
                limit: 0,
                consumed: 0,
                next_charge: Integer::one(),
                charge_point: ChargePoint::IntegerArithmeticOperands,
            })
            .completed(),
            None
        );
    }

    /// TC-318: `CardinalityOutOfBound` is the only refusal with a `code`,
    /// and its `cause` names which bound side was violated.
    #[trace("TC-318")]
    #[test]
    fn tc_318_cardinality_refusal_names_code_and_cause() {
        let bound = CardinalityBound::new(0, 1).unwrap();
        let refusal = Refusal::CardinalityOutOfBound {
            violation: BoundViolation::AboveMaximum,
            kind: CollectionKind::Sequence,
            bound,
            count: 2,
        };
        assert_eq!(refusal.code(), Some("cardinality_out_of_bound"));
        assert_eq!(refusal.cause(), Some("above-maximum"));
    }
}
