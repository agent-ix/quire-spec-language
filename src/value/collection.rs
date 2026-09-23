// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-144 collection kinds: bounded, metered construction and the canonical
//! order.
//!
//! `CollectionType`, `CollectionValue`, `construct_collection`,
//! `form_collection` and `from_admitted` are `quire_exact`'s own kernel
//! items now (QSL-131 V5, ADR-011 §6.1's K row) and are re-exported below
//! rather than duplicated. `form`, `form_grouped`, `coalesce`,
//! `member_equal` and `sort_by_key` stay QSL's own: the kernel keeps its own
//! equivalents `pub(crate)` (its private, trusted, no-recheck building
//! blocks for its own checked `record`/`tuple`-adjacent construction), so
//! they are not reachable from this separate crate. This module's own
//! `member_equal` charges the `collection.member-walk`/
//! `collection.member-test` points on `value::equality`'s own
//! [`plan_pairs`](super::equality::plan_pairs) (kept there for the same
//! reason: the kernel's `plan_pairs` is `pub(crate)` too, and no public
//! kernel primitive gives both a pair count and an equality Boolean under
//! these charge points -- `quire_exact::planned_equality` charges
//! `equality.*`, not `collection.*`). `form`/`form_grouped` are the
//! trusted, unmetered-recheck primitives `value::expression::evaluate`'s
//! `Machine` needs for values it already knows are admitted; `form_grouped`
//! wraps `quire_exact::form_grouped`'s own metered charge-and-bound logic,
//! converting its `Outcome` back to this module's internal `Stop` carrier.

use std::cell::Cell;
use std::cmp::Ordering;

use super::composite::Value;
use super::equality::plan_pairs;
use super::key::compare_keys;
use super::stop::{outcome_into_stop, Stop};
pub use quire_exact::{construct_collection, form_collection, from_admitted};
use quire_exact::{
    length_amount, Charge, ChargePoint, CollectionKind, Integer, LimitKind, Meter, Refusal,
};
pub use quire_exact::{CollectionType, CollectionValue};

/// Form a collection of `collection_type` from completed occurrences in
/// source or visiting order, trusting that every occurrence is already
/// admitted (the caller -- `value::expression::evaluate`'s `Machine` -- built
/// them from a checked, already-typed expression): membership comparisons,
/// `collection.bound`, the bound check and `collection.result-retain`.
pub(crate) fn form(
    collection_type: &CollectionType,
    occurrences: Vec<Value>,
    meter: &mut Meter,
) -> Result<Value, Stop> {
    let kind = collection_type.kind();
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
/// canonical order and `collection.result-retain`. Wraps
/// [`quire_exact::form_grouped`], converting its `Outcome` back to this
/// module's internal [`Stop`] carrier for `?`-propagation in
/// `value::expression::evaluate`'s `Machine`.
pub(crate) fn form_grouped(
    collection_type: &CollectionType,
    elements: Vec<Value>,
    meter: &mut Meter,
) -> Result<Value, Stop> {
    outcome_into_stop(quire_exact::form_grouped(collection_type, elements, meter))
}

fn bound_and_retain(
    collection_type: &CollectionType,
    mut elements: Vec<Value>,
    count: u64,
    meter: &mut Meter,
) -> Result<Value, Stop> {
    let kind = collection_type.kind();
    meter.charge(
        Charge::new(ChargePoint::CollectionBound).size(LimitKind::ValueOccurrences, count),
    )?;
    let bound = collection_type.bound();
    let violation = if count < bound.minimum() {
        Some(quire_exact::BoundViolation::BelowMinimum)
    } else if count > bound.maximum() {
        Some(quire_exact::BoundViolation::AboveMaximum)
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
            .exact_results(occ),
    )?;
    Ok(from_admitted(collection_type.clone(), elements))
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
