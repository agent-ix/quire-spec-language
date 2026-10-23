// SPDX-License-Identifier: AGPL-3.0-or-later
//! The item adapters of the engines `analyze` runs: each builds the
//! [`AnalyzeItem`] whose work calls the engine and hands its result to the
//! engine's layer-6 settlement map, or classifies its refusal by FR-281's
//! "Engine refusals" table.

use qsl_foundation::digest::DigestRecord;
use qsl_foundation::RequestIndex;
use qsl_replay::statistical::{settle_statistical, settle_statistical_refusal, SubjectLimits};
use qsl_replay::{
    CertificateRequest, DeclineCode, ModelTraceReplayResult, ProofRefusalCause, ReplayRefusal,
    TemporalCounterexample,
};
use qsl_route::Mode;
use qsl_semantics::check::Capability;
use qsl_semantics::model::observation::AdmissionFailure;
use quire_semantic_value::catalog::CatalogCode;

use super::{AnalyzeItem, EngineReport, Refusal};
use crate::model_check::{check_model, manifest as en1, ModelCheckRefusal, ModelCheckRequest};
use crate::statistical::{check_statistical, manifest as en4, StatisticalRequest};

/// `runtime_invariant`/`established-invariant-broken`: the diagnostic of a
/// refusal whose own type names no code.
const BROKEN: CatalogCode = CatalogCode::new("runtime_invariant", "established-invariant-broken");
const NOT_YET: CatalogCode = CatalogCode::new("unsupported_construct", "not-yet-implemented");
const INVALID_VALUE: CatalogCode = CatalogCode::new("invalid_runtime_input", "invalid-value");

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
/// by `settle_model_check` (FR-127). It is `Supported(EN-1)` until the
/// caller's negotiation says otherwise ([`AnalyzeItem::disposed`]).
pub fn model_item<'a>(index: RequestIndex, item: ModelItem<'a>) -> AnalyzeItem<'a> {
    AnalyzeItem::new(
        index,
        Capability::TemporalSatisfaction,
        Mode::Bounded,
        en1::BACKEND_ID,
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
                Err(refusal) => EngineReport::Refused(model_refusal(&refusal)),
            }
        },
    )
}

/// The `probabilistic-satisfaction` item that runs `request` on EN-4 and
/// settles it by `check_statistical` and FR-192's map, the two calls the
/// FR-280 provider entry makes.
pub fn statistical_item<'a>(
    index: RequestIndex,
    request: StatisticalRequest<'a>,
) -> AnalyzeItem<'a> {
    AnalyzeItem::new(
        index,
        Capability::ProbabilisticSatisfaction,
        Mode::Bounded,
        en4::BACKEND_ID,
        move |cancel| {
            let simulation = &request.subject.simulation;
            let expansion = simulation.expansion;
            let meter = simulation.meter;
            match check_statistical(request, cancel) {
                Ok(outcome) => EngineReport::Statistical(Box::new(settle_statistical(
                    &outcome,
                    SubjectLimits {
                        expansion,
                        meter: &meter,
                    },
                ))),
                Err(refusal) => statistical_refusal(&refusal),
            }
        },
    )
}

fn admission(failure: &AdmissionFailure) -> Refusal {
    match failure {
        AdmissionFailure::Refused(record) => Refusal::Declined(
            ProofRefusalCause::InvalidInput,
            DeclineCode::Qsl(record.code),
        ),
        AdmissionFailure::Incomplete(record) => Refusal::Declined(
            ProofRefusalCause::IncompleteInput,
            DeclineCode::Qsl(record.code),
        ),
        AdmissionFailure::Fault(fault) => Refusal::Failed(fault.catalog_code()),
    }
}

fn symmetry() -> Refusal {
    Refusal::Declined(
        ProofRefusalCause::InvalidInput,
        DeclineCode::Qsl(qsl_foundation::Code::InvalidRuntimeInput),
    )
}

/// The FR-281 "Engine refusals" row of an EN-1 refusal.
fn model_refusal(refusal: &ModelCheckRefusal) -> Refusal {
    match refusal {
        ModelCheckRefusal::Admission(failure) => admission(failure),
        ModelCheckRefusal::RequiresBound(_) => Refusal::RequiresBound,
        ModelCheckRefusal::Symmetry(_) => symmetry(),
        ModelCheckRefusal::UnknownConstraint { .. } => Refusal::Failed(INVALID_VALUE),
        ModelCheckRefusal::Fault(fault) => Refusal::Failed(fault.catalog_code()),
        // The forms and subjects EN-1 does not decide yet.
        ModelCheckRefusal::ProtocolRefinement
        | ModelCheckRefusal::FormNotDecided { .. }
        | ModelCheckRefusal::GuardedActivation
        | ModelCheckRefusal::WeakLiveness
        | ModelCheckRefusal::InstanceLiveness
        | ModelCheckRefusal::ReducedLiveness
        | ModelCheckRefusal::ProtocolAbstractSide
        | ModelCheckRefusal::Liveness(_)
        | ModelCheckRefusal::Hyper(_) => Refusal::Unsupported(NOT_YET),
        // Refusals whose types name no catalog code: a failed record with
        // the broken-invariant diagnostic.
        ModelCheckRefusal::Extent(_)
        | ModelCheckRefusal::Protocol(_)
        | ModelCheckRefusal::Translation(_)
        | ModelCheckRefusal::Encoding(_)
        | ModelCheckRefusal::Key(_)
        | ModelCheckRefusal::Item(_)
        | ModelCheckRefusal::Footprint(_) => Refusal::Failed(BROKEN),
    }
}

/// An EN-4 refusal: FR-192's map settles most; the rest are the "Engine
/// refusals" rows.
fn statistical_refusal(refusal: &qsl_replay::statistical::StatisticalRefusal) -> EngineReport {
    use qsl_replay::statistical::StatisticalRefusal as R;
    EngineReport::Refused(match refusal {
        R::Admission(failure) => admission(failure),
        R::Symmetry(_) => symmetry(),
        R::RequiresBound(_) => Refusal::RequiresBound,
        R::NotStatistical { .. } => Refusal::NotStatistical,
        R::PredicateRefused(_) => Refusal::Failed(BROKEN),
        settled => match settle_statistical_refusal(settled) {
            Ok(record) => return EngineReport::Statistical(Box::new(record)),
            Err(_) => Refusal::Failed(BROKEN),
        },
    })
}
