// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-415 step 10 (FR-093-AC-14, QSL-228): a function body nested to the
//! check stage's depth limit checks, and one nested past it refuses on the
//! named depth limit; neither overflows the stack. Each check runs on a
//! spawned thread with a 2 MiB stack, the default test-thread size, in
//! whatever profile the suite runs in. Before QSL-228 a debug build aborted
//! at 20 nested `a and (…)`: typing, the definedness walk and lowering each
//! recursed once per level, and measuring the declaration recursed once per
//! level before any limit was checked.

use qsl_forms::{Accumulation, FieldInitializer};

use super::*;
use crate::check::{CheckMode, Obligation, WrongSnapshotCause, MAX_CHECKING_DEPTH};

/// The stack each check runs on.
const STACK: usize = 2 * 1024 * 1024;

/// How many levels the refused bodies nest.
const TOO_DEEP: usize = 1_000;

fn handle(label: &str) -> NodeKey {
    NodeKey::from_digest(qsl_foundation::ByteDigest::of(label.as_bytes()).as_bytes())
}

/// `record N { v: Boolean; next?: N; }` and `tuple T(Boolean)`.
fn types() -> TypeEnvironment {
    TypeEnvironment::new(
        vec![
            CompositeDeclaration::new(
                handle("N"),
                "N",
                CompositeShape::Record(vec![
                    FieldDeclaration::new("v", ValueType::Boolean, Presence::Required),
                    FieldDeclaration::new(
                        "next",
                        ValueType::Composite(handle("N")),
                        Presence::Optional,
                    ),
                ]),
            ),
            CompositeDeclaration::new(
                handle("T"),
                "T",
                CompositeShape::Tuple(vec![ValueType::Boolean]),
            ),
        ],
        [],
    )
    .expect("FR-143 admits N and T")
}

/// `Sequence<element>[0, 5]`.
fn sequence_of(element: TypeForm) -> TypeForm {
    TypeForm::collection(CollectionKind::Sequence, SPAN)
        .with_arguments(vec![element])
        .with_bounds(vec!["0".into(), "5".into()])
}

fn boxed(expression: Expression) -> Box<Expression> {
    Box::new(expression)
}

/// A nested expression form: `f`'s parameters, result and body when the
/// form is nested `levels` times.
#[derive(Clone, Copy, Debug)]
enum Form {
    /// `a and (a and (… a))`, the ticket's reproduction.
    And,
    /// `((a and a) and a) …`.
    AndLeft,
    /// `if a then (…) else false`.
    IfThen,
    /// `if (…) then a else a`.
    IfCondition,
    /// `let b0 = a in let b1 = a in … a`.
    Let,
    /// `not not … a`.
    Not,
    /// `g(g(… a))`: each argument is typed against its parameter, the
    /// typer's upcast step.
    Call,
    /// `(x < x) = ((x < x) = (… a))`.
    Comparison,
    /// `if ((a and x < 1) and x < 1) … then a else a`: a guard whose facts
    /// the definedness walk derives through every level.
    Guard,
    /// `x + (x + (… x))`.
    Add,
    /// `((x + x) + x) …`.
    AddLeft,
    /// `-(-(… x))`.
    Negate,
    /// `h(x * (x * (… x)))`: a narrowing whose range the definedness walk
    /// proves through every level.
    Narrowed,
    /// `N { v: a, next: N { v: a, next: … } }`.
    Record,
    /// `value(value(n.next).next) … .v`, unguarded.
    Field,
    /// `forall(v0 in s: forall(v1 in s: … true))`.
    Forall,
    /// `sum<Total>(v0 in s: sum<Total>(v1 in s: … x))`.
    Sum,
    /// `count<Total>(v0 in s: count<Total>(v1 in s: … x = 0) = 0)`.
    Count,
    /// `fold<Total>(acc0, v0 in s: fold<Total>(…), identity: 0)`.
    Fold,
    /// `map(v0 in map(v1 in … s: v1): v0)`.
    Map,
    /// `filter(v0 in filter(v1 in … s: true): true)`.
    Filter,
    /// `contains(bs, contains(bs, … a))`.
    Contains,
    /// `convert<Int[0, 9]>(convert<Int[0, 9]>(… x))`.
    Convert,
    /// `gs(sequence[gs(sequence[… a])])`: a collection literal, typed
    /// against `gs`'s parameter.
    Collection,
    /// `tv(T(tv(T(… a))))`: a tuple literal, typed against `tv`'s
    /// parameter.
    Tuple,
}

const FORMS: [Form; 25] = [
    Form::And,
    Form::AndLeft,
    Form::IfThen,
    Form::IfCondition,
    Form::Let,
    Form::Not,
    Form::Call,
    Form::Comparison,
    Form::Guard,
    Form::Add,
    Form::AddLeft,
    Form::Negate,
    Form::Narrowed,
    Form::Record,
    Form::Field,
    Form::Forall,
    Form::Sum,
    Form::Count,
    Form::Fold,
    Form::Map,
    Form::Filter,
    Form::Contains,
    Form::Convert,
    Form::Collection,
    Form::Tuple,
];

impl Form {
    /// The deepest nesting the default limits admit: the body's deepest
    /// expression is then at the depth limit. A form that nests two
    /// expressions per level (`value(….next)`, `count`'s `… = 0`, a literal
    /// inside a call) admits 63.
    fn deepest(self) -> usize {
        match self {
            Self::Field | Self::Count | Self::Collection | Self::Tuple => 63,
            Self::Guard => 125,
            Self::Comparison | Self::Narrowed | Self::Record => 126,
            Self::And
            | Self::AndLeft
            | Self::IfThen
            | Self::IfCondition
            | Self::Let
            | Self::Not
            | Self::Call
            | Self::Add
            | Self::AddLeft
            | Self::Negate
            | Self::Forall
            | Self::Sum
            | Self::Fold
            | Self::Map
            | Self::Filter
            | Self::Contains
            | Self::Convert => 127,
        }
    }

    /// `f`'s declaration with this form nested `levels` times.
    fn function(self, levels: usize) -> FunctionDeclaration {
        let a = || name_expr("a");
        let x = || name_expr("x");
        let x_below_one = || binary(BinaryOperator::Less, x(), integer_expr(1));
        let integer = || TypeForm::builtin(BuiltinType::Integer, SPAN);
        let sequence = sequence_of(int_form(0, 9));
        let record = |value: Expression, next: Option<Expression>| Expression::Record {
            name: "N".to_owned(),
            fields: std::iter::once(("v".to_owned(), FieldInitializer::Value(value)))
                .chain(next.map(|next| ("next".to_owned(), FieldInitializer::Value(next))))
                .collect(),
        };
        let binder_query = |query: BinderQuery, binder: String, source, body| Expression::Query {
            query,
            binder,
            source: boxed(source),
            body: boxed(body),
        };
        let (parameters, result, leaf) = match self {
            Self::Add | Self::AddLeft | Self::Negate | Self::Narrowed => {
                (vec![("x", int_form(0, 1))], integer(), x())
            }
            Self::Sum | Self::Count | Self::Fold => (
                vec![("x", int_form(0, 1)), ("s", sequence.clone())],
                integer(),
                x(),
            ),
            Self::Convert => (vec![("x", int_form(0, 1))], int_form(0, 9), x()),
            Self::Map | Self::Filter => (
                vec![("s", sequence.clone())],
                sequence.clone(),
                name_expr("s"),
            ),
            Self::Contains => (
                vec![("a", boolean()), ("bs", sequence_of(boolean()))],
                boolean(),
                a(),
            ),
            Self::Collection | Self::Tuple => (vec![("a", boolean())], boolean(), a()),
            Self::Record => (
                vec![("a", boolean())],
                TypeForm::name("N", SPAN),
                record(a(), None),
            ),
            Self::Field => (
                vec![("n", TypeForm::name("N", SPAN))],
                boolean(),
                name_expr("n"),
            ),
            Self::Forall => (vec![("s", sequence)], boolean(), Expression::Boolean(true)),
            Self::And
            | Self::AndLeft
            | Self::IfThen
            | Self::IfCondition
            | Self::Let
            | Self::Not
            | Self::Call
            | Self::Comparison
            | Self::Guard => (
                vec![("a", boolean()), ("x", int_form(0, 1))],
                boolean(),
                a(),
            ),
        };
        let mut body = leaf;
        for level in 0..levels {
            body = match self {
                Self::And => binary(BinaryOperator::And, a(), body),
                Self::AndLeft => binary(BinaryOperator::And, body, a()),
                Self::IfThen => Expression::If {
                    condition: boxed(a()),
                    then: boxed(body),
                    otherwise: boxed(Expression::Boolean(false)),
                },
                Self::IfCondition => Expression::If {
                    condition: boxed(body),
                    then: boxed(a()),
                    otherwise: boxed(a()),
                },
                Self::Let => Expression::Let {
                    name: format!("b{level}"),
                    value: boxed(a()),
                    body: boxed(body),
                },
                Self::Not => Expression::Not(boxed(body)),
                Self::Call => Expression::Call {
                    name: "g".to_owned(),
                    arguments: vec![body],
                },
                Self::Comparison => binary(
                    BinaryOperator::Equal,
                    binary(BinaryOperator::Less, x(), x()),
                    body,
                ),
                Self::Guard => binary(BinaryOperator::And, body, x_below_one()),
                Self::Add => binary(BinaryOperator::Add, x(), body),
                Self::AddLeft => binary(BinaryOperator::Add, body, x()),
                Self::Negate => Expression::Negate(boxed(body)),
                Self::Narrowed => binary(BinaryOperator::Multiply, x(), body),
                Self::Record => record(a(), Some(body)),
                Self::Field => Expression::Value(boxed(Expression::Field {
                    operand: boxed(body),
                    field: "next".to_owned(),
                })),
                Self::Forall => Expression::Query {
                    query: BinderQuery::Forall,
                    binder: format!("v{level}"),
                    source: boxed(name_expr("s")),
                    body: boxed(body),
                },
                Self::Sum => Expression::Sum {
                    result_type: "Total".to_owned(),
                    binder: format!("v{level}"),
                    source: boxed(name_expr("s")),
                    summand: boxed(body),
                },
                Self::Count => Expression::Count {
                    result_type: "Total".to_owned(),
                    binder: format!("v{level}"),
                    source: boxed(name_expr("s")),
                    predicate: boxed(binary(BinaryOperator::Equal, body, integer_expr(0))),
                },
                Self::Fold => Expression::Accumulate {
                    form: Accumulation::Fold,
                    accumulator_type: "Total".to_owned(),
                    accumulator: format!("acc{level}"),
                    binder: format!("v{level}"),
                    source: boxed(name_expr("s")),
                    step: boxed(body),
                    identity: Some(boxed(integer_expr(0))),
                },
                Self::Map => {
                    let binder = format!("v{level}");
                    binder_query(BinderQuery::Map, binder.clone(), body, name_expr(&binder))
                }
                Self::Filter => binder_query(
                    BinderQuery::Filter,
                    format!("v{level}"),
                    body,
                    Expression::Boolean(true),
                ),
                Self::Contains => Expression::Contains {
                    collection: boxed(name_expr("bs")),
                    item: boxed(body),
                },
                Self::Convert => Expression::Convert {
                    target: int_form(0, 9),
                    operand: boxed(body),
                },
                Self::Collection => Expression::Call {
                    name: "gs".to_owned(),
                    arguments: vec![Expression::Collection {
                        kind: CollectionKind::Sequence,
                        elements: vec![body],
                    }],
                },
                Self::Tuple => Expression::Call {
                    name: "tv".to_owned(),
                    arguments: vec![Expression::Call {
                        name: "T".to_owned(),
                        arguments: vec![body],
                    }],
                },
            };
        }
        let body = match self {
            Self::Guard => Expression::If {
                condition: boxed(body),
                then: boxed(a()),
                otherwise: boxed(a()),
            },
            Self::Narrowed => Expression::Call {
                name: "h".to_owned(),
                arguments: vec![body],
            },
            Self::Field => Expression::Field {
                operand: boxed(body),
                field: "v".to_owned(),
            },
            Self::And
            | Self::AndLeft
            | Self::IfThen
            | Self::IfCondition
            | Self::Let
            | Self::Not
            | Self::Call
            | Self::Comparison
            | Self::Add
            | Self::AddLeft
            | Self::Negate
            | Self::Record
            | Self::Forall
            | Self::Sum
            | Self::Count
            | Self::Fold
            | Self::Map
            | Self::Filter
            | Self::Contains
            | Self::Convert
            | Self::Collection
            | Self::Tuple => body,
        };
        function("f", &parameters, result, None, body)
    }
}

/// Check `form` nested `levels` times, beside `g(c: Boolean): Boolean`,
/// `h(n: Int[0, 9]): Int[0, 9]`, `gs(c: Sequence<Boolean>[0, 5]): Boolean`
/// and `tv(t: T): Boolean`, with the alias `Total = Integer`, under `limits`
/// on a [`STACK`]-byte thread. The body is built and dropped on that thread
/// too.
fn check_on_small_stack(
    form: Form,
    levels: usize,
    limits: CheckingLimits,
) -> Result<CheckedGraph, Vec<CheckRefusal>> {
    std::thread::Builder::new()
        .stack_size(STACK)
        .spawn(move || {
            PackageDeclarations {
                functions: vec![
                    form.function(levels),
                    function("g", &[("c", boolean())], boolean(), None, name_expr("c")),
                    function(
                        "h",
                        &[("n", int_form(0, 9))],
                        int_form(0, 9),
                        None,
                        name_expr("n"),
                    ),
                    function(
                        "gs",
                        &[("c", sequence_of(boolean()))],
                        boolean(),
                        None,
                        Expression::Boolean(true),
                    ),
                    function(
                        "tv",
                        &[("t", TypeForm::name("T", SPAN))],
                        boolean(),
                        None,
                        Expression::Boolean(true),
                    ),
                ],
                types: types(),
                aliases: vec![("Total".to_owned(), ValueType::Integer)],
                ..PackageDeclarations::new(fixture_source())
            }
            .check(limits)
        })
        .expect("the check thread spawns")
        .join()
        .expect("the check completes on a 2 MiB stack")
}

/// Every refusal is the typing stage's depth limit `limit`, and there is at
/// least one.
fn assert_depth_refusals(form: Form, refusals: &[CheckRefusal], limit: u64) {
    assert!(!refusals.is_empty(), "{form:?}");
    for refusal in refusals {
        assert_eq!(
            refusal.cause,
            CheckCause::ResourceExhausted {
                stage: CheckingStage::Typing,
                kind: CheckingLimitKind::Depth,
                limit,
            },
            "{form:?}: {refusal:?}"
        );
    }
}

/// The ticket's reproduction: 127 nested `a and (…)` put the innermost `a`
/// at the default depth limit and check; 128 refuse naming it.
#[trace("FR-093-AC-14", "TC-415")]
#[test]
fn a_127_level_and_chain_checks_on_a_small_stack_and_128_refuses() {
    let graph = check_on_small_stack(Form::And, 127, CheckingLimits::default())
        .unwrap_or_else(|refusals| panic!("127 levels check: {refusals:?}"));
    let connectives = graph
        .semantic_graph()
        .nodes()
        .filter(|node| node.semantic_form() == "binary")
        .count();
    // `a and (…)` at every level is one node: each level's operand
    // differs, so no two are one content.
    assert_eq!(connectives, 127);
    let refusals = check_on_small_stack(Form::And, 128, CheckingLimits::default())
        .expect_err("128 levels pass the depth limit");
    assert_depth_refusals(Form::And, &refusals, MAX_CHECKING_DEPTH);
}

/// Every nested form checks at the deepest nesting the default limits
/// admit, through typing, the definedness walk and lowering, and one level
/// deeper refuses on the depth limit. The unguarded field chain reaches the
/// definedness walk and refuses there on its first unproved `value`.
#[trace("FR-093-AC-14", "TC-415")]
#[test]
fn every_nested_form_checks_at_the_depth_limit_on_a_small_stack() {
    for form in FORMS {
        let deepest = form.deepest();
        match (
            form,
            check_on_small_stack(form, deepest, CheckingLimits::default()),
        ) {
            (Form::Field, Err(refusals)) => assert!(
                refusals
                    .iter()
                    .all(|refusal| refusal.cause == CheckCause::Unproved(Obligation::Presence)),
                "{form:?}: {refusals:?}"
            ),
            (Form::Field, Ok(_)) => panic!("an unguarded `value` is unproved"),
            (_, Ok(_)) => {}
            (_, Err(refusals)) => panic!("{form:?} at {deepest} levels checks: {refusals:?}"),
        }
        let refusals = check_on_small_stack(form, deepest + 1, CheckingLimits::default())
            .expect_err("one level more passes the depth limit");
        assert_depth_refusals(form, &refusals, MAX_CHECKING_DEPTH);
    }
}

/// 1,000 levels of every form refuse naming the depth limit, at the
/// default limits, at the maximum depth with nodes and work unlimited, and
/// at a caller depth of 16.
#[trace("FR-093-AC-14", "TC-415")]
#[test]
fn a_1000_level_nesting_of_every_form_refuses_on_the_depth_limit() {
    let maximum = CheckingLimits::new(u64::MAX, MAX_CHECKING_DEPTH)
        .expect("the maximum depth is admitted")
        .with_work_budget(u64::MAX)
        .with_input_bytes(u64::MAX);
    let narrowed = CheckingLimits::new(u64::MAX, 16).expect("16 is admitted");
    for form in FORMS {
        for (limits, limit) in [
            (CheckingLimits::default(), MAX_CHECKING_DEPTH),
            (maximum, MAX_CHECKING_DEPTH),
            (narrowed, 16),
        ] {
            let refusals = check_on_small_stack(form, TOO_DEEP, limits)
                .expect_err("1,000 levels pass the depth limit");
            assert_depth_refusals(form, &refusals, limit);
        }
    }
}

/// `pre(…)`'s syntactic checks walk the whole operand before typing enters
/// it: a postcondition `pre` over 1,000 nested levels with no eligible read
/// refuses as a forbidden pre-read, and one whose eligible read sits under
/// 1,000 `let`s refuses on the depth limit once typing reaches it.
#[trace("FR-093-AC-14", "TC-415")]
#[test]
fn a_postcondition_pre_over_1000_levels_refuses_on_a_small_stack() {
    let graph = PackageDeclarations::new(fixture_source())
        .check(CheckingLimits::default())
        .expect("an empty package checks");
    let refusals = std::thread::scope(|scope| {
        std::thread::Builder::new()
            .stack_size(STACK)
            .spawn_scoped(scope, || {
                let parameters = vec![("a".to_owned(), ValueType::Boolean)];
                let mut unread = name_expr("a");
                let mut bound = Expression::Pre(boxed(name_expr("a")));
                for level in 0..TOO_DEEP {
                    unread = binary(BinaryOperator::And, name_expr("a"), unread);
                    bound = Expression::Let {
                        name: format!("b{level}"),
                        value: boxed(name_expr("a")),
                        body: boxed(bound),
                    };
                }
                [unread, bound].map(|operand| {
                    graph.check_postcondition_expression(
                        parameters.clone(),
                        &Expression::Pre(boxed(operand)),
                        Some(&ValueType::Boolean),
                        CheckMode::Linked,
                        CheckingLimits::default(),
                    )
                })
            })
            .expect("the check thread spawns")
            .join()
            .expect("the check completes on a 2 MiB stack")
    });
    let [unread, bound] = refusals;
    assert_eq!(
        unread.expect_err("no eligible read").cause,
        CheckCause::WrongSnapshot(WrongSnapshotCause::ForbiddenPreRead)
    );
    assert_eq!(
        bound.expect_err("1,000 `let`s pass the depth limit").cause,
        CheckCause::ResourceExhausted {
            stage: CheckingStage::Typing,
            kind: CheckingLimitKind::Depth,
            limit: MAX_CHECKING_DEPTH,
        }
    );
}
