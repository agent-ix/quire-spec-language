// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-079: explicit lexical, capture and transitive-obligation inventories.

use qsl_foundation::Span;
use quire_spec_language::linking::{
    DeclarationIdentity, DeclarationKey, DeclarationLocation, ResolutionTarget,
};
use serde_json::{json, Value};

use super::*;

// This composes known fixture fragments and their expected byte positions.
// It does not parse text or consult production AST/resolution output.
struct OccurrenceFixture {
    text: String,
    linked: Vec<(Option<usize>, Span, ResolutionTarget)>,
    wire: Vec<Value>,
}

impl OccurrenceFixture {
    fn append(&mut self, text: &str) -> Span {
        let start = self.text.len();
        self.text.push_str(text);
        Span {
            start,
            end: self.text.len(),
        }
    }
    fn formal(
        &mut self,
        text: &str,
        expression: Option<usize>,
        declaration: &(DeclarationLocation, Value),
    ) {
        let span = self.append(text);
        self.formal_span(span, expression, declaration);
    }
    fn formal_span(
        &mut self,
        span: Span,
        expression: Option<usize>,
        declaration: &(DeclarationLocation, Value),
    ) {
        self.linked.push((
            expression,
            span,
            ResolutionTarget::Formal(declaration.0.clone()),
        ));
        self.wire.push(
            json!({"expression":expression,"span":{"start":span.start,"end":span.end},
            "target":{"kind":"formal","declaration":declaration.1}}),
        );
    }
    fn local(&mut self, text: &str, expression: usize, binding: Span) {
        let span = self.append(text);
        self.linked
            .push((Some(expression), span, ResolutionTarget::Local(binding)));
        self.wire.push(
            json!({"expression":expression,"span":{"start":span.start,"end":span.end},
            "target":{"kind":"local","binding":{"start":binding.start,"end":binding.end}}}),
        );
    }
}

fn declaration(
    model: &NativeModel,
    key: DeclarationKey,
    wire_key: Value,
    source: &ir::SourceSpan,
) -> (DeclarationLocation, Value) {
    (
        DeclarationLocation {
            identity: DeclarationIdentity {
                owner: model.environment().owner().clone(),
                key,
            },
            source: source.clone(),
        },
        json!({"identity":{"owner":model.environment().owner(),"key":wire_key},"source":source}),
    )
}

fn model(change: impl FnOnce(&mut Value)) -> NativeModel {
    let mut data: Value = serde_json::from_str(native_rule_model::FIXTURE).unwrap();
    change(&mut data);
    native_rule_model::from_text(
        &serde_json::to_string_pretty(&data).unwrap(),
        "package-captures.json",
        "1",
    )
    .unwrap()
    .model()
}

fn operation_binding(post: bool) -> ClauseBinding {
    ClauseBinding {
        execution_point: if post {
            ir::ExecutionPoint::Post {
                operation: ir::AnchorName::new("step").unwrap(),
            }
        } else {
            ir::ExecutionPoint::Pre {
                operation: ir::AnchorName::new("step").unwrap(),
            }
        },
        ..binding()
    }
}

#[test]
#[trace("TC-079", "FR-019-AC-4")]
fn guarded_parent_reads_export_each_original_field_occurrence() {
    let models = [native_rule_model::parts().model()];
    let model = &models[0];
    let ir::TypeDeclaration::Record {
        declaration: record,
    } = &model.environment().types()[0]
    else {
        panic!("the qualified parent fixture has a Node record");
    };
    let context = declaration(
        model,
        DeclarationKey::Type(native_rule_model::symbol("Node")),
        json!({"kind":"type","name":"Node"}),
        record.source(),
    );
    let field = |name: &str| {
        let field = record
            .fields()
            .iter()
            .find(|field| field.name().as_str() == name)
            .unwrap();
        declaration(
            model,
            DeclarationKey::Field {
                record: native_rule_model::symbol("Node"),
                field: native_rule_model::symbol(name),
            },
            json!({"kind":"field","record":"Node","field":name}),
            field.source(),
        )
    };
    let mut fixture = OccurrenceFixture { text: format!("{HEADER}model M = \"example/rule-tests\" version \"1\" digest \"{}\";\ninvariant Rule on M::", model.digest()), linked: Vec::new(), wire: Vec::new() };
    fixture.formal("Node", None, &context);
    fixture.append(" at current { present(");
    fixture.formal("self", Some(0), &context);
    fixture.append(".");
    fixture.formal("parent", Some(1), &field("parent"));
    fixture.append(") implies ");
    let dereference_start = fixture.append("deref(value(").start;
    fixture.formal("self", Some(3), &context);
    fixture.append(".");
    fixture.formal("parent", Some(4), &field("parent"));
    fixture.append("))");
    // Dereferencing contributes its target record after its argument reads.
    fixture.formal_span(
        Span {
            start: dereference_start,
            end: fixture.text.len(),
        },
        Some(6),
        &context,
    );
    fixture.append(".");
    fixture.formal("n", Some(7), &field("n"));
    fixture.append(" < ");
    fixture.formal("self", Some(8), &context);
    fixture.append(".");
    fixture.formal("n", Some(9), &field("n"));
    fixture.append(" }\n");
    let package = NativePackage::new(
        checked(
            &fixture.text,
            "test:parent-package",
            "1",
            1,
            "parent.native",
            &models,
            vec![binding()],
        ),
        PackageLimits::default(),
    )
    .unwrap();
    let wire: Value = serde_json::from_slice(package.bytes()).unwrap();
    assert_eq!(wire["clauses"][0]["expression"], json!(11));
    assert_eq!(wire["clauses"][0]["occurrences"], json!(fixture.wire));
    let linked = package.checked().linked();
    assert_eq!(
        linked.clauses()[0].occurrences().len(),
        fixture.linked.len()
    );
    for (actual, (index, span, target)) in linked.clauses()[0]
        .occurrences()
        .iter()
        .zip(&fixture.linked)
    {
        assert_eq!(actual.span, *span);
        assert_eq!(actual.target, *target);
        assert_eq!(
            actual
                .expression
                .map(|id| linked.unit().expression(id).unwrap()),
            index.map(|index| &linked.unit().expressions()[index])
        );
    }
    assert_eq!(
        wire["clauses"][0]["runtime"],
        json!({"context":context.1,
        "context_observations":["current"],"universes":[{"model":model.environment().owner(),"record":"Node","universe":"nodes","observations":["current"]}],
        "operation":null,"validate_frame":false})
    );
}

#[test]
#[trace("TC-079", "FR-019-AC-3", "FR-019-AC-4")]
fn nested_aliases_and_ordered_unused_parameters_retain_exact_correspondence() {
    let models = [model(|data| {
        data["values"].as_array_mut().unwrap().extend([
            json!({"name":"requested","kind":"input","type":{"kind":"record","name":"NodeRef"}}),
            json!({"name":"second","kind":"input","type":{"kind":"boolean"}}),
        ]);
        data["operations"][0]["parameters"] = json!(["second", "requested"]);
    })];
    let model = &models[0];
    let env = model.environment();
    let context = declaration(
        model,
        DeclarationKey::Type(native_rule_model::symbol("Node")),
        json!({"kind":"type","name":"Node"}),
        env.types()[0].source(),
    );
    let role = &model.roles().operations[0];
    let operation = declaration(
        model,
        DeclarationKey::Operation {
            context: native_rule_model::symbol("Node"),
            name: native_rule_model::symbol("step"),
        },
        json!({"kind":"operation","context":"Node","name":"step"}),
        &role.source,
    );
    let value = |name: &str| {
        let input = env
            .values()
            .iter()
            .find(|value| value.name().as_str() == name)
            .unwrap();
        declaration(
            model,
            DeclarationKey::Value(native_rule_model::symbol(name)),
            json!({"kind":"value","name":name}),
            input.source(),
        )
    };
    for post in [false, true] {
        let mut fixture = OccurrenceFixture {
            text: format!(
                "{HEADER}model M = \"example/rule-tests\" version \"1\" digest \"{}\";\n",
                model.digest()
            ),
            linked: Vec::new(),
            wire: Vec::new(),
        };
        fixture.append(if post {
            "post Rule on M::"
        } else {
            "pre Rule on M::"
        });
        fixture.formal("Node", None, &context);
        fixture.append("::");
        fixture.formal("step", None, &operation);
        fixture.append(" { let ");
        let outer = fixture.append("p");
        fixture.append(" = ");
        fixture.formal("requested", Some(0), &value("requested"));
        fixture.append(" in let ");
        let inner = fixture.append("q");
        fixture.append(" = ");
        fixture.local("p", 1, outer);
        fixture.append(" in ");
        fixture.local("q", 2, inner);
        fixture.append(" = ");
        fixture.local("q", 3, inner);
        if post {
            fixture.append(" and ");
            fixture.formal("result", Some(5), &value("step_result"));
        }
        fixture.append(" }\n");
        let package = NativePackage::new(
            checked(
                &fixture.text,
                "test:captures",
                "1",
                1,
                "captures.native",
                &models,
                vec![operation_binding(post)],
            ),
            PackageLimits::default(),
        )
        .unwrap();
        let wire: Value = serde_json::from_slice(package.bytes()).unwrap();
        let clause = &wire["clauses"][0];
        assert_eq!(clause["occurrences"], json!(fixture.wire));
        let linked = package.checked().linked();
        let occurrences = linked.clauses()[0].occurrences();
        assert_eq!(occurrences.len(), fixture.linked.len());
        for (actual, (index, span, target)) in occurrences.iter().zip(&fixture.linked) {
            assert_eq!(actual.span, *span);
            assert_eq!(actual.target, *target);
            match (actual.expression, index) {
                (None, None) => {}
                (Some(expression), Some(index)) => {
                    assert!(std::ptr::eq(
                        linked.unit().expression(expression).unwrap(),
                        &linked.unit().expressions()[*index]
                    ));
                    assert_eq!(
                        package.checked().clauses()[0].observation(expression),
                        Some(quire_spec_language::checking::Observation::Snapshot(
                            if post && *index == 5 {
                                ir::StateObservation::Post
                            } else {
                                ir::StateObservation::Pre
                            }
                        ))
                    );
                }
                _ => panic!("context/operation and expression occurrence roles must differ"),
            }
        }
        assert_eq!(clause["context"], context.1);
        assert_eq!(clause["operation"], operation.1);
        assert_eq!(clause["expression"], json!(if post { 8 } else { 6 }));
        assert_eq!(
            clause["runtime"],
            json!({"context":context.1,
            "context_observations":if post { vec!["pre", "post"] } else { vec!["pre"] },
            "universes":[{"model":env.owner(),"record":"Node","universe":"nodes","observations":["pre","post"]}],
            "operation":{"model":env.owner(),"context":"Node","name":"step"},"validate_frame":true})
        );
        let retained = package.checked().clauses()[0].runtime_requirements();
        let selected = retained.operation.unwrap();
        assert!(std::ptr::eq(selected, role));
        assert_eq!(
            selected
                .parameters
                .iter()
                .map(|name| name.as_str())
                .collect::<Vec<_>>(),
            ["second", "requested"]
        );
        assert_eq!(selected.result.as_ref().unwrap().as_str(), "step_result");
        assert_eq!(
            serde_json::to_value(&selected.frame).unwrap(),
            json!({"fields":[["Node","n"]],"created":[],"deleted":[]})
        );
        let model_wire: Value =
            serde_json::from_str(wire["models"][0]["artifact"].as_str().unwrap()).unwrap();
        assert_eq!(
            model_wire["roles"]["operations"][0]["parameters"],
            json!(["second", "requested"])
        );
        assert_eq!(
            model_wire["roles"]["operations"][0]["result"],
            json!("step_result")
        );
        assert_eq!(
            wire["models"][0]["artifact"].as_str().unwrap().as_bytes(),
            model.artifact_bytes()
        );
    }
}

#[test]
#[trace("TC-079", "FR-019-AC-3", "FR-019-AC-4")]
fn skipped_nested_context_parameter_and_result_references_retain_transitive_universes() {
    for site in ["context", "parameter", "result"] {
        let models = [model(|data| {
            data["records"].as_array_mut().unwrap().extend([
                json!({"name":"OtherNode","fields":[{"name":"back","type":{"kind":"record","name":"NodeRef"}}]}),
                json!({"name":"OtherRef","fields":[{"name":"id","type":{"kind":"scalar","name":"ObjectId"}}]}),
                json!({"name":"Envelope","fields":[{"name":"target","type":{"kind":"record","name":"OtherRef"}}]}),
            ]);
            data["objects"].as_array_mut().unwrap().push(json!({"record":"OtherNode","reference":"OtherRef","identity_field":"id","universe":"other_nodes"}));
            let nested = json!({"kind":"sequence","maximum":3,"value":{"kind":"option","value":{"kind":"record","name":"Envelope"}}});
            match site {
                "context" => data["records"][0]["fields"]
                    .as_array_mut()
                    .unwrap()
                    .push(json!({"name":"bridge","type":nested})),
                "parameter" => {
                    data["values"]
                        .as_array_mut()
                        .unwrap()
                        .push(json!({"name":"unused","kind":"input","type":nested}));
                    data["operations"][0]["parameters"] = json!(["unused"]);
                }
                "result" => {
                    data["values"]
                        .as_array_mut()
                        .unwrap()
                        .iter_mut()
                        .find(|value| value["name"] == "step_result")
                        .unwrap()["type"] = nested
                }
                _ => unreachable!(),
            }
        })];
        let (clause, bindings, observations, other_observations) = match site {
            "context" => (
                "invariant Rule on M::Node at current { true }",
                binding(),
                vec!["current"],
                vec!["current"],
            ),
            "parameter" => (
                "pre Rule on M::Node::step { true }",
                operation_binding(false),
                vec!["pre", "post"],
                vec!["pre"],
            ),
            "result" => (
                "post Rule on M::Node::step { true }",
                operation_binding(true),
                vec!["pre", "post"],
                vec!["post"],
            ),
            _ => unreachable!(),
        };
        let source = format!(
            "{HEADER}model M = \"example/rule-tests\" version \"1\" digest \"{}\";\n{clause}\n",
            models[0].digest()
        );
        let package = NativePackage::new(
            checked(
                &source,
                "test:closure",
                "1",
                1,
                "closure.native",
                &models,
                vec![bindings],
            ),
            PackageLimits::default(),
        )
        .unwrap();
        let wire: Value = serde_json::from_slice(package.bytes()).unwrap();
        assert_eq!(
            wire["clauses"][0]["runtime"]["universes"],
            json!([
                {"model":models[0].environment().owner(),"record":"Node","universe":"nodes","observations":observations},
                {"model":models[0].environment().owner(),"record":"OtherNode","universe":"other_nodes","observations":other_observations},
            ]),
            "{site}"
        );
        let requirements = package.checked().clauses()[0].runtime_requirements();
        assert_eq!(requirements.universes.len(), 2);
        assert_eq!(requirements.universes[0].object.record.as_str(), "Node");
        assert_eq!(
            requirements.universes[1].object.record.as_str(),
            "OtherNode"
        );
        assert_eq!(requirements.validate_frame, site != "context");
        assert_eq!(
            wire["clauses"][0]["runtime"]["context_observations"],
            json!(match site {
                "context" => vec!["current"],
                "parameter" => vec!["pre"],
                "result" => vec!["pre", "post"],
                _ => unreachable!(),
            })
        );
        assert_eq!(
            wire["models"][0]["artifact"].as_str().unwrap().as_bytes(),
            models[0].artifact_bytes()
        );
    }
}
