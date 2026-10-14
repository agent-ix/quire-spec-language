// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-197: systems-model typed binding (FR-152), the kind-mapping,
//! connection and allocation slice this rung builds. Y01-Y06 only —
//! Y07-Y09 exercise `model.navigate`/`self.<field>` traversal over a
//! runtime population, which `src/model/systems.rs`'s module doc records
//! as out of scope (no qualified-name binder, no object population).
//!
//! Fixture Y is TC-197's own fixture, transcribed field-for-field; every
//! assertion below is checked against TC-197's table, not against this
//! module's own computed values.

use ix_trace_rs::trace;
use qsl_foundation::diagnostic::Code;
use qsl_semantics::model::accounting::{ChargePoint, Meter, ModelNormalizationLimits};
use qsl_semantics::model::domain_package::{
    AllocationRecord, ComponentRecord, DomainPackage, DomainPackageRecord, DomainPackageRef,
    EndpointRecord, Multiplicity, ObjectTypeRecord, OperationEffect, OperationMemberRecord,
    PortDirection, RelationshipDirection, RelationshipEnd, RelationshipRecord, ScalarTypeRecord,
    ScalarValueDomain,
};
use qsl_semantics::model::index::ModelIndex;
use qsl_semantics::model::key::DeclarationKey;
use qsl_semantics::model::key::{RULE_INHERIT, RULE_QUALIFY};
use qsl_semantics::model::normalize::{
    normalize, ModelRefusal, ModelRefusalCause, NormalizeOutcome,
};
use qsl_semantics::model::systems::{
    check_allocation, check_connection, classify, resolve_kind, AllocationCheckOutcome,
    ConditionFailure, ConnectionCheckOutcome, ConnectionOutcome, Kind,
};

fn mult(lower: u64, upper: Option<u64>) -> Multiplicity {
    Multiplicity {
        lower,
        upper,
        ordered: false,
        unique: true,
    }
}

fn one() -> Multiplicity {
    mult(1, Some(1))
}

fn object_type(
    identity: &str,
    interface_features: Option<Vec<&str>>,
    supertypes: Vec<&str>,
) -> DomainPackageRecord {
    DomainPackageRecord::ObjectType(ObjectTypeRecord {
        key: DeclarationKey::fixture(identity),
        interface_features: interface_features
            .map(|features| features.into_iter().map(DeclarationKey::fixture).collect()),
        abstract_type: false,
        supertypes: supertypes
            .into_iter()
            .map(DeclarationKey::fixture)
            .collect(),
    })
}

fn scalar_type(identity: &str, lower: i128, upper: i128) -> DomainPackageRecord {
    DomainPackageRecord::ScalarType(ScalarTypeRecord {
        key: DeclarationKey::fixture(identity),
        domain: ScalarValueDomain::Integer { lower, upper },
    })
}

fn field_member(
    identity: &str,
    owner: &str,
    value_type: &str,
    m: Multiplicity,
) -> DomainPackageRecord {
    DomainPackageRecord::FieldMember(qsl_semantics::model::domain_package::FieldMemberRecord {
        key: DeclarationKey::fixture(identity),
        owner: DeclarationKey::fixture(owner),
        value_type: qsl_semantics::model::domain_package::ValueTypeRef::Package(
            DeclarationKey::fixture(value_type),
        ),
        multiplicity: m,
        presence: quire_exact::Presence::Required,
        subsets: vec![],
        redefines: None,
    })
}

fn component(
    identity: &str,
    owning_type: &str,
    value_type: &str,
    m: Multiplicity,
) -> DomainPackageRecord {
    DomainPackageRecord::Component(ComponentRecord {
        key: DeclarationKey::fixture(identity),
        owning_type: DeclarationKey::fixture(owning_type),
        value_type: DeclarationKey::fixture(value_type),
        multiplicity: m,
        has_part_signature: true,
    })
}

fn endpoint(
    identity: &str,
    owning_component: &str,
    value_type: &str,
    direction: Option<PortDirection>,
    m: Multiplicity,
) -> DomainPackageRecord {
    DomainPackageRecord::Endpoint(EndpointRecord {
        key: DeclarationKey::fixture(identity),
        owning_component: DeclarationKey::fixture(owning_component),
        value_type: DeclarationKey::fixture(value_type),
        direction,
        multiplicity: m,
    })
}

fn relationship(
    identity: &str,
    source: &str,
    source_m: Multiplicity,
    target: &str,
    target_m: Multiplicity,
    direction: RelationshipDirection,
) -> DomainPackageRecord {
    DomainPackageRecord::Relationship(RelationshipRecord {
        key: DeclarationKey::fixture(identity),
        declared_at: None,
        source: RelationshipEnd {
            type_identity: DeclarationKey::fixture(source),
            role: None,
            multiplicity: source_m,
        },
        target: RelationshipEnd {
            type_identity: DeclarationKey::fixture(target),
            role: None,
            multiplicity: target_m,
        },
        direction,
    })
}

fn allocation(identity: &str, source_element: &str, target_element: &str) -> DomainPackageRecord {
    DomainPackageRecord::Allocation(AllocationRecord {
        key: DeclarationKey::fixture(identity),
        source_element: DeclarationKey::fixture(source_element),
        target_element: DeclarationKey::fixture(target_element),
    })
}

fn operation_run() -> DomainPackageRecord {
    DomainPackageRecord::OperationMember(OperationMemberRecord {
        key: DeclarationKey::fixture("model.Pump.run"),
        owner: DeclarationKey::fixture("model.Pump"),
        parameters: vec![],
        result: None,
        effect: OperationEffect::default(),
        requires: Vec::new(),
        ensures: Vec::new(),
        has_body: true,
        redefines: None,
    })
}

/// TC-197 fixture Y (domain package `bundle.y`), field-for-field. `mutate` edits the
/// base record set before the domain package is built, so every Y02-Y06 variant is
/// this same fixture with exactly the one documented change.
fn fixture_y(mutate: impl FnOnce(&mut Vec<DomainPackageRecord>)) -> DomainPackage {
    let mut records = vec![
        scalar_type("model.Count", 0, 9),
        object_type("model.Sys", None, vec![]),
        object_type("model.SysBridge", None, vec!["model.Sys"]),
        object_type("model.SysSub", None, vec!["model.Sys", "model.SysBridge"]),
        object_type("model.Pump", None, vec![]),
        object_type("model.Tank", None, vec![]),
        object_type("model.Flow", Some(vec!["model.Flow.rate"]), vec![]),
        object_type(
            "model.Flow2",
            Some(vec!["model.Flow.rate"]),
            vec!["model.Flow"],
        ),
        field_member("model.Flow.rate", "model.Flow", "model.Count", one()),
        component("model.Sys.pump", "model.Sys", "model.Pump", one()),
        component("model.Sys.tank", "model.Sys", "model.Tank", one()),
        endpoint(
            "model.Sys.pump.out",
            "model.Sys.pump",
            "model.Flow",
            Some(PortDirection::Out),
            one(),
        ),
        endpoint(
            "model.Sys.tank.in",
            "model.Sys.tank",
            "model.Flow",
            Some(PortDirection::In),
            one(),
        ),
        relationship(
            "model.Sys.pipe",
            "model.Sys.pump.out",
            one(),
            "model.Sys.tank.in",
            one(),
            RelationshipDirection::SourceToTarget,
        ),
        operation_run(),
        allocation("model.Pump.alloc", "model.Pump.run", "model.Sys.pump"),
        relationship(
            "model.rel.parts",
            "model.Sys",
            one(),
            "model.Pump",
            mult(0, Some(3)),
            RelationshipDirection::SourceToTarget,
        ),
        relationship(
            "model.rel.home",
            "model.Pump",
            one(),
            "model.Tank",
            one(),
            RelationshipDirection::SourceToTarget,
        ),
    ];
    mutate(&mut records);
    DomainPackage::new(DomainPackageRef::fixture("bundle.y"), records)
}

fn unlimited_meter() -> Meter {
    Meter::new(ModelNormalizationLimits::UNLIMITED)
}

fn find_relationship<'a>(
    records: &'a mut [DomainPackageRecord],
    identity: &str,
) -> &'a mut RelationshipRecord {
    records
        .iter_mut()
        .find_map(|record| match record {
            DomainPackageRecord::Relationship(relationship)
                if relationship.key.node == identity =>
            {
                Some(relationship)
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("fixture Y has no relationship {identity}"))
}

fn find_allocation<'a>(
    records: &'a mut [DomainPackageRecord],
    identity: &str,
) -> &'a mut AllocationRecord {
    records
        .iter_mut()
        .find_map(|record| match record {
            DomainPackageRecord::Allocation(allocation) if allocation.key.node == identity => {
                Some(allocation)
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("fixture Y has no allocation {identity}"))
}

fn find_endpoint<'a>(
    records: &'a mut [DomainPackageRecord],
    identity: &str,
) -> &'a mut EndpointRecord {
    records
        .iter_mut()
        .find_map(|record| match record {
            DomainPackageRecord::Endpoint(endpoint) if endpoint.key.node == identity => {
                Some(endpoint)
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("fixture Y has no endpoint {identity}"))
}

fn find_component<'a>(
    records: &'a mut [DomainPackageRecord],
    identity: &str,
) -> &'a mut ComponentRecord {
    records
        .iter_mut()
        .find_map(|record| match record {
            DomainPackageRecord::Component(component) if component.key.node == identity => {
                Some(component)
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("fixture Y has no component {identity}"))
}

// FR-152-AC-1 dropped (retagged, PR #144 review finding #1): AC-1 requires
// resolving "exact producer AND effective-declaration identity," and this
// module computes no effective-declaration identity at all (see the module
// doc's own scope note) — nothing here can honestly claim AC-1.
/// FR-208-AC-9: a relationship end naming a record value type refuses
/// `invalid_model_binding`/`malformed-declaration`, not as a dangling end:
/// the end names a declared type of the wrong meaning.
#[trace("QSpec-TC-197", "TC-997", "FR-056-AC-32")]
#[test]
fn a_relationship_end_naming_a_record_value_type_refuses_malformed() {
    let domain_package = fixture_y(|records| {
        records.push(DomainPackageRecord::RecordValueType(
            qsl_semantics::model::domain_package::RecordValueTypeRecord {
                key: DeclarationKey::fixture("model.Money"),
                identity_fields: Vec::new(),
                supertypes: Vec::new(),
            },
        ));
        records.push(relationship(
            "model.Sys.owes",
            "model.Sys",
            one(),
            "model.Money",
            one(),
            RelationshipDirection::SourceToTarget,
        ));
    });
    let classification =
        classify(&domain_package, &mut unlimited_meter()).expect("classify completes");
    let refusals: Vec<_> = classification
        .refusals
        .iter()
        .filter(|refusal| {
            matches!(&refusal.cause, ModelRefusalCause::IntakeMalformedDeclaration { node, .. }
                if node == "model.Sys.owes")
        })
        .collect();
    assert_eq!(refusals.len(), 1, "{:?}", classification.refusals);
    assert_eq!(refusals[0].code, Code::InvalidModelBinding);
    assert_eq!(
        refusals[0].cause,
        ModelRefusalCause::IntakeMalformedDeclaration {
            node: "model.Sys.owes".to_owned(),
            artifact: None,
            span: None,
            member: "Relationship.target".parse().unwrap(),
        }
    );
}

/// FR-208-AC-9: a relationship source end naming a record value type refuses
/// `invalid_model_binding`/`malformed-declaration`, not as a dangling end:
/// the end names a declared type of the wrong meaning.
#[trace("QSpec-TC-197", "TC-997", "FR-056-AC-32")]
#[test]
fn a_relationship_source_end_naming_a_record_value_type_refuses_malformed() {
    let domain_package = fixture_y(|records| {
        records.push(DomainPackageRecord::RecordValueType(
            qsl_semantics::model::domain_package::RecordValueTypeRecord {
                key: DeclarationKey::fixture("model.Money"),
                identity_fields: Vec::new(),
                supertypes: Vec::new(),
            },
        ));
        records.push(relationship(
            "model.Sys.owes",
            "model.Money",
            one(),
            "model.Sys",
            one(),
            RelationshipDirection::SourceToTarget,
        ));
    });
    let classification =
        classify(&domain_package, &mut unlimited_meter()).expect("classify completes");
    let refusals: Vec<_> = classification
        .refusals
        .iter()
        .filter(|refusal| {
            matches!(&refusal.cause, ModelRefusalCause::IntakeMalformedDeclaration { node, .. }
                if node == "model.Sys.owes")
        })
        .collect();
    assert_eq!(refusals.len(), 1, "{:?}", classification.refusals);
    assert_eq!(refusals[0].code, Code::InvalidModelBinding);
    assert_eq!(
        refusals[0].cause,
        ModelRefusalCause::IntakeMalformedDeclaration {
            node: "model.Sys.owes".to_owned(),
            artifact: None,
            span: None,
            member: "Relationship.source".parse().unwrap(),
        }
    );
}

#[trace("TC-235", "FR-086-AC-3")]
#[trace("QSpec-TC-197")]
#[test]
fn y01_every_kind_resolves_to_its_exact_producer_key() {
    let domain_package = fixture_y(|_| {});
    let mut meter = unlimited_meter();
    let classification = classify(&domain_package, &mut meter).expect("classify admitted");
    assert!(classification.refusals.is_empty());

    for (required, identity) in [
        (Kind::Part, "model.Sys.pump"),
        (Kind::Port, "model.Sys.pump.out"),
        (Kind::Interface, "model.Flow"),
        (Kind::Connection, "model.Sys.pipe"),
        (Kind::Allocation, "model.Pump.alloc"),
    ] {
        let key = DeclarationKey::fixture(identity);
        let resolved = resolve_kind(&classification, required, &key).unwrap_or_else(|refusal| {
            panic!("resolve({required:?}, {identity}) refused: {refusal:?}")
        });
        assert_eq!(resolved.kind, required);
        assert_eq!(resolved.key, key);
    }

    match check_connection(
        &ModelIndex::build(domain_package.clone()),
        &classification,
        &DeclarationKey::fixture("model.Sys.pipe"),
        &mut meter,
    ) {
        ConnectionCheckOutcome::Completed(ConnectionOutcome::Admitted) => {}
        other => panic!("expected the connection admitted, got {other:?}"),
    }
    match check_allocation(
        &classification,
        &DeclarationKey::fixture("model.Pump.alloc"),
        &mut meter,
    ) {
        AllocationCheckOutcome::Admitted => {}
        other => panic!("expected the allocation admitted, got {other:?}"),
    }
}

/// FR-152 systems connections have role-less ends and are validated by the
/// systems boundary; model normalization must retain them for that pass.
#[trace("QSpec-TC-197")]
#[test]
fn y01_roleless_systems_connection_survives_model_normalization() {
    let domain_package = fixture_y(|_| {});
    match normalize(&domain_package, ModelNormalizationLimits::UNLIMITED) {
        NormalizeOutcome::Completed(_) => {}
        other => panic!("the role-less systems connection must survive normalization: {other:?}"),
    }
}

/// Phase 2 retains each systems member at its resolved end-owner type.
#[trace("TC-213", "FR-081-AC-1")]
#[trace("QSpec-TC-197", "QSpec-FR-150-AC-1")]
#[test]
fn systems_members_have_effective_owner_entries() {
    let view = match normalize(&fixture_y(|_| {}), ModelNormalizationLimits::UNLIMITED) {
        NormalizeOutcome::Completed(view) => view,
        other => panic!("systems fixture must normalize: {other:?}"),
    };
    let cases = [
        ("model.Sys", "model.Sys.pump", 1),
        ("model.Sys", "model.Sys.pump.out", 1),
        ("model.Sys", "model.Sys.pipe", 2),
        ("model.Pump", "model.Pump.alloc", 1),
        ("model.Sys", "model.Pump.alloc", 1),
        ("model.Sys", "model.rel.parts", 1),
        ("model.Pump", "model.rel.home", 1),
    ];
    for (owner, member, qualify_facts) in cases {
        let owner = view.type_identities()[&DeclarationKey::fixture(owner)];
        let entries: Vec<_> = view
            .declarations()
            .iter()
            .filter(|entry| {
                entry.preimage.owner_effective_type == Some(owner)
                    && entry.preimage.original == DeclarationKey::fixture(member)
            })
            .collect();
        assert_eq!(
            entries.len(),
            1,
            "{member} must have one effective entry at its owner"
        );
        let facts = &entries[0].preimage.derivation;
        assert_eq!(
            facts.len(),
            qualify_facts,
            "{member} must retain every end fact"
        );
        assert!(facts.iter().all(|fact| fact.rule == RULE_QUALIFY));
    }
}

/// A subtype inherits every systems member from its composite ancestor. An
/// inherited member carries one inherit fact per direct supertype that has it
/// and no qualify fact (TC-195 N01), even for the role-less connection whose owner holds two
/// same-owner qualify facts. This also exercises the repeated same-owner
/// path control: dropping the ancestor member grouping makes the inherit
/// fact assertions fail. The fact-boundary assertion below stops before any
/// inherited fact is charged and does not catch that mutation.
#[trace("TC-213", "QSpec-TC-197", "QSpec-FR-150-AC-1")]
#[test]
fn systems_members_inherit_with_ordered_duplicate_end_facts() {
    let view = match normalize(&fixture_y(|_| {}), ModelNormalizationLimits::UNLIMITED) {
        NormalizeOutcome::Completed(view) => view,
        other => panic!("systems subtype fixture must normalize: {other:?}"),
    };
    let owner = view.type_identities()[&DeclarationKey::fixture("model.SysSub")];
    for (member, expected_rules, expected_inputs) in [
        (
            "model.Sys.pump",
            vec![RULE_INHERIT.identity, RULE_INHERIT.identity],
            vec![
                vec![
                    DeclarationKey::fixture("model.Sys"),
                    DeclarationKey::fixture("model.Sys.pump"),
                ],
                vec![
                    DeclarationKey::fixture("model.SysBridge"),
                    DeclarationKey::fixture("model.Sys.pump"),
                ],
            ],
        ),
        (
            "model.Sys.pipe",
            vec![RULE_INHERIT.identity, RULE_INHERIT.identity],
            vec![
                vec![
                    DeclarationKey::fixture("model.Sys"),
                    DeclarationKey::fixture("model.Sys.pipe"),
                ],
                vec![
                    DeclarationKey::fixture("model.SysBridge"),
                    DeclarationKey::fixture("model.Sys.pipe"),
                ],
            ],
        ),
        (
            "model.Sys.pump.out",
            vec![RULE_INHERIT.identity, RULE_INHERIT.identity],
            vec![
                vec![
                    DeclarationKey::fixture("model.Sys"),
                    DeclarationKey::fixture("model.Sys.pump.out"),
                ],
                vec![
                    DeclarationKey::fixture("model.SysBridge"),
                    DeclarationKey::fixture("model.Sys.pump.out"),
                ],
            ],
        ),
        (
            "model.rel.parts",
            vec![RULE_INHERIT.identity, RULE_INHERIT.identity],
            vec![
                vec![
                    DeclarationKey::fixture("model.Sys"),
                    DeclarationKey::fixture("model.rel.parts"),
                ],
                vec![
                    DeclarationKey::fixture("model.SysBridge"),
                    DeclarationKey::fixture("model.rel.parts"),
                ],
            ],
        ),
        (
            "model.Pump.alloc",
            vec![RULE_INHERIT.identity, RULE_INHERIT.identity],
            vec![
                vec![
                    DeclarationKey::fixture("model.Sys"),
                    DeclarationKey::fixture("model.Pump.alloc"),
                ],
                vec![
                    DeclarationKey::fixture("model.SysBridge"),
                    DeclarationKey::fixture("model.Pump.alloc"),
                ],
            ],
        ),
    ] {
        let entry = view
            .declarations()
            .iter()
            .find(|entry| {
                entry.preimage.owner_effective_type == Some(owner)
                    && entry.preimage.original == DeclarationKey::fixture(member)
            })
            .unwrap_or_else(|| panic!("{member} is inherited by model.SysSub"));
        assert_eq!(
            entry
                .preimage
                .derivation
                .iter()
                .map(|fact| fact.rule.identity)
                .collect::<Vec<_>>(),
            expected_rules,
            "{member} retains the full ordered derivation"
        );
        assert_eq!(
            entry
                .preimage
                .derivation
                .iter()
                .map(|fact| fact.inputs.clone())
                .collect::<Vec<_>>(),
            expected_inputs,
            "{member} retains every inherited input path"
        );
    }

    let mut limits = ModelNormalizationLimits::UNLIMITED;
    // The reduced fixture below has six type facts and eleven direct member
    // facts, followed by four repeated-path type facts. The next charge is
    // the first inherited systems member fact, so this boundary cannot pass
    // if the ancestor members are dropped before phase 3.
    limits.derivation_facts = 21;
    match normalize(&systems_only_repeated_fixture(), limits) {
        NormalizeOutcome::Incomplete(incomplete) => {
            assert_eq!(incomplete.charge_point, ChargePoint::NormalizeFact);
            assert_eq!(incomplete.consumed, 21);
        }
        other => panic!("the inherited systems facts must hit the fact boundary: {other:?}"),
    }
}

fn systems_only_repeated_fixture() -> DomainPackage {
    let keep = [
        "model.Flow",
        "model.Pump",
        "model.Sys",
        "model.SysBridge",
        "model.SysSub",
        "model.Tank",
        "model.Sys.pump",
        "model.Sys.tank",
        "model.Sys.pump.out",
        "model.Sys.tank.in",
        "model.Sys.pipe",
        "model.Pump.run",
        "model.Pump.alloc",
        "model.rel.parts",
        "model.rel.home",
    ];
    let mut package = fixture_y(|records| {
        records.retain(|record| keep.contains(&record.key().node.as_str()));
    });
    package
        .records
        .retain(|record| keep.contains(&record.key().node.as_str()));
    package
}

#[trace("TC-235", "FR-086-AC-3")]
#[trace("QSpec-TC-197", "QSpec-FR-152-AC-2")]
#[test]
fn y02_wrong_export_substitutions_name_the_required_and_actual_kind() {
    let domain_package = fixture_y(|_| {});
    let mut meter = unlimited_meter();
    let classification = classify(&domain_package, &mut meter).expect("classify admitted");

    let refusal = resolve_kind(
        &classification,
        Kind::Port,
        &DeclarationKey::fixture("model.Sys.pump"),
    )
    .expect_err("model.Sys.pump is a Part, not a Port");
    assert_eq!(refusal.code, Code::InvalidModelBinding);
    assert_eq!(refusal.cause, ModelRefusalCause::WrongExport);
    assert!(refusal.detail.contains("required kind Port"));
    assert!(refusal.detail.contains("actual kind Part"));

    let refusal = resolve_kind(
        &classification,
        Kind::Allocation,
        &DeclarationKey::fixture("model.Sys.pipe"),
    )
    .expect_err("model.Sys.pipe is a Connection, not an Allocation");
    assert_eq!(refusal.code, Code::InvalidModelBinding);
    assert_eq!(refusal.cause, ModelRefusalCause::WrongExport);
    assert!(refusal.detail.contains("required kind Allocation"));
    assert!(refusal.detail.contains("actual kind Connection"));

    let domain_package = fixture_y(|records| {
        find_allocation(records, "model.Pump.alloc").target_element =
            DeclarationKey::fixture("model.Sys.pump.out");
    });
    let classification =
        classify(&domain_package, &mut unlimited_meter()).expect("classify admitted");
    match check_allocation(
        &classification,
        &DeclarationKey::fixture("model.Pump.alloc"),
        &mut unlimited_meter(),
    ) {
        AllocationCheckOutcome::Refused(refusal) => {
            assert_eq!(refusal.code, Code::InvalidModelBinding);
            assert_eq!(refusal.cause, ModelRefusalCause::WrongExport);
            assert!(refusal.detail.contains("required kind Part"));
            assert!(refusal.detail.contains("actual kind Port"));
        }
        other => panic!("expected the allocation refused, got {other:?}"),
    }
}

// Retagged AC-3 -> AC-6 (PR #144 review finding #1): y03a-e test the
// per-`RelationshipDirection` port-pair admit/refuse rule directly — AC-6's
// own subject.
#[trace("QSpec-TC-197", "QSpec-FR-152-AC-6")]
#[test]
fn y03a_swapped_connection_ends_refuse_on_port_direction() {
    let domain_package = fixture_y(|records| {
        let pipe = find_relationship(records, "model.Sys.pipe");
        pipe.source.type_identity = DeclarationKey::fixture("model.Sys.tank.in");
        pipe.target.type_identity = DeclarationKey::fixture("model.Sys.pump.out");
    });
    let mut meter = unlimited_meter();
    let classification = classify(&domain_package, &mut meter).expect("classify admitted");
    match check_connection(
        &ModelIndex::build(domain_package.clone()),
        &classification,
        &DeclarationKey::fixture("model.Sys.pipe"),
        &mut meter,
    ) {
        ConnectionCheckOutcome::Completed(ConnectionOutcome::Refused(failures)) => {
            assert_eq!(failures.len(), 1);
            assert_eq!(failures[0].condition, "port-direction");
            assert_eq!(
                failures[0].cause,
                ModelRefusalCause::PortDirection {
                    source: DeclarationKey::fixture("model.Sys.tank.in"),
                    target: DeclarationKey::fixture("model.Sys.pump.out"),
                }
            );
        }
        other => panic!("expected a port-direction refusal, got {other:?}"),
    }
}

#[trace("QSpec-TC-197", "QSpec-FR-152-AC-6")]
#[test]
fn y03b_target_to_source_against_the_declared_ports_refuses() {
    let domain_package = fixture_y(|records| {
        find_relationship(records, "model.Sys.pipe").direction =
            RelationshipDirection::TargetToSource;
    });
    let mut meter = unlimited_meter();
    let classification = classify(&domain_package, &mut meter).expect("classify admitted");
    match check_connection(
        &ModelIndex::build(domain_package.clone()),
        &classification,
        &DeclarationKey::fixture("model.Sys.pipe"),
        &mut meter,
    ) {
        ConnectionCheckOutcome::Completed(ConnectionOutcome::Refused(failures)) => {
            assert_eq!(failures[0].condition, "port-direction");
        }
        other => panic!("expected a port-direction refusal, got {other:?}"),
    }
}

#[trace("QSpec-TC-197", "QSpec-FR-152-AC-6")]
#[test]
fn y03c_bidirectional_against_non_inout_ports_refuses() {
    let domain_package = fixture_y(|records| {
        find_relationship(records, "model.Sys.pipe").direction =
            RelationshipDirection::Bidirectional;
    });
    let mut meter = unlimited_meter();
    let classification = classify(&domain_package, &mut meter).expect("classify admitted");
    match check_connection(
        &ModelIndex::build(domain_package.clone()),
        &classification,
        &DeclarationKey::fixture("model.Sys.pipe"),
        &mut meter,
    ) {
        ConnectionCheckOutcome::Completed(ConnectionOutcome::Refused(failures)) => {
            assert_eq!(failures[0].condition, "port-direction");
        }
        other => panic!("expected a port-direction refusal, got {other:?}"),
    }
}

#[trace("QSpec-TC-197", "QSpec-FR-152-AC-6")]
#[test]
fn y03d_bidirectional_with_both_ports_inout_admits() {
    let domain_package = fixture_y(|records| {
        find_relationship(records, "model.Sys.pipe").direction =
            RelationshipDirection::Bidirectional;
        find_endpoint(records, "model.Sys.pump.out").direction = Some(PortDirection::InOut);
        find_endpoint(records, "model.Sys.tank.in").direction = Some(PortDirection::InOut);
    });
    let mut meter = unlimited_meter();
    let classification = classify(&domain_package, &mut meter).expect("classify admitted");
    match check_connection(
        &ModelIndex::build(domain_package.clone()),
        &classification,
        &DeclarationKey::fixture("model.Sys.pipe"),
        &mut meter,
    ) {
        ConnectionCheckOutcome::Completed(ConnectionOutcome::Admitted) => {}
        other => panic!("expected the connection admitted, got {other:?}"),
    }
}

#[trace("QSpec-TC-197", "QSpec-FR-152-AC-6")]
#[test]
fn y03e_undirected_is_never_a_connection_direction() {
    let domain_package = fixture_y(|records| {
        find_relationship(records, "model.Sys.pipe").direction = RelationshipDirection::Undirected;
    });
    let mut meter = unlimited_meter();
    let classification = classify(&domain_package, &mut meter).expect("classify admitted");
    match check_connection(
        &ModelIndex::build(domain_package.clone()),
        &classification,
        &DeclarationKey::fixture("model.Sys.pipe"),
        &mut meter,
    ) {
        ConnectionCheckOutcome::Completed(ConnectionOutcome::Refused(failures)) => {
            assert_eq!(failures[0].condition, "port-direction");
        }
        other => panic!("expected a port-direction refusal, got {other:?}"),
    }
}

// Retagged AC-3 -> AC-6 (PR #144 review finding #1; AC-4 kept, per the same
// finding): this exercises the interface-type condition (AC-4: "port
// direction, endpoint type and multiplicity are all enforced") over a
// connection whose port direction is the same admissible pair y03d already
// covers under AC-6.
#[trace("QSpec-TC-197", "QSpec-FR-152-AC-4", "QSpec-FR-152-AC-6")]
#[test]
fn y04_flow_source_must_conform_to_flow_target_not_the_reverse() {
    // model.Flow2 generalizes to model.Flow (Flow2 is more derived), so
    // retyping the flow TARGET (tank.in) to Flow2 asks whether the
    // unrelated-in-that-direction model.Flow conforms to model.Flow2: it
    // does not.
    let domain_package = fixture_y(|records| {
        find_endpoint(records, "model.Sys.tank.in").value_type =
            DeclarationKey::fixture("model.Flow2");
    });
    let mut meter = unlimited_meter();
    let classification = classify(&domain_package, &mut meter).expect("classify admitted");
    let before = meter.consumed(qsl_semantics::model::accounting::LimitKind::WorkUnits);
    match check_connection(
        &ModelIndex::build(domain_package.clone()),
        &classification,
        &DeclarationKey::fixture("model.Sys.pipe"),
        &mut meter,
    ) {
        ConnectionCheckOutcome::Completed(ConnectionOutcome::Refused(failures)) => {
            let interface_failure = failures
                .iter()
                .find(|f| f.condition == "interface-type")
                .expect("an interface-type failure");
            assert_eq!(interface_failure.code, Code::IllTyped);
            assert_eq!(interface_failure.cause, ModelRefusalCause::TypeMismatch);
            assert!(interface_failure.detail.contains("model.Flow"));
            assert!(interface_failure.detail.contains("model.Flow2"));
        }
        other => panic!("expected an interface-type refusal, got {other:?}"),
    }
    let after = meter.consumed(qsl_semantics::model::accounting::LimitKind::WorkUnits);
    // Three systems.connection-condition charges, one flat work unit each
    // (this module's recorded scope choice — see systems.rs's module doc —
    // not FR-152's own f(Flow)/f(Flow2) derivation-fact pricing).
    assert_eq!(after - before, 3);

    // Retyping the flow SOURCE (pump.out) to Flow2 instead: Flow2 conforms
    // to Flow directly, so the connection admits.
    let domain_package = fixture_y(|records| {
        find_endpoint(records, "model.Sys.pump.out").value_type =
            DeclarationKey::fixture("model.Flow2");
    });
    let mut meter = unlimited_meter();
    let classification = classify(&domain_package, &mut meter).expect("classify admitted");
    match check_connection(
        &ModelIndex::build(domain_package.clone()),
        &classification,
        &DeclarationKey::fixture("model.Sys.pipe"),
        &mut meter,
    ) {
        ConnectionCheckOutcome::Completed(ConnectionOutcome::Admitted) => {}
        other => panic!("expected the connection admitted, got {other:?}"),
    }
    let charges = meter
        .admitted_charges()
        .iter()
        .filter(|point| **point == ChargePoint::SystemsConnectionCondition)
        .count();
    assert_eq!(charges, 3);
}

// Retagged AC-3 -> AC-6 (PR #144 review finding #1).
#[trace("QSpec-TC-197", "QSpec-FR-152-AC-6")]
#[test]
fn y05_a_narrowed_end_multiplicity_refuses_and_exposes_no_connection() {
    let domain_package = fixture_y(|records| {
        find_relationship(records, "model.Sys.pipe")
            .target
            .multiplicity = mult(0, Some(2));
    });
    let mut meter = unlimited_meter();
    let classification = classify(&domain_package, &mut meter).expect("classify admitted");
    match check_connection(
        &ModelIndex::build(domain_package.clone()),
        &classification,
        &DeclarationKey::fixture("model.Sys.pipe"),
        &mut meter,
    ) {
        ConnectionCheckOutcome::Completed(ConnectionOutcome::Refused(failures)) => {
            let failure = failures
                .iter()
                .find(|f| f.condition == "multiplicity")
                .expect("a multiplicity failure");
            assert_eq!(failure.code, Code::IllTyped);
            assert_eq!(
                failure.cause,
                ModelRefusalCause::MultiplicityNarrowing {
                    from: mult(0, Some(2)),
                    to: one(),
                }
            );
        }
        other => panic!("expected a multiplicity refusal (no connection exposed), got {other:?}"),
    }
}

// FR-152-AC-5 dropped (retagged, PR #144 review finding #1): AC-5's real
// subject is display-name-vs-identity substitution in relationship
// navigation, which finding #2 showed this module got wrong before this
// PR's re-keying fix — this test never exercises that axis, so it stays
// honestly unbacked. Also renamed: the cascade is three refusals
// (component -> endpoint -> relationship), not four.
#[trace("QSpec-TC-197", "QSpec-FR-152-AC-2")]
#[test]
fn y06_removing_the_part_capability_cascades_three_refusals_in_rule_order() {
    let domain_package = fixture_y(|records| {
        find_component(records, "model.Sys.pump").has_part_signature = false;
    });
    let mut meter = unlimited_meter();
    let classification = classify(&domain_package, &mut meter).expect("classify admitted");

    assert_eq!(classification.refusals.len(), 3);
    let cascade: Vec<(&Code, ModelRefusalCause)> = classification
        .refusals
        .iter()
        .map(|r: &ModelRefusal| (&r.code, r.cause.clone()))
        .collect();
    assert_eq!(
        cascade,
        vec![
            (
                &Code::InvalidModelBinding,
                ModelRefusalCause::UnsuppliedProducerRecord
            ),
            (&Code::InvalidModelBinding, ModelRefusalCause::WrongExport),
            (&Code::InvalidModelBinding, ModelRefusalCause::WrongExport),
        ]
    );
    assert!(classification.refusals[0].detail.contains("model.Sys.pump"));
    assert!(classification.refusals[0].detail.contains("part-signature"));
    assert!(classification.refusals[1]
        .detail
        .contains("model.Sys.pump.out"));
    assert!(classification.refusals[1]
        .detail
        .contains("actual kind none"));
    assert!(classification.refusals[2].detail.contains("model.Sys.pipe"));
    assert!(classification.refusals[2]
        .detail
        .contains("actual kind none"));

    match check_allocation(
        &classification,
        &DeclarationKey::fixture("model.Pump.alloc"),
        &mut meter,
    ) {
        AllocationCheckOutcome::Refused(refusal) => {
            assert_eq!(refusal.code, Code::InvalidModelBinding);
            assert_eq!(refusal.cause, ModelRefusalCause::WrongExport);
            assert!(refusal.detail.contains("model.Sys.pump"));
            assert!(refusal.detail.contains("required kind Part"));
            assert!(refusal.detail.contains("actual kind none"));
        }
        other => panic!("expected the allocation refused, got {other:?}"),
    }

    let refusal = resolve_kind(
        &classification,
        Kind::Part,
        &DeclarationKey::fixture("model.Sys.pump"),
    )
    .expect_err("model.Sys.pump no longer resolves to any kind");
    assert_eq!(refusal.cause, ModelRefusalCause::WrongExport);
    assert!(refusal.detail.contains("required kind Part"));
    assert!(refusal.detail.contains("actual kind none"));
}

/// TC-197 Y06 as written: `pump_out`'s owner changed to the declared object
/// type `Pump`. Exactly two `invalid_model_binding`/`wrong-export` refusals
/// follow, both with actual kind `none`: the port (required `Part`) and the
/// source end of `pipe` (required `Port`). Neither is a dangling reference,
/// because `Pump` is declared; `pipe` gets no kind and no connection check.
///
/// Mutation used (derived by reading, not executed): treating an owner absent
/// from the component map as an absent package key yields
/// `dangling_reference`/`unknown-component` and fails the code assertions.
#[trace("QSpec-TC-197", "QSpec-FR-152-AC-8")]
#[test]
fn y06_as_written_an_object_type_owner_refuses_wrong_export_with_actual_kind_none() {
    let domain_package = fixture_y(|records| {
        find_endpoint(records, "model.Sys.pump.out").owning_component =
            DeclarationKey::fixture("model.Pump");
    });
    let classification =
        classify(&domain_package, &mut unlimited_meter()).expect("classify admitted");
    assert_eq!(
        classification.refusals.len(),
        2,
        "{:?}",
        classification.refusals
    );
    for refusal in &classification.refusals {
        assert_eq!(refusal.code, Code::InvalidModelBinding);
        assert_eq!(refusal.cause, ModelRefusalCause::WrongExport);
        assert!(refusal.detail.contains("actual kind none"), "{refusal:?}");
    }
    assert!(classification.refusals[0]
        .detail
        .contains("model.Sys.pump.out"));
    assert!(classification.refusals[0]
        .detail
        .contains("required kind Part"));
    assert!(classification.refusals[1].detail.contains("model.Sys.pipe"));
    assert!(classification.refusals[1]
        .detail
        .contains("required kind Port"));
    assert!(matches!(
        check_connection(
            &ModelIndex::build(domain_package.clone()),
            &classification,
            &DeclarationKey::fixture("model.Sys.pipe"),
            &mut unlimited_meter(),
        ),
        ConnectionCheckOutcome::Refused(_)
    ));
}

/// TC-197 Y10 clause 2: `pipe`'s target end replaced by a declared node that
/// has no systems kind (here the object type `Pump`) refuses
/// `invalid_model_binding`/`wrong-export` with required `Port` and actual
/// `none`, never `dangling_reference`, and `pipe` gets no kind.
///
/// Mutation used (derived by reading, not executed): looking the end up only
/// in the endpoint map reports `unknown-endpoint` and fails the code check.
#[trace("QSpec-TC-197", "QSpec-FR-152-AC-8")]
#[test]
fn y10_clause_2_a_kindless_declared_end_refuses_wrong_export_not_dangling() {
    let domain_package = fixture_y(|records| {
        find_relationship(records, "model.Sys.pipe")
            .target
            .type_identity = DeclarationKey::fixture("model.Pump");
    });
    let classification =
        classify(&domain_package, &mut unlimited_meter()).expect("classify admitted");
    let [refusal] = classification.refusals.as_slice() else {
        panic!("one refusal: {:?}", classification.refusals);
    };
    assert_eq!(refusal.code, Code::InvalidModelBinding);
    assert_eq!(refusal.cause, ModelRefusalCause::WrongExport);
    assert!(refusal.detail.contains("required kind Port"), "{refusal:?}");
    assert!(refusal.detail.contains("actual kind none"), "{refusal:?}");
    assert_eq!(
        classification.actual_kind(&DeclarationKey::fixture("model.Sys.pipe")),
        Kind::None
    );
}

/// FR-152-AC-3, dangling end reference: an endpoint whose owning component is
/// absent refuses `dangling_reference`/`unknown-component` and a relationship
/// end naming an absent endpoint refuses `unknown-endpoint`, both naming the
/// absent key. Neither is substituted by a same-named declaration: the
/// affected nodes resolve to no kind, and no connection is exposed. The
/// ambiguous-effective-target half of AC-3 needs the checker's member
/// resolution and is not exercised here.
///
/// Mutation used (derived by reading, not executed): resolving the absent key
/// through a name match or a default component classifies the endpoint as a
/// Port and drops the dangling refusal, failing the first assertion.
#[trace("QSpec-TC-197", "QSpec-FR-152-AC-3")]
#[test]
fn a_dangling_end_reference_refuses_without_substituting_a_declaration() {
    let domain_package = fixture_y(|records| {
        find_endpoint(records, "model.Sys.pump.out").owning_component =
            DeclarationKey::fixture("model.Sys.nowhere");
    });
    let classification =
        classify(&domain_package, &mut unlimited_meter()).expect("classify admitted");
    let dangling = &classification.refusals[0];
    assert_eq!(dangling.code, Code::DanglingReference);
    assert_eq!(
        dangling.cause,
        ModelRefusalCause::UnknownComponent {
            item: DeclarationKey::fixture("model.Sys.pump.out"),
            missing: DeclarationKey::fixture("model.Sys.nowhere"),
        }
    );
    let refusal = resolve_kind(
        &classification,
        Kind::Port,
        &DeclarationKey::fixture("model.Sys.pump.out"),
    )
    .expect_err("the endpoint with a dangling owner has no kind");
    assert_eq!(refusal.cause, ModelRefusalCause::WrongExport);
    assert!(matches!(
        check_connection(
            &ModelIndex::build(domain_package.clone()),
            &classification,
            &DeclarationKey::fixture("model.Sys.pipe"),
            &mut unlimited_meter(),
        ),
        ConnectionCheckOutcome::Refused(_)
    ));

    let domain_package = fixture_y(|records| {
        find_relationship(records, "model.Sys.pipe")
            .target
            .type_identity = DeclarationKey::fixture("model.Sys.nowhere");
    });
    let classification =
        classify(&domain_package, &mut unlimited_meter()).expect("classify admitted");
    let dangling = classification
        .refusals
        .iter()
        .find(|refusal| refusal.code == Code::DanglingReference)
        .expect("the absent target endpoint is a dangling reference");
    assert_eq!(
        dangling.cause,
        ModelRefusalCause::UnknownEndpoint {
            end: "target",
            relationship: DeclarationKey::fixture("model.Sys.pipe"),
            missing: DeclarationKey::fixture("model.Sys.nowhere"),
        }
    );
}

/// The bidirectional interface-type condition compares `flow_source` and
/// `flow_target` by their full `DeclarationKey`: two ports whose value
/// types share a display node but declare it in different packages are not
/// the same interface type. `model.Sys.pump.out` and `model.Sys.tank.in`
/// both name an interface type with node `model.Flow`, one in `test/orders`
/// and one in `other/pkg`.
#[trace("QSpec-TC-197", "QSpec-FR-152-AC-4", "QSpec-FR-152-AC-6")]
#[test]
fn y07_bidirectional_interface_type_does_not_confuse_two_packages_sharing_a_node() {
    let domain_package = fixture_y(|records| {
        find_relationship(records, "model.Sys.pipe").direction =
            RelationshipDirection::Bidirectional;
        find_endpoint(records, "model.Sys.pump.out").direction = Some(PortDirection::InOut);
        find_endpoint(records, "model.Sys.tank.in").direction = Some(PortDirection::InOut);
        let other_flow = DeclarationKey {
            package: "other/pkg".to_owned(),
            node: "model.Flow".to_owned(),
        };
        records.push(DomainPackageRecord::ObjectType(ObjectTypeRecord {
            key: other_flow.clone(),
            interface_features: Some(vec![]),
            abstract_type: false,
            supertypes: vec![],
        }));
        find_endpoint(records, "model.Sys.tank.in").value_type = other_flow;
    });
    let mut meter = unlimited_meter();
    let classification = classify(&domain_package, &mut meter).expect("classify admitted");
    match check_connection(
        &ModelIndex::build(domain_package.clone()),
        &classification,
        &DeclarationKey::fixture("model.Sys.pipe"),
        &mut meter,
    ) {
        ConnectionCheckOutcome::Completed(ConnectionOutcome::Refused(failures)) => {
            assert_eq!(
                failures,
                vec![ConditionFailure {
                    condition: "interface-type",
                    code: Code::IllTyped,
                    cause: ModelRefusalCause::TypeMismatch,
                    detail: "model.Flow does not conform to model.Flow".to_owned(),
                }]
            );
        }
        other => panic!("expected an interface-type refusal, got {other:?}"),
    }
}

/// FR-154: "Two nodes share one identity" refuses
/// `invalid_model_binding`/`conflicting-binding`. Retargets the pre-#131
/// `f2_components_sharing_an_identity_but_differing_in_revision_both_survive`
/// regression test: under the dropped `revision`/`digest` fields, two
/// `Component` records that once differed only in `revision` now share the
/// exact same `DeclarationKey`, and `normalize` -- which every real
/// pipeline runs before `classify` ever sees a domain package -- must
/// refuse before either component reaches `SystemsClassification`.
#[trace("QSpec-TC-197")]
#[test]
fn two_components_sharing_one_declaration_key_refuse_conflicting_binding() {
    let domain_package = DomainPackage::new(
        DomainPackageRef::fixture("bundle.f2-systems-conflict"),
        vec![
            object_type("model.Pump", None, vec![]),
            DomainPackageRecord::Component(ComponentRecord {
                key: DeclarationKey::fixture("model.Sys.pump"),
                owning_type: DeclarationKey::fixture("model.Sys"),
                value_type: DeclarationKey::fixture("model.Pump"),
                multiplicity: one(),
                has_part_signature: true,
            }),
            DomainPackageRecord::Component(ComponentRecord {
                key: DeclarationKey::fixture("model.Sys.pump"),
                owning_type: DeclarationKey::fixture("model.Sys"),
                value_type: DeclarationKey::fixture("model.Pump"),
                multiplicity: one(),
                has_part_signature: false,
            }),
        ],
    );
    match normalize(&domain_package, ModelNormalizationLimits::UNLIMITED) {
        NormalizeOutcome::Refused(refusal) => {
            assert_eq!(
                refusal.len(),
                1,
                "expected exactly one refusal: {refusal:?}"
            );
            let refusal = refusal[0].clone();
            assert_eq!(refusal.code, Code::InvalidModelBinding);
            assert_eq!(
                refusal.cause,
                ModelRefusalCause::ConflictingBinding {
                    key: DeclarationKey::fixture("model.Sys.pump"),
                    member: None,
                }
            );
        }
        other => {
            panic!("expected Refused(invalid_model_binding/conflicting-binding), got {other:?}")
        }
    }
}

fn refusal_codes(
    classification: &qsl_semantics::model::systems::SystemsClassification,
) -> Vec<(Code, &'static str)> {
    classification
        .refusals
        .iter()
        .map(|refusal| (refusal.code, refusal.cause.as_str()))
        .collect()
}

/// TC-233 (FR-086-AC-1): an endpoint with no direction and an endpoint whose
/// owner names no declared node both cascade to no kind, and the
/// interface-qualified object type classifies `Interface`; both cascades are
/// reported, in declaration-key order, and each record is charged once.
///
/// Mutation used (derived by reading, not executed): returning from
/// `classify` at the first pushed refusal leaves one entry and fails the
/// length assertion; dropping the interface charge fails the charge count.
#[trace("TC-233", "FR-086-AC-1")]
#[test]
fn tc_233_every_no_kind_cascade_is_reported_not_only_the_first() {
    let domain_package = DomainPackage::new(
        DomainPackageRef::fixture("bundle.tc233"),
        vec![
            object_type("model.Sys", None, vec![]),
            object_type("model.I1", Some(vec![]), vec![]),
            component("model.C2", "model.Sys", "model.Sys", one()),
            endpoint("model.E1", "model.C2", "model.I1", None, one()),
            endpoint(
                "model.E2",
                "model.C3",
                "model.I1",
                Some(PortDirection::Out),
                one(),
            ),
        ],
    );
    let mut meter = unlimited_meter();
    let classification = classify(&domain_package, &mut meter).expect("classify admitted");

    assert_eq!(
        refusal_codes(&classification),
        vec![
            (Code::InvalidModelBinding, "unsupplied-producer-record"),
            (Code::DanglingReference, "unknown-component"),
        ]
    );
    assert!(classification.refusals[0].detail.contains("model.E1"));
    assert!(classification.refusals[0].detail.contains("port-direction"));
    assert_eq!(
        classification.refusals[1].cause,
        ModelRefusalCause::UnknownComponent {
            item: DeclarationKey::fixture("model.E2"),
            missing: DeclarationKey::fixture("model.C3"),
        }
    );
    for (identity, kind) in [
        ("model.C2", Kind::Part),
        ("model.E1", Kind::None),
        ("model.E2", Kind::None),
        ("model.I1", Kind::Interface),
    ] {
        assert_eq!(
            classification.actual_kind(&DeclarationKey::fixture(identity)),
            kind,
            "{identity}"
        );
    }
    assert_eq!(
        meter.admission_count(),
        4,
        "one systems.kind charge each for I1, C2, E1 and E2"
    );
}

/// TC-234 (FR-086-AC-2): a port-direction failure and an end-multiplicity
/// failure are both reported while the interface-type condition holds, and
/// the unmodified connection admits.
///
/// Mutation used (derived by reading, not executed): returning after the
/// first failed condition yields one failure and fails the length assertion;
/// pushing `interface-type` unconditionally fails the absence assertion.
#[trace("TC-234", "FR-086-AC-2")]
#[test]
fn tc_234_a_connection_reports_every_failing_condition() {
    let pipe = DeclarationKey::fixture("model.Sys.pipe");
    let domain_package = fixture_y(|records| {
        find_endpoint(records, "model.Sys.pump.out").direction = Some(PortDirection::In);
        find_relationship(records, "model.Sys.pipe")
            .source
            .multiplicity = mult(0, Some(2));
    });
    let mut meter = unlimited_meter();
    let classification = classify(&domain_package, &mut meter).expect("classify admitted");
    match check_connection(
        &ModelIndex::build(domain_package.clone()),
        &classification,
        &pipe,
        &mut meter,
    ) {
        ConnectionCheckOutcome::Completed(ConnectionOutcome::Refused(failures)) => {
            let conditions: Vec<_> = failures.iter().map(|failure| failure.condition).collect();
            assert_eq!(conditions, vec!["port-direction", "multiplicity"]);
            assert_eq!(
                failures[0].cause,
                ModelRefusalCause::PortDirection {
                    source: DeclarationKey::fixture("model.Sys.pump.out"),
                    target: DeclarationKey::fixture("model.Sys.tank.in"),
                }
            );
            assert_eq!(
                failures[1].cause,
                ModelRefusalCause::MultiplicityNarrowing {
                    from: mult(0, Some(2)),
                    to: one(),
                }
            );
        }
        other => panic!("expected two failures, got {other:?}"),
    }

    let admitted = fixture_y(|_| {});
    let mut meter = unlimited_meter();
    let classification = classify(&admitted, &mut meter).expect("classify admitted");
    assert_eq!(
        check_connection(
            &ModelIndex::build(admitted.clone()),
            &classification,
            &pipe,
            &mut meter,
        ),
        ConnectionCheckOutcome::Completed(ConnectionOutcome::Admitted)
    );
}

/// TC-236 (FR-086-AC-4): a component and an endpoint that share a display
/// node but differ in declaration key classify and resolve independently.
///
/// Mutation used (derived by reading, not executed): indexing the kind maps
/// by `key.node` lets the endpoint overwrite or borrow the component's kind
/// and resolves the endpoint as `Part`, failing the refusal assertion.
#[trace("TC-236", "FR-086-AC-4")]
#[test]
fn tc_236_equal_display_nodes_resolve_independently_by_key() {
    let part_key = DeclarationKey::fixture("model.Pump");
    let port_key = DeclarationKey {
        package: "other/pkg".to_owned(),
        node: "model.Pump".to_owned(),
    };
    let domain_package = DomainPackage::new(
        DomainPackageRef::fixture("bundle.tc236"),
        vec![
            object_type("model.Sys", None, vec![]),
            object_type("model.Flow", Some(vec![]), vec![]),
            component("model.Pump", "model.Sys", "model.Sys", one()),
            DomainPackageRecord::Endpoint(EndpointRecord {
                key: port_key.clone(),
                owning_component: part_key.clone(),
                value_type: DeclarationKey::fixture("model.Flow"),
                direction: Some(PortDirection::Out),
                multiplicity: one(),
            }),
        ],
    );
    let classification =
        classify(&domain_package, &mut unlimited_meter()).expect("classify admitted");
    assert_eq!(classification.refusals, Vec::new());

    let resolved = resolve_kind(&classification, Kind::Part, &part_key).expect("the Part resolves");
    assert_eq!(resolved.key, part_key);
    assert_eq!(resolved.kind, Kind::Part);

    let refusal = resolve_kind(&classification, Kind::Part, &port_key)
        .expect_err("the endpoint is a Port, not a Part");
    assert_eq!(refusal.code, Code::InvalidModelBinding);
    assert_eq!(refusal.cause, ModelRefusalCause::WrongExport);
    assert!(refusal.detail.contains("required kind Part"), "{refusal:?}");
    assert!(refusal.detail.contains("actual kind Port"), "{refusal:?}");
    assert_eq!(classification.actual_kind(&part_key), Kind::Part);
    assert_eq!(classification.actual_kind(&port_key), Kind::Port);
}

/// TC-241 step 2 (FR-086-AC-5): the Part's key sorts before the Interface's
/// key, yet the Interface is charged and resolved first, so the Port that
/// names it classifies `Port`. Each of the three records is charged once.
///
/// Mutation used (derived by reading, not executed): collecting interfaces
/// inline during the record scan, or running the interface pass after the
/// endpoint pass, leaves the Port's interface type unresolved and refuses it,
/// and dropping the interface charge leaves the count at two.
#[trace("TC-241", "FR-086-AC-5")]
#[test]
fn tc_241_interfaces_classify_and_charge_before_the_port_that_names_them() {
    let domain_package = DomainPackage::new(
        DomainPackageRef::fixture("bundle.tc241-order"),
        vec![
            object_type("model.Sys", None, vec![]),
            component("model.A.part", "model.Sys", "model.Sys", one()),
            object_type("model.Z.iface", Some(vec![]), vec![]),
            endpoint(
                "model.M.port",
                "model.A.part",
                "model.Z.iface",
                Some(PortDirection::Out),
                one(),
            ),
        ],
    );
    let mut meter = unlimited_meter();
    let classification = classify(&domain_package, &mut meter).expect("classify admitted");
    assert_eq!(classification.refusals, Vec::new());
    assert_eq!(
        classification.actual_kind(&DeclarationKey::fixture("model.M.port")),
        Kind::Port
    );
    assert_eq!(
        classification.actual_kind(&DeclarationKey::fixture("model.Z.iface")),
        Kind::Interface
    );
    assert_eq!(meter.admission_count(), 3);
}

/// TC-241 step 4 (FR-086-AC-5): a Port whose interface-type reference is an
/// ordinary object type, a Part, or a key no record declares records one
/// wrong-export cascade naming required `Interface` and the reference's
/// actual kind, and gets no kind.
///
/// Mutation used (derived by reading, not executed): never checking the
/// reference classifies the Port and leaves no refusal.
#[trace("TC-241", "FR-086-AC-5")]
#[test]
fn tc_241_a_port_whose_interface_type_is_not_an_interface_refuses_wrong_export() {
    for (interface_type, actual) in [
        ("model.Pump", "none"),
        ("model.Sys.pump", "Part"),
        ("model.Nowhere", "none"),
    ] {
        let domain_package = fixture_y(|records| {
            find_endpoint(records, "model.Sys.pump.out").value_type =
                DeclarationKey::fixture(interface_type);
        });
        let classification =
            classify(&domain_package, &mut unlimited_meter()).expect("classify admitted");
        let port = DeclarationKey::fixture("model.Sys.pump.out");
        let port_refusals: Vec<_> = classification
            .refusals
            .iter()
            .filter(|refusal| refusal.detail.contains("model.Sys.pump.out"))
            .collect();
        let [refusal] = port_refusals.as_slice() else {
            panic!(
                "{interface_type}: one port refusal: {:?}",
                classification.refusals
            );
        };
        assert_eq!(refusal.code, Code::InvalidModelBinding, "{interface_type}");
        assert_eq!(
            refusal.cause,
            ModelRefusalCause::WrongExport,
            "{interface_type}"
        );
        assert!(
            refusal.detail.contains("required kind Interface"),
            "{interface_type}: {refusal:?}"
        );
        assert!(
            refusal.detail.contains(&format!("actual kind {actual}")),
            "{interface_type}: {refusal:?}"
        );
        assert_eq!(
            classification.actual_kind(&port),
            Kind::None,
            "{interface_type}"
        );
    }
}
