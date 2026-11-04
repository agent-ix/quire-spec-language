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
    let text = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n{UNIT}"
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
/// source text through the checker and `CheckedPackage::link`. Not covered
/// here, and left to the follow-on under QSL-1681: the nominal unit `U` on the
/// `Amount`, `Total` and `Count` domains, and evaluation of an admitted
/// (emitted and read-back) artifact agreeing with these values.
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
