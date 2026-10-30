// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-093: a present option is an identity-bound `value/option_value` whose
//! named payload reference survives emission and the full I2 read.

use super::*;

/// FR-093-AC-48, FR-140-AC-11: `Some(x)` is a value constructor, not an
/// application of an operator or a package function.
#[trace("TC-1273", "FR-093-AC-48", "FR-140-AC-11")]
#[test]
fn some_emits_an_identity_bound_option_value_and_reads_back() {
    let integer = TypeForm::builtin(BuiltinType::Integer, SPAN);
    let optional_integer =
        TypeForm::builtin(BuiltinType::Option, SPAN).with_arguments(vec![integer.clone()]);
    let present = FunctionDeclaration::new(
        "present",
        vec![("x".to_owned(), integer)],
        optional_integer,
        None,
        Expression::call("Some".to_owned(), vec![name("x")]),
    );
    let emission = emit(&package(vec![present]));
    assert_eq!(emission.omitted, [], "nothing is omitted");
    let wire = wire(&emission);
    let options: Vec<_> = nodes(&wire)
        .iter()
        .filter(|node| node["semantic_form"] == "option_value")
        .collect();
    let [option] = options.as_slice() else {
        panic!(
            "one identity-bound option_value node, got {}",
            options.len()
        );
    };
    assert_eq!(option["node_tag"], "value");
    assert_eq!(option["body"]["term"], "aggregate");
    assert_eq!(option["body"]["members"][0]["name"], "value");
    assert_eq!(option["body"]["members"][0]["value"]["term"], "reference");
    assert!(option["body"]["members"][0]["value"]["target"].is_object());
    assert!(nodes(&wire)
        .iter()
        .all(|node| { node["body"]["operation"]["identity"] != "quire.op.option.some" }));
    let Read::Verified { .. } = read_back(&emission) else {
        panic!("FR-264 admits the emitted package");
    };
}
