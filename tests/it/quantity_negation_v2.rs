// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1285 step 1: the checked-package bytes of each quantity-negation
//! function of FR-093-AC-49 read back through IR's strict v2 reader, and the
//! `quire.op.quantity.negate` nodes they carry are unary with the operand's
//! unit as `result_type` and no law, mode, member or leaves.

use std::path::Path;

use ix_trace_rs::trace;
use serde_json::Value;

use crate::emit_checked_v2::{producer_bytes, read_back};

const UNITS: &str = "\
    dimension Time;\n\
    unit s : Time = rational(1, 1);\n\
    unit ms : Time = rational(1, 1000) * s;\n\
    dimension Length;\n\
    unit m : Length = rational(1, 1);\n";

/// Every `quire.op.quantity.negate` application in `wire`, anywhere in it.
fn applications<'w>(wire: &'w Value, identity: &str, found: &mut Vec<&'w Value>) {
    match wire {
        Value::Object(object) => {
            if object
                .get("operation")
                .is_some_and(|operation| operation["identity"] == identity)
                && object.get("term").is_some_and(|term| term == "application")
            {
                found.push(wire);
            }
            object
                .values()
                .for_each(|value| applications(value, identity, found));
        }
        Value::Array(items) => items
            .iter()
            .for_each(|value| applications(value, identity, found)),
        _ => {}
    }
}

/// The package's negate nodes, after it reads back admitted.
fn applications_of(function: &str, identity: &str) -> Vec<Value> {
    let source = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n{UNITS}{function}\n"
    );
    let bytes = producer_bytes(Path::new("quantity-negation.native"), &source, &[]);
    read_back(&bytes, &[]);
    let wire: Value = serde_json::from_slice(&bytes).expect("JSON");
    let mut found = Vec::new();
    applications(&wire, identity, &mut found);
    found.into_iter().cloned().collect()
}

/// The result type of `node`: its unit's name, or `compound` for a compound
/// unit node.
fn result_unit(node: &Value) -> String {
    let path = node["result_type"]["path"].as_array().expect("a path");
    match path[0].as_str() {
        Some("scalar_type") => {
            assert_eq!(path[1], "compound_unit", "{path:?}");
            "compound".to_owned()
        }
        Some(unit) => unit.to_owned(),
        None => panic!("{path:?}"),
    }
}

/// FR-093-AC-49 a, b, c, c2, z, q, r, k, k2 and v2 (TC-1285 step 1): each
/// package is admitted and carries the expected negate nodes, unary with the
/// operand's unit node as `result_type` and no law, mode, member or leaves.
#[trace("FR-093-AC-49", "TC-1285")]
#[test]
fn each_quantity_negation_package_reads_back_with_unary_negate_nodes() {
    // (function, the result unit of each negate node, in sorted order). `v2`'s
    // two equal sides are content-addressed to one node.
    let cases = [
        (
            "function a using v(): Quantity<m> pure { -(2 m) }",
            vec!["m"],
        ),
        (
            "function b using v(): Quantity<m> pure { -(2 m) + 1 m }",
            vec!["m"],
        ),
        (
            "function c using v(): Quantity<ms> pure { -(3 ms) }",
            vec!["ms"],
        ),
        (
            "function c2 using v(): Quantity<ms> pure { -3 ms }",
            vec!["ms"],
        ),
        (
            "function z using v(): Quantity<m> pure { -(0 m) }",
            vec!["m"],
        ),
        (
            "function q using v(): Boolean pure { -(-(3 ms)) = 3 ms }",
            vec!["ms", "ms"],
        ),
        (
            "function r using v(d: Quantity<ms>): Boolean pure { d = -3 ms }",
            vec!["ms"],
        ),
        (
            "function k using v(d: Quantity<ms>): Boolean pure { d / -3 ms = -(d / 3 ms) }",
            vec!["compound", "ms"],
        ),
        (
            "function k2 using v(d: Quantity<ms>): Boolean pure { d / -(-(3 ms)) = d / 3 ms }",
            vec!["ms", "ms"],
        ),
        (
            "function v2 using v(): Boolean pure { -(3 m / 1 s) = -(3 m / 1 s) }",
            vec!["compound"],
        ),
    ];
    for (function, expected) in cases {
        let negations = applications_of(function, "quire.op.quantity.negate");
        let mut units: Vec<String> = negations.iter().map(result_unit).collect();
        units.sort();
        assert_eq!(units, expected, "{function}");
        for negation in &negations {
            assert_eq!(negation["operator"], "unary", "{function}");
            let operation = &negation["operation"];
            assert_eq!(operation["member"], Value::Null, "{function}");
            assert_eq!(operation["mode"], Value::Null, "{function}");
            assert_eq!(operation["laws"], serde_json::json!([]), "{function}");
            assert_eq!(operation["leaves"], serde_json::json!([]), "{function}");
            assert_eq!(
                negation["arguments"].as_array().map(Vec::len),
                Some(1),
                "{function}"
            );
        }
    }
}

/// FR-093-AC-49 c and c2: `-3 ms` and `-(3 ms)` lower to the same nodes.
#[trace("FR-093-AC-49", "TC-1285")]
#[test]
fn a_prefix_minus_literal_and_a_negated_literal_lower_to_the_same_nodes() {
    let negate = |function| applications_of(function, "quire.op.quantity.negate");
    assert_eq!(
        negate("function c using v(): Quantity<ms> pure { -(3 ms) }"),
        negate("function c2 using v(): Quantity<ms> pure { -3 ms }")
    );
}

/// FR-093-AC-49 v2: the negate node's `result_type` is the quotient's own
/// compound-unit node (`m` to the power 1, `s` to the power -1).
#[trace("FR-093-AC-49", "TC-1285")]
#[test]
fn a_compound_negation_is_typed_by_the_quotients_compound_unit_node() {
    let function = "function v2 using v(): Boolean pure { -(3 m / 1 s) = -(3 m / 1 s) }";
    let negations = applications_of(function, "quire.op.quantity.negate");
    let quotients = applications_of(function, "quire.op.quantity.div");
    assert_eq!((negations.len(), quotients.len()), (1, 1));
    assert_eq!(negations[0]["result_type"], quotients[0]["result_type"]);
    let path = negations[0]["result_type"]["path"]
        .as_array()
        .expect("a path");
    assert_eq!(path[..2], ["scalar_type", "compound_unit"]);
}

/// FR-093-AC-49 r: the comparison is one `quire.op.quantity.eq` over `d` and
/// the negate node of the `3 ms` literal.
#[trace("FR-093-AC-49", "TC-1285")]
#[test]
fn a_comparison_with_a_negated_literal_is_a_quantity_eq_over_the_negate_node() {
    let function = "function r using v(d: Quantity<ms>): Boolean pure { d = -3 ms }";
    let equalities = applications_of(function, "quire.op.quantity.eq");
    assert_eq!(equalities.len(), 1);
    let arguments = equalities[0]["arguments"].as_array().expect("arguments");
    assert_eq!(arguments.len(), 2);
    let parameter = &arguments[0]["target"]["path"];
    assert_eq!(parameter[0], "value");
    assert_eq!(parameter[1], "parameter");
    let negate = &arguments[1]["target"]["path"];
    assert_eq!(negate[0], "expression");
    assert_eq!(negate[1], "unary");
    assert_eq!(negate[3], "quire.op.quantity.negate");
}
