// SPDX-License-Identifier: AGPL-3.0-or-later
//! Unit-language `=` and `!=` on native `String`, `UUID` and `Timestamp`,
//! evaluated from source text (TC-1028 E1 to E14 and F1 to F6).
//!
//! Operand values are supplied as call arguments or read from a record's
//! fields; the unit language has no literal for these types.

use ix_trace_rs::trace;
use qsl_eval::value::{CheckedPackageEvaluation, QualifiedName};
use qsl_package::CheckedPackage;
use qsl_semantics::check::PackageDeclarations;
use qsl_semantics::family::FamilyOutcome;
use qsl_semantics::model::intake::SelectedModels;
use qsl_semantics::model::object_environment::ObjectEnvironment;
use quire_exact::{
    ConstructionCause, FieldValue, Meter, Outcome, ScalarLimits, Timestamp, Uuid, Value, ValueType,
};
use quire_semantic_value::checking::CheckingLimits;
use quire_semantic_value::declaration::TypeEnvironment;

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

const FUNCTIONS: &str = "\
    record Event { id: UUID; moment: Timestamp; label: String; }\n\
    function eqs using v(a: String, b: String): Boolean pure { a = b }\n\
    function nes using v(a: String, b: String): Boolean pure { a != b }\n\
    function equ using v(a: UUID, b: UUID): Boolean pure { a = b }\n\
    function neu using v(a: UUID, b: UUID): Boolean pure { a != b }\n\
    function eqt using v(a: Timestamp, b: Timestamp): Boolean pure { a = b }\n\
    function net using v(a: Timestamp, b: Timestamp): Boolean pure { a != b }\n\
    function ft using v(x: Event, y: Event): Boolean pure { x.moment = y.moment }\n\
    function fu using v(x: Event, y: Event): Boolean pure { x.id != y.id }\n\
    function fs using v(x: Event, y: Event): Boolean pure { x.label = y.label }\n\
    function stamp using v(e: Event): Timestamp pure { e.moment }\n\
    function fc using v(x: Event, y: Event): Boolean pure { stamp(x) = stamp(y) }\n\
    function fl using v(x: Event, y: Event): Boolean pure { let p = x.id in let q = y.id in p = q }\n";

const UUID_A: &str = "00112233-4455-6677-8899-aabbccddeeff";
const UUID_B: &str = "00112233-4455-6677-8899-aabbccddeefe";
const MIN: &str = "-170141183460469231731687303715884105728";
const MAX: &str = "170141183460469231731687303715884105727";

struct Fixture {
    package: CheckedPackage,
    types: TypeEnvironment,
}

fn fixture() -> Fixture {
    let text = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n{FUNCTIONS}"
    );
    let parsed = qsl_cst::parse(
        qsl_foundation::SourceIdentity::new("a", "u", "git", "1"),
        "unit.native",
        text.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let unit = qsl_forms::build_unit(&parsed).expect("S2 builds the unit");
    let declarations = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        unit,
        SelectedModels::default(),
        Vec::new(),
    )
    .expect("the assembler builds the declarations");
    let types = declarations.types.clone();
    let package = CheckedPackage::link(
        declarations
            .check(CheckingLimits::default())
            .expect("the unit checks"),
    );
    Fixture { package, types }
}

fn call(fixture: &Fixture, name: &str, arguments: Vec<Value>) -> Outcome<Value> {
    let evaluation = fixture
        .package
        .call(
            &QualifiedName::unqualified(name).expect("an identifier"),
            arguments,
            &ObjectEnvironment::default(),
            &mut Meter::new(UNLIMITED),
        )
        .expect("the call runs");
    match evaluation.outcome {
        FamilyOutcome::Evaluated(outcome) => outcome,
        other => panic!("a kernel outcome, not {other:?}"),
    }
}

fn truth(outcome: Outcome<Value>) -> bool {
    match outcome {
        Outcome::Completed(Value::Boolean(truth)) => truth,
        other => panic!("a completed Boolean, not {other:?}"),
    }
}

fn string(text: &str) -> Value {
    Value::String(text.to_owned())
}

fn uuid(text: &str) -> Value {
    Value::Uuid(Uuid::from_canonical_text(text).expect("canonical UUID"))
}

fn timestamp(text: &str) -> Value {
    Value::Timestamp(Timestamp::from_canonical_text(text).expect("canonical Timestamp"))
}

/// Both operands of `=` and `!=` for one vector: `=` gives `expected` and
/// `!=` its negation (E14).
fn assert_pair(fixture: &Fixture, (eq, ne): (&str, &str), a: Value, b: Value, expected: bool) {
    assert_eq!(
        truth(call(fixture, eq, vec![a.clone(), b.clone()])),
        expected,
        "{eq}"
    );
    assert_eq!(truth(call(fixture, ne, vec![a, b])), !expected, "{ne}");
}

/// TC-1028 E1 to E5, E14: String equality is the identical scalar sequence.
#[trace("TC-1028")]
#[test]
fn string_equality_is_the_identical_scalar_sequence() {
    let fixture = fixture();
    let names = ("eqs", "nes");
    assert_pair(&fixture, names, string(""), string(""), true);
    assert_pair(&fixture, names, string("abc"), string("abc"), true);
    assert_pair(&fixture, names, string("\u{e9}"), string("e\u{301}"), false);
    assert_pair(&fixture, names, string("A"), string("a"), false);
    assert_pair(&fixture, names, string(""), string("a"), false);
    assert_pair(&fixture, names, string(UUID_A), string(UUID_A), true);
}

/// TC-1028 E6, E7, E14: UUID equality is payload equality.
#[trace("TC-1028")]
#[test]
fn uuid_equality_is_payload_equality() {
    let fixture = fixture();
    let names = ("equ", "neu");
    assert_pair(&fixture, names, uuid(UUID_A), uuid(UUID_A), true);
    assert_pair(&fixture, names, uuid(UUID_A), uuid(UUID_B), false);
}

/// TC-1028 E8, kernel half only: this pins quire-exact's refusal of the
/// uppercase spelling. It does not test the QSL runtime-input adapter, which
/// is not built here, so it supports no FR-056-AC-35 claim.
#[trace("TC-1028")]
#[test]
fn a_noncanonical_uuid_spelling_is_refused_before_any_value_exists() {
    let refusal = Uuid::from_canonical_text("00112233-4455-6677-8899-AABBCCDDEEFF")
        .expect_err("uppercase is noncanonical");
    assert_eq!(refusal.cause, ConstructionCause::UuidNoncanonical);
}

/// TC-1028 E9 to E13, E14: Timestamp equality is nanosecond-count equality.
#[trace("TC-1028")]
#[test]
fn timestamp_equality_is_nanosecond_count_equality() {
    let fixture = fixture();
    let names = ("eqt", "net");
    assert_pair(&fixture, names, timestamp("0"), timestamp("0"), true);
    assert_pair(&fixture, names, timestamp("0"), timestamp("1"), false);
    assert_pair(&fixture, names, timestamp("-1"), timestamp("0"), false);
    assert_pair(&fixture, names, timestamp(MIN), timestamp(MIN), true);
    assert_pair(&fixture, names, timestamp(MAX), timestamp(MAX), true);
    assert_pair(&fixture, names, timestamp(MAX), timestamp(MIN), false);
}

fn event(fixture: &Fixture, id: &str, at: &str, label: &str) -> Value {
    let ValueType::Composite(key) = event_type(fixture) else {
        panic!("Event is a composite");
    };
    fixture
        .types
        .record(
            key,
            vec![
                ("id", FieldValue::Present(uuid(id))),
                ("moment", FieldValue::Present(timestamp(at))),
                ("label", FieldValue::Present(string(label))),
            ],
        )
        .expect("an Event")
}

fn event_type(fixture: &Fixture) -> ValueType {
    let callable = fixture.package.graph().callable("ft").expect("ft");
    callable.parameters[0].1.clone()
}

/// TC-1028 F1 to F6: field reads, call results and `let` bindings compare by
/// the native rules.
#[trace("TC-1028")]
#[test]
fn field_call_result_and_let_operands_evaluate() {
    let fixture = fixture();
    let a = event(&fixture, UUID_A, "5", "a");
    let b = event(&fixture, UUID_B, "6", "b");
    let same =
        |name: &str, x: &Value, y: &Value| truth(call(&fixture, name, vec![x.clone(), y.clone()]));
    // F1, F2
    assert!(same("ft", &a, &a));
    assert!(!same("ft", &a, &b));
    // F3
    assert!(same("fu", &a, &b));
    assert!(!same("fu", &a, &a));
    // F4
    assert!(same("fs", &a, &a));
    assert!(!same("fs", &a, &b));
    // F5
    assert!(same("fc", &a, &a));
    assert!(!same("fc", &a, &b));
    // F6
    assert!(same("fl", &a, &a));
    assert!(!same("fl", &a, &b));
}
