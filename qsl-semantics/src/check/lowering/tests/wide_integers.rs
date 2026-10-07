// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-910 step 1 (FR-092-AC-14) and TC-909 step 4 (FR-091-AC-38): `Int`
//! ranges up to the i128 extremes key to FR-092's T13 to T16, and the folded
//! `i128::MIN` literal keys to L7. The vectors are the ones FR-092 publishes.

use super::*;
use crate::check::{CheckCause, I128Limit, IntegerOutsideI128, IntegerSite};

fn wide(lower: i128, upper: i128) -> ValueType {
    ValueType::Int(IntegerInterval::new(Integer::from(lower), Integer::from(upper)).unwrap())
}

/// The `Int[lower, upper]` type form with the bounds as written.
fn wide_form(lower: &str, upper: &str) -> TypeForm {
    TypeForm::builtin(BuiltinType::Int, SPAN).with_bounds(vec![lower.to_owned(), upper.to_owned()])
}

/// FR-092-AC-14: the type nodes of the wide `Int` ranges are T13 to T16
/// byte for byte, beside T4, and every bound is a decimal string.
#[trace("FR-092-AC-14", "TC-910")]
#[test]
fn wide_integer_ranges_key_to_t13_to_t16() {
    let cases = [
        ("T13", wide(0, i128::from(u64::MAX))),
        ("T14", wide(0, 9_223_372_036_854_775_808)),
        ("T15", wide(i128::MIN, i128::MAX)),
        ("T16", wide(0, 18_446_744_073_709_551_616)),
        ("T4", int(0, 9)),
    ];
    let (graph, keys) = type_nodes(
        &empty_scope(),
        &SourceOwner::from(&fixture_source()),
        &cases
            .iter()
            .map(|(_, value_type)| value_type.clone())
            .collect::<Vec<_>>(),
    );
    for ((name, _), key) in cases.iter().zip(&keys) {
        assert_vector(&graph, *key, name);
    }
    let t15 = std::str::from_utf8(graph.node(keys[2]).unwrap().preimage()).unwrap();
    for bound in [
        "\"-170141183460469231731687303715884105728\"",
        "\"170141183460469231731687303715884105727\"",
    ] {
        assert!(t15.contains(bound), "T15 spells {bound} as a string: {t15}");
    }
}

/// FR-092-AC-14: with those parameters in one checked unit, each parameter's
/// type is its vector, `Int[0, 9]` included.
#[trace("FR-092-AC-14", "TC-910")]
#[test]
fn wide_integer_parameters_key_to_their_vectors_in_one_unit() {
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
    for (parameter, vector) in [
        ("a", "T13"),
        ("b", "T14"),
        ("c", "T15"),
        ("d", "T16"),
        ("e", "T4"),
    ] {
        assert_eq!(
            parameter_named(graph.semantic_graph(), parameter)
                .semantic_type()
                .map(|key| key.to_string()),
            Some(vector_key(vector)),
            "{parameter}"
        );
    }
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

/// FR-091-AC-38: the literal `i128::MIN` is one literal node, L7, with its
/// value spelled as a string.
#[trace("FR-091-AC-38", "TC-909")]
#[test]
fn the_i128_minimum_literal_keys_to_l7() {
    let graph = compared_with(Integer::from(i128::MIN)).expect("i128::MIN checks");
    let semantic = graph.semantic_graph();
    let key = node_by_key(semantic, &vector_key("L7")).key();
    assert_vector(semantic, key, "L7");
    assert_eq!(
        preimage(semantic.node(key).unwrap())["body"]["value"],
        json!("-170141183460469231731687303715884105728")
    );
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
