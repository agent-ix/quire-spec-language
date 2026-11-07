// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-046-AC-3 (TC-127): sequence queries over the items `[2, 2, 3]` keep
//! duplicates and source order, and `size`, `count` and `sum` read them, on
//! the checked route: source text through S1, S2, the assembler and the
//! checker, then the evaluator.

use ix_trace_rs::trace;
use qsl_eval::value::{CheckedPackageEvaluation, QualifiedName};
use qsl_package::CheckedPackage;
use qsl_semantics::check::PackageDeclarations;
use qsl_semantics::model::object_environment::ObjectEnvironment;
use quire_eval::FamilyOutcome;
use quire_exact::{Integer, Meter, Outcome, ScalarLimits, Value};
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

/// Independently authored domains: `Amount` items 1 to 20, a sequence of
/// at most five of them, a `Total` of 0 to 100 and a `Count` of 0 to 5.
const UNIT: &str = "type Amount = Int[1, 20];\n\
    type Total = Int[0, 100];\n\
    type Count = Int[0, 5];\n\
    function amounts using v(): Sequence<Amount>[0, 5] pure { sequence[2, 2, 3] }\n\
    function twos using v(): Sequence<Amount>[0, 5] pure \
        { filter(x in amounts(): x = 2) }\n\
    function flags using v(): Sequence<Boolean>[0, 5] pure \
        { map(x in amounts(): x = 2) }\n\
    function length using v(): Count pure { size(amounts()) }\n\
    function twos_counted using v(): Count pure { count<Count>(x in amounts(): x = 2) }\n\
    function summed using v(): Total pure { sum<Total>(x in amounts(): x) }\n";

fn package() -> CheckedPackage {
    package_of(UNIT)
}

fn package_of(declarations: &str) -> CheckedPackage {
    let text = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n{declarations}"
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

fn call(package: &CheckedPackage, name: &str) -> Value {
    let evaluation = package
        .call(
            &QualifiedName::unqualified(name).expect("an identifier"),
            Vec::new(),
            &ObjectEnvironment::default(),
            &mut Meter::new(UNLIMITED),
        )
        .unwrap_or_else(|error| panic!("the {name} call runs: {error:?}"));
    match evaluation.outcome {
        FamilyOutcome::Evaluated(Outcome::Completed(value)) => value,
        other => panic!("{name}: a completed value, not {other:?}"),
    }
}

fn integers(value: &Value) -> Vec<Integer> {
    let Value::Collection(collection) = value else {
        panic!("a collection, not {value:?}");
    };
    collection
        .elements()
        .iter()
        .map(|element| match element {
            Value::Integer(integer) => integer.clone(),
            other => panic!("an integer, not {other:?}"),
        })
        .collect()
}

fn booleans(value: &Value) -> Vec<bool> {
    let Value::Collection(collection) = value else {
        panic!("a collection, not {value:?}");
    };
    collection
        .elements()
        .iter()
        .map(|element| match element {
            Value::Boolean(truth) => *truth,
            other => panic!("a Boolean, not {other:?}"),
        })
        .collect()
}

fn integer_of(value: &Value) -> Integer {
    match value {
        Value::Integer(integer) => integer.clone(),
        other => panic!("an integer, not {other:?}"),
    }
}

/// FR-046-AC-3 (TC-127), the part that runs on the checked route: over the
/// items `[2, 2, 3]`, `filter` and `map` keep the duplicate and the source
/// order, and size is 3, the count of 2 is 2 and the sum is 7, evaluated from
/// source text through the checker and `CheckedPackage::link`. The nominal
/// unit `U` is [`duplicate_items_of_a_nominal_unit_survive_the_queries`].
#[trace("FR-046-AC-3", "TC-127")]
#[test]
fn duplicate_items_survive_the_queries_and_size_count_and_sum_read_them() {
    let package = package();
    let amounts: Vec<Integer> = [2_i64, 2, 3].map(Integer::from).to_vec();
    assert_eq!(integers(&call(&package, "amounts")), amounts);
    assert_eq!(
        integers(&call(&package, "twos")),
        [2_i64, 2].map(Integer::from).to_vec(),
        "filter keeps both 2s"
    );
    assert_eq!(
        booleans(&call(&package, "flags")),
        [true, true, false],
        "map keeps one result per item, in order"
    );
    assert_eq!(integer_of(&call(&package, "length")), Integer::from(3_i64));
    assert_eq!(
        integer_of(&call(&package, "twos_counted")),
        Integer::from(2_i64)
    );
    assert_eq!(integer_of(&call(&package, "summed")), Integer::from(7_i64));
}

/// The same domains over the nominal unit `U`: `Amount` is `Quantity<U, Int[1, 20]>`,
/// `Total` is `Quantity<U, Int[0, 100]>` and `Count` stays `Int[0, 5]`. A carried
/// quantity is not summable, so the sum reads each item back as `Quantity<U>`
/// (FR-497 "Type identity").
const UNIT_U: &str = "dimension D;\n\
    unit U : D = rational(1, 1);\n\
    type Amount = Quantity<U, Int[1, 20]>;\n\
    type Total = Quantity<U, Int[0, 100]>;\n\
    type Plain = Quantity<U>;\n\
    type Count = Int[0, 5];\n\
    function two using v(): Amount pure { convert<Amount>(2 U) }\n\
    function three using v(): Amount pure { convert<Amount>(3 U) }\n\
    function amounts using v(): Sequence<Amount>[0, 5] pure { sequence[two(), two(), three()] }\n\
    function plain using v(): Sequence<Plain>[0, 5] pure \
        { map(x in amounts(): convert<Plain>(x)) }\n\
    function twos using v(): Sequence<Plain>[0, 5] pure \
        { filter(x in plain(): x = 2 U) }\n\
    function flags using v(): Sequence<Boolean>[0, 5] pure \
        { map(x in plain(): x = 2 U) }\n\
    function length using v(): Count pure { size(amounts()) }\n\
    function twos_counted using v(): Count pure { count<Count>(x in plain(): x = 2 U) }\n\
    function summed using v(): Total pure { convert<Total>(sum<Plain>(x in plain(): x)) }\n";

/// The `(numerator, denominator)` of each quantity of a collection and the
/// unit each carries.
fn quantities(value: &Value) -> Vec<((Integer, Integer), quire_exact::UnitId)> {
    let Value::Collection(collection) = value else {
        panic!("a collection, not {value:?}");
    };
    collection
        .elements()
        .iter()
        .map(|element| match element {
            Value::Quantity(quantity) => {
                let magnitude = quantity.magnitude();
                (
                    (
                        magnitude.numerator().clone(),
                        magnitude.denominator().clone(),
                    ),
                    quantity.unit().clone(),
                )
            }
            other => panic!("a quantity, not {other:?}"),
        })
        .collect()
}

/// FR-046-AC-3 (TC-127), the nominal-unit half: over `Amount` 1 to 20 of the
/// unit `U`, N = 5, `Total` 0 to 100 of `U` and `Count` 0 to 5, `[2 U, 2 U, 3 U]`
/// keeps the duplicate and the order through `filter` and `map`, and size is 3,
/// the count of `2 U` is 2 and the sum is `7 U`. The expected values are
/// literals, and every quantity carries the one unit `U`.
///
/// The admitted-artifact leg is not here: FR-531's capability inventory lists
/// `call` by `QualifiedName` as unmet parity on the bytes-only route (the
/// package holds no source declaration name), so a function's value cannot be
/// read from the admitted artifact; only a protocol slot or temporal evidence
/// can, and no query-bearing slot fixture exists yet.
#[trace("TC-127")]
#[test]
fn duplicate_items_of_a_nominal_unit_survive_the_queries() {
    let package = package_of(UNIT_U);
    let amounts = call(&package, "amounts");
    let Value::Collection(collection) = &amounts else {
        panic!("a collection, not {amounts:?}");
    };
    assert_eq!(
        collection.collection_type().element(),
        &quire_exact::ValueType::carried_quantity(quire_exact::CarriedQuantityType::new(
            quantities(&amounts)[0].1.clone(),
            quire_exact::QuantityCarrier::Integer {
                domain: quire_exact::IntegerInterval::new(
                    Integer::from(1_i64),
                    Integer::from(20_i64)
                )
                .expect("ordered bounds"),
                rounding: quire_exact::RoundingMode::Exact,
            },
        )),
        "the items are Amount, 1 to 20 of U"
    );
    let unit = quantities(&amounts)[0].1.clone();
    let of = |numbers: &[i64]| -> Vec<((Integer, Integer), quire_exact::UnitId)> {
        numbers
            .iter()
            .map(|n| ((Integer::from(*n), Integer::one()), unit.clone()))
            .collect()
    };
    assert_eq!(quantities(&amounts), of(&[2, 2, 3]));
    assert_eq!(quantities(&call(&package, "plain")), of(&[2, 2, 3]));
    assert_eq!(
        quantities(&call(&package, "twos")),
        of(&[2, 2]),
        "filter keeps both 2 U"
    );
    assert_eq!(booleans(&call(&package, "flags")), [true, true, false]);
    assert_eq!(integer_of(&call(&package, "length")), Integer::from(3_i64));
    assert_eq!(
        integer_of(&call(&package, "twos_counted")),
        Integer::from(2_i64)
    );
    let Value::Quantity(total) = call(&package, "summed") else {
        panic!("the sum is a quantity");
    };
    assert_eq!(
        (
            total.magnitude().numerator().clone(),
            total.magnitude().denominator().clone()
        ),
        (Integer::from(7_i64), Integer::one())
    );
    assert_eq!(total.unit(), &unit, "the total is 7 U");
}
