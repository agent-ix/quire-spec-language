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
        qsl_replay::CompileUnit {
            source: SourceIdentity::new("a", "ieee", "git", "1"),
            package_name: "fixture".to_owned(),
            path: "ieee.native",
            bytes: text.as_bytes(),
        },
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
    assert_eq!(refusal.cause(), Some(cause), "{text}: {refusal:?}");
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
        "type Reading = Float64[nearest-even]; function anchor using v(): Boolean pure { true }",
        "record Reading { samples: Sequence<Float64[nearest-even]>[0, 2]; }",
        "function keep using v(a: Option<Float64[nearest-even]>): Boolean pure { true }",
    ] {
        let wire = wire(&format!("{HEADER}{declaration}"));
        assert_eq!(ieee_selection(&wire).len(), 1, "{declaration}");
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
    for source in [ADD32, ADD64, TO_R].into_iter().map(str::to_owned).chain([
        ADD32.replace("a + b", "a * b"),
        ADD64.replace("a + b", "a * b"),
    ]) {
        let compiled = compile(&source).expect("source compiles");
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
    }
}

#[trace("TC-1091", "FR-397-AC-7", "FR-397-AC-12", "FR-397-AC-15")]
#[test]
fn strict_ir_reader_admits_fma_relations_and_float_orderings() {
    for (width, ty) in [
        ("float32", "Float32[nearest-even]"),
        ("float64", "Float64[nearest-even]"),
    ] {
        let fused = format!("{HEADER}function fused using v(a: {ty}, b: {ty}, c: {ty}): {ty} pure {{ quire::value::ieee::fma(a, b, c) }}");
        let sources = [
            fused,
            format!("{HEADER}function f using v(a: {ty}, b: {ty}): Boolean pure {{ quire::value::ieee::totalOrder(a, b) }}"),
            format!("{HEADER}function f using v(a: {ty}, b: {ty}): Boolean pure {{ quire::value::ieee::bitIdentical(a, b) }}"),
            format!("{HEADER}function f using v(a: {ty}, b: {ty}): Boolean pure {{ quire::value::ieee::numericEqual(a, b) }}"),
            format!("{HEADER}function f using v(a: {ty}, b: {ty}): Boolean pure {{ a < b }}"),
            format!("{HEADER}function f using v(a: {ty}, b: {ty}): Boolean pure {{ a >= b }}"),
            format!("{HEADER}function f using v(a: {ty}, b: {ty}, c: {ty}): Boolean pure {{ quire::value::ieee::fma(a, b, c) > a }}"),
        ];
        for source in sources {
            let compiled = compile(&source).expect("source compiles");
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
                panic!("strict IR admission failed for {source}: {read:?}");
            };
            assert_eq!(
                package.graph().nodes.len(),
                wire["semantic_graph"]["nodes"].as_array().unwrap().len(),
                "{source}"
            );
        }
        let wire = wire(&format!("{HEADER}function fused using v(a: {ty}, b: {ty}, c: {ty}): {ty} pure {{ quire::value::ieee::fma(a, b, c) }}"));
        let node = operation(&wire, &format!("quire.op.ieee.{width}.fma"));
        assert_eq!(node["body"]["operation"]["identity"], format!("quire.op.ieee.{width}.fma"));
        assert_eq!(node["body"]["operator"], "call");
        assert_eq!(node["body"]["arguments"].as_array().unwrap().len(), 3);
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

#[trace("TC-1091", "FR-322-AC-112", "FR-397-AC-6")]
#[test]
fn same_width_float_equality_lowers_as_ieee_numeric_equal() {
    for (_width, ty) in [
        ("float32", "Float32[nearest-even]"),
        ("float64", "Float64[nearest-even]"),
    ] {
        for operator in ["=", "!="] {
            let source = format!("{HEADER}function compare using v(a: {ty}, b: {ty}): Boolean pure {{ a {operator} b }}");
            let wire = wire(&source);
            let numeric_equal = operation(&wire, "quire.op.ieee.numeric_equal");
            assert_ieee_law(&wire, numeric_equal);
            if operator == "!=" {
                assert!(wire["semantic_graph"]["nodes"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|node| node["body"]["operation"]["identity"] == "quire.op.boolean.not"));
            }
        }
        for other in ["Float64[nearest-even]", "Float32[nearest-even]"] {
            if ty == other {
                continue;
            }
            let source = format!(
                "{HEADER}function compare using v(a: {ty}, b: {other}): Boolean pure {{ a = b }}"
            );
            refusal(&source, Code::IllTyped, "type-mismatch");
        }
        let source = format!(
            "{HEADER}function compare using v(a: {ty}, b: Int[0, 10]): Boolean pure {{ a = b }}"
        );
        refusal(&source, Code::IllTyped, "operator-ineligible");
    }
}

#[trace("TC-1091", "FR-397-AC-6")]
#[test]
fn mixed_float_widths_and_exact_arithmetic_refuse_type_mismatch() {
    for width in ["Float32[nearest-even]", "Float64[nearest-even]"] {
        for other in [
            "Float32[nearest-even]",
            "Float64[nearest-even]",
            "Int[0, 10]",
            "Rational[-100, 100; 1, 16]",
        ] {
            if width == other {
                continue;
            }
            for (left, right) in [(width, other), (other, width)] {
                for operator in ["+", "-", "*", "/"] {
                    refusal(
                        &format!("{HEADER}function mixed using v(a: {left}, b: {right}): {width} pure {{ a {operator} b }}"),
                        Code::IllTyped,
                        "type-mismatch",
                    );
                }
            }
        }
    }
}

fn operations<'a>(wire: &'a Value, identity: &str) -> Vec<&'a Value> {
    wire["semantic_graph"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|node| node["body"]["operation"]["identity"] == identity)
        .collect()
}

#[trace("TC-1091", "FR-397-AC-7")]
#[test]
fn relation_intrinsics_lower_to_boolean_ieee_nodes_with_the_law_and_no_mode() {
    for (width, ty) in [
        ("float32", "Float32[nearest-even]"),
        ("float64", "Float64[nearest-even]"),
    ] {
        let _ = width;
        for (intrinsic, identity) in [
            ("numericEqual", "quire.op.ieee.numeric_equal"),
            ("totalOrder", "quire.op.ieee.total_order"),
            ("bitIdentical", "quire.op.ieee.bit_identical"),
        ] {
            let source = format!("{HEADER}function call using v(a: {ty}, b: {ty}): Boolean pure {{ quire::value::ieee::{intrinsic}(a, b) }}");
            let wire = wire(&source);
            let nodes = operations(&wire, identity);
            assert_eq!(nodes.len(), 1, "{identity}");
            assert_ieee_law(&wire, nodes[0]);
            assert!(
                nodes[0]["body"]["operation"]["mode"].is_null(),
                "{identity}"
            );
            assert_eq!(nodes[0]["body"]["arguments"].as_array().unwrap().len(), 2);
            let first = &nodes[0]["body"]["arguments"][0];
            let second = &nodes[0]["body"]["arguments"][1];
            assert_ne!(first, second, "{identity}: arguments are in call order");
        }
    }
}

#[trace("TC-1091", "FR-397-AC-12")]
#[test]
fn float_comparisons_lower_to_numeric_equal_and_total_order_compositions() {
    for ty in ["Float32[nearest-even]", "Float64[nearest-even]"] {
        let lower = |expression: &str| {
            wire(&format!("{HEADER}function compare using v(a: {ty}, b: {ty}): Boolean pure {{ {expression} }}"))
        };
        let count = |wire: &Value, identity: &str| operations(wire, identity).len();
        let equal = lower("a = b");
        assert_eq!(count(&equal, "quire.op.ieee.numeric_equal"), 1);
        assert_eq!(count(&equal, "quire.op.boolean.not"), 0);
        let unequal = lower("a != b");
        assert_eq!(count(&unequal, "quire.op.ieee.numeric_equal"), 1);
        assert_eq!(count(&unequal, "quire.op.boolean.not"), 1);
        for expression in ["a < b", "a <= b", "a > b", "a >= b", "b < a", "b <= a"] {
            let wire = lower(expression);
            // numericEqual(a, a), numericEqual(b, b) and numericEqual(a, b)
            // (or its exchange), one totalOrder, and no bit identity.
            assert_eq!(
                count(&wire, "quire.op.ieee.numeric_equal"),
                3,
                "{expression}"
            );
            assert_eq!(count(&wire, "quire.op.ieee.total_order"), 1, "{expression}");
            assert_eq!(
                count(&wire, "quire.op.ieee.bit_identical"),
                0,
                "{expression}"
            );
            for identity in ["quire.op.ieee.numeric_equal", "quire.op.ieee.total_order"] {
                for node in operations(&wire, identity) {
                    assert_ieee_law(&wire, node);
                }
            }
            let strict = expression.contains('<') && !expression.contains("<=")
                || expression.contains('>') && !expression.contains(">=");
            assert_eq!(
                count(&wire, "quire.op.boolean.not"),
                usize::from(strict),
                "{expression}"
            );
            assert_eq!(
                count(&wire, "quire.op.boolean.or"),
                usize::from(!strict),
                "{expression}"
            );
        }
        // `a > b` is the node of `b < a`, and `a >= b` of `b <= a`.
        let node_identities = |wire: &Value| {
            let mut identities: Vec<String> = wire["semantic_graph"]["nodes"]
                .as_array()
                .unwrap()
                .iter()
                .filter_map(|node| node["body"]["operation"]["identity"].as_str())
                .map(str::to_owned)
                .collect();
            identities.sort();
            identities
        };
        assert_eq!(
            node_identities(&lower("a > b")),
            node_identities(&lower("b < a"))
        );
        assert_eq!(
            node_identities(&lower("a >= b")),
            node_identities(&lower("b <= a"))
        );
    }
}

#[trace("TC-1091", "FR-397-AC-12")]
#[test]
fn float_orderings_refuse_unlike_operands_at_the_operator() {
    for operator in ["<", "<=", ">", ">="] {
        for (left, right) in [
            ("Float32[nearest-even]", "Float64[nearest-even]"),
            ("Float64[nearest-even]", "Float64[toward-zero]"),
            ("Float64[nearest-even]", "Int[0, 10]"),
            ("Int[0, 10]", "Float32[nearest-even]"),
        ] {
            refusal(
                &format!("{HEADER}function compare using v(a: {left}, b: {right}): Boolean pure {{ a {operator} b }}"),
                Code::IllTyped,
                "type-mismatch",
            );
        }
    }
}

#[trace("TC-1091", "FR-397-AC-16")]
#[test]
fn ieee_intrinsic_calls_refuse_bad_arity_types_and_unknown_names() {
    let f64_ty = "Float64[nearest-even]";
    let compile_call = |parameters: &str, call: &str| {
        format!("{HEADER}function call using v({parameters}): Boolean pure {{ {call} }}")
    };
    let ab = format!("a: {f64_ty}, b: {f64_ty}");
    // Argument counts refuse at the call.
    for call in [
        "quire::value::ieee::sqrt(a, b)",
        "quire::value::ieee::fma(a, b)",
        "quire::value::ieee::numericEqual(a)",
        "quire::value::ieee::totalOrder(a, b, a)",
    ] {
        refusal(&compile_call(&ab, call), Code::IllTyped, "type-mismatch");
    }
    // Argument types refuse at the argument.
    for (parameters, call) in [
        (
            format!("i: Int[0, 10], a: {f64_ty}"),
            "quire::value::ieee::sqrt(i)".to_owned(),
        ),
        (
            "f32: Float32[nearest-even], f64: Float64[nearest-even]".to_owned(),
            "quire::value::ieee::totalOrder(f32, f64)".to_owned(),
        ),
        (
            "f32: Float32[nearest-even], f64: Float64[nearest-even]".to_owned(),
            "quire::value::ieee::numericEqual(f32, f64)".to_owned(),
        ),
        (
            "f32: Float32[nearest-even], f64: Float64[nearest-even]".to_owned(),
            "quire::value::ieee::bitIdentical(f32, f64)".to_owned(),
        ),
        (
            format!("a: {f64_ty}, b: Float64[toward-zero]"),
            "quire::value::ieee::numericEqual(a, b)".to_owned(),
        ),
        (
            format!("a: {f64_ty}, b: {f64_ty}, i: Int[0, 10]"),
            "quire::value::ieee::fma(a, b, i)".to_owned(),
        ),
    ] {
        refusal(
            &compile_call(&parameters, &call),
            Code::IllTyped,
            "type-mismatch",
        );
    }
    // Any other name under the namespace is not an intrinsic.
    let refusal = refusal(
        &compile_call(&ab, "quire::value::ieee::cbrt(a)"),
        Code::MissingDeclaration,
        "missing-name",
    );
    let qsl_replay::spine::CompileRefusal::Check { refusals, .. } = *refusal else {
        panic!("an unknown intrinsic refuses at checking");
    };
    assert!(matches!(
        &refusals[0].cause,
        CheckCause::MissingQualifiedName(name) if name.name == "quire::value::ieee::cbrt"
    ));
}

#[trace("TC-1091", "FR-397-AC-16")]
#[test]
fn zero_argument_intrinsic_calls_are_refused_by_the_parser() {
    for call in [
        "quire::value::ieee::sqrt()",
        "quire::value::ieee::bitIdentical()",
    ] {
        let source = format!(
            "{HEADER}function call using v(a: Float64[nearest-even]): Boolean pure {{ {call} }}"
        );
        let Err(AdmittedRefusal::Package(refusal)) = compile(&source) else {
            panic!("{call}: expected a package refusal");
        };
        assert_eq!(refusal.code(), Code::InvalidSyntax, "{call}: {refusal:?}");
        // The refusal is at the closing parenthesis.
        let close = source.find("()").unwrap() + 1;
        assert!(
            format!("{refusal:?}").contains(&format!("start: {close}")),
            "{call}: {refusal:?}"
        );
    }
}

#[trace("TC-1091", "FR-397-AC-16")]
#[test]
fn profile_admission_precedes_intrinsic_argument_typing() {
    // No float is declared, so the IEEE profile is not selected and the
    // call refuses before its argument is typed.
    let source = format!("{HEADER}function call using v(i: Int[0, 10]): Boolean pure {{ quire::value::ieee::sqrt(i) }}");
    let Err(AdmittedRefusal::Package(refusal)) = compile(&source) else {
        panic!("expected a package refusal from {source}");
    };
    assert_eq!(refusal.code(), Code::InvalidPackage, "{refusal:?}");
    let qsl_replay::spine::CompileRefusal::Check { refusals, .. } = *refusal else {
        panic!("the call refuses at checking");
    };
    assert!(matches!(
        refusals[0].cause,
        CheckCause::IeeeProfileNotAdmitted
    ));
}

#[trace("TC-1091", "FR-397-AC-24")]
#[test]
fn f20_cross_width_and_exact_operands_refuse_before_any_charge() {
    let f32_ty = "Float32[nearest-even]";
    let f64_ty = "Float64[nearest-even]";
    let params = format!("a: {f32_ty}, b: {f64_ty}, i: Int[0, 10]");
    for (result, body) in [
        ("Boolean", "quire::value::ieee::numericEqual(a, b)"),
        ("Boolean", "quire::value::ieee::totalOrder(a, b)"),
        ("Boolean", "quire::value::ieee::bitIdentical(a, b)"),
        (f32_ty, "a + b"),
        (f32_ty, "a + i"),
        (
            "Decimal[0, 10; 0, 0; nearest-even]",
            "convert<Decimal[0, 10; 0, 0; nearest-even]>(a)",
        ),
        ("Int[0, 1]", "convert<Int[0, 1]>(a)"),
    ] {
        refusal(
            &format!("{HEADER}function refused using v({params}): {result} pure {{ {body} }}"),
            Code::IllTyped,
            "type-mismatch",
        );
    }
}

#[trace("TC-1091", "FR-397-AC-15")]
#[test]
fn fused_multiply_add_lowers_with_its_float_width_rounding_mode_and_profile_law() {
    for (width, ty) in [
        ("float32", "Float32[nearest-even]"),
        ("float64", "Float64[nearest-even]"),
    ] {
        let source = format!(
            "{HEADER}function fused using v(a: {ty}, b: {ty}, c: {ty}): {ty} pure {{ quire::value::ieee::fma(a, b, c) }}"
        );
        let wire = wire(&source);
        let operation = operation(&wire, &format!("quire.op.ieee.{width}.fma"));
        assert_ieee_law(&wire, operation);
        assert_eq!(
            operation["body"]["operation"]["mode"],
            json!({"kind": "rounding", "value": "nearest-even"}),
        );
        assert_eq!(operation["body"]["arguments"].as_array().unwrap().len(), 3);
    }
}

#[trace("TC-1091", "FR-397-AC-15")]
#[test]
fn square_root_lowers_with_its_float_width_rounding_mode_and_profile_law() {
    for (width, ty) in [
        ("float32", "Float32[nearest-even]"),
        ("float64", "Float64[nearest-even]"),
    ] {
        let source = format!(
            "{HEADER}function root using v(a: {ty}): {ty} pure {{ quire::value::ieee::sqrt(a) }}"
        );
        let wire = wire(&source);
        let operation = operation(&wire, &format!("quire.op.ieee.{width}.sqrt"));
        assert_ieee_law(&wire, operation);
        assert_eq!(
            operation["body"]["operation"]["mode"],
            json!({"kind": "rounding", "value": "nearest-even"}),
        );
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

// FR-322's `float32_bits` and `float64_bits` scalar literal forms.
#[trace("TC-1091", "FR-397-AC-20")]
#[test]
fn bit_level_float_constructors_lower_to_exact_wire_literals() {
    for (width, bits) in [("32", "0x3f800000"), ("64", "0x3ff0000000000000")] {
        let source = format!("{HEADER}function one using v(): Float{width}[nearest-even] pure {{ float{width}(bits: {bits}) }}");
        let wire = wire(&source);
        let value_kind = format!("float{width}_bits");
        let literal_bits = bits.trim_start_matches("0x");
        assert!(wire["semantic_graph"]["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|node| node["body"]["term"] == "literal"
                && node["body"]["value_kind"] == value_kind
                && node["body"]["value"] == literal_bits));
        assert_eq!(ieee_selection(&wire).len(), 1);
    }
}

#[trace("TC-1091", "FR-397-AC-1", "FR-397-AC-3")]
#[test]
fn admitted_float_arithmetic_and_rational_conversion_evaluate_through_the_source_spine() {
    use qsl_eval::value::{CheckedPackageEvaluation, QualifiedName};
    use qsl_replay::spine::{self, ParseRequest, SpineLimits};
    use qsl_semantics::model::object_environment::ObjectEnvironment;
    use quire_eval::FamilyOutcome;
    use quire_exact::{Cancel, IeeeValue, Integer, Meter, Outcome, Rational, Value as KernelValue};

    for (width, left, right) in [
        (
            "Float32",
            IeeeValue::binary32(1.5_f32.to_bits()),
            IeeeValue::binary32(0.5_f32.to_bits()),
        ),
        (
            "Float64",
            IeeeValue::binary64(1.5_f64.to_bits()),
            IeeeValue::binary64(0.5_f64.to_bits()),
        ),
    ] {
        for (operator, expected) in [("+", 2_i64), ("-", 1), ("*", 0), ("/", 3)] {
            // Multiplication has the exact rational result 3/4; the other
            // three results are integers, represented as rationals too.
            let expected = if operator == "*" {
                Rational::new(Integer::from(3_i64), Integer::from(4_i64)).unwrap()
            } else {
                Rational::new(Integer::from(expected), Integer::one()).unwrap()
            };
            let text = format!("{HEADER}function agrees using v(a: {width}[nearest-even], b: {width}[nearest-even], expected: Rational[-100, 100; 1, 16]): Boolean pure {{ convert<Rational[-100, 100; 1, 16]>(a {operator} b) = expected }}");
            let emitted = wire(&text);
            let arithmetic_name = match operator {
                "+" => "add",
                "-" => "sub",
                "*" => "mul",
                "/" => "div",
                _ => unreachable!("the test enumerates four IEEE arithmetic operators"),
            };
            let arithmetic = operation(
                &emitted,
                &format!("quire.op.ieee.{}.{arithmetic_name}", width.to_lowercase()),
            );
            assert_ieee_law(&emitted, arithmetic);
            assert_eq!(
                arithmetic["body"]["operation"]["mode"],
                json!({"kind": "rounding", "value": "nearest-even"}),
            );
            assert_ieee_law(&emitted, operation(&emitted, "quire.op.ieee.to_rational"));
            let source = SourceIdentity::new("a", "ieee", "git", "1");
            let limits = SpineLimits::default();
            let cancel = Cancel::new();
            let parsed = spine::parse(
                &ParseRequest {
                    source: &source,
                    path: "ieee.native",
                    bytes: text.as_bytes(),
                },
                limits.source,
                &cancel,
            )
            .expect("source parses")
            .into_value();
            let models = spine::select(&parsed, &Default::default(), limits.model, &cancel)
                .expect("models select")
                .into_value();
            let checked = spine::check(
                &parsed,
                "fixture".to_owned(),
                &models,
                &DependencyInput::default(),
                limits,
                &cancel,
            )
            .expect("source checks without an injected IEEE profile")
            .into_value();
            let evaluation = checked
                .package()
                .call(
                    &QualifiedName::unqualified("agrees").unwrap(),
                    vec![
                        KernelValue::Float(left),
                        KernelValue::Float(right),
                        KernelValue::Rational(expected),
                    ],
                    &ObjectEnvironment::default(),
                    &mut Meter::new(spine::default_accounting(1_000_000)),
                )
                .expect("the admitted float call runs");
            assert!(
                matches!(
                    evaluation.outcome,
                    FamilyOutcome::Evaluated(Outcome::Completed(KernelValue::Boolean(true)))
                ),
                "{width} {operator}: {:?}",
                evaluation.outcome,
            );
        }
    }
}
