// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-140 (ADR-020 RM-2, RM-3, RM-7): `map(s)`, the abstract state a
//! concrete state maps to.
//!
//! Each abstract population is the key-preserving image of its source
//! population. Each image object has the abstract type whose object row names
//! the concrete object's most-specific type, and each visible field is the
//! value of its row, evaluated by the one clause evaluator over the concrete
//! state's synthesized observation with `self` bound to the concrete object
//! and a fresh meter per evaluation. A reference value maps to the reference
//! with the same key.

use std::collections::BTreeMap;

use qsl_eval::simulation::model::{ClauseOutcome, ModelSystem, RowReading, StateObservation};
use qsl_eval::temporal::UndefinedCause;
use qsl_foundation::CatalogCode;
use qsl_semantics::check::{CheckedRefinement, FieldRow, ObjectRow};
use qsl_semantics::model::key::DeclarationKey;
use qsl_semantics::model::state::{ModelState, StateObject};
use quire_exact::{ObjectReference, Value};

/// An object of a population, by the population and the object's key.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ObjectKey {
    /// The object's population.
    pub population: DeclarationKey,
    /// The object's key in that population.
    pub key: String,
}

/// The mapping row a failure is about (FR-140 `MappingRowRef`).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MappingRowRef {
    /// A visible field row of an object row.
    Field {
        /// The abstract object type the row is of.
        object_type: DeclarationKey,
        /// The abstract field the row defines.
        field: DeclarationKey,
    },
    /// The object map itself: the object row of a concrete object's
    /// most-specific type, which names the abstract type it maps to.
    Object {
        /// The concrete most-specific type.
        concrete_type: DeclarationKey,
    },
}

/// An abstract field a refinement leaves open, on one abstract object
/// (ADR-020 AX-2).
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct HiddenSlot {
    /// The abstract object.
    pub object: ObjectKey,
    /// The abstract field no row defines.
    pub field: DeclarationKey,
}

/// A mapping feature this build has not built.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MappingFeature {
    /// A reference nested inside an option, record, union or collection
    /// value, which reference lifting (RM-3) does not yet rewrite.
    NestedReference,
    /// An abstract field declared `?`, which FR-140 gives no rule for.
    OptionalField,
}

/// Why a row evaluation gave no value and no refutation (ADR-020 RE-4).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EvalFailure {
    /// The evaluator refused the row or ran out of meter.
    Evaluation(Box<ClauseOutcome>),
    /// The row's value needs a mapping feature this build has not built.
    Unsupported(MappingFeature),
    /// No object row names the concrete object's most-specific type.
    UnmappedType(DeclarationKey),
    /// The evaluator stopped with a broken invariant, with this code.
    Stopped(CatalogCode),
}

/// A mapping row that is refused or incomplete (FR-140 `MappingUndetermined`).
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MappingUndetermined {
    /// The row the mapping could not evaluate.
    pub row: MappingRowRef,
    /// The concrete object it was evaluated for.
    pub object: Option<ObjectKey>,
    /// Why.
    pub cause: EvalFailure,
}

/// Why `map_state` returned no state (FR-140).
#[derive(Clone, Debug)]
pub enum MappingFailure {
    /// A row evaluated undefined: the refinement is refuted at this state
    /// (ADR-020 RE-5).
    Undefined {
        /// The row.
        row: MappingRowRef,
        /// The concrete object it was evaluated for.
        object: Option<ObjectKey>,
        /// The evaluator's undefined cause.
        cause: Box<UndefinedCause>,
    },
    /// A row was refused or incomplete: the mapping is undetermined here
    /// (ADR-020 RE-4).
    Undetermined(MappingUndetermined),
}

/// The abstract state a concrete state maps to.
#[derive(Clone, Debug)]
pub struct MappedState {
    /// The abstract state with every hidden field unset.
    pub visible: ModelState,
    /// The abstract fields left open, in (object, field) order.
    pub hidden: Vec<HiddenSlot>,
}

/// FR-140: `map(state)`, the abstract state that the concrete `state` of
/// `concrete_system` maps to under `refinement`, built in `abstract_system`'s
/// state model. A function of its inputs only.
///
/// `concrete_system` is a parameter beside FR-140's listed inputs: the rows
/// are evaluated by the concrete package's clause evaluator over the concrete
/// state's observation, which only the system holds.
///
/// # Errors
///
/// [`MappingFailure::Undefined`] for the first row that evaluates undefined,
/// and [`MappingFailure::Undetermined`] for the first that is refused or
/// incomplete, in population-row, object-key, field-row order; neither
/// returns a partial state.
#[allow(
    clippy::result_large_err,
    reason = "FR-140 fixes MappingFailure's shape: a row, an object and the evaluator's cause"
)]
pub fn map_state(
    refinement: &CheckedRefinement,
    concrete_system: &ModelSystem<'_>,
    abstract_system: &ModelSystem<'_>,
    state: &ModelState,
) -> Result<MappedState, MappingFailure> {
    let mut visible = ModelState {
        populations: BTreeMap::new(),
        extents: Vec::new(),
    };
    let mut hidden = Vec::new();
    // The concrete state's observation, built once for every row read.
    let mut observed: Option<StateObservation> = None;
    for row in refinement.populations() {
        let mut image = BTreeMap::new();
        if let Some(objects) = state.populations.get(&row.concrete_population) {
            for (key, object) in objects {
                let at = ObjectKey {
                    population: row.concrete_population.clone(),
                    key: key.clone(),
                };
                let Some(object_row) = refinement
                    .objects()
                    .iter()
                    .find(|candidate| candidate.concrete_type == object.type_identity)
                else {
                    return Err(unmapped(&object.type_identity, at));
                };
                if observed.is_none() {
                    observed = Some(concrete_system.observe(state).map_err(|stop| {
                        object_failure(
                            &object.type_identity,
                            at.clone(),
                            EvalFailure::Stopped(stop.cause),
                        )
                    })?);
                }
                let Some(observed) = observed.as_ref() else {
                    continue;
                };
                let mapped = map_object(
                    refinement,
                    concrete_system,
                    abstract_system,
                    observed,
                    object_row,
                    &at,
                )?;
                for field in &object_row.hidden {
                    hidden.push(HiddenSlot {
                        object: ObjectKey {
                            population: row.abstract_population.clone(),
                            key: key.clone(),
                        },
                        field: field.clone(),
                    });
                }
                image.insert(key.clone(), mapped);
            }
        }
        visible
            .populations
            .insert(row.abstract_population.clone(), image);
    }
    hidden.sort();
    Ok(MappedState { visible, hidden })
}

/// The failure of a concrete object whose type no object row maps. S3
/// requires an object row for each abstract type, not for each concrete
/// subtype a source population may hold, so the case is reported as a row
/// that cannot be evaluated, never as a dropped object.
fn unmapped(concrete_type: &DeclarationKey, at: ObjectKey) -> MappingFailure {
    object_failure(
        concrete_type,
        at,
        EvalFailure::UnmappedType(concrete_type.clone()),
    )
}

fn object_failure(
    concrete_type: &DeclarationKey,
    at: ObjectKey,
    cause: EvalFailure,
) -> MappingFailure {
    MappingFailure::Undetermined(MappingUndetermined {
        row: MappingRowRef::Object {
            concrete_type: concrete_type.clone(),
        },
        object: Some(at),
        cause,
    })
}

/// The abstract object that the concrete object at `at` maps to.
#[allow(
    clippy::result_large_err,
    reason = "FR-140 fixes MappingFailure's shape: a row, an object and the evaluator's cause"
)]
fn map_object(
    refinement: &CheckedRefinement,
    concrete_system: &ModelSystem<'_>,
    abstract_system: &ModelSystem<'_>,
    observed: &StateObservation,
    object_row: &ObjectRow,
    at: &ObjectKey,
) -> Result<StateObject, MappingFailure> {
    let stopped = |row: &FieldRow, cause: EvalFailure| {
        MappingFailure::Undetermined(MappingUndetermined {
            row: field_ref(object_row, row),
            object: Some(at.clone()),
            cause,
        })
    };
    let receiver = concrete_system
        .object_reference(&object_row.concrete_type, &at.key)
        .map_err(|_| {
            object_failure(
                &object_row.concrete_type,
                at.clone(),
                EvalFailure::Stopped(BROKEN_INVARIANT),
            )
        })?;
    let declared = abstract_system.fields_of(&object_row.abstract_type);
    let mut fields = BTreeMap::new();
    for row in &object_row.fields {
        let reading = concrete_system
            .read_row(
                &row.expression,
                observed,
                vec![Value::Reference(receiver.clone())],
            )
            .map_err(|stop| stopped(row, EvalFailure::Stopped(stop.cause)))?;
        let value = match reading {
            RowReading::Value(value) => value,
            RowReading::Undefined(cause) => {
                return Err(MappingFailure::Undefined {
                    row: field_ref(object_row, row),
                    object: Some(at.clone()),
                    cause: Box::new(cause),
                })
            }
            RowReading::Undetermined(outcome) => {
                return Err(stopped(row, EvalFailure::Evaluation(Box::new(outcome))))
            }
        };
        // FR-140 states no rule for a `?` field, whose row has type
        // `Option<T>` while a state holds the bare value or nothing, so it
        // is refused rather than guessed.
        if declared
            .and_then(|fields| fields.iter().find(|field| field.key == row.field))
            .is_some_and(|field| field.optional)
        {
            return Err(stopped(
                row,
                EvalFailure::Unsupported(MappingFeature::OptionalField),
            ));
        }
        let lifted = lift(refinement, concrete_system, abstract_system, value)
            .map_err(|cause| stopped(row, cause))?;
        fields.insert(row.field.clone(), lifted);
    }
    Ok(StateObject {
        type_identity: object_row.abstract_type.clone(),
        fields,
    })
}

/// The `Stopped` code of a broken invariant: the concrete state names an
/// object its own system cannot reference.
const BROKEN_INVARIANT: CatalogCode =
    CatalogCode::new("runtime_invariant", "established-invariant-broken");

fn field_ref(object_row: &ObjectRow, row: &FieldRow) -> MappingRowRef {
    MappingRowRef::Field {
        object_type: object_row.abstract_type.clone(),
        field: row.field.clone(),
    }
}

/// `value` with a concrete reference replaced by the abstract reference with
/// the same key (RM-3). A reference nested inside another value is not yet
/// rewritten, and is refused.
fn lift(
    refinement: &CheckedRefinement,
    concrete_system: &ModelSystem<'_>,
    abstract_system: &ModelSystem<'_>,
    value: Value,
) -> Result<Value, EvalFailure> {
    match value {
        Value::Reference(reference) => {
            lift_reference(refinement, concrete_system, abstract_system, &reference)
                .map(Value::Reference)
        }
        nested if holds_reference(&nested) => {
            Err(EvalFailure::Unsupported(MappingFeature::NestedReference))
        }
        plain => Ok(plain),
    }
}

fn lift_reference(
    refinement: &CheckedRefinement,
    concrete_system: &ModelSystem<'_>,
    abstract_system: &ModelSystem<'_>,
    reference: &ObjectReference,
) -> Result<ObjectReference, EvalFailure> {
    let concrete_type = concrete_system
        .type_key_of(reference)
        .ok_or(EvalFailure::Stopped(BROKEN_INVARIANT))?;
    let row = refinement
        .objects()
        .iter()
        .find(|row| row.concrete_type == *concrete_type)
        .ok_or_else(|| EvalFailure::UnmappedType(concrete_type.clone()))?;
    abstract_system
        .object_reference(&row.abstract_type, reference.object().as_str())
        .map_err(|_| EvalFailure::Stopped(BROKEN_INVARIANT))
}

/// Whether `value` holds an object reference anywhere inside it.
fn holds_reference(value: &Value) -> bool {
    match value {
        Value::Reference(_) => true,
        Value::Option(option) => option.payload().is_some_and(holds_reference),
        Value::Composite(composite) => composite.slots().iter().any(|slot| match slot {
            quire_exact::FieldValue::Present(inner) => holds_reference(inner),
            quire_exact::FieldValue::Absent | quire_exact::FieldValue::Null => false,
        }),
        Value::Union(union) => union.payload().iter().any(holds_reference),
        Value::Collection(collection) => collection.elements().iter().any(holds_reference),
        Value::Boolean(_)
        | Value::Integer(_)
        | Value::Rational(_)
        | Value::Decimal(_)
        | Value::Float(_)
        | Value::Quantity(_)
        | Value::Text(_)
        | Value::String(_)
        | Value::Uuid(_)
        | Value::Timestamp(_)
        | Value::Enum(_)
        | Value::Population(_) => false,
    }
}
