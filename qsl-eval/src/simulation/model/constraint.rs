// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-021 SC-1 and SC-2 (FR-158): a state constraint's truth at a model
//! state.
//!
//! The constraint is a checked `at current` state clause of the package,
//! named by the request. It holds at a state when it holds for every object
//! whose most-specific type conforms to its context type, each evaluated
//! through FR-107 in ascending reference-key order under a fresh meter, as
//! an invariant is (FR-120 "Invariants").

use super::contracts::{invariant_observations, verdict, Verdict};
use super::walk::{exhausted, stop, walk_of};
use super::{ClauseOutcome, ModelSimState, ModelSystem, BROKEN};
use crate::simulation::expansion::{ExpansionLimit, ExpansionStop};
use crate::simulation::reduction::ConstraintValue;
use crate::value::CheckedPackageEvaluation;
use quire_exact::Meter;

impl ModelSystem<'_> {
    /// Whether the package declares a checked `at current` state clause
    /// named `name`, which a request may name as its state constraint.
    pub fn has_constraint(&self, name: &str) -> bool {
        self.clauses
            .invariants
            .iter()
            .any(|clause| clause.name == name)
    }

    /// FR-158 "Truth at a state": `Holds` when every evaluation is true;
    /// `Undefined` for the first object in key order whose evaluation is
    /// undefined; otherwise `Boundary` when some evaluation is false.
    ///
    /// # Errors
    ///
    /// `resource_exhausted`/`insufficient-next-charge` naming the evaluation
    /// meter when an evaluation runs out of it; a refused evaluation stops
    /// with its refusal's code, or `runtime_invariant`/
    /// `established-invariant-broken` for a kernel refusal, a non-Boolean
    /// value, an unknown clause or a call failure.
    pub fn constraint_value(
        &self,
        state: &ModelSimState,
        name: &str,
    ) -> Result<ConstraintValue, ExpansionStop> {
        let clause = self
            .clauses
            .invariants
            .iter()
            .find(|clause| clause.name == name)
            .ok_or_else(|| stop(BROKEN))?;
        let walk = walk_of(self, state, None)?;
        let mut order: Vec<usize> = (0..walk.rows.len()).collect();
        order.sort_by(|left, right| walk.rows[*left].reference.cmp(&walk.rows[*right].reference));
        let mut boundary = false;
        for row in order {
            let reference = &walk.rows[row].reference;
            if !self.types.conforms(reference.object_type(), clause.context) {
                continue;
            }
            let observations = invariant_observations(&walk, clause, row);
            let evaluation = self
                .package
                .evaluate_clause(&clause.qualified, &observations, &mut Meter::new(self.meter))
                .map_err(|_| stop(BROKEN))?;
            match verdict(&evaluation.evaluation.outcome) {
                Verdict::True => {}
                Verdict::False => boundary = true,
                Verdict::Undecided(outcome) => {
                    if let Some(cause) = outcome.undefined_cause() {
                        return Ok(ConstraintValue::Undefined {
                            object: reference.clone(),
                            cause,
                        });
                    }
                    return Err(match outcome {
                        ClauseOutcome::Incomplete(_) => exhausted(ExpansionLimit::EvaluationMeter),
                        ClauseOutcome::FamilyRefused(code) => stop(code),
                        ClauseOutcome::KernelRefused(_)
                        | ClauseOutcome::NotBoolean
                        | ClauseOutcome::KernelUndefined(_)
                        | ClauseOutcome::FamilyUndefined(_) => stop(BROKEN),
                    });
                }
            }
        }
        Ok(if boundary {
            ConstraintValue::Boundary
        } else {
            ConstraintValue::Holds
        })
    }
}
