// SPDX-License-Identifier: AGPL-3.0-or-later
//! Bounded nominal quantities retain their exact magnitude interval.
use super::*;
use quire_exact::{BoundedQuantityType, PathSegment, Quantity, Rational};

fn rational(numerator: i64, denominator: i64) -> Rational {
    Rational::new(Integer::from(numerator), Integer::from(denominator))
        .expect("nonzero denominator")
}

fn bounded(unit: UnitId, min: Option<Rational>, max: Option<Rational>) -> ValueType {
    ValueType::BoundedQuantity(std::sync::Arc::new(
        BoundedQuantityType::new(
            unit.clone(),
            min.map(|value| Quantity::new(value, unit.clone())),
            max.map(|value| Quantity::new(value, unit)),
        )
        .expect("bounds use the declared unit"),
    ))
}

#[trace("TC-413", "FR-092-AC-28")]
#[test]
fn bounded_quantity_identity_retains_both_exact_endpoints_and_unit() {
    let metre = UnitId::declared(node_key(METRE));
    let value_type = |unit, lo, hi| bounded(unit, Some(lo), Some(hi));
    let types = [
        value_type(metre.clone(), rational(-3, 2), rational(7, 4)),
        value_type(metre.clone(), rational(-6, 4), rational(14, 8)),
        value_type(metre.clone(), rational(-1, 2), rational(7, 4)),
        value_type(metre, rational(-3, 2), rational(8, 4)),
        value_type(
            UnitId::declared(node_key(SECOND)),
            rational(-3, 2),
            rational(7, 4),
        ),
        value_type(
            UnitId::declared(node_key(METRE)),
            rational(0, 1),
            rational(20, 1),
        ),
    ];
    let (keys, graph) = lower(&[], units(), |lowering| {
        types.each_ref().map(|ty| {
            lowering
                .type_node(ty, &generated_location())
                .expect("bounded quantity lowers")
        })
    });
    assert_eq!(
        keys[0], keys[1],
        "equivalent reduced endpoints share identity"
    );
    for key in &keys[2..] {
        assert_ne!(&keys[0], key);
    }
    let node = graph.node(keys[0].clone()).expect("bounded node");
    assert_eq!(node.node_tag(), NodeTag::ScalarType);
    assert_eq!(node.semantic_form(), "bounded_quantity");
    assert!(node.semantic_type().is_none(), "self typed");
    assert!(node.owner().is_none() && node.declaration().is_none() && node.recursion().is_none());
    let rational = graph
        .nodes()
        .find(|node| node.semantic_form() == "rational")
        .expect("rational scalar")
        .key();
    let as_ref = |key| node_ref(&NodeRef::Admitted(key));
    let expected = json!({"term": "aggregate", "members": [
        {"term": "binding", "name": "unit", "value": {"term": "reference", "target": as_ref(node_key(METRE))}},
        {"term": "binding", "name": "min", "value": {"term": "literal", "type": as_ref(rational.clone()), "value_kind": "rational", "value": "-3/2"}},
        {"term": "binding", "name": "max", "value": {"term": "literal", "type": as_ref(rational.clone()), "value_kind": "rational", "value": "7/4"}}
    ]});
    assert_eq!(preimage(node)["body"], expected);
    assert_eq!(
        keys[0].path(),
        vec![
            PathSegment::Text("scalar_type".into()),
            PathSegment::Text("bounded_quantity".into()),
            PathSegment::Node(node_key(METRE)),
            PathSegment::Text("min".into()),
            PathSegment::Node(rational.clone()),
            PathSegment::Text("-3/2".into()),
            PathSegment::Text("max".into()),
            PathSegment::Node(rational),
            PathSegment::Text("7/4".into()),
        ]
    );
    let integer_path = keys[5].path();
    assert_eq!(
        integer_path[4], integer_path[7],
        "both exact bounds retain their rational type"
    );
    assert_eq!(integer_path[5], PathSegment::Text("0".into()));
    assert_eq!(integer_path[8], PathSegment::Text("20".into()));
}

#[trace("TC-413", "FR-092-AC-28")]
#[test]
fn bounded_quantity_missing_reversed_or_compound_endpoints_are_refused() {
    let metre = UnitId::declared(node_key(METRE));
    let types = [
        bounded(metre.clone(), None, Some(rational(2, 1))),
        bounded(metre.clone(), Some(rational(0, 1)), None),
        bounded(metre, Some(rational(2, 1)), Some(rational(0, 1))),
        bounded(
            UnitId::compound(vec![]),
            Some(rational(0, 1)),
            Some(rational(2, 1)),
        ),
    ];
    let (_, graph) = lower(&[], units(), |lowering| {
        for ty in &types {
            let refusal = lowering
                .type_node(ty, &generated_location())
                .expect_err("unsupported bounded form");
            assert_eq!(
                refusal.cause,
                CheckCause::IllTyped(quire_exact::IllTypedCause::TypeMismatch)
            );
        }
    });
    assert!(!graph
        .nodes()
        .any(|node| node.semantic_form() == "bounded_quantity"));
}

#[trace("TC-413", "FR-093-AC-61")]
#[test]
fn bounded_quantity_add_and_less_than_preserve_operand_types() {
    let ty = bounded(
        UnitId::declared(node_key(METRE)),
        Some(rational(0, 1)),
        Some(rational(2, 1)),
    );
    let package = check_declarations(PackageDeclarations {
        types: TypeEnvironment::default().with_units(units()),
        aliases: vec![
            ("Bounded".into(), ty.clone()),
            ("Alias".into(), ty),
            ("Length".into(), quantity(METRE)),
        ],
        functions: vec![
            function(
                "add",
                &[("a", named("Bounded")), ("b", named("Alias"))],
                named("Length"),
                Expression::binary(BinaryOperator::Add, name("a"), name("b")),
            ),
            function(
                "lt",
                &[("a", named("Bounded")), ("b", named("Bounded"))],
                builtin(BuiltinType::Boolean),
                Expression::binary(BinaryOperator::Less, name("a"), name("b")),
            ),
        ],
        ..PackageDeclarations::new(
            fixture_source(),
            "fixture",
            qsl_foundation::IdentityLimits::default(),
        )
    });
    let graph = package.semantic_graph();
    let adds = applications(graph, "quire.op.quantity.add");
    assert_eq!(adds.len(), 1);
    assert_eq!(
        preimage(adds[0])["body"]["result_type"],
        node_ref(&NodeRef::Admitted(node_key(METRE)))
    );
    assert_eq!(applications(graph, "quire.op.quantity.lt").len(), 1);
    let bounded_key = graph
        .nodes()
        .find(|node| node.semantic_form() == "bounded_quantity")
        .expect("bounded operand")
        .key();
    assert_eq!(parameter(graph, "a", 0).semantic_type(), Some(bounded_key));
}
