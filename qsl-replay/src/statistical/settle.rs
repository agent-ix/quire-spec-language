// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-192 (ADR-024 SV-1 to SV-5, SV-9, SV-10): the one settlement map from
//! an EN-4 result to a QSpec FR-331 terminal record.
//!
//! A completed run settles `measured` with its verdict, on the `measured`
//! axis and never `proved` or `refuted`; a stopped run settles `incomplete`
//! or `failed`; an unsupported run or a refusal settles `unsupported`,
//! `inconclusive` or `failed`; a `NotStatistical` refusal is a negotiation
//! outcome with no terminal record. No record carries an FR-242 truth or an
//! FR-243 basis: the record has neither.

use qsl_eval::simulation::model::ExpansionLimits;
use qsl_eval::simulation::{ExpansionLimit, ExpansionStop};
use qsl_foundation::diagnostic::{CatalogCode, Category};
use qsl_foundation::{BackendId, RequestIndex};
use quire_exact::ScalarLimits;

use super::{
    ClaimForm, Decision, StatisticalMethod, StatisticalOutcome, StatisticalRefusal,
    StatisticalStop, StatisticalUnsupported,
};
use crate::model_check::limits::ModelCheckLimit;
use crate::proof_result::{
    IncompleteCause, InconclusiveCause, StatisticalRequestRefusal, TerminalRecord, TerminalValue,
    UnavailabilityCause,
};

/// `runtime_invariant`/`established-invariant-broken`: the diagnostic of an
/// internal failure (ADR-013 O-16).
const BROKEN: CatalogCode = CatalogCode::new("runtime_invariant", "established-invariant-broken");

/// The FR-120 limits of a statistical request's subject: the configured
/// bound a `Stopped` by an expansion that names a limit takes as its value,
/// since the stop carries only its cause and limit.
#[derive(Clone, Copy, Debug)]
pub struct SubjectLimits<'a> {
    /// The per-expansion candidate bound.
    pub expansion: ExpansionLimits,
    /// The evaluation meter budget.
    pub meter: &'a ScalarLimits,
}

/// One EN-4 item's terminal record (FR-192).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StatisticalRecord {
    /// The terminal value.
    pub value: TerminalValue,
    /// The one diagnostic a `failed` record carries: an internal failure's
    /// `runtime_invariant`/`established-invariant-broken`, or the stop's own
    /// cause for an expansion stop that names no limit.
    pub diagnostic: Option<CatalogCode>,
}

impl StatisticalRecord {
    fn of(value: TerminalValue) -> Self {
        Self {
            value,
            diagnostic: None,
        }
    }

    fn failed(diagnostic: CatalogCode) -> Self {
        Self {
            value: TerminalValue::Failed,
            diagnostic: Some(diagnostic),
        }
    }

    /// The record's QSpec FR-409 `measured` decision; `None` for every
    /// record that is not `measured`.
    pub fn measured(&self) -> Option<Decision> {
        match &self.value {
            TerminalValue::Measured(verdict) => Some(verdict.decision),
            TerminalValue::Proved { .. }
            | TerminalValue::Tested
            | TerminalValue::Refuted
            | TerminalValue::Declined { .. }
            | TerminalValue::Unsupported(_)
            | TerminalValue::Incomplete(_)
            | TerminalValue::Inconclusive(_)
            | TerminalValue::Failed => None,
        }
    }

    /// The record's ADR-013 O-16 category.
    pub fn category(&self) -> Category {
        self.value.category()
    }

    /// This record as the FR-331 terminal record of the item at
    /// `request_index`, produced by `backend`, with its diagnostic.
    pub fn into_terminal(self, request_index: RequestIndex, backend: BackendId) -> TerminalRecord {
        TerminalRecord::with_backend(request_index, backend, self.value)
            .with_diagnostic(self.diagnostic)
    }
}

/// FR-192's `NotStatistical` row: the method does not decide the claim's
/// form, so the request is a negotiation `invalid-request` with no terminal
/// record (QSpec FR-408 Admission, FR-331).
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[error("{method:?} does not decide a {form:?} claim")]
pub struct NotStatisticalRequest {
    /// The requested method.
    pub method: StatisticalMethod,
    /// The claim's form.
    pub form: ClaimForm,
}

impl NotStatisticalRequest {
    /// The negotiation diagnostic, `invalid_capability`/`not-statistical`.
    pub const fn diagnostic(&self) -> CatalogCode {
        CatalogCode::new("invalid_capability", "not-statistical")
    }
}

/// A refusal FR-192 settles with no terminal record.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum UnsettledRefusal {
    /// FR-192's `NotStatistical` row: a negotiation outcome.
    #[error(transparent)]
    NotStatistical(NotStatisticalRequest),
    /// The subject requires a bound (FR-101): a disposition, not an outcome
    /// (ADR-016 EX-8).
    #[error("the subject requires a bound")]
    RequiresBound,
    /// A claim predicate's read was refused (FR-196's `PredicateRefused`):
    /// FR-192's map has no row for it.
    #[error("a claim predicate's read was refused")]
    PredicateRefused,
    /// The subject did not admit (FR-120): FR-192's map has no row for it,
    /// so the refusal is the caller's to report as FR-106's admission
    /// failure.
    #[error("the subject did not admit")]
    Admission,
}

/// FR-192: settle `outcome` as one terminal record. `subject` supplies the
/// configured bound an expansion stop naming a limit settles with.
pub fn settle_statistical(
    outcome: &StatisticalOutcome,
    subject: SubjectLimits<'_>,
) -> StatisticalRecord {
    match outcome {
        StatisticalOutcome::Completed(verdict) => {
            StatisticalRecord::of(TerminalValue::Measured(verdict.clone()))
        }
        StatisticalOutcome::Stopped { stop, .. } => match stop {
            StatisticalStop::Limit { limit, value } => {
                StatisticalRecord::of(TerminalValue::Incomplete(IncompleteCause::LimitReached {
                    limit: ModelCheckLimit::Statistical(*limit),
                    value: *value,
                }))
            }
            StatisticalStop::Expansion(stop) => expansion_stop(stop, subject),
            StatisticalStop::AutomatonStates { value } => {
                StatisticalRecord::of(TerminalValue::Incomplete(IncompleteCause::LimitReached {
                    limit: ModelCheckLimit::MaxAutomatonStates,
                    value: *value,
                }))
            }
            StatisticalStop::Cancelled(source) => {
                StatisticalRecord::of(TerminalValue::Incomplete(IncompleteCause::Cancelled {
                    source: *source,
                }))
            }
        },
        StatisticalOutcome::Unsupported { cause, .. } => {
            StatisticalRecord::of(TerminalValue::Unsupported(match cause {
                StatisticalUnsupported::NotMarkov(_) => UnavailabilityCause::NotMarkov,
                StatisticalUnsupported::TotalAboveDrawRange { total } => {
                    UnavailabilityCause::TotalAboveDrawRange {
                        total: total.clone(),
                    }
                }
            }))
        }
    }
}

/// FR-192's expansion rows: a named limit settles `limit-reached` with its
/// configured bound; an unnamed stop settles `failed`, with
/// `runtime_invariant`/`established-invariant-broken` for an internal
/// failure and the stop's own cause otherwise.
fn expansion_stop(stop: &ExpansionStop, subject: SubjectLimits<'_>) -> StatisticalRecord {
    let reached = |limit, value| {
        StatisticalRecord::of(TerminalValue::Incomplete(IncompleteCause::LimitReached {
            limit,
            value,
        }))
    };
    match stop.limit {
        Some(ExpansionLimit::EvaluationMeter) => {
            reached(ModelCheckLimit::EvaluationMeter, subject.meter.work_units)
        }
        Some(ExpansionLimit::MaxCandidates) => reached(
            ModelCheckLimit::MaxCandidates,
            subject.expansion.max_candidates,
        ),
        None if stop.is_internal_failure() => StatisticalRecord::failed(BROKEN),
        None => StatisticalRecord::failed(stop.cause),
    }
}

/// FR-192: settle `refusal`, a request EN-4 refused with no outcome.
///
/// # Errors
///
/// [`UnsettledRefusal`] for a `NotStatistical`, `RequiresBound` or
/// `Admission` refusal, which settles no terminal record.
pub fn settle_statistical_refusal(
    refusal: &StatisticalRefusal,
) -> Result<StatisticalRecord, UnsettledRefusal> {
    let request = |refusal| {
        StatisticalRecord::of(TerminalValue::Unsupported(
            UnavailabilityCause::StatisticalRequest(refusal),
        ))
    };
    Ok(match refusal {
        StatisticalRefusal::NotStatistical { method, form } => {
            return Err(UnsettledRefusal::NotStatistical(NotStatisticalRequest {
                method: *method,
                form: *form,
            }))
        }
        StatisticalRefusal::RequiresBound(_) => return Err(UnsettledRefusal::RequiresBound),
        StatisticalRefusal::Admission(_) => return Err(UnsettledRefusal::Admission),
        StatisticalRefusal::PredicateRefused(_) => return Err(UnsettledRefusal::PredicateRefused),
        StatisticalRefusal::GeneratorMismatch { .. } => {
            request(StatisticalRequestRefusal::GeneratorMismatch)
        }
        StatisticalRefusal::TooManyTests { .. } => request(StatisticalRequestRefusal::TooManyTests),
        StatisticalRefusal::TraceIndexRange { .. } => {
            request(StatisticalRequestRefusal::TraceIndexRange)
        }
        StatisticalRefusal::FractionForEvent => {
            request(StatisticalRequestRefusal::FractionForEvent)
        }
        StatisticalRefusal::EveryScheduler => request(StatisticalRequestRefusal::EveryScheduler),
        StatisticalRefusal::ExactOnlyForm => request(StatisticalRequestRefusal::ExactOnlyForm),
        StatisticalRefusal::MissingConfidence => {
            request(StatisticalRequestRefusal::MissingConfidence)
        }
        StatisticalRefusal::ConfidenceBeyondPrecision => StatisticalRecord::of(
            TerminalValue::Unsupported(UnavailabilityCause::ConfidenceBeyondPrecision),
        ),
        StatisticalRefusal::NoTest => StatisticalRecord::of(TerminalValue::Inconclusive(
            InconclusiveCause::NoInitialState,
        )),
        StatisticalRefusal::Internal(fault) => StatisticalRecord::failed(fault.catalog_code()),
    })
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;
    use qsl_foundation::selection::DefinitionRef;
    use qsl_foundation::InternalFault;
    use quire_exact::{CancelCause, Integer, Rational};

    use super::*;
    use crate::outcome::{Operation, OutcomeDocument, OutcomeItem};
    use crate::statistical::{
        Confidence, Coverage, StatisticalBasis, StatisticalLimit, StatisticalProvenance,
        StatisticalVerdict, UndecidedCause,
    };

    fn ratio(numerator: i64, denominator: i64) -> Rational {
        Rational::new(Integer::from(numerator), Integer::from(denominator)).expect("ratio")
    }

    fn basis() -> StatisticalBasis {
        let hundredth = ratio(1, 100);
        StatisticalBasis {
            method: StatisticalMethod::Okamoto,
            confidence: Confidence::new(hundredth.clone(), hundredth.clone(), hundredth.clone())
                .expect("confidence"),
            samples: 23_026,
            draws: 23_026,
            tests: 1,
            alpha_per_test: hundredth.clone(),
            beta_per_test: hundredth,
            coverage: Coverage::FiniteSample,
        }
    }

    /// A non-unique post-state of a `tick` with no argument.
    fn not_markov() -> qsl_eval::simulation::model::NotMarkov {
        qsl_eval::simulation::model::NotMarkov {
            state: qsl_foundation::digest::DigestRecord::raw_artifact(b"healthy"),
            transition: qsl_eval::simulation::model::ScheduledIdentity {
                operation: "tick".to_owned(),
                receiver: quire_exact::ObjectReference::new(
                    quire_exact::UniverseId::from_digest([1; 32]),
                    quire_exact::EffectiveId::from_digest([2; 32]),
                    quire_exact::ObjectId::new("n").expect("a key"),
                ),
                arguments: Vec::new(),
            },
            random: Vec::new(),
            post_states: 2,
        }
    }

    fn provenance() -> StatisticalProvenance {
        StatisticalProvenance {
            seed: 7,
            sampler: DefinitionRef::new("agent-ix", "quire.simulation.sampler/v1")
                .expect("sampler"),
            tests: Vec::new(),
        }
    }

    fn completed(decision: Decision) -> StatisticalOutcome {
        StatisticalOutcome::Completed(Box::new(StatisticalVerdict {
            decision,
            estimate: Some(ratio(97, 100)),
            interval: Some((ratio(96, 100), ratio(98, 100))),
            basis: basis(),
            provenance: provenance(),
            tests: Vec::new(),
            witnesses: Vec::new(),
        }))
    }

    fn stopped(stop: StatisticalStop) -> StatisticalOutcome {
        StatisticalOutcome::Stopped {
            stop,
            partial: Box::new(basis()),
            provenance: provenance(),
        }
    }

    fn meter() -> ScalarLimits {
        ScalarLimits {
            integer_bits: 64,
            decimal_digits: 64,
            scale_expansion: 64,
            text_input_bytes: 64,
            text_scalars: 64,
            normalized_scalars: 64,
            unit_edges: 64,
            value_occurrences: 64,
            work_units: 4_096,
            result_units: 64,
        }
    }

    fn settle(outcome: &StatisticalOutcome) -> StatisticalRecord {
        settle_statistical(
            outcome,
            SubjectLimits {
                expansion: ExpansionLimits {
                    max_candidates: 512,
                },
                meter: &meter(),
            },
        )
    }

    /// `record`'s outcome item, as the wire writes it.
    fn wire(record: StatisticalRecord) -> serde_json::Value {
        let terminal = record.into_terminal(RequestIndex::new(0), BackendId::new("en-4"));
        serde_json::to_value(OutcomeItem::from_terminal(&terminal)).expect("an item")
    }

    fn incomplete(limit: ModelCheckLimit, value: u64) -> TerminalValue {
        TerminalValue::Incomplete(IncompleteCause::LimitReached { limit, value })
    }

    /// FR-192's map from `StatisticalOutcome`: a completed run settles
    /// `measured` with its decision, accepted as success, rejected as a
    /// violation and undecided as inconclusive; a budget, an expansion that
    /// names a limit and a cancellation settle `incomplete` (the budget
    /// naming `statistical.<member>`, the expansion its configured bound);
    /// an expansion naming no limit settles `failed` with
    /// `runtime_invariant`/`established-invariant-broken` when internal and
    /// its own cause otherwise; an unsupported run settles `unsupported`
    /// with its cause.
    #[trace("TC-627", "FR-192-AC-1")]
    #[test]
    fn each_outcome_row_settles_as_the_map_states() {
        for (decision, category) in [
            (Decision::Accepted, Category::Success),
            (Decision::Rejected, Category::Violation),
            (
                Decision::Undecided(UndecidedCause::IndifferenceRegion),
                Category::Inconclusive,
            ),
        ] {
            let outcome = completed(decision);
            let record = settle(&outcome);
            let StatisticalOutcome::Completed(verdict) = &outcome else {
                unreachable!("a completed outcome")
            };
            assert_eq!(record.value, TerminalValue::Measured(verdict.clone()));
            assert_eq!(record.measured(), Some(decision));
            assert_eq!(record.category(), category);
            assert_eq!(record.diagnostic, None);
            let json = wire(record);
            assert_eq!(json["result"], "measured");
            assert!(json.get("basis").is_none() && json.get("certification").is_none());
        }

        let budget = settle(&stopped(StatisticalStop::Limit {
            limit: StatisticalLimit::MaxSamples,
            value: 5,
        }));
        let max_samples = ModelCheckLimit::Statistical(StatisticalLimit::MaxSamples);
        assert_eq!(budget.value, incomplete(max_samples, 5));
        assert_eq!(max_samples.setting(), "statistical.max_samples");
        assert_eq!(max_samples.as_str(), "max-samples");
        assert_eq!(budget.category(), Category::Incomplete);
        assert_eq!(budget.measured(), None);
        let json = wire(budget);
        assert_eq!(json["cause"]["setting"], "statistical.max_samples");
        assert!(json.get("basis").is_none());

        let expansion = |limit, cause| {
            settle(&stopped(StatisticalStop::Expansion(ExpansionStop {
                cause,
                limit,
            })))
        };
        let exhausted = CatalogCode::new("resource_exhausted", "insufficient-next-charge");
        let metered = expansion(Some(ExpansionLimit::EvaluationMeter), exhausted);
        assert_eq!(
            metered.value,
            incomplete(ModelCheckLimit::EvaluationMeter, 4_096)
        );
        let json = wire(metered);
        assert_eq!(json["result"], "incomplete");
        assert_eq!(json["cause"]["kind"], "limit-reached");
        assert_eq!(json["cause"]["limit"], "evaluation-meter");
        assert_eq!(json["cause"]["value"], 4_096);
        assert_eq!(json["cause"]["setting"], "meter");
        assert_eq!(
            expansion(Some(ExpansionLimit::MaxCandidates), exhausted).value,
            incomplete(ModelCheckLimit::MaxCandidates, 512)
        );
        let internal = expansion(None, BROKEN);
        assert_eq!(internal.value, TerminalValue::Failed);
        assert_eq!(internal.diagnostic, Some(BROKEN));
        assert_eq!(internal.category(), Category::InternalFailure);
        let refused = CatalogCode::new("unsupported_construct", "not-yet-implemented");
        assert!(!ExpansionStop {
            cause: refused,
            limit: None
        }
        .is_internal_failure());
        let own = expansion(None, refused);
        assert_eq!(own.value, TerminalValue::Failed);
        assert_eq!(own.diagnostic, Some(refused));

        assert_eq!(
            settle(&stopped(StatisticalStop::AutomatonStates {
                value: 1 << 20
            }))
            .value,
            TerminalValue::Incomplete(IncompleteCause::LimitReached {
                limit: ModelCheckLimit::MaxAutomatonStates,
                value: 1 << 20,
            })
        );
        assert_eq!(
            settle(&stopped(StatisticalStop::Cancelled(CancelCause::Requested))).value,
            TerminalValue::Incomplete(IncompleteCause::Cancelled {
                source: CancelCause::Requested
            })
        );

        let not_markov = settle(&StatisticalOutcome::Unsupported {
            cause: StatisticalUnsupported::NotMarkov(Box::new(not_markov())),
            provenance: provenance(),
        });
        assert_eq!(
            not_markov.value,
            TerminalValue::Unsupported(UnavailabilityCause::NotMarkov)
        );
        assert_eq!(not_markov.category(), Category::Unsupported);
        assert!(wire(not_markov).get("basis").is_none());
        let total = Integer::from(7_i64);
        let range = settle(&StatisticalOutcome::Unsupported {
            cause: StatisticalUnsupported::TotalAboveDrawRange {
                total: total.clone(),
            },
            provenance: provenance(),
        });
        let TerminalValue::Unsupported(cause) = &range.value else {
            panic!("unsupported: {range:?}")
        };
        assert_eq!(cause, &UnavailabilityCause::TotalAboveDrawRange { total });
        assert_eq!(cause.as_str(), "total-above-draw-range");
        assert_eq!(cause.code(), None);
        assert_eq!(wire(range)["cause"]["total"], "7");
    }

    /// FR-192's refusal rows: `NoTest` settles `inconclusive`,
    /// `no-initial-state`; `ConfidenceBeyondPrecision` settles `unsupported`,
    /// `confidence-beyond-precision`; a request EN-4 cannot run as asked
    /// settles `unsupported_projection`/`unsupported-requested-capability`
    /// naming the refusal; `Internal` settles `failed` with
    /// `runtime_invariant`/`established-invariant-broken`; `NotStatistical`
    /// settles no record, with `invalid_capability`/`not-statistical`.
    #[trace("TC-627", "FR-192-AC-1")]
    #[test]
    fn each_refusal_row_settles_as_the_map_states() {
        let settled = |refusal: StatisticalRefusal| {
            settle_statistical_refusal(&refusal).expect("a terminal record")
        };
        assert_eq!(
            settled(StatisticalRefusal::NoTest).value,
            TerminalValue::Inconclusive(InconclusiveCause::NoInitialState)
        );
        let precision = settled(StatisticalRefusal::ConfidenceBeyondPrecision);
        let TerminalValue::Unsupported(cause) = &precision.value else {
            panic!("unsupported: {precision:?}")
        };
        assert_eq!(cause.as_str(), "confidence-beyond-precision");

        let sampler = DefinitionRef::new("agent-ix", "quire.simulation.other/v1").expect("sampler");
        for (refusal, named) in [
            (
                StatisticalRefusal::GeneratorMismatch { supplied: sampler },
                StatisticalRequestRefusal::GeneratorMismatch,
            ),
            (
                StatisticalRefusal::TooManyTests { count: 1 << 33 },
                StatisticalRequestRefusal::TooManyTests,
            ),
            (
                StatisticalRefusal::TraceIndexRange {
                    max_draws: u64::MAX,
                    tests: 2,
                },
                StatisticalRequestRefusal::TraceIndexRange,
            ),
            (
                StatisticalRefusal::FractionForEvent,
                StatisticalRequestRefusal::FractionForEvent,
            ),
            (
                StatisticalRefusal::EveryScheduler,
                StatisticalRequestRefusal::EveryScheduler,
            ),
            (
                StatisticalRefusal::ExactOnlyForm,
                StatisticalRequestRefusal::ExactOnlyForm,
            ),
            (
                StatisticalRefusal::MissingConfidence,
                StatisticalRequestRefusal::MissingConfidence,
            ),
        ] {
            let record = settled(refusal);
            let TerminalValue::Unsupported(cause) = &record.value else {
                panic!("unsupported: {record:?}")
            };
            assert_eq!(cause, &UnavailabilityCause::StatisticalRequest(named));
            assert_eq!(cause.code(), Some("unsupported_projection"));
            assert_eq!(cause.as_str(), "unsupported-requested-capability");
            let json = wire(record);
            assert_eq!(json["cause"]["refusal"], named.as_str());
            if let Some(literal) = match named {
                StatisticalRequestRefusal::EveryScheduler => Some("every-scheduler"),
                StatisticalRequestRefusal::ExactOnlyForm => Some("exact-only-form"),
                StatisticalRequestRefusal::MissingConfidence => Some("missing-confidence"),
                _ => None,
            } {
                // FR-192's spellings, not the function under test's.
                assert_eq!(json["cause"]["refusal"], literal);
            }
            assert_eq!(json["cause"]["code"], "unsupported_projection");
        }

        let internal = settled(StatisticalRefusal::Internal(InternalFault::new(
            "statistical",
            "x",
        )));
        assert_eq!(internal.value, TerminalValue::Failed);
        assert_eq!(internal.diagnostic, Some(BROKEN));
        let terminal = internal
            .clone()
            .into_terminal(RequestIndex::new(0), BackendId::new("en-4"));
        assert_eq!(terminal.diagnostic(), Some(BROKEN));
        let json = wire(internal);
        assert_eq!(json["result"], "failed");
        assert_eq!(json["cause"]["kind"], "established-invariant-broken");
        assert_eq!(json["cause"]["code"], "runtime_invariant");

        let negotiation = settle_statistical_refusal(&StatisticalRefusal::NotStatistical {
            method: StatisticalMethod::Sprt,
            form: ClaimForm::MeanOfFraction,
        })
        .expect_err("no terminal record");
        let UnsettledRefusal::NotStatistical(negotiation) = negotiation else {
            panic!("a negotiation outcome: {negotiation:?}")
        };
        assert_eq!(
            negotiation.diagnostic(),
            CatalogCode::new("invalid_capability", "not-statistical")
        );
        let refused = StatisticalRefusal::Admission(Box::new(
            qsl_semantics::model::observation::AdmissionFailure::Fault(InternalFault::new(
                "statistical",
                "admits",
            )),
        ));
        assert_eq!(
            settle_statistical_refusal(&refused),
            Err(UnsettledRefusal::Admission)
        );
        assert_eq!(
            settle_statistical_refusal(&StatisticalRefusal::PredicateRefused(
                qsl_eval::simulation::model::ClauseOutcome::NotBoolean
            )),
            Err(UnsettledRefusal::PredicateRefused)
        );
    }

    /// FR-192: a `measured` item writes its result `measured`, its decision
    /// on the `measured` axis and no basis or certification; a rejected
    /// measurement makes the document's category a violation, and an
    /// accepted one success.
    #[trace("TC-627")]
    #[test]
    fn a_measured_item_writes_its_decision_and_no_basis() {
        let item = |decision| {
            let record = settle(&completed(decision))
                .into_terminal(RequestIndex::new(0), BackendId::new("en-4"));
            OutcomeItem::from_terminal(&record)
        };
        let rejected = item(Decision::Rejected);
        let json = serde_json::to_value(&rejected).expect("an item");
        assert_eq!(json["result"], "measured");
        assert_eq!(json["measured"], "rejected");
        assert_eq!(json["category"], "violation");
        assert!(json.get("basis").is_none() && json.get("certification").is_none());

        let failing = OutcomeDocument::settled(Operation::Analyze, None, vec![rejected]);
        assert_eq!(failing.category(), Category::Violation);
        let passing =
            OutcomeDocument::settled(Operation::Analyze, None, vec![item(Decision::Accepted)]);
        assert_eq!(passing.category(), Category::Success);
    }
}
