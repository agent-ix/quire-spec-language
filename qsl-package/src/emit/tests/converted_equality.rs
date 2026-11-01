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
fn checked_graph(declarations: &str) -> qsl_semantics::check::CheckedGraph {
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
    PackageDeclarations::assemble(raw, "pkg".to_owned(), unit, Vec::new(), Vec::new())
        .expect("the unit assembles")
        .check(CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("the package checks: {refusals:?}"))
}

/// `declarations` through S1 to S4 and the v2 emitter, with nothing omitted.
fn emitted(declarations: &str) -> Emission {
    let graph = checked_graph(declarations);
    let emission = emit_checked(&CheckedPackage::link(graph)).expect("the package emits");
    assert_eq!(emission.omitted, []);
    emission
}

/// QSpec-TC-233's EQ-01..05 positive rows: each typed equality retains its
/// selected schedule, explicit conversion target, resolved units and enum
/// sub-index before the IR-owned wire writer is connected.
#[ix_trace_rs::trace("QSpec-TC-233", "QSL-1390")]
#[test]
fn qspec_positive_equality_vectors_match_typed_producer_facts() {
    use quire_semantic_value::declaration::{EqualityOperator, EqualitySchedule};

    let declarations = "dimension Time;\n\
        unit s : Time = rational(1, 1);\n\
        unit ms : Time = rational(1, 1000) * s;\n\
        dimension Length;\n\
        unit m : Length = rational(1, 1);\n\
        enum Color { Blue, Green }\n\
        record Point { x: Integer; }\n\
        function text using v(a: Text[0, 8; nfc], b: Text[0, 8; nfc]): Boolean pure { a = b }\n\
        function enumeration using v(a: Color, b: Color): Boolean pure { a != b }\n\
        function quantity using v(a: Quantity<ms>, b: Quantity<s>): Boolean pure { convert<Quantity<s>>(a) = b }\n\
        function plan using v(a: Point, b: Point): Boolean pure { a = b }\n\
        function compound using v(a: Quantity<m>, b: Quantity<m>): Boolean pure { a * b = a * b }\n";
    let graph = checked_graph(declarations);
    let facts: Vec<_> = graph.equality_producer_data().values().collect();
    assert_eq!(facts.len(), 5, "each equality application is retained once");

    let text = facts
        .iter()
        .find(|fact| fact.schedule() == EqualitySchedule::Text)
        .expect("EQ-01-text selects the text schedule");
    assert_eq!(text.operator(), EqualityOperator::Equal);
    assert_eq!(text.authored_operands()[0].target(), None);
    assert!(text.units().ids().next().is_none());

    let enumeration = facts
        .iter()
        .find(|fact| fact.schedule() == EqualitySchedule::Enum)
        .expect("EQ-02-enum selects the enum schedule");
    assert_eq!(enumeration.operator(), EqualityOperator::NotEqual);
    assert!(format!("{:?}", enumeration.enum_members()).contains("Blue"));
    assert!(format!("{:?}", enumeration.enum_members()).contains("Green"));
    assert!(enumeration.units().ids().next().is_none());

    let converted_quantity = facts
        .iter()
        .find(|fact| {
            fact.schedule() == EqualitySchedule::Quantity
                && fact.authored_operands()[0].target().is_some()
        })
        .expect("EQ-03-quantity retains its explicit ms-to-s conversion");
    assert_eq!(converted_quantity.operator(), EqualityOperator::Equal);
    assert_eq!(converted_quantity.units().ids().count(), 2);
    assert!(converted_quantity.compared_operands()[0].target().is_none());

    let plan = facts
        .iter()
        .find(|fact| fact.schedule() == EqualitySchedule::Plan)
        .expect("EQ-04-plan selects the plan schedule");
    assert_eq!(plan.operator(), EqualityOperator::Equal);
    assert!(plan.units().ids().next().is_none());

    let compound = facts
        .iter()
        .find(|fact| {
            fact.units().ids().count() == 1
                && fact.schedule() == EqualitySchedule::Quantity
                && fact.authored_operands()[0].target().is_none()
        })
        .expect("EQ-05-compound-quantity is backed by one compound quantity unit");
    assert_eq!(compound.operator(), EqualityOperator::Equal);
    assert!(compound.units().ids().any(|id| {
        matches!(
            compound.units().get(id),
            Some(quire_semantic_value::quantity::QuantityUnit::Compound(_))
        )
    }));
}

/// QSL-1390: every nested equality application is retained independently.
#[ix_trace_rs::trace("QSL-1390")]
#[test]
fn nested_equalities_each_retain_producer_facts() {
    let graph = checked_graph(
        "function nested using v(a: Integer, b: Integer, c: Integer): Boolean pure { a = b and b = c }",
    );
    assert_eq!(
        graph.equality_producer_data().len(),
        2,
        "both nested equality applications survive lowering"
    );
    assert!(graph.equality_producer_data().values().all(|facts| {
        facts.schedule() == quire_semantic_value::declaration::EqualitySchedule::Plan
    }));
}

/// QSL-1390: distinct source occurrences that lower to the same structural
/// equality key retain their independent authored conversion decisions.
#[ix_trace_rs::trace("QSL-1390")]
#[test]
fn structurally_reused_equality_keys_retain_each_occurrence() {
    let graph = checked_graph(
        "dimension Time; unit s : Time = rational(1, 1); unit ms : Time = rational(1, 1000) * s;\n\
         function same using v(a: Quantity<ms>, b: Quantity<ms>): Boolean pure { a = b and convert<Quantity<ms>>(a) = b }",
    );
    let occurrences = graph.equality_producer_data();
    assert_eq!(occurrences.len(), 2, "both source occurrences are retained");
    let mut rows = occurrences.iter().collect::<Vec<_>>();
    rows.sort_by_key(|(_, data)| data.authored_operands()[0].target().is_some());

    let keys = occurrences.keys().collect::<Vec<_>>();
    assert_eq!(keys[0].0, keys[1].0, "lowering reused one structural key");
    assert_ne!(keys[0].1, keys[1].1, "source locations distinguish them");
    assert_eq!(rows[0].1.authored_operands()[0].target(), None);
    assert!(rows[1].1.authored_operands()[0].target().is_some());

    let emission = emitted(
        "dimension Time; unit s : Time = rational(1, 1); unit ms : Time = rational(1, 1000) * s;\n\
         function same using v(a: Quantity<ms>, b: Quantity<ms>): Boolean pure { a = b and convert<Quantity<ms>>(a) = b }",
    );
    let identity_rows = emission.equality_producer_identity_rows();
    assert_eq!(identity_rows.len(), 2);
    assert_eq!(
        identity_rows[0].application(),
        identity_rows[1].application(),
        "both authored occurrences retain the admitted application key"
    );
    assert_ne!(
        identity_rows[0].location(),
        identity_rows[1].location(),
        "each identity row retains its own source occurrence"
    );
    assert!(identity_rows
        .iter()
        .all(|row| !row.quantity_type_nodes().is_empty()));
}

/// QSL-1390: emitted identity rows retain exact quantity type and nominal
/// descriptor keys, including the root closure of a compound quantity.
#[ix_trace_rs::trace("QSpec-TC-233", "QSL-1390")]
#[test]
fn emission_retains_quantity_descriptor_node_keys() {
    use quire_exact::UnitDomain;

    let emission = emitted(
        "dimension Length; unit m : Length = rational(1, 1);\n\
         function quantity using v(a: Quantity<m>, b: Quantity<m>): Boolean pure { a = b }\n\
         function compound using v(a: Quantity<m>, b: Quantity<m>): Boolean pure { a * b = a * b }",
    );
    let rows = emission.equality_producer_identity_rows();
    assert_eq!(rows.len(), 2);
    let declared = rows
        .iter()
        .find(|row| {
            row.quantity_type_nodes()
                .keys()
                .any(|unit| unit.domain() == UnitDomain::Declared)
        })
        .expect("declared quantity rows retain a quantity type node");
    assert!(declared
        .quantity_type_nodes()
        .iter()
        .all(|(unit, node)| unit.declared_key() == Some(node)));

    let compound = rows
        .iter()
        .find(|row| {
            row.quantity_type_nodes()
                .keys()
                .any(|unit| unit.domain() == UnitDomain::Compound)
        })
        .expect("compound quantity rows retain a quantity type node");
    assert!(compound
        .quantity_type_nodes()
        .keys()
        .any(|unit| unit.domain() == UnitDomain::Compound));
    assert!(!compound.nominal_nodes().is_empty());
    for (key, node) in compound.nominal_nodes() {
        assert_eq!(key, &node.key);
        match &node.form {
            quire_semantic_value::unit::NominalUnitForm::Dimension { terms } => {
                assert!(terms
                    .iter()
                    .all(|(base, _)| compound.nominal_nodes().contains_key(base)));
            }
            quire_semantic_value::unit::NominalUnitForm::Unit {
                dimension, target, ..
            } => {
                assert!(compound.nominal_nodes().contains_key(dimension));
                assert!(target
                    .as_ref()
                    .is_none_or(|target| compound.nominal_nodes().contains_key(target)));
            }
        }
    }
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
    let key = &term["target"];
    nodes(wire)
        .iter()
        .find(|node| &node["node_id"] == key)
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
        arguments[converted_at]["target"], convert["node_id"],
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
        .map(|argument| &argument["target"])
        .collect();
    for (argument, parameter) in arguments.iter().zip(["a", "c"]) {
        let convert = converts
            .iter()
            .find(|convert| &convert["node_id"] == *argument)
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
        admit_unit_graph, DimensionPreimage, NodeOwner, OwnerSelection, UnitPreimage,
    };
    let owner = json!({"kind": "source", "authority": "a", "identity": "u", "package_name": "pkg"});
    let length = DimensionPreimage::from_json(json!({
        "version": "quire.dimension-node/v1",
        "owner": owner,
        "qualified_declaration": ["Example", "Length"],
        "terms": [],
    }))
    .expect("a base dimension");
    let length_key = length.key().expect("a dimension declaration key");
    let node_ref = |key: &NodeKey| {
        qsl_foundation::node_wire::node_key_to_json(key).expect("a structural fixture key")
    };
    let unit = |name: &str, target: Option<NodeKey>, scale: &str| {
        UnitPreimage::from_json(json!({
            "version": "quire.unit-node/v1",
            "owner": owner,
            "qualified_declaration": ["Example", name],
            "dimension_node_id": node_ref(&length_key),
            "target_unit_node_id": target.as_ref().map(node_ref),
            "scale": {"numerator": scale, "denominator": "1"},
            "offset": {"numerator": "0", "denominator": "1"},
        }))
        .expect("a unit")
    };
    let metre = unit("metre", None, "1");
    let metre_key = metre.key().expect("a unit declaration key");
    let kilometre = unit("kilometre", Some(metre_key.clone()), "1000");
    let kilometre_key = kilometre.key().expect("a unit declaration key");
    let selection: NodeOwner = serde_json::from_value(owner).expect("an owner");
    let graph = admit_unit_graph(
        [(length, length_key)],
        [
            (metre, metre_key.clone()),
            (kilometre, kilometre_key.clone()),
        ],
        &OwnerSelection::new([selection]),
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
            ..PackageDeclarations::new(source(), "pkg", qsl_foundation::IdentityLimits::default())
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
            equalities[0]["body"]["arguments"][at]["target"], converts[0]["node_id"],
            "{suffix}: the converted operand is the convert node"
        );
    }
}

/// FR-093-AC-32: a guard (`=`, `!=` or an ordering) over a value-keeping
/// integer conversion, a chain of them included, bounds its operand as the
/// guard over the operand itself does.
#[trace("FR-093-AC-32", "TC-415")]
#[test]
fn a_guard_over_an_integer_conversion_bounds_the_converted_operand() {
    for body in [
        "if x = 2 then x else 0",
        "if convert<Integer>(x) = 2 then x else 0",
        "if convert<Int[-10,10]>(x) = 2 then x else 0",
        "if 2 = convert<Integer>(x) then x else 0",
        "if convert<Integer>(x) != 2 then 0 else x",
        "if convert<Int[-5,5]>(convert<Decimal[-5,5;0,0;nearest-even]>(x)) = 2 then x else 0",
        "if convert<Integer>(x) < 4 then (if convert<Integer>(x) > -1 then x else 0) else 0",
        "if convert<Integer>(x) >= 0 then (if convert<Int[-10,10]>(x) <= 3 then x else 0) else 0",
        "if 4 > convert<Integer>(x) then (if -1 < convert<Integer>(x) then x else 0) else 0",
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
        "if convert<Integer>(x) < 4 then x else 0",
        "if convert<Integer>(x) > -1 then x else 0",
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
        let refused =
            PackageDeclarations::assemble(raw, "pkg".to_owned(), unit, Vec::new(), Vec::new())
                .expect("the unit assembles")
                .check(CheckingLimits::default());
        assert!(refused.is_err(), "{body}: the body is not in [0, 3]");
    }
}

/// FR-093-AC-31: the IR-17 host conversion sources, each with its own scalar
/// domains: `convert<T>(a) = b` lowers a convert node and the equality of
/// `T`'s family, and strict IR admits the package.
#[trace("FR-093-AC-31", "TC-415")]
#[test]
fn the_host_conversion_sources_lower_a_convert_and_read_back_verified() {
    for (source, target, family) in [
        ("Int[-100, 100]", "Integer", "integer"),
        (
            "Int[-100, 100]",
            "Decimal[-100, 100; 0, 0; nearest-even]",
            "decimal",
        ),
        (
            "Rational[-10, 10; 1, 1]",
            "Rational[-100, 100; 1, 5]",
            "rational",
        ),
        ("Rational[-50, 50; 1, 1]", "Integer", "integer"),
        (
            "Decimal[-100, 100; 0, 0; nearest-even]",
            "Rational[-100, 100; 1, 5]",
            "rational",
        ),
        (
            "Decimal[-100, 100; 0, 0; nearest-even]",
            "Decimal[-1000, 1000; 0, 2; nearest-even]",
            "decimal",
        ),
        (
            "Decimal[-100, 100; 0, 0; nearest-even]",
            "Integer",
            "integer",
        ),
    ] {
        assert_converted(
            source,
            target,
            &format!("convert<{target}>(a) = b"),
            family,
            "eq",
            0,
        );
    }
}
