// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-183 (ADR-023 HX-2, HX-5, PA-7): [`replay_hyper_trace_tuple`], the replay
//! of a hyper counterexample's `Lockstep`, `WitnessExhausted`, `Projected` and
//! `Undefined` arms
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
//!   the budget stops the replay at it, stating the count it reached. The
//!   engine rotates the lasso to its canonical form, so `position` can be past
//!   the last step; positions past it read around the loop. An expansion whose
//!   findings say a member was dropped, or could not be told to be kept (an
//!   undefined or refused `μ` or contract), completes no value.
//! - `Undefined`: the traces are paths to the joint position the payload
//!   names. The replay reads `μ_U`, `μ_E`, the body's atoms and each component's
//!   contract conjunctions at the joint positions up to it, in the order EN-1
//!   reads them, and settles `Reproduced` exactly when the first expression
//!   with no value is the payload's, at its position, with its cause.
//!
//! - `Projected` (HP-6): each trace's skipped marks name operations of the
//!   clause's `align skip` list and its visible marks name none, the traces
//!   take equal numbers of visible steps, `trace_position` is that number,
//!   and `μ_U` holds at every joint visible step. The replay then evaluates
//!   the body three-valued over the joint projected prefix, whose position
//!   `j` reads each trace's state after its `j`-th visible step (FR-178):
//!   `false` settles `Reproduced` at the shortest bad prefix's position, anything else
//!   `Inconclusive`. A trace of a projected counterexample has no loop and no
//!   fairness to check.
//!
//! The facade entry `replay_model_trace_tuple` recompiles the package and
//! resolves the clause, which the S4 emission of a hyper clause will supply;
//! this function takes the recompiled clause and the admitted systems it
//! would hand over. A counterexample that breaks QSpec FR-400's reader rules
//! is refused here as the shape it breaks. The `step-tuple` arm joins with
//! its replay. An `Undefined` counterexample of an `align skip` clause holds
//! skipped steps, unmarked, and is refused as not yet implemented.

use qsl_eval::automaton::translate_negation;
use qsl_eval::hyper::{
    ComponentStep, Halt, HyperFinding, HyperLetters, HyperState, HyperTransition, MatchReading,
    ParameterBinding, Property, StepLabel, StepMatch, WitnessInputs, WitnessProduct,
};
use qsl_eval::simulation::model::{ModelSimState, ModelSystem};
use qsl_eval::simulation::TransitionSystem;
use qsl_eval::temporal::{
    evaluate_projected_prefix, evaluate_tuple, AtomReading, ComponentPosition, ComponentTrace,
    LetterStop, TemporalOutcome, TemporalPosition, TupleLetters, UndefinedCause,
};
use std::collections::BTreeSet;

use qsl_foundation::digest::DigestRecord;
use qsl_semantics::check::{CheckedHyperClause, FormulaNodeId, HyperForm};
use quire_exact::{Cancel, Meter, ScalarLimits};
use quire_semantic_value::catalog::Category;
use quire_semantic_value::model_transition::ModelTransition;

use crate::execute::ReplayRefusal;
use crate::model_check::counterexample::{Lasso, ModelStep};
use crate::model_check::hyper_counterexample::{
    HyperCounterexample, HyperCounterexampleKind, HyperLocus, HyperStep, HyperTrace,
    HyperUndefined, StepMark,
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
/// does; an `Undefined` counterexample of an `align skip` clause,
/// `HyperProjected`; a `Projected` counterexample with a mark that does not
/// agree with the `align skip` list, unequal visible counts or a step that is a
/// stutter (`HyperShape`); a joint step at which `μ_U` is false
/// (`HyperUnmatched`); and a trace that is unfair on its own loop
/// (`HyperUnfair`). An expansion that stops, a
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
    if counterexample.kind == HyperCounterexampleKind::Projected {
        return projected(replay, counterexample);
    }
    // A clause with `align skip` is read over projected prefixes, so even an
    // `Undefined` counterexample of one has paths of unequal length and
    // positions that count visible steps: FR-183 does not state their replay.
    if !replay.clause.align_skip().is_empty() {
        return Err(match counterexample.kind {
            HyperCounterexampleKind::Undefined(_) => refused(ModelTraceRefusal::HyperProjected),
            _ => shape_error(WRONG_KIND_FOR_ALIGN_SKIP),
        });
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

/// The reason a lockstep or witness counterexample of an `align skip` clause
/// is refused: that clause is read over projected prefixes (FR-183 "Projected").
const WRONG_KIND_FOR_ALIGN_SKIP: &str =
    "is of a kind that a clause with `align skip` is not read as: it is projected";
const SKIPPED_NOT_LISTED: &str =
    "marks a step skipped whose operation is not in the `align skip` list";
const VISIBLE_LISTED: &str = "marks a step visible whose operation is in the `align skip` list";
const UNEQUAL_VISIBLE: &str = "holds traces of unequal visible step counts";
const POSITION_NOT_VISIBLE: &str = "names a joint position other than its visible step count";

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
    let traced = match counterexample.kind {
        HyperCounterexampleKind::WitnessExhausted { .. } => replay.clause.universals(),
        _ => replay.clause.quantifiers().len(),
    };
    about_clause(replay, counterexample, traced)?;
    let steps = counterexample.steps();
    if counterexample
        .traces
        .iter()
        .any(|trace| trace.steps.len() != steps)
    {
        return Err(shape_error("holds traces of unequal step counts"));
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
            if *position != counterexample.trace_position {
                return Err(shape_error(
                    "names a joint position other than its trace position",
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

/// The checks of a counterexample's shape that every kind shares: `traced`
/// traces, each with its quantifier's variable and alias in quantifier order,
/// and the request's instance.
fn about_clause(
    replay: &HyperReplay<'_, '_>,
    counterexample: &HyperCounterexample,
    traced: usize,
) -> Result<(), ReplayRefusal> {
    let quantifiers = replay.clause.quantifiers();
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
    let bound: Vec<(String, String)> = replay
        .instance
        .iter()
        .map(|binding| (binding.parameter.clone(), binding.instance.key.clone()))
        .collect();
    if counterexample.instance != bound {
        return Err(shape_error("names an instance other than the request's"));
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
    Ok(universal_reading(replay, &labels))
}

/// `μ_U` over the universal components' step `labels`; `None` when its
/// evaluation is refused or incomplete.
fn universal_reading(
    replay: &HyperReplay<'_, '_>,
    labels: &[StepLabel<'_>],
) -> Option<MatchReading> {
    let mut meter = Meter::new(replay.meter);
    replay
        .matcher
        .universal(replay.instance, labels, &mut meter)
        .ok()
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

fn fault_of(invariant: &'static str) -> quire_semantic_value::catalog::InternalFault {
    quire_semantic_value::catalog::InternalFault::new("replay-hyper-trace-tuple", invariant)
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
    let steps = counterexample.steps();
    let entry = usize::try_from(loop_entry).map_err(|_| fault("a-loop-entry-fits-usize"))?;
    let lasso = Unrolled {
        entry,
        loop_len: steps - entry,
        steps,
    };
    let at = |state: &HyperState, joint: usize| -> Result<bool, ReplayRefusal> {
        for (component, model) in state.components.iter().enumerate() {
            let digest = digest_of(replay.systems[component], model)?;
            if executed[component][lasso.state(joint)].digest != digest {
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
                    .map(|trace| match &trace.steps[lasso.step(joint)] {
                        HyperStep::Model(step) => ComponentStep::Model(step.transition.clone()),
                        HyperStep::TerminalStutter => ComponentStep::Stutter,
                    })
                    .collect(),
            );
            let Ok(expansion) = product.successors(&current) else {
                return halted(replay, &product);
            };
            // A finding says the expansion dropped a member, or could not tell
            // whether it did: the set that follows is not the one the engine
            // computed, so nothing is reproduced (FR-177).
            if expansion.findings.iter().any(drops_a_member) {
                return Ok(no_value());
            }
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

/// A lasso of `steps` steps re-entering at `entry`, read past its last step
/// by going around the loop: the engine's canonical rotation can leave a
/// counterexample fewer steps than its joint position (FR-177).
struct Unrolled {
    entry: usize,
    loop_len: usize,
    steps: usize,
}

impl Unrolled {
    /// The state index of joint position `joint`.
    fn state(&self, joint: usize) -> usize {
        if joint <= self.steps {
            joint
        } else {
            self.entry + (joint - self.entry) % self.loop_len
        }
    }

    /// The index in a trace's steps of the step into joint position `joint`,
    /// which is `1` or more.
    fn step(&self, joint: usize) -> usize {
        let from = joint - 1;
        if from < self.steps {
            from
        } else {
            self.entry + (from - self.entry) % self.loop_len
        }
    }
}

/// Whether `finding` of a witness-product expansion says a member of a
/// witness set was dropped or could not be told to be kept: an undefined or
/// refused contract, an undefined or refused `μ`.
fn drops_a_member(finding: &HyperFinding) -> bool {
    match finding {
        HyperFinding::Contract { finding, .. } => {
            finding.undefined_contract().is_some() || finding.is_refused_contract()
        }
        HyperFinding::Enabled { .. } => false,
        HyperFinding::MatchUndefined { .. }
        | HyperFinding::MatchRefused
        | HyperFinding::ExistentialMatchUndefined { .. }
        | HyperFinding::MemberContract => true,
    }
}

/// The replay of a witness product that halted: a set that would pass
/// `max_witness_set` stops it at that limit and states the count it reached
/// (FR-183, FR-184), any other halt completes no value.
fn halted(
    replay: &HyperReplay<'_, '_>,
    product: &WitnessProduct<'_, '_>,
) -> Result<ModelTraceSettlement, ReplayRefusal> {
    match product.take_halt() {
        Some(Halt::WitnessSet(count)) => Ok(ModelTraceSettlement::Stopped {
            reached: ReachedLimit {
                limit: ModelCheckLimit::MaxWitnessSet,
                value: replay.max_witness_set,
            },
            count: Some(count),
        }),
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
    let automaton = translate_negation(
        replay.clause.body(),
        replay.clause.profile(),
        u64::MAX,
        None,
    )
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

/// HP-6 (FR-183 "Projected"): the marks agree with the `align skip` list, the
/// visible counts are equal and `μ_U` holds at every joint visible step; the
/// body is then read three-valued over the joint projected prefix, and a bad
/// prefix reproduces.
fn projected(
    replay: &HyperReplay<'_, '_>,
    counterexample: &HyperCounterexample,
) -> Result<ModelTraceSettlement, ReplayRefusal> {
    if replay.clause.form() != Some(&HyperForm::ProjectionAligned) {
        return Err(shape_error(
            "is of a clause that is not a projection-aligned hyperproperty",
        ));
    }
    about_clause(replay, counterexample, replay.clause.quantifiers().len())?;
    let visible = visible_steps(replay, counterexample)?;
    let projected = visible.first().map_or(0, Vec::len);
    if visible.iter().any(|steps| steps.len() != projected) {
        return Err(shape_error(UNEQUAL_VISIBLE));
    }
    if counterexample.trace_position != u64::try_from(projected).unwrap_or(u64::MAX) {
        return Err(shape_error(POSITION_NOT_VISIBLE));
    }
    let Some(executed) = execute(replay, counterexample)? else {
        return Ok(no_value());
    };
    for joint in 1..=projected {
        let labels: Vec<StepLabel<'_>> = (0..replay.clause.universals())
            .map(|component| {
                match &counterexample.traces[component].steps[visible[component][joint - 1]] {
                    HyperStep::Model(step) => StepLabel::Operation(&step.transition),
                    HyperStep::TerminalStutter => StepLabel::Stutter,
                }
            })
            .collect();
        let reading = if replay.clause.mu_universal().is_empty() {
            Some(MatchReading::Holds)
        } else {
            universal_reading(replay, &labels)
        };
        match reading {
            Some(MatchReading::Holds) => {}
            Some(_) => {
                return Err(refused(ModelTraceRefusal::HyperUnmatched {
                    joint: u64::try_from(joint).unwrap_or(u64::MAX),
                }))
            }
            None => return Ok(no_value()),
        }
    }
    // Projected position 0 is a trace's initial state; position `j` is the
    // state its `j`-th visible step enters.
    let entered: Vec<Vec<usize>> = visible
        .iter()
        .map(|steps| {
            std::iter::once(0)
                .chain(steps.iter().map(|step| step + 1))
                .collect()
        })
        .collect();
    let evaluate = |len: usize| -> Result<TemporalOutcome, ReplayRefusal> {
        let mut letters = ProjectedLetters {
            letters: replay.letters,
            instance: replay.instance,
            executed: &executed,
            entered: &entered,
        };
        let mut meter = Meter::new(replay.meter).with_cancel(replay.cancel.clone());
        evaluate_projected_prefix(
            replay.clause,
            u64::try_from(len).unwrap_or(u64::MAX),
            &mut letters,
            &mut meter,
        )
        .map_err(|_| fault("the-evaluator-reads-the-projected-prefix"))
    };
    let longest = projected + 1;
    // FR-183 and ADR-023 PA-7 differ on the kind of an inconclusive projected
    // replay (`Verdicts` or `ReplayParity`); this follows FR-183 (QSL-1044).
    match evaluate(longest)? {
        TemporalOutcome::Violated { .. } => {}
        TemporalOutcome::Incomplete(_) | TemporalOutcome::WindowLimitReached(_) => {
            return Ok(no_value())
        }
        other => {
            return Ok(ModelTraceSettlement::Inconclusive(disagreement(
                other.category(),
            )))
        }
    }
    // A bad prefix stays bad as it grows, so the shortest one is found by
    // bisection: `low` is not known to be bad, `high` is. The replay reports
    // the position that fixes the `false`, as the lockstep replay does, not the
    // position the payload names.
    let (mut low, mut high) = (0, longest);
    while high - low > 1 {
        let middle = low + (high - low) / 2;
        match evaluate(middle)? {
            TemporalOutcome::Violated { .. } => high = middle,
            TemporalOutcome::Incomplete(_) | TemporalOutcome::WindowLimitReached(_) => {
                return Ok(no_value())
            }
            _ => low = middle,
        }
    }
    Ok(ModelTraceSettlement::Reproduced {
        value: ModelTraceValue::Truth(false),
        trace_position: Some(u64::try_from(high - 1).unwrap_or(u64::MAX)),
    })
}

/// The indices of each trace's visible steps, after checking that each
/// skipped mark names an operation of the clause's `align skip` list in its
/// trace's subject and each visible mark one that is not. A projected path
/// holds no stutter step (FR-178).
fn visible_steps(
    replay: &HyperReplay<'_, '_>,
    counterexample: &HyperCounterexample,
) -> Result<Vec<Vec<usize>>, ReplayRefusal> {
    let mut visible = Vec::with_capacity(counterexample.traces.len());
    for (trace, system) in counterexample.traces.iter().zip(&replay.systems) {
        let listed: BTreeSet<&str> = replay
            .clause
            .align_skip()
            .iter()
            .filter_map(|key| system.operation_text_of(key))
            .collect();
        let mut steps = Vec::new();
        for (index, (step, mark)) in trace.steps.iter().zip(&trace.marks).enumerate() {
            let HyperStep::Model(step) = step else {
                return Err(shape_error(
                    "holds a terminal stutter step in a projected path",
                ));
            };
            let skipped = listed.contains(step.transition.operation());
            match (mark, skipped) {
                (StepMark::Skipped, true) => {}
                (StepMark::Visible, false) => steps.push(index),
                (StepMark::Skipped, false) => return Err(shape_error(SKIPPED_NOT_LISTED)),
                (StepMark::Visible, true) => return Err(shape_error(VISIBLE_LISTED)),
            }
        }
        visible.push(steps);
    }
    Ok(visible)
}

/// The body's atoms on the joint projected prefix: component `c` at projected
/// position `p` reads the state its `p`-th visible step entered, which is
/// `entered[c][p]` of its re-executed positions (FR-178 "Projection").
struct ProjectedLetters<'a> {
    letters: &'a dyn HyperLetters,
    instance: &'a [ParameterBinding],
    executed: &'a [Vec<Position>],
    entered: &'a [Vec<usize>],
}

impl TupleLetters for ProjectedLetters<'_> {
    fn atom(
        &mut self,
        atom: FormulaNodeId,
        _joint: TemporalPosition,
        components: &[ComponentPosition],
        meter: &mut Meter,
    ) -> Result<AtomReading, LetterStop> {
        let mut states: Vec<&ModelSimState> = Vec::with_capacity(components.len());
        for (component, read) in components.iter().enumerate() {
            let projected = usize::try_from(read.position.index())
                .map_err(|_| LetterStop::Fault(fault_of("a-position-fits-usize")))?;
            let position = self
                .entered
                .get(component)
                .and_then(|entered| entered.get(projected))
                .and_then(|index| self.executed.get(component)?.get(*index))
                .ok_or_else(|| LetterStop::Fault(fault_of("a-projected-position-is-a-state")))?;
            states.push(&position.state);
        }
        self.letters
            .atom(atom, self.instance, &states, meter)
            .map_err(LetterStop::Incomplete)
    }
}
