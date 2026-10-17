// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-321: a supplied union value is admitted only when it is a well-formed
//! value of a union of the evaluating package, and is otherwise refused before
//! evaluation.
//!
//! No checked package can hold a union until the FR-319 lowering exists, so
//! these tests drive the argument admission (`validate`) over the sealed
//! fixture unions (`qsl_semantics::check::union_fixture`). Every assertion
//! reads a typed refusal or stop, never message text.

use std::cmp::Ordering;

use super::*;
use ix_trace_rs::trace;
use qsl_semantics::check::mint_union_member_key;
use qsl_semantics::check::union_fixture as fixture;
use quire_exact::{
    compare_keys, Identifier, Integer, ObjectId, UnionMember, UnionValue, UniverseId,
};

/// The model object type `T` that `Holder` references; the kernel
/// constructor is called here, in a test, not in the fixture.
fn object_type() -> quire_exact::EffectiveId {
    quire_exact::EffectiveId::from_digest([9; 32])
}

fn int(number: i64) -> Value {
    Value::Integer(Integer::from(number))
}

/// `validate` of one argument `argument` for a parameter of `parameter`,
/// under the default supplied limits.
fn admit(
    parameter: ValueType,
    argument: &Value,
    objects: &ObjectEnvironment,
) -> Result<(), CallFailure> {
    let mut run = SuppliedRun::new(SuppliedLimits::default(), quire_exact::Cancel::new())
        .expect("a run has a path arena");
    validate(
        &[("s".to_owned(), parameter)],
        std::slice::from_ref(argument),
        objects,
        &fixture::types(object_type()),
        &mut run,
    )
}

fn shape() -> ValueType {
    fixture::union_type("Shape")
}

fn refused_as_wrong_kind(result: Result<(), CallFailure>, case: &str) {
    match result {
        Err(CallFailure::Input(InputRefusal::WrongValueKind { parameter: 0 })) => {}
        other => panic!("{case}: wrong-value-kind at parameter 0, not {other:?}"),
    }
}

/// A member of `union`, keyed under `union`'s settled key, that its
/// declaration does not list.
fn undeclared_member(union: &str, name: &str, payload: Vec<Value>) -> Value {
    UnionValue::from_admitted(
        UnionMember::from_admitted(
            fixture::settled(union),
            mint_union_member_key(fixture::settled(union), name),
            Identifier::new(name).expect("an identifier"),
        ),
        payload,
    )
}

/// FR-321-AC-1: each malformed `Shape` argument is refused
/// `invalid_runtime_input`/`wrong-value-kind`, and `Rect(2, 3)` is admitted.
#[trace("FR-321-AC-1", "TC-829")]
#[test]
fn a_supplied_union_value_is_admitted_only_as_a_member_of_its_union() {
    let objects = ObjectEnvironment::default();
    let admitted = fixture::value("Shape", "Rect", vec![int(2), int(3)]);
    assert!(admit(shape(), &admitted, &objects).is_ok());

    let refused = [
        (
            "a union key of another package",
            undeclared_member("Foreign", "Rect", vec![int(2), int(3)]),
        ),
        (
            "a member of another union",
            fixture::value("Other", "Only", vec![int(1)]),
        ),
        (
            "a member Shape does not declare",
            undeclared_member("Shape", "Square", vec![int(2)]),
        ),
        (
            "Rect with one payload value",
            fixture::value("Shape", "Rect", vec![int(2)]),
        ),
        (
            "Circle with payload true",
            fixture::value("Shape", "Circle", vec![Value::Boolean(true)]),
        ),
    ];
    for (case, argument) in &refused {
        let result = admit(shape(), argument, &objects);
        if let Err(CallFailure::Input(refusal)) = &result {
            assert_eq!(
                input_refusal_code(refusal),
                qsl_foundation::diagnostic::Code::InvalidRuntimeInput,
                "{case}"
            );
        }
        refused_as_wrong_kind(result, case);
    }
}

/// FR-321-AC-2: a union payload is walked for references as a record's slots
/// are, so a dangling reference inside it is refused by the existing walk.
#[trace("FR-321-AC-2", "TC-829")]
#[test]
fn a_dangling_reference_in_a_union_payload_is_refused_by_the_reference_walk() {
    let objects = ObjectEnvironment::default();
    let holder = fixture::union_type("Holder");
    let reference = Value::Reference(quire_exact::ObjectReference::new(
        UniverseId::from_digest([1; 32]),
        object_type(),
        ObjectId::new("i1").expect("an object id"),
    ));
    let dangling = fixture::value("Holder", "Some", vec![reference]);
    match admit(holder.clone(), &dangling, &objects) {
        Err(CallFailure::Input(InputRefusal::DanglingReference { parameter: 0 })) => {}
        other => panic!("a dangling reference at parameter 0, not {other:?}"),
    }
    let nothing = fixture::value("Holder", "Nothing", vec![]);
    assert!(admit(holder, &nothing, &objects).is_ok());
}

/// FR-321-AC-4 (admission half): a `Tree` 10,000 levels deep is admitted under
/// the default supplied limits and equals itself, with no host stack growth.
/// The value-occurrence stop is the replay conversion's
/// (`qsl-replay`'s `union` tests); a directly supplied value is bounded by the
/// meter at its first charge.
#[trace("FR-321-AC-4", "TC-830")]
#[test]
fn a_ten_thousand_level_supplied_tree_is_admitted() {
    let objects = ObjectEnvironment::default();
    let tree = fixture::tree(10_000);
    assert_eq!(tree.occ().to_u64(), Some(29_999));
    assert!(admit(fixture::union_type("Tree"), &tree, &objects).is_ok());
    assert_eq!(compare_keys(&tree, &tree.clone()), Some(Ordering::Equal));
}
