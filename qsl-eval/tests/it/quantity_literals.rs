// SPDX-License-Identifier: AGPL-3.0-or-later
//! Quantity literals evaluated from source text with their own-unit
//! magnitude (FR-093-AC-33, TC-483).

use ix_trace_rs::trace;
use qsl_eval::value::{CheckedPackageEvaluation, QualifiedName};
use qsl_package::CheckedPackage;
use qsl_semantics::check::PackageDeclarations;
use qsl_semantics::model::intake::SelectedModels;
use qsl_semantics::model::object_environment::ObjectEnvironment;
use quire_eval::FamilyOutcome;
use quire_exact::{Meter, Outcome, ScalarLimits, Value};
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

const FUNCTIONS: &str = "\
    dimension Time;\n\
    unit s : Time = rational(1, 1);\n\
    unit ms : Time = rational(1, 1000) * s;\n\
    function total using v(): Boolean pure { 3 ms + 2 ms = 5 ms }\n\
    function lower using v(): Boolean pure { rational(1, 2) ms < 1 ms }\n\
    function fraction using v(): Boolean pure { 0.5 ms = rational(2, 4) ms }\n\
    function other using v(): Boolean pure { 3 ms = 4 ms }\n\
    function negative using v(): Boolean pure { rational(-3, 1) ms < 0 ms }\n\
    function converted using v(): Boolean pure { convert<Quantity<s>>(1000 ms) = 1 s }\n";

fn package() -> CheckedPackage {
    let text = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n{FUNCTIONS}"
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
        SelectedModels::default(),
        Vec::new(),
    )
    .expect("the assembler builds the declarations");
    CheckedPackage::link(
        declarations
            .check(CheckingLimits::default())
            .expect("the unit checks"),
    )
}

fn truth(package: &CheckedPackage, name: &str) -> bool {
    let evaluation = package
        .call(
            &QualifiedName::unqualified(name).expect("an identifier"),
            vec![],
            &ObjectEnvironment::default(),
            &mut Meter::new(UNLIMITED),
        )
        .expect("the call runs");
    match evaluation.outcome {
        FamilyOutcome::Evaluated(Outcome::Completed(Value::Boolean(truth))) => truth,
        other => panic!("a completed Boolean, not {other:?}"),
    }
}

/// FR-093-AC-33: a quantity literal evaluates to its magnitude in its own
/// unit; equal magnitudes in one unit are equal, and a conversion into
/// another unit rescales it.
#[trace("FR-093-AC-33", "TC-483")]
#[test]
fn a_quantity_literal_evaluates_with_its_own_unit_magnitude() {
    let package = package();
    assert!(truth(&package, "total"));
    assert!(truth(&package, "lower"));
    assert!(truth(&package, "fraction"));
    assert!(!truth(&package, "other"));
    assert!(truth(&package, "negative"));
    assert!(truth(&package, "converted"));
}
