// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-092 and FR-093: the checked semantic graph of a package's
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
//! Every node is identified by its admitted structural `NodeKey`; lowering never
//! hashes content to identify a node. Recursive functions and records form
//! recursion groups (FR-092 "Recursion groups"). The text law is
//! QSL's own text profile; the protocol law is the profile the unit's header
//! selected, and a protocol node with none refuses
//! (`missing_declaration`/`missing-selection`).
//!
//! FR-094 keys the rest (`model`): the model declaration nodes a
//! `Reference<T>` or a model row's member names, the `Reference<T>` and
//! `Population<T>[N]` type nodes, a clause function's `ModelOwner` and
//! `clause` binding, and quantity type nodes. `check` records each model
//! declaration node it keys in the model correspondence.

use std::collections::{btree_map, BTreeMap, BTreeSet, HashMap, HashSet};
use std::ops::ControlFlow;

use quire_exact::{
    ArithmeticOperator, Charge, ChargePoint, CollectionKind, CollectionType, EffectiveId,
    Identifier, Integer, Meter, NodeKey, OrderingOperator, Origin as OccurrenceOrigin, PathSegment,
    Presence, TextProfile, Value, ValueType,
};

use super::check::{EnumBinding, Scope};
use super::claims::BinderSite;
use super::family::{OccurrenceMap, OccurrenceRole};
use super::identity::ModelCorrespondence;
use super::ir::{
    Arithmetic, CaseArm, CheckedLiteral, CheckedNode, Connective, IntegerDivisionOperator, NodeId,
    NodeKind, OrderedKind, RecordField, RecordSlot, Slot, Visit,
};
use super::node_key::{
    AggregateTerm, Binding, BindingValue, BodyTerm, GroupMember, GroupTerm, LeafTerm, MemberTerm,
    TupleMember, TupleTerm,
};
use super::node_key::{
    LawRole, LeafSegment, LiteralValue, NodeKeyRefusal, NodeTag, Operation, OperationLaw,
    OperationLeaf, OperationMode, Operator, Owner, SourceOwner,
};
use super::node_key::{NodeRef, PlaceholderId};
use super::refusal::{
    CheckCause, CheckRefusal, CheckingLimitKind, CheckingStage, KeyFault, StageLimitCause,
};
use crate::value::definition::{DefinitionReference, DefinitionRole};
use crate::value::member::Member;
use crate::value::nominal_payload::{EnumPayload, NominalPayload};
use quire_semantic_value::declaration::{
    CompositeShape, EqualityOperator, FieldDeclaration, TypeEnvironment, UnionMemberDeclaration,
};
use quire_semantic_value::enumeration::EnumValue;
use quire_semantic_value::location::{Location, Origin};
use quire_semantic_value::quantity::UnitTable;

mod abstraction;
mod hybrid;
mod hyper;
mod model;
mod protocol;
mod refinement_rows;
mod state;
mod state_graph;
mod state_transition;
mod temporal;
mod terminal;
mod timed_clause;

pub(crate) use hybrid::{
    hybrid_location, hybrid_site, HybridClaimInput, HybridModelInput, HybridSite,
};
pub(crate) use hyper::{
    finish_step_relation, HyperClauseInput, LoweredStepRelation, StepRelationInput,
};
pub(crate) use model::ParentSlotFailure;
pub(crate) use model::{
    bound_origin, equivalence_origin, model_declaration_key, quantity_origin, record_node_key,
    strictly_below, QuantityKind,
};
pub use model::{
    collection_kind, model_root_key, AdmittedModel, AuthoredBound, CollectionShape, ForeignView,
    ModelClause, PlaceRow, PlaceStep,
};
pub(crate) use model::{model_node_content, population_node_content};
pub(crate) use protocol::{
    compensation_origin, declaration_origin, protocol_location, protocol_site, AttemptNodes,
    CompensationNodes, ProtocolInput, ProtocolSite,
};
pub(crate) use state::{
    protocol_attempt_operation_location, protocol_attempt_operation_site,
    state_clause_name_location, state_clause_name_site, state_operation_location,
    state_operation_site, AnchorInput, AttemptInput, LoweredAttempt, LoweredClause,
    OperationBinding, StateClauseInput,
};
pub(crate) use state_graph::StateGraphClaimInput;
pub(crate) use state_transition::TransitionClause;
pub(crate) use temporal::{
    temporal_location, temporal_site, LoweredTemporalClause, TemporalClauseInput, TemporalSite,
};
pub(crate) use terminal::{terminal_location, terminal_site, TerminalInput, TerminalSite};
pub(crate) use timed_clause::TimedClauseInput;

/// A node's place in its recursion group (FR-092 "Recursion groups"): the
/// node's ordinal and the number of the group's nodes.
/// FR-093's emission writes a group's nodes in ordinal order.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NodeRecursion {
    ordinal: usize,
    size: usize,
    members: std::sync::Arc<[NodeKey]>,
}

impl NodeRecursion {
    /// The node's ordinal: its rank in the group order.
    pub fn ordinal(&self) -> usize {
        self.ordinal
    }

    /// The number of the group's nodes.
    pub fn size(&self) -> usize {
        self.size
    }

    /// The group's members as admitted keys, in group (ordinal) order. The
    /// node's own key is `members()[ordinal()]`.
    pub fn members(&self) -> &[NodeKey] {
        &self.members
    }

    /// The group's anchor: its first member's key. Producer-assigned and
    /// structural; every member of one group reports the same anchor, so it
    /// is the shared `recursion_group` label when rendered as text.
    pub fn anchor(&self) -> &NodeKey {
        &self.members[0]
    }
}

/// One lowered, keyed node of the checked semantic graph.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticNode {
    key: NodeKey,
    recursion: Option<NodeRecursion>,
    nominal: Option<NominalPayload>,
    /// Outside every recursion group, the content [`Self::key`] keys; a
    /// group member's names its group's members by their keys.
    content: NodeContent,
}

impl SemanticNode {
    /// The `correspondence`/`abstraction_relation` node of `term` under its
    /// admitted structural `key` (QSpec FR-451).
    pub(super) fn abstraction_relation(
        key: NodeKey,
        term: super::node_key::AbstractionTerm,
    ) -> Self {
        Self::declared(
            key,
            NodeContent {
                owner_model: None,
                node_tag: NodeTag::Correspondence,
                semantic_form: "abstraction_relation",
                semantic_type: None,
                declaration: None,
                owner: None,
                body: BodyTerm::Abstraction(term),
            },
        )
    }

    /// A declared node under its admitted structural `key`, outside every
    /// recursion group.
    pub(super) fn declared(key: NodeKey, content: NodeContent) -> Self {
        Self {
            key,
            recursion: None,
            nominal: None,
            content,
        }
    }

    /// The node's `quire.checked-semantic-node/v1` key.
    pub fn key(&self) -> NodeKey {
        self.key.clone()
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
        self.content
            .semantic_type
            .as_ref()
            .and_then(NodeRef::admitted)
            .cloned()
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

    /// QSpec FR-464's `owner_model`: the exact model declaration node a
    /// `protocol_clause` node is declared over; `None` for every other node.
    pub fn owner_model(&self) -> Option<NodeKey> {
        self.content.owner_model.clone()
    }

    /// The facts a nominal declaration node carries beyond its key (an
    /// enum's ordering and cases, a dimension's terms, a unit's dimension,
    /// target and conversion); `None` for every other node.
    pub fn nominal_payload(&self) -> Option<&NominalPayload> {
        self.nominal.as_ref()
    }

    /// The node's recursion group, when it is in one.
    pub fn recursion(&self) -> Option<&NodeRecursion> {
        self.recursion.as_ref()
    }

    /// The node's FR-322 body. A reference to a member of the node's own
    /// recursion group names that member's key.
    pub fn body(&self) -> &BodyTerm {
        &self.content.body
    }

    /// FR-092 "Function nodes": a function node's parameter nodes, in
    /// declared parameter order, read from its `parameters` binding. `None`
    /// for every node that is not a function node. The replay executor
    /// joins arguments keyed by parameter node id to parameter positions
    /// through this (ADR-013 O-25, C-11).
    pub fn function_parameters(&self) -> Option<Vec<NodeKey>> {
        if self.content.node_tag != NodeTag::Function {
            return None;
        }
        let BodyTerm::Aggregate(AggregateTerm { members }) = &self.content.body else {
            return None;
        };
        // FR-092 "Function nodes": the parameters binding is first among a
        // function node's members (`lowering`'s own emitter always pushes it
        // before `body`/`decreases`), so this reads it positionally rather
        // than re-deriving the role by comparing the binding's name.
        let MemberTerm::Binding(Binding {
            value: BindingValue::Group(parameters),
            ..
        }) = members.first()?
        else {
            return None;
        };
        parameters
            .members
            .iter()
            .map(|member| match member {
                GroupMember::Leaf(LeafTerm::Reference { target }) => target.admitted().cloned(),
                GroupMember::Leaf(_) | GroupMember::Binding(_) => None,
            })
            .collect()
    }
}

/// The name of the binding a function node's body holds its parameter
/// nodes under, first among its members (FR-092 "Function nodes").
const FUNCTION_PARAMETERS: &str = "parameters";

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

    /// Add a node finalized after the main lowering pass, provided a node
    /// with the same content-addressed identity is absent or identical.
    pub(crate) fn insert_finalized(&mut self, node: SemanticNode) -> Result<(), ()> {
        match self.nodes.entry(node.key.clone()) {
            btree_map::Entry::Vacant(slot) => {
                slot.insert(node);
                Ok(())
            }
            btree_map::Entry::Occupied(slot) if slot.get().content == node.content => Ok(()),
            btree_map::Entry::Occupied(_) => Err(()),
        }
    }

    /// Take out the node keyed `key`, as a re-key leaves the key it had.
    #[cfg(any(test, feature = "test-support"))]
    pub fn remove(&mut self, key: NodeKey) -> Option<SemanticNode> {
        self.nodes.remove(&key)
    }

    /// Add an enclosing narrow node for a claims-keying fixture. The cloned
    /// checked narrow is changed to reference `operand`, so the fixture can
    /// distinguish it from the direct conversion without supplying a
    /// test-authored operation predicate.
    #[cfg(test)]
    pub(crate) fn with_test_narrow_over(&self, key: NodeKey, operand: NodeKey) -> Self {
        let mut graph = self.clone();
        let mut enclosing = self
            .nodes
            .values()
            .find(|node| {
                matches!(
                    &node.content.body,
                    super::node_key::BodyTerm::Application(application)
                        if application.operation.identity() == "quire.op.numeric.narrow"
                )
            })
            .expect("the checked fixture contains a narrow node")
            .clone();
        enclosing.key = key.clone();
        let super::node_key::BodyTerm::Application(application) = &mut enclosing.content.body
        else {
            unreachable!("the selected template is a narrow application")
        };
        application.arguments = vec![super::node_key::MemberTerm::Leaf(
            super::node_key::LeafTerm::Reference {
                target: operand.into(),
            },
        )];
        graph.nodes.insert(key, enclosing);
        graph
    }
}

/// One checked function, as the lowering reads it.
pub(crate) struct FunctionInput<'a> {
    /// The declared name, `::`-separated.
    pub(crate) name: &'a str,
    /// The declaration kind: a `Predicate`'s node `semantic_form` is
    /// `predicate` rather than `pure_function` (FR-092 "Function nodes"). A
    /// recursion group's members are `recursive_function` whatever their
    /// kind.
    pub(crate) kind: qsl_forms::DeclarationKind,
    /// The declaration's region.
    pub(crate) location: &'a Location,
    /// The parameters in order.
    pub(crate) parameters: &'a [(String, ValueType)],
    /// Source parameter declaration spans, separate from the body location.
    pub(crate) parameter_spans: Option<&'a [qsl_foundation::Span]>,
    /// The declared result type.
    pub(crate) result: &'a ValueType,
    /// The checked body.
    pub(crate) body: CheckedNode<'a>,
    /// The name each body slot was bound under, indexed by slot.
    pub(crate) body_slots: &'a [String],
    /// The checked measure, when one is written.
    pub(crate) measure: Option<CheckedNode<'a>>,
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
#[derive(Clone)]
struct Binder {
    slot: Slot,
    parameter: NodeRef,
    semantic_type: NodeRef,
}

/// The binders in scope at one point of a function body.
struct Binders<'s> {
    /// The name each slot was bound under.
    slot_names: &'s [String],
    /// Each binder in scope, outermost first.
    scope: Vec<Binder>,
    /// Each binder in scope by its slot. A slot is bound once at a time, so
    /// a local read finds its binder in constant time at any nesting.
    by_slot: HashMap<Slot, Binder>,
}

impl<'s> Binders<'s> {
    /// The scope that holds `parameters`.
    fn new(slot_names: &'s [String], parameters: &[Binder]) -> Self {
        let mut binders = Self {
            slot_names,
            scope: Vec::new(),
            by_slot: HashMap::new(),
        };
        for parameter in parameters {
            binders.push(parameter.clone());
        }
        binders
    }

    /// Bring `binder` into scope.
    fn push(&mut self, binder: Binder) {
        self.by_slot.insert(binder.slot, binder.clone());
        self.scope.push(binder);
    }

    /// Take the innermost binder out of scope.
    fn pop(&mut self) {
        if let Some(binder) = self.scope.pop() {
            self.by_slot.remove(&binder.slot);
        }
    }

    /// Take every binder past the first `len` out of scope.
    fn truncate(&mut self, len: usize) {
        while self.scope.len() > len {
            self.pop();
        }
    }

    /// The binder in scope for `slot`.
    fn find(&self, slot: Slot) -> Option<&Binder> {
        self.by_slot.get(&slot)
    }
}

/// Every member of a node's preimage outside every recursion group: what
/// [`node_key`] keys. [`node_key`] is a function of these members alone, so
/// equal content has one key, which [`Lowering::insert_node`] computes once.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) struct NodeContent {
    pub(crate) node_tag: NodeTag,
    pub(crate) semantic_form: &'static str,
    pub(crate) semantic_type: Option<NodeRef>,
    pub(crate) declaration: Option<Vec<Identifier>>,
    pub(crate) owner: Option<Owner>,
    /// QSpec FR-464's `owner_model`: set for a `protocol_clause` node alone.
    pub(crate) owner_model: Option<NodeKey>,
    pub(crate) body: BodyTerm,
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

/// What lowering yields: the graph, its model correspondence and each
/// function's node key by index.
pub(crate) struct Lowered {
    pub(crate) graph: SemanticGraph,
    pub(crate) correspondence: ModelCorrespondence,
    pub(crate) functions: Vec<Option<NodeKey>>,
    /// Each binder's parameter node, by the binding node's location and
    /// the slot it binds: the roots a claim's extent is keyed by.
    pub(crate) binders: BTreeMap<BinderSite, NodeKey>,
    pub(crate) parameter_spans: BTreeMap<(NodeKey, OccurrenceOrigin), qsl_foundation::Span>,
    /// Each `bounded_domain`/`model_population` node lowering built (FR-094,
    /// FR-380), whose `Type` occurrences take their ordinals in ascending
    /// source order.
    pub(crate) population_nodes: BTreeSet<NodeKey>,
    /// Each declared composite's node key, by its declaration key: the final
    /// key a union is sealed under (FR-319).
    pub(crate) composites: BTreeMap<NodeKey, NodeKey>,
    /// The S4 identities of each protocol's clause and emitted members,
    /// retained for FR-390's checked-body selector.
    pub(crate) protocol_members: BTreeMap<String, protocol::ProtocolMembers>,
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
    /// The admitted package name every structural node key is minted under.
    package_name: String,
    /// The admitted node of each distinct content: equal content is one node.
    node_index: HashMap<NodeContent, NodeKey>,
    /// The next unused ordinal of each structural path, keyed by the
    /// path's ordinal-0 identity.
    ordinals: BTreeMap<NodeKey, u64>,
    /// The checked unit's own source: the owner of every node it declares.
    owner: SourceOwner,
    models: &'a [AdmittedModel],
    /// The package's units and every compound unit the check stage formed.
    units: UnitTable,
    /// Each model declaration node keyed, one-to-one with its
    /// `DeclarationKey` (FR-303).
    correspondence: ModelCorrespondence,
    /// The `protocol_profile` the unit's header selected (FR-110).
    protocol_profile: Option<&'a DefinitionReference>,
    graph: SemanticGraph,
    occurrences: &'a mut OccurrenceMap<Location>,
    /// Each lowered function's node, by function index.
    functions: Vec<Option<NodeRef>>,
    /// Each declared composite's node, by its declaration key.
    composites: BTreeMap<NodeKey, NodeRef>,
    /// The model record value type or event type each model-owned composite
    /// declaration key stands for (FR-359 item 9), built on first use.
    /// The `functions` slots and `composites` entries written since the
    /// last settle, the only keys that settle can still resolve
    /// (a settle per function group resolves its own keys, not every key
    /// the package has so far).
    unsettled_functions: Vec<usize>,
    unsettled_composites: Vec<NodeKey>,
    /// Composites whose node is being built.
    composites_in_progress: Vec<NodeKey>,
    /// The placeholder naming each composite in progress that reached
    /// itself.
    composite_placeholders: BTreeMap<NodeKey, NodeRef>,
    /// Whether a recursive function group is being lowered.
    functions_open: bool,
    /// The `clock_place` template node of each clock place of the unit's
    /// timed models, which a `presence_read` names (QSpec FR-322
    /// "Presence").
    timed_places: Vec<timed_clause::TimedPlace>,
    /// Each placeholder, and the draft it became once built.
    placeholders: BTreeMap<PlaceholderId, Option<NodeRef>>,
    /// How many placeholders this lowering has made.
    placeholder_count: u64,
    /// The drafts, by their local handle.
    drafts: BTreeMap<PlaceholderId, Draft>,
    /// Occurrences of drafts, recorded once the drafts are keyed.
    draft_occurrences: Vec<(NodeRef, OccurrenceRole, Location)>,
    /// Each binder's parameter node, by its [`BinderSite`].
    binders: BTreeMap<BinderSite, NodeKey>,
    /// Binders whose parameter node is a draft, recorded once it is keyed.
    draft_binders: Vec<(BinderSite, NodeRef)>,
    /// Authored function parameters, located by the same binder site whose
    /// node is resolved when a recursion group settles.
    parameter_spans: BTreeMap<BinderSite, qsl_foundation::Span>,
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
    /// FR-104: the `Origin` recording one occurrence of a named operation's
    /// `frame` node, by the operation's own identity (its declaring type
    /// and name), not by the frame node's key. Two operations whose frame
    /// content happens to coincide (equal `modifies`/`creates`/`deletes`)
    /// still key one node, but this map gives each its own occurrence, so
    /// each keeps its own `operation-contract` record (FR-104-AC-5). The
    /// first clause naming an operation, in source order, mints the
    /// occurrence; every later clause naming the same operation shares it.
    frame_origins: BTreeMap<(EffectiveId, String), OccurrenceOrigin>,
    /// Each `model_population` node built so far (FR-094, FR-380).
    population_nodes: BTreeSet<NodeKey>,
    /// The (`bound` label, population node) pairs whose one `Type`
    /// occurrence of the bound's `[N]` is recorded (FR-380).
    bound_sites: BTreeSet<(String, NodeKey)>,
    /// What lowering holds for each checked protocol declaration, by its
    /// index in the unit's protocols (FR-092).
    protocol_keys: Vec<protocol::ProtocolKey>,
    /// The member keys of each keyed protocol clause, by clause name: the
    /// nodes a refinement class catalog names (FR-375).
    protocol_members: BTreeMap<String, protocol::ProtocolMembers>,
    /// Attempts and compensation nodes retained for clauses delayed until
    /// their required temporal clause keys are available.
    protocol_attempts: Vec<Vec<protocol::AttemptNodes>>,
    protocol_compensations: Vec<Vec<protocol::CompensationNodes>>,
    /// Temporal clause nodes keyed before a protocol that requires them.
    temporal_keys: BTreeMap<usize, NodeKey>,
    /// Temporal clauses whose lowering refused: the refusal is reported at
    /// the clause, so a protocol requiring one is left unkeyed without a
    /// second refusal of its own.
    refused_temporals: BTreeSet<usize>,
    /// Parameter nodes minted while lowering one protocol's expression
    /// bodies, by enclosing protocol declaration.
    protocol_binder_keys:
        BTreeMap<String, Vec<crate::check::protocol_clause::CheckedProtocolBinderKey>>,
    /// Exact source-parameter mapping for each protocol expression body.
    protocol_expression_binders:
        BTreeMap<String, Vec<crate::check::protocol_clause::CheckedProtocolExpressionBindings>>,
    /// The protocol whose S4 bodies are being lowered, when any.
    active_protocol: Option<String>,
}

/// The admitted key of `node`: a node that names no pending handle is
/// admitted when built, so a pending one here is an internal fault.
pub(crate) fn require_admitted(
    node: NodeRef,
    location: &Location,
) -> Result<NodeKey, CheckRefusal> {
    match node {
        NodeRef::Admitted(key) => Ok(key),
        NodeRef::Pending(_) => Err(fault(location, KeyFault::UnresolvedDraft)),
    }
}

/// The declared structural key of a step-relation node, which is built
/// outside a [`Lowering`]: its one declared name under the package, at
/// ordinal 0 (a unit declares one step relation per name).
pub(crate) fn admitted_step_relation_key(
    package_name: &str,
    content: &NodeContent,
    location: &Location,
) -> Result<NodeKey, CheckRefusal> {
    let names = content
        .declaration
        .as_ref()
        .ok_or_else(|| fault(location, KeyFault::InvalidGroup))?;
    super::declared_node_key(
        package_name.to_owned(),
        names.iter().map(|name| name.as_str().to_owned()).collect(),
        0,
    )
    .map_err(|refusal| fault(location, KeyFault::ModelRootKey(refusal.to_string())))
}

fn refuse(location: &Location, cause: CheckCause) -> CheckRefusal {
    CheckRefusal {
        location: location.clone(),
        cause,
    }
}

/// A broken invariant of the union lowering, never a refusal of the input.
fn union_fault(location: &Location, invariant: &'static str) -> CheckRefusal {
    fault(
        location,
        KeyFault::StageFault(quire_semantic_value::catalog::InternalFault::new(
            "S3", invariant,
        )),
    )
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
            // The meter's own report: the denied charge carries work units
            // only, so the refused charge would have taken the cumulative
            // spend to what was consumed plus this one charge (`work`),
            // matching `ValueFunctionFamily::check`'s own `WorkBudget` charge.
            actual: u128::from(incomplete.consumed) + u128::from(work),
        })
}

fn preimage_refusal(location: &Location, refusal: NodeKeyRefusal) -> CheckRefusal {
    match refusal {
        NodeKeyRefusal::WorkBudget { limit, actual } => refuse(
            location,
            CheckCause::ResourceExhausted(Box::new(StageLimitCause {
                stage: CheckingStage::Typing,
                kind: CheckingLimitKind::WorkBudget,
                limit,
                actual,
                region: None,
            })),
        ),
        NodeKeyRefusal::InvalidGroup => fault(location, KeyFault::InvalidGroup),
        NodeKeyRefusal::Identity(identity) => refuse(location, CheckCause::Identity(identity)),
        refusal => refuse(location, CheckCause::NodePreimage(refusal)),
    }
}

/// One step of [`Lowering::type_node`]'s loop.
enum TypeStep<'v> {
    /// Build the node of this type.
    Descend(&'v ValueType),
    /// This node is built: hand its key to the frame waiting for it.
    Built(NodeRef),
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
    /// Where its `declaration` occurrence is located.
    site: Location,
    shape: &'v CompositeShape,
    /// The members built so far, in declaration order.
    members: Vec<MemberTerm>,
    /// The record field whose type node is being built.
    awaiting: Option<&'v FieldDeclaration>,
    /// The payload type nodes built so far of the union member being built,
    /// in position order.
    payload: Vec<NodeRef>,
}

impl CompositeFrame<'_> {
    /// Append the member whose type node is `key`: a record field's
    /// binding (an optional one's through `optional`), a tuple position's
    /// reference, or the next payload type of the union member being built.
    fn accept(&mut self, key: NodeRef) {
        if matches!(self.shape, CompositeShape::Union(_)) {
            self.payload.push(key);
            return;
        }
        let reference = LeafTerm::reference_node(key);
        self.members.push(match self.awaiting.take() {
            None => MemberTerm::Leaf(reference),
            Some(field) => MemberTerm::binding(
                field.name(),
                match field.presence() {
                    Presence::Required => BindingValue::Leaf(reference),
                    Presence::Optional => BindingValue::named_leaf("optional", reference),
                },
            ),
        });
    }
}

/// How a declared composite's node starts.
enum OpenComposite<'v> {
    /// Its node, or the placeholder naming it while it is being built.
    Built(NodeRef),
    /// Its frame: its members are still to build.
    Open(CompositeFrame<'v>),
}

/// One step of [`Lowering::expression`]'s loop.
enum LowerStep<'n> {
    /// Lower this checked node.
    Descend(CheckedNode<'n>),
    /// A node is lowered to this term: hand it to the frame waiting for it.
    Lowered(LeafTerm),
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
        node: CheckedNode<'n>,
        object: NodeKey,
        name: &'n str,
    },
    /// A record value: its present fields' values, in declaration order.
    Record(Box<RecordFrame<'n>>),
    /// `fold`/`reduce`: its source, its identity, then its step under the
    /// accumulator and element binders.
    Fold(Box<FoldFrame<'n>>),
    /// `case`: its scrutinee, then each arm's body under the arm's binders.
    Case(Box<CaseFrame<'n>>),
}

/// A checked node's operands, in order.
#[derive(Clone, Copy)]
enum Operands<'n> {
    One(&'n NodeId),
    Two(&'n NodeId, &'n NodeId),
    Three(&'n NodeId, &'n NodeId, &'n NodeId),
    Each(&'n [NodeId]),
    /// A dispatch call's receiver, then its arguments.
    Receiver(&'n NodeId, &'n [NodeId]),
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

    fn get(self, index: usize) -> Option<NodeId> {
        let operand = match (self, index) {
            (Self::One(first) | Self::Two(first, _) | Self::Three(first, _, _), 0)
            | (Self::Receiver(first, _), 0) => Some(first),
            (Self::Two(_, second) | Self::Three(_, second, _), 1) => Some(second),
            (Self::Three(_, _, third), 2) => Some(third),
            (Self::Each(nodes), index) => nodes.get(index),
            (Self::Receiver(_, arguments), index) => arguments.get(index.checked_sub(1)?),
            (Self::One(_) | Self::Two(..) | Self::Three(..), _) => None,
        };
        operand.copied()
    }
}

/// What an operands frame keys once its operands are lowered.
enum Keyed {
    /// An application node.
    Application(Operator, Operation),
    /// A `value` node of this form, typed at this type node, whose body
    /// aggregates the operands.
    Value(&'static str, NodeRef),
    /// A union construction (QSpec FR-440 "Union value node"): the
    /// `union_value` node typed at the union's type node, whose body is one
    /// binding of the active member's name over the payload terms.
    UnionValue {
        semantic_type: NodeRef,
        member: String,
    },
    /// A present option's payload binding, typed at its option node.
    OptionValue(NodeRef),
}

/// A node over its operands' terms.
struct OperandsFrame<'n> {
    node: CheckedNode<'n>,
    operands: Operands<'n>,
    /// The next operand to lower.
    next: usize,
    /// The terms so far: a call's callee reference, then each operand's.
    terms: Vec<LeafTerm>,
    keyed: Keyed,
}

/// `let name = value in body`.
struct LetFrame<'n> {
    node: CheckedNode<'n>,
    slot: Slot,
    value: CheckedNode<'n>,
    body: CheckedNode<'n>,
    /// The value's term and the binder's name, once the value is lowered.
    bound: Option<(LeafTerm, String)>,
}

/// A one-binder operation's operands `[ref(source), binding{name,
/// ref(body)}]`, the binder typed at `source`'s element type.
struct BinderFrame<'n> {
    node: CheckedNode<'n>,
    slot: Slot,
    source: CheckedNode<'n>,
    body: CheckedNode<'n>,
    operator: Operator,
    operation: Operation,
    /// The source's term and the binder's name, once the source is lowered.
    bound: Option<(LeafTerm, String)>,
}

/// A record value's members, one binding per declared field.
struct RecordFrame<'n> {
    node: CheckedNode<'n>,
    semantic_type: NodeRef,
    fields: Vec<FieldDeclaration>,
    slots: &'n [RecordSlot],
    members: Vec<MemberTerm>,
}

/// `case` over a union: the arms are lowered in the union's declared member
/// order, which is the order the checked node holds them in.
struct CaseFrame<'n> {
    node: CheckedNode<'n>,
    /// The scrutinee's union declaration.
    declaration: NodeKey,
    arms: &'n [CaseArm],
    /// The scrutinee's term, once it is lowered.
    scrutinee: Option<LeafTerm>,
    /// The arms lowered so far: each a binding of the member's name over its
    /// binder aggregate and its body.
    lowered: Vec<MemberTerm>,
    /// The arm whose body is being lowered.
    open: Option<OpenArm>,
}

/// An arm whose binders are in scope and whose body is being lowered.
struct OpenArm {
    /// The parameter node of each binder, in payload position order.
    parameters: Vec<NodeRef>,
}

/// `fold`/`reduce` over `source`.
struct FoldFrame<'n> {
    node: CheckedNode<'n>,
    accumulator: Slot,
    binder: Slot,
    source: CheckedNode<'n>,
    step: CheckedNode<'n>,
    identity: Option<CheckedNode<'n>>,
    stage: FoldStage,
}

#[allow(
    clippy::large_enum_variant,
    reason = "this stage lives inside the already boxed FoldFrame; boxing its LeafTerm payload would add a second allocation per fold"
)]
enum FoldStage {
    /// The source is being lowered.
    Source,
    /// The identity is being lowered.
    Identity(LeafTerm),
    /// The step is being lowered under the accumulator and element binders.
    Step {
        source: LeafTerm,
        identity: Option<LeafTerm>,
        accumulator: String,
        binder: String,
    },
}

/// `name`'s `::`-separated segments, each an identifier.
fn qualified_name(name: &str, location: &Location) -> Result<Vec<Identifier>, CheckRefusal> {
    identifiers(name.split("::"), location)
}

/// `segments` as a qualified name's identifiers, refusing the first segment
/// that is not one.
fn identifiers<'s>(
    segments: impl IntoIterator<Item = &'s str>,
    location: &Location,
) -> Result<Vec<Identifier>, CheckRefusal> {
    segments
        .into_iter()
        .map(|segment| {
            Identifier::new(segment).map_err(|_| {
                refuse(
                    location,
                    CheckCause::NodePreimage(NodeKeyRefusal::InvalidQualifiedNameSegment {
                        segment: segment.to_owned(),
                    }),
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
    /// `member:<name>`, the name already checked to be an identifier.
    Member(&'w str),
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
            Self::Member(name) => key_length("member:".len() + name.len()),
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
            Self::Member(name) => LeafSegment::Member(
                Identifier::new(name).map_err(|_| NodeKeyRefusal::EmptyBindingName)?,
            ),
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
///
/// The walk runs on the walker toolkit ([`quire_walk::walk`]), one typed
/// frame per entered type (FR-258), so its native stack use is the same at
/// every depth. Its heap stack holds, for each type on the current path,
/// that type's frame and its not yet entered siblings: every one an element
/// of a declared type the check stage already charged (FR-062's declaration
/// preimage bytes for a parameter's type, and one work unit for each
/// composite the walk enters).
struct LeafWalk<'w> {
    /// The package's declared composites.
    types: &'w TypeEnvironment,
    /// Where the walked type is compared: every refusal names it.
    location: &'w Location,
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
    /// The open composites, in the order the walk entered them.
    open: Vec<NodeKey>,
    /// The path length each open composite was entered at.
    entered_at: BTreeMap<NodeKey, usize>,
    /// The node-limit units left.
    budget: u64,
    /// The node limit, which a refusal names.
    limit: u64,
}

/// One type the text-leaf walk reaches.
#[derive(Clone, Copy)]
enum LeafVisit<'w> {
    /// `value_type`, reached one `segment` below the path (none for the
    /// compared type itself).
    Type {
        segment: Option<WalkSegment<'w>>,
        value_type: &'w ValueType,
    },
    /// A record field (rule 5): `field:<name>`, then the field's type, an
    /// optional field's through `inner`.
    Field(&'w FieldDeclaration),
    /// A union member (QSpec FR-440 "Structural leaves"): `member:<name>`,
    /// then each payload type at `position:<i>`, `i` its index in the
    /// member's payload even when the payload is one type.
    Member(&'w UnionMemberDeclaration),
}

/// One entered type: what leaving it undoes.
struct LeafFrame {
    /// Whether entering it pushed a path segment.
    segment: bool,
    /// The open-list length on entry, when it opened a composite.
    opened: Option<usize>,
}

impl<'w> quire_walk::Walk for LeafWalk<'w> {
    type Node = LeafVisit<'w>;
    type Frame = LeafFrame;
    type Stop = CheckRefusal;

    fn enter(
        &mut self,
        visit: LeafVisit<'w>,
        children: &mut quire_walk::Children<'_, LeafVisit<'w>>,
    ) -> ControlFlow<CheckRefusal, LeafFrame> {
        let mut queued = Vec::new();
        let frame = match visit {
            LeafVisit::Type {
                segment,
                value_type,
            } => {
                if let Some(segment) = segment {
                    self.push(segment);
                }
                let opened = self.enter_type(value_type, &mut queued);
                LeafFrame {
                    segment: segment.is_some(),
                    opened: into_flow(opened)?,
                }
            }
            LeafVisit::Member(member) => {
                let name = member.identifier();
                if !quire_exact::is_identifier(name) {
                    return ControlFlow::Break(refuse(
                        self.location,
                        CheckCause::NodePreimage(NodeKeyRefusal::EmptyBindingName),
                    ));
                }
                self.push(WalkSegment::Member(name));
                queued.extend(
                    (0_u64..)
                        .zip(member.positions())
                        .map(|(position, value_type)| LeafVisit::Type {
                            segment: Some(WalkSegment::Position(position)),
                            value_type,
                        }),
                );
                LeafFrame {
                    segment: true,
                    opened: None,
                }
            }
            LeafVisit::Field(field) => {
                let name = field.name();
                if !quire_exact::is_identifier(name) {
                    return ControlFlow::Break(refuse(
                        self.location,
                        CheckCause::NodePreimage(NodeKeyRefusal::EmptyBindingName),
                    ));
                }
                self.push(WalkSegment::Field(name));
                queued.push(LeafVisit::Type {
                    segment: (field.presence() == Presence::Optional).then_some(WalkSegment::Inner),
                    value_type: field.value_type(),
                });
                LeafFrame {
                    segment: true,
                    opened: None,
                }
            }
        };
        children.extend(queued);
        ControlFlow::Continue(frame)
    }

    fn exit(&mut self, frame: LeafFrame) -> ControlFlow<CheckRefusal> {
        if let Some(open) = frame.opened {
            for closed in self.open.drain(open..) {
                self.entered_at.remove(&closed);
            }
        }
        if frame.segment {
            self.stack.pop();
        }
        ControlFlow::Continue(())
    }
}

/// `result` as the walk's control flow.
fn into_flow<T>(result: Result<T, CheckRefusal>) -> ControlFlow<CheckRefusal, T> {
    match result {
        Ok(value) => ControlFlow::Continue(value),
        Err(refusal) => ControlFlow::Break(refusal),
    }
}

impl<'w> LeafWalk<'w> {
    /// Walk `value_type`, appending each text leaf and recursion leaf in the
    /// order the walk reaches it.
    fn run(&mut self, value_type: &'w ValueType) -> Result<(), CheckRefusal> {
        let root = LeafVisit::Type {
            segment: None,
            value_type,
        };
        match quire_walk::walk(self, root) {
            ControlFlow::Continue(()) => Ok(()),
            ControlFlow::Break(refusal) => Err(refusal),
        }
    }

    /// Extend the path by `segment`.
    fn push(&mut self, segment: WalkSegment<'w>) {
        self.stack.push(StackEntry {
            segment,
            interned: None,
        });
    }

    /// Enter `value_type` at the walk's path: append its leaf, or queue the
    /// types inside it. Returns the open-list length on entry when it opens
    /// a composite.
    fn enter_type(
        &mut self,
        value_type: &'w ValueType,
        queued: &mut Vec<LeafVisit<'w>>,
    ) -> Result<Option<usize>, CheckRefusal> {
        let location = self.location;
        match value_type {
            // Rule 1.
            ValueType::Text(text) => self.append(Leaf::Text(text.profile()), location)?,
            // Native String is not profile-bounded Text and adds no Text leaf.
            ValueType::String => {}
            // Rule 2.
            ValueType::Option(payload) => queued.push(LeafVisit::Type {
                segment: Some(WalkSegment::Inner),
                value_type: payload,
            }),
            ValueType::Collection(collection) => queued.push(LeafVisit::Type {
                segment: Some(WalkSegment::Inner),
                value_type: collection.element(),
            }),
            ValueType::Map(map) => queued.push(LeafVisit::Type {
                segment: Some(WalkSegment::Inner),
                value_type: map.values(),
            }),
            ValueType::Composite(declaration) => {
                // A composite from which no text type is reachable adds no
                // leaf, open or not, so the walk does not enter it (rules 3
                // and 4).
                if !self.reaches_text(declaration.clone(), location)? {
                    return Ok(None);
                }
                // Rule 3: an open composite ends the path.
                if let Some(entered) = self.entered(declaration.clone()) {
                    self.append(Leaf::Recursion(entered), location)?;
                    return Ok(None);
                }
                // Rules 4 and 5.
                let types = self.types;
                let Some(composite) = types.composite(declaration.clone()) else {
                    return Err(fault(
                        location,
                        KeyFault::UnknownComposite(declaration.clone()),
                    ));
                };
                charge_work(self.meter, 1)
                    .map_err(|refusal| preimage_refusal(location, refusal))?;
                let open = self.open.len();
                self.open.push(declaration.clone());
                self.entered_at
                    .insert(declaration.clone(), self.stack.len());
                match composite.shape() {
                    CompositeShape::Record(fields) => {
                        queued.extend(fields.iter().map(LeafVisit::Field));
                    }
                    CompositeShape::Tuple(positions) => {
                        queued.extend((0_u64..).zip(positions).map(|(position, value_type)| {
                            LeafVisit::Type {
                                segment: Some(WalkSegment::Position(position)),
                                value_type,
                            }
                        }));
                    }
                    CompositeShape::Union(members) => {
                        queued.extend(members.iter().map(LeafVisit::Member));
                    }
                }
                return Ok(Some(open));
            }
            // Rule 6.
            ValueType::Boolean
            | ValueType::Integer
            | ValueType::Int(_)
            | ValueType::Rational(_)
            | ValueType::Decimal(_)
            | ValueType::Float(_)
            | ValueType::Quantity(_)
            | ValueType::BoundedQuantity(_)
            | ValueType::Enum(_)
            | ValueType::Reference(_)
            | ValueType::Population(_) => {}
            ValueType::Uuid | ValueType::Timestamp => {}
        }
        Ok(None)
    }

    /// Whether a text type is reachable from the composite `declaration`
    /// (FR-093 "Text leaves"). One search answers it for every composite
    /// reachable from `declaration` whose answer is not yet known, and keeps
    /// each answer for the lowering's later walks, so a chain of composites
    /// is searched once, not once per link.
    fn reaches_text(
        &mut self,
        declaration: NodeKey,
        location: &Location,
    ) -> Result<bool, CheckRefusal> {
        if let Some(reaches) = self.reach.get(&declaration) {
            return Ok(*reaches);
        }
        let types = self.types;
        // Each composite reached and not yet answered, with the unanswered
        // composites its members name; and those whose members name a text
        // type, or a composite known to reach one.
        let mut names: BTreeMap<NodeKey, Vec<NodeKey>> = BTreeMap::new();
        let mut reaching: Vec<NodeKey> = Vec::new();
        let mut pending = vec![declaration.clone()];
        while let Some(composite) = pending.pop() {
            if names.contains_key(&composite) {
                continue;
            }
            let mut members: Vec<&ValueType> =
                match types.composite(composite.clone()).map(|c| c.shape()) {
                    Some(CompositeShape::Record(fields)) => {
                        fields.iter().map(FieldDeclaration::value_type).collect()
                    }
                    Some(CompositeShape::Tuple(positions)) => positions.iter().collect(),
                    Some(CompositeShape::Union(union_members)) => union_members
                        .iter()
                        .flat_map(|member| member.positions())
                        .collect(),
                    None => return Err(fault(location, KeyFault::UnknownComposite(composite))),
                };
            let mut named = Vec::new();
            let mut direct = false;
            while let Some(value_type) = members.pop() {
                match value_type {
                    ValueType::Text(_) => direct = true,
                    ValueType::Option(payload) => members.push(payload),
                    ValueType::Collection(collection) => members.push(collection.element()),
                    ValueType::Map(map) => members.push(map.values()),
                    ValueType::Composite(next) => match self.reach.get(next) {
                        Some(true) => direct = true,
                        Some(false) => {}
                        None => {
                            named.push(next.clone());
                            pending.push(next.clone());
                        }
                    },
                    ValueType::Boolean
                    | ValueType::Integer
                    | ValueType::Int(_)
                    | ValueType::Rational(_)
                    | ValueType::Decimal(_)
                    | ValueType::Float(_)
                    | ValueType::Quantity(_)
                    | ValueType::BoundedQuantity(_)
                    | ValueType::String
                    | ValueType::Enum(_)
                    | ValueType::Reference(_)
                    | ValueType::Population(_) => {}
                    ValueType::Uuid | ValueType::Timestamp => {}
                }
            }
            if direct {
                reaching.push(composite.clone());
            }
            names.insert(composite, named);
        }
        // A composite reaches a text type when one it names does: follow the
        // names backwards from every composite that reaches one directly.
        let mut named_by: BTreeMap<NodeKey, Vec<NodeKey>> = BTreeMap::new();
        for (composite, named) in &names {
            for next in named {
                named_by
                    .entry(next.clone())
                    .or_default()
                    .push(composite.clone());
            }
        }
        let mut reaches = BTreeSet::new();
        while let Some(composite) = reaching.pop() {
            if reaches.insert(composite.clone()) {
                reaching.extend(named_by.get(&composite).into_iter().flatten().cloned());
            }
        }
        for composite in names.keys() {
            self.reach
                .insert(composite.clone(), reaches.contains(composite));
        }
        Ok(reaches.contains(&declaration))
    }

    /// The path length `declaration` was entered at, when it is open.
    fn entered(&self, declaration: NodeKey) -> Option<usize> {
        self.entered_at.get(&declaration).copied()
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
                CheckCause::ResourceExhausted(Box::new(StageLimitCause {
                    stage: CheckingStage::Typing,
                    kind: CheckingLimitKind::Nodes,
                    limit: self.limit,
                    // `budget` reaches 0 once exactly `self.limit` leaves
                    // have been appended; this one would have been the
                    // `self.limit + 1`-th.
                    actual: u128::from(self.limit) + 1,
                    region: None,
                })),
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
        protocol_profile: Option<&'a DefinitionReference>,
        function_count: usize,
        occurrences: &'a mut OccurrenceMap<Location>,
        meter: &'a mut Meter,
        package_name: &str,
    ) -> Self {
        Self {
            package_name: package_name.to_owned(),
            node_index: HashMap::new(),
            ordinals: BTreeMap::new(),
            scope,
            owner: owner.clone(),
            models,
            units,
            correspondence: ModelCorrespondence::default(),
            protocol_profile,
            graph: SemanticGraph::default(),
            occurrences,
            functions: vec![None; function_count],
            composites: BTreeMap::new(),
            unsettled_functions: Vec::new(),
            unsettled_composites: Vec::new(),
            composites_in_progress: Vec::new(),
            composite_placeholders: BTreeMap::new(),
            functions_open: false,
            timed_places: Vec::new(),
            placeholders: BTreeMap::new(),
            placeholder_count: 0,
            drafts: BTreeMap::new(),
            draft_occurrences: Vec::new(),
            binders: BTreeMap::new(),
            draft_binders: Vec::new(),
            parameter_spans: BTreeMap::new(),
            meter,
            node_limit: u64::MAX,
            node_budget: u64::MAX,
            text_reach: BTreeMap::new(),
            frame_origins: BTreeMap::new(),
            population_nodes: BTreeSet::new(),
            bound_sites: BTreeSet::new(),
            protocol_keys: Vec::new(),
            protocol_members: BTreeMap::new(),
            protocol_attempts: Vec::new(),
            protocol_compensations: Vec::new(),
            temporal_keys: BTreeMap::new(),
            refused_temporals: BTreeSet::new(),
            protocol_binder_keys: BTreeMap::new(),
            protocol_expression_binders: BTreeMap::new(),
            active_protocol: None,
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
    /// `stage_limit_exceeded` on the work budget at `location`.
    fn charge(&mut self, work: u64, location: &Location) -> Result<(), CheckRefusal> {
        charge_work(self.meter, work).map_err(|refusal| preimage_refusal(location, refusal))
    }

    /// The finished graph, its model correspondence and each
    /// function's key. Every node that no region denotes gets its one
    /// `generated` occurrence (FR-093), at `root`.
    pub(crate) fn finish(self, root: &Location) -> Lowered {
        let enclosing = enclosing_declarations(&self.graph, self.occurrences);
        for key in self.graph.nodes.keys() {
            if !self.occurrences.has(key.clone()) {
                let at = enclosing.get(key).unwrap_or(root);
                self.occurrences
                    .record(key.clone(), OccurrenceRole::Generated, at.clone());
            }
        }
        let mut parameter_sites: BTreeMap<(NodeKey, Location), qsl_foundation::Span> = self
            .parameter_spans
            .iter()
            .filter_map(|(site, span)| {
                self.binders
                    .get(site)
                    .map(|key| ((key.clone(), site.binder.clone()), *span))
            })
            .collect();
        let parameter_spans = self
            .occurrences
            .iter()
            .filter_map(|(key, origin, location)| {
                // The declaration occurrence precedes reads of its parameter;
                // removing the site assigns its span to that one occurrence only.
                parameter_sites
                    .remove(&(key.clone(), location.clone()))
                    .map(|span| ((key, origin), span))
            })
            .collect();
        Lowered {
            graph: self.graph,
            correspondence: self.correspondence,
            functions: self
                .functions
                .into_iter()
                .map(|node| node.and_then(|node| node.admitted().cloned()))
                .collect(),
            binders: self.binders,
            parameter_spans,
            population_nodes: self.population_nodes,
            composites: self
                .composites
                .into_iter()
                .filter_map(|(declaration, node)| {
                    node.admitted().cloned().map(|node| (declaration, node))
                })
                .collect(),
            protocol_members: self.protocol_members,
        }
    }

    /// Record an occurrence of `key`: a draft's waits until it is keyed.
    fn record(&mut self, key: impl Into<NodeRef>, role: OccurrenceRole, location: Location) {
        match key.into() {
            NodeRef::Admitted(key) => {
                self.occurrences.record(key, role, location);
            }
            pending @ NodeRef::Pending(_) => {
                self.draft_occurrences.push((pending, role, location));
            }
        }
    }

    /// The `Origin` of one `generated` occurrence of `frame`, the operation
    /// `(declaring, name)`'s frame node (FR-104 "Requirements", FR-104-AC-5;
    /// FR-105's `frame` row): the first call for a given operation mints it,
    /// and every later call naming the same operation returns the same
    /// `Origin`, in the caller's own dedup key -- never the frame node's
    /// key, since two unrelated operations can key one equal-content frame
    /// node and must still keep two records. [`Lowering::register_frame_occurrences`]
    /// calls this for every operation the unit's clauses name, in ascending
    /// (declaring `DeclarationKey`, name) order, before any clause is
    /// lowered, so the ordinal an operation's occurrence gets never depends
    /// on which clause names it first (SR-736 FND-008): a later, per-clause
    /// call only ever hits the cache below. State clauses are lowered after
    /// every function, so a frame node is never a pending draft in
    /// practice; that case is an internal fault, not a silent fallback.
    fn frame_occurrence(
        &mut self,
        declaring: EffectiveId,
        name: &str,
        frame: impl Into<NodeRef>,
        location: &Location,
    ) -> Result<OccurrenceOrigin, CheckRefusal> {
        let key = (declaring, name.to_owned());
        if let Some(origin) = self.frame_origins.get(&key) {
            return Ok(origin.clone());
        }
        let NodeRef::Admitted(frame) = frame.into() else {
            return Err(fault(location, KeyFault::UnresolvedDraft));
        };
        let origin = self
            .occurrences
            .record(frame, OccurrenceRole::Generated, location.clone());
        self.frame_origins.insert(key, origin.clone());
        Ok(origin)
    }

    /// Record `key` as the parameter node of the binder at `site`: a
    /// draft's waits until it is keyed.
    fn record_binder(&mut self, site: BinderSite, key: impl Into<NodeRef>) {
        match key.into() {
            NodeRef::Admitted(key) => {
                self.binders.insert(site, key);
            }
            pending @ NodeRef::Pending(_) => self.draft_binders.push((site, pending)),
        }
    }

    /// Whether `node` is a local handle of a placeholder or a draft.
    fn pending(node: &NodeRef) -> bool {
        matches!(node, NodeRef::Pending(_))
    }

    /// A new placeholder.
    fn placeholder(&mut self) -> NodeRef {
        let handle = self.fresh_handle();
        self.placeholders.insert(handle, None);
        NodeRef::Pending(handle)
    }

    /// A fresh local handle that names no admitted node.
    fn fresh_handle(&mut self) -> PlaceholderId {
        let handle = PlaceholderId(self.placeholder_count as usize);
        self.placeholder_count += 1;
        handle
    }

    /// [`Self::insert_ref`] for a node that names no pending handle: the
    /// admitted key.
    fn insert(
        &mut self,
        location: &Location,
        node_tag: NodeTag,
        semantic_form: &'static str,
        semantic_type: Option<NodeKey>,
        declaration: Option<Vec<Identifier>>,
        body: BodyTerm,
    ) -> Result<NodeKey, CheckRefusal> {
        let node = self.insert_ref(
            location,
            node_tag,
            semantic_form,
            semantic_type.map(NodeRef::from),
            declaration,
            body,
        )?;
        require_admitted(node, location)
    }

    /// Key an unowned or source-declared node and add it to the graph: a
    /// node with a `declaration` carries the unit's `SourceOwner`.
    fn insert_ref(
        &mut self,
        location: &Location,
        node_tag: NodeTag,
        semantic_form: &'static str,
        semantic_type: Option<NodeRef>,
        declaration: Option<Vec<Identifier>>,
        body: BodyTerm,
    ) -> Result<NodeRef, CheckRefusal> {
        let owner = declaration
            .as_ref()
            .map(|_| Owner::Source(self.owner.clone()));
        self.insert_node(
            location,
            NodeContent {
                owner_model: None,
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
    fn insert_owned_ref(
        &mut self,
        location: &Location,
        node_tag: NodeTag,
        semantic_form: &'static str,
        semantic_type: Option<NodeRef>,
        owner: Owner,
        body: BodyTerm,
    ) -> Result<NodeRef, CheckRefusal> {
        self.insert_node(
            location,
            NodeContent {
                owner_model: None,
                node_tag,
                semantic_form,
                semantic_type,
                declaration: None,
                owner: Some(owner),
                body,
            },
        )
    }

    /// Add a node to the graph under its admitted structural key. A node
    /// naming a pending handle is a draft until [`Self::settle`] admits it.
    fn insert_node(
        &mut self,
        location: &Location,
        content: NodeContent,
    ) -> Result<NodeRef, CheckRefusal> {
        self.charge(1, location)?;
        let mut names_pending = content.semantic_type.as_ref().is_some_and(Self::pending);
        content
            .body
            .for_each_ref(&mut |named| names_pending |= Self::pending(named));
        if names_pending {
            // Admitted by `settle`; until then a local handle names the draft.
            let handle = self.fresh_handle();
            self.drafts.insert(
                handle,
                Draft {
                    location: location.clone(),
                    content,
                },
            );
            return Ok(NodeRef::Pending(handle));
        }
        let key = self.admitted_node_key(location, &content)?;
        if let btree_map::Entry::Vacant(slot) = self.graph.nodes.entry(key.clone()) {
            slot.insert(SemanticNode {
                key: key.clone(),
                recursion: None,
                nominal: None,
                content,
            });
        }
        Ok(NodeRef::Admitted(key))
    }

    /// [`Self::insert_node`] for content that names no pending handle: the
    /// admitted key.
    pub(super) fn insert_content_key(
        &mut self,
        location: &Location,
        content: NodeContent,
    ) -> Result<NodeKey, CheckRefusal> {
        let node = self.insert_node(location, content)?;
        require_admitted(node, location)
    }

    /// The structural key of an unpended `content`: equal content is one
    /// node; a declared node is keyed by its declaration's path, any other
    /// by its closed tag and semantic form alone. Repeats of one path take
    /// the next unused ordinal, so a key's size never depends on how deep
    /// its operands nest (QSL-381: no depth cap, and a key that nested its
    /// operands' keys would cost memory quadratic in a chain's depth).
    fn admitted_node_key(
        &mut self,
        location: &Location,
        content: &NodeContent,
    ) -> Result<NodeKey, CheckRefusal> {
        if let Some(key) = self.node_index.get(content) {
            return Ok(key.clone());
        }
        let package = self.package_name.clone();
        let key = self.mint_key(location, |ordinal| match &content.declaration {
            Some(names) => super::declared_node_key(
                package.clone(),
                names.iter().map(|name| name.as_str().to_owned()).collect(),
                ordinal,
            ),
            // BLOCKED-BY QSL-1312 (owner: planner titan:QSL:plan): shareable operand representation for derived node keys (FR-322 embeds the operand node ids in the path; interim is tag, form and producer ordinal)
            None => super::derived_node_key(
                &package,
                content.node_tag,
                content.semantic_form,
                Vec::new(),
                ordinal,
            ),
        })?;
        self.node_index.insert(content.clone(), key.clone());
        Ok(key)
    }

    /// Mint the first unused key of one structural path, `build` forming the
    /// key at an ordinal: 0 unless the same package and path repeat.
    fn mint_key(
        &mut self,
        location: &Location,
        build: impl Fn(u64) -> Result<NodeKey, quire_exact::InvalidNodeKey>,
    ) -> Result<NodeKey, CheckRefusal> {
        let invalid = |refusal: quire_exact::InvalidNodeKey| {
            fault(location, KeyFault::ModelRootKey(refusal.to_string()))
        };
        let base = build(0).map_err(invalid)?;
        let next = self.ordinals.entry(base.clone()).or_insert(0);
        loop {
            let ordinal = *next;
            *next += 1;
            let key = if ordinal == 0 {
                base.clone()
            } else {
                build(ordinal).map_err(invalid)?
            };
            if !self.graph.nodes.contains_key(&key) {
                return Ok(key);
            }
        }
    }

    /// The admitted keys of one recursion group's members, in member order.
    /// A declared member is keyed by its declaration's path; any other
    /// member by the group's anchor (its first declared member) and its
    /// own tag, form and position. A group with no declared member has no
    /// anchor, which is an internal fault.
    fn admitted_group_keys(
        &mut self,
        members: &[&Draft],
        location: &Location,
    ) -> Result<Vec<NodeKey>, CheckRefusal> {
        let package = self.package_name.clone();
        let mut keys: Vec<Option<NodeKey>> = Vec::with_capacity(members.len());
        for draft in members {
            keys.push(match &draft.content.declaration {
                Some(names) => Some(self.mint_key(&draft.location, |ordinal| {
                    super::declared_node_key(
                        package.clone(),
                        names.iter().map(|name| name.as_str().to_owned()).collect(),
                        ordinal,
                    )
                })?),
                None => None,
            });
        }
        let anchor = keys
            .iter()
            .flatten()
            .next()
            .cloned()
            .ok_or_else(|| fault(location, KeyFault::InvalidGroup))?;
        let mut admitted = Vec::with_capacity(members.len());
        for (draft, key) in members.iter().zip(keys) {
            admitted.push(match key {
                Some(key) => key,
                None => self.mint_key(&draft.location, |ordinal| {
                    super::structural_node_key(
                        &package,
                        vec![
                            PathSegment::Text("recursion_group".to_owned()),
                            PathSegment::Node(anchor.clone()),
                            PathSegment::Text(draft.content.node_tag.as_str().to_owned()),
                            PathSegment::Text(draft.content.semantic_form.to_owned()),
                        ],
                        ordinal,
                    )
                })?,
            });
        }
        Ok(admitted)
    }

    // ------------------------------------------------------------------
    // FR-092 type nodes
    // ------------------------------------------------------------------

    /// A builtin `scalar_type` node: its own semantic type, empty body.
    fn scalar(&mut self, form: &'static str, location: &Location) -> Result<NodeKey, CheckRefusal> {
        let node = self.insert_ref(
            location,
            NodeTag::ScalarType,
            form,
            None,
            None,
            BodyTerm::aggregate(Vec::new()),
        )?;
        require_admitted(node, location)
    }

    /// An integer literal typed at the builtin `Integer` node (T2).
    fn integer_literal(
        &mut self,
        value: Integer,
        location: &Location,
    ) -> Result<LeafTerm, CheckRefusal> {
        let integer = self.scalar("integer", location)?;
        Ok(LeafTerm::literal(integer, LiteralValue::Integer(value)))
    }

    /// A text literal typed at the builtin text scalar node (T3).
    fn text_literal(&mut self, value: &str, location: &Location) -> Result<LeafTerm, CheckRefusal> {
        let text = self.scalar("text", location)?;
        Ok(LeafTerm::literal(
            text,
            LiteralValue::Text(value.to_owned()),
        ))
    }

    /// A `bounded_domain` node over `base` with `bindings`.
    fn bounded(
        &mut self,
        form: &'static str,
        base: impl Into<NodeRef>,
        bindings: Vec<(&'static str, LeafTerm)>,
        location: &Location,
    ) -> Result<NodeRef, CheckRefusal> {
        let members = bindings
            .into_iter()
            .map(|(name, value)| MemberTerm::bound(name, value))
            .collect();
        self.insert_ref(
            location,
            NodeTag::BoundedDomain,
            form,
            Some(base.into()),
            None,
            BodyTerm::aggregate(members),
        )
    }

    /// The FR-092 type node of `value_type`, built without native
    /// recursion: each `option`, collection and declared
    /// composite still being built is a [`TypeFrame`] on an explicit stack,
    /// so a long composite chain costs heap, not stack. Nodes are built,
    /// charged and refused in the order a depth-first recursive build would
    /// reach them, so every key is the recursive build's key.
    pub(crate) fn type_ref<'v>(
        &mut self,
        value_type: &'v ValueType,
        location: &Location,
    ) -> Result<NodeRef, CheckRefusal>
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

    /// [`Self::type_node`] for a type that names no pending handle: the
    /// admitted key.
    pub(crate) fn type_node<'v>(
        &mut self,
        value_type: &'v ValueType,
        location: &Location,
    ) -> Result<NodeKey, CheckRefusal>
    where
        'a: 'v,
    {
        let node = self.type_ref(value_type, location)?;
        require_admitted(node, location)
    }

    /// [`Self::type_ref`]'s loop: descend into a type, or hand a built
    /// node's key to the frame waiting for it.
    fn build_type_node<'v>(
        &mut self,
        value_type: &'v ValueType,
        location: &Location,
    ) -> Result<NodeRef, CheckRefusal>
    where
        'a: 'v,
    {
        let mut frames: Vec<TypeFrame<'v>> = Vec::new();
        let mut step = TypeStep::Descend(value_type);
        loop {
            step = match step {
                TypeStep::Descend(value_type) => self.descend(value_type, location, &mut frames)?,
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

    /// Start `value_type`'s node: a type with no type inside it
    /// is built now; an `option`, a collection or a composite not yet built
    /// pushes its frame and descends into its first inner type.
    fn descend<'v>(
        &mut self,
        value_type: &'v ValueType,
        location: &Location,
        frames: &mut Vec<TypeFrame<'v>>,
    ) -> Result<TypeStep<'v>, CheckRefusal>
    where
        'a: 'v,
    {
        let key = match value_type {
            ValueType::Option(payload) => {
                frames.push(TypeFrame::Option);
                return Ok(TypeStep::Descend(payload));
            }
            ValueType::Collection(collection) => {
                frames.push(TypeFrame::Collection(collection));
                return Ok(TypeStep::Descend(collection.element()));
            }
            ValueType::Composite(declaration) => {
                return match self.open_composite(declaration.clone(), location)? {
                    OpenComposite::Built(key) => Ok(TypeStep::Built(key)),
                    OpenComposite::Open(composite) => self.advance(composite, location, frames),
                };
            }
            ValueType::Boolean => self.scalar("boolean", location).map(NodeRef::from),
            ValueType::Integer => self.scalar("integer", location).map(NodeRef::from),
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
            ValueType::Float(float) => {
                let form = match float.width() {
                    quire_exact::IeeeWidth::Binary32 => "float32",
                    quire_exact::IeeeWidth::Binary64 => "float64",
                };
                let base = self.scalar(form, location)?;
                let rounding = self.text_literal(float.rounding().as_str(), location)?;
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
                let scope: &'a Scope = self.scope;
                if let Some(binding) = scope.enum_binding_of(shape) {
                    Ok(NodeRef::from(binding.declaration.key()))
                } else if let Some(variant) = scope.variant_type_of(shape) {
                    // FR-360 item 12: a model variant type is its
                    // `model`/`variant_type` node.
                    self.variant_type_nodes(variant, location)
                        .map(NodeRef::from)
                } else {
                    Err(refuse(
                        location,
                        CheckCause::IllTyped(quire_exact::IllTypedCause::TypeMismatch),
                    ))
                }
            }
            ValueType::Quantity(unit) => self
                .quantity_type(unit.clone(), location)
                .map(NodeRef::from),
            // No checked type node carries a map or a bounded quantity yet:
            // a typed refusal, never a guessed node.
            ValueType::Map(_) | ValueType::BoundedQuantity(_) => Err(refuse(
                location,
                CheckCause::IllTyped(quire_exact::IllTypedCause::TypeMismatch),
            )),
            ValueType::Reference(target) => self
                .reference_type(target.clone(), location)
                .map(NodeRef::from),
            // FR-094: a `Population<T>[N]` node is built from its binding's
            // resolved type form ([`Self::binder_type`]); the checked type
            // alone carries no `T`.
            ValueType::Population(_) => Err(fault(location, KeyFault::UntargetedPopulation)),
            // FR-056: the native String, UUID and Timestamp each a distinct
            // bare scalar node, as a record or event field carries them.
            ValueType::String => self.scalar("string", location).map(NodeRef::from),
            ValueType::Uuid => self.scalar("uuid", location).map(NodeRef::from),
            ValueType::Timestamp => self.scalar("timestamp", location).map(NodeRef::from),
        }?;
        Ok(TypeStep::Built(key))
    }

    /// The `option` node over the payload node `payload`.
    fn option_type(
        &mut self,
        payload: NodeRef,
        location: &Location,
    ) -> Result<NodeRef, CheckRefusal> {
        self.insert_ref(
            location,
            NodeTag::CompositeType,
            "option",
            None,
            None,
            BodyTerm::aggregate(vec![MemberTerm::reference_node(payload)]),
        )
    }

    /// `collection`'s bounded node over the element node `element`.
    fn collection_type(
        &mut self,
        collection: &CollectionType,
        element: NodeRef,
        location: &Location,
    ) -> Result<NodeRef, CheckRefusal> {
        let base = self.insert_ref(
            location,
            NodeTag::CompositeType,
            collection_form(collection.kind()),
            None,
            None,
            BodyTerm::aggregate(vec![MemberTerm::reference_node(element)]),
        )?;
        // An unbounded `K<T>` is its composite node alone: it has no
        // `collection_bounds` domain (ADR-014 N-3), which is the form IR's
        // `requires-bound` reads as unbounded.
        let Some(bound) = collection.bound() else {
            return Ok(base);
        };
        let min = self.integer_literal(Integer::from(bound.minimum()), location)?;
        let max = self.integer_literal(Integer::from(bound.maximum()), location)?;
        self.bounded(
            "collection_bounds",
            base,
            vec![("min", min), ("max", max)],
            location,
        )
    }

    /// Start the declared record or tuple `declaration`: its node
    /// if it is built, its placeholder if it is being built (it reached
    /// itself), or its open frame.
    fn open_composite<'v>(
        &mut self,
        declaration: NodeKey,
        location: &Location,
    ) -> Result<OpenComposite<'v>, CheckRefusal>
    where
        'a: 'v,
    {
        if let Some(key) = self.composites.get(&declaration) {
            return Ok(OpenComposite::Built(key.clone()));
        }
        // FR-359 item 3: a model record value type or event type is a
        // model-owned node, never a `composite_type`.
        // BLOCKED-BY QSL-1282 (owner: planner titan:QSL:plan): model record declaration lookup was keyed by a digest node key; needs the structural model-record key source
        if let Some(model_record) = self.model_record_declaration(declaration.clone(), location)? {
            let key = self.model_node(&model_record, location)?;
            // The record has no checked occurrence of its own: root it at the
            // `model` declaration that selects its package, as a relationship
            // is. The selecting declaration is the model that holds the
            // record, not a part of its display name.
            let alias = self
                .models
                .iter()
                .find(|model| model.record(&model_record).is_some())
                .and_then(|model| model.alias())
                .ok_or_else(|| fault(location, KeyFault::UnknownDeclaration(model_record.clone())))?
                .to_owned();
            self.record(
                key.clone(),
                OccurrenceRole::Generated,
                type_declaration(&alias),
            );
            let key = NodeRef::Admitted(key);
            self.composites.insert(declaration, key.clone());
            return Ok(OpenComposite::Built(key));
        }
        let scope: &'a Scope = self.scope;
        let Some(composite) = scope.types().composite(declaration.clone()) else {
            return Err(refuse(
                location,
                CheckCause::IllTyped(quire_exact::IllTypedCause::TypeMismatch),
            ));
        };
        if self.composites_in_progress.contains(&declaration) {
            // FR-092: a record reaching itself is in a recursion group; a
            // placeholder names it until its node is built.
            if let Some(placeholder) = self.composite_placeholders.get(&declaration) {
                return Ok(OpenComposite::Built(placeholder.clone()));
            }
            let placeholder = self.placeholder();
            self.composite_placeholders
                .insert(declaration, placeholder.clone());
            return Ok(OpenComposite::Built(placeholder));
        }
        let name = qualified_name(composite.name(), location)?;
        self.composites_in_progress.push(declaration.clone());
        let shape = composite.shape();
        let members = match shape {
            CompositeShape::Record(fields) => fields.len(),
            CompositeShape::Tuple(positions) => positions.len(),
            CompositeShape::Union(members) => members.len(),
        };
        Ok(OpenComposite::Open(CompositeFrame {
            declaration,
            name,
            site: type_declaration(composite.name()),
            shape,
            members: Vec::with_capacity(members),
            awaiting: None,
            payload: Vec::new(),
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
        let at = composite.members.len();
        let shape: &'v CompositeShape = composite.shape;
        let next = match shape {
            CompositeShape::Record(fields) => fields.get(at).map(|field| {
                composite.awaiting = Some(field);
                (field.value_type(), field.presence())
            }),
            CompositeShape::Tuple(positions) => positions
                .get(at)
                .map(|position| (position, Presence::Required)),
            CompositeShape::Union(members) => {
                return self.advance_union(composite, members, location, frames)
            }
        };
        let Some((value_type, presence)) = next else {
            return Ok(TypeStep::Built(self.close_composite(composite, location)?));
        };
        frames.push(TypeFrame::Composite(composite));
        Ok(match presence {
            Presence::Required => TypeStep::Descend(value_type),
            // An optional field's member is the `option` node over its
            // declared type, one level further down.
            Presence::Optional => {
                frames.push(TypeFrame::Option);
                TypeStep::Descend(value_type)
            }
        })
    }

    /// Descend into the next payload type of the union `composite` being
    /// built, closing each member whose payload is built as one binding of
    /// the member's name over the references to its payload type nodes
    /// (QSpec FR-440 "Union type node"), and finishing the node once every
    /// member is closed.
    fn advance_union<'v>(
        &mut self,
        mut composite: CompositeFrame<'v>,
        members: &'v [UnionMemberDeclaration],
        location: &Location,
        frames: &mut Vec<TypeFrame<'v>>,
    ) -> Result<TypeStep<'v>, CheckRefusal> {
        loop {
            let Some(member) = members.get(composite.members.len()) else {
                return Ok(TypeStep::Built(self.close_composite(composite, location)?));
            };
            match member.positions().get(composite.payload.len()) {
                Some(value_type) => {
                    frames.push(TypeFrame::Composite(composite));
                    return Ok(TypeStep::Descend(value_type));
                }
                None => {
                    let payload = std::mem::take(&mut composite.payload);
                    composite.members.push(MemberTerm::binding(
                        member.identifier(),
                        BindingValue::Group(GroupTerm::references_nodes(payload)),
                    ));
                }
            }
        }
    }

    /// Key the composite whose every member is built, and settle once no
    /// composite or function is still open.
    fn close_composite(
        &mut self,
        composite: CompositeFrame<'_>,
        location: &Location,
    ) -> Result<NodeRef, CheckRefusal> {
        let CompositeFrame {
            declaration,
            name,
            site,
            shape,
            members,
            ..
        } = composite;
        self.composites_in_progress.pop();
        let form = match shape {
            CompositeShape::Record(_) => "record",
            CompositeShape::Tuple(_) => "tuple",
            CompositeShape::Union(_) => "union",
        };
        let key = self.insert_ref(
            location,
            NodeTag::CompositeType,
            form,
            None,
            Some(name),
            BodyTerm::aggregate(members),
        )?;
        // FR-322: a node carrying `declaration` has a `declaration`
        // occurrence.
        self.record(key.clone(), OccurrenceRole::Declaration, site);
        if let Some(NodeRef::Pending(placeholder)) =
            self.composite_placeholders.remove(&declaration)
        {
            self.placeholders.insert(placeholder, Some(key.clone()));
        }
        self.composites.insert(declaration.clone(), key.clone());
        self.unsettled_composites.push(declaration.clone());
        if self.composites_in_progress.is_empty() && !self.functions_open {
            self.settle()?;
        }
        Ok(self.composites.get(&declaration).cloned().unwrap_or(key))
    }

    /// A declared record or tuple's node: its `declaration` and the unit's
    /// `owner`, one binding per field (record) or one reference per position
    /// (tuple).
    fn composite_type_node(
        &mut self,
        declaration: NodeKey,
        location: &Location,
    ) -> Result<NodeRef, CheckRefusal> {
        self.type_ref(&ValueType::Composite(declaration), location)
    }

    /// The node of the declared composite `declaration` (FR-092-AC-12): its
    /// checked type node's id.
    pub(crate) fn composite_node(&mut self, declaration: NodeKey) -> Result<NodeKey, CheckRefusal> {
        let location = generated_location();
        let node = self.composite_type_node(declaration, &location)?;
        require_admitted(node, &location)
    }

    /// Build the admitted enum `binding`'s declaration node and one node
    /// per member (FR-092 rule 1), each keyed by its QSpec nominal preimage:
    /// the keys admission retained, which every enum type and enum literal
    /// already names. The declaration carries its qualified name and a
    /// `declaration` occurrence at the declared name (FR-322).
    pub(crate) fn enum_nodes(&mut self, binding: &EnumBinding) -> Result<(), CheckRefusal> {
        let site = type_declaration(&binding.name);
        let declaration = binding.declaration.key();
        let name = identifiers(
            binding
                .declaration
                .qualified_declaration()
                .iter()
                .map(String::as_str),
            &site,
        )?;
        let inserted = self.insert_nominal(
            &site,
            declaration.clone(),
            NodeContent {
                owner_model: None,
                node_tag: NodeTag::ScalarType,
                semantic_form: "enum",
                semantic_type: None,
                declaration: Some(name),
                owner: Some(Owner::Source(self.owner.clone())),
                body: BodyTerm::aggregate(Vec::new()),
            },
            Some(NominalPayload::Enum(EnumPayload::of(
                &binding.declaration.preimage(),
            ))),
        )?;
        // Two bindings of one admitted declaration are one node, declared
        // once.
        if inserted {
            self.record(
                declaration.clone(),
                OccurrenceRole::Declaration,
                site.clone(),
            );
        }
        for member in &binding.members {
            self.enum_member_node(&site, declaration.clone(), member)?;
        }
        Ok(())
    }

    /// The `value`/`enum_value` node of `member`, a case of the enumeration
    /// node `declaration` (FR-092 rule 1, FR-360 item 12): keyed by its QSpec
    /// nominal preimage over `declaration` and the exact case name.
    pub(crate) fn enum_member_node(
        &mut self,
        location: &Location,
        declaration: NodeKey,
        member: &EnumValue,
    ) -> Result<(), CheckRefusal> {
        let key = member.member();
        self.insert_nominal(
            location,
            key,
            NodeContent {
                owner_model: None,
                node_tag: NodeTag::Value,
                semantic_form: "enum_value",
                semantic_type: Some(declaration.clone().into()),
                declaration: None,
                owner: None,
                body: BodyTerm::literal(
                    declaration.clone(),
                    LiteralValue::Enum(member.case().to_owned()),
                ),
            },
            None,
        )?;
        Ok(())
    }

    /// Add the node `content` under its admitted structural `key` and return
    /// the key. Nodes are identified by `key` alone, never by their content:
    /// a key already in the graph is left as it is.
    pub(super) fn insert_declared_node(
        &mut self,
        location: &Location,
        key: NodeKey,
        content: NodeContent,
    ) -> Result<NodeKey, CheckRefusal> {
        self.insert_nominal(location, key.clone(), content, None)?;
        Ok(key)
    }

    /// Add the nominal node `key`, whose content is `content`, to the graph,
    /// and whether it was not there yet. A key already in the graph is left
    /// as it is.
    fn insert_nominal(
        &mut self,
        location: &Location,
        key: NodeKey,
        content: NodeContent,
        nominal: Option<NominalPayload>,
    ) -> Result<bool, CheckRefusal> {
        self.charge(1, location)?;
        match self.graph.nodes.entry(key.clone()) {
            btree_map::Entry::Occupied(_) => Ok(false),
            btree_map::Entry::Vacant(slot) => {
                slot.insert(SemanticNode {
                    key,
                    recursion: None,
                    nominal,
                    content,
                });
                Ok(true)
            }
        }
    }

    /// The law of `role`, or the FR-093 refusal.
    fn law(&self, role: LawRole, location: &Location) -> Result<OperationLaw, CheckRefusal> {
        let definition = match role {
            LawRole::TextProfile => Some(DefinitionRole::TextProfile.reference()),
            LawRole::IeeeProfile => self
                .scope
                .ieee_profile
                .as_ref()
                .map(|profile| profile.definition().clone()),
            LawRole::IntegerDivision => self.scope.integer_division.map(|selection| {
                let role = match selection.profile() {
                    quire_exact::DivisionProfile::Euclidean => {
                        DefinitionRole::IntegerDivisionEuclidean
                    }
                    quire_exact::DivisionProfile::Floor => DefinitionRole::IntegerDivisionFloor,
                    quire_exact::DivisionProfile::Truncating => {
                        DefinitionRole::IntegerDivisionTruncating
                    }
                };
                role.reference()
            }),
            LawRole::ProtocolProfile => self.protocol_profile.cloned(),
            LawRole::TemporalProfile => None,
        };
        definition
            .map(|definition| OperationLaw { role, definition })
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
            location,
            meter: &mut *self.meter,
            reach: &mut self.text_reach,
            stack: Vec::new(),
            prefixes: Vec::new(),
            leaves: Vec::new(),
            open: Vec::new(),
            entered_at: BTreeMap::new(),
            budget: self.node_budget,
            limit: self.node_limit,
        };
        walk.run(compared)?;
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
    fn binder_ref(
        &mut self,
        value_type: &ValueType,
        target: Option<EffectiveId>,
        location: &Location,
    ) -> Result<NodeRef, CheckRefusal> {
        match (value_type, target) {
            (ValueType::Population(maximum), Some(target)) => self
                .population_type(target, *maximum, location)
                .map(NodeRef::from),
            _ => self.type_ref(value_type, location),
        }
    }

    /// [`Self::binder_ref`] for a type that names no pending handle.
    fn binder_type(
        &mut self,
        value_type: &ValueType,
        target: Option<EffectiveId>,
        location: &Location,
    ) -> Result<NodeKey, CheckRefusal> {
        let node = self.binder_ref(value_type, target, location)?;
        require_admitted(node, location)
    }

    /// [`Self::parameter_ref`] over an admitted type node: the admitted key.
    fn parameter(
        &mut self,
        name: &str,
        level: usize,
        semantic_type: NodeKey,
        location: &Location,
    ) -> Result<NodeKey, CheckRefusal> {
        let node = self.parameter_ref(name, level, semantic_type.into(), location)?;
        require_admitted(node, location)
    }

    /// A binder's parameter node: `value`/`parameter`, typed at its binder's
    /// type node `semantic_type`, its body binding its name and level.
    fn parameter_ref(
        &mut self,
        name: &str,
        level: usize,
        semantic_type: NodeRef,
        location: &Location,
    ) -> Result<NodeRef, CheckRefusal> {
        let name_literal = self.text_literal(name, location)?;
        let level_literal = self.integer_literal(Integer::from(level), location)?;
        let key = self.insert_ref(
            location,
            NodeTag::Value,
            "parameter",
            Some(semantic_type.clone()),
            None,
            BodyTerm::aggregate(vec![
                MemberTerm::bound("name", name_literal),
                MemberTerm::bound("level", level_literal),
            ]),
        )?;
        if let Some(protocol) = &self.active_protocol {
            self.protocol_binder_keys
                .entry(protocol.clone())
                .or_default()
                .push(crate::check::protocol_clause::CheckedProtocolBinderKey {
                    name: name.to_owned(),
                    level,
                    introducing: None,
                    node: require_admitted(key.clone(), location)?,
                    semantic_type: require_admitted(semantic_type, location)?,
                });
        }
        self.record(key.clone(), OccurrenceRole::Expression, location.clone());
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
                    *slot = Some(placeholder.clone());
                    self.unsettled_functions.push(*index);
                }
                if let NodeRef::Pending(handle) = placeholder {
                    placeholders.push(handle);
                }
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
        let draft_binders = std::mem::take(&mut self.draft_binders);
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
        let alias = |node: &NodeRef| match node {
            NodeRef::Pending(handle) => {
                aliases.get(handle).cloned().unwrap_or_else(|| node.clone())
            }
            NodeRef::Admitted(_) => node.clone(),
        };
        let handles: Vec<PlaceholderId> = drafts.keys().copied().collect();
        let drafts: Vec<Draft> = drafts.into_values().collect();
        let position: HashMap<PlaceholderId, usize> = handles
            .iter()
            .enumerate()
            .map(|(position, handle)| (*handle, position))
            .collect();
        let mut graph_refusal = None;
        let edges: Vec<Vec<usize>> = drafts
            .iter()
            .enumerate()
            .map(|(at, draft)| {
                if let Err(refusal) = charge_work(self.meter, 1) {
                    graph_refusal = Some(preimage_refusal(&draft.location, refusal));
                    return Vec::new();
                }
                let mut named = Vec::new();
                let mut visit = |node: &NodeRef| {
                    if graph_refusal.is_some() {
                        return;
                    }
                    if let Err(refusal) = charge_work(self.meter, 1) {
                        graph_refusal = Some(preimage_refusal(&drafts[at].location, refusal));
                        return;
                    }
                    if let NodeRef::Pending(handle) = alias(node) {
                        if let Some(target) = position.get(&handle) {
                            named.push(*target);
                        }
                    }
                };
                if let Some(ref semantic_type) = draft.content.semantic_type {
                    visit(semantic_type);
                }
                draft.content.body.for_each_ref(&mut visit);
                named
            })
            .collect();
        if let Some(refusal) = graph_refusal {
            return Err(refusal);
        }
        let mut resolved: HashMap<PlaceholderId, NodeKey> = HashMap::new();
        // A name of a draft that is keyed nowhere: dependency order keys
        // every draft outside a component before the component.
        let unresolved = |location: &Location| fault(location, KeyFault::UnresolvedDraft);
        let components = strongly_connected_metered(&edges, |node| {
            charge_work(self.meter, 1)
                .map_err(|refusal| preimage_refusal(&drafts[node].location, refusal))
        })?;
        for component in components {
            let members: Vec<PlaceholderId> = component.iter().map(|at| handles[*at]).collect();
            let member_set: HashSet<PlaceholderId> = members.iter().copied().collect();
            // Names outside the component take their keys; names inside it
            // stay the members' handles; other keys are not drafts.
            let mut stranded = false;
            let mut substitute = |node: &NodeRef| {
                let node = alias(node);
                match node {
                    NodeRef::Pending(handle) if member_set.contains(&handle) => node,
                    NodeRef::Pending(handle) => {
                        if let Some(resolved) = resolved.get(&handle) {
                            NodeRef::Admitted(resolved.clone())
                        } else {
                            stranded |= position.contains_key(&handle);
                            node
                        }
                    }
                    NodeRef::Admitted(_) => node,
                }
            };
            let mut bodies: Vec<BodyTerm> = Vec::with_capacity(component.len());
            for at in &component {
                let body = drafts[*at]
                    .content
                    .body
                    .map_refs(&mut substitute)
                    .map_err(|refusal| preimage_refusal(&drafts[*at].location, refusal))?;
                bodies.push(body);
            }
            let types: Vec<Option<NodeRef>> = component
                .iter()
                .map(|at| {
                    drafts[*at]
                        .content
                        .semantic_type
                        .as_ref()
                        .map(&mut substitute)
                })
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
                        owner_model: draft.content.owner_model.clone(),
                        node_tag: draft.content.node_tag,
                        semantic_form: draft.content.semantic_form,
                        semantic_type: semantic_type.clone(),
                        declaration: draft.content.declaration.clone(),
                        owner: draft.content.owner.clone(),
                        body,
                    },
                )?;
                let NodeRef::Admitted(key) = key else {
                    return Err(unresolved(&draft.location));
                };
                resolved.insert(handles[*at], key);
            }
        }
        // Every draft is keyed now: a key that still names one is a fault.
        let resolve = |node: &NodeRef| match alias(node) {
            NodeRef::Pending(handle) => match resolved.get(&handle) {
                Some(resolved) => Ok(NodeRef::Admitted(resolved.clone())),
                None => Err(()),
            },
            admitted @ NodeRef::Admitted(_) => Ok(admitted),
        };
        let root = generated_location();
        // A key an earlier settle resolved names no draft or placeholder of
        // this one, so only the keys written since then can change.
        for index in unsettled_functions {
            if let Some(Some(node)) = self.functions.get_mut(index) {
                *node = resolve(node).map_err(|()| unresolved(&root))?;
            }
        }
        for declaration in unsettled_composites {
            if let Some(node) = self.composites.get_mut(&declaration) {
                *node = resolve(node).map_err(|()| unresolved(&root))?;
            }
        }
        for (node, role, location) in occurrences {
            let node = resolve(&node).map_err(|()| unresolved(&location))?;
            self.record(node, role, location);
        }
        for (site, node) in draft_binders {
            let node = resolve(&node).map_err(|()| unresolved(&site.binder))?;
            self.record_binder(site, node);
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
        handles: &[PlaceholderId],
        bodies: &[BodyTerm],
        types: &[Option<NodeRef>],
        resolved: &mut HashMap<PlaceholderId, NodeKey>,
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
        let member_admitted: Vec<NodeKey> = self.admitted_group_keys(&members, &first.location)?;
        let member_keys: BTreeMap<PlaceholderId, NodeKey> = handles
            .iter()
            .zip(&member_admitted)
            .map(|(handle, key)| (*handle, key.clone()))
            .collect();
        let mut to_key = |node: &NodeRef| match node {
            NodeRef::Pending(handle) => member_keys
                .get(handle)
                .cloned()
                .map_or_else(|| node.clone(), NodeRef::Admitted),
            NodeRef::Admitted(_) => node.clone(),
        };
        let group: std::sync::Arc<[NodeKey]> = member_admitted.clone().into();
        for (at, draft) in members.iter().enumerate() {
            let key = member_admitted[at].clone();
            let body = bodies[at]
                .map_refs(&mut to_key)
                .map_err(|refusal| preimage_refusal(&draft.location, refusal))?;
            let semantic_type = types[at].as_ref().map(&mut to_key);
            self.graph
                .nodes
                .entry(key.clone())
                .or_insert_with(|| SemanticNode {
                    key: key.clone(),
                    recursion: Some(NodeRecursion {
                        ordinal: at,
                        size: members.len(),
                        members: group.clone(),
                    }),
                    nominal: None,
                    content: NodeContent {
                        owner_model: draft.content.owner_model.clone(),
                        node_tag: draft.content.node_tag,
                        semantic_form: forms[at],
                        semantic_type,
                        declaration: draft.content.declaration.clone(),
                        owner: draft.content.owner.clone(),
                        body,
                    },
                });
            resolved.insert(handles[at], key);
        }
        Ok(())
    }

    /// Lower and key `function`, whose callees outside its group are
    /// already lowered.
    pub(crate) fn function(
        &mut self,
        index: usize,
        function: &FunctionInput<'_>,
    ) -> Result<NodeRef, CheckRefusal> {
        let mut parameters = Vec::with_capacity(function.parameters.len());
        for (level, (name, value_type)) in function.parameters.iter().enumerate() {
            let target = function.population_targets.get(level).cloned().flatten();
            let type_key = self.binder_ref(value_type, target, function.location)?;
            let key = self.parameter_ref(name, level, type_key.clone(), function.location)?;
            if let Some(span) = function.parameter_spans.and_then(|spans| spans.get(level)) {
                self.parameter_spans.insert(
                    BinderSite {
                        binder: function.location.clone(),
                        slot: level,
                    },
                    *span,
                );
            }
            self.record(
                type_key.clone(),
                OccurrenceRole::Type,
                function.location.clone(),
            );
            self.record_binder(
                BinderSite {
                    binder: function.location.clone(),
                    slot: level,
                },
                key.clone(),
            );
            parameters.push(Binder {
                slot: level,
                parameter: key,
                semantic_type: type_key,
            });
        }
        let result = self.type_ref(function.result, function.location)?;
        self.record(
            result.clone(),
            OccurrenceRole::Type,
            function.location.clone(),
        );
        let mut members = vec![MemberTerm::binding(
            FUNCTION_PARAMETERS,
            BindingValue::Group(GroupTerm::references_nodes(
                parameters.iter().map(|binder| binder.parameter.clone()),
            )),
        )];
        let mut binders = Binders::new(function.body_slots, &parameters);
        let body = self.expression(function.body, &mut binders)?;
        members.push(MemberTerm::bound("body", body));
        if let Some(measure) = function.measure {
            let mut binders = Binders::new(function.measure_slots, &parameters);
            let measure = self.expression(measure, &mut binders)?;
            members.push(MemberTerm::bound("decreases", measure));
        }
        let semantic_form = match function.kind {
            qsl_forms::DeclarationKind::Predicate => "predicate",
            qsl_forms::DeclarationKind::Function => "pure_function",
        };
        let key = match function.clause {
            // FR-094: a clause function carries its declaration's
            // `ModelOwner`, no `declaration` and a trailing `clause`
            // binding; it has no source `declaration` occurrence. Its one
            // occurrence is `generated`, at its own body, which FR-096
            // resolves to the clause or body the function was built from
            // (FR-302), so emission places it.
            Some(clause) => {
                let (owner, clause) = self.clause_owner(clause, function.location)?;
                members.push(clause);
                let key = self.insert_owned_ref(
                    function.location,
                    NodeTag::Function,
                    semantic_form,
                    Some(result),
                    owner,
                    BodyTerm::aggregate(members),
                )?;
                self.record(
                    key.clone(),
                    OccurrenceRole::Generated,
                    function.location.clone(),
                );
                key
            }
            None => {
                let declaration = qualified_name(function.name, function.location)?;
                let key = self.insert_ref(
                    function.location,
                    NodeTag::Function,
                    semantic_form,
                    Some(result),
                    Some(declaration),
                    BodyTerm::aggregate(members),
                )?;
                self.record(
                    key.clone(),
                    OccurrenceRole::Declaration,
                    function.location.clone(),
                );
                key
            }
        };
        if let Some(slot) = self.functions.get_mut(index) {
            *slot = Some(key.clone());
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
        semantic_type: impl Into<NodeRef>,
        location: &Location,
    ) -> Result<String, CheckRefusal> {
        let semantic_type = semantic_type.into();
        let name = binders.slot_names.get(slot).cloned().ok_or_else(|| {
            refuse(
                location,
                CheckCause::NodePreimage(NodeKeyRefusal::EmptyBindingName),
            )
        })?;
        let level = binders.scope.len();
        let parameter = self.parameter_ref(&name, level, semantic_type.clone(), location)?;
        self.record_binder(
            BinderSite {
                binder: location.clone(),
                slot,
            },
            parameter.clone(),
        );
        binders.push(Binder {
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
        node: CheckedNode<'_>,
        binders: &Binders<'_>,
    ) -> Result<NodeRef, CheckRefusal> {
        if let NodeKind::Local(slot) = node.kind() {
            if let Some(binder) = binders.find(*slot) {
                return Ok(binder.semantic_type.clone());
            }
        }
        self.type_ref(node.value_type(), node.location())
    }

    /// Key an application node for checked `node` and return a reference to
    /// it.
    fn application(
        &mut self,
        node: CheckedNode<'_>,
        operator: Operator,
        operation: Operation,
        arguments: Vec<MemberTerm>,
    ) -> Result<LeafTerm, CheckRefusal> {
        let result_type = self.type_ref(node.value_type(), node.location())?;
        self.typed_application(node.location(), result_type, operator, operation, arguments)
    }

    /// Key an application node typed at `result_type`, denoted by the
    /// region `location`, and return a reference to it.
    fn typed_application(
        &mut self,
        location: &Location,
        result_type: impl Into<NodeRef>,
        operator: Operator,
        operation: Operation,
        arguments: Vec<MemberTerm>,
    ) -> Result<LeafTerm, CheckRefusal> {
        let result_type = result_type.into();
        let key = self.insert_ref(
            location,
            NodeTag::Expression,
            operator.semantic_form(),
            Some(result_type.clone()),
            None,
            BodyTerm::application_node(operator, operation, result_type, arguments),
        )?;
        self.record(key.clone(), OccurrenceRole::Expression, location.clone());
        Ok(LeafTerm::reference_node(key.into()))
    }

    /// Key a `value` node for checked `node` and return a reference to it.
    fn value_node(
        &mut self,
        node: CheckedNode<'_>,
        form: &'static str,
        semantic_type: NodeRef,
        body: BodyTerm,
    ) -> Result<LeafTerm, CheckRefusal> {
        let key = self.insert_ref(
            node.location(),
            NodeTag::Value,
            form,
            Some(semantic_type),
            None,
            body,
        )?;
        self.record(
            key.clone(),
            OccurrenceRole::Expression,
            node.location().clone(),
        );
        Ok(LeafTerm::reference_node(key.into()))
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
    /// kept on an explicit heap stack, so a body of any depth lowers in the
    /// same host stack as a flat one. Each frame waits on a node typing
    /// already charged against the node ceiling. Nodes are
    /// built, charged and refused in the order the earlier recursive
    /// lowering reached them -- each node's own type and member nodes before
    /// its operands, its operands in order, then its own node -- so every
    /// key is that lowering's. On a refusal the binders the refused
    /// expression bound are no longer in scope.
    fn expression(
        &mut self,
        root: CheckedNode<'_>,
        binders: &mut Binders<'_>,
    ) -> Result<LeafTerm, CheckRefusal> {
        if let Some(protocol) = &self.active_protocol {
            let expression_binders = binders
                .scope
                .iter()
                .filter_map(|binder| {
                    Some(crate::check::protocol_clause::CheckedProtocolBinderKey {
                        name: binders.slot_names.get(binder.slot)?.clone(),
                        level: binder.slot,
                        introducing: None,
                        node: binder.parameter.admitted()?.clone(),
                        semantic_type: binder.semantic_type.admitted()?.clone(),
                    })
                })
                .collect();
            self.protocol_expression_binders
                .entry(protocol.clone())
                .or_default()
                .push(
                    crate::check::protocol_clause::CheckedProtocolExpressionBindings {
                        location: root.location().clone(),
                        binders: expression_binders,
                    },
                );
        }
        let scope = binders.scope.len();
        let lowered = self.lower(root, binders);
        if lowered.is_err() {
            binders.truncate(scope);
        }
        lowered
    }

    /// [`Self::expression`]'s loop: lower a node, or hand a lowered node's
    /// term to the frame waiting for it.
    fn lower(
        &mut self,
        root: CheckedNode<'_>,
        binders: &mut Binders<'_>,
    ) -> Result<LeafTerm, CheckRefusal> {
        let mut frames: Vec<LowerFrame<'_>> = Vec::new();
        let mut step = LowerStep::Descend(root);
        loop {
            step = match step {
                LowerStep::Descend(node) => self.lower_node(node, binders, &mut frames)?,
                LowerStep::Lowered(term) => match frames.pop() {
                    None => return Ok(term),
                    Some(frame) => self.accept_term(frame, term, binders, &mut frames)?,
                },
            };
        }
    }

    /// Start lowering checked `node`: a literal or local read is
    /// lowered now; a node the FR-093 table builds no node for lowers its
    /// operand in its place; any other node builds its own type and member
    /// nodes, pushes its frame and descends into its first operand.
    #[deny(clippy::wildcard_enum_match_arm)]
    fn lower_node<'n>(
        &mut self,
        node: CheckedNode<'n>,
        binders: &Binders<'_>,
        frames: &mut Vec<LowerFrame<'n>>,
    ) -> Result<LowerStep<'n>, CheckRefusal> {
        let plain = Operation::plain;
        let (keyed, operands) = match node.kind() {
            NodeKind::Literal(CheckedLiteral(value)) => {
                return self.literal(node, value).map(LowerStep::Lowered)
            }
            NodeKind::Local(slot) => {
                let key = binders
                    .find(*slot)
                    .map(|binder| binder.parameter.clone())
                    .ok_or_else(|| {
                        refuse(
                            node.location(),
                            CheckCause::NodePreimage(NodeKeyRefusal::EmptyBindingName),
                        )
                    })?;
                self.record(
                    key.clone(),
                    OccurrenceRole::Expression,
                    node.location().clone(),
                );
                return Ok(LowerStep::Lowered(LeafTerm::reference_node(key)));
            }
            // FR-391 "Lowering": the population operand of `allInstances` or
            // `lookup` references the population's `relation`/`population`
            // node (FR-322 "Population references").
            NodeKind::NamedPopulation { population, .. } => {
                let key = self.population_reference_node(population, node.location())?;
                return Ok(LowerStep::Lowered(LeafTerm::reference_node(key.into())));
            }
            NodeKind::Coerce(operand, interval) => {
                if !super::ir::coerce_builds_narrow(node.at(*operand).value_type(), interval) {
                    return Ok(LowerStep::Descend(node.at(*operand)));
                }
                let member =
                    self.type_argument(&ValueType::Int(interval.clone()), node.location())?;
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
                    slot: *slot,
                    value: node.at(*value),
                    body: node.at(*body),
                    bound: None,
                })));
                return Ok(LowerStep::Descend(node.at(*value)));
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
            NodeKind::IntegerDivision {
                operator,
                left,
                right,
                ..
            } => {
                let (identity, laws) = match operator {
                    IntegerDivisionOperator::Div => (
                        "quire.op.integer.div",
                        vec![self.law(LawRole::IntegerDivision, node.location())?],
                    ),
                    IntegerDivisionOperator::Rem => (
                        "quire.op.integer.rem",
                        vec![self.law(LawRole::IntegerDivision, node.location())?],
                    ),
                    IntegerDivisionOperator::Mod => ("quire.op.integer.mod", Vec::new()),
                };
                (
                    Keyed::Application(
                        Operator::Binary,
                        Operation {
                            laws,
                            ..plain(identity)
                        },
                    ),
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
            NodeKind::Ieee(operator, rounding, left, right) => {
                let ValueType::Float(float) = node.at(*left).value_type() else {
                    return Err(fault(node.location(), KeyFault::UntypedIeeeOperand));
                };
                let width = match float.width() {
                    quire_exact::IeeeWidth::Binary32 => "float32",
                    quire_exact::IeeeWidth::Binary64 => "float64",
                };
                let law = self.law(LawRole::IeeeProfile, node.location())?;
                let identity = format!("quire.op.ieee.{width}.{}", arithmetic_suffix(*operator));
                (
                    Keyed::Application(
                        Operator::Binary,
                        Operation {
                            laws: vec![law],
                            mode: Some(OperationMode::Rounding(*rounding)),
                            ..plain(&identity)
                        },
                    ),
                    Operands::Two(left, right),
                )
            }
            NodeKind::IeeeSquareRoot(rounding, operand) => {
                let ValueType::Float(float) = node.at(*operand).value_type() else {
                    return Err(fault(node.location(), KeyFault::UntypedIeeeOperand));
                };
                let width = match float.width() {
                    quire_exact::IeeeWidth::Binary32 => "float32",
                    quire_exact::IeeeWidth::Binary64 => "float64",
                };
                let law = self.law(LawRole::IeeeProfile, node.location())?;
                let identity = format!("quire.op.ieee.{width}.sqrt");
                (
                    Keyed::Application(
                        Operator::Unary,
                        Operation {
                            laws: vec![law],
                            mode: Some(OperationMode::Rounding(*rounding)),
                            ..plain(&identity)
                        },
                    ),
                    Operands::One(operand),
                )
            }
            NodeKind::ConvertDecimal(operand, target) => {
                // FR-149 scale reduction: a decimal of larger maximum scale,
                // or a rational whose denominators exceed 1.
                let reduces = if let ValueType::Decimal(source) = node.at(*operand).value_type() {
                    source.max_scale() > target.max_scale()
                } else if let ValueType::Rational(domain) = node.at(*operand).value_type() {
                    *domain.denominator().upper() > Integer::one()
                } else {
                    false
                };
                let member =
                    self.type_argument(&ValueType::Decimal(target.clone()), node.location())?;
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
                    operation.laws = vec![self.law(LawRole::TextProfile, node.location())?];
                    operation.mode =
                        text_profile(node.at(*left).value_type()).map(OperationMode::TextProfile);
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
                let operation =
                    self.equality(node.at(*left).value_type(), suffix, node.location())?;
                (
                    Keyed::Application(Operator::Binary, operation),
                    Operands::Two(left, right),
                )
            }
            NodeKind::IeeeNumericEquality(left, right) => (
                Keyed::Application(
                    Operator::Binary,
                    Operation {
                        laws: vec![self.law(LawRole::IeeeProfile, node.location())?],
                        ..plain("quire.op.ieee.numeric_equal")
                    },
                ),
                Operands::Two(left, right),
            ),
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
            NodeKind::Field { operand, field, .. } => {
                let member = self.field_member(field, node.location())?;
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
                reference, field, ..
            } => {
                // FR-093/FR-094 (QC-24): `quire.op.model.deref` over the
                // reference, typed at its object type `T`'s model node, and
                // over it the `record.project` of field `name` of `T`.
                let object =
                    self.referenced_object(node.at(*reference).value_type(), node.location())?;
                frames.push(LowerFrame::Attribute {
                    node,
                    object,
                    name: &field.name,
                });
                return Ok(LowerStep::Descend(node.at(*reference)));
            }
            NodeKind::Navigate {
                reference,
                relationship,
                role,
                ..
            } => {
                // FR-093/FR-094 (FR-363): one `quire.op.model.navigate` over
                // the reference, naming the end by the relationship's model
                // node and the role. No `deref` node: the reference is the
                // argument.
                let declaration = self.model_node(relationship, node.location())?;
                let name = Identifier::new(role.clone()).map_err(|_| {
                    refuse(
                        node.location(),
                        CheckCause::NodePreimage(NodeKeyRefusal::EmptyBindingName),
                    )
                })?;
                (
                    Keyed::Application(
                        Operator::Query,
                        Operation {
                            member: Some(Member::RelationshipEnd {
                                declaration,
                                name,
                                population: None,
                            }),
                            ..plain("quire.op.model.navigate")
                        },
                    ),
                    Operands::One(reference),
                )
            }
            // FR-230 "Presence is derived", QSpec FR-322 "Presence":
            // `present(r.f)` of a clock place is the `presence_read`
            // application over the place's `clock_place` template and the
            // owner term `r`.
            NodeKind::PlacePresence {
                reference,
                field,
                case,
            } => {
                let mut path = vec![PlaceStep::Field(field.name.clone())];
                path.extend(case.clone().map(PlaceStep::Union));
                let place = self
                    .timed_place(field.owner.clone(), &path)
                    .ok_or_else(|| {
                        refuse(
                            node.location(),
                            CheckCause::UnsupportedFeature {
                                loci: vec![node.location().clone()],
                            },
                        )
                    })?;
                return self.next_operand(
                    Box::new(OperandsFrame {
                        node,
                        operands: Operands::One(reference),
                        next: 0,
                        terms: vec![LeafTerm::reference_node(place.into())],
                        keyed: Keyed::Application(
                            Operator::Present,
                            plain("quire.op.option.present"),
                        ),
                    }),
                    frames,
                );
            }
            // FR-139, FR-138: a population-valued form and a history read
            // live in a refinement row, which S4 does not emit yet
            // (FR-310's node): no FR-093 row lowers either. A quantifier
            // over a clock collection's slots has no checked form until
            // STD-554, so S3 keeps `forall(e in r.q: present(e))` out of
            // every lowered clause body.
            NodeKind::PopulationObjects { .. }
            | NodeKind::History { .. }
            | NodeKind::SlotPresence { .. }
            | NodeKind::Seq { .. } => {
                return Err(refuse(
                    node.location(),
                    CheckCause::UnsupportedFeature {
                        loci: vec![node.location().clone()],
                    },
                ))
            }
            NodeKind::Present(operand) => (
                Keyed::Application(Operator::Present, plain("quire.op.option.present")),
                Operands::One(operand),
            ),
            NodeKind::Value(operand) => (
                Keyed::Application(Operator::Value, plain("quire.op.option.value")),
                Operands::One(operand),
            ),
            NodeKind::OptionSome(operand) => {
                let semantic_type = self.value_type_node(node, binders)?;
                (Keyed::OptionValue(semantic_type), Operands::One(operand))
            }
            NodeKind::Call {
                function,
                arguments,
            } => {
                let callee = self
                    .functions
                    .get(*function)
                    .cloned()
                    .flatten()
                    .ok_or_else(|| {
                        refuse(
                            node.location(),
                            CheckCause::UnsupportedFeature {
                                loci: vec![node.location().clone()],
                            },
                        )
                    })?;
                let mut terms = Vec::with_capacity(arguments.len() + 1);
                terms.push(LeafTerm::reference_node(callee.into()));
                return self.next_operand(
                    Box::new(OperandsFrame {
                        node,
                        operands: Operands::Each(arguments),
                        next: 0,
                        terms,
                        keyed: Keyed::Application(Operator::Call, plain("quire.op.function.call")),
                    }),
                    frames,
                );
            }
            // ADR-015 D-5: an imported call is a `quire.op.function.call`
            // application whose callee is the `dependency_reference` term.
            NodeKind::ImportedCall {
                callee, arguments, ..
            } => {
                let mut terms = Vec::with_capacity(arguments.len() + 1);
                terms.push(LeafTerm::DependencyReference {
                    node: NodeRef::Admitted(callee.clone()),
                });
                return self.next_operand(
                    Box::new(OperandsFrame {
                        node,
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
                let semantic_type =
                    self.composite_type_node(declaration.clone(), node.location())?;
                (
                    Keyed::Value("tuple_value", semantic_type),
                    Operands::Each(arguments),
                )
            }
            NodeKind::Record { declaration, slots } => {
                let semantic_type =
                    self.composite_type_node(declaration.clone(), node.location())?;
                let Some(CompositeShape::Record(fields)) = self
                    .scope
                    .types()
                    .composite(declaration.clone())
                    .map(|c| c.shape().clone())
                else {
                    return Err(refuse(
                        node.location(),
                        CheckCause::IllTyped(quire_exact::IllTypedCause::TypeMismatch),
                    ));
                };
                if fields.len() != slots.len() {
                    return Err(fault(node.location(), KeyFault::RecordSlotCount));
                }
                let members = Vec::with_capacity(slots.len());
                return self.record_members(
                    Box::new(RecordFrame {
                        node,
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
                    self.leaves(LeafSource::ResultInner(node.value_type()), node.location())?;
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
                    self.type_argument(&ValueType::collection(target.clone()), node.location())?;
                let leaves =
                    self.leaves(LeafSource::ResultInner(node.value_type()), node.location())?;
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
                let Some(target) =
                    super::ir::scalar_conversion_target(target, node.at(*operand).value_type())
                else {
                    return Ok(LowerStep::Descend(node.at(*operand)));
                };
                let target = target.clone();
                let member = self.type_argument(&target, node.location())?;
                let operation = if let ValueType::Quantity(_) = node.at(*operand).value_type() {
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
                let law = self.law(LawRole::IeeeProfile, node.location())?;
                let member =
                    self.type_argument(&ValueType::Rational(domain.clone()), node.location())?;
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
                                LeafSource::ResultInner(node.value_type()),
                                node.location(),
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
                            member: Some(self.type_argument(node.value_type(), node.location())?),
                            ..plain("quire.op.collection.count")
                        },
                    ),
                    Visit::Only => {
                        return Err(refuse(
                            node.location(),
                            CheckCause::UnsupportedFeature {
                                loci: vec![node.location().clone()],
                            },
                        ))
                    }
                    Visit::Sum => (
                        Operator::Collection,
                        Operation {
                            member: Some(self.type_argument(node.value_type(), node.location())?),
                            ..plain("quire.op.collection.sum.integer")
                        },
                    ),
                };
                frames.push(LowerFrame::Binder(Box::new(BinderFrame {
                    node,
                    slot: *slot,
                    source: node.at(*source),
                    body: node.at(*body),
                    operator,
                    operation,
                    bound: None,
                })));
                return Ok(LowerStep::Descend(node.at(*source)));
            }
            NodeKind::Flatten(operand) => {
                let leaves =
                    self.leaves(LeafSource::ResultInner(node.value_type()), node.location())?;
                if let NodeKind::Query {
                    visit: Visit::Map,
                    slot,
                    source,
                    body,
                } = node.at(*operand).kind()
                {
                    // FR-093: `flatMap`, and `flatten(map(..))`, is one
                    // `flat_map` node; no node is built for the inner map.
                    frames.push(LowerFrame::Binder(Box::new(BinderFrame {
                        node,
                        slot: *slot,
                        source: node.at(*source),
                        body: node.at(*body),
                        operator: Operator::Collection,
                        operation: Operation {
                            leaves,
                            ..plain("quire.op.collection.flat_map")
                        },
                        bound: None,
                    })));
                    return Ok(LowerStep::Descend(node.at(*source)));
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
                    accumulator: *accumulator,
                    binder: *binder,
                    source: node.at(*source),
                    step: node.at(*step),
                    identity: identity.map(|identity| node.at(identity)),
                    stage: FoldStage::Source,
                })));
                return Ok(LowerStep::Descend(node.at(*source)));
            }
            NodeKind::Size(operand) => {
                let member = self.type_argument(node.value_type(), node.location())?;
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
                let leaves = if let ValueType::Collection(collection_type) =
                    node.at(*collection).value_type()
                {
                    self.leaves(
                        LeafSource::Compared(collection_type.element()),
                        node.location(),
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
                let object = self.referenced_object(node.value_type(), node.location())?;
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
                let object = self.referenced_object(node.value_type(), node.location())?;
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
            NodeKind::Reaches {
                source,
                target,
                edge,
            } => {
                // FR-105 (pending STD-111's spelling): the edge is named as
                // an attribute read names its field, by the operands'
                // static object type's model node and the field's name.
                let object =
                    self.referenced_object(node.at(*source).value_type(), node.location())?;
                let name = Identifier::new(edge.name.clone()).map_err(|_| {
                    refuse(
                        node.location(),
                        CheckCause::NodePreimage(NodeKeyRefusal::EmptyBindingName),
                    )
                })?;
                (
                    Keyed::Application(
                        Operator::Reaches,
                        Operation {
                            member: Some(Member::Field {
                                declaration: object,
                                name,
                            }),
                            ..plain("quire.op.model.reaches_field")
                        },
                    ),
                    Operands::Two(source, target),
                )
            }
            NodeKind::ReachesEnd {
                source,
                target,
                relationship,
                role,
                near,
                population,
            } => {
                // FR-365/FR-105: one `quire.op.model.reaches` over both
                // references, naming the end by the relationship's model
                // node and the role. No `deref` node: the references are
                // the arguments.
                let declaration = self.model_node(relationship, node.location())?;
                let name = Identifier::new(role.clone()).map_err(|_| {
                    refuse(
                        node.location(),
                        CheckCause::NodePreimage(NodeKeyRefusal::EmptyBindingName),
                    )
                })?;
                // FR-365 "The population link": only a walked population
                // with an authored bound names its node.
                let population = match population {
                    None => None,
                    Some(bound) => {
                        let near_object = self
                            .scope
                            .relationships()
                            .end_object(relationship, *near)
                            .ok_or_else(|| {
                                refuse(
                                    node.location(),
                                    CheckCause::IllTyped(quire_exact::IllTypedCause::TypeMismatch),
                                )
                            })?;
                        Some(self.bounded_population(bound, near_object, node.location())?)
                    }
                };
                (
                    Keyed::Application(
                        Operator::Reaches,
                        Operation {
                            member: Some(Member::RelationshipEnd {
                                declaration,
                                name,
                                population,
                            }),
                            ..plain("quire.op.model.reaches")
                        },
                    ),
                    Operands::Two(source, target),
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
                let object =
                    self.referenced_object(node.at(*receiver).value_type(), node.location())?;
                let member = self
                    .scope
                    .dispatch_operations
                    .get(*operation)
                    .map(|operation| operation.member.clone())
                    .ok_or_else(|| {
                        refuse(
                            node.location(),
                            CheckCause::IllTyped(quire_exact::IllTypedCause::TypeMismatch),
                        )
                    })?;
                let name = Identifier::new(member).map_err(|_| {
                    refuse(
                        node.location(),
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
            // FR-319, FR-320: a construction is a `union_value` node over the
            // union's type node.
            NodeKind::UnionValue {
                declaration,
                member,
                arguments,
            } => {
                let semantic_type =
                    self.composite_type_node(declaration.clone(), node.location())?;
                let member =
                    self.union_member_name(declaration.clone(), *member, node.location())?;
                (
                    Keyed::UnionValue {
                        semantic_type,
                        member: member.to_owned(),
                    },
                    Operands::Each(arguments),
                )
            }
            // FR-319, FR-320: a `case` is an `expression`/`case` node whose
            // arms are in the union's declared member order.
            NodeKind::Case {
                scrutinee,
                declaration,
                arms,
            } => {
                // The union's type node, which the arm binders' types name.
                self.composite_type_node(declaration.clone(), node.location())?;
                frames.push(LowerFrame::Case(Box::new(CaseFrame {
                    node,
                    declaration: declaration.clone(),
                    arms,
                    scrutinee: None,
                    lowered: Vec::with_capacity(arms.len()),
                    open: None,
                })));
                return Ok(LowerStep::Descend(node.at(*scrutinee)));
            }
            NodeKind::Pre(operand) => (
                Keyed::Application(Operator::Pre, plain("quire.op.state.pre")),
                Operands::One(operand),
            ),
            NodeKind::ProtocolReference { protocol } => {
                return self
                    .protocol_reference(node, *protocol)
                    .map(LowerStep::Lowered)
            }
            // FR-063/S3 seam: no arm for `NodeKind::__SeamProbe`
            // under `--cfg seam_probe` alone -- this match (already
            // `#[deny(clippy::wildcard_enum_match_arm)]`, this function's own
            // attribute above) is deliberately non-exhaustive (`E0004`) in
            // `xtask seam-probe`'s build of `qsl-semantics`, the seam
            // probe's evidence for the FR-093 identity-lowering pass that
            // feeds v2 emission (ADR-012 §5.1 row S3, "v2 emitter"). Do not
            // add a catch-all to make it compile.
            //
            // The arm below exists only in the probe's build of the crates
            // that depend on this one (`--cfg seam_probe --cfg
            // seam_probe_downstream`), the same shape `FamilyKind::
            // catalog_code_prefix`'s own doc explains.
            #[cfg(seam_probe_downstream)]
            NodeKind::__SeamProbe => unreachable!("never constructed outside the probe build"),
        };
        self.next_operand(
            Box::new(OperandsFrame {
                node,
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
            let operand = frame.node.at(operand);
            // FR-358, ADR-013 O-09: an integer literal operand of an FR-357
            // scalar operator or of an integer `eq`/`ne` is a literal term in
            // the application's argument, never a node of its own.
            if let Keyed::Application(_, operation) = &frame.keyed {
                if takes_inline_integer_literals(&operation.identity) {
                    if let NodeKind::Literal(CheckedLiteral(Value::Integer(value))) = operand.kind()
                    {
                        let term = self.integer_literal(value.clone(), operand.location())?;
                        frame.terms.push(term);
                        return self.next_operand(frame, frames);
                    }
                }
            }
            frames.push(LowerFrame::Operands(frame));
            return Ok(LowerStep::Descend(operand));
        }
        let OperandsFrame {
            node, terms, keyed, ..
        } = *frame;
        match keyed {
            Keyed::Application(operator, operation) => self.application(
                node,
                operator,
                operation,
                terms.into_iter().map(MemberTerm::Leaf).collect(),
            ),
            Keyed::Value(form, semantic_type) => self.value_node(
                node,
                form,
                semantic_type,
                BodyTerm::aggregate(terms.into_iter().map(MemberTerm::Leaf).collect()),
            ),
            Keyed::UnionValue {
                semantic_type,
                member,
            } => {
                let payload = GroupTerm::new(terms.into_iter().map(GroupMember::Leaf).collect());
                self.value_node(
                    node,
                    "union_value",
                    semantic_type,
                    BodyTerm::aggregate(vec![MemberTerm::binding(
                        member,
                        BindingValue::Group(payload),
                    )]),
                )
            }
            Keyed::OptionValue(semantic_type) => self.value_node(
                node,
                "option_value",
                semantic_type,
                BodyTerm::aggregate(vec![MemberTerm::binding(
                    "value",
                    BindingValue::Leaf(
                        terms
                            .into_iter()
                            .next()
                            .ok_or_else(|| fault(node.location(), KeyFault::RecordSlotCount))?,
                    ),
                )]),
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
                    let value = frame.node.at(*value);
                    frames.push(LowerFrame::Record(frame));
                    return Ok(LowerStep::Descend(value));
                }
                RecordSlot::Null | RecordSlot::Absent => {
                    let option = ValueType::option(field.value_type().clone());
                    let option = self.type_ref(&option, frame.node.location())?;
                    let member = MemberTerm::bound(
                        field.name(),
                        LeafTerm::literal_node(option.into(), LiteralValue::None),
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
            BodyTerm::aggregate(members),
        )
        .map(LowerStep::Lowered)
    }

    /// Hand the lowered `term` to `frame`, which descends into its next
    /// operand or keys its own node.
    fn accept_term<'n>(
        &mut self,
        frame: LowerFrame<'n>,
        term: LeafTerm,
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
                    let name = self.bind(binders, frame.slot, value_type, frame.node.location())?;
                    frame.bound = Some((term, name));
                    let body = frame.body;
                    frames.push(LowerFrame::Let(frame));
                    Ok(LowerStep::Descend(body))
                }
                Some((value, name)) => {
                    binders.pop();
                    self.application(
                        frame.node,
                        Operator::Let,
                        plain("quire.op.control.let"),
                        vec![MemberTerm::bound(name, value), MemberTerm::Leaf(term)],
                    )
                    .map(LowerStep::Lowered)
                }
            },
            LowerFrame::Binder(mut frame) => match frame.bound.take() {
                None => {
                    let location = frame.node.location();
                    let element = element_type(frame.source.value_type(), location)?;
                    let element = self.type_ref(&element, location)?;
                    let name = self.bind(binders, frame.slot, element, location)?;
                    frame.bound = Some((term, name));
                    let body = frame.body;
                    frames.push(LowerFrame::Binder(frame));
                    Ok(LowerStep::Descend(body))
                }
                Some((source, name)) => {
                    binders.pop();
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
                        vec![MemberTerm::Leaf(source), MemberTerm::bound(name, term)],
                    )
                    .map(LowerStep::Lowered)
                }
            },
            LowerFrame::Attribute { node, object, name } => {
                let dereferenced = self.typed_application(
                    node.location(),
                    object.clone(),
                    Operator::Deref,
                    plain("quire.op.model.deref"),
                    vec![MemberTerm::Leaf(term)],
                )?;
                let name = Identifier::new(name).map_err(|_| {
                    refuse(
                        node.location(),
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
                    vec![MemberTerm::Leaf(dereferenced)],
                )
                .map(LowerStep::Lowered)
            }
            LowerFrame::Record(mut frame) => {
                if let Some(field) = frame.fields.get(frame.members.len()) {
                    let member = MemberTerm::bound(field.name(), term);
                    frame.members.push(member);
                }
                self.record_members(frame, frames)
            }
            LowerFrame::Fold(frame) => self.accept_fold(frame, term, binders, frames),
            LowerFrame::Case(frame) => self.accept_case(frame, term, binders, frames),
        }
    }

    /// The declared members of the union `declaration`.
    fn union_members(
        &self,
        declaration: NodeKey,
        location: &Location,
    ) -> Result<&'a [UnionMemberDeclaration], CheckRefusal> {
        let scope: &'a Scope = self.scope;
        match scope.types().composite(declaration).map(|c| c.shape()) {
            Some(CompositeShape::Union(members)) => Ok(members),
            Some(CompositeShape::Record(_) | CompositeShape::Tuple(_)) | None => {
                Err(union_fault(location, "union-declaration"))
            }
        }
    }

    /// The declared member at position `member` of the union `declaration`.
    fn union_member(
        &self,
        declaration: NodeKey,
        member: usize,
        location: &Location,
    ) -> Result<&'a UnionMemberDeclaration, CheckRefusal> {
        self.union_members(declaration, location)?
            .get(member)
            .ok_or_else(|| union_fault(location, "union-member-position"))
    }

    /// The name of the member at position `member` of the union
    /// `declaration`.
    fn union_member_name(
        &self,
        declaration: NodeKey,
        member: usize,
        location: &Location,
    ) -> Result<&'a str, CheckRefusal> {
        Ok(self
            .union_member(declaration, member, location)?
            .identifier())
    }

    /// Hand a `case` its lowered scrutinee, or the lowered body of its open
    /// arm.
    fn accept_case<'n>(
        &mut self,
        mut frame: Box<CaseFrame<'n>>,
        term: LeafTerm,
        binders: &mut Binders<'_>,
        frames: &mut Vec<LowerFrame<'n>>,
    ) -> Result<LowerStep<'n>, CheckRefusal> {
        let location = frame.node.location();
        match frame.open.take() {
            None if frame.scrutinee.is_none() => frame.scrutinee = Some(term),
            None => return Err(union_fault(location, "case-arm-body-unexpected")),
            Some(open) => {
                let at = frame.lowered.len();
                let arm = frame
                    .arms
                    .get(at)
                    .ok_or_else(|| union_fault(location, "case-arm-position"))?;
                for _ in &arm.binders {
                    binders.pop();
                }
                let name =
                    self.union_member_name(frame.declaration.clone(), arm.member, location)?;
                let parameters = GroupTerm::references_nodes(open.parameters);
                frame.lowered.push(MemberTerm::binding(
                    name,
                    BindingValue::Tuple(TupleTerm::new(vec![
                        TupleMember::Group(parameters),
                        TupleMember::Leaf(term),
                    ])),
                ))
            }
        }
        self.open_arm(frame, binders, frames)
    }

    /// Bind the next arm's payload binders and descend into its body, or key
    /// the `case` node once every arm is lowered.
    fn open_arm<'n>(
        &mut self,
        mut frame: Box<CaseFrame<'n>>,
        binders: &mut Binders<'_>,
        frames: &mut Vec<LowerFrame<'n>>,
    ) -> Result<LowerStep<'n>, CheckRefusal> {
        let node = frame.node;
        let location = node.location();
        let Some(arm) = frame.arms.get(frame.lowered.len()) else {
            let CaseFrame {
                scrutinee, lowered, ..
            } = *frame;
            let scrutinee = scrutinee.ok_or_else(|| union_fault(location, "case-scrutinee"))?;
            let mut arguments = Vec::with_capacity(lowered.len() + 1);
            arguments.push(MemberTerm::Leaf(scrutinee));
            arguments.extend(lowered);
            return self
                .application(
                    node,
                    Operator::Case,
                    Operation::plain("quire.op.control.case"),
                    arguments,
                )
                .map(LowerStep::Lowered);
        };
        let member = self.union_member(frame.declaration.clone(), arm.member, location)?;
        let payload = member.positions();
        if payload.len() != arm.binders.len() {
            return Err(union_fault(location, "case-arm-binders"));
        }
        let mut parameters = Vec::with_capacity(payload.len());
        for (slot, value_type) in arm.binders.iter().zip(payload) {
            let semantic_type = self.type_ref(value_type, location)?;
            self.bind(binders, *slot, semantic_type, location)?;
            let parameter = binders
                .find(*slot)
                .map(|binder| binder.parameter.clone())
                .ok_or_else(|| union_fault(location, "case-arm-binder"))?;
            parameters.push(parameter);
        }
        frame.open = Some(OpenArm { parameters });
        let body = node.at(arm.body);
        frames.push(LowerFrame::Case(frame));
        Ok(LowerStep::Descend(body))
    }

    /// Hand a `fold`/`reduce` its lowered source, identity or step.
    fn accept_fold<'n>(
        &mut self,
        mut frame: Box<FoldFrame<'n>>,
        term: LeafTerm,
        binders: &mut Binders<'_>,
        frames: &mut Vec<LowerFrame<'n>>,
    ) -> Result<LowerStep<'n>, CheckRefusal> {
        let (source, identity) = match std::mem::replace(&mut frame.stage, FoldStage::Source) {
            FoldStage::Source => match frame.identity {
                Some(identity) => {
                    frame.stage = FoldStage::Identity(term);
                    frames.push(LowerFrame::Fold(frame));
                    return Ok(LowerStep::Descend(identity));
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
                binders.pop();
                binders.pop();
                let node = frame.node;
                let member = self.type_argument(node.value_type(), node.location())?;
                let mut arguments = vec![
                    MemberTerm::Leaf(source),
                    MemberTerm::binding(accumulator, BindingValue::named_leaf(binder, term)),
                ];
                let identity_name = match identity {
                    Some(identity) => {
                        arguments.push(MemberTerm::Leaf(identity));
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
        let element = element_type(frame.source.value_type(), node.location())?;
        let element = self.type_ref(&element, node.location())?;
        let accumulator_type = self.type_ref(node.value_type(), node.location())?;
        let accumulator = self.bind(
            binders,
            frame.accumulator,
            accumulator_type,
            node.location(),
        )?;
        let binder = self.bind(binders, frame.binder, element, node.location())?;
        frame.stage = FoldStage::Step {
            source,
            identity,
            accumulator,
            binder,
        };
        let step = frame.step;
        frames.push(LowerFrame::Fold(frame));
        Ok(LowerStep::Descend(step))
    }

    /// A literal's node: `value`/`literal` for a Boolean, integer, rational,
    /// quantity or absent option, the enum member's nominal node for an
    /// enum member.
    fn literal(&mut self, node: CheckedNode<'_>, value: &Value) -> Result<LeafTerm, CheckRefusal> {
        let literal = match value {
            Value::Boolean(value) => LiteralValue::Boolean(*value),
            Value::Integer(value) => LiteralValue::Integer(value.clone()),
            Value::Rational(value) => LiteralValue::Rational(value.clone()),
            Value::Float(value) => match value.width() {
                quire_exact::IeeeWidth::Binary32 => LiteralValue::Float32Bits(value.bits() as u32),
                quire_exact::IeeeWidth::Binary64 => LiteralValue::Float64Bits(value.bits()),
            },
            Value::Enum(member) => {
                // FR-093: the enum member's QSpec `enum_value` node, whose
                // key is its `VariantId` (ADR-013 O-14). A model variant
                // type's nodes are built where it is first named, here.
                self.type_ref(node.value_type(), node.location())?;
                let variant = member.variant();
                let key = super::sum_case::mint_union_member_key(variant.owner(), variant.case())
                    .map_err(|_| {
                    fault(
                        node.location(),
                        KeyFault::InvalidMemberKey(variant.owner().clone()),
                    )
                })?;
                self.record(
                    key.clone(),
                    OccurrenceRole::Expression,
                    node.location().clone(),
                );
                return Ok(LeafTerm::reference_node(key.into()));
            }
            // FR-093-AC-33: a quantity literal is a `quantity` literal typed
            // by its unit's node; the magnitude is already reduced.
            Value::Quantity(quantity) => LiteralValue::Quantity(quantity.magnitude().clone()),
            // `absent` (STD-493 ruling): QSpec FR-322's `none` literal,
            // typed by its option node.
            Value::Option(option) if option.payload().is_none() => LiteralValue::None,
            Value::Decimal(_)
            | Value::Text(_)
            | Value::Option(_)
            | Value::Composite(_)
            | Value::Collection(_)
            | Value::Reference(_)
            | Value::Population(_)
            | Value::Union(_) => return Err(fault(node.location(), KeyFault::UnbuiltLiteral)),
            Value::Uuid(value) => LiteralValue::Uuid(*value),
            Value::Timestamp(value) => LiteralValue::Timestamp(*value),
            Value::String(value) => LiteralValue::String(value.clone()),
        };
        let semantic_type = self.type_ref(node.value_type(), node.location())?;
        self.value_node(
            node,
            "literal",
            semantic_type.clone(),
            BodyTerm::literal_node(semantic_type.into(), literal),
        )
    }

    /// The `record.project` member naming `field` (FR-359 item 2): the node
    /// of the record the read is typed at, and the field's name there.
    fn field_member(
        &mut self,
        field: &RecordField,
        location: &Location,
    ) -> Result<Member, CheckRefusal> {
        let name = Identifier::new(field.name()).map_err(|_| {
            refuse(
                location,
                CheckCause::NodePreimage(NodeKeyRefusal::EmptyBindingName),
            )
        })?;
        Ok(Member::Field {
            declaration: require_admitted(
                self.composite_type_node(field.declaration(), location)?,
                location,
            )?,
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
            ValueType::Quantity(_) | ValueType::BoundedQuantity(_) => "quantity",
            ValueType::Reference(_) => "reference",
            // QSpec FR-149 states no map equality relation.
            ValueType::Map(_) => {
                return Err(refuse(
                    location,
                    CheckCause::IllTyped(quire_exact::IllTypedCause::OperatorIneligible),
                ))
            }
            // FR-093 "Native scalar equality": no law, mode, member or
            // leaves, and no text-profile selection.
            ValueType::String => "string",
            ValueType::Uuid => "uuid",
            ValueType::Timestamp => "timestamp",
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
                ));
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

/// One step of lowering a unit's functions and protocols together.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum LoweringStep {
    /// A group of functions that call each other, or one function.
    Functions(FunctionGroup),
    /// The `index`-th protocol, whose callees and referenced protocols are
    /// all keyed before it.
    Protocol(usize),
    /// The `index`-th checked temporal clause, whose formula callees and
    /// protocol references are keyed before its node.
    Temporal(usize),
    /// A cyclic component of function calls and protocol references that
    /// holds a protocol (QSL-844): its protocols have no node, then its
    /// functions are lowered in their own order.
    Cycle {
        /// The component's protocols, ascending.
        protocols: Vec<usize>,
        /// The component's functions, each callee's group first.
        functions: Vec<FunctionGroup>,
        /// Temporal clauses in the component, ascending.
        temporals: Vec<usize>,
    },
}

/// The order to lower functions and protocols in (FR-093: a node's key
/// hashes the keys of the nodes it names, so each is keyed before the
/// nodes that name it). `function_edges[f]` are the functions `f` calls and
/// the protocols it refers to; `protocol_edges[p]` are the functions `p`'s
/// bodies call and the protocols they refer to.
pub(crate) fn dependency_order(
    function_edges: &[(Vec<usize>, Vec<usize>)],
    protocol_edges: &[(Vec<usize>, Vec<usize>, Vec<usize>)],
    temporal_edges: &[(Vec<usize>, Vec<usize>)],
) -> Vec<LoweringStep> {
    let functions = function_edges.len();
    let protocols = protocol_edges.len();
    let temporals = temporal_edges.len();
    let edges: Vec<Vec<usize>> = function_edges
        .iter()
        .map(|(called, referenced)| {
            called
                .iter()
                .copied()
                .filter(|callee| *callee < functions)
                .chain(
                    referenced
                        .iter()
                        .copied()
                        .filter(|protocol| *protocol < protocols)
                        .map(|protocol| functions + protocol),
                )
                .collect()
        })
        .chain(protocol_edges.iter().map(|(called, referenced, required)| {
            called
                .iter()
                .copied()
                .filter(|callee| *callee < functions)
                .chain(
                    referenced
                        .iter()
                        .copied()
                        .filter(|protocol| *protocol < protocols)
                        .map(|protocol| functions + protocol),
                )
                .chain(
                    required
                        .iter()
                        .copied()
                        .filter(|temporal| *temporal < temporals)
                        .map(|temporal| functions + protocols + temporal),
                )
                .collect()
        }))
        .chain(temporal_edges.iter().map(|(called, referenced)| {
            called
                .iter()
                .copied()
                .filter(|callee| *callee < functions)
                .chain(
                    referenced
                        .iter()
                        .copied()
                        .filter(|protocol| *protocol < protocols)
                        .map(|protocol| functions + protocol),
                )
                .collect()
        }))
        .collect();
    let mut steps = Vec::new();
    for mut members in strongly_connected(&edges) {
        members.sort_unstable();
        let (members_functions, members_non_functions): (Vec<usize>, Vec<usize>) =
            members.iter().partition(|member| **member < functions);
        let (members_protocols, members_temporals): (Vec<usize>, Vec<usize>) =
            members_non_functions
                .into_iter()
                .partition(|member| *member < functions + protocols);
        let protocol_indices: Vec<usize> = members_protocols
            .iter()
            .map(|member| member - functions)
            .collect();
        let temporal_indices: Vec<usize> = members_temporals
            .iter()
            .map(|member| member - functions - protocols)
            .collect();
        if members_protocols.is_empty() {
            if members_functions.is_empty()
                && temporal_indices.len() == 1
                && members
                    .first()
                    .is_some_and(|member| !edges[*member].contains(member))
            {
                steps.push(LoweringStep::Temporal(temporal_indices[0]));
                continue;
            }
            if !temporal_indices.is_empty() {
                let local: Vec<Vec<usize>> = members_functions
                    .iter()
                    .map(|member| {
                        function_edges[*member]
                            .0
                            .iter()
                            .filter_map(|callee| members_functions.iter().position(|m| m == callee))
                            .collect()
                    })
                    .collect();
                let groups = lowering_order(&local)
                    .into_iter()
                    .map(|group| FunctionGroup {
                        members: group
                            .members
                            .into_iter()
                            .map(|member| members_functions[member])
                            .collect(),
                        recursive: group.recursive,
                    })
                    .collect();
                steps.push(LoweringStep::Cycle {
                    protocols: Vec::new(),
                    functions: groups,
                    temporals: temporal_indices,
                });
                continue;
            }
            let recursive = members_functions.len() > 1
                || members_functions
                    .first()
                    .is_some_and(|member| edges[*member].contains(member));
            steps.push(LoweringStep::Functions(FunctionGroup {
                members: members_functions,
                recursive,
            }));
            continue;
        }
        let cyclic = members.len() > 1
            || members
                .first()
                .is_some_and(|member| edges[*member].contains(member));
        if !cyclic {
            steps.extend(protocol_indices.into_iter().map(LoweringStep::Protocol));
            continue;
        }
        // The functions of the component, ordered by their calls alone.
        let local: Vec<Vec<usize>> = members_functions
            .iter()
            .map(|member| {
                function_edges[*member]
                    .0
                    .iter()
                    .filter_map(|callee| members_functions.iter().position(|m| m == callee))
                    .collect()
            })
            .collect();
        let groups = lowering_order(&local)
            .into_iter()
            .map(|group| FunctionGroup {
                members: group
                    .members
                    .into_iter()
                    .map(|member| members_functions[member])
                    .collect(),
                recursive: group.recursive,
            })
            .collect();
        steps.push(LoweringStep::Cycle {
            protocols: protocol_indices,
            functions: groups,
            temporals: temporal_indices,
        });
    }
    steps
}

/// The strongly connected components of the graph `edges` (Tarjan's
/// algorithm, with an explicit stack), each component after every
/// component it has an edge to.
pub(super) fn strongly_connected(edges: &[Vec<usize>]) -> Vec<Vec<usize>> {
    strongly_connected_metered(edges, |_| Ok(())).expect("unmetered SCC discovery succeeds")
}

/// The strongly-connected components of `edges`, charging one unit for
/// each discovered vertex and each examined edge. `charge` receives the
/// source vertex whose work is about to be performed.
fn strongly_connected_metered(
    edges: &[Vec<usize>],
    mut charge: impl FnMut(usize) -> Result<(), CheckRefusal>,
) -> Result<Vec<Vec<usize>>, CheckRefusal> {
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
        charge(root)?;
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
                charge(node)?;
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
    Ok(components)
}

/// The location of the `declaration` occurrence of the type declared as
/// `name`.
pub(crate) fn type_declaration(name: &str) -> Location {
    Location::root(Origin::TypeDeclaration {
        name: name.to_owned(),
    })
}

/// The nodes `node` names: its semantic type and every node its body
/// references.
fn named_nodes(node: &SemanticNode) -> Vec<NodeKey> {
    let body = &node.content.body;
    let mut named: Vec<NodeKey> = node
        .content
        .semantic_type
        .as_ref()
        .and_then(NodeRef::admitted)
        .cloned()
        .into_iter()
        .collect();
    match body {
        BodyTerm::Application(application) => {
            named.extend(application.result_type.admitted().cloned());
        }
        BodyTerm::Frame(frame) => {
            named.extend(
                frame
                    .modifies
                    .iter()
                    .filter_map(|entry| entry.declaration().admitted().cloned()),
            );
            named.extend(
                frame
                    .creates
                    .iter()
                    .chain(&frame.deletes)
                    .filter_map(|node| node.admitted().cloned()),
            );
        }
        BodyTerm::Abstraction(term) => named.extend(term.targets().into_iter().flatten()),
        BodyTerm::Leaf(_) | BodyTerm::Aggregate(_) => {}
    }
    // A dependency's node is not a node of this graph.
    named.extend(body.leaves().into_iter().filter_map(LeafTerm::key));
    named
}

/// The nodes an anchor reaches from `node` (FR-096): the nodes it names,
/// and for an enum declaration its member nodes. A member names its
/// declaration and nothing names a member but an enum literal, yet a
/// declaration a function reaches (an enum-typed parameter) reaches each
/// member it declares: the members are part of the declaration (FR-092
/// rule 1), so each is placed with it whether or not a literal names it.
fn reached_nodes<'g>(
    node: &'g SemanticNode,
    members: &'g BTreeMap<NodeKey, Vec<NodeKey>>,
) -> impl Iterator<Item = NodeKey> + 'g {
    named_nodes(node)
        .into_iter()
        .chain(members.get(&node.key).into_iter().flatten().cloned())
}

/// Each enum declaration node's member nodes in `graph`.
fn enum_members(graph: &SemanticGraph) -> BTreeMap<NodeKey, Vec<NodeKey>> {
    let mut members: BTreeMap<NodeKey, Vec<NodeKey>> = BTreeMap::new();
    for node in graph.nodes.values() {
        if let (NodeTag::Value, "enum_value", Some(NodeRef::Admitted(declaration))) = (
            node.content.node_tag,
            node.content.semantic_form,
            &node.content.semantic_type,
        ) {
            members
                .entry(declaration.clone())
                .or_default()
                .push(node.key.clone());
        }
    }
    members
}

/// FR-096: for each node a declaration reaches ([`reached_nodes`]),
/// directly or through other nodes, the root of the least such declaration.
/// A `generated` occurrence of the node is placed there: the node has no
/// source position of its own, and IR requires a region for every
/// occurrence. A function body or measure, a state clause body or a
/// protocol attempt places first; a node none of those reaches is placed
/// at the least declared type name that reaches it (a declared record,
/// tuple or enum no function names, and the nodes only it names). A node
/// no declaration reaches has no entry.
fn enclosing_declarations(
    graph: &SemanticGraph,
    occurrences: &OccurrenceMap<Location>,
) -> BTreeMap<NodeKey, Location> {
    let members = enum_members(graph);
    // Only a function body or measure, a state clause body (FR-104), or a
    // protocol attempt's own declared name (FR-114: an operation named only
    // by an attempt has no clause to place its anchor-named nodes under,
    // SR-770 FND-002) resolves to a region (FR-096).
    let mut anchors = relax(
        graph,
        &members,
        roots(occurrences, |location| {
            matches!(
                location.origin,
                Origin::Body { .. }
                    | Origin::Measure { .. }
                    | Origin::StateClause { .. }
                    | Origin::ProtocolAttempt { .. }
            )
        }),
        &BTreeMap::new(),
    );
    // A declared type's `declaration` occurrence names its declared name
    // (FR-322), which the assembler located.
    let declared = roots(occurrences, |location| {
        matches!(location.origin, Origin::TypeDeclaration { .. }) && location.depth() == 0
    })
    .into_iter()
    .filter(|(key, _)| !anchors.contains_key(key))
    .collect();
    let declared = relax(graph, &members, declared, &anchors);
    anchors.extend(declared);
    anchors
}

/// For each node with an occurrence `admits`, the least root of those
/// occurrences' declarations.
fn roots(
    occurrences: &OccurrenceMap<Location>,
    admits: impl Fn(&Location) -> bool,
) -> BTreeMap<NodeKey, Location> {
    let mut roots: BTreeMap<NodeKey, Location> = BTreeMap::new();
    for (key, _, location) in occurrences.iter() {
        if !admits(location) {
            continue;
        }
        let root = Location::root(location.origin.clone());
        match roots.entry(key) {
            btree_map::Entry::Vacant(entry) => {
                entry.insert(root);
            }
            btree_map::Entry::Occupied(mut entry) => {
                if root < *entry.get() {
                    entry.insert(root);
                }
            }
        }
    }
    roots
}

/// `anchors` relaxed along [`reached_nodes`] until no anchor lowers, never
/// into a node `placed` already anchors. Each anchor only decreases, over
/// the finitely many declaration roots, so this ends.
fn relax(
    graph: &SemanticGraph,
    members: &BTreeMap<NodeKey, Vec<NodeKey>>,
    mut anchors: BTreeMap<NodeKey, Location>,
    placed: &BTreeMap<NodeKey, Location>,
) -> BTreeMap<NodeKey, Location> {
    let mut work: Vec<NodeKey> = anchors.keys().cloned().collect();
    while let Some(key) = work.pop() {
        let (Some(node), Some(anchor)) = (graph.nodes.get(&key), anchors.get(&key).cloned()) else {
            continue;
        };
        for named in reached_nodes(node, members) {
            if placed.contains_key(&named) {
                continue;
            }
            let lowers = anchors.get(&named).is_none_or(|current| anchor < *current);
            if lowers {
                anchors.insert(named.clone(), anchor.clone());
                work.push(named);
            }
        }
    }
    anchors
}

/// The package root location a `generated` occurrence names when no
/// declaration reaches its node.
pub(crate) fn generated_location() -> Location {
    Location::root(Origin::Expression)
}

/// Whether `identity` is an integer operator whose integer literal operands
/// are inline literal terms: the FR-357 operators `add`, `sub`, `mul` and
/// `negate`, and the integer `eq` and `ne` of FR-358 step 6.
fn takes_inline_integer_literals(identity: &str) -> bool {
    matches!(
        identity,
        "quire.op.integer.add"
            | "quire.op.integer.sub"
            | "quire.op.integer.mul"
            | "quire.op.integer.negate"
            | "quire.op.integer.eq"
            | "quire.op.integer.ne"
    )
}

#[cfg(test)]
mod tests;
