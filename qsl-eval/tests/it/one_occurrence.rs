// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1335 (FR-093-AC-59): `dropFirst` and `removeOne` through S1, S2, the
//! checker and the evaluator, with the values and the charges QSpec FR-145
//! states (FR-145-AC-10 and AC-11, TC-190).

use ix_trace_rs::trace;
use qsl_eval::value::{CheckedPackageEvaluation, QualifiedName};
use qsl_package::CheckedPackage;
use qsl_semantics::check::{CheckCause, CheckRefusal, PackageDeclarations};
use qsl_semantics::model::object_environment::ObjectEnvironment;
use qsl_semantics::value::{admit_ieee_profile, DefinitionRole};
use quire_eval::FamilyOutcome;
use quire_exact::{
    admit_text, CardinalityBound, ChargePoint, CollectionKind, CollectionType, IllTypedCause,
    Integer, Meter, Outcome, ScalarLimits, TextPayload, TextProfile, TextType, Value, ValueType,
};
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

/// `declarations` as one unit, checked.
fn compile(declarations: &str) -> Result<CheckedPackage, Vec<CheckRefusal>> {
    let text = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n\
         {declarations}\n"
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
    .expect("the assembler builds the package declarations");
    declarations.ieee_profile =
        Some(admit_ieee_profile(&[DefinitionRole::IeeeProfile.reference()], &[]).unwrap());
    declarations
        .check(
            quire_semantic_value::checking::CheckMode::Linked,
            CheckingLimits::default(),
        )
        .map(CheckedPackage::link)
}

/// The one refusal `declarations` draws, as its `ill_typed` cause and the
/// depth of its location (0 is the call itself, 1 an operand of it).
fn refused(declarations: &str) -> (IllTypedCause, usize) {
    let Err(refusals) = compile(declarations) else {
        panic!("{declarations}: expected a refusal");
    };
    let [refusal] = refusals.as_slice() else {
        panic!("{declarations}: one refusal, not {refusals:?}");
    };
    let CheckCause::IllTyped(cause) = &refusal.cause else {
        panic!("{declarations}: ill_typed, not {refusal:?}");
    };
    (*cause, refusal.location.depth())
}

fn int(value: i64) -> Value {
    Value::Integer(Integer::from(value))
}

fn collection(
    kind: CollectionKind,
    element: ValueType,
    bound: (u64, u64),
    items: Vec<Value>,
) -> Value {
    let collection_type = CollectionType::new(
        kind,
        element,
        Some(CardinalityBound::new(bound.0, bound.1).unwrap()),
    );
    quire_exact::form_collection(&collection_type, items, &mut Meter::new(UNLIMITED))
        .unwrap()
        .completed()
        .unwrap()
}

fn integers(kind: CollectionKind, bound: (u64, u64), items: &[i64]) -> Value {
    collection(
        kind,
        ValueType::Integer,
        bound,
        items.iter().copied().map(int).collect(),
    )
}

/// What a call completed with and the charge points it admitted, in order.
struct Called {
    outcome: Outcome<Value>,
    charges: Vec<ChargePoint>,
}

fn call(package: &CheckedPackage, function: &str, arguments: Vec<Value>) -> Called {
    let mut meter = Meter::new(UNLIMITED);
    let evaluation = package
        .call(
            &QualifiedName::unqualified(function).expect("an identifier"),
            arguments,
            &ObjectEnvironment::default(),
            &mut meter,
        )
        .expect("the call runs");
    let FamilyOutcome::Evaluated(outcome) = evaluation.outcome else {
        panic!("a kernel outcome, not {:?}", evaluation.outcome);
    };
    Called {
        outcome,
        charges: crate::support::admitted_charge_points(&meter),
    }
}

/// The collection a call completed with, as its `Debug` text.
fn result_of(called: &Called) -> String {
    let Outcome::Completed(Value::Collection(collection)) = &called.outcome else {
        panic!("a completed collection, not {:?}", called.outcome);
    };
    format!("{:?}", collection.elements())
}

fn ints_text(items: &[i64]) -> String {
    format!("{:?}", items.iter().copied().map(int).collect::<Vec<_>>())
}

#[trace("FR-093-AC-59", "TC-1335", "QSpec-TC-190", "QSpec-FR-145-AC-10")]
#[test]
fn drop_first_removes_the_first_occurrence_of_a_sequence_or_ordered_set() {
    let package = compile(
        "function f1 using v(s: Sequence<Integer>[0, 9]): Sequence<Integer>[0, 8] pure { dropFirst(s) }\n\
         function f2 using v(o: OrderedSet<Integer>[1, 5]): OrderedSet<Integer>[0, 4] pure { dropFirst(o) }",
    )
    .expect("f1 and f2 check with the derived bounds");
    let sequence = |items: &[i64]| integers(CollectionKind::Sequence, (0, 9), items);
    assert_eq!(
        result_of(&call(&package, "f1", vec![sequence(&[3, 1, 3])])),
        ints_text(&[1, 3])
    );
    assert_eq!(
        result_of(&call(&package, "f1", vec![sequence(&[])])),
        ints_text(&[])
    );
    let ordered = integers(CollectionKind::OrderedSet, (1, 5), &[2, 7]);
    assert_eq!(
        result_of(&call(&package, "f2", vec![ordered])),
        ints_text(&[7])
    );
}

#[trace("FR-093-AC-59", "TC-1335", "QSpec-FR-145-AC-10")]
#[test]
fn drop_first_derives_the_bound_max_a_minus_one_zero_to_max_b_minus_one_zero() {
    // `[0, 7]` is below the derived `[0, 8]`, so the body no longer fits it.
    assert!(compile(
        "function f using v(s: Sequence<Integer>[0, 9]): Sequence<Integer>[0, 7] pure { dropFirst(s) }"
    )
    .is_err());
    // `[2, 9]` has a minimum above the derived `[1, 8]`.
    assert!(compile(
        "function f using v(s: Sequence<Integer>[2, 9]): Sequence<Integer>[2, 8] pure { dropFirst(s) }"
    )
    .is_err());
    compile(
        "function f using v(s: Sequence<Integer>[2, 9]): Sequence<Integer>[1, 8] pure { dropFirst(s) }",
    )
    .expect("[2, 9] derives [1, 8]");
}

#[trace("FR-093-AC-59", "TC-1335", "QSpec-FR-145-AC-10")]
#[test]
fn drop_first_over_a_set_or_bag_is_operator_ineligible_at_the_call() {
    for kind in ["Set", "Bag"] {
        let declarations = format!(
            "function f using v(t: {kind}<Integer>[0, 9]): {kind}<Integer>[0, 9] pure {{ dropFirst(t) }}"
        );
        assert_eq!(
            refused(&declarations),
            (IllTypedCause::OperatorIneligible, 0),
            "{kind}"
        );
    }
}

#[trace("FR-093-AC-59", "TC-1335", "QSpec-FR-145-AC-10", "QSpec-FR-145-AC-11")]
#[test]
fn a_non_collection_operand_is_a_type_mismatch_at_the_operand() {
    assert_eq!(
        refused("function f using v(i: Int[0, 9]): Int[0, 9] pure { dropFirst(i) }"),
        (IllTypedCause::TypeMismatch, 1)
    );
    assert_eq!(
        refused("function f using v(i: Int[0, 9]): Int[0, 9] pure { removeOne(i, 1) }"),
        (IllTypedCause::TypeMismatch, 1)
    );
}

#[trace("FR-093-AC-59", "TC-1335", "QSpec-TC-190", "QSpec-FR-145-AC-11")]
#[test]
fn remove_one_drops_exactly_one_occurrence_of_each_collection_kind() {
    let package = compile(
        "function g1 using v(s: Sequence<Integer>[0, 9]): Sequence<Integer>[0, 9] pure { removeOne(s, 3) }\n\
         function g2 using v(b: Bag<Integer>[0, 9]): Bag<Integer>[0, 9] pure { removeOne(b, 2) }\n\
         function g3 using v(t: Set<Integer>[0, 9]): Set<Integer>[0, 9] pure { removeOne(t, 2) }\n\
         function g4 using v(o: OrderedSet<Integer>[0, 9]): OrderedSet<Integer>[0, 9] pure { removeOne(o, 2) }",
    )
    .expect("the removeOne bodies check with the derived bound [max(a-1, 0), b]");
    let case = |function: &str, kind, items: &[i64], expected: &[i64]| {
        let called = call(&package, function, vec![integers(kind, (0, 9), items)]);
        assert_eq!(
            result_of(&called),
            ints_text(expected),
            "{function} {items:?}"
        );
    };
    // The first equal occurrence of a sequence only.
    case("g1", CollectionKind::Sequence, &[3, 1, 3], &[1, 3]);
    // No member equal to the value: the collection is unchanged.
    case("g1", CollectionKind::Sequence, &[1], &[1]);
    // One multiplicity of a bag's equal member.
    case("g2", CollectionKind::Bag, &[2, 2, 5], &[2, 5]);
    case("g2", CollectionKind::Bag, &[5, 7], &[5, 7]);
    // The member of a set or an ordered set.
    case("g3", CollectionKind::Set, &[2, 5], &[5]);
    case("g3", CollectionKind::Set, &[5], &[5]);
    case("g4", CollectionKind::OrderedSet, &[5, 2, 7], &[5, 7]);
}

#[trace("FR-093-AC-59", "TC-1335", "QSpec-FR-145-AC-11")]
#[test]
fn remove_one_types_its_value_as_contains_does_and_derives_its_bound() {
    assert_eq!(
        refused(
            "function f using v(s: Sequence<Integer>[0, 9]): Sequence<Integer>[0, 9] pure { removeOne(s, true) }"
        ),
        (IllTypedCause::TypeMismatch, 1)
    );
    // `[2, 9]` derives `[1, 9]`: the maximum is kept.
    compile(
        "function f using v(s: Sequence<Integer>[2, 9]): Sequence<Integer>[1, 9] pure { removeOne(s, 3) }",
    )
    .expect("[2, 9] derives [1, 9]");
    assert!(compile(
        "function f using v(s: Sequence<Integer>[2, 9]): Sequence<Integer>[2, 9] pure { removeOne(s, 3) }"
    )
    .is_err());
}

#[trace("FR-093-AC-59", "TC-1335", "QSpec-FR-145-AC-11")]
#[test]
fn remove_one_compares_text_members_under_their_profile() {
    let package = compile(
        "function rt using v(t: Bag<Text[0, 8; nfc]>[0, 9], x: Text[0, 8; nfc]): Bag<Text[0, 8; nfc]>[0, 9] pure { removeOne(t, x) }",
    )
    .expect("the text bag checks");

    let text_type = TextType::new(0, 8, TextProfile::Nfc).unwrap();
    let text = |payload: &str| {
        Value::Text(
            admit_text(
                &TextPayload::from_utf8(payload.as_bytes()).unwrap(),
                &text_type,
                &mut Meter::new(UNLIMITED),
            )
            .completed()
            .unwrap(),
        )
    };
    let texts = |items: Vec<Value>| {
        collection(
            CollectionKind::Bag,
            ValueType::Text(text_type),
            (0, 9),
            items,
        )
    };
    let removed = call(
        &package,
        "rt",
        vec![texts(vec![text("a"), text("a"), text("b")]), text("a")],
    );
    assert_eq!(result_of(&removed), format!("{:?}", [text("a"), text("b")]));
}

#[trace("FR-093-AC-59", "TC-1335", "QSpec-FR-145-AC-10")]
#[test]
fn drop_first_charges_only_the_formation_of_the_survivors() {
    let package = compile(
        "function d using v(s: Sequence<Integer>[0, 9]): Sequence<Integer>[0, 8] pure { dropFirst(s) }",
    )
    .unwrap();
    let called = call(
        &package,
        "d",
        vec![integers(CollectionKind::Sequence, (0, 9), &[1, 2, 3])],
    );
    assert_eq!(result_of(&called), ints_text(&[2, 3]));
    // Formation of the two survivors only: no visit, no member walk or test.
    assert_eq!(
        called.charges,
        [
            ChargePoint::FunctionCall,
            ChargePoint::CollectionBound,
            ChargePoint::CollectionResultRetain
        ]
    );
}

#[trace("FR-093-AC-59", "TC-1335", "QSpec-FR-145-AC-11")]
#[test]
fn remove_one_charges_the_contains_scan_then_the_formation() {
    let package = compile(
        "function r using v(s: Sequence<Integer>[0, 9]): Sequence<Integer>[0, 9] pure { removeOne(s, 2) }\n\
         function c using v(s: Sequence<Integer>[0, 9]): Boolean pure { contains(s, 2) }",
    )
    .unwrap();
    let source = || vec![integers(CollectionKind::Sequence, (0, 9), &[1, 2, 3])];
    let removed = call(&package, "r", source());
    assert_eq!(result_of(&removed), ints_text(&[1, 3]));
    // The scan of `1` and `2` (a walk and a test each, stopping at `2`), then
    // the formation of two survivors; `contains` ends instead with a scalar
    // retain, which removeOne does not charge.
    assert_eq!(
        removed.charges,
        [
            ChargePoint::FunctionCall,
            ChargePoint::CollectionMemberWalk,
            ChargePoint::CollectionMemberTest,
            ChargePoint::CollectionMemberWalk,
            ChargePoint::CollectionMemberTest,
            ChargePoint::CollectionBound,
            ChargePoint::CollectionResultRetain
        ]
    );
    let scanned = call(&package, "c", source());
    assert_eq!(removed.charges[..5], scanned.charges[..5]);
}
