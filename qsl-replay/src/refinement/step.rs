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
//! History fields (FR-138) and hidden fields (the AX-2 candidate sets) are
//! not built: S3 refuses a history row, and a refinement with a hidden field
//! returns [`StepVerdict::Fault`] here, because no candidate set stands for
//! its abstract state.

use qsl_eval::simulation::model::{
    operation_identity_text, AbstractApplication, Judgement, ModelFinding, ModelSimState,
    ModelSystem, Rejection, RowReading,
};
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
use crate::refinement::failure::{
    RefinementFailure, StepRejection, StepVerdict, Taken, Undetermined,
};
use crate::refinement::map::{
    lift_value, map_state, EvalFailure, Lifting, MappedState, MappingFailure, MappingRowRef,
    MappingUndetermined, ObjectKey, StepArgument,
};

/// A concrete state with the abstract states it stands for (FR-141
/// `RefinementPosition`). Without hidden fields the abstract side is the one
/// state the mapping gives; the AX-2 set and the history values arrive with
/// the hidden and history fields.
#[derive(Clone, Debug)]
pub struct RefinementPosition {
    /// The concrete state.
    pub concrete: ModelState,
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

/// FR-141 "Initial states" (RS-2): `concrete_initial`, the initial state at
/// `index` of the concrete subject, maps to one of the abstract subject's
/// own initial states, compared by state key.
///
/// `Passes` carries `Taken::Stutter`: an initial state takes no step.
pub fn check_initial(
    refinement: &CheckedRefinement,
    concrete_system: &ModelSystem<'_>,
    abstract_system: &ModelSystem<'_>,
    index: usize,
    concrete_initial: &ModelState,
) -> StepVerdict {
    let mapped = match map_state(
        refinement,
        concrete_system,
        abstract_system,
        concrete_initial,
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
    if !mapped.hidden.is_empty() {
        return fault("hidden-fields-need-candidate-sets");
    }
    let Ok((mapped_key, _)) = abstract_system.state_key_of(&mapped.visible) else {
        return fault("mapped-state-encodes");
    };
    for initial in abstract_system.initial_states() {
        let Ok((key, _)) = abstract_system.state_key(initial) else {
            return fault("initial-state-encodes");
        };
        if key == mapped_key {
            return StepVerdict::Passes {
                post: RefinementPosition {
                    concrete: concrete_initial.clone(),
                    abstract_states: vec![mapped.visible],
                },
                taken: Taken::Stutter,
            };
        }
    }
    StepVerdict::Fails(RefinementFailure::InitialNotAbstract { initial: index })
}

/// FR-141 "Deciding the step rules" (RS-3 to RS-6): `step` taken from `pre`.
///
/// `abstract_successors` counts every abstract transition identity this call
/// judged or expanded, which FR-142 adds to the run's `max_transitions`.
pub fn check_step(
    refinement: &CheckedRefinement,
    concrete_system: &ModelSystem<'_>,
    abstract_system: &ModelSystem<'_>,
    pre: &RefinementPosition,
    step: &ConcreteStep<'_>,
    abstract_successors: &mut u64,
) -> StepVerdict {
    let [before] = pre.abstract_states.as_slice() else {
        return fault("one-abstract-state-without-hidden-fields");
    };
    let row = match step.transition {
        None => None,
        Some(transition) => match step_row(refinement, transition) {
            Some(row) => Some(row),
            None => return fault("every-concrete-operation-has-a-step-row"),
        },
    };
    // The row's receiver and arguments are read at the pre-state, before the
    // post-state is mapped.
    let arguments = match (row.map(|row| &row.target), step.transition) {
        (Some(StepTarget::Abstract { .. }), Some(transition)) => {
            match evaluate_arguments(
                refinement,
                concrete_system,
                abstract_system,
                pre,
                step,
                (row, transition),
            ) {
                Ok(arguments) => Some(arguments),
                Err(verdict) => return verdict,
            }
        }
        _ => None,
    };
    let mapped = match map_state(
        refinement,
        concrete_system,
        abstract_system,
        step.post.state(),
    ) {
        Ok(mapped) => mapped,
        Err(failure) => {
            return mapping_verdict(
                || state_digest(concrete_system, step.post.state()),
                step.position,
                failure,
            )
        }
    };
    if !mapped.hidden.is_empty() {
        return fault("hidden-fields-need-candidate-sets");
    }
    let Ok((after_key, _)) = abstract_system.state_key_of(&mapped.visible) else {
        return fault("mapped-state-encodes");
    };
    let Ok((before_key, _)) = abstract_system.state_key_of(before) else {
        return fault("mapped-state-encodes");
    };
    let stuttered = after_key == before_key;
    let passes = |taken| StepVerdict::Passes {
        post: RefinementPosition {
            concrete: step.post.state().clone(),
            abstract_states: vec![mapped.visible.clone()],
        },
        taken,
    };
    let Some(row) = row else {
        return stutter(step, stuttered, passes);
    };
    match &row.target {
        StepTarget::Stutter => stutter(step, stuttered, passes),
        StepTarget::Any => {
            if stuttered {
                return passes(Taken::Stutter);
            }
            let Ok(expansion) = abstract_system.expand_state(before) else {
                return fault("abstract-expansion");
            };
            let mut matched = Vec::new();
            for (transition, successor) in &expansion.successors {
                match abstract_system.state_key(successor) {
                    Ok((key, _)) if key == after_key => matched.push(transition.clone()),
                    Ok(_) => {}
                    Err(_) => return fault("successor-encodes"),
                }
            }
            *abstract_successors = abstract_successors
                .saturating_add(u64::try_from(expansion.successors.len()).unwrap_or(u64::MAX));
            if !matched.is_empty() {
                return match canonical(abstract_system, matched) {
                    Ok(matched) => passes(Taken::Abstract(matched)),
                    Err(verdict) => verdict,
                };
            }
            if expansion
                .findings
                .iter()
                .any(|finding| matches!(finding, ModelFinding::ContractUndetermined { .. }))
            {
                return StepVerdict::Undetermined(Undetermined::UndecidedSuccessor);
            }
            StepVerdict::Fails(RefinementFailure::NoAbstractMatch {
                position: step.position,
            })
        }
        StepTarget::Abstract { operation, .. } => {
            let Some(arguments) = arguments else {
                return fault("an-abstract-row-evaluates-its-arguments");
            };
            judge_operation(
                abstract_system,
                (before, &mapped),
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
    passes: impl FnOnce(Taken) -> StepVerdict,
) -> StepVerdict {
    if stuttered {
        passes(Taken::Stutter)
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
/// arguments (FR-141 "Abstract-operation evaluation").
#[allow(
    clippy::result_large_err,
    reason = "FR-141 fixes StepVerdict's shape: the verdict is also the early return of an inner step"
)]
fn evaluate_arguments(
    refinement: &CheckedRefinement,
    concrete_system: &ModelSystem<'_>,
    abstract_system: &ModelSystem<'_>,
    pre: &RefinementPosition,
    step: &ConcreteStep<'_>,
    (row, transition): (Option<&StepRow>, &ModelTransition),
) -> Result<Arguments, StepVerdict> {
    let Some(StepRow {
        class: StepClass::Operation(concrete_operation),
        target:
            StepTarget::Abstract {
                receiver,
                arguments,
                binds_result,
                ..
            },
    }) = row
    else {
        return Err(fault("an-abstract-row-names-an-operation"));
    };
    let Ok(resolved) = concrete_system.resolve_step(&pre.concrete, transition, step.post) else {
        return Err(fault("concrete-step-resolves"));
    };
    let Some(resolved) = resolved else {
        return Err(fault("concrete-step-names-an-argument-vector"));
    };
    let Ok(observed) = concrete_system.observe(&pre.concrete) else {
        return Err(fault("pre-state-observes"));
    };
    let mut bindings = vec![Value::Reference(resolved.receiver.clone())];
    bindings.extend(resolved.arguments.iter().cloned());
    let object = object_of(&pre.concrete, &resolved.receiver);
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
    let receiver = match evaluate(receiver, StepArgument::Receiver)? {
        Value::Reference(reference) => reference,
        _ => return Err(fault("a-receiver-row-is-a-reference")),
    };
    let mut supplied = Vec::with_capacity(arguments.len());
    for (index, argument) in arguments.iter().enumerate() {
        supplied.push(match argument {
            None => None,
            Some(expression) => Some(evaluate(expression, StepArgument::Parameter(index))?),
        });
    }
    Ok(Arguments {
        receiver,
        supplied,
        result: if *binds_result { resolved.result } else { None },
    })
}

/// RS-4: the abstract operation `operation` over the evaluated `arguments`,
/// judged against the mapped post-state with the calls the abstract system
/// makes when it expands the mapped pre-state.
fn judge_operation(
    abstract_system: &ModelSystem<'_>,
    (before, mapped): (&ModelState, &MappedState),
    operation: &DeclarationKey,
    arguments: Arguments,
    step: &ConcreteStep<'_>,
    abstract_successors: &mut u64,
    passes: impl FnOnce(Taken) -> StepVerdict,
) -> StepVerdict {
    let judged = match abstract_system.judge_application(&AbstractApplication {
        pre: before,
        receiver: &arguments.receiver,
        operation,
        arguments: &arguments.supplied,
        post: &mapped.visible,
        result: arguments.result.as_ref(),
    }) {
        Ok(judged) => judged,
        Err(_) => return fault("abstract-judgement"),
    };
    *abstract_successors =
        abstract_successors.saturating_add(u64::try_from(judged.len()).unwrap_or(u64::MAX));
    let mut keyed = Vec::with_capacity(judged.len());
    for found in judged {
        let Ok(key) = abstract_system.transition_key(&found.transition) else {
            return fault("transition-encodes");
        };
        keyed.push((key, found));
    }
    keyed.sort_by(|left, right| left.0.cmp(&right.0));
    let accepted: Vec<ModelTransition> = keyed
        .iter()
        .filter(|(_, found)| found.judgement == Judgement::Accepted)
        .map(|(_, found)| found.transition.clone())
        .collect();
    if !accepted.is_empty() {
        return passes(Taken::Abstract(accepted));
    }
    if keyed
        .iter()
        .any(|(_, found)| matches!(found.judgement, Judgement::Undecided(_)))
    {
        return StepVerdict::Undetermined(Undetermined::UndecidedSuccessor);
    }
    match keyed.into_iter().next() {
        Some((_, found)) => {
            let Judgement::Rejected(rejection) = found.judgement else {
                return fault("a-judgement-is-accepted-undecided-or-rejected");
            };
            StepVerdict::Fails(RefinementFailure::AbstractStepRejected {
                position: step.position,
                transition: found.transition,
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

/// The population and key of `reference` in `state`.
fn object_of(state: &ModelState, reference: &ObjectReference) -> Option<ObjectKey> {
    state.populations.iter().find_map(|(population, objects)| {
        objects
            .contains_key(reference.object().as_str())
            .then(|| ObjectKey {
                population: population.clone(),
                key: reference.object().as_str().to_owned(),
            })
    })
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
