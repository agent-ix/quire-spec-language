// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-185 exact decimal semantics over the real `value` boundary.
//!
//! Vectors D01–D09 are transcribed from the vendored TC-185 procedure pinned by
//! `tests/complete_value_lock.rs`. The generated oracle uses independent `i128`
//! rational arithmetic; no floating-point value appears in either side.

use ix_trace_rs::trace;
use quire_spec_language::value::{
    evaluate_decimal, ChargePoint, Decimal, DecimalOperation, DecimalResult, DecimalTarget,
    Incomplete, InjectedDenial, Integer, IntegerInterval, LimitKind, Meter, Outcome, Refusal,
    RoundingMode, ScalarLimits, Undefined,
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

fn dec(coefficient: i64, scale: u32) -> Decimal {
    Decimal::new(Integer::from(coefficient), scale)
}

fn int(value: &Integer) -> i128 {
    value.to_string().parse().unwrap()
}

fn pair(decimal: &Decimal) -> ((i128, u32), (i128, u32)) {
    let (repr, norm) = (decimal.representation(), decimal.normalized());
    (
        (int(repr.coefficient()), repr.scale()),
        (int(norm.coefficient()), norm.scale()),
    )
}

fn run(operation: DecimalOperation<'_>, target: &DecimalTarget) -> Outcome<DecimalResult> {
    evaluate_decimal(operation, target, &mut Meter::new(UNLIMITED))
}

fn completed(outcome: Outcome<DecimalResult>) -> DecimalResult {
    match outcome {
        Outcome::Completed(result) => result,
        other => panic!("expected a completed decimal, got {other:?}"),
    }
}

fn domain(lower: i64, upper: i64) -> IntegerInterval {
    IntegerInterval::new(Integer::from(lower), Integer::from(upper)).unwrap()
}

#[trace("TC-185", "FR-140-AC-1")]
#[test]
fn d01_equal_values_retain_distinct_representations() {
    let (a, b) = (dec(10, 1), dec(100, 2));
    assert!(a.numerically_equal(&b));
    assert_eq!(a.compare(&b), std::cmp::Ordering::Equal);
    assert_eq!(pair(&a), ((10, 1), (1, 0)));
    assert_eq!(pair(&b), ((100, 2), (1, 0)));
    assert!(!a.numerically_equal(&dec(11, 1)));
    for source in [&a, &b] {
        let result = completed(run(
            DecimalOperation::Round(source),
            &DecimalTarget::new(2, RoundingMode::Exact),
        ));
        assert_eq!(pair(result.value()).1, (1, 0));
        assert_eq!(result.value().representation(), source.representation());
        assert!(result.loss().is_none());
    }
}

#[trace("TC-185", "FR-140-AC-2")]
#[test]
fn d02_d04_exact_arithmetic_has_no_loss_in_every_mode() {
    for mode in RoundingMode::ALL {
        let sum = completed(run(
            DecimalOperation::Add(&dec(125, 2), &dec(75, 2)),
            &DecimalTarget::new(2, mode),
        ));
        assert_eq!(pair(sum.value()), ((200, 2), (2, 0)));
        assert!(sum.loss().is_none());

        let quotient = completed(run(
            DecimalOperation::Divide(&dec(1, 0), &dec(8, 0)),
            &DecimalTarget::new(3, mode),
        ));
        assert_eq!(pair(quotient.value()), ((125, 3), (125, 3)));
        assert!(quotient.loss().is_none());
    }
}

#[trace("TC-185", "FR-140-AC-3", "FR-140-AC-4")]
#[test]
fn d03_every_mode_rounds_both_signed_halves_with_a_typed_loss() {
    let table = [
        (RoundingMode::TowardZero, 2, -2),
        (RoundingMode::TowardPositive, 3, -2),
        (RoundingMode::TowardNegative, 2, -3),
        (RoundingMode::NearestEven, 2, -2),
        (RoundingMode::NearestAway, 3, -3),
    ];
    let target = |mode| DecimalTarget::new(0, mode);
    for coefficient in [25, -25] {
        assert_eq!(
            run(
                DecimalOperation::Round(&dec(coefficient, 1)),
                &target(RoundingMode::Exact)
            ),
            Outcome::Refused(Refusal::InexactDecimal)
        );
    }
    for (mode, positive, negative) in table {
        for (coefficient, expected, exact_numerator) in [(25, positive, 5), (-25, negative, -5)] {
            let result = completed(run(
                DecimalOperation::Round(&dec(coefficient, 1)),
                &target(mode),
            ));
            assert_eq!(
                pair(result.value()).0,
                (expected, 0),
                "{mode:?} {coefficient}"
            );
            let loss = result.loss().expect("rounded result carries loss");
            assert_eq!(int(loss.exact_numerator()), exact_numerator);
            assert_eq!(int(loss.exact_denominator()), 2);
            assert_eq!(int(loss.rounded_coefficient()), expected);
            assert_eq!(loss.rounded_scale(), 0);
            assert_eq!(loss.mode(), mode);
        }
    }
}

#[trace("TC-185", "FR-140-AC-3")]
#[test]
fn d05_recurring_quotient_refuses_exact_and_records_nearest_even_loss() {
    let (one, three) = (dec(1, 0), dec(3, 0));
    assert_eq!(
        run(
            DecimalOperation::Divide(&one, &three),
            &DecimalTarget::new(2, RoundingMode::Exact)
        ),
        Outcome::Refused(Refusal::InexactDecimal)
    );
    let result = completed(run(
        DecimalOperation::Divide(&one, &three),
        &DecimalTarget::new(2, RoundingMode::NearestEven),
    ));
    assert_eq!(pair(result.value()).0, (33, 2));
    let loss = result.loss().unwrap();
    assert_eq!(
        (
            int(loss.exact_numerator()),
            int(loss.exact_denominator()),
            int(loss.rounded_coefficient()),
            loss.rounded_scale(),
            loss.mode()
        ),
        (1, 3, 33, 2, RoundingMode::NearestEven)
    );
}

#[trace("TC-185", "FR-140-AC-5")]
#[test]
fn d06_d08_zero_divisors_are_undefined_and_domains_refuse() {
    for zero in [dec(0, 0), dec(0, 3)] {
        for mode in RoundingMode::ALL {
            assert_eq!(
                run(
                    DecimalOperation::Divide(&dec(1, 0), &zero),
                    &DecimalTarget::new(2, mode)
                ),
                Outcome::Undefined(Undefined::DivisionByZero)
            );
        }
    }
    assert_eq!(
        run(
            DecimalOperation::Round(&dec(25, 1)),
            &DecimalTarget::new(0, RoundingMode::NearestAway)
                .with_coefficient_domain(domain(-2, 2))
        ),
        Outcome::Refused(Refusal::DecimalOutOfDomain)
    );
    let admit = |coefficient| {
        run(
            DecimalOperation::Round(&dec(coefficient, 0)),
            &DecimalTarget::new(0, RoundingMode::Exact).with_coefficient_domain(domain(-2, 2)),
        )
    };
    for endpoint in [-2, 2] {
        assert_eq!(
            pair(completed(admit(endpoint)).value()).1,
            (i128::from(endpoint), 0)
        );
    }
    for outside in [-3, 3] {
        assert_eq!(
            admit(outside),
            Outcome::Refused(Refusal::DecimalOutOfDomain)
        );
    }
}

const D09: ScalarLimits = ScalarLimits {
    integer_bits: 7,
    decimal_digits: 3,
    scale_expansion: 2,
    text_input_bytes: 0,
    text_scalars: 0,
    normalized_scalars: 0,
    unit_edges: 0,
    value_occurrences: 2,
    work_units: 5,
    result_units: 1,
};

fn d05(meter: &mut Meter) -> Outcome<DecimalResult> {
    evaluate_decimal(
        DecimalOperation::Divide(&dec(1, 0), &dec(3, 0)),
        &DecimalTarget::new(2, RoundingMode::NearestEven),
        meter,
    )
}

#[trace("TC-185", "FR-140-AC-6")]
#[test]
fn d09_exact_bound_succeeds_and_each_named_denial_is_incomplete() {
    let mut meter = Meter::new(D09);
    let result = completed(d05(&mut meter));
    assert_eq!(pair(result.value()).0, (33, 2));
    assert_eq!(
        meter.admitted_charges(),
        [
            ChargePoint::DecimalOperands,
            ChargePoint::DecimalScaleExpansion,
            ChargePoint::DecimalArithmetic,
            ChargePoint::DecimalRounding,
            ChargePoint::DecimalResultRetain,
        ]
    );
    let consumed: Vec<_> = LimitKind::ALL.map(|kind| meter.consumed(kind)).to_vec();
    assert_eq!(consumed, [7, 3, 2, 0, 0, 0, 0, 2, 5, 1]);

    let one_less = ScalarLimits {
        work_units: 4,
        ..D09
    };
    assert_eq!(
        d05(&mut Meter::new(one_less)),
        Outcome::Incomplete(Incomplete {
            limit_kind: LimitKind::WorkUnits,
            limit: 4,
            consumed: 4,
            next_charge: 1,
            charge_point: ChargePoint::DecimalResultRetain,
        })
    );
    let no_result = ScalarLimits {
        result_units: 0,
        ..D09
    };
    assert!(matches!(
        d05(&mut Meter::new(no_result)),
        Outcome::Incomplete(Incomplete {
            limit_kind: LimitKind::ResultUnits,
            charge_point: ChargePoint::DecimalResultRetain,
            ..
        })
    ));
    let narrow_bits = ScalarLimits {
        integer_bits: 6,
        ..D09
    };
    assert!(matches!(
        d05(&mut Meter::new(narrow_bits)),
        Outcome::Incomplete(Incomplete {
            limit_kind: LimitKind::IntegerBits,
            next_charge: 7,
            charge_point: ChargePoint::DecimalScaleExpansion,
            ..
        })
    ));
    let narrow_shift = ScalarLimits {
        scale_expansion: 1,
        ..D09
    };
    assert!(matches!(
        d05(&mut Meter::new(narrow_shift)),
        Outcome::Incomplete(Incomplete {
            limit_kind: LimitKind::ScaleExpansion,
            next_charge: 2,
            charge_point: ChargePoint::DecimalScaleExpansion,
            ..
        })
    ));

    for point in meter.admitted_charges() {
        let mut denied = Meter::new(D09).with_injected_denial(InjectedDenial {
            point: *point,
            occurrence: 1,
        });
        match d05(&mut denied) {
            Outcome::Incomplete(record) => {
                assert_eq!(record.charge_point, *point);
                assert_eq!(
                    (record.limit_kind, record.next_charge),
                    (LimitKind::WorkUnits, 1)
                );
            }
            other => panic!("{point:?}: {other:?}"),
        }
        assert_eq!(denied.consumed(LimitKind::ResultUnits), 0);
    }

    // An independently metered sibling is unaffected by the exhausted request.
    assert_eq!(
        pair(completed(d05(&mut Meter::new(D09))).value()).0,
        (33, 2)
    );
}

// ---- generated vectors -----------------------------------------------------

fn gcd(a: i128, b: i128) -> i128 {
    if b == 0 {
        a.abs()
    } else {
        gcd(b, a % b)
    }
}

fn reduce(numerator: i128, denominator: i128) -> (i128, i128) {
    let sign = if denominator < 0 { -1 } else { 1 };
    let divisor = gcd(numerator, denominator).max(1);
    (sign * numerator / divisor, sign * denominator / divisor)
}

fn floor_div(a: i128, b: i128) -> i128 {
    let quotient = a / b;
    if (a % b != 0) && ((a < 0) != (b < 0)) {
        quotient - 1
    } else {
        quotient
    }
}

/// Independent FR-140 oracle: exact rational, one rounding, domain admission.
enum Expected {
    Undefined,
    Inexact,
    OutOfDomain,
    Value {
        value: (i128, i128),
        loss: Option<(i128, i128, i128)>,
    },
}

fn oracle(value: Option<(i128, i128)>, scale: u32, mode: RoundingMode, bound: i128) -> Expected {
    let Some((numerator, denominator)) = value else {
        return Expected::Undefined;
    };
    let unit = 10_i128.pow(scale);
    let (n, d) = reduce(numerator * unit, denominator);
    let rounded = if d == 1 {
        n
    } else {
        let floor = floor_div(n, d);
        let twice = 2 * (n - floor * d);
        match mode {
            RoundingMode::Exact => return Expected::Inexact,
            RoundingMode::TowardNegative => floor,
            RoundingMode::TowardPositive => floor + 1,
            RoundingMode::TowardZero => {
                if n < 0 {
                    floor + 1
                } else {
                    floor
                }
            }
            RoundingMode::NearestEven | RoundingMode::NearestAway => match twice.cmp(&d) {
                std::cmp::Ordering::Less => floor,
                std::cmp::Ordering::Greater => floor + 1,
                std::cmp::Ordering::Equal if mode == RoundingMode::NearestEven => {
                    if floor % 2 == 0 {
                        floor
                    } else {
                        floor + 1
                    }
                }
                std::cmp::Ordering::Equal => {
                    if n < 0 {
                        floor
                    } else {
                        floor + 1
                    }
                }
            },
        }
    };
    let (mut coefficient, mut normalized_scale) = (rounded, scale);
    while normalized_scale > 0 && coefficient % 10 == 0 {
        coefficient /= 10;
        normalized_scale -= 1;
    }
    if coefficient.abs() > bound {
        return Expected::OutOfDomain;
    }
    Expected::Value {
        value: reduce(rounded, unit),
        loss: (d != 1).then(|| {
            let exact = reduce(numerator, denominator);
            (exact.0, exact.1, rounded)
        }),
    }
}

fn rational(decimal: &Decimal) -> (i128, i128) {
    let repr = decimal.representation();
    reduce(int(repr.coefficient()), 10_i128.pow(repr.scale()))
}

#[trace(
    "TC-185",
    "FR-140-AC-2",
    "FR-140-AC-3",
    "FR-140-AC-4",
    "FR-140-AC-5",
    "FR-140-AC-6"
)]
#[test]
fn generated_operations_match_the_exact_rational_oracle_and_every_denial() {
    const COEFFICIENTS: [i64; 10] = [-25, -10, -7, -5, -1, 0, 1, 3, 8, 10];
    const BOUND: i64 = 40;
    let mut checked = [0_usize; 4];
    for (ca, sa) in COEFFICIENTS
        .iter()
        .flat_map(|c| (0..3).map(move |s| (*c, s)))
    {
        for (cb, sb) in COEFFICIENTS
            .iter()
            .flat_map(|c| (0..3).map(move |s| (*c, s)))
        {
            let (a, b) = (dec(ca, sa), dec(cb, sb));
            let (ra, rb) = (rational(&a), rational(&b));
            let operations = [
                (
                    DecimalOperation::Add(&a, &b),
                    Some(reduce(ra.0 * rb.1 + rb.0 * ra.1, ra.1 * rb.1)),
                ),
                (
                    DecimalOperation::Subtract(&a, &b),
                    Some(reduce(ra.0 * rb.1 - rb.0 * ra.1, ra.1 * rb.1)),
                ),
                (
                    DecimalOperation::Multiply(&a, &b),
                    Some(reduce(ra.0 * rb.0, ra.1 * rb.1)),
                ),
                (
                    DecimalOperation::Divide(&a, &b),
                    (rb.0 != 0).then(|| reduce(ra.0 * rb.1, ra.1 * rb.0)),
                ),
                (DecimalOperation::Negate(&a), Some((-ra.0, ra.1))),
                (DecimalOperation::Round(&a), Some(ra)),
            ];
            for (operation, exact) in operations {
                for scale in 0..3 {
                    for mode in RoundingMode::ALL {
                        let target = DecimalTarget::new(scale, mode)
                            .with_coefficient_domain(domain(-BOUND, BOUND));
                        let mut meter = Meter::new(UNLIMITED);
                        let outcome = evaluate_decimal(operation, &target, &mut meter);
                        let case = format!("{operation:?} scale {scale} {mode:?}");
                        match (oracle(exact, scale, mode, i128::from(BOUND)), &outcome) {
                            (
                                Expected::Undefined,
                                Outcome::Undefined(Undefined::DivisionByZero),
                            ) => checked[0] += 1,
                            (Expected::Inexact, Outcome::Refused(Refusal::InexactDecimal)) => {
                                checked[1] += 1
                            }
                            (
                                Expected::OutOfDomain,
                                Outcome::Refused(Refusal::DecimalOutOfDomain),
                            ) => checked[2] += 1,
                            (Expected::Value { value, loss }, Outcome::Completed(result)) => {
                                checked[3] += 1;
                                assert_eq!(rational(result.value()), value, "{case}");
                                assert!(result.value().representation().scale() <= scale, "{case}");
                                let actual = result.loss().map(|loss| {
                                    assert_eq!(loss.mode(), mode, "{case}");
                                    assert_eq!(loss.rounded_scale(), scale, "{case}");
                                    (
                                        int(loss.exact_numerator()),
                                        int(loss.exact_denominator()),
                                        int(loss.rounded_coefficient()),
                                    )
                                });
                                assert_eq!(actual, loss, "{case}");
                            }
                            (_, other) => panic!("{case}: oracle disagrees with {other:?}"),
                        }
                        for point in meter.admitted_charges() {
                            let mut denied =
                                Meter::new(UNLIMITED).with_injected_denial(InjectedDenial {
                                    point: *point,
                                    occurrence: 1,
                                });
                            match evaluate_decimal(operation, &target, &mut denied) {
                                Outcome::Incomplete(record) if record.charge_point == *point => {}
                                other => panic!("{case} denied at {point:?}: {other:?}"),
                            }
                            assert_eq!(denied.consumed(LimitKind::ResultUnits), 0, "{case}");
                        }
                    }
                }
            }
        }
    }
    // Every oracle class, including ties and recurring quotients, was exercised.
    assert!(checked.iter().all(|count| *count > 100), "{checked:?}");
}
