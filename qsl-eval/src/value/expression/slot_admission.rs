// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-049-AC-2/AC-3: admission of a caller-supplied object environment
//! against the evaluating package's own declarations.
//!
//! `TypeEnvironment::admits` accepts a record by its declaration key, on the
//! premise that its members were admitted against that declaration. A value
//! in a caller-built `ObjectClosure` carries no such premise, so every record
//! member reachable through options, collections, nested records and union
//! payloads is admitted here against the package's own record declaration.
//! The closure names each slot by attribute name, so a slot is found by name
//! whatever order the caller's environment gave the attributes.
//!
//! [`admit_objects`] admits the whole environment at a call or expression
//! entry, before any charge. [`admits_slot`] is the same check for one read;
//! the reads keep it because the protocol, temporal and clause entries reach
//! the evaluator without passing [`admit_objects`]. The walks are explicit
//! worklists, so a deep value cannot overflow the stack.

use std::collections::BTreeMap;

use qsl_semantics::model::object_environment::ObjectEnvironment;
use qsl_semantics::model::observation::OutOfRange;
use quire_eval::{CallAdmissionFailure, CallAdmissionLocus, CallAdmissionRecord, CallFailure};
use quire_exact::{FieldValue, ObjectReference, Presence, Value, ValueType};
use quire_semantic_value::admission::{AdmissionRecord, PathStep};
use quire_semantic_value::catalog::{CatalogCode, InternalFault};
use quire_semantic_value::declaration::{
    CompositeShape, EnvironmentFailure, FieldDeclaration, TypeEnvironment,
};
use quire_semantic_value::supplied_stop::{SuppliedPhase, SuppliedRun};

/// Admit every object of `objects` against `types`: each object's type is a
/// model object type of the package, each attribute the package declares
/// for it has an admitted slot ([`admits_slot`]), and the caller declared no
/// attribute the package does not. A defect is caller input and refuses
/// `invalid_runtime_input`/`wrong-value-kind` naming the object and the
/// attribute; nothing is charged to the meter.
///
/// `run` is the call's supplied-value run (FR-277): the walk polls its
/// original cancellation before each object, stopping as a supplied-value
/// admission stop at `position`, the position after the last argument. It
/// charges nothing to the run's admission budget, which counts the supplied
/// arguments' membership events and has no unit for an environment value;
/// the walk is linear in the environment, whose size its own admission
/// (`ObjectClosure::new`, FR-106 document limits) already bounded.
pub(super) fn admit_objects(
    objects: &ObjectEnvironment,
    types: &TypeEnvironment,
    run: &mut SuppliedRun,
    position: usize,
) -> Result<(), CallFailure> {
    for (reference, held) in objects.objects().iter() {
        run.check_cancelled(SuppliedPhase::Admission, position)?;
        let object_type = reference.object_type().clone();
        let Some(declared) = types.attributes(object_type.clone()) else {
            return Err(refused(reference, None));
        };
        for attribute in declared {
            let field = attribute.field();
            let Some(slot) = held.get(field.name()) else {
                return Err(refused(reference, Some(field.name())));
            };
            let identity = attribute.identity();
            let witnesses: Vec<&OutOfRange> =
                objects.witnesses(types, reference, &identity).collect();
            if !admits_slot(types, field, slot, &witnesses, objects.holds_out_of_range())
                .map_err(entry_fault)?
            {
                return Err(refused(reference, Some(field.name())));
            }
        }
        if let Some((surplus, _)) = held
            .iter()
            .find(|(name, _)| types.attribute(object_type.clone(), name).is_none())
        {
            return Err(refused(reference, Some(surplus)));
        }
    }
    Ok(())
}

/// The locus member an object-environment entry refusal names.
const OBJECT_ENVIRONMENT: &str = "object-environment";

/// The record of `failure` when it is [`admit_objects`]'s refusal of the
/// object environment -- caller input, refused before any charge, that a
/// caller maps to its own typed input refusal -- and `None` for any other
/// call admission failure.
pub fn object_environment_refusal(failure: &CallAdmissionFailure) -> Option<&AdmissionRecord> {
    match failure {
        CallAdmissionFailure::Refused(CallAdmissionRecord {
            record,
            locus: CallAdmissionLocus::Member { name, index: None },
        }) if name == OBJECT_ENVIRONMENT => Some(record),
        CallAdmissionFailure::Refused(_) | CallAdmissionFailure::Incomplete(_) => None,
    }
}

fn refused(reference: &ObjectReference, field: Option<&str>) -> CallFailure {
    let mut fields = BTreeMap::from([("object", reference.object().as_str().to_owned())]);
    if let Some(field) = field {
        fields.insert("field", field.to_owned());
    }
    CallFailure::Admission(Box::new(CallAdmissionFailure::Refused(
        CallAdmissionRecord {
            record: AdmissionRecord {
                catalog: CatalogCode::new("invalid_runtime_input", "wrong-value-kind"),
                fields,
                path: Vec::new(),
                kernel: None,
            },
            locus: CallAdmissionLocus::Member {
                name: OBJECT_ENVIRONMENT.to_owned(),
                index: None,
            },
        },
    )))
}

fn entry_fault(failure: EnvironmentFailure) -> CallFailure {
    CallFailure::Fault(InternalFault::new("S6a", fault_identifier(&failure)))
}

/// The FR-090 invariant a failed admission of a stored value reports: the
/// package's own declarations admit a value with no limit or refusal, so
/// only storage can fail.
pub(super) fn fault_identifier(failure: &EnvironmentFailure) -> &'static str {
    match failure {
        EnvironmentFailure::Allocation(_) => "object-field-admission-allocation-denied",
        EnvironmentFailure::Capacity(_) => "object-field-admission-capacity-overflow",
        EnvironmentFailure::Limit(_) | EnvironmentFailure::Refused(_) => {
            "object-field-admission-contract-broken"
        }
    }
}

/// Whether `slot` is admitted for the attribute `field` under `types`: a
/// required attribute holds a present value, and a present value is a
/// member of the declared type ([`admits`]). An absent or null slot carries
/// no integer, so a witness for it is forged.
pub(super) fn admits_slot(
    types: &TypeEnvironment,
    field: &FieldDeclaration,
    slot: &FieldValue,
    witnesses: &[&OutOfRange],
    widened: bool,
) -> Result<bool, EnvironmentFailure> {
    match slot {
        FieldValue::Present(value) => {
            let root = presence_steps(field.presence());
            admits(types, field.value_type(), value, root, witnesses, widened)
        }
        FieldValue::Absent | FieldValue::Null => {
            Ok(field.presence() == Presence::Optional && witnesses.is_empty())
        }
    }
}

/// The path step FR-106 enters for a present value of an attribute or
/// record member of `presence`: an optional one's present value is admitted
/// "one option-value step down" (`document.rs`'s `admit_value`), although the
/// slot holds the bare value.
fn presence_steps(presence: Presence) -> Vec<PathStep> {
    match presence {
        Presence::Optional => vec![PathStep::OptionValue],
        Presence::Required => Vec::new(),
    }
}

/// Whether `value` is a member of `declared` under `types`, with every record
/// member admitted against the package's own record declaration.
///
/// `witnesses` are the FR-106 check 6.5 out-of-range witnesses recorded for
/// the slot. Each names one integer by its path, in FR-106's step model: the
/// slot's `index` when the slot is a sequence and its element is the first
/// step, then `steps`, outermost first. The steps are `OptionValue` into an
/// `Option` value's payload and into the present value of an optional
/// attribute or record member, `Index` into a collection element and `Field`
/// into a record member; `root` is the slot's own prefix
/// ([`presence_steps`]). A witness relaxes the declared range at its
/// position only, for that observed integer. Every other integer is admitted at its declared
/// range, and every witness must name an integer the slot holds; a witness
/// that names none is forged. Type and unit are never relaxed. No witness
/// names a position inside a tuple or a union payload.
///
/// `widened`: the environment holds out-of-range witnesses, so FR-106 check
/// 6.5 built every option and collection it holds with `Int[lo, hi]` widened
/// to `Integer` in the type the value carries. Such a carried type is
/// admitted for its declared one; each integer is still checked at its
/// declared range.
fn admits(
    types: &TypeEnvironment,
    declared: &ValueType,
    value: &Value,
    root: Vec<PathStep>,
    witnesses: &[&OutOfRange],
    widened: bool,
) -> Result<bool, EnvironmentFailure> {
    let carries = |actual: &ValueType, declared: &ValueType| {
        types.same_type(actual, declared)
            || (widened && types.same_type(actual, &widen_integer_ranges(declared)))
    };
    let targets: Vec<Vec<PathStep>> = witnesses
        .iter()
        .map(|witness| {
            witness
                .index
                .map(PathStep::Index)
                .into_iter()
                .chain(witness.steps.iter().cloned())
                .collect()
        })
        .collect();
    let mut named = vec![false; witnesses.len()];
    // A position's path, or `None` where no witness can name a position (no
    // witness at all, or inside a tuple or a union payload).
    let root = (!witnesses.is_empty()).then_some(root);
    let mut pending: Vec<(&ValueType, &Value, Option<Vec<PathStep>>)> =
        vec![(declared, value, root)];
    let child = |path: &Option<Vec<PathStep>>, step: PathStep| {
        path.as_ref().map(|path| {
            let mut child = path.clone();
            child.push(step);
            child
        })
    };
    while let Some((declared, value, path)) = pending.pop() {
        match (declared, value) {
            (ValueType::Int(range), Value::Integer(held)) => {
                if range.contains(held) {
                    continue;
                }
                let Some(path) = &path else {
                    return Ok(false);
                };
                let mut witnessed = false;
                for ((witness, target), named) in witnesses.iter().zip(&targets).zip(&mut named) {
                    if target == path && witness.observed == *held {
                        *named = true;
                        witnessed = true;
                    }
                }
                if !witnessed {
                    return Ok(false);
                }
            }
            (ValueType::Option(payload), Value::Option(option)) => {
                if !carries(option.payload_type(), payload) {
                    return Ok(false);
                }
                if let Some(present) = option.payload() {
                    pending.push((payload, present, child(&path, PathStep::OptionValue)));
                }
            }
            (ValueType::Collection(collection), Value::Collection(held)) => {
                let actual = held.collection_type();
                if actual.kind() != collection.kind()
                    || actual.bound() != collection.bound()
                    || !carries(actual.element(), collection.element())
                {
                    return Ok(false);
                }
                for (index, element) in held.elements().iter().enumerate() {
                    pending.push((
                        collection.element(),
                        element,
                        child(&path, PathStep::Index(index)),
                    ));
                }
            }
            (ValueType::Composite(key), Value::Composite(record)) => {
                let own = record.declaration().clone();
                if !types.composite_conforms(own.clone(), key.clone()) {
                    return Ok(false);
                }
                let Some(declaration) = types.composite(own) else {
                    return Ok(false);
                };
                match declaration.shape() {
                    CompositeShape::Record(fields) => {
                        if fields.len() != record.slots().len() {
                            return Ok(false);
                        }
                        for (field, slot) in fields.iter().zip(record.slots()) {
                            match slot {
                                FieldValue::Present(member) => {
                                    let mut at =
                                        child(&path, PathStep::Field(field.name().to_owned()));
                                    for step in presence_steps(field.presence()) {
                                        at = child(&at, step);
                                    }
                                    pending.push((field.value_type(), member, at));
                                }
                                FieldValue::Absent | FieldValue::Null => {
                                    if field.presence() == Presence::Required {
                                        return Ok(false);
                                    }
                                }
                            }
                        }
                    }
                    CompositeShape::Tuple(positions) => {
                        if positions.len() != record.slots().len() {
                            return Ok(false);
                        }
                        for (position, slot) in positions.iter().zip(record.slots()) {
                            match slot {
                                FieldValue::Present(member) => {
                                    pending.push((position, member, None));
                                }
                                FieldValue::Absent | FieldValue::Null => return Ok(false),
                            }
                        }
                    }
                    CompositeShape::Union(_) => return Ok(false),
                }
            }
            (ValueType::Composite(_), Value::Union(union)) => {
                // Membership checks the member and each payload position's
                // declared type; a record in the payload is then admitted
                // member by member like any other.
                if !types.admits(declared, value)? {
                    return Ok(false);
                }
                let Some((_, member)) =
                    types.union_member(union.declaration().clone(), union.variant().clone())
                else {
                    return Ok(false);
                };
                for (position, payload) in member.positions().iter().zip(union.payload()) {
                    pending.push((position, payload, None));
                }
            }
            _ => {
                if !types.admits(declared, value)? {
                    return Ok(false);
                }
            }
        }
    }
    Ok(named.into_iter().all(|named| named))
}

/// `value_type` with every `Int[lo, hi]` it is, or holds as the payload of an
/// option or the element of a collection or map at any depth, widened to
/// `Integer`: the type FR-106 check 6.5's widened environment gives it.
fn widen_integer_ranges(value_type: &ValueType) -> ValueType {
    match value_type {
        ValueType::Int(_) => ValueType::Integer,
        ValueType::Option(payload) => ValueType::option(widen_integer_ranges(payload)),
        ValueType::Collection(collection) => {
            ValueType::collection(quire_exact::CollectionType::new(
                collection.kind(),
                widen_integer_ranges(collection.element()),
                collection.bound(),
            ))
        }
        ValueType::Map(map) => ValueType::Map(std::sync::Arc::new(quire_exact::MapType::new(
            widen_integer_ranges(map.values()),
            map.bound(),
        ))),
        other => other.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_ids;
    use ix_trace_rs::trace;
    use quire_exact::{Identifier, ObjectId, UnionMember, UnionValue};
    use quire_semantic_value::declaration::{
        CompositeDeclaration, ObjectTypeDeclaration, UnionMemberDeclaration,
    };
    use quire_semantic_value::object_closure::ObjectClosure;

    fn record_key() -> quire_exact::NodeKey {
        test_ids::node_key(b"P")
    }

    fn union_handle() -> quire_exact::NodeKey {
        test_ids::node_key(b"U")
    }

    fn union_settled() -> quire_exact::NodeKey {
        test_ids::node_key(b"U-settled")
    }

    fn object_type() -> quire_exact::EffectiveId {
        test_ids::effective_id(b"E")
    }

    fn reference() -> ObjectReference {
        ObjectReference::new(
            test_ids::universe_id(b"u"),
            object_type(),
            ObjectId::new("e").expect("a fixture key"),
        )
    }

    fn required(name: &str, value_type: ValueType) -> FieldDeclaration {
        FieldDeclaration::new(name, value_type, Presence::Required)
    }

    /// `P { b: member }`, the sealed union `U { M(P) }`, and the model object
    /// type `E { u: U, m: map }`.
    fn types(member: ValueType, map: ValueType) -> TypeEnvironment {
        let mut types = TypeEnvironment::new(
            [
                CompositeDeclaration::new(
                    record_key(),
                    "P",
                    CompositeShape::Record(vec![required("b", member)]),
                ),
                CompositeDeclaration::new(
                    union_handle(),
                    "U",
                    CompositeShape::Union(vec![UnionMemberDeclaration::resolved(
                        Identifier::new("M").expect("a fixture member name"),
                        vec![ValueType::Composite(record_key())],
                    )]),
                ),
            ],
            [ObjectTypeDeclaration::new(
                object_type(),
                "E",
                vec![
                    required("u", ValueType::Composite(union_handle())),
                    required("m", map),
                ],
            )],
        )
        .expect("the fixture declarations admit");
        types
            .seal_union_verified(union_handle(), union_settled(), vec![member_m()])
            .expect("the fixture union seals");
        types
    }

    fn member_m() -> UnionMember {
        UnionMember::from_admitted(union_settled(), "M").expect("a fixture member key")
    }

    fn map_type() -> ValueType {
        ValueType::Map(std::sync::Arc::new(quire_exact::MapType::new(
            ValueType::Integer,
            None,
        )))
    }

    /// An environment built under `caller` whose `E.u` holds `M(P { b })`
    /// and whose `E.m` holds `m`.
    fn environment(caller: &TypeEnvironment, b: Value, m: Value) -> ObjectEnvironment {
        let record = caller
            .record(record_key(), vec![("b", FieldValue::Present(b))])
            .expect("the caller's record admits");
        let union = UnionValue::from_admitted(member_m(), vec![record]);
        ObjectEnvironment::new(
            ObjectClosure::new(
                caller,
                [(
                    reference(),
                    vec![
                        ("u", FieldValue::Present(union)),
                        ("m", FieldValue::Present(m)),
                    ],
                )],
                &[],
            )
            .expect("the caller's closure admits"),
        )
    }

    fn refused_field(result: Result<(), CallFailure>) -> Option<String> {
        match result {
            Err(CallFailure::Admission(failure)) => match *failure {
                CallAdmissionFailure::Refused(record) => {
                    assert_eq!(
                        (record.record.catalog.code(), record.record.catalog.cause()),
                        ("invalid_runtime_input", "wrong-value-kind")
                    );
                    record.record.fields.get("field").cloned()
                }
                other => panic!("a refusal, not {other:?}"),
            },
            other => panic!("an entry refusal, not {other:?}"),
        }
    }

    /// QSL-1565 (FR-049-AC-2/AC-3): a record inside a union payload is
    /// admitted member by member against the package's record declaration,
    /// so a caller's `P { b: Integer }` inside `M(..)` refuses at entry where
    /// the package declares `P { b: Boolean }`.
    #[trace("TC-136", "FR-049-AC-2", "FR-049-AC-3")]
    #[test]
    fn a_record_inside_a_union_payload_is_admitted_against_the_package_record() {
        let package = types(ValueType::Boolean, ValueType::Integer);
        let declared = environment(&package, Value::Boolean(true), int(1));
        assert!(admit_objects(&declared, &package, &mut run(), 0).is_ok());
        let caller = types(ValueType::Integer, ValueType::Integer);
        let foreign = environment(&caller, int(7), int(1));
        assert_eq!(
            refused_field(admit_objects(&foreign, &package, &mut run(), 0)).as_deref(),
            Some("u")
        );
    }

    /// QSL-1565 (FR-049-AC-2/AC-3): the kernel has no map value, so a present
    /// value at a map-typed attribute -- here a caller's integer where the
    /// package declares `Map<Text, Integer>` -- is never admitted.
    #[trace("TC-136", "FR-049-AC-2", "FR-049-AC-3")]
    #[test]
    fn a_present_value_at_a_map_typed_attribute_refuses_at_entry() {
        let package = types(ValueType::Boolean, map_type());
        let caller = types(ValueType::Boolean, ValueType::Integer);
        let objects = environment(&caller, Value::Boolean(true), int(1));
        assert_eq!(
            refused_field(admit_objects(&objects, &package, &mut run(), 0)).as_deref(),
            Some("m")
        );
    }

    fn int(value: i64) -> Value {
        Value::Integer(quire_exact::Integer::from(value))
    }

    fn run() -> SuppliedRun {
        SuppliedRun::new(
            quire_semantic_value::supplied_stop::SuppliedLimits::default(),
            quire_exact::Cancel::new(),
        )
        .expect("a run has a path arena")
    }
}
