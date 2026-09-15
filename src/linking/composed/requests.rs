// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-036: retain every requested clause/capability pair and its disposition.
//!
//! When the compiler hands a linked subject to a downstream checker or backend,
//! each requested pair keeps an explicit disposition. An unsupported request is
//! never converted into success and never removed, and complete aggregate
//! success is unavailable while any required request is unsupported, refused or
//! unfinished.
//!
//! Nothing here checks, proves or executes a clause body. `Admitted` means only
//! that the request may be handed downstream; the selected backend's declared
//! support never rewrites the admitted source, profile or model selection.

use super::binding;
use super::{DeclarationId, SyntaxNamespace};
use crate::syntax::composed::DeclarationKind;

/// The declared semantic family of a requested declaration.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Family {
    /// Reusable Boolean declaration.
    Predicate,
    /// Invariant or operation pre/postcondition.
    State,
    /// Temporal obligation with an explicit clock interpretation.
    Temporal,
    /// Choreography declaration.
    Protocol,
}

/// A requested claim, independent of which declaration it is requested for.
///
/// These are request labels consumed by a later stage. Selecting one grants no
/// checking, lowering or execution here.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Capability {
    /// Static family admission of the declaration's own body.
    FamilyCheck,
    /// A state operation evaluated against supplied assessment inputs.
    StateOperation,
    /// Finite replay of a choreography declaration.
    FiniteReplay,
    /// Independent projection of one temporal obligation over its own window.
    TemporalProjection,
}

impl Capability {
    /// Families for which this claim is defined at all, in stable order.
    pub fn families(self) -> &'static [Family] {
        match self {
            Self::FamilyCheck => &[
                Family::Predicate,
                Family::State,
                Family::Temporal,
                Family::Protocol,
            ],
            Self::StateOperation => &[Family::State],
            Self::FiniteReplay => &[Family::Protocol],
            Self::TemporalProjection => &[Family::Temporal],
        }
    }
}

/// One requested clause/capability pair over a linked subject.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Request {
    /// Namespace-local declaration this claim is requested for.
    pub declaration: DeclarationId,
    /// Requested claim.
    pub capability: Capability,
    /// Whether complete aggregate success depends on this request.
    pub required: bool,
}

/// The selected backend's own declaration of what it admits.
///
/// This is the caller's statement about a downstream implementation. It selects
/// dispositions; it never selects or rewrites the static subject.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Backend<'a> {
    /// Selected backend implementation identity.
    pub identity: &'a str,
    /// Claims this backend declares it implements.
    pub capabilities: &'a [Capability],
    /// Semantic families whose bodies this backend declares it admits.
    pub families: &'a [Family],
}

/// Assessment inputs consumed only by later stages.
///
/// The linker accepts none of these. They are retained as assessment provenance
/// so that changing one is visible without touching any static component.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Assessment<'a> {
    /// Selected finite population, when the caller selected one.
    pub population: Option<&'a str>,
    /// Selected snapshot or window.
    pub window: Option<&'a str>,
    /// Selected observation trace.
    pub trace: Option<&'a str>,
    /// Selected backend and its declared support.
    pub backend: Backend<'a>,
}

/// Retained assessment provenance, separate from the static subject.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssessmentProvenance {
    /// Selected finite population.
    pub population: Option<String>,
    /// Selected snapshot or window.
    pub window: Option<String>,
    /// Selected observation trace.
    pub trace: Option<String>,
    /// Selected backend implementation identity.
    pub backend: String,
    /// Claims the selected backend declared.
    pub capabilities: Vec<Capability>,
    /// Families the selected backend declared it admits.
    pub families: Vec<Family>,
}

/// One requested pair's typed disposition. Each cause is distinct; none of them
/// asserts that a clause body was checked, proved or executed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Disposition {
    /// The subject bound its names and the backend declares this claim and
    /// family. Downstream checking still applies in full.
    Admitted,
    /// The claim is recognized and applicable, but the selected backend does
    /// not declare it.
    UnsupportedCapability,
    /// The backend declares the claim but does not admit this family's bodies.
    UnsupportedFamily(Family),
    /// The claim is not defined for this declaration's family at all.
    InapplicableCapability(Family),
    /// Static binding refused this declaration; no claim over it can proceed.
    RefusedSubject,
    /// Binding did not finish for this declaration.
    UnfinishedSubject,
    /// The request names no declaration in this namespace.
    UnknownSubject,
}

impl Disposition {
    /// Whether a downstream stage may receive this request at all.
    pub fn admitted(self) -> bool {
        matches!(self, Self::Admitted)
    }
}

/// One retained request and its disposition. Requests are never dropped.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Response {
    /// The original requested pair, unchanged.
    pub request: Request,
    /// The declaration's family, absent when the subject is unknown.
    pub family: Option<Family>,
    /// Typed disposition for this pair only.
    pub disposition: Disposition,
}

/// Whether complete aggregate success remains attainable for these requests.
///
/// Complete success is a downstream result and is never granted at this stage.
/// This reports only whether a required request has already made it impossible.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Aggregate {
    /// Every required request was admitted for downstream processing.
    Attainable,
    /// A required request was not admitted; the index addresses responses().
    Unavailable { response: usize },
}

/// A consumer's requested inventory does not agree with this report.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InventoryGap {
    /// The report holds no response for this requested index.
    MissingResponse { request: usize },
    /// The report holds a response the consumer did not request.
    ExtraResponse { response: usize },
    /// The response at this index answers a different requested pair.
    MismatchedResponse { request: usize },
}

/// Every requested pair with its disposition, plus the assessment provenance
/// that selected those dispositions.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Report {
    assessment: AssessmentProvenance,
    responses: Vec<Response>,
    aggregate: Aggregate,
}

impl Report {
    /// Retained assessment inputs, never part of the static subject.
    pub fn assessment(&self) -> &AssessmentProvenance {
        &self.assessment
    }

    /// Every requested pair, in the caller's own request order.
    pub fn responses(&self) -> &[Response] {
        &self.responses
    }

    /// One requested pair's disposition, by request index.
    pub fn response(&self, index: usize) -> Option<Response> {
        self.responses.get(index).copied()
    }

    /// First retained disposition for an exact clause/capability pair.
    pub fn disposition(
        &self,
        declaration: DeclarationId,
        capability: Capability,
    ) -> Option<Disposition> {
        self.responses
            .iter()
            .find(|response| {
                response.request.declaration == declaration
                    && response.request.capability == capability
            })
            .map(|response| response.disposition)
    }

    /// Whether complete aggregate success remains attainable.
    pub fn aggregate(&self) -> Aggregate {
        self.aggregate
    }

    /// Declarations whose bodies a downstream checker may receive: exactly the
    /// admitted family-check requests. An unsupported family never appears
    /// here, so no unsupported body can be represented as checked.
    pub fn admitted_bodies(&self) -> Vec<DeclarationId> {
        self.responses
            .iter()
            .filter(|response| {
                response.request.capability == Capability::FamilyCheck
                    && response.disposition.admitted()
            })
            .map(|response| response.request.declaration)
            .collect()
    }

    /// Check that this report retains exactly the consumer's requested
    /// inventory, pair for pair and in order. A deleted or added entry on
    /// either side yields its own typed gap.
    pub fn retains(&self, requests: &[Request]) -> Result<(), InventoryGap> {
        for (index, request) in requests.iter().enumerate() {
            match self.responses.get(index) {
                None => return Err(InventoryGap::MissingResponse { request: index }),
                Some(response) if response.request != *request => {
                    return Err(InventoryGap::MismatchedResponse { request: index })
                }
                Some(_) => {}
            }
        }
        if self.responses.len() > requests.len() {
            return Err(InventoryGap::ExtraResponse {
                response: requests.len(),
            });
        }
        Ok(())
    }
}

/// Family of one parsed declaration, from its authored syntax alone.
pub fn family(namespace: &SyntaxNamespace, declaration: DeclarationId) -> Option<Family> {
    Some(match namespace.syntax(declaration)?.kind {
        DeclarationKind::Predicate { .. } => Family::Predicate,
        DeclarationKind::State { .. } => Family::State,
        DeclarationKind::Temporal { .. } => Family::Temporal,
        DeclarationKind::Protocol(_) => Family::Protocol,
    })
}

/// Record a disposition for every requested clause/capability pair.
///
/// Requests are answered in the caller's order and none is dropped or merged.
/// The selected backend's declared support and the supplied assessment inputs
/// are retained as provenance; neither is consulted by, or written back into,
/// the static binding this report reads.
pub fn report(
    bound: &binding::Report<'_>,
    assessment: &Assessment<'_>,
    requests: &[Request],
) -> Report {
    let namespace = bound.namespace();
    let mut responses = Vec::with_capacity(requests.len());
    for request in requests {
        let family = family(namespace, request.declaration);
        let disposition = match (family, bound.disposition(request.declaration)) {
            (None, _) | (_, None) => Disposition::UnknownSubject,
            (_, Some(binding::Disposition::Refused)) => Disposition::RefusedSubject,
            (_, Some(binding::Disposition::Unfinished)) => Disposition::UnfinishedSubject,
            (Some(family), Some(binding::Disposition::NamesResolved)) => {
                if !request.capability.families().contains(&family) {
                    Disposition::InapplicableCapability(family)
                } else if !assessment
                    .backend
                    .capabilities
                    .contains(&request.capability)
                {
                    Disposition::UnsupportedCapability
                } else if !assessment.backend.families.contains(&family) {
                    Disposition::UnsupportedFamily(family)
                } else {
                    Disposition::Admitted
                }
            }
        };
        responses.push(Response {
            request: *request,
            family,
            disposition,
        });
    }
    let aggregate = responses
        .iter()
        .position(|response| response.request.required && !response.disposition.admitted())
        .map_or(Aggregate::Attainable, |response| Aggregate::Unavailable {
            response,
        });
    Report {
        assessment: AssessmentProvenance {
            population: assessment.population.map(str::to_owned),
            window: assessment.window.map(str::to_owned),
            trace: assessment.trace.map(str::to_owned),
            backend: assessment.backend.identity.to_owned(),
            capabilities: assessment.backend.capabilities.to_vec(),
            families: assessment.backend.families.to_vec(),
        },
        responses,
        aggregate,
    }
}
