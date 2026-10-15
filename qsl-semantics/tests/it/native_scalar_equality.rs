// SPDX-License-Identifier: AGPL-3.0-or-later
//! Unit-language `=` and `!=` on native `String`, `UUID` and `Timestamp`:
//! type-name resolution (FR-091), typing (FR-056) and lowering (FR-093).
//!
//! Every assertion compares typed codes, causes, locations and operation
//! identities, never message text.

use ix_trace_rs::trace;
use qsl_forms::build_unit;
use qsl_foundation::{SourceIdentity, Span};
use qsl_semantics::check::{
    AssemblyCause, CheckCause, CheckRefusal, CheckedGraph, PackageDeclarations,
};
use quire_exact::IllTypedCause;
use quire_semantic_value::checking::CheckingLimits;
use quire_semantic_value::location::Origin;
use serde_json::Value;

use crate::model_operations::{admit_and_assemble_with_body, tc465_document};

const HEADER: &str = "language \"ix:native\" edition \"1-draft\";\n\
                      profile v = \"quire.value.complete/v1\";\n";

fn parse(
    declarations: &str,
) -> (
    String,
    qsl_forms::ParsedUnit,
    qsl_foundation::source::provenance::RawSourceRef,
) {
    let text = format!("{HEADER}{declarations}\n");
    let parsed = qsl_cst::parse(
        SourceIdentity::new("test", "native-eq", "fixture", "fixture:1"),
        "native-eq.native",
        text.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let unit = build_unit(&parsed).expect("S2 builds the unit");
    (text, unit, parsed.source().reference().clone())
}

fn assemble(declarations: &str) -> Result<PackageDeclarations, qsl_semantics::check::AssemblyRefusal> {
    let (_, unit, source) = parse(declarations);
    PackageDeclarations::assemble(source, unit, Vec::new(), Vec::new())
}

fn check(declarations: &str) -> Result<CheckedGraph, Vec<CheckRefusal>> {
    assemble(declarations)
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

const EVENT: &str = "record Event { id: UUID; moment: Timestamp; label: String; }\n";

/// The single refusal of `declarations`, located in the body of `function`
/// at its root expression (the comparison).
fn refused_at_comparison(declarations: &str, function: &str, cause: IllTypedCause) {
    let refusals = check(declarations).expect_err("the unit does not check");
    assert_eq!(refusals.len(), 1, "{declarations}: {refusals:?}");
    assert_eq!(refusals[0].cause, CheckCause::IllTyped(cause), "{declarations}");
    assert!(
        matches!(&refusals[0].location.origin,
            Origin::Body { function: found, .. } if found == function),
        "{declarations}: {:?}",
        refusals[0].location
    );
    assert!(refusals[0].location.path().is_empty(), "{declarations}");
}

/// TC-1028 step 1, FR-091-AC-40: `String`, `UUID` and `Timestamp` name the
/// native types as parameter types with no `using` selection or domain
/// package, and the six comparison functions check.
#[trace("FR-091-AC-40", "FR-056-AC-35", "TC-1028")]
#[test]
fn the_six_native_comparison_functions_check() {
    check(
        "function eqs using v(a: String, b: String): Boolean pure { a = b }\n\
         function nes using v(a: String, b: String): Boolean pure { a != b }\n\
         function equ using v(a: UUID, b: UUID): Boolean pure { a = b }\n\
         function neu using v(a: UUID, b: UUID): Boolean pure { a != b }\n\
         function eqt using v(a: Timestamp, b: Timestamp): Boolean pure { a = b }\n\
         function net using v(a: Timestamp, b: Timestamp): Boolean pure { a != b }",
    )
    .expect("every native comparison checks");
}

/// FR-091-AC-40: a native name resolves as the result and as an alias target,
/// not only as a parameter.
#[trace("FR-091-AC-40", "TC-1028")]
#[test]
fn native_names_resolve_as_results_and_alias_targets() {
    let declarations = assemble(
        "type Stamp = Timestamp;\n\
         function f using v(a: String, b: UUID, c: Timestamp): Timestamp pure { c }",
    )
    .expect("the assembler builds the declarations");
    assert_eq!(
        declarations.aliases,
        [("Stamp".to_owned(), quire_exact::ValueType::Timestamp)]
    );
    declarations
        .check(CheckingLimits::default())
        .expect("the unit checks");
}

/// TC-1028 step 3, FR-093-AC-28: each comparison lowers to exactly one
/// `quire.op.<family>.<eq|ne>` expression, with no law, mode, member or
/// leaves, and no `quire.op.text.*` node.
#[trace("FR-093-AC-28", "TC-1028")]
#[test]
fn each_native_comparison_lowers_to_its_native_equality_identity() {
    let graph = check(
        "function eqs using v(a: String, b: String): Boolean pure { a = b }\n\
         function nes using v(a: String, b: String): Boolean pure { a != b }\n\
         function equ using v(a: UUID, b: UUID): Boolean pure { a = b }\n\
         function neu using v(a: UUID, b: UUID): Boolean pure { a != b }\n\
         function eqt using v(a: Timestamp, b: Timestamp): Boolean pure { a = b }\n\
         function net using v(a: Timestamp, b: Timestamp): Boolean pure { a != b }",
    )
    .expect("every native comparison checks");
    let all = preimages(&graph);
    for identity in [
        "quire.op.string.eq",
        "quire.op.string.ne",
        "quire.op.uuid.eq",
        "quire.op.uuid.ne",
        "quire.op.timestamp.eq",
        "quire.op.timestamp.ne",
    ] {
        let found: Vec<&Value> = all
            .iter()
            .filter(|preimage| preimage["body"]["operation"]["identity"] == identity)
            .collect();
        assert_eq!(found.len(), 1, "{identity} is one node");
        let operation = &found[0]["body"]["operation"];
        assert_eq!(operation["laws"], serde_json::json!([]), "{identity} laws");
        assert_eq!(operation["leaves"], serde_json::json!([]), "{identity} leaves");
        assert_eq!(operation["member"], Value::Null, "{identity} member");
        assert_eq!(operation["mode"], Value::Null, "{identity} mode");
        assert_eq!(
            found[0]["body"]["arguments"].as_array().map(Vec::len),
            Some(2),
            "{identity} is binary"
        );
    }
    assert!(
        all.iter().all(|preimage| !preimage["body"]["operation"]["identity"]
            .as_str()
            .is_some_and(|identity| identity.starts_with("quire.op.text."))),
        "no text-profile equality is selected"
    );
}

/// TC-1028 R1 to R5, FR-056-AC-36, FR-093-AC-28: unlike operand types refuse
/// `ill_typed`/`type-mismatch` at the comparison.
#[trace("FR-056-AC-36", "FR-093-AC-28", "TC-1028")]
#[test]
fn unlike_operand_types_refuse_type_mismatch_at_the_comparison() {
    for (parameters, body) in [
        ("a: String, b: UUID", "a = b"),
        ("a: UUID, b: Timestamp", "a = b"),
        ("a: String, b: Timestamp", "a != b"),
        ("a: String, b: Text[0, 8; nfc]", "a = b"),
        ("a: Text[0, 8; nfc], b: String", "a = b"),
        ("a: Timestamp, b: Integer", "a = b"),
        ("a: UUID, b: Integer", "a != b"),
    ] {
        refused_at_comparison(
            &format!("function bad using v({parameters}): Boolean pure {{ {body} }}"),
            "bad",
            IllTypedCause::TypeMismatch,
        );
    }
}

/// TC-1028 R6, FR-056-AC-36: ordering over two operands of one native type
/// refuses `ill_typed`/`operator-ineligible`.
#[trace("FR-056-AC-36", "TC-1028")]
#[test]
fn ordering_over_native_scalars_refuses_operator_ineligible() {
    for native in ["String", "UUID", "Timestamp"] {
        for operator in ["<", "<=", ">", ">="] {
            refused_at_comparison(
                &format!(
                    "function bad using v(a: {native}, b: {native}): Boolean pure {{ a {operator} b }}"
                ),
                "bad",
                IllTypedCause::OperatorIneligible,
            );
        }
    }
}

/// TC-1028 R7: a `UUID` against a `Reference<T>` refuses
/// `ill_typed`/`type-mismatch`.
#[trace("FR-056-AC-36", "TC-1028")]
#[test]
fn a_uuid_against_a_reference_refuses_type_mismatch() {
    let declarations = admit_and_assemble_with_body(
        &tc465_document(),
        "function bad using v(a: UUID, b: Reference<Config::ConfigVersion>): Boolean pure { a = b }",
    )
    .expect("the unit assembles");
    let refusals = declarations
        .check(CheckingLimits::default())
        .expect_err("the unit does not check");
    assert_eq!(refusals.len(), 1, "{refusals:?}");
    assert_eq!(
        refusals[0].cause,
        CheckCause::IllTyped(IllTypedCause::TypeMismatch)
    );
    assert!(refusals[0].location.path().is_empty());
}

/// TC-1028 step 5 F1 to F7: a field read, a call result and a `let` binding
/// are comparison operands; a `Timestamp` field against a `UUID` field
/// refuses.
#[trace("FR-056-AC-35", "FR-056-AC-36", "TC-1028")]
#[test]
fn field_call_result_and_let_operands_compare_and_a_mixed_pair_refuses() {
    let declarations = format!(
        "{EVENT}\
         function ft using v(x: Event, y: Event): Boolean pure {{ x.moment = y.moment }}\n\
         function fu using v(x: Event, y: Event): Boolean pure {{ x.id != y.id }}\n\
         function fs using v(x: Event, y: Event): Boolean pure {{ x.label = y.label }}\n\
         function stamp using v(e: Event): Timestamp pure {{ e.moment }}\n\
         function fc using v(x: Event, y: Event): Boolean pure {{ stamp(x) = stamp(y) }}\n\
         function fl using v(x: Event, y: Event): Boolean pure {{ let p = x.id in let q = y.id in p = q }}"
    );
    check(&declarations).expect("F1 to F6 check");
    refused_at_comparison(
        &format!(
            "{EVENT}function fm using v(x: Event, y: Event): Boolean pure {{ x.moment = y.id }}"
        ),
        "fm",
        IllTypedCause::TypeMismatch,
    );
}

/// TC-1028 step 4, FR-091-AC-40: a unit that declares `record UUID` and uses
/// `UUID` as a type refuses `ambiguous_declaration`/`ambiguous-name`, naming
/// the use, the record's span and the native candidate with no span.
#[trace("FR-091-AC-40", "TC-1028")]
#[test]
fn a_declared_form_named_like_a_native_type_makes_the_use_ambiguous() {
    let declarations = "record UUID { x: Integer; }\n\
                        function f using v(a: UUID): Boolean pure { true }";
    let (text, _, _) = parse(declarations);
    let span_of = |needle: &str, nth: usize| {
        let start = text.match_indices(needle).nth(nth).expect("present").0;
        Span {
            start,
            end: start + needle.len(),
        }
    };
    let record = span_of("UUID", 0);
    let used = span_of("UUID", 1);
    let refusal = assemble(declarations).expect_err("the use is ambiguous");
    let [error] = refusal.errors.as_slice() else {
        panic!("one error: {refusal:?}");
    };
    assert_eq!(error.span, used);
    assert_eq!(
        error.cause,
        AssemblyCause::AmbiguousTypeName {
            name: "UUID".to_owned(),
            candidates: vec![record],
            native: true,
        }
    );
    let catalog = error.cause.catalog_code();
    assert_eq!(
        (catalog.code(), catalog.cause()),
        ("ambiguous_declaration", "ambiguous-name")
    );
}

/// FR-091: a declared form with a native name and no use of the name stays
/// admitted; only a use is ambiguous.
#[trace("FR-091-AC-40", "TC-1028")]
#[test]
fn a_declared_form_named_like_a_native_type_is_admitted_while_unused() {
    assemble("record UUID { x: Integer; }\nfunction f using v(a: Integer): Boolean pure { true }")
        .expect("no use, no ambiguity");
}

/// FR-091 "Unresolved type name": other unknown names still refuse as
/// unresolved; only the three native spellings are exempt.
#[trace("FR-091-AC-40", "TC-1028")]
#[test]
fn a_near_miss_of_a_native_name_stays_unresolved() {
    for name in ["Uuid", "string", "TimeStamp"] {
        let refusal = assemble(&format!(
            "function f using v(a: {name}): Boolean pure {{ true }}"
        ))
        .expect_err("not a native name");
        assert!(
            matches!(
                refusal.errors.as_slice(),
                [error] if error.cause == AssemblyCause::UnresolvedTypeName { name: name.to_owned() }
            ),
            "{name}: {refusal:?}"
        );
    }
}
