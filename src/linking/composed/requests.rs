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
//! that the request may be handed downstream for further processing; whether a
//! backend actually supports the claim is decided elsewhere (the `#185`
//! registry and `quire-contract-codegen`'s `negotiate_*`), never here.
//!
//! ## FR-077: no backend negotiation (ADR-010 OBS-003, ADR-012 §10)
//!
//! [`report`] takes no `Backend` parameter and performs no backend
//! negotiation: it records every requested pair as data
//! (declaration, capability kind, `required` flag, request index) and
//! reports only the admission-level dispositions that do not depend on
//! backend state (`Admitted`, `InapplicableCapability`, `RefusedSubject`,
//! `UnfinishedSubject`, `UnknownSubject`). The former backend-dependent
//! dispositions `UnsupportedCapability` and `UnsupportedFamily` are
//! removed, unconditionally: candidates and routing live only in the
//! `#185` registry (`qsl_route`), and every disposition that depends on
//! backend support is settled only by `quire-contract-codegen`'s
//! `negotiate_*` (quire-contract-codegen#86).
//!
//! ## One capability type (ADR-010 OBS-012)
//!
//! [`Request::capability`] carries [`qsl_semantics::check::Capability`], the
//! canonical ten-label FR-290 type (ADR-013 O-19), not a local request-label
//! enum: OBS-012 records that "the QSL four-variant request label enum is
//! replaced by the #213 type." [`families`] maps each of the ten kinds onto
//! the QSL declaration [`Family`] it applies to. This mapping is
//! **provisional**: it is inferred from FR-057's admitted-vocabulary table
//! (its "FR-290 family" column) plus the retired four-variant enum's own
//! `FiniteReplay` -> Protocol case, not itself published by any FR. Which
//! family records which `Requirements` is decided in #210
//! (agent-ix/quire-spec-language#210); this mapping should be revisited once
//! that lands.
//!
//! The mapping already has a concrete effect: a *required* capability
//! request whose kind is not in the requested declaration's `families()` set
//! becomes [`Disposition::InapplicableCapability`], and a required
//! inapplicable request makes the whole [`Report`] unavailable. For example,
//! a required `value-validity` request against a `State` or `Temporal`
//! declaration is `InapplicableCapability` under the current mapping, since
//! [`Capability::ValueValidity`] names only [`Family::Predicate`].
//!
//! Checking a declaration's body under its semantic family is language
//! admission, not a capability kind (FR-057, "Family-body admission"): no
//! capability request selects or withholds a body, so
//! [`Report::admitted_bodies`] is computed from binding disposition alone,
//! independent of `requests`.

use super::binding;
use super::{DeclarationId, SyntaxNamespace};
use crate::syntax::composed::DeclarationKind;
use qsl_semantics::check::Capability;

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

/// Declaration [`Family`] values for which `capability` is defined at all,
/// in stable order.
///
/// **Provisional**, pending #210 (agent-ix/quire-spec-language#210), which
/// owns which family records which `Requirements`: inferred from FR-057's
/// admitted-vocabulary table (its "FR-290 family" column) plus the retired
/// four-variant enum's own `FiniteReplay` -> Protocol case, not itself
/// published by any FR. `value` -> [`Family::Predicate`] (a predicate is "a
/// reusable Boolean declaration", exactly a value-validity claim's own
/// family); `state-model` -> [`Family::State`]; `finite-replay` ->
/// [`Family::Protocol`] (only a choreography declaration is replayed);
/// `temporal-trace` -> [`Family::Temporal`]; every other listed family is
/// `protocol` -> [`Family::Protocol`]. No kind is defined for more than one
/// [`Family`]: each of the ten [`Capability`] kinds names exactly one
/// FR-290 family. QSL's [`Family`] has four members where FR-290 has five
/// (`value`, `state-model`, `finite-replay`, `temporal-trace`, `protocol`
/// collapse the two protocol-shaped rows into one [`Family::Protocol`]),
/// so this is not a 1:1 mapping onto FR-290's own family set.
///
/// A required request whose [`Capability`] is not in the requested
/// declaration's `families()` set becomes
/// [`Disposition::InapplicableCapability`]: for example, a required
/// `value-validity` request against a `State` or `Temporal` declaration.
pub fn families(capability: Capability) -> &'static [Family] {
    match capability {
        Capability::ValueValidity => &[Family::Predicate],
        Capability::OperationContract => &[Family::State],
        Capability::FiniteReplay => &[Family::Protocol],
        Capability::TemporalSatisfaction => &[Family::Temporal],
        Capability::GlobalConformance
        | Capability::Monitorability
        | Capability::LocalProjection
        | Capability::Refinement
        | Capability::Realizability
        | Capability::Composition => &[Family::Protocol],
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

/// Assessment inputs consumed only by later stages.
///
/// The linker accepts none of these. They are retained as assessment provenance
/// so that changing one is visible without touching any static component.
///
/// FR-077: this type carries no backend-shaped field. A selected backend's
/// declared support is not an assessment input the composed linker reads;
/// it is computed downstream by the `#185` registry and settled by
/// `quire-contract-codegen`'s `negotiate_*`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Assessment<'a> {
    /// Selected finite population, when the caller selected one.
    pub population: Option<&'a str>,
    /// Selected snapshot or window.
    pub window: Option<&'a str>,
    /// Selected observation trace.
    pub trace: Option<&'a str>,
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
}

/// One requested pair's typed disposition. Each cause is distinct; none of them
/// asserts that a clause body was checked, proved or executed.
///
/// FR-077 (ADR-010 OBS-003): this type carries no `UnsupportedCapability`
/// and no `UnsupportedFamily` variant. Whether a backend supports a claim
/// is not decided here; `Admitted` means only that the pair passed the
/// admission-level checks this module can make without reading backend
/// state (the subject bound, and the claim applies to the declaration's
/// family).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Disposition {
    /// The subject bound its names and the claim applies to this
    /// declaration's family. Downstream checking, and any backend
    /// negotiation, still apply in full.
    Admitted,
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
    Unavailable {
        /// Index into responses() for the required request that was not admitted.
        response: usize,
    },
}

/// A consumer's requested inventory does not agree with this report.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InventoryGap {
    /// The report holds no response for this requested index.
    MissingResponse {
        /// Index into the consumer's requested inventory with no matching response.
        request: usize,
    },
    /// The report holds a response the consumer did not request.
    ExtraResponse {
        /// Index into responses() for the response the consumer did not request.
        response: usize,
    },
    /// The response at this index answers a different requested pair.
    MismatchedResponse {
        /// Index into the consumer's requested inventory whose response answers a different pair.
        request: usize,
    },
}

/// Every requested pair with its disposition, plus the assessment provenance
/// that selected those dispositions.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Report {
    assessment: AssessmentProvenance,
    responses: Vec<Response>,
    aggregate: Aggregate,
    admitted_bodies: Vec<DeclarationId>,
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

    /// Declarations whose bodies a downstream checker may receive: every
    /// declaration whose names resolved (binding disposition
    /// `NamesResolved`), independent of `requests` (FR-057, "Family-body
    /// admission": checking a declaration's body is language admission, not
    /// a capability kind, and no capability request selects or withholds
    /// it -- a declaration with no capability request over it at all is
    /// still an admitted body here, and one whose only requests are
    /// inapplicable or unrequired is unaffected). A refused or unfinished
    /// subject never appears here, so no such body can be represented as
    /// checked.
    pub fn admitted_bodies(&self) -> Vec<DeclarationId> {
        self.admitted_bodies.clone()
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
/// The supplied assessment inputs are retained as provenance; they are not
/// consulted by, or written back into, the static binding this report reads.
///
/// FR-077: this function takes no backend argument and reads no backend
/// state. It reports each pair's admission-level disposition only
/// (`Admitted` when the subject bound and the claim applies to the
/// declaration's family); whether any backend actually supports the claim
/// is computed downstream by the `#185` registry (`qsl_route`) and
/// settled by `quire-contract-codegen`'s `negotiate_*`.
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
                if !families(request.capability).contains(&family) {
                    Disposition::InapplicableCapability(family)
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
    // FR-057 "Family-body admission": every declaration whose names
    // resolved is an admitted body, unconditionally -- independent of
    // `requests` above, which may ask for no capability at all over a
    // given declaration, or only for one that turns out inapplicable.
    let admitted_bodies = bound
        .declarations()
        .iter()
        .filter(|declaration| {
            matches!(
                bound.disposition(declaration.declaration()),
                Some(binding::Disposition::NamesResolved)
            )
        })
        .map(|declaration| declaration.declaration())
        .collect();
    Report {
        assessment: AssessmentProvenance {
            population: assessment.population.map(str::to_owned),
            window: assessment.window.map(str::to_owned),
            trace: assessment.trace.map(str::to_owned),
        },
        responses,
        aggregate,
        admitted_bodies,
    }
}

/// TC-199 (FR-077-AC-1, FR-077-AC-2): `Backend` no longer exists in this
/// module -- `report` takes no parameter of a `Backend`-shaped type, and no
/// such value can be constructed to try to pass one in.
///
/// ```compile_fail,E0432
/// use quire_spec_language::linking::composed::requests::Backend;
/// ```
///
/// A positive control: the surviving `Assessment` type still imports
/// cleanly, so the failure above is the removed name, not a broken module
/// path.
///
/// ```
/// use quire_spec_language::linking::composed::requests::Assessment;
/// fn _use(_: Assessment<'_>) {}
/// ```
///
/// TC-199 (FR-077/FR-077-AC-1, FR-077-AC-2): the disposition type carries no
/// `UnsupportedCapability` and no `UnsupportedFamily` variant.
///
/// ```compile_fail,E0599
/// use quire_spec_language::linking::composed::requests::Disposition;
/// fn _use() { let _ = Disposition::UnsupportedCapability; }
/// ```
///
/// ```compile_fail,E0599
/// use quire_spec_language::linking::composed::requests::{Disposition, Family};
/// fn _use() { let _ = Disposition::UnsupportedFamily(Family::Temporal); }
/// ```
//
// This item carries the doctests above rather than an ordinary `#[test]`
// function because their evidence is inherently compile-time (FR-077-AC-1/2
// require that certain code *fails to compile*, not that it panics at
// runtime) -- a `#[cfg(doctest)]`-gated item is this repository's existing
// pattern for scoping such doctests out of normal builds (they document no
// public API and would otherwise pollute rustdoc output).
//
// The invented "Tracing: TC-199 / ACs: ..." doc-comment header this item
// carried before (PR #305 review round 2, finding 6) is not how this
// repository traces anything: every other traced item uses the real
// `#[trace(...)]` attribute from `ix-trace-rs`, which quire-rs's FR-051
// parses statically to mint `verifies` relations regardless of what kind of
// item it decorates (see `ix_trace_rs::trace`'s own docs -- the marker must
// be the bare, unqualified `#[trace(...)]` form, imported with `use
// ix_trace_rs::trace;`, since the path-qualified form binds nothing).
#[cfg(doctest)]
use ix_trace_rs::trace;

#[cfg(doctest)]
#[trace("TC-199", "FR-077-AC-1", "FR-077-AC-2")]
struct FR077Doctests;
