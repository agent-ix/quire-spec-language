// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1336 step 1: the checked-package bytes of each quantity-power function
//! of FR-093-AC-58 read back through IR's strict v2 reader, and the
//! `quire.op.quantity.pow` nodes they carry have the operands and result
//! types the criterion names (QSpec TC-187 QP-01 to QP-03).

use std::path::Path;

use ix_trace_rs::trace;
use serde_json::Value;

use crate::emit_checked_v2::{producer_bytes, read_back};

const UNITS: &str = "\
    dimension Time;\n\
    unit s : Time = rational(1, 1);\n\
    unit ms : Time = rational(1, 1000) * s;\n\
    dimension Length;\n\
    unit m : Length = rational(1, 1);\n\
    dimension Area = Length ^ 2;\n\
    unit m2 : Area = rational(1, 1);\n";

/// The unit text with `function`, under the complete-V1 header.
fn unit(function: &str) -> String {
    format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n{UNITS}{function}\n"
    )
}

/// Every `quire.op.quantity.pow` application in `wire`, anywhere in it.
fn power_nodes<'w>(wire: &'w Value, found: &mut Vec<&'w Value>) {
    match wire {
        Value::Object(object) => {
            if object
                .get("operation")
                .is_some_and(|operation| operation["identity"] == "quire.op.quantity.pow")
                && object.get("term").is_some_and(|term| term == "application")
            {
                found.push(wire);
            }
            object.values().for_each(|value| power_nodes(value, found));
        }
        Value::Array(items) => items.iter().for_each(|value| power_nodes(value, found)),
        _ => {}
    }
}

/// The packages' power nodes, after each package reads back admitted.
fn powers_of(function: &str) -> Vec<Value> {
    let source = unit(function);
    let bytes = producer_bytes(Path::new("quantity-power.native"), &source, &[]);
    read_back(&bytes, &[]);
    let wire: Value = serde_json::from_slice(&bytes).expect("JSON");
    let mut found = Vec::new();
    power_nodes(&wire, &mut found);
    found.into_iter().cloned().collect()
}

/// The `path` of a wire reference.
fn path(reference: &Value) -> Vec<Value> {
    reference["target"]["path"]
        .as_array()
        .or_else(|| reference["path"].as_array())
        .expect("a keyed path")
        .clone()
}

/// FR-093-AC-58 w1, w2, w3, w5, w6 and w8 (TC-1336 step 1): each package is
/// admitted by IR's v2 reader. A power node is binary over `[ref(base),
/// ref(N)]` with no law, mode, member or leaves; its `result_type` is the
/// compound unit of the exponents times `n`: `m` squared for w1 and w2, the
/// dimensionless unit for w3, `s` to the power 1 for w5, `m` to the power -1
/// for w6.
#[trace("FR-093-AC-58", "TC-1336", "QSpec-TC-187")]
#[test]
fn each_quantity_power_package_reads_back_with_its_power_nodes_typed_by_the_result_unit() {
    // (function, exponent, base is a quantity literal, result unit exponent)
    let cases = [
        ("function w1 using v(): Boolean pure { 2 m ^ 2 = 2 m * 2 m }", "2", true, Some("2")),
        ("function w2 using v(): Boolean pure { convert<Quantity<m2>>(3 m ^ 2) = 9 m2 }", "2", true, Some("2")),
        ("function w3 using v(): Boolean pure { let d = 5 ms in d ^ 0 = 1 m / 1 m }", "0", false, None),
        ("function w5 using v(): Boolean pure { let d = 1500 ms in d ^ 1 = convert<Quantity<s>>(d) * (1 s / 1 s) }", "1", false, Some("1")),
        ("function w6 using v(): Boolean pure { 2 m ^ -1 = (1 m / 2 m) / 1 m }", "-1", true, Some("-1")),
        ("function w8 using v(): Boolean pure { 2 m ^ 18446744073709551616 = 2 m ^ 18446744073709551616 }", "18446744073709551616", true, Some("18446744073709551616")),
    ];
    let mut total = 0;
    for (function, exponent, literal_base, unit_exponent) in cases {
        let powers = powers_of(function);
        assert!(!powers.is_empty(), "{function}");
        total += powers.len();
        for power in &powers {
            assert_eq!(power["operator"], "binary", "{function}");
            let operation = &power["operation"];
            assert_eq!(operation["member"], Value::Null, "{function}");
            assert_eq!(operation["mode"], Value::Null, "{function}");
            assert_eq!(operation["laws"], serde_json::json!([]), "{function}");
            assert_eq!(operation["leaves"], serde_json::json!([]), "{function}");
            let arguments = power["arguments"].as_array().expect("arguments");
            assert_eq!(arguments.len(), 2, "{function}");
            if literal_base {
                assert_eq!(path(&arguments[0])[..2], ["value", "literal"], "{function}");
            }
            // The exponent is the `Integer` literal node of the exponent's value.
            let literal = path(&arguments[1]);
            assert_eq!(
                literal[..2],
                ["value", "literal"],
                "{function}: {literal:?}"
            );
            assert_eq!(literal[3], exponent, "{function}: {literal:?}");
            // The result type is the compound unit with each exponent times n.
            let result = path(&power["result_type"]);
            assert_eq!(result[..2], ["scalar_type", "compound_unit"], "{function}");
            match unit_exponent {
                Some(unit_exponent) => {
                    assert_eq!(result.len(), 4, "{function}: {result:?}");
                    assert_eq!(result[3], unit_exponent, "{function}: {result:?}");
                }
                None => assert_eq!(result.len(), 2, "{function}: {result:?}"),
            }
        }
    }
    assert!(total >= cases.len(), "{total} power nodes");
}
