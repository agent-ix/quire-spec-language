// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-107 "Clause sets": the state clauses of one invocation, one object or
//! one state evaluated together, and their combined outcome (FR-120).

use qsl_semantics::family::FamilyOutcome;
use qsl_semantics::model::clause_observation::Observation;
use qsl_semantics::model::observation::AdmittedObservations;
use quire_semantic_value::call::InputRefusal;
use std::collections::BTreeSet;

use quire_exact::{Meter, Outcome, ScalarLimits, Value};

use super::{
    resolve_clause, CallFailure, CheckedPackageEvaluation, ClauseEvaluation, QualifiedName,
};
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

/// Evaluate `group` in order, each clause with a fresh meter; `false` when a
/// clause was `Incomplete`, which ends the set.
fn run_group(
    package: &CheckedPackage,
    group: &[(&QualifiedName, &AdmittedObservations)],
    limits: ScalarLimits,
    results: &mut Vec<ClauseSetEntry>,
    seen: &mut Vec<Verdict>,
) -> Result<bool, CallFailure> {
    for (name, observations) in group {
        let evaluation = package.evaluate_clause(name, observations, &mut Meter::new(limits))?;
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
}

/// FR-107 "Clause sets": the set's outcome from its entries' verdicts.
fn combine(verdicts: &[Verdict]) -> ClauseSetOutcome {
    if verdicts.contains(&Verdict::Incomplete) {
        ClauseSetOutcome::Incomplete
    } else if verdicts.contains(&Verdict::False) {
        ClauseSetOutcome::False
    } else if verdicts.contains(&Verdict::Undecided) {
        ClauseSetOutcome::Undecided
    } else {
        ClauseSetOutcome::True
    }
}

pub(super) fn evaluate(
    package: &CheckedPackage,
    clauses: &[(QualifiedName, &AdmittedObservations)],
    limits: ScalarLimits,
) -> Result<ClauseSetEvaluation, CallFailure> {
    // Refuse a bad set whole, before any clause is evaluated.
    let mut named = BTreeSet::new();
    for (name, _) in clauses {
        if !named.insert(name) {
            return Err(InputRefusal::DuplicateClause(name.to_string()).into());
        }
    }
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
    let mut run = |group: &[(&QualifiedName, &AdmittedObservations)]| {
        run_group(package, group, limits, &mut results, &mut seen)
    };
    let invariants_done = run(&invariants)?;
    let preconditions_done = invariants_done && run(&preconditions)?;
    // Postconditions follow only when every precondition was evaluated and
    // is true; an invariant never skips them.
    let preconditions_true = preconditions.iter().all(|(name, _)| {
        results
            .iter()
            .any(|entry| &entry.name == *name && verdict(&entry.evaluation) == Verdict::True)
    });
    if preconditions_done && preconditions_true {
        run_group(package, &postconditions, limits, &mut results, &mut seen)?;
    }
    let outcome = combine(&seen);
    Ok(ClauseSetEvaluation { results, outcome })
}

#[cfg(test)]
mod tests {
    use super::{combine, ClauseSetOutcome, Verdict};

    /// FR-107 "Clause sets": `outcome` is `Incomplete` over everything,
    /// `False` over `Undecided`, `Undecided` over `True`, and `True` for an
    /// empty set. The `Undecided` entries are given here directly
    /// (TC-1039 step 6's clause does not check).
    #[test]
    fn the_outcome_combines_verdicts_by_precedence() {
        use Verdict::{False, Incomplete, True, Undecided};
        assert_eq!(combine(&[]), ClauseSetOutcome::True);
        assert_eq!(combine(&[True, True]), ClauseSetOutcome::True);
        assert_eq!(combine(&[True, Undecided]), ClauseSetOutcome::Undecided);
        assert_eq!(combine(&[Undecided, False]), ClauseSetOutcome::False);
        assert_eq!(combine(&[False, Incomplete]), ClauseSetOutcome::Incomplete);
    }
}
