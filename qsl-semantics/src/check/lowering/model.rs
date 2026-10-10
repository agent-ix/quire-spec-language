// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-094: model-owned nodes, the `Reference<T>` and
//! `Population<T>[N]` type nodes, clause-function owners and quantity type
//! nodes.
//!
//! A model declaration node stands for one domain package declaration. It
//! is keyed by `quire.structural-node/v1` with QSpec's content-only
//! `ModelOwner` (the declaring package's `DomainPackageRef` identity and
//! the declaration's IR node identity, no version), a `null` `declaration`
//! and an empty body. A `Reference<T>` carries `T` as an `EffectiveId` (ADR-013 O-05);
//! `check` maps it to its `DeclarationKey` through the admitted effective
//! view's `type_identities` and keys the node by that key, so no preimage
//! member is computed from an `EffectiveId` (FR-094-CON-2). Each model
//! declaration node `check` keys is one model correspondence entry, and
//! `check` is the correspondence's only writer.

use std::collections::BTreeMap;

use quire_exact::{EffectiveId, Integer, NodeKey, UnitDomain, UnitId, ValueType};

use super::{identifiers, refuse, type_declaration, Lowering, NodeContent, NominalNode};
use crate::check::family::OccurrenceRole;
use crate::check::node_key::{
    Binding, BodyTerm, GroupMember, GroupTerm, LeafTerm, MemberTerm, ModelOwner, NodeTag, Owner,
};
use crate::check::refusal::{CheckCause, CheckRefusal, KeyFault};
use crate::model::domain_package::{
    DomainPackage, DomainPackageRecord, DomainPackageRef, PopulationRecord,
};
use crate::model::key::DeclarationKey;
use crate::model::normalize::EffectiveView;
use crate::value::unit::{DimensionPreimage, UnitPreimage};
use qsl_forms::DeclaredClauseKind;
use quire_semantic_value::location::Location;
use quire_semantic_value::quantity::QuantityUnit;
use quire_semantic_value::unit::{NominalDeclaration, NominalUnitForm};

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

    /// The key of `declaration`'s model node, keyed from the same content
    /// [`Lowering::model_node`] keys, without adding a node to any graph:
    /// a model node's key depends on its declaration alone, so this names
    /// the node whether or not the package's own lowering minted it.
    /// `None` when this package does not declare `declaration`, or declares
    /// it as a record kind no checked node names.
    pub(crate) fn model_node_key(
        &self,
        declaration: &DeclarationKey,
        identity: qsl_foundation::IdentityLimits,
    ) -> Option<NodeKey> {
        if self.selection.identity != declaration.package {
            return None;
        }
        let (node_tag, form) = record_form(self.records.get(declaration)?)?;
        let owner =
            ModelOwner::new(self.selection.identity.clone(), declaration.node.clone()).ok()?;
        crate::check::node_key::node_key(
            &model_node_content(owner, node_tag, form).input(),
            identity,
        )
        .ok()
        .map(|keyed| keyed.key)
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
        self.population_domains()
            .filter(|(_, population, _)| population.member_types.iter().any(covers))
            .map(|(ordinal, population, canonical)| (ordinal, &population.key, canonical))
            .collect()
    }

    /// Every population declaration of this package, in ascending
    /// `DeclarationKey` order, with its ordinal in that order and the
    /// `EffectiveId` of its canonical member type (the least declared
    /// member type by `DeclarationKey`): the ordering and canonical member
    /// [`Self::populations_of`] keys a covering population by, and the one
    /// `CheckedGraph::population_domain` names a population by. A
    /// population whose canonical member is not an admitted type is
    /// skipped, keeping its ordinal.
    pub(crate) fn population_domains(
        &self,
    ) -> impl Iterator<Item = (usize, &PopulationRecord, EffectiveId)> {
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
                // The canonical member: the least declared member type, by
                // `DeclarationKey`, regardless of which one covers a target.
                let canonical = population.member_types.iter().min()?;
                let node = self
                    .types
                    .iter()
                    .find_map(|(id, declared)| (declared == canonical).then_some(*id))?;
                Some((ordinal, population, node))
            })
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

/// A model declaration node's content: QSpec's `ModelOwner`, the record's
/// node tag and form, no semantic type, a `null` `declaration` and an empty
/// body. The one definition [`Lowering::model_node`] keys and adds and
/// [`AdmittedModel::model_node_key`] only keys.
fn model_node_content(owner: ModelOwner, node_tag: NodeTag, form: &'static str) -> NodeContent {
    NodeContent {
        node_tag,
        semantic_form: form,
        semantic_type: None,
        declaration: None,
        owner: Some(Owner::Model(owner)),
        body: BodyTerm::aggregate(Vec::new()),
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
        | DomainPackageRecord::Clause(_)
        | DomainPackageRecord::Namespace(_)
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
    /// identity and its IR node identity.
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
        let owner = ModelOwner::new(model.selection.identity.clone(), declaration.node.clone())
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
        let key = self.insert_node(location, model_node_content(owner, node_tag, form))?;
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
            BodyTerm::aggregate(vec![MemberTerm::reference(object)]),
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
            BodyTerm::aggregate(vec![MemberTerm::reference(reference)]),
        )?;
        let max = self.integer_literal(Integer::from(maximum), location)?;
        self.bounded("model_population", set, vec![("max", max)], location)
    }

    /// A quantity's type node: a declared unit's nominal node, or a
    /// compound unit's `scalar_type`/`compound_unit` node. Either way the
    /// graph then holds every unit and dimension node the type names.
    pub(super) fn quantity_type(
        &mut self,
        unit: UnitId,
        location: &Location,
    ) -> Result<NodeKey, CheckRefusal> {
        match unit.domain() {
            UnitDomain::Declared => {
                let key = NodeKey::from_digest(*unit.as_bytes());
                self.unit_nodes(key, location)?;
                Ok(key)
            }
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
                    self.unit_nodes(root, location)?;
                    let exponent = self.integer_literal(exponent, location)?;
                    members.push(MemberTerm::Group(GroupTerm::new(vec![
                        GroupMember::Binding(Binding::new("unit", LeafTerm::reference(root))),
                        GroupMember::Binding(Binding::new("exponent", exponent)),
                    ])));
                }
                self.insert(
                    location,
                    NodeTag::ScalarType,
                    "compound_unit",
                    None,
                    None,
                    BodyTerm::aggregate(members),
                )
            }
        }
    }

    /// FR-094, FR-092 rule 1: add the declared unit node `unit` to the
    /// graph, and every node it names: its dimension, that dimension's base
    /// dimensions and its target units. Each is built from the admitted node
    /// the unit table holds for it, keyed by its admitted preimage bytes, as
    /// QSpec's `scalar_type`/`unit` or `scalar_type`/`dimension` node: its
    /// preimage's qualified name as its `declaration`, with a `declaration`
    /// occurrence at that declared name, an empty body, and a unit typed by
    /// its dimension. Its nominal preimage is rebuilt under the checked
    /// unit's own source owner and kept when its bytes are the admitted ones;
    /// a node another owner declares keeps none. A node already in the graph
    /// is left as it is. The walk is a heap work list, bounded by the unit
    /// table's size.
    fn unit_nodes(&mut self, unit: NodeKey, location: &Location) -> Result<(), CheckRefusal> {
        let owner = self.owner.node_owner();
        let mut pending = vec![unit];
        while let Some(key) = pending.pop() {
            if self.graph.nodes.contains_key(&key) {
                continue;
            }
            let Some(node) = self.units.nominal_node(key.as_bytes()).cloned() else {
                return Err(fault(location, KeyFault::UnheldNominal(key)));
            };
            let NominalDeclaration {
                qualified_declaration: qualified,
                preimage: bytes,
            } = node.declaration;
            let site = type_declaration(&qualified.join("."));
            let own = |rebuilt: Result<Vec<u8>, _>| rebuilt.is_ok_and(|rebuilt| rebuilt == bytes);
            let (semantic_form, semantic_type, nominal) = match node.form {
                NominalUnitForm::Unit {
                    dimension,
                    target,
                    scale,
                    offset,
                } => {
                    pending.push(dimension);
                    pending.extend(target);
                    let preimage = UnitPreimage::new(
                        owner.clone(),
                        qualified.clone(),
                        dimension,
                        target,
                        &scale,
                        &offset,
                    )
                    .ok()
                    .filter(|preimage| own(preimage.preimage_bytes(self.identity)));
                    ("unit", Some(dimension), NominalNode::Unit(preimage))
                }
                NominalUnitForm::Dimension { terms } => {
                    pending.extend(terms.iter().map(|(base, _)| *base));
                    let preimage = DimensionPreimage::new(owner.clone(), qualified.clone(), terms)
                        .ok()
                        .filter(|preimage| own(preimage.preimage_bytes(self.identity)));
                    ("dimension", None, NominalNode::Dimension(preimage))
                }
            };
            let declaration = identifiers(qualified.iter().map(String::as_str), &site)?;
            let inserted = self.insert_nominal(
                &site,
                key,
                bytes,
                NodeContent {
                    node_tag: NodeTag::ScalarType,
                    semantic_form,
                    semantic_type,
                    declaration: Some(declaration),
                    owner: None,
                    body: BodyTerm::aggregate(Vec::new()),
                },
                nominal,
            )?;
            if inserted {
                self.record(key, OccurrenceRole::Declaration, site);
            }
        }
        Ok(())
    }

    /// A clause function's `ModelOwner` and trailing `clause` binding.
    pub(super) fn clause_owner(
        &mut self,
        clause: &ModelClause,
        location: &Location,
    ) -> Result<(Owner, MemberTerm), CheckRefusal> {
        let (owner, _) = self.model_owner(&clause.declaration, location)?;
        let spelling = self.text_literal(clause_spelling(clause.kind), location)?;
        Ok((Owner::Model(owner), MemberTerm::bound("clause", spelling)))
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
