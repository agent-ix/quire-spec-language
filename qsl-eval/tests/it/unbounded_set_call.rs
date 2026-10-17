// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-845 (FR-335-AC-5): S6a evaluates a function over an unbounded set of
//! any size, stopping only on the caller's meter.

use ix_trace_rs::trace;
use qsl_eval::value::{CheckedPackageEvaluation, QualifiedName};
use qsl_package::CheckedPackage;
use qsl_semantics::check::PackageDeclarations;
use qsl_semantics::family::FamilyOutcome;
use qsl_semantics::model::object_environment::ObjectEnvironment;
use quire_exact::{
    form_collection, ChargePoint, CollectionKind, CollectionType, Integer, IntegerInterval, Meter,
    Outcome, ScalarLimits, Value, ValueType,
};
use quire_semantic_value::checking::CheckingLimits;

const UNIT: &str = "language \"ix:native\" edition \"1-draft\";\n\
    profile v = \"quire.value.complete/v1\";\n\
    function f using v(s: Set<Int[0, 9999]>): Integer pure { size(s) + 1 }\n";

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

fn package() -> CheckedPackage {
    let parsed = qsl_cst::parse(
        qsl_foundation::SourceIdentity::new("a", "u", "git", "1"),
        "unit.native",
        UNIT.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let unit = qsl_forms::build_unit(&parsed).expect("S2 builds the unit");
    let declarations = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        unit,
        Vec::new(),
        Vec::new(),
    )
    .expect("the unit assembles");
    CheckedPackage::link(
        declarations
            .check(CheckingLimits::default())
            .expect("the package checks"),
    )
}

/// The unbounded `Set<Int[0, 9999]>` holding `0..count`, formed on `meter`.
fn set_of(count: i64, meter: &mut Meter) -> Outcome<Value> {
    let element = ValueType::Int(
        IntegerInterval::new(Integer::from(0_i64), Integer::from(9999_i64)).unwrap(),
    );
    let collection = CollectionType::new(CollectionKind::Set, element, None);
    let members = (0..count)
        .map(|n| Value::Integer(Integer::from(n)))
        .collect();
    form_collection(&collection, members, meter).expect("every member is in the element type")
}

fn call(package: &CheckedPackage, argument: Value, meter: &mut Meter) -> Outcome<Value> {
    let evaluation = package
        .call(
            &QualifiedName::unqualified("f").expect("f is an identifier"),
            vec![argument],
            &ObjectEnvironment::default(),
            meter,
        )
        .expect("the call runs");
    match evaluation.outcome {
        FamilyOutcome::Evaluated(outcome) => outcome,
        other => panic!("a kernel outcome, not {other:?}"),
    }
}

/// A 1,000-member set is below no declared bound and `f` returns 1,001; a
/// meter too small to build it stops at a collection charge point.
#[trace("TC-845", "FR-335-AC-5")]
#[test]
fn an_unbounded_set_of_any_size_evaluates_and_stops_only_on_the_meter() {
    let package = package();
    let mut meter = Meter::new(UNLIMITED);
    let Outcome::Completed(set) = set_of(1_000, &mut meter) else {
        panic!("the set forms on an unlimited meter");
    };
    let mut meter = Meter::new(UNLIMITED);
    assert_eq!(
        format!("{:?}", call(&package, set, &mut meter)),
        format!(
            "{:?}",
            Outcome::Completed(Value::Integer(Integer::from(1_001_i64)))
        )
    );

    let mut small = Meter::new(ScalarLimits {
        work_units: 10,
        ..UNLIMITED
    });
    let Outcome::Incomplete(stopped) = set_of(1_000, &mut small) else {
        panic!("a 10-unit meter cannot build 1,000 members");
    };
    assert!(
        matches!(
            stopped.charge_point,
            ChargePoint::CollectionBound | ChargePoint::CollectionElement
        ),
        "{:?}",
        stopped.charge_point
    );
}
