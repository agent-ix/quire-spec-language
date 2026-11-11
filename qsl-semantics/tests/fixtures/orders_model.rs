// SPDX-License-Identifier: AGPL-3.0-or-later
// TC-1408 fixture D: the model twin of fixture S, the domain package
// `test/orders`. Included by the tests that need it.

use qsl_semantics::model::domain_package::{
    DomainPackage, DomainPackageRecord, DomainPackageRef, Extent, FieldMemberRecord, Multiplicity,
    NativeValueType, ObjectTypeRecord, OperationEffect, OperationMemberRecord,
    OperationParameterRecord, OperationResult, PopulationRecord, ScalarTypeRecord,
    ScalarValueDomain, ValueTypeRef,
};
use quire_checked::key::DeclarationKey;
use quire_exact::{Presence, TextProfile, TextType};

// ---------------------------------------------------------------------------
// Fixture D: the model twin of fixture S, a domain package `test/orders`.
// ---------------------------------------------------------------------------

pub fn key(node: &str) -> DeclarationKey {
    DeclarationKey::fixture(format!("ix://test/orders/{node}"))
}

pub fn one() -> Multiplicity {
    Multiplicity {
        lower: 1,
        upper: Some(1),
        ordered: false,
        unique: true,
    }
}

pub fn object(name: &str, supertypes: &[&str], abstract_type: bool) -> DomainPackageRecord {
    DomainPackageRecord::ObjectType(ObjectTypeRecord {
        key: key(name),
        interface_features: None,
        abstract_type,
        supertypes: supertypes.iter().map(|name| key(name)).collect(),
    })
}

pub fn field(
    owner: &str,
    name: &str,
    value_type: ValueTypeRef,
    presence: Presence,
    position: usize,
) -> DomainPackageRecord {
    DomainPackageRecord::FieldMember(FieldMemberRecord {
        key: key(&format!("{owner}/{name}")),
        owner: key(owner),
        value_type,
        multiplicity: one(),
        presence,
        subsets: Vec::new(),
        redefines: None,
        position,
    })
}

/// `owner::name(parameter: Integer): Integer` or `(): Integer`, a query.
pub fn query(
    owner: &str,
    name: &str,
    parameter: Option<&str>,
    redefines: Option<&str>,
) -> DomainPackageRecord {
    operation(
        owner,
        name,
        parameter,
        Some(ValueTypeRef::Native(NativeValueType::Integer)),
        redefines,
        OperationEffect::default(),
    )
}

pub fn operation(
    owner: &str,
    name: &str,
    parameter: Option<&str>,
    result: Option<ValueTypeRef>,
    redefines: Option<&str>,
    effect: OperationEffect,
) -> DomainPackageRecord {
    DomainPackageRecord::OperationMember(OperationMemberRecord {
        key: key(&format!("{owner}/{name}")),
        owner: key(owner),
        parameters: parameter
            .map(|parameter| OperationParameterRecord {
                key: key(&format!("{owner}/{name}/{parameter}")),
                value_type: ValueTypeRef::Native(NativeValueType::Integer),
                multiplicity: one(),
            })
            .into_iter()
            .collect(),
        result: result.map(|value_type| OperationResult {
            value_type,
            multiplicity: one(),
        }),
        effect,
        requires: Vec::new(),
        ensures: Vec::new(),
        has_body: false,
        redefines: redefines.map(key),
    })
}

pub fn bump(owner: &str, redefines: Option<&str>, fields: &[&str]) -> DomainPackageRecord {
    operation(
        owner,
        "bump",
        None,
        Some(ValueTypeRef::Native(NativeValueType::Boolean)),
        redefines,
        OperationEffect {
            receiver_modifies: fields
                .iter()
                .map(|name| key(&format!("Order/{name}")))
                .collect(),
            ..OperationEffect::default()
        },
    )
}

/// Fixture D's records; `redefine_bump` adds D-R' (`Sub` redefines `bump`,
/// also modifying `note`).
pub fn model_records(redefine_bump: bool) -> Vec<DomainPackageRecord> {
    let mut records = vec![
        object("Order", &[], false),
        object("Sub", &["Order"], false),
        object("Shape", &[], true),
        DomainPackageRecord::ScalarType(ScalarTypeRecord {
            key: key("Total"),
            domain: ScalarValueDomain::Integer { lower: 0, upper: 9 },
        }),
        DomainPackageRecord::ScalarType(ScalarTypeRecord {
            key: key("Note"),
            domain: ScalarValueDomain::Text(TextType::new(0, 8, TextProfile::Nfc).unwrap()),
        }),
        field(
            "Order",
            "total",
            ValueTypeRef::Package(key("Total")),
            Presence::Required,
            0,
        ),
        field(
            "Order",
            "note",
            ValueTypeRef::Package(key("Note")),
            Presence::Optional,
            1,
        ),
        DomainPackageRecord::Population(PopulationRecord {
            key: key("orders"),
            member_types: vec![key("Order")],
            extent: Extent::Closed,
            symmetric: false,
        }),
        DomainPackageRecord::Population(PopulationRecord {
            key: key("shapes"),
            member_types: vec![key("Shape")],
            extent: Extent::Closed,
            symmetric: false,
        }),
        query("Order", "size", None, None),
        query("Order", "count", None, None),
        query("Order", "scaled", Some("n"), None),
        bump("Order", None, &["total"]),
        query("Sub", "size", None, Some("Order/size")),
        query("Sub", "scaled", Some("m"), Some("Order/scaled")),
    ];
    if redefine_bump {
        records.push(bump("Sub", Some("Order/bump"), &["total", "note"]));
    }
    records
}

pub fn model_package(redefine_bump: bool) -> DomainPackage {
    DomainPackage::new(
        DomainPackageRef::fixture("test/orders"),
        model_records(redefine_bump),
    )
}

