// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-038: independent exact vectors and strict numeric wire refusal controls.

use ix_trace_rs::trace;
use quire_spec_language::protocol_artifact::{
    ExactInteger, ExactRational, NumberComponent, NumberError, NumberWire, ProtocolNumber,
    NUMERIC_PROFILE,
};
use serde_json::{json, Value};

#[test]
#[trace("TC-117", "FR-038-AC-1")]
fn integers_match_independent_exact_objects() {
    assert_eq!(NUMERIC_PROFILE, "quire.protocol.numeric/1");
    let vectors = [
        (0, "0"),
        (-9_007_199_254_740_991, "-9007199254740991"),
        (9_007_199_254_740_991, "9007199254740991"),
        (-9_007_199_254_740_992, "-9007199254740992"),
        (9_007_199_254_740_992, "9007199254740992"),
        (i64::MIN, "-9223372036854775808"),
        (i64::MAX, "9223372036854775807"),
    ];
    for (value, decimal) in vectors {
        let integer = ExactInteger::new(value);
        let expected = json!({"kind": "integer", "decimal": decimal});
        assert_eq!(serde_json::to_value(integer).unwrap(), expected);
        let decoded: ProtocolNumber = serde_json::from_value(expected.clone()).unwrap();
        assert_eq!(decoded, ProtocolNumber::Integer(integer));
        assert_eq!(serde_json::to_value(decoded).unwrap(), expected);
        let exact: ExactInteger = serde_json::from_value(expected).unwrap();
        assert_eq!(exact.value(), value);
        assert_eq!(ExactInteger::from_decimal(decimal), Ok(integer));
    }
}

#[test]
#[trace("TC-117", "FR-038-AC-2")]
fn rationals_match_independent_exact_objects() {
    let vectors = [
        (1, 3, "1", "3"),
        (-1, 2, "-1", "2"),
        (1, 1, "1", "1"),
        (0, 1, "0", "1"),
        (i64::MIN, 1, "-9223372036854775808", "1"),
        (i64::MAX, 1, "9223372036854775807", "1"),
        (1, i64::MAX, "1", "9223372036854775807"),
        (
            i64::MIN,
            i64::MAX,
            "-9223372036854775808",
            "9223372036854775807",
        ),
    ];
    for (numerator, denominator, numerator_text, denominator_text) in vectors {
        let rational = ExactRational::new(numerator, denominator).unwrap();
        let expected = json!({
            "kind": "rational", "numerator": numerator_text, "denominator": denominator_text,
        });
        assert_eq!(serde_json::to_value(rational).unwrap(), expected);
        let decoded: ProtocolNumber = serde_json::from_value(expected.clone()).unwrap();
        assert_eq!(decoded, ProtocolNumber::Rational(rational));
        assert_eq!(serde_json::to_value(decoded).unwrap(), expected);
        let exact: ExactRational = serde_json::from_value(expected).unwrap();
        assert_eq!(
            (exact.numerator(), exact.denominator()),
            (numerator, denominator)
        );
    }
}

#[test]
#[trace("TC-117", "FR-038-AC-2", "FR-038-AC-4")]
fn integer_and_rational_kinds_are_not_coerced() {
    let integer = json!({"kind": "integer", "decimal": "1"});
    let rational = json!({"kind": "rational", "numerator": "1", "denominator": "1"});
    let decoded_integer: ProtocolNumber = serde_json::from_value(integer.clone()).unwrap();
    let decoded_rational: ProtocolNumber = serde_json::from_value(rational.clone()).unwrap();
    assert_ne!(decoded_integer, decoded_rational);
    assert!(serde_json::from_value::<ExactInteger>(rational).is_err());
    assert!(serde_json::from_value::<ExactRational>(integer).is_err());
}

fn offered_component(component: NumberComponent, text: &str) -> Value {
    match component {
        NumberComponent::Decimal => json!({"kind": "integer", "decimal": text}),
        NumberComponent::Numerator => {
            json!({"kind": "rational", "numerator": text, "denominator": "1"})
        }
        NumberComponent::Denominator => {
            json!({"kind": "rational", "numerator": "1", "denominator": text})
        }
    }
}

#[test]
#[trace("TC-117", "FR-038-AC-3")]
fn noncanonical_decimal_components_have_typed_failures() {
    for text in [
        "", "-", "-0", "+1", "01", "-01", "1.0", "1e3", "1E3", "１", "١", " 1", "1 ", "--1",
    ] {
        for component in [
            NumberComponent::Decimal,
            NumberComponent::Numerator,
            NumberComponent::Denominator,
        ] {
            let object = offered_component(component, text);
            let wire: NumberWire = serde_json::from_value(object.clone()).unwrap();
            assert_eq!(
                ProtocolNumber::try_from(&wire),
                Err(NumberError::NonCanonicalDecimal { component }),
                "{component}: {text:?}",
            );
            assert!(serde_json::from_value::<ProtocolNumber>(object).is_err());
        }
    }
}

#[test]
#[trace("TC-117", "FR-038-AC-3")]
fn component_overflow_has_a_distinct_typed_failure() {
    for text in ["9223372036854775808", "-9223372036854775809"] {
        for component in [
            NumberComponent::Decimal,
            NumberComponent::Numerator,
            NumberComponent::Denominator,
        ] {
            let object = offered_component(component, text);
            let wire: NumberWire = serde_json::from_value(object.clone()).unwrap();
            assert_eq!(
                ProtocolNumber::try_from(wire),
                Err(NumberError::ComponentOutOfRange { component }),
            );
            assert!(serde_json::from_value::<ProtocolNumber>(object).is_err());
        }
    }
}

#[test]
#[trace("TC-117", "FR-038-AC-3")]
fn invalid_rationals_are_refused_without_reduction_or_sign_repair() {
    let vectors = [
        ("1", "0", NumberError::NonPositiveDenominator),
        ("1", "-2", NumberError::NonPositiveDenominator),
        (
            "1",
            "-9223372036854775808",
            NumberError::NonPositiveDenominator,
        ),
        ("2", "4", NumberError::UnreducedRational),
        ("-2", "4", NumberError::UnreducedRational),
        ("0", "2", NumberError::UnreducedRational),
        ("-9223372036854775808", "2", NumberError::UnreducedRational),
    ];
    for (numerator, denominator, expected) in vectors {
        assert_eq!(
            ExactRational::from_decimals(numerator, denominator),
            Err(expected)
        );
        let object = json!({
            "kind": "rational", "numerator": numerator, "denominator": denominator,
        });
        let wire: NumberWire = serde_json::from_value(object.clone()).unwrap();
        assert_eq!(ProtocolNumber::try_from(wire), Err(expected));
        assert!(serde_json::from_value::<ProtocolNumber>(object).is_err());
    }
    for (numerator, denominator) in [(1, 1), (1, 2), (-1, 2), (0, 1), (i64::MIN / 2, 1)] {
        let exact = ExactRational::new(numerator, denominator).unwrap();
        assert_eq!(
            (exact.numerator(), exact.denominator()),
            (numerator, denominator)
        );
    }
    assert_eq!(
        ExactRational::new(1, 0),
        Err(NumberError::NonPositiveDenominator)
    );
    assert_eq!(
        ExactRational::new(2, 4),
        Err(NumberError::UnreducedRational)
    );
}

#[test]
#[trace("TC-117", "FR-038-AC-3", "FR-038-AC-4")]
fn raw_number_tokens_never_become_decimal_components() {
    for token in [
        "0",
        "1",
        "-9007199254740991",
        "9007199254740991",
        "-9007199254740992",
        "9007199254740992",
        "-9223372036854775808",
        "9223372036854775807",
        "9223372036854775808",
        "1.0",
        "1e3",
        "-0",
        "true",
        "null",
        "[]",
        "{}",
    ] {
        for offered in [
            format!(r#"{{"kind":"integer","decimal":{token}}}"#),
            format!(r#"{{"kind":"rational","numerator":{token},"denominator":"1"}}"#),
            format!(r#"{{"kind":"rational","numerator":"1","denominator":{token}}}"#),
        ] {
            assert!(
                serde_json::from_str::<NumberWire>(&offered).is_err(),
                "{offered}"
            );
            assert!(
                serde_json::from_str::<ProtocolNumber>(&offered).is_err(),
                "{offered}"
            );
        }
        assert!(
            serde_json::from_str::<ProtocolNumber>(token).is_err(),
            "{token}"
        );
    }
}

#[test]
#[trace("TC-117", "FR-038-AC-4")]
fn closed_shapes_reject_unknown_missing_duplicate_and_cross_kind_members() {
    for offered in [
        r#"{}"#,
        r#"{"decimal":"1"}"#,
        r#"{"kind":"integer"}"#,
        r#"{"numerator":"1","denominator":"2"}"#,
        r#"{"kind":"rational","numerator":"1"}"#,
        r#"{"kind":"rational","denominator":"2"}"#,
        r#"{"kind":"integer","decimal":"1","extra":"x"}"#,
        r#"{"kind":"rational","numerator":"1","denominator":"2","extra":"x"}"#,
        r#"{"kind":"integer","kind":"integer","decimal":"1"}"#,
        r#"{"kind":"integer","\u006bind":"integer","decimal":"1"}"#,
        r#"{"kind":"integer","decimal":"1","decimal":"1"}"#,
        r#"{"kind":"rational","kind":"rational","numerator":"1","denominator":"2"}"#,
        r#"{"kind":"rational","numerator":"1","numerator":"1","denominator":"2"}"#,
        r#"{"kind":"rational","numerator":"1","denominator":"2","denominator":"2"}"#,
        r#"{"kind":"number","decimal":"1"}"#,
        r#"{"kind":"Integer","decimal":"1"}"#,
        r#"{"kind":1,"decimal":"1"}"#,
        r#"{"kind":null,"decimal":"1"}"#,
        r#"{"kind":"rational","decimal":"1"}"#,
        r#"{"kind":"integer","numerator":"1","denominator":"1"}"#,
        r#"{"kind":"integer","decimal":"1","numerator":"1"}"#,
        r#"{"kind":"integer","decimal":"1","denominator":"1"}"#,
        r#"{"kind":"rational","numerator":"1","denominator":"1","decimal":"1"}"#,
        r#"["integer","1"]"#,
        r#"["rational","1","2"]"#,
    ] {
        assert!(
            serde_json::from_str::<NumberWire>(offered).is_err(),
            "{offered}"
        );
        assert!(
            serde_json::from_str::<ProtocolNumber>(offered).is_err(),
            "{offered}"
        );
    }
    // Member ordering is the enclosing canonicalizer's concern, not another tag.
    let reordered: ProtocolNumber =
        serde_json::from_str(r#"{ "denominator": "2", "numerator": "1", "kind": "rational" }"#)
            .unwrap();
    assert_eq!(
        reordered,
        ProtocolNumber::Rational(ExactRational::new(1, 2).unwrap())
    );
}

#[test]
#[trace("TC-117", "FR-038-AC-5")]
fn wire_validation_does_not_apply_model_bounds_or_frontend_normalization() {
    let integer: ExactInteger =
        serde_json::from_str(r#"{"kind":"integer","decimal":"20"}"#).unwrap();
    let separately_selected_model_range = 0..=7;
    assert_eq!(integer.value(), 20);
    assert!(!separately_selected_model_range.contains(&integer.value()));

    // The already normalized frontend result has an independently valid wire
    // form. This codec neither runs that frontend nor admits its offered 2/4.
    let normalized: ExactRational =
        serde_json::from_str(r#"{"kind":"rational","numerator":"1","denominator":"2"}"#).unwrap();
    assert_eq!((normalized.numerator(), normalized.denominator()), (1, 2));
    assert_eq!(
        ExactRational::from_decimals("2", "4"),
        Err(NumberError::UnreducedRational)
    );
}
