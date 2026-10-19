// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-840, TC-847, TC-848 (FR-330): `run_clause`'s `Temporal` selection over
//! the `Counter` unit, from source, through genuine FR-106 admission and the
//! TemporalTrace evaluator.
//!
//! The package is TC-1043's `Counter` (`value`, `step`, operation `noop`).
//! Each clause below is the spec's, with `Counters::Counter` for the model
//! alias and `noop` for the fairness operation.

use qsl_eval::temporal::{
    TemporalOutcome, TemporalPosition, TemporalRefusal, UndefinedCause, UndefinedEvaluation,
};
use qsl_foundation::diagnostic::Category;
use qsl_semantics::model::observation::AdmissionFailure;
use quire_semantic_value::fairness::{FairnessGranularity, FairnessKind};

use super::receiver_frames::{counter, fixture, snapshot, unit_and_packages};
use super::*;
use crate::proof_result::SettlementBasis;
use crate::spine::{ClauseRunReport, OverObject, TraceInput};

const OPERATIONS: &[&str] = &["noop"];

const CLAUSES: &str = "profile e = \"quire.temporal.event-position.false-extension/v1\";\n\
     profile i = \"quire.temporal.infinite-trace/v1\";\n\
     temporal Bounded using e over (c: Counters::Counter) clock \"model-steps\" on origin \
     { eventually[0,1] holds(c.value = 2) }\n\
     temporal Reaches using i over (c: Counters::Counter) clock \"model-steps\" on origin \
     { eventually holds(c.value = 2) }\n\
     temporal Undef using e over (c: Counters::Counter) clock \"model-steps\" on origin \
     { always[0,2] holds(2 / (2 - c.value) >= 1) }\n\
     temporal Fair using i over (c: Counters::Counter) clock \"model-steps\" on origin \
     { fair weak Counters::Counter::noop; always eventually holds(c.value = 2) }\n";

/// The `Counter` unit and a request for `selection` over snapshots with
/// `c1.value` at each of `values`, in the order the trace names them.
fn request(
    selection: impl FnOnce(&[DocumentRef]) -> ClauseRunSelection,
    values: &[i64],
) -> ClauseRunRequest {
    let (mut unit, packages) = unit_and_packages(OPERATIONS);
    unit.push_str(CLAUSES);
    let documents: Vec<(DocumentRef, Vec<u8>)> = values
        .iter()
        .enumerate()
        .map(|(position, value)| {
            document_ref_and_bytes(&format!("counter-{position}"), |label| {
                snapshot(OPERATIONS, label, "current", &[counter("c1", *value)])
            })
        })
        .collect();
    let labels: Vec<DocumentRef> = documents.iter().map(|(label, _)| label.clone()).collect();
    let mut request =
        config_version_request_for(unit, packages, "counter-temporal.native", selection(&labels));
    for (label, bytes) in documents {
        request.snapshots.insert(label.digest, bytes);
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
fn finite(name: &str, values: &[i64]) -> ClauseRunRequest {
    request(
        |labels| ClauseRunSelection::Temporal {
            name: name.to_owned(),
            trace: TraceInput::Finite(labels.to_vec()),
            over: over("c1"),
        },
        values,
    )
}

/// `name` over a lasso whose prefix is the first `prefix` of `values` and
/// whose loop is the rest.
fn lasso(name: &str, prefix: usize, values: &[i64]) -> ClauseRunRequest {
    request(
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

fn outcome(report: &ClauseRunReport) -> &TemporalOutcome {
    let ClauseDisposition::Temporal(outcome) = &report.disposition else {
        panic!("expected a temporal outcome, got {:?}", report.disposition);
    };
    outcome
}

fn exit_code(report: &ClauseRunReport) -> u8 {
    report.disposition.category().trace_exit_code()
}

/// `Bounded` (`eventually[0,1] holds(c.value = 2)`) over `Finite` 0, 1, 2 is
/// a violation at position 0.
#[trace("TC-840", "FR-330-AC-1")]
#[test]
fn a_bounded_clause_over_a_finite_trace_violates_at_its_position() {
    let report = report(finite("Bounded", &[0, 1, 2]));
    assert_eq!(report.disposition.stage(), ClauseRunStage::Evaluate);
    assert_eq!(
        outcome(&report),
        &TemporalOutcome::Violated {
            at: TemporalPosition::new(0)
        }
    );
    assert_eq!(report.disposition.category(), Category::Violation);
    assert_eq!(exit_code(&report), 10);
    assert!(report.package_id.is_some());
}

/// `Reaches` over the same `Finite` trace is pending: inconclusive, exit 0,
/// no truth. Over a lasso with loop 0, 1 it violates at position 0; over a
/// lasso with loop 0, 1, 2 it holds.
#[trace("TC-840", "FR-330-AC-2")]
#[test]
fn an_infinite_trace_clause_reads_a_finite_prefix_as_pending_and_a_lasso_exactly() {
    let pending = report(finite("Reaches", &[0, 1, 2]));
    assert_eq!(outcome(&pending), &TemporalOutcome::Pending);
    assert_eq!(pending.disposition.category(), Category::Inconclusive);
    assert_eq!(exit_code(&pending), 0);
    assert_eq!(pending.disposition.truth(), None);

    let violated = report(lasso("Reaches", 0, &[0, 1]));
    assert_eq!(
        outcome(&violated),
        &TemporalOutcome::Violated {
            at: TemporalPosition::new(0)
        }
    );

    let holds = report(lasso("Reaches", 0, &[0, 1, 2]));
    assert_eq!(outcome(&holds), &TemporalOutcome::Holds);
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
    let absent = report(finite("Absent", &[0, 1, 2]));
    assert_eq!(absent.disposition.stage(), ClauseRunStage::Select);
    assert!(
        matches!(&absent.disposition, ClauseDisposition::MissingName { name } if name == "Absent"),
        "{:?}",
        absent.disposition
    );

    let bounded_lasso = report(lasso("Bounded", 0, &[0, 1, 2]));
    assert_eq!(bounded_lasso.disposition.stage(), ClauseRunStage::Admit);
    let ClauseDisposition::Admit(AdmissionFailure::Refused(record)) = &bounded_lasso.disposition
    else {
        panic!("expected an admission refusal, got {:?}", bounded_lasso.disposition);
    };
    assert_eq!(
        (record.code.as_str(), record.cause),
        ("invalid_runtime_input", "invalid-value")
    );

    let ghost = report(request(
        |labels| ClauseRunSelection::Temporal {
            name: "Reaches".to_owned(),
            trace: TraceInput::Finite(labels.to_vec()),
            over: over("ghost"),
        },
        &[0, 1, 2],
    ));
    assert_eq!(ghost.disposition.stage(), ClauseRunStage::Admit);
    let ClauseDisposition::Admit(AdmissionFailure::Refused(record)) = &ghost.disposition else {
        panic!("expected an admission refusal, got {:?}", ghost.disposition);
    };
    assert_eq!(
        (record.code.as_str(), record.cause),
        ("invalid_runtime_input", "wrong-role-mapping")
    );

    let empty_loop = report(lasso("Reaches", 3, &[0, 1, 2]));
    assert_eq!(empty_loop.disposition.stage(), ClauseRunStage::Evaluate);
    assert_eq!(empty_loop.disposition.category(), Category::Refusal);
    let ClauseDisposition::TemporalRefusal(refusal) = &empty_loop.disposition else {
        panic!("expected a trace refusal, got {:?}", empty_loop.disposition);
    };
    assert_eq!(refusal, &TemporalRefusal::EmptyLoop);
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
    let mut starved = lasso("Reaches", 0, &[0, 1, 2]);
    starved.accounting = default_accounting(0);
    let starved = report(starved);
    assert!(
        matches!(outcome(&starved), TemporalOutcome::Incomplete(_)),
        "{:?}",
        starved.disposition
    );
    assert_eq!(starved.disposition.category(), Category::Incomplete);
    assert_eq!(exit_code(&starved), 22);

    let requests: [fn() -> ClauseRunRequest; 6] = [
        || finite("Bounded", &[0, 1, 2]),
        || finite("Reaches", &[0, 1, 2]),
        || lasso("Reaches", 0, &[0, 1]),
        || lasso("Reaches", 0, &[0, 1, 2]),
        || finite("Absent", &[0, 1, 2]),
        || lasso("Bounded", 0, &[0, 1, 2]),
    ];
    for build in requests {
        assert_eq!(
            format!("{:?}", report(build())),
            format!("{:?}", report(build()))
        );
    }
}

/// `always[0,2] holds(2 / (2 - c.value) >= 1)` over 0, 1, 2 is undefined at
/// position 2 by division by zero, and exits as a violation.
#[trace("TC-847", "FR-330-AC-5")]
#[test]
fn an_undefined_atom_is_a_violation_at_its_position_with_its_cause() {
    let report = report(finite("Undef", &[0, 1, 2]));
    let TemporalOutcome::Undefined(UndefinedEvaluation { at, cause }) = outcome(&report) else {
        panic!("expected an undefined evaluation, got {:?}", report.disposition);
    };
    assert_eq!(*at, TemporalPosition::new(2));
    assert_eq!(
        cause,
        &UndefinedCause::Kernel(quire_exact::Undefined::DivisionByZero)
    );
    assert_eq!(report.disposition.category(), Category::Violation);
    assert_eq!(exit_code(&report), 10);
}

/// `Fair` (`always eventually holds(c.value = 2)` under `fair weak noop`)
/// over a lasso is unsupported: an observed trace carries no enabledness.
#[trace("TC-848", "FR-330-AC-6")]
#[test]
fn a_fairness_premise_over_a_supplied_trace_is_unsupported() {
    let report = report(lasso("Fair", 0, &[0, 1, 2]));
    assert_eq!(report.disposition.stage(), ClauseRunStage::Evaluate);
    let TemporalOutcome::MissingFairnessPremise { constraint } = outcome(&report) else {
        panic!("expected a missing premise, got {:?}", report.disposition);
    };
    assert_eq!(constraint.kind(), FairnessKind::Weak);
    assert_eq!(constraint.granularity(), FairnessGranularity::Whole);
    assert_eq!(constraint.operation().name(), "noop");
    assert_eq!(report.disposition.category(), Category::Unsupported);
    assert_eq!(exit_code(&report), 21);
}

/// `Reaches` run at S6a on a lasso where it holds is evidence for that trace
/// only: it settles no truth and derives no witness, so nothing joins it to a
/// requested item (FR-332: S6a evaluation never settles an item).
#[trace("TC-842", "FR-332-AC-4")]
#[test]
fn a_clause_run_over_a_trace_settles_no_item() {
    let held = report(lasso("Reaches", 0, &[0, 1, 2]));
    assert_eq!(outcome(&held), &TemporalOutcome::Holds);
    assert_eq!(held.basis, SettlementBasis::Unavailable);
    assert!(held.witness.is_none());

    let violated = report(lasso("Reaches", 0, &[0, 1]));
    assert_eq!(violated.basis, SettlementBasis::Unavailable);
    assert!(violated.witness.is_none());
}
