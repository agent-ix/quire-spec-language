// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-167 (ADR-022 GM-3): how one state-graph predicate reads at one model
//! state, with the claim's `over` parameter bound.
//!
//! The predicate is evaluated through the one clause evaluator (FR-107) over
//! the state's synthesized observation. The bound parameter reads as the
//! state's own reference to the bound object, matched by universe and key,
//! never by comparing the whole reference (FR-125 Instancing); where the
//! state holds no such object the predicate is `absent-key` undefined, as
//! ADR-018 QS-5 states for temporal atoms.

use std::collections::BTreeMap;

use qsl_foundation::diagnostic::{UndefinedReason, UndefinedRecord};
use qsl_semantics::check::CheckedPredicateBody;
use quire_exact::{Meter, ObjectReference};
use quire_semantic_value::location::Location;

use super::contracts::{verdict, Verdict};
use super::walk::{exhausted, stop, walk_of};
use super::{ClauseOutcome, ModelSimState, ModelSystem, BROKEN};
use crate::simulation::expansion::{ExpansionLimit, ExpansionStop};
use crate::temporal::UndefinedCause;
use crate::value::evaluate_state_predicate;

/// How a state-graph predicate read at one state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PredicateReading {
    /// The predicate holds.
    True,
    /// The predicate does not hold.
    False,
    /// The predicate has no value at the state (ADR-018 UE-1).
    Undefined {
        /// Why it has no value.
        cause: UndefinedCause,
        /// The expression with no value, when the evaluator located it;
        /// `None` for an absent bound object.
        location: Option<Location>,
    },
    /// The evaluation was refused, with none undefined: the predicate's
    /// truth at the state is not known.
    Refused(ClauseOutcome),
}

impl ModelSystem<'_> {
    /// `predicate` at `state` with its `over` parameter bound to `over`
    /// (`None` for a claim with no `over`), under a fresh meter of the
    /// subject's budget.
    ///
    /// # Errors
    ///
    /// `resource_exhausted`/`insufficient-next-charge` naming the evaluation
    /// meter when the evaluation exhausts it;
    /// `runtime_invariant`/`established-invariant-broken` when the state
    /// cannot be observed or the evaluator faults.
    pub fn state_predicate(
        &self,
        state: &ModelSimState,
        predicate: &CheckedPredicateBody,
        over: Option<&ObjectReference>,
    ) -> Result<PredicateReading, ExpansionStop> {
        let walk = walk_of(self, state, None)?;
        let bound = match over {
            None => None,
            Some(binding) => {
                let Some(row) = walk.rows.iter().find(|row| {
                    row.reference.universe() == binding.universe()
                        && row.reference.object() == binding.object()
                }) else {
                    return Ok(PredicateReading::Undefined {
                        cause: UndefinedCause::Family(UndefinedRecord {
                            reason: UndefinedReason::AbsentKey,
                            fields: BTreeMap::new(),
                        }),
                        location: None,
                    });
                };
                Some(row.reference.clone())
            }
        };
        let evaluation = evaluate_state_predicate(
            self.package,
            predicate,
            &walk.observation.environment,
            bound,
            &mut Meter::new(self.meter),
        )
        .map_err(|_| stop(BROKEN))?;
        Ok(match verdict(&evaluation.outcome) {
            Verdict::True => PredicateReading::True,
            Verdict::False => PredicateReading::False,
            Verdict::Undecided(outcome) => match outcome.undefined_cause() {
                Some(cause) => PredicateReading::Undefined {
                    cause,
                    location: evaluation.location,
                },
                None if matches!(outcome, ClauseOutcome::Incomplete(_)) => {
                    return Err(exhausted(ExpansionLimit::EvaluationMeter))
                }
                None => PredicateReading::Refused(outcome),
            },
        })
    }
}
