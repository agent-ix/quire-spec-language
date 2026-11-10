// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1391: the `analyze` row.

use ix_trace_rs::trace;
use qsl_conformance::{OutcomeKind, Verb};
use qsl_foundation::RequestIndex;
use qsl_replay::{TerminalValue, UnavailabilityCause};
use quire_semantic_value::catalog::CatalogCode;

use crate::support::{diagnostic, outcome_of, written};

/// The one backend the case's one record names.
fn backend_of(outcome: &qsl_conformance::CaseOutcome) -> String {
    let [record] = outcome.records() else {
        panic!("one record: {:?}", outcome.records());
    };
    record
        .backend()
        .map(|id| id.as_str().to_owned())
        .expect("a named engine")
}

/// Each of `analyze`'s applicable cells returns its kind: the EN-5 item
/// `proved`, the EN-1 item `refuted` and `incomplete` under `max_states` 1,
/// the binding refusal, EN-1's `RequiresBound` refusal, and the cancelled EN-4
/// item. FR-564-AC-4 is not tagged here: its `ItemCounterexample` for the
/// `refuted` cell is not produced yet, so the AC stays open.
#[trace("TC-1391")]
#[test]
fn analyze_returns_each_reachable_kind() {
    let proved = outcome_of(Verb::Analyze, OutcomeKind::Success);
    assert_eq!(proved.exit_code(), 0);
    let item = &written(&proved)["items"][0];
    assert_eq!(item["result"], "proved");
    assert_eq!(backend_of(&proved), "en-5");

    let refuted = outcome_of(Verb::Analyze, OutcomeKind::Violation);
    assert_eq!(refuted.exit_code(), 10);
    let item = &written(&refuted)["items"][0];
    assert_eq!(item["result"], "refuted");
    assert_eq!(backend_of(&refuted), "en-1");

    let incomplete = outcome_of(Verb::Analyze, OutcomeKind::Incomplete);
    assert_eq!(incomplete.exit_code(), 22);
    let item = &written(&incomplete)["items"][0];
    assert_eq!(item["result"], "incomplete");
    assert_eq!(backend_of(&incomplete), "en-1");
    assert_eq!(item["cause"]["limit"], "max_states");

    let refusal = outcome_of(Verb::Analyze, OutcomeKind::Refusal);
    assert_eq!(refusal.exit_code(), 20);
    assert_eq!(written(&refusal)["items"], serde_json::json!([]));
    assert_eq!(
        diagnostic(&refusal),
        (
            "invalid_runtime_input".to_owned(),
            Some("wrong-role-mapping".to_owned())
        )
    );

    let unsupported = outcome_of(Verb::Analyze, OutcomeKind::Unsupported);
    assert_eq!(unsupported.exit_code(), 21);
    let [record] = unsupported.records() else {
        panic!("one EN-1 refusal record: {:?}", unsupported.records());
    };
    assert_eq!(record.request_index(), RequestIndex::new(0));
    assert_eq!(backend_of(&unsupported), "en-1");
    assert_eq!(
        record.value(),
        &TerminalValue::Unsupported(UnavailabilityCause::Disposition)
    );
    assert_eq!(
        record.diagnostic(),
        Some(CatalogCode::new(
            "unsupported_projection",
            "unbounded-extent"
        ))
    );
    let detail = serde_json::json!({
        "kind": "temporal-satisfaction",
        "candidate": "en-1",
        "modes": ["bounded", "unbounded"],
    });
    assert_eq!(record.disposition_detail(), detail.as_object());
    // FR-281-AC-11 and FR-286: the engine refusal preserves its diagnostic
    // payload in the item and writes no cause member.
    let item = &written(&unsupported)["items"][0];
    assert_eq!(item["result"], "unsupported");
    assert!(item.get("cause").is_none(), "{item}");
    assert_eq!(item["diagnostic"]["code"], "unsupported_projection");
    assert_eq!(item["diagnostic"]["cause"], "unbounded-extent");
    assert_eq!(item["diagnostic"]["detail"], detail);
    assert!(written(&unsupported)["diagnostics"]
        .as_array()
        .is_none_or(Vec::is_empty));

    for (kind, source) in [
        (OutcomeKind::Timeout, "deadline"),
        (OutcomeKind::Cancellation, "requested"),
    ] {
        let outcome = outcome_of(Verb::Analyze, kind);
        assert_eq!(outcome.exit_code(), 22);
        let item = &written(&outcome)["items"][0];
        assert_eq!(item["result"], "incomplete");
        assert_eq!(
            item["cause"],
            serde_json::json!({"kind": "cancelled", "source": source})
        );
    }
}
