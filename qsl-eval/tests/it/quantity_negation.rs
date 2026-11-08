// SPDX-License-Identifier: AGPL-3.0-or-later
//! Quantity negation, `-e`, from source text to its evaluated value (FR-093
//! "Negation", FR-093-AC-49, TC-1285; QSpec TC-187 U39, FR-142-AC-25,
//! FR-146-AC-9).

use ix_trace_rs::trace;
use qsl_eval::value::{CheckedPackageEvaluation, QualifiedName};
use qsl_package::CheckedPackage;
use qsl_semantics::check::{CheckCause, CheckRefusal, Obligation};
use qsl_semantics::model::object_environment::ObjectEnvironment;
use quire_eval::FamilyOutcome;
use quire_exact::{
    IllTypedCause, Integer, Meter, Outcome, Quantity as ExactQuantity, Rational, Value, ValueType,
};
use quire_semantic_value::location::ExpressionLocation as Location;

use super::quantity_literals::{package_with, truth, UNLIMITED};

/// FR-091-AC-44's units: `Time`, `Length` and a temperature scale with an
/// affine unit.
const UNITS: &str = "\
    dimension Time;\n\
    unit s : Time = rational(1, 1);\n\
    unit ms : Time = rational(1, 1000) * s;\n\
    dimension Length;\n\
    unit m : Length = rational(1, 1);\n\
    dimension Temperature;\n\
    unit kelvin : Temperature = rational(1, 1);\n\
    unit degC : Temperature = rational(1, 1) * kelvin + decimal(27315, 2);\n";

fn units_and(functions: &str) -> String {
    format!("{UNITS}{functions}")
}

/// The one refusal `functions` check with. The unit is refused whole, so no
/// package, checked graph or node exists, and checking runs under no charge.
fn refusal(functions: &str) -> CheckRefusal {
    let mut refusals = package_with(&units_and(functions)).expect_err("the unit refuses");
    assert_eq!(refusals.len(), 1, "{refusals:?}");
    refusals.remove(0)
}

fn assert_at(refusal: &CheckRefusal, path: &[usize]) {
    assert_eq!(
        refusal.location,
        Location::at(refusal.location.origin.clone(), path),
        "{refusal:?}"
    );
}

/// The quantity function `name` evaluates to.
fn quantity(package: &CheckedPackage, name: &str) -> ExactQuantity {
    let evaluation = package
        .call(
            &QualifiedName::unqualified(name).expect("an identifier"),
            vec![],
            &ObjectEnvironment::default(),
            &mut Meter::new(UNLIMITED),
        )
        .unwrap_or_else(|error| panic!("the {name} call runs: {error:?}"));
    match evaluation.outcome {
        FamilyOutcome::Evaluated(Outcome::Completed(Value::Quantity(quantity))) => quantity,
        other => panic!("a completed quantity, not {other:?}"),
    }
}

/// The Boolean function `name` over `d: Quantity<ms>` = `magnitude` ms.
fn truth_at(package: &CheckedPackage, name: &str, magnitude: i64) -> bool {
    let callable = package.graph().callable(name).expect("a callable");
    let ValueType::Quantity(unit) = &callable.parameters[0].1 else {
        panic!("{name} takes a quantity");
    };
    let evaluation = package
        .call(
            &QualifiedName::unqualified(name).expect("an identifier"),
            vec![Value::Quantity(ExactQuantity::new(
                integer(magnitude),
                unit.clone(),
            ))],
            &ObjectEnvironment::default(),
            &mut Meter::new(UNLIMITED),
        )
        .unwrap_or_else(|error| panic!("the {name} call runs: {error:?}"));
    match evaluation.outcome {
        FamilyOutcome::Evaluated(Outcome::Completed(Value::Boolean(truth))) => truth,
        other => panic!("a completed Boolean, not {other:?}"),
    }
}

fn integer(value: i64) -> Rational {
    Rational::from_integer(Integer::from(value))
}

/// FR-093-AC-49 a, b, c, c2, z, q, r, k, k2 and v2 (TC-1285 step 1): each
/// checks and evaluates to the exact negation in the operand's own unit.
#[trace("FR-093-AC-49", "TC-1285", "FR-146-AC-9")]
#[test]
fn a_quantity_negation_evaluates_to_the_exact_negation_in_its_own_unit() {
    let package = package_with(&units_and(
        "function a using v(): Quantity<m> pure { -(2 m) }\n\
         function b using v(): Quantity<m> pure { -(2 m) + 1 m }\n\
         function c using v(): Quantity<ms> pure { -(3 ms) }\n\
         function c2 using v(): Quantity<ms> pure { -3 ms }\n\
         function z using v(): Quantity<m> pure { -(0 m) }\n\
         function q using v(): Boolean pure { -(-(3 ms)) = 3 ms }\n\
         function r using v(d: Quantity<ms>): Boolean pure { d = -3 ms }\n\
         function k using v(d: Quantity<ms>): Boolean pure { d / -3 ms = -(d / 3 ms) }\n\
         function k_value using v(d: Quantity<ms>): Boolean pure { d / -3 ms = (0 ms - 2 ms) / 1 ms }\n\
         function k2 using v(d: Quantity<ms>): Boolean pure { d / -(-(3 ms)) = d / 3 ms }\n\
         function k2_value using v(d: Quantity<ms>): Boolean pure { d / -(-(3 ms)) = (2 ms) / 1 ms }\n\
         function v2 using v(): Boolean pure { -(3 m / 1 s) = -(3 m / 1 s) }\n\
         function v2_value using v(): Boolean pure { -(3 m / 1 s) = (0 m - 3 m) / 1 s }\n",
    ))
    .expect("every negation checks");
    assert_eq!(quantity(&package, "a").magnitude(), &integer(-2));
    assert_eq!(quantity(&package, "b").magnitude(), &integer(-1));
    // In `ms`, not in the root unit.
    assert_eq!(quantity(&package, "c").magnitude(), &integer(-3));
    assert_eq!(quantity(&package, "c2"), quantity(&package, "c"));
    assert_eq!(quantity(&package, "z").magnitude(), &integer(0));
    for name in ["q", "v2", "v2_value"] {
        assert!(truth(&package, name), "{name}");
    }
    // The AC's parameter form, at d = 6 ms (d = -3 ms and 3 ms for r).
    for name in ["k", "k_value", "k2", "k2_value"] {
        assert!(truth_at(&package, name, 6), "{name}");
    }
    assert!(truth_at(&package, "r", -3));
    assert!(!truth_at(&package, "r", 3));
}

/// FR-093-AC-49 n, n2 and b2 (TC-1285 step 2): an affine unit is
/// `ill_typed`/`operator-ineligible`, a non-quantity operand
/// `ill_typed`/`type-mismatch`, and a bounded or carried quantity keeps its
/// refusal (FR-497-AC-2).
#[trace("FR-093-AC-49", "TC-1285", "FR-497-AC-2")]
#[test]
fn a_negation_of_an_affine_or_non_quantity_operand_refuses_without_a_node() {
    for (functions, cause) in [
        (
            "function n using v(t: Quantity<degC>): Quantity<degC> pure { -t }",
            IllTypedCause::OperatorIneligible,
        ),
        (
            "function n2 using v(): Quantity<degC> pure { -(5 degC) }",
            IllTypedCause::OperatorIneligible,
        ),
        (
            "function b2 using v(b: Boolean): Boolean pure { -b }",
            IllTypedCause::TypeMismatch,
        ),
        (
            "function c1 using v(c: Quantity<m, Int[-2, 2]>): Boolean pure { -c = -c }",
            IllTypedCause::TypeMismatch,
        ),
    ] {
        let refusal = refusal(functions);
        assert!(
            matches!(&refusal.cause, CheckCause::IllTyped(found) if *found == cause),
            "{functions}: {refusal:?}"
        );
    }
    assert_at(
        &refusal("function n2 using v(): Quantity<degC> pure { -(5 degC) }"),
        &[],
    );
    // A type mismatch is reported at the operand.
    assert_at(
        &refusal("function b2 using v(b: Boolean): Boolean pure { -b }"),
        &[0],
    );
}

/// FR-093-AC-49 k3 and k4 (TC-1285 step 2; FR-146-AC-9): a negated zero
/// literal and a negated parameter are no nonzero fact.
#[trace("FR-093-AC-49", "TC-1285", "FR-146-AC-9")]
#[test]
fn a_division_by_a_negation_not_rooted_in_a_nonzero_literal_refuses() {
    for functions in [
        "function k3 using v(d: Quantity<ms>): Boolean pure { d / -(0 ms) = d / 3 ms }",
        "function k4 using v(d: Quantity<ms>, e: Quantity<ms>): Boolean pure { d / -e = d / 3 ms }",
    ] {
        let refusal = refusal(functions);
        assert!(
            matches!(refusal.cause, CheckCause::Unproved(Obligation::Nonzero)),
            "{functions}: {refusal:?}"
        );
        assert_at(&refusal, &[0]);
    }
}
