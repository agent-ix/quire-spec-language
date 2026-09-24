// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-094 (QSL-156 A4b): model-owned nodes, the `Reference<T>` and
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
use crate::check::refusal::{CheckCause, CheckRefusal, KeyFault, Location};
use crate::model::domain_package::{DomainPackage, DomainPackageRecord, DomainPackageRef};
use crate::model::key::DeclarationKey;
use crate::model::normalize::EffectiveView;
use crate::value::quantity::QuantityUnit;
use qsl_forms::DeclaredClauseKind;

/// One admitted domain package, as `check` keys its declarations (FR-094
/// "Inputs"): its model selection, its records by declaration key, and
/// its effective view's `type_identities`, read from `EffectiveId` to
/// `DeclarationKey`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdmittedModel {
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
    /// selection could differ (QSL-217).
    pub fn from_view(view: &EffectiveView) -> Self {
        Self::assemble(view.domain_package(), view)
    }

    fn assemble(domain_package: &DomainPackage, view: &EffectiveView) -> Self {
        Self {
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
            selection: domain_package.model_selection.clone(),
            records: domain_package
                .records
                .iter()
                .map(|record| (record.key().clone(), record.clone()))
                .collect(),
            types: types.into_iter().collect(),
        }
    }

    /// The domain package's model selection.
    pub fn selection(&self) -> &DomainPackageRef {
        &self.selection
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
        self.correspondence.insert(declaration.clone(), key);
        Ok(key)
    }

    /// The model node of the object type `T` a `Reference<T>` names.
    pub(super) fn object_node(
        &mut self,
        target: EffectiveId,
        location: &Location,
    ) -> Result<NodeKey, CheckRefusal> {
        let declaration = self
            .models
            .iter()
            .find_map(|model| model.types.get(&target))
            .cloned()
            .ok_or_else(|| fault(location, KeyFault::UnknownEffectiveId(target)))?;
        self.model_node(&declaration, location)
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
    pub(super) fn population_type(
        &mut self,
        target: EffectiveId,
        maximum: u64,
        location: &Location,
    ) -> Result<NodeKey, CheckRefusal> {
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

/// Every model correspondence entry, `(node key, DeclarationKey)`.
pub(super) fn correspondence_entries(
    correspondence: BTreeMap<DeclarationKey, NodeKey>,
) -> Vec<(NodeKey, DeclarationKey)> {
    correspondence
        .into_iter()
        .map(|(declaration, key)| (key, declaration))
        .collect()
}

#[cfg(test)]
mod tests;
