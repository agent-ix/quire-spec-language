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
//! with the same key, and a whole-population reference row maps element by
//! element.
//!
//! The rows are read in an order that does not depend on how they were
//! written: populations by abstract declaration, objects by ascending key, and
//! per object its object row, then its field rows by ascending field name as
//! UTF-8 bytes. Every row is read, because an internal fault in any of them
//! outranks every row outcome; of the row outcomes the first undefined one
//! outranks the first undetermined one (QSpec FR-377 "Failure precedence").

use std::collections::BTreeMap;

use qsl_eval::simulation::model::{ClauseOutcome, ModelSystem, RowReading, StateObservation};
use qsl_eval::temporal::UndefinedCause;
use qsl_foundation::CatalogCode;
use qsl_semantics::check::{CheckedRefinement, FieldRow, ObjectRow, PopulationRow};
use qsl_semantics::model::key::DeclarationKey;
use qsl_semantics::model::state::{ModelState, StateField, StateObject};
use quire_exact::{CollectionKind, ObjectReference, Value, ValueType};

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
    /// An expression of a `step` row, evaluated over the concrete pre-state.
    Argument {
        /// The concrete operation the row is for.
        operation: DeclarationKey,
        /// The expression.
        argument: StepArgument,
    },
}

/// Which expression of an abstract-operation `step` row.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StepArgument {
    /// The receiver.
    Receiver,
    /// The abstract parameter at this zero-based position; a `_` argument
    /// has no expression.
    Parameter(usize),
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

/// Why a row evaluation gave no value and no refutation (ADR-020 RE-4).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EvalFailure {
    /// The evaluator refused the row or ran out of meter.
    Evaluation(Box<ClauseOutcome>),
    /// No object row names the concrete object's most-specific type.
    UnmappedType(DeclarationKey),
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
    /// A system that is not the refinement's own was passed:
    /// `invalid_runtime_input`/`wrong-role-mapping`. It is not a verdict;
    /// the caller passed the wrong system, an internal fault of the calling
    /// code (FR-141).
    WrongSystem(CatalogCode),
    /// The evaluator hit an internal fault, with this catalog code. It is
    /// no row's outcome and takes no part in the failure order; the item
    /// settles `failed` (QSpec FR-383).
    Fault(CatalogCode),
}

/// The abstract state a concrete state maps to.
#[derive(Clone, Debug)]
pub struct MappedState {
    /// The abstract state with every hidden field unset.
    pub visible: ModelState,
    /// The abstract fields left open, in (object, field) order.
    pub hidden: Vec<HiddenSlot>,
}

/// `invalid_runtime_input`/`wrong-role-mapping`: a system over another model
/// than the refinement's.
const WRONG_ROLE_MAPPING: CatalogCode =
    CatalogCode::new("invalid_runtime_input", "wrong-role-mapping");

/// `runtime_invariant`/`established-invariant-broken`: the concrete state
/// names an object its own system cannot reference.
const BROKEN_INVARIANT: CatalogCode =
    CatalogCode::new("runtime_invariant", "established-invariant-broken");

/// The first row outcome of each kind, in reading order.
#[derive(Default)]
struct Outcomes {
    undefined: Option<MappingFailure>,
    undetermined: Option<MappingFailure>,
}

impl Outcomes {
    /// Keep `failure` when no earlier one of its kind was kept.
    fn record(&mut self, failure: MappingFailure) {
        let slot = match failure {
            MappingFailure::Undefined { .. } => &mut self.undefined,
            MappingFailure::Undetermined(_) => &mut self.undetermined,
            MappingFailure::WrongSystem(_) | MappingFailure::Fault(_) => return,
        };
        slot.get_or_insert(failure);
    }

    /// The failure that outranks the rest: the first undefined row, else the
    /// first undetermined one.
    fn first(self) -> Option<MappingFailure> {
        self.undefined.or(self.undetermined)
    }
}

/// FR-140: `map(state)`, the abstract state that the concrete `state` of
/// `concrete_system` maps to under `refinement`, built in `abstract_system`'s
/// state model. A function of its inputs only.
///
/// The rows are evaluated by the concrete package's clause evaluator over the
/// concrete state's observation, which only `concrete_system` holds, so it is
/// a parameter beside FR-140's listed inputs. History values (FR-138) are not
/// an input yet: a refinement with a history row is refused at S3.
///
/// # Errors
///
/// [`MappingFailure::WrongSystem`] before any row is read when a system is
/// not over its model; [`MappingFailure::Fault`] at once for an internal
/// fault; otherwise [`MappingFailure::Undefined`] for the first row that
/// evaluates undefined, else [`MappingFailure::Undetermined`] for the first
/// that is refused or incomplete, in the reading order above. Neither
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
    check_systems(refinement, concrete_system, abstract_system)?;
    let mut populations: Vec<&PopulationRow> = refinement.populations().iter().collect();
    populations.sort_by(|left, right| left.abstract_population.cmp(&right.abstract_population));
    let mut visible = ModelState {
        populations: BTreeMap::new(),
        extents: Vec::new(),
    };
    let mut hidden = Vec::new();
    let mut outcomes = Outcomes::default();
    // The concrete state's observation, built once for every row read.
    let mut observed: Option<StateObservation> = None;
    for row in populations {
        let mut image = BTreeMap::new();
        for (key, object) in state
            .populations
            .get(&row.concrete_population)
            .into_iter()
            .flatten()
        {
            let at = ObjectKey {
                population: row.concrete_population.clone(),
                key: key.clone(),
            };
            let Some(object_row) = refinement
                .objects()
                .iter()
                .find(|candidate| candidate.concrete_type == object.type_identity)
            else {
                outcomes.record(unmapped(&object.type_identity, at));
                continue;
            };
            let observation = match &observed {
                Some(observation) => observation,
                None => observed.insert(
                    concrete_system
                        .observe(state)
                        .map_err(|stop| MappingFailure::Fault(stop.cause))?,
                ),
            };
            if let Some(mapped) = map_object(
                refinement,
                concrete_system,
                abstract_system,
                observation,
                object_row,
                &at,
                &mut outcomes,
            )? {
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
    if let Some(failure) = outcomes.first() {
        return Err(failure);
    }
    hidden.sort();
    Ok(MappedState { visible, hidden })
}

/// The identities of the two domain packages `refinement` maps between,
/// concrete first, read off the declarations its rows name; `None` for a
/// refinement with no population or object row.
pub(crate) fn subject_models(refinement: &CheckedRefinement) -> Option<(&str, &str)> {
    if let Some(row) = refinement.populations().first() {
        return Some((
            &row.concrete_population.package,
            &row.abstract_population.package,
        ));
    }
    refinement
        .objects()
        .first()
        .map(|row| (&*row.concrete_type.package, &*row.abstract_type.package))
}

/// Check that `concrete_system` is over the refinement's concrete model and
/// `abstract_system` over its abstract one.
#[allow(
    clippy::result_large_err,
    reason = "FR-140 fixes MappingFailure's shape: a row, an object and the evaluator's cause"
)]
fn check_systems(
    refinement: &CheckedRefinement,
    concrete_system: &ModelSystem<'_>,
    abstract_system: &ModelSystem<'_>,
) -> Result<(), MappingFailure> {
    let over = |system: &ModelSystem<'_>, model: &str| system.model() == Some(model);
    match subject_models(refinement) {
        Some((concrete, abstracted))
            if !(over(concrete_system, concrete) && over(abstract_system, abstracted)) =>
        {
            Err(MappingFailure::WrongSystem(WRONG_ROLE_MAPPING))
        }
        _ => Ok(()),
    }
}

/// The failure of a concrete object whose type no object row maps. S3
/// requires an object row for each abstract type, not for each concrete
/// subtype a source population may hold, so the case is reported as a row
/// that cannot be evaluated, never as a dropped object.
fn unmapped(concrete_type: &DeclarationKey, at: ObjectKey) -> MappingFailure {
    MappingFailure::Undetermined(MappingUndetermined {
        row: MappingRowRef::Object {
            concrete_type: concrete_type.clone(),
        },
        object: Some(at),
        cause: EvalFailure::UnmappedType(concrete_type.clone()),
    })
}

/// The abstract object that the concrete object at `at` maps to, or `None`
/// once a row of it is undefined or undetermined (recorded in `outcomes`).
///
/// # Errors
///
/// [`MappingFailure::Fault`] at once for an internal fault.
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
    outcomes: &mut Outcomes,
) -> Result<Option<StateObject>, MappingFailure> {
    let receiver = concrete_system
        .object_reference(&object_row.concrete_type, &at.key)
        .map_err(|_| MappingFailure::Fault(BROKEN_INVARIANT))?;
    let declared = abstract_system
        .fields_of(&object_row.abstract_type)
        .ok_or(MappingFailure::Fault(BROKEN_INVARIANT))?;
    let mut rows: Vec<(&FieldRow, &StateField)> = object_row
        .fields
        .iter()
        .map(|row| {
            declared
                .iter()
                .find(|field| field.key == row.field)
                .map(|field| (row, field))
                .ok_or(MappingFailure::Fault(BROKEN_INVARIANT))
        })
        .collect::<Result<_, _>>()?;
    rows.sort_by(|left, right| left.1.name.as_bytes().cmp(right.1.name.as_bytes()));
    let mut fields = BTreeMap::new();
    let mut mapped = true;
    for (row, field) in rows {
        let reading = concrete_system
            .read_row(
                &row.expression,
                observed,
                vec![Value::Reference(receiver.clone())],
            )
            .map_err(|stop| MappingFailure::Fault(stop.cause))?;
        let value = match reading {
            RowReading::Value(value) => value,
            RowReading::Undefined(cause) => {
                outcomes.record(MappingFailure::Undefined {
                    row: field_ref(object_row, row),
                    object: Some(at.clone()),
                    cause: Box::new(cause),
                });
                mapped = false;
                continue;
            }
            RowReading::Undetermined(outcome) => {
                outcomes.record(undetermined(
                    object_row,
                    row,
                    at,
                    EvalFailure::Evaluation(Box::new(outcome)),
                ));
                mapped = false;
                continue;
            }
        };
        match lift_field(refinement, concrete_system, abstract_system, field, value) {
            Ok(Some(lifted)) => {
                fields.insert(row.field.clone(), lifted);
            }
            Ok(None) => {}
            Err(Lifting::Unmapped(concrete_type)) => {
                outcomes.record(undetermined(
                    object_row,
                    row,
                    at,
                    EvalFailure::UnmappedType(concrete_type),
                ));
                mapped = false;
            }
            Err(Lifting::Fault) => return Err(MappingFailure::Fault(BROKEN_INVARIANT)),
        }
    }
    Ok(mapped.then(|| StateObject {
        type_identity: object_row.abstract_type.clone(),
        fields,
    }))
}

fn field_ref(object_row: &ObjectRow, row: &FieldRow) -> MappingRowRef {
    MappingRowRef::Field {
        object_type: object_row.abstract_type.clone(),
        field: row.field.clone(),
    }
}

fn undetermined(
    object_row: &ObjectRow,
    row: &FieldRow,
    at: &ObjectKey,
    cause: EvalFailure,
) -> MappingFailure {
    MappingFailure::Undetermined(MappingUndetermined {
        row: field_ref(object_row, row),
        object: Some(at.clone()),
        cause,
    })
}

/// Why a row's value could not be lifted into the abstract model.
pub(super) enum Lifting {
    /// A reference names an object whose most-specific type no object row
    /// names.
    Unmapped(DeclarationKey),
    /// The concrete state names an object its own system cannot reference.
    Fault,
}

/// What `field` of the abstract model takes from the row value `value`
/// (RM-3, RM-7): `None` leaves the field unset.
///
/// A field declared `T?` takes `v` from `Some(v)` and stays unset for `none`;
/// a field declared `Option<T>` takes the option value unchanged, so `none`
/// stays an explicit `none`. A reference maps to the abstract reference with
/// the same key, and a whole-population reference row (`Set<Reference<T>>`)
/// maps element by element. A reference nested in any other value is not
/// lifted: FR-135 refuses a row whose type nests one.
fn lift_field(
    refinement: &CheckedRefinement,
    concrete_system: &ModelSystem<'_>,
    abstract_system: &ModelSystem<'_>,
    field: &StateField,
    value: Value,
) -> Result<Option<Value>, Lifting> {
    let Some(value) = unwrap_optional(field, value) else {
        return Ok(None);
    };
    match value {
        Value::Reference(reference) => {
            lift_reference(refinement, concrete_system, abstract_system, &reference)
                .map(|lifted| Some(Value::Reference(lifted)))
        }
        Value::Collection(collection)
            if collection.collection_type().kind() == CollectionKind::Set
                && collection
                    .elements()
                    .iter()
                    .all(|element| matches!(element, Value::Reference(_))) =>
        {
            let ValueType::Collection(declared) = &field.value_type else {
                return Err(Lifting::Fault);
            };
            let elements = collection
                .elements()
                .iter()
                .map(|element| match element {
                    Value::Reference(reference) => {
                        lift_reference(refinement, concrete_system, abstract_system, reference)
                            .map(Value::Reference)
                    }
                    _ => Err(Lifting::Fault),
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(Some(quire_exact::from_admitted(
                (**declared).clone(),
                elements,
            )))
        }
        plain => Ok(Some(plain)),
    }
}

/// What `field` takes from the row value `value` before lifting: a field
/// declared `T?` takes `v` from `Some(v)` and stays unset (`None`) for
/// `none`; any other field takes the value unchanged, so a field declared
/// `Option<T>` keeps an explicit `none`.
fn unwrap_optional(field: &StateField, value: Value) -> Option<Value> {
    match value {
        Value::Option(option) if field.optional => option.payload().cloned(),
        other => Some(other),
    }
}

fn lift_reference(
    refinement: &CheckedRefinement,
    concrete_system: &ModelSystem<'_>,
    abstract_system: &ModelSystem<'_>,
    reference: &ObjectReference,
) -> Result<ObjectReference, Lifting> {
    let concrete_type = concrete_system
        .type_key_of(reference)
        .ok_or(Lifting::Fault)?;
    let row = refinement
        .objects()
        .iter()
        .find(|row| row.concrete_type == *concrete_type)
        .ok_or_else(|| Lifting::Unmapped(concrete_type.clone()))?;
    abstract_system
        .object_reference(&row.abstract_type, reference.object().as_str())
        .map_err(|_| Lifting::Fault)
}

/// `value` as the abstract model reads it: a reference maps to the abstract
/// reference with the same key (RM-3), any other value is unchanged. Used for
/// the receiver and arguments of an abstract-operation `step` row.
pub(super) fn lift_value(
    refinement: &CheckedRefinement,
    concrete_system: &ModelSystem<'_>,
    abstract_system: &ModelSystem<'_>,
    value: Value,
) -> Result<Value, Lifting> {
    match value {
        Value::Reference(reference) => {
            lift_reference(refinement, concrete_system, abstract_system, &reference)
                .map(Value::Reference)
        }
        plain => Ok(plain),
    }
}

#[cfg(test)]
mod tests {
    use ix_trace_rs::trace;

    use super::*;

    fn field(optional: bool) -> StateField {
        StateField {
            key: DeclarationKey {
                package: "p".to_owned(),
                node: "head".to_owned(),
            },
            name: "head".to_owned(),
            value_type: ValueType::Boolean,
            optional,
        }
    }

    /// TC-1086 step 8: a field declared `T?` is unset for
    /// `none` and takes `v` from `Some(v)`; a field declared `Option<T>`
    /// keeps the option value unchanged, so `none` stays an explicit `none`.
    /// The constructors `none` and `Some(e)` are not yet a spelling of a
    /// refinement row (S2), so the rule is exercised on the values.
    #[trace("TC-1086")]
    #[test]
    fn an_optional_field_is_unset_for_none_and_takes_the_payload_of_some() {
        let none = quire_exact::OptionValue::none(ValueType::Boolean);
        let some = quire_exact::OptionValue::present(ValueType::Boolean, Value::Boolean(true))
            .expect("a present option");
        assert!(unwrap_optional(&field(true), none.clone()).is_none());
        assert!(matches!(
            unwrap_optional(&field(true), some.clone()),
            Some(Value::Boolean(true))
        ));
        assert!(matches!(
            unwrap_optional(&field(false), none),
            Some(Value::Option(_))
        ));
        assert!(matches!(
            unwrap_optional(&field(false), some),
            Some(Value::Option(_))
        ));
    }

    fn undefined(field: &str) -> MappingFailure {
        MappingFailure::Undefined {
            row: MappingRowRef::Object {
                concrete_type: DeclarationKey {
                    package: "p".to_owned(),
                    node: field.to_owned(),
                },
            },
            object: None,
            cause: Box::new(UndefinedCause::InsufficientData(
                qsl_eval::temporal::InsufficientData {
                    count: 0,
                    min_count: 1,
                },
            )),
        }
    }

    fn undetermined(field: &str) -> MappingFailure {
        unmapped(
            &DeclarationKey {
                package: "p".to_owned(),
                node: field.to_owned(),
            },
            ObjectKey {
                population: DeclarationKey {
                    package: "p".to_owned(),
                    node: "population".to_owned(),
                },
                key: "k".to_owned(),
            },
        )
    }

    /// FR-140-AC-13 (TC-1086 step 4): of the row outcomes read, the first
    /// undefined one is reported even when an undetermined one was read
    /// first, and a later outcome of a kind never replaces an earlier one.
    #[trace("TC-1086", "FR-140-AC-13")]
    #[test]
    fn the_first_undefined_row_outranks_every_undetermined_one() {
        let mut outcomes = Outcomes::default();
        outcomes.record(undetermined("first-undetermined"));
        outcomes.record(undefined("first-undefined"));
        outcomes.record(undefined("second-undefined"));
        outcomes.record(undetermined("second-undetermined"));
        let Some(MappingFailure::Undefined {
            row: MappingRowRef::Object { concrete_type },
            ..
        }) = outcomes.first()
        else {
            panic!("the first undefined row is reported");
        };
        assert_eq!(concrete_type.node, "first-undefined");

        let mut outcomes = Outcomes::default();
        outcomes.record(undetermined("first-undetermined"));
        outcomes.record(undetermined("second-undetermined"));
        let Some(MappingFailure::Undetermined(found)) = outcomes.first() else {
            panic!("the first undetermined row is reported");
        };
        assert_eq!(
            found.cause,
            EvalFailure::UnmappedType(DeclarationKey {
                package: "p".to_owned(),
                node: "first-undetermined".to_owned()
            })
        );
    }
}
