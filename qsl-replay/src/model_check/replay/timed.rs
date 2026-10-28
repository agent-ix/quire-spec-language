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
    check_formula, digest_of, disagreement, disagrees, fault, hints_of, no_value, position_of,
    refused, replayed_letters, undefined, undefined_letter, DelayBar, FormulaReplay, HintState,
    ItemReads, LetterRead, ModelTraceRefusal, ModelTraceSettlement, ModelTraceValue, Position,
    StrongFairnessHint, UndefinedLetter,
};
use crate::execute::ReplayRefusal;
use crate::model_check::counterexample::{
    CounterexampleKind, Lasso, TemporalCounterexample, TimedPoint, UndefinedEvaluation,
};
use crate::model_check::limits::{ModelCheckLimit, ReachedLimit};
use crate::timed::explore::{explore, widen, Closure};

/// The timed subject of the recompiled system under the timed model of
/// `model`, the alias the subject names, bounded by `max_clocks`.
///
/// # Errors
///
/// [`ModelTraceRefusal::TimedSubject`] when the package has no such timed
/// model or it does not build a timed subject.
pub(super) fn timed_subject<'s, 'a>(
    system: &'s ModelSystem<'a>,
    model: &str,
    max_clocks: std::num::NonZeroU64,
) -> Result<TimedSystem<'s, 'a>, ReplayRefusal> {
    TimedSystem::with_max_clocks(system, model, max_clocks)
        .map_err(|refusal| refused(ModelTraceRefusal::TimedSubject(refusal)))
}

/// One replayed timed position: the timed state, its time stamp, its
/// FR-236 digest and the step that reached it.
struct TimedPosition {
    state: TimedState,
    stamp: Rational,
    /// The timed state's FR-236 digest, clocks included.
    digest: qsl_foundation::digest::DigestRecord,
    /// The step that reached it, the one whose post-state digest is the
    /// recorded one; `None` at position 0.
    step: Option<GuardedStep>,
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
    // FR-236: `final_delay` belongs to a time-lock alone, and a time-lock is
    // a finite prefix.
    match (&payload.kind, &payload.final_delay, payload.lasso) {
        (CounterexampleKind::TimeLock, _, Some(_)) => {
            return Err(refused(ModelTraceRefusal::TimeLockLasso))
        }
        (CounterexampleKind::TimeLock, _, None) | (_, None, _) => {}
        (_, Some(_), _) => return Err(refused(ModelTraceRefusal::StrayFinalDelay)),
    }
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
    let mut behaviour = None;
    if let Some(lasso) = payload.lasso {
        close_loop(timed, &replayed, &positions, lasso)?;
        let Some(built) = behaviour_of(timed, &replayed) else {
            return Ok((no_value(), Vec::new()));
        };
        if let Some(formula) = &formula {
            match unfair(timed, &built, lasso, formula.clause.fairness())? {
                Fairness::Fair => {}
                Fairness::Unfair(constraint) => {
                    return Err(ReplayRefusal::UnfairLasso { constraint })
                }
                Fairness::NoValue => return Ok((no_value(), Vec::new())),
            }
        }
        behaviour = Some((built, lasso));
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
    // FR-133: a reproduced liveness lasso's strong-fairness hints, read
    // over time.
    let hints = match (&payload.kind, &settlement, behaviour) {
        (
            CounterexampleKind::Formula,
            ModelTraceSettlement::Reproduced { .. },
            Some((behaviour, lasso)),
        ) => timed_hints(timed, &behaviour, lasso).unwrap_or_default(),
        _ => Vec::new(),
    };
    Ok((settlement, hints))
}

/// The replayed positions as a timed behaviour, each step the recorded one;
/// `None` when the subject cannot decide a step.
fn behaviour_of(timed: &TimedSystem<'_, '_>, replayed: &[TimedPosition]) -> Option<TimedBehaviour> {
    let mut behaviour = TimedBehaviour::new(replayed.first()?.state.clone());
    for pair in replayed.windows(2) {
        let (before, after) = (&pair[0], &pair[1]);
        let step = after.step.as_ref()?;
        let delay = after
            .stamp
            .sub(&before.stamp)
            .sub(&timed.tick_of(step.operation()));
        behaviour
            .advance_where(timed, &delay, |candidate, state| {
                candidate.transition() == step.transition()
                    && timed
                        .state_digest(state)
                        .is_ok_and(|digest| digest == after.digest)
            })
            .ok()?;
    }
    Some(behaviour)
}

/// FR-133 over a timed loop: the identities enabled at some instant of
/// each window of the repetition every later one reads alike, against the
/// step that ends it. `None` when the subject cannot decide one.
fn timed_hints(
    timed: &TimedSystem<'_, '_>,
    behaviour: &TimedBehaviour,
    lasso: Lasso,
) -> Option<Vec<StrongFairnessHint>> {
    let stutter_delay = stutter_delay(timed);
    let enabled: Vec<Vec<quire_semantic_value::model_transition::ModelTransition>> = timed
        .loop_enabled(behaviour, lasso.entry, &stutter_delay)
        .ok()?
        .into_iter()
        .map(|steps| steps.iter().map(|step| step.transition().clone()).collect())
        .collect();
    let positions = behaviour.positions();
    let states: Vec<HintState<'_>> = enabled
        .iter()
        .enumerate()
        .map(|(position, enabled)| HintState {
            position,
            enabled,
            taken: positions
                .get(lasso.entry + position + 1)
                .and_then(|next| next.step())
                .map(GuardedStep::transition),
        })
        .collect();
    Some(hints_of(timed.system(), &states))
}

/// The delay of one idle-tail stutter: one unit, or the tick period.
fn stutter_delay(timed: &TimedSystem<'_, '_>) -> Rational {
    timed
        .tick_period()
        .cloned()
        .unwrap_or_else(|| Rational::from_integer(Integer::one()))
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
        step: None,
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
        let mut named = Vec::new();
        let mut enabled = false;
        let mut reached = None;
        let mut digests = Vec::new();
        for candidate in guarded
            .iter()
            .filter(|candidate| candidate.transition() == &step.transition)
        {
            named.push(candidate);
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
            // Enabled by no data precondition, or by one whose clock guard
            // fails after the delay.
            return Err(refused(match named.first() {
                None => ModelTraceRefusal::NotEnabled {
                    step: index,
                    transition: Box::new(step.transition.clone()),
                },
                Some(candidate) => ModelTraceRefusal::GuardFails {
                    step: index,
                    transition: Box::new(step.transition.clone()),
                    guards: candidate.guards().to_vec(),
                },
            }));
        }
        let (candidate, next, digest) = reached.ok_or_else(|| {
            refused(ModelTraceRefusal::PostState {
                step: index,
                recorded: step.post_state,
                reached: digests,
            })
        })?;
        match timed.broken_invariant(&next) {
            Ok(None) => {}
            Ok(Some(invariant)) => {
                return Err(refused(ModelTraceRefusal::TargetInvariant {
                    step: index,
                    invariant,
                }))
            }
            Err(_) => return Ok(None),
        }
        let stamp = current
            .stamp
            .add(delay)
            .add(&timed.tick_of(candidate.operation()));
        positions.push(TimedPosition {
            state: next,
            stamp,
            digest,
            step: Some(candidate.clone()),
        });
    }
    Ok(Some(positions))
}

/// Each replayed position's discrete state with its digest, and its timed
/// state.
pub(crate) type TimedPrefix = (Vec<Position>, Vec<TimedState>);

/// QSpec FR-430: `payload`'s prefix re-executed over time by FR-237's rules
/// (each delay admissible, each step enabled after it, each post-state
/// digest the recorded one, each target invariant holding), a lasso's loop
/// closing with a positive total delay: every
/// position's discrete state with its digest, and its timed state; `None`
/// when the subject cannot decide a step.
///
/// # Errors
///
/// As FR-237's prefix replay refuses.
pub(crate) fn timed_prefix(
    timed: &TimedSystem<'_, '_>,
    payload: &TemporalCounterexample,
) -> Result<Option<TimedPrefix>, ReplayRefusal> {
    let Some(replayed) = replay_prefix(timed, payload)? else {
        return Ok(None);
    };
    let system = timed.system();
    let mut positions = Vec::with_capacity(replayed.len());
    let mut states = Vec::with_capacity(replayed.len());
    for position in &replayed {
        let state = position.state.discrete().clone();
        let digest = digest_of(system, &state)?;
        positions.push(Position { state, digest });
        states.push(position.state.clone());
    }
    // A lasso's loop closes over time (FR-237 "Lasso").
    if let Some(lasso) = payload.lasso {
        close_loop(timed, &replayed, &positions, lasso)?;
    }
    Ok(Some((positions, states)))
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
                DelayBar::Negative
            } else if timed.tick_period().is_some() {
                DelayBar::TickSource
            } else {
                match timed.is_urgent(state.discrete()) {
                    Ok(true) => DelayBar::Urgent,
                    Ok(false) => {
                        // Invariants are upper bounds: the delay's end
                        // breaks one.
                        let end = TimedState::new(
                            state.discrete().clone(),
                            state.clocks().delayed(delay),
                        );
                        match timed.broken_invariant(&end) {
                            Ok(Some(invariant)) => DelayBar::TimeInvariant(invariant),
                            Ok(None) => return Err(fault("an-inadmissible-delay-has-a-bar")),
                            Err(_) => return Ok(None),
                        }
                    }
                    Err(_) => return Ok(None),
                }
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

/// FR-237 "Lasso" over FR-236's timed lasso: a loop of steps ends at its
/// entry's discrete state after a positive total delay, each clock the
/// loop resets at its entry value and each clock it never resets above
/// every constant of the model at entry, so it runs free; a
/// terminal-stutter loop stands where the idle tail lets time diverge
/// (FR-231): a quiescent state, or under a tick source a terminal one.
/// `discrete` holds each position's discrete digest.
fn close_loop(
    timed: &TimedSystem<'_, '_>,
    positions: &[TimedPosition],
    discrete: &[Position],
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
    let same_discrete = discrete
        .get(lasso.entry)
        .zip(discrete.last())
        .is_some_and(|(entry, last)| entry.digest == last.digest);
    if lasso.entry >= steps || !same_discrete {
        return Err(open());
    }
    if last.stamp <= entry.stamp {
        return Err(refused(ModelTraceRefusal::LoopZeroDelay {
            entry: lasso.entry,
        }));
    }
    let ceiling = timed.max_constant();
    let looped = &positions[lasso.entry + 1..];
    let entry_clocks = entry.state.clocks().values();
    let last_clocks = last.state.clocks().values();
    for (place, (at_entry, at_end)) in entry_clocks.iter().zip(last_clocks).enumerate() {
        let reset = looped.iter().any(|position| {
            position.step.as_ref().is_some_and(|step| {
                // A carried place takes another place's value: the step
                // sets it, as a reset does.
                step.resets().iter().any(|(reset, _)| *reset == place)
                    || step.carries().iter().any(|(carried, _)| *carried == place)
            })
        });
        let closes = if reset {
            at_entry == at_end
        } else {
            at_entry > &ceiling
        };
        if !closes {
            return Err(open());
        }
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
/// lasso's loop is unfair under, read over the replayed behaviour (each step
/// the recorded successor). A weak constraint fails when, from some
/// repetition on, it stays enabled at every instant of the loop while the
/// loop takes none of its steps; a strong one, when it is enabled at some
/// instant of infinitely many repetitions while the loop takes none of its
/// steps. An `each` constraint is one per transition identity of its
/// operation.
fn unfair(
    timed: &TimedSystem<'_, '_>,
    behaviour: &TimedBehaviour,
    lasso: Lasso,
    fairness: &[FairnessConstraint],
) -> Result<Fairness, ReplayRefusal> {
    if fairness.is_empty() {
        return Ok(Fairness::Fair);
    }
    let system = timed.system();
    let stutter_delay = stutter_delay(timed);
    let positions = behaviour.positions();
    let looped = &positions[lasso.entry.min(positions.len() - 1)..];
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
            let fair = match constraint.kind() {
                FairnessKind::Weak => {
                    timed.weakly_fair(behaviour, lasso.entry, &stutter_delay, constrained)
                }
                FairnessKind::Strong => {
                    timed.strongly_fair(behaviour, lasso.entry, &stutter_delay, constrained)
                }
            };
            match fair {
                Ok(true) => {}
                Ok(false) => return Ok(Fairness::Unfair(constraint.clone())),
                Err(_) => return Ok(Fairness::NoValue),
            }
        }
    }
    Ok(Fairness::Fair)
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
            let delay = stutter_delay(timed);
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
