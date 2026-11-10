// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1390: the `execute` row.

use ix_trace_rs::trace;
use qsl_conformance::{run_case, CaseOutcome, OutcomeKind, Verb};
use quire_eval::{CallFailure, FamilyOutcome, UnsupportedCause};
use quire_exact::{CancelCause, Incomplete, LimitKind, Outcome};

use crate::support::{case_of, diagnostic, outcome_of, written};

/// The typed `CallFailure` of an `execute` case, which must have failed.
fn failure_of(outcome: &CaseOutcome) -> &CallFailure {
    outcome
        .execution()
        .expect("an execute case holds its typed result")
        .result()
        .as_ref()
        .expect_err("the cell's call fails")
}

/// The kernel's actual incomplete arm, rather than its Debug rendering.
fn incomplete_of(outcome: &CaseOutcome) -> &Incomplete {
    let evaluated = outcome
        .execution()
        .expect("typed result")
        .result()
        .as_ref()
        .expect("an incomplete evaluation is not a call failure");
    match &evaluated.outcome {
        FamilyOutcome::Evaluated(Outcome::Incomplete(limit)) => limit,
        other => panic!("expected the kernel incomplete arm, got {other:?}"),
    }
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
    assert_eq!(incomplete_of(&incomplete).limit_kind, LimitKind::WorkUnits);

    let fault = outcome_of(Verb::Execute, OutcomeKind::ToolFailure);
    assert_eq!(fault.exit_code(), 30);
    assert!(matches!(failure_of(&fault), CallFailure::Fault(_)));
    assert!(written(&fault)
        .get("result")
        .is_none_or(|result| result.is_null()));
}

/// A non-work counter supplied by the case reaches both execution paths.
#[trace("TC-1390")]
#[test]
fn execute_preserves_the_cases_non_work_accounting_limit() {
    let mut case = case_of(Verb::Execute, OutcomeKind::Success);
    let unlimited = run_case(&case).expect("the original case runs");
    assert_eq!(unlimited.exit_code(), 0);
    assert_eq!(written(&unlimited)["result"]["value"]["decimal"], "7");

    case.parameters.accounting.integer_bits = 0;
    let limited = run_case(&case).expect("the limited case runs");
    assert_eq!(limited.exit_code(), 22);
    let native = incomplete_of(&limited);
    assert_eq!(native.limit_kind, LimitKind::IntegerBits);
    assert_eq!(native.limit, 0);
    let document = written(&limited);
    assert_eq!(document["result"]["kind"], "incomplete");
    assert_eq!(document["result"]["limit"]["field"], "integer_bits");
    assert_eq!(document["result"]["limit"]["bound"], "0");
}
