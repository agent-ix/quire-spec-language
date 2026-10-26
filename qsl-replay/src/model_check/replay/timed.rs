// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-237 (ADR-026 CT-3, TD-3): the replay of a counterexample over a timed
//! subject, in exact rational arithmetic and with no zone engine.
//!
//! Every clock starts at 0. Each step's delay must be admissible where it
//! starts (FR-231 "Delay"); after it, the step's transition identity must
//! be enabled (a convex part of its guard holds), its successor is the one
//! whose post-state digest, clocks included, is the recorded one, and the
//! target's time invariants must hold after the resets. A lasso's loop must
//! return to its entry's clock values with a positive total delay, and be
//! fair over time (FR-232). The formula is evaluated over the timed lasso,
//! or over the positions for a clause whose profile reads no time. A finite
//! violation settles only where the last replayed state has a
//! time-divergent continuation, which a fresh exploration of its forward
//! closure finds ([`crate::timed::explore`]); a time-lock counterexample
//! settles by TD-1's two arms.

use qsl_eval::simulation::model::{
    GuardedStep, ModelSystem, TimedBehaviour, TimedState, TimedSystem,
};
use qsl_eval::simulation::Limits;
use qsl_eval::temporal::{
    evaluate_timed_at, TemporalOutcome, TemporalPosition, TemporalRefusal, TimedTrace,
};
use qsl_semantics::check::TemporalProfile;
use quire_exact::{Integer, Meter, Rational};
use quire_semantic_value::catalog::Category;
use quire_semantic_value::fairness::{FairnessConstraint, FairnessGranularity, FairnessKind};

use super::{
    check_formula, digest_of, disagreement, disagrees, fault, no_value, position_of, refused,
    replayed_letters, undefined, undefined_letter, FormulaReplay, ItemReads, LetterRead,
    ModelTraceRefusal, ModelTraceSettlement, ModelTraceValue, Position, StrongFairnessHint,
    UndefinedLetter,
};
use crate::execute::ReplayRefusal;
use crate::model_check::counterexample::{
    CounterexampleKind, Lasso, TemporalCounterexample, TimedPoint, UndefinedEvaluation,
};
use crate::model_check::limits::{ModelCheckLimit, ReachedLimit};
use crate::timed::explore::{explore, Closure};

/// The timed subject a recompiled package's system is over: the timed
/// model of its time source, or `None` for an untimed subject.
///
/// # Errors
///
/// [`ModelTraceRefusal::TimedSubject`] when the timed model does not build
/// a timed subject.
pub(super) fn timed_subject<'s, 'a>(
    system: &'s ModelSystem<'a>,
    package: &'a qsl_package::CheckedPackage,
) -> Result<Option<TimedSystem<'s, 'a>>, ReplayRefusal> {
    let graph = package.graph();
    let Some(source) = graph
        .time_sources()
        .iter()
        .find(|source| graph.timed_model(&source.model).is_some())
    else {
        return Ok(None);
    };
    TimedSystem::new(system, &source.model)
        .map(Some)
        .map_err(|refusal| refused(ModelTraceRefusal::TimedSubject(refusal.to_string())))
}

/// One replayed timed position: the timed state, its time stamp and its
/// model state's position.
struct TimedPosition {
    state: TimedState,
    stamp: Rational,
    /// The timed state's FR-236 digest, clocks included.
    digest: qsl_foundation::digest::DigestRecord,
}

fn zero() -> Rational {
    Rational::from_integer(Integer::zero())
}

/// FR-237: settle `payload` over the timed subject `timed`; `exploration`
/// bounds the fresh exploration of a finite violation's endpoint and of a
/// non-local time-lock.
pub(super) fn settle(
    timed: &TimedSystem<'_, '_>,
    payload: &TemporalCounterexample,
    formula: Option<FormulaReplay<'_>>,
    exploration: &Limits,
) -> Result<(ModelTraceSettlement, Vec<StrongFairnessHint>), ReplayRefusal> {
    let Some(replayed) = replay_prefix(timed, payload)? else {
        return Ok((no_value(), Vec::new()));
    };
    let system = timed.system();
    let mut positions = Vec::with_capacity(replayed.len());
    for position in &replayed {
        let state = position.state.discrete().clone();
        let digest = digest_of(system, &state)?;
        positions.push(Position { state, digest });
    }
    if let Some(lasso) = payload.lasso {
        close_loop(timed, &replayed, lasso)?;
        if let Some(formula) = &formula {
            match unfair(timed, payload, lasso, formula.clause.fairness())? {
                Fairness::Fair => {}
                Fairness::Unfair(constraint) => {
                    return Err(ReplayRefusal::UnfairLasso { constraint })
                }
                Fairness::NoValue => return Ok((no_value(), Vec::new())),
            }
        }
    }
    let last = replayed.last().ok_or_else(|| fault("a-position-exists"))?;
    let settlement = match (&payload.kind, formula) {
        (CounterexampleKind::Race(_), _) => return Err(refused(ModelTraceRefusal::RaceOverModel)),
        (CounterexampleKind::TimeLock, _) => time_lock(timed, payload, last, exploration)?,
        (CounterexampleKind::Deadlock, _) => match timed.deadlocked(&last.state) {
            Ok(true) => ModelTraceSettlement::Reproduced {
                value: ModelTraceValue::Truth(false),
                trace_position: payload.trace_position(),
            },
            Ok(false) => disagrees(Category::Success),
            Err(_) => no_value(),
        },
        (CounterexampleKind::UndefinedEvaluation(claimed), formula) => {
            let reads = match &formula {
                Some(formula) => {
                    let read = if formula.clause.profile() == TemporalProfile::Timed {
                        timed_undefined_letter(system, payload, &positions, &replayed, formula)?
                    } else {
                        undefined_letter(system, payload, &positions, formula)?
                    };
                    match read {
                        LetterRead::NoValue => return Ok((no_value(), Vec::new())),
                        LetterRead::Read(first) => ItemReads::Authored(first),
                    }
                }
                None => ItemReads::Deadlock,
            };
            timed_undefined(timed, payload, &positions, &replayed, claimed, &reads)?
        }
        (CounterexampleKind::Formula, Some(formula)) => {
            if formula.clause.profile() == TemporalProfile::Timed {
                timed_formula(timed, payload, &positions, &replayed, &formula)?
            } else {
                // Fairness over time is checked above; the clause reads no
                // time, so it is evaluated over the positions.
                check_formula(system, payload, &positions, &formula, |_| {
                    Ok(Ok(Vec::new()))
                })?
                .0
            }
        }
        (CounterexampleKind::Formula, None) => {
            return Err(fault("formula-counterexample-names-a-clause"))
        }
    };
    let finite = payload.lasso.is_none()
        && matches!(
            payload.kind,
            CounterexampleKind::Formula
                | CounterexampleKind::Deadlock
                | CounterexampleKind::UndefinedEvaluation(_)
        );
    if finite && matches!(settlement, ModelTraceSettlement::Reproduced { .. }) {
        // FR-237 "Admitted endpoint": the violation counts only where time
        // can diverge after it (ADR-026 TD-1, TV-1).
        return Ok((
            admitted(timed, &last.state, exploration, settlement),
            Vec::new(),
        ));
    }
    Ok((settlement, Vec::new()))
}

/// Re-execute the prefix from its initial timed state, every clock at 0.
/// `None` when the timed subject cannot decide a step.
fn replay_prefix(
    timed: &TimedSystem<'_, '_>,
    payload: &TemporalCounterexample,
) -> Result<Option<Vec<TimedPosition>>, ReplayRefusal> {
    let initial = timed.initial();
    let count = initial.len();
    let state = initial
        .into_iter()
        .nth(payload.initial_state)
        .ok_or_else(|| {
            refused(ModelTraceRefusal::InitialState {
                index: payload.initial_state,
                count,
            })
        })?;
    let digest = timed_digest(timed, &state)?;
    let mut positions = vec![TimedPosition {
        state,
        stamp: zero(),
        digest,
    }];
    for (index, step) in payload.prefix.iter().enumerate() {
        let current = positions.last().ok_or_else(|| fault("a-position-exists"))?;
        let delay = step
            .delay
            .as_ref()
            .ok_or_else(|| refused(ModelTraceRefusal::MissingDelay { step: Some(index) }))?;
        let Some(delayed) = delay_by(timed, &current.state, delay, Some(index))? else {
            return Ok(None);
        };
        let Ok(guarded) = timed.guarded_steps(delayed.discrete()) else {
            return Ok(None);
        };
        let mut enabled = false;
        let mut reached = None;
        let mut digests = Vec::new();
        for candidate in guarded
            .iter()
            .filter(|candidate| candidate.transition() == &step.transition)
        {
            if !candidate
                .parts()
                .iter()
                .any(|part| part.iter().all(|c| c.holds(delayed.clocks())))
            {
                continue;
            }
            enabled = true;
            let next = TimedState::new(candidate.post().clone(), candidate.reset(delayed.clocks()));
            let digest = timed_digest(timed, &next)?;
            if digest == step.post_state {
                reached = Some((candidate, next, digest));
                break;
            }
            digests.push(digest);
        }
        if !enabled {
            return Err(refused(ModelTraceRefusal::NotEnabled {
                step: index,
                transition: Box::new(step.transition.clone()),
            }));
        }
        let (candidate, next, digest) = reached.ok_or_else(|| {
            refused(ModelTraceRefusal::PostState {
                step: index,
                recorded: step.post_state,
                reached: digests,
            })
        })?;
        let Ok(invariants) = timed.invariants(next.discrete()) else {
            return Ok(None);
        };
        if !invariants.iter().all(|bound| bound.holds(next.clocks())) {
            return Err(refused(ModelTraceRefusal::TargetInvariant { step: index }));
        }
        let stamp = current
            .stamp
            .add(delay)
            .add(&timed.tick_of(candidate.operation()));
        positions.push(TimedPosition {
            state: next,
            stamp,
            digest,
        });
    }
    Ok(Some(positions))
}

fn timed_digest(
    timed: &TimedSystem<'_, '_>,
    state: &TimedState,
) -> Result<qsl_foundation::digest::DigestRecord, ReplayRefusal> {
    timed
        .state_digest(state)
        .map_err(|_| fault("replayed-timed-state-encodes"))
}

/// The state a delay by `delay` reaches from `state`, refusing one that is
/// not admissible there; `None` when the subject cannot decide it.
fn delay_by(
    timed: &TimedSystem<'_, '_>,
    state: &TimedState,
    delay: &Rational,
    step: Option<usize>,
) -> Result<Option<TimedState>, ReplayRefusal> {
    match timed.delay(state, delay) {
        Ok(Some(delayed)) => Ok(Some(delayed)),
        Ok(None) => {
            let bar = if delay < &zero() {
                "negative"
            } else if timed.tick_period().is_some() {
                "tick-source"
            } else if timed.is_urgent(state.discrete()).unwrap_or(false) {
                "urgent"
            } else {
                "time-invariant"
            };
            Err(refused(ModelTraceRefusal::DelayNotAdmissible {
                step,
                delay: delay.clone(),
                bar,
            }))
        }
        Err(_) => Ok(None),
    }
}

/// FR-237 "Lasso": a loop of steps ends at its entry's timed state, clocks
/// included, after a positive total delay; a terminal-stutter loop stands
/// where the idle tail lets time diverge (FR-231): a quiescent state, or
/// under a tick source a terminal one.
fn close_loop(
    timed: &TimedSystem<'_, '_>,
    positions: &[TimedPosition],
    lasso: Lasso,
) -> Result<(), ReplayRefusal> {
    let steps = positions.len() - 1;
    let open = || {
        refused(ModelTraceRefusal::LoopOpen {
            entry: lasso.entry,
            steps,
        })
    };
    let last = positions.last().ok_or_else(|| fault("a-position-exists"))?;
    if lasso.terminal_stutter {
        if lasso.entry != steps {
            return Err(open());
        }
        // Under a tick source the idle tail is a terminal state's stutter,
        // which ticks; under a dense source, a quiescent state's.
        let idles = match timed.tick_period() {
            Some(_) => timed.steps(&last.state).map(|steps| steps.is_empty()),
            None => timed.is_quiescent(last.state.discrete()),
        };
        return match idles {
            Ok(true) => Ok(()),
            Ok(false) | Err(_) => Err(refused(ModelTraceRefusal::StutterNotTerminal {
                position: steps,
            })),
        };
    }
    let entry = positions.get(lasso.entry).ok_or_else(open)?;
    if lasso.entry >= steps || entry.digest != last.digest {
        return Err(open());
    }
    if last.stamp <= entry.stamp {
        return Err(refused(ModelTraceRefusal::LoopZeroDelay {
            entry: lasso.entry,
        }));
    }
    Ok(())
}

/// The fairness of a timed lasso.
enum Fairness {
    Fair,
    Unfair(FairnessConstraint),
    NoValue,
}

/// FR-232 "Fairness over time": the first constraint of `fairness` the
/// lasso's loop is unfair under. A weak constraint fails when, from some
/// repetition on, it stays enabled at every instant of the loop while the
/// loop takes none of its steps; a strong one, when it is enabled at some
/// instant of the loop while the loop takes none of its steps. An `each`
/// constraint is one per transition identity of its operation.
fn unfair(
    timed: &TimedSystem<'_, '_>,
    payload: &TemporalCounterexample,
    lasso: Lasso,
    fairness: &[FairnessConstraint],
) -> Result<Fairness, ReplayRefusal> {
    if fairness.is_empty() {
        return Ok(Fairness::Fair);
    }
    let Some(initial) = timed.initial().into_iter().nth(payload.initial_state) else {
        return Err(fault("a-replayed-initial-state"));
    };
    let mut behaviour = TimedBehaviour::new(initial);
    for step in &payload.prefix {
        let delay = step.delay.clone().unwrap_or_else(zero);
        if behaviour.advance(timed, &delay, &step.transition).is_err() {
            return Ok(Fairness::NoValue);
        }
    }
    let system = timed.system();
    let stutter_delay = timed
        .tick_period()
        .cloned()
        .unwrap_or_else(|| Rational::from_integer(Integer::one()));
    let positions = behaviour.positions();
    // The loop's taken steps and its enabled guarded steps.
    let looped = &positions[lasso.entry.min(positions.len() - 1)..];
    let taken: Vec<&GuardedStep> = looped.iter().skip(1).filter_map(|p| p.step()).collect();
    for constraint in fairness {
        let operation = constraint.operation();
        let text = system
            .operation_text(operation.declaring_type(), operation.name())
            .ok_or_else(|| fault("a-recompiled-fairness-names-a-declared-operation"))?;
        let mut identities = Vec::new();
        match constraint.granularity() {
            FairnessGranularity::Whole => identities.push(None),
            FairnessGranularity::Each => {
                for position in looped {
                    let Ok(steps) = timed.guarded_steps(position.state().discrete()) else {
                        return Ok(Fairness::NoValue);
                    };
                    for step in steps {
                        if step.operation() == text
                            && !identities.contains(&Some(step.transition().clone()))
                        {
                            identities.push(Some(step.transition().clone()));
                        }
                    }
                }
            }
        }
        for identity in identities {
            let constrained = |step: &GuardedStep| {
                step.operation() == text
                    && identity.as_ref().is_none_or(|id| step.transition() == id)
            };
            if taken.iter().any(|step| constrained(step)) {
                continue;
            }
            let fails = match constraint.kind() {
                FairnessKind::Weak => {
                    match timed.weakly_fair(&behaviour, lasso.entry, &stutter_delay, constrained) {
                        Ok(fair) => !fair,
                        Err(_) => return Ok(Fairness::NoValue),
                    }
                }
                FairnessKind::Strong => {
                    match enabled_somewhere(timed, looped, lasso, &stutter_delay, &constrained) {
                        Some(enabled) => enabled,
                        None => return Ok(Fairness::NoValue),
                    }
                }
            };
            if fails {
                return Ok(Fairness::Unfair(constraint.clone()));
            }
        }
    }
    Ok(Fairness::Fair)
}

/// Whether a step `constrained` accepts is enabled at some instant of the
/// loop: at a loop position, or during the delay before the next one;
/// `None` when the subject cannot decide it.
fn enabled_somewhere(
    timed: &TimedSystem<'_, '_>,
    looped: &[qsl_eval::simulation::model::TimedPosition],
    lasso: Lasso,
    stutter_delay: &Rational,
    constrained: &dyn Fn(&GuardedStep) -> bool,
) -> Option<bool> {
    let states: &[qsl_eval::simulation::model::TimedPosition] = if lasso.terminal_stutter {
        looped
    } else {
        // The loop's last position is its entry again.
        &looped[..looped.len().saturating_sub(1)]
    };
    for (index, position) in states.iter().enumerate() {
        let delay = match looped.get(index + 1) {
            Some(next) if !lasso.terminal_stutter => next.stamp().sub(position.stamp()),
            _ => stutter_delay.clone(),
        };
        for step in timed.guarded_steps(position.state().discrete()).ok()? {
            if !constrained(&step) {
                continue;
            }
            let intervals = timed.enabled_delays(position.state(), &step).ok()?;
            if intervals.iter().any(|interval| {
                let (lower, _) = interval.lower();
                lower <= &delay
            }) {
                return Some(true);
            }
        }
    }
    Some(false)
}

/// The timed trace of the replayed positions: a lasso's loop re-enters its
/// entry after the loop's total delay, an idle tail repeats the last
/// position after one unit (or a tick), and a finite prefix ends.
fn trace_of(
    timed: &TimedSystem<'_, '_>,
    payload: &TemporalCounterexample,
    replayed: &[TimedPosition],
) -> Result<(TimedTrace, usize), TemporalRefusal> {
    let stamps = |positions: &[TimedPosition]| -> Vec<Rational> {
        positions.iter().map(|p| p.stamp.clone()).collect()
    };
    match payload.lasso {
        None => Ok((TimedTrace::new(stamps(replayed))?, replayed.len())),
        Some(lasso) if lasso.terminal_stutter => {
            let delay = timed
                .tick_period()
                .cloned()
                .unwrap_or_else(|| Rational::from_integer(Integer::one()));
            let last = position_of(replayed.len() - 1);
            Ok((
                TimedTrace::lasso(stamps(replayed), last, delay)?,
                replayed.len(),
            ))
        }
        Some(lasso) => {
            // The last position is the entry's again.
            let represented = replayed.len() - 1;
            let delay = replayed[represented]
                .stamp
                .sub(&replayed[lasso.entry].stamp);
            Ok((
                TimedTrace::lasso(
                    stamps(&replayed[..represented]),
                    position_of(lasso.entry),
                    delay,
                )?,
                represented,
            ))
        }
    }
}

/// FR-237 "Evaluation" under the timed profile: the formula over the timed
/// lasso (FR-231), `false` at the origin reproducing.
fn timed_formula(
    timed: &TimedSystem<'_, '_>,
    payload: &TemporalCounterexample,
    positions: &[Position],
    replayed: &[TimedPosition],
    formula: &FormulaReplay<'_>,
) -> Result<ModelTraceSettlement, ReplayRefusal> {
    let Ok((trace, represented)) = trace_of(timed, payload, replayed) else {
        return Ok(no_value());
    };
    let mut letters = replayed_letters(
        timed.system(),
        payload,
        &positions[..represented],
        formula.letters,
    )?;
    let mut meter = Meter::new(formula.meter);
    Ok(
        match evaluate_timed_at(
            formula.clause,
            &trace,
            TemporalPosition::new(0),
            &mut letters,
            &mut meter,
        ) {
            Ok(TemporalOutcome::Violated { .. }) => ModelTraceSettlement::Reproduced {
                value: ModelTraceValue::Truth(false),
                trace_position: payload.trace_position(),
            },
            Ok(TemporalOutcome::Holds) => disagrees(Category::Success),
            Ok(_) => no_value(),
            Err(TemporalRefusal::Fault(_)) => return Err(fault("temporal-evaluation")),
            Err(_) => no_value(),
        },
    )
}

/// ADR-018 UE-5 under the timed profile: the first undefined reading of
/// the clause's letters along the timed trace.
fn timed_undefined_letter(
    system: &ModelSystem<'_>,
    payload: &TemporalCounterexample,
    positions: &[Position],
    replayed: &[TimedPosition],
    formula: &FormulaReplay<'_>,
) -> Result<LetterRead, ReplayRefusal> {
    let stamps: Vec<Rational> = replayed.iter().map(|p| p.stamp.clone()).collect();
    let Ok(trace) = TimedTrace::new(stamps) else {
        return Ok(LetterRead::NoValue);
    };
    let mut letters = replayed_letters(system, payload, positions, formula.letters)?;
    read_first_undefined(&trace, formula, &mut letters)
}

fn read_first_undefined<S: qsl_eval::simulation::SubjectSystem>(
    trace: &TimedTrace,
    formula: &FormulaReplay<'_>,
    letters: &mut super::ReplayedLetters<'_, S>,
) -> Result<LetterRead, ReplayRefusal> {
    let mut meter = Meter::new(formula.meter);
    match evaluate_timed_at(
        formula.clause,
        trace,
        TemporalPosition::new(0),
        letters,
        &mut meter,
    ) {
        Ok(TemporalOutcome::Undefined(undefined)) => {
            let Some((position, locus)) = letters.undefined.take() else {
                return Err(fault("an-undefined-letter-names-its-locus"));
            };
            if position != undefined.at.index() {
                return Err(fault("the-first-undefined-letter-is-the-evaluators"));
            }
            let position = usize::try_from(position)
                .map_err(|_| fault("an-undefined-position-is-a-position"))?;
            Ok(LetterRead::Read(Some(UndefinedLetter {
                position,
                locus,
                cause: undefined.cause,
            })))
        }
        Err(TemporalRefusal::Fault(_)) => Err(fault("temporal-evaluation")),
        Ok(TemporalOutcome::Incomplete(_) | TemporalOutcome::WindowLimitReached(_)) | Err(_) => {
            Ok(LetterRead::NoValue)
        }
        Ok(_) => Ok(LetterRead::Read(None)),
    }
}

/// FR-237 "Undefined evaluation": FR-128's comparison of the first
/// undefined evaluation over the model states, and the claimed position's
/// time stamp and clock valuation against the replayed ones.
fn timed_undefined(
    timed: &TimedSystem<'_, '_>,
    payload: &TemporalCounterexample,
    positions: &[Position],
    replayed: &[TimedPosition],
    claimed: &UndefinedEvaluation,
    reads: &ItemReads,
) -> Result<ModelTraceSettlement, ReplayRefusal> {
    let system = timed.system();
    let mut untimed = claimed.clone();
    untimed.site.time = None;
    let settlement = undefined(system, system, payload, positions, &untimed, reads)?;
    let ModelTraceSettlement::Reproduced {
        value: ModelTraceValue::Undefined(found),
        trace_position,
    } = settlement
    else {
        return Ok(settlement);
    };
    let at = usize::try_from(found.site.position)
        .ok()
        .and_then(|at| replayed.get(at))
        .ok_or_else(|| fault("an-undefined-position-is-replayed"))?;
    let present: Vec<Rational> = timed
        .places()
        .iter()
        .zip(at.state.clocks().values())
        .filter(|(place, _)| place.present(at.state.discrete()))
        .map(|(_, value)| value.clone())
        .collect();
    let time = Some(Box::new(TimedPoint {
        stamp: at.stamp.clone(),
        clocks: present,
    }));
    if claimed.site.time != time {
        return Ok(ModelTraceSettlement::Inconclusive(disagreement(
            Category::Violation,
        )));
    }
    Ok(ModelTraceSettlement::Reproduced {
        value: ModelTraceValue::Undefined(claimed.clone()),
        trace_position,
    })
}

/// FR-237 "Arm selection" for `kind: TimeLock`: after the prefix and its
/// `final_delay`, a state that admits no positive delay and enables no
/// transition identity is a local time-lock and reproduces; any other
/// state takes the non-local arm, whose fresh exploration reproduces when
/// it completes with no time-divergent continuation.
fn time_lock(
    timed: &TimedSystem<'_, '_>,
    payload: &TemporalCounterexample,
    last: &TimedPosition,
    exploration: &Limits,
) -> Result<ModelTraceSettlement, ReplayRefusal> {
    let delay = payload
        .final_delay
        .as_ref()
        .ok_or_else(|| refused(ModelTraceRefusal::MissingDelay { step: None }))?;
    let Some(locked) = delay_by(timed, &last.state, delay, None)? else {
        return Ok(no_value());
    };
    let reproduced = ModelTraceSettlement::Reproduced {
        value: ModelTraceValue::Truth(false),
        trace_position: payload.trace_position(),
    };
    let Ok(admissible) = timed.admissible_delays(&locked) else {
        return Ok(no_value());
    };
    let positive = admissible.upper().is_none_or(|(upper, _)| upper > &zero());
    let Ok(steps) = timed.steps(&locked) else {
        return Ok(no_value());
    };
    if !positive && steps.is_empty() && timed.tick_period().is_none() {
        return Ok(reproduced);
    }
    Ok(match explore(timed, &locked, exploration) {
        Ok(Closure::Locked) => reproduced,
        Ok(Closure::Divergent) => {
            ModelTraceSettlement::Inconclusive(disagreement(Category::Success))
        }
        Ok(Closure::Stopped { states, count }) => stopped(exploration, states, count),
        Err(_) => no_value(),
    })
}

/// FR-237 "Admitted endpoint": `settlement`, a reproduced finite violation,
/// stands only where the last replayed state has a time-divergent
/// continuation; one with none settles `inconclusive`, and a limit stops
/// the replay.
fn admitted(
    timed: &TimedSystem<'_, '_>,
    last: &TimedState,
    exploration: &Limits,
    settlement: ModelTraceSettlement,
) -> ModelTraceSettlement {
    match explore(timed, last, exploration) {
        Ok(Closure::Divergent) => settlement,
        Ok(Closure::Locked) => ModelTraceSettlement::Inconclusive(disagreement(Category::Success)),
        Ok(Closure::Stopped { states, count }) => stopped(exploration, states, count),
        Err(_) => no_value(),
    }
}

fn stopped(exploration: &Limits, states: bool, count: u64) -> ModelTraceSettlement {
    let widen = |value: usize| u64::try_from(value).unwrap_or(u64::MAX);
    ModelTraceSettlement::Stopped {
        reached: if states {
            ReachedLimit {
                limit: ModelCheckLimit::MaxStates,
                value: widen(exploration.max_states),
            }
        } else {
            ReachedLimit {
                limit: ModelCheckLimit::MaxTransitions,
                value: widen(exploration.max_transitions),
            }
        },
        count: Some(count),
    }
}
