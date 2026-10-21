// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-093-AC-31 and FR-093-AC-32 (TC-415): an equality operand the checker converts
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

/// FR-093-AC-31: `convert<Decimal>(a) = b` over an Int `a` lowers a Decimal
/// `numeric.convert` over `a` and `decimal.eq` over it, and strict IR admits
/// the package.
#[trace("FR-093-AC-31", "TC-415")]
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

/// FR-093-AC-31: the same for `!=`.
#[trace("FR-093-AC-31", "TC-415")]
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

/// FR-093-AC-31: the converted operand on the right.
#[trace("FR-093-AC-31", "TC-415")]
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

/// FR-093-AC-31: the Rational family takes the same treatment.
#[trace("FR-093-AC-31", "TC-415")]
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

/// FR-093-AC-31: an equality with no conversion is unchanged: `integer.eq`
/// over the two parameters and no convert node.
#[trace("FR-093-AC-31", "TC-415")]
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

/// FR-093-AC-31: an Int converted to a wider Int and a Decimal converted to
/// a Decimal of a larger scale are converted operands of their own family.
#[trace("FR-093-AC-31", "TC-415")]
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

/// FR-093-AC-31: the other numeric directions, each a converted operand of
/// the family of the converted type.
#[trace("FR-093-AC-31", "TC-415")]
#[test]
fn a_converted_equality_between_decimal_rational_and_int_converts_its_operand() {
    for (source, target, family) in [
        (DEC, INT, "integer"),
        (RAT, INT, "integer"),
        (RAT, DEC, "decimal"),
        (DEC, RAT, "rational"),
    ] {
        for (body, suffix, at) in [
            (format!("convert<{target}>(a) = b"), "eq", 0),
            (format!("convert<{target}>(a) != b"), "ne", 0),
            (format!("b = convert<{target}>(a)"), "eq", 1),
        ] {
            assert_converted(source, target, &body, family, suffix, at);
        }
    }
}

/// FR-093-AC-31: both operands converted builds one convert per operand and
/// an equality over the two.
#[trace("FR-093-AC-31", "TC-415")]
#[test]
fn an_equality_with_both_operands_converted_converts_each() {
    let emission = emitted(&format!(
        "function f using v(a: {INT}, c: Int[-50,50]): Boolean pure {{ convert<{DEC}>(a) = convert<{DEC}>(c) }}"
    ));
    assert!(matches!(read_back(&emission), Read::Verified { .. }));
    let wire = wire(&emission);
    let converts = applications(&wire, "quire.op.numeric.convert");
    assert_eq!(converts.len(), 2);
    let equalities = applications(&wire, "quire.op.decimal.eq");
    assert_eq!(equalities.len(), 1);
    let arguments: Vec<&Value> = equalities[0]["body"]["arguments"]
        .as_array()
        .unwrap()
        .iter()
        .map(|argument| &argument["target"]["digest"])
        .collect();
    for (argument, parameter) in arguments.iter().zip(["a", "c"]) {
        let convert = converts
            .iter()
            .find(|convert| &convert["node_id"]["digest"] == *argument)
            .expect("each argument is a convert node");
        let source = referenced(&wire, &convert["body"]["arguments"][0]);
        assert_eq!(source["body"]["members"][0]["value"]["value"], parameter);
    }
}

/// A unit `kilometre` of 1000 `metre`, the root unit `metre` and their
/// dimension, owned by the fixture source.
fn length_units() -> (
    quire_semantic_value::quantity::UnitTable,
    ValueType,
    ValueType,
) {
    use qsl_semantics::value::{
        admit_unit_graph, DimensionPreimage, NodeIdentityPreimage, NodeOwner, OwnerSelection,
        UnitPreimage,
    };
    let owner = json!({"kind": "source", "authority": "a", "identity": "u"});
    let digest = |preimage: &dyn NodeIdentityPreimage| {
        NodeKey::from_digest(
            preimage
                .digest(qsl_foundation::IdentityLimits::default())
                .expect("a digest"),
        )
    };
    let length = DimensionPreimage::from_json(json!({
        "version": "quire.dimension-node/v1",
        "owner": owner,
        "qualified_declaration": ["Example", "Length"],
        "terms": [],
    }))
    .expect("a base dimension");
    let length_key = digest(&length);
    let node_ref = |key: NodeKey| json!({"domain": NODE_KEY_DOMAIN, "digest": key.to_string()});
    let unit = |name: &str, target: Option<NodeKey>, scale: &str| {
        UnitPreimage::from_json(json!({
            "version": "quire.unit-node/v1",
            "owner": owner,
            "qualified_declaration": ["Example", name],
            "dimension_node_id": node_ref(length_key),
            "target_unit_node_id": target.map(node_ref),
            "scale": {"numerator": scale, "denominator": "1"},
            "offset": {"numerator": "0", "denominator": "1"},
        }))
        .expect("a unit")
    };
    let metre = unit("metre", None, "1");
    let metre_key = digest(&metre);
    let kilometre = unit("kilometre", Some(metre_key), "1000");
    let kilometre_key = digest(&kilometre);
    let selection: NodeOwner = serde_json::from_value(owner).expect("an owner");
    let graph = admit_unit_graph(
        [(length, length_key)],
        [(metre, metre_key), (kilometre, kilometre_key)],
        &OwnerSelection::new([selection]),
        qsl_foundation::IdentityLimits::default(),
    )
    .expect("the unit graph admits");
    (
        quire_semantic_value::quantity::UnitTable::declared(&graph),
        ValueType::Quantity(quire_exact::UnitId::declared(metre_key)),
        ValueType::Quantity(quire_exact::UnitId::declared(kilometre_key)),
    )
}

/// FR-093-AC-31: `convert<Km>(a) = b` over a metre quantity `a` lowers a
/// `quantity.convert` with mode `exact` and `quantity.eq`/`.ne` over it.
#[trace("FR-093-AC-31", "TC-415")]
#[test]
fn a_converted_quantity_equality_converts_the_quantity_operand() {
    let (units, metre, kilometre) = length_units();
    let named = |text: &str| TypeForm::name(text, SPAN);
    for (operator, suffix, converted_first) in [
        (BinaryOperator::Equal, "eq", true),
        (BinaryOperator::NotEqual, "ne", true),
        (BinaryOperator::Equal, "eq", false),
        (BinaryOperator::NotEqual, "ne", false),
    ] {
        let converted = Expression::convert(named("Km"), name("a"));
        let (left, right) = if converted_first {
            (converted, name("b"))
        } else {
            (name("b"), converted)
        };
        let declared = FunctionDeclaration::new(
            "f",
            vec![
                ("a".to_owned(), named("Length")),
                ("b".to_owned(), named("Km")),
            ],
            boolean(),
            None,
            Expression::binary(operator, left, right),
        );
        let graph = PackageDeclarations {
            types: TypeEnvironment::default().with_units(units.clone()),
            aliases: vec![
                ("Length".to_owned(), metre.clone()),
                ("Km".to_owned(), kilometre.clone()),
            ],
            functions: vec![declared],
            ..PackageDeclarations::new(source(), qsl_foundation::IdentityLimits::default())
        }
        .check(CheckingLimits::default())
        .expect("the package checks");
        let emission = emit(&CheckedPackage::link(graph));
        assert!(
            matches!(read_back(&emission), Read::Verified { .. }),
            "{suffix}: the strict reader admits it"
        );
        let wire = wire(&emission);
        let converts = applications(&wire, "quire.op.quantity.convert");
        assert_eq!(converts.len(), 1, "{suffix}: one convert node");
        assert_eq!(
            converts[0]["body"]["operation"]["mode"],
            json!({"kind": "rounding", "value": "exact"})
        );
        assert!(applications(&wire, "quire.op.numeric.convert").is_empty());
        let equalities = applications(&wire, &format!("quire.op.quantity.{suffix}"));
        assert_eq!(equalities.len(), 1, "{suffix}: one quantity.{suffix}");
        let at = usize::from(!converted_first);
        assert_eq!(
            equalities[0]["body"]["arguments"][at]["target"]["digest"],
            converts[0]["node_id"]["digest"],
            "{suffix}: the converted operand is the convert node"
        );
    }
}

/// FR-093-AC-32: a guard over a value-keeping integer conversion bounds its
/// operand as the guard over the operand itself does.
#[trace("FR-093-AC-32", "TC-415")]
#[test]
fn a_guard_over_an_integer_conversion_bounds_the_converted_operand() {
    for body in [
        "if x = 2 then x else 0",
        "if convert<Integer>(x) = 2 then x else 0",
        "if convert<Int[-10,10]>(x) = 2 then x else 0",
        "if 2 = convert<Integer>(x) then x else 0",
        "if convert<Integer>(x) != 2 then 0 else x",
    ] {
        let emission = emitted(&format!(
            "function g using v(x: Int[-5,5]): Int[0,3] pure {{ {body} }}"
        ));
        assert!(
            matches!(read_back(&emission), Read::Verified { .. }),
            "{body}"
        );
    }
}

/// FR-093-AC-32: the bound is the guard's, so an unguarded or differently
/// guarded body still refuses.
#[trace("FR-093-AC-32", "TC-415")]
#[test]
fn a_guard_over_an_integer_conversion_bounds_only_what_it_states() {
    for body in [
        "if convert<Integer>(x) = 4 then x else 0",
        "if convert<Integer>(x) != 2 then x else 0",
        "x",
    ] {
        let text =
            format!("{HEADER}function g using v(x: Int[-5,5]): Int[0,3] pure {{ {body} }}\n");
        let parsed = qsl_cst::parse(
            qsl_foundation::SourceIdentity::new("agent-ix", "test:guard", "fixture", "fixture:1"),
            "program.native",
            text.as_bytes(),
            qsl_cst::Limits::default(),
        )
        .expect("S1 admits the unit");
        let raw = parsed.source().reference().clone();
        let unit = qsl_forms::build_unit(&parsed).expect("S2 builds the unit");
        let refused = PackageDeclarations::assemble(raw, unit, Vec::new(), Vec::new())
            .expect("the unit assembles")
            .check(CheckingLimits::default());
        assert!(refused.is_err(), "{body}: the body is not in [0, 3]");
    }
}
