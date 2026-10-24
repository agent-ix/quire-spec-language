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
    CheckCause, CheckRefusal, CheckedGraph, I128Limit, IntegerOutsideI128, IntegerSite,
    PackageDeclarations,
};
use qsl_semantics::model::intake::SelectedModels;
use quire_exact::IllTypedCause;
use quire_semantic_value::checking::CheckingLimits;
use quire_semantic_value::location::Origin;
use serde_json::{json, Value};

const HEADER: &str = "language \"ix:native\" edition \"1-draft\";\n\
                      profile v = \"quire.value.complete/v1\";\n";

const U1: &str = "00112233-4455-6677-8899-aabbccddeeff";
const U2: &str = "00112233-4455-6677-8899-aabbccddeefe";
const MIN: &str = "170141183460469231731687303715884105728";
const MAX: &str = "170141183460469231731687303715884105727";

fn check(declarations: &str) -> Result<CheckedGraph, Vec<CheckRefusal>> {
    let text = format!("{HEADER}{declarations}\n");
    let parsed = qsl_cst::parse(
        SourceIdentity::new("test", "literals", "fixture", "fixture:1"),
        "literals.native",
        text.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let unit = build_unit(&parsed).expect("S2 builds the unit");
    PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        unit,
        SelectedModels::default(),
        Vec::new(),
    )
    .unwrap_or_else(|refusal| panic!("assembly succeeds: {refusal:?}"))
    .check(CheckingLimits::default())
}

fn preimages(graph: &CheckedGraph) -> Vec<Value> {
    graph
        .semantic_graph()
        .nodes()
        .map(|node| serde_json::from_slice(node.preimage()).expect("a JSON preimage"))
        .collect()
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

/// The literal nodes of `graph` with `value_kind`, as `(value, node)`.
fn literals<'a>(all: &'a [Value], kind: &str) -> Vec<&'a Value> {
    all.iter()
        .filter(|node| node["body"]["term"] == "literal" && node["body"]["value_kind"] == kind)
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
    let all = preimages(&graph);
    let values = |kind: &str| -> Vec<Value> {
        let mut found: Vec<Value> = literals(&all, kind)
            .into_iter()
            .map(|node| node["body"]["value"].clone())
            .collect();
        found.sort_by_key(ToString::to_string);
        found
    };
    assert_eq!(values("string"), [json!("abc"), json!("a\u{e9}")]);
    assert_eq!(values("uuid"), [json!(U1)]);
    assert_eq!(
        values("timestamp"),
        [
            json!(format!("-{MIN}")),
            json!("-5"),
            json!("0"),
            json!(MAX)
        ]
    );
    for kind in ["string", "uuid", "timestamp"] {
        for node in literals(&all, kind) {
            assert_eq!(node["node_tag"], "value", "{kind}");
            assert_eq!(node["semantic_form"], "literal", "{kind}");
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
    let all = preimages(&graph);
    for (identity, kind) in [
        ("quire.op.string.eq", "string"),
        ("quire.op.uuid.ne", "uuid"),
        ("quire.op.timestamp.eq", "timestamp"),
    ] {
        let found: Vec<&Value> = all
            .iter()
            .filter(|node| node["body"]["operation"]["identity"] == identity)
            .collect();
        assert_eq!(found.len(), 1, "{identity}");
        let operation = &found[0]["body"]["operation"];
        assert_eq!(operation["laws"], json!([]), "{identity}");
        assert_eq!(operation["leaves"], json!([]), "{identity}");
        let arguments = found[0]["body"]["arguments"].as_array().expect("arguments");
        assert_eq!(arguments.len(), 2, "{identity}");
        let literal = literals(&all, kind)[0];
        assert_eq!(
            arguments[1]["node_id"], literal["node_id"],
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
