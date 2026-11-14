// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-036 PA-20, ADR-018 SM-7: the realizability counterexample replay
//! arm. It re-walks the stated histories or defeating plays against the
//! arena (the checked graph under the premises, stepped by the one
//! `qsl-eval` system), checks each step is an admitted successor, and that
//! the walk ends in the stated violation. It is independent of the engine's
//! search: it reads no winning set, only the arena.

use qsl_eval::simulation::model::TerminalClass;
use qsl_eval::simulation::protocol::{ProtocolState, ProtocolTransition, Sc};

use super::arena::{finding_value, Arena, ArenaError, FindingValue, Seen};
use super::{RealizabilityCounterexample, StrategyAction};

/// Why a realizability counterexample's replay disagrees with the arena.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RealizabilityDisagreement {
    /// From no initial state is every stated step an admitted successor of
    /// the one before; `at` is the furthest step any walk reached.
    StepNotSuccessor {
        /// The index of the first step no walk admitted.
        at: usize,
    },
    /// The stated role is no role of the protocol.
    UnknownRole,
    /// The role's observations of the two histories differ.
    Distinguishable,
    /// The two stated actions are one action.
    SameAction,
    /// A stated action is not enabled for the role at its history's end.
    ActionNotEnabled,
    /// A defeating play does not end in a violation of the stated
    /// obligation, or no play is stated.
    NotTheStatedViolation,
}

/// The replay walk of a realizability counterexample (PA-20).
///
/// * Indistinguishable histories: both histories walk from an initial state
///   through admitted successors, the role observes them equally, the two
///   actions differ, and each is enabled for the role at its history's end
///   (`End` at an intended terminal state).
/// * Environment defeat: every defeating play walks from an initial state
///   through admitted successors and ends in a violation of the obligation:
///   a violated invariant of the clause, or a deadlock against `finish`.
///
/// # Errors
///
/// [`ArenaError`] when the arena cannot be read; a disagreement is the
/// inner `Err`.
pub fn replay_realizability_counterexample(
    arena: &Arena<'_, '_>,
    counterexample: &RealizabilityCounterexample,
) -> Result<Result<(), RealizabilityDisagreement>, ArenaError> {
    match counterexample {
        RealizabilityCounterexample::IndistinguishableHistories {
            role,
            history_a,
            history_b,
            action_a,
            action_b,
            obligation,
            ..
        } => {
            let Ok(role) = arena.names.roles().binary_search(role) else {
                return Ok(Err(RealizabilityDisagreement::UnknownRole));
            };
            if obligation != arena.names.finish() {
                return Ok(Err(RealizabilityDisagreement::NotTheStatedViolation));
            }
            let (end_a, seen_a) = match walk(arena, history_a)? {
                Ok(end) => end,
                Err(disagreement) => return Ok(Err(disagreement)),
            };
            let (end_b, seen_b) = match walk(arena, history_b)? {
                Ok(end) => end,
                Err(disagreement) => return Ok(Err(disagreement)),
            };
            if seen_a.observations()[role] != seen_b.observations()[role] {
                return Ok(Err(RealizabilityDisagreement::Distinguishable));
            }
            if action_a == action_b {
                return Ok(Err(RealizabilityDisagreement::SameAction));
            }
            for (end, action) in [(&end_a, action_a), (&end_b, action_b)] {
                if !enabled(arena, end, role, action)? {
                    return Ok(Err(RealizabilityDisagreement::ActionNotEnabled));
                }
            }
            Ok(Ok(()))
        }
        RealizabilityCounterexample::EnvironmentDefeat {
            defeats_per_branch,
            obligation,
            ..
        } => {
            if defeats_per_branch.is_empty() {
                return Ok(Err(RealizabilityDisagreement::NotTheStatedViolation));
            }
            for defeat in defeats_per_branch {
                let (end, _) = match walk(arena, &defeat.play)? {
                    Ok(end) => end,
                    Err(disagreement) => return Ok(Err(disagreement)),
                };
                let (expansion, terminal) = arena.system.successors_and_terminal(&end)?;
                let violated = match finding_value(&expansion.findings) {
                    Some(FindingValue::Lose) => obligation == arena.names.clause(),
                    Some(FindingValue::Undecided) => false,
                    None => {
                        expansion.successors.is_empty()
                            && terminal == TerminalClass::Deadlocked
                            && obligation == arena.names.finish()
                    }
                };
                if !violated {
                    return Ok(Err(RealizabilityDisagreement::NotTheStatedViolation));
                }
            }
            Ok(Ok(()))
        }
    }
}

/// The end state of `steps` walked from the first initial state that admits
/// every step, with what the walk showed the roles.
fn walk(
    arena: &Arena<'_, '_>,
    steps: &[ProtocolTransition],
) -> Result<Result<(ProtocolState<Sc>, Seen), RealizabilityDisagreement>, ArenaError> {
    let roles = arena.names.roles().len();
    let mut furthest = 0;
    'initial: for initial in arena.system.initial() {
        let mut seen = arena.bind(&initial, Seen::new(roles))?;
        let mut state = initial;
        for (at, step) in steps.iter().enumerate() {
            let (expansion, _) = arena.system.successors_and_terminal(&state)?;
            let Some((transition, next)) = expansion
                .successors
                .into_iter()
                .find(|(transition, _)| transition == step)
            else {
                furthest = furthest.max(at);
                continue 'initial;
            };
            let taken = arena.classify(&transition)?;
            seen = arena.bind(&next, seen.extended(taken.as_ref()))?;
            state = next;
        }
        return Ok(Ok((state, seen)));
    }
    Ok(Err(RealizabilityDisagreement::StepNotSuccessor { at: furthest }))
}

/// Whether `action` is enabled for `role` at `state`: `End` at an intended
/// terminal state with no violated invariant, a send or receive when some
/// successor is that step of the role.
fn enabled(
    arena: &Arena<'_, '_>,
    state: &ProtocolState<Sc>,
    role: usize,
    action: &StrategyAction,
) -> Result<bool, ArenaError> {
    let (expansion, terminal) = arena.system.successors_and_terminal(state)?;
    if *action == StrategyAction::End {
        return Ok(expansion.successors.is_empty()
            && terminal == TerminalClass::Intended
            && finding_value(&expansion.findings).is_none());
    }
    for (transition, _) in &expansion.successors {
        if arena
            .classify(transition)?
            .is_some_and(|step| step.role == role && step.action == *action)
        {
            return Ok(true);
        }
    }
    Ok(false)
}
