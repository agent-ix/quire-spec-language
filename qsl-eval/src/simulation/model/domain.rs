// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-120 "Applications and transition identities": the finite value domain
//! of a declared type, indexed rather than materialized so a caller walks a
//! product of domains one element at a time.

use quire_exact::{
    from_admitted, from_admitted_slots, CollectionKind, CollectionType, EnumMember, FieldValue,
    Integer, NodeKey, ObjectReference, OptionValue, Presence, UnionMember, UnionValue, Value,
    ValueType,
};
use quire_semantic_value::declaration::{
    CompositeDeclaration, CompositeShape, TypeEnvironment, UnionMemberDeclaration,
};

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
        | ValueType::BoundedQuantity(_)
        | ValueType::Map(_)
        | ValueType::Text(_)
        | ValueType::String
        | ValueType::Uuid
        | ValueType::Timestamp
        | ValueType::Enum(_)
        | ValueType::Composite(_)
        | ValueType::Population(_) => false,
    }
}

/// The finite space of a template's attempt type (ADR-027 ST-11): a type FR-120
/// enumerates, an enum, a record with optional fields, or a record or tuple of
/// such, indexed in declaration order with the last field moving fastest and
/// an absent optional field first.
enum Space {
    Plain(Domain),
    Enum(Vec<Value>),
    Composite {
        declaration: NodeKey,
        fields: Vec<(bool, Space)>,
    },
    /// A union: its members in declaration order, each with the spaces of
    /// its payload positions.
    Union(Vec<(UnionMember, Vec<Space>)>),
}

impl Space {
    /// The space of `value_type`; `None` for a type with no finite space
    /// here (a recursive record, or a field FR-120 does not enumerate).
    fn of(
        value_type: &ValueType,
        objects: &[ObjectReference],
        types: &TypeEnvironment,
        visiting: &mut Vec<NodeKey>,
    ) -> Option<Self> {
        match value_type {
            ValueType::Enum(shape) => Some(Self::Enum(
                shape
                    .variants()
                    .enumerate()
                    .map(|(rank, variant)| {
                        u32::try_from(rank)
                            .ok()
                            .map(|rank| Value::Enum(EnumMember::new(variant, rank)))
                    })
                    .collect::<Option<Vec<Value>>>()?,
            )),
            ValueType::Composite(declaration) => {
                if visiting.contains(declaration) {
                    return None;
                }
                let members: Vec<(bool, &ValueType)> = match types
                    .composite(*declaration)
                    .map(CompositeDeclaration::shape)?
                {
                    CompositeShape::Record(fields) => fields
                        .iter()
                        .map(|field| (field.presence() == Presence::Optional, field.value_type()))
                        .collect(),
                    CompositeShape::Tuple(positions) => {
                        positions.iter().map(|position| (false, position)).collect()
                    }
                    CompositeShape::Union(declared) => {
                        visiting.push(*declaration);
                        let spaces = declared
                            .iter()
                            .map(|member| {
                                let verified = member.member()?.clone();
                                let positions = member
                                    .positions()
                                    .iter()
                                    .map(|position| Self::of(position, objects, types, visiting))
                                    .collect::<Option<Vec<_>>>()?;
                                Some((verified, positions))
                            })
                            .collect::<Option<Vec<_>>>();
                        visiting.pop();
                        return spaces.map(Self::Union);
                    }
                };
                visiting.push(*declaration);
                let fields = members
                    .into_iter()
                    .map(|(optional, member)| {
                        Self::of(member, objects, types, visiting).map(|space| (optional, space))
                    })
                    .collect::<Option<Vec<_>>>();
                visiting.pop();
                Some(Self::Composite {
                    declaration: *declaration,
                    fields: fields?,
                })
            }
            other => Domain::of(other, objects, types).map(Self::Plain),
        }
    }

    /// How many values the space holds, saturating at `u128::MAX`.
    fn len(&self) -> u128 {
        match self {
            Self::Plain(domain) => domain.len(),
            Self::Enum(values) => values.len() as u128,
            Self::Union(members) => members.iter().fold(0u128, |total, (_, positions)| {
                total.saturating_add(
                    positions
                        .iter()
                        .fold(1u128, |product, space| product.saturating_mul(space.len())),
                )
            }),
            Self::Composite { fields, .. } => {
                fields.iter().fold(1u128, |total, (optional, space)| {
                    total.saturating_mul(space.len().saturating_add(u128::from(*optional)))
                })
            }
        }
    }

    /// The value at `index`; `None` past the end.
    fn nth(&self, index: u128) -> Option<Value> {
        match self {
            Self::Plain(domain) => domain.nth(index),
            Self::Enum(values) => values.get(usize::try_from(index).ok()?).cloned(),
            Self::Union(members) => {
                let mut remaining = index;
                for (member, positions) in members {
                    let count = positions
                        .iter()
                        .fold(1u128, |product, space| product.saturating_mul(space.len()));
                    if remaining >= count {
                        remaining -= count;
                        continue;
                    }
                    let mut payload = Vec::with_capacity(positions.len());
                    for space in positions.iter().rev() {
                        let radix = space.len();
                        payload.push(space.nth(remaining.checked_rem(radix)?)?);
                        remaining = remaining.checked_div(radix)?;
                    }
                    payload.reverse();
                    return Some(UnionValue::from_admitted(member.clone(), payload));
                }
                None
            }
            Self::Composite {
                declaration,
                fields,
            } => {
                if index >= self.len() {
                    return None;
                }
                let mut remaining = index;
                let mut slots = Vec::with_capacity(fields.len());
                for (optional, space) in fields.iter().rev() {
                    let radix = space.len().saturating_add(u128::from(*optional));
                    let digit = remaining.checked_rem(radix)?;
                    remaining = remaining.checked_div(radix)?;
                    slots.push(match (optional, digit.checked_sub(1)) {
                        (true, None) => FieldValue::Absent,
                        (true, Some(inner)) => FieldValue::Present(space.nth(inner)?),
                        (false, _) => FieldValue::Present(space.nth(digit)?),
                    });
                }
                slots.reverse();
                Some(from_admitted_slots(*declaration, slots.into_boxed_slice()))
            }
        }
    }
}

/// Whether [`composite_values`] and [`Domain`] together enumerate `value_type`:
/// a type FR-120 enumerates, or an enum, record, tuple or union of finite
/// fields (ADR-027 ST-11: the records of a template's attempt type).
pub(crate) fn enumerable_in(value_type: &ValueType, types: &TypeEnvironment) -> bool {
    match value_type {
        ValueType::Composite(_) | ValueType::Enum(_) => {
            Space::of(value_type, &[], types, &mut Vec::new()).is_some()
        }
        other => enumerable(other),
    }
}

/// The child-index paths (the classifier's: record field, tuple position,
/// flattened union payload position, each from 0) of the leaves of
/// `value_type` this module cannot enumerate; the type's own path when no
/// leaf is to blame.
pub(crate) fn unenumerable_paths(value_type: &ValueType, types: &TypeEnvironment) -> Vec<Vec<u32>> {
    fn walk(
        value_type: &ValueType,
        types: &TypeEnvironment,
        path: &mut Vec<u32>,
        visiting: &mut Vec<NodeKey>,
        found: &mut Vec<Vec<u32>>,
    ) {
        match value_type {
            ValueType::Enum(_) => {}
            ValueType::Composite(declaration) => {
                if visiting.contains(declaration) {
                    return;
                }
                let Some(declared) = types.composite(*declaration) else {
                    found.push(path.clone());
                    return;
                };
                let members: Vec<&ValueType> = match declared.shape() {
                    CompositeShape::Record(fields) => {
                        fields.iter().map(|field| field.value_type()).collect()
                    }
                    CompositeShape::Tuple(positions) => positions.iter().collect(),
                    CompositeShape::Union(members) => members
                        .iter()
                        .flat_map(UnionMemberDeclaration::positions)
                        .collect(),
                };
                visiting.push(*declaration);
                for (index, member) in members.into_iter().enumerate() {
                    let Ok(index) = u32::try_from(index) else {
                        break;
                    };
                    path.push(index);
                    walk(member, types, path, visiting, found);
                    path.pop();
                }
                visiting.pop();
            }
            other => {
                if Domain::of(other, &[], types).is_none() {
                    found.push(path.clone());
                }
            }
        }
    }
    let mut found = Vec::new();
    walk(
        value_type,
        types,
        &mut Vec::new(),
        &mut Vec::new(),
        &mut found,
    );
    if found.is_empty() && !enumerable_in(value_type, types) {
        found.push(Vec::new());
    }
    found
}

/// Every value of the composite type `value_type`, in index order; `charge`
/// is called once per value. `None` when the type has no finite space.
pub(super) fn composite_values<E>(
    value_type: &ValueType,
    objects: &[ObjectReference],
    types: &TypeEnvironment,
    mut charge: impl FnMut() -> Result<(), E>,
) -> Result<Option<Vec<Value>>, E> {
    let Some(space) = Space::of(value_type, objects, types, &mut Vec::new()) else {
        return Ok(None);
    };
    let mut values = Vec::new();
    let mut index = 0u128;
    while let Some(value) = space.nth(index) {
        charge()?;
        values.push(value);
        index = index.saturating_add(1);
    }
    Ok(Some(values))
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
    /// A random parameter's support, in declared order (FR-187, ADR-024
    /// PM-1): its finite domain whatever its declared type.
    Listed(Vec<Value>),
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
            Self::Listed(values) => values.len() as u128,
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
            Self::Listed(values) => values.get(usize::try_from(index).ok()?).cloned(),
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

    /// The type environment of `Inner { e: Boolean, o?: Boolean }` and
    /// `Outer { inner: Inner, flag: Boolean }`.
    fn nested() -> (TypeEnvironment, NodeKey, NodeKey) {
        use quire_semantic_value::declaration::FieldDeclaration;
        let inner = NodeKey::from_digest([2; 32]);
        let outer = NodeKey::from_digest([3; 32]);
        let declarations = [
            CompositeDeclaration::new(
                inner,
                "Inner",
                CompositeShape::Record(vec![
                    FieldDeclaration::new("e", ValueType::Boolean, Presence::Required),
                    FieldDeclaration::new("o", ValueType::Boolean, Presence::Optional),
                ]),
            ),
            CompositeDeclaration::new(
                outer,
                "Outer",
                CompositeShape::Record(vec![
                    FieldDeclaration::new("inner", ValueType::Composite(inner), Presence::Required),
                    FieldDeclaration::new("flag", ValueType::Boolean, Presence::Required),
                ]),
            ),
        ];
        (
            TypeEnvironment::new(declarations, []).expect("the declarations admit"),
            inner,
            outer,
        )
    }

    /// ADR-027 ST-11: the records of an attempt type range over its whole
    /// finite domain: an optional field (absent first) and a nested record of
    /// finite fields enumerate, each combination once.
    #[test]
    fn an_attempt_type_of_finite_fields_enumerates_every_record() {
        let (types, inner, outer) = nested();
        let mut charged = 0;
        let values = |declaration, charged: &mut u32| {
            composite_values::<()>(&ValueType::Composite(declaration), &[], &types, || {
                *charged += 1;
                Ok(())
            })
            .expect("no stop")
            .expect("a finite domain")
        };
        assert_eq!(values(inner, &mut charged).len(), 6);
        assert_eq!(values(outer, &mut charged).len(), 12);
        assert_eq!(charged, 18);
        let all = values(outer, &mut charged);
        let distinct: std::collections::BTreeSet<String> = all
            .iter()
            .map(|value| {
                format!(
                    "{:?}",
                    super::super::encode::typed_value_enc(value, &types, &[])
                )
            })
            .collect();
        assert_eq!(distinct.len(), 12);
    }

    /// ADR-027 ST-11: a union over finite members enumerates every member
    /// with every combination of its payload, in declaration order, and is
    /// written as FR-120's union form.
    #[test]
    fn a_union_attempt_type_enumerates_every_member() {
        use quire_exact::{Identifier, UnionMember};
        use quire_semantic_value::declaration::UnionMemberDeclaration;
        let union = NodeKey::from_digest([5; 32]);
        let member = |name: &str, key: u8, positions: Vec<ValueType>| {
            UnionMemberDeclaration::from_verified(
                UnionMember::from_admitted(
                    union,
                    NodeKey::from_digest([key; 32]),
                    Identifier::new(name).expect("an identifier"),
                ),
                positions,
            )
        };
        let declaration = CompositeDeclaration::new(
            union,
            "Choice",
            CompositeShape::Union(vec![
                member("Flag", 6, vec![ValueType::Boolean]),
                member("Pair", 7, vec![ValueType::Boolean, int(0, 1)]),
                member("Nothing", 8, vec![]),
            ]),
        );
        let types = TypeEnvironment::new([declaration], []).expect("the declaration admits");
        assert!(enumerable_in(&ValueType::Composite(union), &types));
        let values = composite_values::<()>(&ValueType::Composite(union), &[], &types, || Ok(()))
            .expect("no stop")
            .expect("a finite domain");
        assert_eq!(values.len(), 2 + 4 + 1);
        let written = quire_canonical::to_vec(
            &super::super::encode::typed_value_enc(&values[1], &types, &[]),
            quire_canonical::Limits::new(u64::MAX),
        )
        .expect("encodes");
        assert_eq!(
            String::from_utf8(written).expect("UTF-8"),
            format!(
                r#"{{"components":[{{"type":"boolean","value":true}}],"member":"Flag","name":"{}","type":"union"}}"#,
                "05".repeat(32)
            )
        );
    }

    /// FR-120-AC-25 / TC-1270: an admitted model variant union adds the
    /// nullary case's one value to each value of a finite Boolean payload.
    #[trace("TC-1270", "FR-120-AC-25")]
    #[test]
    fn a_variant_union_adds_nullary_and_payload_case_domains() {
        use quire_exact::{Identifier, UnionMember};
        use quire_semantic_value::declaration::UnionMemberDeclaration;
        let union = NodeKey::from_digest([9; 32]);
        let empty = UnionMember::from_admitted(
            union,
            NodeKey::from_digest([10; 32]),
            Identifier::new("Empty").expect("an identifier"),
        );
        let enabled = UnionMember::from_admitted(
            union,
            NodeKey::from_digest([11; 32]),
            Identifier::new("Enabled").expect("an identifier"),
        );
        let declaration = CompositeDeclaration::new(
            union,
            "Choice",
            CompositeShape::Union(vec![
                UnionMemberDeclaration::from_verified(empty.clone(), vec![]),
                UnionMemberDeclaration::from_verified(enabled.clone(), vec![ValueType::Boolean]),
            ]),
        );
        let types = TypeEnvironment::new([declaration], []).expect("the declaration admits");
        assert!(enumerable_in(&ValueType::Composite(union), &types));
        let values = composite_values::<()>(&ValueType::Composite(union), &[], &types, || Ok(()))
            .expect("no stop")
            .expect("the union has a finite domain");
        assert_eq!(values.len(), 3);
        let Value::Union(empty_value) = &values[0] else {
            panic!("the nullary case is first")
        };
        assert_eq!(empty_value.member().variant(), empty.variant());
        assert!(empty_value.payload().is_empty());
        for (index, expected) in [(1, false), (2, true)] {
            let Value::Union(enabled_value) = &values[index] else {
                panic!("the payload case follows the nullary case")
            };
            assert_eq!(enabled_value.member().variant(), enabled.variant());
            assert!(
                matches!(enabled_value.payload(), [Value::Boolean(value)] if *value == expected)
            );
        }
    }

    /// FR-120's record form, written exactly: `type`, the declaration's node
    /// id as `name`, and one `{name, value}` entry per declared field with an
    /// absent optional field `null`.
    #[test]
    fn a_record_value_is_written_in_fr_120_s_record_form() {
        let (types, inner, _) = nested();
        let first = composite_values::<()>(&ValueType::Composite(inner), &[], &types, || Ok(()))
            .expect("no stop")
            .expect("a finite domain")
            .remove(0);
        let written = quire_canonical::to_vec(
            &super::super::encode::typed_value_enc(&first, &types, &[]),
            quire_canonical::Limits::new(u64::MAX),
        )
        .expect("encodes");
        let name = "02".repeat(32);
        assert_eq!(
            String::from_utf8(written).expect("UTF-8"),
            format!(
                r#"{{"fields":[{{"name":"e","value":{{"type":"boolean","value":false}}}},{{"name":"o","value":null}}],"name":"{name}","type":"record"}}"#
            )
        );
    }
}
