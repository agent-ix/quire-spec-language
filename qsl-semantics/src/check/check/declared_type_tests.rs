// SPDX-License-Identifier: AGPL-3.0-or-later
//! Private-consumer controls use the owner's thread-local allocator fixture.
use super::*;
use ix_trace_rs::trace;
use quire_semantic_value::declaration::{StorageRequest, StorageUnit, TypeCheckError};

fn scope() -> Scope {
    Scope::new(
        TypeEnvironment::new([], []).unwrap(),
        OperationTable::default(),
        vec![],
        vec![],
        vec![],
        None,
        vec![],
    )
}

fn key(name: &str) -> NodeKey {
    NodeKey::new(
        "declared-type-test".to_owned(),
        vec![quire_exact::PathSegment::Text(name.to_owned())],
        0,
    )
    .unwrap()
}

fn object(name: &str) -> EffectiveId {
    EffectiveId::new(
        None,
        quire_exact::OriginalDeclarationKey::new("declared-type-test", name),
    )
}

#[trace("TC-1339", "FR-534-AC-2", "FR-534-AC-4")]
#[test]
fn owner_traversal_denial_retains_request_and_allows_identical_retry() {
    let scope = scope();
    let before = scope.types.clone();
    let signatures = Signatures::default();
    let mut nodes = 0;
    let typer = Typer::new(
        &scope,
        &signatures,
        CheckingLimits::default(),
        &mut nodes,
        ClauseKind::Body,
    );
    let ty = ValueType::option(ValueType::Integer);
    let original = ty.clone();
    let location = Location::at(Origin::Expression, &[2, 4]);
    let fresh = typer.check_declared_type(&ty, &location);
    assert_eq!(fresh, Ok(()));
    qsv_failing_allocator::arm(0);
    let owner_failure = scope.types.try_check_type(&ty);
    let owner_denied = qsv_failing_allocator::disarm();
    let Err(TypeCheckError::Allocation(request)) = owner_failure else {
        panic!("the owner must expose the denied traversal request");
    };
    assert_eq!(request, StorageRequest::new(1, StorageUnit::AdditionalElements));
    qsv_failing_allocator::arm(0);
    let failed = typer.check_declared_type(&ty, &location);
    let denied = qsv_failing_allocator::disarm();
    assert_eq!(denied, Some(core::mem::size_of::<&ValueType>()));
    assert_eq!(denied, owner_denied);
    let failure = failed.expect_err("the real owner root-worklist reservation must fail");
    assert_eq!(failure.location, location);
    assert_eq!(
        failure.cause,
        CheckCause::TypeCheckAllocation(request)
    );
    assert_eq!(
        failure.cause.code(),
        qsl_foundation::diagnostic::Code::ResourceExhausted
    );
    assert_eq!(failure.cause.cause(), Some("allocation-failed"));
    assert_eq!(
        failure.cause.type_check_stage(),
        Some(CheckingStage::Typing)
    );
    assert_eq!(
        failure.cause.type_check_fields(),
        Some(BTreeMap::from([(
            "requested",
            "additional-elements:1".to_owned()
        )]))
    );
    assert_eq!(scope.types, before);
    assert_eq!(ty, original);
    assert_eq!(typer.check_declared_type(&ty, &location), fresh);
}

#[trace("TC-1339", "FR-534-AC-3")]
#[test]
fn owner_storage_classes_and_units_survive_conversion() {
    let location = Location::at(Origin::Expression, &[3, 1]);
    for (amount, unit, name) in [
        (17, StorageUnit::Bytes, "bytes"),
        (29, StorageUnit::AdditionalElements, "additional-elements"),
    ] {
        let request = StorageRequest::new(amount, unit);
        for (error, expected, field, cause) in [
            (
                TypeCheckError::Allocation(request),
                CheckCause::TypeCheckAllocation(request),
                "requested",
                "allocation-failed",
            ),
            (
                TypeCheckError::Capacity(request),
                CheckCause::TypeCheckCapacity(request),
                "upstream",
                "reservation-size-overflow",
            ),
        ] {
            let refusal = CheckRefusal::from_type_check(&location, error);
            assert_eq!(refusal.location, location);
            assert_eq!(refusal.cause, expected);
            assert_eq!(
                refusal.cause.type_check_stage(),
                Some(CheckingStage::Typing)
            );
            assert_eq!(
                refusal.cause.code(),
                qsl_foundation::diagnostic::Code::ResourceExhausted
            );
            assert_eq!(refusal.cause.cause(), Some(cause));
            assert_eq!(
                refusal.cause.type_check_fields(),
                Some(BTreeMap::from([(field, format!("{name}:{amount}"))]))
            );
        }
    }
}

#[trace("TC-1339", "FR-534-AC-1")]
#[test]
fn declared_type_semantics_keep_owner_refusal_priority_and_location() {
    use quire_semantic_value::declaration::{CompositeDeclaration, ObjectTypeDeclaration};
    let mut scope = scope();
    let record = key("Record");
    let reference = object("Object");
    let ieee = ValueType::Float(quire_exact::FloatType::exact(
        quire_exact::IeeeWidth::Binary64,
    ));
    scope.types = TypeEnvironment::new(
        [CompositeDeclaration::new(
            record.clone(),
            "Record",
            CompositeShape::Tuple(vec![ValueType::Boolean]),
        )],
        [ObjectTypeDeclaration::new(
            reference.clone(),
            "Object",
            vec![],
        )],
    )
    .unwrap();
    let signatures = Signatures::default();
    let mut nodes = 0;
    let typer = Typer::new(
        &scope,
        &signatures,
        CheckingLimits::default(),
        &mut nodes,
        ClauseKind::Body,
    );
    let location = Location::at(Origin::Expression, &[5, 2]);
    for ty in [
        ValueType::Boolean,
        ValueType::Integer,
        ValueType::Composite(record),
        ValueType::Reference(reference),
        ValueType::option(ValueType::option(ValueType::Integer)),
        ValueType::collection(CollectionType::new(
            CollectionKind::Sequence,
            ValueType::collection(CollectionType::new(
                CollectionKind::Set,
                ValueType::Integer,
                None,
            )),
            None,
        )),
    ] {
        assert_eq!(typer.check_declared_type(&ty, &location), Ok(()));
    }
    let ty = ValueType::collection(CollectionType::new(CollectionKind::Set, ieee, None));
    let expected = CheckRefusal::from_ill_typed(
        &location,
        quire_exact::IllTyped {
            cause: IllTypedCause::OperatorIneligible,
        },
    );
    assert_eq!(
        typer.check_declared_type(&ty, &location),
        Err(expected.clone())
    );
    // Both unordered collection layers prohibit the nested IEEE element;
    // the outer layer retains the owner's first-refusal priority.
    let competing = ValueType::collection(CollectionType::new(CollectionKind::Bag, ty, None));
    assert_eq!(
        typer.check_declared_type(&competing, &location),
        Err(expected)
    );
    for ty in [
        ValueType::Composite(key("Missing")),
        ValueType::Reference(object("Missing")),
    ] {
        assert_eq!(
            typer.check_declared_type(&ty, &location),
            Err(CheckRefusal::from_ill_typed(
                &location,
                quire_exact::IllTyped {
                    cause: IllTypedCause::TypeMismatch
                }
            ))
        );
    }
}
