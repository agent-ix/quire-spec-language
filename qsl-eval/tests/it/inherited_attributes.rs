// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSL-57 item 2: `deref(r).f` through an inherited or redefined field, run
//! through the real checker and evaluator (`CheckedPackage::check_expression`
//! and `CheckedPackage::evaluate`).
//!
//! `f` resolves at check time in `r`'s static type's flattened attribute
//! set; at evaluation the referenced object's own type, which conforms to
//! that static type, has exactly one slot standing for the resolved field.
//! A parameter admits only an object of its own declared type, so the
//! supertype view of a subtype object, `deref(lookup<M::A>(p, rb)).f`, is
//! in `model_reference_queries.rs`, where `lookup` produces it.

use ix_trace_rs::trace;
use qsl_eval::value::{CheckedPackageEvaluation, Evaluation};
use qsl_forms::Expression;
use qsl_package::CheckedPackage;
use qsl_semantics::check::{
    CheckCause, CheckMode, CheckRefusal, CheckedExpression, CheckingLimits, PackageDeclarations,
};
use qsl_semantics::family::FamilyOutcome;
use qsl_semantics::model::object_environment::ObjectEnvironment;
use qsl_semantics::value::declaration::{
    FieldDeclaration, FieldRef, ObjectTypeDeclaration, TypeEnvironment,
};
use quire_exact::{
    EffectiveId, FieldValue, IllTypedCause, Integer, Meter, ObjectId, ObjectReference, Outcome,
    Presence, ScalarLimits, UniverseId, Value, ValueType,
};
use sha2::{Digest, Sha256};

const UNLIMITED: ScalarLimits = ScalarLimits {
    integer_bits: u64::MAX,
    decimal_digits: u64::MAX,
    scale_expansion: u64::MAX,
    text_input_bytes: u64::MAX,
    text_scalars: u64::MAX,
    normalized_scalars: u64::MAX,
    unit_edges: u64::MAX,
    value_occurrences: u64::MAX,
    work_units: u64::MAX,
    result_units: u64::MAX,
};

fn effective(label: &str) -> EffectiveId {
    EffectiveId::from_digest(Sha256::digest(label.as_bytes()).into())
}

fn integer_field(name: &str) -> FieldDeclaration {
    FieldDeclaration::new(name, ValueType::Integer, Presence::Required)
}

fn redefining(name: &str, owner: &str, target: &str) -> FieldDeclaration {
    integer_field(name).with_redefines(FieldRef::new(effective(owner), target))
}

fn object(
    label: &str,
    fields: Vec<FieldDeclaration>,
    supertypes: &[&str],
) -> ObjectTypeDeclaration {
    ObjectTypeDeclaration::new(effective(label), label, fields).with_supertypes(
        supertypes
            .iter()
            .map(|general| effective(general))
            .collect(),
    )
}

fn package(object_types: Vec<ObjectTypeDeclaration>) -> (CheckedPackage, TypeEnvironment) {
    let types = TypeEnvironment::new([], object_types).unwrap();
    let graph = PackageDeclarations {
        types: types.clone(),
        ..PackageDeclarations::new(qsl_semantics::check::fixture_source())
    }
    .check(CheckingLimits::default())
    .unwrap();
    (CheckedPackage::link(graph), types)
}

fn reference(label: &str, object: &str) -> ObjectReference {
    ObjectReference::new(
        UniverseId::from_digest([0x42; 32]),
        effective(label),
        ObjectId::new(object).unwrap(),
    )
}

fn int(value: i64) -> FieldValue {
    FieldValue::Present(Value::Integer(Integer::from(value)))
}

/// `deref(r).field`.
fn attribute(field: &str) -> Expression {
    Expression::Field {
        operand: Box::new(Expression::Deref(Box::new(Expression::Name(
            "r".to_owned(),
        )))),
        field: field.to_owned(),
    }
}

fn check(
    package: &CheckedPackage,
    static_type: &str,
    field: &str,
) -> Result<CheckedExpression, CheckRefusal> {
    package.graph().check_expression(
        vec![("r".to_owned(), ValueType::Reference(effective(static_type)))],
        &attribute(field),
        None,
        CheckMode::Kernel,
        CheckingLimits::default(),
    )
}

/// `deref(r).field` for `r: Reference<static_type>` holding `object`.
fn read(
    package: &CheckedPackage,
    objects: &ObjectEnvironment,
    static_type: &str,
    field: &str,
    object: &ObjectReference,
) -> Integer {
    let checked = check(package, static_type, field).unwrap();
    assert_eq!(*checked.value_type(), ValueType::Integer);
    let evaluation: Evaluation = package
        .evaluate(
            &checked,
            vec![Value::Reference(object.clone())],
            objects,
            &mut Meter::new(UNLIMITED),
        )
        .unwrap();
    match evaluation.outcome {
        FamilyOutcome::Evaluated(Outcome::Completed(Value::Integer(value))) => value,
        other => panic!("expected a completed integer, got {other:?}"),
    }
}

fn refused_at_check(package: &CheckedPackage, static_type: &str, field: &str) -> CheckCause {
    match check(package, static_type, field) {
        Err(refusal) => refusal.cause,
        Ok(checked) => panic!("expected a check refusal, got {:?}", checked.value_type()),
    }
}

/// `A.x` is inherited by `B`: `deref(r).x` resolves for `r: Reference<B>`
/// and reads the `B` object's inherited slot, beside its own `y`. `A`'s
/// view does not see `B`'s own `y`.
#[trace("TC-198", "FR-153-AC-6")]
#[test]
fn deref_reads_an_inherited_field_through_the_subtype() {
    let (package, types) = package(vec![
        object("M::A", vec![integer_field("x")], &[]),
        object("M::B", vec![integer_field("y")], &["M::A"]),
    ]);
    let b1 = reference("M::B", "b1");
    let objects =
        ObjectEnvironment::new(&types, [(b1.clone(), vec![("x", int(7)), ("y", int(8))])]).unwrap();
    assert_eq!(
        read(&package, &objects, "M::B", "x", &b1),
        Integer::from(7_i64)
    );
    assert_eq!(
        read(&package, &objects, "M::B", "y", &b1),
        Integer::from(8_i64)
    );
    assert_eq!(
        refused_at_check(&package, "M::A", "y"),
        CheckCause::IllTyped(IllTypedCause::TypeMismatch)
    );
}

/// FR-151 hide: `B.x` redefines `A.x`. An `A` object's `x` is `A.x`; a `B`
/// object's `x` is `B.x`'s one slot.
#[trace("TC-198", "FR-153-AC-6")]
#[trace("TC-196", "FR-151-AC-1")]
#[test]
fn deref_through_a_redefined_field_reads_the_redefiner() {
    let (package, types) = package(vec![
        object("M::A", vec![integer_field("x")], &[]),
        object("M::B", vec![redefining("x", "M::A", "x")], &["M::A"]),
    ]);
    let (a1, b1) = (reference("M::A", "a1"), reference("M::B", "b1"));
    let objects = ObjectEnvironment::new(
        &types,
        [
            (a1.clone(), vec![("x", int(1))]),
            (b1.clone(), vec![("x", int(2))]),
        ],
    )
    .unwrap();
    assert_eq!(
        read(&package, &objects, "M::A", "x", &a1),
        Integer::from(1_i64)
    );
    assert_eq!(
        read(&package, &objects, "M::B", "x", &b1),
        Integer::from(2_i64)
    );
}

/// FR-151 most derived wins: `B.x` and `C.x` both redefine `A.x`, `C -> B`.
/// `C`'s one `x` is `C.x`, read through `Reference<C>`.
#[trace("TC-198", "FR-153-AC-6")]
#[trace("TC-196", "FR-151-AC-1")]
#[test]
fn deref_reads_the_most_derived_redefinition() {
    let (package, types) = package(vec![
        object("M::A", vec![integer_field("x")], &[]),
        object("M::B", vec![redefining("x", "M::A", "x")], &["M::A"]),
        object("M::C", vec![redefining("x", "M::A", "x")], &["M::B"]),
    ]);
    let c1 = reference("M::C", "c1");
    let objects = ObjectEnvironment::new(&types, [(c1.clone(), vec![("x", int(9))])]).unwrap();
    assert_eq!(
        read(&package, &objects, "M::C", "x", &c1),
        Integer::from(9_i64)
    );
}

/// A renamed redefinition: `B.y` redefines `A.x`. Through `Reference<B>`
/// the name is `y`, and `x` no longer resolves: refused at check time.
#[trace("TC-198", "FR-153-AC-6")]
#[trace("TC-196", "FR-151-AC-1")]
#[test]
fn deref_through_a_renamed_redefinition_reads_the_new_name() {
    let (package, types) = package(vec![
        object("M::A", vec![integer_field("x")], &[]),
        object("M::B", vec![redefining("y", "M::A", "x")], &["M::A"]),
    ]);
    let b1 = reference("M::B", "b1");
    let objects = ObjectEnvironment::new(&types, [(b1.clone(), vec![("y", int(4))])]).unwrap();
    assert_eq!(
        read(&package, &objects, "M::B", "y", &b1),
        Integer::from(4_i64)
    );
    assert_eq!(
        refused_at_check(&package, "M::B", "x"),
        CheckCause::IllTyped(IllTypedCause::TypeMismatch)
    );
}
