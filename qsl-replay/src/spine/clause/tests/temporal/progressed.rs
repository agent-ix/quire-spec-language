// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1299 (FR-504, FR-330-AC-10): `run_clause` over `TraceInput::Progressed`
//! for event-position and fixed-sample clauses, from source, through genuine
//! FR-106 admission and the progress-aware evaluator.
//!
//! `p` is `c.value = 1`: a position whose value is 1 has `p` true. Indices are
//! coordinates. Expected values are the FR-043 reading of silence written in
//! the TC-1299 table, never the evaluator's own output.

use super::*;
use crate::spine::{
    ContradictedDimension, FiniteWindowReport, ProgressAssertion, ProgressClosure,
    ProgressCompleteness, ProgressLedger, ProgressLowerBoundary, TemporalIndeterminateReason,
};

const OPEN: ProgressClosure = ProgressClosure::Open;
const CLOSED: ProgressClosure = ProgressClosure::Closed;

fn complete(through: u64) -> ProgressCompleteness {
    ProgressCompleteness::Complete { through }
}

/// One progressed request for the `Sample` clause of `unit`.
struct Progressed {
    unit: Unit,
    values: Vec<i64>,
    watermark: u64,
    completeness: ProgressCompleteness,
    scope: ProgressClosure,
    boundary: ProgressLowerBoundary,
    ledger: Option<ProgressLedger>,
}

impl Progressed {
    fn new(unit: Unit, values: &[i64], watermark: u64, completeness: ProgressCompleteness) -> Self {
        Self {
            unit,
            values: values.to_vec(),
            watermark,
            completeness,
            scope: OPEN,
            boundary: ProgressLowerBoundary::AuthoritativeOrigin,
            ledger: None,
        }
    }

    fn scope(mut self, scope: ProgressClosure) -> Self {
        self.scope = scope;
        self
    }

    fn boundary(mut self, boundary: ProgressLowerBoundary) -> Self {
        self.boundary = boundary;
        self
    }

    fn ledger(mut self, ledger: ProgressLedger) -> Self {
        self.ledger = Some(ledger);
        self
    }

    fn request(&self) -> ClauseRunRequest {
        self.request_with(|binding| binding)
    }

    fn request_with(
        &self,
        binding: impl FnOnce(crate::spine::TraceBinding) -> crate::spine::TraceBinding,
    ) -> ClauseRunRequest {
        request(
            self.unit,
            |labels| ClauseRunSelection::Temporal {
                name: "Sample".to_owned(),
                trace: TraceInput::Progressed {
                    trace: labels.to_vec(),
                    progress: ProgressAssertion {
                        watermark: self.watermark,
                        completeness: self.completeness,
                        scope: self.scope,
                        execution: self.scope,
                    },
                    lower_boundary: self.boundary,
                    binding: binding(progressed_binding(self.unit)),
                },
                over: over("c1"),
                ledger: self.ledger.clone(),
            },
            &self.values,
        )
    }

    fn run(&self) -> ClauseRunReport {
        report(self.request())
    }
}

fn window(report: &ClauseRunReport) -> &FiniteWindowReport {
    report
        .finite_window
        .as_ref()
        .expect("a progressed verdict reports its basis, support and premises")
}

fn indeterminate(at: u64, reason: TemporalIndeterminateReason) -> TemporalVerdict {
    TemporalVerdict::Indeterminate { at, reason }
}

/// TC-1299 row 1 and 2: a fixed-sample watermark settles a silent deadline
/// with no new observation, and says so in its basis.
#[trace("TC-1299", "FR-504-AC-1", "FR-330-AC-10")]
#[test]
fn a_fixed_sample_watermark_settles_a_silent_deadline() {
    let before = Progressed::new(Unit::FixedEventually, &[0], 0, complete(0)).run();
    assert_eq!(
        outcome(&before),
        &TemporalVerdict::PendingWindow { at: 0, deadline: 3 }
    );

    let at_deadline = Progressed::new(Unit::FixedEventually, &[0], 3, complete(3)).run();
    assert_eq!(outcome(&at_deadline), &TemporalVerdict::Violated { at: 0 });
    assert_eq!(
        window(&at_deadline).basis,
        SettlementBasis::DecisiveCounterexample
    );

    let closed = Progressed::new(Unit::FixedEventually, &[0], 3, complete(3))
        .scope(CLOSED)
        .run();
    assert_eq!(outcome(&closed), &TemporalVerdict::Violated { at: 0 });
    assert_eq!(window(&closed).basis, SettlementBasis::ClosedScope);

    // One tick short of the deadline settles nothing.
    let short = Progressed::new(Unit::FixedEventually, &[0], 2, complete(2)).run();
    assert_eq!(
        outcome(&short),
        &TemporalVerdict::PendingWindow { at: 0, deadline: 3 }
    );
}

/// TC-1299 rows 3 and 4: an event-position watermark is a premise that
/// advances nothing; a closed complete scope reads by false extension.
#[trace("TC-1299", "FR-504-AC-2", "FR-330-AC-10")]
#[test]
fn an_event_position_watermark_is_a_premise_that_advances_nothing() {
    let open = Progressed::new(Unit::EventEventually, &[0], 3, complete(3)).run();
    assert_eq!(outcome(&open), &TemporalVerdict::PendingNextEvent { at: 0 });
    assert_eq!(window(&open).premises.watermark, 3);

    let closed = Progressed::new(Unit::EventEventually, &[0], 0, complete(0))
        .scope(CLOSED)
        .run();
    assert_eq!(outcome(&closed), &TemporalVerdict::Violated { at: 0 });
}

/// TC-1299 rows 5 and 6: the deadline position participates at `W = D` and a
/// position one past it does not.
#[trace("TC-1299", "FR-504-AC-3")]
#[test]
fn the_deadline_position_participates_and_the_next_does_not() {
    let at_three = Progressed::new(Unit::FixedEventually, &[0, 0, 0, 1], 3, complete(3)).run();
    assert_eq!(outcome(&at_three), &TemporalVerdict::Holds);

    let at_four = Progressed::new(Unit::FixedEventually, &[0, 0, 0, 0, 1], 4, complete(4)).run();
    assert_eq!(outcome(&at_four), &TemporalVerdict::Violated { at: 0 });
}

/// TC-1299 rows 7 and 8: an incomplete input applies no false extension even
/// in a closed scope; the same watermark with complete input reads the silent
/// samples as false.
#[trace("TC-1299", "FR-504-AC-4")]
#[test]
fn an_incomplete_input_applies_no_false_extension() {
    let incomplete = Progressed::new(Unit::FixedAlways, &[1], 5, ProgressCompleteness::Incomplete)
        .scope(CLOSED)
        .run();
    assert_eq!(
        outcome(&incomplete),
        &indeterminate(0, TemporalIndeterminateReason::IncompleteInput)
    );
    assert_eq!(window(&incomplete).basis, SettlementBasis::Unavailable);

    let complete_input = Progressed::new(Unit::FixedAlways, &[1], 5, complete(5))
        .scope(CLOSED)
        .run();
    assert_eq!(
        outcome(&complete_input),
        &TemporalVerdict::Violated { at: 0 }
    );
}

/// TC-1299 rows 9 to 11: an open event-position scope settles only on a
/// decisive witness or counterexample, and otherwise waits for the next event.
#[trace("TC-1299", "FR-504-AC-5")]
#[test]
fn an_open_event_position_scope_settles_only_on_a_decisive_fact() {
    let witness = Progressed::new(Unit::EventEventually, &[0, 1, 0], 2, complete(2)).run();
    assert_eq!(outcome(&witness), &TemporalVerdict::Holds);
    assert_eq!(window(&witness).basis, SettlementBasis::DecisiveWitness);

    let counterexample = Progressed::new(Unit::EventAlways, &[1, 0, 1], 2, complete(2)).run();
    assert_eq!(
        outcome(&counterexample),
        &TemporalVerdict::Violated { at: 0 }
    );
    assert_eq!(
        window(&counterexample).basis,
        SettlementBasis::DecisiveCounterexample
    );

    let undecided = Progressed::new(Unit::EventAlways, &[1, 1, 1], 2, complete(2)).run();
    assert_eq!(
        outcome(&undecided),
        &TemporalVerdict::PendingNextEvent { at: 0 }
    );
}

/// TC-1299 row 12: a past operator reaching before position 0 is missing
/// history under a history cutoff and reads as FR-327 from an origin.
#[trace("TC-1299", "FR-504-AC-6")]
#[test]
fn a_past_operator_before_position_zero_follows_the_lower_boundary() {
    let cutoff = Progressed::new(Unit::EventOnce, &[0, 0], 1, complete(1))
        .scope(CLOSED)
        .boundary(ProgressLowerBoundary::HistoryCutoff)
        .run();
    assert_eq!(
        outcome(&cutoff),
        &indeterminate(0, TemporalIndeterminateReason::MissingHistory)
    );

    let origin = Progressed::new(Unit::EventOnce, &[0, 0], 1, complete(1))
        .scope(CLOSED)
        .run();
    assert_eq!(outcome(&origin), &TemporalVerdict::Violated { at: 0 });
}

/// TC-1299 rows 13 to 15: a `Finite` trace without progress is FR-327's
/// closed complete reading unchanged; inconsistent progress and a progressed
/// request for a clause that carries none are refused at admit.
#[trace("TC-1299", "FR-504-AC-7", "FR-330-AC-10")]
#[test]
fn a_finite_trace_is_unchanged_and_inconsistent_progress_is_refused() {
    let holds = report(finite(Unit::FixedEventually, "Sample", &[0, 0, 0, 1]));
    assert_eq!(outcome(&holds), &TemporalVerdict::Holds);
    let violated = report(finite(Unit::FixedEventually, "Sample", &[0, 0, 0, 0]));
    assert_eq!(outcome(&violated), &TemporalVerdict::Violated { at: 0 });

    let watermark = Progressed::new(Unit::FixedEventually, &[0, 0], 0, complete(1)).run();
    assert_admit_invalid_value(&watermark);
    let through = Progressed::new(Unit::FixedEventually, &[0, 0], 1, complete(0)).run();
    assert_admit_invalid_value(&through);
    let event_through = Progressed::new(Unit::EventEventually, &[0, 0], 0, complete(0)).run();
    assert_admit_invalid_value(&event_through);
    // The refusals did not shorten the assertions: the accepted boundary is
    // the last position's coordinate.
    let accepted = Progressed::new(Unit::FixedEventually, &[0, 0], 1, complete(1)).run();
    assert_eq!(
        outcome(&accepted),
        &TemporalVerdict::PendingWindow { at: 0, deadline: 3 }
    );

    let infinite = request(
        Unit::Infinite,
        |labels| ClauseRunSelection::Temporal {
            name: "Reaches".to_owned(),
            trace: TraceInput::Progressed {
                trace: labels.to_vec(),
                progress: ProgressAssertion {
                    watermark: 2,
                    completeness: complete(2),
                    scope: OPEN,
                    execution: OPEN,
                },
                lower_boundary: ProgressLowerBoundary::AuthoritativeOrigin,
                binding: crate::spine::TraceBinding {
                    profile: "quire.temporal.infinite-trace/v1".to_owned(),
                    clock_name: "model-steps".to_owned(),
                    parameters: qsl_package::ClockConfiguration::EventPosition {
                        sequence_authority: "counter-event-sequence".into(),
                    },
                },
            },
            over: over("c1"),
            ledger: None,
        },
        &[0, 1, 2],
    );
    assert_admit_invalid_value(&report(infinite));

    let finite_window = request(
        Unit::Timestamped,
        |labels| ClauseRunSelection::Temporal {
            name: "Window".to_owned(),
            trace: TraceInput::Progressed {
                trace: labels.to_vec(),
                progress: ProgressAssertion {
                    watermark: 2,
                    completeness: complete(2),
                    scope: OPEN,
                    execution: OPEN,
                },
                lower_boundary: ProgressLowerBoundary::AuthoritativeOrigin,
                binding: finite_window_binding(),
            },
            over: over("c1"),
            ledger: None,
        },
        &[0, 1, 2],
    );
    assert_admit_invalid_value(&report(finite_window));
}

/// FR-330-AC-11 for `Progressed`: a binding asserting another profile or clock
/// is refused at admit naming the dimension, before any position is visited.
#[trace("TC-1317", "FR-330-AC-11")]
#[test]
fn a_progressed_binding_that_asserts_another_profile_or_clock_is_refused() {
    let progressed = Progressed::new(Unit::FixedEventually, &[0], 3, complete(3));
    let profile = report(
        progressed.request_with(|binding| crate::spine::TraceBinding {
            profile: "quire.temporal.event-position.false-extension/v1".to_owned(),
            ..binding
        }),
    );
    assert_admit_invalid_value(&profile);
    let ClauseDisposition::Admit(AdmissionFailure::Refused(record)) = &profile.disposition else {
        panic!(
            "expected an admission refusal, got {:?}",
            profile.disposition
        );
    };
    assert_eq!(
        record
            .fields
            .get("mismatched_dimension")
            .map(String::as_str),
        Some("Profile")
    );

    let clock = report(
        progressed.request_with(|binding| crate::spine::TraceBinding {
            clock_name: "ms".to_owned(),
            ..binding
        }),
    );
    assert_admit_invalid_value(&clock);
    let ClauseDisposition::Admit(AdmissionFailure::Refused(record)) = &clock.disposition else {
        panic!("expected an admission refusal, got {:?}", clock.disposition);
    };
    assert_eq!(
        record
            .fields
            .get("mismatched_dimension")
            .map(String::as_str),
        Some("Clock")
    );
    // The binding was refused before any position was read or charged.
    assert_eq!(clock.usage.temporal_visits, 0);
}

/// TC-1299 rows 16 and 17: the ledger covers these shapes. A fixed-sample
/// watermark that regresses is refused as a contradiction; an event-position
/// watermark that advances is retained and settles nothing.
#[trace("TC-1299", "TC-1296", "FR-504-AC-8", "FR-330-AC-10")]
#[test]
fn the_ledger_retains_progress_for_both_shapes() {
    let first = Progressed::new(Unit::FixedEventually, &[0], 3, complete(3))
        .ledger(ProgressLedger::new())
        .run();
    assert_eq!(outcome(&first), &TemporalVerdict::Violated { at: 0 });
    let ledger = first
        .ledger
        .expect("an accepted request returns its updated ledger");
    let binding = ledger.bindings().next().expect("one binding").clone();
    assert_eq!(
        ledger.entry(&binding).map(|entry| entry.watermark()),
        Some(3)
    );

    let regressed = Progressed::new(Unit::FixedEventually, &[0], 1, complete(3))
        .ledger(ledger.clone())
        .run();
    let ClauseDisposition::ProgressContradiction(refusal) = &regressed.disposition else {
        panic!("expected a contradiction, got {:?}", regressed.disposition);
    };
    assert_eq!(
        refusal.contradiction.dimension,
        ContradictedDimension::Watermark
    );
    assert_eq!(regressed.ledger, Some(ledger));

    let at_three = Progressed::new(Unit::EventEventually, &[0], 3, complete(3))
        .ledger(ProgressLedger::new())
        .run();
    assert_eq!(
        outcome(&at_three),
        &TemporalVerdict::PendingNextEvent { at: 0 }
    );
    let retained = at_three.ledger.expect("the updated ledger");
    let at_nine = Progressed::new(Unit::EventEventually, &[0], 9, complete(9))
        .ledger(retained)
        .run();
    assert_eq!(
        outcome(&at_nine),
        &TemporalVerdict::PendingNextEvent { at: 0 }
    );
    let binding = at_nine
        .ledger
        .as_ref()
        .and_then(|ledger| ledger.bindings().next().cloned())
        .expect("the advanced ledger");
    assert_eq!(
        at_nine
            .ledger
            .as_ref()
            .and_then(|ledger| ledger.entry(&binding))
            .map(|entry| entry.watermark()),
        Some(9)
    );
}

/// FR-504 "Silent samples": a watermark far past the last observation
/// settles by reading only the samples the clause can reach, so the work does
/// not grow with the watermark.
#[trace("TC-1299", "FR-504-AC-1")]
#[test]
fn a_distant_watermark_reads_only_the_samples_the_clause_reaches() {
    let near = Progressed::new(Unit::FixedEventually, &[0], 3, complete(3)).run();
    let distant = Progressed::new(
        Unit::FixedEventually,
        &[0],
        1_000_000_000_000,
        complete(1_000_000_000_000),
    )
    .run();
    assert_eq!(outcome(&distant), &TemporalVerdict::Violated { at: 0 });
    assert_eq!(distant.usage.temporal_visits, near.usage.temporal_visits);
}

/// FR-504 "Silent samples": a reach the work budget cannot afford ends
/// `Incomplete` with no truth before any silent sample is allocated. A reach
/// of 10^12 with a watermark of 10^12 would otherwise materialize 10^12
/// samples.
#[trace("TC-1299", "FR-504-AC-1")]
#[test]
fn a_reach_past_the_work_budget_is_incomplete_without_materializing_samples() {
    let limit = 1_000_000_000_000;
    let starved = Progressed::new(Unit::FixedHugeReach, &[0], limit, complete(limit)).run();
    assert!(
        matches!(outcome(&starved), TemporalVerdict::Incomplete(_)),
        "{:?}",
        starved.disposition
    );
    assert_eq!(starved.disposition.truth(), None);
}

/// FR-504 "Silent samples (fixed-sample only)": samples after the last
/// observation are no observation, so an `on each` clause anchors no
/// activation on them. `eventually[0,1] holds(p)` anchored on the silent
/// sample at index 2 would read two false samples and violate.
#[trace("TC-1299", "FR-504-AC-1", "FR-504-AC-3")]
#[test]
fn silent_samples_anchor_no_on_each_activation() {
    let holds = Progressed::new(Unit::FixedEach, &[1, 1], 5, complete(5)).run();
    assert_eq!(outcome(&holds), &TemporalVerdict::Holds);
    // The control: the observed anchors still decide. The last observation
    // reads `p` false with a silent false sample after it.
    let violated = Progressed::new(Unit::FixedEach, &[1, 0], 5, complete(5)).run();
    assert_eq!(outcome(&violated), &TemporalVerdict::Violated { at: 1 });
}

/// FR-504 "Open scope" and "Incomplete input": under an open scope an
/// unsettled fixed-sample obligation is `Pending` with deadline `anchor +
/// reach` until `W` reaches it, and `Indeterminate` from there when the input
/// stays incomplete.
#[trace("TC-1299", "FR-504-AC-1", "FR-504-AC-4")]
#[test]
fn an_open_fixed_sample_obligation_is_indeterminate_once_the_watermark_reaches_the_deadline() {
    let before = Progressed::new(
        Unit::FixedEventually,
        &[0],
        2,
        ProgressCompleteness::Incomplete,
    )
    .run();
    assert_eq!(
        outcome(&before),
        &TemporalVerdict::PendingWindow { at: 0, deadline: 3 }
    );
    let at = Progressed::new(
        Unit::FixedEventually,
        &[0],
        3,
        ProgressCompleteness::Incomplete,
    )
    .run();
    assert_eq!(
        outcome(&at),
        &indeterminate(0, TemporalIndeterminateReason::IncompleteInput)
    );
}

/// FR-504-AC-3: a fixed-sample witness inside the seen horizon settles an
/// open scope as `Completed(true)` with basis `decisive-witness`.
#[trace("TC-1299", "FR-504-AC-3")]
#[test]
fn a_fixed_sample_witness_inside_the_seen_horizon_is_decisive() {
    let witness = Progressed::new(Unit::FixedEventually, &[0, 1], 3, complete(3)).run();
    assert_eq!(outcome(&witness), &TemporalVerdict::Holds);
    assert_eq!(window(&witness).basis, SettlementBasis::DecisiveWitness);
}
