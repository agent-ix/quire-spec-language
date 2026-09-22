// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-025: public source frontend, exact provenance and actual native execution.

// Existing helpers construct runtime data and call public compiler APIs.
use crate::support::runtime_setup as setup;

use ix_trace_rs::trace;
use quire_contract_ir as ir;
use quire_spec_language::formal_source::FormalSource;
use quire_spec_language::model_source::{
    read, EntryKind, ModelSourceCause, ModelSourceLimits, FORMAT,
};
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
    let text = include_str!("../fixtures/native-package/model-source.json");
    let source = source(text, "1");
    let draft = read(source.clone(), FORMAT, ModelSourceLimits::default()).unwrap();
    assert_eq!(draft.declared_license, "AGPL-3.0-only");
    assert_eq!(draft.source_limits, ModelSourceLimits::default());
    assert_eq!(draft.source.source().text(), text);
    let model = draft.admit(ModelLimits::default()).unwrap();
    let frozen: Value = serde_json::from_slice(include_bytes!(
        "../fixtures/native-package/minimal.package.json"
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
#[trace("TC-041", "FR-015-AC-2", "TC-102", "FR-025-AC-3")]
fn source_sequence_maxima_are_checked_at_native_admission() {
    for maximum in [10_000, 10_001, u32::MAX] {
        for unused in [false, true] {
            let mut input: Value = serde_json::from_str(setup::native_rule_model::FIXTURE).unwrap();
            if unused {
                input["values"].as_array_mut().unwrap().push(json!({
                    "name":"unused_sequence", "kind":"state",
                    "type":{"kind":"option", "value":{"kind":"sequence", "maximum":1,
                        "value":{"kind":"sequence", "maximum":maximum, "value":{"kind":"boolean"}}}}
                }));
            } else {
                let field = input["records"][0]["fields"]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .find(|field| field["name"] == "items")
                    .unwrap();
                field["type"]["maximum"] = json!(maximum);
            }
            let text = serde_json::to_string(&input).unwrap();
            let binding = source(&text, "sequence-boundary");
            let draft = read(binding.clone(), FORMAT, ModelSourceLimits::default())
                .expect("valid IR declarations must reach the native admission boundary");
            let result = draft.admit(ModelLimits::default());
            if maximum == 10_000 {
                let model = result.unwrap();
                assert_eq!(model.source().source().text(), text);
                assert_eq!(model.source().source().digest(), binding.source().digest());
            } else {
                let error = result.unwrap_err();
                assert_eq!(error.code(), Code::UnrepresentableConstraint);
                assert!(!error.is_incomplete());
                assert_eq!(error.source().source().digest(), binding.source().digest());
                let ModelSourceCause::Admission(cause) = error.cause else {
                    panic!("native admission must own the sequence refusal");
                };
                assert_eq!(cause.diagnostic.code, Code::UnrepresentableConstraint);
                assert_eq!(cause.diagnostic.source, *binding.source().identity());
            }
        }
    }
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
    let effective = ModelSourceLimits {
        source_bytes: usize::MAX,
        entries: usize::MAX,
        type_depth: usize::MAX,
    }
    .bounded();
    assert_eq!(effective.source_bytes, 1_048_576);
    assert_eq!(effective.entries, 10_000);
    assert_eq!(effective.type_depth, 64);
    let overlarge = ModelSourceLimits {
        source_bytes: usize::MAX,
        entries: usize::MAX,
        type_depth: usize::MAX,
    };
    let draft = read(
        source(setup::native_rule_model::FIXTURE, "effective"),
        FORMAT,
        overlarge,
    )
    .unwrap();
    assert_eq!(draft.source_limits, effective);
    let failure = draft
        .admit(ModelLimits {
            roles: 0,
            ..ModelLimits::default()
        })
        .unwrap_err();
    assert_eq!(failure.source_limits(), effective);
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
        assert_eq!(failure.source_limits(), limits.bounded());
        match &failure.cause {
            ModelSourceCause::SourceBytes { actual, maximum } => {
                assert_eq!(limits.source_bytes, 0);
                assert_eq!(*actual, binding.source().text().len());
                assert_eq!(*maximum, 0);
            }
            ModelSourceCause::Entries {
                kind,
                requested,
                remaining,
                maximum,
            } => {
                assert_eq!(limits.entries, 0);
                assert_eq!(*kind, EntryKind::Declaration);
                assert!(*requested > 0);
                assert_eq!((*remaining, *maximum), (0, 0));
            }
            ModelSourceCause::TypeDepth { actual, maximum } => {
                assert_eq!(limits.type_depth, 1);
                assert_eq!((*actual, *maximum), (2, 1));
            }
            cause => panic!("expected the selected resource cause, got {cause:?}"),
        }
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
                matches!(report.outcome(), ExecutionOutcome::ValidationFailed(f) if f.diagnostics.iter().any(|d| d.diagnostic.code == Code::FrameViolation))
            );
        } else {
            assert_eq!(report.truth(), Some(true));
        }
    }
}

#[test]
#[trace("TC-102", "FR-025-AC-3")]
fn duplicate_scalar_retains_the_exact_declared_name() {
    let mut input: Value = serde_json::from_str(setup::native_rule_model::FIXTURE).unwrap();
    let scalar = input["scalars"][0].clone();
    let name = scalar["name"].as_str().unwrap().to_owned();
    input["scalars"].as_array_mut().unwrap().push(scalar);
    let text = input.to_string();
    let failure = read(
        source(&text, "duplicate"),
        FORMAT,
        ModelSourceLimits::default(),
    )
    .unwrap_err();
    assert_eq!(failure.code(), Code::InvalidModelBinding);
    assert_eq!(failure.source().source().text(), text);
    assert!(
        matches!(failure.cause, ModelSourceCause::DuplicateScalar { name: actual } if actual.as_str() == name)
    );
}

#[test]
#[trace("TC-102", "FR-025-AC-3")]
fn unknown_scalar_retains_the_exact_referenced_name() {
    let mut input: Value = serde_json::from_str(setup::native_rule_model::FIXTURE).unwrap();
    input["records"][0]["fields"][0]["type"] = json!({"kind":"scalar","name":"MissingScalar"});
    let text = input.to_string();
    let failure = read(
        source(&text, "unknown"),
        FORMAT,
        ModelSourceLimits::default(),
    )
    .unwrap_err();
    assert_eq!(failure.code(), Code::InvalidModelBinding);
    assert_eq!(failure.source().source().text(), text);
    assert!(
        matches!(failure.cause, ModelSourceCause::UnknownScalar { name } if name.as_str() == "MissingScalar")
    );
}

#[test]
#[trace("TC-102", "FR-025-AC-4")]
fn entry_budgets_precede_value_decoding_and_later_operations() {
    for (kind, pointer, declarations) in [
        (EntryKind::Declaration, "/records", 0),
        (EntryKind::Field, "/records/0/fields", 1),
        (EntryKind::Variant, "/enums/0/variants", 1),
        (EntryKind::Parameter, "/operations/0/parameters", 2),
        (EntryKind::FrameField, "/operations/0/frame/fields", 2),
        (EntryKind::Created, "/operations/0/frame/created", 2),
        (EntryKind::Deleted, "/operations/0/frame/deleted", 2),
    ] {
        let mut input = json!({
            "license":"AGPL-3.0-or-later", "package":"test:model", "requirement":"Model", "revision":1,
            "scalars":[], "records":[], "enums":[], "values":[], "objects":[], "operations":[]
        });
        match kind {
            EntryKind::Declaration => {}
            EntryKind::Field => input["records"] = json!([{"name":"Record", "fields":[]}]),
            EntryKind::Variant => input["enums"] = json!([{"name":"Enum", "variants":[]}]),
            EntryKind::Parameter
            | EntryKind::FrameField
            | EntryKind::Created
            | EntryKind::Deleted => {
                input["operations"] = json!([
                    {"name":"update", "context":"Record", "anchor":"update", "parameters":[],
                     "result":null, "frame":{"fields":[],"created":[],"deleted":[]}},
                    {"malformed_later_operation":true}
                ]);
            }
        }
        // A false value is malformed for every selected group. Its decoder must
        // not run when the budget is exhausted, nor may the later operation run.
        *input.pointer_mut(pointer).unwrap() = json!([false]);
        let text = input.to_string();
        let limits = ModelSourceLimits {
            entries: declarations,
            ..ModelSourceLimits::default()
        };
        let failure = read(source(&text, "budget"), FORMAT, limits).unwrap_err();
        assert_eq!(failure.code(), Code::ResourceExhausted, "{kind:?}");
        assert!(
            matches!(failure.cause,
                ModelSourceCause::Entries { kind: actual, requested: 1, remaining: 0, maximum }
                if actual == kind && maximum == declarations
            ),
            "{kind:?}: {failure:?}"
        );
        // Admit the one entry: the exact same bytes now reach its JSON decoder.
        let failure = read(
            source(&text, "budget-retry"),
            FORMAT,
            ModelSourceLimits {
                entries: declarations + 1,
                ..limits
            },
        )
        .unwrap_err();
        assert!(
            matches!(failure.cause, ModelSourceCause::Decode(_)),
            "{kind:?}: {failure:?}"
        );
    }
}
