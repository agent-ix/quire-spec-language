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
use crate::model_check::{
    check_model, manifest as en1, HyperRefusal, LivenessRefusal, ModelCheckRefusal,
    ModelCheckRequest,
};
use crate::statistical::{check_statistical, manifest as en4, StatisticalRequest};
use qsl_eval::simulation::model::ClauseOutcome;

/// `runtime_invariant`/`established-invariant-broken`: the diagnostic of a
/// refusal whose own type names no code.
const BROKEN: CatalogCode = CatalogCode::new("runtime_invariant", "established-invariant-broken");
const NOT_YET: CatalogCode = CatalogCode::new("unsupported_construct", "not-yet-implemented");
const EXPRESSION_FORM: CatalogCode = CatalogCode::new("unsupported_construct", "expression-form");
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
    let declined = |cause, record: &qsl_semantics::model::observation::AdmissionRecord| {
        Refusal::Declined(
            cause,
            DeclineCode::Qsl(record.code),
            CatalogCode::new(record.code.as_str(), record.cause),
        )
    };
    match failure {
        AdmissionFailure::Refused(record) => declined(ProofRefusalCause::InvalidInput, record),
        AdmissionFailure::Incomplete(record) => {
            declined(ProofRefusalCause::IncompleteInput, record)
        }
        AdmissionFailure::Fault(fault) => Refusal::Failed(fault.catalog_code()),
    }
}

fn symmetry() -> Refusal {
    Refusal::Declined(
        ProofRefusalCause::InvalidInput,
        DeclineCode::Qsl(qsl_foundation::Code::InvalidRuntimeInput),
        INVALID_VALUE,
    )
}

/// The FR-281 "Engine refusals" row of an EN-1 refusal: `unsupported` only
/// for a refusal whose own FR-272 code is `unsupported_construct` or
/// `unsupported_projection`, `failed` for every other one.
pub(super) fn model_refusal(refusal: &ModelCheckRefusal) -> Refusal {
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
        | ModelCheckRefusal::Liveness(LivenessRefusal::ComponentCertificateNotBuilt { .. })
        | ModelCheckRefusal::Hyper(
            HyperRefusal::FormNotDecided { .. } | HyperRefusal::NotSingleExistential { .. },
        ) => Refusal::Unsupported(NOT_YET),
        // Clauses and operations the engine cannot read: unsupported forms.
        ModelCheckRefusal::Translation(_) | ModelCheckRefusal::Footprint(_) => {
            Refusal::Unsupported(EXPRESSION_FORM)
        }
        // A request defect: an operation or subject count the request names
        // wrongly.
        ModelCheckRefusal::Liveness(LivenessRefusal::UnknownOperation { .. })
        | ModelCheckRefusal::Hyper(
            HyperRefusal::UnknownOperation { .. } | HyperRefusal::SubjectCount { .. },
        ) => Refusal::Failed(INVALID_VALUE),
        // Engine defects, and refusals whose types name no catalog code
        // (a spec point for STD-444's follow-up): `failed` with the
        // broken-invariant diagnostic.
        ModelCheckRefusal::Liveness(LivenessRefusal::Automaton(_))
        | ModelCheckRefusal::Hyper(HyperRefusal::Automaton(_) | HyperRefusal::NoPartnerPath)
        | ModelCheckRefusal::Extent(_)
        | ModelCheckRefusal::Protocol(_)
        | ModelCheckRefusal::Encoding(_)
        | ModelCheckRefusal::Key(_)
        | ModelCheckRefusal::Item(_) => Refusal::Failed(BROKEN),
    }
}

/// An EN-4 refusal: FR-192's map settles most; the rest are the "Engine
/// refusals" rows.
pub(super) fn statistical_refusal(
    refusal: &qsl_replay::statistical::StatisticalRefusal,
) -> EngineReport {
    use qsl_replay::statistical::StatisticalRefusal as R;
    EngineReport::Refused(match refusal {
        R::Admission(failure) => admission(failure),
        R::Symmetry(_) => symmetry(),
        R::RequiresBound(_) => Refusal::RequiresBound,
        R::NotStatistical { .. } => Refusal::NotStatistical,
        // FR-192-AC-7 settles a refused predicate by its own code (and an
        // incomplete read `incomplete`/`limit-reached`), but FR-192's map
        // returns no record for it yet (QSL-570), so `analyze` inherits the
        // gap: a family refusal keeps its code, every other one is `failed`
        // with the broken-invariant diagnostic.
        R::PredicateRefused(ClauseOutcome::FamilyRefused(code)) => Refusal::Failed(*code),
        R::PredicateRefused(_) => Refusal::Failed(BROKEN),
        settled => match settle_statistical_refusal(settled) {
            Ok(record) => return EngineReport::Statistical(Box::new(record)),
            Err(_) => Refusal::Failed(BROKEN),
        },
    })
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use ix_trace_rs::trace;
    use qsl_foundation::Code;
    use qsl_replay::statistical::StatisticalRefusal;
    use qsl_semantics::model::observation::AdmissionRecord;

    use super::*;

    fn record(code: Code, cause: &'static str) -> AdmissionRecord {
        AdmissionRecord {
            code,
            cause,
            fields: BTreeMap::new(),
            path: Vec::new(),
            kernel: None,
        }
    }

    fn code(refusal: &Refusal) -> Option<String> {
        match refusal {
            Refusal::Declined(_, _, code) | Refusal::Unsupported(code) | Refusal::Failed(code) => {
                Some(code.to_string())
            }
            Refusal::RequiresBound | Refusal::NotStatistical => None,
        }
    }

    /// FR-281-AC-11: a refused admission is `declined` `invalid-input` and an
    /// incomplete one `incomplete-input`, each carrying the record's full
    /// code and cause, and an admission fault is `failed`.
    #[trace("TC-762", "FR-281-AC-11")]
    #[test]
    fn admission_declines_with_the_records_full_diagnostic() {
        let refused =
            AdmissionFailure::Refused(record(Code::InvalidRuntimeInput, "wrong-role-mapping"));
        let Refusal::Declined(cause, decline, diagnostic) = admission(&refused) else {
            panic!("expected declined");
        };
        assert_eq!(cause, ProofRefusalCause::InvalidInput);
        assert_eq!(decline, DeclineCode::Qsl(Code::InvalidRuntimeInput));
        assert_eq!(
            diagnostic.to_string(),
            "invalid_runtime_input/wrong-role-mapping"
        );

        let incomplete = AdmissionFailure::Incomplete(record(
            Code::UnavailableObservation,
            "missing-required-artifact",
        ));
        let Refusal::Declined(cause, _, diagnostic) = admission(&incomplete) else {
            panic!("expected declined");
        };
        assert_eq!(cause, ProofRefusalCause::IncompleteInput);
        assert_eq!(
            diagnostic.to_string(),
            "unavailable_observation/missing-required-artifact"
        );
    }

    /// FR-281-AC-11: a malformed symmetry declaration is `declined`,
    /// `invalid-input`, `invalid_runtime_input`/`invalid-value`.
    #[trace("TC-762", "FR-281-AC-11")]
    #[test]
    fn symmetry_declines_invalid_value() {
        let Refusal::Declined(cause, decline, diagnostic) = symmetry() else {
            panic!("expected declined");
        };
        assert_eq!(cause, ProofRefusalCause::InvalidInput);
        assert_eq!(decline, DeclineCode::Qsl(Code::InvalidRuntimeInput));
        assert_eq!(diagnostic, INVALID_VALUE);
    }

    /// FR-281's table: only a refusal whose own code is `unsupported_*` is
    /// `unsupported`; liveness and hyper request or engine defects are
    /// `failed`.
    #[trace("TC-762", "FR-281-AC-11")]
    #[test]
    fn model_refusals_are_unsupported_only_for_unsupported_codes() {
        let unsupported = |refusal: ModelCheckRefusal| match model_refusal(&refusal) {
            Refusal::Unsupported(code) => Some(code.to_string()),
            _ => None,
        };
        let failed = |refusal: ModelCheckRefusal| match model_refusal(&refusal) {
            Refusal::Failed(code) => Some(code.to_string()),
            _ => None,
        };
        let not_yet = Some("unsupported_construct/not-yet-implemented".to_owned());
        assert_eq!(unsupported(ModelCheckRefusal::GuardedActivation), not_yet);
        assert_eq!(unsupported(ModelCheckRefusal::ReducedLiveness), not_yet);
        assert_eq!(
            unsupported(ModelCheckRefusal::Liveness(
                LivenessRefusal::ComponentCertificateNotBuilt {
                    stats: qsl_replay::RunStats::default()
                }
            )),
            not_yet
        );
        assert_eq!(
            unsupported(ModelCheckRefusal::Hyper(HyperRefusal::FormNotDecided {
                form: None
            })),
            not_yet
        );
        assert_eq!(
            unsupported(ModelCheckRefusal::Hyper(
                HyperRefusal::NotSingleExistential { form: None }
            )),
            not_yet
        );
        assert_eq!(
            failed(ModelCheckRefusal::Liveness(
                LivenessRefusal::UnknownOperation {
                    name: "op".to_owned()
                }
            )),
            Some("invalid_runtime_input/invalid-value".to_owned())
        );
        assert_eq!(
            failed(ModelCheckRefusal::Hyper(HyperRefusal::SubjectCount {
                expected: 2,
                found: 1
            })),
            Some("invalid_runtime_input/invalid-value".to_owned())
        );
        assert_eq!(
            failed(ModelCheckRefusal::Hyper(HyperRefusal::UnknownOperation {
                name: "op".to_owned()
            })),
            Some("invalid_runtime_input/invalid-value".to_owned())
        );
        assert_eq!(
            failed(ModelCheckRefusal::Hyper(HyperRefusal::NoPartnerPath)),
            Some("runtime_invariant/established-invariant-broken".to_owned())
        );
        assert_eq!(
            failed(ModelCheckRefusal::UnknownConstraint {
                name: "c".to_owned()
            }),
            Some("invalid_runtime_input/invalid-value".to_owned())
        );
        assert!(matches!(
            model_refusal(&ModelCheckRefusal::Admission(Box::new(
                AdmissionFailure::Refused(record(Code::InvalidRuntimeInput, "wrong-role-mapping"))
            ))),
            Refusal::Declined(..)
        ));
    }

    /// FR-281's table for EN-4, and FR-192-AC-7's gap: a family-refused
    /// predicate keeps its code, every other predicate refusal is `failed`
    /// with the broken-invariant diagnostic; `NotStatistical` is a
    /// disposition; the rest settle through FR-192's map.
    #[trace("TC-762", "FR-281-AC-11")]
    #[test]
    fn statistical_refusals_follow_the_table() {
        let class = |refusal: StatisticalRefusal| match statistical_refusal(&refusal) {
            EngineReport::Refused(refusal) => code(&refusal).unwrap_or_else(|| match refusal {
                Refusal::NotStatistical => "not-statistical".to_owned(),
                _ => "requires-bound".to_owned(),
            }),
            _ => "settled".to_owned(),
        };
        let outside = CatalogCode::new("integer_out_of_domain", "outside-domain");
        assert_eq!(
            class(StatisticalRefusal::PredicateRefused(
                ClauseOutcome::FamilyRefused(outside)
            )),
            "integer_out_of_domain/outside-domain"
        );
        assert_eq!(
            class(StatisticalRefusal::PredicateRefused(
                ClauseOutcome::NotBoolean
            )),
            "runtime_invariant/established-invariant-broken"
        );
        assert_eq!(class(StatisticalRefusal::EveryScheduler), "settled");
        assert_eq!(
            class(StatisticalRefusal::Admission(Box::new(
                AdmissionFailure::Refused(record(Code::InvalidRuntimeInput, "wrong-role-mapping"))
            ))),
            "invalid_runtime_input/wrong-role-mapping"
        );
    }
}
