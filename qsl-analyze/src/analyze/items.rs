// SPDX-License-Identifier: AGPL-3.0-or-later
//! The item adapters of the engines `analyze` runs: each builds the
//! [`AnalyzeItem`] whose work calls the engine and hands its result to the
//! engine's layer-6 settlement map, or classifies its refusal by FR-281's
//! "Engine refusals" table.

use qsl_eval::simulation::model::ClauseOutcome;
use qsl_foundation::digest::DigestRecord;
use qsl_foundation::RequestIndex;
use qsl_replay::statistical::{settle_statistical, settle_statistical_refusal, SubjectLimits};
use qsl_replay::{
    settle_hyper_form, CertificateRequest, DeclineCode, ModelTraceReplayResult, ProofRefusalCause,
    ReplayRefusal, TemporalCounterexample, TerminalValue, UnavailabilityCause,
};
use qsl_route::request::RequirementItem;
use qsl_route::Mode;
use qsl_semantics::check::{Capability, HyperForm};
use qsl_semantics::family::UnsupportedItem;
use qsl_semantics::model::observation::AdmissionFailure;
use quire_semantic_value::catalog::CatalogCode;

use super::{AnalyzeItem, EngineReport, Refusal};
use crate::model_check::{
    check_model, manifest as en1, HyperRefusal, LivenessRefusal, ModelCheckRefusal,
    ModelCheckRequest,
};
use crate::statistical::{check_statistical, manifest as en4, StatisticalRequest};

/// The FR-396/FR-399 negotiation seam for a routed hyper item. HP-4 calls
/// [`settle_hyper_form`] before an engine adapter is built, then becomes the
/// existing unsupported-requested-capability disposition. The route writer
/// supplies the matching typed cause and an empty candidate set; admitted
/// forms and mismatched request items return `None` for their ordinary path.
pub fn hyper_form_item(item: &RequirementItem, form: &HyperForm) -> Option<AnalyzeItem<'static>> {
    if item.kind() != Capability::TemporalSatisfaction {
        return None;
    }
    let UnsupportedItem::HyperForm { reason } = item.unsupported()? else {
        return None;
    };
    let Some(TerminalValue::Unsupported(UnavailabilityCause::UnsupportedHyperForm {
        reason: settled_reason,
    })) = settle_hyper_form(form)
    else {
        return None;
    };
    (*reason == settled_reason).then(|| AnalyzeItem::unadapted(item.index(), item.kind()))
}

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
        // A refused `holds` evaluation settles as FR-203 and FR-192 settle
        // the same outcome: `failed` with the refusal's own code and cause,
        // and a broken invariant for a refusal that names none or an outcome
        // that is no refusal (QSL-1039).
        ModelCheckRefusal::AtomRefused(ClauseOutcome::KernelRefused(refusal)) => {
            match (refusal.code(), refusal.cause()) {
                (Some(code), Some(cause)) => Refusal::Failed(CatalogCode::new(code, cause)),
                _ => Refusal::Failed(BROKEN),
            }
        }
        ModelCheckRefusal::AtomRefused(ClauseOutcome::FamilyRefused(code)) => {
            Refusal::Failed(*code)
        }
        ModelCheckRefusal::AtomRefused(_) => Refusal::Failed(BROKEN),
        // An unobservable state settles by the stop's own cause, which is
        // the broken invariant for an internal failure.
        ModelCheckRefusal::StateUnobservable(stop) => Refusal::Failed(stop.cause),
        // The forms and subjects EN-1 does not decide yet.
        ModelCheckRefusal::ProtocolRefinement
        | ModelCheckRefusal::FormNotDecided { .. }
        | ModelCheckRefusal::CaptureRead { .. }
        | ModelCheckRefusal::WeakLiveness
        | ModelCheckRefusal::InstanceLiveness
        | ModelCheckRefusal::ReducedLiveness
        | ModelCheckRefusal::ProtocolAbstractSide
        | ModelCheckRefusal::Hyper(
            HyperRefusal::FormNotDecided { .. } | HyperRefusal::NotSingleExistential { .. },
        ) => Refusal::Unsupported(NOT_YET),
        // Clauses and operations the engine cannot read: unsupported forms.
        ModelCheckRefusal::Translation(_) | ModelCheckRefusal::Footprint(_) => {
            Refusal::Unsupported(EXPRESSION_FORM)
        }
        // A request defect: an operation or subject count the request names
        // wrongly, or the race-freedom item over a model subject.
        ModelCheckRefusal::RaceOverModel
        | ModelCheckRefusal::Liveness(LivenessRefusal::UnknownOperation { .. })
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
        // FR-192's map settles the rest, a refused predicate by FR-192-AC-7's
        // rows (its own code, or `incomplete`/`limit-reached`).
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
    use qsl_replay::{IncompleteCause, TerminalValue};
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
    #[trace("TC-763", "FR-281-AC-11")]
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
    #[trace("TC-763", "FR-281-AC-11")]
    #[test]
    fn symmetry_declines_invalid_value() {
        let Refusal::Declined(cause, decline, diagnostic) = symmetry() else {
            panic!("expected declined");
        };
        assert_eq!(cause, ProofRefusalCause::InvalidInput);
        assert_eq!(decline, DeclineCode::Qsl(Code::InvalidRuntimeInput));
        assert_eq!(diagnostic, INVALID_VALUE);
    }

    /// QSL-1039: a refused `holds` atom settles `failed` with the refusal's
    /// own code and cause as FR-203 and FR-192 settle the same outcome, a
    /// refusal naming none and a non-refusal settle the broken invariant,
    /// and an unobservable state settles by its stop's own cause.
    #[trace("TC-763")]
    #[test]
    fn a_refused_holds_atom_and_an_unobservable_state_settle_failed_with_their_own_code() {
        use quire_exact::{BoundViolation, CardinalityBound, CollectionKind, Refusal as Kernel};

        let failed = |refusal: ModelCheckRefusal| match model_refusal(&refusal) {
            Refusal::Failed(code) => code,
            _ => panic!("expected failed"),
        };
        let above = Kernel::CardinalityOutOfBound {
            violation: BoundViolation::AboveMaximum,
            kind: CollectionKind::Set,
            bound: CardinalityBound::new(0, 1).expect("a bound"),
            count: 2,
        };
        assert_eq!(
            failed(ModelCheckRefusal::AtomRefused(
                ClauseOutcome::KernelRefused(above)
            )),
            CatalogCode::new("cardinality_out_of_bound", "above-maximum")
        );
        let family = CatalogCode::new("integer_out_of_domain", "outside-domain");
        assert_eq!(
            failed(ModelCheckRefusal::AtomRefused(
                ClauseOutcome::FamilyRefused(family)
            )),
            family
        );
        assert_eq!(
            failed(ModelCheckRefusal::AtomRefused(ClauseOutcome::NotBoolean)),
            BROKEN
        );
        let exhausted = CatalogCode::new("resource_exhausted", "insufficient-next-charge");
        assert_eq!(
            failed(ModelCheckRefusal::StateUnobservable(
                qsl_eval::simulation::ExpansionStop {
                    cause: exhausted,
                    limit: None
                }
            )),
            exhausted
        );
        assert_eq!(
            failed(ModelCheckRefusal::StateUnobservable(
                qsl_eval::simulation::ExpansionStop {
                    cause: BROKEN,
                    limit: None
                }
            )),
            BROKEN
        );
    }

    /// FR-281's table: only a refusal whose own code is `unsupported_*` is
    /// `unsupported`; liveness and hyper request or engine defects are
    /// `failed`.
    #[trace("TC-763", "FR-281-AC-11")]
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
        assert_eq!(unsupported(ModelCheckRefusal::ReducedLiveness), not_yet);
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

    /// FR-281's table for EN-4: a refused predicate is not intercepted, so
    /// `analyze` settles it as FR-192-AC-7's rows do (a family refusal by its
    /// own code, an exhausted meter `incomplete`/`limit-reached`, anything else
    /// broken), equal to the provider entry's record (FR-280); `NotStatistical`
    /// is a disposition; the rest settle through FR-192's map.
    #[trace("TC-763", "FR-281-AC-11")]
    #[test]
    fn statistical_refusals_follow_the_table() {
        use quire_exact::{ChargePoint, Incomplete, Integer, LimitKind};

        let settled = |refusal: StatisticalRefusal| match statistical_refusal(&refusal) {
            EngineReport::Statistical(record) => {
                assert_eq!(
                    *record,
                    settle_statistical_refusal(&refusal).expect("a terminal record")
                );
                *record
            }
            _ => panic!("expected the map's record"),
        };
        let outside = CatalogCode::new("integer_out_of_domain", "outside-domain");
        let family = settled(StatisticalRefusal::PredicateRefused(
            ClauseOutcome::FamilyRefused(outside),
        ));
        assert_eq!(family.value, TerminalValue::Failed);
        assert_eq!(family.diagnostic, Some(outside));

        let exhausted = settled(StatisticalRefusal::PredicateRefused(
            ClauseOutcome::Incomplete(Incomplete {
                limit_kind: LimitKind::WorkUnits,
                limit: 9,
                consumed: 9,
                next_charge: Integer::from(1_i64),
                charge_point: ChargePoint::TextInputBytes,
            }),
        ));
        assert!(matches!(
            exhausted.value,
            TerminalValue::Incomplete(IncompleteCause::LimitReached { value: 9, .. })
        ));

        let broken = settled(StatisticalRefusal::PredicateRefused(
            ClauseOutcome::NotBoolean,
        ));
        assert_eq!(broken.value, TerminalValue::Failed);
        assert_eq!(broken.diagnostic, Some(BROKEN));

        let class = |refusal: StatisticalRefusal| match statistical_refusal(&refusal) {
            EngineReport::Refused(refusal) => code(&refusal),
            _ => None,
        };
        assert_eq!(
            class(StatisticalRefusal::Admission(Box::new(
                AdmissionFailure::Refused(record(Code::InvalidRuntimeInput, "wrong-role-mapping"))
            ))),
            Some("invalid_runtime_input/wrong-role-mapping".to_owned())
        );
    }
}
