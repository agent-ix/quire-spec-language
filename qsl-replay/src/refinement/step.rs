// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-141 (ADR-020 RS-2 to RS-6): [`check_initial`] and [`check_step`], the
//! two functions that decide a refinement's initial states and steps.
//!
//! The model checker (FR-142) and replay (FR-145) call these and no other
//! step rule, as every temporal engine answers to the trace evaluator
//! (ADR-018 SM-1). A step is decided against the abstract model with the
//! calls the abstract `ModelSystem` makes when it expands the mapped
//! pre-state, applied to the one mapped post-state.
//!
//! A refinement's history values (FR-138) ride in the position beside the
//! concrete state. When it has a hidden abstract field (AX-2) a position
//! holds the candidate set: every abstract state whose visible part is the
//! mapped one and that is consistent with the behaviour so far. A step is
//! decided against each member, over every value the hidden fields of the
//! mapped post-state may take, and the set after it is the union of the
//! accepted ones.

use qsl_eval::simulation::model::{
    operation_identity_text, AbstractApplication, Judgement, ModelFinding, ModelSimState,
    ModelSystem, Rejection, ResolvedStep, RowReading,
};
use qsl_eval::simulation::{ExpansionLimit, ExpansionStop};
use qsl_foundation::digest::DigestRecord;
use qsl_foundation::InternalFault;
use qsl_semantics::check::{CheckedRefinement, CheckedRow, StepClass, StepRow, StepTarget};
use qsl_semantics::model::key::DeclarationKey;
use qsl_semantics::model::state::ModelState;
use quire_exact::{ObjectReference, Value};
use quire_semantic_value::model_transition::ModelTransition;

use crate::model_check::counterexample::{
    MappingSite, UndefinedEvaluation, UndefinedLocus, UndefinedSite,
};
use crate::model_check::limits::{ModelCheckLimit, ReachedLimit};
use crate::refinement::failure::{
    RefinementFailure, StepRejection, StepResult, StepVerdict, Taken, Undetermined,
};
use crate::refinement::history::{advance, has_update, initial_history, HistoryValues};
use crate::refinement::map::{
    lift_value, map_state, object_of, EvalFailure, Lifting, MappedState, MappingFailure,
    MappingRowRef, MappingUndetermined, StepArgument,
};

/// A concrete state with its history values and the abstract states it
/// stands for (FR-141 `RefinementPosition`). Without hidden fields the
/// abstract side is the one state the mapping gives; with them it is the
/// AX-2 candidate set.
#[derive(Clone, Debug)]
pub struct RefinementPosition {
    /// The concrete state.
    pub concrete: ModelState,
    /// The refinement's history values at it (FR-138).
    pub history: HistoryValues,
    /// The abstract states, sorted by state key.
    pub abstract_states: Vec<ModelState>,
}

/// One concrete step to decide: where it ends and what it did.
#[derive(Clone, Copy, Debug)]
pub struct ConcreteStep<'s> {
    /// The index of the step's post-state in the behaviour.
    pub position: u64,
    /// The transition identity; `None` is the concrete terminal stutter step
    /// (ADR-018 SM-4), which is a `stutter` step whatever rows the
    /// refinement has (RS-6).
    pub transition: Option<&'s ModelTransition>,
    /// The post-state, with the effect of the step that reached it.
    pub post: &'s ModelSimState,
}

fn fault(invariant: &'static str) -> StepVerdict {
    StepVerdict::Fault(InternalFault::new("refinement-step", invariant))
}

/// `state` without its hidden fields: the part of an abstract state a
/// mapping gives (AX-2), which every member of a candidate set shares.
fn visible_part(refinement: &CheckedRefinement, state: &ModelState) -> ModelState {
    let mut visible = state.clone();
    for objects in visible.populations.values_mut() {
        for object in objects.values_mut() {
            let hidden = refinement
                .objects()
                .iter()
                .find(|row| row.abstract_type == object.type_identity)
                .map(|row| row.hidden.as_slice())
                .unwrap_or_default();
            for field in hidden {
                object.fields.remove(field);
            }
        }
    }
    visible
}

/// `states` in state-key order, each state once.
#[allow(
    clippy::result_large_err,
    reason = "FR-141 fixes StepVerdict's shape: the verdict is also the early return of an inner step"
)]
fn sorted(
    system: &ModelSystem<'_>,
    states: impl IntoIterator<Item = ModelState>,
) -> Result<Vec<ModelState>, StepVerdict> {
    let mut keyed = Vec::new();
    for state in states {
        let (key, _) = system
            .state_key_of(&state)
            .map_err(|_| fault("abstract-state-encodes"))?;
        keyed.push((key, state));
    }
    keyed.sort_by(|left, right| left.0.cmp(&right.0));
    keyed.dedup_by(|later, earlier| later.0 == earlier.0);
    Ok(keyed.into_iter().map(|(_, state)| state).collect())
}

/// FR-141 "Initial states" (RS-2): `concrete_initial`, the initial state at
/// `index` of the concrete subject, maps to the abstract subject's own
/// initial states, compared by state key on the visible part (the whole
/// state when no field is hidden).
///
/// `Passes` carries `Taken::Stutter`: an initial state takes no step.
pub fn check_initial(
    refinement: &CheckedRefinement,
    concrete_system: &ModelSystem<'_>,
    abstract_system: &ModelSystem<'_>,
    index: usize,
    concrete_initial: &ModelState,
) -> StepVerdict {
    let history = initial_history(refinement, concrete_system, concrete_initial);
    let mapped = match map_state(
        refinement,
        concrete_system,
        abstract_system,
        concrete_initial,
        &history,
    ) {
        Ok(mapped) => mapped,
        Err(failure) => {
            return mapping_verdict(
                || state_digest(concrete_system, concrete_initial),
                0,
                failure,
            )
        }
    };
    let Ok((mapped_key, _)) = abstract_system.state_key_of(&mapped.visible) else {
        return fault("mapped-state-encodes");
    };
    let mut matching = Vec::new();
    for initial in abstract_system.initial_states() {
        let visible = visible_part(refinement, initial.state());
        let Ok((key, _)) = abstract_system.state_key_of(&visible) else {
            return fault("initial-state-encodes");
        };
        if key == mapped_key {
            matching.push(initial.state().clone());
        }
    }
    let candidates = match sorted(abstract_system, matching) {
        Ok(candidates) => candidates,
        Err(verdict) => return verdict,
    };
    if candidates.is_empty() {
        return StepVerdict::Fails(RefinementFailure::InitialNotAbstract { initial: index });
    }
    StepVerdict::Passes {
        post: RefinementPosition {
            concrete: concrete_initial.clone(),
            history,
            abstract_states: candidates,
        },
        taken: Taken::Stutter,
    }
}

/// FR-141 "Deciding the step rules" (RS-3 to RS-6): `step` taken from `pre`.
///
/// The outcome also counts every abstract transition identity the check
/// judged or expanded, which FR-142 adds to the run's `max_transitions`.
pub fn check_step(
    refinement: &CheckedRefinement,
    concrete_system: &ModelSystem<'_>,
    abstract_system: &ModelSystem<'_>,
    pre: &RefinementPosition,
    step: &ConcreteStep<'_>,
) -> StepResult {
    let mut abstract_successors = 0;
    let verdict = decide_step(
        refinement,
        concrete_system,
        abstract_system,
        pre,
        step,
        &mut abstract_successors,
    );
    StepResult {
        verdict,
        abstract_successors,
    }
}

/// The first undefined and the first undetermined outcome of the step-row
/// arguments, the history updates and the post-state's rows, in reading
/// order. Of the row outcomes the first undefined one outranks the first
/// undetermined one (QSpec FR-377 "Failure precedence").
#[derive(Default)]
struct Soft {
    undefined: Option<StepVerdict>,
    undetermined: Option<StepVerdict>,
}

impl Soft {
    /// Keep `verdict` if it is a row outcome; hand back any other verdict,
    /// which ends the check at once.
    #[allow(
        clippy::result_large_err,
        reason = "FR-141 fixes StepVerdict's shape: the verdict is also the early return of an inner step"
    )]
    fn record(&mut self, verdict: StepVerdict) -> Result<(), StepVerdict> {
        let slot = match verdict {
            StepVerdict::Undefined(_) => &mut self.undefined,
            StepVerdict::Undetermined(_) => &mut self.undetermined,
            other => return Err(other),
        };
        slot.get_or_insert(verdict);
        Ok(())
    }

    fn first(self) -> Option<StepVerdict> {
        self.undefined.or(self.undetermined)
    }
}

/// A verdict for an `ExpansionStop` of `system`: a budget stop is a
/// [`StepVerdict::Stopped`] naming the limit and its value (a result domain
/// too large to order counts against the candidate bound), a broken
/// invariant a fault.
fn stop_verdict(system: &ModelSystem<'_>, stop: ExpansionStop) -> StepVerdict {
    if stop.is_internal_failure() {
        return fault("system-invariant");
    }
    StepVerdict::Stopped(match stop.limit {
        Some(ExpansionLimit::EvaluationMeter) => ReachedLimit {
            limit: ModelCheckLimit::EvaluationMeter,
            value: system.meter().work_units,
        },
        Some(ExpansionLimit::MaxCandidates) | None => ReachedLimit {
            limit: ModelCheckLimit::MaxCandidates,
            value: system.max_candidates(),
        },
    })
}

#[allow(
    clippy::result_large_err,
    reason = "FR-141 fixes StepVerdict's shape: the verdict is also the early return of an inner step"
)]
fn decide_step(
    refinement: &CheckedRefinement,
    concrete_system: &ModelSystem<'_>,
    abstract_system: &ModelSystem<'_>,
    pre: &RefinementPosition,
    step: &ConcreteStep<'_>,
    abstract_successors: &mut u64,
) -> StepVerdict {
    let Some(before) = pre.abstract_states.first() else {
        return fault("a-position-holds-an-abstract-state");
    };
    let row = match step.transition {
        None => None,
        Some(transition) => match step_row(refinement, transition) {
            Some(row) => Some(row),
            None => return fault("every-concrete-operation-has-a-step-row"),
        },
    };
    let applies = matches!(
        row.map(|row| &row.target),
        Some(StepTarget::Abstract { .. })
    );
    // The step's receiver and arguments, found when a row or an update reads
    // them.
    let mut resolved = None;
    if let Some(transition) = step.transition {
        let Ok(updates) = has_update(refinement, transition) else {
            return fault("an-operation-has-an-identity-text");
        };
        if applies || updates {
            resolved = match concrete_system.resolve_step(&pre.concrete, transition, step.post) {
                Ok(Some(found)) => Some(found),
                Ok(None) => return fault("concrete-step-names-an-argument-vector"),
                Err(stop) => return stop_verdict(concrete_system, stop),
            };
        }
    }
    // Every receiver and argument of the row is read at the pre-state, the
    // history updates are evaluated and every row of the post-state is
    // mapped, in that order; the first undefined outcome of them outranks
    // the first undetermined one.
    let mut soft = Soft::default();
    let mut arguments = None;
    if let (Some(row), Some(resolved), true) = (row, resolved.as_ref(), applies) {
        match evaluate_arguments(
            refinement,
            (concrete_system, abstract_system),
            pre,
            step,
            (row, resolved),
        ) {
            Ok((evaluated, found)) => {
                arguments = evaluated;
                soft = found;
            }
            Err(verdict) => return verdict,
        }
    }
    let advanced = match advance(
        refinement,
        concrete_system,
        &pre.history,
        (&pre.concrete, step.post),
        step.transition.zip(resolved.as_ref()),
    ) {
        Ok(advanced) => advanced,
        Err(failure) => {
            return mapping_verdict(
                || state_digest(concrete_system, step.post.state()),
                step.position,
                failure,
            )
        }
    };
    for failure in advanced.outcomes.into_failures() {
        let verdict = mapping_verdict(
            || state_digest(concrete_system, step.post.state()),
            step.position,
            failure,
        );
        if let Err(ended) = soft.record(verdict) {
            return ended;
        }
    }
    let history = advanced.values;
    let mapped = match map_state(
        refinement,
        concrete_system,
        abstract_system,
        step.post.state(),
        &history,
    ) {
        Ok(mapped) => Some(mapped),
        Err(failure) => {
            let verdict = mapping_verdict(
                || state_digest(concrete_system, step.post.state()),
                step.position,
                failure,
            );
            if let Err(ended) = soft.record(verdict) {
                return ended;
            }
            None
        }
    };
    if let Some(verdict) = soft.first() {
        return verdict;
    }
    let Some(mapped) = mapped else {
        return fault("a-mapping-gives-a-state-or-a-verdict");
    };
    let Ok((after_key, _)) = abstract_system.state_key_of(&mapped.visible) else {
        return fault("mapped-state-encodes");
    };
    let Ok((before_key, _)) = abstract_system.state_key_of(&visible_part(refinement, before))
    else {
        return fault("mapped-state-encodes");
    };
    let stuttered = after_key == before_key;
    let hidden = !mapped.hidden.is_empty();
    let passes = |taken, states: Vec<ModelState>| StepVerdict::Passes {
        post: RefinementPosition {
            concrete: step.post.state().clone(),
            history: history.clone(),
            abstract_states: states,
        },
        taken,
    };
    let Some(row) = row else {
        return stutter(step, stuttered, || {
            passes(Taken::Stutter, pre.abstract_states.clone())
        });
    };
    match &row.target {
        StepTarget::Stutter => stutter(step, stuttered, || {
            passes(Taken::Stutter, pre.abstract_states.clone())
        }),
        StepTarget::Any => {
            if stuttered && !hidden {
                return passes(Taken::Stutter, pre.abstract_states.clone());
            }
            // The members after the step: the set itself for a stutter, and
            // every successor of a member whose visible part is the mapped
            // one.
            let mut next: Vec<ModelState> = if stuttered {
                pre.abstract_states.clone()
            } else {
                Vec::new()
            };
            let mut matched = Vec::new();
            let mut undecided = false;
            for candidate in &pre.abstract_states {
                let expansion = match abstract_system.expand_state(candidate) {
                    Ok(expansion) => expansion,
                    Err(stop) => return stop_verdict(abstract_system, stop),
                };
                *abstract_successors = abstract_successors
                    .saturating_add(u64::try_from(expansion.successors.len()).unwrap_or(u64::MAX));
                for (transition, successor) in &expansion.successors {
                    let visible = visible_part(refinement, successor.state());
                    match abstract_system.state_key_of(&visible) {
                        Ok((key, _)) if key == after_key => {
                            matched.push(transition.clone());
                            next.push(successor.state().clone());
                        }
                        Ok(_) => {}
                        Err(_) => return fault("successor-encodes"),
                    }
                }
                undecided |= expansion
                    .findings
                    .iter()
                    .any(|finding| matches!(finding, ModelFinding::ContractUndetermined { .. }));
            }
            if next.is_empty() {
                if undecided {
                    return StepVerdict::Undetermined(Undetermined::UndecidedSuccessor);
                }
                return StepVerdict::Fails(RefinementFailure::NoAbstractMatch {
                    position: step.position,
                });
            }
            let next = match sorted(abstract_system, next) {
                Ok(next) => next,
                Err(verdict) => return verdict,
            };
            if stuttered {
                return passes(Taken::Stutter, next);
            }
            match canonical(abstract_system, matched) {
                Ok(matched) => passes(Taken::Abstract(matched), next),
                Err(verdict) => verdict,
            }
        }
        StepTarget::Abstract { operation, .. } => {
            let Some(arguments) = arguments else {
                return fault("an-abstract-row-evaluates-its-arguments");
            };
            judge_operation(
                abstract_system,
                (&pre.abstract_states, &mapped),
                operation,
                arguments,
                step,
                abstract_successors,
                passes,
            )
        }
    }
}

/// RS-3: the step passes when the mapped state is unchanged.
fn stutter(
    step: &ConcreteStep<'_>,
    stuttered: bool,
    passes: impl FnOnce() -> StepVerdict,
) -> StepVerdict {
    if stuttered {
        passes()
    } else {
        StepVerdict::Fails(RefinementFailure::StutterChanged {
            position: step.position,
        })
    }
}

/// The step row of the operation `transition` applies (FR-136).
fn step_row<'r>(
    refinement: &'r CheckedRefinement,
    transition: &ModelTransition,
) -> Option<&'r StepRow> {
    refinement.steps().iter().find(|row| {
        let StepClass::Operation(operation) = &row.class;
        operation_identity_text(operation).is_ok_and(|text| text == transition.operation())
    })
}

/// `identities` in canonical order (FR-101): ascending transition-identity
/// bytes.
#[allow(
    clippy::result_large_err,
    reason = "FR-141 fixes StepVerdict's shape: the verdict is also the early return of an inner step"
)]
fn canonical(
    system: &ModelSystem<'_>,
    identities: Vec<ModelTransition>,
) -> Result<Vec<ModelTransition>, StepVerdict> {
    let mut keyed = Vec::with_capacity(identities.len());
    for identity in identities {
        let key = system
            .transition_key(&identity)
            .map_err(|_| fault("transition-encodes"))?;
        keyed.push((key, identity));
    }
    keyed.sort_by(|left, right| left.0.cmp(&right.0));
    keyed.dedup_by(|later, earlier| later.0 == earlier.0);
    Ok(keyed.into_iter().map(|(_, identity)| identity).collect())
}

/// What an abstract-operation row reads at the pre-state: the abstract
/// receiver, one value for each argument (`None` is `_`) and the abstract
/// result the concrete step's result binds, when the row binds one.
struct Arguments {
    receiver: ObjectReference,
    supplied: Vec<Option<Value>>,
    result: Option<Value>,
}

/// RS-4: the receiver and arguments of the abstract-operation row `row`,
/// evaluated at the pre-state over the concrete step's receiver and
/// arguments (FR-141 "Abstract-operation evaluation"), with the history
/// values the pre-state holds.
#[allow(
    clippy::result_large_err,
    reason = "FR-141 fixes StepVerdict's shape: the verdict is also the early return of an inner step"
)]
fn evaluate_arguments(
    refinement: &CheckedRefinement,
    (concrete_system, abstract_system): (&ModelSystem<'_>, &ModelSystem<'_>),
    pre: &RefinementPosition,
    step: &ConcreteStep<'_>,
    (row, resolved): (&StepRow, &ResolvedStep),
) -> Result<(Option<Arguments>, Soft), StepVerdict> {
    let StepRow {
        class: StepClass::Operation(concrete_operation),
        target:
            StepTarget::Abstract {
                receiver,
                arguments,
                binds_result,
                ..
            },
    } = row
    else {
        return Err(fault("an-abstract-row-names-an-operation"));
    };
    let observed = pre
        .history
        .environment(concrete_system, &pre.concrete)
        .map_err(|_| fault("pre-history-names-its-objects"))
        .and_then(|history| {
            concrete_system
                .observe_with_history(&pre.concrete, history)
                .map_err(|_| fault("pre-state-observes"))
        })?;
    let mut bindings = vec![Value::Reference(resolved.receiver.clone())];
    bindings.extend(resolved.arguments.iter().cloned());
    let object = object_of(concrete_system, &pre.concrete, &resolved.receiver);
    let evaluate =
        |expression: &CheckedRow, argument: StepArgument| -> Result<Value, StepVerdict> {
            let at = MappingRowRef::Argument {
                operation: concrete_operation.clone(),
                argument,
            };
            match concrete_system.read_row(expression, &observed, bindings.clone()) {
                Err(_) => Err(fault("argument-row-evaluates")),
                Ok(RowReading::Value(value)) => {
                    lift_value(refinement, concrete_system, abstract_system, value).map_err(
                        |lifting| match lifting {
                            Lifting::Unmapped(concrete_type) => StepVerdict::Undetermined(
                                Undetermined::Mapping(Box::new(MappingUndetermined {
                                    row: at.clone(),
                                    object: object.clone(),
                                    cause: EvalFailure::UnmappedType(concrete_type),
                                })),
                            ),
                            Lifting::Fault => fault("argument-lifts"),
                        },
                    )
                }
                Ok(RowReading::Undefined(cause)) => {
                    let Ok((_, digest)) = concrete_system.state_key_of(step.post.state()) else {
                        return Err(fault("post-state-encodes"));
                    };
                    Err(StepVerdict::Undefined(UndefinedEvaluation {
                        site: UndefinedSite {
                            position: step.position,
                            state_key: digest,
                            locus: UndefinedLocus::Mapping(Box::new(MappingSite {
                                row: at,
                                object: object.clone(),
                            })),
                            transition: None,
                        },
                        cause,
                    }))
                }
                Ok(RowReading::Undetermined(outcome)) => Err(StepVerdict::Undetermined(
                    Undetermined::Mapping(Box::new(MappingUndetermined {
                        row: at,
                        object: object.clone(),
                        cause: EvalFailure::Evaluation(Box::new(outcome)),
                    })),
                )),
            }
        };
    let mut soft = Soft::default();
    let mut read =
        |expression: &CheckedRow, argument: StepArgument| match evaluate(expression, argument) {
            Ok(value) => Ok(Some(value)),
            Err(verdict) => soft.record(verdict).map(|()| None),
        };
    let receiver = read(receiver, StepArgument::Receiver)?;
    let mut supplied = Vec::with_capacity(arguments.len());
    let mut complete = receiver.is_some();
    for (index, argument) in arguments.iter().enumerate() {
        match argument {
            None => supplied.push(None),
            Some(expression) => match read(expression, StepArgument::Parameter(index))? {
                Some(value) => supplied.push(Some(value)),
                None => complete = false,
            },
        }
    }
    let evaluated = match receiver {
        Some(Value::Reference(receiver)) if complete => Some(Arguments {
            receiver,
            supplied,
            result: if *binds_result {
                resolved.result.clone()
            } else {
                None
            },
        }),
        Some(Value::Reference(_)) | None => None,
        Some(_) => return Err(fault("a-receiver-row-is-a-reference")),
    };
    Ok((evaluated, soft))
}

/// The abstract states a mapped post-state stands for once its hidden fields
/// take each value of their finite domains (AX-2): the visible state itself
/// when no field is hidden.
#[allow(
    clippy::result_large_err,
    reason = "FR-141 fixes StepVerdict's shape: the verdict is also the early return of an inner step"
)]
fn completions(
    system: &ModelSystem<'_>,
    mapped: &MappedState,
) -> Result<Vec<ModelState>, StepVerdict> {
    if mapped.hidden.is_empty() {
        return Ok(vec![mapped.visible.clone()]);
    }
    // A hidden reference ranges over the objects of the mapped state.
    let objects: Vec<ObjectReference> = mapped
        .visible
        .populations
        .values()
        .flatten()
        .filter_map(|(key, object)| system.object_reference(&object.type_identity, key).ok())
        .collect();
    let mut domains: Vec<Vec<Option<Value>>> = Vec::with_capacity(mapped.hidden.len());
    for slot in &mapped.hidden {
        let Some(object) = mapped
            .visible
            .populations
            .get(&slot.object.population)
            .and_then(|objects| objects.get(&slot.object.key))
        else {
            return Err(fault("a-hidden-slot-names-an-object"));
        };
        let Some(field) = system
            .fields_of(&object.type_identity)
            .and_then(|fields| fields.iter().find(|field| field.key == slot.field))
        else {
            return Err(fault("a-hidden-slot-names-a-field"));
        };
        let mut values: Vec<Option<Value>> = system
            .domain_values(&field.value_type, &objects)
            .map_err(|stop| stop_verdict(system, stop))?
            .into_iter()
            .map(Some)
            .collect();
        if field.optional {
            values.insert(0, None);
        }
        domains.push(values);
    }
    let total = domains.iter().try_fold(1u64, |total, domain| {
        total.checked_mul(u64::try_from(domain.len()).ok()?)
    });
    if total.is_none_or(|total| total > system.max_candidates()) {
        return Err(StepVerdict::Stopped(ReachedLimit {
            limit: ModelCheckLimit::MaxCandidates,
            value: system.max_candidates(),
        }));
    }
    let mut digits = vec![0_usize; domains.len()];
    let mut states = Vec::new();
    'assignments: loop {
        if domains.iter().any(Vec::is_empty) {
            break;
        }
        let mut state = mapped.visible.clone();
        for ((slot, domain), digit) in mapped.hidden.iter().zip(&domains).zip(&digits) {
            let Some(Some(value)) = domain.get(*digit) else {
                continue;
            };
            if let Some(object) = state
                .populations
                .get_mut(&slot.object.population)
                .and_then(|objects| objects.get_mut(&slot.object.key))
            {
                object.fields.insert(slot.field.clone(), value.clone());
            }
        }
        states.push(state);
        // The last slot moves fastest.
        let mut at = digits.len();
        loop {
            let Some(previous) = at.checked_sub(1) else {
                break 'assignments;
            };
            at = previous;
            digits[at] += 1;
            if digits[at] < domains[at].len() {
                break;
            }
            digits[at] = 0;
        }
    }
    Ok(states)
}

/// RS-4: the abstract operation `operation` over the evaluated `arguments`,
/// judged against each completion of the mapped post-state from each member
/// of `pre_states`, with the calls the abstract system makes when it expands
/// the member. The members after the step are the accepted completions.
fn judge_operation(
    abstract_system: &ModelSystem<'_>,
    (pre_states, mapped): (&[ModelState], &MappedState),
    operation: &DeclarationKey,
    arguments: Arguments,
    step: &ConcreteStep<'_>,
    abstract_successors: &mut u64,
    passes: impl FnOnce(Taken, Vec<ModelState>) -> StepVerdict,
) -> StepVerdict {
    let completions = match completions(abstract_system, mapped) {
        Ok(completions) => completions,
        Err(verdict) => return verdict,
    };
    let mut next = Vec::new();
    let mut accepted = Vec::new();
    let mut rejected = Vec::new();
    let mut undecided = false;
    let mut order = 0_usize;
    for before in pre_states {
        for post in &completions {
            let judged = match abstract_system.judge_application(&AbstractApplication {
                pre: before,
                receiver: &arguments.receiver,
                operation,
                arguments: &arguments.supplied,
                post,
                result: arguments.result.as_ref(),
            }) {
                Ok(judged) => judged,
                Err(stop) => return stop_verdict(abstract_system, stop),
            };
            *abstract_successors =
                abstract_successors.saturating_add(u64::try_from(judged.len()).unwrap_or(u64::MAX));
            for found in judged {
                let Ok(key) = abstract_system.transition_key(&found.transition) else {
                    return fault("transition-encodes");
                };
                match found.judgement {
                    Judgement::Accepted => {
                        next.push(post.clone());
                        accepted.push(found.transition);
                    }
                    Judgement::Undecided(_) => undecided = true,
                    Judgement::Rejected(rejection) => {
                        rejected.push((key, order, found.transition, rejection));
                    }
                }
                order += 1;
            }
        }
    }
    if !accepted.is_empty() {
        let (taken, next) = match (
            canonical(abstract_system, accepted),
            sorted(abstract_system, next),
        ) {
            (Ok(taken), Ok(next)) => (taken, next),
            (Err(verdict), _) | (_, Err(verdict)) => return verdict,
        };
        return passes(Taken::Abstract(taken), next);
    }
    if undecided {
        return StepVerdict::Undetermined(Undetermined::UndecidedSuccessor);
    }
    rejected.sort_by(|left, right| left.0.cmp(&right.0).then(left.1.cmp(&right.1)));
    match rejected.into_iter().next() {
        Some((_, _, transition, rejection)) => {
            StepVerdict::Fails(RefinementFailure::AbstractStepRejected {
                position: step.position,
                transition,
                cause: match rejection {
                    Rejection::Precondition { clause } => StepRejection::Precondition { clause },
                    Rejection::Frame { code } => StepRejection::Frame { code },
                    Rejection::Postcondition => StepRejection::Postcondition,
                },
            })
        }
        None => StepVerdict::Fails(RefinementFailure::NoAbstractMatch {
            position: step.position,
        }),
    }
}

/// The state-key digest of `state` in `system`.
fn state_digest(system: &ModelSystem<'_>, state: &ModelState) -> Option<DigestRecord> {
    system.state_key_of(state).ok().map(|(_, digest)| digest)
}

/// The verdict of a mapping that gave no state, for the state at `position`,
/// whose state-key digest `digest` reads.
fn mapping_verdict(
    digest: impl FnOnce() -> Option<DigestRecord>,
    position: u64,
    failure: MappingFailure,
) -> StepVerdict {
    match failure {
        MappingFailure::Undefined { row, object, cause } => {
            let Some(digest) = digest() else {
                return fault("concrete-state-encodes");
            };
            StepVerdict::Undefined(UndefinedEvaluation {
                site: UndefinedSite {
                    position,
                    state_key: digest,
                    locus: UndefinedLocus::Mapping(Box::new(MappingSite { row, object })),
                    transition: None,
                },
                cause: *cause,
            })
        }
        MappingFailure::Undetermined(undetermined) => {
            StepVerdict::Undetermined(Undetermined::Mapping(Box::new(undetermined)))
        }
        MappingFailure::WrongSystem(_) => fault("the-engine-passed-the-refinements-own-systems"),
        MappingFailure::Fault(_) => fault("mapping-evaluation"),
    }
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;
    use qsl_foundation::CatalogCode;

    use super::*;

    /// FR-141 (TC-555): an undetermined row outcome is kept for the
    /// precedence, and any other verdict ends the check at once.
    #[trace("TC-555")]
    #[test]
    fn an_undetermined_row_is_kept_and_any_other_verdict_ends_the_check() {
        let mut soft = Soft::default();
        assert!(soft
            .record(StepVerdict::Undetermined(Undetermined::UndecidedSuccessor))
            .is_ok());
        assert!(soft
            .record(StepVerdict::Fails(RefinementFailure::NoAbstractMatch {
                position: 1
            }))
            .is_err());
        assert!(matches!(
            soft.first(),
            Some(StepVerdict::Undetermined(Undetermined::UndecidedSuccessor))
        ));
    }

    /// FR-141 (TC-555): of the row outcomes recorded the undefined one
    /// outranks an undetermined one recorded before it.
    #[trace("TC-555")]
    #[test]
    fn an_undefined_row_outranks_an_undetermined_one_recorded_first() {
        use qsl_foundation::digest::{DigestDomain, DigestRecord};

        let undefined = StepVerdict::Undefined(UndefinedEvaluation {
            site: UndefinedSite {
                position: 2,
                state_key: DigestRecord::mint(DigestDomain::Sha256Jcs, [0; 32]),
                locus: UndefinedLocus::Constraint("c".to_owned()),
                transition: None,
            },
            cause: qsl_eval::temporal::UndefinedCause::InsufficientData(
                qsl_eval::temporal::InsufficientData {
                    count: 0,
                    min_count: 1,
                },
            ),
        });
        let mut soft = Soft::default();
        assert!(soft
            .record(StepVerdict::Undetermined(Undetermined::UndecidedSuccessor))
            .is_ok());
        assert!(soft.record(undefined).is_ok());
        assert!(matches!(soft.first(), Some(StepVerdict::Undefined(_))));
    }

    /// FR-141-AC-8 (TC-555 step 4): a `Fault` and a `WrongSystem` from
    /// FR-140 are each a `StepVerdict::Fault`, never `Undefined`,
    /// `Undetermined` or `Fails`, and the state is not even keyed for them.
    #[trace("TC-555", "FR-141-AC-8")]
    #[test]
    fn a_mapping_fault_and_a_wrong_system_are_faults() {
        let code = CatalogCode::new("runtime_invariant", "established-invariant-broken");
        for failure in [
            MappingFailure::Fault(code),
            MappingFailure::WrongSystem(code),
        ] {
            let verdict = mapping_verdict(|| unreachable!("a fault keys no state"), 3, failure);
            assert!(matches!(verdict, StepVerdict::Fault(_)), "{verdict:?}");
        }
    }
}
