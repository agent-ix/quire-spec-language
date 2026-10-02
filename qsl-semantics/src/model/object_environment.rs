// SPDX-License-Identifier: AGPL-3.0-or-later
//! The object environment S6a evaluates over: SV's core object closure plus
//! FR-089's populations map (ADR-011 §6.2).
//!
//! The closure -- the admitted objects and the reference closure between
//! them, keyed by kernel ids -- is SV's
//! [`ObjectClosure`].
//! This module holds only what SV cannot: FR-089's recorded `PopulationId`
//! -> [`PopulationBinding`] correspondence. `PopulationBinding` is `model`'s
//! (ADR-011 §6.1, "K is a leaf"), and §6.1's order `semantic_value < model`
//! forbids the upward import, so the populations map stays in `model`.

use std::collections::BTreeMap;
use std::sync::Arc;

use crate::model::population::PopulationBinding;
use quire_exact::PopulationId;
use quire_semantic_value::object_closure::ObjectClosure;

/// [`ObjectEnvironment::with_population`]'s refusal: `population_id` is
/// already recorded under a binding that does not equal the one this call
/// tried to record.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[error("population {population_id} is already recorded under a different binding")]
pub struct PopulationConflict {
    /// The colliding [`PopulationId`].
    pub population_id: PopulationId,
}

/// An object environment: SV's closed object closure, plus FR-089's
/// recorded `PopulationId` -> `PopulationBinding` correspondence. `model`
/// mints a `PopulationId` at binding-admission time
/// (`admit_binding`/`admit_invocation`), and the caller records the pair
/// here, in the same environment already threaded through
/// `CheckedPackage::call`/`evaluate` and `Machine`, before constructing the
/// `Value::Population(population_id)` argument that names it.
#[derive(Clone, Debug, Default)]
pub struct ObjectEnvironment {
    objects: ObjectClosure,
    populations: BTreeMap<PopulationId, Arc<PopulationBinding>>,
}

impl ObjectEnvironment {
    /// The environment over `objects`, with no population recorded yet.
    pub fn new(objects: ObjectClosure) -> Self {
        Self {
            objects,
            populations: BTreeMap::new(),
        }
    }

    /// The environment's core object closure.
    pub fn objects(&self) -> &ObjectClosure {
        &self.objects
    }

    /// Records `binding`'s admission under its own
    /// [`PopulationBinding::population_id`] -- FR-089's `PopulationId` ->
    /// `PopulationBinding` correspondence `model` mints at binding-admission
    /// time. Consuming builder: call once per admitted binding, before
    /// constructing the `Value::Population(population_id)` argument that
    /// names it, so [`Self::resolve_population`] can resolve it. Keyed by
    /// the binding's own id (never a separately supplied one), so a caller
    /// cannot record a binding under an id it does not carry.
    ///
    /// FR-089's own admission preimage (package, `population_key`, role --
    /// see `model::population::population_id_preimage`'s own doc) does not
    /// yet include the admitted document's content or its declared maximum,
    /// an open spec question. Two distinct
    /// bindings can therefore collide on one id within a single evaluation
    /// (for example, two invocations of the same population role with
    /// different declared maxima). This is the interim guard: recording an
    /// id already bound to an *equal* binding is `Ok` (idempotent
    /// re-admission, TC-291's own case), but recording a *different*
    /// binding under an id already bound refuses loudly, with
    /// [`PopulationConflict`], rather than silently letting the later
    /// admission overwrite the earlier one.
    pub fn with_population(
        mut self,
        binding: PopulationBinding,
    ) -> Result<Self, PopulationConflict> {
        let population_id = binding.population_id();
        if let Some(existing) = self.populations.get(&population_id) {
            if **existing != binding {
                return Err(PopulationConflict { population_id });
            }
            return Ok(self);
        }
        self.populations.insert(population_id, Arc::new(binding));
        Ok(self)
    }

    /// The `PopulationBinding` FR-089's recorded correspondence resolves
    /// `population_id` to, or `None` when `population_id` names no binding
    /// [`Self::with_population`] recorded in this environment
    /// (FR-089-AC-4: the evaluator refuses this case, never panicking or
    /// substituting a default binding).
    pub fn resolve_population(&self, population_id: PopulationId) -> Option<&PopulationBinding> {
        self.populations.get(&population_id).map(Arc::as_ref)
    }
}
