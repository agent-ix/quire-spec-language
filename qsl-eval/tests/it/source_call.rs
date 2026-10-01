// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-091-AC-13 (TC-399): a function compiled from complete-V1 source text
//! through S1, S2 and the assembler is checked, linked and called by name.

use ix_trace_rs::trace;
use qsl_eval::value::{CheckedPackageEvaluation, QualifiedName};
use qsl_package::CheckedPackage;
use qsl_semantics::check::PackageDeclarations;
use qsl_semantics::family::FamilyOutcome;
use qsl_semantics::model::object_environment::ObjectEnvironment;
use quire_exact::{Integer, Meter, Outcome, ScalarLimits, Value, ValueType};
use quire_semantic_value::checking::CheckingLimits;

const UNIT: &str = "language \"ix:native\" edition \"1-draft\";\n\
    profile v = \"quire.value.complete/v1\" version \"1\" digest \
    \"sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\";\n\
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
    let unit = qsl_forms::build_unit(&parsed, qsl_forms::FormsLimits::default())
        .expect("S2 builds the unit");
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
         profile v = \"quire.value.complete/v1\" version \"1\" digest \
         \"sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\";\n\
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
    let unit = qsl_forms::build_unit(&parsed, qsl_forms::FormsLimits::default())
        .expect("S2 builds the unit");
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
