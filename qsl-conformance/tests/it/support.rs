// SPDX-License-Identifier: AGPL-3.0-or-later
//! Helpers the row tests share.

use qsl_conformance::{
    lifecycle_matrix, run_case, CaseOutcome, MatrixCase, MatrixCell, OutcomeKind, Report, Verb,
};
use qsl_replay::OutcomeDocument;

/// The case of the cell `(verb, kind)`.
pub fn case_of(verb: Verb, kind: OutcomeKind) -> MatrixCase {
    lifecycle_matrix()
        .iter()
        .find_map(|cell| match cell {
            MatrixCell::Case(case) if case.verb == verb && case.kind == kind => Some(*case),
            _ => None,
        })
        .unwrap_or_else(|| panic!("the matrix holds a case for {verb:?}/{kind:?}"))
}

/// The outcome of the case `(verb, kind)`.
pub fn outcome_of(verb: Verb, kind: OutcomeKind) -> CaseOutcome {
    run_case(&case_of(verb, kind)).unwrap_or_else(|error| panic!("{verb:?}/{kind:?}: {error}"))
}

/// The document of an outcome that is one.
pub fn document(outcome: &CaseOutcome) -> &OutcomeDocument {
    match outcome.report() {
        Report::Document(document) => document,
        Report::Item(item) => panic!("expected a document, got the item {item:?}"),
    }
}

/// The written form of an outcome.
pub fn written(outcome: &CaseOutcome) -> serde_json::Value {
    match outcome.report() {
        Report::Document(document) => serde_json::to_value(document),
        Report::Item(item) => serde_json::to_value(item),
    }
    .expect("an outcome is JSON")
}

/// The catalog code and cause of the document's one diagnostic.
pub fn diagnostic(outcome: &CaseOutcome) -> (String, Option<String>) {
    let [diagnostic] = document(outcome).diagnostics() else {
        panic!("one diagnostic: {:?}", document(outcome).diagnostics());
    };
    (
        diagnostic.code().to_owned(),
        diagnostic.cause().map(str::to_owned),
    )
}
