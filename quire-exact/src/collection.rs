// SPDX-License-Identifier: AGPL-3.0-or-later
//! Bounded collection kinds: metered construction and the canonical order.
//!
//! A collection type `K<T>[min, max]` includes its bound. Construction
//! charges the `quire.value.accounting/v1` collection family: one
//! `collection.element` before each element expression; for a set, bag or
//! ordered set, one `collection.member-walk` and `collection.member-test`
//! per membership comparison against the members retained so far, in
//! retention order and stopping at the first equal member;
//! `collection.bound` before the uncharged bound check; then
//! `collection.result-retain`. A set or bag stores its occurrences in
//! ascending canonical-key order (a bag listing each occurrence), which is
//! its canonical representation and visiting order.
//!
//! Ported from QSL `value::collection` with imports repointed at this
//! crate's modules; [`from_admitted`] is widened from `pub(crate)` to `pub`
//! since its caller (QSL's own `model::population::all_instances`, which
//! this comment used to name directly) is now a separate crate.
//!
//! QSL-131 V5b widens two more trusted, no-recheck primitives from
//! `pub(crate)` to `pub`, each as an `Outcome`-returning wrapper around its
//! existing `Stop`-based body (the same split [`form_grouped`] already used):
//! [`form`] (QSL's `value::expression::evaluate` `Machine` builds occurrences
//! from a checked, already-typed expression and forms them with no
//! re-admission check) and [`member_equal`] (the same `Machine` needs one
//! charged membership comparison for `Contains`, under these same
//! `collection.member-walk`/`collection.member-test` charge points). Neither
//! kernel `Stop` (this crate's own private early-exit carrier) nor
//! `bound_and_retain`/`coalesce`/`sort_by_key` (their private helpers) are
//! exposed: QSL adapts the `Outcome` return with its own `value::stop`
//! helpers, the same way it already adapts [`form_grouped`].

use std::cell::Cell;
use std::cmp::Ordering;
use std::sync::Arc;

use crate::accounting::{length_amount, Charge, ChargePoint, LimitKind, Meter};
use crate::equality::plan_pairs;
use crate::integer::Integer;
use crate::key::compare_keys;
use crate::outcome::{BoundViolation, Outcome, Refusal, Stop};
use crate::value::{Component, ConstructionCause, ConstructionRefusal, Deferred, Value, ValueType};

/// A collection kind.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum CollectionKind {
    /// Ordered, duplicates retained.
    Sequence,
    /// Unordered, unique members.
    Set,
    /// Unordered, duplicates retained as multiplicity.
    Bag,
    /// Unique members in first-occurrence order.
    OrderedSet,
}

impl CollectionKind {
    /// Every kind.
    pub const ALL: [Self; 4] = [Self::Sequence, Self::Set, Self::Bag, Self::OrderedSet];

    /// Whether equal occurrences coalesce into one member.
    pub fn is_unique(self) -> bool {
        matches!(self, Self::Set | Self::OrderedSet)
    }

    /// Whether occurrence order is semantic.
    pub fn is_ordered(self) -> bool {
        matches!(self, Self::Sequence | Self::OrderedSet)
    }
}

/// An inclusive declared cardinality bound `[minimum, maximum]`. It counts
/// occurrences for sequences and bags and members for sets and ordered sets.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct CardinalityBound {
    minimum: u64,
    maximum: u64,
}

/// A cardinality bound with `minimum > maximum`.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, thiserror::Error)]
#[error("empty cardinality bound [{minimum}, {maximum}]")]
pub struct EmptyCardinalityBound {
    /// The declared minimum.
    pub minimum: u64,
    /// The declared maximum.
    pub maximum: u64,
}

impl CardinalityBound {
    /// The inclusive bound `[minimum, maximum]`.
    pub fn new(minimum: u64, maximum: u64) -> Result<Self, EmptyCardinalityBound> {
        if minimum > maximum {
            return Err(EmptyCardinalityBound { minimum, maximum });
        }
        Ok(Self { minimum, maximum })
    }

    /// The inclusive minimum.
    pub fn minimum(self) -> u64 {
        self.minimum
    }

    /// The inclusive maximum.
    pub fn maximum(self) -> u64 {
        self.maximum
    }

    fn violation(self, count: u64) -> Option<BoundViolation> {
        if count < self.minimum {
            Some(BoundViolation::BelowMinimum)
        } else if count > self.maximum {
            Some(BoundViolation::AboveMaximum)
        } else {
            None
        }
    }
}

/// A collection type `K<T>[min, max]`. Two collection types are the same
/// type exactly when kind, element type and bound are all equal.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CollectionType {
    kind: CollectionKind,
    element: ValueType,
    bound: CardinalityBound,
}

impl CollectionType {
    /// `kind<element>[bound]`. Whether a set, bag or ordered-set element
    /// type admits `=` is a check made above the kernel, against the
    /// declaration registry that knows which types are comparable.
    pub fn new(kind: CollectionKind, element: ValueType, bound: CardinalityBound) -> Self {
        Self {
            kind,
            element,
            bound,
        }
    }

    /// The kind.
    pub fn kind(&self) -> CollectionKind {
        self.kind
    }

    /// The element type.
    pub fn element(&self) -> &ValueType {
        &self.element
    }

    /// The declared bound.
    pub fn bound(&self) -> CardinalityBound {
        self.bound
    }
}

/// A completed collection value.
#[derive(Clone, Debug)]
pub struct CollectionValue {
    collection_type: CollectionType,
    elements: Box<[Value]>,
    occ: Integer,
}

impl CollectionValue {
    /// The declared collection type.
    pub fn collection_type(&self) -> &CollectionType {
        &self.collection_type
    }

    /// The canonical representation: occurrence order for a sequence,
    /// first-occurrence member order for an ordered set, ascending
    /// canonical key for a set, and ascending key listing each occurrence
    /// for a bag.
    pub fn elements(&self) -> &[Value] {
        &self.elements
    }

    /// `occ` of the collection.
    pub(crate) fn occ(&self) -> &Integer {
        &self.occ
    }
}

/// Materialize an already-charged, already-ordered result as a
/// `Value::Collection`, charging nothing and checking no bound: the caller
/// must already have charged its own `collection.bound`/
/// `collection.result-retain` against this same meter and already checked
/// its own declared maximum, and `elements` must already be the
/// collection's exact canonical-key order with no duplicate.
pub fn from_admitted(collection_type: CollectionType, elements: Vec<Value>) -> Value {
    let occ = elements
        .iter()
        .fold(Integer::one(), |occ, element| occ.add(&element.occ()));
    Value::Collection(Arc::new(CollectionValue {
        collection_type,
        elements: elements.into_boxed_slice(),
        occ,
    }))
}

/// Evaluate a collection constructor expression `K[e1, ..., en]` checked
/// against `collection_type`. Each element charges `collection.element` and
/// then runs; the first element that does not complete becomes the outcome
/// and no later element runs.
pub fn construct_collection(
    collection_type: &CollectionType,
    elements: Vec<Deferred<'_>>,
    meter: &mut Meter,
) -> Outcome<Value> {
    Outcome::from_stop(construct(collection_type, elements, meter))
}

fn construct(
    collection_type: &CollectionType,
    elements: Vec<Deferred<'_>>,
    meter: &mut Meter,
) -> Result<Value, Stop> {
    let mut occurrences = Vec::with_capacity(elements.len());
    for element in elements {
        meter.charge(Charge::new(ChargePoint::CollectionElement))?;
        let value = element(meter).into_stop()?;
        if !collection_type.element.admits(&value) {
            return Err(Stop::Refused(Refusal::CheckedInvariant));
        }
        occurrences.push(value);
    }
    form_stop(collection_type, occurrences, meter)
}

/// Form a collection of `collection_type` from completed occurrences in
/// source or visiting order: membership comparisons, `collection.bound`,
/// the bound check and `collection.result-retain`. An occurrence outside
/// the element type refuses before any charge.
pub fn form_collection(
    collection_type: &CollectionType,
    occurrences: Vec<Value>,
    meter: &mut Meter,
) -> Result<Outcome<Value>, ConstructionRefusal> {
    if let Some(index) = occurrences
        .iter()
        .position(|value| !collection_type.element.admits(value))
    {
        return Err(ConstructionRefusal {
            component: Component::Element(index),
            cause: ConstructionCause::TypeMismatch,
        });
    }
    Ok(Outcome::from_stop(form_stop(
        collection_type,
        occurrences,
        meter,
    )))
}

/// Form a collection of `collection_type` from completed occurrences in
/// source or visiting order, trusting that every occurrence is already
/// admitted: membership comparisons, `collection.bound`, the bound check
/// and `collection.result-retain`. `pub`, not `pub(crate)`: its caller
/// (QSL's own expression evaluator's `Machine`, which already knows its
/// occurrences are admitted, from a checked, already-evaluated expression)
/// is a separate crate from this one.
pub fn form(
    collection_type: &CollectionType,
    occurrences: Vec<Value>,
    meter: &mut Meter,
) -> Outcome<Value> {
    Outcome::from_stop(form_stop(collection_type, occurrences, meter))
}

fn form_stop(
    collection_type: &CollectionType,
    occurrences: Vec<Value>,
    meter: &mut Meter,
) -> Result<Value, Stop> {
    let kind = collection_type.kind;
    let occurrence_count = length_amount(occurrences.len());
    let elements = if kind == CollectionKind::Sequence {
        occurrences
    } else {
        coalesce(kind, occurrences, meter)?
    };
    let count = if kind.is_unique() {
        length_amount(elements.len())
    } else {
        occurrence_count
    };
    bound_and_retain(collection_type, elements, count, meter)
}

/// Form a collection from occurrences that are already distinct members
/// (for a set or ordered set) or grouped equal occurrences (for a bag), so
/// no membership comparison is charged: `collection.bound`, the bound
/// check, canonical order and `collection.result-retain`. `pub`, not
/// `pub(crate)`: its caller (QSL's own expression evaluator, which decides
/// *when* occurrences are already grouped -- e.g. a comprehension's own
/// iteration order) is a separate crate from this one.
pub fn form_grouped(
    collection_type: &CollectionType,
    elements: Vec<Value>,
    meter: &mut Meter,
) -> Outcome<Value> {
    Outcome::from_stop(form_grouped_stop(collection_type, elements, meter))
}

fn form_grouped_stop(
    collection_type: &CollectionType,
    elements: Vec<Value>,
    meter: &mut Meter,
) -> Result<Value, Stop> {
    let count = length_amount(elements.len());
    bound_and_retain(collection_type, elements, count, meter)
}

fn bound_and_retain(
    collection_type: &CollectionType,
    mut elements: Vec<Value>,
    count: u64,
    meter: &mut Meter,
) -> Result<Value, Stop> {
    let kind = collection_type.kind;
    meter.charge(
        Charge::new(ChargePoint::CollectionBound).size(LimitKind::ValueOccurrences, count),
    )?;
    if let Some(violation) = collection_type.bound.violation(count) {
        return Err(Stop::Refused(Refusal::CardinalityOutOfBound {
            violation,
            kind,
            bound: collection_type.bound,
            count,
        }));
    }
    if !kind.is_ordered() {
        sort_by_key(&mut elements)?;
    }
    let occ = elements
        .iter()
        .fold(Integer::one(), |occ, element| occ.add(&element.occ()));
    meter.charge(
        Charge::new(ChargePoint::CollectionResultRetain)
            .exact_size(LimitKind::ValueOccurrences, occ.clone())
            .exact_results(occ.clone()),
    )?;
    Ok(Value::Collection(Arc::new(CollectionValue {
        collection_type: collection_type.clone(),
        elements: elements.into_boxed_slice(),
        occ,
    })))
}

/// Occurrence formation for a set, bag or ordered set. The result lists
/// each retained member in retention order, a bag member once per
/// occurrence and adjacent to its equal occurrences.
fn coalesce(
    kind: CollectionKind,
    occurrences: Vec<Value>,
    meter: &mut Meter,
) -> Result<Vec<Value>, Stop> {
    // (member, multiplicity) in retention order.
    let mut members: Vec<(Value, usize)> = Vec::new();
    for candidate in occurrences {
        let mut equal_member = None;
        for (index, (member, _)) in members.iter().enumerate() {
            if member_equal_stop(&candidate, member, meter)? {
                equal_member = Some(index);
                break;
            }
        }
        match equal_member.and_then(|index| members.get_mut(index)) {
            Some((_, multiplicity)) => {
                if kind == CollectionKind::Bag {
                    *multiplicity = multiplicity.saturating_add(1);
                }
            }
            None => members.push((candidate, 1)),
        }
    }
    Ok(members
        .into_iter()
        .flat_map(|(member, multiplicity)| std::iter::repeat_n(member, multiplicity))
        .collect())
}

/// One charged membership comparison of candidate `c` with member `m`,
/// under `collection.member-walk`/`collection.member-test`. `pub`, not
/// `pub(crate)`: its caller (QSL's own expression evaluator's `Machine`,
/// for `Contains`) is a separate crate from this one.
pub fn member_equal(candidate: &Value, member: &Value, meter: &mut Meter) -> Outcome<bool> {
    Outcome::from_stop(member_equal_stop(candidate, member, meter))
}

fn member_equal_stop(candidate: &Value, member: &Value, meter: &mut Meter) -> Result<bool, Stop> {
    let (candidate_occ, member_occ) = (candidate.occ(), member.occ());
    meter.charge(
        Charge::new(ChargePoint::CollectionMemberWalk)
            .exact_size(
                LimitKind::ValueOccurrences,
                candidate_occ.clone().max(member_occ.clone()),
            )
            .work(candidate_occ.add(&member_occ)),
    )?;
    let plan = plan_pairs(candidate, member).map_err(Stop::Refused)?;
    meter.charge(
        Charge::new(ChargePoint::CollectionMemberTest)
            .exact_size(LimitKind::ValueOccurrences, plan.pairs.clone())
            .work(plan.pairs),
    )?;
    Ok(plan.equal)
}

/// Stable ascending canonical-key order.
fn sort_by_key(elements: &mut [Value]) -> Result<(), Stop> {
    let unkeyed = Cell::new(false);
    elements.sort_by(|left, right| {
        compare_keys(left, right).unwrap_or_else(|| {
            unkeyed.set(true);
            Ordering::Equal
        })
    });
    if unkeyed.get() {
        return Err(Stop::Refused(Refusal::CheckedInvariant));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;

    use super::*;
    use crate::accounting::ScalarLimits;

    fn generous_meter() -> Meter {
        Meter::new(ScalarLimits {
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
        })
    }

    /// TC-314: an empty bound `[0, 3]` admits an empty sequence with zero
    /// occurrences retained.
    #[trace("TC-314")]
    #[test]
    fn tc_314_empty_sequence_within_bound_completes() {
        let bound = CardinalityBound::new(0, 3).unwrap();
        let collection_type =
            CollectionType::new(CollectionKind::Sequence, ValueType::Integer, bound);
        let mut meter = generous_meter();
        let outcome = form_collection(&collection_type, vec![], &mut meter).unwrap();
        let value = outcome.completed().expect("within bound");
        assert_eq!(value.occ(), Integer::one());
    }

    /// TC-315: forming a sequence past its declared maximum is refused with
    /// `AboveMaximum` and no collection is materialized.
    #[trace("TC-315")]
    #[test]
    fn tc_315_sequence_above_maximum_is_refused() {
        let bound = CardinalityBound::new(0, 1).unwrap();
        let collection_type =
            CollectionType::new(CollectionKind::Sequence, ValueType::Integer, bound);
        let occurrences = vec![
            Value::Integer(Integer::one()),
            Value::Integer(Integer::one()),
        ];
        let mut meter = generous_meter();
        let outcome = form_collection(&collection_type, occurrences, &mut meter).unwrap();
        assert!(matches!(
            outcome,
            Outcome::Refused(Refusal::CardinalityOutOfBound {
                violation: BoundViolation::AboveMaximum,
                ..
            })
        ));
    }
}
