// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-147 integer division: the three `div`/`rem` laws, independent Euclidean
//! `mod`, atomic pair admission and the named integer-division charges.

use super::accounting::{Charge, ChargePoint, LimitKind, Meter};
use super::definition::AdmittedIntegerDivision;
use super::integer::{Integer, IntegerDomain};
use super::outcome::{Outcome, Refusal, Stop, Undefined};

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
        Exposure::Pair,
        meter,
    ))
}

/// Evaluate `mod`: always the Euclidean remainder, independent of any selected
/// `div`/`rem` law.
pub fn modulo(
    dividend: &Integer,
    divisor: &Integer,
    domain: &IntegerDomain,
    meter: &mut Meter,
) -> Outcome<Integer> {
    // SPEC-GAP(2): FR-147 and `value-accounting.md` define no `mod` charge
    // schedule. `mod` reuses the four `integer-division.*` charges, including
    // the two-unit `result-pair`, through this single call.
    Outcome::from_stop(
        paired(
            DivisionProfile::Euclidean,
            dividend,
            divisor,
            domain,
            Exposure::Remainder,
            meter,
        )
        .map(|pair| pair.remainder),
    )
}

/// Which members an operation exposes and therefore must admit.
#[derive(Clone, Copy)]
enum Exposure {
    Pair,
    Remainder,
}

/// Charge, compute and admit the pair. Membership of every exposed member is
/// decided before the atomic `integer-division.result-pair` retention.
fn paired(
    profile: DivisionProfile,
    dividend: &Integer,
    divisor: &Integer,
    domain: &IntegerDomain,
    exposure: Exposure,
    meter: &mut Meter,
) -> Result<QuotientRemainder, Stop> {
    meter.charge(
        Charge::new(ChargePoint::IntegerDivisionOperands)
            .size(
                LimitKind::IntegerBits,
                dividend.magnitude_bits().max(divisor.magnitude_bits()),
            )
            .size(LimitKind::ValueOccurrences, 2),
    )?;
    if divisor.is_zero() {
        return Err(Stop::Undefined(Undefined::DivisionByZero));
    }
    let (quotient, remainder) = profile.apply(dividend, divisor);
    meter.charge(
        Charge::new(ChargePoint::IntegerDivisionArithmetic).size(
            LimitKind::IntegerBits,
            [dividend, divisor, &quotient, &remainder]
                .into_iter()
                .map(Integer::magnitude_bits)
                .max()
                .unwrap_or(1),
        ),
    )?;
    meter.charge(
        Charge::new(ChargePoint::IntegerDivisionDomainPair).size(LimitKind::ValueOccurrences, 2),
    )?;
    let quotient_admitted = domain.contains(&quotient);
    let remainder_admitted = domain.contains(&remainder);
    match exposure {
        Exposure::Pair if !(quotient_admitted && remainder_admitted) => {
            return Err(Stop::Refused(Refusal::DivisionPairOutOfDomain {
                quotient_admitted,
                remainder_admitted,
            }));
        }
        Exposure::Remainder if !remainder_admitted => {
            return Err(Stop::Refused(Refusal::ModuloOutOfDomain));
        }
        Exposure::Pair | Exposure::Remainder => {}
    }
    meter.charge(Charge::new(ChargePoint::IntegerDivisionResultPair).results(2))?;
    Ok(QuotientRemainder {
        quotient,
        remainder,
    })
}
