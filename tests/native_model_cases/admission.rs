// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-040/041: independently expected model shape and native-only refusals.

use super::*;
use ix_trace_rs::trace;

#[test]
#[trace("TC-040", "FR-015-AC-1")]
fn tc_040_all_sites_wrappers_and_operation_roles_match_authored_model() {
    let model = parts().model();
    let expected = [
        (
            "Version",
            vec![field_site("Node", "n"), field_site("Node", "items")],
        ),
        (
            "Signed",
            vec![field_site("Node", "signed"), field_site("Node", "den")],
        ),
        ("Count", vec![field_site("Node", "count")]),
        ("Wide", vec![field_site("Node", "wide")]),
        ("Distance", vec![field_site("Node", "distance")]),
        ("Duration", vec![field_site("Node", "duration")]),
        ("ObjectId", vec![field_site("NodeRef", "id")]),
    ];
    assert_eq!(model.roles().scalars.len(), expected.len());
    for (name, mut sites) in expected {
        sites.sort();
        let role = model
            .roles()
            .scalars
            .iter()
            .find(|s| s.name.as_str() == name)
            .unwrap();
        assert_eq!(role.sites, sites, "{name}");
        let span = model.source().to_native(&role.source).unwrap();
        let original: Value =
            serde_json::from_str(&model.source().source().text()[span.start..span.end]).unwrap();
        assert_eq!(original["name"], name);
        let expected_kind = match name {
            "ObjectId" => ScalarKind::Text { max_scalars: 256 },
            "Distance" => ScalarKind::Integer {
                unit: Unit::Named(symbol("metre")),
            },
            "Duration" => ScalarKind::Integer {
                unit: Unit::Named(symbol("second")),
            },
            _ => ScalarKind::Integer {
                unit: Unit::Dimensionless,
            },
        };
        assert_eq!(role.kind, expected_kind);
    }
    let node = model
        .environment()
        .types()
        .iter()
        .find(|d| d.name().as_str() == "Node")
        .unwrap();
    let ir::TypeDeclaration::Record { declaration: node } = node else {
        panic!("Node record")
    };
    let ty = |name: &str| {
        node.fields()
            .iter()
            .find(|f| f.name().as_str() == name)
            .unwrap()
            .value_type()
    };
    let reference = ir::ValueType::Record {
        name: symbol("NodeRef"),
    };
    assert_eq!(ty("parent"), &ir::ValueType::option(reference.clone()));
    assert_eq!(ty("peer"), &reference);
    assert_eq!(
        ty("items"),
        &ir::ValueType::collection(ir::CollectionType::new(integer(0, 1000), 3).unwrap())
    );
    // The admitted Collection is the profile's ordered, duplicate-preserving Seq;
    // this model test establishes its representation, not runtime list evaluation.
    assert_eq!(ty("distance"), &integer(0, 1000));
    assert_eq!(ty("duration"), &integer(0, 1000));
    assert_eq!(ty("den"), &integer(-10, 10));

    let ir::TypeDeclaration::Record {
        declaration: carrier,
    } = model
        .environment()
        .types()
        .iter()
        .find(|d| d.name().as_str() == "NodeRef")
        .unwrap()
    else {
        panic!("carrier record")
    };
    assert_eq!(carrier.fields().len(), 1);
    assert_eq!(carrier.fields()[0].name().as_str(), "id");
    assert_eq!(carrier.fields()[0].value_type(), &ir::ValueType::Text);
    for field in node.fields().iter().chain(carrier.fields()) {
        let span = model.source().to_native(field.source()).unwrap();
        let original: Value =
            serde_json::from_str(&model.source().source().text()[span.start..span.end]).unwrap();
        assert_eq!(original["name"], field.name().as_str());
    }
    assert_eq!(model.roles().objects.len(), 1);
    let object = &model.roles().objects[0];
    assert_eq!(
        (
            &object.record,
            &object.reference,
            &object.identity_field,
            &object.universe
        ),
        (
            &symbol("Node"),
            &symbol("NodeRef"),
            &symbol("id"),
            &symbol("nodes")
        )
    );
    assert_eq!(model.roles().operations.len(), 1);
    let operation = &model.roles().operations[0];
    assert_eq!(
        (
            operation.context.as_str(),
            operation.name.as_str(),
            operation.anchor.as_str()
        ),
        ("Node", "step", "step")
    );
    assert!(operation.parameters.is_empty());
    assert_eq!(operation.result, Some(symbol("step_result")));
    assert_eq!(operation.frame.fields, vec![(symbol("Node"), symbol("n"))]);
    assert!(operation.frame.created.is_empty() && operation.frame.deleted.is_empty());
    for (span, key, name) in [
        (&object.source, "record", "Node"),
        (&operation.source, "name", "step"),
    ] {
        let span = model.source().to_native(span).unwrap();
        let original: Value =
            serde_json::from_str(&model.source().source().text()[span.start..span.end]).unwrap();
        assert_eq!(original[key], name);
    }
    for value in model.environment().values() {
        let expected_kind = if value.name().as_str() == "step_result" {
            ir::ValueDeclarationKind::Input
        } else {
            ir::ValueDeclarationKind::State
        };
        assert_eq!(value.kind(), expected_kind);
        let span = model.source().to_native(value.source()).unwrap();
        let original: Value =
            serde_json::from_str(&model.source().source().text()[span.start..span.end]).unwrap();
        assert_eq!(original["name"], value.name().as_str());
    }
    assert!(model.environment().functions().is_empty());
    let owner = model.environment().owner();
    assert_eq!(
        (
            owner.package().as_str(),
            owner.requirement().as_str(),
            owner.revision().get()
        ),
        ("example/rule-tests", "RuleModel", 1)
    );
    assert_eq!(
        model.source().identity().document().as_str(),
        "RuleModelSource"
    );
    assert_eq!(model.source().identity().revision().get(), 1);
    assert_eq!(
        model.source().source().identity().identity,
        "test:rule-model"
    );
    assert_eq!(model.source().source().identity().revision, "draft:1");
    assert_eq!(
        model.source().source().digest(),
        quire_spec_language::ByteDigest::of(native_rule_model::FIXTURE.as_bytes())
    );
}

#[test]
#[trace("TC-040", "FR-015-AC-1")]
fn tc_040_scalar_value_sites_retain_nested_optional_sequences() {
    let model = authored(|data| {
        data["values"].as_array_mut().unwrap().push(json!({
            "name":"nested", "kind":"state", "type":{
                "kind":"option", "value":{"kind":"sequence", "maximum":3,
                    "value":{"kind":"option", "value":{"kind":"scalar", "name":"Version"}}}
            }
        }));
    })
    .model();
    let nested = model
        .environment()
        .values()
        .iter()
        .find(|v| v.name().as_str() == "nested")
        .unwrap();
    assert_eq!(
        nested.value_type(),
        &ir::ValueType::option(ir::ValueType::collection(
            ir::CollectionType::new(ir::ValueType::option(integer(0, 1000)), 3).unwrap()
        ))
    );
    let role = model
        .roles()
        .scalars
        .iter()
        .find(|s| s.name.as_str() == "Version")
        .unwrap();
    assert!(role.sites.contains(&ScalarSite::Value {
        name: symbol("nested")
    }));
    assert_eq!(role.sites.len(), 3);
}

#[test]
#[trace("TC-041", "FR-015-AC-2")]
fn tc_041_incomplete_or_inconsistent_scalar_assignments_refuse() {
    let mutations: &[fn(&mut Parts)] = &[
        |p| {
            scalar(p, "Version").sites.pop();
        },
        |p| scalar(p, "Version").sites.clear(),
        |p| scalar(p, "Version").kind = ScalarKind::Text { max_scalars: 10 },
        |p| {
            scalar(p, "Version").sites.push(ScalarSite::Value {
                name: symbol("missing"),
            })
        },
        |p| {
            scalar(p, "Version").sites.push(ScalarSite::Value {
                name: symbol("step_result"),
            })
        },
        |p| {
            scalar(p, "Version")
                .sites
                .push(field_site("Node", "parent"))
        },
        |p| scalar(p, "Count").sites.push(field_site("Node", "n")),
        |p| replace_field_type(p, "Node", "n", integer(0, 999)),
        |p| {
            scalar(p, "ObjectId").kind = ScalarKind::Text {
                max_scalars: ir::MAX_TEXT_LENGTH + 1,
            }
        },
        |p| p.roles.scalars.retain(|s| s.name.as_str() != "ObjectId"),
    ];
    for mutate in mutations {
        let mut input = parts();
        mutate(&mut input);
        refuse(input, Code::InvalidModelBinding);
    }
    let mut enumeration = extra_inventory();
    scalar(&mut enumeration, "Version")
        .sites
        .push(ScalarSite::Value {
            name: symbol("unused"),
        });
    refuse(enumeration, Code::InvalidModelBinding);
}

#[test]
#[trace("TC-041", "FR-015-AC-2")]
fn tc_041_unsupported_representations_include_unused_ir_declarations() {
    let unsupported = [
        ir::ValueType::integer(
            ir::IntegerType::new(
                ir::IntegerDomain::Unsigned,
                0,
                1000,
                ir::OverflowPolicy::Reject,
            )
            .unwrap(),
        ),
        ir::ValueType::integer(
            ir::IntegerType::new(
                ir::IntegerDomain::Signed,
                0,
                1000,
                ir::OverflowPolicy::Saturate,
            )
            .unwrap(),
        ),
        ir::ValueType::rational(ir::RationalType::new(-10, 10, 3).unwrap()),
    ];
    for ty in unsupported {
        let mut used = parts();
        replace_field_type(&mut used, "Node", "n", ty.clone());
        refuse(used, Code::UnsupportedConstruct);
        let mut unused = parts();
        add_value(
            &mut unused,
            "unused",
            ir::ValueDeclarationKind::State,
            ir::ValueType::option(ty),
        );
        refuse(unused, Code::UnsupportedConstruct);
    }
    let mut input = parts();
    let function = ir::PureFunctionDeclaration::new(
        symbol("unused_function"),
        Vec::new(),
        ir::ValueType::Boolean,
        input.roles.operations[0].source.clone(),
    )
    .unwrap();
    input.environment = ir::DeclarationEnvironment::new(
        input.environment.owner().clone(),
        input.environment.types().to_vec(),
        input.environment.values().to_vec(),
        vec![function],
    )
    .unwrap();
    refuse(input, Code::UnsupportedConstruct);
}

#[test]
#[trace("TC-041", "FR-015-AC-2")]
fn tc_041_sequence_field_maxima_are_profile_bounded() {
    for maximum in [10_000, 10_001, u32::MAX] {
        for (record, field, element) in [
            ("Node", "items", integer(0, 1000)),
            ("Unused", "spare", ir::ValueType::Boolean),
        ] {
            let ty = ir::ValueType::collection(ir::CollectionType::new(element, maximum).unwrap());
            let mut input = extra_inventory();
            replace_field_type(&mut input, record, field, ty.clone());
            if maximum == 10_000 {
                let model = input.model();
                let declaration = model
                    .environment()
                    .types()
                    .iter()
                    .find(|declaration| declaration.name().as_str() == record)
                    .unwrap();
                let ir::TypeDeclaration::Record { declaration } = declaration else {
                    panic!("record fixture");
                };
                let selected = declaration
                    .fields()
                    .iter()
                    .find(|selected| selected.name().as_str() == field)
                    .unwrap();
                assert_eq!(selected.value_type(), &ty);
            } else {
                refuse(input, Code::UnsupportedConstruct);
            }
        }
    }
}

#[test]
#[trace("TC-041", "FR-015-AC-2")]
fn tc_041_nested_sequence_value_maxima_cannot_be_raised_with_work_limits() {
    fn sequence(value: ir::ValueType, maximum: u32) -> ir::ValueType {
        ir::ValueType::collection(ir::CollectionType::new(value, maximum).unwrap())
    }
    for maximum in [10_000, 10_001, u32::MAX] {
        for ty in [
            ir::ValueType::option(sequence(ir::ValueType::Boolean, maximum)),
            sequence(
                ir::ValueType::option(sequence(ir::ValueType::Boolean, maximum)),
                1,
            ),
            sequence(
                ir::ValueType::option(sequence(ir::ValueType::Boolean, 1)),
                maximum,
            ),
        ] {
            let mut input = parts();
            add_value(
                &mut input,
                "unused_sequence",
                ir::ValueDeclarationKind::State,
                ty.clone(),
            );
            let source = input.source.source().clone();
            let result = NativeModel::new(
                input.source,
                input.environment,
                input.roles,
                ModelLimits {
                    artifact_bytes: usize::MAX,
                    roles: usize::MAX,
                    entries: usize::MAX,
                    nodes: usize::MAX,
                    depth: usize::MAX,
                },
            );
            if maximum == 10_000 {
                let model = result.unwrap();
                let selected = model
                    .environment()
                    .values()
                    .iter()
                    .find(|value| value.name().as_str() == "unused_sequence")
                    .unwrap();
                assert_eq!(selected.value_type(), &ty);
            } else {
                let error = result.unwrap_err();
                assert_eq!(error.code, Code::UnsupportedConstruct);
                assert_eq!(error.phase, Phase::Link);
                assert!(!error.is_incomplete());
                assert_eq!(error.source, *source.identity());
                assert_eq!(error.path, source.path());
            }
        }
    }
}

#[test]
#[trace("TC-041", "FR-015-AC-2")]
fn tc_041_ordinary_text_accepts_zero_and_requires_a_role() {
    for maximum in [0, 1, ir::MAX_TEXT_LENGTH] {
        let input = authored(|data| {
            data["scalars"]
                .as_array_mut()
                .unwrap()
                .push(json!({"name":"Label", "kind":"text", "max_scalars":maximum}));
            data["values"].as_array_mut().unwrap().push(
                json!({"name":"label", "kind":"state", "type":{"kind":"scalar", "name":"Label"}}),
            );
        });
        let model = NativeModel::new(
            input.source.clone(),
            input.environment.clone(),
            input.roles.clone(),
            ModelLimits::default(),
        )
        .unwrap();
        assert_eq!(
            model
                .roles()
                .scalars
                .iter()
                .find(|s| s.name.as_str() == "Label")
                .unwrap()
                .kind,
            ScalarKind::Text {
                max_scalars: maximum
            }
        );
        let mut missing = input;
        missing.roles.scalars.retain(|s| s.name.as_str() != "Label");
        refuse(missing, Code::InvalidModelBinding);
    }
}

#[test]
#[trace("TC-041", "FR-015-AC-2")]
fn tc_041_reference_carriers_and_object_ownership_are_explicit() {
    let mutations: &[fn(&mut Parts)] = &[
        |p| scalar(p, "ObjectId").kind = ScalarKind::Text { max_scalars: 0 },
        |p| {
            replace_field_type(
                p,
                "NodeRef",
                "id",
                ir::ValueType::option(ir::ValueType::Text),
            )
        },
        |p| {
            replace_field_type(p, "NodeRef", "id", ir::ValueType::Boolean);
            p.roles.scalars.retain(|s| s.name.as_str() != "ObjectId");
        },
        |p| {
            replace_record(p, "NodeRef", |r| {
                ir::RecordDeclaration::new(r.name().clone(), r.source().clone(), Vec::new())
                    .unwrap()
            });
            p.roles.scalars.retain(|s| s.name.as_str() != "ObjectId");
        },
        |p| {
            replace_record(p, "NodeRef", |r| {
                let mut fields = r.fields().to_vec();
                fields.push(ir::RecordFieldDeclaration::new(
                    symbol("extra"),
                    ir::ValueType::Boolean,
                    r.source().clone(),
                ));
                ir::RecordDeclaration::new(r.name().clone(), r.source().clone(), fields).unwrap()
            })
        },
        |p| p.roles.objects[0].record = symbol("missing"),
        |p| p.roles.objects[0].reference = symbol("missing"),
        |p| p.roles.objects[0].record = symbol("NodeRef"),
        |p| p.roles.objects.push(p.roles.objects[0].clone()),
    ];
    for mutate in mutations {
        let mut input = parts();
        mutate(&mut input);
        refuse(input, Code::InvalidModelBinding);
    }
    let mut shared_carrier = extra_inventory();
    shared_carrier.roles.objects[1].reference = symbol("NodeRef");
    shared_carrier.roles.objects[1].identity_field = symbol("id");
    refuse(shared_carrier, Code::InvalidModelBinding);
}

#[test]
#[trace("TC-041", "FR-015-AC-2")]
fn tc_041_operation_inputs_and_all_frame_inventories_are_checked() {
    let mutations: &[fn(&mut Parts)] = &[
        |p| p.roles.operations.clear(),
        |p| p.roles.operations.push(p.roles.operations[0].clone()),
        |p| p.roles.operations[0].context = symbol("NodeRef"),
        |p| p.roles.operations[0].parameters.push(symbol("missing")),
        |p| p.roles.operations[0].parameters = vec![symbol("step_result"), symbol("step_result")],
        |p| p.roles.operations[0].parameters.push(symbol("step_result")),
        |p| p.roles.operations[0].result = Some(symbol("self")),
        |p| p.roles.operations[0].result = Some(symbol("missing")),
        |p| p.roles.operations[0].result = None,
        |p| {
            add_value(
                p,
                "orphan",
                ir::ValueDeclarationKind::Input,
                ir::ValueType::Boolean,
            )
        },
        |p| {
            p.roles.operations[0]
                .frame
                .fields
                .push((symbol("NodeRef"), symbol("id")))
        },
        |p| {
            p.roles.operations[0]
                .frame
                .fields
                .push((symbol("Node"), symbol("n")))
        },
        |p| p.roles.operations[0].frame.created = vec![symbol("missing")],
        |p| p.roles.operations[0].frame.deleted = vec![symbol("NodeRef")],
        |p| p.roles.operations[0].frame.created = vec![symbol("Node"), symbol("Node")],
        |p| p.roles.operations[0].frame.deleted = vec![symbol("Node"), symbol("Node")],
    ];
    for mutate in mutations {
        let mut input = parts();
        mutate(&mut input);
        refuse(input, Code::InvalidModelBinding);
    }
    let valid = extra_inventory().model();
    assert_eq!(valid.roles().operations.len(), 2);
    let step = valid
        .roles()
        .operations
        .iter()
        .find(|o| o.name.as_str() == "step")
        .unwrap();
    assert_eq!(step.parameters, vec![symbol("second"), symbol("first")]);
    assert_eq!(step.frame.created, vec![symbol("Aux"), symbol("Node")]);
    assert_eq!(step.frame.deleted, vec![symbol("Aux"), symbol("Node")]);
}
