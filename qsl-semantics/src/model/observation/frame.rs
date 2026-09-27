// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-106 check 11: the frame and delta check.
//!
//! The decision itself is [`crate::model::population::decide_frame`], the
//! one frame decision `enforce_frame` (and so `admit_invocation`) also
//! runs. This module only builds its per-object input from the admitted
//! wire documents -- each object's most-specific type and each declared
//! field's [`DeclarationKey`] with its wire value -- once per population,
//! in FR-106's population order, and maps the decision's refusal to an
//! admission record.

use std::collections::BTreeMap;

use quire_exact::{EffectiveId, ObjectReference};

use super::document::{find_declaration, raw_field, RawObject, RawPopulation};
use super::helpers::object_reference;
use super::{
    fault, refuse, view_of, AdmissionFailure, AdmissionRecord, ModelView, RawValue, SelectedObject,
};
use crate::model::domain_package::OperationEffect;
use crate::model::key::DeclarationKey;
use crate::model::normalize::{ModelRefusal, ModelRefusalCause};
use crate::model::population::{decide_frame, FrameDecision, FrameObject};
use crate::value::declaration::TypeEnvironment;

/// What check 11 reads besides the documents: the re-derived model views,
/// the checked type environment, and the operation's frame.
pub(super) struct FrameContext<'a> {
    /// Every re-derived model view.
    pub(super) views: &'a [ModelView],
    /// The checked package's type environment.
    pub(super) types: &'a TypeEnvironment,
    /// The view the clause's context type and operation resolve in.
    pub(super) context_view: &'a ModelView,
    /// The conformance walk's `ancestor_steps` ceiling.
    pub(super) ancestor_steps: u64,
    /// The operation's authored frame.
    pub(super) effect: &'a OperationEffect,
}

/// What this module keeps about one wire object beside its
/// [`FrameObject`]: its effective type, to reference it once it is created
/// or deleted, and each declared field's wire name, to report a refusal.
struct WireFacts<'a> {
    effective_type: EffectiveId,
    names: BTreeMap<DeclarationKey, &'a str>,
}

/// One population's objects on one side, in document order:
/// `frame[i]` and `facts[i]` describe the same object.
struct WireSide<'a> {
    frame: Vec<FrameObject<'a, &'a RawValue>>,
    facts: Vec<WireFacts<'a>>,
}

impl WireSide<'_> {
    fn facts_of(&self, object: &str) -> Option<&WireFacts<'_>> {
        self.frame
            .iter()
            .position(|frame| frame.object == object)
            .and_then(|at| self.facts.get(at))
    }
}

/// `object` as [`decide_frame`] reads it. Check 6 already admitted it, so
/// its type resolves and every declared field is present: a failure here is
/// a broken admission invariant, never a document defect.
fn wire_object<'a>(
    context: &FrameContext<'a>,
    object: &'a RawObject,
) -> Result<(FrameObject<'a, &'a RawValue>, WireFacts<'a>), AdmissionFailure> {
    let (view, type_identity) = find_declaration(context.views, object.type_identity.as_str())
        .ok_or_else(|| fault("frame-object-type-unresolved"))?;
    let effective_type = *view
        .view
        .type_identities()
        .get(&type_identity)
        .ok_or_else(|| fault("frame-object-type-has-no-effective-id"))?;
    let attributes = context
        .types
        .attributes(effective_type)
        .ok_or_else(|| fault("frame-object-type-has-no-checked-attributes"))?;
    let mut fields = BTreeMap::new();
    let mut names = BTreeMap::new();
    for attribute in attributes {
        let name = attribute.field().name();
        let owner = view_of(context.views, attribute.owner())
            .and_then(|owner_view| owner_view.declaration_key(attribute.owner()))
            .ok_or_else(|| fault("frame-field-owner-unresolved"))?;
        let key = DeclarationKey {
            package: owner.package.clone(),
            node: format!("{}/{name}", owner.node),
        };
        let value =
            raw_field(&object.fields, name).ok_or_else(|| fault("frame-declared-field-missing"))?;
        fields.insert(key.clone(), value);
        names.insert(key, name);
    }
    Ok((
        FrameObject {
            object: object.key.as_str(),
            type_identity,
            fields,
        },
        WireFacts {
            effective_type,
            names,
        },
    ))
}

/// Population `name`'s objects in `populations`, in document order; empty
/// when `populations` does not list `name`.
fn wire_side<'a>(
    context: &FrameContext<'a>,
    populations: &'a [RawPopulation],
    name: &str,
) -> Result<WireSide<'a>, AdmissionFailure> {
    let mut side = WireSide {
        frame: Vec::new(),
        facts: Vec::new(),
    };
    let objects = populations
        .iter()
        .filter(|population| population.population == name)
        .flat_map(|population| &population.objects);
    for object in objects {
        let (frame, facts) = wire_object(context, object)?;
        side.frame.push(frame);
        side.facts.push(facts);
    }
    Ok(side)
}

/// `refusal` (from [`decide_frame`]) as an admission refusal naming
/// `population`, and the object and field it names.
fn admission_refusal(
    refusal: &ModelRefusal,
    population: &str,
    pre: &WireSide<'_>,
) -> AdmissionFailure {
    let record = AdmissionRecord::new(refusal.code, refusal.cause.as_str())
        .with("population", population.to_owned());
    let record = match &refusal.cause {
        ModelRefusalCause::FrameCreateOutsideGrant { object, .. }
        | ModelRefusalCause::FrameTypeChanged { object, .. }
        | ModelRefusalCause::FrameDeleteOutsideGrant { object, .. } => {
            record.with("object", object.clone())
        }
        ModelRefusalCause::FrameFieldWriteOutsideGrant { object, field } => {
            let name = pre
                .facts_of(object)
                .and_then(|facts| facts.names.get(field).copied())
                .unwrap_or(field.node.as_str());
            record
                .with("object", object.clone())
                .with("field", name.to_owned())
        }
        // The delta causes and a conformance-walk limit name no single
        // object or field: the record names the population alone.
        _ => record,
    };
    refuse(record)
}

/// FR-106 check 11: run [`decide_frame`] once per population, in the pre
/// snapshot's population document order, then any population only in the
/// post snapshot, then any population only `declared_created`/
/// `declared_deleted` names (so 11.4 still checks it). A population missing
/// from one side is an empty document there. Returns the admitted
/// `(created, deleted)` object references over the whole invocation.
pub(super) fn enforce(
    context: &FrameContext<'_>,
    pre_populations: &[RawPopulation],
    post_populations: &[RawPopulation],
    declared_created: &[SelectedObject],
    declared_deleted: &[SelectedObject],
) -> Result<(Vec<ObjectReference>, Vec<ObjectReference>), AdmissionFailure> {
    let mut order: Vec<&str> = Vec::new();
    let names = pre_populations
        .iter()
        .chain(post_populations)
        .map(|population| population.population.as_str())
        .chain(
            declared_created
                .iter()
                .chain(declared_deleted)
                .map(|object| object.population.as_str()),
        );
    for name in names {
        if !order.contains(&name) {
            order.push(name);
        }
    }

    let keys_in = |declared: &[SelectedObject], name: &str| -> Vec<String> {
        declared
            .iter()
            .filter(|object| object.population == name)
            .map(|object| object.key.clone())
            .collect()
    };

    let mut created = Vec::new();
    let mut deleted = Vec::new();
    for name in order {
        let pre = wire_side(context, pre_populations, name)?;
        let post = wire_side(context, post_populations, name)?;
        let population_created = keys_in(declared_created, name);
        let population_deleted = keys_in(declared_deleted, name);
        let frame = FrameDecision {
            index: context.context_view.view.model_index(),
            ancestor_steps: context.ancestor_steps,
            effect: context.effect,
            declared_created: &population_created,
            declared_deleted: &population_deleted,
        };
        let delta = decide_frame(&frame, &pre.frame, &post.frame, |_, left, right| {
            raw_values_equal(left.copied(), right.copied())
        })
        .map_err(|refusal| admission_refusal(&refusal, name, &pre))?;

        for (keys, side, into) in [
            (&delta.created, &post, &mut created),
            (&delta.deleted, &pre, &mut deleted),
        ] {
            for key in keys {
                let facts = side
                    .facts_of(key)
                    .ok_or_else(|| fault("frame-delta-object-unresolved"))?;
                into.push(object_reference(context.views, facts.effective_type, key)?);
            }
        }
    }
    Ok((created, deleted))
}

fn raw_values_equal(left: Option<&RawValue>, right: Option<&RawValue>) -> bool {
    match (left, right) {
        (None, None) => true,
        (Some(left), Some(right)) => raw_value_eq(left, right),
        _ => false,
    }
}

fn raw_value_eq(left: &RawValue, right: &RawValue) -> bool {
    use RawValue::{Absent, Boolean, Integer, Present, Reference, Sequence};
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
