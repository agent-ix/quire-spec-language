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
    let value = document(&violation);
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

    let bounded = document(&report(finite(Unit::Bounded, "Bounded", &[0, 1, 2])));
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
    read(&bounded).expect("the bounded violation reads");
}

/// FR-267-AC-14: each FR-330 report writes its row and reads back, and each
/// listed edit refuses naming the member.
#[trace("TC-1160", "FR-267-AC-14")]
#[trace("TC-1160", "FR-266-AC-4")]
#[test]
fn each_temporal_report_writes_its_row_and_each_edit_refuses() {
    let pending = report(finite(Unit::Infinite, "Reaches", &[0, 1, 2]));
    assert_eq!(pending.basis, SettlementBasis::Unsettled);
    let pending = document(&pending);
    assert_eq!(temporal(&pending), &json!({"kind": "pending"}));
    assert_eq!(pending["disposition"]["category"], "pending");
    assert_eq!(pending["basis"], "unsettled");
    assert!(pending["disposition"].get("truth").is_none());
    assert!(pending["disposition"].get("outcome").is_none());
    read(&pending).expect("pending reads");
    refuses(
        &edited(&pending, |value| value["basis"] = json!("unavailable")),
        "basis",
    );

    let holds = report(lasso(Unit::Infinite, "Reaches", 0, &[0, 1, 2]));
    assert_eq!(holds.basis, SettlementBasis::ClosedScope);
    let holds = document(&holds);
    assert_eq!(temporal(&holds), &json!({"kind": "holds"}));
    assert_eq!(holds["disposition"]["category"], "success");
    read(&holds).expect("holds reads");
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
    let incomplete = document(&starved);
    assert_eq!(temporal(&incomplete), &json!({"kind": "incomplete"}));
    assert_eq!(incomplete["disposition"]["outcome"]["kind"], "incomplete");
    assert_eq!(
        incomplete["disposition"]["outcome"]["limit"], "work_units",
        "{incomplete}"
    );
    read(&incomplete).expect("incomplete reads");

    let undefined = report(finite(Unit::Undefined, "Undefined", &[0, 1, 2]));
    assert_eq!(undefined.basis, SettlementBasis::ClosedScope);
    let undefined = document(&undefined);
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
    read(&undefined).expect("undefined reads");

    let fair = report(lasso(Unit::Infinite, "Fair", 0, &[0, 1, 2]));
    let fair = document(&fair);
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
    read(&fair).expect("missing premise reads");

    let empty_loop = document(&report(lasso(Unit::Infinite, "Reaches", 3, &[0, 1, 2])));
    assert_eq!(
        temporal(&empty_loop),
        &json!({"kind": "trace-refused", "variant": "empty-loop"})
    );
    assert_eq!(
        empty_loop["disposition"]["outcome"],
        json!({"kind": "refused", "code": "invalid_runtime_input", "cause": "invalid-value"})
    );
    assert!(empty_loop["disposition"].get("refusal").is_none());
    let record = read(&empty_loop).expect("empty loop reads");
    assert!(matches!(
        record.temporal(),
        Some(TemporalRecord::TraceRefused { variant, .. }) if variant == "empty-loop"
    ));
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
    assert_eq!(exit_code(&run), 11);
    assert_eq!(run.basis, SettlementBasis::Unavailable);
    let value = document(&run);
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
    let pending = document(&pending);
    assert_eq!(
        temporal(&pending),
        &json!({"kind": "pending-window", "position": 0, "deadline": 5})
    );
    assert_eq!(pending["disposition"]["category"], "pending");
    assert_eq!(pending["basis"], "unsettled");
    read(&pending).expect("pending-window reads");
    refuses(
        &edited(&pending, |value| value["basis"] = json!("unavailable")),
        "basis",
    );

    let next_event = document(&with_disposition(ClauseDisposition::Temporal(
        TemporalVerdict::PendingNextEvent { at: 3 },
    )));
    assert_eq!(
        temporal(&next_event),
        &json!({"kind": "pending-next-event", "position": 3})
    );
    read(&next_event).expect("pending-next-event reads");

    let guard = quire_semantic_value::location::Location::root(
        quire_semantic_value::location::Origin::Expression,
    );
    let activation = document(&with_disposition(ClauseDisposition::Temporal(
        TemporalVerdict::ActivationIncomplete { at: 1, guard },
    )));
    assert_eq!(temporal(&activation)["kind"], "activation-incomplete");
    assert_eq!(temporal(&activation)["position"], 1);
    assert_eq!(
        temporal(&activation)["guard"]["origin"],
        json!({"kind": "expression"})
    );
    assert_eq!(activation["disposition"]["category"], "incomplete");
    read(&activation).expect("activation-incomplete reads");

    let limited = document(&with_disposition(ClauseDisposition::Temporal(
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
    read(&limited).expect("progress-limit-reached reads");

    let window = document(&with_disposition(ClauseDisposition::Temporal(
        TemporalVerdict::WindowLimitReached {
            limit: "max_window_values",
            value: 2,
            setting: "monitor.max_window_values",
        },
    )));
    assert_eq!(
        window["disposition"]["outcome"],
        json!({"kind": "incomplete", "limit": "max_window_values"})
    );
    read(&window).expect("window-limit-reached reads");

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
        let value = document(&with_disposition(ClauseDisposition::Temporal(
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
    let value = document(&refused);
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
