// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-144 collection kinds: bounded, metered construction and the canonical
//! order.
//!
//! A collection type `K<T>[min, max]` includes its bound. Construction charges
//! the `quire.value.accounting/v1` collection family: one `collection.element`
//! before each element expression; for a set, bag or ordered set, one
//! `collection.member-walk` and `collection.member-test` per membership
//! comparison against the members retained so far, in retention order and
//! stopping at the first equal member; `collection.bound` before the
//! uncharged bound check; then `collection.result-retain`. A set or bag stores
//! its occurrences in ascending canonical-key order (a bag listing each
//! occurrence), which is its canonical representation and visiting order.

use std::cell::Cell;
use std::cmp::Ordering;
use std::sync::Arc;

use super::composite::{
    Component, ConstructionCause, ConstructionRefusal, Deferred, Value, ValueType,
};
use super::equality::plan_pairs;
use super::key::compare_keys;
use super::outcome::{Outcome, Refusal, Stop};
use quire_exact::{
    length_amount, BoundViolation, CardinalityBound, Charge, ChargePoint, CollectionKind, Integer,
    LimitKind, Meter,
};

// `CardinalityBound`/`EmptyCardinalityBound` are `quire_exact`'s own FR-144
// kernel types (QSL-131 S-1b). `bound_and_retain` below recomputes the
// three-way bound comparison from their public `minimum()`/`maximum()`
// accessors rather than calling `quire_exact`'s own `violation` fn: that fn
// is `pub(crate)` there. `BoundViolation` is this same `quire_exact::
// BoundViolation`, so no `From` conversion is needed for the type; only the
// private `violation` fn stays unreachable.
//
// `CollectionType` and `CollectionValue` are not cut over (QSL-131,
// remaining work): `quire_exact::collection` has same-named types, but they
// are parameterized over `quire_exact`'s own `Value`/`ValueType`, which
// diverge from this crate's (`ValueType::Enum`'s and `ValueType::Reference`'s
// payloads differ -- see `value::composite`'s module doc). A `pub use` would
// not type-check against this crate's own `ValueType`/`Value`, so this
// crate's own `CollectionType`/`CollectionValue` and the construction
// functions built on them stay local, the same reason `value::composite`
// keeps its own `OptionValue`/`FieldValue`/`CompositeValue`.

/// A collection type `K<T>[min, max]`. Two collection types are the same type
/// exactly when kind, element type and bound are all equal.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CollectionType {
    kind: CollectionKind,
    element: ValueType,
    bound: CardinalityBound,
}

impl CollectionType {
    /// `kind<element>[bound]`. Whether a set, bag or ordered-set element type
    /// admits `=` is decided by
    /// [`TypeEnvironment::check_type`](super::TypeEnvironment::check_type).
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
    /// first-occurrence member order for an ordered set, ascending canonical
    /// key for a set, and ascending key listing each occurrence for a bag.
    pub fn elements(&self) -> &[Value] {
        &self.elements
    }

    /// `occ` of the collection.
    pub(crate) fn occ(&self) -> &Integer {
        &self.occ
    }
}

/// Materialize an already-charged, already-ordered result as a
/// `Value::Collection`, charging nothing and checking no bound: FR-153's
/// `allInstances<T>(p)` (`crate::model::population::all_instances`) already
/// charges its own `collection.bound`/`collection.result-retain` against this
/// same meter, and already checked its own declared maximum, before handing
/// its selection to this bridge, so charging or checking either again here
/// would double-count. `elements` must already be the collection's exact
/// canonical-key order with no duplicate; `allInstances`'s `ReferenceSet`
/// members are already unique (a `BTreeSet<ReferenceKey>`) and its
/// FR-143 identity bridge (`crate::value::reference`) preserves `ReferenceKey`
/// ascending order byte-for-byte into `ObjectReference` ascending order, so
/// no sort is needed either.
pub(crate) fn from_admitted(collection_type: CollectionType, elements: Vec<Value>) -> Value {
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
/// then runs; the first element that does not complete becomes the outcome and
/// no later element runs.
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
    form(collection_type, occurrences, meter)
}

/// Form a collection of `collection_type` from completed occurrences in source
/// or visiting order: membership comparisons, `collection.bound`, the bound
/// check and `collection.result-retain`. An occurrence outside the element
/// type refuses before any charge.
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
    Ok(Outcome::from_stop(form(
        collection_type,
        occurrences,
        meter,
    )))
}

pub(crate) fn form(
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

/// Form a collection from occurrences that are already distinct members (for
/// a set or ordered set) or grouped equal occurrences (for a bag), so no
/// membership comparison is charged: `collection.bound`, the bound check,
/// canonical order and `collection.result-retain`.
pub(crate) fn form_grouped(
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
    let bound = collection_type.bound;
    let violation = if count < bound.minimum() {
        Some(BoundViolation::BelowMinimum)
    } else if count > bound.maximum() {
        Some(BoundViolation::AboveMaximum)
    } else {
        None
    };
    if let Some(violation) = violation {
        return Err(Stop::Refused(Refusal::CardinalityOutOfBound {
            violation,
            kind,
            bound,
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

/// FR-144 occurrence formation for a set, bag or ordered set. The result lists
/// each retained member in retention order, a bag member once per occurrence
/// and adjacent to its equal occurrences.
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
            if member_equal(&candidate, member, meter)? {
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

/// One charged membership comparison of candidate `c` with member `m`.
pub(crate) fn member_equal(
    candidate: &Value,
    member: &Value,
    meter: &mut Meter,
) -> Result<bool, Stop> {
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
