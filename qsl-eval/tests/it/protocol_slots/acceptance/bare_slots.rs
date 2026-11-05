// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-533, TC-1333: the protocol-slot call of a package read from its bytes
//! alone returns the in-process route's value and charge log.

use super::*;
use qsl_eval::bare::BarePackage;

/// The bytes-route descriptor of the slot the source route selects with
/// `wanted`: the source selector reads a compensation node's slots from the
/// template at the node's position in the id-sorted keys, so the bytes
/// descriptor names the node whose declaration is the owner.
fn bytes_slot(
    v2: &qsl_package::CheckedPackageV2,
    parity: &ParitySlot<'_>,
    wanted: &ProtocolSlot,
) -> ProtocolSlot {
    let node = if parity.protocol == parity.owner {
        wanted.node.clone()
    } else {
        let declared = v2
            .graph()
            .nodes
            .iter()
            .find(|node| {
                node.declaration.as_ref().is_some_and(|d| {
                    d.qualified_name.last().map(|s| &**s) == Some(parity.owner)
                        && d.qualified_name.first().map(|s| &**s) == Some(parity.protocol)
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

/// FR-533-AC-1, AC-2, AC-9, AC-16 (evaluate halves): over every TC-1070 slot
/// that reads no observation, the bare route returns the source route's
/// value, outcome and ordered charge log, at the exact budget, one short of
/// each used counter, and with every binder unavailable; the integer
/// equality of `Refund`'s retry relation and recovery predicate is among
/// them and reads the equality rows of the handoff.
#[trace("FR-533-AC-1", "FR-533-AC-2", "FR-533-AC-16", "FR-531-AC-1", "TC-1333")]
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
        let on_bytes = bytes_slot(v2, spec, &wanted);
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
