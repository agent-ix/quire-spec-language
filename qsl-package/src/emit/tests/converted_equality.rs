// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-093-AC-3 (TC-415): an equality operand the checker converts
//! (`convert<T>(e) = x`) lowers as a typed `numeric.convert` node, and the
//! equality takes its operator family from the converted type, so IR's
//! checked-package/v2 reader admits the package.

use super::*;

const HEADER: &str = "language \"ix:native\" edition \"1-draft\";\n\
                      profile v = \"quire.value.complete/v1\";\n";

const DEC: &str = "Decimal[-100,100;0,0;nearest-even]";
const INT: &str = "Int[-100,100]";
const WIDE: &str = "Int[-200,200]";
const DEC2: &str = "Decimal[-100,100;0,2;nearest-even]";
const RAT: &str = "Rational[-100, 100; 1, 1]";

/// `declarations` through S1 to S4 and the v2 emitter, with nothing omitted.
fn emitted(declarations: &str) -> Emission {
    let text = format!("{HEADER}{declarations}\n");
    let parsed = qsl_cst::parse(
        qsl_foundation::SourceIdentity::new("agent-ix", "test:conv-eq", "fixture", "fixture:1"),
        "program.native",
        text.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 admits the unit");
    assert_eq!(parsed.diagnostics(), []);
    let raw = parsed.source().reference().clone();
    let unit = qsl_forms::build_unit(&parsed).expect("S2 builds the unit");
    let graph = PackageDeclarations::assemble(raw, unit, Vec::new(), Vec::new())
        .expect("the unit assembles")
        .check(CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("the package checks: {refusals:?}"));
    let emission = emit_checked(&CheckedPackage::link(graph)).expect("the package emits");
    assert_eq!(emission.omitted, []);
    emission
}

/// The application nodes of operation `identity` in `wire`.
fn applications<'w>(wire: &'w Value, identity: &str) -> Vec<&'w Value> {
    nodes(wire)
        .iter()
        .filter(|node| node["body"]["operation"]["identity"] == identity)
        .collect()
}

/// The node `reference` term `term` names.
fn referenced<'w>(wire: &'w Value, term: &Value) -> &'w Value {
    let digest = term["target"]["digest"].as_str().expect("a reference");
    nodes(wire)
        .iter()
        .find(|node| node["node_id"]["digest"] == digest)
        .expect("the referenced node")
}

/// `function f using v(a: {source}, b: {other}): Boolean pure { body }`
/// emitted and read back by IR's strict reader, which must admit it. Returns
/// the wire.
fn admitted(source: &str, other: &str, body: &str) -> Value {
    let emission = emitted(&format!(
        "function f using v(a: {source}, b: {other}): Boolean pure {{ {body} }}"
    ));
    match read_back(&emission) {
        Read::Verified { .. } => {}
        refused => panic!("{body}: the strict reader must admit it: {refused:?}"),
    }
    wire(&emission)
}

/// `body` lowers to one `numeric.convert` over the parameter `a`, typed
/// at the type node of `b`, and one `quire.op.<family>.<suffix>` whose
/// argument at `converted_at` is that node and whose other argument is `b`.
fn assert_converted(
    source: &str,
    other: &str,
    body: &str,
    family: &str,
    suffix: &str,
    converted_at: usize,
) {
    let wire = admitted(source, other, body);
    let converts = applications(&wire, "quire.op.numeric.convert");
    assert_eq!(converts.len(), 1, "{body}: one convert node");
    let convert = converts[0];
    let source = referenced(&wire, &convert["body"]["arguments"][0]);
    assert_eq!(source["semantic_form"], "parameter", "{body}");
    assert_eq!(
        source["body"]["members"][0]["value"]["value"], "a",
        "{body}: the convert reads `a`"
    );
    let identity = format!("quire.op.{family}.{suffix}");
    let equalities = applications(&wire, &identity);
    assert_eq!(equalities.len(), 1, "{body}: one {identity}");
    let arguments = equalities[0]["body"]["arguments"].as_array().unwrap();
    assert_eq!(arguments.len(), 2, "{body}");
    assert_eq!(
        arguments[converted_at]["target"]["digest"], convert["node_id"]["digest"],
        "{body}: the converted operand is the convert node"
    );
    let peer = referenced(&wire, &arguments[1 - converted_at]);
    assert_eq!(
        peer["body"]["members"][0]["value"]["value"], "b",
        "{body}: the other operand is `b`"
    );
    assert_eq!(
        convert["semantic_type"], peer["semantic_type"],
        "{body}: the convert is typed at the peer's type"
    );
    for wrong in ["integer", "rational", "decimal"] {
        if wrong != family {
            for suffix in ["eq", "ne"] {
                assert_eq!(
                    applications(&wire, &format!("quire.op.{wrong}.{suffix}")).len(),
                    0,
                    "{body}: no {wrong}.{suffix}"
                );
            }
        }
    }
}

/// FR-093-AC-3: `convert<Decimal>(a) = b` over an Int `a` lowers a Decimal
/// `numeric.convert` over `a` and `decimal.eq` over it, and strict IR admits
/// the package.
#[trace("FR-093-AC-3", "TC-415")]
#[test]
fn a_converted_decimal_equality_converts_the_int_operand() {
    assert_converted(
        INT,
        DEC,
        &format!("convert<{DEC}>(a) = b"),
        "decimal",
        "eq",
        0,
    );
}

/// FR-093-AC-3: the same for `!=`.
#[trace("FR-093-AC-3", "TC-415")]
#[test]
fn a_converted_decimal_inequality_converts_the_int_operand() {
    assert_converted(
        INT,
        DEC,
        &format!("convert<{DEC}>(a) != b"),
        "decimal",
        "ne",
        0,
    );
}

/// FR-093-AC-3: the converted operand on the right.
#[trace("FR-093-AC-3", "TC-415")]
#[test]
fn a_converted_operand_on_the_right_is_the_second_argument() {
    assert_converted(
        INT,
        DEC,
        &format!("b = convert<{DEC}>(a)"),
        "decimal",
        "eq",
        1,
    );
    assert_converted(
        INT,
        DEC,
        &format!("b != convert<{DEC}>(a)"),
        "decimal",
        "ne",
        1,
    );
}

/// FR-093-AC-3: the Rational family takes the same treatment.
#[trace("FR-093-AC-3", "TC-415")]
#[test]
fn a_converted_rational_equality_converts_the_int_operand() {
    assert_converted(
        INT,
        RAT,
        &format!("convert<{RAT}>(a) = b"),
        "rational",
        "eq",
        0,
    );
    assert_converted(
        INT,
        RAT,
        &format!("convert<{RAT}>(a) != b"),
        "rational",
        "ne",
        0,
    );
    assert_converted(
        INT,
        RAT,
        &format!("b = convert<{RAT}>(a)"),
        "rational",
        "eq",
        1,
    );
}

/// FR-093-AC-3: an equality with no conversion is unchanged: `integer.eq`
/// over the two parameters and no convert node.
#[trace("FR-093-AC-3", "TC-415")]
#[test]
fn an_unconverted_equality_builds_no_convert_node() {
    let wire = admitted("Int[-100,100]", "Int[-100,100]", "a = b");
    assert_eq!(applications(&wire, "quire.op.numeric.convert").len(), 0);
    let equalities = applications(&wire, "quire.op.integer.eq");
    assert_eq!(equalities.len(), 1);
    for (index, expected) in ["a", "b"].into_iter().enumerate() {
        let operand = referenced(&wire, &equalities[0]["body"]["arguments"][index]);
        assert_eq!(operand["semantic_form"], "parameter");
        assert_eq!(operand["body"]["members"][0]["value"]["value"], expected);
    }
}

/// FR-093-AC-3: an Int converted to a wider Int and a Decimal converted to
/// a Decimal of a larger scale are converted operands of their own family.
#[trace("FR-093-AC-3", "TC-415")]
#[test]
fn a_converted_integer_or_widened_decimal_equality_converts_its_operand() {
    assert_converted(
        INT,
        WIDE,
        &format!("convert<{WIDE}>(a) = b"),
        "integer",
        "eq",
        0,
    );
    assert_converted(
        DEC,
        DEC2,
        &format!("b != convert<{DEC2}>(a)"),
        "decimal",
        "ne",
        1,
    );
}
