// SPDX-License-Identifier: AGPL-3.0-or-later
//! `String`, `UUID` and `Timestamp` literals in the unit language: typing
//! (FR-091, FR-056) and lowering (FR-093).
//!
//! Every assertion compares typed causes, codes, locations and node fields,
//! never message text.

use ix_trace_rs::trace;
use qsl_forms::build_unit;
use qsl_foundation::SourceIdentity;
use qsl_semantics::check::{
    ApplicationTerm, BodyTerm, CheckCause, CheckRefusal, CheckedGraph, I128Limit,
    IntegerOutsideI128, IntegerSite, LawRole, LeafTerm, LiteralValue, MemberTerm, NodeTag,
    OperationMode, PackageDeclarations, SemanticNode,
};
use qsl_semantics::model::intake::SelectedModels;
use quire_exact::IllTypedCause;
use quire_semantic_value::checking::CheckingLimits;
use quire_semantic_value::location::ExpressionOwner as Origin;

const HEADER: &str = "language \"ix:native\" edition \"1-draft\";\n\
                      profile v = \"quire.value.complete/v1\";\n";

const U1: &str = "00112233-4455-6677-8899-aabbccddeeff";
const U2: &str = "00112233-4455-6677-8899-aabbccddeefe";
const MIN: &str = "170141183460469231731687303715884105728";
const MAX: &str = "170141183460469231731687303715884105727";

fn check(declarations: &str) -> Result<CheckedGraph, Vec<CheckRefusal>> {
    let text = format!("{HEADER}{declarations}\n");
    let parsed = qsl_cst::parse(
        SourceIdentity::new("test", "literals"),
        "literals.native",
        text.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let unit = build_unit(&parsed).expect("S2 builds the unit");
    PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        crate::TEST_PACKAGE_NAME.to_owned(),
        unit,
        SelectedModels::default(),
        Vec::new(),
    )
    .unwrap_or_else(|refusal| panic!("assembly succeeds: {refusal:?}"))
    .check(
        CheckingLimits::default(),
        quire_semantic_value::checking::CheckMode::Linked,
    )
}

fn nodes(graph: &CheckedGraph) -> Vec<&SemanticNode> {
    graph.semantic_graph().nodes().collect()
}

/// The kind name and printed value of a literal node, or `None` for any other node.
fn literal_of(node: &SemanticNode) -> Option<(&'static str, String)> {
    let BodyTerm::Leaf(LeafTerm::Literal { value, .. }) = node.body() else {
        return None;
    };
    match value {
        LiteralValue::String(text) => Some(("string", text.clone())),
        LiteralValue::Text(text) => Some(("text", text.clone())),
        LiteralValue::Uuid(uuid) => Some(("uuid", uuid.to_string())),
        LiteralValue::Timestamp(stamp) => Some(("timestamp", stamp.to_string())),
        _ => None,
    }
}

/// The one refusal of `declarations`.
fn refused(declarations: &str) -> CheckRefusal {
    let mut refusals = check(declarations).expect_err("the unit does not check");
    assert_eq!(refusals.len(), 1, "{declarations}: {refusals:?}");
    refusals.remove(0)
}

fn mismatch() -> CheckCause {
    CheckCause::IllTyped(IllTypedCause::TypeMismatch)
}

/// The literal nodes of `all` with `kind`, as `(node, printed value)`.
fn literals<'a>(all: &[&'a SemanticNode], kind: &str) -> Vec<(&'a SemanticNode, String)> {
    all.iter()
        .filter_map(|node| {
            let (found, value) = literal_of(node)?;
            (found == kind).then_some((*node, value))
        })
        .collect()
}

/// FR-091-AC-41, FR-093-AC-29, TC-1052 steps 1 and 5: each literal lowers to
/// one `value`/`literal` node of the stated kind and value; equal literals
/// share a node and `Timestamp(-0)` is `Timestamp(0)`.
#[trace("FR-091-AC-41", "FR-093-AC-29", "TC-1052")]
#[test]
fn each_literal_lowers_to_one_literal_node_of_its_kind() {
    let graph = check(&format!(
        "function s1 using v(): String pure {{ \"a\\u00e9\" }}\n\
         function s2 using v(): String pure {{ \"abc\" }}\n\
         function s3 using v(): String pure {{ \"abc\" }}\n\
         function u1 using v(): UUID pure {{ UUID(\"{U1}\") }}\n\
         function t1 using v(): Timestamp pure {{ Timestamp(-5) }}\n\
         function t2 using v(): Timestamp pure {{ Timestamp(0) }}\n\
         function t3 using v(): Timestamp pure {{ Timestamp(-0) }}\n\
         function t4 using v(): Timestamp pure {{ Timestamp(-{MIN}) }}\n\
         function t5 using v(): Timestamp pure {{ Timestamp({MAX}) }}"
    ))
    .expect("every literal checks");
    let all = nodes(&graph);
    let values = |kind: &str| -> Vec<String> {
        let mut found: Vec<String> = literals(&all, kind)
            .into_iter()
            .map(|(_, value)| value)
            .collect();
        found.sort();
        found
    };
    assert_eq!(values("string"), ["abc", "a\u{e9}"]);
    assert_eq!(values("uuid"), [U1]);
    assert_eq!(
        values("timestamp"),
        [
            format!("-{MIN}"),
            "-5".to_owned(),
            "0".to_owned(),
            MAX.to_owned()
        ]
    );
    for kind in ["string", "uuid", "timestamp"] {
        for (node, _) in literals(&all, kind) {
            assert_eq!(node.node_tag(), NodeTag::Value, "{kind}");
            assert_eq!(node.semantic_form(), "literal", "{kind}");
        }
    }
}

/// FR-093-AC-29, TC-1052 step 5: a literal is the second argument of the
/// native equality node.
#[trace("FR-093-AC-29", "TC-1052")]
#[test]
fn a_literal_comparison_lowers_to_the_native_equality_with_the_literal_node() {
    let graph = check(&format!(
        "function e2 using v(a: String): Boolean pure {{ a = \"abc\" }}\n\
         function e3 using v(a: UUID): Boolean pure {{ a != UUID(\"{U1}\") }}\n\
         function e4 using v(a: Timestamp): Boolean pure {{ a = Timestamp(5) }}"
    ))
    .expect("the comparisons check");
    let all = nodes(&graph);
    for (identity, kind) in [
        ("quire.op.string.eq", "string"),
        ("quire.op.uuid.ne", "uuid"),
        ("quire.op.timestamp.eq", "timestamp"),
    ] {
        let found: Vec<&ApplicationTerm> = all
            .iter()
            .filter_map(|node| match node.body() {
                BodyTerm::Application(term) if term.operation.identity() == identity => Some(term),
                _ => None,
            })
            .collect();
        assert_eq!(found.len(), 1, "{identity}");
        let operation = &found[0].operation;
        assert!(operation.laws().is_empty(), "{identity}");
        assert!(operation.leaves().is_empty(), "{identity}");
        let arguments = &found[0].arguments;
        assert_eq!(arguments.len(), 2, "{identity}");
        let literal = literals(&all, kind)[0].0.key();
        let MemberTerm::Leaf(LeafTerm::Reference { target }) = &arguments[1] else {
            panic!("{identity}: the second argument is a reference");
        };
        assert_eq!(
            target.admitted(),
            Some(&literal),
            "{identity}: the literal's node is the second argument"
        );
    }
}

/// FR-091-AC-41, FR-056-AC-40, TC-1052 step 4: literals compare in call
/// arguments, `let` bindings and field reads.
#[trace("FR-056-AC-40", "TC-1052")]
#[test]
fn literals_compare_in_arguments_bindings_and_field_reads() {
    check(&format!(
        "record R {{ label: String; }}\n\
         function same using v(a: String, b: String): Boolean pure {{ a = b }}\n\
         function viaCall using v(): Boolean pure {{ same(\"a\", \"a\") }}\n\
         function viaLet using v(): Boolean pure {{ let x = \"a\" in x = \"a\" }}\n\
         function viaField using v(x: R): Boolean pure {{ x.label = \"a\" }}\n\
         function lits using v(): Boolean pure {{ \"\\u00e9\" = \"e\\u0301\" }}\n\
         function stamps using v(): Boolean pure {{ Timestamp(0) = Timestamp(-0) }}\n\
         function ids using v(u: UUID): Boolean pure {{ u != UUID(\"{U2}\") }}"
    ))
    .expect("every literal comparison checks");
}

/// FR-091-AC-43, TC-1052 step 3: a non-canonical UUID text refuses
/// `MalformedUuidLiteral`, naming the argument, at the call.
#[trace("FR-091-AC-43", "TC-1052")]
#[test]
fn a_non_canonical_uuid_literal_refuses_with_its_argument() {
    for text in [
        "00112233-4455-6677-8899-AABBCCDDEEFF",
        "00112233445566778899aabbccddeeff",
        "{00112233-4455-6677-8899-aabbccddeeff}",
        "urn:uuid:00112233-4455-6677-8899-aabbccddeeff",
        "00112233-4455-6677-8899-aabbccddeef",
        "00112233-4455-6677-8899-aabbccddeefg",
        "",
    ] {
        let refusal = refused(&format!(
            "function f using v(): UUID pure {{ UUID(\"{text}\") }}"
        ));
        assert_eq!(
            refusal.cause,
            CheckCause::MalformedUuidLiteral {
                argument: text.to_owned()
            },
            "{text:?}"
        );
        assert_eq!(
            (refusal.cause.code().as_str(), refusal.cause.cause()),
            ("ill_typed", Some("type-mismatch"))
        );
        assert!(
            matches!(&refusal.location.origin, Origin::Body { function, .. } if function == "f")
        );
        assert!(refusal.location.path().is_empty(), "at the call");
    }
}

/// FR-091-AC-43, TC-1052 step 3: a count outside `i128` is the argument
/// literal's `IntegerOutsideI128`; the negated one names the positive
/// literal and the `i128::MAX` limit; the in-range maximum checks.
#[trace("FR-091-AC-43", "TC-1052")]
#[test]
fn a_timestamp_outside_i128_refuses_as_the_integer_literal() {
    let too_high = "170141183460469231731687303715884105728";
    let refusal = refused(&format!(
        "function f using v(): Timestamp pure {{ Timestamp({too_high}) }}"
    ));
    assert_eq!(
        refusal.cause,
        CheckCause::IntegerOutsideI128(IntegerOutsideI128 {
            site: IntegerSite::Literal,
            value: too_high.to_owned(),
            limit: I128Limit::Max,
        })
    );
    assert_eq!(refusal.location.path(), [0]);
    let too_low = "170141183460469231731687303715884105729";
    let refusal = refused(&format!(
        "function f using v(): Timestamp pure {{ Timestamp(-{too_low}) }}"
    ));
    assert_eq!(
        refusal.cause,
        CheckCause::IntegerOutsideI128(IntegerOutsideI128 {
            site: IntegerSite::Literal,
            value: too_low.to_owned(),
            limit: I128Limit::Max,
        })
    );
    assert_eq!(refusal.location.path(), [0, 0]);
    check(&format!(
        "function f using v(): Timestamp pure {{ Timestamp({MAX}) }}"
    ))
    .expect("the maximum checks");
}

/// FR-091-AC-42, TC-1052 step 2: an argument that is not the literal form,
/// or a wrong argument count, refuses `ill_typed`/`type-mismatch`, at the
/// argument or, for the count, at the call.
#[trace("FR-091-AC-42", "TC-1052")]
#[test]
fn a_call_that_is_not_the_literal_form_refuses_type_mismatch() {
    for (parameters, body, result, path) in [
        ("x: String", "UUID(x)", "UUID", vec![0]),
        ("", "UUID(5)", "UUID", vec![0]),
        ("x: Integer", "Timestamp(x)", "Timestamp", vec![0]),
        ("", "Timestamp(\"5\")", "Timestamp", vec![0]),
        ("", "Timestamp(1 + 2)", "Timestamp", vec![0]),
        ("", "Timestamp()", "Timestamp", vec![]),
        ("", "Timestamp(1, 2)", "Timestamp", vec![]),
        ("", "UUID()", "UUID", vec![]),
    ] {
        let refusal = refused(&format!(
            "function f using v({parameters}): {result} pure {{ {body} }}"
        ));
        assert_eq!(refusal.cause, mismatch(), "{body}");
        assert_eq!(refusal.location.path(), path, "{body}");
    }
}

/// FR-091-AC-42, TC-1052 step 2: a unit that declares `function UUID` and
/// uses `UUID("...")` refuses `ambiguous_declaration`/`ambiguous-name`,
/// naming the use and the function; a parameter named `timestamp` or `uuid`
/// stays valid.
#[trace("FR-091-AC-42", "TC-1052")]
#[test]
fn a_function_named_like_a_native_literal_makes_the_call_ambiguous() {
    let refusal = refused(&format!(
        "function UUID using v(): Boolean pure {{ true }}\n\
         function f using v(): UUID pure {{ UUID(\"{U1}\") }}"
    ));
    let CheckCause::AmbiguousName { name, loci } = &refusal.cause else {
        panic!("ambiguous name: {:?}", refusal.cause);
    };
    assert_eq!(name, "UUID");
    assert_eq!(loci.len(), 2, "the use and the declaration");
    assert!(matches!(&loci[0].origin, Origin::Body { function, .. } if function == "f"));
    assert!(matches!(&loci[1].origin, Origin::Body { function, .. } if function == "UUID"));
    assert_eq!(
        (refusal.cause.code().as_str(), refusal.cause.cause()),
        ("ambiguous_declaration", Some("ambiguous-name"))
    );
    check(
        "function f using v(timestamp: Integer, uuid: Integer): Boolean pure { timestamp = uuid }",
    )
    .expect("the lowercase words are ordinary names");
}

/// FR-056-AC-41, TC-1052 step 4: a literal never changes type to fit its
/// slot, and the native types have no ordering.
#[trace("FR-056-AC-41", "TC-1052")]
#[test]
fn a_literal_never_adapts_to_a_slot_of_another_type() {
    for (parameters, result, body) in [
        ("x: Text[0, 8; nfc]", "Boolean", "x = \"abc\"".to_owned()),
        ("u: UUID", "Boolean", format!("u = \"{U1}\"")),
        ("x: String", "Boolean", format!("x = UUID(\"{U1}\")")),
        ("t: Timestamp", "Boolean", "t = 5".to_owned()),
        ("n: Integer", "Boolean", "n = Timestamp(5)".to_owned()),
        ("", "Timestamp", "5".to_owned()),
        ("", "String", format!("UUID(\"{U1}\")")),
        ("", "Text[0, 8; nfc]", "\"abc\"".to_owned()),
    ] {
        let refusal = refused(&format!(
            "function f using v({parameters}): {result} pure {{ {body} }}"
        ));
        assert_eq!(refusal.cause, mismatch(), "{body}");
    }
    for body in [
        "\"abc\" < \"abd\"".to_owned(),
        format!("UUID(\"{U1}\") <= UUID(\"{U2}\")"),
        "Timestamp(1) > Timestamp(2)".to_owned(),
    ] {
        let refusal = refused(&format!("function f using v(): Boolean pure {{ {body} }}"));
        assert_eq!(
            refusal.cause,
            CheckCause::IllTyped(IllTypedCause::OperatorIneligible),
            "{body}"
        );
    }
}

/// FR-390-AC-24: native scalar members inside a record retain their exact
/// equality behavior, and native String/UUID/Timestamp values do not acquire
/// ordering operators through a protocol value profile.
#[trace("FR-390-AC-24")]
#[test]
fn native_scalars_reject_ordering() {
    for ty in ["String", "UUID", "Timestamp"] {
        for operator in ["<", "<=", ">", ">="] {
            let body = match ty {
                "String" => format!("\"a\" {operator} \"b\""),
                "UUID" => format!("UUID(\"{U1}\") {operator} UUID(\"{U2}\")"),
                "Timestamp" => format!("Timestamp(1) {operator} Timestamp(2)"),
                _ => unreachable!(),
            };
            let refusal = refused(&format!("function f using v(): Boolean pure {{ {body} }}"));
            assert_eq!(
                refusal.cause,
                CheckCause::IllTyped(IllTypedCause::OperatorIneligible),
                "{ty} rejects {operator}"
            );
        }
    }
}

/// QSL-1461, FR-056-AC-51 / FR-093-AC-50, TC-1052 steps 7–9: `Text("...")`
/// takes the `Text[..]` type from its position, retains the selected profile's
/// normalization, and can take that type from a typed equality peer.
#[trace("FR-056-AC-51", "FR-093-AC-50", "TC-1052")]
#[test]
fn text_literals_take_their_type_from_context_and_retain_under_its_profile() {
    let graph = check(
        "function a1 using v(): Text[0, 8; nfc] pure { Text(\"\\u00e9\") }\n\
         function a2 using v(): Text[0, 8; nfc] pure { Text(\"e\\u0301\") }\n\
         function a3 using v(): Text[0, 8; unicode-scalars] pure { Text(\"e\\u0301\") }\n\
         function eq using v(x: Text[0, 8; nfc]): Boolean pure { x = Text(\"e\\u0301\") }",
    )
    .expect("the explicit Text constructors have their positional types");
    let all = nodes(&graph);
    let mut values: Vec<String> = literals(&all, "text")
        .into_iter()
        .map(|(_, value)| value)
        .collect();
    values.sort();
    assert_eq!(values, ["e\u{301}", "\u{e9}"]);
    for (node, _) in literals(&all, "text") {
        assert_eq!(node.node_tag(), NodeTag::Value);
        assert_eq!(node.semantic_form(), "literal");
    }
}

/// QSL-1461, FR-091-AC-69: declaring `function Text` makes the explicit
/// constructor and declaration ambiguous, naming both the use and declaration.
#[trace("FR-091-AC-69", "TC-1052")]
#[test]
fn a_function_named_text_makes_the_constructor_call_ambiguous() {
    let refusal = refused(
        "function Text using v(): Boolean pure { true }\n\
         function f using v(): Text[0, 8; nfc] pure { Text(\"abc\") }",
    );
    let CheckCause::AmbiguousName { name, loci } = &refusal.cause else {
        panic!("ambiguous name: {:?}", refusal.cause);
    };
    assert_eq!(name, "Text");
    assert_eq!(loci.len(), 2, "the use and the declaration");
    assert_eq!(
        (refusal.cause.code().as_str(), refusal.cause.cause()),
        ("ambiguous_declaration", Some("ambiguous-name"))
    );
}

/// QSL-1461, FR-091-AC-69: only the unqualified spelling is the native
/// constructor. A qualified `Z::Text`, `Z::UUID` or `Z::Timestamp` is an
/// ordinary name and refuses as one, identically for all three.
#[trace("FR-091-AC-69", "TC-1052")]
#[test]
fn a_qualified_constructor_name_is_an_ordinary_name() {
    let mut causes = Vec::new();
    for (result, body) in [
        ("Text[0, 8; nfc]", "Z::Text(\"a\")".to_owned()),
        ("UUID", format!("Z::UUID(\"{U1}\")")),
        ("Timestamp", "Z::Timestamp(5)".to_owned()),
    ] {
        let refusal = refused(&format!("function f using v(): {result} pure {{ {body} }}"));
        assert!(
            matches!(
                refusal.cause,
                CheckCause::MissingQualifiedName(_) | CheckCause::MissingName(_)
            ),
            "{body}: {:?}",
            refusal.cause
        );
        causes.push(std::mem::discriminant(&refusal.cause));
    }
    assert!(causes.windows(2).all(|pair| pair[0] == pair[1]));
}

/// QSL-1461, FR-056-AC-51: under `binary-utf8` the length is the UTF-8 byte
/// count, so U+00E9 (two bytes) fills `Text[0, 2; binary-utf8]` and is
/// retained as written.
#[trace("FR-056-AC-51", "TC-1052")]
#[test]
fn a_two_byte_scalar_fits_a_two_byte_binary_text() {
    let graph = check("function f using v(): Text[0, 2; binary-utf8] pure { Text(\"\\u00e9\") }")
        .expect("two bytes fit the maximum of two");
    let all = nodes(&graph);
    let texts = literals(&all, "text");
    assert_eq!(texts.len(), 1);
    assert_eq!(texts[0].1, "\u{e9}");
}

/// QSL-1461, FR-093-AC-50, TC-1052 step 9: the literal's `semantic_type` is
/// its body `type`; the precomposed and decomposed spellings under `nfc`
/// lower to one node; `x = Text("é")` lowers to one text equality with the
/// `text_profile` law, the `nfc` mode and arguments `[ref(x), ref(literal)]`.
#[trace("FR-093-AC-50", "TC-1052")]
#[test]
fn text_literals_share_one_node_and_compare_by_the_profile_equality() {
    let graph = check(
        "function t1 using v(): Text[0, 8; nfc] pure { Text(\"e\\u0301\") }\n\
         function t2 using v(): Text[0, 8; nfc] pure { Text(\"\\u00e9\") }\n\
         function e3 using v(x: Text[0, 8; nfc]): Boolean pure { x = Text(\"\\u00e9\") }",
    )
    .expect("the unit checks");
    let all = nodes(&graph);
    let texts = literals(&all, "text");
    assert_eq!(texts.len(), 1, "both spellings lower to the same node");
    let (literal, value) = &texts[0];
    assert_eq!(value, "\u{e9}");
    let BodyTerm::Leaf(LeafTerm::Literal { ty, .. }) = literal.body() else {
        panic!("a literal body");
    };
    assert_eq!(
        ty.admitted(),
        literal.semantic_type().as_ref(),
        "semantic_type is the body type"
    );
    let equalities: Vec<&ApplicationTerm> = all
        .iter()
        .filter_map(|node| match node.body() {
            BodyTerm::Application(term) if term.operation.identity() == "quire.op.text.eq" => {
                Some(term)
            }
            _ => None,
        })
        .collect();
    assert_eq!(equalities.len(), 1);
    let operation = &equalities[0].operation;
    assert_eq!(operation.laws().len(), 1);
    assert_eq!(operation.laws()[0].role, LawRole::TextProfile);
    assert_eq!(
        operation.mode(),
        Some(&OperationMode::TextProfile(quire_exact::TextProfile::Nfc))
    );
    let arguments = &equalities[0].arguments;
    assert_eq!(arguments.len(), 2);
    let MemberTerm::Leaf(LeafTerm::Reference { target }) = &arguments[1] else {
        panic!("the second argument is a reference");
    };
    assert_eq!(target.admitted(), Some(&literal.key()));
    let MemberTerm::Leaf(LeafTerm::Reference { target: first }) = &arguments[0] else {
        panic!("the first argument is a reference");
    };
    assert_ne!(first.admitted(), Some(&literal.key()), "the parameter");
}

/// QSL-1461, FR-056-AC-52 / TC-1052: the retained sequence determines the
/// Text bound refusal and its typed cause carries the declared profile data.
#[trace("FR-056-AC-52", "TC-1052")]
#[test]
fn a_text_literal_outside_its_retained_bounds_refuses_at_the_call() {
    for (result, literal, min, max, profile, retained_length) in [
        (
            "Text[0, 2; nfc]",
            "abc",
            0,
            2,
            quire_exact::TextProfile::Nfc,
            3,
        ),
        (
            "Text[2, 2; nfc]",
            "e\\u0301",
            2,
            2,
            quire_exact::TextProfile::Nfc,
            1,
        ),
        (
            "Text[0, 1; binary-utf8]",
            "\\u00e9",
            0,
            1,
            quire_exact::TextProfile::BinaryUtf8,
            2,
        ),
        (
            "Text[1, 8; nfc]",
            "",
            1,
            8,
            quire_exact::TextProfile::Nfc,
            0,
        ),
    ] {
        let refusal = refused(&format!(
            "function f using v(): {result} pure {{ Text(\"{literal}\") }}"
        ));
        assert_eq!(
            refusal.cause,
            CheckCause::TextLiteralOutsideBounds {
                argument: literal
                    .replace("\\u0301", "\u{301}")
                    .replace("\\u00e9", "\u{e9}"),
                min,
                max,
                profile,
                retained_length,
            }
        );
        assert!(refusal.location.path().is_empty());
        assert!(matches!(
            refusal.location.origin,
            Origin::Body { ref function, .. } if function == "f"
        ));
    }
}

/// QSL-1461, FR-056-AC-52: a `Text` constructor has no standalone type and
/// does not convert arbitrary expressions or fit a non-Text peer.
#[trace("FR-056-AC-52", "TC-1052")]
#[test]
fn text_literals_require_a_text_position_and_one_quoted_argument() {
    for (parameters, result, body, path) in [
        ("", "String", "Text(\"a\")", vec![]),
        ("", "Integer", "Text(\"a\")", vec![]),
        ("", "UUID", "Text(\"a\")", vec![]),
        ("", "Boolean", "Text(\"a\") = Text(\"a\")", vec![0]),
        ("x: String", "Boolean", "x = Text(\"a\")", vec![1]),
        ("x: String", "Text[0, 8; nfc]", "Text(x)", vec![0]),
        ("", "Text[0, 8; nfc]", "Text(5)", vec![0]),
        ("", "Text[0, 8; nfc]", "Text()", vec![]),
        ("", "Text[0, 8; nfc]", "Text(\"a\", \"b\")", vec![]),
    ] {
        let refusal = refused(&format!(
            "function f using v({parameters}): {result} pure {{ {body} }}"
        ));
        assert_eq!(refusal.cause, mismatch(), "{body}");
        assert_eq!(refusal.location.path(), path, "{body}");
        assert!(matches!(
            refusal.location.origin,
            Origin::Body { ref function, .. } if function == "f"
        ));
    }
}

/// FR-040-AC-3 and QSpec FR-140-AC-7 / TC-185 D25: decimal arithmetic keeps
/// one exact named Decimal type across both operands and the result. A
/// different operand type requires an explicit conversion.
#[trace("FR-040-AC-3", "QSpec-FR-140-AC-7", "QSpec-TC-185")]
#[test]
fn decimal_arithmetic_rejects_mixed_types_unless_an_operand_is_converted() {
    let refusal = refused(
        "type P = Decimal[0, 1000; 2, 2; exact];\n\
         type R = Decimal[0, 1000; 1, 1; exact];\n\
         function mixed using v(p: P, r: R): P pure { p + r }",
    );
    assert_eq!(refusal.cause, mismatch());
    assert_eq!(
        (refusal.cause.code().as_str(), refusal.cause.cause()),
        ("ill_typed", Some("type-mismatch"))
    );

    let refusal = refused(
        "type P = Decimal[0, 1000; 2, 2; exact];\n\
         type R = Decimal[0, 1000; 1, 1; exact];\n\
         function wrongResult using v(p: P, q: P): R pure { p + q }",
    );
    assert_eq!(refusal.cause, mismatch());

    check(
        "type P = Decimal[0, 1000; 2, 2; exact];\n\
         type R = Decimal[0, 1000; 1, 1; exact];\n\
         function same using v(p: P, q: P): P pure { p + q }\n\
         function converted using v(p: P, r: R): P pure { p + convert<P>(r) }",
    )
    .expect("same-type operands and an explicit conversion admit");
}

/// FR-091-AC-70, TC-1052 step 10: two spellings of one text lower to one
/// literal node of each kind, so the spelling takes no part in node identity
/// or lowering; `Text` literals share a node only under one slot type.
#[trace("FR-091-AC-70", "TC-1052")]
#[test]
fn the_spelling_of_a_text_literal_takes_no_part_in_node_identity_or_lowering() {
    let graph = check(
        "function s1 using v(): String pure { \"\u{e9}\" }\n\
         function s2 using v(): String pure { \"\\u00e9\" }\n\
         function s3 using v(): String pure { \"\\u00E9\" }\n\
         function x1 using v(): Text[0, 8; nfc] pure { Text(\"\u{e9}\") }\n\
         function x2 using v(): Text[0, 8; nfc] pure { Text(\"\\u00e9\") }\n\
         function x3 using v(): Text[0, 8; nfc] pure { Text(\"\\u00E9\") }\n\
         function y1 using v(): Text[0, 8; binary-utf8] pure { Text(\"\u{e9}\") }\n\
         function y2 using v(): Text[0, 8; binary-utf8] pure { Text(\"\\u00e9\") }",
    )
    .expect("every literal checks");
    let all = nodes(&graph);
    assert_eq!(
        literals(&all, "string")
            .into_iter()
            .map(|(_, value)| value)
            .collect::<Vec<_>>(),
        ["\u{e9}"]
    );
    // One `Text` node per slot type: the `nfc` pair shares one and the
    // `binary-utf8` pair shares one.
    assert_eq!(literals(&all, "text").len(), 2);
}
