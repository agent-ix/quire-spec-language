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
    construct_collection, form_collection, ChargePoint, CollectionKind, CollectionType, Deferred,
    Integer, IntegerInterval, Meter, Outcome, ScalarLimits, Value, ValueType,
};
use quire_semantic_value::checking::CheckingLimits;

const UNIT: &str = "language \"ix:native\" edition \"1-draft\";\n\
    profile v = \"quire.value.complete/v1\";\n\
    function f using v(s: Set<Int[0, 9999]>): Integer pure { size(s) + 1 }\n\
    function g using v(): Integer pure { f(set[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]) }\n";

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

fn call(
    package: &CheckedPackage,
    name: &str,
    arguments: Vec<Value>,
    meter: &mut Meter,
) -> Outcome<Value> {
    let evaluation = package
        .call(
            &QualifiedName::unqualified(name).expect("a function name is an identifier"),
            arguments,
            &ObjectEnvironment::default(),
            meter,
        )
        .expect("the call runs");
    match evaluation.outcome {
        FamilyOutcome::Evaluated(outcome) => outcome,
        other => panic!("a kernel outcome, not {other:?}"),
    }
}

/// `qsl-eval` has no request writer, so the AC-1 item is not read here: the
/// observable results are `f`'s outcomes and the meter's state, and the
/// evaluation takes no request item as input.
///
/// A 1,000-member set is below no declared bound and `f` returns 1,001; a
/// meter too small to form it stops at the member walk, and one too small
/// for the call stops at `function.call`.
#[trace("TC-845", "FR-335-AC-5")]
#[test]
fn an_unbounded_set_of_any_size_evaluates_and_stops_only_on_the_meter() {
    let package = package();
    let mut meter = Meter::new(UNLIMITED);
    let Outcome::Completed(set) = set_of(1_000, &mut meter) else {
        panic!("the set forms on an unlimited meter");
    };
    let formed = set.clone();
    let mut meter = Meter::new(UNLIMITED);
    assert_eq!(
        format!("{:?}", call(&package, "f", vec![set], &mut meter)),
        format!(
            "{:?}",
            Outcome::Completed(Value::Integer(Integer::from(1_001_i64)))
        )
    );

    // Forming a set of 1,000 members charges `collection.member-walk` for
    // each membership comparison before `collection.bound`, so a 10-unit
    // meter stops at the member walk.
    let mut small = Meter::new(ScalarLimits {
        work_units: 10,
        ..UNLIMITED
    });
    let Outcome::Incomplete(stopped) = set_of(1_000, &mut small) else {
        panic!("a 10-unit meter cannot build 1,000 members");
    };
    assert_eq!(stopped.charge_point, ChargePoint::CollectionMemberWalk);

    // `f` itself on a meter with no work left stops at `function.call`.
    let mut spent = Meter::new(ScalarLimits {
        work_units: 0,
        ..UNLIMITED
    });
    let Outcome::Incomplete(stopped) = call(&package, "f", vec![formed], &mut spent) else {
        panic!("a spent meter denies the call");
    };
    assert_eq!(stopped.charge_point, ChargePoint::FunctionCall);

    // A set built inside `g` is charged on the caller's meter, so `g` itself
    // returns `Incomplete` when the meter is too small, and completes with
    // the same count on an ample one.
    let mut ample = Meter::new(UNLIMITED);
    assert_eq!(
        format!("{:?}", call(&package, "g", Vec::new(), &mut ample)),
        format!(
            "{:?}",
            Outcome::Completed(Value::Integer(Integer::from(13_i64)))
        )
    );
    let mut scarce = Meter::new(ScalarLimits {
        work_units: 6,
        ..UNLIMITED
    });
    assert!(
        matches!(
            call(&package, "g", Vec::new(), &mut scarce),
            Outcome::Incomplete(_)
        ),
        "a 6-unit meter cannot build the 12-member set inside g"
    );
    assert!(scarce.consumed(quire_exact::LimitKind::WorkUnits) <= 6);
}

/// `collection.element` and `collection.bound` are reached through
/// `construct_collection`, where a set coalesces first but an element is
/// charged before it runs and a sequence goes straight to the bound.
#[trace("TC-845", "FR-335-AC-5")]
#[test]
fn construct_collection_stops_at_the_element_and_bound_charge_points() {
    let element =
        ValueType::Int(IntegerInterval::new(Integer::from(0_i64), Integer::from(9_i64)).unwrap());
    let set = CollectionType::new(CollectionKind::Set, element.clone(), None);
    let one: Vec<Deferred<'_>> = vec![Box::new(|_| {
        Outcome::Completed(Value::Integer(Integer::from(1_i64)))
    })];
    let mut spent = Meter::new(ScalarLimits {
        work_units: 0,
        ..UNLIMITED
    });
    let Outcome::Incomplete(stopped) = construct_collection(&set, one, &mut spent) else {
        panic!("a spent meter denies the first element");
    };
    assert_eq!(stopped.charge_point, ChargePoint::CollectionElement);

    let sequence = CollectionType::new(CollectionKind::Sequence, element, None);
    let mut spent = Meter::new(ScalarLimits {
        work_units: 0,
        ..UNLIMITED
    });
    let Outcome::Incomplete(stopped) = construct_collection(&sequence, Vec::new(), &mut spent)
    else {
        panic!("a spent meter denies the bound charge");
    };
    assert_eq!(stopped.charge_point, ChargePoint::CollectionBound);
}
