// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-153 closed-population queries (`allInstances<T>(p)`, `lookup<T>(p, r)
//! absent m`) as complete-V1 source forms.
//!
//! `crate::model::population`'s typed wrappers
//! ([`ReferenceSet`](crate::model::population::ReferenceSet),
//! [`TypedReference`](crate::model::population::TypedReference)) stay in
//! `crate::model`'s own identity domain rather than constructing
//! [`ObjectReference`] themselves; this module is the documented canonical
//! encoding that bridges the two. FR-143 defines a reference's `universe`
//! and most-specific `type` as literally the model's own
//! `quire.model.object-universe/v1` and `quire.model.effective-declaration/v1`
//! digests, so the bridge is a direct byte transfer, never a re-hash --
//! [`NodeKey`]/[`UniverseIdentity`] carry no domain tag of their own, and an
//! `EffectiveId` is exactly 32 bytes, so the two are the same bytes under
//! different types.
//!
//! `crate::model::population::all_instances`/`lookup` already perform every
//! FR-153 charge (`lookup.key`, `lookup.result-retain`, `population.visit`,
//! `collection.bound`, `collection.result-retain`) against this crate's own
//! `quire.value.accounting/v1` [`Meter`] before returning their typed
//! outcome, so this module never charges again for a well-formed reference:
//! it only bridges identities and materializes the already-charged,
//! already-ordered result as a [`Value`]. The two malformed-reference cases
//! that can never even become a well-formed [`ReferenceKey`] -- a universe
//! that is not the model's own 32 bytes, or an object identity that is not
//! valid UTF-8 -- are folded into a value that provably cannot be this
//! binding's own universe, or (for all practical purposes) any real member's
//! own identity, so `lookup`'s own single `lookup.key` charge and its own
//! `foreign-universe`/absent-key handling still decide them through the one
//! function that owns that logic, rather than a second copy of it here; see
//! [`bridged_universe`]/[`bridged_object`].
//!
//! # The TypeEnvironment island
//!
//! This module bridges identities, never model *conformance*: `crate::value`'s
//! `TypeEnvironment` (`crate::value::composite`) admits no generalization
//! graph, so nothing on this side of the bridge can decide whether one
//! object type conforms to another -- only `crate::model::conformance`
//! (reachable from a runtime [`PopulationBinding`], never from a checked
//! package's static types) can. `allInstances`/`lookup`'s own conformance
//! decisions ([`all_instances`]/[`lookup`]) are unaffected by this -- they
//! run entirely inside `crate::model::population`, which does carry that
//! graph -- but three FR-153/FR-149 obligations that would need it at
//! *check* time cannot get it, tracked at
//! <https://github.com/agent-ix/quire-spec-language/issues/164>: FR-153-AC-6
//! upcast equality, `deref(r).f` display-name resolution, and refusing
//! `lookup<T>(p, r)` at check time when `r`'s declared type does not conform
//! to `T` (today refused only at evaluation, inside [`lookup`]'s own
//! `type_conforms` call).

use std::collections::HashMap;

use crate::diagnostic::Code;
use crate::model::key::{EffectiveId, ProducerKey};
use crate::model::normalize::ModelRefusal;
use crate::model::population::{
    all_instances, lookup, AbsenceMode, AllInstancesOutcome, LookupKey, LookupOutcome,
    PopulationBinding, ReferenceKey,
};

use super::accounting::Meter;
use super::collection::{self, CollectionType};
use super::composite::{OptionValue, Value, ValueType};
use super::node::NodeKey;
use super::outcome::{ModelQueryRefusal, Refusal, Stop, Undefined};
use super::reference::{ObjectIdentity, ObjectReference, UniverseIdentity};

/// A reference's identity triple could not be bridged; unreachable for an
/// admitted program (see the module docs).
fn invariant() -> Stop {
    Stop::Refused(Refusal::CheckedInvariant)
}

fn model_refusal(refusal: ModelRefusal) -> Stop {
    Stop::Refused(Refusal::Model(ModelQueryRefusal {
        code: refusal.code,
        cause: refusal.cause,
    }))
}

/// The direct FR-143 byte transfer of a model [`ReferenceKey`] into its
/// `crate::value` [`ObjectReference`]. Fails only when a component's bytes
/// are empty, which an admitted [`PopulationBinding`] never produces (a
/// checked invariant).
fn to_object_reference(key: &ReferenceKey) -> Result<ObjectReference, Stop> {
    let universe = UniverseIdentity::new(key.universe.as_bytes()).map_err(|_| invariant())?;
    let object_type = NodeKey::from_bytes(*key.type_identity.as_bytes());
    let identity = ObjectIdentity::new(key.object.as_bytes()).map_err(|_| invariant())?;
    Ok(ObjectReference::new(universe, object_type, identity))
}

/// The universe component of `reference`, bridged into a model
/// [`EffectiveId`]. Every real binding's own universe
/// (`crate::model::normalize::object_universe`) is exactly 32 bytes, so a
/// `reference` whose universe is any other length can never name it. A local
/// refusal here could not honor FR-153's "type-mismatch before any charge"
/// ordering the way [`lookup`] itself does (the "TypeEnvironment island" --
/// see the module docs -- keeps `crate::model::conformance::type_conforms` a
/// call this module cannot make), so instead of refusing, this substitutes
/// the binding's own universe bit-complemented: a value that can never equal
/// it, so `lookup`'s single `lookup.key` charge and its own
/// `foreign-universe` refusal (never duplicated here) still decide it.
fn bridged_universe(reference: &ObjectReference, binding: &PopulationBinding) -> EffectiveId {
    match <[u8; 32]>::try_from(reference.universe().as_bytes()) {
        Ok(bytes) => EffectiveId::from_digest_bytes(bytes),
        Err(_) => {
            let mut complement = *binding.universe().as_bytes();
            complement.iter_mut().for_each(|byte| *byte = !*byte);
            EffectiveId::from_digest_bytes(complement)
        }
    }
}

/// The object-identity component of `reference`, bridged into a model key
/// string. Every real population member's object identity
/// (`crate::model::population::PopulationDocument`'s member records) is a
/// JSON string, so a `reference` whose identity bytes are not valid UTF-8
/// can never name one. This falls back to a lossy conversion -- the U+FFFD
/// replacement character makes a collision with any real member's own
/// identity vanishingly unlikely -- so `lookup`'s own member lookup and
/// absence-mode dispatch (never duplicated here) still decide it.
fn bridged_object(reference: &ObjectReference) -> String {
    String::from_utf8(reference.identity().as_bytes().to_vec())
        .unwrap_or_else(|error| String::from_utf8_lossy(error.as_bytes()).into_owned())
}

/// The reverse FR-143 byte transfer of an [`ObjectReference`] into a model
/// [`ReferenceKey`] against `binding`'s own universe; see
/// [`bridged_universe`]/[`bridged_object`]. Never fails.
fn from_object_reference(reference: &ObjectReference, binding: &PopulationBinding) -> ReferenceKey {
    ReferenceKey {
        universe: bridged_universe(reference, binding),
        type_identity: EffectiveId::from_digest_bytes(*reference.object_type().as_bytes()),
        object: bridged_object(reference),
    }
}

/// A one-time reverse index of `binding`'s own
/// [`PopulationBinding::type_catalog`], built once per query rather than
/// scanned linearly once per resolved type (`allInstances` resolves one
/// type; `lookup` resolves two).
fn reverse_catalog(binding: &PopulationBinding) -> HashMap<EffectiveId, ProducerKey> {
    binding
        .type_catalog()
        .iter()
        .map(|(producer, effective)| (effective.clone(), producer.clone()))
        .collect()
}

/// The queried type `t`'s original [`ProducerKey`], resolved from a checked
/// `Reference<T>`'s `T` (a [`NodeKey`], the same 32 bytes as `t`'s
/// `EffectiveId`) through `catalog` (see [`reverse_catalog`]). `Err` for a
/// `T` the checked package declares but this particular runtime binding's
/// model does not -- a real FR-153 `ill_typed`/`type-mismatch`, the same
/// cause `crate::model::population::all_instances`'s own `is_object_type`
/// gives a declared type the model doesn't recognize, never a checked
/// invariant: the checker cannot tie a package's declared object types to a
/// particular runtime binding's model (the "TypeEnvironment island" -- see
/// the module docs -- runs the other way here too, since which types a
/// *binding* declares is model data, not package data).
fn resolve_target(
    catalog: &HashMap<EffectiveId, ProducerKey>,
    target: NodeKey,
) -> Result<ProducerKey, Stop> {
    let target_id = EffectiveId::from_digest_bytes(*target.as_bytes());
    catalog.get(&target_id).cloned().ok_or_else(|| {
        model_refusal(ModelRefusal {
            code: Code::IllTyped,
            cause: "type-mismatch",
            detail: format!(
                "{} is not a declared type of this population's model",
                target_id.hex()
            ),
        })
    })
}

/// `allInstances<T>(p)` (FR-153). `collection_type` is this call's own
/// checked `Set<Reference<T>>[0,N]` result type
/// (`crate::value::expression::ir::NodeKind::AllInstances`'s node's own
/// `value_type`); its element names `T`.
pub(crate) fn evaluate_all_instances(
    binding: &PopulationBinding,
    collection_type: &CollectionType,
    meter: &mut Meter,
) -> Result<Value, Stop> {
    let ValueType::Reference(target_key) = collection_type.element() else {
        return Err(invariant());
    };
    let catalog = reverse_catalog(binding);
    let target = resolve_target(&catalog, *target_key)?;
    match all_instances(binding, &target, meter) {
        AllInstancesOutcome::Completed(set) => {
            let mut elements = Vec::with_capacity(set.len());
            for key in set.members() {
                elements.push(Value::Reference(to_object_reference(key)?));
            }
            Ok(collection::from_admitted(collection_type.clone(), elements))
        }
        AllInstancesOutcome::Refused(refusal) => Err(model_refusal(refusal)),
        AllInstancesOutcome::Incomplete(incomplete) => Err(Stop::Incomplete(incomplete)),
    }
}

/// `lookup<T>(p, r) absent m` (FR-153). `target` is `T`; `static_type` is
/// `r`'s own declared static type `S` (`crate::value::expression::ir::Node`'s
/// generic `value_type` field on the `reference` child, read by the caller);
/// `result_type` is this call's own checked result type (a bare
/// `Reference<T>` for `undefined`/`refused`, an `Option<Reference<T>>` for
/// `empty`, per FR-153's own table).
pub(crate) fn evaluate_lookup(
    binding: &PopulationBinding,
    target: NodeKey,
    static_type: NodeKey,
    reference: Value,
    absence: AbsenceMode,
    result_type: &ValueType,
    meter: &mut Meter,
) -> Result<Value, Stop> {
    let Value::Reference(reference) = reference else {
        return Err(invariant());
    };
    let catalog = reverse_catalog(binding);
    let target = resolve_target(&catalog, target)?;
    let static_type = resolve_target(&catalog, static_type)?;
    let key = from_object_reference(&reference, binding);
    let lookup_key = LookupKey { static_type, key };
    let option_payload = || match result_type {
        ValueType::Option(payload) => Ok((**payload).clone()),
        _ => Err(invariant()),
    };
    match lookup(binding, &target, &lookup_key, absence, meter) {
        LookupOutcome::Completed(Some(typed)) => {
            let found = Value::Reference(to_object_reference(typed.key())?);
            match absence {
                // `found`'s `object_type` is `r`'s own runtime most-specific
                // type `F` (FR-143's identity triple), not `T`; see
                // `OptionValue::from_admitted`'s own doc comment for the
                // soundness chain (`F == S` by parameter admission, `S`
                // conforms to `T` by `lookup`'s own `type_conforms` call) that
                // lets this bypass the checked `OptionValue::present`, whose
                // structural `admits()` call would wrongly refuse every
                // genuine upcast (`b1: M::B` present as `Reference<M::A>`)
                // that this operation's whole Outputs clause exists to
                // produce.
                AbsenceMode::Empty => {
                    Ok(OptionValue::from_admitted(option_payload()?, Some(found)))
                }
                AbsenceMode::Undefined | AbsenceMode::Refused => Ok(found),
            }
        }
        LookupOutcome::Completed(None) => Ok(OptionValue::from_admitted(option_payload()?, None)),
        LookupOutcome::Undefined => Err(Stop::Undefined(Undefined::AbsentKey)),
        LookupOutcome::Refused(refusal) => Err(model_refusal(refusal)),
        LookupOutcome::Incomplete(incomplete) => Err(Stop::Incomplete(incomplete)),
    }
}
