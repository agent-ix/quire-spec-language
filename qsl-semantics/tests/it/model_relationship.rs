// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-085: relationship-end resolution. A dangling end refuses the whole
//! relationship record `missing_declaration`/`missing-name`; a resolved end
//! retains its declared role and multiplicity; a navigation relationship
//! carries no systems-model kind.

use ix_trace_rs::trace;
use qsl_foundation::diagnostic::Code;
use qsl_semantics::model::accounting::{Meter, ModelNormalizationLimits};
use qsl_semantics::model::admitted::{AdmittedPackage, DeclarationKind};
use qsl_semantics::model::domain_package::{
    DomainPackage, DomainPackageRecord, DomainPackageRef, Multiplicity, ObjectTypeRecord,
    RelationshipDirection, RelationshipEnd, RelationshipRecord,
};
use qsl_semantics::model::key::DeclarationKey;
use qsl_semantics::model::normalize::{
    normalize, ModelRefusal, ModelRefusalCause, NormalizeOutcome,
};
use qsl_semantics::model::refusal::{MemberPath, MemberRoot};

fn object_type(identity: &str) -> DomainPackageRecord {
    DomainPackageRecord::ObjectType(ObjectTypeRecord {
        key: DeclarationKey::fixture(identity),
        interface_features: None,
        abstract_type: false,
        supertypes: Vec::new(),
    })
}

fn multiplicity(lower: u64, upper: Option<u64>) -> Multiplicity {
    Multiplicity {
        lower,
        upper,
        ordered: false,
        unique: true,
    }
}

fn end(type_identity: &str, role: Option<&str>, multiplicity: Multiplicity) -> RelationshipEnd {
    RelationshipEnd {
        type_identity: DeclarationKey::fixture(type_identity),
        role: role.map(str::to_owned),
        multiplicity,
    }
}

/// A business relationship `model.places` from `source` to `target`; the
/// source end declares a role, as the reader requires of every business
/// relationship.
fn places(source: &str, target: &str) -> DomainPackageRecord {
    DomainPackageRecord::Relationship(RelationshipRecord {
        key: DeclarationKey::fixture("ix://test/orders/places"),
        declared_at: None,
        source: end(source, Some("placed-by"), multiplicity(1, Some(1))),
        target: end(target, None, multiplicity(0, None)),
        direction: RelationshipDirection::SourceToTarget,
    })
}

fn package(records: Vec<DomainPackageRecord>) -> DomainPackage {
    DomainPackage::new(DomainPackageRef::fixture("bundle.rel"), records)
}

fn refusals_of(package: &DomainPackage) -> Vec<ModelRefusal> {
    match normalize(package, ModelNormalizationLimits::UNLIMITED) {
        NormalizeOutcome::Refused(refusals) => refusals.iter().cloned().collect(),
        other => panic!("expected a refusal, got {other:?}"),
    }
}

fn assert_missing_end(refusal: &ModelRefusal, missing: &str, end: &str) {
    assert_eq!(refusal.code, Code::MissingDeclaration, "{refusal:?}");
    assert_eq!(
        refusal.cause,
        ModelRefusalCause::UnknownRelationshipEnd {
            relationship: DeclarationKey::fixture("ix://test/orders/places"),
            missing: DeclarationKey::fixture(missing),
            member: MemberPath::new(MemberRoot::Relationship).name(end),
        }
    );
    assert_eq!(refusal.cause.as_str(), "missing-name");
}

/// FR-085-AC-1: a target end naming an undeclared type refuses the whole
/// record, naming that type and the `target` end only; the same shape with
/// both types declared is admitted.
#[trace("TC-230", "FR-085-AC-1")]
#[test]
fn a_dangling_target_end_refuses_missing_name_naming_the_target_end() {
    let refusals = refusals_of(&package(vec![
        object_type("ix://test/orders/Customer"),
        places("ix://test/orders/Customer", "ix://test/orders/Order"),
    ]));
    assert_eq!(refusals.len(), 1, "{refusals:?}");
    assert_missing_end(&refusals[0], "ix://test/orders/Order", "target");

    let resolved = package(vec![
        object_type("ix://test/orders/Customer"),
        object_type("ix://test/orders/Order"),
        places("ix://test/orders/Customer", "ix://test/orders/Order"),
    ]);
    assert!(matches!(
        normalize(&resolved, ModelNormalizationLimits::UNLIMITED),
        NormalizeOutcome::Completed(_)
    ));
}

/// FR-085-AC-1: the source end is held to the same rule, and only the
/// dangling end is named.
#[trace("TC-230", "FR-085-AC-1")]
#[test]
fn a_dangling_source_end_refuses_missing_name_naming_the_source_end() {
    let refusals = refusals_of(&package(vec![
        object_type("ix://test/orders/Order"),
        places("ix://test/orders/Customer", "ix://test/orders/Order"),
    ]));
    assert_eq!(refusals.len(), 1, "{refusals:?}");
    assert_missing_end(&refusals[0], "ix://test/orders/Customer", "source");
}

/// FR-085-AC-1: both ends dangling refuse once per end.
#[trace("TC-230", "FR-085-AC-1")]
#[test]
fn two_dangling_ends_refuse_once_each() {
    let refusals = refusals_of(&package(vec![places(
        "ix://test/orders/Customer",
        "ix://test/orders/Order",
    )]));
    assert_eq!(refusals.len(), 2, "{refusals:?}");
    assert_missing_end(&refusals[0], "ix://test/orders/Customer", "source");
    assert_missing_end(&refusals[1], "ix://test/orders/Order", "target");
}

/// FR-085-AC-2: a resolved source end keeps its role and bounded
/// multiplicity; a target end declaring neither keeps no role and its own
/// declared multiplicity, with nothing substituted.
#[trace("TC-231", "FR-085-AC-2")]
#[test]
fn a_resolved_end_retains_its_declared_role_and_multiplicity() {
    let admitted = AdmittedPackage::admit(
        package(vec![
            object_type("ix://test/orders/Customer"),
            object_type("ix://test/orders/Order"),
            places("ix://test/orders/Customer", "ix://test/orders/Order"),
        ]),
        &mut Meter::new(ModelNormalizationLimits::UNLIMITED),
    )
    .expect("a fully resolved relationship admits");
    let declaration = admitted
        .declaration_by_key(&DeclarationKey::fixture("ix://test/orders/places"))
        .expect("the relationship is admitted");
    let DomainPackageRecord::Relationship(relationship) = declaration.record else {
        panic!(
            "expected a relationship record, got {:?}",
            declaration.record
        );
    };
    assert_eq!(relationship.source.role.as_deref(), Some("placed-by"));
    assert_eq!(relationship.source.multiplicity, multiplicity(1, Some(1)));
    assert_eq!(relationship.target.role, None);
    assert_eq!(relationship.target.multiplicity, multiplicity(0, None));
    assert_eq!(
        relationship.direction,
        RelationshipDirection::SourceToTarget
    );
}

/// TC-232, object-type half of FR-085-AC-3: a relationship between two object types resolves as a plain
/// relationship; end resolution assigns no systems-model kind.
#[trace("TC-232")]
#[test]
fn a_relationship_between_object_types_carries_no_systems_kind() {
    let admitted = AdmittedPackage::admit(
        package(vec![
            object_type("ix://test/orders/Customer"),
            object_type("ix://test/orders/Order"),
            places("ix://test/orders/Customer", "ix://test/orders/Order"),
        ]),
        &mut Meter::new(ModelNormalizationLimits::UNLIMITED),
    )
    .expect("a fully resolved relationship admits");
    let declaration = admitted
        .declaration_by_key(&DeclarationKey::fixture("ix://test/orders/places"))
        .expect("the relationship is admitted");
    assert_eq!(declaration.kind, DeclarationKind::Relationship);
}
