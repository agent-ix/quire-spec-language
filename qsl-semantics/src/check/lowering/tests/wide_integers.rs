// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-910 step 1 (FR-092-AC-14) and TC-909 step 4 (FR-091-AC-38): `Int`
//! ranges up to the i128 extremes each give a type node spelling its bounds
//! as decimal strings, and the folded `i128::MIN` literal is one literal node.

use super::*;
use crate::check::{CheckCause, I128Limit, IntegerOutsideI128, IntegerSite};

fn wide(lower: i128, upper: i128) -> ValueType {
    ValueType::Int(IntegerInterval::new(Integer::from(lower), Integer::from(upper)).unwrap())
}

/// The `Int[lower, upper]` type form with the bounds as written.
fn wide_form(lower: &str, upper: &str) -> TypeForm {
    TypeForm::builtin(BuiltinType::Int, SPAN).with_bounds(vec![lower.to_owned(), upper.to_owned()])
}

/// `g(x: Int[i128::MIN, 0]): Boolean { x >= <literal> }`.
fn compared_with(literal: Integer) -> Result<CheckedGraph, Vec<CheckRefusal>> {
    check(vec![function(
        "g",
        &[(
            "x",
            wide_form("-170141183460469231731687303715884105728", "0"),
        )],
        boolean(),
        None,
        binary(
            BinaryOperator::GreaterOrEqual,
            name_expr("x"),
            Expression::integer(literal),
        ),
    )])
}

/// FR-092-AC-14: the type nodes of the wide `Int` ranges are distinct, beside
/// `Int[0, 9]`, and every bound is a decimal string.
#[trace("FR-092-AC-14", "TC-910")]
#[test]
fn wide_integer_ranges_give_distinct_nodes_with_decimal_string_bounds() {
    let cases = [
        wide(0, i128::from(u64::MAX)),
        wide(0, 9_223_372_036_854_775_808),
        wide(i128::MIN, i128::MAX),
        wide(0, 18_446_744_073_709_551_616),
        int(0, 9),
    ];
    let (graph, keys) = type_nodes(
        &empty_scope(),
        &SourceOwner::from(&fixture_source()),
        &cases,
    );
    let distinct: BTreeSet<NodeKey> = keys.iter().copied().collect();
    assert_eq!(distinct.len(), cases.len(), "each range has its own node");
    let t15 = std::str::from_utf8(graph.node(keys[2]).unwrap().preimage()).unwrap();
    for bound in [
        "\"-170141183460469231731687303715884105728\"",
        "\"170141183460469231731687303715884105727\"",
    ] {
        assert!(t15.contains(bound), "the range spells {bound} as a string: {t15}");
    }
    let t13 = std::str::from_utf8(graph.node(keys[0]).unwrap().preimage()).unwrap();
    assert!(t13.contains("\"18446744073709551615\""), "{t13}");
}

/// FR-092-AC-14: with those parameters in one checked unit, each parameter's
/// type is its range's node and `Int[0, 9]` keeps the node it has alone.
#[trace("FR-092-AC-14", "TC-910")]
#[test]
fn wide_integer_parameters_keep_their_nodes_in_one_unit() {
    let g = function(
        "g",
        &[
            ("a", wide_form("0", "18446744073709551615")),
            ("b", wide_form("0", "9223372036854775808")),
            (
                "c",
                wide_form(
                    "-170141183460469231731687303715884105728",
                    "170141183460469231731687303715884105727",
                ),
            ),
            ("d", wide_form("0", "18446744073709551616")),
            ("e", int_form(0, 9)),
        ],
        boolean(),
        None,
        Expression::boolean(true),
    );
    let graph = check(vec![g]).expect("g checks");
    let types: Vec<NodeKey> = ["a", "b", "c", "d", "e"]
        .iter()
        .map(|name| {
            parameter_named(graph.semantic_graph(), name)
                .semantic_type()
                .expect("a typed parameter")
        })
        .collect();
    assert_eq!(types.iter().copied().collect::<BTreeSet<_>>().len(), 5);
    let alone = check(vec![function(
        "g",
        &[("e", int_form(0, 9))],
        boolean(),
        None,
        Expression::boolean(true),
    )])
    .expect("g checks");
    assert_eq!(
        parameter_named(alone.semantic_graph(), "e").semantic_type(),
        Some(types[4]),
        "the other wide parameters leave Int[0, 9] unchanged"
    );
}

/// FR-091-AC-38: the literal `i128::MIN` is one literal node with its value
/// spelled as a string.
#[trace("FR-091-AC-38", "TC-909")]
#[test]
fn the_i128_minimum_literal_is_one_node_with_a_string_value() {
    let graph = compared_with(Integer::from(i128::MIN)).expect("i128::MIN checks");
    let literals: Vec<Json> = graph
        .semantic_graph()
        .nodes()
        .filter(|node| node.semantic_form() == "literal")
        .map(preimage)
        .filter(|json| json["body"]["value"] == json!("-170141183460469231731687303715884105728"))
        .collect();
    assert_eq!(literals.len(), 1);
}

/// FR-091-AC-38: a literal one past either i128 limit refuses
/// `IntegerOutsideI128` at the literal, naming the limit it crosses.
#[trace("FR-091-AC-38", "TC-909")]
#[test]
fn a_literal_outside_i128_refuses_naming_the_limit_it_crosses() {
    let above = Integer::from(i128::MAX).add(&Integer::one());
    let below = Integer::from(i128::MIN).sub(&Integer::one());
    for (literal, limit) in [(above, I128Limit::Max), (below, I128Limit::Min)] {
        let refusals = compared_with(literal.clone()).expect_err("the literal is outside i128");
        assert_eq!(refusals.len(), 1, "{literal}");
        assert_eq!(
            refusals[0].cause,
            CheckCause::IntegerOutsideI128(IntegerOutsideI128 {
                site: IntegerSite::Literal,
                value: literal.to_string(),
                limit,
            })
        );
        assert_eq!(refusals[0].cause.code().as_str(), "ill_typed");
        assert_eq!(refusals[0].cause.cause(), Some("type-mismatch"));
    }
}
