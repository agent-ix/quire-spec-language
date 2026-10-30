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
use crate::execute::run_spine;
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
}

impl Unit {
    fn profile(self) -> &'static str {
        match self {
            Self::Bounded => "profile t = \"quire.temporal.event-position.false-extension/v1\";\n",
            Self::Infinite => "profile t = \"quire.temporal.infinite-trace/v1\";\n",
            Self::Undefined | Self::Guarded | Self::GuardUndefined => {
                "profile t = \"quire.temporal.event-position.false-extension/v1\";\n"
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
        }
    }
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
        },
        values,
    )
}

fn report(request: ClauseRunRequest) -> ClauseRunReport {
    run_clause(request).expect("a well-formed request always reports")
}

fn outcome(report: &ClauseRunReport) -> &TemporalVerdict {
    let ClauseDisposition::Temporal(outcome) = &report.disposition else {
        panic!("expected a temporal outcome, got {:?}", report.disposition);
    };
    outcome
}

fn exit_code(report: &ClauseRunReport) -> u8 {
    report.disposition.category().trace_exit_code()
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
    let reproduced = crate::replay(false_packet, crate::ReplayLimits::default())
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
    let inconclusive = crate::replay(holding_packet, crate::ReplayLimits::default())
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
    let replayed = crate::replay(reproduced, crate::ReplayLimits::default())
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
    let replayed = crate::replay(wrong_position, crate::ReplayLimits::default())
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
        crate::replay(wrong_interval, crate::ReplayLimits::default()),
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
    let reproduced = crate::replay(packet, crate::ReplayLimits::default())
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
    let inconclusive = crate::replay(packet, crate::ReplayLimits::default())
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
    let replayed = crate::replay(packet, crate::ReplayLimits::default())
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
    let replayed = crate::replay(wrong_cause, crate::ReplayLimits::default())
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
    let replayed = crate::replay(wrong_locus, crate::ReplayLimits::default())
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
    let replayed = crate::replay(limited, crate::ReplayLimits::default())
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
    let refusal = crate::replay(wire, crate::ReplayLimits::default())
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
    let refusal = crate::replay(wrong_source, crate::ReplayLimits::default())
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
    let refusal = crate::replay(wrong_target, crate::ReplayLimits::default())
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
        crate::replay(ordinary_state_clause, crate::ReplayLimits::default()),
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
    let refusal = crate::replay(mismatched_position, crate::ReplayLimits::default())
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
    let refusal = crate::replay(empty_loop, crate::ReplayLimits::default())
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
    let refusal = crate::replay(out_of_range, crate::ReplayLimits::default())
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
) -> ReplayRequestWire {
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
    let compiled = run_spine(
        &source_identity,
        &qsl_semantics::library::LibraryName::new("fixture").expect("a library name"),
        path,
        &source_bytes,
        &packages,
        &crate::spine::DependencyInput::default(),
        crate::spine::SpineLimits::default(),
    )
    .expect("the temporal fixture compiles");
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
    wire
}

fn rewrite_observed_payload(
    wire: ReplayRequestWire,
    rewrite: impl FnOnce(&mut crate::ObservedTemporalCounterexample),
) -> ReplayRequestWire {
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
    mut wire: ReplayRequestWire,
    rewrite: impl FnOnce(&mut crate::WitnessPacket<crate::ObservedTemporalCounterexample>),
) -> ReplayRequestWire {
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
        (record.code.as_str(), record.cause),
        ("invalid_runtime_input", "invalid-value")
    );

    let ghost = report(ghost());
    assert_eq!(ghost.disposition.stage(), ClauseRunStage::Admit);
    let ClauseDisposition::Admit(AdmissionFailure::Refused(record)) = &ghost.disposition else {
        panic!("expected an admission refusal, got {:?}", ghost.disposition);
    };
    assert_eq!(
        (record.code.as_str(), record.cause),
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
