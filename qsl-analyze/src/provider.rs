// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-280's in-process provider entry for QSL-owned engines.

use crate::analyze::{
    analyze, analyze_items, statistical_item, terminal_records, AnalyzeItem, AnalyzeLimits,
    AnalyzeRefusal, AnalyzeRequest, Engines, SubjectBinding,
};
use crate::statistical::StatisticalRequest;
use qsl_eval::model_subject::ModelSubject;
use qsl_eval::simulation::model::{ExpansionLimits, ModelSimulationRequest};
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
    let pending_items = build_analyze_items(package, binding, items, limits);
    let request = AnalyzeRequest {
        binding: binding.clone(),
        items: pending_items,
    };

    match analyze(package, request, engines, limits, cancel) {
        Ok(staged) => staged.into_value().records,
        Err(qsl_foundation::diagnostic::StageFailure::Refused(AnalyzeRefusal::Binding(_))) => {
            terminal_records(analyze_items(
                engines,
                build_analyze_items(package, binding, items, limits),
                cancel,
            ))
        }
        Err(_) => {
            return items
                .iter()
                .map(|item| {
                    TerminalRecord::failed_with(
                        item.item.request_index,
                        item.backend.clone(),
                        CatalogCode::new("runtime_invariant", "established-invariant-broken"),
                    )
                })
                .collect();
        }
    }
}

fn build_analyze_items<'a>(
    package: &'a CheckedPackage,
    binding: &SubjectBinding<'a>,
    items: &'a [RoutedItem<'a>],
    limits: &AnalyzeLimits,
) -> Vec<AnalyzeItem<'a>> {
    items
        .iter()
        .map(|routed| {
            let request = StatisticalRequest {
                subject: ModelSubject {
                    simulation: ModelSimulationRequest {
                        package,
                        packages: binding.packages,
                        normalization: limits.normalization,
                        snapshots: binding.snapshots,
                        initial: binding.initial.clone(),
                        universes: binding.universes.clone(),
                        observation: limits.observation,
                        expansion: ExpansionLimits {
                            max_candidates: u64::MAX,
                        },
                        meter: limits.accounting,
                    },
                    proof_bounds: Vec::new(),
                },
                claim: routed.item.claim,
                method: routed.item.method,
                seed: routed.item.seed,
                sampler: routed.item.sampler.clone(),
                limits: routed.item.limits,
                max_witnesses: routed.item.max_witnesses,
                symmetry: routed.item.symmetry.clone(),
            };
            statistical_item(routed.item.request_index, request).disposed(
                qsl_route::routing::Disposition::Supported(qsl_route::Candidate::new(
                    routed.backend.clone(),
                )),
                None,
            )
        })
        .collect()
}
