// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-106 check 11: the frame and delta check.
//!
//! One matching choice, not a simplification, cited against FR-106's own
//! text (`spec/functional/FR-106-admit-snapshots-and-invocations.md:249-260`):
//!
//! - **11.1's retype refusal is exact-type, by design.** "a surviving
//!   object whose most-specific type differs between pre and post
//!   (`FrameTypeChanged`; no frame authorizes a retype)" (line 252) states a
//!   retype is *never* authorized, so comparing `producer_identity` strings
//!   directly (never conformance) is what the line says, not a narrowing of
//!   it.
//!
//! **11.3's `modifies` grant is matched by field display name, scoped to the
//! surviving object's own declaring type (SR-750 FND-007).** The wire
//! document has no `DeclarationKey`, only `"fields" ... keyed by member
//! name` (FR-106 "Document forms", `fields is keyed by member name`), so a
//! grant is matched at the wire boundary by resolving the *object's own*
//! most-specific type (`find_declaration`, the same resolution 11.1 already
//! does) and checking that some `effect.modifies` entry's owning type the
//! object's type conforms to, and whose own last path segment equals the
//! field's display name -- never a bare name compared across every type in
//! the package regardless of which one the surviving object actually is.
//! Matching by name alone let a grant on one type's field authorize a
//! same-named field on any unrelated type; this closes that gap while
//! keeping the wire boundary's own name-only field identity.
//!
//! **Still open (SR-750 FND-007 round 2, disclosed, not attempted here):**
//! FR-106's own check 11 text says to run this through
//! `crate::model::population::admit_invocation`/`enforce_frame`
//! (`population.rs:1259`), rather than reimplementing it. Investigated and
//! not done, for three concrete reasons found while building the bridge:
//!
//! 1. `population::PopulationMember.field_values` (`population.rs:466-483`)
//!    is FCD FR-121's own runtime population-document shape: `Vec<String>`
//!    "named objects" for a *reference*-valued field's subsetting check
//!    only ("This rung's only consumer" is `binding.subset-value`). It has
//!    no native representation for a *scalar* field value, where FR-106
//!    check 11.3 needs "every declared field, scalar and reference alike"
//!    compared. Reusing it would mean lossily string-encoding every scalar
//!    kind (`Boolean`, `Integer`, `Rational`, `Decimal`, `Float`,
//!    `Quantity`, `Text`, `Enum`) into that same `Vec<String>` shape, an
//!    encoding this module cannot verify is loss-free for every kind
//!    without much wider testing than this fix round's own scope.
//! 2. `InvocationContext::subtype_closure: GeneralizationClosure`
//!    (`population.rs:1164-1176`) is not a property `ModelView`/
//!    `EffectiveView` exposes at admission time: it is a check-time,
//!    dispatch-linking artifact (`check/checked_dispatch.rs`'s own
//!    `ancestor_closure`/`root.closure`, computed per operation during S3
//!    override resolution). This module would have to assume `Closed`
//!    unconditionally for every package FR-106 ever admits, a claim this
//!    fix round cannot verify against every package shape.
//! 3. `admit_invocation` returns only the admitted `PopulationBinding`, never
//!    the `(created, deleted): (Vec<ObjectReference>, Vec<ObjectReference>)`
//!    pair `AdmittedObservations` itself needs (FR-106's own check 10/11
//!    output). Deriving that pair from `PopulationBinding::members()`'s pre/
//!    post difference is a second, independent computation this module
//!    would still own regardless of how much of the frame decision itself
//!    delegates -- "delete your copy" cannot fully happen either way.
//!
//! This module's own algorithm already matches `enforce_frame`'s documented
//! behavior and order exactly (both functions' own doc comments describe
//! the identical four-step sequence); what is open is the *call*, not the
//! *decision*.

use std::collections::{BTreeMap, BTreeSet};

use quire_exact::ObjectReference;

use super::document::{find_declaration, raw_field, RawPopulation, RawTypeIdentity};
use super::helpers::{admission_record, object_reference};
use super::{refuse, AdmissionFailure, ModelView, SelectedObject};
use crate::model::domain_package::OperationEffect;
use crate::model::key::DeclarationKey;

fn field_name_of(key: &DeclarationKey) -> &str {
    key.node.rsplit('/').next().unwrap_or(&key.node)
}

/// `key`'s owning type: `key`'s own `node` with its last `/`-separated
/// segment (the field's display name) removed.
fn owner_of(key: &DeclarationKey) -> DeclarationKey {
    let owner_node = key
        .node
        .rsplit_once('/')
        .map_or(key.node.as_str(), |(owner, _)| owner);
    DeclarationKey {
        package: key.package.clone(),
        node: owner_node.to_owned(),
    }
}

/// Whether `effect.modifies` authorizes a write to `field` on an object
/// whose most-specific type is `object_type`: some `modifies` entry names
/// `field` (by display name) on a type `object_type` conforms to (SR-750
/// FND-007) -- not merely a same-named field on an unrelated type.
fn field_write_authorized(
    modifies: &[DeclarationKey],
    field: &str,
    object_type: &DeclarationKey,
    view: &ModelView,
) -> bool {
    modifies.iter().any(|key| {
        field_name_of(key) == field
            && view
                .view
                .model_index()
                .conforms(object_type, &owner_of(key), 64)
                .unwrap_or(false)
    })
}

struct PopulationSide<'a> {
    keys: BTreeMap<String, &'a [(String, super::RawValue)]>,
    producer_identity: BTreeMap<String, &'a RawTypeIdentity>,
}

fn side<'a>(populations: &'a [RawPopulation], name: &str) -> PopulationSide<'a> {
    let mut keys = BTreeMap::new();
    let mut producer_identity = BTreeMap::new();
    if let Some(population) = populations.iter().find(|entry| entry.population == name) {
        for object in &population.objects {
            keys.insert(object.key.clone(), object.fields.as_slice());
            producer_identity.insert(object.key.clone(), &object.type_identity);
        }
    }
    PopulationSide {
        keys,
        producer_identity,
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
    let creates: BTreeSet<&str> = effect.creates.iter().map(|key| key.node.as_str()).collect();
    let deletes: BTreeSet<&str> = effect.deletes.iter().map(|key| key.node.as_str()).collect();

    // Every population this invocation could possibly need to check: the
    // pre snapshot's own, then any post-only one, then any population
    // `declared_created`/`declared_deleted` names that neither snapshot
    // lists at all (SR-750 FND-007) -- an invocation cannot escape 11.4's
    // delta-disagreement check just by naming a population no snapshot
    // has any objects in.
    let mut order: Vec<String> = pre_populations
        .iter()
        .map(|population| population.population.clone())
        .collect();
    for population in post_populations {
        if !order.contains(&population.population) {
            order.push(population.population.clone());
        }
    }
    for object in declared_created.iter().chain(declared_deleted) {
        if !order.contains(&object.population) {
            order.push(object.population.clone());
        }
    }

    let mut created = Vec::new();
    let mut deleted = Vec::new();

    for name in &order {
        let pre_side = side(pre_populations, name);
        let post_side = side(post_populations, name);

        // 11.1: post objects in ascending key order.
        for (key, post_type) in &post_side.producer_identity {
            match pre_side.producer_identity.get(key) {
                None => {
                    if !creates.contains(post_type.as_str()) {
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
        for key in pre_side.producer_identity.keys() {
            if !post_side.producer_identity.contains_key(key) {
                let pre_type = pre_side.producer_identity[key];
                if !deletes.contains(pre_type.as_str()) {
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
                if !post_side.producer_identity.contains_key(key) {
                    continue;
                }
                // A field's own `modifies` grant is scoped to this
                // object's own most-specific type (SR-750 FND-007): a
                // same-named field of an unrelated type is never
                // authorized by it.
                let Some((object_view, object_type)) =
                    find_declaration(views, pre_object.type_identity.as_str())
                else {
                    return Err(refuse(admission_record(
                        "invalid_runtime_input",
                        "wrong-role-mapping",
                    )));
                };
                let empty: &[(String, super::RawValue)] = &[];
                let post_fields = post_side.keys.get(key).copied().unwrap_or(empty);
                let field_names: BTreeSet<&str> = pre_object
                    .fields
                    .iter()
                    .map(|(name, _)| name.as_str())
                    .chain(post_fields.iter().map(|(name, _)| name.as_str()))
                    .collect();
                for field in field_names {
                    if field_write_authorized(&effect.modifies, field, &object_type, object_view) {
                        continue;
                    }
                    let pre_value = raw_field(&pre_object.fields, field);
                    let post_value = raw_field(post_fields, field);
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
            .producer_identity
            .keys()
            .filter(|key| !pre_side.producer_identity.contains_key(*key))
            .cloned()
            .collect();
        let computed_deleted: BTreeSet<String> = pre_side
            .producer_identity
            .keys()
            .filter(|key| !post_side.producer_identity.contains_key(*key))
            .cloned()
            .collect();
        if population_created != computed_created || population_deleted != computed_deleted {
            return Err(refuse(admission_record(
                "population_delta_mismatch",
                "delta-disagreement",
            )));
        }
        for key in &population_created {
            let Some((_, object_key)) =
                find_declaration(views, post_side.producer_identity[key].as_str())
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
                object_reference(views, effective_type, key)
                    .map_err(|_| super::fault("empty-object-identity"))?,
            );
        }
        for key in &population_deleted {
            let Some((_, object_key)) =
                find_declaration(views, pre_side.producer_identity[key].as_str())
            else {
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
                object_reference(views, effective_type, key)
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
