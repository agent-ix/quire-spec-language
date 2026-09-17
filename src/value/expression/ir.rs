// SPDX-License-Identifier: AGPL-3.0-or-later
//! The typed, name-resolved expression tree that checking produces and the
//! evaluator runs.

use super::super::collection::{CollectionKind, CollectionType};
use super::super::composite::{Value, ValueType};
use super::super::decimal::DecimalType;
use super::super::equality::{CheckedEquality, EqualityOperand, EqualityOperator};
use super::super::integer::IntegerInterval;
use super::super::node::NodeKey;
use super::super::numeric::{ArithmeticOperator, OrderingOperator};
use super::super::rational::RationalDomain;
use super::refusal::Location;

/// A local slot of one function frame or checked expression.
pub(crate) type Slot = usize;

/// One typed node.
#[derive(Clone, Debug)]
pub(crate) struct Node {
    pub(crate) kind: NodeKind,
    pub(crate) value_type: ValueType,
    pub(crate) location: Location,
}

/// A connective with a skippable right operand.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Connective {
    And,
    Or,
    Implies,
}

/// An integer arithmetic operator.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Arithmetic {
    Add,
    Subtract,
    Multiply,
}

/// Which values an ordering compares.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum OrderedKind {
    Integers,
    Rationals,
    Decimals,
    Enums,
    /// Text of one profile, by the FR-141 profile order.
    Texts,
    /// Quantities of the identical unit, by FR-142 root value.
    Quantities,
}

/// One record slot in declaration order.
#[derive(Clone, Debug)]
pub(crate) enum RecordSlot {
    Absent,
    Null,
    Present(Box<Node>),
}

/// A one-binder query that visits every occurrence or stops early.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Visit {
    Map,
    Filter,
    Forall,
    Exists,
    Count,
    /// `sum<N>` over integer summands.
    Sum,
}

/// A collection property a kind conversion can discard.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum CollectionProperty {
    /// `order`.
    Order,
    /// `uniqueness`.
    Uniqueness,
    /// `multiplicity`.
    Multiplicity,
}

impl CollectionProperty {
    /// The property spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Order => "order",
            Self::Uniqueness => "uniqueness",
            Self::Multiplicity => "multiplicity",
        }
    }

    fn of(kind: CollectionKind) -> &'static [Self] {
        match kind {
            CollectionKind::Sequence => &[Self::Order, Self::Multiplicity],
            CollectionKind::Set => &[Self::Uniqueness],
            CollectionKind::Bag => &[Self::Multiplicity],
            CollectionKind::OrderedSet => &[Self::Order, Self::Uniqueness],
        }
    }

    /// The static FR-145 loss of converting `source` to `target`, in the
    /// order `order`, `uniqueness`, `multiplicity`.
    pub(crate) fn discarded(source: CollectionKind, target: CollectionKind) -> Vec<Self> {
        let kept = Self::of(target);
        let mut discarded: Vec<Self> = Self::of(source)
            .iter()
            .copied()
            .filter(|property| !kept.contains(property))
            .collect();
        discarded.sort();
        discarded
    }
}

/// `CollectionLoss { discarded }` of one `convert` in the checked expression.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct CollectionLoss {
    /// The `convert` expression.
    pub location: Location,
    /// The discarded properties in the order `order`, `uniqueness`,
    /// `multiplicity`.
    pub discarded: Vec<CollectionProperty>,
}

#[derive(Clone, Debug)]
pub(crate) enum NodeKind {
    Literal(Value),
    /// An integer operand admitted into `Int[..]`: a static range obligation
    /// and an uncharged runtime membership check.
    Coerce(Box<Node>, IntegerInterval),
    Local(Slot),
    Let {
        slot: Slot,
        value: Box<Node>,
        body: Box<Node>,
    },
    If {
        condition: Box<Node>,
        then: Box<Node>,
        otherwise: Box<Node>,
    },
    Arithmetic(Arithmetic, Box<Node>, Box<Node>),
    Negate(Box<Node>),
    /// Integer `/` producing `Rational[..]`.
    Divide {
        left: Box<Node>,
        right: Box<Node>,
        domain: RationalDomain,
    },
    /// FR-044 `Rational[..]` arithmetic into `domain`.
    Rational {
        operator: ArithmeticOperator,
        left: Box<Node>,
        right: Box<Node>,
        domain: RationalDomain,
    },
    /// Unary `-` of a `Rational[..]` into `domain`.
    RationalNegate(Box<Node>, RationalDomain),
    /// FR-140 decimal arithmetic into `target`.
    Decimal {
        operator: ArithmeticOperator,
        left: Box<Node>,
        right: Box<Node>,
        target: DecimalType,
    },
    /// Unary `-` of a decimal into `target`.
    DecimalNegate(Box<Node>, DecimalType),
    /// FR-148 IEEE arithmetic of one width under the omitted, strict `exact`
    /// rounding spelling.
    Ieee(ArithmeticOperator, Box<Node>, Box<Node>),
    /// FR-142 quantity arithmetic; the result unit is the node type's.
    Quantity(ArithmeticOperator, Box<Node>, Box<Node>),
    /// An ordinary FR-140 conversion of a rational or decimal into `target`,
    /// with its loss record.
    ConvertDecimal(Box<Node>, DecimalType),
    Order(OrderingOperator, OrderedKind, Box<Node>, Box<Node>),
    Equality(EqualityOperator, Box<CheckedEquality>, Box<Node>, Box<Node>),
    Connective(Connective, Box<Node>, Box<Node>),
    Not(Box<Node>),
    /// A record field slot; an optional field projects to `Option<T>`.
    Field {
        operand: Box<Node>,
        index: usize,
        optional: bool,
    },
    /// `deref(r).f` of a model object attribute.
    Attribute {
        reference: Box<Node>,
        name: String,
        optional: bool,
    },
    Present(Box<Node>),
    Value(Box<Node>),
    Call {
        function: usize,
        arguments: Vec<Node>,
    },
    Tuple {
        declaration: NodeKey,
        arguments: Vec<Node>,
    },
    Record {
        declaration: NodeKey,
        slots: Vec<RecordSlot>,
    },
    Collection {
        collection_type: CollectionType,
        elements: Vec<Node>,
    },
    ConvertCollection {
        target: CollectionType,
        operand: Box<Node>,
    },
    ConvertScalar(EqualityOperand, Box<Node>),
    IeeeToRational(Box<Node>, RationalDomain),
    Query {
        visit: Visit,
        slot: Slot,
        source: Box<Node>,
        body: Box<Node>,
    },
    Flatten(Box<Node>),
    Fold {
        accumulator: Slot,
        binder: Slot,
        source: Box<Node>,
        step: Box<Node>,
        /// `None` for `reduce`.
        identity: Option<Box<Node>>,
    },
    Size(Box<Node>),
    Contains(Box<Node>, Box<Node>),
}

impl Node {
    /// Direct children in evaluation order.
    pub(crate) fn children(&self) -> Vec<&Node> {
        match &self.kind {
            NodeKind::Literal(_) | NodeKind::Local(_) => Vec::new(),
            NodeKind::Coerce(operand, _)
            | NodeKind::Negate(operand)
            | NodeKind::Not(operand)
            | NodeKind::Field { operand, .. }
            | NodeKind::Attribute {
                reference: operand, ..
            }
            | NodeKind::Present(operand)
            | NodeKind::Value(operand)
            | NodeKind::ConvertCollection { operand, .. }
            | NodeKind::ConvertScalar(_, operand)
            | NodeKind::IeeeToRational(operand, _)
            | NodeKind::RationalNegate(operand, _)
            | NodeKind::DecimalNegate(operand, _)
            | NodeKind::ConvertDecimal(operand, _)
            | NodeKind::Flatten(operand)
            | NodeKind::Size(operand) => vec![operand],
            NodeKind::Let { value, body, .. } => vec![value, body],
            NodeKind::If {
                condition,
                then,
                otherwise,
            } => vec![condition, then, otherwise],
            NodeKind::Arithmetic(_, left, right)
            | NodeKind::Divide { left, right, .. }
            | NodeKind::Rational { left, right, .. }
            | NodeKind::Decimal { left, right, .. }
            | NodeKind::Ieee(_, left, right)
            | NodeKind::Quantity(_, left, right)
            | NodeKind::Order(_, _, left, right)
            | NodeKind::Equality(_, _, left, right)
            | NodeKind::Connective(_, left, right)
            | NodeKind::Contains(left, right) => vec![left, right],
            NodeKind::Call { arguments, .. } | NodeKind::Tuple { arguments, .. } => {
                arguments.iter().collect()
            }
            NodeKind::Record { slots, .. } => slots
                .iter()
                .filter_map(|slot| match slot {
                    RecordSlot::Present(node) => Some(&**node),
                    RecordSlot::Absent | RecordSlot::Null => None,
                })
                .collect(),
            NodeKind::Collection { elements, .. } => elements.iter().collect(),
            NodeKind::Query { source, body, .. } => vec![source, body],
            NodeKind::Fold {
                source,
                step,
                identity,
                ..
            } => {
                let mut children: Vec<&Node> = vec![source];
                children.extend(identity.as_deref());
                children.push(step);
                children
            }
        }
    }

    /// Every node of the tree in pre-order, without host recursion.
    pub(crate) fn descendants(&self) -> Vec<&Node> {
        let mut visited = Vec::new();
        let mut pending = vec![self];
        while let Some(node) = pending.pop() {
            visited.push(node);
            pending.extend(node.children().into_iter().rev());
        }
        visited
    }

    /// Every `convert` loss in pre-order.
    pub(crate) fn losses(&self) -> Vec<CollectionLoss> {
        self.descendants()
            .into_iter()
            .filter_map(|node| match &node.kind {
                NodeKind::ConvertCollection { target, operand } => match &operand.value_type {
                    ValueType::Collection(source) => Some(CollectionLoss {
                        location: node.location.clone(),
                        discarded: CollectionProperty::discarded(source.kind(), target.kind()),
                    }),
                    _ => None,
                },
                _ => None,
            })
            .collect()
    }

    /// Every `deref(r).f` location, whose target existence is a runtime input
    /// requirement.
    pub(crate) fn dereferences(&self) -> Vec<Location> {
        self.descendants()
            .into_iter()
            .filter(|node| matches!(node.kind, NodeKind::Attribute { .. }))
            .map(|node| node.location.clone())
            .collect()
    }
}
