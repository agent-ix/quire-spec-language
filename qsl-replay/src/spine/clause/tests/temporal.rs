// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-840, TC-848 (FR-330): `run_clause`'s `Temporal` selection over
//! the `Counter` unit, from source, through genuine FR-106 admission and the
//! TemporalTrace evaluator.
//!
//! The package is TC-1043's `Counter` (`value`, `step`, operation `noop`).
//! Each clause below is the spec's, with `Counters::Counter` for the model
//! alias and `noop` for the fairness operation.

use qsl_foundation::diagnostic::Category;
use qsl_semantics::model::observation::AdmissionFailure;
use quire_semantic_value::fairness::{FairnessGranularity, FairnessKind};

use super::receiver_frames::{counter, fixture, snapshot, unit_and_packages};
use super::*;
use crate::proof_result::SettlementBasis;
use crate::spine::{
    ClauseRunReport, OverObject, TemporalTraceRefusal, TemporalVerdict, TraceInput,
};

const OPERATIONS: &[&str] = &["noop"];

/// A unit holds one temporal profile, so the bounded and the infinite-trace
/// clauses of the spec's `Counter` unit are two units.
#[derive(Clone, Copy)]
enum Unit {
    /// `Bounded`, under event-position false-extension.
    Bounded,
    /// `Reaches` and `Fair`, under infinite-trace.
    Infinite,
}

impl Unit {
    fn profile(self) -> &'static str {
        match self {
            Self::Bounded => "profile t = \"quire.temporal.event-position.false-extension/v1\";\n",
            Self::Infinite => "profile t = \"quire.temporal.infinite-trace/v1\";\n",
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
                snapshot(OPERATIONS, label, "current", &[counter("c1", *value)])
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
    assert!(report.package_id.is_some());
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
            qsl_foundation::diagnostic::InternalFault::new("temporal", "broken")
        )),
        Category::InternalFailure
    );
}
