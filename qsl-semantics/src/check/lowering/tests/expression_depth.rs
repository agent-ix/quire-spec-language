// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-415 step 10 (FR-093-AC-14): a function body of any depth checks, or
//! refuses on its own unproved obligation, the same way at 1,000 levels as
//! at 2; no limit names a depth and nothing overflows the stack. Each check
//! runs on a spawned thread with a 512 KiB stack, in whatever profile the
//! suite runs in. A debug build once aborted at 20 nested `a and (…)`:
//! typing, the definedness walk and lowering each recursed once per level,
//! and measuring the declaration recursed once per level before any limit
//! was checked.

use qsl_forms::Accumulation;

use super::*;
use crate::check::{Obligation, WrongSnapshotCause};
use quire_semantic_value::checking::CheckMode;

/// The stack each check runs on.
const STACK: usize = 512 * 1024;

/// How many levels the deep bodies nest.
const DEEP: usize = 1_000;

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

/// A nested expression form: `f`'s parameters, result and body when the
/// form is nested `levels` times.
#[derive(Clone, Copy, Debug)]
pub(super) enum Form {
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
    /// `let b0 = x + 0 in if b0 > 0 then (let b1 = x + 1 in if b1 > 0 then
    /// (… a) else a) else a`: nested guards on distinct subjects, each
    /// level's facts one more than its parent's.
    DistinctGuards,
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

pub(super) const FORMS: [Form; 26] = [
    Form::And,
    Form::AndLeft,
    Form::IfThen,
    Form::IfCondition,
    Form::Let,
    Form::Not,
    Form::Call,
    Form::Comparison,
    Form::Guard,
    Form::DistinctGuards,
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
    /// `f`'s declaration with this form nested `levels` times.
    fn function(self, levels: usize) -> FunctionDeclaration {
        let a = || name_expr("a");
        let x = || name_expr("x");
        let x_below_one = || binary(BinaryOperator::Less, x(), integer_expr(1));
        let integer = || TypeForm::builtin(BuiltinType::Integer, SPAN);
        let sequence = sequence_of(int_form(0, 9));
        let record = |value: Expression, next: Option<Expression>| {
            Expression::record(
                "N".to_owned(),
                std::iter::once(("v".to_owned(), Some(value)))
                    .chain(next.map(|next| ("next".to_owned(), Some(next))))
                    .collect(),
            )
        };
        let binder_query = |query: BinderQuery, binder: String, source, body| {
            Expression::query(query, binder, source, body)
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
            Self::Forall => (vec![("s", sequence)], boolean(), Expression::boolean(true)),
            Self::And
            | Self::AndLeft
            | Self::IfThen
            | Self::IfCondition
            | Self::Let
            | Self::Not
            | Self::Call
            | Self::Comparison
            | Self::Guard
            | Self::DistinctGuards => (
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
                Self::IfThen => Expression::if_then_else(a(), body, Expression::boolean(false)),
                Self::IfCondition => Expression::if_then_else(body, a(), a()),
                Self::Let => Expression::let_in(format!("b{level}"), a(), body),
                Self::Not => Expression::logical_not(body),
                Self::Call => Expression::call("g".to_owned(), vec![body]),
                Self::Comparison => binary(
                    BinaryOperator::Equal,
                    binary(BinaryOperator::Less, x(), x()),
                    body,
                ),
                Self::Guard => binary(BinaryOperator::And, body, x_below_one()),
                Self::DistinctGuards => {
                    let slot = format!("b{level}");
                    Expression::let_in(
                        slot.clone(),
                        binary(
                            BinaryOperator::Add,
                            x(),
                            integer_expr(i64::try_from(level).unwrap_or(0)),
                        ),
                        Expression::if_then_else(
                            binary(BinaryOperator::Greater, name_expr(&slot), integer_expr(0)),
                            body,
                            a(),
                        ),
                    )
                }
                Self::Add => binary(BinaryOperator::Add, x(), body),
                Self::AddLeft => binary(BinaryOperator::Add, body, x()),
                Self::Negate => Expression::negate(body),
                Self::Narrowed => binary(BinaryOperator::Multiply, x(), body),
                Self::Record => record(a(), Some(body)),
                Self::Field => Expression::value(Expression::field(body, "next".to_owned())),
                Self::Forall => Expression::query(
                    BinderQuery::Forall,
                    format!("v{level}"),
                    name_expr("s"),
                    body,
                ),
                Self::Sum => Expression::sum(
                    qsl_forms::DeclaredName {
                        name: "Total".to_owned(),
                        span: qsl_foundation::Span { start: 0, end: 0 },
                    },
                    format!("v{level}"),
                    name_expr("s"),
                    body,
                ),
                Self::Count => Expression::count(
                    qsl_forms::DeclaredName {
                        name: "Total".to_owned(),
                        span: qsl_foundation::Span { start: 0, end: 0 },
                    },
                    format!("v{level}"),
                    name_expr("s"),
                    binary(BinaryOperator::Equal, body, integer_expr(0)),
                ),
                Self::Fold => Expression::accumulate(
                    Accumulation::Fold,
                    qsl_forms::DeclaredName {
                        name: "Total".to_owned(),
                        span: qsl_foundation::Span { start: 0, end: 0 },
                    },
                    format!("acc{level}"),
                    format!("v{level}"),
                    name_expr("s"),
                    body,
                    Some(integer_expr(0)),
                ),
                Self::Map => {
                    let binder = format!("v{level}");
                    binder_query(BinderQuery::Map, binder.clone(), body, name_expr(&binder))
                }
                Self::Filter => binder_query(
                    BinderQuery::Filter,
                    format!("v{level}"),
                    body,
                    Expression::boolean(true),
                ),
                Self::Contains => Expression::contains(name_expr("bs"), body),
                Self::Convert => Expression::convert(int_form(0, 9), body),
                Self::Collection => Expression::call(
                    "gs".to_owned(),
                    vec![Expression::collection(CollectionKind::Sequence, vec![body])],
                ),
                Self::Tuple => Expression::call(
                    "tv".to_owned(),
                    vec![Expression::call("T".to_owned(), vec![body])],
                ),
            };
        }
        let body = match self {
            Self::Guard => Expression::if_then_else(body, a(), a()),
            Self::Narrowed => Expression::call("h".to_owned(), vec![body]),
            Self::Field => Expression::field(body, "v".to_owned()),
            Self::And
            | Self::AndLeft
            | Self::IfThen
            | Self::IfCondition
            | Self::Let
            | Self::Not
            | Self::Call
            | Self::Comparison
            | Self::DistinctGuards
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
pub(super) fn check_on_small_stack(
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
                        Expression::boolean(true),
                    ),
                    function(
                        "tv",
                        &[("t", TypeForm::name("T", SPAN))],
                        boolean(),
                        None,
                        Expression::boolean(true),
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
        .expect("the check completes on a 512 KiB stack")
}

/// What a check gives: checked, or the distinct causes it refused on.
fn outcome(checked: Result<CheckedGraph, Vec<CheckRefusal>>) -> Result<(), Vec<CheckCause>> {
    checked.map(|_| ()).map_err(|refusals| {
        let mut causes: Vec<CheckCause> = Vec::new();
        for refusal in refusals {
            if !causes.contains(&refusal.cause) {
                causes.push(refusal.cause);
            }
        }
        causes
    })
}

/// Every nested form, nested 1,000 deep, checks or refuses on its own
/// unproved obligation exactly as it does nested 2 deep, through typing,
/// the definedness walk and lowering: the unguarded field chain refuses on
/// its unproved `value` both times, and every other form checks.
#[trace("FR-093-AC-14", "TC-415")]
#[test]
fn every_nested_form_checks_at_1000_levels_as_at_2_on_a_small_stack() {
    for form in FORMS {
        let shallow = outcome(check_on_small_stack(form, 2, CheckingLimits::default()));
        match (form, &shallow) {
            (Form::Field, Err(causes)) => {
                assert_eq!(causes, &[CheckCause::Unproved(Obligation::Presence)]);
            }
            (Form::Field, Ok(())) => panic!("an unguarded `value` is unproved"),
            (_, Ok(())) => {}
            (_, Err(causes)) => panic!("{form:?} at 2 levels checks: {causes:?}"),
        }
        let deep = outcome(check_on_small_stack(form, DEEP, CheckingLimits::default()));
        assert_eq!(deep, shallow, "{form:?} at {DEEP} levels");
    }
}

/// `pre(…)`'s syntactic checks walk the whole operand before typing enters
/// it: a postcondition `pre` over 1,000 nested `a and (…)` with no eligible
/// read refuses as a forbidden pre-read.
#[trace("FR-093-AC-14", "TC-415")]
#[test]
fn a_postcondition_pre_over_1000_levels_refuses_on_a_small_stack() {
    let graph = PackageDeclarations::new(fixture_source())
        .check(CheckingLimits::default())
        .expect("an empty package checks");
    let refused = std::thread::scope(|scope| {
        std::thread::Builder::new()
            .stack_size(STACK)
            .spawn_scoped(scope, || {
                let parameters = vec![("a".to_owned(), ValueType::Boolean)];
                let mut unread = name_expr("a");
                for _ in 0..DEEP {
                    unread = binary(BinaryOperator::And, name_expr("a"), unread);
                }
                graph.check_postcondition_expression(
                    parameters,
                    &Expression::pre(unread),
                    Some(&ValueType::Boolean),
                    CheckMode::Linked,
                    CheckingLimits::default(),
                )
            })
            .expect("the check thread spawns")
            .join()
            .expect("the check completes on a 512 KiB stack")
    });
    assert_eq!(
        refused.expect_err("no eligible read").cause,
        CheckCause::WrongSnapshot(WrongSnapshotCause::ForbiddenPreRead)
    );
}
