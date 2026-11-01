// SPDX-License-Identifier: AGPL-3.0-or-later
//! The typed, name-resolved expression tree that checking produces and the
//! evaluator runs.

use quire_checked::node::{Arithmetic, AuthoredBound, CheckedLiteral, NodeKind, RecordSlot};
use quire_exact::EffectiveId;
use quire_exact::{CollectionKind, Integer, IntegerInterval, NodeKey};
use quire_exact::{Value, ValueType};
use quire_semantic_value::declaration::EqualityOperand;
use quire_semantic_value::declaration::{EqualityOperator, EqualitySchedule};
use quire_semantic_value::enumeration::EnumMemberIndex;
use quire_semantic_value::location::Location;
use quire_semantic_value::quantity::UnitTable;
use std::collections::BTreeSet;
use std::convert::Infallible;
use std::fmt;
use std::ops::ControlFlow;

/// A local slot of one function frame or checked expression.
pub type Slot = usize;

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
/// use qsl_semantics::check::{Node, NodeId};
/// use quire_checked::NodeKind;
/// use quire_semantic_value::location::Location;
/// fn forge(kind: NodeKind<NodeId>, location: Location) -> Node {
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
    pub(in crate::check) kind: NodeKind<NodeId>,
    pub(in crate::check) value_type: ValueType,
    pub(in crate::check) location: Location,
    /// QSL producer facts that the equality checker selected, retained until
    /// lowering assigns the checked application its structural key.
    pub(in crate::check) equality_producer: Option<Box<EqualityProducerData>>,
}

/// QSL's pre-wire equality facts, retained from typing through lowering.
/// This is deliberately not IR's checked-term-data wire representation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EqualityProducerData {
    pub(crate) operator: EqualityOperator,
    pub(crate) schedule: EqualitySchedule,
    /// Operands as authored, including conversion source and target.
    pub(crate) authored_operands: [EqualityOperand; 2],
    /// Operands used for the stored equality schedule after explicit
    /// conversions have become their own typed nodes.
    pub(crate) compared_operands: [EqualityOperand; 2],
    /// Exact source and conversion-target type values retained per operand.
    pub(crate) operand_type_facts: Vec<EqualityOperandTypeFact>,
    /// The units resolved for this equality's source and comparison types.
    pub(crate) units: UnitTable,
    /// Exact operand type values paired to the UnitIds selected during typing.
    pub(crate) unit_type_facts: Vec<EqualityUnitTypeFact>,
    /// The compared enum's own filtered member index, or empty otherwise.
    pub(crate) enum_members: EnumMemberIndex,
}

/// Exact QSL identity witnesses retained for one typed equality occurrence.
/// The application key and occurrence come from lowering; quantity type keys
/// come from the checker's typed UnitIds, and nominal unit/dimension keys come
/// from the admitted UnitTable closure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EqualityProducerIdentity {
    application: NodeKey,
    location: Location,
    unit_ids: Vec<quire_exact::UnitId>,
    text_plan_type_nodes: Vec<(usize, NodeKey)>,
    enum_member_facts: Vec<EqualityEnumMemberFact>,
    unit_facts: Vec<EqualityUnitIdentityFact>,
    operand_type_nodes: Vec<EqualityOperandTypeIdentity>,
}

/// Exact selected enum identities: source declaration, lowered declaration,
/// lowered member, case, ordinal, and ordering flag.
pub type EqualityEnumMemberFact = (NodeKey, NodeKey, NodeKey, Box<str>, usize, bool);

/// The role of a typed quantity operand within equality.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EqualityUnitOperandRole {
    /// The quantity type before an explicit conversion.
    Source,
    /// The quantity type selected as an explicit conversion target.
    ConversionTarget,
}

/// A direct checker-held pairing of an operand's UnitId and exact type value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EqualityUnitTypeFact {
    /// Authored operand position in the equality.
    pub operand_position: usize,
    /// The operand's source or conversion-target role.
    pub role: EqualityUnitOperandRole,
    /// Exact quantity identity selected by typing.
    pub unit_id: quire_exact::UnitId,
    /// Exact checked type value from the typed operand.
    pub value_type: ValueType,
}

/// A typed source or conversion-target type fact for one equality operand.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EqualityOperandTypeFact {
    /// Authored operand position in the equality.
    pub operand_position: usize,
    /// The operand's source or conversion-target role.
    pub role: EqualityUnitOperandRole,
    /// Exact checked type value from typing.
    pub value_type: ValueType,
}

/// A structural node paired to one typed operand role by lowering.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EqualityOperandTypeIdentity {
    /// Authored operand position in the equality.
    pub operand_position: usize,
    /// The operand's source or conversion-target role.
    pub role: EqualityUnitOperandRole,
    /// Exact structural key of the type node.
    pub node: NodeKey,
}

/// The producer's exact dimension-declaration role for one resolved unit.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum EqualityDimensionRole {
    /// The dimension declared by a source unit.
    UnitDimension,
    /// A dimension declared by a unit on the conversion path.
    ConversionDimension,
    /// A normalized base dimension of a compound unit.
    CompoundBase,
    /// A base dimension reached from a declared dimension's ordered terms.
    BaseDimension,
}

/// One exact dimension declaration retained for a unit identity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EqualityDimensionIdentity {
    /// Unit whose checked quantity identity needs this dimension.
    pub unit_id: quire_exact::UnitId,
    /// Exact structural key of the declaration.
    pub node: NodeKey,
    /// The fact's role in this unit's identity closure.
    pub role: EqualityDimensionRole,
    /// Stable order within the retained unit closure.
    pub ordinal: usize,
}

/// One ordered direct term of a nominal dimension used by a unit identity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EqualityDimensionTermIdentity {
    /// Unit whose checked quantity identity needs this term.
    pub unit_id: quire_exact::UnitId,
    /// Exact structural key of the dimension declaration containing the term.
    pub declaration: NodeKey,
    /// Position in that declaration's ordered term list.
    pub ordinal: usize,
    /// Exact structural key of the base dimension.
    pub base: NodeKey,
    /// The authored exponent retained by semantic checking.
    pub exponent: Integer,
}

/// One ordered target edge on a declared unit's exact conversion path.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EqualityConversionEdgeIdentity {
    /// Unit whose conversion path contains this edge.
    pub unit_id: quire_exact::UnitId,
    /// Exact structural key of this edge's source unit.
    pub source: NodeKey,
    /// Position on the source unit's ordered conversion path.
    pub ordinal: usize,
    /// Exact structural key of the edge's target unit.
    pub target: NodeKey,
}

/// Per-unit identity closure retained by the equality checker.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EqualityUnitIdentityFact {
    /// The exact checked UnitId this fact closure belongs to.
    pub unit_id: quire_exact::UnitId,
    /// Exact type node paired to this UnitId by the equality checker.
    pub structural_node: NodeKey,
    /// Original operand position for this unit/type pairing.
    pub operand_position: usize,
    /// Source or conversion-target role for the pairing.
    pub operand_role: EqualityUnitOperandRole,
    /// Dimension declarations selected for this unit.
    pub dimensions: Vec<EqualityDimensionIdentity>,
    /// Ordered direct terms of selected dimension declarations.
    pub dimension_terms: Vec<EqualityDimensionTermIdentity>,
    /// Ordered conversion path edges selected for this unit.
    pub conversion_edges: Vec<EqualityConversionEdgeIdentity>,
}

/// Type-related identity facts retained for an equality occurrence.
pub struct EqualityTypeIdentityFacts {
    text_plan_type_nodes: Vec<(usize, NodeKey)>,
    enum_member_facts: Vec<EqualityEnumMemberFact>,
    unit_facts: Vec<EqualityUnitIdentityFact>,
    operand_type_nodes: Vec<EqualityOperandTypeIdentity>,
}

impl EqualityTypeIdentityFacts {
    pub(crate) fn new(
        text_plan_type_nodes: Vec<(usize, NodeKey)>,
        enum_member_facts: Vec<EqualityEnumMemberFact>,
        unit_facts: Vec<EqualityUnitIdentityFact>,
        operand_type_nodes: Vec<EqualityOperandTypeIdentity>,
    ) -> Self {
        Self {
            text_plan_type_nodes,
            enum_member_facts,
            unit_facts,
            operand_type_nodes,
        }
    }
}

impl EqualityProducerIdentity {
    pub(crate) fn new(
        application: NodeKey,
        location: Location,
        unit_ids: Vec<quire_exact::UnitId>,
        type_identity_facts: EqualityTypeIdentityFacts,
    ) -> Self {
        let EqualityTypeIdentityFacts {
            text_plan_type_nodes,
            enum_member_facts,
            unit_facts,
            operand_type_nodes,
        } = type_identity_facts;
        Self {
            application,
            location,
            unit_ids,
            text_plan_type_nodes,
            enum_member_facts,
            unit_facts,
            operand_type_nodes,
        }
    }

    /// The exact checked application node selected by QSL lowering.
    pub fn application(&self) -> &NodeKey {
        &self.application
    }

    /// The authored occurrence paired with the application node.
    pub fn location(&self) -> &Location {
        &self.location
    }

    /// Source and compound unit identities retained by the equality checker.
    pub fn unit_ids(&self) -> &[quire_exact::UnitId] {
        &self.unit_ids
    }

    /// Exact Text/Plan comparison type keys by authored operand position.
    pub fn text_plan_type_nodes(&self) -> &[(usize, NodeKey)] {
        &self.text_plan_type_nodes
    }

    /// Exact filtered enum declaration/member facts used by this equality.
    pub fn enum_member_facts(&self) -> &[EqualityEnumMemberFact] {
        &self.enum_member_facts
    }

    /// Per-UnitId dimension and ordered conversion identity facts.
    pub fn unit_facts(&self) -> &[EqualityUnitIdentityFact] {
        &self.unit_facts
    }

    /// Exact source and conversion-target operand type nodes.
    pub fn operand_type_nodes(&self) -> &[EqualityOperandTypeIdentity] {
        &self.operand_type_nodes
    }
}

impl EqualityProducerData {
    /// The operator selected during typing.
    pub fn operator(&self) -> EqualityOperator {
        self.operator
    }

    /// The schedule selected during typing.
    pub fn schedule(&self) -> EqualitySchedule {
        self.schedule
    }

    /// The operands as authored, including conversion source and target.
    pub fn authored_operands(&self) -> &[EqualityOperand; 2] {
        &self.authored_operands
    }

    /// The operands used by the stored equality after explicit conversions.
    pub fn compared_operands(&self) -> &[EqualityOperand; 2] {
        &self.compared_operands
    }

    /// Units resolved while checking this equality.
    pub fn units(&self) -> &UnitTable {
        &self.units
    }

    /// Exact quantity UnitId/type/operand-role joins retained during typing.
    pub fn unit_type_facts(&self) -> &[EqualityUnitTypeFact] {
        &self.unit_type_facts
    }

    /// Exact source and conversion-target type values for each operand.
    pub fn operand_type_facts(&self) -> &[EqualityOperandTypeFact] {
        &self.operand_type_facts
    }

    /// The filtered enum-member index used by this equality, empty unless its
    /// schedule is `Enum`.
    pub fn enum_members(&self) -> &EnumMemberIndex {
        &self.enum_members
    }
}

impl Node {
    /// What the node computes.
    pub fn kind(&self) -> &NodeKind<NodeId> {
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
    /// A malformed checked-state fixture for producer tests, never shipped.
    #[cfg(any(test, feature = "test-support"))]
    pub fn fixture(
        operands: Vec<ValueType>,
        kind: impl FnOnce(&[NodeId]) -> NodeKind<NodeId>,
        result: ValueType,
    ) -> Self {
        let ids: Vec<_> = (0..operands.len()).map(NodeId).collect();
        let mut nodes: Vec<_> = operands
            .into_iter()
            .map(|value_type| Node {
                kind: NodeKind::Literal(CheckedLiteral(Value::Boolean(true))),
                value_type,
                location: Location::root(quire_semantic_value::location::Origin::Expression),
                equality_producer: None,
            })
            .collect();
        nodes.push(Node {
            kind: kind(&ids),
            value_type: result,
            location: Location::root(quire_semantic_value::location::Origin::Expression),
            equality_producer: None,
        });
        Self { nodes }
    }

    /// Link the relationship-end reachability `id` to the authored bound of
    /// the population it walks (FR-380). A node that is no such
    /// reachability is left as it is.
    pub(in crate::check) fn link_population(&mut self, id: NodeId, bound: AuthoredBound) {
        if let Some(Node {
            kind: NodeKind::ReachesEnd { population, .. },
            ..
        }) = self.nodes.get_mut(id.0)
        {
            *population = Some(bound);
        }
    }

    /// A body of `nodes`, each stored after the nodes it names, the last the
    /// root, for evaluator tests of node kinds no checked package can hold
    /// yet. A node names an operand by the [`NodeId`] of its position in
    /// `nodes`, so a test builds operands first. Never shipped.
    #[cfg(any(test, feature = "test-support"))]
    pub fn from_nodes(nodes: Vec<(NodeKind<NodeId>, ValueType)>) -> Self {
        assert!(!nodes.is_empty(), "a body holds its root");
        Self {
            nodes: nodes
                .into_iter()
                .map(|(kind, value_type)| Node {
                    kind,
                    value_type,
                    location: Location::root(quire_semantic_value::location::Origin::Expression),
                    equality_producer: None,
                })
                .collect(),
        }
    }

    /// The id of the node at `index` of [`Self::from_nodes`]'s input.
    #[cfg(any(test, feature = "test-support"))]
    pub fn id_at(index: usize) -> NodeId {
        NodeId(index)
    }

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

    /// Whether a node of the body reads the presences of a clock
    /// collection's elements (`forall(e in r.q: present(e))`), whose
    /// quantifier over the collection's slots has no checked form until
    /// STD-554.
    pub(crate) fn reads_slot_presence(&self) -> bool {
        self.nodes
            .iter()
            .any(|node| matches!(node.kind, NodeKind::SlotPresence { .. }))
    }

    /// Whether a node of the body reads the local slot `slot`: for a slot
    /// below the body's parameter count, whether it reads that parameter.
    pub fn reads_local(&self, slot: Slot) -> bool {
        self.nodes
            .iter()
            .any(|node| matches!(node.kind, NodeKind::Local(read) if read == slot))
    }

    /// The evaluation frame this body needs when its first `bound` slots
    /// are its parameters: `bound`, or one past the highest slot a node of
    /// the body reads or binds, whichever is larger.
    pub fn frame_slots(&self, bound: usize) -> usize {
        self.nodes
            .iter()
            .flat_map(|node| match &node.kind {
                NodeKind::Local(slot) | NodeKind::Let { slot, .. } => vec![*slot],
                NodeKind::Query { slot, .. } | NodeKind::Seq { slot, .. } => vec![*slot],
                NodeKind::Fold {
                    accumulator,
                    binder,
                    ..
                } => vec![*accumulator, *binder],
                // Every other kind reads and binds no local slot.
                _ => Vec::new(),
            })
            .map(|slot| slot.saturating_add(1))
            .fold(bound, usize::max)
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
    pub fn kind(self) -> &'a NodeKind<NodeId> {
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

    /// The QSL producer facts for a checked equality, if typing selected any.
    pub(crate) fn equality_producer(self) -> Option<&'a EqualityProducerData> {
        self.node().equality_producer.as_deref()
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
        node_children(self.kind())
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

    /// Every protocol declaration index a `NodeKind::ProtocolReference` in
    /// this subtree names, in pre-order: the protocols whose clause keys this
    /// subtree's own key hashes.
    pub(crate) fn protocol_references(self) -> Vec<usize> {
        self.descendants()
            .into_iter()
            .filter_map(|node| match node.kind() {
                NodeKind::ProtocolReference { protocol } => Some(*protocol),
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
/// The inclusive range of an integer: its lower and upper bound.
pub(crate) type IntegerRange = (Integer, Integer);

/// The static range of the integer a node computes (FR-139; QSpec FR-376's
/// "declared bounds" of a `seq` bound or a `sum` summand): the interval of an
/// `Int[..]` node, the value of a literal, and the interval arithmetic of
/// `+`, `-`, `*` and negation over those. `None` when a node on the way has
/// no static range. `node` names the operands of `root`.
///
/// The walk keeps its pending nodes on a heap stack, so no depth of checked
/// expression is a limit (ADR-030 D-1).
pub(crate) fn static_integer_range<'n>(
    root: (&'n NodeKind<NodeId>, &'n ValueType),
    node: &dyn Fn(NodeId) -> Option<(&'n NodeKind<NodeId>, &'n ValueType)>,
) -> Option<IntegerRange> {
    enum Work<'n> {
        Enter((&'n NodeKind<NodeId>, &'n ValueType)),
        Exit(&'n NodeKind<NodeId>),
    }
    let mut work = vec![Work::Enter(root)];
    let mut ranges: Vec<(Integer, Integer)> = Vec::new();
    while let Some(step) = work.pop() {
        match step {
            Work::Enter((kind, value_type)) => {
                if let ValueType::Int(interval) = value_type {
                    ranges.push((interval.lower().clone(), interval.upper().clone()));
                    continue;
                }
                match kind {
                    NodeKind::Literal(CheckedLiteral(Value::Integer(value))) => {
                        ranges.push((value.clone(), value.clone()));
                    }
                    NodeKind::Arithmetic(_, left, right) => {
                        work.push(Work::Exit(kind));
                        work.push(Work::Enter(node(*right)?));
                        work.push(Work::Enter(node(*left)?));
                    }
                    NodeKind::Negate(operand) => {
                        work.push(Work::Exit(kind));
                        work.push(Work::Enter(node(*operand)?));
                    }
                    // Either branch: the range spans both.
                    NodeKind::If {
                        then, otherwise, ..
                    } => {
                        work.push(Work::Exit(kind));
                        work.push(Work::Enter(node(*otherwise)?));
                        work.push(Work::Enter(node(*then)?));
                    }
                    _ => return None,
                }
            }
            Work::Exit(NodeKind::Negate(_)) => {
                let (lower, upper) = ranges.pop()?;
                ranges.push((upper.neg(), lower.neg()));
            }
            Work::Exit(NodeKind::If { .. }) => {
                let (otherwise_lower, otherwise_upper) = ranges.pop()?;
                let (then_lower, then_upper) = ranges.pop()?;
                ranges.push((
                    then_lower.min(otherwise_lower),
                    then_upper.max(otherwise_upper),
                ));
            }
            Work::Exit(NodeKind::Arithmetic(operator, _, _)) => {
                let (right_lower, right_upper) = ranges.pop()?;
                let (left_lower, left_upper) = ranges.pop()?;
                ranges.push(match operator {
                    Arithmetic::Add => (left_lower.add(&right_lower), left_upper.add(&right_upper)),
                    Arithmetic::Subtract => {
                        (left_lower.sub(&right_upper), left_upper.sub(&right_lower))
                    }
                    Arithmetic::Multiply => {
                        let products = [
                            left_lower.mul(&right_lower),
                            left_lower.mul(&right_upper),
                            left_upper.mul(&right_lower),
                            left_upper.mul(&right_upper),
                        ];
                        let lowest = products.iter().min()?.clone();
                        let highest = products.iter().max()?.clone();
                        (lowest, highest)
                    }
                });
            }
            Work::Exit(_) => return None,
        }
    }
    let range = ranges.pop()?;
    ranges.is_empty().then_some(range)
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

pub(crate) fn node_children(kind: &NodeKind<NodeId>) -> Vec<NodeId> {
    match kind {
        NodeKind::Literal(_)
        | NodeKind::Local(_)
        | NodeKind::ProtocolReference { .. }
        | NodeKind::PopulationObjects { .. }
        | NodeKind::NamedPopulation { .. } => Vec::new(),
        NodeKind::Seq {
            low, high, body, ..
        } => vec![*low, *high, *body],
        NodeKind::Coerce(operand, _)
        | NodeKind::Negate(operand)
        | NodeKind::Not(operand)
        | NodeKind::Field { operand, .. }
        | NodeKind::Attribute {
            reference: operand, ..
        }
        | NodeKind::History {
            reference: operand, ..
        }
        | NodeKind::Navigate {
            reference: operand, ..
        }
        | NodeKind::Present(operand)
        | NodeKind::PlacePresence {
            reference: operand, ..
        }
        | NodeKind::SlotPresence {
            reference: operand, ..
        }
        | NodeKind::Value(operand)
        | NodeKind::OptionSome(operand)
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
        | NodeKind::IntegerDivision { left, right, .. }
        | NodeKind::Divide { left, right, .. }
        | NodeKind::Rational { left, right, .. }
        | NodeKind::Decimal { left, right, .. }
        | NodeKind::Ieee(_, _, left, right)
        | NodeKind::Quantity(_, left, right)
        | NodeKind::Order(_, _, left, right)
        | NodeKind::Equality(_, _, left, right)
        | NodeKind::IeeeNumericEquality(left, right)
        | NodeKind::Connective(_, left, right)
        | NodeKind::Contains(left, right)
        | NodeKind::Reaches {
            source: left,
            target: right,
            ..
        }
        | NodeKind::ReachesEnd {
            source: left,
            target: right,
            ..
        } => vec![*left, *right],
        NodeKind::IeeeSquareRoot(_, operand) => vec![*operand],
        NodeKind::Call { arguments, .. }
        | NodeKind::ImportedCall { arguments, .. }
        | NodeKind::Tuple { arguments, .. }
        | NodeKind::UnionValue { arguments, .. } => arguments.clone(),
        NodeKind::Record { slots, .. } => slots
            .iter()
            .filter_map(|slot| match slot {
                RecordSlot::<NodeId>::Present(node) => Some(*node),
                RecordSlot::<NodeId>::Absent | RecordSlot::<NodeId>::Null => None,
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
        NodeKind::Case {
            scrutinee, arms, ..
        } => std::iter::once(*scrutinee)
            .chain(arms.iter().map(|arm| arm.body))
            .collect(),
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
            equality_producer: None,
        };
        let mut builder = BodyBuilder::default();
        let mut operand = builder.push(node(NodeKind::Local(slot)));
        for _ in 2..DEEP {
            operand = builder.push(node(NodeKind::Negate(operand)));
        }
        builder.finish(node(NodeKind::Negate(operand)))
    }

    /// A literal holding a float, alone or inside an option, or a
    /// population has no canonical key, and still equals its own clone and
    /// differs from another bit pattern or identity.
    #[trace("TC-725", "FR-258-AC-1")]
    #[test]
    fn a_float_literal_equals_its_clone_and_differs_by_bit_pattern() {
        let float = |bits| Value::Float(quire_exact::IeeeValue::binary64(bits));
        let option = |bits| {
            quire_exact::OptionValue::present(
                quire_exact::ValueType::Float(quire_exact::FloatType::new(
                    quire_exact::IeeeWidth::Binary64,
                    quire_exact::RoundingMode::Exact,
                )),
                float(bits),
            )
            .expect("the payload fits its type")
        };
        let nan = CheckedLiteral(float(0x7ff8_0000_0000_0001));
        assert!(quire_exact::compare_keys(&nan.0, &nan.0).is_none());
        assert_eq!(nan, nan.clone());
        assert_ne!(nan, CheckedLiteral(float(0x7ff8_0000_0000_0002)));
        assert_ne!(nan, CheckedLiteral(float(0x3ff0_0000_0000_0000)));
        let nested = CheckedLiteral(option(0x7ff8_0000_0000_0001));
        assert_eq!(nested, nested.clone());
        assert_ne!(nested, CheckedLiteral(option(0x7ff8_0000_0000_0002)));
        // A population has no canonical key and is compared by identity.
        let population = |byte: u8| {
            Value::Population(quire_exact::PopulationId::new(
                quire_exact::NodeKey::new(
                    "fixture".to_owned(),
                    vec![quire_exact::PathSegment::Text(format!("p{byte}"))],
                    0,
                )
                .expect("a fixture key"),
            ))
        };
        let one = CheckedLiteral(population(1));
        assert!(quire_exact::compare_keys(&one.0, &one.0).is_none());
        assert_eq!(one, one.clone());
        assert_ne!(one, CheckedLiteral(population(2)));
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
