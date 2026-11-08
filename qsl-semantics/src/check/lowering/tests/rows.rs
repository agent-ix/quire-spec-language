// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-415 step 3 (FR-093-AC-3): the application rows whose operation
//! carries a member, a mode, laws or leaves, each checked through
//! `PackageDeclarations::check` and compared with its FR-093 row in full.

use qsl_forms::Accumulation;

use super::*;
use crate::check::check::EnumBinding;
use crate::check::Obligation;
use crate::value::definition::{admit_ieee_profile, DefinitionRole};
use quire_exact::IllTypedCause;

/// QSL's text-profile definition, the law a text operation names.
fn text_definition() -> DefinitionReference {
    DefinitionRole::TextProfile.reference()
}

/// The pinned lock's IEEE profile definition.
fn ieee_definition() -> DefinitionReference {
    DefinitionRole::IeeeProfile.reference()
}

fn law(role: LawRole, definition: DefinitionReference) -> Json {
    view::law_json(&OperationLaw { role, definition })
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
        .clone()
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

/// A package holding `functions`, with the
/// pinned IEEE profile and the accumulator aliases these rows use.
fn package(functions: Vec<FunctionDeclaration>) -> PackageDeclarations {
    let ieee =
        admit_ieee_profile(&[ieee_definition()], &[]).expect("the pinned IEEE profile admits");
    PackageDeclarations {
        functions,
        aliases: vec![
            ("Total".to_owned(), ValueType::Integer),
            ("Small".to_owned(), int(0, 5)),
        ],
        ieee_profile: Some(ieee),
        ..PackageDeclarations::new(
            fixture_source(),
            "fixture",
            qsl_foundation::IdentityLimits::default(),
        )
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
            Expression::accumulate(
                form,
                qsl_forms::DeclaredName {
                    name: "Total".to_owned(),
                    span: qsl_foundation::Span { start: 0, end: 0 },
                },
                "acc".to_owned(),
                "x".to_owned(),
                name_expr("s"),
                binary(BinaryOperator::Add, name_expr("acc"), name_expr("x")),
                identity,
            ),
        )
    };
    vec![
        row(
            function(
                "rdiv",
                &[("x", r19()), ("y", r19())],
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
                Expression::negate(name_expr("x")),
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
                decimal_form(["0", "100", "2", "2", "nearest-even"]),
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
                Expression::negate(name_expr("p")),
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
                Expression::convert(
                    decimal_form(["0", "1000", "1", "1", "toward-zero"]),
                    name_expr("p"),
                ),
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
                Expression::convert(rational_form(["-9", "9", "1", "9"]), name_expr("f")),
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
                Expression::convert(
                    collection_form(CollectionKind::Set, text_form(), "5"),
                    name_expr("s"),
                ),
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
                Expression::count(
                    qsl_forms::DeclaredName {
                        name: "Small".to_owned(),
                        span: qsl_foundation::Span { start: 0, end: 0 },
                    },
                    "x".to_owned(),
                    name_expr("s"),
                    Expression::boolean(true),
                ),
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
                Expression::sum(
                    qsl_forms::DeclaredName {
                        name: "Total".to_owned(),
                        span: qsl_foundation::Span { start: 0, end: 0 },
                    },
                    "x".to_owned(),
                    name_expr("s"),
                    name_expr("x"),
                ),
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
                Expression::flatten(name_expr("ss")),
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
                Expression::collection(CollectionKind::Set, vec![name_expr("t")]),
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
                Expression::contains(name_expr("s"), name_expr("t")),
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
    use crate::value::enumeration::{
        AdmittedEnumDeclaration, EnumDeclarationPreimage, EnumMemberPreimage,
    };
    use crate::value::semantic_node::{NodeOwner, OwnerSelection, OwnerSubject};
    let subject = OwnerSubject::new("agent-ix", "example-model", "example-model");
    let owners = OwnerSelection::new([NodeOwner::Definition(subject.clone())]);
    let preimage = EnumDeclarationPreimage::new(
        NodeOwner::Definition(subject),
        vec!["Example".to_owned(), name.to_owned()],
        false,
        cases.iter().map(|case| (*case).to_owned()).collect(),
    )
    .unwrap();
    let key = preimage.key().unwrap();
    let declaration = AdmittedEnumDeclaration::admit(preimage, key.clone(), &owners).unwrap();
    let members = cases
        .iter()
        .map(|case| {
            let member = EnumMemberPreimage::new(key.clone(), *case).unwrap();
            let member_key = member.key().unwrap();
            declaration.admit_member(&member, member_key).unwrap()
        })
        .collect();
    EnumBinding {
        name: name.to_owned(),
        declaration,
        members,
    }
}

/// FR-092-AC-19: two bindings of one admitted enum declaration whose names
/// resolve different source owners build one declaration node, and its
/// source enum row takes the owner of the binding that built the node, so the
/// row's source authority and identity equal the node's owner.
#[trace("FR-092-AC-19", "TC-1021")]
#[test]
fn a_second_binding_of_one_enum_keeps_the_first_owner_in_its_row() {
    let first = SourceOwner::new("first", "unit-a").expect("a source owner");
    let second = SourceOwner::new("second", "unit-b").expect("a source owner");
    let status = status();
    let alias = EnumBinding {
        name: "Alias".to_owned(),
        ..status.clone()
    };
    let owners = std::collections::BTreeMap::from([
        (status.name.clone(), first.clone()),
        (alias.name.clone(), second),
    ]);
    let scope = crate::check::family::fixtures::empty_scope();
    let unit = SourceOwner::from(&fixture_source());
    let mut occurrences = OccurrenceMap::default();
    let mut meter = quire_exact::Meter::new(crate::check::family::SCALAR_LIMITS_UNLIMITED);
    let mut lowering = Lowering::new_with_source_owners(
        &scope,
        &unit,
        &owners,
        &[],
        &[],
        scope.types().units().clone(),
        None,
        0,
        &mut occurrences,
        &mut meter,
        "fixture",
    );
    lowering
        .enum_nodes(&status)
        .expect("the first binding lowers");
    lowering
        .enum_nodes(&alias)
        .expect("the second binding lowers");

    let key = status.declaration.key();
    let node = lowering
        .graph
        .node(key.clone())
        .expect("one declaration node");
    assert_eq!(node.content.owner, Some(Owner::Source(first.clone())));
    let row = lowering
        .graph
        .source_enum_declaration(&key)
        .expect("one source enum row");
    assert_eq!(row.owner, first);
    assert_eq!(lowering.graph.source_enum_declarations().count(), 1);
    assert_eq!(
        lowering.graph.source_enum_members().count(),
        status.members.len()
    );
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
                    Expression::boolean(true),
                )
            })
            .collect(),
        ..PackageDeclarations::new(
            fixture_source(),
            "fixture",
            qsl_foundation::IdentityLimits::default(),
        )
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
    assert_rows(rows());
}

/// Each row lowers to one node with the row's operator, form, identity,
/// member, mode, laws, leaves and argument count, whose `result_type` is its
/// semantic type.
fn assert_rows(rows: Vec<Row>) {
    for row in rows {
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

/// TC-1335 step 1 (FR-093-AC-59): `dropFirst` lowers to the one-occurrence
/// identity of its operand's kind with no law, mode, member or leaf, and
/// `removeOne` to `remove_one` with the leaves `contains` lists, each over
/// its operands and typed by the node of the derived type.
#[trace("FR-093-AC-59", "TC-1335", "FR-091-AC-3")]
#[test]
fn one_occurrence_forms_lower_to_their_fr_322_identities() {
    let collection_op = ("collection", "collection");
    let ints = |kind, maximum: &str| collection_form(kind, int_form(0, 9), maximum);
    let one_row = |name: &str,
                   parameters: &[(&str, TypeForm)],
                   result: TypeForm,
                   body: Expression,
                   identity: &'static str,
                   leaves: Json,
                   arity: usize| {
        row(
            function(name, parameters, result, None, body),
            identity,
            collection_op,
            (Json::Null, Json::Null, json!([]), leaves),
            arity,
        )
    };
    assert_rows(vec![
        one_row(
            "f1",
            &[("s", ints(CollectionKind::Sequence, "9"))],
            ints(CollectionKind::Sequence, "8"),
            Expression::drop_first(name_expr("s")),
            "quire.op.collection.sequence.drop_first",
            json!([]),
            1,
        ),
        one_row(
            "f2",
            &[("o", ints(CollectionKind::OrderedSet, "5"))],
            ints(CollectionKind::OrderedSet, "4"),
            Expression::drop_first(name_expr("o")),
            "quire.op.collection.ordered_set.drop_first",
            json!([]),
            1,
        ),
        one_row(
            "g1",
            &[("s", ints(CollectionKind::Sequence, "9"))],
            ints(CollectionKind::Sequence, "9"),
            Expression::remove_one(name_expr("s"), integer_expr(3)),
            "quire.op.collection.remove_one",
            json!([]),
            2,
        ),
        one_row(
            "g2",
            &[
                ("t", collection_form(CollectionKind::Bag, text_form(), "9")),
                ("x", text_form()),
            ],
            collection_form(CollectionKind::Bag, text_form(), "9"),
            Expression::remove_one(name_expr("t"), name_expr("x")),
            "quire.op.collection.remove_one",
            json!([nfc_leaf(&[])]),
            2,
        ),
    ]);
}

/// FR-093 `Tuple` row: `Pair(x, y)` is a `value`/`tuple_value` node typed
/// at the tuple's node, whose body references each argument in order.
#[trace("FR-093-AC-3", "TC-415")]
#[test]
fn a_tuple_value_is_a_value_node_over_its_arguments() {
    let pair = fixture_key("pair");
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
        Expression::call("Pair".to_owned(), vec![name_expr("x"), name_expr("b")]),
    );
    let checked = PackageDeclarations {
        types,
        functions: vec![make],
        ..PackageDeclarations::new(
            fixture_source(),
            "fixture",
            qsl_foundation::IdentityLimits::default(),
        )
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
/// `Rational[-9, 9; 1, 9]` literal, through `check`, give the same one
/// literal node.
#[trace("FR-092-AC-9", "TC-413")]
#[test]
fn rational_literals_are_one_literal_node_through_check() {
    let mut keys = Vec::new();
    for (numerator, denominator) in [(1_i64, 2_i64), (2, 4)] {
        let literal = function(
            "half",
            &[],
            rational_form(["-9", "9", "1", "9"]),
            None,
            Expression::rational(Integer::from(numerator), Integer::from(denominator)),
        );
        let checked = check(vec![literal]).expect("the literal checks");
        let literals: Vec<NodeKey> = checked
            .semantic_graph()
            .nodes()
            .filter(|node| node.semantic_form() == "literal")
            .map(SemanticNode::key)
            .collect();
        assert_eq!(literals.len(), 1);
        keys.push(literals[0].clone());
    }
    assert_eq!(keys[0], keys[1], "an equal rational is one node");
}

/// The result of checking `rational(n, d)` where a `Rational[-1, 1; 1, 2]`
/// is expected: the one literal node's body, or the causes it refused with.
fn rational_literal_outcome(numerator: i64, denominator: i64) -> Result<Json, Vec<CheckCause>> {
    let literal = function(
        "lit",
        &[],
        rational_form(["-1", "1", "1", "2"]),
        None,
        Expression::rational(Integer::from(numerator), Integer::from(denominator)),
    );
    match check(vec![literal]) {
        Ok(checked) => {
            let literals: Vec<Json> = checked
                .semantic_graph()
                .nodes()
                .filter(|node| node.semantic_form() == "literal")
                .map(preimage)
                .collect();
            assert_eq!(literals.len(), 1, "{numerator}/{denominator}");
            Ok(literals[0]["body"].clone())
        }
        Err(refusals) => Err(refusals.into_iter().map(|refusal| refusal.cause).collect()),
    }
}

/// QSpec FR-039-AC-1 and AC-3: a rational literal normalizes by the gcd with
/// the sign moved to the numerator and a zero numerator as `0/1`, before the
/// authored bounds are checked, so `2/4`, `-2/-4` and `0/-7` all check and
/// equal values are one node; every inclusive boundary of the numerator range
/// [-1, 1] and denominator range [1, 2] admits.
#[trace("QSpec-FR-039-AC-1", "QSpec-FR-039-AC-3", "QSpec-TC-039")]
#[test]
fn rational_literals_normalize_before_the_bounds_and_the_boundaries_admit() {
    let spelled = |numerator, denominator| {
        rational_literal_outcome(numerator, denominator)
            .unwrap_or_else(|causes| panic!("{numerator}/{denominator}: {causes:?}"))
    };
    for (numerator, denominator, normalized) in [
        (2, 4, (1, 2)),
        (-2, -4, (1, 2)),
        (0, -7, (0, 1)),
        (-1, 1, (-1, 1)),
        (1, 1, (1, 1)),
        (-1, 2, (-1, 2)),
        (1, 2, (1, 2)),
        (0, 1, (0, 1)),
        (4, 4, (1, 1)),
    ] {
        assert_eq!(
            spelled(numerator, denominator),
            spelled(normalized.0, normalized.1),
            "{numerator}/{denominator} is {}/{}",
            normalized.0,
            normalized.1
        );
    }
    assert_ne!(spelled(1, 2), spelled(1, 1), "distinct values differ");
}

/// QSpec FR-039-AC-2 and AC-3: `rational(1,0)` and `rational(0,0)` refuse
/// as an unproved nonzero denominator, and `rational(2,1)`, `rational(1,3)`,
/// `rational(-3,2)` and `rational(-2,-1)` outside the authored ranges refuse
/// as an unproved rational range, never as a value.
#[trace("QSpec-FR-039-AC-2", "QSpec-FR-039-AC-3", "QSpec-TC-039")]
#[test]
fn zero_denominators_and_out_of_range_literals_refuse_with_distinct_causes() {
    for (numerator, denominator, obligation) in [
        (1, 0, Obligation::Nonzero),
        (0, 0, Obligation::Nonzero),
        (2, 1, Obligation::RationalRange),
        (1, 3, Obligation::RationalRange),
        (-3, 2, Obligation::RationalRange),
        (-2, -1, Obligation::RationalRange),
    ] {
        assert_eq!(
            rational_literal_outcome(numerator, denominator),
            Err(vec![CheckCause::Unproved(obligation)]),
            "rational({numerator},{denominator})"
        );
    }
}

/// QSpec FR-039-AC-5: normalizing a rational literal is charged against
/// `s3.rational_bits` by the larger written bit length. With the limit at 64
/// bits, a numerator of 65 bits (`2^64`) or a denominator of 65 bits refuses
/// as a `RationalBits` stage limit (resource incompleteness), never a value,
/// widening or Boolean; the 64-bit literals just inside admit.
#[trace("QSpec-FR-039-AC-5", "QSpec-TC-039")]
#[test]
fn rational_literal_normalization_exhausting_the_bits_limit_is_a_stage_limit() {
    let power = |bits: u64| Integer::from(2_i64).pow(&Integer::from(bits));
    let outcome = |numerator: Integer, denominator: Integer| {
        let literal = function(
            "lit",
            &[],
            rational_form(["-9", "9", "1", "9"]),
            None,
            Expression::rational(numerator, denominator),
        );
        PackageDeclarations {
            functions: vec![literal],
            ..PackageDeclarations::new(
                fixture_source(),
                "fixture",
                qsl_foundation::IdentityLimits::default(),
            )
        }
        .check(CheckingLimits::default().with_rational_bits(64))
    };
    let exhausted = |actual: u128| {
        CheckCause::ResourceExhausted(Box::new(StageLimitCause {
            stage: CheckingStage::Typing,
            kind: CheckingLimitKind::RationalBits,
            limit: 64,
            actual,
            region: None,
        }))
    };
    let causes = |numerator, denominator| {
        outcome(numerator, denominator)
            .expect_err("the normalization exceeds the limit")
            .into_iter()
            .map(|refusal| refusal.cause)
            .collect::<Vec<_>>()
    };
    assert_eq!(
        causes(power(64), power(64)),
        [exhausted(65)],
        "a 65-bit numerator and denominator"
    );
    assert_eq!(causes(Integer::from(1_i64), power(64)), [exhausted(65)]);
    // Just inside: 2^63 / 2^63 has 64-bit parts and normalizes to 1/1.
    outcome(power(63), power(63)).expect("64-bit parts admit");
}

/// QSpec FR-039-AC-4: a literal with no expected rational type, or one
/// compared with an integer, refuses rather than choosing a domain.
#[trace("QSpec-FR-039-AC-4", "QSpec-TC-039")]
#[test]
fn a_rational_literal_never_picks_its_own_domain() {
    let half = || Expression::rational(Integer::from(1_i64), Integer::from(2_i64));
    let refused = |parameters: &[(&str, TypeForm)], body: Expression| {
        let checked = check(vec![function("lit", parameters, boolean(), None, body)]);
        checked
            .expect_err("the literal picks no domain")
            .into_iter()
            .map(|refusal| refusal.cause)
            .collect::<Vec<_>>()
    };
    assert_eq!(
        refused(&[], binary(BinaryOperator::Equal, half(), half())),
        [CheckCause::IllTyped(IllTypedCause::AmbiguousLiteral)]
    );
    let fraction = rational_form(["-9", "9", "1", "9"]);
    assert_eq!(
        refused(
            &[("p", fraction)],
            binary(BinaryOperator::Equal, name_expr("p"), integer_expr(1))
        ),
        [CheckCause::IllTyped(IllTypedCause::TypeMismatch)]
    );
    assert_eq!(
        refused(
            &[("i", int_form(-9, 9))],
            binary(BinaryOperator::Equal, name_expr("i"), half())
        ),
        [CheckCause::IllTyped(IllTypedCause::TypeMismatch)]
    );
}

/// TC-1232 step 3 (QSpec FR-322 decimal literal spelling): a `decimal(c, s)`
/// literal lowers to one `decimal` literal node spelled with exactly its
/// written `s` fractional digits, so `decimal(125, 2)` is `"1.25"`,
/// `decimal(5, 2)` is `"0.05"`, `decimal(10, 0)` is `"10"`, `decimal(0, 2)`
/// is `"0.00"` and `decimal(-5, 1)` is `"-0.5"`. A zero coefficient is a
/// member of every type whatever its scale, and spells at that scale
/// without a formatting width, so 65534 and 65535 digits build and a larger
/// scale is refused `stage_limit_exceeded`/`work-budget-exceeded`.
#[trace("FR-093-AC-38", "FR-093-AC-39", "TC-1232")]
#[test]
fn a_decimal_constructor_lowers_with_its_written_scale_and_a_scale_past_65535_is_a_stage_limit() {
    let bounds = ["-99999", "99999", "0", "2", "nearest-even"];
    let lowered = |coefficient: i64, scale: u32| {
        let literal = function(
            "lit",
            &[],
            decimal_form(bounds),
            None,
            Expression::decimal(Integer::from(coefficient), scale),
        );
        let checked = check(vec![literal])?;
        let literals: Vec<Json> = checked
            .semantic_graph()
            .nodes()
            .filter(|node| node.semantic_form() == "literal")
            .map(preimage)
            .collect();
        assert_eq!(literals.len(), 1, "{coefficient}/{scale}");
        Ok::<_, Vec<CheckRefusal>>(literals[0]["body"].clone())
    };
    for (coefficient, scale, text) in [
        (125, 2, "1.25"),
        (5, 2, "0.05"),
        (10, 0, "10"),
        (0, 2, "0.00"),
        (-5, 1, "-0.5"),
        (1, 0, "1"),
    ] {
        let body = lowered(coefficient, scale).expect("the member literal checks");
        assert_eq!(body["value_kind"], "decimal", "{coefficient}/{scale}");
        assert_eq!(body["value"], text, "{coefficient}/{scale}");
    }
    for scale in [65_534_u32, 65_535] {
        let body = lowered(0, scale).expect("a zero literal of that scale checks");
        let value = body["value"].as_str().expect("a spelled value");
        assert_eq!(value.len(), usize::try_from(scale).unwrap() + 2, "{scale}");
        assert!(value.starts_with("0.") && value[2..].bytes().all(|each| each == b'0'));
    }
    for scale in [65_536_u32, u32::MAX] {
        let refusals = lowered(0, scale).expect_err("a scale past the literal scale limit");
        let [refusal] = refusals.as_slice() else {
            panic!("one refusal, got {refusals:?}");
        };
        assert_eq!(
            refusal.cause.code().as_str(),
            "stage_limit_exceeded",
            "{scale}"
        );
        assert_eq!(
            refusal.cause.cause(),
            Some("work-budget-exceeded"),
            "{scale}"
        );
    }
}
