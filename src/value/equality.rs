// SPDX-License-Identifier: AGPL-3.0-or-later
//! The FR-149 complete typed equality matrix.
//!
//! [`evaluate_equality`] first type-checks the operands: without a common
//! declared type it returns [`IllTyped`] and consumes nothing. A top-level
//! text, enum or quantity equality delegates to its FR-141/FR-142 comparison
//! schedule. Every other equality computes its complete [`EqualityPlan`],
//! reserves it through `equality.plan`, charges one `equality.pair` per planned
//! occurrence-path pair and retains one Boolean result.
//!
//! Traversal is iterative, so value depth never reaches the host stack.
//! Planning and deciding memoize immutable shared subvalues; that optimization
//! never changes the plan, because the plan counts occurrence-path pairs of the
//! value trees.

use std::collections::HashMap;
use std::sync::Arc;

use super::accounting::{Charge, ChargePoint, Meter};
use super::collection::CollectionKind;
use super::comparison::{ComparisonOperator, IllTyped, IllTypedCause};
use super::composite::{FieldValue, Value, ValueType};
use super::enumeration::compare_enum;
use super::integer::Integer;
use super::outcome::{Outcome, Stop};
use super::quantity::compare_quantity;
use super::rational::Rational;
use super::text::compare_text;

/// The complete occurrence-pair plan of one equality evaluation.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct EqualityPlan {
    pair_events: Integer,
}

impl EqualityPlan {
    /// The exact number of planned `equality.pair` events. The reservation
    /// is `pair_events + 2` work units and one result unit.
    pub fn pair_events(&self) -> &Integer {
        &self.pair_events
    }
}

/// Evaluate `left == right` under FR-149.
pub fn evaluate_equality(
    left: &Value,
    right: &Value,
    meter: &mut Meter,
) -> Result<Outcome<bool>, IllTyped> {
    match (left, right) {
        (Value::Text(left), Value::Text(right)) => {
            compare_text(ComparisonOperator::Equal, left, right, meter)
        }
        (Value::Enum(left), Value::Enum(right)) => {
            compare_enum(ComparisonOperator::Equal, left, right, meter)
        }
        (Value::Quantity(left), Value::Quantity(right)) => {
            compare_quantity(ComparisonOperator::Equal, left, right, meter)
        }
        _ => {
            let (plan, equal) = analyze(left, right)?;
            Ok(Outcome::from_stop(evaluate_plan(&plan, equal, meter)))
        }
    }
}

/// The complete plan of a planned equality, after the static type check.
// SPEC-GAP(119-1): FR-149 plans records, tuples, options and collections but
// does not say whether a top-level Boolean, integer, rational or decimal
// equality is planned. It is: a one-pair plan, since these rows have no other
// named schedule.
pub fn plan_equality(left: &Value, right: &Value) -> Result<EqualityPlan, IllTyped> {
    analyze(left, right).map(|(plan, _)| plan)
}

/// The plan and the relation's Boolean. Deciding before the reservation is
/// unobservable: no Boolean exists unless every planned charge is admitted.
fn analyze(left: &Value, right: &Value) -> Result<(EqualityPlan, bool), IllTyped> {
    check_common_type(left, right)?;
    let Analysis { pairs, equal } = traverse(left, right)?;
    Ok((EqualityPlan { pair_events: pairs }, equal))
}

/// Explicitly convert `value` to the comparison type `target`. Only a declared
/// lossless conversion exists; any other request is ill-typed. The source
/// value is not changed.
// SPEC-GAP(119-2): FR-149 requires "an explicitly declared lossless
// common-type conversion" without listing the declared conversions. The
// declared lossless conversions are identity and Integer or Decimal to
// Rational; a conversion to Decimal or Integer is lossy for some member of the
// source type and is therefore never declared, even for a representable value.
pub fn convert_for_equality(value: &Value, target: &ValueType) -> Result<Value, IllTyped> {
    match (value, target) {
        (Value::Integer(integer), ValueType::Rational) => {
            Ok(Value::Rational(Rational::from_integer(integer.clone())))
        }
        (Value::Decimal(decimal), ValueType::Rational) => {
            Ok(Value::Rational(decimal.normalized().to_rational()))
        }
        (value, target) if target.admits(value) => Ok(value.clone()),
        _ => Err(IllTyped {
            cause: IllTypedCause::NoLosslessConversion,
        }),
    }
}

/// The unmetered relation, for callers that normalize values of one declared
/// type (set uniqueness). Operands of different types are unequal here.
pub(crate) fn decide_equal(left: &Value, right: &Value) -> bool {
    matches!(traverse(left, right), Ok(Analysis { equal: true, .. }))
}

fn evaluate_plan(plan: &EqualityPlan, equal: bool, meter: &mut Meter) -> Result<bool, Stop> {
    meter.charge_plan(&plan.pair_events)?;
    let mut remaining = plan.pair_events.clone();
    while !remaining.is_zero() {
        meter.charge(Charge::new(ChargePoint::EqualityPair))?;
        remaining = remaining.sub(&Integer::one());
    }
    meter.charge(Charge::new(ChargePoint::EqualityResultRetain).results(1))?;
    Ok(equal)
}

fn ill_typed<T>(cause: IllTypedCause) -> Result<T, IllTyped> {
    Err(IllTyped { cause })
}

fn check_common_type(left: &Value, right: &Value) -> Result<(), IllTyped> {
    match (left, right) {
        (Value::Boolean(_), Value::Boolean(_))
        | (Value::Integer(_), Value::Integer(_))
        | (Value::Rational(_), Value::Rational(_))
        | (Value::Decimal(_), Value::Decimal(_)) => Ok(()),
        (Value::Quantity(l), Value::Quantity(r)) if !l.unit().has_dimension_of(r.unit()) => {
            ill_typed(IllTypedCause::IncompatibleDimensions)
        }
        (Value::Quantity(l), Value::Quantity(r)) if l.unit() != r.unit() => {
            ill_typed(IllTypedCause::DistinctUnits)
        }
        (Value::Text(l), Value::Text(r)) if l.text_type().profile() != r.text_type().profile() => {
            ill_typed(IllTypedCause::DistinctTextProfiles)
        }
        (Value::Enum(l), Value::Enum(r)) if l.declaration() != r.declaration() => {
            ill_typed(IllTypedCause::DistinctEnumDeclarations)
        }
        (Value::Quantity(_), Value::Quantity(_))
        | (Value::Text(_), Value::Text(_))
        | (Value::Enum(_), Value::Enum(_)) => Ok(()),
        (Value::Option(l), Value::Option(r)) if l.payload_type() == r.payload_type() => Ok(()),
        (Value::Collection(l), Value::Collection(r))
            if l.kind() == r.kind() && l.element_type() == r.element_type() =>
        {
            Ok(())
        }
        (Value::Composite(l), Value::Composite(r)) if l.declaration() == r.declaration() => Ok(()),
        (Value::Reference(l), Value::Reference(r)) if l.object_type() == r.object_type() => Ok(()),
        (Value::Composite(_), Value::Composite(_)) | (Value::Reference(_), Value::Reference(_)) => {
            ill_typed(IllTypedCause::DistinctDeclarations)
        }
        (
            Value::Boolean(_)
            | Value::Integer(_)
            | Value::Rational(_)
            | Value::Decimal(_)
            | Value::Quantity(_)
            | Value::Text(_)
            | Value::Enum(_)
            | Value::Option(_)
            | Value::Composite(_)
            | Value::Collection(_)
            | Value::Reference(_),
            _,
        ) => ill_typed(IllTypedCause::DistinctValueTypes),
    }
}

/// One planned occurrence-path pair.
#[derive(Clone, Copy)]
enum Pair<'a> {
    /// Two values of one declared type.
    Values(&'a Value, &'a Value),
    /// Two slot states that are not both present: terminal.
    Terminal(bool),
}

/// Whether a multiset relation compares members or multiplicities.
#[derive(Clone, Copy)]
enum Membership {
    Set,
    Bag,
}

/// What a value pair descends into.
enum Shape<'a> {
    /// A terminal leaf comparison.
    Leaf(bool),
    /// Declaration/index-order subpairs.
    Positional(Vec<Pair<'a>>),
    /// The full left-by-right element cross-product, row-major.
    CrossProduct {
        membership: Membership,
        left: &'a [Value],
        right: &'a [Value],
    },
}

/// The planned pair events and Boolean of one pair and its subpairs.
#[derive(Clone)]
struct Analysis {
    pairs: Integer,
    equal: bool,
}

/// Decide a set or bag relation from its row-major `rows × columns` pair
/// matrix, without early exit.
fn decide_membership(membership: Membership, rows: usize, columns: usize, matrix: &[bool]) -> bool {
    if rows != columns {
        return false;
    }
    let row = |index: usize| {
        matrix
            .get(index.saturating_mul(columns)..)
            .unwrap_or_default()
            .iter()
            .take(columns)
    };
    let row_count = |index: usize| row(index).filter(|equal| **equal).count();
    let column_count = |column: usize| {
        (0..rows)
            .filter(|index| row(*index).nth(column).copied().unwrap_or(false))
            .count()
    };
    match membership {
        Membership::Set => {
            (0..rows).all(|index| row_count(index) > 0)
                && (0..columns).all(|column| column_count(column) > 0)
        }
        Membership::Bag => (0..rows).all(|index| {
            let count = row_count(index);
            count > 0
                && row(index)
                    .enumerate()
                    .filter(|(_, equal)| **equal)
                    .all(|(column, _)| column_count(column) == count)
        }),
    }
}

/// The subpairs of one value pair. Operands of different declared types are
/// ill-typed.
// SPEC-GAP(119-3): FR-149 does not state how a structural mismatch within one
// declared type is planned: a different variant constructor, sequence length,
// option state, or absent/null/present slot state. Each is one terminal
// (unequal) pair with no subpairs; two absent or two null slots are one
// terminal equal pair.
// SPEC-GAP(119-4): FR-149 lets a set or bag "with a total canonical element
// key" use that key but does not say which element types have one or how a
// keyed plan is counted. Every set and bag uses the full cross-product plan,
// over bag occurrences; unequal cardinalities still plan the full product.
fn shape<'a>(left: &'a Value, right: &'a Value) -> Result<Shape<'a>, IllTyped> {
    let leaf = |equal| Ok(Shape::Leaf(equal));
    match (left, right) {
        (Value::Boolean(l), Value::Boolean(r)) => leaf(l == r),
        (Value::Integer(l), Value::Integer(r)) => leaf(l == r),
        (Value::Rational(l), Value::Rational(r)) => leaf(l == r),
        (Value::Decimal(l), Value::Decimal(r)) => leaf(l.numerically_equal(r)),
        // One declared type fixes one unit (FR-149 N1: value only).
        (Value::Quantity(l), Value::Quantity(r)) => leaf(l.value() == r.value()),
        (Value::Text(l), Value::Text(r)) => leaf(l.retained() == r.retained()),
        (Value::Enum(l), Value::Enum(r)) => leaf(l.member() == r.member()),
        (Value::Reference(l), Value::Reference(r)) => leaf(l.identity() == r.identity()),
        (Value::Option(l), Value::Option(r)) => match (l.payload(), r.payload()) {
            (Some(l), Some(r)) => Ok(Shape::Positional(vec![Pair::Values(l, r)])),
            (None, None) => leaf(true),
            (Some(_), None) | (None, Some(_)) => leaf(false),
        },
        (Value::Composite(l), Value::Composite(r)) => {
            if l.constructor() != r.constructor() || l.slots().len() != r.slots().len() {
                return leaf(false);
            }
            let pairs = l
                .slots()
                .iter()
                .zip(r.slots())
                .map(|slots| match slots {
                    (FieldValue::Present(l), FieldValue::Present(r)) => Pair::Values(l, r),
                    (FieldValue::Absent, FieldValue::Absent)
                    | (FieldValue::Null, FieldValue::Null) => Pair::Terminal(true),
                    (FieldValue::Present(_) | FieldValue::Absent | FieldValue::Null, _) => {
                        Pair::Terminal(false)
                    }
                })
                .collect();
            Ok(Shape::Positional(pairs))
        }
        (Value::Collection(l), Value::Collection(r)) => {
            let membership = match l.kind() {
                CollectionKind::Sequence | CollectionKind::OrderedSet => {
                    if l.elements().len() != r.elements().len() {
                        return leaf(false);
                    }
                    let pairs = l
                        .elements()
                        .iter()
                        .zip(r.elements())
                        .map(|(l, r)| Pair::Values(l, r))
                        .collect();
                    return Ok(Shape::Positional(pairs));
                }
                CollectionKind::Set => Membership::Set,
                CollectionKind::Bag => Membership::Bag,
            };
            Ok(Shape::CrossProduct {
                membership,
                left: l.elements(),
                right: r.elements(),
            })
        }
        (
            Value::Boolean(_)
            | Value::Integer(_)
            | Value::Rational(_)
            | Value::Decimal(_)
            | Value::Quantity(_)
            | Value::Text(_)
            | Value::Enum(_)
            | Value::Reference(_)
            | Value::Option(_)
            | Value::Composite(_)
            | Value::Collection(_),
            _,
        ) => ill_typed(IllTypedCause::DistinctValueTypes),
    }
}

type MemoKey = (*const (), *const ());

/// The identity of a shared immutable pair, for memoization only.
fn memo_key(left: &Value, right: &Value) -> Option<MemoKey> {
    fn address(value: &Value) -> Option<*const ()> {
        match value {
            Value::Option(node) => Some(Arc::as_ptr(node).cast()),
            Value::Composite(node) => Some(Arc::as_ptr(node).cast()),
            Value::Collection(node) => Some(Arc::as_ptr(node).cast()),
            Value::Boolean(_)
            | Value::Integer(_)
            | Value::Rational(_)
            | Value::Decimal(_)
            | Value::Quantity(_)
            | Value::Text(_)
            | Value::Enum(_)
            | Value::Reference(_) => None,
        }
    }
    Some((address(left)?, address(right)?))
}

/// How a frame combines its subpair results.
enum Combine {
    /// The synthetic root: exactly the one top-level pair.
    Root,
    /// One pair event over positional subpairs.
    Positional,
    /// One pair event over a row-major cross-product matrix.
    CrossProduct {
        membership: Membership,
        rows: usize,
        columns: usize,
        matrix: Vec<bool>,
    },
}

/// One pair awaiting its subpairs.
struct Frame<'a> {
    key: Option<MemoKey>,
    combine: Combine,
    pending: std::vec::IntoIter<Pair<'a>>,
    pairs: Integer,
    equal: bool,
}

impl<'a> Frame<'a> {
    fn new(key: Option<MemoKey>, combine: Combine, pending: Vec<Pair<'a>>) -> Self {
        let pairs = match combine {
            Combine::Root => Integer::zero(),
            Combine::Positional | Combine::CrossProduct { .. } => Integer::one(),
        };
        Self {
            key,
            combine,
            pending: pending.into_iter(),
            pairs,
            equal: true,
        }
    }

    fn absorb(&mut self, child: &Analysis) {
        self.pairs = self.pairs.add(&child.pairs);
        match &mut self.combine {
            Combine::Root | Combine::Positional => self.equal = self.equal && child.equal,
            Combine::CrossProduct { matrix, .. } => matrix.push(child.equal),
        }
    }

    fn finish(self) -> (Option<MemoKey>, Analysis) {
        let equal = match &self.combine {
            Combine::Root | Combine::Positional => self.equal,
            Combine::CrossProduct {
                membership,
                rows,
                columns,
                matrix,
            } => decide_membership(*membership, *rows, *columns, matrix),
        };
        (
            self.key,
            Analysis {
                pairs: self.pairs,
                equal,
            },
        )
    }
}

/// Plan and decide every occurrence-path pair of `left == right` in
/// declaration/index depth-first order without host recursion.
fn traverse(left: &Value, right: &Value) -> Result<Analysis, IllTyped> {
    let mut memo: HashMap<MemoKey, Analysis> = HashMap::new();
    let mut root = Frame::new(None, Combine::Root, vec![Pair::Values(left, right)]);
    let mut stack: Vec<Frame<'_>> = Vec::new();
    loop {
        let top = stack.last_mut().unwrap_or(&mut root);
        let Some(pair) = top.pending.next() else {
            let Some(done) = stack.pop() else {
                return Ok(root.finish().1);
            };
            let (key, analysis) = done.finish();
            if let Some(key) = key {
                memo.insert(key, analysis.clone());
            }
            stack.last_mut().unwrap_or(&mut root).absorb(&analysis);
            continue;
        };
        let (left, right) = match pair {
            Pair::Terminal(equal) => {
                top.absorb(&leaf(equal));
                continue;
            }
            Pair::Values(left, right) => (left, right),
        };
        let key = memo_key(left, right);
        if let Some(done) = key.and_then(|key| memo.get(&key)) {
            top.absorb(done);
            continue;
        }
        match shape(left, right)? {
            Shape::Leaf(equal) => top.absorb(&leaf(equal)),
            Shape::Positional(pairs) => stack.push(Frame::new(key, Combine::Positional, pairs)),
            Shape::CrossProduct {
                membership,
                left,
                right,
            } => {
                let pairs = left
                    .iter()
                    .flat_map(|l| right.iter().map(move |r| Pair::Values(l, r)))
                    .collect();
                let combine = Combine::CrossProduct {
                    membership,
                    rows: left.len(),
                    columns: right.len(),
                    matrix: Vec::new(),
                };
                stack.push(Frame::new(key, combine, pairs));
            }
        }
    }
}

fn leaf(equal: bool) -> Analysis {
    Analysis {
        pairs: Integer::one(),
        equal,
    }
}
