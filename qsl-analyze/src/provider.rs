// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-280's in-process provider entry for QSL-owned engines.

use crate::analyze::{
    analyze, AnalyzeLimits, AnalyzeRefusal, AnalyzeRequest, Engines, SubjectBinding,
};
use qsl_foundation::{BackendId, RequestIndex};
use qsl_package::CheckedPackage;
use qsl_replay::statistical::StatisticalLimits;
use qsl_replay::TerminalRecord;
use qsl_semantics::check::CheckedProbabilisticClaim;
use quire_exact::Cancel;
use quire_semantic_value::catalog::CatalogCode;

/// An EN-4 claim and its sampling settings, without the subject supplied by
/// the provider request.
#[derive(Clone)]
pub struct ClaimItem<'a> {
    /// The item's request index.
    pub request_index: RequestIndex,
    /// The checked probabilistic claim.
    pub claim: &'a CheckedProbabilisticClaim,
    /// The statistical method selected by the request.
    pub method: qsl_replay::statistical::StatisticalMethod,
    /// The deterministic sample seed.
    pub seed: u64,
    /// The sampler definition selected by the request.
    pub sampler: qsl_foundation::selection::DefinitionRef,
    /// The sample and accounting budgets.
    pub limits: StatisticalLimits,
    /// The maximum number of witnesses retained.
    pub max_witnesses: u32,
    /// The declared symmetries selected for the request.
    pub symmetry: Vec<qsl_route::reductions::SymmetryDeclaration>,
}

/// One item routed by the caller to a QSL engine.
#[derive(Clone)]
pub struct RoutedItem<'a> {
    /// The typed claim and its request settings.
    pub item: ClaimItem<'a>,
    /// The backend selected by the caller's negotiation.
    pub backend: BackendId,
}

/// Run routed EN-4 items over the checked package and subject binding, using
/// the same analyze operation that serves direct in-process requests.
///
/// The returned records follow the input order and preserve each item's
/// request index. The caller supplies items already negotiated to a QSL
/// backend.
pub fn run(
    package: &CheckedPackage,
    binding: &SubjectBinding<'_>,
    items: &[RoutedItem<'_>],
    engines: &Engines,
    limits: &AnalyzeLimits,
    cancel: &Cancel,
) -> Vec<TerminalRecord> {
    let request = AnalyzeRequest {
        binding: binding.clone(),
        items: items.to_vec(),
    };

    match analyze(package, request, engines, limits, cancel) {
        Ok(staged) => staged.into_value().records,
        Err(qsl_foundation::diagnostic::StageFailure::Refused(AnalyzeRefusal::Binding(
            failure,
        ))) => admission_records(items, failure),
        Err(_) => items
            .iter()
            .map(|item| {
                TerminalRecord::failed_with(
                    item.item.request_index,
                    item.backend.clone(),
                    CatalogCode::new("runtime_invariant", "established-invariant-broken"),
                )
            })
            .collect(),
    }
}

fn admission_records(
    items: &[RoutedItem<'_>],
    failure: qsl_semantics::model::observation::AdmissionFailure,
) -> Vec<TerminalRecord> {
    use qsl_semantics::model::observation::AdmissionFailure;

    items
        .iter()
        .map(|item| match &failure {
            AdmissionFailure::Refused(record) => TerminalRecord::disposition_declined(
                item.item.request_index,
                qsl_replay::ProofRefusalCause::InvalidInput,
                qsl_replay::DeclineCode::Qsl(
                    qsl_foundation::diagnostic::Code::from_code(record.catalog.code())
                        .unwrap_or(qsl_foundation::diagnostic::Code::RuntimeInvariant),
                ),
                record.catalog,
            ),
            AdmissionFailure::Incomplete(record) => TerminalRecord::disposition_declined(
                item.item.request_index,
                qsl_replay::ProofRefusalCause::IncompleteInput,
                qsl_replay::DeclineCode::Qsl(
                    qsl_foundation::diagnostic::Code::from_code(record.catalog.code())
                        .unwrap_or(qsl_foundation::diagnostic::Code::RuntimeInvariant),
                ),
                record.catalog,
            ),
            AdmissionFailure::Fault(fault) => {
                TerminalRecord::disposition_failed(item.item.request_index, fault.catalog_code())
            }
        })
        .collect()
}
