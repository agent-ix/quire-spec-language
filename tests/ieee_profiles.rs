// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-193 IEEE exceptional and rounding profiles over the real `value` boundary.
//!
//! Vectors F01–F31 exercise the closed IEEE profile's rounding directions,
//! exceptional cases and accounting. The generated class matrix uses an
//! independent oracle: exact `BigInt` rationals and a binary search over the
//! ordered bit patterns of each width, with rounding, overflow and
//! tininess-after-rounding decided by rational comparisons. No host
//! floating-point value appears on either side.

use std::cmp::Ordering;
use std::collections::BTreeSet;
use std::sync::OnceLock;

use ix_trace_rs::trace;
use num_bigint::BigInt;
use num_traits::{One, Signed, Zero};
use quire_exact::{Integer, IntegerInterval};
use quire_spec_language::value::{
    compare_ieee, convert_ieee_width, evaluate_ieee, exact_to_ieee, ieee_intrinsic_identities,
    ieee_to_exact, AdmittedIeeeProfile, CatalogRole, ChargePoint, Decimal, DecimalType,
    DefinitionLock, DefinitionReference, DefinitionRevision, ExactScalar, IeeeComparison,
    IeeeExact, IeeeExactLoss, IeeeExactTarget, IeeeFlag, IeeeFlags, IeeeOperand, IeeeOperation,
    IeeeResult, IeeeValue, IeeeWidth, IllTyped, IllTypedCause, Incomplete, InjectedDenial,
    LimitKind, Meter, Outcome, PackageCause, PackageRefusalCode, Rational, RationalDomain, Refusal,
    RoundingMode, ScalarLimits, Undefined, IEEE_DEFINITION,
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

/// The F17/F18 tuple.
const F17: ScalarLimits = ScalarLimits {
    integer_bits: 32,
    decimal_digits: 0,
    scale_expansion: 0,
    text_input_bytes: 0,
    text_scalars: 0,
    normalized_scalars: 0,
    unit_edges: 0,
    value_occurrences: 2,
    work_units: 2,
    result_units: 1,
};

/// The F19 tuple.
const F19: ScalarLimits = ScalarLimits {
    value_occurrences: 1,
    work_units: 4,
    ..F17
};

/// The binary64 F10 tuple.
const F10: ScalarLimits = ScalarLimits {
    integer_bits: 64,
    value_occurrences: 2,
    work_units: 4,
    ..F17
};

const DIRECTIONS: [RoundingMode; 5] = [
    RoundingMode::NearestEven,
    RoundingMode::NearestAway,
    RoundingMode::TowardZero,
    RoundingMode::TowardPositive,
    RoundingMode::TowardNegative,
];

const FINITE_CHARGES: [ChargePoint; 4] = [
    ChargePoint::IeeeOperands,
    ChargePoint::IeeeExactIntermediate,
    ChargePoint::IeeeRound,
    ChargePoint::IeeeResultRetain,
];

const CLASSIFIED_CHARGES: [ChargePoint; 2] =
    [ChargePoint::IeeeOperands, ChargePoint::IeeeResultRetain];

/// A finite IEEE-to-exact conversion's charges.
const TO_EXACT_CHARGES: [ChargePoint; 3] = [
    ChargePoint::IeeeOperands,
    ChargePoint::IeeeExactIntermediate,
    ChargePoint::IeeeResultRetain,
];

fn big(value: &BigInt) -> Integer {
    value.to_string().parse().unwrap()
}

/// `Rational[lo, hi; dmin, dmax]`.
fn rational_type(lo: &BigInt, hi: &BigInt, dmin: &BigInt, dmax: &BigInt) -> RationalDomain {
    RationalDomain::new(
        IntegerInterval::new(big(lo), big(hi)).unwrap(),
        IntegerInterval::new(big(dmin), big(dmax)).unwrap(),
    )
    .unwrap()
}

/// A `Rational[..]` type holding every finite binary32 and binary64 value.
fn every_finite_ieee() -> RationalDomain {
    let numerator = BigInt::one() << 1024_u32;
    rational_type(
        &-numerator.clone(),
        &numerator,
        &BigInt::one(),
        &(BigInt::one() << 1074_u32),
    )
}

fn to_rational(value: IeeeValue, domain: &RationalDomain, meter: &mut Meter) -> Outcome<IeeeExact> {
    ieee_to_exact(profile(), value, IeeeExactTarget::Rational(domain), meter)
        .expect("a Rational[..] target is well-typed")
}

fn ratio(numerator: i64, denominator: i64) -> Rational {
    Rational::new(Integer::from(numerator), Integer::from(denominator)).unwrap()
}

fn profile() -> &'static AdmittedIeeeProfile {
    static PROFILE: OnceLock<AdmittedIeeeProfile> = OnceLock::new();
    PROFILE.get_or_init(|| {
        let lock = DefinitionLock::pinned();
        lock.admit_ieee_profile(&[ieee_reference(lock)], &[])
            .unwrap()
    })
}

/// A well-formed [`DefinitionReference`] for the IEEE profile role, built
/// from the catalog's own identity/authority/revision fields. There is no
/// digest to carry over: the catalog holds none, so this uses a placeholder
/// that admission never inspects.
fn ieee_reference(lock: &DefinitionLock) -> DefinitionReference {
    let entry = lock.entry(CatalogRole::IeeeProfile).unwrap();
    DefinitionReference {
        authority: entry.authority.to_owned(),
        identity: entry.identity.to_owned(),
        revision: DefinitionRevision {
            namespace: entry.revision_namespace.to_owned(),
            value: entry.revision_value.to_owned(),
        },
        digest_domain: "quire.definition.bytes/v1".to_owned(),
        digest: "0".repeat(64),
    }
}

fn f32v(bits: u32) -> IeeeValue {
    IeeeValue::binary32(bits)
}

fn f64v(bits: u64) -> IeeeValue {
    IeeeValue::binary64(bits)
}

fn flags(list: &[IeeeFlag]) -> IeeeFlags {
    list.iter().copied().collect()
}

fn eval_with(
    operation: IeeeOperation,
    mode: RoundingMode,
    meter: &mut Meter,
) -> Outcome<IeeeResult> {
    evaluate_ieee(profile(), operation, mode, meter).expect("same-width operands")
}

fn eval(operation: IeeeOperation, mode: RoundingMode) -> Outcome<IeeeResult> {
    eval_with(operation, mode, &mut Meter::new(UNLIMITED))
}

fn done(outcome: Outcome<IeeeResult>) -> IeeeResult {
    match outcome {
        Outcome::Completed(result) => result,
        other => panic!("expected completed bits, got {other:?}"),
    }
}

fn bits_flags(outcome: Outcome<IeeeResult>) -> (u64, IeeeFlags) {
    let result = done(outcome);
    (result.value().bits(), result.flags())
}

fn compare(comparison: IeeeComparison, left: IeeeValue, right: IeeeValue) -> bool {
    match compare_ieee(
        profile(),
        comparison,
        left,
        right,
        &mut Meter::new(UNLIMITED),
    ) {
        Ok(Outcome::Completed(value)) => value,
        other => panic!("{comparison:?}: {other:?}"),
    }
}

fn injected(point: ChargePoint) -> Meter {
    Meter::new(UNLIMITED).with_injected_denial(InjectedDenial {
        point,
        occurrence: 1,
    })
}

fn assert_denied<T: std::fmt::Debug>(outcome: Outcome<T>, point: ChargePoint, consumed: u64) {
    match outcome {
        Outcome::Incomplete(record) => {
            assert_eq!(record.charge_point, point);
            assert_eq!(record.limit_kind, LimitKind::WorkUnits);
            assert_eq!(
                (record.consumed, record.next_charge),
                (consumed, Integer::from(1_u64))
            );
        }
        other => panic!("{point:?}: expected incomplete, got {other:?}"),
    }
}

// ---- comparison vectors ------------------------------------------------------------

#[trace("TC-193", "FR-148-AC-1", "FR-148-AC-5", "TC-202", "FR-078-AC-3")]
#[test]
fn f01_f02_f02b_zeros_and_nans_separate_equality_order_and_identity() {
    use IeeeComparison::{BitIdentical, NumericEqual, TotalOrder};
    let (positive, negative) = (f32v(0x0000_0000), f32v(0x8000_0000));
    assert!(compare(NumericEqual, positive, negative));
    assert!(!compare(BitIdentical, positive, negative));
    assert!(compare(TotalOrder, negative, positive));
    assert!(!compare(TotalOrder, positive, negative));

    let (nan1, nan2) = (f32v(0x7fc0_0001), f32v(0x7fc0_0002));
    for nan in [nan1, nan2] {
        assert!(!compare(NumericEqual, nan, nan));
        assert!(compare(BitIdentical, nan, nan));
    }
    assert!(compare(TotalOrder, nan1, nan2));
    assert!(!compare(TotalOrder, nan2, nan1));

    let (negative2, negative1) = (f32v(0xffc0_0002), f32v(0xffc0_0001));
    assert!(compare(TotalOrder, negative2, negative1));
    assert!(!compare(TotalOrder, negative1, negative2));
    let (signaling, quiet) = (f32v(0x7f80_0001), f32v(0x7fc0_0001));
    assert!(compare(TotalOrder, signaling, quiet));
    assert!(!compare(TotalOrder, quiet, signaling));
}

#[trace("TC-193", "FR-148-AC-1", "FR-148-AC-5", "TC-202", "FR-078-AC-3")]
#[test]
fn f05_infinities_and_finite_extrema_follow_value_order() {
    use IeeeComparison::{BitIdentical, NumericEqual, TotalOrder};
    // -inf, -max, -min subnormal, -0, +0, +min subnormal, +max, +inf.
    let ordered = [
        0xff80_0000,
        0xff7f_ffff,
        0x8000_0001,
        0x8000_0000,
        0x0000_0000,
        0x0000_0001,
        0x7f7f_ffff,
        0x7f80_0000,
    ]
    .map(f32v);
    for (i, left) in ordered.iter().enumerate() {
        for (j, right) in ordered.iter().enumerate() {
            let zeros = matches!((i, j), (3, 4) | (4, 3));
            assert_eq!(compare(TotalOrder, *left, *right), i <= j, "{i} {j}");
            assert_eq!(compare(NumericEqual, *left, *right), i == j || zeros);
            assert_eq!(compare(BitIdentical, *left, *right), i == j);
        }
    }
    // Bit identity follows width plus bits: the same bit value in binary64 is a
    // different, ill-typed operand.
    assert!(compare(
        BitIdentical,
        f64v(0x7ff0_0000_0000_0000),
        f64v(0x7ff0_0000_0000_0000)
    ));
}

#[trace("TC-193", "FR-148-AC-1", "TC-202", "FR-078-AC-3")]
#[test]
fn f06_cross_width_comparison_is_ill_typed_until_an_explicit_conversion() {
    let (narrow, wide) = (f32v(0x3f80_0000), f64v(0x3ff0_0000_0000_0000));
    let ill_typed = IllTyped {
        cause: IllTypedCause::DistinctIeeeWidths,
    };
    for comparison in IeeeComparison::ALL {
        let mut meter = Meter::new(UNLIMITED);
        assert_eq!(
            compare_ieee(profile(), comparison, narrow, wide, &mut meter),
            Err(ill_typed)
        );
        assert!(meter.admitted_charges().is_empty());
    }
    let mut meter = Meter::new(UNLIMITED);
    assert_eq!(
        evaluate_ieee(
            profile(),
            IeeeOperation::Add(narrow, wide),
            RoundingMode::NearestEven,
            &mut meter
        ),
        Err(ill_typed)
    );
    assert!(meter.admitted_charges().is_empty());

    let converted = done(convert_ieee_width(
        profile(),
        narrow,
        IeeeWidth::Binary64,
        RoundingMode::Exact,
        &mut Meter::new(UNLIMITED),
    ));
    assert_eq!(converted.value(), wide);
    assert!(converted.flags().is_empty());
    assert!(compare(
        IeeeComparison::NumericEqual,
        converted.value(),
        wide
    ));
    assert!(compare(
        IeeeComparison::BitIdentical,
        converted.value(),
        wide
    ));
}

// ---- NaN propagation ------------------------------------------------------------

#[trace("TC-193", "FR-148-AC-6", "TC-202", "FR-078-AC-3")]
#[test]
fn f03_f04_f04b_leftmost_nan_is_retained_and_signaling_raises_invalid() {
    let even = RoundingMode::NearestEven;
    assert_eq!(
        bits_flags(eval(
            IeeeOperation::Add(f32v(0xffc0_0021), f32v(0x7fc0_0012)),
            even
        )),
        (0xffc0_0021, IeeeFlags::EMPTY)
    );
    assert_eq!(
        bits_flags(eval(
            IeeeOperation::Add(f32v(0x7f80_0021), f32v(0x7fc0_0012)),
            even
        )),
        (0x7fc0_0021, flags(&[IeeeFlag::Invalid]))
    );
    assert_eq!(
        bits_flags(eval(
            IeeeOperation::Add(f32v(0x7fc0_0012), f32v(0xff80_0021)),
            even
        )),
        (0x7fc0_0012, flags(&[IeeeFlag::Invalid]))
    );
    assert_eq!(
        bits_flags(eval(
            IeeeOperation::FusedMultiplyAdd(
                f32v(0x3f80_0000),
                f32v(0x7fc0_0012),
                f32v(0xff80_0021)
            ),
            even
        )),
        (0x7fc0_0012, flags(&[IeeeFlag::Invalid]))
    );
    // Strict `exact` retains the NaN result rather than refusing it.
    assert_eq!(
        bits_flags(eval(
            IeeeOperation::Add(f32v(0x7f80_0021), f32v(0x7fc0_0012)),
            RoundingMode::Exact
        )),
        (0x7fc0_0021, flags(&[IeeeFlag::Invalid]))
    );
}

// ---- rounding ---------------------------------------------------------------------

fn rounding_table(
    operation: IeeeOperation,
    down: u64,
    up: u64,
    up_modes: &[RoundingMode],
) -> Vec<(RoundingMode, u64)> {
    DIRECTIONS
        .into_iter()
        .map(|mode| {
            let result = done(eval(operation, mode));
            let expected = if up_modes.contains(&mode) { up } else { down };
            assert_eq!(result.value().bits(), expected, "{mode:?}");
            assert_eq!(result.flags(), flags(&[IeeeFlag::Inexact]), "{mode:?}");
            assert_eq!(result.provenance().rounding(), mode);
            assert_eq!(result.provenance().width(), operation_width(operation));
            assert_eq!(result.provenance().definition(), IEEE_DEFINITION);
            (mode, result.value().bits())
        })
        .collect()
}

fn operation_width(operation: IeeeOperation) -> IeeeWidth {
    match operation {
        IeeeOperation::Add(a, _)
        | IeeeOperation::Subtract(a, _)
        | IeeeOperation::Multiply(a, _)
        | IeeeOperation::Divide(a, _)
        | IeeeOperation::SquareRoot(a)
        | IeeeOperation::FusedMultiplyAdd(a, _, _) => a.width(),
    }
}

#[trace("TC-193", "FR-148-AC-2", "TC-202", "FR-078-AC-3")]
#[test]
fn f07_f08_f10_each_direction_rounds_once_and_changes_provenance() {
    use RoundingMode::{NearestAway, TowardNegative, TowardPositive};
    let f07 = rounding_table(
        IeeeOperation::Add(f32v(0x3f80_0000), f32v(0x3380_0000)),
        0x3f80_0000,
        0x3f80_0001,
        &[NearestAway, TowardPositive],
    );
    rounding_table(
        IeeeOperation::Add(f32v(0xbf80_0000), f32v(0xb380_0000)),
        0xbf80_0000,
        0xbf80_0001,
        &[NearestAway, TowardNegative],
    );
    rounding_table(
        IeeeOperation::Add(f64v(0x3ff0_0000_0000_0000), f64v(0x3ca0_0000_0000_0000)),
        0x3ff0_0000_0000_0000,
        0x3ff0_0000_0000_0001,
        &[NearestAway, TowardPositive],
    );
    // A changed rounding mode changes both the bits and the run provenance.
    let distinct_bits: BTreeSet<_> = f07.iter().map(|(_, bits)| *bits).collect();
    assert_eq!(distinct_bits.len(), 2);
}

#[trace("TC-193", "FR-148-AC-6", "TC-202", "FR-078-AC-3")]
#[test]
fn f09_f14_strict_exact_refuses_with_would_be_flags_and_no_bits() {
    assert_eq!(
        eval(
            IeeeOperation::Add(f32v(0x3f80_0000), f32v(0x3380_0000)),
            RoundingMode::Exact
        ),
        Outcome::Refused(Refusal::IeeeNotExact {
            would_be: flags(&[IeeeFlag::Inexact])
        })
    );
    let exact = done(eval(
        IeeeOperation::Add(f32v(0x3f80_0000), f32v(0x3400_0000)),
        RoundingMode::Exact,
    ));
    assert_eq!(
        (exact.value().bits(), exact.flags()),
        (0x3f80_0001, IeeeFlags::EMPTY)
    );
    assert_eq!(exact.provenance().rounding(), RoundingMode::Exact);

    let overflow = IeeeOperation::Add(f32v(0x7f7f_ffff), f32v(0x7f7f_ffff));
    let overflow_flags = flags(&[IeeeFlag::Overflow, IeeeFlag::Inexact]);
    assert_eq!(
        bits_flags(eval(overflow, RoundingMode::NearestEven)),
        (0x7f80_0000, overflow_flags)
    );
    assert_eq!(
        eval(overflow, RoundingMode::Exact),
        Outcome::Refused(Refusal::IeeeNotExact {
            would_be: overflow_flags
        })
    );
}

#[trace("TC-193", "FR-148-AC-4", "TC-202", "FR-078-AC-3")]
#[test]
fn f11_fused_multiply_add_rounds_once_unlike_multiply_then_add() {
    let (a, b, c) = (f32v(0x3f80_0001), f32v(0x3f7f_fffe), f32v(0xbf80_0000));
    let even = RoundingMode::NearestEven;
    let fused = done(eval(IeeeOperation::FusedMultiplyAdd(a, b, c), even));
    assert_eq!(
        (fused.value().bits(), fused.flags()),
        (0xa880_0000, IeeeFlags::EMPTY)
    );
    let product = done(eval(IeeeOperation::Multiply(a, b), even));
    assert_eq!(product.flags(), flags(&[IeeeFlag::Inexact]));
    assert_eq!(
        bits_flags(eval(IeeeOperation::Add(product.value(), c), even)),
        (0x0000_0000, IeeeFlags::EMPTY)
    );
}

#[trace("TC-193", "FR-148-AC-6", "TC-202", "FR-078-AC-3")]
#[test]
fn f12_f13_invalid_and_divide_by_zero_under_every_policy_are_operation_local() {
    let invalid = flags(&[IeeeFlag::Invalid]);
    let divide = flags(&[IeeeFlag::DivideByZero]);
    for mode in RoundingMode::ALL {
        for operation in [
            IeeeOperation::Multiply(f32v(0x0000_0000), f32v(0x7f80_0000)),
            IeeeOperation::SquareRoot(f32v(0xbf80_0000)),
        ] {
            assert_eq!(
                bits_flags(eval(operation, mode)),
                (0x7fc0_0000, invalid),
                "{mode:?}"
            );
        }
        for (dividend, divisor, expected) in [
            (0x3f80_0000, 0x0000_0000, 0x7f80_0000),
            (0x3f80_0000, 0x8000_0000, 0xff80_0000),
            (0xbf80_0000, 0x0000_0000, 0xff80_0000),
            (0xbf80_0000, 0x8000_0000, 0x7f80_0000),
        ] {
            let mut meter = Meter::new(UNLIMITED);
            assert_eq!(
                bits_flags(eval_with(
                    IeeeOperation::Divide(f32v(dividend), f32v(divisor)),
                    mode,
                    &mut meter
                )),
                (expected, divide),
                "{mode:?}"
            );
            // The next operation on the same meter starts from a fresh set.
            let next = done(eval_with(
                IeeeOperation::Add(f32v(0x3f80_0000), f32v(0x3400_0000)),
                RoundingMode::Exact,
                &mut meter,
            ));
            assert_eq!(
                (next.value().bits(), next.flags()),
                (0x3f80_0001, IeeeFlags::EMPTY)
            );
        }
    }
}

#[trace("TC-193", "FR-148-AC-6", "TC-202", "FR-078-AC-3")]
#[test]
fn f15_f16_exact_subnormals_raise_nothing_and_tiny_inexact_underflows() {
    let even = RoundingMode::NearestEven;
    assert_eq!(
        bits_flags(eval(
            IeeeOperation::Multiply(f32v(0x0080_0000), f32v(0x3f00_0000)),
            even
        )),
        (0x0040_0000, IeeeFlags::EMPTY)
    );
    assert_eq!(
        bits_flags(eval(
            IeeeOperation::Multiply(f32v(0x0080_0000), f32v(0x3f00_0000)),
            RoundingMode::Exact
        )),
        (0x0040_0000, IeeeFlags::EMPTY)
    );
    assert_eq!(
        bits_flags(eval(
            IeeeOperation::Multiply(f32v(0x0000_0001), f32v(0x3f00_0000)),
            even
        )),
        (
            0x0000_0000,
            flags(&[IeeeFlag::Underflow, IeeeFlag::Inexact])
        )
    );
}

// ---- accounting -------------------------------------------------------------------

#[trace("TC-193", "FR-148-AC-7", "TC-202", "FR-078-AC-3")]
#[test]
fn f17_f18_classified_paths_charge_only_operands_and_retention() {
    let nan_equal = |meter: &mut Meter| {
        compare_ieee(
            profile(),
            IeeeComparison::NumericEqual,
            f32v(0x7fc0_0001),
            f32v(0x3f80_0000),
            meter,
        )
        .unwrap()
    };
    let mut meter = Meter::new(F17);
    assert_eq!(nan_equal(&mut meter), Outcome::Completed(false));
    assert_eq!(meter.admitted_charges(), CLASSIFIED_CHARGES);
    assert_eq!(meter.consumed(LimitKind::IntegerBits), 32);
    assert_eq!(meter.consumed(LimitKind::ValueOccurrences), 2);
    assert_eq!(meter.consumed(LimitKind::WorkUnits), 2);
    assert_eq!(meter.consumed(LimitKind::ResultUnits), 1);
    for (index, point) in CLASSIFIED_CHARGES.into_iter().enumerate() {
        let mut denied = Meter::new(F17).with_injected_denial(InjectedDenial {
            point,
            occurrence: 1,
        });
        assert_denied(nan_equal(&mut denied), point, index as u64);
    }
    assert!(matches!(
        nan_equal(&mut Meter::new(ScalarLimits {
            work_units: 1,
            ..F17
        })),
        Outcome::Incomplete(Incomplete {
            limit_kind: LimitKind::WorkUnits,
            charge_point: ChargePoint::IeeeResultRetain,
            ..
        })
    ));

    let invalid = IeeeOperation::Multiply(f32v(0x0000_0000), f32v(0x7f80_0000));
    let mut meter = Meter::new(F17);
    assert_eq!(
        bits_flags(eval_with(invalid, RoundingMode::NearestEven, &mut meter)),
        (0x7fc0_0000, flags(&[IeeeFlag::Invalid]))
    );
    assert_eq!(meter.admitted_charges(), CLASSIFIED_CHARGES);
    let mut denied = Meter::new(F17).with_injected_denial(InjectedDenial {
        point: ChargePoint::IeeeResultRetain,
        occurrence: 1,
    });
    assert_denied(
        eval_with(invalid, RoundingMode::NearestEven, &mut denied),
        ChargePoint::IeeeResultRetain,
        1,
    );
    assert_eq!(
        eval_with(
            invalid,
            RoundingMode::NearestEven,
            &mut Meter::new(ScalarLimits {
                result_units: 0,
                ..F17
            })
        ),
        Outcome::Incomplete(Incomplete {
            limit_kind: LimitKind::ResultUnits,
            limit: 0,
            consumed: 0,
            next_charge: Integer::from(1_u64),
            charge_point: ChargePoint::IeeeResultRetain,
        })
    );
}

#[trace("TC-193", "FR-148-AC-7", "TC-202", "FR-078-AC-3")]
#[test]
fn f19_irrational_square_root_charges_the_fixed_width_allowance() {
    let root = IeeeOperation::SquareRoot(f32v(0x4000_0000));
    let mut meter = Meter::new(F19);
    assert_eq!(
        bits_flags(eval_with(root, RoundingMode::NearestEven, &mut meter)),
        (0x3fb5_04f3, flags(&[IeeeFlag::Inexact]))
    );
    assert_eq!(meter.admitted_charges(), FINITE_CHARGES);
    let consumed: Vec<_> = LimitKind::ALL.map(|kind| meter.consumed(kind)).to_vec();
    assert_eq!(consumed, [32, 0, 0, 0, 0, 0, 0, 1, 4, 1]);
    for (index, point) in FINITE_CHARGES.into_iter().enumerate() {
        let mut denied = Meter::new(F19).with_injected_denial(InjectedDenial {
            point,
            occurrence: 1,
        });
        assert_denied(
            eval_with(root, RoundingMode::NearestEven, &mut denied),
            point,
            index as u64,
        );
        assert_eq!(denied.consumed(LimitKind::ResultUnits), 0);
    }
}

#[trace("TC-193", "FR-148-AC-7", "TC-202", "FR-078-AC-3")]
#[test]
fn f10_binary64_limit_tuple_succeeds_and_its_final_charge_denial_is_incomplete() {
    let f10 = IeeeOperation::Add(f64v(0x3ff0_0000_0000_0000), f64v(0x3ca0_0000_0000_0000));
    let mut meter = Meter::new(F10);
    assert_eq!(
        bits_flags(eval_with(f10, RoundingMode::NearestEven, &mut meter)),
        (0x3ff0_0000_0000_0000, flags(&[IeeeFlag::Inexact]))
    );
    assert_eq!(meter.admitted_charges(), FINITE_CHARGES);
    let consumed: Vec<_> = LimitKind::ALL.map(|kind| meter.consumed(kind)).to_vec();
    assert_eq!(consumed, [64, 0, 0, 0, 0, 0, 0, 2, 4, 1]);

    let mut denied = Meter::new(F10).with_injected_denial(InjectedDenial {
        point: ChargePoint::IeeeResultRetain,
        occurrence: 1,
    });
    assert_denied(
        eval_with(f10, RoundingMode::NearestEven, &mut denied),
        ChargePoint::IeeeResultRetain,
        3,
    );
    assert_eq!(
        eval_with(
            f10,
            RoundingMode::NearestEven,
            &mut Meter::new(ScalarLimits {
                work_units: 3,
                ..F10
            })
        ),
        Outcome::Incomplete(Incomplete {
            limit_kind: LimitKind::WorkUnits,
            limit: 3,
            consumed: 3,
            next_charge: Integer::from(1_u64),
            charge_point: ChargePoint::IeeeResultRetain,
        })
    );
    assert_eq!(
        eval_with(
            f10,
            RoundingMode::NearestEven,
            &mut Meter::new(ScalarLimits {
                integer_bits: 63,
                ..F10
            })
        ),
        Outcome::Incomplete(Incomplete {
            limit_kind: LimitKind::IntegerBits,
            limit: 63,
            consumed: 0,
            next_charge: Integer::from(64_u64),
            charge_point: ChargePoint::IeeeOperands,
        })
    );
    // An independently metered sibling keeps its own bits and flags.
    assert_eq!(
        bits_flags(eval(f10, RoundingMode::TowardPositive)),
        (0x3ff0_0000_0000_0001, flags(&[IeeeFlag::Inexact]))
    );
}

// ---- admission --------------------------------------------------------------------

#[trace("TC-193", "FR-148-AC-10")]
#[test]
fn semantic_admission_refuses_missing_repeated_mismatched_or_reserved_bindings() {
    let lock = DefinitionLock::pinned();
    let mut roles: Vec<&str> = lock
        .always_roles()
        .iter()
        .map(|role| role.as_str())
        .collect();
    roles.push("ieee_profile");
    assert!(lock.admit_selection(&["ieee_operation"], &roles).is_ok());

    let reference = ieee_reference(lock);
    assert_eq!(reference.identity, IEEE_DEFINITION);
    let admitted = lock
        .admit_ieee_profile(std::slice::from_ref(&reference), &["acme::fma"])
        .unwrap();
    assert_eq!(admitted.definition(), &reference);

    let mut revision = reference.clone();
    revision.revision.value = "1-draft.2".to_owned();
    let mut version = reference.clone();
    version.identity = "quire.value.ieee754-2019-default/v2".to_owned();
    let cases = [
        (
            lock.admit_ieee_profile(&[], &[]),
            PackageCause::MissingMember,
        ),
        (
            lock.admit_ieee_profile(&[reference.clone(), reference.clone()], &[]),
            PackageCause::ConflictingDefinition,
        ),
        (
            lock.admit_ieee_profile(&[revision], &[]),
            PackageCause::RevisionMismatch,
        ),
        (
            lock.admit_ieee_profile(&[version], &[]),
            PackageCause::IncompatibleDefinition,
        ),
    ];
    for (outcome, cause) in cases {
        let refusal = outcome.unwrap_err();
        assert_eq!(refusal.code, PackageRefusalCode::InvalidPackage);
        assert_eq!(refusal.cause, cause);
    }
    let reserved: Vec<_> = ieee_intrinsic_identities().collect();
    assert_eq!(
        reserved,
        [
            "quire::value::ieee::sqrt",
            "quire::value::ieee::fma",
            "quire::value::ieee::numericEqual",
            "quire::value::ieee::totalOrder",
            "quire::value::ieee::bitIdentical",
        ]
    );
    for identity in reserved {
        let refusal = lock
            .admit_ieee_profile(std::slice::from_ref(&reference), &[identity])
            .unwrap_err();
        assert_eq!(
            (refusal.code, refusal.cause),
            (
                PackageRefusalCode::InvalidPackage,
                PackageCause::ConflictingDefinition
            ),
            "{identity}"
        );
    }
}

// ---- explicit conversions ----------------------------------------------------------------

#[trace("TC-193", "FR-148-AC-6", "TC-202", "FR-078-AC-3")]
#[test]
fn explicit_width_and_exact_conversions_report_loss_or_refuse() {
    let convert = |value, target, mode| {
        convert_ieee_width(profile(), value, target, mode, &mut Meter::new(UNLIMITED))
    };
    let even = RoundingMode::NearestEven;
    assert_eq!(
        bits_flags(convert(
            f64v(0x3ff0_0000_0000_0001),
            IeeeWidth::Binary32,
            even
        )),
        (0x3f80_0000, flags(&[IeeeFlag::Inexact]))
    );
    assert_eq!(
        convert(
            f64v(0x3ff0_0000_0000_0001),
            IeeeWidth::Binary32,
            RoundingMode::Exact
        ),
        Outcome::Refused(Refusal::IeeeNotExact {
            would_be: flags(&[IeeeFlag::Inexact])
        })
    );
    let overflow = flags(&[IeeeFlag::Overflow, IeeeFlag::Inexact]);
    assert_eq!(
        bits_flags(convert(
            f64v(0x7fef_ffff_ffff_ffff),
            IeeeWidth::Binary32,
            even
        )),
        (0x7f80_0000, overflow)
    );
    assert_eq!(
        bits_flags(convert(
            f64v(0x7fef_ffff_ffff_ffff),
            IeeeWidth::Binary32,
            RoundingMode::TowardZero
        )),
        (0x7f7f_ffff, overflow)
    );
    assert_eq!(
        bits_flags(convert(
            f64v(0x0000_0000_0000_0001),
            IeeeWidth::Binary32,
            even
        )),
        (
            0x0000_0000,
            flags(&[IeeeFlag::Underflow, IeeeFlag::Inexact])
        )
    );
    assert_eq!(
        bits_flags(convert(
            f32v(0x8000_0000),
            IeeeWidth::Binary64,
            RoundingMode::Exact
        )),
        (0x8000_0000_0000_0000, IeeeFlags::EMPTY)
    );
    assert_eq!(
        bits_flags(convert(f32v(0x7f80_0001), IeeeWidth::Binary64, even)),
        (0x7ff8_0000_0000_0001, flags(&[IeeeFlag::Invalid]))
    );
    assert_eq!(
        bits_flags(convert(
            f64v(0xfff8_0000_0000_0005),
            IeeeWidth::Binary32,
            even
        )),
        (0xffc0_0005, IeeeFlags::EMPTY)
    );
    assert_eq!(
        convert(f64v(0x7ff8_0100_0000_0000), IeeeWidth::Binary32, even),
        Outcome::Refused(Refusal::IeeeNanPayloadNotRepresentable)
    );

    let every = every_finite_ieee();
    let to_exact = |value| to_rational(value, &every, &mut Meter::new(UNLIMITED));
    let half = to_exact(f32v(0xbf00_0000)).completed().unwrap();
    assert_eq!(
        half.value(),
        &Rational::new(Integer::from(-1_i64), Integer::from(2_i64)).unwrap()
    );
    assert_eq!(half.loss(), None);
    let zero = to_exact(f64v(0x8000_0000_0000_0000)).completed().unwrap();
    assert_eq!(zero.value(), &Rational::from_integer(Integer::from(0_i64)));
    assert_eq!(zero.loss(), Some(IeeeExactLoss::NegativeZeroSign));
    for undefined in [
        f32v(0x7f80_0000),
        f32v(0x7fc0_0000),
        f64v(0xfff0_0000_0000_0001),
    ] {
        assert_eq!(
            to_exact(undefined),
            Outcome::Undefined(Undefined::IeeeNotFinite)
        );
    }

    let from_exact = |numerator: BigInt, denominator: BigInt, width, mode| {
        let value = Rational::new(
            numerator.to_string().parse().unwrap(),
            denominator.to_string().parse().unwrap(),
        )
        .unwrap();
        exact_to_ieee(profile(), &value, width, mode, &mut Meter::new(UNLIMITED))
    };
    let int = BigInt::from;
    assert_eq!(
        bits_flags(from_exact(int(1), int(3), IeeeWidth::Binary32, even)),
        (0x3eaa_aaab, flags(&[IeeeFlag::Inexact]))
    );
    assert_eq!(
        from_exact(int(1), int(3), IeeeWidth::Binary32, RoundingMode::Exact),
        Outcome::Refused(Refusal::IeeeNotExact {
            would_be: flags(&[IeeeFlag::Inexact])
        })
    );
    assert_eq!(
        bits_flags(from_exact(
            int(-1),
            int(2),
            IeeeWidth::Binary32,
            RoundingMode::Exact
        )),
        (0xbf00_0000, IeeeFlags::EMPTY)
    );
    assert_eq!(
        bits_flags(from_exact(
            int(0),
            int(1),
            IeeeWidth::Binary64,
            RoundingMode::Exact
        )),
        (0, IeeeFlags::EMPTY)
    );
    assert_eq!(
        bits_flags(from_exact(
            BigInt::one() << 2000_u32,
            int(1),
            IeeeWidth::Binary64,
            even
        )),
        (0x7ff0_0000_0000_0000, overflow)
    );
    assert_eq!(
        bits_flags(from_exact(
            int(1),
            BigInt::one() << 2000_u32,
            IeeeWidth::Binary64,
            RoundingMode::TowardPositive
        )),
        (
            0x0000_0000_0000_0001,
            flags(&[IeeeFlag::Underflow, IeeeFlag::Inexact])
        )
    );

    // Every finite class round-trips exactly, and each conversion's charges
    // are generated and denied one at a time.
    for width in IeeeWidth::ALL {
        let spec = Spec::of(width);
        for bits in spec.classes() {
            let value = spec.value(bits);
            let mut meter = Meter::new(UNLIMITED);
            let exact = match to_rational(value, &every, &mut meter) {
                Outcome::Completed(exact) => exact,
                Outcome::Undefined(Undefined::IeeeNotFinite) => {
                    assert!(matches!(
                        spec.decode(bits),
                        OClass::Nan { .. } | OClass::Inf(_)
                    ));
                    continue;
                }
                other => panic!("{bits:#x}: {other:?}"),
            };
            assert_eq!(meter.admitted_charges(), TO_EXACT_CHARGES);
            // The exact-intermediate is sized analytically as the result's
            // `maxparts`; consumption records the largest charge.
            assert_eq!(
                meter.consumed(LimitKind::IntegerBits),
                u64::from(width.bits()).max(exact.value().max_part_bits()),
                "{bits:#x}"
            );
            let mut meter = Meter::new(UNLIMITED);
            let back = done(exact_to_ieee(
                profile(),
                exact.value(),
                width,
                RoundingMode::Exact,
                &mut meter,
            ));
            let expected = if exact.loss() == Some(IeeeExactLoss::NegativeZeroSign) {
                0
            } else {
                bits
            };
            assert_eq!(
                (back.value().bits(), back.flags()),
                (expected, IeeeFlags::EMPTY)
            );
            assert_eq!(meter.admitted_charges(), FINITE_CHARGES);
            for (index, point) in FINITE_CHARGES.into_iter().enumerate() {
                assert_denied(
                    exact_to_ieee(profile(), exact.value(), width, even, &mut injected(point)),
                    point,
                    index as u64,
                );
                let widened = convert_ieee_width(
                    profile(),
                    value,
                    IeeeWidth::Binary64,
                    even,
                    &mut injected(point),
                );
                assert_denied(widened, point, index as u64);
            }
            for (index, point) in TO_EXACT_CHARGES.into_iter().enumerate() {
                assert_denied(
                    to_rational(value, &every, &mut injected(point)),
                    point,
                    index as u64,
                );
            }
        }
    }
}

// ---- generated class matrix -----------------------------------------------------------

/// A nonnegative-denominator exact rational.
type Q = (BigInt, BigInt);

fn q_int(value: i64) -> Q {
    (BigInt::from(value), BigInt::one())
}

fn q_cmp(left: &Q, right: &Q) -> Ordering {
    (&left.0 * &right.1).cmp(&(&right.0 * &left.1))
}

fn q_add(left: &Q, right: &Q) -> Q {
    (&left.0 * &right.1 + &right.0 * &left.1, &left.1 * &right.1)
}

fn q_mul(left: &Q, right: &Q) -> Q {
    (&left.0 * &right.0, &left.1 * &right.1)
}

fn q_div(left: &Q, right: &Q) -> Q {
    let (numerator, denominator) = (&left.0 * &right.1, &left.1 * &right.0);
    if denominator.is_negative() {
        (-numerator, -denominator)
    } else {
        (numerator, denominator)
    }
}

fn q_pow2(exponent: i64) -> Q {
    let shift = exponent.unsigned_abs();
    if exponent >= 0 {
        (BigInt::one() << shift, BigInt::one())
    } else {
        (BigInt::one(), BigInt::one() << shift)
    }
}

fn q_mid(left: &Q, right: &Q) -> Q {
    let sum = q_add(left, right);
    (sum.0, sum.1 * 2)
}

#[derive(Clone, Copy)]
struct Spec {
    width: IeeeWidth,
    exponent_bits: u32,
    fraction_bits: u32,
}

#[derive(Clone, Debug)]
enum OClass {
    Nan {
        signaling: bool,
    },
    Inf(bool),
    Zero(bool),
    /// Sign and magnitude.
    Fin(bool, Q),
}

impl Spec {
    fn of(width: IeeeWidth) -> Self {
        match width {
            IeeeWidth::Binary32 => Self {
                width,
                exponent_bits: 8,
                fraction_bits: 23,
            },
            IeeeWidth::Binary64 => Self {
                width,
                exponent_bits: 11,
                fraction_bits: 52,
            },
        }
    }

    fn value(self, bits: u64) -> IeeeValue {
        match self.width {
            IeeeWidth::Binary32 => IeeeValue::binary32(u32::try_from(bits).unwrap()),
            IeeeWidth::Binary64 => IeeeValue::binary64(bits),
        }
    }

    fn bias(self) -> i64 {
        (1 << (self.exponent_bits - 1)) - 1
    }

    fn emin(self) -> i64 {
        1 - self.bias()
    }

    fn sign(self) -> u64 {
        1 << (self.exponent_bits + self.fraction_bits)
    }

    fn exponent_mask(self) -> u64 {
        ((1 << self.exponent_bits) - 1) << self.fraction_bits
    }

    fn fraction_mask(self) -> u64 {
        (1 << self.fraction_bits) - 1
    }

    fn quiet(self) -> u64 {
        1 << (self.fraction_bits - 1)
    }

    fn infinity(self, negative: bool) -> u64 {
        self.exponent_mask() | if negative { self.sign() } else { 0 }
    }

    fn max_finite(self) -> u64 {
        self.exponent_mask() - (1 << self.fraction_bits) + self.fraction_mask()
    }

    fn signed(self, negative: bool, bits: u64) -> u64 {
        bits | if negative { self.sign() } else { 0 }
    }

    /// Magnitude of the nonnegative finite pattern `bits`.
    fn magnitude(self, bits: u64) -> Q {
        let field = (bits & self.exponent_mask()) >> self.fraction_bits;
        let fraction = BigInt::from(bits & self.fraction_mask());
        let fraction_bits = i64::from(self.fraction_bits);
        if field == 0 {
            q_mul(
                &(fraction, BigInt::one()),
                &q_pow2(self.emin() - fraction_bits),
            )
        } else {
            let significand = fraction + (BigInt::one() << self.fraction_bits);
            let exponent = i64::try_from(field).unwrap() - self.bias() - fraction_bits;
            q_mul(&(significand, BigInt::one()), &q_pow2(exponent))
        }
    }

    fn decode(self, bits: u64) -> OClass {
        let negative = bits & self.sign() != 0;
        let magnitude_bits = bits & !self.sign();
        if magnitude_bits & self.exponent_mask() == self.exponent_mask() {
            if magnitude_bits & self.fraction_mask() == 0 {
                OClass::Inf(negative)
            } else {
                OClass::Nan {
                    signaling: magnitude_bits & self.quiet() == 0,
                }
            }
        } else if magnitude_bits == 0 {
            OClass::Zero(negative)
        } else {
            OClass::Fin(negative, self.magnitude(magnitude_bits))
        }
    }

    /// Signs, zeros, subnormal and normal extrema, an odd significand, the
    /// infinities and signaling/quiet NaNs of both signs.
    fn classes(self) -> Vec<u64> {
        let one = (self.bias() as u64) << self.fraction_bits;
        let three = one + (1 << self.fraction_bits) + self.quiet();
        let exponent = self.exponent_mask();
        [
            0,
            1,
            self.fraction_mask(),
            1 << self.fraction_bits,
            self.max_finite(),
            one,
            three,
            exponent,
        ]
        .into_iter()
        .flat_map(|bits| [bits, bits | self.sign()])
        .chain([
            exponent | 1,
            self.sign() | exponent | 2,
            exponent | self.quiet() | 3,
            self.sign() | exponent | self.quiet(),
        ])
        .collect()
    }

    /// Round a nonzero real, given only through `cmp(q) = |x| ⋚ q`, by
    /// searching the ordered positive patterns.
    fn round(
        self,
        negative: bool,
        cmp: &dyn Fn(&Q) -> Ordering,
        mode: RoundingMode,
    ) -> (u64, IeeeFlags) {
        let max = self.max_finite();
        let beyond = q_pow2(self.bias() + 1);
        let value = |bits: u64| {
            if bits > max {
                beyond.clone()
            } else {
                self.magnitude(bits)
            }
        };
        let (mut low, mut high) = (0_u64, max + 1);
        if cmp(&beyond) != Ordering::Less {
            low = max;
        } else {
            while high - low > 1 {
                let middle = low + (high - low) / 2;
                if cmp(&value(middle)) == Ordering::Less {
                    high = middle;
                } else {
                    low = middle;
                }
            }
        }
        if low > 0 && cmp(&value(low)) == Ordering::Equal {
            return (self.signed(negative, low), IeeeFlags::EMPTY);
        }
        let direction = if mode == RoundingMode::Exact {
            RoundingMode::NearestEven
        } else {
            mode
        };
        let away = match direction {
            RoundingMode::NearestEven => match cmp(&q_mid(&value(low), &value(low + 1))) {
                Ordering::Greater => true,
                Ordering::Equal => low & 1 == 1,
                Ordering::Less => false,
            },
            RoundingMode::NearestAway => {
                cmp(&q_mid(&value(low), &value(low + 1))) != Ordering::Less
            }
            RoundingMode::TowardZero => false,
            RoundingMode::TowardPositive => !negative,
            RoundingMode::TowardNegative | RoundingMode::Exact => negative,
        };
        let mut raised = vec![IeeeFlag::Inexact];
        let chose_beyond = away && low == max;
        if chose_beyond || cmp(&beyond) != Ordering::Less {
            raised.push(IeeeFlag::Overflow);
            let bits = if chose_beyond {
                self.infinity(negative)
            } else {
                self.signed(negative, max)
            };
            return (bits, flags(&raised));
        }
        let least_normal = q_pow2(self.emin());
        let below = q_add(
            &least_normal,
            &(
                -q_pow2(self.emin() - i64::from(self.fraction_bits) - 1).0,
                q_pow2(self.emin() - i64::from(self.fraction_bits) - 1).1,
            ),
        );
        let reaches_normal = cmp(&below) == Ordering::Greater
            && match direction {
                RoundingMode::NearestEven | RoundingMode::NearestAway => {
                    cmp(&q_mid(&below, &least_normal)) != Ordering::Less
                }
                RoundingMode::TowardZero => false,
                RoundingMode::TowardPositive => !negative,
                RoundingMode::TowardNegative | RoundingMode::Exact => negative,
            };
        if cmp(&least_normal) == Ordering::Less && !reaches_normal {
            raised.push(IeeeFlag::Underflow);
        }
        let bits = if away { low + 1 } else { low };
        (self.signed(negative, bits), flags(&raised))
    }
}

#[derive(Clone, Copy, Debug)]
enum OOp {
    Add,
    Subtract,
    Multiply,
    Divide,
    SquareRoot,
    Fma,
}

#[derive(Debug, PartialEq)]
enum Expected {
    Done {
        bits: u64,
        flags: IeeeFlags,
        finite_path: bool,
    },
    NotExact(IeeeFlags),
}

enum Real {
    Zero(bool),
    Rational(Q),
    Root(Q),
}

fn signed_q(class: &OClass) -> Q {
    match class {
        OClass::Fin(true, magnitude) => (-magnitude.0.clone(), magnitude.1.clone()),
        OClass::Fin(false, magnitude) => magnitude.clone(),
        OClass::Zero(_) | OClass::Inf(_) | OClass::Nan { .. } => q_int(0),
    }
}

fn is_negative(class: &OClass) -> bool {
    matches!(
        class,
        OClass::Inf(true) | OClass::Zero(true) | OClass::Fin(true, _)
    )
}

fn zero_sum(left: bool, right: bool, mode: RoundingMode) -> bool {
    if left == right {
        left
    } else {
        mode == RoundingMode::TowardNegative
    }
}

fn oracle(spec: Spec, operation: OOp, operands: &[u64], mode: RoundingMode) -> Expected {
    let special = |bits, raised: &[IeeeFlag]| Expected::Done {
        bits,
        flags: flags(raised),
        finite_path: false,
    };
    let invalid = special(spec.exponent_mask() | spec.quiet(), &[IeeeFlag::Invalid]);
    let classes: Vec<OClass> = operands.iter().map(|bits| spec.decode(*bits)).collect();
    if let Some(index) = classes
        .iter()
        .position(|class| matches!(class, OClass::Nan { .. }))
    {
        let signaling = classes
            .iter()
            .any(|class| matches!(class, OClass::Nan { signaling: true }));
        let raised: &[IeeeFlag] = if signaling { &[IeeeFlag::Invalid] } else { &[] };
        return special(operands[index] | spec.quiet(), raised);
    }
    let negate = |class: &OClass| match class {
        OClass::Inf(negative) => OClass::Inf(!negative),
        OClass::Zero(negative) => OClass::Zero(!negative),
        OClass::Fin(negative, magnitude) => OClass::Fin(!negative, magnitude.clone()),
        OClass::Nan { signaling } => OClass::Nan {
            signaling: *signaling,
        },
    };
    let sum_real = |left: &OClass, right: &OClass| -> Real {
        if let (OClass::Zero(x), OClass::Zero(y)) = (left, right) {
            return Real::Zero(zero_sum(*x, *y, mode));
        }
        let total = q_add(&signed_q(left), &signed_q(right));
        if total.0.is_zero() {
            Real::Zero(mode == RoundingMode::TowardNegative)
        } else {
            Real::Rational(total)
        }
    };
    let real = match (operation, classes.as_slice()) {
        (OOp::Add | OOp::Subtract, [a, b]) => {
            let b = if matches!(operation, OOp::Subtract) {
                negate(b)
            } else {
                b.clone()
            };
            match (a, &b) {
                (OClass::Inf(x), OClass::Inf(y)) if x != y => return invalid,
                (OClass::Inf(x), _) | (_, OClass::Inf(x)) => {
                    return special(spec.infinity(*x), &[])
                }
                _ => sum_real(a, &b),
            }
        }
        (OOp::Multiply, [a, b]) => {
            let negative = is_negative(a) != is_negative(b);
            match (a, b) {
                (OClass::Inf(_), OClass::Zero(_)) | (OClass::Zero(_), OClass::Inf(_)) => {
                    return invalid
                }
                (OClass::Inf(_), _) | (_, OClass::Inf(_)) => {
                    return special(spec.infinity(negative), &[])
                }
                (OClass::Zero(_), _) | (_, OClass::Zero(_)) => Real::Zero(negative),
                _ => Real::Rational(q_mul(&signed_q(a), &signed_q(b))),
            }
        }
        (OOp::Divide, [a, b]) => {
            let negative = is_negative(a) != is_negative(b);
            match (a, b) {
                (OClass::Inf(_), OClass::Inf(_)) | (OClass::Zero(_), OClass::Zero(_)) => {
                    return invalid
                }
                (OClass::Inf(_), _) => return special(spec.infinity(negative), &[]),
                (_, OClass::Inf(_)) => {
                    return special(spec.signed(negative, 0), &[]);
                }
                (_, OClass::Zero(_)) => {
                    return special(spec.infinity(negative), &[IeeeFlag::DivideByZero])
                }
                (OClass::Zero(_), _) => Real::Zero(negative),
                _ => Real::Rational(q_div(&signed_q(a), &signed_q(b))),
            }
        }
        (OOp::SquareRoot, [a]) => match a {
            OClass::Inf(false) => return special(spec.infinity(false), &[]),
            OClass::Inf(true) | OClass::Fin(true, _) => return invalid,
            OClass::Zero(negative) => Real::Zero(*negative),
            OClass::Fin(false, magnitude) => Real::Root(magnitude.clone()),
            OClass::Nan { .. } => unreachable!(),
        },
        (OOp::Fma, [a, b, c]) => {
            let product_negative = is_negative(a) != is_negative(b);
            match (a, b, c) {
                (OClass::Inf(_), OClass::Zero(_), _) | (OClass::Zero(_), OClass::Inf(_), _) => {
                    return invalid
                }
                (OClass::Inf(_), _, OClass::Inf(z)) | (_, OClass::Inf(_), OClass::Inf(z))
                    if *z != product_negative =>
                {
                    return invalid
                }
                (OClass::Inf(_), _, _) | (_, OClass::Inf(_), _) => {
                    return special(spec.infinity(product_negative), &[])
                }
                (_, _, OClass::Inf(z)) => return special(spec.infinity(*z), &[]),
                _ => {
                    let product = if matches!(a, OClass::Zero(_)) || matches!(b, OClass::Zero(_)) {
                        OClass::Zero(product_negative)
                    } else {
                        let value = q_mul(&signed_q(a), &signed_q(b));
                        let magnitude = (value.0.abs(), value.1);
                        OClass::Fin(product_negative, magnitude)
                    };
                    sum_real(&product, c)
                }
            }
        }
        _ => unreachable!("operand count matches the operation"),
    };
    let (bits, raised) = match real {
        Real::Zero(negative) => (spec.signed(negative, 0), IeeeFlags::EMPTY),
        Real::Rational(value) => {
            let negative = value.0.is_negative();
            let magnitude = (value.0.abs(), value.1.abs());
            spec.round(negative, &|q: &Q| q_cmp(&magnitude, q), mode)
        }
        Real::Root(radicand) => spec.round(false, &|q: &Q| q_cmp(&radicand, &q_mul(q, q)), mode),
    };
    if mode == RoundingMode::Exact && !raised.is_empty() {
        Expected::NotExact(raised)
    } else {
        Expected::Done {
            bits,
            flags: raised,
            finite_path: true,
        }
    }
}

fn to_operation(spec: Spec, operation: OOp, operands: &[u64]) -> IeeeOperation {
    let value = |index: usize| spec.value(operands[index]);
    match operation {
        OOp::Add => IeeeOperation::Add(value(0), value(1)),
        OOp::Subtract => IeeeOperation::Subtract(value(0), value(1)),
        OOp::Multiply => IeeeOperation::Multiply(value(0), value(1)),
        OOp::Divide => IeeeOperation::Divide(value(0), value(1)),
        OOp::SquareRoot => IeeeOperation::SquareRoot(value(0)),
        OOp::Fma => IeeeOperation::FusedMultiplyAdd(value(0), value(1), value(2)),
    }
}

fn check_vector(spec: Spec, operation: OOp, operands: &[u64], mode: RoundingMode, deny: bool) {
    let expected = oracle(spec, operation, operands, mode);
    let request = to_operation(spec, operation, operands);
    let mut meter = Meter::new(UNLIMITED);
    let outcome = eval_with(request, mode, &mut meter);
    let context = format!("{:?} {operation:?} {operands:#x?} {mode:?}", spec.width);
    let charges: &[ChargePoint] = match (&expected, outcome) {
        (
            Expected::Done {
                bits,
                flags,
                finite_path,
            },
            Outcome::Completed(result),
        ) => {
            assert_eq!(
                (result.value().bits(), result.flags()),
                (*bits, *flags),
                "{context}"
            );
            assert_eq!(result.value().width(), spec.width, "{context}");
            assert_eq!(result.provenance().rounding(), mode, "{context}");
            if *finite_path {
                &FINITE_CHARGES
            } else {
                &CLASSIFIED_CHARGES
            }
        }
        (
            Expected::NotExact(would_be),
            Outcome::Refused(Refusal::IeeeNotExact { would_be: got }),
        ) => {
            assert_eq!(*would_be, got, "{context}");
            &FINITE_CHARGES[..3]
        }
        (expected, outcome) => panic!("{context}: expected {expected:?}, got {outcome:?}"),
    };
    assert_eq!(meter.admitted_charges(), charges, "{context}");
    if deny {
        for (index, point) in charges.iter().enumerate() {
            let mut denied = injected(*point);
            assert_denied(eval_with(request, mode, &mut denied), *point, index as u64);
            assert_eq!(denied.consumed(LimitKind::ResultUnits), 0, "{context}");
        }
    }
}

#[trace(
    "TC-193",
    "FR-148-AC-1",
    "FR-148-AC-2",
    "FR-148-AC-4",
    "FR-148-AC-6",
    "FR-148-AC-7",
    "TC-202",
    "FR-078-AC-3"
)]
#[test]
fn generated_class_matrix_matches_the_exact_real_oracle_with_every_denial() {
    for width in IeeeWidth::ALL {
        let spec = Spec::of(width);
        let classes = spec.classes();
        for mode in RoundingMode::ALL {
            for a in &classes {
                check_vector(spec, OOp::SquareRoot, &[*a], mode, true);
                for b in &classes {
                    for operation in [OOp::Add, OOp::Subtract, OOp::Multiply, OOp::Divide] {
                        check_vector(spec, operation, &[*a, *b], mode, true);
                    }
                    for c in &classes {
                        check_vector(
                            spec,
                            OOp::Fma,
                            &[*a, *b, *c],
                            mode,
                            mode == RoundingMode::NearestEven,
                        );
                    }
                }
            }
        }
    }
}

#[trace("TC-193", "FR-148-AC-6", "TC-202", "FR-078-AC-3")]
#[test]
fn generated_signed_zero_and_directed_overflow_rules_hold_for_every_direction() {
    for width in IeeeWidth::ALL {
        let spec = Spec::of(width);
        let one = (spec.bias() as u64) << spec.fraction_bits;
        for mode in RoundingMode::ALL {
            // x + (-x) is +0 except toward-negative; -0 + -0 is -0.
            let cancel = done(eval(
                IeeeOperation::Add(spec.value(one), spec.value(one | spec.sign())),
                mode,
            ));
            let negative_zero = mode == RoundingMode::TowardNegative;
            assert_eq!(
                cancel.value().bits(),
                spec.signed(negative_zero, 0),
                "{mode:?}"
            );
            let zeros = done(eval(
                IeeeOperation::Add(spec.value(spec.sign()), spec.value(spec.sign())),
                mode,
            ));
            assert_eq!(zeros.value().bits(), spec.sign());
            let root = done(eval(
                IeeeOperation::SquareRoot(spec.value(spec.sign())),
                mode,
            ));
            assert_eq!(root.value().bits(), spec.sign());
        }
        for (mode, positive, negative) in [
            (
                RoundingMode::NearestEven,
                spec.infinity(false),
                spec.infinity(true),
            ),
            (
                RoundingMode::NearestAway,
                spec.infinity(false),
                spec.infinity(true),
            ),
            (
                RoundingMode::TowardZero,
                spec.max_finite(),
                spec.signed(true, spec.max_finite()),
            ),
            (
                RoundingMode::TowardPositive,
                spec.infinity(false),
                spec.signed(true, spec.max_finite()),
            ),
            (
                RoundingMode::TowardNegative,
                spec.max_finite(),
                spec.infinity(true),
            ),
        ] {
            let max = spec.max_finite();
            for (sign, expected) in [(0, positive), (spec.sign(), negative)] {
                assert_eq!(
                    bits_flags(eval(
                        IeeeOperation::Add(spec.value(max | sign), spec.value(max | sign)),
                        mode
                    )),
                    (expected, flags(&[IeeeFlag::Overflow, IeeeFlag::Inexact])),
                    "{width:?} {mode:?}"
                );
            }
        }
    }
}

// ---- charge-position, signed-zero and conversion vectors ---------------------------------

/// `I(i,o,w,r)` from TC-193 F20-F31.
fn limits(
    integer_bits: u64,
    value_occurrences: u64,
    work_units: u64,
    result_units: u64,
) -> ScalarLimits {
    ScalarLimits {
        integer_bits,
        value_occurrences,
        work_units,
        result_units,
        ..F17
    }
}

fn incomplete(
    limit_kind: LimitKind,
    limit: u64,
    consumed: u64,
    next_charge: u64,
    charge_point: ChargePoint,
) -> Incomplete {
    Incomplete {
        limit_kind,
        limit,
        consumed,
        next_charge: next_charge.into(),
        charge_point,
    }
}

#[trace("TC-193", "FR-148-AC-7", "TC-202", "FR-078-AC-3")]
#[test]
fn a_largest_scale_decimal_source_is_sized_without_materializing_its_power() {
    // bits(10^4294967295) = floor(4294967295 × log2(10)) + 1.
    let source = Decimal::new(Integer::from(1_i64), u32::MAX);
    let mut meter = Meter::new(limits(64, 1, 4, 1));
    assert_eq!(
        exact_to_ieee(
            profile(),
            &source,
            IeeeWidth::Binary64,
            RoundingMode::NearestEven,
            &mut meter
        ),
        Outcome::Incomplete(incomplete(
            LimitKind::IntegerBits,
            64,
            0,
            14_267_572_524,
            ChargePoint::IeeeOperands
        ))
    );
    assert!(meter.admitted_charges().is_empty());
}

#[trace("TC-193", "FR-148-AC-8", "TC-202", "FR-078-AC-3")]
#[test]
fn f20_cross_width_comparisons_refuse_ill_typed_before_any_charge() {
    for comparison in IeeeComparison::ALL {
        let mut meter = Meter::new(limits(0, 0, 0, 0));
        assert_eq!(
            compare_ieee(
                profile(),
                comparison,
                f32v(0x3f80_0000),
                f64v(0x3ff0_0000_0000_0000),
                &mut meter
            ),
            Err(IllTyped {
                cause: IllTypedCause::DistinctIeeeWidths
            })
        );
        assert!(meter.admitted_charges().is_empty());
        assert!(LimitKind::ALL.iter().all(|kind| meter.consumed(*kind) == 0));
    }

    let one = Integer::from(1_i64);
    let wide = f64v(0x3ff0_0000_0000_0000);
    let decimal = DecimalType::new(
        Integer::from(0_i64),
        Integer::from(10_i64),
        0,
        0,
        RoundingMode::NearestEven,
    )
    .unwrap();
    let bit = IntegerInterval::new(Integer::from(0_i64), Integer::from(1_i64)).unwrap();
    for (attempt, cause) in [
        (
            evaluate_ieee(
                profile(),
                IeeeOperation::Add(f32v(0x3f80_0000), wide),
                RoundingMode::NearestEven,
                &mut Meter::new(limits(0, 0, 0, 0)),
            )
            .map(|_| ()),
            IllTypedCause::DistinctIeeeWidths,
        ),
        (
            evaluate_ieee(
                profile(),
                IeeeOperation::Add(
                    IeeeOperand::Ieee(f32v(0x3f80_0000)),
                    IeeeOperand::Exact(ExactScalar::from(&one)),
                ),
                RoundingMode::NearestEven,
                &mut Meter::new(limits(0, 0, 0, 0)),
            )
            .map(|_| ()),
            IllTypedCause::IeeeWithExactOperand,
        ),
        (
            compare_ieee(
                profile(),
                IeeeComparison::NumericEqual,
                f32v(0x3f80_0000),
                ExactScalar::from(&one),
                &mut Meter::new(limits(0, 0, 0, 0)),
            )
            .map(|_| ()),
            IllTypedCause::IeeeWithExactOperand,
        ),
        (
            ieee_to_exact(
                profile(),
                f32v(0x3f80_0000),
                IeeeExactTarget::Decimal(&decimal),
                &mut Meter::new(limits(0, 0, 0, 0)),
            )
            .map(|_| ()),
            IllTypedCause::IeeeToNonRationalExact,
        ),
        (
            ieee_to_exact(
                profile(),
                f32v(0x3f80_0000),
                IeeeExactTarget::BoundedInteger(&bit),
                &mut Meter::new(limits(0, 0, 0, 0)),
            )
            .map(|_| ()),
            IllTypedCause::IeeeToNonRationalExact,
        ),
        (
            ieee_to_exact(
                profile(),
                f32v(0x3f80_0000),
                IeeeExactTarget::Integer,
                &mut Meter::new(limits(0, 0, 0, 0)),
            )
            .map(|_| ()),
            IllTypedCause::IeeeToNonRationalExact,
        ),
    ] {
        assert_eq!(attempt, Err(IllTyped { cause }));
    }
}

#[trace("TC-193", "FR-148-AC-8", "TC-202", "FR-078-AC-3")]
#[test]
fn f21_zero_operands_take_all_four_finite_charges() {
    let even = RoundingMode::NearestEven;
    for (operation, expected) in [
        (
            IeeeOperation::Add(f32v(0x0000_0000), f32v(0x0000_0000)),
            0x0000_0000,
        ),
        (
            IeeeOperation::Multiply(f32v(0x8000_0000), f32v(0x3f80_0000)),
            0x8000_0000,
        ),
    ] {
        let mut meter = Meter::new(limits(32, 2, 4, 1));
        assert_eq!(
            bits_flags(eval_with(operation, even, &mut meter)),
            (expected, IeeeFlags::EMPTY)
        );
        assert_eq!(meter.admitted_charges(), FINITE_CHARGES);
        assert_eq!(
            eval_with(operation, even, &mut Meter::new(limits(32, 2, 3, 1))),
            Outcome::Incomplete(incomplete(
                LimitKind::WorkUnits,
                3,
                3,
                1,
                ChargePoint::IeeeResultRetain
            ))
        );
    }
}

#[trace("TC-193", "FR-148-AC-8", "TC-202", "FR-078-AC-3")]
#[test]
fn f22_strict_exact_refuses_after_round_and_before_retention() {
    let f09 = IeeeOperation::Add(f32v(0x3f80_0000), f32v(0x3380_0000));
    let mut meter = Meter::new(limits(32, 2, 3, 1));
    assert_eq!(
        eval_with(f09, RoundingMode::Exact, &mut meter),
        Outcome::Refused(Refusal::IeeeNotExact {
            would_be: flags(&[IeeeFlag::Inexact])
        })
    );
    assert_eq!(meter.admitted_charges(), &FINITE_CHARGES[..3]);
    assert_eq!(
        eval_with(
            f09,
            RoundingMode::NearestEven,
            &mut Meter::new(limits(32, 2, 3, 1))
        ),
        Outcome::Incomplete(incomplete(
            LimitKind::WorkUnits,
            3,
            3,
            1,
            ChargePoint::IeeeResultRetain
        ))
    );
    assert_eq!(
        eval_with(
            f09,
            RoundingMode::Exact,
            &mut Meter::new(limits(32, 2, 2, 1))
        ),
        Outcome::Incomplete(incomplete(
            LimitKind::WorkUnits,
            2,
            2,
            1,
            ChargePoint::IeeeRound
        ))
    );
}

#[trace("TC-193", "FR-148-AC-8", "TC-202", "FR-078-AC-3")]
#[test]
fn f23_strict_exact_would_be_flags_follow_nearest_even() {
    let f23 = IeeeOperation::Add(f32v(0x7f7f_ffff), f32v(0x7300_0000));
    assert_eq!(
        bits_flags(eval(f23, RoundingMode::NearestEven)),
        (0x7f80_0000, flags(&[IeeeFlag::Overflow, IeeeFlag::Inexact]))
    );
    assert_eq!(
        bits_flags(eval(f23, RoundingMode::TowardZero)),
        (0x7f7f_ffff, flags(&[IeeeFlag::Inexact]))
    );
    assert_eq!(
        eval(f23, RoundingMode::Exact),
        Outcome::Refused(Refusal::IeeeNotExact {
            would_be: flags(&[IeeeFlag::Overflow, IeeeFlag::Inexact])
        })
    );
}

#[trace("TC-193", "FR-148-AC-9", "TC-202", "FR-078-AC-3")]
#[test]
fn f24_conversions_charge_at_their_stated_widths_and_positions() {
    let even = RoundingMode::NearestEven;
    let mut meter = Meter::new(limits(64, 1, 4, 1));
    assert_eq!(
        bits_flags(convert_ieee_width(
            profile(),
            f32v(0x3f80_0000),
            IeeeWidth::Binary64,
            even,
            &mut meter
        )),
        (0x3ff0_0000_0000_0000, IeeeFlags::EMPTY)
    );
    assert_eq!(meter.admitted_charges(), FINITE_CHARGES);
    assert_eq!(
        convert_ieee_width(
            profile(),
            f32v(0x3f80_0000),
            IeeeWidth::Binary64,
            even,
            &mut Meter::new(limits(32, 1, 4, 1))
        ),
        Outcome::Incomplete(incomplete(
            LimitKind::IntegerBits,
            32,
            32,
            64,
            ChargePoint::IeeeExactIntermediate
        ))
    );
    assert_eq!(
        convert_ieee_width(
            profile(),
            f64v(0x3ff0_0000_0000_0000),
            IeeeWidth::Binary32,
            even,
            &mut Meter::new(limits(32, 1, 4, 1))
        ),
        Outcome::Incomplete(incomplete(
            LimitKind::IntegerBits,
            32,
            0,
            64,
            ChargePoint::IeeeOperands
        ))
    );

    let third = Rational::new(Integer::from(1_i64), Integer::from(3_i64)).unwrap();
    let mut meter = Meter::new(limits(32, 1, 4, 1));
    assert_eq!(
        bits_flags(exact_to_ieee(
            profile(),
            &third,
            IeeeWidth::Binary32,
            even,
            &mut meter
        )),
        (0x3eaa_aaab, flags(&[IeeeFlag::Inexact]))
    );
    assert_eq!(meter.admitted_charges(), FINITE_CHARGES);
    assert_eq!(
        exact_to_ieee(
            profile(),
            &third,
            IeeeWidth::Binary32,
            even,
            &mut Meter::new(limits(2, 1, 4, 1))
        ),
        Outcome::Incomplete(incomplete(
            LimitKind::IntegerBits,
            2,
            2,
            32,
            ChargePoint::IeeeExactIntermediate
        ))
    );

    let int = BigInt::from;
    let unit_halves = rational_type(&int(0), &int(1), &int(1), &int(2));
    let mut meter = Meter::new(limits(32, 1, 3, 1));
    let half = to_rational(f32v(0x3f00_0000), &unit_halves, &mut meter)
        .completed()
        .unwrap();
    assert_eq!(half.value(), &ratio(1, 2));
    assert_eq!(meter.admitted_charges(), TO_EXACT_CHARGES);
    assert_eq!(half.value().max_part_bits(), 2);
    let mut meter = Meter::new(limits(32, 1, 2, 1));
    assert_eq!(
        to_rational(f32v(0x7fc0_0000), &unit_halves, &mut meter),
        Outcome::Undefined(Undefined::IeeeNotFinite)
    );
    assert_eq!(meter.admitted_charges(), [ChargePoint::IeeeOperands]);
}

#[trace("TC-193", "FR-148-AC-9", "TC-202", "FR-078-AC-3")]
#[test]
fn f25_nan_width_conversion_keeps_sign_and_payload_or_refuses() {
    let even = RoundingMode::NearestEven;
    for (source, target, limit_bits, expected, raised) in [
        (
            f64v(0x7ff0_0000_0000_0001),
            IeeeWidth::Binary32,
            64,
            0x7fc0_0001,
            flags(&[IeeeFlag::Invalid]),
        ),
        (
            f64v(0xfff8_0000_0000_0003),
            IeeeWidth::Binary32,
            64,
            0xffc0_0003,
            IeeeFlags::EMPTY,
        ),
        (
            f32v(0x7fc0_0001),
            IeeeWidth::Binary64,
            32,
            0x7ff8_0000_0000_0001,
            IeeeFlags::EMPTY,
        ),
    ] {
        let mut meter = Meter::new(limits(limit_bits, 1, 2, 1));
        let result = done(convert_ieee_width(
            profile(),
            source,
            target,
            even,
            &mut meter,
        ));
        assert_eq!((result.value().bits(), result.flags()), (expected, raised));
        assert_eq!(result.value().width(), target);
        assert_eq!(meter.admitted_charges(), CLASSIFIED_CHARGES);
    }

    let wide_payload = f64v(0x7ff8_0000_0040_0000);
    let mut meter = Meter::new(limits(64, 1, 1, 0));
    assert_eq!(
        convert_ieee_width(
            profile(),
            wide_payload,
            IeeeWidth::Binary32,
            even,
            &mut meter
        ),
        Outcome::Refused(Refusal::IeeeNanPayloadNotRepresentable)
    );
    assert_eq!(meter.admitted_charges(), [ChargePoint::IeeeOperands]);
    assert_eq!(
        convert_ieee_width(
            profile(),
            wide_payload,
            IeeeWidth::Binary32,
            even,
            &mut Meter::new(limits(64, 1, 0, 0))
        ),
        Outcome::Incomplete(incomplete(
            LimitKind::WorkUnits,
            0,
            0,
            1,
            ChargePoint::IeeeOperands
        ))
    );
    assert_eq!(
        Refusal::IeeeNanPayloadNotRepresentable.code(),
        Some("ieee_nan_payload_not_representable")
    );

    // A signaling source is refused before the NaN is consumed, so no
    // `invalid` flag exists to report.
    let mut meter = Meter::new(limits(64, 1, 1, 0));
    assert_eq!(
        convert_ieee_width(
            profile(),
            f64v(0x7ff0_0000_0040_0000),
            IeeeWidth::Binary32,
            even,
            &mut meter
        ),
        Outcome::Refused(Refusal::IeeeNanPayloadNotRepresentable)
    );
    assert_eq!(meter.admitted_charges(), [ChargePoint::IeeeOperands]);
}

#[trace("TC-193", "FR-148-AC-8", "FR-148-AC-9", "TC-202", "FR-078-AC-3")]
#[test]
fn f26_zero_signs_survive_width_conversion_sums_and_differences() {
    let even = RoundingMode::NearestEven;
    let mut meter = Meter::new(limits(64, 1, 4, 1));
    assert_eq!(
        bits_flags(convert_ieee_width(
            profile(),
            f32v(0x8000_0000),
            IeeeWidth::Binary64,
            even,
            &mut meter
        )),
        (0x8000_0000_0000_0000, IeeeFlags::EMPTY)
    );
    assert_eq!(meter.admitted_charges(), FINITE_CHARGES);
    assert_eq!(
        convert_ieee_width(
            profile(),
            f32v(0x8000_0000),
            IeeeWidth::Binary64,
            even,
            &mut Meter::new(limits(64, 1, 3, 1))
        ),
        Outcome::Incomplete(incomplete(
            LimitKind::WorkUnits,
            3,
            3,
            1,
            ChargePoint::IeeeResultRetain
        ))
    );

    let (negative_zero, positive_zero) = (f32v(0x8000_0000), f32v(0x0000_0000));
    let (one, minus_one) = (f32v(0x3f80_0000), f32v(0xbf80_0000));
    for mode in DIRECTIONS.into_iter().chain([RoundingMode::Exact]) {
        let cancelled = if mode == RoundingMode::TowardNegative {
            0x8000_0000
        } else {
            0x0000_0000
        };
        for (operation, expected) in [
            (
                IeeeOperation::Add(negative_zero, negative_zero),
                0x8000_0000,
            ),
            (IeeeOperation::Add(one, minus_one), cancelled),
            (
                IeeeOperation::Subtract(negative_zero, positive_zero),
                0x8000_0000,
            ),
            (
                IeeeOperation::Subtract(positive_zero, positive_zero),
                cancelled,
            ),
            (IeeeOperation::Subtract(one, one), cancelled),
        ] {
            assert_eq!(
                bits_flags(eval(operation, mode)),
                (expected, IeeeFlags::EMPTY),
                "{operation:?} {mode:?}"
            );
        }
    }
}

#[trace("TC-193", "FR-148-AC-8", "TC-202", "FR-078-AC-3")]
#[test]
fn fused_multiply_add_exact_zero_takes_the_sum_sign_rule() {
    // FR-148: `fma(x, y, z)` applies the sum rule to `x × y`, signed by the
    // exclusive or of the signs of `x` and `y`, and `z`.
    let (negative_zero, positive_zero) = (f32v(0x8000_0000), f32v(0x0000_0000));
    let (one, minus_one) = (f32v(0x3f80_0000), f32v(0xbf80_0000));
    for mode in DIRECTIONS.into_iter().chain([RoundingMode::Exact]) {
        let opposite = if mode == RoundingMode::TowardNegative {
            0x8000_0000
        } else {
            0x0000_0000
        };
        for (operands, expected) in [
            ((negative_zero, one, negative_zero), 0x8000_0000),
            ((negative_zero, minus_one, positive_zero), 0x0000_0000),
            ((positive_zero, minus_one, negative_zero), 0x8000_0000),
            ((negative_zero, one, positive_zero), opposite),
            ((positive_zero, one, negative_zero), opposite),
            ((one, one, minus_one), opposite),
            ((minus_one, one, one), opposite),
        ] {
            let (x, y, z) = operands;
            assert_eq!(
                bits_flags(eval(IeeeOperation::FusedMultiplyAdd(x, y, z), mode)),
                (expected, IeeeFlags::EMPTY),
                "{operands:?} {mode:?}"
            );
        }
    }
}

#[trace("TC-193", "FR-148-AC-9", "TC-202", "FR-078-AC-3")]
#[test]
fn f27_ieee_to_rational_sizes_maxparts_and_admits_membership_before_retention() {
    let int = BigInt::from;
    let smallest = BigInt::one() << 1074_u32;
    let tiny_type = rational_type(&int(0), &int(1), &int(1), &smallest);
    let mut meter = Meter::new(limits(1075, 1, 3, 1));
    let tiny = to_rational(f64v(0x0000_0000_0000_0001), &tiny_type, &mut meter)
        .completed()
        .unwrap();
    assert_eq!(
        tiny.value(),
        &Rational::new(Integer::from(1_i64), big(&smallest)).unwrap()
    );
    assert_eq!(meter.admitted_charges(), TO_EXACT_CHARGES);
    assert_eq!(
        to_rational(
            f64v(0x0000_0000_0000_0001),
            &tiny_type,
            &mut Meter::new(limits(1074, 1, 3, 1))
        ),
        Outcome::Incomplete(incomplete(
            LimitKind::IntegerBits,
            1074,
            64,
            1075,
            ChargePoint::IeeeExactIntermediate
        ))
    );

    let zero_type = rational_type(&int(0), &int(0), &int(1), &int(1));
    let mut meter = Meter::new(limits(32, 1, 3, 1));
    let zero = to_rational(f32v(0x8000_0000), &zero_type, &mut meter)
        .completed()
        .unwrap();
    assert_eq!(zero.value(), &ratio(0, 1));
    assert_eq!(zero.loss(), Some(IeeeExactLoss::NegativeZeroSign));
    assert_eq!(meter.admitted_charges(), TO_EXACT_CHARGES);
    assert_eq!(
        to_rational(
            f32v(0x8000_0000),
            &zero_type,
            &mut Meter::new(limits(32, 1, 2, 1))
        ),
        Outcome::Incomplete(incomplete(
            LimitKind::WorkUnits,
            2,
            2,
            1,
            ChargePoint::IeeeResultRetain
        ))
    );

    let unit_halves = rational_type(&int(0), &int(1), &int(1), &int(2));
    let mut meter = Meter::new(limits(32, 1, 1, 0));
    assert_eq!(
        to_rational(f32v(0x7f80_0000), &unit_halves, &mut meter),
        Outcome::Undefined(Undefined::IeeeNotFinite)
    );
    assert_eq!(meter.admitted_charges(), [ChargePoint::IeeeOperands]);
    assert_eq!(
        to_rational(
            f32v(0x7f80_0000),
            &unit_halves,
            &mut Meter::new(limits(32, 1, 0, 0))
        ),
        Outcome::Incomplete(incomplete(
            LimitKind::WorkUnits,
            0,
            0,
            1,
            ChargePoint::IeeeOperands
        ))
    );

    let integers = rational_type(&int(0), &int(1), &int(1), &int(1));
    let mut meter = Meter::new(limits(32, 1, 2, 0));
    assert_eq!(
        to_rational(f32v(0x3f00_0000), &integers, &mut meter),
        Outcome::Refused(Refusal::IeeeRationalOutOfDomain)
    );
    assert_eq!(meter.admitted_charges(), &TO_EXACT_CHARGES[..2]);
    assert_eq!(
        Refusal::IeeeRationalOutOfDomain.code(),
        Some("ieee_rational_out_of_domain")
    );
    assert_eq!(
        to_rational(
            f32v(0x3f00_0000),
            &integers,
            &mut Meter::new(limits(32, 1, 1, 0))
        ),
        Outcome::Incomplete(incomplete(
            LimitKind::WorkUnits,
            1,
            1,
            1,
            ChargePoint::IeeeExactIntermediate
        ))
    );
}

#[trace("TC-193", "FR-148-AC-9", "TC-202", "FR-078-AC-3")]
#[test]
fn f28_decimal_source_is_sized_by_its_retained_representation() {
    let decimal = Decimal::new(Integer::from(100_i64), 2);
    let even = RoundingMode::NearestEven;
    let mut meter = Meter::new(limits(32, 1, 4, 1));
    assert_eq!(
        bits_flags(exact_to_ieee(
            profile(),
            &decimal,
            IeeeWidth::Binary32,
            even,
            &mut meter
        )),
        (0x3f80_0000, IeeeFlags::EMPTY)
    );
    assert_eq!(meter.admitted_charges(), FINITE_CHARGES);
    assert_eq!(
        exact_to_ieee(
            profile(),
            &decimal,
            IeeeWidth::Binary32,
            even,
            &mut Meter::new(limits(6, 1, 4, 1))
        ),
        Outcome::Incomplete(incomplete(
            LimitKind::IntegerBits,
            6,
            0,
            7,
            ChargePoint::IeeeOperands
        ))
    );
}

#[trace("TC-193", "FR-148-AC-8", "TC-202", "FR-078-AC-3")]
#[test]
fn f29_strict_exact_near_extremes_reports_only_inexact() {
    let inexact = flags(&[IeeeFlag::Inexact]);
    for (operation, expected) in [
        (
            IeeeOperation::Add(f32v(0x7f7f_ffff), f32v(0x7280_0000)),
            0x7f7f_ffff,
        ),
        (
            IeeeOperation::Multiply(f32v(0x007f_ffff), f32v(0x3f80_0001)),
            0x0080_0000,
        ),
    ] {
        assert_eq!(
            bits_flags(eval(operation, RoundingMode::NearestEven)),
            (expected, inexact),
            "{operation:?}"
        );
        assert_eq!(
            eval(operation, RoundingMode::Exact),
            Outcome::Refused(Refusal::IeeeNotExact { would_be: inexact }),
            "{operation:?}"
        );
    }
}

#[trace("TC-193", "FR-148-AC-8", "TC-202", "FR-078-AC-3")]
#[test]
fn f30_square_root_of_negative_zero_is_negative_zero() {
    let root = IeeeOperation::SquareRoot(f32v(0x8000_0000));
    let even = RoundingMode::NearestEven;
    let mut meter = Meter::new(limits(32, 1, 4, 1));
    assert_eq!(
        bits_flags(eval_with(root, even, &mut meter)),
        (0x8000_0000, IeeeFlags::EMPTY)
    );
    assert_eq!(meter.admitted_charges(), FINITE_CHARGES);
    assert_eq!(
        eval_with(root, even, &mut Meter::new(limits(32, 1, 3, 1))),
        Outcome::Incomplete(incomplete(
            LimitKind::WorkUnits,
            3,
            3,
            1,
            ChargePoint::IeeeResultRetain
        ))
    );
}

#[trace("TC-193", "FR-148-AC-9", "TC-202", "FR-078-AC-3")]
#[test]
fn f31_narrowing_conversion_rounds_overflows_and_underflows_once() {
    let narrow = |bits, mode, meter: &mut Meter| {
        convert_ieee_width(profile(), f64v(bits), IeeeWidth::Binary32, mode, meter)
    };
    let even = RoundingMode::NearestEven;
    let retain_denied = |work| {
        Outcome::Incomplete(incomplete(
            LimitKind::WorkUnits,
            work,
            work,
            1,
            ChargePoint::IeeeResultRetain,
        ))
    };

    let mut meter = Meter::new(limits(64, 1, 2, 1));
    assert_eq!(
        bits_flags(narrow(0xfff0_0000_0000_0000, even, &mut meter)),
        (0xff80_0000, IeeeFlags::EMPTY)
    );
    assert_eq!(meter.admitted_charges(), CLASSIFIED_CHARGES);
    assert_eq!(
        narrow(
            0xfff0_0000_0000_0000,
            even,
            &mut Meter::new(limits(64, 1, 1, 1))
        ),
        retain_denied(1)
    );

    let inexact = flags(&[IeeeFlag::Inexact]);
    let tenth = 0x3fb9_9999_9999_999a;
    let mut meter = Meter::new(limits(64, 1, 4, 1));
    assert_eq!(
        bits_flags(narrow(tenth, even, &mut meter)),
        (0x3dcc_cccd, inexact)
    );
    assert_eq!(meter.admitted_charges(), FINITE_CHARGES);
    let mut meter = Meter::new(limits(64, 1, 3, 1));
    assert_eq!(
        narrow(tenth, RoundingMode::Exact, &mut meter),
        Outcome::Refused(Refusal::IeeeNotExact { would_be: inexact })
    );
    assert_eq!(meter.admitted_charges(), &FINITE_CHARGES[..3]);
    assert_eq!(
        narrow(
            tenth,
            RoundingMode::Exact,
            &mut Meter::new(limits(64, 1, 2, 1))
        ),
        Outcome::Incomplete(incomplete(
            LimitKind::WorkUnits,
            2,
            2,
            1,
            ChargePoint::IeeeRound
        ))
    );

    assert_eq!(
        bits_flags(narrow(
            0x47ef_ffff_e000_0000,
            even,
            &mut Meter::new(limits(64, 1, 4, 1))
        )),
        (0x7f7f_ffff, IeeeFlags::EMPTY)
    );
    let halfway = 0x47ef_ffff_f000_0000;
    assert_eq!(
        bits_flags(narrow(halfway, even, &mut Meter::new(limits(64, 1, 4, 1)))),
        (0x7f80_0000, flags(&[IeeeFlag::Overflow, IeeeFlag::Inexact]))
    );
    assert_eq!(
        narrow(halfway, even, &mut Meter::new(limits(64, 1, 3, 1))),
        retain_denied(3)
    );
    assert_eq!(
        bits_flags(narrow(
            halfway,
            RoundingMode::TowardZero,
            &mut Meter::new(limits(64, 1, 4, 1))
        )),
        (0x7f7f_ffff, inexact)
    );

    let smallest = 0x0000_0000_0000_0001;
    assert_eq!(
        bits_flags(narrow(smallest, even, &mut Meter::new(limits(64, 1, 4, 1)))),
        (
            0x0000_0000,
            flags(&[IeeeFlag::Underflow, IeeeFlag::Inexact])
        )
    );
    assert_eq!(
        narrow(smallest, even, &mut Meter::new(limits(64, 1, 3, 1))),
        retain_denied(3)
    );
}
