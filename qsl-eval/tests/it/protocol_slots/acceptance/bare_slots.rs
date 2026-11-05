// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-533, TC-1333: the protocol-slot call of a package read from its bytes
//! alone returns the in-process route's value and charge log.

use super::tc1070::model;
use super::*;
use qsl_eval::bare::BarePackage;
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

/// A record `value` of the source environment rebuilt in the bytes
/// environment: the bytes route declares a record's fields in the order the
/// package gives them, so a consumer builds its values from that
/// environment, field by field by name.
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
    let named = fields
        .iter()
        .zip(composite.slots())
        .map(|(field, slot)| (field.name(), slot.clone()))
        .collect::<Vec<_>>();
    bytes
        .record(key, named)
        .expect("the record is declared in the package")
}

/// FR-533-AC-16 and FR-531-AC-1 (evaluate halves), and the source-route
/// halves of AC-1 and AC-2: over every TC-1070 slot
/// that reads no observation, the bare route returns the source route's
/// value, outcome and ordered charge log, at the exact budget, one short of
/// each used counter, and with every binder unavailable; the integer
/// equality of `Refund`'s retry relation and recovery predicate is among
/// them and reads the equality rows of the handoff.
#[trace("FR-533-AC-16", "FR-531-AC-1", "TC-1333")]
#[test]
fn the_bare_route_returns_the_source_value_and_charge_log_for_every_unobserved_slot() {
    let package = fixture();
    let read = read_checked_package(&package);
    let v2 = read.admitted();
    let rows = bare_rows(&package, v2);
    let equality = qsl_package::emit_checked(&package)
        .expect("emits")
        .equality_witness(v2);
    let bare = BarePackage::new(v2).with_supply(qsl_package::BareSupply {
        identities: &rows,
        equality: Some(&equality),
    });
    let world = bare.world().expect("world");
    let mut compared = Vec::new();
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
        if let Some(expected) = &spec.expected {
            assert_value_eq!(completed(bytes.unwrap()), *expected, "bare slot value");
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
            assert!(matches!(
                on_bare,
                Ok(ProtocolSlotEvaluation::Unavailable { .. })
            ));
            assert_eq!(without_location(&on_source), without_location(&on_bare));
            charges_and_usage_match(spec.name, &source_meter, &bare_meter);
        }
        compared.push((spec.owner, spec.member));
    }
    assert!(compared.len() >= 10, "{compared:?}");
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
    let bag = model::declaration(
        "Bag",
        "record",
        json!({"fields": [field("items", false, true), field("one", true, false), field("maybe", true, true)]}),
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
    let equality = qsl_package::emit_checked(&package)
        .expect("emits")
        .equality_witness(v2);
    let bare = BarePackage::new(v2).with_supply(qsl_package::BareSupply {
        identities: &rows,
        equality: Some(&equality),
    });
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
            ("one".to_owned(), quire_exact::Presence::Optional),
            ("maybe".to_owned(), quire_exact::Presence::Optional),
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
                tally,
                Some(CardinalityBound::new(0, 3).expect("bound")),
            ),
            vec![int(3)],
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
    let mut source_meter = Meter::new(UNLIMITED);
    let on_source = package.evaluate_protocol_slot(
        &wanted,
        &supply(&package, "Refunds", vec![("forward", source_receipt)], None),
        &mut source_meter,
    );
    let mut bytes_meter = Meter::new(UNLIMITED);
    let on_bare = bare.evaluate_protocol_slot(
        &on_bytes,
        &supply(&package, "Refunds", vec![("forward", bytes_receipt)], None),
        &mut bytes_meter,
    );
    // The two environments declare a record's fields in different orders
    // (the source's by name, the package's as declared), so the returned
    // record is compared with the same record built in its own environment,
    // and the charge logs with each other.
    charges_and_usage_match("bag capture", &source_meter, &bytes_meter);
    let (Ok(ProtocolSlotEvaluation::Evaluated(..)), Ok(ProtocolSlotEvaluation::Evaluated(..))) =
        (&on_source, &on_bare)
    else {
        panic!("both routes evaluate: {on_source:?} {on_bare:?}");
    };
    assert_value_eq!(completed(on_source.unwrap()), source_bag, "source bag");
    assert_value_eq!(completed(on_bare.unwrap()), bytes_bag, "bytes bag");
}

/// The strict read of `package` under the library name it was compiled as.
fn read_named<'p>(package: &'p CheckedPackage, name: &str) -> qsl_package::AdmittedPackageRead<'p> {
    let bytes = serde_json::to_vec(&document()).unwrap();
    let domain_packages = qsl_semantics::model::intake::package_input([bytes.as_slice()]).unwrap();
    qsl_package::read_evaluation_package(
        package,
        qsl_semantics::library::LibraryName::new(name).unwrap(),
        &domain_packages,
        qsl_package::V2ReadLimits::default(),
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
#[trace("FR-533-AC-15", "FR-390-AC-66", "FR-531-AC-5", "TC-1333", "TC-1331")]
#[test]
fn every_witness_refusal_of_the_call_is_at_the_package_locus() {
    let package = fixture();
    let read = read_checked_package(&package);
    let v2 = read.admitted();
    let rows = bare_rows(&package, v2);
    let equality = qsl_package::emit_checked(&package)
        .expect("emits")
        .equality_witness(v2);
    let good = BarePackage::new(v2).with_supply(qsl_package::BareSupply {
        identities: &rows,
        equality: Some(&equality),
    });
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
    let other_equality = qsl_package::emit_checked(&other)
        .expect("emits")
        .equality_witness(other_read.admitted());
    let foreign = BarePackage::new(v2).with_supply(qsl_package::BareSupply {
        identities: &other_rows,
        equality: Some(&equality),
    });
    assert_eq!(
        call(&foreign, &on_bytes),
        package_refusal("foreign-identity-witness")
    );
    let mut conflicting = rows.declarations().to_vec();
    let first = conflicting[0].clone();
    let mut again = first.clone();
    again.original_identity = format!("{}-again", first.original_identity).into();
    conflicting.push(again);
    let unknown_rows =
        qsl_package::ReadIdentityRows::of_rows(v2, conflicting, rows.fields().to_vec());
    let unknown = BarePackage::new(v2).with_supply(qsl_package::BareSupply {
        identities: &unknown_rows,
        equality: Some(&equality),
    });
    assert_eq!(
        call(&unknown, &on_bytes),
        package_refusal("unknown-identity-witness")
    );
    // Equality witnesses.
    let supply_with = |equality| {
        BarePackage::new(v2).with_supply(qsl_package::BareSupply {
            identities: &rows,
            equality,
        })
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
    let equality = qsl_package::emit_checked(&package)
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
    let foreign = BarePackage::new(v2).with_supply(qsl_package::BareSupply {
        identities: &other_rows,
        equality: None,
    });
    assert_eq!(
        call(&foreign),
        (
            "foreign-identity-witness".to_owned(),
            CallAdmissionLocus::Package
        )
    );
    let stripped =
        qsl_package::ReadIdentityRows::of_rows(v2, rows.declarations().to_vec(), Vec::new());
    let no_fields = |equality| {
        BarePackage::new(v2).with_supply(qsl_package::BareSupply {
            identities: &stripped,
            equality,
        })
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
    let equality = qsl_package::emit_checked(&package)
        .expect("emits")
        .equality_witness(v2);
    let bare = BarePackage::new(v2).with_supply(qsl_package::BareSupply {
        identities: &rows,
        equality: Some(&equality),
    });
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
    let bare = BarePackage::new(v2).with_supply(qsl_package::BareSupply {
        identities: rows,
        equality: Some(equality),
    });
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

/// FR-520-AC-11, FR-533 "The call": the call owns the meter's interval
/// capture while it runs and leaves it as the caller had it, enabled or not.
#[trace("FR-520-AC-11", "FR-533-AC-9", "TC-1333")]
#[test]
fn a_bare_call_leaves_the_callers_charge_capture_as_it_found_it() {
    let package = fixture();
    let read = read_checked_package(&package);
    let v2 = read.admitted();
    let rows = bare_rows(&package, v2);
    let equality = qsl_package::emit_checked(&package)
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
    let equality = qsl_package::emit_checked(&package)
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
    let equality = qsl_package::emit_checked(&package)
        .expect("emits")
        .equality_witness(v2);
    let bare = BarePackage::new(v2).with_supply(qsl_package::BareSupply {
        identities: &rows,
        equality: Some(&equality),
    });
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
    let equality = qsl_package::emit_checked(&package)
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
    let without_rows = BarePackage::new(v2).with_supply(qsl_package::BareSupply {
        identities: &rows,
        equality: None,
    });
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
