// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-428 steps 4 and 5 (FR-096-AC-8, FR-096-AC-13): `kernel_refusal_record`
//! builds each kernel value refusal's record from the variant's own target
//! domain or width, spelled as the catalog spells it.

use std::collections::BTreeMap;

use ix_trace_rs::trace;
use qsl_foundation::diagnostic::{catalog_category, category_of, kernel_refusal_record, Locus};
use quire_exact::{
    DecimalType, DivisionMember, IeeeFlag, IeeeWidth, InexactTarget, Integer, IntegerInterval,
    Location, NodeKey, Origin, PathSegment, QuantityResultInterval, Rational, RationalDomain,
    Refusal, RoundingMode, TextProfile, TextType, UnitId,
};
use quire_semantic_value::catalog::{CatalogCode, Category};

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
        NodeKey::new("pkg", vec![PathSegment::Text("n".into())], 7).expect("node key"),
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

/// A bounded quantity retains exact endpoints and complete unit identity;
/// only the expected target is exposed, at the caller's locus.
#[trace("TC-428", "FR-096-AC-8")]
#[test]
fn a_bounded_quantity_refusal_renders_its_structural_target() {
    for package in ["acme/rpc", "other/rpc"] {
        let key = NodeKey::new(package, vec![PathSegment::Text("cm".into())], 0).expect("unit key");
        let declared = UnitId::declared(key.clone());
        let compound = UnitId::compound(vec![(key, int(2))]);
        for (unit, spelling) in [
            (declared, format!("{package}:[\"cm\",]:0")),
            (compound, format!("{package}:[\"cm\",]:0^2")),
        ] {
            let refusal = Refusal::BoundedQuantityOutOfDomain {
                target: Box::new(QuantityResultInterval::new(
                    unit,
                    Rational::new(int(2), int(2)).expect("minimum"),
                    Rational::new(int(6), int(3)).expect("maximum"),
                )),
            };
            assert_eq!(
                record(&refusal, "rational_out_of_domain", "outside-domain"),
                expected(&format!("Quantity<{spelling}>[1/1, 2/1]"))
            );
            let result_locus = Locus::Artifact {
                pointer: "/result_type".parse().expect("result type pointer"),
            };
            let built = kernel_refusal_record(&refusal, Some(result_locus.clone()))
                .expect("bounded quantity record");
            assert_eq!(built.locus(), Some(&result_locus));
            assert!(!built.fields().contains_key("actual"));
        }
    }
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

/// FR-497-AC-7: the `InexactConversion` kernel refusal renders as code
/// `inexact_conversion`, cause `inexact-conversion`, with `expected` the
/// carrier type text, for a `Decimal` and an `Int` carrier.
#[trace("TC-1287", "FR-497-AC-7", "FR-096-AC-8")]
#[test]
fn an_inexact_conversion_renders_its_carrier_type() {
    let decimal = DecimalType::new(int(-100), int(100), 2, 2, RoundingMode::Exact).unwrap();
    assert_eq!(
        record(
            &Refusal::InexactConversion {
                target: InexactTarget::Decimal(Box::new(decimal))
            },
            "inexact_conversion",
            "inexact-conversion"
        ),
        expected("Decimal[-100, 100; 2, 2]")
    );
    assert_eq!(
        record(
            &Refusal::InexactConversion {
                target: InexactTarget::Integer(interval(-2, 2))
            },
            "inexact_conversion",
            "inexact-conversion"
        ),
        expected("Int[-2, 2]")
    );
    // The code is in the catalog as a refusal, so it exits 20 like the other
    // value refusals and the wire reader accepts its spelling.
    let refusal = Refusal::InexactConversion {
        target: InexactTarget::Integer(interval(-2, 2)),
    };
    let built = kernel_refusal_record(&refusal, Some(locus())).expect("a record");
    assert_eq!(category_of(&built.code()), Some(Category::Refusal));
    assert_eq!(
        catalog_category("inexact_conversion"),
        Some(Category::Refusal)
    );
    assert_eq!(Category::Refusal.exit_code(), 20);
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
        kernel_refusal_record(
            &Refusal::CheckedInvariant {
                cause: quire_exact::CheckedInvariantCause::CollectionElementNotAdmitted
            },
            Some(locus())
        ),
        None
    );
}

/// FR-096-AC-13 (TC-428 step 5): a division refusal's cause follows the
/// exposed member, and `expected` is the consumer's `Int[0, 9]`.
#[trace("TC-428", "FR-096-AC-13")]
#[test]
fn a_division_cause_follows_the_exposed_member() {
    for (member, cause) in [
        (DivisionMember::Quotient, "quotient-outside-domain"),
        (DivisionMember::Remainder, "remainder-outside-domain"),
    ] {
        let refusal = Refusal::DivisionOutOfDomain {
            domain: interval(0, 9),
            member,
        };
        assert_eq!(
            record(&refusal, "division_out_of_domain", cause),
            expected("Int[0, 9]")
        );
    }
}
