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

/// The one `ItemCounterexample` of a `refuted` EN-1 case, asserted to be the
/// formula counterexample of the one record, and the item written with it in
/// FR-364's wire form (FR-286 `from_analyze`).
pub fn assert_one_formula_counterexample(outcome: &CaseOutcome) {
    use qsl_analyze::analyze::Counterexample;
    use qsl_replay::{CounterexampleKind, OutcomeItem, OutcomeItemError};

    let [record] = outcome.records() else {
        panic!("one record: {:?}", outcome.records());
    };
    let [found] = outcome.counterexamples() else {
        panic!("one counterexample: {:?}", outcome.counterexamples());
    };
    assert_eq!(found.request_index, record.request_index());
    assert_eq!(Some(&found.backend), record.backend());
    assert_eq!(found.backend.as_str(), "en-1");
    let Counterexample::Temporal {
        counterexample: temporal,
        ..
    } = &found.counterexample
    else {
        panic!("a temporal counterexample: {found:?}");
    };
    assert!(matches!(temporal.kind, CounterexampleKind::Formula));
    assert!(
        !temporal.prefix.is_empty(),
        "the violation is reached by steps"
    );
    // A refuted record without its counterexample is no item.
    assert_eq!(
        OutcomeItem::from_analyze(record, None, None).unwrap_err(),
        OutcomeItemError::MissingCounterexample
    );
    // The item written carries the counterexample in FR-364's wire form.
    let item = match outcome.report() {
        Report::Item(item) => serde_json::to_value(item),
        Report::Document(document) => serde_json::to_value(document).map(|d| d["items"][0].clone()),
    }
    .expect("an outcome is JSON");
    let wire = &item["counterexample"];
    assert_eq!(wire["trace"], "model");
    assert_eq!(wire["kind"], "formula");
    assert_eq!(wire["initial_state"], 0);
    assert_eq!(wire["over_binding"]["type"], "reference");
    assert_eq!(wire["loop"], serde_json::json!([]));
    assert_eq!(wire["trace_position"], temporal.prefix.len());
    assert_eq!(
        wire["prefix"].as_array().map(Vec::len),
        Some(temporal.prefix.len())
    );
    for step in wire["prefix"].as_array().expect("steps") {
        assert_eq!(step["transition"]["type"], "transition");
        assert!(step["post_state"]["key"].is_string());
    }
    assert!(item.get("position").is_none());
    assert!(item.get("decisive").is_none());
    assert!(item.get("witness").is_none());
}

/// A case that does not refute holds no counterexample.
pub fn assert_no_counterexample(outcome: &CaseOutcome) {
    assert!(outcome.counterexamples().is_empty());
}

/// Two outcomes agree: the same report, records and counterexamples.
pub fn assert_same_outcome(left: &CaseOutcome, right: &CaseOutcome, label: &str) {
    assert_eq!(left.report(), right.report(), "{label}");
    assert_eq!(left.records(), right.records(), "{label}");
    assert_eq!(left.counterexamples(), right.counterexamples(), "{label}");
}
