// SPDX-License-Identifier: AGPL-3.0-or-later
//! The wire spelling of an `analyze` or `prove` item's counterexample
//! (FR-286 "Analyze items and their counterexamples"): FSpec FR-364's
//! `TemporalCounterexample` over a model subject, and canonical assignments.

use crate::{TaskProductTransition, TaskTransitionIdentity};
use filament_exact::{ObjectReference, Rational, Value};
use filament_semantic_value::fairness::FairnessConstraint;
use filament_semantic_value::model_transition::ModelTransition;
use fsl_eval::model_subject::ModelInstance;
use fsl_eval::simulation::protocol::{
    blocked_json, fairness_json, location_json, send_record_json, transition_json,
    ProtocolFairness, ProtocolTransition,
};
use fsl_eval::simulation::SubjectTransition;
use fsl_foundation::node_wire::node_key_to_json;
use fsl_foundation::source::provenance::OccurrenceKey;
use serde_json::{json, Value as Json};
use std::any::{Any, TypeId};

use super::{undefined_reason, TimedStateWire, TimedTransitionWire};
use crate::model_check::counterexample::{
    CounterexampleKind, ModelStep, Race, TemporalCounterexample, UndefinedEvaluation,
    UndefinedLocus,
};
use crate::refinement::{RefinementFailure, StepRejection};

/// Why a counterexample has no wire spelling here.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum CounterexampleWireError {
    /// The counterexample's member has no FR-286 spelling yet.
    #[error("the counterexample's {0} has no wire spelling")]
    Unspelled(&'static str),
}

/// Why a temporal counterexample document failed its typed FR-364 readback.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum TemporalCounterexampleReadRefusal {
    /// The supplied JSON does not have the canonical typed counterexample's
    /// shape or value at this pointer.
    #[error("malformed temporal counterexample ({cause}) at {pointer}")]
    MalformedWire {
        /// The FR-364 wire cause.
        cause: &'static str,
        /// The JSON pointer of the first disagreement.
        pointer: String,
    },
    /// A well-shaped transition belongs to another subject alphabet.
    #[error("temporal counterexample uses another trace alphabet at {pointer}")]
    TraceAlphabet {
        /// The transition type that selected the other alphabet.
        pointer: String,
    },
    /// A well-shaped task identity disagrees with the recomputed product.
    #[error("invalid task trace value at {pointer}: supplied {supplied}, recomputed {recomputed}")]
    InvalidRuntimeInput {
        /// The JSON pointer of the changed value.
        pointer: String,
        /// The supplied value.
        supplied: String,
        /// The value recomputed from the checked product.
        recomputed: String,
    },
}

/// Why a typed task-product counterexample could not be replayed against its
/// rebuilt product.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub enum TaskProductTraceReplayRefusal {
    /// The supplied typed counterexample uses the model transition alphabet.
    #[error("task-product replay received another trace alphabet at {pointer}")]
    TraceAlphabet {
        /// The first transition pointer.
        pointer: String,
    },
    /// A task trace step does not carry the required exact delay.
    #[error("task-product replay is missing a delay at {pointer}")]
    MissingDelay {
        /// The missing delay pointer.
        pointer: String,
    },
    /// The rebuilt product could not replay the typed transition sequence.
    #[error("task-product replay refused the transition sequence: {0}")]
    Product(#[from] fsl_eval::simulation::model::TaskProductReplayRefusal),
    /// The rebuilt product could not form a key for a replayed state.
    #[error("task-product replay could not key a replayed state: {0}")]
    StateKey(#[from] fsl_eval::simulation::model::TaskProductStateKeyRefusal),
    /// A recorded post-state differs from the state's recomputed key.
    #[error("task-product replay found a post-state mismatch at {pointer}")]
    InvalidRuntimeInput {
        /// The first differing JSON pointer.
        pointer: String,
        /// The supplied state key, in canonical hexadecimal form.
        supplied: String,
        /// The recomputed state key, in canonical hexadecimal form.
        recomputed: String,
    },
}

/// A losslessly read task-product FR-364 model trace and its optional
/// inductive queue-growth continuation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaskProductCounterexampleDocument {
    /// The typed source-plus-scheduler trace.
    pub counterexample: TemporalCounterexample<TaskProductTransition>,
    /// FSpec FR-364's queue-growth continuation, retained in its wire shape.
    pub queue_growth: Option<Json>,
}

/// Replay a typed temporal trace against a rebuilt task product.
///
/// The transition alphabet is checked before product expansion. Every delay,
/// enabled transition, post-state key and positive-time lasso closure is then
/// checked by the product's FR-237 replay path.
#[allow(private_bounds)] // Only FR-364 subject transitions with canonical spelling are accepted.
pub fn replay_task_product_trace<T: Spelling + Clone + 'static>(
    product: &fsl_eval::simulation::model::TaskProductSystem<'_, '_>,
    counterexample: &TemporalCounterexample<T>,
) -> Result<(), TaskProductTraceReplayRefusal> {
    if TypeId::of::<T>() != TypeId::of::<TaskProductTransition>() {
        return Err(TaskProductTraceReplayRefusal::TraceAlphabet {
            pointer: "/prefix/0/transition/type".to_owned(),
        });
    }
    let mut steps = Vec::with_capacity(counterexample.prefix.len());
    for (index, step) in counterexample.prefix.iter().enumerate() {
        let Some(delay) = step.delay.clone() else {
            return Err(TaskProductTraceReplayRefusal::MissingDelay {
                pointer: format!("/prefix/{index}/delay"),
            });
        };
        let Some(transition) = (&step.transition as &dyn Any)
            .downcast_ref::<TaskProductTransition>()
            .cloned()
        else {
            return Err(TaskProductTraceReplayRefusal::TraceAlphabet {
                pointer: format!("/prefix/{index}/transition/type"),
            });
        };
        steps.push(fsl_eval::simulation::model::TaskProductTraceStep { delay, transition });
    }
    if counterexample
        .lasso
        .as_ref()
        .is_some_and(|lasso| lasso.terminal_stutter)
    {
        let Some(delay) = counterexample
            .lasso
            .as_ref()
            .and_then(|lasso| lasso.terminal_stutter_delay.clone())
        else {
            return Err(TaskProductTraceReplayRefusal::MissingDelay {
                pointer: "/loop/0/delay".to_owned(),
            });
        };
        steps.push(fsl_eval::simulation::model::TaskProductTraceStep {
            delay,
            transition: TaskProductTransition::TerminalStutter,
        });
    }
    let trace = fsl_eval::simulation::model::TaskProductTrace {
        initial_state: counterexample.initial_state,
        steps,
        lasso_entry: counterexample.lasso.as_ref().map(|lasso| lasso.entry),
    };
    let entry = counterexample.lasso.as_ref().map(|lasso| lasso.entry);
    // A refusal names the step on the wire: steps from the lasso entry are
    // the `loop`, the rest are the `prefix` (FSpec FR-364).
    let step_pointer = |index: usize| match entry {
        Some(entry) if index >= entry => format!("/loop/{}", index - entry),
        _ => format!("/prefix/{index}"),
    };
    let mut key_refusal = None;
    let replayed = product.replay_trace_observed(&trace, |index, state| {
        let Some(step) = counterexample.prefix.get(index) else {
            return Ok(());
        };
        match product.state_key(state) {
            Ok(bytes) => {
                let recomputed = fsl_eval::simulation::StateKey::from_canonical_bytes(bytes);
                if step.post_state == recomputed {
                    return Ok(());
                }
                Err(
                    fsl_eval::simulation::model::TaskProductReplayRefusal::Member {
                        step: Some(index),
                        member: "/post_state".to_owned(),
                        supplied: step.post_state.hex(),
                        recomputed: recomputed.hex(),
                    },
                )
            }
            Err(error) => {
                key_refusal = Some(error);
                Err(fsl_eval::simulation::model::TaskProductReplayRefusal::Transition)
            }
        }
    });
    if let Some(error) = key_refusal {
        return Err(error.into());
    }
    match replayed {
        Ok(_) => Ok(()),
        Err(fsl_eval::simulation::model::TaskProductReplayRefusal::Member {
            step,
            member,
            supplied,
            recomputed,
        }) => Err(TaskProductTraceReplayRefusal::InvalidRuntimeInput {
            pointer: match step {
                Some(index) => format!("{}{member}", step_pointer(index)),
                None => member,
            },
            supplied,
            recomputed,
        }),
        Err(other) => Err(other.into()),
    }
}

/// Read an FSpec FR-364 model-arm counterexample document with step alphabet
/// `T`, `ModelTransition` or `TaskProductTransition` (FR-237 "Reading the
/// wire"). The reader takes no expected value: it reads exactly the FR-331
/// schema members, checks the whole payload's shape first, and returns a step
/// outside `T`'s alphabet as an alphabet refusal only for a payload with no
/// shape fault.
///
/// The model alphabet reads the formula-kind, unfair, untimed-interval subset
/// of the model arm that [`read_task_temporal_document`] reads.
#[allow(private_bounds)] // The reader accepts only subject types with sealed FR-364 spellings.
pub fn read_temporal_counterexample<T: Spelling + Clone + 'static>(
    document: &Json,
) -> Result<TemporalCounterexample<T>, TemporalCounterexampleReadRefusal> {
    let read = read_task_temporal_document(document)?;
    let task = read.counterexample;
    if TypeId::of::<T>() == TypeId::of::<TaskProductTransition>() {
        return (&task as &dyn Any)
            .downcast_ref::<TemporalCounterexample<T>>()
            .cloned()
            .ok_or_else(unsupported_alphabet);
    }
    if TypeId::of::<T>() != TypeId::of::<ModelTransition>() {
        return Err(unsupported_alphabet());
    }
    let entry = task.lasso.as_ref().map(|lasso| lasso.entry);
    let mut prefix = Vec::with_capacity(task.prefix.len());
    for (index, step) in task.prefix.iter().enumerate() {
        let TaskProductTransition::Source(transition) = &step.transition else {
            return Err(TemporalCounterexampleReadRefusal::TraceAlphabet {
                pointer: match entry {
                    Some(entry) if index >= entry => {
                        format!("/loop/{}/transition/type", index - entry)
                    }
                    _ => format!("/prefix/{index}/transition/type"),
                },
            });
        };
        prefix.push(ModelStep {
            transition: transition.clone(),
            post_state: step.post_state.clone(),
            delay: step.delay.clone(),
        });
    }
    let model = TemporalCounterexample::<ModelTransition> {
        initial_state: task.initial_state,
        prefix,
        over_binding: task.over_binding,
        fairness: task.fairness,
        kind: task.kind,
        lasso: task.lasso,
        blocked: task.blocked,
        refinement: task.refinement,
        final_delay: task.final_delay,
    };
    (&model as &dyn Any)
        .downcast_ref::<TemporalCounterexample<T>>()
        .cloned()
        .ok_or_else(unsupported_alphabet)
}

fn unsupported_alphabet() -> TemporalCounterexampleReadRefusal {
    TemporalCounterexampleReadRefusal::MalformedWire {
        cause: "unsupported-alphabet",
        pointer: "/".to_owned(),
    }
}

/// Read the task-product subset of an FR-364 model counterexample without an
/// independently supplied typed expectation. The rebuilt product remains
/// responsible for validating enabledness, derived identity and post-state.
pub fn read_task_temporal_document(
    document: &Json,
) -> Result<TaskProductCounterexampleDocument, TemporalCounterexampleReadRefusal> {
    use base64::Engine;
    use filament_exact::Integer;
    use fsl_eval::simulation::model::{TaskTransitionAction as Action, TaskTransitionIdentity};

    fn malformed(
        cause: &'static str,
        pointer: impl Into<String>,
    ) -> TemporalCounterexampleReadRefusal {
        TemporalCounterexampleReadRefusal::MalformedWire {
            cause,
            pointer: pointer.into(),
        }
    }
    fn object<'a>(
        value: &'a Json,
        pointer: &str,
    ) -> Result<&'a serde_json::Map<String, Json>, TemporalCounterexampleReadRefusal> {
        value
            .as_object()
            .ok_or_else(|| malformed("wrong-value-kind", pointer))
    }
    fn required<'a>(
        value: &'a Json,
        key: &str,
        pointer: &str,
    ) -> Result<&'a Json, TemporalCounterexampleReadRefusal> {
        value
            .get(key)
            .ok_or_else(|| malformed("missing-member", format!("{pointer}/{key}")))
    }
    fn exact_members(
        value: &Json,
        required_names: &[&str],
        optional_names: &[&str],
        pointer: &str,
    ) -> Result<(), TemporalCounterexampleReadRefusal> {
        let found = object(value, pointer)?;
        for name in required_names {
            if !found.contains_key(*name) {
                return Err(malformed("missing-member", format!("{pointer}/{name}")));
            }
        }
        if let Some(name) = found.keys().find(|name| {
            !required_names.contains(&name.as_str()) && !optional_names.contains(&name.as_str())
        }) {
            return Err(malformed("unknown-member", format!("{pointer}/{name}")));
        }
        Ok(())
    }
    fn rational(
        value: &Json,
        pointer: &str,
    ) -> Result<Rational, TemporalCounterexampleReadRefusal> {
        exact_members(value, &["numerator", "denominator"], &[], pointer)?;
        let integer = |member: &str| -> Result<Integer, TemporalCounterexampleReadRefusal> {
            required(value, member, pointer)?
                .as_str()
                .ok_or_else(|| malformed("wrong-value-kind", format!("{pointer}/{member}")))?
                .parse()
                .map_err(|_| malformed("invalid-value", format!("{pointer}/{member}")))
        };
        Rational::new(integer("numerator")?, integer("denominator")?)
            .map_err(|_| malformed("invalid-value", pointer))
    }
    fn source_transition(
        value: &Json,
        pointer: &str,
    ) -> Result<ModelTransition, TemporalCounterexampleReadRefusal> {
        exact_members(
            value,
            &["type", "operation", "receiver", "arguments"],
            &[],
            pointer,
        )?;
        if required(value, "type", pointer)?.as_str() != Some("transition") {
            return Err(TemporalCounterexampleReadRefusal::TraceAlphabet {
                pointer: format!("{pointer}/type"),
            });
        }
        let operation_wire = required(value, "operation", pointer)?;
        // FR-364 carries source operations as their stable node URI. Recover
        // the canonical node/package identity used by ModelTransition. Other
        // canonical operation identities, including a model arm's structured
        // operation value, are preserved verbatim.
        let operation_value = match operation_wire {
            Json::String(operation) => match serde_json::from_str::<Json>(operation) {
                Ok(value) => value,
                Err(_) if operation.starts_with("ix://") => {
                    let mut parts = operation
                        .strip_prefix("ix://")
                        .unwrap_or_default()
                        .split('/');
                    let owner = parts.next().filter(|part| !part.is_empty());
                    let repository = parts.next().filter(|part| !part.is_empty());
                    let package = owner
                        .zip(repository)
                        .map(|(owner, repository)| format!("{owner}/{repository}"))
                        .ok_or_else(|| {
                            malformed("invalid-value", format!("{pointer}/operation"))
                        })?;
                    json!({"node": operation, "package": package})
                }
                Err(_) => Json::String(operation.clone()),
            },
            Json::Object(_) => operation_wire.clone(),
            _ => {
                return Err(malformed(
                    "wrong-value-kind",
                    format!("{pointer}/operation"),
                ))
            }
        };
        let receiver = match required(value, "receiver", pointer)? {
            Json::Null => None,
            value => Some(
                serde_json::to_string(value)
                    .map_err(|_| malformed("invalid-value", format!("{pointer}/receiver")))?,
            ),
        };
        let arguments = required(value, "arguments", pointer)?
            .as_array()
            .ok_or_else(|| malformed("wrong-value-kind", format!("{pointer}/arguments")))?
            .iter()
            .enumerate()
            .map(|(index, value)| {
                serde_json::to_string(value)
                    .map_err(|_| malformed("invalid-value", format!("{pointer}/arguments/{index}")))
            })
            .collect::<Result<Vec<_>, _>>()?;
        ModelTransition::new(
            serde_json::to_string(&operation_value)
                .map_err(|_| malformed("invalid-value", format!("{pointer}/operation")))?,
            receiver,
            arguments,
            quire_canonical::Limits::new(u64::MAX),
        )
        .map_err(|_| malformed("invalid-value", pointer))
    }
    fn task_transition(
        value: &Json,
        pointer: &str,
    ) -> Result<TaskTransitionIdentity, TemporalCounterexampleReadRefusal> {
        let action_text = required(value, "action", pointer)?
            .as_str()
            .ok_or_else(|| malformed("wrong-value-kind", format!("{pointer}/action")))?;
        let action = match action_text {
            "activate" => Action::Activate,
            "release" => Action::Release,
            "dispatch" => Action::Dispatch,
            "preempt" => Action::Preempt,
            "resume" => Action::Resume,
            "complete" => Action::Complete,
            "budget-change" => Action::BudgetChange,
            "deadline-miss" => Action::DeadlineMiss,
            _ => return Err(malformed("unknown-variant", format!("{pointer}/action"))),
        };
        let budget_present = value.get("budget_delta").is_some();
        if budget_present != (action == Action::BudgetChange) {
            return Err(malformed(
                "budget-delta-presence",
                format!("{pointer}/budget_delta"),
            ));
        }
        let required_names = [
            "type",
            "action",
            "taskset",
            "task",
            "job",
            "slot",
            "generation",
            "release",
            "source_transition",
            "demand",
            "jitter",
            "deadline",
        ];
        let optional_names = ["priority", "budget_delta"];
        exact_members(value, &required_names, &optional_names, pointer)?;
        if required(value, "type", pointer)?.as_str() != Some("task-transition") {
            return Err(TemporalCounterexampleReadRefusal::TraceAlphabet {
                pointer: format!("{pointer}/type"),
            });
        }
        let string = |member: &str| -> Result<String, TemporalCounterexampleReadRefusal> {
            required(value, member, pointer)?
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| malformed("wrong-value-kind", format!("{pointer}/{member}")))
        };
        let integer = |member: &str| -> Result<u64, TemporalCounterexampleReadRefusal> {
            required(value, member, pointer)?
                .as_u64()
                .ok_or_else(|| malformed("wrong-value-kind", format!("{pointer}/{member}")))
        };
        let optional_integer = match value.get("priority") {
            None => None,
            Some(value) => Some(
                value
                    .as_u64()
                    .ok_or_else(|| malformed("wrong-value-kind", format!("{pointer}/priority")))?,
            ),
        };
        let source_optional = match required(value, "source_transition", pointer)? {
            Json::Null => None,
            source => Some(source_transition(
                source,
                &format!("{pointer}/source_transition"),
            )?),
        };
        Ok(TaskTransitionIdentity {
            action,
            task_set: string("taskset")?,
            task: string("task")?,
            job: integer("job")?,
            slot: integer("slot")?,
            generation: integer("generation")?,
            release: source_transition(
                required(value, "release", pointer)?,
                &format!("{pointer}/release"),
            )?,
            source_transition: source_optional,
            demand: rational(
                required(value, "demand", pointer)?,
                &format!("{pointer}/demand"),
            )?,
            jitter: rational(
                required(value, "jitter", pointer)?,
                &format!("{pointer}/jitter"),
            )?,
            deadline: rational(
                required(value, "deadline", pointer)?,
                &format!("{pointer}/deadline"),
            )?,
            priority: optional_integer,
            budget_delta: value
                .get("budget_delta")
                .map(|value| rational(value, &format!("{pointer}/budget_delta")))
                .transpose()?,
        })
    }
    fn check_step_shape(
        value: &Json,
        pointer: &str,
    ) -> Result<(), TemporalCounterexampleReadRefusal> {
        exact_members(value, &["transition", "post_state"], &["delay"], pointer)?;
        let transition_pointer = format!("{pointer}/transition");
        let transition = required(value, "transition", pointer)?;
        let kind = required(transition, "type", &transition_pointer)?
            .as_str()
            .ok_or_else(|| malformed("wrong-value-kind", format!("{transition_pointer}/type")))?;
        match kind {
            "transition" => exact_members(
                transition,
                &["type", "operation", "receiver", "arguments"],
                &[],
                &transition_pointer,
            ),
            "task-transition" => exact_members(
                transition,
                &[
                    "type",
                    "action",
                    "taskset",
                    "task",
                    "job",
                    "slot",
                    "generation",
                    "release",
                    "source_transition",
                    "demand",
                    "jitter",
                    "deadline",
                ],
                &["priority", "budget_delta"],
                &transition_pointer,
            ),
            "terminal-stutter" => exact_members(transition, &["type"], &[], &transition_pointer),
            _ => {
                // Validate the remainder before returning the alphabet refusal.
                exact_members(transition, &["type"], &[], &transition_pointer)
            }
        }?;
        let state = required(value, "post_state", pointer)?;
        exact_members(state, &["key"], &[], &format!("{pointer}/post_state"))?;
        if required(state, "key", &format!("{pointer}/post_state"))?
            .as_str()
            .is_none()
        {
            return Err(malformed(
                "wrong-value-kind",
                format!("{pointer}/post_state/key"),
            ));
        }
        // A source step of the model alphabet may omit its delay; a task step
        // and the terminal stutter carry theirs (FR-237 "Reading the wire").
        if kind == "transition" && value.get("delay").is_none() {
            return Ok(());
        }
        let _ = rational(
            required(value, "delay", pointer)?,
            &format!("{pointer}/delay"),
        )?;
        Ok(())
    }
    fn parse_step(
        value: &Json,
        pointer: &str,
    ) -> Result<ModelStep<TaskProductTransition>, TemporalCounterexampleReadRefusal> {
        let transition = required(value, "transition", pointer)?;
        let transition_pointer = format!("{pointer}/transition");
        let kind = required(transition, "type", &transition_pointer)?
            .as_str()
            .unwrap_or_default();
        let transition = match kind {
            "transition" => {
                TaskProductTransition::Source(source_transition(transition, &transition_pointer)?)
            }
            "task-transition" => TaskProductTransition::Task(Box::new(task_transition(
                transition,
                &transition_pointer,
            )?)),
            "terminal-stutter" => TaskProductTransition::TerminalStutter,
            _ => {
                return Err(TemporalCounterexampleReadRefusal::TraceAlphabet {
                    pointer: format!("{transition_pointer}/type"),
                })
            }
        };
        let post_state = required(
            required(value, "post_state", pointer)?,
            "key",
            &format!("{pointer}/post_state"),
        )?
        .as_str()
        .ok_or_else(|| malformed("wrong-value-kind", format!("{pointer}/post_state/key")))?;
        let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(post_state)
            .map_err(|_| malformed("invalid-value", format!("{pointer}/post_state/key")))?;
        Ok(ModelStep {
            transition,
            post_state: fsl_eval::simulation::StateKey::from_canonical_bytes(bytes),
            delay: value
                .get("delay")
                .map(|delay| rational(delay, &format!("{pointer}/delay")))
                .transpose()?,
        })
    }

    exact_members(
        document,
        &[
            "trace",
            "kind",
            "initial_state",
            "prefix",
            "loop",
            "over_binding",
            "fairness",
            "interval",
            "trace_position",
        ],
        &["final_delay", "queue_growth"],
        "",
    )?;
    if required(document, "trace", "")?.as_str() != Some("model")
        || required(document, "kind", "")?.as_str() != Some("formula")
        || required(document, "over_binding", "")? != &Json::Null
        || required(document, "fairness", "")?
            .as_array()
            .is_none_or(|items| !items.is_empty())
        || required(document, "interval", "")? != &Json::Null
        || document.get("final_delay").is_some()
    {
        return Err(malformed("invalid-value", "/kind"));
    }
    let prefix = required(document, "prefix", "")?
        .as_array()
        .ok_or_else(|| malformed("wrong-value-kind", "/prefix"))?;
    let looped = required(document, "loop", "")?
        .as_array()
        .ok_or_else(|| malformed("wrong-value-kind", "/loop"))?;
    let queue_growth = document.get("queue_growth").cloned();
    // FSpec FR-364 (FR-364-AC-17): a task counterexample whose prefix holds a
    // deadline miss carries exactly one continuation, a non-empty `loop` or a
    // `queue_growth` witness; a bare prefix is refused before replay. A trace
    // with no task miss is the plain finite model-arm shape.
    let holds_miss = prefix.iter().any(|step| {
        let identity = &step["transition"];
        identity["type"] == "task-transition" && identity["action"] == "deadline-miss"
    });
    if holds_miss && looped.is_empty() && queue_growth.is_none() {
        return Err(malformed("missing-member", "/loop"));
    }
    if !looped.is_empty() && queue_growth.is_some() {
        return Err(malformed("invalid-value", "/queue_growth"));
    }
    if queue_growth.is_some() && !holds_miss {
        return Err(malformed("invalid-value", "/queue_growth"));
    }
    if let Some(witness) = &queue_growth {
        if !valid_queue_growth_envelope(witness) {
            return Err(TemporalCounterexampleReadRefusal::InvalidRuntimeInput {
                pointer: "/queue_growth".to_owned(),
                supplied: witness.to_string(),
                recomputed: "a well-formed queue-growth continuation".to_owned(),
            });
        }
    }
    // Shape-check the entire payload before classifying a foreign transition
    // alphabet, so a later malformed member wins over an earlier type.
    for (index, step) in prefix.iter().enumerate() {
        check_step_shape(step, &format!("/prefix/{index}"))?;
    }
    for (index, step) in looped.iter().enumerate() {
        check_step_shape(step, &format!("/loop/{index}"))?;
    }
    let initial_state = required(document, "initial_state", "")?
        .as_u64()
        .and_then(|value| usize::try_from(value).ok())
        .ok_or_else(|| malformed("wrong-value-kind", "/initial_state"))?;
    let mut steps = Vec::with_capacity(prefix.len() + looped.len());
    let mut alphabet_refusal = None;
    for (index, step) in prefix.iter().enumerate() {
        match parse_step(step, &format!("/prefix/{index}")) {
            Ok(step) => steps.push(step),
            Err(error @ TemporalCounterexampleReadRefusal::TraceAlphabet { .. }) => {
                alphabet_refusal.get_or_insert(error);
            }
            Err(error) => return Err(error),
        }
    }
    let entry = steps.len();
    let terminal_stutter =
        looped.len() == 1 && looped[0]["transition"]["type"].as_str() == Some("terminal-stutter");
    // The terminal-stutter step is never part of the read prefix: replay
    // appends it from `Lasso::terminal_stutter_delay` (FR-249-AC-11).
    let mut stutter_delay = None;
    for (index, step) in looped.iter().enumerate() {
        match parse_step(step, &format!("/loop/{index}")) {
            Ok(step) if terminal_stutter => stutter_delay = step.delay,
            Ok(step) => steps.push(step),
            Err(error @ TemporalCounterexampleReadRefusal::TraceAlphabet { .. }) => {
                alphabet_refusal.get_or_insert(error);
            }
            Err(error) => return Err(error),
        }
    }
    if let Some(error) = alphabet_refusal {
        return Err(error);
    }
    if looped
        .iter()
        .any(|step| step["transition"]["type"] == "terminal-stutter")
        && !terminal_stutter
    {
        return Err(malformed("invalid-value", "/loop/0/transition/type"));
    }
    let lasso = (!looped.is_empty()).then(|| crate::Lasso {
        entry,
        terminal_stutter,
        terminal_stutter_delay: stutter_delay,
    });
    let counterexample = TemporalCounterexample {
        initial_state,
        prefix: steps,
        over_binding: None,
        fairness: Vec::new(),
        kind: CounterexampleKind::Formula,
        lasso,
        blocked: Vec::new(),
        refinement: None,
        final_delay: None,
    };
    let position = required(document, "trace_position", "")?;
    let expected_position = if queue_growth.is_some() {
        task_miss_position(&counterexample).map(Json::from)
    } else {
        counterexample.trace_position().map(Json::from)
    };
    if expected_position.as_ref() != Some(position) {
        return Err(malformed("invalid-value", "/trace_position"));
    }
    if queue_growth.is_some()
        && task_miss_position(&counterexample)
            != counterexample
                .prefix
                .len()
                .checked_sub(1)
                .and_then(|index| u64::try_from(index).ok())
    {
        return Err(TemporalCounterexampleReadRefusal::InvalidRuntimeInput {
            pointer: "/trace_position".to_owned(),
            supplied: position.to_string(),
            recomputed: "the final deadline-miss step".to_owned(),
        });
    }
    Ok(TaskProductCounterexampleDocument {
        counterexample,
        queue_growth,
    })
}

/// Read the typed task trace when callers do not need to retain a queue-growth
/// continuation value.
pub fn read_task_temporal_counterexample(
    document: &Json,
) -> Result<TemporalCounterexample<TaskProductTransition>, TemporalCounterexampleReadRefusal> {
    read_task_temporal_document(document).map(|read| read.counterexample)
}

fn rational(value: &Rational) -> Json {
    json!({
        "numerator": value.numerator().to_string(),
        "denominator": value.denominator().to_string(),
    })
}

fn state(key: &fsl_eval::simulation::StateKey) -> Result<Json, CounterexampleWireError> {
    serde_json::to_value(TimedStateWire::from(key))
        .map_err(|_| CounterexampleWireError::Unspelled("state key"))
}

fn model_transition(identity: &ModelTransition) -> Result<Json, CounterexampleWireError> {
    serde_json::to_value(TimedTransitionWire::from(identity))
        .map_err(|_| CounterexampleWireError::Unspelled("transition"))
}

fn canonical<T: quire_canonical::Encode + ?Sized>(
    value: &T,
    member: &'static str,
) -> Result<Json, CounterexampleWireError> {
    fsl_eval::simulation::model::canonical_json(value)
        .ok_or(CounterexampleWireError::Unspelled(member))
}

/// How a subject's steps, send records and fairness members are written.
trait Spelling: SubjectTransition {
    fn transition(&self) -> Result<Json, CounterexampleWireError>;
    fn send_record(&self) -> Result<Option<Json>, CounterexampleWireError>;
    fn fairness(member: &Self::Fairness) -> Result<Json, CounterexampleWireError>;

    fn terminal_stutter_delay_required() -> bool {
        false
    }
}

impl Spelling for ModelTransition {
    fn transition(&self) -> Result<Json, CounterexampleWireError> {
        model_transition(self)
    }

    fn send_record(&self) -> Result<Option<Json>, CounterexampleWireError> {
        Ok(None)
    }

    fn fairness(member: &FairnessConstraint) -> Result<Json, CounterexampleWireError> {
        canonical(member, "fairness")
    }
}

impl Spelling for TaskTransitionIdentity {
    fn transition(&self) -> Result<Json, CounterexampleWireError> {
        task_transition_wire(self)
    }

    fn send_record(&self) -> Result<Option<Json>, CounterexampleWireError> {
        Ok(None)
    }

    fn fairness(member: &FairnessConstraint) -> Result<Json, CounterexampleWireError> {
        canonical(member, "fairness")
    }
}

impl Spelling for TaskProductTransition {
    fn transition(&self) -> Result<Json, CounterexampleWireError> {
        match self {
            Self::Source(transition) => task_source_transition_wire(transition),
            Self::Task(transition) => transition.transition(),
            Self::TerminalStutter => Ok(json!({"type": "terminal-stutter"})),
        }
    }

    fn send_record(&self) -> Result<Option<Json>, CounterexampleWireError> {
        Ok(None)
    }

    fn fairness(member: &FairnessConstraint) -> Result<Json, CounterexampleWireError> {
        canonical(member, "fairness")
    }

    fn terminal_stutter_delay_required() -> bool {
        true
    }
}

impl Spelling for ProtocolTransition {
    fn transition(&self) -> Result<Json, CounterexampleWireError> {
        transition_json(self).ok_or(CounterexampleWireError::Unspelled("protocol transition"))
    }

    fn send_record(&self) -> Result<Option<Json>, CounterexampleWireError> {
        ProtocolTransition::send_record(self)
            .map(|record| {
                send_record_json(record).ok_or(CounterexampleWireError::Unspelled("send record"))
            })
            .transpose()
    }

    fn fairness(member: &ProtocolFairness) -> Result<Json, CounterexampleWireError> {
        fairness_json(member).ok_or(CounterexampleWireError::Unspelled("protocol fairness"))
    }
}

fn step<T: Spelling>(step: &ModelStep<T>) -> Result<Json, CounterexampleWireError> {
    let mut written = json!({
        "transition": step.transition.transition()?,
        "post_state": state(&step.post_state)?,
    });
    if let Some(record) = step.transition.send_record()? {
        written["send_record"] = record;
    }
    if let Some(delay) = &step.delay {
        written["delay"] = rational(delay);
    }
    Ok(written)
}

fn reference(reference: &ObjectReference) -> Result<Json, CounterexampleWireError> {
    fsl_eval::simulation::model::value_json(&Value::Reference(reference.clone()))
        .ok_or(CounterexampleWireError::Unspelled("object reference"))
}

/// The `{node, occurrence}` of the item's claim. The engines keep an
/// undefined expression as a formula atom, capture, contract clause,
/// terminal predicate, constraint, protocol step or mapping row, none of which
/// holds a node of its own, so the claim that contains it is the locus.
fn locus(claim: Option<&OccurrenceKey>) -> Result<Json, CounterexampleWireError> {
    let claim = claim.ok_or(CounterexampleWireError::Unspelled("undefined locus"))?;
    Ok(json!({
        "node": node_key_to_json(claim.node())
            .map_err(|_| CounterexampleWireError::Unspelled("undefined locus"))?,
        "occurrence": {
            "role": claim.origin().role().to_string(),
            "ordinal": claim.origin().ordinal(),
        },
    }))
}

fn undefined(
    evaluation: &UndefinedEvaluation,
    claim: Option<&OccurrenceKey>,
) -> Result<Json, CounterexampleWireError> {
    // The place and clock valuation of a timed site follow from the prefix's
    // delays, and FR-364's `where` has no member for them.
    let site = &evaluation.site;
    let mut at = json!({
        "position": site.position,
        "state_key": state(&site.state_key)?,
        "locus": locus(claim)?,
    });
    if let Some(guard) = &site.transition {
        at["transition"] = model_transition(guard)?;
    }
    if let UndefinedLocus::Mapping(mapping) = &site.locus {
        at["row"] = json!(mapping_row(&mapping.row)?);
    }
    Ok(json!({
        "type": "undefined-evaluation",
        "where": at,
        "cause": undefined_reason(&evaluation.cause),
    }))
}

/// The FR-410 `where` of an undefined sample: its test, trace, position and
/// state. FR-364's `locus` is not here: the verdict does not hold the claim.
pub(super) fn sampled_undefined_wire(
    undefined: &crate::statistical::SampledUndefined,
) -> Result<Json, CounterexampleWireError> {
    let site = &undefined.evaluation.site;
    Ok(json!({
        "type": "undefined-evaluation",
        "where": {
            "position": site.position,
            "state_key": state(&site.state_key)?,
            "test": undefined.test,
            "trace_index": undefined.trace.to_string(),
        },
        "cause": undefined_reason(&undefined.evaluation.cause),
    }))
}

/// A `binding` member (FSpec FR-410): the typed reference, or `null`.
pub(super) fn binding_wire(
    binding: Option<&ObjectReference>,
) -> Result<Json, CounterexampleWireError> {
    binding.map_or(Ok(Json::Null), reference)
}

/// Why `witness` cannot be written to a verdict's `witnesses`: the first
/// member with no schema-valid spelling.
pub(super) fn witness_omission(
    witness: &crate::statistical::SampledWitness,
) -> super::WitnessOmission {
    use crate::statistical::{MeasureValue, WitnessValue};
    use fsl_eval::simulation::model::TransitionId;
    if matches!(witness.value, WitnessValue::Measure(MeasureValue::Censored)) {
        return super::WitnessOmission::CensoredMeasure;
    }
    if witness
        .steps
        .iter()
        .any(|step| matches!(step.transition, TransitionId::Stutter))
    {
        return super::WitnessOmission::TerminalStutterStep;
    }
    super::WitnessOmission::ScheduledIdentityNotSpellable
}

/// One sampled witness (FSpec FR-410 "Sampled witness"), its drawn values
/// spelled with the package's declarations. The schema requires each step's
/// `scheduled` as a `TypedValue` that cannot hold the scheduled identity, so
/// no witness is spelled yet: the first member with no spelling is the error.
///
/// # Errors
///
/// [`super::WitnessOmission`] naming why the witness is left out.
pub fn sampled_witness_wire(
    witness: &crate::statistical::SampledWitness,
    types: &filament_semantic_value::declaration::TypeEnvironment,
    variants: &[fsl_semantics::check::VariantTypeDeclaration],
) -> Result<Json, super::WitnessOmission> {
    use crate::statistical::{MeasureValue, WitnessValue};
    use fsl_eval::simulation::model::TransitionId;
    let omitted = |_| witness_omission(witness);
    let _value = match &witness.value {
        WitnessValue::Event(holds) => json!({"type": "event", "value": holds}),
        WitnessValue::Fraction(fraction) | WitnessValue::Measure(MeasureValue::Value(fraction)) => {
            json!({"type": "measure", "value": {"value": rational(fraction)}})
        }
        WitnessValue::Measure(MeasureValue::Censored) => return Err(witness_omission(witness)),
        WitnessValue::Undefined(evaluation) => {
            sampled_undefined_wire(&crate::statistical::SampledUndefined {
                test: witness.test,
                trace: witness.trace,
                evaluation: evaluation.clone(),
            })
            .map_err(omitted)?
        }
    };
    let mut steps = Vec::with_capacity(witness.steps.len());
    for step in &witness.steps {
        let TransitionId::Operation(transition) = &step.transition else {
            return Err(witness_omission(witness));
        };
        let mut random = Vec::with_capacity(step.random.len());
        for drawn in &step.random {
            random.push(
                fsl_eval::simulation::model::typed_value_json(drawn, types, variants)
                    .ok_or_else(|| witness_omission(witness))?,
            );
        }
        let mut written = json!({
            "transition": model_transition(transition).map_err(omitted)?,
            "post_state": state(&step.post_state).map_err(omitted)?,
            "random": random,
        });
        if let Some(delay) = &step.delay {
            written["delay"] = rational(delay);
        }
        steps.push(written);
    }
    // `scheduled` has no spelling, so no step is complete.
    let _ = steps;
    Err(super::WitnessOmission::ScheduledIdentityNotSpellable)
}

fn race(race: &Race) -> Result<Json, CounterexampleWireError> {
    Ok(json!({
        "location": location_json(&race.location)
            .ok_or(CounterexampleWireError::Unspelled("race location"))?,
        "earlier": race.earlier.to_string(),
        "later": race.later.to_string(),
    }))
}

/// The mapping row an `Undefined` failure names (FSpec FR-379): its kind and
/// the declarations that identify it, as one string.
fn mapping_row(row: &crate::refinement::MappingRowRef) -> Result<String, CounterexampleWireError> {
    use crate::refinement::{MappingRowRef, StepArgument};
    let key = |key: &filament_checked::key::DeclarationKey| {
        fsl_eval::simulation::model::declaration_identity_text(key)
            .ok_or(CounterexampleWireError::Unspelled("mapping row"))
    };
    Ok(match row {
        MappingRowRef::Field { object_type, field } => {
            format!("field {} {}", key(object_type)?, key(field)?)
        }
        MappingRowRef::Object { concrete_type } => format!("object {}", key(concrete_type)?),
        MappingRowRef::Argument {
            operation,
            argument,
        } => match argument {
            StepArgument::Receiver => format!("argument {} receiver", key(operation)?),
            StepArgument::Parameter(position) => {
                format!("argument {} parameter {position}", key(operation)?)
            }
        },
        MappingRowRef::History {
            object_type,
            field,
            operation,
        } => format!("history {} {field} {}", key(object_type)?, key(operation)?),
    })
}

/// FSpec FR-379's `refinement_failure` member.
fn refinement(failure: &RefinementFailure) -> Result<Json, CounterexampleWireError> {
    Ok(match failure {
        RefinementFailure::InitialNotAbstract { initial } => {
            json!({"kind": "InitialNotAbstract", "initial": initial})
        }
        RefinementFailure::StutterChanged { position } => {
            json!({"kind": "StutterChanged", "position": position})
        }
        RefinementFailure::NoAbstractMatch { position } => {
            json!({"kind": "NoAbstractMatch", "position": position})
        }
        RefinementFailure::AbstractClockGuard {
            position,
            operation,
        } => json!({
            "kind": "AbstractClockGuard",
            "position": position,
            "operation": operation,
        }),
        RefinementFailure::AbstractTimeInvariant { position, model } => json!({
            "kind": "AbstractTimeInvariant",
            "position": position,
            "model": model,
        }),
        RefinementFailure::AbstractStepRejected {
            position,
            transition,
            cause,
        } => json!({
            "kind": "AbstractStepRejected",
            "position": position,
            "transition": model_transition(transition)?,
            "cause": match cause {
                StepRejection::Precondition { clause } => {
                    json!({"type": "Precondition", "clause": clause})
                }
                StepRejection::Frame { code } => json!({"type": "Frame", "code": code.code()}),
                StepRejection::Postcondition => json!({"type": "Postcondition"}),
            },
        }),
        RefinementFailure::Divergence { constraint, .. } => {
            let constraint = constraint
                .as_ref()
                .ok_or(CounterexampleWireError::Unspelled("divergence constraint"))?;
            json!({"kind": "Divergence", "constraint": canonical(constraint, "fairness")?})
        }
        RefinementFailure::AbstractUnfair { constraint } => {
            json!({"kind": "AbstractUnfair", "constraint": canonical(constraint, "fairness")?})
        }
        RefinementFailure::HiddenUnfair { constraint } => {
            json!({"kind": "HiddenUnfair", "constraint": canonical(constraint, "fairness")?})
        }
        RefinementFailure::RefusalNotPermitted { position, label } => json!({
            "kind": "RefusalNotPermitted",
            "position": position,
            "label": serde_json::to_value(label)
                .map_err(|_| CounterexampleWireError::Unspelled("observation label"))?,
        }),
        RefinementFailure::TerminalSuccessMismatch {
            position,
            direction,
        } => json!({
            "kind": "TerminalSuccessMismatch",
            "position": position,
            "direction": serde_json::to_value(direction)
                .map_err(|_| CounterexampleWireError::Unspelled("terminal success direction"))?,
        }),
        RefinementFailure::Undefined { position, row } => json!({
            "kind": "Undefined",
            "position": position,
            "row": mapping_row(row)?,
        }),
    })
}

fn temporal<T: Spelling>(
    counterexample: &TemporalCounterexample<T>,
    over: Option<&ObjectReference>,
    claim: Option<&OccurrenceKey>,
) -> Result<Json, CounterexampleWireError> {
    let mut race_member = None;
    let mut loop_proof_member = None;
    let kind = match &counterexample.kind {
        CounterexampleKind::Formula => json!("formula"),
        CounterexampleKind::Deadlock => json!("deadlock"),
        CounterexampleKind::TimeLock => json!("time-lock"),
        CounterexampleKind::UndefinedEvaluation(evaluation) => undefined(evaluation, claim)?,
        CounterexampleKind::Race(found) => {
            race_member = Some(race(found)?);
            json!("race")
        }
        CounterexampleKind::LoopProof(found) => {
            loop_proof_member = Some(json!({
                "repeat": found.repeat,
                "obligation": match found.obligation {
                    crate::LoopProofObligation::Invariant => "loop-invariant",
                    crate::LoopProofObligation::Termination => "loop-termination",
                },
                "failure": match found.failure {
                    crate::LoopProofFailure::InvariantFalse => "invariant-false",
                    crate::LoopProofFailure::Undefined => "undefined",
                    crate::LoopProofFailure::VariantDidNotDecrease => "variant-did-not-decrease",
                    crate::LoopProofFailure::VariantBelowFloor => "variant-below-floor",
                }
            }));
            json!("loop-proof")
        }
    };
    let steps = counterexample
        .prefix
        .iter()
        .map(step::<T>)
        .collect::<Result<Vec<_>, _>>()?;
    let (stem, mut looped) = match counterexample.lasso.as_ref() {
        Some(lasso) => {
            let split = lasso.entry.min(steps.len());
            let mut stem = steps;
            let looped = stem.split_off(split);
            (stem, looped)
        }
        None => (steps, Vec::new()),
    };
    if counterexample
        .lasso
        .as_ref()
        .is_some_and(|lasso| lasso.terminal_stutter)
    {
        let last = counterexample
            .prefix
            .last()
            .ok_or(CounterexampleWireError::Unspelled(
                "initial terminal stutter",
            ))?;
        let mut stutter = json!({
            "transition": {"type": "terminal-stutter"},
            "post_state": state(&last.post_state)?,
        });
        match counterexample
            .lasso
            .as_ref()
            .and_then(|lasso| lasso.terminal_stutter_delay.as_ref())
        {
            Some(delay) if delay > &zero_rational() => stutter["delay"] = rational(delay),
            Some(_) => {
                return Err(CounterexampleWireError::Unspelled(
                    "non-positive terminal stutter delay",
                ));
            }
            None if T::terminal_stutter_delay_required() => {
                return Err(CounterexampleWireError::Unspelled(
                    "missing terminal stutter delay",
                ));
            }
            None => {}
        }
        looped.push(stutter);
    }
    let over_binding = match (&counterexample.over_binding, over) {
        (None, _) => Json::Null,
        (Some(_), Some(found)) => reference(found)?,
        (Some(ModelInstance { .. }), None) => {
            return Err(CounterexampleWireError::Unspelled("over binding"))
        }
    };
    let fairness = counterexample
        .fairness
        .iter()
        .map(T::fairness)
        .collect::<Result<Vec<Json>, _>>()?;
    let mut written = json!({
        "trace": "model",
        "kind": kind,
        "initial_state": counterexample.initial_state,
        "prefix": stem,
        "loop": looped,
        "over_binding": over_binding,
        "fairness": fairness,
        "interval": Json::Null,
        "trace_position": counterexample.trace_position(),
    });
    if let Some(delay) = &counterexample.final_delay {
        written["final_delay"] = rational(delay);
    }
    if let Some(found) = race_member {
        written["race"] = found;
    }
    if let Some(found) = loop_proof_member {
        written["loop_proof"] = found;
    }
    if let Some(failure) = &counterexample.refinement {
        written["refinement_failure"] = refinement(failure)?;
    }
    Ok(written)
}

/// FSpec FR-364's wire form of a model-arm counterexample. `over` is the
/// reference the clause's `over` parameter is bound to, which only the
/// subject can name, and `claim` the item's claim, the locus of an undefined
/// evaluation.
///
/// # Errors
///
/// [`CounterexampleWireError`] for a member FR-364's table does not spell, a
/// terminal stutter at an initial state, an `over` binding given no
/// reference, or an undefined evaluation given no claim.
pub fn temporal_wire(
    counterexample: &TemporalCounterexample,
    over: Option<&ObjectReference>,
    claim: Option<&OccurrenceKey>,
) -> Result<Json, CounterexampleWireError> {
    temporal(counterexample, over, claim)
}

/// The FSpec FR-364 wire form for a task-product counterexample. Source
/// transitions and derived task transitions retain their existing tagged
/// wire arms.
pub fn task_temporal_wire(
    counterexample: &TemporalCounterexample<TaskProductTransition>,
    over: Option<&ObjectReference>,
    claim: Option<&OccurrenceKey>,
) -> Result<Json, CounterexampleWireError> {
    temporal(counterexample, over, claim)
}

/// The step alphabet a resolved item fixes for its counterexample document
/// (FR-237 "Document form"). It is never read from the payload.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TraceAlphabet {
    /// The plain model alphabet, [`ModelTransition`].
    Model,
    /// The task-derived alphabet of a release-backed schedulability item over
    /// an FR-249 task product, [`TaskProductTransition`].
    Task,
}

/// A counterexample document read with its item's alphabet.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReadCounterexample {
    /// A model-alphabet counterexample.
    Model(TemporalCounterexample<ModelTransition>),
    /// A task-alphabet counterexample.
    Task(TemporalCounterexample<TaskProductTransition>),
}

/// Read a model-trace JSON document with the alphabet its resolved item
/// fixes, then run `replay` over the typed counterexample.
///
/// This is the analyze-side readback seam: a refutation settles only when the
/// JSON the engine wrote passes the reader an auditor uses and then replays.
///
/// # Errors
///
/// The reader's [`TemporalCounterexampleReadRefusal`] for a malformed or
/// foreign-alphabet document, and any refusal `replay` returns.
pub fn replay_model_trace_document(
    document: &Json,
    alphabet: TraceAlphabet,
    replay: impl FnOnce(&ReadCounterexample) -> Result<bool, TemporalCounterexampleReadRefusal>,
) -> Result<bool, TemporalCounterexampleReadRefusal> {
    let read = match alphabet {
        TraceAlphabet::Task => {
            let read = read_task_temporal_document(document)?;
            if read.queue_growth.is_some() {
                return Err(TemporalCounterexampleReadRefusal::InvalidRuntimeInput {
                    pointer: "/queue_growth".to_owned(),
                    supplied: "queue-growth continuation".to_owned(),
                    recomputed: "queue-growth replay support is required".to_owned(),
                });
            }
            ReadCounterexample::Task(read.counterexample)
        }
        TraceAlphabet::Model => {
            if document.get("queue_growth").is_some() {
                return Err(TemporalCounterexampleReadRefusal::MalformedWire {
                    cause: "unknown-member",
                    pointer: "/queue_growth".to_owned(),
                });
            }
            ReadCounterexample::Model(read_temporal_counterexample::<ModelTransition>(document)?)
        }
    };
    replay(&read)
}

/// The FSpec FR-364 task-counterexample spelling with an inductive
/// `queue_growth` continuation. The finite model trace remains the existing
/// FR-364 arm; this helper only supplies its already-specified continuation
/// member and decisive miss position.
///
/// # Errors
///
/// [`CounterexampleWireError`] when the counterexample is not a non-empty
/// finite task miss prefix or the continuation has no JSON spelling.
pub fn task_temporal_wire_with_queue_growth(
    counterexample: &TemporalCounterexample<TaskProductTransition>,
    over: Option<&ObjectReference>,
    claim: Option<&OccurrenceKey>,
    queue_growth: Json,
    trace_position: u64,
) -> Result<Json, CounterexampleWireError> {
    if counterexample.lasso.is_some()
        || counterexample.prefix.is_empty()
        || !matches!(counterexample.kind, CounterexampleKind::Formula)
        || task_miss_position(counterexample) != Some(trace_position)
        || !valid_queue_growth_envelope(&queue_growth)
    {
        return Err(CounterexampleWireError::Unspelled(
            "task queue-growth counterexample shape",
        ));
    }
    let mut written = temporal(counterexample, over, claim)?;
    written["queue_growth"] = queue_growth;
    written["trace_position"] = json!(trace_position);
    Ok(written)
}

fn task_miss_position(
    counterexample: &TemporalCounterexample<TaskProductTransition>,
) -> Option<u64> {
    let mut misses = counterexample
        .prefix
        .iter()
        .enumerate()
        .filter(|(_, step)| {
            matches!(
                &step.transition,
                TaskProductTransition::Task(identity)
                    if identity.action == crate::TaskTransitionAction::DeadlineMiss
            )
        });
    let (position, _) = misses.next()?;
    if misses.next().is_some() || position + 1 != counterexample.prefix.len() {
        return None;
    }
    u64::try_from(position).ok()
}

fn valid_queue_growth_envelope(witness: &Json) -> bool {
    let Some(object) = witness.as_object() else {
        return false;
    };
    let required = [
        "version",
        "stem",
        "source_entry",
        "source_exit",
        "clock_order",
        "region_entry",
        "region_exit",
        "elapsed",
        "before",
        "after",
        "cycle",
        "iteration_shift",
        "identity_map",
        "fresh_releases",
        "backlog_growth",
    ];
    if object.len() != required.len()
        || required.iter().any(|member| !object.contains_key(*member))
        || object.get("version").and_then(Json::as_str) != Some("queue-growth/v1")
        || object.get("source_entry") != object.get("source_exit")
        || object.get("region_entry") != object.get("region_exit")
        || object.get("stem").and_then(Json::as_array).is_none()
        || object
            .get("clock_order")
            .and_then(Json::as_array)
            .is_none_or(Vec::is_empty)
        || object
            .get("cycle")
            .and_then(Json::as_array)
            .is_none_or(Vec::is_empty)
        || object
            .get("identity_map")
            .and_then(Json::as_array)
            .is_none()
        || object
            .get("fresh_releases")
            .and_then(Json::as_array)
            .is_none()
        || !valid_queue_growth_identity_map(&object["identity_map"])
    {
        return false;
    }

    let Some(before) = object.get("before").and_then(Json::as_object) else {
        return false;
    };
    let Some(after) = object.get("after").and_then(Json::as_object) else {
        return false;
    };
    let word_members = [
        "cycle_offset",
        "block_offset",
        "prefix",
        "growth_block",
        "suffix",
    ];
    if before.len() != word_members.len()
        || after.len() != word_members.len()
        || word_members
            .iter()
            .any(|member| !before.contains_key(*member) || !after.contains_key(*member))
    {
        return false;
    }
    let Some(shift) = object.get("iteration_shift").and_then(Json::as_u64) else {
        return false;
    };
    let Some(growth_block_len) = before
        .get("growth_block")
        .and_then(Json::as_array)
        .map(Vec::len)
        .and_then(|len| u64::try_from(len).ok())
    else {
        return false;
    };
    let Some(after_block_len) = after
        .get("growth_block")
        .and_then(Json::as_array)
        .map(Vec::len)
    else {
        return false;
    };
    let Some(before_offset) = before.get("block_offset").and_then(Json::as_u64) else {
        return false;
    };
    let Some(after_offset) = after.get("block_offset").and_then(Json::as_u64) else {
        return false;
    };
    if shift == 0
        || growth_block_len == 0
        || after_block_len == 0
        || before.get("cycle_offset").and_then(Json::as_u64) != Some(0)
        || after.get("cycle_offset").and_then(Json::as_u64) != Some(shift)
        || before_offset.checked_add(shift) != Some(after_offset)
        || object.get("backlog_growth").and_then(Json::as_u64)
            != shift.checked_mul(growth_block_len)
    {
        return false;
    }

    let Some(elapsed) = object.get("elapsed").and_then(rational_from_wire) else {
        return false;
    };
    if elapsed <= zero_rational() {
        return false;
    }
    let mut cycle_elapsed = zero_rational();
    for step in object
        .get("cycle")
        .and_then(Json::as_array)
        .into_iter()
        .flatten()
    {
        let Some(delay) = step.get("delay").and_then(rational_from_wire) else {
            return false;
        };
        cycle_elapsed = cycle_elapsed.add(&delay);
    }
    cycle_elapsed == elapsed
}

fn valid_queue_growth_identity_map(value: &Json) -> bool {
    let Some(mappings) = value.as_array() else {
        return false;
    };
    let mut destinations = std::collections::BTreeSet::new();
    mappings.iter().all(|mapping| {
        let Some(mapping) = mapping.as_object() else {
            return false;
        };
        let required = [
            "from_ref",
            "from_index_start",
            "from_index_end",
            "to_ref",
            "to_index_start",
            "to_index_stride",
            "cycle_shift",
        ];
        if mapping.len() != required.len()
            || required.iter().any(|field| !mapping.contains_key(*field))
            || mapping
                .get("from_ref")
                .and_then(Json::as_str)
                .is_none_or(str::is_empty)
            || mapping
                .get("to_ref")
                .and_then(Json::as_str)
                .is_none_or(str::is_empty)
        {
            return false;
        }
        let Some(from_start) = mapping.get("from_index_start").and_then(Json::as_u64) else {
            return false;
        };
        let from_end = match mapping.get("from_index_end") {
            Some(Json::Null) => true,
            Some(Json::String(end)) => end == "word-end",
            Some(Json::Number(end)) => end.as_u64().is_some_and(|end| end >= from_start),
            _ => false,
        };
        let Some(to_start) = mapping.get("to_index_start").and_then(Json::as_u64) else {
            return false;
        };
        if !from_end
            || mapping
                .get("to_index_stride")
                .and_then(Json::as_u64)
                .is_none_or(|stride| stride == 0)
            || mapping
                .get("cycle_shift")
                .and_then(Json::as_u64)
                .is_none_or(|shift| shift == 0)
        {
            return false;
        }
        let destination = (mapping["to_ref"].as_str().unwrap_or_default(), to_start);
        destinations.insert(destination)
    })
}

fn rational_from_wire(value: &Json) -> Option<Rational> {
    let numerator = value.get("numerator")?.as_str()?.parse().ok()?;
    let denominator = value.get("denominator")?.as_str()?.parse().ok()?;
    Rational::new(numerator, denominator).ok()
}

fn zero_rational() -> Rational {
    Rational::from_integer(filament_exact::Integer::zero())
}

/// FSpec FR-181's source-transition identity spelling for a task queue-growth
/// cycle or release template.
pub fn task_source_transition_wire(
    identity: &ModelTransition,
) -> Result<Json, CounterexampleWireError> {
    serde_json::to_value(super::TaskSourceTransitionWire::from(identity))
        .map_err(|_| CounterexampleWireError::Unspelled("source transition"))
}

/// FSpec FR-181's finite task-transition identity spelling.
pub fn task_transition_wire(
    identity: &TaskTransitionIdentity,
) -> Result<Json, CounterexampleWireError> {
    serde_json::to_value(super::TaskTransitionWire::from(identity))
        .map_err(|_| CounterexampleWireError::Unspelled("task transition"))
}

/// Spell a replayed hyper trace tuple using FSpec FR-400's wire form.
///
/// # Errors
///
/// Returns an error for a malformed key, transition or undefined source locus.
pub fn hyper_counterexample_wire(
    counterexample: &crate::HyperCounterexample,
    claim: Option<&OccurrenceKey>,
) -> Result<Json, CounterexampleWireError> {
    use crate::{HyperCounterexampleKind, HyperStep, StepMark};
    let mut traces = Vec::with_capacity(counterexample.traces.len());
    for trace in &counterexample.traces {
        let mut steps = Vec::with_capacity(trace.steps.len());
        for (index, found) in trace.steps.iter().enumerate() {
            let mut written = match found {
                HyperStep::Model(found) => {
                    let mut written = step(found)?;
                    written["type"] = json!("step");
                    written["post_state"] = key_text(&found.post_state)?;
                    written
                }
                HyperStep::TerminalStutter => json!({"type": "terminal-stutter"}),
            };
            if let Some(mark) = trace.marks.get(index) {
                written["mark"] = json!(match mark {
                    StepMark::Skipped => "skipped",
                    StepMark::Visible => "visible",
                });
            }
            steps.push(written);
        }
        traces.push(json!({"type": "hyper-trace", "variable": trace.variable,
            "alias": trace.alias, "initial": trace.initial.to_string(), "steps": steps}));
    }
    let instance: Vec<Json> = counterexample
        .instance
        .iter()
        .map(|(parameter, key)| json!({"parameter": parameter, "key": {"type": "string", "value": key}}))
        .collect();
    let mut written = json!({"type": "hyper-counterexample", "instance": instance,
        "traces": traces, "trace_position": counterexample.trace_position.to_string()});
    match &counterexample.kind {
        HyperCounterexampleKind::Lockstep { loop_entry } => {
            written["kind"] = json!("lockstep");
            written["loop_entry"] = json!(loop_entry.to_string());
        }
        HyperCounterexampleKind::WitnessExhausted {
            loop_entry,
            position,
        } => {
            written["kind"] = json!("witness-exhausted");
            written["loop_entry"] = json!(loop_entry.to_string());
            written["position"] = json!(position.to_string());
        }
        HyperCounterexampleKind::Projected => written["kind"] = json!("projected"),
        HyperCounterexampleKind::Closed => written["kind"] = json!("closed"),
        HyperCounterexampleKind::StepTuple => written["kind"] = json!("step-tuple"),
        HyperCounterexampleKind::Undefined(undefined) => {
            written["kind"] = json!("undefined");
            let variables: Vec<Json> = undefined
                .variables
                .iter()
                .map(|(variable, alias)| json!({"variable": variable, "alias": alias}))
                .collect();
            let state_keys = undefined
                .state_keys
                .iter()
                .map(key_text)
                .collect::<Result<Vec<_>, _>>()?;
            written["undefined"] = json!({"type": "undefined-evaluation", "where": {
                "position": undefined.position.to_string(), "state_keys": state_keys,
                "variables": variables, "locus": locus(claim)?,
            }, "cause": undefined_reason(&undefined.cause)});
            if let Some(transition) = &undefined.transition {
                written["undefined"]["where"]["transition"] = model_transition(transition)?;
            }
        }
    }
    Ok(written)
}
/// The protocol subject a counterexample was found over (FSpec FR-433's
/// `subject` and `max_live_instances` members).
#[derive(Clone, Debug)]
pub struct ProtocolWire {
    /// The protocol clause's qualified name.
    pub protocol: String,
    /// The reference the clause's `over` parameter is bound to.
    pub over: ObjectReference,
    /// Each static role and the domain package whose model plays it.
    pub roles: Vec<(String, String)>,
    /// Each symbolic capacity name and its bound value (FSpec FR-785);
    /// empty when the clause has no symbolic capacity.
    pub capacities: Vec<(String, u64)>,
    /// For an `on each` subject, the instance budget the run used.
    pub max_live_instances: Option<u64>,
}

/// FSpec FR-433's wire form of a counterexample over a protocol subject.
///
/// # Errors
///
/// [`CounterexampleWireError`] for a member FR-433 does not spell here.
pub fn protocol_temporal_wire(
    counterexample: &TemporalCounterexample<ProtocolTransition>,
    subject: &ProtocolWire,
    claim: Option<&OccurrenceKey>,
) -> Result<Json, CounterexampleWireError> {
    let mut written = temporal(counterexample, Some(&subject.over), claim)?;
    let roles: serde_json::Map<String, Json> = subject
        .roles
        .iter()
        .map(|(role, model)| (role.clone(), json!(model)))
        .collect();
    written["subject"] = json!({
        "type": "protocol-subject",
        "protocol": subject.protocol,
        "over": reference(&subject.over)?,
        "roles": roles,
    });
    // FSpec FR-433, FR-785: present exactly when the clause has a symbolic
    // capacity, each value as canonical decimal text.
    if !subject.capacities.is_empty() {
        let capacities: serde_json::Map<String, Json> = subject
            .capacities
            .iter()
            .map(|(name, value)| (name.clone(), json!(value.to_string())))
            .collect();
        written["subject"]["capacities"] = Json::Object(capacities);
    }
    if let Some(bound) = subject.max_live_instances {
        written["max_live_instances"] = json!(bound);
    }
    if matches!(counterexample.kind, CounterexampleKind::Deadlock) {
        written["blocked"] = Json::Array(
            counterexample
                .blocked
                .iter()
                .map(|thread| {
                    blocked_json(thread).ok_or(CounterexampleWireError::Unspelled("blocked thread"))
                })
                .collect::<Result<Vec<_>, _>>()?,
        );
    }
    Ok(written)
}

/// FR-286's `Assignments` spelling `{type: "assignments", values}`: one typed
/// value per parameter node, in the parameters' order, each in FR-181's typed
/// canonical form read from `types` and `variants`.
///
/// # Errors
///
/// [`CounterexampleWireError`] for a parameter node or value with no typed
/// form, or an assignment that does not give one value per parameter.
pub fn assignments_wire(
    parameters: &[filament_exact::NodeKey],
    values: &[Value],
    types: &filament_semantic_value::declaration::TypeEnvironment,
    variants: &[fsl_semantics::check::VariantTypeDeclaration],
) -> Result<Json, CounterexampleWireError> {
    if parameters.len() != values.len() {
        return Err(CounterexampleWireError::Unspelled("assignment arity"));
    }
    let values = parameters
        .iter()
        .zip(values)
        .map(|(parameter, value)| {
            Ok(json!({
                "parameter": node_key_to_json(parameter)
                    .map_err(|_| CounterexampleWireError::Unspelled("parameter"))?,
                "value": fsl_eval::simulation::model::typed_value_json(value, types, variants)
                    .ok_or(CounterexampleWireError::Unspelled("assignment value"))?,
            }))
        })
        .collect::<Result<Vec<_>, CounterexampleWireError>>()?;
    Ok(json!({"type": "assignments", "values": values}))
}

/// A `ProbabilisticCounterexample`'s `path-set` spelling (FSpec FR-413
/// "ProbabilisticCounterexample"): the subject's one initial state, the
/// binding, the claim's bound, the witness scheduler's entries and each path's
/// steps with their drawn random vectors. `state_key` reads a state's FR-181
/// key and `scheduler` is the scheduler's entries by the key of the state
/// each is made at.
///
/// # Errors
///
/// [`CounterexampleWireError::Unspelled`] when a state or value has no
/// encoding.
pub fn probabilistic_wire(
    witness: &crate::ProbabilisticCounterexample,
    bound: &fsl_semantics::check::Bound,
    types: &filament_semantic_value::declaration::TypeEnvironment,
    variants: &[fsl_semantics::check::VariantTypeDeclaration],
    state_key: &dyn Fn(
        &fsl_eval::simulation::model::ModelSimState,
    ) -> Option<fsl_eval::simulation::StateKey>,
    scheduler: &[(
        fsl_eval::simulation::StateKey,
        fsl_eval::simulation::model::ScheduledIdentity,
    )],
) -> Result<Json, CounterexampleWireError> {
    use CounterexampleWireError::Unspelled;
    let typed = |value: &Value| {
        fsl_eval::simulation::model::typed_value_json(value, types, variants)
            .ok_or(Unspelled("probabilistic value"))
    };
    let key = |at: &fsl_eval::simulation::model::ModelSimState| {
        state_key(at)
            .ok_or(Unspelled("probabilistic state"))
            .and_then(|key| state(&key))
    };
    let identity = |identity: &fsl_eval::simulation::model::ScheduledIdentity| {
        let operation: Json = serde_json::from_str(&identity.operation)
            .map_err(|_| Unspelled("scheduled operation"))?;
        Ok::<_, CounterexampleWireError>(json!({
            "type": "transition",
            "operation": operation,
            "receiver": reference(&identity.receiver)?,
            "arguments": identity.arguments.iter().map(typed).collect::<Result<Vec<_>, _>>()?,
        }))
    };
    let paths = witness
        .paths
        .iter()
        .map(|path| {
            path.steps
                .iter()
                .map(|step| {
                    let transition = step.transition.as_ref().map(model_transition).transpose()?;
                    let random = step
                        .draw
                        .as_ref()
                        .map(|draw| draw.iter().map(typed).collect::<Result<Vec<_>, _>>())
                        .transpose()?;
                    let mut written = json!({
                        "transition": transition,
                        "post_state": key(&step.post)?,
                        "random": random,
                    });
                    if step.elapsed != Rational::from_integer(filament_exact::Integer::from(0_i64))
                    {
                        written["delay"] = rational(&step.elapsed);
                    }
                    Ok(written)
                })
                .collect::<Result<Vec<_>, CounterexampleWireError>>()
        })
        .collect::<Result<Vec<_>, _>>()?;
    let (comparison, threshold) = match bound {
        fsl_semantics::check::Bound::AtLeast(threshold) => (">=", threshold),
        fsl_semantics::check::Bound::AtMost(threshold) => ("<=", threshold),
    };
    let mut written = json!({
        "type": "probabilistic",
        "initial_state": 0,
        "binding": witness.binding.as_ref().map(reference).transpose()?,
        "bound": {"comparison": comparison, "threshold": rational(threshold)},
        "evidence": {"type": "path-set", "paths": paths},
    });
    if let Some(subsystem) = &witness.subsystem {
        if !witness.paths.is_empty() {
            return Err(Unspelled("mixed probabilistic evidence"));
        }
        let certificate: Json = serde_json::from_slice(subsystem.as_bytes())
            .map_err(|_| Unspelled("probabilistic subsystem certificate"))?;
        written["instance"] = Json::Null;
        written["evidence"] = json!({"type":"subsystem","certificates":[certificate]});
    }
    let mut entries = scheduler
        .iter()
        .map(|(at, choice)| Ok(json!({"state": state(at)?, "choice": identity(choice)?})))
        .collect::<Result<Vec<_>, CounterexampleWireError>>()?;
    for (at, choice) in &witness.scheduler {
        if matches!(
            choice,
            crate::ProductAction::PostState(_) | crate::ProductAction::TerminalStutter
        ) {
            let choice = choice
                .to_bytes()
                .map_err(|_| Unspelled("probabilistic internal choice"))?;
            let choice: Json = serde_json::from_slice(&choice)
                .map_err(|_| Unspelled("probabilistic internal choice"))?;
            let at = fsl_eval::simulation::StateKey::from_canonical_bytes(at.as_bytes().to_vec());
            entries.push(json!({"state":state(&at)?,"choice":choice}));
        }
    }
    if !entries.is_empty() {
        written["scheduler"] = json!({"entries": entries});
    }
    Ok(written)
}

/// Spell a checked-subject undefined path with its complete pre-read product key.
/// This writes untrusted evidence; only semantic replay can reproduce it.
pub fn probabilistic_undefined_wire(
    witness: &crate::ProbabilityUndefinedWitness,
    claim: &OccurrenceKey,
    bound: &fsl_semantics::check::Bound,
    types: &filament_semantic_value::declaration::TypeEnvironment,
    variants: &[fsl_semantics::check::VariantTypeDeclaration],
    state_key: &dyn Fn(
        &fsl_eval::simulation::model::ModelSimState,
    ) -> Option<fsl_eval::simulation::StateKey>,
    scheduler: &[(
        fsl_eval::simulation::StateKey,
        fsl_eval::simulation::model::ScheduledIdentity,
    )],
) -> Result<Json, CounterexampleWireError> {
    use base64::Engine;
    let mut written =
        probabilistic_wire(&witness.path, bound, types, variants, state_key, scheduler)?;
    let paths = written["evidence"]["paths"].take();
    let mut evaluation = undefined(&witness.evaluation, Some(claim))?;
    evaluation["where"]["product_state"] = json!({"key": base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(witness.product_state.as_bytes())});
    written["evidence"] =
        json!({"type": "undefined-path", "path": paths[0], "undefined": evaluation});
    Ok(written)
}

/// The unpadded base64url spelling of a state key, as FSpec FR-393 writes a
/// `post_state` and a `state_key`.
fn key_text(key: &fsl_eval::simulation::StateKey) -> Result<Json, CounterexampleWireError> {
    state(key)?
        .get("key")
        .cloned()
        .ok_or(CounterexampleWireError::Unspelled("state key"))
}

fn graph_step(
    step: &crate::model_check::counterexample::ModelStep,
) -> Result<Json, CounterexampleWireError> {
    let mut written = json!({
        "type": "step",
        "transition": model_transition(&step.transition)?,
        "post_state": key_text(&step.post_state)?,
    });
    if let Some(delay) = &step.delay {
        written["delay"] = rational(delay);
    }
    Ok(written)
}

fn graph_steps(
    steps: &[crate::model_check::counterexample::ModelStep],
) -> Result<Vec<Json>, CounterexampleWireError> {
    steps.iter().map(graph_step).collect()
}

fn model_path(
    path: &crate::model_check::state_graph::ModelPath,
) -> Result<Json, CounterexampleWireError> {
    Ok(json!({
        "type": "model-path",
        "initial": path.initial.to_string(),
        "steps": graph_steps(&path.steps)?,
    }))
}

fn graph_witness(
    path: &crate::model_check::state_graph::WitnessPath,
    source: &crate::model_check::state_graph::WitnessSource,
) -> Result<Json, CounterexampleWireError> {
    let crate::model_check::state_graph::WitnessPath::Path(path) = path else {
        return Err(CounterexampleWireError::Unspelled("lasso witness"));
    };
    Ok(json!({
        "path": model_path(path)?,
        "source": serde_json::to_value(source)
            .map_err(|_| CounterexampleWireError::Unspelled("witness source"))?,
    }))
}

/// FSpec FR-393's spelling of the state-graph `evidence` of the instance
/// bound to `over` (the claim's `over` parameter `parameter`), with
/// `sampled` the witnesses phase 0 found per initial state (FR-628), and
/// `claim` the claim occurrence, the `locus` of an `undefined` arm's
/// expression (FR-628 "Evidence encoding").
///
/// # Errors
///
/// [`CounterexampleWireError`] for a lasso witness, which no state-graph
/// claim yields, and for a state or transition with no spelling.
pub fn graph_evidence_wire(
    evidence: &crate::model_check::state_graph::GraphEvidence,
    form: crate::model_check::state_graph::StateGraphPropertyForm,
    parameter: Option<&str>,
    over: Option<&ObjectReference>,
    sampled: &[Option<crate::model_check::state_graph::GraphWitness>],
    claim: Option<&OccurrenceKey>,
) -> Result<Json, CounterexampleWireError> {
    use crate::model_check::state_graph::{GraphEvidence, StateGraphPropertyForm};
    let key = match over {
        Some(found) => reference(found)?,
        None => Json::Null,
    };
    let instance = json!({"type": "instance", "parameter": parameter, "key": key});
    let mut written = json!({"type": "graph-evidence", "instance": instance});
    match evidence {
        GraphEvidence::Witness { paths, .. } => {
            written["arm"] = json!("witness");
            written["paths"] = Json::Array(
                paths
                    .iter()
                    .map(|(path, source)| graph_witness(path, source))
                    .collect::<Result<Vec<_>, _>>()?,
            );
            if !sampled.is_empty() {
                written["sampled"] = Json::Array(
                    sampled
                        .iter()
                        .map(|found| match found {
                            Some(found) => graph_witness(
                                &crate::model_check::state_graph::WitnessPath::Path(
                                    found.path.clone(),
                                ),
                                &found.source,
                            ),
                            None => Ok(Json::Null),
                        })
                        .collect::<Result<Vec<_>, _>>()?,
                );
            }
        }
        GraphEvidence::Trap { stem, .. } => {
            written["arm"] = json!("trap");
            written["stem"] = model_path(stem)?;
        }
        GraphEvidence::PathPair {
            stem,
            first,
            second,
            ..
        } => {
            written["arm"] = json!("path-pair");
            written["stem"] = model_path(stem)?;
            written["first"] = Json::Array(graph_steps(first)?);
            written["second"] = Json::Array(graph_steps(second)?);
        }
        GraphEvidence::Undefined {
            stem, undefined, ..
        } => {
            let role = undefined.predicate.map(|role| match (role, form) {
                (
                    fsl_semantics::check::PredicateRole::Target,
                    StateGraphPropertyForm::UniquePath,
                ) => "Y",
                (fsl_semantics::check::PredicateRole::Target, _) => "P",
                (
                    fsl_semantics::check::PredicateRole::From,
                    StateGraphPropertyForm::AlwaysPossible,
                ) => "Q",
                (fsl_semantics::check::PredicateRole::From, _) => "X",
            });
            let mut at = json!({
                "position": stem.steps.len(),
                "state_key": key_text(&undefined.state_key)?,
                "locus": locus(claim)?,
            });
            if let Some(role) = role {
                at["predicate"] = json!(role);
            }
            if let Some(guard) = &undefined.transition {
                at["transition"] = model_transition(guard)?;
            }
            written["arm"] = json!("undefined");
            written["stem"] = model_path(stem)?;
            written["undefined"] = json!({
                "type": "undefined-evaluation",
                "where": at,
                "cause": undefined_reason(&undefined.cause),
            });
        }
    }
    Ok(written)
}

/// FSpec FR-331's `TransitionIdentity` spelling of one task-derived
/// transition: a source `transition`, a `task-transition` or the
/// `terminal-stutter` marker.
///
/// # Errors
///
/// [`CounterexampleWireError`] when the identity has no canonical spelling.
pub fn task_product_transition_wire(
    transition: &TaskProductTransition,
) -> Result<Json, CounterexampleWireError> {
    Spelling::transition(transition)
}

/// FSpec FR-331's `State` locus `{key}` of a state key.
///
/// # Errors
///
/// [`CounterexampleWireError`] when the key has no wire spelling.
pub fn state_key_wire(
    key: &fsl_eval::simulation::StateKey,
) -> Result<Json, CounterexampleWireError> {
    state(key)
}

#[cfg(test)]
mod tests {
    use filament_exact::{Integer, Origin, Role, Undefined};
    use filament_semantic_value::model_transition::ModelTransition;
    use fsl_eval::simulation::StateKey;
    use fsl_semantics::check::FormulaNodeId;
    use ix_trace_rs::trace;

    use super::*;
    use crate::model_check::counterexample::{Lasso, UndefinedSite};
    use crate::refinement::RefinementFailure;
    use filament_exact::NodeKey;

    fn rational(numerator: i64, denominator: i64) -> Rational {
        Rational::new(Integer::from(numerator), Integer::from(denominator))
            .expect("a nonzero denominator")
    }

    fn key(index: usize) -> StateKey {
        StateKey::from_canonical_bytes(format!("[{index},[]]").into_bytes())
    }

    fn step(index: usize) -> ModelStep {
        ModelStep {
            transition: ModelTransition::new(
                format!(r#"{{"name":"s{index}"}}"#),
                None,
                Vec::new(),
                quire_canonical::Limits::new(u64::MAX),
            )
            .expect("a canonical transition"),
            post_state: key(index),
            delay: None,
        }
    }

    /// FSpec FR-712 timed state-graph paths retain the chosen exact delay
    /// before each discrete step. Untimed paths keep the FR-393 shape.
    #[test]
    fn a_timed_graph_step_carries_its_exact_delay() {
        let mut found = step(1);
        found.delay = Some(rational(3, 2));

        assert_eq!(
            graph_step(&found).expect("spelled"),
            json!({
                "type": "step",
                "transition": {"type": "transition", "operation": {"name": "s1"}, "receiver": null, "arguments": []},
                "post_state": key_text(&key(1)).expect("key"),
                "delay": {"numerator": "3", "denominator": "2"},
            })
        );
    }

    fn counterexample(steps: usize, kind: CounterexampleKind) -> TemporalCounterexample {
        TemporalCounterexample {
            initial_state: 0,
            prefix: (1..=steps).map(step).collect(),
            over_binding: None,
            fairness: Vec::new(),
            kind,
            lasso: None,
            blocked: Vec::new(),
            refinement: None,
            final_delay: None,
        }
    }

    /// FR-364 retains the existing model arm when an unbounded task queue is
    /// carried as its checked continuation.
    #[trace(
        "TC-372-RT-25",
        "FR-237-AC-7",
        "FR-237-AC-10",
        "FR-237-AC-11",
        "FR-237-AC-12",
        "FR-364-AC-2"
    )]
    #[test]
    fn task_queue_growth_uses_the_existing_model_counterexample_shape() {
        let release = step(1).transition;
        let source_release = task_source_transition_wire(&release).expect("source transition");
        let one = json!({"base": 0, "cycle_stride": 0, "cohort_stride": 0});
        let one_rational = json!({
            "base": {"numerator": "0", "denominator": "1"},
            "cycle_stride": {"numerator": "0", "denominator": "1"},
            "cohort_stride": {"numerator": "0", "denominator": "1"}
        });
        let job = json!({
            "ref": "G",
            "taskset": "S",
            "task": "t",
            "occurrence": one,
            "generation": one,
            "queue_position": one,
            "demand": {"numerator": "2", "denominator": "1"},
            "remaining": {"numerator": "2", "denominator": "1"},
            "activation_offset": one_rational,
            "deadline_offset": one_rational,
            "jitter": {"numerator": "0", "denominator": "1"},
            "priority": 1,
            "allowance": {"numerator": "2", "denominator": "1"},
            "status": "ready"
        });
        let queue_word = json!({
            "cycle_offset": 0,
            "block_offset": 0,
            "prefix": [],
            "growth_block": [job],
            "suffix": []
        });
        let missed_job = TaskTransitionIdentity {
            action: crate::TaskTransitionAction::DeadlineMiss,
            task_set: "S".to_owned(),
            task: "t".to_owned(),
            job: 0,
            slot: 0,
            generation: 0,
            release: release.clone(),
            source_transition: None,
            demand: rational(2, 1),
            jitter: rational(0, 1),
            deadline: rational(1, 1),
            priority: Some(1),
            budget_delta: None,
        };
        let task_counterexample = TemporalCounterexample {
            initial_state: 0,
            prefix: vec![ModelStep {
                transition: TaskProductTransition::Task(Box::new(missed_job.clone())),
                post_state: key(1),
                delay: Some(rational(1, 1)),
            }],
            over_binding: None,
            fairness: Vec::new(),
            kind: CounterexampleKind::Formula,
            lasso: None,
            blocked: Vec::new(),
            refinement: None,
            final_delay: None,
        };
        let mut activation = missed_job.clone();
        activation.action = crate::TaskTransitionAction::Activate;
        activation.source_transition = Some(release.clone());
        let readable_counterexample = TemporalCounterexample {
            initial_state: 0,
            prefix: vec![
                ModelStep {
                    transition: TaskProductTransition::Task(Box::new(activation)),
                    post_state: key(0),
                    delay: Some(rational(0, 1)),
                },
                task_counterexample.prefix[0].clone(),
                ModelStep {
                    transition: TaskProductTransition::Source(step(2).transition),
                    post_state: key(2),
                    delay: Some(rational(1, 1)),
                },
            ],
            over_binding: None,
            fairness: Vec::new(),
            kind: CounterexampleKind::Formula,
            lasso: Some(Lasso {
                entry: 2,
                terminal_stutter: false,
                terminal_stutter_delay: None,
            }),
            blocked: Vec::new(),
            refinement: None,
            final_delay: None,
        };
        let readable_wire = task_temporal_wire(&readable_counterexample, None, None)
            .expect("activation, miss and loop use the task-product alphabet");
        let independently_read = read_task_temporal_counterexample(&readable_wire)
            .expect("the task-product document reads without a typed expectation");
        assert_eq!(
            task_temporal_wire(&independently_read, None, None)
                .expect("independently read task trace rewrites losslessly"),
            readable_wire
        );
        let admitted = read_temporal_counterexample::<TaskProductTransition>(&readable_wire)
            .expect("the typed task counterexample reads back");
        assert_eq!(
            task_temporal_wire(&admitted, None, None).expect("rewritten task trace"),
            readable_wire
        );

        let mut unknown_member = readable_wire.clone();
        unknown_member["prefix"][1]["transition"]["phase"] = json!("hi");
        assert_eq!(
            read_task_temporal_counterexample(&unknown_member),
            Err(TemporalCounterexampleReadRefusal::MalformedWire {
                cause: "unknown-member",
                pointer: "/prefix/1/transition/phase".to_owned(),
            })
        );
        let mut alphabet_then_malformed = readable_wire.clone();
        alphabet_then_malformed["prefix"][0]["transition"] = json!({"type": "other"});
        alphabet_then_malformed["prefix"][1]["transition"]["phase"] = json!("hi");
        assert_eq!(
            read_task_temporal_counterexample(&alphabet_then_malformed),
            Err(TemporalCounterexampleReadRefusal::MalformedWire {
                cause: "unknown-member",
                pointer: "/prefix/1/transition/phase".to_owned(),
            })
        );
        assert_eq!(
            read_temporal_counterexample::<TaskProductTransition>(&unknown_member),
            Err(TemporalCounterexampleReadRefusal::MalformedWire {
                cause: "unknown-member",
                pointer: "/prefix/1/transition/phase".to_owned(),
            })
        );
        let mut unknown_action = readable_wire.clone();
        unknown_action["prefix"][1]["transition"]["action"] = json!("overflow");
        assert_eq!(
            read_temporal_counterexample::<TaskProductTransition>(&unknown_action),
            Err(TemporalCounterexampleReadRefusal::MalformedWire {
                cause: "unknown-variant",
                pointer: "/prefix/1/transition/action".to_owned(),
            })
        );
        let mut unexpected_delta = readable_wire.clone();
        unexpected_delta["prefix"][0]["transition"]["budget_delta"] = json!({
            "numerator": "1",
            "denominator": "1"
        });
        assert_eq!(
            read_temporal_counterexample::<TaskProductTransition>(&unexpected_delta),
            Err(TemporalCounterexampleReadRefusal::MalformedWire {
                cause: "budget-delta-presence",
                pointer: "/prefix/0/transition/budget_delta".to_owned(),
            })
        );
        let mut missing_delay = readable_wire.clone();
        missing_delay["prefix"][0]
            .as_object_mut()
            .unwrap()
            .remove("delay");
        assert_eq!(
            read_temporal_counterexample::<TaskProductTransition>(&missing_delay),
            Err(TemporalCounterexampleReadRefusal::MalformedWire {
                cause: "missing-member",
                pointer: "/prefix/0/delay".to_owned(),
            })
        );
        let mut wrong_job_kind = readable_wire.clone();
        wrong_job_kind["prefix"][0]["transition"]["job"] = json!("0");
        assert_eq!(
            read_temporal_counterexample::<TaskProductTransition>(&wrong_job_kind),
            Err(TemporalCounterexampleReadRefusal::MalformedWire {
                cause: "wrong-value-kind",
                pointer: "/prefix/0/transition/job".to_owned(),
            })
        );
        // A step outside the requested alphabet is an alphabet refusal, and
        // a shape fault anywhere wins over it.
        assert_eq!(
            read_temporal_counterexample::<ModelTransition>(&readable_wire),
            Err(TemporalCounterexampleReadRefusal::TraceAlphabet {
                pointer: "/prefix/0/transition/type".to_owned(),
            })
        );
        let mut malformed_before_alphabet = readable_wire.clone();
        malformed_before_alphabet["prefix"][0]["transition"]["phase"] = json!("hi");
        assert_eq!(
            read_temporal_counterexample::<ModelTransition>(&malformed_before_alphabet),
            Err(TemporalCounterexampleReadRefusal::MalformedWire {
                cause: "unknown-member",
                pointer: "/prefix/0/transition/phase".to_owned(),
            })
        );
        let mut tampered_deadline = readable_wire.clone();
        tampered_deadline["prefix"][1]["transition"]["deadline"]["numerator"] = json!("2");
        // A well-shaped value is the replay's to check against the rebuilt
        // product, not the reader's: it reads back, and rewriting gives the
        // tampered JSON unchanged.
        let tampered_read =
            read_temporal_counterexample::<TaskProductTransition>(&tampered_deadline)
                .expect("a well-shaped tampered value reads");
        assert_eq!(
            task_temporal_wire(&tampered_read, None, None).expect("rewritten tampered trace"),
            tampered_deadline
        );
        let continuation = json!({
            "version": "queue-growth/v1",
            "stem": [],
            "source_entry": {"key": "AA"},
            "source_exit": {"key": "AA"},
            "clock_order": ["0"],
            "region_entry": [[{"bound": "0", "strict": false}]],
            "region_exit": [[{"bound": "0", "strict": false}]],
            "elapsed": {"numerator": "1", "denominator": "1"},
            "before": {
                "cycle_offset": 0,
                "block_offset": 0,
                "prefix": [],
                "growth_block": [job],
                "suffix": []
            },
            "after": {
                "cycle_offset": 1,
                "block_offset": 1,
                "prefix": [],
                "growth_block": [job],
                "suffix": []
            },
            "cycle": [{
                "transition": {
                    "type": "task-queue-growth-transition",
                    "action": "complete",
                    "taskset": "S",
                    "task": "t",
                    "job_ref": "G",
                    "occurrence": one,
                    "generation": one,
                    "virtual_position": one,
                    "release_transition": source_release,
                    "source_transition": null,
                    "demand": {"numerator": "2", "denominator": "1"},
                    "jitter": {"numerator": "0", "denominator": "1"},
                    "deadline_offset": one_rational,
                    "priority": 1
                },
                "job_ref": "G",
                "delay": {"numerator": "1", "denominator": "1"},
                "service": {"numerator": "2", "denominator": "1"},
                "ready_order": queue_word
            }],
            "iteration_shift": 1,
            "identity_map": [],
            "fresh_releases": [],
            "backlog_growth": 1
        });
        let wire = task_temporal_wire_with_queue_growth(
            &task_counterexample,
            None,
            None,
            continuation.clone(),
            0,
        )
        .expect("the typed task prefix keeps the existing FR-364 model arm");
        assert_eq!(wire["trace"], "model");
        assert_eq!(wire["prefix"].as_array().map(Vec::len), Some(1));
        assert_eq!(wire["loop"], json!([]));
        assert_eq!(wire["trace_position"], 0);
        assert_eq!(wire["queue_growth"], continuation);
        assert!(wire.get("task_automaton_trace").is_none());
        let read_document = read_task_temporal_document(&wire)
            .expect("the queue-growth model arm reads as one FR-364 document");
        assert_eq!(read_document.queue_growth.as_ref(), Some(&continuation));
        assert!(read_document.counterexample.lasso.is_none());

        assert!(task_temporal_wire_with_queue_growth(
            &task_counterexample,
            None,
            None,
            wire["queue_growth"].clone(),
            1,
        )
        .is_err());
        let mut repeated_miss = task_counterexample.clone();
        repeated_miss.prefix.push(repeated_miss.prefix[0].clone());
        assert!(task_temporal_wire_with_queue_growth(
            &repeated_miss,
            None,
            None,
            wire["queue_growth"].clone(),
            0,
        )
        .is_err());
        let mut looped = task_counterexample.clone();
        looped.lasso = Some(Lasso {
            entry: 0,
            terminal_stutter: false,
            terminal_stutter_delay: None,
        });
        assert!(task_temporal_wire_with_queue_growth(
            &looped,
            None,
            None,
            wire["queue_growth"].clone(),
            0,
        )
        .is_err());
        let mut trailing_step = task_counterexample.clone();
        trailing_step.prefix.push(ModelStep {
            transition: TaskProductTransition::Source(step(2).transition),
            post_state: key(2),
            delay: Some(rational(0, 1)),
        });
        assert!(task_temporal_wire_with_queue_growth(
            &trailing_step,
            None,
            None,
            wire["queue_growth"].clone(),
            0,
        )
        .is_err());

        for (member, value) in [
            ("backlog_growth", json!(0)),
            ("elapsed", json!({"numerator": "2", "denominator": "1"})),
        ] {
            let mut invalid = wire["queue_growth"].clone();
            invalid[member] = value;
            assert!(task_temporal_wire_with_queue_growth(
                &task_counterexample,
                None,
                None,
                invalid,
                0,
            )
            .is_err());
        }
        let mut invalid_offset = wire["queue_growth"].clone();
        invalid_offset["after"]["block_offset"] = json!(0);
        assert!(task_temporal_wire_with_queue_growth(
            &task_counterexample,
            None,
            None,
            invalid_offset,
            0,
        )
        .is_err());

        let mut duplicate_destination = wire["queue_growth"].clone();
        let mapping = json!({
            "from_ref": "G",
            "from_index_start": 0,
            "from_index_end": 0,
            "to_ref": "head",
            "to_index_start": 0,
            "to_index_stride": 1,
            "cycle_shift": 1
        });
        duplicate_destination["identity_map"] = json!([mapping.clone(), mapping]);
        assert!(task_temporal_wire_with_queue_growth(
            &task_counterexample,
            None,
            None,
            duplicate_destination,
            0,
        )
        .is_err());

        let mut malformed_identity = wire["queue_growth"].clone();
        malformed_identity["identity_map"] = json!([{
            "from_ref": "G",
            "from_index_start": 0,
            "from_index_end": "past-end",
            "to_ref": "head",
            "to_index_start": 0,
            "to_index_stride": 1,
            "cycle_shift": 1
        }]);
        assert!(task_temporal_wire_with_queue_growth(
            &task_counterexample,
            None,
            None,
            malformed_identity,
            0,
        )
        .is_err());
    }

    /// FSpec FR-364-AC-15: task-transition spelling preserves a checked
    /// declared priority and omits priority when the task has none.
    #[trace("FSpec-FR-364-AC-15")]
    #[test]
    fn task_transition_wire_preserves_optional_priority() {
        let release = ModelTransition::new(
            r#"{"name":"release"}"#.to_owned(),
            None,
            Vec::new(),
            quire_canonical::Limits::new(u64::MAX),
        )
        .expect("a canonical release identity");
        let identity = |priority| TaskTransitionIdentity {
            action: crate::TaskTransitionAction::Activate,
            task_set: "S".to_owned(),
            task: "T".to_owned(),
            job: 0,
            slot: 0,
            generation: 0,
            release: release.clone(),
            source_transition: Some(release.clone()),
            demand: rational(2, 1),
            jitter: rational(0, 1),
            deadline: rational(1, 1),
            priority,
            budget_delta: None,
        };

        let with_priority = task_transition_wire(&identity(Some(3))).expect("spelled task step");
        assert_eq!(with_priority["priority"], json!(3));
        assert_eq!(with_priority["release"]["operation"], "release");

        let without_priority = task_transition_wire(&identity(None)).expect("spelled task step");
        assert!(without_priority.get("priority").is_none());
    }

    fn claim() -> OccurrenceKey {
        OccurrenceKey::new(crate::fixture_node(7), Origin::new(Role::new("claim"), 2))
    }

    fn undefined_at(time: bool) -> CounterexampleKind {
        CounterexampleKind::UndefinedEvaluation(UndefinedEvaluation {
            site: UndefinedSite {
                position: 2,
                state_key: key(2),
                locus: UndefinedLocus::Atom(FormulaNodeId::from_arena_index(1)),
                transition: None,
                time: time.then(|| {
                    Box::new(crate::model_check::counterexample::TimedPoint {
                        stamp: rational(3, 2),
                        clocks: vec![rational(1, 2)],
                    })
                }),
            },
            cause: filament_eval_undefined(),
        })
    }

    fn filament_eval_undefined() -> fsl_eval::temporal::UndefinedCause {
        fsl_eval::temporal::UndefinedCause::Kernel(Undefined::DivisionByZero)
    }

    /// FR-286-AC-14: a lasso writes its stem as `prefix` and its loop as
    /// `loop`, split at the entry the lasso re-enters.
    #[trace("TC-770", "FR-286-AC-14")]
    #[test]
    fn a_lasso_splits_its_stem_from_a_non_empty_loop() {
        let mut found = counterexample(3, CounterexampleKind::Formula);
        found.lasso = Some(Lasso {
            entry: 1,
            terminal_stutter: false,
            terminal_stutter_delay: None,
        });
        let wire = temporal_wire(&found, None, None).expect("spelled");
        assert_eq!(wire["prefix"].as_array().map(Vec::len), Some(1));
        let looped = wire["loop"].as_array().expect("a loop");
        assert_eq!(looped.len(), 2);
        assert_eq!(looped[1]["post_state"], state(&key(3)).expect("a key"));
        assert_eq!(wire["trace_position"], 0);
    }

    /// FR-237-AC-10: analyze only settles a refutation after its emitted
    /// document matches the canonical typed trace and that trace replays.
    #[trace("FR-237-AC-10")]
    #[test]
    fn emitted_model_trace_is_re_admitted_before_replay() {
        let found = counterexample(1, CounterexampleKind::Formula);
        let wire = temporal_wire(&found, None, None).expect("spelled");
        assert!(
            replay_model_trace_document(&wire, TraceAlphabet::Model, |_| Ok(true))
                .expect("wire can be checked")
        );

        let mut changed = wire;
        changed["kind"] = json!("time-lock");
        let mut replayed = false;
        assert!(
            replay_model_trace_document(&changed, TraceAlphabet::Model, |_| {
                replayed = true;
                Ok(true)
            })
            .is_err()
        );
        assert!(!replayed, "mismatched wire must not reach replay");
    }

    /// FR-286-AC-14: a loop that is the terminal stutter appends the
    /// terminal-stutter identity at the last post-state, and one at an
    /// initial state has no step to repeat.
    #[trace("TC-770", "FR-286-AC-14")]
    #[test]
    fn a_terminal_stutter_loop_repeats_the_last_post_state() {
        let mut found = counterexample(2, CounterexampleKind::Formula);
        found.lasso = Some(Lasso {
            entry: 2,
            terminal_stutter: true,
            terminal_stutter_delay: None,
        });
        let wire = temporal_wire(&found, None, None).expect("spelled");
        assert_eq!(wire["prefix"].as_array().map(Vec::len), Some(2));
        assert_eq!(
            wire["loop"],
            json!([{
                "transition": {"type": "terminal-stutter"},
                "post_state": state(&key(2)).expect("a key"),
            }])
        );
        let mut initial = counterexample(0, CounterexampleKind::Formula);
        initial.lasso = Some(Lasso {
            entry: 0,
            terminal_stutter: true,
            terminal_stutter_delay: None,
        });
        assert_eq!(
            temporal_wire(&initial, None, None),
            Err(CounterexampleWireError::Unspelled(
                "initial terminal stutter"
            ))
        );
    }

    /// FR-286-AC-14: a time-lock counterexample carries its `final_delay`,
    /// and no other kind does.
    #[trace("TC-770", "FR-286-AC-14")]
    #[test]
    fn a_time_lock_carries_its_final_delay() {
        let mut found = counterexample(1, CounterexampleKind::TimeLock);
        found.final_delay = Some(rational(3, 2));
        let wire = temporal_wire(&found, None, None).expect("spelled");
        assert_eq!(wire["kind"], "time-lock");
        assert_eq!(
            wire["final_delay"],
            json!({"numerator": "3", "denominator": "2"})
        );
        assert_eq!(wire["trace_position"], Json::Null);
        let formula = temporal_wire(&counterexample(1, CounterexampleKind::Formula), None, None)
            .expect("spelled");
        assert!(formula.get("final_delay").is_none());
    }

    /// FR-286-AC-14: a timed step carries the delay that elapses before it.
    #[trace("TC-770", "FR-286-AC-14")]
    #[test]
    fn a_timed_step_carries_its_delay() {
        let mut found = counterexample(2, CounterexampleKind::Formula);
        found.prefix[1].delay = Some(rational(1, 4));
        let wire = temporal_wire(&found, None, None).expect("spelled");
        assert!(wire["prefix"][0].get("delay").is_none());
        assert_eq!(
            wire["prefix"][1]["delay"],
            json!({"numerator": "1", "denominator": "4"})
        );
    }

    /// FR-286-AC-14: an undefined evaluation is FSpec FR-364's envelope: the
    /// item's claim is the `{node, occurrence}` locus, a timed site is
    /// spelled without its clock valuation, and a counterexample with no
    /// claim has no locus.
    #[trace("TC-770", "FR-286-AC-14")]
    #[test]
    fn an_undefined_evaluation_names_the_claim_as_its_locus() {
        for time in [false, true] {
            let found = counterexample(2, undefined_at(time));
            let wire = temporal_wire(&found, None, Some(&claim())).expect("spelled");
            assert_eq!(wire["kind"]["type"], "undefined-evaluation");
            assert_eq!(wire["kind"]["cause"], "division-by-zero");
            let at = &wire["kind"]["where"];
            assert_eq!(at["position"], 2);
            assert_eq!(at["state_key"], state(&key(2)).expect("a key"));
            assert_eq!(
                at["locus"]["occurrence"],
                json!({"role": "claim", "ordinal": 2})
            );
            assert!(at["locus"]["node"]["path"].is_array());
            assert!(at.get("time").is_none());
            assert_eq!(at.as_object().map(serde_json::Map::len), Some(3));
        }
        assert_eq!(
            temporal_wire(&counterexample(2, undefined_at(false)), None, None),
            Err(CounterexampleWireError::Unspelled("undefined locus"))
        );
    }

    /// FSpec FR-379: a `Divergence` names the constraint it violates, and one
    /// without a constraint is not spelled.
    #[trace("TC-770", "FR-286-AC-14")]
    #[test]
    fn a_divergence_without_a_constraint_is_not_spelled() {
        let mut found = counterexample(1, CounterexampleKind::Formula);
        found.refinement = Some(RefinementFailure::Divergence {
            constraint: None,
            protocol_class: None,
        });
        assert_eq!(
            temporal_wire(&found, None, None),
            Err(CounterexampleWireError::Unspelled("divergence constraint"))
        );
    }

    /// FR-286: a refinement failure with a position writes the FR-379 member
    /// and no other.
    #[trace("TC-770", "FR-286-AC-14")]
    #[test]
    fn a_refinement_failure_is_written_as_its_own_member() {
        let mut found = counterexample(1, CounterexampleKind::Formula);
        found.refinement = Some(RefinementFailure::StutterChanged { position: 1 });
        let wire = temporal_wire(&found, None, None).expect("spelled");
        assert_eq!(
            wire["refinement_failure"],
            json!({"kind": "StutterChanged", "position": 1})
        );
    }

    /// FR-286: an assignment gives one value per parameter, and one that does
    /// not is not spelled.
    #[trace("TC-770", "FR-286-AC-14")]
    #[test]
    fn an_assignment_that_does_not_match_its_parameters_is_not_spelled() {
        let parameter = NodeKey::new(
            "fixture",
            vec![filament_exact::PathSegment::Text("x".to_owned())],
            0,
        )
        .expect("a node");
        let types = filament_semantic_value::declaration::TypeEnvironment::default();
        let one = [Value::Integer(Integer::from(1_i64))];
        let wire = assignments_wire(std::slice::from_ref(&parameter), &one, &types, &[])
            .expect("one value per parameter");
        assert_eq!(
            wire["values"][0]["value"],
            json!({"type": "integer", "value": "1"})
        );
        for (parameters, values) in [
            (std::slice::from_ref(&parameter), &[][..]),
            (&[][..], &one[..]),
        ] {
            assert_eq!(
                assignments_wire(parameters, values, &types, &[]),
                Err(CounterexampleWireError::Unspelled("assignment arity"))
            );
        }
    }
}
