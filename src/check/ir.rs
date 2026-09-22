// SPDX-License-Identifier: AGPL-3.0-or-later
//! The typed, name-resolved expression tree that checking produces and the
//! evaluator runs.

use super::refusal::Location;
use crate::value::collection::CollectionType;
use crate::value::composite::{Value, ValueType};
use crate::value::decimal::DecimalType;
use crate::value::equality::{CheckedEquality, EqualityOperand, EqualityOperator};
use crate::value::node::NodeKey;
use crate::value::numeric::{ArithmeticOperator, OrderingOperator};
use crate::value::rational::RationalDomain;
use qsl_foundation::absence::AbsenceMode;
use quire_exact::{CollectionKind, IntegerInterval};
use std::collections::BTreeSet;

/// FR-062/FR-065: a checked function-application node's identity (ADR-013
/// O-04), minted by [`super::family::mint_call_identity`]. Aliased from the
/// pre-existing local `NodeKey` (a same-name, unrelated composite-type
/// identity this module already imports for `Tuple`/`Record`) so the two are
/// never confused at a call site.
pub(crate) use quire_exact::NodeKey as FamilyNodeKey;

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

/// One [`DispatchTable`] entry (FR-151): a linked candidate's own function
/// indices in the same checked package.
#[derive(Clone, Debug)]
pub struct DispatchCandidate {
    /// The candidate operation body's index in the package's function list.
    pub body: usize,
    /// The candidate's effective precondition's index, when it, or a
    /// redefinition ancestor it disjoins with, declares one. This is the
    /// *runtime*-evaluated function: Boolean absorption (`true ∨ X = true`)
    /// means an absent own clause makes this `None` even when an ancestor
    /// still contributes a clause, so this alone underclaims the FR-146
    /// call-graph edges (TC-196 D08) — see
    /// [`precondition_clauses`](Self::precondition_clauses) for those.
    pub precondition: Option<usize>,
    /// Every authored (non-absent) precondition clause function index
    /// reachable from this candidate's own precondition or any redefinition
    /// ancestor's effective precondition, regardless of runtime
    /// short-circuiting: the full static FR-146 dispatch call-graph edge set
    /// FR-151 requires ("an edge... to every precondition clause of every
    /// candidate's effective precondition, which the call evaluates").
    pub precondition_clauses: Vec<usize>,
}

/// One FR-151 dispatch table, checked and ready for the evaluator: the
/// receiver's most-specific runtime type mapped to its linked candidate, plus
/// the table's own distinct-candidate count for `dispatch.select`'s charge
/// (`value-accounting.md`: `c` is "the number of linked candidates of the
/// called effective operation"). Built by the caller (the `crate::model`
/// bridge) before checking, from `crate::model::dispatch::link_dispatch`'s
/// own `DeclarationKey`-keyed table translated into this package's own function
/// indices; the checker only threads it through unchanged.
#[derive(Clone, Debug)]
pub struct DispatchTable {
    entries: Vec<(NodeKey, DispatchCandidate)>,
    candidate_count: u64,
}

impl DispatchTable {
    /// Build a table from its linked entries and total candidate count.
    pub fn new(entries: Vec<(NodeKey, DispatchCandidate)>, candidate_count: u64) -> Self {
        Self {
            entries,
            candidate_count,
        }
    }

    /// The linked candidate for the receiver's most-specific runtime type.
    pub(crate) fn linked_for(&self, subtype: &NodeKey) -> Option<&DispatchCandidate> {
        self.entries
            .iter()
            .find(|(key, _)| key == subtype)
            .map(|(_, candidate)| candidate)
    }

    /// The table's own distinct-candidate count, for `dispatch.select`.
    pub(crate) fn candidate_count(&self) -> u64 {
        self.candidate_count
    }

    /// Every entry, in the order this table carries them.
    pub(crate) fn entries(&self) -> &[(NodeKey, DispatchCandidate)] {
        &self.entries
    }

    /// Every distinct function index this table can reach: every entry's
    /// body and every clause in its full static effective-precondition
    /// ancestry, deduplicated, ascending. These are the FR-146 dispatch
    /// call-graph edges a dispatched call through this table contributes,
    /// from every candidate, not only the one a particular receiver's
    /// runtime type would select, and regardless of runtime short-circuiting
    /// (TC-196 D08: both candidates' static ancestries reach a cycle even
    /// though one candidate's runtime-evaluated precondition is absent).
    pub(crate) fn callees(&self) -> BTreeSet<usize> {
        let mut callees = BTreeSet::new();
        for (_, candidate) in &self.entries {
            callees.insert(candidate.body);
            callees.extend(candidate.precondition_clauses.iter().copied());
        }
        callees
    }
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
        /// FR-062/FR-065: content-addressed identity, minted once at check
        /// from the call's parsed structure (not from `function`, which is
        /// a position-dependent index -- see
        /// [`super::family::mint_call_identity`]'s doc).
        identity: FamilyNodeKey,
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
    /// `allInstances<T>(p)` (FR-153). `T`, `N` and the bound `[0,N]` are
    /// exactly this node's own checked `value_type`
    /// (`ValueType::Collection`), never restated here.
    AllInstances {
        population: Box<Node>,
    },
    /// `lookup<T>(p, r) absent m` (FR-153). `T` is exactly this node's own
    /// checked `value_type` (a bare `Reference<T>` for
    /// `undefined`/`refused`, an `Option<Reference<T>>` for `empty`), never
    /// restated here.
    Lookup {
        population: Box<Node>,
        reference: Box<Node>,
        absence: AbsenceMode,
    },
    /// `receiver.member(args)` (FR-151), resolved at check time to one
    /// checked dispatch table (identified by `table`, an index into the
    /// package's own `dispatch_tables`) and the exact call site that
    /// produced it (`operation`, an index into the package's own
    /// `dispatch_operations` — never re-derived at runtime by searching
    /// `dispatch_operations` for a `table` match, which picks the wrong
    /// operation's `member` when two call sites share a table), selected at
    /// runtime by the receiver's most-specific type (TC-196 D06).
    Dispatch {
        receiver: Box<Node>,
        table: usize,
        operation: usize,
        arguments: Vec<Node>,
    },
    /// `pre(e)` (FR-153): evaluate `e` with `allInstances`/`lookup`
    /// underneath it reading the invocation pre population. Identity-typed:
    /// this node's `value_type` is always exactly its operand's.
    Pre(Box<Node>),
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
            | NodeKind::Size(operand)
            | NodeKind::Pre(operand) => vec![operand],
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
            NodeKind::AllInstances { population } => vec![population],
            NodeKind::Lookup {
                population,
                reference,
                ..
            } => vec![population, reference],
            NodeKind::Dispatch {
                receiver,
                arguments,
                ..
            } => {
                let mut children: Vec<&Node> = vec![receiver];
                children.extend(arguments);
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

    /// FR-062-AC-2/FR-065-AC-3: every function-application occurrence in
    /// this subtree, as (checked identity, source location) -- the source
    /// half of the occurrence-keyed source map `PackageDeclarations::check`
    /// builds. Reads each `NodeKind::Call`'s own identity and location
    /// fields; mints nothing and re-derives no span.
    pub(crate) fn call_occurrences(&self) -> Vec<(FamilyNodeKey, Location)> {
        self.descendants()
            .into_iter()
            .filter_map(|node| match &node.kind {
                NodeKind::Call { identity, .. } => Some((*identity, node.location.clone())),
                _ => None,
            })
            .collect()
    }
}
