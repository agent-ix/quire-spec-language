// SPDX-License-Identifier: AGPL-3.0-or-later
//! The item adapters of the engines `analyze` runs: each builds the
//! [`AnalyzeItem`] whose work calls the engine and hands its result to the
//! engine's layer-6 settlement map.

use qsl_foundation::digest::DigestRecord;
use qsl_foundation::RequestIndex;
use qsl_replay::{
    CertificateRequest, ModelTraceReplayResult, ReplayRefusal, TemporalCounterexample,
};
use qsl_route::{EvidenceKind, ItemEvidence};
use qsl_semantics::check::Capability;

use super::{AnalyzeItem, EngineReport, Unsettled};
use crate::model_check::{check_model, ModelCheckRequest};
use crate::statistical::provider::serve_request;
use crate::statistical::StatisticalRequest;

/// An EN-1 item (FR-281 Inputs): the run, the certificate request that
/// re-admits its subject for the closure checker, the `package_id` the run
/// checked, and the replay of a counterexample (FR-128), which in production
/// is `replay_model_trace` over the same source.
pub struct ModelItem<'a> {
    /// The EN-1 run.
    pub request: ModelCheckRequest<'a>,
    /// The closure checker's request over the same subject and item.
    pub certificate: CertificateRequest<'a>,
    /// The `package_id` of the package the run checked.
    pub package_id: DigestRecord,
    /// Replays the run's counterexample.
    pub replay: Box<CounterexampleReplayFn<'a>>,
}

/// The replay of an EN-1 counterexample (FR-128).
pub type CounterexampleReplayFn<'a> =
    dyn FnOnce(&TemporalCounterexample) -> Result<ModelTraceReplayResult, ReplayRefusal> + 'a;

/// The `temporal-satisfaction` item that runs `item` on EN-1 and settles it
/// by `settle_model_check` (FR-127).
pub fn model_item<'a>(index: RequestIndex, item: ModelItem<'a>) -> AnalyzeItem<'a> {
    AnalyzeItem::new(
        index,
        Capability::TemporalSatisfaction,
        ItemEvidence::Absent,
        move |cancel| {
            let ModelItem {
                request,
                certificate,
                package_id,
                replay,
            } = item;
            let limits = request.limits;
            match check_model(request, cancel) {
                Ok(outcome) => EngineReport::Model(Box::new(qsl_replay::settle_model_check(
                    &outcome,
                    &limits,
                    &package_id,
                    &certificate,
                    cancel,
                    replay,
                ))),
                Err(refusal) => EngineReport::Refused(Unsettled::Model(Box::new(refusal))),
            }
        },
    )
}

/// The `probabilistic-satisfaction` item, evidence `statistical`, that runs
/// `request` on EN-4 and settles it through
/// [`serve_request`](crate::statistical::provider::serve_request), the call
/// the FR-280 provider entry makes.
pub fn statistical_item<'a>(
    index: RequestIndex,
    request: StatisticalRequest<'a>,
) -> AnalyzeItem<'a> {
    AnalyzeItem::new(
        index,
        Capability::ProbabilisticSatisfaction,
        ItemEvidence::Kind(EvidenceKind::Statistical),
        move |cancel| match serve_request(request, cancel) {
            Ok(record) => EngineReport::Statistical(Box::new(record)),
            Err(refusal) => EngineReport::Refused(Unsettled::Statistical(refusal)),
        },
    )
}
