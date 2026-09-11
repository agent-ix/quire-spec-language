// SPDX-License-Identifier: AGPL-3.0-only
//! TC-114: real native-model artifacts and composed export ownership.

#[path = "support/native_rule_model.rs"]
mod native_rule_model;

use ix_trace_rs::trace;
use quire_contract_ir as ir;
use quire_spec_language::checking::NativeType;
use quire_spec_language::formal_source::FormalSource;
use quire_spec_language::linking::composed::binding_work::{
    Dimension, Limits as BindingLimits, Work,
};
use quire_spec_language::linking::composed::models::{
    bind_models, ImportRefusal, ModelConflictKind, ModelErrorKind, ModelInput, ModelTarget,
};
use quire_spec_language::linking::composed::{
    admit_namespace, ExpectedSource, SourceInventory, SyntaxNamespace, UnitId, WorkLimits,
};
use quire_spec_language::linking::DeclarationKey;
use quire_spec_language::native_model::NativeModel;
use quire_spec_language::syntax::composed::{
    DeclarationKind, Operation, ParameterType, QualifiedName,
};
use quire_spec_language::{ByteDigest, Limits, Source, SourceIdentity, Spanned};

fn model(package: &str, formal: &str, native: &str) -> NativeModel {
    let mut document: serde_json::Value = serde_json::from_str(native_rule_model::FIXTURE).unwrap();
    document["package"] = package.into();
    document_model(document, formal, native)
}

fn document_model(document: serde_json::Value, formal: &str, native: &str) -> NativeModel {
    let text = serde_json::to_string(&document).unwrap();
    let source = Source::read(
        SourceIdentity {
            identity: native.into(),
            revision: "selected".into(),
        },
        format!("{native}.json"),
        text.as_bytes(),
        Limits::default().source_bytes,
    )
    .unwrap();
    native_rule_model::from_source(FormalSource::new(
        source,
        ir::SourceIdentity::new(
            ir::SourceDocumentId::new(formal).unwrap(),
            ir::SourceRevision::new(1).unwrap(),
        ),
    ))
    .unwrap()
    .model()
}

#[test]
#[trace("TC-114", "FR-036-AC-1", "FR-036-AC-3")]
fn enum_members_bind_actual_variant_loci_and_reject_wrong_or_missing_kinds() {
    let mut document: serde_json::Value = serde_json::from_str(native_rule_model::FIXTURE).unwrap();
    document["enums"] = serde_json::json!([{ "name": "Color", "variants": ["Red", "Blue"] }]);
    let model = document_model(document, "EnumModel", "enum-model");
    let inputs = [ModelInput::Native(&model)];
    with_namespace(&[program(&model, "predicate ColorRule using S (color: M::Color): Boolean { M::Color::Red = color }\npredicate Missing using S (): Boolean { M::Color::Green = M::Color::Blue }\npredicate Wrong using S (): Boolean { M::Node::Red = M::Node::Red }")], |namespace| {
        let bindings = bind_models(namespace, &inputs, &mut work());
        let resolved = bindings.resolve_declaration(namespace, namespace.lookup("ColorRule")[0], &mut work()).unwrap();
        assert!(resolved.complete && resolved.refusals.is_empty());
        let variant = resolved.occurrences.iter().find_map(|occurrence| match &occurrence.target {
            ModelTarget::Type(bound) if matches!(bound.location().identity.key, DeclarationKey::Variant { .. }) => Some(bound),
            _ => None,
        }).unwrap();
        assert_eq!(variant.location().identity.key, DeclarationKey::Variant { enumeration: native_rule_model::symbol("Color"), variant: native_rule_model::symbol("Red") });
        let ir::TypeDeclaration::Enum { declaration } = model.environment().types().iter().find(|ty| ty.name().as_str() == "Color").unwrap() else { panic!("actual enumeration export") };
        let red = declaration.variants().iter().find(|variant| variant.name().as_str() == "Red").unwrap();
        assert_eq!(&variant.location().source, red.source());
        for (name, cause) in [("Missing", ModelErrorKind::MissingExport), ("Wrong", ModelErrorKind::WrongExportKind)] {
            let report = bindings.resolve_declaration(namespace, namespace.lookup(name)[0], &mut work()).unwrap();
            assert!(report.complete);
            assert!(report.refusals.iter().all(|error| error.kind == cause));
            assert!(!report.refusals.is_empty());
        }
    });
}

#[test]
#[trace("TC-114", "FR-036-AC-3", "FR-036-AC-4")]
fn a_named_record_cannot_grant_related_instance_authority() {
    let model = model("test/native", "NativeModel", "native-model");
    let inputs = [ModelInput::Native(&model)];
    with_namespace(
        &[program(
            &model,
            r#"protocol Workflow using S over (view: M::Node) on origin {
        role Actor on M::Node;
        relationship Relation = M::Node;
        run sequence Main { check Valid using S { true }; }
        finish Closed as (closed: M::Node) { true };
    }"#,
        )],
        |namespace| {
            let bindings = bind_models(namespace, &inputs, &mut work());
            let id = namespace.lookup("Workflow")[0];
            let report = bindings
                .resolve_declaration(namespace, id, &mut work())
                .unwrap();
            assert!(report.complete);
            assert_eq!(report.refusals.len(), 1);
            let error = &report.refusals[0];
            assert_eq!(error.kind, ModelErrorKind::UnsupportedRelationshipContract);
            let source = namespace.unit(error.unit).unwrap().source();
            assert_eq!(
                source.slice(error.span),
                Some("relationship Relation = M::Node;")
            );
            assert!(report.occurrences.iter().any(|occurrence| matches!(&occurrence.target, ModelTarget::Type(bound) if bound.location().identity.key == DeclarationKey::Type(native_rule_model::symbol("Node")))));
        },
    );
}

fn program(model: &NativeModel, declarations: &str) -> String {
    format!(
        "language \"ix:native\" edition \"1-draft\";\n\
        profile S = \"test:unresolved-definition\" version \"1\" digest \"unresolved\";\n\
        model M = \"{}\" version \"{}\" digest \"{}\";\n{declarations}",
        model.environment().owner().package().as_str(),
        model.environment().owner().revision().get(),
        model.digest()
    )
}

fn with_namespace(texts: &[String], test: impl FnOnce(&SyntaxNamespace)) {
    let sources: Vec<_> = texts
        .iter()
        .enumerate()
        .map(|(index, text)| {
            Source::read(
                SourceIdentity {
                    identity: format!("unit-{index}"),
                    revision: "selected".into(),
                },
                format!("unit-{index}.native"),
                text.as_bytes(),
                Limits::default().source_bytes,
            )
            .unwrap()
        })
        .collect();
    let inventory = SourceInventory {
        language: "ix:native".into(),
        edition: "1-draft".into(),
        units: sources
            .iter()
            .map(|source| ExpectedSource {
                authority: source.identity().identity.clone(),
                identity: source.identity().clone(),
                digest: source.digest(),
            })
            .collect(),
    };
    let report = admit_namespace(
        &inventory,
        &sources,
        WorkLimits::default(),
        Limits::default(),
    );
    assert!(report.issues().is_empty(), "{:?}", report.issues());
    test(report.namespace().unwrap());
}

fn parameter<'a>(
    namespace: &'a SyntaxNamespace,
    declaration: &str,
    index: usize,
) -> (UnitId, &'a QualifiedName) {
    let id = namespace.lookup(declaration)[0];
    let unit = namespace.declaration(id).unwrap().unit();
    let DeclarationKind::Predicate { parameters, .. } = &namespace.syntax(id).unwrap().kind else {
        panic!("predicate fixture")
    };
    let ParameterType::Model(name) = &parameters[index].ty else {
        panic!("qualified model parameter")
    };
    (unit, name)
}

fn work() -> Work {
    Work::new(BindingLimits::default())
}

#[test]
#[trace("TC-114", "FR-036-AC-1", "FR-036-AC-4")]
fn same_alias_in_two_units_keeps_nominal_model_and_source_owners() {
    let first = model("test/first", "FirstModel", "first-model");
    let second = model("test/second", "SecondModel", "second-model");
    let inputs = [ModelInput::Native(&first), ModelInput::Native(&second)];
    with_namespace(
        &[
            program(
                &first,
                "predicate First using S (item: M::Node): Boolean { true }",
            ),
            program(
                &second,
                "predicate Second using S (item: M::Node): Boolean { true }",
            ),
        ],
        |namespace| {
            let bindings = bind_models(namespace, &inputs, &mut work());
            assert!(bindings.complete());
            assert!(bindings.conflicts().is_empty());
            let (left_unit, left_name) = parameter(namespace, "First", 0);
            let (right_unit, right_name) = parameter(namespace, "Second", 0);
            let left = bindings
                .resolve_type(left_unit, left_name, &mut work())
                .unwrap();
            let right = bindings
                .resolve_type(right_unit, right_name, &mut work())
                .unwrap();
            assert!(std::ptr::eq(left.model(), &first));
            assert!(std::ptr::eq(right.model(), &second));
            assert_ne!(left.native(), right.native());
            assert_eq!(left.location().identity.owner, *first.environment().owner());
            assert_eq!(
                right.location().identity.owner,
                *second.environment().owner()
            );
            assert_eq!(left.location().source.source(), first.source().identity());
            assert_eq!(right.location().source.source(), second.source().identity());
        },
    );
}

#[test]
#[trace("TC-114", "FR-036-AC-1", "FR-036-AC-3")]
fn scalar_field_and_operation_exports_reuse_actual_native_roles() {
    let model = model("test/native", "NativeModel", "native-model");
    let inputs = [ModelInput::Native(&model)];
    with_namespace(&[program(&model, "predicate Value using S (item: M::Node, amount: M::Version): Boolean { true }\npost After using S on M::Node::step { result }")], |namespace| {
        let bindings = bind_models(namespace, &inputs, &mut work());
        let (unit, node_name) = parameter(namespace, "Value", 0);
        let (_, scalar_name) = parameter(namespace, "Value", 1);
        let node = bindings.resolve_type(unit, node_name, &mut work()).unwrap();
        let scalar = bindings.resolve_type(unit, scalar_name, &mut work()).unwrap();
        let field_name = Spanned { value: "n".into(), span: node_name.name.span };
        let field = bindings.field(unit, &node, &field_name, &mut work()).unwrap();
        assert_eq!(field.native(), scalar.native());
        assert_eq!(scalar.location().identity.key, DeclarationKey::Scalar(native_rule_model::symbol("Version")));
        assert_eq!(field.location().identity.key, DeclarationKey::Field { record: native_rule_model::symbol("Node"), field: native_rule_model::symbol("n") });
        let id = namespace.lookup("After")[0];
        let DeclarationKind::State { context, operation: Some(name), .. } = &namespace.syntax(id).unwrap().kind else { panic!("postcondition") };
        let bound = bindings.resolve_operation(unit, &Operation { context: context.clone(), name: name.clone() }, &mut work()).unwrap();
        assert!(std::ptr::eq(bound.role(), &model.roles().operations[0]));
        assert_eq!(bound.location().source, model.roles().operations[0].source);
        let result = bindings.value(unit, &bound, &Spanned { value: "step_result".into(), span: name.span }, &mut work()).unwrap();
        assert_eq!(result.native(), &NativeType::Boolean);
        assert_eq!(result.location().identity.key, DeclarationKey::Value(native_rule_model::symbol("step_result")));
    });
}

#[test]
#[trace("TC-114", "FR-036-AC-3")]
fn changed_import_components_refuse_only_the_dependent_model_lookup() {
    let model = model("test/native", "NativeModel", "native-model");
    let inputs = [ModelInput::Native(&model)];
    let good = program(&model, "predicate Dependent using S (item: M::Node): Boolean { true }\npredicate Independent using S (): Boolean { true }");
    for (text, expected) in [
        (
            good.replace("test/native", "test/missing"),
            ImportRefusal::MissingPackage,
        ),
        (
            good.replace(
                "model M = \"test/native\" version \"1\"",
                "model M = \"test/native\" version \"2\"",
            ),
            ImportRefusal::StaleSelection,
        ),
        (
            good.replace(
                &model.digest().to_string(),
                &ByteDigest::of(b"other bytes").to_string(),
            ),
            ImportRefusal::StaleSelection,
        ),
        (
            good.replace(&model.digest().to_string(), "canonical:unsupported"),
            ImportRefusal::InvalidDigest,
        ),
    ] {
        with_namespace(&[text], |namespace| {
            let bindings = bind_models(namespace, &inputs, &mut work());
            assert_eq!(bindings.imports()[0].selection, Err(expected));
            let dependent = bindings
                .resolve_declaration(namespace, namespace.lookup("Dependent")[0], &mut work())
                .unwrap();
            assert!(dependent.complete);
            assert!(matches!(
                dependent.refusals[0].kind,
                ModelErrorKind::RefusedImport { import: 0 }
            ));
            let independent = bindings
                .resolve_declaration(namespace, namespace.lookup("Independent")[0], &mut work())
                .unwrap();
            assert!(independent.complete);
            assert!(independent.refusals.is_empty());
        });
    }
}

#[test]
#[trace("TC-114", "FR-036-AC-3")]
fn conflicting_formal_owner_retains_both_artifacts_and_independent_import() {
    let first = model("test/conflict", "FirstModel", "first-model");
    let changed = model("test/conflict", "ChangedModel", "changed-model");
    let spare = model("test/spare", "SpareModel", "spare-model");
    let inputs = [
        ModelInput::Native(&first),
        ModelInput::Native(&changed),
        ModelInput::Native(&spare),
    ];
    with_namespace(
        &[
            program(
                &first,
                "predicate Conflicted using S (item: M::Node): Boolean { true }",
            ),
            program(
                &spare,
                "predicate Spare using S (item: M::Node): Boolean { true }",
            ),
        ],
        |namespace| {
            let bindings = bind_models(namespace, &inputs, &mut work());
            assert_eq!(bindings.conflicts().len(), 1);
            assert_eq!(bindings.conflicts()[0].kind, ModelConflictKind::Owner);
            assert_eq!(bindings.conflicts()[0].inputs, [0, 1]);
            assert!(
                matches!(&bindings.imports()[0].selection, Err(ImportRefusal::ConflictingModel { groups }) if groups == &[0])
            );
            assert_eq!(bindings.imports()[1].selection, Ok(2));
            let (unit, name) = parameter(namespace, "Spare", 0);
            assert!(std::ptr::eq(
                bindings
                    .resolve_type(unit, name, &mut work())
                    .unwrap()
                    .model(),
                &spare
            ));
        },
    );
}

#[test]
#[trace("TC-114", "FR-036-AC-3")]
fn formal_source_aliasing_refuses_even_when_formal_owners_differ() {
    let first = model("test/first", "SharedSource", "first-model");
    let second = model("test/second", "SharedSource", "second-model");
    let inputs = [ModelInput::Native(&first), ModelInput::Native(&second)];
    with_namespace(
        &[program(
            &first,
            "predicate Rule using S (item: M::Node): Boolean { true }",
        )],
        |namespace| {
            let bindings = bind_models(namespace, &inputs, &mut work());
            assert_eq!(bindings.conflicts()[0].kind, ModelConflictKind::Source);
            assert_eq!(bindings.conflicts()[0].inputs, [0, 1]);
            assert!(matches!(
                bindings.imports()[0].selection,
                Err(ImportRefusal::ConflictingModel { .. })
            ));
        },
    );
}

#[test]
#[trace("TC-114", "FR-036-AC-3")]
fn unsupported_producer_selection_cannot_assert_native_correspondence() {
    let model = model("test/native", "NativeModel", "native-model");
    let inputs = [ModelInput::UnsupportedProducer {
        package: "test/native",
        revision: "1",
        digest: model.digest(),
        interface: "producer/canonical-domain",
    }];
    with_namespace(
        &[program(
            &model,
            "predicate Rule using S (item: M::Node): Boolean { true }",
        )],
        |namespace| {
            let bindings = bind_models(namespace, &inputs, &mut work());
            assert_eq!(
                bindings.imports()[0].selection,
                Err(ImportRefusal::UnsupportedCorrespondence { input: 0 })
            );
            let (unit, name) = parameter(namespace, "Rule", 0);
            assert!(matches!(
                bindings
                    .resolve_type(unit, name, &mut work())
                    .unwrap_err()
                    .kind,
                ModelErrorKind::RefusedImport { import: 0 }
            ));
        },
    );
}

#[test]
#[trace("TC-114", "FR-036-AC-1", "FR-036-AC-3")]
fn duplicate_aliases_and_missing_exports_keep_original_refusal_loci() {
    let model = model("test/native", "NativeModel", "native-model");
    let inputs = [ModelInput::Native(&model)];
    let valid = program(
        &model,
        "predicate Rule using S (item: M::Missing): Boolean { true }",
    );
    with_namespace(std::slice::from_ref(&valid), |namespace| {
        let bindings = bind_models(namespace, &inputs, &mut work());
        let (unit, name) = parameter(namespace, "Rule", 0);
        let error = bindings.resolve_type(unit, name, &mut work()).unwrap_err();
        assert_eq!(error.kind, ModelErrorKind::MissingExport);
        assert_eq!(
            namespace.unit(unit).unwrap().source().slice(error.span),
            Some("Missing")
        );
    });
    let import = valid
        .lines()
        .find(|line| line.starts_with("model "))
        .unwrap();
    let duplicate = valid.replace(import, &format!("{import}\n{import}"));
    with_namespace(&[duplicate], |namespace| {
        let bindings = bind_models(namespace, &inputs, &mut work());
        let (unit, name) = parameter(namespace, "Rule", 0);
        let error = bindings.resolve_type(unit, name, &mut work()).unwrap_err();
        assert_eq!(
            error.kind,
            ModelErrorKind::AmbiguousAlias {
                imports: vec![0, 1]
            }
        );
        assert_eq!(
            namespace.unit(unit).unwrap().source().slice(error.span),
            Some("M")
        );
    });
}

#[test]
#[trace("TC-114", "FR-036-AC-3")]
fn foreign_bound_types_and_reference_carrier_fields_refuse() {
    let first = model("test/first", "FirstModel", "first-model");
    let second = model("test/second", "SecondModel", "second-model");
    let inputs = [ModelInput::Native(&first), ModelInput::Native(&second)];
    with_namespace(
        &[
            program(
                &first,
                "predicate First using S (item: M::Node, reference: M::NodeRef): Boolean { true }",
            ),
            program(
                &second,
                "predicate Second using S (item: M::Node): Boolean { true }",
            ),
        ],
        |namespace| {
            let bindings = bind_models(namespace, &inputs, &mut work());
            let (unit, name) = parameter(namespace, "First", 0);
            let (other, _) = parameter(namespace, "Second", 0);
            let node = bindings.resolve_type(unit, name, &mut work()).unwrap();
            let member = Spanned {
                value: "n".into(),
                span: name.name.span,
            };
            assert_eq!(
                bindings
                    .field(other, &node, &member, &mut work())
                    .unwrap_err()
                    .kind,
                ModelErrorKind::ForeignModel
            );
            let (_, reference) = parameter(namespace, "First", 1);
            let reference = bindings.resolve_type(unit, reference, &mut work()).unwrap();
            assert_eq!(
                bindings
                    .field(
                        unit,
                        &reference,
                        &Spanned {
                            value: "id".into(),
                            span: member.span
                        },
                        &mut work()
                    )
                    .unwrap_err()
                    .kind,
                ModelErrorKind::WrongExportKind
            );
        },
    );
}

#[test]
#[trace("TC-114", "FR-036-AC-7")]
fn exact_model_bytes_entries_and_lookup_limits_preserve_partial_report_and_retry() {
    let model = native_rule_model::parts().model();
    let inputs = [ModelInput::Native(&model)];
    with_namespace(
        &[program(
            &model,
            "predicate Rule using S (item: M::Node): Boolean { true }",
        )],
        |namespace| {
            let import = &namespace.units()[0].models()[0];
            let bytes = model.artifact_bytes().len()
                + [
                    &import.alias.value,
                    &import.package.value,
                    &import.version.value,
                    &import.digest.value,
                ]
                .iter()
                .map(|value| value.len())
                .sum::<usize>();
            // Independent fixture inventory: 2 types + 11 fields + 3 values + 7
            // scalar roles + 9 scalar sites + 1 object + 1 operation + 1 frame field
            // + 1 import. Counts reflect the authored fixture, not runtime usage.
            let bindings_count = 2 + 11 + 3 + 7 + 9 + 1 + 1 + 1 + 1;
            let exact = BindingLimits {
                bytes,
                models: 1,
                bindings: bindings_count,
                references: 1,
                definitions: 0,
                edges: 0,
            };
            let mut exact_work = Work::new(exact);
            let bound = bind_models(namespace, &inputs, &mut exact_work);
            assert!(bound.complete(), "{:?}", bound.exhaustion());
            assert_eq!(exact_work.usage().bytes, bytes);
            assert_eq!(exact_work.usage().bindings, bindings_count);
            for (dimension, limits) in [
                (Dimension::Models, BindingLimits { models: 0, ..exact }),
                (
                    Dimension::Bytes,
                    BindingLimits {
                        bytes: bytes - 1,
                        ..exact
                    },
                ),
                (
                    Dimension::Bindings,
                    BindingLimits {
                        bindings: bindings_count - 1,
                        ..exact
                    },
                ),
                (
                    Dimension::References,
                    BindingLimits {
                        references: 0,
                        ..exact
                    },
                ),
            ] {
                let mut meter = Work::new(limits);
                let partial = bind_models(namespace, &inputs, &mut meter);
                assert!(!partial.complete());
                assert_eq!(partial.exhaustion().unwrap().dimension, dimension);
                let before = meter.usage();
                let retried = bind_models(namespace, &inputs, &mut Work::new(exact));
                assert!(retried.complete());
                assert!(!partial.complete());
                assert_eq!(meter.usage(), before);
            }
            let (unit, name) = parameter(namespace, "Rule", 0);
            let mut low = Work::new(BindingLimits {
                references: 1,
                ..BindingLimits::default()
            });
            assert!(
                matches!(bound.resolve_type(unit, name, &mut low).unwrap_err().kind, ModelErrorKind::ResourceExhausted(error) if error.dimension == Dimension::References)
            );
            assert!(bound.resolve_type(unit, name, &mut work()).is_ok());
        },
    );
}

#[test]
#[trace("TC-114", "FR-036-AC-1", "FR-036-AC-3")]
fn declaration_walker_binds_context_and_input_types_with_located_partial_failures() {
    let model = model("test/native", "NativeModel", "native-model");
    let inputs = [ModelInput::Native(&model)];
    with_namespace(&[program(&model, "predicate Mixed using S (one: M::Node, two: M::Missing, three: M::Version): Boolean { true }\npre Before using S on M::Node::step { true }")], |namespace| {
        let bindings = bind_models(namespace, &inputs, &mut work());
        let mixed = bindings.resolve_declaration(namespace, namespace.lookup("Mixed")[0], &mut work()).unwrap();
        assert!(mixed.complete);
        assert_eq!(mixed.occurrences.len(), 2);
        assert_eq!(mixed.refusals.len(), 1);
        assert_eq!(mixed.refusals[0].kind, ModelErrorKind::MissingExport);
        let before = bindings.resolve_declaration(namespace, namespace.lookup("Before")[0], &mut work()).unwrap();
        assert!(before.complete && before.refusals.is_empty());
        assert!(matches!(before.occurrences[0].target, ModelTarget::Operation(_)));
        let refused = bindings.resolve_declaration(namespace, namespace.lookup("Mixed")[0], &mut Work::new(BindingLimits { references: 0, ..BindingLimits::default() })).unwrap();
        assert!(!refused.complete);
        assert_eq!(refused.exhaustion().unwrap().dimension, Dimension::References);
    });
}

#[test]
#[trace("TC-114", "FR-036-AC-3")]
fn missing_alias_and_unfinished_catalog_retain_distinct_original_causes() {
    let model = model("test/native", "NativeModel", "native-model");
    let inputs = [ModelInput::Native(&model)];
    with_namespace(
        &[program(
            &model,
            "predicate Rule using S (foreign: X::Node, local: M::Node): Boolean { true }",
        )],
        |namespace| {
            let bindings = bind_models(namespace, &inputs, &mut work());
            let (unit, absent) = parameter(namespace, "Rule", 0);
            let error = bindings
                .resolve_type(unit, absent, &mut work())
                .unwrap_err();
            assert_eq!(error.kind, ModelErrorKind::MissingAlias);
            assert_eq!(error.unit, unit);
            assert_eq!(error.span, absent.model.span);
            let (_, present) = parameter(namespace, "Rule", 1);
            assert!(bindings.resolve_type(unit, present, &mut work()).is_ok());

            let incomplete = bind_models(
                namespace,
                &inputs,
                &mut Work::new(BindingLimits {
                    models: 0,
                    ..BindingLimits::default()
                }),
            );
            assert!(!incomplete.complete());
            assert_eq!(
                incomplete.exhaustion().unwrap().dimension,
                Dimension::Models
            );
            let error = incomplete
                .resolve_type(unit, present, &mut work())
                .unwrap_err();
            assert_eq!(error.kind, ModelErrorKind::IncompleteCatalog);
            assert_eq!(error.span, present.model.span);
            assert_eq!(error.unit, unit);
        },
    );
}

#[test]
#[trace("TC-114", "FR-036-AC-3")]
fn scalar_and_record_name_collision_refuses_the_ambiguous_export() {
    let mut document: serde_json::Value = serde_json::from_str(native_rule_model::FIXTURE).unwrap();
    document["records"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!({
            "name": "Version", "fields": [{"name": "valid", "type": {"kind": "boolean"}}]
        }));
    let model = document_model(document, "CollisionModel", "collision-model");
    let inputs = [ModelInput::Native(&model)];
    with_namespace(
        &[program(
            &model,
            "predicate Rule using S (ambiguous: M::Version, valid: M::Node): Boolean { true }",
        )],
        |namespace| {
            let bindings = bind_models(namespace, &inputs, &mut work());
            assert!(bindings.complete());
            let (unit, name) = parameter(namespace, "Rule", 0);
            let error = bindings.resolve_type(unit, name, &mut work()).unwrap_err();
            assert_eq!(error.kind, ModelErrorKind::AmbiguousExport);
            assert_eq!(error.span, name.name.span);
            let (_, valid) = parameter(namespace, "Rule", 1);
            assert!(bindings.resolve_type(unit, valid, &mut work()).is_ok());
        },
    );
}

#[test]
#[trace("TC-114", "FR-036-AC-3", "FR-036-AC-7")]
fn duplicate_exact_models_are_ambiguous_and_conflict_comparisons_are_charged() {
    let model = model("test/native", "NativeModel", "native-model");
    let inputs = [ModelInput::Native(&model), ModelInput::Native(&model)];
    with_namespace(
        &[program(
            &model,
            "predicate Rule using S (item: M::Node): Boolean { true }",
        )],
        |namespace| {
            // Two inputs in each owner/source comparison, then two import candidates.
            let exact = BindingLimits {
                references: 2 + 2 + 2,
                ..BindingLimits::default()
            };
            let mut meter = Work::new(exact);
            let bindings = bind_models(namespace, &inputs, &mut meter);
            assert!(bindings.complete());
            assert!(bindings.conflicts().is_empty());
            assert_eq!(meter.usage().references, 6);
            assert_eq!(
                bindings.imports()[0].selection,
                Err(ImportRefusal::AmbiguousSelection { inputs: vec![0, 1] })
            );
            let (unit, name) = parameter(namespace, "Rule", 0);
            assert_eq!(
                bindings
                    .resolve_type(unit, name, &mut work())
                    .unwrap_err()
                    .kind,
                ModelErrorKind::RefusedImport { import: 0 }
            );
            let partial = bind_models(
                namespace,
                &inputs,
                &mut Work::new(BindingLimits {
                    references: 5,
                    ..exact
                }),
            );
            assert!(!partial.complete());
            assert!(partial.imports().is_empty());
            assert_eq!(partial.exhaustion().unwrap().used, 5);
            assert_eq!(partial.exhaustion().unwrap().requested, 1);
        },
    );
}

#[test]
#[trace("TC-114", "FR-036-AC-3")]
fn native_revision_spelling_refuses_without_reclassifying_a_stale_revision() {
    let model = model("test/native", "NativeModel", "native-model");
    let inputs = [ModelInput::Native(&model)];
    let original = program(
        &model,
        "predicate Rule using S (item: M::Node): Boolean { true }",
    );
    for (revision, expected) in [
        ("1", Ok(0)),
        ("2", Err(ImportRefusal::StaleSelection)),
        ("01", Err(ImportRefusal::InvalidRevision)),
        ("+1", Err(ImportRefusal::InvalidRevision)),
        (" 1", Err(ImportRefusal::InvalidRevision)),
        ("1 ", Err(ImportRefusal::InvalidRevision)),
        ("0", Err(ImportRefusal::InvalidRevision)),
        ("-1", Err(ImportRefusal::InvalidRevision)),
        ("1.0", Err(ImportRefusal::InvalidRevision)),
        ("1e0", Err(ImportRefusal::InvalidRevision)),
        ("18446744073709551616", Err(ImportRefusal::InvalidRevision)),
        ("", Err(ImportRefusal::InvalidRevision)),
    ] {
        let text = original.replace(
            "model M = \"test/native\" version \"1\"",
            &format!("model M = \"test/native\" version \"{revision}\""),
        );
        with_namespace(&[text], |namespace| {
            let bindings = bind_models(namespace, &inputs, &mut work());
            assert!(bindings.complete());
            assert_eq!(bindings.imports().len(), 1);
            assert_eq!(bindings.imports()[0].selection, expected, "{revision:?}");
            assert_eq!(namespace.units()[0].models()[0].version.value, revision);
        });
    }
}

#[test]
#[trace("TC-114", "FR-036-AC-1", "FR-036-AC-7")]
fn parameter_types_preserve_boolean_and_nominal_types_with_exact_scan_costs() {
    let model = model("test/native", "NativeModel", "native-model");
    let inputs = [ModelInput::Native(&model)];
    with_namespace(
        &[program(
            &model,
            "predicate Rule using S (flag: Boolean, amount: M::Version): Boolean { true }",
        )],
        |namespace| {
            let bindings = bind_models(namespace, &inputs, &mut work());
            let id = namespace.lookup("Rule")[0];
            let unit = namespace.declaration(id).unwrap().unit();
            let DeclarationKind::Predicate { parameters, .. } = &namespace.syntax(id).unwrap().kind
            else {
                panic!("predicate fixture")
            };
            let mut boolean = Work::new(BindingLimits {
                references: 1,
                ..BindingLimits::default()
            });
            assert_eq!(
                bindings
                    .parameter_type(unit, &parameters[0].ty, &mut boolean)
                    .unwrap(),
                NativeType::Boolean
            );
            assert_eq!(boolean.usage().references, 1);
            let error = bindings
                .parameter_type(
                    unit,
                    &parameters[0].ty,
                    &mut Work::new(BindingLimits {
                        references: 0,
                        ..BindingLimits::default()
                    }),
                )
                .unwrap_err();
            assert!(
                matches!(error.kind, ModelErrorKind::ResourceExhausted(exhaustion)
                if exhaustion.dimension == Dimension::References && exhaustion.used == 0)
            );
            // Alias/type lookups + both record candidates + Sequence + scalar
            // leaf: Version's first normalized site is Node.items, before Node.n.
            let exact = BindingLimits {
                references: 6,
                ..BindingLimits::default()
            };
            let mut nominal = Work::new(exact);
            let ty = bindings
                .parameter_type(unit, &parameters[1].ty, &mut nominal)
                .unwrap();
            assert!(matches!(ty, NativeType::Scalar { model: owner, role, .. }
                if std::ptr::eq(owner, &model) && role.name.as_str() == "Version"));
            assert_eq!(nominal.usage().references, 6);
            let error = bindings
                .parameter_type(
                    unit,
                    &parameters[1].ty,
                    &mut Work::new(BindingLimits {
                        references: 5,
                        ..exact
                    }),
                )
                .unwrap_err();
            assert!(
                matches!(error.kind, ModelErrorKind::ResourceExhausted(exhaustion)
                if exhaustion.dimension == Dimension::References && exhaustion.used == 5)
            );
        },
    );
}

#[test]
#[trace("TC-114", "FR-036-AC-3", "FR-036-AC-7")]
fn missing_field_scan_exhausts_before_its_last_candidate() {
    let model = model("test/native", "NativeModel", "native-model");
    let inputs = [ModelInput::Native(&model)];
    with_namespace(
        &[program(
            &model,
            "predicate Rule using S (item: M::Node): Boolean { true }",
        )],
        |namespace| {
            let bindings = bind_models(namespace, &inputs, &mut work());
            let (unit, name) = parameter(namespace, "Rule", 0);
            let node = bindings.resolve_type(unit, name, &mut work()).unwrap();
            let missing = Spanned {
                value: "missing".into(),
                span: name.name.span,
            };
            // One catalog entry, one import alias, one lookup, ten Node fields.
            let exact = BindingLimits {
                references: 13,
                ..BindingLimits::default()
            };
            let mut meter = Work::new(exact);
            let error = bindings
                .field(unit, &node, &missing, &mut meter)
                .unwrap_err();
            assert_eq!(error.kind, ModelErrorKind::MissingExport);
            assert_eq!(meter.usage().references, 13);
            let error = bindings
                .field(
                    unit,
                    &node,
                    &missing,
                    &mut Work::new(BindingLimits {
                        references: 12,
                        ..exact
                    }),
                )
                .unwrap_err();
            assert!(
                matches!(error.kind, ModelErrorKind::ResourceExhausted(exhaustion)
            if exhaustion.dimension == Dimension::References && exhaustion.used == 12
                && exhaustion.requested == 1)
            );
        },
    );
}

#[test]
#[trace("TC-114", "FR-036-AC-1", "FR-036-AC-7")]
fn operation_parameters_values_and_enum_members_charge_each_candidate() {
    let mut document: serde_json::Value = serde_json::from_str(native_rule_model::FIXTURE).unwrap();
    document["enums"] = serde_json::json!([{ "name": "Color", "variants": ["Red", "Blue"] }]);
    document["values"].as_array_mut().unwrap().extend([
        serde_json::json!({"name": "alpha", "kind": "input", "type": {"kind": "boolean"}}),
        serde_json::json!({"name": "beta", "kind": "input", "type": {"kind": "boolean"}}),
    ]);
    document["operations"][0]["parameters"] = serde_json::json!(["alpha", "beta"]);
    let model = document_model(document, "ScanModel", "scan-model");
    let inputs = [ModelInput::Native(&model)];
    with_namespace(
        &[program(
            &model,
            "predicate Rule using S (color: M::Color): Boolean { true }\n\
         post After using S on M::Node::step { result }",
        )],
        |namespace| {
            let bindings = bind_models(namespace, &inputs, &mut work());
            let (unit, color) = parameter(namespace, "Rule", 0);
            let missing = Spanned {
                value: "Missing".into(),
                span: color.name.span,
            };
            // Two lookup attempts + two records + one enum + member lookup + two variants.
            let mut variants = Work::new(BindingLimits {
                references: 8,
                ..BindingLimits::default()
            });
            let error = bindings
                .variant(unit, color, &missing, &mut variants)
                .unwrap_err();
            assert_eq!(error.kind, ModelErrorKind::MissingExport);
            assert_eq!(variants.usage().references, 8);
            let error = bindings
                .variant(
                    unit,
                    color,
                    &missing,
                    &mut Work::new(BindingLimits {
                        references: 7,
                        ..BindingLimits::default()
                    }),
                )
                .unwrap_err();
            assert!(
                matches!(error.kind, ModelErrorKind::ResourceExhausted(exhaustion)
            if exhaustion.dimension == Dimension::References && exhaustion.used == 7)
            );

            let id = namespace.lookup("After")[0];
            let DeclarationKind::State {
                context,
                operation: Some(name),
                ..
            } = &namespace.syntax(id).unwrap().kind
            else {
                panic!("postcondition fixture")
            };
            let operation = Operation {
                context: context.clone(),
                name: name.clone(),
            };
            // Alias/type lookup, Node record, Color enum, operation lookup and one role.
            let mut operations = Work::new(BindingLimits {
                references: 6,
                ..BindingLimits::default()
            });
            let bound = bindings
                .resolve_operation(unit, &operation, &mut operations)
                .unwrap();
            assert_eq!(operations.usage().references, 6);
            let error = bindings
                .resolve_operation(
                    unit,
                    &operation,
                    &mut Work::new(BindingLimits {
                        references: 5,
                        ..BindingLimits::default()
                    }),
                )
                .unwrap_err();
            assert!(
                matches!(error.kind, ModelErrorKind::ResourceExhausted(exhaustion)
            if exhaustion.dimension == Dimension::References && exhaustion.used == 5)
            );
            let result = Spanned {
                value: "step_result".into(),
                span: name.span,
            };
            // Catalog + alias + lookup, two parameters, result, five values, Boolean leaf.
            let mut values = Work::new(BindingLimits {
                references: 12,
                ..BindingLimits::default()
            });
            assert_eq!(
                bindings
                    .value(unit, &bound, &result, &mut values)
                    .unwrap()
                    .native(),
                &NativeType::Boolean
            );
            assert_eq!(values.usage().references, 12);
            let error = bindings
                .value(
                    unit,
                    &bound,
                    &result,
                    &mut Work::new(BindingLimits {
                        references: 11,
                        ..BindingLimits::default()
                    }),
                )
                .unwrap_err();
            assert!(
                matches!(error.kind, ModelErrorKind::ResourceExhausted(exhaustion)
            if exhaustion.dimension == Dimension::References && exhaustion.used == 11)
            );
        },
    );
}

#[test]
#[trace("TC-114", "FR-036-AC-3", "FR-036-AC-7")]
fn direct_declaration_refusals_reserve_their_own_binding_records() {
    let model = model("test/native", "NativeModel", "native-model");
    let inputs = [ModelInput::Native(&model)];
    with_namespace(
        &[program(
            &model,
            r#"
        predicate Missing using S (item: M::Absent): Boolean { true }
        protocol Workflow using S over (view: M::Node) on origin {
            role Actor on M::Node;
            relationship Relation = M::Node;
            run sequence Main { check Valid using S { true }; }
            finish Closed as (closed: M::Node) { true };
        }"#,
        )],
        |namespace| {
            let bindings = bind_models(namespace, &inputs, &mut work());
            for (name, exact, before_cause, expected) in [
                // Declaration report + missing-export cause.
                ("Missing", 2, 1, ModelErrorKind::MissingExport),
                // Declaration + input/role/relation/finish types + relationship cause.
                (
                    "Workflow",
                    6,
                    4,
                    ModelErrorKind::UnsupportedRelationshipContract,
                ),
            ] {
                let id = namespace.lookup(name)[0];
                let mut meter = Work::new(BindingLimits {
                    bindings: exact,
                    ..BindingLimits::default()
                });
                let report = bindings
                    .resolve_declaration(namespace, id, &mut meter)
                    .unwrap();
                assert!(report.complete);
                assert_eq!(report.refusals.len(), 1);
                assert_eq!(report.refusals[0].kind, expected);
                assert_eq!(meter.usage().bindings, exact);
                let stopped = bindings
                    .resolve_declaration(
                        namespace,
                        id,
                        &mut Work::new(BindingLimits {
                            bindings: before_cause,
                            ..BindingLimits::default()
                        }),
                    )
                    .unwrap();
                assert!(!stopped.complete);
                assert_eq!(stopped.refusals.len(), 1);
                let exhaustion = stopped.exhaustion().unwrap();
                assert_eq!(exhaustion.dimension, Dimension::Bindings);
                assert_eq!(exhaustion.used, before_cause);
                assert_eq!(exhaustion.requested, 1);
                assert_eq!(exhaustion.limit, before_cause);
            }
        },
    );
}
