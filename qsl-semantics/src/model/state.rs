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

use quire_exact::Value;
use quire_semantic_value::declaration::TypeEnvironment;

use crate::model::accounting::ModelNormalizationLimits;
use crate::model::domain_package::DomainPackageRef;
use crate::model::index::ModelIndex;
use crate::model::key::DeclarationKey;
use crate::model::observation::{
    admit_initial_state, model_views, observe_state, AdmissionFailure, DocumentRef, ModelView,
    Observation, ObservationLimits, SelectedAnchor,
};

/// One complete, closed population state: for each population, its objects
/// by declared key.
#[derive(Clone, Debug)]
pub struct ModelState {
    /// Each listed population's objects, by the object's declared key.
    pub populations: BTreeMap<DeclarationKey, BTreeMap<String, StateObject>>,
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
}
