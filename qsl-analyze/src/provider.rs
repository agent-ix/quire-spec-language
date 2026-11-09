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
    /// The timed sampler settings (FR-254).
    pub timed_sampling: qsl_replay::statistical::TimedSampling,
    /// Whether the caller requires this item for complete aggregate success
    /// (FR-507-AC-5).
    pub required: bool,
}

/// One item routed by the caller to a QSL engine.
#[derive(Clone)]
pub struct RoutedItem<'a> {
    /// The typed claim and its request settings.
    pub item: ClaimItem<'a>,
    /// The backend selected by the caller's negotiation.
    pub backend: BackendId,
}

/// An EN-1 authored-clause item (FR-281's `temporal` member of `ClaimItem`),
/// identified by bytes: the clause's occurrence key in the request's package
/// and the source unit's bytes the package was checked from. `analyze` builds
/// the [`ClauseSubject`](crate::analyze::ClauseSubject) over its own package,
/// binding and limits, so the item can name no other subject.
#[derive(Clone)]
pub struct ClauseClaimItem<'a> {
    /// The item's request index.
    pub request_index: RequestIndex,
    /// The clause's node and `claim` occurrence.
    pub clause: qsl_foundation::source::provenance::OccurrenceKey,
    /// The source bytes the package was checked from (replay recompiles them).
    pub unit: &'a [u8],
    /// The run's budgets.
    pub run: crate::analyze::ClauseLimits,
    /// Whether the caller requires this item for complete aggregate success
    /// (FR-507-AC-5).
    pub required: bool,
}

/// One authored-clause item routed by the caller: the EN-1 counterpart of
/// [`RoutedItem`].
#[derive(Clone)]
pub struct RoutedClause<'a> {
    /// The clause and its run settings.
    pub item: ClauseClaimItem<'a>,
    /// The backend selected by the caller's negotiation. Only EN-1 runs an
    /// authored clause; any other backend settles the item `unsupported`.
    pub backend: BackendId,
}

/// A request item of either kind `analyze` runs. `RoutedItem` (EN-4) converts
/// into it, so a request of EN-4 items is built exactly as before.
#[derive(Clone)]
pub enum RoutedWork<'a> {
    /// An EN-4 probabilistic claim.
    Claim(RoutedItem<'a>),
    /// An EN-1 authored clause.
    Clause(RoutedClause<'a>),
}

impl<'a> From<RoutedItem<'a>> for RoutedWork<'a> {
    fn from(item: RoutedItem<'a>) -> Self {
        Self::Claim(item)
    }
}

impl<'a> From<RoutedClause<'a>> for RoutedWork<'a> {
    fn from(item: RoutedClause<'a>) -> Self {
        Self::Clause(item)
    }
}

impl RoutedWork<'_> {
    fn request_index(&self) -> RequestIndex {
        match self {
            Self::Claim(routed) => routed.item.request_index,
            Self::Clause(routed) => routed.item.request_index,
        }
    }

    fn backend(&self) -> &BackendId {
        match self {
            Self::Claim(routed) => &routed.backend,
            Self::Clause(routed) => &routed.backend,
        }
    }
}

/// Run routed items (EN-4 [`RoutedItem`]s, or [`RoutedWork`] with EN-1
/// authored clauses) over the checked package and subject binding, using
/// the same analyze operation that serves direct in-process requests.
///
/// The returned records follow the input order and preserve each item's
/// request index. The caller supplies items already negotiated to a QSL
/// backend.
pub fn run<'a, W>(
    package: &CheckedPackage,
    binding: &SubjectBinding<'a>,
    items: &[W],
    engines: &Engines,
    limits: &AnalyzeLimits,
    cancel: &Cancel,
) -> Vec<TerminalRecord>
where
    W: Clone + Into<RoutedWork<'a>>,
{
    let items: Vec<RoutedWork<'a>> = items.iter().cloned().map(Into::into).collect();
    let request = AnalyzeRequest {
        binding: binding.clone(),
        items: items.clone(),
    };

    match analyze(package, request, engines, limits, cancel) {
        Ok(staged) => staged.into_value().records,
        Err(qsl_foundation::diagnostic::StageFailure::Refused(AnalyzeRefusal::Binding(
            failure,
        ))) => admission_records(&items, failure),
        Err(_) => items
            .iter()
            .map(|item| {
                TerminalRecord::failed_with(
                    item.request_index(),
                    item.backend().clone(),
                    CatalogCode::new("runtime_invariant", "established-invariant-broken"),
                )
            })
            .collect(),
    }
}

fn admission_records(
    items: &[RoutedWork<'_>],
    failure: qsl_semantics::model::observation::AdmissionFailure,
) -> Vec<TerminalRecord> {
    use qsl_semantics::model::observation::AdmissionFailure;

    items
        .iter()
        .map(|item| match &failure {
            AdmissionFailure::Refused(record) => TerminalRecord::disposition_declined(
                item.request_index(),
                qsl_replay::ProofRefusalCause::InvalidInput,
                qsl_replay::DeclineCode::Qsl(
                    qsl_foundation::diagnostic::Code::from_code(record.catalog.code())
                        .unwrap_or(qsl_foundation::diagnostic::Code::RuntimeInvariant),
                ),
                record.catalog,
            ),
            AdmissionFailure::Incomplete(record) => TerminalRecord::disposition_declined(
                item.request_index(),
                qsl_replay::ProofRefusalCause::IncompleteInput,
                qsl_replay::DeclineCode::Qsl(
                    qsl_foundation::diagnostic::Code::from_code(record.catalog.code())
                        .unwrap_or(qsl_foundation::diagnostic::Code::RuntimeInvariant),
                ),
                record.catalog,
            ),
            AdmissionFailure::Fault(fault) => {
                TerminalRecord::disposition_failed(item.request_index(), fault.catalog_code())
            }
        })
        .collect()
}
