// SPDX-License-Identifier: AGPL-3.0-only
//! TC-078/079/081: complete source-ordered clause and operation obligations.

use serde_json::{json, Value};

use super::*;

fn location(model: &NativeModel, key: Value, source: &ir::SourceSpan) -> Value {
    json!({"identity":{"owner":model.environment().owner(),"key":key},"source":source})
}

#[test]
#[trace("TC-078", "TC-079", "TC-081")]
#[trace("FR-019-AC-1", "FR-019-AC-3", "FR-019-AC-4", "FR-019-AC-7")]
fn source_order_preserves_multiple_model_owners_and_complete_invariant_pre_post_records() {
    let mut extra = native_rule_model::parts();
    let old = &extra.environment;
    extra.environment = ir::DeclarationEnvironment::new(
        ir::RequirementRef::parse("example/extra-model", "ExtraModel", 1).unwrap(),
        old.types().to_vec(),
        old.values().to_vec(),
        old.functions().to_vec(),
    )
    .unwrap();
    let models = [native_rule_model::parts().model(), extra.model()];
    let prefix = format!(
        "{HEADER}model Z = \"example/extra-model\" version \"1\" digest \"{}\";\nmodel A = \"example/rule-tests\" version \"1\" digest \"{}\";\n",
        models[1].digest(), models[0].digest(),
    );
    let clauses = [
        "invariant Always on Z::Node at current { true }",
        "pre Before on A::Node::step { true }",
        "post After on Z::Node::step { true }",
    ];
    let bindings = [
        ClauseBinding {
            name: "Always".into(),
            requirement: ir::RequirementRef::parse("example/z-rules", "Invariant", 4).unwrap(),
            clause: ir::ClauseId::new("always").unwrap(),
            execution_point: ir::ExecutionPoint::Initialization {
                name: ir::AnchorName::new("initialize").unwrap(),
            },
        },
        ClauseBinding {
            name: "Before".into(),
            requirement: ir::RequirementRef::parse("example/a-rules", "Precondition", 9).unwrap(),
            clause: ir::ClauseId::new("before").unwrap(),
            execution_point: ir::ExecutionPoint::Pre {
                operation: ir::AnchorName::new("step").unwrap(),
            },
        },
        ClauseBinding {
            name: "After".into(),
            requirement: ir::RequirementRef::parse("example/q-rules", "Postcondition", 2).unwrap(),
            clause: ir::ClauseId::new("after").unwrap(),
            execution_point: ir::ExecutionPoint::Post {
                operation: ir::AnchorName::new("step").unwrap(),
            },
        },
    ];
    let text = format!("{prefix}{}\n", clauses.join("\n"));
    let package = NativePackage::new(
        checked(
            &text,
            "test:multi-owner",
            "1",
            1,
            "multi-owner.native",
            &models,
            vec![
                bindings[2].clone(),
                bindings[0].clone(),
                bindings[1].clone(),
            ],
        ),
        PackageLimits::default(),
    )
    .unwrap();
    let wire: Value = serde_json::from_slice(package.bytes()).unwrap();
    assert_eq!(
        wire["models"],
        json!([
            {"alias":"Z","owner":models[1].environment().owner(),"digest":models[1].digest().to_string(),"artifact":std::str::from_utf8(models[1].artifact_bytes()).unwrap()},
            {"alias":"A","owner":models[0].environment().owner(),"digest":models[0].digest().to_string(),"artifact":std::str::from_utf8(models[0].artifact_bytes()).unwrap()},
        ])
    );
    let mut expected = Vec::new();
    let mut offset = prefix.len();
    for (
        index,
        (model_index, kind, before_context, before_operation, before_expression, observations),
    ) in [
        (
            1,
            "invariant",
            "invariant Always on Z::",
            None,
            "invariant Always on Z::Node at current { ",
            vec!["current"],
        ),
        (
            0,
            "precondition",
            "pre Before on A::",
            Some("pre Before on A::Node::"),
            "pre Before on A::Node::step { ",
            vec!["pre"],
        ),
        (
            1,
            "postcondition",
            "post After on Z::",
            Some("post After on Z::Node::"),
            "post After on Z::Node::step { ",
            vec!["pre", "post"],
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let model = &models[model_index];
        let record = model
            .environment()
            .types()
            .iter()
            .find(|declaration| declaration.name().as_str() == "Node")
            .unwrap();
        let context = location(model, json!({"kind":"type","name":"Node"}), record.source());
        let context_start = offset + before_context.len();
        let mut occurrences = vec![json!({
            "expression":null,"span":{"start":context_start,"end":context_start+4},
            "target":{"kind":"formal","declaration":context}
        })];
        let operation = before_operation.map(|before| {
            let operation = location(model, json!({"kind":"operation","context":"Node","name":"step"}), &model.roles().operations[0].source);
            let start = offset + before.len();
            occurrences.push(json!({"expression":null,"span":{"start":start,"end":start+4},"target":{"kind":"formal","declaration":operation}}));
            operation
        });
        let runtime_operation = before_operation
            .map(|_| json!({"model":model.environment().owner(),"context":"Node","name":"step"}));
        // Frame validation requires both populations even for a constant precondition.
        let universe_observations = if before_operation.is_some() {
            vec!["pre", "post"]
        } else {
            vec!["current"]
        };
        let root_start = offset + before_expression.len();
        let binding = &bindings[index];
        expected.push(json!({
            "name":binding.name,"owner":binding.requirement,"clause":binding.clause,
            "kind":kind,"execution_point":binding.execution_point,
            "span":{"start":offset,"end":offset+clauses[index].len()},"expression":index,
            "context":context,"operation":operation,"occurrences":occurrences,
            "runtime":{
                "context":context,"context_observations":observations,
                "universes":[{"model":model.environment().owner(),"record":"Node","universe":"nodes","observations":universe_observations}],
                "operation":runtime_operation,"validate_frame":before_operation.is_some()
            },
            "projections":[
                {"target":"native-reference/1","status":"available","cost_model":"native-ref-cost/1-draft"},
                {"target":"quire.contract.executable-projection/v1","status":"unlowered","code":"unsupported_construct","span":{"start":root_start,"end":root_start+4}}
            ]
        }));
        assert_eq!(package.checked().clauses()[index].binding(), binding);
        offset += clauses[index].len() + 1;
    }
    assert_eq!(wire["clauses"], json!(expected));
    assert_eq!(
        wire["required_features"],
        json!([
            "boolean",
            "integer",
            "object",
            "option",
            "postcondition",
            "precondition",
            "reference",
            "sequence",
            "text"
        ])
    );
    assert_eq!(package.checked().linked().unit().source().text(), text);
}
