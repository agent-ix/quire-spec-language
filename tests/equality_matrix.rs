// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-194 complete equality matrix over the real `value` boundary (FR-149).
//!
//! E01–E16 and E20–E24 are transcribed from the vendored TC-194 procedure.
//! E17–E19 need the FR-148 IEEE module, which is not in this branch's base
//! (`Remaining work: #118`). Unit and enum node keys come from an independent
//! RFC 8785 canonicalizer; composite declaration keys are opaque fixture keys
//! (SPEC-GAP(119-5)).

use ix_trace_rs::trace;
use quire_spec_language::value::{
    admit_text, convert_for_equality, convert_quantity, evaluate_equality, plan_equality,
    ChargePoint, CollectionKind, CollectionValue, CompositeDeclaration, CompositeShape,
    ConstructorDeclaration, ConvertedValue, Decimal, DecimalType, DimensionPreimage,
    EnumDeclaration, EnumDeclarationPreimage, EnumMemberPreimage, EnumValue, FieldDeclaration,
    FieldExpression, FieldValue, IllTyped, IllTypedCause, Incomplete, InjectedDenial, Integer,
    LimitKind, Meter, NodeKey, NodeOwner, ObjectEnvironment, ObjectIdentity, ObjectReference,
    OptionValue, Outcome, OwnerSelection, OwnerSubject, Presence, Quantity, QuantityTarget,
    QuantityUnit, Rational, Refusal, RoundingMode, ScalarLimits, Text, TextPayload, TextProfile,
    TextType, TypeEnvironment, Undefined, UnitGraph, UnitPreimage, Value, ValueType,
};
use serde_json::json;
use sha2::{Digest, Sha256};

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

const ZERO: ScalarLimits = ScalarLimits {
    integer_bits: 0,
    decimal_digits: 0,
    scale_expansion: 0,
    text_input_bytes: 0,
    text_scalars: 0,
    normalized_scalars: 0,
    unit_edges: 0,
    value_occurrences: 0,
    work_units: 0,
    result_units: 0,
};

// ---- fixtures --------------------------------------------------------------

/// Independent RFC 8785 JCS for the string/Boolean-only preimages.
fn jcs(value: &serde_json::Value) -> String {
    use serde_json::Value as Json;
    match value {
        Json::Null | Json::Bool(_) | Json::String(_) => value.to_string(),
        Json::Number(_) => panic!("preimages carry no JSON numbers"),
        Json::Array(items) => format!("[{}]", items.iter().map(jcs).collect::<Vec<_>>().join(",")),
        Json::Object(members) => {
            let mut entries: Vec<_> = members.iter().collect();
            entries.sort_by(|(a, _), (b, _)| a.encode_utf16().cmp(b.encode_utf16()));
            let body: Vec<_> = entries
                .into_iter()
                .map(|(key, item)| format!("{}:{}", Json::String(key.clone()), jcs(item)))
                .collect();
            format!("{{{}}}", body.join(","))
        }
    }
}

fn hex(bytes: &[u8]) -> NodeKey {
    let digest: String = Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    NodeKey::from_hex(&digest).unwrap()
}

fn fixture_key(preimage: &serde_json::Value) -> NodeKey {
    hex(jcs(preimage).as_bytes())
}

/// An opaque composite declaration, field or constructor key.
fn key(label: &str) -> NodeKey {
    hex(label.as_bytes())
}

fn owners() -> OwnerSelection {
    OwnerSelection::new([NodeOwner::Definition(OwnerSubject {
        authority: "agent-ix".into(),
        identity: "example-model".into(),
    })])
}

fn owner_json() -> serde_json::Value {
    json!({"kind": "definition", "authority": "agent-ix", "identity": "example-model"})
}

fn node_id(key: NodeKey) -> serde_json::Value {
    json!({"domain": "quire.checked-semantic-node/v1", "digest": key.to_string()})
}

struct Units {
    m: QuantityUnit,
    cm: QuantityUnit,
    s: QuantityUnit,
}

fn units() -> Units {
    let dimension = |name: &str| {
        json!({
            "version": "quire.dimension-node/v1",
            "owner": owner_json(),
            "qualified_declaration": ["Example", name],
            "terms": [],
        })
    };
    let unit = |name: &str, dimension: NodeKey, target: Option<NodeKey>, scale: &str| {
        json!({
            "version": "quire.unit-node/v1",
            "owner": owner_json(),
            "qualified_declaration": ["Example", name],
            "dimension_node_id": node_id(dimension),
            "target_unit_node_id": target.map_or(serde_json::Value::Null, node_id),
            "scale": {"numerator": "1", "denominator": scale},
            "offset": {"numerator": "0", "denominator": "1"},
        })
    };
    let length = dimension("Length");
    let time = dimension("Time");
    let (length_key, time_key) = (fixture_key(&length), fixture_key(&time));
    let m = unit("metre", length_key, None, "1");
    let m_key = fixture_key(&m);
    let s = unit("second", time_key, None, "1");
    let cm = unit("centimetre", length_key, Some(m_key), "100");
    let keys = [fixture_key(&m), fixture_key(&cm), fixture_key(&s)];
    let graph = UnitGraph::admit(
        [length, time].into_iter().map(|preimage| {
            let key = fixture_key(&preimage);
            (DimensionPreimage::from_json(preimage).unwrap(), key)
        }),
        [m, cm, s].into_iter().map(|preimage| {
            let key = fixture_key(&preimage);
            (UnitPreimage::from_json(preimage).unwrap(), key)
        }),
        &owners(),
    )
    .unwrap();
    let declared = |key| QuantityUnit::Declared(Box::new(graph.unit(key).unwrap().clone()));
    Units {
        m: declared(keys[0]),
        cm: declared(keys[1]),
        s: declared(keys[2]),
    }
}

fn enum_declaration(name: &str) -> EnumDeclaration {
    let preimage = json!({
        "version": "quire.enum-declaration-node/v1",
        "owner": owner_json(),
        "qualified_declaration": ["Example", name],
        "ordered": false,
        "members": ["DONE", "READY"],
    });
    let key = fixture_key(&preimage);
    EnumDeclaration::admit(
        EnumDeclarationPreimage::from_json(preimage).unwrap(),
        key,
        &owners(),
    )
    .unwrap()
}

fn member(declaration: &EnumDeclaration, case: &str) -> Value {
    let preimage = json!({
        "version": "quire.enum-member-node/v1",
        "declaration_node_id": node_id(declaration.key()),
        "case": case,
    });
    let key = fixture_key(&preimage);
    let member: EnumValue = declaration
        .admit_member(&EnumMemberPreimage::from_json(preimage).unwrap(), key)
        .unwrap();
    Value::Enum(member)
}

fn int(value: i64) -> Value {
    Value::Integer(Integer::from(value))
}

fn rational(numerator: i64, denominator: i64) -> Value {
    Value::Rational(Rational::new(Integer::from(numerator), Integer::from(denominator)).unwrap())
}

fn decimal(coefficient: i64, scale: u32) -> Value {
    Value::Decimal(Decimal::new(Integer::from(coefficient), scale))
}

fn text(payload: &str, profile: TextProfile) -> Value {
    let text: Text = admit_text(
        &TextPayload::from_utf8(payload.as_bytes()).unwrap(),
        &TextType::new(0, 16, profile).unwrap(),
        &mut Meter::new(UNLIMITED),
    )
    .completed()
    .unwrap();
    Value::Text(text)
}

fn equal(left: &Value, right: &Value) -> Result<Outcome<bool>, IllTyped> {
    evaluate_equality(left, right, &mut Meter::new(UNLIMITED))
}

fn is(expected: bool) -> Result<Outcome<bool>, IllTyped> {
    Ok(Outcome::Completed(expected))
}

fn ill_typed(cause: IllTypedCause) -> Result<Outcome<bool>, IllTyped> {
    Err(IllTyped { cause })
}

fn pairs(left: &Value, right: &Value) -> u64 {
    plan_equality(left, right)
        .unwrap()
        .pair_events()
        .to_u64()
        .unwrap()
}

/// Every value-kind row also compares its left operand with a disjoint kind.
fn assert_disjoint(left: &Value) {
    let disjoint = match left {
        Value::Boolean(_) => int(1),
        _ => Value::Boolean(true),
    };
    assert_eq!(
        equal(left, &disjoint),
        ill_typed(IllTypedCause::DistinctValueTypes),
        "{left:?}"
    );
    assert_eq!(
        equal(&disjoint, left),
        ill_typed(IllTypedCause::DistinctValueTypes),
        "{left:?}"
    );
}

fn collection(kind: CollectionKind, elements: &[i64]) -> Value {
    CollectionValue::construct(
        kind,
        ValueType::Integer,
        elements.iter().map(|value| int(*value)).collect(),
    )
    .unwrap()
}

// ---- scalar rows -------------------------------------------------------------

#[trace("TC-194", "FR-149-AC-1", "FR-149-AC-4")]
#[test]
fn e01_boolean_identical_truth_value() {
    let t = Value::Boolean(true);
    assert_eq!(equal(&t, &Value::Boolean(true)), is(true));
    assert_eq!(equal(&t, &Value::Boolean(false)), is(false));
    assert_disjoint(&t);
}

#[trace("TC-194", "FR-149-AC-3", "FR-149-AC-5")]
#[test]
fn e02_integers_and_explicit_lossless_rational_conversion() {
    let one = int(1);
    assert_eq!(equal(&one, &int(1)), is(true));
    assert_eq!(equal(&one, &int(2)), is(false));
    let converted = convert_for_equality(&one, &ValueType::Rational).unwrap();
    assert_eq!(equal(&converted, &rational(1, 1)), is(true));
    assert!(matches!(&one, Value::Integer(value) if *value == Integer::from(1_i64)));
    assert_eq!(
        equal(&one, &rational(1, 1)),
        ill_typed(IllTypedCause::DistinctValueTypes)
    );
    assert_disjoint(&one);
    assert_disjoint(&rational(1, 1));
}

#[trace("TC-194", "FR-149-AC-4", "FR-149-AC-5")]
#[test]
fn e03_decimals_compare_mathematically_and_convert_without_mutation() {
    let left = decimal(10, 1);
    assert_eq!(equal(&left, &decimal(100, 2)), is(true));
    assert_eq!(equal(&left, &decimal(11, 1)), is(false));
    let converted = convert_for_equality(&left, &ValueType::Rational).unwrap();
    let one = rational(1, 1);
    assert_eq!(equal(&converted, &one), is(true));
    let Value::Decimal(source) = &left else {
        panic!("the source stays a decimal");
    };
    assert_eq!(
        source.representation().coefficient(),
        &Integer::from(10_i64)
    );
    assert_eq!(source.representation().scale(), 1);
    assert!(matches!(&one, Value::Rational(_)));
    assert_disjoint(&left);
}

#[trace("TC-194", "FR-149-AC-3")]
#[test]
fn e04_lossy_decimal_conversion_is_ill_typed_not_false() {
    let third = rational(1, 3);
    let target = DecimalType::new(
        Integer::from(-100_i64),
        Integer::from(100_i64),
        0,
        2,
        RoundingMode::Exact,
    )
    .unwrap();
    assert!(matches!(
        convert_for_equality(&third, &ValueType::Decimal(target)),
        Err(IllTyped {
            cause: IllTypedCause::NoLosslessConversion
        })
    ));
    assert_eq!(
        equal(&third, &decimal(33, 2)),
        ill_typed(IllTypedCause::DistinctValueTypes)
    );
}

#[trace("TC-194", "FR-149-AC-3", "FR-149-AC-5")]
#[test]
fn e05_quantities_after_explicit_canonical_unit_conversion() {
    let units = units();
    let whole = |value: i64| Rational::from_integer(Integer::from(value));
    let centimetres = Quantity::new(whole(100), units.cm.clone());
    let snapshot = centimetres.clone();
    let conversion = convert_quantity(
        &centimetres,
        &units.m,
        &QuantityTarget::Exact,
        &mut Meter::new(UNLIMITED),
    )
    .unwrap()
    .completed()
    .unwrap();
    let ConvertedValue::Exact(metres) = conversion.value() else {
        panic!("an exact target converts exactly");
    };
    let converted = Value::Quantity(Quantity::new(metres.clone(), units.m.clone()));
    let one_metre = Value::Quantity(Quantity::new(whole(1), units.m.clone()));
    assert_eq!(equal(&converted, &one_metre), is(true));
    assert_eq!(centimetres, snapshot);
    assert_eq!(
        equal(&Value::Quantity(centimetres), &one_metre),
        ill_typed(IllTypedCause::DistinctUnits)
    );
    let one_second = Value::Quantity(Quantity::new(whole(1), units.s));
    assert_eq!(
        equal(&one_metre, &one_second),
        ill_typed(IllTypedCause::IncompatibleDimensions)
    );
    assert_disjoint(&one_metre);
}

#[trace("TC-194", "FR-149-AC-4")]
#[test]
fn e06_text_under_one_pinned_profile() {
    let composed = text("\u{e9}", TextProfile::Nfc);
    assert_eq!(
        equal(&composed, &text("e\u{301}", TextProfile::Nfc)),
        is(true)
    );
    assert_eq!(
        equal(&text("a", TextProfile::Nfc), &text("b", TextProfile::Nfc)),
        is(false)
    );
    assert_eq!(
        equal(&composed, &text("\u{e9}", TextProfile::BinaryUtf8)),
        ill_typed(IllTypedCause::DistinctTextProfiles)
    );
    assert_disjoint(&composed);
}

#[trace("TC-194", "FR-149-AC-3", "FR-149-AC-4")]
#[test]
fn e07_enumerations_by_declaration_node_and_case() {
    let a = enum_declaration("EnumA");
    let b = enum_declaration("EnumB");
    let ready = member(&a, "READY");
    assert_eq!(equal(&ready, &member(&a, "READY")), is(true));
    assert_eq!(equal(&ready, &member(&a, "DONE")), is(false));
    assert_eq!(
        equal(&ready, &member(&b, "READY")),
        ill_typed(IllTypedCause::DistinctEnumDeclarations)
    );
    assert_disjoint(&ready);
}

// ---- presence, composite and collection rows -------------------------------

#[trace("TC-194", "FR-149-AC-4", "FR-149-AC-6")]
#[test]
fn e08_options_compare_state_then_payload() {
    let none = || OptionValue::none(ValueType::Integer);
    let present = |value| OptionValue::present(ValueType::Integer, int(value)).unwrap();
    assert_eq!(equal(&none(), &none()), is(true));
    assert_eq!(equal(&present(1), &present(1)), is(true));
    assert_eq!(equal(&none(), &present(1)), is(false));
    assert_eq!(equal(&present(1), &present(2)), is(false));
    // SPEC-GAP(119-3): one pair per option state, plus the payload pair.
    assert_eq!(pairs(&none(), &none()), 1);
    assert_eq!(pairs(&present(1), &present(1)), 2);
    assert_eq!(pairs(&none(), &present(1)), 1);
    assert_eq!(
        equal(
            &present(1),
            &OptionValue::present(ValueType::Rational, rational(1, 1)).unwrap()
        ),
        ill_typed(IllTypedCause::DistinctValueTypes)
    );
    assert_disjoint(&none());
}

#[trace("TC-194", "FR-149-AC-4", "FR-149-AC-6")]
#[test]
fn e09_absence_and_null_stay_distinct() {
    let env = TypeEnvironment::new([CompositeDeclaration::new(
        key("Holder"),
        CompositeShape::Record(vec![FieldDeclaration::new(
            key("value"),
            ValueType::Integer,
            Presence::OptionalNullable,
        )]),
    )])
    .unwrap();
    let holder = |state| {
        env.record(key("Holder"), vec![(key("value"), state)])
            .unwrap()
    };
    let null = holder(FieldValue::Null);
    assert_eq!(equal(&null, &holder(FieldValue::Null)), is(true));
    assert_eq!(
        equal(&holder(FieldValue::Absent), &holder(FieldValue::Absent)),
        is(true)
    );
    assert_eq!(equal(&holder(FieldValue::Absent), &null), is(false));
    assert_eq!(
        equal(&holder(FieldValue::Present(int(1))), &null),
        is(false)
    );
    assert_eq!(
        equal(
            &holder(FieldValue::Present(int(1))),
            &holder(FieldValue::Present(int(1)))
        ),
        is(true)
    );
    assert_eq!(pairs(&null, &holder(FieldValue::Absent)), 2);
    assert_disjoint(&null);
}

fn record_environment() -> TypeEnvironment {
    let inner = CompositeShape::Record(vec![FieldDeclaration::new(
        key("v"),
        ValueType::Integer,
        Presence::Required,
    )]);
    let outer = |inner_key: &str| {
        CompositeShape::Record(vec![
            FieldDeclaration::new(key("id"), ValueType::Integer, Presence::Required),
            FieldDeclaration::new(
                key("inner"),
                ValueType::Composite(key(inner_key)),
                Presence::Required,
            ),
        ])
    };
    let pair = || CompositeShape::Tuple(vec![ValueType::Integer, ValueType::Boolean]);
    TypeEnvironment::new([
        CompositeDeclaration::new(key("Inner"), inner),
        CompositeDeclaration::new(key("record-A"), outer("Inner")),
        CompositeDeclaration::new(key("record-B"), outer("Inner")),
        CompositeDeclaration::new(key("tuple-A"), pair()),
        CompositeDeclaration::new(key("tuple-B"), pair()),
    ])
    .unwrap()
}

fn nested(env: &TypeEnvironment, declaration: &str, id: i64, v: i64) -> Value {
    let inner = env
        .record(key("Inner"), vec![(key("v"), FieldValue::Present(int(v)))])
        .unwrap();
    env.record(
        key(declaration),
        vec![
            (key("id"), FieldValue::Present(int(id))),
            (key("inner"), FieldValue::Present(inner)),
        ],
    )
    .unwrap()
}

#[trace("TC-194", "FR-149-AC-2", "FR-149-AC-4")]
#[test]
fn e10_records_compare_structurally_within_one_declaration() {
    let env = record_environment();
    let left = nested(&env, "record-A", 1, 2);
    assert_eq!(equal(&left, &nested(&env, "record-A", 1, 2)), is(true));
    assert_eq!(equal(&left, &nested(&env, "record-A", 1, 3)), is(false));
    assert_eq!(
        equal(&left, &nested(&env, "record-B", 1, 2)),
        ill_typed(IllTypedCause::DistinctDeclarations)
    );
    // `$`, `$.id`, `$.inner`, `$.inner.v`.
    assert_eq!(pairs(&left, &nested(&env, "record-A", 9, 9)), 4);
    assert_disjoint(&left);
}

#[trace("TC-194", "FR-149-AC-4")]
#[test]
fn e11_tuples_compare_positionally_within_one_declaration() {
    let env = record_environment();
    let tuple = |declaration: &str, first, second| {
        env.tuple(key(declaration), vec![int(first), Value::Boolean(second)])
            .unwrap()
    };
    let left = tuple("tuple-A", 1, true);
    assert_eq!(equal(&left, &tuple("tuple-A", 1, true)), is(true));
    assert_eq!(equal(&left, &tuple("tuple-A", 1, false)), is(false));
    assert_eq!(
        equal(&left, &tuple("tuple-B", 1, true)),
        ill_typed(IllTypedCause::DistinctDeclarations)
    );
    assert_disjoint(&left);
}

fn assert_wrong_collection_kind(left: &Value, elements: &[i64]) {
    for kind in [
        CollectionKind::Sequence,
        CollectionKind::Set,
        CollectionKind::Bag,
        CollectionKind::OrderedSet,
    ] {
        let other = collection(kind, elements);
        if pairs_kind(left) != Some(kind) {
            assert_eq!(
                equal(left, &other),
                ill_typed(IllTypedCause::DistinctValueTypes)
            );
        }
    }
    let rationals = CollectionValue::construct(
        pairs_kind(left).unwrap(),
        ValueType::Rational,
        vec![rational(1, 1)],
    )
    .unwrap();
    assert_eq!(
        equal(left, &rationals),
        ill_typed(IllTypedCause::DistinctValueTypes)
    );
    assert_disjoint(left);
}

fn pairs_kind(value: &Value) -> Option<CollectionKind> {
    match value {
        Value::Collection(collection) => Some(collection.kind()),
        _ => None,
    }
}

#[trace("TC-194", "FR-149-AC-4")]
#[test]
fn e12_sequences_compare_by_index() {
    let left = collection(CollectionKind::Sequence, &[1, 2]);
    assert_eq!(
        equal(&left, &collection(CollectionKind::Sequence, &[1, 2])),
        is(true)
    );
    assert_eq!(
        equal(&left, &collection(CollectionKind::Sequence, &[2, 1])),
        is(false)
    );
    assert_eq!(
        equal(&left, &collection(CollectionKind::Sequence, &[1, 2, 2])),
        is(false)
    );
    assert_wrong_collection_kind(&left, &[1, 2]);
}

#[trace("TC-194", "FR-149-AC-4", "FR-149-AC-8")]
#[test]
fn e13_sets_ignore_insertion_order() {
    let left = collection(CollectionKind::Set, &[1, 2]);
    assert_eq!(
        equal(&left, &collection(CollectionKind::Set, &[2, 1])),
        is(true)
    );
    assert_eq!(
        equal(&left, &collection(CollectionKind::Set, &[1, 3])),
        is(false)
    );
    assert_eq!(
        equal(&left, &collection(CollectionKind::Set, &[2, 1, 2])),
        is(true)
    );
    assert_wrong_collection_kind(&left, &[1, 2]);
}

#[trace("TC-194", "FR-149-AC-4", "FR-149-AC-8")]
#[test]
fn e14_bags_compare_multiplicity() {
    let left = collection(CollectionKind::Bag, &[1, 1, 2]);
    assert_eq!(
        equal(&left, &collection(CollectionKind::Bag, &[2, 1, 1])),
        is(true)
    );
    assert_eq!(
        equal(&left, &collection(CollectionKind::Bag, &[1, 2, 2])),
        is(false)
    );
    assert_eq!(
        equal(&left, &collection(CollectionKind::Bag, &[1, 2])),
        is(false)
    );
    assert_wrong_collection_kind(&left, &[1, 1, 2]);
}

#[trace("TC-194", "FR-149-AC-4")]
#[test]
fn e15_ordered_sets_normalize_to_first_occurrence() {
    let left = collection(CollectionKind::OrderedSet, &[1, 2, 1]);
    assert_eq!(
        equal(&left, &collection(CollectionKind::OrderedSet, &[1, 2])),
        is(true)
    );
    assert_eq!(
        equal(&left, &collection(CollectionKind::OrderedSet, &[2, 1])),
        is(false)
    );
    assert_wrong_collection_kind(&left, &[1, 2]);
}

#[trace("TC-194", "FR-149-AC-2", "FR-149-AC-4")]
#[test]
fn e16_references_compare_qualified_identity_only() {
    let env = TypeEnvironment::new([
        CompositeDeclaration::new(
            key("object-A"),
            CompositeShape::Record(vec![FieldDeclaration::new(
                key("balance"),
                ValueType::Integer,
                Presence::Required,
            )]),
        ),
        CompositeDeclaration::new(key("object-B"), CompositeShape::Tuple(vec![])),
    ])
    .unwrap();
    let identity = |name: &str| ObjectIdentity::new(vec!["accounts".into(), name.into()]).unwrap();
    let reference =
        |declaration: &str, name: &str| ObjectReference::new(key(declaration), identity(name));
    let state = |balance| {
        env.record(
            key("object-A"),
            vec![(key("balance"), FieldValue::Present(int(balance)))],
        )
        .unwrap()
    };
    let before = ObjectEnvironment::new([(reference("object-A", "a"), state(1))]).unwrap();
    let after = ObjectEnvironment::new([
        (reference("object-A", "a"), state(2)),
        (reference("object-A", "b"), state(1)),
    ])
    .unwrap();
    let a = reference("object-A", "a");
    assert!(before.resolve(&a).is_some() && after.resolve(&a).is_some());
    let left = Value::Reference(a.clone());
    assert_eq!(equal(&left, &Value::Reference(a)), is(true));
    assert_eq!(
        equal(&left, &Value::Reference(reference("object-A", "b"))),
        is(false)
    );
    assert_eq!(pairs(&left, &left), 1);
    assert_eq!(
        equal(&left, &Value::Reference(reference("object-B", "a"))),
        ill_typed(IllTypedCause::DistinctDeclarations)
    );
    assert_disjoint(&left);
}

// ---- dispositions and accounting -------------------------------------------

#[trace("TC-194", "FR-149-AC-7")]
#[test]
fn e20_nested_construction_dispositions_propagate() {
    let env = record_environment();
    let incomplete = Incomplete {
        limit_kind: LimitKind::WorkUnits,
        limit: 0,
        consumed: 0,
        next_charge: Integer::from(1_i64),
        charge_point: ChargePoint::DecimalArithmetic,
    };
    let stopped: [Outcome<Value>; 3] = [
        Outcome::Undefined(Undefined::DivisionByZero),
        Outcome::Refused(Refusal::InexactDecimal),
        Outcome::Incomplete(incomplete),
    ];
    for nested in stopped {
        let expected: Outcome<Value> = match &nested {
            Outcome::Undefined(reason) => Outcome::Undefined(*reason),
            Outcome::Refused(reason) => Outcome::Refused(*reason),
            Outcome::Incomplete(record) => Outcome::Incomplete(record.clone()),
            Outcome::Completed(_) => unreachable!(),
        };
        for _outer in 0..2 {
            let outcome = env
                .evaluate_record(
                    key("Inner"),
                    vec![(key("v"), FieldExpression::Evaluated(nested.clone()))],
                )
                .unwrap();
            assert_eq!(format!("{outcome:?}"), format!("{expected:?}"));
            assert!(outcome.completed().is_none());
        }
    }
    let completed = env
        .evaluate_record(
            key("Inner"),
            vec![(
                key("v"),
                FieldExpression::Evaluated(Outcome::Completed(int(1))),
            )],
        )
        .unwrap()
        .completed()
        .unwrap();
    assert_eq!(equal(&completed, &nested_inner(&env, 1)), is(true));
}

fn nested_inner(env: &TypeEnvironment, v: i64) -> Value {
    env.record(key("Inner"), vec![(key("v"), FieldValue::Present(int(v)))])
        .unwrap()
}

fn list_environment() -> TypeEnvironment {
    TypeEnvironment::new([CompositeDeclaration::new(
        key("List"),
        CompositeShape::Variant(vec![
            ConstructorDeclaration::new(key("Nil"), vec![]),
            ConstructorDeclaration::new(
                key("Cons"),
                vec![
                    FieldDeclaration::new(key("head"), ValueType::Integer, Presence::Required),
                    FieldDeclaration::new(
                        key("tail"),
                        ValueType::Composite(key("List")),
                        Presence::Required,
                    ),
                ],
            ),
        ]),
    )])
    .unwrap()
}

fn nil(env: &TypeEnvironment) -> Value {
    env.variant(key("List"), key("Nil"), vec![]).unwrap()
}

fn cons(env: &TypeEnvironment, head: i64, tail: Value) -> Value {
    env.variant(
        key("List"),
        key("Cons"),
        vec![
            (key("head"), FieldValue::Present(int(head))),
            (key("tail"), FieldValue::Present(tail)),
        ],
    )
    .unwrap()
}

/// `[from..=8]` ending in `tail`.
fn list_onto(env: &TypeEnvironment, from: i64, tail: Value) -> Value {
    (from..=8)
        .rev()
        .fold(tail, |tail, head| cons(env, head, tail))
}

const E21: ScalarLimits = ScalarLimits {
    integer_bits: 4,
    value_occurrences: 17,
    work_units: 19,
    result_units: 1,
    ..ZERO
};

fn e21_operands(env: &TypeEnvironment) -> [(Value, Value); 2] {
    let duplicated = (list_onto(env, 1, nil(env)), list_onto(env, 1, nil(env)));
    // Both operands share one immutable `[5..=8]` tail.
    let shared_tail = list_onto(env, 5, nil(env));
    let prefix = |tail: Value| (1..=4).rev().fold(tail, |tail, head| cons(env, head, tail));
    let shared = (prefix(shared_tail.clone()), prefix(shared_tail));
    [duplicated, shared]
}

#[trace("TC-194", "FR-149-AC-7")]
#[test]
fn e21_duplicated_and_shared_lists_charge_seventeen_pairs() {
    let env = list_environment();
    let mut expected_points = vec![ChargePoint::EqualityPlan];
    expected_points.extend([ChargePoint::EqualityPair; 17]);
    expected_points.push(ChargePoint::EqualityResultRetain);
    for (left, right) in e21_operands(&env) {
        assert_eq!(pairs(&left, &right), 17);
        let mut meter = Meter::new(E21);
        assert_eq!(evaluate_equality(&left, &right, &mut meter), is(true));
        assert_eq!(meter.admitted_charges(), expected_points.as_slice());
        assert_eq!(meter.consumed(LimitKind::WorkUnits), 19);
        assert_eq!(meter.consumed(LimitKind::ResultUnits), 1);
        assert_eq!(meter.consumed(LimitKind::ValueOccurrences), 17);
        assert_eq!(meter.consumed(LimitKind::IntegerBits), 0);

        let denied = |point, occurrence| {
            Meter::new(E21).with_injected_denial(InjectedDenial { point, occurrence })
        };
        assert_eq!(
            evaluate_equality(&left, &right, &mut denied(ChargePoint::EqualityPair, 17)),
            Ok(Outcome::Incomplete(Incomplete {
                limit_kind: LimitKind::WorkUnits,
                limit: 17,
                consumed: 17,
                next_charge: Integer::from(1_i64),
                charge_point: ChargePoint::EqualityPair,
            }))
        );
        assert_eq!(
            evaluate_equality(
                &left,
                &right,
                &mut denied(ChargePoint::EqualityResultRetain, 1)
            ),
            Ok(Outcome::Incomplete(Incomplete {
                limit_kind: LimitKind::WorkUnits,
                limit: 18,
                consumed: 18,
                next_charge: Integer::from(1_i64),
                charge_point: ChargePoint::EqualityResultRetain,
            }))
        );
    }
}

#[trace("TC-194", "FR-149-AC-8")]
#[test]
fn e22_unkeyed_sets_and_bags_use_the_full_cross_product() {
    let env = record_environment();
    // Record elements: equality without a total canonical serialization key.
    let element = |v| nested_inner(&env, v);
    let of = |kind, values: &[i64]| {
        CollectionValue::construct(
            kind,
            ValueType::Composite(key("Inner")),
            values.iter().map(|v| element(*v)).collect(),
        )
        .unwrap()
    };
    let run = |left: &Value, right: &Value| {
        let mut meter = Meter::new(UNLIMITED);
        let outcome = evaluate_equality(left, right, &mut meter);
        (outcome, meter.consumed(LimitKind::WorkUnits))
    };
    // 1 + 2 × 2 element pairs × 2 (`$[i,j]`, `$[i,j].v`) = 9 pairs, 11 work.
    let set = of(CollectionKind::Set, &[1, 2]);
    for (right, expected) in [(&[2, 1], true), (&[1, 2], true), (&[1, 3], false)] {
        let right = of(CollectionKind::Set, right);
        assert_eq!(pairs(&set, &right), 9);
        assert_eq!(run(&set, &right), (is(expected), 11));
        assert_eq!(run(&right, &set), (is(expected), 11));
    }
    // 1 + 3 × 3 × 2 = 19 pairs, 21 work, for every insertion order.
    let bag = of(CollectionKind::Bag, &[1, 1, 2]);
    for (right, expected) in [
        (&[2, 1, 1], true),
        (&[1, 2, 1], true),
        (&[1, 2, 2], false),
        (&[2, 2, 2], false),
    ] {
        let right = of(CollectionKind::Bag, right);
        assert_eq!(pairs(&bag, &right), 19);
        assert_eq!(run(&bag, &right), (is(expected), 21));
        assert_eq!(run(&right, &bag), (is(expected), 21));
    }
}

#[trace("TC-194", "FR-149-AC-7")]
#[test]
fn e23_plan_reservation_refuses_before_any_counter_changes() {
    let env = list_environment();
    let [(left, right), _] = e21_operands(&env);
    let mut meter = Meter::new(ScalarLimits {
        work_units: 18,
        ..E21
    });
    assert_eq!(
        evaluate_equality(&left, &right, &mut meter),
        Ok(Outcome::Incomplete(Incomplete {
            limit_kind: LimitKind::WorkUnits,
            limit: 18,
            consumed: 0,
            next_charge: Integer::from(19_i64),
            charge_point: ChargePoint::EqualityPlan,
        }))
    );
    assert!(meter.admitted_charges().is_empty());
    for kind in LimitKind::ALL {
        assert_eq!(meter.consumed(kind), 0, "{kind:?}");
    }
}

#[trace("TC-194", "FR-149-AC-7")]
#[test]
fn e24_quantity_leaf_charges_only_its_pair() {
    let units = units();
    let env = TypeEnvironment::new([CompositeDeclaration::new(
        key("Length"),
        CompositeShape::Record(vec![FieldDeclaration::new(
            key("d"),
            ValueType::Quantity(units.cm.clone()),
            Presence::Required,
        )]),
    )])
    .unwrap();
    let record = || {
        let d = Quantity::new(
            Rational::from_integer(Integer::from(1_i64)),
            units.cm.clone(),
        );
        env.record(
            key("Length"),
            vec![(key("d"), FieldValue::Present(Value::Quantity(d)))],
        )
        .unwrap()
    };
    let limits = ScalarLimits {
        value_occurrences: 2,
        work_units: 4,
        result_units: 1,
        ..ZERO
    };
    let mut meter = Meter::new(limits);
    assert_eq!(
        evaluate_equality(&record(), &record(), &mut meter),
        is(true)
    );
    assert_eq!(
        meter.admitted_charges(),
        [
            ChargePoint::EqualityPlan,
            ChargePoint::EqualityPair,
            ChargePoint::EqualityPair,
            ChargePoint::EqualityResultRetain,
        ]
    );
    assert_eq!(meter.consumed(LimitKind::WorkUnits), 4);
    assert_eq!(meter.consumed(LimitKind::ResultUnits), 1);

    let mut short = Meter::new(ScalarLimits {
        work_units: 3,
        ..limits
    });
    assert_eq!(
        evaluate_equality(&record(), &record(), &mut short),
        Ok(Outcome::Incomplete(Incomplete {
            limit_kind: LimitKind::WorkUnits,
            limit: 3,
            consumed: 0,
            next_charge: Integer::from(4_i64),
            charge_point: ChargePoint::EqualityPlan,
        }))
    );
    assert!(short.admitted_charges().is_empty());
}
