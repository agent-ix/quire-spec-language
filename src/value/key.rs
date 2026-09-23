// SPDX-License-Identifier: AGPL-3.0-or-later
//! The FR-144 type-owned total canonical key.
//!
//! Every type that admits FR-149 `=` has a key, and two values of one type
//! have equal keys exactly when they are FR-149 equal. The key fixes set and
//! bag canonical order and visiting order; it is not an ordering operator.
//! Comparison is iterative, so value depth never reaches the host stack.

use std::cmp::Ordering;

use quire_exact::{FieldValue, Value};

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
/// never produces.
pub(crate) fn compare_keys(left: &Value, right: &Value) -> Option<Ordering> {
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
        // FR-144-AC-9: declaration position for an ordered enum, case-byte
        // order otherwise. `rank` (ADR-013 O-14/OQ-D) already *is* that
        // canonical-list index either way -- the shape that admitted this
        // value fixed it there (declaration order when ordered, case-sorted
        // order otherwise) -- so comparing ranks numerically reproduces
        // FR-144's rule for both cases at once, with no declaration lookup.
        //
        // FND-003 (SR-511): a rank alone does not name a declaration. Two
        // members of *different* enum declarations can share a rank without
        // being FR-149 equal (they have different `VariantId`s). When ranks
        // are equal but the `VariantId`s differ, the pair is not one keyed
        // type, so this returns `None` rather than reporting a false
        // `Equal` -- the same contract violation `crate::equality`'s leaf
        // match already refuses on a `VariantId` mismatch.
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
            | Value::Reference(_)
            | Value::Option(_)
            | Value::Composite(_)
            | Value::Collection(_)
            | Value::Population(_),
            _,
        ) => return None,
    };
    Some(Some(ordering))
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;
    use quire_exact::{EnumMember, NodeKey};

    use super::*;
    use crate::value::enumeration::mint_variant_id;

    fn node_key(byte: u8) -> NodeKey {
        let mut bytes = [0_u8; 32];
        bytes[31] = byte;
        NodeKey::from_digest(bytes)
    }

    /// FND-003 (SR-511): two members of *different* enum declarations that
    /// share a rank are not FR-149 equal -- they have different
    /// `VariantId`s -- so `compare_keys` must not report them as one keyed
    /// type. Mutation proof: dropping the `left.variant() != right.variant()`
    /// guard back to a bare `left.rank().cmp(&right.rank())` makes this
    /// return `Some(Ordering::Equal)` instead of `None`.
    #[trace("TC-409")]
    #[test]
    fn compare_keys_refuses_same_rank_different_declaration_members() {
        let left_variant = mint_variant_id(node_key(1), "READY");
        let right_variant = mint_variant_id(node_key(2), "READY");
        assert_ne!(left_variant, right_variant);

        let left = Value::Enum(EnumMember::new(left_variant, 0));
        let right = Value::Enum(EnumMember::new(right_variant, 0));
        assert_eq!(compare_keys(&left, &right), None);
    }

    /// Sanity companion to the FND-003 test above: two members of *one*
    /// declaration with equal ranks (the same member twice) do key-compare
    /// equal, and members at different ranks order by rank.
    #[trace("TC-409")]
    #[test]
    fn compare_keys_orders_same_declaration_members_by_rank() {
        let declaration = node_key(1);
        let ready = mint_variant_id(declaration, "READY");
        let done = mint_variant_id(declaration, "DONE");

        let ready_value = Value::Enum(EnumMember::new(ready, 0));
        let done_value = Value::Enum(EnumMember::new(done, 1));
        assert_eq!(
            compare_keys(&ready_value, &ready_value),
            Some(Ordering::Equal)
        );
        assert_eq!(
            compare_keys(&ready_value, &done_value),
            Some(Ordering::Less)
        );
    }
}
