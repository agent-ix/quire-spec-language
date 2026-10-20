// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-120 "Applications and transition identities": the finite value domain
//! of a declared type, indexed rather than materialized so a caller walks a
//! product of domains one element at a time.

use quire_exact::{
    from_admitted, CollectionKind, CollectionType, Integer, ObjectReference, OptionValue, Value,
    ValueType,
};
use quire_semantic_value::declaration::TypeEnvironment;

/// Whether this module can enumerate `value_type`: `Boolean`, `Int[..]`,
/// `Option<T>`, `Reference<T>` and a sequence with a declared maximum over
/// one of them. Every other type is an unbounded domain (FR-120 "Bounds").
pub(crate) fn enumerable(value_type: &ValueType) -> bool {
    match value_type {
        ValueType::Boolean | ValueType::Int(_) | ValueType::Reference(_) => true,
        ValueType::Option(payload) => enumerable(payload),
        ValueType::Collection(collection) => {
            collection.kind() == CollectionKind::Sequence
                && collection.bound().is_some()
                && enumerable(collection.element())
        }
        ValueType::Integer
        | ValueType::Rational(_)
        | ValueType::Decimal(_)
        | ValueType::Float(_)
        | ValueType::Quantity(_)
        | ValueType::Text(_)
        | ValueType::String
        | ValueType::Uuid
        | ValueType::Timestamp
        | ValueType::Enum(_)
        | ValueType::Composite(_)
        | ValueType::Population(_) => false,
    }
}

/// The finite domain of one declared type over a set of objects, in
/// FR-120's order: `false` then `true`; the integers ascending; absent then
/// each payload; the objects in the order given; sequences by length then
/// by element index, the first element most significant.
pub(super) enum Domain {
    Boolean,
    Int {
        lower: Integer,
        len: u128,
    },
    Option {
        payload_type: ValueType,
        inner: Box<Domain>,
    },
    Reference(Vec<ObjectReference>),
    Sequence {
        collection: CollectionType,
        element: Box<Domain>,
        min: u64,
        max: u64,
    },
}

impl Domain {
    /// The domain of `value_type`, where a `Reference<T>` ranges over the
    /// members of `objects` whose most-specific type conforms to `T`.
    /// `None` when `value_type` is not [`enumerable`].
    pub(super) fn of(
        value_type: &ValueType,
        objects: &[ObjectReference],
        types: &TypeEnvironment,
    ) -> Option<Self> {
        match value_type {
            ValueType::Boolean => Some(Self::Boolean),
            ValueType::Int(interval) => {
                let span = interval.upper().sub(interval.lower()).add(&Integer::one());
                Some(Self::Int {
                    lower: interval.lower().clone(),
                    len: span.to_u64().map_or(u128::MAX, u128::from),
                })
            }
            ValueType::Option(payload) => Some(Self::Option {
                payload_type: (**payload).clone(),
                inner: Box::new(Self::of(payload, objects, types)?),
            }),
            ValueType::Reference(target) => Some(Self::Reference(
                objects
                    .iter()
                    .filter(|object| types.conforms(object.object_type(), *target))
                    .cloned()
                    .collect(),
            )),
            ValueType::Collection(collection) => {
                let bound = collection.bound()?;
                if collection.kind() != CollectionKind::Sequence {
                    return None;
                }
                Some(Self::Sequence {
                    collection: (**collection).clone(),
                    element: Box::new(Self::of(collection.element(), objects, types)?),
                    min: bound.minimum(),
                    max: bound.maximum(),
                })
            }
            _ => None,
        }
    }

    /// How many values the domain holds, saturating at `u128::MAX`.
    pub(super) fn len(&self) -> u128 {
        match self {
            Self::Boolean => 2,
            Self::Int { len, .. } => *len,
            Self::Option { inner, .. } => inner.len().saturating_add(1),
            Self::Reference(objects) => objects.len() as u128,
            Self::Sequence {
                element, min, max, ..
            } => {
                let base = element.len();
                (*min..=*max).fold(0u128, |total, length| {
                    total.saturating_add(power(base, length))
                })
            }
        }
    }

    /// The value at `index`; `None` past the end.
    pub(super) fn nth(&self, index: u128) -> Option<Value> {
        match self {
            Self::Boolean => match index {
                0 => Some(Value::Boolean(false)),
                1 => Some(Value::Boolean(true)),
                _ => None,
            },
            Self::Int { lower, len } => {
                if index >= *len {
                    return None;
                }
                let offset = u64::try_from(index).ok()?;
                Some(Value::Integer(lower.add(&Integer::from(offset))))
            }
            Self::Option {
                payload_type,
                inner,
            } => match index.checked_sub(1) {
                None => Some(OptionValue::none(payload_type.clone())),
                Some(payload) => Some(OptionValue::from_admitted(
                    payload_type.clone(),
                    Some(inner.nth(payload)?),
                )),
            },
            Self::Reference(objects) => objects
                .get(usize::try_from(index).ok()?)
                .cloned()
                .map(Value::Reference),
            Self::Sequence {
                collection,
                element,
                min,
                max,
            } => {
                let base = element.len();
                let mut remaining = index;
                for length in *min..=*max {
                    let count = power(base, length);
                    if remaining < count {
                        let mut digits = Vec::new();
                        for _ in 0..length {
                            digits.push(remaining % base);
                            remaining /= base;
                        }
                        digits.reverse();
                        let elements = digits
                            .into_iter()
                            .map(|digit| element.nth(digit))
                            .collect::<Option<Vec<Value>>>()?;
                        return Some(from_admitted(collection.clone(), elements));
                    }
                    remaining -= count;
                }
                None
            }
        }
    }
}

/// Every combination of digits `0..radix` for a list of radices, the last
/// digit moving fastest, produced one at a time. No radices is one empty
/// combination; a zero radix is none.
pub(super) struct Odometer {
    radices: Vec<u128>,
    digits: Vec<u128>,
    started: bool,
    exhausted: bool,
}

impl Odometer {
    /// An odometer over `radices`.
    pub(super) fn new(radices: Vec<u128>) -> Self {
        Self {
            digits: vec![0; radices.len()],
            radices,
            started: false,
            exhausted: false,
        }
    }

    /// The next combination, or `None` once every one has been produced.
    pub(super) fn next_digits(&mut self) -> Option<Vec<u128>> {
        if self.exhausted {
            return None;
        }
        if !self.started {
            self.started = true;
            if self.radices.contains(&0) {
                self.exhausted = true;
                return None;
            }
            return Some(self.digits.clone());
        }
        let mut position = self.digits.len();
        loop {
            let Some(previous) = position.checked_sub(1) else {
                self.exhausted = true;
                return None;
            };
            position = previous;
            self.digits[position] += 1;
            if self.digits[position] < self.radices[position] {
                return Some(self.digits.clone());
            }
            self.digits[position] = 0;
        }
    }
}

/// `base` to the `exponent`, saturating at `u128::MAX`.
fn power(base: u128, exponent: u64) -> u128 {
    (0..exponent).fold(1u128, |product, _| product.saturating_mul(base))
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;
    use quire_exact::{CardinalityBound, IntegerInterval};

    use super::*;

    fn types() -> TypeEnvironment {
        TypeEnvironment::new([], []).expect("an empty environment admits")
    }

    fn int(lower: i64, upper: i64) -> ValueType {
        ValueType::Int(
            IntegerInterval::new(Integer::from(lower), Integer::from(upper)).expect("nonempty"),
        )
    }

    fn integer(value: &Value) -> String {
        match value {
            Value::Integer(integer) => integer.to_string(),
            other => panic!("expected an integer, got {other:?}"),
        }
    }

    /// FR-120 "Applications": an `Int[lower, upper]` domain is every integer
    /// in the range, ascending, and `Option<T>` is absent then each value.
    #[trace("TC-471", "FR-120-AC-2")]
    #[test]
    fn integers_ascend_and_an_option_starts_absent() {
        let domain = Domain::of(&int(-1, 1), &[], &types()).expect("Int is enumerable");
        let values: Vec<String> = (0..domain.len())
            .map(|index| integer(&domain.nth(index).expect("in range")))
            .collect();
        assert_eq!(values, ["-1", "0", "1"]);
        assert!(domain.nth(3).is_none());

        let option = Domain::of(&ValueType::option(int(0, 1)), &[], &types()).expect("enumerable");
        assert_eq!(option.len(), 3);
        let Some(Value::Option(none)) = option.nth(0) else {
            panic!("index 0 is an option")
        };
        assert!(none.payload().is_none());
    }

    /// FR-120: a sequence with a declared maximum ranges over every length
    /// from its minimum to its maximum; with `Int[0, 1]` elements and bound
    /// `[0, 2]` that is 1 + 2 + 4 sequences.
    #[trace("TC-471", "FR-120-AC-2")]
    #[test]
    fn a_bounded_sequence_ranges_over_every_length() {
        let sequence = ValueType::collection(CollectionType::new(
            CollectionKind::Sequence,
            int(0, 1),
            Some(CardinalityBound::new(0, 2).expect("bound")),
        ));
        let domain = Domain::of(&sequence, &[], &types()).expect("enumerable");
        assert_eq!(domain.len(), 7);
        let lengths: Vec<usize> = (0..7)
            .map(|index| match domain.nth(index) {
                Some(Value::Collection(collection)) => collection.elements().len(),
                other => panic!("expected a sequence, got {other:?}"),
            })
            .collect();
        assert_eq!(lengths, [0, 1, 1, 2, 2, 2, 2]);
    }

    /// The odometer visits every combination once, last digit fastest.
    #[trace("TC-471", "FR-120-AC-3")]
    #[test]
    fn an_odometer_visits_every_combination_once() {
        let mut odometer = Odometer::new(vec![2, 3]);
        let mut seen = Vec::new();
        while let Some(digits) = odometer.next_digits() {
            seen.push(digits);
        }
        assert_eq!(seen.len(), 6);
        assert_eq!(seen[0], [0, 0]);
        assert_eq!(seen[1], [0, 1]);
        assert_eq!(seen[5], [1, 2]);
        assert_eq!(Odometer::new(vec![]).next_digits(), Some(vec![]));
        assert_eq!(Odometer::new(vec![2, 0]).next_digits(), None);
    }

    /// Every other type is unbounded here.
    #[trace("TC-472", "FR-120-AC-7")]
    #[test]
    fn an_unenumerable_type_has_no_domain() {
        assert!(!enumerable(&ValueType::Integer));
        assert!(Domain::of(&ValueType::String, &[], &types()).is_none());
    }
}
