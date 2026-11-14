// SPDX-License-Identifier: AGPL-3.0-or-later
//! ADR-036 PA-20, ADR-018 SM-7: the realizability counterexample replay
//! arm. It re-walks the stated plays against the arena (the checked graph
//! under the premises, stepped by the one `qsl-eval` system) and checks that
//! they end in the stated violation. Every play names its initial state and
//! each step's successor index, so the walk is determinate and guesses
//! nothing. It is independent of the engine's search: it reads no winning
//! set, only the arena and the counterexample.

use std::collections::BTreeSet;

use qsl_eval::simulation::model::TerminalClass;
use qsl_eval::simulation::protocol::{ProtocolState, Sc};
use qsl_eval::simulation::TransitionSystem;
use quire_contract_model::CheckedNodeId;

use super::arena::{finding_value, Arena, ArenaError, FindingValue, Seen};
use super::{
    AssessedRoot, Play, PlayStep, ProtocolAnalysisKey, RealizabilityCounterexample, StrategyAction,
};

/// Why a realizability counterexample's replay disagrees with the arena or
/// the request.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RealizabilityDisagreement {
    /// The counterexample's key is not the request's realizability key.
    KeyMismatch,
    /// The FR-351 record is not the one the request's root and the
    /// counterexample give.
    RecordMismatch,
    /// A play's initial state does not exist, or a step is not the admitted
    /// successor its index names; `at` is the step's index in its play.
    StepNotSuccessor {
        /// The step's index.
        at: usize,
    },
    /// A play exceeds `max_depth`, or a history's end that the role could
    /// still step from exceeds `max_history`.
    BeyondBounds,
    /// The stated role is no role of the protocol.
    UnknownRole,
    /// The role's observations of the two histories differ.
    Distinguishable,
    /// The two stated actions are one action.
    SameAction,
    /// A stated action is not enabled for the role at its history's end.
    ActionNotEnabled,
    /// An action is enabled at both ends, so the two ends need not require
    /// different actions.
    NotDisjoint,
    /// At a state where a play takes a role step some move is another step,
    /// so the role could delay it or another could reorder around it.
    Avoidable,
    /// Another role or the environment can move at a history's end, so the
    /// role could wait there for an action the end does not enable yet.
    NotSoleMover,
    /// A state a play passes through, or a history's end, carries a finding
    /// (a violated invariant or an undecided evaluation), which the engine
    /// never steps past or decides at.
    FindingOnPlay,
    /// A defeating play does not end in a violation of the stated
    /// obligation, or no play is stated.
    NotTheStatedViolation,
}

type Walked = Result<(ProtocolState<Sc>, Seen), RealizabilityDisagreement>;

/// The replay walk of a realizability counterexample (PA-20).
///
/// Both forms: the key equals the request's `key`, the FR-351 record is the
/// one `root` gives for the obligation and position, and every play walks
/// from its initial state through the successors its steps name, within
/// `max_depth`, through states with no finding, taking environment steps,
/// which no table filters, or role steps where every move is that step, so
/// no role can delay it or reorder around it (a role waits until its
/// committed action is enabled, QSpec FR-175).
///
/// * Indistinguishable histories: neither end carries a finding; the role
///   observes the two ends equally, within `max_history` where it can still
///   step; the two actions differ and each is enabled at its own end; the
///   obligation is `finish`; every move at either end is a step of the
///   role; and no action is enabled at both ends (PA-18 (i)). Every profile
///   either is stuck on a play or reaches both ends, where its one action
///   for the class is not enabled at one of them and nothing else can move.
///   At a forced pair no node is lost, so the not-lost sets are the enabled
///   sets.
/// * Environment defeat: every defeating play ends in a violation of the
///   obligation: a violated invariant of the clause, or a deadlock against
///   `finish`.
///
/// # Errors
///
/// [`ArenaError`] when the arena cannot be read; a disagreement is the
/// inner `Err`.
pub fn replay_realizability_counterexample(
    arena: &Arena<'_, '_>,
    key: &ProtocolAnalysisKey,
    root: &AssessedRoot,
    counterexample: &RealizabilityCounterexample,
) -> Result<Result<(), RealizabilityDisagreement>, ArenaError> {
    let ProtocolAnalysisKey::Realizability(held) = key else {
        return Ok(Err(RealizabilityDisagreement::KeyMismatch));
    };
    let max_depth = held.bounds.max_depth;
    let max_history = held.bounds.max_history.unwrap_or_default();
    let within = |steps: usize| u64::try_from(steps).is_ok_and(|steps| steps <= max_depth);
    let (stated_key, obligation, record, position) = match counterexample {
        RealizabilityCounterexample::IndistinguishableHistories {
            key,
            history_a,
            history_b,
            obligation,
            separating_witness,
            ..
        } => (
            key,
            obligation,
            separating_witness,
            history_a
                .steps
                .len()
                .max(history_b.steps.len())
                .saturating_sub(1),
        ),
        RealizabilityCounterexample::EnvironmentDefeat {
            key,
            defeats_per_branch,
            obligation,
            separating_witness,
        } => (
            key,
            obligation,
            separating_witness,
            defeats_per_branch
                .first()
                .map_or(0, |defeat| defeat.play.steps.len().saturating_sub(1)),
        ),
    };
    if stated_key != key {
        return Ok(Err(RealizabilityDisagreement::KeyMismatch));
    }
    if root.record(obligation, position).ok().as_ref() != Some(record) {
        return Ok(Err(RealizabilityDisagreement::RecordMismatch));
    }
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
            if !within(history_a.steps.len()) || !within(history_b.steps.len()) {
                return Ok(Err(RealizabilityDisagreement::BeyondBounds));
            }
            let (end_a, seen_a) = match walk(arena, history_a)? {
                Ok(end) => end,
                Err(disagreement) => return Ok(Err(disagreement)),
            };
            let (end_b, seen_b) = match walk(arena, history_b)? {
                Ok(end) => end,
                Err(disagreement) => return Ok(Err(disagreement)),
            };
            let moves_a = moves(arena, &end_a, role)?;
            let moves_b = moves(arena, &end_b, role)?;
            if moves_a.finding || moves_b.finding {
                return Ok(Err(RealizabilityDisagreement::FindingOnPlay));
            }
            // The engine decides an end's findings and terminal class before
            // it cuts at `max_history`, so a won leaf may lie past it.
            if (moves_a.open && seen_a.exceeds(max_history))
                || (moves_b.open && seen_b.exceeds(max_history))
            {
                return Ok(Err(RealizabilityDisagreement::BeyondBounds));
            }
            if seen_a.observations()[role] != seen_b.observations()[role] {
                return Ok(Err(RealizabilityDisagreement::Distinguishable));
            }
            if action_a == action_b {
                return Ok(Err(RealizabilityDisagreement::SameAction));
            }
            if !moves_a.enabled.contains(action_a) || !moves_b.enabled.contains(action_b) {
                return Ok(Err(RealizabilityDisagreement::ActionNotEnabled));
            }
            if !moves_a.sole || !moves_b.sole {
                return Ok(Err(RealizabilityDisagreement::NotSoleMover));
            }
            if !moves_a.enabled.is_disjoint(&moves_b.enabled) {
                return Ok(Err(RealizabilityDisagreement::NotDisjoint));
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
                if !within(defeat.play.steps.len()) {
                    return Ok(Err(RealizabilityDisagreement::BeyondBounds));
                }
                let (end, _) = match walk(arena, &defeat.play)? {
                    Ok(end) => end,
                    Err(disagreement) => return Ok(Err(disagreement)),
                };
                if violation(arena, &end)? != Some(obligation) {
                    return Ok(Err(RealizabilityDisagreement::NotTheStatedViolation));
                }
            }
            Ok(Ok(()))
        }
    }
}

/// The end state of `play`, walked from its initial state through the
/// successors its steps name, with what the walk showed the roles.
fn walk(arena: &Arena<'_, '_>, play: &Play) -> Result<Walked, ArenaError> {
    let Some(initial) = arena.system.initial().into_iter().nth(play.initial) else {
        return Ok(Err(RealizabilityDisagreement::StepNotSuccessor { at: 0 }));
    };
    let seen = arena.bind(&initial, Seen::new(arena.names.roles().len()))?;
    walk_from(arena, initial, seen, &play.steps)
}

/// The end state of `steps` from `state`, each step the successor at its
/// index with an equal transition, taken from a state with no finding; an
/// environment step is always forced (no table filters it), a role step only
/// where every move is that same step (PA-18: no role can delay it or
/// reorder around it).
fn walk_from(
    arena: &Arena<'_, '_>,
    mut state: ProtocolState<Sc>,
    mut seen: Seen,
    steps: &[PlayStep],
) -> Result<Walked, ArenaError> {
    for (at, step) in steps.iter().enumerate() {
        let (expansion, _) = arena.system.successors_and_terminal(&state)?;
        if finding_value(&expansion.findings).is_some() {
            return Ok(Err(RealizabilityDisagreement::FindingOnPlay));
        }
        let mut classified = Vec::with_capacity(expansion.successors.len());
        for (transition, _) in &expansion.successors {
            classified.push(arena.classify(transition)?);
        }
        let Some((transition, next)) = expansion.successors.into_iter().nth(step.successor) else {
            return Ok(Err(RealizabilityDisagreement::StepNotSuccessor { at }));
        };
        if transition != step.transition {
            return Ok(Err(RealizabilityDisagreement::StepNotSuccessor { at }));
        }
        let identity = |held: &Option<super::arena::RoleStep>| {
            held.as_ref().map(|held| (held.role, held.action.clone()))
        };
        let taken = identity(&classified[step.successor]);
        if taken.is_some() && classified.iter().any(|held| identity(held) != taken) {
            return Ok(Err(RealizabilityDisagreement::Avoidable));
        }
        seen = arena.bind(&next, seen.extended(classified[step.successor].as_ref()))?;
        state = next;
    }
    Ok(Ok((state, seen)))
}

/// The obligation `state` violates: the clause for a violated invariant,
/// `finish` for a deadlock; `None` for any other state.
fn violation<'n>(
    arena: &'n Arena<'_, '_>,
    state: &ProtocolState<Sc>,
) -> Result<Option<&'n CheckedNodeId>, ArenaError> {
    let (expansion, terminal) = arena.system.successors_and_terminal(state)?;
    Ok(match finding_value(&expansion.findings) {
        Some(FindingValue::Lose) => Some(arena.names.clause()),
        Some(FindingValue::Undecided) => None,
        None => (expansion.successors.is_empty() && terminal == TerminalClass::Deadlocked)
            .then(|| arena.names.finish()),
    })
}

/// What `role` can do at a state.
struct Moves {
    /// Its sends and receives among the successors, and `End` at an
    /// intended terminal state with no violated invariant.
    enabled: BTreeSet<StrategyAction>,
    /// The state carries a finding.
    finding: bool,
    /// Every successor is a step of the role.
    sole: bool,
    /// The state has a successor.
    open: bool,
}

fn moves(
    arena: &Arena<'_, '_>,
    state: &ProtocolState<Sc>,
    role: usize,
) -> Result<Moves, ArenaError> {
    let (expansion, terminal) = arena.system.successors_and_terminal(state)?;
    let mut enabled = BTreeSet::new();
    if expansion.successors.is_empty()
        && terminal == TerminalClass::Intended
        && finding_value(&expansion.findings).is_none()
    {
        enabled.insert(StrategyAction::End);
    }
    let mut sole = true;
    for (transition, _) in &expansion.successors {
        match arena.classify(transition)? {
            Some(taken) if taken.role == role => {
                enabled.insert(taken.action);
            }
            _ => sole = false,
        }
    }
    Ok(Moves {
        enabled,
        finding: finding_value(&expansion.findings).is_some(),
        sole,
        open: !expansion.successors.is_empty(),
    })
}
