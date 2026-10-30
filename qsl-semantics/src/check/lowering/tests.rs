// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-413, TC-414 and TC-415: the FR-092 keys and the FR-093 lowering,
//! checked against FR-092's golden vectors. The vectors are read from this
//! repository's own FR-092 text at compile time, so the spec and the test
//! cannot drift apart.

use std::collections::{BTreeMap, BTreeSet};

use ix_trace_rs::trace;
use qsl_forms::{
    BinaryOperator, BinderQuery, BuiltinType, ExprNode, Expression, FunctionDeclaration, TypeForm,
};
use quire_exact::{
    CardinalityBound, CollectionType, DecimalType, IntegerInterval, LimitKind as MeterLimitKind,
    Meter, Presence, RationalDomain, RoundingMode, ScalarLimits, TextType,
};
use serde_json::{json, Value as Json};

use super::*;
use crate::check::depth_forms::{binary, boolean, function, int_form, integer_expr, name_expr};
use crate::check::family::fixtures::{admitted_source, empty_scope, fixture_source, scope_with};
use crate::check::{CheckedGraph, CheckedTypeNode, PackageDeclarations};
use qsl_foundation::source::provenance::RawSourceRef;
use qsl_foundation::SourceIdentity;
use quire_semantic_value::checking::CheckingLimits;
use quire_semantic_value::declaration::{CompositeDeclaration, FieldDeclaration, TypeEnvironment};

mod binder_scope;
mod deep_bodies;
mod depth;
mod differential;
mod expression_depth;
mod identity_depth;
mod leaves;
mod rows;
mod wide_integers;

const SPAN: qsl_foundation::Span = qsl_foundation::Span { start: 0, end: 0 };

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
        Some(CardinalityBound::new(minimum, maximum).unwrap()),
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
    let mut occurrences = OccurrenceMap::default();
    let location = generated_location();
    let mut meter = quire_exact::Meter::new(crate::check::family::SCALAR_LIMITS_UNLIMITED);
    let mut lowering = Lowering::new(
        scope,
        owner,
        &[],
        scope.types().units().clone(),
        None,
        0,
        &mut occurrences,
        &mut meter,
        qsl_foundation::IdentityLimits::default(),
    );
    let keys = value_types
        .iter()
        .map(|value_type| {
            lowering
                .type_node(value_type, &location)
                .expect("the type keys")
        })
        .collect();
    (lowering.finish(&location).graph, keys)
}

/// Every node key of the package holding `function p(x: T): Boolean {
/// true }` for `T` 100,000 `Option`s around `Boolean`, checked and dropped
/// on a thread with a `stack`-byte stack, under limits raised to fit it.
fn deep_option_keys(stack: usize) -> BTreeSet<NodeKey> {
    std::thread::Builder::new()
        .stack_size(stack)
        .spawn(|| {
            let parameter = (0..100_000).fold(boolean(), |inner, _| {
                TypeForm::builtin(BuiltinType::Option, SPAN).with_arguments(vec![inner])
            });
            let limits = CheckingLimits::new(u64::MAX)
                .with_input_bytes(u64::MAX)
                .with_work_budget(u64::MAX);
            let checked = PackageDeclarations {
                functions: vec![function(
                    "p",
                    &[("x", parameter)],
                    boolean(),
                    None,
                    Expression::boolean(true),
                )],
                ..PackageDeclarations::new(
                    fixture_source(),
                    qsl_foundation::IdentityLimits::default(),
                )
            }
            .check(limits)
            .unwrap_or_else(|refusals| panic!("the deep option parameter checks: {refusals:?}"));
            let options = checked
                .semantic_graph()
                .nodes()
                .filter(|node| node.semantic_form() == "option")
                .count();
            assert_eq!(options, 100_000);
            checked
                .semantic_graph()
                .nodes()
                .map(SemanticNode::key)
                .collect()
        })
        .expect("the check thread spawns")
        .join()
        .expect("the check completes")
}

/// TC-413 step 5 (FR-092-AC-7, FR-258-AC-2): FR-092's type-keying walk keys
/// a parameter typed with 100,000 nested `Option`s around `Boolean` on a
/// 512 KiB stack, to the keys the same package has on an 8 MiB stack.
#[trace("FR-092-AC-7", "TC-413")]
#[trace("FR-258-AC-2", "TC-726")]
#[test]
fn a_100000_deep_option_type_keys_on_a_small_stack() {
    assert_eq!(
        deep_option_keys(512 * 1024),
        deep_option_keys(8 * 1024 * 1024)
    );
}

/// `function name(x: Int[0, 9]): Boolean decreases(x) { if x > 0 then
/// callee(x - 1) else true }`.
fn recursive(name: &str, callee: &str) -> FunctionDeclaration {
    function(
        name,
        &[("x", int_form(0, 9))],
        boolean(),
        Some(name_expr("x")),
        Expression::if_then_else(
            binary(BinaryOperator::Greater, name_expr("x"), integer_expr(0)),
            Expression::call(
                callee.to_owned(),
                vec![binary(
                    BinaryOperator::Subtract,
                    name_expr("x"),
                    integer_expr(1),
                )],
            ),
            Expression::boolean(true),
        ),
    )
}

/// `record List { next: List?; }`, with no text field.
fn list_types() -> TypeEnvironment {
    let list = NodeKey::from_digest([5; 32]);
    TypeEnvironment::new(
        [CompositeDeclaration::new(
            list,
            "List",
            CompositeShape::Record(vec![FieldDeclaration::new(
                "next",
                ValueType::Composite(list),
                Presence::Optional,
            )]),
        )],
        [],
    )
    .expect("FR-143 admits List")
}

/// TC-415 (FR-093-AC-3): structural equality and `contains` over the
/// recursive `List`, which has no text leaf, check and carry no leaves; the
/// leaf walk stops at the recursion instead of running to the depth limit.
#[trace("FR-093-AC-3", "TC-415")]
#[test]
fn equality_and_contains_over_a_recursive_record_without_text_have_no_leaves() {
    let list = || TypeForm::name("List", SPAN);
    let eq = function(
        "eq",
        &[("a", list()), ("b", list())],
        boolean(),
        None,
        binary(BinaryOperator::Equal, name_expr("a"), name_expr("b")),
    );
    let lists = TypeForm::collection(CollectionKind::Sequence, SPAN)
        .with_arguments(vec![list()])
        .with_bounds(vec!["0".into(), "3".into()]);
    let has = function(
        "has",
        &[("s", lists), ("a", list())],
        boolean(),
        None,
        Expression::contains(name_expr("s"), name_expr("a")),
    );
    let checked = PackageDeclarations {
        types: list_types(),
        functions: vec![eq, has],
        ..PackageDeclarations::new(fixture_source(), qsl_foundation::IdentityLimits::default())
    }
    .check(CheckingLimits::default())
    .expect("equality and contains over List check");
    for identity in ["quire.op.structural.eq", "quire.op.collection.contains"] {
        let node = application(checked.semantic_graph(), identity);
        assert_eq!(node["body"]["operation"]["leaves"], json!([]), "{identity}");
    }
}

/// TC-413 step 9 (FR-092-AC-11): in `h`, `h(x - 1) and h(x - 1)` calls `h`
/// through one node, so `h`'s group has four nodes.
#[trace("FR-092-AC-11", "TC-413")]
#[test]
fn equal_recursive_calls_are_one_node() {
    let call = || {
        Expression::call(
            "h".to_owned(),
            vec![binary(
                BinaryOperator::Subtract,
                name_expr("x"),
                integer_expr(1),
            )],
        )
    };
    let h = function(
        "h",
        &[("x", int_form(0, 9))],
        boolean(),
        Some(name_expr("x")),
        Expression::if_then_else(
            binary(BinaryOperator::Greater, name_expr("x"), integer_expr(0)),
            binary(BinaryOperator::And, call(), call()),
            Expression::boolean(true),
        ),
    );
    let checked = check(vec![h]).expect("h checks");
    let members: Vec<&SemanticNode> = checked
        .semantic_graph()
        .nodes()
        .filter(|node| node.recursion().is_some())
        .collect();
    assert_eq!(
        members.len(),
        4,
        "h, the conditional, the conjunction and the call"
    );
    assert!(members
        .iter()
        .all(|node| node.recursion().map(|recursion| recursion.size()) == Some(4)));
    let mut forms: Vec<&str> = members.iter().map(|node| node.semantic_form()).collect();
    forms.sort_unstable();
    assert_eq!(
        forms,
        ["binary", "call", "conditional", "recursive_function"]
    );
}

/// A ring of `k` functions, `r0` calling `r1` ... calling `r0`, all with
/// the body of `recursive` except the last, whose `else` is `false`.
fn ring(k: usize) -> Vec<FunctionDeclaration> {
    (0..k)
        .map(|at| {
            let mut function = recursive(&format!("r{at}"), &format!("r{}", (at + 1) % k));
            if at + 1 == k {
                // The body's one Boolean literal is its `else`.
                function.body = function
                    .body
                    .respelled(|_, node| {
                        if let ExprNode::Boolean(value) = node {
                            *value = false;
                        }
                    })
                    .expect("a literal rewrite keeps the shape");
            }
            function
        })
        .collect()
}

/// The work the checking stage's declaration checks charge for `functions`
/// (`family::measure_declaration`, one `declaration.check` charge each).
fn declaration_work(functions: &[FunctionDeclaration]) -> u64 {
    let scope = empty_scope();
    let location = generated_location();
    let targets = crate::check::family::TargetTypes::new(&scope, &location);
    functions
        .iter()
        .map(|function| {
            let signature = crate::check::check::Signature {
                name: function.name.clone(),
                parameters: vec![("x".to_owned(), int(0, 9))],
                result: ValueType::Boolean,
                callable_by_name: true,
            };
            crate::check::family::measure_declaration(function, &signature, &targets)
                .expect("the declaration measures")
                .work_budget
        })
        .sum()
}

/// FR-092 "The group order": keying a recursion group charges the checking
/// stage's work budget (`CheckingLimits::work_budget`) for every node built
/// and every refinement round, so a ring of mutually recursive functions
/// whose declarations fit the budget refuses `resource_exhausted` on the
/// work budget when its keying does not.
#[trace("FR-092-AC-11", "TC-413")]
#[test]
fn keying_a_recursion_group_is_charged_to_the_work_budget() {
    let k = 8;
    let checked = |budget: u64| {
        PackageDeclarations {
            functions: ring(k),
            ..PackageDeclarations::new(fixture_source(), qsl_foundation::IdentityLimits::default())
        }
        .check(CheckingLimits::default().with_work_budget(budget))
    };
    assert!(checked(u64::MAX).is_ok(), "the ring checks unbounded");
    let declared = declaration_work(&ring(k));
    let refusals = checked(declared).expect_err("keying the ring exceeds the budget");
    assert!(
        refusals.iter().any(|refusal| matches!(
            refusal.cause,
            CheckCause::ResourceExhausted(ref exceeded)
                if exceeded.kind == CheckingLimitKind::WorkBudget && exceeded.limit == declared
        )),
        "{refusals:?}"
    );
    // The smallest budget that checks the ring: the keying's share grows
    // with rounds times members, beyond one unit per node.
    let (mut refused, mut admitted) = (declared, declared.saturating_mul(64));
    assert!(checked(admitted).is_ok());
    while admitted - refused > 1 {
        let middle = refused + (admitted - refused) / 2;
        if checked(middle).is_ok() {
            admitted = middle;
        } else {
            refused = middle;
        }
    }
    let keying = admitted - declared;
    let members = u64::try_from(3 * k).unwrap();
    assert!(
        keying > members,
        "the group keying charges each member: {keying}"
    );
    assert!(checked(0).is_err(), "zero work refuses the group");
    assert!(
        checked(admitted).is_ok(),
        "the exact required budget admits"
    );
    let one_short = checked(admitted - 1).expect_err("one unit short refuses");
    assert!(
        one_short.iter().any(|refusal| matches!(
            refusal.cause,
            CheckCause::ResourceExhausted(ref exceeded)
                if exceeded.kind == CheckingLimitKind::WorkBudget
                    && exceeded.limit == admitted - 1
        )),
        "{one_short:?}"
    );
}

/// `charge_work`'s refusal reports `actual` as exactly the
/// meter's already-consumed work plus this one denied charge, not the
/// configured bound. Mutating `charge_work` to report `limit` as `actual`
/// (the bug this guards) fails this assertion, since the budget here is
/// configured strictly below `consumed + charge`.
#[test]
fn charge_work_refusal_names_consumed_plus_this_charge_as_actual() {
    let limits = ScalarLimits {
        work_units: 100,
        ..crate::check::family::SCALAR_LIMITS_UNLIMITED
    };
    let mut meter = Meter::new(limits);
    charge_work(&mut meter, 40).expect("40 of 100 work units is well within budget");
    let consumed = meter.consumed(MeterLimitKind::WorkUnits);
    let refusal = charge_work(&mut meter, 90).expect_err("40 already consumed plus 90 exceeds 100");
    match refusal {
        NodeKeyRefusal::WorkBudget { limit, actual } => {
            assert_eq!(limit, 100);
            assert_eq!(actual, u128::from(consumed) + 90);
        }
        other => panic!("expected NodeKeyRefusal::WorkBudget, got {other:?}"),
    }
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
    for signature in [
        "fn descend<'v>(",
        "fn expression(",
        "fn lower_node<'n>(",
        "fn literal(",
    ] {
        assert!(
            !body_of(signature).contains("_ =>"),
            "{signature} has a catch-all arm"
        );
    }
}

// ---------------------------------------------------------------------
// Function fixtures, checked through `PackageDeclarations::check`.
// ---------------------------------------------------------------------

fn check_under(
    source: RawSourceRef,
    functions: Vec<FunctionDeclaration>,
) -> Result<CheckedGraph, Vec<CheckRefusal>> {
    PackageDeclarations {
        functions,
        ..PackageDeclarations::new(source, qsl_foundation::IdentityLimits::default())
    }
    .check(CheckingLimits::default())
}

fn check(functions: Vec<FunctionDeclaration>) -> Result<CheckedGraph, Vec<CheckRefusal>> {
    check_under(fixture_source(), functions)
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

/// The node whose body's operation identity is `identity`.
fn application(graph: &SemanticGraph, identity: &str) -> Json {
    let found: Vec<Json> = graph
        .nodes()
        .map(preimage)
        .filter(|json| json["body"]["operation"]["identity"] == identity)
        .collect();
    assert_eq!(found.len(), 1, "one `{identity}` node");
    found.into_iter().next().unwrap()
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
    let query = |query: BinderQuery, binder: &str, body: Expression| {
        Expression::query(query, binder.to_owned(), name_expr("s"), body)
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
            query(BinderQuery::Exists, "z", Expression::boolean(true)),
        ),
    );
    let graph = check(vec![q]).expect("q checks");
    let semantic = graph.semantic_graph();
    let level = |name: &str| {
        preimage(parameter_named(semantic, name))["body"]["members"][1]["value"]["value"].clone()
    };
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
    assert_eq!(
        arguments[1]["value"]["target"]["digest"],
        json!(exists_key.to_string())
    );
}

/// TC-415 step 6 (FR-093-AC-6): a text equality carries QSL's
/// `text_profile` law and the operands' profile.
#[trace("FR-093-AC-6", "TC-415")]
#[test]
fn a_text_equality_carries_the_text_profile_law() {
    let text = || {
        TypeForm::builtin(BuiltinType::Text, SPAN).with_bounds(vec![
            "0".into(),
            "8".into(),
            "nfc".into(),
        ])
    };
    let te = function(
        "te",
        &[("p", text()), ("r", text())],
        boolean(),
        None,
        Expression::logical_not(binary(
            BinaryOperator::Equal,
            name_expr("p"),
            name_expr("r"),
        )),
    );
    let definition = DefinitionRole::TextProfile.reference();
    let graph = PackageDeclarations {
        functions: vec![te],
        ..PackageDeclarations::new(fixture_source(), qsl_foundation::IdentityLimits::default())
    }
    .check(CheckingLimits::default())
    .expect("the text equality checks");
    let equality = application(graph.semantic_graph(), "quire.op.text.eq");
    let operation = &equality["body"]["operation"];
    assert_eq!(operation["laws"][0]["role"], "text_profile");
    assert_eq!(
        operation["laws"][0]["definition"],
        serde_json::to_value(&definition).unwrap()
    );
    assert_eq!(
        operation["mode"],
        json!({"kind": "text_profile", "value": "nfc"})
    );
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
    let option =
        || TypeForm::builtin(BuiltinType::Option, SPAN).with_arguments(vec![int_form(0, 9)]);
    // (fixture, operation identity, operator, semantic form, arguments)
    let rows: Vec<(FunctionDeclaration, &str, &str, &str, usize)> = vec![
        (
            function(
                "add",
                &[("x", integer()), ("y", integer())],
                integer(),
                None,
                binary(BinaryOperator::Add, name_expr("x"), name_expr("y")),
            ),
            "quire.op.integer.add",
            "binary",
            "binary",
            2,
        ),
        (
            function(
                "sub",
                &[("x", integer()), ("y", integer())],
                integer(),
                None,
                binary(BinaryOperator::Subtract, name_expr("x"), name_expr("y")),
            ),
            "quire.op.integer.sub",
            "binary",
            "binary",
            2,
        ),
        (
            function(
                "mul",
                &[("x", integer()), ("y", integer())],
                integer(),
                None,
                binary(BinaryOperator::Multiply, name_expr("x"), name_expr("y")),
            ),
            "quire.op.integer.mul",
            "binary",
            "binary",
            2,
        ),
        (
            function(
                "neg",
                &[("x", integer())],
                integer(),
                None,
                Expression::negate(name_expr("x")),
            ),
            "quire.op.integer.negate",
            "unary",
            "unary",
            1,
        ),
        (
            function(
                "lt",
                &[("x", integer()), ("y", integer())],
                boolean(),
                None,
                binary(BinaryOperator::Less, name_expr("x"), name_expr("y")),
            ),
            "quire.op.integer.lt",
            "binary",
            "binary",
            2,
        ),
        (
            function(
                "ge",
                &[("x", integer()), ("y", integer())],
                boolean(),
                None,
                binary(
                    BinaryOperator::GreaterOrEqual,
                    name_expr("x"),
                    name_expr("y"),
                ),
            ),
            "quire.op.integer.ge",
            "binary",
            "binary",
            2,
        ),
        (
            function(
                "ieq",
                &[("x", integer()), ("y", integer())],
                boolean(),
                None,
                binary(BinaryOperator::Equal, name_expr("x"), name_expr("y")),
            ),
            "quire.op.integer.eq",
            "binary",
            "binary",
            2,
        ),
        (
            function(
                "bne",
                &[("a", boolean()), ("b", boolean())],
                boolean(),
                None,
                binary(BinaryOperator::NotEqual, name_expr("a"), name_expr("b")),
            ),
            "quire.op.boolean.ne",
            "binary",
            "binary",
            2,
        ),
        (
            function(
                "seq",
                &[("s", s()), ("r", s())],
                boolean(),
                None,
                binary(BinaryOperator::Equal, name_expr("s"), name_expr("r")),
            ),
            "quire.op.structural.eq",
            "binary",
            "binary",
            2,
        ),
        (
            function(
                "or",
                &[("a", boolean()), ("b", boolean())],
                boolean(),
                None,
                binary(BinaryOperator::Or, name_expr("a"), name_expr("b")),
            ),
            "quire.op.boolean.or",
            "binary",
            "binary",
            2,
        ),
        (
            function(
                "imp",
                &[("a", boolean()), ("b", boolean())],
                boolean(),
                None,
                binary(BinaryOperator::Implies, name_expr("a"), name_expr("b")),
            ),
            "quire.op.boolean.implies",
            "binary",
            "binary",
            2,
        ),
        (
            function(
                "not",
                &[("a", boolean())],
                boolean(),
                None,
                Expression::logical_not(name_expr("a")),
            ),
            "quire.op.boolean.not",
            "unary",
            "unary",
            1,
        ),
        (
            function(
                "pres",
                &[("o", option())],
                boolean(),
                None,
                Expression::present(name_expr("o")),
            ),
            "quire.op.option.present",
            "present",
            "presence_read",
            1,
        ),
        (
            function(
                "val",
                &[("o", option())],
                int_form(0, 9),
                None,
                Expression::if_then_else(
                    Expression::present(name_expr("o")),
                    Expression::value(name_expr("o")),
                    integer_expr(0),
                ),
            ),
            "quire.op.option.value",
            "value",
            "value_read",
            1,
        ),
        (
            function(
                "mk",
                &[("x", int_form(0, 9))],
                s(),
                None,
                Expression::collection(
                    CollectionKind::Sequence,
                    vec![name_expr("x"), name_expr("x")],
                ),
            ),
            "quire.op.collection.sequence",
            "collection",
            "collection",
            2,
        ),
        (
            function(
                "mkset",
                &[("x", int_form(0, 9))],
                set(),
                None,
                Expression::collection(CollectionKind::Set, vec![name_expr("x")]),
            ),
            "quire.op.collection.set",
            "collection",
            "collection",
            1,
        ),
        (
            function(
                "map",
                &[("s", s())],
                s(),
                None,
                Expression::query(BinderQuery::Map, "x", name_expr("s"), name_expr("x")),
            ),
            "quire.op.collection.map",
            "collection",
            "collection",
            2,
        ),
        (
            function(
                "filter",
                &[("s", s())],
                s(),
                None,
                Expression::query(
                    BinderQuery::Filter,
                    "x",
                    name_expr("s"),
                    Expression::boolean(true),
                ),
            ),
            "quire.op.collection.filter",
            "collection",
            "collection",
            2,
        ),
        (
            function(
                "exists",
                &[("s", s())],
                boolean(),
                None,
                Expression::query(
                    BinderQuery::Exists,
                    "x",
                    name_expr("s"),
                    Expression::boolean(true),
                ),
            ),
            "quire.op.collection.exists",
            "quantify",
            "quantify",
            2,
        ),
        (
            function(
                "size",
                &[("s", s())],
                integer(),
                None,
                Expression::size(name_expr("s")),
            ),
            "quire.op.collection.size",
            "collection",
            "collection",
            1,
        ),
        (
            function(
                "has",
                &[("s", s()), ("x", int_form(0, 9))],
                boolean(),
                None,
                Expression::contains(name_expr("s"), name_expr("x")),
            ),
            "quire.op.collection.contains",
            "collection",
            "collection",
            2,
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
        // FR-093: none of these rows names a law, a mode or a text leaf;
        // only `size` names a member, `type_argument` of its result type.
        let operation = &node["body"]["operation"];
        assert_eq!(operation["laws"], json!([]), "{name}");
        assert_eq!(operation["mode"], Json::Null, "{name}");
        assert_eq!(operation["leaves"], json!([]), "{name}");
        let member = if identity == "quire.op.collection.size" {
            json!({"kind": "type_argument", "declaration": node["semantic_type"]})
        } else {
            Json::Null
        };
        assert_eq!(operation["member"], member, "{name}");
    }
}

/// TC-424 step 3 (FR-001-AC-7): a record's node key depends on its unit's
/// authority and identity, taken from the unit's `RawSourceRef`, and not on
/// the revision or the bytes.
#[trace("TC-424", "FR-001-AC-7")]
#[test]
fn a_declaration_key_follows_the_authority_and_identity_not_the_revision() {
    let point_key = |source: RawSourceRef| {
        let point = NodeKey::from_digest([4; 32]);
        let graph = PackageDeclarations {
            types: TypeEnvironment::new(
                [CompositeDeclaration::new(
                    point,
                    "Point",
                    CompositeShape::Record(vec![FieldDeclaration::new(
                        "x",
                        int(0, 9),
                        Presence::Required,
                    )]),
                )],
                [],
            )
            .unwrap(),
            ..PackageDeclarations::new(source.clone(), qsl_foundation::IdentityLimits::default())
        }
        .check(CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("Point checks: {refusals:?}"));
        assert_eq!(graph.source(), &source);
        let keys: Vec<_> = graph
            .checked_type_nodes()
            .filter_map(|node| match node {
                CheckedTypeNode::Composite { node, .. } => Some(*node),
                _ => None,
            })
            .collect();
        assert_eq!(keys.len(), 1, "one composite");
        keys[0]
    };
    let under = |authority, identity, revision, bytes: &[u8]| {
        point_key(admitted_source(
            SourceIdentity::new(authority, identity, "git", revision),
            bytes,
        ))
    };
    let first = under("a", "u", "1", b"b");
    assert_eq!(first, under("a", "u", "2", b"b'"));
    let other_authority = under("c", "u", "1", b"b");
    let other_identity = under("a", "v", "1", b"b");
    assert_ne!(first, other_authority);
    assert_ne!(first, other_identity);
    assert_ne!(other_authority, other_identity);
}

/// TC-441 (ADR-014 N-3): an unbounded `Sequence<Int[0, 9]>` keys to its
/// `composite_type`/`sequence` node alone, with no `collection_bounds`
/// domain, and is a different node from the widest bounded type
/// `[0, u64::MAX]`.
#[trace("TC-441", "FR-097-AC-7")]
#[test]
fn tc_441_an_unbounded_collection_is_its_composite_node_alone() {
    let (graph, keys) = type_nodes(
        &empty_scope(),
        &SourceOwner::from(&fixture_source()),
        &[
            ValueType::collection(CollectionType::new(
                CollectionKind::Sequence,
                int(0, 9),
                None,
            )),
            // This file's `sequence(_, None)` is the widest bounded type.
            sequence(int(0, 9), None),
        ],
    );
    assert_ne!(keys[0], keys[1]);
    let forms: Vec<_> = graph
        .nodes()
        .map(|node| node.content.semantic_form.to_owned())
        .collect();
    assert!(forms.iter().any(|form| form == "collection_bounds"));
    let unbounded = graph
        .nodes()
        .find(|node| node.key == keys[0])
        .expect("the unbounded type's node");
    assert_eq!(unbounded.content.semantic_form, "sequence");
}

fn integer_form() -> TypeForm {
    TypeForm::builtin(BuiltinType::Integer, SPAN)
}

/// A checked function `name(x: Int[0, 9]): Integer { body }`.
fn integer_function(name: &str, body: Expression) -> FunctionDeclaration {
    function(name, &[("x", int_form(0, 9))], integer_form(), None, body)
}

/// Whether `argument` is an inline integer literal term of `value`.
fn is_inline_integer(argument: &Json, value: &str) -> bool {
    argument["term"] == "literal"
        && argument["value_kind"] == "integer"
        && argument["value"] == value
}

/// Assert no node of `graph` is a `value`/`literal` holding the integer
/// `value`: an inline literal operand has no node of its own.
fn assert_no_literal_node(graph: &SemanticGraph, value: &str) {
    for node in graph.nodes() {
        let json = preimage(node);
        assert!(
            !(json["body"]["term"] == "literal"
                && json["body"]["value_kind"] == "integer"
                && json["body"]["value"] == value),
            "a node holds the literal {value}: {json}"
        );
    }
}

/// FR-358, ADR-013 O-09: the integer literal operand of `x + 1` is a literal
/// term in the application's arguments, and no node holds that literal.
#[trace("FR-093-AC-30", "TC-415")]
#[test]
fn an_integer_literal_right_operand_of_add_is_inline() {
    let body = binary(BinaryOperator::Add, name_expr("x"), integer_expr(1));
    let graph = check(vec![integer_function("f", body)]).expect("f checks");
    let add = application(graph.semantic_graph(), "quire.op.integer.add");
    let arguments = &add["body"]["arguments"];
    assert_eq!(arguments[0]["term"], "reference");
    assert!(is_inline_integer(&arguments[1], "1"), "{arguments}");
    assert_no_literal_node(graph.semantic_graph(), "1");
}

/// The literal on the left, and every operator FR-357 settles.
#[trace("FR-093-AC-30", "TC-415")]
#[test]
fn integer_literals_are_inline_on_either_side_of_each_scalar_operator() {
    for (operator, identity) in [
        (BinaryOperator::Add, "quire.op.integer.add"),
        (BinaryOperator::Subtract, "quire.op.integer.sub"),
        (BinaryOperator::Multiply, "quire.op.integer.mul"),
    ] {
        let left = binary(operator, integer_expr(1), name_expr("x"));
        let graph = check(vec![integer_function("f", left)]).expect("checks");
        let node = application(graph.semantic_graph(), identity);
        let arguments = &node["body"]["arguments"];
        assert!(
            is_inline_integer(&arguments[0], "1"),
            "{identity}: {arguments}"
        );
        assert_eq!(arguments[1]["term"], "reference", "{identity}");
        assert_no_literal_node(graph.semantic_graph(), "1");
    }
    let right = binary(BinaryOperator::Subtract, name_expr("x"), integer_expr(1));
    let graph = check(vec![integer_function("f", right)]).expect("checks");
    assert_no_literal_node(graph.semantic_graph(), "1");
    let right = binary(BinaryOperator::Multiply, name_expr("x"), integer_expr(2));
    let graph = check(vec![integer_function("f", right)]).expect("checks");
    assert_no_literal_node(graph.semantic_graph(), "2");
    let negate = Expression::negate(integer_expr(5));
    let graph = check(vec![integer_function("f", negate)]).expect("checks");
    let node = application(graph.semantic_graph(), "quire.op.integer.negate");
    assert!(is_inline_integer(&node["body"]["arguments"][0], "5"));
    assert_no_literal_node(graph.semantic_graph(), "5");
}

/// Nested arithmetic: each literal is inline in its own application, and the
/// inner application stays a graph child of the outer.
#[trace("FR-093-AC-30", "TC-415")]
#[test]
fn nested_arithmetic_keeps_each_literal_inline_in_its_application() {
    let inner = binary(BinaryOperator::Add, name_expr("x"), integer_expr(1));
    let body = binary(BinaryOperator::Multiply, integer_expr(2), inner);
    let graph = check(vec![integer_function("f", body)]).expect("f checks");
    let semantic = graph.semantic_graph();
    let add = application(semantic, "quire.op.integer.add");
    let mul = application(semantic, "quire.op.integer.mul");
    assert!(is_inline_integer(&add["body"]["arguments"][1], "1"));
    assert!(is_inline_integer(&mul["body"]["arguments"][0], "2"));
    assert_eq!(mul["body"]["arguments"][1]["term"], "reference");
    assert_no_literal_node(semantic, "1");
    assert_no_literal_node(semantic, "2");
}

/// A lowering over an empty scope, run through `body`.
fn with_lowering<T>(body: impl FnOnce(&mut Lowering<'_>) -> T) -> T {
    let scope = empty_scope();
    let owner = SourceOwner::from(&fixture_source());
    let mut occurrences = OccurrenceMap::default();
    let mut meter = quire_exact::Meter::new(crate::check::family::SCALAR_LIMITS_UNLIMITED);
    let mut lowering = Lowering::new(
        &scope,
        &owner,
        &[],
        scope.types().units().clone(),
        None,
        0,
        &mut occurrences,
        &mut meter,
        qsl_foundation::IdentityLimits::default(),
    );
    body(&mut lowering)
}

fn map_of(values: ValueType) -> ValueType {
    ValueType::Map(std::sync::Arc::new(quire_exact::MapType::new(values, None)))
}

fn bounded_metre() -> ValueType {
    ValueType::BoundedQuantity(std::sync::Arc::new(
        quire_exact::BoundedQuantityType::new(metre(), None, None).expect("an unbounded type"),
    ))
}

fn metre() -> quire_exact::UnitId {
    quire_exact::UnitId::compound([5; 32])
}

/// No checked type node carries a map or a bounded quantity yet (quire-exact
/// FR-386 and FR-387): building one refuses `TypeMismatch`, never a guessed
/// node and never a panic. No QSL criterion governs this defensive refusal.
#[test]
fn a_map_and_a_bounded_quantity_have_no_type_node() {
    for value_type in [map_of(ValueType::Boolean), bounded_metre()] {
        let refusal = with_lowering(|lowering| {
            lowering
                .type_node(&value_type, &generated_location())
                .expect_err("no type node")
        });
        assert!(
            matches!(
                refusal.cause,
                CheckCause::IllTyped(quire_exact::IllTypedCause::TypeMismatch)
            ),
            "{:?}",
            refusal.cause
        );
    }
}

/// A map has no equality (QSpec FR-149 states none): `OperatorIneligible`.
/// A bounded quantity is compared as `Quantity<u>` (quire-semantic-value
/// FR-127 item 6), so it lowers to the same equality as the quantity.
#[test]
fn equality_refuses_a_map_and_reads_a_bounded_quantity_as_its_quantity() {
    with_lowering(|lowering| {
        let location = generated_location();
        let refusal = lowering
            .equality(&map_of(ValueType::Boolean), "eq", &location)
            .expect_err("a map has no equality");
        assert!(matches!(
            refusal.cause,
            CheckCause::IllTyped(quire_exact::IllTypedCause::OperatorIneligible)
        ));
        let bounded = lowering
            .equality(&bounded_metre(), "eq", &location)
            .expect("a bounded quantity compares as its quantity");
        let plain = lowering
            .equality(&ValueType::Quantity(metre()), "eq", &location)
            .expect("a quantity compares");
        assert_eq!(format!("{bounded:?}"), format!("{plain:?}"));
    });
}
