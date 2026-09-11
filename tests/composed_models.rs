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
            // Stored indexes: 3 per record, 2 per field/value/object/operation,
            // one per scalar role/site/frame field, then one import record.
            let bindings_count = 3 * 2 + 2 * 11 + 2 * 3 + 7 + 9 + 2 + 2 + 1 + 1;
            // Borrowed type/value/scalar/operation names + selection + cache lookup.
            let references = 2 + 3 + 7 + 1 + 1 + 1;
            let exact = BindingLimits {
                bytes,
                models: 1,
                bindings: bindings_count,
                references,
                definitions: 0,
                edges: 0,
            };
            let mut exact_work = Work::new(exact);
            let bound = bind_models(namespace, &inputs, &mut exact_work);
            assert!(bound.complete(), "{:?}", bound.exhaustion());
            assert_eq!(exact_work.usage().bytes, bytes);
            assert_eq!(exact_work.usage().bindings, bindings_count);
            assert_eq!(exact_work.usage().references, references);
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
                        references: references - 1,
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
#[trace("TC-114", "FR-036-AC-1", "FR-036-AC-3", "FR-036-AC-7")]
fn multi_model_indexes_reserve_only_selected_inputs_and_share_repeated_imports() {
    let records = (0..400)
        .map(|index| {
            serde_json::json!({
                "name": format!("R{index:04}"),
                "fields": [{"name":"ready", "type":{"kind":"boolean"}}]
            })
        })
        .collect::<Vec<_>>();
    let models = (0..16)
        .map(|index| {
            document_model(
                serde_json::json!({
                    "license":"AGPL-3.0-only", "package":format!("test/supply{index}"),
                    "requirement":"Supply", "revision":1, "scalars":[],
                    "records":records, "values":[], "objects":[], "operations":[]
                }),
                &format!("Supply{index}"),
                &format!("supply-{index}"),
            )
        })
        .collect::<Vec<_>>();
    let inputs = models.iter().map(ModelInput::Native).collect::<Vec<_>>();
    // Eight different models selected by nine imports; the second reuses input 0.
    let selections = [0, 0, 2, 4, 6, 8, 10, 12, 14];
    let sources = selections
        .iter()
        .enumerate()
        .map(|(index, selected)| {
            program(
                &models[*selected],
                &format!("predicate Rule{index} using S (item: M::R0399): Boolean {{ true }}"),
            )
        })
        .collect::<Vec<_>>();
    with_namespace(&sources, |namespace| {
        // Each selected model stores 3 maps per record plus two field entries.
        // The 8 unimported models allocate no export index. Import records cost 9.
        let exact_bindings = 8 * 400 * (3 + 2) + 9;
        // 400 type-name entries per selected model; 16 candidates and one cache
        // lookup per import. The 16 distinct inventory owners/sources do not conflict.
        let exact_references = 8 * 400 + 9 * (16 + 1);
        let exact_bytes = models
            .iter()
            .map(|model| model.artifact_bytes().len())
            .sum::<usize>()
            + namespace
                .units()
                .iter()
                .flat_map(|unit| unit.models())
                .map(|import| {
                    import.alias.value.len()
                        + import.package.value.len()
                        + import.version.value.len()
                        + import.digest.value.len()
                })
                .sum::<usize>();
        let exact = BindingLimits {
            models: 16,
            bindings: exact_bindings,
            references: exact_references,
            bytes: exact_bytes,
            ..BindingLimits::default()
        };
        let mut meter = Work::new(exact);
        let report = bind_models(namespace, &inputs, &mut meter);
        assert!(report.complete(), "{:?}", report.exhaustion());
        assert!(report.conflicts().is_empty());
        assert_eq!(meter.usage().models, 16);
        assert_eq!(meter.usage().bytes, exact_bytes);
        assert_eq!(meter.usage().bindings, exact_bindings);
        assert_eq!(meter.usage().references, exact_references);
        assert_eq!(report.imports().len(), 9);
        for (index, selected) in selections.into_iter().enumerate() {
            assert_eq!(report.imports()[index].selection, Ok(selected));
            let (unit, name) = parameter(namespace, &format!("Rule{index}"), 0);
            let bound = report.resolve_type(unit, name, &mut work()).unwrap();
            assert!(std::ptr::eq(bound.model(), &models[selected]));
            assert_eq!(
                bound.location().identity.key,
                DeclarationKey::Type(native_rule_model::symbol("R0399"))
            );
        }
        let mut low = Work::new(BindingLimits {
            bindings: exact_bindings - 1,
            ..exact
        });
        let partial = bind_models(namespace, &inputs, &mut low);
        assert!(!partial.complete());
        let exhausted = partial.exhaustion().unwrap();
        assert_eq!(exhausted.dimension, Dimension::Bindings);
        assert_eq!(
            (exhausted.used, exhausted.requested),
            (exact_bindings - 1, 1)
        );
        // Selection is retained, but its unfinished catalog cannot supply a type.
        assert_eq!(partial.imports().len(), 9);
        assert_eq!(partial.imports()[8].selection, Ok(14));
        let (unit, name) = parameter(namespace, "Rule8", 0);
        assert_eq!(
            partial
                .resolve_type(unit, name, &mut work())
                .unwrap_err()
                .kind,
            ModelErrorKind::IncompleteCatalog
        );
        let before = low.usage();
        assert!(bind_models(namespace, &inputs, &mut Work::new(exact)).complete());
        assert_eq!(low.usage(), before);
        assert!(!partial.complete());
        let zero = bind_models(
            namespace,
            &inputs,
            &mut Work::new(BindingLimits {
                bindings: 0,
                ..exact
            }),
        );
        assert!(!zero.complete());
        assert_eq!(zero.exhaustion().unwrap().dimension, Dimension::Bindings);
        assert_eq!(zero.exhaustion().unwrap().used, 0);
    });
}

#[test]
#[trace("TC-114", "FR-036-AC-3", "FR-036-AC-7")]
fn unselected_model_conflicts_remain_visible_without_allocating_their_indexes() {
    let first = model("test/conflict", "Shared", "first");
    let second = model("test/conflict", "Shared", "second");
    let spare = model("test/spare", "Spare", "spare");
    let inputs = [
        ModelInput::Native(&first),
        ModelInput::Native(&second),
        ModelInput::Native(&spare),
    ];
    with_namespace(
        &[program(
            &spare,
            "predicate Rule using S (item: M::Node): Boolean { true }",
        )],
        |namespace| {
            // One selected fixture's 55 index entries, one import, two shared conflicts.
            let mut meter = Work::new(BindingLimits {
                bindings: 58,
                ..BindingLimits::default()
            });
            let report = bind_models(namespace, &inputs, &mut meter);
            assert!(report.complete(), "{:?}", report.exhaustion());
            assert_eq!(meter.usage().models, 3);
            assert_eq!(meter.usage().bindings, 58);
            assert_eq!(report.conflicts().len(), 2);
            assert_eq!(report.conflicts()[0].kind, ModelConflictKind::Owner);
            assert_eq!(report.conflicts()[1].kind, ModelConflictKind::Source);
            assert_eq!(report.conflicts()[0].inputs, [0, 1]);
            assert_eq!(report.conflicts()[1].inputs, [0, 1]);
            assert_eq!(report.imports()[0].selection, Ok(2));
            let (unit, name) = parameter(namespace, "Rule", 0);
            assert!(std::ptr::eq(
                report
                    .resolve_type(unit, name, &mut work())
                    .unwrap()
                    .model(),
                &spare
            ));
            let partial = bind_models(
                namespace,
                &inputs,
                &mut Work::new(BindingLimits {
                    bytes: first.artifact_bytes().len() + second.artifact_bytes().len() - 1,
                    ..BindingLimits::default()
                }),
            );
            assert!(!partial.complete());
            assert_eq!(partial.exhaustion().unwrap().dimension, Dimension::Bytes);
            assert!(partial.imports().is_empty());
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
            // Two owner comparisons, two source comparisons, two candidates.
            // Ambiguous selections construct no export indexes.
            let exact = BindingLimits {
                references: 2 + 2 + 2,
                bindings: 1,
                ..BindingLimits::default()
            };
            let mut meter = Work::new(exact);
            let bindings = bind_models(namespace, &inputs, &mut meter);
            assert!(bindings.complete());
            assert!(bindings.conflicts().is_empty());
            assert_eq!(meter.usage().references, 6);
            assert_eq!(meter.usage().bindings, 1);
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
            // Indexed alias/type lookups + Sequence + scalar
            // leaf: Version's first normalized site is Node.items, before Node.n.
            let exact = BindingLimits {
                references: 4,
                ..BindingLimits::default()
            };
            let mut nominal = Work::new(exact);
            let ty = bindings
                .parameter_type(unit, &parameters[1].ty, &mut nominal)
                .unwrap();
            assert!(matches!(ty, NativeType::Scalar { model: owner, role, .. }
                if std::ptr::eq(owner, &model) && role.name.as_str() == "Version"));
            assert_eq!(nominal.usage().references, 4);
            let error = bindings
                .parameter_type(
                    unit,
                    &parameters[1].ty,
                    &mut Work::new(BindingLimits {
                        references: 3,
                        ..exact
                    }),
                )
                .unwrap_err();
            assert!(
                matches!(error.kind, ModelErrorKind::ResourceExhausted(exhaustion)
                if exhaustion.dimension == Dimension::References && exhaustion.used == 3)
            );
        },
    );
}

#[test]
#[trace("TC-114", "FR-036-AC-3", "FR-036-AC-7")]
fn missing_field_binary_search_exhausts_before_its_last_comparison() {
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
            // Catalog + alias + lookup; sorted candidates n, distance, items.
            let exact = BindingLimits {
                references: 6,
                ..BindingLimits::default()
            };
            let mut meter = Work::new(exact);
            let error = bindings
                .field(unit, &node, &missing, &mut meter)
                .unwrap_err();
            assert_eq!(error.kind, ModelErrorKind::MissingExport);
            assert_eq!(meter.usage().references, 6);
            let error = bindings
                .field(
                    unit,
                    &node,
                    &missing,
                    &mut Work::new(BindingLimits {
                        references: 5,
                        ..exact
                    }),
                )
                .unwrap_err();
            assert!(
                matches!(error.kind, ModelErrorKind::ResourceExhausted(exhaustion)
            if exhaustion.dimension == Dimension::References && exhaustion.used == 5
                && exhaustion.requested == 1)
            );
        },
    );
}

#[test]
#[trace("TC-114", "FR-036-AC-1", "FR-036-AC-3", "FR-036-AC-7")]
fn wide_record_fields_resolve_at_defaults_without_repeated_linear_scans() {
    let fields = (0..1_500)
        .map(|index| {
            serde_json::json!({
                "name":format!("f{index:04}"), "type":{"kind":"boolean"}
            })
        })
        .collect::<Vec<_>>();
    let model = document_model(
        serde_json::json!({
            "license":"AGPL-3.0-only", "package":"test/wide", "requirement":"WideModel",
            "revision":1, "scalars":[], "values":[], "objects":[], "operations":[],
            "records":[{"name":"Wide","fields":fields}, {"name":"Empty","fields":[]}]
        }),
        "WideModel",
        "wide-model",
    );
    let input = [ModelInput::Native(&model)];
    let body = (0..1_400)
        .map(|index| match index {
            0 => "item.f0000",
            1 => "item.f0749",
            _ => "item.f1499",
        })
        .collect::<Vec<_>>()
        .join(" and ");
    with_namespace(
        &[program(
            &model,
            &format!(
                "predicate Rule using S (item: M::Wide, empty: M::Empty): Boolean {{ {body} }}"
            ),
        )],
        |namespace| {
            let bindings = bind_models(namespace, &input, &mut work());
            assert!(bindings.complete(), "{:?}", bindings.exhaustion());
            let (unit, wide) = parameter(namespace, "Rule", 0);
            let receiver = bindings.resolve_type(unit, wide, &mut work()).unwrap();
            let fields = namespace
                .unit(unit)
                .unwrap()
                .expressions()
                .iter()
                .filter_map(|node| match &node.kind {
                    quire_spec_language::syntax::composed::ValueKind::Shared(
                        quire_spec_language::syntax::ExprKind::Field { name, .. },
                    ) => Some(name),
                    _ => None,
                })
                .collect::<Vec<_>>();
            assert_eq!(fields.len(), 1_400);
            assert_eq!(fields[0].value, "f0000");
            assert_eq!(fields[1].value, "f0749");
            assert_eq!(fields[1_399].value, "f1499");
            let mut meter = work();
            for field in &fields {
                let bound = bindings.field(unit, &receiver, field, &mut meter).unwrap();
                assert_eq!(bound.native(), &NativeType::Boolean);
                assert!(std::ptr::eq(bound.model(), &model));
                assert_eq!(
                    bound.location().identity.key,
                    DeclarationKey::Field {
                        record: native_rule_model::symbol("Wide"),
                        field: native_rule_model::symbol(&field.value),
                    }
                );
            }
            // The old scans need 1 + 750 + 1398*1500 = 2,097,751 comparisons,
            // exceeding the hard 2M References cap before all offered fields resolve.
            assert!(meter.usage().references < 30_000, "{:?}", meter.usage());

            let (_, empty) = parameter(namespace, "Rule", 1);
            let empty = bindings.resolve_type(unit, empty, &mut work()).unwrap();
            let mut empty_work = Work::new(BindingLimits {
                references: 3,
                ..BindingLimits::default()
            });
            let error = bindings
                .field(unit, &empty, fields[0], &mut empty_work)
                .unwrap_err();
            assert_eq!(error.kind, ModelErrorKind::MissingExport);
            assert_eq!(empty_work.usage().references, 3); // catalog, alias, lookup; no fields
            assert_eq!(error.span, fields[0].span);
        },
    );
}

#[test]
#[trace("TC-114", "FR-036-AC-1", "FR-036-AC-7")]
fn indexed_operations_and_values_charge_only_selected_parameters_and_members() {
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
            let mut catalog_work = work();
            let bindings = bind_models(namespace, &inputs, &mut catalog_work);
            // Baseline indexes 55 + enum maps/members 5 + two value-map pairs 4
            // + import 1. The operation's two borrowed parameters add no index.
            assert_eq!(catalog_work.usage().bindings, 65);
            assert_eq!(catalog_work.usage().references, 3 + 5 + 7 + 1 + 1 + 1);
            let (unit, color) = parameter(namespace, "Rule", 0);
            let missing = Spanned {
                value: "Missing".into(),
                span: color.name.span,
            };
            // Indexed alias/type lookup + member lookup + two variants.
            let mut variants = Work::new(BindingLimits {
                references: 5,
                ..BindingLimits::default()
            });
            let error = bindings
                .variant(unit, color, &missing, &mut variants)
                .unwrap_err();
            assert_eq!(error.kind, ModelErrorKind::MissingExport);
            assert_eq!(variants.usage().references, 5);
            let error = bindings
                .variant(
                    unit,
                    color,
                    &missing,
                    &mut Work::new(BindingLimits {
                        references: 4,
                        ..BindingLimits::default()
                    }),
                )
                .unwrap_err();
            assert!(
                matches!(error.kind, ModelErrorKind::ResourceExhausted(exhaustion)
            if exhaustion.dimension == Dimension::References && exhaustion.used == 4)
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
            // Indexed alias, type and operation lookups.
            let mut operations = Work::new(BindingLimits {
                references: 3,
                ..BindingLimits::default()
            });
            let bound = bindings
                .resolve_operation(unit, &operation, &mut operations)
                .unwrap();
            assert_eq!(operations.usage().references, 3);
            let error = bindings
                .resolve_operation(
                    unit,
                    &operation,
                    &mut Work::new(BindingLimits {
                        references: 2,
                        ..BindingLimits::default()
                    }),
                )
                .unwrap_err();
            assert!(
                matches!(error.kind, ModelErrorKind::ResourceExhausted(exhaustion)
            if exhaustion.dimension == Dimension::References && exhaustion.used == 2)
            );
            let result = Spanned {
                value: "step_result".into(),
                span: name.span,
            };
            // Catalog + alias + indexed lookup, two parameters, result, Boolean leaf.
            let mut values = Work::new(BindingLimits {
                references: 7,
                ..BindingLimits::default()
            });
            assert_eq!(
                bindings
                    .value(unit, &bound, &result, &mut values)
                    .unwrap()
                    .native(),
                &NativeType::Boolean
            );
            assert_eq!(values.usage().references, 7);
            let error = bindings
                .value(
                    unit,
                    &bound,
                    &result,
                    &mut Work::new(BindingLimits {
                        references: 6,
                        ..BindingLimits::default()
                    }),
                )
                .unwrap_err();
            assert!(
                matches!(error.kind, ModelErrorKind::ResourceExhausted(exhaustion)
            if exhaustion.dimension == Dimension::References && exhaustion.used == 6)
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
