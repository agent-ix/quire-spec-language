// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-144 (FR-052-AC-9 to AC-14): the Layer-4 checked-package route owns
//! temporal truth, settlement and support. Each test runs `run_clause` over a
//! unit compiled through S1 to S4 and compares the reported verdict with the
//! FR-043/FR-501 reference rule written in the TC-144 table, never with the
//! route's own output.
//!
//! The fixture is TC-1043's `Counter` (`c.value` stands for `Widget.level`).
//! A unit holds one temporal profile (FR-110), so `Reach` (event-position) and
//! `Window` (finite-window) are two units. Trace **A** is `value` 0, 1, 2 at
//! stamps 0, 2, 5 and trace **B** is 0, 1, 0.

use super::*;
use crate::spine::{
    FiniteWindowSupport, ProgressCompleteness, TemporalIndeterminateReason, TemporalTraceRefusal,
};

const STAMPS: [u64; 3] = [0, 2, 5];
const ATOM: &str = "holds(c.value = 2)";

/// `Window` over trace A or B with the stated watermark and completeness.
fn window_over(values: &[i64], watermark: u64, complete_through: Option<u64>) -> ClauseRunRequest {
    timestamped_with(
        Unit::OwnerWindow,
        &STAMPS,
        values,
        watermark,
        complete_through,
    )
}

fn set_scope(request: &mut ClauseRunRequest, scope: crate::spine::ProgressClosure) {
    let ClauseRunSelection::Temporal { trace, .. } = &mut request.selection else {
        panic!("a temporal selection");
    };
    let TraceInput::Timestamped { progress, .. } = trace else {
        panic!("a timestamped trace");
    };
    progress.scope = scope;
}

fn remove_valuation(mut request: ClauseRunRequest, position: u64) -> ClauseRunRequest {
    let ClauseRunSelection::Temporal { trace, .. } = &mut request.selection else {
        panic!("a temporal selection");
    };
    let TraceInput::Timestamped { window, .. } = trace else {
        panic!("a timestamped trace");
    };
    window
        .missing_valuations
        .push(crate::spine::TimestampedMissingValuation { position, atom: 1 });
    request
}

fn window_report(report: &ClauseRunReport) -> &crate::spine::FiniteWindowReport {
    report
        .finite_window
        .as_ref()
        .expect("a finite-window verdict reports its basis, support and premises")
}

fn support_of_position_two() -> Vec<FiniteWindowSupport> {
    vec![FiniteWindowSupport {
        position: 2,
        atom: ATOM.to_owned(),
    }]
}

/// Row 1: `Reach` is `Completed(true)` over trace A although its leaf
/// `holds(c.value = 2)` is false at position 0, so the result is the
/// formula's and not a leaf valuation; over a trace that never reaches 2 the
/// same clause is violated, so the result depends on the evidence.
#[trace("TC-144", "FR-052-AC-9")]
#[test]
fn the_route_result_is_the_formula_not_the_leaf_valuation() {
    let reaches = report(finite(Unit::OwnerReach, "Reach", &[0, 1, 2]));
    assert_eq!(outcome(&reaches), &TemporalVerdict::Holds);
    let never = report(finite(Unit::OwnerReach, "Reach", &[0, 1, 0]));
    assert_eq!(outcome(&never), &TemporalVerdict::Violated { at: 0 });
}

/// Rows 4a and 7: `Window` over A at `W` 5 complete through 5 and a closed
/// scope is `Completed(true)` with basis `closed-scope`, the support is the
/// one valuation that decides it, and the profile is not refused.
#[trace("TC-144", "FR-052-AC-9", "FR-052-AC-14")]
#[test]
fn the_finite_window_profile_is_evaluated_with_its_basis_and_support() {
    let closed = report(window_over(&[0, 1, 2], 5, Some(5)));
    assert_eq!(outcome(&closed), &TemporalVerdict::Holds);
    assert_eq!(window_report(&closed).basis, SettlementBasis::ClosedScope);
    assert_eq!(window_report(&closed).support, support_of_position_two());
    assert!(!matches!(
        closed.disposition,
        ClauseDisposition::TemporalRefusal(TemporalTraceRefusal::ProfileNotEvaluated { .. })
    ));
}

/// Row 4f: the same trace with an open scope is a verdict with basis
/// `decisive-witness`, not a non-truth outcome.
#[trace("TC-144", "FR-052-AC-9", "FR-052-AC-10")]
#[test]
fn an_open_scope_settles_on_a_decisive_witness() {
    let mut request = window_over(&[0, 1, 2], 5, Some(5));
    set_scope(&mut request, crate::spine::ProgressClosure::Open);
    let open = report(request);
    assert_eq!(outcome(&open), &TemporalVerdict::Holds);
    assert_eq!(window_report(&open).basis, SettlementBasis::DecisiveWitness);
    assert_eq!(window_report(&open).support, support_of_position_two());
}

/// Rows 4b and 4c: a missing valuation outside the decision support leaves
/// the verdict and the support unchanged; one at the deciding position is a
/// typed non-truth outcome naming the position and atom, never `false`.
#[trace("TC-144", "FR-052-AC-10")]
#[test]
fn a_missing_fact_is_a_non_truth_outcome_only_where_it_decides() {
    let outside = report(remove_valuation(window_over(&[0, 1, 2], 5, Some(5)), 1));
    assert_eq!(outcome(&outside), &TemporalVerdict::Holds);
    assert_eq!(window_report(&outside).support, support_of_position_two());

    let deciding = report(remove_valuation(window_over(&[0, 1, 2], 5, Some(5)), 2));
    assert_eq!(
        outcome(&deciding),
        &TemporalVerdict::Indeterminate {
            at: 0,
            reason: TemporalIndeterminateReason::MissingValuation {
                position: 2,
                atom: ATOM.to_owned(),
                instance: None,
                also_held_by: Vec::new(),
            },
        }
    );
    assert_eq!(window_report(&deciding).basis, SettlementBasis::Unavailable);
}

/// Rows 4d and 4e: over trace B the closed complete window is `Completed
/// (false)`; the same evidence declared incomplete is `IncompleteInput`, not
/// a default `false` or `complete`; a watermark short of the window's end is
/// `Pending` with the deadline.
#[trace("TC-144", "FR-052-AC-10", "FR-052-AC-11")]
#[test]
fn incomplete_or_early_evidence_is_never_defaulted_to_a_truth() {
    let complete = report(window_over(&[0, 1, 0], 5, Some(5)));
    assert_eq!(outcome(&complete), &TemporalVerdict::Violated { at: 0 });

    let incomplete = report(window_over(&[0, 1, 0], 5, None));
    assert_eq!(
        outcome(&incomplete),
        &TemporalVerdict::Indeterminate {
            at: 0,
            reason: TemporalIndeterminateReason::IncompleteInput,
        }
    );
    assert_eq!(
        window_report(&incomplete).premises.completeness,
        ProgressCompleteness::Incomplete
    );

    let early = report(window_over(&[0, 1, 0], 3, Some(3)));
    assert_eq!(
        outcome(&early),
        &TemporalVerdict::PendingWindow { at: 0, deadline: 5 }
    );
}

/// Rows 4h and 5: a trace of the wrong shape for the clause's profile is
/// refused at admit with no partial truth.
#[trace("TC-144", "FR-052-AC-10", "FR-052-AC-14")]
#[test]
fn a_trace_of_the_wrong_shape_for_the_profile_is_refused_at_admit() {
    let timestamped_for_reach = request(
        Unit::OwnerReach,
        |labels| ClauseRunSelection::Temporal {
            name: "Reach".to_owned(),
            trace: TraceInput::Timestamped {
                observations: labels
                    .iter()
                    .zip(STAMPS)
                    .map(|(document, stamp)| crate::spine::TimestampedObservation {
                        document: document.clone(),
                        stamp,
                        order_key: None,
                    })
                    .collect(),
                progress: crate::spine::ProgressAssertion {
                    watermark: 5,
                    completeness: ProgressCompleteness::Complete { through: 5 },
                    scope: crate::spine::ProgressClosure::Closed,
                    execution: crate::spine::ProgressClosure::Closed,
                },
                window: Box::new(crate::spine::TimestampedWindow {
                    binding: Some(finite_window_binding()),
                    authoritative_origin: true,
                    cover: None,
                    missing_valuations: Vec::new(),
                }),
            },
            over: over("c1"),
            ledger: None,
        },
        &[0, 1, 2],
    );
    assert_admit_invalid_value(&report(timestamped_for_reach));

    let finite_for_window = request(
        Unit::OwnerWindow,
        |labels| ClauseRunSelection::Temporal {
            name: "Window".to_owned(),
            trace: TraceInput::Finite(labels.to_vec()),
            over: over("c1"),
            ledger: None,
        },
        &[0, 1, 2],
    );
    assert_admit_invalid_value(&report(finite_for_window));

    let lasso_for_reach = report(lasso(Unit::OwnerReach, "Reach", 0, &[0, 1, 2]));
    assert_admit_invalid_value(&lasso_for_reach);
    assert_eq!(lasso_for_reach.disposition.truth(), None);
}

/// Row 5, Timestamped part: `Window` listed at stamps 0, 5, 2 with `level` 0,
/// 2, 1 is not refused; its canonical order (stamps 0, 2, 5) is trace A's, so
/// the outcome, basis, support and premises are row 4a's whatever the listed
/// order (FR-501 "Canonical order").
#[trace("TC-144", "FR-052-AC-9", "FR-501-AC-19")]
#[test]
fn a_timestamped_listing_out_of_stamp_order_is_evaluated_in_canonical_order() {
    let in_order = report(window_over(&[0, 1, 2], 5, Some(5)));
    let listed = report(timestamped_with(
        Unit::OwnerWindow,
        &[0, 5, 2],
        &[0, 2, 1],
        5,
        Some(5),
    ));
    assert_eq!(outcome(&listed), &TemporalVerdict::Holds);
    assert_eq!(outcome(&listed), outcome(&in_order));
    assert_eq!(window_report(&listed).basis, SettlementBasis::ClosedScope);
    assert_eq!(window_report(&listed).support, support_of_position_two());
    assert_eq!(
        window_report(&listed).premises,
        window_report(&in_order).premises
    );

    // The same observations listed in another order are the same evidence:
    // the result, and its identity, do not depend on the listing.
    let mut shuffled = window_over(&[0, 1, 2], 5, Some(5));
    let ClauseRunSelection::Temporal { trace, .. } = &mut shuffled.selection else {
        panic!("a temporal selection");
    };
    let TraceInput::Timestamped { observations, .. } = trace else {
        panic!("a timestamped trace");
    };
    observations.swap(1, 2);
    assert_eq!(
        observations.iter().map(|o| o.stamp).collect::<Vec<_>>(),
        [0, 5, 2]
    );
    let shuffled = report(shuffled);
    assert_eq!(outcome(&shuffled), &TemporalVerdict::Holds);
    assert_eq!(window_report(&shuffled), window_report(&in_order));
}

/// Row 6: `Reach` settled over A keeps its result when a sibling run of the
/// same clause under a zero work budget stops `Incomplete` with no truth.
#[trace("TC-144", "FR-052-AC-11")]
#[test]
fn an_over_budget_sibling_stops_incomplete_and_the_settled_one_is_unchanged() {
    let alone = report(finite(Unit::OwnerReach, "Reach", &[0, 1, 2]));
    let mut starved = finite(Unit::OwnerReach, "Reach", &[0, 1, 2]);
    starved.accounting = crate::spine::default_accounting(0);
    let starved = report(starved);
    let alongside = report(finite(Unit::OwnerReach, "Reach", &[0, 1, 2]));

    assert_eq!(outcome(&alone), &TemporalVerdict::Holds);
    assert_eq!(format!("{alone:?}"), format!("{alongside:?}"));
    assert!(
        matches!(outcome(&starved), TemporalVerdict::Incomplete(_)),
        "{:?}",
        starved.disposition
    );
    assert_eq!(starved.disposition.truth(), None);
}

/// Row 8: the `Window` binding's ledger refuses a regressing watermark and a
/// retreating completeness as FR-502 states, leaves the retained progress
/// unchanged, and progress asserted under a foreign clause or package is
/// another binding's: it settles nothing here and is never a contradiction.
#[trace("TC-144", "TC-1296", "FR-052-AC-13")]
#[test]
fn the_ledger_refuses_a_contradiction_and_a_foreign_binding_settles_nothing() {
    let first = report(with_ledger(
        window_over(&[0, 1, 2], 100, Some(100)),
        &crate::spine::ProgressLedger::new(),
    ));
    let retained = first
        .ledger
        .expect("an accepted request returns its ledger");
    let binding = retained.bindings().next().expect("one binding").clone();
    assert_eq!(
        retained.entry(&binding).map(|entry| entry.watermark()),
        Some(100)
    );

    let regressed = report(with_ledger(
        window_over(&[0, 1, 2], 60, Some(100)),
        &retained,
    ));
    let ClauseDisposition::ProgressContradiction(refusal) = &regressed.disposition else {
        panic!("expected a contradiction, got {:?}", regressed.disposition);
    };
    assert_eq!(
        refusal.contradiction.dimension,
        crate::spine::ContradictedDimension::Watermark
    );
    assert_eq!(regressed.ledger, Some(retained.clone()));

    let retreated = report(with_ledger(
        window_over(&[0, 1, 2], 100, Some(60)),
        &retained,
    ));
    let ClauseDisposition::ProgressContradiction(refusal) = &retreated.disposition else {
        panic!("expected a contradiction, got {:?}", retreated.disposition);
    };
    assert_eq!(
        refusal.contradiction.dimension,
        crate::spine::ContradictedDimension::Completeness
    );
    assert_eq!(retreated.ledger, Some(retained.clone()));

    // The same progress that regressed under `Window` is accepted for the
    // foreign clause `Other`: a new binding, no contradiction, and the
    // `Window` entry is untouched.
    let mut foreign_clause = window_over(&[0, 1, 2], 60, Some(60));
    let ClauseRunSelection::Temporal { name, .. } = &mut foreign_clause.selection else {
        panic!("a temporal selection");
    };
    "Other".clone_into(name);
    let foreign = report(with_ledger(foreign_clause, &retained));
    assert_eq!(outcome(&foreign), &TemporalVerdict::Holds);
    let updated = foreign.ledger.expect("the foreign clause was accepted");
    assert_eq!(updated.bindings().count(), 2);
    assert_eq!(
        updated.entry(&binding).map(|entry| entry.watermark()),
        Some(100)
    );

    // And under a foreign package (another source identity).
    let mut foreign_package = window_over(&[0, 1, 2], 60, Some(60));
    #[cfg(feature = "quire-extraction")]
    let ClauseRunSource::Program { identity, .. } = &mut foreign_package.source
    else {
        panic!("a program source");
    };
    #[cfg(not(feature = "quire-extraction"))]
    let ClauseRunSource::Program { identity, .. } = &mut foreign_package.source;
    *identity = qsl_foundation::SourceIdentity::new("agent-ix", "clause-run-foreign-package");
    let foreign = report(with_ledger(foreign_package, &retained));
    assert_eq!(outcome(&foreign), &TemporalVerdict::Holds);
    let updated = foreign.ledger.expect("the foreign package was accepted");
    assert_eq!(updated.bindings().count(), 2);
    assert_eq!(
        updated.entry(&binding).map(|entry| entry.watermark()),
        Some(100)
    );
}
