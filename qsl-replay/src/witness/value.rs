// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-070: the typed concrete value a witness entry or an `Input`
//! assignment carries, [`WitnessValue`].
//!
//! A value is a leaf of any value family or a composite nested to any depth.
//! No walk over it uses the native stack in proportion to its depth
//! (ADR-030 D-1): `Clone`, `PartialEq`, `Debug` and `Drop` are written over
//! an explicit heap stack, as the kernel's own values are.

use std::fmt;

use qsl_foundation::digest::WireNodeId;
use quire_exact::Integer;

/// A quantity's magnitude: its rational or decimal form (FR-070).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum QuantityMagnitude {
    /// A reduced rational magnitude.
    Rational {
        /// The numerator.
        numerator: Integer,
        /// The positive denominator.
        denominator: Integer,
    },
    /// A decimal magnitude, before normalization.
    Decimal {
        /// The retained coefficient.
        coefficient: Integer,
        /// The retained scale.
        scale: Integer,
    },
}

/// One slot of a record field (FR-070): a value, an absent optional field, or
/// a `null` slot.
#[derive(Eq, PartialEq)]
pub enum WitnessSlot {
    /// The field holds a value.
    Present(WitnessValue),
    /// An optional (`?`) field with no value.
    Absent,
    /// A `null` slot.
    Null,
}

/// One declared field of a record value: its name and its slot.
#[derive(Eq, PartialEq)]
pub struct WitnessField {
    /// The declared field identifier.
    pub name: String,
    /// The field's slot.
    pub slot: WitnessSlot,
}

/// One concrete argument value, typed by the binding or assignment that
/// carries it. `Boolean` and `Integer` are the scalar forms of a `Boolean`
/// or integer-typed parameter; every other form is a witness value text's
/// shape (FR-070 "Witness value text"), and an integer inside a value text is
/// an [`Self::ExactInteger`].
///
/// A declaration (`declaration`) is a checked node id; a `member` and a field
/// `name` are declared identifiers. The value checks its own form only: the
/// declared type is checked when the executor converts it (FR-098).
pub enum WitnessValue {
    /// A Boolean value.
    Boolean(bool),
    /// A signed 64-bit integer value.
    Integer(i64),
    /// An exact integer of any size, as a witness value text holds it.
    ExactInteger(Integer),
    /// An enum member.
    Enum {
        /// The enum declaration's node id.
        declaration: WireNodeId,
        /// The member identifier.
        member: String,
    },
    /// A text value: its Unicode scalars.
    Text(String),
    /// A reduced rational with a positive denominator.
    Rational {
        /// The numerator.
        numerator: Integer,
        /// The denominator.
        denominator: Integer,
    },
    /// A decimal, retaining its coefficient and scale.
    Decimal {
        /// The retained coefficient.
        coefficient: Integer,
        /// The retained scale.
        scale: Integer,
    },
    /// A 32-bit IEEE float, by its exact bits.
    Float32(u32),
    /// A 64-bit IEEE float, by its exact bits.
    Float64(u64),
    /// A quantity: a magnitude and its unit.
    Quantity {
        /// The magnitude.
        magnitude: QuantityMagnitude,
        /// The unit's identity digest.
        unit: [u8; 32],
    },
    /// A reference to an object.
    Reference {
        /// The universe identity.
        universe: [u8; 32],
        /// The effective object type identity.
        object_type: [u8; 32],
        /// The object's identity bytes.
        identity: Vec<u8>,
    },
    /// An `Option`: `None` for `none`.
    Option(Option<Box<WitnessValue>>),
    /// A record, one field per declared field in declaration order.
    Record {
        /// The record declaration's node id.
        declaration: WireNodeId,
        /// The fields.
        fields: Vec<WitnessField>,
    },
    /// A tuple, its components in declaration order.
    Tuple {
        /// The tuple declaration's node id.
        declaration: WireNodeId,
        /// The components.
        components: Vec<WitnessValue>,
    },
    /// A union member and its components.
    Union {
        /// The union declaration's node id.
        declaration: WireNodeId,
        /// The member identifier.
        member: String,
        /// The member's components.
        components: Vec<WitnessValue>,
    },
    /// A `Sequence`, in sequence order.
    Sequence(Vec<WitnessValue>),
    /// An `OrderedSet`, in its order.
    OrderedSet(Vec<WitnessValue>),
    /// A `Set`, ascending by each element's value text bytes.
    Set(Vec<WitnessValue>),
    /// A `Bag`, ascending by each element's value text bytes.
    Bag(Vec<WitnessValue>),
}

impl WitnessValue {
    /// The values nested directly in this one, in the order the text writes
    /// them.
    pub(crate) fn children(&self) -> Vec<&WitnessValue> {
        match self {
            Self::Option(inner) => inner.iter().map(Box::as_ref).collect(),
            Self::Record { fields, .. } => fields
                .iter()
                .filter_map(|field| match &field.slot {
                    WitnessSlot::Present(value) => Some(value),
                    WitnessSlot::Absent | WitnessSlot::Null => None,
                })
                .collect(),
            Self::Tuple { components, .. } | Self::Union { components, .. } => {
                components.iter().collect()
            }
            Self::Sequence(elements)
            | Self::OrderedSet(elements)
            | Self::Set(elements)
            | Self::Bag(elements) => elements.iter().collect(),
            Self::Boolean(_)
            | Self::Integer(_)
            | Self::ExactInteger(_)
            | Self::Enum { .. }
            | Self::Text(_)
            | Self::Rational { .. }
            | Self::Decimal { .. }
            | Self::Float32(_)
            | Self::Float64(_)
            | Self::Quantity { .. }
            | Self::Reference { .. } => Vec::new(),
        }
    }

    /// A copy of this value's own data with `children` in place of its
    /// nested values, taken in [`Self::children`] order.
    fn with_children(&self, children: Vec<WitnessValue>) -> WitnessValue {
        let mut children = children.into_iter();
        match self {
            Self::Boolean(value) => Self::Boolean(*value),
            Self::Integer(value) => Self::Integer(*value),
            Self::ExactInteger(value) => Self::ExactInteger(value.clone()),
            Self::Enum {
                declaration,
                member,
            } => Self::Enum {
                declaration: *declaration,
                member: member.clone(),
            },
            Self::Text(text) => Self::Text(text.clone()),
            Self::Rational {
                numerator,
                denominator,
            } => Self::Rational {
                numerator: numerator.clone(),
                denominator: denominator.clone(),
            },
            Self::Decimal { coefficient, scale } => Self::Decimal {
                coefficient: coefficient.clone(),
                scale: scale.clone(),
            },
            Self::Float32(bits) => Self::Float32(*bits),
            Self::Float64(bits) => Self::Float64(*bits),
            Self::Quantity { magnitude, unit } => Self::Quantity {
                magnitude: magnitude.clone(),
                unit: *unit,
            },
            Self::Reference {
                universe,
                object_type,
                identity,
            } => Self::Reference {
                universe: *universe,
                object_type: *object_type,
                identity: identity.clone(),
            },
            Self::Option(inner) => {
                Self::Option(inner.as_ref().and_then(|_| children.next().map(Box::new)))
            }
            Self::Record {
                declaration,
                fields,
            } => Self::Record {
                declaration: *declaration,
                fields: fields
                    .iter()
                    .map(|field| WitnessField {
                        name: field.name.clone(),
                        slot: match &field.slot {
                            WitnessSlot::Present(_) => children
                                .next()
                                .map_or(WitnessSlot::Absent, WitnessSlot::Present),
                            WitnessSlot::Absent => WitnessSlot::Absent,
                            WitnessSlot::Null => WitnessSlot::Null,
                        },
                    })
                    .collect(),
            },
            Self::Tuple { declaration, .. } => Self::Tuple {
                declaration: *declaration,
                components: children.collect(),
            },
            Self::Union {
                declaration,
                member,
                ..
            } => Self::Union {
                declaration: *declaration,
                member: member.clone(),
                components: children.collect(),
            },
            Self::Sequence(_) => Self::Sequence(children.collect()),
            Self::OrderedSet(_) => Self::OrderedSet(children.collect()),
            Self::Set(_) => Self::Set(children.collect()),
            Self::Bag(_) => Self::Bag(children.collect()),
        }
    }

    /// Whether this value's own data, apart from its nested values, equals
    /// `other`'s: the same form, leaf data, declarations, names and slot
    /// states.
    fn same_data(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Boolean(a), Self::Boolean(b)) => a == b,
            (Self::Integer(a), Self::Integer(b)) => a == b,
            (Self::ExactInteger(a), Self::ExactInteger(b)) => a == b,
            (
                Self::Enum {
                    declaration: da,
                    member: ma,
                },
                Self::Enum {
                    declaration: db,
                    member: mb,
                },
            ) => da == db && ma == mb,
            (Self::Text(a), Self::Text(b)) => a == b,
            (
                Self::Rational {
                    numerator: na,
                    denominator: da,
                },
                Self::Rational {
                    numerator: nb,
                    denominator: db,
                },
            ) => na == nb && da == db,
            (
                Self::Decimal {
                    coefficient: ca,
                    scale: sa,
                },
                Self::Decimal {
                    coefficient: cb,
                    scale: sb,
                },
            ) => ca == cb && sa == sb,
            (Self::Float32(a), Self::Float32(b)) => a == b,
            (Self::Float64(a), Self::Float64(b)) => a == b,
            (
                Self::Quantity {
                    magnitude: ma,
                    unit: ua,
                },
                Self::Quantity {
                    magnitude: mb,
                    unit: ub,
                },
            ) => ma == mb && ua == ub,
            (
                Self::Reference {
                    universe: ua,
                    object_type: ta,
                    identity: ia,
                },
                Self::Reference {
                    universe: ub,
                    object_type: tb,
                    identity: ib,
                },
            ) => ua == ub && ta == tb && ia == ib,
            (Self::Option(a), Self::Option(b)) => a.is_some() == b.is_some(),
            (
                Self::Record {
                    declaration: da,
                    fields: fa,
                },
                Self::Record {
                    declaration: db,
                    fields: fb,
                },
            ) => {
                da == db
                    && fa.len() == fb.len()
                    && fa.iter().zip(fb).all(|(a, b)| {
                        a.name == b.name
                            && matches!(
                                (&a.slot, &b.slot),
                                (WitnessSlot::Present(_), WitnessSlot::Present(_))
                                    | (WitnessSlot::Absent, WitnessSlot::Absent)
                                    | (WitnessSlot::Null, WitnessSlot::Null)
                            )
                    })
            }
            (
                Self::Tuple {
                    declaration: da,
                    components: ca,
                },
                Self::Tuple {
                    declaration: db,
                    components: cb,
                },
            ) => da == db && ca.len() == cb.len(),
            (
                Self::Union {
                    declaration: da,
                    member: ma,
                    components: ca,
                },
                Self::Union {
                    declaration: db,
                    member: mb,
                    components: cb,
                },
            ) => da == db && ma == mb && ca.len() == cb.len(),
            (Self::Sequence(a), Self::Sequence(b))
            | (Self::OrderedSet(a), Self::OrderedSet(b))
            | (Self::Set(a), Self::Set(b))
            | (Self::Bag(a), Self::Bag(b)) => a.len() == b.len(),
            (
                Self::Boolean(_)
                | Self::Integer(_)
                | Self::ExactInteger(_)
                | Self::Enum { .. }
                | Self::Text(_)
                | Self::Rational { .. }
                | Self::Decimal { .. }
                | Self::Float32(_)
                | Self::Float64(_)
                | Self::Quantity { .. }
                | Self::Reference { .. }
                | Self::Option(_)
                | Self::Record { .. }
                | Self::Tuple { .. }
                | Self::Union { .. }
                | Self::Sequence(_)
                | Self::OrderedSet(_)
                | Self::Set(_)
                | Self::Bag(_),
                _,
            ) => false,
        }
    }

    /// Move this value's nested values onto `out`, leaving none behind.
    fn take_children(&mut self, out: &mut Vec<WitnessValue>) {
        match self {
            Self::Option(inner) => out.extend(inner.take().map(|boxed| *boxed)),
            Self::Record { fields, .. } => {
                for field in fields.drain(..) {
                    if let WitnessSlot::Present(value) = field.slot {
                        out.push(value);
                    }
                }
            }
            Self::Tuple { components, .. } | Self::Union { components, .. } => {
                out.append(components);
            }
            Self::Sequence(elements)
            | Self::OrderedSet(elements)
            | Self::Set(elements)
            | Self::Bag(elements) => out.append(elements),
            Self::Boolean(_)
            | Self::Integer(_)
            | Self::ExactInteger(_)
            | Self::Enum { .. }
            | Self::Text(_)
            | Self::Rational { .. }
            | Self::Decimal { .. }
            | Self::Float32(_)
            | Self::Float64(_)
            | Self::Quantity { .. }
            | Self::Reference { .. } => {}
        }
    }

    /// The number of values in this tree, this one included.
    pub fn node_count(&self) -> u64 {
        let mut count = 0_u64;
        let mut pending = vec![self];
        while let Some(value) = pending.pop() {
            count = count.saturating_add(1);
            pending.extend(value.children());
        }
        count
    }

    /// The form's name, without any of its content.
    fn kind(&self) -> &'static str {
        match self {
            Self::Boolean(_) => "boolean",
            Self::Integer(_) | Self::ExactInteger(_) => "integer",
            Self::Enum { .. } => "enum",
            Self::Text(_) => "text",
            Self::Rational { .. } => "rational",
            Self::Decimal { .. } => "decimal",
            Self::Float32(_) => "float32",
            Self::Float64(_) => "float64",
            Self::Quantity { .. } => "quantity",
            Self::Reference { .. } => "reference",
            Self::Option(_) => "option",
            Self::Record { .. } => "record",
            Self::Tuple { .. } => "tuple",
            Self::Union { .. } => "union",
            Self::Sequence(_) => "sequence",
            Self::OrderedSet(_) => "ordered-set",
            Self::Set(_) => "set",
            Self::Bag(_) => "bag",
        }
    }
}

/// A copy of the whole tree, built from an explicit heap stack.
impl Clone for WitnessValue {
    fn clone(&self) -> Self {
        struct Frame<'a> {
            node: &'a WitnessValue,
            children: Vec<&'a WitnessValue>,
            next: usize,
            done: Vec<WitnessValue>,
        }
        fn frame(node: &WitnessValue) -> Frame<'_> {
            Frame {
                node,
                children: node.children(),
                next: 0,
                done: Vec::new(),
            }
        }
        let mut stack = vec![frame(self)];
        loop {
            let Some(top) = stack.last_mut() else {
                // The loop returns the root's copy before the stack empties.
                return Self::Boolean(false);
            };
            if let Some(child) = top.children.get(top.next).copied() {
                top.next += 1;
                stack.push(frame(child));
                continue;
            }
            let Some(finished) = stack.pop() else {
                return Self::Boolean(false);
            };
            let built = finished.node.with_children(finished.done);
            match stack.last_mut() {
                Some(parent) => parent.done.push(built),
                None => return built,
            }
        }
    }
}

/// Dropping walks the tree from an explicit heap stack, so a value of any
/// depth drops on a small fixed native stack.
impl Drop for WitnessValue {
    fn drop(&mut self) {
        let mut pending = Vec::new();
        self.take_children(&mut pending);
        while let Some(mut value) = pending.pop() {
            value.take_children(&mut pending);
        }
    }
}

/// Redacted (FR-073): the form and the node count, never any content.
impl fmt::Debug for WitnessValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WitnessValue")
            .field("kind", &self.kind())
            .field("nodes", &self.node_count())
            .finish()
    }
}

impl fmt::Debug for WitnessField {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("WitnessField")
    }
}

impl fmt::Debug for WitnessSlot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Present(_) => "Present",
            Self::Absent => "Absent",
            Self::Null => "Null",
        })
    }
}

impl WitnessValue {
    /// Whether this tree equals `other`, walked from an explicit heap stack.
    fn tree_eq(&self, other: &Self) -> bool {
        let mut pending = vec![(self, other)];
        while let Some((left, right)) = pending.pop() {
            if !left.same_data(right) {
                return false;
            }
            let (left_children, right_children) = (left.children(), right.children());
            if left_children.len() != right_children.len() {
                return false;
            }
            pending.extend(left_children.into_iter().zip(right_children));
        }
        true
    }
}

/// Equality walks both trees from an explicit heap stack.
impl PartialEq for WitnessValue {
    fn eq(&self, other: &Self) -> bool {
        self.tree_eq(other)
    }
}

impl Eq for WitnessValue {}
