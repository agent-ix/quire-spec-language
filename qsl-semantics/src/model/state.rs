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
use crate::model::key::DeclarationKey;
use crate::model::normalize::EffectiveView;
use crate::model::observation::{
    admit_initial_state, model_views, observe_state, state_fields, state_reference,
    AdmissionFailure, DocumentRef, ModelView, Observation, ObservationLimits, SelectedAnchor,
};

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

/// One population of the selected models and its declared member types.
#[derive(Clone, Debug)]
pub struct StatePopulation {
    /// The population's declaration key.
    pub key: DeclarationKey,
    /// The population's declared member types, in declaration order.
    pub member_types: Vec<DeclarationKey>,
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
        admit_initial_state(
            &self.views,
            &self.types,
            provision,
            selected,
            limits,
            self.limits.ancestor_steps,
        )
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
