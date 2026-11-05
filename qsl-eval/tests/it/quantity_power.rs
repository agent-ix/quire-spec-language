// SPDX-License-Identifier: AGPL-3.0-or-later
//! Quantity integer power, `e ^ n`, from source text to its evaluated value
//! (FR-091 "Integer power spelling", FR-093 "Integer power", FR-093-AC-58,
//! TC-1336; QSpec TC-187 U12, U13, U21).

use ix_trace_rs::trace;
use qsl_eval::value::{CheckedPackageEvaluation, QualifiedName};
use qsl_semantics::check::{CheckCause, CheckRefusal, Obligation};
use qsl_semantics::model::object_environment::ObjectEnvironment;
use quire_eval::FamilyOutcome;
use quire_exact::{
    ChargePoint, IllTypedCause, Incomplete, Integer, LimitKind, Meter, Outcome, ScalarLimits,
    Undefined,
};
use quire_semantic_value::checking::{CheckMode, CheckingLimits};
use quire_semantic_value::location::Location;

use super::quantity_literals::{package_with, truth, UNLIMITED};

/// FR-091-AC-44's units, `Length` and its square, and a temperature scale
/// with an affine unit.
const UNITS: &str = "\
    dimension Time;\n\
    unit s : Time = rational(1, 1);\n\
    unit ms : Time = rational(1, 1000) * s;\n\
    dimension Length;\n\
    unit m : Length = rational(1, 1);\n\
    dimension Area = Length ^ 2;\n\
    unit m2 : Area = rational(1, 1);\n\
    dimension Temperature;\n\
    unit kelvin : Temperature = rational(1, 1);\n\
    unit degC : Temperature = rational(1, 1) * kelvin + decimal(27315, 2);\n";

fn units_and(functions: &str) -> String {
    format!("{UNITS}{functions}")
}

/// The one cause the `functions` unit refuses with.
fn refusal(functions: &str) -> CheckRefusal {
    let mut refusals = package_with(&units_and(functions)).expect_err("the unit refuses");
    assert_eq!(refusals.len(), 1, "{refusals:?}");
    refusals.remove(0)
}

/// The refusal's cause when it is `ill_typed`, with the child path it was
/// issued at from the root of its function body (`Equal` is the root, so `[0]`
/// is the left operand and `[0, 0]` its own operand).
fn ill_typed(functions: &str, path: &[usize]) -> IllTypedCause {
    let refusal = refusal(functions);
    assert_at(&refusal, path);
    match refusal.cause {
        CheckCause::IllTyped(cause) => cause,
        other => panic!("an ill_typed refusal, not {other:?}"),
    }
}

fn assert_at(refusal: &CheckRefusal, path: &[usize]) {
    assert_eq!(
        refusal.location,
        Location::at(refusal.location.origin.clone(), path),
        "{refusal:?}"
    );
}

/// FR-093-AC-58 w1 to w6 (TC-1336; QSpec TC-187 U12 and U21): the square, a
/// conversion of a square, the zeroth power of a nonzero and a zero base, the
/// unit-changing first power, and a negative power of a literal base.
#[trace("FR-093-AC-58", "TC-1336", "QSpec-TC-187")]
#[test]
fn an_integer_power_evaluates_to_the_repeated_product_in_its_compound_unit() {
    let package = package_with(&units_and(
        "function w1 using v(): Boolean pure { 2 m ^ 2 = 2 m * 2 m }\n\
         function w1_value using v(): Boolean pure { convert<Quantity<m2>>(2 m ^ 2) = 4 m2 }\n\
         function w1_wrong using v(): Boolean pure { convert<Quantity<m2>>(2 m ^ 2) = 2 m2 }\n\
         function w2 using v(): Boolean pure { convert<Quantity<m2>>(3 m ^ 2) = 9 m2 }\n\
         function w3 using v(): Boolean pure { let d = 5 ms in d ^ 0 = 1 m / 1 m }\n\
         function w3_zero using v(): Boolean pure { let d = 0 ms in d ^ 0 = 1 m / 1 m }\n\
         function w4 using v(): Boolean pure { 0 m ^ 0 = 1 m / 1 m }\n\
         function w5 using v(): Boolean pure { let d = 1500 ms in d ^ 1 = convert<Quantity<s>>(d) * (1 s / 1 s) }\n\
         function w5_scale using v(): Boolean pure { let d = 1500 ms in d ^ 1 = d * (1 s / 1 s) }\n\
         function w6 using v(): Boolean pure { 2 m ^ -1 = (1 m / 2 m) / 1 m }\n\
         function w6_value using v(): Boolean pure { 2 m ^ -2 = ((1 m / 4 m) / 1 m) / 1 m }\n\
         function w9 using v(): Boolean pure { 2 ms ^ 3 = 2 ms * 2 ms * 2 ms }\n",
    ))
    .expect("every power checks");
    for name in [
        "w1", "w1_value", "w2", "w3", "w3_zero", "w4", "w5", "w6", "w9",
    ] {
        assert!(truth(&package, name), "{name}");
    }
    assert!(!truth(&package, "w1_wrong"));
}

/// FR-093-AC-58 w8 (TC-1336; QSpec TC-187 U13): an exponent above `u64::MAX`
/// is an exact integer; the package checks, and a call under U13's budget is
/// `incomplete` at the first rational charge before the power is computed.
#[trace("FR-093-AC-58", "TC-1336", "QSpec-TC-187")]
#[test]
fn an_exponent_above_u64_max_checks_and_is_incomplete_before_it_is_computed() {
    let package = package_with(&units_and(
        "function w8 using v(): Boolean pure { 2 m ^ 18446744073709551616 = 2 m ^ 18446744073709551616 }\n",
    ))
    .expect("the 2^64 exponent checks");
    let limits = ScalarLimits {
        integer_bits: 64,
        decimal_digits: 0,
        scale_expansion: 0,
        text_input_bytes: 0,
        text_scalars: 0,
        normalized_scalars: 0,
        unit_edges: 0,
        value_occurrences: 1,
        work_units: 5,
        result_units: 1,
    };
    let evaluation = package
        .call(
            &QualifiedName::unqualified("w8").expect("an identifier"),
            vec![],
            &ObjectEnvironment::default(),
            &mut Meter::new(limits),
        )
        .expect("the call runs");
    let next: Integer = "36893488147419103232".parse().unwrap();
    let FamilyOutcome::Evaluated(Outcome::Incomplete(incomplete)) = evaluation.outcome else {
        panic!("an incomplete outcome, not {:?}", evaluation.outcome);
    };
    assert_eq!(
        incomplete,
        Incomplete {
            limit_kind: LimitKind::IntegerBits,
            limit: 64,
            consumed: 2,
            next_charge: next,
            charge_point: ChargePoint::UnitRationalArithmetic,
        }
    );
}

/// FR-093-AC-58 r1 and r2 under `CheckMode::Linked`, and the literal bases
/// that discharge the obligation: a negative exponent over anything but a
/// nonzero quantity literal is `unproved-nonzero` at the power.
#[trace("FR-093-AC-58", "TC-1336", "FR-495-AC-3")]
#[test]
fn a_negative_exponent_over_a_base_not_proved_nonzero_refuses() {
    for (functions, path) in [
        (
            "function r1 using v(d: Quantity<m>): Boolean pure { d ^ -1 = d ^ -1 }",
            &[0][..],
        ),
        (
            "function r2 using v(): Boolean pure { 0 m ^ -1 = 0 m ^ -1 }",
            &[0][..],
        ),
        (
            "function r1b using v(): Boolean pure { let d = 2 m in d ^ -1 = d ^ -1 }",
            &[1, 0][..],
        ),
    ] {
        let refusal = refusal(functions);
        assert!(
            matches!(refusal.cause, CheckCause::Unproved(Obligation::Nonzero)),
            "{functions}: {refusal:?}"
        );
        // At the first `^`: the left operand of the comparison, under the
        // `let` body in `r1b`.
        assert_at(&refusal, path);
    }
    // A zero or positive exponent carries no obligation, whatever the base.
    package_with(&units_and(
        "function p1 using v(d: Quantity<m>): Boolean pure { d ^ 0 = d ^ 0 }\n\
         function p2 using v(d: Quantity<m>): Boolean pure { d ^ 2 = d ^ 2 }\n\
         function p3 using v(): Boolean pure { 0 m ^ 3 = 0 m * 0 m * 0 m }\n",
    ))
    .expect("a nonnegative exponent carries no obligation");
}

/// FR-093-AC-58 r2 under `CheckMode::Kernel` (QSpec TC-187 U21, FR-495): the
/// checker records no nonzero obligation, and the first power of a zero base
/// under a negative exponent is `Undefined::DivisionByZero` after one
/// `unit.identity-read`; with no budget for that read it is `incomplete`.
#[trace("FR-093-AC-58", "TC-1336", "FR-495-AC-3", "QSpec-TC-187")]
#[test]
fn kernel_mode_evaluates_a_zero_base_under_a_negative_exponent_to_division_by_zero() {
    let functions = units_and("function r2 using v(): Boolean pure { 0 m ^ -1 = 0 m ^ -1 }\n");
    let package = package_with(UNITS).expect("the units check");
    let body = r2_body(&functions);
    let checked = package
        .graph()
        .check_expression(
            Vec::new(),
            &body,
            None,
            CheckMode::Kernel,
            CheckingLimits::default(),
        )
        .expect("Kernel mode records no nonzero obligation");
    let run = |limits: ScalarLimits| {
        let mut meter = Meter::new(limits);
        let evaluation = package
            .evaluate(
                &checked,
                Vec::new(),
                &ObjectEnvironment::default(),
                &mut meter,
            )
            .expect("the evaluation runs");
        match evaluation.outcome {
            FamilyOutcome::Evaluated(outcome) => outcome,
            other => panic!("an evaluated outcome, not {other:?}"),
        }
    };
    assert!(matches!(
        run(UNLIMITED),
        Outcome::Undefined(Undefined::DivisionByZero)
    ));
    let budget = |work_units| ScalarLimits {
        integer_bits: 1,
        decimal_digits: 0,
        scale_expansion: 0,
        text_input_bytes: 0,
        text_scalars: 0,
        normalized_scalars: 0,
        unit_edges: 0,
        value_occurrences: 1,
        work_units,
        result_units: 0,
    };
    assert!(matches!(
        run(budget(1)),
        Outcome::Undefined(Undefined::DivisionByZero)
    ));
    let Outcome::Incomplete(incomplete) = run(budget(0)) else {
        panic!("an incomplete outcome");
    };
    assert_eq!(
        incomplete,
        Incomplete {
            limit_kind: LimitKind::WorkUnits,
            limit: 0,
            consumed: 0,
            next_charge: Integer::from(1_i64),
            charge_point: ChargePoint::UnitIdentityRead,
        }
    );
}

/// The checked-body expression of function `r2` in `functions`, read through
/// S1 and S2 (the unit is not checked: Linked mode would refuse it).
fn r2_body(functions: &str) -> qsl_forms::Expression {
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
    unit.forms()
        .iter()
        .find_map(|form| match form.form() {
            qsl_forms::DeclarationForm::Function(function) if function.name == "r2" => {
                Some(function.body.clone())
            }
            _ => None,
        })
        .expect("the r2 function")
}

/// FR-093-AC-58 r3 to r5 (TC-1336): an affine base is
/// `ill_typed`/`operator-ineligible` at the power; a non-quantity base and a
/// carried quantity base are `type-mismatch`, the first at the operand and
/// the second at the power.
#[trace("FR-093-AC-58", "TC-1336")]
#[test]
fn a_power_over_an_affine_non_quantity_or_carried_operand_refuses() {
    assert_eq!(
        ill_typed(
            "function r3 using v(t: Quantity<degC>): Boolean pure { t ^ 2 = t ^ 2 }",
            &[0]
        ),
        IllTypedCause::OperatorIneligible
    );
    assert_eq!(
        ill_typed(
            "function r3b using v(): Boolean pure { 5 degC ^ 2 = 5 degC ^ 2 }",
            &[0]
        ),
        IllTypedCause::OperatorIneligible
    );
    assert_eq!(
        ill_typed(
            "function r4 using v(i: Int[0, 9]): Boolean pure { i ^ 2 = i }",
            &[0, 0]
        ),
        IllTypedCause::TypeMismatch
    );
    assert_eq!(
        ill_typed(
            "function r5 using v(c: Quantity<m, Int[0, 9]>): Boolean pure { c ^ 2 = c ^ 2 }",
            &[0]
        ),
        IllTypedCause::TypeMismatch
    );
    assert_eq!(
        ill_typed(
            "function r6 using v(): Boolean pure { true ^ 2 = true }",
            &[0, 0]
        ),
        IllTypedCause::TypeMismatch
    );
}

/// FR-091-OQ-15: an exponent literal outside `i128` refuses
/// `IntegerOutsideI128`; the largest and smallest `i128` are admitted.
#[trace("FR-093-AC-58", "TC-1336", "FR-091-OQ-15")]
#[test]
fn an_exponent_outside_i128_refuses_and_the_i128_extremes_check() {
    let refusal = refusal(
        "function big using v(): Boolean pure { 2 m ^ 170141183460469231731687303715884105728 = 2 m ^ 0 }",
    );
    assert!(
        matches!(refusal.cause, CheckCause::IntegerOutsideI128(_)),
        "{refusal:?}"
    );
    assert_at(&refusal, &[0]);
    package_with(&units_and(
        "function top using v(): Boolean pure { 1 m ^ 170141183460469231731687303715884105727 = 1 m ^ 170141183460469231731687303715884105727 }\n\
         function bottom using v(): Boolean pure { 1 m ^ -170141183460469231731687303715884105728 = 1 m ^ -170141183460469231731687303715884105728 }\n",
    ))
    .expect("both i128 extremes check");
}
