// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1392: the `monitor` row.

use ix_trace_rs::trace;
use qsl_conformance::{OutcomeKind, Verb};

use crate::support::{diagnostic, outcome_of, written};

fn item(kind: OutcomeKind) -> (u8, serde_json::Value) {
    let outcome = outcome_of(Verb::Monitor, kind);
    let document = written(&outcome);
    assert_eq!(document["operation"], "monitor");
    (outcome.exit_code(), document["items"][0].clone())
}

/// FR-564-AC-5: each of `monitor`'s seven applicable cells returns its kind.
#[trace("TC-1392", "FR-564-AC-5")]
#[test]
fn monitor_returns_each_applicable_kind() {
    let (exit, tested) = item(OutcomeKind::Success);
    assert_eq!((exit, tested["result"].as_str()), (0, Some("tested")));

    let (exit, violated) = item(OutcomeKind::Violation);
    assert_eq!((exit, violated["result"].as_str()), (10, Some("refuted")));
    assert_eq!(violated["counterexample"]["trace_position"], 0);

    let refusal = outcome_of(Verb::Monitor, OutcomeKind::Refusal);
    assert_eq!(refusal.exit_code(), 20);
    assert_eq!(written(&refusal)["items"], serde_json::json!([]));
    assert_eq!(
        diagnostic(&refusal),
        (
            "invalid_runtime_input".to_owned(),
            Some("invalid-value".to_owned())
        )
    );

    let (exit, unsupported) = item(OutcomeKind::Unsupported);
    assert_eq!(
        (exit, unsupported["result"].as_str()),
        (21, Some("unsupported"))
    );
    assert_eq!(unsupported["cause"]["kind"], "missing-fairness-premise");

    for (kind, cause) in [
        (OutcomeKind::Timeout, "deadline"),
        (OutcomeKind::Cancellation, "requested"),
    ] {
        let outcome = outcome_of(Verb::Monitor, kind);
        assert_eq!(outcome.exit_code(), 22);
        assert_eq!(written(&outcome)["last_stage"], serde_json::Value::Null);
        assert_eq!(
            diagnostic(&outcome),
            ("cancelled".to_owned(), Some(cause.to_owned()))
        );
    }

    let (exit, incomplete) = item(OutcomeKind::Incomplete);
    assert_eq!(
        (exit, incomplete["result"].as_str()),
        (22, Some("incomplete"))
    );
    assert_eq!(incomplete["cause"]["limit"], "work_units");
}
