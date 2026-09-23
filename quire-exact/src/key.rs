// SPDX-License-Identifier: AGPL-3.0-or-later
//! The type-owned total canonical key.
//!
//! Every type that admits `=` has a key, and two values of one type have
//! equal keys exactly when they are equal under [`crate::equality`]. The key
//! fixes set and bag canonical order and visiting order; it is not an
//! ordering operator. Comparison is iterative, so value depth never reaches
//! the host stack.
//!
//! **M-2: `Value::Float` has no key, matching its exclusion from `=`.**
//! [`crate::equality`]'s own leaf match excludes `Value::Float` the same
//! way this module's `leaf` function does -- both fall through to their
//! catch-all refusal arm. This is not an oversight in either module: IEEE
//! equality (`NaN != NaN`, `-0.0 == 0.0`) is not the total-order `=` this
//! key and the generic equality plan give every other leaf type, so IEEE
//! comparison stays [`crate::compare_ieee`]'s own `IeeeComparison`
//! (`NumericEqual`/`TotalOrder`/`BitIdentical`), never this key or `=`.
//! `crate::collection`'s set/bag coalescing groups occurrences by
//! `equality.rs`'s charged member comparison, not this key, so a Float set
//! or bag of one element still forms; the same `Refusal::CheckedInvariant`
//! that would stop this key on a Float pair also stops the first
//! two-element membership comparison there.
//!
//! One adaptation against the original: `Value::Enum` is [`crate::value::
//! EnumMember`] here (ADR-013 T-6, OQ-D ruling) -- a bare [`crate::identity::
//! VariantId`] paired with its zero-based canonical rank -- not a
//! declaration-aware `EnumValue` carrying a live position/case lookup. FR-144's
//! enumeration key row (FR-144-AC-9) fixes canonical order as declaration
//! position for an `ordered enum` and case-identifier byte order otherwise;
//! because `EnumMember::rank` is already that canonical-list index (the
//! shape that admitted it fixed it there, `crate::value::EnumShape::rank`),
//! comparing two same-enum members' ranks numerically reproduces FR-144's
//! order for *both* cases at once, with no declaration lookup and no
//! case-name string needed here. This corrects the prior digest-ordered
//! comparison this file carried (`quire-exact/src/key.rs:103` before this
//! change), which did not conform to FR-144 (ADR-013 O-14).

use std::cmp::Ordering;

use crate::value::{FieldValue, Value};

/// One pending key comparison.
enum Task<'a> {
    Values(&'a Value, &'a Value),
    Slots(&'a FieldValue, &'a FieldValue),
    /// The proper-prefix tiebreak of two element lists whose common prefix
    /// has equal keys.
    Lengths(usize, usize),
}

/// Compare the canonical keys of two values of one declared type. `None`
/// means the operands are not of one keyed type, which a checked program
/// never produces. `pub`: QSL's expression evaluator's `Machine` groups
/// adjacent equal elements with this key before [`crate::form_grouped`] and
/// in `Contains`, so it must order exactly as [`crate::form`]'s canonical
/// sort does.
pub fn compare_keys(left: &Value, right: &Value) -> Option<Ordering> {
    let mut tasks = vec![Task::Values(left, right)];
    while let Some(task) = tasks.pop() {
        let ordering = match task {
            Task::Lengths(left, right) => left.cmp(&right),
            Task::Slots(left, right) => match (left, right) {
                (FieldValue::Present(left), FieldValue::Present(right)) => {
                    tasks.push(Task::Values(left, right));
                    continue;
                }
                (left, right) => slot_rank(left).cmp(&slot_rank(right)),
            },
            Task::Values(left, right) => match leaf(left, right, &mut tasks)? {
                Some(ordering) => ordering,
                None => continue,
            },
        };
        if ordering.is_ne() {
            return Some(ordering);
        }
    }
    Some(Ordering::Equal)
}

/// `absent` before `null` before a present value.
fn slot_rank(slot: &FieldValue) -> u8 {
    match slot {
        FieldValue::Absent => 0,
        FieldValue::Null => 1,
        FieldValue::Present(_) => 2,
    }
}

/// The ordering of a leaf pair, or `Some(None)` after scheduling the children
/// of a structured pair in comparison order.
fn leaf<'a>(
    left: &'a Value,
    right: &'a Value,
    tasks: &mut Vec<Task<'a>>,
) -> Option<Option<Ordering>> {
    let ordering = match (left, right) {
        (Value::Boolean(left), Value::Boolean(right)) => left.cmp(right),
        (Value::Integer(left), Value::Integer(right)) => left.cmp(right),
        (Value::Rational(left), Value::Rational(right)) => left.cmp(right),
        (Value::Decimal(left), Value::Decimal(right)) => left.compare(right),
        (Value::Quantity(left), Value::Quantity(right)) if left.unit() == right.unit() => {
            left.magnitude().cmp(right.magnitude())
        }
        (Value::Text(left), Value::Text(right)) => {
            left.retained().as_bytes().cmp(right.retained().as_bytes())
        }
        // FND-003 (SR-511): a rank alone does not name a declaration. Two
        // members of *different* enum declarations can share a rank without
        // being equal under `crate::equality` (they have different
        // `VariantId`s), and this leaf carries no declaration to guard on.
        // When ranks are equal but the `VariantId`s differ, the pair is not
        // one keyed type, so this returns `None` rather than reporting a
        // false `Equal` -- the same contract `crate::equality`'s leaf match
        // already enforces on a `VariantId` mismatch.
        (Value::Enum(left), Value::Enum(right)) => {
            let ordering = left.rank().cmp(&right.rank());
            if ordering.is_eq() && left.variant() != right.variant() {
                return None;
            }
            ordering
        }
        (Value::Reference(left), Value::Reference(right)) => left.cmp(right),
        (Value::Option(left), Value::Option(right)) => match (left.payload(), right.payload()) {
            (Some(left), Some(right)) => {
                tasks.push(Task::Values(left, right));
                return Some(None);
            }
            (left, right) => left.is_some().cmp(&right.is_some()),
        },
        (Value::Composite(left), Value::Composite(right))
            if left.declaration() == right.declaration()
                && left.slots().len() == right.slots().len() =>
        {
            let slots = left.slots().iter().zip(right.slots()).rev();
            tasks.extend(slots.map(|(left, right)| Task::Slots(left, right)));
            return Some(None);
        }
        (Value::Collection(left), Value::Collection(right)) => {
            let (left, right) = (left.elements(), right.elements());
            tasks.push(Task::Lengths(left.len(), right.len()));
            let elements = left.iter().zip(right).rev();
            tasks.extend(elements.map(|(left, right)| Task::Values(left, right)));
            return Some(None);
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
        ) => return None,
    };
    Some(Some(ordering))
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;

    use super::*;
    use crate::integer::Integer;

    /// TC-312: two equal integers key-compare equal; unequal integers key
    /// in the same order as their value ordering.
    #[trace("TC-312")]
    #[test]
    fn tc_312_integer_keys_follow_value_ordering() {
        let one = Value::Integer(Integer::one());
        let two = Value::Integer(Integer::one().add(&Integer::one()));
        assert_eq!(compare_keys(&one, &one), Some(Ordering::Equal));
        assert_eq!(compare_keys(&one, &two), Some(Ordering::Less));
    }

    /// TC-313: comparing values of unrelated types (a checked program never
    /// produces this) returns `None`, not a panic.
    #[trace("TC-313")]
    #[test]
    fn tc_313_mismatched_leaf_types_return_none() {
        let boolean = Value::Boolean(true);
        let integer = Value::Integer(Integer::one());
        assert_eq!(compare_keys(&boolean, &integer), None);
    }

    /// TC-297 (FR-089-AC-6): a population pair has no key in the kernel.
    #[trace("TC-297", "FR-089-AC-6")]
    #[test]
    fn compare_keys_yields_no_key_for_a_population_pair() {
        use crate::identity::PopulationId;

        fn digest(byte: u8) -> [u8; 32] {
            let mut bytes = [0_u8; 32];
            bytes[31] = byte;
            bytes
        }

        let left = Value::Population(PopulationId::from_digest(digest(1)));
        let right = Value::Population(PopulationId::from_digest(digest(2)));
        assert_eq!(compare_keys(&left, &right), None);
    }

    /// TC-349 (M-2): a same-type `Value::Float` pair has no key, matching
    /// its exclusion from `=` in `crate::equality`'s leaf match.
    #[trace("TC-349")]
    #[test]
    fn tc_349_float_pair_has_no_key() {
        use crate::ieee::IeeeValue;

        let left = Value::Float(IeeeValue::binary64(0x3ff0_0000_0000_0000));
        let right = Value::Float(IeeeValue::binary64(0x3ff0_0000_0000_0000));
        assert_eq!(compare_keys(&left, &right), None);
    }

    /// FND-003 (SR-511): the kernel leaf carries no declaration to guard
    /// on, so two members of *different* enum declarations that happen to
    /// share a rank must still be told apart by `VariantId` -- they are not
    /// equal under `crate::equality`'s leaf match. Mutation proof: dropping
    /// the `left.variant() != right.variant()` guard back to a bare
    /// `left.rank().cmp(&right.rank())` makes this return
    /// `Some(Ordering::Equal)` instead of `None`.
    #[trace("TC-409")]
    #[test]
    fn compare_keys_refuses_same_rank_different_declaration_members() {
        use crate::identity::VariantId;
        use crate::value::EnumMember;

        fn digest(byte: u8) -> [u8; 32] {
            let mut bytes = [0_u8; 32];
            bytes[31] = byte;
            bytes
        }

        let left = Value::Enum(EnumMember::new(VariantId::from_digest(digest(1)), 0));
        let right = Value::Enum(EnumMember::new(VariantId::from_digest(digest(2)), 0));
        assert_eq!(compare_keys(&left, &right), None);
    }
}
