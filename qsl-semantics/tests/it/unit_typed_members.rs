// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-230 "Unit-typed members": `Quantity<u>` is a type in every position
//! that takes a type, each typed by the unit's own nominal node.

use ix_trace_rs::trace;
use qsl_foundation::diagnostic::Code;
use qsl_semantics::check::{AssemblyCause, CheckCause, CheckedGraph};
use qsl_semantics::model::accounting::ModelNormalizationLimits;
use qsl_semantics::model::intake::admit_unit;
use quire_checked::key::DeclarationKey;
use quire_exact::{IllTypedCause, ValueType};
use serde_json::{json, Value};

use crate::model_operations::{
    admit_and_assemble_with_body, config_unit, config_version_document_with_operations,
    empty_frame, operation, operation_parameter, parse_and_build,
};

const UNITS: &str = "dimension Time;\nunit s : Time = rational(1, 1);\n\
                     unit ms : Time = rational(1, 1000) * s;\n";

/// `ConfigVersion` with the operation `measure(d: ms): ms`, its parameter
/// and result typed `ix://quire/unit/ms`.
fn document() -> Vec<u8> {
    config_version_document_with_operations(vec![operation(
        "measure",
        json!([operation_parameter("measure", "d", "ix://quire/unit/ms")]),
        Some("ix://quire/unit/ms"),
        empty_frame(),
    )])
}

/// Every semantic node's preimage, by its key's hex digest.
fn preimages(graph: &CheckedGraph) -> Vec<(String, Value)> {
    graph
        .semantic_graph()
        .nodes()
        .map(|node| {
            (
                node.key().to_string(),
                serde_json::from_slice(node.preimage()).expect("a JSON preimage"),
            )
        })
        .collect()
}

/// The digest of each reference target under `term`, in order.
fn targets(term: &Value) -> Vec<String> {
    let mut found = Vec::new();
    let mut pending = vec![term];
    while let Some(term) = pending.pop() {
        match term {
            Value::Object(members) => {
                if let Some(Value::String(digest)) =
                    term.get("target").and_then(|t| t.get("digest"))
                {
                    found.push(digest.clone());
                }
                pending.extend(members.values().rev());
            }
            Value::Array(items) => pending.extend(items.iter().rev()),
            _ => {}
        }
    }
    found
}

/// FR-230-AC-15 (TC-685 step 17): with dimension `Time`, root `s` and `ms`
/// of scale `1/1000`, a record field, a tuple element, a union payload, an
/// `Option` and a `Set` of `Quantity<ms>`, a function parameter and result,
/// a predicate parameter of the alias `Span = Quantity<ms>`, and an
/// operation parameter and result typed `ix://quire/unit/ms` each check.
/// Every position is typed by the one nominal unit node of `ms`; the alias
/// has no node, the tuple element and union payload reference the unit node
/// itself, and `Option`'s composite node holds the unit node. `f(3 s)`
/// refuses `ill_typed`/`type-mismatch`, and `record R { d: Quantity<Time> }`
/// `missing_declaration`/`missing-name`.
#[trace("TC-685", "FR-230-AC-15")]
#[test]
fn quantity_types_check_in_every_position_over_the_one_unit_node() {
    let body = format!(
        "{UNITS}record Window {{ d: Quantity<ms>; n: Integer; o: Option<Quantity<ms>>; \
             marks: Set<Quantity<ms>>; }}\n\
         type Span = Quantity<ms>;\n\
         tuple Pair(Quantity<ms>, Integer);\n\
         union Bound {{ Within(Quantity<ms>), Open }}\n\
         function f using v(d: Quantity<ms>): Quantity<ms> pure {{ d }}\n\
         predicate p using v(d: Span): Boolean {{ d = 3 ms }}\n"
    );
    let document = document();
    let graph = admit_and_assemble_with_body(&document, &body)
        .expect("every position assembles")
        .check(quire_semantic_value::checking::CheckingLimits::default())
        .expect("every position checks");
    let nodes = preimages(&graph);
    let unit = nodes
        .iter()
        .find(|(_, preimage)| {
            preimage["version"] == "quire.unit-node/v1"
                && preimage["qualified_declaration"] == json!(["ms"])
        })
        .map(|(key, _)| key.clone())
        .expect("the nominal unit node of ms");
    let declared = |name: &str| {
        nodes
            .iter()
            .find(|(_, preimage)| preimage["declaration"]["qualified_name"] == json!([name]))
            .map(|(_, preimage)| preimage.clone())
    };
    assert!(declared("Span").is_none(), "an alias has no node");
    let window = declared("Window").expect("the Window record");
    let members = window["body"]["members"]
        .as_array()
        .expect("record members");
    assert_eq!(targets(&members[0]), [unit.clone()], "d is typed by ms");
    let option = targets(&members[2]);
    let [option] = option.as_slice() else {
        panic!("o is typed by one composite: {option:?}");
    };
    let (_, option) = nodes
        .iter()
        .find(|(key, _)| key == option)
        .expect("the Option node");
    assert_eq!(option["semantic_form"], "option");
    assert_eq!(targets(&option["body"]), [unit.clone()]);
    for (name, position) in [("Pair", 0), ("Bound", 0)] {
        let node = declared(name).expect("the composite");
        let member = &node["body"]["members"][position];
        assert_eq!(targets(member), [unit.clone()], "{name}");
    }
    for function in ["f", "p"] {
        let node = declared(function).expect("the function");
        let parameter = targets(&node["body"]["members"][0]);
        let (_, parameter) = nodes
            .iter()
            .find(|(key, _)| Some(key) == parameter.first())
            .expect("the parameter node");
        assert_eq!(
            parameter["semantic_type"]["digest"],
            json!(unit),
            "{function}'s parameter is typed by ms: {parameter}"
        );
    }

    // The operation's parameter and result read as `Quantity<ms>`.
    let (unit_text, packages) = config_unit(&document);
    let built = parse_and_build(&unit_text);
    let views = admit_unit(
        &built.selections().models,
        &packages,
        ModelNormalizationLimits::default(),
    )
    .expect("the package admits");
    let owner = views[0].view.type_identities()[&DeclarationKey {
        package: "example/config-version".to_owned(),
        node: "ix://example/config-version/ConfigVersion".to_owned(),
    }];
    let measure = graph
        .scope()
        .operations()
        .declared_by(owner)
        .iter()
        .find(|operation| operation.name() == "measure")
        .expect("measure is declared");
    let [(_, parameter)] = measure.parameters() else {
        panic!("one parameter");
    };
    assert!(matches!(parameter, ValueType::Quantity(_)), "{parameter:?}");
    assert_eq!(measure.result(), Some(parameter));

    // A quantity of another unit of the same dimension does not stand for
    // `Quantity<ms>`.
    let refusals = admit_and_assemble_with_body(
        &document,
        &format!(
            "{UNITS}function f using v(d: Quantity<ms>): Quantity<ms> pure {{ d }}\n\
             function g using v(): Quantity<ms> pure {{ f(3 s) }}\n"
        ),
    )
    .expect("assembles")
    .check(quire_semantic_value::checking::CheckingLimits::default())
    .expect_err("3 s is not a Quantity<ms>");
    assert!(
        refusals
            .iter()
            .all(|refusal| refusal.cause == CheckCause::IllTyped(IllTypedCause::TypeMismatch)),
        "{refusals:?}"
    );

    // `Time` is a dimension, not a unit.
    let refusal = admit_and_assemble_with_body(
        &document,
        &format!("{UNITS}record R {{ d: Quantity<Time>; }}\n"),
    )
    .expect_err("Quantity<Time> names no unit");
    let [error] = refusal.errors.as_slice() else {
        panic!("one error: {:?}", refusal.errors);
    };
    assert_eq!(
        error.cause,
        AssemblyCause::UnresolvedTypeName {
            name: "Time".to_owned()
        }
    );
    assert_eq!(error.cause.code(), Code::MissingDeclaration);
    assert_eq!(
        error.cause.catalog_code().to_string(),
        "missing_declaration/missing-name"
    );
}
