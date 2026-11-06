// SPDX-License-Identifier: AGPL-3.0-or-later
//! Quantity literals evaluated from source text with their own-unit
//! magnitude (FR-093-AC-33, TC-483).

use ix_trace_rs::trace;
use qsl_eval::value::{CheckedPackageEvaluation, QualifiedName};
use qsl_package::CheckedPackage;
use qsl_semantics::check::PackageDeclarations;
use qsl_semantics::check::{CheckCause, CheckRefusal};
use qsl_semantics::model::accounting::ModelNormalizationLimits;
use qsl_semantics::model::domain_package::{
    DomainPackage, DomainPackageRef, PackageDimensionRecord, PackageUnitRecord,
};
use qsl_semantics::model::intake::SelectedModels;
use qsl_semantics::model::object_environment::ObjectEnvironment;
use quire_eval::FamilyOutcome;
use quire_exact::{Meter, Outcome, ScalarLimits, Value, ValueType};
use quire_semantic_value::checking::CheckingLimits;
use quire_semantic_value::loss::ValueLoss;

pub(super) const UNLIMITED: ScalarLimits = ScalarLimits {
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
    function carried_inexact_outside using v(): Quantity<m, Int[-2, 2]> pure { convert<Quantity<m, Int[-2, 2]>>(3 m + convert<Quantity<m>>(1 inch)) }\n\
    function carried_decimal_outside using v(): Quantity<m, Decimal[-2, 2; 0, 0; exact]> pure { convert<Quantity<m, Decimal[-2, 2; 0, 0; exact]>>(3 m) }\n\
    function carried_back using v(): Quantity<m> pure { convert<Quantity<m>>(convert<Quantity<m, Int[-2, 2]>>(2 m)) }\n\
    function carried_decimal_exact using v(): Quantity<m, Decimal[-100, 100; 2, 2; exact]> pure { convert<Quantity<m, Decimal[-100, 100; 2, 2; exact]>>(0.03 m) }\n\
    function carried_carried_rounded using v(): Quantity<m, Decimal[-100, 100; 1, 1; nearest-even]> pure { convert<Quantity<m, Decimal[-100, 100; 1, 1; nearest-even]>>(carried_decimal_exact()) }\n\
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

pub(super) fn package_with(functions: &str) -> Result<CheckedPackage, Vec<CheckRefusal>> {
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

pub(super) fn truth(package: &CheckedPackage, name: &str) -> bool {
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

/// FR-517-AC-20 (QSL-1692): a `sum` of same-unit quantities retains the unit
/// and adds exact magnitudes.
#[trace("TC-1312", "FR-517-AC-20")]
#[test]
fn a_quantity_sum_evaluates_to_its_magnitude_in_the_same_unit() {
    let package = package_with(
        "dimension Currency;\n\
         unit U : Currency = rational(1, 1);\n\
         type Cash = Quantity<U>;\n\
         function receipts using v(): Sequence<Cash>[2, 2] pure { sequence[2 U, 3 U] }\n\
         function quantity_sum using v(): Boolean pure { \
             sum<Cash>(receipt in receipts(): receipt) = 5 U }",
    )
    .expect("quantity sum checks and lowers");
    assert!(truth(&package, "quantity_sum"));
}

/// FR-517-AC-20 (QSL-1692): every numeric sum identity keeps its value family,
/// including the zero seed for an empty source.
#[trace("TC-1312", "FR-517-AC-20")]
#[test]
fn numeric_sums_keep_their_family_and_empty_identity() {
    let package = package_with(
        "type R = Rational[-100, 100; 1, 1];\n\
         type RP = Rational[-3, 3; 1, 1];\n\
         type D = Decimal[-100, 100; 0, 0; nearest-even];\n\
         type DP = Decimal[-3, 3; 0, 0; nearest-even];\n\
         type F = Float32[nearest-even];\n\
         type G = Float64[nearest-even];\n\
         dimension Currency;\n\
         unit U : Currency = rational(1, 1);\n\
         type Q = Quantity<U>;\n\
         function r_items using v(): Sequence<RP>[0, 2] pure { sequence[rational(2, 1), rational(3, 1)] }\n\
         function d_items using v(): Sequence<DP>[0, 2] pure { sequence[decimal(2, 0), decimal(3, 0)] }\n\
         function f_items using v(): Sequence<F>[0, 2] pure { sequence[float32(bits: 0x40000000), float32(bits: 0x40400000)] }\n\
         function g_items using v(): Sequence<G>[0, 2] pure { sequence[float64(bits: 0x4000000000000000), float64(bits: 0x4008000000000000)] }\n\
         function q_items using v(): Sequence<Q>[0, 2] pure { sequence[2 U, 3 U] }\n\
         function r_empty_items using v(): Sequence<R>[0, 0] pure { sequence[] }\n\
         function d_empty_items using v(): Sequence<D>[0, 0] pure { sequence[] }\n\
         function f_empty_items using v(): Sequence<F>[0, 0] pure { sequence[] }\n\
         function g_empty_items using v(): Sequence<G>[0, 0] pure { sequence[] }\n\
         function q_empty_items using v(): Sequence<Q>[0, 0] pure { sequence[] }\n\
         function rational_sum using v(): Boolean pure { sum<R>(x in r_items(): x) = rational(5, 1) }\n\
         function decimal_sum using v(): Boolean pure { sum<D>(x in d_items(): x) = decimal(5, 0) }\n\
         function float32_sum using v(): Boolean pure { sum<F>(x in f_items(): x) = float32(bits: 0x40a00000) }\n\
         function float64_sum using v(): Boolean pure { sum<G>(x in g_items(): x) = float64(bits: 0x4014000000000000) }\n\
         function quantity_sum using v(): Boolean pure { sum<Q>(x in q_items(): x) = 5 U }\n\
         function rational_empty using v(): Boolean pure { sum<R>(x in r_empty_items(): x) = rational(0, 1) }\n\
         function decimal_empty using v(): Boolean pure { sum<D>(x in d_empty_items(): x) = decimal(0, 0) }\n\
         function float32_empty using v(): Boolean pure { sum<F>(x in f_empty_items(): x) = float32(bits: 0x00000000) }\n\
         function float64_empty using v(): Boolean pure { sum<G>(x in g_empty_items(): x) = float64(bits: 0x0000000000000000) }\n\
         function quantity_empty using v(): Boolean pure { sum<Q>(x in q_empty_items(): x) = 0 U }",
    )
    .expect("all numeric sums check and lower");

    for name in [
        "rational_sum",
        "decimal_sum",
        "float32_sum",
        "float64_sum",
        "quantity_sum",
        "rational_empty",
        "decimal_empty",
        "float32_empty",
        "float64_empty",
        "quantity_empty",
    ] {
        assert!(truth(&package, name), "{name}");
    }
}

fn sum_expression() -> qsl_forms::Expression {
    qsl_forms::Expression::sum(
        qsl_forms::DeclaredName {
            name: "N".into(),
            span: qsl_foundation::Span { start: 0, end: 0 },
        },
        "part",
        qsl_forms::Expression::name("items"),
        qsl_forms::Expression::name("part"),
    )
}

fn sum_source(element: ValueType) -> ValueType {
    ValueType::collection(quire_exact::CollectionType::new(
        quire_exact::CollectionKind::Sequence,
        element,
        Some(quire_exact::CardinalityBound::new(0, 2).unwrap()),
    ))
}

fn run_numeric_sum(
    package: &CheckedPackage,
    source: &ValueType,
    values: Vec<Value>,
) -> (
    quire_eval::Evaluation,
    quire_semantic_value::location::Location,
    quire_semantic_value::location::Location,
) {
    let checked = package
        .graph()
        .check_expression(
            vec![("items".into(), source.clone())],
            &sum_expression(),
            None,
            quire_semantic_value::checking::CheckMode::Kernel,
            CheckingLimits::default(),
        )
        .unwrap();
    let ValueType::Collection(collection_type) = source else {
        panic!("a collection source");
    };
    let supplied =
        quire_exact::form_collection(collection_type, values, &mut Meter::new(UNLIMITED))
            .unwrap()
            .completed()
            .unwrap();
    let evaluation = package
        .evaluate(
            &checked,
            vec![supplied],
            &ObjectEnvironment::default(),
            &mut Meter::new(UNLIMITED),
        )
        .unwrap();
    (
        evaluation,
        checked.root().location().clone(),
        checked.root().children()[1].location().clone(),
    )
}

#[trace("TC-1312", "FR-517-AC-20")]
#[test]
fn numeric_sum_rejects_cross_family_float_width_and_unit_mismatches() {
    for declarations in [
        "type N = Rational[-10,10;1,1]; type S = Decimal[-10,10;0,0;exact];
         function items using v(): Sequence<S>[0,1] pure { sequence[] }
         function total using v(): N pure { sum<N>(part in items(): part) }",
        "type N = Float32[nearest-even]; type S = Float64[nearest-even];
         function items using v(): Sequence<S>[0,1] pure { sequence[] }
         function total using v(): N pure { sum<N>(part in items(): part) }",
        "dimension C; unit U : C = rational(1,1); unit V : C = rational(2,1) * U;
         type N = Quantity<U>; type S = Quantity<V>;
         function items using v(): Sequence<S>[0,1] pure { sequence[] }
         function total using v(): N pure { sum<N>(part in items(): part) }",
    ] {
        let errors = package_with(declarations).expect_err("incompatible summand refuses");
        assert!(
            errors.iter().any(|error| matches!(
                error.cause,
                CheckCause::IllTyped(quire_exact::IllTypedCause::TypeMismatch)
            )),
            "{errors:?}"
        );
    }
}

#[trace("TC-1312", "FR-517-AC-20")]
#[test]
fn rational_and_decimal_sum_domains_refuse_unsafe_linked_prefixes_and_locate_runtime_stops() {
    let interval =
        |low: i64, high: i64| quire_exact::IntegerInterval::new(low.into(), high.into()).unwrap();
    let rational_source = sum_source(ValueType::Rational(
        quire_exact::RationalDomain::new(interval(-10, 10), interval(1, 1)).unwrap(),
    ));
    let decimal_source = sum_source(ValueType::Decimal(
        quire_exact::DecimalType::new(
            (-10_i64).into(),
            10_i64.into(),
            0,
            0,
            quire_exact::RoundingMode::Exact,
        )
        .unwrap(),
    ));
    for (declaration, source, decimal) in [
        ("type N = Rational[1,3;1,1];", rational_source, false),
        ("type N = Decimal[1,3;0,0;exact];", decimal_source, true),
    ] {
        let package = package_with(declaration).unwrap();
        let refused = package
            .graph()
            .check_expression(
                vec![("items".into(), source.clone())],
                &sum_expression(),
                None,
                quire_semantic_value::checking::CheckMode::Linked,
                CheckingLimits::default(),
            )
            .unwrap_err();
        assert!(
            matches!(
                refused.cause,
                CheckCause::Unproved(qsl_semantics::check::Obligation::Range { .. })
            ),
            "{refused:?}"
        );
        for (numbers, seed) in [(vec![5_i64, 0], true), (vec![2, 2], false), (vec![], false)] {
            let values = numbers
                .into_iter()
                .map(|number| {
                    if decimal {
                        Value::Decimal(quire_exact::Decimal::new(number.into(), 0))
                    } else {
                        Value::Rational(quire_exact::Rational::from_integer(number.into()))
                    }
                })
                .collect();
            let (evaluation, sum, summand) = run_numeric_sum(&package, &source, values);
            assert!(
                matches!(
                    evaluation.outcome,
                    FamilyOutcome::Evaluated(Outcome::Undefined(
                        quire_exact::Undefined::SumOutOfDomain
                    ))
                ),
                "{:?}",
                evaluation.outcome
            );
            assert_eq!(
                evaluation.location.as_ref(),
                Some(if seed { &summand } else { &sum })
            );
        }
    }
}

#[trace("TC-1312", "FR-517-AC-20")]
#[test]
fn numeric_sum_records_decimal_rounding_and_ieee_inexact_at_the_sum() {
    let decimal_package = package_with("type N = Decimal[-100,100;1,1;nearest-even];").unwrap();
    let decimal_source = sum_source(ValueType::Decimal(
        quire_exact::DecimalType::new(
            (-100_i64).into(),
            100_i64.into(),
            1,
            2,
            quire_exact::RoundingMode::NearestEven,
        )
        .unwrap(),
    ));
    let (decimal, sum, _) = run_numeric_sum(
        &decimal_package,
        &decimal_source,
        vec![
            Value::Decimal(quire_exact::Decimal::new(1_i64.into(), 1)),
            Value::Decimal(quire_exact::Decimal::new(15_i64.into(), 2)),
        ],
    );
    assert!(
        matches!(decimal.outcome, FamilyOutcome::Evaluated(Outcome::Completed(
        Value::Decimal(ref value)
    )) if value.numerically_equal(&quire_exact::Decimal::new(2_i64.into(), 1))),
        "{:?}",
        decimal.outcome
    );
    assert_eq!(decimal.losses.len(), 1, "{:?}", decimal.losses);
    assert!(matches!(decimal.losses[0].loss, ValueLoss::Decimal(_)));
    assert_eq!(decimal.losses[0].location, sum);

    let float_package = package_with("type N = Float32[nearest-even];").unwrap();
    let float_source = sum_source(ValueType::Float(quire_exact::FloatType::new(
        quire_exact::IeeeWidth::Binary32,
        quire_exact::RoundingMode::NearestEven,
    )));
    let (float, sum, _) = run_numeric_sum(
        &float_package,
        &float_source,
        vec![
            Value::Float(quire_exact::IeeeValue::binary32(0x3f800000)),
            Value::Float(quire_exact::IeeeValue::binary32(0x33800000)),
        ],
    );
    assert!(
        matches!(float.outcome, FamilyOutcome::Evaluated(Outcome::Completed(
        Value::Float(value)
    )) if value == quire_exact::IeeeValue::binary32(0x3f800000)),
        "{:?}",
        float.outcome
    );
    assert_eq!(float.losses.len(), 1, "{:?}", float.losses);
    let ValueLoss::IeeeFlags(flags) = &float.losses[0].loss else {
        panic!("IEEE flags, not {:?}", float.losses);
    };
    assert_eq!(
        flags.iter().collect::<Vec<_>>(),
        [quire_exact::IeeeFlag::Inexact]
    );
    assert_eq!(float.losses[0].location, sum);
}

/// FR-091-AC-72: the checked package-unit conversion is evaluated with the
/// specified argument, `2 tick`, and returns `true`.
#[trace("FR-091-AC-72", "QSpec-FR-142-AC-18", "QSpec-FR-142-AC-19")]
#[test]
fn a_package_typed_unit_conversion_evaluates_true_for_two_ticks() {
    let rate = quire_checked::key::DeclarationKey::fixture("dimension/Rate");
    let us = quire_checked::key::DeclarationKey::fixture("unit/us");
    let mut model = DomainPackage::new(DomainPackageRef::fixture("rpc"), vec![]);
    model.dimensions.push(PackageDimensionRecord {
        key: rate.clone(),
        name: "Rate".to_owned(),
        base: true,
        terms: vec![],
    });
    model.units.push(PackageUnitRecord {
        key: us,
        name: "us".to_owned(),
        dimension: rate,
        target: None,
        scale: "1".to_owned(),
        offset: "0".to_owned(),
    });
    let selected = SelectedModels::fixture(
        "M",
        qsl_foundation::Span { start: 0, end: 1 },
        model,
        ModelNormalizationLimits::UNLIMITED,
    )
    .expect("the package model admits");
    let text = "language \"ix:native\" edition \"1-draft\";\n\
                profile v = \"quire.value.complete/v1\";\n\
                model M = \"test/orders\";\n\
                unit tick : M::Rate = rational(1, 2) * M::us;\n\
                unit tock : M::Rate = rational(3, 1) * tick;\n\
                function k1 using v(d: Quantity<tick>): Boolean pure { \
                    convert<Quantity<M::us>>(d) = 1 M::us }\n\
                function k1_at_two_ticks using v(): Boolean pure { k1(2 tick) }\n\
                function k1_at_three_ticks using v(): Boolean pure { k1(3 tick) }\n\
                function chained using v(): Boolean pure { \
                    convert<Quantity<M::us>>(1 tock) = rational(3, 2) M::us }";
    let parsed = qsl_cst::parse(
        qsl_foundation::SourceIdentity::new("test", "fr-091-ac-72"),
        "fr-091-ac-72.native",
        text.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let built = qsl_forms::build_unit(&parsed).expect("S2 builds the unit");
    let declarations = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        "fixture".to_owned(),
        built,
        selected,
        Vec::new(),
    )
    .expect("the package-unit declarations assemble");
    let package = CheckedPackage::link(
        declarations
            .check(CheckingLimits::default())
            .expect("k1 and its call check"),
    );
    assert!(truth(&package, "k1_at_two_ticks"));
    assert!(!truth(&package, "k1_at_three_ticks"));
    assert!(truth(&package, "chained"));
    // The emit path carries the model-owned unit nodes.
    qsl_package::emit_checked(&package).expect("the package with package-owned units emits");
}

/// FR-497-AC-4 (TC-1287): a source-checked carried conversion returns its
/// exact value with the target carrier admitted as the function result.
#[trace("FR-497-AC-4", "TC-1287", "QSpec-TC-187", "QSpec-FR-142-AC-28")]
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
#[trace(
    "FR-497-AC-4",
    "FR-497-AC-5",
    "FR-497-AC-7",
    "TC-1287",
    "QSpec-TC-187",
    "QSpec-FR-142-AC-28",
    "QSpec-FR-142-AC-29"
)]
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
    match call("carried_inexact_outside").outcome {
        FamilyOutcome::Evaluated(Outcome::Refused(quire_exact::Refusal::InexactConversion {
            target: quire_exact::InexactTarget::Integer(target),
        })) => {
            assert_eq!(target.lower().to_string(), "-2");
            assert_eq!(target.upper().to_string(), "2");
        }
        other => panic!("inexactness precedes out-of-domain refusal, got {other:?}"),
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
    let carried_to_carried = call("carried_carried_rounded");
    assert_eq!(carried_to_carried.losses.len(), 1);
    assert!(matches!(
        carried_to_carried.losses[0].loss,
        ValueLoss::Decimal(_)
    ));
    match carried_to_carried.outcome {
        FamilyOutcome::Evaluated(Outcome::Completed(Value::Quantity(quantity))) => {
            assert_eq!(quantity.magnitude().to_string(), "0/1");
        }
        other => panic!("a rounded carried-to-carried conversion, not {other:?}"),
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
#[trace("FR-497-AC-6", "TC-1287", "QSpec-TC-187", "QSpec-FR-142-AC-30")]
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
#[trace("FR-497-AC-2", "TC-1287", "QSpec-TC-187", "QSpec-FR-142-AC-27")]
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
    let prohibited = [
        "illegal_add",
        "illegal_subtract",
        "illegal_negate",
        "illegal_multiply",
        "illegal_divide",
        "illegal_equal",
        "illegal_order",
        "illegal_dimension",
    ];
    for function in prohibited {
        let matching: Vec<_> = refusals
            .iter()
            .filter(|refusal| {
                matches!(
                    &refusal.location.origin,
                    quire_semantic_value::location::Origin::Body { function: found, .. }
                        if found == function
                )
            })
            .collect();
        assert_eq!(
            matching.len(),
            1,
            "{function} has exactly one expression refusal: {refusals:?}"
        );
        let refusal = matching[0];
        assert!(
            matches!(
                &refusal.cause,
                CheckCause::IllTyped(quire_exact::IllTypedCause::TypeMismatch)
            ),
            "{function} is refused as a type mismatch: {refusal:?}"
        );
        assert!(
            refusal.location.path().is_empty(),
            "{function} is refused at its root expression: {refusal:?}"
        );
    }
    // The checker returns no package for any prohibited expression, so there
    // is no evaluator to call and no runtime meter can be charged.
    assert_eq!(
        refusals.len(),
        prohibited.len(),
        "every invalid operation has one refusal and no extra diagnostics: {refusals:?}"
    );
}

/// FR-497-AC-6 (TC-1287): a carried quantity is not an element of a sum. A
/// `sum` over carried elements refuses during checking as a type mismatch,
/// with no package to evaluate, while the same elements converted back to
/// `Quantity<m>` check.
#[trace("FR-497-AC-6", "TC-1287", "QSpec-TC-187", "QSpec-FR-142-AC-30")]
#[test]
fn a_carried_quantity_is_not_a_summand() {
    let source = format!(
        "{FUNCTIONS}\
         type Total = Integer;\n\
         function illegal_sum using v(xs: Sequence<Quantity<m, Int[-2, 2]>>[0, 4]): Integer pure {{ sum<Total>(x in xs: x) }}\n"
    );
    let refusals = package_with(&source).expect_err("a carried summand refuses during checking");
    let in_sum: Vec<_> = refusals
        .iter()
        .filter(|refusal| {
            matches!(
                &refusal.location.origin,
                quire_semantic_value::location::Origin::Body { function, .. }
                    if function == "illegal_sum"
            )
        })
        .collect();
    assert_eq!(
        in_sum.len(),
        refusals.len(),
        "only the sum refuses: {refusals:?}"
    );
    assert!(
        in_sum.iter().any(|refusal| matches!(
            &refusal.cause,
            CheckCause::IllTyped(quire_exact::IllTypedCause::TypeMismatch)
        )),
        "the carried summand is a type mismatch: {refusals:?}"
    );

    let converted_back = format!(
        "{FUNCTIONS}\
         function back using v(xs: Sequence<Quantity<m, Int[-2, 2]>>[0, 4]): Sequence<Quantity<m>>[0, 4] pure {{ map(x in xs: convert<Quantity<m>>(x)) }}\n"
    );
    package_with(&converted_back).expect("a carried element converted back checks");
}

/// FR-497-AC-2 (TC-1287): exponentiation spelling is refused by the source
/// reader, so a carried quantity can never reach an evaluator as `a ** 2`.
#[trace("FR-497-AC-2", "TC-1287", "QSpec-TC-187", "QSpec-FR-142-AC-27")]
#[test]
fn a_carried_quantity_exponent_expression_is_not_admitted() {
    let source = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n\
         {FUNCTIONS}\
         function illegal_power using v(a: Quantity<m, Int[-2, 2]>): Quantity<m, Int[-2, 2]> pure {{ a ** 2 }}\n"
    );
    let parsed = qsl_cst::parse(
        qsl_foundation::SourceIdentity::new("a", "u"),
        "unit.native",
        source.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    assert!(!parsed.is_admissible(), "the unsupported exponent refuses");
}
