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
    assert!(rows[1].member_identity.ends_with("[key]"));

    let timers = places.iter().find(|p| p.name() == "t.[key].x").unwrap();
    let rows = timers.rows();
    assert_eq!(rows[0].collection_kind, "map");
    assert_eq!(rows[0].maximum, "unbounded");
}

/// FR-230-AC-62: a map slot's `selector` in a `clock_place` node references
/// the same node as the `key` binding of the map's own type node, built by the
/// one helper: the bare Text node for a `Text` key (never the native `string`
/// node) and the key type's own node for an `Integer` or bounded `Int` key.
#[trace("FR-230-AC-62")]
#[test]
fn a_map_slot_selector_is_the_key_node_of_the_map_type() {
    use super::{key, object, one, DomainPackage, DomainPackageRecord};
    use super::{FieldMemberRecord, NativeValueType, ValueTypeRef};
    use crate::check::family::fixtures::{fixture_source, scope_with};
    use crate::check::family::OccurrenceMap;
    use crate::check::lowering::model::AdmittedModel;
    use crate::check::lowering::{generated_location, Lowering};
    use crate::check::timed_model::clock_fields;
    use crate::model::accounting::ModelNormalizationLimits;
    use crate::model::domain_package::MapTypeRecord;
    use crate::model::normalize::{normalize, NormalizeOutcome};
    use quire_exact::{MapType, Presence, ValueType};
    use quire_semantic_value::declaration::{
        FieldDeclaration, ObjectTypeDeclaration, TypeEnvironment,
    };

    let keyed = [
        ("mt", NativeValueType::Text),
        ("mi", NativeValueType::Integer),
        ("mb", NativeValueType::Boolean),
    ];
    let mut records = vec![object("Call", &[])];
    for (position, (name, native)) in keyed.iter().enumerate() {
        records.push(DomainPackageRecord::MapType(MapTypeRecord {
            key: key(&format!("{name}Type")),
            keys: ValueTypeRef::Native(*native),
            values: ValueTypeRef::Clock,
            lower: 0,
            upper: Some(2),
        }));
        records.push(DomainPackageRecord::FieldMember(FieldMemberRecord {
            key: key(&format!("Call/{name}")),
            owner: key("Call"),
            value_type: ValueTypeRef::Package(key(&format!("{name}Type"))),
            multiplicity: one(),
            presence: Presence::Required,
            subsets: Vec::new(),
            redefines: None,
            position,
        }));
    }
    let package = DomainPackage::new(
        super::DomainPackageRef {
            identity: super::PACKAGE.to_owned(),
        },
        records,
    );
    let NormalizeOutcome::Completed(view) =
        normalize(&package, ModelNormalizationLimits::UNLIMITED)
    else {
        panic!("the package normalizes");
    };
    let model = AdmittedModel::new(&package, &view).expect("the view is the package's own");
    let call = view.type_identities()[&key("Call")].clone();
    let map_of = |keys: ValueType| {
        ValueType::Map(std::sync::Arc::new(
            MapType::new(keys, ValueType::Integer, None).expect("a leaf key"),
        ))
    };
    let types = TypeEnvironment::new(
        [],
        [ObjectTypeDeclaration::new(
            call,
            "M::Call",
            vec![
                FieldDeclaration::new("mt", map_of(ValueType::String), Presence::Required),
                FieldDeclaration::new("mi", map_of(ValueType::Integer), Presence::Required),
                FieldDeclaration::new("mb", map_of(ValueType::Boolean), Presence::Required),
            ],
        )],
    )
    .expect("the object type admits");
    let scope = scope_with(types, Vec::new());
    let fields: Vec<_> = clock_fields(&model);
    assert_eq!(fields.len(), 3);

    let owner = crate::check::SourceOwner::from(&fixture_source());
    let mut occurrences = OccurrenceMap::default();
    let mut meter = quire_exact::Meter::new(crate::check::family::SCALAR_LIMITS_UNLIMITED);
    let models = [model];
    let location = generated_location();
    let mut lowering = Lowering::new_with_source_owners(
        &scope,
        &owner,
        &std::collections::BTreeMap::new(),
        &models,
        &[],
        Default::default(),
        None,
        0,
        &mut occurrences,
        &mut meter,
        "fixture",
    );
    let model_node = lowering
        .model_node(&key("Call"), &location)
        .expect("the object keys");
    let places = lowering
        .clock_places(&fields, model_node, &location)
        .expect("the places lower");
    let expected = |lowering: &mut Lowering<'_>, keys: &ValueType| {
        lowering
            .map_key_node(keys, &location)
            .expect("the key node")
            .admitted()
            .cloned()
            .expect("an admitted key node")
            .to_string()
    };
    let text = expected(&mut lowering, &ValueType::String);
    let integer = expected(&mut lowering, &ValueType::Integer);
    let boolean = expected(&mut lowering, &ValueType::Boolean);
    let native_string = lowering
        .type_node(&ValueType::String, &location)
        .expect("the native string node")
        .to_string();
    assert_ne!(text, native_string, "Text is not the native string node");
    let graph = lowering.finish(&location).expect("lowering finishes").graph;
    // Each place's field name and the digest its slot step's selector
    // references.
    let member = |node: &serde_json::Value, name: &str| -> serde_json::Value {
        node["members"]
            .as_array()
            .and_then(|members| members.iter().find(|m| m["name"] == name))
            .map(|m| m["value"].clone())
            .unwrap_or_default()
    };
    let mut selectors = std::collections::BTreeMap::new();
    for place in &places {
        let preimage = crate::check::lowering::tests::view::preimage(
            graph.node(place.clone()).expect("a place node"),
        );
        let path = member(&preimage["body"], "path");
        let steps = path["members"].as_array().expect("path steps");
        let field = member(&steps[0], "name")["value"]
            .as_str()
            .expect("a field name")
            .to_owned();
        let selector = member(&steps[1], "selector")["target"]["digest"]
            .as_str()
            .expect("a selector reference")
            .to_owned();
        selectors.insert(field, selector);
    }
    assert_eq!(selectors["mt"], text, "a Text key selects by the Text node");
    assert_ne!(selectors["mt"], native_string);
    assert_eq!(selectors["mi"], integer);
    assert_eq!(selectors["mb"], boolean);
}
