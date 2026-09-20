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
//! # Malformed references
//!
//! A checked `Reference<T>` value's identity triple is supplied by an
//! untrusted producer, so not every component is well-formed: its universe
//! ([`UniverseIdentity`]) or object identity ([`ObjectIdentity`]) may carry
//! bytes no real [`PopulationBinding`] could ever admit. [`bridge_lookup_key`]
//! never substitutes a derived value for either component, never classifies
//! either one itself, and cannot itself fail: it hands [`LookupKey`] `r`'s
//! own raw bytes exactly as supplied, and [`lookup`] alone decides the
//! outcome for a well-formed and a malformed reference alike, in its own
//! single order (`type_conforms(S, T)`, then `lookup.key`, then the universe
//! check, then membership or absence -- see [`LookupKey`]'s own doc
//! comment). A universe is carried as its raw bytes rather than a bridged
//! [`EffectiveId`]: a length other than 32 can never equal a real universe,
//! so [`lookup`]'s own byte comparison already decides it correctly with no
//! separate malformed case. An object identity is carried as its own raw
//! bytes too, never a lossily decoded substitute: [`lookup`] treats them as
//! a candidate member identity only when they are valid UTF-8 (every real
//! population member's own identity is a JSON string, `PopulationDocument`'s
//! member records), since a lossy decode of arbitrary invalid bytes can
//! coincide with a real, validly admitted member's own identity string
//! (`String::from_utf8_lossy(&[0xFF, 0xFE])` is `"\u{FFFD}\u{FFFD}"`, a value
//! a population is free to admit) and so could let a malformed reference be
//! mistaken for that member.
//!
//! # The TypeEnvironment island
//!
//! This module bridges identities; model *conformance* still runs entirely
//! inside `crate::model::population`/`crate::model::conformance`, reachable
//! from a runtime [`PopulationBinding`], never from a checked package's
//! static types -- including, for a malformed reference, the short-circuit
//! above, since [`lookup`] itself decides `type_conforms(S, T)` before any
//! charge for every reference it is called with, well-formed or not.
//!
//! `crate::value`'s `TypeEnvironment` (`crate::value::composite`) *does* now
//! admit its own object-type generalization graph (`ancestors`/`conforms`,
//! PR #204) for the checked package's own declared supertypes, and the
//! checker uses it directly for TC-198 L08 upcast equality and check-time
//! `lookup<T>(p, r)` S-vs-T conformance (`crate::value::expression::check`,
//! `crate::value::equality`) -- no bridge through this module is needed for
//! either. Two things this graph still cannot do, tracked at
//! <https://github.com/agent-ix/quire-spec-language/issues/164>: resolve
//! `deref(r).f` through an inherited, non-overridden field (a storage
//! question -- `ObjectEnvironment`'s slots are sized to an object's own
//! declared type alone, so an ancestor-only field has nowhere to live on a
//! subtype's instance, not just nowhere to look it up); and reach a real
//! *production* checked package at all -- `crate::model::checked_dispatch`'s
//! sole production `PackageDeclarations` builder leaves `types` at
//! `TypeEnvironment::default()` (empty), so today `ancestors`/`conforms` are
//! exercised only where a caller hand-builds a `TypeEnvironment`, as tests
//! do. `TypeEnvironment::conforms` is also a precomputed, unbounded-depth
//! transitive closure, unlike `type_conforms`'s bounded (128-step) walk
//! here, so a supertype chain deeper than that is checker-admitted and
//! evaluation-`ResourceExhausted`-refused -- a real, currently accepted
//! divergence between the two, not a soundness gap (evaluation still
//! refuses, never silently admits).

use std::collections::HashMap;

use crate::diagnostic::Code;
use crate::model::key::{DeclarationKey, EffectiveId};
use crate::model::normalize::{ModelRefusal, ModelRefusalCause};
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

/// The FR-143 byte transfer of `reference`'s own identity triple into a
/// [`LookupKey`] for [`lookup`], paired with `static_type`. Always succeeds
/// (see the module docs): every component is carried as `reference`'s own
/// raw bytes, exactly as supplied, never bridged, classified or substituted
/// here -- [`lookup`] alone decides whether `universe` or `object` is
/// well-formed.
fn bridge_lookup_key(static_type: DeclarationKey, reference: &ObjectReference) -> LookupKey {
    LookupKey {
        static_type,
        universe: reference.universe().as_bytes().to_vec(),
        type_identity: EffectiveId::from_digest_bytes(*reference.object_type().as_bytes()),
        object: reference.identity().as_bytes().to_vec(),
    }
}

/// A one-time reverse index of `binding`'s own
/// [`PopulationBinding::type_catalog`], built once per query rather than
/// scanned linearly once per resolved type (`allInstances` resolves one
/// type; `lookup` resolves two).
fn reverse_catalog(binding: &PopulationBinding) -> HashMap<EffectiveId, DeclarationKey> {
    binding
        .type_catalog()
        .iter()
        .map(|(producer, effective)| (effective.clone(), producer.clone()))
        .collect()
}

/// The queried type `t`'s original [`DeclarationKey`], resolved from a checked
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
    catalog: &HashMap<EffectiveId, DeclarationKey>,
    target: NodeKey,
) -> Result<DeclarationKey, Stop> {
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

    let lookup_key = bridge_lookup_key(static_type, &reference);
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
