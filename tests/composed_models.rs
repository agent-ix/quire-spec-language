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
