// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-092 and FR-093 (QSL-156 A4b): the checked semantic graph of a package's
//! `Value`-family functions, and every node's key.
//!
//! `check` mints node keys at E3 (ADR-011 §2.2, FB-13), and each key hashes
//! its lowered body, so `check` lowers before it keys. [`Lowering`] turns
//! each checked function (its parameters, its checked body and measure and
//! its declared result type) into FR-322 nodes:
//!
//! - one type node per checked `ValueType` (FR-092 "Type nodes");
//! - one parameter node per binder (FR-092 "Parameter nodes");
//! - one node per checked expression, except a local read, which is a
//!   `reference` to its binder's parameter node (FR-093 "One node per
//!   checked expression"), with the operator, catalogued operation, member,
//!   mode, laws, leaves and arguments of FR-093's application table;
//! - one function node whose body references its parameters, its body's
//!   root node and its measure's (FR-092 "Function nodes").
//!
//! Every node is content-addressed: equal content is one node, and each
//! source occurrence of it is its own occurrence entry. Recursive functions
//! and records form recursion groups (FR-092 "Recursion groups"): their
//! nodes are keyed together, in dependency order, by the group order
//! ([`super::node_key::group_keys`]), and two groups whose members' keys
//! coincide refuse. Every key comes from
//! [`super::node_key::node_key`], which runs once per distinct content: a
//! node built again takes the key its content was given. Laws come only
//! from the package's lock evidence ([`LockEvidence`]); a law it does not
//! supply refuses the node (`missing_declaration`/`missing-selection`),
//! never a constant.
//!
//! FR-094 keys the rest (`model`): the model declaration nodes a
//! `Reference<T>` or a model row's member names, the `Reference<T>` and
//! `Population<T>[N]` type nodes, a clause function's `ModelOwner` and
//! `clause` binding, and quantity type nodes. `check` records each model
//! declaration node it keys in the model correspondence.

use std::collections::{btree_map, BTreeMap, BTreeSet, HashMap};
use std::hash::{BuildHasher, BuildHasherDefault, DefaultHasher};

use quire_exact::{
    ArithmeticOperator, Charge, ChargePoint, CollectionKind, CollectionType, EffectiveId,
    Identifier, Integer, Meter, NodeKey, OrderingOperator, Presence, TextProfile, Value, ValueType,
};

use super::check::Scope;
use super::family::OccurrenceMap;
use super::ir::{Arithmetic, Connective, Node, NodeKind, OrderedKind, RecordSlot, Slot, Visit};
use super::node_key::{
    group_keys, node_key, LawRole, LeafSegment, LiteralValue, NodeInput, NodeKeyRefusal, NodeTag,
    Operation, OperationLaw, OperationLeaf, OperationMode, Operator, Owner, SemanticTerm,
    SourceOwner,
};
use super::refusal::{
    CheckCause, CheckRefusal, CheckingLimitKind, CheckingStage, KeyFault, Location, Origin,
};
use crate::model::key::DeclarationKey;
use crate::value::declaration::{
    CompositeShape, EqualityOperator, FieldDeclaration, TypeEnvironment,
};
use crate::value::definition::DefinitionReference;
use crate::value::member::Member;
use crate::value::quantity::UnitTable;

mod model;

pub use model::{AdmittedModel, ForeignView, ModelClause};

/// The package's lock evidence as the lowering reads it (ADR-011 §2.4): the
/// `DefinitionRef` the `text_profile` law role selects. QSpec publishes no
/// `complete-value-lock.json` accessor yet, so every production package
/// supplies none, and each text-law operation refuses
/// (`missing_declaration`/`missing-selection`, FR-093-AC-6). The
/// `ieee_profile` law is the package's one admitted IEEE profile
/// (`PackageDeclarations::ieee_profile`, admitted against the definition
/// lock by `DefinitionLock::admit_ieee_profile`), so the two cannot
/// disagree.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct LockEvidence {
    text_profile: Option<DefinitionReference>,
}

impl LockEvidence {
    /// Lock evidence selecting `definition` for the `text_profile` role.
    pub fn with_text_profile(mut self, definition: DefinitionReference) -> Self {
        self.text_profile = Some(definition);
        self
    }

    fn text_profile(&self) -> Option<&DefinitionReference> {
        self.text_profile.as_ref()
    }
}

/// A node's place in its recursion group (FR-092 "Recursion groups"): the
/// group digest, the node's ordinal and the number of the group's nodes.
/// FR-093's emission writes a group's nodes in ordinal order.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NodeRecursion {
    group: [u8; 32],
    ordinal: usize,
    size: usize,
}

impl NodeRecursion {
    /// The group digest.
    pub fn group(&self) -> &[u8; 32] {
        &self.group
    }

    /// The node's ordinal: its rank in the group order.
    pub fn ordinal(&self) -> usize {
        self.ordinal
    }

    /// The number of the group's nodes.
    pub fn size(&self) -> usize {
        self.size
    }
}

/// One lowered, keyed node of the checked semantic graph.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticNode {
    key: NodeKey,
    preimage: Vec<u8>,
    recursion: Option<NodeRecursion>,
    /// Outside every recursion group, the content [`Self::key`] keys; a
    /// group member's names its group's members by their keys.
    content: NodeContent,
}

impl SemanticNode {
    /// The node's `quire.checked-semantic-node/v1` key.
    pub fn key(&self) -> NodeKey {
        self.key
    }

    /// The exact preimage bytes [`Self::key`] is the SHA-256 of.
    pub fn preimage(&self) -> &[u8] {
        &self.preimage
    }

    /// The node's FR-322 `node_tag`.
    pub fn node_tag(&self) -> NodeTag {
        self.content.node_tag
    }

    /// The node's FR-322 `semantic_form`.
    pub fn semantic_form(&self) -> &'static str {
        self.content.semantic_form
    }

    /// The node's semantic type, or `None` for a self-typed node.
    pub fn semantic_type(&self) -> Option<NodeKey> {
        self.content.semantic_type
    }

    /// The node's `declaration.qualified_name`, when it declares.
    pub fn declaration(&self) -> Option<&[Identifier]> {
        self.content.declaration.as_deref()
    }

    /// The node's owner: a declared node's `SourceOwner`, a model-owned
    /// node's `ModelOwner`.
    pub fn owner(&self) -> Option<&Owner> {
        self.content.owner.as_ref()
    }

    /// The node's recursion group, when it is in one.
    pub fn recursion(&self) -> Option<&NodeRecursion> {
        self.recursion.as_ref()
    }

    /// The node's FR-322 body. A reference to a member of the node's own
    /// recursion group names that member's key.
    pub fn body(&self) -> &SemanticTerm {
        &self.content.body
    }
}

/// The checked semantic graph: every lowered node by key.
#[derive(Clone, Debug, Default)]
pub struct SemanticGraph {
    nodes: BTreeMap<NodeKey, SemanticNode>,
}

impl SemanticGraph {
    /// The node keyed `key`.
    pub fn node(&self, key: NodeKey) -> Option<&SemanticNode> {
        self.nodes.get(&key)
    }

    /// Every node, ascending by key.
    pub fn nodes(&self) -> impl Iterator<Item = &SemanticNode> {
        self.nodes.values()
    }
}

/// One checked function, as the lowering reads it.
pub(crate) struct FunctionInput<'a> {
    /// The declared name, `::`-separated.
    pub(crate) name: &'a str,
    /// The declaration's region.
    pub(crate) location: &'a Location,
    /// The parameters in order.
    pub(crate) parameters: &'a [(String, ValueType)],
    /// The declared result type.
    pub(crate) result: &'a ValueType,
    /// The checked body.
    pub(crate) body: &'a Node,
    /// The name each body slot was bound under, indexed by slot.
    pub(crate) body_slots: &'a [String],
    /// The checked measure, when one is written.
    pub(crate) measure: Option<&'a Node>,
    /// The name each measure slot was bound under, indexed by slot.
    pub(crate) measure_slots: &'a [String],
    /// The object type `T` of each `Population<T>[N]` parameter, by
    /// parameter position (`None` for every other parameter).
    pub(crate) population_targets: &'a [Option<EffectiveId>],
    /// The owner and kind of a clause function synthesized from a model
    /// clause (FR-094), or `None` for a source-declared function.
    pub(crate) clause: Option<&'a ModelClause>,
}

/// One binder in scope: its slot, its parameter node and that node's
/// semantic type.
#[derive(Clone, Copy)]
struct Binder {
    slot: Slot,
    parameter: NodeKey,
    semantic_type: NodeKey,
}

/// The binders in scope at one point of a function body.
struct Binders<'s> {
    /// The name each slot was bound under.
    slot_names: &'s [String],
    /// Each binder in scope, outermost first.
    scope: Vec<Binder>,
}

/// Every member of a node's preimage outside every recursion group: what
/// [`node_key`] keys. [`node_key`] is a function of these members alone, so
/// equal content has one key, which [`Lowering::insert_node`] computes once
/// (QSL-221).
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct NodeContent {
    node_tag: NodeTag,
    semantic_form: &'static str,
    semantic_type: Option<NodeKey>,
    declaration: Option<Vec<Identifier>>,
    owner: Option<Owner>,
    body: SemanticTerm,
}

impl NodeContent {
    /// The content as [`node_key`]'s input.
    fn input(&self) -> NodeInput<'_> {
        NodeInput {
            owner: self.owner.as_ref(),
            node_tag: self.node_tag,
            semantic_form: self.semantic_form,
            semantic_type: self.semantic_type,
            declaration: self.declaration.as_deref(),
            body: &self.body,
        }
    }
}

/// A node whose key waits on its recursion group (FR-092 "Recursion
/// groups"): its content names a placeholder or another draft.
struct Draft {
    location: Location,
    content: NodeContent,
}

/// A package's functions that call each other, lowered together: callees
/// outside the group come earlier in [`lowering_order`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct FunctionGroup {
    /// The functions' indices, ascending.
    pub(crate) members: Vec<usize>,
    /// Whether the functions call each other (or one calls itself).
    pub(crate) recursive: bool,
}

/// What lowering yields: the graph, its model correspondence entries `(node
/// key, DeclarationKey)` and each function's node key by index.
pub(crate) struct Lowered {
    pub(crate) graph: SemanticGraph,
    pub(crate) correspondence: Vec<(NodeKey, DeclarationKey)>,
    pub(crate) functions: Vec<Option<NodeKey>>,
}

/// Builds and keys the nodes of one package.
///
/// A node outside every recursion group is keyed when it is built. A node
/// that names a function or record still being built names it by a
/// placeholder, and is a [`Draft`] until nothing is left open; then
/// [`Self::settle`] keys the drafts in dependency order, each recursion
/// group by FR-092's group order.
pub(crate) struct Lowering<'a> {
    scope: &'a Scope,
    owner: Owner,
    models: &'a [AdmittedModel],
    /// The package's units and every compound unit the check stage formed.
    units: UnitTable,
    /// Each model declaration node keyed, by its `DeclarationKey`.
    correspondence: BTreeMap<DeclarationKey, NodeKey>,
    lock: &'a LockEvidence,
    depth_limit: u64,
    graph: SemanticGraph,
    occurrences: &'a mut OccurrenceMap<Location>,
    /// Each lowered function's key, by function index.
    functions: Vec<Option<NodeKey>>,
    /// Each declared composite's node key, by its declaration key.
    composites: BTreeMap<NodeKey, NodeKey>,
    /// The `functions` slots and `composites` entries written since the
    /// last settle, the only keys that settle can still resolve (QSL-205:
    /// a settle per function group resolves its own keys, not every key
    /// the package has so far).
    unsettled_functions: Vec<usize>,
    unsettled_composites: Vec<NodeKey>,
    /// Composites whose node is being built.
    composites_in_progress: Vec<NodeKey>,
    /// The placeholder naming each composite in progress that reached
    /// itself.
    composite_placeholders: BTreeMap<NodeKey, NodeKey>,
    /// Whether a recursive function group is being lowered.
    functions_open: bool,
    /// Each placeholder, and the draft it became once built.
    placeholders: BTreeMap<NodeKey, Option<NodeKey>>,
    /// How many placeholders this lowering has made.
    placeholder_count: u64,
    /// The drafts, by the digest of their content.
    drafts: BTreeMap<NodeKey, Draft>,
    /// A key [`Self::insert_node`] computed, by a hash of the content it
    /// keyed (QSL-221). Lowering builds the same node many times (each
    /// function's parameter and result types, each builtin scalar); a
    /// content whose hash names a key whose node or draft holds that same
    /// content ([`Self::holds`]) takes the key without its canonical
    /// encoding and SHA-256. A hash names at most one key: a colliding
    /// content is keyed in full and replaces the entry.
    keys: HashMap<u64, NodeKey>,
    /// The hasher of [`Self::keys`].
    content_hashes: BuildHasherDefault<DefaultHasher>,
    /// Occurrences of drafts, recorded once the drafts are keyed.
    draft_occurrences: Vec<(NodeKey, &'static str, Location)>,
    /// Each keyed recursion-group member by the key its content would have
    /// outside the group (its body naming the members' keys, `recursion`
    /// `null`): a node built later with that content is the member (FR-092,
    /// G17 is `Option<Node>` wherever it is named), whatever order the
    /// package names it in.
    rebuilt_members: BTreeMap<NodeKey, NodeKey>,
    /// Each keyed recursion-group member's group digest.
    group_of: BTreeMap<NodeKey, [u8; 32]>,
    /// Each keyed group's declared members' regions, by group digest.
    group_regions: BTreeMap<[u8; 32], Vec<Location>>,
    /// The checking stage's work meter (`CheckingLimits::work_budget`):
    /// each node built and each group keyed charges it.
    meter: &'a mut Meter,
    /// The check stage's node limit (`CheckingLimits::nodes`), and the
    /// units typing left of it: each text or recursion leaf costs one
    /// (FR-093 "Text leaves").
    node_limit: u64,
    node_budget: u64,
    /// Whether a text type is reachable from each composite a text-leaf
    /// walk asked about.
    text_reach: BTreeMap<NodeKey, bool>,
}

fn refuse(location: &Location, cause: CheckCause) -> CheckRefusal {
    CheckRefusal {
        location: location.clone(),
        cause,
    }
}

fn fault(location: &Location, fault: KeyFault) -> CheckRefusal {
    refuse(location, CheckCause::InternalFault(Box::new(fault)))
}

/// Charge `work` units to `meter` at `declaration.check`.
fn charge_work(meter: &mut Meter, work: u64) -> Result<(), NodeKeyRefusal> {
    meter
        .charge(Charge::new(ChargePoint::DeclarationCheck).work(Integer::from(work)))
        .map_err(|incomplete| NodeKeyRefusal::WorkBudget {
            limit: incomplete.limit,
        })
}

fn preimage_refusal(location: &Location, refusal: NodeKeyRefusal) -> CheckRefusal {
    match refusal {
        NodeKeyRefusal::WorkBudget { limit } => refuse(
            location,
            CheckCause::ResourceExhausted {
                stage: CheckingStage::Typing,
                kind: CheckingLimitKind::WorkBudget,
                limit,
            },
        ),
        NodeKeyRefusal::InvalidGroup => fault(location, KeyFault::InvalidGroup),
        NodeKeyRefusal::TooDeep { limit } => refuse(
            location,
            CheckCause::ResourceExhausted {
                stage: CheckingStage::Typing,
                kind: CheckingLimitKind::Depth,
                limit,
            },
        ),
        refusal => refuse(location, CheckCause::NodePreimage(refusal)),
    }
}

/// One step of [`Lowering::type_node`]'s loop.
enum TypeStep<'v> {
    /// Build the node of this type at this nesting depth.
    Descend(&'v ValueType, u64),
    /// This node is built: hand its key to the frame waiting for it.
    Built(NodeKey),
}

/// A type node waiting for the node of a type inside it.
enum TypeFrame<'v> {
    /// An `option` node, waiting for its payload's node.
    Option,
    /// A collection's nodes, waiting for its element's node.
    Collection(&'v CollectionType),
    /// A declared record or tuple, waiting for its next member's node.
    Composite(CompositeFrame<'v>),
}

/// A declared record or tuple whose node is being built.
struct CompositeFrame<'v> {
    declaration: NodeKey,
    /// The declaration's qualified name.
    name: Vec<Identifier>,
    shape: &'v CompositeShape,
    /// The nesting depth the composite was reached at.
    depth: u64,
    /// The members built so far, in declaration order.
    members: Vec<SemanticTerm>,
    /// The record field whose type node is being built.
    awaiting: Option<&'v FieldDeclaration>,
}

impl CompositeFrame<'_> {
    /// Append the member whose type node is `key`: a record field's
    /// binding (an optional one's through `optional`), or a tuple
    /// position's reference.
    fn accept(&mut self, key: NodeKey) {
        let reference = SemanticTerm::reference(key);
        self.members.push(match self.awaiting.take() {
            None => reference,
            Some(field) => SemanticTerm::binding(
                field.name(),
                match field.presence() {
                    Presence::Required => reference,
                    Presence::Optional => SemanticTerm::binding("optional", reference),
                },
            ),
        });
    }
}

/// How a declared composite's node starts.
enum OpenComposite<'v> {
    /// Its node, or the placeholder naming it while it is being built.
    Built(NodeKey),
    /// Its frame: its members are still to build.
    Open(CompositeFrame<'v>),
}

/// One step of [`Lowering::expression`]'s loop.
enum LowerStep<'n> {
    /// Lower this checked node at this nesting depth (the body root is 0).
    Descend(&'n Node, u64),
    /// A node is lowered to this term: hand it to the frame waiting for it.
    Lowered(SemanticTerm),
}

/// A checked node waiting for the terms of its operands.
enum LowerFrame<'n> {
    /// An application or value node over its operands, in order.
    Operands(Box<OperandsFrame<'n>>),
    /// `let`: its value, then its body under the new binder.
    Let(Box<LetFrame<'n>>),
    /// A one-binder operation: its source, then its body under the binder.
    Binder(Box<BinderFrame<'n>>),
    /// `deref(r).f` over the reference `r`, of object type `object`.
    Attribute {
        node: &'n Node,
        object: NodeKey,
        name: &'n str,
    },
    /// A record value: its present fields' values, in declaration order.
    Record(Box<RecordFrame<'n>>),
    /// `fold`/`reduce`: its source, its identity, then its step under the
    /// accumulator and element binders.
    Fold(Box<FoldFrame<'n>>),
}

/// A checked node's operands, in order.
#[derive(Clone, Copy)]
enum Operands<'n> {
    One(&'n Node),
    Two(&'n Node, &'n Node),
    Three(&'n Node, &'n Node, &'n Node),
    Each(&'n [Node]),
    /// A dispatch call's receiver, then its arguments.
    Receiver(&'n Node, &'n [Node]),
}

impl<'n> Operands<'n> {
    fn len(self) -> usize {
        match self {
            Self::One(_) => 1,
            Self::Two(..) => 2,
            Self::Three(..) => 3,
            Self::Each(nodes) => nodes.len(),
            Self::Receiver(_, arguments) => arguments.len() + 1,
        }
    }

    fn get(self, index: usize) -> Option<&'n Node> {
        match (self, index) {
            (Self::One(first) | Self::Two(first, _) | Self::Three(first, _, _), 0)
            | (Self::Receiver(first, _), 0) => Some(first),
            (Self::Two(_, second) | Self::Three(_, second, _), 1) => Some(second),
            (Self::Three(_, _, third), 2) => Some(third),
            (Self::Each(nodes), index) => nodes.get(index),
            (Self::Receiver(_, arguments), index) => arguments.get(index.checked_sub(1)?),
            (Self::One(_) | Self::Two(..) | Self::Three(..), _) => None,
        }
    }
}

/// What an operands frame keys once its operands are lowered.
enum Keyed {
    /// An application node.
    Application(Operator, Operation),
    /// A `value` node of this form, typed at this type node, whose body
    /// aggregates the operands.
    Value(&'static str, NodeKey),
}

/// A node over its operands' terms.
struct OperandsFrame<'n> {
    node: &'n Node,
    depth: u64,
    operands: Operands<'n>,
    /// The next operand to lower.
    next: usize,
    /// The terms so far: a call's callee reference, then each operand's.
    terms: Vec<SemanticTerm>,
    keyed: Keyed,
}

/// `let name = value in body`.
struct LetFrame<'n> {
    node: &'n Node,
    depth: u64,
    slot: Slot,
    value: &'n Node,
    body: &'n Node,
    /// The value's term and the binder's name, once the value is lowered.
    bound: Option<(SemanticTerm, String)>,
}

/// A one-binder operation's operands `[ref(source), binding{name,
/// ref(body)}]`, the binder typed at `source`'s element type.
struct BinderFrame<'n> {
    node: &'n Node,
    depth: u64,
    slot: Slot,
    source: &'n Node,
    body: &'n Node,
    operator: Operator,
    operation: Operation,
    /// The source's term and the binder's name, once the source is lowered.
    bound: Option<(SemanticTerm, String)>,
}

/// A record value's members, one binding per declared field.
struct RecordFrame<'n> {
    node: &'n Node,
    depth: u64,
    semantic_type: NodeKey,
    fields: Vec<FieldDeclaration>,
    slots: &'n [RecordSlot],
    members: Vec<SemanticTerm>,
}

/// `fold`/`reduce` over `source`.
struct FoldFrame<'n> {
    node: &'n Node,
    depth: u64,
    accumulator: Slot,
    binder: Slot,
    source: &'n Node,
    step: &'n Node,
    identity: Option<&'n Node>,
    stage: FoldStage,
}

#[allow(
    clippy::large_enum_variant,
    reason = "this stage lives inside the already boxed FoldFrame; boxing its SemanticTerm payload would add a second allocation per fold"
)]
enum FoldStage {
    /// The source is being lowered.
    Source,
    /// The identity is being lowered.
    Identity(SemanticTerm),
    /// The step is being lowered under the accumulator and element binders.
    Step {
        source: SemanticTerm,
        identity: Option<SemanticTerm>,
        accumulator: String,
        binder: String,
    },
}

/// `name`'s `::`-separated segments, each an identifier.
fn qualified_name(name: &str, location: &Location) -> Result<Vec<Identifier>, CheckRefusal> {
    name.split("::")
        .map(|segment| {
            Identifier::new(segment).map_err(|_| {
                refuse(
                    location,
                    CheckCause::NodePreimage(NodeKeyRefusal::EmptyQualifiedName),
                )
            })
        })
        .collect()
}

fn collection_form(kind: CollectionKind) -> &'static str {
    match kind {
        CollectionKind::Sequence => "sequence",
        CollectionKind::Set => "set",
        CollectionKind::Bag => "bag",
        CollectionKind::OrderedSet => "ordered_set",
    }
}

fn arithmetic_suffix(operator: ArithmeticOperator) -> &'static str {
    match operator {
        ArithmeticOperator::Add => "add",
        ArithmeticOperator::Subtract => "sub",
        ArithmeticOperator::Multiply => "mul",
        ArithmeticOperator::Divide => "div",
    }
}

fn ordering_suffix(operator: OrderingOperator) -> &'static str {
    match operator {
        OrderingOperator::Less => "lt",
        OrderingOperator::LessOrEqual => "le",
        OrderingOperator::Greater => "gt",
        OrderingOperator::GreaterOrEqual => "ge",
    }
}

/// Which leaves an operation's catalog entry names (FR-093 "Application
/// nodes").
#[derive(Clone, Copy)]
enum LeafSource<'t> {
    /// Every text leaf of this compared type (`operand:0`, `inner:0`).
    Compared(&'t ValueType),
    /// Every text leaf of the element type of this `set`, `bag` or
    /// `ordered_set` result (`result_inner`).
    ResultInner(&'t ValueType),
}

/// One leaf the text-leaf walk appends at its path.
#[derive(Clone, Copy)]
enum Leaf {
    /// A text leaf of this profile.
    Text(TextProfile),
    /// A recursion leaf into the composite entered after this many segments.
    Recursion(usize),
}

/// One path segment as the walk holds it: a field name borrowed from its
/// declaration, so the walk allocates no name per step.
#[derive(Clone, Copy)]
enum WalkSegment<'w> {
    /// `field:<name>`, the name already checked to be an identifier.
    Field(&'w str),
    /// `position:<n>`.
    Position(u64),
    /// `inner`.
    Inner,
}

impl WalkSegment<'_> {
    /// The segment's key spelling's byte length (`field:<name>`,
    /// `position:<n>`, `inner`): what one materialized leaf path costs for
    /// this segment.
    fn key_bytes(self) -> u64 {
        match self {
            Self::Field(name) => key_length("field:".len() + name.len()),
            Self::Position(position) => key_length("position:".len() + decimal_digits(position)),
            Self::Inner => key_length("inner".len()),
        }
    }

    /// The key segment itself.
    fn materialize(self) -> Result<LeafSegment, NodeKeyRefusal> {
        Ok(match self {
            Self::Field(name) => LeafSegment::Field(
                Identifier::new(name).map_err(|_| NodeKeyRefusal::EmptyBindingName)?,
            ),
            Self::Position(position) => LeafSegment::Position(position),
            Self::Inner => LeafSegment::Inner,
        })
    }
}

/// `length` as a charge amount.
fn key_length(length: usize) -> u64 {
    quire_exact::length_amount(length)
}

/// The number of decimal digits in `value`.
fn decimal_digits(value: u64) -> usize {
    value.checked_ilog10().map_or(1, |digits| {
        usize::try_from(digits).map_or(usize::MAX, |d| d + 1)
    })
}

/// One interned path prefix: its last segment, its parent prefix and the
/// key bytes of the whole prefix. Leaves that share a prefix share its
/// nodes, so the walk holds each prefix once however many leaves end
/// under it.
struct PathNode<'w> {
    parent: Option<usize>,
    segment: WalkSegment<'w>,
    key_bytes: u64,
}

/// One segment of the walk's current path, and its interned node once a
/// leaf under it has been appended.
struct StackEntry<'w> {
    segment: WalkSegment<'w>,
    interned: Option<usize>,
}

/// One FR-093 text-leaf walk ("Text leaves").
struct LeafWalk<'w> {
    /// The package's declared composites.
    types: &'w TypeEnvironment,
    /// The check stage's depth limit.
    depth_limit: u64,
    /// The checking work meter: each composite the walk enters charges one
    /// unit, and each leaf charges its materialized key bytes.
    meter: &'w mut Meter,
    /// Whether a text type is reachable from each composite asked so far.
    reach: &'w mut BTreeMap<NodeKey, bool>,
    /// The path from the compared type.
    stack: Vec<StackEntry<'w>>,
    /// Every path prefix a leaf was appended under, shared between leaves.
    prefixes: Vec<PathNode<'w>>,
    /// The leaves appended so far, each at its interned path (`None` for
    /// the empty path).
    leaves: Vec<(Option<usize>, Leaf)>,
    /// The open composites, each with the path length it was entered at.
    open: Vec<(NodeKey, usize)>,
    /// The node-limit units left.
    budget: u64,
    /// The node limit, which a refusal names.
    limit: u64,
}

impl<'w> LeafWalk<'w> {
    /// Walk `value_type` at the walk's path, appending each text leaf and
    /// recursion leaf in the order the walk reaches it.
    fn walk(
        &mut self,
        value_type: &'w ValueType,
        location: &Location,
        depth: u64,
    ) -> Result<(), CheckRefusal> {
        if depth > self.depth_limit {
            return Err(refuse(
                location,
                CheckCause::ResourceExhausted {
                    stage: CheckingStage::Typing,
                    kind: CheckingLimitKind::Depth,
                    limit: self.depth_limit,
                },
            ));
        }
        match value_type {
            // Rule 1.
            ValueType::Text(text) => self.append(Leaf::Text(text.profile()), location)?,
            // Rule 2.
            ValueType::Option(payload) => {
                self.within(WalkSegment::Inner, payload, location, depth + 1)?;
            }
            ValueType::Collection(collection) => {
                self.within(
                    WalkSegment::Inner,
                    collection.element(),
                    location,
                    depth + 1,
                )?;
            }
            ValueType::Composite(declaration) => {
                // A composite from which no text type is reachable adds no
                // leaf, open or not, so the walk does not enter it (rules 3
                // and 4).
                if !self.reaches_text(*declaration, location)? {
                    return Ok(());
                }
                // Rule 3: an open composite ends the path.
                if let Some(entered) = self.entered(*declaration) {
                    return self.append(Leaf::Recursion(entered), location);
                }
                // Rules 4 and 5.
                let types = self.types;
                let Some(composite) = types.composite(*declaration) else {
                    return Err(fault(location, KeyFault::UnknownComposite(*declaration)));
                };
                charge_work(self.meter, 1)
                    .map_err(|refusal| preimage_refusal(location, refusal))?;
                self.open.push((*declaration, self.stack.len()));
                let walked = self.fields(composite.shape(), location, depth);
                self.open.pop();
                walked?;
            }
            // Rule 6.
            ValueType::Boolean
            | ValueType::Integer
            | ValueType::Int(_)
            | ValueType::Rational(_)
            | ValueType::Decimal(_)
            | ValueType::Float(_)
            | ValueType::Quantity(_)
            | ValueType::Enum(_)
            | ValueType::Reference(_)
            | ValueType::Population(_) => {}
        }
        Ok(())
    }

    /// Walk `value_type` one `segment` further down the path.
    fn within(
        &mut self,
        segment: WalkSegment<'w>,
        value_type: &'w ValueType,
        location: &Location,
        depth: u64,
    ) -> Result<(), CheckRefusal> {
        self.stack.push(StackEntry {
            segment,
            interned: None,
        });
        let walked = self.walk(value_type, location, depth);
        self.stack.pop();
        walked
    }

    /// Rules 4 and 5: a record's fields, an optional one's through `inner`,
    /// or a tuple's positions.
    fn fields(
        &mut self,
        shape: &'w CompositeShape,
        location: &Location,
        depth: u64,
    ) -> Result<(), CheckRefusal> {
        match shape {
            CompositeShape::Record(fields) => {
                for field in fields {
                    let name = field.name();
                    if !quire_exact::is_identifier(name) {
                        return Err(refuse(
                            location,
                            CheckCause::NodePreimage(NodeKeyRefusal::EmptyBindingName),
                        ));
                    }
                    self.stack.push(StackEntry {
                        segment: WalkSegment::Field(name),
                        interned: None,
                    });
                    let walked = if field.presence() == Presence::Optional {
                        self.within(WalkSegment::Inner, field.value_type(), location, depth + 1)
                    } else {
                        self.walk(field.value_type(), location, depth + 1)
                    };
                    self.stack.pop();
                    walked?;
                }
            }
            CompositeShape::Tuple(positions) => {
                for (position, value_type) in (0_u64..).zip(positions) {
                    self.within(
                        WalkSegment::Position(position),
                        value_type,
                        location,
                        depth + 1,
                    )?;
                }
            }
        }
        Ok(())
    }

    /// Whether a text type is reachable from the composite `declaration`
    /// (FR-093 "Text leaves"), each composite visited once and the answer
    /// kept for the lowering's later walks.
    fn reaches_text(
        &mut self,
        declaration: NodeKey,
        location: &Location,
    ) -> Result<bool, CheckRefusal> {
        if let Some(reaches) = self.reach.get(&declaration) {
            return Ok(*reaches);
        }
        let types = self.types;
        let mut visited = BTreeSet::new();
        let root = ValueType::Composite(declaration);
        let mut pending = vec![&root];
        let mut reaches = false;
        while let Some(value_type) = pending.pop() {
            match value_type {
                ValueType::Text(_) => {
                    reaches = true;
                    break;
                }
                ValueType::Option(payload) => pending.push(payload),
                ValueType::Collection(collection) => pending.push(collection.element()),
                ValueType::Composite(composite) => {
                    if self.reach.get(composite) == Some(&true) {
                        reaches = true;
                        break;
                    }
                    if !visited.insert(*composite) {
                        continue;
                    }
                    match types.composite(*composite).map(|c| c.shape()) {
                        Some(CompositeShape::Record(fields)) => {
                            pending.extend(fields.iter().map(|field| field.value_type()));
                        }
                        Some(CompositeShape::Tuple(positions)) => pending.extend(positions.iter()),
                        None => {
                            return Err(fault(location, KeyFault::UnknownComposite(*composite)))
                        }
                    }
                }
                ValueType::Boolean
                | ValueType::Integer
                | ValueType::Int(_)
                | ValueType::Rational(_)
                | ValueType::Decimal(_)
                | ValueType::Float(_)
                | ValueType::Quantity(_)
                | ValueType::Enum(_)
                | ValueType::Reference(_)
                | ValueType::Population(_) => {}
            }
        }
        if !reaches {
            // No composite this search visited reaches a text type.
            for composite in visited {
                self.reach.insert(composite, false);
            }
        }
        self.reach.insert(declaration, reaches);
        Ok(reaches)
    }

    /// The path length `declaration` was entered at, when it is open.
    fn entered(&self, declaration: NodeKey) -> Option<usize> {
        self.open
            .iter()
            .find(|(open, _)| *open == declaration)
            .map(|(_, entered)| *entered)
    }

    /// Intern the current path, sharing every prefix an earlier leaf
    /// interned, and return its last node (`None` for the empty path).
    fn intern(&mut self) -> Option<usize> {
        let mut parent = None;
        for entry in &mut self.stack {
            let node = match entry.interned {
                Some(node) => node,
                None => {
                    let above = parent.map_or(0, |parent: usize| self.prefixes[parent].key_bytes);
                    self.prefixes.push(PathNode {
                        parent,
                        segment: entry.segment,
                        key_bytes: above.saturating_add(entry.segment.key_bytes()),
                    });
                    let node = self.prefixes.len() - 1;
                    entry.interned = Some(node);
                    node
                }
            };
            parent = Some(node);
        }
        parent
    }

    /// Append `leaf` at the current path, charging one unit of the node
    /// limit and then the leaf's materialized key bytes to the work meter.
    fn append(&mut self, leaf: Leaf, location: &Location) -> Result<(), CheckRefusal> {
        self.budget = self.budget.checked_sub(1).ok_or_else(|| {
            refuse(
                location,
                CheckCause::ResourceExhausted {
                    stage: CheckingStage::Typing,
                    kind: CheckingLimitKind::Nodes,
                    limit: self.limit,
                },
            )
        })?;
        let path = self.intern();
        let tail = match leaf {
            Leaf::Text(_) => 0,
            Leaf::Recursion(entered) => key_length(
                "recursion:".len() + decimal_digits(u64::try_from(entered).unwrap_or(u64::MAX)),
            ),
        };
        let bytes = path
            .map_or(0, |node| self.prefixes[node].key_bytes)
            .saturating_add(tail);
        charge_work(self.meter, bytes).map_err(|refusal| preimage_refusal(location, refusal))?;
        self.leaves.push((path, leaf));
        Ok(())
    }

    /// The segments of the interned path ending at `node`, root first.
    fn path(&self, node: Option<usize>) -> Result<Vec<LeafSegment>, NodeKeyRefusal> {
        let mut segments = Vec::new();
        let mut at = node;
        while let Some(index) = at {
            let prefix = &self.prefixes[index];
            segments.push(prefix.segment.materialize()?);
            at = prefix.parent;
        }
        segments.reverse();
        Ok(segments)
    }
}

impl<'a> Lowering<'a> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        scope: &'a Scope,
        owner: &SourceOwner,
        models: &'a [AdmittedModel],
        units: UnitTable,
        lock: &'a LockEvidence,
        depth_limit: u64,
        function_count: usize,
        occurrences: &'a mut OccurrenceMap<Location>,
        meter: &'a mut Meter,
    ) -> Self {
        Self {
            scope,
            owner: Owner::Source(owner.clone()),
            models,
            units,
            correspondence: BTreeMap::new(),
            lock,
            depth_limit,
            graph: SemanticGraph::default(),
            occurrences,
            functions: vec![None; function_count],
            composites: BTreeMap::new(),
            unsettled_functions: Vec::new(),
            unsettled_composites: Vec::new(),
            composites_in_progress: Vec::new(),
            composite_placeholders: BTreeMap::new(),
            functions_open: false,
            placeholders: BTreeMap::new(),
            placeholder_count: 0,
            drafts: BTreeMap::new(),
            keys: HashMap::new(),
            content_hashes: BuildHasherDefault::default(),
            draft_occurrences: Vec::new(),
            rebuilt_members: BTreeMap::new(),
            group_of: BTreeMap::new(),
            group_regions: BTreeMap::new(),
            meter,
            node_limit: u64::MAX,
            node_budget: u64::MAX,
            text_reach: BTreeMap::new(),
        }
    }

    /// Bound the text-leaf walks by the node limit `limit`, of which typing
    /// used `used`.
    pub(crate) fn with_node_limit(mut self, limit: u64, used: u64) -> Self {
        self.node_limit = limit;
        self.node_budget = limit.saturating_sub(used);
        self
    }

    /// Charge `work` units of checking work (`declaration.check`), refusing
    /// `resource_exhausted` on the work budget at `location`.
    fn charge(&mut self, work: u64, location: &Location) -> Result<(), CheckRefusal> {
        charge_work(self.meter, work).map_err(|refusal| preimage_refusal(location, refusal))
    }

    /// The finished graph, its model correspondence entries and each
    /// function's key. Every node that no region denotes gets its one
    /// `generated` occurrence (FR-093), at `root`.
    pub(crate) fn finish(self, root: &Location) -> Lowered {
        for key in self.graph.nodes.keys() {
            if !self.occurrences.has(*key) {
                self.occurrences.record(*key, "generated", root.clone());
            }
        }
        Lowered {
            graph: self.graph,
            correspondence: model::correspondence_entries(self.correspondence),
            functions: self.functions,
        }
    }

    /// Record an occurrence of `key`: a draft's waits until it is keyed.
    fn record(&mut self, key: NodeKey, role: &'static str, location: Location) {
        if self.pending(key) {
            self.draft_occurrences.push((key, role, location));
        } else {
            self.occurrences.record(key, role, location);
        }
    }

    /// Whether `key` names a placeholder or a draft.
    fn pending(&self, key: NodeKey) -> bool {
        self.placeholders.contains_key(&key) || self.drafts.contains_key(&key)
    }

    /// A new placeholder: a digest over a counter under a label no preimage
    /// begins with, so it equals no node key.
    fn placeholder(&mut self) -> NodeKey {
        self.placeholder_count += 1;
        let label = b"qsl.check.lowering-placeholder\0";
        let preimage = [&label[..], &self.placeholder_count.to_be_bytes()].concat();
        let key = NodeKey::from_digest(qsl_foundation::ByteDigest::of(&preimage).as_bytes());
        self.placeholders.insert(key, None);
        key
    }

    /// Key an unowned or source-declared node and add it to the graph: a
    /// node with a `declaration` carries the unit's `SourceOwner`.
    fn insert(
        &mut self,
        location: &Location,
        node_tag: NodeTag,
        semantic_form: &'static str,
        semantic_type: Option<NodeKey>,
        declaration: Option<Vec<Identifier>>,
        body: SemanticTerm,
    ) -> Result<NodeKey, CheckRefusal> {
        let owner = declaration.as_ref().map(|_| self.owner.clone());
        self.insert_node(
            location,
            NodeContent {
                node_tag,
                semantic_form,
                semantic_type,
                declaration,
                owner,
                body,
            },
        )
    }

    /// Key a model-owned node (FR-094): `owner`, no `declaration`.
    fn insert_owned(
        &mut self,
        location: &Location,
        node_tag: NodeTag,
        semantic_form: &'static str,
        semantic_type: Option<NodeKey>,
        owner: Owner,
        body: SemanticTerm,
    ) -> Result<NodeKey, CheckRefusal> {
        self.insert_node(
            location,
            NodeContent {
                node_tag,
                semantic_form,
                semantic_type,
                declaration: None,
                owner: Some(owner),
                body,
            },
        )
    }

    /// Key a node from every preimage member and add it to the graph.
    fn insert_node(
        &mut self,
        location: &Location,
        content: NodeContent,
    ) -> Result<NodeKey, CheckRefusal> {
        self.charge(1, location)?;
        let hash = self.content_hashes.hash_one(&content);
        let known = self
            .keys
            .get(&hash)
            .copied()
            .filter(|key| self.holds(*key, &content));
        // The preimage bytes, when this call encoded them: a content keyed
        // before is encoded again only if its node is not in the graph yet.
        let (key, preimage) = match known {
            Some(key) => (key, None),
            None => {
                let keyed = node_key(&content.input())
                    .map_err(|refusal| preimage_refusal(location, refusal))?;
                self.keys.insert(hash, keyed.key);
                (keyed.key, Some(keyed.preimage))
            }
        };
        if let Some(member) = self.rebuilt_members.get(&key) {
            return Ok(*member);
        }
        let mut names_pending = content
            .semantic_type
            .is_some_and(|named| self.pending(named));
        content
            .body
            .for_each_key(&mut |named| names_pending |= self.pending(named));
        if names_pending {
            // Keyed by `settle`; until then `key` digests the content with
            // its placeholders, so equal drafts are one draft.
            self.drafts.entry(key).or_insert_with(|| Draft {
                location: location.clone(),
                content,
            });
            return Ok(key);
        }
        if let btree_map::Entry::Vacant(slot) = self.graph.nodes.entry(key) {
            let preimage = match preimage {
                Some(preimage) => preimage,
                None => {
                    node_key(&content.input())
                        .map_err(|refusal| preimage_refusal(location, refusal))?
                        .preimage
                }
            };
            slot.insert(SemanticNode {
                key,
                preimage,
                recursion: None,
                content,
            });
        }
        Ok(key)
    }

    /// Whether `key` is [`node_key`]'s key of `content`: a graph node
    /// outside every recursion group or a draft holds exactly the content
    /// its key keys, so an equal content has that key.
    fn holds(&self, key: NodeKey, content: &NodeContent) -> bool {
        match self.graph.nodes.get(&key) {
            Some(node) => node.recursion.is_none() && node.content == *content,
            None => self
                .drafts
                .get(&key)
                .is_some_and(|draft| draft.content == *content),
        }
    }

    fn check_depth(&self, depth: u64, location: &Location) -> Result<(), CheckRefusal> {
        if depth > self.depth_limit {
            return Err(refuse(
                location,
                CheckCause::ResourceExhausted {
                    stage: CheckingStage::Typing,
                    kind: CheckingLimitKind::Depth,
                    limit: self.depth_limit,
                },
            ));
        }
        Ok(())
    }

    // ------------------------------------------------------------------
    // FR-092 type nodes
    // ------------------------------------------------------------------

    /// A builtin `scalar_type` node: its own semantic type, empty body.
    fn scalar(&mut self, form: &'static str, location: &Location) -> Result<NodeKey, CheckRefusal> {
        self.insert(
            location,
            NodeTag::ScalarType,
            form,
            None,
            None,
            SemanticTerm::Aggregate {
                members: Vec::new(),
            },
        )
    }

    /// An integer literal typed at the builtin `Integer` node (T2).
    fn integer_literal(
        &mut self,
        value: Integer,
        location: &Location,
    ) -> Result<SemanticTerm, CheckRefusal> {
        let integer = self.scalar("integer", location)?;
        Ok(SemanticTerm::literal(integer, LiteralValue::Integer(value)))
    }

    /// A text literal typed at the builtin text scalar node (T3).
    fn text_literal(
        &mut self,
        value: &str,
        location: &Location,
    ) -> Result<SemanticTerm, CheckRefusal> {
        let text = self.scalar("text", location)?;
        Ok(SemanticTerm::literal(
            text,
            LiteralValue::Text(value.to_owned()),
        ))
    }

    /// A `bounded_domain` node over `base` with `bindings`.
    fn bounded(
        &mut self,
        form: &'static str,
        base: NodeKey,
        bindings: Vec<(&'static str, SemanticTerm)>,
        location: &Location,
    ) -> Result<NodeKey, CheckRefusal> {
        let members = bindings
            .into_iter()
            .map(|(name, value)| SemanticTerm::binding(name, value))
            .collect();
        self.insert(
            location,
            NodeTag::BoundedDomain,
            form,
            Some(base),
            None,
            SemanticTerm::Aggregate { members },
        )
    }

    /// The FR-092 type node of `value_type`, built without native
    /// recursion (QSL-224): each `option`, collection and declared
    /// composite still being built is a [`TypeFrame`] on an explicit stack,
    /// so a long composite chain costs heap, not stack. Nodes are built,
    /// charged and refused in the order a depth-first recursive build would
    /// reach them, so every key is the recursive build's key.
    pub(crate) fn type_node<'v>(
        &mut self,
        value_type: &'v ValueType,
        location: &Location,
    ) -> Result<NodeKey, CheckRefusal>
    where
        'a: 'v,
    {
        let open = self.composites_in_progress.len();
        let built = self.build_type_node(value_type, location);
        if built.is_err() {
            // The composites a refusal left mid-build are no longer being
            // built.
            self.composites_in_progress.truncate(open);
        }
        built
    }

    /// [`Self::type_node`]'s loop: descend into a type, or hand a built
    /// node's key to the frame waiting for it.
    fn build_type_node<'v>(
        &mut self,
        value_type: &'v ValueType,
        location: &Location,
    ) -> Result<NodeKey, CheckRefusal>
    where
        'a: 'v,
    {
        let mut frames: Vec<TypeFrame<'v>> = Vec::new();
        let mut step = TypeStep::Descend(value_type, 0);
        loop {
            step = match step {
                TypeStep::Descend(value_type, depth) => {
                    self.descend(value_type, location, depth, &mut frames)?
                }
                TypeStep::Built(key) => match frames.pop() {
                    None => return Ok(key),
                    Some(TypeFrame::Option) => TypeStep::Built(self.option_type(key, location)?),
                    Some(TypeFrame::Collection(collection)) => {
                        TypeStep::Built(self.collection_type(collection, key, location)?)
                    }
                    Some(TypeFrame::Composite(mut composite)) => {
                        composite.accept(key);
                        self.advance(composite, location, &mut frames)?
                    }
                },
            };
        }
    }

    /// Start `value_type`'s node at `depth`: a type with no type inside it
    /// is built now; an `option`, a collection or a composite not yet built
    /// pushes its frame and descends into its first inner type.
    fn descend<'v>(
        &mut self,
        value_type: &'v ValueType,
        location: &Location,
        depth: u64,
        frames: &mut Vec<TypeFrame<'v>>,
    ) -> Result<TypeStep<'v>, CheckRefusal>
    where
        'a: 'v,
    {
        self.check_depth(depth, location)?;
        let key = match value_type {
            ValueType::Option(payload) => {
                frames.push(TypeFrame::Option);
                return Ok(TypeStep::Descend(payload, depth + 1));
            }
            ValueType::Collection(collection) => {
                frames.push(TypeFrame::Collection(collection));
                return Ok(TypeStep::Descend(collection.element(), depth + 1));
            }
            ValueType::Composite(declaration) => {
                return match self.open_composite(*declaration, location, depth)? {
                    OpenComposite::Built(key) => Ok(TypeStep::Built(key)),
                    OpenComposite::Open(composite) => self.advance(composite, location, frames),
                };
            }
            ValueType::Boolean => self.scalar("boolean", location),
            ValueType::Integer => self.scalar("integer", location),
            ValueType::Int(interval) => {
                let base = self.scalar("integer", location)?;
                let min = self.integer_literal(interval.lower().clone(), location)?;
                let max = self.integer_literal(interval.upper().clone(), location)?;
                self.bounded(
                    "integer_range",
                    base,
                    vec![("min", min), ("max", max)],
                    location,
                )
            }
            ValueType::Rational(domain) => {
                let base = self.scalar("rational", location)?;
                let numerator_min =
                    self.integer_literal(domain.numerator().lower().clone(), location)?;
                let numerator_max =
                    self.integer_literal(domain.numerator().upper().clone(), location)?;
                let denominator_min =
                    self.integer_literal(domain.denominator().lower().clone(), location)?;
                let denominator_max =
                    self.integer_literal(domain.denominator().upper().clone(), location)?;
                self.bounded(
                    "rational_range",
                    base,
                    vec![
                        ("numerator_min", numerator_min),
                        ("numerator_max", numerator_max),
                        ("denominator_min", denominator_min),
                        ("denominator_max", denominator_max),
                    ],
                    location,
                )
            }
            ValueType::Decimal(decimal) => {
                let base = self.scalar("decimal", location)?;
                let coefficient_min = self.integer_literal(decimal.lower().clone(), location)?;
                let coefficient_max = self.integer_literal(decimal.upper().clone(), location)?;
                let scale_min =
                    self.integer_literal(Integer::from(u64::from(decimal.min_scale())), location)?;
                let scale_max =
                    self.integer_literal(Integer::from(u64::from(decimal.max_scale())), location)?;
                let rounding = self.text_literal(decimal.rounding().as_str(), location)?;
                self.bounded(
                    "decimal_range",
                    base,
                    vec![
                        ("coefficient_min", coefficient_min),
                        ("coefficient_max", coefficient_max),
                        ("scale_min", scale_min),
                        ("scale_max", scale_max),
                        ("rounding", rounding),
                    ],
                    location,
                )
            }
            ValueType::Float(width) => {
                let form = match width {
                    quire_exact::IeeeWidth::Binary32 => "float32",
                    quire_exact::IeeeWidth::Binary64 => "float64",
                };
                let base = self.scalar(form, location)?;
                // The checker admits only the omitted, strict `exact`
                // rounding spelling (`NodeKind::Ieee`'s own doc).
                let rounding =
                    self.text_literal(quire_exact::RoundingMode::Exact.as_str(), location)?;
                self.bounded(
                    "float_rounding",
                    base,
                    vec![("rounding", rounding)],
                    location,
                )
            }
            ValueType::Text(text) => {
                let base = self.scalar("text", location)?;
                let min = self.integer_literal(Integer::from(text.min()), location)?;
                let max = self.integer_literal(Integer::from(text.max()), location)?;
                let profile = self.text_literal(text.profile().as_str(), location)?;
                self.bounded(
                    "text_bounds",
                    base,
                    vec![("min", min), ("max", max), ("text_profile", profile)],
                    location,
                )
            }
            ValueType::Enum(shape) => {
                // FR-092 rule 1: an enum type is its QSpec nominal
                // declaration node, `value::enumeration`'s key.
                let declaration = self
                    .scope
                    .enum_binding_of(shape)
                    .map(|binding| binding.declaration.key());
                declaration.ok_or_else(|| {
                    refuse(
                        location,
                        CheckCause::IllTyped(quire_exact::IllTypedCause::TypeMismatch),
                    )
                })
            }
            ValueType::Quantity(unit) => self.quantity_type(*unit, location),
            ValueType::Reference(target) => self.reference_type(*target, location),
            // FR-094: a `Population<T>[N]` node is built from its binding's
            // resolved type form ([`Self::binder_type`]); the checked type
            // alone carries no `T`.
            ValueType::Population(_) => Err(fault(location, KeyFault::UntargetedPopulation)),
        }?;
        Ok(TypeStep::Built(key))
    }

    /// The `option` node over the payload node `payload`.
    fn option_type(
        &mut self,
        payload: NodeKey,
        location: &Location,
    ) -> Result<NodeKey, CheckRefusal> {
        self.insert(
            location,
            NodeTag::CompositeType,
            "option",
            None,
            None,
            SemanticTerm::Aggregate {
                members: vec![SemanticTerm::reference(payload)],
            },
        )
    }

    /// `collection`'s bounded node over the element node `element`.
    fn collection_type(
        &mut self,
        collection: &CollectionType,
        element: NodeKey,
        location: &Location,
    ) -> Result<NodeKey, CheckRefusal> {
        let base = self.insert(
            location,
            NodeTag::CompositeType,
            collection_form(collection.kind()),
            None,
            None,
            SemanticTerm::Aggregate {
                members: vec![SemanticTerm::reference(element)],
            },
        )?;
        let bound = collection.bound();
        let min = self.integer_literal(Integer::from(bound.minimum()), location)?;
        let max = self.integer_literal(Integer::from(bound.maximum()), location)?;
        self.bounded(
            "collection_bounds",
            base,
            vec![("min", min), ("max", max)],
            location,
        )
    }

    /// Start the declared record or tuple `declaration` at `depth`: its node
    /// if it is built, its placeholder if it is being built (it reached
    /// itself), or its open frame.
    fn open_composite<'v>(
        &mut self,
        declaration: NodeKey,
        location: &Location,
        depth: u64,
    ) -> Result<OpenComposite<'v>, CheckRefusal>
    where
        'a: 'v,
    {
        if let Some(key) = self.composites.get(&declaration) {
            return Ok(OpenComposite::Built(*key));
        }
        let scope: &'a Scope = self.scope;
        let Some(composite) = scope.types().composite(declaration) else {
            return Err(refuse(
                location,
                CheckCause::IllTyped(quire_exact::IllTypedCause::TypeMismatch),
            ));
        };
        if self.composites_in_progress.contains(&declaration) {
            // FR-092: a record reaching itself is in a recursion group; a
            // placeholder names it until its node is built.
            if let Some(placeholder) = self.composite_placeholders.get(&declaration) {
                return Ok(OpenComposite::Built(*placeholder));
            }
            let placeholder = self.placeholder();
            self.composite_placeholders.insert(declaration, placeholder);
            return Ok(OpenComposite::Built(placeholder));
        }
        let name = qualified_name(composite.name(), location)?;
        self.composites_in_progress.push(declaration);
        let shape = composite.shape();
        let members = match shape {
            CompositeShape::Record(fields) => fields.len(),
            CompositeShape::Tuple(positions) => positions.len(),
        };
        Ok(OpenComposite::Open(CompositeFrame {
            declaration,
            name,
            shape,
            depth,
            members: Vec::with_capacity(members),
            awaiting: None,
        }))
    }

    /// Descend into `composite`'s next field or position, or, when every
    /// member is built, finish its node.
    fn advance<'v>(
        &mut self,
        mut composite: CompositeFrame<'v>,
        location: &Location,
        frames: &mut Vec<TypeFrame<'v>>,
    ) -> Result<TypeStep<'v>, CheckRefusal> {
        let member = composite.depth + 1;
        let at = composite.members.len();
        let next = match composite.shape {
            CompositeShape::Record(fields) => fields.get(at).map(|field| {
                composite.awaiting = Some(field);
                (field.value_type(), field.presence())
            }),
            CompositeShape::Tuple(positions) => positions
                .get(at)
                .map(|position| (position, Presence::Required)),
        };
        let Some((value_type, presence)) = next else {
            return Ok(TypeStep::Built(self.close_composite(composite, location)?));
        };
        frames.push(TypeFrame::Composite(composite));
        Ok(match presence {
            Presence::Required => TypeStep::Descend(value_type, member),
            // An optional field's member is the `option` node over its
            // declared type, one level further down.
            Presence::Optional => {
                self.check_depth(member, location)?;
                frames.push(TypeFrame::Option);
                TypeStep::Descend(value_type, member + 1)
            }
        })
    }

    /// Key the composite whose every member is built, and settle once no
    /// composite or function is still open.
    fn close_composite(
        &mut self,
        composite: CompositeFrame<'_>,
        location: &Location,
    ) -> Result<NodeKey, CheckRefusal> {
        let CompositeFrame {
            declaration,
            name,
            shape,
            members,
            ..
        } = composite;
        self.composites_in_progress.pop();
        let form = match shape {
            CompositeShape::Record(_) => "record",
            CompositeShape::Tuple(_) => "tuple",
        };
        let key = self.insert(
            location,
            NodeTag::CompositeType,
            form,
            None,
            Some(name),
            SemanticTerm::Aggregate { members },
        )?;
        if let Some(placeholder) = self.composite_placeholders.remove(&declaration) {
            self.placeholders.insert(placeholder, Some(key));
        }
        self.composites.insert(declaration, key);
        self.unsettled_composites.push(declaration);
        if self.composites_in_progress.is_empty() && !self.functions_open {
            self.settle()?;
        }
        Ok(self.composites.get(&declaration).copied().unwrap_or(key))
    }

    /// A declared record or tuple's node: its `declaration` and the unit's
    /// `owner`, one binding per field (record) or one reference per position
    /// (tuple).
    fn composite(
        &mut self,
        declaration: NodeKey,
        location: &Location,
    ) -> Result<NodeKey, CheckRefusal> {
        self.type_node(&ValueType::Composite(declaration), location)
    }

    /// The node of the declared composite `declaration` (FR-092-AC-12): its
    /// checked type node's id.
    pub(crate) fn composite_node(&mut self, declaration: NodeKey) -> Result<NodeKey, CheckRefusal> {
        self.composite(declaration, &generated_location())
    }

    /// The law of `role` the lock evidence selects, or the FR-093 refusal.
    fn law(&self, role: LawRole, location: &Location) -> Result<OperationLaw, CheckRefusal> {
        let definition = match role {
            LawRole::TextProfile => self.lock.text_profile(),
            LawRole::IeeeProfile => self
                .scope
                .ieee_profile
                .as_ref()
                .map(|profile| profile.definition()),
            LawRole::IntegerDivision | LawRole::TemporalProfile | LawRole::ProtocolProfile => None,
        };
        definition
            .map(|definition| OperationLaw {
                role,
                definition: definition.clone(),
            })
            .ok_or_else(|| refuse(location, CheckCause::MissingSelection { role }))
    }

    /// The `leaves` of an operation whose catalog entry names `source`.
    fn leaves(
        &mut self,
        source: LeafSource<'_>,
        location: &Location,
    ) -> Result<Vec<OperationLeaf>, CheckRefusal> {
        let compared = match source {
            LeafSource::Compared(value_type) => Some(value_type),
            LeafSource::ResultInner(value_type) => match value_type {
                ValueType::Collection(collection)
                    if collection.kind() != CollectionKind::Sequence =>
                {
                    Some(collection.element())
                }
                _ => None,
            },
        };
        let Some(compared) = compared else {
            return Ok(Vec::new());
        };
        // The walk completes, or refuses on the node limit or the work
        // budget, before any leaf's law is read (FR-093 "Text leaves").
        let mut walk = LeafWalk {
            types: self.scope.types(),
            depth_limit: self.depth_limit,
            meter: &mut *self.meter,
            reach: &mut self.text_reach,
            stack: Vec::new(),
            prefixes: Vec::new(),
            leaves: Vec::new(),
            open: Vec::new(),
            budget: self.node_budget,
            limit: self.node_limit,
        };
        walk.walk(compared, location, 0)?;
        self.node_budget = walk.budget;
        let found = std::mem::take(&mut walk.leaves);
        let paths = found
            .iter()
            .map(|(node, _)| walk.path(*node))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|refusal| preimage_refusal(location, refusal))?;
        found
            .into_iter()
            .zip(paths)
            .map(|((_, leaf), path)| {
                Ok(match leaf {
                    Leaf::Text(profile) => OperationLeaf {
                        path,
                        laws: vec![self.law(LawRole::TextProfile, location)?],
                        mode: Some(OperationMode::TextProfile(profile)),
                    },
                    Leaf::Recursion(entered) => OperationLeaf {
                        path: {
                            let mut path = path;
                            path.push(LeafSegment::Recursion(entered));
                            path
                        },
                        laws: Vec::new(),
                        mode: None,
                    },
                })
            })
            .collect()
    }

    // ------------------------------------------------------------------
    // FR-092 parameter and function nodes
    // ------------------------------------------------------------------

    /// The type node of a binder of `value_type`: for a `Population<T>[N]`
    /// binding, the FR-094 node over `target`, its resolved `T`.
    fn binder_type(
        &mut self,
        value_type: &ValueType,
        target: Option<EffectiveId>,
        location: &Location,
    ) -> Result<NodeKey, CheckRefusal> {
        match (value_type, target) {
            (ValueType::Population(maximum), Some(target)) => {
                self.population_type(target, *maximum, location)
            }
            _ => self.type_node(value_type, location),
        }
    }

    /// A binder's parameter node: `value`/`parameter`, typed at its binder's
    /// type node `semantic_type`, its body binding its name and level.
    fn parameter(
        &mut self,
        name: &str,
        level: usize,
        semantic_type: NodeKey,
        location: &Location,
    ) -> Result<NodeKey, CheckRefusal> {
        let name_literal = self.text_literal(name, location)?;
        let level_literal = self.integer_literal(Integer::from(level), location)?;
        let key = self.insert(
            location,
            NodeTag::Value,
            "parameter",
            Some(semantic_type),
            None,
            SemanticTerm::Aggregate {
                members: vec![
                    SemanticTerm::binding("name", name_literal),
                    SemanticTerm::binding("level", level_literal),
                ],
            },
        )?;
        self.record(key, "anchor", location.clone());
        Ok(key)
    }

    /// Lower and key the functions of `group`, each with its index. The
    /// functions of a recursive group name each other by placeholders until
    /// every one is built, then [`Self::settle`] keys them.
    pub(crate) fn function_group(
        &mut self,
        group: &FunctionGroup,
        functions: &[FunctionInput<'_>],
    ) -> Vec<CheckRefusal> {
        let mut refusals = Vec::new();
        let mut placeholders = Vec::new();
        if group.recursive {
            self.functions_open = true;
            for index in &group.members {
                let placeholder = self.placeholder();
                if let Some(slot) = self.functions.get_mut(*index) {
                    *slot = Some(placeholder);
                    self.unsettled_functions.push(*index);
                }
                placeholders.push(placeholder);
            }
        }
        for (position, (index, function)) in group.members.iter().zip(functions).enumerate() {
            match self.function(*index, function) {
                Ok(key) => {
                    if let Some(placeholder) = placeholders.get(position) {
                        self.placeholders.insert(*placeholder, Some(key));
                    }
                }
                Err(refusal) => refusals.push(refusal),
            }
        }
        self.functions_open = false;
        if let Err(refusal) = self.settle() {
            refusals.push(refusal);
        }
        refusals
    }

    /// Key every draft (FR-092 "Recursion groups"), once nothing is open:
    /// each strongly connected component of the drafts' names, in
    /// dependency order, a single draft by its own preimage and a recursion
    /// group by the group order. Then every key recorded under a draft or a
    /// placeholder becomes the node's key.
    fn settle(&mut self) -> Result<(), CheckRefusal> {
        let placeholders = std::mem::take(&mut self.placeholders);
        let drafts = std::mem::take(&mut self.drafts);
        let occurrences = std::mem::take(&mut self.draft_occurrences);
        let unsettled_functions = std::mem::take(&mut self.unsettled_functions);
        let unsettled_composites = std::mem::take(&mut self.unsettled_composites);
        self.composite_placeholders.clear();
        let mut aliases = BTreeMap::new();
        for (placeholder, built) in placeholders {
            match built {
                Some(built) => {
                    aliases.insert(placeholder, built);
                }
                // A placeholder's function or record refused, and that
                // refusal is already reported: the package has no graph.
                None => return Ok(()),
            }
        }
        let alias = |key: NodeKey| aliases.get(&key).copied().unwrap_or(key);
        let handles: Vec<NodeKey> = drafts.keys().copied().collect();
        let drafts: Vec<Draft> = drafts.into_values().collect();
        let position: BTreeMap<NodeKey, usize> = handles
            .iter()
            .enumerate()
            .map(|(position, handle)| (*handle, position))
            .collect();
        let edges: Vec<Vec<usize>> = drafts
            .iter()
            .map(|draft| {
                let mut named = Vec::new();
                let mut visit = |key: NodeKey| {
                    if let Some(target) = position.get(&alias(key)) {
                        named.push(*target);
                    }
                };
                if let Some(semantic_type) = draft.content.semantic_type {
                    visit(semantic_type);
                }
                draft.content.body.for_each_key(&mut visit);
                named
            })
            .collect();
        let mut resolved: BTreeMap<NodeKey, NodeKey> = BTreeMap::new();
        // A name of a draft that is keyed nowhere: dependency order keys
        // every draft outside a component before the component.
        let unresolved = |location: &Location| fault(location, KeyFault::UnresolvedDraft);
        for component in strongly_connected(&edges) {
            let members: Vec<NodeKey> = component.iter().map(|at| handles[*at]).collect();
            let member_set: BTreeSet<NodeKey> = members.iter().copied().collect();
            // Names outside the component take their keys; names inside it
            // stay the members' handles; other keys are not drafts.
            let mut stranded = false;
            let mut substitute = |key: NodeKey| {
                let key = alias(key);
                if member_set.contains(&key) {
                    key
                } else if let Some(resolved) = resolved.get(&key) {
                    *resolved
                } else {
                    stranded |= position.contains_key(&key);
                    key
                }
            };
            let bodies: Vec<SemanticTerm> = component
                .iter()
                .map(|at| drafts[*at].content.body.map_keys(&mut substitute))
                .collect();
            let types: Vec<Option<NodeKey>> = component
                .iter()
                .map(|at| drafts[*at].content.semantic_type.map(&mut substitute))
                .collect();
            if stranded {
                return Err(unresolved(&drafts[component[0]].location));
            }
            let recursive =
                component.len() > 1 || component.first().is_some_and(|at| edges[*at].contains(at));
            if recursive {
                self.key_group(
                    &component,
                    &drafts,
                    &members,
                    &bodies,
                    &types,
                    &mut resolved,
                )?;
            } else if let (Some(at), Some(body), Some(semantic_type)) =
                (component.first(), bodies.into_iter().next(), types.first())
            {
                let draft = &drafts[*at];
                let key = self.insert_node(
                    &draft.location,
                    NodeContent {
                        node_tag: draft.content.node_tag,
                        semantic_form: draft.content.semantic_form,
                        semantic_type: *semantic_type,
                        declaration: draft.content.declaration.clone(),
                        owner: draft.content.owner.clone(),
                        body,
                    },
                )?;
                resolved.insert(handles[*at], key);
            }
        }
        // Every draft is keyed now: a key that still names one is a fault.
        let resolve = |key: NodeKey| {
            let key = alias(key);
            match resolved.get(&key) {
                Some(resolved) => Ok(*resolved),
                None if position.contains_key(&key) => Err(()),
                None => Ok(key),
            }
        };
        let root = generated_location();
        // A key an earlier settle resolved names no draft or placeholder of
        // this one, so only the keys written since then can change.
        for index in unsettled_functions {
            if let Some(Some(key)) = self.functions.get_mut(index) {
                *key = resolve(*key).map_err(|()| unresolved(&root))?;
            }
        }
        for declaration in unsettled_composites {
            if let Some(key) = self.composites.get_mut(&declaration) {
                *key = resolve(*key).map_err(|()| unresolved(&root))?;
            }
        }
        for (key, role, location) in occurrences {
            let key = resolve(key).map_err(|()| unresolved(&location))?;
            self.occurrences.record(key, role, location);
        }
        Ok(())
    }

    /// Key the recursion group `component` of `drafts`, whose members are
    /// named by `handles` and whose names outside the group are keyed in
    /// `bodies` and `types`, and add its nodes. Two groups' members with
    /// equal keys refuse (FR-092 "Groups that collide").
    fn key_group(
        &mut self,
        component: &[usize],
        drafts: &[Draft],
        handles: &[NodeKey],
        bodies: &[SemanticTerm],
        types: &[Option<NodeKey>],
        resolved: &mut BTreeMap<NodeKey, NodeKey>,
    ) -> Result<(), CheckRefusal> {
        let members: Vec<&Draft> = component.iter().map(|at| &drafts[*at]).collect();
        let Some(first) = members.first() else {
            return Ok(());
        };
        // FR-092: a function in a recursion group is a `recursive_function`.
        let forms: Vec<&'static str> = members
            .iter()
            .map(|draft| {
                if draft.content.node_tag == NodeTag::Function {
                    "recursive_function"
                } else {
                    draft.content.semantic_form
                }
            })
            .collect();
        let inputs: Vec<NodeInput<'_>> = members
            .iter()
            .enumerate()
            .map(|(at, draft)| NodeInput {
                owner: draft.content.owner.as_ref(),
                node_tag: draft.content.node_tag,
                semantic_form: forms[at],
                semantic_type: types[at],
                declaration: draft.content.declaration.as_deref(),
                body: &bodies[at],
            })
            .collect();
        let meter = &mut *self.meter;
        let keys = group_keys(&inputs, handles, &mut |work| charge_work(meter, work))
            .map_err(|refusal| preimage_refusal(&first.location, refusal))?;
        let regions: Vec<Location> = members
            .iter()
            .filter(|draft| draft.content.declaration.is_some())
            .map(|draft| draft.location.clone())
            .collect();
        let collision = keys.members.iter().find_map(|keyed| {
            self.group_of
                .get(&keyed.key)
                .filter(|digest| **digest != keys.digest)
        });
        if let Some(other) = collision {
            let mut loci = self.group_regions.get(other).cloned().unwrap_or_default();
            loci.extend(regions);
            let location = loci
                .first()
                .cloned()
                .unwrap_or_else(|| first.location.clone());
            return Err(refuse(&location, CheckCause::UnsupportedFeature { loci }));
        }
        let member_keys: BTreeMap<NodeKey, NodeKey> = handles
            .iter()
            .zip(&keys.members)
            .map(|(handle, keyed)| (*handle, keyed.key))
            .collect();
        let mut to_key = |key: NodeKey| member_keys.get(&key).copied().unwrap_or(key);
        for (at, draft) in members.iter().enumerate() {
            let keyed = &keys.members[at];
            // The member's content outside the group, as a later build of
            // the same type or node would write it.
            let body = bodies[at].map_keys(&mut to_key);
            let semantic_type = types[at].map(&mut to_key);
            let rebuilt = node_key(&NodeInput {
                owner: draft.content.owner.as_ref(),
                node_tag: draft.content.node_tag,
                semantic_form: draft.content.semantic_form,
                semantic_type,
                declaration: draft.content.declaration.as_deref(),
                body: &body,
            })
            .map_err(|refusal| preimage_refusal(&draft.location, refusal))?;
            self.rebuilt_members.insert(rebuilt.key, keyed.key);
            self.graph
                .nodes
                .entry(keyed.key)
                .or_insert_with(|| SemanticNode {
                    key: keyed.key,
                    preimage: keyed.preimage.clone(),
                    recursion: Some(NodeRecursion {
                        group: keys.digest,
                        ordinal: keys.ordinals[at],
                        size: keys.size,
                    }),
                    content: NodeContent {
                        node_tag: draft.content.node_tag,
                        semantic_form: forms[at],
                        semantic_type,
                        declaration: draft.content.declaration.clone(),
                        owner: draft.content.owner.clone(),
                        body,
                    },
                });
            self.group_of.insert(keyed.key, keys.digest);
            resolved.insert(handles[at], keyed.key);
        }
        self.group_regions
            .entry(keys.digest)
            .or_default()
            .extend(regions);
        Ok(())
    }

    /// Lower and key `function`, whose callees outside its group are
    /// already lowered.
    pub(crate) fn function(
        &mut self,
        index: usize,
        function: &FunctionInput<'_>,
    ) -> Result<NodeKey, CheckRefusal> {
        let mut parameters = Vec::with_capacity(function.parameters.len());
        for (level, (name, value_type)) in function.parameters.iter().enumerate() {
            let target = function.population_targets.get(level).copied().flatten();
            let type_key = self.binder_type(value_type, target, function.location)?;
            let key = self.parameter(name, level, type_key, function.location)?;
            self.record(type_key, "type", function.location.clone());
            parameters.push(Binder {
                slot: level,
                parameter: key,
                semantic_type: type_key,
            });
        }
        let result = self.type_node(function.result, function.location)?;
        self.record(result, "type", function.location.clone());
        let mut members = vec![SemanticTerm::binding(
            "parameters",
            SemanticTerm::Aggregate {
                members: parameters
                    .iter()
                    .map(|binder| SemanticTerm::reference(binder.parameter))
                    .collect(),
            },
        )];
        let mut binders = Binders {
            slot_names: function.body_slots,
            scope: parameters.clone(),
        };
        let body = self.expression(function.body, &mut binders)?;
        members.push(SemanticTerm::binding("body", body));
        if let Some(measure) = function.measure {
            let mut binders = Binders {
                slot_names: function.measure_slots,
                scope: parameters.clone(),
            };
            let measure = self.expression(measure, &mut binders)?;
            members.push(SemanticTerm::binding("decreases", measure));
        }
        let key = match function.clause {
            // FR-094: a clause function carries its declaration's
            // `ModelOwner`, no `declaration` and a trailing `clause`
            // binding; it has no source `declaration` occurrence.
            Some(clause) => {
                let (owner, clause) = self.clause_owner(clause, function.location)?;
                members.push(clause);
                self.insert_owned(
                    function.location,
                    NodeTag::Function,
                    "pure_function",
                    Some(result),
                    owner,
                    SemanticTerm::Aggregate { members },
                )?
            }
            None => {
                let declaration = qualified_name(function.name, function.location)?;
                let key = self.insert(
                    function.location,
                    NodeTag::Function,
                    "pure_function",
                    Some(result),
                    Some(declaration),
                    SemanticTerm::Aggregate { members },
                )?;
                self.record(key, "declaration", function.location.clone());
                key
            }
        };
        if let Some(slot) = self.functions.get_mut(index) {
            *slot = Some(key);
            self.unsettled_functions.push(index);
        }
        Ok(key)
    }

    // ------------------------------------------------------------------
    // FR-093 expression lowering
    // ------------------------------------------------------------------

    /// Bind `slot` to a new parameter node at the next level, typed at the
    /// type node `semantic_type`; the caller pops it after lowering its
    /// scope.
    fn bind(
        &mut self,
        binders: &mut Binders<'_>,
        slot: Slot,
        semantic_type: NodeKey,
        location: &Location,
    ) -> Result<String, CheckRefusal> {
        let name = binders.slot_names.get(slot).cloned().ok_or_else(|| {
            refuse(
                location,
                CheckCause::NodePreimage(NodeKeyRefusal::EmptyBindingName),
            )
        })?;
        let level = binders.scope.len();
        let parameter = self.parameter(&name, level, semantic_type, location)?;
        binders.scope.push(Binder {
            slot,
            parameter,
            semantic_type,
        });
        Ok(name)
    }

    /// The type node of checked `node`'s value: a local read's is its
    /// binder's, so a `Population<T>[N]` binding keeps its `T`.
    fn value_type_node(
        &mut self,
        node: &Node,
        binders: &Binders<'_>,
    ) -> Result<NodeKey, CheckRefusal> {
        if let NodeKind::Local(slot) = &node.kind {
            if let Some(binder) = binders
                .scope
                .iter()
                .rev()
                .find(|binder| binder.slot == *slot)
            {
                return Ok(binder.semantic_type);
            }
        }
        self.type_node(&node.value_type, &node.location)
    }

    /// Key an application node for checked `node` and return a reference to
    /// it.
    fn application(
        &mut self,
        node: &Node,
        operator: Operator,
        operation: Operation,
        arguments: Vec<SemanticTerm>,
    ) -> Result<SemanticTerm, CheckRefusal> {
        let result_type = self.type_node(&node.value_type, &node.location)?;
        self.typed_application(&node.location, result_type, operator, operation, arguments)
    }

    /// Key an application node typed at `result_type`, denoted by the
    /// region `location`, and return a reference to it.
    fn typed_application(
        &mut self,
        location: &Location,
        result_type: NodeKey,
        operator: Operator,
        operation: Operation,
        arguments: Vec<SemanticTerm>,
    ) -> Result<SemanticTerm, CheckRefusal> {
        let key = self.insert(
            location,
            NodeTag::Expression,
            operator.semantic_form(),
            Some(result_type),
            None,
            SemanticTerm::Application {
                operator,
                operation,
                result_type: super::node_key::NodeRef(result_type),
                arguments,
            },
        )?;
        self.record(key, "expression", location.clone());
        Ok(SemanticTerm::reference(key))
    }

    /// Key a `value` node for checked `node` and return a reference to it.
    fn value_node(
        &mut self,
        node: &Node,
        form: &'static str,
        semantic_type: NodeKey,
        body: SemanticTerm,
    ) -> Result<SemanticTerm, CheckRefusal> {
        let key = self.insert(
            &node.location,
            NodeTag::Value,
            form,
            Some(semantic_type),
            None,
            body,
        )?;
        self.record(key, "expression", node.location.clone());
        Ok(SemanticTerm::reference(key))
    }

    /// The `type_argument` member naming `value_type`'s node.
    fn type_argument(
        &mut self,
        value_type: &ValueType,
        location: &Location,
    ) -> Result<Member, CheckRefusal> {
        Ok(Member::TypeArgument {
            declaration: self.type_node(value_type, location)?,
        })
    }

    /// Lower checked `root` and return the term that names it: a reference
    /// to its node, or to its binder's parameter node for a local read.
    ///
    /// The nodes still to lower and the nodes waiting on their operands are
    /// kept on an explicit heap stack (QSL-228), so a body nested to the
    /// depth limit lowers in the same host stack as a flat one. Nodes are
    /// built, charged and refused in the order the recursive lowering before
    /// QSL-228 reached them -- each node's own type and member nodes before
    /// its operands, its operands in order, then its own node -- so every
    /// key is that lowering's. On a refusal the binders the refused
    /// expression bound are no longer in scope.
    fn expression(
        &mut self,
        root: &Node,
        binders: &mut Binders<'_>,
    ) -> Result<SemanticTerm, CheckRefusal> {
        let scope = binders.scope.len();
        let lowered = self.lower(root, binders);
        if lowered.is_err() {
            binders.scope.truncate(scope);
        }
        lowered
    }

    /// [`Self::expression`]'s loop: lower a node, or hand a lowered node's
    /// term to the frame waiting for it.
    fn lower(
        &mut self,
        root: &Node,
        binders: &mut Binders<'_>,
    ) -> Result<SemanticTerm, CheckRefusal> {
        let mut frames: Vec<LowerFrame<'_>> = Vec::new();
        let mut step = LowerStep::Descend(root, 0);
        loop {
            step = match step {
                LowerStep::Descend(node, depth) => {
                    self.lower_node(node, depth, binders, &mut frames)?
                }
                LowerStep::Lowered(term) => match frames.pop() {
                    None => return Ok(term),
                    Some(frame) => self.accept_term(frame, term, binders, &mut frames)?,
                },
            };
        }
    }

    /// Start lowering checked `node` at `depth`: a literal or local read is
    /// lowered now; a node the FR-093 table builds no node for lowers its
    /// operand in its place; any other node builds its own type and member
    /// nodes, pushes its frame and descends into its first operand.
    #[deny(clippy::wildcard_enum_match_arm)]
    fn lower_node<'n>(
        &mut self,
        node: &'n Node,
        depth: u64,
        binders: &Binders<'_>,
        frames: &mut Vec<LowerFrame<'n>>,
    ) -> Result<LowerStep<'n>, CheckRefusal> {
        self.check_depth(depth, &node.location)?;
        let plain = Operation::plain;
        let (keyed, operands) = match &node.kind {
            NodeKind::Literal(value) => return self.literal(node, value).map(LowerStep::Lowered),
            NodeKind::Local(slot) => {
                let key = binders
                    .scope
                    .iter()
                    .rev()
                    .find(|binder| binder.slot == *slot)
                    .map(|binder| binder.parameter)
                    .ok_or_else(|| {
                        refuse(
                            &node.location,
                            CheckCause::NodePreimage(NodeKeyRefusal::EmptyBindingName),
                        )
                    })?;
                self.record(key, "expression", node.location.clone());
                return Ok(LowerStep::Lowered(SemanticTerm::reference(key)));
            }
            NodeKind::Coerce(operand, interval) => {
                // FR-093: an integer whose type the target range contains is
                // admitted with no node.
                if let ValueType::Int(source) = &operand.value_type {
                    if interval.contains(source.lower()) && interval.contains(source.upper()) {
                        return Ok(LowerStep::Descend(operand, depth + 1));
                    }
                }
                let member =
                    self.type_argument(&ValueType::Int(interval.clone()), &node.location)?;
                (
                    Keyed::Application(
                        Operator::Convert,
                        Operation {
                            member: Some(member),
                            ..plain("quire.op.numeric.narrow")
                        },
                    ),
                    Operands::One(operand),
                )
            }
            NodeKind::Let { slot, value, body } => {
                frames.push(LowerFrame::Let(Box::new(LetFrame {
                    node,
                    depth,
                    slot: *slot,
                    value,
                    body,
                    bound: None,
                })));
                return Ok(LowerStep::Descend(value, depth + 1));
            }
            NodeKind::If {
                condition,
                then,
                otherwise,
            } => (
                Keyed::Application(Operator::Conditional, plain("quire.op.control.if")),
                Operands::Three(condition, then, otherwise),
            ),
            NodeKind::Arithmetic(operator, left, right) => {
                let identity = match operator {
                    Arithmetic::Add => "quire.op.integer.add",
                    Arithmetic::Subtract => "quire.op.integer.sub",
                    Arithmetic::Multiply => "quire.op.integer.mul",
                };
                (
                    Keyed::Application(Operator::Binary, plain(identity)),
                    Operands::Two(left, right),
                )
            }
            NodeKind::Negate(operand) => (
                Keyed::Application(Operator::Unary, plain("quire.op.integer.negate")),
                Operands::One(operand),
            ),
            NodeKind::Divide { left, right, .. } => (
                Keyed::Application(Operator::Binary, plain("quire.op.rational.div")),
                Operands::Two(left, right),
            ),
            NodeKind::Rational {
                operator,
                left,
                right,
                ..
            } => {
                let identity = format!("quire.op.rational.{}", arithmetic_suffix(*operator));
                (
                    Keyed::Application(Operator::Binary, plain(&identity)),
                    Operands::Two(left, right),
                )
            }
            NodeKind::RationalNegate(operand, _) => (
                Keyed::Application(Operator::Unary, plain("quire.op.rational.negate")),
                Operands::One(operand),
            ),
            NodeKind::Decimal {
                operator,
                left,
                right,
                target,
            } => {
                let identity = format!("quire.op.decimal.{}", arithmetic_suffix(*operator));
                (
                    Keyed::Application(
                        Operator::Binary,
                        Operation {
                            mode: Some(OperationMode::Rounding(target.rounding())),
                            ..plain(&identity)
                        },
                    ),
                    Operands::Two(left, right),
                )
            }
            NodeKind::DecimalNegate(operand, _) => (
                Keyed::Application(Operator::Unary, plain("quire.op.decimal.negate")),
                Operands::One(operand),
            ),
            NodeKind::Quantity(operator, left, right) => {
                let identity = format!("quire.op.quantity.{}", arithmetic_suffix(*operator));
                (
                    Keyed::Application(Operator::Binary, plain(&identity)),
                    Operands::Two(left, right),
                )
            }
            NodeKind::Ieee(operator, left, right) => {
                let ValueType::Float(width) = &left.value_type else {
                    return Err(fault(&node.location, KeyFault::UntypedIeeeOperand));
                };
                let width = match width {
                    quire_exact::IeeeWidth::Binary32 => "float32",
                    quire_exact::IeeeWidth::Binary64 => "float64",
                };
                let law = self.law(LawRole::IeeeProfile, &node.location)?;
                let identity = format!("quire.op.ieee.{width}.{}", arithmetic_suffix(*operator));
                (
                    Keyed::Application(
                        Operator::Binary,
                        Operation {
                            laws: vec![law],
                            mode: Some(OperationMode::Rounding(quire_exact::RoundingMode::Exact)),
                            ..plain(&identity)
                        },
                    ),
                    Operands::Two(left, right),
                )
            }
            NodeKind::ConvertDecimal(operand, target) => {
                // FR-149 scale reduction: a decimal of larger maximum scale,
                // or a rational whose denominators exceed 1.
                let reduces = if let ValueType::Decimal(source) = &operand.value_type {
                    source.max_scale() > target.max_scale()
                } else if let ValueType::Rational(domain) = &operand.value_type {
                    *domain.denominator().upper() > Integer::one()
                } else {
                    false
                };
                let member =
                    self.type_argument(&ValueType::Decimal(target.clone()), &node.location)?;
                let operation = if reduces {
                    Operation {
                        member: Some(member),
                        mode: Some(OperationMode::Rounding(target.rounding())),
                        ..plain("quire.op.numeric.convert_rounding")
                    }
                } else {
                    Operation {
                        member: Some(member),
                        ..plain("quire.op.numeric.convert")
                    }
                };
                (
                    Keyed::Application(Operator::Convert, operation),
                    Operands::One(operand),
                )
            }
            NodeKind::Order(operator, kind, left, right) => {
                let family = match kind {
                    OrderedKind::Integers => "integer",
                    OrderedKind::Rationals => "rational",
                    OrderedKind::Decimals => "decimal",
                    OrderedKind::Enums => "enum",
                    OrderedKind::Texts => "text",
                    OrderedKind::Quantities => "quantity",
                };
                let mut operation =
                    plain(&format!("quire.op.{family}.{}", ordering_suffix(*operator)));
                if let OrderedKind::Texts = kind {
                    operation.laws = vec![self.law(LawRole::TextProfile, &node.location)?];
                    operation.mode = text_profile(&left.value_type).map(OperationMode::TextProfile);
                }
                (
                    Keyed::Application(Operator::Binary, operation),
                    Operands::Two(left, right),
                )
            }
            NodeKind::Equality(operator, _, left, right) => {
                let suffix = match operator {
                    EqualityOperator::Equal => "eq",
                    EqualityOperator::NotEqual => "ne",
                };
                let operation = self.equality(&left.value_type, suffix, &node.location)?;
                (
                    Keyed::Application(Operator::Binary, operation),
                    Operands::Two(left, right),
                )
            }
            NodeKind::Connective(connective, left, right) => {
                let identity = match connective {
                    Connective::And => "quire.op.boolean.and",
                    Connective::Or => "quire.op.boolean.or",
                    Connective::Implies => "quire.op.boolean.implies",
                };
                (
                    Keyed::Application(Operator::Binary, plain(identity)),
                    Operands::Two(left, right),
                )
            }
            NodeKind::Not(operand) => (
                Keyed::Application(Operator::Unary, plain("quire.op.boolean.not")),
                Operands::One(operand),
            ),
            NodeKind::Field { operand, index, .. } => {
                let member = self.field_member(&operand.value_type, *index, &node.location)?;
                (
                    Keyed::Application(
                        Operator::Query,
                        Operation {
                            member: Some(member),
                            ..plain("quire.op.record.project")
                        },
                    ),
                    Operands::One(operand),
                )
            }
            NodeKind::Attribute {
                reference, name, ..
            } => {
                // FR-093/FR-094 (QC-24): `quire.op.model.deref` over the
                // reference, typed at its object type `T`'s model node, and
                // over it the `record.project` of field `name` of `T`.
                let object = self.referenced_object(&reference.value_type, &node.location)?;
                frames.push(LowerFrame::Attribute { node, object, name });
                return Ok(LowerStep::Descend(reference, depth + 1));
            }
            NodeKind::Present(operand) => (
                Keyed::Application(Operator::Present, plain("quire.op.option.present")),
                Operands::One(operand),
            ),
            NodeKind::Value(operand) => (
                Keyed::Application(Operator::Value, plain("quire.op.option.value")),
                Operands::One(operand),
            ),
            NodeKind::Call {
                function,
                arguments,
            } => {
                let callee = self
                    .functions
                    .get(*function)
                    .copied()
                    .flatten()
                    .ok_or_else(|| {
                        refuse(
                            &node.location,
                            CheckCause::UnsupportedFeature {
                                loci: vec![node.location.clone()],
                            },
                        )
                    })?;
                let mut terms = Vec::with_capacity(arguments.len() + 1);
                terms.push(SemanticTerm::reference(callee));
                return self.next_operand(
                    Box::new(OperandsFrame {
                        node,
                        depth,
                        operands: Operands::Each(arguments),
                        next: 0,
                        terms,
                        keyed: Keyed::Application(Operator::Call, plain("quire.op.function.call")),
                    }),
                    frames,
                );
            }
            NodeKind::Tuple {
                declaration,
                arguments,
            } => {
                let semantic_type = self.composite(*declaration, &node.location)?;
                (
                    Keyed::Value("tuple_value", semantic_type),
                    Operands::Each(arguments),
                )
            }
            NodeKind::Record { declaration, slots } => {
                let semantic_type = self.composite(*declaration, &node.location)?;
                let Some(CompositeShape::Record(fields)) = self
                    .scope
                    .types()
                    .composite(*declaration)
                    .map(|c| c.shape().clone())
                else {
                    return Err(refuse(
                        &node.location,
                        CheckCause::IllTyped(quire_exact::IllTypedCause::TypeMismatch),
                    ));
                };
                if fields.len() != slots.len() {
                    return Err(fault(&node.location, KeyFault::RecordSlotCount));
                }
                let members = Vec::with_capacity(slots.len());
                return self.record_members(
                    Box::new(RecordFrame {
                        node,
                        depth,
                        semantic_type,
                        fields,
                        slots,
                        members,
                    }),
                    frames,
                );
            }
            NodeKind::Collection {
                collection_type,
                elements,
            } => {
                let identity = format!(
                    "quire.op.collection.{}",
                    collection_form(collection_type.kind())
                );
                let leaves =
                    self.leaves(LeafSource::ResultInner(&node.value_type), &node.location)?;
                (
                    Keyed::Application(
                        Operator::Collection,
                        Operation {
                            leaves,
                            ..plain(&identity)
                        },
                    ),
                    Operands::Each(elements),
                )
            }
            NodeKind::ConvertCollection { target, operand } => {
                let member =
                    self.type_argument(&ValueType::collection(target.clone()), &node.location)?;
                let leaves =
                    self.leaves(LeafSource::ResultInner(&node.value_type), &node.location)?;
                (
                    Keyed::Application(
                        Operator::Convert,
                        Operation {
                            member: Some(member),
                            leaves,
                            ..plain("quire.op.collection.convert")
                        },
                    ),
                    Operands::One(operand),
                )
            }
            NodeKind::ConvertScalar(target, operand) => {
                let target = target.target().unwrap_or(&operand.value_type).clone();
                if target == operand.value_type {
                    // FR-093: a conversion to the operand's own type builds
                    // no node.
                    return Ok(LowerStep::Descend(operand, depth + 1));
                }
                let member = self.type_argument(&target, &node.location)?;
                let operation = if let ValueType::Quantity(_) = &operand.value_type {
                    // FR-093 `quire.op.quantity.convert`: a quantity's
                    // magnitude is an exact rational, and its conversion is
                    // exact (`value::quantity`'s `QuantityTarget::Exact`).
                    Operation {
                        member: Some(member),
                        mode: Some(OperationMode::Rounding(quire_exact::RoundingMode::Exact)),
                        ..plain("quire.op.quantity.convert")
                    }
                } else {
                    Operation {
                        member: Some(member),
                        ..plain("quire.op.numeric.convert")
                    }
                };
                (
                    Keyed::Application(Operator::Convert, operation),
                    Operands::One(operand),
                )
            }
            NodeKind::IeeeToRational(operand, domain) => {
                let law = self.law(LawRole::IeeeProfile, &node.location)?;
                let member =
                    self.type_argument(&ValueType::Rational(domain.clone()), &node.location)?;
                (
                    Keyed::Application(
                        Operator::Convert,
                        Operation {
                            laws: vec![law],
                            member: Some(member),
                            ..plain("quire.op.ieee.to_rational")
                        },
                    ),
                    Operands::One(operand),
                )
            }
            NodeKind::Query {
                visit,
                slot,
                source,
                body,
            } => {
                let (operator, operation) = match visit {
                    Visit::Map => (
                        Operator::Collection,
                        Operation {
                            leaves: self.leaves(
                                LeafSource::ResultInner(&node.value_type),
                                &node.location,
                            )?,
                            ..plain("quire.op.collection.map")
                        },
                    ),
                    Visit::Filter => (Operator::Collection, plain("quire.op.collection.filter")),
                    Visit::Forall => (Operator::Quantify, plain("quire.op.collection.forall")),
                    Visit::Exists => (Operator::Quantify, plain("quire.op.collection.exists")),
                    Visit::Count => (
                        Operator::Collection,
                        Operation {
                            member: Some(self.type_argument(&node.value_type, &node.location)?),
                            ..plain("quire.op.collection.count")
                        },
                    ),
                    Visit::Sum => (
                        Operator::Collection,
                        Operation {
                            member: Some(self.type_argument(&node.value_type, &node.location)?),
                            ..plain("quire.op.collection.sum.integer")
                        },
                    ),
                };
                frames.push(LowerFrame::Binder(Box::new(BinderFrame {
                    node,
                    depth,
                    slot: *slot,
                    source,
                    body,
                    operator,
                    operation,
                    bound: None,
                })));
                return Ok(LowerStep::Descend(source, depth + 1));
            }
            NodeKind::Flatten(operand) => {
                let leaves =
                    self.leaves(LeafSource::ResultInner(&node.value_type), &node.location)?;
                if let NodeKind::Query {
                    visit: Visit::Map,
                    slot,
                    source,
                    body,
                } = &operand.kind
                {
                    // FR-093: `flatMap`, and `flatten(map(..))`, is one
                    // `flat_map` node; no node is built for the inner map.
                    frames.push(LowerFrame::Binder(Box::new(BinderFrame {
                        node,
                        depth,
                        slot: *slot,
                        source,
                        body,
                        operator: Operator::Collection,
                        operation: Operation {
                            leaves,
                            ..plain("quire.op.collection.flat_map")
                        },
                        bound: None,
                    })));
                    return Ok(LowerStep::Descend(source, depth + 1));
                }
                (
                    Keyed::Application(
                        Operator::Collection,
                        Operation {
                            leaves,
                            ..plain("quire.op.collection.flatten")
                        },
                    ),
                    Operands::One(operand),
                )
            }
            NodeKind::Fold {
                accumulator,
                binder,
                source,
                step,
                identity,
            } => {
                frames.push(LowerFrame::Fold(Box::new(FoldFrame {
                    node,
                    depth,
                    accumulator: *accumulator,
                    binder: *binder,
                    source,
                    step,
                    identity: identity.as_deref(),
                    stage: FoldStage::Source,
                })));
                return Ok(LowerStep::Descend(source, depth + 1));
            }
            NodeKind::Size(operand) => {
                let member = self.type_argument(&node.value_type, &node.location)?;
                (
                    Keyed::Application(
                        Operator::Collection,
                        Operation {
                            member: Some(member),
                            ..plain("quire.op.collection.size")
                        },
                    ),
                    Operands::One(operand),
                )
            }
            NodeKind::Contains(collection, item) => {
                let leaves = if let ValueType::Collection(collection_type) = &collection.value_type
                {
                    self.leaves(
                        LeafSource::Compared(collection_type.element()),
                        &node.location,
                    )?
                } else {
                    Vec::new()
                };
                (
                    Keyed::Application(
                        Operator::Collection,
                        Operation {
                            leaves,
                            ..plain("quire.op.collection.contains")
                        },
                    ),
                    Operands::Two(collection, item),
                )
            }
            NodeKind::AllInstances { population } => {
                let object = self.referenced_object(&node.value_type, &node.location)?;
                (
                    Keyed::Application(
                        Operator::Query,
                        Operation {
                            member: Some(Member::TypeArgument {
                                declaration: object,
                            }),
                            ..plain("quire.op.model.all_instances")
                        },
                    ),
                    Operands::One(population),
                )
            }
            NodeKind::Lookup {
                population,
                reference,
                absence,
            } => {
                let object = self.referenced_object(&node.value_type, &node.location)?;
                (
                    Keyed::Application(
                        Operator::Query,
                        Operation {
                            member: Some(Member::TypeArgument {
                                declaration: object,
                            }),
                            mode: Some(OperationMode::Absence(*absence)),
                            ..plain("quire.op.model.lookup")
                        },
                    ),
                    Operands::Two(population, reference),
                )
            }
            NodeKind::Dispatch {
                receiver,
                operation,
                arguments,
                ..
            } => {
                // FR-094: the member names the receiver's static object
                // type's model node, whichever supertype declares it.
                let object = self.referenced_object(&receiver.value_type, &node.location)?;
                let member = self
                    .scope
                    .dispatch_operations
                    .get(*operation)
                    .map(|operation| operation.member.clone())
                    .ok_or_else(|| {
                        refuse(
                            &node.location,
                            CheckCause::IllTyped(quire_exact::IllTypedCause::TypeMismatch),
                        )
                    })?;
                let name = Identifier::new(member).map_err(|_| {
                    refuse(
                        &node.location,
                        CheckCause::NodePreimage(NodeKeyRefusal::EmptyBindingName),
                    )
                })?;
                (
                    Keyed::Application(
                        Operator::Call,
                        Operation {
                            member: Some(Member::Operation {
                                declaration: object,
                                name,
                            }),
                            ..plain("quire.op.model.dispatch_call")
                        },
                    ),
                    Operands::Receiver(receiver, arguments),
                )
            }
            NodeKind::Pre(operand) => (
                Keyed::Application(Operator::Pre, plain("quire.op.state.pre")),
                Operands::One(operand),
            ),
        };
        self.next_operand(
            Box::new(OperandsFrame {
                node,
                depth,
                operands,
                next: 0,
                terms: Vec::with_capacity(operands.len()),
                keyed,
            }),
            frames,
        )
    }

    /// Descend into `frame`'s next operand, or key its node once every
    /// operand is lowered.
    fn next_operand<'n>(
        &mut self,
        mut frame: Box<OperandsFrame<'n>>,
        frames: &mut Vec<LowerFrame<'n>>,
    ) -> Result<LowerStep<'n>, CheckRefusal> {
        if let Some(operand) = frame.operands.get(frame.next) {
            frame.next += 1;
            let depth = frame.depth + 1;
            frames.push(LowerFrame::Operands(frame));
            return Ok(LowerStep::Descend(operand, depth));
        }
        let OperandsFrame {
            node, terms, keyed, ..
        } = *frame;
        match keyed {
            Keyed::Application(operator, operation) => {
                self.application(node, operator, operation, terms)
            }
            Keyed::Value(form, semantic_type) => self.value_node(
                node,
                form,
                semantic_type,
                SemanticTerm::Aggregate { members: terms },
            ),
        }
        .map(LowerStep::Lowered)
    }

    /// Append a record value's members in declaration order up to its next
    /// present field, whose value it descends into, or key the value once
    /// every member is built. An absent or `null` field's member is the
    /// `none` literal of its `option` type.
    fn record_members<'n>(
        &mut self,
        mut frame: Box<RecordFrame<'n>>,
        frames: &mut Vec<LowerFrame<'n>>,
    ) -> Result<LowerStep<'n>, CheckRefusal> {
        let slots = frame.slots;
        while let Some(slot) = slots.get(frame.members.len()) {
            let Some(field) = frame.fields.get(frame.members.len()) else {
                break;
            };
            match slot {
                RecordSlot::Present(value) => {
                    let depth = frame.depth + 1;
                    frames.push(LowerFrame::Record(frame));
                    return Ok(LowerStep::Descend(value, depth));
                }
                RecordSlot::Null | RecordSlot::Absent => {
                    let option = ValueType::option(field.value_type().clone());
                    let option = self.type_node(&option, &frame.node.location)?;
                    let member = SemanticTerm::binding(
                        field.name(),
                        SemanticTerm::literal(option, LiteralValue::None),
                    );
                    frame.members.push(member);
                }
            }
        }
        let RecordFrame {
            node,
            semantic_type,
            members,
            ..
        } = *frame;
        self.value_node(
            node,
            "record_value",
            semantic_type,
            SemanticTerm::Aggregate { members },
        )
        .map(LowerStep::Lowered)
    }

    /// Hand the lowered `term` to `frame`, which descends into its next
    /// operand or keys its own node.
    fn accept_term<'n>(
        &mut self,
        frame: LowerFrame<'n>,
        term: SemanticTerm,
        binders: &mut Binders<'_>,
        frames: &mut Vec<LowerFrame<'n>>,
    ) -> Result<LowerStep<'n>, CheckRefusal> {
        let plain = Operation::plain;
        match frame {
            LowerFrame::Operands(mut frame) => {
                frame.terms.push(term);
                self.next_operand(frame, frames)
            }
            LowerFrame::Let(mut frame) => match frame.bound.take() {
                None => {
                    let value_type = self.value_type_node(frame.value, binders)?;
                    let name = self.bind(binders, frame.slot, value_type, &frame.node.location)?;
                    frame.bound = Some((term, name));
                    let (body, depth) = (frame.body, frame.depth + 1);
                    frames.push(LowerFrame::Let(frame));
                    Ok(LowerStep::Descend(body, depth))
                }
                Some((value, name)) => {
                    binders.scope.pop();
                    self.application(
                        frame.node,
                        Operator::Let,
                        plain("quire.op.control.let"),
                        vec![SemanticTerm::binding(name, value), term],
                    )
                    .map(LowerStep::Lowered)
                }
            },
            LowerFrame::Binder(mut frame) => match frame.bound.take() {
                None => {
                    let location = &frame.node.location;
                    let element = element_type(&frame.source.value_type, location)?;
                    let element = self.type_node(&element, location)?;
                    let name = self.bind(binders, frame.slot, element, location)?;
                    frame.bound = Some((term, name));
                    let (body, depth) = (frame.body, frame.depth + 1);
                    frames.push(LowerFrame::Binder(frame));
                    Ok(LowerStep::Descend(body, depth))
                }
                Some((source, name)) => {
                    binders.scope.pop();
                    let BinderFrame {
                        node,
                        operator,
                        operation,
                        ..
                    } = *frame;
                    self.application(
                        node,
                        operator,
                        operation,
                        vec![source, SemanticTerm::binding(name, term)],
                    )
                    .map(LowerStep::Lowered)
                }
            },
            LowerFrame::Attribute { node, object, name } => {
                let dereferenced = self.typed_application(
                    &node.location,
                    object,
                    Operator::Deref,
                    plain("quire.op.model.deref"),
                    vec![term],
                )?;
                let name = Identifier::new(name).map_err(|_| {
                    refuse(
                        &node.location,
                        CheckCause::NodePreimage(NodeKeyRefusal::EmptyBindingName),
                    )
                })?;
                self.application(
                    node,
                    Operator::Query,
                    Operation {
                        member: Some(Member::Field {
                            declaration: object,
                            name,
                        }),
                        ..plain("quire.op.record.project")
                    },
                    vec![dereferenced],
                )
                .map(LowerStep::Lowered)
            }
            LowerFrame::Record(mut frame) => {
                if let Some(field) = frame.fields.get(frame.members.len()) {
                    let member = SemanticTerm::binding(field.name(), term);
                    frame.members.push(member);
                }
                self.record_members(frame, frames)
            }
            LowerFrame::Fold(frame) => self.accept_fold(frame, term, binders, frames),
        }
    }

    /// Hand a `fold`/`reduce` its lowered source, identity or step.
    fn accept_fold<'n>(
        &mut self,
        mut frame: Box<FoldFrame<'n>>,
        term: SemanticTerm,
        binders: &mut Binders<'_>,
        frames: &mut Vec<LowerFrame<'n>>,
    ) -> Result<LowerStep<'n>, CheckRefusal> {
        let (source, identity) = match std::mem::replace(&mut frame.stage, FoldStage::Source) {
            FoldStage::Source => match frame.identity {
                Some(identity) => {
                    frame.stage = FoldStage::Identity(term);
                    let depth = frame.depth + 1;
                    frames.push(LowerFrame::Fold(frame));
                    return Ok(LowerStep::Descend(identity, depth));
                }
                None => (term, None),
            },
            FoldStage::Identity(source) => (source, Some(term)),
            FoldStage::Step {
                source,
                identity,
                accumulator,
                binder,
            } => {
                binders.scope.pop();
                binders.scope.pop();
                let node = frame.node;
                let member = self.type_argument(&node.value_type, &node.location)?;
                let mut arguments = vec![
                    source,
                    SemanticTerm::binding(accumulator, SemanticTerm::binding(binder, term)),
                ];
                let identity_name = match identity {
                    Some(identity) => {
                        arguments.push(identity);
                        "quire.op.collection.fold"
                    }
                    None => "quire.op.collection.reduce",
                };
                return self
                    .application(
                        node,
                        Operator::Collection,
                        Operation {
                            member: Some(member),
                            ..Operation::plain(identity_name)
                        },
                        arguments,
                    )
                    .map(LowerStep::Lowered);
            }
        };
        // The source and identity are lowered: bind the accumulator and the
        // element, then lower the step under them.
        let node = frame.node;
        let element = element_type(&frame.source.value_type, &node.location)?;
        let element = self.type_node(&element, &node.location)?;
        let accumulator_type = self.type_node(&node.value_type, &node.location)?;
        let accumulator =
            self.bind(binders, frame.accumulator, accumulator_type, &node.location)?;
        let binder = self.bind(binders, frame.binder, element, &node.location)?;
        frame.stage = FoldStage::Step {
            source,
            identity,
            accumulator,
            binder,
        };
        let (step, depth) = (frame.step, frame.depth + 1);
        frames.push(LowerFrame::Fold(frame));
        Ok(LowerStep::Descend(step, depth))
    }

    /// A literal's node: `value`/`literal` for a Boolean, integer or
    /// rational, the enum member's nominal node for an enum member.
    fn literal(&mut self, node: &Node, value: &Value) -> Result<SemanticTerm, CheckRefusal> {
        let literal = match value {
            Value::Boolean(value) => LiteralValue::Boolean(*value),
            Value::Integer(value) => LiteralValue::Integer(value.clone()),
            Value::Rational(value) => LiteralValue::Rational(value.clone()),
            Value::Enum(member) => {
                // FR-093: the enum member's QSpec `enum_value` node, whose
                // key is its `VariantId` (ADR-013 O-14).
                let key = NodeKey::from_digest(*member.variant().as_bytes());
                self.record(key, "expression", node.location.clone());
                return Ok(SemanticTerm::reference(key));
            }
            Value::Decimal(_)
            | Value::Float(_)
            | Value::Quantity(_)
            | Value::Text(_)
            | Value::Option(_)
            | Value::Composite(_)
            | Value::Collection(_)
            | Value::Reference(_)
            | Value::Population(_) => return Err(fault(&node.location, KeyFault::UnbuiltLiteral)),
        };
        let semantic_type = self.type_node(&node.value_type, &node.location)?;
        self.value_node(
            node,
            "literal",
            semantic_type,
            SemanticTerm::literal(semantic_type, literal),
        )
    }

    /// The `record.project` member naming field `index` of `record`.
    fn field_member(
        &mut self,
        record: &ValueType,
        index: usize,
        location: &Location,
    ) -> Result<Member, CheckRefusal> {
        let mismatch = || {
            refuse(
                location,
                CheckCause::IllTyped(quire_exact::IllTypedCause::TypeMismatch),
            )
        };
        let ValueType::Composite(declaration) = record else {
            return Err(mismatch());
        };
        let name = match self
            .scope
            .types()
            .composite(*declaration)
            .map(|c| c.shape())
        {
            Some(CompositeShape::Record(fields)) => fields
                .get(index)
                .map(|field| field.name().to_owned())
                .ok_or_else(mismatch)?,
            _ => return Err(mismatch()),
        };
        let name = Identifier::new(name).map_err(|_| {
            refuse(
                location,
                CheckCause::NodePreimage(NodeKeyRefusal::EmptyBindingName),
            )
        })?;
        Ok(Member::Field {
            declaration: self.composite(*declaration, location)?,
            name,
        })
    }

    /// The equality operation for operands of `compared` type (FR-093
    /// `Equality` row).
    fn equality(
        &mut self,
        compared: &ValueType,
        suffix: &str,
        location: &Location,
    ) -> Result<Operation, CheckRefusal> {
        let family = match compared {
            ValueType::Boolean => "boolean",
            ValueType::Integer | ValueType::Int(_) => "integer",
            ValueType::Rational(_) => "rational",
            ValueType::Decimal(_) => "decimal",
            ValueType::Text(text) => {
                let law = self.law(LawRole::TextProfile, location)?;
                return Ok(Operation {
                    laws: vec![law],
                    mode: Some(OperationMode::TextProfile(text.profile())),
                    ..Operation::plain(&format!("quire.op.text.{suffix}"))
                });
            }
            ValueType::Enum(_) => "enum",
            ValueType::Quantity(_) => "quantity",
            ValueType::Reference(_) => "reference",
            ValueType::Option(_) | ValueType::Composite(_) | ValueType::Collection(_) => {
                let leaves = self.leaves(LeafSource::Compared(compared), location)?;
                return Ok(Operation {
                    leaves,
                    ..Operation::plain(&format!("quire.op.structural.{suffix}"))
                });
            }
            // FR-322: IEEE values have no `eq`/`ne`; the checker admits
            // none, so a float operand here is ill-typed.
            ValueType::Float(_) => {
                return Err(refuse(
                    location,
                    CheckCause::IllTyped(quire_exact::IllTypedCause::OperatorIneligible),
                ))
            }
            // FR-153: a population is never an equality operand; the
            // checker refuses one before lowering.
            ValueType::Population(_) => {
                return Err(refuse(
                    location,
                    CheckCause::IllTyped(quire_exact::IllTypedCause::OperatorIneligible),
                ))
            }
        };
        Ok(Operation::plain(&format!("quire.op.{family}.{suffix}")))
    }
}

/// The text profile of a text type.
fn text_profile(value_type: &ValueType) -> Option<TextProfile> {
    match value_type {
        ValueType::Text(text) => Some(text.profile()),
        _ => None,
    }
}

/// The element type of a collection type.
fn element_type(value_type: &ValueType, location: &Location) -> Result<ValueType, CheckRefusal> {
    match value_type {
        ValueType::Collection(collection) => Ok(collection.element().clone()),
        _ => Err(refuse(
            location,
            CheckCause::IllTyped(quire_exact::IllTypedCause::TypeMismatch),
        )),
    }
}

/// The order to lower a package's functions in (FR-092 "Recursion
/// groups"): each group of functions that call each other, every callee's
/// group before its callers' (a call node's key hashes its callee's key,
/// FR-093).
pub(crate) fn lowering_order(callees: &[Vec<usize>]) -> Vec<FunctionGroup> {
    let count = callees.len();
    let edges: Vec<Vec<usize>> = callees
        .iter()
        .map(|callees| {
            callees
                .iter()
                .copied()
                .filter(|callee| *callee < count)
                .collect()
        })
        .collect();
    strongly_connected(&edges)
        .into_iter()
        .map(|mut members| {
            members.sort_unstable();
            let recursive = members.len() > 1
                || members
                    .first()
                    .is_some_and(|member| edges[*member].contains(member));
            FunctionGroup { members, recursive }
        })
        .collect()
}

/// The strongly connected components of the graph `edges` (Tarjan's
/// algorithm, with an explicit stack), each component after every
/// component it has an edge to.
fn strongly_connected(edges: &[Vec<usize>]) -> Vec<Vec<usize>> {
    const UNVISITED: usize = usize::MAX;
    let count = edges.len();
    let mut index = vec![UNVISITED; count];
    let mut low = vec![0; count];
    let mut on_stack = vec![false; count];
    let mut stack = Vec::new();
    let mut next = 0;
    let mut components = Vec::new();
    for root in 0..count {
        if index[root] != UNVISITED {
            continue;
        }
        let mut work = vec![(root, 0_usize)];
        while let Some((node, edge)) = work.pop() {
            if edge == 0 && index[node] == UNVISITED {
                index[node] = next;
                low[node] = next;
                next += 1;
                stack.push(node);
                on_stack[node] = true;
            }
            if let Some(&target) = edges[node].get(edge) {
                work.push((node, edge + 1));
                if index[target] == UNVISITED {
                    work.push((target, 0));
                } else if on_stack[target] {
                    low[node] = low[node].min(index[target]);
                }
                continue;
            }
            if low[node] == index[node] {
                let mut component = Vec::new();
                while let Some(member) = stack.pop() {
                    on_stack[member] = false;
                    component.push(member);
                    if member == node {
                        break;
                    }
                }
                components.push(component);
            }
            if let Some((parent, _)) = work.last() {
                low[*parent] = low[*parent].min(low[node]);
            }
        }
    }
    components
}

/// The package root location a `generated` occurrence names.
pub(crate) fn generated_location() -> Location {
    Location {
        origin: Origin::Expression,
        path: Vec::new(),
    }
}

#[cfg(test)]
mod tests;
