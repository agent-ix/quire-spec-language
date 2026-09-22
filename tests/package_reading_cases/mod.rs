// SPDX-License-Identifier: AGPL-3.0-or-later
//! Closed wire mutations and independent authority controls.

use super::*;
use qsl_foundation::SourceIdentity;
use quire_contract_ir as ir;
use quire_spec_language::checking::{check, CheckBindings, CheckLimits, ClauseBinding};
use quire_spec_language::formal_source::FormalSource;
use quire_spec_language::package::PackageCause;
use quire_spec_language::{link_native, parse, Limits, LinkLimits};
use serde_json::{json, Value};

mod authority;
mod limits;
mod raw;

fn read<'a>(
    value: &Value,
    bindings: CheckBindings,
    models: &'a [NativeModel],
) -> Result<NativePackage<'a>, Box<quire_spec_language::package::PackageError>> {
    let bytes = serde_json::to_vec(value).unwrap();
    NativePackage::read_verified(
        &bytes,
        NativePackageRef::new(ByteDigest::of(&bytes)),
        bindings,
        models,
        &PackageSupport::default(),
        PackageReadLimits::default(),
    )
}

fn rule<'a>(models: &'a [NativeModel], kind: &str, expression: &str) -> NativePackage<'a> {
    let (context, point) = match kind {
        "invariant" => (
            "M::Node at current",
            ir::ExecutionPoint::Handler {
                name: ir::AnchorName::new("validate").unwrap(),
            },
        ),
        "pre" => (
            "M::Node::step",
            ir::ExecutionPoint::Pre {
                operation: ir::AnchorName::new("step").unwrap(),
            },
        ),
        "post" => (
            "M::Node::step",
            ir::ExecutionPoint::Post {
                operation: ir::AnchorName::new("step").unwrap(),
            },
        ),
        _ => unreachable!("known test clause kind"),
    };
    let text = format!("language \"ix:native\" edition \"0-draft\";\nprofile \"state-finite/0-draft\";\nmodel M = \"example/rule-tests\" version \"1\" digest \"{}\";\n{kind} Rule on {context} {{ {expression} }}\n", models[0].digest());
    let unit = parse(
        SourceIdentity {
            identity: "test:reader-rule".into(),
            revision: "1".into(),
        },
        "reader-rule.native",
        text.as_bytes(),
        Limits::default(),
    )
    .unwrap();
    let source = FormalSource::new(
        unit.source().clone(),
        ir::SourceIdentity::new(
            ir::SourceDocumentId::new("ReaderRule").unwrap(),
            ir::SourceRevision::new(1).unwrap(),
        ),
    );
    let linked = link_native(unit, models, LinkLimits::default()).unwrap();
    let clauses = vec![ClauseBinding {
        name: "Rule".into(),
        requirement: ir::RequirementRef::parse("example/reader", "Rule", 1).unwrap(),
        clause: ir::ClauseId::new("rule").unwrap(),
        execution_point: point,
    }];
    NativePackage::new(
        check(
            linked,
            CheckBindings { source, clauses },
            CheckLimits::default(),
        )
        .unwrap(),
        PackageLimits::default(),
    )
    .unwrap()
}

fn objects(value: &Value, pointer: String, output: &mut Vec<String>) {
    match value {
        Value::Object(object) => {
            output.push(pointer.clone());
            for (key, value) in object {
                objects(
                    value,
                    format!("{pointer}/{}", key.replace('~', "~0").replace('/', "~1")),
                    output,
                );
            }
        }
        Value::Array(array) => {
            for (index, value) in array.iter().enumerate() {
                objects(value, format!("{pointer}/{index}"), output);
            }
        }
        _ => {}
    }
}

#[test]
#[trace("TC-083", "FR-020-AC-2")]
fn every_closed_record_rejects_extra_missing_and_wrong_typed_members() {
    let models = [native_rule_model::parts().model()];
    assert_eq!(
        models[0].roles().objects[0].record,
        native_rule_model::symbol("Node")
    );
    let schema: Value = serde_json::from_str(include_str!(
        "../../schemas/native-linked-package-1.schema.json"
    ))
    .unwrap();
    let schema = jsonschema::JSONSchema::options()
        .with_draft(jsonschema::Draft::Draft202012)
        .compile(&schema)
        .unwrap();
    for (kind, expression) in [
        ("invariant", "let x = self.n in x = self.n"),
        ("pre", "true"),
        ("post", "result"),
    ] {
        let package = rule(&models, kind, expression);
        let original: Value = serde_json::from_slice(package.bytes()).unwrap();
        let bindings = package.checked().bindings().clone();
        assert!(schema.is_valid(&original));
        read(&original, bindings.clone(), &models).unwrap();
        let mut paths = Vec::new();
        objects(&original, String::new(), &mut paths);
        for pointer in paths {
            let record = original.pointer(&pointer).unwrap().as_object().unwrap();
            let mut extra = original.clone();
            extra
                .pointer_mut(&pointer)
                .unwrap()
                .as_object_mut()
                .unwrap()
                .insert("extra".into(), json!(false));
            let error = read(&extra, bindings.clone(), &models).unwrap_err();
            assert_eq!(
                (error.code, error.stage),
                (Code::InvalidPackage, PackageStage::Decode),
                "extra at {pointer}"
            );
            assert!(!schema.is_valid(&extra), "schema extra {pointer}");
            for (key, value) in record {
                let mut missing = original.clone();
                missing
                    .pointer_mut(&pointer)
                    .unwrap()
                    .as_object_mut()
                    .unwrap()
                    .remove(key);
                let error = read(&missing, bindings.clone(), &models).unwrap_err();
                assert_eq!(
                    (error.code, error.stage),
                    (Code::InvalidPackage, PackageStage::Decode),
                    "missing {pointer}/{key}"
                );
                assert!(!schema.is_valid(&missing), "schema missing {pointer}/{key}");
                let mut wrong = original.clone();
                let replacement = match value {
                    Value::String(_) => json!(0),
                    Value::Array(_) => json!({}),
                    Value::Object(_) => json!(false),
                    Value::Number(_) => json!(false),
                    Value::Bool(_) => json!("wrong"),
                    Value::Null => json!({"extra":true}),
                };
                wrong.pointer_mut(&pointer).unwrap()[key] = replacement;
                let error = read(&wrong, bindings.clone(), &models)
                    .err()
                    .unwrap_or_else(|| panic!("accepted wrong primitive at {pointer}/{key}"));
                assert_eq!(
                    (error.code, error.stage),
                    (Code::InvalidPackage, PackageStage::Decode),
                    "type {pointer}/{key}"
                );
                assert!(!schema.is_valid(&wrong), "schema type {pointer}/{key}");
            }
        }
    }
}

#[test]
#[trace("TC-084", "TC-082", "FR-020-AC-3", "FR-020-AC-4", "FR-020-AC-10")]
#[trace("FR-021-AC-2", "FR-021-AC-5")]
fn selection_order_and_feature_sets_do_not_depend_on_object_order() {
    let models = [model()];
    let [vector, _, _] = package_vector_setup::cases();
    let checked = package_vector_setup::checked(&vector, &models);
    let bindings = checked.bindings().clone();
    let original: Value = serde_json::from_slice(vector.artifact).unwrap();
    let selections = [
        ("/semantics/language", Code::UnknownLanguage),
        ("/semantics/edition", Code::UnknownEdition),
        ("/semantics/syntax_profile", Code::UnknownProfile),
        ("/semantics/model_profile", Code::UnknownProfile),
        ("/semantics/checking_contract", Code::UnknownProfile),
        ("/semantics/ir_revision", Code::UnknownProfile),
        ("/semantics/base_definition/revision", Code::UnknownProfile),
        ("/semantics/base_definition/digest", Code::UnknownProfile),
        ("/semantics/rules_definition/revision", Code::UnknownProfile),
        ("/semantics/rules_definition/digest", Code::UnknownProfile),
        ("/canonical_identity/domain", Code::UnknownProfile),
        ("/canonical_identity/version", Code::UnknownProfile),
        ("/canonical_identity/algorithm", Code::UnknownProfile),
    ];
    // Leave every later selector wrong too: the first selected mismatch must
    // win even though serde_json emits these object members in a different order.
    for (index, (pointer, code)) in selections.iter().enumerate() {
        let mut bad = original.clone();
        for (later, _) in &selections[index..] {
            *bad.pointer_mut(later).unwrap() = if later.ends_with("/digest") {
                json!(ByteDigest::of(b"changed definition").to_string())
            } else {
                json!("future")
            };
        }
        let error = read(&bad, bindings.clone(), &models).unwrap_err();
        assert_eq!(error.code, *code, "{pointer}");
        let expected_path: Vec<_> = pointer
            .trim_start_matches('/')
            .split('/')
            .map(|field| quire_spec_language::package::PackagePathSegment::Field(field.into()))
            .collect();
        assert_eq!(error.path, expected_path, "{pointer}");
        assert_eq!(error.stage, PackageStage::Decode);
        assert!(error.usage.derive.is_none());
    }
    let mut changed = original.clone();
    changed["required_features"]
        .as_array_mut()
        .unwrap()
        .reverse();
    let raw = serde_json::to_vec_pretty(&changed).unwrap();
    let package = NativePackage::read_verified(
        &raw,
        NativePackageRef::new(ByteDigest::of(&raw)),
        bindings.clone(),
        &models,
        &PackageSupport::default(),
        PackageReadLimits::default(),
    )
    .unwrap();
    assert_eq!(package.bytes(), raw);
    assert_ne!(package.bytes(), vector.artifact);
    assert_ne!(package.digest(), ByteDigest::of(vector.artifact));
    assert_eq!(
        package.canonical_identity().to_string(),
        vector.digest.trim_end()
    );
    for (feature, expected) in [
        ("boolean", Code::InvalidPackage),
        ("future-feature", Code::UnknownRequiredFeature),
    ] {
        let mut bad = original.clone();
        bad["required_features"]
            .as_array_mut()
            .unwrap()
            .push(json!(feature));
        assert_eq!(
            read(&bad, bindings.clone(), &models).unwrap_err().code,
            expected
        );
    }
    let mut support = PackageSupport::default();
    support.features.remove("boolean");
    let error = NativePackage::read_verified(
        vector.artifact,
        NativePackageRef::new(ByteDigest::of(vector.artifact)),
        bindings,
        &models,
        &support,
        PackageReadLimits::default(),
    )
    .unwrap_err();
    assert_eq!(error.code, Code::UnknownRequiredFeature);
}

#[test]
#[trace("TC-086", "TC-091", "FR-020-AC-7", "FR-020-AC-11")]
#[trace("FR-021-AC-3", "FR-021-AC-4")]
fn validly_shaped_forged_claims_fail_actual_reconstruction_comparison() {
    let models = [native_rule_model::parts().model()];
    let package = rule(&models, "post", "result");
    let original: Value = serde_json::from_slice(package.bytes()).unwrap();
    let bindings = package.checked().bindings().clone();
    for (pointer, value) in [
        ("/clauses/0/expression", json!(99)),
        ("/clauses/0/runtime/validate_frame", json!(false)),
        ("/clauses/0/runtime/operation/name", json!("foreign")),
        (
            "/clauses/0/runtime/context/identity/key/name",
            json!("Foreign"),
        ),
        ("/clauses/0/projections/0/cost_model", json!("future")),
        ("/clauses/0/projections/1/status", json!("available")),
        ("/clauses/0/projections/1/span/end", json!(999)),
        ("/models/0/artifact", json!("forged model")),
        ("/canonical_identity/digest", json!("0".repeat(64))),
    ] {
        assert_ne!(
            original.pointer(pointer).unwrap(),
            &value,
            "mutation changes its selected claim"
        );
        let mut bad = original.clone();
        *bad.pointer_mut(pointer).unwrap() = value;
        let error = read(&bad, bindings.clone(), &models).unwrap_err();
        assert_eq!(
            (error.code, error.stage),
            (Code::InvalidPackage, PackageStage::Compare),
            "{pointer}: {error}"
        );
        assert!(error.usage.encode.is_some());
        assert!(error.usage.compare.is_some());
        assert!(!error.path.is_empty());
    }
    for add in [false, true] {
        let mut bad = original.clone();
        let features = bad["required_features"].as_array_mut().unwrap();
        if add {
            assert!(!features.contains(&json!("reachability")));
            features.push(json!("reachability"));
        } else {
            let index = features.iter().position(|f| f == "boolean").unwrap();
            features.remove(index);
        }
        let error = read(&bad, bindings.clone(), &models).unwrap_err();
        assert_eq!(
            (error.code, error.stage),
            (Code::InvalidPackage, PackageStage::Compare)
        );
    }
}

#[test]
#[trace("TC-087", "TC-088", "FR-020-AC-8", "FR-020-AC-9")]
fn real_frontend_limits_preserve_native_causes_and_retry_state() {
    let models = [native_rule_model::parts().model()];
    let package = rule(
        &models,
        "invariant",
        "let guard = present(self.parent) in guard implies deref(value(self.parent)).n < self.n",
    );
    let bindings = package.checked().bindings().clone();
    let expected_usage = *package.checked().usage();
    macro_rules! lowered {
        ($stage:ident; $($field:ident),+ $(,)?) => {
            [$( {
                let mut limits = PackageReadLimits::default();
                limits.$stage.$field = 0;
                (concat!(stringify!($stage), ".", stringify!($field)), limits)
            } ),+]
        };
    }
    let requests = lowered!(syntax; source_bytes, tokens, nodes, nesting)
        .into_iter()
        .chain(
            lowered!(link; models, imports, clauses, nodes, depth, model_bytes, total_model_bytes),
        )
        .chain(
            lowered!(check; nodes, depth, proof_values, proof_graph_nodes,
            presence_work, materialized_nodes, goal_nodes),
        );
    for (dimension, limits) in requests {
        let error = NativePackage::read_verified(
            package.bytes(),
            package.reference(),
            bindings.clone(),
            &models,
            &PackageSupport::default(),
            limits,
        )
        .unwrap_err();
        assert_eq!(
            (error.code, error.stage),
            (Code::ResourceExhausted, PackageStage::Rebind),
            "{dimension}"
        );
        if dimension == "link.clauses" {
            assert!(
                error.cause.is_none(),
                "external binding count precedes linking"
            );
        } else {
            let code = match error.cause {
                Some(PackageCause::Native(cause)) => cause.code,
                Some(PackageCause::Linking(cause)) => cause.diagnostic.code,
                Some(PackageCause::Checking(cause)) => cause.diagnostic.code,
                _ => panic!("the actual native diagnostic must survive: {dimension}"),
            };
            assert_eq!(code, Code::ResourceExhausted);
        }
        assert!(error.usage.decode.is_some());
        assert!(error.usage.derive.is_none());
        let retry = NativePackage::read_verified(
            package.bytes(),
            package.reference(),
            bindings.clone(),
            &models,
            &PackageSupport::default(),
            PackageReadLimits::default(),
        )
        .unwrap();
        assert_eq!(retry.usage().checking, Some(expected_usage));
    }
}
