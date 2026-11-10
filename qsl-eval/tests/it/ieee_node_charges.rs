// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-397-AC-27 and FR-397-AC-28 (TC-1091 steps 22 and 23): the IEEE width
//! conversion, exact-to-IEEE conversion, IEEE-to-exact conversion, IEEE
//! negation and the standalone numeric-equality node evaluate through the
//! owned `quire_exact` operation and charge exactly its `ieee.*` points, read
//! from the meter's charge log under the row's limit tuple.

use ix_trace_rs::trace;
use qsl_eval::value::{CheckedPackageEvaluation, QualifiedName};
use qsl_package::CheckedPackage;
use qsl_semantics::check::PackageDeclarations;
use qsl_semantics::model::object_environment::ObjectEnvironment;
use qsl_semantics::value::{admit_ieee_profile, DefinitionRole};
use quire_eval::FamilyOutcome;
use quire_exact::{
    compare_ieee, ChargePoint, Decimal, IeeeComparison, IeeeFlag, IeeeFlags, IeeeValue, Incomplete,
    Integer, LimitKind, Meter, Outcome, Rational, Refusal, ScalarLimits, Undefined, Value,
};
use quire_semantic_value::checking::CheckMode;
use quire_semantic_value::checking::CheckingLimits;
use quire_semantic_value::loss::ValueLoss;

/// `I(i,o,w,r)` of QSpec TC-193: integer bits, value occurrences, work units
/// and result units; every other limit is zero.
fn limits(integer_bits: u64, occurrences: u64, work: u64, results: u64) -> ScalarLimits {
    ScalarLimits {
        integer_bits,
        decimal_digits: 0,
        scale_expansion: 0,
        text_input_bytes: 0,
        text_scalars: 0,
        normalized_scalars: 0,
        unit_edges: 0,
        value_occurrences: occurrences,
        work_units: work,
        result_units: results,
    }
}

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

struct Run {
    outcome: Outcome<Value>,
    charges: Vec<ChargePoint>,
    flags: Vec<IeeeFlag>,
    consumed: [u64; 10],
}

/// A function body that reads the node's operands and does nothing else: the
/// call's own charges, which come before the node and are not its budget.
type Baseline<'a> = (&'a str, &'a str);

/// Evaluates the node with `row` as the meter's remaining budget at the node:
/// the budget is the row plus whatever the call spends before it, and the
/// outcome and the charge log are read back relative to the node.
fn run(
    params: &str,
    result: &str,
    body: &str,
    arguments: Vec<Value>,
    row: ScalarLimits,
    base: Baseline<'_>,
) -> Run {
    let before = evaluate(params, base.0, base.1, arguments.clone(), UNLIMITED);
    let row_kinds = [
        row.integer_bits,
        row.decimal_digits,
        row.scale_expansion,
        row.text_input_bytes,
        row.text_scalars,
        row.normalized_scalars,
        row.unit_edges,
        row.value_occurrences,
        row.work_units,
        row.result_units,
    ];
    let mut sum = [0_u64; 10];
    for (slot, (row, before)) in sum.iter_mut().zip(row_kinds.iter().zip(before.consumed)) {
        *slot = row + before;
    }
    let mut at_node = evaluate(params, result, body, arguments, limits_of(sum));
    at_node.charges.drain(..before.charges.len());
    if let Outcome::Incomplete(incomplete) = &mut at_node.outcome {
        let index = LimitKind::ALL
            .iter()
            .position(|kind| *kind == incomplete.limit_kind)
            .expect("a limit kind");
        incomplete.limit -= before.consumed[index];
        incomplete.consumed -= before.consumed[index];
    }
    at_node
}

fn limits_of(kinds: [u64; 10]) -> ScalarLimits {
    ScalarLimits {
        integer_bits: kinds[0],
        decimal_digits: kinds[1],
        scale_expansion: kinds[2],
        text_input_bytes: kinds[3],
        text_scalars: kinds[4],
        normalized_scalars: kinds[5],
        unit_edges: kinds[6],
        value_occurrences: kinds[7],
        work_units: kinds[8],
        result_units: kinds[9],
    }
}

/// Checks `function f using v(<params>): <result> pure { <body> }` and
/// evaluates it at `arguments` with `limits` as the meter's budget.
fn evaluate(
    params: &str,
    result: &str,
    body: &str,
    arguments: Vec<Value>,
    limits: ScalarLimits,
) -> Run {
    let text = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n\
         function f using v({params}): {result} pure {{ {body} }}\n"
    );
    let parsed = qsl_cst::parse(
        qsl_foundation::SourceIdentity::new("a", "u"),
        "unit.native",
        text.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    assert!(parsed.is_admissible(), "{text}\n{:?}", parsed.diagnostics());
    let unit = qsl_forms::build_unit(&parsed).expect("S2 builds the unit");
    let mut declarations = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        "fixture".to_owned(),
        unit,
        Vec::new(),
        Vec::new(),
    )
    .expect("the assembler admits the unit");
    declarations.ieee_profile =
        Some(admit_ieee_profile(&[DefinitionRole::IeeeProfile.reference()], &[]).unwrap());
    let package = CheckedPackage::link(
        declarations
            .check(CheckingLimits::default(), CheckMode::Linked)
            .unwrap_or_else(|refusal| panic!("{body}: {refusal:?}")),
    );
    let mut meter = Meter::new(limits);
    let evaluation = package
        .call(
            &QualifiedName::unqualified("f").expect("f is an identifier"),
            arguments,
            &ObjectEnvironment::default(),
            &mut meter,
        )
        .expect("the call runs");
    let FamilyOutcome::Evaluated(outcome) = evaluation.outcome else {
        panic!("a kernel outcome, not {:?}", evaluation.outcome);
    };
    let flags = evaluation
        .losses
        .into_iter()
        .filter_map(|each| match each.loss {
            ValueLoss::IeeeFlags(flags) => Some(flags),
            _ => None,
        })
        .flat_map(IeeeFlags::iter)
        .collect();
    Run {
        outcome,
        charges: meter
            .admitted_charges()
            .iter()
            .map(|charge| charge.point)
            .collect(),
        flags,
        consumed: LimitKind::ALL.map(|kind| meter.consumed(kind)),
    }
}

fn f32v(bits: u32) -> Value {
    Value::Float(IeeeValue::binary32(bits))
}

fn f64v(bits: u64) -> Value {
    Value::Float(IeeeValue::binary64(bits))
}

fn incomplete(
    kind: LimitKind,
    limit: u64,
    consumed: u64,
    next: u64,
    point: ChargePoint,
) -> Outcome<Value> {
    Outcome::Incomplete(Incomplete {
        limit_kind: kind,
        limit,
        consumed,
        next_charge: Integer::from(next),
        charge_point: point,
    })
}

/// `Value` has no equality; outcomes compare by their rendering.
fn same(actual: &Outcome<Value>, expected: Outcome<Value>) {
    assert_eq!(format!("{actual:?}"), format!("{expected:?}"));
}

/// A refusal's code and cause, as the kernel names them.
fn refusal_of(outcome: &Outcome<Value>) -> (&'static str, &'static str) {
    match outcome {
        Outcome::Refused(refusal) => (
            refusal.code().expect("a coded refusal"),
            refusal.cause().expect("a refusal cause"),
        ),
        other => panic!("a refusal, not {other:?}"),
    }
}

/// The flags of a run, in vocabulary order, compared as a set.
fn assert_flags(run: &Run, expected: &[IeeeFlag]) {
    let mut seen = run.flags.clone();
    seen.sort();
    let mut expected = expected.to_vec();
    expected.sort();
    assert_eq!(seen, expected);
}

/// The decimal literal of `2^exponent`.
fn power_of_two_literal(exponent: u32) -> String {
    let mut digits = vec![1_u8];
    for _ in 0..exponent {
        let mut carry = 0;
        for digit in &mut digits {
            let doubled = *digit * 2 + carry;
            *digit = doubled % 10;
            carry = doubled / 10;
        }
        if carry > 0 {
            digits.push(carry);
        }
    }
    digits
        .iter()
        .rev()
        .map(|digit| char::from(b'0' + digit))
        .collect()
}

const DECIMAL: &str = "Decimal[0, 100; 0, 2; nearest-even]";
const F32: &str = "Float32[nearest-even]";
const F64: &str = "Float64[nearest-even]";

/// F32: IEEE negation flips only the sign bit, under every rounding mode,
/// including signed zero and signaling NaN encodings; applying it twice
/// restores every source bit pattern.
#[trace("QSpec-TC-193", "FR-397-AC-27")]
#[test]
fn f32_ieee_negation_preserves_all_non_sign_bits_under_every_rounding_mode() {
    let cases = [
        (0x0000_0000, 0x8000_0000),
        (0x8000_0000, 0x0000_0000),
        (0x0000_0001, 0x8000_0001),
        (0x3f80_0000, 0xbf80_0000),
        (0x7f80_0000, 0xff80_0000),
        (0x7fc0_0012, 0xffc0_0012),
        (0x7f80_0001, 0xff80_0001),
    ];
    for mode in ["nearest-even", "toward-zero", "exact"] {
        let float = format!("Float32[{mode}]");
        for (source, expected) in cases {
            let negated = run(
                &format!("a: {float}"),
                &float,
                "-a",
                vec![f32v(source)],
                limits(32, 1, 2, 1),
                (&float, "a"),
            );
            same(&negated.outcome, Outcome::Completed(f32v(expected)));
            assert!(negated.flags.is_empty(), "{mode}, {source:#010x}");
            assert_eq!(
                negated.charges,
                [ChargePoint::IeeeOperands, ChargePoint::IeeeResultRetain],
                "{mode}, {source:#010x}"
            );

            let twice = run(
                &format!("a: {float}"),
                &float,
                "-(-a)",
                vec![f32v(source)],
                limits(64, 2, 4, 2),
                (&float, "a"),
            );
            same(&twice.outcome, Outcome::Completed(f32v(source)));
            assert!(twice.flags.is_empty(), "{mode}, {source:#010x}");
            assert_eq!(
                twice.charges,
                [
                    ChargePoint::IeeeOperands,
                    ChargePoint::IeeeResultRetain,
                    ChargePoint::IeeeOperands,
                    ChargePoint::IeeeResultRetain,
                ],
                "{mode}, {source:#010x}"
            );
        }
    }
}

/// F33: negation charges the operand and retained result in order, and
/// refuses to produce a value when either charge cannot be paid.
#[trace("QSpec-TC-193", "FR-397-AC-27")]
#[test]
fn f33_ieee_negation_obeys_operand_and_result_meter_limits() {
    let complete = run(
        "a: Float32[nearest-even]",
        F32,
        "-a",
        vec![f32v(0x3f80_0000)],
        limits(32, 1, 2, 1),
        (F32, "a"),
    );
    same(&complete.outcome, Outcome::Completed(f32v(0xbf80_0000)));
    assert!(complete.flags.is_empty());
    assert_eq!(
        complete.charges,
        [ChargePoint::IeeeOperands, ChargePoint::IeeeResultRetain]
    );

    let retain = run(
        "a: Float32[nearest-even]",
        F32,
        "-a",
        vec![f32v(0x3f80_0000)],
        limits(32, 1, 1, 1),
        (F32, "a"),
    );
    same(
        &retain.outcome,
        incomplete(LimitKind::WorkUnits, 1, 1, 1, ChargePoint::IeeeResultRetain),
    );
    assert_eq!(retain.charges, [ChargePoint::IeeeOperands]);

    let operands = run(
        "a: Float32[nearest-even]",
        F32,
        "-a",
        vec![f32v(0x3f80_0000)],
        limits(32, 0, 2, 1),
        (F32, "a"),
    );
    same(
        &operands.outcome,
        incomplete(
            LimitKind::ValueOccurrences,
            0,
            0,
            1,
            ChargePoint::IeeeOperands,
        ),
    );
    assert!(operands.charges.is_empty());
}

/// AC-21: binary64 negation preserves the source mode and flips only the sign
/// bit, including for infinities, NaN payloads and signed zero.
#[trace("QSpec-TC-193", "FR-397-AC-21", "FR-397-AC-27")]
#[test]
fn f64_ieee_negation_preserves_the_source_mode_and_payload() {
    let cases = [
        (0x0000_0000_0000_0000, 0x8000_0000_0000_0000),
        (0x8000_0000_0000_0000, 0x0000_0000_0000_0000),
        (0x0000_0000_0000_0001, 0x8000_0000_0000_0001),
        (0x3ff0_0000_0000_0000, 0xbff0_0000_0000_0000),
        (0x7ff0_0000_0000_0000, 0xfff0_0000_0000_0000),
        (0x7ff8_0000_0000_0012, 0xfff8_0000_0000_0012),
        (0x7ff0_0000_0000_0001, 0xfff0_0000_0000_0001),
    ];
    for mode in ["nearest-even", "toward-zero", "exact"] {
        let float = format!("Float64[{mode}]");
        for (source, expected) in cases {
            let negated = run(
                &format!("a: {float}"),
                &float,
                "-a",
                vec![f64v(source)],
                limits(64, 1, 2, 1),
                (&float, "a"),
            );
            same(&negated.outcome, Outcome::Completed(f64v(expected)));
            assert!(negated.flags.is_empty(), "{mode}, {source:#018x}");
            assert_eq!(
                negated.charges,
                [ChargePoint::IeeeOperands, ChargePoint::IeeeResultRetain],
                "{mode}, {source:#018x}"
            );

            let twice = run(
                &format!("a: {float}"),
                &float,
                "-(-a)",
                vec![f64v(source)],
                limits(128, 2, 4, 2),
                (&float, "a"),
            );
            same(&twice.outcome, Outcome::Completed(f64v(source)));
            assert!(twice.flags.is_empty(), "{mode}, {source:#018x}");
            assert_eq!(
                twice.charges,
                [
                    ChargePoint::IeeeOperands,
                    ChargePoint::IeeeResultRetain,
                    ChargePoint::IeeeOperands,
                    ChargePoint::IeeeResultRetain,
                ],
                "{mode}, {source:#018x}"
            );
        }
    }
}

/// F24 and F25: a width conversion rounds once through
/// `convert_ieee_width`, keeps a NaN's sign and payload, and refuses a
/// payload the target cannot hold after `ieee.operands` only.
#[trace("TC-1091", "FR-397-AC-27")]
#[test]
fn f24_f25_width_conversion_evaluates_and_charges_through_the_owned_operation() {
    let widen = |bits, budget| {
        run(
            &format!("a: {F32}"),
            F64,
            &format!("convert<{F64}>(a)"),
            vec![f32v(bits)],
            budget,
            (F32, "a"),
        )
    };
    let wide = widen(0x3f80_0000, limits(64, 1, 4, 1));
    same(
        &wide.outcome,
        Outcome::Completed(f64v(0x3ff0_0000_0000_0000)),
    );
    assert!(wide.flags.is_empty());
    same(
        &widen(0x3f80_0000, limits(32, 1, 4, 1)).outcome,
        incomplete(
            LimitKind::IntegerBits,
            32,
            32,
            64,
            ChargePoint::IeeeExactIntermediate,
        ),
    );
    // The widening of the quiet NaN keeps its payload and raises nothing.
    let nan = widen(0x7fc0_0001, limits(32, 1, 2, 1));
    same(
        &nan.outcome,
        Outcome::Completed(f64v(0x7ff8_0000_0000_0001)),
    );
    assert!(nan.flags.is_empty());

    let narrow = |bits, budget| {
        run(
            &format!("a: {F64}"),
            F32,
            &format!("convert<{F32}>(a)"),
            vec![f64v(bits)],
            budget,
            (F64, "a"),
        )
    };
    let signaling = narrow(0x7ff0_0000_0000_0001, limits(64, 1, 2, 1));
    same(&signaling.outcome, Outcome::Completed(f32v(0x7fc0_0001)));
    assert_eq!(signaling.flags, [IeeeFlag::Invalid]);
    let negative = narrow(0xfff8_0000_0000_0003, limits(64, 1, 2, 1));
    same(&negative.outcome, Outcome::Completed(f32v(0xffc0_0003)));
    assert!(negative.flags.is_empty());
    // Payload 2^22 does not fit binary32: refused after `ieee.operands`,
    // before `ieee.result-retain`; the signaling source raises no `invalid`.
    for bits in [0x7ff8_0000_0040_0000, 0x7ff0_0000_0040_0000] {
        let refused = narrow(bits, limits(64, 1, 1, 0));
        assert_eq!(
            refusal_of(&refused.outcome),
            (
                "ieee_nan_payload_not_representable",
                "payload-exceeds-target"
            ),
            "{bits:#x}"
        );
        assert!(refused.flags.is_empty(), "{bits:#x}");
        assert_eq!(refused.charges, [ChargePoint::IeeeOperands], "{bits:#x}");
    }
    same(
        &narrow(0x7ff8_0000_0040_0000, limits(64, 1, 0, 0)).outcome,
        incomplete(LimitKind::WorkUnits, 0, 0, 1, ChargePoint::IeeeOperands),
    );
    // F31: charges of a finite narrowing, then strict `exact`.
    let tenth = narrow(0x3fb9_9999_9999_999a, limits(64, 1, 4, 1));
    same(&tenth.outcome, Outcome::Completed(f32v(0x3dcc_cccd)));
    assert_eq!(tenth.flags, [IeeeFlag::Inexact]);
    let strict = |budget| {
        run(
            &format!("a: {F64}"),
            "Float32[exact]",
            "convert<Float32[exact]>(a)",
            vec![f64v(0x3fb9_9999_9999_999a)],
            budget,
            (F64, "a"),
        )
    };
    let refused = strict(limits(64, 1, 3, 1));
    assert_eq!(
        refusal_of(&refused.outcome),
        ("ieee_not_exact", "rounding-required")
    );
    let Outcome::Refused(Refusal::IeeeNotExact { would_be, .. }) = refused.outcome else {
        panic!("ieee_not_exact, not {:?}", refused.outcome);
    };
    assert_eq!(would_be.iter().collect::<Vec<_>>(), [IeeeFlag::Inexact]);
    assert_eq!(refused.charges.last(), Some(&ChargePoint::IeeeRound));
    same(
        &strict(limits(64, 1, 2, 1)).outcome,
        incomplete(LimitKind::WorkUnits, 2, 2, 1, ChargePoint::IeeeRound),
    );
    // F24: the binary64 one narrows under I(32,1,4,1) only after the operand
    // charge, which already needs 64 bits.
    same(
        &narrow(0x3ff0_0000_0000_0000, limits(32, 1, 4, 1)).outcome,
        incomplete(LimitKind::IntegerBits, 32, 0, 64, ChargePoint::IeeeOperands),
    );
    // F31: the binary32 maximum is exact; one rounding step above it with
    // the odd-significand tie overflows to infinity.
    let maximum = narrow(0x47ef_ffff_e000_0000, limits(64, 1, 4, 1));
    same(&maximum.outcome, Outcome::Completed(f32v(0x7f7f_ffff)));
    assert_flags(&maximum, &[]);
    let overflow = narrow(0x47ef_ffff_f000_0000, limits(64, 1, 4, 1));
    same(&overflow.outcome, Outcome::Completed(f32v(0x7f80_0000)));
    assert_flags(&overflow, &[IeeeFlag::Overflow, IeeeFlag::Inexact]);
    same(
        &narrow(0x47ef_ffff_f000_0000, limits(64, 1, 3, 1)).outcome,
        incomplete(LimitKind::WorkUnits, 3, 3, 1, ChargePoint::IeeeResultRetain),
    );
    let toward_zero = run(
        &format!("a: {F64}"),
        "Float32[toward-zero]",
        "convert<Float32[toward-zero]>(a)",
        vec![f64v(0x47ef_ffff_f000_0000)],
        limits(64, 1, 4, 1),
        (F64, "a"),
    );
    same(&toward_zero.outcome, Outcome::Completed(f32v(0x7f7f_ffff)));
    assert_flags(&toward_zero, &[IeeeFlag::Inexact]);
    let tiny = narrow(0x0000_0000_0000_0001, limits(64, 1, 4, 1));
    same(&tiny.outcome, Outcome::Completed(f32v(0x0000_0000)));
    assert_flags(&tiny, &[IeeeFlag::Underflow, IeeeFlag::Inexact]);
    same(
        &narrow(0x0000_0000_0000_0001, limits(64, 1, 3, 1)).outcome,
        incomplete(LimitKind::WorkUnits, 3, 3, 1, ChargePoint::IeeeResultRetain),
    );
    let infinity = narrow(0xfff0_0000_0000_0000, limits(64, 1, 2, 1));
    same(&infinity.outcome, Outcome::Completed(f32v(0xff80_0000)));
    assert_eq!(
        infinity.charges,
        [ChargePoint::IeeeOperands, ChargePoint::IeeeResultRetain]
    );
    same(
        &narrow(0xfff0_0000_0000_0000, limits(64, 1, 1, 1)).outcome,
        incomplete(LimitKind::WorkUnits, 1, 1, 1, ChargePoint::IeeeResultRetain),
    );
}

/// F28 and the F24 rational: an exact operand rounds once under the target's
/// mode and a decimal is sized by its retained coefficient and scale.
#[trace("TC-1091", "FR-397-AC-27")]
#[test]
fn f28_exact_to_ieee_conversion_sizes_a_decimal_by_its_retained_representation() {
    let decimal = || Value::Decimal(Decimal::new(Integer::from(100_i64), 2));
    let convert = |budget| {
        run(
            "d: Decimal[0, 100; 0, 2; nearest-even]",
            F32,
            &format!("convert<{F32}>(d)"),
            vec![decimal()],
            budget,
            (DECIMAL, "d"),
        )
    };
    let done = convert(limits(32, 1, 4, 1));
    same(&done.outcome, Outcome::Completed(f32v(0x3f80_0000)));
    assert!(done.flags.is_empty());
    assert_eq!(
        done.charges,
        [
            ChargePoint::IeeeOperands,
            ChargePoint::IeeeExactIntermediate,
            ChargePoint::IeeeRound,
            ChargePoint::IeeeResultRetain
        ]
    );
    same(
        &convert(limits(6, 1, 4, 1)).outcome,
        incomplete(LimitKind::IntegerBits, 6, 0, 7, ChargePoint::IeeeOperands),
    );

    let third = |budget| {
        run(
            "r: Rational[0, 1; 1, 3]",
            F32,
            &format!("convert<{F32}>(r)"),
            vec![Value::Rational(
                Rational::new(Integer::from(1_i64), Integer::from(3_i64)).unwrap(),
            )],
            budget,
            ("Rational[0, 1; 1, 3]", "r"),
        )
    };
    let rounded = third(limits(32, 1, 4, 1));
    same(&rounded.outcome, Outcome::Completed(f32v(0x3eaa_aaab)));
    assert_eq!(rounded.flags, [IeeeFlag::Inexact]);
    same(
        &third(limits(2, 1, 4, 1)).outcome,
        incomplete(
            LimitKind::IntegerBits,
            2,
            2,
            32,
            ChargePoint::IeeeExactIntermediate,
        ),
    );
}

/// F24: the IEEE-to-exact conversion of `1/2` and of a NaN.
#[trace("TC-1091", "FR-397-AC-27")]
#[test]
fn f24_ieee_to_exact_conversion_of_a_half_and_of_a_nan() {
    let to_half = |bits, budget| {
        run(
            &format!("a: {F32}"),
            "Rational[0, 1; 1, 2]",
            "convert<Rational[0, 1; 1, 2]>(a)",
            vec![f32v(bits)],
            budget,
            (F32, "a"),
        )
    };
    let done = to_half(0x3f00_0000, limits(32, 1, 3, 1));
    same(&done.outcome, Outcome::Completed(rational(1, 1)));
    assert_eq!(
        done.charges,
        [
            ChargePoint::IeeeOperands,
            ChargePoint::IeeeExactIntermediate,
            ChargePoint::IeeeResultRetain
        ]
    );
    let nan = to_half(0x7fc0_0000, limits(32, 1, 2, 1));
    same(&nan.outcome, Outcome::Undefined(Undefined::IeeeNotFinite));
    assert_eq!(nan.charges, [ChargePoint::IeeeOperands]);
}

/// `numerator / 2^power` as a rational value.
fn rational(numerator: i64, power: i64) -> Value {
    Value::Rational(
        Rational::new(
            Integer::from(numerator),
            Integer::from(2_i64).pow(&Integer::from(power)),
        )
        .unwrap(),
    )
}

/// F27: the IEEE-to-exact conversion at its limits: the smallest binary64
/// subnormal, a negative zero, an infinity, and a value outside the target
/// domain.
#[trace("TC-1091", "FR-397-AC-27")]
#[test]
fn f27_ieee_to_exact_conversion_charges_and_refuses_through_the_owned_operation() {
    // 2^-1074 into `Rational[0,1;1,2^1074]`.
    let domain = format!("Rational[0, 1; 1, {}]", power_of_two_literal(1074));
    let tiny = |budget| {
        run(
            &format!("a: {F64}"),
            &domain,
            &format!("convert<{domain}>(a)"),
            vec![f64v(1)],
            budget,
            (F64, "a"),
        )
    };
    let exact = tiny(limits(1075, 1, 3, 1));
    same(&exact.outcome, Outcome::Completed(rational(1, 1074)));
    assert_eq!(
        exact.charges,
        [
            ChargePoint::IeeeOperands,
            ChargePoint::IeeeExactIntermediate,
            ChargePoint::IeeeResultRetain
        ]
    );
    same(
        &tiny(limits(1074, 1, 3, 1)).outcome,
        incomplete(
            LimitKind::IntegerBits,
            1074,
            64,
            1075,
            ChargePoint::IeeeExactIntermediate,
        ),
    );

    // Negative zero into `Rational[0,0;1,1]` is the exact 0.
    let zero = |budget| {
        run(
            &format!("a: {F32}"),
            "Rational[0, 0; 1, 1]",
            "convert<Rational[0, 0; 1, 1]>(a)",
            vec![f32v(0x8000_0000)],
            budget,
            (F32, "a"),
        )
    };
    same(
        &zero(limits(32, 1, 3, 1)).outcome,
        Outcome::Completed(rational(0, 0)),
    );
    same(
        &zero(limits(32, 1, 2, 1)).outcome,
        incomplete(LimitKind::WorkUnits, 2, 2, 1, ChargePoint::IeeeResultRetain),
    );

    // An infinity is undefined after `ieee.operands` only.
    let infinity = |budget| {
        run(
            &format!("a: {F32}"),
            "Rational[0, 1; 1, 2]",
            "convert<Rational[0, 1; 1, 2]>(a)",
            vec![f32v(0x7f80_0000)],
            budget,
            (F32, "a"),
        )
    };
    let undefined = infinity(limits(32, 1, 1, 0));
    same(
        &undefined.outcome,
        Outcome::Undefined(Undefined::IeeeNotFinite),
    );
    assert_eq!(undefined.charges, [ChargePoint::IeeeOperands]);
    same(
        &infinity(limits(32, 1, 0, 0)).outcome,
        incomplete(LimitKind::WorkUnits, 0, 0, 1, ChargePoint::IeeeOperands),
    );

    // `1/2` is outside `Rational[0,1;1,1]`: refused after
    // `ieee.exact-intermediate`, before `ieee.result-retain`.
    let outside = |budget| {
        run(
            &format!("a: {F32}"),
            "Rational[0, 1; 1, 1]",
            "convert<Rational[0, 1; 1, 1]>(a)",
            vec![f32v(0x3f00_0000)],
            budget,
            (F32, "a"),
        )
    };
    let refused = outside(limits(32, 1, 2, 0));
    assert_eq!(
        refusal_of(&refused.outcome),
        ("ieee_rational_out_of_domain", "outside-domain")
    );
    assert_eq!(
        refused.charges,
        [
            ChargePoint::IeeeOperands,
            ChargePoint::IeeeExactIntermediate
        ]
    );
    same(
        &outside(limits(32, 1, 1, 0)).outcome,
        incomplete(
            LimitKind::WorkUnits,
            1,
            1,
            1,
            ChargePoint::IeeeExactIntermediate,
        ),
    );
}

/// FR-397-AC-18: a conversion to the same width under another mode keeps the
/// operand's exact bits (a signaling NaN stays signaling), raises no flag and
/// charges nothing, under every target mode including `exact`.
#[trace("TC-1091", "FR-397-AC-18")]
#[test]
fn same_width_conversion_keeps_the_exact_bits_and_charges_nothing() {
    let big = limits(1 << 40, 1 << 40, 1 << 40, 1 << 40);
    for target in ["Float64[toward-zero]", "Float64[exact]"] {
        for bits in [
            0x3ff0_0000_0000_0001_u64,
            0x8000_0000_0000_0000,
            0xfff0_0000_0000_0000,
            0x7ff0_0000_0000_0001,
            0xfff8_0000_0000_0005,
        ] {
            let converted = run(
                &format!("a: {F64}"),
                target,
                &format!("convert<{target}>(a)"),
                vec![f64v(bits)],
                big,
                (F64, "a"),
            );
            same(&converted.outcome, Outcome::Completed(f64v(bits)));
            assert!(converted.flags.is_empty(), "{target} {bits:#x}");
            assert!(converted.charges.is_empty(), "{target} {bits:#x}");
            let Outcome::Completed(Value::Float(result)) = converted.outcome else {
                unreachable!()
            };
            assert_eq!(result.is_signaling_nan(), bits == 0x7ff0_0000_0000_0001);
        }
    }
    let narrow = run(
        "a: Float32[toward-zero]",
        "Float32[exact]",
        "convert<Float32[exact]>(a)",
        vec![f32v(0x7f80_0001)],
        big,
        ("Float32[toward-zero]", "a"),
    );
    same(&narrow.outcome, Outcome::Completed(f32v(0x7f80_0001)));
    assert!(narrow.flags.is_empty() && narrow.charges.is_empty());
}

/// FR-397-AC-17: the values of a conversion into a float.
#[trace("TC-1091", "FR-397-AC-17")]
#[test]
fn ac17_conversion_into_a_float_gives_the_published_bits() {
    let big = limits(1 << 40, 1 << 40, 1 << 40, 1 << 40);
    let bits = |name: &'static str, ty: &'static str, target: &str, argument: Value| {
        run(
            &format!("{name}: {ty}"),
            target,
            &format!("convert<{target}>({name})"),
            vec![argument],
            big,
            (ty, name),
        )
        .outcome
    };
    let integer = |text: &str| Value::Integer(text.parse::<Integer>().expect("an integer"));
    let tenth = Rational::new(Integer::from(1_i64), Integer::from(10_i64)).unwrap();
    let third = Rational::new(Integer::from(1_i64), Integer::from(3_i64)).unwrap();
    same(
        &bits(
            "n",
            "Int[0, 20]",
            "Float64[nearest-even]",
            Value::Integer(Integer::from(3_i64)),
        ),
        Outcome::Completed(f64v(0x4008_0000_0000_0000)),
    );
    same(
        &bits(
            "r",
            "Rational[-10, 10; 1, 10]",
            "Float64[nearest-even]",
            Value::Rational(tenth),
        ),
        Outcome::Completed(f64v(0x3fb9_9999_9999_999a)),
    );
    same(
        &bits(
            "d",
            "Decimal[0, 100; 0, 3; nearest-even]",
            "Float64[nearest-even]",
            Value::Decimal(Decimal::new(Integer::from(1_i64), 1)),
        ),
        Outcome::Completed(f64v(0x3fb9_9999_9999_999a)),
    );
    for (target, expected) in [
        ("Float64[nearest-even]", 0x4340_0000_0000_0000_u64),
        ("Float64[toward-positive]", 0x4340_0000_0000_0001),
    ] {
        same(
            &bits("n", "Integer", target, integer("9007199254740993")),
            Outcome::Completed(f64v(expected)),
        );
    }
    assert_eq!(
        refusal_of(&bits(
            "n",
            "Integer",
            "Float64[exact]",
            integer("9007199254740993")
        )),
        ("ieee_not_exact", "rounding-required")
    );
    let one_third = Value::Rational(third);
    for (target, expected) in [
        ("Float32[nearest-even]", 0x3eaa_aaab_u32),
        ("Float32[toward-zero]", 0x3eaa_aaaa),
    ] {
        same(
            &bits("r", "Rational[-10, 10; 1, 10]", target, one_third.clone()),
            Outcome::Completed(f32v(expected)),
        );
    }
    for (source, target, expected) in [
        (
            0x3ff0_0000_0000_0001_u64,
            "Float32[nearest-even]",
            Outcome::Completed(f32v(0x3f80_0000)),
        ),
        (
            0x7ff8_0000_0000_0001,
            "Float32[nearest-even]",
            Outcome::Completed(f32v(0x7fc0_0001)),
        ),
    ] {
        same(&bits("a", F64, target, f64v(source)), expected);
    }
    assert_eq!(
        refusal_of(&bits(
            "a",
            F64,
            "Float32[nearest-even]",
            f64v(0x7ff8_0000_4000_0000)
        )),
        (
            "ieee_nan_payload_not_representable",
            "payload-exceeds-target"
        )
    );
    assert_eq!(
        refusal_of(&bits(
            "a",
            F64,
            "Float32[exact]",
            f64v(0x3ff0_0000_0000_0001)
        )),
        ("ieee_not_exact", "rounding-required")
    );
    for (source, expected) in [
        (0x3f80_0000_u32, 0x3ff0_0000_0000_0000_u64),
        (0x8000_0000, 0x8000_0000_0000_0000),
        (0x7f80_0000, 0x7ff0_0000_0000_0000),
        (0x7fc0_0001, 0x7ff8_0000_0000_0001),
    ] {
        same(
            &bits("a", F32, "Float64[exact]", f32v(source)),
            Outcome::Completed(f64v(expected)),
        );
    }
}

/// FR-397-AC-28 (F17): a standalone numeric equality charges as
/// `compare_ieee`: `ieee.operands` then `ieee.result-retain`, identical to
/// the owned comparison over the same operands and budget.
#[trace("TC-1091", "FR-397-AC-28")]
#[test]
fn f17_standalone_numeric_equality_charges_as_compare_ieee() {
    for (width, ty, qn, one) in [
        (
            32_u64,
            F32,
            Value::Float(IeeeValue::binary32(0x7fc0_0001)),
            Value::Float(IeeeValue::binary32(0x3f80_0000)),
        ),
        (
            64,
            F64,
            Value::Float(IeeeValue::binary64(0x7ff8_0000_0000_0001)),
            Value::Float(IeeeValue::binary64(0x3ff0_0000_0000_0000)),
        ),
    ] {
        let params = format!("qn: {ty}, one: {ty}");
        let args = vec![qn.clone(), one.clone()];
        let (Value::Float(qn), Value::Float(one)) = (qn, one) else {
            unreachable!()
        };
        let budgets = [
            limits(width, 2, 2, 1),
            limits(width, 2, 1, 1),
            limits(width, 0, 2, 1),
        ];
        for (body, left, right) in [
            ("qn = one", qn, one),
            ("quire::value::ieee::numericEqual(qn, one)", qn, one),
            ("one = one", one, one),
        ] {
            for budget in budgets {
                let mut reference = Meter::new(budget);
                let expected =
                    compare_ieee(IeeeComparison::NumericEqual, left, right, &mut reference)
                        .expect("same-width operands");
                let reference_charges: Vec<_> = reference
                    .admitted_charges()
                    .iter()
                    .map(|charge| charge.point)
                    .collect();
                let actual = run(&params, "Boolean", body, args.clone(), budget, (ty, "qn"));
                let expected = match expected {
                    Outcome::Completed(value) => Outcome::Completed(Value::Boolean(value)),
                    Outcome::Incomplete(incomplete) => Outcome::Incomplete(incomplete),
                    other => panic!("{other:?}"),
                };
                assert_eq!(
                    format!("{:?}", actual.outcome),
                    format!("{expected:?}"),
                    "{body} under {budget:?}"
                );
                assert_eq!(actual.charges, reference_charges, "{body} under {budget:?}");
            }
        }
        // The values and the exact charge log under the F17 tuple.
        let equal = run(
            &params,
            "Boolean",
            "qn = one",
            args.clone(),
            budgets[0],
            (ty, "qn"),
        );
        same(&equal.outcome, Outcome::Completed(Value::Boolean(false)));
        assert_eq!(
            equal.charges,
            [ChargePoint::IeeeOperands, ChargePoint::IeeeResultRetain]
        );
        let unequal = run(
            &params,
            "Boolean",
            "qn != one",
            args.clone(),
            // `!=` is the node followed by `not`, which charges its own work
            // and result after the node; leave room for it.
            limits(width, 2, 2 + 8, 1 + 8),
            (ty, "qn"),
        );
        same(&unequal.outcome, Outcome::Completed(Value::Boolean(true)));
        assert_eq!(
            unequal.charges,
            [
                ChargePoint::IeeeOperands,
                ChargePoint::IeeeResultRetain,
                ChargePoint::BooleanResultRetain
            ]
        );
        same(
            &run(
                &params,
                "Boolean",
                "qn = one",
                args.clone(),
                budgets[1],
                (ty, "qn"),
            )
            .outcome,
            incomplete(LimitKind::WorkUnits, 1, 1, 1, ChargePoint::IeeeResultRetain),
        );
        same(
            &run(&params, "Boolean", "qn = one", args, budgets[2], (ty, "qn")).outcome,
            incomplete(
                LimitKind::ValueOccurrences,
                0,
                0,
                2,
                ChargePoint::IeeeOperands,
            ),
        );
    }
}

/// FR-397-AC-28 and FR-397 "Evaluate once" (QSL-1724, IR-1282 AC3): the
/// numeric-equality part of a float ordering's composition takes its decision
/// from the owned `quire_exact` comparison yet adds no charge: the ordering
/// charges the `ieee.*` points of its own comparison only. Covers NaN, signed
/// zeros, equal and unequal operands, and both widths.
#[trace("TC-1091", "FR-397-AC-26", "FR-397-AC-28")]
#[test]
fn ordering_part_numeric_equality_uses_the_owned_comparison_and_adds_no_charge() {
    let big = limits(1 << 40, 1 << 40, 1 << 40, 1 << 40);
    for (width, ty) in [(32_u8, F32), (64, F64)] {
        let (zero, neg_zero, one, two, nan) = if width == 32 {
            (
                f32v(0),
                f32v(0x8000_0000),
                f32v(0x3f80_0000),
                f32v(0x4000_0000),
                f32v(0x7fc0_0001),
            )
        } else {
            (
                f64v(0),
                f64v(0x8000_0000_0000_0000),
                f64v(0x3ff0_0000_0000_0000),
                f64v(0x4000_0000_0000_0000),
                f64v(0x7ff8_0000_0000_0001),
            )
        };
        let params = format!("a: {ty}, b: {ty}");
        let rows = [
            ("a <= b", &zero, &neg_zero, true),
            ("a <= b", &neg_zero, &zero, true),
            ("a >= b", &zero, &neg_zero, true),
            ("a < b", &zero, &neg_zero, false),
            ("a <= b", &one, &one, true),
            ("a >= b", &one, &one, true),
            ("a < b", &one, &one, false),
            ("a <= b", &one, &two, true),
            ("a >= b", &one, &two, false),
            ("a <= b", &nan, &one, false),
            ("a >= b", &nan, &nan, false),
            ("a <= b", &nan, &nan, false),
        ];
        for (body, left, right, expected) in rows {
            let args = vec![left.clone(), right.clone()];
            let actual = run(&params, "Boolean", body, args, big, (ty, "a"));
            same(
                &actual.outcome,
                Outcome::Completed(Value::Boolean(expected)),
            );
            // The ordering's own comparison charges its `ieee.*` points; its
            // numeric-equality part adds none, so `<=` and `>=` charge the
            // `ieee.*` points the plain `<` over the same operands does.
            let ieee_points = |run: &Run| {
                run.charges
                    .iter()
                    .filter(|point| {
                        matches!(
                            point,
                            ChargePoint::IeeeOperands | ChargePoint::IeeeResultRetain
                        )
                    })
                    .count()
            };
            let plain = run(
                &params,
                "Boolean",
                "a < b",
                vec![left.clone(), right.clone()],
                big,
                (ty, "a"),
            );
            assert_eq!(
                ieee_points(&actual),
                ieee_points(&plain),
                "{body} on {width}-bit operands: {:?}",
                actual.charges
            );
            assert!(ieee_points(&actual) <= 2, "{:?}", actual.charges);
        }
    }
}

/// FR-397-AC-26: a float ordering never meets operands of two widths. The call
/// boundary refuses an argument whose width is not its parameter's, so the
/// owned comparison's ill-typed refusal cannot reach the ordering-part
/// numeric-equality decision.
#[trace("TC-1091", "FR-397-AC-26")]
#[test]
fn ordering_over_a_cross_width_argument_is_refused_at_the_call() {
    let big = limits(1 << 40, 1 << 40, 1 << 40, 1 << 40);
    let params = format!("a: {F64}, b: {F64}");
    let text = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n\
         function f using v({params}): Boolean pure {{ a <= b }}\n"
    );
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
    .expect("the assembler admits the unit");
    declarations.ieee_profile =
        Some(admit_ieee_profile(&[DefinitionRole::IeeeProfile.reference()], &[]).unwrap());
    let package = CheckedPackage::link(
        declarations
            .check(CheckingLimits::default(), CheckMode::Linked)
            .expect("the unit checks"),
    );
    let mut meter = Meter::new(big);
    let refusal = package
        .call(
            &QualifiedName::unqualified("f").expect("f is an identifier"),
            vec![f64v(0), f32v(0)],
            &ObjectEnvironment::default(),
            &mut meter,
        )
        .expect_err("a binary32 argument for a binary64 parameter is refused");
    assert_eq!(
        format!("{refusal:?}"),
        "Input(WrongValueKind { parameter: 1 })"
    );
    assert!(meter.admitted_charges().is_empty());
}
