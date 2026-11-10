// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1393: the QSL half of `prove` yields an item of each kind.

use ix_trace_rs::trace;
use qsl_conformance::{OutcomeKind, Report, Verb};
use qsl_foundation::RequestIndex;
use qsl_replay::{OutcomeItem, TerminalValue, UnavailabilityCause};
use quire_semantic_value::catalog::CatalogCode;

use crate::support::{outcome_of, written};

/// Each applicable cell of `prove` (QSL half) yields one FR-286 item of its
/// kind. FR-564-AC-6 is not tagged here: the `refuted` item's counterexample
/// is not produced yet, so the AC stays open.
#[trace("TC-1393")]
#[test]
fn prove_yields_an_item_of_each_reachable_kind() {
    for (kind, exit, result) in [
        (OutcomeKind::Success, 0, "proved"),
        (OutcomeKind::Violation, 10, "refuted"),
        (OutcomeKind::Unsupported, 21, "unsupported"),
        (OutcomeKind::Incomplete, 22, "incomplete"),
    ] {
        let outcome = outcome_of(Verb::Prove, kind);
        assert!(matches!(outcome.report(), Report::Item(_)), "{kind:?}");
        assert_eq!(outcome.exit_code(), exit, "{kind:?}");
        assert_eq!(written(&outcome)["result"], result, "{kind:?}");
    }
    let unsupported_outcome = outcome_of(Verb::Prove, OutcomeKind::Unsupported);
    let [record] = unsupported_outcome.records() else {
        panic!("one EN-1 refusal record: {:?}", unsupported_outcome.records());
    };
    assert_eq!(record.request_index(), RequestIndex::new(0));
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
    let Report::Item(item) = unsupported_outcome.report() else {
        panic!("prove returns an item");
    };
    assert_eq!(item.as_ref(), &OutcomeItem::from_terminal(record));

    let unsupported = written(&unsupported_outcome);
    assert_eq!(unsupported_outcome.exit_code(), 21);
    assert!(unsupported.get("cause").is_none(), "{unsupported}");
    assert_eq!(unsupported["diagnostic"]["code"], "unsupported_projection");
    assert_eq!(unsupported["diagnostic"]["cause"], "unbounded-extent");
    assert_eq!(unsupported["diagnostic"]["detail"], detail);
    let incomplete = written(&outcome_of(Verb::Prove, OutcomeKind::Incomplete));
    assert_eq!(incomplete["cause"]["limit"], "max_states");

    for (kind, exit, result, cause) in [
        (
            OutcomeKind::Refusal,
            20,
            "declined",
            serde_json::json!({"kind": "invalid-input", "code": "invalid_runtime_input"}),
        ),
        (
            OutcomeKind::Timeout,
            22,
            "incomplete",
            serde_json::json!({"kind": "timed-out"}),
        ),
        (
            OutcomeKind::Cancellation,
            22,
            "incomplete",
            serde_json::json!({"kind": "cancelled", "source": "requested"}),
        ),
        (
            OutcomeKind::ToolFailure,
            30,
            "failed",
            serde_json::json!({"kind": "tool-failure", "defect": "crash"}),
        ),
    ] {
        let outcome = outcome_of(Verb::Prove, kind);
        assert!(matches!(outcome.report(), Report::Item(_)), "{kind:?}");
        assert_eq!(outcome.exit_code(), exit, "{kind:?}");
        let item = written(&outcome);
        assert_eq!(item["result"], result, "{kind:?}");
        assert_eq!(item["cause"], cause, "{kind:?}");
    }
}
