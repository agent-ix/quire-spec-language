// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSpec FR-143-AC-18 to AC-20 (TC-188): `absent` is the `none` of the
//! `Option<T>` its position expects; with no unique expected type, or one
//! that is not an option, it refuses; a `?` field takes no `absent` literal.

use ix_trace_rs::trace;
use qsl_eval::value::{CheckedPackageEvaluation, QualifiedName};
use qsl_package::CheckedPackage;
use qsl_semantics::check::{CheckRefusal, PackageDeclarations};
use qsl_semantics::model::object_environment::ObjectEnvironment;
use quire_eval::FamilyOutcome;
use quire_exact::{Integer, Meter, OptionValue, Outcome, ScalarLimits, Value, ValueType};
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

const RECORD: &str = "record P { a: Integer; b: Integer?; }\n\
                      record Q { a: Integer; v: Option<Integer>; }\n";

/// A complete-value unit holding `declarations`.
fn compile(declarations: &str) -> Result<CheckedPackage, Vec<CheckRefusal>> {
    let text = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n{RECORD}{declarations}\n"
    );
    let parsed = qsl_cst::parse(
        qsl_foundation::SourceIdentity::new("a", "u", "git", "1"),
        "unit.native",
        text.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let unit = qsl_forms::build_unit(&parsed).expect("S2 builds the unit");
    let declarations = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        "fixture".to_owned(),
        unit,
        Vec::new(),
        Vec::new(),
    )
    .expect("the assembler admits the declarations");
    declarations
        .check(CheckingLimits::default())
        .map(CheckedPackage::link)
}

fn none() -> Value {
    OptionValue::none(ValueType::Integer)
}

fn some(value: i64) -> Value {
    OptionValue::present(ValueType::Integer, Value::Integer(Integer::from(value))).unwrap()
}

/// The outcome of `name(arguments)`.
fn call(package: &CheckedPackage, name: &str, arguments: Vec<Value>) -> Outcome<Value> {
    let mut meter = Meter::new(UNLIMITED);
    let evaluation = package
        .call(
            &QualifiedName::unqualified(name).expect("an identifier"),
            arguments,
            &ObjectEnvironment::default(),
            &mut meter,
        )
        .expect("the call runs");
    let FamilyOutcome::Evaluated(outcome) = evaluation.outcome else {
        panic!("a kernel outcome, not {:?}", evaluation.outcome);
    };
    outcome
}

fn truth(outcome: Outcome<Value>) -> bool {
    match outcome {
        Outcome::Completed(Value::Boolean(value)) => value,
        other => panic!("a completed Boolean, not {other:?}"),
    }
}

/// The `(code, cause)` of the one refusal of `declarations`.
fn refusal(declarations: &str) -> (String, Option<String>) {
    let Err(refusals) = compile(declarations) else {
        panic!("{declarations} is refused");
    };
    let [refusal] = refusals.as_slice() else {
        panic!("one refusal for {declarations}, got {refusals:?}");
    };
    (
        refusal.cause.code().as_str().to_owned(),
        refusal.cause.cause().map(str::to_owned),
    )
}

/// FR-143-AC-18: `absent` is the `none` of its position's unique expected
/// `Option<T>`: either operand of `=` against an `Option<Integer>`, a call
/// argument, a function result, an `if` branch beside an `Option<Integer>`
/// and an `Option` record field; `x = y` of two options holding none is true
/// and `x = absent` is true exactly when `x` holds none.
#[trace("QSpec-TC-188", "QSpec-FR-143-AC-18")]
#[test]
fn absent_denotes_the_none_of_the_option_its_position_expects() {
    let package = compile(
        "function left using v(x: Option<Integer>): Boolean pure { x = absent }\n\
         function right using v(x: Option<Integer>): Boolean pure { absent = x }\n\
         function both using v(x: Option<Integer>, y: Option<Integer>): Boolean pure { x = y }\n\
         function take using v(x: Option<Integer>): Boolean pure { x = absent }\n\
         function passed using v(): Boolean pure { take(absent) }\n\
         function produce using v(): Option<Integer> pure { absent }\n\
         function choose using v(x: Option<Integer>): Option<Integer> pure { if true then absent else x }\n\
         function built using v(): Q pure { Q { a: 1, v: absent } }\n\
         function built_none using v(q: Q): Boolean pure { q.v = absent }",
    )
    .unwrap_or_else(|refusals| panic!("every position admits absent: {refusals:?}"));
    for name in ["left", "right"] {
        assert!(truth(call(&package, name, vec![none()])), "{name}");
        assert!(!truth(call(&package, name, vec![some(1)])), "{name}");
    }
    assert!(truth(call(&package, "both", vec![none(), none()])));
    assert!(!truth(call(&package, "both", vec![none(), some(1)])));
    assert!(truth(call(&package, "passed", vec![])));
    let holds_none = |outcome: Outcome<Value>| {
        format!("{outcome:?}") == format!("{:?}", Outcome::Completed(none()))
    };
    assert!(holds_none(call(&package, "produce", vec![])));
    assert!(holds_none(call(&package, "choose", vec![some(7)])));
}

/// FR-143-AC-19: an `absent` with no unique expected type (both operands of
/// `=`, `let x = absent in x`, `present(absent)`, `value(absent)`) refuses
/// `ill_typed`/`ambiguous-literal`, and one whose expected type is not an
/// option (an `Integer` parameter, a `Set<Integer>`) refuses
/// `ill_typed`/`type-mismatch`; neither is read as the `none` of a guessed
/// type.
#[trace("QSpec-TC-188", "QSpec-FR-143-AC-19")]
#[test]
fn absent_with_no_unique_option_type_refuses() {
    let ambiguous = ("ill_typed".to_owned(), Some("ambiguous-literal".to_owned()));
    for body in [
        "absent = absent",
        "let x = absent in true",
        "present(absent)",
        "value(absent) = 1",
    ] {
        assert_eq!(
            refusal(&format!("function f using v(): Boolean pure {{ {body} }}")),
            ambiguous,
            "{body}"
        );
    }
    let mismatch = ("ill_typed".to_owned(), Some("type-mismatch".to_owned()));
    for declaration in [
        "function f using v(n: Integer): Boolean pure { n = absent }",
        "function f using v(s: Set<Integer>[0, 2]): Boolean pure { s = absent }",
        "function f using v(): Integer pure { absent }",
    ] {
        assert_eq!(refusal(declaration), mismatch, "{declaration}");
    }
}

/// FR-143-AC-20: a `?` field takes no `absent` literal (`P { a: 1, b: absent }`
/// refuses `ill_typed`/`type-mismatch`) while omitting `b` still constructs the
/// absent slot, `null` for an `Option<Integer>` parameter stays `ill_typed`,
/// and `absent` after a `lookup`'s `)` is a mode, never a second mode.
#[trace("QSpec-TC-188", "QSpec-FR-143-AC-20")]
#[test]
fn an_optional_field_takes_no_absent_literal() {
    assert_eq!(
        refusal("function f using v(): P pure { P { a: 1, b: absent } }"),
        ("ill_typed".to_owned(), Some("type-mismatch".to_owned()))
    );
    compile("function f using v(): P pure { P { a: 1 } }")
        .unwrap_or_else(|refusals| panic!("omitting b constructs the absent slot: {refusals:?}"));
    let (code, _) = refusal("function f using v(): Q pure { Q { a: 1, v: null } }");
    assert_eq!(code, "ill_typed");

    let text = "language \"ix:native\" edition \"1-draft\";\n\
                profile v = \"quire.value.complete/v1\";\n\
                function f using v(p: Population<Integer>[2], r: Integer): Boolean pure \
                { lookup<Integer>(p, r) absent absent = r }\n";
    let parsed = qsl_cst::parse(
        qsl_foundation::SourceIdentity::new("a", "u", "git", "1"),
        "unit.native",
        text.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    assert!(!parsed.is_admissible(), "`absent absent` is not a mode");
    let rendered = format!("{:?}", parsed.diagnostics());
    assert!(
        rendered.contains("InvalidSyntax") && rendered.contains("UnexpectedToken"),
        "{rendered}"
    );
}
