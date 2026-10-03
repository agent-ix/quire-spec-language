// SPDX-License-Identifier: AGPL-3.0-or-later
//! The typed, name-resolved expression tree that checking produces and the
//! evaluator runs.

use qsl_foundation::absence::AbsenceMode;
use quire_exact::DecimalType;
use quire_exact::EffectiveId;
use quire_exact::NodeKey;
use quire_exact::RoundingMode;
use quire_exact::{ArithmeticOperator, OrderingOperator};
use quire_exact::{CollectionKind, CollectionType, IntegerInterval, RationalDomain};
use quire_exact::{Value, ValueType};
use quire_semantic_value::declaration::{
    CheckedEquality, EqualityOperand, EqualityOperator, FieldRef,
};
use quire_semantic_value::location::Location;
use std::collections::BTreeSet;
use std::convert::Infallible;
use std::fmt;
use std::ops::ControlFlow;

/// A local slot of one function frame or checked expression.
pub type Slot = usize;

/// The state a model read in a state clause observes (FR-104
/// "Observations of reads"): `current` in an invariant, `pre` in a
/// precondition and `post` in a postcondition, except inside `pre(e)`,
/// where reads of `self` and reads through references obtained inside `e`
/// observe `pre`.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum Observation {
    /// An invariant's `at current` state.
    Current,
    /// The operation's pre-state.
    Pre,
    /// The operation's post-state.
    Post,
}

/// The typed index of one node of a [`CheckedBody`]: its position there.
///
/// An id names a node of the body that minted it. Ids are positions, so
/// they stay valid in a clone of that body.
#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct NodeId(usize);

impl NodeId {
    /// The node's position in its body: the number of nodes stored before
    /// it.
    pub fn index(self) -> usize {
        self.0
    }
}

impl fmt::Debug for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{}", self.0)
    }
}

/// One typed node: what it computes, with each operand named by its
/// [`NodeId`] in the same [`CheckedBody`].
///
/// Only `check` builds or edits a `Node`: its fields are private to `check`,
/// and every other layer reads a stored node through [`CheckedNode`]. A node
/// from outside `check` therefore always went through checking. Building
/// one from outside `check` does not compile:
/// ```compile_fail,E0451
/// use qsl_semantics::check::{Node, NodeKind};
/// use quire_semantic_value::location::Location;
/// fn forge(kind: NodeKind, location: Location) -> Node {
///     Node { kind, value_type: quire_exact::ValueType::Boolean, location }
/// }
/// ```
/// Nor does retyping a clone of a checked node:
/// ```compile_fail,E0616
/// use qsl_semantics::check::Node;
/// fn retype(checked: &Node) -> Node {
///     let mut node = checked.clone();
///     node.value_type = quire_exact::ValueType::Boolean;
///     node
/// }
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    pub(in crate::check) kind: NodeKind,
    pub(in crate::check) value_type: ValueType,
    pub(in crate::check) location: Location,
}

impl Node {
    /// What the node computes.
    pub fn kind(&self) -> &NodeKind {
        &self.kind
    }

    /// The node's checked static type.
    pub fn value_type(&self) -> &ValueType {
        &self.value_type
    }

    /// The node's source location.
    pub fn location(&self) -> &Location {
        &self.location
    }
}

/// One checked expression: an arena of [`Node`]s, each stored after its
/// operands, with the root last (ADR-030 D-4.3, FR-258).
///
/// Operands are named by [`NodeId`], so `Clone`, `PartialEq`, `Debug` and
/// `Drop` run over one flat vector and never recurse, however deep the
/// expression. Only `check` builds one. A node no path from the root
/// reaches is never read.
#[derive(Clone, Debug, PartialEq)]
pub struct CheckedBody {
    /// Never empty: the root is the last node.
    nodes: Vec<Node>,
}

impl CheckedBody {
    /// The root node.
    pub fn root(&self) -> CheckedNode<'_> {
        CheckedNode {
            body: self,
            id: NodeId(self.nodes.len().saturating_sub(1)),
        }
    }

    /// The node `id` names.
    ///
    /// # Panics
    ///
    /// When `id` is from another body and past this one's last node.
    pub fn node(&self, id: NodeId) -> CheckedNode<'_> {
        assert!(id.0 < self.nodes.len(), "{id:?} names no node of this body");
        CheckedNode { body: self, id }
    }

    /// How many nodes the body stores.
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Whether the body stores no node: never, since it holds its root.
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
}

/// The nodes one typing pass builds, each pushed once a node naming it as
/// an operand is built, so each is stored after its operands.
#[derive(Debug, Default)]
pub(in crate::check) struct BodyBuilder {
    nodes: Vec<Node>,
}

impl BodyBuilder {
    /// Store `node` after every node pushed so far, and return its id.
    pub(in crate::check) fn push(&mut self, node: Node) -> NodeId {
        self.nodes.push(node);
        NodeId(self.nodes.len() - 1)
    }

    /// The pushed node `id` names, when it names one.
    pub(in crate::check) fn get(&self, id: NodeId) -> Option<&Node> {
        self.nodes.get(id.0)
    }

    /// The body whose root is `root`, stored after every pushed node.
    pub(in crate::check) fn finish(mut self, root: Node) -> CheckedBody {
        self.nodes.push(root);
        CheckedBody { nodes: self.nodes }
    }
}

/// A node of a [`CheckedBody`] together with the body it lives in, so its
/// operands can be reached.
#[derive(Clone, Copy)]
pub struct CheckedNode<'a> {
    body: &'a CheckedBody,
    id: NodeId,
}

impl<'a> CheckedNode<'a> {
    /// The node itself.
    fn node(self) -> &'a Node {
        &self.body.nodes[self.id.0]
    }

    /// This node's id in its body.
    pub fn id(self) -> NodeId {
        self.id
    }

    /// The body this node lives in.
    pub fn body(self) -> &'a CheckedBody {
        self.body
    }

    /// What the node computes.
    pub fn kind(self) -> &'a NodeKind {
        &self.node().kind
    }

    /// The node's checked static type.
    pub fn value_type(self) -> &'a ValueType {
        &self.node().value_type
    }

    /// The node's source location.
    pub fn location(self) -> &'a Location {
        &self.node().location
    }

    /// The node `id` names in this node's body.
    ///
    /// # Panics
    ///
    /// When `id` is from another body and past this one's last node.
    pub fn at(self, id: NodeId) -> CheckedNode<'a> {
        self.body.node(id)
    }

    /// Direct operands in evaluation order.
    pub fn children(self) -> Vec<CheckedNode<'a>> {
        self.kind()
            .children()
            .into_iter()
            .map(|id| self.at(id))
            .collect()
    }

    /// Every node of the tree under this one in pre-order, without host
    /// recursion.
    pub(crate) fn descendants(self) -> Vec<CheckedNode<'a>> {
        struct Collect<'n> {
            visited: Vec<CheckedNode<'n>>,
        }
        impl<'n> quire_walk::Walk for Collect<'n> {
            type Node = CheckedNode<'n>;
            type Frame = ();
            type Stop = Infallible;

            fn enter(
                &mut self,
                node: CheckedNode<'n>,
                children: &mut quire_walk::Children<'_, CheckedNode<'n>>,
            ) -> ControlFlow<Infallible, ()> {
                self.visited.push(node);
                children.extend(node.children());
                ControlFlow::Continue(())
            }

            fn exit(&mut self, (): ()) -> ControlFlow<Infallible> {
                ControlFlow::Continue(())
            }
        }
        let mut collect = Collect {
            visited: Vec::new(),
        };
        let ControlFlow::Continue(()) = quire_walk::walk(&mut collect, self);
        collect.visited
    }

    /// Every `convert` loss in pre-order.
    pub(crate) fn losses(self) -> Vec<CollectionLoss> {
        self.descendants()
            .into_iter()
            .filter_map(|node| match node.kind() {
                NodeKind::ConvertCollection { target, operand } => {
                    match node.at(*operand).value_type() {
                        ValueType::Collection(source) => Some(CollectionLoss {
                            location: node.location().clone(),
                            discarded: CollectionProperty::discarded(source.kind(), target.kind()),
                        }),
                        _ => None,
                    }
                }
                _ => None,
            })
            .collect()
    }

    /// Every `deref(r).f` location, whose target existence is a runtime input
    /// requirement.
    pub(crate) fn dereferences(self) -> Vec<Location> {
        self.descendants()
            .into_iter()
            .filter(|node| matches!(node.kind(), NodeKind::Attribute { .. }))
            .map(|node| node.location().clone())
            .collect()
    }

    /// Every function index a `NodeKind::Call` in this subtree names, in
    /// pre-order: the callees whose keys this subtree's own key hashes
    /// (FR-093 `Call` row).
    pub(crate) fn callees(self) -> Vec<usize> {
        self.descendants()
            .into_iter()
            .filter_map(|node| match node.kind() {
                NodeKind::Call { function, .. } => Some(*function),
                _ => None,
            })
            .collect()
    }
}

impl fmt::Debug for CheckedNode<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CheckedNode")
            .field("id", &self.id)
            .field("node", self.node())
            .finish()
    }
}

/// A connective with a skippable right operand.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Connective {
    /// `and`.
    And,
    /// `or`.
    Or,
    /// `implies`.
    Implies,
}

/// An integer arithmetic operator.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Arithmetic {
    /// `+`.
    Add,
    /// Binary `-`.
    Subtract,
    /// `*`.
    Multiply,
}

/// Which values an ordering compares.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OrderedKind {
    /// Integers, by value.
    Integers,
    /// Rationals, by value.
    Rationals,
    /// Decimals, by value.
    Decimals,
    /// Members of one ordered enum, by declared rank.
    Enums,
    /// Text of one profile, by the FR-141 profile order.
    Texts,
    /// Quantities of the identical unit, by FR-142 root value.
    Quantities,
}

/// A checked literal's value. Two literals are equal when their canonical
/// keys are: [`Value`] has no structural equality of its own. A value with
/// no canonical key (one that holds a float, which ADR-013 O-13 excludes
/// from `=`) is equal to another exactly when the two print identically, bit
/// pattern for bit pattern, so a literal always equals its own clone.
#[derive(Clone, Debug)]
pub struct CheckedLiteral(pub Value);

impl PartialEq for CheckedLiteral {
    fn eq(&self, other: &Self) -> bool {
        match quire_exact::compare_keys(&self.0, &other.0) {
            Some(order) => order == std::cmp::Ordering::Equal,
            None => format!("{:?}", self.0) == format!("{:?}", other.0),
        }
    }
}

/// One record slot in declaration order.
#[derive(Clone, Debug, PartialEq)]
pub enum RecordSlot {
    /// An omitted optional field.
    Absent,
    /// An optional field given `null`.
    Null,
    /// A field given a value.
    Present(NodeId),
}

/// A one-binder query that visits every occurrence or stops early.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Visit {
    /// `map`: the body's value at every occurrence.
    Map,
    /// `filter`: the occurrences the body admits.
    Filter,
    /// `forall`: stops at the first occurrence the body refuses.
    Forall,
    /// `exists`: stops at the first occurrence the body admits.
    Exists,
    /// `count`: how many occurrences the body admits.
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
    /// Keyed by the receiver type's effective identity (ADR-013 O-05), the
    /// type component a runtime `Reference<T>` value carries.
    entries: Vec<(EffectiveId, DispatchCandidate)>,
    candidate_count: u64,
}

impl DispatchTable {
    /// Build a table from its linked entries and total candidate count.
    pub fn new(entries: Vec<(EffectiveId, DispatchCandidate)>, candidate_count: u64) -> Self {
        Self {
            entries,
            candidate_count,
        }
    }

    /// The linked candidate for the receiver's most-specific runtime type.
    pub fn linked_for(&self, subtype: &EffectiveId) -> Option<&DispatchCandidate> {
        self.entries
            .iter()
            .find(|(key, _)| key == subtype)
            .map(|(_, candidate)| candidate)
    }

    /// The table's own distinct-candidate count, for `dispatch.select`.
    pub fn candidate_count(&self) -> u64 {
        self.candidate_count
    }

    /// Every entry, in the order this table carries them.
    pub(crate) fn entries(&self) -> &[(EffectiveId, DispatchCandidate)] {
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

#[derive(Clone, Debug, PartialEq)]
/// What a checked [`Node`] computes. Each operand is a [`NodeId`] in the
/// node's own [`CheckedBody`].
///
/// `#[cfg(seam_probe)]` adds one further probe-only variant (ADR-012 §5.1
/// S3, FR-063): under `--cfg seam_probe`, every closed `match` over
/// this type below this module's own [`NodeKind::children`] becomes
/// non-exhaustive (`E0004`) unless it has its own probe arm. Never
/// constructed outside the probe build.
pub enum NodeKind {
    /// A literal value.
    Literal(CheckedLiteral),
    /// An integer operand admitted into `Int[..]`: a static range obligation
    /// and an uncharged runtime membership check.
    Coerce(NodeId, IntegerInterval),
    /// A read of a local slot.
    Local(Slot),
    /// `let slot = value in body`.
    Let {
        /// The bound slot.
        slot: Slot,
        /// The bound value.
        value: NodeId,
        /// The body the binding scopes.
        body: NodeId,
    },
    /// `if condition then then else otherwise`.
    If {
        /// The condition.
        condition: NodeId,
        /// The branch taken when the condition holds.
        then: NodeId,
        /// The branch taken otherwise.
        otherwise: NodeId,
    },
    /// Integer arithmetic.
    Arithmetic(Arithmetic, NodeId, NodeId),
    /// Unary integer `-`.
    Negate(NodeId),
    /// Integer `/` producing `Rational[..]`.
    Divide {
        /// The dividend.
        left: NodeId,
        /// The divisor.
        right: NodeId,
        /// The result domain.
        domain: RationalDomain,
    },
    /// FR-044 `Rational[..]` arithmetic into `domain`.
    Rational {
        /// The operator.
        operator: ArithmeticOperator,
        /// The left operand.
        left: NodeId,
        /// The right operand.
        right: NodeId,
        /// The result domain.
        domain: RationalDomain,
    },
    /// Unary `-` of a `Rational[..]` into `domain`.
    RationalNegate(NodeId, RationalDomain),
    /// FR-140 decimal arithmetic into `target`.
    Decimal {
        /// The operator.
        operator: ArithmeticOperator,
        /// The left operand.
        left: NodeId,
        /// The right operand.
        right: NodeId,
        /// The result decimal type.
        target: DecimalType,
    },
    /// Unary `-` of a decimal into `target`.
    DecimalNegate(NodeId, DecimalType),
    /// FR-148 IEEE arithmetic of one width under the rounding mode the
    /// operand types carry (FR-091-OQ-4).
    Ieee(ArithmeticOperator, RoundingMode, NodeId, NodeId),
    /// FR-142 quantity arithmetic; the result unit is the node type's.
    Quantity(ArithmeticOperator, NodeId, NodeId),
    /// An ordinary FR-140 conversion of a rational or decimal into `target`,
    /// with its loss record.
    ConvertDecimal(NodeId, DecimalType),
    /// An ordering comparison of two values of one [`OrderedKind`].
    Order(OrderingOperator, OrderedKind, NodeId, NodeId),
    /// An FR-149 equality comparison under its checked schedule.
    Equality(EqualityOperator, Box<CheckedEquality>, NodeId, NodeId),
    /// A boolean connective with a skippable right operand.
    Connective(Connective, NodeId, NodeId),
    /// `not`.
    Not(NodeId),
    /// A record field slot; an optional field projects to `Option<T>`.
    Field {
        /// The record.
        operand: NodeId,
        /// The field's declaration-order slot.
        index: usize,
        /// Whether the field is optional.
        optional: bool,
    },
    /// `deref(r).f` of a model object attribute.
    Attribute {
        /// The object reference.
        reference: NodeId,
        /// The field `f` resolves to in the reference's static type's
        /// effective attribute set: its declaring type and name.
        field: FieldRef,
        /// Whether the attribute is optional.
        optional: bool,
        /// Whether the source wrote an explicit `deref(r).f`, rather than
        /// `self.f`/`r.f` in a state clause (`check/check/typing.rs`'s
        /// `Frame::Field`/`Frame::Attribute` split): FR-107's own accounting
        /// rule charges `model.deref` once per `deref(...)`, never for a
        /// bare `self.f`/`r.f` read, though both give this same node kind
        /// (SR-750 FND-008 round 2).
        derefed: bool,
    },
    /// Whether an `Option` operand holds a value.
    Present(NodeId),
    /// The payload of an `Option` operand.
    Value(NodeId),
    /// A call of a function of this package. Its checked identity is the
    /// key of the `expression` node the FR-093 lowering builds for it
    /// (`check::lowering`), minted after every callee is keyed.
    Call {
        /// The callee's function index in this package.
        function: usize,
        /// The arguments, in parameter order.
        arguments: Vec<NodeId>,
    },
    /// A call of an imported library's function (ADR-015 D-5). Its
    /// callee lowers to a `dependency_reference` term.
    ImportedCall {
        /// The function, by the library's `package_id` and its node id.
        callee: crate::library::PackageNodeKey,
        /// The callee's function index in the library's checked graph,
        /// resolved from `callee.node` once, at check time.
        function: usize,
        /// The arguments, in parameter order.
        arguments: Vec<NodeId>,
    },
    /// A tuple of a declared tuple type.
    Tuple {
        /// The tuple type's declaration.
        declaration: NodeKey,
        /// The components, in order.
        arguments: Vec<NodeId>,
    },
    /// A record of a declared record type.
    Record {
        /// The record type's declaration.
        declaration: NodeKey,
        /// The fields, in declaration order.
        slots: Vec<RecordSlot>,
    },
    /// A collection literal.
    Collection {
        /// The collection type.
        collection_type: CollectionType,
        /// The elements, in source order.
        elements: Vec<NodeId>,
    },
    /// A collection kind conversion.
    ConvertCollection {
        /// The target collection type.
        target: CollectionType,
        /// The collection converted.
        operand: NodeId,
    },
    /// A scalar conversion of the operand into the equality operand's target type.
    ConvertScalar(EqualityOperand, NodeId),
    /// An exact conversion of an IEEE value into `domain`.
    IeeeToRational(NodeId, RationalDomain),
    /// A one-binder query over `source`, binding each occurrence to `slot`.
    Query {
        /// What the query computes.
        visit: Visit,
        /// The slot each occurrence binds.
        slot: Slot,
        /// The collection visited.
        source: NodeId,
        /// The body evaluated per occurrence.
        body: NodeId,
    },
    /// A collection of collections, flattened.
    Flatten(NodeId),
    /// `fold`/`reduce` over `source`.
    Fold {
        /// The accumulator slot.
        accumulator: Slot,
        /// The slot each occurrence binds.
        binder: Slot,
        /// The collection folded.
        source: NodeId,
        /// The step evaluated per occurrence.
        step: NodeId,
        /// `None` for `reduce`.
        identity: Option<NodeId>,
    },
    /// A collection's size.
    Size(NodeId),
    /// Whether a collection contains an item.
    Contains(NodeId, NodeId),
    /// `allInstances<T>(p)` (FR-153). `T`, `N` and the bound `[0,N]` are
    /// exactly this node's own checked `value_type`
    /// (`ValueType::Collection`), never restated here.
    AllInstances {
        /// The population.
        population: NodeId,
    },
    /// `lookup<T>(p, r) absent m` (FR-153). `T` is exactly this node's own
    /// checked `value_type` (a bare `Reference<T>` for
    /// `undefined`/`refused`, an `Option<Reference<T>>` for `empty`), never
    /// restated here.
    Lookup {
        /// The population.
        population: NodeId,
        /// The reference looked up.
        reference: NodeId,
        /// The authored absence mode.
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
        /// The receiver.
        receiver: NodeId,
        /// The dispatch table index.
        table: usize,
        /// The dispatch operation index.
        operation: usize,
        /// The arguments after the receiver.
        arguments: Vec<NodeId>,
    },
    /// `pre(e)` (FR-153): evaluate `e` with `allInstances`/`lookup`
    /// underneath it reading the invocation pre population. Identity-typed:
    /// this node's `value_type` is always exactly its operand's.
    Pre(NodeId),
    /// `reaches(source, target, edge)` (FR-104, ADR-012 §15.2): whether
    /// `target` is reachable from `source` by following `edge`, in the state
    /// clause's own observation. Checked only inside a state clause, with
    /// both operands `Reference<T>` of one object type `T` and `edge` a
    /// field of `T` typed `Reference<T>`, `Option<Reference<T>>` or a
    /// sequence of `Reference<T>`.
    Reaches {
        /// The source reference.
        source: NodeId,
        /// The target reference.
        target: NodeId,
        /// The edge field, in `T`'s effective attribute set.
        edge: FieldRef,
    },
    /// FR-063/S3: exists only so `--cfg seam_probe` makes every
    /// match over `NodeKind` outside this module non-exhaustive. Never
    /// constructed outside the probe build.
    #[cfg(seam_probe)]
    __SeamProbe,
}

/// Whether FR-093 lowers a [`NodeKind::Coerce`] of an `operand`-typed node
/// into `interval` to a `quire.op.numeric.narrow` node: an integer whose
/// type the target range contains is admitted with no node.
pub(crate) fn coerce_builds_narrow(operand: &ValueType, interval: &IntegerInterval) -> bool {
    !matches!(operand, ValueType::Int(source)
        if interval.contains(source.lower()) && interval.contains(source.upper()))
}

/// The type a [`NodeKind::ConvertScalar`] of an `operand`-typed node
/// converts to, when FR-093 lowers it to a node: a conversion to the
/// operand's own type builds none.
pub(crate) fn scalar_conversion_target<'t>(
    target: &'t EqualityOperand,
    operand: &'t ValueType,
) -> Option<&'t ValueType> {
    let target = target.target().unwrap_or(operand);
    (target != operand).then_some(target)
}

impl NodeKind {
    /// Direct operands in evaluation order.
    pub fn children(&self) -> Vec<NodeId> {
        match self {
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
            | NodeKind::Pre(operand) => vec![*operand],
            NodeKind::Let { value, body, .. } => vec![*value, *body],
            NodeKind::If {
                condition,
                then,
                otherwise,
            } => vec![*condition, *then, *otherwise],
            NodeKind::Arithmetic(_, left, right)
            | NodeKind::Divide { left, right, .. }
            | NodeKind::Rational { left, right, .. }
            | NodeKind::Decimal { left, right, .. }
            | NodeKind::Ieee(_, _, left, right)
            | NodeKind::Quantity(_, left, right)
            | NodeKind::Order(_, _, left, right)
            | NodeKind::Equality(_, _, left, right)
            | NodeKind::Connective(_, left, right)
            | NodeKind::Contains(left, right)
            | NodeKind::Reaches {
                source: left,
                target: right,
                ..
            } => vec![*left, *right],
            NodeKind::Call { arguments, .. }
            | NodeKind::ImportedCall { arguments, .. }
            | NodeKind::Tuple { arguments, .. } => arguments.clone(),
            NodeKind::Record { slots, .. } => slots
                .iter()
                .filter_map(|slot| match slot {
                    RecordSlot::Present(node) => Some(*node),
                    RecordSlot::Absent | RecordSlot::Null => None,
                })
                .collect(),
            NodeKind::Collection { elements, .. } => elements.clone(),
            NodeKind::Query { source, body, .. } => vec![*source, *body],
            NodeKind::Fold {
                source,
                step,
                identity,
                ..
            } => {
                let mut children = vec![*source];
                children.extend(*identity);
                children.push(*step);
                children
            }
            NodeKind::AllInstances { population } => vec![*population],
            NodeKind::Lookup {
                population,
                reference,
                ..
            } => vec![*population, *reference],
            NodeKind::Dispatch {
                receiver,
                arguments,
                ..
            } => {
                let mut children = vec![*receiver];
                children.extend(arguments);
                children
            }
            // Not the S3 seam (`Machine::apply`'s own doc, `qsl-eval`): this
            // is `NodeKind`'s own module, so its match gets an unconditional
            // probe arm rather than being left to break.
            #[cfg(seam_probe)]
            NodeKind::__SeamProbe => unreachable!("never constructed outside the probe build"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ix_trace_rs::trace;
    use quire_semantic_value::location::Origin;

    /// How many nodes the deep body holds.
    const DEEP: usize = 100_000;

    /// `-(-(… -x))`: [`DEEP`] nodes, the local read `slot` at the bottom.
    fn negations(slot: Slot) -> CheckedBody {
        let node = |kind| Node {
            kind,
            value_type: ValueType::Integer,
            location: Location::root(Origin::Expression),
        };
        let mut builder = BodyBuilder::default();
        let mut operand = builder.push(node(NodeKind::Local(slot)));
        for _ in 2..DEEP {
            operand = builder.push(node(NodeKind::Negate(operand)));
        }
        builder.finish(node(NodeKind::Negate(operand)))
    }

    /// A literal holding a float has no canonical key, and still equals its
    /// own clone and differs from another bit pattern.
    #[trace("TC-725", "FR-258-AC-1")]
    #[test]
    fn a_float_literal_equals_its_clone_and_differs_by_bit_pattern() {
        let float = |bits| CheckedLiteral(Value::Float(quire_exact::IeeeValue::binary64(bits)));
        let nan = float(0x7ff8_0000_0000_0001);
        assert!(quire_exact::compare_keys(&nan.0, &nan.0).is_none());
        assert_eq!(nan, nan.clone());
        assert_ne!(nan, float(0x7ff8_0000_0000_0002));
    }

    /// FR-258 behaviour 2: a 100,000-deep checked body clones, compares
    /// equal to its clone and unequal to a body differing at its deepest
    /// node, walks every node, formats for debug and drops on a 512 KiB
    /// stack, since each of those runs over the arena's one vector.
    #[trace("TC-725", "FR-258-AC-1")]
    #[test]
    fn a_100000_deep_checked_body_clones_compares_formats_and_drops_on_a_small_stack() {
        std::thread::Builder::new()
            .stack_size(512 * 1024)
            .spawn(|| {
                let body = negations(0);
                assert_eq!(body.len(), DEEP);
                assert_eq!(body.root().descendants().len(), DEEP);
                let clone = body.clone();
                assert_eq!(clone, body);
                assert_ne!(negations(1), body);
                assert!(format!("{clone:?}").matches("Negate").count() == DEEP - 1);
                drop(clone);
                drop(body);
            })
            .expect("the thread spawns")
            .join()
            .expect("the deep body completes on a 512 KiB stack");
    }
}
