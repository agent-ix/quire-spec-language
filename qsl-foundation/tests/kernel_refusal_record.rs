// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-428 steps 4 and 5 (FR-096-AC-8, FR-096-AC-13): `kernel_refusal_record`
//! builds each kernel value refusal's record from the variant's own target
//! domain or width, spelled as the catalog spells it.

use std::collections::BTreeMap;

use ix_trace_rs::trace;
use qsl_foundation::diagnostic::{kernel_refusal_record, CatalogCode, Category, Locus};
use quire_exact::{
    DecimalType, IeeeFlag, IeeeWidth, InexactTarget, Integer, IntegerInterval, Location, NodeKey,
    Origin, RationalDomain, Refusal, RoundingMode, TextProfile, TextType,
};

fn int(value: i64) -> Integer {
    Integer::from(value)
}

fn interval(lower: i64, upper: i64) -> Box<IntegerInterval> {
    Box::new(IntegerInterval::spanning(int(lower), int(upper)))
}

fn rational(lo: i64, hi: i64, dmin: i64, dmax: i64) -> Box<RationalDomain> {
    Box::new(RationalDomain::new(*interval(lo, hi), *interval(dmin, dmax)).unwrap())
}

fn locus() -> Locus {
    Locus::Occurrence(Location::new(
        NodeKey::from_digest([7; 32]),
        Origin::new("body".into(), 0),
    ))
}

/// The record of `refusal` at [`locus`], its catalog code and cause checked
/// against the refusal's own `code()`/`cause()`, its category and locus, and
/// its fields as `(key, value)` pairs.
fn record(
    refusal: &Refusal,
    code: &'static str,
    cause: &'static str,
) -> Vec<(&'static str, String)> {
    assert_eq!(refusal.code(), Some(code));
    assert_eq!(refusal.cause(), Some(cause));
    let record = kernel_refusal_record(refusal, Some(locus())).expect("a record");
    assert_eq!(record.code(), CatalogCode::new(code, cause));
    assert_eq!(record.category(), Category::Refusal);
    assert_eq!(record.locus(), Some(&locus()));
    let fields: &BTreeMap<&'static str, String> = record.fields();
    fields
        .iter()
        .map(|(key, value)| (*key, value.clone()))
        .collect()
}

fn expected(value: &str) -> Vec<(&'static str, String)> {
    vec![("expected", value.to_owned())]
}

/// FR-096-AC-8: each domain refusal's `expected` field is its declared
/// target's exact spelling.
#[trace("TC-428", "FR-096-AC-8")]
#[test]
fn a_domain_refusal_renders_its_declared_target() {
    assert_eq!(
        record(
            &Refusal::IntegerOutOfDomain {
                target: interval(-5, 9)
            },
            "integer_out_of_domain",
            "outside-domain"
        ),
        expected("Int[-5, 9]")
    );
    let decimal = DecimalType::new(int(-100), int(100), 0, 2, RoundingMode::Exact).unwrap();
    assert_eq!(
        record(
            &Refusal::DecimalOutOfDomain {
                target: Box::new(decimal.clone())
            },
            "decimal_out_of_domain",
            "outside-domain"
        ),
        expected("Decimal[-100, 100; 0, 2]")
    );
    assert_eq!(
        record(
            &Refusal::RationalOutOfDomain {
                target: rational(-9, 9, 1, 9)
            },
            "rational_out_of_domain",
            "outside-domain"
        ),
        expected("Rational[-9, 9; 1, 9]")
    );
    assert_eq!(
        record(
            &Refusal::IeeeRationalOutOfDomain {
                target: rational(-9, 9, 1, 9)
            },
            "ieee_rational_out_of_domain",
            "outside-domain"
        ),
        expected("Rational[-9, 9; 1, 9]")
    );
    assert_eq!(
        record(
            &Refusal::TextLengthOutOfDomain {
                target: TextType::new(1, 8, TextProfile::Nfc).unwrap()
            },
            "text_length_out_of_domain",
            "outside-domain"
        ),
        expected("Text[1, 8; nfc]")
    );
    assert_eq!(
        record(
            &Refusal::ModuloOutOfDomain {
                domain: interval(0, 9)
            },
            "modulo_out_of_domain",
            "outside-domain"
        ),
        expected("Int[0, 9]")
    );
    // A decimal target keeps `Decimal[..]`; an integer target renders the
    // declared `Int[lo, hi]`, not a scale-zero decimal.
    assert_eq!(
        record(
            &Refusal::InexactDecimal {
                target: InexactTarget::Decimal(Box::new(decimal))
            },
            "inexact_decimal",
            "nonzero-discarded-digit"
        ),
        expected("Decimal[-100, 100; 0, 2]")
    );
    assert_eq!(
        record(
            &Refusal::InexactDecimal {
                target: InexactTarget::Integer(interval(0, 9))
            },
            "inexact_decimal",
            "nonzero-discarded-digit"
        ),
        expected("Int[0, 9]")
    );
}

/// FR-096-AC-8: the IEEE refusals render widths and the flag set in
/// vocabulary order, joined by `,`.
#[trace("TC-428", "FR-096-AC-8")]
#[test]
fn an_ieee_refusal_renders_widths_and_ordered_flags() {
    // Given in reverse of vocabulary order: the record still orders them.
    let would_be = [IeeeFlag::Inexact, IeeeFlag::Overflow]
        .into_iter()
        .collect();
    assert_eq!(
        record(
            &Refusal::IeeeNotExact {
                target: IeeeWidth::Binary32,
                would_be
            },
            "ieee_not_exact",
            "rounding-required"
        ),
        vec![
            ("expected", "binary32".to_owned()),
            ("flags", "overflow,inexact".to_owned())
        ]
    );
    assert_eq!(
        record(
            &Refusal::IeeeNanPayloadNotRepresentable {
                target: IeeeWidth::Binary32,
                source: IeeeWidth::Binary64
            },
            "ieee_nan_payload_not_representable",
            "payload-exceeds-target"
        ),
        vec![
            ("actual", "binary64".to_owned()),
            ("expected", "binary32".to_owned())
        ]
    );
}

/// FR-096-AC-8: the checked-invariant refusal builds no record.
#[trace("TC-428", "FR-096-AC-8")]
#[test]
fn a_checked_invariant_builds_no_record() {
    assert_eq!(
        kernel_refusal_record(&Refusal::CheckedInvariant, Some(locus())),
        None
    );
}

/// FR-096-AC-13 (TC-428 step 5): a division pair's cause follows which
/// members fail membership, and `expected` is the consumer's `Int[0, 9]`.
#[trace("TC-428", "FR-096-AC-13")]
#[test]
fn a_division_pair_cause_follows_which_members_fail() {
    for (quotient_admitted, remainder_admitted, cause) in [
        (false, true, "quotient-outside-domain"),
        (true, false, "remainder-outside-domain"),
        (false, false, "both-outside-domain"),
    ] {
        let refusal = Refusal::DivisionPairOutOfDomain {
            domain: interval(0, 9),
            quotient_admitted,
            remainder_admitted,
        };
        assert_eq!(
            record(&refusal, "division_pair_out_of_domain", cause),
            expected("Int[0, 9]")
        );
    }
}
