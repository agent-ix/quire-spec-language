// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-183 (ADR-023 HX-2, HX-5): [`replay_hyper_trace_tuple`], the replay of a
//! hyper counterexample's `Lockstep`, `WitnessExhausted` and `Undefined` arms
//! through each trace variable's `ModelSystem`.
//!
//! It re-executes every trace from its initial snapshot, a step's post-state
//! digest selecting its successor among the post-states of its transition
//! identity, and a terminal-stutter step standing only at a state that
//! enables no transition identity. It needs no model checker and no solver.
//! What it checks of the traces then depends on the arm:
//!
//! - `Lockstep` (HP-2): the traces have equal step counts and one loop entry
//!   that every trace's last state closes; `μ_U` holds at every joint step,
//!   the loop's closing step included; each trace is fair under its own
//!   fairness set on its own loop. It then evaluates the body over the tuple
//!   of lassos by FR-175: `false` settles `Reproduced` at the position that
//!   fixes it, anything else `Inconclusive`.
//! - `WitnessExhausted` (HP-3): its traces are the universal variables' alone.
//!   They are checked as a `Lockstep` lasso is, with no body evaluation. The
//!   replay then recomputes the witness sets `X_0` to `X_position` along them
//!   with `qsl-eval`'s witness product, the code the engine runs, under
//!   `max_witness_set`: an empty `X_position` after non-empty earlier sets
//!   reproduces, any other sets are `Inconclusive`, and a set that would pass
//!   the budget stops the replay at it.
//! - `Undefined`: the traces are paths to the joint position the payload
//!   names. The replay reads `μ_U`, `μ_E`, the body's atoms and each component's
//!   contract conjunctions at the joint positions up to it, in the order EN-1
//!   reads them, and settles `Reproduced` exactly when the first expression
//!   with no value is the payload's, at its position, with its cause.
//!
//! The facade entry `replay_model_trace_tuple` recompiles the package and
//! resolves the clause, which the S4 emission of a hyper clause will supply;
//! this function takes the recompiled clause and the admitted systems it
//! would hand over. A counterexample that breaks QSpec FR-400's reader rules
//! is refused here as the shape it breaks. The other arms (`projected`,
//! `step-tuple`) join with their replays: `projected` is refused as not yet
//! implemented.

use qsl_eval::automaton::translate_negation;
use qsl_eval::hyper::{
    ComponentStep, Halt, HyperLetters, HyperState, HyperTransition, MatchReading, ParameterBinding,
    Property, StepLabel, StepMatch, WitnessInputs, WitnessProduct,
};
use qsl_eval::simulation::model::{ModelSimState, ModelSystem};
use qsl_eval::simulation::TransitionSystem;
use qsl_eval::temporal::{
    evaluate_tuple, AtomReading, ComponentPosition, ComponentTrace, LetterStop, TemporalOutcome,
    TemporalPosition, TupleLetters, UndefinedCause,
};
use qsl_foundation::diagnostic::Category;
use qsl_foundation::digest::DigestRecord;
use qsl_semantics::check::{CheckedHyperClause, FormulaNodeId, HyperForm};
use quire_exact::{Cancel, Meter, ScalarLimits};
use quire_semantic_value::model_transition::ModelTransition;

use crate::execute::ReplayRefusal;
use crate::model_check::counterexample::{Lasso, ModelStep};
use crate::model_check::hyper_counterexample::{
    HyperCounterexample, HyperCounterexampleKind, HyperLocus, HyperStep, HyperTrace, HyperUndefined,
};
use crate::model_check::limits::{ModelCheckLimit, ReachedLimit};
use crate::model_check::replay::{
    digest_of, disagreement, expand, fault, no_value_cause, refused, unfair_constraint, LoopState,
    ModelTraceRefusal, ModelTraceSettlement, ModelTraceValue, Position,
};

/// What replaying a hyper counterexample reads beside the traces: the
/// recompiled clause, each trace variable's admitted system, the instance the
/// counterexample binds, the seams to the clause's atoms and `match`, and the
/// evaluation budget.
pub struct HyperReplay<'a, 's> {
    /// The recompiled clause.
    pub clause: &'a CheckedHyperClause,
    /// The `ModelSystem` of each trace variable's subject, in quantifier
    /// order.
    pub systems: Vec<&'a ModelSystem<'s>>,
    /// The clause instance: each object parameter and its key, in declaration
    /// order.
    pub instance: &'a [ParameterBinding],
    /// The seam to the body's `holds` atoms.
    pub letters: &'a dyn HyperLetters,
    /// The seam to `μ_U`.
    pub matcher: &'a dyn StepMatch,
    /// The budget of each atom or `μ_U` evaluation.
    pub meter: ScalarLimits,
    /// The most members one witness set may reach (FR-184) when a
    /// `WitnessExhausted` counterexample recomputes its sets.
    pub max_witness_set: u64,
    /// The replay's cancellation handle.
    pub cancel: &'a Cancel,
}

/// Replay `counterexample` over `replay`'s systems (FR-183).
///
/// Refuses, with no partial result, a counterexample whose traces do not
/// match the clause's variables, whose step counts differ or whose loop entry
/// is not a step (`HyperShape`); an `initial` outside its subject, a step
/// that is not enabled, a post-state digest no successor has, a loop that does
/// not close and a stutter step at a state that is not terminal, as FR-128
/// does; a counterexample of an `align skip` clause, `HyperProjected`; a joint step at which `μ_U` is false (`HyperUnmatched`); and a trace
/// that is unfair on its own loop (`HyperUnfair`). An expansion that stops, a
/// meter that runs out or a cancellation completes no value; a witness set
/// that would pass `max_witness_set` stops the replay at that limit.
///
/// # Errors
///
/// [`ReplayRefusal`] as above.
pub fn replay_hyper_trace_tuple(
    replay: &HyperReplay<'_, '_>,
    counterexample: &HyperCounterexample,
) -> Result<ModelTraceSettlement, ReplayRefusal> {
    if !counterexample.marks_match_kind() {
        return Err(shape_error(
            "marks its steps other than as its kind does: every step of a projected one and none of any other",
        ));
    }
    match counterexample.kind {
        HyperCounterexampleKind::Projected => {
            return Err(refused(ModelTraceRefusal::HyperProjected));
        }
        HyperCounterexampleKind::Lockstep { .. }
        | HyperCounterexampleKind::WitnessExhausted { .. }
        | HyperCounterexampleKind::Undefined(_) => {}
    }
    // A clause with `align skip` is read over projected prefixes, so even an
    // `Undefined` counterexample of one has paths of unequal length and
    // positions that count visible steps: FR-183 does not state their replay.
    if !replay.clause.align_skip().is_empty() {
        return Err(refused(ModelTraceRefusal::HyperProjected));
    }
    shape(replay, counterexample)?;
    let Some(executed) = execute(replay, counterexample)? else {
        return Ok(no_value());
    };
    match &counterexample.kind {
        HyperCounterexampleKind::Lockstep { loop_entry } => {
            lockstep(replay, counterexample, &executed, *loop_entry)
        }
        HyperCounterexampleKind::WitnessExhausted {
            loop_entry,
            position,
        } => witness_exhausted(replay, counterexample, &executed, *loop_entry, *position),
        HyperCounterexampleKind::Undefined(claimed) => {
            undefined(replay, counterexample, &executed, claimed)
        }
        HyperCounterexampleKind::Projected => {
            Err(fault("an-unreplayed-counterexample-is-refused-first"))
        }
    }
}

fn no_value() -> ModelTraceSettlement {
    ModelTraceSettlement::Inconclusive(no_value_cause())
}

fn shape_error(reason: &'static str) -> ReplayRefusal {
    refused(ModelTraceRefusal::HyperShape { reason })
}

/// The counterexample must be about this clause: one trace per variable in
/// quantifier order (the universal variables alone for a `WitnessExhausted`
/// one), each with its variable and alias, equal step counts, a loop entry
/// that is a step of a lasso counterexample, the instance of the request, and
/// for an `Undefined` one the paths' length as its position.
fn shape(
    replay: &HyperReplay<'_, '_>,
    counterexample: &HyperCounterexample,
) -> Result<(), ReplayRefusal> {
    let quantifiers = replay.clause.quantifiers();
    let traced = match counterexample.kind {
        HyperCounterexampleKind::WitnessExhausted { .. } => replay.clause.universals(),
        _ => quantifiers.len(),
    };
    if counterexample.traces.len() != traced || replay.systems.len() != quantifiers.len() {
        return Err(shape_error("does not hold one trace per trace variable"));
    }
    for (trace, quantifier) in counterexample.traces.iter().zip(quantifiers) {
        if trace.variable != quantifier.variable || trace.alias != quantifier.alias.as_str() {
            return Err(shape_error(
                "names a trace variable or alias the clause does not",
            ));
        }
    }
    let steps = counterexample.steps();
    if counterexample
        .traces
        .iter()
        .any(|trace| trace.steps.len() != steps)
    {
        return Err(shape_error("holds traces of unequal step counts"));
    }
    let bound: Vec<(String, String)> = replay
        .instance
        .iter()
        .map(|binding| (binding.parameter.clone(), binding.instance.key.clone()))
        .collect();
    if counterexample.instance != bound {
        return Err(shape_error("names an instance other than the request's"));
    }
    match &counterexample.kind {
        HyperCounterexampleKind::Lockstep { loop_entry } => {
            if usize::try_from(*loop_entry).map_or(true, |entry| entry >= steps) {
                return Err(shape_error("re-enters its loop outside its steps"));
            }
        }
        HyperCounterexampleKind::WitnessExhausted {
            loop_entry,
            position,
        } => {
            if replay.clause.form() != Some(&HyperForm::ForallExistsSafety) {
                return Err(shape_error(
                    "is of a clause that is not a forall-exists safety hyperproperty",
                ));
            }
            if usize::try_from(*loop_entry).map_or(true, |entry| entry >= steps) {
                return Err(shape_error("re-enters its loop outside its steps"));
            }
            if *position != counterexample.trace_position
                || usize::try_from(*position).map_or(true, |at| at > steps)
            {
                return Err(shape_error(
                    "names a joint position other than its trace position, or past its lasso",
                ));
            }
        }
        HyperCounterexampleKind::Projected => {
            return Err(fault("an-unreplayed-counterexample-is-refused-first"));
        }
        HyperCounterexampleKind::Undefined(claimed) => {
            let position = u64::try_from(steps).unwrap_or(u64::MAX);
            if claimed.position != position || counterexample.trace_position != position {
                return Err(shape_error(
                    "does not end its paths at the joint position it names",
                ));
            }
            if claimed.state_keys.len() != claimed.variables.len() {
                return Err(shape_error("names a state for other than each variable"));
            }
        }
    }
    Ok(())
}

/// Every trace re-executed: one position per state, for each trace variable.
/// `None` when an expansion stops.
fn execute(
    replay: &HyperReplay<'_, '_>,
    counterexample: &HyperCounterexample,
) -> Result<Option<Vec<Vec<Position>>>, ReplayRefusal> {
    let mut executed = Vec::with_capacity(counterexample.traces.len());
    for (trace, system) in counterexample.traces.iter().zip(&replay.systems) {
        match reexecute(system, trace)? {
            Some(positions) => executed.push(positions),
            None => return Ok(None),
        }
    }
    Ok(Some(executed))
}

/// Re-execute `trace` from its initial state (FR-128's rules).
fn reexecute(
    system: &ModelSystem<'_>,
    trace: &HyperTrace,
) -> Result<Option<Vec<Position>>, ReplayRefusal> {
    let initial = system.initial();
    let count = initial.len();
    let index = usize::try_from(trace.initial).unwrap_or(usize::MAX);
    let state = initial
        .into_iter()
        .nth(index)
        .ok_or_else(|| refused(ModelTraceRefusal::InitialState { index, count }))?;
    let digest = digest_of(system, &state)?;
    let mut positions = vec![Position { state, digest }];
    for (index, step) in trace.steps.iter().enumerate() {
        let current = positions.last().ok_or_else(|| fault("a-position-exists"))?;
        let Ok(expansion) = expand(system, &current.state)? else {
            return Ok(None);
        };
        let next = match step {
            HyperStep::TerminalStutter => {
                if !expansion.successors.is_empty() {
                    return Err(refused(ModelTraceRefusal::StutterNotTerminal {
                        position: index,
                    }));
                }
                Position {
                    state: current.state.clone(),
                    digest: current.digest,
                }
            }
            HyperStep::Model(step) => {
                let mut enabled = false;
                let mut reached = None;
                let mut digests = Vec::new();
                for (transition, next) in expansion.successors {
                    if transition != step.transition {
                        continue;
                    }
                    enabled = true;
                    let digest = digest_of(system, &next)?;
                    digests.push(digest);
                    if digest == step.post_state {
                        reached = Some(Position {
                            state: next,
                            digest,
                        });
                        break;
                    }
                }
                if !enabled {
                    return Err(refused(ModelTraceRefusal::NotEnabled {
                        step: index,
                        transition: Box::new(step.transition.clone()),
                    }));
                }
                reached.ok_or_else(|| {
                    refused(ModelTraceRefusal::PostState {
                        step: index,
                        recorded: step.post_state,
                        reached: digests,
                    })
                })?
            }
        };
        positions.push(next);
    }
    Ok(Some(positions))
}

/// The step trace `component` took into joint position `joint` (`1`-based).
fn label<'a>(
    counterexample: &'a HyperCounterexample,
    component: usize,
    joint: usize,
) -> StepLabel<'a> {
    match &counterexample.traces[component].steps[joint - 1] {
        HyperStep::Model(step) => StepLabel::Operation(&step.transition),
        HyperStep::TerminalStutter => StepLabel::Stutter,
    }
}

/// `μ_U` over joint step `joint`, which reads the universal components'
/// steps alone.
fn matched(
    replay: &HyperReplay<'_, '_>,
    counterexample: &HyperCounterexample,
    joint: usize,
) -> Result<Option<MatchReading>, ReplayRefusal> {
    if replay.clause.mu_universal().is_empty() {
        return Ok(Some(MatchReading::Holds));
    }
    let labels: Vec<StepLabel<'_>> = (0..replay.clause.universals())
        .map(|component| label(counterexample, component, joint))
        .collect();
    let mut meter = Meter::new(replay.meter);
    Ok(replay
        .matcher
        .universal(replay.instance, &labels, &mut meter)
        .ok())
}

/// `μ_E` over joint step `joint`, which reads every component's step.
fn matched_existentially(
    replay: &HyperReplay<'_, '_>,
    counterexample: &HyperCounterexample,
    joint: usize,
) -> Option<MatchReading> {
    if replay.clause.mu_existential().is_empty() {
        return Some(MatchReading::Holds);
    }
    let labels: Vec<StepLabel<'_>> = (0..counterexample.traces.len())
        .map(|component| label(counterexample, component, joint))
        .collect();
    let mut meter = Meter::new(replay.meter);
    replay
        .matcher
        .existential(replay.instance, &labels, &mut meter)
        .ok()
}

/// The states of joint position `joint`, one per trace variable.
fn states(executed: &[Vec<Position>], joint: usize) -> Result<Vec<&ModelSimState>, ReplayRefusal> {
    executed
        .iter()
        .map(|positions| {
            positions
                .get(joint)
                .map(|position| &position.state)
                .ok_or_else(|| fault("a-joint-position-is-in-every-trace"))
        })
        .collect()
}

/// The body's atoms on the re-executed lasso: component `c` at joint position
/// `p` reads its own position's state (FR-175 "Indexed atoms").
struct Reexecuted<'a> {
    letters: &'a dyn HyperLetters,
    instance: &'a [ParameterBinding],
    executed: &'a [Vec<Position>],
}

impl TupleLetters for Reexecuted<'_> {
    fn atom(
        &mut self,
        atom: FormulaNodeId,
        _joint: TemporalPosition,
        components: &[ComponentPosition],
        meter: &mut Meter,
    ) -> Result<AtomReading, LetterStop> {
        let mut states: Vec<&ModelSimState> = Vec::with_capacity(components.len());
        for (component, read) in components.iter().enumerate() {
            let index = usize::try_from(read.position.index())
                .map_err(|_| LetterStop::Fault(fault_of("a-position-fits-usize")))?;
            let position = self
                .executed
                .get(component)
                .and_then(|positions| positions.get(index))
                .ok_or_else(|| LetterStop::Fault(fault_of("a-position-is-a-state")))?;
            states.push(&position.state);
        }
        self.letters
            .atom(atom, self.instance, &states, meter)
            .map_err(LetterStop::Incomplete)
    }
}

fn fault_of(invariant: &'static str) -> qsl_foundation::InternalFault {
    qsl_foundation::InternalFault::new("replay-hyper-trace-tuple", invariant)
}

/// The checks `Lockstep` and `WitnessExhausted` share (FR-183 "Common"): the
/// universal traces close at `loop_entry`, `μ_U` holds at every joint step, the
/// loop's closing step included, and each trace is fair on its own loop.
/// `None` when an evaluation or an expansion completes no value.
fn universal_lasso(
    replay: &HyperReplay<'_, '_>,
    counterexample: &HyperCounterexample,
    executed: &[Vec<Position>],
    loop_entry: u64,
) -> Result<Option<()>, ReplayRefusal> {
    let steps = counterexample.steps();
    let entry = usize::try_from(loop_entry).map_err(|_| fault("a-loop-entry-fits-usize"))?;
    for positions in executed {
        let (Some(at_entry), Some(last)) = (positions.get(entry), positions.last()) else {
            return Err(fault("a-trace-holds-its-loop-entry"));
        };
        if at_entry.digest != last.digest {
            return Err(refused(ModelTraceRefusal::LoopOpen { entry, steps }));
        }
    }
    for joint in 1..=steps {
        match matched(replay, counterexample, joint)? {
            Some(MatchReading::Holds) => {}
            Some(_) => {
                return Err(refused(ModelTraceRefusal::HyperUnmatched {
                    joint: u64::try_from(joint).unwrap_or(u64::MAX),
                }))
            }
            None => return Ok(None),
        }
    }
    for (component, trace) in counterexample.traces.iter().enumerate() {
        let Some(unfair) = unfair_trace(replay, component, trace, &executed[component], entry)?
        else {
            return Ok(None);
        };
        if unfair {
            return Err(refused(ModelTraceRefusal::HyperUnfair {
                variable: trace.variable.clone(),
            }));
        }
    }
    Ok(Some(()))
}

/// HP-2 (FR-183 "Lockstep"): the traces close at `loop_entry`, `μ_U` holds at
/// every joint step, each trace is fair on its own loop, and the body is
/// false on the tuple.
fn lockstep(
    replay: &HyperReplay<'_, '_>,
    counterexample: &HyperCounterexample,
    executed: &[Vec<Position>],
    loop_entry: u64,
) -> Result<ModelTraceSettlement, ReplayRefusal> {
    if universal_lasso(replay, counterexample, executed, loop_entry)?.is_none() {
        return Ok(no_value());
    }
    let steps = counterexample.steps();
    let entry = usize::try_from(loop_entry).map_err(|_| fault("a-loop-entry-fits-usize"))?;
    let shape = ComponentTrace::Lasso {
        prefix: u64::try_from(entry).unwrap_or(u64::MAX),
        loop_len: u64::try_from(steps - entry).unwrap_or(u64::MAX),
    };
    let components = vec![shape; executed.len()];
    let mut letters = Reexecuted {
        letters: replay.letters,
        instance: replay.instance,
        executed,
    };
    let mut meter = Meter::new(replay.meter).with_cancel(replay.cancel.clone());
    let outcome = evaluate_tuple(
        replay.clause,
        &components,
        &mut letters,
        &mut meter,
        Some(replay.cancel),
    )
    .map_err(|_| fault("the-evaluator-reads-the-lasso"))?;
    Ok(match outcome {
        TemporalOutcome::Violated { at } => ModelTraceSettlement::Reproduced {
            value: ModelTraceValue::Truth(false),
            trace_position: Some(at.index()),
        },
        TemporalOutcome::Incomplete(_) | TemporalOutcome::WindowLimitReached(_) => no_value(),
        other => ModelTraceSettlement::Inconclusive(disagreement(other.category())),
    })
}

/// HP-3 (FR-183 "WitnessExhausted"): the universal traces are checked as a
/// lasso, then the witness sets `X_0` to `X_position` are recomputed along
/// them by the witness product (FR-177's rule, under `max_witness_set`). The
/// counterexample reproduces when `X_position` is empty and every earlier set
/// is not.
fn witness_exhausted(
    replay: &HyperReplay<'_, '_>,
    counterexample: &HyperCounterexample,
    executed: &[Vec<Position>],
    loop_entry: u64,
    position: u64,
) -> Result<ModelTraceSettlement, ReplayRefusal> {
    if universal_lasso(replay, counterexample, executed, loop_entry)?.is_none() {
        return Ok(no_value());
    }
    let clause = replay.clause;
    let mut product = WitnessProduct::new(WitnessInputs {
        systems: replay.systems.clone(),
        universal: clause.universals(),
        instance: replay.instance,
        body: clause.body(),
        profile: clause.profile(),
        letters: replay.letters,
        universal_match: (!clause.mu_universal().is_empty()).then_some(replay.matcher),
        existential_match: (!clause.mu_existential().is_empty()).then_some(replay.matcher),
        max_automaton_states: u64::MAX,
        max_witness_set: replay.max_witness_set,
        meter: replay.meter,
        cancel: replay.cancel,
    })
    .map_err(|_| fault("the-recompiled-body-translates"))?;
    if product.start().is_err() {
        return halted(replay, &product);
    }
    // The product's state at joint position 0 is the one whose components are
    // the traces' initial states.
    let at = |state: &HyperState, joint: usize| -> Result<bool, ReplayRefusal> {
        for (component, model) in state.components.iter().enumerate() {
            let digest = digest_of(replay.systems[component], model)?;
            if executed[component][joint].digest != digest {
                return Ok(false);
            }
        }
        Ok(true)
    };
    let mut current = None;
    for state in product.initial() {
        if at(&state, 0)? {
            current = Some(state);
            break;
        }
    }
    let mut current = current.ok_or_else(|| fault("a-trace-starts-at-an-initial-product-state"))?;
    let position = usize::try_from(position).map_err(|_| fault("a-position-fits-usize"))?;
    for joint in 0..=position {
        if joint > 0 {
            let step = HyperTransition(
                counterexample
                    .traces
                    .iter()
                    .map(|trace| match &trace.steps[joint - 1] {
                        HyperStep::Model(step) => ComponentStep::Model(step.transition.clone()),
                        HyperStep::TerminalStutter => ComponentStep::Stutter,
                    })
                    .collect(),
            );
            let Ok(expansion) = product.successors(&current) else {
                return halted(replay, &product);
            };
            let mut next = None;
            for (taken, state) in expansion.successors {
                if taken == step
                    && matches!(state.property, Property::Witnesses(_))
                    && at(&state, joint)?
                {
                    next = Some(state);
                    break;
                }
            }
            current = next.ok_or_else(|| fault("a-matched-step-is-a-product-successor"))?;
        }
        let Property::Witnesses(set) = &current.property else {
            return Ok(ModelTraceSettlement::Inconclusive(disagreement(
                Category::Violation,
            )));
        };
        if set.is_empty() {
            return Ok(if joint == position {
                ModelTraceSettlement::Reproduced {
                    value: ModelTraceValue::Truth(false),
                    trace_position: Some(counterexample.trace_position),
                }
            } else {
                ModelTraceSettlement::Inconclusive(disagreement(Category::Success))
            });
        }
    }
    Ok(ModelTraceSettlement::Inconclusive(disagreement(
        Category::Success,
    )))
}

/// The replay of a witness product that halted: a set that would pass
/// `max_witness_set` stops it at that limit (FR-183), any other halt completes
/// no value.
fn halted(
    replay: &HyperReplay<'_, '_>,
    product: &WitnessProduct<'_, '_>,
) -> Result<ModelTraceSettlement, ReplayRefusal> {
    match product.take_halt() {
        Some(Halt::WitnessSet(_)) => Ok(ModelTraceSettlement::Stopped(ReachedLimit {
            limit: ModelCheckLimit::MaxWitnessSet,
            value: replay.max_witness_set,
        })),
        Some(Halt::Fault(_) | Halt::Encoding(_)) => Err(fault("the-witness-product-expands")),
        Some(_) => Ok(no_value()),
        None => Err(fault("a-halted-product-recorded-its-halt")),
    }
}

/// Whether trace `component` is unfair under its variable's own fairness set
/// on its loop from `entry` (ADR-023 HM-1); `None` when an expansion stops.
fn unfair_trace(
    replay: &HyperReplay<'_, '_>,
    component: usize,
    trace: &HyperTrace,
    positions: &[Position],
    entry: usize,
) -> Result<Option<bool>, ReplayRefusal> {
    let system = replay.systems[component];
    let fairness = &replay.clause.quantifiers()[component].fairness;
    let mut states = Vec::new();
    let mut stutters = 0;
    for (index, step) in trace.steps.iter().enumerate().skip(entry) {
        match step {
            HyperStep::TerminalStutter => stutters += 1,
            HyperStep::Model(ModelStep { transition, .. }) => {
                let Ok(expansion) = expand(system, &positions[index].state)? else {
                    return Ok(None);
                };
                states.push(LoopState {
                    position: index - entry,
                    enabled: expansion
                        .successors
                        .into_iter()
                        .map(|(transition, _)| transition)
                        .collect(),
                    taken: transition.clone(),
                });
            }
        }
    }
    // A terminal component stutters from its terminal state on, so its loop
    // is all stutter; a loop mixing the two does not close on a state that
    // both enables a step and does not.
    if stutters != 0 && !states.is_empty() {
        return Err(refused(ModelTraceRefusal::StutterNotTerminal {
            position: entry,
        }));
    }
    let lasso = Lasso {
        entry,
        terminal_stutter: stutters != 0,
    };
    Ok(Some(
        unfair_constraint(system, &states, lasso, fairness)?.is_some(),
    ))
}

/// An expression with no value, as the replay found it.
struct Found {
    position: usize,
    locus: HyperLocus,
    components: Vec<usize>,
    transition: Option<ModelTransition>,
    cause: UndefinedCause,
}

/// FR-183 "Undefined evaluation": the first expression with no value along
/// the paths, at the joint positions in EN-1's reading order.
fn undefined(
    replay: &HyperReplay<'_, '_>,
    counterexample: &HyperCounterexample,
    executed: &[Vec<Position>],
    claimed: &HyperUndefined,
) -> Result<ModelTraceSettlement, ReplayRefusal> {
    let steps = counterexample.steps();
    let automaton = translate_negation(replay.clause.body(), replay.clause.profile(), u64::MAX)
        .map_err(|_| fault("the-recompiled-body-translates"))?;
    let mut found = None;
    'positions: for joint in 0..=steps {
        if joint >= 1 {
            match matched(replay, counterexample, joint)? {
                Some(MatchReading::Holds) => {}
                Some(MatchReading::Undefined { conjunct, cause }) => {
                    found = Some(Found {
                        position: joint,
                        locus: HyperLocus::Match { conjunct },
                        components: (0..executed.len()).collect(),
                        transition: None,
                        cause,
                    });
                    break 'positions;
                }
                Some(MatchReading::Fails) => {
                    return Err(refused(ModelTraceRefusal::HyperUnmatched {
                        joint: u64::try_from(joint).unwrap_or(u64::MAX),
                    }))
                }
                Some(MatchReading::Refused) | None => return Ok(no_value()),
            }
            match matched_existentially(replay, counterexample, joint) {
                Some(MatchReading::Holds) => {}
                Some(MatchReading::Undefined { conjunct, cause }) => {
                    found = Some(Found {
                        position: joint,
                        locus: HyperLocus::ExistentialMatch { conjunct },
                        components: (0..executed.len()).collect(),
                        transition: None,
                        cause,
                    });
                    break 'positions;
                }
                // The path is one member's: a step `μ_E` rejects is not one
                // it took.
                Some(MatchReading::Fails) => {
                    return Ok(ModelTraceSettlement::Inconclusive(disagreement(
                        Category::Success,
                    )))
                }
                Some(MatchReading::Refused) | None => return Ok(no_value()),
            }
        }
        let at = states(executed, joint)?;
        for atom in automaton.atoms() {
            let mut meter = Meter::new(replay.meter);
            match replay.letters.atom(*atom, replay.instance, &at, &mut meter) {
                Ok(AtomReading::Undefined(cause)) => {
                    found = Some(Found {
                        position: joint,
                        locus: HyperLocus::Atom(*atom),
                        components: (0..executed.len()).collect(),
                        transition: None,
                        cause,
                    });
                    break 'positions;
                }
                Ok(AtomReading::True | AtomReading::False) => {}
                Err(_) => return Ok(no_value()),
            }
        }
        for (component, system) in replay.systems.iter().enumerate() {
            let Ok(expansion) = expand(*system, &executed[component][joint].state)? else {
                return Ok(no_value());
            };
            if let Some(undefined) = expansion
                .findings
                .iter()
                .find_map(qsl_eval::simulation::model::ModelFinding::undefined_contract)
            {
                found = Some(Found {
                    position: joint,
                    locus: HyperLocus::Contract(undefined.clause),
                    components: vec![component],
                    transition: undefined.guard_of,
                    cause: undefined.cause,
                });
                break 'positions;
            }
        }
    }
    let Some(found) = found else {
        return Ok(ModelTraceSettlement::Inconclusive(disagreement(
            Category::Success,
        )));
    };
    let evaluated = evaluated(replay, executed, &found)?;
    Ok(if evaluated == *claimed {
        ModelTraceSettlement::Reproduced {
            value: ModelTraceValue::HyperUndefined(evaluated),
            trace_position: Some(claimed.position),
        }
    } else {
        ModelTraceSettlement::Inconclusive(disagreement(Category::Violation))
    })
}

/// `found` as the payload spells it: the joint position, the digests of the
/// named components' states there and their variables with aliases.
fn evaluated(
    replay: &HyperReplay<'_, '_>,
    executed: &[Vec<Position>],
    found: &Found,
) -> Result<HyperUndefined, ReplayRefusal> {
    let quantifiers = replay.clause.quantifiers();
    let mut state_keys: Vec<DigestRecord> = Vec::new();
    let mut variables = Vec::new();
    for component in &found.components {
        state_keys.push(executed[*component][found.position].digest);
        let quantifier = &quantifiers[*component];
        variables.push((
            quantifier.variable.clone(),
            quantifier.alias.as_str().to_owned(),
        ));
    }
    Ok(HyperUndefined {
        position: u64::try_from(found.position).map_err(|_| fault("a-position-fits-u64"))?,
        state_keys,
        variables,
        locus: found.locus.clone(),
        transition: found.transition.clone(),
        cause: found.cause.clone(),
    })
}
