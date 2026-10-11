// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-091-AC-13 (TC-399): a function compiled from complete-V1 source text
//! through S1, S2 and the assembler is checked, linked and called by name.

use ix_trace_rs::trace;
use qsl_eval::value::{CheckedPackageEvaluation, QualifiedName};
use qsl_package::CheckedPackage;
use qsl_semantics::check::PackageDeclarations;
use qsl_semantics::family::FamilyOutcome;
use qsl_semantics::model::object_environment::ObjectEnvironment;
use qsl_semantics::value::{CatalogRole, DefinitionLock};
use quire_exact::{
    DivisionProfile, Integer, Meter, Outcome, ScalarLimits, Undefined, Value, ValueType,
};
use quire_semantic_value::checking::CheckingLimits;

const UNIT: &str = "language \"ix:native\" edition \"1-draft\";\n\
    profile v = \"quire.value.complete/v1\";\n\
    type Digit = Int[0, 9];\n\
    function inc using v(x: Digit): Int[0, 10] pure { x + 1 }\n\
    function two using v(): Int[0, 10] pure { inc(1) }\n";

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

#[trace("FR-091-AC-12", "FR-091-AC-13", "TC-399")]
#[test]
fn a_function_compiled_from_source_is_called_by_name() {
    let parsed = qsl_cst::parse(
        qsl_foundation::SourceIdentity::new("a", "u", "git", "1"),
        "unit.native",
        UNIT.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let unit = qsl_forms::build_unit(&parsed).expect("S2 builds the unit");
    let declarations = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        unit,
        Vec::new(),
        Vec::new(),
    )
    .expect("the assembler builds the package declarations");
    let digit =
        ValueType::Int(quire_exact::IntegerInterval::new(0_i64.into(), 9_i64.into()).unwrap());
    assert_eq!(declarations.aliases, [("Digit".to_owned(), digit)]);
    let names: Vec<&str> = declarations
        .functions
        .iter()
        .map(|function| function.name.as_str())
        .collect();
    assert_eq!(names, ["inc", "two"]);
    let package = CheckedPackage::link(
        declarations
            .check(CheckingLimits::default())
            .expect("the package checks"),
    );
    let mut meter = Meter::new(UNLIMITED);
    let evaluation = package
        .call(
            &QualifiedName::unqualified("two").expect("two is an identifier"),
            Vec::new(),
            &ObjectEnvironment::default(),
            &mut meter,
        )
        .expect("the call runs");
    let FamilyOutcome::Evaluated(outcome) = evaluation.outcome else {
        panic!("a kernel outcome, not {:?}", evaluation.outcome);
    };
    assert_eq!(
        format!("{outcome:?}"),
        format!(
            "{:?}",
            Outcome::Completed(Value::Integer(Integer::from(2_i64)))
        )
    );
}

/// The unit's package, checked and linked, over source `declarations`.
fn package_of(declarations: &str) -> CheckedPackage {
    let text = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n\
         {declarations}\n"
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
        unit,
        Vec::new(),
        Vec::new(),
    )
    .expect("the assembler builds the package declarations");
    CheckedPackage::link(
        declarations
            .check(CheckingLimits::default())
            .expect("the package checks"),
    )
}

/// Whether calling `name` with no arguments completes with `true`.
fn completes_true(package: &CheckedPackage, name: &str) -> bool {
    let mut meter = Meter::new(UNLIMITED);
    let evaluation = package
        .call(
            &QualifiedName::unqualified(name).expect("an identifier"),
            Vec::new(),
            &ObjectEnvironment::default(),
            &mut meter,
        )
        .expect("the call runs");
    let FamilyOutcome::Evaluated(outcome) = evaluation.outcome else {
        panic!("a kernel outcome, not {:?}", evaluation.outcome);
    };
    format!("{outcome:?}") == format!("{:?}", Outcome::Completed(Value::Boolean(true)))
}

fn integer_operator(operator: &str, profile: Option<DivisionProfile>) -> CheckedPackage {
    let text = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n\
         function op using v(a: Integer, b: Integer): Integer pure {{ a {operator} b }}\n"
    );
    let parsed = qsl_cst::parse(
        qsl_foundation::SourceIdentity::new("a", "division", "git", "1"),
        "division.native",
        text.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the operator");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let unit = qsl_forms::build_unit(&parsed).expect("S2 maps the operator");
    let mut declarations = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        unit,
        Vec::new(),
        Vec::new(),
    )
    .expect("the assembler retains the selection");
    if let Some(profile) = profile {
        let role = match profile {
            DivisionProfile::Truncating => CatalogRole::IntegerDivisionTruncating,
            DivisionProfile::Floor => CatalogRole::IntegerDivisionFloor,
            DivisionProfile::Euclidean => CatalogRole::IntegerDivisionEuclidean,
        };
        let lock = DefinitionLock::pinned();
        declarations.integer_division = Some(
            lock.admit_integer_division(&[lock.entry(role).reference()], None)
                .unwrap(),
        );
    }
    CheckedPackage::link(
        declarations
            .check(CheckingLimits::default())
            .expect("the operator checks"),
    )
}

fn call_integer(package: &CheckedPackage, a: i128, b: i128) -> Outcome<Value> {
    let evaluation = package
        .call(
            &QualifiedName::unqualified("op").unwrap(),
            vec![
                Value::Integer(Integer::from(a)),
                Value::Integer(Integer::from(b)),
            ],
            &ObjectEnvironment::default(),
            &mut Meter::new(UNLIMITED),
        )
        .expect("the call runs");
    match evaluation.outcome {
        FamilyOutcome::Evaluated(outcome) => outcome,
        other => panic!("expected a kernel outcome, got {other:?}"),
    }
}

#[trace(
    "FR-091-AC-39",
    "QSpec-FR-147-AC-1",
    "QSpec-FR-147-AC-4",
    "QSpec-FR-147-AC-5",
    "TC-394"
)]
#[test]
fn integer_division_operators_follow_each_selected_signed_law() {
    let vectors = [
        (
            DivisionProfile::Truncating,
            [(2, 1), (-2, -1), (-2, 1), (2, -1)],
        ),
        (DivisionProfile::Floor, [(2, 1), (-3, 2), (-3, -2), (2, -1)]),
        (
            DivisionProfile::Euclidean,
            [(2, 1), (-3, 2), (-2, 1), (3, 2)],
        ),
    ];
    let pairs = [(7, 3), (-7, 3), (7, -3), (-7, -3)];
    for (profile, expected) in vectors {
        let div = integer_operator("div", Some(profile));
        let rem = integer_operator("rem", Some(profile));
        let modulo = integer_operator("mod", Some(profile));
        for ((a, b), (q, r)) in pairs.into_iter().zip(expected) {
            assert_eq!(
                format!("{:?}", call_integer(&div, a, b)),
                format!("{:?}", Outcome::Completed(Value::Integer(Integer::from(q))))
            );
            assert_eq!(
                format!("{:?}", call_integer(&rem, a, b)),
                format!("{:?}", Outcome::Completed(Value::Integer(Integer::from(r))))
            );
            let euclidean = a.rem_euclid(b.abs());
            assert_eq!(
                format!("{:?}", call_integer(&modulo, a, b)),
                format!(
                    "{:?}",
                    Outcome::Completed(Value::Integer(Integer::from(euclidean)))
                )
            );
        }
    }
    let modulo_without_division = integer_operator("mod", None);
    assert_eq!(
        format!("{:?}", call_integer(&modulo_without_division, -7, 3)),
        format!("{:?}", Outcome::Completed(Value::Integer(Integer::from(2))))
    );
}

#[trace("FR-091-AC-39", "QSpec-FR-147-AC-2", "QSpec-FR-147-AC-5", "TC-394")]
#[test]
fn integer_division_zero_and_signed_minimum_remain_exact() {
    for profile in DivisionProfile::ALL {
        for operator in ["div", "rem", "mod"] {
            let package = integer_operator(operator, Some(profile));
            assert_eq!(
                format!("{:?}", call_integer(&package, 7, 0)),
                format!(
                    "{:?}",
                    Outcome::<Value>::Undefined(Undefined::DivisionByZero)
                )
            );
        }
        let div = integer_operator("div", Some(profile));
        let expected: Integer = "170141183460469231731687303715884105728".parse().unwrap();
        assert_eq!(
            format!("{:?}", call_integer(&div, i128::MIN, -1)),
            format!("{:?}", Outcome::Completed(Value::Integer(expected)))
        );
    }
}

#[trace("FR-091-AC-28", "TC-481")]
#[test]
fn a_source_enum_is_called_through_the_checked_package() {
    let package = package_of(
        "ordered enum Status { READY, DONE }\n\
         function isReady using v(s: Status): Boolean pure { s = Status::READY }\n\
         function ok using v(): Boolean pure { isReady(Status::READY) }\n\
         function later using v(): Boolean pure { Status::READY < Status::DONE }",
    );
    assert!(completes_true(&package, "ok"));
    assert!(completes_true(&package, "later"));
}

#[trace("FR-091-AC-30", "TC-481")]
#[test]
fn a_source_predicate_is_called_through_the_checked_package() {
    let package = package_of(
        "predicate Positive using v(x: Int[0, 9]): Boolean { x > 0 }\n\
         function three using v(): Boolean pure { Positive(3) }",
    );
    assert!(completes_true(&package, "three"));
}
