// SPDX-License-Identifier: AGPL-3.0-or-later
//! O-13 occurrence-pair equality planning.
//!
//! Ported from QSL `value::equality`, trimmed to the operand-agnostic
//! occurrence-pair plan: [`plan_pairs`], [`plan_equality`], `planned_equality`
//! and [`EqualityPlan`]. Everything the original module built on top of that
//! plan is dropped, because it needs the declaration registry
//! (`TypeEnvironment::check_equality`, `contains_ieee`), the original
//! declaration-aware `EnumValue`'s ordering/case lookup (`compare_enum` --
//! superseded here by `ValueType::Enum`'s inline `EnumShape` set, see
//! `crate::value`'s module doc comment, which needs no declaration lookup)
//! or the unit graph (`operand_value`'s Quantity conversion arm,
//! `admits_equality_conversion`'s Quantity row): `EqualityOperator`/
//! `EqualityOperand`/`EqualitySchedule`/`CheckedEquality`/`check_equality`/
//! `operand_value`/`admits_equality_conversion`/`integer_to_decimal`/
//! `decimal_to_rational`. Selecting and running the top-level text/enum/
//! quantity schedules ahead of the generic occurrence-pair plan is therefore
//! QSL's job, done above the kernel with the registry and unit graph it
//! holds.
//!
//! The leaf comparison inside `plan_pairs` is adapted for the kernel's bare
//! `Value::Enum` payload: two enum values compare equal exactly when their
//! `VariantId` digests are equal. A checked program guarantees both operands
//! share one declared `ValueType::Enum(EnumShape)` before this ever runs --
//! the same invariant every other leaf type here relies on (an `Integer`
//! carries no declared bound either) -- so this needs no same-enum check of
//! its own, exactly as before. A quantity pair compares equal only in the
//! same unit.

use crate::accounting::{Charge, ChargePoint, LimitKind, Meter};
use crate::integer::Integer;
use crate::outcome::{Outcome, Refusal, Stop};
use crate::value::{FieldValue, Value};

/// The complete occurrence-pair plan of one planned equality.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct EqualityPlan {
    pair_events: Integer,
}

impl EqualityPlan {
    /// The exact number of planned `equality.pair` events.
    pub fn pair_events(&self) -> &Integer {
        &self.pair_events
    }
}

/// Form the plan of two completed operands of one type, without charge. A
/// reference pair of different universes refuses with `foreign_reference`.
pub fn plan_equality(left: &Value, right: &Value) -> Result<EqualityPlan, Refusal> {
    plan_pairs(left, right).map(|plan| EqualityPlan {
        pair_events: plan.pairs,
    })
}

/// The equality schedule over completed operands of one type: charges
/// `equality.plan-form`, `equality.plan`, one `equality.pair` per planned
/// occurrence-path pair and `equality.result-retain`.
pub fn planned_equality(left: &Value, right: &Value, meter: &mut Meter) -> Outcome<bool> {
    Outcome::from_stop(planned_equality_stop(left, right, meter))
}

fn planned_equality_stop(left: &Value, right: &Value, meter: &mut Meter) -> Result<bool, Stop> {
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

/// Walk the occurrence-pair tree of two values of one type. Operands that
/// are not of one type, which a checked program never produces, refuse with
/// the checked invariant.
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
            (Value::Enum(l), Value::Enum(r)) => l == r,
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
                | Value::Collection(_),
                _,
            ) => return Err(Refusal::CheckedInvariant),
        };
        equal = equal && leaf;
    }
    Ok(PlannedPairs { pairs, equal })
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;

    use super::*;

    /// TC-321: two equal integers plan one pair and compare equal.
    #[trace("TC-321")]
    #[test]
    fn tc_321_equal_integers_plan_one_equal_pair() {
        let left = Value::Integer(Integer::one());
        let right = Value::Integer(Integer::one());
        let plan = plan_pairs(&left, &right).unwrap();
        assert_eq!(plan.pairs, Integer::one());
        assert!(plan.equal);
    }

    /// TC-322: a reference pair of different universes refuses with
    /// `ForeignReference` rather than comparing structurally.
    #[trace("TC-322")]
    #[test]
    fn tc_322_foreign_reference_pair_is_refused() {
        use crate::identity::{EffectiveId, ObjectId, UniverseId};
        use crate::reference::ObjectReference;

        fn digest(byte: u8) -> [u8; 32] {
            let mut bytes = [0_u8; 32];
            bytes[31] = byte;
            bytes
        }

        let object_type = EffectiveId::from_digest(digest(1));
        let object = ObjectId::from_digest(digest(2));
        let left = Value::Reference(ObjectReference::new(
            UniverseId::from_digest(digest(10)),
            object_type,
            object,
        ));
        let right = Value::Reference(ObjectReference::new(
            UniverseId::from_digest(digest(20)),
            object_type,
            object,
        ));
        assert!(matches!(
            plan_pairs(&left, &right),
            Err(Refusal::ForeignReference)
        ));
    }
}
