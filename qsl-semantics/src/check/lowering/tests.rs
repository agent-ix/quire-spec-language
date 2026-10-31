// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-413, TC-414 and TC-415: the FR-092 keys and the FR-093 lowering,
//! checked structurally: which nodes exist, what each preimage carries and
//! which node each reference names.

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
use crate::check::{CheckedGraph, PackageDeclarations};
use qsl_foundation::source::provenance::RawSourceRef;
use qsl_foundation::SourceIdentity;
use quire_semantic_value::checking::CheckingLimits;
use quire_semantic_value::declaration::{CompositeDeclaration, FieldDeclaration, TypeEnvironment};

mod binder_scope;
mod deep_bodies;
mod depth;
mod dimension_equivalence;
mod expression_depth;
mod leaves;
mod rows;
pub(crate) mod view;
mod wide_integers;

use view::preimage;

/// A structural fixture key for a declaration handle: package `fixture`, one
/// text segment, ordinal 0.
pub(crate) fn fixture_key(label: &str) -> NodeKey {
    NodeKey::new(
        "fixture".to_owned(),
        vec![quire_exact::PathSegment::Text(label.to_owned())],
        0,
    )
    .expect("a fixture key is a valid structural identity")
}

const SPAN: qsl_foundation::Span = qsl_foundation::Span { start: 0, end: 0 };

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
        "fixture",
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
                    "fixture",
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
    let list = fixture_key("list");
    TypeEnvironment::new(
        [CompositeDeclaration::new(
            list.clone(),
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
        ..PackageDeclarations::new(
            fixture_source(),
            "fixture",
            qsl_foundation::IdentityLimits::default(),
        )
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
            ..PackageDeclarations::new(
                fixture_source(),
                "fixture",
                qsl_foundation::IdentityLimits::default(),
            )
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
        ..PackageDeclarations::new(source, "fixture", qsl_foundation::IdentityLimits::default())
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
        ..PackageDeclarations::new(
            fixture_source(),
            "fixture",
            qsl_foundation::IdentityLimits::default(),
        )
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
        "fixture",
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
    quire_exact::UnitId::compound(vec![(fixture_key("metre"), Integer::from(1_i64))])
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

// ---------------------------------------------------------------------
// Structure of the checked graph: nodes, owners and references.
// ---------------------------------------------------------------------

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
        Expression::call(
            "both".to_owned(),
            vec![name_expr("a"), Expression::boolean(true)],
        ),
    )
}

fn h() -> FunctionDeclaration {
    function(
        "h",
        &[("a", boolean())],
        boolean(),
        None,
        Expression::let_in("y".to_owned(), name_expr("a"), name_expr("y")),
    )
}

fn t() -> FunctionDeclaration {
    function(
        "t",
        &[],
        boolean(),
        None,
        Expression::if_then_else(
            Expression::boolean(true),
            Expression::boolean(true),
            Expression::boolean(true),
        ),
    )
}

/// The checked type node of a one-parameter function over `form`.
fn parameter_type(form: TypeForm, aliases: Vec<(String, ValueType)>) -> NodeKey {
    let graph = PackageDeclarations {
        aliases,
        functions: vec![function(
            "g",
            &[("x", form)],
            boolean(),
            None,
            Expression::boolean(true),
        )],
        ..PackageDeclarations::new(
            fixture_source(),
            "fixture",
            qsl_foundation::IdentityLimits::default(),
        )
    }
    .check(CheckingLimits::default())
    .expect("g checks");
    parameter_named(graph.semantic_graph(), "x")
        .semantic_type()
        .expect("a typed parameter")
}

/// TC-413 (FR-092-AC-1): the base, bounded, option and collection types give
/// the type nodes the Type nodes table names: their tags and forms, a bounded
/// form typed at its base, and every key structural.
#[trace("FR-092-AC-1", "TC-413")]
#[test]
fn type_nodes_carry_the_tags_forms_and_semantic_types_of_the_type_table() {
    let text = ValueType::Text(TextType::new(0, 8, quire_exact::TextProfile::Nfc).unwrap());
    let cases = [
        (ValueType::Boolean, "scalar_type", "boolean"),
        (ValueType::Integer, "scalar_type", "integer"),
        (int(0, 9), "bounded_domain", "integer_range"),
        (ValueType::option(int(0, 9)), "composite_type", "option"),
        (
            ValueType::collection(CollectionType::new(
                CollectionKind::Sequence,
                int(0, 9),
                None,
            )),
            "composite_type",
            "sequence",
        ),
        (
            sequence(int(0, 9), Some((1, 3))),
            "bounded_domain",
            "collection_bounds",
        ),
        (text, "bounded_domain", "text_bounds"),
    ];
    let types: Vec<ValueType> = cases
        .iter()
        .map(|(value_type, ..)| value_type.clone())
        .collect();
    let (graph, keys) = type_nodes(
        &empty_scope(),
        &SourceOwner::from(&fixture_source()),
        &types,
    );
    for ((_, tag, form), key) in cases.iter().zip(&keys) {
        let node = preimage(graph.node(key.clone()).expect("the type node"));
        assert_eq!(node["node_tag"], *tag, "{form}");
        assert_eq!(node["semantic_form"], *form);
        assert!(node["declaration"].is_null() && node["owner"].is_null());
    }
    let semantic_type = |index: usize| {
        preimage(graph.node(keys[index].clone()).unwrap())["semantic_type"]["digest"].clone()
    };
    assert_eq!(
        semantic_type(2),
        json!(keys[1].to_string()),
        "Int[0, 9] is typed at Integer"
    );
    assert!(semantic_type(3).is_null(), "an option is its own type");
    assert_eq!(
        semantic_type(5),
        json!(keys[4].to_string()),
        "bounds type at the collection"
    );
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
            Expression::boolean(true),
        )],
        ..PackageDeclarations::new(
            fixture_source(),
            "fixture",
            qsl_foundation::IdentityLimits::default(),
        )
    }
    .check(CheckingLimits::default())
    .expect("g checks");
    let parameter = parameter_named(graph.semantic_graph(), "x");
    assert_eq!(
        parameter.semantic_type(),
        Some(parameter_type(int_form(0, 9), Vec::new()))
    );
    assert!(graph.semantic_graph().nodes().all(|node| node
        .declaration()
        .is_none_or(|name| name.iter().all(|segment| segment.as_str() != "Digit"))));
}

/// TC-413 step 3 (FR-092-AC-2): a declared record's node carries the owner
/// and its declaration, and an anonymous type's node carries neither.
#[trace("FR-092-AC-2", "TC-413")]
#[test]
fn a_declared_record_carries_its_owner_and_an_anonymous_type_does_not() {
    let point = fixture_key("point");
    let scope = scope_with(
        TypeEnvironment::new(
            [CompositeDeclaration::new(
                point.clone(),
                "Point",
                CompositeShape::Record(vec![
                    FieldDeclaration::new("x", int(0, 9), Presence::Required),
                    FieldDeclaration::new("y", int(0, 9), Presence::Required),
                ]),
            )],
            [],
        )
        .unwrap(),
        Vec::new(),
    );
    let w = admitted_source(SourceIdentity::new("a", "w", "git", "1"), b"");
    let types = [ValueType::Composite(point), int(0, 9)];
    let u = SourceOwner::from(&fixture_source());
    let (under_u, u_keys) = type_nodes(&scope, &u, &types);
    let (again, again_keys) = type_nodes(&scope, &u, &types);
    let (under_w, w_keys) = type_nodes(&scope, &SourceOwner::from(&w), &types);

    assert_eq!(u_keys[0], again_keys[0], "one owner, one key");
    assert_eq!(u_keys[1], w_keys[1], "an anonymous type has no owner");
    assert_eq!(
        under_u.node(u_keys[0].clone()),
        again.node(again_keys[0].clone())
    );
    let d1 = preimage(under_u.node(u_keys[0].clone()).unwrap());
    assert!(!d1["owner"].is_null() && !d1["declaration"].is_null());
    let d2 = preimage(under_w.node(w_keys[0].clone()).unwrap());
    assert_eq!(d2["owner"]["identity"], "w");
    let t4 = preimage(under_u.node(u_keys[1].clone()).unwrap());
    assert!(t4["owner"].is_null() && t4["declaration"].is_null());

    let g1 = function(
        "g1",
        &[("x", int_form(0, 9))],
        boolean(),
        None,
        Expression::boolean(true),
    );
    let mut parameter_keys = Vec::new();
    for owner in [fixture_source(), w] {
        let graph = check_under(owner, vec![g1.clone()]).expect("g1 checks");
        parameter_keys.push(parameter_named(graph.semantic_graph(), "x").semantic_type());
    }
    assert_eq!(parameter_keys[0], parameter_keys[1]);
    assert_eq!(parameter_keys[0], Some(u_keys[1].clone()));
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
            .map(|member| {
                member["target"]["digest"]
                    .as_str()
                    .expect("a reference")
                    .to_owned()
            })
            .collect::<Vec<_>>()
    };
    let expected = vec![
        parameter_named(semantic, "a").key().to_string(),
        parameter_named(semantic, "b").key().to_string(),
    ];
    assert_eq!(parameters_of("unused"), expected);
    assert_eq!(parameters_of("both2"), expected);
    assert_eq!(parameters_of("both"), expected);
    let named = |name: &str| {
        semantic
            .nodes()
            .filter(|node| {
                node.semantic_form() == "parameter"
                    && preimage(node)["body"]["members"][0]["value"]["value"] == json!(name)
            })
            .count()
    };
    assert_eq!((named("a"), named("b")), (1, 1), "one node per parameter");
}

/// TC-414 step 3 (FR-092-AC-6, FR-065-AC-8): no function body holds an
/// application; every function is a structural-node key carrying the
/// owner; `a and b` and `both(a, true)` are application-node keys with no
/// owner.
#[trace("FR-092-AC-6", "FR-092-AC-4", "TC-414", "FR-065-AC-8", "TC-163")]
#[test]
fn functions_are_structural_and_applications_carry_no_owner() {
    let graph = check(vec![
        both(),
        function("f", &[], boolean(), None, Expression::boolean(true)),
        h(),
        nb(),
    ])
    .expect("the fixtures check");
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
    let applications: Vec<Json> = semantic
        .nodes()
        .map(preimage)
        .filter(|json| json["version"] == "quire.application-node/v1")
        .collect();
    assert!(
        applications.len() >= 3,
        "`a and b`, `both(a, true)` and the let each build an application node"
    );
    for application in &applications {
        assert!(application["owner"].is_null());
    }
}

/// TC-414 step 4 (FR-092-AC-10): `m`'s `decreases` binding references its
/// parameter node.
#[trace("FR-092-AC-10", "TC-414")]
#[test]
fn a_measure_is_a_decreases_reference() {
    let m = function(
        "m",
        &[("x", int_form(0, 9))],
        boolean(),
        Some(name_expr("x")),
        Expression::boolean(true),
    );
    let graph = check(vec![m]).expect("m checks");
    let semantic = graph.semantic_graph();
    let function_key = graph.function_identity("m").expect("declared");
    let parameter = parameter_named(semantic, "x").key();
    let decreases = &preimage(semantic.node(function_key).unwrap())["body"]["members"][2];
    assert_eq!(decreases["name"], "decreases");
    assert_eq!(
        decreases["value"]["target"]["digest"],
        json!(parameter.to_string())
    );
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
    let boolean_type = literals[0].semantic_type().expect("a typed literal");
    let conditionals: Vec<&SemanticNode> = semantic
        .nodes()
        .filter(|node| node.semantic_form() == "conditional")
        .collect();
    assert_eq!(conditionals.len(), 1);
    let conditional = preimage(conditionals[0]);
    assert_eq!(
        conditional["body"]["operation"]["identity"],
        "quire.op.control.if"
    );
    assert_eq!(
        conditional["semantic_type"]["digest"],
        json!(boolean_type.to_string())
    );
    assert_eq!(
        conditional["body"]["result_type"]["digest"],
        json!(boolean_type.to_string())
    );
    let arguments = conditional["body"]["arguments"].as_array().unwrap();
    assert_eq!(arguments.len(), 3);
    for argument in arguments {
        assert_eq!(argument["target"]["digest"], json!(literal.to_string()));
    }
    let occurrences: Vec<Location> = (0..3)
        .map(|ordinal| {
            graph
                .occurrence(
                    literal.clone(),
                    &quire_exact::Origin::new(quire_exact::Role::new("expression"), ordinal),
                )
                .expect("an expression occurrence")
                .clone()
        })
        .collect();
    let paths: Vec<Vec<usize>> = occurrences
        .into_iter()
        .map(|location| location.path())
        .collect();
    assert_eq!(paths, vec![vec![0], vec![1], vec![2]], "source order");
    assert!(graph
        .occurrence(
            literal,
            &quire_exact::Origin::new(quire_exact::Role::new("expression"), 3)
        )
        .is_none());
}

/// TC-415 step 2 (FR-093-AC-2): `a and b`, `both(a, true)` and `let y = a in
/// y` each build an application node; a local read is a reference, never a
/// node.
#[trace("FR-093-AC-2", "TC-415")]
#[test]
fn applications_build_one_node_each_and_local_reads_build_no_node() {
    let graph = check(vec![both(), nb(), h()]).expect("the fixtures check");
    let semantic = graph.semantic_graph();
    let applications = semantic
        .nodes()
        .filter(|node| preimage(node)["version"] == "quire.application-node/v1")
        .count();
    assert_eq!(applications, 3, "E1, E2 and E3");
    let parameters: Vec<String> = ["a", "b"]
        .iter()
        .map(|name| parameter_named(semantic, name).key().to_string())
        .collect();
    for node in semantic.nodes() {
        let json = preimage(node);
        assert!(
            !(json["body"]["term"] == "reference"
                && parameters.contains(
                    &json["body"]["target"]["digest"]
                        .as_str()
                        .unwrap_or("")
                        .to_owned()
                )),
            "no node's body is a bare local read"
        );
    }
}

// ---------------------------------------------------------------------
// Conversions, record nodes, literals and recursion groups.
// ---------------------------------------------------------------------

/// The key of the one node of `graph` whose `semantic_form` is `form`.
fn only_node_of_form(graph: &SemanticGraph, form: &str) -> NodeKey {
    let found: Vec<NodeKey> = graph
        .nodes()
        .filter(|node| node.semantic_form() == form)
        .map(SemanticNode::key)
        .collect();
    assert_eq!(found.len(), 1, "one `{form}` node");
    found[0].clone()
}

/// TC-415 step 4 (FR-093-AC-4): a contained integer builds no convert node;
/// a narrowing is `numeric.narrow` naming `Int[0, 9]`; a numeric conversion
/// is `numeric.convert`; `flatMap` is one `flat_map` node with no map node.
#[trace("FR-093-AC-4", "TC-415")]
#[test]
fn conversions_are_classified_and_flat_map_builds_one_node() {
    let c1 = function(
        "c1",
        &[("x", int_form(0, 9))],
        int_form(0, 10),
        None,
        name_expr("x"),
    );
    let c2 = function("c2", &[], int_form(0, 9), None, integer_expr(3));
    let rational = || {
        TypeForm::builtin(BuiltinType::Rational, SPAN).with_bounds(vec![
            "0".into(),
            "9".into(),
            "1".into(),
            "1".into(),
        ])
    };
    let c3 = function(
        "c3",
        &[("x", int_form(0, 9))],
        rational(),
        None,
        Expression::convert(rational(), name_expr("x")),
    );
    let bounded_sequence = |element: TypeForm, maximum: &str| {
        TypeForm::collection(CollectionKind::Sequence, SPAN)
            .with_arguments(vec![element])
            .with_bounds(vec!["0".into(), maximum.into()])
    };
    let fm = function(
        "fm",
        &[(
            "s",
            bounded_sequence(bounded_sequence(int_form(0, 9), "2"), "3"),
        )],
        bounded_sequence(int_form(0, 9), "6"),
        None,
        Expression::query(
            BinderQuery::FlatMap,
            "x".to_owned(),
            name_expr("s"),
            name_expr("x"),
        ),
    );

    let graph = check(vec![c1]).expect("c1 checks");
    let semantic = graph.semantic_graph();
    let c1_node = preimage(
        semantic
            .node(graph.function_identity("c1").unwrap())
            .unwrap(),
    );
    let x = parameter_named(semantic, "x").key();
    assert_eq!(
        c1_node["body"]["members"][1]["value"]["target"]["digest"],
        json!(x.to_string())
    );
    assert!(semantic
        .nodes()
        .all(|node| node.semantic_form() != "conversion"));

    let graph = check(vec![c2]).expect("c2 checks");
    let narrow = application(graph.semantic_graph(), "quire.op.numeric.narrow");
    assert_eq!(
        narrow["body"]["operation"]["member"]["kind"],
        "type_argument"
    );
    assert_eq!(
        narrow["body"]["operation"]["member"]["declaration"]["digest"],
        json!(parameter_type(int_form(0, 9), Vec::new()).to_string()),
        "the narrowing names the node of `Int[0, 9]`"
    );
    assert_eq!(narrow["semantic_form"], "conversion");

    let graph = check(vec![c3]).expect("c3 checks");
    application(graph.semantic_graph(), "quire.op.numeric.convert");

    let graph = check(vec![fm]).expect("fm checks");
    application(graph.semantic_graph(), "quire.op.collection.flat_map");
    assert!(graph
        .semantic_graph()
        .nodes()
        .map(preimage)
        .all(|json| json["body"]["operation"]["identity"] != "quire.op.collection.map"));
}

/// FR-093-AC-3: a record projection names its record's node, and a record
/// value is typed by it, with an omitted optional field a typed `none`.
#[trace("FR-093-AC-3", "TC-415")]
#[test]
fn record_projection_and_record_values_name_their_record_node() {
    let point = fixture_key("point");
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
        Expression::field(name_expr("p"), "x".to_owned()),
    );
    let build = function(
        "mk",
        &[("x", int_form(0, 9))],
        TypeForm::name("Point", SPAN),
        None,
        Expression::record(
            "Point".to_owned(),
            vec![("x".to_owned(), Some(name_expr("x")))],
        ),
    );
    let graph = PackageDeclarations {
        types,
        functions: vec![project, build],
        ..PackageDeclarations::new(
            fixture_source(),
            "fixture",
            qsl_foundation::IdentityLimits::default(),
        )
    }
    .check(CheckingLimits::default())
    .expect("the record fixtures check");
    let semantic = graph.semantic_graph();
    let record = only_node_of_form(semantic, "record");
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
    let none_type = y["value"]["type"]["digest"].as_str().expect("a type node");
    let held: Vec<&SemanticNode> = semantic
        .nodes()
        .filter(|node| node.key().to_string() == none_type)
        .collect();
    assert_eq!(held.len(), 1);
    assert_eq!(held[0].semantic_form(), "option");
}

/// FR-092 L6: a literal `1` that is not an operand of an FR-357 operator is
/// a `value`/`literal` node.
#[trace("FR-092-AC-11", "FR-092-AC-4", "TC-413")]
#[test]
fn a_standalone_integer_literal_is_a_node() {
    let one = function("one", &[], integer_form(), None, integer_expr(1));
    let graph = check(vec![one]).expect("one checks");
    let literal = preimage(
        graph
            .semantic_graph()
            .node(only_node_of_form(graph.semantic_graph(), "literal"))
            .unwrap(),
    );
    assert_eq!(literal["body"]["value"], json!("1"));
}

/// A literal operand of any other operator stays a `value`/`literal` node:
/// `x > 0` references its node, and a Boolean literal operand references its
/// node.
#[trace("FR-093-AC-30", "TC-415")]
#[test]
fn literal_operands_of_other_operators_stay_nodes() {
    let compare = function(
        "g",
        &[("x", int_form(0, 9))],
        boolean(),
        None,
        binary(BinaryOperator::Greater, name_expr("x"), integer_expr(0)),
    );
    let graph = check(vec![compare]).expect("g checks");
    let gt = application(graph.semantic_graph(), "quire.op.integer.gt");
    assert_eq!(gt["body"]["arguments"][1]["term"], "reference");
    let literal = only_node_of_form(graph.semantic_graph(), "literal");
    assert_eq!(
        gt["body"]["arguments"][1]["target"]["digest"],
        json!(literal.to_string())
    );
    let conjunction = function(
        "h",
        &[("a", boolean())],
        boolean(),
        None,
        binary(
            BinaryOperator::And,
            name_expr("a"),
            Expression::boolean(true),
        ),
    );
    let graph = check(vec![conjunction]).expect("h checks");
    only_node_of_form(graph.semantic_graph(), "literal");
}

/// FR-358 step 6: an integer literal operand of an integer `==` or `!=`
/// over `Integer` operands, on either side, is inline. Against an
/// `Int[lo, hi]` operand the literal is narrowed to it, so the operand is a
/// `numeric.narrow` application (a graph child) over the literal's node. A
/// Boolean equality keeps its literal node.
#[trace("FR-093-AC-30", "TC-415")]
#[test]
fn integer_literals_are_inline_operands_of_integer_equality() {
    for (operator, identity) in [
        (BinaryOperator::Equal, "quire.op.integer.eq"),
        (BinaryOperator::NotEqual, "quire.op.integer.ne"),
    ] {
        for literal_left in [false, true] {
            let (left, right) = if literal_left {
                (integer_expr(1), name_expr("x"))
            } else {
                (name_expr("x"), integer_expr(1))
            };
            let compare = function(
                "g",
                &[("x", integer_form())],
                boolean(),
                None,
                binary(operator, left, right),
            );
            let graph = check(vec![compare]).expect("g checks");
            let node = application(graph.semantic_graph(), identity);
            let (inline, other) = if literal_left { (0, 1) } else { (1, 0) };
            let arguments = &node["body"]["arguments"];
            assert!(
                is_inline_integer(&arguments[inline], "1"),
                "{identity}: {arguments}"
            );
            assert_eq!(arguments[other]["term"], "reference", "{identity}");
            assert_no_literal_node(graph.semantic_graph(), "1");
        }
        let narrowed = function(
            "n",
            &[("x", int_form(0, 9))],
            boolean(),
            None,
            binary(operator, name_expr("x"), integer_expr(1)),
        );
        let graph = check(vec![narrowed]).expect("n checks");
        let semantic = graph.semantic_graph();
        let node = application(semantic, identity);
        let narrow_key = semantic
            .nodes()
            .find(|candidate| {
                preimage(candidate)["body"]["operation"]["identity"] == "quire.op.numeric.narrow"
            })
            .expect("the literal is narrowed")
            .key()
            .to_string();
        assert_eq!(
            node["body"]["arguments"][1]["target"]["digest"], narrow_key,
            "{identity} compares `x` with the narrow node"
        );
        let narrow = application(semantic, "quire.op.numeric.narrow");
        let literal = only_node_of_form(semantic, "literal");
        assert_eq!(
            narrow["body"]["arguments"][0]["target"]["digest"],
            json!(literal.to_string()),
            "the narrow node applies to the literal node"
        );
    }
    let boolean_equality = function(
        "h",
        &[("a", boolean())],
        boolean(),
        None,
        binary(
            BinaryOperator::Equal,
            name_expr("a"),
            Expression::boolean(true),
        ),
    );
    let graph = check(vec![boolean_equality]).expect("h checks");
    let node = application(graph.semantic_graph(), "quire.op.boolean.eq");
    assert_eq!(node["body"]["arguments"][1]["term"], "reference");
    only_node_of_form(graph.semantic_graph(), "literal");
}

/// Assert every node of `graph` in a recursion group carries its place in
/// its content: `{ordinal, size}`.
fn assert_group_members_carry_their_recursion(graph: &SemanticGraph) {
    for node in graph.nodes() {
        let preimage = preimage(node);
        let Some(recursion) = node.recursion() else {
            assert!(preimage["recursion"].is_null());
            continue;
        };
        let expected = json!({"ordinal": recursion.ordinal(), "size": recursion.size()});
        assert_eq!(preimage["recursion"], expected, "{}", node.key());
    }
}

/// TC-413 step 9 (FR-092-AC-11): the self-recursive `f` is a group of one
/// recursive function, and every member carries its place in the group.
#[trace("FR-092-AC-11", "TC-413")]
#[test]
fn a_self_recursive_function_is_a_group_of_one() {
    let graph = check(vec![recursive("f", "f")]).expect("f checks");
    let graph = graph.semantic_graph();
    assert_group_members_carry_their_recursion(graph);
    let members: Vec<&SemanticNode> = graph
        .nodes()
        .filter(|node| node.recursion().is_some())
        .collect();
    assert!(!members.is_empty());
    assert!(members
        .iter()
        .any(|node| node.semantic_form() == "recursive_function"));
}

/// TC-413 step 9 (FR-092-AC-11): `ping` and `pong` key alike, whichever is
/// declared first.
#[trace("FR-092-AC-11", "FR-383-AC-25", "TC-413")]
#[test]
fn a_mutually_recursive_pair_keys_alike_in_either_order() {
    let keys = |functions: Vec<FunctionDeclaration>| {
        let checked = check(functions).expect("ping and pong check");
        let graph = checked.semantic_graph();
        assert_group_members_carry_their_recursion(graph);
        let members = graph
            .nodes()
            .filter(|node| node.recursion().is_some())
            .count();
        assert!(members >= 2, "both functions are group members");
        graph.nodes().map(SemanticNode::key).collect::<Vec<_>>()
    };
    let ping_first = keys(vec![recursive("ping", "pong"), recursive("pong", "ping")]);
    let pong_first = keys(vec![recursive("pong", "ping"), recursive("ping", "pong")]);
    assert_eq!(ping_first, pong_first);
}
