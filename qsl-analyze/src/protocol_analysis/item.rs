// SPDX-License-Identifier: AGPL-3.0-or-later
//! The `analyze` item adapters of the protocol analysis engine and its
//! settlement into FR-331 terminal records (ADR-036 PA-4, PA-20, FR-573).
//! [`projection_item`] and [`realizability_item`] are the only constructors
//! of the engine's work, so no caller can hand `analyze` a pre-built
//! result.

use qsl_foundation::{BackendId, Code, RequestIndex};
use qsl_replay::protocol_analysis::ProtocolAnalysisSettlement;
use qsl_replay::{DeclineCode, ProofRefusalCause, TerminalRecord};
use qsl_route::Mode;
use qsl_semantics::check::{Capability, DeclarationFamily};
use quire_contract_model::CheckedPackageV2;
use quire_semantic_value::catalog::CatalogCode;

use super::manifest::BACKEND_ID;
use super::{
    project, realize, ProjectionOutcome, ProjectionRefusal, ProjectionRequest,
    RealizabilityOutcome, RealizabilityRefusal, RealizabilityRequest,
};
use crate::analyze::{AnalyzeItem, EngineReport, Refusal};

/// The result an analysis returned: its structural key, carried on the
/// result, and its outcome with the witness or the counterexample and its
/// FR-351 record (FR-572, FR-574).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProtocolAnalysisArtifact {
    /// A projection's result.
    Projection(Box<ProjectionOutcome>),
    /// A realizability result.
    Realizability(Box<RealizabilityOutcome>),
}

impl ProtocolAnalysisArtifact {
    /// The result's FR-573 settlement.
    #[must_use]
    pub fn settlement(&self) -> ProtocolAnalysisSettlement {
        match self {
            Self::Projection(result) => ProtocolAnalysisSettlement::of(result),
            Self::Realizability(result) => ProtocolAnalysisSettlement::of(result),
        }
    }
}

fn invalid(cause: &'static str) -> EngineReport {
    EngineReport::Refused(Refusal::Declined(
        ProofRefusalCause::InvalidInput,
        DeclineCode::Qsl(Code::InvalidRuntimeInput),
        CatalogCode::new("invalid_runtime_input", cause),
    ))
}

fn fault() -> EngineReport {
    EngineReport::Refused(Refusal::Failed(CatalogCode::new(
        "runtime_invariant",
        "established-invariant-broken",
    )))
}

fn found(artifact: ProtocolAnalysisArtifact) -> EngineReport {
    EngineReport::ProtocolAnalysis(Box::new(artifact))
}

/// The `local-projection` item at `index` that projects `package`'s protocol
/// as `request` asks (FR-570). A request the engine refuses for an absent
/// key member, an unknown profile or role is `declined` `invalid-input`; a
/// read the package refuses is `failed`.
pub fn projection_item<'a>(
    index: RequestIndex,
    package: &'a CheckedPackageV2,
    request: ProjectionRequest,
) -> AnalyzeItem<'a> {
    AnalyzeItem::new(
        index,
        Capability::LocalProjection,
        Some(DeclarationFamily::Protocol),
        Mode::Unbounded,
        BACKEND_ID,
        move |_| match project(package, request) {
            Ok(result) => found(ProtocolAnalysisArtifact::Projection(Box::new(result))),
            Err(ProjectionRefusal::Missing(_)) => invalid("missing-member"),
            Err(ProjectionRefusal::InvalidProfile(_) | ProjectionRefusal::UnknownRole(_)) => {
                invalid("invalid-value")
            }
            Err(
                ProjectionRefusal::View(_)
                | ProjectionRefusal::Premises(_)
                | ProjectionRefusal::Inconsistent(_)
                | ProjectionRefusal::Structure(_),
            ) => fault(),
        },
    )
}

/// The `realizability` item at `index` that decides `package`'s protocol as
/// `request` asks (FR-571). Refusals settle as for [`projection_item`]; a
/// run limit or the evaluation meter that stops the arena is the result's
/// own `failed` outcome, naming the limit and its value.
pub fn realizability_item<'a>(
    index: RequestIndex,
    package: &'a CheckedPackageV2,
    request: RealizabilityRequest<'a>,
) -> AnalyzeItem<'a> {
    AnalyzeItem::new(
        index,
        Capability::Realizability,
        Some(DeclarationFamily::Protocol),
        Mode::Unbounded,
        BACKEND_ID,
        move |_| match realize(package, request) {
            Ok(result) => found(ProtocolAnalysisArtifact::Realizability(Box::new(result))),
            Err(RealizabilityRefusal::Missing(_)) => invalid("missing-member"),
            Err(RealizabilityRefusal::InvalidProfile(_) | RealizabilityRefusal::PremiseFact(_)) => {
                invalid("invalid-value")
            }
            Err(
                RealizabilityRefusal::View(_)
                | RealizabilityRefusal::Premises(_)
                | RealizabilityRefusal::Admission(_)
                | RealizabilityRefusal::Protocol(_)
                | RealizabilityRefusal::UnnamedNode(_)
                | RealizabilityRefusal::Stopped(_),
            ) => fault(),
        },
    )
}

/// Settle the engine's result as the item's FR-331 terminal record by the
/// FR-573 map; the record carries the result.
pub(crate) fn settle(
    artifact: ProtocolAnalysisArtifact,
    index: RequestIndex,
    backend: BackendId,
) -> (TerminalRecord, ProtocolAnalysisArtifact) {
    (
        TerminalRecord::of_protocol_analysis(index, backend, &artifact.settlement()),
        artifact,
    )
}
