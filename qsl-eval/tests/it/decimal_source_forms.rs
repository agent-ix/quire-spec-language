// SPDX-License-Identifier: AGPL-3.0-or-later
//! QSpec TC-185 D24, D26, D27 and D28 (QSpec FR-140-AC-7, FR-140-AC-8):
//! the decimal vectors that have a QSL source form. D24 converts a
//! `Rational` to a `Decimal`; D26 and D27 multiply and divide `decimal(c, s)`
//! literals that are members of the declared result type; D28 refuses a
//! `decimal(c, s)` literal that is not a member at type checking, before any
//! charge. D13 and D18 are evaluator-boundary vectors with no source form.
//!
//! The vectors' `ScalarLimitsV1` figures are for the arithmetic itself. A
//! source call also charges `FunctionCall`, one work unit, so a call through
//! a function needs `work_units` one above the vector's figure to complete
//! and one above its short figure to stop at the same charge point.

use ix_trace_rs::trace;
use qsl_eval::value::{CheckedPackageEvaluation, QualifiedName};
use qsl_package::CheckedPackage;
use qsl_semantics::check::{CheckRefusal, PackageDeclarations};
use qsl_semantics::model::object_environment::ObjectEnvironment;
use quire_eval::FamilyOutcome;
use quire_exact::{ChargePoint, Integer, LimitKind, Meter, Outcome, Rational, ScalarLimits, Value};
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

/// The vector's `ScalarLimitsV1` with the three decimal counters, the work
/// units and the two cardinality counters it names; the rest are unbounded.
fn limits(integer_bits: u64, decimal_digits: u64, scale_expansion: u64, work: u64) -> ScalarLimits {
    ScalarLimits {
        integer_bits,
        decimal_digits,
        scale_expansion,
        value_occurrences: 2,
        work_units: work,
        result_units: 1,
        ..UNLIMITED
    }
}

fn compile(declarations: &str) -> Result<CheckedPackage, Vec<CheckRefusal>> {
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
    PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        "fixture".to_owned(),
        unit,
        Vec::new(),
        Vec::new(),
    )
    .expect("the unit assembles")
    .check(CheckingLimits::default())
    .map(CheckedPackage::link)
}

/// An admitted charge: its point and the integer-bits, decimal-digits and
/// scale-expansion sizes it requested.
type Charged = (ChargePoint, Option<u64>, Option<u64>, Option<u64>);

/// One finished call of `f`: the kernel outcome and the charge points the
/// meter admitted, with the high-water sizes of the decimal charges.
struct Run {
    outcome: Outcome<Value>,
    charges: Vec<Charged>,
}

fn call(package: &CheckedPackage, arguments: Vec<Value>, limits: ScalarLimits) -> Run {
    let mut meter = Meter::new(limits);
    let evaluation = package
        .call(
            &QualifiedName::unqualified("f").expect("f is an identifier"),
            arguments,
            &ObjectEnvironment::default(),
            &mut meter,
        )
        .expect("the call returns a kernel outcome");
    let FamilyOutcome::Evaluated(outcome) = evaluation.outcome else {
        panic!("a kernel outcome, not {:?}", evaluation.outcome);
    };
    Run {
        outcome,
        charges: meter
            .admitted_charges()
            .iter()
            .map(|charge| {
                (
                    charge.point,
                    charge.integer_bits,
                    charge.decimal_digits,
                    charge.scale_expansion,
                )
            })
            .collect(),
    }
}

impl Run {
    fn points(&self) -> Vec<ChargePoint> {
        self.charges.iter().map(|charge| charge.0).collect()
    }

    /// The `(coefficient, scale)` the call completed with.
    fn decimal(&self) -> (Integer, u32) {
        match &self.outcome {
            Outcome::Completed(Value::Decimal(value)) => (
                value.representation().coefficient().clone(),
                value.representation().scale(),
            ),
            other => panic!("a completed decimal, not {other:?}"),
        }
    }

    /// `(limit_kind, limit, consumed, next_charge, charge_point)` of the stop.
    fn stop(&self) -> (LimitKind, u64, u64, Integer, ChargePoint) {
        match &self.outcome {
            Outcome::Incomplete(record) => (
                record.limit_kind,
                record.limit,
                record.consumed,
                record.next_charge.clone(),
                record.charge_point,
            ),
            other => panic!("an incomplete stop, not {other:?}"),
        }
    }
}

fn int(value: i64) -> Integer {
    Integer::from(value)
}

fn one(value: u64) -> Integer {
    Integer::from(value)
}

/// `(code, cause)` of the one refusal of a unit that must not check.
fn refusal(declarations: &str) -> (String, Option<String>) {
    let Err(refusals) = compile(declarations) else {
        panic!("{declarations} is refused");
    };
    let [refusal] = refusals.as_slice() else {
        panic!("one refusal for {declarations}, got {refusals:?}");
    };
    (
        refusal.cause.code().as_str().to_owned(),
        refusal.cause.cause().map(str::to_owned),
    )
}

fn rational(numerator: i64, denominator: i64) -> Value {
    Value::Rational(Rational::new(int(numerator), int(denominator)).expect("a nonzero denominator"))
}

/// The conversion source of D24 for `mode`.
fn conversion(mode: &str) -> CheckedPackage {
    compile(&format!(
        "function f using v(q: Rational[0, 2; 1, 3]): Decimal[0, 100; 2, 2; {mode}] pure \
         {{ convert<Decimal[0, 100; 2, 2; {mode}]>(q) }}\n"
    ))
    .unwrap_or_else(|refusals| panic!("the {mode} conversion checks: {refusals:?}"))
}

/// TC-185 D24: `convert<Decimal[0,100;2,2;nearest-even]>(q)` of `q = 2/3` is
/// the division of `(2,0)` by `(3,0)` at scale 2: five charges in the
/// vector's order, result `(67,2)`; `scale_expansion: 1` stops before any
/// arithmetic; the `exact` target refuses after `decimal.arithmetic` with no
/// rounding charge and no value.
#[trace("QSpec-TC-185", "QSpec-FR-140-AC-7")]
#[test]
fn d24_a_rational_converts_to_a_decimal_as_the_division_at_the_target_scale() {
    let package = conversion("nearest-even");
    let run = call(&package, vec![rational(2, 3)], limits(9, 3, 2, 6));
    assert_eq!(run.decimal(), (int(67), 2));
    assert_eq!(
        run.charges,
        vec![
            (ChargePoint::FunctionCall, None, None, None),
            (ChargePoint::DecimalOperands, Some(2), Some(1), None),
            (
                ChargePoint::DecimalScaleExpansion,
                Some(9),
                Some(3),
                Some(2)
            ),
            (ChargePoint::DecimalArithmetic, Some(9), Some(3), None),
            (ChargePoint::DecimalRounding, Some(9), Some(3), None),
            (ChargePoint::DecimalResultRetain, Some(7), Some(2), Some(0)),
        ]
    );

    let short = call(&package, vec![rational(2, 3)], limits(9, 3, 1, 6));
    assert_eq!(
        short.stop(),
        (
            LimitKind::ScaleExpansion,
            1,
            0,
            one(2),
            ChargePoint::DecimalScaleExpansion
        )
    );
    assert_eq!(
        short.points(),
        vec![ChargePoint::FunctionCall, ChargePoint::DecimalOperands]
    );

    let exact = call(
        &conversion("exact"),
        vec![rational(2, 3)],
        limits(9, 3, 2, 6),
    );
    assert!(
        matches!(exact.outcome, Outcome::Refused(_)),
        "{:?}",
        exact.outcome
    );
    assert_eq!(
        exact.points(),
        vec![
            ChargePoint::FunctionCall,
            ChargePoint::DecimalOperands,
            ChargePoint::DecimalScaleExpansion,
            ChargePoint::DecimalArithmetic,
        ]
    );
}

/// TC-185 D24: `3` is not a member of `Decimal[0,100;2,2]` (`c* = 300 > 100`),
/// so there is no `decimal(2, 0) / decimal(3, 0)` source form into that type.
#[trace("QSpec-TC-185", "QSpec-FR-140-AC-7")]
#[test]
fn d24_has_no_literal_division_form_into_the_target_type() {
    assert_eq!(
        refusal(
            "function f using v(): Decimal[0, 100; 2, 2; nearest-even] pure \
             { decimal(2, 0) / decimal(3, 0) }\n"
        ),
        ("ill_typed".to_owned(), Some("type-mismatch".to_owned()))
    );
}

fn product(mode: &str) -> CheckedPackage {
    compile(&format!(
        "function f using v(): Decimal[-9, 9; 0, 0; {mode}] pure \
         {{ decimal(100, 2) * decimal(3, 0) }}\n"
    ))
    .unwrap_or_else(|refusals| panic!("the {mode} product checks: {refusals:?}"))
}

/// TC-185 D26: the retained operand `(100,2)` is a member of the scale-0
/// type. Both modes return `(3,0)` with no `decimal.rounding` charge (the
/// discarded digits `00` are no rounding step); the multiplication amount
/// is `bits(100) + bits(3) = 9` on the retained operand, not `3` on the
/// normalized `(1,0)`; a short work-unit limit stops at `decimal.result-retain`.
#[trace("QSpec-TC-185", "QSpec-FR-140-AC-3", "QSpec-FR-140-AC-8")]
#[test]
fn d26_a_member_literal_multiplies_at_its_retained_representation() {
    for mode in ["toward-zero", "exact"] {
        let package = product(mode);
        let run = call(&package, vec![], limits(9, 4, 0, 5));
        assert_eq!(run.decimal(), (int(3), 0), "{mode}");
        assert_eq!(
            run.charges,
            vec![
                (ChargePoint::FunctionCall, None, None, None),
                (ChargePoint::DecimalOperands, Some(7), Some(3), None),
                (ChargePoint::DecimalScaleExpansion, None, None, Some(0)),
                (ChargePoint::DecimalArithmetic, Some(9), Some(4), None),
                (ChargePoint::DecimalResultRetain, Some(2), Some(1), Some(0)),
            ],
            "{mode}"
        );

        let short = call(&package, vec![], limits(9, 4, 0, 4));
        assert_eq!(
            short.stop(),
            (
                LimitKind::WorkUnits,
                4,
                4,
                one(1),
                ChargePoint::DecimalResultRetain
            ),
            "{mode}"
        );

        let narrow = call(&package, vec![], limits(8, 4, 0, 5));
        assert_eq!(
            narrow.stop(),
            (
                LimitKind::IntegerBits,
                8,
                7,
                one(9),
                ChargePoint::DecimalArithmetic
            ),
            "{mode}"
        );
    }
}

/// TC-185 D27: the retained operand `(1200,3)` is a member of the max-scale-1
/// type and is divided by `(7,0)` on the divisor side: result `(2,1)`; a
/// `scale_expansion: 1` limit stops at `decimal.scale-expansion` needing 2.
#[trace("QSpec-TC-185", "QSpec-FR-140-AC-4", "QSpec-FR-140-AC-8")]
#[test]
fn d27_a_member_literal_divides_with_the_divisor_side_expansion() {
    let package = compile(
        "function f using v(): Decimal[-1000, 1000; 0, 1; nearest-even] pure \
         { decimal(1200, 3) / decimal(7, 0) }\n",
    )
    .expect("the quotient checks: a nonzero literal divisor discharges its obligation");
    let run = call(&package, vec![], limits(11, 4, 2, 6));
    assert_eq!(run.decimal(), (int(2), 1));
    assert_eq!(
        run.points(),
        vec![
            ChargePoint::FunctionCall,
            ChargePoint::DecimalOperands,
            ChargePoint::DecimalScaleExpansion,
            ChargePoint::DecimalArithmetic,
            ChargePoint::DecimalRounding,
            ChargePoint::DecimalResultRetain,
        ]
    );
    assert_eq!(
        run.charges[1],
        (ChargePoint::DecimalOperands, Some(11), Some(4), None)
    );
    assert_eq!(run.charges[2].3, Some(2));

    let short = call(&package, vec![], limits(11, 4, 1, 6));
    assert_eq!(
        short.stop(),
        (
            LimitKind::ScaleExpansion,
            1,
            0,
            one(2),
            ChargePoint::DecimalScaleExpansion
        )
    );
}

/// TC-185 D28, FR-140-AC-8: a `decimal(c, s)` literal that is not a member of
/// its unique expected type is `ill_typed`/`type-mismatch` at type checking,
/// so no call, meter or charge exists; the member literal is admitted.
#[trace("QSpec-TC-185", "QSpec-FR-140-AC-8")]
#[test]
fn d28_a_non_member_literal_is_refused_at_type_checking() {
    let mismatch = ("ill_typed".to_owned(), Some("type-mismatch".to_owned()));
    assert_eq!(
        refusal(
            "function f using v(): Decimal[-9, 9; 0, 0; toward-zero] pure \
             { decimal(150, 2) * decimal(2, 0) }\n"
        ),
        mismatch
    );
    assert_eq!(
        refusal(
            "function f using v(): Decimal[-1000, 1000; 0, 1; nearest-even] pure \
             { decimal(1234, 3) / decimal(2, 0) }\n"
        ),
        mismatch
    );
    assert!(compile(
        "function f using v(): Decimal[-9, 9; 0, 0; toward-zero] pure \
         { decimal(100, 2) * decimal(3, 0) }\n"
    )
    .is_ok());
}

/// FR-104-AC-16, FR-140-AC-8: a literal has no type of its own. With no
/// expected type (`let`) it is ambiguous; where a non-decimal is expected it
/// is a mismatch; a value outside the type's interval is not a member.
#[trace("QSpec-TC-185", "QSpec-FR-140-AC-8")]
#[test]
fn a_decimal_literal_takes_only_a_decimal_expected_type_it_is_a_member_of() {
    assert_eq!(
        refusal(
            "type P = Decimal[0, 1000; 2, 2; exact];\n\
             function f using v(): P pure { let v = decimal(15, 1) + decimal(225, 2) in v }\n"
        ),
        ("ill_typed".to_owned(), Some("ambiguous-literal".to_owned()))
    );
    assert_eq!(
        refusal("function f using v(): Int[0, 9] pure { decimal(1, 0) }\n"),
        ("ill_typed".to_owned(), Some("type-mismatch".to_owned()))
    );
    assert_eq!(
        refusal(
            "function f using v(): Decimal[0, 9; 0, 0; exact] pure { decimal(10, 0) + decimal(1, 0) }\n"
        ),
        ("ill_typed".to_owned(), Some("type-mismatch".to_owned()))
    );
}

const P: &str =
    "type P = Decimal[0, 1000; 2, 2; exact];\ntype R = Decimal[0, 1000; 1, 1; exact];\n";

fn mismatch() -> (String, Option<String>) {
    ("ill_typed".to_owned(), Some("type-mismatch".to_owned()))
}

fn ambiguous() -> (String, Option<String>) {
    ("ill_typed".to_owned(), Some("ambiguous-literal".to_owned()))
}

fn unproved_nonzero() -> (String, Option<String>) {
    (
        "undefined_expression".to_owned(),
        Some("unproved-nonzero".to_owned()),
    )
}

/// FR-104-AC-16 e1 to e6 (TC-1084 step 5): the six D25 expressions.
#[trace("FR-104-AC-16", "TC-1084")]
#[test]
fn the_six_d25_expressions_check_or_refuse_as_the_criterion_states() {
    let case = |parameters: &str, result: &str, body: &str| {
        format!("{P}function f using v({parameters}): {result} pure {{ {body} }}\n")
    };
    assert_eq!(
        refusal(&case(
            "",
            "P",
            "let v = decimal(15, 1) + decimal(225, 2) in v"
        )),
        ambiguous()
    );
    assert_eq!(
        refusal(&case("", "Boolean", "decimal(15, 1) = decimal(150, 2)")),
        ambiguous()
    );
    assert!(compile(&case("p: P", "P", "p + decimal(15, 1)")).is_ok());
    assert_eq!(refusal(&case("p: P, r: R", "P", "p + r")), mismatch());
    assert!(compile(&case("p: P, r: R", "P", "p + convert<P>(r)")).is_ok());
    assert!(compile(&case("", "P", "decimal(15, 1) + decimal(225, 2)")).is_ok());
}

/// FR-104-AC-13 (TC-1084): a `decimal(c, s)` operand takes the unique
/// decimal type of its typed peer when its position gives no expected type,
/// and a negated literal takes its operand's decimal type.
#[trace("FR-104-AC-13", "FR-104-AC-16", "TC-1084")]
#[test]
fn a_decimal_literal_takes_the_type_of_its_typed_peer() {
    let case = |parameters: &str, body: &str| {
        format!("{P}function f using v({parameters}): Boolean pure {{ {body} }}\n")
    };
    for body in [
        "p + decimal(1, 2) = p",
        "p + -decimal(5, 0) = p",
        "-decimal(5, 0) + p = p",
        "decimal(1, 2) + p = p",
        "p = p + decimal(1, 2)",
        "p * decimal(1, 2) = p",
        "p - decimal(1, 2) = p",
        "decimal(15, 1) = p",
        "p < decimal(15, 1)",
        "-decimal(5, 0) = p",
        "r + decimal(15, 1) = r",
    ] {
        let package = compile(&case("p: P, r: R", body))
            .unwrap_or_else(|refusals| panic!("{body} checks: {refusals:?}"));
        let _ = package;
    }
    let converted =
        format!("{P}function f using v(r: R): P pure {{ convert<P>(decimal(15, 1) + r) }}\n");
    assert!(
        compile(&converted).is_ok(),
        "{:?}",
        compile(&converted).err()
    );
    assert_eq!(refusal(&case("p: P, r: R", "p + r = p")), mismatch());
    assert_eq!(
        refusal(&case(
            "p: P",
            "let s = decimal(1, 2) + decimal(2, 2) in s = p"
        )),
        ambiguous()
    );
    let negated = format!("{P}function f using v(): P pure {{ -decimal(5, 0) }}\n");
    let package = compile(&negated);
    assert!(package.is_ok(), "{:?}", package.err());
}

/// FR-104-AC-13: with `a: P` and `b: P`, `a + decimal(1, 2) = b` evaluates to
/// `true` at `a = 1.00` ... through the checked literal typed `P`.
#[trace("FR-104-AC-13", "TC-1084")]
#[test]
fn a_literal_beside_a_typed_operand_evaluates_at_the_peer_type() {
    let package = compile(&format!(
        "{P}function f using v(a: P, b: P): Boolean pure {{ a + decimal(1, 2) = b }}\n"
    ))
    .expect("the body checks with the literal typed P");
    let value = |coefficient: i64| Value::Decimal(quire_exact::Decimal::new(int(coefficient), 2));
    let yes = call(&package, vec![value(1), value(2)], UNLIMITED);
    assert!(
        matches!(yes.outcome, Outcome::Completed(Value::Boolean(true))),
        "{:?}",
        yes.outcome
    );
    let no = call(&package, vec![value(1), value(3)], UNLIMITED);
    assert!(
        matches!(no.outcome, Outcome::Completed(Value::Boolean(false))),
        "{:?}",
        no.outcome
    );
}

/// FR-140-AC-8 and the nonzero obligation of a divisor: only a literal with
/// a nonzero coefficient discharges it, whatever its scale. A zero literal,
/// a parameter, `p / q` and an `if` of two literals do not.
#[trace("QSpec-TC-185", "QSpec-FR-140-AC-4")]
#[test]
fn only_a_nonzero_literal_divisor_discharges_the_nonzero_obligation() {
    let case = |parameters: &str, body: &str| {
        format!("{P}function f using v({parameters}): P pure {{ {body} }}\n")
    };
    assert!(compile(&case("p: P", "p / decimal(4, 0)")).is_ok());
    assert!(compile(&case("p: P", "p / decimal(400, 2)")).is_ok());
    assert_eq!(
        refusal(&case("p: P", "p / decimal(0, 2)")),
        unproved_nonzero()
    );
    assert_eq!(
        refusal(&case("p: P", "p / decimal(0, 0)")),
        unproved_nonzero()
    );
    assert_eq!(refusal(&case("p: P, q: P", "p / q")), unproved_nonzero());
    assert_eq!(
        refusal(&case(
            "p: P, b: Boolean",
            "p / (if b then decimal(5, 2) else decimal(6, 2))"
        )),
        unproved_nonzero()
    );
}

/// FR-104-AC-13 (TC-1084): the sum, difference and product of two operands of
/// one decimal type are typed by it with no expected type (QSpec FR-140-AC-7),
/// so `a + b = b + a` checks, evaluates to `true`, and the overflow of
/// `0.99 + 0.01` at `P` is a runtime refusal, not a typing one.
#[trace("FR-104-AC-13", "QSpec-FR-140-AC-7", "TC-1084")]
#[test]
fn sums_differences_and_products_of_one_typed_pair_need_no_expected_type() {
    let wide = "type P = Decimal[-99, 99; 2, 2; nearest-even];\n";
    for body in ["a + b = b + a", "a * b = b * a", "a - b = a - b"] {
        let package = compile(&format!(
            "{wide}function f using v(a: P, b: P): Boolean pure {{ {body} }}\n"
        ))
        .unwrap_or_else(|refusals| panic!("{body} checks: {refusals:?}"));
        let value =
            |coefficient: i64| Value::Decimal(quire_exact::Decimal::new(int(coefficient), 2));
        let run = call(&package, vec![value(1), value(2)], UNLIMITED);
        assert!(
            matches!(run.outcome, Outcome::Completed(Value::Boolean(true))),
            "{body}: {:?}",
            run.outcome
        );
    }
    let package = compile(&format!(
        "{wide}function f using v(a: P, b: P): Boolean pure {{ a + b = b + a }}\n"
    ))
    .expect("the sum checks");
    let value = |coefficient: i64| Value::Decimal(quire_exact::Decimal::new(int(coefficient), 2));
    let over = call(&package, vec![value(99), value(1)], UNLIMITED);
    assert!(
        matches!(over.outcome, Outcome::Refused(_)),
        "{:?}",
        over.outcome
    );
    assert_eq!(
        refusal(&format!(
            "{wide}function d using v(a: P, b: P): Boolean pure {{ let s = a / b in s = a }}\n"
        )),
        ambiguous()
    );
}
