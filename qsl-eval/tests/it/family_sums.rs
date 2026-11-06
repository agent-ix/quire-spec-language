// SPDX-License-Identifier: AGPL-3.0-or-later
//! `sum<N>` over every numeric family a `quire.op.collection.sum` identity
//! exists for, from source text to its evaluated value (FR-517-AC-20, FR-145
//! "sum", QSpec value-accounting "Collection sum schedule", TC-1312 row 20).

use ix_trace_rs::trace;
use qsl_eval::value::{CheckedPackageEvaluation, QualifiedName};
use qsl_package::CheckedPackage;
use qsl_semantics::check::{CheckCause, PackageDeclarations};
use qsl_semantics::model::object_environment::ObjectEnvironment;
use qsl_semantics::value::{admit_ieee_profile, DefinitionRole};
use quire_eval::FamilyOutcome;
use quire_exact::{IllTypedCause, Meter, Outcome, Value};
use quire_semantic_value::checking::CheckingLimits;
use quire_semantic_value::loss::{LocatedLoss, ValueLoss};

use super::quantity_literals::{package_with, UNLIMITED};

const UNIT: &str = "dimension Length; unit m : Length = rational(1, 1);\n\
                    type Len = Quantity<m>;\n\
                    function two using v(): Sequence<Len>[0, 4] pure { sequence[2 m, 3 m] }\n\
                    function three using v(): Sequence<Len>[0, 4] pure { sequence[2 m, 3 m, 4 m] }\n\
                    function nothing using v(): Sequence<Len>[0, 4] pure { filter(y in two(): y > 9 m) }\n";

/// The outcome of calling `f` in the package that `functions` declares, with
/// the IEEE profile selected so that a float sum checks.
fn outcome(functions: &str) -> Outcome<Value> {
    evaluation(functions).0
}

/// [`outcome`] and the value losses the evaluation reported.
fn evaluation(functions: &str) -> (Outcome<Value>, Vec<LocatedLoss>) {
    let text = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n{functions}"
    );
    let parsed = qsl_cst::parse(
        qsl_foundation::SourceIdentity::new("a", "u"),
        "unit.native",
        text.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let unit = qsl_forms::build_unit(&parsed).expect("S2 builds the unit");
    let mut declarations = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        "fixture".to_owned(),
        unit,
        Vec::new(),
        Vec::new(),
    )
    .expect("the assembler builds the declarations");
    declarations.ieee_profile =
        Some(admit_ieee_profile(&[DefinitionRole::IeeeProfile.reference()], &[]).unwrap());
    let package = CheckedPackage::link(
        declarations
            .check(CheckingLimits::default())
            .unwrap_or_else(|refusal| panic!("{functions}: {refusal:?}")),
    );
    let evaluation = package
        .call(
            &QualifiedName::unqualified("f").expect("f is an identifier"),
            Vec::new(),
            &ObjectEnvironment::default(),
            &mut Meter::new(UNLIMITED),
        )
        .expect("the call runs");
    let FamilyOutcome::Evaluated(outcome) = evaluation.outcome else {
        panic!("a kernel outcome, not {:?}", evaluation.outcome);
    };
    (outcome, evaluation.losses)
}

/// The checker refuses `functions` for an unproved range (FR-046, FR-100).
fn assert_unproved_range(functions: &str) {
    let refusals = package_with(functions).expect_err("an unproved total is refused");
    assert!(
        refusals
            .iter()
            .any(|refusal| matches!(refusal.cause, CheckCause::Unproved(_))),
        "{refusals:?}"
    );
}

fn truth(functions: &str) -> bool {
    match outcome(functions) {
        Outcome::Completed(Value::Boolean(truth)) => truth,
        other => panic!("a completed Boolean, not {other:?}"),
    }
}

/// FR-517-AC-20: a quantity sum adds in the unit of `N`, an empty one is `0 m`
/// and a summand of another family is a type mismatch at the summand.
#[trace("FR-517-AC-20", "TC-1312")]
#[test]
fn a_quantity_sum_adds_the_summands_in_the_unit_of_n() {
    let check = |expression: &str| {
        truth(&format!(
            "{UNIT}function f using v(): Boolean pure {{ {expression} }}\n"
        ))
    };
    assert!(check("sum<Len>(x in two(): x) = 5 m"));
    assert!(!check("sum<Len>(x in two(): x) = 6 m"));
    assert!(check("sum<Len>(x in three(): x + 1 m) = 12 m"));
    assert!(check("sum<Len>(x in nothing(): x) = 0 m"));

    let refusals = package_with(&format!(
        "{UNIT}function ints using v(): Sequence<Integer>[0, 4] pure {{ sequence[1, 2] }}\nfunction f using v(): Boolean pure {{ sum<Len>(x in ints(): x) = 3 m }}\n"
    ))
    .expect_err("an integer summand is not a quantity of m");
    assert!(
        refusals.iter().any(|refusal| matches!(
            refusal.cause,
            CheckCause::IllTyped(IllTypedCause::TypeMismatch)
        )),
        "{refusals:?}"
    );
}

/// A unit declaring the summand type `S` and the result type `N`, the values
/// `values` as the functions `e0`, `e1`, .. of type `S`, their sequence `xs`
/// and the Boolean `body` as `f`.
fn over(summand: &str, result: &str, values: &[&str], body: &str) -> String {
    let mut text = format!("type S = {summand};\ntype N = {result};\n");
    for (index, value) in values.iter().enumerate() {
        text += &format!("function e{index} using v(): S pure {{ {value} }}\n");
    }
    let elements: Vec<_> = (0..values.len())
        .map(|index| format!("e{index}()"))
        .collect();
    text += &format!(
        "function xs using v(): Sequence<S>[0, 4] pure {{ sequence[{}] }}\n\
         function f using v(): Boolean pure {{ {body} }}\n",
        elements.join(", ")
    );
    text
}

/// FR-517-AC-20: the decimal family adds under the type `N` pins, an exact sum
/// is the exact total, and a total that can leave `N`'s domain is refused.
#[trace("FR-517-AC-20", "TC-1312")]
#[test]
fn a_decimal_sum_is_exact_and_leaves_its_domain_undefined() {
    // Four summands of at most 10.00 total at most 40.00, so `N` is wider.
    let ty = "Decimal[0, 1000; 2, 2; nearest-even]";
    let sum = |total: &str| {
        truth(&over(
            ty,
            "Decimal[0, 4000; 2, 2; nearest-even]",
            &["decimal(150, 2)", "decimal(275, 2)"],
            &format!("sum<N>(x in xs(): x) = {total}"),
        ))
    };
    assert!(sum("decimal(425, 2)"));
    assert!(!sum("decimal(426, 2)"));
    // A total that can leave `N`'s domain is refused when checked (FR-100):
    // it never reaches a linked package's evaluation.
    let text = over(ty, ty, &["decimal(600, 2)", "decimal(600, 2)"], "true").replace(
        "function f using v(): Boolean pure { true }",
        "function f using v(): N pure { sum<N>(x in xs(): x) }",
    );
    assert_unproved_range(&text);
}

/// FR-517-AC-20: the rational family, and the sum of an empty source being the
/// family's zero, which `N` must admit.
#[trace("FR-517-AC-20", "TC-1312")]
#[test]
fn a_rational_sum_adds_exactly_and_an_empty_one_is_zero() {
    let sum = |second: &str| {
        truth(&over(
            "Rational[0, 10; 1, 2]",
            "Rational[-80, 80; 1, 2]",
            &["rational(1, 2)", second],
            "sum<N>(x in xs(): x) = rational(3, 2)",
        ))
    };
    assert!(sum("rational(1, 1)"));
    assert!(!sum("rational(3, 2)"));
    let text = over(
        "Rational[1, 10; 1, 2]",
        "Rational[1, 80; 1, 2]",
        &["rational(2, 1)"],
        "true",
    )
    .replace(
        "function f using v(): Boolean pure { true }",
        "function f using v(): N pure { sum<N>(x in filter(y in xs(): y > rational(3, 1)): x) }",
    );
    assert_unproved_range(&text);
}

/// FR-517-AC-20: the float family adds under the rounding mode `N` pins.
#[trace("FR-517-AC-20", "TC-1312")]
#[test]
fn a_float_sum_adds_under_its_pinned_rounding() {
    let sum = |second: &str| {
        truth(&over(
            "Float64[nearest-even]",
            "Float64[nearest-even]",
            &["float64(bits: 0x3ff8000000000000)", second],
            "quire::value::ieee::bitIdentical(sum<N>(x in xs(): x), \
             float64(bits: 0x400e000000000000))",
        ))
    };
    // 1.5 + 2.25 = 3.75
    assert!(sum("float64(bits: 0x4002000000000000)"));
    // 1.5 + 1.5 = 3.0
    assert!(!sum("float64(bits: 0x3ff8000000000000)"));
}

/// Whether checking `functions` refuses with a type mismatch.
fn is_mismatch(functions: &str) -> bool {
    package_with(functions)
        .expect_err("the sum is refused")
        .iter()
        .any(|refusal| {
            matches!(
                refusal.cause,
                CheckCause::IllTyped(IllTypedCause::TypeMismatch)
            )
        })
}

/// FR-517-AC-20, FR-145: a summand must be of the very float
/// type or unit of the total, so another float width or unit is a type mismatch.
#[trace("FR-517-AC-20", "TC-1312")]
#[test]
fn a_summand_of_another_float_width_or_unit_is_a_type_mismatch() {
    let sum = |summand: &str, result: &str| {
        format!(
            "type S = {summand};\ntype N = {result};\n\
             function f using v(s: Sequence<S>[0, 4]): N pure {{ sum<N>(x in s: x) }}\n"
        )
    };
    assert!(is_mismatch(&sum(
        "Float32[nearest-even]",
        "Float64[nearest-even]"
    )));
    assert!(is_mismatch(&format!(
        "dimension Length; unit m : Length = rational(1, 1); unit km : Length = rational(1000, 1) * m;\n{}",
        sum("Quantity<km>", "Quantity<m>")
    )));
}

/// FR-517-AC-20, QSpec FR-148: an inexact float addition records the IEEE flag
/// set it raised, located at the sum; an exact one records nothing.
#[trace("FR-517-AC-20", "TC-1312")]
#[test]
fn an_inexact_float_sum_records_its_ieee_flags_at_the_sum() {
    let inexact = |second: &str| {
        evaluation(
            &over(
                "Float64[nearest-even]",
                "Float64[nearest-even]",
                &["float64(bits: 0x3ff0000000000000)", second],
                "true",
            )
            .replace(
                "function f using v(): Boolean pure { true }",
                "function f using v(): N pure { sum<N>(x in xs(): x) }",
            ),
        )
        .1
    };
    // 1 + 2^-60 is inexact; 1 + 1 is not.
    let losses = inexact("float64(bits: 0x3c30000000000000)");
    assert!(
        matches!(losses.as_slice(), [loss] if matches!(loss.loss, ValueLoss::IeeeFlags(_))),
        "{losses:?}"
    );
    assert!(inexact("float64(bits: 0x3ff0000000000000)").is_empty());
}
