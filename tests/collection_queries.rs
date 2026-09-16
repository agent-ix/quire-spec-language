// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-190 collection query and conversion algebra over the real `value`
//! boundary (FR-145).
//!
//! TC-190 names a generic procedure with no vectors; these cases run
//! map/filter/flatten/count/fold/reduce and explicit conversions across every
//! collection kind.

use std::cell::Cell;

use ix_trace_rs::trace;
use quire_spec_language::value::{
    convert, count, filter, flatten, fold, map, reduce, AlgebraicProperties, CardinalityBound,
    CardinalityViolation, CollectionKind, CollectionLoss, CollectionValue, FoldFunction, IllTyped,
    IllTypedCause, Integer, Meter, Outcome, Refusal, ScalarLimits, Undefined, Value, ValueFunction,
    ValueType,
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

use CollectionKind::{Bag, OrderedSet, Sequence, Set};

// ---- fixtures --------------------------------------------------------------

fn int(value: i64) -> Value {
    Value::Integer(Integer::from(value))
}

fn integer(value: &Value) -> i64 {
    match value {
        Value::Integer(value) => value.to_string().parse().unwrap(),
        other => panic!("not an integer: {other:?}"),
    }
}

fn bound() -> Option<CardinalityBound> {
    Some(CardinalityBound::new(0, 16).unwrap())
}

fn meter() -> Meter {
    Meter::new(UNLIMITED)
}

fn collection(kind: CollectionKind, values: &[i64]) -> Value {
    CollectionValue::construct(
        kind,
        ValueType::Integer,
        bound(),
        values.iter().copied().map(int).collect(),
    )
    .unwrap()
}

fn nested(outer: CollectionKind, inner: CollectionKind, values: &[&[i64]]) -> Value {
    CollectionValue::construct(
        outer,
        ValueType::Collection(inner, Box::new(ValueType::Integer)),
        bound(),
        values
            .iter()
            .map(|inner_values| collection(inner, inner_values))
            .collect(),
    )
    .unwrap()
}

fn members(value: &Value) -> &CollectionValue {
    match value {
        Value::Collection(collection) => collection,
        other => panic!("not a collection: {other:?}"),
    }
}

fn completed(outcome: Result<Outcome<Value>, IllTyped>) -> Value {
    match outcome {
        Ok(Outcome::Completed(value)) => value,
        other => panic!("not completed: {other:?}"),
    }
}

/// The semantic occurrences of a result: occurrence order for ordered kinds,
/// canonical `(member, multiplicity)` entries flattened for unordered kinds.
fn semantic(value: &Value) -> (CollectionKind, Vec<i64>) {
    let collection = members(value);
    let occurrences = collection
        .canonical_form()
        .unwrap()
        .entries()
        .iter()
        .flat_map(|entry| {
            let times = entry.multiplicity().to_u64().unwrap();
            (0..times).map(|_| integer(entry.value()))
        })
        .collect();
    (collection.kind(), occurrences)
}

fn ill_typed(cause: IllTypedCause) -> IllTyped {
    IllTyped { cause }
}

struct Unary {
    parameter: ValueType,
    result: ValueType,
    body: fn(i64) -> Outcome<Value>,
}

impl ValueFunction for Unary {
    fn parameter_type(&self) -> &ValueType {
        &self.parameter
    }
    fn result_type(&self) -> &ValueType {
        &self.result
    }
    fn apply(&self, argument: &Value, _meter: &mut Meter) -> Outcome<Value> {
        (self.body)(integer(argument))
    }
}

fn unary(result: ValueType, body: fn(i64) -> Outcome<Value>) -> Unary {
    Unary {
        parameter: ValueType::Integer,
        result,
        body,
    }
}

fn parity() -> Unary {
    unary(ValueType::Integer, |x| {
        Outcome::Completed(int(x.rem_euclid(2)))
    })
}

fn is_odd() -> Unary {
    unary(ValueType::Boolean, |x| {
        Outcome::Completed(Value::Boolean(x.rem_euclid(2) == 1))
    })
}

struct Binary {
    properties: AlgebraicProperties,
    body: fn(i64, i64) -> i64,
    calls: Cell<u32>,
}

impl FoldFunction for Binary {
    fn accumulator_type(&self) -> &ValueType {
        &ValueType::Integer
    }
    fn element_type(&self) -> &ValueType {
        &ValueType::Integer
    }
    fn properties(&self) -> AlgebraicProperties {
        self.properties
    }
    fn apply(&self, accumulator: &Value, element: &Value, _meter: &mut Meter) -> Outcome<Value> {
        self.calls.set(self.calls.get() + 1);
        Outcome::Completed(int((self.body)(integer(accumulator), integer(element))))
    }
}

fn binary(commutative: bool, associative: bool, body: fn(i64, i64) -> i64) -> Binary {
    Binary {
        properties: AlgebraicProperties {
            commutative,
            associative,
        },
        body,
        calls: Cell::new(0),
    }
}

fn sum() -> Binary {
    binary(true, true, |a, b| a + b)
}

fn difference() -> Binary {
    binary(false, false, |a, b| a - b)
}

// ---- cases -----------------------------------------------------------------

#[trace("TC-190", "FR-145-AC-1", "FR-145-AC-4")]
#[test]
fn map_filter_and_count_follow_the_operation_matrix() {
    let input = [3, 1, 3, 2];
    // (kind, map parity, filter odd, count of 3)
    let rows: [(CollectionKind, Vec<i64>, Vec<i64>, i64); 4] = [
        (Sequence, vec![1, 1, 1, 0], vec![3, 1, 3], 2),
        (Set, vec![0, 1], vec![1, 3], 1),
        (Bag, vec![0, 1, 1, 1], vec![1, 3, 3], 2),
        (OrderedSet, vec![1, 0], vec![3, 1], 1),
    ];
    for (kind, mapped, filtered, threes) in rows {
        let source = collection(kind, &input);
        let source = members(&source);
        let result = completed(map(source, &parity(), bound(), &mut meter()));
        assert_eq!(semantic(&result), (kind, mapped), "map {kind:?}");
        let result = completed(filter(source, &is_odd(), bound(), &mut meter()));
        assert_eq!(semantic(&result), (kind, filtered), "filter {kind:?}");
        assert_eq!(count(source, &int(3)), Ok(Integer::from(threes)));
        assert_eq!(count(source, &int(4)), Ok(Integer::zero()));
    }
    let source = collection(Sequence, &input);
    let source = members(&source);
    // Ordered-set map keeps first-result order, not key order.
    let ordered = completed(map(
        members(&collection(OrderedSet, &[2, 3, 4])),
        &parity(),
        bound(),
        &mut meter(),
    ));
    assert_eq!(semantic(&ordered), (OrderedSet, vec![0, 1]));
    // Type errors, non-Boolean predicates and out-of-type results.
    let boolean_parameter = Unary {
        parameter: ValueType::Boolean,
        ..parity()
    };
    assert_eq!(
        map(source, &boolean_parameter, bound(), &mut meter()).err(),
        Some(ill_typed(IllTypedCause::FunctionParameterType))
    );
    assert_eq!(
        filter(source, &parity(), bound(), &mut meter()).err(),
        Some(ill_typed(IllTypedCause::PredicateNotBoolean))
    );
    assert_eq!(
        count(source, &Value::Boolean(true)),
        Err(ill_typed(IllTypedCause::FunctionParameterType))
    );
    let lying = unary(ValueType::Integer, |_| {
        Outcome::Completed(Value::Boolean(false))
    });
    assert!(matches!(
        map(source, &lying, bound(), &mut meter()),
        Ok(Outcome::Refused(Refusal::FunctionResultOutsideType))
    ));
    // A non-completed function outcome stops the operation unchanged.
    let undefined = unary(ValueType::Integer, |_| {
        Outcome::Undefined(Undefined::DivisionByZero)
    });
    assert!(matches!(
        map(source, &undefined, bound(), &mut meter()),
        Ok(Outcome::Undefined(Undefined::DivisionByZero))
    ));
    // The result bound counts result occurrences or members, and a missing
    // result bound refuses.
    let tight = Some(CardinalityBound::new(0, 2).unwrap());
    assert!(matches!(
        map(source, &parity(), tight, &mut meter()),
        Ok(Outcome::Refused(Refusal::Cardinality(
            CardinalityViolation::AboveMaximum { maximum: 2 }
        )))
    ));
    let set = collection(Set, &input);
    assert_eq!(
        semantic(&completed(map(
            members(&set),
            &parity(),
            tight,
            &mut meter()
        ))),
        (Set, vec![0, 1])
    );
    assert!(matches!(
        filter(source, &is_odd(), None, &mut meter()),
        Ok(Outcome::Refused(Refusal::Cardinality(
            CardinalityViolation::Missing
        )))
    ));
}

#[trace("TC-190", "FR-145-AC-1", "FR-145-AC-4")]
#[test]
fn flatten_combines_occurrences_per_outer_kind() {
    let rows = [
        (Sequence, Sequence, vec![1, 2, 2, 3]),
        (Set, Set, vec![1, 2, 3]),
        (Bag, Bag, vec![1, 2, 2, 3]),
        (Set, Sequence, vec![1, 2, 3]),
        (Bag, Set, vec![1, 2, 2, 3]),
        (OrderedSet, Sequence, vec![1, 2, 3]),
    ];
    for (outer, inner, expected) in rows {
        let source = nested(outer, inner, &[&[1, 2], &[2, 3]]);
        let result = completed(flatten(members(&source), bound()));
        assert_eq!(semantic(&result), (outer, expected), "{outer:?}<{inner:?}>");
    }
    // Ordered kinds traverse outer then inner order.
    let source = nested(OrderedSet, OrderedSet, &[&[3, 1], &[2, 3]]);
    let result = completed(flatten(members(&source), bound()));
    assert_eq!(semantic(&result), (OrderedSet, vec![3, 1, 2]));
    // flatMap is flatten(map(...)).
    let pairs = Unary {
        parameter: ValueType::Integer,
        result: ValueType::Collection(Sequence, Box::new(ValueType::Integer)),
        body: |x| Outcome::Completed(collection(Sequence, &[x, x])),
    };
    let mapped = completed(map(
        members(&collection(Sequence, &[1, 2])),
        &pairs,
        bound(),
        &mut meter(),
    ));
    let result = completed(flatten(members(&mapped), bound()));
    assert_eq!(semantic(&result), (Sequence, vec![1, 1, 2, 2]));
    // Undefined combinations and non-collection elements are ill-typed.
    for (outer, inner) in [(Sequence, Set), (OrderedSet, Bag)] {
        let source = nested(outer, inner, &[&[1]]);
        assert_eq!(
            flatten(members(&source), bound()).err(),
            Some(ill_typed(IllTypedCause::UnsupportedFlattenSource))
        );
    }
    assert_eq!(
        flatten(members(&collection(Sequence, &[1])), bound()).err(),
        Some(ill_typed(IllTypedCause::UnsupportedFlattenSource))
    );
    // The bound counts flattened occurrences before materialization.
    let source = nested(Sequence, Sequence, &[&[1, 2], &[2, 3]]);
    assert!(matches!(
        flatten(members(&source), Some(CardinalityBound::new(0, 3).unwrap())),
        Ok(Outcome::Refused(Refusal::Cardinality(
            CardinalityViolation::AboveMaximum { maximum: 3 }
        )))
    ));
}

#[trace("TC-190", "FR-145-AC-2", "FR-145-AC-4")]
#[test]
fn empty_fold_returns_only_its_identity_and_empty_reduce_has_no_value() {
    for kind in CollectionKind::ALL {
        let empty = collection(kind, &[]);
        let function = sum();
        let result = completed(fold(members(&empty), &function, int(42), &mut meter()));
        assert_eq!(
            (integer(&result), function.calls.get()),
            (42, 0),
            "{kind:?}"
        );

        let outcome = reduce(members(&empty), &sum(), &mut meter());
        match kind {
            Sequence | Set => assert!(
                matches!(outcome, Ok(Outcome::Undefined(Undefined::EmptyReduction))),
                "{kind:?}"
            ),
            Bag | OrderedSet => assert!(
                matches!(outcome, Ok(Outcome::Refused(Refusal::EmptyReduction))),
                "{kind:?}"
            ),
        }

        // Non-empty folds apply once per retained occurrence.
        let source = collection(kind, &[2, 5, 2]);
        let function = sum();
        let result = completed(fold(members(&source), &function, int(0), &mut meter()));
        let (total, calls) = if kind.is_unique() { (7, 2) } else { (9, 3) };
        assert_eq!((integer(&result), function.calls.get()), (total, calls));
        let function = sum();
        let result = completed(reduce(members(&source), &function, &mut meter()));
        assert_eq!((integer(&result), function.calls.get()), (total, calls - 1));
    }
    let source = collection(Sequence, &[1]);
    assert_eq!(
        fold(members(&source), &sum(), Value::Boolean(true), &mut meter()).err(),
        Some(ill_typed(IllTypedCause::FoldIdentityType))
    );
}

#[trace("TC-190", "FR-145-AC-3", "FR-145-AC-4")]
#[test]
fn lossy_conversions_need_explicit_acceptance_and_record_the_exact_loss() {
    let loss = |order, uniqueness, multiplicity| CollectionLoss {
        order,
        uniqueness,
        multiplicity,
    };
    // (source, target, exact loss, converted semantic occurrences of [3,1,3,2])
    let rows = [
        (
            Sequence,
            Sequence,
            loss(false, false, false),
            vec![3, 1, 3, 2],
        ),
        (Sequence, Set, loss(true, false, true), vec![1, 2, 3]),
        (Sequence, Bag, loss(true, false, false), vec![1, 2, 3, 3]),
        (
            Sequence,
            OrderedSet,
            loss(false, false, true),
            vec![3, 1, 2],
        ),
        (Set, Sequence, loss(false, true, false), vec![1, 2, 3]),
        (Set, Bag, loss(false, true, false), vec![1, 2, 3]),
        (Set, OrderedSet, loss(false, false, false), vec![1, 2, 3]),
        (Bag, Sequence, loss(false, false, false), vec![1, 2, 3, 3]),
        (Bag, Set, loss(false, false, true), vec![1, 2, 3]),
        (Bag, OrderedSet, loss(false, false, true), vec![1, 2, 3]),
        (
            OrderedSet,
            Sequence,
            loss(false, true, false),
            vec![3, 1, 2],
        ),
        (OrderedSet, Set, loss(true, false, false), vec![1, 2, 3]),
        (OrderedSet, Bag, loss(true, true, false), vec![1, 2, 3]),
    ];
    for (source_kind, target, expected_loss, expected) in rows {
        let source = collection(source_kind, &[3, 1, 3, 2]);
        let source = members(&source);
        assert_eq!(
            CollectionLoss::of_conversion(source_kind, target),
            expected_loss
        );
        let refused = convert(source, target, CollectionLoss::NONE, bound());
        if expected_loss == CollectionLoss::NONE {
            assert!(refused.is_ok(), "{source_kind:?} -> {target:?}");
        } else {
            assert_eq!(
                refused.err(),
                Some(ill_typed(IllTypedCause::UnacceptedCollectionLoss)),
                "{source_kind:?} -> {target:?}"
            );
        }
        // Accepting exactly the loss is enough.
        let conversion = match convert(source, target, expected_loss, bound()) {
            Ok(Outcome::Completed(conversion)) => conversion,
            other => panic!("{source_kind:?} -> {target:?}: {other:?}"),
        };
        assert_eq!(conversion.loss(), expected_loss);
        assert_eq!(
            semantic(conversion.value()),
            (target, expected),
            "{source_kind:?} -> {target:?}"
        );
    }
    // Ordering an unordered collection needs a total element key.
    let option_type = ValueType::Option(Box::new(ValueType::Integer));
    let options = CollectionValue::construct(Set, option_type, bound(), Vec::new()).unwrap();
    assert_eq!(
        convert(members(&options), Sequence, CollectionLoss::ALL, bound()).err(),
        Some(ill_typed(IllTypedCause::NoTotalElementKey))
    );
    // The target bound applies to the converted value.
    let source = collection(Sequence, &[3, 1, 3, 2]);
    assert!(matches!(
        convert(
            members(&source),
            Set,
            CollectionLoss::ALL,
            Some(CardinalityBound::new(4, 8).unwrap())
        ),
        Ok(Outcome::Refused(Refusal::Cardinality(
            CardinalityViolation::BelowMinimum {
                minimum: 4,
                counted: 3
            }
        )))
    ));
}

#[trace("TC-190", "FR-145-AC-5")]
#[test]
fn unordered_folds_require_a_commutative_associative_function() {
    let associative_only = || binary(false, true, |a, b| a + b);
    let commutative_only = || binary(true, false, |a, b| a + b);
    for kind in CollectionKind::ALL {
        let source = collection(kind, &[1, 2, 3]);
        let source = members(&source);
        for function in [difference(), associative_only(), commutative_only()] {
            let folded = fold(source, &function, int(0), &mut meter());
            let reduced = reduce(source, &function, &mut meter());
            if kind.is_ordered() {
                assert!(folded.is_ok() && reduced.is_ok(), "{kind:?}");
            } else {
                let refusal = Some(ill_typed(
                    IllTypedCause::UnorderedFoldRequiresCommutativeAssociative,
                ));
                assert_eq!(folded.err(), refusal, "{kind:?}");
                assert_eq!(reduced.err(), refusal, "{kind:?}");
                assert_eq!(function.calls.get(), 0, "{kind:?}");
            }
        }
        // Ordered kinds traverse occurrence order from the identity.
        if kind.is_ordered() {
            let result = completed(fold(source, &difference(), int(0), &mut meter()));
            assert_eq!(integer(&result), -6);
            let result = completed(reduce(source, &difference(), &mut meter()));
            assert_eq!(integer(&result), -4);
        }
    }
}
