// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-152 qualified-name binder for the systems-model kinds.

use crate::model::accounting::{Incomplete, Meter};
use crate::model::admitted::AdmittedPackage;
use crate::model::domain_package::DomainPackageRef;
use crate::model::key::{DeclarationKey, EffectiveId};
use crate::model::normalize::{EffectiveView, ModelRefusal, ModelRefusalCause};
use crate::model::systems::Kind;
use qsl_foundation::diagnostic::Code;

/// A model alias of the unit being checked and the selection it names.
#[derive(Clone, Copy, Debug)]
pub struct ModelAlias<'a> {
    /// The `M` of `M::pump_out`.
    pub alias: &'a str,
    /// The selection the alias names.
    pub selection: &'a DomainPackageRef,
}

/// A resolved systems-model element.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedSystemsElement {
    /// The element's kind.
    pub kind: Kind,
    /// The construct's declaration key.
    pub key: DeclarationKey,
    /// The effective declaration the name resolves to.
    pub effective: EffectiveId,
    /// The owning effective type, absent for an Interface (itself an
    /// effective type).
    pub owner: Option<EffectiveId>,
}

/// Why a qualified name did not resolve.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ResolveFailure {
    /// The name is refused.
    Refused(Box<ModelRefusal>),
    /// A `systems.resolve` charge was denied.
    Incomplete(Incomplete),
}

/// Stub.
pub fn resolve_systems(
    _admitted: &AdmittedPackage,
    _view: &EffectiveView,
    _aliases: &[ModelAlias<'_>],
    _required: Kind,
    name: &str,
    _meter: &mut Meter,
) -> Result<ResolvedSystemsElement, ResolveFailure> {
    Err(ResolveFailure::Refused(Box::new(ModelRefusal {
        code: Code::MissingDeclaration,
        cause: ModelRefusalCause::UnresolvedName {
            name: name.to_owned(),
            segment: 0,
        },
        detail: String::new(),
    })))
}
