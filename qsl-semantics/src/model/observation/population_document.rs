// SPDX-License-Identifier: AGPL-3.0-or-later
//! Retain population-admission input from a checked snapshot's actual fields.

use quire_exact::{FieldValue, Value};
use quire_semantic_value::declaration::{FieldRef, TypeEnvironment};

use super::{allocation_failed, fault, AdmissionFailure};
use crate::model::domain_package::DomainPackageRecord;
use crate::model::normalize::EffectiveView;
use crate::model::object_environment::ObjectEnvironment;
use crate::model::population::{MemberFieldValues, PopulationDocument, PopulationMember};

/// Read an already admitted snapshot's population into the existing binding
/// admission input. `member_order` is the original snapshot order; every
/// identity must resolve in that exact admitted population. Values come only
/// from the checked object closure, including inherited and redefined slots.
///
/// This performs no new binding admission or accounting reset. The caller
/// passes the result to `population::admit_binding` with its existing meter.
/// Retention uses fallible reservations; the admission schedule has no extra
/// charge for copying already admitted field facts.
///
/// # Errors
///
/// An invariant fault if the population, member order, selected view or field
/// correspondence does not match the admitted snapshot; allocation refusal if
/// retained storage cannot be reserved.
pub fn admitted_population_document(
    view: &EffectiveView,
    types: &TypeEnvironment,
    objects: &ObjectEnvironment,
    population: &str,
    member_order: &[&str],
) -> Result<PopulationDocument, AdmissionFailure> {
    let provenance = objects
        .population_node(population)
        .and_then(|node| objects.population_provenance(node))
        .ok_or_else(|| fault("population-provenance-unresolved"))?;
    if view.population(provenance.declaration()).is_none() {
        return Err(fault("population-view-mismatch"));
    }
    let admitted = objects
        .population_members(population)
        .ok_or_else(|| fault("population-members-unresolved"))?;
    if member_order.len() != admitted.len() {
        return Err(fault("population-member-order-mismatch"));
    }
    let mut members = Vec::new();
    members.try_reserve(member_order.len()).map_err(|_| {
        allocation_failed(
            member_order
                .len()
                .saturating_mul(std::mem::size_of::<PopulationMember>()),
        )
    })?;
    for identity in member_order {
        if members
            .iter()
            .any(|member: &PopulationMember| member.object == *identity)
        {
            return Err(fault("population-member-order-duplicate"));
        }
        let reference = admitted
            .iter()
            .find(|reference| reference.object().as_str() == *identity)
            .ok_or_else(|| fault("population-member-unresolved"))?;
        let type_identity = view
            .type_identities()
            .iter()
            .find(|(_, effective)| *effective == reference.object_type())
            .map(|(key, _)| key.clone())
            .ok_or_else(|| fault("population-type-unresolved"))?;
        let attributes = types
            .attributes(reference.object_type().clone())
            .ok_or_else(|| fault("population-attributes-unresolved"))?;
        let held = objects
            .objects()
            .object(reference)
            .ok_or_else(|| fault("population-object-unresolved"))?;
        let mut field_values = Vec::new();
        // Iterate original field declarations, retaining both sides of every
        // subset relation. A storage slot can stand for several inherited or
        // redefined declarations; each keeps its own original field key.
        for record in &view.domain_package().records {
            let DomainPackageRecord::FieldMember(field) = record else {
                continue;
            };
            let Some(owner) = view.type_identities().get(&field.owner) else {
                return Err(fault("population-field-owner-unresolved"));
            };
            let prefix = format!("{}/", field.owner.node);
            let name = field
                .key
                .node
                .strip_prefix(&prefix)
                .ok_or_else(|| fault("population-field-key-unresolved"))?;
            let original = FieldRef::new(owner.clone(), name);
            let Some(attribute) = attributes
                .iter()
                .find(|attribute| attribute.stands_for(&original))
            else {
                continue;
            };
            // Resolve the visible declaration through the same native key
            // conversion snapshot admission uses, rather than inventing an id.
            let actual_owner = view
                .type_identities()
                .iter()
                .find(|(_, effective)| **effective == attribute.owner())
                .map(|(key, _)| key)
                .ok_or_else(|| fault("population-slot-owner-unresolved"))?;
            let actual_key = super::initial::field_key_of(actual_owner, attribute);
            if !view.domain_package().records.iter().any(|record| {
                matches!(record,
                DomainPackageRecord::FieldMember(actual) if actual.key == actual_key)
            }) {
                return Err(fault("population-slot-key-unresolved"));
            }
            let slot = held
                .get(attribute.field().name())
                .ok_or_else(|| fault("population-slot-unresolved"))?;
            let values = references(slot)?;
            field_values
                .try_reserve(1)
                .map_err(|_| allocation_failed(std::mem::size_of::<MemberFieldValues>()))?;
            field_values.push(MemberFieldValues {
                field: field.key.clone(),
                values,
            });
        }
        members.push(PopulationMember {
            object: copy(identity)?,
            type_identity,
            field_values,
        });
    }
    Ok(PopulationDocument {
        model_identity: copy(&view.model_selection().identity)?,
        members,
    })
}

fn copy(value: &str) -> Result<String, AdmissionFailure> {
    let mut retained = String::new();
    retained
        .try_reserve(value.len())
        .map_err(|_| allocation_failed(value.len()))?;
    retained.push_str(value);
    Ok(retained)
}

fn references(slot: &FieldValue) -> Result<Vec<String>, AdmissionFailure> {
    let mut values = Vec::new();
    let mut pending = Vec::new();
    if let FieldValue::Present(value) = slot {
        pending
            .try_reserve(1)
            .map_err(|_| allocation_failed(std::mem::size_of::<&Value>()))?;
        pending.push(value);
    }
    while let Some(value) = pending.pop() {
        match value {
            Value::Reference(reference) => {
                values
                    .try_reserve(1)
                    .map_err(|_| allocation_failed(std::mem::size_of::<String>()))?;
                values.push(copy(reference.object().as_str())?);
            }
            Value::Option(option) => {
                if let Some(payload) = option.payload() {
                    pending
                        .try_reserve(1)
                        .map_err(|_| allocation_failed(std::mem::size_of::<&Value>()))?;
                    pending.push(payload);
                }
            }
            Value::Collection(collection) => {
                pending
                    .try_reserve(collection.elements().len())
                    .map_err(|_| {
                        allocation_failed(
                            collection
                                .elements()
                                .len()
                                .saturating_mul(std::mem::size_of::<&Value>()),
                        )
                    })?;
                pending.extend(collection.elements().iter().rev());
            }
            Value::Composite(composite) => {
                pending.try_reserve(composite.slots().len()).map_err(|_| {
                    allocation_failed(
                        composite
                            .slots()
                            .len()
                            .saturating_mul(std::mem::size_of::<&Value>()),
                    )
                })?;
                for field in composite.slots().iter().rev() {
                    if let FieldValue::Present(value) = field {
                        pending.push(value);
                    }
                }
            }
            Value::Union(union) => {
                pending.try_reserve(union.payload().len()).map_err(|_| {
                    allocation_failed(
                        union
                            .payload()
                            .len()
                            .saturating_mul(std::mem::size_of::<&Value>()),
                    )
                })?;
                pending.extend(union.payload().iter().rev());
            }
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
            | Value::Population(_) => {}
        }
    }
    Ok(values)
}
