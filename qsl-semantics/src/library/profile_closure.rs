// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-036-AC-8: resolve one selected profile's exact dependency closure.
//!
//! This reports definition closure only. It neither admits the selected
//! profile for a particular use site nor builds a checked or emitted package.

use std::collections::BTreeSet;

use qsl_foundation::selection::DefinitionRef;

use super::bundle::{
    resolve_definition_closure, BundleRefusal, DefinitionCatalog, DefinitionRole, PackageLimits,
};

/// The exact protocol definition required by ObservationBinding.
const PROTOCOL_PROFILE: &str = "quire.protocol.finite-global/v1";
const OBSERVATION_BINDING: &str = "quire.observation.binding/v1";
const AUTHORITY: &str = "agent-ix";

/// Exact closure of one selected profile definition.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProfileClosure {
    selected: DefinitionRef,
    role: DefinitionRole,
    definitions: BTreeSet<DefinitionRef>,
}

impl ProfileClosure {
    /// The exact profile definition selected by the header identity.
    pub fn selected(&self) -> &DefinitionRef {
        &self.selected
    }

    /// The semantic role supplied by the selected profile definition.
    pub fn role(&self) -> DefinitionRole {
        self.role
    }

    /// Every exact definition reached by the shared dependency walk,
    /// including the selected profile itself.
    pub fn definitions(&self) -> &BTreeSet<DefinitionRef> {
        &self.definitions
    }
}

/// Why profile dependency closure could not be completed.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum ProfileClosureRefusal {
    /// The selected identity is not registered under the required authority.
    #[error("profile selection {0:?} is not registered")]
    MissingSelection(String),
    /// The selected profile's exact closure could not be resolved.
    #[error("profile dependency closure refused: {0}")]
    Dependency(Box<BundleRefusal>),
    /// ObservationBinding omitted, conflicted with, or resolved a definition
    /// that does not supply the required finite-global Protocol profile.
    #[error("ObservationBinding does not close over the registered protocol profile")]
    IncompatibleProtocolDependency,
}

/// Resolve `selected` and its exact transitive requirements from the caller's
/// registry. `limits` applies to the shared definition graph walk. This does
/// not perform use-site-kind admission.
pub fn resolve_profile_closure(
    selected: &DefinitionRef,
    catalog: &DefinitionCatalog,
    limits: PackageLimits,
) -> Result<ProfileClosure, ProfileClosureRefusal> {
    let definition = catalog
        .resolve(selected)
        .ok_or_else(|| ProfileClosureRefusal::MissingSelection(selected.identity().to_owned()))?;
    let role = definition.role();
    let graph = resolve_definition_closure(std::slice::from_ref(selected), catalog, limits)
        .map_err(|refusal| ProfileClosureRefusal::Dependency(Box::new(refusal)))?;
    let definitions: BTreeSet<_> = graph.definitions().keys().cloned().collect();

    if selected.identity() == OBSERVATION_BINDING {
        let protocol = DefinitionRef::new(AUTHORITY, PROTOCOL_PROFILE)
            .expect("the registered protocol profile identity is valid");
        let selected_definition = catalog
            .resolve(selected)
            .expect("the selected definition was resolved above");
        let protocol_definition = definitions
            .contains(&protocol)
            .then(|| graph.definitions().get(&protocol))
            .flatten();
        if role != DefinitionRole::Observation
            || !selected_definition.dependencies().contains(&protocol)
            || protocol_definition
                .is_none_or(|definition| definition.role() != DefinitionRole::Protocol)
        {
            return Err(ProfileClosureRefusal::IncompatibleProtocolDependency);
        }
    }

    Ok(ProfileClosure {
        selected: selected.clone(),
        role,
        definitions,
    })
}

/// Resolve a source-selected profile identity through the exact authority
/// already registered in `catalog`.
pub fn resolve_registered_profile(
    identity: &str,
    catalog: &DefinitionCatalog,
    limits: PackageLimits,
) -> Result<ProfileClosure, ProfileClosureRefusal> {
    let definition = catalog
        .definition(identity)
        .ok_or_else(|| ProfileClosureRefusal::MissingSelection(identity.to_owned()))?;
    resolve_profile_closure(definition.exact(), catalog, limits)
}
