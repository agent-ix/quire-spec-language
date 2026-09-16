// SPDX-License-Identifier: AGPL-3.0-or-later
//! The minimal collection value representation consumed by FR-149 equality.
//!
//! This slice owns only element typing and the uniqueness normalization that
//! set equality depends on. Collection construction accounting, canonical
//! serialization keys and the FR-144/FR-145 algebra are later slices
//! (`Remaining work: #119`).

use std::sync::Arc;

use super::composite::{Component, ConstructionCause, ConstructionRefusal, Value, ValueType};
use super::equality::decide_equal;

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
    fn is_unique(self) -> bool {
        matches!(self, Self::Set | Self::OrderedSet)
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
    /// A collection of `kind` over `element_type` from elements in insertion
    /// order. A set or ordered set keeps each member's first occurrence under
    /// the FR-149 equality relation of `element_type`.
    pub fn construct(
        kind: CollectionKind,
        element_type: ValueType,
        elements: Vec<Value>,
    ) -> Result<Value, ConstructionRefusal> {
        if let Some(index) = elements
            .iter()
            .position(|element| !element_type.admits(element))
        {
            return Err(ConstructionRefusal {
                component: Component::Element(index),
                cause: ConstructionCause::TypeMismatch,
            });
        }
        let elements = if kind.is_unique() {
            let mut unique: Vec<Value> = Vec::with_capacity(elements.len());
            for element in elements {
                if !unique.iter().any(|kept| decide_equal(kept, &element)) {
                    unique.push(element);
                }
            }
            unique
        } else {
            elements
        };
        Ok(Value::Collection(Arc::new(Self {
            kind,
            element_type,
            elements: elements.into_boxed_slice(),
        })))
    }

    /// The collection kind.
    pub fn kind(&self) -> CollectionKind {
        self.kind
    }

    /// The declared element type.
    pub fn element_type(&self) -> &ValueType {
        &self.element_type
    }

    /// Retained elements in occurrence order.
    pub fn elements(&self) -> &[Value] {
        &self.elements
    }
}
