// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-142 "Subjects and pre-check" and FR-145 "Systems": the two
//! `ModelSystem`s of a refinement's subjects, built the one way the engine
//! and replay both build them.
//!
//! The concrete system is over the refinement's concrete model, with the
//! concrete subject's request. The abstract system is over its abstract model
//! with the abstract subject's initial snapshots and the derived universes:
//! each abstract population's universe is the concrete universe of its
//! source population (ADR-020 RM-2).

use qsl_eval::simulation::model::{ModelSimulationRequest, ModelSystem, PopulationUniverse};
use qsl_package::CheckedPackage;
use qsl_semantics::check::CheckedRefinement;
use qsl_semantics::model::observation::{AdmissionFailure, DocumentRef};

use crate::refinement::map::subject_models;

/// Why a refinement's subjects have no systems.
#[derive(Debug)]
pub enum SubjectRefusal {
    /// A subject's model system did not admit (FR-106's record).
    Admission(Box<AdmissionFailure>),
    /// The refinement maps no population or object, so it names no model.
    NoModels,
    /// An abstract population's source population has no universe in the
    /// concrete request.
    NoUniverse,
}

/// The concrete and abstract systems of `refinement`.
pub struct RefinementSystems<'a> {
    /// The system whose states a refinement explores and maps.
    pub concrete: ModelSystem<'a>,
    /// The system the mapped states are built in and judged by.
    pub abstract_system: ModelSystem<'a>,
}

/// Build both systems of `refinement`: the concrete one from `concrete`, the
/// abstract one over `abstract_package` from `abstract_initial` and the
/// derived universes.
///
/// # Errors
///
/// [`SubjectRefusal`] when a system does not admit or a universe cannot be
/// derived.
pub fn open_systems<'a>(
    refinement: &CheckedRefinement,
    concrete: &ModelSimulationRequest<'a>,
    abstract_package: &'a CheckedPackage,
    abstract_initial: Vec<DocumentRef>,
) -> Result<RefinementSystems<'a>, SubjectRefusal> {
    let (concrete_model, abstract_model) =
        subject_models(refinement).ok_or(SubjectRefusal::NoModels)?;
    let universes = refinement
        .populations()
        .iter()
        .map(|row| {
            concrete
                .universes
                .iter()
                .find(|universe| universe.population == row.concrete_population)
                .map(|universe| PopulationUniverse {
                    population: row.abstract_population.clone(),
                    keys: universe.keys.clone(),
                })
                .ok_or(SubjectRefusal::NoUniverse)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let admit = |failure| SubjectRefusal::Admission(Box::new(failure));
    Ok(RefinementSystems {
        concrete: ModelSystem::over_model(concrete.clone(), concrete_model).map_err(admit)?,
        abstract_system: ModelSystem::over_model(
            ModelSimulationRequest {
                package: abstract_package,
                packages: concrete.packages,
                normalization: concrete.normalization,
                snapshots: concrete.snapshots,
                initial: abstract_initial,
                universes,
                observation: concrete.observation,
                expansion: concrete.expansion,
                meter: concrete.meter,
            },
            abstract_model,
        )
        .map_err(admit)?,
    })
}
