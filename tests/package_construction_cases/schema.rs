// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-083: structural Draft 2020-12 checks, separate from compiler authority.

use jsonschema::{Draft, JSONSchema};
use serde_json::{json, Value};

use super::*;

fn schema() -> JSONSchema {
    let schema: Value = serde_json::from_str(include_str!(
        "../../schemas/native-linked-package-1.schema.json"
    ))
    .unwrap();
    JSONSchema::options()
        .with_draft(Draft::Draft202012)
        .compile(&schema)
        .expect("the reviewed local schema must compile as Draft 2020-12")
}

#[test]
#[trace("TC-083", "FR-020-AC-2")]
fn structural_schema_accepts_fixed_and_real_producer_manifests() {
    let schema = schema();
    let models = [vectors::model()];
    for vector in package_vector_setup::cases() {
        let package = NativePackage::new(
            package_vector_setup::checked(&vector, &models),
            PackageLimits::default(),
        )
        .unwrap();
        for bytes in [vector.artifact, package.bytes()] {
            let value: Value = serde_json::from_slice(bytes).unwrap();
            assert!(
                schema.is_valid(&value),
                "{} structural manifest",
                vector.name
            );
        }
    }
    let models = [native_rule_model::parts().model()];
    for (clause, execution_point) in [
        (
            "invariant Rule on M::Node at current { let x = self.n in x = self.n }",
            binding().execution_point,
        ),
        (
            "pre Rule on M::Node::step { true }",
            ir::ExecutionPoint::Pre {
                operation: ir::AnchorName::new("step").unwrap(),
            },
        ),
        (
            "post Rule on M::Node::step { result }",
            ir::ExecutionPoint::Post {
                operation: ir::AnchorName::new("step").unwrap(),
            },
        ),
    ] {
        let source = format!(
            "{HEADER}model M = \"example/rule-tests\" version \"1\" digest \"{}\";\n{clause}\n",
            models[0].digest()
        );
        let package = NativePackage::new(
            checked(
                &source,
                "test:schema",
                "1",
                1,
                "schema.native",
                &models,
                vec![ClauseBinding {
                    execution_point,
                    ..binding()
                }],
            ),
            PackageLimits::default(),
        )
        .unwrap();
        let wire = serde_json::from_slice(package.bytes()).unwrap();
        assert!(schema.is_valid(&wire), "{clause}");
        assert_closed_records(&schema, &wire);
    }
}

#[test]
#[trace("TC-083", "FR-020-AC-2")]
fn structural_schema_rejects_missing_unknown_and_wrong_type_members() {
    let schema = schema();
    let value: Value = serde_json::from_slice(include_bytes!(
        "../fixtures/native-package/multiple.package.json"
    ))
    .unwrap();
    assert!(schema.is_valid(&value), "positive structural precondition");
    assert_closed_records(&schema, &value);
    for (path, replacement) in [
        ("/format", json!("native-linked-package/2")),
        ("/canonical_identity/domain", json!("foreign.domain")),
        ("/canonical_identity/version", json!("v2")),
        ("/canonical_identity/algorithm", json!("sha512")),
        ("/canonical_identity/digest", json!("A".repeat(64))),
        ("/source/formal/revision", json!(0)),
        ("/source/formal/revision", json!(1.5)),
        ("/required_features", json!(["boolean", "future-feature"])),
        ("/required_features", json!(["boolean", "boolean"])),
        ("/clauses/0/runtime/context_observations", json!(["future"])),
        (
            "/clauses/0/runtime/context_observations",
            json!(["current", "current"]),
        ),
    ] {
        let mut changed = value.clone();
        *changed.pointer_mut(path).unwrap() = replacement;
        assert!(!schema.is_valid(&changed), "{path}");
    }
    // Structural validity deliberately cannot establish model correspondence.
    // The verified reader must reject this claim after actual reconstruction.
    let mut wrong_binding = value.clone();
    wrong_binding["models"][0]["artifact"] = json!("opaque but not the selected model");
    assert!(schema.is_valid(&wrong_binding));
}

fn assert_closed_records(schema: &JSONSchema, original: &Value) {
    let mut pending = vec![(String::new(), original)];
    let mut records = 0;
    while let Some((path, value)) = pending.pop() {
        match value {
            Value::Object(members) => {
                records += 1;
                let mut extra = original.clone();
                extra
                    .pointer_mut(&path)
                    .unwrap()
                    .as_object_mut()
                    .unwrap()
                    .insert("unexpected".into(), json!(null));
                assert!(!schema.is_valid(&extra), "unknown member at {path}");
                for (key, child) in members {
                    let mut missing = original.clone();
                    missing
                        .pointer_mut(&path)
                        .unwrap()
                        .as_object_mut()
                        .unwrap()
                        .remove(key);
                    assert!(!schema.is_valid(&missing), "missing {path}/{key}");
                    let mut mistyped = original.clone();
                    mistyped
                        .pointer_mut(&path)
                        .unwrap()
                        .as_object_mut()
                        .unwrap()
                        .insert(
                            key.clone(),
                            if child.is_array() {
                                json!(false)
                            } else {
                                json!([])
                            },
                        );
                    assert!(!schema.is_valid(&mistyped), "wrong type at {path}/{key}");
                    let escaped = key.replace('~', "~0").replace('/', "~1");
                    pending.push((format!("{path}/{escaped}"), child));
                }
            }
            Value::Array(elements) => {
                pending.extend(
                    elements
                        .iter()
                        .enumerate()
                        .map(|(index, child)| (format!("{path}/{index}"), child)),
                );
            }
            Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {}
        }
    }
    assert!(
        records > 0,
        "the admitted package must exercise closed records"
    );
}
