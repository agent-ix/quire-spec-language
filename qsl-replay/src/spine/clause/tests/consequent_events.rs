// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1269 (FR-109-AC-11 to FR-109-AC-13): the native consequent source
//! events of `run_clause`, derived from the checked clause run and compared
//! with the checked package's source map.

use super::refinement::current_report;
use super::*;
use crate::spine::{
    ClauseRunReport, ConsequentEventRefusal, ConsequentSourceEvent, ConsequentSourceEvents,
    ConsequentState,
};
use qsl_foundation::source::provenance::SourceRegion;
use qsl_semantics::check::{Connective, NodeKind};

const NO_CYCLE_BODY: &str = "not reaches(self, self, next)";

/// The `NoCycle` request over `chain` with its body replaced by `body`.
fn request_with_body(body: &str, chain: &[(&str, Option<&str>)]) -> ClauseRunRequest {
    let (mut request, _) = no_cycle_request(chain);
    // Assign the source rather than destructure it: `ClauseRunSource` is
    // `#[non_exhaustive]` and gains a variant under `quire-extraction`, so a
    // pattern is irrefutable in one build and refutable in the other.
    let text = unit_and_packages().0.replace(NO_CYCLE_BODY, body);
    request.source = ClauseRunSource::Program {
        identity: source(),
        path: "clause-run.native".to_owned(),
        bytes: text.into_bytes(),
    };
    request
}

/// Compile the request's own source and return the checked package's
/// source-map regions (implication, consequent) of
/// every `implies` node of the `NoCycle` body, in pre-order.
fn checked_implications(
    request: &ClauseRunRequest,
    body: &str,
) -> Vec<(SourceRegion, SourceRegion)> {
    let text = unit_and_packages().0.replace(NO_CYCLE_BODY, body);
    let composed = compose(
        source(),
        "clause-run.native",
        text.as_bytes(),
        &request.packages,
        &DependencyInput::default(),
        SpineLimits::default(),
    )
    .unwrap_or_else(|refusal| panic!("compile refused: {refusal:?}"));
    let graph = composed.package.graph();
    let clause = graph.state_clause("NoCycle").expect("NoCycle is declared");
    let mut pending = vec![clause.body()];
    let mut found = Vec::new();
    while let Some(node) = pending.pop() {
        if let NodeKind::Connective(Connective::Implies, _, consequent) = node.kind() {
            found.push((
                graph.region(node.location()).expect("implication region"),
                graph
                    .region(node.at(*consequent).location())
                    .expect("consequent region"),
            ));
        }
        pending.extend(node.children().into_iter().rev());
    }
    found
}

fn produced(report: &ClauseRunReport) -> &[ConsequentSourceEvent] {
    let ConsequentSourceEvents::Produced(events) = &report.consequent_source_events else {
        panic!("expected events, got {:?}", report.consequent_source_events);
    };
    events.as_slice()
}

fn states(report: &ClauseRunReport) -> Vec<ConsequentState> {
    produced(report).iter().map(|event| event.state).collect()
}

/// Run `body` over `chain`, assert the events' identity equals the checked
/// package source map's, and return the report.
fn run_checked(body: &str, chain: &[(&str, Option<&str>)]) -> ClauseRunReport {
    let request = request_with_body(body, chain);
    let checked = checked_implications(&request, body);
    let report = run_clause(request).expect("a well-formed request reports");
    let events = produced(&report);
    assert_eq!(events.len(), checked.len(), "{body}");
    for (position, (event, (implication, consequent))) in events.iter().zip(&checked).enumerate() {
        assert_eq!(event.index, position, "{body}");
        assert_eq!(&event.implication, implication, "{body}");
        assert_eq!(&event.consequent, consequent, "{body}");
    }
    report
}

/// FR-109-AC-11: nested implications under `and`, `or`, `not` and `implies`
/// are each reported once, indexed in pre-order, with the state native
/// evaluation gave them and the identity of the checked source map.
#[trace("TC-1269", "FR-109-AC-11")]
#[test]
fn nested_implications_report_indexed_states_with_checked_identity() {
    use ConsequentState::{Entered, NotReached, Skipped};
    let a: &[(&str, Option<&str>)] = &[("a", None)];

    let and = run_checked(
        "(true implies true) and (false implies (true implies false))",
        a,
    );
    assert_eq!(and.disposition.truth(), Some(true));
    assert_eq!(states(&and), [Entered, Skipped, NotReached]);

    // The left disjunct is true, so the right operand is never reached.
    let or = run_checked(
        "(true implies true) or (true implies (false implies true))",
        a,
    );
    assert_eq!(states(&or), [Entered, NotReached, NotReached]);

    // A false left disjunct reaches the right operand.
    let or_right = run_checked("(true implies false) or (false implies true)", a);
    assert_eq!(or_right.disposition.truth(), Some(true));
    assert_eq!(states(&or_right), [Entered, Skipped]);

    let not = run_checked("not (true implies (false implies true))", a);
    assert_eq!(not.disposition.truth(), Some(false));
    assert_eq!(states(&not), [Entered, Skipped]);

    // An implication nested in the antecedent position, under `not` and `or`.
    let deep = run_checked("not ((false implies true) or (true implies false))", a);
    assert_eq!(states(&deep), [Skipped, NotReached]);

    let short = run_checked("false and (true implies true)", a);
    assert_eq!(states(&short), [NotReached]);

    // Antecedents from data: the snapshot alone selects the state.
    let body = "present(deref(self).next) implies true";
    let with_next = run_checked(body, &[("a", Some("b")), ("b", None)]);
    assert_eq!(states(&with_next), [Entered]);
    assert_eq!(states(&run_checked(body, a)), [Skipped]);
}

/// A falsified clause reports its events, and a clause with no implication
/// reports an empty produced list, not a refusal.
#[test]
fn a_falsified_clause_and_a_clause_without_implications_report_produced_events() {
    let falsified = run_checked("true implies false", &[("a", None)]);
    assert_eq!(falsified.disposition.truth(), Some(false));
    assert_eq!(states(&falsified), [ConsequentState::Entered]);

    let plain = run_checked(NO_CYCLE_BODY, &[("a", None)]);
    assert_eq!(plain.disposition.truth(), Some(true));
    assert_eq!(
        plain.consequent_source_events,
        ConsequentSourceEvents::Produced(Vec::new())
    );
}

/// A refinement-style invariant over the ConfigVersion model reports its
/// entered consequent in a satisfied and in a violated run.
#[test]
fn a_refinement_clause_reports_its_consequent_for_both_verdicts() {
    let satisfied = current_report("ParentOrder", 1, 2);
    assert_eq!(satisfied.disposition.truth(), Some(true));
    let violated = current_report("ParentOrder", 3, 2);
    assert_eq!(violated.disposition.truth(), Some(false));
    for report in [&satisfied, &violated] {
        assert_eq!(states(report), [ConsequentState::Entered]);
        assert_eq!(produced(report)[0].index, 0);
    }
    assert_eq!(
        produced(&satisfied)[0].implication,
        produced(&violated)[0].implication,
        "one checked implication has one source identity across runs"
    );
}

/// FR-109-AC-12: an implication the evaluation cannot decide is a typed
/// refusal, never `NotReached`, and the run's truth is unchanged.
#[trace("TC-1269", "FR-109-AC-12")]
#[test]
fn undecidable_consequents_refuse_typed_and_keep_the_truth() {
    let chain: &[(&str, Option<&str>)] = &[("a", Some("b")), ("b", None)];
    let refusal = |body: &str| {
        let report = run_clause(request_with_body(body, chain)).expect("reports");
        assert_eq!(report.disposition.truth(), Some(true), "{body}");
        let ConsequentSourceEvents::Refused(refusal) = report.consequent_source_events else {
            panic!("{body}: expected a refusal");
        };
        refusal
    };
    assert_eq!(
        refusal("forall(n in M::all_nodes : present(deref(n).next) implies true)"),
        ConsequentEventRefusal::UnderBinder
    );
    assert_eq!(
        refusal("exists(n in M::all_nodes : present(deref(n).next) implies true)"),
        ConsequentEventRefusal::UnderBinder
    );
    assert_eq!(
        refusal("hasNext(self) implies true"),
        ConsequentEventRefusal::CalledFunction
    );
}

/// The stable codes of the refusals are distinct and documented.
#[test]
fn refusal_codes_are_stable() {
    assert_eq!(
        [
            ConsequentEventRefusal::UnderBinder.code(),
            ConsequentEventRefusal::CalledFunction.code(),
            ConsequentEventRefusal::DecisionNotRecorded.code(),
            ConsequentEventRefusal::Temporal.code(),
            ConsequentEventRefusal::SourceRegionUnavailable.code(),
        ],
        [
            "consequent-events-under-binder",
            "consequent-events-called-function",
            "consequent-events-decision-not-recorded",
            "consequent-events-temporal-clause",
            "consequent-events-source-region-unavailable",
        ]
    );
}

/// FR-109-AC-13: a run that completes no Boolean truth, or selects a
/// function, reports no events (`Unavailable`), never an empty list.
#[trace("TC-1269", "FR-109-AC-13")]
#[test]
fn runs_without_a_boolean_truth_report_unavailable_events() {
    // An exhausted work budget completes no truth after a successful
    // evaluation call. With the truth gate removed the missing decisions
    // would surface as a refusal here.
    let mut exhausted = request_with_body("true implies true", &[("a", None)]);
    exhausted.accounting = default_accounting(0);
    let exhausted = run_clause(exhausted).expect("reports");
    assert_eq!(exhausted.disposition.truth(), None);
    assert_eq!(
        exhausted.consequent_source_events,
        ConsequentSourceEvents::Unavailable
    );

    let (mut function, label) = no_cycle_request(&[("a", None)]);
    function.selection = ClauseRunSelection::Function {
        name: "seven".to_owned(),
        arguments: Vec::new(),
        snapshot: label,
    };
    let function = run_clause(function).expect("reports");
    assert_eq!(
        function.consequent_source_events,
        ConsequentSourceEvents::Unavailable
    );
}
