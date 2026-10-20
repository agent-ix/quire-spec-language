// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-138 "Layer 5" (ADR-020 AX-1, AX-3): the history values of a concrete
//! behaviour.
//!
//! A history field belongs to the refinement: it never enters the concrete
//! package, its clauses or its successor relation, and an update never
//! removes a step. [`initial_history`] gives every object of a type with a
//! history row the row's initial literal. [`step_history`] evaluates the
//! updates of a step's operation through the one clause evaluator over the
//! pre-state and post-state observations, each with a fresh meter, and writes
//! the receiver's fields only; every other value is kept, an object the step
//! creates takes the initial literal, and an object it deletes loses its
//! values. The values are a function of the concrete prefix.
//!
//! An update that reads a history field outside `pre(..)` reads the value the
//! pre-state holds: the post-state's value is what the update computes.

use std::collections::{BTreeMap, BTreeSet};

use qsl_eval::simulation::model::{
    operation_identity_text, ModelSimState, ModelSystem, ResolvedStep, RowReading,
};
use qsl_semantics::check::{CheckedRefinement, HistoryRow};
use qsl_semantics::model::state::ModelState;
use quire_exact::{ObjectReference, Value};
use quire_semantic_value::model_transition::ModelTransition;

use super::map::{
    object_of, EvalFailure, MappingFailure, MappingRowRef, MappingUndetermined, ObjectKey,
    Outcomes, BROKEN_INVARIANT,
};

/// The value of each history field of each object of a concrete state
/// (FR-138 `HistoryValues`), sorted by object key and field.
#[derive(Clone, Debug, Default)]
pub struct HistoryValues {
    entries: BTreeMap<(ObjectKey, String), Value>,
}

impl HistoryValues {
    /// The value of `field` of `object`, when it has one.
    pub fn get(&self, object: &ObjectKey, field: &str) -> Option<&Value> {
        self.entries.get(&(object.clone(), field.to_owned()))
    }

    /// Every value with its object and field, in key order.
    pub fn iter(&self) -> impl Iterator<Item = (&ObjectKey, &str, &Value)> {
        self.entries
            .iter()
            .map(|((object, field), value)| (object, field.as_str(), value))
    }

    /// The canonical bytes that key these values in a product state
    /// (FR-142): each object, field and typed canonical value, in key order.
    ///
    /// # Errors
    ///
    /// [`qsl_eval::simulation::EncodingRefusal`] when a value has no typed
    /// canonical form.
    pub fn key(
        &self,
        system: &ModelSystem<'_>,
    ) -> Result<Vec<u8>, qsl_eval::simulation::EncodingRefusal> {
        let mut bytes = Vec::new();
        for ((object, field), value) in &self.entries {
            for part in [
                object.population.package.as_bytes(),
                object.population.node.as_bytes(),
                object.key.as_bytes(),
                field.as_bytes(),
                &system.value_key(value)?,
            ] {
                bytes.extend_from_slice(
                    &u64::try_from(part.len()).unwrap_or(u64::MAX).to_be_bytes(),
                );
                bytes.extend_from_slice(part);
            }
        }
        Ok(bytes)
    }

    /// These values as a row reads them beside the observation of `state`:
    /// by the reference of the object that holds each.
    ///
    /// # Errors
    ///
    /// The fault code when a value names an object `state` does not hold.
    pub(super) fn environment(
        &self,
        system: &ModelSystem<'_>,
        state: &ModelState,
    ) -> Result<BTreeMap<(ObjectReference, String), Value>, qsl_foundation::CatalogCode> {
        self.entries
            .iter()
            .map(|((object, field), value)| {
                let held = state
                    .populations
                    .get(&object.population)
                    .and_then(|objects| objects.get(&object.key))
                    .ok_or(BROKEN_INVARIANT)?;
                let reference = system
                    .object_reference(&held.type_identity, &object.key)
                    .map_err(|_| BROKEN_INVARIANT)?;
                Ok(((reference, field.clone()), value.clone()))
            })
            .collect()
    }
}

/// The history values `state` starts with: every object of a type with a
/// history row holds the row's initial literal.
pub fn initial_history(
    refinement: &CheckedRefinement,
    concrete_system: &ModelSystem<'_>,
    state: &ModelState,
) -> HistoryValues {
    carried(
        refinement,
        concrete_system,
        &HistoryValues::default(),
        state,
    )
}

/// The values of `state`'s objects: those `kept` holds, and the initial
/// literal for an object it does not know. An object `state` lacks loses its
/// values.
fn carried(
    refinement: &CheckedRefinement,
    system: &ModelSystem<'_>,
    kept: &HistoryValues,
    state: &ModelState,
) -> HistoryValues {
    let mut entries = BTreeMap::new();
    for (population, objects) in &state.populations {
        for (key, object) in objects {
            let at = ObjectKey {
                population: population.clone(),
                key: key.clone(),
            };
            for row in refinement.history() {
                if !system.type_conforms(&object.type_identity, &row.object_type) {
                    continue;
                }
                let value = kept
                    .get(&at, &row.field)
                    .cloned()
                    .unwrap_or_else(|| row.initial.clone());
                entries.insert((at.clone(), row.field.clone()), value);
            }
        }
    }
    HistoryValues { entries }
}

/// What advancing the history over one step gave: the values, and the first
/// failure of each kind.
pub(super) struct Advanced {
    /// The history at the post-state. An update that failed wrote nothing:
    /// its field holds the value the step started from, which no row may read.
    pub(super) values: HistoryValues,
    /// The history fields, by object, whose update failed (undefined, refused,
    /// incomplete or out of type). A post-state row that reads one is not
    /// evaluated (FR-141 "Undetermined history update").
    pub(super) failed: BTreeSet<(ObjectKey, String)>,
    /// The updates that evaluated undefined or could not be decided.
    pub(super) outcomes: Outcomes,
}

/// The operation `transition` applies is the one `row` has an update for.
fn update_of<'r>(
    row: &'r HistoryRow,
    transition: &ModelTransition,
) -> Result<Option<&'r qsl_semantics::check::HistoryUpdate>, qsl_foundation::CatalogCode> {
    for update in &row.updates {
        let text = operation_identity_text(&update.operation).map_err(|stop| stop.cause)?;
        if text == transition.operation() {
            return Ok(Some(update));
        }
    }
    Ok(None)
}

/// Whether some history row has an update for the operation `transition`
/// applies, which is when a step must be resolved to its receiver and
/// arguments.
pub(super) fn has_update(
    refinement: &CheckedRefinement,
    transition: &ModelTransition,
) -> Result<bool, qsl_foundation::CatalogCode> {
    for row in refinement.history() {
        if update_of(row, transition)?.is_some() {
            return Ok(true);
        }
    }
    Ok(false)
}

/// The history at `post` after the step `step` (a transition with the
/// receiver and arguments it applied) took the concrete state `pre` to it,
/// or only the carried values for the terminal stutter step (`None`).
///
/// # Errors
///
/// [`MappingFailure::Fault`] at once for an internal fault. An update that is
/// undefined, refused or incomplete is in [`Advanced::outcomes`], not an
/// error: the step stays a step of the behaviour (AX-3).
#[allow(
    clippy::result_large_err,
    reason = "FR-140 fixes MappingFailure's shape: a row, an object and the evaluator's cause"
)]
pub(super) fn advance(
    refinement: &CheckedRefinement,
    system: &ModelSystem<'_>,
    history: &HistoryValues,
    (pre, post): (&ModelState, &ModelSimState),
    step: Option<(&ModelTransition, &ResolvedStep)>,
) -> Result<Advanced, MappingFailure> {
    let mut values = carried(refinement, system, history, post.state());
    let mut outcomes = Outcomes::default();
    let mut failed = BTreeSet::new();
    let Some((transition, resolved)) = step else {
        return Ok(Advanced {
            values,
            failed,
            outcomes,
        });
    };
    let Some(receiver) = object_of(system, post.state(), &resolved.receiver) else {
        // The step deleted its receiver, so there is no field to write.
        return Ok(Advanced {
            values,
            failed,
            outcomes,
        });
    };
    let Some(receiver_type) = post
        .state()
        .populations
        .get(&receiver.population)
        .and_then(|objects| objects.get(&receiver.key))
        .map(|object| object.type_identity.clone())
    else {
        return Err(MappingFailure::Fault(BROKEN_INVARIANT));
    };
    let fault = |stop: qsl_eval::simulation::ExpansionStop| MappingFailure::Fault(stop.cause);
    let observed_pre = system
        .observe_with_history(
            pre,
            history
                .environment(system, pre)
                .map_err(MappingFailure::Fault)?,
        )
        .map_err(fault)?;
    let observed_post = system
        .observe_with_history(
            post.state(),
            values
                .environment(system, post.state())
                .map_err(MappingFailure::Fault)?,
        )
        .map_err(fault)?;
    let mut bindings = vec![Value::Reference(resolved.receiver.clone())];
    bindings.extend(resolved.arguments.iter().cloned());
    // Every update reads the values the step started from; the writes land
    // together.
    let mut written = Vec::new();
    for row in refinement.history() {
        if !system.type_conforms(&receiver_type, &row.object_type) {
            continue;
        }
        let Some(update) = update_of(row, transition).map_err(MappingFailure::Fault)? else {
            continue;
        };
        let at = MappingRowRef::History {
            object_type: row.object_type.clone(),
            field: row.field.clone(),
            operation: update.operation.clone(),
        };
        let reading = system
            .read_history_update(
                &update.expression,
                (&observed_pre, &observed_post),
                bindings.clone(),
            )
            .map_err(fault)?;
        match reading {
            RowReading::Value(value) => {
                if row.declared.admits(&value) {
                    written.push((row.field.clone(), value));
                } else {
                    failed.insert((receiver.clone(), row.field.clone()));
                    outcomes.record(MappingFailure::Undetermined(MappingUndetermined {
                        row: at,
                        object: Some(receiver.clone()),
                        cause: EvalFailure::OutOfType,
                    }));
                }
            }
            RowReading::Undefined(cause) => {
                failed.insert((receiver.clone(), row.field.clone()));
                outcomes.record(MappingFailure::Undefined {
                    row: at,
                    object: Some(receiver.clone()),
                    cause: Box::new(cause),
                });
            }
            RowReading::Undetermined(outcome) => {
                failed.insert((receiver.clone(), row.field.clone()));
                outcomes.record(MappingFailure::Undetermined(MappingUndetermined {
                    row: at,
                    object: Some(receiver.clone()),
                    cause: EvalFailure::Evaluation(Box::new(outcome)),
                }));
            }
        }
    }
    for (field, value) in written {
        values.entries.insert((receiver.clone(), field), value);
    }
    Ok(Advanced {
        values,
        failed,
        outcomes,
    })
}

/// FR-138: the history at `post`, after the concrete step `transition` took
/// the state `pre`, whose history is `history`, to it.
///
/// # Errors
///
/// [`MappingFailure::Undefined`] naming the first update that evaluates
/// undefined, its object, and the evaluator's cause; otherwise
/// [`MappingFailure::Undetermined`] for the first that is refused,
/// incomplete or leaves its declared type; [`MappingFailure::Fault`] at once
/// for an internal fault, and for a step the concrete system cannot resolve.
/// The step stays a step of the behaviour either way.
#[allow(
    clippy::result_large_err,
    reason = "FR-140 fixes MappingFailure's shape: a row, an object and the evaluator's cause"
)]
pub fn step_history(
    refinement: &CheckedRefinement,
    concrete_system: &ModelSystem<'_>,
    history: &HistoryValues,
    pre: &ModelState,
    transition: &ModelTransition,
    post: &ModelSimState,
) -> Result<HistoryValues, MappingFailure> {
    let resolved = concrete_system
        .resolve_step(pre, transition, post)
        .map_err(|stop| MappingFailure::Fault(stop.cause))?
        .ok_or(MappingFailure::Fault(BROKEN_INVARIANT))?;
    let advanced = advance(
        refinement,
        concrete_system,
        history,
        (pre, post),
        Some((transition, &resolved)),
    )?;
    match advanced.outcomes.first() {
        Some(failure) => Err(failure),
        None => Ok(advanced.values),
    }
}
