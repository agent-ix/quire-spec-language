// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-360 / TC-1008: evaluation of a variant value over the FR-360 domain
//! package checked and linked from source.
//!
//! Equality is by case and never a checked-invariant fault (items 6 and 7),
//! it reaches a variant value inside a record, an `Option`, a `Sequence`, a
//! `Set` and a tuple position, and the canonical order of the cases of a `Set`
//! is the ascending UTF-8 bytes of their names. Every assertion compares typed
//! values and outcomes, never message text.

use ix_trace_rs::trace;
use qsl_eval::value::CheckedPackageEvaluation;
use qsl_forms::{BinaryOperator, Expression};
use qsl_package::CheckedPackage;
use qsl_semantics::check::{CheckCause, PackageDeclarations};
use qsl_semantics::family::FamilyOutcome;
use qsl_semantics::model::accounting::ModelNormalizationLimits;
use qsl_semantics::model::intake::{admit_unit, package_input};
use qsl_semantics::model::object_environment::ObjectEnvironment;
use quire_exact::{
    CollectionKind, CollectionType, FieldValue, IllTypedCause, Integer, Meter, Outcome,
    ScalarLimits, Value, ValueType,
};
use quire_semantic_value::checking::{CheckMode, CheckingLimits};

#[path = "../../../qsl-semantics/tests/support/variant_fields.rs"]
mod fixture;

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

/// The FR-360 package linked as model `alias`, in a unit that declares only
/// a function naming no variant type.
fn package_as(alias: &str) -> CheckedPackage {
    package_over(&fixture::document(), alias)
}

/// `document` linked as model `alias`, in a unit that declares only a
/// function naming no variant type.
fn package_over(document: &serde_json::Value, alias: &str) -> CheckedPackage {
    let bytes = serde_json::to_vec(document).expect("the fixture serializes");
    let packages = package_input([bytes.as_slice()]);
    let [(digest, _)] = packages.iter().collect::<Vec<_>>()[..] else {
        panic!("one model package");
    };
    let digest = qsl_semantics::model::key::hex(digest);
    let source = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n\
         model {alias} = {package:?} version \"1\" digest \"sha256-jcs:{digest}\";\n\
         function keep using v(): Boolean pure {{ true }}\n",
        package = fixture::PACKAGE,
    );
    let parsed = qsl_cst::parse(
        qsl_foundation::SourceIdentity::new("test", "variant-fields", "fixture", "fixture:1"),
        "variant-fields.native",
        source.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the source unit");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let unit = qsl_forms::build_unit(&parsed).expect("S2 builds the source unit");
    let selected = admit_unit(
        &unit.selections().models,
        &packages,
        ModelNormalizationLimits::default(),
    )
    .expect("I1 admits the fixture package");
    let declarations = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        unit,
        selected,
        Vec::new(),
    )
    .expect("assembly succeeds");
    CheckedPackage::link(
        declarations
            .check(CheckingLimits::default())
            .unwrap_or_else(|refusals| panic!("the unit checks: {refusals:?}")),
    )
}

fn package() -> CheckedPackage {
    package_as("Config")
}

fn variant_type(package: &CheckedPackage, alias: &str, name: &str) -> ValueType {
    let wanted = format!("{alias}::{name}");
    ValueType::Enum(
        package
            .graph()
            .scope()
            .variant_types()
            .iter()
            .find(|declared| declared.name() == wanted)
            .unwrap_or_else(|| panic!("{wanted} is declared"))
            .shape(),
    )
}

fn name(spelling: &str) -> Expression {
    Expression::name(spelling.to_owned())
}

fn case(spelling: &str) -> Expression {
    name(&format!("Config::{spelling}"))
}

fn equal(left: Expression, right: Expression) -> Expression {
    Expression::binary(BinaryOperator::Equal, left, right)
}

fn unequal(left: Expression, right: Expression) -> Expression {
    Expression::binary(BinaryOperator::NotEqual, left, right)
}

/// The value `expression` evaluates to over `parameters` bound to
/// `arguments`.
fn evaluate(
    package: &CheckedPackage,
    parameters: &[(&str, ValueType)],
    arguments: Vec<Value>,
    expression: &Expression,
) -> Value {
    evaluate_as(package, parameters, arguments, expression, None)
}

/// [`evaluate`] with the type the expression is expected to have, which a
/// collection literal needs for its element type.
fn evaluate_as(
    package: &CheckedPackage,
    parameters: &[(&str, ValueType)],
    arguments: Vec<Value>,
    expression: &Expression,
    expected: Option<&ValueType>,
) -> Value {
    let checked = package
        .graph()
        .check_expression(
            parameters
                .iter()
                .map(|(name, value_type)| ((*name).to_owned(), value_type.clone()))
                .collect(),
            expression,
            expected,
            CheckMode::Linked,
            CheckingLimits::default(),
        )
        .unwrap_or_else(|refusal| panic!("the expression checks: {refusal:?}"));
    let mut meter = Meter::new(UNLIMITED);
    let evaluation = package
        .evaluate(
            &checked,
            arguments,
            &ObjectEnvironment::default(),
            &mut meter,
        )
        .unwrap_or_else(|failure| panic!("the expression runs: {failure:?}"));
    match evaluation.outcome {
        FamilyOutcome::Evaluated(Outcome::Completed(value)) => value,
        other => panic!("a completed value, not {other:?}"),
    }
}

fn holds(package: &CheckedPackage, expression: &Expression) -> bool {
    match evaluate(package, &[], Vec::new(), expression) {
        Value::Boolean(value) => value,
        other => panic!("a Boolean, not {other:?}"),
    }
}

/// Whether two values are one value by the kernel's canonical key order.
fn same_value(left: &Value, right: &Value) -> bool {
    quire_exact::compare_keys(left, right) == Some(std::cmp::Ordering::Equal)
}

fn settings(package: &CheckedPackage, tier: Value, size: i64) -> Value {
    let types = package.graph().scope().types();
    let declaration = types
        .composites()
        .find(|declared| declared.name() == "Config::Settings")
        .expect("Settings is declared")
        .key();
    types
        .record(
            declaration,
            vec![
                ("tier", FieldValue::Present(tier)),
                (
                    "size",
                    FieldValue::Present(Value::Integer(Integer::from(size))),
                ),
            ],
        )
        .unwrap_or_else(|refusal| panic!("Settings constructs: {refusal:?}"))
}

fn elements(value: &Value) -> Vec<Value> {
    match value {
        Value::Collection(collection) => collection.elements().to_vec(),
        other => panic!("a collection, not {other:?}"),
    }
}

/// FR-360-AC-6: `==` over a variant type is true for one case and false for
/// two, `!=` is its negation, in either operand order, and the result is a
/// Boolean rather than a checked-invariant fault.
#[trace("FR-360-AC-6", "TC-1008")]
#[test]
fn equality_over_a_variant_type_is_by_case_in_either_order() {
    let package = package();
    for (left, right, expected) in [
        ("Tier::Gold", "Tier::Gold", true),
        ("Tier::Standard", "Tier::Standard", true),
        ("Tier::Gold", "Tier::Standard", false),
        ("Tier::Standard", "Tier::Gold", false),
    ] {
        assert_eq!(
            holds(&package, &equal(case(left), case(right))),
            expected,
            "{left} == {right}"
        );
        assert_eq!(
            holds(&package, &unequal(case(left), case(right))),
            !expected,
            "{left} != {right}"
        );
    }
}

/// FR-360-AC-6: a case of another variant type with the same name never
/// compares: the comparison refuses at check and never evaluates.
#[trace("FR-360-AC-6", "TC-1008")]
#[test]
fn a_cross_type_equality_refuses_at_check_and_never_evaluates() {
    let package = package();
    let refusal = package
        .graph()
        .check_expression(
            Vec::new(),
            &equal(case("Level::Gold"), case("Tier::Gold")),
            None,
            CheckMode::Linked,
            CheckingLimits::default(),
        )
        .map(|_| ())
        .expect_err("two different variant types do not compare");
    assert_eq!(
        refusal.cause,
        CheckCause::IllTyped(IllTypedCause::TypeMismatch)
    );
}

/// FR-360-AC-6: a case read from a field equals the same case written as a
/// literal, and differs from another.
#[trace("FR-360-AC-6", "TC-1008")]
#[test]
fn a_case_read_from_a_field_equals_the_literal() {
    let package = package();
    let tier = variant_type(&package, "Config", "Tier");
    let standard = evaluate(&package, &[], Vec::new(), &case("Tier::Standard"));
    let value = settings(&package, standard, 1);
    let parameters = [(
        "s",
        ValueType::Composite(
            package
                .graph()
                .scope()
                .types()
                .composites()
                .find(|declared| declared.name() == "Config::Settings")
                .unwrap()
                .key(),
        ),
    )];
    for (literal, expected) in [("Tier::Standard", true), ("Tier::Gold", false)] {
        let read = Expression::field(name("s"), "tier");
        let found = evaluate(
            &package,
            &parameters,
            vec![value.clone()],
            &equal(read, case(literal)),
        );
        assert!(
            matches!(found, Value::Boolean(value) if value == expected),
            "s.tier == {literal}"
        );
    }
    assert!(matches!(tier, ValueType::Enum(_)));
}

/// FR-360-AC-7: two `Settings` records are equal exactly when their `tier` is
/// the same case and their other fields are equal.
#[trace("FR-360-AC-7", "TC-1008")]
#[test]
fn records_compare_their_variant_fields_by_case() {
    let package = package();
    let gold = evaluate(&package, &[], Vec::new(), &case("Tier::Gold"));
    let standard = evaluate(&package, &[], Vec::new(), &case("Tier::Standard"));
    let settings_type = ValueType::Composite(
        package
            .graph()
            .scope()
            .types()
            .composites()
            .find(|declared| declared.name() == "Config::Settings")
            .unwrap()
            .key(),
    );
    let parameters = [("a", settings_type.clone()), ("b", settings_type)];
    for (left, right, expected) in [
        (
            settings(&package, gold.clone(), 1),
            settings(&package, gold.clone(), 1),
            true,
        ),
        (
            settings(&package, gold.clone(), 1),
            settings(&package, standard.clone(), 1),
            false,
        ),
        (
            settings(&package, gold.clone(), 1),
            settings(&package, gold.clone(), 2),
            false,
        ),
    ] {
        let found = evaluate(
            &package,
            &parameters,
            vec![left, right],
            &equal(name("a"), name("b")),
        );
        assert!(matches!(found, Value::Boolean(value) if value == expected));
    }
}

/// FR-360-AC-7: a `Set<Tier>` built from `Gold, Gold, Standard` has two
/// elements and one built from `Standard, Gold` has the same two, each in the
/// order `Gold`, `Standard` (ascending UTF-8 bytes of the case names).
#[trace("FR-360-AC-7", "TC-1008")]
#[test]
fn a_set_of_cases_holds_each_case_once_in_case_name_byte_order() {
    let package = package();
    let gold = evaluate(&package, &[], Vec::new(), &case("Tier::Gold"));
    let standard = evaluate(&package, &[], Vec::new(), &case("Tier::Standard"));
    let set_type = ValueType::collection(CollectionType::new(
        CollectionKind::Set,
        variant_type(&package, "Config", "Tier"),
        None,
    ));
    let set = |cases: &[&str]| {
        evaluate_as(
            &package,
            &[],
            Vec::new(),
            &Expression::collection(
                CollectionKind::Set,
                cases.iter().map(|spelling| case(spelling)).collect(),
            ),
            Some(&set_type),
        )
    };
    let duplicated = set(&["Tier::Gold", "Tier::Gold", "Tier::Standard"]);
    let reversed = set(&["Tier::Standard", "Tier::Gold"]);
    for value in [&duplicated, &reversed] {
        let members = elements(value);
        assert_eq!(members.len(), 2);
        assert!(same_value(&members[0], &gold));
        assert!(same_value(&members[1], &standard));
    }
    assert!(same_value(&duplicated, &reversed));
}

/// FR-360-AC-7: a `Set<Grade>` over cases declared `Standard`, `Gold`,
/// `Bronze` orders `Bronze`, `Gold`, `Standard`, whatever order it is
/// written in, and the reverse order fails.
#[trace("FR-360-AC-7", "TC-1008")]
#[test]
fn a_set_orders_cases_by_name_bytes_not_declaration_order() {
    let package = package();
    let cases = ["Bronze", "Gold", "Standard"];
    let values: Vec<Value> = cases
        .iter()
        .map(|spelling| {
            evaluate(
                &package,
                &[],
                Vec::new(),
                &case(&format!("Grade::{spelling}")),
            )
        })
        .collect();
    let set_type = ValueType::collection(CollectionType::new(
        CollectionKind::Set,
        variant_type(&package, "Config", "Grade"),
        None,
    ));
    let set = evaluate_as(
        &package,
        &[],
        Vec::new(),
        &Expression::collection(
            CollectionKind::Set,
            ["Standard", "Gold", "Bronze"]
                .iter()
                .map(|spelling| case(&format!("Grade::{spelling}")))
                .collect(),
        ),
        Some(&set_type),
    );
    let members = elements(&set);
    assert_eq!(members.len(), 3);
    for (member, expected) in members.iter().zip(&values) {
        assert!(same_value(member, expected));
    }
    let reversed: Vec<&Value> = values.iter().rev().collect();
    assert!(members
        .iter()
        .zip(reversed)
        .any(|(member, other)| !same_value(member, other)));
}

/// A collection of `kind` over `tier` holding `cases`, formed by the kernel.
fn formed(kind: CollectionKind, tier: &ValueType, cases: &[&Value]) -> Value {
    let collection_type = CollectionType::new(kind, tier.clone(), None);
    quire_exact::form_collection(
        &collection_type,
        cases.iter().map(|value| (*value).clone()).collect(),
        &mut Meter::new(UNLIMITED),
    )
    .unwrap()
    .completed()
    .unwrap()
}

/// `left = right` over two parameters of type `value_type`.
fn equals(package: &CheckedPackage, value_type: &ValueType, left: Value, right: Value) -> bool {
    let parameters = [("a", value_type.clone()), ("b", value_type.clone())];
    match evaluate(
        package,
        &parameters,
        vec![left, right],
        &equal(name("a"), name("b")),
    ) {
        Value::Boolean(value) => value,
        other => panic!("a Boolean, not {other:?}"),
    }
}

/// FR-360-AC-7: two `Sequence<Tier>` compare their cases in supply order.
#[trace("FR-360-AC-7", "TC-1008")]
#[test]
fn a_sequence_of_cases_compares_by_case_in_supply_order() {
    let package = package();
    let tier = variant_type(&package, "Config", "Tier");
    let gold = evaluate(&package, &[], Vec::new(), &case("Tier::Gold"));
    let standard = evaluate(&package, &[], Vec::new(), &case("Tier::Standard"));
    let kind = CollectionKind::Sequence;
    let sequence_type = ValueType::collection(CollectionType::new(kind, tier.clone(), None));
    for (left, right, expected) in [
        (&[&gold, &standard], &[&gold, &standard], true),
        (&[&gold, &standard], &[&standard, &gold], false),
    ] {
        assert_eq!(
            equals(
                &package,
                &sequence_type,
                formed(kind, &tier, left),
                formed(kind, &tier, right)
            ),
            expected
        );
    }
}

/// FR-360-AC-7: two `Set<Tier>` are equal exactly when they hold the same
/// cases, whatever order they were built in.
#[trace("FR-360-AC-7", "TC-1008")]
#[test]
fn a_set_of_cases_compares_by_case_whatever_its_build_order() {
    let package = package();
    let tier = variant_type(&package, "Config", "Tier");
    let gold = evaluate(&package, &[], Vec::new(), &case("Tier::Gold"));
    let standard = evaluate(&package, &[], Vec::new(), &case("Tier::Standard"));
    let kind = CollectionKind::Set;
    let set_type = ValueType::collection(CollectionType::new(kind, tier.clone(), None));
    for (left, right, expected) in [
        (&[&gold, &standard][..], &[&standard, &gold][..], true),
        (&[&gold, &gold, &standard], &[&gold, &standard], true),
        (&[&gold, &standard], &[&gold], false),
        (&[&gold], &[&standard], false),
    ] {
        assert_eq!(
            equals(
                &package,
                &set_type,
                formed(kind, &tier, left),
                formed(kind, &tier, right)
            ),
            expected
        );
    }
}

/// FR-360-AC-7: two `Option<Tier>` are equal exactly when both are `none` or
/// both hold the same case.
#[trace("FR-360-AC-7", "TC-1008")]
#[test]
fn an_option_of_a_case_compares_by_case() {
    let package = package();
    let tier = variant_type(&package, "Config", "Tier");
    let gold = evaluate(&package, &[], Vec::new(), &case("Tier::Gold"));
    let standard = evaluate(&package, &[], Vec::new(), &case("Tier::Standard"));
    let option_type = ValueType::option(tier.clone());
    let held = |value: Option<&Value>| {
        quire_exact::OptionValue::from_admitted(tier.clone(), value.cloned())
    };
    for (left, right, expected) in [
        (None, None, true),
        (None, Some(&gold), false),
        (Some(&gold), None, false),
        (Some(&gold), Some(&gold), true),
        (Some(&gold), Some(&standard), false),
    ] {
        assert_eq!(
            equals(&package, &option_type, held(left), held(right)),
            expected
        );
    }
}

/// FR-360-AC-6 over cases that are not source identifiers: `=` holds exactly
/// for the same exact name and `!=` is its negation, in either operand order,
/// whatever the case is spelled (`2x`, `Café`, `Gold Plus`).
#[trace("FR-360-AC-6", "TC-1008")]
#[test]
fn equality_over_non_identifier_cases_is_by_the_exact_name() {
    const CASES: [&str; 3] = ["2x", "Café", "Gold Plus"];
    let package = package_over(
        &fixture::with_types(vec![fixture::variant_type("Odd", &CASES)]),
        "Config",
    );
    let odd = variant_type(&package, "Config", "Odd");
    let declared = package
        .graph()
        .scope()
        .variant_types()
        .iter()
        .find(|declared| declared.name() == "Config::Odd")
        .expect("Odd is declared");
    let values: Vec<Value> = CASES
        .iter()
        .map(|name| {
            let rank = declared
                .cases()
                .iter()
                .position(|case| case.name() == *name)
                .expect("the case is declared");
            Value::Enum(quire_exact::EnumMember::new(
                declared.cases()[rank].variant(),
                u32::try_from(rank).unwrap(),
            ))
        })
        .collect();
    let parameters = [("a", odd.clone()), ("b", odd)];
    for (left, a) in values.iter().enumerate() {
        for (right, b) in values.iter().enumerate() {
            let arguments = || vec![a.clone(), b.clone()];
            let equals = evaluate(
                &package,
                &parameters,
                arguments(),
                &equal(name("a"), name("b")),
            );
            let differs = evaluate(
                &package,
                &parameters,
                arguments(),
                &unequal(name("a"), name("b")),
            );
            assert!(
                matches!(equals, Value::Boolean(held) if held == (left == right)),
                "{left} = {right}"
            );
            assert!(
                matches!(differs, Value::Boolean(held) if held == (left != right)),
                "{left} != {right}"
            );
        }
    }
}
