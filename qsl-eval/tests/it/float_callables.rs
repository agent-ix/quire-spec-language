// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-397 "Intrinsics" and "Comparison": the `fma`, `numericEqual`,
//! `totalOrder` and `bitIdentical` calls and the float ordering operators,
//! evaluated from source through the checked package.

use ix_trace_rs::trace;
use qsl_eval::value::{CheckedPackageEvaluation, QualifiedName};
use qsl_package::CheckedPackage;
use qsl_semantics::check::PackageDeclarations;
use qsl_semantics::model::object_environment::ObjectEnvironment;
use qsl_semantics::value::{admit_ieee_profile, DefinitionRole};
use quire_eval::FamilyOutcome;
use quire_exact::{IeeeFlag, IeeeFlags, IeeeValue, LimitKind, Meter, Outcome, ScalarLimits, Value};
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

const IEEE: &str = "quire::value::ieee::";

/// Compiles `function f using v(): <result> pure { <body> }` and evaluates it.
fn run(result: &str, body: &str) -> (Outcome<Value>, Vec<ValueLoss>) {
    let text = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n\
         function f using v(): {result} pure {{ {body} }}\n"
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
    .expect("the assembler admits the floating type");
    declarations.ieee_profile =
        Some(admit_ieee_profile(&[DefinitionRole::IeeeProfile.reference()], &[]).unwrap());
    let package = CheckedPackage::link(
        declarations
            .check(CheckingLimits::default())
            .unwrap_or_else(|refusal| panic!("{body}: {refusal:?}")),
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
    (
        outcome,
        evaluation
            .losses
            .into_iter()
            .map(|each| each.loss)
            .collect(),
    )
}

fn boolean(body: &str) -> bool {
    match run("Boolean", body).0 {
        Outcome::Completed(Value::Boolean(value)) => value,
        other => panic!("{body}: a completed Boolean, not {other:?}"),
    }
}

/// Source for a float constant of `width` (32 or 64) with `bits`.
fn constant(width: u32, bits: u64) -> String {
    if width == 32 {
        format!("float32(bits: {bits:#010x})")
    } else {
        format!("float64(bits: {bits:#018x})")
    }
}

fn call(name: &str, width: u32, left: u64, right: u64) -> bool {
    boolean(&format!(
        "{IEEE}{name}({}, {})",
        constant(width, left),
        constant(width, right)
    ))
}

fn operator(width: u32, operator: &str, left: u64, right: u64) -> bool {
    boolean(&format!(
        "{} {operator} {}",
        constant(width, left),
        constant(width, right)
    ))
}

fn float(width: u32, body: &str) -> (u64, Vec<ValueLoss>) {
    let result = format!("Float{width}[nearest-even]");
    match run(&result, body) {
        (Outcome::Completed(Value::Float(value)), losses) => (value.bits(), losses),
        other => panic!("{body}: a completed float, not {other:?}"),
    }
}

fn flags(losses: &[ValueLoss]) -> Vec<IeeeFlag> {
    losses
        .iter()
        .filter_map(|loss| match loss {
            ValueLoss::IeeeFlags(flags) => Some(*flags),
            _ => None,
        })
        .flat_map(IeeeFlags::iter)
        .collect()
}

/// The FR-397-AC-13 vector table at one width: zero, negative zero, one,
/// the infinities, a quiet NaN and a signaling NaN.
struct Vectors {
    width: u32,
    p0: u64,
    n0: u64,
    one: u64,
    pinf: u64,
    ninf: u64,
    qnan: u64,
    snan: u64,
}

const VECTORS: [Vectors; 2] = [
    Vectors {
        width: 64,
        p0: 0x0000_0000_0000_0000,
        n0: 0x8000_0000_0000_0000,
        one: 0x3ff0_0000_0000_0000,
        pinf: 0x7ff0_0000_0000_0000,
        ninf: 0xfff0_0000_0000_0000,
        qnan: 0x7ff8_0000_0000_0000,
        snan: 0x7ff0_0000_0000_0001,
    },
    Vectors {
        width: 32,
        p0: 0x0000_0000,
        n0: 0x8000_0000,
        one: 0x3f80_0000,
        pinf: 0x7f80_0000,
        ninf: 0xff80_0000,
        qnan: 0x7fc0_0000,
        snan: 0x7f80_0001,
    },
];

/// Trace: FR-397-AC-13, FR-397-AC-24 (F34); QSpec FR-148-AC-12's vectors,
/// with no flag, at both widths.
#[trace(
    "TC-1091",
    "FR-397-AC-13",
    "FR-397-AC-24",
    "QSpec-TC-193",
    "QSpec-FR-148-AC-12"
)]
#[test]
fn ordering_and_equality_vectors_hold_at_both_widths() {
    for v in &VECTORS {
        let w = v.width;
        let expected = [
            (v.n0, "=", v.p0, true),
            (v.n0, "<", v.p0, false),
            (v.n0, "<=", v.p0, true),
            (v.p0, ">=", v.n0, true),
            (v.p0, ">", v.n0, false),
            (v.one, "<", v.pinf, true),
            (v.ninf, "<", v.one, true),
            (v.qnan, "=", v.qnan, false),
            (v.qnan, "!=", v.qnan, true),
            (v.qnan, "!=", v.one, true),
            (v.qnan, "<", v.one, false),
            (v.qnan, "<=", v.one, false),
            (v.one, ">", v.qnan, false),
            (v.one, ">=", v.qnan, false),
            (v.snan, "<", v.one, false),
            (v.snan, "=", v.snan, false),
            (v.pinf, "=", v.pinf, true),
            (v.pinf, "<=", v.pinf, true),
        ];
        for (left, op, right, value) in expected {
            let body = format!("{} {op} {}", constant(w, left), constant(w, right));
            let (outcome, losses) = run("Boolean", &body);
            assert!(
                matches!(outcome, Outcome::Completed(Value::Boolean(holds)) if holds == value),
                "{w}: {left:#x} {op} {right:#x}: {outcome:?}"
            );
            // QSpec FR-148-AC-12: a quiet comparison raises no flag, a
            // signaling NaN operand included.
            assert!(flags(&losses).is_empty(), "{w}: {body}: {losses:?}");
        }
    }
}

/// Trace: FR-397-AC-16
#[trace("TC-1091", "FR-397-AC-16")]
#[test]
fn total_order_and_bit_identity_calls_separate_zero_signs_and_nan_payloads() {
    let f64 = |name, left, right| call(name, 64, left, right);
    assert!(f64("totalOrder", 0x8000_0000_0000_0000, 0));
    assert!(!f64("totalOrder", 0, 0x8000_0000_0000_0000));
    assert!(f64(
        "totalOrder",
        0x7ff0_0000_0000_0000,
        0x7ff8_0000_0000_0000
    ));
    assert!(!f64(
        "totalOrder",
        0x7ff8_0000_0000_0000,
        0x7ff0_0000_0000_0000
    ));
    assert!(f64(
        "totalOrder",
        0x7ff0_0000_0000_0001,
        0x7ff8_0000_0000_0000
    ));
    assert!(f64(
        "totalOrder",
        0xfff8_0000_0000_0000,
        0xfff0_0000_0000_0000
    ));
    assert!(!f64("bitIdentical", 0x8000_0000_0000_0000, 0));
    assert!(f64("numericEqual", 0x8000_0000_0000_0000, 0));
    assert!(f64(
        "bitIdentical",
        0x7ff8_0000_0000_0000,
        0x7ff8_0000_0000_0000
    ));
    assert!(!f64(
        "numericEqual",
        0x7ff8_0000_0000_0000,
        0x7ff8_0000_0000_0000
    ));
}

/// Trace: FR-397-AC-24 (F01, F02, F02b)
#[trace("TC-1091", "FR-397-AC-24")]
#[test]
fn f01_f02_f02b_zeros_and_nans_separate_equality_order_and_identity() {
    let f32 = |name, left, right| call(name, 32, left, right);
    // F01
    assert!(operator(32, "=", 0, 0x8000_0000));
    assert!(!f32("bitIdentical", 0x8000_0000, 0));
    assert!(f32("totalOrder", 0x8000_0000, 0));
    assert!(!f32("totalOrder", 0, 0x8000_0000));
    // F02
    let (a, b) = (0x7fc0_0001, 0x7fc0_0002);
    assert!(!operator(32, "=", a, a));
    assert!(f32("bitIdentical", a, a));
    assert!(f32("totalOrder", a, b));
    assert!(!f32("totalOrder", b, a));
    // F02b
    for (low, high) in [(0xffc0_0002, 0xffc0_0001), (0x7f80_0001, 0x7fc0_0001)] {
        assert!(f32("totalOrder", low, high), "{low:#x} {high:#x}");
        assert!(!f32("totalOrder", high, low), "{high:#x} {low:#x}");
    }
}

/// Trace: FR-397-AC-24 (F05)
#[trace("TC-1091", "FR-397-AC-24")]
#[test]
fn f05_infinities_and_finite_extrema_follow_value_order() {
    let f32 = |name, left, right| call(name, 32, left, right);
    for (low, high) in [
        (0xff80_0000, 0xff7f_ffff),
        (0xff7f_ffff, 0x7f7f_ffff),
        (0x7f7f_ffff, 0x7f80_0000),
    ] {
        assert!(operator(32, "<", low, high));
        assert!(operator(32, "<=", low, high));
        assert!(f32("totalOrder", low, high));
        assert!(!f32("numericEqual", low, high));
        assert!(!operator(32, "<", high, low));
        assert!(!operator(32, "<=", high, low));
    }
    assert!(f32("bitIdentical", 0x7f80_0000, 0x7f80_0000));
    assert!(!f32("bitIdentical", 0x7f80_0000, 0xff80_0000));
}

/// Trace: FR-397-AC-15
#[trace("TC-1091", "FR-397-AC-15")]
#[test]
fn fma_rounds_once_where_multiply_then_add_rounds_twice() {
    let (bits, _) = float(
        64,
        &format!(
            "{IEEE}fma({}, {}, {})",
            constant(64, 0x3ff0_0000_0000_0001),
            constant(64, 0x3fef_ffff_ffff_ffff),
            constant(64, 0xbff0_0000_0000_0000)
        ),
    );
    assert_eq!(bits, 0x3c9f_ffff_ffff_fffe);
    let (separate, _) = float(
        64,
        &format!(
            "{} * {} + {}",
            constant(64, 0x3ff0_0000_0000_0001),
            constant(64, 0x3fef_ffff_ffff_ffff),
            constant(64, 0xbff0_0000_0000_0000)
        ),
    );
    assert_eq!(separate, 0);
}

/// Trace: FR-397-AC-24 (F11)
#[trace("TC-1091", "FR-397-AC-24")]
#[test]
fn f11_fused_multiply_add_rounds_once_unlike_multiply_then_add() {
    let (bits, _) = float(
        32,
        &format!(
            "{IEEE}fma({}, {}, {})",
            constant(32, 0x3f80_0001),
            constant(32, 0x3f7f_fffe),
            constant(32, 0xbf80_0000)
        ),
    );
    assert_eq!(bits, 0xa880_0000);
    let (separate, _) = float(
        32,
        &format!(
            "{} * {} + {}",
            constant(32, 0x3f80_0001),
            constant(32, 0x3f7f_fffe),
            constant(32, 0xbf80_0000)
        ),
    );
    assert_eq!(separate, 0);
}

/// Trace: FR-397-AC-24 (F04b)
#[trace("TC-1091", "FR-397-AC-24")]
#[test]
fn f04b_leftmost_nan_payload_is_kept_and_a_signaling_nan_raises_invalid() {
    let sum = format!(
        "{} + {}",
        constant(32, 0x7fc0_0012),
        constant(32, 0xff80_0021)
    );
    let fused = format!(
        "{IEEE}fma({}, {}, {})",
        constant(32, 0x3f80_0000),
        constant(32, 0x7fc0_0012),
        constant(32, 0xff80_0021)
    );
    for body in [sum, fused] {
        let (bits, losses) = float(32, &body);
        assert_eq!(bits, 0x7fc0_0012, "{body}");
        assert_eq!(flags(&losses), [IeeeFlag::Invalid], "{body}");
    }
}

/// Trace: FR-397-AC-12, FR-397-AC-13
///
/// A signaling NaN sum records one `invalid` flag per evaluation, so the
/// number of flag records counts how often an operand is evaluated.
#[trace("TC-1091", "FR-397-AC-12", "FR-397-AC-13")]
#[test]
fn each_ordering_operand_is_evaluated_exactly_once() {
    let signaling_sum = format!(
        "{} + {}",
        constant(32, 0x7f80_0001),
        constant(32, 0x3f80_0000)
    );
    let one = constant(32, 0x3f80_0000);
    for operator in ["<", "<=", ">", ">="] {
        for body in [
            format!("({signaling_sum}) {operator} {one}"),
            format!("{one} {operator} ({signaling_sum})"),
        ] {
            let (outcome, losses) = run("Boolean", &body);
            assert!(
                matches!(outcome, Outcome::Completed(Value::Boolean(false))),
                "{body}: {outcome:?}"
            );
            assert_eq!(flags(&losses), [IeeeFlag::Invalid], "{body}");
        }
    }
}

/// Compiles `function f using v(p: Float64[nearest-even], q: ..): <result>
/// pure { <body> }` and evaluates it at `p` and `q` under `limits`, returning
/// the outcome and the work and value-occurrence units it spent.
fn spend(result: &str, body: &str, p: u64, q: u64, limits: ScalarLimits) -> (Outcome<Value>, u64) {
    let text = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n\
         function f using v(p: Float64[nearest-even], q: Float64[nearest-even]): {result} pure {{ {body} }}\n"
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
    .expect("the assembler admits the floating type");
    declarations.ieee_profile =
        Some(admit_ieee_profile(&[DefinitionRole::IeeeProfile.reference()], &[]).unwrap());
    let package = CheckedPackage::link(
        declarations
            .check(CheckingLimits::default())
            .unwrap_or_else(|refusal| panic!("{body}: {refusal:?}")),
    );
    let mut meter = Meter::new(limits);
    let evaluation = package
        .call(
            &QualifiedName::unqualified("f").expect("f is an identifier"),
            vec![
                Value::Float(IeeeValue::binary64(p)),
                Value::Float(IeeeValue::binary64(q)),
            ],
            &ObjectEnvironment::default(),
            &mut meter,
        )
        .expect("the call runs");
    let FamilyOutcome::Evaluated(outcome) = evaluation.outcome else {
        panic!("a kernel outcome, not {:?}", evaluation.outcome);
    };
    let spent = meter.consumed(LimitKind::WorkUnits);
    (outcome, spent)
}

/// Trace: FR-397-AC-26
#[trace("TC-1091", "FR-397-AC-26")]
#[test]
fn an_ordering_spends_each_operand_once_and_stops_at_its_limit() {
    const FOUR: u64 = 0x4010_0000_0000_0000;
    const NINE: u64 = 0x4022_0000_0000_0000;
    const QNAN: u64 = 0x7ff8_0000_0000_0000;
    let root = |operand: &str| format!("{IEEE}sqrt({operand})");
    let at = |result, body: &str, p, q| spend(result, body, p, q, UNLIMITED);
    let float = "Float64[nearest-even]";
    let (_, p_alone) = at(float, "p", FOUR, NINE);
    let (_, q_alone) = at(float, "q", FOUR, NINE);
    let (_, sqrt_p) = at(float, &root("p"), FOUR, NINE);
    let (_, sqrt_q) = at(float, &root("q"), FOUR, NINE);
    for (operator, expected) in [("<", true), ("<=", true), (">", false), (">=", false)] {
        let parameters = format!("p {operator} q");
        let roots = format!("{} {operator} {}", root("p"), root("q"));
        let (plain, plain_spend) = at("Boolean", &parameters, FOUR, NINE);
        let (sqrt, sqrt_spend) = at("Boolean", &roots, FOUR, NINE);
        assert!(matches!(plain, Outcome::Completed(Value::Boolean(value)) if value == expected));
        assert!(matches!(sqrt, Outcome::Completed(Value::Boolean(value)) if value == expected));
        assert_eq!(
            sqrt_spend,
            plain_spend + (sqrt_p - p_alone) + (sqrt_q - q_alone),
            "{roots}: each operand is evaluated once"
        );
        let limited = |units: u64| {
            let mut limits = UNLIMITED;
            limits.work_units = units;
            spend("Boolean", &roots, FOUR, NINE, limits).0
        };
        assert!(
            matches!(limited(sqrt_spend), Outcome::Completed(Value::Boolean(value)) if value == expected),
            "{roots}: completes at its spend"
        );
        assert!(
            matches!(limited(sqrt_spend - 1), Outcome::Incomplete(..)),
            "{roots}: incomplete one unit below its spend"
        );
        // A NaN operand: false, each operand once, no more spend than the
        // ordered operands.
        let (nan, nan_spend) = at("Boolean", &roots, QNAN, NINE);
        assert!(
            matches!(nan, Outcome::Completed(Value::Boolean(false))),
            "{roots}: {nan:?}"
        );
        assert!(
            nan_spend <= sqrt_spend,
            "{roots}: {nan_spend} > {sqrt_spend}"
        );
    }
}

/// FR-397-AC-26 and FR-148: a standalone `=`, `!=` and `numericEqual` charge
/// the operands and the result as one owned comparison core (the same spend
/// as `totalOrder`), while the numeric-equality parts of an ordering's
/// composition add no charge, so an ordering costs far less than the three
/// comparisons its parts would charge on top of its one `totalOrder`.
#[trace("TC-1091", "FR-397-AC-26")]
#[test]
fn a_standalone_numeric_equality_charges_and_an_orderings_parts_do_not() {
    const FOUR: u64 = 0x4010_0000_0000_0000;
    const NINE: u64 = 0x4022_0000_0000_0000;
    let at = |body: &str| spend("Boolean", body, FOUR, NINE, UNLIMITED).1;
    let nothing = at("true");
    let compare = at(&format!("{IEEE}totalOrder(p, q)")) - nothing;
    assert!(compare > 0, "a comparison charges its operands and result");
    assert_eq!(at("p = q") - nothing, compare, "standalone `=`");
    assert_eq!(
        at(&format!("{IEEE}numericEqual(p, q)")) - nothing,
        compare,
        "standalone numericEqual"
    );
    assert!(at("p != q") - nothing >= compare, "standalone `!=`");
    for operator in ["<", "<=", ">", ">="] {
        let ordering = at(&format!("p {operator} q")) - nothing;
        // One totalOrder plus the connectives costs 3 comparisons; charging
        // the three numeric-equality parts would make it 6.
        assert!(
            ordering < 5 * compare,
            "p {operator} q spends {ordering}; a comparison is {compare}"
        );
    }
}
