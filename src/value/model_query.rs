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
//! already-ordered result as a [`Value`].
//!
//! Two malformed-reference cases can never even become a well-formed
//! [`ReferenceKey`]: a universe that is not the model's own 32 bytes, or an
//! object identity that is not valid UTF-8. An earlier version of this module
//! substituted a derived value for the malformed bytes (the universe's own
//! bit-complement; a lossy UTF-8 decode of the identity) and delegated to
//! [`lookup`] as if the reference had always carried that substitute. The
//! universe substitution is sound -- a bit-complement of a real 32-byte
//! universe can never equal it -- but the identity substitution is not: the
//! lossy decode of arbitrary invalid bytes can coincide with a real, validly
//! admitted member's own identity string (`String::from_utf8_lossy(&[0xFF,
//! 0xFE])` is `"\u{FFFD}\u{FFFD}"`, a value a population is free to admit),
//! which would let a producer-supplied malformed reference be mistaken for
//! that member across all three absence modes. [`evaluate_lookup`] now takes
//! no substitution at all for either component: [`bridge_reference`] reports
//! which component (if any) cannot be losslessly bridged, and
//! [`evaluate_unresolvable_lookup`] then decides `type_conforms(S, T)` first,
//! charges `lookup.key` once, and dispatches directly to the same outcome
//! [`lookup`] itself would reach for a structurally-absent member --
//! `foreign-universe` for a malformed universe (reporting the bytes the
//! caller actually supplied), or the mode's own absence outcome for a
//! malformed identity -- without ever constructing a key that could alias a
//! real member.
//!
//! # The TypeEnvironment island
//!
//! This module bridges identities, never model *conformance*, on its own:
//! `crate::value`'s `TypeEnvironment` (`crate::value::composite`) admits no
//! generalization graph, so nothing on this side of the bridge can decide
//! whether one object type conforms to another by itself -- only
//! `crate::model::conformance` (reachable from a runtime
//! [`PopulationBinding`], never from a checked package's static types) can.
//! `allInstances`/`lookup`'s own conformance decisions
//! ([`all_instances`]/[`lookup`]) run entirely inside
//! `crate::model::population`, which does carry that graph;
//! [`evaluate_unresolvable_lookup`]'s own conformance decision, needed for
//! the malformed-reference short-circuit above, goes through
//! [`crate::model::population::conforms`], a narrow `pub(crate)` door onto
//! the same graph -- evaluation-time-only, since a [`PopulationBinding`] is
//! already in scope here. Three FR-153/FR-149 obligations that would need
//! this graph at *check* time still cannot get it, tracked at
//! <https://github.com/agent-ix/quire-spec-language/issues/164>: `deref(r).f`
//! display-name resolution, TC-198 L08 upcast equality, and refusing
//! `lookup<T>(p, r)` at check time when `r`'s declared type does not conform
//! to `T` (today refused only at evaluation).

use std::collections::HashMap;

use crate::diagnostic::Code;
use crate::model::key::{EffectiveId, ProducerKey};
use crate::model::normalize::{ModelRefusal, ModelRefusalCause};
use crate::model::population::{
    all_instances, conforms, lookup, AbsenceMode, AllInstancesOutcome, LookupKey, LookupOutcome,
    PopulationBinding, ReferenceKey,
};

use super::accounting::{Charge, ChargePoint, LimitKind, Meter};
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
        cause: refusal.cause.as_str(),
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

/// Which of `reference`'s two identity-triple components (see the module
/// docs) could not be losslessly bridged into a well-formed [`ReferenceKey`]
/// -- so [`evaluate_lookup`] must decide the outcome directly
/// ([`evaluate_unresolvable_lookup`]) rather than build a key and delegate to
/// [`lookup`].
enum Unbridgeable {
    /// Not exactly 32 bytes: every real binding's own universe
    /// (`crate::model::normalize::object_universe`) is exactly 32 bytes, so a
    /// `reference` whose universe is any other length can never name it.
    Universe,
    /// Not valid UTF-8: every real population member's own object identity
    /// (`crate::model::population::PopulationDocument`'s member records) is a
    /// JSON string, so a `reference` whose identity bytes are not valid UTF-8
    /// can never name one. Carries the reference's own universe -- already
    /// losslessly parsed by `bridge_reference` -- so
    /// `evaluate_unresolvable_lookup` can check it against `binding`'s own
    /// universe before deciding absence, exactly as `lookup` checks the
    /// universe before membership. Never re-derived or re-parsed: this is
    /// the same bytes `bridge_reference` already validated once.
    Identity { universe: EffectiveId },
}

/// The reverse FR-143 byte transfer of an [`ObjectReference`] into a model
/// [`ReferenceKey`], or which component made that impossible. No
/// substitution: an earlier version of this bridge substituted a derived
/// value for either malformed component and delegated to [`lookup`] as if
/// the reference had always carried it, which is unsound for the identity
/// case (see the module docs) and is not attempted here at all.
fn bridge_reference(reference: &ObjectReference) -> Result<ReferenceKey, Unbridgeable> {
    let universe: [u8; 32] = reference
        .universe()
        .as_bytes()
        .try_into()
        .map_err(|_| Unbridgeable::Universe)?;
    let universe = EffectiveId::from_digest_bytes(universe);
    let object = String::from_utf8(reference.identity().as_bytes().to_vec()).map_err(|_| {
        Unbridgeable::Identity {
            universe: universe.clone(),
        }
    })?;
    Ok(ReferenceKey {
        universe,
        type_identity: EffectiveId::from_digest_bytes(*reference.object_type().as_bytes()),
        object,
    })
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
            cause: ModelRefusalCause::TypeMismatch,
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

    let key = match bridge_reference(&reference) {
        Ok(key) => key,
        Err(failure) => {
            return evaluate_unresolvable_lookup(
                binding,
                &static_type,
                &target,
                &reference,
                failure,
                absence,
                result_type,
                meter,
            )
        }
    };
    let lookup_key = LookupKey { static_type, key };
    match lookup(binding, &target, &lookup_key, absence, meter) {
        LookupOutcome::Completed(Some(typed)) => {
            let found = Value::Reference(to_object_reference(typed.key())?);
            match absence {
                // `found`'s `object_type` is `r`'s own runtime most-specific
                // type `F` (FR-143's identity triple), not `T`; see
                // `OptionValue::from_admitted`'s own doc comment for the
                // soundness chain (`F` conforms to `S` by parameter
                // admission, `S` conforms to `T` by `lookup`'s own
                // `type_conforms` call) that lets this bypass the checked
                // `OptionValue::present`, whose structural `admits()` call
                // would wrongly refuse every genuine upcast (`b1: M::B`
                // present as `Reference<M::A>`) that this operation's whole
                // Outputs clause exists to produce.
                AbsenceMode::Empty => Ok(OptionValue::from_admitted(
                    option_payload(result_type)?,
                    Some(found),
                )),
                AbsenceMode::Undefined | AbsenceMode::Refused => Ok(found),
            }
        }
        LookupOutcome::Completed(None) => Ok(OptionValue::from_admitted(
            option_payload(result_type)?,
            None,
        )),
        LookupOutcome::Undefined => Err(Stop::Undefined(Undefined::AbsentKey)),
        LookupOutcome::Refused(refusal) => Err(model_refusal(refusal)),
        LookupOutcome::Incomplete(incomplete) => Err(Stop::Incomplete(incomplete)),
    }
}

fn option_payload(result_type: &ValueType) -> Result<ValueType, Stop> {
    match result_type {
        ValueType::Option(payload) => Ok((**payload).clone()),
        _ => Err(invariant()),
    }
}

/// Lowercase hex of `bytes`, for a refusal detail that must report exactly
/// what the caller supplied, never a derived or substituted value.
fn hex_bytes(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// `reference` could not be bridged into a well-formed [`ReferenceKey`]
/// (`failure` says which component); decide the outcome [`lookup`] itself
/// would reach for a structurally-absent member, without ever constructing a
/// key that could alias a real one. Mirrors [`lookup`]'s own ordering
/// exactly: `type_conforms(S, T)` before any charge, then `lookup.key`, then
/// the universe check, then the outcome -- `foreign-universe` for a
/// malformed universe, or for a malformed identity naming another universe
/// (`bridge_reference` still parses the universe on that path), or the
/// mode's own absence outcome (with `empty` mode's extra
/// `lookup.result-retain`) for a malformed identity naming this binding's
/// own universe.
#[allow(clippy::too_many_arguments)]
fn evaluate_unresolvable_lookup(
    binding: &PopulationBinding,
    static_type: &ProducerKey,
    target: &ProducerKey,
    reference: &ObjectReference,
    failure: Unbridgeable,
    absence: AbsenceMode,
    result_type: &ValueType,
    meter: &mut Meter,
) -> Result<Value, Stop> {
    match conforms(binding, static_type, target) {
        Ok(true) => {}
        Ok(false) => {
            return Err(model_refusal(ModelRefusal {
                code: Code::IllTyped,
                cause: ModelRefusalCause::TypeMismatch,
                detail: format!(
                    "{} does not conform to {}",
                    static_type.identity, target.identity
                ),
            }))
        }
        Err(refusal) => return Err(model_refusal(refusal)),
    }

    if let Err(incomplete) =
        meter.charge(Charge::new(ChargePoint::LookupKey).size(LimitKind::ValueOccurrences, 1))
    {
        return Err(Stop::Incomplete(incomplete));
    }

    match failure {
        Unbridgeable::Universe => Err(model_refusal(ModelRefusal {
            code: Code::ForeignReference,
            cause: ModelRefusalCause::ForeignUniverse {
                actual: reference.universe().as_bytes().to_vec(),
                expected: binding.universe().clone(),
            },
            detail: format!(
                "reference key names universe {}, not the binding's {}",
                hex_bytes(reference.universe().as_bytes()),
                binding.universe().hex()
            ),
        })),
        Unbridgeable::Identity { universe } if universe != *binding.universe() => {
            Err(model_refusal(ModelRefusal {
                code: Code::ForeignReference,
                cause: ModelRefusalCause::ForeignUniverse {
                    actual: universe.as_bytes().to_vec(),
                    expected: binding.universe().clone(),
                },
                detail: format!(
                    "reference key names universe {}, not the binding's {}",
                    universe.hex(),
                    binding.universe().hex()
                ),
            }))
        }
        Unbridgeable::Identity { .. } => match absence {
            AbsenceMode::Undefined => Err(Stop::Undefined(Undefined::AbsentKey)),
            AbsenceMode::Refused => Err(model_refusal(ModelRefusal {
                code: Code::InvalidRuntimeInput,
                cause: ModelRefusalCause::AbsentKey {
                    key: reference.identity().as_bytes().to_vec(),
                },
                detail: format!(
                    "{} is not a member of the bound population",
                    hex_bytes(reference.identity().as_bytes())
                ),
            })),
            AbsenceMode::Empty => {
                if let Err(incomplete) = meter.charge(
                    Charge::new(ChargePoint::LookupResultRetain)
                        .size(LimitKind::ValueOccurrences, 1)
                        .results(1),
                ) {
                    return Err(Stop::Incomplete(incomplete));
                }
                Ok(OptionValue::from_admitted(
                    option_payload(result_type)?,
                    None,
                ))
            }
        },
    }
}
