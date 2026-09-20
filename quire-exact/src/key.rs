// SPDX-License-Identifier: AGPL-3.0-or-later
//! The type-owned total canonical key (ported from QSL `value::key`).
//!
//! Every type that admits `=` has a key, and two values of one type have
//! equal keys exactly when they are equal under [`crate::equality`]. The key
//! fixes set and bag canonical order and visiting order; it is not an
//! ordering operator. Comparison is iterative, so value depth never reaches
//! the host stack.
//!
//! One adaptation against the original: `Value::Enum` is a bare
//! [`crate::identity::VariantId`] digest here (ADR-013 T-6), not a
//! declaration-aware `EnumValue` with a position and case. Two enum values
//! of the *same* enum therefore key-order by raw digest bytes rather than by
//! declared position/case order; a declaration-ordered enum's canonical
//! collection order is something the layer that still holds the enum
//! declaration (QSL `model`) must reproduce on top of this, not something
//! this kernel key can give it. Flagged for review alongside the identical
//! cut in `crate::value`'s module doc comment.

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
        (Value::Enum(left), Value::Enum(right)) => left.cmp(right),
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
}
