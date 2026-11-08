// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1160 (FR-267 "Temporal disposition wire", FR-266-AC-4, FR-285-AC-7):
//! every temporal report `run_clause` returns is written as
//! `native-run-result/2` with its `disposition.temporal` row and read back
//! by the strict reader; each edit the ACs list refuses naming the member.

use serde_json::{json, Value};

use super::*;
use crate::spine::{
    read_native_run_result, write_clause_run_result, IndeterminateReasonRead, NativeRunResult,
    TemporalIndeterminateReason, TemporalRecord,
};

fn document(report: &ClauseRunReport) -> Value {
    let bytes = write_clause_run_result(report).expect("every temporal report is written");
    let canonical = quire_canonical::to_vec(
        &quire_canonical::read(&bytes, u64::MAX)
            .expect("the document is canonical JSON")
            .root(),
        quire_canonical::Limits::new(u64::MAX),
    )
    .expect("the document re-encodes");
    assert_eq!(bytes, canonical, "the bytes are their RFC 8785 encoding");
    serde_json::from_slice(&bytes).expect("the document is JSON")
}

fn read(value: &Value) -> Result<crate::spine::ClauseRunRecord, String> {
    match read_native_run_result(&serde_json::to_vec(value).expect("JSON")) {
        Ok(NativeRunResult::Clause(record)) => Ok(*record),
        Ok(NativeRunResult::CommandError(_)) => Err("command error".to_owned()),
        Err(error) => Err(error.to_string()),
    }
}

/// Write `report`, read it back, and check that the record restores the
/// report's typed disposition (FR-267): its verdict, trace refusal or
/// contradiction. An `Incomplete` restores none: its outcome carries the
/// limit kind only.
fn round_trips(report: &ClauseRunReport) -> Value {
    let value = document(report);
    let record = read(&value).expect("the document reads");
    let temporal = record.temporal().expect("a temporal row");
    match &report.disposition {
        ClauseDisposition::Temporal(TemporalVerdict::Incomplete(_)) => {
            assert_eq!(temporal.verdict(), None);
        }
        ClauseDisposition::Temporal(verdict) => {
            assert_eq!(temporal.verdict().as_ref(), Some(verdict));
        }
        ClauseDisposition::TemporalRefusal(refusal) => {
            assert_eq!(temporal.trace_refusal().as_ref(), Some(refusal));
            assert_eq!(temporal.verdict(), None);
        }
        ClauseDisposition::ProgressContradiction(refusal) => {
            assert_eq!(
                temporal.contradiction().as_ref(),
                Some(&refusal.contradiction)
            );
        }
        other => panic!("a temporal disposition, got {other:?}"),
    }
    value
}

fn refuses(value: &Value, member: &str) {
    let error = read(value).err().unwrap_or_else(|| {
        panic!("the edited document must refuse: {value}");
    });
    assert!(error.contains(member), "`{error}` names `{member}`");
}

fn edited(value: &Value, edit: impl FnOnce(&mut Value)) -> Value {
    let mut copy = value.clone();
    edit(&mut copy);
    copy
}

fn temporal(value: &Value) -> &Value {
    &value["disposition"]["temporal"]
}

/// FR-330-AC-2's lasso violation: stage `evaluate`, `violation`, `closed-scope`,
/// no witness, and the observed-arm counterexample at position 0.
#[trace("TC-1160", "FR-267-AC-13")]
#[trace("TC-1160", "FR-266-AC-4")]
#[test]
fn a_lasso_violation_writes_its_observed_counterexample() {
    let violation = report(lasso(Unit::Infinite, "Reaches", 0, &[0, 1]));
    assert_eq!(violation.basis, SettlementBasis::ClosedScope);
    let value = round_trips(&violation);
    assert_eq!(value["disposition"]["stage"], "evaluate");
    assert_eq!(value["disposition"]["category"], "violation");
    assert_eq!(value["disposition"]["truth"], false);
    assert_eq!(value["basis"], "closed-scope");
    assert!(value.get("witness").is_none());
    assert!(value["disposition"].get("outcome").is_none());
    let counterexample = &temporal(&value)["counterexample"];
    assert_eq!(temporal(&value)["kind"], "violated");
    assert_eq!(temporal(&value)["position"], 0);
    assert_eq!(counterexample["trace"], "observed");
    assert_eq!(counterexample["kind"], "formula");
    assert_eq!(counterexample["initial_state"], Value::Null);
    assert_eq!(counterexample["over_binding"], Value::Null);
    assert_eq!(counterexample["prefix"], json!([]));
    assert_eq!(counterexample["loop"].as_array().map(Vec::len), Some(2));
    assert_eq!(counterexample["fairness"], json!([]));
    assert_eq!(counterexample["interval"], Value::Null);
    assert!(counterexample["trace_position"].is_u64());
    assert_eq!(counterexample["trace_position"], 0);

    let record = read(&value).expect("the document reads");
    let Some(TemporalRecord::Violated {
        position,
        counterexample,
    }) = record.temporal()
    else {
        panic!("a violated row");
    };
    assert_eq!(*position, 0);
    let expected = violation.counterexample.as_ref().expect("a counterexample");
    assert_eq!(
        counterexample
            .loop_documents()
            .map(|(authority, identity)| (authority.to_owned(), identity.to_owned()))
            .collect::<Vec<_>>(),
        expected
            .prefix
            .iter()
            .map(|step| (
                step.document.authority.clone(),
                step.document.identity.clone()
            ))
            .collect::<Vec<_>>()
    );
    assert_eq!(record.basis(), "closed-scope");

    let bounded = round_trips(&report(finite(Unit::Bounded, "Bounded", &[0, 1, 2])));
    assert_eq!(
        temporal(&bounded)["counterexample"]["interval"],
        json!({"lower": "0", "upper": "1"})
    );
    assert_eq!(
        temporal(&bounded)["counterexample"]["prefix"]
            .as_array()
            .map(Vec::len),
        Some(3)
    );
}

/// FR-267-AC-14: each FR-330 report writes its row and reads back, and each
/// listed edit refuses naming the member.
#[trace("TC-1160", "FR-267-AC-14")]
#[trace("TC-1160", "FR-266-AC-4")]
#[test]
fn each_temporal_report_writes_its_row_and_each_edit_refuses() {
    let pending = report(finite(Unit::Infinite, "Reaches", &[0, 1, 2]));
    assert_eq!(pending.basis, SettlementBasis::Unsettled);
    let pending = round_trips(&pending);
    assert_eq!(temporal(&pending), &json!({"kind": "pending"}));
    assert_eq!(pending["disposition"]["category"], "pending");
    assert_eq!(pending["basis"], "unsettled");
    assert!(pending["disposition"].get("truth").is_none());
    assert!(pending["disposition"].get("outcome").is_none());
    refuses(
        &edited(&pending, |value| value["basis"] = json!("unavailable")),
        "basis",
    );

    let holds = report(lasso(Unit::Infinite, "Reaches", 0, &[0, 1, 2]));
    assert_eq!(holds.basis, SettlementBasis::ClosedScope);
    let holds = round_trips(&holds);
    assert_eq!(temporal(&holds), &json!({"kind": "holds"}));
    assert_eq!(holds["disposition"]["category"], "success");
    refuses(
        &edited(&holds, |value| {
            value["disposition"]["temporal"]["kind"] = json!("held");
        }),
        "disposition.temporal",
    );

    let mut starved = lasso(Unit::Infinite, "Reaches", 0, &[0, 1, 2]);
    starved.accounting = default_accounting(0);
    let starved = report(starved);
    assert_eq!(starved.basis, SettlementBasis::Unavailable);
    let incomplete = round_trips(&starved);
    assert_eq!(temporal(&incomplete), &json!({"kind": "incomplete"}));
    assert_eq!(incomplete["disposition"]["outcome"]["kind"], "incomplete");
    assert_eq!(
        incomplete["disposition"]["outcome"]["limit"], "work_units",
        "{incomplete}"
    );

    let undefined = report(finite(Unit::Undefined, "Undefined", &[0, 1, 2]));
    assert_eq!(undefined.basis, SettlementBasis::ClosedScope);
    let undefined = round_trips(&undefined);
    assert_eq!(temporal(&undefined)["kind"], "undefined");
    assert_eq!(temporal(&undefined)["position"], 2);
    assert_eq!(
        undefined["disposition"]["outcome"],
        json!({"kind": "undefined", "reason": "division-by-zero"})
    );
    let envelope = &temporal(&undefined)["counterexample"]["kind"];
    assert_eq!(envelope["type"], "undefined-evaluation");
    assert_eq!(envelope["cause"], "division-by-zero");
    assert_eq!(envelope["where"]["position"], 2);
    assert_eq!(
        temporal(&undefined)["counterexample"]["prefix"]
            .as_array()
            .map(Vec::len),
        Some(3)
    );
    assert_eq!(
        temporal(&undefined)["counterexample"]["kind"]["where"]["locus"]["formula_node"],
        json!(1),
        "the first undefined atom is the clause's `holds` node, under `always`"
    );

    let fair = report(lasso(Unit::Infinite, "Fair", 0, &[0, 1, 2]));
    let fair = round_trips(&fair);
    assert_eq!(temporal(&fair)["kind"], "missing-fairness-premise");
    assert_eq!(temporal(&fair)["constraint"]["kind"], "weak");
    assert_eq!(temporal(&fair)["constraint"]["granularity"], "whole");
    assert_eq!(
        fair["disposition"]["outcome"],
        json!({
            "kind": "refused",
            "code": "unsupported_projection",
            "cause": "missing-fairness-premise",
        })
    );

    let empty_loop = round_trips(&report(lasso(Unit::Infinite, "Reaches", 3, &[0, 1, 2])));
    assert_eq!(
        temporal(&empty_loop),
        &json!({"kind": "trace-refused", "variant": "empty-loop"})
    );
    assert_eq!(
        empty_loop["disposition"]["outcome"],
        json!({"kind": "refused", "code": "invalid_runtime_input", "cause": "invalid-value"})
    );
    assert!(empty_loop["disposition"].get("refusal").is_none());
    refuses(
        &edited(&empty_loop, |value| {
            value["disposition"]["temporal"]["variant"] = json!("empty-hoop");
        }),
        "disposition.temporal.variant",
    );
    refuses(
        &edited(&empty_loop, |value| {
            value["disposition"]["refusal"] =
                json!({"code": "invalid_runtime_input", "cause": "invalid-value"});
        }),
        "disposition.refusal",
    );

    // FR-330-AC-3's empty `Finite` report.
    let empty = report(finite(Unit::Infinite, "Reaches", &[]));
    assert!(matches!(
        empty.disposition,
        ClauseDisposition::TemporalRefusal(crate::spine::TemporalTraceRefusal::EmptyTrace)
    ));
    let empty = round_trips(&empty);
    assert_eq!(
        temporal(&empty),
        &json!({"kind": "trace-refused", "variant": "empty-trace"})
    );
    assert_eq!(
        empty["disposition"]["outcome"],
        json!({"kind": "refused", "code": "invalid_runtime_input", "cause": "invalid-value"})
    );

    // FR-109-AC-3's dangling-parent document is a stage `admit` refusal.
    let dangling = document(
        &run_clause(super::super::witness::parent_order_request(
            "dangling-parent",
            vec![super::super::frame::object("child", 2, Some("ghost"))],
            true,
        ))
        .expect("a well-formed request always reports"),
    );
    assert_eq!(dangling["disposition"]["stage"], "admit");
    read(&dangling).expect("the admit document reads");
    refuses(
        &edited(&dangling, |value| {
            value["disposition"]["temporal"] = json!({"kind": "holds"});
        }),
        "disposition.temporal",
    );

    let violated = document(&report(lasso(Unit::Infinite, "Reaches", 0, &[0, 1])));
    refuses(
        &edited(&violated, |value| {
            value["disposition"]["temporal"]
                .as_object_mut()
                .expect("an object")
                .remove("counterexample");
        }),
        "counterexample",
    );
    for (position, member) in [
        (json!(1), "trace_position"),
        (json!("0"), "trace_position"),
        (json!(-1), "trace_position"),
        (json!(0.5), "trace_position"),
        (json!(9_007_199_254_740_992_u64), "trace_position"),
    ] {
        refuses(
            &edited(&violated, |value| {
                value["disposition"]["temporal"]["counterexample"]["trace_position"] = position;
            }),
            member,
        );
    }
    refuses(
        &edited(&violated, |value| {
            value["witness"] = json!({"deciding_element": {"kind": "boolean", "value": true}});
        }),
        "witness",
    );
    refuses(
        &edited(&violated, |value| {
            value["disposition"]["refusal"] = json!({"code": "invalid_runtime_input"});
        }),
        "disposition.refusal",
    );
}

/// FR-267-AC-16: the undefined reason `insufficient-data` and the incomplete
/// limit `max_window_values` are read in their own rows only.
#[trace("TC-1160", "FR-267-AC-16")]
#[test]
fn temporal_only_outcomes_refuse_outside_their_rows() {
    let undefined = document(&report(finite(Unit::Undefined, "Undefined", &[0, 1, 2])));
    refuses(
        &edited(&undefined, |value| {
            value["disposition"]["outcome"]["reason"] = json!("insufficient-data");
        }),
        "disposition.outcome",
    );
    let mut starved = lasso(Unit::Infinite, "Reaches", 0, &[0, 1, 2]);
    starved.accounting = default_accounting(0);
    let incomplete = document(&report(starved));
    refuses(
        &edited(&incomplete, |value| {
            value["disposition"]["outcome"]["limit"] = json!("max_window_values");
        }),
        "disposition.outcome",
    );
    // Without `temporal` the document is a non-temporal clause run.
    refuses(
        &edited(&incomplete, |value| {
            value["disposition"]
                .as_object_mut()
                .expect("an object")
                .remove("temporal");
            value["disposition"]["outcome"]["limit"] = json!("max_window_values");
        }),
        "disposition.outcome.limit",
    );

    // FR-109-AC-3's exhausted-work report is a stage `evaluate` incomplete.
    let mut exhausted = super::super::witness::parent_order_request(
        "healthy-parent",
        vec![
            super::super::frame::object("root", 1, None),
            super::super::frame::object("child", 2, Some("root")),
        ],
        true,
    );
    exhausted.accounting = default_accounting(0);
    let exhausted = document(&run_clause(exhausted).expect("a well-formed request always reports"));
    assert_eq!(exhausted["disposition"]["stage"], "evaluate");
    assert_eq!(exhausted["disposition"]["outcome"]["kind"], "incomplete");
    read(&exhausted).expect("the exhausted document reads");
    refuses(
        &edited(&exhausted, |value| {
            value["disposition"]["temporal"] = json!({"kind": "holds"});
        }),
        "disposition.outcome",
    );
    refuses(
        &edited(&exhausted, |value| {
            value["disposition"]["outcome"]["limit"] = json!("max_window_values");
        }),
        "disposition.outcome.limit",
    );
    // An undefined outcome outside a temporal row, as FR-109-AC-7's
    // `precondition-false` report carries it.
    refuses(
        &edited(&exhausted, |value| {
            value["disposition"]["category"] = json!("undefined");
            value["disposition"]["outcome"] =
                json!({"kind": "undefined", "reason": "insufficient-data"});
        }),
        "disposition.outcome.reason",
    );
}

/// The `Window` clause over stamps 0, 2, 5 with position 1's `holds` atom
/// valuation absent (FR-501-AC-4).
fn missing_valuation(watermark: u64) -> ClauseRunReport {
    let mut request = timestamped(&[0, 2, 5], &[0, 2, 0], watermark, Some(5));
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

/// FR-285-AC-7: an `Indeterminate` run keeps the label `indeterminate` and
/// its reason in the machine output and exits 11, not 10 or 0.
#[trace("TC-769", "FR-285-AC-7")]
#[trace("TC-1160", "FR-267-AC-18")]
#[test]
fn an_indeterminate_run_keeps_its_label_and_reason_in_machine_output() {
    let run = missing_valuation(5);
    let TemporalVerdict::Indeterminate { at, reason } = outcome(&run) else {
        panic!("an indeterminate verdict, got {:?}", run.disposition);
    };
    assert_eq!((*at, exit_code(&run)), (0, 11));
    assert_eq!(run.basis, SettlementBasis::Unavailable);
    let value = round_trips(&run);
    assert_eq!(value["disposition"]["category"], "indeterminate");
    assert_eq!(value["basis"], "unavailable");
    assert!(value["disposition"].get("truth").is_none());
    assert!(value["disposition"].get("outcome").is_none());
    assert_eq!(
        temporal(&value),
        &json!({
            "kind": "indeterminate",
            "position": at,
            "reason": {
                "kind": "missing-valuation",
                "position": 1,
                "atom": "holds(c.value > 0)",
            },
        })
    );
    let record = read(&value).expect("the indeterminate document reads");
    assert_eq!(record.category(), "indeterminate");
    let Some(TemporalRecord::Indeterminate {
        position,
        reason: read_reason,
    }) = record.temporal()
    else {
        panic!("an indeterminate row");
    };
    assert_eq!(position, at);
    assert_eq!(&read_reason.typed(), reason);

    refuses(
        &edited(&value, |value| {
            value["disposition"]["category"] = json!("undefined");
        }),
        "disposition.category",
    );
    refuses(
        &edited(&value, |value| {
            value["disposition"]["temporal"]
                .as_object_mut()
                .expect("an object")
                .remove("reason");
        }),
        "reason",
    );
    refuses(
        &edited(&value, |value| {
            value["disposition"]["temporal"]["reason"]["source"] = json!("x");
        }),
        "source",
    );
    // Without `temporal` the label belongs to no row.
    refuses(
        &edited(&value, |value| {
            value["disposition"]
                .as_object_mut()
                .expect("an object")
                .remove("temporal");
        }),
        "disposition.category",
    );
}

/// A report holding `disposition`, built from a real run's report.
fn with_disposition(disposition: ClauseDisposition) -> ClauseRunReport {
    let mut report = missing_valuation(5);
    report.basis = match &disposition {
        ClauseDisposition::Temporal(verdict) => verdict.basis(),
        _ => SettlementBasis::Unavailable,
    };
    report.disposition = disposition;
    report.counterexample = None;
    report
}

/// FR-267-AC-18: each finite-window row writes and reads back to an equal
/// verdict; each listed edit refuses naming the member.
#[trace("TC-1160", "FR-267-AC-18")]
#[test]
fn finite_window_rows_round_trip_and_each_edit_refuses() {
    let pending = missing_valuation(3);
    assert_eq!(
        outcome(&pending),
        &TemporalVerdict::PendingWindow { at: 0, deadline: 5 }
    );
    let pending = round_trips(&pending);
    assert_eq!(
        temporal(&pending),
        &json!({"kind": "pending-window", "position": 0, "deadline": 5})
    );
    assert_eq!(pending["disposition"]["category"], "pending");
    assert_eq!(pending["basis"], "unsettled");
    refuses(
        &edited(&pending, |value| value["basis"] = json!("unavailable")),
        "basis",
    );

    let next_event = round_trips(&with_disposition(ClauseDisposition::Temporal(
        TemporalVerdict::PendingNextEvent { at: 3 },
    )));
    assert_eq!(
        temporal(&next_event),
        &json!({"kind": "pending-next-event", "position": 3})
    );

    let guard = quire_semantic_value::location::ExpressionLocation::at(
        quire_semantic_value::location::ExpressionOwner::StateClause {
            clause: "Window".to_owned(),
            index: 0,
        },
        &[1, 0],
    );
    let activation = round_trips(&with_disposition(ClauseDisposition::Temporal(
        TemporalVerdict::ActivationIncomplete { at: 1, guard },
    )));
    assert_eq!(temporal(&activation)["kind"], "activation-incomplete");
    assert_eq!(temporal(&activation)["position"], 1);
    assert_eq!(
        temporal(&activation)["guard"],
        json!({
            "origin": {"kind": "state-clause", "clause": "Window", "index": 0},
            "path": [1, 0],
        })
    );
    assert_eq!(activation["disposition"]["category"], "incomplete");

    let limited = round_trips(&with_disposition(ClauseDisposition::Temporal(
        TemporalVerdict::ProgressLimitReached(crate::spine::ProgressLimitReached {
            limit: "node count",
            value: 4,
            setting: "progress.covered_positions",
        }),
    )));
    assert_eq!(
        temporal(&limited),
        &json!({
            "kind": "progress-limit-reached",
            "limit": "node count",
            "value": 4,
            "setting": "progress.covered_positions",
        })
    );
    refuses(
        &edited(&limited, |value| {
            value["disposition"]["temporal"]["setting"] = json!("progress.positions");
        }),
        "disposition.temporal.limit",
    );

    let conflicting = TemporalIndeterminateReason::ConflictingValuation {
        position: 1,
        atom: "holds(c.value > 0)".to_owned(),
    };
    let instanced = TemporalIndeterminateReason::MissingValuation {
        position: 0,
        atom: "holds(c.value > 0)".to_owned(),
        instance: Some("c1".to_owned()),
        also_held_by: vec!["c2".to_owned()],
    };
    for reason in [
        conflicting,
        instanced,
        TemporalIndeterminateReason::IncompleteInput,
        TemporalIndeterminateReason::MissingHistory,
        TemporalIndeterminateReason::StraddlesBound,
    ] {
        let value = round_trips(&with_disposition(ClauseDisposition::Temporal(
            TemporalVerdict::Indeterminate {
                at: 0,
                reason: reason.clone(),
            },
        )));
        let record = read(&value).expect("indeterminate reads");
        let Some(TemporalRecord::Indeterminate {
            reason: read_reason,
            ..
        }) = record.temporal()
        else {
            panic!("an indeterminate row");
        };
        assert_eq!(read_reason.typed(), reason);
        if matches!(
            read_reason,
            IndeterminateReasonRead::MissingValuation { .. }
        ) {
            refuses(
                &edited(&value, |value| {
                    value["disposition"]["temporal"]["reason"]
                        .as_object_mut()
                        .expect("an object")
                        .remove("also_held_by");
                }),
                "also_held_by",
            );
        }
    }
}

/// FR-267-AC-18: FR-502-AC-1's watermark contradiction writes
/// `progress-contradicted` and reads back to an equal contradiction.
#[trace("TC-1160", "FR-267-AC-18")]
#[test]
fn a_progress_contradiction_round_trips_its_typed_dimensions() {
    let with_watermark = |watermark: u64, carried: &crate::spine::ProgressLedger| {
        let mut request = timestamped(&[0, 2, 5], &[0, 1, 0], 100, Some(100));
        timestamped_trace_mut(&mut request).progress.watermark = watermark;
        let ClauseRunSelection::Temporal { ledger, .. } = &mut request.selection else {
            panic!("a temporal selection");
        };
        *ledger = Some(carried.clone());
        request
    };
    let first = report(with_watermark(100, &crate::spine::ProgressLedger::new()));
    let retained = first.ledger.expect("an accepted request keeps its ledger");
    let refused = report(with_watermark(60, &retained));
    let ClauseDisposition::ProgressContradiction(refusal) = &refused.disposition else {
        panic!("a contradiction, got {:?}", refused.disposition);
    };
    let value = round_trips(&refused);
    assert_eq!(temporal(&value)["kind"], "progress-contradicted");
    assert_eq!(temporal(&value)["dimension"], "watermark");
    assert_eq!(
        temporal(&value)["retained"],
        json!({"kind": "watermark", "watermark": 100})
    );
    assert_eq!(
        temporal(&value)["asserted"],
        json!({"kind": "watermark", "watermark": 60})
    );
    assert_eq!(value["disposition"]["category"], "refusal");
    assert_eq!(
        value["disposition"]["outcome"]["code"],
        "invalid_runtime_input"
    );
    let record = read(&value).expect("the contradiction reads");
    assert_eq!(
        record.temporal().and_then(TemporalRecord::contradiction),
        Some(refusal.contradiction.clone())
    );
    refuses(
        &edited(&value, |value| {
            value["disposition"]["outcome"]["fields"]["asserted"] = json!("61");
        }),
        "disposition.outcome",
    );
}

/// FR-267-AC-15: the `insufficient-data`, `window-limit-reached`,
/// family-`undefined` and infinite-trace `violated` rows write and read back
/// to equal verdicts; dropping a listed member refuses naming it.
#[trace("TC-1160", "FR-267-AC-15")]
#[test]
fn insufficient_data_window_limit_and_family_undefined_rows_round_trip() {
    // A wire vector: FR-330 refuses a window aggregate over a model subject
    // at compile (`AggregateModelSubject`), so no clause run reaches it.
    let insufficient = round_trips(&with_disposition(ClauseDisposition::Temporal(
        TemporalVerdict::Undefined {
            at: 0,
            cause: crate::spine::TemporalUndefinedCause::InsufficientData {
                atom: 0,
                count: 1,
                min_count: 5,
            },
        },
    )));
    assert_eq!(
        temporal(&insufficient),
        &json!({"kind": "insufficient-data", "position": 0, "count": 1, "min_count": 5})
    );
    assert_eq!(insufficient["disposition"]["category"], "undefined");
    assert_eq!(insufficient["basis"], "unavailable");
    assert!(insufficient["disposition"].get("truth").is_none());
    assert_eq!(
        insufficient["disposition"]["outcome"],
        json!({"kind": "undefined", "reason": "insufficient-data"})
    );
    for member in ["count", "min_count"] {
        refuses(
            &edited(&insufficient, |value| {
                value["disposition"]["temporal"]
                    .as_object_mut()
                    .expect("an object")
                    .remove(member);
            }),
            member,
        );
    }
    refuses(
        &edited(&insufficient, |value| {
            value["disposition"]["temporal"]["atom"] = json!(0);
        }),
        "atom",
    );

    // A wire vector: a clause run has no window-limit input.
    let window = round_trips(&with_disposition(ClauseDisposition::Temporal(
        TemporalVerdict::WindowLimitReached {
            limit: "max_window_values",
            value: 2,
            setting: "monitor.max_window_values",
        },
    )));
    assert_eq!(
        temporal(&window),
        &json!({
            "kind": "window-limit-reached",
            "limit": "max_window_values",
            "value": 2,
            "setting": "monitor.max_window_values",
        })
    );
    assert_eq!(window["disposition"]["category"], "incomplete");
    assert_eq!(
        window["disposition"]["outcome"],
        json!({"kind": "incomplete", "limit": "max_window_values"})
    );
    for member in ["limit", "value", "setting"] {
        refuses(
            &edited(&window, |value| {
                value["disposition"]["temporal"]
                    .as_object_mut()
                    .expect("an object")
                    .remove(member);
            }),
            member,
        );
    }

    // A wire vector: FR-330-AC-5's run holding the family cause
    // `absent-key` (the `Counter` unit's one population holds every `over`
    // object, so no `lookup` of it is absent).
    let mut absent = report(finite(Unit::Undefined, "Undefined", &[0, 1, 2]));
    let record = quire_semantic_value::catalog::UndefinedRecord {
        reason: quire_semantic_value::catalog::UndefinedReason::AbsentKey,
        fields: std::collections::BTreeMap::new(),
    };
    absent.disposition = ClauseDisposition::Temporal(TemporalVerdict::Undefined {
        at: 2,
        cause: crate::spine::TemporalUndefinedCause::Family(record.clone()),
    });
    let Some(crate::ObservedTemporalKind::UndefinedEvaluation { cause, .. }) = absent
        .counterexample
        .as_mut()
        .map(|counterexample| &mut counterexample.kind)
    else {
        panic!("an undefined-evaluation counterexample");
    };
    *cause = qsl_eval::temporal::UndefinedCause::Family(record);
    let absent = round_trips(&absent);
    assert_eq!(
        absent["disposition"]["outcome"],
        json!({"kind": "undefined", "reason": "absent-key"})
    );
    assert_eq!(
        temporal(&absent)["counterexample"]["kind"]["cause"],
        "absent-key"
    );
    refuses(
        &edited(&absent, |value| {
            value["disposition"]["outcome"]["reason"] = json!("absent-value");
            value["disposition"]["temporal"]["counterexample"]["kind"]["cause"] =
                json!("absent-value");
        }),
        "disposition.outcome.reason",
    );

    // FR-328-AC-1: a bad prefix of an infinite-trace clause with no bounded
    // operator.
    let safety = report(finite(Unit::Safety, "Safety", &[0, 1, 2]));
    assert_eq!(outcome(&safety), &TemporalVerdict::Violated { at: 0 });
    let safety = round_trips(&safety);
    assert_eq!(temporal(&safety)["position"], 0);
    let counterexample = &temporal(&safety)["counterexample"];
    assert_eq!(counterexample["prefix"].as_array().map(Vec::len), Some(3));
    assert_eq!(counterexample["loop"], json!([]));
    assert_eq!(counterexample["interval"], Value::Null);
}

/// FR-267 (QSL-1566 review): an `Undefined` raised by an undefined `on each`
/// guard is written with the `guard` locus and reads back to an equal
/// verdict; its envelope names no formula node.
#[trace("TC-1160", "FR-267-AC-14")]
#[test]
fn an_undefined_on_each_guard_writes_its_guard_locus() {
    let guarded = report(finite(Unit::GuardUndefined, "GuardUndefined", &[2, 0, 1]));
    assert!(
        matches!(
            outcome(&guarded),
            TemporalVerdict::Undefined {
                at: 0,
                cause: crate::spine::TemporalUndefinedCause::Kernel(_)
            }
        ),
        "{:?}",
        guarded.disposition
    );
    let value = round_trips(&guarded);
    assert_eq!(temporal(&value)["kind"], "undefined");
    let envelope = &temporal(&value)["counterexample"]["kind"];
    assert_eq!(envelope["where"]["locus"], json!({"guard": "on-each"}));
    assert_eq!(envelope["cause"], "division-by-zero");
    assert_eq!(
        temporal(&value)["counterexample"]["prefix"]
            .as_array()
            .map(Vec::len),
        Some(1)
    );
    let record = read(&value).expect("the guard document reads");
    let Some(TemporalRecord::Undefined { counterexample, .. }) = record.temporal() else {
        panic!("an undefined row");
    };
    assert_eq!(
        counterexample.undefined_locus(),
        Some(&crate::spine::UndefinedLocusRead::Guard(
            crate::spine::GuardRead::OnEach
        ))
    );
    refuses(
        &edited(&value, |value| {
            value["disposition"]["temporal"]["counterexample"]["kind"]["where"]["locus"] =
                json!({"guard": "on-each", "formula_node": 0});
        }),
        "disposition.temporal.counterexample.kind",
    );
}
