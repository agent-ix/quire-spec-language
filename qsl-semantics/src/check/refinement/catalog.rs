// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-375 and FR-464: source facts for the complete refinement class catalog.
//!
//! The catalog is assembled from checked model and protocol subjects, not
//! from the refinement's authored step rows. Its node association remains an
//! S3 identity until S4 lowers the corresponding declaration to a node key.

use std::collections::BTreeMap;

use qsl_foundation::MemoryModelName;
use quire_checked::key::DeclarationKey;
use quire_exact::{NodeKey, ValueType};

use super::protocol::MemoryStepKind;
use super::ProtocolNodeId;

/// The checked identity that owns one declared refinement step class.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum RefinementCatalogNode {
    /// An effective operation of one of the selected model's operation
    /// owners: the owner's declaration, whose model node is the class node.
    Operation(DeclarationKey),
    /// A class declared by a static protocol control node.
    ProtocolNode {
        /// The protocol clause that declares the node.
        protocol: String,
        /// The node's declaration index in that clause.
        node: ProtocolNodeId,
    },
    /// A class declared by a protocol channel.
    Channel {
        /// The protocol clause that declares the channel.
        protocol: String,
        /// The channel's source name.
        channel: String,
    },
    /// A class declared by a replicated protocol role.
    Role {
        /// The protocol clause that declares the role.
        protocol: String,
        /// The role's source name.
        role: String,
    },
    /// The `activate` class declared by a protocol's `on each` trigger.
    Trigger(String),
}

/// The unique, ordered catalog key: (`class`, `memory_model`) compared as
/// UTF-8 bytes, class first.
pub(super) type CatalogKey = (String, &'static str);

/// One source-level class of a checked model or protocol subject.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RefinementCatalogClass {
    class: String,
    node: Option<RefinementCatalogNode>,
    binder_types: Vec<ValueType>,
    memory_model: Option<MemoryModelName>,
    memory_step: Option<MemoryStepKind>,
}

/// The sole drain operation of a `memory drain` class (FR-375-AC-23).
const STORE_BUFFER_FLUSH: &str = "store-buffer-flush";

impl RefinementCatalogClass {
    /// Alias-free class spelling, such as `Cell::increment` or `memory flush`.
    pub fn class(&self) -> &str {
        &self.class
    }

    /// The declaration identity for a declared class; memory classes have
    /// none.
    pub fn node(&self) -> Option<&RefinementCatalogNode> {
        self.node.as_ref()
    }

    /// The checked parameter or node-binder types, in declaration order.
    pub fn binder_types(&self) -> &[ValueType] {
        &self.binder_types
    }

    /// The sole drain operation of a `memory drain` class (FR-375-AC-23):
    /// the store-buffer flush, a no-op under `sc`. Other classes have none.
    pub fn drain(&self) -> Option<&'static str> {
        match self.memory_step? {
            MemoryStepKind::Drain => Some(STORE_BUFFER_FLUSH),
            MemoryStepKind::Flush => None,
        }
    }

    /// The memory step kind of a memory class, by construction: `drain` or
    /// `flush`. Declared classes have none.
    pub fn memory_step(&self) -> Option<MemoryStepKind> {
        self.memory_step
    }

    /// The catalog ordering key (FR-375-AC-26): the alias-free `class`
    /// spelling, then the memory model label (empty for a declared class).
    pub(super) fn catalog_key(&self) -> CatalogKey {
        (
            self.class.clone(),
            self.memory_model.map_or("", MemoryModelName::label),
        )
    }

    /// The source-default memory model for a memory class.
    pub fn memory_model(&self) -> Option<MemoryModelName> {
        self.memory_model
    }

    pub(super) fn declared(
        class: String,
        node: RefinementCatalogNode,
        binder_types: Vec<ValueType>,
    ) -> Self {
        Self {
            class,
            node: Some(node),
            binder_types,
            memory_model: None,
            memory_step: None,
        }
    }

    /// The `memory <kind>` class of `memory_model`, carrying its typed step
    /// kind (and, for `drain`, its store-buffer flush) by construction
    /// rather than by class spelling.
    pub(super) fn memory(kind: MemoryStepKind, memory_model: MemoryModelName) -> Self {
        Self {
            class: format!("memory {}", kind.label()),
            node: None,
            binder_types: Vec::new(),
            memory_model: Some(memory_model),
            memory_step: Some(kind),
        }
    }
}

/// The independent checked catalog for a refinement's concrete and abstract
/// subjects. Each side is ordered by alias-free UTF-8 class spelling.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RefinementClassCatalog {
    concrete: Vec<RefinementCatalogClass>,
    abstract_side: Vec<RefinementCatalogClass>,
}

impl RefinementClassCatalog {
    /// Concrete classes, complete for the selected concrete subject.
    pub fn concrete(&self) -> &[RefinementCatalogClass] {
        &self.concrete
    }

    /// Abstract classes, complete for the selected abstract subject.
    pub fn abstract_side(&self) -> &[RefinementCatalogClass] {
        &self.abstract_side
    }

    pub(super) fn from_sides(
        concrete: Vec<RefinementCatalogClass>,
        abstract_side: Vec<RefinementCatalogClass>,
    ) -> Self {
        Self {
            concrete: canonical(concrete),
            abstract_side: canonical(abstract_side),
        }
    }
}

/// One catalog class lowered to the package graph's node keys (FR-375).
#[derive(Clone, Eq, PartialEq)]
pub enum KeyedCatalogClass {
    /// A declared class: its node and its binder types, in binder-record
    /// order.
    Declared {
        /// Alias-free class spelling.
        class: String,
        /// The class node's key.
        node: NodeKey,
        /// The binder types' node keys.
        binder_types: Vec<NodeKey>,
    },
    /// A source-default memory class, which has no node.
    Memory(KeyedMemoryClass),
}

impl std::fmt::Debug for KeyedCatalogClass {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Declared {
                class,
                node,
                binder_types,
            } => f
                .debug_struct("Declared")
                .field("class", &class)
                .field(
                    "node",
                    &qsl_foundation::source::node_table::NodeKeyDebug(node),
                )
                .field(
                    "binder_types",
                    &qsl_foundation::source::node_table::NodeList(binder_types.iter()),
                )
                .finish(),
            Self::Memory(value0) => f.debug_tuple("Memory").field(&value0).finish(),
        }
    }
}

/// A source-default memory class of a keyed catalog.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KeyedMemoryClass {
    /// `memory drain` of one source-default model.
    Drain(MemoryModelName),
    /// `memory flush`, a `tso` class.
    Flush,
}

impl KeyedCatalogClass {
    /// Alias-free class spelling.
    pub fn class(&self) -> &str {
        match self {
            Self::Declared { class, .. } => class,
            Self::Memory(KeyedMemoryClass::Drain(_)) => "memory drain",
            Self::Memory(KeyedMemoryClass::Flush) => "memory flush",
        }
    }
}

/// A refinement's class catalog lowered to node keys, each side in the
/// checked catalog's canonical order.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct KeyedClassCatalog {
    pub(crate) concrete: Vec<KeyedCatalogClass>,
    pub(crate) abstract_side: Vec<KeyedCatalogClass>,
}

impl KeyedClassCatalog {
    /// Concrete classes.
    pub fn concrete(&self) -> &[KeyedCatalogClass] {
        &self.concrete
    }

    /// Abstract classes.
    pub fn abstract_side(&self) -> &[KeyedCatalogClass] {
        &self.abstract_side
    }

    /// Every class node and binder-type key, ascending and once each: the
    /// catalog's share of the refinement node's local `dependencies`.
    pub fn node_keys(&self) -> Vec<&NodeKey> {
        let keys: std::collections::BTreeSet<&NodeKey> = self
            .concrete
            .iter()
            .chain(&self.abstract_side)
            .flat_map(|class| match class {
                KeyedCatalogClass::Declared {
                    node, binder_types, ..
                } => std::iter::once(node).chain(binder_types.iter()).collect(),
                KeyedCatalogClass::Memory(_) => Vec::new(),
            })
            .collect();
        keys.into_iter().collect()
    }
}

fn canonical(mut classes: Vec<RefinementCatalogClass>) -> Vec<RefinementCatalogClass> {
    let model =
        |class: &RefinementCatalogClass| class.memory_model.map_or("", MemoryModelName::label);
    classes.sort_by(|left, right| {
        (left.class.as_bytes(), model(left).as_bytes())
            .cmp(&(right.class.as_bytes(), model(right).as_bytes()))
    });
    classes
}

/// Add the selected model's operation classes: one per effective operation
/// of each operation owner (object type, state machine or process), named
/// `<Owner>::<operation>` and keyed by the owner's declaration, so an
/// operation an ancestor declares is a class of every owner that inherits it
/// (QSpec FR-375 "The concrete catalog covers the selected concrete model's
/// operation classes"). The binder record is FR-432's attempt record:
/// the owner's reference type, each parameter's type, then the result type
/// when one is declared.
pub(super) fn add_model_operations(
    classes: &mut BTreeMap<CatalogKey, RefinementCatalogClass>,
    alias: &str,
    model: &super::super::lowering::AdmittedModel,
    operations: &crate::model::operation::OperationTable,
    types: &quire_semantic_value::declaration::TypeEnvironment,
) -> Result<(), quire_semantic_value::catalog::InternalFault> {
    let fault = |invariant: &'static str| {
        quire_semantic_value::catalog::InternalFault::new(
            "check.refinement-class-catalog",
            invariant,
        )
    };
    if model.alias() != Some(alias) {
        return Err(fault("selected-model-alias-matches-catalog-subject"));
    }
    for (owner_key, owner) in model.operation_owners() {
        if types.object_type(owner.clone()).is_none() {
            return Err(fault("selected-model-object-has-checked-type"));
        }
        if owner_key.package != model.selection().identity {
            return Err(fault("operation-owner-belongs-to-selected-model"));
        }
        let Some(owner_name) =
            crate::model::intake::type_identity_segment(&owner_key.package, &owner_key.node)
        else {
            return Err(fault(
                "selected-model-object-has-alias-free-identity-segment",
            ));
        };
        for (_, operation) in operations.effective(types, owner.clone()) {
            let class = format!("{owner_name}::{}", operation.name());
            let binder_types = std::iter::once(ValueType::Reference(owner.clone()))
                .chain(
                    operation
                        .parameters()
                        .iter()
                        .map(|(_, value_type)| value_type.clone()),
                )
                .chain(operation.result().cloned())
                .collect();
            let entry = RefinementCatalogClass::declared(
                class,
                RefinementCatalogNode::Operation(owner_key.clone()),
                binder_types,
            );
            if classes.insert(entry.catalog_key(), entry).is_some() {
                return Err(fault("selected-model-operation-classes-are-unique"));
            }
        }
    }
    Ok(())
}

/// Build both sides of the class catalog from the checked subjects and the
/// checked protocol clauses in the declaring package.
pub(super) struct CatalogBuildInput<'a> {
    pub(super) concrete_alias: &'a str,
    pub(super) concrete_model: &'a super::super::lowering::AdmittedModel,
    pub(super) abstract_alias: &'a str,
    pub(super) abstract_model: &'a super::super::lowering::AdmittedModel,
    pub(super) abstract_classes: Option<&'a super::ProtocolClasses>,
    pub(super) declarations: &'a crate::check::state_clause::ClauseDeclarations<'a>,
    pub(super) protocols: &'a [super::ProtocolView<'a>],
    pub(super) has_abstract_protocol: bool,
}

pub(super) fn build(
    input: CatalogBuildInput<'_>,
) -> Result<Option<RefinementClassCatalog>, quire_semantic_value::catalog::InternalFault> {
    let CatalogBuildInput {
        concrete_alias,
        concrete_model,
        abstract_alias,
        abstract_model,
        abstract_classes,
        declarations,
        protocols,
        has_abstract_protocol,
    } = input;
    if !has_abstract_protocol && protocols.is_empty() {
        return Ok(None);
    }

    let mut concrete = BTreeMap::new();
    let mut abstract_side = BTreeMap::new();
    add_model_operations(
        &mut concrete,
        concrete_alias,
        concrete_model,
        declarations.scope.operations(),
        declarations.scope.types(),
    )?;
    if !has_abstract_protocol {
        add_model_operations(
            &mut abstract_side,
            abstract_alias,
            abstract_model,
            declarations.scope.operations(),
            declarations.scope.types(),
        )?;
    }

    let types = declarations.scope.types();
    for view in protocols {
        let Some(checked) = view.checked else {
            continue;
        };
        let Some(over) = view
            .form
            .binders
            .iter()
            .find(|binder| binder.kind == qsl_forms::BinderKind::Input)
        else {
            return Err(quire_semantic_value::catalog::InternalFault::new(
                "check.refinement-class-catalog",
                "checked-protocol-has-over-parameter",
            ));
        };
        let over = crate::check::type_form::resolve_form(declarations.scope, &over.value_type)
            .map_err(|_| {
                quire_semantic_value::catalog::InternalFault::new(
                    "check.refinement-class-catalog",
                    "checked-protocol-over-type-resolves",
                )
            })?;
        let ValueType::Reference(context) = &over else {
            continue;
        };
        if !concrete_model
            .object_types()
            .any(|(_, object)| object == *context)
        {
            continue;
        }
        let classes = super::ProtocolClasses::of(view.form, checked, view.attempts, types);
        for class in classes.catalog_classes()? {
            insert(&mut concrete, class)?;
        }
    }

    if has_abstract_protocol {
        let Some(classes) = abstract_classes else {
            return Err(quire_semantic_value::catalog::InternalFault::new(
                "check.refinement-class-catalog",
                "selected-abstract-protocol-has-checked-class-table",
            ));
        };
        for class in classes.catalog_classes()? {
            insert(&mut abstract_side, class)?;
        }
    }

    let catalog = RefinementClassCatalog::from_sides(
        concrete.into_values().collect(),
        abstract_side.into_values().collect(),
    );
    let has_protocol_or_memory_class =
        catalog
            .concrete
            .iter()
            .chain(&catalog.abstract_side)
            .any(|class| {
                class.memory_model.is_some()
                    || matches!(
                        class.node,
                        Some(
                            RefinementCatalogNode::ProtocolNode { .. }
                                | RefinementCatalogNode::Channel { .. }
                                | RefinementCatalogNode::Role { .. }
                                | RefinementCatalogNode::Trigger(_)
                        )
                    )
            });
    Ok(has_protocol_or_memory_class.then_some(catalog))
}

fn insert(
    classes: &mut BTreeMap<CatalogKey, RefinementCatalogClass>,
    class: RefinementCatalogClass,
) -> Result<(), quire_semantic_value::catalog::InternalFault> {
    match classes.entry(class.catalog_key()) {
        std::collections::btree_map::Entry::Vacant(entry) => {
            entry.insert(class);
        }
        std::collections::btree_map::Entry::Occupied(entry) if entry.get() == &class => {}
        std::collections::btree_map::Entry::Occupied(_) => {
            return Err(quire_semantic_value::catalog::InternalFault::new(
                "check.refinement-class-catalog",
                "duplicate-subject-class-facts-agree",
            ));
        }
    }
    Ok(())
}
