// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-147 integer division: the three `div`/`rem` laws, independent Euclidean
//! `mod`, atomic pair admission and the named integer-division and
//! integer-modulus charges, and the I13 negotiation disposition of a finite
//! consumer.

use super::accounting::{Charge, ChargePoint, LimitKind, Meter};
use super::definition::AdmittedIntegerDivision;
use super::outcome::{Outcome, Refusal, Stop, Undefined};
use quire_exact::{Integer, IntegerDomain, IntegerInterval};

/// A selectable `div`/`rem` law.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum DivisionProfile {
    /// `quire.value.integer-division.truncating/v1`.
    Truncating,
    /// `quire.value.integer-division.floor/v1`.
    Floor,
    /// `quire.value.integer-division.euclidean/v1`.
    Euclidean,
}

impl DivisionProfile {
    /// Every law.
    pub const ALL: [Self; 3] = [Self::Truncating, Self::Floor, Self::Euclidean];

    /// The exact definition identity that selects this law.
    pub fn definition_identity(self) -> &'static str {
        match self {
            Self::Truncating => "quire.value.integer-division.truncating/v1",
            Self::Floor => "quire.value.integer-division.floor/v1",
            Self::Euclidean => "quire.value.integer-division.euclidean/v1",
        }
    }

    /// The unique `(q, r)` with `a = b*q + r` under this law; `b` is nonzero.
    fn apply(self, dividend: &Integer, divisor: &Integer) -> (Integer, Integer) {
        match self {
            Self::Truncating => dividend.div_rem_truncating(divisor),
            Self::Floor => dividend.div_mod_floor(divisor),
            Self::Euclidean => {
                let (quotient, remainder) = dividend.div_mod_floor(divisor);
                if remainder.is_negative() {
                    // Only a negative divisor yields a negative floor remainder.
                    (quotient.add(&Integer::one()), remainder.sub(divisor))
                } else {
                    (quotient, remainder)
                }
            }
        }
    }
}

/// An atomically admitted quotient/remainder pair.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct QuotientRemainder {
    quotient: Integer,
    remainder: Integer,
}

impl QuotientRemainder {
    /// Quotient.
    pub fn quotient(&self) -> &Integer {
        &self.quotient
    }

    /// Remainder.
    pub fn remainder(&self) -> &Integer {
        &self.remainder
    }
}

/// Evaluate paired `div`/`rem` under the admitted package law.
pub fn divide(
    selection: &AdmittedIntegerDivision,
    dividend: &Integer,
    divisor: &Integer,
    domain: &IntegerDomain,
    meter: &mut Meter,
) -> Outcome<QuotientRemainder> {
    Outcome::from_stop(paired(
        selection.profile(),
        dividend,
        divisor,
        domain,
        meter,
    ))
}

/// Evaluate `mod`: always the Euclidean remainder, independent of any selected
/// `div`/`rem` law, charged only at the four `integer-modulus.*` points.
pub fn modulo(
    dividend: &Integer,
    divisor: &Integer,
    domain: &IntegerDomain,
    meter: &mut Meter,
) -> Outcome<Integer> {
    Outcome::from_stop(euclidean_remainder(dividend, divisor, domain, meter))
}

fn operand_bits(dividend: &Integer, divisor: &Integer) -> u64 {
    dividend.magnitude_bits().max(divisor.magnitude_bits())
}

/// The `integer-division.arithmetic` and `integer-modulus.arithmetic` amount:
/// `max(bits(a),bits(b))`, which bounds both quotient and remainder.
fn arithmetic_bits(dividend: &Integer, divisor: &Integer) -> u64 {
    operand_bits(dividend, divisor)
}

fn reject_zero_divisor(divisor: &Integer) -> Result<(), Stop> {
    if divisor.is_zero() {
        Err(Stop::Undefined(Undefined::DivisionByZero))
    } else {
        Ok(())
    }
}

/// Charge, compute and admit the pair. Membership of both members is decided
/// after `integer-division.domain-pair` and before the atomic retention.
fn paired(
    profile: DivisionProfile,
    dividend: &Integer,
    divisor: &Integer,
    domain: &IntegerDomain,
    meter: &mut Meter,
) -> Result<QuotientRemainder, Stop> {
    meter.charge(
        Charge::new(ChargePoint::IntegerDivisionOperands)
            .size(LimitKind::IntegerBits, operand_bits(dividend, divisor))
            .size(LimitKind::ValueOccurrences, 2),
    )?;
    reject_zero_divisor(divisor)?;
    meter.charge(
        Charge::new(ChargePoint::IntegerDivisionArithmetic)
            .size(LimitKind::IntegerBits, arithmetic_bits(dividend, divisor)),
    )?;
    let (quotient, remainder) = profile.apply(dividend, divisor);
    meter.charge(
        Charge::new(ChargePoint::IntegerDivisionDomainPair).size(LimitKind::ValueOccurrences, 2),
    )?;
    let quotient_admitted = domain.contains(&quotient);
    let remainder_admitted = domain.contains(&remainder);
    if !(quotient_admitted && remainder_admitted) {
        return Err(Stop::Refused(Refusal::DivisionPairOutOfDomain {
            quotient_admitted,
            remainder_admitted,
        }));
    }
    meter.charge(Charge::new(ChargePoint::IntegerDivisionResultPair).results(2))?;
    Ok(QuotientRemainder {
        quotient,
        remainder,
    })
}

fn euclidean_remainder(
    dividend: &Integer,
    divisor: &Integer,
    domain: &IntegerDomain,
    meter: &mut Meter,
) -> Result<Integer, Stop> {
    meter.charge(
        Charge::new(ChargePoint::IntegerModulusOperands)
            .size(LimitKind::IntegerBits, operand_bits(dividend, divisor))
            .size(LimitKind::ValueOccurrences, 2),
    )?;
    reject_zero_divisor(divisor)?;
    meter.charge(
        Charge::new(ChargePoint::IntegerModulusArithmetic)
            .size(LimitKind::IntegerBits, arithmetic_bits(dividend, divisor)),
    )?;
    let (_, remainder) = DivisionProfile::Euclidean.apply(dividend, divisor);
    meter.charge(
        Charge::new(ChargePoint::IntegerModulusDomain).size(LimitKind::ValueOccurrences, 1),
    )?;
    if !domain.contains(&remainder) {
        return Err(Stop::Refused(Refusal::ModuloOutOfDomain));
    }
    meter.charge(Charge::new(ChargePoint::IntegerModulusResultRetain).results(1))?;
    Ok(remainder)
}

/// The declared bounds a finite I13 consumer offers for one `div`/`rem` or
/// `mod` item. An absent member is a missing bound.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct IntegerDivisionBounds {
    /// Bound on both operands.
    pub operand: Option<IntegerInterval>,
    /// Bound on the exact intermediate quotient and remainder.
    pub intermediate: Option<IntegerInterval>,
    /// Bound on the exposed results.
    pub result: Option<IntegerInterval>,
}

impl IntegerDivisionBounds {
    fn complete(&self) -> bool {
        self.operand.is_some() && self.intermediate.is_some() && self.result.is_some()
    }
}

/// The consumer of one integer-division item at I13 negotiation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum IntegerDivisionConsumer {
    /// Unbounded mathematical integers; no bound is needed.
    Mathematical,
    /// A finite backend with its declared bounds.
    Finite(IntegerDivisionBounds),
}

/// The per-item I13 negotiation disposition. It is not an evaluator outcome:
/// negotiation never evaluates and never narrows mathematical integers.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum IntegerDivisionDisposition {
    /// The consumer can execute the item.
    Supported,
    /// `requires-bound`: a finite consumer lacks an operand, intermediate or
    /// result bound.
    RequiresBound,
}

/// Negotiate each item independently, before and without evaluation.
pub fn negotiate_integer_division(
    items: &[IntegerDivisionConsumer],
) -> Vec<IntegerDivisionDisposition> {
    items
        .iter()
        .map(|consumer| match consumer {
            IntegerDivisionConsumer::Mathematical => IntegerDivisionDisposition::Supported,
            IntegerDivisionConsumer::Finite(bounds) if bounds.complete() => {
                IntegerDivisionDisposition::Supported
            }
            IntegerDivisionConsumer::Finite(_) => IntegerDivisionDisposition::RequiresBound,
        })
        .collect()
}
