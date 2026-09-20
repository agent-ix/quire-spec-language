// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-186 text profiles and declaration-qualified enum identity over the real
//! `value` boundary.
//!
//! T01–T15 are transcribed from the vendored TC-186 procedure. Every enum node
//! key is a fixture-supplied value: the pinned `node-identity-vectors.json`
//! digests, or keys from this file's independent RFC 8785 canonicalizer, which
//! is first shown to reproduce every pinned vector digest.

use std::cmp::Ordering;
use std::path::Path;

use ix_trace_rs::trace;
use quire_exact::Integer;
use quire_spec_language::complete;
use quire_spec_language::value::{
    admit_text, compare_enum, compare_text, ChargePoint, ComparisonOperator, EmptyTextBounds,
    EnumDeclaration, EnumDeclarationPreimage, EnumMemberPreimage, EnumValue, IllTyped,
    IllTypedCause, Incomplete, InjectedDenial, InvalidSemanticGraph, InvalidUtf8, LimitKind, Meter,
    NodeKey, NodeOwner, Outcome, OwnerSelection, OwnerSubject, Refusal, ScalarLimits,
    SemanticGraphCause, Text, TextPayload, TextProfile, TextProvenance, TextType, NODE_KEY_DOMAIN,
    UNICODE_TEXT_DEFINITION,
};
use quire_spec_language::{Code, Limits, SourceIdentity};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use unicode_normalization::UnicodeNormalization;

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

use ComparisonOperator::{Equal, GreaterOrEqual, Less, NotEqual};
use TextProfile::{BinaryUtf8, Nfc, Nfd, Nfkc, Nfkd, UnicodeScalars};

// ---- text helpers ----------------------------------------------------------

fn runtime(text: &str) -> TextPayload {
    TextPayload::from_utf8(text.as_bytes()).unwrap()
}

fn text_type(min: u64, max: u64, profile: TextProfile) -> TextType {
    TextType::new(min, max, profile).unwrap()
}

fn admit(text: &str, text_type: &TextType) -> Outcome<Text> {
    admit_text(&runtime(text), text_type, &mut Meter::new(UNLIMITED))
}

fn value(text: &str, profile: TextProfile) -> Text {
    admit(text, &text_type(0, 64, profile)).completed().unwrap()
}

fn compared(operator: ComparisonOperator, left: &Text, right: &Text) -> bool {
    compare_text(operator, left, right, &mut Meter::new(UNLIMITED))
        .unwrap()
        .completed()
        .unwrap()
}

const E_ACUTE: &str = "\u{e9}";
const E_COMBINING: &str = "e\u{301}";

#[trace("TC-186", "FR-141-AC-1", "FR-141-AC-4")]
#[test]
fn t01_scalar_profile_distinguishes_canonically_equivalent_text() {
    let (left, right) = (
        value(E_ACUTE, UnicodeScalars),
        value(E_COMBINING, UnicodeScalars),
    );
    assert!(!compared(Equal, &left, &right));
    assert!(compared(NotEqual, &left, &right));
    assert_eq!((left.retained(), right.retained()), (E_ACUTE, E_COMBINING));
    assert_eq!((left.length(), right.length()), (1, 2));
    assert_eq!(UnicodeScalars.table_definition(), None);
}

#[trace("TC-186", "FR-141-AC-1")]
#[test]
fn t02_t03_normalizing_profiles_equate_equivalents_and_retain_normal_forms() {
    for (profile, retained) in [(Nfc, E_ACUTE), (Nfd, E_COMBINING)] {
        let (left, right) = (value(E_ACUTE, profile), value(E_COMBINING, profile));
        assert!(compared(Equal, &left, &right), "{profile:?}");
        assert_eq!((left.retained(), right.retained()), (retained, retained));
        assert_eq!(left.payload().as_str(), E_ACUTE);
        assert_eq!(profile.table_definition(), Some(UNICODE_TEXT_DEFINITION));
    }
    for profile in [Nfkc, Nfkd] {
        assert!(
            compared(Equal, &value("\u{fb00}", profile), &value("ff", profile)),
            "{profile:?}"
        );
    }
    assert!(!compared(Equal, &value("\u{fb00}", Nfc), &value("ff", Nfc)));
}

#[trace("TC-186", "FR-141-AC-3", "FR-141-AC-4")]
#[test]
fn t04_t05_profile_length_domains_admit_exact_bounds_and_refuse_one_over() {
    let one_scalar = admit(E_ACUTE, &text_type(1, 1, Nfc)).completed().unwrap();
    assert_eq!(one_scalar.length(), 1);
    assert!(matches!(
        admit(E_ACUTE, &text_type(1, 1, BinaryUtf8)),
        Outcome::Refused(Refusal::TextLengthOutOfDomain)
    ));
    assert_eq!(
        admit(E_ACUTE, &text_type(2, 2, BinaryUtf8))
            .completed()
            .unwrap()
            .length(),
        2
    );

    assert_eq!(
        admit("", &text_type(0, 0, Nfc))
            .completed()
            .unwrap()
            .length(),
        0
    );
    assert!(matches!(
        admit("a", &text_type(0, 0, Nfc)),
        Outcome::Refused(Refusal::TextLengthOutOfDomain)
    ));
    assert_eq!(
        admit("a", &text_type(1, 1, Nfc))
            .completed()
            .unwrap()
            .length(),
        1
    );
    assert_eq!(
        TextType::new(2, 1, Nfc),
        Err(EmptyTextBounds { min: 2, max: 1 })
    );
}

fn complete_source(declarations: &str) -> String {
    format!(
        "language \"ix:native\" edition \"1-draft\";\nprofile Complete = \"quire.value.complete/v1\" version \"1\" digest \"sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\";\n{declarations}"
    )
}

fn parse(text: &str) -> complete::ParsedSource {
    complete::parse(
        SourceIdentity {
            identity: "test:tc-186".into(),
            revision: "1".into(),
        },
        "tc-186.native",
        text.as_bytes(),
        Limits::default(),
    )
    .unwrap()
}

#[trace("TC-186", "FR-141-AC-3")]
#[test]
fn t05b_text_without_bounds_is_invalid_syntax_at_source_recognition() {
    let bounded = parse(&complete_source(
        "record Reading { label: Text[0, 1; nfc]; }",
    ));
    assert!(bounded.is_admissible(), "{:?}", bounded.diagnostics());

    let unbounded_text = complete_source("record Reading { label: Text; }");
    let unbounded = parse(&unbounded_text);
    assert!(!unbounded.is_admissible());
    let diagnostic = &unbounded.diagnostics()[0];
    assert_eq!(diagnostic.code, Code::InvalidSyntax);
    let at = unbounded_text.find("Text;").unwrap() + "Text".len();
    assert_eq!(diagnostic.span.start.byte, at);
}

#[trace("TC-186", "FR-141-AC-3", "FR-141-AC-4")]
#[test]
fn t06_t06b_utf8_reader_refuses_invalid_bytes_and_payloads_keep_provenance() {
    assert_eq!(
        TextPayload::from_utf8(&[0xc3, 0x28]),
        Err(InvalidUtf8 { valid_up_to: 0 })
    );
    // Lone surrogates are not scalars and refuse at the source literal too.
    assert!(TextPayload::from_source_literal("\"\\ud800\"").is_err());
    assert!(TextPayload::from_source_literal("\"a\" \"b\"").is_err());

    let raw = TextPayload::from_source_literal("\"\u{e9}\"").unwrap();
    let escaped = TextPayload::from_source_literal("\"\\u00e9\"").unwrap();
    let bytes = TextPayload::from_utf8(&[0xc3, 0xa9]).unwrap();
    let binary = text_type(2, 2, BinaryUtf8);
    let values: Vec<Text> = [&raw, &escaped, &bytes]
        .into_iter()
        .map(|payload| {
            admit_text(payload, &binary, &mut Meter::new(UNLIMITED))
                .completed()
                .unwrap()
        })
        .collect();
    for left in &values {
        assert_eq!(left.retained().as_bytes(), [0xc3, 0xa9]);
        for right in &values {
            assert!(compared(Equal, left, right));
        }
    }
    assert_eq!(
        raw.provenance(),
        &TextProvenance::SourceLiteral("\"\u{e9}\"".into())
    );
    assert_eq!(
        escaped.provenance(),
        &TextProvenance::SourceLiteral("\"\\u00e9\"".into())
    );
    assert_eq!(bytes.provenance(), &TextProvenance::Runtime);
}

#[trace("TC-186", "FR-141-AC-4")]
#[test]
fn t10_equal_payloads_under_distinct_profiles_are_ill_typed() {
    let nfc = value(E_ACUTE, Nfc);
    let binary = value(E_ACUTE, BinaryUtf8);
    for operator in ComparisonOperator::ALL {
        let mut meter = Meter::new(UNLIMITED);
        assert!(matches!(
            compare_text(operator, &nfc, &binary, &mut meter),
            Err(IllTyped {
                cause: IllTypedCause::DistinctTextProfiles
            })
        ));
        assert!(meter.admitted_charges().is_empty());
    }
    assert_eq!(IllTyped::CODE, "ill_typed");
}

const T11: ScalarLimits = ScalarLimits {
    integer_bits: 0,
    decimal_digits: 0,
    scale_expansion: 0,
    text_input_bytes: 5,
    text_scalars: 3,
    normalized_scalars: 2,
    unit_edges: 0,
    value_occurrences: 2,
    work_units: 6,
    result_units: 1,
};

fn consumed(meter: &Meter) -> Vec<u64> {
    LimitKind::ALL
        .into_iter()
        .map(|kind| meter.consumed(kind))
        .collect()
}

#[trace("TC-186", "FR-141-AC-6")]
#[test]
fn t11_text_accounting_exact_bounds_and_named_denials() {
    let (left, right) = (value(E_ACUTE, Nfc), value(E_COMBINING, Nfc));
    let run = |meter: &mut Meter| compare_text(Equal, &left, &right, meter).unwrap();

    let mut exact = Meter::new(T11);
    assert_eq!(run(&mut exact), Outcome::Completed(true));
    assert_eq!(consumed(&exact), [0, 0, 0, 5, 3, 2, 0, 2, 6, 1]);
    assert_eq!(
        exact.admitted_charges(),
        [
            ChargePoint::TextInputBytes,
            ChargePoint::TextDecodeScalars,
            ChargePoint::TextNormalizeInput,
            ChargePoint::TextNormalizeOutput,
            ChargePoint::TextNormalizeOutput,
            ChargePoint::TextResultRetain,
        ]
    );

    let one_less = ScalarLimits {
        work_units: 5,
        ..T11
    };
    assert_eq!(
        run(&mut Meter::new(one_less)),
        Outcome::Incomplete(Incomplete {
            limit_kind: LimitKind::WorkUnits,
            limit: 5,
            consumed: 5,
            next_charge: Integer::from(1_i64),
            charge_point: ChargePoint::TextResultRetain,
        })
    );
    assert_eq!(
        run(&mut Meter::new(ScalarLimits {
            normalized_scalars: 1,
            ..T11
        })),
        Outcome::Incomplete(Incomplete {
            limit_kind: LimitKind::NormalizedScalars,
            limit: 1,
            consumed: 1,
            next_charge: Integer::from(2_i64),
            charge_point: ChargePoint::TextNormalizeOutput,
        })
    );

    for (point, occurrence) in [
        (ChargePoint::TextNormalizeOutput, 2),
        (ChargePoint::TextResultRetain, 1),
        (ChargePoint::TextInputBytes, 1),
        (ChargePoint::TextDecodeScalars, 1),
        (ChargePoint::TextNormalizeInput, 1),
        (ChargePoint::TextNormalizeOutput, 1),
    ] {
        let mut denied = Meter::new(T11).with_injected_denial(InjectedDenial { point, occurrence });
        assert!(matches!(
            run(&mut denied),
            Outcome::Incomplete(Incomplete { charge_point, .. }) if charge_point == point
        ));
        assert_eq!(denied.consumed(LimitKind::ResultUnits), 0);
    }
    // Admission of a value is metered the same way and exposes no truncation.
    let mut denied = Meter::new(UNLIMITED).with_injected_denial(InjectedDenial {
        point: ChargePoint::TextNormalizeOutput,
        occurrence: 2,
    });
    assert!(matches!(
        admit_text(&runtime(E_COMBINING), &text_type(0, 4, Nfd), &mut denied),
        Outcome::Incomplete(_)
    ));
    // An independently metered sibling is unaffected.
    assert_eq!(run(&mut Meter::new(T11)), Outcome::Completed(true));
}

#[trace("TC-186", "FR-141-AC-4")]
#[test]
fn t12_lexicographic_order_is_selected_by_the_profile() {
    for profile in [UnicodeScalars, Nfc, Nfd, Nfkc, Nfkd] {
        assert!(compared(Less, &value("a", profile), &value("b", profile)));
    }
    let low = admit_text(
        &TextPayload::from_utf8(&[0x7f]).unwrap(),
        &text_type(1, 2, BinaryUtf8),
        &mut Meter::new(UNLIMITED),
    )
    .completed()
    .unwrap();
    let high = admit_text(
        &TextPayload::from_utf8(&[0xc2, 0x80]).unwrap(),
        &text_type(1, 2, BinaryUtf8),
        &mut Meter::new(UNLIMITED),
    )
    .completed()
    .unwrap();
    assert!(compared(Less, &low, &high));
    assert!(!compared(GreaterOrEqual, &low, &high));
    // U+212B ANGSTROM SIGN orders above U+00C5 as scalars but is equal after
    // normalization: the profile selects the ordering domain.
    let (angstrom, ring) = ("\u{212b}", "\u{c5}");
    assert!(!compared(
        Less,
        &value(angstrom, UnicodeScalars),
        &value(ring, UnicodeScalars)
    ));
    assert!(compared(Equal, &value(angstrom, Nfc), &value(ring, Nfc)));
    assert!(!compared(
        Equal,
        &value(angstrom, BinaryUtf8),
        &value(ring, BinaryUtf8)
    ));
}

// ---- enum fixtures ---------------------------------------------------------

fn vectors() -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(
        "resources/complete-value/quire-specification/proposals/checked-package-v2/node-identity-vectors.json",
    );
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}

fn schema() -> jsonschema::JSONSchema {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(
        "resources/complete-value/quire-specification/proposals/checked-package-v2/node-identity-preimage.schema.json",
    );
    let schema: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    jsonschema::JSONSchema::options()
        .with_draft(jsonschema::Draft::Draft202012)
        .compile(&schema)
        .unwrap()
}

fn vector(vectors: &Value, name: &str) -> (Value, String) {
    let entry = vectors["vectors"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["name"] == name)
        .unwrap();
    (
        entry["preimage"].clone(),
        entry["sha256"].as_str().unwrap().to_owned(),
    )
}

/// Independent RFC 8785 JCS for the string/boolean/array/object/null values
/// the preimage schema admits.
fn jcs(value: &Value) -> String {
    match value {
        Value::Null | Value::Bool(_) | Value::String(_) => value.to_string(),
        Value::Number(_) => panic!("preimages carry no JSON numbers"),
        Value::Array(items) => format!("[{}]", items.iter().map(jcs).collect::<Vec<_>>().join(",")),
        Value::Object(members) => {
            let mut entries: Vec<_> = members.iter().collect();
            entries.sort_by(|(a, _), (b, _)| a.encode_utf16().cmp(b.encode_utf16()));
            let body: Vec<_> = entries
                .into_iter()
                .map(|(key, item)| format!("{}:{}", Value::String(key.clone()), jcs(item)))
                .collect();
            format!("{{{}}}", body.join(","))
        }
    }
}

fn fixture_key(preimage: &Value) -> NodeKey {
    let digest: String = Sha256::digest(jcs(preimage).as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    NodeKey::from_hex(&digest).unwrap()
}

fn example_owner() -> NodeOwner {
    NodeOwner::Definition(OwnerSubject {
        authority: "agent-ix".into(),
        identity: "example-model".into(),
    })
}

fn owners() -> OwnerSelection {
    OwnerSelection::new([example_owner()])
}

fn declaration(preimage: &Value, key: NodeKey) -> Result<EnumDeclaration, InvalidSemanticGraph> {
    EnumDeclaration::admit(
        EnumDeclarationPreimage::from_json(preimage.clone())?,
        key,
        &owners(),
    )
}

fn member_preimage(declaration: NodeKey, case: &str) -> Value {
    json!({
        "version": "quire.enum-member-node/v1",
        "declaration_node_id": {"domain": NODE_KEY_DOMAIN, "digest": declaration.to_string()},
        "case": case,
    })
}

fn member(declaration: &EnumDeclaration, case: &str) -> EnumValue {
    let preimage = member_preimage(declaration.key(), case);
    declaration
        .admit_member(
            &EnumMemberPreimage::from_json(preimage.clone()).unwrap(),
            fixture_key(&preimage),
        )
        .unwrap()
}

fn declaration_preimage(qualified: [&str; 2], ordered: bool, members: &[&str]) -> Value {
    json!({
        "version": "quire.enum-declaration-node/v1",
        "owner": {"kind": "definition", "authority": "agent-ix", "identity": "example-model"},
        "qualified_declaration": qualified,
        "ordered": ordered,
        "members": members,
    })
}

fn fixture_declaration(qualified: [&str; 2], ordered: bool, members: &[&str]) -> EnumDeclaration {
    let preimage = declaration_preimage(qualified, ordered, members);
    declaration(&preimage, fixture_key(&preimage)).unwrap()
}

fn enum_compared(
    operator: ComparisonOperator,
    left: &EnumValue,
    right: &EnumValue,
) -> Result<bool, IllTyped> {
    compare_enum(operator, left, right, &mut Meter::new(UNLIMITED))
        .map(|outcome| outcome.completed().unwrap())
}

const ENUM_DECLARATION_VECTORS: [&str; 2] = ["enum-status", "enum-color-unordered"];
const ENUM_MEMBER_VECTORS: [&str; 1] = ["enum-status-ready"];
/// Dimension and unit identities are consumed by `tests/quantities.rs` (TC-187).
const TC_187_VECTORS: [&str; 11] = [
    "dimension-length",
    "unit-metre",
    "dimension-time",
    "dimension-temperature",
    "dimension-velocity",
    "unit-second",
    "unit-kelvin",
    "unit-degree-celsius",
    "unit-centimetre",
    "unit-millimetre",
    "unit-huge-exact-scale",
];

#[trace("TC-186", "FR-141-AC-3")]
#[test]
fn enum_node_identity_vectors_reproduce_and_noncanonical_preimages_refuse() {
    let vectors = vectors();
    assert_eq!(vectors["version"], "quire.checked-semantic-node-vectors/v1");
    let names: Vec<&str> = vectors["vectors"]
        .as_array()
        .unwrap()
        .iter()
        .map(|entry| entry["name"].as_str().unwrap())
        .collect();
    assert_eq!(names.len(), 14);
    for name in &names {
        assert!(
            ENUM_DECLARATION_VECTORS.contains(name)
                || ENUM_MEMBER_VECTORS.contains(name)
                || TC_187_VECTORS.contains(name),
            "{name} is unclassified"
        );
    }
    // The independent canonicalizer reproduces every pinned digest.
    for entry in vectors["vectors"].as_array().unwrap() {
        assert_eq!(
            fixture_key(&entry["preimage"]).to_string(),
            entry["sha256"].as_str().unwrap(),
            "{}",
            entry["name"]
        );
    }

    let schema = schema();
    for name in ENUM_DECLARATION_VECTORS {
        let (preimage, digest) = vector(&vectors, name);
        assert!(schema.is_valid(&preimage));
        let key = NodeKey::from_hex(&digest).unwrap();
        let parsed = EnumDeclarationPreimage::from_json(preimage.clone()).unwrap();
        assert_eq!(parsed.node_key().unwrap(), key, "{name}");
        assert_eq!(declaration(&preimage, key).unwrap().key(), key);
    }
    let (status, status_digest) = vector(&vectors, "enum-status");
    let status = declaration(&status, NodeKey::from_hex(&status_digest).unwrap()).unwrap();
    let (ready, ready_digest) = vector(&vectors, "enum-status-ready");
    assert!(schema.is_valid(&ready));
    let ready_key = NodeKey::from_hex(&ready_digest).unwrap();
    let ready_preimage = EnumMemberPreimage::from_json(ready).unwrap();
    assert_eq!(ready_preimage.node_key().unwrap(), ready_key);
    assert_eq!(ready_preimage.declaration(), status.key());
    let ready = status.admit_member(&ready_preimage, ready_key).unwrap();
    assert_eq!(
        (ready.declaration(), ready.member()),
        (status.key(), ready_key)
    );

    // Every non-canonical preimage is also rejected by the pinned schema.
    let base = declaration_preimage(["Example", "Status"], true, &["READY", "DONE"]);
    let mutated = |edit: &dyn Fn(&mut Value)| {
        let mut value = base.clone();
        edit(&mut value);
        value
    };
    let noncanonical = [
        mutated(&|v| v["extra"] = json!(1)),
        mutated(&|v| {
            v.as_object_mut().unwrap().remove("ordered");
        }),
        mutated(&|v| v["version"] = json!("quire.enum-declaration-node/v2")),
        mutated(&|v| v["members"] = json!([])),
        mutated(&|v| v["members"] = json!(["READY", "READY"])),
        mutated(&|v| v["members"] = json!(["1READY"])),
        mutated(&|v| v["qualified_declaration"] = json!([])),
        mutated(&|v| v["owner"]["kind"] = json!("package")),
        mutated(&|v| v["owner"]["authority"] = json!("")),
        mutated(&|v| v["owner"]["extra"] = json!("x")),
        mutated(&|v| v["owner"]["export"] = json!("E")),
        mutated(&|v| v["owner"] = json!({"kind": "model", "identity": "b"})),
        mutated(&|v| v["owner"] = json!({"kind": "model", "identity": "b", "node": null})),
        // A model owner naming `authority` and `export` instead of `node` is
        // refused: `node` is required and absent, and `authority`/`export`
        // are unknown members.
        mutated(&|v| {
            v["owner"] = json!({"kind": "model", "authority": "a", "identity": "b", "export": "E"})
        }),
        mutated(&|v| v["ordered"] = json!("true")),
    ];
    for preimage in noncanonical {
        assert!(!schema.is_valid(&preimage), "{preimage}");
        assert_eq!(
            EnumDeclarationPreimage::from_json(preimage.clone()),
            Err(InvalidSemanticGraph {
                cause: SemanticGraphCause::NonCanonicalPreimage
            }),
            "{preimage}"
        );
    }
    let model = mutated(&|v| v["owner"] = json!({"kind": "model", "identity": "b", "node": "E"}));
    assert!(schema.is_valid(&model));
    assert!(EnumDeclarationPreimage::from_json(model).is_ok());

    let member = member_preimage(status.key(), "READY");
    let member_noncanonical = [
        {
            let mut v = member.clone();
            v["declaration_node_id"]["digest"] = json!(status.key().to_string().to_uppercase());
            v
        },
        {
            let mut v = member.clone();
            v["declaration_node_id"]["domain"] = json!("quire.definition.bytes/v1");
            v
        },
        {
            let mut v = member.clone();
            v["case"] = json!("not-an-identifier");
            v
        },
        {
            let mut v = member.clone();
            v["version"] = json!("quire.enum-declaration-node/v1");
            v
        },
    ];
    for preimage in member_noncanonical {
        assert!(!schema.is_valid(&preimage), "{preimage}");
        assert_eq!(
            EnumMemberPreimage::from_json(preimage),
            Err(InvalidSemanticGraph {
                cause: SemanticGraphCause::NonCanonicalPreimage
            })
        );
    }
    assert_eq!(InvalidSemanticGraph::CODE, "invalid_semantic_graph");

    // RFC 8785 string serialization: C0 controls as lowercase `\u00xx`, short
    // escapes for quote, backslash and `\n`, and no escape for other scalars.
    let escaped = EnumDeclarationPreimage::from_json(json!({
        "version": "quire.enum-declaration-node/v1",
        "owner": {"kind": "source", "authority": "a\u{1f}\"\\\n\u{e9}\u{2028}\u{7f}", "identity": "i"},
        "qualified_declaration": ["Q"],
        "ordered": true,
        "members": ["A"],
    }))
    .unwrap();
    let expected = concat!(
        r#"{"members":["A"],"ordered":true,"owner":{"authority":"a\u001f\"\\\n"#,
        "\u{e9}\u{2028}\u{7f}",
        r#"","identity":"i","kind":"source"},"qualified_declaration":["Q"],"version":"quire.enum-declaration-node/v1"}"#,
    );
    let digest: String = Sha256::digest(expected.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    assert_eq!(escaped.node_key().unwrap().to_string(), digest);
}

#[trace("TC-186", "FR-141-AC-2")]
#[test]
fn t07_equal_spellings_under_distinct_declarations_are_ill_typed() {
    let enum_a = fixture_declaration(["Example", "Status"], true, &["READY", "DONE"]);
    let enum_b = fixture_declaration(["Example", "Phase"], true, &["READY", "DONE"]);
    assert_ne!(enum_a.key(), enum_b.key());
    let (ready_a, ready_b) = (member(&enum_a, "READY"), member(&enum_b, "READY"));
    assert_ne!(ready_a.member(), ready_b.member());
    for operator in ComparisonOperator::ALL {
        let mut meter = Meter::new(UNLIMITED);
        assert!(matches!(
            compare_enum(operator, &ready_a, &ready_b, &mut meter),
            Err(IllTyped {
                cause: IllTypedCause::DistinctEnumDeclarations
            })
        ));
        assert!(meter.admitted_charges().is_empty());
    }
    assert_eq!(
        enum_compared(Equal, &ready_a, &member(&enum_a, "READY")),
        Ok(true)
    );
}

#[trace("TC-186", "FR-141-AC-3", "FR-141-AC-5")]
#[test]
fn t08_ordering_follows_ordered_declarations_only() {
    let ordered = fixture_declaration(["Example", "Status"], true, &["READY", "DONE"]);
    let (ready, done) = (member(&ordered, "READY"), member(&ordered, "DONE"));
    assert_eq!(enum_compared(Less, &ready, &done), Ok(true));
    assert_eq!(enum_compared(GreaterOrEqual, &ready, &done), Ok(false));
    assert_eq!(enum_compared(Equal, &ready, &done), Ok(false));

    // Declaration order, not spelling order: DONE < READY alphabetically.
    assert!("DONE" < "READY");
    let unordered = fixture_declaration(["Example", "Status"], false, &["DONE", "READY"]);
    let (ready, done) = (member(&unordered, "READY"), member(&unordered, "DONE"));
    for operator in ComparisonOperator::ALL {
        let result = enum_compared(operator, &ready, &done);
        if operator.is_ordering() {
            assert_eq!(
                result,
                Err(IllTyped {
                    cause: IllTypedCause::UnorderedEnumOrdering
                })
            );
        } else {
            assert_eq!(result, Ok(operator == NotEqual));
        }
    }
}

fn apply_patch(mut target: Value, patch: &Value) -> Value {
    for operation in patch.as_array().unwrap() {
        let path = operation["path"].as_str().unwrap();
        match operation["op"].as_str().unwrap() {
            "replace" => *target.pointer_mut(path).unwrap() = operation["value"].clone(),
            "move" => {
                let from = operation["from"].as_str().unwrap();
                let (from_parent, from_index) = from.rsplit_once('/').unwrap();
                let (to_parent, to_index) = path.rsplit_once('/').unwrap();
                assert_eq!(from_parent, to_parent, "array move within one parent");
                let items = target
                    .pointer_mut(from_parent)
                    .unwrap()
                    .as_array_mut()
                    .unwrap();
                let moved = items.remove(from_index.parse().unwrap());
                items.insert(to_index.parse().unwrap(), moved);
            }
            other => panic!("unsupported patch op {other}"),
        }
    }
    target
}

#[trace("TC-186", "FR-141-AC-3")]
#[test]
fn t09_stale_enum_keys_refuse_and_recomputed_keys_are_new_identities() {
    let vectors = vectors();
    let enum_mutations: Vec<&Value> = vectors["invalid_mutations"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|mutation| {
            ENUM_DECLARATION_VECTORS.contains(&mutation["base"].as_str().unwrap())
                || ENUM_MEMBER_VECTORS.contains(&mutation["base"].as_str().unwrap())
        })
        .collect();
    let names: Vec<_> = enum_mutations
        .iter()
        .map(|mutation| mutation["name"].as_str().unwrap())
        .collect();
    assert_eq!(
        names,
        [
            "enum-member-stale-case",
            "enum-owner-absent-from-lock",
            "unordered-enum-member-order"
        ]
    );
    let (status, status_digest) = vector(&vectors, "enum-status");
    let status = declaration(&status, NodeKey::from_hex(&status_digest).unwrap()).unwrap();
    for mutation in enum_mutations {
        assert_eq!(mutation["expected_code"], InvalidSemanticGraph::CODE);
        let base = mutation["base"].as_str().unwrap();
        let (preimage, digest) = vector(&vectors, base);
        assert_eq!(mutation["retained_sha256"].as_str().unwrap(), digest);
        let patched = apply_patch(preimage, &mutation["patch"]);
        let retained = NodeKey::from_hex(&digest).unwrap();
        let refusal = if ENUM_MEMBER_VECTORS.contains(&base) {
            status
                .admit_member(&EnumMemberPreimage::from_json(patched).unwrap(), retained)
                .unwrap_err()
        } else {
            declaration(&patched, retained).unwrap_err()
        };
        let expected = match mutation["name"].as_str().unwrap() {
            "enum-member-stale-case" => SemanticGraphCause::StaleKey,
            "enum-owner-absent-from-lock" => SemanticGraphCause::OwnerNotSelected,
            _ => SemanticGraphCause::UnsortedUnorderedMembers,
        };
        assert_eq!(refusal.cause, expected, "{}", mutation["name"]);
    }

    // Owner, case and order changes that keep old keys are stale; recomputed
    // keys are a new identity that is ill-typed against the old one.
    let base = declaration_preimage(["Example", "Status"], true, &["READY", "DONE"]);
    let old_key = fixture_key(&base);
    let old = declaration(&base, old_key).unwrap();
    let other_owner = json!({"kind": "definition", "authority": "agent-ix", "identity": "other"});
    let changes = [
        declaration_preimage(["Example", "Status"], true, &["DONE", "READY"]),
        declaration_preimage(["Example", "Status"], true, &["READY", "CLOSED"]),
        declaration_preimage(["Example", "Status"], false, &["DONE", "READY"]),
        {
            let mut value = base.clone();
            value["owner"] = other_owner.clone();
            value
        },
    ];
    let selection = OwnerSelection::new([
        example_owner(),
        NodeOwner::Definition(OwnerSubject {
            authority: "agent-ix".into(),
            identity: "other".into(),
        }),
    ]);
    for changed in changes {
        let preimage = EnumDeclarationPreimage::from_json(changed.clone()).unwrap();
        assert_eq!(
            EnumDeclaration::admit(preimage.clone(), old_key, &selection),
            Err(InvalidSemanticGraph {
                cause: SemanticGraphCause::StaleKey
            }),
            "{changed}"
        );
        let new_key = fixture_key(&changed);
        assert_ne!(new_key, old_key);
        let new = EnumDeclaration::admit(preimage, new_key, &selection).unwrap();
        let old_ready = member(&old, "READY");
        let new_case = new.preimage().members()[0].clone();
        let new_member = member(&new, &new_case);
        assert_eq!(
            enum_compared(Equal, &old_ready, &new_member),
            Err(IllTyped {
                cause: IllTypedCause::DistinctEnumDeclarations
            })
        );
    }

    // A retained member key moved to another declaration or case refuses.
    let ready_preimage = member_preimage(old_key, "READY");
    let ready_key = fixture_key(&ready_preimage);
    let other = fixture_declaration(["Example", "Phase"], true, &["READY", "DONE"]);
    let ready_preimage = EnumMemberPreimage::from_json(ready_preimage).unwrap();
    assert_eq!(
        other
            .admit_member(&ready_preimage, ready_key)
            .unwrap_err()
            .cause,
        SemanticGraphCause::ForeignDeclaration
    );
    let closed = EnumMemberPreimage::from_json(member_preimage(old_key, "CLOSED")).unwrap();
    assert_eq!(
        old.admit_member(&closed, ready_key).unwrap_err().cause,
        SemanticGraphCause::UndeclaredCase
    );
}

#[trace("TC-186", "FR-141-AC-6")]
#[test]
fn enum_accounting_exact_bounds_and_named_denials() {
    const LIMITS: ScalarLimits = ScalarLimits {
        integer_bits: 0,
        decimal_digits: 0,
        scale_expansion: 0,
        text_input_bytes: 0,
        text_scalars: 0,
        normalized_scalars: 0,
        unit_edges: 0,
        value_occurrences: 2,
        work_units: 3,
        result_units: 1,
    };
    let status = fixture_declaration(["Example", "Status"], true, &["READY", "DONE"]);
    let (ready, done) = (member(&status, "READY"), member(&status, "DONE"));
    let run = |meter: &mut Meter| compare_enum(Less, &ready, &done, meter).unwrap();

    let mut exact = Meter::new(LIMITS);
    assert_eq!(run(&mut exact), Outcome::Completed(true));
    assert_eq!(consumed(&exact), [0, 0, 0, 0, 0, 0, 0, 2, 3, 1]);
    assert_eq!(
        exact.admitted_charges(),
        [
            ChargePoint::EnumIdentityRead,
            ChargePoint::EnumIdentityRead,
            ChargePoint::EnumResultRetain,
        ]
    );
    assert_eq!(
        run(&mut Meter::new(ScalarLimits {
            work_units: 2,
            ..LIMITS
        })),
        Outcome::Incomplete(Incomplete {
            limit_kind: LimitKind::WorkUnits,
            limit: 2,
            consumed: 2,
            next_charge: Integer::from(1_i64),
            charge_point: ChargePoint::EnumResultRetain,
        })
    );
    assert_eq!(
        run(&mut Meter::new(ScalarLimits {
            value_occurrences: 1,
            ..LIMITS
        })),
        Outcome::Incomplete(Incomplete {
            limit_kind: LimitKind::ValueOccurrences,
            limit: 1,
            consumed: 1,
            next_charge: Integer::from(2_i64),
            charge_point: ChargePoint::EnumIdentityRead,
        })
    );
    for (point, occurrence) in [
        (ChargePoint::EnumIdentityRead, 1),
        (ChargePoint::EnumIdentityRead, 2),
        (ChargePoint::EnumResultRetain, 1),
    ] {
        let mut denied =
            Meter::new(LIMITS).with_injected_denial(InjectedDenial { point, occurrence });
        assert!(matches!(
            run(&mut denied),
            Outcome::Incomplete(Incomplete { charge_point, .. }) if charge_point == point
        ));
        assert_eq!(denied.consumed(LimitKind::ResultUnits), 0);
    }
}

// ---- generated vectors -----------------------------------------------------

const ALPHABET: [char; 12] = [
    'a', 'e', 'f', '\u{e9}', '\u{301}', '\u{323}', '\u{fb00}', '\u{212b}', '\u{c5}', '\u{1100}',
    '\u{1161}', '\u{ac00}',
];

fn sequences() -> Vec<String> {
    let mut out = vec![String::new()];
    let mut frontier = vec![String::new()];
    for _ in 0..3 {
        frontier = frontier
            .iter()
            .flat_map(|prefix| {
                ALPHABET.iter().map(move |scalar| {
                    let mut next = prefix.clone();
                    next.push(*scalar);
                    next
                })
            })
            .collect();
        out.extend(frontier.iter().cloned());
    }
    out
}

fn oracle(profile: TextProfile, text: &str) -> String {
    match profile {
        UnicodeScalars | BinaryUtf8 => text.to_owned(),
        Nfc => text.nfc().collect(),
        Nfd => text.nfd().collect(),
        Nfkc => text.nfkc().collect(),
        Nfkd => text.nfkd().collect(),
    }
}

fn oracle_length(profile: TextProfile, retained: &str) -> u64 {
    let count = match profile {
        BinaryUtf8 => retained.len(),
        _ => retained.chars().count(),
    };
    u64::try_from(count).unwrap()
}

#[trace("TC-186", "FR-141-AC-1", "FR-141-AC-3", "FR-141-AC-4", "FR-141-AC-6")]
#[test]
fn generated_text_profiles_match_the_unicode_17_oracle() {
    let all = sequences();
    assert_eq!(all.len(), 1 + 12 + 144 + 1728);
    for profile in TextProfile::ALL {
        let values: Vec<Text> = all
            .iter()
            .map(|text| {
                let expected = oracle(profile, text);
                let length = oracle_length(profile, &expected);
                let exact = admit(text, &text_type(length, length, profile))
                    .completed()
                    .unwrap();
                assert_eq!(exact.retained(), expected, "{profile:?} {text:?}");
                assert_eq!(exact.length(), length);
                assert!(matches!(
                    admit(text, &text_type(length + 1, length + 1, profile)),
                    Outcome::Refused(Refusal::TextLengthOutOfDomain)
                ));
                if length > 0 {
                    assert!(matches!(
                        admit(text, &text_type(0, length - 1, profile)),
                        Outcome::Refused(Refusal::TextLengthOutOfDomain)
                    ));
                }
                exact
            })
            .collect();
        for (index, left) in values.iter().enumerate() {
            let right = &values[(index * 7 + 13) % values.len()];
            let expected = match profile {
                BinaryUtf8 => left.retained().as_bytes().cmp(right.retained().as_bytes()),
                _ => left.retained().chars().cmp(right.retained().chars()),
            };
            for operator in ComparisonOperator::ALL {
                let holds = match operator {
                    Equal => expected == Ordering::Equal,
                    NotEqual => expected != Ordering::Equal,
                    Less => expected == Ordering::Less,
                    ComparisonOperator::LessOrEqual => expected != Ordering::Greater,
                    ComparisonOperator::Greater => expected == Ordering::Greater,
                    GreaterOrEqual => expected != Ordering::Less,
                };
                assert_eq!(compared(operator, left, right), holds);
            }
        }
    }

    // Deny every admitted charge of a sample of normalizing comparisons.
    for (left, right) in [("e\u{301}\u{323}", "\u{e9}\u{323}"), ("\u{fb00}a", "ffa")] {
        for profile in [Nfc, Nfkd] {
            let (left, right) = (value(left, profile), value(right, profile));
            let mut meter = Meter::new(UNLIMITED);
            let expected = compare_text(Equal, &left, &right, &mut meter).unwrap();
            assert!(matches!(expected, Outcome::Completed(_)));
            let mut seen: Vec<ChargePoint> = Vec::new();
            for point in meter.admitted_charges() {
                seen.push(*point);
                let occurrence =
                    u64::try_from(seen.iter().filter(|p| *p == point).count()).unwrap();
                let mut denied = Meter::new(UNLIMITED).with_injected_denial(InjectedDenial {
                    point: *point,
                    occurrence,
                });
                assert!(matches!(
                    compare_text(Equal, &left, &right, &mut denied).unwrap(),
                    Outcome::Incomplete(Incomplete { charge_point, .. }) if charge_point == *point
                ));
                assert_eq!(denied.consumed(LimitKind::ResultUnits), 0);
            }
        }
    }
}

#[trace("TC-186", "FR-141-AC-2", "FR-141-AC-3", "FR-141-AC-5", "FR-141-AC-6")]
#[test]
fn generated_enums_follow_declaration_identity_and_order() {
    const POOL: [&str; 4] = ["ALPHA", "BETA", "DELTA", "GAMMA"];
    let mut declarations = Vec::new();
    for count in 1..=POOL.len() {
        for rotation in 0..count {
            let mut members: Vec<&str> = POOL[..count].to_vec();
            members.rotate_left(rotation);
            for ordered in [true, false] {
                if !ordered {
                    members.sort_unstable();
                }
                let name = format!("E{count}{rotation}{ordered}");
                let preimage = declaration_preimage(["Example", &name], ordered, &members);
                let key = fixture_key(&preimage);
                let admitted = declaration(&preimage, key).unwrap();
                // Retaining a key while changing the flag or owner is stale.
                let mut flipped = preimage.clone();
                flipped["ordered"] = json!(!ordered);
                if !ordered || members.is_sorted() {
                    assert_eq!(
                        declaration(&flipped, key).unwrap_err().cause,
                        SemanticGraphCause::StaleKey
                    );
                }
                let mut foreign = preimage.clone();
                foreign["owner"]["identity"] = json!("absent-model");
                assert_eq!(
                    declaration(&foreign, key).unwrap_err().cause,
                    SemanticGraphCause::OwnerNotSelected
                );
                if !ordered && count > 1 {
                    let mut unsorted = preimage.clone();
                    unsorted["members"].as_array_mut().unwrap().reverse();
                    assert_eq!(
                        declaration(&unsorted, key).unwrap_err().cause,
                        SemanticGraphCause::UnsortedUnorderedMembers
                    );
                }
                declarations.push((admitted, members.clone(), ordered));
            }
        }
    }
    for (declaration, members, ordered) in &declarations {
        let values: Vec<EnumValue> = members
            .iter()
            .map(|case| member(declaration, case))
            .collect();
        for (i, left) in values.iter().enumerate() {
            for (j, right) in values.iter().enumerate() {
                for operator in ComparisonOperator::ALL {
                    let result = enum_compared(operator, left, right);
                    if operator.is_ordering() && !ordered {
                        assert_eq!(
                            result,
                            Err(IllTyped {
                                cause: IllTypedCause::UnorderedEnumOrdering
                            })
                        );
                        continue;
                    }
                    let holds = match operator {
                        Equal => i == j,
                        NotEqual => i != j,
                        Less => i < j,
                        ComparisonOperator::LessOrEqual => i <= j,
                        ComparisonOperator::Greater => i > j,
                        GreaterOrEqual => i >= j,
                    };
                    assert_eq!(result, Ok(holds));
                }
            }
        }
    }
    // Members of different declarations never compare, whatever the spelling.
    for (left, _, _) in &declarations {
        for (right, _, _) in &declarations {
            if left.key() != right.key() {
                assert_eq!(
                    enum_compared(Equal, &member(left, "ALPHA"), &member(right, "ALPHA")),
                    Err(IllTyped {
                        cause: IllTypedCause::DistinctEnumDeclarations
                    })
                );
            }
        }
    }
}

// ---- QSpec #68 amendment 2 (pending re-vendor) --------------------------------

#[trace("TC-186", "FR-141-AC-4")]
#[test]
fn t13_scalar_length_is_measured_after_normalization() {
    let one_scalar = admit(E_COMBINING, &text_type(1, 1, Nfc))
        .completed()
        .unwrap();
    assert_eq!(one_scalar.length(), 1);
    for profile in [Nfd, UnicodeScalars] {
        assert!(
            matches!(
                admit(E_COMBINING, &text_type(1, 1, profile)),
                Outcome::Refused(Refusal::TextLengthOutOfDomain)
            ),
            "{profile:?}"
        );
    }
    let decomposed = admit(E_ACUTE, &text_type(2, 2, Nfd)).completed().unwrap();
    assert_eq!(decomposed.length(), 2);
    assert_eq!(decomposed.retained(), E_COMBINING);
}

const T16: ScalarLimits = ScalarLimits {
    integer_bits: 0,
    decimal_digits: 0,
    scale_expansion: 0,
    text_input_bytes: 3,
    text_scalars: 2,
    normalized_scalars: 2,
    unit_edges: 0,
    value_occurrences: 1,
    work_units: 5,
    result_units: 1,
};

/// A T16 admission: text, profile, the refusing limits and their charges, then
/// the denying limits and their record.
type T16Case = (
    &'static str,
    TextProfile,
    ScalarLimits,
    &'static [ChargePoint],
    ScalarLimits,
    Incomplete,
);

#[trace("TC-186", "FR-141-AC-3", "FR-141-AC-6")]
#[test]
fn t16_length_refusal_follows_the_last_charge_that_measures_the_length() {
    use ChargePoint::{TextDecodeScalars, TextInputBytes, TextNormalizeInput, TextNormalizeOutput};
    let scalars_only = ScalarLimits {
        normalized_scalars: 0,
        work_units: 2,
        ..T16
    };
    let bytes_only = ScalarLimits {
        text_input_bytes: 2,
        text_scalars: 0,
        normalized_scalars: 0,
        work_units: 1,
        ..T16
    };
    let cases: [T16Case; 3] = [
        (
            E_COMBINING,
            Nfd,
            T16,
            &[
                TextInputBytes,
                TextDecodeScalars,
                TextNormalizeInput,
                TextNormalizeOutput,
                TextNormalizeOutput,
            ],
            ScalarLimits {
                normalized_scalars: 1,
                ..T16
            },
            Incomplete {
                limit_kind: LimitKind::NormalizedScalars,
                limit: 1,
                consumed: 1,
                next_charge: Integer::from(2_u64),
                charge_point: TextNormalizeOutput,
            },
        ),
        (
            E_COMBINING,
            UnicodeScalars,
            scalars_only,
            &[TextInputBytes, TextDecodeScalars],
            ScalarLimits {
                text_scalars: 1,
                ..scalars_only
            },
            Incomplete {
                limit_kind: LimitKind::TextScalars,
                limit: 1,
                consumed: 0,
                next_charge: Integer::from(2_u64),
                charge_point: TextDecodeScalars,
            },
        ),
        (
            E_ACUTE,
            BinaryUtf8,
            bytes_only,
            &[TextInputBytes],
            ScalarLimits {
                text_input_bytes: 1,
                ..bytes_only
            },
            Incomplete {
                limit_kind: LimitKind::TextInputBytes,
                limit: 1,
                consumed: 0,
                next_charge: Integer::from(2_u64),
                charge_point: TextInputBytes,
            },
        ),
    ];
    for (text, profile, refused, charges, denied, incomplete) in cases {
        let bound = text_type(1, 1, profile);
        let mut meter = Meter::new(refused);
        assert!(
            matches!(
                admit_text(&runtime(text), &bound, &mut meter),
                Outcome::Refused(Refusal::TextLengthOutOfDomain)
            ),
            "{profile:?}"
        );
        assert_eq!(meter.admitted_charges(), charges, "{profile:?}");
        assert!(
            matches!(
                admit_text(&runtime(text), &bound, &mut Meter::new(denied)),
                Outcome::Incomplete(record) if record == incomplete
            ),
            "{profile:?}"
        );
    }
}

const T14: ScalarLimits = ScalarLimits {
    integer_bits: 0,
    decimal_digits: 0,
    scale_expansion: 0,
    text_input_bytes: 5,
    text_scalars: 3,
    normalized_scalars: 0,
    unit_edges: 0,
    value_occurrences: 2,
    work_units: 3,
    result_units: 1,
};

#[trace("TC-186", "FR-141-AC-4", "FR-141-AC-6")]
#[test]
fn t14_non_normalizing_profiles_charge_no_normalization() {
    for profile in [UnicodeScalars, BinaryUtf8] {
        let (left, right) = (value(E_ACUTE, profile), value(E_COMBINING, profile));
        let mut meter = Meter::new(T14);
        assert_eq!(
            compare_text(Equal, &left, &right, &mut meter).unwrap(),
            Outcome::Completed(false),
            "{profile:?}"
        );
        assert_eq!(
            meter.admitted_charges(),
            [
                ChargePoint::TextInputBytes,
                ChargePoint::TextDecodeScalars,
                ChargePoint::TextResultRetain,
            ],
            "{profile:?}"
        );
        assert_eq!(meter.consumed(LimitKind::NormalizedScalars), 0);
        assert_eq!(
            compare_text(
                Equal,
                &left,
                &right,
                &mut Meter::new(ScalarLimits {
                    work_units: 2,
                    ..T14
                })
            )
            .unwrap(),
            Outcome::Incomplete(Incomplete {
                limit_kind: LimitKind::WorkUnits,
                limit: 2,
                consumed: 2,
                next_charge: Integer::from(1_i64),
                charge_point: ChargePoint::TextResultRetain,
            }),
            "{profile:?}"
        );
    }
}

const ZERO: ScalarLimits = ScalarLimits {
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

#[trace("TC-186", "FR-141-AC-3", "FR-141-AC-5")]
#[test]
fn t15_enum_type_refusals_precede_every_charge() {
    let unordered = fixture_declaration(["Example", "Status"], false, &["DONE", "READY"]);
    let (ready, done) = (member(&unordered, "READY"), member(&unordered, "DONE"));
    let enum_a = fixture_declaration(["Example", "Status"], true, &["READY", "DONE"]);
    let enum_b = fixture_declaration(["Example", "Phase"], true, &["READY", "DONE"]);
    let (ready_a, ready_b) = (member(&enum_a, "READY"), member(&enum_b, "READY"));
    for (operator, left, right, cause) in [
        (Less, &ready, &done, IllTypedCause::UnorderedEnumOrdering),
        (
            Equal,
            &ready_a,
            &ready_b,
            IllTypedCause::DistinctEnumDeclarations,
        ),
    ] {
        let mut meter = Meter::new(ZERO);
        assert_eq!(
            compare_enum(operator, left, right, &mut meter),
            Err(IllTyped { cause })
        );
        assert!(meter.admitted_charges().is_empty());
    }
}
