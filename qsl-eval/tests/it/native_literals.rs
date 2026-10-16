// SPDX-License-Identifier: AGPL-3.0-or-later
//! `String`, `UUID` and `Timestamp` literals evaluated from source text
//! (FR-056-AC-40, TC-1052 step 4).

use ix_trace_rs::trace;
use qsl_eval::value::{CheckedPackageEvaluation, QualifiedName};
use qsl_package::CheckedPackage;
use qsl_semantics::check::PackageDeclarations;
use qsl_semantics::family::FamilyOutcome;
use qsl_semantics::model::intake::SelectedModels;
use qsl_semantics::model::object_environment::ObjectEnvironment;
use quire_exact::{FieldValue, Meter, Outcome, ScalarLimits, Timestamp, Uuid, Value, ValueType};
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

const U1: &str = "00112233-4455-6677-8899-aabbccddeeff";
const U2: &str = "00112233-4455-6677-8899-aabbccddeefe";

const FUNCTIONS: &str = "\
    record R { label: String; }\n\
    function s using v(x: String): Boolean pure { x = \"abc\" }\n\
    function ns using v(x: String): Boolean pure { x != \"abc\" }\n\
    function same using v(a: String, b: String): Boolean pure { a = b }\n\
    function lits using v(): Boolean pure { \"a\" = \"a\" }\n\
    function folded using v(): Boolean pure { \"\\u00e9\" = \"\u{e9}\" }\n\
    function decomposed using v(): Boolean pure { \"\\u00e9\" = \"e\\u0301\" }\n\
    function cased using v(): Boolean pure { \"A\" = \"a\" }\n\
    function viaCall using v(): Boolean pure { same(\"a\", \"a\") }\n\
    function viaLet using v(): Boolean pure { let x = \"a\" in x = \"a\" }\n\
    function viaField using v(r: R): Boolean pure { r.label = \"a\" }\n\
    function u using v(u: UUID): Boolean pure { u = UUID(\"00112233-4455-6677-8899-aabbccddeeff\") }\n\
    function nu using v(u: UUID): Boolean pure { u != UUID(\"00112233-4455-6677-8899-aabbccddeefe\") }\n\
    function t using v(t: Timestamp): Boolean pure { t = Timestamp(5) }\n\
    function zero using v(): Boolean pure { Timestamp(0) = Timestamp(-0) }\n\
    function one using v(): Boolean pure { Timestamp(0) = Timestamp(1) }\n\
    function ends using v(): Boolean pure { Timestamp(-170141183460469231731687303715884105728) = Timestamp(170141183460469231731687303715884105727) }\n";

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

fn truth(fixture: &Fixture, name: &str, arguments: Vec<Value>) -> bool {
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
        FamilyOutcome::Evaluated(Outcome::Completed(Value::Boolean(truth))) => truth,
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

/// FR-056-AC-40: a String parameter compares to `"abc"` by the exact scalar
/// sequence.
#[trace("FR-056-AC-40", "TC-1052")]
#[test]
fn a_string_parameter_compares_with_a_string_literal() {
    let fixture = fixture();
    for (text, equal) in [("abc", true), ("ABC", false), ("abc ", false), ("", false)] {
        assert_eq!(truth(&fixture, "s", vec![string(text)]), equal, "{text:?}");
        assert_eq!(truth(&fixture, "ns", vec![string(text)]), !equal, "{text:?}");
    }
}

/// FR-056-AC-40: two literals compare by the same rules, with no
/// normalization or case folding.
#[trace("FR-056-AC-40", "TC-1052")]
#[test]
fn string_literals_compare_by_the_exact_scalar_sequence() {
    let fixture = fixture();
    assert!(truth(&fixture, "lits", vec![]));
    assert!(truth(&fixture, "folded", vec![]));
    assert!(!truth(&fixture, "decomposed", vec![]));
    assert!(!truth(&fixture, "cased", vec![]));
}

/// FR-056-AC-40: a literal compares as a call argument, a `let` binding and
/// a field read.
#[trace("FR-056-AC-40", "TC-1052")]
#[test]
fn a_literal_compares_in_a_call_a_binding_and_a_field_read() {
    let fixture = fixture();
    assert!(truth(&fixture, "viaCall", vec![]));
    assert!(truth(&fixture, "viaLet", vec![]));
    let ValueType::Composite(key) = fixture
        .package
        .graph()
        .callable("viaField")
        .expect("viaField")
        .parameters[0]
        .1
        .clone()
    else {
        panic!("R is a composite");
    };
    for (label, equal) in [("a", true), ("b", false)] {
        let record = fixture
            .types
            .record(key, vec![("label", FieldValue::Present(string(label)))])
            .expect("an R");
        assert_eq!(truth(&fixture, "viaField", vec![record]), equal, "{label}");
    }
}

/// FR-056-AC-40: `UUID("...")` and `Timestamp(n)` compare as values.
#[trace("FR-056-AC-40", "TC-1052")]
#[test]
fn uuid_and_timestamp_literals_compare_by_value() {
    let fixture = fixture();
    assert!(truth(&fixture, "u", vec![uuid(U1)]));
    assert!(!truth(&fixture, "u", vec![uuid(U2)]));
    assert!(truth(&fixture, "nu", vec![uuid(U1)]));
    assert!(!truth(&fixture, "nu", vec![uuid(U2)]));
    assert!(truth(&fixture, "t", vec![timestamp("5")]));
    assert!(!truth(&fixture, "t", vec![timestamp("6")]));
    assert!(truth(&fixture, "zero", vec![]));
    assert!(!truth(&fixture, "one", vec![]));
    assert!(!truth(&fixture, "ends", vec![]));
}
