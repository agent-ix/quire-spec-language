// SPDX-License-Identifier: AGPL-3.0-or-later
//! Quantity literals evaluated from source text with their own-unit
//! magnitude (FR-093-AC-33, TC-483).

use ix_trace_rs::trace;
use qsl_eval::value::{CheckedPackageEvaluation, QualifiedName};
use qsl_package::CheckedPackage;
use qsl_semantics::check::PackageDeclarations;
use qsl_semantics::check::{CheckCause, CheckRefusal};
use qsl_semantics::model::intake::SelectedModels;
use qsl_semantics::model::object_environment::ObjectEnvironment;
use quire_eval::FamilyOutcome;
use quire_exact::{Meter, Outcome, ScalarLimits, Value};
use quire_semantic_value::checking::CheckingLimits;
use quire_semantic_value::loss::ValueLoss;

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
    dimension Length;\n\
    unit m : Length = rational(1, 1);\n\
    unit inch : Length = rational(127, 5000) * m;\n\
    record Carried { amount: Quantity<m, Int[-2, 2]>; }\n\
    function total using v(): Boolean pure { 3 ms + 2 ms = 5 ms }\n\
    function lower using v(): Boolean pure { rational(1, 2) ms < 1 ms }\n\
    function fraction using v(): Boolean pure { 0.5 ms = rational(2, 4) ms }\n\
    function other using v(): Boolean pure { 3 ms = 4 ms }\n\
    function negative using v(): Boolean pure { rational(-3, 1) ms < 0 ms }\n\
    function converted using v(): Boolean pure { convert<Quantity<s>>(1000 ms) = 1 s }\n\
    function carried_integer using v(): Quantity<m, Int[-2, 2]> pure { convert<Quantity<m, Int[-2, 2]>>(2 m) }\n\
    function carried_rounded using v(): Quantity<m, Decimal[-100, 100; 2, 2; nearest-even]> pure { convert<Quantity<m, Decimal[-100, 100; 2, 2; nearest-even]>>(1 inch) }\n\
    function carried_exact_failure using v(): Quantity<m, Int[-2, 2]> pure { convert<Quantity<m, Int[-2, 2]>>(1 inch) }\n\
    function carried_decimal_exact_failure using v(): Quantity<m, Decimal[-100, 100; 2, 2; exact]> pure { convert<Quantity<m, Decimal[-100, 100; 2, 2; exact]>>(1 inch) }\n\
    function carried_integer_outside using v(): Quantity<m, Int[-2, 2]> pure { convert<Quantity<m, Int[-2, 2]>>(3 m) }\n\
    function carried_decimal_outside using v(): Quantity<m, Decimal[-2, 2; 0, 0; exact]> pure { convert<Quantity<m, Decimal[-2, 2; 0, 0; exact]>>(3 m) }\n\
    function carried_back using v(): Quantity<m> pure { convert<Quantity<m>>(convert<Quantity<m, Int[-2, 2]>>(2 m)) }\n\
    function carried_two using v(): Quantity<m, Int[-2, 2]> pure { convert<Quantity<m, Int[-2, 2]>>(2 m) }\n\
    function carried_one using v(): Quantity<m, Int[-2, 2]> pure { convert<Quantity<m, Int[-2, 2]>>(1 m) }\n\
    function carried_record_equal using v(): Boolean pure { Carried{ amount: carried_two() } = Carried{ amount: carried_two() } }\n\
    function carried_record_unequal using v(): Boolean pure { Carried{ amount: carried_two() } = Carried{ amount: carried_one() } }\n\
    function carried_sequence using v(): Sequence<Quantity<m, Int[-2, 2]>>[1, 1] pure { sequence[carried_two()] }\n\
    function carried_sequence_one using v(): Sequence<Quantity<m, Int[-2, 2]>>[1, 1] pure { sequence[carried_one()] }\n\
    function carried_collection_equal using v(): Boolean pure { carried_sequence() = carried_sequence() }\n\
    function carried_collection_unequal using v(): Boolean pure { carried_sequence() = carried_sequence_one() }\n";

fn package() -> CheckedPackage {
    package_with(FUNCTIONS).expect("the quantity unit checks")
}

fn package_with(functions: &str) -> Result<CheckedPackage, Vec<CheckRefusal>> {
    let text = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n{functions}"
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
        SelectedModels::default(),
        Vec::new(),
    )
    .expect("the assembler builds the declarations");
    declarations
        .check(CheckingLimits::default())
        .map(CheckedPackage::link)
}

fn truth(package: &CheckedPackage, name: &str) -> bool {
    let evaluation = package
        .call(
            &QualifiedName::unqualified(name).expect("an identifier"),
            vec![],
            &ObjectEnvironment::default(),
            &mut Meter::new(UNLIMITED),
        )
        .unwrap_or_else(|error| panic!("the {name} call runs: {error:?}"));
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

/// FR-497-AC-4 (TC-1287): a source-checked carried conversion returns its
/// exact value with the target carrier admitted as the function result.
#[trace("FR-497-AC-4", "TC-1287")]
#[test]
fn a_carried_quantity_conversion_returns_a_carried_value() {
    let package = package();
    let evaluation = package
        .call(
            &QualifiedName::unqualified("carried_integer").expect("an identifier"),
            vec![],
            &ObjectEnvironment::default(),
            &mut Meter::new(UNLIMITED),
        )
        .expect("the checked function call runs");
    match evaluation.outcome {
        FamilyOutcome::Evaluated(Outcome::Completed(Value::Quantity(quantity))) => {
            assert_eq!(quantity.magnitude().to_string(), "2/1");
        }
        other => panic!("a carried Int conversion, not {other:?}"),
    }
}

/// FR-497-AC-4/5/7 (TC-1287): rounding, exact refusals, carrier bounds and
/// conversion back to an uncarried quantity retain their checked behavior.
#[trace("FR-497-AC-4", "FR-497-AC-5", "FR-497-AC-7", "TC-1287")]
#[test]
fn carried_quantity_rounding_refusal_and_back_conversion() {
    let package = package();
    let call = |name: &str| {
        package
            .call(
                &QualifiedName::unqualified(name).expect("an identifier"),
                vec![],
                &ObjectEnvironment::default(),
                &mut Meter::new(UNLIMITED),
            )
            .unwrap_or_else(|error| panic!("the {name} function call runs: {error:?}"))
    };
    let exact_failure = call("carried_exact_failure");
    assert!(exact_failure.losses.is_empty());
    assert!(
        exact_failure.location.is_some(),
        "the refusal has a source locus"
    );
    match exact_failure.outcome {
        FamilyOutcome::Evaluated(Outcome::Refused(quire_exact::Refusal::InexactConversion {
            target: quire_exact::InexactTarget::Integer(target),
        })) => {
            assert_eq!(target.lower().to_string(), "-2");
            assert_eq!(target.upper().to_string(), "2");
        }
        other => panic!("an exact carried conversion refusal, not {other:?}"),
    }
    let decimal_exact_failure = call("carried_decimal_exact_failure");
    assert!(decimal_exact_failure.losses.is_empty());
    assert!(
        decimal_exact_failure.location.is_some(),
        "the refusal has a source locus"
    );
    match decimal_exact_failure.outcome {
        FamilyOutcome::Evaluated(Outcome::Refused(quire_exact::Refusal::InexactConversion {
            target: quire_exact::InexactTarget::Decimal(target),
        })) => {
            assert_eq!(target.lower().to_string(), "-100");
            assert_eq!(target.upper().to_string(), "100");
        }
        other => panic!("an exact carried Decimal conversion refusal, not {other:?}"),
    }
    match call("carried_integer_outside").outcome {
        FamilyOutcome::Evaluated(Outcome::Refused(quire_exact::Refusal::IntegerOutOfDomain {
            target,
        })) => {
            assert_eq!(target.lower().to_string(), "-2");
            assert_eq!(target.upper().to_string(), "2");
        }
        other => panic!("an out-of-domain carried Int refusal, not {other:?}"),
    }
    match call("carried_decimal_outside").outcome {
        FamilyOutcome::Evaluated(Outcome::Refused(quire_exact::Refusal::DecimalOutOfDomain {
            target,
        })) => {
            assert_eq!(target.lower().to_string(), "-2");
            assert_eq!(target.upper().to_string(), "2");
        }
        other => panic!("an out-of-domain carried Decimal refusal, not {other:?}"),
    }
    match call("carried_back").outcome {
        FamilyOutcome::Evaluated(Outcome::Completed(Value::Quantity(quantity))) => {
            assert_eq!(quantity.magnitude().to_string(), "2/1");
        }
        other => panic!("a carried-to-Quantity conversion, not {other:?}"),
    }
    let rounded = call("carried_rounded");
    assert_eq!(rounded.losses.len(), 1, "the carrier rounding is retained");
    assert!(matches!(rounded.losses[0].loss, ValueLoss::Decimal(_)));
    match rounded.outcome {
        FamilyOutcome::Evaluated(Outcome::Completed(Value::Quantity(quantity))) => {
            assert_eq!(quantity.magnitude().to_string(), "3/100");
        }
        other => panic!("a rounded carried Decimal conversion, not {other:?}"),
    }
}

/// FR-497-AC-6 (TC-1287): records and collections retain equality when their
/// fields or elements have the same carried quantity type.
#[trace("FR-497-AC-6", "TC-1287")]
#[test]
fn records_and_collections_compare_same_typed_carried_quantities() {
    let package = package();
    assert!(truth(&package, "carried_record_equal"));
    assert!(!truth(&package, "carried_record_unequal"));
    assert!(truth(&package, "carried_collection_equal"));
    assert!(!truth(&package, "carried_collection_unequal"));
}

/// FR-497-AC-2 (TC-1287): a carried value refuses as an arithmetic,
/// negation, equality or ordering operand, and a conversion to an unlike
/// dimension refuses during checking before an evaluator or meter exists.
#[trace("FR-497-AC-2", "TC-1287")]
#[test]
fn a_carried_quantity_is_not_an_arithmetic_operand() {
    let source = format!(
        "{FUNCTIONS}\
         function illegal_add using v(a: Quantity<m, Int[-2, 2]>): Quantity<m, Int[-2, 2]> pure {{ a + a }}\n\
         function illegal_subtract using v(a: Quantity<m, Int[-2, 2]>): Quantity<m, Int[-2, 2]> pure {{ a - a }}\n\
         function illegal_negate using v(a: Quantity<m, Int[-2, 2]>): Quantity<m, Int[-2, 2]> pure {{ -a }}\n\
         function illegal_multiply using v(a: Quantity<m, Int[-2, 2]>): Quantity<m, Int[-2, 2]> pure {{ a * a }}\n\
         function illegal_divide using v(a: Quantity<m, Int[-2, 2]>): Quantity<m, Int[-2, 2]> pure {{ a / a }}\n\
         function illegal_equal using v(a: Quantity<m, Int[-2, 2]>): Boolean pure {{ a = a }}\n\
         function illegal_order using v(a: Quantity<m, Int[-2, 2]>): Boolean pure {{ a < a }}\n\
         function illegal_dimension using v(): Quantity<m, Int[-2, 2]> pure {{ convert<Quantity<m, Int[-2, 2]>>(1 ms) }}\n"
    );
    let refusals = package_with(&source).expect_err("carried arithmetic refuses during checking");
    assert!(
        refusals.iter().any(|refusal| matches!(
            refusal.cause,
            CheckCause::IllTyped(quire_exact::IllTypedCause::TypeMismatch)
        )),
        "the expression is ill typed: {refusals:?}"
    );
}
