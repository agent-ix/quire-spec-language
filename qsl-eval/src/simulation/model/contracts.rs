// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-120 "Contract clauses" and "Invariants": evaluating a state's
//! invariants and the conjunction of an application's `pre` or `post`
//! clauses through FR-107.

use qsl_semantics::family::{FamilyOutcome, FamilyResult};
use qsl_semantics::model::observation::{AdmissionUsage, AdmittedObservations, SelfKind};
use quire_exact::{Meter, Outcome, Value};

use super::walk::{exhausted, stop, Walk};
use super::{ClauseInfo, ClauseOutcome, ModelFinding, BROKEN};
use crate::simulation::expansion::{ExpansionLimit, ExpansionStop};
use crate::value::{CheckedPackageEvaluation, ClauseSetOutcome};

/// How one clause evaluation decided.
pub(super) enum Verdict {
    True,
    False,
    Undecided(ClauseOutcome),
}

/// Read a clause evaluation's outcome as a verdict.
///
/// A `match` over [`FamilyOutcome`] would have to name its probe-only variant
/// (FR-063), so the two real arms are tested in turn.
pub(super) fn verdict(outcome: &FamilyOutcome<Value>) -> Verdict {
    if let FamilyOutcome::Evaluated(kernel) = outcome {
        return match kernel {
            Outcome::Completed(Value::Boolean(true)) => Verdict::True,
            Outcome::Completed(Value::Boolean(false)) => Verdict::False,
            Outcome::Completed(_) => Verdict::Undecided(ClauseOutcome::NotBoolean),
            Outcome::Undefined(undefined) => {
                Verdict::Undecided(ClauseOutcome::KernelUndefined(undefined.clone()))
            }
            Outcome::Refused(refusal) => {
                Verdict::Undecided(ClauseOutcome::KernelRefused(refusal.clone()))
            }
            Outcome::Incomplete(incomplete) => {
                Verdict::Undecided(ClauseOutcome::Incomplete(incomplete.clone()))
            }
        };
    }
    if let FamilyOutcome::FamilyEvaluated(family) = outcome {
        return Verdict::Undecided(match family {
            FamilyResult::Refused(cause) => ClauseOutcome::FamilyRefused(cause.catalog_code()),
            FamilyResult::Undefined(cause) => {
                ClauseOutcome::FamilyUndefined(cause.undefined_record().reason)
            }
        });
    }
    Verdict::Undecided(ClauseOutcome::NotBoolean)
}

/// The observations of an invariant over one object of the state.
pub(super) fn invariant_observations(
    walk: &Walk<'_, '_>,
    clause: &ClauseInfo,
    row: usize,
) -> AdmittedObservations {
    AdmittedObservations {
        usage: AdmissionUsage::default(),
        clause: clause.identity,
        current: Some(walk.observation.clone()),
        pre: None,
        post: None,
        self_object: walk.rows[row].reference.clone(),
        invocation: None,
        self_kind: SelfKind::Object,
        receiver_pre: None,
        receiver_post: None,
        parameters: Vec::new(),
        result: None,
        created: Vec::new(),
        deleted: Vec::new(),
    }
}

/// Every invariant of the package, in ascending name order, for each object
/// of the state whose most-specific type conforms to the clause's context, in
/// ascending reference-key order (FR-120 "Invariants").
pub(super) fn invariants(walk: &Walk<'_, '_>) -> Result<Vec<ModelFinding>, ExpansionStop> {
    let system = walk.system;
    let mut order: Vec<usize> = (0..walk.rows.len()).collect();
    order.sort_by(|left, right| walk.rows[*left].reference.cmp(&walk.rows[*right].reference));
    let mut findings = Vec::new();
    for clause in &system.clauses.invariants {
        for row in &order {
            let reference = &walk.rows[*row].reference;
            if !system
                .types
                .conforms(reference.object_type(), clause.context)
            {
                continue;
            }
            let observations = invariant_observations(walk, clause, *row);
            let evaluation = system
                .package
                .evaluate_clause(
                    &clause.qualified,
                    &observations,
                    &mut Meter::new(system.meter),
                )
                .map_err(|_| stop(BROKEN))?;
            match verdict(&evaluation.evaluation.outcome) {
                Verdict::True => {}
                Verdict::False => findings.push(ModelFinding::InvariantViolated {
                    clause: clause.name.clone(),
                    self_object: reference.clone(),
                    anchor: walk.state.anchor.clone(),
                }),
                Verdict::Undecided(outcome) => findings.push(ModelFinding::InvariantUndetermined {
                    clause: clause.name.clone(),
                    self_object: reference.clone(),
                    anchor: walk.state.anchor.clone(),
                    evaluation: outcome,
                }),
            }
        }
    }
    Ok(findings)
}

/// The conjunction of a set of clauses of one kind (FR-120 "Contract
/// clauses").
pub(super) enum Conjunction {
    /// Every clause is `true`, or there are none.
    True,
    /// Some clause is `false`; this is the first such clause in evaluation
    /// order (ascending name).
    False(String),
    /// No clause is `false` and these did not decide, in name order.
    Undecided(Vec<(String, ClauseOutcome)>),
}

/// Evaluate `clauses`, each over the observations `observe` builds for it,
/// each with a fresh meter and in the order given (ascending name).
///
/// # Errors
///
/// An `Incomplete` evaluation stops the expansion with
/// `resource_exhausted`/`insufficient-next-charge`; a call failure stops it
/// with `runtime_invariant`/`established-invariant-broken`.
pub(super) fn conjunction(
    walk: &Walk<'_, '_>,
    clauses: &[&ClauseInfo],
    observe: impl Fn(&ClauseInfo) -> AdmittedObservations,
) -> Result<Conjunction, ExpansionStop> {
    if clauses.is_empty() {
        return Ok(Conjunction::True);
    }
    let observations: Vec<AdmittedObservations> =
        clauses.iter().map(|info| observe(info)).collect();
    let set: Vec<_> = clauses
        .iter()
        .zip(&observations)
        .map(|(info, observations)| (info.qualified.clone(), observations))
        .collect();
    let evaluation = walk
        .system
        .package
        .evaluate_clause_set(&set, walk.system.meter)
        .map_err(|_| stop(BROKEN))?;
    match evaluation.outcome {
        ClauseSetOutcome::Incomplete => Err(exhausted(ExpansionLimit::EvaluationMeter)),
        ClauseSetOutcome::True => Ok(Conjunction::True),
        ClauseSetOutcome::False => {
            let entry = evaluation
                .results
                .iter()
                .find(|entry| matches!(verdict(&entry.evaluation.evaluation.outcome), Verdict::False))
                .ok_or_else(|| stop(BROKEN))?;
            let name = clauses
                .iter()
                .find(|info| info.qualified == entry.name)
                .map(|info| info.name.clone())
                .ok_or_else(|| stop(BROKEN))?;
            Ok(Conjunction::False(name))
        }
        ClauseSetOutcome::Undecided => {
            let mut undecided = Vec::new();
            for entry in &evaluation.results {
                if let Verdict::Undecided(outcome) = verdict(&entry.evaluation.evaluation.outcome) {
                    let name = clauses
                        .iter()
                        .find(|info| info.qualified == entry.name)
                        .map(|info| info.name.clone())
                        .ok_or_else(|| stop(BROKEN))?;
                    undecided.push((name, outcome));
                }
            }
            Ok(Conjunction::Undecided(undecided))
        }
    }
}
