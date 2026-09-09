// SPDX-License-Identifier: AGPL-3.0-only
//! TC-048/052: nominal/operator eligibility and retained runtime obligations.

use super::*;
use quire_spec_language::syntax::ExprKind;
use serde_json::json;

fn comparison_model() -> NativeModel {
    authored_model(|data| {
        data["scalars"].as_array_mut().unwrap().extend([
            json!({"name": "Revision", "kind": "integer", "minimum": 0, "maximum": 1000, "unit": null}),
            json!({"name": "Label", "kind": "text", "max_scalars": 2}),
        ]);
        data["enums"] = json!([
            {"name": "Flag", "variants": ["On", "Off"]},
            {"name": "OtherFlag", "variants": ["On", "Off"]}
        ]);
        data["records"].as_array_mut().unwrap().extend([
            json!({"name": "Pair", "fields": [{"name": "number", "type": {"kind": "scalar", "name": "Version"}}]}),
            json!({"name": "OptionalPair", "fields": [{"name": "number", "type": {"kind": "option", "value": {"kind": "scalar", "name": "Version"}}}]}),
        ]);
        data["values"].as_array_mut().unwrap().extend([
            json!({"name": "revision", "kind": "state", "type": {"kind": "scalar", "name": "Revision"}}),
            json!({"name": "label", "kind": "state", "type": {"kind": "scalar", "name": "Label"}}),
            json!({"name": "pair", "kind": "state", "type": {"kind": "record", "name": "Pair"}}),
            json!({"name": "optional_pair", "kind": "state", "type": {"kind": "record", "name": "OptionalPair"}}),
        ]);
    })
}

#[test]
#[trace("TC-048", "FR-016-AC-4", "FR-016-AC-5")]
fn tc_048_comparison_eligibility_distinguishes_native_nominal_kinds() {
    let models = [comparison_model()];
    for expression in [
        "true = false",
        "M::Flag::On != M::Flag::Off",
        "pair = pair",
        "self = other",
        "self.peer = self.peer",
        "label < \"ab\"",
        "label = \"é界\"",
        "(if true then 2 else revision) = revision",
        "let n = 2 in n = revision",
    ] {
        accepted(&models, expression, ClauseKind::Invariant);
    }
    for expression in [
        "self.n = revision",
        "M::Flag::On = M::OtherFlag::On",
        "M::Flag::On < M::Flag::Off",
        "pair < pair",
        "optional_pair = optional_pair",
        "self = self.peer",
        "self < other",
        "self.peer < self.peer",
        "label = \"é界a\"",
        "label + label = label",
        "let n = 2 in n = revision and n = self.n",
        "if true then self.n = self.n else revision = self.n",
    ] {
        refused(
            &models,
            expression,
            ClauseKind::Invariant,
            Code::IllTyped,
            None,
        );
    }
    let checked = request(
        &models,
        "(if true then 2 else revision) = revision",
        ClauseKind::Invariant,
    )
    .unwrap();
    let unit = checked.linked().unit();
    let ExprKind::Binary { left, .. } = unit.expression(unit.clauses()[0].expression).unwrap().kind
    else {
        panic!("comparison root")
    };
    let Some(NativeType::Scalar { model, role, .. }) = checked.clauses()[0].expression_type(left)
    else {
        panic!("inferred nominal scalar")
    };
    assert_eq!(role.name.as_str(), "Revision");
    assert!(std::ptr::eq(*model, &models[0]));
}

#[test]
#[trace("TC-048", "FR-016-AC-4")]
fn tc_048_size_context_must_cover_every_possible_sequence_length() {
    for (minimum, maximum, unit) in [
        (1, 3, json!(null)),
        (0, 2, json!(null)),
        (0, 3, json!("metre")),
    ] {
        let models = [authored_model(|data| {
            let count = data["scalars"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|s| s["name"] == "Count")
                .unwrap();
            count["minimum"] = json!(minimum);
            count["maximum"] = json!(maximum);
            count["unit"] = unit;
        })];
        refused(
            &models,
            "size(self.items) = self.count",
            ClauseKind::Invariant,
            Code::IllTyped,
            None,
        );
    }
    let models = [native_rule_model::parts().model()];
    accepted(
        &models,
        "let size_value = size(self.items) in size_value = self.count",
        ClauseKind::Invariant,
    );
    refused(
        &models,
        "size(self.n) = self.count",
        ClauseKind::Invariant,
        Code::IllTyped,
        None,
    );
}

#[test]
#[trace("TC-052", "FR-016-AC-9")]
fn tc_052_reachability_requires_one_exact_target_universe_and_observation() {
    let models = [authored_model(|data| {
        data["records"].as_array_mut().unwrap().extend([
            json!({"name": "OtherNode", "fields": [
                {"name": "n", "type": {"kind": "scalar", "name": "Version"}},
                {"name": "back", "type": {"kind": "record", "name": "NodeRef"}}
            ]}),
            json!({"name": "OtherRef", "fields": [{"name": "id", "type": {"kind": "scalar", "name": "ObjectId"}}]}),
            json!({"name": "Envelope", "fields": [{"name": "target", "type": {"kind": "record", "name": "OtherRef"}}]}),
        ]);
        data["objects"].as_array_mut().unwrap().push(json!({
            "record": "OtherNode", "reference": "OtherRef", "identity_field": "id", "universe": "other_nodes"
        }));
        data["values"].as_array_mut().unwrap().push(json!({
            "name": "other_target", "kind": "state", "type": {"kind": "record", "name": "OtherNode"}
        }));
        data["values"].as_array_mut().unwrap().push(json!({
            "name": "envelope", "kind": "state", "type": {"kind": "record", "name": "Envelope"}
        }));
    })];
    for expression in [
        "reaches(self, other_target, parent)",
        "reaches(self, self.n, parent)",
    ] {
        // Both operands link successfully; their exact native types are checked here.
        refused(
            &models,
            expression,
            ClauseKind::Invariant,
            Code::IllTyped,
            None,
        );
    }
    for expression in [
        "reaches(pre(self), pre(self.peer), parent)",
        "let p = pre(self.peer) in reaches(pre(self), pre(p), parent)",
        "let p = self.peer in reaches(self, pre(p), parent)",
        "let p = if result then pre(self.peer) else self.peer in reaches(p, p, parent)",
    ] {
        accepted(&models, expression, ClauseKind::Postcondition);
    }
    for expression in [
        "let p = self.peer in reaches(pre(self), pre(p), parent)",
        "let p = if result then pre(self.peer) else self.peer in reaches(p, self, parent)",
    ] {
        refused(
            &models,
            expression,
            ClauseKind::Postcondition,
            Code::IllTyped,
            None,
        );
    }
    let checked = request(
        &models,
        "pre(self.peer) = self.peer",
        ClauseKind::Postcondition,
    )
    .unwrap();
    let runtime = checked.clauses()[0].runtime_requirements();
    assert_eq!(runtime.universes.len(), 1);
    assert_eq!(runtime.universes[0].object.universe.as_str(), "nodes");
    assert_eq!(
        runtime.universes[0].observations,
        vec![ir::StateObservation::Pre, ir::StateObservation::Post]
    );
    let checked = request(
        &models,
        "pre(envelope) = envelope",
        ClauseKind::Postcondition,
    )
    .unwrap();
    let universe = checked.clauses()[0]
        .runtime_requirements()
        .universes
        .iter()
        .find(|required| required.object.universe.as_str() == "other_nodes")
        .expect("references inside structural records still require their target population");
    assert_eq!(
        universe.observations,
        vec![ir::StateObservation::Pre, ir::StateObservation::Post]
    );
    for (location, kind, observation) in [
        (
            "context",
            ClauseKind::Invariant,
            ir::StateObservation::Current,
        ),
        (
            "parameter",
            ClauseKind::Precondition,
            ir::StateObservation::Pre,
        ),
        (
            "result",
            ClauseKind::Postcondition,
            ir::StateObservation::Post,
        ),
    ] {
        let mut data: serde_json::Value =
            serde_json::from_str(models[0].source().source().text()).unwrap();
        match location {
            "context" => data["records"][0]["fields"].as_array_mut().unwrap().push(json!({
                "name": "bridge", "type": {"kind": "option", "value": {"kind": "record", "name": "OtherRef"}}
            })),
            "parameter" => {
                data["values"].as_array_mut().unwrap().push(json!({
                    "name": "requested", "kind": "input", "type": {"kind": "sequence", "maximum": 3,
                        "value": {"kind": "option", "value": {"kind": "record", "name": "Envelope"}}}
                }));
                data["operations"][0]["parameters"] = json!(["requested"]);
            },
            "result" => {
                let result = data["values"].as_array_mut().unwrap().iter_mut().find(|value| value["name"] == "step_result").unwrap();
                result["type"] = json!({"kind": "record", "name": "OtherRef"});
            },
            _ => unreachable!(),
        }
        let text = serde_json::to_string_pretty(&data).unwrap();
        let required = [
            native_rule_model::from_text(&text, "runtime-inputs.json", "draft:1")
                .unwrap()
                .model(),
        ];
        let checked = request(&required, "true", kind).unwrap();
        let universe = checked.clauses()[0]
            .runtime_requirements()
            .universes
            .iter()
            .find(|required| required.object.universe.as_str() == "other_nodes")
            .expect("constant predicates retain skipped context/invocation obligations");
        assert_eq!(universe.observations, vec![observation], "{location}");
    }
}
