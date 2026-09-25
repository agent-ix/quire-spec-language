// SPDX-License-Identifier: AGPL-3.0-or-later
//! Integer division: the three `div`/`rem` laws, independent Euclidean
//! `mod`, and atomic pair admission with the named integer-division and
//! integer-modulus charges.
//!
//! Ported from QSL `value::division`, dropping the I13 negotiation surface
//! entirely: `IntegerDivisionBounds`/`IntegerDivisionConsumer`/
//! `IntegerDivisionDisposition`/`negotiate_integer_division` decide, ahead
//! of and independent of evaluation, whether a *backend* can execute a
//! division item at all -- a QSL `definition`-package capability-negotiation
//! concept, not a kernel operation. [`divide`]'s signature is adapted to
//! match: it takes the selected [`DivisionProfile`] directly, in place of
//! the original `&AdmittedIntegerDivision` (a QSL `definition` type wrapping
//! that same profile plus the now-dropped negotiation state).

use crate::accounting::{Charge, ChargePoint, LimitKind, Meter};
use crate::integer::{Integer, IntegerDomain};
use crate::outcome::{Outcome, Refusal, Stop, Undefined};

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

/// Evaluate paired `div`/`rem` under the selected law.
pub fn divide(
    profile: DivisionProfile,
    dividend: &Integer,
    divisor: &Integer,
    domain: &IntegerDomain,
    meter: &mut Meter,
) -> Outcome<QuotientRemainder> {
    Outcome::from_stop(paired(profile, dividend, divisor, domain, meter))
}

/// Evaluate `mod`: always the Euclidean remainder, independent of any
/// selected `div`/`rem` law, charged only at the four `integer-modulus.*`
/// points.
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

/// The `integer-division.arithmetic` and `integer-modulus.arithmetic`
/// amount: `max(bits(a),bits(b))`, which bounds both quotient and
/// remainder.
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

/// Charge, compute and admit the pair. Membership of both members is
/// decided after `integer-division.domain-pair` and before the atomic
/// retention.
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

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;

    use super::*;
    use crate::accounting::ScalarLimits;

    fn generous_meter() -> Meter {
        Meter::new(ScalarLimits {
            integer_bits: u64::MAX,
            decimal_digits: u64::MAX,
            scale_expansion: u64::MAX,
            text_input_bytes: u64::MAX,
            text_scalars: u64::MAX,
            normalized_scalars: u64::MAX,
            unit_edges: u64::MAX,
            value_occurrences: u64::MAX,
            work_units: u64::MAX,
            result_units: u64::MAX,
        })
    }

    /// TC-323: dividing by zero under any law is undefined, never a panic.
    ///
    /// Also QSpec FR-147-AC-2 ("Division by zero is undefined and produces
    /// no numeric value"), verified through central `TC-192`
    /// (`agent-ix/quire-specification`): this is the kernel-level instance
    /// of that requirement (QSL-163).
    #[trace("TC-323")]
    #[trace("TC-192", "FR-147-AC-2")]
    #[test]
    fn tc_323_division_by_zero_is_undefined() {
        let domain = IntegerDomain::Mathematical;
        let mut meter = generous_meter();
        let outcome = divide(
            DivisionProfile::Truncating,
            &Integer::one(),
            &Integer::zero(),
            &domain,
            &mut meter,
        );
        assert!(matches!(
            outcome,
            Outcome::Undefined(Undefined::DivisionByZero)
        ));
    }

    /// TC-324: Euclidean `mod` never returns a negative remainder for a
    /// negative dividend, unlike truncating `rem`.
    ///
    /// Also QSpec FR-147-AC-4 (truncating, floor and Euclidean results stay
    /// distinguished while preserving `a = b*q + r`), verified through
    /// central `TC-192`: a wrong implementation that gave `mod` the
    /// truncating law's sign instead of Euclidean's would return a
    /// negative remainder here, which this test refuses (QSL-163).
    #[trace("TC-324")]
    #[trace("TC-192", "FR-147-AC-4")]
    #[test]
    fn tc_324_euclidean_modulo_is_nonnegative() {
        let domain = IntegerDomain::Mathematical;
        let mut meter = generous_meter();
        let dividend = Integer::zero().sub(&Integer::from(7_u64));
        let divisor = Integer::from(3_u64);
        let outcome = modulo(&dividend, &divisor, &domain, &mut meter);
        let remainder = outcome
            .completed()
            .expect("euclidean mod is total for a nonzero divisor");
        assert!(!remainder.is_negative());
    }
}
