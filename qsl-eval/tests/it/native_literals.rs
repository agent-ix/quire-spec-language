// SPDX-License-Identifier: AGPL-3.0-or-later
//! `String`, `UUID` and `Timestamp` literals evaluated from source text
//! (FR-056-AC-40, TC-1052 step 4).

use ix_trace_rs::trace;
use qsl_eval::value::{CheckedPackageEvaluation, QualifiedName};
use qsl_package::CheckedPackage;
use qsl_semantics::check::PackageDeclarations;
use qsl_semantics::model::intake::SelectedModels;
use qsl_semantics::model::object_environment::ObjectEnvironment;
use quire_eval::FamilyOutcome;
use quire_exact::{
    FieldValue, Meter, Outcome, ScalarLimits, TextProvenance, Timestamp, Uuid, Value, ValueType,
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
    function isE using v(t: Text[0, 8; binary-utf8]): Boolean pure { t = Text(\"é\") }\n\
    function isEscapedE using v(t: Text[0, 8; binary-utf8]): Boolean pure { t = Text(\"\\u00e9\") }\n\
    function ends using v(): Boolean pure { Timestamp(-170141183460469231731687303715884105728) = Timestamp(170141183460469231731687303715884105727) }\n";

struct Fixture {
    package: CheckedPackage,
    types: TypeEnvironment,
}

fn fixture() -> Fixture {
    fixture_of(FUNCTIONS)
}

fn fixture_of(functions: &str) -> Fixture {
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
    let declarations = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        "fixture".to_owned(),
        unit,
        SelectedModels::default(),
        Vec::new(),
    )
    .expect("the assembler builds the declarations");
    let types = declarations.types.clone();
    let package = CheckedPackage::link(
        declarations
            .check(
                CheckingLimits::default(),
                quire_semantic_value::checking::CheckMode::Linked,
            )
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
        assert_eq!(
            truth(&fixture, "ns", vec![string(text)]),
            !equal,
            "{text:?}"
        );
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
    let ValueType::Composite(ref key) = fixture
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
            .record(
                key.clone(),
                vec![("label", FieldValue::Present(string(label)))],
            )
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

/// The completed value of the zero-argument function `name`.
fn completed(fixture: &Fixture, name: &str) -> Value {
    let evaluation = fixture
        .package
        .call(
            &QualifiedName::unqualified(name).expect("an identifier"),
            vec![],
            &ObjectEnvironment::default(),
            &mut Meter::new(UNLIMITED),
        )
        .expect("the call runs");
    match evaluation.outcome {
        FamilyOutcome::Evaluated(Outcome::Completed(value)) => value,
        other => panic!("a completed value, not {other:?}"),
    }
}

/// Each group is one text, written in each of its spellings.
const SPELLING_GROUPS: [(&str, &[&str]); 6] = [
    ("\u{e9}", &["\"\u{e9}\"", "\"\\u00e9\"", "\"\\u00E9\""]),
    ("\n", &["\"\\n\"", "\"\\u000a\""]),
    ("\u{1f600}", &["\"\u{1f600}\"", "\"\\ud83d\\ude00\""]),
    ("a/b", &["\"a/b\"", "\"a\\/b\""]),
    ("e\u{301}", &["\"e\\u0301\"", "\"e\u{301}\""]),
    ("abc", &["\"abc\"", "\"abc\""]),
];

/// FR-091-AC-70, QSpec TC-186 T06b: under a scalar profile and a normalizing
/// one, every spelling of a text holds that text's payload bytes and carries
/// its own quoted spelling as provenance; equal spellings carry equal
/// provenance and distinct spellings distinct provenance.
#[trace("FR-091-AC-70", "TC-1052", "QSpec-TC-186", "QSpec-FR-141-AC-4")]
#[test]
fn text_literals_keep_their_source_spelling_as_provenance() {
    let mut functions = String::new();
    let mut expected = Vec::new();
    for profile in ["binary-utf8", "nfc"] {
        for (group, (text, spellings)) in SPELLING_GROUPS.iter().enumerate() {
            for (index, spelling) in spellings.iter().enumerate() {
                let name = format!("lit_{profile}_{group}_{index}").replace('-', "_");
                functions.push_str(&format!(
                    "function {name} using v(): Text[0, 8; {profile}] pure {{ Text({spelling}) }}\n"
                ));
                expected.push((name, *text, *spelling, group));
            }
        }
    }
    let fixture = fixture_of(&functions);
    let mut seen: Vec<(usize, &str, TextProvenance)> = Vec::new();
    for (name, text, spelling, group) in &expected {
        let Value::Text(value) = completed(&fixture, name) else {
            panic!("{name}: a Text value");
        };
        assert_eq!(value.payload().as_str(), *text, "{name}");
        assert_eq!(value.payload().bytes(), text.as_bytes(), "{name}");
        let provenance = value.payload().provenance().clone();
        assert_eq!(
            provenance,
            TextProvenance::SourceLiteral((*spelling).into()),
            "{name}"
        );
        seen.push((*group, spelling, provenance));
    }
    for (group, spelling, provenance) in &seen {
        for (other_group, other_spelling, other) in &seen {
            if group == other_group {
                assert_eq!(
                    spelling == other_spelling,
                    provenance == other,
                    "{spelling}"
                );
            }
        }
    }
}

/// QSpec TC-186 T06b: the values written `Text("\u{e9}")` and
/// `Text("\u00e9")` and the runtime payload bytes `c3 a9` compare equal
/// to each other, whichever side the literal is on.
#[trace("FR-091-AC-70", "TC-1052", "QSpec-TC-186", "QSpec-FR-141-AC-4")]
#[test]
fn raw_escaped_and_runtime_text_compare_equal() {
    let fixture = fixture();
    let binary = quire_exact::TextType::new(0, 8, quire_exact::TextProfile::BinaryUtf8)
        .expect("a Text type");
    let admit = |payload: quire_exact::TextPayload| {
        let Outcome::Completed(text) =
            quire_exact::admit_text(&payload, &binary, &mut Meter::new(UNLIMITED))
        else {
            panic!("the payload is admitted");
        };
        Value::Text(text)
    };
    let runtime = admit(quire_exact::TextPayload::from_utf8(&[0xc3, 0xa9]).expect("UTF-8"));
    let raw = admit(quire_exact::TextPayload::from_source_literal(
        "\u{e9}",
        "\"\u{e9}\"",
    ));
    let escaped = admit(quire_exact::TextPayload::from_source_literal(
        "\u{e9}",
        "\"\\u00e9\"",
    ));
    for argument in [&runtime, &raw, &escaped] {
        assert!(truth(&fixture, "isE", vec![argument.clone()]));
        assert!(truth(&fixture, "isEscapedE", vec![argument.clone()]));
    }
    let other = admit(quire_exact::TextPayload::from_utf8(b"e").expect("UTF-8"));
    assert!(!truth(&fixture, "isE", vec![other.clone()]));
    assert!(!truth(&fixture, "isEscapedE", vec![other]));
}
