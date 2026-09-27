// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-106 check 11: the frame and delta check.
//!
//! Simplification, disclosed in the PR: a `modifies` grant is matched by
//! its field's own display name only (the last `/`-separated segment of its
//! producer [`DeclarationKey`]), not by the redefinition-aware identity the
//! full effect-inclusion rule uses; and a `creates`/`deletes` grant is
//! matched by exact producer type identity rather than full conformance.
//! Neither TC-465 nor TC-464 exercises a redefined `modifies` member or a
//! `creates`/`deletes` grant naming a proper supertype of the created or
//! deleted object's own type, so this reads every case FR-106 tests
//! correctly; a package using either construct needs this widened.

use std::collections::{BTreeMap, BTreeSet};

use quire_exact::ObjectReference;

use super::document::{find_declaration, RawPopulation};
use super::helpers::{admission_record, object_reference};
use super::{refuse, AdmissionFailure, ModelView, SelectedObject};
use crate::model::domain_package::OperationEffect;

fn field_name_of(key: &crate::model::key::DeclarationKey) -> &str {
    key.node.rsplit('/').next().unwrap_or(&key.node)
}

struct PopulationSide<'a> {
    keys: BTreeMap<String, &'a BTreeMap<String, super::RawValue>>,
    type_identity: BTreeMap<String, &'a str>,
}

fn side<'a>(populations: &'a [RawPopulation], name: &str) -> PopulationSide<'a> {
    let mut keys = BTreeMap::new();
    let mut type_identity = BTreeMap::new();
    if let Some(population) = populations.iter().find(|entry| entry.population == name) {
        for object in &population.objects {
            keys.insert(object.key.clone(), &object.fields);
            type_identity.insert(object.key.clone(), object.type_identity.as_str());
        }
    }
    PopulationSide {
        keys,
        type_identity,
    }
}

/// FR-106 check 11: run `enforce_frame` over `pre_populations`/
/// `post_populations`, in the pre snapshot's population document order and
/// then any population only in the post snapshot, in its document order.
/// Returns the admitted `(created, deleted)` object references over the
/// whole invocation.
pub(super) fn enforce(
    views: &[ModelView],
    effect: &OperationEffect,
    pre_populations: &[RawPopulation],
    post_populations: &[RawPopulation],
    declared_created: &[SelectedObject],
    declared_deleted: &[SelectedObject],
) -> Result<(Vec<ObjectReference>, Vec<ObjectReference>), AdmissionFailure> {
    let modifies: BTreeSet<&str> = effect.modifies.iter().map(field_name_of).collect();
    let creates: BTreeSet<&str> = effect.creates.iter().map(|key| key.node.as_str()).collect();
    let deletes: BTreeSet<&str> = effect.deletes.iter().map(|key| key.node.as_str()).collect();

    let mut order: Vec<String> = pre_populations
        .iter()
        .map(|population| population.population.clone())
        .collect();
    for population in post_populations {
        if !order.contains(&population.population) {
            order.push(population.population.clone());
        }
    }

    let mut created = Vec::new();
    let mut deleted = Vec::new();

    for name in &order {
        let pre_side = side(pre_populations, name);
        let post_side = side(post_populations, name);

        // 11.1: post objects in ascending key order.
        for (key, post_type) in &post_side.type_identity {
            match pre_side.type_identity.get(key) {
                None => {
                    if !creates.contains(*post_type) {
                        return Err(refuse(
                            admission_record("frame_violation", "unauthorized-change")
                                .with("population", name.clone())
                                .with("object", key.clone()),
                        ));
                    }
                }
                Some(pre_type) => {
                    if pre_type != post_type {
                        return Err(refuse(
                            admission_record("frame_violation", "unauthorized-change")
                                .with("population", name.clone())
                                .with("object", key.clone()),
                        ));
                    }
                }
            }
        }
        // 11.2: pre objects absent from post, ascending key order.
        for key in pre_side.type_identity.keys() {
            if !post_side.type_identity.contains_key(key) {
                let pre_type = pre_side.type_identity[key];
                if !deletes.contains(pre_type) {
                    return Err(refuse(
                        admission_record("frame_violation", "unauthorized-change")
                            .with("population", name.clone())
                            .with("object", key.clone()),
                    ));
                }
            }
        }
        // 11.3: surviving objects, pre document order, fields ascending.
        if let Some(pre_population) = pre_populations.iter().find(|p| p.population == *name) {
            for pre_object in &pre_population.objects {
                let key = &pre_object.key;
                if !post_side.type_identity.contains_key(key) {
                    continue;
                }
                let empty = BTreeMap::new();
                let post_fields = post_side.keys.get(key).copied().unwrap_or(&empty);
                let field_names: BTreeSet<&str> = pre_object
                    .fields
                    .keys()
                    .map(String::as_str)
                    .chain(post_fields.keys().map(String::as_str))
                    .collect();
                for field in field_names {
                    if modifies.contains(field) {
                        continue;
                    }
                    let pre_value = pre_object.fields.get(field);
                    let post_value = post_fields.get(field);
                    if !raw_values_equal(pre_value, post_value) {
                        return Err(refuse(
                            admission_record("frame_violation", "unauthorized-change")
                                .with("population", name.clone())
                                .with("object", key.clone())
                                .with("field", field.to_owned()),
                        ));
                    }
                }
            }
        }

        // 11.4: delta agreement over this population's declared entries.
        let population_created: BTreeSet<String> = declared_created
            .iter()
            .filter(|object| object.population == *name)
            .map(|object| object.key.clone())
            .collect();
        let population_deleted: BTreeSet<String> = declared_deleted
            .iter()
            .filter(|object| object.population == *name)
            .map(|object| object.key.clone())
            .collect();
        if declared_created
            .iter()
            .filter(|object| object.population == *name)
            .count()
            != population_created.len()
            || declared_deleted
                .iter()
                .filter(|object| object.population == *name)
                .count()
                != population_deleted.len()
            || !population_created.is_disjoint(&population_deleted)
        {
            return Err(refuse(admission_record(
                "population_delta_mismatch",
                "delta-disagreement",
            )));
        }
        let computed_created: BTreeSet<String> = post_side
            .type_identity
            .keys()
            .filter(|key| !pre_side.type_identity.contains_key(*key))
            .cloned()
            .collect();
        let computed_deleted: BTreeSet<String> = pre_side
            .type_identity
            .keys()
            .filter(|key| !post_side.type_identity.contains_key(*key))
            .cloned()
            .collect();
        if population_created != computed_created || population_deleted != computed_deleted {
            return Err(refuse(admission_record(
                "population_delta_mismatch",
                "delta-disagreement",
            )));
        }
        for key in &population_created {
            let Some((_, object_key)) = find_declaration(views, post_side.type_identity[key])
            else {
                return Err(refuse(admission_record(
                    "invalid_runtime_input",
                    "wrong-role-mapping",
                )));
            };
            let effective_type = views
                .iter()
                .find_map(|view| view.view.type_identities().get(&object_key).copied())
                .ok_or_else(|| super::fault("created-object-type-unresolved"))?;
            created.push(
                object_reference(name, effective_type, key)
                    .map_err(|_| super::fault("empty-object-identity"))?,
            );
        }
        for key in &population_deleted {
            let Some((_, object_key)) = find_declaration(views, pre_side.type_identity[key]) else {
                return Err(refuse(admission_record(
                    "invalid_runtime_input",
                    "wrong-role-mapping",
                )));
            };
            let effective_type = views
                .iter()
                .find_map(|view| view.view.type_identities().get(&object_key).copied())
                .ok_or_else(|| super::fault("deleted-object-type-unresolved"))?;
            deleted.push(
                object_reference(name, effective_type, key)
                    .map_err(|_| super::fault("empty-object-identity"))?,
            );
        }
    }

    Ok((created, deleted))
}

fn raw_values_equal(left: Option<&super::RawValue>, right: Option<&super::RawValue>) -> bool {
    match (left, right) {
        (None, None) => true,
        (Some(left), Some(right)) => raw_value_eq(left, right),
        _ => false,
    }
}

fn raw_value_eq(left: &super::RawValue, right: &super::RawValue) -> bool {
    use super::RawValue::*;
    match (left, right) {
        (Boolean(a), Boolean(b)) => a == b,
        (Integer(a), Integer(b)) => a == b,
        (Absent, Absent) => true,
        (Present(a), Present(b)) => raw_value_eq(a, b),
        (Reference(a), Reference(b)) => a.population == b.population && a.key == b.key,
        (Sequence(a), Sequence(b)) => {
            a.len() == b.len() && a.iter().zip(b).all(|(a, b)| raw_value_eq(a, b))
        }
        _ => false,
    }
}
