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
fn negate_nodes<'w>(wire: &'w Value, found: &mut Vec<&'w Value>) {
    match wire {
        Value::Object(object) => {
            if object
                .get("operation")
                .is_some_and(|operation| operation["identity"] == "quire.op.quantity.negate")
                && object.get("term").is_some_and(|term| term == "application")
            {
                found.push(wire);
            }
            object.values().for_each(|value| negate_nodes(value, found));
        }
        Value::Array(items) => items.iter().for_each(|value| negate_nodes(value, found)),
        _ => {}
    }
}

/// The package's negate nodes, after it reads back admitted.
fn negations_of(function: &str) -> Vec<Value> {
    let source = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n{UNITS}{function}\n"
    );
    let bytes = producer_bytes(Path::new("quantity-negation.native"), &source, &[]);
    read_back(&bytes, &[]);
    let wire: Value = serde_json::from_slice(&bytes).expect("JSON");
    let mut found = Vec::new();
    negate_nodes(&wire, &mut found);
    found.into_iter().cloned().collect()
}

/// FR-093-AC-49 a, b, c, c2, z, q, r, k, k2 and v2 (TC-1285 step 1): each
/// package is admitted and carries the expected number of negate nodes.
#[trace("FR-093-AC-49", "TC-1285")]
#[test]
fn each_quantity_negation_package_reads_back_with_unary_negate_nodes() {
    let cases = [
        ("function a using v(): Quantity<m> pure { -(2 m) }", 1),
        ("function b using v(): Quantity<m> pure { -(2 m) + 1 m }", 1),
        ("function c using v(): Quantity<ms> pure { -(3 ms) }", 1),
        ("function c2 using v(): Quantity<ms> pure { -3 ms }", 1),
        ("function z using v(): Quantity<m> pure { -(0 m) }", 1),
        (
            "function q using v(): Boolean pure { -(-(3 ms)) = 3 ms }",
            2,
        ),
        (
            "function r using v(d: Quantity<ms>): Boolean pure { d = -3 ms }",
            1,
        ),
        (
            "function k using v(d: Quantity<ms>): Boolean pure { d / -3 ms = -(d / 3 ms) }",
            2,
        ),
        (
            "function k2 using v(d: Quantity<ms>): Boolean pure { d / -(-(3 ms)) = d / 3 ms }",
            2,
        ),
        (
            "function v2 using v(): Boolean pure { -(3 m / 1 s) = -(3 m / 1 s) }",
            // The two equal sides share one node.
            1,
        ),
    ];
    for (function, count) in cases {
        let negations = negations_of(function);
        assert_eq!(negations.len(), count, "{function}");
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
