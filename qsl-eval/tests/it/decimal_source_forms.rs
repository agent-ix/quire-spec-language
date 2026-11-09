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
use qsl_semantics::check::{
    AssemblyCause, CheckCause, CheckRefusal, CheckingLimitKind, DecimalLiteralLimits,
    PackageDeclarations,
};
use qsl_semantics::model::object_environment::ObjectEnvironment;
use quire_eval::FamilyOutcome;
use quire_exact::{ChargePoint, Integer, LimitKind, Meter, Outcome, Rational, ScalarLimits, Value};
use quire_semantic_value::checking::CheckingLimits;
use quire_semantic_value::loss::ValueLoss;

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
    compile_limited(declarations, DecimalLiteralLimits::default())
}

/// [`compile`] under the S3 decimal literal limits `limits`.
fn compile_limited(
    declarations: &str,
    limits: DecimalLiteralLimits,
) -> Result<CheckedPackage, Vec<CheckRefusal>> {
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
    let mut declarations = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        "fixture".to_owned(),
        unit,
        Vec::new(),
        Vec::new(),
    )
    .expect("the unit assembles");
    declarations.decimal_literal_limits = limits;
    declarations
        .check(
            quire_semantic_value::checking::CheckMode::Linked,
            CheckingLimits::default(),
        )
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
    /// The losses the call recorded, in order.
    losses: Vec<ValueLoss>,
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
        losses: evaluation
            .losses
            .into_iter()
            .map(|located| located.loss)
            .collect(),
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

/// `(exact_numerator, exact_denominator, rounded_coefficient, rounded_scale,
/// mode)` of a decimal loss record.
type LossFields = (Integer, Integer, Integer, u32, &'static str);

/// The refusal of a decimal result outside its declared domain.
const OUTSIDE_DOMAIN: (Option<&str>, Option<&str>) =
    (Some("decimal_out_of_domain"), Some("outside-domain"));

impl Run {
    /// The fields of the one decimal loss the call recorded.
    fn one_loss(&self) -> LossFields {
        let [ValueLoss::Decimal(loss)] = self.losses.as_slice() else {
            panic!("exactly one decimal loss, not {:?}", self.losses);
        };
        (
            loss.exact_numerator().clone(),
            loss.exact_denominator(),
            loss.rounded_coefficient().clone(),
            loss.rounded_scale(),
            loss.mode().as_str(),
        )
    }

    /// `(code, cause)` of the refusal the call ended with.
    fn refusal(&self) -> (Option<&'static str>, Option<&'static str>) {
        match &self.outcome {
            Outcome::Refused(refusal) => (refusal.code(), refusal.cause()),
            other => panic!("a refusal, not {other:?}"),
        }
    }

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
#[trace("QSpec-TC-185", "QSpec-FR-140-AC-7", "FR-093-AC-56", "TC-1232")]
#[test]
fn d24_a_rational_converts_to_a_decimal_as_the_division_at_the_target_scale() {
    let package = conversion("nearest-even");
    let run = call(&package, vec![rational(2, 3)], limits(9, 3, 2, 6));
    assert_eq!(run.decimal(), (int(67), 2));
    assert_eq!(run.one_loss(), (int(2), int(3), int(67), 2, "nearest-even"));
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
    assert_eq!(
        exact.refusal(),
        (Some("inexact_decimal"), Some("nonzero-discarded-digit"))
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
        assert!(run.losses.is_empty(), "{mode}: {:?}", run.losses);
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
    assert_eq!(run.one_loss(), (int(6), int(35), int(2), 1, "nearest-even"));
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
    // A quotient of two operands of one type is typed like `+`, `-` and `*`
    // (FR-104); only its nonzero obligation refuses.
    assert_eq!(
        refusal(&format!(
            "{wide}function d using v(a: P, b: P): Boolean pure {{ let s = a / b in s = a }}\n"
        )),
        unproved_nonzero()
    );
}

/// FR-104-AC-13 (TC-1084): the conversion, negation and mismatch forms.
/// `convert<W>(a) + convert<W>(b)` at `0.99 + 0.01` returns `decimal(100, 2)`
/// while `convert<W>(a + b)` is refused outside the domain of `P`; `-a = b`
/// checks; with `b: R`, `a / b = a` refuses `ill_typed`/`type-mismatch`.
#[trace("FR-104-AC-13", "TC-1084")]
#[test]
fn the_conversion_negation_and_mismatch_forms_of_the_criterion() {
    let types = "type P = Decimal[-99, 99; 2, 2; nearest-even];\n\
                 type R = Decimal[0, 1000; 1, 1; exact];\n\
                 type W = Decimal[-198, 198; 2, 2; nearest-even];\n";
    let decimal = |coefficient: i64| Value::Decimal(quire_exact::Decimal::new(int(coefficient), 2));
    let wide = compile(&format!(
        "{types}function f using v(a: P, b: P): W pure {{ convert<W>(a) + convert<W>(b) }}\n"
    ))
    .expect("the widened sum checks");
    assert_eq!(
        call(&wide, vec![decimal(99), decimal(1)], UNLIMITED).decimal(),
        (int(100), 2)
    );
    let narrow = compile(&format!(
        "{types}function f using v(a: P, b: P): W pure {{ convert<W>(a + b) }}\n"
    ))
    .expect("the narrow sum checks");
    let refused = call(&narrow, vec![decimal(99), decimal(1)], UNLIMITED);
    assert_eq!(refused.refusal(), OUTSIDE_DOMAIN);
    let negated = compile(&format!(
        "{types}function f using v(a: P, b: P): Boolean pure {{ -a = b }}\n"
    ))
    .expect("the negation checks");
    let run = call(&negated, vec![decimal(1), decimal(-1)], UNLIMITED);
    assert!(
        matches!(run.outcome, Outcome::Completed(Value::Boolean(true))),
        "{:?}",
        run.outcome
    );
    assert_eq!(
        refusal(&format!(
            "{types}function f using v(a: P, b: R): Boolean pure {{ a / b = a }}\n"
        )),
        mismatch()
    );
}

/// The `(kind, bound, actual)` of the one stage-limit refusal of a unit.
fn limit_refusal(
    declarations: &str,
    limits: DecimalLiteralLimits,
) -> (CheckingLimitKind, u64, u128) {
    let Err(refusals) = compile_limited(declarations, limits) else {
        panic!("{declarations} is refused");
    };
    let [refusal] = refusals.as_slice() else {
        panic!("one refusal, got {refusals:?}");
    };
    assert_eq!(refusal.cause.code().as_str(), "stage_limit_exceeded");
    assert_eq!(refusal.cause.cause(), Some("work-budget-exceeded"));
    let CheckCause::ResourceExhausted(stop) = &refusal.cause else {
        panic!("a stage limit, got {:?}", refusal.cause);
    };
    (stop.kind, stop.limit, stop.actual)
}

const Z: &str = "type Z = Decimal[-99, 99; 0, 2; nearest-even];\n";

fn zero_functions(count: usize, scale: u32) -> String {
    let mut text = String::from(Z);
    for index in 0..count {
        text.push_str(&format!(
            "function z{index} using v(): Z pure {{ decimal(0, {scale}) }}\n"
        ));
    }
    text
}

/// FR-093-AC-38, AC-45, AC-53: the written scale of one literal is limited by
/// `s3.decimal_literal_scale` (default 65535) before membership, as a
/// `stage_limit_exceeded`/`work-budget-exceeded` stop naming the bound and the
/// actual scale; a position that gives no type or the wrong type refuses
/// first.
#[trace("FR-093-AC-38", "FR-093-AC-45", "FR-093-AC-53", "TC-1232")]
#[test]
fn the_literal_scale_limit_refuses_before_membership() {
    let defaults = DecimalLiteralLimits::default();
    for scale in [65_534_u32, 65_535] {
        assert!(compile(&zero_functions(1, scale)).is_ok(), "{scale}");
    }
    for scale in [65_536_u32, u32::MAX] {
        assert_eq!(
            limit_refusal(&zero_functions(1, scale), defaults),
            (
                CheckingLimitKind::DecimalLiteralScale,
                65_535,
                u128::from(scale)
            ),
            "{scale}"
        );
    }
    // A nonzero coefficient is limited the same way, not refused as a
    // non-member.
    assert_eq!(
        limit_refusal(
            &format!("{Z}function z using v(): Z pure {{ decimal(1, 65536) }}\n"),
            defaults
        ),
        (CheckingLimitKind::DecimalLiteralScale, 65_535, 65_536)
    );
    let small = DecimalLiteralLimits {
        scale: 4,
        ..defaults
    };
    assert!(compile_limited(&zero_functions(1, 4), small).is_ok());
    assert_eq!(
        limit_refusal(&zero_functions(1, 5), small),
        (CheckingLimitKind::DecimalLiteralScale, 4, 5)
    );
    assert_eq!(
        refusal(&format!(
            "{Z}function z using v(): Z pure {{ let v = decimal(0, 65536) in v }}\n"
        )),
        ambiguous()
    );
    assert_eq!(
        refusal("function z using v(): Int[0, 9] pure { decimal(0, 65536) }\n"),
        mismatch()
    );
    // S2 builds the largest scale; S1 refuses one past u32::MAX.
    assert_eq!(
        limit_refusal(
            &format!("{Z}function z using v(): Z pure {{ decimal(1, 4294967295) }}\n"),
            defaults
        ),
        (
            CheckingLimitKind::DecimalLiteralScale,
            65_535,
            u128::from(u32::MAX)
        )
    );
}

/// FR-093-AC-52: the written scales of the literals of one checked package
/// total at most `s3.decimal_literal_scale_total` (default 1048576).
#[trace("FR-093-AC-52", "TC-1232")]
#[test]
fn the_literal_scale_total_limits_one_checked_package() {
    let defaults = DecimalLiteralLimits::default();
    assert!(compile(&zero_functions(16, 65_535)).is_ok());
    assert_eq!(
        limit_refusal(&zero_functions(17, 65_535), defaults),
        (
            CheckingLimitKind::DecimalLiteralScaleTotal,
            1_048_576,
            1_114_095
        )
    );
    let ten = DecimalLiteralLimits {
        total: 10,
        ..defaults
    };
    let twice = format!(
        "{Z}function a using v(): Z pure {{ decimal(0, 5) }}\n\
         function b using v(): Z pure {{ decimal(0, 5) }}\n"
    );
    assert!(compile_limited(&twice, ten).is_ok());
    assert_eq!(
        limit_refusal(
            &format!("{twice}function c using v(): Z pure {{ decimal(0, 1) }}\n"),
            ten
        ),
        (CheckingLimitKind::DecimalLiteralScaleTotal, 10, 11)
    );
}

/// FR-093-AC-57: a negated literal is typed by its position and its
/// operand's membership decides: `-decimal(5, 0)` and `decimal(5, 0)` at
/// `Decimal[-9, 0; 0, 0; toward-zero]` are refused (5 is no member);
/// `-decimal(-5, 0)` checks and its negated value 5 is outside the type at
/// evaluation; `-decimal(0, 0)` evaluates to 0; and `-decimal(5, 0)` at
/// `Decimal[-99999, 99999; 0, 2; nearest-even]` evaluates to -5.
#[trace("FR-093-AC-57", "TC-1232")]
#[test]
fn a_negated_literal_is_a_member_when_its_operand_is() {
    let n = "type N = Decimal[-9, 0; 0, 0; toward-zero];\n";
    let case = |body: &str| format!("{n}function f using v(): N pure {{ {body} }}\n");
    assert_eq!(refusal(&case("-decimal(5, 0)")), mismatch());
    assert_eq!(refusal(&case("decimal(5, 0)")), mismatch());
    let outside = compile(&case("-decimal(-5, 0)")).expect("the operand -5 is a member");
    let run = call(&outside, vec![], UNLIMITED);
    assert_eq!(run.refusal(), OUTSIDE_DOMAIN);
    let zero = compile(&case("-decimal(0, 0)")).expect("negative zero is a member");
    assert_eq!(call(&zero, vec![], UNLIMITED).decimal(), (int(0), 0));
    let wide = compile(
        "type Q = Decimal[-99999, 99999; 0, 2; nearest-even];\n\
         function f using v(): Q pure { -decimal(5, 0) }\n",
    )
    .expect("-decimal(5, 0) at Q checks");
    assert_eq!(call(&wide, vec![], UNLIMITED).decimal(), (int(-500), 2));
}

/// FR-104-AC-13 d, d2, e, f, g, h: a quotient by a nonzero literal checks and
/// evaluates; a quotient of two parameters is typed `P` and refuses only its
/// nonzero obligation, whatever its position.
#[trace("FR-104-AC-13", "TC-1084")]
#[test]
fn a_quotient_is_typed_by_its_operands_and_refuses_only_its_nonzero_obligation() {
    let p = "type P = Decimal[-99, 99; 2, 2; nearest-even];\n";
    let function = |name: &str, parameters: &str, result: &str, body: &str| {
        format!("{p}function {name} using v({parameters}): {result} pure {{ {body} }}\n")
    };
    let value = |coefficient: i64| Value::Decimal(quire_exact::Decimal::new(int(coefficient), 2));
    let d = compile(&function("f", "a: P", "P", "a / decimal(8, 2)")).expect("d checks");
    assert_eq!(call(&d, vec![value(1)], UNLIMITED).decimal(), (int(12), 2));
    let e = compile(&function("f", "a: P", "Boolean", "a / decimal(8, 2) = a")).expect("e checks");
    let _ = e;
    let d2 = compile(&function("f", "a: P", "P", "a / decimal(2, 2)")).expect("d2 checks");
    let run = call(&d2, vec![value(5)], UNLIMITED);
    assert_eq!(run.refusal(), OUTSIDE_DOMAIN);
    for (name, result, body) in [
        ("f", "P", "a / b"),
        ("g", "Boolean", "a / b = a"),
        ("h", "Boolean", "let s = a / b in s = a"),
    ] {
        assert_eq!(
            refusal(&function(name, "a: P, b: P", result, body)),
            unproved_nonzero(),
            "{name}"
        );
    }
}

/// FR-093-AC-41, AC-54, AC-55: only a `decimal(c, s)` literal of nonzero
/// coefficient discharges a divisor's obligation, at every scale.
#[trace("FR-093-AC-41", "FR-093-AC-54", "FR-093-AC-55", "TC-1232")]
#[test]
fn only_a_nonzero_literal_discharges_a_divisors_obligation() {
    let w = "type W = Decimal[-99999, 99999; 2, 2; nearest-even];\n";
    let function = |name: &str, parameters: &str, result: &str, body: &str| {
        format!("{w}function {name} using v({parameters}): {result} pure {{ {body} }}\n")
    };
    let x1 = compile(&function("f", "a: W", "W", "a / decimal(4, 0)")).expect("x1 checks");
    assert_eq!(
        call(
            &x1,
            vec![Value::Decimal(quire_exact::Decimal::new(int(600), 2))],
            UNLIMITED
        )
        .decimal(),
        (int(150), 2)
    );
    assert!(compile(&function("f", "a: W", "W", "a / decimal(400, 2)")).is_ok());
    assert!(compile(&function("f", "a: W", "Boolean", "a / decimal(4, 0) = a")).is_ok());
    for (name, parameters, result, body) in [
        ("y1", "a: W", "W", "a / decimal(0, 2)"),
        ("y2", "a: W", "W", "a / decimal(0, 0)"),
        ("y3", "a: W, b: W", "W", "a / b"),
        ("y4", "a: W, b: W", "Boolean", "a / b = a"),
        (
            "y5",
            "a: W, c: Boolean",
            "W",
            "a / (if c then decimal(5, 2) else decimal(6, 2))",
        ),
        ("y6", "a: W", "W", "a / -decimal(4, 0)"),
    ] {
        assert_eq!(
            refusal(&function(name, parameters, result, body)),
            unproved_nonzero(),
            "{name}"
        );
    }
}

const QUANTUM: &str = "type Q = Decimal[-99999, 99999; 0, 2; nearest-even];\n";

/// FR-093-AC-39 (TC-1232): the constructors at `Q` evaluate as written, and a
/// literal beside a typed operand takes its type: `c1` returns coefficient
/// 125, scale 2, and `add` at coefficient 100, scale 2 returns 225, scale 2.
/// The lowered node structure and IR's read are asserted with the emitter.
#[trace("FR-093-AC-39", "TC-1232")]
#[test]
fn the_constructors_at_q_evaluate_as_written() {
    let c1 = compile(&format!(
        "{QUANTUM}function f using v(): Q pure {{ decimal(125, 2) }}\n"
    ))
    .expect("c1 checks");
    assert_eq!(call(&c1, vec![], UNLIMITED).decimal(), (int(125), 2));
    let add = compile(&format!(
        "{QUANTUM}function f using v(p: Q): Q pure {{ p + decimal(125, 2) }}\n"
    ))
    .expect("add checks");
    let p = Value::Decimal(quire_exact::Decimal::new(int(100), 2));
    assert_eq!(call(&add, vec![p], UNLIMITED).decimal(), (int(225), 2));
}

/// FR-093-AC-40 (TC-1232): a literal takes only a decimal type it is a
/// member of by value; the non-members refuse `ill_typed`/`type-mismatch` at
/// the literal, and the members by value check.
#[trace("FR-093-AC-40", "TC-1232")]
#[test]
fn a_literal_that_is_no_member_is_refused_and_a_member_by_value_checks() {
    let big = format!("1{}", "0".repeat(41));
    for (name, result, body) in [
        ("a2", "Int[0, 9]".to_owned(), "decimal(1, 0)".to_owned()),
        ("a3", "Boolean".to_owned(), "decimal(1, 0)".to_owned()),
        (
            "a4",
            "Decimal[-9, 9; 0, 0; toward-zero]".to_owned(),
            "decimal(150, 2)".to_owned(),
        ),
        (
            "a5",
            "Decimal[-1000, 1000; 0, 1; nearest-even]".to_owned(),
            "decimal(1234, 3)".to_owned(),
        ),
        (
            "a6",
            "Decimal[-9, 9; 0, 0; toward-zero]".to_owned(),
            "decimal(10, 0)".to_owned(),
        ),
        ("a7", "Q".to_owned(), format!("decimal({big}, 2)")),
    ] {
        assert_eq!(
            refusal(&format!(
                "{QUANTUM}function {name} using v(): {result} pure {{ {body} }}\n"
            )),
            mismatch(),
            "{name}"
        );
    }
    for (result, body) in [
        ("Decimal[-9, 9; 0, 0; toward-zero]", "decimal(100, 2)"),
        (
            "Decimal[-1000, 1000; 0, 1; nearest-even]",
            "decimal(1200, 3)",
        ),
        ("Decimal[-9, 9; 0, 0; toward-zero]", "decimal(0, 7)"),
    ] {
        compile(&format!(
            "function f using v(): {result} pure {{ {body} }}\n"
        ))
        .unwrap_or_else(|refusals| panic!("{body} is a member by value: {refusals:?}"));
    }
}

/// FR-093-AC-42 (TC-1232): a `Float64` is no operand of a decimal
/// conversion (`ill_typed`/`type-mismatch`, FR-397-AC-18).
#[trace("FR-093-AC-42", "TC-1232")]
#[test]
fn a_float_converts_to_no_decimal() {
    let text = "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n\
         function f using v(a: Float64[nearest-even]): Decimal[0, 100; 0, 3; nearest-even] pure \
         { convert<Decimal[0, 100; 0, 3; nearest-even]>(a) }\n";
    let parsed = qsl_cst::parse(
        qsl_foundation::SourceIdentity::new("a", "u"),
        "unit.native",
        text.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    let unit = qsl_forms::build_unit(&parsed).expect("S2 builds the unit");
    let mut declarations = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        "fixture".to_owned(),
        unit,
        Vec::new(),
        Vec::new(),
    )
    .expect("the unit assembles");
    declarations.ieee_profile = Some(
        qsl_semantics::value::admit_ieee_profile(
            &[qsl_semantics::value::DefinitionRole::IeeeProfile.reference()],
            &[],
        )
        .expect("the pinned IEEE profile admits"),
    );
    let Err(refusals) = declarations.check(
        quire_semantic_value::checking::CheckMode::Linked,
        CheckingLimits::default(),
    ) else {
        panic!("a float to decimal conversion is refused");
    };
    let [refusal] = refusals.as_slice() else {
        panic!("one refusal, got {refusals:?}");
    };
    assert_eq!(refusal.cause.code().as_str(), "ill_typed");
    assert_eq!(refusal.cause.cause(), Some("type-mismatch"));
}

/// FR-093-AC-53 (TC-1232): a unit declaration's `decimal(1, 5000)` is refused
/// by `s3.decimal_scale` with bound 4096, not by either literal setting.
#[trace("FR-093-AC-53", "TC-1232")]
#[test]
fn a_unit_declarations_scale_is_limited_by_the_assembly_setting() {
    let text = "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n\
         dimension Length;\nunit m : Length = rational(1, 1);\n\
         unit c : Length = decimal(1, 5000) * m;\n";
    let parsed = qsl_cst::parse(
        qsl_foundation::SourceIdentity::new("a", "u"),
        "unit.native",
        text.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    let unit = qsl_forms::build_unit(&parsed).expect("S2 builds the unit");
    let Err(refusal) = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        "fixture".to_owned(),
        unit,
        Vec::new(),
        Vec::new(),
    ) else {
        panic!("scale 5000 is above the assembly bound");
    };
    let [error] = refusal.errors.as_slice() else {
        panic!("one error, not {:?}", refusal.errors);
    };
    let AssemblyCause::DecimalScaleLimit(limit) = &error.cause else {
        panic!("a decimal-scale limit, not {:?}", error.cause);
    };
    assert_eq!(limit.configured_bound(), 4096);
    assert_eq!(limit.actual(), 5000);
    assert_eq!(limit.setting(), qsl_foundation::Setting::S3DecimalScale);
}
