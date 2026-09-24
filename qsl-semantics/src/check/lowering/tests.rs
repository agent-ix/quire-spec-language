// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-413, TC-414 and TC-415: the FR-092 keys and the FR-093 lowering,
//! checked against FR-092's golden vectors. The vectors are read from this
//! repository's own FR-092 text at compile time, so the spec and the test
//! cannot drift apart.

use std::collections::BTreeMap;

use ix_trace_rs::trace;
use qsl_forms::{BinaryOperator, BinderQuery, BuiltinType, Expression, FunctionDeclaration, TypeForm};
use quire_exact::{
    CardinalityBound, CollectionType, DecimalType, IntegerInterval, Presence, RationalDomain,
    RoundingMode, TextType,
};
use serde_json::{json, Value as Json};

use super::*;
use crate::check::family::fixtures::{empty_scope, fixture_owner};
use crate::check::{CheckedGraph, CheckingLimits, PackageDeclarations};
use crate::value::declaration::{CompositeDeclaration, FieldDeclaration, TypeEnvironment};

const SPAN: qsl_foundation::Span = qsl_foundation::Span { start: 0, end: 0 };

const FR_092: &str = include_str!(
    "../../../../spec/functional/FR-092-key-type-parameter-and-declared-nodes.md"
);

/// FR-092's golden vectors by name: `(key, preimage)`.
fn vectors() -> BTreeMap<String, (String, String)> {
    let mut vectors = BTreeMap::new();
    let mut lines = FR_092.lines();
    while let Some(line) = lines.next() {
        let Some(rest) = line.strip_prefix("**") else {
            continue;
        };
        let Some((name, _)) = rest.split_once("**: ") else {
            continue;
        };
        if lines.next() != Some("") || lines.next() != Some("```json") {
            continue;
        }
        let preimage = lines.next().expect("a vector preimage line").to_owned();
        assert_eq!(lines.next(), Some("```"), "{name}: one preimage line");
        assert_eq!(lines.next(), Some(""));
        let key = lines
            .next()
            .and_then(|line| line.strip_prefix("Key: `"))
            .and_then(|line| line.strip_suffix('`'))
            .expect("a vector key line")
            .to_owned();
        vectors.insert(name.to_owned(), (key, preimage));
    }
    assert_eq!(vectors.len(), 30, "FR-092 publishes 30 golden vectors");
    vectors
}

fn vector_key(name: &str) -> String {
    vectors()[name].0.clone()
}

/// Assert `node` is vector `name`, bytes and key.
fn assert_vector(graph: &SemanticGraph, key: NodeKey, name: &str) {
    let (expected_key, expected_preimage) = &vectors()[name];
    let node = graph
        .node(key)
        .unwrap_or_else(|| panic!("{name}: no node keyed {key}"));
    assert_eq!(
        std::str::from_utf8(node.preimage()).expect("UTF-8"),
        expected_preimage,
        "{name}: preimage bytes"
    );
    assert_eq!(&key.to_string(), expected_key, "{name}: key");
}

fn node_by_key<'g>(graph: &'g SemanticGraph, hex: &str) -> &'g SemanticNode {
    graph
        .nodes()
        .find(|node| node.key().to_string() == hex)
        .unwrap_or_else(|| panic!("no node keyed {hex}"))
}

fn preimage(node: &SemanticNode) -> Json {
    serde_json::from_slice(node.preimage()).expect("a preimage is JSON")
}

// ---------------------------------------------------------------------
// Type nodes, built directly.
// ---------------------------------------------------------------------

fn int(lower: i64, upper: i64) -> ValueType {
    ValueType::Int(IntegerInterval::new(Integer::from(lower), Integer::from(upper)).unwrap())
}

fn sequence(element: ValueType, bound: Option<(u64, u64)>) -> ValueType {
    let (minimum, maximum) = bound.unwrap_or((0, u64::MAX));
    ValueType::collection(CollectionType::new(
        CollectionKind::Sequence,
        element,
        CardinalityBound::new(minimum, maximum).unwrap(),
    ))
}

fn rational_9() -> ValueType {
    ValueType::Rational(
        RationalDomain::new(
            IntegerInterval::new(Integer::from(-9_i64), Integer::from(9_i64)).unwrap(),
            IntegerInterval::new(Integer::from(1_i64), Integer::from(9_i64)).unwrap(),
        )
        .unwrap(),
    )
}

/// Build `value_types`' nodes under `owner` over `scope`: the graph and
/// each key.
fn type_nodes(
    scope: &Scope,
    owner: &SourceOwner,
    value_types: &[ValueType],
) -> (SemanticGraph, Vec<NodeKey>) {
    let lock = LockEvidence::default();
    let mut occurrences = OccurrenceMap::default();
    let location = generated_location();
    let mut lowering = Lowering::new(
        scope,
        owner,
        &lock,
        crate::check::MAX_CHECKING_DEPTH,
        0,
        &mut occurrences,
    );
    let keys = value_types
        .iter()
        .map(|value_type| lowering.type_node(value_type, &location).expect("the type keys"))
        .collect();
    (lowering.finish(&location), keys)
}

/// TC-413 step 1 (FR-092-AC-1): the builtin, bounded and anonymous type
/// nodes are T1 to T8, byte for byte.
#[trace("FR-092-AC-1", "TC-413")]
#[test]
fn builtin_and_anonymous_type_nodes_match_t1_to_t8() {
    let text_64 = ValueType::Text(TextType::new(0, 64, TextProfile::Nfc).unwrap());
    let cases = [
        ("T1", ValueType::Boolean),
        ("T2", ValueType::Integer),
        ("T4", int(0, 9)),
        ("T5", ValueType::option(int(0, 9))),
        ("T7", sequence(int(0, 9), Some((0, 5)))),
        ("T8", text_64.clone()),
    ];
    let (graph, keys) = type_nodes(
        &empty_scope(),
        &fixture_owner(),
        &cases.iter().map(|(_, value_type)| value_type.clone()).collect::<Vec<_>>(),
    );
    for ((name, _), key) in cases.iter().zip(&keys) {
        assert_vector(&graph, *key, name);
    }
    // T3 (the text scalar) and T6 (the unbounded `Sequence<Int[0, 9]>`)
    // are the base nodes T8 and T7 reference.
    assert_vector(&graph, node_by_key(&graph, &vector_key("T3")).key(), "T3");
    assert_vector(&graph, node_by_key(&graph, &vector_key("T6")).key(), "T6");
}

/// TC-413 step 7 (FR-092-AC-9): the rational and decimal bounded nodes and
/// their bases are T9 to T12; the tuple is D5; an optional field `b?:
/// Int[0, 9]` (D3) differs from `b: Option<Int[0, 9]>` (D4).
#[trace("FR-092-AC-9", "TC-413")]
#[test]
fn rational_decimal_and_declared_composite_nodes_match_their_vectors() {
    let decimal = ValueType::Decimal(
        DecimalType::new(
            Integer::from(-100_000_i64),
            Integer::from(100_000_i64),
            2,
            2,
            RoundingMode::NearestEven,
        )
        .unwrap(),
    );
    let pair = NodeKey::from_digest([1; 32]);
    let optional = NodeKey::from_digest([2; 32]);
    let explicit = NodeKey::from_digest([3; 32]);
    let mut scope = empty_scope();
    scope.types = TypeEnvironment::new(
        [
            CompositeDeclaration::new(pair, "Pair", CompositeShape::Tuple(vec![int(0, 9), int(0, 9)])),
            CompositeDeclaration::new(
                optional,
                "Opt",
                CompositeShape::Record(vec![
                    FieldDeclaration::new("a", int(0, 9), Presence::Required),
                    FieldDeclaration::new("b", int(0, 9), Presence::Optional),
                ]),
            ),
        ],
        [],
    )
    .unwrap();
    let mut explicit_scope = empty_scope();
    explicit_scope.types = TypeEnvironment::new(
        [CompositeDeclaration::new(
            explicit,
            "Opt",
            CompositeShape::Record(vec![
                FieldDeclaration::new("a", int(0, 9), Presence::Required),
                FieldDeclaration::new("b", ValueType::option(int(0, 9)), Presence::Required),
            ]),
        )],
        [],
    )
    .unwrap();

    let (graph, keys) = type_nodes(
        &scope,
        &fixture_owner(),
        &[
            rational_9(),
            decimal,
            ValueType::Composite(pair),
            ValueType::Composite(optional),
        ],
    );
    assert_vector(&graph, keys[0], "T10");
    assert_vector(&graph, node_by_key(&graph, &vector_key("T9")).key(), "T9");
    assert_vector(&graph, keys[1], "T12");
    assert_vector(&graph, node_by_key(&graph, &vector_key("T11")).key(), "T11");
    assert_vector(&graph, keys[2], "D5");
    assert_vector(&graph, keys[3], "D3");

    let (explicit_graph, explicit_keys) = type_nodes(
        &explicit_scope,
        &fixture_owner(),
        &[ValueType::Composite(explicit)],
    );
    assert_vector(&explicit_graph, explicit_keys[0], "D4");
    assert_ne!(keys[3], explicit_keys[0]);
}

/// TC-413 steps 2-3 (FR-092-AC-2): an anonymous type's key is the same
/// under every owner; a declared record's is its owner's, stable across
/// compiles, and carries `owner` and `declaration` where T4 carries neither.
#[trace("FR-092-AC-2", "TC-413")]
#[test]
fn a_declared_record_carries_its_owner_and_an_anonymous_type_does_not() {
    let point = NodeKey::from_digest([4; 32]);
    let mut scope = empty_scope();
    scope.types = TypeEnvironment::new(
        [CompositeDeclaration::new(
            point,
            "Point",
            CompositeShape::Record(vec![
                FieldDeclaration::new("x", int(0, 9), Presence::Required),
                FieldDeclaration::new("y", int(0, 9), Presence::Required),
            ]),
        )],
        [],
    )
    .unwrap();
    let w = SourceOwner::new("a", "w").unwrap();
    let types = [ValueType::Composite(point), int(0, 9)];
    let (under_u, u_keys) = type_nodes(&scope, &fixture_owner(), &types);
    let (again, again_keys) = type_nodes(&scope, &fixture_owner(), &types);
    let (under_w, w_keys) = type_nodes(&scope, &w, &types);

    assert_vector(&under_u, u_keys[0], "D1");
    assert_vector(&again, again_keys[0], "D1");
    assert_vector(&under_w, w_keys[0], "D2");
    assert_eq!(u_keys[1], w_keys[1]);
    assert_vector(&under_w, w_keys[1], "T4");
    let d1 = preimage(under_u.node(u_keys[0]).unwrap());
    assert!(d1.get("owner").is_some() && !d1["declaration"].is_null());
    let t4 = preimage(under_u.node(u_keys[1]).unwrap());
    assert!(t4.get("owner").is_none() && t4["declaration"].is_null());

    // Through `check`: a parameter typed `Int[0, 9]` keys to T4 under both
    // owners.
    let g1 = function(
        "g1",
        &[("x", int_form(0, 9))],
        boolean(),
        None,
        Expression::Boolean(true),
    );
    for owner in [fixture_owner(), w] {
        let graph = check_under(owner, vec![g1.clone()]).expect("g1 checks");
        let parameter = parameter_named(graph.semantic_graph(), "x");
        assert_eq!(
            parameter.semantic_type().map(|key| key.to_string()),
            Some(vector_key("T4"))
        );
    }
}

/// TC-413 step 5 (FR-092-AC-7): the type walk is bounded by the check
/// stage's depth limit.
#[trace("FR-092-AC-7", "TC-413")]
#[test]
fn a_type_nested_past_the_depth_limit_refuses() {
    let nested = |depth: usize| {
        (0..depth).fold(boolean(), |inner, _| {
            TypeForm::builtin(BuiltinType::Option, SPAN).with_arguments(vec![inner])
        })
    };
    let limits = CheckingLimits::new(u64::MAX, 4).expect("a depth of 4 is allowed");
    let checked = |depth: usize| {
        PackageDeclarations {
            functions: vec![function(
                "p",
                &[("x", nested(depth))],
                boolean(),
                None,
                Expression::Boolean(true),
            )],
            ..PackageDeclarations::new(fixture_owner())
        }
        .check(limits)
    };
    assert!(checked(4).is_ok(), "four nested options are keyed");
    let refusals = checked(5).expect_err("five nested options refuse");
    assert!(
        refusals.iter().any(|refusal| matches!(
            refusal.cause,
            CheckCause::ResourceExhausted {
                kind: CheckingLimitKind::Depth,
                limit: 4,
                ..
            }
        )),
        "{refusals:?}"
    );
    assert_eq!(refusals[0].cause.code().as_str(), "resource_exhausted");
    assert_eq!(refusals[0].cause.cause(), Some("insufficient-next-charge"));
}

/// TC-413 step 5 (FR-092-AC-7): two recursive functions of one body shape
/// refuse, naming both functions' regions, and yield no key. FR-092-AC-7's
/// own body `if x = 0 then true else f(0)` is refused earlier, by
/// termination (`f(0)` is not proved to decrease `x`), so the fixture
/// recurses on `x - 1` under `x > 0`, which termination admits.
#[trace("FR-092-AC-7", "TC-413")]
#[test]
fn recursion_groups_refuse_naming_their_members() {
    let recursive = |name: &str| {
        function(
            name,
            &[("x", int_form(0, 9))],
            boolean(),
            Some(name_expr("x")),
            Expression::If {
                condition: Box::new(binary(BinaryOperator::Greater, name_expr("x"), integer_expr(0))),
                then: Box::new(Expression::Call {
                    name: name.to_owned(),
                    arguments: vec![binary(BinaryOperator::Subtract, name_expr("x"), integer_expr(1))],
                }),
                otherwise: Box::new(Expression::Boolean(true)),
            },
        )
    };
    let refusals = check(vec![recursive("f"), recursive("g")]).expect_err("recursion refuses");
    let loci: Vec<Vec<Location>> = refusals
        .iter()
        .filter_map(|refusal| match &refusal.cause {
            CheckCause::UnsupportedFeature { loci } => Some(loci.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(loci.len(), 2, "{refusals:?}");
    let named: Vec<&Origin> = loci.iter().flatten().map(|location| &location.origin).collect();
    assert!(named.iter().any(|origin| matches!(origin, Origin::Body { function, .. } if function == "f")));
    assert!(named.iter().any(|origin| matches!(origin, Origin::Body { function, .. } if function == "g")));
    assert_eq!(refusals[0].cause.code().as_str(), "unknown_required_feature");
    assert_eq!(refusals[0].cause.cause(), Some("unsupported-feature"));
}

/// TC-413 step 4 (FR-092-AC-3): an alias adds no node; the parameter is
/// typed at the aliased type's node.
#[trace("FR-092-AC-3", "TC-413")]
#[test]
fn an_alias_introduces_no_type_node() {
    let graph = PackageDeclarations {
        aliases: vec![("Digit".to_owned(), int(0, 9))],
        functions: vec![function(
            "g",
            &[("x", TypeForm::name("Digit", SPAN))],
            boolean(),
            None,
            Expression::Boolean(true),
        )],
        ..PackageDeclarations::new(fixture_owner())
    }
    .check(CheckingLimits::default())
    .expect("g checks");
    let parameter = parameter_named(graph.semantic_graph(), "x");
    assert_eq!(
        parameter.semantic_type().map(|key| key.to_string()),
        Some(vector_key("T4"))
    );
    assert!(graph.semantic_graph().nodes().all(|node| node
        .declaration()
        .is_none_or(|name| name.iter().all(|segment| segment.as_str() != "Digit"))));
}

/// TC-413 step 8 (FR-092-CON-2, FR-093-CON-1): the preimage selection, the
/// type-node match and the lowering match over `NodeKind` have no `_` arm.
#[trace("FR-092-CON-2", "TC-413", "FR-093-CON-1", "TC-415")]
#[test]
fn the_type_and_lowering_matches_have_no_catch_all_arm() {
    let source = include_str!("../lowering.rs");
    let body_of = |signature: &str| {
        let start = source.find(signature).expect("the function exists");
        let end = source[start..]
            .find("\n    }\n")
            .map(|offset| start + offset)
            .expect("the function ends");
        &source[start..end]
    };
    for signature in ["fn type_node_at(", "fn expression(", "fn literal("] {
        assert!(
            !body_of(signature).contains("_ =>"),
            "{signature} has a catch-all arm"
        );
    }
}

// ---------------------------------------------------------------------
// Function fixtures, checked through `PackageDeclarations::check`.
// ---------------------------------------------------------------------

fn boolean() -> TypeForm {
    TypeForm::builtin(BuiltinType::Boolean, SPAN)
}

fn int_form(lower: i64, upper: i64) -> TypeForm {
    TypeForm::builtin(BuiltinType::Int, SPAN).with_bounds(vec![lower.to_string(), upper.to_string()])
}

fn name_expr(name: &str) -> Expression {
    Expression::Name(name.to_owned())
}

fn integer_expr(value: i64) -> Expression {
    Expression::Integer(Integer::from(value))
}

fn binary(operator: BinaryOperator, left: Expression, right: Expression) -> Expression {
    Expression::Binary {
        operator,
        left: Box::new(left),
        right: Box::new(right),
    }
}

fn function(
    name: &str,
    parameters: &[(&str, TypeForm)],
    result: TypeForm,
    measure: Option<Expression>,
    body: Expression,
) -> FunctionDeclaration {
    FunctionDeclaration::new(
        name,
        parameters
            .iter()
            .map(|(name, form)| ((*name).to_owned(), form.clone()))
            .collect(),
        result,
        measure,
        body,
    )
}

fn check_under(
    owner: SourceOwner,
    functions: Vec<FunctionDeclaration>,
) -> Result<CheckedGraph, Vec<CheckRefusal>> {
    PackageDeclarations {
        functions,
        ..PackageDeclarations::new(owner)
    }
    .check(CheckingLimits::default())
}

fn check(functions: Vec<FunctionDeclaration>) -> Result<CheckedGraph, Vec<CheckRefusal>> {
    check_under(fixture_owner(), functions)
}

/// The parameter node whose `name` binding is `name`.
fn parameter_named<'g>(graph: &'g SemanticGraph, name: &str) -> &'g SemanticNode {
    graph
        .nodes()
        .find(|node| {
            node.semantic_form() == "parameter"
                && preimage(node)["body"]["members"][0]["value"]["value"] == json!(name)
        })
        .unwrap_or_else(|| panic!("no parameter node named {name}"))
}

fn both() -> FunctionDeclaration {
    function(
        "both",
        &[("a", boolean()), ("b", boolean())],
        boolean(),
        None,
        binary(BinaryOperator::And, name_expr("a"), name_expr("b")),
    )
}

fn nb() -> FunctionDeclaration {
    function(
        "nb",
        &[("a", boolean())],
        boolean(),
        None,
        Expression::Call {
            name: "both".to_owned(),
            arguments: vec![name_expr("a"), Expression::Boolean(true)],
        },
    )
}

fn h() -> FunctionDeclaration {
    function(
        "h",
        &[("a", boolean())],
        boolean(),
        None,
        Expression::Let {
            name: "y".to_owned(),
            value: Box::new(name_expr("a")),
            body: Box::new(name_expr("y")),
        },
    )
}

fn f() -> FunctionDeclaration {
    function("f", &[], boolean(), None, Expression::Boolean(true))
}

fn t() -> FunctionDeclaration {
    function(
        "t",
        &[],
        boolean(),
        None,
        Expression::If {
            condition: Box::new(Expression::Boolean(true)),
            then: Box::new(Expression::Boolean(true)),
            otherwise: Box::new(Expression::Boolean(true)),
        },
    )
}

/// TC-414 step 1 (FR-092-AC-4): the parameter, literal and function nodes
/// of `both`, `f`, `h` and `k` are their vectors, bytes and keys.
#[trace("FR-092-AC-4", "TC-414")]
#[test]
fn parameter_literal_and_function_nodes_match_their_vectors() {
    let k = function(
        "k",
        &[("n", boolean())],
        TypeForm::builtin(BuiltinType::Integer, SPAN),
        None,
        integer_expr(7),
    );
    let graph = check(vec![both(), f(), h(), k]).expect("the fixtures check");
    let semantic = graph.semantic_graph();
    for name in ["P1", "P2", "P3", "L1", "L2", "F1", "F2"] {
        let key = node_by_key(semantic, &vector_key(name)).key();
        assert_vector(semantic, key, name);
    }
    assert_eq!(
        graph.function_identity("both").map(|key| key.to_string()),
        Some(vector_key("F2"))
    );
    assert_eq!(
        graph.function_identity("f").map(|key| key.to_string()),
        Some(vector_key("F1"))
    );
    let l2 = preimage(node_by_key(semantic, &vector_key("L2")));
    assert_eq!(l2["body"]["value"], json!("7"), "an integer literal is a string");
}

/// TC-414 step 2 (FR-092-AC-5): a function lists every parameter in order,
/// read or not, and a second function reuses the same parameter nodes.
#[trace("FR-092-AC-5", "TC-414")]
#[test]
fn unread_parameters_keep_the_arity_and_parameter_nodes_are_shared() {
    let unused = function(
        "unused",
        &[("a", boolean()), ("b", boolean())],
        boolean(),
        None,
        name_expr("a"),
    );
    let mut both2 = both();
    both2.name = "both2".to_owned();
    let graph = check(vec![both(), unused, both2]).expect("the fixtures check");
    let semantic = graph.semantic_graph();
    let parameters_of = |name: &str| {
        let key = graph.function_identity(name).expect("declared");
        preimage(semantic.node(key).expect("the function node"))["body"]["members"][0]["value"]
            ["members"]
            .as_array()
            .expect("a parameters aggregate")
            .iter()
            .map(|member| member["target"]["digest"].as_str().expect("a reference").to_owned())
            .collect::<Vec<_>>()
    };
    let expected = vec![vector_key("P1"), vector_key("P2")];
    assert_eq!(parameters_of("unused"), expected);
    assert_eq!(parameters_of("both2"), expected);
}

/// TC-414 step 3 (FR-092-AC-6, FR-065-AC-8): no function body holds an
/// application; every function is a structural-node key carrying the
/// owner; `a and b` is an application-node key with no owner, E1.
#[trace("FR-092-AC-6", "TC-414", "FR-065-AC-8", "TC-163")]
#[test]
fn functions_are_structural_and_applications_carry_no_owner() {
    let graph = check(vec![both(), f(), h(), nb()]).expect("the fixtures check");
    let semantic = graph.semantic_graph();
    for name in ["both", "f", "h", "nb"] {
        let node = semantic
            .node(graph.function_identity(name).expect("declared"))
            .expect("the function node");
        let json = preimage(node);
        assert_eq!(json["version"], "quire.structural-node/v1", "{name}");
        assert_eq!(
            json["owner"],
            json!({"kind": "source", "authority": "a", "identity": "u"}),
            "{name}"
        );
        assert!(
            !serde_json::to_string(&json["body"])
                .expect("JSON")
                .contains("\"application\""),
            "{name}: a function body holds references only"
        );
    }
    let e1 = node_by_key(semantic, &vector_key("E1"));
    assert_vector(semantic, e1.key(), "E1");
    let e1 = preimage(e1);
    assert_eq!(e1["version"], "quire.application-node/v1");
    assert!(e1.get("owner").is_none());
    // FR-065-AC-8: `both` is F2 and the call `both(a, true)` is E2.
    assert_eq!(
        graph.function_identity("both").map(|key| key.to_string()),
        Some(vector_key("F2"))
    );
    let e2 = node_by_key(semantic, &vector_key("E2"));
    assert_vector(semantic, e2.key(), "E2");
    assert!(preimage(e2).get("owner").is_none());
}

/// TC-414 step 4 (FR-092-AC-10): `m`'s parameter is P4, `m` is F3, and its
/// `decreases` binding references P4.
#[trace("FR-092-AC-10", "TC-414")]
#[test]
fn a_measure_is_a_decreases_reference() {
    let m = function(
        "m",
        &[("x", int_form(0, 9))],
        boolean(),
        Some(name_expr("x")),
        Expression::Boolean(true),
    );
    let graph = check(vec![m]).expect("m checks");
    let semantic = graph.semantic_graph();
    let f3 = graph.function_identity("m").expect("declared");
    assert_vector(semantic, f3, "F3");
    assert_vector(semantic, node_by_key(semantic, &vector_key("P4")).key(), "P4");
    let decreases = &preimage(semantic.node(f3).unwrap())["body"]["members"][2];
    assert_eq!(decreases["name"], "decreases");
    assert_eq!(decreases["value"]["target"]["digest"], json!(vector_key("P4")));
}

/// TC-415 step 1 (FR-093-AC-1): three equal literals are one node with
/// three `expression` occurrences in source order, and the conditional's
/// arguments are three references to it.
#[trace("FR-093-AC-1", "TC-415")]
#[test]
fn equal_literals_are_one_node_with_one_occurrence_each() {
    let graph = check(vec![t()]).expect("t checks");
    let semantic = graph.semantic_graph();
    let literals: Vec<&SemanticNode> = semantic
        .nodes()
        .filter(|node| node.semantic_form() == "literal")
        .collect();
    assert_eq!(literals.len(), 1);
    let literal = literals[0].key();
    assert_eq!(literal.to_string(), vector_key("L1"));
    let conditionals: Vec<&SemanticNode> = semantic
        .nodes()
        .filter(|node| node.semantic_form() == "conditional")
        .collect();
    assert_eq!(conditionals.len(), 1);
    let conditional = preimage(conditionals[0]);
    assert_eq!(conditional["body"]["operation"]["identity"], "quire.op.control.if");
    assert_eq!(conditional["semantic_type"]["digest"], json!(vector_key("T1")));
    assert_eq!(conditional["body"]["result_type"]["digest"], json!(vector_key("T1")));
    let arguments = conditional["body"]["arguments"].as_array().unwrap();
    assert_eq!(arguments.len(), 3);
    for argument in arguments {
        assert_eq!(argument["target"]["digest"], json!(literal.to_string()));
    }
    let occurrences: Vec<Location> = (0..3)
        .map(|ordinal| {
            graph
                .occurrence(literal, &quire_exact::Origin::new(quire_exact::Role::new("expression"), ordinal))
                .expect("an expression occurrence")
                .clone()
        })
        .collect();
    let paths: Vec<Vec<usize>> = occurrences.into_iter().map(|location| location.path).collect();
    assert_eq!(paths, vec![vec![0], vec![1], vec![2]], "source order");
    assert!(graph
        .occurrence(literal, &quire_exact::Origin::new(quire_exact::Role::new("expression"), 3))
        .is_none());
}

/// TC-415 step 2 (FR-093-AC-2): `a and b`, `both(a, true)` and `let y = a in
/// y` are E1, E2 and E3; a local read is a reference, never a node.
#[trace("FR-093-AC-2", "TC-415")]
#[test]
fn applications_match_e1_to_e3_and_local_reads_build_no_node() {
    let graph = check(vec![both(), nb(), h()]).expect("the fixtures check");
    let semantic = graph.semantic_graph();
    for name in ["E1", "E2", "E3"] {
        assert_vector(semantic, node_by_key(semantic, &vector_key(name)).key(), name);
    }
    let parameters: Vec<String> = ["P1", "P2", "P3"].iter().map(|name| vector_key(name)).collect();
    for node in semantic.nodes() {
        let json = preimage(node);
        assert!(
            !(json["body"]["term"] == "reference"
                && parameters.contains(&json["body"]["target"]["digest"].as_str().unwrap_or("").to_owned())),
            "no node's body is a bare local read"
        );
    }
}

/// The node whose body's operation identity is `identity`.
fn application<'g>(graph: &'g SemanticGraph, identity: &str) -> Json {
    let found: Vec<Json> = graph
        .nodes()
        .map(preimage)
        .filter(|json| json["body"]["operation"]["identity"] == identity)
        .collect();
    assert_eq!(found.len(), 1, "one `{identity}` node");
    found.into_iter().next().unwrap()
}

/// TC-415 step 4 (FR-093-AC-4): a contained integer builds no convert node;
/// a narrowing is `numeric.narrow` naming `Int[0, 9]`; a numeric conversion
/// is `numeric.convert`; `flatMap` is one `flat_map` node with no map node.
#[trace("FR-093-AC-4", "TC-415")]
#[test]
fn conversions_are_classified_and_flat_map_builds_one_node() {
    let c1 = function("c1", &[("x", int_form(0, 9))], int_form(0, 10), None, name_expr("x"));
    let c2 = function("c2", &[], int_form(0, 9), None, integer_expr(3));
    let c3 = function(
        "c3",
        &[("x", int_form(0, 9))],
        TypeForm::builtin(BuiltinType::Rational, SPAN)
            .with_bounds(vec!["0".into(), "9".into(), "1".into(), "1".into()]),
        None,
        Expression::Convert {
            target: TypeForm::builtin(BuiltinType::Rational, SPAN)
                .with_bounds(vec!["0".into(), "9".into(), "1".into(), "1".into()]),
            operand: Box::new(name_expr("x")),
        },
    );
    let s = TypeForm::collection(CollectionKind::Sequence, SPAN)
        .with_arguments(vec![int_form(0, 9)])
        .with_bounds(vec!["0".into(), "5".into()]);
    // FR-093-AC-4 writes the body `sequence[x]`, a collection literal the
    // checker cannot type without a hint; the fixture flat-maps `s` itself.
    let result = TypeForm::collection(CollectionKind::Sequence, SPAN)
        .with_arguments(vec![int_form(0, 9)])
        .with_bounds(vec!["0".into(), "25".into()]);
    let flat = function(
        "flat",
        &[("s", s)],
        result,
        None,
        Expression::Query {
            query: BinderQuery::FlatMap,
            binder: "x".to_owned(),
            source: Box::new(name_expr("s")),
            body: Box::new(name_expr("s")),
        },
    );

    let graph = check(vec![c1]).expect("c1 checks");
    let semantic = graph.semantic_graph();
    let c1_node = preimage(semantic.node(graph.function_identity("c1").unwrap()).unwrap());
    let x = parameter_named(semantic, "x").key();
    assert_eq!(c1_node["body"]["members"][1]["value"]["target"]["digest"], json!(x.to_string()));
    assert!(semantic.nodes().all(|node| node.semantic_form() != "conversion"));

    let graph = check(vec![c2]).expect("c2 checks");
    let narrow = application(graph.semantic_graph(), "quire.op.numeric.narrow");
    let t4 = vector_key("T4");
    assert_eq!(narrow["body"]["operation"]["member"]["kind"], "type_argument");
    assert_eq!(narrow["body"]["operation"]["member"]["declaration"]["digest"], json!(t4));
    assert_eq!(narrow["semantic_form"], "conversion");

    let graph = check(vec![c3]).expect("c3 checks");
    application(graph.semantic_graph(), "quire.op.numeric.convert");

    let graph = check(vec![flat]).expect("flat checks");
    application(graph.semantic_graph(), "quire.op.collection.flat_map");
    assert!(graph.semantic_graph().nodes().map(preimage).all(|json| json["body"]["operation"]
        ["identity"]
        != "quire.op.collection.map"));
}

/// TC-415 step 5 (FR-093-AC-5): binder levels count the binders in scope;
/// siblings share a level; `forall`'s arguments are the source and a
/// binding of its body.
#[trace("FR-093-AC-5", "TC-415")]
#[test]
fn binder_levels_count_enclosing_binders() {
    let s = TypeForm::collection(CollectionKind::Sequence, SPAN)
        .with_arguments(vec![int_form(0, 9)])
        .with_bounds(vec!["0".into(), "5".into()]);
    let query = |query: BinderQuery, binder: &str, body: Expression| Expression::Query {
        query,
        binder: binder.to_owned(),
        source: Box::new(name_expr("s")),
        body: Box::new(body),
    };
    let q = function(
        "q",
        &[("s", s)],
        boolean(),
        None,
        binary(
            BinaryOperator::And,
            query(
                BinderQuery::Forall,
                "x",
                query(
                    BinderQuery::Exists,
                    "y",
                    binary(BinaryOperator::Equal, name_expr("x"), name_expr("y")),
                ),
            ),
            query(BinderQuery::Exists, "z", Expression::Boolean(true)),
        ),
    );
    let graph = check(vec![q]).expect("q checks");
    let semantic = graph.semantic_graph();
    let level = |name: &str| preimage(parameter_named(semantic, name))["body"]["members"][1]["value"]["value"].clone();
    assert_eq!(level("x"), json!("1"));
    assert_eq!(level("y"), json!("2"));
    assert_eq!(level("z"), json!("1"));
    let forall = application(semantic, "quire.op.collection.forall");
    let arguments = &forall["body"]["arguments"];
    assert_eq!(
        arguments[0]["target"]["digest"],
        json!(parameter_named(semantic, "s").key().to_string())
    );
    assert_eq!(arguments[1]["term"], "binding");
    assert_eq!(arguments[1]["name"], "x");
    let exists = semantic
        .nodes()
        .map(preimage)
        .find(|json| {
            json["body"]["operation"]["identity"] == "quire.op.collection.exists"
                && json["body"]["arguments"][1]["name"] == "y"
        })
        .expect("the inner exists node");
    let exists_key = semantic
        .nodes()
        .find(|node| preimage(node) == exists)
        .unwrap()
        .key();
    assert_eq!(arguments[1]["value"]["target"]["digest"], json!(exists_key.to_string()));
}

/// TC-415 step 6 (FR-093-AC-6): a text equality carries the lock
/// evidence's `text_profile` law and the operands' profile; with no such
/// evidence it refuses `missing_declaration`/`missing-selection` naming the
/// role, and yields no node.
#[trace("FR-093-AC-6", "TC-415")]
#[test]
fn a_law_comes_only_from_the_lock_evidence() {
    let text = || {
        TypeForm::builtin(BuiltinType::Text, SPAN)
            .with_bounds(vec!["0".into(), "8".into(), "nfc".into()])
    };
    let te = function(
        "te",
        &[("p", text()), ("r", text())],
        boolean(),
        None,
        binary(BinaryOperator::Equal, name_expr("p"), name_expr("r")),
    );
    let refusals = check(vec![te.clone()]).expect_err("no text-profile evidence refuses");
    assert_eq!(refusals.len(), 1, "{refusals:?}");
    assert_eq!(
        refusals[0].cause,
        CheckCause::MissingSelection {
            role: LawRole::TextProfile
        }
    );
    assert_eq!(refusals[0].cause.code().as_str(), "missing_declaration");
    assert_eq!(refusals[0].cause.cause(), Some("missing-selection"));

    let definition = DefinitionReference {
        authority: "agent-ix".to_owned(),
        identity: "unicode-text".to_owned(),
        revision: crate::value::definition::DefinitionRevision {
            namespace: "unicode".to_owned(),
            value: "17.0.0".to_owned(),
        },
        digest_domain: "quire.definition.bytes/v1".to_owned(),
        digest: "ab".repeat(32),
    };
    let graph = PackageDeclarations {
        functions: vec![te],
        lock_evidence: LockEvidence::default().with_text_profile(definition.clone()),
        ..PackageDeclarations::new(fixture_owner())
    }
    .check(CheckingLimits::default())
    .expect("text-profile evidence admits the equality");
    let equality = application(graph.semantic_graph(), "quire.op.text.eq");
    let operation = &equality["body"]["operation"];
    assert_eq!(operation["laws"][0]["role"], "text_profile");
    assert_eq!(
        operation["laws"][0]["definition"],
        serde_json::to_value(&definition).unwrap()
    );
    assert_eq!(operation["mode"], json!({"kind": "text_profile", "value": "nfc"}));
}

/// TC-415 step 3 (FR-093-AC-3): one fixture per `Value`-family row. Each
/// lowers to one node of the row's operator, form, identity and argument
/// count, whose `result_type` is the type node of the checked node's type.
#[trace("FR-093-AC-3", "TC-415")]
#[test]
fn value_family_rows_lower_to_their_catalogued_operations() {
    let integer = || TypeForm::builtin(BuiltinType::Integer, SPAN);
    let s = || {
        TypeForm::collection(CollectionKind::Sequence, SPAN)
            .with_arguments(vec![int_form(0, 9)])
            .with_bounds(vec!["0".into(), "5".into()])
    };
    let set = || {
        TypeForm::collection(CollectionKind::Set, SPAN)
            .with_arguments(vec![int_form(0, 9)])
            .with_bounds(vec!["0".into(), "5".into()])
    };
    let option = || TypeForm::builtin(BuiltinType::Option, SPAN).with_arguments(vec![int_form(0, 9)]);
    // (fixture, operation identity, operator, semantic form, arguments)
    let rows: Vec<(FunctionDeclaration, &str, &str, &str, usize)> = vec![
        (
            function("add", &[("x", integer()), ("y", integer())], integer(), None,
                binary(BinaryOperator::Add, name_expr("x"), name_expr("y"))),
            "quire.op.integer.add", "binary", "binary", 2,
        ),
        (
            function("sub", &[("x", integer()), ("y", integer())], integer(), None,
                binary(BinaryOperator::Subtract, name_expr("x"), name_expr("y"))),
            "quire.op.integer.sub", "binary", "binary", 2,
        ),
        (
            function("mul", &[("x", integer()), ("y", integer())], integer(), None,
                binary(BinaryOperator::Multiply, name_expr("x"), name_expr("y"))),
            "quire.op.integer.mul", "binary", "binary", 2,
        ),
        (
            function("neg", &[("x", integer())], integer(), None,
                Expression::Negate(Box::new(name_expr("x")))),
            "quire.op.integer.negate", "unary", "unary", 1,
        ),
        (
            function("lt", &[("x", integer()), ("y", integer())], boolean(), None,
                binary(BinaryOperator::Less, name_expr("x"), name_expr("y"))),
            "quire.op.integer.lt", "binary", "binary", 2,
        ),
        (
            function("ge", &[("x", integer()), ("y", integer())], boolean(), None,
                binary(BinaryOperator::GreaterOrEqual, name_expr("x"), name_expr("y"))),
            "quire.op.integer.ge", "binary", "binary", 2,
        ),
        (
            function("ieq", &[("x", integer()), ("y", integer())], boolean(), None,
                binary(BinaryOperator::Equal, name_expr("x"), name_expr("y"))),
            "quire.op.integer.eq", "binary", "binary", 2,
        ),
        (
            function("bne", &[("a", boolean()), ("b", boolean())], boolean(), None,
                binary(BinaryOperator::NotEqual, name_expr("a"), name_expr("b"))),
            "quire.op.boolean.ne", "binary", "binary", 2,
        ),
        (
            function("seq", &[("s", s()), ("r", s())], boolean(), None,
                binary(BinaryOperator::Equal, name_expr("s"), name_expr("r"))),
            "quire.op.structural.eq", "binary", "binary", 2,
        ),
        (
            function("or", &[("a", boolean()), ("b", boolean())], boolean(), None,
                binary(BinaryOperator::Or, name_expr("a"), name_expr("b"))),
            "quire.op.boolean.or", "binary", "binary", 2,
        ),
        (
            function("imp", &[("a", boolean()), ("b", boolean())], boolean(), None,
                binary(BinaryOperator::Implies, name_expr("a"), name_expr("b"))),
            "quire.op.boolean.implies", "binary", "binary", 2,
        ),
        (
            function("not", &[("a", boolean())], boolean(), None,
                Expression::Not(Box::new(name_expr("a")))),
            "quire.op.boolean.not", "unary", "unary", 1,
        ),
        (
            function("pres", &[("o", option())], boolean(), None,
                Expression::Present(Box::new(name_expr("o")))),
            "quire.op.option.present", "present", "presence_read", 1,
        ),
        (
            function("val", &[("o", option())], int_form(0, 9), None,
                Expression::If {
                    condition: Box::new(Expression::Present(Box::new(name_expr("o")))),
                    then: Box::new(Expression::Value(Box::new(name_expr("o")))),
                    otherwise: Box::new(integer_expr(0)),
                }),
            "quire.op.option.value", "value", "value_read", 1,
        ),
        (
            function("mk", &[("x", int_form(0, 9))], s(), None,
                Expression::Collection { kind: CollectionKind::Sequence, elements: vec![name_expr("x"), name_expr("x")] }),
            "quire.op.collection.sequence", "collection", "collection", 2,
        ),
        (
            function("mkset", &[("x", int_form(0, 9))], set(), None,
                Expression::Collection { kind: CollectionKind::Set, elements: vec![name_expr("x")] }),
            "quire.op.collection.set", "collection", "collection", 1,
        ),
        (
            function("map", &[("s", s())], s(), None,
                Expression::Query { query: BinderQuery::Map, binder: "x".into(), source: Box::new(name_expr("s")), body: Box::new(name_expr("x")) }),
            "quire.op.collection.map", "collection", "collection", 2,
        ),
        (
            function("filter", &[("s", s())], s(), None,
                Expression::Query { query: BinderQuery::Filter, binder: "x".into(), source: Box::new(name_expr("s")), body: Box::new(Expression::Boolean(true)) }),
            "quire.op.collection.filter", "collection", "collection", 2,
        ),
        (
            function("exists", &[("s", s())], boolean(), None,
                Expression::Query { query: BinderQuery::Exists, binder: "x".into(), source: Box::new(name_expr("s")), body: Box::new(Expression::Boolean(true)) }),
            "quire.op.collection.exists", "quantify", "quantify", 2,
        ),
        (
            function("size", &[("s", s())], integer(), None,
                Expression::Size(Box::new(name_expr("s")))),
            "quire.op.collection.size", "collection", "collection", 1,
        ),
        (
            function("has", &[("s", s()), ("x", int_form(0, 9))], boolean(), None,
                Expression::Contains { collection: Box::new(name_expr("s")), item: Box::new(name_expr("x")) }),
            "quire.op.collection.contains", "collection", "collection", 2,
        ),
    ];
    for (fixture, identity, operator, form, arity) in rows {
        let name = fixture.name.clone();
        let graph = check(vec![fixture]).unwrap_or_else(|refusals| panic!("{name}: {refusals:?}"));
        let node = application(graph.semantic_graph(), identity);
        assert_eq!(node["body"]["operator"], operator, "{name}");
        assert_eq!(node["semantic_form"], form, "{name}");
        assert_eq!(
            node["body"]["arguments"].as_array().map(Vec::len),
            Some(arity),
            "{name}"
        );
        assert_eq!(node["body"]["result_type"], node["semantic_type"], "{name}");
        assert_eq!(node["version"], "quire.application-node/v1", "{name}");
    }
}

/// FR-093 `Field` row: a record projection names the record's node and the
/// field; a record value binds each slot, an absent optional slot as a
/// `none` literal typed at the field's option node.
#[trace("FR-093-AC-3", "TC-415")]
#[test]
fn record_projection_and_record_values_name_their_record_node() {
    let point = NodeKey::from_digest([4; 32]);
    let types = TypeEnvironment::new(
        [CompositeDeclaration::new(
            point,
            "Point",
            CompositeShape::Record(vec![
                FieldDeclaration::new("x", int(0, 9), Presence::Required),
                FieldDeclaration::new("y", int(0, 9), Presence::Optional),
            ]),
        )],
        [],
    )
    .unwrap();
    let project = function(
        "px",
        &[("p", TypeForm::name("Point", SPAN))],
        int_form(0, 9),
        None,
        Expression::Field {
            operand: Box::new(name_expr("p")),
            field: "x".to_owned(),
        },
    );
    let build = function(
        "mk",
        &[("x", int_form(0, 9))],
        TypeForm::name("Point", SPAN),
        None,
        Expression::Record {
            name: "Point".to_owned(),
            fields: vec![("x".to_owned(), qsl_forms::FieldInitializer::Value(name_expr("x")))],
        },
    );
    let graph = PackageDeclarations {
        types,
        functions: vec![project, build],
        ..PackageDeclarations::new(fixture_owner())
    }
    .check(CheckingLimits::default())
    .expect("the record fixtures check");
    let semantic = graph.semantic_graph();
    let record = semantic
        .nodes()
        .find(|node| node.semantic_form() == "record")
        .expect("the record's node")
        .key();
    let projection = application(semantic, "quire.op.record.project");
    assert_eq!(
        projection["body"]["operation"]["member"],
        json!({"kind": "field", "declaration": {"domain": "quire.checked-semantic-node/v1", "digest": record.to_string()}, "name": "x"})
    );
    let value = semantic
        .nodes()
        .find(|node| node.semantic_form() == "record_value")
        .map(preimage)
        .expect("the record value node");
    assert_eq!(value["semantic_type"]["digest"], json!(record.to_string()));
    let y = &value["body"]["members"][1];
    assert_eq!(y["name"], "y");
    assert_eq!(y["value"]["value_kind"], "none");
    assert_eq!(y["value"]["value"], Json::Null);
    assert_eq!(y["value"]["type"]["digest"], json!(vector_key("T5")));
}
