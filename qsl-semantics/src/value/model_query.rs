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
//! digests. The type component is the kernel [`EffectiveId`] on both sides
//! (ADR-013 O-05), so it passes through unchanged: no `EffectiveId` <->
//! `NodeKey` transfer exists (ADR-010 OBS-018). The universe is the kernel
//! [`quire_exact::UniverseId`] `model` already computes
//! (`crate::model::population::ReferenceKey::universe`, ADR-013 §8 OQ-C
//! ruling), never re-hashed.
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
//! untrusted producer, but ADR-013 §8's OQ-C ruling now bounds what "not
//! well-formed" can mean: the kernel [`quire_exact::UniverseId`] is always a
//! 32-byte digest and the kernel [`quire_exact::ObjectId`] is always
//! non-empty, exact UTF-8 (its one constructor, `ObjectId::new`, refuses
//! anything else), so a `Value::Reference` can no longer carry a
//! wrong-length universe or invalid-UTF-8 object identity at all -- those
//! byte-level malformations are prevented at construction, not handled at
//! lookup time. What remains representable, and what [`lookup`] still
//! decides, is a *foreign* reference: a well-formed universe or object
//! identity that simply names no member of the bound population.
//! `bridge_lookup_key` never substitutes a derived value for either
//! component and never classifies it itself: it hands [`LookupKey`] `r`'s
//! own bytes exactly as supplied, and [`lookup`] alone decides the outcome,
//! in its own single order (`type_conforms(S, T)`, then `lookup.key`, then
//! the universe check, then membership or absence -- see [`LookupKey`]'s own
//! doc comment). `LookupKey.universe`/`.object` stay raw bytes there because
//! `crate::model::population`'s own direct callers (`tests/it/
//! model_population.rs`) still exercise `lookup`'s byte-level defenses
//! (wrong-length universe, non-UTF-8 identity) directly, bypassing
//! `ObjectReference` -- those callers, not this bridge, are where a
//! genuinely malformed `LookupKey` can still originate.
//!
//! # The TypeEnvironment island
//!
//! This module bridges identities, never model *conformance*, on its own:
//! `crate::value`'s `TypeEnvironment` (`crate::value::declaration`) admits no
//! generalization graph, so nothing on this side of the bridge can decide
//! whether one object type conforms to another by itself -- only
//! `crate::model::conformance` (reachable from a runtime
//! [`PopulationBinding`], never from a checked package's static types) can.
//! `allInstances`/`lookup`'s own conformance decisions
//! ([`all_instances`]/[`lookup`]) run entirely inside
//! `crate::model::population`, which does carry that graph -- including,
//! for a malformed reference, the short-circuit above, since [`lookup`]
//! itself decides `type_conforms(S, T)` before any charge for every
//! reference it is called with, well-formed or not. Three FR-153/FR-149
//! obligations that would need this graph at *check* time still cannot get
//! it, tracked at
//! <https://github.com/agent-ix/quire-spec-language/issues/164>: `deref(r).f`
//! display-name resolution, TC-198 L08 upcast equality, and refusing
//! `lookup<T>(p, r)` at check time when `r`'s declared type does not conform
//! to `T` (today refused only at evaluation).

use std::collections::HashMap;

use crate::model::key::{DeclarationKey, EffectiveId};
use crate::model::normalize::{ModelRefusal, ModelRefusalCause};
use crate::model::population::{
    all_instances, lookup, AllInstancesOutcome, LookupKey, LookupOutcome, PopulationBinding,
    ReferenceKey,
};
use qsl_foundation::absence::AbsenceMode;
use qsl_foundation::diagnostic::Code;

use super::stop::Stop;
use quire_exact::{
    from_admitted, CollectionType, Incomplete, Meter, ObjectId, ObjectReference, OptionValue,
    PopulationId, Refusal, Value, ValueType,
};

/// Why a population query stopped without a value. The evaluator
/// (`value::expression`, layer 5) turns `Refused` and `AbsentKey` into the
/// `StateModel` family's own evaluation-time results (FR-090-AC-8,
/// AC-12); this layer names no `check`-core outcome type (ADR-011 §6.2).
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ModelQueryHalt {
    /// An ordinary evaluator stop.
    Stop(Stop),
    /// `model::population` refused the query. Boxed: `ModelRefusal` is
    /// large, and this is the cold path.
    Refused(Box<ModelRefusal>),
    /// A `lookup<T>(p, r) absent undefined` query found no member of `r`'s
    /// key in the population bound to `p`.
    AbsentKey {
        /// The population bound to `p`.
        population: PopulationId,
        /// `r`'s object identity bytes, exactly as supplied.
        key: Vec<u8>,
    },
}

impl From<Incomplete> for ModelQueryHalt {
    fn from(record: Incomplete) -> Self {
        Self::Stop(Stop::Incomplete(record))
    }
}

/// A reference's identity triple could not be bridged; unreachable for an
/// admitted program (see the module docs).
fn invariant() -> ModelQueryHalt {
    ModelQueryHalt::Stop(Stop::Refused(Refusal::CheckedInvariant))
}

/// The FR-143 conversion of a model [`ReferenceKey`] into its kernel
/// [`ObjectReference`]: the universe is already the same
/// [`quire_exact::UniverseId`] (ADR-013 §8 OQ-C ruling), the type is the same
/// [`EffectiveId`], and the object identity is minted as an
/// [`quire_exact::ObjectId`] from `key.object`'s own authored bytes. Fails
/// only when the object identity is empty, which an admitted
/// [`PopulationBinding`] never produces (a checked invariant).
fn to_object_reference(key: &ReferenceKey) -> Result<ObjectReference, ModelQueryHalt> {
    let object = ObjectId::new(key.object.clone()).map_err(|_| invariant())?;
    Ok(ObjectReference::new(
        key.universe,
        key.type_identity,
        object,
    ))
}

/// `reference`'s own identity triple as a [`LookupKey`] for [`lookup`],
/// paired with `static_type`. Always succeeds (see the module docs): the
/// type is `reference`'s own [`EffectiveId`], and `universe` and `object` are
/// its own bytes, exactly as supplied, never bridged, classified or
/// substituted here -- [`lookup`] alone decides whether `universe` or
/// `object` names a member.
fn bridge_lookup_key(static_type: DeclarationKey, reference: &ObjectReference) -> LookupKey {
    LookupKey {
        static_type,
        universe: reference.universe().as_bytes().to_vec(),
        type_identity: reference.object_type(),
        object: reference.object().as_str().as_bytes().to_vec(),
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
        .map(|(producer, effective)| (*effective, producer.clone()))
        .collect()
}

/// The queried type `t`'s original [`DeclarationKey`], resolved from a checked
/// `Reference<T>`'s `T` (its [`EffectiveId`]) through `catalog` (see
/// [`reverse_catalog`]). `Err` for a
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
    target: EffectiveId,
) -> Result<DeclarationKey, ModelQueryHalt> {
    catalog.get(&target).cloned().ok_or_else(|| {
        ModelQueryHalt::Refused(Box::new(ModelRefusal {
            code: Code::IllTyped,
            cause: ModelRefusalCause::TypeMismatch,
            detail: format!("{target} is not a declared type of this population's model"),
        }))
    })
}

/// `allInstances<T>(p)` (FR-153). `collection_type` is this call's own
/// checked `Set<Reference<T>>[0,N]` result type
/// (`crate::value::expression::ir::NodeKind::AllInstances`'s node's own
/// `value_type`); its element names `T`.
pub fn evaluate_all_instances(
    binding: &PopulationBinding,
    collection_type: &CollectionType,
    meter: &mut Meter,
) -> Result<Value, ModelQueryHalt> {
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
            Ok(from_admitted(collection_type.clone(), elements))
        }
        AllInstancesOutcome::Refused(refusal) => Err(ModelQueryHalt::Refused(Box::new(refusal))),
        AllInstancesOutcome::Incomplete(incomplete) => Err(incomplete.into()),
    }
}

/// `lookup<T>(p, r) absent m` (FR-153). `target` is `T`; `static_type` is
/// `r`'s own declared static type `S` (`crate::value::expression::ir::Node`'s
/// generic `value_type` field on the `reference` child, read by the caller);
/// `result_type` is this call's own checked result type (a bare
/// `Reference<T>` for `undefined`/`refused`, an `Option<Reference<T>>` for
/// `empty`, per FR-153's own table).
pub fn evaluate_lookup(
    binding: &PopulationBinding,
    target: EffectiveId,
    static_type: EffectiveId,
    reference: Value,
    absence: AbsenceMode,
    result_type: &ValueType,
    meter: &mut Meter,
) -> Result<Value, ModelQueryHalt> {
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
        LookupOutcome::Undefined => Err(ModelQueryHalt::AbsentKey {
            population: binding.population_id(),
            key: lookup_key.object,
        }),
        LookupOutcome::Refused(refusal) => Err(ModelQueryHalt::Refused(Box::new(refusal))),
        LookupOutcome::Incomplete(incomplete) => Err(incomplete.into()),
    }
}

fn option_payload(result_type: &ValueType) -> Result<ValueType, ModelQueryHalt> {
    match result_type {
        ValueType::Option(payload) => Ok((**payload).clone()),
        _ => Err(invariant()),
    }
}
