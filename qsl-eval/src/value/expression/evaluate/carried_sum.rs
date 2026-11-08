// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSpec FR-142 "Carried quantity sum": the left fold of
//! `quire.op.collection.sum.carried_quantity`.
//!
//! Both operands are members of the carrier `C`, so they lie on its grid and
//! the grid is closed under addition: the sum is one exact rational addition
//! of the magnitudes, never rounded under any mode, whose result is then
//! decided against `C` without a charge. A total outside `C` is the located
//! undefined outcome of FR-145, never the carrier's membership refusal.

use quire_exact::{
    rational_arithmetic_bits, sbits, sdigits, CarriedQuantityType, Charge, ChargePoint, LimitKind,
    Meter, Quantity, QuantityCarrier, Rational, RationalArithmetic, Refusal, Undefined,
};
use quire_semantic_value::stop::Stop;

/// Whether `magnitude` is a member of the carrier of `carried` (FR-142
/// "Carried quantity").
pub(super) fn carrier_admits(carried: &CarriedQuantityType, magnitude: &Rational) -> bool {
    match carried.carrier() {
        QuantityCarrier::Integer { domain, .. } => {
            magnitude.is_integer() && domain.contains(magnitude.numerator())
        }
        QuantityCarrier::Decimal(decimal) => decimal
            .placement(magnitude)
            .and_then(|placement| placement.materialize())
            .and_then(|placed| placed.admit())
            .is_ok_and(|admitted| admitted.retain().loss().is_none()),
        QuantityCarrier::Rational(domain) => domain.contains(magnitude),
    }
}

/// One addition of the fold: charges the unit addition schedule with the
/// carrier as its target, and decides the carrier's bounds on the new total.
pub(super) fn add(
    carried: &CarriedQuantityType,
    total: &Quantity,
    summand: &Quantity,
    meter: &mut Meter,
) -> Result<Quantity, Stop> {
    let (left, right) = (total.magnitude(), summand.magnitude());
    let mut bits = 0;
    for (read, operand) in (1_u64..).zip([left, right]) {
        bits = operand.max_part_bits().max(bits);
        meter.charge(
            Charge::new(ChargePoint::UnitIdentityRead)
                .size(LimitKind::ValueOccurrences, read)
                .size(LimitKind::IntegerBits, bits),
        )?;
    }
    meter.charge(Charge::new(ChargePoint::UnitRationalArithmetic).exact_size(
        LimitKind::IntegerBits,
        rational_arithmetic_bits(RationalArithmetic::Add(left, right)),
    ))?;
    let exact = left.add(right);
    let target_domain = Charge::new(ChargePoint::UnitTargetDomain);
    let out_of_domain = || Stop::Undefined(Undefined::SumOutOfDomain);
    let magnitude = match carried.carrier() {
        QuantityCarrier::Integer { domain, .. } => {
            meter.charge(
                target_domain.size(LimitKind::IntegerBits, exact.numerator().magnitude_bits()),
            )?;
            if !exact.is_integer() || !domain.contains(exact.numerator()) {
                return Err(out_of_domain());
            }
            exact
        }
        QuantityCarrier::Decimal(decimal) => {
            let placement = decimal.placement(&exact).map_err(|_| out_of_domain())?;
            let scale = u64::from(decimal.max_scale());
            let numerator = exact.numerator();
            meter.charge(
                target_domain
                    .exact_size(LimitKind::IntegerBits, sbits(numerator, scale))
                    .exact_size(LimitKind::DecimalDigits, sdigits(numerator, scale)),
            )?;
            match placement.materialize().and_then(|placed| placed.admit()) {
                Ok(admitted) => admitted.retain().value().normalized().to_rational(),
                Err(Refusal::DecimalOutOfDomain { .. }) => return Err(out_of_domain()),
                Err(refusal) => return Err(Stop::Refused(refusal)),
            }
        }
        QuantityCarrier::Rational(domain) => {
            meter.charge(
                target_domain.exact_size(LimitKind::IntegerBits, exact.max_part_bits().into()),
            )?;
            if !domain.contains(&exact) {
                return Err(out_of_domain());
            }
            exact
        }
    };
    meter.charge(
        Charge::new(ChargePoint::UnitResultRetain)
            .size(LimitKind::ValueOccurrences, 1)
            .work(quire_exact::Integer::from(1_u64))
            .results(1),
    )?;
    Ok(Quantity::new(magnitude, carried.unit().clone()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;
    use quire_exact::{
        DecimalType, Integer, IntegerInterval, RationalDomain, RoundingMode, ScalarLimits, UnitId,
    };

    const UNLIMITED: ScalarLimits = ScalarLimits {
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
    };

    fn unit() -> UnitId {
        UnitId::declared(crate::test_ids::node_key("metre"))
    }

    fn interval(lower: i64, upper: i64) -> IntegerInterval {
        IntegerInterval::new(Integer::from(lower), Integer::from(upper)).expect("ordered")
    }

    fn metres(numerator: i64, denominator: i64) -> Quantity {
        let value = Rational::new(Integer::from(numerator), Integer::from(denominator))
            .expect("nonzero denominator");
        Quantity::new(value, unit())
    }

    fn integer_carrier() -> CarriedQuantityType {
        CarriedQuantityType::new(
            unit(),
            QuantityCarrier::Integer {
                domain: interval(-5, 5),
                rounding: RoundingMode::Exact,
            },
        )
    }

    fn decimal_carrier(rounding: RoundingMode) -> CarriedQuantityType {
        let decimal = DecimalType::new(
            Integer::from(-500_i64),
            Integer::from(500_i64),
            2,
            2,
            rounding,
        )
        .expect("well-formed");
        CarriedQuantityType::new(unit(), QuantityCarrier::Decimal(decimal))
    }

    fn rational_carrier() -> CarriedQuantityType {
        let domain = RationalDomain::new(interval(-3, 7), interval(1, 12)).expect("positive");
        CarriedQuantityType::new(unit(), QuantityCarrier::Rational(domain))
    }

    fn undefined(result: Result<Quantity, Stop>) -> bool {
        matches!(result, Err(Stop::Undefined(Undefined::SumOutOfDomain)))
    }

    /// FR-142-AC-34: an addition is five work units and one result unit.
    #[trace("QSpec-TC-187", "QSpec-FR-142-AC-34")]
    #[test]
    fn an_addition_charges_five_work_units_and_one_result_unit() {
        let mut meter = Meter::new(UNLIMITED);
        let sum = add(
            &integer_carrier(),
            &metres(2, 1),
            &metres(-1, 1),
            &mut meter,
        );
        assert_eq!(sum.expect("1 m"), metres(1, 1));
        assert_eq!(meter.consumed(LimitKind::WorkUnits), 5);
        assert_eq!(meter.consumed(LimitKind::ResultUnits), 1);
    }

    /// FR-142-AC-34: the first addition of `2 m, -1 m` is `incomplete` at
    /// `unit.rational-arithmetic` under `integer_bits` three.
    #[trace("QSpec-TC-187", "QSpec-FR-142-AC-34")]
    #[test]
    fn the_addition_is_incomplete_under_a_lower_integer_bit_limit() {
        let mut meter = Meter::new(ScalarLimits {
            integer_bits: 3,
            ..UNLIMITED
        });
        let Err(Stop::Incomplete(record)) = add(
            &integer_carrier(),
            &metres(2, 1),
            &metres(-1, 1),
            &mut meter,
        ) else {
            panic!("an incomplete addition");
        };
        assert_eq!(record.charge_point, ChargePoint::UnitRationalArithmetic);
        assert_eq!(record.limit_kind, LimitKind::IntegerBits);
        assert_eq!(record.consumed, 2);
    }

    /// FR-142-AC-35: the bound is decided after the addition's target-domain
    /// charge, as the located undefined outcome, with no result unit.
    #[trace("QSpec-TC-187", "QSpec-FR-142-AC-35")]
    #[test]
    fn a_total_outside_the_carrier_is_undefined_before_the_retain() {
        let mut meter = Meter::new(UNLIMITED);
        assert!(undefined(add(
            &integer_carrier(),
            &metres(3, 1),
            &metres(3, 1),
            &mut meter
        )));
        assert_eq!(meter.consumed(LimitKind::WorkUnits), 4);
        assert_eq!(meter.consumed(LimitKind::ResultUnits), 0);
        let mut meter = Meter::new(UNLIMITED);
        let mut bounds = |left, right| {
            add(
                &integer_carrier(),
                &metres(left, 1),
                &metres(right, 1),
                &mut meter,
            )
        };
        assert_eq!(bounds(5, 0).expect("5 m"), metres(5, 1));
        assert_eq!(bounds(-3, -2).expect("-5 m"), metres(-5, 1));
    }

    /// FR-142-AC-36: zero is a member of `Int[-5, 5]` and not of `Int[1, 5]`.
    #[trace("QSpec-TC-187", "QSpec-FR-142-AC-36")]
    #[test]
    fn zero_is_a_member_only_of_a_carrier_that_holds_it() {
        let zero = Rational::from_integer(Integer::zero());
        assert!(carrier_admits(&integer_carrier(), &zero));
        let positive = CarriedQuantityType::new(
            unit(),
            QuantityCarrier::Integer {
                domain: interval(1, 5),
                rounding: RoundingMode::Exact,
            },
        );
        assert!(!carrier_admits(&positive, &zero));
    }

    /// FR-142-AC-37: a decimal carrier's sum never rounds under any mode and
    /// is undefined, never `decimal_out_of_domain`, above the carrier.
    #[trace("QSpec-TC-187", "QSpec-FR-142-AC-37")]
    #[test]
    fn a_decimal_sum_never_rounds_under_any_mode() {
        for rounding in [RoundingMode::NearestEven, RoundingMode::Exact] {
            let carrier = decimal_carrier(rounding);
            let mut meter = Meter::new(UNLIMITED);
            let sum = add(&carrier, &metres(5, 4), &metres(5, 2), &mut meter);
            assert_eq!(sum.expect("3.75 m"), metres(15, 4), "{rounding:?}");
            assert_eq!(meter.consumed(LimitKind::WorkUnits), 5);
            assert!(undefined(add(
                &carrier,
                &metres(4, 1),
                &metres(2, 1),
                &mut meter
            )));
        }
    }

    /// FR-142-AC-38: a rational carrier keeps reduced exact magnitudes and is
    /// undefined, never `rational_out_of_domain`, outside its bounds.
    #[trace("QSpec-TC-187", "QSpec-FR-142-AC-38")]
    #[test]
    fn a_rational_sum_is_reduced_and_never_rounded() {
        let carrier = rational_carrier();
        let mut meter = Meter::new(UNLIMITED);
        let mut sum = |left: Quantity, right: Quantity| add(&carrier, &left, &right, &mut meter);
        assert_eq!(
            sum(metres(1, 4), metres(1, 3)).expect("7/12 m"),
            metres(7, 12)
        );
        assert_eq!(sum(metres(1, 2), metres(1, 2)).expect("1 m"), metres(1, 1));
        assert!(undefined(sum(metres(1, 4), metres(1, 5))));
        assert!(undefined(sum(metres(6, 1), metres(2, 1))));
    }
}
