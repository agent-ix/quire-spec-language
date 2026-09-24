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
//! source occurrence of it is its own occurrence entry. Every key comes from
//! [`super::node_key::node_key`]. Laws come only from the package's lock
//! evidence ([`LockEvidence`]); a law it does not supply refuses the node
//! (`missing_declaration`/`missing-selection`), never a constant.
//!
//! FR-094 keys the rest (`model`): the model declaration nodes a
//! `Reference<T>` or a model row's member names, the `Reference<T>` and
//! `Population<T>[N]` type nodes, a clause function's `ModelOwner` and
//! `clause` binding, and quantity type nodes. `check` records each model
//! declaration node it keys in the model correspondence.

use std::collections::BTreeMap;

use quire_exact::{
    ArithmeticOperator, CollectionKind, EffectiveId, Identifier, Integer, NodeKey,
    OrderingOperator, TextProfile, Value, ValueType,
};

use super::check::Scope;
use super::family::OccurrenceMap;
use super::ir::{Arithmetic, Connective, Node, NodeKind, OrderedKind, RecordSlot, Slot, Visit};
use super::node_key::{
    node_key, LawRole, LeafSegment, LiteralValue, NodeInput, NodeKeyRefusal, NodeTag, Operation,
    OperationLaw, OperationLeaf, OperationMode, Operator, Owner, SemanticTerm, SourceOwner,
};
use super::refusal::{
    CheckCause, CheckRefusal, CheckingLimitKind, CheckingStage, KeyFault, Location, Origin,
};
use crate::model::key::DeclarationKey;
use crate::value::declaration::{CompositeShape, EqualityOperator};
use crate::value::definition::DefinitionReference;
use crate::value::member::Member;
use crate::value::quantity::UnitTable;

mod model;

pub use model::{AdmittedModel, ForeignView, ModelClause};

/// The package's lock evidence as the lowering reads it (ADR-011 §2.4): the
/// `DefinitionRef` each profile law role selects. QSpec publishes no
/// `complete-value-lock.json` accessor yet, so every production package
/// supplies none, and each law-bearing operation refuses
/// (`missing_declaration`/`missing-selection`, FR-093-AC-6).
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct LockEvidence {
    text_profile: Option<DefinitionReference>,
    ieee_profile: Option<DefinitionReference>,
}

impl LockEvidence {
    /// Lock evidence selecting `definition` for the `text_profile` role.
    pub fn with_text_profile(mut self, definition: DefinitionReference) -> Self {
        self.text_profile = Some(definition);
        self
    }

    /// Lock evidence selecting `definition` for the `ieee_profile` role.
    pub fn with_ieee_profile(mut self, definition: DefinitionReference) -> Self {
        self.ieee_profile = Some(definition);
        self
    }

    fn definition(&self, role: LawRole) -> Option<&DefinitionReference> {
        match role {
            LawRole::TextProfile => self.text_profile.as_ref(),
            LawRole::IeeeProfile => self.ieee_profile.as_ref(),
            LawRole::IntegerDivision | LawRole::TemporalProfile | LawRole::ProtocolProfile => None,
        }
    }
}

/// One lowered, keyed node of the checked semantic graph.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticNode {
    key: NodeKey,
    preimage: Vec<u8>,
    node_tag: NodeTag,
    semantic_form: &'static str,
    semantic_type: Option<NodeKey>,
    declaration: Option<Vec<Identifier>>,
    owner: Option<Owner>,
    body: SemanticTerm,
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
        self.node_tag
    }

    /// The node's FR-322 `semantic_form`.
    pub fn semantic_form(&self) -> &'static str {
        self.semantic_form
    }

    /// The node's semantic type, or `None` for a self-typed node.
    pub fn semantic_type(&self) -> Option<NodeKey> {
        self.semantic_type
    }

    /// The node's `declaration.qualified_name`, when it declares.
    pub fn declaration(&self) -> Option<&[Identifier]> {
        self.declaration.as_deref()
    }

    /// The node's owner: a declared node's `SourceOwner`, a model-owned
    /// node's `ModelOwner`.
    pub fn owner(&self) -> Option<&Owner> {
        self.owner.as_ref()
    }

    /// The node's FR-322 body.
    pub fn body(&self) -> &SemanticTerm {
        &self.body
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

/// Builds and keys the nodes of one package.
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
    /// Composites whose node is being built, for cycle detection.
    composites_in_progress: Vec<NodeKey>,
}

fn refuse(location: &Location, cause: CheckCause) -> CheckRefusal {
    CheckRefusal {
        location: location.clone(),
        cause,
    }
}

fn fault(location: &Location, fault: KeyFault) -> CheckRefusal {
    refuse(location, CheckCause::InternalFault(fault))
}

fn preimage_refusal(location: &Location, refusal: NodeKeyRefusal) -> CheckRefusal {
    match refusal {
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
            composites_in_progress: Vec::new(),
        }
    }

    /// The finished graph and its model correspondence entries, `(node
    /// key, DeclarationKey)`. Every node that no region denotes gets its one
    /// `generated` occurrence (FR-093), at `root`.
    pub(crate) fn finish(self, root: &Location) -> (SemanticGraph, Vec<(NodeKey, DeclarationKey)>) {
        for key in self.graph.nodes.keys() {
            if !self.occurrences.has(*key) {
                self.occurrences.record(*key, "generated", root.clone());
            }
        }
        (self.graph, model::correspondence_entries(self.correspondence))
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
        self.insert_node(location, node_tag, semantic_form, semantic_type, declaration, owner, body)
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
        self.insert_node(location, node_tag, semantic_form, semantic_type, None, Some(owner), body)
    }

    /// Key a node from every preimage member and add it to the graph.
    #[allow(clippy::too_many_arguments)]
    fn insert_node(
        &mut self,
        location: &Location,
        node_tag: NodeTag,
        semantic_form: &'static str,
        semantic_type: Option<NodeKey>,
        declaration: Option<Vec<Identifier>>,
        owner: Option<Owner>,
        body: SemanticTerm,
    ) -> Result<NodeKey, CheckRefusal> {
        let keyed = node_key(&NodeInput {
            owner: owner.as_ref(),
            node_tag,
            semantic_form,
            semantic_type,
            declaration: declaration.as_deref(),
            recursion: None,
            body: &body,
        })
        .map_err(|refusal| preimage_refusal(location, refusal))?;
        let key = keyed.key;
        self.graph.nodes.entry(key).or_insert_with(|| SemanticNode {
            key,
            preimage: keyed.preimage,
            node_tag,
            semantic_form,
            semantic_type,
            declaration,
            owner,
            body,
        });
        Ok(key)
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
        Ok(SemanticTerm::literal(text, LiteralValue::Text(value.to_owned())))
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

    /// The FR-092 type node of `value_type`.
    pub(crate) fn type_node(
        &mut self,
        value_type: &ValueType,
        location: &Location,
    ) -> Result<NodeKey, CheckRefusal> {
        self.type_node_at(value_type, location, 0)
    }

    fn type_node_at(
        &mut self,
        value_type: &ValueType,
        location: &Location,
        depth: u64,
    ) -> Result<NodeKey, CheckRefusal> {
        self.check_depth(depth, location)?;
        match value_type {
            ValueType::Boolean => self.scalar("boolean", location),
            ValueType::Integer => self.scalar("integer", location),
            ValueType::Int(interval) => {
                let base = self.scalar("integer", location)?;
                let min = self.integer_literal(interval.lower().clone(), location)?;
                let max = self.integer_literal(interval.upper().clone(), location)?;
                self.bounded("integer_range", base, vec![("min", min), ("max", max)], location)
            }
            ValueType::Rational(domain) => {
                let base = self.scalar("rational", location)?;
                let numerator_min = self.integer_literal(domain.numerator().lower().clone(), location)?;
                let numerator_max = self.integer_literal(domain.numerator().upper().clone(), location)?;
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
                let rounding = self.text_literal(quire_exact::RoundingMode::Exact.as_str(), location)?;
                self.bounded("float_rounding", base, vec![("rounding", rounding)], location)
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
                    .enums
                    .iter()
                    .find(|binding| binding.shape() == *shape)
                    .map(|binding| binding.declaration.key());
                declaration.ok_or_else(|| {
                    refuse(
                        location,
                        CheckCause::IllTyped(quire_exact::IllTypedCause::TypeMismatch),
                    )
                })
            }
            ValueType::Option(payload) => {
                let payload = self.type_node_at(payload, location, depth + 1)?;
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
            ValueType::Collection(collection) => {
                let element = self.type_node_at(collection.element(), location, depth + 1)?;
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
                self.bounded("collection_bounds", base, vec![("min", min), ("max", max)], location)
            }
            ValueType::Composite(declaration) => self.composite(*declaration, location, depth),
            ValueType::Quantity(unit) => self.quantity_type(*unit, location),
            ValueType::Reference(target) => self.reference_type(*target, location),
            // FR-094: a `Population<T>[N]` node is built from its binding's
            // resolved type form ([`Self::binder_type`]); the checked type
            // alone carries no `T`.
            ValueType::Population(_) => Err(fault(location, KeyFault::UntargetedPopulation)),
        }
    }

    /// A declared record or tuple's node: its `declaration` and the unit's
    /// `owner`, one binding per field (record) or one reference per position
    /// (tuple).
    fn composite(
        &mut self,
        declaration: NodeKey,
        location: &Location,
        depth: u64,
    ) -> Result<NodeKey, CheckRefusal> {
        if let Some(key) = self.composites.get(&declaration) {
            return Ok(*key);
        }
        let Some(composite) = self.scope.types.composite(declaration) else {
            return Err(refuse(
                location,
                CheckCause::IllTyped(quire_exact::IllTypedCause::TypeMismatch),
            ));
        };
        if self.composites_in_progress.contains(&declaration) {
            // A record reaching itself is a recursion group (FR-092-OQ-1).
            return Err(refuse(
                location,
                CheckCause::UnsupportedFeature {
                    loci: vec![location.clone()],
                },
            ));
        }
        let name = qualified_name(composite.name(), location)?;
        let shape = composite.shape().clone();
        self.composites_in_progress.push(declaration);
        let body = self.composite_body(&shape, location, depth);
        self.composites_in_progress.pop();
        let (form, body) = body?;
        let key = self.insert(
            location,
            NodeTag::CompositeType,
            form,
            None,
            Some(name),
            body,
        )?;
        self.composites.insert(declaration, key);
        Ok(key)
    }

    fn composite_body(
        &mut self,
        shape: &CompositeShape,
        location: &Location,
        depth: u64,
    ) -> Result<(&'static str, SemanticTerm), CheckRefusal> {
        match shape {
            CompositeShape::Record(fields) => {
                let mut members = Vec::with_capacity(fields.len());
                for field in fields {
                    let value = match field.presence() {
                        quire_exact::Presence::Required => SemanticTerm::reference(
                            self.type_node_at(field.value_type(), location, depth + 1)?,
                        ),
                        quire_exact::Presence::Optional => {
                            let option = ValueType::option(field.value_type().clone());
                            SemanticTerm::binding(
                                "optional",
                                SemanticTerm::reference(
                                    self.type_node_at(&option, location, depth + 1)?,
                                ),
                            )
                        }
                    };
                    members.push(SemanticTerm::binding(field.name(), value));
                }
                Ok(("record", SemanticTerm::Aggregate { members }))
            }
            CompositeShape::Tuple(positions) => {
                let mut members = Vec::with_capacity(positions.len());
                for position in positions {
                    members.push(SemanticTerm::reference(
                        self.type_node_at(position, location, depth + 1)?,
                    ));
                }
                Ok(("tuple", SemanticTerm::Aggregate { members }))
            }
        }
    }

    /// The text leaves of `value_type`, each with its path and profile, in
    /// declaration order.
    fn text_leaves(
        &self,
        value_type: &ValueType,
        path: &mut Vec<LeafSegment>,
        leaves: &mut Vec<(Vec<LeafSegment>, TextProfile)>,
        location: &Location,
        depth: u64,
    ) -> Result<(), CheckRefusal> {
        self.check_depth(depth, location)?;
        match value_type {
            ValueType::Text(text) => leaves.push((path.clone(), text.profile())),
            ValueType::Option(payload) => {
                path.push(LeafSegment::Inner);
                self.text_leaves(payload, path, leaves, location, depth + 1)?;
                path.pop();
            }
            ValueType::Collection(collection) => {
                path.push(LeafSegment::Inner);
                self.text_leaves(collection.element(), path, leaves, location, depth + 1)?;
                path.pop();
            }
            ValueType::Composite(declaration) => {
                let Some(composite) = self.scope.types.composite(*declaration) else {
                    return Ok(());
                };
                match composite.shape() {
                    CompositeShape::Record(fields) => {
                        for field in fields {
                            let name = Identifier::new(field.name()).map_err(|_| {
                                refuse(
                                    location,
                                    CheckCause::NodePreimage(NodeKeyRefusal::EmptyBindingName),
                                )
                            })?;
                            path.push(LeafSegment::Field(name));
                            self.text_leaves(field.value_type(), path, leaves, location, depth + 1)?;
                            path.pop();
                        }
                    }
                    CompositeShape::Tuple(positions) => {
                        for (position, value_type) in (0_u64..).zip(positions) {
                            path.push(LeafSegment::Position(position));
                            self.text_leaves(value_type, path, leaves, location, depth + 1)?;
                            path.pop();
                        }
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
        Ok(())
    }

    /// The law of `role` the lock evidence selects, or the FR-093 refusal.
    fn law(&self, role: LawRole, location: &Location) -> Result<OperationLaw, CheckRefusal> {
        self.lock
            .definition(role)
            .map(|definition| OperationLaw {
                role,
                definition: definition.clone(),
            })
            .ok_or_else(|| refuse(location, CheckCause::MissingSelection { role }))
    }

    /// The `leaves` of an operation whose catalog entry names `source`.
    fn leaves(
        &self,
        source: LeafSource<'_>,
        location: &Location,
    ) -> Result<Vec<OperationLeaf>, CheckRefusal> {
        let compared = match source {
            LeafSource::Compared(value_type) => Some(value_type),
            LeafSource::ResultInner(value_type) => match value_type {
                ValueType::Collection(collection) if collection.kind() != CollectionKind::Sequence => {
                    Some(collection.element())
                }
                _ => None,
            },
        };
        let Some(compared) = compared else {
            return Ok(Vec::new());
        };
        let mut found = Vec::new();
        self.text_leaves(compared, &mut Vec::new(), &mut found, location, 0)?;
        found
            .into_iter()
            .map(|(path, profile)| {
                Ok(OperationLeaf {
                    path,
                    laws: vec![self.law(LawRole::TextProfile, location)?],
                    mode: Some(OperationMode::TextProfile(profile)),
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
        let level_literal = self.integer_literal(Integer::from(level as u64), location)?;
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
        self.occurrences.record(key, "anchor", location.clone());
        Ok(key)
    }

    /// Lower and key `function`, whose callees are already lowered.
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
            self.occurrences.record(type_key, "type", function.location.clone());
            parameters.push(Binder {
                slot: level,
                parameter: key,
                semantic_type: type_key,
            });
        }
        let result = self.type_node(function.result, function.location)?;
        self.occurrences.record(result, "type", function.location.clone());
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
        let body = self.expression(function.body, &mut binders, 0)?;
        members.push(SemanticTerm::binding("body", body));
        if let Some(measure) = function.measure {
            let mut binders = Binders {
                slot_names: function.measure_slots,
                scope: parameters.clone(),
            };
            let measure = self.expression(measure, &mut binders, 0)?;
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
                self.occurrences
                    .record(key, "declaration", function.location.clone());
                key
            }
        };
        if let Some(slot) = self.functions.get_mut(index) {
            *slot = Some(key);
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
            if let Some(binder) = binders.scope.iter().rev().find(|binder| binder.slot == *slot) {
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
        self.occurrences.record(key, "expression", location.clone());
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
        self.occurrences
            .record(key, "expression", node.location.clone());
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

    /// `ref(x)` for each of `nodes`, in order.
    fn operands(
        &mut self,
        nodes: &[&Node],
        binders: &mut Binders<'_>,
        depth: u64,
    ) -> Result<Vec<SemanticTerm>, CheckRefusal> {
        nodes
            .iter()
            .map(|node| self.expression(node, binders, depth + 1))
            .collect()
    }

    /// A one-binder operation's operands: `[ref(source), binding{name,
    /// ref(body)}]`, the binder typed at `source`'s element type.
    fn binder_operands(
        &mut self,
        node: &Node,
        slot: Slot,
        source: &Node,
        body: &Node,
        binders: &mut Binders<'_>,
        depth: u64,
    ) -> Result<Vec<SemanticTerm>, CheckRefusal> {
        let source_ref = self.expression(source, binders, depth + 1)?;
        let element = element_type(&source.value_type, &node.location)?;
        let element = self.type_node(&element, &node.location)?;
        let name = self.bind(binders, slot, element, &node.location)?;
        let body = self.expression(body, binders, depth + 1);
        binders.scope.pop();
        Ok(vec![source_ref, SemanticTerm::binding(name, body?)])
    }

    /// Lower checked `node` and return the term that names it: a reference
    /// to its node, or to its binder's parameter node for a local read.
    #[deny(clippy::wildcard_enum_match_arm)]
    fn expression(
        &mut self,
        node: &Node,
        binders: &mut Binders<'_>,
        depth: u64,
    ) -> Result<SemanticTerm, CheckRefusal> {
        self.check_depth(depth, &node.location)?;
        let plain = Operation::plain;
        match &node.kind {
            NodeKind::Literal(value) => self.literal(node, value),
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
                self.occurrences
                    .record(key, "expression", node.location.clone());
                Ok(SemanticTerm::reference(key))
            }
            NodeKind::Coerce(operand, interval) => {
                // FR-093: an integer whose type the target range contains is
                // admitted with no node.
                if let ValueType::Int(source) = &operand.value_type {
                    if interval.contains(source.lower()) && interval.contains(source.upper()) {
                        return self.expression(operand, binders, depth + 1);
                    }
                }
                let member = self.type_argument(&ValueType::Int(interval.clone()), &node.location)?;
                let arguments = self.operands(&[operand], binders, depth)?;
                self.application(
                    node,
                    Operator::Convert,
                    Operation {
                        member: Some(member),
                        ..plain("quire.op.numeric.narrow")
                    },
                    arguments,
                )
            }
            NodeKind::Let { slot, value, body } => {
                let value_ref = self.expression(value, binders, depth + 1)?;
                let value_type = self.value_type_node(value, binders)?;
                let name = self.bind(binders, *slot, value_type, &node.location)?;
                let body_ref = self.expression(body, binders, depth + 1);
                binders.scope.pop();
                self.application(
                    node,
                    Operator::Let,
                    plain("quire.op.control.let"),
                    vec![SemanticTerm::binding(name, value_ref), body_ref?],
                )
            }
            NodeKind::If {
                condition,
                then,
                otherwise,
            } => {
                let arguments = self.operands(&[condition, then, otherwise], binders, depth)?;
                self.application(node, Operator::Conditional, plain("quire.op.control.if"), arguments)
            }
            NodeKind::Arithmetic(operator, left, right) => {
                let identity = match operator {
                    Arithmetic::Add => "quire.op.integer.add",
                    Arithmetic::Subtract => "quire.op.integer.sub",
                    Arithmetic::Multiply => "quire.op.integer.mul",
                };
                let arguments = self.operands(&[left, right], binders, depth)?;
                self.application(node, Operator::Binary, plain(identity), arguments)
            }
            NodeKind::Negate(operand) => {
                let arguments = self.operands(&[operand], binders, depth)?;
                self.application(node, Operator::Unary, plain("quire.op.integer.negate"), arguments)
            }
            NodeKind::Divide { left, right, .. } => {
                let arguments = self.operands(&[left, right], binders, depth)?;
                self.application(node, Operator::Binary, plain("quire.op.rational.div"), arguments)
            }
            NodeKind::Rational {
                operator,
                left,
                right,
                ..
            } => {
                let identity = format!("quire.op.rational.{}", arithmetic_suffix(*operator));
                let arguments = self.operands(&[left, right], binders, depth)?;
                self.application(node, Operator::Binary, plain(&identity), arguments)
            }
            NodeKind::RationalNegate(operand, _) => {
                let arguments = self.operands(&[operand], binders, depth)?;
                self.application(node, Operator::Unary, plain("quire.op.rational.negate"), arguments)
            }
            NodeKind::Decimal {
                operator,
                left,
                right,
                target,
            } => {
                let identity = format!("quire.op.decimal.{}", arithmetic_suffix(*operator));
                let arguments = self.operands(&[left, right], binders, depth)?;
                self.application(
                    node,
                    Operator::Binary,
                    Operation {
                        mode: Some(OperationMode::Rounding(target.rounding())),
                        ..plain(&identity)
                    },
                    arguments,
                )
            }
            NodeKind::DecimalNegate(operand, _) => {
                let arguments = self.operands(&[operand], binders, depth)?;
                self.application(node, Operator::Unary, plain("quire.op.decimal.negate"), arguments)
            }
            NodeKind::Quantity(operator, left, right) => {
                let identity = format!("quire.op.quantity.{}", arithmetic_suffix(*operator));
                let arguments = self.operands(&[left, right], binders, depth)?;
                self.application(node, Operator::Binary, plain(&identity), arguments)
            }
            NodeKind::Ieee(operator, left, right) => {
                let width = if let ValueType::Float(quire_exact::IeeeWidth::Binary32) = &left.value_type {
                    "float32"
                } else {
                    "float64"
                };
                let law = self.law(LawRole::IeeeProfile, &node.location)?;
                let identity = format!("quire.op.ieee.{width}.{}", arithmetic_suffix(*operator));
                let arguments = self.operands(&[left, right], binders, depth)?;
                self.application(
                    node,
                    Operator::Binary,
                    Operation {
                        laws: vec![law],
                        mode: Some(OperationMode::Rounding(quire_exact::RoundingMode::Exact)),
                        ..plain(&identity)
                    },
                    arguments,
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
                let arguments = self.operands(&[operand], binders, depth)?;
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
                self.application(node, Operator::Convert, operation, arguments)
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
                let mut operation = plain(&format!("quire.op.{family}.{}", ordering_suffix(*operator)));
                if let OrderedKind::Texts = kind {
                    operation.laws = vec![self.law(LawRole::TextProfile, &node.location)?];
                    operation.mode = text_profile(&left.value_type).map(OperationMode::TextProfile);
                }
                let arguments = self.operands(&[left, right], binders, depth)?;
                self.application(node, Operator::Binary, operation, arguments)
            }
            NodeKind::Equality(operator, _, left, right) => {
                let suffix = match operator {
                    EqualityOperator::Equal => "eq",
                    EqualityOperator::NotEqual => "ne",
                };
                let operation = self.equality(&left.value_type, suffix, &node.location)?;
                let arguments = self.operands(&[left, right], binders, depth)?;
                self.application(node, Operator::Binary, operation, arguments)
            }
            NodeKind::Connective(connective, left, right) => {
                let identity = match connective {
                    Connective::And => "quire.op.boolean.and",
                    Connective::Or => "quire.op.boolean.or",
                    Connective::Implies => "quire.op.boolean.implies",
                };
                let arguments = self.operands(&[left, right], binders, depth)?;
                self.application(node, Operator::Binary, plain(identity), arguments)
            }
            NodeKind::Not(operand) => {
                let arguments = self.operands(&[operand], binders, depth)?;
                self.application(node, Operator::Unary, plain("quire.op.boolean.not"), arguments)
            }
            NodeKind::Field { operand, index, .. } => {
                let member = self.field_member(&operand.value_type, *index, &node.location)?;
                let arguments = self.operands(&[operand], binders, depth)?;
                self.application(
                    node,
                    Operator::Query,
                    Operation {
                        member: Some(member),
                        ..plain("quire.op.record.project")
                    },
                    arguments,
                )
            }
            NodeKind::Attribute {
                reference, name, ..
            } => {
                // FR-093/FR-094 (QC-24): `quire.op.model.deref` over the
                // reference, typed at its object type `T`'s model node, and
                // over it the `record.project` of field `name` of `T`.
                let object = self.referenced_object(&reference.value_type, &node.location)?;
                let arguments = self.operands(&[reference], binders, depth)?;
                let dereferenced = self.typed_application(
                    &node.location,
                    object,
                    Operator::Deref,
                    plain("quire.op.model.deref"),
                    arguments,
                )?;
                let name = Identifier::new(name.as_str()).map_err(|_| {
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
            }
            NodeKind::Present(operand) => {
                let arguments = self.operands(&[operand], binders, depth)?;
                self.application(node, Operator::Present, plain("quire.op.option.present"), arguments)
            }
            NodeKind::Value(operand) => {
                let arguments = self.operands(&[operand], binders, depth)?;
                self.application(node, Operator::Value, plain("quire.op.option.value"), arguments)
            }
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
                let mut operands = vec![SemanticTerm::reference(callee)];
                for argument in arguments {
                    operands.push(self.expression(argument, binders, depth + 1)?);
                }
                self.application(node, Operator::Call, plain("quire.op.function.call"), operands)
            }
            NodeKind::Tuple {
                declaration,
                arguments,
            } => {
                let semantic_type = self.composite(*declaration, &node.location, 0)?;
                let mut members = Vec::with_capacity(arguments.len());
                for argument in arguments {
                    members.push(self.expression(argument, binders, depth + 1)?);
                }
                self.value_node(node, "tuple_value", semantic_type, SemanticTerm::Aggregate { members })
            }
            NodeKind::Record { declaration, slots } => {
                let semantic_type = self.composite(*declaration, &node.location, 0)?;
                let Some(CompositeShape::Record(fields)) =
                    self.scope.types.composite(*declaration).map(|c| c.shape().clone())
                else {
                    return Err(refuse(
                        &node.location,
                        CheckCause::IllTyped(quire_exact::IllTypedCause::TypeMismatch),
                    ));
                };
                let mut members = Vec::with_capacity(slots.len());
                for (field, slot) in fields.iter().zip(slots) {
                    let value = match slot {
                        RecordSlot::Present(value) => self.expression(value, binders, depth + 1)?,
                        RecordSlot::Null | RecordSlot::Absent => {
                            let option = ValueType::option(field.value_type().clone());
                            let option = self.type_node(&option, &node.location)?;
                            SemanticTerm::literal(option, LiteralValue::None)
                        }
                    };
                    members.push(SemanticTerm::binding(field.name(), value));
                }
                self.value_node(node, "record_value", semantic_type, SemanticTerm::Aggregate { members })
            }
            NodeKind::Collection {
                collection_type,
                elements,
            } => {
                let identity = format!("quire.op.collection.{}", collection_form(collection_type.kind()));
                let leaves = self.leaves(LeafSource::ResultInner(&node.value_type), &node.location)?;
                let mut operands = Vec::with_capacity(elements.len());
                for element in elements {
                    operands.push(self.expression(element, binders, depth + 1)?);
                }
                self.application(
                    node,
                    Operator::Collection,
                    Operation {
                        leaves,
                        ..plain(&identity)
                    },
                    operands,
                )
            }
            NodeKind::ConvertCollection { target, operand } => {
                let member = self.type_argument(&ValueType::collection(target.clone()), &node.location)?;
                let leaves = self.leaves(LeafSource::ResultInner(&node.value_type), &node.location)?;
                let arguments = self.operands(&[operand], binders, depth)?;
                self.application(
                    node,
                    Operator::Convert,
                    Operation {
                        member: Some(member),
                        leaves,
                        ..plain("quire.op.collection.convert")
                    },
                    arguments,
                )
            }
            NodeKind::ConvertScalar(target, operand) => {
                let target = target.target().unwrap_or(&operand.value_type).clone();
                if target == operand.value_type {
                    // FR-093: a conversion to the operand's own type builds
                    // no node.
                    return self.expression(operand, binders, depth + 1);
                }
                let member = self.type_argument(&target, &node.location)?;
                if let ValueType::Quantity(_) = &operand.value_type {
                    // FR-093 `quire.op.quantity.convert`: a quantity's
                    // magnitude is an exact rational, and its conversion is
                    // exact (`value::quantity`'s `QuantityTarget::Exact`).
                    let arguments = self.operands(&[operand], binders, depth)?;
                    return self.application(
                        node,
                        Operator::Convert,
                        Operation {
                            member: Some(member),
                            mode: Some(OperationMode::Rounding(quire_exact::RoundingMode::Exact)),
                            ..plain("quire.op.quantity.convert")
                        },
                        arguments,
                    );
                }
                let arguments = self.operands(&[operand], binders, depth)?;
                self.application(
                    node,
                    Operator::Convert,
                    Operation {
                        member: Some(member),
                        ..plain("quire.op.numeric.convert")
                    },
                    arguments,
                )
            }
            NodeKind::IeeeToRational(operand, domain) => {
                let law = self.law(LawRole::IeeeProfile, &node.location)?;
                let member = self.type_argument(&ValueType::Rational(domain.clone()), &node.location)?;
                let arguments = self.operands(&[operand], binders, depth)?;
                self.application(
                    node,
                    Operator::Convert,
                    Operation {
                        laws: vec![law],
                        member: Some(member),
                        ..plain("quire.op.ieee.to_rational")
                    },
                    arguments,
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
                            leaves: self.leaves(LeafSource::ResultInner(&node.value_type), &node.location)?,
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
                let arguments = self.binder_operands(node, *slot, source, body, binders, depth)?;
                self.application(node, operator, operation, arguments)
            }
            NodeKind::Flatten(operand) => {
                let leaves = self.leaves(LeafSource::ResultInner(&node.value_type), &node.location)?;
                if let NodeKind::Query {
                    visit: Visit::Map,
                    slot,
                    source,
                    body,
                } = &operand.kind
                {
                    // FR-093: `flatMap`, and `flatten(map(..))`, is one
                    // `flat_map` node; no node is built for the inner map.
                    let arguments = self.binder_operands(node, *slot, source, body, binders, depth)?;
                    return self.application(
                        node,
                        Operator::Collection,
                        Operation {
                            leaves,
                            ..plain("quire.op.collection.flat_map")
                        },
                        arguments,
                    );
                }
                let arguments = self.operands(&[operand], binders, depth)?;
                self.application(
                    node,
                    Operator::Collection,
                    Operation {
                        leaves,
                        ..plain("quire.op.collection.flatten")
                    },
                    arguments,
                )
            }
            NodeKind::Fold {
                accumulator,
                binder,
                source,
                step,
                identity,
            } => {
                let source_ref = self.expression(source, binders, depth + 1)?;
                let identity_ref = match identity {
                    Some(identity) => Some(self.expression(identity, binders, depth + 1)?),
                    None => None,
                };
                let element = element_type(&source.value_type, &node.location)?;
                let element = self.type_node(&element, &node.location)?;
                let accumulator_type = self.type_node(&node.value_type, &node.location)?;
                let accumulator_name =
                    self.bind(binders, *accumulator, accumulator_type, &node.location)?;
                let bound = self.bind(binders, *binder, element, &node.location);
                let step_ref = match bound {
                    Ok(binder_name) => {
                        let step = self.expression(step, binders, depth + 1);
                        binders.scope.pop();
                        step.map(|step| (binder_name, step))
                    }
                    Err(refusal) => Err(refusal),
                };
                binders.scope.pop();
                let (binder_name, step_ref) = step_ref?;
                let member = self.type_argument(&node.value_type, &node.location)?;
                let mut arguments = vec![
                    source_ref,
                    SemanticTerm::binding(
                        accumulator_name,
                        SemanticTerm::binding(binder_name, step_ref),
                    ),
                ];
                let identity_name = match identity_ref {
                    Some(identity_ref) => {
                        arguments.push(identity_ref);
                        "quire.op.collection.fold"
                    }
                    None => "quire.op.collection.reduce",
                };
                self.application(
                    node,
                    Operator::Collection,
                    Operation {
                        member: Some(member),
                        ..plain(identity_name)
                    },
                    arguments,
                )
            }
            NodeKind::Size(operand) => {
                let member = self.type_argument(&node.value_type, &node.location)?;
                let arguments = self.operands(&[operand], binders, depth)?;
                self.application(
                    node,
                    Operator::Collection,
                    Operation {
                        member: Some(member),
                        ..plain("quire.op.collection.size")
                    },
                    arguments,
                )
            }
            NodeKind::Contains(collection, item) => {
                let leaves = if let ValueType::Collection(collection_type) = &collection.value_type {
                    self.leaves(LeafSource::Compared(collection_type.element()), &node.location)?
                } else {
                    Vec::new()
                };
                let arguments = self.operands(&[collection, item], binders, depth)?;
                self.application(
                    node,
                    Operator::Collection,
                    Operation {
                        leaves,
                        ..plain("quire.op.collection.contains")
                    },
                    arguments,
                )
            }
            NodeKind::AllInstances { population } => {
                let object = self.referenced_object(&node.value_type, &node.location)?;
                let arguments = self.operands(&[population], binders, depth)?;
                self.application(
                    node,
                    Operator::Query,
                    Operation {
                        member: Some(Member::TypeArgument {
                            declaration: object,
                        }),
                        ..plain("quire.op.model.all_instances")
                    },
                    arguments,
                )
            }
            NodeKind::Lookup {
                population,
                reference,
                absence,
            } => {
                let object = self.referenced_object(&node.value_type, &node.location)?;
                let arguments = self.operands(&[population, reference], binders, depth)?;
                self.application(
                    node,
                    Operator::Query,
                    Operation {
                        member: Some(Member::TypeArgument {
                            declaration: object,
                        }),
                        mode: Some(OperationMode::Absence(*absence)),
                        ..plain("quire.op.model.lookup")
                    },
                    arguments,
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
                let mut operands = vec![self.expression(receiver, binders, depth + 1)?];
                for argument in arguments {
                    operands.push(self.expression(argument, binders, depth + 1)?);
                }
                self.application(
                    node,
                    Operator::Call,
                    Operation {
                        member: Some(Member::Operation {
                            declaration: object,
                            name,
                        }),
                        ..plain("quire.op.model.dispatch_call")
                    },
                    operands,
                )
            }
            NodeKind::Pre(operand) => {
                let arguments = self.operands(&[operand], binders, depth)?;
                self.application(node, Operator::Pre, plain("quire.op.state.pre"), arguments)
            }
        }
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
                self.occurrences
                    .record(key, "expression", node.location.clone());
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
        let name = match self.scope.types.composite(*declaration).map(|c| c.shape()) {
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
            declaration: self.composite(*declaration, location, 0)?,
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

/// The order to lower a package's functions in: every callee before its
/// callers (a call node's key hashes its callee's key, FR-093). A function in
/// a recursion group (a self-call, or a cycle of calls) cannot be keyed
/// apart from another group's (FR-092-OQ-1), so each group refuses
/// `unknown_required_feature`/`unsupported-feature`, naming every member's
/// region (FR-092-AC-7).
pub(crate) fn lowering_order(
    callees: &[Vec<usize>],
    locations: &[Location],
) -> Result<Vec<usize>, Vec<CheckRefusal>> {
    let count = callees.len();
    let reach: Vec<std::collections::BTreeSet<usize>> = (0..count)
        .map(|start| {
            let mut seen = std::collections::BTreeSet::new();
            let mut pending: Vec<usize> = callees[start].clone();
            while let Some(next) = pending.pop() {
                if next < count && seen.insert(next) {
                    pending.extend(callees[next].iter().copied());
                }
            }
            seen
        })
        .collect();
    let mut refusals = Vec::new();
    let mut grouped = std::collections::BTreeSet::new();
    for index in 0..count {
        if grouped.contains(&index) || !reach[index].contains(&index) {
            continue;
        }
        let group: Vec<usize> = (0..count)
            .filter(|&other| reach[index].contains(&other) && reach[other].contains(&index))
            .collect();
        grouped.extend(group.iter().copied());
        let loci: Vec<Location> = group
            .iter()
            .filter_map(|&member| locations.get(member).cloned())
            .collect();
        if let Some(first) = loci.first() {
            refusals.push(refuse(first, CheckCause::UnsupportedFeature { loci: loci.clone() }));
        }
    }
    if !refusals.is_empty() {
        return Err(refusals);
    }
    // Acyclic: a post-order walk puts every callee first.
    let mut order = Vec::with_capacity(count);
    let mut placed = vec![false; count];
    for root in 0..count {
        let mut stack = vec![(root, 0_usize)];
        while let Some((node, next)) = stack.pop() {
            if placed[node] {
                continue;
            }
            match callees[node].get(next) {
                Some(&callee) => {
                    stack.push((node, next + 1));
                    if callee < count && !placed[callee] {
                        stack.push((callee, 0));
                    }
                }
                None => {
                    placed[node] = true;
                    order.push(node);
                }
            }
        }
    }
    Ok(order)
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
