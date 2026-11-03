// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-840, TC-848 (FR-330): `run_clause`'s `Temporal` selection over
//! the `Counter` unit, from source, through genuine FR-106 admission and the
//! TemporalTrace evaluator.
//!
//! The package is TC-1043's `Counter` (`value`, `step`, operation `noop`).
//! Each clause below is the spec's, with `Counters::Counter` for the model
//! alias and `noop` for the fairness operation.

use qsl_semantics::model::observation::AdmissionFailure;
use quire_semantic_value::catalog::Category;
use quire_semantic_value::fairness::{FairnessGranularity, FairnessKind};

use super::receiver_frames::{counter, fixture, snapshot, unit_and_packages};
use super::*;
use crate::execute::{run_spine_with_temporal_selections_and_execution_points, SpineUnit};
use crate::proof_result::SettlementBasis;
use crate::spine::{
    ClauseRunReport, OverObject, TemporalTraceRefusal, TemporalVerdict, TraceInput,
};
use crate::ReplayRequestWire;
use quire_exact::Identifier;

const OPERATIONS: &[&str] = &["noop"];

enum ObservedReplayExpectation {
    Formula,
    Undefined(qsl_eval::temporal::UndefinedCause),
    UndefinedAtNode(qsl_eval::temporal::UndefinedCause, usize),
}

/// A unit holds one temporal profile, so the bounded and the infinite-trace
/// clauses of the spec's `Counter` unit are two units.
#[derive(Clone, Copy)]
enum Unit {
    /// `Bounded`, under event-position false-extension.
    Bounded,
    /// `Reaches` and `Fair`, under infinite-trace.
    Infinite,
    /// `Undefined`, under the bounded profile, with integer modulo by zero
    /// at position 2 (FR-330-AC-5).
    Undefined,
    /// `Guarded`, under the bounded profile: an `on each` clause activated
    /// where the counter's value is 1 (FR-337, FR-330).
    Guarded,
    /// An on-each activation guard that becomes undefined at value 2.
    GuardUndefined,
    /// Timestamped finite-window evaluation (FR-501).
    Timestamped,
    /// Timestamped finite-window `[0,1]` property.
    TimestampedOne,
    /// Timestamped finite-window `[0,2]` property.
    TimestampedTwo,
    /// Timestamped finite-window `[2,3]` existential.
    TimestampedLaterEventually,
    /// Timestamped finite-window `[2,3]` universal.
    TimestampedLaterAlways,
    /// Timestamped finite-window `on each` property.
    TimestampedEach,
    /// Timestamped finite-window `[0,5]` universal.
    TimestampedAlwaysFive,
    /// Timestamped finite-window past operator.
    TimestampedOnce,
}

impl Unit {
    fn profile(self) -> &'static str {
        match self {
            Self::Bounded => "profile t = \"quire.temporal.event-position.false-extension/v1\";\n",
            Self::Infinite => "profile t = \"quire.temporal.infinite-trace/v1\";\n",
            Self::Undefined | Self::Guarded | Self::GuardUndefined => {
                "profile t = \"quire.temporal.event-position.false-extension/v1\";\n"
            }
            Self::Timestamped => {
                "profile t = \"quire.temporal.timestamped-event.finite-window/v1\";\n"
            }
            Self::TimestampedOne
            | Self::TimestampedTwo
            | Self::TimestampedLaterEventually
            | Self::TimestampedLaterAlways => {
                "profile t = \"quire.temporal.timestamped-event.finite-window/v1\";\n"
            }
            Self::TimestampedEach => {
                "profile t = \"quire.temporal.timestamped-event.finite-window/v1\";\n"
            }
            Self::TimestampedAlwaysFive | Self::TimestampedOnce => {
                "profile t = \"quire.temporal.timestamped-event.finite-window/v1\";\n"
            }
        }
    }

    fn clauses(self) -> &'static str {
        match self {
            Self::Bounded => {
                "temporal Bounded using t over (c: Counters::Counter) clock \"model-steps\" \
                 on origin { eventually[0,1] holds(c.value = 2) }\n"
            }
            Self::Infinite => {
                "temporal Reaches using t over (c: Counters::Counter) clock \"model-steps\" \
                 on origin { eventually holds(c.value = 2) }\n\
                 temporal Fair using t over (c: Counters::Counter) clock \"model-steps\" \
                 on origin { fair weak Counters::Counter::noop; \
                 always eventually holds(c.value = 2) }\n"
            }
            Self::Undefined => {
                "temporal Undefined using t over (c: Counters::Counter) clock \"model-steps\" \
                 on origin { always[0,2] holds(3 mod (2 - c.value) >= 0) }\n"
            }
            Self::Guarded => {
                "temporal Guarded using t over (c: Counters::Counter) clock \"model-steps\" \
                 on each (q: Counters::Counter) when (q.value = 1) \
                 { eventually[0,1] holds(c.value = 2) }\n"
            }
            Self::GuardUndefined => {
                "temporal GuardUndefined using t over (c: Counters::Counter) clock \"model-steps\" \
                 on each (q: Counters::Counter) when (3 mod (2 - q.value) >= 0) \
                 { always[0,0] holds(c.value = c.value) }\n"
            }
            Self::Timestamped => {
                "temporal Window using t over (c: Counters::Counter) clock \"ms\" \
                 on origin { eventually[0,5] holds(c.value > 0) }\n"
            }
            Self::TimestampedOne => {
                "temporal Window using t over (c: Counters::Counter) clock \"ms\" \
                 on origin { eventually[0,1] holds(c.value > 0) }\n"
            }
            Self::TimestampedTwo => {
                "temporal Window using t over (c: Counters::Counter) clock \"ms\" \
                 on origin { eventually[0,2] holds(c.value > 0) }\n"
            }
            Self::TimestampedLaterEventually => {
                "temporal Window using t over (c: Counters::Counter) clock \"ms\" \
                 on origin { eventually[2,3] holds(c.value > 0) }\n"
            }
            Self::TimestampedLaterAlways => {
                "temporal Window using t over (c: Counters::Counter) clock \"ms\" \
                 on origin { always[2,3] holds(c.value > 0) }\n"
            }
            Self::TimestampedEach => {
                "temporal Window using t over (c: Counters::Counter) clock \"ms\" \
                 on each (q: Counters::Counter) { eventually[0,2] holds(c.value > 0) }\n"
            }
            Self::TimestampedAlwaysFive => {
                "temporal Window using t over (c: Counters::Counter) clock \"ms\" \
                 on origin { always[0,5] holds(c.value > 0) }\n"
            }
            Self::TimestampedOnce => {
                "temporal Window using t over (c: Counters::Counter) clock \"ms\" \
                 on each (q: Counters::Counter) { once[0,4] holds(c.value > 0) }\n"
            }
        }
    }
}

/// The binding a trace of the `Window` clause asserts (FR-522): its profile,
/// clock name and clock parameters.
fn finite_window_binding() -> crate::spine::TraceBinding {
    crate::spine::TraceBinding {
        profile: "quire.temporal.timestamped-event.finite-window/v1".to_owned(),
        clock_name: "ms".to_owned(),
        parameters: qsl_package::ClockConfiguration::TimestampedEvent {
            timestamp_unit: "ms".into(),
        },
    }
}

/// `Window` over timestamped finite-window positions.
fn timestamped(
    stamps: &[u64],
    values: &[i64],
    watermark: u64,
    complete_through: Option<u64>,
) -> ClauseRunRequest {
    timestamped_with(
        Unit::Timestamped,
        stamps,
        values,
        watermark,
        complete_through,
    )
}

fn timestamped_with(
    unit: Unit,
    stamps: &[u64],
    values: &[i64],
    watermark: u64,
    complete_through: Option<u64>,
) -> ClauseRunRequest {
    assert_eq!(stamps.len(), values.len());
    request(
        unit,
        |labels| ClauseRunSelection::Temporal {
            name: "Window".to_owned(),
            trace: TraceInput::Timestamped {
                observations: labels
                    .iter()
                    .zip(stamps)
                    .map(|(document, stamp)| crate::spine::TimestampedObservation {
                        document: document.clone(),
                        stamp: *stamp,
                        order_key: None,
                    })
                    .collect(),
                progress: crate::spine::ProgressAssertion {
                    watermark,
                    completeness: complete_through
                        .map_or(crate::spine::ProgressCompleteness::Incomplete, |through| {
                            crate::spine::ProgressCompleteness::Complete { through }
                        }),
                    scope: crate::spine::ProgressClosure::Closed,
                    execution: crate::spine::ProgressClosure::Closed,
                },
                window: Box::new(crate::spine::TimestampedWindow {
                    binding: Some(finite_window_binding()),
                    authoritative_origin: true,
                    cover: None,
                    missing_valuations: Vec::new(),
                }),
            },
            over: over("c1"),
            ledger: None,
        },
        values,
    )
}

/// The `Counter` unit and a request for `selection` over snapshots with
/// `c1.value` at each of `values`, in the order the trace names them.
fn request(
    unit: Unit,
    selection: impl FnOnce(&[DocumentRef]) -> ClauseRunSelection,
    values: &[i64],
) -> ClauseRunRequest {
    let (text, packages) = unit_and_packages(OPERATIONS);
    // The profile declaration belongs to the header, before any clause.
    let header = "profile v = \"quire.value.complete/v1\";\n";
    assert!(text.contains(header), "{text}");
    let mut text = text.replacen(header, &format!("{header}{}", unit.profile()), 1);
    text.push_str(unit.clauses());
    let documents: Vec<(DocumentRef, Vec<u8>)> = values
        .iter()
        .enumerate()
        .map(|(position, value)| {
            document_ref_and_bytes(&format!("counter-{position}"), |label| {
                snapshot(label, "current", &[counter("c1", *value)])
            })
        })
        .collect();
    let labels: Vec<DocumentRef> = documents.iter().map(|(label, _)| label.clone()).collect();
    let mut request = config_version_request_for(
        text,
        packages,
        "counter-temporal.native",
        selection(&labels),
    );
    for (label, bytes) in documents {
        request.snapshots.insert(label.clone(), bytes);
    }
    request
}

fn over(key: &str) -> OverObject {
    OverObject {
        population: fixture::id("counters"),
        key: key.to_owned(),
    }
}

/// `name` over a finite trace of `values`.
fn finite(unit: Unit, name: &str, values: &[i64]) -> ClauseRunRequest {
    request(
        unit,
        |labels| ClauseRunSelection::Temporal {
            name: name.to_owned(),
            trace: TraceInput::Finite(labels.to_vec()),
            over: over("c1"),
            ledger: None,
        },
        values,
    )
}

/// `Reaches` over `Finite` 0, 1, 2 with an `over` naming key `ghost`.
fn ghost() -> ClauseRunRequest {
    request(
        Unit::Infinite,
        |labels| ClauseRunSelection::Temporal {
            name: "Reaches".to_owned(),
            trace: TraceInput::Finite(labels.to_vec()),
            over: over("ghost"),
            ledger: None,
        },
        &[0, 1, 2],
    )
}

/// `name` over a lasso whose prefix is the first `prefix` of `values` and
/// whose loop is the rest.
fn lasso(unit: Unit, name: &str, prefix: usize, values: &[i64]) -> ClauseRunRequest {
    request(
        unit,
        |labels| ClauseRunSelection::Temporal {
            name: name.to_owned(),
            trace: TraceInput::Lasso {
                prefix: labels[..prefix].to_vec(),
                repeat: labels[prefix..].to_vec(),
            },
            over: over("c1"),
            ledger: None,
        },
        values,
    )
}

fn report(request: ClauseRunRequest) -> ClauseRunReport {
    let temporal_selections = match &request.source {
        ClauseRunSource::Program {
            identity,
            path,
            bytes,
        } => temporal_clock_selections(identity, path, bytes),
        #[cfg(feature = "quire-extraction")]
        ClauseRunSource::Extracted(extracted) => {
            let body = extracted.map().body();
            temporal_clock_selections(body.identity(), body.path(), body.text().as_bytes())
        }
    };
    crate::spine::run_clause_with_temporal_selections(request, &temporal_selections)
        .expect("a well-formed request always reports")
}

fn temporal_clock_selections(
    identity: &qsl_foundation::SourceIdentity,
    path: &str,
    bytes: &[u8],
) -> Vec<qsl_package::TemporalClockSelection> {
    use qsl_forms::DeclarationForm;
    use qsl_package::{ClockConfiguration, TemporalClockSelection};

    let parsed = qsl_cst::parse(identity.clone(), path, bytes, qsl_cst::Limits::default())
        .expect("the test source parses");
    let unit = qsl_forms::build_unit(&parsed).expect("the test source forms");
    let profiles = &unit.selections().profiles;
    unit.forms()
        .iter()
        .filter_map(|form| {
            let DeclarationForm::Temporal(clause) = form.form() else {
                return None;
            };
            let selected = profiles
                .iter()
                .find(|profile| profile.alias == clause.profile.alias)
                .expect("the temporal profile is selected");
            let clock = match selected.identity.as_str() {
                "quire.temporal.event-position.false-extension/v1" => {
                    ClockConfiguration::EventPosition {
                        sequence_authority: "counter-event-sequence".into(),
                    }
                }
                "quire.temporal.timestamped-event.finite-window/v1" => {
                    ClockConfiguration::TimestampedEvent {
                        timestamp_unit: "ms".into(),
                    }
                }
                _ => return None,
            };
            Some(TemporalClockSelection {
                source: parsed.source().reference().clone(),
                declaration_span: form.span(),
                clock,
            })
        })
        .collect()
}

fn outcome(report: &ClauseRunReport) -> &TemporalVerdict {
    let ClauseDisposition::Temporal(outcome) = &report.disposition else {
        panic!("expected a temporal outcome, got {:?}", report.disposition);
    };
    outcome
}

fn exit_code(report: &ClauseRunReport) -> u8 {
    report.disposition.exit_code()
}

struct ObservedReplayInput {
    wire: ReplayRequestWire,
    temporal_selections: Vec<qsl_package::TemporalClockSelection>,
}

impl std::ops::Deref for ObservedReplayInput {
    type Target = ReplayRequestWire;

    fn deref(&self) -> &Self::Target {
        &self.wire
    }
}

impl std::ops::DerefMut for ObservedReplayInput {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.wire
    }
}

fn replay_observed(
    input: ObservedReplayInput,
    replay_limits: crate::ReplayLimits,
) -> Result<crate::ReplayResult, crate::ReplayRefusal> {
    let ObservedReplayInput {
        wire,
        temporal_selections,
    } = input;
    crate::replay_with_temporal_selections(wire, &temporal_selections, replay_limits)
}

#[trace("TC-841", "FR-331-AC-1")]
#[test]
fn replay_facade_replays_the_observed_temporal_lasso() {
    let false_packet = observed_replay_request(
        "Reaches",
        Unit::Infinite,
        &[0, 1],
        Some(0),
        0,
        ObservedReplayExpectation::Formula,
    );
    let reproduced = replay_observed(false_packet, crate::ReplayLimits::default())
        .expect("the observed lasso is admissible");
    let crate::ReplayResult::ObservedTemporal(reproduced) = reproduced else {
        panic!("the replay returns its observed-temporal arm");
    };
    assert_eq!(
        reproduced.settlement(),
        crate::ObservedTemporalSettlement::ReproducedWithEvaluatedWitness,
        "{reproduced:?}"
    );
    assert_eq!(reproduced.category(), Category::Violation);
    assert!(matches!(
        reproduced.outcome(),
        qsl_eval::temporal::TemporalOutcome::Violated { at } if at.index() == 0
    ));

    let holding_packet = observed_replay_request(
        "Reaches",
        Unit::Infinite,
        &[0, 1, 2],
        Some(0),
        0,
        ObservedReplayExpectation::Formula,
    );
    let inconclusive = replay_observed(holding_packet, crate::ReplayLimits::default())
        .expect("the observed lasso is admissible");
    let crate::ReplayResult::ObservedTemporal(inconclusive) = inconclusive else {
        panic!("the replay returns its observed-temporal arm");
    };
    assert_eq!(
        inconclusive.settlement(),
        crate::ObservedTemporalSettlement::Inconclusive
    );
    assert_eq!(inconclusive.category(), Category::Success);
    assert!(inconclusive.parity().is_some());
}

#[trace("TC-841", "FR-331-AC-2")]
#[test]
fn observed_temporal_replay_checks_bounded_interval_and_position() {
    let reproduced = observed_replay_request(
        "Bounded",
        Unit::Bounded,
        &[0, 1, 2],
        None,
        0,
        ObservedReplayExpectation::Formula,
    );
    let replayed = replay_observed(reproduced, crate::ReplayLimits::default())
        .expect("the bounded counterexample is admissible");
    let crate::ReplayResult::ObservedTemporal(replayed) = replayed else {
        panic!("the replay returns its observed-temporal arm");
    };
    assert_eq!(
        replayed.settlement(),
        crate::ObservedTemporalSettlement::ReproducedWithEvaluatedWitness
    );
    assert_eq!(replayed.category(), Category::Violation);

    let wrong_position = observed_replay_request(
        "Bounded",
        Unit::Bounded,
        &[0, 1, 2],
        None,
        1,
        ObservedReplayExpectation::Formula,
    );
    let replayed = replay_observed(wrong_position, crate::ReplayLimits::default())
        .expect("the bounded counterexample is admissible");
    let crate::ReplayResult::ObservedTemporal(replayed) = replayed else {
        panic!("the replay returns its observed-temporal arm");
    };
    assert_eq!(
        replayed.settlement(),
        crate::ObservedTemporalSettlement::Inconclusive
    );
    assert_eq!(replayed.position().index(), 1);

    let wrong_interval = rewrite_observed_payload(
        observed_replay_request(
            "Bounded",
            Unit::Bounded,
            &[0, 1, 2],
            None,
            0,
            ObservedReplayExpectation::Formula,
        ),
        |payload| payload.interval = None,
    );
    assert!(matches!(
        replay_observed(wrong_interval, crate::ReplayLimits::default()),
        Err(crate::ReplayRefusal::ObservedTemporalMismatch { member: "interval" })
    ));
}

#[trace("TC-847", "FR-331-AC-4")]
#[trace("TC-847", "FR-072-AC-6")]
#[test]
fn replay_facade_preserves_and_matches_an_observed_undefined_value() {
    let cause = qsl_eval::temporal::UndefinedCause::Kernel(quire_exact::Undefined::DivisionByZero);
    let packet = observed_replay_request(
        "Undefined",
        Unit::Undefined,
        &[0, 1, 2],
        None,
        2,
        ObservedReplayExpectation::Undefined(cause.clone()),
    );
    let reproduced = replay_observed(packet, crate::ReplayLimits::default())
        .expect("the undefined temporal packet is admissible");
    let crate::ReplayResult::ObservedTemporal(reproduced) = reproduced else {
        panic!("the replay returns its observed-temporal arm");
    };
    assert_eq!(
        reproduced.settlement(),
        crate::ObservedTemporalSettlement::ReproducedWithEvaluatedWitness,
        "{reproduced:?}"
    );
    assert_eq!(reproduced.position().index(), 2);
    assert!(reproduced.undefined_locus().is_some());
    assert!(matches!(
        reproduced.outcome(),
        qsl_eval::temporal::TemporalOutcome::Undefined(undefined)
            if undefined.at.index() == 2 && undefined.cause == cause
    ));

    let packet = observed_replay_request(
        "Undefined",
        Unit::Undefined,
        &[0, 1, 2],
        None,
        1,
        ObservedReplayExpectation::Undefined(qsl_eval::temporal::UndefinedCause::Kernel(
            quire_exact::Undefined::DivisionByZero,
        )),
    );
    let inconclusive = replay_observed(packet, crate::ReplayLimits::default())
        .expect("the undefined temporal packet is admissible");
    let crate::ReplayResult::ObservedTemporal(inconclusive) = inconclusive else {
        panic!("the replay returns its observed-temporal arm");
    };
    assert_eq!(
        inconclusive.settlement(),
        crate::ObservedTemporalSettlement::Inconclusive
    );
    assert_eq!(inconclusive.position().index(), 1);
    assert!(matches!(
        inconclusive.outcome(),
        qsl_eval::temporal::TemporalOutcome::Undefined(undefined)
            if undefined.at.index() == 2
    ));
    assert!(inconclusive.parity().is_some());
}

/// FR-331-AC-9: observed packets can name undefined atom evaluations, but
/// cannot identify an undefined `on each` activation guard.
#[trace("TC-847", "FR-331-AC-9")]
#[test]
fn an_observed_undefined_on_each_guard_settles_inconclusive() {
    let cause = qsl_eval::temporal::UndefinedCause::Kernel(quire_exact::Undefined::DivisionByZero);
    let packet = observed_replay_request(
        "GuardUndefined",
        Unit::GuardUndefined,
        &[2, 0, 1],
        None,
        0,
        ObservedReplayExpectation::Undefined(cause.clone()),
    );
    let replayed = replay_observed(packet, crate::ReplayLimits::default())
        .expect("the observed guarded packet is admissible");
    let crate::ReplayResult::ObservedTemporal(replayed) = replayed else {
        panic!("the replay returns its observed-temporal arm");
    };
    assert_eq!(
        replayed.settlement(),
        crate::ObservedTemporalSettlement::Inconclusive,
        "the packet has no activation locus"
    );
    assert_eq!(replayed.position().index(), 0);
    assert!(matches!(
        replayed.outcome(),
        qsl_eval::temporal::TemporalOutcome::Undefined(undefined)
            if undefined.at.index() == 0 && undefined.cause == cause
    ));
    assert_eq!(replayed.undefined_locus(), None);
    assert!(replayed.parity().is_some());
}

#[trace("TC-847", "FR-331-AC-8")]
#[test]
fn observed_temporal_replay_retains_short_incomplete_and_mismatched_values() {
    let cause = qsl_eval::temporal::UndefinedCause::Kernel(quire_exact::Undefined::DivisionByZero);
    let mut wrong_cause = observed_replay_request(
        "Undefined",
        Unit::Undefined,
        &[0, 1, 2],
        None,
        2,
        ObservedReplayExpectation::Undefined(cause.clone()),
    );
    wrong_cause = rewrite_observed_payload(wrong_cause, |payload| {
        let crate::ObservedTemporalKind::UndefinedEvaluation { cause, .. } = &mut payload.kind
        else {
            unreachable!("the packet describes an undefined evaluation");
        };
        *cause = qsl_eval::temporal::UndefinedCause::Kernel(quire_exact::Undefined::NoneValue);
    });
    let replayed = replay_observed(wrong_cause, crate::ReplayLimits::default())
        .expect("the mismatched undefined packet is admissible");
    let crate::ReplayResult::ObservedTemporal(replayed) = replayed else {
        panic!("the replay returns its observed-temporal arm");
    };
    assert_eq!(
        replayed.settlement(),
        crate::ObservedTemporalSettlement::Inconclusive
    );
    assert!(matches!(
        replayed.outcome(),
        qsl_eval::temporal::TemporalOutcome::Undefined(undefined)
            if undefined.at.index() == 2 && undefined.cause == cause
    ));
    let insufficient = qsl_eval::temporal::TemporalOutcome::InsufficientData(
        qsl_eval::temporal::InsufficientDataEvaluation {
            at: qsl_eval::temporal::TemporalPosition::new(0),
            atom: replayed
                .undefined_locus()
                .expect("the replay records its checked atom"),
            data: qsl_eval::temporal::InsufficientData {
                count: 1,
                min_count: 2,
            },
        },
    );
    let settled = crate::ObservedTemporalResult::settle(
        crate::ObservedTemporalKind::Formula,
        qsl_eval::temporal::TemporalPosition::new(0),
        insufficient.clone(),
        None,
        replayed.charges(),
    );
    assert_eq!(
        settled.settlement(),
        crate::ObservedTemporalSettlement::Inconclusive
    );
    assert_eq!(settled.outcome(), &insufficient);

    let wrong_locus = observed_replay_request(
        "Undefined",
        Unit::Undefined,
        &[0, 1, 2],
        None,
        2,
        ObservedReplayExpectation::UndefinedAtNode(cause.clone(), 0),
    );
    let replayed = replay_observed(wrong_locus, crate::ReplayLimits::default())
        .expect("the mismatched-locus packet is admissible");
    let crate::ReplayResult::ObservedTemporal(replayed) = replayed else {
        panic!("the replay returns its observed-temporal arm");
    };
    assert_eq!(
        replayed.settlement(),
        crate::ObservedTemporalSettlement::Inconclusive
    );
    assert!(replayed.undefined_locus().is_some());

    let mut limited = observed_replay_request(
        "Bounded",
        Unit::Bounded,
        &[0, 1, 2],
        None,
        0,
        ObservedReplayExpectation::Formula,
    );
    limited.accounting_limits.work_units = 0;
    let replayed = replay_observed(limited, crate::ReplayLimits::default())
        .expect("the limited temporal packet is admissible");
    let crate::ReplayResult::ObservedTemporal(replayed) = replayed else {
        panic!("the replay returns its observed-temporal arm");
    };
    assert_eq!(
        replayed.settlement(),
        crate::ObservedTemporalSettlement::Inconclusive
    );
    assert!(matches!(
        replayed.outcome(),
        qsl_eval::temporal::TemporalOutcome::Incomplete(_)
    ));
    assert_eq!(replayed.category(), Category::Incomplete);
    assert!(replayed.limit().is_some());
}

#[trace("TC-841", "FR-331-AC-5")]
#[test]
fn observed_temporal_replay_refuses_a_missing_fairness_premise() {
    let wire = observed_replay_request(
        "Fair",
        Unit::Infinite,
        &[0, 1],
        Some(0),
        0,
        ObservedReplayExpectation::Formula,
    );
    let refusal = replay_observed(wire, crate::ReplayLimits::default())
        .expect_err("an observed trace has no enabledness for fairness");
    assert!(matches!(
        refusal,
        crate::ReplayRefusal::MissingFairnessPremise { .. }
    ));
    assert_eq!(refusal.code().as_str(), "unsupported_projection");
    assert_eq!(refusal.cause(), Some("missing-fairness-premise"));
}

#[trace("TC-847", "FR-331-AC-6")]
#[trace("TC-847", "FR-071-AC-8")]
#[trace("TC-847", "FR-331-AC-3")]
#[test]
fn observed_temporal_source_and_target_refusals_precede_admission() {
    let mut wrong_source = observed_replay_request(
        "Reaches",
        Unit::Infinite,
        &[0, 1],
        Some(0),
        0,
        ObservedReplayExpectation::Formula,
    );
    let crate::ReplaySource::ObservedTemporal(envelope) = &wrong_source.source else {
        unreachable!("the helper constructs an observed temporal source");
    };
    let mut packet = envelope.to_packet();
    packet.source = Some(crate::EnvelopeSource::Input(Vec::new()));
    let envelope = crate::WitnessEnvelope::<crate::ObservedTemporalCounterexample>::reconstruct(
        packet,
        crate::ReplayLimits::default(),
    )
    .expect("the envelope admits either source arm");
    wrong_source.source = crate::ReplaySource::ObservedTemporal(Box::new(envelope));
    let refusal = replay_observed(wrong_source, crate::ReplayLimits::default())
        .expect_err("the observed temporal source must be a witness envelope");
    assert_eq!(refusal.pointer(), Some("/replay/source"));
    assert_eq!(refusal.code().as_str(), "invalid_runtime_input");
    assert_eq!(refusal.cause(), Some("invalid-value"));

    let mut wrong_target = observed_replay_request(
        "Reaches",
        Unit::Infinite,
        &[0, 1],
        Some(0),
        0,
        ObservedReplayExpectation::Formula,
    );
    wrong_target.replay_target.members.state_clause = None;
    wrong_target.replay_target.members.function =
        Some(crate::QualifiedName::new(vec![Identifier::new("Reaches").unwrap()]).unwrap());
    let crate::ReplaySource::ObservedTemporal(envelope) = &wrong_target.source else {
        unreachable!("the helper constructs an observed source");
    };
    wrong_target.replay_target.occurrence_key = Some(envelope.occurrence_key().clone());
    wrong_target.replay_target.over = None;
    let refusal = replay_observed(wrong_target, crate::ReplayLimits::default())
        .expect_err("the observed temporal arm requires a state-clause target");
    assert_eq!(refusal.pointer(), Some("/replay/observed_counterexample"));
    assert_eq!(refusal.cause(), Some("unknown-member"));

    let mut ordinary_state_clause = observed_replay_request(
        "Reaches",
        Unit::Infinite,
        &[0, 1],
        Some(0),
        0,
        ObservedReplayExpectation::Formula,
    );
    ordinary_state_clause.source = crate::ReplaySource::Input(Vec::new());
    assert!(matches!(
        replay_observed(ordinary_state_clause, crate::ReplayLimits::default()),
        Err(crate::ReplayRefusal::WrongReplayTarget { .. })
    ));

    let mismatched_position = rewrite_observed_packet(
        observed_replay_request(
            "Reaches",
            Unit::Infinite,
            &[0, 1],
            Some(0),
            0,
            ObservedReplayExpectation::Formula,
        ),
        |packet| packet.trace_position = Some(Some(crate::TracePosition::new("1".to_owned()))),
    );
    let refusal = replay_observed(mismatched_position, crate::ReplayLimits::default())
        .expect_err("the envelope and payload positions must agree");
    assert_eq!(
        refusal.pointer(),
        Some("/replay/observed_counterexample/trace_position")
    );
    assert_eq!(refusal.code().as_str(), "invalid_runtime_input");
    assert_eq!(refusal.cause(), Some("invalid-value"));

    let empty_loop = rewrite_observed_payload(
        observed_replay_request(
            "Reaches",
            Unit::Infinite,
            &[0, 1],
            Some(0),
            0,
            ObservedReplayExpectation::Formula,
        ),
        |payload| payload.loop_start = Some(2),
    );
    let refusal = replay_observed(empty_loop, crate::ReplayLimits::default())
        .expect_err("an observed lasso cannot have an empty loop");
    assert_eq!(
        refusal.pointer(),
        Some("/replay/observed_counterexample/loop")
    );
    assert_eq!(refusal.cause(), Some("invalid-value"));

    let out_of_range = rewrite_observed_packet(
        observed_replay_request(
            "Reaches",
            Unit::Infinite,
            &[0, 1],
            Some(0),
            0,
            ObservedReplayExpectation::Formula,
        ),
        |packet| {
            packet
                .family_payload
                .as_mut()
                .expect("the envelope carries its temporal payload")
                .trace_position = qsl_eval::temporal::TemporalPosition::new(7);
            packet.trace_position = Some(Some(crate::TracePosition::new("7".to_owned())));
        },
    );
    let refusal = replay_observed(out_of_range, crate::ReplayLimits::default())
        .expect_err("the decoded position must lie in the observed trace");
    assert_eq!(
        refusal.pointer(),
        Some("/replay/observed_counterexample/trace_position")
    );
    assert_eq!(refusal.cause(), Some("invalid-value"));
}

/// Build an FR-331 observed temporal request for a generated temporal unit.
fn observed_replay_request(
    name: &str,
    unit: Unit,
    values: &[i64],
    loop_start: Option<u64>,
    position: u64,
    expectation: ObservedReplayExpectation,
) -> ObservedReplayInput {
    use crate::model_check::counterexample::ObservedTemporalStep;
    use crate::{
        ReplayOverBinding, ReplaySource, ReplayTarget, TracePosition, Witness, WitnessEnvelope,
        WitnessPacket,
    };
    use qsl_foundation::source::provenance::OccurrenceKey;

    let (base, packages) = unit_and_packages(OPERATIONS);
    let header = "profile v = \"quire.value.complete/v1\";\n";
    let mut text = base.replacen(header, &format!("{header}{}", unit.profile()), 1);
    text.push_str(unit.clauses());
    let source_bytes = text.into_bytes();
    let source_identity =
        qsl_foundation::SourceIdentity::new("agent-ix", "clause-run-fixture", "git", "1");
    let path = "counter-temporal.native";
    let temporal_selections = temporal_clock_selections(&source_identity, path, &source_bytes);
    let package_name = qsl_semantics::library::LibraryName::new("fixture").expect("a library name");
    let dependencies = crate::spine::DependencyInput::default();
    let execution_points = crate::spine::StateClauseExecutionPoints::new();
    let compiled = run_spine_with_temporal_selections_and_execution_points(
        SpineUnit {
            identity: &source_identity,
            package: &package_name,
            path,
            bytes: &source_bytes,
        },
        &packages,
        &dependencies,
        &execution_points,
        &temporal_selections,
        crate::spine::SpineLimits::default(),
    )
    .expect("the temporal fixture compiles with its explicit clock selection");
    let package = compiled.checked.package();
    let clause_node = package
        .graph()
        .temporal_clauses()
        .iter()
        .find(|node| node.clause().name() == name)
        .expect("the fixture temporal clause exists");
    let node = clause_node.identity().clone();
    let occurrence = OccurrenceKey::new(node.clone(), clause_node.claim().clone());
    let domain_document = packages.values().next().expect("one model package");
    let documents: Vec<(DocumentRef, Vec<u8>)> = values
        .iter()
        .enumerate()
        .map(|(index, value)| {
            document_ref_and_bytes(&format!("replay-counter-{index}"), |label| {
                snapshot(label, "current", &[counter("c1", *value)])
            })
        })
        .collect();
    let document_refs: Vec<_> = documents
        .iter()
        .map(|(document, _)| ObservedTemporalStep {
            document: document.clone(),
        })
        .collect();
    let target = ReplayTarget::state_clause(
        crate::QualifiedName::new(vec![Identifier::new(name).expect("fixture name")])
            .expect("qualified name"),
    );
    let mut wire =
        super::frame_replay::request(&source_bytes, domain_document, &documents, target.clone());
    wire.replay_target.over = Some(ReplayOverBinding {
        population: fixture::id("counters"),
        key: "c1".to_owned(),
    });
    let kind = match expectation {
        ObservedReplayExpectation::Formula => crate::ObservedTemporalKind::Formula,
        ObservedReplayExpectation::Undefined(cause) => {
            let locus = clause_node
                .clause()
                .formula()
                .nodes()
                .iter()
                .position(|node| matches!(node, qsl_semantics::check::FormulaNode::Holds { .. }))
                .and_then(|index| clause_node.clause().formula().node_id(index))
                .expect("the formula has a holds atom");
            crate::ObservedTemporalKind::UndefinedEvaluation { locus, cause }
        }
        ObservedReplayExpectation::UndefinedAtNode(cause, index) => {
            let locus = clause_node
                .clause()
                .formula()
                .node_id(index)
                .expect("the formula node exists");
            crate::ObservedTemporalKind::UndefinedEvaluation { locus, cause }
        }
    };
    let payload = crate::ObservedTemporalCounterexample {
        prefix: document_refs,
        loop_start,
        fairness: clause_node.clause().fairness().to_vec(),
        interval: match &kind {
            crate::ObservedTemporalKind::Formula => clause_node
                .clause()
                .operators()
                .iter()
                .find_map(|operator| match operator.interval.as_ref() {
                    Some(qsl_semantics::check::CheckedInterval::Steps(interval)) => Some(*interval),
                    _ => None,
                }),
            crate::ObservedTemporalKind::UndefinedEvaluation { .. } => None,
        },
        kind,
        trace_position: qsl_eval::temporal::TemporalPosition::new(position),
    };
    let packet = WitnessPacket::<crate::ObservedTemporalCounterexample> {
        obligation_identity: Some(wire.obligation_identity.clone()),
        occurrence_key: Some(occurrence),
        clause_node: Some(node),
        replay_target: target.into(),
        profile_selections: Some(Vec::new()),
        run_limits: Some(wire.accounting_limits),
        declared_domains: Some(Vec::new()),
        backend: Some("kani-backend-1".to_owned()),
        trace_position: Some(Some(TracePosition::new(position.to_string()))),
        source: Some(crate::EnvelopeSource::Witness(
            Witness::parse("<<<assertion|temporal_harness|temporal|>>>").unwrap(),
        )),
        family_payload: Some(payload),
    };
    let envelope = WitnessEnvelope::reconstruct(packet, crate::ReplayLimits::default())
        .expect("the typed temporal envelope reconstructs");
    wire.source = ReplaySource::ObservedTemporal(Box::new(envelope));
    ObservedReplayInput {
        wire,
        temporal_selections,
    }
}

fn rewrite_observed_payload(
    wire: ObservedReplayInput,
    rewrite: impl FnOnce(&mut crate::ObservedTemporalCounterexample),
) -> ObservedReplayInput {
    rewrite_observed_packet(wire, |packet| {
        rewrite(
            packet
                .family_payload
                .as_mut()
                .expect("the envelope carries its temporal payload"),
        );
    })
}

fn rewrite_observed_packet(
    mut wire: ObservedReplayInput,
    rewrite: impl FnOnce(&mut crate::WitnessPacket<crate::ObservedTemporalCounterexample>),
) -> ObservedReplayInput {
    let crate::ReplaySource::ObservedTemporal(envelope) = &wire.source else {
        unreachable!("the helper constructs an observed source");
    };
    let mut packet = envelope.to_packet();
    rewrite(&mut packet);
    let envelope = crate::WitnessEnvelope::<crate::ObservedTemporalCounterexample>::reconstruct(
        packet,
        crate::ReplayLimits::default(),
    )
    .expect("the changed payload remains a complete envelope");
    wire.source = crate::ReplaySource::ObservedTemporal(Box::new(envelope));
    wire
}

/// `Bounded` (`eventually[0,1] holds(c.value = 2)`) over `Finite` 0, 1, 2 is
/// a violation at position 0.
#[trace("TC-840", "FR-330-AC-1")]
#[test]
fn a_bounded_clause_over_a_finite_trace_violates_at_its_position() {
    let report = report(finite(Unit::Bounded, "Bounded", &[0, 1, 2]));
    assert_eq!(report.disposition.stage(), ClauseRunStage::Evaluate);
    assert_eq!(outcome(&report), &TemporalVerdict::Violated { at: 0 });
    assert_eq!(report.disposition.category(), Category::Violation);
    assert_eq!(exit_code(&report), 10);
}

/// FR-330-AC-5 / TC-847: the third value makes the integer modulo divisor
/// zero, and the undefined evaluation is reported as the temporal violation
/// at position 2.
#[trace("TC-847", "FR-330-AC-5")]
#[test]
fn an_undefined_evaluation_over_a_finite_trace_is_a_violation_at_its_position() {
    let report = report(finite(Unit::Undefined, "Undefined", &[0, 1, 2]));
    assert_eq!(report.disposition.stage(), ClauseRunStage::Evaluate);
    assert!(matches!(
        outcome(&report),
        TemporalVerdict::Undefined { at: 2, .. }
    ));
    assert_eq!(report.disposition.category(), Category::Violation);
    assert_eq!(exit_code(&report), 10);
}

/// `Reaches` over the same `Finite` trace is pending: inconclusive, exit 0,
/// no truth. Over a lasso with loop 0, 1 it violates at position 0; over a
/// lasso with loop 0, 1, 2 it holds.
#[trace("TC-840", "FR-330-AC-2")]
#[test]
fn an_infinite_trace_clause_reads_a_finite_prefix_as_pending_and_a_lasso_exactly() {
    let pending = report(finite(Unit::Infinite, "Reaches", &[0, 1, 2]));
    assert_eq!(outcome(&pending), &TemporalVerdict::Pending);
    assert_eq!(pending.disposition.category(), Category::Inconclusive);
    assert_eq!(exit_code(&pending), 0);
    assert_eq!(pending.disposition.truth(), None);

    let violated = report(lasso(Unit::Infinite, "Reaches", 0, &[0, 1]));
    assert_eq!(outcome(&violated), &TemporalVerdict::Violated { at: 0 });

    let holds = report(lasso(Unit::Infinite, "Reaches", 0, &[0, 1, 2]));
    assert_eq!(outcome(&holds), &TemporalVerdict::Holds);
    assert_eq!(holds.disposition.category(), Category::Success);
    assert_eq!(exit_code(&holds), 0);
    assert_eq!(holds.disposition.truth(), Some(true));
}

/// A name that selects nothing reports at `select`; a bounded clause over a
/// lasso, an `over` naming key `ghost` and a lasso with an empty loop report
/// at `admit`, `admit` and `evaluate`.
#[trace("TC-840", "FR-330-AC-3")]
#[test]
fn a_selection_that_cannot_run_reports_its_stage_and_cause() {
    let absent = report(finite(Unit::Infinite, "Absent", &[0, 1, 2]));
    assert_eq!(absent.disposition.stage(), ClauseRunStage::Select);
    assert!(
        matches!(&absent.disposition, ClauseDisposition::MissingName { name } if name == "Absent"),
        "{:?}",
        absent.disposition
    );

    let bounded_lasso = report(lasso(Unit::Bounded, "Bounded", 0, &[0, 1, 2]));
    assert_eq!(bounded_lasso.disposition.stage(), ClauseRunStage::Admit);
    let ClauseDisposition::Admit(AdmissionFailure::Refused(record)) = &bounded_lasso.disposition
    else {
        panic!(
            "expected an admission refusal, got {:?}",
            bounded_lasso.disposition
        );
    };
    assert_eq!(
        (record.catalog.code(), record.catalog.cause()),
        ("invalid_runtime_input", "invalid-value")
    );

    let ghost = report(ghost());
    assert_eq!(ghost.disposition.stage(), ClauseRunStage::Admit);
    let ClauseDisposition::Admit(AdmissionFailure::Refused(record)) = &ghost.disposition else {
        panic!("expected an admission refusal, got {:?}", ghost.disposition);
    };
    assert_eq!(
        (record.catalog.code(), record.catalog.cause()),
        ("invalid_runtime_input", "wrong-role-mapping")
    );

    let empty_loop = report(lasso(Unit::Infinite, "Reaches", 3, &[0, 1, 2]));
    assert_eq!(empty_loop.disposition.stage(), ClauseRunStage::Evaluate);
    assert_eq!(empty_loop.disposition.category(), Category::Refusal);
    let ClauseDisposition::TemporalRefusal(refusal) = &empty_loop.disposition else {
        panic!("expected a trace refusal, got {:?}", empty_loop.disposition);
    };
    assert_eq!(refusal, &TemporalTraceRefusal::EmptyLoop);
    let code = refusal.catalog_code();
    assert_eq!(
        (code.code(), code.cause()),
        ("invalid_runtime_input", "invalid-value")
    );
}

/// `Reaches` over the loop 0, 1, 2 with no work budget is incomplete, exit
/// 22; every request of AC-1 to AC-3 reports the same twice.
#[trace("TC-840", "FR-330-AC-4")]
#[test]
fn a_zero_work_budget_is_incomplete_and_a_repeated_run_reports_equal() {
    let mut starved = lasso(Unit::Infinite, "Reaches", 0, &[0, 1, 2]);
    starved.accounting = default_accounting(0);
    let starved = report(starved);
    assert!(
        matches!(outcome(&starved), TemporalVerdict::Incomplete(_)),
        "{:?}",
        starved.disposition
    );
    assert_eq!(starved.disposition.category(), Category::Incomplete);
    assert_eq!(exit_code(&starved), 22);

    let requests: [fn() -> ClauseRunRequest; 8] = [
        || finite(Unit::Bounded, "Bounded", &[0, 1, 2]),
        || finite(Unit::Infinite, "Reaches", &[0, 1, 2]),
        || lasso(Unit::Infinite, "Reaches", 0, &[0, 1]),
        || lasso(Unit::Infinite, "Reaches", 0, &[0, 1, 2]),
        || finite(Unit::Infinite, "Absent", &[0, 1, 2]),
        || lasso(Unit::Bounded, "Bounded", 0, &[0, 1, 2]),
        ghost,
        || lasso(Unit::Infinite, "Reaches", 3, &[0, 1, 2]),
    ];
    for build in requests {
        assert_eq!(
            format!("{:?}", report(build())),
            format!("{:?}", report(build()))
        );
    }
}

/// `Fair` (`always eventually holds(c.value = 2)` under `fair weak noop`)
/// over a lasso is unsupported: an observed trace carries no enabledness.
///
/// The report carries the outcome, not a catalog code: the wire cause
/// `unsupported_projection`/`missing-fairness-premise` is implied by
/// `MissingFairnessPremise` and its only writer refuses temporal dispositions
/// until the wire form is specified.
#[trace("TC-848", "FR-330-AC-6")]
#[test]
fn a_fairness_premise_over_a_supplied_trace_is_unsupported() {
    let report = report(lasso(Unit::Infinite, "Fair", 0, &[0, 1, 2]));
    assert_eq!(report.disposition.stage(), ClauseRunStage::Evaluate);
    let TemporalVerdict::MissingFairnessPremise { constraint } = outcome(&report) else {
        panic!("expected a missing premise, got {:?}", report.disposition);
    };
    assert_eq!(constraint.kind(), FairnessKind::Weak);
    assert_eq!(constraint.granularity(), FairnessGranularity::Whole);
    assert_eq!(constraint.operation().name(), "noop");
    assert_eq!(report.disposition.category(), Category::Unsupported);
    assert_eq!(exit_code(&report), 21);
}

/// FR-109-AC-13: a completed temporal verdict reports its consequent source
/// events as the typed temporal refusal, not an empty list; a pending verdict
/// completes no truth and reports them unavailable.
#[trace("TC-1269", "FR-109-AC-13")]
#[test]
fn a_temporal_run_refuses_consequent_events_typed() {
    use crate::spine::{ConsequentEventRefusal, ConsequentSourceEvents};
    let held = report(lasso(Unit::Infinite, "Reaches", 0, &[0, 1, 2]));
    assert_eq!(held.disposition.truth(), Some(true));
    assert_eq!(
        held.consequent_source_events,
        ConsequentSourceEvents::Refused(ConsequentEventRefusal::Temporal)
    );
    let violated = report(lasso(Unit::Infinite, "Reaches", 0, &[0, 1]));
    assert_eq!(
        violated.consequent_source_events,
        ConsequentSourceEvents::Refused(ConsequentEventRefusal::Temporal)
    );
    let pending = report(finite(Unit::Infinite, "Reaches", &[0, 1, 2]));
    assert_eq!(pending.disposition.truth(), None);
    assert_eq!(
        pending.consequent_source_events,
        ConsequentSourceEvents::Unavailable
    );
}

/// `Reaches` run at S6a on a lasso where it holds is evidence for that trace
/// only: it settles no truth and derives no witness, so nothing joins it to a
/// requested item. This is the report half of FR-332-AC-4; the accounting
/// record half does not exist in QSL yet, so the test is not tagged.
#[test]
fn a_clause_run_over_a_trace_settles_no_item() {
    let held = report(lasso(Unit::Infinite, "Reaches", 0, &[0, 1, 2]));
    assert_eq!(outcome(&held), &TemporalVerdict::Holds);
    assert_eq!(held.basis, SettlementBasis::Unavailable);
    assert!(held.witness.is_none());

    let violated = report(lasso(Unit::Infinite, "Reaches", 0, &[0, 1]));
    assert_eq!(violated.basis, SettlementBasis::Unavailable);
    assert!(violated.witness.is_none());
}

/// A temporal refusal's category comes from its own catalog code: a malformed
/// trace is a refusal, a profile the evaluator does not read is unsupported
/// (exit 21), and a broken invariant is an internal failure (exit 30).
#[test]
fn a_temporal_refusal_takes_its_category_from_its_catalog_code() {
    use qsl_semantics::check::TemporalProfile;
    let category = |refusal| ClauseDisposition::TemporalRefusal(refusal).category();
    assert_eq!(category(TemporalTraceRefusal::EmptyLoop), Category::Refusal);
    assert_eq!(
        category(TemporalTraceRefusal::ProfileNotEvaluated {
            profile: TemporalProfile::TimestampedEventFiniteWindow
        }),
        Category::Unsupported
    );
    assert_eq!(
        category(TemporalTraceRefusal::Fault(
            quire_semantic_value::catalog::InternalFault::new("temporal", "broken")
        )),
        Category::InternalFailure
    );
}

/// FR-501-AC-1/2 (TC-1295): timestamp distances select a sparse finite
/// window, and a closed window does not synthesize a false tail.
#[trace("TC-1295", "FR-501-AC-1", "FR-501-AC-2")]
#[test]
fn timestamped_windows_use_ticks_and_close_without_false_extension() {
    let outside = report(timestamped(&[0, 2, 5], &[0, 1, 0], 5, Some(5)));
    assert_eq!(outside.disposition.stage(), ClauseRunStage::Evaluate);
    assert_eq!(outcome(&outside), &TemporalVerdict::Holds);
    assert_eq!(outside.usage.temporal_visits, 7);

    let boundary = report(timestamped(&[0, 5], &[0, 1], 5, Some(5)));
    assert_eq!(outcome(&boundary), &TemporalVerdict::Holds);

    let sparse = report(timestamped(&[0], &[1], 1, Some(1)));
    assert_eq!(outcome(&sparse), &TemporalVerdict::Holds);

    let excluded = report(timestamped(&[0, 6], &[0, 1], 6, Some(6)));
    assert_eq!(outcome(&excluded), &TemporalVerdict::Violated { at: 0 });

    let bound_one = report(timestamped_with(
        Unit::TimestampedOne,
        &[0, 2],
        &[0, 1],
        2,
        Some(2),
    ));
    assert_eq!(outcome(&bound_one), &TemporalVerdict::Violated { at: 0 });

    let bound_two = report(timestamped_with(
        Unit::TimestampedTwo,
        &[0, 2],
        &[0, 1],
        2,
        Some(2),
    ));
    assert_eq!(outcome(&bound_two), &TemporalVerdict::Holds);

    let empty_exists = report(timestamped_with(
        Unit::TimestampedLaterEventually,
        &[0],
        &[1],
        3,
        Some(3),
    ));
    assert_eq!(outcome(&empty_exists), &TemporalVerdict::Violated { at: 0 });

    let empty_forall = report(timestamped_with(
        Unit::TimestampedLaterAlways,
        &[0],
        &[1],
        3,
        Some(3),
    ));
    assert_eq!(outcome(&empty_forall), &TemporalVerdict::Holds);
}

/// FR-501-AC-3 (TC-1295): an open future window remains pending until its
/// deadline, while a witness already inside it is decisive.
#[trace("TC-1295", "FR-501-AC-3")]
#[test]
fn timestamped_windows_pending_and_decisive_witnesses() {
    let pending = report(timestamped(&[0, 1], &[0, 0], 1, Some(1)));
    assert_eq!(
        outcome(&pending),
        &TemporalVerdict::PendingWindow { at: 0, deadline: 5 }
    );
    assert_eq!(pending.category_label(), "pending");

    let settled = report(timestamped(&[0, 1], &[0, 0], 5, Some(5)));
    assert_eq!(outcome(&settled), &TemporalVerdict::Violated { at: 0 });

    let witness = report(timestamped(&[0, 1], &[0, 1], 1, Some(1)));
    assert_eq!(outcome(&witness), &TemporalVerdict::Holds);

    let incomplete = report(timestamped(&[0, 1], &[0, 0], 5, None));
    assert_eq!(
        outcome(&incomplete),
        &TemporalVerdict::Indeterminate {
            at: 0,
            reason: crate::spine::TemporalIndeterminateReason::IncompleteInput,
        }
    );
    assert_eq!(incomplete.category_label(), "indeterminate");

    let incomplete_universal = report(timestamped_with(
        Unit::TimestampedAlwaysFive,
        &[0],
        &[1],
        5,
        None,
    ));
    assert_eq!(
        outcome(&incomplete_universal),
        &TemporalVerdict::Indeterminate {
            at: 0,
            reason: crate::spine::TemporalIndeterminateReason::IncompleteInput,
        }
    );

    let short_completeness = report(timestamped_with(
        Unit::TimestampedLaterEventually,
        &[0],
        &[1],
        3,
        Some(2),
    ));
    assert_eq!(
        outcome(&short_completeness),
        &TemporalVerdict::Indeterminate {
            at: 0,
            reason: crate::spine::TemporalIndeterminateReason::IncompleteInput,
        }
    );

    let on_each_false_then_pending = report(timestamped_with(
        Unit::TimestampedEach,
        &[0, 10],
        &[0, 0],
        10,
        Some(10),
    ));
    assert_eq!(
        outcome(&on_each_false_then_pending),
        &TemporalVerdict::Violated { at: 0 }
    );

    let on_each_pending = report(timestamped_with(
        Unit::TimestampedEach,
        &[0, 10],
        &[1, 0],
        10,
        Some(10),
    ));
    assert_eq!(
        outcome(&on_each_pending),
        &TemporalVerdict::PendingWindow {
            at: 1,
            deadline: 12,
        }
    );
}

/// FR-330-AC-9 (TC-1295): only the finite-window profile reads timestamped
/// input, and it rejects a position-only trace at admission.
#[trace("TC-1295", "FR-330-AC-9")]
#[test]
fn timestamped_profile_and_trace_shape_must_match() {
    let timestamped_clause_on_positions = request(
        Unit::Timestamped,
        |labels| ClauseRunSelection::Temporal {
            name: "Window".to_owned(),
            trace: TraceInput::Finite(labels.to_vec()),
            over: over("c1"),
            ledger: None,
        },
        &[1],
    );
    let first_report = report(timestamped_clause_on_positions);
    assert_eq!(first_report.disposition.stage(), ClauseRunStage::Admit);
    let ClauseDisposition::Admit(AdmissionFailure::Refused(record)) = &first_report.disposition
    else {
        panic!(
            "expected an admission refusal, got {:?}",
            first_report.disposition
        );
    };
    assert_eq!(
        (record.catalog.code(), record.catalog.cause()),
        ("invalid_runtime_input", "invalid-value")
    );

    let position_clause_on_timestamps = request(
        Unit::Bounded,
        |labels| ClauseRunSelection::Temporal {
            name: "Bounded".to_owned(),
            trace: TraceInput::Timestamped {
                observations: labels
                    .iter()
                    .map(|document| crate::spine::TimestampedObservation {
                        document: document.clone(),
                        stamp: 0,
                        order_key: None,
                    })
                    .collect(),
                progress: crate::spine::ProgressAssertion {
                    watermark: 0,
                    completeness: Some(0)
                        .map_or(crate::spine::ProgressCompleteness::Incomplete, |through| {
                            crate::spine::ProgressCompleteness::Complete { through }
                        }),
                    scope: crate::spine::ProgressClosure::Closed,
                    execution: crate::spine::ProgressClosure::Closed,
                },
                window: Box::new(crate::spine::TimestampedWindow {
                    binding: Some(finite_window_binding()),
                    authoritative_origin: true,
                    cover: None,
                    missing_valuations: Vec::new(),
                }),
            },
            over: over("c1"),
            ledger: None,
        },
        &[1],
    );
    let second_report = report(position_clause_on_timestamps);
    assert_eq!(second_report.disposition.stage(), ClauseRunStage::Admit);
    let ClauseDisposition::Admit(AdmissionFailure::Refused(record)) = &second_report.disposition
    else {
        panic!(
            "expected an admission refusal, got {:?}",
            second_report.disposition
        );
    };
    assert_eq!(
        (record.catalog.code(), record.catalog.cause()),
        ("invalid_runtime_input", "invalid-value")
    );
}

/// FR-501-AC-11/FR-285 (TC-1295): temporal-only result classes retain their
/// category and exit semantics through the QSL-local spine report.
#[trace("TC-1295", "FR-501-AC-11")]
#[test]
fn finite_window_report_preserves_pending_and_indeterminate_exit_classes() {
    let true_report = report(timestamped(&[0, 5], &[0, 1], 5, Some(5)));
    assert_eq!(true_report.category_label(), "success");
    assert_eq!(exit_code(&true_report), 0);

    let false_report = report(timestamped(&[0], &[0], 5, Some(5)));
    assert_eq!(false_report.category_label(), "violation");
    assert_eq!(exit_code(&false_report), 10);

    let pending = report(timestamped(&[0], &[0], 1, Some(1)));
    assert_eq!(pending.category_label(), "pending");
    assert_eq!(exit_code(&pending), 0);

    let indeterminate = report(timestamped(&[0], &[0], 5, None));
    assert_eq!(indeterminate.category_label(), "indeterminate");
    assert_eq!(exit_code(&indeterminate), 11);

    let indeterminate = TemporalVerdict::Indeterminate {
        at: 0,
        reason: crate::spine::TemporalIndeterminateReason::MissingValuation {
            position: 1,
            atom: "holds(c.value > 0)".to_owned(),
            instance: None,
            also_held_by: Vec::new(),
        },
    };
    let outcomes = [indeterminate.clone(), TemporalVerdict::Violated { at: 0 }];
    assert_eq!(crate::spine::temporal_batch_exit_code(&outcomes), 10);
    let outcomes = [indeterminate, TemporalVerdict::Holds];
    assert_eq!(crate::spine::temporal_batch_exit_code(&outcomes), 11);
}

/// FR-501-AC-9 (TC-1295): counter-tick cover comparisons preserve the
/// half-open endpoints, and an eligible witness whose cover straddles the
/// interval bound is indeterminate.
#[trace("TC-1295", "FR-501-AC-9")]
#[test]
fn timestamped_counter_cover_uses_half_open_boundary_rules() {
    use qsl_eval::temporal::{ClockCover, Containment};

    let exact = ClockCover::Exact;
    let counter0 = ClockCover::Counter { uncertainty: 0 };
    let counter1 = ClockCover::Counter { uncertainty: 1 };
    // Every vector of QSpec's tick-cover file, read at test time: the
    // expected value of each is the file's.
    // A required input: the conformance vectors are QSpec's, and a run
    // without them proves nothing.
    let qspec = std::env::var_os("QSPEC_DIR")
        .expect("QSPEC_DIR names the quire-specification checkout the tick-cover vectors live in");
    let path =
        std::path::Path::new(&qspec).join("proposals/checked-package-v2/tick-cover-vectors.json");
    let document: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(path).expect("the vectors read"))
            .expect("the vectors parse");
    let cases = document["cases"].as_array().expect("cases");
    assert_eq!(cases.len(), 15);
    for case in cases {
        let id = case["id"].as_str().expect("id");
        let cover = ClockCover::Counter {
            uncertainty: case["uncertainty"].as_u64().expect("uncertainty"),
        };
        match case["kind"].as_str().expect("kind") {
            "within_bound" => {
                let got = cover.within_bound(
                    case["bound"].as_i64().expect("bound"),
                    case["request_tick"].as_u64().expect("request"),
                    case["ack_tick"].as_u64().expect("ack"),
                );
                let want = match case["expected"].as_str().expect("expected") {
                    "true" => Containment::True,
                    "false" => Containment::False,
                    _ => Containment::Indeterminate,
                };
                assert_eq!(got, want, "{id}");
            }
            _ => assert_eq!(
                cover.definitely_before(
                    case["a_tick"].as_u64().expect("a"),
                    case["b_tick"].as_u64().expect("b"),
                ),
                case["expected"].as_bool().expect("expected"),
                "{id}"
            ),
        }
    }

    assert_eq!(exact.within_bound(0, 3000, 3000), Containment::True);

    fn request(ack_tick: u64, cover: ClockCover) -> ClauseRunRequest {
        let mut request = timestamped(&[0, ack_tick], &[1, 2], ack_tick.max(3_010), Some(3_010));
        let bytes = program_source_bytes(&mut request);
        let source = String::from_utf8(bytes.clone()).unwrap();
        *bytes = source
            .replace(
                "eventually[0,5] holds(c.value > 0)",
                "holds(c.value = 1) implies eventually[0,3000] holds(c.value = 2)",
            )
            .into_bytes();
        let trace = timestamped_trace_mut(&mut request);
        trace.window.cover = Some(spine_cover(cover));
        request
    }

    assert_eq!(
        outcome(&report(request(2_998, counter0))),
        &TemporalVerdict::Holds
    );
    assert_eq!(
        outcome(&report(request(3_000, counter0))),
        &TemporalVerdict::Indeterminate {
            at: 0,
            reason: crate::spine::TemporalIndeterminateReason::StraddlesBound,
        }
    );
    assert_eq!(
        outcome(&report(request(3_001, counter0))),
        &TemporalVerdict::Violated { at: 0 }
    );
    assert_eq!(
        outcome(&report(request(3_000, exact))),
        &TemporalVerdict::Holds
    );
    assert_eq!(
        outcome(&report(request(2_997, counter1))),
        &TemporalVerdict::Holds
    );
    assert_eq!(
        outcome(&report(request(2_998, counter1))),
        &TemporalVerdict::Indeterminate {
            at: 0,
            reason: crate::spine::TemporalIndeterminateReason::StraddlesBound,
        }
    );
    assert_eq!(
        outcome(&report(request(3_003, counter1))),
        &TemporalVerdict::Violated { at: 0 }
    );
}

/// FR-501-AC-4 (TC-1295): an unresolved atom reading is a possibility, a
/// later witness can settle around it, and differing same-stamp readings are
/// reported as conflicting valuation.
#[trace("TC-1295", "FR-501-AC-4")]
#[test]
fn timestamped_missing_and_conflicting_atom_readings_are_indeterminate() {
    fn missing(values: &[i64], watermark: u64) -> ClauseRunReport {
        let mut request = timestamped(&[0, 2, 5], values, watermark, Some(5));
        let trace = timestamped_trace_mut(&mut request);
        trace
            .window
            .missing_valuations
            .push(crate::spine::TimestampedMissingValuation {
                position: 1,
                atom: 1,
            });
        report(request)
    }

    let missing_closed = missing(&[0, 2, 0], 5);
    assert_eq!(
        outcome(&missing_closed),
        &TemporalVerdict::Indeterminate {
            at: 0,
            reason: crate::spine::TemporalIndeterminateReason::MissingValuation {
                position: 1,
                atom: "holds(c.value > 0)".to_owned(),
                instance: None,
                also_held_by: Vec::new(),
            },
        }
    );
    assert_eq!(
        outcome(&missing(&[0, 2, 0], 3)),
        &TemporalVerdict::PendingWindow { at: 0, deadline: 5 }
    );
    assert_eq!(outcome(&missing(&[0, 2, 1], 5)), &TemporalVerdict::Holds);

    // Observations that share a stamp are readings of one instant: differing
    // readings of the atom are a conflict, named at the lowest canonical
    // position of the stamp, whatever order they were listed in.
    let conflicting = report(timestamped(&[0, 2, 2, 5], &[0, 1, 0, 0], 5, Some(5)));
    let conflict = TemporalVerdict::Indeterminate {
        at: 0,
        reason: crate::spine::TemporalIndeterminateReason::ConflictingValuation {
            position: 1,
            atom: "holds(c.value > 0)".to_owned(),
        },
    };
    assert_eq!(outcome(&conflicting), &conflict);
    let reversed = report(listed_in(
        timestamped(&[0, 2, 2, 5], &[0, 1, 0, 0], 5, Some(5)),
        &[3, 2, 1, 0],
    ));
    assert_eq!(outcome(&reversed), &conflict);
    assert_eq!(
        window_report(&reversed).identity,
        window_report(&conflicting).identity
    );
    // A later witness settles around the conflict.
    let later_witness = report(timestamped(&[0, 2, 2, 5], &[0, 1, 0, 1], 5, Some(5)));
    assert_eq!(outcome(&later_witness), &TemporalVerdict::Holds);
}

/// FR-501-AC-23 (TC-1295 row 37): equal readings at one stamp are not a
/// conflict.
#[trace("TC-1295", "FR-501-AC-23")]
#[test]
fn timestamped_equal_readings_at_one_stamp_are_not_a_conflict() {
    let report = report(timestamped(&[0, 2, 2, 5], &[0, 1, 1, 0], 5, Some(5)));
    assert_eq!(outcome(&report), &TemporalVerdict::Holds);
}

/// FR-501-AC-22, AC-24, AC-25 (TC-1295 rows 36, 38, 39): the cover comes from
/// the trace input and defaults to `Exact`; the execution closure is carried
/// and does not change the verdict.
#[trace("TC-1295", "FR-501-AC-22")]
#[trace("TC-1295", "FR-501-AC-24")]
#[trace("TC-1295", "FR-501-AC-25")]
#[test]
fn timestamped_cover_and_execution_come_from_the_trace_input() {
    use crate::spine::ProgressClosure as Closure;
    use qsl_eval::temporal::ClockCover;
    let handshake = |ack: u64, watermark: u64, through: u64, cover: Option<ClockCover>| {
        let request = with_clause_body(
            timestamped(&[0, ack], &[1, 2], watermark, Some(through)),
            "holds(c.value = 1) implies eventually[0,3000] holds(c.value = 2)",
        );
        report(timestamped_changed(request, |trace| {
            trace.window.cover = cover.map(spine_cover)
        }))
    };
    // AC-22: no cover is `Exact`.
    assert_eq!(
        outcome(&handshake(2_998, 3_001, 3_001, None)),
        &TemporalVerdict::Holds
    );
    assert_eq!(
        outcome(&handshake(3_000, 3_001, 3_001, None)),
        &TemporalVerdict::Holds
    );
    assert_eq!(
        outcome(&handshake(3_001, 3_001, 3_001, None)),
        &TemporalVerdict::Violated { at: 0 }
    );
    // AC-24: a supplied cover is the one used.
    let counter = Some(ClockCover::Counter { uncertainty: 1 });
    let supplied = handshake(3_500, 3_001, 3_002, counter);
    let exact = handshake(3_500, 3_001, 3_002, Some(ClockCover::Exact));
    assert_eq!(
        outcome(&supplied),
        &TemporalVerdict::PendingWindow {
            at: 0,
            deadline: 3_002
        }
    );
    assert_eq!(outcome(&exact), &TemporalVerdict::Violated { at: 0 });
    assert_ne!(
        window_report(&supplied).identity,
        window_report(&exact).identity
    );
    // AC-25: the execution closure is carried and changes no verdict.
    let closed = report(timestamped(&[0, 2, 5], &[0, 1, 0], 5, Some(5)));
    let open = report(timestamped_changed(
        timestamped(&[0, 2, 5], &[0, 1, 0], 5, Some(5)),
        |trace| trace.progress.execution = crate::spine::ProgressClosure::Open,
    ));
    assert_eq!(outcome(&open), outcome(&closed));
    assert_eq!(window_report(&closed).premises.execution, Closure::Closed);
    assert_eq!(window_report(&open).premises.execution, Closure::Open);
}

/// FR-501-AC-12 (TC-1295): a closed `[0,5]` existential on stamps 0,2,5
/// charges one root visit, three atom reads and three scan positions.
#[trace("TC-1295", "FR-501-AC-12")]
#[test]
fn timestamped_window_charges_exact_visits_and_stops_on_the_denied_visit() {
    let full = report(timestamped(&[0, 2, 5], &[0, 1, 0], 5, Some(5)));
    assert_eq!(outcome(&full), &TemporalVerdict::Holds);
    assert_eq!(full.usage.temporal_visits, 7);
    let work = full
        .usage
        .evaluation_consumed
        .iter()
        .find(|(kind, _)| *kind == quire_exact::LimitKind::WorkUnits)
        .map(|(_, consumed)| *consumed)
        .unwrap();

    // Every ceiling below the run's work denies a charge and returns no truth
    // value; a ceiling of zero denies the first, a `temporal.visit`.
    for ceiling in 0..work {
        let mut starved = timestamped(&[0, 2, 5], &[0, 1, 0], 5, Some(5));
        starved.accounting.work_units = ceiling;
        let starved = report(starved);
        let TemporalVerdict::Incomplete(incomplete) = outcome(&starved) else {
            panic!("ceiling {ceiling}: {:?}", starved.disposition);
        };
        assert_eq!(starved.disposition.truth(), None);
        if ceiling == 0 {
            assert_eq!(
                incomplete.charge_point,
                quire_exact::ChargePoint::TemporalVisit
            );
        }
    }

    let mut seven = timestamped(&[0, 2, 5], &[0, 1, 0], 5, Some(5));
    seven.accounting.work_units = work;
    let seven = report(seven);
    assert_eq!(outcome(&seven), &TemporalVerdict::Holds);
    assert_eq!(seven.usage.temporal_visits, 7);

    let full_siblings = report(timestamped_with(
        Unit::TimestampedEach,
        &[0, 10],
        &[0, 0],
        12,
        Some(12),
    ));
    assert_eq!(
        outcome(&full_siblings),
        &TemporalVerdict::Violated { at: 0 }
    );
    let total_work = full_siblings
        .usage
        .evaluation_consumed
        .iter()
        .find(|(kind, _)| *kind == quire_exact::LimitKind::WorkUnits)
        .map(|(_, consumed)| *consumed)
        .unwrap();
    let mut preserved_first = false;
    for ceiling in 0..total_work {
        let mut request = timestamped_with(Unit::TimestampedEach, &[0, 10], &[0, 0], 12, Some(12));
        request.accounting.work_units = ceiling;
        let partial = report(request);
        if outcome(&partial) == &(TemporalVerdict::Violated { at: 0 })
            && partial.usage.temporal_visits < full_siblings.usage.temporal_visits
        {
            preserved_first = true;
            break;
        }
    }
    assert!(
        preserved_first,
        "a settled first activation survives a later denied charge"
    );
}

/// FR-501-AC-7/18 (TC-1295): order-sensitive operators require comparable
/// authority-scoped keys for equal stamps; insertion order does not decide
/// an ordered reading.
#[trace("TC-1295", "FR-501-AC-7")]
#[test]
fn order_sensitive_equal_stamps_require_one_authority_and_sort_by_key() {
    fn until_request(values: &[i64]) -> ClauseRunRequest {
        let mut request = timestamped(&[0, 1, 1], values, 1, Some(1));
        let bytes = program_source_bytes(&mut request);
        let source = String::from_utf8(bytes.clone()).unwrap();
        *bytes = source
            .replace(
                "eventually[0,5] holds(c.value > 0)",
                "holds(c.value > 0) until[0,1] holds(c.value = 2)",
            )
            .into_bytes();
        request
    }
    fn set_keys(request: &mut ClauseRunRequest, keys: &[(&str, i64)]) {
        let trace = timestamped_trace_mut(request);
        for (position, (authority, key)) in trace.positions.iter_mut().zip(keys) {
            position.order_key = Some(crate::spine::TimestampedOrderKey {
                authority: (*authority).to_owned(),
                key: *key,
            });
        }
    }

    let refused = report(until_request(&[1, 0, 2]));
    assert!(matches!(
        refused.disposition,
        ClauseDisposition::TemporalRefusal(TemporalTraceRefusal::SameStampOrderUnavailable {
            position: 2,
        })
    ));

    let mut ordered = until_request(&[1, 0, 2]);
    set_keys(&mut ordered, &[("ledger", 0), ("ledger", 1), ("ledger", 2)]);
    let ordered = report(ordered);

    let mut reversed = until_request(&[1, 0, 2]);
    set_keys(
        &mut reversed,
        &[("ledger", 0), ("ledger", 1), ("ledger", 2)],
    );
    let trace = timestamped_trace_mut(&mut reversed);
    trace.positions.swap(1, 2);
    let reversed = report(reversed);
    assert_eq!(outcome(&ordered), outcome(&reversed));

    let mut incomparable = until_request(&[1, 0, 2]);
    set_keys(
        &mut incomparable,
        &[("ledger-a", 0), ("ledger-a", 1), ("ledger-b", 2)],
    );
    assert!(matches!(
        report(incomparable).disposition,
        ClauseDisposition::TemporalRefusal(TemporalTraceRefusal::SameStampOrderUnavailable {
            position: 2,
        })
    ));
}

/// FR-501-AC-8 (TC-1295): a past window before a history cutoff is
/// indeterminate only when the represented history does not decide it.
#[trace("TC-1295", "FR-501-AC-8")]
#[test]
fn timestamped_past_windows_respect_history_cutoff() {
    fn once(values: &[i64], authoritative_origin: bool) -> ClauseRunReport {
        let mut request = timestamped_with(Unit::TimestampedOnce, &[3, 4], values, 4, Some(4));
        let trace = timestamped_trace_mut(&mut request);
        trace.window.authoritative_origin = authoritative_origin;
        report(request)
    }

    let cutoff = once(&[0, 0], false);
    assert_eq!(
        outcome(&cutoff),
        &TemporalVerdict::Indeterminate {
            at: 0,
            reason: crate::spine::TemporalIndeterminateReason::MissingHistory,
        }
    );
    assert_eq!(
        outcome(&once(&[0, 0], true)),
        &TemporalVerdict::Violated { at: 0 }
    );
    assert_eq!(outcome(&once(&[1, 0], false)), &TemporalVerdict::Holds);
}

/// FR-501-AC-10 (TC-1295): guarded finite-window activations run only where
/// their guard is true, with the same first-false ordering as unguarded runs.
#[trace("TC-1295", "FR-501-AC-10")]
#[test]
fn timestamped_guarded_on_each_activations_are_evaluated() {
    fn guarded(values: &[i64]) -> ClauseRunReport {
        let mut request = timestamped_with(Unit::TimestampedEach, &[0, 10], values, 12, Some(12));
        let bytes = program_source_bytes(&mut request);
        let source = String::from_utf8(bytes.clone()).unwrap();
        *bytes = source
            .replace(
                "on each (q: Counters::Counter) { eventually[0,2] holds(c.value > 0) }",
                "on each (q: Counters::Counter) when (q.value = 1) { eventually[0,2] holds(c.value > 1) }",
            )
            .into_bytes();
        report(request)
    }

    assert_eq!(outcome(&guarded(&[0, 0])), &TemporalVerdict::Holds);
    assert_eq!(
        outcome(&guarded(&[1, 0])),
        &TemporalVerdict::Violated { at: 0 }
    );
}

/// FR-337 and FR-330: an `on each ... when (g)` clause over a finite trace
/// evaluates its guard on the observed trace (no evaluator fault): over the
/// values 0, 1, 2 the guard holds at position 1 alone, where the property
/// holds; over 0, 1, 1 the property is violated at that position.
#[trace("TC-840", "FR-330-AC-1")]
#[test]
fn a_guarded_on_each_clause_over_a_finite_trace_is_evaluated_at_its_guard() {
    let report = report(finite(Unit::Guarded, "Guarded", &[0, 1, 2]));
    assert_eq!(report.disposition.stage(), ClauseRunStage::Evaluate);
    assert!(
        !matches!(outcome(&report), TemporalVerdict::Violated { .. }),
        "{:?}",
        report.disposition
    );
    let report = self::report(finite(Unit::Guarded, "Guarded", &[0, 1, 1]));
    assert_eq!(outcome(&report), &TemporalVerdict::Violated { at: 1 });
}

/// The program source bytes of `request`, which these tests rewrite.
fn program_source_bytes(request: &mut ClauseRunRequest) -> &mut Vec<u8> {
    match &mut request.source {
        ClauseRunSource::Program { bytes, .. } => bytes,
        #[cfg(feature = "quire-extraction")]
        ClauseRunSource::Extracted(_) => panic!("a program source"),
    }
}

/// The spine's form of an evaluator cover.
fn spine_cover(cover: qsl_eval::temporal::ClockCover) -> crate::spine::TimestampedCover {
    match cover {
        qsl_eval::temporal::ClockCover::Exact => crate::spine::TimestampedCover::Exact,
        qsl_eval::temporal::ClockCover::Counter { uncertainty } => {
            crate::spine::TimestampedCover::Counter { uncertainty }
        }
    }
}

/// The parts of a request's timestamped trace a test changes.
struct TsMut<'a> {
    positions: &'a mut Vec<crate::spine::TimestampedObservation>,
    progress: &'a mut crate::spine::ProgressAssertion,
    window: &'a mut crate::spine::TimestampedWindow,
}

/// The timestamped trace of `request`, by part.
fn timestamped_trace_mut(request: &mut ClauseRunRequest) -> TsMut<'_> {
    let ClauseRunSelection::Temporal {
        trace:
            TraceInput::Timestamped {
                observations,
                progress,
                window,
            },
        ..
    } = &mut request.selection
    else {
        panic!("a timestamped selection");
    };
    TsMut {
        positions: observations,
        progress,
        window,
    }
}

fn closure(closed: bool) -> crate::spine::ProgressClosure {
    if closed {
        crate::spine::ProgressClosure::Closed
    } else {
        crate::spine::ProgressClosure::Open
    }
}

/// `request` with its timestamped trace changed by `change`.
fn timestamped_changed(
    mut request: ClauseRunRequest,
    change: impl FnOnce(&mut TsMut<'_>),
) -> ClauseRunRequest {
    let mut trace = timestamped_trace_mut(&mut request);
    change(&mut trace);
    request
}

fn window_report(report: &ClauseRunReport) -> &crate::spine::FiniteWindowReport {
    report
        .finite_window
        .as_ref()
        .expect("a finite-window run carries its result report")
}

/// FR-501-AC-13 (TC-1295 row 27): the counter margin `2u` holds an
/// activation pending until `t0 + H + 2u`; the exact cover has no margin.
#[trace("TC-1295", "FR-501-AC-13")]
#[test]
fn timestamped_counter_margin_delays_establishment() {
    fn handshake(
        watermark: u64,
        through: u64,
        cover: qsl_eval::temporal::ClockCover,
    ) -> ClauseRunReport {
        let mut request = timestamped(&[0], &[1], watermark, Some(through));
        let bytes = program_source_bytes(&mut request);
        let source = String::from_utf8(bytes.clone()).unwrap();
        *bytes = source
            .replace(
                "eventually[0,5] holds(c.value > 0)",
                "holds(c.value = 1) implies eventually[0,3000] holds(c.value = 2)",
            )
            .into_bytes();
        report(timestamped_changed(request, |trace| {
            trace.window.cover = Some(spine_cover(cover))
        }))
    }
    let counter = qsl_eval::temporal::ClockCover::Counter { uncertainty: 1 };
    assert_eq!(
        outcome(&handshake(3_001, 3_002, counter)),
        &TemporalVerdict::PendingWindow {
            at: 0,
            deadline: 3_002
        }
    );
    assert_eq!(
        outcome(&handshake(3_002, 3_002, counter)),
        &TemporalVerdict::Violated { at: 0 }
    );
    assert_eq!(
        outcome(&handshake(
            3_000,
            3_000,
            qsl_eval::temporal::ClockCover::Exact
        )),
        &TemporalVerdict::Violated { at: 0 }
    );
}

/// FR-501-AC-14 (TC-1295 row 28): the basis, verdict and decision support
/// are separate fields of the report.
#[trace("TC-1295", "FR-501-AC-14")]
#[test]
fn timestamped_report_gives_basis_and_decision_support() {
    use qsl_eval::temporal::Basis;
    let always = |scope_closed: bool| {
        report(timestamped_changed(
            timestamped_with(
                Unit::TimestampedAlwaysFive,
                &[0, 2, 5],
                &[1, 0, 1],
                5,
                Some(5),
            ),
            |trace| trace.progress.scope = closure(scope_closed),
        ))
    };
    let closed = always(true);
    assert_eq!(outcome(&closed), &TemporalVerdict::Violated { at: 0 });
    assert_eq!(window_report(&closed).basis, SettlementBasis::ClosedScope);
    let support: Vec<u64> = window_report(&closed)
        .support
        .iter()
        .map(|pair| pair.position)
        .collect();
    assert_eq!(support, vec![1]);
    let open = always(false);
    assert_eq!(outcome(&open), &TemporalVerdict::Violated { at: 0 });
    assert_eq!(
        window_report(&open).basis,
        SettlementBasis::DecisiveCounterexample
    );
    assert_eq!(window_report(&open).support, window_report(&closed).support);

    let witnessed = report(timestamped(&[0, 2, 5], &[0, 0, 1], 5, Some(5)));
    assert_eq!(outcome(&witnessed), &TemporalVerdict::Holds);
    assert_eq!(
        window_report(&witnessed).basis,
        SettlementBasis::ClosedScope
    );
    let support: Vec<u64> = window_report(&witnessed)
        .support
        .iter()
        .map(|pair| pair.position)
        .collect();
    assert_eq!(support, vec![2]);

    let pending = report(timestamped(&[0, 1], &[0, 0], 1, Some(1)));
    assert_eq!(window_report(&pending).basis, SettlementBasis::Unsettled);
    let incomplete = report(timestamped(&[0, 1], &[0, 0], 5, None));
    assert_eq!(
        window_report(&incomplete).basis,
        SettlementBasis::Unavailable
    );
}

/// FR-501-AC-15 (TC-1295 row 29): scope closure, surrounding-execution
/// closure, assessment disposition and input completeness stay four
/// independent fields.
#[trace("TC-1295", "FR-501-AC-15")]
#[test]
fn timestamped_report_keeps_four_independent_dimensions() {
    use crate::spine::FiniteWindowAssessment as Assessment;
    use crate::spine::{ProgressClosure as Closure, ProgressCompleteness as Completeness};
    let base = || {
        timestamped_with(
            Unit::TimestampedAlwaysFive,
            &[0, 2, 5],
            &[1, 0, 1],
            5,
            Some(5),
        )
    };
    let closed = report(base());
    let open = report(timestamped_changed(base(), |trace| {
        trace.progress.scope = crate::spine::ProgressClosure::Open
    }));
    assert_eq!(outcome(&open), outcome(&closed));
    assert_eq!(window_report(&open).support, window_report(&closed).support);
    assert_ne!(window_report(&open).basis, window_report(&closed).basis);
    assert_eq!(window_report(&open).premises.scope, Closure::Open);
    assert_eq!(
        window_report(&open).premises.execution,
        window_report(&closed).premises.execution
    );
    assert_eq!(
        window_report(&open).premises.completeness,
        window_report(&closed).premises.completeness
    );

    let moved = report(timestamped_changed(base(), |trace| {
        trace.progress.execution = crate::spine::ProgressClosure::Open;
    }));
    assert_eq!(window_report(&moved).premises.execution, Closure::Open);
    assert_eq!(window_report(&moved).premises.scope, Closure::Closed);
    assert_eq!(outcome(&moved), outcome(&closed));
    assert_eq!(window_report(&moved).basis, window_report(&closed).basis);

    assert_eq!(
        window_report(&closed).premises.assessment,
        Assessment::Completed
    );
    let mut starved = base();
    starved.accounting.work_units = 1;
    let failed = report(starved);
    assert!(matches!(outcome(&failed), TemporalVerdict::Incomplete(_)));
    assert_eq!(
        window_report(&failed).premises.assessment,
        Assessment::Failed
    );

    let incomplete = report(timestamped_changed(
        timestamped_with(Unit::TimestampedAlwaysFive, &[0, 2, 5], &[1, 1, 1], 5, None),
        |_| {},
    ));
    assert_eq!(
        outcome(&incomplete),
        &TemporalVerdict::Indeterminate {
            at: 0,
            reason: crate::spine::TemporalIndeterminateReason::IncompleteInput,
        }
    );
    assert_eq!(
        window_report(&incomplete).premises.completeness,
        Completeness::Incomplete
    );
    assert_eq!(window_report(&incomplete).premises.scope, Closure::Closed);
}

/// FR-501-AC-16 (TC-1295 row 30): one result identity per input; each input
/// changed alone gives a distinct identity.
#[trace("TC-1295", "FR-501-AC-16")]
#[test]
fn timestamped_result_identity_follows_every_input() {
    let base = || {
        timestamped_with(
            Unit::TimestampedAlwaysFive,
            &[0, 2, 5],
            &[1, 0, 1],
            5,
            Some(5),
        )
    };
    let identity = |request: ClauseRunRequest| window_report(&report(request)).identity.clone();
    let original = identity(base());
    assert_eq!(original, identity(base()));
    let mut seen = vec![original.clone()];
    let mut record = |name: &str, request: ClauseRunRequest| {
        let changed = identity(request);
        assert!(
            !seen.contains(&changed),
            "{name} did not change the identity"
        );
        seen.push(changed);
    };
    record(
        "cover",
        timestamped_changed(base(), |trace| {
            trace.window.cover = Some(crate::spine::TimestampedCover::Counter { uncertainty: 0 });
        }),
    );
    record(
        "stamp",
        timestamped_with(
            Unit::TimestampedAlwaysFive,
            &[0, 3, 5],
            &[1, 0, 1],
            5,
            Some(5),
        ),
    );
    record(
        "valuation",
        timestamped_with(
            Unit::TimestampedAlwaysFive,
            &[0, 2, 5],
            &[1, 1, 1],
            5,
            Some(5),
        ),
    );
    record(
        "order key",
        timestamped_changed(base(), |trace| {
            trace.positions[2].order_key = Some(crate::spine::TimestampedOrderKey {
                authority: "A".to_owned(),
                key: 1,
            });
        }),
    );
    record(
        "watermark",
        timestamped_with(
            Unit::TimestampedAlwaysFive,
            &[0, 2, 5],
            &[1, 0, 1],
            6,
            Some(5),
        ),
    );
    record(
        "through",
        timestamped_with(
            Unit::TimestampedAlwaysFive,
            &[0, 2, 5],
            &[1, 0, 1],
            5,
            Some(6),
        ),
    );
    record(
        "lower boundary",
        timestamped_changed(base(), |trace| trace.window.authoritative_origin = false),
    );
    let mut ceiling = base();
    ceiling.accounting.work_units = 1_000;
    record("ceiling", ceiling);
    assert_eq!(original, identity(base()));
}

/// FR-501-AC-17 (TC-1295 row 31): completeness may run ahead of the
/// watermark; an empty trace is refused at stage `admit`,
/// `invalid_runtime_input`/`invalid-value`; a listing whose stamps decrease is
/// read in canonical order, not refused.
#[trace("TC-1295", "FR-501-AC-17")]
#[test]
fn timestamped_completeness_may_run_ahead_and_an_empty_trace_is_refused() {
    let ahead = report(timestamped(&[0, 2, 5], &[0, 0, 0], 5, Some(9)));
    assert_eq!(outcome(&ahead), &TemporalVerdict::Violated { at: 0 });
    let short = report(timestamped(&[0, 2, 5], &[0, 0, 0], 3, Some(9)));
    assert_eq!(
        outcome(&short),
        &TemporalVerdict::PendingWindow { at: 0, deadline: 5 }
    );
    let refused = report(timestamped(&[], &[], 5, Some(5)));
    assert_eq!(refused.disposition.stage(), ClauseRunStage::Admit);
    let ClauseDisposition::Admit(AdmissionFailure::Refused(record)) = &refused.disposition else {
        panic!(
            "expected an admission refusal, got {:?}",
            refused.disposition
        );
    };
    assert_eq!(record.catalog.code(), "invalid_runtime_input");
    assert_eq!(record.catalog.cause(), "invalid-value");
    assert_eq!(refused.usage.temporal_visits, 0);
    let decreasing = report(timestamped(&[0, 5, 2], &[0, 0, 0], 5, Some(5)));
    assert_eq!(outcome(&decreasing), &TemporalVerdict::Violated { at: 0 });
}

/// `request` with the positions of its timestamped trace listed in `order`.
fn listed_in(request: ClauseRunRequest, order: &[usize]) -> ClauseRunRequest {
    timestamped_changed(request, |trace| {
        let listed = trace.positions.clone();
        *trace.positions = order.iter().map(|index| listed[*index].clone()).collect();
    })
}

/// FR-501-AC-18 (TC-1295 row 32): the outcome, decision support and
/// `result_identity` do not depend on the listing order.
#[trace("TC-1295", "FR-501-AC-18")]
#[test]
fn timestamped_listing_order_changes_neither_outcome_support_nor_identity() {
    let base = || {
        timestamped_with(
            Unit::TimestampedAlwaysFive,
            &[0, 2, 5],
            &[1, 0, 1],
            5,
            Some(5),
        )
    };
    let canonical = report(base());
    for order in [[2, 0, 1], [1, 2, 0], [2, 1, 0], [1, 0, 2]] {
        let listed = report(listed_in(base(), &order));
        assert_eq!(outcome(&listed), outcome(&canonical), "{order:?}");
        assert_eq!(
            window_report(&listed),
            window_report(&canonical),
            "{order:?}"
        );
    }
}

/// FR-501-AC-19 (TC-1295 row 33): positions and support are indices in
/// canonical order, not in the listed order.
#[trace("TC-1295", "FR-501-AC-19")]
#[test]
fn timestamped_positions_are_canonical_indices() {
    let listed = report(listed_in(
        timestamped_with(
            Unit::TimestampedAlwaysFive,
            &[0, 2, 5],
            &[1, 0, 1],
            5,
            Some(5),
        ),
        &[2, 0, 1],
    ));
    assert_eq!(outcome(&listed), &TemporalVerdict::Violated { at: 0 });
    let support: Vec<u64> = window_report(&listed)
        .support
        .iter()
        .map(|pair| pair.position)
        .collect();
    assert_eq!(support, vec![1]);
}

/// FR-501-AC-20 (TC-1295 row 34): the `until` of AC-7 over keys from one
/// authority gives one outcome for the listed order and for its reverse.
#[trace("TC-1295", "FR-501-AC-20")]
#[test]
fn timestamped_until_over_one_authority_is_the_same_reversed() {
    let until = || {
        let mut request = timestamped_with(Unit::Timestamped, &[0, 1, 1], &[1, 0, 2], 1, Some(1));
        let bytes = program_source_bytes(&mut request);
        let source = String::from_utf8(bytes.clone()).unwrap();
        *bytes = source
            .replace(
                "eventually[0,5] holds(c.value > 0)",
                "holds(c.value > 0) until[0,1] holds(c.value = 2)",
            )
            .into_bytes();
        timestamped_changed(request, |trace| {
            for (index, position) in trace.positions.iter_mut().enumerate() {
                position.order_key = Some(crate::spine::TimestampedOrderKey {
                    authority: "ledger".to_owned(),
                    key: i64::try_from(index).unwrap(),
                });
            }
        })
    };
    let forward = report(until());
    let backward = report(listed_in(until(), &[2, 1, 0]));
    assert_eq!(outcome(&forward), outcome(&backward));
    assert_eq!(window_report(&forward), window_report(&backward));
}

/// FR-501-AC-21 (TC-1295 row 35): four observations at one stamp, keyed A1,
/// A2 (authority A), B1 (authority B) and none, listed in three orders, give
/// one canonical order and one `result_identity`.
#[trace("TC-1295", "FR-501-AC-21")]
#[test]
fn timestamped_mixed_authorities_have_one_canonical_order() {
    // Equal readings; the B1 observation's valuation is absent, so the
    // position the reason names is B1's index in canonical order: A1, A2, B1,
    // the keyless one puts it at 2 under every listing.
    let keyed = || {
        timestamped_changed(
            timestamped_with(Unit::Timestamped, &[0, 0, 0, 0], &[0, 0, 0, 0], 5, Some(5)),
            |trace| {
                let keys = [Some(("A", 1)), Some(("A", 2)), Some(("B", 1)), None];
                for (position, key) in trace.positions.iter_mut().zip(keys) {
                    position.order_key =
                        key.map(|(authority, key)| crate::spine::TimestampedOrderKey {
                            authority: authority.to_owned(),
                            key,
                        });
                }
            },
        )
    };
    let run = |order: &[usize]| {
        report(timestamped_changed(listed_in(keyed(), order), |trace| {
            let b1 = trace
                .positions
                .iter()
                .position(|position| {
                    position
                        .order_key
                        .as_ref()
                        .is_some_and(|key| key.authority == "B")
                })
                .unwrap();
            trace
                .window
                .missing_valuations
                .push(crate::spine::TimestampedMissingValuation {
                    position: u64::try_from(b1).unwrap(),
                    atom: 1,
                });
        }))
    };
    let canonical = run(&[0, 1, 2, 3]);
    assert!(
        matches!(
            outcome(&canonical),
            TemporalVerdict::Indeterminate {
                at: 0,
                reason: crate::spine::TemporalIndeterminateReason::MissingValuation {
                    position: 2,
                    ..
                },
            }
        ),
        "{:?}",
        canonical.disposition
    );
    for order in [[3, 2, 1, 0], [1, 3, 0, 2], [2, 0, 3, 1]] {
        let listed = run(&order);
        assert_eq!(outcome(&listed), outcome(&canonical), "{order:?}");
        assert_eq!(
            window_report(&listed),
            window_report(&canonical),
            "{order:?}"
        );
    }
}

/// FR-330-AC-11 (TC-1317): a timestamped request with no binding, or whose
/// binding asserts another profile or clock name, is refused at stage `admit`,
/// `invalid_runtime_input`/`invalid-value`, naming the first differing
/// dimension, before any position is visited.
#[trace("TC-1295", "FR-330-AC-9")]
#[trace("TC-1317", "FR-330-AC-11")]
#[test]
fn timestamped_binding_is_required_and_authenticated() {
    let refusal = |request: ClauseRunRequest| {
        let refused = report(request);
        assert_eq!(refused.disposition.stage(), ClauseRunStage::Admit);
        assert_eq!(refused.usage.temporal_visits, 0);
        assert_eq!(refused.usage.admission_consumed.objects, 0);
        let ClauseDisposition::Admit(AdmissionFailure::Refused(record)) = refused.disposition
        else {
            panic!("expected an admission refusal");
        };
        assert_eq!(record.catalog.code(), "invalid_runtime_input");
        assert_eq!(record.catalog.cause(), "invalid-value");
        record.fields
    };
    let base = || timestamped(&[0, 2, 5], &[0, 1, 0], 5, Some(5));
    assert!(refusal(timestamped_changed(base(), |trace| trace.window.binding = None)).is_empty());
    let clock = refusal(timestamped_changed(base(), |trace| {
        trace.window.binding.as_mut().unwrap().clock_name = "us".to_owned();
    }));
    assert_eq!(
        clock.get("mismatched_dimension").map(String::as_str),
        Some("Clock")
    );
    assert_eq!(clock.get("asserted").map(String::as_str), Some("us"));
    assert_eq!(clock.get("expected").map(String::as_str), Some("ms"));
    // Both differ: the first dimension in order, `Profile`, is reported, and
    // it is reported even for an empty trace.
    let profile = refusal(timestamped_changed(
        timestamped(&[], &[], 5, Some(5)),
        |trace| {
            let binding = trace.window.binding.as_mut().unwrap();
            binding.clock_name = "us".to_owned();
            binding.profile = "quire.temporal.infinite-trace/v1".to_owned();
        },
    ));
    assert_eq!(
        profile.get("mismatched_dimension").map(String::as_str),
        Some("Profile")
    );
    // An agreeing binding runs.
    assert_eq!(outcome(&report(base())), &TemporalVerdict::Holds);
}

/// `request` with its `Window` clause body replaced by `body`.
fn with_clause_body(mut request: ClauseRunRequest, body: &str) -> ClauseRunRequest {
    let bytes = program_source_bytes(&mut request);
    let source = String::from_utf8(bytes.clone()).unwrap();
    let source = [
        "eventually[0,5] holds(c.value > 0)",
        "eventually[0,2] holds(c.value > 0)",
        "once[0,4] holds(c.value > 0)",
    ]
    .iter()
    .fold(source, |source, pattern| source.replace(pattern, body));
    *bytes = source.into_bytes();
    request
}

/// FR-501 "Verdicts" (TC-1295 rows 8 to 10, 19): through `run_clause`, a
/// valuation absent at one position settles a clause that does not depend on
/// it (`p or not p`), and a history cutoff is `MissingHistory` only when the
/// history present does not already decide.
#[trace("TC-1295", "FR-501-AC-4")]
#[trace("TC-1295", "FR-501-AC-8")]
#[test]
fn timestamped_possibility_lift_settles_independent_of_an_absent_valuation() {
    let tautology = with_clause_body(
        timestamped(&[0], &[0], 0, Some(0)),
        "holds(c.value > 0) or not holds(c.value > 0)",
    );
    let tautology = timestamped_changed(tautology, |trace| {
        // The atom's valuation is absent at position 0: both nodes of the one
        // atom lose it.
        trace
            .window
            .missing_valuations
            .push(crate::spine::TimestampedMissingValuation {
                position: 0,
                atom: 1,
            });
    });
    assert_eq!(outcome(&report(tautology)), &TemporalVerdict::Holds);

    let historically = |values: &[i64]| {
        timestamped_changed(
            with_clause_body(
                timestamped_with(Unit::TimestampedOnce, &[3, 4], values, 6, Some(6)),
                "historically[0,4] holds(c.value > 0)",
            ),
            |trace| trace.window.authoritative_origin = false,
        )
    };
    assert_eq!(
        outcome(&report(historically(&[0, 1]))),
        &TemporalVerdict::Violated { at: 0 }
    );
    assert_eq!(
        outcome(&report(historically(&[1, 1]))),
        &TemporalVerdict::Indeterminate {
            at: 0,
            reason: crate::spine::TemporalIndeterminateReason::MissingHistory,
        }
    );
}

/// FR-285-AC-10 and AC-12 (TC-769), FR-501-AC-11 (TC-1295 row 24): one
/// interim mapping gives a finite-window outcome's label and exit, and the
/// multi-item ranking reads the same one: pending `pending`/0, indeterminate
/// `indeterminate`/11, true `success`/0, false `violation`/10, work stop
/// `incomplete`/22.
#[trace("TC-769", "FR-285-AC-10")]
#[trace("TC-769", "FR-285-AC-12")]
#[trace("TC-1295", "FR-501-AC-11")]
#[test]
fn timestamped_category_label_and_exit_come_from_one_mapping() {
    let mut starved = timestamped(&[0, 2], &[0, 0], 5, Some(5));
    starved.accounting.work_units = 0;
    let runs = [
        (
            report(timestamped(&[0, 5], &[0, 1], 5, Some(5))),
            "success",
            0,
        ),
        (report(timestamped(&[0], &[0], 5, Some(5))), "violation", 10),
        (report(timestamped(&[0], &[0], 1, Some(1))), "pending", 0),
        (
            report(timestamped(&[0], &[0], 5, None)),
            "indeterminate",
            11,
        ),
        (report(starved), "incomplete", 22),
    ];
    for (run, label, exit) in &runs {
        assert_eq!(run.category_label(), *label);
        assert_eq!(run.disposition.exit_code(), *exit);
        // The ranking of the one outcome is the run path's pair.
        let ClauseDisposition::Temporal(verdict) = &run.disposition else {
            panic!("expected a temporal verdict, got {:?}", run.disposition);
        };
        assert_eq!(
            crate::spine::temporal_batch_outcome([verdict]),
            Some((*label, *exit))
        );
    }
    // FR-285's order: {violation, indeterminate} exits 10, {success,
    // indeterminate} 11, {indeterminate, incomplete} 22.
    let verdicts: Vec<&TemporalVerdict> = runs
        .iter()
        .map(|(run, _, _)| match &run.disposition {
            ClauseDisposition::Temporal(verdict) => verdict,
            other => panic!("{other:?}"),
        })
        .collect();
    let pick = |indices: &[usize]| {
        crate::spine::temporal_batch_exit_code(indices.iter().map(|index| verdicts[*index]))
    };
    assert_eq!(pick(&[1, 3]), 10);
    assert_eq!(pick(&[0, 3]), 11);
    assert_eq!(pick(&[3, 4]), 22);
    assert_eq!(pick(&[0, 2]), 0);
}

/// FR-501-AC-12 (TC-1295 row 26): running a request twice gives equal outcomes
/// and one result report.
#[trace("TC-1295", "FR-501-AC-12")]
#[test]
fn timestamped_requests_run_twice_give_equal_reports() {
    let requests: [fn() -> ClauseRunRequest; 4] = [
        || timestamped(&[0, 2, 5], &[0, 1, 0], 5, Some(5)),
        || timestamped(&[0, 1], &[0, 0], 1, Some(1)),
        || timestamped(&[0], &[0], 5, None),
        || timestamped_with(Unit::TimestampedEach, &[0, 10], &[0, 0], 12, Some(12)),
    ];
    for make in requests {
        let first = report(make());
        let second = report(make());
        assert_eq!(outcome(&first), outcome(&second));
        assert_eq!(first.finite_window, second.finite_window);
        assert_eq!(first.usage.temporal_visits, second.usage.temporal_visits);
    }
}

/// FR-501 "Established window": the watermark settles an activation at exactly
/// its deadline and not one tick before.
#[trace("TC-1295", "FR-501-AC-3")]
#[test]
fn timestamped_watermark_at_the_deadline_settles() {
    let at_deadline = report(timestamped(&[0, 1], &[0, 0], 5, Some(5)));
    assert_eq!(outcome(&at_deadline), &TemporalVerdict::Violated { at: 0 });
    let before = report(timestamped(&[0, 1], &[0, 0], 4, Some(5)));
    assert_eq!(
        outcome(&before),
        &TemporalVerdict::PendingWindow { at: 0, deadline: 5 }
    );
    let incomplete = report(timestamped(&[0, 1], &[0, 0], 5, Some(4)));
    assert_eq!(
        outcome(&incomplete),
        &TemporalVerdict::Indeterminate {
            at: 0,
            reason: crate::spine::TemporalIndeterminateReason::IncompleteInput,
        }
    );
}

fn with_progress(
    mut request: ClauseRunRequest,
    watermark: u64,
    completeness: crate::spine::ProgressCompleteness,
    carried: &crate::spine::ProgressLedger,
) -> ClauseRunRequest {
    let ClauseRunSelection::Temporal { trace, ledger, .. } = &mut request.selection else {
        panic!("a temporal selection");
    };
    let documents = match trace {
        TraceInput::Finite(documents) => documents.clone(),
        _ => panic!("a finite source trace"),
    };
    *trace = TraceInput::Progressed {
        trace: documents,
        progress: crate::spine::ProgressAssertion {
            watermark,
            completeness,
            scope: crate::spine::ProgressClosure::Open,
            execution: crate::spine::ProgressClosure::Open,
        },
    };
    *ledger = Some(carried.clone());
    request
}

/// A ledger returned by an accepted `run_clause` progress request.
fn retained_ledger() -> (crate::spine::LedgerBinding, crate::spine::ProgressLedger) {
    let report = report(with_progress(
        finite(Unit::Bounded, "Bounded", &[0, 1, 2]),
        100,
        crate::spine::ProgressCompleteness::Complete { through: 100 },
        &crate::spine::ProgressLedger::new(),
    ));
    let ledger = report
        .ledger
        .expect("an accepted request returns its updated ledger");
    let binding = ledger
        .bindings()
        .next()
        .expect("the request retained one binding")
        .clone();
    (binding, ledger)
}

fn with_ledger(
    mut request: ClauseRunRequest,
    carried: &crate::spine::ProgressLedger,
) -> ClauseRunRequest {
    let ClauseRunSelection::Temporal { ledger, .. } = &mut request.selection else {
        panic!("a temporal selection");
    };
    *ledger = Some(carried.clone());
    request
}

fn assert_admit_invalid_value(report: &ClauseRunReport) {
    let ClauseDisposition::Admit(AdmissionFailure::Refused(record)) = &report.disposition else {
        panic!(
            "expected an admission refusal, got {:?}",
            report.disposition
        );
    };
    assert_eq!(record.catalog.code(), "invalid_runtime_input");
    assert_eq!(record.catalog.cause(), "invalid-value");
    assert_eq!(report.disposition.stage(), ClauseRunStage::Admit);
    assert_eq!(report.disposition.category(), Category::Refusal);
}

#[trace("FR-502-AC-5", "TC-1296")]
#[test]
fn a_ledger_with_a_finite_trace_without_progress_is_refused_at_admit_and_returned_unchanged() {
    let (_, ledger) = retained_ledger();
    let report = report(with_ledger(
        finite(Unit::Bounded, "Bounded", &[0, 1, 2]),
        &ledger,
    ));
    assert_admit_invalid_value(&report);
    assert_eq!(report.ledger, Some(ledger));
}

#[trace("FR-502-AC-5", "TC-1296")]
#[test]
fn a_ledger_with_a_lasso_is_refused_at_admit_and_returned_unchanged() {
    let (_, ledger) = retained_ledger();
    let report = report(with_ledger(
        lasso(Unit::Infinite, "Reaches", 0, &[0, 1, 2]),
        &ledger,
    ));
    assert_admit_invalid_value(&report);
    assert_eq!(report.ledger, Some(ledger));
}

#[trace("FR-502-AC-5", "TC-1296")]
#[test]
fn a_ledger_with_an_infinite_trace_clause_is_refused_at_admit_and_returned_unchanged() {
    let (_, ledger) = retained_ledger();
    let report = report(with_ledger(
        finite(Unit::Infinite, "Reaches", &[0, 1, 2]),
        &ledger,
    ));
    assert_admit_invalid_value(&report);
    assert_eq!(report.ledger, Some(ledger));
}

#[trace("FR-502-AC-6", "TC-1296")]
#[test]
fn a_request_without_a_ledger_evaluates_its_trace_and_returns_none() {
    let report = report(finite(Unit::Bounded, "Bounded", &[0, 1, 2]));
    assert_eq!(report.ledger, None);
    assert_eq!(outcome(&report), &TemporalVerdict::Violated { at: 0 });
}

#[trace("FR-502-AC-1", "FR-502-AC-6", "FR-330-AC-9", "TC-1296")]
#[test]
fn run_clause_returns_the_updated_ledger_and_preserves_it_on_regression() {
    let first = report(with_progress(
        finite(Unit::Bounded, "Bounded", &[0, 1, 2]),
        100,
        crate::spine::ProgressCompleteness::Complete { through: 100 },
        &crate::spine::ProgressLedger::new(),
    ));
    assert_eq!(first.disposition.stage(), ClauseRunStage::Evaluate);
    assert_eq!(first.disposition.category(), Category::Unsupported);
    assert!(matches!(
        &first.disposition,
        ClauseDisposition::TemporalRefusal(TemporalTraceRefusal::ProfileNotEvaluated { .. })
    ));
    let ledger = first
        .ledger
        .expect("the accepted run returns its updated ledger");
    let binding = ledger
        .bindings()
        .next()
        .expect("the accepted run retained its binding");
    assert_eq!(
        ledger.entry(binding).map(|entry| entry.watermark()),
        Some(100)
    );

    let second = report(with_progress(
        finite(Unit::Bounded, "Bounded", &[0, 1, 2]),
        60,
        crate::spine::ProgressCompleteness::Complete { through: 100 },
        &ledger,
    ));
    assert_eq!(second.disposition.stage(), ClauseRunStage::Evaluate);
    assert_eq!(second.disposition.category(), Category::Refusal);
    assert_eq!(second.ledger, Some(ledger));
    let ClauseDisposition::ProgressContradiction(refusal) = &second.disposition else {
        panic!(
            "expected a progress contradiction, got {:?}",
            second.disposition
        );
    };
    assert_eq!(refusal.record.catalog.code(), "invalid_runtime_input");
    assert_eq!(refusal.record.catalog.cause(), "invalid-value");
    assert_eq!(
        refusal.record.fields.clone(),
        std::collections::BTreeMap::from([
            ("contradicted_dimension", "Watermark".to_owned()),
            ("clock", "model-steps".to_owned()),
            ("retained", "100".to_owned()),
            ("asserted", "60".to_owned()),
        ])
    );
}

#[trace("FR-502-AC-8", "TC-1296")]
#[test]
fn covered_positions_over_the_setting_report_incomplete_with_the_limit_fields() {
    let binding = retained_ledger().0;
    let positions: Vec<crate::spine::ObservedPosition> = (0..4)
        .map(|stamp| crate::spine::ObservedPosition {
            identity: format!("p{stamp}"),
            stamp,
        })
        .collect();
    let assertion = crate::spine::ProgressAssertion {
        watermark: 3,
        completeness: crate::spine::ProgressCompleteness::Incomplete,
        scope: crate::spine::ProgressClosure::Open,
        execution: crate::spine::ProgressClosure::Open,
    };
    let disposition = crate::spine::admit_progress(
        &crate::spine::ProgressLedger::new(),
        &binding,
        assertion,
        &positions,
        3,
    )
    .expect_err("four positions over a capacity of three");
    assert_eq!(disposition.category(), Category::Incomplete);
    let ClauseDisposition::Temporal(TemporalVerdict::ProgressLimitReached(reached)) = *disposition
    else {
        panic!("expected a progress limit outcome");
    };
    assert_eq!(
        (reached.limit, reached.value, reached.setting),
        ("node count", 3, "progress.covered_positions")
    );
}

/// FR-044-AC-7, FR-501-AC-10 (TC-1295): an `on each` guard that cannot be
/// established is an incomplete activation naming that guard, not a false
/// guard and not a violation.
#[trace("TC-1295", "FR-501-AC-10")]
#[test]
fn timestamped_undefined_guard_reports_a_named_incomplete_activation() {
    let mut request = timestamped_with(Unit::TimestampedEach, &[0, 10], &[2, 2], 12, Some(12));
    let bytes = program_source_bytes(&mut request);
    let source = String::from_utf8(bytes.clone()).unwrap();
    *bytes = source
        .replace(
            "on each (q: Counters::Counter) { eventually[0,2] holds(c.value > 0) }",
            "on each (q: Counters::Counter) when (3 mod (2 - q.value) >= 0) { eventually[0,2] holds(c.value > 0) }",
        )
        .into_bytes();
    let report = report(request);
    assert_eq!(report.category_label(), "incomplete");
    assert_eq!(report.disposition.exit_code(), 22);
    assert_eq!(report.disposition.truth(), None);
    let ClauseDisposition::Temporal(TemporalVerdict::ActivationIncomplete { at, guard }) =
        &report.disposition
    else {
        panic!(
            "expected a named incomplete activation, got {:?}",
            report.disposition
        );
    };
    assert_eq!(*at, 0);
    assert!(guard.path().is_empty());
}

/// FR-330-AC-9, FR-502 (TC-1295, TC-1296): a finite-window request that
/// carries a ledger retains its progress; a later request under the same
/// binding with a lower watermark is refused for `Watermark`, stage
/// `evaluate`, category `refusal`.
#[trace("TC-1295", "FR-330-AC-9")]
#[test]
fn timestamped_progress_is_admitted_into_the_ledger() {
    let with_ledger_and_watermark = |watermark: u64, carried: &crate::spine::ProgressLedger| {
        let mut request = timestamped(&[0, 2, 5], &[0, 1, 0], 100, Some(100));
        let trace = timestamped_trace_mut(&mut request);
        trace.progress.watermark = watermark;
        let ClauseRunSelection::Temporal { ledger, .. } = &mut request.selection else {
            panic!("a temporal selection");
        };
        *ledger = Some(carried.clone());
        request
    };
    let first = report(with_ledger_and_watermark(
        100,
        &crate::spine::ProgressLedger::new(),
    ));
    assert_eq!(outcome(&first), &TemporalVerdict::Holds);
    let retained = first
        .ledger
        .expect("an accepted request returns its updated ledger");
    assert_eq!(retained.len(), 1);
    let refused = report(with_ledger_and_watermark(60, &retained));
    assert_eq!(refused.disposition.stage(), ClauseRunStage::Evaluate);
    assert_eq!(refused.disposition.category(), Category::Refusal);
    let ClauseDisposition::ProgressContradiction(refusal) = &refused.disposition else {
        panic!(
            "expected a progress contradiction, got {:?}",
            refused.disposition
        );
    };
    assert_eq!(
        refusal
            .record
            .fields
            .get("contradicted_dimension")
            .map(String::as_str),
        Some("Watermark")
    );
    assert_eq!(refused.ledger.as_ref(), Some(&retained));
}
