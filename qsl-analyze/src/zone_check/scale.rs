// SPDX-License-Identifier: AGPL-3.0-or-later
//! Scaling a check's constants to integers (ADR-026 EZ-2, FR-238).

use num_bigint::BigInt;
use num_integer::Integer as _;
use num_traits::One;
use quire_exact::Rational;

/// Every constant of a check multiplied by one scale factor, the least
/// common multiple of their denominators, so each is an integer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScaledConstants {
    factor: BigInt,
    constants: Vec<BigInt>,
}

impl ScaledConstants {
    /// The scale factor: the least common multiple of every denominator,
    /// 1 when there are no constants.
    pub fn factor(&self) -> &BigInt {
        &self.factor
    }

    /// The scaled constants, in the order they were given.
    pub fn constants(&self) -> &[BigInt] {
        &self.constants
    }
}

/// Scale every constant of the model and the claim by the least common
/// multiple of their denominators.
pub fn scale_constants(constants: &[Rational]) -> ScaledConstants {
    let factor = constants.iter().fold(BigInt::one(), |factor, constant| {
        factor.lcm(constant.denominator().as_big())
    });
    let constants = constants
        .iter()
        .map(|constant| {
            let multiplier = &factor / constant.denominator().as_big();
            constant.numerator().as_big() * multiplier
        })
        .collect();
    ScaledConstants { factor, constants }
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;
    use num_bigint::BigInt;
    use quire_exact::{Integer, Rational};

    use super::scale_constants;
    use crate::zone_check::{Bound, Dbm};

    fn rational(numerator: BigInt, denominator: i64) -> Rational {
        Rational::new(Integer::from_big(numerator), Integer::from(denominator))
            .expect("a non-zero denominator")
    }

    /// FR-238-AC-1: `1/3`, `5/2` and `1` scale by 6 to 2, 15 and 6.
    #[trace("TC-693", "FR-238-AC-1")]
    #[test]
    fn tc_693_constants_scale_by_the_lcm_of_their_denominators() {
        let scaled = scale_constants(&[
            rational(BigInt::from(1), 3),
            rational(BigInt::from(5), 2),
            rational(BigInt::from(1), 1),
        ]);
        assert_eq!(scaled.factor(), &BigInt::from(6));
        assert_eq!(
            scaled.constants(),
            [BigInt::from(2), BigInt::from(15), BigInt::from(6)]
        );
    }

    /// FR-238-AC-1: `10^30` and `1/7` scale by 7, and every DBM bound
    /// built from them is exact, far past any machine integer.
    #[trace("TC-693", "FR-238-AC-1")]
    #[test]
    fn tc_693_a_huge_constant_scales_and_computes_without_overflow() {
        let huge = BigInt::from(10).pow(30);
        let scaled = scale_constants(&[rational(huge.clone(), 1), rational(BigInt::from(1), 7)]);
        assert_eq!(scaled.factor(), &BigInt::from(7));
        let c: BigInt = &huge * 7_u32;
        assert_eq!(scaled.constants(), [c.clone(), BigInt::from(1)]);

        // x, y: let time pass up to c, reset x, let time pass to x <= c
        // again: y reaches 2c, and x - y spans [-c, 0].
        let mut zone = Dbm::zero(3);
        zone.up();
        zone.constrain(1, 0, Bound::le(c.clone()))
            .expect("x is a clock");
        zone.reset(1, &BigInt::from(0)).expect("x is a clock");
        zone.up();
        zone.constrain(1, 0, Bound::le(c.clone()))
            .expect("x is a clock");
        zone.constrain(0, 2, Bound::lt(-&scaled.constants()[1]))
            .expect("y is a clock");
        assert!(!zone.is_empty());
        assert_eq!(zone.bound(2, 0), Ok(&Bound::le(&c * 2_u32)));
        assert_eq!(zone.bound(1, 2), Ok(&Bound::le(BigInt::from(0))));
        assert_eq!(zone.bound(2, 1), Ok(&Bound::le(c.clone())));
        assert_eq!(zone.bound(0, 2), Ok(&Bound::lt(BigInt::from(-1))));
    }
}
