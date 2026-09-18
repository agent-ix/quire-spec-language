// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-153 closed-population queries (`allInstances<T>(p)`, `lookup<T>(p, r)
//! absent m`) as complete-V1 source forms.
//!
//! `crate::model::population`'s own module docs record that its typed
//! wrappers ([`ReferenceSet`](crate::model::population::ReferenceSet),
//! [`TypedReference`](crate::model::population::TypedReference)) stay in
//! `crate::model`'s own identity domain because "this crate defines no
//! mapping from `crate::model`'s `EffectiveId`/`ProducerKey` identities into
//! [`ObjectReference`]'s byte space anywhere. Fabricating one here without a
//! documented canonical encoding would risk a worse defect than the untyped
//! result it replaces." This module is that documented encoding: FR-143
//! defines a reference's `universe` and most-specific `type` as literally the
//! model's own `quire.model.object-universe/v1` and
//! `quire.model.effective-declaration/v1` digests, so the bridge is a direct
//! byte transfer, never a re-hash — [`NodeKey`]/[`UniverseIdentity`] carry no
//! domain tag of their own, and an `EffectiveId` is exactly 32 bytes, so the
//! two are the same bytes under different types.
//!
//! `crate::model::population::all_instances`/`lookup` already perform every
//! FR-153 charge (`lookup.key`, `lookup.result-retain`, `population.visit`,
//! `collection.bound`, `collection.result-retain`) against this crate's own
//! `quire.value.accounting/v1` [`Meter`] before returning their typed
//! outcome, so this module never charges again: it only bridges identities
//! and materializes the already-charged, already-ordered result as a
//! [`Value`].

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
    let object_type = NodeKey::from_hex(&key.type_identity.hex())
        .expect("an EffectiveId's hex is always 64 lowercase hex digits");
    let identity = ObjectIdentity::new(key.object.as_bytes()).map_err(|_| invariant())?;
    Ok(ObjectReference::new(universe, object_type, identity))
}

/// The reverse FR-143 byte transfer of an [`ObjectReference`] into a model
/// [`ReferenceKey`]. Fails only for a reference this bridge did not itself
/// produce and no admitted object environment holds either (a checked
/// invariant, per the module docs).
fn from_object_reference(reference: &ObjectReference) -> Result<ReferenceKey, Stop> {
    let universe_bytes: [u8; 32] = reference
        .universe()
        .as_bytes()
        .try_into()
        .map_err(|_| invariant())?;
    let universe = EffectiveId::from_digest_bytes(universe_bytes);
    let type_identity = EffectiveId::from_digest_bytes(*reference.object_type().as_bytes());
    let object =
        String::from_utf8(reference.identity().as_bytes().to_vec()).map_err(|_| invariant())?;
    Ok(ReferenceKey {
        universe,
        type_identity,
        object,
    })
}

/// The queried type `t`'s original [`ProducerKey`], resolved from a checked
/// `Reference<T>`'s `T` (a [`NodeKey`], the same 32 bytes as `t`'s
/// `EffectiveId`) through `binding`'s own [`PopulationBinding::type_catalog`].
/// `None` only for a `T` the admitted effective view never declared (a
/// checked invariant).
fn resolve_target(binding: &PopulationBinding, target: NodeKey) -> Result<ProducerKey, Stop> {
    let target_id = EffectiveId::from_digest_bytes(*target.as_bytes());
    binding
        .type_catalog()
        .iter()
        .find_map(|(producer, effective)| (*effective == target_id).then(|| producer.clone()))
        .ok_or_else(invariant)
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
    let target = resolve_target(binding, *target_key)?;
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
    let target = resolve_target(binding, target)?;
    let static_type = resolve_target(binding, static_type)?;
    let key = from_object_reference(&reference)?;
    let lookup_key = LookupKey { static_type, key };
    let option_payload = || match result_type {
        ValueType::Option(payload) => Ok((**payload).clone()),
        _ => Err(invariant()),
    };
    match lookup(binding, &target, &lookup_key, absence, meter) {
        LookupOutcome::Completed(Some(typed)) => {
            let found = Value::Reference(to_object_reference(typed.key())?);
            match absence {
                // `lookup` (`crate::model::population`) already proved `found`
                // conforms to the queried `T` via `type_conforms` before
                // returning it (FR-153's own selection check), so this uses
                // the same uncharged, unchecked bridge `collection::from_admitted`
                // uses for `allInstances`'s subtype members, not the checked
                // `OptionValue::present`: that constructor's `admits()` call is
                // exact-type structural equality with no model-conformance
                // knowledge (the "TypeEnvironment island" this crate's `value`
                // layer has no live import of `model`'s declaration/conformance
                // data to close — see this crate's item-3/5 deferrals), so it
                // would wrongly refuse every genuine upcast (`b1: M::B` present
                // as `Reference<M::A>`) that this operation's whole Outputs
                // clause exists to produce.
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
