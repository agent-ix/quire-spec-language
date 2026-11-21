// SPDX-License-Identifier: AGPL-3.0-or-later
//! Native numeric collection element admission.

use super::*;
use filament_exact::{
    CardinalityBound, CollectionType, DecimalType, FloatType, IeeeWidth, RoundingMode, ValueType,
};
use serde_json::json;

#[trace("TC-1171", "FR-106-AC-60", "FR-106-AC-61")]
#[test]
fn native_collection_string_bytes_and_json_members_keep_their_payloads() {
    for (slot, first, second) in [
        (
            ValueType::String,
            json!({"kind":"string","text":"é"}),
            json!({"kind":"string","text":"e\u{301}"}),
        ),
        (
            ValueType::Bytes,
            json!({"kind":"bytes","text":"00ff"}),
            json!({"kind":"bytes","text":"01"}),
        ),
        (
            ValueType::JsonObject,
            json!({"kind":"json_object","text":"{\"a\":1}"}),
            json!({"kind":"json_object","text":"{\"b\":2}"}),
        ),
    ] {
        for (kind, tag) in [
            (CollectionKind::Set, "set"),
            (CollectionKind::Bag, "bag"),
            (CollectionKind::OrderedSet, "ordered-set"),
        ] {
            let value = admitted(
                json!({tag:[first.clone(),second.clone()]}),
                collection(kind, slot.clone()),
            )
            .unwrap();
            let Value::Collection(value) = value else {
                panic!("collection")
            };
            assert_eq!(value.elements().len(), 2);
            let duplicate = admitted(
                json!({tag:[first.clone(),first.clone()]}),
                collection(kind, slot.clone()),
            );
            if kind.is_unique() {
                assert_eq!(duplicate.unwrap_err().catalog.cause(), "duplicate-member");
            } else {
                assert!(duplicate.is_ok());
            }
        }
    }
}

fn decimal_slot() -> ValueType {
    ValueType::Decimal(
        DecimalType::new(
            Integer::from(0_i64),
            Integer::from(10000_i64),
            2,
            2,
            RoundingMode::Exact,
        )
        .unwrap(),
    )
}

#[trace("TC-1171", "FR-106-AC-54", "FR-106-AC-58")]
#[test]
fn native_collection_numeric_payload_defects_name_the_member() {
    for (slot, value, cause, member) in [
        (
            decimal_slot(),
            json!({"kind":"decimal","coefficient":"01","scale":"2"}),
            "invalid-value",
            "coefficient",
        ),
        (
            decimal_slot(),
            json!({"kind":"decimal","coefficient":"1","scale":"01"}),
            "invalid-value",
            "scale",
        ),
        (
            decimal_slot(),
            json!({"kind":"decimal","coefficient":"1","scale":2}),
            "wrong-value-kind",
            "scale",
        ),
        (
            decimal_slot(),
            json!({"kind":"decimal","coefficient":"1"}),
            "missing-member",
            "scale",
        ),
        (
            ValueType::Float(FloatType::exact(IeeeWidth::Binary64)),
            json!({"kind":"float64","bits":"3FF0000000000000"}),
            "invalid-value",
            "bits",
        ),
        (
            ValueType::Float(FloatType::exact(IeeeWidth::Binary64)),
            json!({"kind":"float64","bits":"3ff000000000000"}),
            "invalid-value",
            "bits",
        ),
        (
            ValueType::Float(FloatType::exact(IeeeWidth::Binary32)),
            json!({"kind":"float32","bits":"3ff0000000000000"}),
            "invalid-value",
            "bits",
        ),
        (
            ValueType::Float(FloatType::exact(IeeeWidth::Binary64)),
            json!({"kind":"float64","bits":0}),
            "wrong-value-kind",
            "bits",
        ),
    ] {
        let refusal = admitted(
            json!({"set":[value]}),
            collection(CollectionKind::Set, slot),
        )
        .unwrap_err();
        assert_eq!(refusal.catalog.cause(), cause);
        assert_eq!(refusal.path, [PathStep::Index(0)]);
        assert_eq!(
            refusal.fields.get("member").map(String::as_str),
            Some(member)
        );
    }
}

fn admitted(object: serde_json::Value, slot: ValueType) -> Result<Value, AdmissionRecord> {
    admitted_source(&object.to_string(), slot)
}

fn admitted_source(source: &str, slot: ValueType) -> Result<Value, AdmissionRecord> {
    let raw = snapshot(source);
    let types = TypeEnvironment::default();
    let mut references = References {
        views: &[],
        types: &types,
        admitted: BTreeMap::new(),
        completeness: BTreeMap::new(),
        unresolved: Vec::new(),
        clock_references: Vec::new(),
        record_keys: None,
        variant_keys: None,
    };
    admit_form(&mut references, raw.form(), &slot, None)
}

fn collection(kind: CollectionKind, element: ValueType) -> ValueType {
    ValueType::collection(CollectionType::new(
        kind,
        element,
        Some(CardinalityBound::new(0, 6).unwrap()),
    ))
}

#[trace(
    "TC-1171",
    "FR-106-AC-53",
    "FR-106-AC-54",
    "FR-106-AC-55",
    "FR-106-AC-56"
)]
#[trace("FSpec-FR-144-AC-17", "FSpec-FR-144-AC-18")]
#[test]
fn native_collection_numeric_equivalence_controls_duplicates_and_bags() {
    for (slot, first, equal) in [
        (
            decimal_slot(),
            json!({"kind":"decimal","coefficient":"150","scale":"2"}),
            json!({"kind":"decimal","coefficient":"15","scale":"1"}),
        ),
        (
            rational_slot(),
            json!({"kind":"rational","numerator":"2","denominator":"4"}),
            json!({"kind":"rational","numerator":"1","denominator":"2"}),
        ),
    ] {
        for (kind, tag) in [
            (CollectionKind::Set, "set"),
            (CollectionKind::OrderedSet, "ordered-set"),
        ] {
            let refusal = admitted(
                json!({tag:[first.clone(), equal.clone()]}),
                collection(kind, slot.clone()),
            )
            .unwrap_err();
            assert_eq!(refusal.catalog.cause(), "duplicate-member");
            assert_eq!(refusal.path, [PathStep::Index(1)]);
            assert_eq!(
                refusal.fields.get("earlier-index").map(String::as_str),
                Some("0")
            );
        }
        let bag = admitted(
            json!({"bag":[first, equal]}),
            collection(CollectionKind::Bag, slot),
        )
        .unwrap();
        let Value::Collection(bag) = bag else {
            panic!("collection")
        };
        assert_eq!(bag.elements().len(), 2);
        assert_eq!(
            filament_exact::compare_keys(&bag.elements()[0], &bag.elements()[1]),
            Some(std::cmp::Ordering::Equal)
        );
    }
}

#[trace("TC-1171", "FR-106-AC-53", "FR-106-AC-54")]
#[test]
fn native_collection_decimal_retains_ordered_spelling_and_checks_value_membership() {
    let decimal = |coefficient: &str, scale: &str| json!({"kind":"decimal","coefficient":coefficient,"scale":scale});
    let value = admitted(decimal("150", "2"), decimal_slot()).unwrap();
    let Value::Decimal(value) = value else {
        panic!("decimal")
    };
    assert_eq!(value.representation().coefficient().to_string(), "150");
    assert_eq!(value.representation().scale(), 2);
    assert!(admitted(decimal("1000", "3"), decimal_slot()).is_ok());
    for (coefficient, scale) in [("1", "3"), ("10001", "2"), ("-1", "0")] {
        let refusal = admitted(decimal(coefficient, scale), decimal_slot()).unwrap_err();
        assert_eq!(refusal.catalog.cause(), "invalid-value");
        assert_eq!(
            refusal.fields.get("expected").map(String::as_str),
            Some("Decimal[0, 10000; 2, 2]")
        );
    }
    let nested = collection(
        CollectionKind::Bag,
        collection(CollectionKind::Sequence, decimal_slot()),
    );
    let value = admitted(
        json!({"bag":[{"sequence":[decimal("150","2"),decimal("100","2")]}]}),
        nested,
    )
    .unwrap();
    let Value::Collection(outer) = value else {
        panic!("bag")
    };
    let Value::Collection(inner) = &outer.elements()[0] else {
        panic!("sequence")
    };
    let Value::Decimal(first) = &inner.elements()[0] else {
        panic!("decimal")
    };
    assert_eq!(
        (
            first.representation().coefficient().to_string(),
            first.representation().scale()
        ),
        ("15".to_owned(), 1)
    );
}

#[trace("TC-1171", "FR-106-AC-53", "FR-106-AC-60")]
#[test]
fn native_collection_options_admit_and_normalize_nested_decimals() {
    let slot = collection(
        CollectionKind::Set,
        ValueType::Option(std::sync::Arc::new(decimal_slot())),
    );
    let value = admitted(json!({"set":[{"present":{"kind":"decimal","coefficient":"150","scale":"2"}},{"absent":{}}]}),slot).unwrap();
    let Value::Collection(value) = value else {
        panic!("set")
    };
    assert_eq!(value.elements().len(), 2);
    let present = value
        .elements()
        .iter()
        .find_map(|element| {
            let Value::Option(option) = element else {
                panic!("option")
            };
            option.payload()
        })
        .expect("present option");
    let Value::Decimal(decimal) = present else {
        panic!("decimal")
    };
    assert_eq!(
        (
            decimal.representation().coefficient().to_string(),
            decimal.representation().scale()
        ),
        ("15".to_owned(), 1)
    );
}

#[trace("TC-1171", "FR-106-AC-57", "FR-106-AC-58")]
#[test]
fn native_collection_float_bits_preserve_zero_sign_and_nan_payload() {
    for (width, tag, bits) in [
        (
            IeeeWidth::Binary32,
            "float32",
            ["00000000", "80000000", "7fc00001", "7fc00002", "7f800001"],
        ),
        (
            IeeeWidth::Binary64,
            "float64",
            [
                "0000000000000000",
                "8000000000000000",
                "7ff8000000000001",
                "7ff8000000000002",
                "7ff0000000000001",
            ],
        ),
    ] {
        let slot = ValueType::Float(FloatType::exact(width));
        let values = bits.map(|bits| json!({"kind":tag,"bits":bits}));
        for (object, bits) in values.iter().zip(bits) {
            let value = admitted(object.clone(), slot.clone()).unwrap();
            let Value::Float(value) = value else {
                panic!("float")
            };
            assert_eq!(value.bits(), u64::from_str_radix(bits, 16).unwrap());
            assert_eq!(value.width(), width);
        }
        for (kind, tag) in [
            (CollectionKind::Set, "set"),
            (CollectionKind::OrderedSet, "ordered-set"),
        ] {
            let value = admitted(json!({tag:values}), collection(kind, slot.clone())).unwrap();
            let Value::Collection(value) = value else {
                panic!("collection")
            };
            assert_eq!(value.elements().len(), 5);
            let refusal = admitted(
                json!({tag:[values[2],values[2]]}),
                collection(kind, slot.clone()),
            )
            .unwrap_err();
            assert_eq!(refusal.catalog.cause(), "duplicate-member");
            assert_eq!(refusal.path, [PathStep::Index(1)]);
        }
        let bag = admitted(
            json!({"bag":[values[2],values[2]]}),
            collection(CollectionKind::Bag, slot.clone()),
        )
        .unwrap();
        let Value::Collection(bag) = bag else {
            panic!("bag")
        };
        assert_eq!(bag.elements().len(), 2);
        let nested = collection(
            CollectionKind::Set,
            ValueType::Option(std::sync::Arc::new(slot)),
        );
        let refusal = admitted(
            json!({"set":[{"present":values[2]},{"present":values[2]}]}),
            nested,
        )
        .unwrap_err();
        assert_eq!(refusal.catalog.cause(), "duplicate-member");
    }
}

#[trace("TC-1171", "FR-106-AC-53", "FR-106-AC-57")]
#[test]
fn native_collection_frame_identity_uses_decimal_value_and_float_bits() {
    let decimal = |coefficient: &str, scale: &str| {
        snapshot(&json!({"kind":"decimal","coefficient":coefficient,"scale":scale}).to_string())
    };
    assert!(decimal("150", "2").equals_by(&decimal("15", "1"), Some(&decimal_slot()), &|_, _| None));
    assert!(!decimal("150", "2").equals_by(
        &decimal("151", "2"),
        Some(&decimal_slot()),
        &|_, _| None
    ));
    for (first, second, equal) in [
        ("0000000000000000", "8000000000000000", false),
        ("7ff8000000000001", "7ff8000000000002", false),
        ("7ff8000000000001", "7ff8000000000001", true),
    ] {
        let float = |bits: &str| snapshot(&json!({"kind":"float64","bits":bits}).to_string());
        assert_eq!(
            float(first).equals_by(&float(second), None, &|_, _| None),
            equal
        );
    }
}

#[trace("TC-1231", "FR-106-AC-61")]
#[test]
fn native_collection_bytes_and_json_reject_noncanonical_or_mistyped_payloads() {
    for (slot, value, cause) in [
        (
            ValueType::Bytes,
            json!({"kind":"bytes","text":"00FF"}),
            "invalid-value",
        ),
        (
            ValueType::JsonObject,
            json!({"kind":"json_object","text":"{\"a\": 1}"}),
            "invalid-value",
        ),
        (
            ValueType::Bytes,
            json!({"kind":"json_object","text":"{}"}),
            "wrong-value-kind",
        ),
        (
            ValueType::JsonObject,
            json!({"kind":"bytes","text":"00ff"}),
            "wrong-value-kind",
        ),
        (
            ValueType::Bytes,
            json!({"kind":"string","text":"00ff"}),
            "wrong-value-kind",
        ),
        (
            ValueType::JsonObject,
            json!({"kind":"string","text":"{}"}),
            "wrong-value-kind",
        ),
        (
            ValueType::Bytes,
            json!({"kind":"bytes","text":1}),
            "wrong-value-kind",
        ),
        (
            ValueType::JsonObject,
            json!({"kind":"json_object","text":1}),
            "wrong-value-kind",
        ),
        (
            ValueType::Bytes,
            json!({"kind":"bytes","text":"00ff","extra":1}),
            "unknown-member",
        ),
        (
            ValueType::JsonObject,
            json!({"kind":"json_object","text":"{}","extra":1}),
            "unknown-member",
        ),
    ] {
        let record = admitted(value.clone(), slot.clone()).unwrap_err();
        assert_eq!(record.catalog.code(), Code::InvalidRuntimeInput.as_str());
        assert_eq!(record.catalog.cause(), cause, "{value}");
        let record = admitted(
            json!({"set":[value]}),
            collection(CollectionKind::Set, slot),
        )
        .unwrap_err();
        assert_eq!(record.catalog.cause(), cause);
        assert_eq!(record.path, [PathStep::Index(0)]);
    }
}

#[trace(
    "TC-1171",
    "FR-106-AC-54",
    "FR-106-AC-56",
    "FR-106-AC-58",
    "FR-106-AC-60"
)]
#[trace("FSpec-FR-144-AC-18", "FSpec-FR-144-AC-20")]
#[test]
fn native_collection_bounds_and_element_validation_precede_duplicates() {
    for (slot, value) in [
        (
            decimal_slot(),
            json!({"kind":"decimal","coefficient":"150","scale":"2"}),
        ),
        (
            rational_slot(),
            json!({"kind":"rational","numerator":"1","denominator":"2"}),
        ),
        (
            ValueType::Float(FloatType::exact(IeeeWidth::Binary64)),
            json!({"kind":"float64","bits":"0000000000000000"}),
        ),
        (ValueType::String, json!({"kind":"string","text":"a"})),
    ] {
        let slot = collection(CollectionKind::Set, slot);
        let bad = json!({"kind":"string","text":1});
        let record = admitted(
            json!({"set":[value.clone(),value,bad.clone()]}),
            slot.clone(),
        )
        .unwrap_err();
        assert_eq!(record.catalog.cause(), "wrong-value-kind");
        assert_eq!(record.path, [PathStep::Index(2)]);
        let record = admitted(json!({"set":vec![bad;7]}), slot).unwrap_err();
        assert_eq!(record.catalog.code(), Code::CardinalityOutOfBound.as_str());
        assert_eq!(record.catalog.cause(), "above-maximum");
        assert!(record.path.is_empty());
    }
}

#[trace("TC-1171", "FR-106-AC-66", "FR-106-AC-68", "FR-106-AC-69")]
#[test]
fn native_collection_envelopes_keep_document_order_and_nested_paths() {
    let slot = collection(CollectionKind::Set, ValueType::Integer);
    for (source, cause, member) in [
        (r#"{"set":1,"z":0,"a":0}"#, "unknown-member", "z"),
        (r#"{"z":0,"set":[],"a":0}"#, "unknown-member", "z"),
        (r#"{"set":1}"#, "wrong-value-kind", "set"),
        (r#"{"unrecognized":1}"#, "missing-member", "set"),
    ] {
        let record = admitted_source(source, slot.clone()).unwrap_err();
        assert_eq!(record.catalog.cause(), cause);
        assert_eq!(
            record.fields.get("member").map(String::as_str),
            Some(member)
        );
        let wrapped = format!(r#"{{"sequence":[{{"present":{source}}}]}}"#);
        let nested = collection(
            CollectionKind::Sequence,
            ValueType::Option(std::sync::Arc::new(slot.clone())),
        );
        let record = admitted_source(&wrapped, nested).unwrap_err();
        assert_eq!(record.catalog.cause(), cause);
        assert_eq!(record.path, [PathStep::Index(0), PathStep::OptionValue]);
    }
}

/// The refusal of `object` at `slot`, and the document value its emitted
/// pointer resolves to.
fn located(object: serde_json::Value, slot: ValueType) -> (AdmissionRecord, serde_json::Value) {
    let record = admitted(object.clone(), slot.clone()).unwrap_err();
    let raw = snapshot(&object.to_string());
    // The pointer a snapshot field refusal carries in its `path` member.
    let pointer = pointer_of(&refusal_steps(raw.form(), &record));
    let resolved = object
        .pointer(&pointer)
        .unwrap_or_else(|| panic!("{pointer} resolves in {object}"))
        .clone();
    (record, resolved)
}

/// FR-663 and FR-106: the pointer to a nested duplicate names the failing
/// element, written as the document spells it (`/set/1`), and resolves to it.
#[trace("TC-1171", "FR-106-AC-54")]
#[test]
fn a_nested_duplicate_pointer_resolves_to_the_second_element() {
    let first = json!({"kind":"decimal","coefficient":"150","scale":"2"});
    let second = json!({"kind":"decimal","coefficient":"15","scale":"1"});
    let (record, resolved) = located(
        json!({"set":[first.clone(), second.clone()]}),
        collection(CollectionKind::Set, decimal_slot()),
    );
    assert_eq!(record.catalog.cause(), "duplicate-member");
    assert_eq!(resolved, second);
    let (_, resolved) = located(
        json!({"present": {"set":[first.clone(), first.clone()]}}),
        ValueType::Option(std::sync::Arc::new(collection(
            CollectionKind::Set,
            decimal_slot(),
        ))),
    );
    assert_eq!(resolved, first);
}

/// FR-663 and FR-106: the pointer to a nested Decimal defect ends at the
/// defective member of the element (`/set/0/scale`) and resolves to it.
#[trace("TC-1171", "FR-106-AC-54")]
#[test]
fn a_nested_decimal_defect_pointer_resolves_to_the_member() {
    let (record, resolved) = located(
        json!({"set":[{"kind":"decimal","coefficient":"1","scale":"01"}]}),
        collection(CollectionKind::Set, decimal_slot()),
    );
    assert_eq!(record.catalog.cause(), "invalid-value");
    assert_eq!(resolved, json!("01"));
}
