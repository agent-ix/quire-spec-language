// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-428 (FR-096-AC-8): every kernel raise site of a value refusal fills its
//! payload from the declared target or width in play, checked by raising the
//! refusal for real and reading it back through `kernel_refusal_record`'s
//! rendered `expected` (and `actual`/`flags`) fields. A raise site that
//! swaps two widths, hard-wires one, or drops the domain fails here.

use ix_trace_rs::trace;
use qsl_foundation::diagnostic::kernel_refusal_record;
use quire_exact::{
    admit_text, convert_ieee_width, divide, evaluate_decimal, evaluate_ieee,
    evaluate_integer_arithmetic, evaluate_rational_arithmetic, exact_to_ieee, ieee_to_exact,
    modulo, Decimal, DecimalOperation, DecimalType, DivisionMember, DivisionProfile, IeeeExactTarget,
    IeeeOperation, IeeeValue, IeeeWidth, Integer, IntegerArithmetic, IntegerDomain,
    IntegerInterval, Meter, Outcome, Rational, RationalArithmetic, RationalDomain, Refusal,
    RoundingMode, ScalarLimits, TextPayload, TextProfile, TextType,
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

fn meter() -> Meter {
    Meter::new(UNLIMITED)
}

fn int(value: i64) -> Integer {
    Integer::from(value)
}

fn interval(lower: i64, upper: i64) -> IntegerInterval {
    IntegerInterval::spanning(int(lower), int(upper))
}

/// The refusal an outcome carries, or a failed test.
fn refusal<T: std::fmt::Debug>(outcome: Outcome<T>) -> Refusal {
    match outcome {
        Outcome::Refused(refusal) => refusal,
        other => panic!("a refusal, not {other:?}"),
    }
}

/// The record's code, cause and every field of a refusal, read back through
/// the real record map.
fn rendered(refusal: &Refusal) -> (String, String, Vec<(String, String)>) {
    let record = kernel_refusal_record(refusal, None).expect("a record");
    (
        record.code().code().to_owned(),
        record.code().cause().to_owned(),
        record
            .fields()
            .iter()
            .map(|(key, value)| ((*key).to_owned(), value.clone()))
            .collect(),
    )
}

fn assert_record(refusal: &Refusal, code: &str, cause: &str, fields: &[(&str, &str)]) {
    let expected: Vec<(String, String)> = fields
        .iter()
        .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
        .collect();
    assert_eq!(
        rendered(refusal),
        (code.to_owned(), cause.to_owned(), expected)
    );
}

/// The NaN payload refusal names the conversion's target as `expected` and
/// its source as `actual`, for both directions of `binary32`/`binary64`.
#[trace("TC-428", "FR-096-AC-8")]
#[test]
fn nan_payload_refusal_keeps_target_and_source_apart() {
    let refused = refusal(convert_ieee_width(
        IeeeValue::binary64(0x7ff0_0000_0040_0000),
        IeeeWidth::Binary32,
        RoundingMode::NearestEven,
        &mut meter(),
    ));
    assert_record(
        &refused,
        "ieee_nan_payload_not_representable",
        "payload-exceeds-target",
        &[("actual", "binary64"), ("expected", "binary32")],
    );
}

/// The strict-`exact` IEEE refusal names the operation's or conversion's
/// result width, not a fixed one, with its would-be flags.
#[trace("TC-428", "FR-096-AC-8")]
#[test]
fn ieee_not_exact_names_the_result_width_and_flags() {
    let inexact_f64 = refusal(
        evaluate_ieee(
            IeeeOperation::Add(
                IeeeValue::binary64(0x3ff0_0000_0000_0000),
                IeeeValue::binary64(0x3c30_0000_0000_0000),
            ),
            RoundingMode::Exact,
            &mut meter(),
        )
        .unwrap(),
    );
    assert_record(
        &inexact_f64,
        "ieee_not_exact",
        "rounding-required",
        &[("expected", "binary64"), ("flags", "inexact")],
    );
    let inexact_f32 = refusal(
        evaluate_ieee(
            IeeeOperation::Add(
                IeeeValue::binary32(0x3f80_0000),
                IeeeValue::binary32(0x3380_0000),
            ),
            RoundingMode::Exact,
            &mut meter(),
        )
        .unwrap(),
    );
    assert_record(
        &inexact_f32,
        "ieee_not_exact",
        "rounding-required",
        &[("expected", "binary32"), ("flags", "inexact")],
    );
    // A conversion names its target width, not its source's.
    let narrowed = refusal(convert_ieee_width(
        IeeeValue::binary64(0x3ff0_0000_0000_0001),
        IeeeWidth::Binary32,
        RoundingMode::Exact,
        &mut meter(),
    ));
    assert_record(
        &narrowed,
        "ieee_not_exact",
        "rounding-required",
        &[("expected", "binary32"), ("flags", "inexact")],
    );
    let from_exact = refusal(exact_to_ieee(
        &Rational::new(int(1), int(3)).unwrap(),
        IeeeWidth::Binary64,
        RoundingMode::Exact,
        &mut meter(),
    ));
    assert_record(
        &from_exact,
        "ieee_not_exact",
        "rounding-required",
        &[("expected", "binary64"), ("flags", "inexact")],
    );
    // Overflow and inexact, in vocabulary order.
    let overflow = refusal(convert_ieee_width(
        IeeeValue::binary64(0x7fef_ffff_ffff_ffff),
        IeeeWidth::Binary32,
        RoundingMode::Exact,
        &mut meter(),
    ));
    assert_record(
        &overflow,
        "ieee_not_exact",
        "rounding-required",
        &[("expected", "binary32"), ("flags", "overflow,inexact")],
    );
}

/// `ieee_to_exact` refuses with its `Rational[..]` target domain.
#[trace("TC-428", "FR-096-AC-8")]
#[test]
fn ieee_rational_refusal_names_its_target_domain() {
    let domain = RationalDomain::new(interval(0, 1), interval(1, 1)).unwrap();
    let refused = refusal(
        ieee_to_exact(
            IeeeValue::binary32(0x3f00_0000),
            IeeeExactTarget::Rational(&domain),
            &mut meter(),
        )
        .unwrap(),
    );
    assert_record(
        &refused,
        "ieee_rational_out_of_domain",
        "outside-domain",
        &[("expected", "Rational[0, 1; 1, 1]")],
    );
}

/// `mod` and `div`/`rem` refuse with the consumer's `Int[..]` domain, and
/// the `div`/`rem` cause follows the exposed member.
#[trace("TC-428", "FR-096-AC-8", "FR-096-AC-13")]
#[test]
fn division_refusals_name_the_consumer_domain_and_exposed_member() {
    let domain = IntegerDomain::Bounded(interval(0, 1));
    let refused = refusal(modulo(&int(7), &int(4), &domain, &mut meter()));
    assert_record(
        &refused,
        "modulo_out_of_domain",
        "outside-domain",
        &[("expected", "Int[0, 1]")],
    );
    for profile in DivisionProfile::ALL {
        for (member, dividend, divisor, cause) in [
            (DivisionMember::Quotient, 7, 2, "quotient-outside-domain"),
            (DivisionMember::Remainder, 7, 4, "remainder-outside-domain"),
        ] {
            let refused = refusal(divide(
                profile,
                member,
                &int(dividend),
                &int(divisor),
                &domain,
                &mut meter(),
            ));
            assert_record(
                &refused,
                "division_out_of_domain",
                cause,
                &[("expected", "Int[0, 1]")],
            );
        }
    }
}

/// Bounded integer and rational arithmetic refuse with their result domain.
#[trace("TC-428", "FR-096-AC-8")]
#[test]
fn arithmetic_refusals_name_their_result_domain() {
    let (five, bound) = (int(5), interval(0, 9));
    let refused = refusal(evaluate_integer_arithmetic(
        IntegerArithmetic::Add(&five, &five),
        Some(&bound),
        &mut meter(),
    ));
    assert_record(
        &refused,
        "integer_out_of_domain",
        "outside-domain",
        &[("expected", "Int[0, 9]")],
    );
    let half = Rational::new(int(1), int(2)).unwrap();
    let domain = RationalDomain::new(interval(-3, 9), interval(2, 9)).unwrap();
    let refused = refusal(evaluate_rational_arithmetic(
        RationalArithmetic::Add(&half, &half),
        Some(&domain),
        &mut meter(),
    ));
    assert_record(
        &refused,
        "rational_out_of_domain",
        "outside-domain",
        &[("expected", "Rational[-3, 9; 2, 9]")],
    );
}

/// Decimal refusals name the declared `Decimal[..]` target: strict `exact`
/// through an operation and through `placement`, and membership.
#[trace("TC-428", "FR-096-AC-8")]
#[test]
fn decimal_refusals_name_the_declared_target() {
    let target = DecimalType::new(int(-100), int(100), 0, 2, RoundingMode::Exact).unwrap();
    let three_places = Decimal::new(int(1234), 3);
    let refused = refusal(evaluate_decimal(
        DecimalOperation::Round(&three_places),
        &target,
        &mut meter(),
    ));
    assert_record(
        &refused,
        "inexact_decimal",
        "nonzero-discarded-digit",
        &[("expected", "Decimal[-100, 100; 0, 2]")],
    );
    let third = Rational::new(int(1), int(3)).unwrap();
    let refused = target.placement(&third).unwrap_err();
    assert_record(
        &refused,
        "inexact_decimal",
        "nonzero-discarded-digit",
        &[("expected", "Decimal[-100, 100; 0, 2]")],
    );
    let large = Decimal::new(int(50_000), 2);
    let refused = refusal(evaluate_decimal(
        DecimalOperation::Round(&large),
        &target,
        &mut meter(),
    ));
    assert_record(
        &refused,
        "decimal_out_of_domain",
        "outside-domain",
        &[("expected", "Decimal[-100, 100; 0, 2]")],
    );
}

/// Text admission refuses with the declared `Text[min, max; profile]`.
#[trace("TC-428", "FR-096-AC-8")]
#[test]
fn text_refusal_names_the_declared_bounds_and_profile() {
    let payload = TextPayload::from_utf8(b"hello").unwrap();
    let declared = TextType::new(0, 3, TextProfile::Nfc).unwrap();
    let refused = refusal(admit_text(&payload, &declared, &mut meter()));
    assert_record(
        &refused,
        "text_length_out_of_domain",
        "outside-domain",
        &[("expected", "Text[0, 3; nfc]")],
    );
}
