// SPDX-License-Identifier: AGPL-3.0-or-later
//! `sum<N>` over every numeric family a `quire.op.collection.sum` identity
//! exists for, from source text to its evaluated value (FR-517-AC-20, FR-145
//! "sum", QSpec value-accounting "Collection sum schedule", TC-1312 row 20).

use ix_trace_rs::trace;
use qsl_eval::value::{CheckedPackageEvaluation, QualifiedName};
use qsl_package::CheckedPackage;
use qsl_semantics::check::{CheckCause, PackageDeclarations};
use qsl_semantics::model::object_environment::ObjectEnvironment;
use qsl_semantics::value::{admit_ieee_profile, DefinitionRole};
use quire_eval::FamilyOutcome;
use quire_exact::{IllTypedCause, Meter, Outcome, Value};
use quire_semantic_value::checking::CheckingLimits;
use quire_semantic_value::loss::{LocatedLoss, ValueLoss};

use super::quantity_literals::{package_with, UNLIMITED};
use quire_semantic_value::checking::CheckMode;

const UNIT: &str = "dimension Length; unit m : Length = rational(1, 1);\n\
                    type Len = Quantity<m>;\n\
                    function two using v(): Sequence<Len>[0, 4] pure { sequence[2 m, 3 m] }\n\
                    function three using v(): Sequence<Len>[0, 4] pure { sequence[2 m, 3 m, 4 m] }\n\
                    function nothing using v(): Sequence<Len>[0, 4] pure { filter(y in two(): y > 9 m) }\n";

/// The outcome of calling `f` in the package that `functions` declares, with
/// the IEEE profile selected so that a float sum checks.
fn outcome(functions: &str) -> Outcome<Value> {
    evaluation(functions).0
}

/// [`outcome`] and the value losses the evaluation reported.
fn evaluation(functions: &str) -> (Outcome<Value>, Vec<LocatedLoss>) {
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
    let mut declarations = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        "fixture".to_owned(),
        unit,
        Vec::new(),
        Vec::new(),
    )
    .expect("the assembler builds the declarations");
    declarations.ieee_profile =
        Some(admit_ieee_profile(&[DefinitionRole::IeeeProfile.reference()], &[]).unwrap());
    let package = CheckedPackage::link(
        declarations
            .check(CheckingLimits::default(), CheckMode::Linked)
            .unwrap_or_else(|refusal| panic!("{functions}: {refusal:?}")),
    );
    let evaluation = package
        .call(
            &QualifiedName::unqualified("f").expect("f is an identifier"),
            Vec::new(),
            &ObjectEnvironment::default(),
            &mut Meter::new(UNLIMITED),
        )
        .expect("the call runs");
    let FamilyOutcome::Evaluated(outcome) = evaluation.outcome else {
        panic!("a kernel outcome, not {:?}", evaluation.outcome);
    };
    (outcome, evaluation.losses)
}

/// The checker refuses `functions` for an unproved range (FR-046, FR-100).
fn assert_unproved_range(functions: &str) {
    let refusals = package_with(functions).expect_err("an unproved total is refused");
    assert!(
        refusals
            .iter()
            .any(|refusal| matches!(refusal.cause, CheckCause::Unproved(_))),
        "{refusals:?}"
    );
}

fn truth(functions: &str) -> bool {
    match outcome(functions) {
        Outcome::Completed(Value::Boolean(truth)) => truth,
        other => panic!("a completed Boolean, not {other:?}"),
    }
}

/// FR-517-AC-20: a quantity sum adds in the unit of `N`, an empty one is `0 m`
/// and a summand of another family is a type mismatch at the summand.
#[trace("FR-517-AC-20", "TC-1312")]
#[test]
fn a_quantity_sum_adds_the_summands_in_the_unit_of_n() {
    let check = |expression: &str| {
        truth(&format!(
            "{UNIT}function f using v(): Boolean pure {{ {expression} }}\n"
        ))
    };
    assert!(check("sum<Len>(x in two(): x) = 5 m"));
    assert!(!check("sum<Len>(x in two(): x) = 6 m"));
    assert!(check("sum<Len>(x in three(): x + 1 m) = 12 m"));
    assert!(check("sum<Len>(x in nothing(): x) = 0 m"));

    let refusals = package_with(&format!(
        "{UNIT}function ints using v(): Sequence<Integer>[0, 4] pure {{ sequence[1, 2] }}\nfunction f using v(): Boolean pure {{ sum<Len>(x in ints(): x) = 3 m }}\n"
    ))
    .expect_err("an integer summand is not a quantity of m");
    assert!(
        refusals.iter().any(|refusal| matches!(
            refusal.cause,
            CheckCause::IllTyped(IllTypedCause::TypeMismatch)
        )),
        "{refusals:?}"
    );
}

/// A unit declaring the summand type `S` and the result type `N`, the values
/// `values` as the functions `e0`, `e1`, .. of type `S`, their sequence `xs`
/// and the Boolean `body` as `f`.
fn over(summand: &str, result: &str, values: &[&str], body: &str) -> String {
    let mut text = format!("type S = {summand};\ntype N = {result};\n");
    for (index, value) in values.iter().enumerate() {
        text += &format!("function e{index} using v(): S pure {{ {value} }}\n");
    }
    let elements: Vec<_> = (0..values.len())
        .map(|index| format!("e{index}()"))
        .collect();
    text += &format!(
        "function xs using v(): Sequence<S>[0, 4] pure {{ sequence[{}] }}\n\
         function f using v(): Boolean pure {{ {body} }}\n",
        elements.join(", ")
    );
    text
}

/// FR-517-AC-20: the decimal family adds under the type `N` pins, an exact sum
/// is the exact total, and a total that can leave `N`'s domain is refused.
#[trace("FR-517-AC-20", "TC-1312")]
#[test]
fn a_decimal_sum_is_exact_and_leaves_its_domain_undefined() {
    // Four summands of at most 10.00 total at most 40.00, so `N` is wider.
    let ty = "Decimal[0, 1000; 2, 2; nearest-even]";
    let sum = |total: &str| {
        truth(&over(
            ty,
            "Decimal[0, 4000; 2, 2; nearest-even]",
            &["decimal(150, 2)", "decimal(275, 2)"],
            &format!("sum<N>(x in xs(): x) = {total}"),
        ))
    };
    assert!(sum("decimal(425, 2)"));
    assert!(!sum("decimal(426, 2)"));
    // A total that can leave `N`'s domain is refused when checked (FR-100):
    // it never reaches a linked package's evaluation.
    let text = over(ty, ty, &["decimal(600, 2)", "decimal(600, 2)"], "true").replace(
        "function f using v(): Boolean pure { true }",
        "function f using v(): N pure { sum<N>(x in xs(): x) }",
    );
    assert_unproved_range(&text);
}

/// FR-517-AC-20: the rational family, and the sum of an empty source being the
/// family's zero, which `N` must admit.
#[trace("FR-517-AC-20", "TC-1312")]
#[test]
fn a_rational_sum_adds_exactly_and_an_empty_one_is_zero() {
    let sum = |second: &str| {
        truth(&over(
            "Rational[0, 10; 1, 2]",
            "Rational[-80, 80; 1, 2]",
            &["rational(1, 2)", second],
            "sum<N>(x in xs(): x) = rational(3, 2)",
        ))
    };
    assert!(sum("rational(1, 1)"));
    assert!(!sum("rational(3, 2)"));
    let text = over(
        "Rational[1, 10; 1, 2]",
        "Rational[1, 80; 1, 2]",
        &["rational(2, 1)"],
        "true",
    )
    .replace(
        "function f using v(): Boolean pure { true }",
        "function f using v(): N pure { sum<N>(x in filter(y in xs(): y > rational(3, 1)): x) }",
    );
    assert_unproved_range(&text);
}

/// FR-517-AC-20: the float family adds under the rounding mode `N` pins.
#[trace("FR-517-AC-20", "TC-1312")]
#[test]
fn a_float_sum_adds_under_its_pinned_rounding() {
    let sum = |second: &str| {
        truth(&over(
            "Float64[nearest-even]",
            "Float64[nearest-even]",
            &["float64(bits: 0x3ff8000000000000)", second],
            "quire::value::ieee::bitIdentical(sum<N>(x in xs(): x), \
             float64(bits: 0x400e000000000000))",
        ))
    };
    // 1.5 + 2.25 = 3.75
    assert!(sum("float64(bits: 0x4002000000000000)"));
    // 1.5 + 1.5 = 3.0
    assert!(!sum("float64(bits: 0x3ff8000000000000)"));
}

/// Whether checking `functions` refuses with a type mismatch.
fn is_mismatch(functions: &str) -> bool {
    package_with(functions)
        .expect_err("the sum is refused")
        .iter()
        .any(|refusal| {
            matches!(
                refusal.cause,
                CheckCause::IllTyped(IllTypedCause::TypeMismatch)
            )
        })
}

/// FR-517-AC-20, FR-145: a summand must be of the very float
/// type or unit of the total, so another float width or unit is a type mismatch.
#[trace("FR-517-AC-20", "TC-1312")]
#[test]
fn a_summand_of_another_float_width_or_unit_is_a_type_mismatch() {
    let sum = |summand: &str, result: &str| {
        format!(
            "type S = {summand};\ntype N = {result};\n\
             function f using v(s: Sequence<S>[0, 4]): N pure {{ sum<N>(x in s: x) }}\n"
        )
    };
    assert!(is_mismatch(&sum(
        "Float32[nearest-even]",
        "Float64[nearest-even]"
    )));
    assert!(is_mismatch(&format!(
        "dimension Length; unit m : Length = rational(1, 1); unit km : Length = rational(1000, 1) * m;\n{}",
        sum("Quantity<km>", "Quantity<m>")
    )));
}

/// FR-517-AC-20, QSpec FR-148: an inexact float addition records the IEEE flag
/// set it raised, located at the sum; an exact one records nothing.
#[trace("FR-517-AC-20", "TC-1312")]
#[test]
fn an_inexact_float_sum_records_its_ieee_flags_at_the_sum() {
    let inexact = |second: &str| {
        evaluation(
            &over(
                "Float64[nearest-even]",
                "Float64[nearest-even]",
                &["float64(bits: 0x3ff0000000000000)", second],
                "true",
            )
            .replace(
                "function f using v(): Boolean pure { true }",
                "function f using v(): N pure { sum<N>(x in xs(): x) }",
            ),
        )
        .1
    };
    // 1 + 2^-60 is inexact; 1 + 1 is not.
    let losses = inexact("float64(bits: 0x3c30000000000000)");
    assert!(
        matches!(losses.as_slice(), [loss] if matches!(loss.loss, ValueLoss::IeeeFlags(_))),
        "{losses:?}"
    );
    assert!(inexact("float64(bits: 0x3ff0000000000000)").is_empty());
}

const CARRIED: &str = "dimension Length; unit m : Length = rational(1, 1);\n\
                       dimension Time; unit s : Time = rational(1, 1);\n\
                       unit cm : Length = rational(1, 100) * m;\n\
                       type Li = Quantity<m, Int[-5, 5]>;\n\
                       type Lm = Quantity<m>;\n";

/// FR-517-AC-20, QSpec FR-142-AC-33 (TC-187 U45): `sum<N>` over a carried
/// `N` checks a summand of that very carried type, and refuses every other
/// summand at the summand as a type mismatch; the unit sum takes a converted
/// summand, and so does the carried sum.
#[trace("FR-517-AC-20", "TC-1312", "QSpec-FR-142-AC-33", "QSpec-TC-187")]
#[test]
fn a_carried_sum_takes_a_summand_of_its_very_carried_type() {
    let sum = |element: &str, result: &str, body: &str| {
        format!(
            "{CARRIED}function f using v(q: Sequence<{element}>[0, 1]): {result} pure \
             {{ sum<{result}>(x in q: {body}) }}\n"
        )
    };
    package_with(&sum("Li", "Li", "x")).expect("the carried type itself");
    for element in [
        "Quantity<m, Int[-5, 5; nearest-even]>",
        "Quantity<m, Int[-4, 4]>",
        "Quantity<cm, Int[-5, 5]>",
        "Lm",
        "Int[-5, 5]",
    ] {
        assert!(is_mismatch(&sum(element, "Li", "x")), "{element}");
    }
    assert!(
        is_mismatch(&sum("Li", "Lm", "x")),
        "a carried summand of a unit sum"
    );
    package_with(&sum("Li", "Lm", "convert<Quantity<m>>(x)")).expect("a converted unit summand");
    package_with(&sum(
        "Quantity<m, Int[-2, 2]>",
        "Li",
        "convert<Quantity<m, Int[-5, 5]>>(x)",
    ))
    .expect("a converted carried summand");
}

/// FR-517-AC-20, QSpec FR-142-AC-35, AC-36 (TC-187 U47, U48): a linked carried
/// sum is admitted only when every prefix total, and the zero of a source that
/// can be empty, is proved inside the carrier.
#[trace("FR-517-AC-20", "TC-1312", "QSpec-FR-142-AC-35", "QSpec-FR-142-AC-36")]
#[test]
fn a_linked_carried_sum_proves_every_prefix_inside_the_carrier() {
    let sum = |carrier: &str, source: &str| {
        format!(
            "{CARRIED}type Lp = Quantity<m, {carrier}>;\n\
             function f using v(q: Sequence<Lp>{source}): Lp pure {{ sum<Lp>(x in q: x) }}\n"
        )
    };
    assert_unproved_range(&sum("Int[-5, 5]", "[0, 3]"));
    package_with(&sum("Int[-5, 5]", "[0, 1]")).expect("one summand, or none, is inside");
    assert_unproved_range(&sum("Int[1, 5]", "[0, 1]"));
    package_with(&sum("Int[1, 5]", "[1, 1]")).expect("one summand is inside");
    assert_unproved_range(&sum("Decimal[-500, 500; 2, 2; exact]", "[0, 3]"));
    package_with(&sum("Rational[-3, 7; 1, 12]", "[0, 1]")).expect("one rational summand");
    assert_unproved_range(&sum("Rational[-3, 7; 1, 12]", "[0, 3]"));
}

/// FR-517-AC-20, QSpec FR-142-AC-34, AC-36 (TC-187 U46, U48): a carried sum of
/// one summand is that summand and the empty sum is `0 m`, evaluated from
/// source.
#[trace("FR-517-AC-20", "TC-1312", "QSpec-FR-142-AC-34", "QSpec-FR-142-AC-36")]
#[test]
fn a_carried_sum_of_one_summand_or_none_evaluates() {
    let check = |expression: &str| {
        truth(&format!(
            "{CARRIED}function one using v(): Sequence<Li>[0, 1] pure \
             {{ sequence[convert<Li>(2 m)] }}\n\
             function vacant using v(): Sequence<Li>[0, 1] pure {{ filter(y in one(): false) }}\n\
             function total using v(q: Sequence<Li>[0, 1]): Li pure {{ sum<Li>(x in q: x) }}\n\
             function back using v(c: Li): Quantity<m> pure {{ convert<Quantity<m>>(c) }}\n\
             function f using v(): Boolean pure {{ {expression} }}\n"
        ))
    };
    assert!(check("back(total(one())) = 2 m"));
    assert!(!check("back(total(one())) = 3 m"));
    assert!(check("back(total(vacant())) = 0 m"));
}
