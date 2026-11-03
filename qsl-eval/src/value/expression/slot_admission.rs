// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-049-AC-2/AC-3: admission of a caller-supplied object-attribute value
//! against the evaluating package's own declarations.
//!
//! `TypeEnvironment::admits` accepts a record by its declaration key, on the
//! premise that its members were admitted against that declaration. A value
//! in a caller-built `ObjectClosure` carries no such premise, so every record
//! member reachable through options, collections and nested records is
//! admitted here against the package's own record declaration. The walk is an
//! explicit worklist, so a deep value cannot overflow the stack.

use qsl_semantics::model::observation::OutOfRange;
use quire_exact::{FieldValue, Value, ValueType};
use quire_semantic_value::declaration::{CompositeShape, EnvironmentFailure, TypeEnvironment};

/// Whether `value` is a member of `declared` under `types`, with every record
/// member admitted against the package's own record declaration.
///
/// `witnesses` are the FR-106 check 6.5 out-of-range witnesses recorded for
/// the slot. A witness with no record path relaxes only the integer range at
/// its own position: a scalar witness must name the stored integer, and an
/// indexed one the element at its index, which must equal the observed
/// integer; every other element is admitted at its declared range. A witness
/// inside a record (non-empty path) relaxes every integer range of the slot,
/// because relaxing a single record member would need the record rebuilt.
/// Type and unit are always admitted.
pub(super) fn admits(
    types: &TypeEnvironment,
    declared: &ValueType,
    value: &Value,
    witnesses: &[&OutOfRange],
) -> Result<bool, EnvironmentFailure> {
    if witnesses.is_empty() {
        return admits_members(types, declared, value, false);
    }
    let in_record = witnesses.iter().any(|witness| !witness.steps.is_empty());
    if !admits_members(types, &widen_integer_ranges(declared), value, in_record)? {
        return Ok(false);
    }
    for witness in witnesses {
        if !witness.steps.is_empty() {
            continue;
        }
        let observes = |candidate: &Value| matches!(candidate, Value::Integer(held) if *held == witness.observed);
        match witness.index {
            None => {
                if !observes(value) {
                    return Ok(false);
                }
            }
            Some(index) => {
                let (ValueType::Collection(collection), Value::Collection(held)) =
                    (declared, value)
                else {
                    return Ok(false);
                };
                if !held.elements().get(index).is_some_and(observes) {
                    return Ok(false);
                }
                for (position, element) in held.elements().iter().enumerate() {
                    let witnessed = witnesses
                        .iter()
                        .any(|other| other.steps.is_empty() && other.index == Some(position));
                    if !witnessed && !admits_members(types, collection.element(), element, false)? {
                        return Ok(false);
                    }
                }
            }
        }
    }
    Ok(true)
}

/// `types.admits`, then each record member of `value` against the package's
/// record declaration of the record's own key (its integer ranges relaxed when
/// `relax_members`).
fn admits_members(
    types: &TypeEnvironment,
    declared: &ValueType,
    value: &Value,
    relax_members: bool,
) -> Result<bool, EnvironmentFailure> {
    if !types.admits(declared, value)? {
        return Ok(false);
    }
    let mut pending = vec![(declared, value)];
    while let Some((declared, value)) = pending.pop() {
        match (declared, value) {
            (ValueType::Option(payload), Value::Option(option)) => {
                pending.extend(
                    option
                        .payload()
                        .map(|payload_value| (&**payload, payload_value)),
                );
            }
            (ValueType::Collection(collection), Value::Collection(held)) => {
                pending.extend(held.elements().iter().map(|e| (collection.element(), e)));
            }
            (ValueType::Composite(_), Value::Composite(record)) => {
                let Some(declaration) = types.composite(record.declaration().clone()) else {
                    return Ok(false);
                };
                let member_types: Vec<&ValueType> = match declaration.shape() {
                    CompositeShape::Record(fields) => {
                        fields.iter().map(|field| field.value_type()).collect()
                    }
                    CompositeShape::Tuple(positions) => positions.iter().collect(),
                    CompositeShape::Union(_) => continue,
                };
                if member_types.len() != record.slots().len() {
                    return Ok(false);
                }
                for (member_type, slot) in member_types.into_iter().zip(record.slots()) {
                    if let FieldValue::Present(member) = slot {
                        let checked = if relax_members {
                            std::borrow::Cow::Owned(widen_integer_ranges(member_type))
                        } else {
                            std::borrow::Cow::Borrowed(member_type)
                        };
                        if !types.admits(&checked, member)? {
                            return Ok(false);
                        }
                        pending.push((member_type, member));
                    }
                }
            }
            _ => {}
        }
    }
    Ok(true)
}

/// `value_type` with every `Int[lo, hi]` range replaced by `Integer`, through
/// options and collection elements.
fn widen_integer_ranges(value_type: &ValueType) -> ValueType {
    match value_type {
        ValueType::Int(_) => ValueType::Integer,
        ValueType::Option(payload) => ValueType::option(widen_integer_ranges(payload)),
        ValueType::Collection(collection) => {
            ValueType::Collection(std::sync::Arc::new(quire_exact::CollectionType::new(
                collection.kind(),
                widen_integer_ranges(collection.element()),
                collection.bound(),
            )))
        }
        other => other.clone(),
    }
}
