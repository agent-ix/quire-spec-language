// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-185 exact decimal semantics over the real `value` boundary.
//!
//! Vectors D01–D23 are transcribed from the vendored TC-185 procedure pinned by
//! `tests/complete_value_lock.rs`. The generated oracle uses independent `i128`
//! rational arithmetic; no floating-point value appears in either side.

use ix_trace_rs::trace;
use num_bigint::BigInt;
use num_traits::Pow;
use quire_spec_language::model::population::AdmissionChargePoint;
use quire_spec_language::value::{
    evaluate_decimal, order_numbers, ChargePoint, Decimal, DecimalOperation, DecimalResult,
    DecimalType, IllTyped, IllTypedCause, Incomplete, InjectedDenial, Integer, LimitKind, Meter,
    OrderedOperands, OrderingOperator, Outcome, Refusal, RoundingMode, ScalarLimits, Undefined,
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

fn run(operation: DecimalOperation<'_>, target: &DecimalType) -> Outcome<DecimalResult> {
    evaluate_decimal(operation, target, &mut Meter::new(UNLIMITED))
}

fn completed(outcome: Outcome<DecimalResult>) -> DecimalResult {
    match outcome {
        Outcome::Completed(result) => result,
        other => panic!("expected a completed decimal, got {other:?}"),
    }
}

/// `Decimal[lower,upper;0,scale;mode]`.
fn target(lower: i64, upper: i64, scale: u64, mode: RoundingMode) -> DecimalType {
    DecimalType::new(Integer::from(lower), Integer::from(upper), 0, scale, mode).unwrap()
}

/// `Decimal[-1000,1000;0,scale;mode]`.
fn wide(scale: u64, mode: RoundingMode) -> DecimalType {
    target(-1000, 1000, scale, mode)
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
            &wide(2, RoundingMode::Exact),
        ));
        assert_eq!(pair(result.value()), ((100, 2), (1, 0)));
        assert!(result.loss().is_none());
    }
}

#[trace("TC-185", "FR-140-AC-2")]
#[test]
fn d02_d04_exact_arithmetic_has_no_loss_in_every_mode() {
    for mode in RoundingMode::ALL {
        let sum = completed(run(
            DecimalOperation::Add(&dec(125, 2), &dec(75, 2)),
            &wide(2, mode),
        ));
        assert_eq!(pair(sum.value()), ((200, 2), (2, 0)));
        assert!(sum.loss().is_none());

        let quotient = completed(run(
            DecimalOperation::Divide(&dec(1, 0), &dec(8, 0)),
            &wide(3, mode),
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
    let target = |mode| wide(0, mode);
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
            assert_eq!(int(&loss.exact_denominator()), 2);
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
            &wide(2, RoundingMode::Exact)
        ),
        Outcome::Refused(Refusal::InexactDecimal)
    );
    let result = completed(run(
        DecimalOperation::Divide(&one, &three),
        &wide(2, RoundingMode::NearestEven),
    ));
    assert_eq!(pair(result.value()).0, (33, 2));
    let loss = result.loss().unwrap();
    assert_eq!(
        (
            int(loss.exact_numerator()),
            int(&loss.exact_denominator()),
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
                run(DecimalOperation::Divide(&dec(1, 0), &zero), &wide(2, mode)),
                Outcome::Undefined(Undefined::DivisionByZero)
            );
        }
    }
    assert_eq!(
        run(
            DecimalOperation::Round(&dec(25, 1)),
            &target(-2, 2, 0, RoundingMode::NearestAway)
        ),
        Outcome::Refused(Refusal::DecimalOutOfDomain)
    );
    let admit = |coefficient| {
        run(
            DecimalOperation::Round(&dec(coefficient, 0)),
            &target(-2, 2, 0, RoundingMode::Exact),
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
    integer_bits: 8,
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
        &wide(2, RoundingMode::NearestEven),
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
    assert_eq!(consumed, [8, 3, 2, 0, 0, 0, 0, 2, 5, 1]);

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
            next_charge: Integer::from(1_i64),
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
    // `sbits(1,2) = bits(1) + bits(100) = 8` on the expanded dividend.
    let narrow_bits = ScalarLimits {
        integer_bits: 7,
        ..D09
    };
    assert_eq!(
        d05(&mut Meter::new(narrow_bits)),
        Outcome::Incomplete(Incomplete {
            limit_kind: LimitKind::IntegerBits,
            limit: 7,
            consumed: 2,
            next_charge: Integer::from(8_i64),
            charge_point: ChargePoint::DecimalScaleExpansion,
        })
    );
    let narrow_shift = ScalarLimits {
        scale_expansion: 1,
        ..D09
    };
    assert_eq!(
        d05(&mut Meter::new(narrow_shift)),
        Outcome::Incomplete(Incomplete {
            limit_kind: LimitKind::ScaleExpansion,
            limit: 1,
            consumed: 0,
            next_charge: Integer::from(2_i64),
            charge_point: ChargePoint::DecimalScaleExpansion,
        })
    );

    for (work, point) in (0_u64..).zip(meter.admitted_charges()) {
        let mut denied = Meter::new(D09).with_injected_denial(InjectedDenial {
            point: *point,
            occurrence: 1,
        });
        assert_eq!(d05(&mut denied), work_denied(work, *point));
        assert_eq!(denied.consumed(LimitKind::ResultUnits), 0);
    }

    // An independently metered sibling is unaffected by the exhausted request.
    assert_eq!(
        pair(completed(d05(&mut Meter::new(D09))).value()).0,
        (33, 2)
    );
}

/// The injected-denial record after `work` admitted work units.
fn work_denied<T>(work: u64, point: ChargePoint) -> Outcome<T> {
    Outcome::Incomplete(Incomplete {
        limit_kind: LimitKind::WorkUnits,
        limit: work,
        consumed: work,
        next_charge: Integer::from(1_i64),
        charge_point: point,
    })
}

fn declared(lower: i64, upper: i64, min: u64, max: u64) -> Result<DecimalType, IllTyped> {
    DecimalType::new(
        Integer::from(lower),
        Integer::from(upper),
        min,
        max,
        RoundingMode::Exact,
    )
}

#[trace("TC-185", "FR-140-AC-5")]
#[test]
fn d10_malformed_decimal_types_are_ill_typed() {
    let malformed = Err(IllTyped {
        cause: IllTypedCause::MalformedDecimalType,
    });
    assert_eq!(declared(3, 2, 0, 0), malformed);
    assert_eq!(declared(0, 9, 2, 1), malformed);
    assert_eq!(declared(0, 9, 0, 4_294_967_296), malformed);
    assert!(declared(2, 2, 1, 1).is_ok());
    assert!(declared(0, 9, 0, 4_294_967_295).is_ok());
}

#[trace("TC-185", "FR-140-AC-5")]
#[test]
fn d11_membership_lifts_to_the_minimum_scale() {
    let wide_type = declared(0, 10_000, 2, 2).unwrap();
    for member in [dec(1, 0), dec(11, 1), dec(1000, 3), dec(9999, 2)] {
        assert!(wide_type.contains(&member), "{member:?}");
    }
    let narrow = declared(0, 9999, 2, 2).unwrap();
    assert!(!narrow.contains(&dec(100, 0)));
    assert!(narrow.contains(&dec(9999, 2)));
    assert!(!declared(-9, 9, 0, 2).unwrap().contains(&dec(1, 3)));
    // Equal values always have equal membership.
    for (a, b) in [
        (dec(1, 0), dec(1000, 3)),
        (dec(100, 0), dec(10_000, 2)),
        (dec(1, 3), dec(10, 4)),
    ] {
        for decimal_type in [&wide_type, &narrow] {
            assert_eq!(decimal_type.contains(&a), decimal_type.contains(&b));
        }
    }
}

#[trace("TC-185", "FR-140-AC-3", "FR-140-AC-5")]
#[test]
fn d12_a_rounded_coefficient_outside_the_domain_is_never_re_rounded() {
    let (one, three) = (dec(1, 0), dec(3, 0));
    let result = completed(run(
        DecimalOperation::Divide(&one, &three),
        &wide(2, RoundingMode::NearestEven),
    ));
    assert_eq!(pair(result.value()).0, (33, 2));
    let loss = result.loss().unwrap();
    assert_eq!(
        (int(loss.exact_numerator()), int(&loss.exact_denominator())),
        (1, 3)
    );
    assert_eq!(
        run(
            DecimalOperation::Divide(&one, &three),
            &target(-10, 10, 2, RoundingMode::NearestEven)
        ),
        Outcome::Refused(Refusal::DecimalOutOfDomain)
    );
}

const D13: ScalarLimits = ScalarLimits {
    integer_bits: 10,
    decimal_digits: 4,
    scale_expansion: 0,
    text_input_bytes: 0,
    text_scalars: 0,
    normalized_scalars: 0,
    unit_edges: 0,
    value_occurrences: 2,
    work_units: 4,
    result_units: 1,
};

#[trace("TC-185", "FR-140-AC-2", "FR-140-AC-6")]
#[test]
fn d13_discarded_zero_digits_are_not_a_rounding_step() {
    let (a, b) = (dec(150, 2), dec(2, 0));
    for mode in [RoundingMode::TowardZero, RoundingMode::Exact] {
        let decimal_type = target(-9, 9, 0, mode);
        let run = |limits| {
            evaluate_decimal(
                DecimalOperation::Multiply(&a, &b),
                &decimal_type,
                &mut Meter::new(limits),
            )
        };
        let mut meter = Meter::new(D13);
        let result = completed(evaluate_decimal(
            DecimalOperation::Multiply(&a, &b),
            &decimal_type,
            &mut meter,
        ));
        assert_eq!(pair(result.value()).0, (3, 0), "{mode:?}");
        assert!(result.loss().is_none());
        assert_eq!(
            meter.admitted_charges(),
            [
                ChargePoint::DecimalOperands,
                ChargePoint::DecimalScaleExpansion,
                ChargePoint::DecimalArithmetic,
                ChargePoint::DecimalResultRetain,
            ]
        );
        assert_eq!(
            run(ScalarLimits {
                work_units: 3,
                ..D13
            }),
            work_denied(3, ChargePoint::DecimalResultRetain)
        );
        assert_eq!(
            // `bits(150) + bits(2) = 10` on the retained `(150,2)`.
            run(ScalarLimits {
                integer_bits: 9,
                ..D13
            }),
            Outcome::Incomplete(Incomplete {
                limit_kind: LimitKind::IntegerBits,
                limit: 9,
                consumed: 8,
                next_charge: Integer::from(10_i64),
                charge_point: ChargePoint::DecimalArithmetic,
            })
        );
    }
}

#[trace("TC-185", "FR-140-AC-3", "FR-140-AC-6")]
#[test]
fn d14_strict_exact_refuses_before_rounding() {
    let run = |work_units| {
        let mut meter = Meter::new(ScalarLimits { work_units, ..D09 });
        let outcome = evaluate_decimal(
            DecimalOperation::Divide(&dec(1, 0), &dec(3, 0)),
            &wide(2, RoundingMode::Exact),
            &mut meter,
        );
        (outcome, meter.admitted_charges().to_vec())
    };
    let (outcome, charges) = run(3);
    assert_eq!(outcome, Outcome::Refused(Refusal::InexactDecimal));
    assert_eq!(
        charges,
        [
            ChargePoint::DecimalOperands,
            ChargePoint::DecimalScaleExpansion,
            ChargePoint::DecimalArithmetic,
        ]
    );
    assert_eq!(run(2).0, work_denied(2, ChargePoint::DecimalArithmetic));
}

#[trace("TC-185", "FR-140-AC-5", "FR-140-AC-6")]
#[test]
fn d15_zero_divisor_is_undefined_after_the_operands_charge() {
    let run = |work_units| {
        let mut meter = Meter::new(ScalarLimits { work_units, ..D09 });
        let outcome = evaluate_decimal(
            DecimalOperation::Divide(&dec(1, 0), &dec(0, 3)),
            &wide(2, RoundingMode::NearestEven),
            &mut meter,
        );
        (outcome, meter.admitted_charges().to_vec())
    };
    assert_eq!(
        run(1),
        (
            Outcome::Undefined(Undefined::DivisionByZero),
            vec![ChargePoint::DecimalOperands]
        )
    );
    assert_eq!(run(0).0, work_denied(0, ChargePoint::DecimalOperands));
}

#[trace("TC-185", "FR-140-AC-5", "FR-140-AC-6")]
#[test]
fn d16_membership_refusal_follows_rounding_and_precedes_retention() {
    let limits = ScalarLimits {
        integer_bits: 5,
        decimal_digits: 2,
        scale_expansion: 0,
        text_input_bytes: 0,
        text_scalars: 0,
        normalized_scalars: 0,
        unit_edges: 0,
        value_occurrences: 1,
        work_units: 4,
        result_units: 0,
    };
    let run = |limits| {
        let mut meter = Meter::new(limits);
        let outcome = evaluate_decimal(
            DecimalOperation::Round(&dec(25, 1)),
            &target(-2, 2, 0, RoundingMode::NearestAway),
            &mut meter,
        );
        (outcome, meter.admitted_charges().to_vec())
    };
    let (outcome, charges) = run(limits);
    assert_eq!(outcome, Outcome::Refused(Refusal::DecimalOutOfDomain));
    assert_eq!(
        charges,
        [
            ChargePoint::DecimalOperands,
            ChargePoint::DecimalScaleExpansion,
            ChargePoint::DecimalArithmetic,
            ChargePoint::DecimalRounding,
        ]
    );
    assert_eq!(
        run(ScalarLimits {
            work_units: 3,
            ..limits
        })
        .0,
        work_denied(3, ChargePoint::DecimalRounding)
    );
}

#[trace("TC-185", "FR-140-AC-2", "FR-140-AC-6")]
#[test]
fn d17_scale_expansion_uses_the_retained_representation() {
    let limits = ScalarLimits {
        integer_bits: 7,
        decimal_digits: 3,
        scale_expansion: 1,
        text_input_bytes: 0,
        text_scalars: 0,
        normalized_scalars: 0,
        unit_edges: 0,
        value_occurrences: 2,
        work_units: 4,
        result_units: 1,
    };
    let result = completed(evaluate_decimal(
        DecimalOperation::Divide(&dec(100, 2), &dec(1, 0)),
        &wide(2, RoundingMode::Exact),
        &mut Meter::new(limits),
    ));
    assert_eq!(pair(result.value()), ((100, 2), (1, 0)));
    assert!(result.loss().is_none());
}

#[trace("TC-185", "FR-140-AC-3", "FR-140-AC-6")]
#[test]
fn d18_division_expands_the_divisor_side() {
    let limits = ScalarLimits {
        integer_bits: 11,
        decimal_digits: 4,
        scale_expansion: 2,
        text_input_bytes: 0,
        text_scalars: 0,
        normalized_scalars: 0,
        unit_edges: 0,
        value_occurrences: 2,
        work_units: 5,
        result_units: 1,
    };
    let run = |limits| {
        evaluate_decimal(
            DecimalOperation::Divide(&dec(1234, 3), &dec(2, 0)),
            &wide(1, RoundingMode::NearestEven),
            &mut Meter::new(limits),
        )
    };
    let result = completed(run(limits));
    assert_eq!(pair(result.value()).0, (6, 1));
    let loss = result.loss().unwrap();
    assert_eq!(
        (
            int(loss.exact_numerator()),
            int(&loss.exact_denominator()),
            int(loss.rounded_coefficient()),
            loss.rounded_scale(),
            loss.mode()
        ),
        (617, 1000, 6, 1, RoundingMode::NearestEven)
    );
    assert_eq!(
        run(ScalarLimits {
            scale_expansion: 1,
            ..limits
        }),
        Outcome::Incomplete(Incomplete {
            limit_kind: LimitKind::ScaleExpansion,
            limit: 1,
            consumed: 0,
            next_charge: Integer::from(2_i64),
            charge_point: ChargePoint::DecimalScaleExpansion,
        })
    );
}

#[trace("TC-185", "FR-140-AC-5")]
#[test]
fn d19_membership_at_the_largest_scale_never_materializes_the_lift() {
    let decimal_type = declared(0, 10, 4_294_967_295, 4_294_967_295).unwrap();
    assert!(decimal_type.contains(&dec(0, 0)));
    assert!(!decimal_type.contains(&dec(1, 0)));
}

/// `ScalarLimitsV1` of D20/D21 with the text and unit counters at zero.
fn ordering_limits(integer_bits: u64, decimal_digits: u64, scale_expansion: u64) -> ScalarLimits {
    ScalarLimits {
        integer_bits,
        decimal_digits,
        scale_expansion,
        text_input_bytes: 0,
        text_scalars: 0,
        normalized_scalars: 0,
        unit_edges: 0,
        value_occurrences: 2,
        work_units: 3,
        result_units: 1,
    }
}

/// Order `left < right` and report the outcome, admitted points and the
/// consumed `[integer_bits, decimal_digits, scale_expansion,
/// value_occurrences, work_units, result_units]`.
fn order_less(
    left: &Decimal,
    right: &Decimal,
    limits: ScalarLimits,
) -> (Outcome<bool>, Vec<ChargePoint>, [u64; 6]) {
    let mut meter = Meter::new(limits);
    let outcome = order_numbers(
        OrderingOperator::Less,
        OrderedOperands::Decimals(left, right),
        &mut meter,
    );
    let consumed = [
        LimitKind::IntegerBits,
        LimitKind::DecimalDigits,
        LimitKind::ScaleExpansion,
        LimitKind::ValueOccurrences,
        LimitKind::WorkUnits,
        LimitKind::ResultUnits,
    ]
    .map(|kind| meter.consumed(kind));
    (outcome, meter.admitted_charges().to_vec(), consumed)
}

const ORDERING: [ChargePoint; 3] = [
    ChargePoint::OrderingOperands,
    ChargePoint::OrderingArithmetic,
    ChargePoint::OrderingResultRetain,
];

fn bits_denied(limit: u64, consumed: u64, next: u64) -> Outcome<bool> {
    Outcome::Incomplete(Incomplete {
        limit_kind: LimitKind::IntegerBits,
        limit,
        consumed,
        next_charge: Integer::from(next),
        charge_point: ChargePoint::OrderingArithmetic,
    })
}

#[trace("TC-185", "FR-140-AC-1", "FR-140-AC-6")]
#[test]
fn d20_decimal_ordering_charges_the_aligned_coefficients() {
    let (left, right) = (dec(15, 1), dec(2, 0));
    assert_eq!(
        order_less(&left, &right, ordering_limits(6, 2, 1)),
        (
            Outcome::Completed(true),
            ORDERING.to_vec(),
            [6, 2, 1, 2, 3, 1]
        )
    );
    let (outcome, admitted, consumed) = order_less(&left, &right, ordering_limits(5, 2, 1));
    assert_eq!(outcome, bits_denied(5, 4, 6));
    assert_eq!(admitted, [ChargePoint::OrderingOperands]);
    assert_eq!(consumed, [4, 2, 0, 2, 1, 0]);
}

#[trace("TC-185", "FR-140-AC-1", "FR-140-AC-6")]
#[test]
fn d21_decimal_ordering_measures_the_retained_representation() {
    let (left, right) = (dec(100, 2), dec(2, 0));
    assert_eq!(
        order_less(&left, &right, ordering_limits(9, 3, 2)),
        (
            Outcome::Completed(true),
            ORDERING.to_vec(),
            [9, 3, 2, 2, 3, 1]
        )
    );
    let (outcome, admitted, consumed) = order_less(&left, &right, ordering_limits(8, 3, 2));
    assert_eq!(outcome, bits_denied(8, 7, 9));
    assert_eq!(admitted, [ChargePoint::OrderingOperands]);
    assert_eq!(consumed, [7, 3, 0, 2, 1, 0]);
}

const VALUE_ACCOUNTING: &str = include_str!(
    "../resources/complete-value/quire-specification/proposals/quire-v1/definitions/value-accounting.md"
);

/// Charge-point families of the FR-150–FR-153 model domain still deferred
/// past #120's FR-153 rung. FR-153 itself implemented `lookup.*` and
/// `population.visit` (`src/model/population.rs`), so those two families are
/// no longer listed here. `binding.member`/`binding.subset-value` stay under
/// a deferred family from `ChargePoint`'s own perspective: the vendored
/// document's own "Population admission limits" section calls their
/// schedule `PopulationAdmissionLimitsV1` OF `quire.value.accounting/v1` —
/// the same overall document as `ChargePoint`'s own `ScalarLimitsV1`
/// schedule, but a separate, independently metered `u64` limits struct with
/// its own charge-point vocabulary (`model::population::AdmissionChargePoint`),
/// so neither point is, or should be, part of
/// `crate::value::accounting::ChargePoint` itself. `binding.member` IS
/// genuinely charged, just by that other schedule
/// (`IMPLEMENTED_ELSEWHERE_POINTS` below asserts this directly rather than
/// merely omitting it); so is `binding.subset-value`, as of #120 slice 4
/// (`model::population::admit_binding`'s subsetting-runtime loop).
const DEFERRED_FAMILIES: [&str; 7] = [
    "model",
    "graph",
    "dispatch",
    "normalize",
    "systems",
    "conformance",
    "binding",
];

/// `PopulationAdmissionLimitsV1` charge points genuinely implemented in this
/// crate, just not by `crate::value::accounting::ChargePoint` — by
/// `model::population::AdmissionChargePoint` instead. Filtered out of
/// [`DEFERRED_POINTS`] below (which is reserved for points nothing
/// implements) and asserted directly against that other enum in
/// `every_vendored_charge_point_is_named_in_table_order`, so the registry
/// test can tell "implemented elsewhere" apart from "unimplemented anywhere".
const IMPLEMENTED_ELSEWHERE_POINTS: [&str; 2] = ["binding.member", "binding.subset-value"];

/// Every charge point genuinely unimplemented anywhere in this crate, in
/// ascending order.
const DEFERRED_POINTS: [&str; 21] = [
    "conformance.axis",
    "dispatch.candidate",
    "dispatch.dominance",
    "dispatch.select",
    "dispatch.subtype",
    "graph.edge",
    "graph.expand",
    "graph.result-retain",
    "model.deref",
    "model.navigate",
    "normalize.conflict-check",
    "normalize.cycle-check",
    "normalize.declaration",
    "normalize.fact",
    "normalize.hash",
    "normalize.record",
    "normalize.redefinition-check",
    "systems.allocation",
    "systems.connection-condition",
    "systems.kind",
    "systems.resolve",
];

/// `sum.quantity` matches [`charge_point_codes`]'s `family.point` regex
/// incidentally: the `## Collection sum schedule` prose names it, alongside
/// `quire.op.collection.sum.decimal`, `sum.float32` and `sum.float64` (the
/// four multi-dot/digit-bearing siblings the regex already excludes), as one
/// of FR-322's checked-package sum-operator application identities, not a
/// `crate::value::accounting::ChargePoint` code -- the schedule charges only
/// the already-implemented `collection.visit`/`collection.result-retain`
/// plus each family's own existing addition schedule. Excluded here rather
/// than added to [`DEFERRED_POINTS`], because `is_deferred` would then need
/// a `sum` entry in [`DEFERRED_FAMILIES`] that misrepresents it as tracked
/// by some other charge-point enum, which it is not.
const NON_CHARGE_POINT_MATCHES: [&str; 1] = ["sum.quantity"];

/// The backticked `family.point` codes of `text`, in order of appearance.
fn charge_point_codes(text: &str) -> Vec<String> {
    text.split('`')
        .skip(1)
        .step_by(2)
        .filter(|code| {
            code.split_once('.').is_some_and(|(family, point)| {
                family.len() > 1
                    && !point.contains('.')
                    && code
                        .chars()
                        .all(|c| c.is_ascii_lowercase() || c == '.' || c == '-')
            })
        })
        .map(str::to_owned)
        .collect()
}

fn is_deferred(code: &str) -> bool {
    code.split_once('.')
        .is_some_and(|(family, _)| DEFERRED_FAMILIES.contains(&family))
}

/// The charge points of the vendored operation-family table, in table order.
fn vendored_charge_points() -> Vec<String> {
    let table = VALUE_ACCOUNTING
        .split("| Operation family | Ordered charge points |")
        .nth(1)
        .unwrap()
        .split("\n\n")
        .next()
        .unwrap();
    table
        .lines()
        .skip(2)
        .flat_map(|row| charge_point_codes(row.rsplit('|').nth(1).unwrap()))
        .collect()
}

#[trace("TC-185", "FR-140-AC-6")]
#[test]
fn every_vendored_charge_point_is_named_in_table_order() {
    let named: Vec<String> = ChargePoint::ALL
        .iter()
        .map(|point| point.as_str().to_owned())
        .collect();
    let mut value_points = Vec::new();
    for code in vendored_charge_points() {
        if !is_deferred(&code) && !value_points.contains(&code) {
            value_points.push(code);
        }
    }
    assert_eq!(named, value_points);
    for point in ChargePoint::ALL {
        assert_eq!(ChargePoint::from_code(point.as_str()), Some(point));
    }

    let mut deferred: Vec<String> = charge_point_codes(VALUE_ACCOUNTING)
        .into_iter()
        .filter(|code| !named.contains(code))
        .filter(|code| !IMPLEMENTED_ELSEWHERE_POINTS.contains(&code.as_str()))
        .filter(|code| !NON_CHARGE_POINT_MATCHES.contains(&code.as_str()))
        .collect();
    deferred.sort();
    deferred.dedup();
    assert_eq!(deferred, DEFERRED_POINTS);
    assert!(DEFERRED_POINTS.iter().all(|code| is_deferred(code)));
    assert!(DEFERRED_POINTS
        .iter()
        .all(|code| ChargePoint::from_code(code).is_none()));

    // `binding.member` and `binding.subset-value` are both genuinely charged
    // by `model::population::AdmissionChargePoint`'s own independent
    // schedule, just not by `crate::value::accounting::ChargePoint`.
    for &code in &IMPLEMENTED_ELSEWHERE_POINTS {
        assert!(
            is_deferred(code),
            "{code} must still be under a deferred family from ChargePoint's perspective"
        );
        assert!(
            ChargePoint::from_code(code).is_none(),
            "{code} must never be quire.value.accounting/v1's own ChargePoint"
        );
        assert!(
            AdmissionChargePoint::ALL
                .iter()
                .any(|point| point.as_str() == code),
            "{code} must be a real AdmissionChargePoint"
        );
    }
}

#[trace("TC-185", "FR-140-AC-1")]
#[test]
fn comparison_decides_extreme_scales_by_sign_and_aligned_magnitude() {
    use std::cmp::Ordering::{Equal, Greater, Less};
    let smallest = dec(1, u32::MAX);
    let cases = [
        (smallest.clone(), dec(0, 0), Greater),
        (dec(-1, u32::MAX), dec(0, 0), Less),
        (dec(0, u32::MAX), dec(0, 0), Equal),
        (smallest.clone(), dec(1, u32::MAX - 1), Less),
        (dec(10, u32::MAX), dec(1, u32::MAX - 1), Equal),
        (dec(11, u32::MAX), dec(1, u32::MAX - 1), Greater),
        (dec(-5, u32::MAX), dec(-1, u32::MAX - 1), Greater),
        (smallest.clone(), dec(-1, 0), Greater),
        (dec(999, 3), dec(1, 0), Less),
        (dec(1001, 3), dec(1, 0), Greater),
        (dec(-1001, 3), dec(-1, 0), Less),
        (dec(10, 1), dec(1, 0), Equal),
        (dec(12_345, 4), dec(12_346, 4), Less),
    ];
    for (left, right, expected) in cases {
        assert_eq!(left.compare(&right), expected, "{left:?} vs {right:?}");
        assert_eq!(
            right.compare(&left),
            expected.reverse(),
            "{right:?} vs {left:?}"
        );
    }
}

const TIGHT: ScalarLimits = ScalarLimits {
    integer_bits: 4,
    decimal_digits: 2,
    scale_expansion: 0,
    text_input_bytes: 0,
    text_scalars: 0,
    normalized_scalars: 0,
    unit_edges: 0,
    value_occurrences: 2,
    work_units: 5,
    result_units: 0,
};

#[trace("TC-185", "FR-140-AC-6")]
#[test]
fn working_scales_above_the_target_never_materialize_the_excess_power() {
    let arithmetic = [
        ChargePoint::DecimalOperands,
        ChargePoint::DecimalScaleExpansion,
        ChargePoint::DecimalArithmetic,
    ];
    let mut meter = Meter::new(UNLIMITED);
    assert_eq!(
        evaluate_decimal(
            DecimalOperation::Round(&dec(1, u32::MAX)),
            &target(0, 10, 0, RoundingMode::Exact),
            &mut meter
        ),
        Outcome::Refused(Refusal::InexactDecimal)
    );
    assert_eq!(meter.admitted_charges(), arithmetic);

    // Working scale 2 × u32::MAX: -6 × 10^-8589934590.
    let (left, right) = (dec(3, u32::MAX), dec(-2, u32::MAX));
    let product = DecimalOperation::Multiply(&left, &right);
    let mut meter = Meter::new(TIGHT);
    assert_eq!(
        evaluate_decimal(product, &target(0, 10, 0, RoundingMode::Exact), &mut meter),
        Outcome::Refused(Refusal::InexactDecimal)
    );
    assert_eq!(meter.admitted_charges(), arithmetic);
    assert_eq!(
        evaluate_decimal(
            product,
            &target(0, 10, 0, RoundingMode::TowardNegative),
            &mut Meter::new(TIGHT)
        ),
        Outcome::Refused(Refusal::DecimalOutOfDomain)
    );
    let mut meter = Meter::new(TIGHT);
    assert_eq!(
        evaluate_decimal(
            product,
            &target(0, 10, 0, RoundingMode::NearestEven),
            &mut meter
        ),
        Outcome::Incomplete(Incomplete {
            limit_kind: LimitKind::ResultUnits,
            limit: 0,
            consumed: 0,
            next_charge: 1_u64.into(),
            charge_point: ChargePoint::DecimalResultRetain,
        })
    );
    // `bits(3) + bits(2) = 4`: the operands bound the amount, not the
    // working scale.
    assert_eq!(meter.consumed(LimitKind::IntegerBits), 4);

    // The loss record keeps the working-scale power factored; only the
    // explicit denominator accessor would materialize it.
    let result = completed(run(product, &target(0, 10, 0, RoundingMode::NearestEven)));
    assert_eq!(pair(result.value()), ((0, 0), (0, 0)));
    let loss = result
        .loss()
        .expect("a nonzero discarded digit records a loss");
    assert_eq!(int(loss.exact_numerator()), -3);
    assert_eq!(int(loss.rounded_coefficient()), 0);
    assert_eq!(loss.mode(), RoundingMode::NearestEven);
}

/// `ScalarLimitsV1` of D22/D23 with the text and unit counters at zero.
fn retain_limits(integer_bits: u64, decimal_digits: u64, scale_expansion: u64) -> ScalarLimits {
    ScalarLimits {
        integer_bits,
        decimal_digits,
        scale_expansion,
        text_input_bytes: 0,
        text_scalars: 0,
        normalized_scalars: 0,
        unit_edges: 0,
        value_occurrences: 2,
        work_units: 4,
        result_units: 1,
    }
}

/// `(1,0) × (1,0)` into `Decimal[0,1;0,scale;exact]`, reporting the outcome,
/// admitted points and every consumed counter.
fn unit_product(
    scale: u64,
    limits: ScalarLimits,
) -> (Outcome<DecimalResult>, Vec<ChargePoint>, Vec<u64>) {
    let one = dec(1, 0);
    let mut meter = Meter::new(limits);
    let outcome = evaluate_decimal(
        DecimalOperation::Multiply(&one, &one),
        &target(0, 1, scale, RoundingMode::Exact),
        &mut meter,
    );
    let consumed = LimitKind::ALL.map(|kind| meter.consumed(kind)).to_vec();
    (outcome, meter.admitted_charges().to_vec(), consumed)
}

#[trace("TC-185", "FR-140-AC-6")]
#[test]
fn d22_result_retention_charges_the_target_scale_upscale_analytically() {
    let (outcome, admitted, consumed) = unit_product(1000, retain_limits(3323, 1001, 1000));
    let result = completed(outcome);
    let representation = result.value().representation();
    assert_eq!(representation.scale(), 1000);
    assert_eq!(
        big(&BigInt::from(10).pow(1000_u32)),
        *representation.coefficient()
    );
    assert!(result.loss().is_none());
    assert_eq!(
        admitted,
        [
            ChargePoint::DecimalOperands,
            ChargePoint::DecimalScaleExpansion,
            ChargePoint::DecimalArithmetic,
            ChargePoint::DecimalResultRetain,
        ]
    );
    assert_eq!(consumed, [3323, 1001, 1000, 0, 0, 0, 0, 2, 4, 1]);

    let denied = |limit_kind, limit, consumed, next: u64| {
        Outcome::Incomplete(Incomplete {
            limit_kind,
            limit,
            consumed,
            next_charge: Integer::from(next),
            charge_point: ChargePoint::DecimalResultRetain,
        })
    };
    assert_eq!(
        unit_product(1000, retain_limits(3322, 1001, 1000)).0,
        denied(LimitKind::IntegerBits, 3322, 2, 3323)
    );
    assert_eq!(
        unit_product(1000, retain_limits(3323, 1001, 999)).0,
        denied(LimitKind::ScaleExpansion, 999, 0, 1000)
    );
}

#[trace("TC-185", "FR-140-AC-6")]
#[test]
fn d23_the_largest_target_scale_is_sized_without_materializing_its_power() {
    let (outcome, admitted, consumed) = unit_product(
        4_294_967_295,
        ScalarLimits {
            integer_bits: u64::MAX,
            ..retain_limits(u64::MAX, 64, 4_294_967_295)
        },
    );
    assert_eq!(
        outcome,
        Outcome::Incomplete(Incomplete {
            limit_kind: LimitKind::DecimalDigits,
            limit: 64,
            consumed: 2,
            next_charge: Integer::from(4_294_967_296_u64),
            charge_point: ChargePoint::DecimalResultRetain,
        })
    );
    assert_eq!(
        admitted,
        [
            ChargePoint::DecimalOperands,
            ChargePoint::DecimalScaleExpansion,
            ChargePoint::DecimalArithmetic,
        ]
    );
    assert_eq!(consumed[LimitKind::ALL.len() - 2], 3);
}

fn big(value: &BigInt) -> Integer {
    value.to_string().parse().unwrap()
}

#[trace("TC-185", "FR-140-AC-3")]
#[test]
fn loss_records_strip_large_powers_of_five_exactly() {
    // `Round(v × 10^-s)` toward zero into `Decimal[-v,v;0,0]` records the
    // reduced `v / 10^s`.
    let loss_of = |coefficient: &BigInt, scale: u32| {
        let decimal_type = DecimalType::new(
            big(&-coefficient),
            big(coefficient),
            0,
            0,
            RoundingMode::TowardZero,
        )
        .unwrap();
        let source = Decimal::new(big(coefficient), scale);
        let result = completed(run(DecimalOperation::Round(&source), &decimal_type));
        let loss = result
            .loss()
            .expect("a nonzero discarded digit records a loss");
        (loss.exact_numerator().clone(), loss.exact_denominator())
    };
    let pow = |base: u32, exponent: u32| BigInt::from(base).pow(exponent);
    let large = pow(5, 4096) * 7;
    // 5^4096 × 7 / 10^5000 = 7 / (2^5000 × 5^904).
    assert_eq!(
        loss_of(&large, 5000),
        (Integer::from(7_i64), big(&(pow(2, 5000) * pow(5, 904))))
    );
    // The scale caps stripping: 5^4096 × 7 / 10^100 = 5^3996 × 7 / 2^100.
    assert_eq!(
        loss_of(&large, 100),
        (big(&(pow(5, 3996) * 7)), big(&pow(2, 100)))
    );
    for (coefficient, numerator, denominator) in [(1, 1_i64, 1000_i64), (5, 1, 200), (75, 3, 40)] {
        assert_eq!(
            loss_of(&BigInt::from(coefficient), 3),
            (Integer::from(numerator), Integer::from(denominator))
        );
    }
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
                        let decimal_type = target(-BOUND, BOUND, u64::from(scale), mode);
                        let mut meter = Meter::new(UNLIMITED);
                        let outcome = evaluate_decimal(operation, &decimal_type, &mut meter);
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
                                assert_eq!(
                                    result.value().representation().scale(),
                                    scale,
                                    "{case}"
                                );
                                let actual = result.loss().map(|loss| {
                                    assert_eq!(loss.mode(), mode, "{case}");
                                    assert_eq!(loss.rounded_scale(), scale, "{case}");
                                    (
                                        int(loss.exact_numerator()),
                                        int(&loss.exact_denominator()),
                                        int(loss.rounded_coefficient()),
                                    )
                                });
                                assert_eq!(actual, loss, "{case}");
                            }
                            (_, other) => panic!("{case}: oracle disagrees with {other:?}"),
                        }
                        for (work, point) in (0_u64..).zip(meter.admitted_charges()) {
                            let mut denied =
                                Meter::new(UNLIMITED).with_injected_denial(InjectedDenial {
                                    point: *point,
                                    occurrence: 1,
                                });
                            assert_eq!(
                                evaluate_decimal(operation, &decimal_type, &mut denied),
                                work_denied(work, *point),
                                "{case}"
                            );
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
