// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-120: the layer-3 state-model seam a simulator of a checked package's
//! state family calls.
//!
//! [`StateModel`] re-derives the domain package views FR-106 admission reads
//! and keeps the checked [`TypeEnvironment`]. It admits an initial snapshot
//! into a [`ModelState`] ([`StateModel::admit_initial`]) and builds the
//! [`Observation`] FR-107 evaluates a clause over for any state
//! ([`StateModel::observation`]). The frame decision a simulator makes
//! between two states is [`crate::model::population::decide_frame`], called
//! directly over the model's [`StateModel::population_index`].

use std::collections::BTreeMap;

use quire_exact::{EffectiveId, ObjectReference, Value, ValueType};
use quire_semantic_value::declaration::TypeEnvironment;
use quire_semantic_value::relationship_extent::RelationshipExtent;

use crate::model::accounting::ModelNormalizationLimits;
use crate::model::domain_package::{DomainPackageRecord, DomainPackageRef};
use crate::model::index::ModelIndex;
use crate::model::normalize::EffectiveView;
use crate::model::observation::{
    admit_initial_state, model_views, observe_state, state_fields, state_reference,
    AdmissionFailure, AdmissionRecord, DocumentRef, ModelView, Observation, ObservationLimits,
    PathStep, SelectedAnchor,
};
use quire_checked::key::DeclarationKey;

/// One complete, closed population state: for each population, its objects
/// by declared key.
#[derive(Clone, Debug)]
pub struct ModelState {
    /// Each listed population's objects, by the object's declared key.
    pub populations: BTreeMap<DeclarationKey, BTreeMap<String, StateObject>>,
    /// The relationship extents the state lists, one per listed
    /// relationship.
    pub extents: Vec<RelationshipExtent<DeclarationKey>>,
}

/// One object of a [`ModelState`].
#[derive(Clone, Debug)]
pub struct StateObject {
    /// The object's most-specific type.
    pub type_identity: DeclarationKey,
    /// The object's present declared field values, by the field's own
    /// declaration key. A field declared `?` that the object leaves absent
    /// has no entry.
    pub fields: BTreeMap<DeclarationKey, Value>,
}

/// One declared field of an object type, in the type's effective attribute
/// order.
#[derive(Clone, Debug)]
pub struct StateField {
    /// The field's own declaration key, the key a [`StateObject`] holds its
    /// value under.
    pub key: DeclarationKey,
    /// The field's declared name.
    pub name: String,
    /// The field's declared value type.
    pub value_type: ValueType,
    /// Whether the field is declared `?` and so may be absent.
    pub optional: bool,
}

/// A `min` or `max` bound on the quantities a field holds (QSpec FR-154
/// "Unit bounds"): read from a constraint on the unit the field is typed by,
/// or on an alias on the way to it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MemberBound {
    /// The field's own declaration key, the key a [`StateObject`] holds its
    /// value under.
    pub field: DeclarationKey,
    /// The constraint's declaration key.
    pub constraint: DeclarationKey,
    /// `max` or `min`.
    pub upper: bool,
    /// The bound, a magnitude in the unit.
    pub bound: quire_exact::Rational,
}

impl MemberBound {
    /// The constraint's keyword, `max` or `min`.
    pub fn keyword(&self) -> &'static str {
        if self.upper {
            "max"
        } else {
            "min"
        }
    }

    /// The bound spelled canonically, as the constraint's `operands.value`
    /// is: `n`, or `n/d` when the denominator is not 1.
    pub fn canonical_bound(&self) -> String {
        if self.bound.denominator() == &quire_exact::Integer::from(1_i64) {
            self.bound.numerator().to_string()
        } else {
            self.bound.to_string()
        }
    }

    /// Whether the quantity `value` satisfies the bound, equality
    /// satisfying; `true` for any value that is no quantity.
    pub fn admits(&self, value: &Value) -> bool {
        let Value::Quantity(quantity) = value else {
            return true;
        };
        if self.upper {
            quantity.magnitude() <= &self.bound
        } else {
            quantity.magnitude() >= &self.bound
        }
    }

    /// The first quantity `value` holds that violates the bound, with its
    /// path below the field: the value itself, each element of a
    /// collection, or the payload of a present optional value.
    pub fn violation(&self, value: &Value) -> Option<Vec<PathStep>> {
        match value {
            Value::Collection(collection) => {
                collection
                    .elements()
                    .iter()
                    .enumerate()
                    .find_map(|(index, element)| {
                        let mut path = self.violation(element)?;
                        path.insert(0, PathStep::Index(index));
                        Some(path)
                    })
            }
            Value::Option(option) => {
                let mut path = self.violation(option.payload()?)?;
                path.insert(0, PathStep::OptionValue);
                Some(path)
            }
            value => (!self.admits(value)).then(Vec::new),
        }
    }
}

/// One population of the selected models and its declared member types.
#[derive(Clone, Debug)]
pub struct StatePopulation {
    /// The population's declaration key.
    pub key: DeclarationKey,
    /// The population's declared member types, in declaration order.
    pub member_types: Vec<DeclarationKey>,
    /// Whether the population is declared `symmetric` (FR-150).
    pub symmetric: bool,
}

/// FR-106's re-derived model views and the checked type environment.
#[derive(Debug)]
pub struct StateModel {
    pub(crate) views: Vec<ModelView>,
    pub(crate) types: TypeEnvironment,
    pub(crate) limits: ModelNormalizationLimits,
}

impl StateModel {
    /// Re-admit and re-normalize the domain packages `model_selections` name
    /// against `packages`, as FR-106 admission does.
    ///
    /// # Errors
    ///
    /// An [`AdmissionFailure`] when the packages do not re-normalize within
    /// `limits`.
    pub fn new(
        model_selections: &[DomainPackageRef],
        types: &TypeEnvironment,
        packages: &BTreeMap<[u8; 32], Vec<u8>>,
        limits: ModelNormalizationLimits,
    ) -> Result<Self, AdmissionFailure> {
        Ok(Self {
            views: model_views(model_selections, packages, limits)?,
            types: types.clone(),
            limits,
        })
    }

    /// The model of a package still being checked, over the effective views
    /// its `model` declarations admitted. The views stand for the same
    /// packages [`Self::new`] re-admits from their bytes.
    pub(crate) fn from_views(
        views: impl IntoIterator<Item = EffectiveView>,
        types: &TypeEnvironment,
        limits: ModelNormalizationLimits,
    ) -> Self {
        Self {
            views: views.into_iter().map(ModelView::new).collect(),
            types: types.clone(),
            limits,
        }
    }

    /// Admit the `quire.state.snapshot/v1` document `selected` names in
    /// `provision` as an initial state: FR-106 check 1 (read), check 3 (an
    /// `observation` of `current` anchored by an `initialization`), check 4
    /// (model), check 6 (populations and values), check 7 (every population
    /// the document lists is required complete) and check 8 (closure).
    ///
    /// # Errors
    ///
    /// The first [`AdmissionFailure`] in check order.
    pub fn admit_initial(
        &self,
        provision: &BTreeMap<[u8; 32], Vec<u8>>,
        selected: &DocumentRef,
        limits: ObservationLimits,
    ) -> Result<(ModelState, SelectedAnchor), AdmissionFailure> {
        let (state, anchor) = admit_initial_state(
            &self.views,
            &self.types,
            provision,
            selected,
            limits,
            self.limits.ancestor_steps,
        )?;
        // QSpec FR-154 "Unit bounds": a supplied value that violates a bound
        // refuses `invalid_runtime_input`/`invalid-value`.
        let bounds = self.member_bounds();
        for objects in state.populations.values() {
            for (key, object) in objects {
                for bound in &bounds {
                    let Some(value) = object.fields.get(&bound.field) else {
                        continue;
                    };
                    if let Some(path) = bound.violation(value) {
                        let mut record = AdmissionRecord {
                            code: qsl_foundation::diagnostic::Code::InvalidRuntimeInput,
                            cause: "invalid-value",
                            fields: BTreeMap::new(),
                            path,
                            kernel: None,
                        };
                        record.fields.insert("object", key.clone());
                        record.fields.insert("field", bound.field.node.clone());
                        record
                            .fields
                            .insert("constraint", bound.constraint.node.clone());
                        record.fields.insert("keyword", bound.keyword().to_owned());
                        record.fields.insert("bound", bound.canonical_bound());
                        return Err(AdmissionFailure::Refused(record));
                    }
                }
            }
        }
        Ok((state, anchor))
    }

    /// The `min`/`max` bounds on the quantities each field of the selected
    /// models holds (QSpec FR-154 "Unit bounds"): for a field typed by the
    /// unit `u`, directly, through aliases or as a sequence's items, every
    /// bound on `u` itself and every bound on an alias the field's type
    /// passes. A bound on an alias bounds no other member of `u`.
    pub fn member_bounds(&self) -> Vec<MemberBound> {
        use crate::model::domain_package::ValueTypeRef;
        let mut bounds = Vec::new();
        for view in &self.views {
            let records = &view.view.domain_package().records;
            let record = |key: &DeclarationKey| records.iter().find(|record| record.key() == key);
            for field in records.iter().filter_map(|record| match record {
                DomainPackageRecord::FieldMember(field) => Some(field),
                _ => None,
            }) {
                let mut aliases = Vec::new();
                let mut value_type = &field.value_type;
                let mut steps = 0_usize;
                while let ValueTypeRef::Package(key) = value_type {
                    steps += 1;
                    value_type = match record(key) {
                        Some(DomainPackageRecord::AliasType(alias)) if steps <= records.len() => {
                            aliases.push(key);
                            &alias.target
                        }
                        Some(DomainPackageRecord::SequenceType(sequence))
                            if steps <= records.len() =>
                        {
                            &sequence.items
                        }
                        _ => break,
                    };
                }
                let ValueTypeRef::Unit(unit) = value_type else {
                    continue;
                };
                for record in records {
                    let DomainPackageRecord::ValueBound(bound) = record else {
                        continue;
                    };
                    let applies = match &bound.alias {
                        Some(alias) => aliases.contains(&alias),
                        None => bound.target == ValueTypeRef::Unit(unit.clone()),
                    };
                    if applies {
                        bounds.push(MemberBound {
                            field: field.key.clone(),
                            constraint: bound.key.clone(),
                            upper: bound.upper,
                            bound: bound.value.clone(),
                        });
                    }
                }
            }
        }
        bounds
    }

    /// The [`Observation`] of `state`, named `identity`: the object
    /// environment of every object with every field value, and every listed
    /// population complete.
    ///
    /// # Errors
    ///
    /// A `Fault` when `state` is not a state of this model.
    pub fn observation(
        &self,
        state: &ModelState,
        identity: DocumentRef,
    ) -> Result<Observation, AdmissionFailure> {
        observe_state(&self.views, &self.types, state, identity)
    }

    /// The model index `population`'s declarations resolve in, for the
    /// [`FrameDecision`](crate::model::population::FrameDecision) over that
    /// population; `None` when no selected model declares it.
    pub fn population_index(&self, population: &DeclarationKey) -> Option<&ModelIndex> {
        self.views
            .iter()
            .find(|view| view.view.population(population).is_some())
            .map(|view| view.view.model_index())
    }

    /// The conformance walk's `ancestor_steps` ceiling, for
    /// [`FrameDecision::ancestor_steps`](crate::model::population::FrameDecision).
    pub fn ancestor_steps(&self) -> u64 {
        self.limits.ancestor_steps
    }

    /// Every population of the selected models, in ascending declaration
    /// key order.
    pub fn populations(&self) -> Vec<StatePopulation> {
        let mut populations: Vec<StatePopulation> = self
            .views
            .iter()
            .flat_map(|view| view.view.domain_package().records.iter())
            .filter_map(|record| match record {
                DomainPackageRecord::Population(population) => Some(StatePopulation {
                    key: population.key.clone(),
                    member_types: population.member_types.clone(),
                    symmetric: population.symmetric,
                }),
                _ => None,
            })
            .collect();
        populations.sort_by(|left, right| left.key.cmp(&right.key));
        populations
    }

    /// The effective identity of the declared type `key`.
    pub fn type_identity(&self, key: &DeclarationKey) -> Option<EffectiveId> {
        self.views
            .iter()
            .find_map(|view| view.view.type_identities().get(key).copied())
    }

    /// The declaration key of the type whose effective identity is
    /// `identity`.
    pub fn type_key(&self, identity: EffectiveId) -> Option<&DeclarationKey> {
        self.views
            .iter()
            .find_map(|view| view.declaration_key(identity))
    }

    /// Whether the object type `key` is declared abstract.
    pub fn is_abstract(&self, key: &DeclarationKey) -> bool {
        self.views
            .iter()
            .any(|view| view.view.model_index().is_abstract(key))
    }

    /// The fields of the object type `type_key`, in its effective attribute
    /// order; `None` when no selected model declares it.
    ///
    /// # Errors
    ///
    /// A `Fault` when a field's owner has no declaration key.
    pub fn fields(
        &self,
        type_key: &DeclarationKey,
    ) -> Result<Option<Vec<StateField>>, AdmissionFailure> {
        state_fields(&self.views, &self.types, type_key)
    }

    /// The reference to the object `object` of the type `type_key`, in the
    /// universe admission assigns it.
    ///
    /// # Errors
    ///
    /// A `Refused` record when `object` is empty, and a `Fault` when no
    /// selected model declares the type.
    pub fn object_reference(
        &self,
        type_key: &DeclarationKey,
        object: &str,
    ) -> Result<ObjectReference, AdmissionFailure> {
        state_reference(&self.views, type_key, object)
    }
}
