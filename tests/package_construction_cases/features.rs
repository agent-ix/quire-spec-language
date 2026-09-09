// SPDX-License-Identifier: AGPL-3.0-only
//! TC-078/080: complete selected closure and exhaustive native feature families.

use std::collections::BTreeSet;

use quire_spec_language::syntax::ClauseKind;
use serde_json::{json, Value};

use super::*;

const RULE_FEATURES: [&str; 7] = [
    "boolean",
    "integer",
    "object",
    "option",
    "reference",
    "sequence",
    "text",
];

fn changed_model(change: impl FnOnce(&mut Value)) -> NativeModel {
    let mut data = serde_json::from_str(native_rule_model::FIXTURE).unwrap();
    change(&mut data);
    native_rule_model::from_text(
        &serde_json::to_string_pretty(&data).unwrap(),
        "package-features.json",
        "1",
    )
    .expect("real source-derived model variant")
    .model()
}

fn rule<'a>(models: &'a [NativeModel], expression: &str, kind: ClauseKind) -> NativePackage<'a> {
    let mut binding = binding();
    let clause = match kind {
        ClauseKind::Invariant => format!("invariant Rule on M::Node at current {{ {expression} }}"),
        ClauseKind::Precondition => {
            binding.execution_point = ir::ExecutionPoint::Pre {
                operation: ir::AnchorName::new("step").unwrap(),
            };
            format!("pre Rule on M::Node::step {{ {expression} }}")
        }
        ClauseKind::Postcondition => {
            binding.execution_point = ir::ExecutionPoint::Post {
                operation: ir::AnchorName::new("step").unwrap(),
            };
            format!("post Rule on M::Node::step {{ {expression} }}")
        }
    };
    let text = format!(
        "{HEADER}model M = \"example/rule-tests\" version \"1\" digest \"{}\";\n{clause}\n",
        models[0].digest()
    );
    NativePackage::new(
        checked(
            &text,
            "test:features",
            "1",
            1,
            "features.native",
            models,
            vec![binding],
        ),
        PackageLimits::default(),
    )
    .expect("checked feature variant can be packaged")
}

fn assert_features(package: &NativePackage<'_>, additional: &[&str], scenario: &str) {
    let expected = RULE_FEATURES
        .into_iter()
        .chain(additional.iter().copied())
        .collect::<BTreeSet<_>>();
    let wire: Value = serde_json::from_slice(package.bytes()).unwrap();
    assert_eq!(wire["required_features"], json!(expected), "{scenario}");
}

#[test]
#[trace("TC-080", "FR-019-AC-5", "FR-019-AC-6")]
fn every_operator_and_builtin_has_its_exact_declared_feature() {
    let models = [native_rule_model::parts().model()];
    for (expression, added) in [
        ("true", vec![]),
        ("not false", vec!["boolean-control"]),
        ("true and false", vec!["boolean-control"]),
        ("true or false", vec!["boolean-control"]),
        ("false implies true", vec!["boolean-control"]),
        (
            "-self.signed = self.signed",
            vec!["integer-arithmetic", "comparison"],
        ),
        (
            "self.n + 0 = self.n",
            vec!["integer-arithmetic", "comparison"],
        ),
        (
            "self.n - 0 = self.n",
            vec!["integer-arithmetic", "comparison"],
        ),
        (
            "self.n * 1 = self.n",
            vec!["integer-arithmetic", "comparison"],
        ),
        (
            "self.n div 1 = self.n",
            vec!["integer-arithmetic", "comparison"],
        ),
        (
            "self.n rem 1 = self.n",
            vec!["integer-arithmetic", "comparison"],
        ),
        ("self.n = self.n", vec!["comparison"]),
        ("self.n != self.n", vec!["comparison"]),
        ("self.n < self.n", vec!["comparison"]),
        ("self.n <= self.n", vec!["comparison"]),
        ("self.n > self.n", vec!["comparison"]),
        ("self.n >= self.n", vec!["comparison"]),
        ("present(self.parent)", vec![]),
        (
            "present(self.parent) implies value(self.parent) = self.peer",
            vec!["boolean-control", "comparison"],
        ),
        ("deref(self.peer).n = self.n", vec!["comparison"]),
        ("size(self.items) = self.count", vec!["comparison"]),
        ("if true then false else true", vec!["conditional"]),
        ("let p = true in p", vec!["let"]),
        ("forall(n in self.items: true)", vec!["quantification"]),
        ("exists(n in self.items: false)", vec!["quantification"]),
        ("reaches(self, self.peer, parent)", vec!["reachability"]),
        (
            "if true then true else self.n + 1 = self.n",
            vec!["conditional", "integer-arithmetic", "comparison"],
        ),
        (
            "false and self.n div 0 = self.n",
            vec!["boolean-control", "integer-arithmetic", "comparison"],
        ),
        (
            "let n = size(self.items) in n = self.count",
            vec!["let", "comparison"],
        ),
    ] {
        assert_features(
            &rule(&models, expression, ClauseKind::Invariant),
            &added,
            expression,
        );
    }
    assert_features(
        &rule(&models, "true", ClauseKind::Precondition),
        &["precondition"],
        "constant pre",
    );
    assert_features(
        &rule(&models, "result", ClauseKind::Postcondition),
        &["postcondition"],
        "post result",
    );
    assert_features(
        &rule(&models, "pre(self.n) = self.n", ClauseKind::Postcondition),
        &["postcondition", "pre-observation", "comparison"],
        "explicit pre",
    );
}

#[test]
#[trace("TC-078", "TC-080", "FR-019-AC-3", "FR-019-AC-6")]
fn unused_selected_types_and_nested_wrappers_contribute_without_spurious_carrier_features() {
    let original = [native_rule_model::parts().model()];
    let original_package = rule(&original, "true", ClauseKind::Invariant);
    assert_features(
        &original_package,
        &[],
        "object/reference carriers are nominal",
    );
    for (data, added) in [
        (json!({"kind":"enum"}), vec!["enumeration"]),
        (json!({"kind":"record"}), vec!["structural-record"]),
    ] {
        let models = [changed_model(|model| {
            if data["kind"] == "enum" {
                model["enums"] = json!([{"name":"UnusedFlag","variants":["On","Off"]}]);
            } else {
                model["records"].as_array_mut().unwrap().push(json!({
                    "name":"UnusedWrapper","fields":[{"name":"nested","type":{
                        "kind":"option","value":{"kind":"sequence","maximum":3,"value":{
                            "kind":"option","value":{"kind":"boolean"}
                        }}
                    }}]
                }));
            }
        })];
        let package = rule(&models, "true", ClauseKind::Invariant);
        assert_features(&package, &added, "unused selected declaration");
        assert_ne!(
            package.canonical_identity(),
            original_package.canonical_identity()
        );
        let wire: Value = serde_json::from_slice(package.bytes()).unwrap();
        assert_eq!(
            wire["models"][0]["artifact"],
            std::str::from_utf8(models[0].artifact_bytes()).unwrap()
        );
    }
    let models = [changed_model(|model| {
        model["enums"] = json!([{"name":"Flag","variants":["On","Off"]}]);
        model["records"]
            .as_array_mut()
            .unwrap()
            .push(json!({"name":"Pair","fields":[{"name":"flag","type":{"kind":"boolean"}}]}));
        model["values"].as_array_mut().unwrap().extend([
            json!({"name":"pair","kind":"state","type":{"kind":"record","name":"Pair"}}),
            json!({"name":"label","kind":"state","type":{"kind":"scalar","name":"ObjectId"}}),
        ]);
    })];
    for expression in [
        "M::Flag::On != M::Flag::Off",
        "pair = pair",
        "label = \"é🦀\"",
    ] {
        assert_features(
            &rule(&models, expression, ClauseKind::Invariant),
            &["enumeration", "structural-record", "comparison"],
            expression,
        );
    }
}

#[test]
#[trace("TC-078", "TC-080", "FR-019-AC-2", "FR-019-AC-3", "FR-019-AC-6")]
fn repeated_aliases_preserve_source_order_and_unselected_models_stay_out_of_closure() {
    // Two declarations may intentionally share the same exact physical source.
    // Distinct source bytes cannot reuse the fixture's fixed formal identity.
    let mut offered = native_rule_model::parts();
    let old = &offered.environment;
    offered.environment = ir::DeclarationEnvironment::new(
        ir::RequirementRef::parse("example/extra-model", "ExtraModel", 1).unwrap(),
        old.types().to_vec(),
        old.values().to_vec(),
        old.functions().to_vec(),
    )
    .unwrap();
    let models = [native_rule_model::parts().model(), offered.model()];
    let normal = rule(&models, "true", ClauseKind::Invariant);
    assert_features(&normal, &[], "unselected second model");
    let normal_wire: Value = serde_json::from_slice(normal.bytes()).unwrap();
    assert_eq!(normal_wire["models"].as_array().unwrap().len(), 1);
    let text = format!(
        "{HEADER}model Z = \"example/rule-tests\" version \"1\" digest \"{digest}\";\nmodel A = \"example/rule-tests\" version \"1\" digest \"{digest}\";\ninvariant Rule on A::Node at current {{ true }}\n",
        digest = models[0].digest(),
    );
    let package = NativePackage::new(
        checked(
            &text,
            "test:aliases",
            "1",
            1,
            "aliases.native",
            &models,
            vec![binding()],
        ),
        PackageLimits::default(),
    )
    .unwrap();
    assert_features(
        &package,
        &[],
        "two source aliases, one model feature closure",
    );
    let wire: Value = serde_json::from_slice(package.bytes()).unwrap();
    assert_eq!(wire["models"].as_array().unwrap().len(), 2);
    assert_eq!(wire["models"][0]["alias"], "Z");
    assert_eq!(wire["models"][1]["alias"], "A");
    for model in wire["models"].as_array().unwrap() {
        assert_eq!(
            model["owner"],
            json!({"package":"example/rule-tests","requirement":"RuleModel","revision":1})
        );
        assert_eq!(model["digest"], models[0].digest().to_string());
        assert_eq!(
            model["artifact"],
            std::str::from_utf8(models[0].artifact_bytes()).unwrap()
        );
    }
}

#[test]
#[trace("TC-080", "FR-019-AC-6")]
fn finite_recursive_object_families_terminate_with_exact_features() {
    for size in 1..=8 {
        let models = [changed_model(|model| {
            for index in 0..size {
                let record = format!("Cycle{index}");
                let reference = format!("CycleRef{index}");
                let next = format!("CycleRef{}", (index + 1) % size);
                model["records"].as_array_mut().unwrap().extend([
                    json!({"name":record,"fields":[{"name":"next","type":{"kind":"record","name":next}}]}),
                    json!({"name":reference,"fields":[{"name":"id","type":{"kind":"scalar","name":"ObjectId"}}]}),
                ]);
                model["objects"].as_array_mut().unwrap().push(json!({
                    "record":record,"reference":reference,"identity_field":"id","universe":format!("cycle{index}")
                }));
            }
        })];
        let package = rule(&models, "true", ClauseKind::Invariant);
        assert_features(&package, &[], "unused finite object cycle");
        let wire: Value = serde_json::from_slice(package.bytes()).unwrap();
        assert_eq!(
            wire["models"][0]["artifact"],
            std::str::from_utf8(models[0].artifact_bytes()).unwrap()
        );
    }
}
