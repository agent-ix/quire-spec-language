// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-137 (FR-049-AC-7, NFR-009-AC-3): caller-owned accounting across the
//! checked-term entry points `CheckedPackageEvaluation::{call, evaluate}`.
//!
//! Every fixture is source compiled through S1 and S2 and checked by S3, then
//! driven through a `quire-exact::Meter` built with the `test-support`
//! feature. The meter's admitted-charge log is the oracle that does not come
//! from the evaluator under test: each fixture's quantity is counted from how
//! the fixture is built. Boundary limits follow the accounting schedule
//! independently of a successful run.
//!
//! The ten counters are those of FR-255's accounting table. FR-255 gives
//! `work_units` the default of [`DEFAULT_WORK_UNITS`] and every other counter
//! `u64::MAX`.

use ix_trace_rs::trace;
use qsl_eval::value::{CheckedPackageEvaluation, QualifiedName};
use qsl_forms::Expression;
use qsl_package::CheckedPackage;
use qsl_semantics::check::PackageDeclarations;
use qsl_semantics::model::object_environment::ObjectEnvironment;
use quire_eval::{CallFailure, Evaluation, FamilyOutcome};
use quire_exact::{
    admit_text, form_collection, AdmittedCharge, CardinalityBound, Charge, ChargePoint,
    CollectionKind, CollectionType, Decimal, Incomplete, InjectedDenial, Integer, IntegerInterval,
    LimitKind, Meter, Outcome, ScalarLimits, Text, TextPayload, TextProfile, TextType, Value,
    ValueType,
};
use quire_semantic_value::checking::{CheckMode, CheckingLimits};
use std::num::NonZeroU64;

/// FR-255: the published default of `accounting.work_units`.
const DEFAULT_WORK_UNITS: u64 = 1_000_000;

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

const COUNTERS: [LimitKind; 10] = [
    LimitKind::IntegerBits,
    LimitKind::DecimalDigits,
    LimitKind::ScaleExpansion,
    LimitKind::TextInputBytes,
    LimitKind::TextScalars,
    LimitKind::NormalizedScalars,
    LimitKind::UnitEdges,
    LimitKind::ValueOccurrences,
    LimitKind::WorkUnits,
    LimitKind::ResultUnits,
];

/// The edges of the unit conversion chain `us -> ms -> s` declared in [`UNIT`].
const CONVERSION_EDGES: u64 = 2;

const SEQUENCE_ELEMENTS: [i64; 3] = [1, 2, 3];

const UNIT: &str = "dimension Time;\n\
 unit s : Time = rational(1, 1);\n\
 unit ms : Time = rational(1, 1000) * s;\n\
 unit us : Time = rational(1, 1000) * ms;\n\
 type Total = Integer;\n\
 record Chain { head: Int[0, 9]; tail: Chain?; }\n\
 function integer_sum using v(a: Integer, b: Integer): Integer pure { a + b }\n\
 function decimal_sum using v(a: Decimal[0, 2000000; 0, 3; exact], b: Decimal[0, 2000000; 0, 3; exact]): Decimal[0, 2000000; 0, 3; exact] pure { a + b }\n\
 function text_order using v(a: Text[0, 16; nfd], b: Text[0, 16; nfd]): Boolean pure { a < b }\n\
 function unit_chain using v(): Boolean pure { convert<Quantity<s>>(5 us) = 5 s }\n\
 function two_elements using v(): Sequence<Integer>[0, 16] pure { sequence[1, 2] }\n\
 function three_elements using v(): Sequence<Integer>[0, 16] pure { sequence[@sequence-elements@] }\n\
 function nothing using v(): Boolean pure { true }\n\
 function link0 using v(x: Integer): Boolean pure { true }\n\
 function link1 using v(x: Integer): Boolean pure { link0(x) }\n\
 function link2 using v(x: Integer): Boolean pure { link1(x) }\n\
 function walk using v(xs: Sequence<Int[0, 9]>[0, 2000000]): Integer pure { sum<Total>(x in xs: x) }\n\
 function head_above using v(l: Chain): Boolean pure { l.head > 5 }\n";

/// Compile `declarations` as one complete-V1 unit into a checked package.
fn package_of(declarations: &str) -> CheckedPackage {
    let elements = SEQUENCE_ELEMENTS
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(", ");
    let declarations = declarations.replace("@sequence-elements@", &elements);
    let text = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n{declarations}"
    );
    let parsed = qsl_cst::parse(
        qsl_foundation::SourceIdentity::new("a", "u", "git", "1"),
        "unit.native",
        text.as_bytes(),
        qsl_cst::Limits::default()
            .with_tokens(1 << 24)
            .with_nodes(1 << 24)
            .with_source_bytes(1 << 26)
            .with_work_units(1 << 40),
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
    CheckedPackage::link(
        declarations
            .check(
                CheckingLimits::default()
                    .with_nodes(1 << 30)
                    .with_input_bytes(1 << 30)
                    .with_work_budget(1 << 40),
            )
            .expect("the unit checks"),
    )
}

fn name(function: &str) -> QualifiedName {
    QualifiedName::unqualified(function).expect("an identifier")
}

fn int(value: i64) -> Value {
    Value::Integer(Integer::from(value))
}

/// A `Text[0, 16; nfd]` value holding `payload`.
fn text(payload: &str) -> Value {
    let text: Text = admit_text(
        &TextPayload::from_utf8(payload.as_bytes()).expect("valid UTF-8"),
        &TextType::new(0, 16, TextProfile::Nfd).expect("a text type"),
        &mut Meter::new(UNLIMITED),
    )
    .completed()
    .expect("the text admits");
    Value::Text(text)
}

/// A `Sequence<Int[0, 9]>[0, 2000000]` of `count` ones.
fn ones(count: usize) -> Value {
    let element = ValueType::Int(
        IntegerInterval::new(Integer::from(0_i64), Integer::from(9_i64)).expect("an interval"),
    );
    let collection_type = CollectionType::new(
        CollectionKind::Sequence,
        element,
        Some(CardinalityBound::new(0, 2_000_000).expect("a bound")),
    );
    form_collection(
        &collection_type,
        vec![int(1); count],
        &mut Meter::new(UNLIMITED),
    )
    .expect("the sequence is well typed")
    .completed()
    .expect("the sequence forms")
}

/// The limits with `kind` set to `bound` and the other nine sufficient.
fn only(kind: LimitKind, bound: u64) -> ScalarLimits {
    let mut limits = UNLIMITED;
    match kind {
        LimitKind::IntegerBits => limits.integer_bits = bound,
        LimitKind::DecimalDigits => limits.decimal_digits = bound,
        LimitKind::ScaleExpansion => limits.scale_expansion = bound,
        LimitKind::TextInputBytes => limits.text_input_bytes = bound,
        LimitKind::TextScalars => limits.text_scalars = bound,
        LimitKind::NormalizedScalars => limits.normalized_scalars = bound,
        LimitKind::UnitEdges => limits.unit_edges = bound,
        LimitKind::ValueOccurrences => limits.value_occurrences = bound,
        LimitKind::WorkUnits => limits.work_units = bound,
        LimitKind::ResultUnits => limits.result_units = bound,
    }
    limits
}

/// The size an admitted charge requested against the high-water counter
/// `kind`; `None` for a charge that made no such request and for the two
/// cumulative counters, which the log does not record.
fn requested(charge: &AdmittedCharge, kind: LimitKind) -> Option<u64> {
    match kind {
        LimitKind::IntegerBits => charge.integer_bits,
        LimitKind::DecimalDigits => charge.decimal_digits,
        LimitKind::ScaleExpansion => charge.scale_expansion,
        LimitKind::TextInputBytes => charge.text_input_bytes,
        LimitKind::TextScalars => charge.text_scalars,
        LimitKind::NormalizedScalars => charge.normalized_scalars,
        LimitKind::UnitEdges => charge.unit_edges,
        LimitKind::ValueOccurrences => charge.value_occurrences,
        LimitKind::WorkUnits | LimitKind::ResultUnits => None,
    }
}

/// One finished call: its debug-rendered result, the meter it ran on and the
/// kernel outcome when the call returned one.
struct Run {
    outcome: Outcome<Value>,
    meter: Meter,
}

impl Run {
    fn log(&self) -> Vec<AdmittedCharge> {
        assert!(
            !self.meter.charge_log_truncated(),
            "the fixture's charge log fits the diagnostic log"
        );
        self.meter.admitted_charges().to_vec()
    }

    fn usage(&self) -> Vec<u64> {
        COUNTERS
            .iter()
            .map(|kind| self.meter.consumed(*kind))
            .collect()
    }

    fn rendered(&self) -> String {
        format!("{:?}", self.outcome)
    }

    /// The denied charge's record: any outcome but `Incomplete` is a
    /// completed or partial value the stop must not expose.
    fn incomplete(&self) -> &Incomplete {
        match &self.outcome {
            Outcome::Incomplete(record) => record,
            other => panic!("expected the meter's incomplete record, got {other:?}"),
        }
    }
}

fn kernel(evaluation: Evaluation) -> Outcome<Value> {
    match evaluation.outcome {
        FamilyOutcome::Evaluated(outcome) => outcome,
        other => panic!("expected FamilyOutcome::Evaluated(_), got {other:?}"),
    }
}

/// `function` called with `arguments` on a fresh meter of `limits`.
fn call(
    package: &CheckedPackage,
    function: &str,
    arguments: Vec<Value>,
    limits: ScalarLimits,
) -> Run {
    call_on(package, function, arguments, Meter::new(limits))
}

fn call_on(
    package: &CheckedPackage,
    function: &str,
    arguments: Vec<Value>,
    mut meter: Meter,
) -> Run {
    let evaluation = package
        .call(
            &name(function),
            arguments,
            &ObjectEnvironment::default(),
            &mut meter,
        )
        .expect("the call returns a kernel outcome");
    Run {
        outcome: kernel(evaluation),
        meter,
    }
}

/// One fixture of the TC-137 per-counter table.
struct Case {
    counter: LimitKind,
    function: &'static str,
    arguments: fn() -> Vec<Value>,
    /// The charge point the fixture must reach.
    point: ChargePoint,
    /// Other charge points in the independent schedule that this fixture
    /// must also reach, even when they do not determine the high-water value.
    required_points: &'static [ChargePoint],
    /// Independently counted high-water request at every relevant charge
    /// point. `None` marks points that contribute to another counter but do
    /// not request this size. These amounts do not come from `Meter::consumed`.
    expected_sizes: Vec<(ChargePoint, Option<u64>)>,
    /// The fixture's quantity, counted from its construction; `None` for a
    /// counter whose individual log entries do not carry that size.
    counted: Option<u64>,
    /// Independently counted peak or cumulative total, including conservative
    /// intermediate sizes and the retained collection's container occurrence.
    expected_usage: u64,
}

/// The bit length of `value`, counted without the evaluator.
fn bit_length(value: u64) -> u64 {
    u64::from(u64::BITS - value.leading_zeros())
}

fn decimal(coefficient: i64, scale: u32) -> Value {
    Value::Decimal(Decimal::new(Integer::from(coefficient), scale))
}

/// A text of two scalars (4 bytes) whose NFD form is four scalars.
const ACCENTED: &str = "\u{e9}\u{e9}";
const ACCENTED_NFD: &str = "e\u{301}e\u{301}";
/// The larger integer operand of `integer_sum`.
const LARGE: i64 = 1 << 40;
/// The decimal operands of `decimal_sum`: `123` at scale 0 and `5` at scale 3.
const WIDE_COEFFICIENT: i64 = 123;
const WIDE_SCALE: u32 = 0;
const NARROW_SCALE: u32 = 3;
const SMALL_COEFFICIENT: i64 = 5;

fn decimal_digits(value: i64) -> u64 {
    value.unsigned_abs().to_string().len() as u64
}

fn decimal_shifted_digits(coefficient: i64, shift: u32) -> u64 {
    decimal_digits(coefficient) + u64::from(shift)
}

fn aligned_decimal_scale(left: u32, right: u32) -> u32 {
    left.max(right)
}

fn cases() -> Vec<Case> {
    let integer_arguments: fn() -> Vec<Value> = || vec![int(LARGE), int(3)];
    let decimal_arguments: fn() -> Vec<Value> = || {
        vec![
            decimal(WIDE_COEFFICIENT, WIDE_SCALE),
            decimal(5, NARROW_SCALE),
        ]
    };
    let text_arguments: fn() -> Vec<Value> = || vec![text(ACCENTED), text("")];
    let none: fn() -> Vec<Value> = Vec::new;
    let one: fn() -> Vec<Value> = || vec![int(1)];
    let aligned_left_digits = decimal_shifted_digits(WIDE_COEFFICIENT, NARROW_SCALE - WIDE_SCALE);
    let aligned_right_digits = decimal_digits(SMALL_COEFFICIENT);
    let decimal_arithmetic_digits = aligned_left_digits.max(aligned_right_digits) + 1;
    let retained_decimal_coefficient =
        WIDE_COEFFICIENT * 10_i64.pow(NARROW_SCALE - WIDE_SCALE) + SMALL_COEFFICIENT;
    let retained_decimal_digits = decimal_digits(retained_decimal_coefficient);
    let aligned_scale = aligned_decimal_scale(WIDE_SCALE, NARROW_SCALE);
    let retained_scale_expansion = NARROW_SCALE - aligned_scale;
    vec![
        Case {
            counter: LimitKind::IntegerBits,
            function: "integer_sum",
            arguments: integer_arguments,
            point: ChargePoint::IntegerArithmeticOperands,
            required_points: &[ChargePoint::IntegerArithmeticArithmetic],
            expected_sizes: vec![
                (
                    ChargePoint::IntegerArithmeticOperands,
                    Some(bit_length(LARGE.unsigned_abs()).max(bit_length(3))),
                ),
                (
                    ChargePoint::IntegerArithmeticArithmetic,
                    Some(bit_length(LARGE.unsigned_abs()).max(bit_length(3)) + 1),
                ),
            ],
            counted: Some(bit_length(LARGE.unsigned_abs())),
            expected_usage: bit_length(LARGE.unsigned_abs()).max(bit_length(3)) + 1,
        },
        Case {
            counter: LimitKind::DecimalDigits,
            function: "decimal_sum",
            arguments: decimal_arguments,
            point: ChargePoint::DecimalOperands,
            required_points: &[
                ChargePoint::DecimalScaleExpansion,
                ChargePoint::DecimalArithmetic,
                ChargePoint::DecimalResultRetain,
            ],
            expected_sizes: vec![
                (
                    ChargePoint::DecimalOperands,
                    Some(decimal_digits(WIDE_COEFFICIENT).max(decimal_digits(SMALL_COEFFICIENT))),
                ),
                (
                    ChargePoint::DecimalScaleExpansion,
                    Some(aligned_left_digits),
                ),
                (
                    ChargePoint::DecimalArithmetic,
                    Some(decimal_arithmetic_digits),
                ),
                (
                    ChargePoint::DecimalResultRetain,
                    Some(retained_decimal_digits),
                ),
            ],
            counted: Some(decimal_digits(WIDE_COEFFICIENT)),
            expected_usage: decimal_digits(WIDE_COEFFICIENT)
                .max(decimal_digits(SMALL_COEFFICIENT))
                .max(aligned_left_digits)
                .max(aligned_right_digits)
                .max(decimal_arithmetic_digits)
                .max(retained_decimal_digits),
        },
        Case {
            counter: LimitKind::ScaleExpansion,
            function: "decimal_sum",
            arguments: decimal_arguments,
            point: ChargePoint::DecimalScaleExpansion,
            required_points: &[ChargePoint::DecimalResultRetain],
            expected_sizes: vec![
                (
                    ChargePoint::DecimalScaleExpansion,
                    Some(u64::from(NARROW_SCALE - WIDE_SCALE)),
                ),
                (
                    ChargePoint::DecimalResultRetain,
                    Some(u64::from(retained_scale_expansion)),
                ),
            ],
            counted: Some(u64::from(NARROW_SCALE - WIDE_SCALE)),
            expected_usage: u64::from(NARROW_SCALE - WIDE_SCALE),
        },
        Case {
            counter: LimitKind::TextInputBytes,
            function: "text_order",
            arguments: text_arguments,
            point: ChargePoint::TextInputBytes,
            required_points: &[],
            expected_sizes: vec![(ChargePoint::TextInputBytes, Some(ACCENTED.len() as u64))],
            counted: Some(ACCENTED.len() as u64),
            expected_usage: ACCENTED.len() as u64,
        },
        Case {
            counter: LimitKind::TextScalars,
            function: "text_order",
            arguments: text_arguments,
            point: ChargePoint::TextDecodeScalars,
            required_points: &[],
            expected_sizes: vec![(
                ChargePoint::TextDecodeScalars,
                Some(ACCENTED.chars().count() as u64),
            )],
            counted: Some(ACCENTED.chars().count() as u64),
            expected_usage: ACCENTED.chars().count() as u64,
        },
        Case {
            counter: LimitKind::NormalizedScalars,
            function: "text_order",
            arguments: text_arguments,
            point: ChargePoint::TextNormalizeOutput,
            required_points: &[],
            expected_sizes: vec![(
                ChargePoint::TextNormalizeOutput,
                Some(ACCENTED_NFD.chars().count() as u64),
            )],
            counted: Some(ACCENTED_NFD.chars().count() as u64),
            expected_usage: ACCENTED_NFD.chars().count() as u64,
        },
        Case {
            counter: LimitKind::UnitEdges,
            function: "unit_chain",
            arguments: none,
            point: ChargePoint::UnitEdge,
            required_points: &[],
            expected_sizes: vec![(ChargePoint::UnitEdge, Some(CONVERSION_EDGES))],
            counted: Some(CONVERSION_EDGES),
            expected_usage: CONVERSION_EDGES,
        },
        Case {
            counter: LimitKind::ValueOccurrences,
            function: "three_elements",
            arguments: none,
            point: ChargePoint::CollectionElement,
            required_points: &[
                ChargePoint::CollectionBound,
                ChargePoint::CollectionResultRetain,
            ],
            expected_sizes: vec![
                (ChargePoint::CollectionElement, None),
                (
                    ChargePoint::CollectionBound,
                    Some(SEQUENCE_ELEMENTS.len() as u64),
                ),
                (
                    ChargePoint::CollectionResultRetain,
                    Some(1 + SEQUENCE_ELEMENTS.len() as u64),
                ),
            ],
            counted: None,
            expected_usage: 1 + SEQUENCE_ELEMENTS.len() as u64,
        },
        Case {
            counter: LimitKind::WorkUnits,
            function: "link2",
            arguments: one,
            point: ChargePoint::FunctionCall,
            required_points: &[],
            expected_sizes: vec![(ChargePoint::FunctionCall, None)],
            counted: None,
            expected_usage: 3,
        },
        Case {
            counter: LimitKind::ResultUnits,
            function: "three_elements",
            arguments: none,
            point: ChargePoint::CollectionResultRetain,
            required_points: &[],
            expected_sizes: vec![(ChargePoint::CollectionResultRetain, None)],
            counted: None,
            expected_usage: 1 + SEQUENCE_ELEMENTS.len() as u64,
        },
    ]
}

/// TC-137 per-counter boundaries for one `case`: independent count, zero,
/// one-short and exact runs, the charge-before-work prefix and the retry.
fn exercise(package: &CheckedPackage, case: &Case) {
    let kind = case.counter;
    let arguments = (case.arguments)();
    let rendered_arguments = format!("{arguments:?}");
    let full = call(package, case.function, arguments.clone(), UNLIMITED);
    assert!(
        matches!(full.outcome, Outcome::Completed(_)),
        "{kind:?}: a sufficient meter completes: {:?}",
        full.outcome
    );
    let log = full.log();
    let used = case.expected_usage;
    assert_eq!(
        full.meter.consumed(kind),
        used,
        "{kind:?}: usage equals the independently counted accounting quantity"
    );
    assert!(used > 0, "{kind:?}: the fixture requires work");
    assert!(
        log.iter().any(|charge| charge.point == case.point),
        "{kind:?}: the admitted-charge log holds {:?}",
        case.point
    );
    for point in case.required_points {
        assert!(
            log.iter().any(|charge| charge.point == *point),
            "{kind:?}: the independent schedule reaches {point:?}"
        );
    }
    for &(point, expected_size) in &case.expected_sizes {
        let actual_size = log
            .iter()
            .filter(|charge| charge.point == point)
            .filter_map(|charge| requested(charge, kind))
            .max();
        assert_eq!(
            actual_size, expected_size,
            "{kind:?}: independently counted size at {point:?}"
        );
    }

    if kind == LimitKind::WorkUnits {
        assert_eq!(
            log.iter().map(|charge| charge.point).collect::<Vec<_>>(),
            vec![ChargePoint::FunctionCall; 3],
            "three nested constant predicates charge only their calls"
        );
    }
    if matches!(kind, LimitKind::ValueOccurrences | LimitKind::ResultUnits) {
        assert_eq!(
            log.iter()
                .filter(|charge| charge.point == ChargePoint::CollectionElement)
                .count(),
            SEQUENCE_ELEMENTS.len(),
            "one element charge per source element"
        );
        assert_eq!(
            log.iter()
                .filter(|charge| charge.point == ChargePoint::CollectionResultRetain)
                .count(),
            1,
            "one retained sequence, with its container and scalar occurrences"
        );
    }

    // The denied charge is the next one the sufficient run admits, and the
    // stopped meter holds exactly the charges admitted before it.
    let assert_stopped_before = |stopped: &Run, limit: u64, why: &str| {
        let record = stopped.incomplete();
        let admitted = stopped.log();
        assert_eq!(admitted, log[..admitted.len()], "{kind:?} {why}: prefix");
        assert!(admitted.len() < log.len(), "{kind:?} {why}: stops early");
        assert_eq!(
            record.charge_point,
            log[admitted.len()].point,
            "{kind:?} {why}: the stop is at the next charge"
        );
        assert_eq!(record.limit_kind, kind, "{kind:?} {why}");
        assert_eq!(record.limit, limit, "{kind:?} {why}");
        assert_eq!(
            record.consumed,
            stopped.meter.consumed(kind),
            "{kind:?} {why}: only successful usage is retained"
        );
        assert!(record.consumed <= limit, "{kind:?} {why}");
    };

    let one_short = used - 1;
    if kind.is_cumulative() {
        let zero = call(package, case.function, arguments.clone(), only(kind, 0));
        assert_stopped_before(&zero, 0, "zero");
        assert_eq!(zero.meter.consumed(kind), 0);
        let short = call(
            package,
            case.function,
            arguments.clone(),
            only(kind, one_short),
        );
        assert_stopped_before(&short, one_short, "one-short");
        let record = short.incomplete();
        assert!(
            record.next_charge > Integer::from(one_short - record.consumed),
            "{kind:?}: the denied addition does not fit the remaining budget"
        );
    } else {
        let largest = log
            .iter()
            .filter_map(|charge| requested(charge, kind))
            .max();
        assert_eq!(
            largest,
            Some(used),
            "{kind:?}: a high-water counter is the largest single charge"
        );
        if let Some(counted) = case.counted {
            let at_point = log
                .iter()
                .filter(|charge| charge.point == case.point)
                .filter_map(|charge| requested(charge, kind))
                .max();
            assert_eq!(
                at_point,
                Some(counted),
                "{kind:?}: the independently counted quantity is charged at {:?}",
                case.point
            );
        }
        // The first admitted charge that asks for any of the counter.
        let first = log
            .iter()
            .position(|charge| requested(charge, kind).is_some_and(|size| size > 0))
            .expect("the log holds a nonzero request");
        let zero = call(package, case.function, arguments.clone(), only(kind, 0));
        assert_stopped_before(&zero, 0, "zero");
        assert_eq!(
            zero.log().len(),
            first,
            "{kind:?}: zero stops at the first request"
        );
        assert_eq!(
            zero.incomplete().next_charge,
            Integer::from(requested(&log[first], kind).expect("a request"))
        );

        let short = call(
            package,
            case.function,
            arguments.clone(),
            only(kind, one_short),
        );
        assert_stopped_before(&short, one_short, "one-short");
        assert_eq!(
            short.incomplete().next_charge,
            Integer::from(used),
            "{kind:?}: the denied size is the largest charge"
        );
        let before_denied = short.log();
        assert_eq!(
            short.incomplete().consumed,
            before_denied
                .iter()
                .filter_map(|charge| requested(charge, kind))
                .max()
                .unwrap_or(0),
            "{kind:?}: usage is the high-water of the admitted charges"
        );
    }

    let exact = call(package, case.function, arguments.clone(), only(kind, used));
    assert_eq!(
        exact.rendered(),
        full.rendered(),
        "{kind:?}: exact completes"
    );
    assert_eq!(exact.log(), log, "{kind:?}: exact admits every charge");
    assert_eq!(exact.usage(), full.usage(), "{kind:?}: exact usage");

    // After a resource stop, the same immutable package and inputs complete
    // on a fresh sufficient meter with the same value, log and usage.
    let retry = call(package, case.function, arguments.clone(), UNLIMITED);
    assert_eq!(retry.rendered(), full.rendered(), "{kind:?}: retry value");
    assert_eq!(retry.log(), log, "{kind:?}: retry charges");
    assert_eq!(retry.usage(), full.usage(), "{kind:?}: retry usage");
    assert_eq!(
        format!("{arguments:?}"),
        rendered_arguments,
        "{kind:?}: the inputs are unchanged"
    );
}

/// Every accounting counter has independently counted zero, exact and
/// one-short boundaries. The high-water oracle follows the full charge
/// schedule: arithmetic includes intermediate carry/alignment sizes, and
/// collection retention includes the container occurrence.
#[trace("TC-137", "FR-049-AC-7")]
#[test]
fn every_accounting_counter_has_zero_exact_and_one_short_evidence() {
    let package = package_of(UNIT);
    let cases = cases();
    assert_eq!(
        cases.iter().map(|case| case.counter).collect::<Vec<_>>(),
        COUNTERS,
        "one fixture for each counter of the accounting table"
    );
    for case in &cases {
        exercise(&package, case);
    }
}

/// TC-137 / FR-049-AC-7: a fixture that charges none of nine counters
/// completes at zero for each of them, and its zero `work_units`
/// stops at its first charge: no-work needs no budget, work needs one.
#[trace("TC-137", "FR-049-AC-7")]
#[test]
fn a_fixture_with_no_work_completes_at_zero_for_every_counter_but_work() {
    let package = package_of(UNIT);
    let full = call(&package, "nothing", Vec::new(), UNLIMITED);
    for kind in COUNTERS
        .iter()
        .filter(|kind| **kind != LimitKind::WorkUnits)
    {
        assert_eq!(full.meter.consumed(*kind), 0, "{kind:?}: no work");
        let at_zero = call(&package, "nothing", Vec::new(), only(*kind, 0));
        assert_eq!(at_zero.rendered(), full.rendered(), "{kind:?}: completes");
        assert_eq!(at_zero.log(), full.log(), "{kind:?}: every charge admitted");
    }
    let kind = LimitKind::WorkUnits;
    assert!(full.meter.consumed(kind) > 0, "the call itself spends work");
    let at_zero = call(&package, "nothing", Vec::new(), only(kind, 0));
    assert_eq!(at_zero.incomplete().limit_kind, kind);
    assert_eq!(at_zero.incomplete().limit, 0);
}

/// A checked Boolean constant performs no charged operation: even work_units
/// at zero admits the whole evaluation.
#[trace("TC-137", "FR-049-AC-7")]
#[test]
fn a_checked_constant_completes_with_every_counter_at_zero() {
    let package = package_of(UNIT);
    let expression = package
        .graph()
        .check_expression(
            Vec::new(),
            &Expression::boolean(true),
            None,
            CheckMode::Linked,
            CheckingLimits::default(),
        )
        .expect("the constant checks");
    let zero = ScalarLimits {
        integer_bits: 0,
        decimal_digits: 0,
        scale_expansion: 0,
        text_input_bytes: 0,
        text_scalars: 0,
        normalized_scalars: 0,
        unit_edges: 0,
        value_occurrences: 0,
        work_units: 0,
        result_units: 0,
    };
    let mut meter = Meter::new(zero);
    let evaluation = package
        .evaluate(
            &expression,
            Vec::new(),
            &ObjectEnvironment::default(),
            &mut meter,
        )
        .expect("the checked constant evaluates without a charged operation");
    assert!(matches!(
        kernel(evaluation),
        Outcome::Completed(Value::Boolean(true))
    ));
    assert_eq!(meter, Meter::new(zero), "no counter is charged");
}

/// TC-137: raising a caller-set counter above its default completes work the
/// default stops. FR-255 gives `work_units` the default
/// [`DEFAULT_WORK_UNITS`]; a 400,000-element sum exceeds it.
#[trace("TC-137")]
#[test]
fn raising_work_units_above_the_default_completes_work_the_default_stops() {
    let package = package_of(UNIT);
    let elements = 400_000;
    let arguments = || vec![ones(elements)];
    let full = call(&package, "walk", arguments(), UNLIMITED);
    assert_eq!(full.rendered(), "Completed(Integer(Integer(400000)))");
    let used = full.meter.consumed(LimitKind::WorkUnits);
    assert!(used > DEFAULT_WORK_UNITS, "the fixture exceeds the default");

    let stopped = call(
        &package,
        "walk",
        arguments(),
        only(LimitKind::WorkUnits, DEFAULT_WORK_UNITS),
    );
    let record = stopped.incomplete();
    assert_eq!(record.limit_kind, LimitKind::WorkUnits);
    assert_eq!(record.limit, DEFAULT_WORK_UNITS);
    assert!(stopped.meter.consumed(LimitKind::WorkUnits) <= DEFAULT_WORK_UNITS);

    let raised = call(
        &package,
        "walk",
        arguments(),
        only(LimitKind::WorkUnits, used),
    );
    assert_eq!(raised.rendered(), full.rendered());
    assert_eq!(raised.usage(), full.usage());
}

/// A checked expression calling `link0(1)`, evaluated through
/// `CheckedPackageEvaluation::evaluate`.
fn evaluate_link0(package: &CheckedPackage, meter: &mut Meter) -> Result<Evaluation, CallFailure> {
    let expression = package
        .graph()
        .check_expression(
            Vec::new(),
            &Expression::call("link0".to_owned(), vec![Expression::integer(1_i64)]),
            None,
            CheckMode::Linked,
            CheckingLimits::default(),
        )
        .expect("the expression checks");
    package.evaluate(
        &expression,
        Vec::new(),
        &ObjectEnvironment::default(),
        meter,
    )
}

/// TC-137 C1 / NFR-009-AC-3: a charge that would carry `work_units` past
/// `u64::MAX` returns the meter's incomplete record naming the counter and
/// charge point, with no completed value and no wrapped or saturated count.
#[trace("TC-137", "NFR-009-AC-3")]
#[test]
fn a_work_units_overflow_is_the_meters_incomplete_record() {
    let package = package_of(UNIT);
    let raised = || {
        let mut meter = Meter::new(UNLIMITED);
        meter
            .charge(Charge::new(ChargePoint::FunctionCall).work(Integer::from(u64::MAX)))
            .expect("the prior charge fits the unlimited budget");
        assert_eq!(meter.consumed(LimitKind::WorkUnits), u64::MAX);
        meter
    };
    let expect_overflow = |outcome: Outcome<Value>, meter: &Meter| {
        let Outcome::Incomplete(record) = outcome else {
            panic!("no completed value past u64::MAX: {outcome:?}");
        };
        assert_eq!(record.limit_kind, LimitKind::WorkUnits);
        assert_eq!(record.limit, u64::MAX);
        assert_eq!(record.consumed, u64::MAX);
        assert_eq!(record.next_charge, Integer::from(1_u64));
        assert_eq!(record.charge_point, ChargePoint::FunctionCall);
        assert_eq!(
            meter.consumed(LimitKind::WorkUnits),
            u64::MAX,
            "not wrapped"
        );
        assert_eq!(
            meter.admission_count(),
            1,
            "the denied charge is not admitted"
        );
    };

    let mut meter = raised();
    let evaluation = package
        .call(
            &name("link0"),
            vec![int(1)],
            &ObjectEnvironment::default(),
            &mut meter,
        )
        .expect("the call returns the incomplete record");
    expect_overflow(kernel(evaluation), &meter);

    let mut meter = raised();
    let evaluation = evaluate_link0(&package, &mut meter).expect("evaluate returns it too");
    expect_overflow(kernel(evaluation), &meter);

    let retry = call(&package, "link0", vec![int(1)], UNLIMITED);
    assert!(matches!(
        retry.outcome,
        Outcome::Completed(Value::Boolean(true))
    ));
    let mut fresh = Meter::new(UNLIMITED);
    let evaluation =
        evaluate_link0(&package, &mut fresh).expect("a fresh evaluate retry completes");
    assert!(matches!(
        kernel(evaluation),
        Outcome::Completed(Value::Boolean(true))
    ));
    assert_eq!(fresh.admitted_charges(), retry.meter.admitted_charges());
    for kind in COUNTERS {
        assert_eq!(
            fresh.consumed(kind),
            retry.meter.consumed(kind),
            "{kind:?}: fresh retry usage"
        );
    }
}

/// TC-137 C2 / NFR-009-AC-3: a charge denied for a reason other than a
/// configured limit, the stand-in for an allocator refusal, returns the
/// meter's incomplete record recorded against `work_units` at the denied
/// charge point, with `limit` equal to `consumed` and no completed value.
#[trace("TC-137", "NFR-009-AC-3")]
#[test]
fn an_injected_denial_is_the_meters_incomplete_record() {
    let package = package_of(UNIT);
    let denying = || {
        Meter::new(UNLIMITED).with_injected_denial(InjectedDenial {
            point: ChargePoint::FunctionCall,
            occurrence: NonZeroU64::MIN,
        })
    };
    let expect_denied = |outcome: Outcome<Value>, meter: &Meter| {
        let Outcome::Incomplete(record) = outcome else {
            panic!("no completed value under a denied charge: {outcome:?}");
        };
        assert_eq!(record.limit_kind, LimitKind::WorkUnits);
        assert_eq!(record.charge_point, ChargePoint::FunctionCall);
        assert_eq!(record.limit, record.consumed);
        assert_eq!(record.consumed, meter.consumed(LimitKind::WorkUnits));
        assert_eq!(
            meter.admission_count(),
            0,
            "the denied charge is not admitted"
        );
    };

    let run = call_on(&package, "link0", vec![int(1)], denying());
    expect_denied(run.outcome, &run.meter);

    let mut meter = denying();
    let evaluation = evaluate_link0(&package, &mut meter).expect("evaluate returns it too");
    expect_denied(kernel(evaluation), &meter);

    let retry = call(&package, "link0", vec![int(1)], UNLIMITED);
    assert!(matches!(
        retry.outcome,
        Outcome::Completed(Value::Boolean(true))
    ));
    let mut fresh = Meter::new(UNLIMITED);
    let evaluation =
        evaluate_link0(&package, &mut fresh).expect("a fresh evaluate retry completes");
    assert!(matches!(
        kernel(evaluation),
        Outcome::Completed(Value::Boolean(true))
    ));
    assert_eq!(fresh.admitted_charges(), retry.meter.admitted_charges());
    for kind in COUNTERS {
        assert_eq!(
            fresh.consumed(kind),
            retry.meter.consumed(kind),
            "{kind:?}: fresh retry usage"
        );
    }
}

/// Run `body` against `package` on a thread with a 512 KiB stack.
fn on_small_stack<T: Send>(body: impl FnOnce() -> T + Send) -> T {
    std::thread::scope(|scope| {
        std::thread::Builder::new()
            .stack_size(512 * 1024)
            .spawn_scoped(scope, body)
            .expect("spawn a 512 KiB thread")
            .join()
            .expect("the evaluation does not overflow a 512 KiB stack")
    })
}

/// `links` predicates, `q0` returning true and each later one calling the
/// one before.
fn predicate_chain(links: usize) -> String {
    let mut source = String::from("function q0 using v(x: Integer): Boolean pure { true }\n");
    for link in 1..links {
        source.push_str(&format!(
            "function q{link} using v(x: Integer): Boolean pure {{ q{}(x) }}\n",
            link - 1
        ));
    }
    source
}

/// TC-137: a 10,000-call nested predicate chain completes on a 512 KiB
/// stack, spends one call charge per predicate, and stops one unit below
/// the independently counted number of predicates.
#[trace("TC-137")]
#[test]
fn a_ten_thousand_call_predicate_chain_completes_on_a_512_kib_stack() {
    const LINKS: usize = 10_000;
    // Each predicate has one call charge; the leaf is a Boolean constant.
    let expected = LINKS as u64;

    let package = package_of(&predicate_chain(LINKS));
    let last = format!("q{}", LINKS - 1);
    on_small_stack(|| {
        let full = call(&package, &last, vec![int(1)], UNLIMITED);
        assert_eq!(full.rendered(), "Completed(Boolean(true))");
        assert_eq!(full.meter.consumed(LimitKind::WorkUnits), expected);
        let short = call(
            &package,
            &last,
            vec![int(1)],
            only(LimitKind::WorkUnits, expected - 1),
        );
        let record = short.incomplete();
        assert_eq!(record.limit_kind, LimitKind::WorkUnits);
        assert_eq!(record.limit, expected - 1);
        let exact = call(
            &package,
            &last,
            vec![int(1)],
            only(LimitKind::WorkUnits, expected),
        );
        assert_eq!(exact.rendered(), full.rendered());
    });
}

/// TC-137: a 100,000-record-deep input, built, evaluated and dropped on a
/// 512 KiB stack, completes with its value and the same result and usage on a
/// fresh sufficient meter.
#[trace("TC-137")]
#[test]
fn a_hundred_thousand_record_deep_input_completes_on_a_512_kib_stack() {
    const DEPTH: usize = 100_000;
    let package = package_of(UNIT);
    on_small_stack(|| {
        let types = package.graph().scope().types();
        let declaration = types
            .composites()
            .find(|composite| composite.name() == "Chain")
            .expect("the unit declares Chain")
            .key();
        let link = |tail: quire_exact::FieldValue| {
            types
                .record(
                    declaration.clone(),
                    vec![
                        ("head", quire_exact::FieldValue::Present(int(1))),
                        ("tail", tail),
                    ],
                )
                .expect("a well-typed link")
        };
        let mut deep = link(quire_exact::FieldValue::Absent);
        for _ in 1..DEPTH {
            deep = link(quire_exact::FieldValue::Present(deep));
        }
        let first = call(&package, "head_above", vec![deep.clone()], UNLIMITED);
        assert_eq!(first.rendered(), "Completed(Boolean(false))");
        let retry = call(&package, "head_above", vec![deep], UNLIMITED);
        assert_eq!(retry.rendered(), first.rendered());
        assert_eq!(retry.usage(), first.usage());
    });
}
