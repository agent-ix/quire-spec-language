// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-327 "Refused condition" and FR-501 ("an atom that evaluates Undefined
//! or a condition that refuses is returned as FR-327 states, in evaluation
//! order"): the finite-window evaluator and the event-position evaluator
//! return the same result for the same clause over every trace of three
//! counter values, whichever of an undefined atom (`U`, where `c.value` is
//! 2), a refused condition (`R`, where it is 1 or more) and a false atom
//! comes first, in a conjunction, a scan, an `on each` activation, a `when`
//! guard and a capture.
//!
//! The traces sit at stamps 0, 1, 2 of a closed window, and every interval
//! lies inside them, so false extension past the end changes no result.

use super::*;

/// The names of the clauses of `differential_clauses!`.
const CLAUSES: usize = 14;

/// What the run reported, reduced to the parts both evaluators define.
fn summary(report: &ClauseRunReport) -> String {
    if let Some(at) = report.refused_at {
        return format!("refused@{at}/{:?}", report.category());
    }
    match &report.disposition {
        ClauseDisposition::Temporal(TemporalVerdict::Holds) => "holds".to_owned(),
        ClauseDisposition::Temporal(TemporalVerdict::Violated { at }) => format!("violated@{at}"),
        ClauseDisposition::Temporal(TemporalVerdict::Undefined { at, .. }) => {
            format!("undefined@{at}")
        }
        other => format!("{other:?}").chars().take(300).collect(),
    }
}

/// Every trace of three counter values `0..=2`.
fn traces() -> Vec<[i64; 3]> {
    let mut all = Vec::new();
    for a in 0..3 {
        for b in 0..3 {
            for c in 0..3 {
                all.push([a, b, c]);
            }
        }
    }
    all
}

/// The rows where the two evaluators differ, `clause values: event | window`.
fn divergences() -> Vec<String> {
    let mut rows = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for index in 0..CLAUSES {
        let name = format!("D{index}");
        for values in traces() {
            let event = summary(&report(finite(Unit::DifferentialEvent, &name, &values)));
            let window = summary(&report(timestamped_named(
                Unit::DifferentialWindow,
                &name,
                &[0, 1, 2],
                &values,
                5,
                Some(5),
            )));
            seen.insert(event.split('@').next().unwrap_or_default().to_owned());
            if event != window {
                rows.push(format!(
                    "{name} {values:?}: event {event} | window {window}"
                ));
            }
        }
    }
    // The comparison is not vacuous: the traces reach every result kind.
    for kind in ["holds", "violated", "undefined", "refused"] {
        assert!(seen.contains(kind), "no trace reaches {kind}: {seen:?}");
    }
    rows
}

/// FR-327-AC-6, FR-501: the two evaluators agree on every clause and trace.
#[trace("TC-1160", "FR-327-AC-6")]
#[test]
fn the_finite_window_and_event_position_evaluators_return_the_same_precedence() {
    let rows = divergences();
    assert!(
        rows.is_empty(),
        "{} divergences:\n{}",
        rows.len(),
        rows.join("\n")
    );
}
