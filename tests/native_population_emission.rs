// SPDX-License-Identifier: AGPL-3.0-only
//! Real model/source emission controls for static population authority.
//! No fixture supplies runtime populations or claims relationship correspondence.

#[path = "support/native_protocol/mod.rs"]
mod setup;

use ix_trace_rs::trace;
use quire_contract_ir as ir;
use quire_spec_language::checking::composed::{proofs, CauseKind, TypeDisposition, TypeLimits};
use quire_spec_language::formal_source::FormalSource;
use quire_spec_language::model_source::{self, ModelSourceLimits};
use quire_spec_language::native_model::{ModelLimits, NativeModel};
use quire_spec_language::protocol_artifact::{
    self as artifact, native, wire as w, Error, Invalid, Limits, Unsupported,
};
use quire_spec_language::{ByteDigest, Source, SourceIdentity};
use serde_json::{json, Value};
use setup::{Inputs, Unit};

fn two_objects(owner: &str) -> NativeModel {
    let mut document: Value =
        serde_json::from_str(include_str!("fixtures/native-rule-model.json")).unwrap();
    document["package"] = json!(format!("test/{owner}"));
    document["records"].as_array_mut().unwrap().extend([
        json!({"name":"Other", "fields":[
            {"name":"n","type":{"kind":"scalar","name":"Version"}},
            {"name":"peer","type":{"kind":"record","name":"OtherRef"}},
            {"name":"parent","type":{"kind":"option","value":{"kind":"record","name":"OtherRef"}}}
        ]}),
        json!({"name":"OtherRef", "fields":[
            {"name":"id","type":{"kind":"scalar","name":"ObjectId"}}
        ]}),
    ]);
    document["objects"].as_array_mut().unwrap().push(json!({
        "record":"Other", "reference":"OtherRef", "identity_field":"id", "universe":"nodes"
    }));
    let text = serde_json::to_vec_pretty(&document).unwrap();
    let source = Source::read(
        SourceIdentity {
            identity: format!("model:{owner}"),
            revision: "authored".into(),
        },
        format!("{owner}.json"),
        &text,
        quire_spec_language::Limits::default().source_bytes,
    )
    .unwrap();
    let formal = FormalSource::new(
        source,
        ir::SourceIdentity::new(
            ir::SourceDocumentId::new(format!("{owner}Model")).unwrap(),
            ir::SourceRevision::new(1).unwrap(),
        ),
    );
    model_source::read(
        formal,
        model_source::FORMAT_V2,
        ModelSourceLimits::default(),
    )
    .unwrap()
    .admit(ModelLimits::default())
    .unwrap()
}

fn graph_inputs(owner: &str) -> Inputs {
    Inputs::with_model(
        &[Unit {
            name: "two-populations",
            body: "invariant Left using G on M::Node at current {
            self.peer = self.peer and deref(self.peer).n >= 0 and reaches(self,self,parent)
        }
        invariant Right using G on M::Other at current {
            self.peer = self.peer and deref(self.peer).n >= 0 and reaches(self,self,parent)
        }
        protocol Flow using P over(view:M::Node) on origin {
            role Service on M::Node;
            run check Verify using G { view.peer = view.peer };
            finish Closed as(closed:M::Node) { true };
        }",
            declarations: &["Left", "Right", "Flow"],
        }],
        two_objects(owner),
    )
}

fn discharged(proofs: &proofs::ProofReport<'_, '_, '_>) {
    assert!(proofs.exhaustion().is_none(), "{:?}", proofs.exhaustion());
    for declaration in proofs.declarations() {
        assert_eq!(
            proofs.types().disposition(declaration.declaration()),
            Some(TypeDisposition::Typed)
        );
        assert_eq!(
            declaration.disposition(),
            proofs::ProofDisposition::Discharged,
            "{:?}",
            declaration.causes()
        );
    }
}

fn export(package: &w::Package, kind: w::ExportKind, path: &[&str]) -> w::ExportRef {
    w::ExportRef {
        model: 0,
        export: package.models[0]
            .exports
            .iter()
            .position(|export| export.kind == kind && export.path == path)
            .unwrap() as u32,
    }
}

#[track_caller]
fn failure<T>(report: &artifact::Report<T>, expected: Error) {
    match report.result() {
        Ok(_) => panic!("expected {expected:?}, admission succeeded"),
        Err(actual) => assert_eq!(actual, &expected),
    }
}

#[test]
#[trace("TC-121", "FR-042-AC-1", "FR-042-AC-4", "FR-042-AC-6", "FR-042-AC-7")]
fn original_object_roles_grant_distinct_populations_despite_equal_universe_names() {
    let inputs = graph_inputs("Populations");
    inputs.with_proofs(
        TypeLimits::default(),
        proofs::ProofLimits::default(),
        |proofs, selections| {
            discharged(proofs);
            let admitted = native::admit(proofs, selections, Limits::default())
                .into_result()
                .expect("real finite object population authority");
            let package = admitted.package();
            let left = export(package, w::ExportKind::Population, &["Node", "nodes"]);
            let right = export(package, w::ExportKind::Population, &["Other", "nodes"]);
            assert_ne!(left, right);
            assert_eq!(
                package.models[0]
                    .exports
                    .iter()
                    .filter(|export| export.kind == w::ExportKind::Population)
                    .count(),
                2
            );
            for (name, record, reference) in
                [("Left", "Node", "NodeRef"), ("Right", "Other", "OtherRef")]
            {
                let population = export(package, w::ExportKind::Population, &[record, "nodes"]);
                let object = export(package, w::ExportKind::Object, &[record]);
                let reference = export(package, w::ExportKind::Reference, &[reference]);
                assert!(package.types.iter().any(|ty| ty
                    == &w::Type::Reference {
                        export: reference.clone(),
                        object: object.clone(),
                        universe: population.clone()
                    }));
                let role = inputs
                    .model
                    .roles()
                    .objects
                    .iter()
                    .find(|role| role.record.as_str() == record)
                    .unwrap();
                let original = inputs.model.source().to_native(&role.source).unwrap();
                let exported = &package.models[0].exports[population.export as usize];
                assert_eq!(
                    exported.locus.span,
                    w::Span {
                        start: original.start as u32,
                        end: original.end as u32
                    }
                );
                assert_eq!(
                    exported.locus.formal.document,
                    inputs.model.source().identity().document().as_str()
                );
                assert_eq!(
                    exported.locus.source.digest,
                    inputs.model.source().source().digest()
                );
                let (index, declaration) = package
                    .declarations
                    .iter()
                    .enumerate()
                    .find(|(_, d)| d.name == name)
                    .unwrap();
                let (at, population_binding) = declaration
                    .bindings
                    .iter()
                    .enumerate()
                    .find(|(_, b)| {
                        b.kind == w::BindingKind::Population
                            && b.model.0.as_ref() == Some(&population)
                    })
                    .unwrap();
                assert_eq!(
                    population_binding.subject,
                    w::Subject::Declaration {
                        declaration: index as u32
                    }
                );
                let anchor = &declaration.anchors[population_binding.anchor.index as usize];
                assert_eq!(anchor.kind, w::AnchorKind::Current);
                assert_eq!(population_binding.requires, [anchor.binding.0.unwrap()]);
                let closure = declaration
                    .bindings
                    .iter()
                    .find(|b| b.kind == w::BindingKind::Closure && b.requires == [at as u32])
                    .unwrap();
                assert_eq!(closure.anchor, population_binding.anchor);
                assert_eq!(closure.model.0.as_ref(), Some(&population));
                assert_eq!(closure.value_type, population_binding.value_type);
                assert_eq!(
                    package.types[population_binding.value_type.0.unwrap() as usize],
                    w::Type::Object { export: object }
                );
                let reaches: Vec<_> = declaration
                    .values
                    .iter()
                    .filter_map(|value| match &value.operation {
                        w::ValueOperation::Reaches { edge, universe, .. } => Some((edge, universe)),
                        _ => None,
                    })
                    .collect();
                assert_eq!(reaches.len(), 1);
                assert_eq!(reaches[0].1, &population);
                assert_eq!(
                    package.models[0].exports[reaches[0].0.export as usize].path,
                    [record, "parent"]
                );
                assert!(declaration.values.iter().any(|value| matches!(
                    value.operation,
                    w::ValueOperation::Unary {
                        operator: w::Unary::Deref,
                        ..
                    }
                )));
            }
            let emitted = native::emit(&admitted, Limits::default())
                .into_result()
                .unwrap();
            assert_eq!(
                inputs
                    .read(proofs, &emitted)
                    .result()
                    .expect("independent original model and source selections")
                    .package(),
                package
            );
        },
    );
}

#[test]
#[trace("TC-121", "FR-042-AC-1", "FR-042-AC-4", "FR-042-AC-6")]
fn captured_pre_references_keep_population_obligations_at_the_original_observation() {
    let mut inputs = Inputs::new(&[Unit {
        name: "population-anchors",
        body: "pre Before using G on M::Node::step { self.peer = self.peer }
            post After using G on M::Node::step {
                let prior = pre(self.peer) in deref(prior).n >= 0 and deref(self.peer).n >= 0
            }
            protocol Flow using P over(view:M::Node) on origin {
                role Service on M::Node;
                run check Verify using G { view.peer = view.peer };
                finish Closed as(closed:M::Node) { true };
            }",
        declarations: &["Before", "After", "Flow"],
    }]);
    inputs.step_contracts("Before", "After");
    inputs.with_proofs(TypeLimits::default(), proofs::ProofLimits::default(), |proofs, selections| {
        discharged(proofs);
        let admitted = native::admit(proofs, selections, Limits::default()).into_result().expect("actual pre and post population requirements");
        let package = admitted.package();
        let after = package.declarations.iter().find(|d| d.name == "After").unwrap();
        let before = package.declarations.iter().find(|d| d.name == "Before").unwrap();
        let population_anchors = |declaration: &w::Declaration| {
            declaration.bindings.iter().filter(|b| b.kind == w::BindingKind::Population).map(|b| declaration.anchors[b.anchor.index as usize].kind).collect::<Vec<_>>()
        };
        assert_eq!(population_anchors(before), [w::AnchorKind::InvocationPre]);
        let after_anchors = population_anchors(after);
        assert_eq!(after_anchors.len(), 2);
        assert!(after_anchors.contains(&w::AnchorKind::InvocationPre));
        assert!(after_anchors.contains(&w::AnchorKind::InvocationPost));
        let (binder, prior) = after.binders.iter().enumerate().find(|(_, b)| b.name == "prior").unwrap();
        assert_eq!(prior.kind, w::BinderKind::Let);
        assert_eq!(after.anchors[prior.anchor.index as usize].kind, w::AnchorKind::InvocationPost);
        let initializer = prior.initializer.0.as_ref().unwrap();
        assert!(matches!(after.values[initializer.index as usize].operation, w::ValueOperation::Pre { .. }));
        let read = after.values.iter().find(|v| matches!(&v.operation, w::ValueOperation::Read { binder: selected } if selected.index as usize == binder)).unwrap();
        assert_eq!(after.anchors[read.anchor.index as usize].kind, w::AnchorKind::InvocationPost);
        let w::Origin::Anchor { anchor } = &read.origin else { panic!("immutable captured pre origin") };
        assert_eq!(after.anchors[anchor.index as usize].kind, w::AnchorKind::InvocationPre);
        let pre_population = after.bindings.iter().find(|b| b.kind == w::BindingKind::Population && b.anchor == *anchor).unwrap();
        assert_eq!(pre_population.anchor, *anchor);
        let emitted = native::emit(&admitted, Limits::default()).into_result().unwrap();
        assert_eq!(inputs.read(proofs, &emitted).result().expect("independent authored execution and model selections").package(), package);
    });
}

#[test]
#[trace("TC-121", "FR-042-AC-4", "FR-042-AC-7", "FR-042-AC-8")]
fn reader_refuses_crossed_reference_object_universe_and_original_role_loci() {
    let inputs = graph_inputs("PopulationMutants");
    inputs.with_proofs(
        TypeLimits::default(),
        proofs::ProofLimits::default(),
        |proofs, selections| {
            discharged(proofs);
            let admitted = native::admit(proofs, selections, Limits::default())
                .into_result()
                .unwrap();
            let package = admitted.package();
            let reference = export(package, w::ExportKind::Reference, &["NodeRef"]);
            let population = export(package, w::ExportKind::Population, &["Node", "nodes"]);
            let other_reference = export(package, w::ExportKind::Reference, &["OtherRef"]);
            let other_object = export(package, w::ExportKind::Object, &["Other"]);
            let other_population = export(package, w::ExportKind::Population, &["Other", "nodes"]);
            let reference_type = package
                .types
                .iter()
                .position(
                    |ty| matches!(ty, w::Type::Reference { export, .. } if export == &reference),
                )
                .unwrap();
            for axis in [
                "reference",
                "object",
                "universe",
                "role_span",
                "source",
                "formal_owner",
                "declaration_owner",
                "closure_pair",
            ] {
                let mut changed = package.clone();
                let expected = match axis {
                    "reference" | "object" | "universe" => {
                        let w::Type::Reference {
                            export,
                            object,
                            universe,
                        } = &mut changed.types[reference_type]
                        else {
                            panic!("original reference")
                        };
                        match axis {
                            "reference" => *export = other_reference.clone(),
                            "object" => *object = other_object.clone(),
                            "universe" => *universe = other_population.clone(),
                            _ => unreachable!(),
                        }
                        Error::Invalid(Invalid::Type)
                    }
                    "role_span" => {
                        changed.models[0].exports[population.export as usize]
                            .locus
                            .span
                            .start += 1;
                        Error::Invalid(Invalid::ForeignLocus)
                    }
                    "source" => {
                        changed.models[0].exports[population.export as usize]
                            .locus
                            .source
                            .authority = "test:foreign-source-owner".into();
                        Error::Invalid(Invalid::Selection)
                    }
                    "formal_owner" => {
                        changed.models[0].exports[population.export as usize]
                            .locus
                            .formal
                            .document = "ForeignModel".into();
                        Error::Invalid(Invalid::ForeignLocus)
                    }
                    "declaration_owner" => {
                        let owner = changed
                            .declarations
                            .iter()
                            .position(|d| d.name == "Left")
                            .unwrap();
                        let other = changed
                            .declarations
                            .iter()
                            .position(|d| d.name == "Right")
                            .unwrap();
                        let population = changed.declarations[owner]
                            .bindings
                            .iter_mut()
                            .find(|b| b.kind == w::BindingKind::Population)
                            .unwrap();
                        population.subject = w::Subject::Declaration {
                            declaration: other as u32,
                        };
                        Error::Invalid(Invalid::Owner)
                    }
                    "closure_pair" => {
                        let declaration = changed
                            .declarations
                            .iter_mut()
                            .find(|d| d.name == "Left")
                            .unwrap();
                        let population = declaration
                            .bindings
                            .iter()
                            .position(|b| b.kind == w::BindingKind::Population)
                            .unwrap() as u32;
                        let closure = declaration
                            .bindings
                            .iter_mut()
                            .find(|b| {
                                b.kind == w::BindingKind::Closure && b.requires == [population]
                            })
                            .unwrap();
                        closure.requires.clear();
                        Error::Invalid(Invalid::Binding)
                    }
                    _ => unreachable!(),
                };
                // This is adverse transport, independently resealed to reach the
                // intended authority check. It never acquires FamilyAdmission.
                let bytes = serde_json::to_vec(&changed).unwrap();
                failure(
                    &inputs.read_bytes(proofs, &bytes, ByteDigest::of(&bytes)),
                    expected,
                );
            }
        },
    );
}

#[test]
#[trace("TC-121", "FR-042-AC-3", "FR-042-AC-4", "FR-042-AC-8")]
fn another_admitted_model_cannot_replace_the_original_population_owner() {
    let inputs = graph_inputs("OriginalOwner");
    let foreign = two_objects("ForeignOwner");
    assert_ne!(
        inputs.model.environment().owner(),
        foreign.environment().owner()
    );
    assert_eq!(
        inputs.model.roles().objects[0].universe,
        foreign.roles().objects[0].universe
    );
    assert_eq!(
        inputs.model.roles().objects[0].record,
        foreign.roles().objects[0].record
    );
    assert_ne!(inputs.model.digest(), foreign.digest());
    inputs.with_proofs(
        TypeLimits::default(),
        proofs::ProofLimits::default(),
        |proofs, selected| {
            discharged(proofs);
            let substituted = [artifact::AdmittedModel {
                model: &foreign,
                ..selected.models[0]
            }];
            let changed = native::Selections {
                models: &substituted,
                ..*selected
            };
            failure(
                &native::admit(proofs, &changed, Limits::default()),
                Error::Invalid(Invalid::Model),
            );
        },
    );
}

#[test]
#[trace("TC-121", "FR-040-AC-7", "FR-042-AC-8")]
fn foreign_object_graph_edges_refuse_before_native_family_admission() {
    let inputs = Inputs::with_model(
        &[Unit {
            name: "foreign-graph",
            body: "predicate Crossed using G (left: M::Node, right: M::Other): Boolean {
            reaches(left,right,parent)
        }",
            declarations: &["Crossed"],
        }],
        two_objects("ForeignGraph"),
    );
    inputs.with_proofs(
        TypeLimits::default(),
        proofs::ProofLimits::default(),
        |proofs, selected| {
            let [id] = proofs.types().binding().namespace().lookup("Crossed") else {
                panic!("original declaration")
            };
            assert_eq!(
                proofs.types().disposition(*id),
                Some(TypeDisposition::Refused)
            );
            let typed = proofs.types().declaration(*id).unwrap();
            assert!(typed
                .causes()
                .iter()
                .any(|cause| matches!(cause.kind, CauseKind::TypeMismatch)));
            assert_eq!(
                proofs.disposition(*id),
                Some(proofs::ProofDisposition::Refused)
            );
            failure(
                &native::admit(proofs, selected, Limits::default()),
                Error::Unsupported(Unsupported::FamilyProof),
            );
        },
    );
}
