// SPDX-License-Identifier: AGPL-3.0-or-later
use super::*;
use crate::spine::{compose, DependencyInput};
use ix_trace_rs::trace;
use qsl_foundation::diagnostic::{CatalogCoded, UndefinedCoded, UndefinedReason, UndefinedRecord};
use qsl_foundation::SourceIdentity;
use qsl_semantics::model::key::{hex, DeclarationKey};
use qsl_semantics::model::refusal::ModelRefusalCause;
use quire_exact::{
    BoundViolation, CardinalityBound, CollectionKind, Incomplete, Integer, UniverseId, Value,
};
use quire_semantic_value::location::Origin;
use std::collections::BTreeMap;

#[test]
fn is_identifier_admits_ascii_identifiers_only() {
    assert!(is_identifier("seven"));
    assert!(is_identifier("_seven"));
    assert!(is_identifier("a1"));
    assert!(!is_identifier(""));
    assert!(!is_identifier("7x"));
    assert!(!is_identifier("a.b"));
}

const FIXTURE: &str = include_str!("../../../../tests/fixtures/spine-run.native");

fn source() -> SourceIdentity {
    SourceIdentity::new("agent-ix", "spine-run-fixture", "git", "1")
}

fn call(function: &str, arguments: Vec<CallArgument>) -> Call {
    Call {
        function: function.to_owned(),
        arguments,
        accounting: default_accounting(1_000_000),
    }
}

fn run_fixture(bytes: &str, call: &Call) -> Result<CallOutcome, Box<RunRefusal>> {
    run(
        source(),
        "spine-run.native",
        bytes.as_bytes(),
        &BTreeMap::new(),
        &DependencyInput::default(),
        SpineLimits::default(),
        call,
        &qsl_semantics::model::object_environment::ObjectEnvironment::default(),
        &Cancel::new(),
    )
    .map(|(_, outcome)| outcome)
}

struct LookupDriverFixture {
    source: String,
    packages: BTreeMap<[u8; 32], Vec<u8>>,
}

fn lookup_driver_fixture(functions: &str) -> LookupDriverFixture {
    let mut document: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../../../../tests/fixtures/spine-model.semantic-ir.json"
    ))
    .expect("the lookup driver model fixture is JSON");
    document["constructs"]
        .as_array_mut()
        .expect("the model fixture has constructs")
        .push(serde_json::json!({
            "kind": {"module": "acme/orders", "name": "population"},
            "moduleVersion": "1.0.0",
            "manifestDigest": "sha256:0000000000000000000000000000000000000000000000000000000000000000",
            "construct": {
                "identity": "none",
                "shape": "record",
                "members": {},
                "meaning": "quire.meaning.model.population/v1"
            }
        }));
    document["populations"] = serde_json::json!([{
        "identity": "ix://acme/orders/Fleet",
        "displayName": "Fleet",
        "kind": {"module": "acme/orders", "name": "population"},
        "members": ["ix://acme/orders/Gadget"],
        "extent": "closed",
        "origin": {"generated": {
            "generatorIdentity": "ix://acme/orders/Fleet",
            "generatorVersion": "1.0.0",
            "inputIdentities": ["ix://acme/orders/Fleet"]
        }}
    }]);
    let bytes = serde_json::to_vec(&document).expect("the lookup driver model serializes");
    let digest = qsl_semantics::model::intake::PackageDocument::parse(
        &bytes,
        qsl_foundation::IntakeLimits::default(),
    )
    .expect("the lookup driver model admits")
    .jcs_digest();
    let source = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n\
         model M = \"acme/orders\" version \"1.0.0\" digest \"sha256-jcs:{}\";\n\
         {}\n",
        hex(&digest),
        functions
    );
    let packages = qsl_semantics::model::intake::package_input([bytes.as_slice()]);
    LookupDriverFixture { source, packages }
}

fn lookup_driver_context(
    fixture: &LookupDriverFixture,
) -> (
    qsl_semantics::model::object_environment::ObjectEnvironment,
    quire_exact::PopulationId,
    quire_exact::ObjectReference,
    quire_exact::ObjectReference,
) {
    use qsl_semantics::model::dispatch::GeneralizationClosure;
    use qsl_semantics::model::population::{
        admit_binding, AdmissionMeter, AdmissionOutcome, PopulationAdmissionLimits,
        PopulationDocument, PopulationMember,
    };
    use quire_semantic_value::object_closure::ObjectClosure;

    let cancel = Cancel::new();
    let source_identity = source();
    let parsed = crate::spine::parse(
        &crate::spine::ParseRequest {
            source: &source_identity,
            path: "tc-792-lookup.native",
            bytes: fixture.source.as_bytes(),
        },
        SpineLimits::default().source,
        &cancel,
    )
    .expect("the lookup driver source parses")
    .into_value();
    let models = crate::spine::select(
        &parsed,
        &fixture.packages,
        SpineLimits::default().model,
        &cancel,
    )
    .expect("the lookup driver model selects")
    .into_value();
    let checked = crate::spine::check(
        &parsed,
        &models,
        &DependencyInput::default(),
        &crate::spine::LockEvidence::default(),
        SpineLimits::default(),
        &cancel,
    )
    .expect("the lookup driver source checks")
    .into_value();
    let view = &models.models()[0].view;
    let (type_key, type_identity) = view
        .type_identities()
        .iter()
        .find(|(key, _)| key.node.rsplit('/').next() == Some("Gadget"))
        .map(|(key, identity)| (key.clone(), *identity))
        .expect("Gadget is declared");
    let population_key = view
        .domain_package()
        .records
        .iter()
        .find_map(|record| match record {
            qsl_semantics::model::domain_package::DomainPackageRecord::Population(population)
                if population.key.node.rsplit('/').next() == Some("Fleet") =>
            {
                Some(population.key.clone())
            }
            _ => None,
        })
        .expect("Fleet is declared");
    let document = PopulationDocument {
        model_identity: view.model_selection().identity.clone(),
        members: vec![PopulationMember {
            object: "g1".to_owned(),
            type_identity: type_key.clone(),
            field_values: Vec::new(),
        }],
    };
    let mut meter = AdmissionMeter::new(PopulationAdmissionLimits::UNLIMITED);
    let AdmissionOutcome::Admitted(binding) = admit_binding(
        view,
        &document,
        &population_key,
        GeneralizationClosure::Closed,
        Some(1),
        &mut meter,
    ) else {
        panic!("the lookup driver population admits");
    };
    let universe = view
        .object_universe_of(&type_key)
        .expect("Gadget has a universe")
        .identity();
    let present = quire_exact::ObjectReference::new(
        universe,
        type_identity,
        quire_exact::ObjectId::new("g1").unwrap(),
    );
    let absent = quire_exact::ObjectReference::new(
        universe,
        type_identity,
        quire_exact::ObjectId::new("missing").unwrap(),
    );
    let world = ObjectClosure::new(
        checked.package().graph().scope().types(),
        [(present.clone(), Vec::new())],
        &[],
    )
    .expect("the lookup driver object closure admits");
    let population_id = binding.population_id();
    let objects = qsl_semantics::model::object_environment::ObjectEnvironment::new(world)
        .with_population(binding)
        .expect("the lookup driver population binds");
    (objects, population_id, present, absent)
}

#[test]
#[trace("TC-792", "FR-100-AC-14", "FR-100-AC-16", "FR-100-AC-17")]
fn tc_792_source_compiled_lookup_driver_preserves_present_and_absent_paths() {
    let fixture = lookup_driver_fixture(
        "function query using v(p: Population<M::Gadget>[1], r: M::Gadget): M::Gadget pure { lookup<M::Gadget>(p, r) absent refused }",
    );
    let (objects, population, present, absent) = lookup_driver_context(&fixture);
    let invoke = |reference| {
        run(
            source(),
            "tc-792-lookup.native",
            fixture.source.as_bytes(),
            &fixture.packages,
            &DependencyInput::default(),
            SpineLimits::default(),
            &Call {
                function: "query".to_owned(),
                arguments: vec![
                    CallArgument {
                        parameter: "p".to_owned(),
                        value: Value::Population(population),
                    },
                    CallArgument {
                        parameter: "r".to_owned(),
                        value: Value::Reference(reference),
                    },
                ],
                accounting: default_accounting(1_000_000),
            },
            &objects,
            &Cancel::new(),
        )
        .map(|(_, outcome)| outcome)
    };
    let present_outcome = invoke(present.clone()).expect("present lookup runs");
    assert!(matches!(
        present_outcome,
        CallOutcome::Completed(CallValue::Reference(reference)) if reference == present
    ));
    let absent_outcome = invoke(absent).expect("absent lookup returns a refusal outcome");
    assert!(matches!(
        absent_outcome,
        CallOutcome::Refused(CallRefusal::Record { code, .. })
            if code.code() == "invalid_runtime_input" && code.cause() == "absent-key"
    ));
}

#[test]
#[trace("TC-792", "FR-100-AC-14", "FR-100-AC-16")]
fn tc_792_source_driver_admits_unused_parameters_before_body_evaluation() {
    let fixture = lookup_driver_fixture(
        "function unused using v(p: Population<M::Gadget>[1], r: M::Gadget): Boolean pure { true }",
    );
    let (objects, population, _, present) = lookup_driver_context(&fixture);
    let foreign_universe = quire_exact::ObjectReference::new(
        UniverseId::from_digest([0xa5; 32]),
        present.object_type(),
        present.object().clone(),
    );
    let refusal = run(
        source(),
        "tc-792-lookup.native",
        fixture.source.as_bytes(),
        &fixture.packages,
        &DependencyInput::default(),
        SpineLimits::default(),
        &Call {
            function: "unused".to_owned(),
            arguments: vec![
                CallArgument {
                    parameter: "p".to_owned(),
                    value: Value::Population(population),
                },
                CallArgument {
                    parameter: "r".to_owned(),
                    value: Value::Reference(foreign_universe),
                },
            ],
            accounting: default_accounting(1_000_000),
        },
        &objects,
        &Cancel::new(),
    )
    .expect_err("unused reference parameters still undergo admission");
    assert!(matches!(
        *refusal,
        RunRefusal::Admission { position: 1, refusal }
            if matches!(refusal.cause, ModelRefusalCause::ForeignUniverse { .. })
    ));
}

#[test]
#[trace("TC-792", "FR-100-AC-14", "FR-100-AC-16")]
fn tc_792_source_driver_binding_precedence_precedes_value_admission() {
    let fixture = lookup_driver_fixture(
        "function query using v(p: Population<M::Gadget>[1], r: M::Gadget): M::Gadget pure { lookup<M::Gadget>(p, r) absent refused }",
    );
    let (objects, _, _, _) = lookup_driver_context(&fixture);
    let refusal = run(
        source(),
        "tc-792-lookup.native",
        fixture.source.as_bytes(),
        &fixture.packages,
        &DependencyInput::default(),
        SpineLimits::default(),
        &Call {
            function: "query".to_owned(),
            arguments: vec![CallArgument {
                parameter: "p".to_owned(),
                value: Value::Boolean(true),
            }],
            accounting: default_accounting(1_000_000),
        },
        &objects,
        &Cancel::new(),
    )
    .expect_err("unbound r precedes p's wrong value kind");
    assert!(matches!(
        *refusal,
        RunRefusal::UnboundParameter { parameter } if parameter == "r"
    ));
}

#[test]
#[trace("TC-792", "FR-100-AC-14", "FR-100-AC-16")]
fn tc_792_source_driver_cancellation_returns_no_lookup_result() {
    let fixture = lookup_driver_fixture(
        "function query using v(p: Population<M::Gadget>[1], r: M::Gadget): M::Gadget pure { lookup<M::Gadget>(p, r) absent refused }",
    );
    let (objects, population, _, _) = lookup_driver_context(&fixture);
    let cancel = Cancel::new();
    cancel.cancel(quire_exact::CancelCause::Requested);
    let refusal = run(
        source(),
        "tc-792-lookup.native",
        fixture.source.as_bytes(),
        &fixture.packages,
        &DependencyInput::default(),
        SpineLimits::default(),
        &Call {
            function: "query".to_owned(),
            arguments: vec![
                CallArgument {
                    parameter: "p".to_owned(),
                    value: Value::Population(population),
                },
                CallArgument {
                    parameter: "r".to_owned(),
                    value: Value::Boolean(true),
                },
            ],
            accounting: default_accounting(1_000_000),
        },
        &objects,
        &cancel,
    )
    .expect_err("a cancelled driver has no lookup result");
    assert!(matches!(*refusal, RunRefusal::Cancelled(_)));
}

fn arg(parameter: &str, value: i64) -> CallArgument {
    CallArgument {
        parameter: parameter.to_owned(),
        value: Value::Integer(Integer::from(value)),
    }
}

/// TC-792 / FR-100-AC-14..19 red contract: the reviewed spine boundary takes
/// shared typed values and the admitted object context, then preserves the
/// existing scalar/accounting/cancel inputs around that call.
#[test]
#[trace("TC-792", "FR-100-AC-16")]
fn tc_792_typed_call_contract_is_not_an_integer_only_facade() {
    let call = Call {
        function: "held".to_owned(),
        arguments: vec![CallArgument {
            parameter: "w".to_owned(),
            value: Value::Boolean(true),
        }],
        accounting: default_accounting(1_000_000),
    };
    let result = run(
        source(),
        "spine-model.native",
        include_bytes!("../../../../tests/fixtures/spine-model.native"),
        &qsl_semantics::model::intake::package_input([include_bytes!(
            "../../../../tests/fixtures/spine-model.semantic-ir.json"
        )
        .as_slice()]),
        &DependencyInput::default(),
        SpineLimits::default(),
        &call,
        &qsl_semantics::model::object_environment::ObjectEnvironment::default(),
        &Cancel::new(),
    );
    assert!(matches!(
        result,
        Err(ref refusal) if matches!(**refusal, RunRefusal::WrongValueKind { position: 0 })
    ));
}

#[test]
fn typed_model_values_bind_without_integer_coercion() {
    let reference = quire_exact::ObjectReference::new(
        UniverseId::from_digest([1; 32]),
        quire_exact::EffectiveId::from_digest([2; 32]),
        quire_exact::ObjectId::new("g1").unwrap(),
    );
    let population = quire_exact::PopulationId::from_digest([3; 32]);
    let bound = bind_arguments(
        &[
            (
                "reference".to_owned(),
                ValueType::Reference(reference.object_type()),
            ),
            ("population".to_owned(), ValueType::Population(Some(1))),
        ],
        &[
            CallArgument {
                parameter: "population".to_owned(),
                value: Value::Population(population),
            },
            CallArgument {
                parameter: "reference".to_owned(),
                value: Value::Reference(reference.clone()),
            },
        ],
    )
    .unwrap();
    assert_eq!(bound.len(), 2);
    assert!(matches!(&bound[0], Value::Reference(value) if value == &reference));
    assert!(matches!(&bound[1], Value::Population(value) if value == &population));
}

#[test]
#[trace("TC-792", "FR-100-AC-14", "FR-100-AC-16")]
fn tc_792_driver_returns_the_admitted_reference_identity() {
    let bytes = include_bytes!("../../../../tests/fixtures/spine-model.native");
    let document = include_bytes!("../../../../tests/fixtures/spine-model.semantic-ir.json");
    let packages = qsl_semantics::model::intake::package_input([document.as_slice()]);
    let cancel = Cancel::new();
    let identity = source();
    let parsed = crate::spine::parse(
        &crate::spine::ParseRequest {
            source: &identity,
            path: "spine-model.native",
            bytes,
        },
        SpineLimits::default().source,
        &cancel,
    )
    .unwrap()
    .into_value();
    let models = crate::spine::select(&parsed, &packages, SpineLimits::default().model, &cancel)
        .unwrap()
        .into_value();
    let checked = crate::spine::check(
        &parsed,
        &models,
        &DependencyInput::default(),
        &crate::spine::LockEvidence::default(),
        SpineLimits::default(),
        &cancel,
    )
    .unwrap()
    .into_value();
    let model = &models.models()[0].view;
    let (type_key, object_type) = model
        .type_identities()
        .iter()
        .find_map(|(key, identity)| {
            key.node
                .rsplit('/')
                .next()
                .is_some_and(|name| name == "Gadget")
                .then_some((key, *identity))
        })
        .expect("Gadget is in the model fixture");
    let reference = quire_exact::ObjectReference::new(
        model.object_universe_of(type_key).unwrap().identity(),
        object_type,
        quire_exact::ObjectId::new("g1").unwrap(),
    );
    let world = quire_semantic_value::object_closure::ObjectClosure::new(
        checked.package().graph().scope().types(),
        [(
            reference.clone(),
            vec![(
                "code",
                quire_exact::FieldValue::Present(Value::Integer(Integer::from(7_i64))),
            )],
        )],
        &[],
    )
    .unwrap();
    let objects = qsl_semantics::model::object_environment::ObjectEnvironment::new(world);
    let outcome = run(
        source(),
        "spine-model.native",
        bytes,
        &packages,
        &DependencyInput::default(),
        SpineLimits::default(),
        &call(
            "keep",
            vec![CallArgument {
                parameter: "g".to_owned(),
                value: Value::Reference(reference.clone()),
            }],
        ),
        &objects,
        &Cancel::new(),
    )
    .unwrap()
    .1;
    assert!(matches!(
        outcome,
        CallOutcome::Completed(CallValue::Reference(value)) if value == reference
    ));
}

fn synthetic_model(
    placeholder: &str,
) -> (
    qsl_semantics::model::normalize::EffectiveView,
    DeclarationKey,
    DeclarationKey,
) {
    use qsl_semantics::model::accounting::ModelNormalizationLimits;
    use qsl_semantics::model::domain_package::{
        DomainPackage, DomainPackageRecord, DomainPackageRef, Extent, ObjectTypeRecord,
        PopulationRecord,
    };
    use qsl_semantics::model::normalize::{normalize, NormalizeOutcome};

    let object = DeclarationKey::fixture(format!("{placeholder}/A"));
    let population = DeclarationKey::fixture(format!("{placeholder}/P"));
    let domain = DomainPackage::new(
        DomainPackageRef::fixture(placeholder),
        vec![
            DomainPackageRecord::ObjectType(ObjectTypeRecord {
                key: object.clone(),
                interface_features: None,
                abstract_type: false,
                supertypes: Vec::new(),
            }),
            DomainPackageRecord::Population(PopulationRecord {
                key: population.clone(),
                member_types: vec![object.clone()],
                extent: Extent::Closed,
            }),
        ],
    );
    let NormalizeOutcome::Completed(view) = normalize(&domain, ModelNormalizationLimits::UNLIMITED)
    else {
        panic!("synthetic model normalizes");
    };
    (view, object, population)
}

fn synthetic_reference_hierarchy(
    placeholder: &str,
) -> (
    qsl_semantics::model::normalize::EffectiveView,
    DeclarationKey,
    DeclarationKey,
    DeclarationKey,
    DeclarationKey,
) {
    use qsl_semantics::model::accounting::ModelNormalizationLimits;
    use qsl_semantics::model::domain_package::{
        DomainPackage, DomainPackageRecord, DomainPackageRef, ObjectTypeRecord,
    };
    use qsl_semantics::model::normalize::{normalize, NormalizeOutcome};

    let root = DeclarationKey::fixture(format!("{placeholder}/Root"));
    let target = DeclarationKey::fixture(format!("{placeholder}/T"));
    let subtype = DeclarationKey::fixture(format!("{placeholder}/S"));
    let unrelated = DeclarationKey::fixture(format!("{placeholder}/U"));
    let domain = DomainPackage::new(
        DomainPackageRef::fixture(placeholder),
        vec![
            DomainPackageRecord::ObjectType(ObjectTypeRecord {
                key: root.clone(),
                interface_features: None,
                abstract_type: false,
                supertypes: Vec::new(),
            }),
            DomainPackageRecord::ObjectType(ObjectTypeRecord {
                key: target.clone(),
                interface_features: None,
                abstract_type: false,
                supertypes: vec![root.clone()],
            }),
            DomainPackageRecord::ObjectType(ObjectTypeRecord {
                key: subtype.clone(),
                interface_features: None,
                abstract_type: false,
                supertypes: vec![target.clone()],
            }),
            DomainPackageRecord::ObjectType(ObjectTypeRecord {
                key: unrelated.clone(),
                interface_features: None,
                abstract_type: false,
                supertypes: vec![root.clone()],
            }),
        ],
    );
    let NormalizeOutcome::Completed(view) = normalize(&domain, ModelNormalizationLimits::UNLIMITED)
    else {
        panic!("synthetic reference hierarchy normalizes");
    };
    (view, target, subtype, unrelated, root)
}

#[test]
#[trace("TC-792", "FR-100-AC-16")]
fn tc_792_population_and_foreign_model_admission_keep_typed_causes() {
    use qsl_semantics::model::dispatch::GeneralizationClosure;
    use qsl_semantics::model::intake::SelectedModel;
    use qsl_semantics::model::object_environment::ObjectEnvironment;
    use qsl_semantics::model::population::{
        admit_binding, AdmissionMeter, AdmissionOutcome, PopulationAdmissionLimits,
        PopulationDocument, PopulationMember,
    };
    use quire_semantic_value::declaration::TypeEnvironment;

    let types = TypeEnvironment::new([], []).unwrap();
    let unknown = Value::Population(quire_exact::PopulationId::from_digest([9; 32]));
    let unregistered = admit_model_arguments(
        &[],
        &types,
        &[("population".to_owned(), ValueType::Population(Some(1)))],
        &[unknown],
        &ObjectEnvironment::default(),
    );
    assert!(matches!(
        unregistered,
        Err(ref refusal) if matches!(**refusal, RunRefusal::WrongValueKind { position: 0 })
    ));

    let (selected_view, _, _) = synthetic_model("selected");
    let (foreign_view, foreign_type, foreign_population) = synthetic_model("foreign");
    let document = PopulationDocument {
        model_identity: foreign_view.model_selection().identity.clone(),
        members: vec![PopulationMember {
            object: "a1".to_owned(),
            type_identity: foreign_type,
            field_values: Vec::new(),
        }],
    };
    let mut meter = AdmissionMeter::new(PopulationAdmissionLimits::UNLIMITED);
    let AdmissionOutcome::Admitted(binding) = admit_binding(
        &foreign_view,
        &document,
        &foreign_population,
        GeneralizationClosure::Closed,
        Some(1),
        &mut meter,
    ) else {
        panic!("foreign population admits");
    };
    let population = Value::Population(binding.population_id());
    let objects = ObjectEnvironment::default()
        .with_population(binding)
        .unwrap();
    let maximum_mismatch = admit_model_arguments(
        &[],
        &types,
        &[("population".to_owned(), ValueType::Population(Some(2)))],
        std::slice::from_ref(&population),
        &objects,
    )
    .expect_err("a declared maximum mismatch refuses before evaluation");
    assert!(matches!(
        *maximum_mismatch,
        RunRefusal::WrongValueKind { position: 0 }
    ));
    let selected = SelectedModel {
        alias: "M".to_owned(),
        span: qsl_foundation::Span { start: 0, end: 0 },
        view: selected_view,
    };
    let refusal = admit_model_arguments(
        &[selected],
        &types,
        &[("population".to_owned(), ValueType::Population(Some(1)))],
        &[population],
        &objects,
    )
    .expect_err("a population from another selected model refuses");
    assert!(matches!(
        *refusal,
        RunRefusal::Admission { position: 0, refusal }
            if matches!(refusal.cause, ModelRefusalCause::ForeignModelSelection { .. })
    ));
}

#[test]
#[trace("TC-792", "FR-100-AC-16")]
fn tc_792_reference_foreign_model_precedes_universe_and_type_checks() {
    use qsl_semantics::model::intake::SelectedModel;
    use quire_semantic_value::declaration::TypeEnvironment;

    let (selected_view, selected_type, _) = synthetic_model("selected-reference");
    let (foreign_view, foreign_type, _) = synthetic_model("foreign-reference");
    let selected_model = SelectedModel {
        alias: "M".to_owned(),
        span: qsl_foundation::Span { start: 0, end: 0 },
        view: selected_view.clone(),
    };
    let foreign_model = SelectedModel {
        alias: "M2".to_owned(),
        span: qsl_foundation::Span { start: 0, end: 0 },
        view: foreign_view.clone(),
    };
    let declared = selected_view.type_identities()[&selected_type];
    let supplied = foreign_view.type_identities()[&foreign_type];
    let reference = Value::Reference(quire_exact::ObjectReference::new(
        foreign_view
            .object_universe_of(&foreign_type)
            .unwrap()
            .identity(),
        supplied,
        quire_exact::ObjectId::new("f1").unwrap(),
    ));
    let types = TypeEnvironment::new([], []).unwrap();
    let refusal = admit_model_arguments(
        &[selected_model, foreign_model],
        &types,
        &[("reference".to_owned(), ValueType::Reference(declared))],
        &[reference],
        &qsl_semantics::model::object_environment::ObjectEnvironment::default(),
    )
    .expect_err("a reference from another selected model refuses");
    assert!(matches!(
        *refusal,
        RunRefusal::Admission { position: 0, refusal }
            if matches!(refusal.cause, ModelRefusalCause::ForeignModelSelection { .. })
    ));
}

#[test]
#[trace("TC-792", "FR-100-AC-16")]
fn tc_792_foreign_type_maximum_and_dangling_controls_keep_admission_causes() {
    use qsl_semantics::model::intake::SelectedModel;
    use quire_semantic_value::declaration::{ObjectTypeDeclaration, TypeEnvironment};

    let (selected_view, selected_type, _) = synthetic_model("selected-controls");
    let (foreign_view, foreign_type, _) = synthetic_model("foreign-controls");
    let models = vec![
        SelectedModel {
            alias: "M".to_owned(),
            span: qsl_foundation::Span { start: 0, end: 0 },
            view: selected_view.clone(),
        },
        SelectedModel {
            alias: "M2".to_owned(),
            span: qsl_foundation::Span { start: 0, end: 0 },
            view: foreign_view.clone(),
        },
    ];
    let foreign_reference = Value::Reference(quire_exact::ObjectReference::new(
        foreign_view
            .object_universe_of(&foreign_type)
            .unwrap()
            .identity(),
        foreign_view.type_identities()[&foreign_type],
        quire_exact::ObjectId::new("foreign").unwrap(),
    ));
    let unknown_type = quire_exact::EffectiveId::from_digest([7; 32]);
    let foreign_type_refusal = admit_model_arguments(
        &models,
        &TypeEnvironment::new([], []).unwrap(),
        &[("reference".to_owned(), ValueType::Reference(unknown_type))],
        &[foreign_reference],
        &ObjectEnvironment::default(),
    )
    .expect_err("an uncovered supplied type retains foreign-type admission");
    assert!(matches!(
        *foreign_type_refusal,
        RunRefusal::Admission { position: 0, refusal }
            if matches!(refusal.cause, ModelRefusalCause::ForeignType { .. })
    ));

    let selected_identity = selected_view.type_identities()[&selected_type];
    let selected_reference = Value::Reference(quire_exact::ObjectReference::new(
        selected_view
            .object_universe_of(&selected_type)
            .unwrap()
            .identity(),
        selected_identity,
        quire_exact::ObjectId::new("dangling").unwrap(),
    ));
    let types = TypeEnvironment::new(
        [],
        [ObjectTypeDeclaration::new(
            selected_identity,
            "A",
            Vec::new(),
        )],
    )
    .unwrap();
    let dangling = admit_model_arguments(
        &models[..1],
        &types,
        &[(
            "reference".to_owned(),
            ValueType::Reference(selected_identity),
        )],
        &[selected_reference],
        &ObjectEnvironment::default(),
    )
    .expect_err("an admitted type with no object retains dangling-reference admission");
    assert!(matches!(
        *dangling,
        RunRefusal::DanglingReference { position: 0 }
    ));

    let malformed = quire_exact::ObjectId::new("");
    assert!(
        malformed.is_err(),
        "raw malformed object identity is refused"
    );
}

#[test]
#[trace("TC-792", "FR-100-AC-14", "FR-100-AC-16")]
fn tc_792_reference_exact_subtype_and_unrelated_vectors_use_model_conformance() {
    use qsl_semantics::model::intake::SelectedModel;
    use qsl_semantics::model::object_environment::ObjectEnvironment;
    use quire_semantic_value::declaration::{ObjectTypeDeclaration, TypeEnvironment};
    use quire_semantic_value::object_closure::ObjectClosure;

    let (view, target, subtype, unrelated, root) = synthetic_reference_hierarchy("tc-792-vectors");
    let target_id = view.type_identities()[&target];
    let subtype_id = view.type_identities()[&subtype];
    let unrelated_id = view.type_identities()[&unrelated];
    let root_id = view.type_identities()[&root];
    let types = TypeEnvironment::new(
        [],
        [
            ObjectTypeDeclaration::new(root_id, "Root", Vec::new()),
            ObjectTypeDeclaration::new(target_id, "T", Vec::new()).with_supertypes(vec![root_id]),
            ObjectTypeDeclaration::new(subtype_id, "S", Vec::new())
                .with_supertypes(vec![target_id]),
            ObjectTypeDeclaration::new(unrelated_id, "U", Vec::new())
                .with_supertypes(vec![root_id]),
        ],
    )
    .unwrap();
    let universe = view.object_universe_of(&target).unwrap().identity();
    let target_ref = quire_exact::ObjectReference::new(
        universe,
        target_id,
        quire_exact::ObjectId::new("t").unwrap(),
    );
    let subtype_ref = quire_exact::ObjectReference::new(
        universe,
        subtype_id,
        quire_exact::ObjectId::new("s").unwrap(),
    );
    let unrelated_ref = quire_exact::ObjectReference::new(
        universe,
        unrelated_id,
        quire_exact::ObjectId::new("u").unwrap(),
    );
    let world = ObjectEnvironment::new(
        ObjectClosure::new(
            &types,
            [
                (target_ref.clone(), Vec::new()),
                (subtype_ref.clone(), Vec::new()),
                (unrelated_ref.clone(), Vec::new()),
            ],
            &[],
        )
        .unwrap(),
    );
    let model = SelectedModel {
        alias: "M".to_owned(),
        span: qsl_foundation::Span { start: 0, end: 0 },
        view,
    };
    let parameter = [("key".to_owned(), ValueType::Reference(target_id))];
    for reference in [target_ref, subtype_ref] {
        admit_model_arguments(
            std::slice::from_ref(&model),
            &types,
            &parameter,
            &[Value::Reference(reference)],
            &world,
        )
        .expect("exact and proper subtype references admit");
    }
    let unrelated = admit_model_arguments(
        std::slice::from_ref(&model),
        &types,
        &parameter,
        &[Value::Reference(unrelated_ref)],
        &world,
    )
    .expect_err("an in-model unrelated type refuses");
    assert!(matches!(
        *unrelated,
        RunRefusal::WrongValueKind { position: 0 }
    ));
}

#[test]
#[trace("TC-792", "FR-100-AC-14", "FR-100-AC-16", "FR-100-AC-17")]
fn tc_792_lookup_present_absent_malformed_and_accounting_vectors_preserve_owners() {
    use qsl_foundation::absence::AbsenceMode;
    use qsl_semantics::model::dispatch::GeneralizationClosure;
    use qsl_semantics::model::population::{
        admit_binding, lookup, AdmissionMeter, AdmissionOutcome, LookupKey, LookupOutcome,
        PopulationAdmissionLimits, PopulationDocument, PopulationMember,
    };
    use quire_exact::{LimitKind, Meter};

    let (view, object_type, population_key) = synthetic_model("tc-792-lookup");
    let document = PopulationDocument {
        model_identity: view.model_selection().identity.clone(),
        members: vec![PopulationMember {
            object: "present".to_owned(),
            type_identity: object_type.clone(),
            field_values: Vec::new(),
        }],
    };
    let mut admission = AdmissionMeter::new(PopulationAdmissionLimits::UNLIMITED);
    let AdmissionOutcome::Admitted(binding) = admit_binding(
        &view,
        &document,
        &population_key,
        GeneralizationClosure::Closed,
        Some(1),
        &mut admission,
    ) else {
        panic!("the lookup population admits");
    };
    let universe = *binding.universe();
    let type_identity = view.type_identities()[&object_type];
    let key = |object: &[u8]| LookupKey {
        static_type: object_type.clone(),
        universe: universe.as_bytes().to_vec(),
        type_identity,
        object: object.to_vec(),
    };
    let mut present_meter = Meter::new(default_accounting(1_000_000));
    let present = lookup(
        &binding,
        &object_type,
        &key(b"present"),
        AbsenceMode::Refused,
        &mut present_meter,
    );
    assert!(matches!(
        present,
        LookupOutcome::Completed(Some(reference))
            if reference.key().object == "present"
    ));
    assert!(present_meter.consumed(LimitKind::WorkUnits) > 0);

    let mut absent_meter = Meter::new(default_accounting(1_000_000));
    let absent = lookup(
        &binding,
        &object_type,
        &key(b"absent"),
        AbsenceMode::Refused,
        &mut absent_meter,
    );
    assert!(matches!(
        absent,
        LookupOutcome::Refused(refusal)
            if matches!(refusal.cause, ModelRefusalCause::AbsentKey { .. })
    ));

    let malformed = lookup(
        &binding,
        &object_type,
        &key(&[0xff, 0xfe]),
        AbsenceMode::Refused,
        &mut Meter::new(default_accounting(1_000_000)),
    );
    assert!(matches!(
        malformed,
        LookupOutcome::Refused(refusal)
            if matches!(refusal.cause, ModelRefusalCause::AbsentKey { ref key, .. } if key == &vec![0xff, 0xfe])
    ));

    let exhausted = lookup(
        &binding,
        &object_type,
        &key(b"present"),
        AbsenceMode::Refused,
        &mut Meter::new(default_accounting(0)),
    );
    assert!(matches!(exhausted, LookupOutcome::Incomplete(_)));
}

/// FR-100-AC-1/AC-7 (TC-450 step 1, TC-452 step 1): `seven()` completes
/// integer `7`, and the returned `package_id` equals the composed operations'.
#[trace("TC-452", "FR-100-AC-7")]
#[test]
fn tc_452_seven_completes_and_agrees_with_compile() {
    let compiled = compose(
        source(),
        "spine-run.native",
        FIXTURE.as_bytes(),
        &BTreeMap::new(),
        &DependencyInput::default(),
        SpineLimits::default(),
    )
    .unwrap();
    let (package_id, outcome) = run(
        source(),
        "spine-run.native",
        FIXTURE.as_bytes(),
        &BTreeMap::new(),
        &DependencyInput::default(),
        SpineLimits::default(),
        &call("seven", Vec::new()),
        &qsl_semantics::model::object_environment::ObjectEnvironment::default(),
        &Cancel::new(),
    )
    .unwrap();
    assert_eq!(package_id, compiled.emitted.package_id());
    match outcome {
        CallOutcome::Completed(CallValue::Integer(value)) => {
            assert_eq!(value.to_string(), "7");
        }
        other => panic!("expected a completed integer, got {other:?}"),
    }
}

/// FR-100-AC-3 (TC-450 step 6): a `1-draft` request supplying both
/// `libraries` and `models` runs, exit 0. `spine-run.native`'s `id` function
/// takes no library or model, so this exercises the two inputs being wired
/// through unused rather than the compiled program actually depending on
/// them; TC-450 step 6 at the CLI layer exercises the request shape this
/// unit test exercises the plumbing for.
#[trace("TC-450", "FR-100-AC-3")]
#[test]
fn tc_450_step_6_libraries_and_models_both_present_runs() {
    let outcome = run(
        source(),
        "spine-run.native",
        FIXTURE.as_bytes(),
        &BTreeMap::new(),
        &DependencyInput::default(),
        SpineLimits::default(),
        &call("seven", Vec::new()),
        &qsl_semantics::model::object_environment::ObjectEnvironment::default(),
        &Cancel::new(),
    )
    .unwrap()
    .1;
    assert!(matches!(outcome, CallOutcome::Completed(_)));
}

/// FR-100-AC-4/AC-7 (TC-451 step 1): `lt` binds `b` before `a` by name,
/// `flag`/`id` complete their declared kind.
#[trace("TC-451", "FR-100-AC-4")]
#[trace("TC-452", "FR-100-AC-7")]
#[test]
fn tc_451_step_1_arguments_bind_by_declared_name() {
    let lt = run_fixture(FIXTURE, &call("lt", vec![arg("b", 3), arg("a", 5)])).unwrap();
    assert!(matches!(
        lt,
        CallOutcome::Completed(CallValue::Boolean(false))
    ));
    let flag = run_fixture(FIXTURE, &call("flag", vec![arg("b", 1)])).unwrap();
    assert!(matches!(
        flag,
        CallOutcome::Completed(CallValue::Boolean(true))
    ));
    let id = run_fixture(FIXTURE, &call("id", vec![arg("x", 4)])).unwrap();
    match id {
        CallOutcome::Completed(CallValue::Integer(value)) => assert_eq!(value.to_string(), "4"),
        other => panic!("expected a completed integer, got {other:?}"),
    }
}

/// FR-100-AC-4 (TC-451 step 2): a value outside the parameter's kind or
/// declared domain refuses `WrongValueKind` at position 0, whether found
/// before the call (`flag`, `px`) or at S6a admission (`id`). `corner`
/// (declared result `Point`, a record) also refuses before the call at all,
/// so it never reaches this admission path -- a distinct oracle from a
/// bad-argument refusal (FR-100-AC-5, `tc_451_step_5`).
#[trace("TC-451", "FR-100-AC-4")]
#[test]
fn tc_451_step_2_wrong_value_kind_names_position() {
    for (function, argument) in [
        ("flag", arg("b", 2)),
        ("id", arg("x", 12)),
        ("px", arg("p", 1)),
    ] {
        let refusal = run_fixture(FIXTURE, &call(function, vec![argument])).unwrap_err();
        assert_eq!(refusal.stage(), Some("call"));
        assert_eq!(refusal.code(), Code::InvalidRuntimeInput);
        assert!(
            matches!(*refusal, RunRefusal::WrongValueKind { position: 0 }),
            "{refusal:?}"
        );
    }
}

/// FR-100-AC-4 (TC-451 step 3): an argument naming no parameter, a
/// parameter bound twice, and a parameter left unbound each refuse
/// `invalid_runtime_input` at stage `call`, naming the parameter.
#[trace("TC-451", "FR-100-AC-4")]
#[test]
fn tc_451_step_3_argument_binding_names_the_parameter() {
    let unknown = run_fixture(FIXTURE, &call("id", vec![arg("y", 4)])).unwrap_err();
    assert!(matches!(*unknown, RunRefusal::UnknownParameter { ref parameter } if parameter == "y"));
    let duplicate = run_fixture(FIXTURE, &call("id", vec![arg("x", 1), arg("x", 2)])).unwrap_err();
    assert!(
        matches!(*duplicate, RunRefusal::DuplicateArgument { ref parameter } if parameter == "x")
    );
    let unbound = run_fixture(FIXTURE, &call("id", Vec::new())).unwrap_err();
    assert!(matches!(*unbound, RunRefusal::UnboundParameter { ref parameter } if parameter == "x"));
    for refusal in [unknown, duplicate, unbound] {
        assert_eq!(refusal.stage(), Some("call"));
        assert_eq!(refusal.code(), Code::InvalidRuntimeInput);
    }
}

/// FND-010: binding every parameter is checked before any value is
/// converted. `lt(a, b)` with `a` bound to a wrong-kind literal but `b`
/// altogether unbound refuses `UnboundParameter { parameter: "b" }`, not
/// `WrongValueKind` for `a` -- deleting the "bind everything first" split in
/// `bind_arguments` (converting eagerly per argument, as the arguments
/// arrive) turns this red, since it would instead report `a`'s wrong kind
/// first.
#[trace("FR-100-AC-4", "TC-451")]
#[test]
fn fnd_010_unbound_parameter_is_reported_before_an_earlier_wrong_kind() {
    let refusal =
        run_fixture(FIXTURE, &call("lt", vec![arg("a", 9223372036854775807)])).unwrap_err();
    assert!(
        matches!(*refusal, RunRefusal::UnboundParameter { ref parameter } if parameter == "b"),
        "{refusal:?}"
    );
}

/// FR-100-AC-5 (TC-451 step 5): each malformed or unresolved `function`
/// string refuses `missing_declaration` at stage `call`, and a record
/// result refuses `unsupported_construct`, before any call.
#[trace("TC-451", "FR-100-AC-5")]
#[test]
fn tc_451_step_5_function_shape_and_lookup() {
    for name in ["nope", "module.seven", "", "seven.", "7x"] {
        let refusal = run_fixture(FIXTURE, &call(name, Vec::new())).unwrap_err();
        assert_eq!(refusal.stage(), Some("call"));
        assert_eq!(refusal.code(), Code::MissingDeclaration);
        assert!(
            matches!(*refusal, RunRefusal::MissingDeclaration { ref function } if function == name),
            "{refusal:?}"
        );
    }
    let corner = run_fixture(FIXTURE, &call("corner", vec![arg("p", 0)])).unwrap_err();
    assert_eq!(corner.stage(), Some("call"));
    assert_eq!(corner.code(), Code::UnsupportedConstruct);
    assert!(
        matches!(*corner, RunRefusal::UnsupportedResult { ref function } if function == "corner")
    );
}

/// FR-100-AC-5 (TC-451 step 6): a syntax error refuses at stage `source`
/// (`invalid_syntax`), and integer `/` with no `Rational` expected type
/// refuses at stage `check` (`ill_typed`).
#[trace("TC-451", "FR-100-AC-5")]
#[test]
fn tc_451_step_6_compile_refusals_carry_their_own_stage() {
    const SYNTAX_ERROR: &str = "language \"ix:native\" edition \"1-draft\";\nfunction (";
    let refusal = run_fixture(SYNTAX_ERROR, &call("f", Vec::new())).unwrap_err();
    assert_eq!(refusal.stage(), Some("source"));
    assert_eq!(refusal.code(), Code::InvalidSyntax);

    const ILL_TYPED: &str = "language \"ix:native\" edition \"1-draft\";\n\
        profile v = \"quire.value.complete/v1\";\n\
        type Digit = Int[0, 9];\n\
        function inv using v(x: Digit): Boolean pure { 1 / x > 0 }\n";
    let refusal = run_fixture(ILL_TYPED, &call("inv", vec![arg("x", 1)])).unwrap_err();
    assert_eq!(refusal.stage(), Some("check"));
    assert_eq!(refusal.code(), Code::IllTyped);
}

/// FR-100-AC-6 (TC-451 step 7): `work_units` 0 exhausts before the
/// charge, reported `incomplete` at `work_units`.
#[trace("TC-451", "FR-100-AC-6")]
#[test]
fn tc_451_step_7_zero_work_units_is_incomplete() {
    let outcome = run_fixture(FIXTURE, &call("seven", vec![])).unwrap();
    assert!(matches!(outcome, CallOutcome::Completed(_)));
    let starved = Call {
        accounting: default_accounting(0),
        ..call("seven", Vec::new())
    };
    let outcome = run_fixture(FIXTURE, &starved).unwrap();
    let CallOutcome::Incomplete(incomplete) = outcome else {
        panic!("expected an incomplete outcome, got {outcome:?}");
    };
    assert_eq!(incomplete.limits_field(), "work_units");
}

fn run_limited(limits: SpineLimits, call: &Call) -> Result<CallOutcome, Box<RunRefusal>> {
    run(
        source(),
        "spine-run.native",
        FIXTURE.as_bytes(),
        &BTreeMap::new(),
        &DependencyInput::default(),
        limits,
        call,
        &qsl_semantics::model::object_environment::ObjectEnvironment::default(),
        &Cancel::new(),
    )
    .map(|(_, outcome)| outcome)
}

fn run_fixture_call(call: &Call) -> CallOutcome {
    run_fixture(FIXTURE, call).expect("the fixture compiles")
}

/// FR-277-AC-3 (TC-758 step 6): `execute` with `work_units` one below what
/// the call needs returns `Incomplete` carrying the limit kind, the
/// configured value, the counter at the failed charge and the field name.
#[trace("TC-758", "FR-277-AC-3")]
#[test]
fn an_execute_accounting_limit_carries_its_kind_bound_counter_and_field() {
    let with_work = |work_units: u64| Call {
        accounting: default_accounting(work_units),
        ..call("seven", Vec::new())
    };
    let needed = (0..=1_000)
        .find(|work| {
            matches!(
                run_fixture_call(&with_work(*work)),
                CallOutcome::Completed(_)
            )
        })
        .expect("the call completes under a small work bound");
    assert!(needed > 0, "the call charges work");
    let CallOutcome::Incomplete(incomplete) = run_fixture_call(&with_work(needed - 1)) else {
        panic!("one work unit short must be incomplete");
    };
    assert_eq!(
        incomplete.record.limit_kind,
        quire_exact::LimitKind::WorkUnits
    );
    assert_eq!(incomplete.record.limit, needed - 1);
    assert_eq!(incomplete.limits_field(), "work_units");
    assert!(incomplete.record.consumed <= incomplete.record.limit);
    assert!(
        incomplete.counter() > Integer::from(incomplete.record.limit),
        "the counter at the failed charge is past the bound"
    );
    assert_eq!(
        incomplete.counter(),
        Integer::from(incomplete.record.consumed).add(&incomplete.record.next_charge)
    );
}

/// FR-285-AC-5 (TC-769 step 5): a stage limit reached by `run`'s compile
/// refuses `stage_limit_exceeded` in category incomplete, exit 22, naming
/// the limits field and configured bound: the source bytes at 1 and a
/// checking node limit.
#[trace("TC-769", "FR-285-AC-5")]
#[test]
fn a_stage_limit_refusal_of_run_is_incomplete_and_exits_22() {
    use crate::spine::CheckingLimits;
    use qsl_foundation::diagnostic::{Category, LimitKind};
    use qsl_foundation::Setting;
    let mut source_limit = SpineLimits::default();
    source_limit.source.source_bytes = 1;
    let checking_limit = SpineLimits {
        checking: CheckingLimits::new(1),
        ..SpineLimits::default()
    };
    for (limits, field, kind) in [
        (source_limit, Setting::S1InputBytes, LimitKind::InputBytes),
        (checking_limit, Setting::S3Nodes, LimitKind::NodeCount),
    ] {
        let refusal = run_limited(limits, &call("seven", Vec::new()))
            .expect_err("a reached stage limit refuses");
        assert_eq!(refusal.code().as_str(), "stage_limit_exceeded");
        assert_eq!(refusal.category(), Category::Incomplete, "{field:?}");
        assert_eq!(refusal.category().exit_code(), 22, "{field:?}");
        let RunRefusal::Compile(compile) = *refusal else {
            panic!("expected a compile refusal for {field:?}");
        };
        let CompileRefusal::Limit(limit) = *compile else {
            panic!("expected a limit refusal for {field:?}");
        };
        assert_eq!(limit.setting(), field);
        assert_eq!(limit.kind(), kind);
        assert_eq!(limit.configured_bound(), 1);
    }
}

#[derive(Debug)]
struct StubUndefined(UndefinedReason);

impl UndefinedCoded for StubUndefined {
    fn undefined_record(&self) -> UndefinedRecord {
        UndefinedRecord {
            reason: self.0,
            fields: BTreeMap::new(),
        }
    }
}

/// A `qsl_semantics::model::refusal::ModelRefusalCause` as a family
/// `CatalogCoded`, exactly the shape `qsl-eval`'s own `ModelQueryRefusal`
/// wraps it in (that type is crate-private to `qsl-eval`, so this test
/// builds its own thin wrapper rather than reaching into it).
#[derive(Debug)]
struct ModelCause(ModelRefusalCause);

impl CatalogCoded for ModelCause {
    fn catalog_code(&self) -> CatalogCode {
        self.0.catalog_code()
    }

    fn catalog_fields(&self) -> Option<BTreeMap<&'static str, String>> {
        self.0.catalog_fields()
    }
}

/// TC-452's fixture unit `F`: three LF-terminated lines with a
/// `function f` whose body's literal `5` (path `[1]`) is at byte 132 to
/// 133, line 3 column 54 to 55.
const FIXTURE_F: &str = "language \"ix:native\" edition \"1-draft\";\nprofile v = \"quire.value.complete/v1\";\nfunction f using v(x: Int[0, 9]): Integer pure { x + 5 }\n";

fn fixture_f_location() -> Location {
    Location::at(
        Origin::Body {
            function: "f".to_owned(),
            index: 0,
        },
        &[1],
    )
}

fn evaluation(outcome: FamilyOutcome<Value>) -> qsl_eval::value::Evaluation {
    qsl_eval::value::Evaluation {
        outcome,
        location: Some(fixture_f_location()),
        losses: Vec::new(),
    }
}

/// FR-100-AC-9 (TC-452 step 4): the outcome mapping converts every S6a
/// outcome category the mapping table names, over the literal fixture unit
/// `F`.
#[trace("TC-452", "FR-100-AC-9")]
#[test]
fn tc_452_step_4_outcome_mapping_covers_every_category() {
    let compiled = compose(
        source(),
        "tc-452-f.native",
        FIXTURE_F.as_bytes(),
        &BTreeMap::new(),
        &DependencyInput::default(),
        SpineLimits::default(),
    )
    .unwrap();
    let graph = compiled.package.graph();
    let sources = vec![Source::read(
        source(),
        "tc-452-f.native",
        FIXTURE_F.as_bytes(),
        qsl_foundation::source::MAX_SOURCE_BYTES,
    )
    .unwrap()];

    let convert =
        |outcome: FamilyOutcome<Value>| convert_outcome(evaluation(outcome), graph, &sources);

    // Completed: each value kind.
    for (value, expected) in [
        (Value::Boolean(true), "true"),
        (Value::Boolean(false), "false"),
    ] {
        match convert(FamilyOutcome::Evaluated(Outcome::Completed(value))).unwrap() {
            CallOutcome::Completed(CallValue::Boolean(rendered)) => {
                assert_eq!(rendered.to_string(), expected);
            }
            other => panic!("{other:?}"),
        }
    }
    for (value, expected) in [(0i64, "0"), (-17, "-17")] {
        match convert(FamilyOutcome::Evaluated(Outcome::Completed(
            Value::Integer(Integer::from(value)),
        )))
        .unwrap()
        {
            CallOutcome::Completed(CallValue::Integer(rendered)) => {
                assert_eq!(rendered.to_string(), expected);
            }
            other => panic!("{other:?}"),
        }
    }
    let big = Integer::from(1i64).shifted_left(70);
    match convert(FamilyOutcome::Evaluated(Outcome::Completed(
        Value::Integer(big),
    )))
    .unwrap()
    {
        CallOutcome::Completed(CallValue::Integer(rendered)) => {
            assert_eq!(rendered.to_string(), "1180591620717411303424");
        }
        other => panic!("{other:?}"),
    }
    let reference = quire_exact::ObjectReference::new(
        UniverseId::from_digest([4; 32]),
        quire_exact::EffectiveId::from_digest([5; 32]),
        quire_exact::ObjectId::new("g1").unwrap(),
    );
    match convert(FamilyOutcome::Evaluated(Outcome::Completed(
        Value::Reference(reference.clone()),
    )))
    .unwrap()
    {
        CallOutcome::Completed(CallValue::Reference(rendered)) => assert_eq!(rendered, reference),
        other => panic!("{other:?}"),
    }

    let location = fixture_f_location();
    let assert_location = |actual: &Option<Location>| {
        assert_eq!(actual.as_ref(), Some(&location));
    };

    // CardinalityOutOfBound: a record, with fields and locus.
    for (violation, count, cause) in [
        (BoundViolation::AboveMaximum, 4u64, "above-maximum"),
        (BoundViolation::BelowMinimum, 0u64, "below-minimum"),
    ] {
        let refusal = Refusal::CardinalityOutOfBound {
            violation,
            kind: CollectionKind::Set,
            bound: CardinalityBound::new(1, 3).unwrap(),
            count,
        };
        match convert(FamilyOutcome::Evaluated(Outcome::Refused(refusal))).unwrap() {
            CallOutcome::Refused(CallRefusal::Record {
                code,
                fields,
                locus,
                location: got_location,
            }) => {
                assert_eq!(code, CatalogCode::new("cardinality_out_of_bound", cause));
                assert_eq!(
                    fields,
                    BTreeMap::from([
                        ("collection", "set".to_owned()),
                        ("bound", "[1, 3]".to_owned()),
                        ("count", count.to_string()),
                    ])
                );
                let locus = locus.expect("a record locus");
                assert_eq!(
                    locus.source_digest,
                    "sha256:0c84cc4af8de870892c28bae81b32ab5fb9e28c46fd5f41e2d3f4db50eca36eb"
                );
                assert_eq!(locus.span.start.byte, 132);
                assert_eq!(locus.span.start.line, 3);
                assert_eq!(locus.span.start.column, 54);
                assert_eq!(locus.span.end.byte, 133);
                assert_eq!(locus.span.end.line, 3);
                assert_eq!(locus.span.end.column, 55);
                assert_location(&got_location);
            }
            other => panic!("{other:?}"),
        }
    }

    // The ten kernel value refusals (TC-452 step 4): each is a
    // record with the refusal's own code and cause, its exact catalog
    // fields (TC-452's payloads), a locus and the location.
    {
        use quire_exact::{
            DecimalType, DivisionMember, IeeeFlag, IeeeFlags, IeeeWidth, InexactTarget, Integer,
            IntegerInterval, RationalDomain, RoundingMode, TextProfile, TextType,
        };
        let int_0_9 = || IntegerInterval::spanning(Integer::zero(), Integer::from(9_i64));
        let decimal_0_100 = || {
            Box::new(
                DecimalType::new(
                    Integer::zero(),
                    Integer::from(100_i64),
                    0,
                    2,
                    RoundingMode::Exact,
                )
                .unwrap(),
            )
        };
        let rational_neg5_5 = || {
            Box::new(
                RationalDomain::new(
                    IntegerInterval::spanning(Integer::from(-5_i64), Integer::from(5_i64)),
                    IntegerInterval::spanning(Integer::one(), Integer::from(12_i64)),
                )
                .unwrap(),
            )
        };
        let kernel_value_refusals = [
            (
                Refusal::InexactDecimal {
                    target: InexactTarget::Decimal(decimal_0_100()),
                },
                "inexact_decimal",
                "nonzero-discarded-digit",
                BTreeMap::from([("expected", "Decimal[0, 100; 0, 2]".to_owned())]),
            ),
            (
                Refusal::DecimalOutOfDomain {
                    target: decimal_0_100(),
                },
                "decimal_out_of_domain",
                "outside-domain",
                BTreeMap::from([("expected", "Decimal[0, 100; 0, 2]".to_owned())]),
            ),
            (
                Refusal::DivisionOutOfDomain {
                    domain: Box::new(int_0_9()),
                    member: DivisionMember::Quotient,
                },
                "division_out_of_domain",
                "quotient-outside-domain",
                BTreeMap::from([("expected", "Int[0, 9]".to_owned())]),
            ),
            (
                Refusal::DivisionOutOfDomain {
                    domain: Box::new(int_0_9()),
                    member: DivisionMember::Remainder,
                },
                "division_out_of_domain",
                "remainder-outside-domain",
                BTreeMap::from([("expected", "Int[0, 9]".to_owned())]),
            ),
            (
                Refusal::ModuloOutOfDomain {
                    domain: Box::new(int_0_9()),
                },
                "modulo_out_of_domain",
                "outside-domain",
                BTreeMap::from([("expected", "Int[0, 9]".to_owned())]),
            ),
            (
                Refusal::TextLengthOutOfDomain {
                    target: TextType::new(1, 8, TextProfile::BinaryUtf8).unwrap(),
                },
                "text_length_out_of_domain",
                "outside-domain",
                BTreeMap::from([("expected", "Text[1, 8; binary-utf8]".to_owned())]),
            ),
            (
                Refusal::IntegerOutOfDomain {
                    target: Box::new(int_0_9()),
                },
                "integer_out_of_domain",
                "outside-domain",
                BTreeMap::from([("expected", "Int[0, 9]".to_owned())]),
            ),
            (
                Refusal::RationalOutOfDomain {
                    target: rational_neg5_5(),
                },
                "rational_out_of_domain",
                "outside-domain",
                BTreeMap::from([("expected", "Rational[-5, 5; 1, 12]".to_owned())]),
            ),
            (
                Refusal::IeeeNotExact {
                    target: IeeeWidth::Binary64,
                    would_be: [IeeeFlag::Overflow, IeeeFlag::Inexact]
                        .into_iter()
                        .collect::<IeeeFlags>(),
                },
                "ieee_not_exact",
                "rounding-required",
                BTreeMap::from([
                    ("expected", "binary64".to_owned()),
                    ("flags", "overflow,inexact".to_owned()),
                ]),
            ),
            (
                Refusal::IeeeNanPayloadNotRepresentable {
                    target: IeeeWidth::Binary32,
                    source: IeeeWidth::Binary64,
                },
                "ieee_nan_payload_not_representable",
                "payload-exceeds-target",
                BTreeMap::from([
                    ("expected", "binary32".to_owned()),
                    ("actual", "binary64".to_owned()),
                ]),
            ),
            (
                Refusal::IeeeRationalOutOfDomain {
                    target: rational_neg5_5(),
                },
                "ieee_rational_out_of_domain",
                "outside-domain",
                BTreeMap::from([("expected", "Rational[-5, 5; 1, 12]".to_owned())]),
            ),
        ];
        for (refusal, code, cause, expected_fields) in kernel_value_refusals {
            match convert(FamilyOutcome::Evaluated(Outcome::Refused(refusal.clone()))).unwrap() {
                CallOutcome::Refused(CallRefusal::Record {
                    code: got_code,
                    fields,
                    locus,
                    location: got_location,
                }) => {
                    assert_eq!(got_code, CatalogCode::new(code, cause), "{refusal:?}");
                    assert_eq!(fields, expected_fields, "{refusal:?}");
                    let locus = locus.expect("a record locus");
                    assert_eq!(
                        locus.source_digest,
                        "sha256:0c84cc4af8de870892c28bae81b32ab5fb9e28c46fd5f41e2d3f4db50eca36eb"
                    );
                    assert_eq!(locus.span.start.byte, 132);
                    assert_eq!(locus.span.end.byte, 133);
                    assert_location(&got_location);
                }
                other => panic!("{refusal:?}: {other:?}"),
            }
        }
    }

    // ForeignReference: a record, with `required`/`supplied` fields
    // and a locus -- no longer bare, now that the kernel variant carries both
    // universes.
    {
        let required = UniverseId::from_digest([0x01; 32]);
        let supplied = UniverseId::from_digest([0x02; 32]);
        let refusal = Refusal::ForeignReference { required, supplied };
        match convert(FamilyOutcome::Evaluated(Outcome::Refused(refusal))).unwrap() {
            CallOutcome::Refused(CallRefusal::Record {
                code,
                fields,
                locus,
                location: got_location,
            }) => {
                assert_eq!(
                    code,
                    CatalogCode::new("foreign_reference", "foreign-universe")
                );
                assert_eq!(
                    fields,
                    BTreeMap::from([("required", "01".repeat(32)), ("supplied", "02".repeat(32)),])
                );
                let locus = locus.expect("a record locus");
                assert_eq!(
                    locus.source_digest,
                    "sha256:0c84cc4af8de870892c28bae81b32ab5fb9e28c46fd5f41e2d3f4db50eca36eb"
                );
                assert_eq!(locus.span.start.byte, 132);
                assert_eq!(locus.span.end.byte, 133);
                assert_location(&got_location);
            }
            other => panic!("{other:?}"),
        }
    }

    // `CheckedInvariant`: an internal failure, not a `refused` outcome.
    let err = convert(FamilyOutcome::Evaluated(Outcome::Refused(
        Refusal::CheckedInvariant {
            cause: quire_exact::CheckedInvariantCause::EqualityOperandSourceNotAdmitted,
        },
    )))
    .unwrap_err();
    assert!(matches!(
        *err,
        RunRefusal::Fault(ref fault)
            if fault.stage() == "S6a" && fault.invariant() == "checked-program-invariant"
    ));

    for cause in qsl_semantics::check::received_kernel_causes() {
        let err = convert(FamilyOutcome::Evaluated(Outcome::Refused(
            Refusal::CheckedInvariant { cause },
        )))
        .unwrap_err();
        assert!(matches!(*err, RunRefusal::Fault(ref fault)
            if fault.stage() == "S6a"
                && fault.invariant() == "checked-program-invariant"
                && fault.kernel_cause() == Some(cause)));
    }

    // Kernel undefined: each of the five reasons.
    for (reason, spelling) in [
        (Undefined::DivisionByZero, "division-by-zero"),
        (Undefined::IeeeNotFinite, "ieee-not-finite"),
        (Undefined::EmptyReduction, "empty-reduction"),
        (Undefined::NoneValue, "none-value"),
        (Undefined::SumOutOfDomain, "sum-out-of-domain"),
    ] {
        match convert(FamilyOutcome::Evaluated(Outcome::Undefined(reason))).unwrap() {
            CallOutcome::Undefined { reason } => assert_eq!(reason, spelling),
            other => panic!("{other:?}"),
        }
    }

    // Incomplete at `work_units`.
    let incomplete = Incomplete {
        limit_kind: quire_exact::LimitKind::WorkUnits,
        limit: 0,
        consumed: 0,
        next_charge: Integer::from(1i64),
        charge_point: quire_exact::ChargePoint::IntegerArithmeticOperands,
    };
    match convert(FamilyOutcome::Evaluated(Outcome::Incomplete(incomplete))).unwrap() {
        CallOutcome::Incomplete(incomplete) => {
            assert_eq!(incomplete.limits_field(), "work_units");
        }
        other => panic!("{other:?}"),
    }

    // `FamilyResult::Refused` with an FR-096 key-table row: `AbsentKey`.
    let absent_key = ModelCause(ModelRefusalCause::AbsentKey {
        binding: "people".to_owned(),
        key: b"p7".to_vec(),
    });
    match convert(FamilyOutcome::FamilyEvaluated(FamilyResult::Refused(
        Box::new(absent_key),
    )))
    .unwrap()
    {
        CallOutcome::Refused(CallRefusal::Record { code, fields, .. }) => {
            assert_eq!(
                code,
                CatalogCode::new("invalid_runtime_input", "absent-key")
            );
            assert_eq!(
                fields,
                BTreeMap::from([("binding", "people".to_owned()), ("key", "p7".to_owned()),])
            );
        }
        other => panic!("{other:?}"),
    }

    // `FamilyResult::Refused` with a row: `ForeignUniverse`.
    let foreign_universe = ModelCause(ModelRefusalCause::ForeignUniverse {
        actual: vec![0x01; 32],
        expected: UniverseId::from_digest([0x02; 32]),
    });
    match convert(FamilyOutcome::FamilyEvaluated(FamilyResult::Refused(
        Box::new(foreign_universe),
    )))
    .unwrap()
    {
        CallOutcome::Refused(CallRefusal::Record { code, fields, .. }) => {
            assert_eq!(
                code,
                CatalogCode::new("foreign_reference", "foreign-universe")
            );
            assert_eq!(
                fields,
                BTreeMap::from([("required", "02".repeat(32)), ("supplied", "01".repeat(32)),])
            );
        }
        other => panic!("{other:?}"),
    }

    // `FamilyResult::Refused` with no row: `TypeMismatch`, `AncestorSteps`.
    let type_mismatch = ModelCause(ModelRefusalCause::TypeMismatch);
    match convert(FamilyOutcome::FamilyEvaluated(FamilyResult::Refused(
        Box::new(type_mismatch),
    )))
    .unwrap()
    {
        CallOutcome::Refused(CallRefusal::Family { code, .. }) => {
            assert_eq!(code, CatalogCode::new("ill_typed", "type-mismatch"));
        }
        other => panic!("{other:?}"),
    }
    let ancestor_steps = ModelCause(ModelRefusalCause::ancestor_steps(
        DeclarationKey {
            package: "test/orders".to_owned(),
            node: "n".to_owned(),
        },
        5,
        qsl_foundation::Setting::ModelAncestorSteps,
    ));
    match convert(FamilyOutcome::FamilyEvaluated(FamilyResult::Refused(
        Box::new(ancestor_steps),
    )))
    .unwrap()
    {
        CallOutcome::Refused(CallRefusal::Family { code, .. }) => {
            assert_eq!(
                code,
                CatalogCode::new("resource_exhausted", "ancestor-steps")
            );
        }
        other => panic!("{other:?}"),
    }

    // `FamilyResult::Undefined`: `precondition-false` and `absent-key`.
    for reason in [
        UndefinedReason::PreconditionFalse,
        UndefinedReason::AbsentKey,
    ] {
        let undefined = FamilyResult::Undefined(Box::new(StubUndefined(reason)));
        match convert(FamilyOutcome::FamilyEvaluated(undefined)).unwrap() {
            CallOutcome::Undefined { reason: rendered } => {
                assert_eq!(rendered, reason.as_str());
            }
            other => panic!("{other:?}"),
        }
    }
}

/// FR-100-AC-10 (TC-452 step 5, SR-749 FND-002): `sum<Pos>(x in q: x)` for
/// `Pos = Int[1, 9]`, checked under `CheckMode::Kernel` (TC-452 step 5's own
/// procedure -- a linked `sum` whose prefixes are not all proved members
/// refuses at checking, QSpec FR-145, so a real compiled program can never
/// reach this outcome; only a standalone expression checked this way can)
/// and evaluated for real, not a hand-built `Evaluation` unlike
/// [`tc_452_step_4_outcome_mapping_covers_every_category`], then converted
/// by the real `convert_outcome`.
///
/// An empty `q` leaves the seed `0` outside `Pos`, and `q` holding `9, 9`
/// leaves the running total `18` outside `Pos`: both are
/// `Outcome::Undefined(Undefined::SumOutOfDomain)`, located at the `sum`
/// node (`checked.root()`'s own location, never a summand's -- FR-100: "a
/// failing addition's running total at the sum node"), and the real
/// `convert_outcome` converts each to `CallOutcome::Undefined { reason:
/// "sum-out-of-domain" }`. `q` holding `4` completes with integer `4`
/// (TC-452 step 5's own third case).
///
/// Each undefined outcome's FR-285 exit code is 10 and the completion's 0.
/// Composes with `undefined_kernel_reasons_render_and_exit_10`
/// (`src/command/output.rs`), which proves every `CallOutcome::Undefined {
/// reason: "sum-out-of-domain" }` renders `{"kind": "undefined", "reason":
/// "sum-out-of-domain"}` and exits 10 -- the root crate names no
/// `qsl_eval` path (FR-100-AC-8, TC-452 step 2/3), so a real `Evaluation`
/// can only be produced here, never there.
#[trace("TC-786", "FR-100-AC-10")]
#[test]
fn tc_786_sum_over_pos_is_sum_out_of_domain_exit_10_or_completes_exit_0() {
    use qsl_eval::value::CheckedPackageEvaluation;
    use qsl_forms::Expression;
    use qsl_semantics::check::PackageDeclarations;
    use quire_exact::{CardinalityBound, CollectionType, Integer, IntegerInterval, Meter};
    use quire_semantic_value::checking::{CheckMode, CheckingLimits};

    let pos = ValueType::Int(IntegerInterval::spanning(
        Integer::one(),
        Integer::from(9_i64),
    ));
    let graph = PackageDeclarations {
        aliases: vec![("Pos".to_owned(), pos.clone())],
        ..PackageDeclarations::new(
            qsl_semantics::check::fixture_source(),
            qsl_foundation::IdentityLimits::default(),
        )
    }
    .check(CheckingLimits::default())
    .unwrap();
    let package = CheckedPackage::link(graph);

    let q_type = ValueType::collection(CollectionType::new(
        CollectionKind::Sequence,
        pos,
        Some(CardinalityBound::new(0, 2).unwrap()),
    ));
    let expression = Expression::sum(
        qsl_forms::DeclaredName {
            name: "Pos".to_owned(),
            span: qsl_foundation::Span { start: 0, end: 0 },
        },
        "x".to_owned(),
        Expression::name("q".to_owned()),
        Expression::name("x".to_owned()),
    );
    let checked = package
        .graph()
        .check_expression(
            vec![("q".to_owned(), q_type.clone())],
            &expression,
            None,
            CheckMode::Kernel,
            CheckingLimits::default(),
        )
        .unwrap();
    let sum_node = checked.root().location().clone();

    let ValueType::Collection(q_collection_type) = &q_type else {
        unreachable!("q_type is always a collection");
    };
    let q = |elements: Vec<i64>| {
        let elements = elements
            .into_iter()
            .map(|value| Value::Integer(Integer::from(value)))
            .collect();
        quire_exact::form_collection(
            q_collection_type,
            elements,
            &mut Meter::new(default_accounting(u64::MAX)),
        )
        .unwrap()
        .completed()
        .expect("q admits its own declared bound")
    };
    let evaluate = |q_value: Value| {
        package
            .evaluate(
                &checked,
                vec![q_value],
                &ObjectEnvironment::default(),
                &mut Meter::new(default_accounting(u64::MAX)),
            )
            .unwrap()
    };

    for elements in [Vec::<i64>::new(), vec![9, 9]] {
        let evaluation = evaluate(q(elements.clone()));
        assert!(
            matches!(
                evaluation.outcome,
                FamilyOutcome::Evaluated(Outcome::Undefined(Undefined::SumOutOfDomain))
            ),
            "{elements:?}: {:?}",
            evaluation.outcome
        );
        assert_eq!(
            evaluation.location.as_ref(),
            Some(&sum_node),
            "{elements:?}: located at the sum node"
        );
        let outcome = convert_outcome(evaluation, package.graph(), &[]).unwrap();
        assert_eq!(outcome.category().exit_code(), 10, "{elements:?}");
        match outcome {
            CallOutcome::Undefined { reason } => assert_eq!(reason, "sum-out-of-domain"),
            other => panic!("{elements:?}: {other:?}"),
        }
    }

    let completed = convert_outcome(evaluate(q(vec![4])), package.graph(), &[]).unwrap();
    assert_eq!(completed.category().exit_code(), 0);
    match completed {
        CallOutcome::Completed(CallValue::Integer(value)) => assert_eq!(value.to_string(), "4"),
        other => panic!("{other:?}"),
    }
}

/// FR-100 "Internal failure at S6a" (SR-674 FND-013): `convert_call_failure`
/// maps `CallFailure::Input(WrongValueKind)` to its own typed refusal, every
/// other `InputRefusal` (already admitted before S6a, so never actually
/// reachable) to an internal fault naming that invariant, and
/// `CallFailure::Fault` straight through, unchanged.
#[test]
fn convert_call_failure_maps_wrong_value_kind_and_forwards_faults() {
    use qsl_eval::value::CallFailure;
    use quire_semantic_value::call::InputRefusal;

    match *convert_call_failure(CallFailure::Input(InputRefusal::WrongValueKind {
        parameter: 2,
    })) {
        RunRefusal::WrongValueKind { position } => assert_eq!(position, 2),
        other => panic!("{other:?}"),
    }

    match *convert_call_failure(CallFailure::Input(InputRefusal::UnknownFunction(
        "ghost".to_owned(),
    ))) {
        RunRefusal::Fault(fault) => {
            assert_eq!(fault.stage(), "call");
            assert_eq!(
                fault.invariant(),
                "spine-run-supplies-admitted-name-and-arity"
            );
        }
        other => panic!("{other:?}"),
    }

    match *convert_call_failure(CallFailure::Fault(InternalFault::new(
        "S6a",
        "checked-program-invariant",
    ))) {
        RunRefusal::Fault(fault) => {
            assert_eq!(fault.stage(), "S6a");
            assert_eq!(fault.invariant(), "checked-program-invariant");
        }
        other => panic!("{other:?}"),
    }
}

/// SR-748 FND-001: `convert_refusal`'s `record == None && fallback == None`
/// branch (call.rs:513-518) is unreachable from `convert_outcome` today (see
/// its own doc comment), but is built directly here -- nothing else can
/// reach it -- to prove it returns a typed `InternalFault`, stage `call`,
/// invariant `kernel-refusal-with-no-record`, rather than panicking.
#[test]
fn convert_refusal_with_no_record_and_no_fallback_is_a_typed_fault() {
    match *convert_refusal(None, None, None, &[]).unwrap_err() {
        RunRefusal::Fault(fault) => {
            assert_eq!(fault.stage(), "call");
            assert_eq!(fault.invariant(), "kernel-refusal-with-no-record");
        }
        other => panic!("{other:?}"),
    }
}

/// FR-277-AC-1 (TC-758 step 1), driver view: each limits field is a
/// caller limit set on its own. Changing `s3.nodes` alone through the
/// re-exported `CheckingLimits` keeps the other s3 fields at their defaults,
/// and the run refuses at that bound naming `s3.nodes`.
#[trace("TC-758", "FR-277-AC-1")]
#[test]
fn changing_s3_nodes_alone_keeps_the_other_checking_limits() {
    use crate::spine::CheckingLimits;
    use qsl_foundation::Setting;
    let defaults = CheckingLimits::default();
    let limits = SpineLimits {
        checking: defaults.with_nodes(1),
        ..SpineLimits::default()
    };
    assert_eq!(limits.checking.nodes(), 1);
    assert_eq!(limits.checking.input_bytes(), defaults.input_bytes());
    assert_eq!(limits.checking.work_budget(), defaults.work_budget());
    let refusal = run_limited(limits, &call("seven", Vec::new())).expect_err("one node refuses");
    let RunRefusal::Compile(compile) = *refusal else {
        panic!("expected a compile refusal");
    };
    let CompileRefusal::Limit(limit) = *compile else {
        panic!("expected a limit refusal");
    };
    assert_eq!(limit.setting(), Setting::S3Nodes);
    assert_eq!(limit.configured_bound(), 1);
}

/// FR-277-AC-3 (TC-758 step 6): for a high-water counter the counter at the
/// failed charge is the denied size itself, not the consumed value plus it;
/// for a cumulative counter it is the sum.
#[trace("TC-758", "FR-277-AC-3")]
#[test]
fn counter_is_the_denied_size_for_a_high_water_limit_and_the_sum_for_a_cumulative_one() {
    let record = |limit_kind| Incomplete {
        limit_kind,
        limit: 8,
        consumed: 5,
        next_charge: Integer::from(9_u64),
        charge_point: quire_exact::ChargePoint::IntegerArithmeticOperands,
    };
    let incomplete = |limit_kind| CallIncomplete {
        record: record(limit_kind),
        location: None,
    };
    let high_water = incomplete(quire_exact::LimitKind::IntegerBits);
    assert_eq!(high_water.counter(), Integer::from(9_u64));
    assert_eq!(high_water.limits_field(), "integer_bits");
    let cumulative = incomplete(quire_exact::LimitKind::WorkUnits);
    assert_eq!(cumulative.counter(), Integer::from(14_u64));
}
