// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-189 collection kind algebra over the real `value` boundary (FR-144).
//!
//! TC-189 names a generic procedure with no vectors; these cases construct,
//! permute, compare and overflow every collection kind.

use ix_trace_rs::trace;
use quire_spec_language::value::{
    count, evaluate_equality, CardinalityBound, CardinalityViolation, CollectionKind,
    CollectionValue, Component, ConstructionCause, ConstructionRefusal, EmptyCardinalityBound,
    Integer, Meter, NoTotalElementKey, OptionValue, Outcome, ScalarLimits, Value, ValueType,
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

const KINDS: [CollectionKind; 4] = CollectionKind::ALL;

fn int(value: i64) -> Value {
    Value::Integer(Integer::from(value))
}

fn integer(value: &Value) -> i64 {
    match value {
        Value::Integer(value) => value.to_string().parse().unwrap(),
        other => panic!("not an integer: {other:?}"),
    }
}

fn bound(minimum: u64, maximum: u64) -> Option<CardinalityBound> {
    Some(CardinalityBound::new(minimum, maximum).unwrap())
}

fn build(
    kind: CollectionKind,
    bound: Option<CardinalityBound>,
    values: &[i64],
) -> Result<Value, ConstructionRefusal> {
    CollectionValue::construct(
        kind,
        ValueType::Integer,
        bound,
        values.iter().copied().map(int).collect(),
    )
}

fn collection(kind: CollectionKind, values: &[i64]) -> Value {
    build(kind, bound(0, 16), values).unwrap()
}

fn members(value: &Value) -> &CollectionValue {
    match value {
        Value::Collection(collection) => collection,
        other => panic!("not a collection: {other:?}"),
    }
}

fn elements(value: &Value) -> Vec<i64> {
    members(value).elements().iter().map(integer).collect()
}

fn equal(left: &Value, right: &Value) -> bool {
    match evaluate_equality(left, right, &mut Meter::new(UNLIMITED)) {
        Ok(Outcome::Completed(equal)) => equal,
        other => panic!("equality did not complete: {other:?}"),
    }
}

/// Canonical `(member, multiplicity)` entries.
fn canonical(value: &Value) -> Result<Vec<(i64, u64)>, NoTotalElementKey> {
    Ok(members(value)
        .canonical_form()?
        .entries()
        .iter()
        .map(|entry| {
            (
                integer(entry.value()),
                entry.multiplicity().to_u64().unwrap(),
            )
        })
        .collect())
}

fn assert_refused(actual: Result<Value, ConstructionRefusal>, expected: CardinalityViolation) {
    assert_eq!(
        actual.map(|_| ()),
        Err(ConstructionRefusal {
            component: Component::Value,
            cause: ConstructionCause::Cardinality(expected),
        })
    );
}

#[trace("TC-189", "FR-144-AC-1")]
#[test]
fn permutation_changes_only_sequence_and_ordered_set_equality() {
    for kind in KINDS {
        let left = collection(kind, &[1, 2, 3, 2]);
        let permuted = collection(kind, &[2, 3, 2, 1]);
        assert_eq!(equal(&left, &permuted), !kind.is_ordered(), "{kind:?}");
        assert!(equal(&left, &collection(kind, &[1, 2, 3, 2])), "{kind:?}");
    }
    // A bag permutation keeps multiplicity; a different multiplicity is unequal.
    assert!(!equal(
        &collection(CollectionKind::Bag, &[1, 2, 2]),
        &collection(CollectionKind::Bag, &[1, 1, 2]),
    ));
}

#[trace("TC-189", "FR-144-AC-2")]
#[test]
fn duplicates_are_kept_by_sequence_and_bag_and_coalesced_by_sets() {
    let input = [2, 1, 2, 3, 1];
    let sequence = collection(CollectionKind::Sequence, &input);
    assert_eq!(elements(&sequence), input);

    let bag = collection(CollectionKind::Bag, &input);
    let multiplicity = |value| count(members(&bag), &int(value)).unwrap();
    assert_eq!(
        [multiplicity(1), multiplicity(2), multiplicity(3)],
        [2_i64, 2, 1].map(Integer::from)
    );

    // Ordered sets keep first-occurrence order; sets keep the same members
    // with no semantic order.
    let ordered = collection(CollectionKind::OrderedSet, &input);
    assert_eq!(elements(&ordered), [2, 1, 3]);
    let set = collection(CollectionKind::Set, &input);
    assert_eq!(canonical(&set), Ok(vec![(1, 1), (2, 1), (3, 1)]));
    assert!(equal(&set, &collection(CollectionKind::Set, &[3, 2, 1])));
}

#[trace("TC-189", "FR-144-AC-3")]
#[test]
fn missing_or_violated_bounds_refuse_without_a_collection() {
    assert_eq!(
        CardinalityBound::new(3, 2),
        Err(EmptyCardinalityBound {
            minimum: 3,
            maximum: 2
        })
    );
    for kind in KINDS {
        assert_refused(build(kind, None, &[1]), CardinalityViolation::Missing);
        assert_refused(
            build(kind, bound(0, 2), &[1, 2, 3]),
            CardinalityViolation::AboveMaximum { maximum: 2 },
        );
        assert_refused(
            build(kind, bound(2, 4), &[1]),
            CardinalityViolation::BelowMinimum {
                minimum: 2,
                counted: 1,
            },
        );
        // An empty input still needs a declared bound; `0..0` admits it.
        assert_refused(build(kind, None, &[]), CardinalityViolation::Missing);
        assert!(elements(&build(kind, bound(0, 0), &[]).unwrap()).is_empty());
        // Inclusive limits admit exactly `minimum` and `maximum`.
        assert_eq!(
            elements(&build(kind, bound(3, 3), &[1, 2, 3]).unwrap()),
            [1, 2, 3]
        );
    }
    // A type mismatch is reported at its element before any bound check.
    assert_eq!(
        CollectionValue::construct(
            CollectionKind::Sequence,
            ValueType::Integer,
            None,
            vec![int(1), Value::Boolean(true)],
        )
        .map(|_| ()),
        Err(ConstructionRefusal {
            component: Component::Element(1),
            cause: ConstructionCause::TypeMismatch,
        })
    );
    // A refusal leaves an independent sibling construction unaffected.
    assert_eq!(elements(&collection(CollectionKind::Bag, &[1, 1])), [1, 1]);
}

#[trace("TC-189", "FR-144-AC-4")]
#[test]
fn insertion_order_never_reaches_unordered_canonical_form() {
    let insertions: [&[i64]; 3] = [&[3, 1, 2, 1], &[1, 1, 2, 3], &[2, 1, 3, 1]];
    for kind in [CollectionKind::Set, CollectionKind::Bag] {
        let expected = match kind {
            CollectionKind::Set => vec![(1, 1), (2, 1), (3, 1)],
            _ => vec![(1, 2), (2, 1), (3, 1)],
        };
        for insertion in insertions {
            let value = collection(kind, insertion);
            assert_eq!(canonical(&value), Ok(expected.clone()), "{kind:?}");
            assert!(equal(&value, &collection(kind, insertions[0])), "{kind:?}");
        }
    }
    // Ordered kinds are canonical in occurrence order.
    assert_eq!(
        canonical(&collection(CollectionKind::Sequence, &[3, 1, 3])),
        Ok(vec![(3, 1), (1, 1), (3, 1)])
    );

    // `Option<Integer>` supplies no total key: set and bag canonical forms
    // refuse, while the ordered kinds still have one.
    let option_type = ValueType::Option(Box::new(ValueType::Integer));
    let options = || {
        vec![
            OptionValue::present(ValueType::Integer, int(1)).unwrap(),
            OptionValue::none(ValueType::Integer),
        ]
    };
    for kind in KINDS {
        let value =
            CollectionValue::construct(kind, option_type.clone(), bound(0, 4), options()).unwrap();
        let form = members(&value).canonical_form();
        if kind.is_ordered() {
            assert_eq!(
                form.map(|form| form.entries().len()).ok(),
                Some(2),
                "{kind:?}"
            );
        } else {
            assert!(matches!(form, Err(NoTotalElementKey)), "{kind:?}");
        }
    }
}

#[trace("TC-189", "FR-144-AC-5")]
#[test]
fn bounds_count_occurrences_or_unique_members_per_kind() {
    let input = [7, 7, 7];
    for kind in KINDS {
        let at_most_one = build(kind, bound(0, 1), &input);
        let at_least_two = build(kind, bound(2, 3), &input);
        if kind.is_unique() {
            // One member, whatever the occurrence count.
            assert_eq!(elements(&at_most_one.unwrap()), [7], "{kind:?}");
            assert_refused(
                at_least_two,
                CardinalityViolation::BelowMinimum {
                    minimum: 2,
                    counted: 1,
                },
            );
        } else {
            // Three occurrences.
            assert_refused(
                at_most_one,
                CardinalityViolation::AboveMaximum { maximum: 1 },
            );
            assert_eq!(elements(&at_least_two.unwrap()), input, "{kind:?}");
        }
    }
}
