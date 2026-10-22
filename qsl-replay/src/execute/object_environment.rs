// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-098 "The object environment": the unused-environment check, the
//! admission of the environment document by FR-106, and FR-364's `resolve`
//! of each reference argument, in that order, before the call.

use std::collections::BTreeSet;

use qsl_foundation::diagnostic::Code;
use qsl_foundation::digest::DigestDomain;
use qsl_foundation::source::provenance::RawSourceRef;
use qsl_semantics::model::object_environment::ObjectEnvironment;
use qsl_semantics::model::observation::{
    admit_object_environment, AdmissionFailure, BundleKey, DocumentRef, ObservationOrigin,
    ObservationRole, ResolveBundle,
};
use quire_exact::{NodeKey, ObjectReference, ValueType};
use quire_semantic_value::catalog::InternalFault;
use quire_semantic_value::declaration::{CompositeShape, TypeEnvironment};
use quire_semantic_value::relationship_extent::Resolution;

use super::{Recompiled, ReplayRefusal};
use crate::request::ReplayRequest;

/// Why a reference argument is not held by the environment (FR-364 `resolve`).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Unresolved {
    /// A complete population covering the object's type lacks it:
    /// `dangling_reference`/`absent-target-in-complete-population`.
    Dangling,
    /// No complete evidence decides it: `incomplete_population`/
    /// `incomplete-scope`.
    Undecidable,
}

/// Why the object environment of a replay was refused (FR-098 steps 3 and 4).
#[derive(Debug)]
pub enum ObjectEnvironmentFailure {
    /// FR-106 refused the environment document, with its own record.
    Admission(AdmissionFailure),
    /// A reference argument the admitted environment does not hold.
    Reference {
        /// The parameter's zero-based declared position.
        position: usize,
        /// The reference as converted.
        reference: Box<ObjectReference>,
        /// FR-364's verdict.
        resolution: Unresolved,
    },
}

impl ObjectEnvironmentFailure {
    /// The catalog code.
    pub fn code(&self) -> Code {
        match self {
            Self::Admission(
                AdmissionFailure::Refused(record) | AdmissionFailure::Incomplete(record),
            ) => record.code,
            Self::Admission(AdmissionFailure::Fault(_)) => Code::RuntimeInvariant,
            Self::Reference {
                resolution: Unresolved::Dangling,
                ..
            } => Code::DanglingReference,
            Self::Reference {
                resolution: Unresolved::Undecidable,
                ..
            } => Code::IncompletePopulation,
        }
    }

    /// The catalog cause, where the failure names one.
    pub fn cause(&self) -> Option<&'static str> {
        match self {
            Self::Admission(
                AdmissionFailure::Refused(record) | AdmissionFailure::Incomplete(record),
            ) => Some(record.cause),
            Self::Admission(AdmissionFailure::Fault(_)) => None,
            Self::Reference {
                resolution: Unresolved::Dangling,
                ..
            } => Some("absent-target-in-complete-population"),
            Self::Reference {
                resolution: Unresolved::Undecidable,
                ..
            } => Some("incomplete-scope"),
        }
    }

    /// Whether the evidence was incomplete rather than the input refused.
    pub fn is_incomplete(&self) -> bool {
        matches!(
            self,
            Self::Admission(AdmissionFailure::Incomplete(_))
                | Self::Reference {
                    resolution: Unresolved::Undecidable,
                    ..
                }
        )
    }
}

impl std::fmt::Display for ObjectEnvironmentFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Admission(failure) => write!(f, "{failure:?}"),
            Self::Reference {
                position,
                resolution,
                ..
            } => write!(
                f,
                "the reference argument at position {position} is not held: {resolution:?}"
            ),
        }
    }
}

/// Whether a parameter of `declared` holds a `Reference`, directly or nested
/// in an `Option`, record, tuple, union or collection.
pub(super) fn holds_reference(types: &TypeEnvironment, declared: &[ValueType]) -> bool {
    let mut pending: Vec<&ValueType> = declared.iter().collect();
    let mut entered = BTreeSet::new();
    while let Some(value_type) = pending.pop() {
        match value_type {
            ValueType::Reference(_) => return true,
            ValueType::Option(payload) => pending.push(payload),
            ValueType::Collection(collection) => pending.push(collection.element()),
            ValueType::Composite(declaration) => {
                if !entered.insert(*declaration) {
                    continue;
                }
                let Some(composite) = types.composite(*declaration) else {
                    continue;
                };
                match composite.shape() {
                    CompositeShape::Record(fields) => {
                        pending.extend(fields.iter().map(|field| field.value_type()));
                    }
                    CompositeShape::Tuple(positions) => pending.extend(positions.iter()),
                    CompositeShape::Union(members) => {
                        pending.extend(members.iter().flat_map(|member| member.positions()));
                    }
                }
            }
            ValueType::Boolean
            | ValueType::Integer
            | ValueType::Int(_)
            | ValueType::Rational(_)
            | ValueType::Decimal(_)
            | ValueType::Float(_)
            | ValueType::Quantity(_)
            | ValueType::String
            | ValueType::Text(_)
            | ValueType::Enum(_)
            | ValueType::Population(_)
            | ValueType::Uuid
            | ValueType::Timestamp => {}
        }
    }
    false
}

/// FR-106's refusal or incomplete record is the environment's; a broken
/// admission invariant stays a fault (ADR-013 C-09), as every other admission
/// does.
pub(super) fn environment_refusal(failure: AdmissionFailure) -> ReplayRefusal {
    match failure {
        AdmissionFailure::Fault(_) => ReplayRefusal::Admission(failure),
        AdmissionFailure::Refused(_) | AdmissionFailure::Incomplete(_) => {
            ReplayRefusal::ObjectEnvironment(ObjectEnvironmentFailure::Admission(failure))
        }
    }
}

fn fault(invariant: &'static str) -> ReplayRefusal {
    ReplayRefusal::Fault(InternalFault::new("replay", invariant))
}

/// FR-098 steps 3 and 4: admit the environment document `request` names,
/// then decide `resolve` for each of `references`, the reference arguments
/// as converted: `(position, reference)` in parameter and walk order.
pub(super) fn admit(
    request: &ReplayRequest,
    compiled: &Recompiled,
    function: NodeKey,
    references: &[(usize, ObjectReference)],
    limits: &crate::limits::CallerLimits,
) -> Result<ObjectEnvironment, ReplayRefusal> {
    let Some(named) = request.object_environment() else {
        return Err(fault("environment-admitted-only-when-named"));
    };
    let package = compiled.checked.package();
    let provision = request
        .byte_provision()
        .entries()
        .filter(|(digest, _)| digest.domain() == DigestDomain::Sha256Jcs)
        .map(|(digest, bytes)| (*digest.as_bytes(), bytes.to_vec()))
        .collect();
    let labels = named.identity();
    let selected = DocumentRef {
        authority: labels.authority.clone(),
        identity: labels.identity.clone(),
        revision_namespace: labels.revision_namespace.clone(),
        revision: labels.revision.clone(),
        digest: *named.digest().as_bytes(),
    };
    let held: Vec<ObjectReference> = references
        .iter()
        .map(|(_, reference)| reference.clone())
        .collect();
    let (environment, _usage) = admit_object_environment(
        package.graph().model_selections(),
        package.graph().scope().types(),
        &compiled.packages,
        compiled.model_limits,
        &provision,
        &selected,
        limits.observation,
        &held,
    )
    .map_err(environment_refusal)?;

    let unit = request
        .source_digests()
        .first()
        .ok_or_else(|| fault("decoded-request-names-a-source"))?;
    let source = RawSourceRef::new(unit.authority(), unit.identity(), unit.digest())
        .map_err(|_| fault("package-source-names-a-source-document"))?;
    let key = BundleKey {
        source,
        clause: function,
        documents: vec![selected.digest],
        role: ObservationRole::Current,
        origin: ObservationOrigin::Document(selected.digest),
    };
    let bundle = ResolveBundle::over(&environment, &key, package.graph().scope().types())
        .map_err(environment_refusal)?;
    for (position, reference) in references {
        let resolution = match bundle.resolve(reference) {
            Resolution::Present => continue,
            Resolution::Dangling => Unresolved::Dangling,
            Resolution::Undecidable => Unresolved::Undecidable,
        };
        return Err(ReplayRefusal::ObjectEnvironment(
            ObjectEnvironmentFailure::Reference {
                position: *position,
                reference: Box::new(reference.clone()),
                resolution,
            },
        ));
    }
    Ok(environment)
}
