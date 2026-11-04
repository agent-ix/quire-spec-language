// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-093-AC-39 to AC-42 (TC-1232): decimal constructors lowered to their
//! `decimal` literal nodes and operation nodes, and read by IR's v2 reader.

use ix_trace_rs::trace;

use super::converted_equality::emitted;
use super::*;
use crate::checked_v2::read_checked_package_v2;
use qsl_foundation::diagnostic::Staged;

/// The bodies of the emitted package's semantic nodes.
fn bodies(emission: &Emission) -> Vec<Value> {
    let package: Value =
        serde_json::from_slice(emission.package.bytes()).expect("the package bytes are JSON");
    package["semantic_graph"]["nodes"]
        .as_array()
        .expect("a semantic graph of nodes")
        .iter()
        .map(|node| node["body"].clone())
        .collect()
}

/// The `(value, type)` of every `decimal` literal node, in graph order.
fn decimal_literals(emission: &Emission) -> Vec<(String, Value)> {
    bodies(emission)
        .into_iter()
        .filter(|body| body["term"] == "literal" && body["value_kind"] == "decimal")
        .map(|body| {
            (
                body["value"].as_str().expect("a spelled value").to_owned(),
                body["type"].clone(),
            )
        })
        .collect()
}

/// The application nodes of the operation `identity`.
fn applications(emission: &Emission, identity: &str) -> Vec<Value> {
    bodies(emission)
        .into_iter()
        .filter(|body| body["operation"]["identity"] == identity)
        .collect()
}

/// The path of a `reference` argument's target, as text.
fn target(argument: &Value) -> String {
    assert_eq!(argument["term"], "reference");
    argument["target"].to_string()
}

/// IR's checked-package/v2 reader admits `emission`.
fn admitted(emission: &Emission) {
    read_checked_package_v2(
        emission.package.bytes(),
        library(),
        V2ReadLimits::default(),
        &read_evidence(emission),
    )
    .map(Staged::into_value)
    .unwrap_or_else(|refusal| panic!("IR's v2 reader refuses the package: {refusal:?}"));
}

const Q: &str = "type Q = Decimal[-99999, 99999; 0, 2; nearest-even];\n";

/// FR-093-AC-39: the six constructors lower one `decimal` literal each, spelled
/// as the AC lists; two functions over `decimal(125, 2)` share one literal node
/// while `decimal(1250, 3)` is another; every literal is typed by `Q`'s node.
#[trace("FR-093-AC-39", "TC-1232")]
#[test]
fn the_constructors_lower_to_decimal_literals_spelled_with_their_scale() {
    let mut declarations = String::from(Q);
    for (name, body) in [
        ("c1", "decimal(125, 2)"),
        ("c1b", "decimal(125, 2)"),
        ("c2", "decimal(5, 2)"),
        ("c3", "decimal(10, 0)"),
        ("c4", "decimal(0, 2)"),
        ("c5", "decimal(-5, 1)"),
        ("c6", "decimal(1250, 3)"),
    ] {
        declarations.push_str(&format!("function {name} using v(): Q pure {{ {body} }}\n"));
    }
    let emission = emitted(&declarations);
    let literals = decimal_literals(&emission);
    let mut values: Vec<&str> = literals.iter().map(|(value, _)| value.as_str()).collect();
    values.sort_unstable();
    assert_eq!(
        values,
        ["-0.5", "0.00", "0.05", "1.25", "1.250", "10"],
        "one literal node per distinct constructor; c1 and c1b share one, c1 and c6 are two"
    );
    assert!(
        literals.iter().all(|(_, ty)| *ty == literals[0].1),
        "each literal is typed at Q's node"
    );
    admitted(&emission);
}

/// FR-093-AC-39: `-decimal(5, 0)` lowers one `quire.op.decimal.negate` node
/// over the literal `"5"`; `p + decimal(125, 2)` lowers the literal typed `Q`
/// and one `quire.op.decimal.add` node over `[ref(p), ref(the literal)]`.
#[trace("FR-093-AC-39", "TC-1232")]
#[test]
fn a_negation_and_a_sum_lower_to_their_operation_nodes() {
    let emission = emitted(&format!(
        "{Q}function n using v(): Q pure {{ -decimal(5, 0) }}\n\
         function add using v(p: Q): Q pure {{ p + decimal(125, 2) }}\n"
    ));
    let negate = applications(&emission, "quire.op.decimal.negate");
    let [negate] = negate.as_slice() else {
        panic!("one negate node, got {negate:?}");
    };
    let operands = negate["arguments"].as_array().expect("arguments");
    assert_eq!(operands.len(), 1);
    assert!(target(&operands[0]).contains("\"5\""), "{operands:?}");

    let add = applications(&emission, "quire.op.decimal.add");
    let [add] = add.as_slice() else {
        panic!("one add node, got {add:?}");
    };
    let operands = add["arguments"].as_array().expect("arguments");
    assert_eq!(operands.len(), 2);
    assert!(target(&operands[0]).contains("\"p\""), "{operands:?}");
    assert!(target(&operands[1]).contains("\"1.25\""), "{operands:?}");
    let literals = decimal_literals(&emission);
    let literal = literals
        .iter()
        .find(|(value, _)| value == "1.25")
        .expect("the 1.25 literal");
    assert_eq!(add["result_type"], literal.1, "the literal is typed Q");
    admitted(&emission);
}

/// FR-093-AC-40: a member by value keeps the scale as written in its literal.
#[trace("FR-093-AC-40", "TC-1232")]
#[test]
fn a_member_by_value_keeps_its_written_scale() {
    let emission = emitted(
        "function a using v(): Decimal[-9, 9; 0, 0; toward-zero] pure { decimal(100, 2) }\n\
         function b using v(): Decimal[-1000, 1000; 0, 1; nearest-even] pure { decimal(1200, 3) }\n\
         function c using v(): Decimal[-9, 9; 0, 0; toward-zero] pure { decimal(0, 7) }\n",
    );
    let mut values: Vec<String> = decimal_literals(&emission)
        .into_iter()
        .map(|(value, _)| value)
        .collect();
    values.sort();
    assert_eq!(values, ["0.0000000", "1.00", "1.200"]);
    admitted(&emission);
}

/// FR-093-AC-41: each quotient by a nonzero literal lowers one
/// `quire.op.decimal.div` node over `[ref(a), ref(the literal)]` typed `W`.
#[trace("FR-093-AC-41", "TC-1232")]
#[test]
fn a_quotient_by_a_nonzero_literal_lowers_one_div_node() {
    let emission = emitted(
        "type W = Decimal[-99999, 99999; 2, 2; nearest-even];\n\
         function x1 using v(a: W): W pure { a / decimal(4, 0) }\n\
         function x2 using v(a: W): W pure { a / decimal(400, 2) }\n\
         function x3 using v(a: W): Boolean pure { a / decimal(4, 0) = a }\n",
    );
    let divisions = applications(&emission, "quire.op.decimal.div");
    assert_eq!(
        divisions.len(),
        2,
        "x1 and x3 divide by the same literal and share one node; x2 divides by 4.00"
    );
    let literals = decimal_literals(&emission);
    for division in &divisions {
        let operands = division["arguments"].as_array().expect("arguments");
        assert_eq!(operands.len(), 2);
        assert!(target(&operands[0]).contains("\"a\""), "{operands:?}");
        assert!(
            ["\"4\"", "\"4.00\""]
                .iter()
                .any(|literal| target(&operands[1]).contains(literal)),
            "{operands:?}"
        );
        assert_eq!(division["result_type"], literals[0].1, "typed W");
    }
    admitted(&emission);
}

/// FR-093-AC-42: `convert<P>(decimal(15, 1) + r)` types the literal `R` by its
/// peer `r` (a `decimal` literal node `"1.5"` at `R`'s type node), the sum `R`,
/// and lowers one `convert` node over the sum whose `result_type` is `P`.
#[trace("FR-093-AC-42", "TC-1232")]
#[test]
fn a_literal_beside_a_typed_operand_lowers_at_the_peers_type() {
    // `p_literal` pins P's own type node: its literal is typed by P.
    let emission = emitted(
        "type P = Decimal[0, 1000; 2, 2; exact];\n\
         type R = Decimal[0, 1000; 1, 1; exact];\n\
         function f using v(r: R): P pure { convert<P>(decimal(15, 1) + r) }\n\
         function p_literal using v(): P pure { decimal(150, 2) }\n",
    );
    let literals = decimal_literals(&emission);
    let type_of = |text: &str| {
        literals
            .iter()
            .find(|(value, _)| value == text)
            .unwrap_or_else(|| panic!("the {text} literal in {literals:?}"))
            .1
            .clone()
    };
    let (r_type, p_type) = (type_of("1.5"), type_of("1.50"));
    assert_ne!(r_type, p_type, "R and P are two type nodes");
    let sums = applications(&emission, "quire.op.decimal.add");
    let [sum] = sums.as_slice() else {
        panic!("one sum, got {sums:?}");
    };
    assert_eq!(sum["result_type"], r_type, "the sum is typed R");
    let converts = applications(&emission, "quire.op.numeric.convert");
    let [convert] = converts.as_slice() else {
        panic!("one convert node, got {converts:?}");
    };
    assert_eq!(
        convert["result_type"], p_type,
        "the conversion's result_type is P's type node"
    );
    assert_eq!(convert["arguments"].as_array().map(Vec::len), Some(1));
    admitted(&emission);
}
