// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-107 "Clause sets": the state clauses of one invocation, one object or
//! one state evaluated together, and their combined outcome (FR-120).

use qsl_semantics::check::Observation;
use qsl_semantics::family::FamilyOutcome;
use qsl_semantics::model::observation::AdmittedObservations;
use quire_exact::{Meter, Outcome, ScalarLimits, Value};

use super::{resolve_clause, CallFailure, CheckedPackageEvaluation, ClauseEvaluation, QualifiedName};
use qsl_package::CheckedPackage;

/// One clause a set evaluated.
#[derive(Debug)]
pub struct ClauseSetEntry {
    /// The clause's name.
    pub name: QualifiedName,
    /// The clause's evaluation.
    pub evaluation: ClauseEvaluation,
}

/// The conjunction of a clause set's entries (FR-120).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClauseSetOutcome {
    /// Every entry is `Completed(true)`, or the set is empty.
    True,
    /// An entry is `Completed(false)`.
    False,
    /// No entry is `Completed(false)` and one or more completed no Boolean.
    Undecided,
    /// A clause evaluated to `Incomplete`; it is the last entry.
    Incomplete,
}

/// The results of a clause set, together.
#[derive(Debug)]
pub struct ClauseSetEvaluation {
    /// One entry for each clause the set evaluated, in evaluation order.
    pub results: Vec<ClauseSetEntry>,
    /// The entries combined.
    pub outcome: ClauseSetOutcome,
}

/// How one clause's evaluation counts toward its set's outcome.
#[derive(Clone, Copy, Eq, PartialEq)]
enum Verdict {
    True,
    False,
    Undecided,
    Incomplete,
}

fn verdict(evaluation: &ClauseEvaluation) -> Verdict {
    let outcome = &evaluation.evaluation.outcome;
    if matches!(
        outcome,
        FamilyOutcome::Evaluated(Outcome::Completed(Value::Boolean(true)))
    ) {
        Verdict::True
    } else if matches!(
        outcome,
        FamilyOutcome::Evaluated(Outcome::Completed(Value::Boolean(false)))
    ) {
        Verdict::False
    } else if matches!(outcome, FamilyOutcome::Evaluated(Outcome::Incomplete(_))) {
        Verdict::Incomplete
    } else {
        Verdict::Undecided
    }
}

pub(super) fn evaluate(
    package: &CheckedPackage,
    clauses: &[(QualifiedName, &AdmittedObservations)],
    limits: ScalarLimits,
) -> Result<ClauseSetEvaluation, CallFailure> {
    // Refuse a bad set whole, before any clause is evaluated.
    let mut invariants = Vec::new();
    let mut preconditions = Vec::new();
    let mut postconditions = Vec::new();
    for (name, observations) in clauses {
        let group = match resolve_clause(package, name, observations)?.observation() {
            Observation::Current => &mut invariants,
            Observation::Pre => &mut preconditions,
            Observation::Post => &mut postconditions,
        };
        group.push((name, *observations));
    }
    for group in [&mut invariants, &mut preconditions, &mut postconditions] {
        group.sort_by(|left, right| left.0.cmp(right.0));
    }

    let mut results = Vec::new();
    let mut seen = Vec::new();
    let mut run = |group: &[(&QualifiedName, &AdmittedObservations)],
                   results: &mut Vec<ClauseSetEntry>|
     -> Result<bool, CallFailure> {
        for (name, observations) in group {
            let evaluation =
                package.evaluate_clause(name, observations, &mut Meter::new(limits))?;
            let verdict = verdict(&evaluation);
            results.push(ClauseSetEntry {
                name: (*name).clone(),
                evaluation,
            });
            seen.push(verdict);
            if verdict == Verdict::Incomplete {
                return Ok(false);
            }
        }
        Ok(true)
    };
    let mut complete = run(&invariants, &mut results)?;
    let preconditions_start = seen.len();
    if complete {
        complete = run(&preconditions, &mut results)?;
    }
    let preconditions_true = seen[preconditions_start..]
        .iter()
        .all(|verdict| *verdict == Verdict::True);
    if complete && preconditions_true {
        run(&postconditions, &mut results)?;
    }
    let outcome = if seen.contains(&Verdict::Incomplete) {
        ClauseSetOutcome::Incomplete
    } else if seen.contains(&Verdict::False) {
        ClauseSetOutcome::False
    } else if seen.contains(&Verdict::Undecided) {
        ClauseSetOutcome::Undecided
    } else {
        ClauseSetOutcome::True
    };
    Ok(ClauseSetEvaluation { results, outcome })
}
