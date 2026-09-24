// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-415 step 11 (FR-093-AC-15, QSL-228): the explicit-stack walks close
//! each binder's scope when its body is done and walk every branch. Typing
//! refuses a binder read after its body and admits a sibling that reuses the
//! name; lowering gives a sibling binder its enclosing level and resolves a
//! read to the binding in scope; the definedness walk proves an obligation
//! that only one `if` branch holds.

use qsl_forms::Accumulation;

use super::*;
use crate::check::Obligation;

fn integer() -> TypeForm {
    TypeForm::builtin(BuiltinType::Integer, SPAN)
}

fn boxed(expression: Expression) -> Box<Expression> {
    Box::new(expression)
}

fn sequence() -> TypeForm {
    TypeForm::collection(CollectionKind::Sequence, SPAN)
        .with_arguments(vec![int_form(0, 9)])
        .with_bounds(vec!["0".into(), "5".into()])
}

fn option() -> TypeForm {
    TypeForm::builtin(BuiltinType::Option, SPAN).with_arguments(vec![int_form(0, 9)])
}

fn let_in(name: &str, value: Expression, body: Expression) -> Expression {
    Expression::Let {
        name: name.to_owned(),
        value: boxed(value),
        body: boxed(body),
    }
}

/// A binder form over `s` that binds `v` (and `acc` for `fold`) around
/// `body`. Each is Integer-valued but `forall`, which is Boolean.
#[derive(Clone, Copy, Debug)]
enum Binder {
    Let,
    Count,
    Sum,
    Forall,
    Fold,
}

const BINDERS: [Binder; 5] = [
    Binder::Let,
    Binder::Count,
    Binder::Sum,
    Binder::Forall,
    Binder::Fold,
];

impl Binder {
    /// This form, binding `v`, whose body reads `v`.
    fn expression(self) -> Expression {
        let v = || name_expr("v");
        match self {
            Self::Let => let_in("v", name_expr("x"), v()),
            Self::Count => Expression::Count {
                result_type: "Total".to_owned(),
                binder: "v".to_owned(),
                source: boxed(name_expr("s")),
                predicate: boxed(binary(BinaryOperator::Less, v(), integer_expr(5))),
            },
            Self::Sum => Expression::Sum {
                result_type: "Total".to_owned(),
                binder: "v".to_owned(),
                source: boxed(name_expr("s")),
                summand: boxed(v()),
            },
            Self::Forall => Expression::Query {
                query: BinderQuery::Forall,
                binder: "v".to_owned(),
                source: boxed(name_expr("s")),
                body: boxed(binary(BinaryOperator::Less, v(), integer_expr(5))),
            },
            Self::Fold => Expression::Accumulate {
                form: Accumulation::Fold,
                accumulator_type: "Total".to_owned(),
                accumulator: "acc".to_owned(),
                binder: "v".to_owned(),
                source: boxed(name_expr("s")),
                step: boxed(binary(BinaryOperator::Add, name_expr("acc"), v())),
                identity: Some(boxed(integer_expr(0))),
            },
        }
    }

    /// `this ⊕ right`: `+` for an Integer form, `and` for `forall`.
    fn beside(self, right: Expression) -> (TypeForm, Expression) {
        match self {
            Self::Forall => (
                boolean(),
                binary(BinaryOperator::And, self.expression(), right),
            ),
            Self::Let | Self::Count | Self::Sum | Self::Fold => (
                integer(),
                binary(BinaryOperator::Add, self.expression(), right),
            ),
        }
    }

    /// A read of `name` after this form, as the right operand's type needs.
    fn read_after(self, name: &str) -> Expression {
        match self {
            Self::Forall => binary(BinaryOperator::Less, name_expr(name), integer_expr(5)),
            Self::Let | Self::Count | Self::Sum | Self::Fold => name_expr(name),
        }
    }
}

/// Check `f(a: Boolean, x: Int[0, 9], s: Sequence<Int[0, 9]>[0, 5], o:
/// Option<Int[0, 9]>): result = body` with the alias `Total = Integer`.
fn check_body(result: TypeForm, body: Expression) -> Result<CheckedGraph, Vec<CheckRefusal>> {
    PackageDeclarations {
        functions: vec![function(
            "f",
            &[
                ("a", boolean()),
                ("x", int_form(0, 9)),
                ("s", sequence()),
                ("o", option()),
            ],
            result,
            None,
            body,
        )],
        aliases: vec![("Total".to_owned(), ValueType::Integer)],
        ..PackageDeclarations::new(fixture_owner())
    }
    .check(CheckingLimits::default())
}

/// A `let`, `count`, `sum`, query or `fold` binder read after the form's
/// body refuses as a missing name, `fold`'s accumulator too.
#[trace("FR-093-AC-15", "TC-415")]
#[test]
fn a_binder_read_after_its_body_is_a_missing_name() {
    let reads = BINDERS
        .into_iter()
        .map(|binder| (binder, "v"))
        .chain([(Binder::Fold, "acc")]);
    for (binder, name) in reads {
        let (result, body) = binder.beside(binder.read_after(name));
        let refusals =
            check_body(result, body).expect_err("a binder is out of scope after its body");
        assert!(
            refusals
                .iter()
                .any(|refusal| refusal.cause == CheckCause::MissingName(name.to_owned())),
            "{binder:?} then `{name}`: {refusals:?}"
        );
    }
}

/// Each binder form beside a sibling that binds the same name checks: the
/// first binding is out of scope when the second is bound.
#[trace("FR-093-AC-15", "TC-415")]
#[test]
fn a_sibling_may_reuse_a_binder_name() {
    for binder in BINDERS {
        let (result, body) = binder.beside(binder.expression());
        if let Err(refusals) = check_body(result, body) {
            panic!("{binder:?} beside itself checks: {refusals:?}");
        }
    }
}

/// `let v = x in ((let w = x in w) + (let u = x in v + u))`: after `f`'s
/// four parameters (levels 0 to 3), `v` is at level 4 and its children `w`
/// and `u` are siblings at level 5; `v + u` reads `v`'s binding.
#[trace("FR-093-AC-15", "TC-415")]
#[test]
fn a_let_after_a_sibling_let_takes_the_enclosing_level() {
    let body = let_in(
        "v",
        name_expr("x"),
        binary(
            BinaryOperator::Add,
            let_in("w", name_expr("x"), name_expr("w")),
            let_in(
                "u",
                name_expr("x"),
                binary(BinaryOperator::Add, name_expr("v"), name_expr("u")),
            ),
        ),
    );
    let graph = check_body(integer(), body).expect("the nested lets check");
    let semantic = graph.semantic_graph();
    let level = |name: &str| {
        preimage(parameter_named(semantic, name))["body"]["members"][1]["value"]["value"].clone()
    };
    assert_eq!(level("v"), json!("4"));
    assert_eq!(level("w"), json!("5"));
    assert_eq!(level("u"), json!("5"));
    let target = |name: &str| json!(parameter_named(semantic, name).key().to_string());
    let reads_v_then_u = semantic.nodes().map(preimage).any(|json| {
        let arguments = &json["body"]["arguments"];
        arguments[0]["target"]["digest"] == target("v")
            && arguments[1]["target"]["digest"] == target("u")
    });
    assert!(reads_v_then_u, "`v + u` reads `v`'s and `u`'s bindings");
}

/// An unproved `value(o)` in only one branch of `if a then … else …`
/// refuses as unproved presence; guarded by `present(o)` it checks.
#[trace("FR-093-AC-15", "TC-415")]
#[test]
fn an_obligation_in_one_branch_is_walked() {
    let value = || Expression::Value(boxed(name_expr("o")));
    let branch = |condition: Expression, then: Expression, otherwise: Expression| Expression::If {
        condition: boxed(condition),
        then: boxed(then),
        otherwise: boxed(otherwise),
    };
    for body in [
        branch(name_expr("a"), value(), integer_expr(0)),
        branch(name_expr("a"), integer_expr(0), value()),
    ] {
        let refusals = check_body(int_form(0, 9), body).expect_err("`value(o)` is unproved");
        assert_eq!(
            refusals
                .iter()
                .map(|refusal| &refusal.cause)
                .collect::<Vec<_>>(),
            [&CheckCause::Unproved(Obligation::Presence)]
        );
    }
    let guarded = branch(
        Expression::Present(boxed(name_expr("o"))),
        value(),
        integer_expr(0),
    );
    if let Err(refusals) = check_body(int_form(0, 9), guarded) {
        panic!("a guarded `value(o)` checks: {refusals:?}");
    }
}
