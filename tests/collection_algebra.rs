// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-189 collection kind algebra over the real `value` boundary (FR-144).
//!
//! C05, C06's query rows, C11's `convert` row and C12 need the FR-146 query
//! and literal-typing evaluator (`Remaining work: #119`).

use ix_trace_rs::trace;
use quire_spec_language::value::{
    admit_text, construct_collection, form_collection, BoundViolation, CardinalityBound,
    ChargePoint, CollectionKind, CollectionType, CompositeDeclaration, CompositeShape, Deferred,
    EqualityOperand, EqualityOperator, FieldDeclaration, FieldValue, IeeeWidth, IllTyped,
    IllTypedCause, Incomplete, Integer, LimitKind, Meter, NodeKey, ObjectIdentity, ObjectReference,
    ObjectTypeDeclaration, OptionValue, Outcome, Presence, Refusal, ScalarLimits, TextPayload,
    TextProfile, TextType, TypeEnvironment, UniverseIdentity, Value, ValueType,
};
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

fn key(label: &str) -> NodeKey {
    let digest: String = Sha256::digest(label.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    NodeKey::from_hex(&digest).unwrap()
}

fn int(value: i64) -> Value {
    Value::Integer(Integer::from(value))
}

fn collection_type(
    kind: CollectionKind,
    element: ValueType,
    minimum: u64,
    maximum: u64,
) -> CollectionType {
    CollectionType::new(
        kind,
        element,
        CardinalityBound::new(minimum, maximum).unwrap(),
    )
}

fn integers(kind: CollectionKind, maximum: u64) -> CollectionType {
    collection_type(kind, ValueType::Integer, 0, maximum)
}

/// Evaluate the constructor `kind[values]` against `collection_type`.
fn construct(
    collection_type: &CollectionType,
    values: Vec<Value>,
    meter: &mut Meter,
) -> Outcome<Value> {
    let elements: Vec<Deferred<'_>> = values
        .into_iter()
        .map(|value| -> Deferred<'_> { Box::new(move |_: &mut Meter| Outcome::Completed(value)) })
        .collect();
    construct_collection(collection_type, elements, meter)
}

fn completed(collection_type: &CollectionType, values: Vec<Value>) -> Value {
    match construct(collection_type, values, &mut Meter::new(UNLIMITED)) {
        Outcome::Completed(value) => value,
        other => panic!("construction completes, not {other:?}"),
    }
}

fn elements(value: &Value) -> &[Value] {
    match value {
        Value::Collection(collection) => collection.elements(),
        other => panic!("a collection, not {other:?}"),
    }
}

fn values(values: &[i64]) -> Vec<Value> {
    values.iter().copied().map(int).collect()
}

fn equal(env: &TypeEnvironment, value_type: &ValueType, left: &Value, right: &Value) -> bool {
    let checked = env
        .check_equality(
            EqualityOperator::Equal,
            EqualityOperand::typed(value_type.clone()),
            EqualityOperand::typed(value_type.clone()),
        )
        .unwrap();
    match checked.evaluate(left, right, &mut Meter::new(UNLIMITED)) {
        Outcome::Completed(result) => result,
        other => panic!("equality completes, not {other:?}"),
    }
}

fn out_of_bound(
    violation: BoundViolation,
    collection_type: &CollectionType,
    count: u64,
) -> Refusal {
    Refusal::CardinalityOutOfBound {
        violation,
        kind: collection_type.kind(),
        bound: collection_type.bound(),
        count,
    }
}

fn incomplete(
    limit_kind: LimitKind,
    limit: u64,
    consumed: u64,
    next_charge: i64,
    charge_point: ChargePoint,
) -> Outcome<Value> {
    Outcome::Incomplete(Incomplete {
        limit_kind,
        limit,
        consumed,
        next_charge: Integer::from(next_charge),
        charge_point,
    })
}

/// `Debug` comparison: completed values carry no `PartialEq`.
fn assert_outcome(actual: &Outcome<Value>, expected: &Outcome<Value>) {
    assert_eq!(format!("{actual:?}"), format!("{expected:?}"));
}

#[trace("TC-189", "FR-144-AC-1")]
#[test]
fn c01_permutation_changes_only_sequence_and_ordered_set_equality() {
    let env = TypeEnvironment::default();
    for (kind, maximum, left, right, expected) in [
        (CollectionKind::Set, 2, vec![1, 2], vec![2, 1], true),
        (CollectionKind::Bag, 3, vec![1, 1, 2], vec![2, 1, 1], true),
        (CollectionKind::Sequence, 2, vec![1, 2], vec![2, 1], false),
        (CollectionKind::OrderedSet, 2, vec![1, 2], vec![2, 1], false),
    ] {
        let declared = integers(kind, maximum);
        let value_type = ValueType::collection(declared.clone());
        let left = completed(&declared, values(&left));
        let right = completed(&declared, values(&right));
        assert_eq!(
            equal(&env, &value_type, &left, &right),
            expected,
            "{kind:?}"
        );
    }
}

#[trace("TC-189", "FR-144-AC-2")]
#[test]
fn c02_duplicates_are_kept_by_sequence_and_bag_and_coalesced_by_sets() {
    let represented = |kind, occurrences: &[i64]| {
        let value = completed(&integers(kind, 3), values(occurrences));
        format!("{:?}", elements(&value))
    };
    let expect = |list: &[i64]| format!("{:?}", values(list));
    assert_eq!(
        represented(CollectionKind::Sequence, &[1, 1, 2]),
        expect(&[1, 1, 2])
    );
    assert_eq!(
        represented(CollectionKind::Set, &[1, 1, 2]),
        expect(&[1, 2])
    );
    // A bag lists each occurrence: `1` twice, `2` once.
    assert_eq!(
        represented(CollectionKind::Bag, &[1, 1, 2]),
        expect(&[1, 1, 2])
    );
    assert_eq!(
        represented(CollectionKind::OrderedSet, &[2, 1, 2]),
        expect(&[2, 1])
    );
}

#[trace("TC-189", "FR-144-AC-3")]
#[test]
fn c03_bound_violations_refuse_after_the_bound_charge() {
    let set = integers(CollectionKind::Set, 2);
    assert_eq!(elements(&completed(&set, values(&[1, 1, 2]))).len(), 2);

    let mut meter = Meter::new(UNLIMITED);
    assert_outcome(
        &construct(&set, values(&[1, 2, 3]), &mut meter),
        &Outcome::Refused(out_of_bound(BoundViolation::AboveMaximum, &set, 3)),
    );
    assert_eq!(
        meter.admitted_charges().last(),
        Some(&ChargePoint::CollectionBound)
    );
    assert!(!meter
        .admitted_charges()
        .contains(&ChargePoint::CollectionResultRetain));
    assert_eq!(meter.consumed(LimitKind::ResultUnits), 0);

    let bag = integers(CollectionKind::Bag, 2);
    assert_outcome(
        &construct(&bag, values(&[1, 1, 2]), &mut Meter::new(UNLIMITED)),
        &Outcome::Refused(out_of_bound(BoundViolation::AboveMaximum, &bag, 3)),
    );
    let sequence = collection_type(CollectionKind::Sequence, ValueType::Integer, 1, 3);
    let refusal = out_of_bound(BoundViolation::BelowMinimum, &sequence, 0);
    assert_outcome(
        &construct(&sequence, vec![], &mut Meter::new(UNLIMITED)),
        &Outcome::Refused(refusal),
    );
    assert_eq!(refusal.code(), Some("cardinality_out_of_bound"));
    assert_eq!(BoundViolation::BelowMinimum.as_str(), "below-minimum");
    assert_eq!(BoundViolation::AboveMaximum.as_str(), "above-maximum");
    assert!(CardinalityBound::new(3, 1).is_err());
}

#[trace("TC-189", "FR-144-AC-4")]
#[test]
fn c04_canonical_order_is_key_order() {
    let set = integers(CollectionKind::Set, 3);
    for source in [[3, 1, 2], [2, 3, 1]] {
        assert_eq!(
            format!("{:?}", elements(&completed(&set, values(&source)))),
            format!("{:?}", values(&[1, 2, 3]))
        );
    }

    let options = collection_type(
        CollectionKind::Set,
        ValueType::option(ValueType::Integer),
        0,
        2,
    );
    let none = OptionValue::none(ValueType::Integer);
    let one = OptionValue::present(ValueType::Integer, int(1)).unwrap();
    assert_eq!(
        format!(
            "{:?}",
            elements(&completed(&options, vec![one.clone(), none.clone()]))
        ),
        format!("{:?}", [none, one])
    );

    let text_type = TextType::new(0, 4, TextProfile::BinaryUtf8).unwrap();
    let text = |payload: &str| match admit_text(
        &TextPayload::from_utf8(payload.as_bytes()).unwrap(),
        &text_type,
        &mut Meter::new(UNLIMITED),
    ) {
        Outcome::Completed(text) => Value::Text(text),
        other => panic!("text admits, not {other:?}"),
    };
    let texts = collection_type(CollectionKind::Set, ValueType::Text(text_type), 0, 2);
    assert_eq!(
        format!(
            "{:?}",
            elements(&completed(&texts, vec![text("b"), text("a")]))
        ),
        format!("{:?}", [text("a"), text("b")])
    );
}

fn holder_environment() -> TypeEnvironment {
    TypeEnvironment::new(
        [
            CompositeDeclaration::new(
                key("Holder"),
                "Holder",
                CompositeShape::Record(vec![FieldDeclaration::new(
                    "r",
                    ValueType::Reference(key("M::Obj")),
                    Presence::Required,
                )]),
            ),
            CompositeDeclaration::new(
                key("F"),
                "F",
                CompositeShape::Record(vec![FieldDeclaration::new(
                    "x",
                    ValueType::Float(IeeeWidth::Binary32),
                    Presence::Required,
                )]),
            ),
        ],
        [ObjectTypeDeclaration::new(key("M::Obj"), "Obj", vec![])],
    )
    .unwrap()
}

fn holder(env: &TypeEnvironment, universe: &str, identity: &str) -> Value {
    let reference = ObjectReference::new(
        UniverseIdentity::new(universe.as_bytes()).unwrap(),
        key("M::Obj"),
        ObjectIdentity::new(identity.as_bytes()).unwrap(),
    );
    env.record(
        key("Holder"),
        vec![("r", FieldValue::Present(Value::Reference(reference)))],
    )
    .unwrap()
}

#[trace("TC-189", "FR-144-AC-4")]
#[trace("TC-189", "FR-144-AC-6")]
#[test]
fn c06_reference_holders_are_keyed_and_ieee_elements_are_ineligible() {
    let env = holder_environment();
    let holders = collection_type(
        CollectionKind::Set,
        ValueType::Composite(key("Holder")),
        0,
        2,
    );
    let (h1, h2) = (holder(&env, "u1", "h1"), holder(&env, "u1", "h2"));
    let hs = completed(&holders, vec![h2.clone(), h1.clone()]);
    assert_eq!(format!("{:?}", elements(&hs)), format!("{:?}", [h1, h2]));
    let hs_type = ValueType::collection(holders);
    assert!(env.check_type(&hs_type).is_ok());
    assert!(equal(&env, &hs_type, &hs, &hs));

    let ineligible = Err(IllTyped {
        cause: IllTypedCause::OperatorIneligible,
    });
    let floats = collection_type(
        CollectionKind::Set,
        ValueType::Float(IeeeWidth::Binary64),
        0,
        2,
    );
    let records = collection_type(CollectionKind::Bag, ValueType::Composite(key("F")), 0, 2);
    for declared in [floats, records] {
        assert_eq!(env.check_type(&ValueType::collection(declared)), ineligible);
    }
}

#[trace("TC-189", "FR-144-AC-7")]
#[test]
fn c07_set_construction_charges_membership_comparisons_in_retention_order() {
    let set = integers(CollectionKind::Set, 3);
    let mut meter = Meter::new(UNLIMITED);
    let outcome = construct(&set, values(&[1, 2, 1]), &mut meter);
    let Outcome::Completed(value) = outcome else {
        panic!("construction completes");
    };
    assert_eq!(
        format!("{:?}", elements(&value)),
        format!("{:?}", values(&[1, 2]))
    );
    assert_eq!(
        meter.admitted_charges(),
        [
            ChargePoint::CollectionElement,
            ChargePoint::CollectionElement,
            ChargePoint::CollectionElement,
            ChargePoint::CollectionMemberWalk,
            ChargePoint::CollectionMemberTest,
            ChargePoint::CollectionMemberWalk,
            ChargePoint::CollectionMemberTest,
            ChargePoint::CollectionBound,
            ChargePoint::CollectionResultRetain,
        ]
    );
    assert_eq!(meter.consumed(LimitKind::WorkUnits), 11);
    assert_eq!(meter.consumed(LimitKind::ResultUnits), 3);
    assert_eq!(meter.consumed(LimitKind::ValueOccurrences), 3);
}

#[trace("TC-189", "FR-144-AC-7")]
#[test]
fn c08_denied_construction_charges_expose_no_collection() {
    let set = integers(CollectionKind::Set, 3);
    for (limits, expected) in [
        (
            ScalarLimits {
                work_units: 4,
                ..UNLIMITED
            },
            incomplete(
                LimitKind::WorkUnits,
                4,
                3,
                2,
                ChargePoint::CollectionMemberWalk,
            ),
        ),
        (
            ScalarLimits {
                work_units: 10,
                ..UNLIMITED
            },
            incomplete(
                LimitKind::WorkUnits,
                10,
                10,
                1,
                ChargePoint::CollectionResultRetain,
            ),
        ),
        (
            ScalarLimits {
                value_occurrences: 2,
                ..UNLIMITED
            },
            incomplete(
                LimitKind::ValueOccurrences,
                2,
                2,
                3,
                ChargePoint::CollectionResultRetain,
            ),
        ),
    ] {
        assert_outcome(
            &construct(&set, values(&[1, 2, 1]), &mut Meter::new(limits)),
            &expected,
        );
    }
}

#[trace("TC-189", "FR-144-AC-7")]
#[test]
fn c09_sequence_and_rejected_set_charge_exact_work() {
    let sequence = integers(CollectionKind::Sequence, 3);
    let mut meter = Meter::new(UNLIMITED);
    assert!(matches!(
        construct(&sequence, values(&[1, 1]), &mut meter),
        Outcome::Completed(_)
    ));
    assert_eq!(meter.consumed(LimitKind::WorkUnits), 4);
    assert_eq!(meter.consumed(LimitKind::ResultUnits), 3);

    let set = integers(CollectionKind::Set, 1);
    let mut meter = Meter::new(UNLIMITED);
    assert_outcome(
        &construct(&set, values(&[1, 2]), &mut meter),
        &Outcome::Refused(out_of_bound(BoundViolation::AboveMaximum, &set, 2)),
    );
    assert_eq!(meter.consumed(LimitKind::WorkUnits), 6);
    assert_eq!(meter.consumed(LimitKind::ResultUnits), 0);
}

#[trace("TC-189", "FR-144-AC-6")]
#[trace("TC-189", "FR-144-AC-7")]
#[test]
fn c10_reference_holder_sets_charge_pairs_and_refuse_foreign_universes() {
    let env = holder_environment();
    let holders = collection_type(
        CollectionKind::Set,
        ValueType::Composite(key("Holder")),
        0,
        3,
    );
    let (h1, h2) = (holder(&env, "u1", "h1"), holder(&env, "u1", "h2"));
    let mut meter = Meter::new(UNLIMITED);
    let Outcome::Completed(value) = construct(
        &holders,
        vec![h1.clone(), h2.clone(), h1.clone()],
        &mut meter,
    ) else {
        panic!("construction completes");
    };
    assert_eq!(
        format!("{:?}", elements(&value)),
        format!("{:?}", [&h1, &h2])
    );
    assert_eq!(meter.consumed(LimitKind::WorkUnits), 17);
    assert_eq!(meter.consumed(LimitKind::ResultUnits), 5);

    let hx = holder(&env, "u2", "hx");
    let mut meter = Meter::new(UNLIMITED);
    assert_outcome(
        &construct(&holders, vec![h1, hx], &mut meter),
        &Outcome::Refused(Refusal::ForeignReference),
    );
    assert_eq!(meter.consumed(LimitKind::WorkUnits), 6);
    assert!(!meter
        .admitted_charges()
        .contains(&ChargePoint::CollectionMemberTest));
}

#[trace("TC-189", "FR-144-AC-3")]
#[test]
fn c11_the_bound_is_part_of_the_collection_type() {
    let env = TypeEnvironment::default();
    let x = ValueType::collection(integers(CollectionKind::Set, 2));
    let s = ValueType::collection(integers(CollectionKind::Set, 3));
    assert_eq!(
        env.check_equality(
            EqualityOperator::Equal,
            EqualityOperand::typed(x),
            EqualityOperand::typed(s),
        ),
        Err(IllTyped {
            cause: IllTypedCause::TypeMismatch
        })
    );
}

#[trace("TC-189", "FR-144-AC-2")]
#[test]
fn formed_occurrences_outside_the_element_type_refuse_at_their_index() {
    let set = integers(CollectionKind::Set, 3);
    let refused = form_collection(
        &set,
        vec![int(1), Value::Boolean(true)],
        &mut Meter::new(UNLIMITED),
    )
    .unwrap_err();
    assert_eq!(
        refused.component,
        quire_spec_language::value::Component::Element(1)
    );
}
