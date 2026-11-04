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
            source: SourceIdentity::new("a", "ieee"),
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
            format!("{HEADER}function root using v(a: {ty}): {ty} pure {{ quire::value::ieee::sqrt(a) }}"),
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
        assert_eq!(
            node["body"]["operation"]["identity"],
            format!("quire.op.ieee.{width}.fma")
        );
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

fn wire_of(text: &str) -> Value {
    wire(text)
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
    for ty in ["Float32[nearest-even]", "Float64[nearest-even]"] {
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
            // The arguments are the two parameters in call order: exchanging
            // the call's operands exchanges the node's arguments.
            let exchanged = wire_of(&format!("{HEADER}function call using v(a: {ty}, b: {ty}): Boolean pure {{ quire::value::ieee::{intrinsic}(b, a) }}"));
            let swapped = operations(&exchanged, identity);
            let arguments = nodes[0]["body"]["arguments"].as_array().unwrap();
            let exchanged_arguments = swapped[0]["body"]["arguments"].as_array().unwrap();
            assert_ne!(arguments[0], arguments[1], "{identity}");
            assert_eq!(arguments[0], exchanged_arguments[1], "{identity}");
            assert_eq!(arguments[1], exchanged_arguments[0], "{identity}");
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

#[trace("TC-1091", "FR-397-AC-25")]
#[test]
fn profile_admission_precedes_intrinsic_arity_and_argument_typing() {
    // No float form: the profile is not selected, so each call refuses
    // `invalid_package`/`IeeeProfileNotAdmitted` before its count or its
    // argument is typed.
    for call in [
        "quire::value::ieee::sqrt(i)",
        "quire::value::ieee::sqrt(i, i)",
        "quire::value::ieee::numericEqual(i)",
    ] {
        let source = format!(
            "{HEADER}function call using v(i: Int[0, 10], a: Boolean): Boolean pure {{ {call} }}"
        );
        let Err(AdmittedRefusal::Package(refusal)) = compile(&source) else {
            panic!("expected a package refusal from {source}");
        };
        assert_eq!(refusal.code(), Code::InvalidPackage, "{call}: {refusal:?}");
        let qsl_replay::spine::CompileRefusal::Check { refusals, .. } = *refusal else {
            panic!("{call}: the call refuses at checking");
        };
        assert!(
            matches!(refusals[0].cause, CheckCause::IeeeProfileNotAdmitted),
            "{call}: {:?}",
            refusals[0].cause
        );
    }
    // One float parameter selects the profile: the same calls are type
    // mismatches.
    for call in [
        "quire::value::ieee::sqrt(i)",
        "quire::value::ieee::sqrt(i, i)",
    ] {
        refusal(
            &format!("{HEADER}function call using v(i: Int[0, 10], a: Boolean, f: Float64[nearest-even]): Boolean pure {{ {call} }}"),
            Code::IllTyped,
            "type-mismatch",
        );
    }
}

#[trace("TC-1091", "FR-397-AC-25")]
#[test]
fn zero_argument_sqrt_is_a_parse_refusal_with_and_without_a_float_form() {
    for parameters in [
        "i: Int[0, 10], a: Boolean",
        "i: Int[0, 10], a: Boolean, f: Float64[nearest-even]",
    ] {
        let source = format!("{HEADER}function call using v({parameters}): Boolean pure {{ quire::value::ieee::sqrt() }}");
        let Err(AdmittedRefusal::Package(refusal)) = compile(&source) else {
            panic!("expected a package refusal from {source}");
        };
        assert_eq!(refusal.code(), Code::InvalidSyntax, "{refusal:?}");
        assert!(
            format!("{refusal:?}").contains("UnexpectedToken"),
            "{refusal:?}"
        );
    }
}

#[trace("TC-1091", "FR-397-AC-24", "FR-397-AC-7", "FR-397-AC-15")]
#[test]
fn bit_constructors_take_their_type_from_a_sibling_operand_in_every_intrinsic() {
    for (width, ty) in [(32, "Float32[nearest-even]"), (64, "Float64[nearest-even]")] {
        let zero = if width == 32 {
            "float32(bits: 0x00000000)"
        } else {
            "float64(bits: 0x0000000000000000)"
        };
        let other = if width == 32 {
            "float64(bits: 0x0000000000000000)"
        } else {
            "float32(bits: 0x00000000)"
        };
        let check = |result: &str, body: String| {
            let source = format!("{HEADER}function f using v(a: {ty}): {result} pure {{ {body} }}");
            wire(&source)
        };
        for intrinsic in ["numericEqual", "totalOrder", "bitIdentical"] {
            check(
                "Boolean",
                format!("quire::value::ieee::{intrinsic}(a, {zero})"),
            );
            check(
                "Boolean",
                format!("quire::value::ieee::{intrinsic}({zero}, a)"),
            );
            refusal(
                &format!("{HEADER}function f using v(a: {ty}): Boolean pure {{ quire::value::ieee::{intrinsic}(a, {other}) }}"),
                Code::IllTyped,
                "type-mismatch",
            );
        }
        for body in [
            format!("quire::value::ieee::fma(a, {zero}, {zero})"),
            format!("quire::value::ieee::fma({zero}, a, {zero})"),
            format!("quire::value::ieee::fma({zero}, {zero}, a)"),
            format!("quire::value::ieee::sqrt({zero})"),
        ] {
            check(ty, body);
        }
        check("Boolean", format!("{zero} < a"));
        check("Boolean", format!("a >= {zero}"));
    }
}

#[trace("TC-1091", "FR-397-AC-16")]
#[test]
fn a_supplied_library_never_replaces_an_ieee_intrinsic() {
    let library_body = format!(
        "{HEADER}function sqrt using v(a: Float64[nearest-even]): Float64[nearest-even] pure {{ a }}\n\
         function fma using v(a: Float64[nearest-even]): Float64[nearest-even] pure {{ a }}\n"
    );
    let dependencies = DependencyInput::new(vec![qsl_replay::SuppliedLibrary {
        identity: "quire::value::ieee".to_owned(),
        source: SourceIdentity::new("a", "ieee-library"),
        path: "ieee-library.native".to_owned(),
        bytes: library_body.into_bytes(),
    }])
    .expect("an admissible dependency input");
    for call in ["sqrt(a)", "fma(a, a, a)"] {
        let unit = format!("{HEADER}import \"quire::value::ieee\" as q;\nfunction call using v(a: Float64[nearest-even]): Float64[nearest-even] pure {{ quire::value::ieee::{call} }}");
        let Err(AdmittedRefusal::Package(refusal)) = compile_admitted(
            qsl_replay::CompileUnit {
                source: SourceIdentity::new("a", "ieee"),
                package_name: "fixture".to_owned(),
                path: "ieee.native",
                bytes: unit.as_bytes(),
            },
            [],
            &dependencies,
            StageLimits::default(),
            ReplayLimits::default(),
        ) else {
            panic!("{call}: the call must refuse");
        };
        assert_eq!(refusal.code(), Code::InvalidPackage, "{call}: {refusal:?}");
        assert_eq!(
            refusal.cause(),
            Some("conflicting-definition"),
            "{call}: {refusal:?}"
        );
        let identity = format!("quire::value::ieee::{}", call.split('(').next().unwrap());
        assert!(
            format!("{refusal:?}").contains(&identity),
            "{call}: {refusal:?}"
        );
    }
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

/// FR-397-AC-8 (QSL-1655): an exact source lowers to `from_exact`, a float of
/// the other width to `to_float32` (with the target's mode) or `to_float64`
/// (with none), each with the law and the target as the type argument; the
/// strict IR reader admits each package (`compile_admitted` reads it).
#[trace("TC-1091", "FR-397-AC-8")]
#[test]
fn conversion_into_a_float_lowers_to_the_owned_ieee_operation() {
    let rounding = |mode: &str| json!({"kind": "rounding", "value": mode});
    for (source, target, identity, mode) in [
        (
            "Int[0, 10]",
            "Float64[nearest-even]",
            "from_exact",
            Some("nearest-even"),
        ),
        (
            "Rational[-10, 10; 1, 10]",
            "Float64[nearest-even]",
            "from_exact",
            Some("nearest-even"),
        ),
        (
            "Decimal[0, 100; 0, 3; nearest-even]",
            "Float64[nearest-even]",
            "from_exact",
            Some("nearest-even"),
        ),
        (
            "Float64[nearest-even]",
            "Float32[toward-zero]",
            "to_float32",
            Some("toward-zero"),
        ),
        (
            "Float32[nearest-even]",
            "Float64[exact]",
            "to_float64",
            None,
        ),
    ] {
        let text = format!(
            "{HEADER}function c using v(a: {source}): {target} pure {{ convert<{target}>(a) }}"
        );
        let wire = wire(&text);
        let node = operation(&wire, &format!("quire.op.ieee.{identity}"));
        assert_ieee_law(&wire, node);
        let lowered = &node["body"]["operation"];
        assert_eq!(
            lowered["mode"],
            mode.map_or(Value::Null, rounding),
            "{text}"
        );
        assert_eq!(lowered["member"]["kind"], "type_argument", "{text}");
        // The member names the target's own scalar type node.
        let declaration = &lowered["member"]["declaration"];
        let target_node = wire["semantic_graph"]["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .find(|candidate| &candidate["node_id"] == declaration)
            .unwrap_or_else(|| panic!("no node {declaration} in {wire}"));
        let (width, target_mode) = target.trim_end_matches(']').split_once('[').unwrap();
        assert_eq!(target_node["semantic_form"], "float_rounding", "{text}");
        assert_eq!(
            target_node["body"]["members"][0]["value"]["value"], target_mode,
            "{text}"
        );
        assert_eq!(
            target_node["semantic_type"]["path"][1],
            width.to_lowercase(),
            "{text}"
        );
        assert_eq!(&node["semantic_type"], declaration, "{text}");
    }
}

/// FR-397-AC-18: a quantity converts to no float, and a same-width conversion
/// to another mode types as the target, so `+` takes the target's mode.
#[trace("TC-1091", "FR-397-AC-18")]
#[test]
fn quantity_to_float_refuses_and_a_retyped_operand_adds_in_the_target_mode() {
    refusal(
        &format!("{HEADER}dimension Length;\nunit m : Length = rational(1, 1);\nfunction c using v(q: Quantity<m>): Float64[nearest-even] pure {{ convert<Float64[nearest-even]>(q) }}"),
        Code::IllTyped,
        "type-mismatch",
    );
    let text = format!("{HEADER}function c using v(f: Float64[nearest-even], b: Float64[toward-zero]): Float64[toward-zero] pure {{ convert<Float64[toward-zero]>(f) + b }}");
    let wire = wire(&text);
    let adds: Vec<_> = wire["semantic_graph"]["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|node| node["body"]["operation"]["identity"] == "quire.op.ieee.float64.add")
        .collect();
    assert_eq!(adds.len(), 1, "{wire}");
    assert_eq!(
        adds[0]["body"]["operation"]["mode"],
        json!({"kind": "rounding", "value": "toward-zero"})
    );
}

/// FR-397-AC-8 and FR-397-AC-18: a conversion to the operand's own type, or to
/// the same width under another mode, builds no conversion node; a float to
/// `Decimal[..]`, `Integer` or `Int[..]`, and a quantity or `Boolean` to a
/// float, refuse `ill_typed`/`type-mismatch`.
#[trace("TC-1091", "FR-397-AC-8", "FR-397-AC-18")]
#[test]
fn same_width_conversion_builds_no_node_and_non_exact_sources_refuse() {
    for target in [
        "Float64[nearest-even]",
        "Float64[toward-zero]",
        "Float64[exact]",
    ] {
        let text = format!("{HEADER}function c using v(a: Float64[nearest-even]): {target} pure {{ convert<{target}>(a) }}");
        let wire = wire(&text);
        let nodes = wire["semantic_graph"]["nodes"].as_array().unwrap();
        assert!(
            nodes.iter().all(|node| {
                let identity = node["body"]["operation"]["identity"].as_str().unwrap_or("");
                !identity.starts_with("quire.op.ieee.to_float")
                    && !identity.contains("convert")
                    && identity != "quire.op.ieee.from_exact"
            }),
            "{target}"
        );
    }
    for (source, target) in [
        (
            "Float64[nearest-even]",
            "Decimal[0, 100; 0, 3; nearest-even]",
        ),
        ("Float64[nearest-even]", "Integer"),
        ("Float64[nearest-even]", "Int[0, 10]"),
        ("Boolean", "Float64[nearest-even]"),
    ] {
        refusal(
            &format!(
                "{HEADER}function c using v(a: {source}): {target} pure {{ convert<{target}>(a) }}"
            ),
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
            let source = SourceIdentity::new("a", "ieee");
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

/// The package of two functions-worth of parameters whose body applies `fma`
/// at both widths and compares an integer, so a wire node of each operand
/// type exists to be referenced.
fn fma_base_wire() -> Value {
    let ty = "Float64[nearest-even]";
    let single = "Float32[nearest-even]";
    wire(&format!("{HEADER}function f using v(a: {ty}, b: {ty}, c: {ty}, s: {single}, t: {single}, u: {single}, i: Int[0, 10]): Boolean pure {{ quire::value::ieee::fma(a, b, c) = quire::value::ieee::fma(a, b, c) and quire::value::ieee::fma(s, t, u) = quire::value::ieee::fma(s, t, u) and i = i }}"))
}

/// Read `wire` through the strict V2 reader: `Ok` when it is admitted, else
/// the refusal's `(code, cause)` as the published tags.
fn strict_read(wire: &Value) -> Result<(), (String, String)> {
    let bytes = serde_json::to_vec(wire).unwrap();
    let mut evidence = CheckedPackageEvidence::new();
    for feature in wire["lock"]["required_features"].as_array().unwrap() {
        evidence.support_feature(feature.as_str().unwrap());
    }
    match CheckedPackageV2::read(&bytes, CheckedPackageReadLimits::bounded(), &evidence) {
        CheckedPackageV2ReadResult::Admitted(_) => Ok(()),
        CheckedPackageV2ReadResult::Refused(refusal) => {
            let tag = |value: Value| value.as_str().unwrap_or_default().to_owned();
            Err((
                tag(serde_json::to_value(refusal.code).unwrap()),
                tag(serde_json::to_value(refusal.cause).unwrap()),
            ))
        }
        other => panic!("unexpected read result {other:?}"),
    }
}

/// The wire node applying `identity`, mutably.
fn application_mut<'a>(wire: &'a mut Value, identity: &str) -> &'a mut Value {
    wire["semantic_graph"]["nodes"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|node| node["body"]["operation"]["identity"] == identity)
        .unwrap_or_else(|| panic!("missing {identity}"))
}

fn set_arguments(node: &mut Value, arguments: Vec<Value>) {
    let mut targets: Vec<Value> = Vec::new();
    for argument in &arguments {
        let target = argument["target"].clone();
        if !targets.contains(&target) {
            targets.push(target);
        }
    }
    // The reader keeps a node's dependencies in key order.
    targets.sort_by_key(|target| target["path"].to_string());
    node["body"]["arguments"] = Value::Array(arguments);
    node["dependencies"] = Value::Array(targets);
}

#[trace("TC-1091", "FR-397-AC-4", "FR-397-AC-15")]
#[test]
fn the_published_fma_vectors_get_their_published_decision_from_the_strict_reader() {
    let Some(root) = std::env::var_os("QSPEC_DIR") else {
        println!("skipped: QSPEC_DIR not set");
        return;
    };
    let path = std::path::Path::new(&root)
        .join("proposals/checked-package-v2/fma-application-vectors.json");
    let vectors: Value =
        serde_json::from_slice(&std::fs::read(path).expect("the FM vectors")).unwrap();
    let base = fma_base_wire();
    let nodes = base["semantic_graph"]["nodes"].as_array().unwrap();
    let find = |identity: &str| {
        nodes
            .iter()
            .find(|node| node["body"]["operation"]["identity"] == identity)
            .unwrap_or_else(|| panic!("missing {identity}"))
            .clone()
    };
    let (double, single, integer) = (
        find("quire.op.ieee.float64.fma"),
        find("quire.op.ieee.float32.fma"),
        find("quire.op.integer.eq"),
    );
    let operand = |width: &str, position: usize| match width {
        "float64" => double["body"]["arguments"][position % 3].clone(),
        "float32" => single["body"]["arguments"][position % 3].clone(),
        "integer" => integer["body"]["arguments"][0].clone(),
        other => panic!("width {other}"),
    };
    let result_type = |width: &str| match width {
        "float64" => double["body"]["result_type"].clone(),
        "float32" => single["body"]["result_type"].clone(),
        other => panic!("width {other}"),
    };
    let mut decided = 0;
    for case in vectors["cases"].as_array().unwrap() {
        let id = case["id"].as_str().unwrap();
        let identity = case["identity"].as_str().unwrap();
        let mut wire = base.clone();
        let node = application_mut(&mut wire, "quire.op.ieee.float64.fma");
        node["body"]["operation"]["identity"] = json!(identity);
        if case["mode"].is_null() {
            node["body"]["operation"]["mode"] = Value::Null;
        }
        if case["laws"].as_array().unwrap().is_empty() {
            node["body"]["operation"]["laws"] = json!([]);
        }
        let arguments: Vec<Value> = case["argument_widths"]
            .as_array()
            .unwrap()
            .iter()
            .enumerate()
            .map(|(position, width)| operand(width.as_str().unwrap(), position))
            .collect();
        set_arguments(node, arguments);
        let result = result_type(case["result_width"].as_str().unwrap());
        node["semantic_type"] = result.clone();
        node["body"]["result_type"] = result;
        let read = strict_read(&wire);
        match case["expected"].as_str().unwrap() {
            "admitted" => assert_eq!(read, Ok(()), "{id}"),
            "refused" => assert_eq!(
                read,
                Err((
                    case["code"].as_str().unwrap().to_owned(),
                    case["cause"].as_str().unwrap().to_owned()
                )),
                "{id}"
            ),
            other => panic!("{id}: {other}"),
        }
        decided += 1;
    }
    assert_eq!(decided, 12);
}

/// Not a published vector: an `fma` whose rounding mode differs from the one
/// its operand and result types carry. The strict reader refuses it, so the
/// mode-to-type agreement is the reader's check and not only the checker's.
#[trace("TC-1091", "FR-397-AC-15")]
#[test]
fn an_fma_mode_differing_from_its_operand_types_is_refused_by_the_strict_reader() {
    let mut wire = fma_base_wire();
    assert_eq!(strict_read(&wire), Ok(()));
    let node = application_mut(&mut wire, "quire.op.ieee.float64.fma");
    node["body"]["operation"]["mode"] = json!({"kind": "rounding", "value": "toward-zero"});
    assert_eq!(
        strict_read(&wire),
        Err((
            "invalid_package".to_owned(),
            "operation-mode-type-mismatch".to_owned()
        ))
    );
}
