// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-147 integer division: the three `div`/`rem` laws, independent Euclidean
//! `mod`, atomic pair admission and the named integer-division and
//! integer-modulus charges.

use super::definition::AdmittedIntegerDivision;
use super::outcome::{Outcome, Refusal, Stop, Undefined};
use quire_exact::{Charge, ChargePoint, DivisionProfile, Integer, IntegerDomain, LimitKind, Meter};

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
