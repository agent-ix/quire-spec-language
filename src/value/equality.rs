// SPDX-License-Identifier: AGPL-3.0-or-later
//! `plan_pairs`/`plan_equality`/`planned_equality`: forming the FR-149
//! occurrence-pair plan over two completed values of one type, walked
//! iteratively so value depth never reaches the host stack and over the
//! occurrence tree so DAG sharing never changes the plan.
//!
//! `quire_exact::equality` (QSL-131) has its own `plan_pairs`/`plan_equality`/
//! `planned_equality` over `quire_exact`'s own `Value`, a distinct type from
//! this module's (`value::composite`'s module doc). Only `EqualityPlan`
//! itself is `quire_exact`'s type here: it wraps an `Integer` pair count with
//! no dependency on either crate's `Value`.
//!
//! These functions take only a pair of completed `Value`s. The type-checked
//! layer built on them (`EqualityOperator`, `EqualityOperand`,
//! `EqualitySchedule`, `CheckedEquality`, `TypeEnvironment::check_equality`,
//! `admits_equality_conversion`, `operand_value`) is parameterized over a
//! `TypeEnvironment` and a checked `ValueType`, so `value::declaration` owns
//! it.

use super::composite::{FieldValue, Value};
use super::outcome::{Refusal, Stop};
use quire_exact::{Charge, ChargePoint, Integer, LimitKind, Meter};

// `EqualityPlan` is `quire_exact`'s own canonical type (QSL-131): it wraps
// nothing but an `Integer` pair count, so it carries no dependency on the
// diverged `Value`/`ValueType` kernel types below (QSL-131's 2026-09-21
// comment: `ValueType::Enum`'s payload and `ValueType::Reference`'s payload
// differ between this crate and `quire_exact`). `EqualityPlan::new` is
// `quire_exact`'s own widening (QSL-131) of what was a private struct
// literal, since the field is unreachable once the type is foreign.
pub use quire_exact::EqualityPlan;

/// Form the plan of two completed operands of one type, without charge. A
/// reference pair of different universes refuses with `foreign_reference`.
pub fn plan_equality(left: &Value, right: &Value) -> Result<EqualityPlan, Refusal> {
    plan_pairs(left, right).map(|plan| EqualityPlan::new(plan.pairs))
}

/// The equality schedule over completed operands of one type.
/// `value::declaration`'s `CheckedEquality::run` uses it for the
/// `EqualitySchedule::Plan` schedule.
pub(crate) fn planned_equality(
    left: &Value,
    right: &Value,
    meter: &mut Meter,
) -> Result<bool, Stop> {
    let (left_occ, right_occ) = (left.occ(), right.occ());
    meter.charge(
        Charge::new(ChargePoint::EqualityPlanForm)
            .exact_size(
                LimitKind::ValueOccurrences,
                left_occ.clone().max(right_occ.clone()),
            )
            .work(left_occ.add(&right_occ)),
    )?;
    let plan = plan_pairs(left, right).map_err(Stop::Refused)?;
    meter.charge_plan(&plan.pairs)?;
    let mut remaining = plan.pairs;
    while !remaining.is_zero() {
        meter.charge(Charge::new(ChargePoint::EqualityPair))?;
        remaining = remaining.sub(&Integer::one());
    }
    meter.charge(Charge::new(ChargePoint::EqualityResultRetain).results(1))?;
    Ok(plan.equal)
}

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
                l.value() == r.value()
            }
            (Value::Text(l), Value::Text(r))
                if l.text_type().profile() == r.text_type().profile() =>
            {
                l.retained() == r.retained()
            }
            (Value::Enum(l), Value::Enum(r)) if l.declaration() == r.declaration() => {
                l.member() == r.member()
            }
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
