// SPDX-License-Identifier: AGPL-3.0-or-later
//! The nested expression forms of TC-415 step 10 (FR-093-AC-14), as
//! declarations a test builds at any depth: `f` with a form nested `levels`
//! times, beside the functions, types and alias it calls. The semantic check
//! tests and the package emitter tests (FR-264-AC-2) both build their deep
//! packages from here, so there is one list of forms.

use qsl_forms::{
    Accumulation, BinaryOperator, BinderQuery, BuiltinType, Expression, FunctionDeclaration,
    TypeForm,
};
use quire_exact::{CollectionKind, Integer, NodeKey, Presence, ValueType};
use quire_semantic_value::declaration::{
    CompositeDeclaration, CompositeShape, FieldDeclaration, TypeEnvironment,
};

use super::family::fixtures::fixture_source;
use super::PackageDeclarations;

const SPAN: qsl_foundation::Span = qsl_foundation::Span { start: 0, end: 0 };

/// The `Boolean` type form.
pub fn boolean() -> TypeForm {
    TypeForm::builtin(BuiltinType::Boolean, SPAN)
}

/// The `Int[lower, upper]` type form.
pub fn int_form(lower: i64, upper: i64) -> TypeForm {
    TypeForm::builtin(BuiltinType::Int, SPAN)
        .with_bounds(vec![lower.to_string(), upper.to_string()])
}

/// A name expression.
pub fn name_expr(name: &str) -> Expression {
    Expression::name(name.to_owned())
}

/// An integer literal expression.
pub fn integer_expr(value: i64) -> Expression {
    Expression::integer(Integer::from(value))
}

/// A binary expression.
pub fn binary(operator: BinaryOperator, left: Expression, right: Expression) -> Expression {
    Expression::binary(operator, left, right)
}

/// A function declaration with `body`, by parameter name and type form.
pub fn function(
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

fn handle(label: &str) -> NodeKey {
    NodeKey::from_digest(qsl_foundation::ByteDigest::of(label.as_bytes()).as_bytes())
}

/// `record N { v: Boolean; next?: N; }` and `tuple T(Boolean)`.
pub fn types() -> TypeEnvironment {
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
pub fn sequence_of(element: TypeForm) -> TypeForm {
    TypeForm::collection(CollectionKind::Sequence, SPAN)
        .with_arguments(vec![element])
        .with_bounds(vec!["0".into(), "5".into()])
}

/// A nested expression form: `f`'s parameters, result and body when the
/// form is nested `levels` times.
#[derive(Clone, Copy, Debug)]
pub enum Form {
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

/// Every nested form, in TC-415's order.
pub const FORMS: [Form; 26] = [
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
    pub fn function(self, levels: usize) -> FunctionDeclaration {
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

/// `form` nested `levels` times, beside `g(c: Boolean): Boolean`,
/// `h(n: Int[0, 9]): Int[0, 9]`, `gs(c: Sequence<Boolean>[0, 5]): Boolean`
/// and `tv(t: T): Boolean`, with the alias `Total = Integer`.
pub fn declarations(form: Form, levels: usize) -> PackageDeclarations {
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
}
