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

use qsl_eval::model_subject::{ModelInstance, ModelSubject, ProtocolSubject, RoleBinding};
use qsl_eval::simulation::model::{ModelSimulationRequest, ModelSystem, PopulationUniverse};
use qsl_eval::simulation::protocol::ResolvedMemory;
use qsl_package::CheckedPackage;
use qsl_semantics::check::{AbstractSide, CheckedRefinement};
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
    /// The refinement's step rows select a concrete protocol (FR-136). The
    /// systems here explore a concrete model, and a protocol subject's
    /// refinement check (FR-142, FR-145) is not built yet.
    ProtocolSubject,
    /// The abstract subject is a protocol clause, whose steps FR-148 decides
    /// and no model system does.
    ProtocolAbstractSide,
}

/// The concrete and abstract systems of `refinement`.
pub struct RefinementSystems<'a> {
    /// The system whose states a refinement explores and maps.
    pub concrete: ModelSystem<'a>,
    /// The system the mapped states are built in and judged by.
    pub abstract_system: ModelSystem<'a>,
    /// The abstract model request from which protocol systems are built.
    pub abstract_request: ModelSimulationRequest<'a>,
    /// The selected abstract model's package identity.
    pub abstract_model: String,
    /// The admitted abstract protocol system when the abstract side is a
    /// protocol subject. Replay uses the same system that produced the run.
    pub abstract_protocol: Option<super::protocol_system::AbstractProtocolSystem<'a>>,
}

/// Build both systems of `refinement`: the concrete one from `concrete`, the
/// abstract one over `abstract_package` from `abstract_initial` and the
/// derived universes.
///
/// # Errors
///
/// [`SubjectRefusal`] when a system does not admit, a universe cannot be
/// derived or the concrete subject is a protocol.
pub fn open_systems<'a>(
    refinement: &CheckedRefinement,
    concrete: &ModelSimulationRequest<'a>,
    abstract_package: &'a CheckedPackage,
    abstract_initial: Vec<DocumentRef>,
) -> Result<RefinementSystems<'a>, SubjectRefusal> {
    open_systems_inner(
        refinement,
        concrete,
        abstract_package,
        abstract_initial,
        false,
    )
}

/// Build model mapping systems and the abstract protocol system selected by
/// the admitted refinement. The request supplies its `over` binding; the
/// clause, roles and resolved default memory come from the checked package.
pub fn open_protocol_systems<'a>(
    refinement: &CheckedRefinement,
    concrete: &ModelSimulationRequest<'a>,
    abstract_package: &'a CheckedPackage,
    abstract_initial: Vec<DocumentRef>,
    over: ModelInstance,
) -> Result<RefinementSystems<'a>, SubjectRefusal> {
    let mut systems = open_systems_inner(
        refinement,
        concrete,
        abstract_package,
        abstract_initial,
        true,
    )?;
    let AbstractSide::Protocol { clause, .. } = refinement.abstract_side() else {
        return Err(SubjectRefusal::ProtocolAbstractSide);
    };
    let protocol = abstract_package
        .graph()
        .protocol(clause)
        .ok_or(SubjectRefusal::ProtocolAbstractSide)?;
    let roles = protocol
        .roles
        .iter()
        .filter(|role| !role.replicated)
        .map(|role| {
            role.model_selection
                .clone()
                .or_else(|| {
                    role.context
                        .as_ref()
                        .map(|context| context.original().package().to_owned())
                })
                .map(|model| RoleBinding {
                    role: role.name.clone(),
                    model,
                })
                .ok_or(SubjectRefusal::ProtocolAbstractSide)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let memory =
        ResolvedMemory::resolve(protocol, &[]).map_err(|_| SubjectRefusal::ProtocolAbstractSide)?;
    let subject = ProtocolSubject {
        model: ModelSubject {
            simulation: systems.abstract_request.clone(),
            proof_bounds: Vec::new(),
        },
        protocol: clause.clone(),
        over,
        roles,
        memory,
    };
    let model = ModelSystem::over_model(systems.abstract_request.clone(), &systems.abstract_model)
        .map_err(|failure| SubjectRefusal::Admission(Box::new(failure)))?;
    let protocol_system = super::protocol_system::AbstractProtocolSystem::new(model, &subject)
        .map_err(|_| SubjectRefusal::ProtocolAbstractSide)?;
    systems.abstract_protocol = Some(protocol_system);
    Ok(systems)
}

fn open_systems_inner<'a>(
    refinement: &CheckedRefinement,
    concrete: &ModelSimulationRequest<'a>,
    abstract_package: &'a CheckedPackage,
    abstract_initial: Vec<DocumentRef>,
    allow_protocol: bool,
) -> Result<RefinementSystems<'a>, SubjectRefusal> {
    if refinement.concrete_protocol().is_some() {
        return Err(SubjectRefusal::ProtocolSubject);
    }
    if !allow_protocol && !matches!(refinement.abstract_side(), AbstractSide::Model) {
        return Err(SubjectRefusal::ProtocolAbstractSide);
    }
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
    let abstract_request = ModelSimulationRequest {
        package: abstract_package,
        packages: concrete.packages,
        normalization: concrete.normalization,
        snapshots: concrete.snapshots,
        initial: abstract_initial,
        universes,
        observation: concrete.observation,
        expansion: concrete.expansion,
        meter: concrete.meter,
    };
    Ok(RefinementSystems {
        concrete: ModelSystem::over_model(concrete.clone(), concrete_model).map_err(admit)?,
        abstract_system: ModelSystem::over_model(abstract_request.clone(), abstract_model)
            .map_err(admit)?,
        abstract_request,
        abstract_model: abstract_model.to_owned(),
        abstract_protocol: None,
    })
}
