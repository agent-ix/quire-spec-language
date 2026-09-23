// SPDX-License-Identifier: AGPL-3.0-or-later
//! `plan_pairs`: forming the FR-149 occurrence-pair plan over two completed
//! values of one type, walked iteratively so value depth never reaches the
//! host stack and over the occurrence tree so DAG sharing never changes the
//! plan.
//!
//! `quire_exact::equality` (QSL-131) has its own, `pub(crate)`
//! `plan_pairs` over `quire_exact`'s own `Value` -- now the same `Value`
//! this module's does, since QSL-131 V5 retyped `value::composite` onto the
//! kernel type. `plan_pairs` still cannot be re-exported from there, though:
//! the kernel keeps it `pub(crate)` (only its own `planned_equality`,
//! `plan_equality` and `crate::collection::coalesce` call it internally), so
//! this crate's own [`value::collection`](super::collection)'s
//! `member_equal` -- which needs a pair count and an equality Boolean under
//! the *collection* `collection.member-walk`/`collection.member-test`
//! charge points, not `planned_equality`'s `equality.*` ones -- still needs
//! its own copy. [`plan_equality`](quire_exact::plan_equality) and
//! [`planned_equality`](quire_exact::planned_equality) themselves *are*
//! `pub` kernel functions with identical behavior to this module's own
//! former copies, so both are re-exported directly rather than duplicated;
//! `value::declaration`'s `CheckedEquality::run` calls
//! `quire_exact::planned_equality` directly for the same reason.
//!
//! The type-checked layer built on `plan_pairs` (`EqualityOperator`,
//! `EqualityOperand`, `EqualitySchedule`, `CheckedEquality`,
//! `TypeEnvironment::check_equality`, `admits_equality_conversion`,
//! `operand_value`) is parameterized over a `TypeEnvironment` and a checked
//! `ValueType`, so `value::declaration` owns it.

use quire_exact::{Integer, Refusal};

// `EqualityPlan` and `plan_equality` are `quire_exact`'s own canonical items
// (QSL-131): `EqualityPlan` wraps nothing but an `Integer` pair count, with
// no dependency on `Value`/`ValueType`, and `plan_equality` is unmetered,
// computing only from `plan_pairs`, so both are reused directly rather than
// duplicated.
pub use quire_exact::{plan_equality, EqualityPlan};

use super::composite::{FieldValue, Value};

/// A formed plan: its pair count and the relation's Boolean.
pub(crate) struct PlannedPairs {
    pub(crate) pairs: Integer,
    pub(crate) equal: bool,
}

/// One node of the occurrence-pair tree still to be formed.
enum Pair<'a> {
    Values(&'a Value, &'a Value),
    Slots(&'a FieldValue, &'a FieldValue),
}

/// Walk the FR-149 occurrence-pair tree of two values of one type. Operands
/// that are not of one type, which a checked program never produces, refuse
/// with the checked invariant.
pub(crate) fn plan_pairs(left: &Value, right: &Value) -> Result<PlannedPairs, Refusal> {
    let mut pairs = Integer::zero();
    let mut equal = true;
    let mut pending = vec![Pair::Values(left, right)];
    while let Some(pair) = pending.pop() {
        pairs = pairs.add(&Integer::one());
        let (left, right) = match pair {
            Pair::Slots(FieldValue::Present(left), FieldValue::Present(right)) => (left, right),
            Pair::Slots(FieldValue::Absent, FieldValue::Absent)
            | Pair::Slots(FieldValue::Null, FieldValue::Null) => continue,
            Pair::Slots(FieldValue::Present(_) | FieldValue::Absent | FieldValue::Null, _) => {
                equal = false;
                continue;
            }
            Pair::Values(left, right) => (left, right),
        };
        let leaf = match (left, right) {
            (Value::Boolean(l), Value::Boolean(r)) => l == r,
            (Value::Integer(l), Value::Integer(r)) => l == r,
            (Value::Rational(l), Value::Rational(r)) => l == r,
            (Value::Decimal(l), Value::Decimal(r)) => l.numerically_equal(r),
            (Value::Quantity(l), Value::Quantity(r)) if l.unit() == r.unit() => {
                l.magnitude() == r.magnitude()
            }
            (Value::Text(l), Value::Text(r))
                if l.text_type().profile() == r.text_type().profile() =>
            {
                l.retained() == r.retained()
            }
            // ADR-013 O-14: "Identity and equality use the `VariantId` only"
            // -- the paired rank (OQ-D) is ignored, and no declaration guard
            // is needed: a checked program never brings enum values of two
            // different declarations together here (FR-141-AC-2 refuses that
            // at type checking), exactly as the other leaf arms above rely on
            // their own declared type rather than re-checking it per pair.
            (Value::Enum(l), Value::Enum(r)) => l.variant() == r.variant(),
            (Value::Reference(l), Value::Reference(r)) => {
                if l.universe() != r.universe() {
                    return Err(Refusal::ForeignReference);
                }
                l == r
            }
            (Value::Option(l), Value::Option(r)) => match (l.payload(), r.payload()) {
                (Some(l), Some(r)) => {
                    pending.push(Pair::Values(l, r));
                    continue;
                }
                (l, r) => l.is_some() == r.is_some(),
            },
            (Value::Composite(l), Value::Composite(r))
                if l.declaration() == r.declaration() && l.slots().len() == r.slots().len() =>
            {
                let slots = l.slots().iter().zip(r.slots()).rev();
                pending.extend(slots.map(|(l, r)| Pair::Slots(l, r)));
                continue;
            }
            (Value::Collection(l), Value::Collection(r)) => {
                let kind = l.collection_type().kind();
                if kind != r.collection_type().kind() {
                    return Err(Refusal::CheckedInvariant);
                }
                let (l, r) = (l.elements(), r.elements());
                // A set's member count and a bag's occurrence count are the
                // stored lengths; the other kinds compare lengths directly.
                if l.len() != r.len() {
                    false
                } else {
                    let ranks = l.iter().zip(r).rev();
                    pending.extend(ranks.map(|(l, r)| Pair::Values(l, r)));
                    continue;
                }
            }
            (
                Value::Boolean(_)
                | Value::Integer(_)
                | Value::Rational(_)
                | Value::Decimal(_)
                | Value::Float(_)
                | Value::Quantity(_)
                | Value::Text(_)
                | Value::Enum(_)
                | Value::Reference(_)
                | Value::Option(_)
                | Value::Composite(_)
                | Value::Collection(_)
                | Value::Population(_),
                _,
            ) => return Err(Refusal::CheckedInvariant),
        };
        equal = equal && leaf;
    }
    Ok(PlannedPairs { pairs, equal })
}
