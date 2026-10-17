// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-120's initial-state admission and state observation: the
//! implementation behind [`crate::model::state::StateModel`].

use std::collections::BTreeMap;

use qsl_foundation::diagnostic::Code;
use quire_exact::{FieldValue, ObjectReference, Presence};
use quire_semantic_value::declaration::{EffectiveAttribute, TypeEnvironment};
use quire_semantic_value::object_closure::ObjectClosure;

use super::document::{
    admit_initial_populations, map_environment_refusal, read_document, DocumentKind,
};
use super::helpers::object_reference;
use super::relationships::{admit_supplied_extents, model_facts};
use super::{
    check_model_any, fault, refuse, view_of, AdmissionFailure, AdmissionRecord, AnchorKind,
    DocumentRef, ModelView, Observation, ObservationLimits, SelectedAnchor,
};
use crate::model::key::DeclarationKey;
use crate::model::object_environment::ObjectEnvironment;
use crate::model::state::{ModelState, StateField, StateObject};

/// `attribute`'s own declaration key: its owner's key with the field name
/// appended, the key FR-106 check 11 compares fields by.
fn field_key(
    views: &[ModelView],
    attribute: EffectiveAttribute<'_>,
) -> Result<DeclarationKey, AdmissionFailure> {
    let owner = view_of(views, attribute.owner())
        .and_then(|view| view.declaration_key(attribute.owner()))
        .ok_or_else(|| fault("state-field-owner-unresolved"))?;
    Ok(DeclarationKey {
        package: owner.package.clone(),
        node: format!("{}/{}", owner.node, attribute.field().name()),
    })
}

/// FR-120 "Initial states": FR-106 check 1, check 3 (`current`, anchored by
/// an `initialization`), check 4, and checks 6 to 8 with every listed
/// population required.
pub(crate) fn admit_initial_state(
    views: &[ModelView],
    types: &TypeEnvironment,
    provision: &BTreeMap<[u8; 32], Vec<u8>>,
    selected: &DocumentRef,
    limits: ObservationLimits,
    ancestor_steps: u64,
) -> Result<(ModelState, SelectedAnchor), AdmissionFailure> {
    let read = read_document(DocumentKind::Snapshot, provision, selected, limits)?;
    let snapshot = read
        .as_snapshot()
        .ok_or_else(|| fault("expected-snapshot-document"))?;

    if snapshot.observation != super::document::ObservationRole::Current {
        return Err(refuse(AdmissionRecord::new(
            Code::WrongSnapshot,
            "wrong-observation",
        )));
    }
    let anchor = snapshot.anchor.as_ref().ok_or_else(|| {
        refuse(AdmissionRecord::new(
            Code::WrongSnapshot,
            "wrong-observation",
        ))
    })?;
    if anchor.kind != AnchorKind::Initialization {
        return Err(refuse(
            AdmissionRecord::new(Code::WrongSnapshot, "wrong-anchor")
                .with("required", "initialization")
                .with(
                    "supplied",
                    format!("{} {}", anchor.kind.as_str(), anchor.name),
                ),
        ));
    }
    let selected_anchor = SelectedAnchor {
        kind: anchor.kind,
        name: anchor.name.clone(),
    };

    check_model_any(views, &snapshot.model)?;

    let admitted_initial =
        admit_initial_populations(views, types, snapshot, selected, ancestor_steps)?;

    let mut populations = BTreeMap::new();
    for population in admitted_initial.populations {
        let mut objects = BTreeMap::new();
        for object in population.objects {
            let type_identity = view_of(views, object.reference.object_type())
                .and_then(|view| view.declaration_key(object.reference.object_type()))
                .ok_or_else(|| fault("state-object-type-unresolved"))?
                .clone();
            let attributes = types
                .attributes(object.reference.object_type())
                .ok_or_else(|| fault("state-object-type-has-no-checked-attributes"))?;
            let mut fields = BTreeMap::new();
            for attribute in attributes {
                let slot = object
                    .fields
                    .iter()
                    .find(|(name, _)| name == attribute.field().name())
                    .map(|(_, slot)| slot)
                    .ok_or_else(|| fault("state-declared-field-missing"))?;
                match slot {
                    FieldValue::Present(value) => {
                        fields.insert(field_key(views, attribute)?, value.clone());
                    }
                    FieldValue::Absent => {}
                    FieldValue::Null => return Err(fault("state-field-holds-null")),
                }
            }
            objects.insert(
                object.reference.object().as_str().to_owned(),
                StateObject {
                    type_identity,
                    fields,
                },
            );
        }
        populations.insert(population.key, objects);
    }
    Ok((
        ModelState {
            populations,
            extents: admitted_initial.extents,
        },
        selected_anchor,
    ))
}

/// FR-120 "Frame and observation seam": `state`'s [`Observation`] named
/// `identity`, every listed population complete.
pub(crate) fn observe_state(
    views: &[ModelView],
    types: &TypeEnvironment,
    state: &ModelState,
    identity: DocumentRef,
) -> Result<Observation, AdmissionFailure> {
    let mut objects = Vec::new();
    let mut completeness = BTreeMap::new();
    for (population, members) in &state.populations {
        completeness.insert(population.node.clone(), true);
        for (key, object) in members {
            let effective = views
                .iter()
                .find_map(|view| view.view.type_identities().get(&object.type_identity))
                .copied()
                .ok_or_else(|| fault("state-object-type-unresolved"))?;
            let attributes = types
                .attributes(effective)
                .ok_or_else(|| fault("state-object-type-has-no-checked-attributes"))?;
            let mut slots: Vec<(&str, FieldValue)> = Vec::with_capacity(attributes.len());
            for attribute in attributes {
                let slot = match object.fields.get(&field_key(views, attribute)?) {
                    Some(value) => FieldValue::Present(value.clone()),
                    None if attribute.field().presence() == Presence::Optional => {
                        FieldValue::Absent
                    }
                    None => return Err(fault("state-required-field-missing")),
                };
                slots.push((attribute.field().name(), slot));
            }
            objects.push((object_reference(views, effective, key)?, slots));
        }
    }
    let closure = ObjectClosure::new(types, objects, &[]).map_err(map_environment_refusal)?;
    let extents = admit_supplied_extents(
        types,
        model_facts(views, types),
        state.extents.clone(),
        &completeness,
    )?;
    Ok(Observation {
        identity,
        environment: ObjectEnvironment::new(closure).with_extents(extents),
        populations: completeness,
        out_of_range: Vec::new(),
    })
}

/// [`crate::model::state::StateModel::fields`].
pub(crate) fn state_fields(
    views: &[ModelView],
    types: &TypeEnvironment,
    type_key: &DeclarationKey,
) -> Result<Option<Vec<StateField>>, AdmissionFailure> {
    let Some(identity) = views
        .iter()
        .find_map(|view| view.view.type_identities().get(type_key).copied())
    else {
        return Ok(None);
    };
    let Some(attributes) = types.attributes(identity) else {
        return Ok(None);
    };
    let mut fields = Vec::new();
    for attribute in attributes {
        fields.push(StateField {
            key: field_key(views, attribute)?,
            name: attribute.field().name().to_owned(),
            value_type: attribute.field().value_type().clone(),
            optional: attribute.field().presence() == Presence::Optional,
        });
    }
    Ok(Some(fields))
}

/// [`crate::model::state::StateModel::object_reference`].
pub(crate) fn state_reference(
    views: &[ModelView],
    type_key: &DeclarationKey,
    object: &str,
) -> Result<ObjectReference, AdmissionFailure> {
    let identity = views
        .iter()
        .find_map(|view| view.view.type_identities().get(type_key).copied())
        .ok_or_else(|| fault("state-object-type-unresolved"))?;
    object_reference(views, identity, object)
}
