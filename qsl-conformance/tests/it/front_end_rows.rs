// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1388 and TC-1389: the `check` and `package` rows.

use ix_trace_rs::trace;
use qsl_conformance::{
    run_case, CaseInput, FrontEndInput, FrontEndLimit, MatrixCase, OutcomeKind, Verb,
};
use quire_semantic_value::catalog::Category;

use crate::support::{case_of, diagnostic, outcome_of, written};

/// A cancel before the call is incomplete at no stage, with the cancel's
/// cause as the diagnostic's.
fn assert_cancelled(verb: Verb, kind: OutcomeKind, cause: &str, operation: &str) {
    let outcome = outcome_of(verb, kind);
    assert_eq!(outcome.exit_code(), 22, "{verb:?}/{kind:?}");
    let document = written(&outcome);
    assert_eq!(document["operation"], operation);
    assert_eq!(document["category"], "incomplete");
    assert_eq!(document["last_stage"], serde_json::Value::Null);
    assert_eq!(
        diagnostic(&outcome),
        ("cancelled".to_owned(), Some(cause.to_owned()))
    );
}

/// FR-564-AC-1: each of `check`'s six applicable cells returns its kind.
#[trace("TC-1388", "FR-564-AC-1")]
#[test]
fn check_returns_each_applicable_kind() {
    let success = outcome_of(Verb::Check, OutcomeKind::Success);
    assert_eq!(success.exit_code(), 0);
    assert_eq!(written(&success)["category"], "success");

    let refusal = outcome_of(Verb::Check, OutcomeKind::Refusal);
    assert_eq!(refusal.exit_code(), 20);
    assert_eq!(diagnostic(&refusal).0, "ill_typed");

    let unsupported = outcome_of(Verb::Check, OutcomeKind::Unsupported);
    assert_eq!(unsupported.exit_code(), 21);
    assert_eq!(
        diagnostic(&unsupported),
        (
            "unsupported_construct".to_owned(),
            Some("declaration-form".to_owned())
        )
    );

    assert_cancelled(Verb::Check, OutcomeKind::Timeout, "deadline", "check");
    assert_cancelled(Verb::Check, OutcomeKind::Cancellation, "requested", "check");

    let incomplete = outcome_of(Verb::Check, OutcomeKind::Incomplete);
    assert_eq!(incomplete.exit_code(), 22);
    assert_eq!(written(&incomplete)["category"], "incomplete");
    assert_eq!(diagnostic(&incomplete).0, "stage_limit_exceeded");
    assert!(
        crate::support::document(&incomplete).diagnostics()[0]
            .message()
            .contains("s3.nodes"),
        "{:?}",
        crate::support::document(&incomplete).diagnostics()
    );
}

/// FR-564-AC-2: each of `package`'s five applicable cells returns its kind.
#[trace("TC-1389", "FR-564-AC-2")]
#[test]
fn package_returns_each_applicable_kind() {
    let success = outcome_of(Verb::Package, OutcomeKind::Success);
    assert_eq!(success.exit_code(), 0);
    assert_eq!(written(&success)["operation"], "package");

    let unsupported = outcome_of(Verb::Package, OutcomeKind::Unsupported);
    assert_eq!(unsupported.exit_code(), 21);
    assert_eq!(diagnostic(&unsupported).0, "unsupported_construct");
    assert_eq!(
        crate::support::document(&unsupported).category(),
        Category::Unsupported
    );

    assert_cancelled(Verb::Package, OutcomeKind::Timeout, "deadline", "package");
    assert_cancelled(
        Verb::Package,
        OutcomeKind::Cancellation,
        "requested",
        "package",
    );

    let incomplete = outcome_of(Verb::Package, OutcomeKind::Incomplete);
    assert_eq!(incomplete.exit_code(), 22);
    assert_eq!(diagnostic(&incomplete).0, "stage_limit_exceeded");
    assert!(crate::support::document(&incomplete).diagnostics()[0]
        .message()
        .contains("emit.output_bytes"));
}

/// The incomplete cells sit exactly one below the counter their input
/// reaches: one above, the same operation succeeds.
#[trace("TC-1388", "FR-564-AC-1")]
#[trace("TC-1389", "FR-564-AC-2")]
#[test]
fn the_incomplete_cells_are_one_below_the_counter_their_input_reaches() {
    let at_counter = |verb, limit_at_counter: fn(u64) -> FrontEndLimit, below: u64| {
        let case = case_of(verb, OutcomeKind::Incomplete);
        let (CaseInput::Check(input) | CaseInput::Package(input)) = case.input else {
            panic!("a front-end case");
        };
        let (FrontEndLimit::S3Nodes(stated) | FrontEndLimit::OutputBytes(stated)) = input.limit
        else {
            panic!("a limited case");
        };
        assert_eq!(stated, below);
        let wrap = |input| match case.input {
            CaseInput::Check(_) => CaseInput::Check(input),
            _ => CaseInput::Package(input),
        };
        let counter = MatrixCase {
            input: wrap(FrontEndInput {
                limit: limit_at_counter(below + 1),
                ..input
            }),
            ..case
        };
        let outcome = run_case(&counter).expect("the case runs");
        assert_eq!(outcome.exit_code(), 0, "{verb:?}");
    };
    at_counter(Verb::Check, FrontEndLimit::S3Nodes, 10);
    at_counter(Verb::Package, FrontEndLimit::OutputBytes, 20_301);
}
