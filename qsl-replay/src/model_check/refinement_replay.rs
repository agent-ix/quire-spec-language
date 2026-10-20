// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-145 (ADR-020 RC-2, RC-3, ADR-018 UE-5): [`replay_refinement_trace`],
//! the replay of a refinement counterexample.
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
//! A lasso (`Divergence` or `AbstractUnfair`, FR-143) is replayed over the
//! prefix and one pass of its loop. Its loop must close in the product, with
//! the history values and candidates it entered with, and be fair under
//! `F_C` as FR-128 reads fairness from the concrete system. The recorded
//! `F_A` constraint must then be enabled at the mapped state of every loop
//! state and taken by no loop step, and the loop is `Divergence` when every
//! step is a `stutter`, `AbstractUnfair` otherwise.
//!
//! What this does not do yet: the facade around it that recompiles the
//! packages and resolves the refinement node by its identity. The recompile is
//! blocked on S4 emitting a refinement, which today is refused as not
//! emitted, so a refinement unit does not compile through the replay request;
//! the caller passes the systems of an already-checked package.

use qsl_eval::simulation::model::ModelFinding;
use qsl_foundation::diagnostic::Category;
use qsl_semantics::model::state::ModelState;
use quire_semantic_value::fairness::FairnessKind;
use quire_semantic_value::model_transition::ModelTransition;

use crate::execute::ReplayRefusal;
use crate::model_check::counterexample::{
    CounterexampleKind, Lasso, TemporalCounterexample, UndefinedEvaluation, UndefinedLocus,
};
use crate::model_check::limits::ReachedLimit;
use crate::model_check::replay::{
    close_loop, disagreement, fault, loop_states_of, no_value_cause, reexecute, refused,
    unfair_constraint, ModelTraceRefusal, Position,
};
use crate::refinement::{
    abstract_classes, check_initial, check_step, initial_history, map_state, step_history,
    stop_verdict, ConcreteStep, RefinementFailure, RefinementPosition, RefinementSystems,
    StepVerdict, Taken,
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
    /// A run budget stopped the replay: V-7 with the limit and its value
    /// (FR-145-AC-6), never `ReplayParity`.
    Stopped(ReachedLimit),
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
    /// A budget stopped a check.
    Stopped(ReachedLimit),
}

/// FR-145: replay `payload`, a refinement counterexample of `refinement`
/// over `systems`.
///
/// # Errors
///
/// [`ReplayRefusal`] when the prefix does not re-execute (FR-128's refusals:
/// an `initial_state` out of range, a step not enabled, a post-state no
/// successor has, a loop that does not close), the payload names no
/// refinement failure, a lasso's history values or candidates differ at the
/// end of its loop from its entry, a lasso is unfair under `F_C`, or an
/// internal fault occurs.
pub fn replay_refinement_trace(
    refinement: &CheckedRefinement,
    systems: &RefinementSystems<'_>,
    payload: &TemporalCounterexample,
) -> Result<RefinementTraceSettlement, ReplayRefusal> {
    let Some(recorded) = payload.refinement.as_ref() else {
        return Err(refused(ModelTraceRefusal::NoRefinementFailure));
    };
    let Some(positions) = reexecute(&systems.concrete, payload)? else {
        return Ok(RefinementTraceSettlement::Inconclusive(no_value_cause()));
    };
    if let Some(lasso) = payload.lasso {
        close_loop(&systems.concrete, &positions, lasso)?;
    }
    let mut mapped = Vec::new();
    let mut previous: Option<RefinementPosition> = None;
    // The position and the abstract steps taken at each step, for a lasso.
    let mut trail = Vec::new();
    let mut taken_by = Vec::new();
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
                )
                .verdict
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
            StepVerdict::Passes { post, taken } => {
                mapped.extend(post.abstract_states.first().cloned());
                trail.push(post.clone());
                if index > 0 {
                    taken_by.push(taken);
                }
                previous = Some(post);
            }
            StepVerdict::Fails(failure) => {
                found = Found::Failed(failure);
                // The failing position's history is the one its step gives.
                let history = match (&previous, payload.prefix.get(index.saturating_sub(1))) {
                    (Some(pre), Some(taken)) if index > 0 => step_history(
                        refinement,
                        &systems.concrete,
                        &pre.history,
                        &pre.concrete,
                        &taken.transition,
                        &position.state,
                    )
                    .ok(),
                    _ => Some(initial_history(
                        refinement,
                        &systems.concrete,
                        position.state.state(),
                    )),
                };
                mapped.extend(history.and_then(|history| {
                    map_state(
                        refinement,
                        &systems.concrete,
                        &systems.abstract_system,
                        position.state.state(),
                        &history,
                    )
                    .ok()
                    .map(|state| state.visible)
                }));
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
            StepVerdict::Stopped(reached) => {
                found = Found::Stopped(reached);
                break;
            }
            StepVerdict::Fault(_) => return Err(fault("refinement-step")),
        }
    }
    if let Some(lasso) = payload.lasso {
        let walk = Walk {
            positions: &positions,
            trail,
            taken_by,
        };
        return replay_lasso(refinement, systems, payload, lasso, walk, found, mapped);
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
        Found::Stopped(reached) => RefinementTraceSettlement::Stopped(reached),
        Found::Nothing => RefinementTraceSettlement::Inconclusive(disagreement(Category::Success)),
    })
}

/// What replay recomputed along a lasso's prefix and one pass of its loop.
struct Walk<'p> {
    /// The concrete positions, re-executed.
    positions: &'p [Position],
    /// The refinement position at each concrete position.
    trail: Vec<RefinementPosition>,
    /// The abstract steps the step leaving each position took.
    taken_by: Vec<Taken>,
}

/// The settlement of a lasso whose prefix and loop `check_step` has run along:
/// `found` is what the checks found, `walk` what they recomputed.
fn replay_lasso(
    refinement: &CheckedRefinement,
    systems: &RefinementSystems<'_>,
    payload: &TemporalCounterexample,
    lasso: Lasso,
    mut walk: Walk<'_>,
    found: Found,
    mapped: Vec<ModelState>,
) -> Result<RefinementTraceSettlement, ReplayRefusal> {
    match found {
        Found::Nothing => {}
        Found::Failed(_) | Found::Undefined(_) => {
            return Ok(RefinementTraceSettlement::Inconclusive(disagreement(
                Category::Violation,
            )))
        }
        Found::Undetermined => {
            return Ok(RefinementTraceSettlement::Inconclusive(no_value_cause()))
        }
        Found::Stopped(reached) => return Ok(RefinementTraceSettlement::Stopped(reached)),
    }
    let (Some(RefinementFailure::Divergence { constraint })
    | Some(RefinementFailure::AbstractUnfair { constraint })) = payload.refinement.as_ref()
    else {
        return Err(refused(ModelTraceRefusal::NoRefinementFailure));
    };
    let steps = walk.positions.len() - 1;
    // The terminal stutter step is one more step of the product.
    if lasso.terminal_stutter {
        let (Some(last), Some(position)) = (walk.trail.last(), walk.positions.last()) else {
            return Err(fault("a-lasso-has-a-position"));
        };
        let verdict = check_step(
            refinement,
            &systems.concrete,
            &systems.abstract_system,
            last,
            &ConcreteStep {
                position: u64::try_from(steps + 1).unwrap_or(u64::MAX),
                transition: None,
                post: &position.state,
            },
        )
        .verdict;
        match verdict {
            StepVerdict::Passes { post, taken } => {
                walk.trail.push(post);
                walk.taken_by.push(taken);
            }
            StepVerdict::Stopped(reached) => {
                return Ok(RefinementTraceSettlement::Stopped(reached))
            }
            StepVerdict::Fault(_) => return Err(fault("refinement-step")),
            StepVerdict::Fails(_) | StepVerdict::Undefined(_) | StepVerdict::Undetermined(_) => {
                return Ok(RefinementTraceSettlement::Inconclusive(disagreement(
                    Category::Violation,
                )))
            }
        }
    }
    let (Some(entry), Some(end)) = (walk.trail.get(lasso.entry), walk.trail.last()) else {
        return Err(fault("a-lasso-has-an-entry-and-an-end"));
    };
    if !same_product_state(systems, entry, end)? {
        return Err(refused(ModelTraceRefusal::LoopDrifts {
            entry: lasso.entry,
        }));
    }
    let Some(loop_states) = loop_states_of(&systems.concrete, payload, walk.positions, lasso)?
    else {
        return Ok(RefinementTraceSettlement::Inconclusive(no_value_cause()));
    };
    if let Some(constraint) = unfair_constraint(
        &systems.concrete,
        &loop_states,
        lasso,
        refinement.concrete_fairness(),
    )? {
        return Err(ReplayRefusal::UnfairLasso { constraint });
    }
    // A strong constraint is outside the weak reading the engine writes
    // lassos by, so no lasso of one is replayed as a reproduction.
    let Some(index) = refinement
        .abstract_fairness()
        .iter()
        .position(|abstract_constraint| {
            abstract_constraint == constraint && abstract_constraint.kind() == FairnessKind::Weak
        })
    else {
        return Ok(RefinementTraceSettlement::Inconclusive(disagreement(
            Category::Success,
        )));
    };
    // The loop's states are the positions from its entry; a terminal stutter
    // loop is the one state it stands at.
    let from = lasso.entry;
    let (Some(loop_trail), Some(loop_taken)) = (walk.trail.get(from..), walk.taken_by.get(from..))
    else {
        return Err(fault("a-lasso-loop-is-in-the-trail"));
    };
    let loop_trail = &loop_trail[..loop_taken.len()];
    let mut enabled = Vec::with_capacity(loop_trail.len());
    for position in loop_trail {
        match abstract_enabled(systems, position)? {
            Enabledness::Enabled(here) => enabled.push(here),
            Enabledness::Undecided => {
                return Ok(RefinementTraceSettlement::Inconclusive(no_value_cause()))
            }
            Enabledness::Stopped(reached) => {
                return Ok(RefinementTraceSettlement::Stopped(reached))
            }
        }
    }
    let identities: Vec<ModelTransition> = enabled
        .iter()
        .flatten()
        .chain(loop_taken.iter().flat_map(|taken| match taken {
            Taken::Abstract(transitions) => transitions.as_slice(),
            Taken::Stutter => &[],
        }))
        .cloned()
        .collect();
    let classes = abstract_classes(
        &systems.abstract_system,
        refinement.abstract_fairness(),
        &identities,
    )
    .map_err(ReplayRefusal::Fault)?;
    let violated = classes.iter().any(|class| {
        class.constraint() == index
            && enabled.iter().all(|here| class.is_enabled_in(here))
            && !loop_taken.iter().any(|taken| class.is_taken_by(taken))
    });
    let divergent = loop_taken.iter().all(|taken| *taken == Taken::Stutter);
    Ok(match (payload.refinement.as_ref(), violated, divergent) {
        (Some(RefinementFailure::Divergence { .. }), true, true)
        | (Some(RefinementFailure::AbstractUnfair { .. }), true, false) => {
            RefinementTraceSettlement::Reproduced {
                failure: payload
                    .refinement
                    .clone()
                    .ok_or_else(|| fault("a-lasso-names-a-failure"))?,
                mapped,
            }
        }
        (_, true, _) => RefinementTraceSettlement::Inconclusive(disagreement(Category::Violation)),
        (_, false, _) => RefinementTraceSettlement::Inconclusive(disagreement(Category::Success)),
    })
}

/// Whether two refinement positions of one concrete state are one product
/// state: equal history values and equal candidate sets.
fn same_product_state(
    systems: &RefinementSystems<'_>,
    left: &RefinementPosition,
    right: &RefinementPosition,
) -> Result<bool, ReplayRefusal> {
    let encodes = |_| fault("a-product-state-encodes");
    let candidates = |position: &RefinementPosition| {
        position
            .abstract_states
            .iter()
            .map(|state| {
                systems
                    .abstract_system
                    .state_key_of(state)
                    .map(|(key, _)| key)
            })
            .collect::<Result<Vec<_>, _>>()
    };
    Ok(left.history.key(&systems.concrete).map_err(encodes)?
        == right.history.key(&systems.concrete).map_err(encodes)?
        && candidates(left).map_err(encodes)? == candidates(right).map_err(encodes)?)
}

/// What enabledness at a refinement position is.
enum Enabledness {
    /// The abstract transition identities enabled from the position's
    /// mapped states.
    Enabled(Vec<ModelTransition>),
    /// A contract conjunction of the abstract model was undecided.
    Undecided,
    /// A budget stopped the expansion.
    Stopped(ReachedLimit),
}

/// The abstract transition identities enabled from the mapped state of
/// `position` (ADR-020 CO-2).
fn abstract_enabled(
    systems: &RefinementSystems<'_>,
    position: &RefinementPosition,
) -> Result<Enabledness, ReplayRefusal> {
    let mut enabled = Vec::new();
    for candidate in &position.abstract_states {
        let expansion = match systems.abstract_system.expand_state(candidate) {
            Ok(expansion) => expansion,
            Err(stop) => {
                return match stop_verdict(&systems.abstract_system, stop) {
                    StepVerdict::Stopped(reached) => Ok(Enabledness::Stopped(reached)),
                    _ => Err(fault("abstract-expansion")),
                }
            }
        };
        if expansion
            .findings
            .iter()
            .any(|finding| matches!(finding, ModelFinding::ContractUndetermined { .. }))
        {
            return Ok(Enabledness::Undecided);
        }
        enabled.extend(
            expansion
                .successors
                .into_iter()
                .map(|(transition, _)| transition),
        );
    }
    Ok(Enabledness::Enabled(enabled))
}
