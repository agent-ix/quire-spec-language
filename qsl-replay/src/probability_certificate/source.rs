// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-282 source-backed probability certificate checking without an analyzer.

use super::document::{check_probability_certificate_document, ProbabilityDocumentError, ProbabilityDocumentLimits};
use super::{ExactProbabilityCertificateCheck, ObligationIdentity};
use crate::spine::{self, DependencyInput, FrontEndFailure, ParseRequest, SourceComponent, SpineLimits};
use crate::{ByteProvision, DefinitionSourceRef};
use qsl_eval::simulation::model::{ExpansionLimits, ModelSimulationRequest, ModelSystem, PopulationUniverse};
use qsl_foundation::source::{Source, SourceReadRefusal};
use qsl_foundation::SourceIdentity;
use qsl_semantics::model::intake::package_input;
use qsl_semantics::model::observation::{AdmissionFailure, DocumentRef, ObservationLimits};
use quire_exact::{Cancel, ScalarLimits};

/// All source and subject data needed to recheck one native-source certificate.
/// Imported library configurations are outside this initial source facade.
pub struct ProbabilitySourceRequest<'a> {
    /// Authored source identity, looked up only through the byte provision.
    pub source: &'a DefinitionSourceRef,
    /// Exact expected source component, including admitted UTF-8 content.
    pub expected_source: &'a SourceComponent,
    /// Native package name supplied to the checking stage.
    pub package_name: &'a str,
    /// Complete source, domain-package and snapshot provision.
    pub provision: &'a ByteProvision,
    /// Requested structural claim identity, independent of the certificate.
    pub item: &'a ObligationIdentity,
    /// Initial document identities; exactly one is admitted by the profile.
    pub initial: Vec<DocumentRef>,
    /// Finite universes keyed by original declaration identities.
    pub universes: Vec<PopulationUniverse>,
    /// Exact canonical certificate bytes.
    pub certificate: &'a [u8],
}

/// Caller-owned stage, admission and certificate ceilings.
#[derive(Clone, Copy)]
pub struct ProbabilitySourceLimits {
    /// Compiler and package-emission ceilings.
    pub spine: SpineLimits,
    /// Snapshot admission ceilings.
    pub observation: ObservationLimits,
    /// Per-state expansion ceiling.
    pub expansion: ExpansionLimits,
    /// Per-evaluation scalar ceilings.
    pub scalar: ScalarLimits,
    /// Document and independent graph ceilings.
    pub document: ProbabilityDocumentLimits,
}

/// Source checking never fabricates a truth value on refusal or stop.
#[derive(Debug, thiserror::Error)]
pub enum ProbabilitySourceError {
    /// A referenced source is absent or differs from the expected component.
    #[error("conflicting_identity: probability certificate source")]
    SourceMismatch,
    /// Source bytes failed the shared source admission rules.
    #[error("probability certificate source refused: {0:?}")]
    Source(Box<SourceReadRefusal>),
    /// A compiler stage refused, stopped or was cancelled.
    #[error("probability certificate compilation stopped: {0:?}")]
    Compile(FrontEndFailure),
    /// Domain package intake failed.
    #[error("probability certificate domain intake failed")]
    Packages,
    /// State/model admission failed with its original typed cause.
    #[error("probability certificate subject refused: {0:?}")]
    Subject(AdmissionFailure),
    /// Certificate admission or independent checking stopped.
    #[error(transparent)]
    Document(#[from] ProbabilityDocumentError),
}

/// Recompile exact admitted source through S1–S3, construct the production
/// model system and check the document. No emitted package, solver product,
/// ambient I/O or analyzer implementation enters this path.
pub fn check_probability_certificate_source(
    request: ProbabilitySourceRequest<'_>, limits: ProbabilitySourceLimits, cancel: &Cancel,
) -> Result<ExactProbabilityCertificateCheck, ProbabilitySourceError> {
    // Refuse a foreign claim before compiler/model work as well as enumeration.
    let document = super::read_probability_certificate(request.certificate, limits.document, cancel)?;
    if &document.identity != request.item {
        return Err(ProbabilityDocumentError::IdentityMismatch.into());
    }
    let bytes = request.provision.get(request.source).ok_or(ProbabilitySourceError::SourceMismatch)?;
    let identity = SourceIdentity::new(request.source.authority(), request.source.identity());
    let source = Source::read_typed(identity.clone(), request.source.identity(), bytes, limits.spine.source.source_bytes)
        .map_err(ProbabilitySourceError::Source)?;
    let actual = SourceComponent { authority: identity.authority.clone(), identity: identity.identity.clone(), content: source.text().to_owned() };
    if actual != *request.expected_source { return Err(ProbabilitySourceError::SourceMismatch); }
    let packages = package_input(request.provision.package_documents()).map_err(|_| ProbabilitySourceError::Packages)?;
    let parsed = spine::parse(&ParseRequest { source: &identity, path: request.source.identity(), bytes }, limits.spine.source, cancel)
        .map_err(ProbabilitySourceError::Compile)?.into_value();
    let models = spine::select(&parsed, &packages, limits.spine.model, cancel)
        .map_err(ProbabilitySourceError::Compile)?.into_value();
    let checked = spine::check(&parsed, request.package_name.to_owned(), &models, &DependencyInput::default(), limits.spine, cancel)
        .map_err(ProbabilitySourceError::Compile)?.into_value();
    let claims = checked.package().graph().probabilistic_model().claims();
    let claim = super::bind_claim(claims, request.item, cancel).map_err(ProbabilityDocumentError::from)?;
    if request.item != &ObligationIdentity::new(claim.occurrence().ok_or(ProbabilityDocumentError::IdentityMismatch)?.node().clone(), None, 0) {
        return Err(ProbabilityDocumentError::IdentityMismatch.into());
    }
    let snapshots = request.provision.snapshots();
    let simulation = ModelSimulationRequest {
        package: checked.package(), packages: &packages, normalization: limits.spine.model,
        snapshots: &snapshots, initial: request.initial, universes: request.universes,
        observation: limits.observation, expansion: limits.expansion, meter: limits.scalar,
        emit: limits.spine.package.emit(),
    };
    let system = ModelSystem::over_model(simulation, &claim.subject.package.identity).map_err(ProbabilitySourceError::Subject)?;
    check_probability_certificate_document(&system, claims, request.item, request.certificate, limits.document, cancel).map_err(Into::into)
}
