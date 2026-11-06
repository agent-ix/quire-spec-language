// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-142-AC-10 and TC-187 U25 through the public checked-frame evaluator.

use ix_trace_rs::trace;
use qsl_eval::value::evaluate_closed_with_frame;
use qsl_forms::{BuiltinType, ClauseKind, Expression, TypeForm};
use qsl_semantics::check::CheckCause;
use quire_checked::node::NodeKind;
use quire_eval::{Evaluation, FamilyOutcome};
use quire_exact::{
    ChargePoint, Integer, Meter, Outcome, Quantity, Rational, ScalarLimits, UnitId, Value,
    ValueType,
};
use quire_semantic_value::checking::{CheckMode, CheckingLimits};

use super::quantity_literals::package_with;

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

const SOURCE: &str = "\
    dimension Time;\n\
    unit s : Time = rational(1, 1);\n\
    dimension Length;\n\
    unit m : Length = rational(1, 1);\n\
    dimension Mass;\n\
    unit kg : Mass = rational(1, 1);\n\
    dimension Torque = Mass * Length ^ 2 / Time ^ 2;\n\
    unit N_m : Torque = rational(1, 1);\n\
    dimension Energy = Mass * Length ^ 2 / Time ^ 2;\n\
    unit J : Energy = rational(1, 1);\n\
    function form_compound using v(n: Quantity<N_m>, metre: Quantity<m>, kilogram: Quantity<kg>, second: Quantity<s>): Boolean pure {\
        1 kg * 1 m * 1 m / 1 s / 1 s = 1 kg * 1 m * 1 m / 1 s / 1 s }\n";

fn evaluated(evaluation: Evaluation) -> Outcome<Value> {
    match evaluation.outcome {
        FamilyOutcome::Evaluated(outcome) => outcome,
        other => panic!("expected kernel evaluation, got {other:?}"),
    }
}

fn convert_to(name: &str) -> Expression {
    Expression::convert(
        TypeForm::builtin(
            BuiltinType::Quantity,
            qsl_foundation::Span { start: 0, end: 0 },
        )
        .with_arguments(vec![TypeForm::name(
            name,
            qsl_foundation::Span { start: 0, end: 0 },
        )]),
        Expression::name("q".to_owned()),
    )
}

/// TC-187 U25's full pivot sequence uses two checker-produced conversion
/// nodes in the original parameter frame. Each leg spends only its own U25
/// conversion charges.
#[trace("QSpec-FR-142-AC-10", "QSpec-TC-187")]
#[test]
fn checked_conversions_run_the_compound_pivot_in_the_retained_frame() {
    let package = package_with(SOURCE).expect("the compound fixture checks");
    let graph = package.graph();
    let function = graph
        .function_state(0)
        .expect("the compound producer exists");
    let NodeKind::Equality(_, _, _, right) = function.body.kind() else {
        panic!("the producer's root is its equality");
    };
    let compound_node = function.body.at(*right);
    let ValueType::Quantity(compound_unit) = compound_node.value_type() else {
        panic!("the compound node has a quantity result");
    };
    let callable = graph
        .callable("form_compound")
        .expect("the producer is callable");
    let ValueType::Quantity(n_m) = &callable.parameters[0].1 else {
        panic!("the producer receives N_m");
    };
    let ValueType::Quantity(metre) = &callable.parameters[1].1 else {
        panic!("the producer also receives metres");
    };
    let ValueType::Quantity(kilogram) = &callable.parameters[2].1 else {
        panic!("the producer also receives kilograms");
    };
    let ValueType::Quantity(second) = &callable.parameters[3].1 else {
        panic!("the producer also receives seconds");
    };
    let UnitId::Compound(compound_terms) = compound_unit else {
        panic!("the pivot target is compound");
    };
    let mut expected_terms = vec![
        (
            kilogram
                .declared_key()
                .expect("kilogram has a declared identity")
                .clone(),
            Integer::one(),
        ),
        (
            metre
                .declared_key()
                .expect("metre has a declared identity")
                .clone(),
            Integer::from(2_i64),
        ),
        (
            second
                .declared_key()
                .expect("second has a declared identity")
                .clone(),
            Integer::from(-2_i64),
        ),
    ];
    expected_terms.sort_by(|left, right| left.0.cmp(&right.0));
    assert_eq!(compound_terms.as_ref(), expected_terms);
    let arguments = || {
        vec![
            Value::Quantity(Quantity::new(
                Rational::from_integer(1_i64.into()),
                n_m.clone(),
            )),
            Value::Quantity(Quantity::new(
                Rational::from_integer(1_i64.into()),
                metre.clone(),
            )),
            Value::Quantity(Quantity::new(
                Rational::from_integer(1_i64.into()),
                kilogram.clone(),
            )),
            Value::Quantity(Quantity::new(
                Rational::from_integer(1_i64.into()),
                second.clone(),
            )),
        ]
    };

    let mut literal_meter = Meter::new(UNLIMITED);
    let literal_result = evaluated(
        evaluate_closed_with_frame(
            &package,
            compound_node,
            function.slots,
            arguments(),
            function.formed_units,
            &mut literal_meter,
        )
        .expect("the checked compound quantity literal evaluates in the genuine function frame"),
    );
    let Outcome::Completed(Value::Quantity(compound_literal)) = literal_result else {
        panic!("the compound quantity literal evaluates: {literal_result:?}");
    };
    assert_eq!(compound_literal.unit(), compound_unit);
    assert_eq!(
        compound_literal.magnitude(),
        &Rational::from_integer(1_i64.into())
    );

    let first_conversion = graph
        .check_quantity_conversion(
            callable.parameters.to_vec(),
            &Expression::name("n".to_owned()),
            compound_unit.clone(),
            ClauseKind::Body,
            CheckMode::Linked,
            CheckingLimits::default(),
        )
        .expect("the retained compound target admits the first U25 conversion");

    let conversion_meter = || {
        Meter::new(ScalarLimits {
            integer_bits: 1,
            decimal_digits: 0,
            scale_expansion: 0,
            text_input_bytes: 0,
            text_scalars: 0,
            normalized_scalars: 0,
            unit_edges: 1,
            value_occurrences: 1,
            work_units: 3,
            result_units: 1,
        })
    };
    let mut first_meter = conversion_meter();
    let first_result = evaluated(
        evaluate_closed_with_frame(
            &package,
            first_conversion.root(),
            first_conversion.slots(),
            arguments(),
            first_conversion.formed_units(),
            &mut first_meter,
        )
        .expect("N_m converts to the checked compound pivot in the bound frame"),
    );
    let Outcome::Completed(Value::Quantity(compound_value)) = first_result else {
        panic!("the first conversion returns the compound pivot: {first_result:?}");
    };
    assert_eq!(compound_value.unit(), compound_unit);
    assert_eq!(
        compound_value.magnitude(),
        &Rational::from_integer(1_i64.into())
    );
    let expected_charges = vec![
        ChargePoint::UnitIdentityRead,
        ChargePoint::UnitTargetDomain,
        ChargePoint::UnitResultRetain,
    ];
    let charge_points = |meter: &Meter| {
        meter
            .admitted_charges()
            .iter()
            .map(|charge| charge.point)
            .collect::<Vec<_>>()
    };
    assert_eq!(charge_points(&first_meter), expected_charges);

    let second_conversion = graph
        .check_clause_expression(
            vec![("q".to_owned(), ValueType::Quantity(compound_unit.clone()))],
            &convert_to("J"),
            None,
            ClauseKind::Body,
            CheckMode::Linked,
            CheckingLimits::default(),
        )
        .expect("the retained compound source admits the second U25 conversion");
    let mut second_meter = conversion_meter();
    let second_result = evaluated(
        evaluate_closed_with_frame(
            &package,
            second_conversion.root(),
            second_conversion.slots(),
            vec![Value::Quantity(compound_value)],
            second_conversion.formed_units(),
            &mut second_meter,
        )
        .expect("the compound pivot converts to J in its bound frame"),
    );
    let Outcome::Completed(Value::Quantity(joule)) = second_result else {
        panic!("the second conversion returns J: {second_result:?}");
    };
    let ValueType::Quantity(joule_unit) = second_conversion.value_type() else {
        panic!("the checked second conversion targets a quantity");
    };
    assert_eq!(joule.unit(), joule_unit);
    assert_eq!(joule.magnitude(), &Rational::from_integer(1_i64.into()));
    assert_eq!(charge_points(&second_meter), expected_charges);

    let unknown_compound = UnitId::compound(vec![(
        crate::support::ids::node_key([0xAB]),
        Integer::one(),
    )]);
    let unknown = graph
        .check_quantity_conversion(
            callable.parameters.to_vec(),
            &Expression::name("n".to_owned()),
            unknown_compound,
            ClauseKind::Body,
            CheckMode::Linked,
            CheckingLimits::default(),
        )
        .expect_err("an unretained compound target refuses during checking");
    assert!(matches!(unknown.cause, CheckCause::IllTyped(_)));

    let mismatch = graph
        .check_quantity_conversion(
            callable.parameters.to_vec(),
            &Expression::name("n".to_owned()),
            metre.clone(),
            ClauseKind::Body,
            CheckMode::Linked,
            CheckingLimits::default(),
        )
        .expect_err("an N_m quantity cannot convert to Length");
    assert!(
        matches!(mismatch.cause, CheckCause::IllTyped(_)),
        "incompatible Energy-to-Length target has a typed refusal: {mismatch:?}"
    );
}
