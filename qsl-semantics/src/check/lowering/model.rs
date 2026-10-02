// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-094: model-owned nodes, the `Reference<T>` and
//! `Population<T>[N]` type nodes, clause-function owners and quantity type
//! nodes.
//!
//! A model declaration node stands for one domain package declaration. It
//! is keyed by `quire.structural-node/v1` with QSpec's `ModelOwner` (the
//! declaring package's `DomainPackageRef` identity and version, and the
//! declaration's IR node identity), a `null` `declaration` and an empty
//! body. A `Reference<T>` carries `T` as an `EffectiveId` (ADR-013 O-05);
//! `check` maps it to its `DeclarationKey` through the admitted effective
//! view's `type_identities` and keys the node by that key, so no preimage
//! member is computed from an `EffectiveId` (FR-094-CON-2). Each model
//! declaration node `check` keys is one model correspondence entry, and
//! `check` is the correspondence's only writer.

use std::collections::BTreeMap;

use quire_exact::{EffectiveId, Integer, NodeKey, UnitDomain, UnitId, ValueType};

use super::{refuse, Lowering};
use crate::check::node_key::{ModelOwner, NodeTag, Owner, SemanticTerm};
use crate::check::refusal::{CheckCause, CheckRefusal, KeyFault};
use crate::model::domain_package::{DomainPackage, DomainPackageRecord, DomainPackageRef};
use crate::model::key::DeclarationKey;
use crate::model::normalize::EffectiveView;
use qsl_forms::DeclaredClauseKind;
use quire_semantic_value::location::Location;
use quire_semantic_value::quantity::QuantityUnit;

/// One admitted domain package, as `check` keys its declarations (FR-094
/// "Inputs"): its model selection, its records by declaration key, and
/// its effective view's `type_identities`, read from `EffectiveId` to
/// `DeclarationKey`; and the alias of the `model` declaration that selects
/// it, when one does.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdmittedModel {
    alias: Option<String>,
    selection: DomainPackageRef,
    records: BTreeMap<DeclarationKey, DomainPackageRecord>,
    types: BTreeMap<EffectiveId, DeclarationKey>,
}

/// Why an [`AdmittedModel`] cannot be formed: the effective view was
/// normalized under another model selection than the domain package's.
/// Both selections are boxed, keeping the error small.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[error("the effective view's model selection {view:?} is not the domain package's {package:?}")]
pub struct ForeignView {
    /// The domain package's own selection.
    pub package: Box<DomainPackageRef>,
    /// The view's selection.
    pub view: Box<DomainPackageRef>,
}

impl AdmittedModel {
    /// `domain_package`, admitted with `view`, its own effective view.
    pub fn new(domain_package: &DomainPackage, view: &EffectiveView) -> Result<Self, ForeignView> {
        if view.model_selection() != &domain_package.model_selection {
            return Err(ForeignView {
                package: Box::new(domain_package.model_selection.clone()),
                view: Box::new(view.model_selection().clone()),
            });
        }
        Ok(Self::assemble(domain_package, view))
    }

    /// `view`'s own domain package, admitted with `view`: the package the
    /// view was normalized from, so there is no second package whose
    /// selection could differ.
    pub fn from_view(view: &EffectiveView) -> Self {
        Self::assemble(view.domain_package(), view)
    }

    fn assemble(domain_package: &DomainPackage, view: &EffectiveView) -> Self {
        Self {
            alias: None,
            selection: domain_package.model_selection.clone(),
            records: domain_package
                .records
                .iter()
                .map(|record| (record.key().clone(), record.clone()))
                .collect(),
            types: view
                .type_identities()
                .iter()
                .map(|(key, id)| (*id, key.clone()))
                .collect(),
        }
    }

    /// `domain_package` with the given `EffectiveId` of each object type,
    /// for a fixture whose object types carry fixed identities rather than
    /// ones an effective view computed.
    #[cfg(any(test, feature = "test-support"))]
    pub fn fixture(
        domain_package: &DomainPackage,
        types: impl IntoIterator<Item = (EffectiveId, DeclarationKey)>,
    ) -> Self {
        Self {
            alias: None,
            selection: domain_package.model_selection.clone(),
            records: domain_package
                .records
                .iter()
                .map(|record| (record.key().clone(), record.clone()))
                .collect(),
            types: types.into_iter().collect(),
        }
    }

    /// This package admitted for the `model` declaration `alias`: FR-115's
    /// `Frame` selection resolves `alias` to this package.
    #[must_use]
    pub fn with_alias(mut self, alias: impl Into<String>) -> Self {
        self.alias = Some(alias.into());
        self
    }

    /// The alias of the `model` declaration this package is admitted for,
    /// if any.
    pub(crate) fn alias(&self) -> Option<&str> {
        self.alias.as_deref()
    }

    /// The domain package's model selection.
    pub fn selection(&self) -> &DomainPackageRef {
        &self.selection
    }

    /// Every object type of this package: its declaration key and its
    /// effective identity.
    pub(crate) fn object_types(&self) -> impl Iterator<Item = (&DeclarationKey, EffectiveId)> {
        self.types
            .iter()
            .filter(|(_, key)| {
                matches!(
                    self.records.get(*key),
                    Some(DomainPackageRecord::ObjectType(_))
                )
            })
            .map(|(id, key)| (key, *id))
    }

    /// The record `key` names in this package.
    pub(super) fn record(&self, key: &DeclarationKey) -> Option<&DomainPackageRecord> {
        self.records.get(key)
    }

    /// FR-104: every population declaration of this package that covers the
    /// object type `target`: one of its member types is `target` itself or
    /// a proper supertype of it, by `conforms` (FR-084's `allInstances<T>`
    /// conformance, not exact identity -- a clause over a subtype still
    /// ranges over its supertype's population). Each is paired with its
    /// ordinal among the package's population declarations in ascending
    /// `DeclarationKey` order (its own `Ord`: `package`, then `node`), and
    /// the `EffectiveId` of the population's *canonical* member type: the
    /// least of its declared member types in ascending `DeclarationKey`
    /// order, whatever the clause's context (SR-736 FND-010, FND-011). A
    /// population with several declared member types would otherwise key
    /// its one domain by a different node per covering member -- one
    /// canonical node per population, chosen without regard to which member
    /// actually covers `target`, keeps every clause over any member giving
    /// the same domain key. The ordinal is stable within this package's
    /// digest only. A domain package population declares no maximum (the
    /// Semantic IR `population` record has no such member), so each one is
    /// an unbounded `Population(None)` domain.
    pub(crate) fn populations_of(
        &self,
        target: EffectiveId,
        conforms: impl Fn(EffectiveId, EffectiveId) -> bool,
    ) -> Vec<(usize, &DeclarationKey, EffectiveId)> {
        let covers = |member: &DeclarationKey| {
            self.types.iter().any(|(id, declared)| {
                declared == member && (*id == target || conforms(target, *id))
            })
        };
        // `records` is a `BTreeMap` keyed by `DeclarationKey`, so its
        // population records iterate in ascending key order.
        self.records
            .values()
            .filter_map(|record| match record {
                DomainPackageRecord::Population(population) => Some(population),
                _ => None,
            })
            .enumerate()
            .filter_map(|(ordinal, population)| {
                if !population.member_types.iter().any(covers) {
                    return None;
                }
                // The canonical member: the least declared member type, by
                // `DeclarationKey`, regardless of which one covers `target`.
                let canonical = population.member_types.iter().min()?;
                let node = self
                    .types
                    .iter()
                    .find_map(|(id, declared)| (declared == canonical).then_some(*id))?;
                Some((ordinal, &population.key, node))
            })
            .collect()
    }
}

/// A clause function's FR-094 owner and clause kind: the declaration whose
/// clause it is (the authoring operation member for an authored
/// precondition, the candidate operation member for an effective
/// precondition and a body, the object type for an invariant), and the
/// kind its `clause` binding spells.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ModelClause {
    /// The declaration whose clause this is.
    pub declaration: DeclarationKey,
    /// The clause kind.
    pub kind: DeclaredClauseKind,
}

fn fault(location: &Location, fault: KeyFault) -> CheckRefusal {
    refuse(location, CheckCause::InternalFault(Box::new(fault)))
}

/// FR-094's `clause` binding value. No postcondition arm:
/// `DeclaredClauseKind` has none (FR-094-CON-1).
fn clause_spelling(kind: DeclaredClauseKind) -> &'static str {
    match kind {
        DeclaredClauseKind::Precondition => "precondition",
        DeclaredClauseKind::Body => "body",
        DeclaredClauseKind::Invariant => "invariant",
    }
}

/// A domain package record's model node tag and form, or `None` for a
/// record kind no checked node names (FR-094 "Model declaration nodes";
/// one arm per record kind, FR-094-CON-1).
fn record_form(record: &DomainPackageRecord) -> Option<(NodeTag, &'static str)> {
    match record {
        DomainPackageRecord::ObjectType(object) => Some(match object.interface_features {
            None => (NodeTag::Model, "object_type"),
            Some(_) => (NodeTag::Model, "systems_interface"),
        }),
        DomainPackageRecord::Relationship(_) => Some((NodeTag::Relation, "relationship")),
        DomainPackageRecord::FieldMember(_)
        | DomainPackageRecord::RecordValueType(_)
        | DomainPackageRecord::ScalarType(_)
        | DomainPackageRecord::OperationMember(_)
        | DomainPackageRecord::Component(_)
        | DomainPackageRecord::Endpoint(_)
        | DomainPackageRecord::Allocation(_)
        | DomainPackageRecord::Population(_) => None,
    }
}

impl<'a> Lowering<'a> {
    /// The `ModelOwner` of `declaration`: its admitted domain package's
    /// identity and version, and its IR node identity.
    pub(super) fn model_owner(
        &self,
        declaration: &DeclarationKey,
        location: &Location,
    ) -> Result<(ModelOwner, &'a AdmittedModel), CheckRefusal> {
        let models: &'a [AdmittedModel] = self.models;
        let model = models
            .iter()
            .find(|model| model.selection.identity == declaration.package)
            .ok_or_else(|| fault(location, KeyFault::UnadmittedPackage(declaration.clone())))?;
        let owner = ModelOwner::new(
            model.selection.identity.clone(),
            model.selection.version.clone(),
            declaration.node.clone(),
        )
        .map_err(|_| fault(location, KeyFault::EmptyNode(declaration.clone())))?;
        Ok((owner, model))
    }

    /// The model declaration node of `declaration`, recorded in the model
    /// correspondence.
    pub(super) fn model_node(
        &mut self,
        declaration: &DeclarationKey,
        location: &Location,
    ) -> Result<NodeKey, CheckRefusal> {
        let (owner, model) = self.model_owner(declaration, location)?;
        let record = model
            .records
            .get(declaration)
            .ok_or_else(|| fault(location, KeyFault::UnknownDeclaration(declaration.clone())))?;
        let (node_tag, form) = record_form(record)
            .ok_or_else(|| fault(location, KeyFault::UnnamedRecordKind(declaration.clone())))?;
        let key = self.insert_owned(
            location,
            node_tag,
            form,
            None,
            Owner::Model(owner),
            SemanticTerm::Aggregate {
                members: Vec::new(),
            },
        )?;
        self.correspondence
            .record(key, declaration.clone())
            .map_err(|conflict| fault(location, KeyFault::CorrespondenceConflict(conflict)))?;
        Ok(key)
    }

    /// The model node of the object type `T` a `Reference<T>` names.
    pub(super) fn object_node(
        &mut self,
        target: EffectiveId,
        location: &Location,
    ) -> Result<NodeKey, CheckRefusal> {
        let declaration = self.declaration_key_of(target, location)?;
        self.model_node(&declaration, location)
    }

    /// The `DeclarationKey` `target` names in its admitted domain package
    /// (FR-104 "Requirements", SR-736 FND-008): its own `Ord` (`package`,
    /// then `node`, each as UTF-8 bytes) is the ascending order a frame
    /// occurrence's ordinal follows, independent of clause or source order.
    pub(super) fn declaration_key_of(
        &self,
        target: EffectiveId,
        location: &Location,
    ) -> Result<DeclarationKey, CheckRefusal> {
        self.models
            .iter()
            .find_map(|model| model.types.get(&target))
            .cloned()
            .ok_or_else(|| fault(location, KeyFault::UnknownEffectiveId(target)))
    }

    /// The model node of the object type a `Reference<T>`-typed value
    /// names, for a model row's member (FR-093).
    pub(super) fn referenced_object(
        &mut self,
        value_type: &ValueType,
        location: &Location,
    ) -> Result<NodeKey, CheckRefusal> {
        match value_type {
            ValueType::Reference(target) => self.object_node(*target, location),
            ValueType::Option(payload) => match payload.as_ref() {
                ValueType::Reference(target) => self.object_node(*target, location),
                _ => Err(mismatch(location)),
            },
            ValueType::Collection(collection) => match collection.element() {
                ValueType::Reference(target) => self.object_node(*target, location),
                _ => Err(mismatch(location)),
            },
            _ => Err(mismatch(location)),
        }
    }

    /// `Reference<T>`: `composite_type`/`reference` over `T`'s model node.
    pub(super) fn reference_type(
        &mut self,
        target: EffectiveId,
        location: &Location,
    ) -> Result<NodeKey, CheckRefusal> {
        let object = self.object_node(target, location)?;
        self.insert(
            location,
            NodeTag::CompositeType,
            "reference",
            None,
            None,
            SemanticTerm::Aggregate {
                members: vec![SemanticTerm::reference(object)],
            },
        )
    }

    /// `Population<T>[N]`: `bounded_domain`/`model_population` over the
    /// `Set<Reference<T>>` node, binding `max` = `N`.
    ///
    /// A population with no declared maximum refuses
    /// (`CheckCause::UnrepresentableBound`). Its bare `Set<Reference<T>>`
    /// node would be the same node as an unbounded `Set<Reference<T>>`, so
    /// the population role would be lost. No source spells `Population<T>`
    /// without a maximum until QSL-42, which gives it its own node.
    pub(super) fn population_type(
        &mut self,
        target: EffectiveId,
        maximum: Option<u64>,
        location: &Location,
    ) -> Result<NodeKey, CheckRefusal> {
        let Some(maximum) = maximum else {
            return Err(refuse(location, CheckCause::UnrepresentableBound));
        };
        let reference = self.reference_type(target, location)?;
        let set = self.insert(
            location,
            NodeTag::CompositeType,
            "set",
            None,
            None,
            SemanticTerm::Aggregate {
                members: vec![SemanticTerm::reference(reference)],
            },
        )?;
        let max = self.integer_literal(Integer::from(maximum), location)?;
        self.bounded("model_population", set, vec![("max", max)], location)
    }

    /// A quantity's type node: a declared unit's nominal node, or a
    /// compound unit's `scalar_type`/`compound_unit` node.
    pub(super) fn quantity_type(
        &mut self,
        unit: UnitId,
        location: &Location,
    ) -> Result<NodeKey, CheckRefusal> {
        match unit.domain() {
            UnitDomain::Declared => Ok(NodeKey::from_digest(*unit.as_bytes())),
            UnitDomain::Compound => {
                let terms: Vec<(NodeKey, Integer)> = match self.units.get(unit) {
                    Some(QuantityUnit::Compound(compound)) => compound
                        .terms()
                        .map(|(root, exponent)| (root, exponent.clone()))
                        .collect(),
                    Some(QuantityUnit::Declared(_)) | None => {
                        return Err(fault(location, KeyFault::UnheldUnit(unit)))
                    }
                };
                let mut members = Vec::with_capacity(terms.len());
                for (root, exponent) in terms {
                    let exponent = self.integer_literal(exponent, location)?;
                    members.push(SemanticTerm::Aggregate {
                        members: vec![
                            SemanticTerm::binding("unit", SemanticTerm::reference(root)),
                            SemanticTerm::binding("exponent", exponent),
                        ],
                    });
                }
                self.insert(
                    location,
                    NodeTag::ScalarType,
                    "compound_unit",
                    None,
                    None,
                    SemanticTerm::Aggregate { members },
                )
            }
        }
    }

    /// A clause function's `ModelOwner` and trailing `clause` binding.
    pub(super) fn clause_owner(
        &mut self,
        clause: &ModelClause,
        location: &Location,
    ) -> Result<(Owner, SemanticTerm), CheckRefusal> {
        let (owner, _) = self.model_owner(&clause.declaration, location)?;
        let spelling = self.text_literal(clause_spelling(clause.kind), location)?;
        Ok((
            Owner::Model(owner),
            SemanticTerm::binding("clause", spelling),
        ))
    }
}

fn mismatch(location: &Location) -> CheckRefusal {
    refuse(
        location,
        CheckCause::IllTyped(quire_exact::IllTypedCause::TypeMismatch),
    )
}

#[cfg(test)]
mod tests;
