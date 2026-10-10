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
};
use qsl_semantics::model::index::ModelIndex;
use qsl_semantics::model::key::DeclarationKey;
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
        lower,
        upper,
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
#[trace("QSpec-TC-197")]
#[test]
fn a_relationship_end_naming_a_record_value_type_refuses_malformed() {
    let domain_package = fixture_y(|records| {
        records.push(DomainPackageRecord::RecordValueType(
            qsl_semantics::model::domain_package::RecordValueTypeRecord {
                key: DeclarationKey::fixture("model.Money"),
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
        .filter(|refusal| refusal.detail.contains("model.Sys.owes"))
        .collect();
    assert_eq!(refusals.len(), 1, "{:?}", classification.refusals);
    assert_eq!(refusals[0].code, Code::InvalidModelBinding);
    assert_eq!(refusals[0].cause, ModelRefusalCause::MalformedDeclaration);
    assert!(refusals[0].detail.contains("target end"));
    assert!(refusals[0].detail.contains("model.Money"));
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
    // Direction and multiplicity cost one each; the flow-source interface
    // Flow has one derivation fact, so its type condition also costs one.
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
    assert_eq!(refusal.cause, ModelRefusalCause::UnsuppliedProducerRecord);
}

/// Bidirectional ports require identical declared interfaces, even when one
/// interface properly specializes the other. The directional subtype case
/// is covered by Y04; this standalone condition fixture declares both types.
#[trace("QSpec-TC-197", "QSpec-FR-152-AC-4", "QSpec-FR-152-AC-6")]
#[test]
fn y07_bidirectional_interface_type_requires_identical_declared_interfaces() {
    for identical in [false, true] {
        let domain_package = fixture_y(|records| {
            find_relationship(records, "model.Sys.pipe").direction = RelationshipDirection::Bidirectional;
            find_endpoint(records, "model.Sys.pump.out").direction = Some(PortDirection::InOut);
            find_endpoint(records, "model.Sys.tank.in").direction = Some(PortDirection::InOut);
            find_endpoint(records, "model.Sys.tank.in").value_type =
                DeclarationKey::fixture(if identical { "model.Flow" } else { "model.Flow2" });
        });
        let mut meter = unlimited_meter();
        let classification = classify(&domain_package, &mut meter).expect("classify declared interfaces");
        assert!(classification.refusals.is_empty());
        match check_connection(
            &ModelIndex::build(domain_package.clone()),
            &classification,
            &DeclarationKey::fixture("model.Sys.pipe"),
            &mut meter,
        ) {
            ConnectionCheckOutcome::Completed(ConnectionOutcome::Admitted) if identical => {}
            ConnectionCheckOutcome::Completed(ConnectionOutcome::Refused(failures)) if !identical => {
                assert_eq!(failures, vec![ConditionFailure {
                    condition: "interface-type",
                    code: Code::IllTyped,
                    cause: ModelRefusalCause::TypeMismatch,
                    detail: "model.Flow does not conform to model.Flow2".to_owned(),
                }]);
            }
            other => panic!("expected identical-interface result {identical}, got {other:?}"),
        }
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
                }
            );
        }
        other => {
            panic!("expected Refused(invalid_model_binding/conflicting-binding), got {other:?}")
        }
    }
}


/// All supported member kinds use the same real normalization owner and
/// qualify/inherit producer. Standalone systems verdicts are not used as
/// evidence for these effective declarations or prices.
#[trace("QSpec-TC-196", "QSpec-TC-213", "FR-082-AC-8")]
#[test]
fn all_member_kinds_qualify_and_inherit_with_exact_end_owner_entries() {
    use std::collections::{BTreeMap, BTreeSet};
    use qsl_semantics::model::accounting::LimitKind;
    use qsl_semantics::model::intake::SelectedModels;
    use qsl_semantics::model::key::{RULE_INHERIT, RULE_QUALIFY};
    for reverse in [false, true] {
        let package = fixture_y(|records| {
            records.push(object_type("model.SysChild", None, vec!["model.Sys"]));
            records.push(object_type("model.PumpChild", None, vec!["model.Pump"]));
            if reverse { records.reverse(); }
        });
        let selected = SelectedModels::fixture("M", qsl_foundation::Span { start: 0, end: 1 },
            package, ModelNormalizationLimits::UNLIMITED).expect("one actual all-kind normalization operation");
        let view = &selected[0].view;
        let owners: BTreeMap<_, _> = view.type_identities().iter()
            .map(|(key, identity)| (*identity, key.clone())).collect();
        let qualified: BTreeSet<_> = view.declarations().iter().filter(|entry|
            entry.preimage.derivation.iter().any(|fact| fact.rule == RULE_QUALIFY))
            .map(|entry| (entry.preimage.owner_effective_type.map(|owner| owners[&owner].clone()),
                entry.preimage.original.clone())).collect();
        let mut expected = BTreeSet::new();
        for ty in ["model.Sys", "model.Pump", "model.Tank", "model.Flow", "model.Flow2",
            "model.SysChild", "model.PumpChild"] {
            expected.insert((None, DeclarationKey::fixture(ty)));
        }
        for (owner, members) in [
            ("model.Sys", vec!["model.Sys.pump", "model.Sys.tank", "model.Sys.pump.out", "model.Sys.tank.in",
                "model.Sys.pipe", "model.Pump.alloc", "model.rel.parts"]),
            ("model.Pump", vec!["model.Pump.run", "model.Pump.alloc", "model.rel.home"]),
            ("model.Flow", vec!["model.Flow.rate"]),
        ] {
            for member in members { expected.insert((Some(DeclarationKey::fixture(owner)), DeclarationKey::fixture(member))); }
        }
        assert_eq!(qualified, expected, "all original-owner qualification pairs, reverse={reverse}");
        for (child, parent, members) in [
            ("model.SysChild", "model.Sys", vec!["model.Sys.pump", "model.Sys.tank", "model.Sys.pump.out", "model.Sys.tank.in",
                "model.Sys.pipe", "model.Pump.alloc", "model.rel.parts"]),
            ("model.PumpChild", "model.Pump", vec!["model.Pump.run", "model.Pump.alloc", "model.rel.home"]),
            ("model.Flow2", "model.Flow", vec!["model.Flow.rate"]),
        ] {
            let owner = view.type_identities()[&DeclarationKey::fixture(child)];
            for member in members {
                let entry = view.declarations().iter().find(|entry|
                    entry.preimage.owner_effective_type == Some(owner)
                        && entry.preimage.original == DeclarationKey::fixture(member)).expect("inherited member entry");
                assert_eq!(entry.preimage.derivation.len(), 1);
                assert_eq!(entry.preimage.derivation[0].rule, RULE_INHERIT);
                assert_eq!(entry.preimage.derivation[0].inputs.iter().cloned().collect::<Vec<_>>(),
                    vec![DeclarationKey::fixture(parent), DeclarationKey::fixture(member)]);
            }
        }
        assert!(!view.declarations().iter().any(|entry| entry.preimage.original == DeclarationKey::fixture("model.Count")));
        assert_eq!(selected.consumed(LimitKind::DeclarationRecords), 18);
        assert_eq!(selected.consumed(LimitKind::EffectiveDeclarations), 29);
        assert_eq!(selected.consumed(LimitKind::DerivationFacts), 32);
    }
}
