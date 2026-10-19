// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-145 (ADR-020 RC-2, RC-3, ADR-018 UE-5): [`replay_refinement_trace`],
//! the replay of a refinement counterexample's finite prefix.
//!
//! It re-executes the concrete steps through the concrete `ModelSystem` as
//! FR-128 does, recomputes the mapped abstract state at every position, and
//! reruns [`check_initial`] and [`check_step`] along the prefix, stopping at
//! the first failure. A failure equal to the recorded one, or an undefined
//! mapping equal to the recorded undefined evaluation, is
//! `reproduced-with-evaluated-witness`; a failure of another kind or at
//! another position, an undetermined check, or no failure at all settles
//! `inconclusive`, `ReplayParity`. Replay needs no model checker and reads no
//! path, environment variable or clock.
//!
//! What this does not do yet: the lasso arm (FR-143's liveness failures), and
//! the facade around it that recompiles the packages and resolves the
//! refinement node by its identity. The recompile is blocked on S4 emitting a
//! refinement, which today is refused as not emitted, so a refinement unit
//! does not compile through the replay request; the caller passes the systems
//! of an already-checked package.

use qsl_foundation::diagnostic::Category;
use qsl_semantics::model::state::ModelState;

use crate::execute::ReplayRefusal;
use crate::model_check::counterexample::{
    CounterexampleKind, TemporalCounterexample, UndefinedEvaluation, UndefinedLocus,
};
use crate::model_check::replay::{
    disagreement, fault, no_value_cause, reexecute, refused, ModelTraceRefusal,
};
use crate::refinement::{
    check_initial, check_step, map_state, ConcreteStep, RefinementFailure, RefinementPosition,
    RefinementSystems, StepVerdict,
};
use crate::result::DisagreementCause;
use qsl_semantics::check::CheckedRefinement;

/// FR-145's settlement of a refinement counterexample's replay.
#[derive(Clone, Debug)]
pub enum RefinementTraceSettlement {
    /// `reproduced-with-evaluated-witness`: replay reached the recorded
    /// failure.
    Reproduced {
        /// The failure it reproduced.
        failure: RefinementFailure,
        /// The abstract state each position maps to, from position 0 to the
        /// failing position, the failing one included when it maps.
        mapped: Vec<ModelState>,
    },
    /// `inconclusive`, `ReplayParity`: replay disagrees with the engine.
    Inconclusive(DisagreementCause),
}

/// What running the checks along the prefix found.
enum Found {
    /// Every position passed.
    Nothing,
    /// A check failed at `position`.
    Failed(RefinementFailure),
    /// A mapping row or argument was undefined.
    Undefined(Box<UndefinedEvaluation>),
    /// A check could not decide.
    Undetermined,
}

/// FR-145: replay `payload`, a refinement counterexample of `refinement`
/// over `systems`.
///
/// # Errors
///
/// [`ReplayRefusal`] when the prefix does not re-execute (FR-128's refusals:
/// an `initial_state` out of range, a step not enabled, a post-state no
/// successor has), the payload names no refinement failure or is a lasso,
/// or an internal fault occurs.
pub fn replay_refinement_trace(
    refinement: &CheckedRefinement,
    systems: &RefinementSystems<'_>,
    payload: &TemporalCounterexample,
) -> Result<RefinementTraceSettlement, ReplayRefusal> {
    let Some(recorded) = payload.refinement.as_ref() else {
        return Err(fault("a-refinement-payload-names-its-failure"));
    };
    if payload.lasso.is_some() {
        return Err(refused(ModelTraceRefusal::RefinementLasso));
    }
    let Some(positions) = reexecute(&systems.concrete, payload)? else {
        return Ok(RefinementTraceSettlement::Inconclusive(no_value_cause()));
    };
    let mut mapped = Vec::new();
    let mut previous: Option<RefinementPosition> = None;
    let mut found = Found::Nothing;
    for (index, position) in positions.iter().enumerate() {
        let verdict = match (&previous, index.checked_sub(1)) {
            (Some(pre), Some(step)) => {
                let Some(taken) = payload.prefix.get(step) else {
                    return Err(fault("a-position-has-a-step"));
                };
                check_step(
                    refinement,
                    &systems.concrete,
                    &systems.abstract_system,
                    pre,
                    &ConcreteStep {
                        position: u64::try_from(index).unwrap_or(u64::MAX),
                        transition: Some(&taken.transition),
                        post: &position.state,
                    },
                    &mut 0,
                )
            }
            _ => check_initial(
                refinement,
                &systems.concrete,
                &systems.abstract_system,
                payload.initial_state,
                position.state.state(),
            ),
        };
        match verdict {
            StepVerdict::Passes { post, .. } => {
                mapped.extend(post.abstract_states.first().cloned());
                previous = Some(post);
            }
            StepVerdict::Fails(failure) => {
                found = Found::Failed(failure);
                mapped.extend(
                    map_state(
                        refinement,
                        &systems.concrete,
                        &systems.abstract_system,
                        position.state.state(),
                    )
                    .ok()
                    .map(|state| state.visible),
                );
                break;
            }
            StepVerdict::Undefined(evaluation) => {
                found = Found::Undefined(Box::new(evaluation));
                break;
            }
            StepVerdict::Undetermined(_) => {
                found = Found::Undetermined;
                break;
            }
            StepVerdict::Fault(_) => return Err(fault("refinement-step")),
        }
    }
    Ok(match found {
        Found::Failed(failure)
            if payload.kind == CounterexampleKind::Formula && *recorded == failure =>
        {
            RefinementTraceSettlement::Reproduced { failure, mapped }
        }
        Found::Failed(_) => {
            RefinementTraceSettlement::Inconclusive(disagreement(Category::Violation))
        }
        Found::Undefined(evaluation) => match (&payload.kind, &evaluation.site.locus) {
            (CounterexampleKind::UndefinedEvaluation(claimed), UndefinedLocus::Mapping(site))
                if claimed == evaluation.as_ref()
                    && *recorded
                        == (RefinementFailure::Undefined {
                            position: evaluation.site.position,
                            row: site.row.clone(),
                        }) =>
            {
                RefinementTraceSettlement::Reproduced {
                    failure: recorded.clone(),
                    mapped,
                }
            }
            _ => RefinementTraceSettlement::Inconclusive(disagreement(Category::Violation)),
        },
        Found::Undetermined => RefinementTraceSettlement::Inconclusive(no_value_cause()),
        Found::Nothing => RefinementTraceSettlement::Inconclusive(disagreement(Category::Success)),
    })
}
