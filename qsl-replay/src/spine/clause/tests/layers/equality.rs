// SPDX-License-Identifier: AGPL-3.0-or-later
//! Typed equality admission through the source-to-checked-package spine.

use super::*;
use qsl_semantics::check::CheckCause;

fn document() -> Vec<u8> {
    let mut document: serde_json::Value = serde_json::from_slice(&domain_document()).unwrap();
    let mut construct = document["constructs"][0].clone();
    construct["kind"]["name"] = json!("record_value_type");
    construct["construct"]["meaning"] = json!("quire.meaning.model.record-value-type/v1");
    document["constructs"]
        .as_array_mut()
        .unwrap()
        .push(construct);
    let field = |owner: &str, name: &str, ty: &str, optional: bool, upper: u64| {
        let mut field = document["types"][0]["fields"][0].clone();
        let identity = format!("ix://{PACKAGE_IDENTITY}/{owner}/{name}");
        field["identity"] = json!(identity);
        field["name"] = json!(name);
        field["typeRef"] = json!(ty);
        field["presence"] = json!(if optional { "optional" } else { "required" });
        field["multiplicity"] =
            json!({"lower": 1, "upper": upper, "ordered": true, "unique": false});
        field["origin"]["generated"]["generatorIdentity"] = json!(identity);
        field["origin"]["generated"]["inputIdentities"] = json!([identity]);
        field
    };
    let integer = format!("ix://{PACKAGE_IDENTITY}/Digit");
    let record_id = format!("ix://{PACKAGE_IDENTITY}/Pair");
    let mut record = document["types"][0].clone();
    record["identity"] = json!(record_id);
    record["displayName"] = json!(record_id);
    record["kind"]["name"] = json!("record_value_type");
    record["fields"] = json!([
        field("Pair", "n", &integer, false, 1),
        field("Pair", "b", "ix://quire/native/Boolean", false, 1)
    ]);
    let fields = [
        field("Node", "pair", &record_id, false, 1),
        field("Node", "numbers", &integer, false, 3),
        field("Node", "optional", &integer, true, 1),
        field("Node", "number", &integer, false, 1),
        field("Node", "flag", "ix://quire/native/Boolean", false, 1),
        field("Node", "references", &node_type(), false, 3),
        field(
            "Node",
            "referencePair",
            &format!("ix://{PACKAGE_IDENTITY}/ReferencePair"),
            false,
            1,
        ),
    ];
    let mut reference_record = record.clone();
    let identity = format!("ix://{PACKAGE_IDENTITY}/ReferencePair");
    reference_record["identity"] = json!(identity);
    reference_record["displayName"] = json!(identity);
    reference_record["fields"] = json!([field("ReferencePair", "target", &node_type(), false, 1)]);
    let mut scalar_construct = document["constructs"][0].clone();
    scalar_construct["kind"]["name"] = json!("value_type");
    scalar_construct["construct"]["meaning"] = json!("quire.meaning.model.value-type/v1");
    let mut digit = document["types"][0].clone();
    digit["identity"] = json!(integer);
    digit["displayName"] = json!(integer);
    digit["kind"]["name"] = json!("value_type");
    for member in ["fields", "operations", "supertypes"] {
        digit.as_object_mut().unwrap().remove(member);
    }
    digit["scalar"] = json!("integer");
    digit["constraints"] = json!([("min", "0"), ("max", "9")].map(|(keyword, value)| json!({
        "identity": format!("{integer}/constraints/{keyword}"), "keyword": keyword,
        "operands": {"value": value}, "appliesTo": "ix://quire/native/Integer",
        "diagnosticCode": format!("bound.{keyword}"), "origin": digit["origin"].clone(),
    })));
    document["types"][0]["fields"]
        .as_array_mut()
        .unwrap()
        .extend(fields);
    document["types"].as_array_mut().unwrap().push(record);
    document["types"]
        .as_array_mut()
        .unwrap()
        .push(reference_record);
    document["types"].as_array_mut().unwrap().push(digit);
    document["constructs"]
        .as_array_mut()
        .unwrap()
        .push(scalar_construct);
    serde_json::to_vec(&document).unwrap()
}

fn equality_compile(
    layer: &str,
    body: &str,
) -> (String, Result<ComposedUnit, Box<CompileRefusal>>) {
    model_compile(
        layer,
        &format!("invariant Equal using v on M::Node at current {{ {body} }}"),
    )
}

fn model_compile(
    layer: &str,
    declarations: &str,
) -> (String, Result<ComposedUnit, Box<CompileRefusal>>) {
    let document = document();
    let packages = qsl_semantics::model::intake::package_input([document.as_slice()]).unwrap();
    let text = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = {layer:?};\nmodel M = {PACKAGE_IDENTITY:?};\n\
         {declarations}"
    );
    let compiled = crate::spine::compose(
        source(),
        "equality.native",
        text.as_bytes(),
        &packages,
        &DependencyInput::default(),
        SpineLimits::default(),
    );
    (text, compiled)
}

fn refuses(layer: &str, operand: &str, required: HeaderLayer) {
    for operator in ["=", "!="] {
        let (text, compiled) = equality_compile(layer, &format!("{operand} {operator} {operand}"));
        let Err(refusal) = compiled else {
            panic!("{layer} admitted {operand} {operator} {operand}");
        };
        let refusal = *refusal;
        let message = refusal.to_string();
        assert!(message.contains(layer), "{message}");
        assert!(message.contains(required.identity()), "{message}");
        let CompileRefusal::Check { refusals, region } = &refusal else {
            panic!("typed equality must refuse at check, got {refusal:?}");
        };
        let [found] = refusals.as_slice() else {
            panic!("one refusal: {refusals:?}")
        };
        let CheckCause::LayerEquality {
            selected,
            required: actual,
            span,
        } = found.cause
        else {
            panic!("expected a typed layer refusal: {found:?}");
        };
        assert_eq!(selected, HeaderLayer::from_identity(layer).unwrap());
        assert_eq!(actual, required);
        assert_eq!(found.cause.code(), Code::UnsupportedConstruct);
        assert_eq!(found.cause.cause(), Some("expression-form"));
        let span = span.expect("an authored operator");
        assert_eq!(&text[span.start..span.end], operator);
        let region = region.as_ref().expect("operator source region");
        assert_eq!(region.start(), u64::try_from(span.start).unwrap());
        assert_eq!(region.end(), u64::try_from(span.end).unwrap());
    }
}

macro_rules! refused_family {
    ($name:ident, $layer:expr, $operand:expr, $required:expr) => {
        #[trace("TC-490", "FR-110-AC-10")]
        #[test]
        fn $name() {
            refuses($layer, $operand, $required);
        }
    };
}

refused_family!(core_record, CORE, "self.pair", HeaderLayer::ValueComplete);
refused_family!(
    core_sequence,
    CORE,
    "self.numbers",
    HeaderLayer::ValueComplete
);
refused_family!(
    core_option,
    CORE,
    "self.optional",
    HeaderLayer::ValueComplete
);
refused_family!(core_reference, CORE, "self", HeaderLayer::StateGraph);
refused_family!(
    queries_record,
    QUERIES,
    "self.pair",
    HeaderLayer::ValueComplete
);
refused_family!(
    queries_sequence,
    QUERIES,
    "self.numbers",
    HeaderLayer::ValueComplete
);
refused_family!(
    queries_option,
    QUERIES,
    "self.optional",
    HeaderLayer::ValueComplete
);
refused_family!(queries_reference, QUERIES, "self", HeaderLayer::StateGraph);
refused_family!(graph_record, GRAPH, "self.pair", HeaderLayer::ValueComplete);
refused_family!(
    graph_sequence,
    GRAPH,
    "self.numbers",
    HeaderLayer::ValueComplete
);
refused_family!(
    graph_option,
    GRAPH,
    "self.optional",
    HeaderLayer::ValueComplete
);
refused_family!(value_reference, VALUE, "self", HeaderLayer::StateGraph);

#[trace("TC-490", "FR-110-AC-10", "FR-110-AC-13")]
#[test]
fn admitted_families_compile_under_each_layer() {
    for layer in [CORE, QUERIES, GRAPH, VALUE, MODEL, TEMPORAL] {
        let mut operands = vec!["self.number", "self.flag"];
        if [GRAPH, MODEL, TEMPORAL].contains(&layer) {
            operands.push("self");
        }
        if [VALUE, MODEL, TEMPORAL].contains(&layer) {
            operands.extend(["self.pair", "self.numbers", "self.optional"]);
        }
        for operand in operands {
            for operator in ["=", "!="] {
                let body = format!("{operand} {operator} {operand}");
                equality_compile(layer, &body)
                    .1
                    .unwrap_or_else(|refusal| panic!("{layer}: {body}: {refusal:?}"));
            }
        }
    }
}

#[trace("TC-490", "FR-110-AC-10")]
#[test]
fn ill_typed_equality_keeps_its_type_refusal() {
    let (_, compiled) = equality_compile(CORE, "self.pair = 1");
    let refusal = compiled.expect_err("record and integer are different types");
    let CompileRefusal::Check { refusals, .. } = *refusal else {
        panic!("{refusal:?}")
    };
    assert_eq!(refusals.len(), 1);
    assert_eq!(refusals[0].cause.code(), Code::IllTyped);
    assert_eq!(refusals[0].cause.cause(), Some("type-mismatch"));
}

#[trace("TC-490", "FR-110-AC-11")]
#[test]
fn nested_reference_families_require_both_layers() {
    for operand in ["self.references", "self.referencePair"] {
        refuses(CORE, operand, HeaderLayer::StateGraph);
        refuses(QUERIES, operand, HeaderLayer::StateGraph);
        refuses(VALUE, operand, HeaderLayer::StateGraph);
        refuses(GRAPH, operand, HeaderLayer::ValueComplete);
        for layer in [MODEL, TEMPORAL] {
            equality_compile(layer, &format!("{operand} = {operand}"))
                .1
                .expect("model closure admits both families");
        }
    }
}

#[trace("TC-490", "FR-110-AC-11")]
#[test]
fn an_unselected_declaration_keeps_default_value_admission() {
    compile(
        &[],
        "record R { n: Int[0, 9]; }\n\
        function records(a: R, b: R): Boolean pure { a = b }\n\
        function integers(a: Int[0, 9], b: Int[0, 9]): Boolean pure { a = b }\n\
        predicate same(a: Int[0, 9], b: Int[0, 9]): Boolean { a = b }",
    )
    .1
    .expect("no header selection leaves equalities unrestricted");
}

#[trace("TC-490", "FR-110-AC-10")]
#[test]
fn tuple_union_and_collection_kinds_are_whole_values() {
    for ty in [
        "Pair",
        "Choice",
        "Set<Int[0, 9]>[0, 3]",
        "Bag<Int[0, 9]>[0, 3]",
        "OrderedSet<Int[0, 9]>[0, 3]",
    ] {
        for layer in [QUERIES, GRAPH, VALUE, MODEL, TEMPORAL] {
            let declarations = format!(
                "tuple Pair(Int[0, 9], Boolean);\n\
                union Choice {{ Empty, Item(Int[0, 9]) }}\n\
                predicate f using v(a: {ty}, b: {ty}): Boolean {{ a != b }}"
            );
            let (_, compiled) = compile(&[("v", layer)], &declarations);
            if [VALUE, MODEL, TEMPORAL].contains(&layer) {
                compiled.unwrap_or_else(|refusal| panic!("{ty} under {layer}: {refusal:?}"));
            } else {
                let Err(refusal) = compiled else {
                    panic!("{layer} admitted whole {ty} equality")
                };
                let CompileRefusal::Check { refusals, .. } = *refusal else {
                    panic!("typed layer refusal")
                };
                assert_eq!(refusals.len(), 1);
                assert!(matches!(
                    refusals[0].cause,
                    CheckCause::LayerEquality {
                        required: HeaderLayer::ValueComplete,
                        ..
                    }
                ));
            }
        }
    }
}

#[trace("TC-490", "FR-110-AC-10")]
#[test]
fn enum_equality_is_scalar_under_core() {
    compile(
        &[("v", CORE)],
        "enum Color { Red, Blue }\n\
        invariant ColorEqual using v on M::Node at current { Color::Red != Color::Blue }",
    )
    .1
    .expect("enum equality is a core scalar form");
}

#[trace("TC-490", "FR-110-AC-10")]
#[test]
fn function_equalities_use_their_selected_layer() {
    for (layer, admitted) in [(VALUE, false), (MODEL, true), (TEMPORAL, true)] {
        let declarations = "function f using v(a: Reference<M::Node>, b: Reference<M::Node>): Boolean pure { a != b }";
        let (_, compiled) = compile(&[("v", layer)], declarations);
        if admitted {
            compiled.expect("model closure admits reference equality");
        } else {
            let CompileRefusal::Check { refusals, .. } =
                *compiled.expect_err("value layer refuses reference equality")
            else {
                panic!("check-stage refusal")
            };
            assert!(matches!(
                refusals[0].cause,
                CheckCause::LayerEquality {
                    required: HeaderLayer::StateGraph,
                    ..
                }
            ));
        }
    }
}

#[trace("TC-490", "FR-110-AC-10")]
#[test]
fn an_ineligible_equality_keeps_its_operator_refusal() {
    let (_, compiled) = compile(
        &[("v", QUERIES)],
        "predicate f using v(a: Population<M::Node>[1], b: Population<M::Node>[1]): Boolean { a = b }",
    );
    let CompileRefusal::Check { refusals, .. } = *compiled.expect_err("unequal types refuse")
    else {
        panic!("check-stage refusal")
    };
    assert_eq!(refusals.len(), 1);
    assert_eq!(refusals[0].cause.code(), Code::IllTyped);
    assert_eq!(refusals[0].cause.cause(), Some("operator-ineligible"));
    assert!(!matches!(
        refusals[0].cause,
        CheckCause::LayerEquality { .. }
    ));
}

#[trace("TC-490", "FR-110-AC-10")]
#[test]
fn option_presence_and_nested_scalar_equality_remain_core() {
    equality_compile(
        CORE,
        "present(self.optional) and (self.number = self.number)",
    )
    .1
    .expect("presence does not compare the option");
    let (_, compiled) = equality_compile(CORE, "true and (self.optional != self.optional)");
    assert!(matches!(
        *compiled.expect_err("nested option equality refuses"),
        CompileRefusal::Check { .. }
    ));
}

#[trace("TC-490", "FR-110-AC-10")]
#[test]
fn state_graph_predicates_keep_their_selected_equality_layer() {
    for layer in [CORE, QUERIES, GRAPH] {
        let (text, compiled) = model_compile(
            layer,
            "possible P using v over (n: M::Node) { n.pair != n.pair }",
        );
        let Err(refusal) = compiled else {
            panic!("{layer} admitted record equality in a claim")
        };
        assert_eq!(refusal.code(), Code::UnsupportedConstruct);
        assert_eq!(refusal.cause(), Some("expression-form"));
        let message = refusal.to_string();
        assert!(message.contains(layer), "{message}");
        assert!(message.contains(VALUE), "{message}");
        let CompileRefusal::Check {
            region: Some(region),
            ..
        } = *refusal
        else {
            panic!("a located check refusal")
        };
        assert_eq!(
            &text[usize::try_from(region.start()).unwrap()..usize::try_from(region.end()).unwrap()],
            "!="
        );
    }
}
