// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-533, TC-1333: the protocol-slot call of a package read from its bytes
//! alone returns the in-process route's value and charge log.

use super::tc1070::model;
use super::*;
use qsl_eval::bare::BarePackage;
use qsl_package::EmitLimits;
use quire_eval::{CallAdmissionLocus, CallFailure};

/// The bytes-route descriptor of the slot the source route selects with
/// `wanted`: the source selector reads a compensation node's slots from the
/// template at the node's position in the id-sorted keys, so the bytes
/// descriptor names the node whose declaration is the owner.
fn bytes_slot(
    v2: &qsl_package::CheckedPackageV2,
    protocol: &str,
    owner: &str,
    wanted: &ProtocolSlot,
) -> ProtocolSlot {
    let node = if protocol == owner {
        wanted.node.clone()
    } else {
        let declared = v2
            .graph()
            .nodes
            .iter()
            .find(|node| {
                node.declaration.as_ref().is_some_and(|d| {
                    d.qualified_name.last().map(|s| &**s) == Some(owner)
                        && d.qualified_name.first().map(|s| &**s) == Some(protocol)
                }) && &*node.node_tag == "protocol"
            })
            .expect("the owner is declared");
        if &*declared.semantic_form == "compensation" {
            qsl_semantics::check::wire_id::WireGraph::new(
                v2.graph().nodes.iter().map(|node| &node.node_id),
            )
            .expect("wire ids")
            .key(&declared.node_id)
            .expect("key")
        } else {
            wanted.node.clone()
        }
    };
    ProtocolSlot {
        node,
        member: wanted.member.clone(),
        index: wanted.index,
    }
}

/// The identical record value must belong to both environments without
/// rebuilding its slots by display name.
fn in_bytes_environment(
    value: &Value,
    source: &quire_semantic_value::declaration::TypeEnvironment,
    bytes: &quire_semantic_value::declaration::TypeEnvironment,
) -> Value {
    let Value::Composite(composite) = value else {
        return value.clone();
    };
    let key = composite.declaration().clone();
    let Some(quire_semantic_value::declaration::CompositeShape::Record(fields)) =
        source.composite(key.clone()).map(|record| record.shape())
    else {
        panic!("the fixture value is a record");
    };
    let Some(quire_semantic_value::declaration::CompositeShape::Record(byte_fields)) =
        bytes.composite(key).map(|record| record.shape())
    else {
        panic!("the bytes environment holds the same record");
    };
    assert_eq!(fields, byte_fields, "the canonical field order is shared");
    value.clone()
}

/// FR-533-AC-16 and FR-531-AC-1 (evaluate halves), and the source-route
/// halves of AC-1 and AC-2: over every TC-1070 slot
/// that reads no observation, the bare route returns the source route's
/// value, outcome and ordered charge log, at the exact budget, one short of
/// each used counter, and with every binder unavailable; the integer
/// equality of `Refund`'s retry relation and recovery predicate is among
/// them and reads the equality rows of the handoff.
#[trace(
    "TC-1333",
    "FR-533-AC-1",
    "FR-533-AC-2",
    "FR-533-AC-8",
    "FR-533-AC-16",
    "FR-531-AC-1"
)]
#[test]
fn the_bare_route_returns_the_source_value_and_charge_log_for_every_unobserved_slot() {
    let package = fixture();
    let read = read_checked_package(&package);
    let v2 = read.admitted();
    let rows = bare_rows(&package, v2);
    let equality = qsl_package::emit_checked(&package, EmitLimits::fixture())
        .expect("emits")
        .equality_witness(v2);
    let bare = BarePackage::new(v2)
        .with_supply(
            qsl_package::BareSupply {
                identities: &rows,
                equality: Some(&equality),
            },
            &mut qsl_package::V2ReadLimits::fixture().native_join_budget(),
        )
        .expect("the fixture budget admits the supply");
    let world = bare.world().expect("world");
    let reader_view = read
        .evaluation_view()
        .expect("the reader-backed view projects");
    let mut compared = Vec::new();
    let mut unavailable_locations = 0;
    for spec in parity_slots(&package).iter().filter(|spec| !spec.observed) {
        let wanted = slot(&package, spec.protocol, spec.owner, spec.member, spec.index);
        let on_bytes = bytes_slot(v2, spec.protocol, spec.owner, &wanted);
        let values = (spec.values)(None);
        let given = supply(&package, spec.protocol, values.clone(), None);
        let bytes_values = values
            .iter()
            .map(|(name, value)| {
                (
                    *name,
                    in_bytes_environment(value, package.graph().scope().types(), world.types()),
                )
            })
            .collect();
        let bytes_given = supply(&package, spec.protocol, bytes_values, None);
        let mut source_meter = Meter::new(UNLIMITED);
        let source = package.evaluate_protocol_slot(&wanted, &given, &mut source_meter);
        let mut bare_meter = Meter::new(UNLIMITED);
        let bytes = bare.evaluate_protocol_slot(&on_bytes, &bytes_given, &mut bare_meter);
        let descriptor = qsl_package::ProtocolSlotDescriptor::new(
            wanted.node.clone(),
            wanted.member.clone(),
            wanted.index,
        );
        let reader_selected = reader_view
            .select_protocol_slot(&descriptor)
            .expect("the reader selects the same slot");
        let mut reader_meter = Meter::new(UNLIMITED);
        let reader = package.evaluate_selected_protocol_slot(
            &reader_view,
            &reader_selected,
            &given,
            &mut reader_meter,
        );
        assert!(
            matches!(source, Ok(ProtocolSlotEvaluation::Evaluated(..))),
            "{}: source {source:?}",
            spec.name
        );
        assert_eq!(
            without_location(&source),
            without_location(&bytes),
            "{}: value, outcome and ordered charge log",
            spec.name
        );
        charges_and_usage_match(spec.name, &source_meter, &bare_meter);
        assert_eq!(
            without_location(&source),
            without_location(&reader),
            "{}: reader-backed result",
            spec.name
        );
        charges_and_usage_match(spec.name, &source_meter, &reader_meter);
        if let Some(expected) = &spec.expected {
            assert_value_eq!(completed(bytes.unwrap()), *expected, "bare slot value");
        }
        if !values.is_empty() {
            let mut wrong_values = values.clone();
            wrong_values[0].1 = if matches!(wrong_values[0].1, Value::Boolean(_)) {
                int(0)
            } else {
                Value::Boolean(false)
            };
            let wrong = supply(&package, spec.protocol, wrong_values, None);
            let mut source_meter = Meter::new(UNLIMITED);
            let source = package.evaluate_protocol_slot(&wanted, &wrong, &mut source_meter);
            let mut bare_meter = Meter::new(UNLIMITED);
            let bytes = bare.evaluate_protocol_slot(&on_bytes, &wrong, &mut bare_meter);
            let mut reader_meter = Meter::new(UNLIMITED);
            let reader = package.evaluate_selected_protocol_slot(
                &reader_view,
                &reader_selected,
                &wrong,
                &mut reader_meter,
            );
            assert!(
                matches!(source, Err(CallFailure::Admission(_))),
                "{}: wrong value refuses",
                spec.name
            );
            assert_eq!(without_location(&source), without_location(&bytes));
            assert_eq!(without_location(&source), without_location(&reader));
            assert_pristine(&source_meter);
            assert_pristine(&bare_meter);
            assert_pristine(&reader_meter);
        }
        // AC-2: the exact budget and one short of each used counter.
        let exact = limits_at(&source_meter);
        let mut exact_source = Meter::new(exact);
        let mut exact_bare = Meter::new(exact);
        assert_eq!(
            without_location(&package.evaluate_protocol_slot(&wanted, &given, &mut exact_source)),
            without_location(&bare.evaluate_protocol_slot(
                &on_bytes,
                &bytes_given,
                &mut exact_bare
            )),
            "{}: exact budget",
            spec.name
        );
        for counter in LimitKind::ALL {
            let used = source_meter.consumed(counter);
            if used == 0 {
                continue;
            }
            let mut limits = exact;
            set_limit(&mut limits, counter, used - 1);
            let mut short_source = Meter::new(limits);
            let mut short_bare = Meter::new(limits);
            let on_source = package.evaluate_protocol_slot(&wanted, &given, &mut short_source);
            let on_bare = bare.evaluate_protocol_slot(&on_bytes, &bytes_given, &mut short_bare);
            assert_eq!(
                without_location(&on_source),
                without_location(&on_bare),
                "{}: one short at {counter:?}",
                spec.name
            );
            assert_incomplete(on_bare.unwrap(), counter);
            charges_and_usage_match(spec.name, &short_source, &short_bare);
        }
        // An unavailable binder stops the read with the same binder, cause
        // and earlier charges.
        if !values.is_empty() {
            let unavailable = ProtocolSupply {
                binders: values
                    .iter()
                    .map(|(name, _)| {
                        (
                            binder(&package, spec.protocol, name),
                            SuppliedValue::Unavailable(CatalogCode::new(
                                "unavailable_observation",
                                "missing-required-artifact",
                            ))
                            .into(),
                        )
                    })
                    .collect(),
                observation: None,
            };
            let mut source_meter = Meter::new(UNLIMITED);
            let mut bare_meter = Meter::new(UNLIMITED);
            let on_source =
                package.evaluate_protocol_slot(&wanted, &unavailable, &mut source_meter);
            let on_bare = bare.evaluate_protocol_slot(&on_bytes, &unavailable, &mut bare_meter);
            let mut reader_meter = Meter::new(UNLIMITED);
            let on_reader = package.evaluate_selected_protocol_slot(
                &reader_view,
                &reader_selected,
                &unavailable,
                &mut reader_meter,
            );
            assert!(matches!(
                on_bare,
                Ok(ProtocolSlotEvaluation::Unavailable { .. })
            ));
            assert_eq!(without_location(&on_source), without_location(&on_bare));
            assert_eq!(without_location(&on_source), without_location(&on_reader));
            charges_and_usage_match(spec.name, &source_meter, &bare_meter);
            charges_and_usage_match(spec.name, &source_meter, &reader_meter);
            let (
                Ok(ProtocolSlotEvaluation::Unavailable {
                    location: source_location,
                    ..
                }),
                Ok(ProtocolSlotEvaluation::Unavailable {
                    location: bytes_location,
                    ..
                }),
                Ok(ProtocolSlotEvaluation::Unavailable {
                    location: reader_location,
                    ..
                }),
            ) = (&on_source, &on_bare, &on_reader)
            else {
                panic!("all three routes must halt at the unavailable read");
            };
            assert_eq!(
                bytes_location, reader_location,
                "{}: locations come from the same admitted rows",
                spec.name
            );
            if let Some(location) = bytes_location {
                assert_eq!(
                    source_location.as_ref(),
                    Some(location),
                    "{}: wire-carried location",
                    spec.name
                );
            } else {
                assert!(
                    source_location.is_some(),
                    "{}: source location is the documented missing-wire parity",
                    spec.name
                );
            }
            unavailable_locations += 1;
        }
        compared.push((spec.owner, spec.member));
    }
    assert!(compared.len() >= 10, "{compared:?}");
    assert!(
        unavailable_locations > 0,
        "location assertions must exercise real reads"
    );
    assert!(
        compared.contains(&("Refund", "retry.relation")),
        "the integer equality slot is compared: {compared:?}"
    );
}

/// TC-1070's model with a record `Bag` whose members are a plain Collection
/// (`items`), a plain Option (`one`) and an Option of a Collection (`maybe`),
/// all of `Tally`, held by `Receipt.bag`.
fn document_bag() -> serde_json::Value {
    use serde_json::json;
    let mut doc = document();
    let field = |name: &str, optional: bool, many: bool| {
        let mut field = model::field("Bag", name, &model::id("Tally"));
        if optional {
            field["presence"] = json!("optional");
        }
        if many {
            field["multiplicity"] =
                json!({"lower": 0, "upper": 3, "ordered": true, "unique": false});
        }
        field
    };
    let mut options = field("options", false, true);
    options["typeRef"] = json!(model::id("MaybeTally"));
    let bag = model::declaration(
        "Bag",
        "record",
        json!({"fields": [field("items", false, true), field("one", true, false), field("maybe", true, true), options]}),
    );
    let types = doc["types"].as_array_mut().unwrap();
    let receipt = types
        .iter_mut()
        .find(|ty| ty["displayName"] == "Receipt")
        .unwrap();
    receipt["fields"].as_array_mut().unwrap().push(model::field(
        "Receipt",
        "bag",
        &model::id("Bag"),
    ));
    types.push(bag);
    types.push(json!({
        "identity": model::id("MaybeTally"), "displayName": "MaybeTally", "kind": "option", "inner": model::id("Tally"),
        "origin": model::origin("MaybeTally"), "roles": [], "constraints": [], "extensions": [], "unknownPolicy": "reject"
    }));
    doc
}

/// FR-533-AC-7 (projection half), the measurement behind IR-1324: with a
/// `Receipt` whose `Bag` holds a plain Collection member, a plain Option
/// member and an Option of a Collection, the bytes route selects the slot
/// that reads the bag, the binder's value type equals the source route's,
/// and the world declares `Bag` with the members' presence. None of the
/// three member shapes reports `MissingTypeRepresentation`.
#[trace("FR-533-AC-7", "TC-1333")]
#[test]
fn records_with_option_and_collection_members_project_on_the_bytes_route() {
    let source = SOURCE.replace(
        "capture target: M::Total = forward.total;",
        "capture target: M::Total = forward.total;\n    \
         capture held: M::Bag = forward.bag;",
    );
    let model_bytes = serde_json::to_vec(&document_bag()).unwrap();
    let (package, _) = fixture_from_document(&source, &model_bytes);
    let read = read_checked_package_with_model_document(&package, &model_bytes);
    let v2 = read.admitted();
    let rows = bare_rows(&package, v2);
    let equality = qsl_package::emit_checked(&package, EmitLimits::fixture())
        .expect("emits")
        .equality_witness(v2);
    let bare = BarePackage::new(v2)
        .with_supply(
            qsl_package::BareSupply {
                identities: &rows,
                equality: Some(&equality),
            },
            &mut qsl_package::V2ReadLimits::fixture().native_join_budget(),
        )
        .expect("the fixture budget admits the supply");
    let wanted = slot(
        &package,
        "Refunds",
        "Refund",
        "registration_captures",
        Some(1),
    );
    let on_bytes = bytes_slot(v2, "Refunds", "Refund", &wanted);
    let selected = bare.select_protocol_slot(&qsl_package::ProtocolSlotDescriptor::new(
        on_bytes.node.clone(),
        on_bytes.member.clone(),
        on_bytes.index,
    ));
    let selected = selected.expect("the bag slot selects over the package's own nodes");
    let source_view = package.evaluation_view();
    let source_selected = source_view
        .select_protocol_slot(&qsl_package::ProtocolSlotDescriptor::new(
            wanted.node.clone(),
            wanted.member.clone(),
            wanted.index,
        ))
        .expect("source selects");
    let types = |sources: &[qsl_package::BareSource]| {
        sources
            .iter()
            .map(|source| source.value_type().clone())
            .collect::<Vec<_>>()
    };
    assert_eq!(
        types(selected.sources()),
        source_selected
            .sources()
            .iter()
            .map(|source| source.value_type().clone())
            .collect::<Vec<_>>()
    );
    let world = bare.world().expect("world");
    let bag = world
        .types()
        .composites()
        .find(|record| record.name().ends_with("Bag"))
        .expect("the bag record is declared from the package");
    let quire_semantic_value::declaration::CompositeShape::Record(fields) = bag.shape() else {
        panic!("a record");
    };
    let kinds: Vec<_> = fields
        .iter()
        .map(|field| (field.name().to_owned(), field.presence()))
        .collect();
    assert_eq!(
        kinds,
        [
            ("items".to_owned(), quire_exact::Presence::Required),
            ("maybe".to_owned(), quire_exact::Presence::Optional),
            ("one".to_owned(), quire_exact::Presence::Optional),
            ("options".to_owned(), quire_exact::Presence::Required),
        ]
    );
    bare.project_protocol_slot(&selected)
        .map(|_| ())
        .expect("the slot's expression projects");
    // The same slot evaluates on both routes to the bag, with equal
    // charges (IR-1324 AC3 and FR-533-AC-7's evaluating half for these
    // member shapes).
    let bag_in = |types: &quire_semantic_value::declaration::TypeEnvironment| {
        let tally = ValueType::Int(
            quire_exact::IntegerInterval::new(0_i64.into(), 3_i64.into()).expect("interval"),
        );
        let items = quire_exact::from_admitted(
            CollectionType::new(
                CollectionKind::Sequence,
                tally.clone(),
                Some(CardinalityBound::new(0, 3).expect("bound")),
            ),
            vec![int(1), int(2)],
        );
        let maybe = quire_exact::from_admitted(
            CollectionType::new(
                CollectionKind::Sequence,
                tally.clone(),
                Some(CardinalityBound::new(0, 3).expect("bound")),
            ),
            vec![int(3)],
        );
        let options = quire_exact::from_admitted(
            CollectionType::new(
                CollectionKind::Sequence,
                ValueType::Option(std::sync::Arc::new(tally.clone())),
                Some(CardinalityBound::new(0, 3).expect("bound")),
            ),
            vec![
                quire_exact::OptionValue::present(tally.clone(), int(1)).expect("some tally"),
                quire_exact::OptionValue::none(tally),
            ],
        );
        let find = |name: &str| {
            types
                .composites()
                .find(|record| record.name().ends_with(name))
                .expect("declared")
                .key()
        };
        let bag = types
            .record(
                find("Bag"),
                vec![
                    ("items", FieldValue::Present(items)),
                    ("one", FieldValue::Present(int(2))),
                    ("maybe", FieldValue::Present(maybe)),
                    ("options", FieldValue::Present(options)),
                ],
            )
            .expect("bag");
        let receipt = types
            .record(
                find("Receipt"),
                vec![
                    ("total", FieldValue::Present(int(5))),
                    ("bag", FieldValue::Present(bag.clone())),
                ],
            )
            .expect("receipt");
        (receipt, bag)
    };
    let (source_receipt, source_bag) = bag_in(package.graph().scope().types());
    let (bytes_receipt, bytes_bag) = bag_in(world.types());
    assert_value_eq!(
        source_receipt,
        bytes_receipt,
        "the identical receipt works in both environments"
    );
    assert_value_eq!(
        source_bag,
        bytes_bag,
        "the identical nested bag works in both environments"
    );
    let mut source_meter = Meter::new(UNLIMITED);
    let on_source = package.evaluate_protocol_slot(
        &wanted,
        &supply(
            &package,
            "Refunds",
            vec![("forward", source_receipt.clone())],
            None,
        ),
        &mut source_meter,
    );
    let mut bytes_meter = Meter::new(UNLIMITED);
    let on_bare = bare.evaluate_protocol_slot(
        &on_bytes,
        &supply(&package, "Refunds", vec![("forward", source_receipt)], None),
        &mut bytes_meter,
    );
    assert_eq!(without_location(&on_source), without_location(&on_bare));
    charges_and_usage_match("bag capture", &source_meter, &bytes_meter);
    let (Ok(ProtocolSlotEvaluation::Evaluated(..)), Ok(ProtocolSlotEvaluation::Evaluated(..))) =
        (&on_source, &on_bare)
    else {
        panic!("both routes evaluate: {on_source:?} {on_bare:?}");
    };
    assert_value_eq!(completed(on_source.unwrap()), source_bag, "source bag");
    assert_value_eq!(completed(on_bare.unwrap()), bytes_bag, "bytes bag");
}

#[trace("TC-1333", "FR-533-AC-7")]
#[test]
fn original_model_field_keys_define_one_value_across_both_environments() {
    let mut document = document();
    let receipt = document["types"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|ty| ty["displayName"] == "Receipt")
        .expect("Receipt");
    let zebra = model::field("Receipt", "zebra", "ix://quire/native/Boolean");
    let alpha = model::field("Receipt", "alpha", &model::id("Total"));
    receipt["fields"]
        .as_array_mut()
        .unwrap()
        .extend([zebra, alpha]);
    let document = serde_json::to_vec(&document).expect("model bytes");
    let source = SOURCE.replace(
        "capture target: M::Total = forward.total;",
        "capture target: M::Total = forward.total;\n    capture held: M::Receipt = forward;",
    );
    let package = fixture_from_document(&source, &document).0;
    let read = read_checked_package_with_model_document(&package, &document);
    let v2 = read.admitted();
    let rows = bare_rows(&package, v2);
    let equality = qsl_package::emit_checked(&package, EmitLimits::fixture())
        .expect("emits")
        .equality_witness(v2);
    let bare = BarePackage::new(v2)
        .with_supply(
            qsl_package::BareSupply {
                identities: &rows,
                equality: Some(&equality),
            },
            &mut qsl_package::V2ReadLimits::fixture().native_join_budget(),
        )
        .expect("the fixture budget admits the supply");
    let world = bare.world().expect("world");
    let value = record(
        &package,
        "Receipt",
        vec![
            ("total", int(5)),
            ("zebra", Value::Boolean(true)),
            ("alpha", int(7)),
        ],
    );
    let Value::Composite(composite) = &value else {
        panic!("record value")
    };
    let key = composite.declaration();
    for types in [package.graph().scope().types(), world.types()] {
        let quire_semantic_value::declaration::CompositeShape::Record(fields) = types
            .composite(key.clone())
            .expect("Receipt in both environments")
            .shape()
        else {
            panic!("Receipt is a record")
        };
        assert_eq!(
            fields.iter().map(|field| field.name()).collect::<Vec<_>>(),
            ["alpha", "total", "zebra"],
            "original-key order opposes the authored field order"
        );
        assert!(types
            .admits(&ValueType::Composite(key.clone()), &value)
            .expect("membership"));
    }
    let wanted = slot(
        &package,
        "Refunds",
        "Refund",
        "registration_captures",
        Some(1),
    );
    let on_bytes = bytes_slot(v2, "Refunds", "Refund", &wanted);
    let given = supply(&package, "Refunds", vec![("forward", value.clone())], None);
    let mut source_meter = Meter::new(UNLIMITED);
    let source = package.evaluate_protocol_slot(&wanted, &given, &mut source_meter);
    let mut bytes_meter = Meter::new(UNLIMITED);
    let bytes = bare.evaluate_protocol_slot(&on_bytes, &given, &mut bytes_meter);
    assert_eq!(without_location(&source), without_location(&bytes));
    assert_value_eq!(
        completed(source.unwrap()),
        value,
        "source retains the unchanged record"
    );
    assert_value_eq!(
        completed(bytes.unwrap()),
        value,
        "bytes retain the unchanged record"
    );
    charges_and_usage_match("original model field order", &source_meter, &bytes_meter);
}

/// The strict read of `package` under the library name it was compiled as.
fn read_named<'p>(package: &'p CheckedPackage, name: &str) -> qsl_package::AdmittedPackageRead<'p> {
    let bytes = serde_json::to_vec(&document()).unwrap();
    let domain_packages = qsl_semantics::model::intake::package_input([bytes.as_slice()]).unwrap();
    qsl_package::read_evaluation_package(
        package,
        qsl_semantics::library::LibraryName::new(name).unwrap(),
        &domain_packages,
        qsl_package::V2ReadLimits::fixture(),
        EmitLimits::fixture(),
    )
    .expect("strict I2 admission accepts the package")
}

/// The code, cause, `reason` field and locus of a refused call.
fn refusal_of(
    result: Result<ProtocolSlotEvaluation, CallFailure>,
) -> (String, String, Option<String>, CallAdmissionLocus) {
    let Err(CallFailure::Admission(failure)) = result else {
        panic!("the call is refused at admission: {result:?}");
    };
    let quire_eval::CallAdmissionFailure::Refused(record) = *failure else {
        panic!("a refusal, not an incomplete result");
    };
    (
        record.record.catalog.code().to_owned(),
        record.record.catalog.cause().to_owned(),
        record.record.fields.get("reason").cloned(),
        record.locus,
    )
}

fn assert_pristine(meter: &Meter) {
    for counter in LimitKind::ALL {
        assert_eq!(meter.consumed(counter), 0, "{counter:?}");
    }
    assert!(meter.admitted_charges().is_empty());
}

/// FR-533-AC-15, FR-390-AC-66, FR-531-AC-5 (evaluate half): a missing,
/// foreign and unknown identity witness, and the same three for the equality
/// rows, are each `unknown_required_feature`/`unsupported-feature` at
/// `CallAdmissionLocus::Package`, never `Node` or `Binder`; a missing node is
/// `Node`, a missing member `Member`; each leaves the meter untouched.
/// This is the bytes-route half of FR-390-AC-66. The in-process reader view's
/// half, the same three identity witness reasons as the `Package` locus of
/// `UnsupportedView`, is asserted by the qsl-package test
/// `supplied_identities_are_validated_against_the_admitted_package` (the
/// equality-witness reasons have no producer on the reader view, which reads
/// its equality from the emission).
#[trace("TC-1333", "TC-1331", "FR-533-AC-15", "FR-531-AC-5", "FR-390-AC-66")]
#[test]
fn every_witness_refusal_of_the_call_is_at_the_package_locus() {
    let package = fixture();
    let read = read_checked_package(&package);
    let v2 = read.admitted();
    let rows = bare_rows(&package, v2);
    let equality = qsl_package::emit_checked(&package, EmitLimits::fixture())
        .expect("emits")
        .equality_witness(v2);
    let good = BarePackage::new(v2)
        .with_supply(
            qsl_package::BareSupply {
                identities: &rows,
                equality: Some(&equality),
            },
            &mut qsl_package::V2ReadLimits::fixture().native_join_budget(),
        )
        .expect("the fixture budget admits the supply");
    let world = good.world().expect("world");
    let wanted = slot(&package, "Refunds", "Refund", "retry.relation", None);
    let on_bytes = bytes_slot(v2, "Refunds", "Refund", &wanted);
    let values = [
        ("earlier", attempt(&package, 0)),
        ("later", attempt(&package, 1)),
        ("target", int(5)),
        ("activated", Value::Boolean(false)),
    ];
    let given = supply(
        &package,
        "Refunds",
        values
            .iter()
            .map(|(name, value)| {
                (
                    *name,
                    in_bytes_environment(value, package.graph().scope().types(), world.types()),
                )
            })
            .collect(),
        None,
    );
    let call = |bare: &BarePackage<'_>, descriptor: &ProtocolSlot| {
        let mut meter = Meter::new(UNLIMITED);
        let result = bare.evaluate_protocol_slot(descriptor, &given, &mut meter);
        assert_pristine(&meter);
        refusal_of(result)
    };
    let package_refusal = |reason: &str| {
        (
            "unknown_required_feature".to_owned(),
            "unsupported-feature".to_owned(),
            Some(reason.to_owned()),
            CallAdmissionLocus::Package,
        )
    };
    // Identity witnesses.
    assert_eq!(
        call(&BarePackage::new(v2), &on_bytes),
        package_refusal("missing-identity-witness")
    );
    let (other, _) = fixture_named(SOURCE, &serde_json::to_vec(&document()).unwrap(), "other");
    let other_read = read_named(&other, "other");
    let other_rows = bare_rows(&other, other_read.admitted());
    let other_equality = qsl_package::emit_checked(&other, EmitLimits::fixture())
        .expect("emits")
        .equality_witness(other_read.admitted());
    let foreign = BarePackage::new(v2)
        .with_supply(
            qsl_package::BareSupply {
                identities: &other_rows,
                equality: Some(&equality),
            },
            &mut qsl_package::V2ReadLimits::fixture().native_join_budget(),
        )
        .expect("the fixture budget admits the supply");
    assert_eq!(
        call(&foreign, &on_bytes),
        package_refusal("foreign-identity-witness")
    );
    let mut conflicting = rows.declarations().to_vec();
    let first = conflicting[0].clone();
    let mut again = first.clone();
    again.original_identity = format!("{}-again", first.original_identity).into();
    conflicting.push(again);
    let unknown_rows = qsl_package::ReadIdentityRows::of_rows(
        v2,
        conflicting,
        rows.fields().to_vec(),
        rows.source_records().to_vec(),
        rows.source_unions().to_vec(),
    );
    let unknown = BarePackage::new(v2)
        .with_supply(
            qsl_package::BareSupply {
                identities: &unknown_rows,
                equality: Some(&equality),
            },
            &mut qsl_package::V2ReadLimits::fixture().native_join_budget(),
        )
        .expect("the fixture budget admits the supply");
    assert_eq!(
        call(&unknown, &on_bytes),
        package_refusal("unknown-identity-witness")
    );
    // Equality witnesses.
    let supply_with = |equality| {
        BarePackage::new(v2)
            .with_supply(
                qsl_package::BareSupply {
                    identities: &rows,
                    equality,
                },
                &mut qsl_package::V2ReadLimits::fixture().native_join_budget(),
            )
            .expect("the fixture budget admits the supply")
    };
    assert_eq!(
        call(&supply_with(None), &on_bytes),
        package_refusal("missing-equality-witness")
    );
    assert_eq!(
        call(&supply_with(Some(&other_equality)), &on_bytes),
        package_refusal("foreign-equality-witness")
    );
    let empty = qsl_package::ReadEqualityRows::read(
        b"{\n  \"format\": \"quire.checked-handoff.equality/1\",\n  \"applications\": [],\n  \"facts\": [],\n  \"source_units\": []\n}\n",
        v2,
    )
    .expect("empty rows read");
    assert_eq!(
        call(&supply_with(Some(&empty)), &on_bytes),
        package_refusal("unknown-equality-witness")
    );
    // A node the package does not hold is `Node`; a member it does not hold
    // is `Member`.
    let missing_node = ProtocolSlot {
        node: binder(&other, "Refunds", "earlier"),
        ..on_bytes.clone()
    };
    let (code, _, _, locus) = call(&good, &missing_node);
    assert_eq!(code, "missing_declaration");
    assert_eq!(locus, CallAdmissionLocus::Node(missing_node.node.clone()));
    let missing_member = ProtocolSlot {
        member: "no.such.member".to_owned(),
        ..on_bytes.clone()
    };
    let (code, _, _, locus) = call(&good, &missing_member);
    assert_eq!(code, "missing_declaration");
    assert_eq!(
        locus,
        CallAdmissionLocus::Member {
            name: "no.such.member".to_owned(),
            index: None
        }
    );
}

/// FR-390-AC-67 (call level): identity before equality, `Package` before
/// `Node`. With no identity rows and no equality rows the call reports the
/// identity witness; foreign identity rows and no equality rows report
/// `ForeignIdentityWitness`; identity rows that leave a binder record
/// unprojectable (no field rows) with no equality rows report
/// `MissingEqualityWitness` at `Package`, and with the equality rows the
/// `MissingTypeRepresentation` of the first such binder at its `Node`.
#[trace("FR-390-AC-67", "FR-533-AC-7", "TC-1070", "TC-1333")]
#[test]
fn the_call_reports_identity_then_equality_then_node_refusals() {
    let package = fixture();
    let read = read_checked_package(&package);
    let v2 = read.admitted();
    let rows = bare_rows(&package, v2);
    let equality = qsl_package::emit_checked(&package, EmitLimits::fixture())
        .expect("emits")
        .equality_witness(v2);
    let wanted = slot(&package, "Refunds", "Refund", "retry.relation", None);
    let on_bytes = bytes_slot(v2, "Refunds", "Refund", &wanted);
    let given = supply(&package, "Refunds", Vec::new(), None);
    let call = |bare: &BarePackage<'_>| {
        let mut meter = Meter::new(UNLIMITED);
        let result = bare.evaluate_protocol_slot(&on_bytes, &given, &mut meter);
        assert_pristine(&meter);
        let (_, _, reason, locus) = refusal_of(result);
        (reason.unwrap_or_default(), locus)
    };
    assert_eq!(
        call(&BarePackage::new(v2)),
        (
            "missing-identity-witness".to_owned(),
            CallAdmissionLocus::Package
        )
    );
    let (other, _) = fixture_named(SOURCE, &serde_json::to_vec(&document()).unwrap(), "other");
    let other_read = read_named(&other, "other");
    let other_rows = bare_rows(&other, other_read.admitted());
    let foreign = BarePackage::new(v2)
        .with_supply(
            qsl_package::BareSupply {
                identities: &other_rows,
                equality: None,
            },
            &mut qsl_package::V2ReadLimits::fixture().native_join_budget(),
        )
        .expect("the fixture budget admits the supply");
    assert_eq!(
        call(&foreign),
        (
            "foreign-identity-witness".to_owned(),
            CallAdmissionLocus::Package
        )
    );
    let stripped = qsl_package::ReadIdentityRows::of_rows(
        v2,
        rows.declarations().to_vec(),
        Vec::new(),
        rows.source_records().to_vec(),
        rows.source_unions().to_vec(),
    );
    let no_fields = |equality| {
        BarePackage::new(v2)
            .with_supply(
                qsl_package::BareSupply {
                    identities: &stripped,
                    equality,
                },
                &mut qsl_package::V2ReadLimits::fixture().native_join_budget(),
            )
            .expect("the fixture budget admits the supply")
    };
    assert_eq!(
        call(&no_fields(None)),
        (
            "missing-equality-witness".to_owned(),
            CallAdmissionLocus::Package
        )
    );
    let (reason, locus) = call(&no_fields(Some(&equality)));
    assert_eq!(reason, "missing-type-representation");
    let binders = [
        binder(&package, "Refunds", "earlier"),
        binder(&package, "Refunds", "later"),
    ];
    assert!(
        matches!(&locus, CallAdmissionLocus::Node(key) if binders.contains(key)),
        "the Node locus is a binder record: {locus:?}"
    );
}

/// TC-1070 fixture L with a `repeat` after the choice: the repeat's guard is
/// a span-named decision slot no other fixture of the family holds.
fn repeat_source() -> String {
    SOURCE.replace(
        "      case no when { not got.ready } check B using S { true };\n    }\n",
        "      case no when { not got.ready } check B using S { true };\n    }\n    \
         repeat Again by Buyer visible (got.ready) max 2 while { got.ready } \
         sequence Body { event Tick by Buyer as (tick: Boolean) { true }; } \
         exhausted sequence Spent { }\n",
    )
}

/// FR-533-AC-12 (repeat guard), FR-390-AC-65, B4: a `repeat`'s guard and
/// visible entry are selected by (node, member, index) alone and take the
/// origin of the `decision` family the source route locates them under, and
/// the guard evaluates on both routes to the same value and charge log. An
/// origin named by any other family (`related`) or none fails the test.
#[trace("FR-533-AC-12", "FR-390-AC-65", "TC-1333")]
#[test]
fn a_repeat_guard_takes_the_decision_origin_of_the_source_route() {
    let source = repeat_source();
    assert_ne!(source, SOURCE, "the repeat is added");
    let (package, _) = fixture_from_source(&source);
    let read = read_checked_package(&package);
    let v2 = read.admitted();
    let rows = bare_rows(&package, v2);
    let equality = qsl_package::emit_checked(&package, EmitLimits::fixture())
        .expect("emits")
        .equality_witness(v2);
    let bare = BarePackage::new(v2)
        .with_supply(
            qsl_package::BareSupply {
                identities: &rows,
                equality: Some(&equality),
            },
            &mut qsl_package::V2ReadLimits::fixture().native_join_budget(),
        )
        .expect("the fixture budget admits the supply");
    let world = bare.world().expect("world");
    let view = package.evaluation_view();
    let mut exercised = 0;
    for (member, index) in [("guard", None), ("visible", Some(0))] {
        let wanted = slot(&package, "Pipeline", "Again", member, index);
        let on_bytes = bytes_slot(v2, "Pipeline", "Again", &wanted);
        let selected = bare
            .select_protocol_slot(&qsl_package::ProtocolSlotDescriptor::new(
                on_bytes.node.clone(),
                on_bytes.member.clone(),
                on_bytes.index,
            ))
            .unwrap_or_else(|failure| panic!("{member}: {failure:?}"));
        let source_selected = view
            .select_protocol_slot(&qsl_package::ProtocolSlotDescriptor::new(
                wanted.node.clone(),
                wanted.member.clone(),
                wanted.index,
            ))
            .expect("source selects");
        let source_origin = format!(
            "{:?}",
            source_selected.checked_body().root().location().origin
        );
        let origin = selected
            .origin()
            .unwrap_or_else(|| panic!("{member}: the application has an origin"));
        assert!(
            origin.contains(" decision "),
            "{member}: the decision family, got {origin}"
        );
        assert_eq!(
            format!("TypeDeclaration {{ name: {origin:?} }}"),
            source_origin,
            "{member}"
        );
        let values = vec![("got", flag(&package, true))];
        let given = supply(&package, "Pipeline", values.clone(), None);
        let bytes_given = supply(
            &package,
            "Pipeline",
            values
                .iter()
                .map(|(name, value)| {
                    (
                        *name,
                        in_bytes_environment(value, package.graph().scope().types(), world.types()),
                    )
                })
                .collect(),
            None,
        );
        let mut source_meter = Meter::new(UNLIMITED);
        let on_source = package.evaluate_protocol_slot(&wanted, &given, &mut source_meter);
        let mut bytes_meter = Meter::new(UNLIMITED);
        let on_bare = bare.evaluate_protocol_slot(&on_bytes, &bytes_given, &mut bytes_meter);
        assert_eq!(
            without_location(&on_source),
            without_location(&on_bare),
            "{member}"
        );
        charges_and_usage_match(member, &source_meter, &bytes_meter);
        assert_value_eq!(
            completed(on_bare.unwrap()),
            Value::Boolean(true),
            "repeat slot value"
        );
        exercised += 1;
    }
    assert_eq!(exercised, 2);
}

/// Everything one bytes-route call of `Refund`'s retry relation needs.
struct RetryCall<'p> {
    bare: BarePackage<'p>,
    on_bytes: ProtocolSlot,
    given: ProtocolSupply<'static>,
    wanted: ProtocolSlot,
}

fn retry_call<'p>(
    package: &CheckedPackage,
    v2: &'p qsl_package::CheckedPackageV2,
    rows: &'p qsl_package::ReadIdentityRows,
    equality: &'p qsl_package::ReadEqualityRows,
) -> RetryCall<'p> {
    let bare = BarePackage::new(v2)
        .with_supply(
            qsl_package::BareSupply {
                identities: rows,
                equality: Some(equality),
            },
            &mut qsl_package::V2ReadLimits::fixture().native_join_budget(),
        )
        .expect("the fixture budget admits the supply");
    let world = bare.world().expect("world");
    let wanted = slot(package, "Refunds", "Refund", "retry.relation", None);
    let on_bytes = bytes_slot(v2, "Refunds", "Refund", &wanted);
    let values = [
        ("earlier", attempt(package, 0)),
        ("later", attempt(package, 1)),
        ("target", int(5)),
        ("activated", Value::Boolean(false)),
    ];
    let given = supply(
        package,
        "Refunds",
        values
            .iter()
            .map(|(name, value)| {
                (
                    *name,
                    in_bytes_environment(value, package.graph().scope().types(), world.types()),
                )
            })
            .collect(),
        None,
    );
    RetryCall {
        bare,
        on_bytes,
        given,
        wanted,
    }
}

#[trace("TC-1333", "FR-533-AC-3", "FR-531-AC-3")]
#[test]
fn replacing_a_checked_source_changes_its_result_but_not_the_bytes_slot() {
    let original = fixture();
    let changed_source = SOURCE.replace(
        "earlier.tally < 3 and later.tally = earlier.tally + 1",
        "earlier.tally < 0 and later.tally = earlier.tally + 1",
    );
    assert_ne!(
        changed_source, SOURCE,
        "the source expression actually changes"
    );
    let replacement = fixture_from_source(&changed_source).0;
    let read = read_checked_package(&original);
    let v2 = read.admitted();
    let rows = bare_rows(&original, v2);
    let equality = qsl_package::emit_checked(&original, EmitLimits::fixture())
        .expect("emits")
        .equality_witness(v2);
    let call = retry_call(&original, v2, &rows, &equality);
    let changed_slot = slot(&replacement, "Refunds", "Refund", "retry.relation", None);
    assert_eq!(
        call.wanted.node, changed_slot.node,
        "the declared protocol identity is unchanged"
    );
    let mut before_meter = Meter::new(UNLIMITED);
    let before = call
        .bare
        .evaluate_protocol_slot(&call.on_bytes, &call.given, &mut before_meter);
    assert_value_eq!(completed(before.unwrap()), Value::Boolean(true));
    let mut source_meter = Meter::new(UNLIMITED);
    let changed = replacement.evaluate_protocol_slot(&changed_slot, &call.given, &mut source_meter);
    assert_value_eq!(completed(changed.unwrap()), Value::Boolean(false));
    let mut after_meter = Meter::new(UNLIMITED);
    let after = call
        .bare
        .evaluate_protocol_slot(&call.on_bytes, &call.given, &mut after_meter);
    assert_value_eq!(completed(after.unwrap()), Value::Boolean(true));
    charges_and_usage_match(
        "unchanged bytes after source replacement",
        &before_meter,
        &after_meter,
    );
}

#[trace("TC-1333", "FR-533-AC-18", "FR-533-AC-17")]
#[test]
fn a_called_equality_body_requires_its_witness_at_the_actual_evaluation_entry() {
    let source = SOURCE
        .replace(
            "profile S = \"quire.state.graph/v1\";",
            "profile S = \"quire.state.graph/v1\";\n\
             profile V = \"quire.value.complete/v1\";",
        )
        .replace(
            "model M = \"test/orders\";",
            "model M = \"test/orders\";\n\
             function isZero using V(x: M::Total): Boolean pure { x = 0 }",
        )
        .replace(
            "when { not trigger.ready and target >= 0 }",
            "when { not trigger.ready and isZero(target) }",
        );
    assert!(source.contains("isZero(target)"));
    let package = fixture_from_source(&source).0;
    let read = read_checked_package(&package);
    let v2 = read.admitted();
    let rows = bare_rows(&package, v2);
    let equality = qsl_package::emit_checked(&package, EmitLimits::fixture())
        .expect("emits")
        .equality_witness(v2);
    let wanted = slot(&package, "Refunds", "Refund", "trigger.guard", None);
    let on_bytes = bytes_slot(v2, "Refunds", "Refund", &wanted);
    let given = supply(
        &package,
        "Refunds",
        vec![("trigger", flag(&package, false)), ("target", int(0))],
        None,
    );
    let no_equality = BarePackage::new(v2)
        .with_supply(
            qsl_package::BareSupply {
                identities: &rows,
                equality: None,
            },
            &mut qsl_package::V2ReadLimits::fixture().native_join_budget(),
        )
        .expect("the fixture budget admits the supply");
    let mut denied_meter = Meter::new(UNLIMITED);
    let missing = no_equality.evaluate_protocol_slot(&on_bytes, &given, &mut denied_meter);
    assert_pristine(&denied_meter);
    let (_, _, reason, locus) = refusal_of(missing);
    assert_eq!(reason.as_deref(), Some("missing-equality-witness"));
    assert_eq!(locus, CallAdmissionLocus::Package);
    let with_equality = BarePackage::new(v2)
        .with_supply(
            qsl_package::BareSupply {
                identities: &rows,
                equality: Some(&equality),
            },
            &mut qsl_package::V2ReadLimits::fixture().native_join_budget(),
        )
        .expect("the fixture budget admits the supply");
    let mut source_meter = Meter::new(UNLIMITED);
    let expected = package.evaluate_protocol_slot(&wanted, &given, &mut source_meter);
    let mut bytes_meter = Meter::new(UNLIMITED);
    let actual = with_equality.evaluate_protocol_slot(&on_bytes, &given, &mut bytes_meter);
    assert_eq!(without_location(&actual), without_location(&expected));
    assert_value_eq!(completed(actual.unwrap()), Value::Boolean(true));
    charges_and_usage_match("called equality body", &source_meter, &bytes_meter);

    let control = slot(
        &package,
        "Refunds",
        "Refund",
        "activation_captures",
        Some(0),
    );
    let on_bytes = bytes_slot(v2, "Refunds", "Refund", &control);
    let control_supply = supply(
        &package,
        "Refunds",
        vec![("trigger", flag(&package, false))],
        None,
    );
    let mut meter = Meter::new(UNLIMITED);
    let result = no_equality.evaluate_protocol_slot(&on_bytes, &control_supply, &mut meter);
    assert_value_eq!(completed(result.unwrap()), Value::Boolean(false));
}

/// FR-520-AC-11, FR-533 "The call": the call owns the meter's interval
/// capture while it runs and leaves it as the caller had it, enabled or not.
#[trace("FR-520-AC-11", "FR-533-AC-9", "TC-1333")]
#[test]
fn a_bare_call_leaves_the_callers_charge_capture_as_it_found_it() {
    let package = fixture();
    let read = read_checked_package(&package);
    let v2 = read.admitted();
    let rows = bare_rows(&package, v2);
    let equality = qsl_package::emit_checked(&package, EmitLimits::fixture())
        .expect("emits")
        .equality_witness(v2);
    let call = retry_call(&package, v2, &rows, &equality);
    for enabled in [false, true] {
        let mut meter = Meter::new(UNLIMITED);
        if enabled {
            meter.enable_charge_accounting();
        }
        let result = call
            .bare
            .evaluate_protocol_slot(&call.on_bytes, &call.given, &mut meter);
        assert!(matches!(result, Ok(ProtocolSlotEvaluation::Evaluated(..))));
        assert_eq!(
            meter.take_charge_accounting().is_some(),
            enabled,
            "capture enabled before: {enabled}"
        );
    }
    // The source route leaves it the same way.
    let mut source = Meter::new(UNLIMITED);
    let _ = package.evaluate_protocol_slot(&call.wanted, &call.given, &mut source);
    assert!(source.take_charge_accounting().is_none());
}

/// FR-390 supply admission on the bytes route: supplied binders are examined
/// in binder-level order, then by key, so the member a refusal names is the
/// one the source route names. Two duplicated binders whose levels order
/// opposite to their keys refuse the lower-level one first.
#[trace("FR-390-AC-9", "FR-533-AC-15", "TC-1333")]
#[test]
fn a_bare_supply_is_examined_in_binder_level_order() {
    let package = fixture();
    let read = read_checked_package(&package);
    let v2 = read.admitted();
    let rows = bare_rows(&package, v2);
    let equality = qsl_package::emit_checked(&package, EmitLimits::fixture())
        .expect("emits")
        .equality_witness(v2);
    let call = retry_call(&package, v2, &rows, &equality);
    let selected = call
        .bare
        .select_protocol_slot(&qsl_package::ProtocolSlotDescriptor::new(
            call.on_bytes.node.clone(),
            call.on_bytes.member.clone(),
            call.on_bytes.index,
        ))
        .expect("selects");
    let sources = selected.sources();
    let mut pair = None;
    for first in sources {
        for second in sources {
            if first.level() < second.level() && first.binder() > second.binder() {
                pair = Some((first.binder().clone(), second.binder().clone()));
            }
        }
    }
    let (low, high) = pair.expect("two binders whose levels order opposite to their keys");
    assert!(low > high);
    let duplicated = |keys: &[&NodeKey]| {
        let mut given = ProtocolSupply {
            binders: call.given.binders.clone(),
            observation: None,
        };
        for key in keys {
            let held = given
                .binders
                .iter()
                .find(|(candidate, _)| candidate == *key)
                .expect("supplied")
                .clone();
            given.binders.push(held);
        }
        given
    };
    let given = duplicated(&[&high, &low]);
    let mut meter = Meter::new(UNLIMITED);
    let (code, cause, _, locus) = refusal_of(call.bare.evaluate_protocol_slot(
        &call.on_bytes,
        &given,
        &mut meter,
    ));
    assert_eq!(
        (code.as_str(), cause.as_str()),
        ("invalid_runtime_input", "duplicate-member")
    );
    assert_eq!(locus, CallAdmissionLocus::Binder(low.clone()));
    let mut source = Meter::new(UNLIMITED);
    let (_, _, _, source_locus) = refusal_of(package.evaluate_protocol_slot(
        &call.wanted,
        &duplicated(&[&high, &low]),
        &mut source,
    ));
    assert_eq!(
        source_locus, locus,
        "the source route names the same member"
    );
}

/// FR-533-AC-9: three slot calls over one meter leave the sum of the fresh
/// meters' `work_units` and `result_units` and the maximum of the other
/// eight counters (FR-520-AC-7), and the same log and usage the source route
/// leaves for the same three calls.
#[trace("FR-533-AC-9", "FR-520-AC-7", "TC-1333")]
#[test]
fn three_bare_calls_over_one_meter_sum_and_maximise_as_the_source_route_does() {
    let package = fixture();
    let read = read_checked_package(&package);
    let v2 = read.admitted();
    let rows = bare_rows(&package, v2);
    let equality = qsl_package::emit_checked(&package, EmitLimits::fixture())
        .expect("emits")
        .equality_witness(v2);
    let bare = BarePackage::new(v2)
        .with_supply(
            qsl_package::BareSupply {
                identities: &rows,
                equality: Some(&equality),
            },
            &mut qsl_package::V2ReadLimits::fixture().native_join_budget(),
        )
        .expect("the fixture budget admits the supply");
    let world = bare.world().expect("world");
    let specs = parity_slots(&package);
    // Each unobserved slot's call and its usage on a fresh bytes-route meter.
    let mut calls = Vec::new();
    for spec in specs.iter().filter(|spec| !spec.observed) {
        let wanted = slot(&package, spec.protocol, spec.owner, spec.member, spec.index);
        let on_bytes = bytes_slot(v2, spec.protocol, spec.owner, &wanted);
        let values = (spec.values)(None);
        let given = supply(&package, spec.protocol, values.clone(), None);
        let bytes_given = supply(
            &package,
            spec.protocol,
            values
                .iter()
                .map(|(name, value)| {
                    (
                        *name,
                        in_bytes_environment(value, package.graph().scope().types(), world.types()),
                    )
                })
                .collect(),
            None,
        );
        let mut alone = Meter::new(UNLIMITED);
        let one = bare.evaluate_protocol_slot(&on_bytes, &bytes_given, &mut alone);
        assert!(matches!(one, Ok(ProtocolSlotEvaluation::Evaluated(..))));
        calls.push((spec.name, wanted, on_bytes, given, bytes_given, alone));
    }
    // The three calls that use the most, so that a sum and a maximum differ.
    calls.sort_by_key(|(.., alone)| {
        std::cmp::Reverse(
            LimitKind::ALL
                .iter()
                .map(|counter| alone.consumed(*counter))
                .sum::<u64>(),
        )
    });
    calls.truncate(3);
    assert_eq!(calls.len(), 3);
    let mut shared_source = Meter::new(UNLIMITED);
    let mut shared_bare = Meter::new(UNLIMITED);
    for (name, wanted, on_bytes, given, bytes_given, _) in &calls {
        let on_source = package.evaluate_protocol_slot(wanted, given, &mut shared_source);
        let on_bare = bare.evaluate_protocol_slot(on_bytes, bytes_given, &mut shared_bare);
        assert_eq!(
            without_location(&on_source),
            without_location(&on_bare),
            "{name}"
        );
    }
    charges_and_usage_match("three calls", &shared_source, &shared_bare);
    let mut distinguishes = false;
    for counter in LimitKind::ALL {
        let each = calls.iter().map(|(.., alone)| alone.consumed(counter));
        let (sum, max) = (each.clone().sum::<u64>(), each.max().unwrap_or(0));
        distinguishes |= sum != max;
        let expected = if matches!(counter, LimitKind::WorkUnits | LimitKind::ResultUnits) {
            sum
        } else {
            max
        };
        assert_eq!(shared_bare.consumed(counter), expected, "{counter:?}");
    }
    assert!(distinguishes, "the chosen calls tell a sum from a maximum");
}

/// FR-533-AC-10: constructing the package, supplying rows, creating the
/// world, selecting and projecting a slot and admitting a supply charge no
/// counter and take no meter, and a call refused at view creation, selection
/// or supply admission leaves the caller's meter, which already holds the
/// charges of an earlier call, equal to what it was.
#[trace("FR-533-AC-10", "TC-1333")]
#[test]
fn nothing_before_the_run_charges_and_a_refusal_leaves_the_meter_unchanged() {
    let package = fixture();
    let read = read_checked_package(&package);
    let v2 = read.admitted();
    let rows = bare_rows(&package, v2);
    let equality = qsl_package::emit_checked(&package, EmitLimits::fixture())
        .expect("emits")
        .equality_witness(v2);
    let call = retry_call(&package, v2, &rows, &equality);
    let mut meter = Meter::new(UNLIMITED);
    let first = call
        .bare
        .evaluate_protocol_slot(&call.on_bytes, &call.given, &mut meter);
    assert!(matches!(first, Ok(ProtocolSlotEvaluation::Evaluated(..))));
    assert!(meter.consumed(LimitKind::WorkUnits) > 0);
    let before = meter.clone();
    let descriptor = qsl_package::ProtocolSlotDescriptor::new(
        call.on_bytes.node.clone(),
        call.on_bytes.member.clone(),
        call.on_bytes.index,
    );
    let selected = call
        .bare
        .select_protocol_slot(&descriptor)
        .expect("selects");
    call.bare.world().expect("world");
    call.bare
        .project_protocol_slot(&selected)
        .expect("projects");
    assert_eq!(meter, before, "the steps before the run take no meter");
    // Refused at view creation: no equality rows.
    let without_rows = BarePackage::new(v2)
        .with_supply(
            qsl_package::BareSupply {
                identities: &rows,
                equality: None,
            },
            &mut qsl_package::V2ReadLimits::fixture().native_join_budget(),
        )
        .expect("the fixture budget admits the supply");
    let refused = without_rows.evaluate_protocol_slot(&call.on_bytes, &call.given, &mut meter);
    assert!(matches!(refused, Err(CallFailure::Admission(_))));
    assert_eq!(meter, before, "view creation");
    // Refused at selection: a member the node does not hold.
    let missing_member = ProtocolSlot {
        member: "no.such.member".to_owned(),
        ..call.on_bytes.clone()
    };
    let refused = call
        .bare
        .evaluate_protocol_slot(&missing_member, &call.given, &mut meter);
    assert!(matches!(refused, Err(CallFailure::Admission(_))));
    assert_eq!(meter, before, "selection");
    // Refused at supply admission: a missing member.
    let mut short = ProtocolSupply {
        binders: call.given.binders.clone(),
        observation: None,
    };
    short.binders.pop();
    let refused = call
        .bare
        .evaluate_protocol_slot(&call.on_bytes, &short, &mut meter);
    let (code, cause, _, _) = refusal_of(refused);
    assert_eq!(
        (code.as_str(), cause.as_str()),
        ("invalid_runtime_input", "missing-member")
    );
    assert_eq!(meter, before, "supply admission");
}

/// The E-P package of fixture source `source` and model document `domain`,
/// read back, with the bare route over its handoff.
struct EpRoutes<'p> {
    read: qsl_package::AdmittedPackageRead<'p>,
    rows: qsl_package::ReadIdentityRows,
    equality: qsl_package::ReadEqualityRows,
}

impl<'p> EpRoutes<'p> {
    fn new(package: &'p CheckedPackage, domain: Vec<u8>) -> Self {
        let read = read_checked_package_with_model_document(package, &domain);
        let v2 = read.admitted();
        let rows = bare_rows(package, v2);
        let equality = qsl_package::emit_checked(package, EmitLimits::fixture())
            .expect("emits")
            .equality_witness(v2);
        Self {
            read,
            rows,
            equality,
        }
    }

    /// A new `BarePackage` value over the package's bytes.
    fn bare(&self) -> BarePackage<'_> {
        BarePackage::new(self.read.admitted())
            .with_supply(
                qsl_package::BareSupply {
                    identities: &self.rows,
                    equality: Some(&self.equality),
                },
                &mut qsl_package::V2ReadLimits::fixture().native_join_budget(),
            )
            .expect("the fixture budget admits the supply")
    }
}

/// The four references and the four-occurrence attribution of TC-1070's E-P
/// receipt, as `nested_reference_provenance_drives_the_named_population_admission`
/// builds them.
fn ep_references(
    package: &CheckedPackage,
    view: &qsl_semantics::model::normalize::EffectiveView,
) -> (Value, Vec<ProtocolReferenceOccurrence>) {
    let peer = other_reference(view, "ghost");
    let nested = other_reference(view, "nested-1");
    let optional = other_reference(view, "option-1");
    let sequence = other_reference(view, "sequence-1");
    let receipt = ep_receipt(package, [&peer, &nested, &optional, &sequence]);
    let occurrence = |path: Vec<PathStep>, population: &str| ProtocolReferenceOccurrence {
        path,
        population: population_node(population),
    };
    let references = vec![
        occurrence(vec![PathStep::Field("peer".into())], "otherSpare"),
        occurrence(
            vec![
                PathStep::Field("bundle".into()),
                PathStep::Field("nested".into()),
            ],
            "otherNodes",
        ),
        occurrence(
            vec![
                PathStep::Field("bundle".into()),
                PathStep::Field("optional".into()),
                PathStep::OptionValue,
            ],
            "otherSpare",
        ),
        occurrence(
            vec![
                PathStep::Field("bundle".into()),
                PathStep::Field("sequence".into()),
                PathStep::Index(0),
            ],
            "otherNodes",
        ),
    ];
    (receipt, references)
}

fn registration_capture(package: &CheckedPackage) -> ProtocolSlot {
    slot(
        package,
        "Refunds",
        "Refund",
        "registration_captures",
        Some(0),
    )
}

/// FR-533-AC-5 (a)-(d): over fixture E-P with the same domain documents,
/// snapshot and limits on both routes, the bare admission returns the source
/// admission, a slot call over either route's admitted observation gives the
/// same result and charges, the bare host's population types and the bare
/// environment equal the source's, and a model the package does not select
/// refuses `wrong-model-selection` with no fields, also without a supply.
#[trace("FR-533-AC-5", "FR-390-AC-21", "TC-1333")]
#[test]
fn the_bare_observation_admission_equals_the_source_admission_on_e_p() {
    let (package, view) = fixture_ep();
    let routes = EpRoutes::new(&package, serde_json::to_vec(&document_ep()).unwrap());
    let bare = routes.bare();
    let source_view = package.evaluation_view();
    let model = package.graph().model_selections()[0].clone();
    assert_eq!(bare.model_selections(), vec![model.clone()]);
    let (receipt, references) = ep_references(&package, &view);
    let wanted = registration_capture(&package);
    let on_bytes = bytes_slot(routes.read.admitted(), "Refunds", "Refund", &wanted);
    let limits = qsl_semantics::model::accounting::ModelNormalizationLimits::default();
    type OtherSpare<'a> = Option<(bool, &'a [&'a str])>;
    let states: [(OtherSpare, bool); 4] = [
        (Some((true, &["ghost", "option-1"])), true),
        (Some((false, &[])), false),
        (None, false),
        (Some((true, &[])), false),
    ];
    for (state, evaluates) in states {
        let (packages, selected, snapshots) = ep_snapshot(state, true, None, None, &[]);
        let on_source = source_view.admit_protocol_observation(
            &model,
            &packages,
            limits,
            &snapshots,
            &selected,
            ObservationLimits::default(),
        );
        let on_bare = bare.admit_protocol_observation(
            &model,
            &packages,
            limits,
            &snapshots,
            &selected,
            ObservationLimits::default(),
        );
        match (&on_source, &on_bare) {
            (ObservationAdmission::Admitted(source), ObservationAdmission::Admitted(bytes)) => {
                assert_eq!(source.usage(), bytes.usage());
                assert_eq!(source.populations(), bytes.populations());
                assert_eq!(
                    source.environment().references(),
                    bytes.environment().references()
                );
            }
            (ObservationAdmission::Refused(source), ObservationAdmission::Refused(bytes)) => {
                assert_eq!(source, bytes);
            }
            (ObservationAdmission::Incomplete(source), ObservationAdmission::Incomplete(bytes)) => {
                assert_eq!(source, bytes);
            }
            other => panic!("the routes admit differently: {other:?}"),
        }
        let given =
            |observation| ep_supply(&package, receipt.clone(), references.clone(), observation);
        let mut source_meter = Meter::new(UNLIMITED);
        let source =
            package.evaluate_protocol_slot(&wanted, &given(Some(&on_source)), &mut source_meter);
        let mut bare_meter = Meter::new(UNLIMITED);
        let bytes = bare.evaluate_protocol_slot(&on_bytes, &given(Some(&on_bare)), &mut bare_meter);
        assert_eq!(without_location(&source), without_location(&bytes));
        charges_and_usage_match("E-P registration capture", &source_meter, &bare_meter);
        if evaluates {
            assert_value_eq!(completed(bytes.unwrap()), int(5));
        } else {
            assert!(bytes.is_err(), "the other states refuse");
        }
    }
    // (b) A reference attributed to a population of another model, and to a
    // population of this model that no slot reads, refuses as the source does.
    let (packages, selected, snapshots) =
        ep_snapshot(Some((true, &["ghost", "option-1"])), true, None, None, &[]);
    let on_source = source_view.admit_protocol_observation(
        &model,
        &packages,
        limits,
        &snapshots,
        &selected,
        ObservationLimits::default(),
    );
    let on_bare = bare.admit_protocol_observation(
        &model,
        &packages,
        limits,
        &snapshots,
        &selected,
        ObservationLimits::default(),
    );
    let other_model = NodeKey::new(
        "other/model",
        vec![
            quire_exact::PathSegment::Text("relation".into()),
            quire_exact::PathSegment::Text("population".into()),
            quire_exact::PathSegment::Text("ix://other/model/externalOthers".into()),
        ],
        0,
    )
    .unwrap();
    for (population, cause) in [
        (population_node("elsewhereNodes"), "wrong-value-kind"),
        (other_model, "wrong-model-selection"),
    ] {
        let mut supplied = references.clone();
        supplied[0].population = population;
        let given =
            |observation| ep_supply(&package, receipt.clone(), supplied.clone(), observation);
        let mut source_meter = Meter::new(UNLIMITED);
        let source =
            package.evaluate_protocol_slot(&wanted, &given(Some(&on_source)), &mut source_meter);
        let mut bare_meter = Meter::new(UNLIMITED);
        let bytes = bare.evaluate_protocol_slot(&on_bytes, &given(Some(&on_bare)), &mut bare_meter);
        assert_eq!(without_location(&source), without_location(&bytes));
        let (_, found, _, _) = refusal_of(bytes);
        assert_eq!(found, cause);
        assert_pristine(&bare_meter);
    }
    // (c) Population types and the type environment.
    let world = bare.world().expect("world");
    for name in ["nodes", "otherNodes", "otherSpare", "elsewhereNodes"] {
        let node = population_node(name);
        let (source_declaration, source_members) = package
            .graph()
            .protocol_population_types(&node)
            .unwrap_or_else(|| panic!("{name} is a source population"));
        let (bare_declaration, bare_members) = bare
            .population_types(&node)
            .unwrap_or_else(|| panic!("{name} is in the catalog"));
        assert_eq!(source_declaration, bare_declaration, "{name}");
        assert_eq!(
            source_members
                .iter()
                .collect::<std::collections::BTreeSet<_>>(),
            bare_members
                .iter()
                .collect::<std::collections::BTreeSet<_>>(),
            "{name}"
        );
    }
    let identity = |name: &str| {
        view.type_identities()[&quire_checked::key::DeclarationKey {
            package: model::PACKAGE.to_owned(),
            node: model::id(name),
        }]
            .clone()
    };
    let attributes = |types: &quire_semantic_value::declaration::TypeEnvironment, name: &str| {
        types
            .attributes(identity(name))
            .map(|set| {
                set.iter()
                    .map(|attribute| {
                        (
                            attribute.field().name().to_owned(),
                            attribute.field().value_type().clone(),
                            attribute.field().presence(),
                        )
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_else(|| panic!("{name} declares attributes"))
    };
    for name in ["Node", "Other", "Elsewhere"] {
        assert_eq!(
            attributes(package.graph().scope().types(), name),
            attributes(world.types(), name),
            "{name}"
        );
        for sup in ["Node", "Other", "Elsewhere"] {
            assert_eq!(
                package
                    .graph()
                    .scope()
                    .types()
                    .conforms(identity(name), identity(sup)),
                world.types().conforms(identity(name), identity(sup)),
                "{name} conforms to {sup}"
            );
        }
    }
    // (d) A model the package does not select.
    let unselected = qsl_semantics::model::domain_package::DomainPackageRef {
        identity: "other/model".to_owned(),
    };
    let on_source = source_view.admit_protocol_observation(
        &unselected,
        &packages,
        limits,
        &snapshots,
        &selected,
        ObservationLimits::default(),
    );
    let ObservationAdmission::Refused(record) = on_source else {
        panic!("the source route refuses an unselected model")
    };
    assert_eq!(
        (record.catalog.code(), record.catalog.cause()),
        ("invalid_model_binding", "wrong-model-selection")
    );
    assert!(record.fields.is_empty() && record.path.is_empty() && record.kernel.is_none());
    let without_supply = BarePackage::new(routes.read.admitted());
    for route in [&bare, &without_supply] {
        let on_bare = route.admit_protocol_observation(
            &unselected,
            &packages,
            limits,
            &snapshots,
            &selected,
            ObservationLimits::default(),
        );
        let ObservationAdmission::Refused(found) = on_bare else {
            panic!("the bare route refuses an unselected model")
        };
        assert_eq!(found, record);
    }
}

/// FR-533-AC-6: an observation is its admitting `BarePackage` value's alone.
/// Another model's package refuses it at the population check, and another
/// `BarePackage` value over the same bytes refuses it, each as the source
/// route does for the same pair, and the call leaves the meter untouched.
#[trace("FR-533-AC-6", "TC-1333")]
#[test]
fn an_observation_belongs_to_the_bare_package_value_that_admitted_it() {
    let (package, view) = fixture_ep();
    let routes = EpRoutes::new(&package, serde_json::to_vec(&document_ep()).unwrap());
    let (receipt, references) = ep_references(&package, &view);
    let wanted = registration_capture(&package);
    let on_bytes = bytes_slot(routes.read.admitted(), "Refunds", "Refund", &wanted);
    let limits = qsl_semantics::model::accounting::ModelNormalizationLimits::default();
    let (packages, selected, snapshots) =
        ep_snapshot(Some((true, &["ghost", "option-1"])), true, None, None, &[]);
    let model = package.graph().model_selections()[0].clone();
    let admit = |bare: &BarePackage<'_>| {
        bare.admit_protocol_observation(
            &model,
            &packages,
            limits,
            &snapshots,
            &selected,
            ObservationLimits::default(),
        )
    };
    let own = routes.bare();
    let other = routes.bare();
    let observation = admit(&own);
    let given = |observation| ep_supply(&package, receipt.clone(), references.clone(), observation);
    let mut meter = Meter::new(UNLIMITED);
    assert_value_eq!(
        completed(
            own.evaluate_protocol_slot(&on_bytes, &given(Some(&observation)), &mut meter)
                .unwrap()
        ),
        int(5)
    );
    let forward = binder(&package, "Refunds", "forward");
    let mut meter = Meter::new(UNLIMITED);
    let refused = other.evaluate_protocol_slot(&on_bytes, &given(Some(&observation)), &mut meter);
    let (code, cause, _, locus) = refusal_of(refused);
    assert_eq!(
        (code.as_str(), cause.as_str(), locus),
        (
            "invalid_model_binding",
            "wrong-model-selection",
            CallAdmissionLocus::Binder(forward.clone())
        )
    );
    assert_pristine(&meter);

    // The same source with its model renamed, as a second package.
    let rename = |text: &str| text.replace(model::PACKAGE, "test/orders-copy");
    let copy_document = rename(&serde_json::to_string(&document_ep()).unwrap());
    let (copy, copy_view) = fixture_from_document(&rename(SOURCE), copy_document.as_bytes());
    let copy_routes = EpRoutes::new(&copy, copy_document.into_bytes());
    let copy_bare = copy_routes.bare();
    let copy_model = copy.graph().model_selections()[0].clone();
    assert_ne!(copy_model, model);
    // The copy's value, with the first package's population keys.
    let in_copy = |key: &str| {
        let declaration = quire_checked::key::DeclarationKey {
            package: "test/orders-copy".to_owned(),
            node: model::id("Other").replace(model::PACKAGE, "test/orders-copy"),
        };
        ObjectReference::new(
            copy_view
                .object_universe_of(&declaration)
                .unwrap()
                .identity()
                .unwrap(),
            copy_view.type_identities()[&declaration].clone(),
            ObjectId::new(key).unwrap(),
        )
    };
    let copy_receipt = ep_receipt(
        &copy,
        [
            &in_copy("ghost"),
            &in_copy("nested-1"),
            &in_copy("option-1"),
            &in_copy("sequence-1"),
        ],
    );
    let source_observation = package.evaluation_view().admit_protocol_observation(
        &model,
        &packages,
        limits,
        &snapshots,
        &selected,
        ObservationLimits::default(),
    );
    let copy_wanted = registration_capture(&copy);
    let copy_on_bytes = bytes_slot(
        copy_routes.read.admitted(),
        "Refunds",
        "Refund",
        &copy_wanted,
    );
    let supplied = |target: &CheckedPackage, observation| {
        ep_supply(
            target,
            copy_receipt.clone(),
            references.clone(),
            observation,
        )
    };
    let mut source_meter = Meter::new(UNLIMITED);
    let on_source = copy.evaluate_protocol_slot(
        &copy_wanted,
        &supplied(&copy, Some(&source_observation)),
        &mut source_meter,
    );
    let mut bare_meter = Meter::new(UNLIMITED);
    let on_bare = copy_bare.evaluate_protocol_slot(
        &copy_on_bytes,
        &supplied(&copy, Some(&observation)),
        &mut bare_meter,
    );
    // The source route's record for the same pair of packages and supply.
    assert_eq!(without_location(&on_source), without_location(&on_bare));
    let (code, cause, _, locus) = refusal_of(on_bare);
    assert_eq!(
        (code.as_str(), cause.as_str(), locus),
        (
            "foreign_reference",
            "foreign-universe",
            CallAdmissionLocus::Binder(binder(&copy, "Refunds", "forward"))
        ),
        "the first observation's universes are no universe of the copy's references"
    );
    assert_pristine(&bare_meter);
}

/// FR-533-AC-5 (e): a snapshot whose `OtherLinks` instance has a `Node`
/// object as its source end refuses `wrong-value-kind` on both routes with
/// equal `relationship`, `instance` and `end` fields and path. Check 6.8
/// refuses it before the extent traversal that names types, so the record
/// carries no `type` or `declared` field to differ ("Names of object types").
#[trace("FR-533-AC-5", "TC-1333")]
#[test]
fn a_relationship_end_of_the_wrong_object_type_refuses_as_on_the_source_route() {
    let (package, _) = fixture_ep();
    let routes = EpRoutes::new(&package, serde_json::to_vec(&document_ep()).unwrap());
    let bare = routes.bare();
    let model = package.graph().model_selections()[0].clone();
    let (packages, selected, mut snapshots) =
        ep_snapshot(Some((true, &["ghost", "option-1"])), true, None, None, &[]);
    let mut snapshot: serde_json::Value = serde_json::from_slice(&snapshots[&selected]).unwrap();
    snapshot["relationships"][0]["instances"] = serde_json::json!([{
        "source": {"population": model::id("nodes"), "key": "n5"},
        "target": {"population": model::id("otherNodes"), "key": "nested-1"},
    }]);
    snapshots.insert(selected.clone(), serde_json::to_vec(&snapshot).unwrap());
    let limits = qsl_semantics::model::accounting::ModelNormalizationLimits::default();
    let ObservationAdmission::Refused(source) =
        package.evaluation_view().admit_protocol_observation(
            &model,
            &packages,
            limits,
            &snapshots,
            &selected,
            ObservationLimits::default(),
        )
    else {
        panic!("the source route refuses the end")
    };
    let ObservationAdmission::Refused(bytes) = bare.admit_protocol_observation(
        &model,
        &packages,
        limits,
        &snapshots,
        &selected,
        ObservationLimits::default(),
    ) else {
        panic!("the bare route refuses the end")
    };
    assert_eq!(
        (bytes.catalog.code(), bytes.catalog.cause()),
        ("invalid_runtime_input", "wrong-value-kind")
    );
    assert_eq!(bytes, source);
    for field in ["relationship", "instance", "end"] {
        assert!(bytes.fields.contains_key(field), "{field}");
    }
    assert!(!bytes.fields.contains_key("type") && !bytes.fields.contains_key("declared"));
    assert!(bytes.path.is_empty());
}

/// FR-533-AC-22 (variant): over a copy of fixture E-P's model that adds one
/// variant type no slot reads, `with_supply` succeeds, `Refund.trigger.guard`
/// evaluates to the source route's value and charge log, and the bare
/// admission refuses `missing-type-representation` where the source route
/// admits the same snapshot.
#[trace("FR-533-AC-22", "TC-1333")]
#[test]
fn an_unread_variant_type_refuses_admission_but_not_the_slots_that_ignore_it() {
    use qsl_semantics::model::intake::meaning;
    let mut document = document_ep();
    document["constructs"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!({
            "kind": {"module": model::BUSINESS, "name": "tier"},
            "moduleVersion": "1.0.0",
            "construct": {
                "identity": "none", "shape": "enumeration",
                "members": {"variants": "required"}, "meaning": meaning::VARIANT_TYPE,
            },
        }));
    let case = |name: &str, line: u64| {
        serde_json::json!({
            "identity": model::id(&format!("variant/Odd-{name}")),
            "name": name,
            "origin": {"source": {
                "sourceIdentity": model::id("spec"), "path": "spec.qspec",
                "startLine": line, "startColumn": 3,
            }},
        })
    };
    document["types"]
        .as_array_mut()
        .unwrap()
        .push(model::declaration(
            "Odd",
            "tier",
            serde_json::json!({"variants": [case("A", 10), case("B", 11)]}),
        ));
    let bytes = serde_json::to_vec(&document).unwrap();
    let (package, _) = fixture_from_document(SOURCE, &bytes);
    let routes = EpRoutes::new(&package, bytes.clone());
    let bare = routes.bare();
    let wanted = slot(&package, "Refunds", "Refund", "trigger.guard", None);
    let on_bytes = bytes_slot(routes.read.admitted(), "Refunds", "Refund", &wanted);
    let values = vec![("trigger", flag(&package, false)), ("target", int(5))];
    let given = supply(&package, "Refunds", values, None);
    let mut source_meter = Meter::new(UNLIMITED);
    let source = package.evaluate_protocol_slot(&wanted, &given, &mut source_meter);
    let mut bare_meter = Meter::new(UNLIMITED);
    let bytes_result = bare.evaluate_protocol_slot(&on_bytes, &given, &mut bare_meter);
    assert!(matches!(source, Ok(ProtocolSlotEvaluation::Evaluated(..))));
    assert_eq!(without_location(&source), without_location(&bytes_result));
    charges_and_usage_match("variant trigger guard", &source_meter, &bare_meter);

    let packages = qsl_semantics::model::intake::package_input([bytes.as_slice()]).unwrap();
    let (_, selected, snapshots) =
        ep_snapshot(Some((true, &["ghost", "option-1"])), true, None, None, &[]);
    let model_ref = package.graph().model_selections()[0].clone();
    let limits = qsl_semantics::model::accounting::ModelNormalizationLimits::default();
    assert!(matches!(
        package.evaluation_view().admit_protocol_observation(
            &model_ref,
            &packages,
            limits,
            &snapshots,
            &selected,
            ObservationLimits::default(),
        ),
        ObservationAdmission::Admitted(_)
    ));
    let ObservationAdmission::Refused(record) = bare.admit_protocol_observation(
        &model_ref,
        &packages,
        limits,
        &snapshots,
        &selected,
        ObservationLimits::default(),
    ) else {
        panic!("the bare route refuses, not faults")
    };
    assert_eq!(
        (record.catalog.code(), record.catalog.cause()),
        ("unknown_required_feature", "unsupported-feature")
    );
    assert_eq!(
        record.fields.get("package").map(String::as_str),
        Some(routes.read.admitted().name())
    );
    assert_eq!(
        record.fields.get("reason").map(String::as_str),
        Some("missing-type-representation")
    );
}

/// FR-533 "Observation admission", step 4: where the caller's documents
/// declare an object type the package did not select, the bare environment
/// declares no attributes for it, and the shared admission refuses
/// `missing-type-representation` rather than faulting.
#[trace("FR-533-AC-22", "TC-1333")]
#[test]
fn a_snapshot_object_of_an_unselected_type_is_a_missing_type_representation() {
    let (package, _) = fixture_ep();
    let routes = EpRoutes::new(&package, serde_json::to_vec(&document_ep()).unwrap());
    let bare = routes.bare();
    let model_ref = package.graph().model_selections()[0].clone();
    let mut document = document_ep();
    document["types"]
        .as_array_mut()
        .unwrap()
        .push(model::declaration(
            "Extra",
            "thing",
            serde_json::json!({
                "identityFields": [model::id("Extra/marker")],
                "fields": [model::field("Extra", "marker", "ix://quire/native/Boolean")],
            }),
        ));
    document["populations"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!({
            "identity": model::id("extras"), "displayName": "extras",
            "kind": {"module": model::BUSINESS, "name": "population"},
            "members": [model::id("Extra")], "extent": "closed", "origin": model::origin("extras"),
        }));
    let (_, selected, mut snapshots) =
        ep_snapshot(Some((true, &["ghost", "option-1"])), true, None, None, &[]);
    let mut snapshot: serde_json::Value = serde_json::from_slice(&snapshots[&selected]).unwrap();
    snapshot["populations"].as_array_mut().unwrap().push(serde_json::json!({
        "population": model::id("extras"), "complete": true,
        "objects": [{"key": "x1", "type": model::id("Extra"), "fields": {"marker": {"boolean": true}}}],
    }));
    snapshots.insert(selected.clone(), serde_json::to_vec(&snapshot).unwrap());
    let bytes = serde_json::to_vec(&document).unwrap();
    let packages = qsl_semantics::model::intake::package_input([bytes.as_slice()]).unwrap();
    let ObservationAdmission::Refused(record) = bare.admit_protocol_observation(
        &model_ref,
        &packages,
        qsl_semantics::model::accounting::ModelNormalizationLimits::default(),
        &snapshots,
        &selected,
        ObservationLimits::default(),
    ) else {
        panic!("the bare route refuses, not faults")
    };
    assert_eq!(
        (record.catalog.code(), record.catalog.cause()),
        ("unknown_required_feature", "unsupported-feature")
    );
    assert_eq!(
        record.fields.get("reason").map(String::as_str),
        Some("missing-type-representation")
    );
}
