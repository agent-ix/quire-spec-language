// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-415 step 3 (FR-093-AC-3): the application rows whose operation
//! carries a member, a mode, laws or leaves, each checked through
//! `PackageDeclarations::check` and compared with its FR-093 row in full.

use qsl_forms::Accumulation;

use super::*;
use crate::check::check::EnumBinding;
use crate::value::definition::{CatalogRole, DefinitionLock, DefinitionRevision};

/// The text-profile definition the lock evidence of these fixtures selects.
fn text_definition() -> DefinitionReference {
    DefinitionReference {
        authority: "agent-ix".to_owned(),
        identity: "unicode-text".to_owned(),
        revision: DefinitionRevision {
            namespace: "unicode".to_owned(),
            value: "17.0.0".to_owned(),
        },
        digest_domain: "quire.definition.bytes/v1".to_owned(),
        digest: "ab".repeat(32),
    }
}

/// The pinned lock's IEEE profile definition.
fn ieee_definition() -> DefinitionReference {
    let entry = DefinitionLock::pinned()
        .entry(CatalogRole::IeeeProfile)
        .expect("the pinned lock names an IEEE profile");
    DefinitionReference {
        authority: entry.authority.to_owned(),
        identity: entry.identity.to_owned(),
        revision: DefinitionRevision {
            namespace: entry.revision_namespace.to_owned(),
            value: entry.revision_value.to_owned(),
        },
        digest_domain: "quire.definition.bytes/v1".to_owned(),
        digest: "0".repeat(64),
    }
}

fn law(role: LawRole, definition: DefinitionReference) -> Json {
    serde_json::to_value(OperationLaw { role, definition }).expect("a law serializes")
}

fn text_law() -> Json {
    law(LawRole::TextProfile, text_definition())
}

fn ieee_law() -> Json {
    law(LawRole::IeeeProfile, ieee_definition())
}

/// A `NodeRef` to `key`.
fn node_ref(key: NodeKey) -> Json {
    json!({"domain": "quire.checked-semantic-node/v1", "digest": key.to_string()})
}

/// The key of an anonymous type's node, the same in every package.
fn type_key(value_type: &ValueType) -> NodeKey {
    type_nodes(
        &empty_scope(),
        &crate::check::SourceOwner::from(&fixture_source()),
        std::slice::from_ref(value_type),
    )
    .1[0]
}

fn type_argument(value_type: &ValueType) -> Json {
    json!({"kind": "type_argument", "declaration": node_ref(type_key(value_type))})
}

fn rounding(mode: &str) -> Json {
    json!({"kind": "rounding", "value": mode})
}

fn nfc() -> Json {
    json!({"kind": "text_profile", "value": "nfc"})
}

/// One text leaf at `path` under the `nfc` profile.
fn nfc_leaf(path: &[&str]) -> Json {
    json!({"path": path, "laws": [text_law()], "mode": nfc()})
}

fn builtin_form(builtin: BuiltinType, bounds: &[&str]) -> TypeForm {
    TypeForm::builtin(builtin, SPAN).with_bounds(bounds.iter().map(|b| (*b).to_owned()).collect())
}

fn rational_form(bounds: [&str; 4]) -> TypeForm {
    builtin_form(BuiltinType::Rational, &bounds)
}

fn decimal_form(bounds: [&str; 5]) -> TypeForm {
    builtin_form(BuiltinType::Decimal, &bounds)
}

fn text_form() -> TypeForm {
    builtin_form(BuiltinType::Text, &["0", "8", "nfc"])
}

fn float64() -> TypeForm {
    TypeForm::builtin(BuiltinType::Float64, SPAN)
}

fn collection_form(kind: CollectionKind, element: TypeForm, maximum: &str) -> TypeForm {
    TypeForm::collection(kind, SPAN)
        .with_arguments(vec![element])
        .with_bounds(vec!["0".into(), maximum.into()])
}

fn text_type() -> ValueType {
    ValueType::Text(TextType::new(0, 8, TextProfile::Nfc).unwrap())
}

fn decimal(lower: i64, upper: i64, scale: u64, mode: RoundingMode) -> ValueType {
    ValueType::Decimal(
        DecimalType::new(
            Integer::from(lower),
            Integer::from(upper),
            scale,
            scale,
            mode,
        )
        .unwrap(),
    )
}

/// One FR-093 row: the package, the operation it lowers to and the
/// operation's full expected member, mode, laws and leaves.
struct Row {
    package: PackageDeclarations,
    identity: &'static str,
    operator: &'static str,
    form: &'static str,
    member: Json,
    mode: Json,
    laws: Json,
    leaves: Json,
    arity: usize,
}

/// A package holding `functions`, with the text-profile lock evidence, the
/// pinned IEEE profile and the accumulator aliases these rows use.
fn package(functions: Vec<FunctionDeclaration>) -> PackageDeclarations {
    let ieee = DefinitionLock::pinned()
        .admit_ieee_profile(&[ieee_definition()], &[])
        .expect("the pinned IEEE profile admits");
    PackageDeclarations {
        functions,
        aliases: vec![
            ("Total".to_owned(), ValueType::Integer),
            ("Small".to_owned(), int(0, 5)),
        ],
        lock_evidence: LockEvidence::default().with_text_profile(text_definition()),
        ieee_profile: Some(ieee),
        ..PackageDeclarations::new(fixture_source())
    }
}

fn row(
    function: FunctionDeclaration,
    identity: &'static str,
    (operator, form): (&'static str, &'static str),
    (member, mode, laws, leaves): (Json, Json, Json, Json),
    arity: usize,
) -> Row {
    Row {
        package: package(vec![function]),
        identity,
        operator,
        form,
        member,
        mode,
        laws,
        leaves,
        arity,
    }
}

const NONE: (Json, Json, Json, Json) = (
    Json::Null,
    Json::Null,
    Json::Array(Vec::new()),
    Json::Array(Vec::new()),
);

fn rows() -> Vec<Row> {
    let r19 = || rational_form(["1", "9", "1", "9"]);
    let d = || decimal_form(["0", "100", "2", "2", "nearest-even"]);
    let texts = || collection_form(CollectionKind::Sequence, text_form(), "5");
    let ints = || collection_form(CollectionKind::Sequence, int_form(0, 9), "5");
    let binary_op = ("binary", "binary");
    let unary_op = ("unary", "unary");
    let convert_op = ("convert", "conversion");
    let collection_op = ("collection", "collection");
    // A reduction needs a nonempty source.
    let totals = || {
        TypeForm::collection(CollectionKind::Sequence, SPAN)
            .with_arguments(vec![TypeForm::builtin(BuiltinType::Integer, SPAN)])
            .with_bounds(vec!["1".into(), "5".into()])
    };
    let accumulate = |name: &str, form: Accumulation, identity: Option<Expression>| {
        function(
            name,
            &[("s", totals())],
            TypeForm::name("Total", SPAN),
            None,
            Expression::Accumulate {
                accumulator_type_span: qsl_foundation::Span { start: 0, end: 0 },
                form,
                accumulator_type: "Total".to_owned(),
                accumulator: "acc".to_owned(),
                binder: "x".to_owned(),
                source: Box::new(name_expr("s")),
                step: Box::new(binary(
                    BinaryOperator::Add,
                    name_expr("acc"),
                    name_expr("x"),
                )),
                identity: identity.map(Box::new),
            },
        )
    };
    vec![
        row(
            function(
                "rdiv",
                &[("x", int_form(1, 9)), ("y", int_form(1, 9))],
                rational_form(["-1000", "1000", "1", "1000"]),
                None,
                binary(BinaryOperator::Divide, name_expr("x"), name_expr("y")),
            ),
            "quire.op.rational.div",
            binary_op,
            NONE,
            2,
        ),
        row(
            function(
                "radd",
                &[("x", r19()), ("y", r19())],
                rational_form(["-1000", "1000", "1", "1000"]),
                None,
                binary(BinaryOperator::Add, name_expr("x"), name_expr("y")),
            ),
            "quire.op.rational.add",
            binary_op,
            NONE,
            2,
        ),
        row(
            function(
                "rneg",
                &[("x", r19())],
                rational_form(["-9", "9", "1", "9"]),
                None,
                Expression::Negate(Box::new(name_expr("x"))),
            ),
            "quire.op.rational.negate",
            unary_op,
            NONE,
            1,
        ),
        row(
            function(
                "dadd",
                &[("p", d()), ("q", d())],
                decimal_form(["0", "20000", "2", "2", "nearest-even"]),
                None,
                binary(BinaryOperator::Add, name_expr("p"), name_expr("q")),
            ),
            "quire.op.decimal.add",
            binary_op,
            (Json::Null, rounding("nearest-even"), json!([]), json!([])),
            2,
        ),
        row(
            function(
                "dneg",
                &[("p", d())],
                decimal_form(["-100", "0", "2", "2", "nearest-even"]),
                None,
                Expression::Negate(Box::new(name_expr("p"))),
            ),
            "quire.op.decimal.negate",
            unary_op,
            NONE,
            1,
        ),
        row(
            function(
                "dround",
                &[("p", d())],
                decimal_form(["0", "1000", "1", "1", "toward-zero"]),
                None,
                Expression::Convert {
                    target: decimal_form(["0", "1000", "1", "1", "toward-zero"]),
                    operand: Box::new(name_expr("p")),
                },
            ),
            "quire.op.numeric.convert_rounding",
            convert_op,
            (
                type_argument(&decimal(0, 1000, 1, RoundingMode::TowardZero)),
                rounding("toward-zero"),
                json!([]),
                json!([]),
            ),
            1,
        ),
        row(
            function(
                "fadd",
                &[("f", float64()), ("g", float64())],
                float64(),
                None,
                binary(BinaryOperator::Add, name_expr("f"), name_expr("g")),
            ),
            "quire.op.ieee.float64.add",
            binary_op,
            (
                Json::Null,
                rounding("exact"),
                json!([ieee_law()]),
                json!([]),
            ),
            2,
        ),
        row(
            function(
                "frat",
                &[("f", float64())],
                rational_form(["-9", "9", "1", "9"]),
                None,
                Expression::Convert {
                    target: rational_form(["-9", "9", "1", "9"]),
                    operand: Box::new(name_expr("f")),
                },
            ),
            "quire.op.ieee.to_rational",
            convert_op,
            (
                type_argument(&rational_9()),
                Json::Null,
                json!([ieee_law()]),
                json!([]),
            ),
            1,
        ),
        row(
            function(
                "cconv",
                &[("s", texts())],
                collection_form(CollectionKind::Set, text_form(), "5"),
                None,
                Expression::Convert {
                    target: collection_form(CollectionKind::Set, text_form(), "5"),
                    operand: Box::new(name_expr("s")),
                },
            ),
            "quire.op.collection.convert",
            convert_op,
            (
                type_argument(&ValueType::collection(CollectionType::new(
                    CollectionKind::Set,
                    text_type(),
                    Some(CardinalityBound::new(0, 5).unwrap()),
                ))),
                Json::Null,
                json!([]),
                json!([nfc_leaf(&[])]),
            ),
            1,
        ),
        row(
            function(
                "count",
                &[("s", ints())],
                TypeForm::name("Small", SPAN),
                None,
                Expression::Count {
                    result_type_span: qsl_foundation::Span { start: 0, end: 0 },
                    result_type: "Small".to_owned(),
                    binder: "x".to_owned(),
                    source: Box::new(name_expr("s")),
                    predicate: Box::new(Expression::Boolean(true)),
                },
            ),
            "quire.op.collection.count",
            collection_op,
            (type_argument(&int(0, 5)), Json::Null, json!([]), json!([])),
            2,
        ),
        row(
            function(
                "sum",
                &[("s", ints())],
                TypeForm::name("Total", SPAN),
                None,
                Expression::Sum {
                    result_type_span: qsl_foundation::Span { start: 0, end: 0 },
                    result_type: "Total".to_owned(),
                    binder: "x".to_owned(),
                    source: Box::new(name_expr("s")),
                    summand: Box::new(name_expr("x")),
                },
            ),
            "quire.op.collection.sum.integer",
            collection_op,
            (
                type_argument(&ValueType::Integer),
                Json::Null,
                json!([]),
                json!([]),
            ),
            2,
        ),
        row(
            function(
                "flat",
                &[(
                    "ss",
                    collection_form(
                        CollectionKind::Set,
                        collection_form(CollectionKind::Set, text_form(), "2"),
                        "3",
                    ),
                )],
                collection_form(CollectionKind::Set, text_form(), "6"),
                None,
                Expression::Flatten(Box::new(name_expr("ss"))),
            ),
            "quire.op.collection.flatten",
            collection_op,
            (Json::Null, Json::Null, json!([]), json!([nfc_leaf(&[])])),
            1,
        ),
        row(
            accumulate("fold", Accumulation::Fold, Some(integer_expr(0))),
            "quire.op.collection.fold",
            collection_op,
            (
                type_argument(&ValueType::Integer),
                Json::Null,
                json!([]),
                json!([]),
            ),
            3,
        ),
        row(
            accumulate("reduce", Accumulation::Reduce, None),
            "quire.op.collection.reduce",
            collection_op,
            (
                type_argument(&ValueType::Integer),
                Json::Null,
                json!([]),
                json!([]),
            ),
            2,
        ),
        row(
            function(
                "tlt",
                &[("p", text_form()), ("r", text_form())],
                boolean(),
                None,
                binary(BinaryOperator::Less, name_expr("p"), name_expr("r")),
            ),
            "quire.op.text.lt",
            binary_op,
            (Json::Null, nfc(), json!([text_law()]), json!([])),
            2,
        ),
        row(
            function(
                "oeq",
                &[
                    (
                        "o",
                        TypeForm::builtin(BuiltinType::Option, SPAN)
                            .with_arguments(vec![text_form()]),
                    ),
                    (
                        "e",
                        TypeForm::builtin(BuiltinType::Option, SPAN)
                            .with_arguments(vec![text_form()]),
                    ),
                ],
                boolean(),
                None,
                binary(BinaryOperator::Equal, name_expr("o"), name_expr("e")),
            ),
            "quire.op.structural.eq",
            binary_op,
            (
                Json::Null,
                Json::Null,
                json!([]),
                json!([nfc_leaf(&["inner"])]),
            ),
            2,
        ),
        row(
            function(
                "tset",
                &[("t", text_form())],
                collection_form(CollectionKind::Set, text_form(), "5"),
                None,
                Expression::Collection {
                    kind: CollectionKind::Set,
                    elements: vec![name_expr("t")],
                },
            ),
            "quire.op.collection.set",
            collection_op,
            (Json::Null, Json::Null, json!([]), json!([nfc_leaf(&[])])),
            1,
        ),
        row(
            function(
                "thas",
                &[("s", texts()), ("t", text_form())],
                boolean(),
                None,
                Expression::Contains {
                    collection: Box::new(name_expr("s")),
                    item: Box::new(name_expr("t")),
                },
            ),
            "quire.op.collection.contains",
            collection_op,
            (Json::Null, Json::Null, json!([]), json!([nfc_leaf(&[])])),
            2,
        ),
        {
            let mut enum_row = row(
                function(
                    "senum",
                    &[
                        ("a", TypeForm::name("Status", SPAN)),
                        ("b", TypeForm::name("Status", SPAN)),
                    ],
                    boolean(),
                    None,
                    binary(BinaryOperator::Equal, name_expr("a"), name_expr("b")),
                ),
                "quire.op.enum.eq",
                binary_op,
                NONE,
                2,
            );
            enum_row.package.enums = vec![status()];
            enum_row
        },
    ]
}

/// `enum Status { Active, Closed }`, admitted under QSpec's own preimages.
fn status() -> EnumBinding {
    enum_binding("Status", &["Active", "Closed"])
}

/// `enum <name> { <cases> }`, unordered, admitted under QSpec's own
/// preimages. `cases` must be sorted.
fn enum_binding(name: &str, cases: &[&str]) -> EnumBinding {
    use crate::value::enumeration::{EnumDeclaration, EnumDeclarationPreimage, EnumMemberPreimage};
    use crate::value::semantic_node::{
        NodeIdentityPreimage, NodeOwner, OwnerSelection, OwnerSubject,
    };
    let owners = OwnerSelection::new([NodeOwner::Definition(OwnerSubject {
        authority: "agent-ix".to_owned(),
        identity: "example-model".to_owned(),
    })]);
    let preimage = EnumDeclarationPreimage::from_json(json!({
        "version": "quire.enum-declaration-node/v1",
        "owner": {"kind": "definition", "authority": "agent-ix", "identity": "example-model"},
        "qualified_declaration": ["Example", name],
        "ordered": false,
        "members": cases,
    }))
    .unwrap();
    let key = NodeKey::from_digest(preimage.digest().unwrap());
    let declaration = EnumDeclaration::admit(preimage, key, &owners).unwrap();
    let members = cases
        .iter()
        .map(|case| {
            let member = EnumMemberPreimage::from_json(json!({
                "version": "quire.enum-member-node/v1",
                "declaration_node_id": node_ref(key),
                "case": case,
            }))
            .unwrap();
            let member_key = NodeKey::from_digest(member.digest().unwrap());
            declaration.admit_member(&member, member_key).unwrap()
        })
        .collect();
    EnumBinding {
        name: name.to_owned(),
        declaration,
        members,
    }
}

/// `functions` functions `fK(x: Country): Boolean { true }` over one
/// 250-variant `Country` enum: each parameter's preimage writes every
/// variant.
fn enum_parameter_package(functions: usize) -> PackageDeclarations {
    let cases: Vec<String> = (0..250).map(|case| format!("C{case:03}")).collect();
    let cases: Vec<&str> = cases.iter().map(String::as_str).collect();
    PackageDeclarations {
        enums: vec![enum_binding("Country", &cases)],
        functions: (0..functions)
            .map(|index| {
                function(
                    &format!("f{index}"),
                    &[("x", TypeForm::name("Country", SPAN))],
                    boolean(),
                    None,
                    Expression::Boolean(true),
                )
            })
            .collect(),
        ..PackageDeclarations::new(fixture_source())
    }
}

/// TC-423 step 4 (NFR-011-M-3, NFR-011-M-4): 4,000 functions over one
/// 250-variant enum parameter -- about 180 KB of source, inside NFR-001 --
/// check at the default ceilings, though each declaration's preimage
/// writes all 250 variants. With the work budget at its default ratio to
/// the preimage byte ceiling (one work unit per byte), a declaration whose
/// preimage passes the byte ceiling refuses on the byte ceiling, not on
/// the work budget.
#[trace("NFR-011-M-3", "NFR-011-M-4", "TC-423")]
#[test]
fn preimage_bytes_bind_before_the_work_budget() {
    enum_parameter_package(4_000)
        .check(CheckingLimits::default())
        .expect("4,000 enum-parameter functions check at the default ceilings");
    let bytes = 10_000;
    let refusals = enum_parameter_package(2)
        .check(
            CheckingLimits::default()
                .with_input_bytes(bytes)
                .with_work_budget(bytes),
        )
        .expect_err("one declaration's preimage passes 10,000 bytes");
    assert!(
        matches!(
            refusals[0].cause,
            CheckCause::ResourceExhausted(ref exceeded)
                if exceeded.stage == CheckingStage::Typing
                    && exceeded.kind == CheckingLimitKind::InputBytes
                    && exceeded.limit == 10_000
        ),
        "{:?}",
        refusals[0]
    );
    assert_eq!(refusals[0].cause.code().as_str(), "stage_limit_exceeded");
    assert_eq!(refusals[0].cause.cause(), Some("input-bytes-exceeded"));
}

/// TC-415 step 3 (FR-093-AC-3): each row lowers to one node with the row's
/// operator, form, identity, member, mode, laws, leaves and argument count,
/// whose `result_type` is its semantic type.
#[trace("FR-093-AC-3", "TC-415")]
#[test]
fn value_family_rows_carry_their_member_mode_laws_and_leaves() {
    for row in rows() {
        let name = row.package.functions[0].name.clone();
        let graph = row
            .package
            .check(CheckingLimits::default())
            .unwrap_or_else(|refusals| panic!("{name}: {refusals:?}"));
        let node = application(graph.semantic_graph(), row.identity);
        let operation = &node["body"]["operation"];
        assert_eq!(node["body"]["operator"], row.operator, "{name}");
        assert_eq!(node["semantic_form"], row.form, "{name}");
        assert_eq!(operation["member"], row.member, "{name}: member");
        assert_eq!(operation["mode"], row.mode, "{name}: mode");
        assert_eq!(operation["laws"], row.laws, "{name}: laws");
        assert_eq!(operation["leaves"], row.leaves, "{name}: leaves");
        assert_eq!(
            node["body"]["arguments"].as_array().map(Vec::len),
            Some(row.arity),
            "{name}"
        );
        assert_eq!(node["body"]["result_type"], node["semantic_type"], "{name}");
    }
}

/// FR-093 `Tuple` row: `Pair(x, y)` is a `value`/`tuple_value` node typed
/// at the tuple's node, whose body references each argument in order.
#[trace("FR-093-AC-3", "TC-415")]
#[test]
fn a_tuple_value_is_a_value_node_over_its_arguments() {
    let pair = NodeKey::from_digest([7; 32]);
    let types = TypeEnvironment::new(
        [CompositeDeclaration::new(
            pair,
            "Pair",
            CompositeShape::Tuple(vec![int(0, 9), ValueType::Boolean]),
        )],
        [],
    )
    .unwrap();
    let make = function(
        "mk",
        &[("x", int_form(0, 9)), ("b", boolean())],
        TypeForm::name("Pair", SPAN),
        None,
        Expression::Call {
            name: "Pair".to_owned(),
            arguments: vec![name_expr("x"), name_expr("b")],
        },
    );
    let checked = PackageDeclarations {
        types,
        functions: vec![make],
        ..PackageDeclarations::new(fixture_source())
    }
    .check(CheckingLimits::default())
    .expect("the tuple fixture checks");
    let graph = checked.semantic_graph();
    let tuple = graph
        .nodes()
        .find(|node| node.semantic_form() == "tuple")
        .expect("the tuple's node")
        .key();
    let value = graph
        .nodes()
        .find(|node| node.semantic_form() == "tuple_value")
        .map(preimage)
        .expect("the tuple value node");
    assert_eq!(value["node_tag"], "value");
    assert_eq!(value["semantic_type"], node_ref(tuple));
    let members = &value["body"]["members"];
    assert_eq!(
        members[0]["target"],
        node_ref(parameter_named(graph, "x").key())
    );
    assert_eq!(
        members[1]["target"],
        node_ref(parameter_named(graph, "b").key())
    );
}

/// TC-413 step 7 (FR-092-AC-9): `rational(1, 2)` and `rational(2, 4)` as a
/// `Rational[-9, 9; 1, 9]` literal, through `check`, key to L3.
#[trace("FR-092-AC-9", "TC-413")]
#[test]
fn rational_literals_key_to_l3_through_check() {
    for (numerator, denominator) in [(1_i64, 2_i64), (2, 4)] {
        let literal = function(
            "half",
            &[],
            rational_form(["-9", "9", "1", "9"]),
            None,
            Expression::Rational(Integer::from(numerator), Integer::from(denominator)),
        );
        let checked = check(vec![literal]).expect("the literal checks");
        let graph = checked.semantic_graph();
        let key = node_by_key(graph, &vector_key("L3")).key();
        assert_vector(graph, key, "L3");
    }
}
