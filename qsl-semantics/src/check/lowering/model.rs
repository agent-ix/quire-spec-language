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
use crate::check::VariantTypeDeclaration;
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
    index: std::sync::Arc<crate::model::index::ModelIndex>,
    /// Each type's direct supertypes, from the inherit facts of its effective
    /// declaration; ancestry beyond one edge is followed on demand.
    supertypes: BTreeMap<DeclarationKey, Vec<DeclarationKey>>,
    /// The authored bound (FR-380) of each population this package's
    /// `bound` declarations name, by the population's declaration key.
    /// Empty until the assembler resolves the unit's bounds.
    population_bounds: BTreeMap<DeclarationKey, AuthoredBound>,
}

/// The `Origin::TypeDeclaration` name the `[N]` of the `bound` declaration
/// labelled `label` is located under. Spaced, so it names no declared type.
pub(crate) fn bound_origin(label: &str) -> String {
    format!("bound {label}")
}

/// An authored finite maximum of one domain-package population (FR-380):
/// part of the population's meaning, never a resource limit (ADR-014 B-1).
/// A population the unit does not bound has none.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct AuthoredBound {
    /// The inclusive maximum object count `N`.
    pub maximum: u64,
    /// The `bound` declaration's label, which names its `[N]` occurrence.
    pub label: String,
    /// The bounded population's declaration key.
    pub population: DeclarationKey,
}

/// One lineage's parent member, as the staged admission selects it.
pub(crate) struct ParentSlot {
    /// The member's identity in the staged admission.
    pub(crate) field: quire_semantic_value::declaration::FieldRef,
    /// Its record, when a record declares it.
    pub(crate) key: Option<DeclarationKey>,
    /// Its declared value type.
    pub(crate) value_type: ValueType,
    /// Its declared presence.
    pub(crate) presence: quire_exact::Presence,
}

/// Why a parent slot could not be read.
pub(crate) enum ParentSlotFailure {
    /// A conformance walk reached its ancestor-edge ceiling.
    Incomplete(crate::model::accounting::Incomplete),
    /// The staged admission refused the read.
    Pending(quire_semantic_value::declaration::PendingFieldFailure),
}

impl From<crate::model::accounting::Incomplete> for ParentSlotFailure {
    fn from(incomplete: crate::model::accounting::Incomplete) -> Self {
        Self::Incomplete(incomplete)
    }
}

impl From<quire_semantic_value::declaration::PendingFieldFailure> for ParentSlotFailure {
    fn from(failure: quire_semantic_value::declaration::PendingFieldFailure) -> Self {
        Self::Pending(failure)
    }
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
    pub(crate) fn model_index(&self) -> &crate::model::index::ModelIndex {
        &self.index
    }

    pub(crate) fn declaration_key(&self, identity: EffectiveId) -> Option<&DeclarationKey> {
        self.types.get(&identity)
    }

    pub(crate) fn field_reference(
        &self,
        key: &DeclarationKey,
    ) -> Option<quire_semantic_value::declaration::FieldRef> {
        let DomainPackageRecord::FieldMember(field) = self.records.get(key)? else {
            return None;
        };
        let (owner, _) = self.types.iter().find(|(_, key)| *key == &field.owner)?;
        let name = crate::model::intake::member_identity_name(&field.owner.node, &field.key.node)?;
        Some(quire_semantic_value::declaration::FieldRef::new(
            *owner, name,
        ))
    }

    /// The effective identity of the object type declared under `key`.
    pub(crate) fn effective_id(&self, key: &DeclarationKey) -> Option<EffectiveId> {
        self.types
            .iter()
            .find_map(|(identity, declared)| (declared == key).then_some(*identity))
    }

    /// The field record a staged admission's field identity stands for.
    pub(crate) fn field_key(
        &self,
        field: &quire_semantic_value::declaration::FieldRef,
    ) -> Option<&DeclarationKey> {
        let owner = self.types.get(&field.owner)?;
        self.records.values().find_map(|record| match record {
            DomainPackageRecord::FieldMember(member)
                if &member.owner == owner
                    && crate::model::intake::member_identity_name(
                        &owner.node,
                        &member.key.node,
                    ) == Some(field.name.as_str()) =>
            {
                Some(&member.key)
            }
            _ => None,
        })
    }

    pub(crate) fn operation_key(
        &self,
        operation: &crate::check::state_clause::ClauseOperation,
    ) -> Option<&DeclarationKey> {
        let owner = self.types.get(&operation.declaring)?;
        self.index
            .operations()
            .find(|record| {
                &record.owner == owner
                    && crate::model::intake::member_identity_name(
                        &record.owner.node,
                        &record.key.node,
                    ) == Some(operation.declaration.name())
            })
            .map(|record| &record.key)
    }

    pub(crate) fn reaches_owner(
        &self,
        specific: &DeclarationKey,
        general: &DeclarationKey,
    ) -> bool {
        if specific == general {
            return true;
        }
        let mut visited: std::collections::BTreeSet<&DeclarationKey> =
            std::collections::BTreeSet::new();
        let mut pending = vec![specific];
        while let Some(at) = pending.pop() {
            for parent in self.supertypes.get(at).into_iter().flatten() {
                if parent == general {
                    return true;
                }
                if visited.insert(parent) {
                    pending.push(parent);
                }
            }
        }
        false
    }

    pub(crate) fn narrows_value_type(
        &self,
        child: &crate::model::domain_package::ValueTypeRef,
        parent: &crate::model::domain_package::ValueTypeRef,
    ) -> bool {
        match (child.as_package(), parent.as_package()) {
            (Some(child), Some(parent)) if child != parent => {
                match (
                    self.index.scalar_bounds(child),
                    self.index.scalar_bounds(parent),
                ) {
                    (Some((lower, upper)), Some((parent_lower, parent_upper))) => {
                        parent_lower <= lower && upper <= parent_upper
                    }
                    (None, None) => self.reaches_owner(child, parent),
                    _ => false,
                }
            }
            _ => false,
        }
    }

    /// The parent members `target` is judged against from the redefiner's
    /// `owner`, one per lineage: for each direct supertype of `owner` from
    /// which `target`'s owner is reached, the member the staged admission
    /// selects through that first edge. The declared target stands alone
    /// when no edge reaches its owner. A slot two lineages share appears once.
    pub(crate) fn parent_slots(
        &self,
        staged: &mut quire_semantic_value::declaration::PendingTypeEnvironment,
        owner: &DeclarationKey,
        target: &DeclarationKey,
        ancestor_steps: u64,
    ) -> Result<Vec<ParentSlot>, ParentSlotFailure> {
        let (Some(target_owner), Some(target_ref), Some(child)) = (
            self.index.member_owner(target),
            self.field_reference(target),
            self.effective_id(owner),
        ) else {
            return Ok(Vec::new());
        };
        // Only object types are staged; a record value type's redefinitions
        // are judged against the declared target alone.
        if !self.index.is_object_type(owner) || !self.index.is_object_type(target_owner) {
            return Ok(Vec::new());
        }
        let mut slots: Vec<ParentSlot> = Vec::new();
        for edge in self.index.direct_supertypes(owner) {
            if edge != target_owner
                && !self
                    .index
                    .conformance_walk(edge, target_owner, ancestor_steps)?
            {
                continue;
            }
            let Some(parent) = self.effective_id(edge) else {
                continue;
            };
            let slot = staged.parent_field(child, parent, &target_ref, (), ())?;
            if slots.iter().any(|known| known.field == slot.field) {
                continue;
            }
            slots.push(ParentSlot {
                key: self.field_key(&slot.field).cloned(),
                value_type: slot.value_type.clone(),
                presence: slot.presence,
                field: slot.field,
            });
        }
        Ok(slots)
    }

    /// The parent's declared type and presence of each field the clause's
    /// operation writes, read through the staged admission and keyed by the
    /// context type's own field. Where lineages differ, the type is the first
    /// lineage's and the field stays optional if any lineage is optional.
    pub(crate) fn parent_projections(
        &self,
        context: EffectiveId,
        operation: &crate::check::state_clause::ClauseOperation,
        staged: &mut quire_semantic_value::declaration::PendingTypeEnvironment,
        ancestor_steps: u64,
    ) -> Result<
        BTreeMap<quire_semantic_value::declaration::FieldRef, (ValueType, quire_exact::Presence)>,
        ParentSlotFailure,
    > {
        use crate::model::intake::member_identity_name;
        use quire_semantic_value::declaration::FieldRef;

        let (Some(context_key), Some(operation_owner)) = (
            self.types.get(&context),
            self.types.get(&operation.declaring),
        ) else {
            return Ok(BTreeMap::new());
        };
        let writer = self.records.values().find_map(|record| match record {
            DomainPackageRecord::OperationMember(writer)
                if &writer.owner == operation_owner
                    && member_identity_name(&writer.owner.node, &writer.key.node)
                        == Some(operation.declaration.name()) =>
            {
                Some(writer)
            }
            _ => None,
        });
        let Some(writer) = writer else {
            return Ok(BTreeMap::new());
        };
        let mut projections = BTreeMap::new();
        for record in self.records.values() {
            let DomainPackageRecord::FieldMember(field) = record else {
                continue;
            };
            if &field.owner != context_key {
                continue;
            }
            let Some(target) = &field.redefines else {
                continue;
            };
            if !self.index.field_lineage(&field.key).iter().any(|member| {
                writer
                    .effect
                    .written_fields()
                    .any(|written| written == *member)
            }) {
                continue;
            }
            let slots = self.parent_slots(staged, &field.owner, target, ancestor_steps)?;
            let Some(first) = slots.first() else {
                continue;
            };
            let Some(name) = member_identity_name(&field.owner.node, &field.key.node) else {
                continue;
            };
            let presence = if slots
                .iter()
                .any(|slot| slot.presence == quire_exact::Presence::Optional)
            {
                quire_exact::Presence::Optional
            } else {
                quire_exact::Presence::Required
            };
            projections.insert(
                FieldRef::new(context, name),
                (first.value_type.clone(), presence),
            );
        }
        Ok(projections)
    }

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
            index: view.shared_model_index().clone(),
            supertypes: view
                .declarations()
                .iter()
                .filter(|entry| entry.preimage.owner_effective_type.is_none())
                .map(|entry| {
                    (
                        entry.preimage.original.clone(),
                        entry
                            .preimage
                            .derivation
                            .iter()
                            .filter(|fact| fact.rule == crate::model::key::RULE_INHERIT)
                            .filter_map(|fact| fact.inputs.iter().last().cloned())
                            .collect(),
                    )
                })
                .collect(),
            alias: None,
            population_bounds: BTreeMap::new(),
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
            index: std::sync::Arc::new(crate::model::index::ModelIndex::build(
                domain_package.clone(),
            )),
            supertypes: BTreeMap::new(),
            alias: None,
            population_bounds: BTreeMap::new(),
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

    /// This package with the authored population bounds the unit's `bound`
    /// declarations resolved to it (FR-380).
    #[must_use]
    pub(crate) fn with_population_bounds(
        mut self,
        bounds: BTreeMap<DeclarationKey, AuthoredBound>,
    ) -> Self {
        self.population_bounds = bounds;
        self
    }

    /// Every authored population bound of this package (FR-380).
    pub(crate) fn population_bounds(&self) -> impl Iterator<Item = &AuthoredBound> {
        self.population_bounds.values()
    }

    /// The authored bound of the population `population`, if the unit
    /// bounds it (FR-380).
    pub(crate) fn population_bound(&self, population: &DeclarationKey) -> Option<&AuthoredBound> {
        self.population_bounds.get(population)
    }

    /// The effective identity of `key` when it declares an object type.
    pub(crate) fn object_type_id(&self, key: &DeclarationKey) -> Option<EffectiveId> {
        matches!(
            self.records.get(key),
            Some(DomainPackageRecord::ObjectType(_))
        )
        .then(|| self.effective_id(key))
        .flatten()
    }

    /// The export whose artifact id is `artifact`: the record keyed
    /// `ix://<package>/<artifact>`, as a model name `M::<artifact>` names it.
    pub(crate) fn export(&self, artifact: &str) -> Option<&DomainPackageRecord> {
        self.records.iter().find_map(|(key, record)| {
            (crate::model::intake::type_identity_segment(&key.package, &key.node) == Some(artifact))
                .then_some(record)
        })
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

    /// Every relationship declaration in this model package. Relationship
    /// nodes are model-owned graph roots even when no checked expression has
    /// a relationship-typed operand to name them.
    pub(crate) fn relationship_declarations(&self) -> impl Iterator<Item = &DeclarationKey> {
        self.records.iter().filter_map(|(key, record)| {
            matches!(record, DomainPackageRecord::Relationship(_)).then_some(key)
        })
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
        record_node_key(self.records.get(declaration)?, identity)
            .ok()
            .flatten()
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
    /// an unbounded `Population(None)` domain unless the unit authors a
    /// bound on it (FR-380), which is paired with it here.
    pub(crate) fn populations_of(
        &self,
        target: EffectiveId,
        conforms: impl Fn(EffectiveId, EffectiveId) -> bool,
    ) -> Vec<(usize, &DeclarationKey, EffectiveId, Option<&AuthoredBound>)> {
        let covers = |member: &DeclarationKey| {
            self.types.iter().any(|(id, declared)| {
                declared == member && (*id == target || conforms(target, *id))
            })
        };
        self.population_domains()
            .filter(|(_, population, _)| population.member_types.iter().any(covers))
            .map(|(ordinal, population, canonical)| {
                (
                    ordinal,
                    &population.key,
                    canonical,
                    self.population_bounds.get(&population.key),
                )
            })
            .collect()
    }

    /// FR-150: each object type of this package that a `symmetric`
    /// population covers, with the least such population's declaration key.
    /// A population covers a type as in [`Self::populations_of`]: a member
    /// type is the type itself or a supertype of it, by `conforms`.
    pub(crate) fn symmetric_object_types(
        &self,
        conforms: impl Fn(EffectiveId, EffectiveId) -> bool,
    ) -> Vec<(EffectiveId, DeclarationKey)> {
        let symmetric: Vec<&PopulationRecord> = self
            .population_domains()
            .map(|(_, population, _)| population)
            .filter(|population| population.symmetric)
            .collect();
        self.object_types()
            .filter_map(|(_, target)| {
                // The population covers `target` as `populations_of` does, and
                // also a supertype of a member type: a `Reference<Super>`
                // may hold an object of the population.
                let covers = |member: &DeclarationKey| {
                    self.types.iter().any(|(id, declared)| {
                        declared == member
                            && (*id == target || conforms(target, *id) || conforms(*id, target))
                    })
                };
                symmetric
                    .iter()
                    .find(|population| population.member_types.iter().any(covers))
                    .map(|population| (target, population.key.clone()))
            })
            .collect()
    }

    /// FR-124: the canonical member object type of each population of this
    /// package, once each, in ascending order: the objects the package's
    /// state is made of.
    pub(crate) fn population_objects(&self) -> std::collections::BTreeSet<EffectiveId> {
        self.population_domains()
            .map(|(_, _, canonical)| canonical)
            .collect()
    }

    /// The key of every declaration record of the package, of any kind.
    pub(crate) fn record_keys(&self) -> impl Iterator<Item = &DeclarationKey> {
        self.records.keys()
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

/// The refusal for a record whose model node key [`record_node_key`] could
/// not form.
fn record_key_refusal(
    location: &Location,
    declaration: &DeclarationKey,
    error: RecordKeyError,
) -> CheckRefusal {
    match error {
        RecordKeyError::EmptyOwner => fault(location, KeyFault::EmptyNode(declaration.clone())),
        RecordKeyError::Preimage(refusal) => super::preimage_refusal(location, refusal),
    }
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

/// The key of the model declaration node of `record`, or `None` for a record
/// kind no checked node names. It is a function of the declaring package's
/// identity and the record's node alone (FR-322 "Model-owned members"
/// step 2), so it is also the declaration key under which a model record
/// value type or event type is declared in the type environment (FR-359
/// item 9): a unit's alias for the model never enters it.
pub(crate) fn record_node_key(
    record: &DomainPackageRecord,
    identity: qsl_foundation::IdentityLimits,
) -> Result<Option<NodeKey>, RecordKeyError> {
    let Some((node_tag, form)) = record_form(record) else {
        return Ok(None);
    };
    let key = record.key();
    let owner = ModelOwner::new(key.package.clone(), key.node.clone())
        .map_err(|_| RecordKeyError::EmptyOwner)?;
    crate::check::node_key::node_key(&model_node_content(owner, node_tag, form).input(), identity)
        .map(|keyed| Some(keyed.key))
        .map_err(RecordKeyError::Preimage)
}

/// The key of the model declaration node of the variant type `record`
/// declares (QSpec FR-322 "Model-owned members" step 2), a function of the
/// declaring package's identity and the type's node alone, so no alias a
/// unit gives the model enters it (FR-360 item 11). `None` for a record that
/// declares no variant type.
pub(crate) fn variant_type_node_key(
    record: &DomainPackageRecord,
    identity: qsl_foundation::IdentityLimits,
) -> Result<Option<NodeKey>, RecordKeyError> {
    match record {
        DomainPackageRecord::Enum(_) => record_node_key(record, identity),
        _ => Ok(None),
    }
}

/// Why [`record_node_key`] has no key for a record it names.
#[derive(Debug)]
pub(crate) enum RecordKeyError {
    /// The record's package identity or node is empty.
    EmptyOwner,
    /// The node's preimage has no key.
    Preimage(crate::check::node_key::NodeKeyRefusal),
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
        DomainPackageRecord::StateMachine(_) => Some((NodeTag::Model, "state_machine")),
        DomainPackageRecord::Process(_) => Some((NodeTag::Model, "process")),
        DomainPackageRecord::PersistenceInterface(_) => {
            Some((NodeTag::Model, "persistence_interface"))
        }
        DomainPackageRecord::Relationship(_) => Some((NodeTag::Relation, "relationship")),
        // FR-359 item 3: a record value type and an event type are model
        // declaration nodes with their `ModelOwner`, never a `composite_type`
        // with a source owner.
        DomainPackageRecord::RecordValueType(_) => Some((NodeTag::Model, "record_value_type")),
        DomainPackageRecord::EventType(_) => Some((NodeTag::Model, "event_type")),
        // FR-360 item 12: a variant type is a model declaration node with
        // its `ModelOwner`, never a unit-owned `scalar_type`/`enum`.
        DomainPackageRecord::Enum(_) => Some((NodeTag::Model, "variant_type")),
        DomainPackageRecord::Variant(_)
        | DomainPackageRecord::FieldMember(_)
        | DomainPackageRecord::Clause(_)
        | DomainPackageRecord::Namespace(_)
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

    /// The model record value type or event type whose model node key is
    /// `handle`, the declaration key a model record carries in the type
    /// environment (FR-359 item 9); `None` when `handle` is a source
    /// composite.
    pub(super) fn model_record_declaration(
        &mut self,
        handle: NodeKey,
        location: &Location,
    ) -> Result<Option<DeclarationKey>, CheckRefusal> {
        if self.model_records.is_none() {
            let mut records = BTreeMap::new();
            for model in self.models {
                for (declaration, record) in &model.records {
                    if !matches!(
                        record,
                        DomainPackageRecord::RecordValueType(_) | DomainPackageRecord::EventType(_)
                    ) {
                        continue;
                    }
                    let key = record_node_key(record, self.identity)
                        .map_err(|error| record_key_refusal(location, declaration, error))?;
                    if let Some(key) = key {
                        records.insert(key, declaration.clone());
                    }
                }
            }
            self.model_records = Some(records);
        }
        Ok(self
            .model_records
            .as_ref()
            .and_then(|records| records.get(&handle))
            .cloned())
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

    /// The model declaration node of the variant type `variant` and one
    /// `enum_value` node per case (FR-360 item 12), each keyed over the
    /// type's node and the exact case name, any non-empty name kept as
    /// written. Returns the type's node key.
    pub(super) fn variant_type_nodes(
        &mut self,
        variant: &VariantTypeDeclaration,
        location: &Location,
    ) -> Result<NodeKey, CheckRefusal> {
        let declaration = self.model_node(variant.declaration(), location)?;
        for member in variant.members() {
            self.enum_member_node(location, declaration, member)?;
        }
        Ok(declaration)
    }

    /// Key every relationship declaration selected by this unit. Unlike an
    /// object type, a relationship is named only by a frame that modifies
    /// it, so it must be rooted explicitly for the checked package to carry
    /// the model graph's relation nodes into emission.
    pub(crate) fn relationship_nodes(
        &mut self,
        locations: &BTreeMap<String, Location>,
    ) -> Result<(), CheckRefusal> {
        let declarations: Vec<(DeclarationKey, Location)> = self
            .models
            .iter()
            .filter_map(|model| {
                let alias = model.alias.as_ref()?;
                let location = locations.get(alias)?.clone();
                Some(
                    model
                        .relationship_declarations()
                        .map(move |declaration| (declaration.clone(), location.clone()))
                        .collect::<Vec<_>>(),
                )
            })
            .flatten()
            .collect();
        for (declaration, location) in declarations {
            let key = self.model_node(&declaration, &location)?;
            // A relationship has no checked value or frame occurrence to
            // anchor it. Record its generated model root at the selected
            // model declaration span so emission can resolve provenance.
            self.record(key, OccurrenceRole::Generated, location);
        }
        Ok(())
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
    /// `Set<Reference<T>>` node, binding `max` = `N`. `Population<T>` with
    /// no maximum is the same `model_population` node with no `max`
    /// member (FR-334), so it is distinct from a bare unbounded
    /// `Set<Reference<T>>` and from every bounded population.
    pub(super) fn population_type(
        &mut self,
        target: EffectiveId,
        maximum: Option<u64>,
        location: &Location,
    ) -> Result<NodeKey, CheckRefusal> {
        let reference = self.reference_type(target, location)?;
        let set = self.insert(
            location,
            NodeTag::CompositeType,
            "set",
            None,
            None,
            BodyTerm::aggregate(vec![MemberTerm::reference(reference)]),
        )?;
        let bindings = match maximum {
            Some(maximum) => vec![(
                "max",
                self.integer_literal(Integer::from(maximum), location)?,
            )],
            None => Vec::new(),
        };
        let node = self.bounded("model_population", set, bindings, location)?;
        self.population_nodes.insert(node);
        Ok(node)
    }

    /// FR-380 "The population node": the `Population<E>[N]` node of the
    /// authored `bound`, `E` the near end's type of the reachability that
    /// walks it, with the one `Type` occurrence of the bound's `[N]` this
    /// node holds for that bound (once, however many reachabilities share
    /// it). A bound no reachability walks never reaches here, so it emits no
    /// node and no occurrence.
    pub(super) fn bounded_population(
        &mut self,
        bound: &AuthoredBound,
        near: EffectiveId,
        reachability: &Location,
    ) -> Result<NodeKey, CheckRefusal> {
        let site = type_declaration(&bound_origin(&bound.label));
        let node = self.population_type(near, Some(bound.maximum), reachability)?;
        if self.bound_sites.insert((bound.label.clone(), node)) {
            self.record(node, OccurrenceRole::Type, site);
        }
        Ok(node)
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
                    (
                        "unit",
                        Some(dimension),
                        NominalNode::Unit(preimage.map(Box::new)),
                    )
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
