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
use quire_spec_language::diagnostic::Code;
use quire_spec_language::model::accounting::{ChargePoint, Meter, ModelNormalizationLimits};
use quire_spec_language::model::bundle::{
    Bundle, BundleRecord, ComponentRecord, EndpointRecord, ModelSelection, Multiplicity,
    ObjectTypeRecord, OperationEffect, OperationMemberRecord, PortDirection, RelationshipDirection,
    RelationshipEnd, RelationshipRecord, ScalarTypeRecord,
};
use quire_spec_language::model::key::ProducerKey;
use quire_spec_language::model::normalize::ModelRefusal;
use quire_spec_language::model::systems::{
    check_allocation, check_connection, classify, resolve_kind, AllocationCheckOutcome,
    ConnectionCheckOutcome, ConnectionOutcome, Kind,
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

fn object_type(identity: &str, interface_features: Option<Vec<&str>>) -> BundleRecord {
    BundleRecord::ObjectType(ObjectTypeRecord {
        key: ProducerKey::fixture(identity),
        interface_features: interface_features
            .map(|features| features.into_iter().map(ProducerKey::fixture).collect()),
    })
}

fn scalar_type(identity: &str, lower: i64, upper: i64) -> BundleRecord {
    BundleRecord::ScalarType(ScalarTypeRecord {
        key: ProducerKey::fixture(identity),
        lower,
        upper,
    })
}

fn field_member(identity: &str, owner: &str, value_type: &str, m: Multiplicity) -> BundleRecord {
    BundleRecord::FieldMember(quire_spec_language::model::bundle::FieldMemberRecord {
        key: ProducerKey::fixture(identity),
        owner: ProducerKey::fixture(owner),
        value_type: ProducerKey::fixture(value_type),
        multiplicity: m,
    })
}

fn generalization(identity: &str, specific: &str, general: &str) -> BundleRecord {
    BundleRecord::Generalization(quire_spec_language::model::bundle::GeneralizationRecord {
        key: ProducerKey::fixture(identity),
        specific: ProducerKey::fixture(specific),
        general: ProducerKey::fixture(general),
    })
}

fn component(identity: &str, owning_type: &str, value_type: &str, m: Multiplicity) -> BundleRecord {
    BundleRecord::Component(ComponentRecord {
        key: ProducerKey::fixture(identity),
        owning_type: ProducerKey::fixture(owning_type),
        value_type: ProducerKey::fixture(value_type),
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
) -> BundleRecord {
    BundleRecord::Endpoint(EndpointRecord {
        key: ProducerKey::fixture(identity),
        owning_component: ProducerKey::fixture(owning_component),
        value_type: ProducerKey::fixture(value_type),
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
    category: &str,
    direction: RelationshipDirection,
) -> BundleRecord {
    BundleRecord::Relationship(RelationshipRecord {
        key: ProducerKey::fixture(identity),
        source: RelationshipEnd {
            type_identity: ProducerKey::fixture(source),
            multiplicity: source_m,
        },
        target: RelationshipEnd {
            type_identity: ProducerKey::fixture(target),
            multiplicity: target_m,
        },
        category: category.to_owned(),
        direction,
    })
}

fn operation_run() -> BundleRecord {
    BundleRecord::OperationMember(OperationMemberRecord {
        key: ProducerKey::fixture("model.Pump.run"),
        owner: ProducerKey::fixture("model.Pump"),
        parameters: vec![],
        result: None,
        effect: OperationEffect::default(),
        has_own_precondition: false,
        own_postcondition_facts: vec![],
        has_body: true,
    })
}

/// TC-197 fixture Y (bundle `bundle.y`), field-for-field. `mutate` edits the
/// base record set before the bundle is built, so every Y02-Y06 variant is
/// this same fixture with exactly the one documented change.
fn fixture_y(mutate: impl FnOnce(&mut Vec<BundleRecord>)) -> Bundle {
    let mut records = vec![
        scalar_type("model.Count", 0, 9),
        object_type("model.Sys", None),
        object_type("model.Pump", None),
        object_type("model.Tank", None),
        object_type("model.Flow", Some(vec!["model.Flow.rate"])),
        object_type("model.Flow2", Some(vec!["model.Flow.rate"])),
        generalization("model.gen.Flow2-Flow", "model.Flow2", "model.Flow"),
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
            "connection",
            RelationshipDirection::SourceToTarget,
        ),
        operation_run(),
        relationship(
            "model.Pump.alloc",
            "model.Pump.run",
            one(),
            "model.Sys.pump",
            one(),
            "allocation",
            RelationshipDirection::SourceToTarget,
        ),
        relationship(
            "model.rel.parts",
            "model.Sys",
            one(),
            "model.Pump",
            mult(0, Some(3)),
            "composition",
            RelationshipDirection::SourceToTarget,
        ),
        relationship(
            "model.rel.home",
            "model.Pump",
            one(),
            "model.Tank",
            one(),
            "composition",
            RelationshipDirection::SourceToTarget,
        ),
    ];
    mutate(&mut records);
    Bundle::new(ModelSelection::fixture("bundle.y"), records)
}

fn unlimited_meter() -> Meter {
    Meter::new(ModelNormalizationLimits::UNLIMITED)
}

fn find_relationship<'a>(
    records: &'a mut [BundleRecord],
    identity: &str,
) -> &'a mut RelationshipRecord {
    records
        .iter_mut()
        .find_map(|record| match record {
            BundleRecord::Relationship(relationship) if relationship.key.identity == identity => {
                Some(relationship)
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("fixture Y has no relationship {identity}"))
}

fn find_endpoint<'a>(records: &'a mut [BundleRecord], identity: &str) -> &'a mut EndpointRecord {
    records
        .iter_mut()
        .find_map(|record| match record {
            BundleRecord::Endpoint(endpoint) if endpoint.key.identity == identity => Some(endpoint),
            _ => None,
        })
        .unwrap_or_else(|| panic!("fixture Y has no endpoint {identity}"))
}

fn find_component<'a>(records: &'a mut [BundleRecord], identity: &str) -> &'a mut ComponentRecord {
    records
        .iter_mut()
        .find_map(|record| match record {
            BundleRecord::Component(component) if component.key.identity == identity => {
                Some(component)
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("fixture Y has no component {identity}"))
}

#[trace("TC-197", "FR-152-AC-1")]
#[test]
fn y01_every_kind_resolves_to_its_exact_producer_key() {
    let bundle = fixture_y(|_| {});
    let mut meter = unlimited_meter();
    let classification = classify(&bundle, &mut meter).expect("classify admitted");
    assert!(classification.refusals.is_empty());

    for (required, identity) in [
        (Kind::Part, "model.Sys.pump"),
        (Kind::Port, "model.Sys.pump.out"),
        (Kind::Interface, "model.Flow"),
        (Kind::Connection, "model.Sys.pipe"),
        (Kind::Allocation, "model.Pump.alloc"),
    ] {
        let key = ProducerKey::fixture(identity);
        let resolved = resolve_kind(&classification, required, &key).unwrap_or_else(|refusal| {
            panic!("resolve({required:?}, {identity}) refused: {refusal:?}")
        });
        assert_eq!(resolved.kind, required);
        assert_eq!(resolved.key, key);
    }

    match check_connection(
        &bundle,
        &classification,
        &ProducerKey::fixture("model.Sys.pipe"),
        &mut meter,
    ) {
        ConnectionCheckOutcome::Completed(ConnectionOutcome::Admitted) => {}
        other => panic!("expected the connection admitted, got {other:?}"),
    }
    match check_allocation(
        &classification,
        &ProducerKey::fixture("model.Pump.alloc"),
        &mut meter,
    ) {
        AllocationCheckOutcome::Admitted => {}
        other => panic!("expected the allocation admitted, got {other:?}"),
    }
}

#[trace("TC-197", "FR-152-AC-2")]
#[test]
fn y02_wrong_export_substitutions_name_the_required_and_actual_kind() {
    let bundle = fixture_y(|_| {});
    let mut meter = unlimited_meter();
    let classification = classify(&bundle, &mut meter).expect("classify admitted");

    let refusal = resolve_kind(
        &classification,
        Kind::Port,
        &ProducerKey::fixture("model.Sys.pump"),
    )
    .expect_err("model.Sys.pump is a Part, not a Port");
    assert_eq!(refusal.code, Code::InvalidModelBinding);
    assert_eq!(refusal.cause, "wrong-export");
    assert!(refusal.detail.contains("required kind Port"));
    assert!(refusal.detail.contains("actual kind Part"));

    let refusal = resolve_kind(
        &classification,
        Kind::Allocation,
        &ProducerKey::fixture("model.Sys.pipe"),
    )
    .expect_err("model.Sys.pipe is a Connection, not an Allocation");
    assert_eq!(refusal.code, Code::InvalidModelBinding);
    assert_eq!(refusal.cause, "wrong-export");
    assert!(refusal.detail.contains("required kind Allocation"));
    assert!(refusal.detail.contains("actual kind Connection"));

    let bundle = fixture_y(|records| {
        find_relationship(records, "model.Pump.alloc")
            .target
            .type_identity = ProducerKey::fixture("model.Sys.pump.out");
    });
    let classification = classify(&bundle, &mut unlimited_meter()).expect("classify admitted");
    match check_allocation(
        &classification,
        &ProducerKey::fixture("model.Pump.alloc"),
        &mut unlimited_meter(),
    ) {
        AllocationCheckOutcome::Refused(refusal) => {
            assert_eq!(refusal.code, Code::InvalidModelBinding);
            assert_eq!(refusal.cause, "wrong-export");
            assert!(refusal.detail.contains("required kind Part"));
            assert!(refusal.detail.contains("actual kind Port"));
        }
        other => panic!("expected the allocation refused, got {other:?}"),
    }
}

#[trace("TC-197", "FR-152-AC-3")]
#[test]
fn y03a_swapped_connection_ends_refuse_on_port_direction() {
    let bundle = fixture_y(|records| {
        let pipe = find_relationship(records, "model.Sys.pipe");
        pipe.source.type_identity = ProducerKey::fixture("model.Sys.tank.in");
        pipe.target.type_identity = ProducerKey::fixture("model.Sys.pump.out");
    });
    let mut meter = unlimited_meter();
    let classification = classify(&bundle, &mut meter).expect("classify admitted");
    match check_connection(
        &bundle,
        &classification,
        &ProducerKey::fixture("model.Sys.pipe"),
        &mut meter,
    ) {
        ConnectionCheckOutcome::Completed(ConnectionOutcome::Refused(failures)) => {
            assert_eq!(failures.len(), 1);
            assert_eq!(failures[0].condition, "port-direction");
            assert_eq!(failures[0].cause, "port-direction");
        }
        other => panic!("expected a port-direction refusal, got {other:?}"),
    }
}

#[trace("TC-197", "FR-152-AC-3")]
#[test]
fn y03b_target_to_source_against_the_declared_ports_refuses() {
    let bundle = fixture_y(|records| {
        find_relationship(records, "model.Sys.pipe").direction =
            RelationshipDirection::TargetToSource;
    });
    let mut meter = unlimited_meter();
    let classification = classify(&bundle, &mut meter).expect("classify admitted");
    match check_connection(
        &bundle,
        &classification,
        &ProducerKey::fixture("model.Sys.pipe"),
        &mut meter,
    ) {
        ConnectionCheckOutcome::Completed(ConnectionOutcome::Refused(failures)) => {
            assert_eq!(failures[0].condition, "port-direction");
        }
        other => panic!("expected a port-direction refusal, got {other:?}"),
    }
}

#[trace("TC-197", "FR-152-AC-3")]
#[test]
fn y03c_bidirectional_against_non_inout_ports_refuses() {
    let bundle = fixture_y(|records| {
        find_relationship(records, "model.Sys.pipe").direction =
            RelationshipDirection::Bidirectional;
    });
    let mut meter = unlimited_meter();
    let classification = classify(&bundle, &mut meter).expect("classify admitted");
    match check_connection(
        &bundle,
        &classification,
        &ProducerKey::fixture("model.Sys.pipe"),
        &mut meter,
    ) {
        ConnectionCheckOutcome::Completed(ConnectionOutcome::Refused(failures)) => {
            assert_eq!(failures[0].condition, "port-direction");
        }
        other => panic!("expected a port-direction refusal, got {other:?}"),
    }
}

#[trace("TC-197", "FR-152-AC-3")]
#[test]
fn y03d_bidirectional_with_both_ports_inout_admits() {
    let bundle = fixture_y(|records| {
        find_relationship(records, "model.Sys.pipe").direction =
            RelationshipDirection::Bidirectional;
        find_endpoint(records, "model.Sys.pump.out").direction = Some(PortDirection::InOut);
        find_endpoint(records, "model.Sys.tank.in").direction = Some(PortDirection::InOut);
    });
    let mut meter = unlimited_meter();
    let classification = classify(&bundle, &mut meter).expect("classify admitted");
    match check_connection(
        &bundle,
        &classification,
        &ProducerKey::fixture("model.Sys.pipe"),
        &mut meter,
    ) {
        ConnectionCheckOutcome::Completed(ConnectionOutcome::Admitted) => {}
        other => panic!("expected the connection admitted, got {other:?}"),
    }
}

#[trace("TC-197", "FR-152-AC-3")]
#[test]
fn y03e_undirected_is_never_a_connection_direction() {
    let bundle = fixture_y(|records| {
        find_relationship(records, "model.Sys.pipe").direction = RelationshipDirection::Undirected;
    });
    let mut meter = unlimited_meter();
    let classification = classify(&bundle, &mut meter).expect("classify admitted");
    match check_connection(
        &bundle,
        &classification,
        &ProducerKey::fixture("model.Sys.pipe"),
        &mut meter,
    ) {
        ConnectionCheckOutcome::Completed(ConnectionOutcome::Refused(failures)) => {
            assert_eq!(failures[0].condition, "port-direction");
        }
        other => panic!("expected a port-direction refusal, got {other:?}"),
    }
}

#[trace("TC-197", "FR-152-AC-3", "FR-152-AC-4")]
#[test]
fn y04_flow_source_must_conform_to_flow_target_not_the_reverse() {
    // model.Flow2 generalizes to model.Flow (Flow2 is more derived), so
    // retyping the flow TARGET (tank.in) to Flow2 asks whether the
    // unrelated-in-that-direction model.Flow conforms to model.Flow2: it
    // does not.
    let bundle = fixture_y(|records| {
        find_endpoint(records, "model.Sys.tank.in").value_type =
            ProducerKey::fixture("model.Flow2");
    });
    let mut meter = unlimited_meter();
    let classification = classify(&bundle, &mut meter).expect("classify admitted");
    let before = meter.consumed(quire_spec_language::model::accounting::LimitKind::WorkUnits);
    match check_connection(
        &bundle,
        &classification,
        &ProducerKey::fixture("model.Sys.pipe"),
        &mut meter,
    ) {
        ConnectionCheckOutcome::Completed(ConnectionOutcome::Refused(failures)) => {
            let interface_failure = failures
                .iter()
                .find(|f| f.condition == "interface-type")
                .expect("an interface-type failure");
            assert_eq!(interface_failure.code, Code::IllTyped);
            assert_eq!(interface_failure.cause, "type-mismatch");
            assert!(interface_failure.detail.contains("model.Flow"));
            assert!(interface_failure.detail.contains("model.Flow2"));
        }
        other => panic!("expected an interface-type refusal, got {other:?}"),
    }
    let after = meter.consumed(quire_spec_language::model::accounting::LimitKind::WorkUnits);
    // Three systems.connection-condition charges, one flat work unit each
    // (this module's recorded scope choice — see systems.rs's module doc —
    // not FR-152's own f(Flow)/f(Flow2) derivation-fact pricing).
    assert_eq!(after - before, 3);

    // Retyping the flow SOURCE (pump.out) to Flow2 instead: Flow2 conforms
    // to Flow directly, so the connection admits.
    let bundle = fixture_y(|records| {
        find_endpoint(records, "model.Sys.pump.out").value_type =
            ProducerKey::fixture("model.Flow2");
    });
    let mut meter = unlimited_meter();
    let classification = classify(&bundle, &mut meter).expect("classify admitted");
    match check_connection(
        &bundle,
        &classification,
        &ProducerKey::fixture("model.Sys.pipe"),
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

#[trace("TC-197", "FR-152-AC-3")]
#[test]
fn y05_a_narrowed_end_multiplicity_refuses_and_exposes_no_connection() {
    let bundle = fixture_y(|records| {
        find_relationship(records, "model.Sys.pipe")
            .target
            .multiplicity = mult(0, Some(2));
    });
    let mut meter = unlimited_meter();
    let classification = classify(&bundle, &mut meter).expect("classify admitted");
    match check_connection(
        &bundle,
        &classification,
        &ProducerKey::fixture("model.Sys.pipe"),
        &mut meter,
    ) {
        ConnectionCheckOutcome::Completed(ConnectionOutcome::Refused(failures)) => {
            let failure = failures
                .iter()
                .find(|f| f.condition == "multiplicity")
                .expect("a multiplicity failure");
            assert_eq!(failure.code, Code::IllTyped);
            assert_eq!(failure.cause, "multiplicity-narrowing");
        }
        other => panic!("expected a multiplicity refusal (no connection exposed), got {other:?}"),
    }
}

#[trace("TC-197", "FR-152-AC-2", "FR-152-AC-5")]
#[test]
fn y06_removing_the_part_capability_cascades_four_refusals_in_rule_order() {
    let bundle = fixture_y(|records| {
        find_component(records, "model.Sys.pump").has_part_signature = false;
    });
    let mut meter = unlimited_meter();
    let classification = classify(&bundle, &mut meter).expect("classify admitted");

    assert_eq!(classification.refusals.len(), 3);
    let cascade: Vec<(&Code, &str)> = classification
        .refusals
        .iter()
        .map(|r: &ModelRefusal| (&r.code, r.cause))
        .collect();
    assert_eq!(
        cascade,
        vec![
            (&Code::InvalidModelBinding, "unsupplied-producer-record"),
            (&Code::InvalidModelBinding, "wrong-export"),
            (&Code::InvalidModelBinding, "wrong-export"),
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
        &ProducerKey::fixture("model.Pump.alloc"),
        &mut meter,
    ) {
        AllocationCheckOutcome::Refused(refusal) => {
            assert_eq!(refusal.code, Code::InvalidModelBinding);
            assert_eq!(refusal.cause, "wrong-export");
            assert!(refusal.detail.contains("model.Sys.pump"));
            assert!(refusal.detail.contains("required kind Part"));
            assert!(refusal.detail.contains("actual kind none"));
        }
        other => panic!("expected the allocation refused, got {other:?}"),
    }

    let refusal = resolve_kind(
        &classification,
        Kind::Part,
        &ProducerKey::fixture("model.Sys.pump"),
    )
    .expect_err("model.Sys.pump no longer resolves to any kind");
    assert_eq!(refusal.cause, "unsupplied-producer-record");
}
