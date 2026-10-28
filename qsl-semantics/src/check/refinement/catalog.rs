// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-375 and FR-464: source facts for the complete refinement class catalog.
//!
//! The catalog is assembled from checked model and protocol subjects, not
//! from the refinement's authored step rows. Its node association remains an
//! S3 identity until S4 lowers the corresponding declaration to a node key.

use std::collections::BTreeMap;

use qsl_foundation::MemoryModelName;
use quire_checked::key::DeclarationKey;
use quire_exact::ValueType;

use super::ProtocolNodeId;

/// The checked identity that owns one declared refinement step class.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum RefinementCatalogNode {
    /// An operation declared by one of the selected model's object types.
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

/// The spelling of the memory class every source-default model defines.
pub(super) const MEMORY_DRAIN: &str = "memory drain";

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
    drain: Option<&'static str>,
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
        self.drain
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
            drain: None,
        }
    }

    pub(super) fn memory(class: String, memory_model: MemoryModelName) -> Self {
        Self {
            class,
            node: None,
            binder_types: Vec::new(),
            memory_model: Some(memory_model),
            drain: None,
        }
    }

    /// The `memory drain` class of `memory_model`, carrying its store-buffer
    /// flush operation by construction rather than by class spelling.
    pub(super) fn memory_drain(memory_model: MemoryModelName) -> Self {
        Self {
            drain: Some(STORE_BUFFER_FLUSH),
            ..Self::memory(MEMORY_DRAIN.to_owned(), memory_model)
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

fn canonical(mut classes: Vec<RefinementCatalogClass>) -> Vec<RefinementCatalogClass> {
    let model =
        |class: &RefinementCatalogClass| class.memory_model.map_or("", MemoryModelName::label);
    classes.sort_by(|left, right| {
        (left.class.as_bytes(), model(left).as_bytes())
            .cmp(&(right.class.as_bytes(), model(right).as_bytes()))
    });
    classes
}

/// Add the selected model's declared operation classes, preserving each
/// checked operation key and its parameter types.
pub(super) fn add_model_operations(
    classes: &mut BTreeMap<CatalogKey, RefinementCatalogClass>,
    alias: &str,
    model: &super::super::lowering::AdmittedModel,
    operations: &crate::model::operation::OperationTable,
    types: &quire_semantic_value::declaration::TypeEnvironment,
) -> Result<(), quire_semantic_value::catalog::InternalFault> {
    if model.alias() != Some(alias) {
        return Err(quire_semantic_value::catalog::InternalFault::new(
            "check.refinement-class-catalog",
            "selected-model-alias-matches-catalog-subject",
        ));
    }
    for (owner_key, owner) in model.object_types() {
        if types.object_type(owner).is_none() {
            return Err(quire_semantic_value::catalog::InternalFault::new(
                "check.refinement-class-catalog",
                "selected-model-object-has-checked-type",
            ));
        }
        let Some(owner_name) =
            crate::model::intake::type_identity_segment(&owner_key.package, &owner_key.node)
        else {
            return Err(quire_semantic_value::catalog::InternalFault::new(
                "check.refinement-class-catalog",
                "selected-model-object-has-alias-free-identity-segment",
            ));
        };
        for operation in operations.declared_by(owner) {
            let operation_key = model.operation_key(&crate::check::state_clause::ClauseOperation {
                declaring: owner,
                declaration: operation.clone(),
            });
            let Some(operation_key) = operation_key else {
                return Err(quire_semantic_value::catalog::InternalFault::new(
                    "check.refinement-class-catalog",
                    "selected-model-operation-has-declaration-key",
                ));
            };
            if operation_key.package != model.selection().identity {
                return Err(quire_semantic_value::catalog::InternalFault::new(
                    "check.refinement-class-catalog",
                    "operation-key-belongs-to-selected-model",
                ));
            }
            let class = format!("{owner_name}::{}", operation.name());
            let binder_types = operation
                .parameters()
                .iter()
                .map(|(_, value_type)| value_type.clone())
                .collect();
            let entry = RefinementCatalogClass::declared(
                class.clone(),
                RefinementCatalogNode::Operation(operation_key.clone()),
                binder_types,
            );
            if classes.insert(entry.catalog_key(), entry).is_some() {
                return Err(quire_semantic_value::catalog::InternalFault::new(
                    "check.refinement-class-catalog",
                    "selected-model-operation-classes-are-unique",
                ));
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
        let ValueType::Reference(context) = over else {
            continue;
        };
        if !concrete_model
            .object_types()
            .any(|(_, object)| object == context)
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
