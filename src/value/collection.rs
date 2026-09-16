// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-144 collection kinds: bounded construction, the type-owned total element
//! key and the canonical representation.
//!
//! Every materialization checks the declared inclusive cardinality bound
//! before a member is retained. The FR-145 queries over these values live in
//! `collection_query`.

use std::sync::Arc;

use super::composite::{Component, ConstructionCause, ConstructionRefusal, Value, ValueType};
use super::equality::decide_equal;
use super::integer::Integer;
use super::rational::Rational;

// SPEC-GAP(119-9): `quire.value.accounting/v1` names no collection
// construction, traversal, membership or retain charge point, so collection
// construction and queries charge nothing themselves: materialization is
// bounded only by the declared cardinality bound, and only caller-supplied
// functions receive the meter. The membership decisions inside construction
// and `count` use the unmetered FR-149 relation.

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

/// An inclusive declared cardinality bound `minimum..maximum`. It counts
/// occurrences for sequences and bags and members for sets and ordered sets.
// SPEC-GAP(119-11): FR-144 lists the bound as a construction input but does
// not say whether it is part of the collection type's identity. It is not:
// `ValueType::Collection` carries only kind and element type, and the bound is
// checked at each materialization.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct CardinalityBound {
    minimum: u64,
    maximum: u64,
}

/// A cardinality bound with `minimum > maximum`.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, thiserror::Error)]
#[error("empty cardinality bound {minimum}..{maximum}")]
pub struct EmptyCardinalityBound {
    /// The declared minimum.
    pub minimum: u64,
    /// The declared maximum.
    pub maximum: u64,
}

impl CardinalityBound {
    /// The inclusive bound `minimum..maximum`.
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
}

/// Why a materialization violates its cardinality bound.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum CardinalityViolation {
    /// No bound was declared.
    Missing,
    /// One more occurrence or member than `maximum` was about to be retained.
    AboveMaximum {
        /// The declared maximum.
        maximum: u64,
    },
    /// Fewer than `minimum` occurrences or members remained.
    BelowMinimum {
        /// The declared minimum.
        minimum: u64,
        /// The counted occurrences or members.
        counted: u64,
    },
}

/// A count as `u64`. Saturation is exact for every bound comparison because no
/// bound exceeds `u64::MAX`.
fn count(length: usize) -> u64 {
    u64::try_from(length).unwrap_or(u64::MAX)
}

/// Bounded materialization of one collection. Nothing is retained past the
/// declared maximum and no value escapes a refused build.
pub(crate) struct CollectionBuilder {
    kind: CollectionKind,
    element_type: ValueType,
    bound: CardinalityBound,
    kept: Vec<Value>,
}

impl CollectionBuilder {
    /// Start a build of at most `occurrences` supplied occurrences. A sequence
    /// or bag whose occurrence count already exceeds the maximum refuses
    /// before anything is allocated.
    pub(crate) fn start(
        kind: CollectionKind,
        element_type: ValueType,
        bound: Option<CardinalityBound>,
        occurrences: usize,
    ) -> Result<Self, CardinalityViolation> {
        let bound = bound.ok_or(CardinalityViolation::Missing)?;
        if !kind.is_unique() && count(occurrences) > bound.maximum {
            return Err(CardinalityViolation::AboveMaximum {
                maximum: bound.maximum,
            });
        }
        let capacity = usize::try_from(bound.maximum)
            .unwrap_or(usize::MAX)
            .min(occurrences);
        Ok(Self {
            kind,
            element_type,
            bound,
            kept: Vec::with_capacity(capacity),
        })
    }

    /// Add one occurrence of the element type. A set or ordered set drops an
    /// occurrence equal to a retained member.
    pub(crate) fn push(&mut self, occurrence: Value) -> Result<(), CardinalityViolation> {
        if self.kind.is_unique() && self.kept.iter().any(|kept| decide_equal(kept, &occurrence)) {
            return Ok(());
        }
        if count(self.kept.len()) >= self.bound.maximum {
            return Err(CardinalityViolation::AboveMaximum {
                maximum: self.bound.maximum,
            });
        }
        self.kept.push(occurrence);
        Ok(())
    }

    /// The collection, if the minimum is met.
    pub(crate) fn finish(self) -> Result<Value, CardinalityViolation> {
        let counted = count(self.kept.len());
        if counted < self.bound.minimum {
            return Err(CardinalityViolation::BelowMinimum {
                minimum: self.bound.minimum,
                counted,
            });
        }
        Ok(Value::Collection(Arc::new(CollectionValue {
            kind: self.kind,
            element_type: self.element_type,
            elements: self.kept.into_boxed_slice(),
        })))
    }
}

/// A collection of one declared element type.
#[derive(Clone, Debug)]
pub struct CollectionValue {
    kind: CollectionKind,
    element_type: ValueType,
    elements: Box<[Value]>,
}

impl CollectionValue {
    /// A collection of `kind` over `element_type` from occurrences in
    /// insertion order, within the declared inclusive `bound`. A set or
    /// ordered set keeps each member's first occurrence under the FR-149
    /// equality relation of `element_type`. A missing or violated bound
    /// refuses with no partial collection.
    pub fn construct(
        kind: CollectionKind,
        element_type: ValueType,
        bound: Option<CardinalityBound>,
        occurrences: Vec<Value>,
    ) -> Result<Value, ConstructionRefusal> {
        if let Some(index) = occurrences
            .iter()
            .position(|occurrence| !element_type.admits(occurrence))
        {
            return Err(ConstructionRefusal {
                component: Component::Element(index),
                cause: ConstructionCause::TypeMismatch,
            });
        }
        let cardinality = |violation| ConstructionRefusal {
            component: Component::Value,
            cause: ConstructionCause::Cardinality(violation),
        };
        let mut builder = CollectionBuilder::start(kind, element_type, bound, occurrences.len())
            .map_err(cardinality)?;
        for occurrence in occurrences {
            builder.push(occurrence).map_err(cardinality)?;
        }
        builder.finish().map_err(cardinality)
    }

    /// The collection kind.
    pub fn kind(&self) -> CollectionKind {
        self.kind
    }

    /// The declared element type.
    pub fn element_type(&self) -> &ValueType {
        &self.element_type
    }

    /// Retained occurrences (members for a set or ordered set) in insertion
    /// order. For a set or bag this order is not semantic.
    pub fn elements(&self) -> &[Value] {
        &self.elements
    }

    /// The FR-144 canonical representation: occurrence order for a sequence
    /// or ordered set; ascending total element key for a set; ascending key
    /// with multiplicity for a bag. A set or bag whose element type has no
    /// total key refuses.
    // SPEC-GAP(119-15): FR-144 names "canonical bytes" but no encoding for a
    // collection, so the canonical representation is returned as typed
    // entries rather than bytes.
    pub fn canonical_form(&self) -> Result<CanonicalCollection, NoTotalElementKey> {
        let single = |value: &Value| CanonicalEntry {
            value: value.clone(),
            multiplicity: Integer::one(),
        };
        let entries = if self.kind.is_ordered() {
            self.elements.iter().map(single).collect()
        } else {
            let mut entries: Vec<CanonicalEntry> = Vec::new();
            for (_, value) in keyed_ascending(&self.element_type, &self.elements)? {
                match entries.last_mut() {
                    Some(last) if decide_equal(&last.value, value) => {
                        last.multiplicity = last.multiplicity.add(&Integer::one());
                    }
                    Some(_) | None => entries.push(single(value)),
                }
            }
            entries
        };
        Ok(CanonicalCollection {
            kind: self.kind,
            entries,
        })
    }
}

/// The FR-144 canonical representation of one collection.
#[derive(Clone, Debug)]
pub struct CanonicalCollection {
    kind: CollectionKind,
    entries: Vec<CanonicalEntry>,
}

impl CanonicalCollection {
    /// The collection kind.
    pub fn kind(&self) -> CollectionKind {
        self.kind
    }

    /// Entries in canonical order.
    pub fn entries(&self) -> &[CanonicalEntry] {
        &self.entries
    }
}

/// One canonical member with its multiplicity (always one outside a bag).
#[derive(Clone, Debug)]
pub struct CanonicalEntry {
    value: Value,
    multiplicity: Integer,
}

impl CanonicalEntry {
    /// The member.
    pub fn value(&self) -> &Value {
        &self.value
    }

    /// Its positive multiplicity.
    pub fn multiplicity(&self) -> &Integer {
        &self.multiplicity
    }
}

/// The element type supplies no total canonical key, so an unordered
/// collection has no canonical order.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, thiserror::Error)]
#[error("the element type has no total canonical key")]
pub struct NoTotalElementKey;

/// Occurrences sorted by ascending total key; ties keep insertion order but
/// are equal members.
pub(crate) fn keyed_ascending<'a>(
    element_type: &ValueType,
    elements: &'a [Value],
) -> Result<Vec<(TotalKey, &'a Value)>, NoTotalElementKey> {
    if !has_total_key(element_type) {
        return Err(NoTotalElementKey);
    }
    let mut keyed = elements
        .iter()
        .map(|value| total_key(value).map(|key| (key, value)))
        .collect::<Option<Vec<_>>>()
        .ok_or(NoTotalElementKey)?;
    keyed.sort_by(|(left, _), (right, _)| left.cmp(right));
    Ok(keyed)
}

/// A type-owned total canonical element key. Within one element type every
/// key has the same variant, and two keys are equal exactly when the FR-149
/// relation holds.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum TotalKey {
    Boolean(bool),
    Number(Rational),
    Text(String),
    Enum(usize),
}

/// Whether `element_type` supplies a total canonical key.
// SPEC-GAP(119-4): FR-144/FR-149 require a "type-owned total canonical key"
// but no type declares one. Boolean (false < true), the exact numbers by
// mathematical value, quantities of one unit by value, text by retained
// UTF-8 bytes (scalar order), and enum members by declaration position have a
// key (SPEC-GAP(119-16): even for an unordered enum); options, composites,
// collections and references do not. This and `total_key` are the only places
// that decide it; the equality plan is decided by `equality::membership_plan`.
pub(crate) fn has_total_key(element_type: &ValueType) -> bool {
    match element_type {
        ValueType::Boolean
        | ValueType::Integer
        | ValueType::Rational
        | ValueType::Decimal(_)
        | ValueType::Quantity(_)
        | ValueType::Text(_)
        | ValueType::Enum(_) => true,
        ValueType::Option(_)
        | ValueType::Composite(_)
        | ValueType::Collection(..)
        | ValueType::Reference(_) => false,
    }
}

/// The total key of one value; see [`has_total_key`].
pub(crate) fn total_key(value: &Value) -> Option<TotalKey> {
    match value {
        Value::Boolean(value) => Some(TotalKey::Boolean(*value)),
        Value::Integer(value) => Some(TotalKey::Number(Rational::from_integer(value.clone()))),
        Value::Rational(value) => Some(TotalKey::Number(value.clone())),
        Value::Decimal(value) => Some(TotalKey::Number(value.normalized().to_rational())),
        Value::Quantity(value) => Some(TotalKey::Number(value.value().clone())),
        Value::Text(value) => Some(TotalKey::Text(value.retained().to_owned())),
        Value::Enum(value) => Some(TotalKey::Enum(value.position())),
        Value::Option(_) | Value::Composite(_) | Value::Collection(_) | Value::Reference(_) => None,
    }
}
