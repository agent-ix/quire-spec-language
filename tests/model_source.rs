// SPDX-License-Identifier: AGPL-3.0-only
//! FR-025: public source frontend, exact provenance and actual native execution.

// Existing helpers construct runtime data and call public compiler APIs.
#[allow(dead_code)]
#[path = "support/runtime_setup.rs"]
mod setup;

use ix_trace_rs::trace;
use quire_contract_ir as ir;
use quire_spec_language::formal_source::FormalSource;
use quire_spec_language::model_source::{read, ModelSourceCause, ModelSourceLimits, FORMAT};
use quire_spec_language::native_model::ModelLimits;
use quire_spec_language::package::{NativePackage, PackageLimits};
use quire_spec_language::runtime::{
    execute, ExecutionLimits, ExecutionOutcome, ValueId, ValueNode,
};
use quire_spec_language::syntax::ClauseKind;
use quire_spec_language::{Code, Source, SourceIdentity};
use serde_json::{json, Value};

fn source(text: &str, revision: &str) -> FormalSource {
    FormalSource::new(
        Source::read(
            SourceIdentity {
                identity: "test:rule-model".into(),
                revision: revision.into(),
            },
            "model.json",
            text.as_bytes(),
            1_048_576,
        )
        .unwrap(),
        ir::SourceIdentity::new(
            ir::SourceDocumentId::new("RuleModelSource").unwrap(),
            ir::SourceRevision::new(1).unwrap(),
        ),
    )
}

#[test]
#[trace("TC-101", "FR-025-AC-1")]
fn public_frontend_matches_the_preexisting_frozen_model_artifact() {
    let text = include_str!("fixtures/native-package/model-source.json");
    let source = source(text, "1");
    let draft = read(source.clone(), FORMAT, ModelSourceLimits::default()).unwrap();
    assert_eq!(draft.declared_license, "AGPL-3.0-only");
    assert_eq!(draft.source.source().text(), text);
    let model = draft.admit(ModelLimits::default()).unwrap();
    let frozen: Value = serde_json::from_slice(include_bytes!(
        "fixtures/native-package/minimal.package.json"
    ))
    .unwrap();
    assert_eq!(
        model.artifact_bytes(),
        frozen["models"][0]["artifact"].as_str().unwrap().as_bytes()
    );
    assert_eq!(
        model.digest().to_string(),
        "sha256:639603a87d9701e1279f796639399a2524f3cabe75897e500260d86f8ef74ea8"
    );
    assert_eq!(model.source().source().digest(), source.source().digest());
}

#[test]
#[trace("TC-101", "FR-025-AC-2")]
fn source_forms_and_repeated_names_keep_exact_original_occurrences() {
    let mut input: Value = serde_json::from_str(setup::native_rule_model::FIXTURE).unwrap();
    input["license"] = json!("Declared metadata Ω");
    input["enums"] = json!([{"name":"Status", "variants":["Ready","Paused"]}]);
    input["records"]
        .as_array_mut()
        .unwrap()
        .push(json!({"name":"Extra", "fields":[
            {"name":"id", "type":{"kind":"scalar", "name":"ObjectId"}},
            {"name":"status", "type":{"kind":"enum", "name":"Status"}},
            {"name":"ready", "type":{"kind":"boolean"}}
        ]}));
    input["scalars"][0].as_object_mut().unwrap().remove("unit");
    let text = serde_json::to_string_pretty(&input)
        .unwrap()
        .replace("NodeRef", "Node\\u0052ef");
    let draft = read(
        source(&text, "draft:1"),
        FORMAT,
        ModelSourceLimits::default(),
    )
    .unwrap();
    assert_eq!(draft.declared_license, "Declared metadata Ω");
    let fields: Vec<_> = draft
        .environment
        .types()
        .iter()
        .filter_map(|ty| {
            let ir::TypeDeclaration::Record { declaration } = ty else {
                return None;
            };
            declaration
                .fields()
                .iter()
                .find(|f| f.name().as_str() == "id")
        })
        .collect();
    assert_eq!(fields.len(), 2);
    assert_ne!(fields[0].source(), fields[1].source());
    for field in fields {
        let span = draft.source.to_native(field.source()).unwrap();
        let occurrence: Value =
            serde_json::from_str(draft.source.source().slice(span).unwrap()).unwrap();
        assert_eq!(occurrence["name"], "id");
    }
    let record = draft
        .environment
        .types()
        .iter()
        .find(|ty| ty.name().as_str() == "NodeRef")
        .unwrap();
    let span = draft.source.to_native(record.source()).unwrap();
    assert!(draft
        .source
        .source()
        .slice(span)
        .unwrap()
        .contains("Node\\u0052ef"));
    let model = draft.admit(ModelLimits::default()).unwrap();
    assert_eq!(model.environment().types().len(), 4);
    assert_eq!(model.roles().scalars.len(), 7);
    assert_eq!(
        model.roles().operations[0].frame.fields,
        vec![(setup::symbol("Node"), setup::symbol("n"))]
    );
}

#[test]
#[trace("TC-102", "FR-025-AC-3")]
fn profile_decode_formal_and_admission_failures_retain_the_original_source() {
    let original = setup::native_rule_model::FIXTURE;
    let binding = source(original, "draft:1");
    let failure = read(binding.clone(), "unknown", ModelSourceLimits::default()).unwrap_err();
    assert_eq!(failure.code(), Code::UnknownWire);
    assert_eq!(failure.source().source().text(), original);
    for text in [
        "{broken".to_owned(),
        original.replacen("\"package\":", "\"unknown\":0,\"package\":", 1),
        original.replacen("\"revision\": 1", "\"revision\":1,\"revision\":1", 1),
        original.replace("\"kind\": \"boolean\"", "\"kind\":\"boolean\",\"extra\":0"),
        "[]".to_owned(),
    ] {
        let failure = read(
            source(&text, "draft:1"),
            FORMAT,
            ModelSourceLimits::default(),
        )
        .unwrap_err();
        assert_eq!(failure.source().source().text(), text);
        assert_eq!(failure.code(), Code::InvalidModelBinding);
        assert!(matches!(failure.cause, ModelSourceCause::Decode(_)));
    }
    let mut value: Value = serde_json::from_str(original).unwrap();
    value["scalars"][0]["minimum"] = json!(1001);
    let text = value.to_string();
    let failure = read(
        source(&text, "draft:1"),
        FORMAT,
        ModelSourceLimits::default(),
    )
    .unwrap_err();
    assert!(matches!(failure.cause, ModelSourceCause::Formal(ref errors) if !errors.is_empty()));
    assert_eq!(failure.source().source().text(), text);
    value["scalars"][0]["minimum"] = json!(0);
    value["objects"][0]["identity_field"] = json!("missing");
    let text = value.to_string();
    let failure = read(
        source(&text, "draft:1"),
        FORMAT,
        ModelSourceLimits::default(),
    )
    .unwrap()
    .admit(ModelLimits::default())
    .unwrap_err();
    assert!(matches!(failure.cause, ModelSourceCause::Admission(_)));
    assert_eq!(failure.source().source().text(), text);
    assert_eq!(failure.code(), Code::InvalidModelBinding);
}

#[test]
#[trace("TC-102", "FR-025-AC-4")]
fn lowered_limits_remain_incomplete_and_fresh_requests_succeed() {
    let binding = source(setup::native_rule_model::FIXTURE, "draft:1");
    for limits in [
        ModelSourceLimits {
            source_bytes: 0,
            ..ModelSourceLimits::default()
        },
        ModelSourceLimits {
            entries: 0,
            ..ModelSourceLimits::default()
        },
        ModelSourceLimits {
            type_depth: 1,
            ..ModelSourceLimits::default()
        },
    ] {
        let failure = read(binding.clone(), FORMAT, limits).unwrap_err();
        assert!(failure.is_incomplete());
        assert_eq!(
            failure.source().source().digest(),
            binding.source().digest()
        );
        assert!(read(binding.clone(), FORMAT, ModelSourceLimits::default())
            .unwrap()
            .admit(ModelLimits::default())
            .is_ok());
    }
    let failure = read(binding.clone(), FORMAT, ModelSourceLimits::default())
        .unwrap()
        .admit(ModelLimits {
            roles: 0,
            ..ModelLimits::default()
        })
        .unwrap_err();
    assert!(failure.is_incomplete());
    assert!(matches!(failure.cause, ModelSourceCause::Admission(_)));
    assert_eq!(
        failure.source().source().digest(),
        binding.source().digest()
    );
    let retry = read(binding, FORMAT, ModelSourceLimits::default())
        .unwrap()
        .admit(ModelLimits::default())
        .unwrap();
    assert_eq!(retry.roles().objects[0].record.as_str(), "Node");
}

#[test]
#[trace("TC-101", "FR-025-AC-5")]
fn public_model_source_reaches_real_aggregate_and_operation_execution() {
    let model = read(
        source(setup::native_rule_model::FIXTURE, "draft:1"),
        FORMAT,
        ModelSourceLimits::default(),
    )
    .unwrap()
    .admit(ModelLimits::default())
    .unwrap();
    let models = [model];
    let package = NativePackage::new(
        setup::checked(&models, "forall(item in self.items: item < self.n)"),
        PackageLimits::default(),
    )
    .unwrap();
    for number in [1, 2] {
        let mut draft = setup::draft(&models[0]);
        setup::change_field(&mut draft, "n", ValueNode::Integer { value: number });
        setup::change_field(
            &mut draft,
            "items",
            ValueNode::Sequence {
                values: vec![ValueId::new(0); 3],
            },
        );
        let snapshot = setup::snapshot(draft);
        let selection = setup::selection(&models[0], snapshot.reference());
        assert_eq!(
            execute(
                &package,
                setup::input(snapshot),
                selection,
                ExecutionLimits::default(),
                || false
            )
            .truth(),
            Some(number == 2)
        );
    }
    let package = NativePackage::new(
        setup::checked_kind(
            &models,
            "pre(self.n) = 1 and self.n = 2 and result",
            ClauseKind::Postcondition,
        ),
        PackageLimits::default(),
    )
    .unwrap();
    for bad_frame in [false, true] {
        let before = setup::draft(&models[0]);
        let mut after = before.clone();
        setup::change_field(&mut after, "n", ValueNode::Integer { value: 2 });
        if bad_frame {
            setup::change_field(&mut after, "signed", ValueNode::Integer { value: 1 });
        }
        let (input, selection) = setup::recorded(&models[0], before, after, |_| {});
        let report = execute(
            &package,
            input,
            selection,
            ExecutionLimits::default(),
            || false,
        );
        if bad_frame {
            assert_eq!(report.truth(), None);
            assert!(
                matches!(report.outcome(), ExecutionOutcome::ValidationFailed(f) if f.diagnostics.iter().any(|d| d.code == Code::FrameViolation))
            );
        } else {
            assert_eq!(report.truth(), Some(true));
        }
    }
}
