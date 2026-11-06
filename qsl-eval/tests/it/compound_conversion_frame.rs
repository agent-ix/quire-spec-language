// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-142-AC-10 and TC-187 U25 through the public checked-frame evaluator.

use ix_trace_rs::trace;
use qsl_eval::value::evaluate_closed_with_frame;
use qsl_forms::{BuiltinType, ClauseKind, Expression, TypeForm};
use qsl_semantics::check::{CheckMode, CheckedNode};
use qsl_semantics::model::object_environment::ObjectEnvironment;
use quire_checked::node::NodeKind;
use quire_eval::{Evaluation, FamilyOutcome};
use quire_exact::{
    ChargePoint, Meter, Outcome, Quantity, Rational, ScalarLimits, Value, ValueType,
};
use quire_semantic_value::checking::CheckingLimits;

use super::quantity_literals::{package_with, UNLIMITED};

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
    function form_compound using v(n: Quantity<N_m>): Boolean pure {\
        n * (1 kg / 1 kg) = n * (1 kg / 1 kg) }\n";

fn quantity_power(node: CheckedNode<'_>) -> Option<CheckedNode<'_>> {
    let mut pending = vec![node];
    while let Some(node) = pending.pop() {
        if matches!(node.kind(), NodeKind::Quantity(_, _, _)) {
            return Some(node);
        }
        pending.extend(node.children());
    }
    None
}

fn evaluated(evaluation: Evaluation) -> Outcome<Value> {
    match evaluation.outcome {
        FamilyOutcome::Evaluated(outcome) => outcome,
        other => panic!("expected kernel evaluation, got {other:?}"),
    }
}

/// TC-187 U25's compound side survives checking and is available to a later
/// checked conversion. Its bound value is produced by evaluating a checked
/// node in the original frame; the conversion itself spends only the U25
/// conversion charges.
#[trace("QSpec-FR-142-AC-10", "QSpec-TC-187")]
#[test]
fn checked_conversion_reads_compound_source_from_its_retained_frame() {
    let package = package_with(SOURCE).expect("the compound fixture checks");
    let graph = package.graph();
    let function = graph
        .function_state(0)
        .expect("the compound producer exists");
    let NodeKind::Equality(_, _, left, _) = function.body.kind() else {
        panic!("the producer's root is its equality");
    };
    let compound_node = quantity_power(function.body.at(*left))
        .expect("the producer contains the checked compound multiplication");
    let ValueType::Quantity(compound_unit) = compound_node.value_type() else {
        panic!("the compound node has a quantity result");
    };
    let ValueType::Quantity(n_m) = &graph
        .callable("form_compound")
        .expect("the producer is callable")
        .parameters[0]
        .1
    else {
        panic!("the producer receives N_m");
    };
    let mut producer_meter = Meter::new(UNLIMITED);
    let compound = evaluated(
        evaluate_closed_with_frame(
            &package,
            compound_node,
            function.slots,
            vec![Value::Quantity(Quantity::new(
                Rational::from_integer(1.into()),
                n_m.clone(),
            ))],
            function.formed_units,
            &mut producer_meter,
        )
        .expect("the checked producer node evaluates"),
    );
    let Outcome::Completed(Value::Quantity(compound_value)) = compound else {
        panic!("the producer returns the compound quantity: {compound:?}");
    };
    assert_eq!(compound_value.unit(), compound_unit);

    let conversion = graph
        .check_clause_expression(
            vec![("q".to_owned(), ValueType::Quantity(compound_unit.clone()))],
            &Expression::convert(
                TypeForm::builtin(
                    BuiltinType::Quantity,
                    qsl_foundation::Span { start: 0, end: 0 },
                )
                .with_arguments(vec![TypeForm::name(
                    "J",
                    qsl_foundation::Span { start: 0, end: 0 },
                )]),
                Expression::name("q".to_owned()),
            ),
            None,
            ClauseKind::Body,
            CheckMode::Linked,
            CheckingLimits::default(),
        )
        .expect("the retained compound type is available to a checked conversion");

    let mut meter = Meter::new(ScalarLimits {
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
    });
    let result = evaluated(
        evaluate_closed_with_frame(
            &package,
            conversion.root(),
            conversion.slots(),
            vec![Value::Quantity(compound_value)],
            conversion.formed_units(),
            &mut meter,
        )
        .expect("the checked conversion evaluates in its bound frame"),
    );
    let Outcome::Completed(Value::Quantity(converted)) = result else {
        panic!("the conversion returns J: {result:?}");
    };
    assert_eq!(converted.magnitude(), &Rational::from_integer(1.into()));
    assert_eq!(
        meter.admitted_charges(),
        [
            ChargePoint::UnitIdentityRead,
            ChargePoint::UnitTargetDomain,
            ChargePoint::UnitResultRetain,
        ]
    );
}
