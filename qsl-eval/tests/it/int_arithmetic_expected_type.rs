// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSpec FR-149-AC-19 to AC-21 (TC-194): a binary `+`, `-` or `*` operand of
//! `=` or `!=` against an `Int[..]` operand takes that interval as its
//! expected type, so the sum is checked against `Int[0, 3]` as an obligation
//! rather than refused for being an `Integer`; nothing else does.

use ix_trace_rs::trace;
use qsl_eval::value::{CheckedPackageEvaluation, QualifiedName};
use qsl_package::CheckedPackage;
use qsl_semantics::check::{CheckRefusal, PackageDeclarations};
use qsl_semantics::model::object_environment::ObjectEnvironment;
use qsl_semantics::value::{admit_integer_division, DefinitionRole};
use quire_eval::FamilyOutcome;
use quire_exact::{ChargePoint, Integer, Meter, Outcome, ScalarLimits, Value};
use quire_semantic_value::checking::CheckMode;
use quire_semantic_value::checking::CheckingLimits;

const UNLIMITED: ScalarLimits = ScalarLimits {
    integer_bits: u64::MAX,
    decimal_digits: u64::MAX,
    scale_expansion: u64::MAX,
    text_input_bytes: u64::MAX,
    text_scalars: u64::MAX,
    normalized_scalars: u64::MAX,
    unit_edges: u64::MAX,
    value_occurrences: u64::MAX,
    work_units: u64::MAX,
    result_units: u64::MAX,
};

/// The source text of `function f using v(parameters): Boolean pure { body }`
/// beside `extra` declarations.
fn source(extra: &str, parameters: &str, body: &str) -> String {
    format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n\
         {extra}\n\
         function f using v({parameters}): Boolean pure {{ {body} }}\n"
    )
}

/// The assembled declarations of `text`.
fn assemble(text: &str) -> PackageDeclarations {
    let parsed = qsl_cst::parse(
        qsl_foundation::SourceIdentity::new("a", "u"),
        "unit.native",
        text.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let unit = qsl_forms::build_unit(&parsed)
        .unwrap_or_else(|error| panic!("S2 builds {text}: {error:?}"));
    let mut declarations = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        "fixture".to_owned(),
        unit,
        Vec::new(),
        Vec::new(),
    )
    .expect("the assembler admits the function");
    declarations.integer_division = Some(
        admit_integer_division(&[DefinitionRole::IntegerDivisionFloor.reference()], None).unwrap(),
    );
    declarations
}

/// `function f using v(parameters): Boolean pure { body }` checked as a
/// package, or the refusals of its check.
fn compile(parameters: &str, body: &str) -> Result<CheckedPackage, Vec<CheckRefusal>> {
    compile_with("", parameters, body)
}

/// [`compile`] beside `extra` declarations.
fn compile_with(
    extra: &str,
    parameters: &str,
    body: &str,
) -> Result<CheckedPackage, Vec<CheckRefusal>> {
    assemble(&source(extra, parameters, body))
        .check(CheckingLimits::default(), CheckMode::Linked)
        .map(CheckedPackage::link)
}

fn int(value: i64) -> Value {
    Value::Integer(Integer::from(value))
}

/// `f(arguments)` as a Boolean, and the charge points the call admitted.
fn run(package: &CheckedPackage, arguments: Vec<Value>) -> (bool, Vec<ChargePoint>) {
    let mut meter = Meter::new(UNLIMITED);
    let evaluation = package
        .call(
            &QualifiedName::unqualified("f").expect("f is an identifier"),
            arguments,
            &ObjectEnvironment::default(),
            &mut meter,
        )
        .expect("the call runs");
    match evaluation.outcome {
        FamilyOutcome::Evaluated(Outcome::Completed(Value::Boolean(value))) => {
            (value, crate::support::admitted_charge_points(&meter))
        }
        other => panic!("a completed Boolean, not {other:?}"),
    }
}

/// `f(arguments)` as a Boolean.
fn truth(package: &CheckedPackage, arguments: Vec<Value>) -> bool {
    run(package, arguments).0
}

/// The `(code, cause)` of the one refusal of a unit that must not check.
fn refusal(parameters: &str, body: &str) -> (String, Option<String>) {
    refusal_with("", parameters, body)
}

/// [`refusal`] beside `extra` declarations.
fn refusal_with(extra: &str, parameters: &str, body: &str) -> (String, Option<String>) {
    let Err(refusals) = compile_with(extra, parameters, body) else {
        panic!("{body} is refused");
    };
    let [refusal] = refusals.as_slice() else {
        panic!("one refusal for {body}, got {refusals:?}");
    };
    (
        refusal.cause.code().as_str().to_owned(),
        refusal.cause.cause().map(str::to_owned),
    )
}

/// The source byte range the one refusal of `body` is located at.
fn refused_at(parameters: &str, body: &str) -> (usize, usize) {
    let text = source("", parameters, body);
    let Err(refusals) = assemble(&text).check(CheckingLimits::default(), CheckMode::Linked) else {
        panic!("{body} is refused");
    };
    let [refusal] = refusals.as_slice() else {
        panic!("one refusal for {body}, got {refusals:?}");
    };
    let region = assemble(&text)
        .region(&refusal.location)
        .expect("the refusal names a source region");
    (
        usize::try_from(region.start()).expect("a start offset"),
        usize::try_from(region.end()).expect("an end offset"),
    )
}

const TALLIES: &str = "earlier: Int[0, 3], later: Int[0, 3]";

/// FR-149-AC-19: with the guard `earlier < 3` the sum's interval `[1, 3]`
/// lies inside `Int[0, 3]`, so `later = earlier + 1` checks and evaluates,
/// and the relation charges the guard's ordering schedule, the sum's own
/// integer-arithmetic schedule and then the equality's one-pair schedule.
#[trace("QSpec-TC-194", "QSpec-FR-149-AC-19")]
#[test]
fn a_guarded_sum_is_typed_by_the_int_it_is_compared_with() {
    let package = compile(TALLIES, "earlier < 3 and later = earlier + 1")
        .unwrap_or_else(|refusals| panic!("the relation checks: {refusals:?}"));
    assert_eq!(
        run(&package, vec![int(1), int(2)]),
        (
            true,
            vec![
                ChargePoint::FunctionCall,
                ChargePoint::OrderingOperands,
                ChargePoint::OrderingArithmetic,
                ChargePoint::OrderingResultRetain,
                ChargePoint::IntegerArithmeticOperands,
                ChargePoint::IntegerArithmeticArithmetic,
                ChargePoint::IntegerArithmeticResultRetain,
                ChargePoint::EqualityPlanForm,
                ChargePoint::EqualityPlan,
                ChargePoint::EqualityPair,
                ChargePoint::EqualityResultRetain,
                ChargePoint::BooleanResultRetain,
            ]
        )
    );
    assert!(!truth(&package, vec![int(1), int(3)]));
    assert!(!truth(&package, vec![int(3), int(3)]));
}

/// FR-149-AC-20: with the guard removed `later = earlier + 1` refuses
/// `undefined_expression`/`unproved-range` (the sum's interval `[1, 4]`
/// exceeds `Int[0, 3]`), and so does `recovered = recovered + 1`; no Boolean
/// is twiced and the value is not clamped.
#[trace("QSpec-TC-194", "QSpec-FR-149-AC-20")]
#[test]
fn an_unguarded_sum_refuses_unproved_range() {
    let expected = (
        "undefined_expression".to_owned(),
        Some("unproved-range".to_owned()),
    );
    assert_eq!(refusal(TALLIES, "later = earlier + 1"), expected);
    assert_eq!(
        refusal("recovered: Int[0, 3]", "recovered = recovered + 1"),
        expected
    );
    for (parameters, body, sum) in [
        (TALLIES, "later = earlier + 1", "earlier + 1"),
        (
            "recovered: Int[0, 3]",
            "recovered = recovered + 1",
            "recovered + 1",
        ),
    ] {
        let text = source("", parameters, body);
        let (start, end) = refused_at(parameters, body);
        assert_eq!(&text[start..end], sum, "{body}");
        assert_eq!(text.matches(sum).count(), 1, "{body}");
    }
}

/// FR-149-AC-21: the rule covers only a binary `+`, `-` or `*` operand of
/// `=` or `!=` against an `Int[..]` operand, on either side and
/// parenthesized; an `Integer` path, a call, `div`, `rem`, an unannotated `let`-bound name and unary `-` against `Int[0, 3]` are
/// `ill_typed`/`type-mismatch` (`count` is asserted in the qsl-semantics
/// `named_population` suite, which has a model population), an arithmetic operand against a `Rational[..]`
/// or `Decimal[..]` operand is `ill_typed`, and `<`, `<=`, `>` and `>=`
/// supply no expected type, so `n + 1 <= later` for `n` of `Integer` and
/// `earlier + 1 <= later` are well typed.
#[trace("QSpec-TC-194", "QSpec-FR-149-AC-21")]
#[test]
fn only_a_binary_arithmetic_operand_of_an_equality_takes_the_expected_type() {
    let mismatch = ("ill_typed".to_owned(), Some("type-mismatch".to_owned()));
    let parameters = "earlier: Int[0, 3], later: Int[0, 3], n: Integer";
    let twice = "function twice using v(a: Integer): Integer pure { a }";
    for body in [
        "later = n",
        "n = later",
        "later != n",
        "later = twice(1)",
        "later = earlier div 2",
        "later = earlier rem 2",
        "let k = earlier + 1 in later = k",
        "later = -earlier",
        "-earlier = later",
    ] {
        assert_eq!(refusal_with(twice, parameters, body), mismatch, "{body}");
    }

    // Either side and parenthesized operands take the expected type: the
    // guarded sum checks, the unguarded one is an unproved range.
    let unproved = (
        "undefined_expression".to_owned(),
        Some("unproved-range".to_owned()),
    );
    for body in [
        "earlier < 3 and later = (earlier + 1)",
        "earlier < 3 and earlier + 1 = later",
        "earlier < 3 and (earlier + 1) != later",
        "earlier > 0 and later = earlier - 1",
        "earlier < 3 and later = earlier * 1 + 1",
    ] {
        compile(parameters, body).unwrap_or_else(|refusals| panic!("{body} checks: {refusals:?}"));
    }
    for body in [
        "later = (earlier + 1)",
        "earlier + 1 = later",
        "later = earlier - 1",
    ] {
        assert_eq!(refusal(parameters, body), unproved, "{body}");
    }

    // A Rational or Decimal operand takes no integer interval.
    for decimal in ["r: Rational[0, 5; 1, 1]", "r: Decimal[0, 5; 0, 0; exact]"] {
        let (code, _) = refusal(&format!("{parameters}, {decimal}"), "r = earlier + 1");
        assert_eq!(code, "ill_typed", "{decimal}");
    }

    // The orderings supply no expected type.
    for body in [
        "n + 1 <= later",
        "earlier + 1 <= later",
        "n + 1 < later",
        "n + 1 > later",
        "n + 1 >= later",
        "later <= n + 1",
    ] {
        compile(parameters, body).unwrap_or_else(|refusals| panic!("{body} checks: {refusals:?}"));
    }
}
