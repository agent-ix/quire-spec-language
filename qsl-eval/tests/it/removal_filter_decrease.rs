// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSpec FR-146 "Calls, measures and definedness facts": a cardinality
//! measure `decreases(size(s))` decreases across a recursive call that passes
//! a removal filter of `s` under a `contains(s, e)` membership fact.

use ix_trace_rs::trace;
use qsl_eval::value::{CheckedPackageEvaluation, QualifiedName};
use qsl_package::CheckedPackage;
use qsl_semantics::check::{CheckCause, CheckRefusal, MeasureObligation, PackageDeclarations};
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

/// Checks a unit of `declarations` under the complete value profile.
fn check(declarations: &str) -> Result<CheckedPackage, Vec<CheckRefusal>> {
    let text = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n\
         tuple M(Integer, Int[0, 9]);\n\
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
    let declarations = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        "fixture".to_owned(),
        unit,
        Vec::new(),
        Vec::new(),
    )
    .expect("the unit assembles");
    declarations
        .check(
            CheckingLimits::default(),
            quire_semantic_value::checking::CheckMode::Linked,
        )
        .map(CheckedPackage::link)
}

/// Checks `declarations` and evaluates its nullary function `g`.
fn evaluate(declarations: &str) -> Integer {
    let package =
        check(declarations).unwrap_or_else(|refusals| panic!("{declarations}: {refusals:?}"));
    let evaluation = package
        .call(
            &QualifiedName::unqualified("g").expect("g is an identifier"),
            Vec::new(),
            &ObjectEnvironment::default(),
            &mut Meter::new(UNLIMITED),
        )
        .expect("the call runs");
    match evaluation.outcome {
        FamilyOutcome::Evaluated(Outcome::Completed(Value::Integer(value))) => value,
        other => panic!("{declarations}: a completed integer, not {other:?}"),
    }
}

/// The measure obligation `declarations` is refused with.
fn refused(declarations: &str) -> MeasureObligation {
    let refusals = match check(declarations) {
        Ok(_) => panic!("{declarations}: admitted"),
        Err(refusals) => refusals,
    };
    match refusals.as_slice() {
        [CheckRefusal {
            cause: CheckCause::UnprovedDecrease { obligation, .. },
            ..
        }] => *obligation,
        other => panic!("{declarations}: one unproved-decrease, not {other:?}"),
    }
}

/// `drop` recurses on `argument` under `guard`; `g` calls it on `[1, 2, 3]`.
fn drop(kind: &str, literal: &str, guard: &str, argument: &str) -> String {
    format!(
        "function drop using v(s: {kind}<Int[0, 9]>, e: Int[0, 9]): Integer pure decreases(size(s)) \
         {{ if {guard} then drop({argument}, e) else size(s) }}\n\
         function g using v(): Integer pure {{ drop({literal}[1, 2, 3], 2) }}"
    )
}

/// The removal-filter clauses of QSpec FR-146-AC-15 over every collection
/// kind and every admitted predicate shape, then its refusals. The
/// `not bitIdentical(x, e)` clause over a float element type is not here:
/// the kernel refuses a float collection element (IR-1320).
// IR-1320 tracks the `not bitIdentical(x, e)` float clause of FR-146-AC-15;
// the tag stays in full and that clause is untested until IR-1320 lands.
#[trace("QSpec-TC-191", "QSpec-FR-146-AC-15")]
#[test]
fn a_removal_filter_under_a_membership_fact_decreases_a_cardinality_measure() {
    for (kind, literal) in [
        ("Sequence", "sequence"),
        ("Set", "set"),
        ("Bag", "bag"),
        ("OrderedSet", "orderedSet"),
    ] {
        for predicate in ["x != e", "e != x", "not (x = e)"] {
            let filter = format!("filter(x in s: {predicate})");
            assert_eq!(
                evaluate(&drop(kind, literal, "contains(s, e)", &filter)),
                Integer::from(2_i64),
                "{kind} {predicate}"
            );
        }
        // The filter inside a same-kind conversion keeps its bound count.
        let converted = format!("convert<{kind}<Int[0, 9]>>(filter(x in s: x != e))");
        assert_eq!(
            evaluate(&drop(kind, literal, "contains(s, e)", &converted)),
            Integer::from(2_i64),
            "{kind} convert"
        );
    }

    // `not contains(d, x)` under a further `contains(d, e)` fact.
    assert_eq!(
        evaluate(
            "function drop using v(s: Set<Int[0, 9]>, d: Set<Int[0, 9]>, e: Int[0, 9]): Integer \
             pure decreases(size(s)) \
             { if contains(s, e) and contains(d, e) then drop(filter(x in s: not contains(d, x)), d, e) \
             else size(s) }\n\
             function g using v(): Integer pure { drop(set[1, 2, 3], set[2, 3], 2) }"
        ),
        Integer::from(1_i64)
    );

    // A lexicographic tuple mixing a cardinality and an integer position
    // decreases by the earlier-equal rule.
    let lexicographic = |shrink: &str, count: &str| {
        format!(
            "function lex using v(s: Set<Int[0, 9]>, n: Int[0, 9], e: Int[0, 9]): Integer \
             pure decreases(M(size(s), n)) \
             {{ if contains(s, e) then lex({shrink}, 9, e) else if n > 0 then lex(s, {count}, e) else size(s) }}\n\
             function g using v(): Integer pure {{ lex(set[1, 2, 3], 3, 2) }}"
        )
    };
    assert_eq!(
        evaluate(&lexicographic("filter(x in s: x != e)", "n - 1")),
        Integer::from(2_i64)
    );
    assert_eq!(
        refused(&lexicographic("filter(x in s: x != e)", "n")),
        MeasureObligation::Decrease
    );

    for (guard, argument) in [
        // `s` unchanged.
        ("contains(s, e)", "s"),
        // A removal filter without the membership fact.
        ("size(s) > 0", "filter(x in s: x != e)"),
        // A filter of another predicate shape.
        ("contains(s, e)", "filter(x in s: x < e)"),
        ("contains(s, e)", "filter(x in s: x = e)"),
        // A removal of a non-stable `e`.
        ("contains(s, e) and e < 9", "filter(x in s: x != e + 1)"),
    ] {
        assert_eq!(
            refused(&drop("Set", "set", guard, argument)),
            MeasureObligation::Decrease,
            "{guard} / {argument}"
        );
    }

    // `size` of a non-parameter operand is not a measure element.
    assert_eq!(
        refused(
            "function drop using v(s: Set<Int[0, 9]>, e: Int[0, 9]): Integer \
             pure decreases(size(filter(x in s: true))) \
             { if contains(s, e) then drop(filter(x in s: x != e), e) else 0 }"
        ),
        MeasureObligation::MeasureKind
    );
    // One position holding a cardinality element in one function and an
    // integer element in another of its component.
    assert_eq!(
        refused(
            "function f using v(s: Set<Int[0, 9]>, n: Int[0, 9], e: Int[0, 9]): Integer \
             pure decreases(size(s)) { if contains(s, e) then h(filter(x in s: x != e), n, e) else 0 }\n\
             function h using v(s: Set<Int[0, 9]>, n: Int[0, 9], e: Int[0, 9]): Integer \
             pure decreases(n) { if n > 0 then f(s, n - 1, e) else 0 }"
        ),
        MeasureObligation::MeasureKind
    );
}

/// QSL-1824: a pass-through argument widened to a wider integer parameter is
/// an exact `ConvertScalar` of the caller's parameter. It is still the equal
/// element of a lexicographic measure, so the second element's decrease
/// proves the cycle (FR-065-AC-9).
#[trace("TC-376", "FR-065-AC-9")]
#[test]
fn a_widened_pass_through_argument_is_still_an_equal_measure_element() {
    let declarations = "\
        function f using v(a: Int[0, 9], n: Int[0, 9]): Integer pure decreases(M(a, n)) \
        { if n > 0 then h(a, n - 1) else 0 }\n\
        function h using v(x: Int[0, 100], n: Int[0, 9]): Integer pure decreases(M(x, n)) \
        { if x <= 9 and n > 0 then f(x, n - 1) else 0 }\n\
        function g using v(): Integer pure { f(3, 3) }";
    assert_eq!(evaluate(declarations), Integer::from(0_i64));
}
