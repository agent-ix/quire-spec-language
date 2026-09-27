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
//! The category-mapping table and `FamilyOutcome`/`FamilyResult` that union
//! several evaluators' outcomes into one reported shape are QSL concepts
//! layered on top of this and are not kernel (ADR-013 O-16).
//!
//! **M-4: `Undefined::PreconditionFalse(PreconditionFailure)` is dropped**
//! against the original (it carried `{ operation, selected, receiver }`: the
//! called member name, the selected redefinition candidate's identity as a
//! bare `String`, and the receiver reference). Choosing among several
//! redefinition candidates by the receiver's most-specific runtime type is
//! family dispatch, and T-6 is explicit that family causes are never
//! kernel causes: resolving *which* candidate linked, and reporting
//! that its precondition evaluated false, is QSL `model`/`check`'s own
//! concept, layered on top of this module the same way the category-mapping
//! table above it is. Retyping `selected` to `EffectiveId` would still leave
//! a dispatch-resolution cause sitting in the kernel's closed `Undefined`
//! set, so removing the variant, not retyping its payload, is the fix.

use crate::accounting::Incomplete;
use crate::collection::{CardinalityBound, CollectionKind};
use crate::decimal::DecimalType;
use crate::identity::UniverseId;
use crate::ieee::{IeeeFlags, IeeeWidth};
use crate::integer::IntegerInterval;
use crate::rational::RationalDomain;
use crate::text::TextType;

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
    /// A `sum<N>` seed or running total is not a member of `N`'s domain, so
    /// the fold has no value in `N` (QSpec FR-145). It names no catalog
    /// undefined reason and builds no record. Only a `sum` checked under
    /// `CheckMode::Kernel` can meet it.
    SumOutOfDomain,
}

/// Why a defined result is refused. Refusals never carry the refused value.
///
/// Each of the ten value refusals carries the declared target domain or IEEE
/// width its catalog record renders (FR-096, QSL-245), so the record is built
/// from the variant and never from a message. Bigint domains are boxed, which
/// keeps `Refusal` small; it is `Clone`, not `Copy`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Refusal {
    /// Strict `exact` rounding would discard a nonzero digit.
    InexactDecimal {
        /// The declared target the value was placed at.
        target: InexactTarget,
    },
    /// The normalized decimal coefficient is outside the target domain.
    DecimalOutOfDomain {
        /// The declared `Decimal[..]` target.
        target: Box<DecimalType>,
    },
    /// At least one member of a quotient/remainder pair is outside the
    /// consumer domain; neither member is exposed.
    DivisionPairOutOfDomain {
        /// The bounded consumer's `Int[..]` domain.
        domain: Box<IntegerInterval>,
        /// Whether the quotient is a domain member.
        quotient_admitted: bool,
        /// Whether the remainder is a domain member.
        remainder_admitted: bool,
    },
    /// The Euclidean `mod` remainder is outside the consumer domain.
    ModuloOutOfDomain {
        /// The bounded consumer's `Int[..]` domain.
        domain: Box<IntegerInterval>,
    },
    /// The profile length (scalars, or bytes for `binary-utf8`) is outside
    /// the declared `Text[min,max; profile]` bounds.
    TextLengthOutOfDomain {
        /// The declared text type.
        target: TextType,
    },
    /// An exact conversion or arithmetic result is outside the target
    /// integer domain.
    IntegerOutOfDomain {
        /// The target `Int[..]` domain.
        target: Box<IntegerInterval>,
    },
    /// An exact rational arithmetic result is outside its `Rational[..]`
    /// result domain.
    RationalOutOfDomain {
        /// The `Rational[..]` result domain.
        target: Box<RationalDomain>,
    },
    /// Strict IEEE `exact` found an inexact, overflowing or tiny-and-inexact
    /// result; only its would-be flags are reported, never rounded bits.
    IeeeNotExact {
        /// The result width.
        target: IeeeWidth,
        /// The flags the rounded result would have raised.
        would_be: IeeeFlags,
    },
    /// A NaN payload does not fit the explicit conversion's target width.
    IeeeNanPayloadNotRepresentable {
        /// The conversion's target width.
        target: IeeeWidth,
        /// The conversion's source width.
        source: IeeeWidth,
    },
    /// An exact rational converted from an IEEE value is outside the
    /// `Rational[..]` target domain.
    IeeeRationalOutOfDomain {
        /// The grammar-named `Rational[..]` conversion target.
        target: Box<RationalDomain>,
    },
    /// A comparison met two references of different universes (FR-096: the
    /// `foreign_reference`/`foreign-universe` key-table row, `required` and
    /// `supplied` rendered as lowercase hex). `required` is the universe
    /// already in force, `supplied` the one tested against it: for a bare
    /// equality (`plan_pairs(left, right)`, `equality.rs`), that is the left
    /// operand's universe and the right's, since equality has no "binding"
    /// side and the raise site's operand order settles which is which; for
    /// membership (`collection.rs`'s `member_equal_stop`, both `Contains`
    /// and collection construction's dedup), that is the already-retained
    /// member's or collection's own universe, not the probed candidate's
    /// (QSL-281).
    ForeignReference {
        /// The universe already in force.
        required: UniverseId,
        /// The universe tested against it.
        supplied: UniverseId,
    },
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
    /// The catalog `refused { code }` spelling, `None` only for
    /// `CheckedInvariant`, which is an internal fault and never a refusal
    /// record. The kernel names its own codes; QSL `diagnostic` maps them to
    /// its catalog (ADR-013 O-17).
    pub fn code(&self) -> Option<&'static str> {
        match self {
            Self::InexactDecimal { .. } => Some("inexact_decimal"),
            Self::DecimalOutOfDomain { .. } => Some("decimal_out_of_domain"),
            Self::DivisionPairOutOfDomain { .. } => Some("division_pair_out_of_domain"),
            Self::ModuloOutOfDomain { .. } => Some("modulo_out_of_domain"),
            Self::TextLengthOutOfDomain { .. } => Some("text_length_out_of_domain"),
            Self::IntegerOutOfDomain { .. } => Some("integer_out_of_domain"),
            Self::RationalOutOfDomain { .. } => Some("rational_out_of_domain"),
            Self::IeeeNotExact { .. } => Some("ieee_not_exact"),
            Self::IeeeNanPayloadNotRepresentable { .. } => {
                Some("ieee_nan_payload_not_representable")
            }
            Self::IeeeRationalOutOfDomain { .. } => Some("ieee_rational_out_of_domain"),
            Self::ForeignReference { .. } => Some("foreign_reference"),
            Self::CardinalityOutOfBound { .. } => Some("cardinality_out_of_bound"),
            Self::CheckedInvariant => None,
        }
    }

    /// The closed cause tag its code's catalog row gives, `None` only for
    /// `CheckedInvariant`.
    pub fn cause(&self) -> Option<&'static str> {
        match self {
            Self::InexactDecimal { .. } => Some("nonzero-discarded-digit"),
            Self::DecimalOutOfDomain { .. }
            | Self::ModuloOutOfDomain { .. }
            | Self::TextLengthOutOfDomain { .. }
            | Self::IntegerOutOfDomain { .. }
            | Self::RationalOutOfDomain { .. }
            | Self::IeeeRationalOutOfDomain { .. } => Some("outside-domain"),
            Self::DivisionPairOutOfDomain {
                quotient_admitted,
                remainder_admitted,
                ..
            } => Some(match (quotient_admitted, remainder_admitted) {
                (false, true) => "quotient-outside-domain",
                (true, false) => "remainder-outside-domain",
                // The kernel raises the refusal only when a member is
                // outside, so `(true, true)` cannot occur; it reads as the
                // widest cause rather than inventing a fourth.
                (false, false) | (true, true) => "both-outside-domain",
            }),
            Self::IeeeNotExact { .. } => Some("rounding-required"),
            Self::IeeeNanPayloadNotRepresentable { .. } => Some("payload-exceeds-target"),
            Self::ForeignReference { .. } => Some("foreign-universe"),
            Self::CardinalityOutOfBound { violation, .. } => Some(violation.as_str()),
            Self::CheckedInvariant => None,
        }
    }
}

/// The declared target an inexact decimal placement was refused at.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum InexactTarget {
    /// A `Decimal[lo, hi; smin, smax]` target.
    Decimal(Box<DecimalType>),
    /// An integer target (scale zero), rendered as its declared `Int[lo, hi]`.
    Integer(Box<IntegerInterval>),
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

    /// TC-428 (FR-096-AC-8): every kernel refusal but `CheckedInvariant`
    /// returns the catalog code and cause of its key-table row, and
    /// `CheckedInvariant` returns neither.
    #[trace("TC-428", "FR-096-AC-8")]
    #[test]
    fn tc_428_every_record_building_refusal_names_code_and_cause() {
        use crate::integer::Integer;
        use crate::text::TextProfile;

        let interval = || Box::new(IntegerInterval::spanning(Integer::zero(), Integer::one()));
        let rational = || {
            Box::new(
                RationalDomain::new(
                    IntegerInterval::spanning(Integer::zero(), Integer::one()),
                    IntegerInterval::spanning(Integer::one(), Integer::one()),
                )
                .unwrap(),
            )
        };
        let decimal = || {
            Box::new(
                DecimalType::new(
                    Integer::zero(),
                    Integer::one(),
                    0,
                    0,
                    crate::decimal::RoundingMode::Exact,
                )
                .unwrap(),
            )
        };
        let pair = |quotient_admitted, remainder_admitted| Refusal::DivisionPairOutOfDomain {
            domain: interval(),
            quotient_admitted,
            remainder_admitted,
        };
        let universe = UniverseId::from_digest([0; 32]);
        let cases: [(Refusal, &str, &str); 15] = [
            (
                Refusal::InexactDecimal {
                    target: InexactTarget::Integer(interval()),
                },
                "inexact_decimal",
                "nonzero-discarded-digit",
            ),
            (
                Refusal::DecimalOutOfDomain { target: decimal() },
                "decimal_out_of_domain",
                "outside-domain",
            ),
            (
                pair(false, true),
                "division_pair_out_of_domain",
                "quotient-outside-domain",
            ),
            (
                pair(true, false),
                "division_pair_out_of_domain",
                "remainder-outside-domain",
            ),
            (
                pair(false, false),
                "division_pair_out_of_domain",
                "both-outside-domain",
            ),
            (
                Refusal::ModuloOutOfDomain { domain: interval() },
                "modulo_out_of_domain",
                "outside-domain",
            ),
            (
                Refusal::TextLengthOutOfDomain {
                    target: TextType::new(1, 8, TextProfile::Nfc).unwrap(),
                },
                "text_length_out_of_domain",
                "outside-domain",
            ),
            (
                Refusal::IntegerOutOfDomain { target: interval() },
                "integer_out_of_domain",
                "outside-domain",
            ),
            (
                Refusal::RationalOutOfDomain { target: rational() },
                "rational_out_of_domain",
                "outside-domain",
            ),
            (
                Refusal::IeeeNotExact {
                    target: IeeeWidth::Binary32,
                    would_be: IeeeFlags::EMPTY,
                },
                "ieee_not_exact",
                "rounding-required",
            ),
            (
                Refusal::IeeeNanPayloadNotRepresentable {
                    target: IeeeWidth::Binary32,
                    source: IeeeWidth::Binary64,
                },
                "ieee_nan_payload_not_representable",
                "payload-exceeds-target",
            ),
            (
                Refusal::IeeeRationalOutOfDomain { target: rational() },
                "ieee_rational_out_of_domain",
                "outside-domain",
            ),
            (
                Refusal::ForeignReference {
                    required: universe,
                    supplied: universe,
                },
                "foreign_reference",
                "foreign-universe",
            ),
            (
                Refusal::CardinalityOutOfBound {
                    violation: BoundViolation::AboveMaximum,
                    kind: CollectionKind::Sequence,
                    bound: CardinalityBound::new(0, 1).unwrap(),
                    count: 2,
                },
                "cardinality_out_of_bound",
                "above-maximum",
            ),
            (
                Refusal::CardinalityOutOfBound {
                    violation: BoundViolation::BelowMinimum,
                    kind: CollectionKind::Sequence,
                    bound: CardinalityBound::new(1, 2).unwrap(),
                    count: 0,
                },
                "cardinality_out_of_bound",
                "below-minimum",
            ),
        ];
        for (refusal, code, cause) in cases {
            assert_eq!(refusal.code(), Some(code), "{refusal:?}");
            assert_eq!(refusal.cause(), Some(cause), "{refusal:?}");
        }
        assert_eq!(Refusal::CheckedInvariant.code(), None);
        assert_eq!(Refusal::CheckedInvariant.cause(), None);
    }
}
