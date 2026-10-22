// SPDX-License-Identifier: AGPL-3.0-or-later
//! Genuine complete-V1 IEEE source routes, lock evidence and strict IR admission.

use ix_trace_rs::trace;
use qsl_replay::{
    compile_admitted, AdmittedCompilation, AdmittedRefusal, Code, DependencyInput, ReplayLimits,
    SourceIdentity, StageLimits,
};
use qsl_semantics::check::CheckCause;
use quire_contract_model::{
    CheckedPackageEvidence, CheckedPackageReadLimits, CheckedPackageV2, CheckedPackageV2ReadResult,
};
use serde_json::{json, Value};

const HEADER: &str =
    "language \"ix:native\" edition \"1-draft\";\nprofile v = \"quire.value.complete/v1\";\n";
const PROFILE: &str = "quire.value.ieee754-2019-default/v1";
const ADD64: &str = include_str!("../fixtures/ieee_float/add64.native");
const ADD32: &str = include_str!("../fixtures/ieee_float/add32.native");
const TO_R: &str = include_str!("../fixtures/ieee_float/to_rational.native");

fn compile(text: &str) -> Result<AdmittedCompilation, AdmittedRefusal> {
    compile_admitted(
        SourceIdentity::new("a", "ieee", "git", "1"),
        "ieee.native",
        text.as_bytes(),
        [],
        &DependencyInput::default(),
        StageLimits::default(),
        ReplayLimits::default(),
    )
}

fn wire(text: &str) -> Value {
    let compiled = compile(text).unwrap_or_else(|error| panic!("{text}\n{error:?}"));
    serde_json::from_slice(compiled.package().bytes()).expect("emitted JSON")
}

fn ieee_selection(wire: &Value) -> Vec<&Value> {
    wire["lock"]["definition_selections"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|definition| definition["identity"] == PROFILE)
        .collect()
}

fn operation<'a>(wire: &'a Value, identity: &str) -> &'a Value {
    wire["semantic_graph"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|node| node["body"]["operation"]["identity"] == identity)
        .unwrap_or_else(|| panic!("missing {identity} in {wire}"))
}

fn assert_ieee_law(wire: &Value, node: &Value) {
    let selected = ieee_selection(wire);
    assert_eq!(selected.len(), 1, "exactly one IEEE selection");
    assert_eq!(
        node["body"]["operation"]["laws"],
        json!([{"role": "ieee_profile", "definition": selected[0]}]),
    );
}

fn refusal(text: &str, code: Code, cause: &str) -> Box<qsl_replay::spine::CompileRefusal> {
    let Err(AdmittedRefusal::Package(refusal)) = compile(text) else {
        panic!("expected {code:?}/{cause} from {text}");
    };
    assert_eq!(refusal.code(), code, "{refusal:?}");
    assert_eq!(refusal.cause(), Some(cause), "{refusal:?}");
    refusal
}

#[trace("TC-1091", "FR-397-AC-1", "FR-397-AC-4")]
#[test]
fn both_float_widths_select_one_profile_and_emit_arithmetic_law_and_rounding() {
    for (source, width) in [(ADD32, "float32"), (ADD64, "float64")] {
        for (spelling, name) in [("+", "add"), ("-", "sub"), ("*", "mul"), ("/", "div")] {
            let wire = wire(&source.replace("a + b", &format!("a {spelling} b")));
            let operation = operation(&wire, &format!("quire.op.ieee.{width}.{name}"));
            assert_ieee_law(&wire, operation);
            assert_eq!(
                operation["body"]["operation"]["mode"],
                json!({"kind": "rounding", "value": "nearest-even"}),
            );
        }
    }
}

#[trace("TC-1091", "FR-397-AC-2")]
#[test]
fn declared_floats_select_the_profile_even_without_an_ieee_application() {
    for declaration in [
        "function keep using v(a: Float64[nearest-even]): Boolean pure { true }",
        "function keep using v(a: Float32[nearest-even]): Float32[nearest-even] pure { a }",
        "record Reading { sample: Float64[nearest-even]; }",
        "tuple Reading(Float32[nearest-even]);",
        "union Reading { Empty, Some(Float64[nearest-even]) }",
        "type Reading = Float64[nearest-even];",
        "record Reading { values: Sequence<Float64[nearest-even]>[0, 2]; }",
    ] {
        let wire = wire(&format!("{HEADER}{declaration}"));
        assert_eq!(ieee_selection(&wire).len(), 1, "{declaration}");
        assert_eq!(
            wire["lock"]["definition_selections"],
            wire["identity_preimage"]["definition_selections"],
        );
    }
    let wire = wire(&format!(
        "{HEADER}function plain using v(a: Int[0, 10], b: Int[0, 10]): Int[0, 20] pure {{ a + b }}"
    ));
    assert!(ieee_selection(&wire).is_empty());
    for node in wire["semantic_graph"]["nodes"].as_array().unwrap() {
        let laws = node["body"]["operation"]["laws"].as_array();
        assert!(!laws
            .into_iter()
            .flatten()
            .any(|law| law["role"] == "ieee_profile"));
    }
}

#[trace("TC-1091", "FR-397-AC-3", "FR-397-AC-4")]
#[test]
fn float_to_rational_is_an_ieee_operation_with_the_selected_law() {
    for source in [TO_R.to_owned(), TO_R.replace("Float64", "Float32")] {
        let wire = wire(&source);
        assert_ieee_law(&wire, operation(&wire, "quire.op.ieee.to_rational"));
    }
}

#[trace("TC-1091", "FR-397-AC-4")]
#[test]
fn strict_ir_reader_admits_the_genuine_float_source_packages() {
    for source in [ADD32, ADD64, TO_R] {
        let compiled = compile(source).expect("source compiles");
        let wire: Value = serde_json::from_slice(compiled.package().bytes()).unwrap();
        let mut evidence = CheckedPackageEvidence::new();
        for feature in wire["lock"]["required_features"].as_array().unwrap() {
            evidence.support_feature(feature.as_str().unwrap());
        }
        let read = CheckedPackageV2::read(
            compiled.package().bytes(),
            CheckedPackageReadLimits::bounded(),
            &evidence,
        );
        let CheckedPackageV2ReadResult::Admitted(package) = read else {
            panic!("strict IR admission failed: {read:?}");
        };
        assert_eq!(
            package.graph().nodes.len(),
            wire["semantic_graph"]["nodes"].as_array().unwrap().len()
        );
        assert_eq!(
            package.package_id().digest.as_ref(),
            compiled.package().package_id().hex()
        );
    }
}

#[trace("TC-1091", "FR-397-AC-5")]
#[test]
fn ieee_profile_is_not_header_selectable_with_or_without_float_functions() {
    for source in [
        ADD64.to_owned(),
        format!("{HEADER}function plain using v(): Boolean pure {{ true }}"),
    ] {
        let source = source.replace(
            "profile v",
            &format!("profile w = \"{PROFILE}\";\nprofile v"),
        );
        refusal(&source, Code::UnknownProfile, "wrong-selection-role");
    }
}

#[trace("TC-1091", "FR-397-AC-6")]
#[test]
fn float_comparisons_and_mixed_width_arithmetic_refuse_at_checking() {
    for width in ["Float32", "Float64"] {
        for operator in ["=", "!=", "<", "<=", ">", ">="] {
            let source = format!("{HEADER}function compare using v(a: {width}[nearest-even], b: {width}[nearest-even]): Boolean pure {{ a {operator} b }}");
            let refusal = refusal(&source, Code::IllTyped, "operator-ineligible");
            let qsl_replay::spine::CompileRefusal::Check {
                region: Some(region),
                ..
            } = *refusal
            else {
                panic!("comparison refusal has a source region");
            };
            assert!(region.start() < region.end());
        }
    }
    refusal(
        &ADD64.replace("b: Float64", "b: Float32"),
        Code::IllTyped,
        "type-mismatch",
    );
}

#[trace("TC-1091", "FR-397-AC-7")]
#[test]
fn reserved_ieee_intrinsic_calls_refuse_with_the_named_missing_identity() {
    for (intrinsic, arguments) in [
        ("sqrt", "a"),
        ("fma", "a, b, a"),
        ("numericEqual", "a, b"),
        ("totalOrder", "a, b"),
        ("bitIdentical", "a, b"),
    ] {
        let identity = format!("quire::value::ieee::{intrinsic}");
        let source = format!("{HEADER}function call using v(a: Float64[nearest-even], b: Float64[nearest-even]): Boolean pure {{ {identity}({arguments}) }}");
        let refusal = refusal(&source, Code::MissingDeclaration, "missing-name");
        let qsl_replay::spine::CompileRefusal::Check { refusals, .. } = *refusal else {
            panic!("the intrinsic refuses at checking");
        };
        assert!(matches!(&refusals[0].cause, CheckCause::MissingName(name) if name == &identity));
    }
}

#[trace("TC-1091", "FR-397-AC-8")]
#[test]
fn conversion_into_a_float_is_never_admitted() {
    for (source, target) in [
        ("Int[0, 10]", "Float64[nearest-even]"),
        ("Float32[nearest-even]", "Float64[nearest-even]"),
        ("Float64[nearest-even]", "Float32[nearest-even]"),
    ] {
        refusal(
            &format!("{HEADER}function convert_float using v(a: {source}): {target} pure {{ convert<{target}>(a) }}"),
            Code::IllTyped,
            "type-mismatch",
        );
    }
}

#[trace("TC-1091", "FR-397-AC-3")]
#[test]
fn rational_comparison_evaluates_float_conversion_as_a_separate_operation() {
    let source = format!("{HEADER}function compare using v(a: Float64[nearest-even], b: Rational[-100, 100; 1, 16]): Boolean pure {{ convert<Rational[-100, 100; 1, 16]>(a) = b }}");
    let wire = wire(&source);
    assert_ieee_law(&wire, operation(&wire, "quire.op.ieee.to_rational"));
}
