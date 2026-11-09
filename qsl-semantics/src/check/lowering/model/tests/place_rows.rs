// SPDX-License-Identifier: AGPL-3.0-or-later
//! The element type a clock place's slot row is declared by (QSpec FR-322
//! "Timed-model nodes").

use super::super::element_type;
use ix_trace_rs::trace;

/// SR-1746002 FND-002: a slot row below an `Option<..>` collection, QSL
/// FR-230-AC-19's `o: Option<Sequence<Clock>[0, 2]>`, is declared by the
/// collection's element type, read through the `Option`; a collection and
/// a plain type read as before.
#[test]
fn the_element_type_reads_through_an_option() {
    let clock = "ix://quire/native/Clock";
    assert_eq!(element_type(&format!("Option<Sequence<{clock}>>")), clock);
    assert_eq!(element_type(&format!("Set<{clock}>")), clock);
    assert_eq!(element_type(clock), clock);
}

/// FR-230-AC-19 and FR-230-AC-62: a field typed by a package map type whose
/// values are `Clock` lists one place `Call.m.[key]`, a map slot selected by
/// the entry's key, with the map's own bound, the `map` collection kind and
/// the key type's builtin scalar form; the places below a map of records are
/// listed under the key as under a slot.
#[trace("FR-230-AC-19", "FR-230-AC-62")]
#[test]
fn a_package_map_of_clocks_lists_a_key_selected_place() {
    use super::super::super::PlaceStep;
    use super::{key, object, one, DomainPackage, DomainPackageRecord};
    use super::{FieldMemberRecord, NativeValueType, ValueTypeRef};
    use crate::check::lowering::model::AdmittedModel;
    use crate::model::accounting::ModelNormalizationLimits;
    use crate::model::domain_package::AliasTypeRecord;
    use crate::model::domain_package::{MapTypeRecord, RecordValueTypeRecord};
    use crate::model::normalize::{normalize, NormalizeOutcome};
    use quire_exact::Presence;

    let package = DomainPackage::new(
        super::DomainPackageRef {
            identity: super::PACKAGE.to_owned(),
        },
        vec![
            object("Call", &[]),
            DomainPackageRecord::MapType(MapTypeRecord {
                key: key("Ticks"),
                keys: ValueTypeRef::Native(NativeValueType::Text),
                values: ValueTypeRef::Clock,
                lower: 0,
                upper: Some(2),
            }),
            DomainPackageRecord::FieldMember(FieldMemberRecord {
                key: key("Call/m"),
                owner: key("Call"),
                value_type: ValueTypeRef::Package(key("Ticks")),
                multiplicity: one(),
                presence: Presence::Required,
                subsets: Vec::new(),
                redefines: None,
                position: 0,
            }),
            DomainPackageRecord::RecordValueType(RecordValueTypeRecord {
                key: key("Timer"),
                identity_fields: Vec::new(),
                supertypes: Vec::new(),
            }),
            DomainPackageRecord::FieldMember(FieldMemberRecord {
                key: key("Timer/x"),
                owner: key("Timer"),
                value_type: ValueTypeRef::Clock,
                multiplicity: one(),
                presence: Presence::Required,
                subsets: Vec::new(),
                redefines: None,
                position: 0,
            }),
            DomainPackageRecord::AliasType(AliasTypeRecord {
                key: key("TimerAlias"),
                target: ValueTypeRef::Package(key("Timer")),
            }),
            DomainPackageRecord::MapType(MapTypeRecord {
                key: key("Timers"),
                keys: ValueTypeRef::Native(NativeValueType::Integer),
                values: ValueTypeRef::Package(key("Timer")),
                lower: 1,
                upper: None,
            }),
            DomainPackageRecord::FieldMember(FieldMemberRecord {
                key: key("Call/t"),
                owner: key("Call"),
                value_type: ValueTypeRef::Package(key("Timers")),
                multiplicity: one(),
                presence: Presence::Required,
                subsets: Vec::new(),
                redefines: None,
                position: 1,
            }),
        ],
    );
    let NormalizeOutcome::Completed(view) =
        normalize(&package, ModelNormalizationLimits::UNLIMITED)
    else {
        panic!("the package normalizes");
    };
    let model = AdmittedModel::new(&package, &view).expect("the view is the package's own");
    let places = model.clock_places();
    let named: Vec<String> = places.iter().map(|place| place.name()).collect();
    assert!(named.contains(&"m.[key]".to_owned()), "{named:?}");
    assert!(named.contains(&"t.[key].x".to_owned()), "{named:?}");

    let ticks = places.iter().find(|p| p.name() == "m.[key]").unwrap();
    let PlaceStep::Slot(shape) = &ticks.path[1] else {
        panic!("a map slot step");
    };
    assert!(shape.map && shape.unique && !shape.ordered);
    assert_eq!((shape.min, shape.max), (0, Some(2)));
    let rows = ticks.rows();
    assert_eq!(rows[0].collection_kind, "map");
    assert_eq!(
        (rows[0].minimum.as_str(), rows[0].maximum.as_str()),
        ("0", "2")
    );
    assert_eq!(rows[1].key_form, Some("text"));
    assert!(rows[1].member_identity.ends_with("[key]"));

    let timers = places.iter().find(|p| p.name() == "t.[key].x").unwrap();
    let rows = timers.rows();
    assert_eq!(rows[0].collection_kind, "map");
    assert_eq!(rows[0].maximum, "unbounded");
    assert_eq!(rows[1].key_form, Some("integer"));
}
