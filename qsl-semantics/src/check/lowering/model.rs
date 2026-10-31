// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-094: model-owned nodes, the `Reference<T>` and
//! `Population<T>[N]` type nodes, clause-function owners and quantity type
//! nodes.
//!
//! A model declaration node stands for one domain package declaration. It
//! is identified by its admitted structural `NodeKey`. A `Reference<T>`
//! carries `T` as an `EffectiveId` (ADR-013 O-05);
//! `check` maps it to its `DeclarationKey` through the admitted effective
//! view's `type_identities` (FR-094-CON-2). Each model
//! declaration node `check` keys is one model correspondence entry, and
//! `check` is the correspondence's only writer.

use quire_checked::node::{AuthoredBound, EndSide};
use std::collections::BTreeMap;

use quire_exact::{EffectiveId, Integer, NodeKey, UnitDomain, UnitId, ValueType};

use super::{identifiers, refuse, require_admitted, type_declaration, Lowering, NodeContent};
use crate::check::assemble::DimensionEquivalence;
use crate::check::family::OccurrenceRole;

use crate::check::node_key::{
    Binding, BodyTerm, GroupMember, GroupTerm, LeafTerm, MemberTerm, ModelOwner, NodeRef, NodeTag,
    Owner,
};
use crate::check::refusal::{CheckCause, CheckRefusal, KeyFault};
use crate::check::VariantTypeDeclaration;
use crate::model::domain_package::{
    DomainPackage, DomainPackageRecord, DomainPackageRef, PopulationRecord,
};
use crate::model::normalize::EffectiveView;
use crate::value::nominal_payload::{
    DimensionEquivalencePayload, DimensionPayload, NominalPayload, UnitPayload,
};
use qsl_forms::{DeclaredClauseKind, NameForm};
use quire_checked::key::DeclarationKey;
use quire_semantic_value::location::Location;
use quire_semantic_value::quantity::QuantityUnit;
use quire_semantic_value::unit::{NominalDeclaration, NominalUnitForm};

/// One step of a clock place's path below its object (FR-230 "Places"): a
/// field of a record by name, or a slot of a clock collection.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum PlaceStep {
    /// A field, by its member name.
    Field(String),
    /// The payload of the union case named, whose place is guarded by the
    /// case: live while the value is that case (FR-230 "Records, tuples and
    /// unions").
    Union(String),
    /// A slot of the collection the steps before it reach: each element
    /// the collection holds is one slot (FR-230 "Clock collections").
    Slot(CollectionShape),
}

/// The shape of a clock collection (FR-230 "Clock collections", QSpec
/// FR-415): a member whose multiplicity is not `[1, 1]`, by its kind and
/// cardinality.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct CollectionShape {
    /// Whether the elements are ordered (`Sequence`, `OrderedSet`).
    pub ordered: bool,
    /// Whether the elements are unique (`Set`, `OrderedSet`).
    pub unique: bool,
    /// The fewest elements the collection holds.
    pub min: u64,
    /// The most elements it holds; `None` when no maximum is declared.
    pub max: Option<u64>,
}

/// One clock place of an object type of the package (FR-230 "Places",
/// QSpec FR-415 "Clock places").
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PackagePlace {
    /// The object type whose field holds the place.
    pub(crate) owner: EffectiveId,
    /// The object's field the place is under: the member an operation's
    /// frame names to reset it.
    pub(crate) field: DeclarationKey,
    /// The path from the object to the clock, the object's field first.
    pub(crate) path: Vec<PlaceStep>,
    /// Whether an optional member (`presence: optional`, `Option<Clock>`)
    /// is on the path.
    pub(crate) optional: bool,
    /// Whether the optional member is the clock's own and no other is on
    /// the path: an `Option<Clock>`, whose presence is a bit (FR-230
    /// "Presence bit").
    pub(crate) bit: bool,
    /// Whether the place is a union case's payload directly under the
    /// object's field, guarded by the case (FR-230 "Records, tuples and
    /// unions"), with no other guard on its path.
    pub(crate) case: bool,
    /// The clock aliases the member holding the clock is typed by, directly
    /// or as a sequence's items, each alias of an alias on the chain.
    pub(crate) aliases: Vec<DeclarationKey>,
    /// The identity of the object type declaring the place's field: the
    /// declaring type of the first resolved row.
    pub(crate) declaring: String,
    /// For each path step, the declared member type it reaches, rendered
    /// (`Sequence<ix://p/Timer>`, `Option<ix://quire/native/Clock>`), and
    /// the member identity of a field step (QSpec FR-322 "Timed-model
    /// nodes", `resolved_path`).
    pub(crate) members: Vec<(String, Option<String>)>,
}

/// One row of a clock place's `resolved_path` (QSpec FR-322 "Timed-model
/// nodes"): for one path step, its declaring type, the member it resolves
/// to and that member's declared type, and for a step a slot follows, the
/// collection's kind and bounds.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlaceRow {
    /// The declaring object, record, variant or collection type.
    pub declaring_type: String,
    /// The member identity: a field's declaration key, `<type>/union/<case>`
    /// or `<collection>[slot]`.
    pub member_identity: String,
    /// The member's declared type, rendered.
    pub member_type: String,
    /// `sequence`, `set`, `bag`, `ordered_set` when a slot step follows,
    /// else `none`.
    pub collection_kind: String,
    /// The collection's minimum when a slot step follows, else empty.
    pub minimum: String,
    /// The collection's maximum (`unbounded` with none) when a slot step
    /// follows, else empty.
    pub maximum: String,
}

/// The wire name of a collection of `shape` (QSpec FR-322 `slot` step).
pub fn collection_kind(shape: CollectionShape) -> &'static str {
    match (shape.ordered, shape.unique) {
        (true, false) => "sequence",
        (false, true) => "set",
        (false, false) => "bag",
        (true, true) => "ordered_set",
    }
}

/// `inner` wrapped in the collection type of `shape`.
fn collection_type(shape: CollectionShape, inner: &str) -> String {
    let kind = match (shape.ordered, shape.unique) {
        (true, false) => "Sequence",
        (false, true) => "Set",
        (false, false) => "Bag",
        (true, true) => "OrderedSet",
    };
    format!("{kind}<{inner}>")
}

/// The element type of the rendered collection type `rendered`, read
/// through an `Option<..>` around it (QSL FR-230-AC-19's `o:
/// Option<Sequence<Clock>[0, 2]>`). A string edge: `rendered` is the
/// `resolved_path` row text this module renders.
#[ix_trace_rs::string_edge]
fn element_type(rendered: &str) -> &str {
    let rendered = rendered
        .strip_prefix("Option<")
        .and_then(|inner| inner.strip_suffix('>'))
        .unwrap_or(rendered);
    ["Sequence<", "Set<", "Bag<", "OrderedSet<"]
        .iter()
        .find_map(|prefix| rendered.strip_prefix(prefix)?.strip_suffix('>'))
        .unwrap_or(rendered)
}

/// Whether `field` holds one value: multiplicity `[1, 1]`, not a collection.
fn single(field: &crate::model::domain_package::FieldMemberRecord) -> bool {
    field.multiplicity.lower == 1 && field.multiplicity.upper == Some(1)
}

/// Whether the place path `place` lies strictly below `path`: the value at
/// `path` holds the place.
pub(crate) fn strictly_below(place: &[PlaceStep], path: &[PlaceStep]) -> bool {
    place.len() > path.len() && place.starts_with(path)
}

impl PackagePlace {
    /// The place's `resolved_path` rows, one for each path step (QSpec
    /// FR-322 "Timed-model nodes").
    pub(crate) fn rows(&self) -> Vec<PlaceRow> {
        let mut rows: Vec<PlaceRow> = Vec::with_capacity(self.path.len());
        for (index, step) in self.path.iter().enumerate() {
            let previous = rows.last().map(|row| row.member_type.clone());
            let (member_type, identity) = self.members.get(index).cloned().unwrap_or_default();
            let (declaring_type, member_identity) = match step {
                PlaceStep::Field(_) => (
                    previous.unwrap_or_else(|| self.declaring.clone()),
                    identity.unwrap_or_default(),
                ),
                PlaceStep::Slot(_) => {
                    let parent = previous.unwrap_or_default();
                    (element_type(&parent).to_owned(), format!("{parent}[slot]"))
                }
                PlaceStep::Union(case) => {
                    let declaring = previous.unwrap_or_default();
                    let identity = format!("{declaring}/union/{case}");
                    (declaring, identity)
                }
            };
            let (collection_kind, minimum, maximum) = match self.path.get(index + 1) {
                Some(PlaceStep::Slot(shape)) => (
                    collection_kind(*shape).to_owned(),
                    shape.min.to_string(),
                    shape
                        .max
                        .map_or_else(|| "unbounded".to_owned(), |max| max.to_string()),
                ),
                _ => ("none".to_owned(), String::new(), String::new()),
            };
            rows.push(PlaceRow {
                declaring_type,
                member_identity,
                member_type,
                collection_kind,
                minimum,
                maximum,
            });
        }
        rows
    }

    /// The place's path written with `.` between its steps.
    pub(crate) fn name(&self) -> String {
        self.path
            .iter()
            .map(|step| match step {
                PlaceStep::Field(name) | PlaceStep::Union(name) => name.as_str(),
                PlaceStep::Slot(_) => "[slot]",
            })
            .collect::<Vec<_>>()
            .join(".")
    }
}

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
    /// FCD dimensions and units retained for FR-230's owner-qualified graph.
    pub(crate) dimensions: Vec<crate::model::domain_package::PackageDimensionRecord>,
    pub(crate) units: Vec<crate::model::domain_package::PackageUnitRecord>,
    types: BTreeMap<EffectiveId, DeclarationKey>,
    index: std::sync::Arc<crate::model::index::ModelIndex>,
    /// Each type's direct supertypes, from the inherit facts of its effective
    /// declaration; ancestry beyond one edge is followed on demand.
    supertypes: BTreeMap<DeclarationKey, Vec<DeclarationKey>>,
    /// The authored bound (FR-380) of each population this package's
    /// `bound` declarations name, by the population's declaration key.
    /// Empty until the assembler resolves the unit's bounds.
    population_bounds: BTreeMap<DeclarationKey, AuthoredBound>,
    /// Every clock place of the package's object types
    /// ([`Self::clock_places`]), found once from `records`.
    places: Vec<PackagePlace>,
    /// Every required single-valued field whose type is a package object
    /// type, `(owner, field name, target)`: [`Self::referenced_object`],
    /// found once from `records`.
    references: Vec<(EffectiveId, String, EffectiveId)>,
}

/// One population of an admitted package as FR-391 names it
/// ([`AdmittedModel::named_populations`]).
pub(crate) struct NamedPopulationRecord<'m> {
    /// The population's name past the package identity, `P` of `M::P`.
    pub(crate) name: &'m str,
    /// The population's declaration.
    pub(crate) key: &'m DeclarationKey,
    /// Its ordinal among the package's populations.
    pub(crate) ordinal: usize,
    /// Its canonical member type.
    pub(crate) canonical: EffectiveId,
    /// Its declared member types the package admits.
    pub(crate) members: Vec<EffectiveId>,
    /// The unit's authored bound on it.
    pub(crate) bound: Option<&'m AuthoredBound>,
}

/// The `Origin::TypeDeclaration` name the `[N]` of the `bound` declaration
/// labelled `label` is located under. Spaced, so it names no declared type.
pub(crate) fn bound_origin(label: &str) -> String {
    format!("bound {label}")
}

/// The `Origin::TypeDeclaration` name the declaration of the dimension or
/// unit `name` is located under (FR-091-AC-44: dimensions, units and types
/// are namespaces of their own). Spaced, so it names no declared type. The
/// position is namespaced, the node's `declaration` name is not: a unit and
/// a record of one name emit a package the strict reader refuses
/// (Remaining work: QSL-950).
pub(crate) fn quantity_origin(kind: QuantityKind, name: &str) -> String {
    match kind {
        QuantityKind::Dimension => format!("dimension {name}"),
        QuantityKind::Unit => format!("unit {name}"),
    }
}

/// Which quantity declaration namespace a [`quantity_origin`] names.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum QuantityKind {
    /// A `dimension` declaration.
    Dimension,
    /// A `unit` declaration.
    Unit,
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

    /// Whether the object type `identity` is a systems interface, whose
    /// model node is a `model`/`systems_interface` node (FR-094).
    pub(crate) fn is_systems_interface(&self, identity: EffectiveId) -> bool {
        self.types
            .get(&identity)
            .and_then(|key| self.records.get(key))
            .is_some_and(|record| {
                matches!(
                    record_form(record),
                    Some((NodeTag::Model, "systems_interface"))
                )
            })
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
            owner.clone(),
            name,
        ))
    }

    /// The effective identity of the object type declared under `key`.
    pub(crate) fn effective_id(&self, key: &DeclarationKey) -> Option<EffectiveId> {
        self.types
            .iter()
            .find_map(|(identity, declared)| (declared == key).then_some(identity.clone()))
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
            let slot = staged.parent_field(child.clone(), parent, &target_ref, (), ())?;
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
                FieldRef::new(context.clone(), name),
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
            places: Vec::new(),
            references: Vec::new(),
            selection: domain_package.model_selection.clone(),
            records: domain_package
                .records
                .iter()
                .map(|record| (record.key().clone(), record.clone()))
                .collect(),
            dimensions: domain_package.dimensions.clone(),
            units: domain_package.units.clone(),
            types: view
                .type_identities()
                .iter()
                .map(|(key, id)| (id.clone(), key.clone()))
                .collect(),
        }
        .with_places()
    }

    /// The model with its clock places found.
    fn with_places(mut self) -> Self {
        self.places = self.find_places();
        self.references = self.find_references();
        self
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
            places: Vec::new(),
            references: Vec::new(),
            selection: domain_package.model_selection.clone(),
            records: domain_package
                .records
                .iter()
                .map(|record| (record.key().clone(), record.clone()))
                .collect(),
            dimensions: domain_package.dimensions.clone(),
            units: domain_package.units.clone(),
            types: types.into_iter().collect(),
        }
        .with_places()
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
    /// `ix://<package>/<artifact>`, or a relationship's canonical slot
    /// `ix://<package>/relationship/<artifact>`, named by `M::<artifact>`.
    #[ix_trace_rs::string_edge]
    pub(crate) fn export(&self, artifact: &str) -> Option<&DomainPackageRecord> {
        self.records.iter().find_map(|(key, record)| {
            let name = crate::model::intake::type_identity_segment(&key.package, &key.node)
                .or_else(|| match record {
                    DomainPackageRecord::Relationship(_) => key
                        .node
                        .strip_prefix(&format!("ix://{}/relationship/", key.package))
                        .filter(|name| !name.is_empty() && !name.contains('/')),
                    _ => None,
                });
            (name == Some(artifact)).then_some(record)
        })
    }

    /// The declaration key of the type `id`, when this package declares it.
    pub(crate) fn type_key(&self, id: EffectiveId) -> Option<&DeclarationKey> {
        self.types.get(&id)
    }

    /// The alias of the `model` declaration this package is admitted for,
    /// if any.
    pub(crate) fn alias(&self) -> Option<&str> {
        self.alias.as_deref()
    }

    /// The package operations a tick source's `operation`,
    /// `Owner::operation`, or `M::Owner::operation` name can denote. The
    /// returned declaration identities distinguish a resolved operation
    /// from merely retaining its source spelling.
    pub(crate) fn tick_operation_keys(&self, written: &NameForm) -> Vec<&DeclarationKey> {
        let (owner, operation) = match &written.segments[..] {
            [operation] => (None, operation.as_str()),
            [owner, operation] => (Some(owner.as_str()), operation.as_str()),
            [qualifier, owner, operation] if self.alias() == Some(qualifier.as_str()) => {
                (Some(owner.as_str()), operation.as_str())
            }
            _ => return Vec::new(),
        };
        let owners: Vec<&DeclarationKey> = owner
            .into_iter()
            .flat_map(|name| {
                self.operation_owners()
                    .filter(move |(key, _)| key.node.rsplit('/').next() == Some(name))
                    .map(|(key, _)| key)
            })
            .collect();
        if owner.is_some() && owners.is_empty() {
            return Vec::new();
        }
        let candidates: Vec<_> = self
            .index
            .operations()
            .filter(|record| {
                (owner.is_none()
                    || owners
                        .iter()
                        .any(|target| self.reaches_owner(target, &record.owner)))
                    && crate::model::intake::member_identity_name(
                        &record.owner.node,
                        &record.key.node,
                    ) == Some(operation)
            })
            .collect();
        candidates
            .iter()
            .filter(|record| {
                owner.is_none()
                    || owners.len() != 1
                    || !candidates.iter().any(|other| {
                        other.key != record.key
                            && other.owner != record.owner
                            && self.reaches_owner(&other.owner, &record.owner)
                    })
            })
            .map(|record| &record.key)
            .collect()
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
            .map(|(id, key)| (key, id.clone()))
    }

    /// Every state-machine declaration of this package and its effective
    /// identity. Refinement field-set rows address these declarations through
    /// the same field type environment as object-type rows.
    pub(crate) fn state_machine_types(
        &self,
    ) -> impl Iterator<Item = (&DeclarationKey, EffectiveId)> {
        self.types
            .iter()
            .filter(|(_, key)| {
                matches!(self.records.get(*key), Some(DomainPackageRecord::StateMachine(_)))
                    // A refinement object row maps a state-machine *field
                    // set*. A machine with no fields has no such set to map.
                    && self.records.values().any(|record| {
                        matches!(record, DomainPackageRecord::FieldMember(field) if &field.owner == *key)
                    })
            })
            .map(|(id, key)| (key, id.clone()))
    }

    /// Every semantic object-type model node in this selected package, in
    /// declaration-key order. FR-230 resolves a time-source alias only when
    /// this package names exactly one such node.
    pub(crate) fn object_type_model_node_keys(&self) -> Vec<NodeKey> {
        self.object_types()
            .filter_map(|(declaration, _)| self.model_node_key(declaration))
            .collect()
    }

    /// The key of `declaration`'s model node, minted by the same recipe
    /// [`Lowering::model_node`] uses, without adding a node to any graph.
    /// `None` when this package does not declare `declaration`, or declares
    /// it as a record kind no checked node names.
    pub(crate) fn model_node_key(&self, declaration: &DeclarationKey) -> Option<NodeKey> {
        if self.selection.identity != declaration.package {
            return None;
        }
        record_node_key(self.records.get(declaration)?)
            .ok()
            .flatten()
    }

    /// The object type a `terminal` declaration's model node names (FR-411):
    /// the one object type the package's populations range over (FR-124),
    /// else its least object type by declaration key, so that a model with
    /// no population still names a model node. QSpec FR-476 does not say
    /// which model node a `terminal any` over several object types names;
    /// the least is the current choice until STD-492 settles it.
    pub(crate) fn terminal_object(&self) -> Option<EffectiveId> {
        let populated = self.population_objects();
        let mut populated = populated.iter();
        match (populated.next(), populated.next()) {
            (Some(only), None) => Some(only.clone()),
            _ => self
                .object_types()
                .min_by(|(left, _), (right, _)| left.cmp(right))
                .map(|(_, id)| id),
        }
    }

    /// The declaring object type's effective identity and operation name for
    /// one admitted FCD operation member.
    pub(crate) fn operation_anchor_declaration(
        &self,
        key: &DeclarationKey,
    ) -> Option<(EffectiveId, String)> {
        let Some(DomainPackageRecord::OperationMember(operation)) = self.records.get(key) else {
            return None;
        };
        let declaring = self.effective_id(&operation.owner)?;
        let name = crate::model::intake::member_identity_name(&operation.owner.node, &key.node)?;
        Some((declaring, name.to_owned()))
    }

    /// Every operation owner of this package that `M::T::op` can name: an
    /// object type, a process or a state machine (FR-104, FR-115, FR-121),
    /// with its declaration key and effective identity.
    pub(crate) fn operation_owners(&self) -> impl Iterator<Item = (&DeclarationKey, EffectiveId)> {
        self.types
            .iter()
            .filter(|(_, key)| {
                matches!(
                    self.records.get(*key),
                    Some(
                        DomainPackageRecord::ObjectType(_)
                            | DomainPackageRecord::Process(_)
                            | DomainPackageRecord::StateMachine(_)
                    )
                )
            })
            .map(|(id, key)| (key, id.clone()))
    }

    /// The declaration key of every top-level declaration of this package
    /// that is not an operation owner (a value type, an enum, a record, ...):
    /// the nodes that cannot declare operations a frame names.
    pub(crate) fn non_owner_declarations(&self) -> impl Iterator<Item = &DeclarationKey> {
        self.records
            .iter()
            .filter(|(_, record)| {
                !matches!(
                    record,
                    DomainPackageRecord::ObjectType(_)
                        | DomainPackageRecord::Process(_)
                        | DomainPackageRecord::StateMachine(_)
                )
            })
            .map(|(key, _)| key)
    }

    /// Every relationship declaration in this model package. Relationship
    /// nodes are model-owned graph roots even when no checked expression has
    /// a relationship-typed operand to name them.
    pub(crate) fn relationship_declarations(&self) -> impl Iterator<Item = &DeclarationKey> {
        self.records.iter().filter_map(|(key, record)| {
            matches!(record, DomainPackageRecord::Relationship(_)).then_some(key)
        })
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
                declared == member && (*id == target || conforms(target.clone(), id.clone()))
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

    /// FR-336: every population whose member type overlaps `target` by
    /// subtyping, for temporal requirement records. A population can overlap
    /// from either direction; this is broader than `populations_of`, whose
    /// coverage direction is used by state-clause resolution.
    pub(crate) fn populations_overlapping(
        &self,
        target: EffectiveId,
        conforms: impl Fn(EffectiveId, EffectiveId) -> bool,
    ) -> Vec<(usize, &DeclarationKey, EffectiveId, Option<&AuthoredBound>)> {
        let overlaps = |member: &DeclarationKey| {
            self.types.iter().any(|(id, declared)| {
                declared == member
                    && (*id == target
                        || conforms(target.clone(), id.clone())
                        || conforms(id.clone(), target.clone()))
            })
        };
        self.population_domains()
            .filter(|(_, population, _)| population.member_types.iter().any(overlaps))
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

    /// FR-391: every population of this package as a population position
    /// names it: the name past the package's identity, its ordinal and
    /// canonical member (as [`Self::population_domains`] gives them), the
    /// effective identity of each declared member type this package admits,
    /// and the unit's authored bound on it (FR-380).
    pub(crate) fn named_populations(&self) -> impl Iterator<Item = NamedPopulationRecord<'_>> {
        self.population_domains()
            .filter_map(move |(ordinal, record, canonical)| {
                let name = crate::model::intake::type_identity_segment(
                    &record.key.package,
                    &record.key.node,
                )?;
                let members = record
                    .member_types
                    .iter()
                    .filter_map(|member| {
                        self.types
                            .iter()
                            .find_map(|(id, declared)| (declared == member).then_some(id.clone()))
                    })
                    .collect();
                Some(NamedPopulationRecord {
                    name,
                    key: &record.key,
                    ordinal,
                    canonical,
                    members,
                    bound: self.population_bounds.get(&record.key),
                })
            })
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
                            && (*id == target
                                || conforms(target.clone(), id.clone())
                                || conforms(id.clone(), target.clone()))
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

    /// FR-230 "Places": every clock place of an object type of this
    /// package, in ascending declaration-key order of the object's fields
    /// and, below a field, of each record's fields. A field typed `Clock` is
    /// one place, and a field of a package record type holds the places of
    /// the record's fields. The FCD semantic IR admits `Clock` only as a
    /// field type, so a package has no clock alias, clock union case,
    /// optional clock or clock collection to place.
    pub(crate) fn clock_places(&self) -> &[PackagePlace] {
        &self.places
    }

    /// The clock places [`Self::clock_places`] lists, found from `records`.
    fn find_places(&self) -> Vec<PackagePlace> {
        let mut places = Vec::new();
        for record in self.records.values() {
            let DomainPackageRecord::FieldMember(field) = record else {
                continue;
            };
            let Some(owner) = self.object_type_id(&field.owner) else {
                continue;
            };
            let Some(name) =
                crate::model::intake::member_identity_name(&field.owner.node, &field.key.node)
            else {
                continue;
            };
            let root = PackagePlace {
                owner,
                field: field.key.clone(),
                path: vec![PlaceStep::Field(name.to_owned())],
                optional: false,
                bit: false,
                case: false,
                aliases: Vec::new(),
                declaring: field.owner.node.clone(),
                members: Vec::new(),
            };
            self.places_of(field, root, &mut Vec::new(), &mut places);
        }
        places
    }

    /// `value_type` rendered for a `resolved_path` row (QSpec FR-322
    /// "Timed-model nodes"): `Clock` and natives by their `ix://quire/native`
    /// identity, a package type by its identity, an alias as the type it
    /// aliases and a package sequence type as `Sequence<items>`.
    fn rendered(&self, value_type: &crate::model::domain_package::ValueTypeRef) -> String {
        use crate::model::domain_package::ValueTypeRef;
        let mut wrappers = 0_usize;
        let mut named = value_type;
        let mut steps = 0_usize;
        let base = loop {
            steps += 1;
            match named {
                ValueTypeRef::Clock => break "ix://quire/native/Clock".to_owned(),
                ValueTypeRef::Native(native) => {
                    break format!("ix://quire/native/{}", native.as_str())
                }
                ValueTypeRef::Unit(name) => break name.clone(),
                ValueTypeRef::Package(key) => match self.records.get(key) {
                    Some(DomainPackageRecord::AliasType(alias)) if steps <= self.records.len() => {
                        named = &alias.target;
                    }
                    Some(DomainPackageRecord::SequenceType(sequence))
                        if steps <= self.records.len() && sequence.items != *named =>
                    {
                        wrappers += 1;
                        named = &sequence.items;
                    }
                    _ => break key.node.clone(),
                },
            }
        };
        (0..wrappers).fold(base, |inner, _| format!("Sequence<{inner}>"))
    }

    /// The places a member `field` holds below `at`, which already names the
    /// member itself. `visiting` holds the record types on the path, so a
    /// recursive record holds no place twice.
    fn places_of<'m>(
        &'m self,
        field: &'m crate::model::domain_package::FieldMemberRecord,
        mut at: PackagePlace,
        visiting: &mut Vec<&'m DeclarationKey>,
        places: &mut Vec<PackagePlace>,
    ) {
        use crate::model::domain_package::ValueTypeRef;
        // The field step's row: the member's declared type, its collection
        // or `Option` around the type it holds.
        let declared = self.rendered(&field.value_type);
        let member_type = if !single(field) {
            // An optional collection member holds optional elements:
            // `q: Sequence<Option<Clock>>[0, 2]` (QSpec FR-322's
            // timed-slot-selector vectors, the presence place `q`).
            let element = if matches!(field.presence, quire_exact::Presence::Optional) {
                format!("Option<{declared}>")
            } else {
                declared
            };
            collection_type(
                CollectionShape {
                    ordered: field.multiplicity.ordered,
                    unique: field.multiplicity.unique,
                    min: field.multiplicity.lower,
                    max: field.multiplicity.upper,
                },
                &element,
            )
        } else if matches!(field.presence, quire_exact::Presence::Optional) {
            format!("Option<{declared}>")
        } else {
            declared
        };
        at.members
            .push((member_type.clone(), Some(field.key.node.clone())));
        let mut reached = member_type;
        if !single(field) {
            at.path.push(PlaceStep::Slot(CollectionShape {
                ordered: field.multiplicity.ordered,
                unique: field.multiplicity.unique,
                min: field.multiplicity.lower,
                max: field.multiplicity.upper,
            }));
            reached = element_type(&reached).to_owned();
            at.members.push((reached.clone(), None));
        }
        let above = at.optional;
        let own = matches!(field.presence, quire_exact::Presence::Optional);
        at.optional |= own;
        // A package sequence type is a slot of an unbounded sequence of its
        // items (QSpec FR-154), a chain of them one slot each, and a package
        // alias the type it aliases.
        let Some((value_type, sequences)) =
            crate::model::domain_package::unaliased(&field.value_type, |key| self.records.get(key))
        else {
            return;
        };
        // Every alias on the way to the clock.
        let mut named = &field.value_type;
        let mut steps = 0_usize;
        while let ValueTypeRef::Package(key) = named {
            steps += 1;
            if steps > self.records.len() {
                break;
            }
            named = match self.records.get(key) {
                Some(DomainPackageRecord::AliasType(alias)) => {
                    at.aliases.push(key.clone());
                    &alias.target
                }
                Some(DomainPackageRecord::SequenceType(sequence)) if sequence.items != *named => {
                    &sequence.items
                }
                _ => break,
            };
        }
        for _ in 0..sequences {
            at.path.push(PlaceStep::Slot(CollectionShape {
                ordered: true,
                unique: false,
                min: 0,
                max: None,
            }));
            reached = element_type(&reached).to_owned();
            at.members.push((reached.clone(), None));
        }
        let key = match value_type {
            ValueTypeRef::Clock => {
                at.bit = own && !above;
                places.push(at);
                return;
            }
            ValueTypeRef::Package(key) => key,
            ValueTypeRef::Native(_) | ValueTypeRef::Unit(_) => return,
        };
        match self.records.get(key) {
            // A variant type: each case whose payload is `Clock` is a place
            // guarded by the case.
            Some(DomainPackageRecord::Enum(variant_type)) => {
                for record in self.records.values() {
                    let DomainPackageRecord::Variant(case) = record else {
                        continue;
                    };
                    let clock = case.owner == variant_type.key
                        && case.payload_type.as_ref().is_some_and(|payload| {
                            crate::model::domain_package::unaliased(payload, |key| {
                                self.records.get(key)
                            }) == Some((&ValueTypeRef::Clock, 0))
                        });
                    if clock {
                        let mut below = at.clone();
                        below.path.push(PlaceStep::Union(case.case.clone()));
                        below
                            .members
                            .push(("ix://quire/native/Clock".to_owned(), None));
                        below.case = !below.optional && below.path.len() == 2;
                        below.optional = true;
                        places.push(below);
                    }
                }
            }
            Some(DomainPackageRecord::RecordValueType(_)) if !visiting.contains(&key) => {
                visiting.push(key);
                for record in self.records.values() {
                    let DomainPackageRecord::FieldMember(inner) = record else {
                        continue;
                    };
                    if &inner.owner != key {
                        continue;
                    }
                    let Some(name) =
                        crate::model::intake::member_identity_name(&key.node, &inner.key.node)
                    else {
                        continue;
                    };
                    let mut below = at.clone();
                    below.path.push(PlaceStep::Field(name.to_owned()));
                    self.places_of(inner, below, visiting, places);
                }
                visiting.pop();
            }
            _ => {}
        }
    }

    /// Whether an object of type `object` has the fields of `owner`: the
    /// two are one type or `owner` is an ancestor of `object`.
    pub(crate) fn conforms(&self, object: EffectiveId, owner: EffectiveId) -> bool {
        match (self.types.get(&object), self.types.get(&owner)) {
            (Some(specific), Some(general)) => self.reaches_owner(specific, general),
            _ => false,
        }
    }

    /// The clock place an object of type `object` holds at `path`: one
    /// declared on `object` itself or on one of its ancestors.
    pub(crate) fn clock_place(
        &self,
        object: EffectiveId,
        path: &[PlaceStep],
    ) -> Option<&PackagePlace> {
        self.clock_places()
            .iter()
            .find(|place| place.path == path && self.conforms(object.clone(), place.owner.clone()))
    }

    /// The package record type a written name `T` or `M::T` names, when
    /// exactly one does.
    pub(crate) fn record_named(&self, written: &NameForm) -> Option<&DeclarationKey> {
        let name = match &written.segments[..] {
            [name] => name.as_str(),
            [qualifier, name] if self.alias() == Some(qualifier.as_str()) => name.as_str(),
            _ => return None,
        };
        let mut found = self
            .records
            .iter()
            .filter(|(key, record)| {
                matches!(record, DomainPackageRecord::RecordValueType(_))
                    && key.node.rsplit('/').next() == Some(name)
            })
            .map(|(key, _)| key);
        let key = found.next()?;
        found.next().is_none().then_some(key)
    }

    /// Whether the record literal `written` (`T` or `M::T`) names a record
    /// type of this package that holds a clock: a clock field, or a field of
    /// a record type that holds one.
    pub(crate) fn literal_bears_clock(&self, written: &NameForm) -> bool {
        self.record_named(written)
            .is_some_and(|key| self.type_bears_clock(key, &mut Vec::new()))
    }

    /// The written names `M::T` of this package's record types that hold a
    /// clock, under the model alias.
    pub(crate) fn clock_bearing_record_names(&self) -> Vec<String> {
        let Some(alias) = self.alias() else {
            return Vec::new();
        };
        self.records
            .iter()
            .filter(|(key, record)| {
                matches!(record, DomainPackageRecord::RecordValueType(_))
                    && self.type_bears_clock(key, &mut Vec::new())
            })
            .filter_map(|(key, _)| Some(format!("{alias}::{}", key.node.rsplit('/').next()?)))
            .collect()
    }

    /// The field members `owner` declares, each with its member name.
    fn members_of<'m>(
        &'m self,
        owner: &'m DeclarationKey,
    ) -> impl Iterator<Item = (&'m str, &'m crate::model::domain_package::FieldMemberRecord)> + 'm
    {
        self.records.values().filter_map(move |record| {
            let DomainPackageRecord::FieldMember(field) = record else {
                return None;
            };
            if &field.owner != owner {
                return None;
            }
            let name = crate::model::intake::member_identity_name(&owner.node, &field.key.node)?;
            Some((name, field))
        })
    }

    /// The clock places of a value of the package record type `key`, each by
    /// its path below the value, in the order [`Self::clock_places`] lists
    /// them.
    pub(crate) fn record_clock_paths(&self, key: &DeclarationKey) -> Vec<Vec<PlaceStep>> {
        use crate::model::domain_package::ValueTypeRef;
        let mut paths = Vec::new();
        let mut pending: Vec<(&DeclarationKey, Vec<PlaceStep>, Vec<&DeclarationKey>)> =
            vec![(key, Vec::new(), vec![key])];
        while let Some((record, prefix, visiting)) = pending.pop() {
            for (name, field) in self.members_of(record) {
                let mut path = prefix.clone();
                path.push(PlaceStep::Field(name.to_owned()));
                let named = crate::model::domain_package::unaliased(&field.value_type, |key| {
                    self.records.get(key)
                });
                let value_type = match named {
                    Some((resolved, 0)) => resolved,
                    _ => &field.value_type,
                };
                match value_type {
                    ValueTypeRef::Clock => paths.push(path),
                    ValueTypeRef::Package(inner)
                        if matches!(
                            self.records.get(inner),
                            Some(DomainPackageRecord::RecordValueType(_))
                        ) && !visiting.contains(&inner) =>
                    {
                        let mut visiting = visiting.clone();
                        visiting.push(inner);
                        pending.push((inner, path, visiting));
                    }
                    _ => {}
                }
            }
        }
        paths.sort();
        paths
    }

    /// The object type a required single-valued field `name` of an object
    /// of type `object` refers to: the field's type is a package object
    /// type, so its value is a reference to one object of it (FR-230-AC-10's
    /// `peer`).
    pub(crate) fn referenced_object(&self, object: EffectiveId, name: &str) -> Option<EffectiveId> {
        self.references
            .iter()
            .find(|(owner, field, _)| field == name && self.conforms(object.clone(), owner.clone()))
            .map(|(_, _, target)| target.clone())
    }

    /// Every required single-valued field of a package object type
    /// referring to one: `(owner, field name, target)`.
    fn find_references(&self) -> Vec<(EffectiveId, String, EffectiveId)> {
        use crate::model::domain_package::ValueTypeRef;
        self.records
            .values()
            .filter_map(|record| {
                let DomainPackageRecord::FieldMember(field) = record else {
                    return None;
                };
                let owner = self.object_type_id(&field.owner)?;
                let name =
                    crate::model::intake::member_identity_name(&field.owner.node, &field.key.node)?;
                let single = field.multiplicity.lower == 1
                    && field.multiplicity.upper == Some(1)
                    && field.presence == quire_exact::Presence::Required;
                match &field.value_type {
                    ValueTypeRef::Package(target) if single => {
                        Some((owner, name.to_owned(), self.object_type_id(target)?))
                    }
                    _ => None,
                }
            })
            .collect()
    }

    /// `path` from an object of type `object` split where it leaves the
    /// objects it reaches by reference: the leading reference fields walked,
    /// counted, and the object type the rest of the path starts from.
    pub(crate) fn through_references(
        &self,
        object: EffectiveId,
        path: &[PlaceStep],
    ) -> (usize, EffectiveId) {
        let mut at = object;
        for (walked, step) in path.iter().enumerate() {
            let PlaceStep::Field(name) = step else {
                return (walked, at);
            };
            match self.referenced_object(at.clone(), name) {
                Some(next) => at = next,
                None => return (walked, at),
            }
        }
        (path.len(), at)
    }

    /// The package record type of the value an object of type `object`
    /// holds at `path`: a record field of the object, or of a record below
    /// it.
    pub(crate) fn record_at(
        &self,
        object: EffectiveId,
        path: &[PlaceStep],
    ) -> Option<&DeclarationKey> {
        use crate::model::domain_package::ValueTypeRef;
        let (PlaceStep::Field(first), rest) = path.split_first()? else {
            return None;
        };
        let mut held = self.records.values().find_map(|record| {
            let DomainPackageRecord::FieldMember(field) = record else {
                return None;
            };
            let owner = self.object_type_id(&field.owner)?;
            let name =
                crate::model::intake::member_identity_name(&field.owner.node, &field.key.node)?;
            (name == first && self.conforms(object.clone(), owner) && single(field))
                .then_some(&field.value_type)
        })?;
        for step in rest {
            let PlaceStep::Field(step) = step else {
                return None;
            };
            let ValueTypeRef::Package(record) = held else {
                return None;
            };
            held = self
                .members_of(record)
                .find(|(name, field)| name == step && single(field))
                .map(|(_, field)| &field.value_type)?;
        }
        match held {
            ValueTypeRef::Package(key)
                if matches!(
                    self.records.get(key),
                    Some(DomainPackageRecord::RecordValueType(_))
                ) =>
            {
                Some(key)
            }
            _ => None,
        }
    }

    /// Whether the package type `key` holds a clock place.
    fn type_bears_clock<'m>(
        &'m self,
        key: &'m DeclarationKey,
        visiting: &mut Vec<&'m DeclarationKey>,
    ) -> bool {
        use crate::model::domain_package::ValueTypeRef;
        match self.records.get(key) {
            Some(DomainPackageRecord::RecordValueType(_)) if !visiting.contains(&key) => {
                visiting.push(key);
                let bears = self.records.values().any(|record| {
                    let DomainPackageRecord::FieldMember(field) = record else {
                        return false;
                    };
                    &field.owner == key
                        && match &field.value_type {
                            ValueTypeRef::Clock => true,
                            ValueTypeRef::Package(inner) => self.type_bears_clock(inner, visiting),
                            ValueTypeRef::Native(_) | ValueTypeRef::Unit(_) => false,
                        }
                });
                visiting.pop();
                bears
            }
            Some(
                record @ (DomainPackageRecord::SequenceType(_) | DomainPackageRecord::AliasType(_)),
            ) if !visiting.contains(&key) => {
                visiting.push(key);
                let named = match record {
                    DomainPackageRecord::SequenceType(sequence) => &sequence.items,
                    DomainPackageRecord::AliasType(alias) => &alias.target,
                    _ => return false,
                };
                let bears = match crate::model::domain_package::unaliased(named, |key| {
                    self.records.get(key)
                }) {
                    Some((ValueTypeRef::Clock, _)) => true,
                    Some((ValueTypeRef::Package(inner), _)) => {
                        self.type_bears_clock(inner, visiting)
                    }
                    _ => false,
                };
                visiting.pop();
                bears
            }
            _ => false,
        }
    }

    /// The package's `min`/`max` constraints on clocks (FR-230 "Clock
    /// constraints on a type").
    pub(crate) fn clock_bounds(
        &self,
    ) -> impl Iterator<Item = &crate::model::domain_package::ValueBoundRecord> {
        self.records.values().filter_map(|record| match record {
            DomainPackageRecord::ValueBound(bound)
                if bound.target == crate::model::domain_package::ValueTypeRef::Clock =>
            {
                Some(bound)
            }
            _ => None,
        })
    }

    /// The object type a type name `T` or `M::T` denotes, when exactly one
    /// object type of the package is named `T`.
    pub(crate) fn object_type_named(&self, written: &NameForm) -> Option<EffectiveId> {
        let name = match &written.segments[..] {
            [name] => name.as_str(),
            [qualifier, name] if self.alias() == Some(qualifier.as_str()) => name.as_str(),
            _ => return None,
        };
        let mut found = self
            .object_types()
            .filter(|(key, _)| key.node.rsplit('/').next() == Some(name))
            .map(|(_, id)| id);
        let id = found.next()?;
        found.next().is_none().then_some(id)
    }

    /// Whether every field member of the package named `name` is a clock,
    /// and one is: a read of `.name` through a path whose object type is
    /// not known reads a clock.
    pub(crate) fn names_only_clocks(&self, name: &str) -> bool {
        let mut clock = false;
        for record in self.records.values() {
            let DomainPackageRecord::FieldMember(field) = record else {
                continue;
            };
            if crate::model::intake::member_identity_name(&field.owner.node, &field.key.node)
                != Some(name)
            {
                continue;
            }
            if !matches!(
                crate::model::domain_package::unaliased(&field.value_type, |key| self
                    .records
                    .get(key)),
                Some((crate::model::domain_package::ValueTypeRef::Clock, 0))
            ) {
                return false;
            }
            clock = true;
        }
        clock
    }

    /// The clock field `name` an object of type `object` holds: one declared
    /// on `object` itself or on one of its ancestors.
    pub(crate) fn clock_field(&self, object: EffectiveId, name: &str) -> Option<&DeclarationKey> {
        self.clock_place(object, &[PlaceStep::Field(name.to_owned())])
            .map(|place| &place.field)
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
                    .find_map(|(id, declared)| (declared == canonical).then_some(id.clone()))?;
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
pub(crate) fn model_node_content(
    owner: ModelOwner,
    node_tag: NodeTag,
    form: &'static str,
) -> NodeContent {
    NodeContent {
        owner_model: None,
        node_tag,
        semantic_form: form,
        semantic_type: None,
        declaration: None,
        owner: Some(Owner::Model(owner)),
        body: BodyTerm::aggregate(Vec::new()),
    }
}

/// The text of the two node tags a model declaration node carries.
fn model_tag_text(tag: NodeTag) -> &'static str {
    match tag {
        NodeTag::Relation => "relation",
        _ => "model",
    }
}

/// The structural key of a model declaration node: the declaring package's
/// identity, the path `[tag, form, node]` (the IR node string is one opaque
/// segment, never split or hashed) and ordinal 0.
///
/// # Errors
///
/// A refusal when the identity is empty or the key cannot be formed.
pub(crate) fn model_declaration_key(
    identity: &str,
    tag: NodeTag,
    form: &str,
    node: &str,
) -> Result<NodeKey, quire_exact::InvalidNodeKey> {
    crate::check::declared_node_key(
        identity.to_owned(),
        vec![
            model_tag_text(tag).to_owned(),
            form.to_owned(),
            node.to_owned(),
        ],
        0,
    )
}

/// QSpec FR-322 `model_root`: the key of the root node of the selected
/// model package `selection`: package `selection.identity`, path
/// `["model", "model_root"]`, ordinal 0. It is separate from the least root
/// type declaration's key.
///
/// # Errors
///
/// A refusal when the selection's identity is empty or the key cannot be
/// formed.
pub fn model_root_key(
    selection: &DomainPackageRef,
) -> Result<NodeKey, quire_exact::InvalidNodeKey> {
    crate::check::declared_node_key(
        selection.identity.clone(),
        vec!["model".to_owned(), "model_root".to_owned()],
        0,
    )
}

/// The content of a `relation`/`population` node (QSpec FR-322 "Population
/// references"): the one definition lowering's population reference and an
/// abstraction relation's population target both key.
pub(crate) fn population_node_content(owner: ModelOwner) -> NodeContent {
    model_node_content(owner, NodeTag::Relation, "population")
}

/// The key of the model declaration node of `record`, or `None` for a record
/// kind no checked node names: `[tag, form, node]` under the declaring
/// package's identity (FR-322 "Model-owned members" step 2), so it is also
/// the declaration key under which a model record value type or event type
/// is declared in the type environment (FR-359 item 9).
///
/// # Errors
///
/// A refusal when the package identity is empty or the key cannot be formed.
pub(crate) fn record_node_key(
    record: &DomainPackageRecord,
) -> Result<Option<NodeKey>, quire_exact::InvalidNodeKey> {
    let Some((node_tag, form)) = record_form(record) else {
        return Ok(None);
    };
    let key = record.key();
    model_declaration_key(&key.package, node_tag, form, &key.node).map(Some)
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
        DomainPackageRecord::TupleValueType(_) => Some((NodeTag::Model, "tuple_value_type")),
        DomainPackageRecord::EventType(_) => Some((NodeTag::Model, "event_type")),
        // FR-360 item 12: a variant type is a model declaration node with
        // its `ModelOwner`, never a unit-owned `scalar_type`/`enum`.
        DomainPackageRecord::Enum(_) => Some((NodeTag::Model, "variant_type")),
        DomainPackageRecord::Variant(_)
        | DomainPackageRecord::TuplePosition(_)
        | DomainPackageRecord::FieldMember(_)
        | DomainPackageRecord::Clause(_)
        | DomainPackageRecord::Namespace(_)
        | DomainPackageRecord::ScalarType(_)
        | DomainPackageRecord::SequenceType(_)
        | DomainPackageRecord::AliasType(_)
        | DomainPackageRecord::ValueBound(_)
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
        for model in self.models {
            for (declaration, record) in &model.records {
                let Some((tag, form)) = record_form(record) else {
                    continue;
                };
                if !matches!(
                    record,
                    DomainPackageRecord::RecordValueType(_) | DomainPackageRecord::EventType(_)
                ) {
                    continue;
                }
                let key = model_declaration_key(&declaration.package, tag, form, &declaration.node)
                    .map_err(|_| fault(location, KeyFault::EmptyNode(declaration.clone())))?;
                if key == handle {
                    return Ok(Some(declaration.clone()));
                }
            }
        }
        Ok(None)
    }

    /// FR-322: the model declaration node of the object type at the end of
    /// `relationship` opposite `near`, the end a navigation or reachability
    /// from `near` arrives at.
    pub(super) fn destination_type_node(
        &mut self,
        relationship: &DeclarationKey,
        near: EndSide,
        location: &Location,
    ) -> Result<NodeKey, CheckRefusal> {
        let far = match near {
            EndSide::Source => EndSide::Target,
            EndSide::Target => EndSide::Source,
        };
        let object = self
            .scope
            .relationships()
            .end_object(relationship, far)
            .ok_or_else(|| mismatch(location))?;
        self.object_node(object, location)
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
        let key =
            model_declaration_key(&model.selection.identity, node_tag, form, &declaration.node)
                .map_err(|_| fault(location, KeyFault::EmptyNode(declaration.clone())))?;
        // COMPILE GAP: add the keyed content to the graph; `insert_node`
        // mints content keys, so a keyed insert is owned by lowering.rs.
        self.insert_declared_node(
            location,
            key.clone(),
            model_node_content(owner, node_tag, form),
        )?;
        self.correspondence
            .record(key.clone(), declaration.clone())
            .map_err(|conflict| fault(location, KeyFault::CorrespondenceConflict(conflict)))?;
        Ok(key)
    }

    /// FR-322 "Population references": the `relation`/`population` node of
    /// the population `declaration`, keyed by its declaration key alone (no
    /// member type, no maximum, no clause). Every read of one population
    /// lowers to this one node.
    pub(super) fn population_reference_node(
        &mut self,
        declaration: &DeclarationKey,
        location: &Location,
    ) -> Result<NodeKey, CheckRefusal> {
        let (owner, model) = self.model_owner(declaration, location)?;
        let key = model_declaration_key(
            &model.selection.identity,
            NodeTag::Relation,
            "population",
            &declaration.node,
        )
        .map_err(|_| fault(location, KeyFault::EmptyNode(declaration.clone())))?;
        self.insert_declared_node(location, key, population_node_content(owner))
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
            self.enum_member_node(location, declaration.clone(), member)?;
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
            ValueType::Reference(target) => self.object_node(target.clone(), location),
            ValueType::Option(payload) => match payload.as_ref() {
                ValueType::Reference(target) => self.object_node(target.clone(), location),
                _ => Err(mismatch(location)),
            },
            ValueType::Collection(collection) => match collection.element() {
                ValueType::Reference(target) => self.object_node(target.clone(), location),
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
        let node = require_admitted(node, location)?;
        self.population_nodes.insert(node.clone());
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
        if self.bound_sites.insert((bound.label.clone(), node.clone())) {
            self.record(node.clone(), OccurrenceRole::Type, site);
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
                let key = unit
                    .declared_key()
                    .cloned()
                    .ok_or_else(|| fault(location, KeyFault::UnheldUnit(unit.clone())))?;
                self.unit_nodes(key.clone(), location)?;
                Ok(key)
            }
            UnitDomain::Compound => {
                let terms: Vec<(NodeKey, Integer)> = match self.units.get(unit.clone()) {
                    Some(QuantityUnit::Compound(compound)) => compound
                        .terms()
                        .map(|(root, exponent)| (root.clone(), exponent.clone()))
                        .collect(),
                    Some(QuantityUnit::Declared(_)) | None => {
                        return Err(fault(location, KeyFault::UnheldUnit(unit)))
                    }
                };
                let mut members = Vec::with_capacity(terms.len());
                for (root, exponent) in terms {
                    self.unit_nodes(root.clone(), location)?;
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
    /// the unit table holds for it, as QSpec's `scalar_type`/`unit` or
    /// `scalar_type`/`dimension` node: its qualified name as its
    /// `declaration`, with a `declaration` occurrence at that declared name,
    /// an empty body, and a unit typed by its dimension. A node already in the graph
    /// is left as it is. The walk is a heap work list, bounded by the unit
    /// table's size.
    fn unit_nodes(&mut self, unit: NodeKey, location: &Location) -> Result<(), CheckRefusal> {
        let mut pending = vec![unit];
        while let Some(key) = pending.pop() {
            let Some(node) = self.units.nominal_node(&key).cloned() else {
                return Err(fault(location, KeyFault::UnheldNominal(key)));
            };
            if let Some(held) = self.graph.nodes.get(&key) {
                let form = match node.form {
                    NominalUnitForm::Unit { .. } => "unit",
                    NominalUnitForm::Dimension { .. } => "dimension",
                };
                if held.content.node_tag == NodeTag::ScalarType
                    && held.content.semantic_form == form
                {
                    continue;
                }
                return Err(fault(location, KeyFault::NominalKeyClash(key)));
            }
            let NominalDeclaration {
                qualified_declaration: qualified,
                ..
            } = node.declaration;
            let (kind, semantic_form, semantic_type, payload) = match node.form {
                NominalUnitForm::Unit {
                    dimension,
                    target,
                    scale,
                    offset,
                } => {
                    pending.push(dimension.clone());
                    pending.extend(target.clone());
                    let payload = UnitPayload::new(dimension.clone(), target, scale, offset);
                    (
                        QuantityKind::Unit,
                        "unit",
                        Some(dimension),
                        NominalPayload::Unit(payload),
                    )
                }
                NominalUnitForm::Dimension { terms } => {
                    pending.extend(terms.iter().map(|(base, _)| base.clone()));
                    (
                        QuantityKind::Dimension,
                        "dimension",
                        None,
                        NominalPayload::Dimension(DimensionPayload::new(terms)),
                    )
                }
            };
            let site = type_declaration(&quantity_origin(kind, &qualified.join(".")));
            // FR-322: a node an admitted domain package declares carries that
            // package's `ModelOwner` and no `declaration`; a node this unit
            // declares carries its name as its `declaration` and its
            // `SourceOwner`. Any other node (a definition-owned unit) keeps
            // its name and no owner.
            let source_owned = key.package() == self.package_name;
            let model_owner = if source_owned {
                None
            } else {
                self.foreign_nominal_owner(&key)
            };
            let model_alias = model_owner.as_ref().map(|(_, alias)| alias.clone());
            let (declaration, owner) = match model_owner {
                Some((owner, _)) => (None, Some(Owner::Model(owner))),
                None => (
                    Some(identifiers(qualified.iter().map(String::as_str), &site)?),
                    source_owned.then(|| Owner::Source(self.owner.clone())),
                ),
            };
            let inserted = self.insert_nominal(
                &site,
                key.clone(),
                NodeContent {
                    owner_model: None,
                    node_tag: NodeTag::ScalarType,
                    semantic_form,
                    semantic_type: semantic_type.map(Into::into),
                    declaration,
                    owner,
                    body: BodyTerm::aggregate(Vec::new()),
                },
                Some(payload),
            )?;
            if inserted {
                match model_alias {
                    // A model-owned node has no declaration of its own: root
                    // it at the `model` declaration that selects its package,
                    // as a model record is.
                    Some(alias) => {
                        self.record(key, OccurrenceRole::Generated, type_declaration(&alias));
                    }
                    None => self.record(key, OccurrenceRole::Declaration, site),
                }
            }
        }
        Ok(())
    }

    /// The `ModelOwner` of the dimension node `key` an admitted domain
    /// package declares: the package's identity and the dimension's IR node
    /// identity.
    /// The selecting model alias comes with it, the declaration its
    /// `generated` occurrence is rooted at.
    fn foreign_nominal_owner(&self, key: &NodeKey) -> Option<(ModelOwner, String)> {
        self.models.iter().find_map(|model| {
            model.dimensions.iter().find_map(|dimension| {
                let (dimension_key, _, _) =
                    crate::check::assemble::package_dimension(model, dimension, &mut Vec::new())?;
                if &dimension_key != key {
                    return None;
                }
                let owner =
                    ModelOwner::new(model.selection.identity.clone(), dimension.key.node.clone())
                        .ok()?;
                Some((owner, model.alias()?.to_owned()))
            })
        })
    }

    /// FR-094 "Dimension equivalence", FR-322-AC-118: one
    /// `scalar_type`/`dimension_equivalence` node per admitted equivalence,
    /// in source order: self-typed, no `declaration`, the unit's source
    /// owner, a body referencing its two dimension nodes in ascending key
    /// order, its `root_scale` in the nominal payload, and one `anchor`
    /// occurrence whose ordinal is the declaration's index in source order. The two
    /// dimension nodes it depends on are built as a declared unit's are.
    pub(crate) fn dimension_equivalence_nodes(
        &mut self,
        equivalences: &[DimensionEquivalence],
    ) -> Result<(), CheckRefusal> {
        for (index, equivalence) in equivalences.iter().enumerate() {
            let site = type_declaration(&equivalence_origin(index));
            for dimension in &equivalence.dimensions {
                // A model-owned dimension is no node of this unit's table:
                // its own model emits it.
                if self.units.nominal_node(dimension).is_some() {
                    self.unit_nodes(dimension.clone(), &site)?;
                }
            }
            let key = equivalence.key.clone();
            let inserted = self.insert_nominal(
                &site,
                key.clone(),
                NodeContent {
                    owner_model: None,
                    node_tag: NodeTag::ScalarType,
                    semantic_form: "dimension_equivalence",
                    semantic_type: Some(NodeRef::Admitted(key.clone())),
                    declaration: None,
                    owner: Some(Owner::Source(self.owner.clone())),
                    body: BodyTerm::aggregate(
                        equivalence
                            .dimensions
                            .iter()
                            .cloned()
                            .map(|dimension| {
                                MemberTerm::reference_node(NodeRef::Admitted(dimension))
                            })
                            .collect(),
                    ),
                },
                Some(NominalPayload::DimensionEquivalence(
                    DimensionEquivalencePayload::new(
                        equivalence.dimensions.clone(),
                        equivalence.root_scale.clone(),
                    ),
                )),
            )?;
            if inserted {
                // FR-322: the anchor's ordinal is the declaration's index
                // among the source's equivalence declarations.
                let ordinal = u64::try_from(index).unwrap_or(u64::MAX);
                self.record_at(key, OccurrenceRole::Anchor, ordinal, site);
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

/// The `Origin` name of the `index`th dimension-equivalence declaration, the
/// key its anchor occurrence's region resolves under.
pub(crate) fn equivalence_origin(index: usize) -> String {
    format!("dimension equivalence {index}")
}

fn mismatch(location: &Location) -> CheckRefusal {
    refuse(
        location,
        CheckCause::IllTyped(quire_exact::IllTypedCause::TypeMismatch),
    )
}

#[cfg(test)]
mod tests;
