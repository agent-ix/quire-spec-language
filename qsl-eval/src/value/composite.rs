// SPDX-License-Identifier: AGPL-3.0-or-later
//! The record and tuple operations the S6a evaluator and `qsl-jit` share:
//! the construction of a composite from its operand values, the faults of a
//! refused construction, a field read, and the projection of a stored slot
//! to the value a read gives.
//!
//! Both backends call these functions, so a record is built, a field read and
//! a refused construction faulted by one body of code; each backend only
//! decides where the operand values come from and wraps the fault
//! identifier in its own fault type.

use std::sync::Arc;

use quire_checked::node::{RecordField, RecordSlot};
use quire_exact::{CompositeValue, FieldValue, NodeKey, OptionValue, Value, ValueType};
use quire_semantic_value::declaration::{
    CompositeShape, ConstructionCause, ConstructionRefusal, EnvironmentFailure, TypeEnvironment,
};

/// Which composite constructor refused.
#[derive(Clone, Copy)]
pub enum ConstructionSite {
    /// A tuple.
    Tuple,
    /// A record.
    Record,
}

/// The fault identifier of a refused tuple or record construction: a storage
/// failure of the admission worklist has its own identifier and is never read
/// as a rejected construction.
#[must_use]
pub fn construction_fault_identifier(
    site: ConstructionSite,
    refusal: &ConstructionRefusal,
) -> &'static str {
    use ConstructionCause::{StorageCapacity, StorageDenied};
    match (site, &refusal.cause) {
        (ConstructionSite::Tuple, StorageDenied(_)) => {
            "tuple-construction-admission-allocation-denied"
        }
        (ConstructionSite::Tuple, StorageCapacity(_)) => {
            "tuple-construction-admission-capacity-overflow"
        }
        (ConstructionSite::Record, StorageDenied(_)) => {
            "record-construction-admission-allocation-denied"
        }
        (ConstructionSite::Record, StorageCapacity(_)) => {
            "record-construction-admission-capacity-overflow"
        }
        (ConstructionSite::Tuple, _) => "tuple-construction-rejected",
        (ConstructionSite::Record, _) => "record-construction-rejected",
    }
}

/// The tuple of `declaration` over `arguments`, or the fault identifier of
/// its refused construction. The caller retains the result.
///
/// # Errors
/// The fault identifier of a refused construction.
pub fn construct_tuple(
    types: &TypeEnvironment,
    declaration: &NodeKey,
    arguments: Vec<Value>,
) -> Result<Value, &'static str> {
    types
        .tuple(declaration.clone(), arguments)
        .map_err(|refusal| construction_fault_identifier(ConstructionSite::Tuple, &refusal))
}

/// The number of `slots` that hold an operand value.
#[must_use]
pub fn present_slots<C>(slots: &[RecordSlot<C>]) -> usize {
    slots
        .iter()
        .filter(|slot| matches!(slot, RecordSlot::Present(_)))
        .count()
}

/// The record of `declaration` whose slots are `slots`, the present ones
/// taking `values` in order, or the fault identifier of its refused
/// construction. The caller retains the result.
///
/// # Errors
/// The fault identifier of an unresolved declaration, a missing value or a
/// refused construction.
pub fn construct_record<C>(
    types: &TypeEnvironment,
    declaration: &NodeKey,
    slots: &[RecordSlot<C>],
    values: Vec<Value>,
) -> Result<Value, &'static str> {
    let mut values = values.into_iter();
    let Some(CompositeShape::Record(declared)) = types
        .composite(declaration.clone())
        .map(|declaration| declaration.shape())
    else {
        return Err("record-declaration-unresolved");
    };
    let mut fields = Vec::with_capacity(slots.len());
    for (slot, field) in slots.iter().zip(declared) {
        let value = match slot {
            RecordSlot::Absent => FieldValue::Absent,
            RecordSlot::Null => FieldValue::Null,
            RecordSlot::Present(_) => {
                FieldValue::Present(values.next().ok_or("record-present-value-missing")?)
            }
        };
        fields.push((field.name(), value));
    }
    types
        .record(declaration.clone(), fields)
        .map_err(|refusal| construction_fault_identifier(ConstructionSite::Record, &refusal))
}

/// A field read: the record read, the slot position and the value the read
/// gives.
pub struct FieldRead {
    /// The record the field was read from.
    pub composite: Arc<CompositeValue>,
    /// The position of the field among the record's own fields.
    pub index: usize,
    /// The value the read gives.
    pub value: Value,
}

/// Read `field` of `record`: the slot is found in the value's own
/// declaration by the field's identity (FR-359 item 2), through `slot_of`
/// (the package's `record_slot`), and projected to `value_type`.
///
/// # Errors
/// The fault identifier of a value that is no record, an unresolved slot or
/// a slot of the wrong shape.
pub fn read_field(
    types: &TypeEnvironment,
    record: Value,
    field: &RecordField,
    slot_of: impl FnOnce(NodeKey, &RecordField) -> Option<usize>,
    optional: bool,
    value_type: &ValueType,
) -> Result<FieldRead, &'static str> {
    let Value::Composite(composite) = record else {
        return Err("field-composite-value-expected");
    };
    let index = slot_of(composite.declaration().clone(), field).ok_or("field-slot-unresolved")?;
    let slot = composite
        .slots()
        .get(index)
        .ok_or("field-slot-unresolved")?;
    let value = project(types, slot, optional, value_type, false)?;
    Ok(FieldRead {
        composite,
        index,
        value,
    })
}

/// The value a read of the stored `slot` gives.
///
/// `witnessed`: the slot holds a value FR-106 check 6.5 admitted outside its
/// declared range, which an optional slot's checked construction would
/// refuse. Every other slot is checked as before.
///
/// # Errors
/// The fault identifier of a slot of the wrong shape or a payload the
/// environment cannot admit.
pub fn project(
    types: &TypeEnvironment,
    slot: &FieldValue,
    optional: bool,
    value_type: &ValueType,
    witnessed: bool,
) -> Result<Value, &'static str> {
    match (slot, optional, value_type) {
        (FieldValue::Present(value), false, _) => Ok(value.clone()),
        (FieldValue::Present(value), true, ValueType::Option(payload)) if witnessed => Ok(
            OptionValue::from_admitted((**payload).clone(), Some(value.clone())),
        ),
        // FR-359 item 5: the payload is admitted by conformance under the
        // type environment, so a child of the payload's record keeps its own
        // identity in the `Option`. `OptionValue::present` is exact.
        (FieldValue::Present(value), true, ValueType::Option(payload)) => {
            // The evaluator's stop channel is the kernel `Stop`, which has no
            // storage variant, so an undecided admission is its own fault
            // identifier and is never read as "rejected".
            match types.admits(payload, value) {
                Ok(true) => Ok(OptionValue::from_admitted(
                    (**payload).clone(),
                    Some(value.clone()),
                )),
                Ok(false) => Err("field-option-payload-rejected"),
                Err(EnvironmentFailure::Allocation(_)) => {
                    Err("field-option-payload-admission-allocation-denied")
                }
                Err(EnvironmentFailure::Capacity(_)) => {
                    Err("field-option-payload-admission-capacity-overflow")
                }
                Err(EnvironmentFailure::Limit(_) | EnvironmentFailure::Refused(_)) => {
                    Err("field-option-payload-admission-contract-broken")
                }
            }
        }
        (FieldValue::Absent | FieldValue::Null, true, ValueType::Option(payload)) => {
            Ok(OptionValue::none((**payload).clone()))
        }
        _ => Err("field-projection-shape-invalid"),
    }
}
