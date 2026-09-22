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
    /// Wrap an already-computed pair count. `pub`, not merely a private
    /// struct literal: `EqualityPlan` is a plain data holder with no
    /// invariant beyond "this many pairs were planned", so it carries no
    /// accounting charge of its own; QSL's own occurrence-pair walk (over its
    /// own `Value`, a distinct type from this crate's) needs this to build
    /// one from a count it computed itself, the same way this module's own
    /// [`plan_equality`] does internally.
    pub fn new(pair_events: Integer) -> Self {
        Self { pair_events }
    }

    /// The exact number of planned `equality.pair` events.
    pub fn pair_events(&self) -> &Integer {
        &self.pair_events
    }
}

/// Form the plan of two completed operands of one type, without charge. A
/// reference pair of different universes refuses with `foreign_reference`.
///
/// **H-5, judgment call: stays `pub`, unmetered, not `pub(crate)`.** This
/// walks the whole occurrence-pair tree of caller-supplied values with no
/// meter, which is exactly the risk profile the other four H-5 findings
/// share. The difference here is that "without charge" is this function's
/// entire documented purpose: it lets a caller size an equality check's cost
/// (via [`EqualityPlan::pair_events`]) *before* spending [`planned_equality`]'s
/// metered budget on it. Giving it a `&mut Meter` would defeat that purpose,
/// and demoting it to `pub(crate)` deletes a QSL-facing capability that has
/// no in-crate substitute (`planned_equality` needs the `equal` bool this
/// plan discards, so it cannot be rewritten to call this instead). Left
/// `pub` and documented: a caller that does not want unbounded work must
/// bound its own operand size before calling this, the same way any
/// pre-metering admission check must.
///
/// **Round 2 correction:** "size the cost before spending the budget"
/// overstates what this buys. The walk here costs exactly what
/// [`planned_equality`]'s does over the same pair -- it is not a cheaper
/// estimate. Calling this first does not let a caller *avoid* paying that
/// cost; it lets them pay it once unmetered to decide whether to pay it
/// again metered.
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
                | Value::Population(_)
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
    use crate::accounting::ScalarLimits;

    fn generous_limits() -> ScalarLimits {
        ScalarLimits {
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
        }
    }

    fn generous_meter() -> Meter {
        Meter::new(generous_limits())
    }

    /// TC-346: `plan_equality` reports the exact pair count of two equal
    /// integers without charging (H-7/H-8: previously untested).
    #[trace("TC-346")]
    #[test]
    fn tc_346_plan_equality_reports_pairs_without_charge() {
        let left = Value::Integer(Integer::one());
        let right = Value::Integer(Integer::one());
        let plan = plan_equality(&left, &right).unwrap();
        assert_eq!(plan.pair_events(), &Integer::one());
    }

    /// TC-347: `planned_equality` completes true for two equal integers
    /// under a generous meter, and a meter with no `value_occurrences` left
    /// cannot admit `equality.plan-form`, so the identical comparison
    /// returns `Outcome::Incomplete` instead (H-7/H-8: `planned_equality`'s
    /// public API and metering path were previously untested, and metering
    /// accumulation/`Incomplete` was unproven end to end for any operation).
    #[trace("TC-347")]
    #[test]
    fn tc_347_planned_equality_charges_and_a_tight_meter_is_incomplete() {
        let left = Value::Integer(Integer::one());
        let right = Value::Integer(Integer::one());

        let mut generous = generous_meter();
        let outcome = planned_equality(&left, &right, &mut generous);
        assert!(outcome.completed().expect("charges available"));

        let mut tight = Meter::new(ScalarLimits {
            value_occurrences: 0,
            ..generous_limits()
        });
        let outcome = planned_equality(&left, &right, &mut tight);
        assert!(matches!(outcome, Outcome::Incomplete(_)));
    }

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

    /// TC-297 (FR-089-AC-6): a population pair is not an equality operand
    /// pair in the kernel.
    #[trace("TC-297", "FR-089-AC-6")]
    #[test]
    fn plan_pairs_refuses_a_population_pair() {
        use crate::identity::PopulationId;

        fn digest(byte: u8) -> [u8; 32] {
            let mut bytes = [0_u8; 32];
            bytes[31] = byte;
            bytes
        }

        let left = Value::Population(PopulationId::from_digest(digest(1)));
        let right = Value::Population(PopulationId::from_digest(digest(2)));
        assert!(matches!(
            plan_pairs(&left, &right),
            Err(Refusal::CheckedInvariant)
        ));
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
