// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1390: the `execute` row.

use ix_trace_rs::trace;
use qsl_conformance::{CaseOutcome, OutcomeKind, Verb};
use quire_eval::{CallFailure, UnsupportedCause};
use quire_exact::CancelCause;

use crate::support::{diagnostic, outcome_of, written};

/// The typed `CallFailure` of an `execute` case, which must have failed.
fn failure_of(outcome: &CaseOutcome) -> &CallFailure {
    outcome
        .execution()
        .expect("an execute case holds its typed result")
        .result()
        .as_ref()
        .expect_err("the cell's call fails")
}

/// FR-564-AC-3: each of `execute`'s seven applicable cells returns its kind,
/// as the typed `execute` result and as the FR-286 document.
#[trace("TC-1390", "FR-564-AC-3")]
#[test]
fn execute_returns_each_applicable_kind() {
    let success = outcome_of(Verb::Execute, OutcomeKind::Success);
    assert_eq!(success.exit_code(), 0);
    assert!(success.execution().expect("typed result").result().is_ok());
    let written_success = written(&success);
    assert_eq!(written_success["operation"], "execute");
    assert_eq!(written_success["result"]["kind"], "completed");
    assert_eq!(written_success["result"]["value"]["decimal"], "7");

    let refusal = outcome_of(Verb::Execute, OutcomeKind::Refusal);
    assert_eq!(refusal.exit_code(), 20);
    assert!(matches!(failure_of(&refusal), CallFailure::Input(_)));
    assert_eq!(written(&refusal)["category"], "refusal");

    let unsupported = outcome_of(Verb::Execute, OutcomeKind::Unsupported);
    assert_eq!(unsupported.exit_code(), 21);
    assert!(
        matches!(
            failure_of(&unsupported),
            CallFailure::Unsupported(entry) if entry.cause == UnsupportedCause::NotSupplied
        ),
        "{:?}",
        failure_of(&unsupported)
    );
    assert_eq!(written(&unsupported)["category"], "unsupported");

    for (kind, cause, source) in [
        (OutcomeKind::Timeout, CancelCause::Deadline, "deadline"),
        (
            OutcomeKind::Cancellation,
            CancelCause::Requested,
            "requested",
        ),
    ] {
        let cancelled = outcome_of(Verb::Execute, kind);
        assert_eq!(cancelled.exit_code(), 22, "{kind:?}");
        assert!(
            matches!(failure_of(&cancelled), CallFailure::Cancelled(got) if *got == cause),
            "{kind:?}: {:?}",
            failure_of(&cancelled)
        );
        assert_eq!(written(&cancelled)["category"], "incomplete");
        assert_eq!(
            diagnostic(&cancelled),
            ("cancelled".to_owned(), Some(source.to_owned())),
            "{kind:?}"
        );
    }

    let incomplete = outcome_of(Verb::Execute, OutcomeKind::Incomplete);
    assert_eq!(incomplete.exit_code(), 22);
    let incomplete_document = written(&incomplete);
    assert_eq!(incomplete_document["result"]["kind"], "incomplete");
    assert_eq!(
        incomplete_document["result"]["limit"]["field"],
        "work_units"
    );
    assert!(
        format!(
            "{:?}",
            incomplete.execution().expect("typed result").result()
        )
        .contains("limit_kind: WorkUnits"),
        "{:?}",
        incomplete.execution().expect("typed result").result()
    );

    let fault = outcome_of(Verb::Execute, OutcomeKind::ToolFailure);
    assert_eq!(fault.exit_code(), 30);
    assert!(matches!(failure_of(&fault), CallFailure::Fault(_)));
    assert!(written(&fault)
        .get("result")
        .is_none_or(|result| result.is_null()));
}
