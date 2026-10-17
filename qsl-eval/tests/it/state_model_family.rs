// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-300/301: source-compiled model forms and enclosing-family outcomes.

use std::collections::BTreeMap;

use ix_trace_rs::trace;
use qsl_eval::value::{CheckedPackageEvaluation, QualifiedName};
use qsl_foundation::diagnostic::{category_of, Category};
use qsl_package::CheckedPackage;
use qsl_semantics::check::{
    checked_dispatch_operation, CheckCause, CheckedGraph, DispatchRoot, DispatchTable, NodeKind,
    OperationClauses, PackageDeclarations,
};
use qsl_semantics::family::{FamilyKind, FamilyOutcome, FamilyResult};
use qsl_semantics::model::accounting::{Meter as ModelMeter, ModelNormalizationLimits};
use qsl_semantics::model::dispatch::GeneralizationClosure;
use qsl_semantics::model::domain_package::{DomainPackage, DomainPackageRecord, DomainPackageRef};
use qsl_semantics::model::intake::SelectedModel;
use qsl_semantics::model::key::DeclarationKey;
use qsl_semantics::model::normalize::{normalize, EffectiveView, NormalizeOutcome};
use qsl_semantics::model::object_environment::ObjectEnvironment;
use qsl_semantics::model::observation::{AdmittedObservations, DocumentRef};
use qsl_semantics::model::population::{
    admit_binding, AdmissionMeter, AdmissionOutcome, PopulationAdmissionLimits, PopulationDocument,
    PopulationMember,
};
use quire_exact::{
    CollectionKind, EffectiveId, FieldValue, IllTypedCause, Meter, ObjectId, ObjectReference,
    Outcome, Value, ValueType,
};
use quire_semantic_value::checking::CheckingLimits;
use quire_semantic_value::declaration::FieldRef;
use quire_semantic_value::object_closure::ObjectClosure;

const A: &str = "ix://test/orders/A";
const C: &str = "ix://test/orders/C";
const READY: &str = "ix://test/orders/A/ready";
const POPULATION: &str = "ix://test/orders/pop";

fn key(name: &str) -> DeclarationKey {
    DeclarationKey::fixture(name)
}

fn model_document() -> Vec<u8> {
    let source = include_str!("../../../examples/config-version/model.semantic-ir.json")
        .replace("example/config-version", "test/orders")
        .replace("ConfigVersion", "A")
        .replace("config_history", "pop")
        .replace("attemptUpdate", "ready")
        .replace("versionNumber", "flag")
        .replace("parent", "next");
    let mut document: serde_json::Value = serde_json::from_str(&source).unwrap();
    document["constructs"]
        .as_array_mut()
        .unwrap()
        .retain(|node| node["kind"]["name"] != "value_type");
    document["types"]
        .as_array_mut()
        .unwrap()
        .retain(|node| node["kind"]["name"] == "object_type");
    let a = &mut document["types"][0];
    a["fields"][0]["typeRef"] = serde_json::json!("ix://quire/native/Boolean");
    a["fields"][1]["presence"] = serde_json::json!("required");
    a["operations"][0]["frame"]["modifies"] = serde_json::json!([]);
    let mut c = a.clone();
    c["fields"] = serde_json::json!([]);
    c["operations"] = serde_json::json!([]);
    let c: serde_json::Value = serde_json::from_str(&c.to_string().replace(A, C)).unwrap();
    document["types"].as_array_mut().unwrap().push(c);
    document["populations"][0]["members"] = serde_json::json!([A]);
    serde_json::to_vec(&document).unwrap()
}

fn view() -> EffectiveView {
    let bytes = model_document();
    let document = qsl_semantics::model::intake::PackageDocument::parse(
        &bytes,
        qsl_foundation::IntakeLimits {
            input_bytes: u64::MAX,
        },
    )
    .expect("the existing Semantic IR fixture parses");
    let selection = DomainPackageRef {
        identity: "test/orders".to_owned(),
        version: "1.0.0".to_owned(),
        digest: document.jcs_digest(),
    };
    let mut records = qsl_semantics::model::intake::read_records(&selection.identity, &document)
        .expect("the actual model document is admitted");
    // Main's published bridge consumes a supplied source body. Intake
    // intentionally does not parse operation bodies; this fixture attaches
    // the source Body declaration below, without automatic AR1 linking.
    for record in &mut records {
        if let DomainPackageRecord::OperationMember(operation) = record {
            operation.has_body = true;
        }
    }
    match normalize(
        &DomainPackage::new(selection, records),
        ModelNormalizationLimits::default(),
    ) {
        NormalizeOutcome::Completed(view) => view,
        other => panic!("normalization must succeed: {other:?}"),
    }
}

fn object_type(view: &EffectiveView) -> EffectiveId {
    view.type_identities()[&key(A)]
}

/// Compile real source into declarations against an existing admitted model
/// interface. Supplying a linked dispatch table is the published bridge path;
/// linking dispatch automatically during unit compilation belongs to FR-302.
fn declarations(body: &str, guard: bool) -> (PackageDeclarations, EffectiveView, String) {
    let view = view();
    let selection = view.model_selection();
    let source = format!("language \"ix:native\" edition \"1-draft\";\nprofile v = \"quire.value.complete/v1\";\nmodel M = {:?} version {:?} digest \"sha256-jcs:{}\";\nfunction Guard using v(): Boolean pure {{ {guard} }}\nfunction Body using v(): Boolean pure {{ true }}\n{body}\n", selection.identity, selection.version, qsl_semantics::model::key::hex(&selection.digest));
    let parsed = qsl_cst::parse(
        qsl_foundation::SourceIdentity::new("test", "sm1", "fixture", "1"),
        "sm1.native",
        source.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 parses source");
    assert!(parsed.is_admissible(), "S1: {:?}", parsed.diagnostics());
    let unit = qsl_forms::build_unit(&parsed).expect("S2 builds source");
    let model_span = unit.selections().models[0].span;
    let mut declarations = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        unit,
        vec![SelectedModel {
            alias: "M".to_owned(),
            span: model_span,
            view: view.clone(),
            operation_clauses: Vec::new(),
            type_invariants: Vec::new(),
        }],
        vec![],
    )
    .expect("assembly succeeds before any S3 assertion");
    let a = object_type(&view);
    let mut clauses = OperationClauses::default();
    clauses.member.insert(key(READY), "ready".to_owned());
    clauses.parameters.insert(
        key(READY),
        vec![("self".to_owned(), ValueType::Reference(a))],
    );
    clauses.result.insert(key(READY), ValueType::Boolean);
    clauses.own_body.insert(
        key(READY),
        declarations
            .functions
            .iter()
            .find(|function| function.name == "Body")
            .unwrap()
            .body
            .clone(),
    );
    clauses.own_precondition.insert(
        key(READY),
        declarations
            .functions
            .iter()
            .find(|function| function.name == "Guard")
            .unwrap()
            .body
            .clone(),
    );
    let bridge = checked_dispatch_operation(
        &view,
        &DispatchRoot {
            key: key(READY),
            closure: GeneralizationClosure::Closed,
        },
        &clauses,
        declarations.source.clone(),
        &mut ModelMeter::new(ModelNormalizationLimits::default()),
        qsl_foundation::IdentityLimits::default(),
    )
    .expect("main's dispatch bridge links the declared query");
    let offset = declarations.functions.len();
    for mut function in bridge.functions {
        // The bridge normally supplies opaque forms plus resolved signatures.
        // These fixture declarations instead use the actual source model name.
        function.parameters = vec![(
            "self".to_owned(),
            qsl_forms::TypeForm::name("M::A", model_span),
        )];
        function.result = qsl_forms::TypeForm::builtin(qsl_forms::BuiltinType::Boolean, model_span);
        declarations.functions.push(function);
    }
    declarations.model_clauses.extend(
        bridge
            .model_clauses
            .into_iter()
            .map(|(index, clause)| (index + offset, clause)),
    );
    declarations.dispatch_operations = bridge.dispatch_operations;
    declarations.dispatch_tables = bridge
        .dispatch_tables
        .into_iter()
        .map(|table| {
            let mut candidate = table
                .linked_for(&a)
                .expect("one concrete model type")
                .clone();
            candidate.body += offset;
            candidate.precondition = candidate.precondition.map(|index| index + offset);
            for index in &mut candidate.precondition_clauses {
                *index += offset;
            }
            DispatchTable::new(vec![(a, candidate)], table.candidate_count())
        })
        .collect();
    (declarations, view, source)
}

/// FR-120-AC-9's fixture: the checked package of `body` over this dispatch
/// model, and the domain package bytes it was checked against.
pub(crate) fn simulation_package(
    body: &str,
    guard: bool,
) -> (qsl_package::CheckedPackage, BTreeMap<[u8; 32], Vec<u8>>) {
    let (declarations, _, _) = declarations(body, guard);
    let graph = declarations
        .check(CheckingLimits::default())
        .expect("S3 checks source");
    let document = model_document();
    (
        qsl_package::CheckedPackage::link(graph),
        qsl_semantics::model::intake::package_input([document.as_slice()]),
    )
}

fn graph(body: &str, guard: bool) -> (CheckedGraph, EffectiveView) {
    let (declarations, view, _) = declarations(body, guard);
    (
        declarations
            .check(CheckingLimits::default())
            .expect("S3 checks source"),
        view,
    )
}

fn function<'a>(graph: &'a CheckedGraph, name: &str) -> qsl_semantics::check::CheckedNode<'a> {
    graph
        .function_by_identity(graph.function_identity(name).unwrap())
        .unwrap()
        .body
}

#[trace("TC-790", "FR-300-AC-1")]
#[test]
fn five_source_model_forms_have_the_declared_nodes_and_types() {
    let (attribute, view) = graph(
        "function F using v(r: M::A): Boolean pure { deref(r).flag }",
        true,
    );
    let root = function(&attribute, "F");
    assert_eq!(root.value_type(), &ValueType::Boolean);
    assert!(
        matches!(root.kind(), NodeKind::Attribute { field, .. } if field == &FieldRef::new(object_type(&view), "flag"))
    );

    let (instances, view) = graph(
        "function F using v(p: Population<M::A>[3]): Set<M::A>[0,3] pure { allInstances<M::A>(p) }",
        true,
    );
    let root = function(&instances, "F");
    assert!(matches!(root.kind(), NodeKind::AllInstances { .. }));
    let ValueType::Collection(result) = root.value_type() else {
        panic!("Set result");
    };
    assert_eq!(result.kind(), CollectionKind::Set);
    assert_eq!(result.element(), &ValueType::Reference(object_type(&view)));
    assert_eq!(
        (
            result.bound().unwrap().minimum(),
            result.bound().unwrap().maximum()
        ),
        (0, 3)
    );

    let (lookup, view) = graph("function F using v(p: Population<M::A>[3], r: M::A): M::A pure { lookup<M::A>(p,r) absent refused }", true);
    let root = function(&lookup, "F");
    assert!(matches!(root.kind(), NodeKind::Lookup { .. }));
    assert_eq!(root.value_type(), &ValueType::Reference(object_type(&view)));

    let (reaches, view) = graph(
        "invariant F using v on M::A at current { reaches(self,self,next) }",
        true,
    );
    let root = reaches.state_clause("F").unwrap().body();
    assert_eq!(root.value_type(), &ValueType::Boolean);
    assert!(
        matches!(root.kind(), NodeKind::Reaches { edge, .. } if edge == &FieldRef::new(object_type(&view), "next"))
    );

    let (dispatch, view) = graph("pre F using v on M::A::ready { self.ready() }", true);
    let root = dispatch.state_clause("F").unwrap().body();
    assert_eq!(root.value_type(), &ValueType::Boolean);
    let NodeKind::Dispatch {
        operation, table, ..
    } = root.kind()
    else {
        panic!("dispatch node");
    };
    let selected = &dispatch.scope().dispatch_operations()[*operation];
    assert_eq!(selected.receiver_type, object_type(&view));
    assert_eq!(selected.member, "ready");
    assert_eq!(selected.table, *table);
    let table = &dispatch.dispatch_tables()[*table];
    assert_eq!(table.candidate_count(), 1);
    let candidate = table.linked_for(&object_type(&view)).unwrap();
    assert_eq!(dispatch.function_state(candidate.body).unwrap().name, READY);
}

#[trace("TC-790", "FR-300-AC-2")]
#[test]
fn five_negative_source_units_refuse_at_s3_with_owned_raw_catalog_pairs_and_loci() {
    for (body, raw_code, raw_cause, authored) in [
        ("function F using v(r: M::A): Boolean pure { deref(r).missing }", "ill_typed", "type-mismatch", "missing"),
        // The undeclared target remains covered by
        // `tc790_all_instances_missing_target_type_refuses_during_assembly` at S2;
        // this row is TC-790's distinct admitted non-population S3 control.
        ("function F using v(r: M::A): Set<M::A>[0,3] pure { allInstances<M::A>(r) }", "ill_typed", "operator-ineligible", "r"),
        ("function F using v(p: Population<M::A>[3], r: M::C): M::A pure { lookup<M::A>(p,r) absent refused }", "ill_typed", "type-mismatch", "r"),
        ("invariant F using v on M::A at current { reaches(self,self,missing) }", "missing_declaration", "missing-name", "missing"),
        ("pre F using v on M::A::ready { self.missing() }", "missing_declaration", "missing-name", "missing"),
    ] {
        let (declarations, _, source) = declarations(body, true);
        let regions = declarations.regions();
        let refusals = declarations.check(CheckingLimits::default()).expect_err("S3 must refuse, after successful assembly");
        let [refusal] = refusals.as_slice() else { panic!("one refusal: {refusals:?}"); };
        let CheckCause::StateModel(cause) = &refusal.cause else { panic!("StateModel ownership: {refusal:?}"); };
        assert_eq!(cause.family(), FamilyKind::StateModel);
        let catalog = cause.catalog_code().unwrap();
        assert_eq!((catalog.code(), catalog.cause()), (raw_code, raw_cause));
        assert_eq!(catalog.prefix(), Some("state-model"));
        assert!(refusal.to_string().starts_with("state-model/"));
        let region = regions.refusal_region(refusal).expect("authored locus");
        let locus = &source[usize::try_from(region.start()).unwrap()..usize::try_from(region.end()).unwrap()];
        assert!(locus.contains(authored), "{authored}: {locus:?}");
    }
}

#[trace("FR-300-AC-1")]
#[test]
fn record_field_stays_a_value_and_standalone_deref_is_not_a_value() {
    let (record, _) = graph(
        "record R { flag: Boolean; } function F using v(r: R): Boolean pure { r.flag }",
        true,
    );
    assert!(matches!(
        function(&record, "F").kind(),
        NodeKind::Field { .. }
    ));
    let (declarations, _, _) =
        declarations("function F using v(r: M::A): M::A pure { deref(r) }", true);
    let refusals = declarations
        .check(CheckingLimits::default())
        .expect_err("deref needs a member read");
    assert!(!matches!(refusals[0].cause, CheckCause::StateModel(_)));
}

/// Available native source producers; this does not qualify the five-form obligation.
#[test]
fn available_source_attribute_and_reaches_have_declared_nodes_and_types() {
    let (attribute, view) = graph(
        "function F using v(r: M::A): Boolean pure { deref(r).flag }",
        true,
    );
    let root = function(&attribute, "F");
    assert_eq!(root.value_type(), &ValueType::Boolean);
    assert!(matches!(root.kind(), NodeKind::Attribute { field, .. }
        if field == &FieldRef::new(object_type(&view), "flag")));
    let (reaches, view) = graph(
        "invariant F using v on M::A at current { reaches(self,self,next) }",
        true,
    );
    let root = reaches.state_clause("F").unwrap().body();
    assert_eq!(root.value_type(), &ValueType::Boolean);
    assert!(matches!(root.kind(), NodeKind::Reaches { edge, .. }
        if edge == &FieldRef::new(object_type(&view), "next")));
}

/// Successful native source assembly followed by an owned S3 refusal.
#[trace("TC-790", "FR-300-AC-2")]
#[test]
fn available_source_attribute_and_reaches_negatives_have_owned_raw_pairs_and_loci() {
    for (body, authored, expected_cause, expected_code) in [
        (
            // FR-056-AC-9 preserves an ill_typed check refusal for a field
            // absent from the effective static type (deref(g).nope).
            "function F using v(r: M::A): Boolean pure { deref(r).missing }",
            "missing",
            CheckCause::IllTyped(IllTypedCause::TypeMismatch),
            ("ill_typed", "type-mismatch"),
        ),
        (
            // FR-104-AC-4: `flag` is a declared Boolean field, so it is
            // ineligible as a reference edge.
            "invariant F using v on M::A at current { reaches(self,self,flag) }",
            "reaches",
            CheckCause::IllTyped(IllTypedCause::OperatorIneligible),
            ("ill_typed", "operator-ineligible"),
        ),
        (
            // FR-300-AC-2: an edge absent from the admitted model is an
            // unresolved declaration, not a declared but ineligible edge.
            "invariant F using v on M::A at current { reaches(self,self,missing) }",
            "missing",
            CheckCause::MissingName("missing".to_owned()),
            ("missing_declaration", "missing-name"),
        ),
        (
            // FR-153-AC-4: a declared target with a non-population operand
            // reaches S3 and is refused as an ineligible operator use.
            "function F using v(r: M::A): Set<M::A>[0,3] pure { allInstances<M::A>(r) }",
            "r",
            CheckCause::IllTyped(IllTypedCause::OperatorIneligible),
            ("ill_typed", "operator-ineligible"),
        ),
        (
            // FR-153-AC-8: lookup's key type does not conform to its target.
            "function F using v(p: Population<M::A>[3], r: M::C): M::A pure { lookup<M::A>(p,r) absent refused }",
            "r",
            CheckCause::IllTyped(IllTypedCause::TypeMismatch),
            ("ill_typed", "type-mismatch"),
        ),
        (
            // FR-151-AC-2: a dispatched operation absent from the model.
            "pre F using v on M::A::ready { self.missing() }",
            "missing",
            CheckCause::MissingName("missing".to_owned()),
            ("missing_declaration", "missing-name"),
        ),
    ] {
        let (declarations, _, source) = declarations(body, true);
        let regions = declarations.regions();
        let refusals = declarations
            .check(CheckingLimits::default())
            .expect_err("S3 refusal");
        let [refusal] = refusals.as_slice() else {
            panic!("one refusal: {refusals:?}");
        };
        let CheckCause::StateModel(cause) = &refusal.cause else {
            panic!("StateModel ownership: {refusal:?}");
        };
        assert_eq!(cause.family(), FamilyKind::StateModel);
        assert_eq!(cause.cause, expected_cause);
        let code = cause.catalog_code().unwrap();
        assert_eq!((code.code(), code.cause()), expected_code);
        assert_eq!(code.prefix(), Some("state-model"));
        assert!(refusal.to_string().starts_with("state-model/"));
        let region = regions.refusal_region(refusal).expect("authored locus");
        let locus = &source
            [usize::try_from(region.start()).unwrap()..usize::try_from(region.end()).unwrap()];
        assert!(locus.contains(authored), "{authored}: {locus:?}");
    }
}

/// FR-091-AC-8: an allInstances target naming no admitted type is refused
/// during assembly, before the StateModel S3 check.
#[trace("TC-790", "FR-091-AC-8")]
#[test]
fn tc790_all_instances_missing_target_type_refuses_during_assembly() {
    let view = view();
    let selection = view.model_selection();
    let source = format!(
        "language \"ix:native\" edition \"1-draft\";\nprofile v = \"quire.value.complete/v1\";\nmodel M = {:?} version {:?} digest \"sha256-jcs:{}\";\nfunction F using v(p: Population<M::A>[3]): Set<M::A>[0,3] pure {{ allInstances<M::Missing>(p) }}\n",
        selection.identity,
        selection.version,
        qsl_semantics::model::key::hex(&selection.digest),
    );
    let parsed = qsl_cst::parse(
        qsl_foundation::SourceIdentity::new("test", "sm1", "fixture", "1"),
        "sm1.native",
        source.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 parses source");
    assert!(parsed.is_admissible(), "S1: {:?}", parsed.diagnostics());
    let unit = qsl_forms::build_unit(&parsed).expect("S2 builds source");
    let model_span = unit.selections().models[0].span;
    let refusal = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        unit,
        vec![SelectedModel {
            alias: "M".to_owned(),
            span: model_span,
            view,
            operation_clauses: Vec::new(),
            type_invariants: Vec::new(),
        }],
        vec![],
    )
    .expect_err("the unresolved allInstances target refuses at assembly");
    let [error] = refusal.errors.as_slice() else {
        panic!("one assembly error: {refusal:?}");
    };
    assert!(matches!(
        &error.cause,
        qsl_semantics::check::AssemblyCause::UnresolvedTypeName { name }
            if name == "M::Missing"
    ));
    assert_eq!(error.cause.code().as_str(), "missing_declaration");
    assert_eq!(error.cause.catalog_code().cause(), "missing-name");
    let authored = &source[error.span.start..error.span.end];
    assert_eq!(authored, "M::Missing");
}

fn objects(
    graph: &CheckedGraph,
    view: &EffectiveView,
) -> (ObjectEnvironment, ObjectReference, ObjectReference) {
    let a = object_type(view);
    let universe = view.object_universe_of(&key(A)).unwrap().identity();
    let present = ObjectReference::new(universe, a, ObjectId::new("present").unwrap());
    let absent = ObjectReference::new(universe, a, ObjectId::new("absent").unwrap());
    let world = [present.clone(), absent.clone()]
        .into_iter()
        .map(|reference| {
            (
                reference.clone(),
                vec![
                    ("flag", FieldValue::Present(Value::Boolean(true))),
                    ("next", FieldValue::Present(Value::Reference(reference))),
                ],
            )
        });
    let objects =
        ObjectEnvironment::new(ObjectClosure::new(graph.scope().types(), world, &[]).unwrap());
    (objects, present, absent)
}

#[trace("TC-792", "FR-301-AC-1")]
#[test]
fn source_lookup_function_carries_model_refusal_and_present_control_completes() {
    let (graph, view) = graph("function F using v(p: Population<M::A>[3], r: M::A): M::A pure { lookup<M::A>(p,r) absent refused }", true);
    let (objects, present, absent) = objects(&graph, &view);
    let document = PopulationDocument {
        model_identity: view.model_selection().identity.clone(),
        members: vec![PopulationMember {
            object: "present".to_owned(),
            type_identity: key(A),
            field_values: vec![],
        }],
    };
    let binding = match admit_binding(
        &view,
        &document,
        &key(POPULATION),
        GeneralizationClosure::Closed,
        Some(3),
        &mut AdmissionMeter::new(PopulationAdmissionLimits::UNLIMITED),
    ) {
        AdmissionOutcome::Admitted(binding) => binding,
        other => panic!("population admission: {other:?}"),
    };
    let population = Value::Population(binding.population_id());
    let objects = objects.with_population(binding).unwrap();
    let package = CheckedPackage::link(graph);
    let name = QualifiedName::unqualified("F").unwrap();
    let evaluation = package
        .call(
            &name,
            vec![population.clone(), Value::Reference(absent)],
            &objects,
            &mut Meter::new(qsl_semantics::check::SCALAR_LIMITS_UNLIMITED),
        )
        .unwrap();
    let FamilyOutcome::FamilyEvaluated(FamilyResult::Refused(cause)) = evaluation.outcome else {
        panic!("lookup must refuse: {:?}", evaluation.outcome);
    };
    let code = cause.catalog_code();
    assert_eq!(
        (code.code(), code.cause()),
        ("invalid_runtime_input", "absent-key")
    );
    assert_eq!(
        code.to_string(),
        "state-model/invalid_runtime_input/absent-key"
    );
    assert_eq!(category_of(&code), Some(Category::Refusal));
    assert_eq!(
        cause.refusal_record(None).unwrap().category(),
        Category::Refusal
    );
    let control = package
        .call(
            &name,
            vec![population, Value::Reference(present.clone())],
            &objects,
            &mut Meter::new(qsl_semantics::check::SCALAR_LIMITS_UNLIMITED),
        )
        .unwrap();
    assert!(
        matches!(control.outcome, FamilyOutcome::Evaluated(Outcome::Completed(Value::Reference(reference))) if reference == present)
    );
}

#[trace("TC-792", "FR-301-AC-2")]
#[test]
fn source_state_clause_retains_false_selected_precondition_as_model_undefined() {
    for guard in [false, true] {
        let (graph, view) = graph("pre F using v on M::A::ready { self.ready() }", guard);
        let observations = observations(&graph, &view);
        let package = CheckedPackage::link(graph);
        let report = package
            .evaluate_clause(
                &QualifiedName::unqualified("F").unwrap(),
                &observations,
                &mut Meter::new(qsl_semantics::check::SCALAR_LIMITS_UNLIMITED),
            )
            .unwrap();
        let rendered = format!("{:?}", report.evaluation.outcome);
        if guard {
            assert!(matches!(
                report.evaluation.outcome,
                FamilyOutcome::Evaluated(Outcome::Completed(Value::Boolean(true)))
            ));
        } else {
            let FamilyOutcome::FamilyEvaluated(FamilyResult::Undefined(cause)) =
                report.evaluation.outcome
            else {
                panic!(
                    "false effective precondition is undefined: {:?}",
                    report.evaluation.outcome
                );
            };
            assert_eq!(
                cause.undefined_record().reason.as_str(),
                "precondition-false"
            );
            assert!(
                rendered.contains("state-model/precondition-false"),
                "{rendered}"
            );
            assert_eq!(cause.catalog_prefix(), Some("state-model"));
            assert_eq!(cause.catalog_code(), "state-model/precondition-false");
            assert_eq!(cause.undefined_record().fields["operation"], "ready");
        }
    }
}

fn observations(graph: &CheckedGraph, view: &EffectiveView) -> AdmittedObservations {
    use qsl_semantics::model::observation::{
        admit_observations, ClauseFacts, ClauseSelection, ClauseSelectionInput, ObservationLimits,
        OperationFacts, Provisions, SelectedObject,
    };
    let model = view.model_selection();
    let bytes = serde_json::to_vec(&serde_json::json!({
        "format": "quire.state.snapshot/v1",
        "identity": {
            "authority": "test", "identity": "sm1-snapshot",
            "revision_namespace": "fixture", "revision": "1"
        },
        "observation": "pre",
        "model": {
            "identity": model.identity, "version": model.version,
            "digest": format!("sha256-jcs:{}", qsl_semantics::model::key::hex(&model.digest))
        },
        "populations": [{
            "population": POPULATION, "complete": true,
            "objects": [{
                "key": "present", "type": A,
                "fields": {
                    "flag": {"boolean": true},
                    "next": {"reference": {"population": POPULATION, "key": "present"}}
                }
            }]
        }]
    }))
    .unwrap();
    let document = quire_canonical::read(&bytes, u64::MAX).unwrap();
    let digest = *quire_canonical::sha256(&document, quire_canonical::Limits::new(u64::MAX))
        .unwrap()
        .as_bytes();
    let snapshot = DocumentRef {
        authority: "test".to_owned(),
        identity: "sm1-snapshot".to_owned(),
        revision_namespace: "fixture".to_owned(),
        revision: "1".to_owned(),
        digest,
    };
    let snapshots = BTreeMap::from([(digest, bytes)]);
    let invocations = BTreeMap::new();
    let model_bytes = model_document();
    let packages = qsl_semantics::model::intake::package_input([model_bytes.as_slice()]);
    let clause = graph.state_clause("F").unwrap();
    let facts = ClauseFacts {
        identity: clause.identity(),
        kind: clause.kind(),
        context: clause.context(),
        operation: clause.operation().map(|operation| OperationFacts {
            declaring: operation.declaring,
            declaration: operation.declaration.clone(),
        }),
        operations: Default::default(),
    };
    let selection = ClauseSelection {
        name: "F".to_owned(),
        input: ClauseSelectionInput::PreCall {
            snapshot,
            self_object: Some(SelectedObject {
                population: POPULATION.to_owned(),
                key: "present".to_owned(),
            }),
            parameters: BTreeMap::new(),
        },
    };
    admit_observations(
        graph.model_selections(),
        graph.scope().types(),
        &facts,
        &packages,
        ModelNormalizationLimits::default(),
        &Provisions {
            snapshots: &snapshots,
            invocations: &invocations,
        },
        &selection,
        ObservationLimits::default(),
    )
    .expect("snapshot admission completes before the semantic assertions")
}

#[trace("FR-300-AC-2")]
#[test]
fn source_reaches_context_refuses_before_operand_name_resolution() {
    let (declarations, _, _) = declarations(
        "function F using v(): Boolean pure { reaches(missing,missing,next) }",
        true,
    );
    let refusals = declarations
        .check(CheckingLimits::default())
        .expect_err("S3 refuses context");
    let [refusal] = refusals.as_slice() else {
        panic!("one refusal: {refusals:?}");
    };
    let CheckCause::StateModel(cause) = &refusal.cause else {
        panic!("model ownership: {refusal:?}");
    };
    let code = cause.catalog_code().unwrap();
    assert_eq!(
        (code.code(), code.cause()),
        ("ill_typed", "operator-ineligible")
    );
    assert_eq!(code.prefix(), Some("state-model"));
}

/// FR-153-AC-8, result typing: `allInstances<T>(p)` over `Population<T>[N]`
/// is `Set<Reference<T>>[0,N]`, including `N = 0`, and `lookup`'s present
/// result is `Reference<T>` under `undefined` and `refused` but
/// `Option<Reference<T>>` under `empty`, always typed to the queried `T`. The
/// source grammar spells every population type with its `[N]`, so the
/// unbounded binding of FR-153-AC-9 is reached through the checked-type API
/// and covered in `model_reference_queries.rs`, not here.
///
/// Mutation used (derived by reading, not executed): treating a zero maximum
/// as absent, widening the lookup result to `Option` in every mode, or typing
/// the result to the declared type of `r` instead of the queried `T` each
/// fails one assertion below.
#[trace("QSpec-TC-198", "QSpec-FR-153-AC-8")]
#[test]
fn population_queries_are_typed_to_the_queried_type_and_the_declared_maximum() {
    let (bounded, view) = graph(
        "function F using v(p: Population<M::A>[0]): Set<M::A>[0,0] pure { allInstances<M::A>(p) }",
        true,
    );
    let ValueType::Collection(result) = function(&bounded, "F").value_type() else {
        panic!("Set result");
    };
    assert_eq!(result.element(), &ValueType::Reference(object_type(&view)));
    assert_eq!(
        (
            result.bound().unwrap().minimum(),
            result.bound().unwrap().maximum()
        ),
        (0, 0),
        "a zero maximum is a declared maximum"
    );

    for mode in ["undefined", "refused"] {
        let (lookup, view) = graph(
            &format!("function F using v(p: Population<M::A>[3], r: M::A): M::A pure {{ lookup<M::A>(p,r) absent {mode} }}"),
            true,
        );
        assert_eq!(
            function(&lookup, "F").value_type(),
            &ValueType::Reference(object_type(&view)),
            "{mode}"
        );
    }
    let (empty, view) = graph(
        "function F using v(p: Population<M::A>[3], r: M::A): Option<M::A> pure { lookup<M::A>(p,r) absent empty }",
        true,
    );
    assert_eq!(
        function(&empty, "F").value_type(),
        &ValueType::Option(std::sync::Arc::new(ValueType::Reference(object_type(
            &view
        )))),
        "empty mode"
    );
}

/// FR-153-AC-8, binding admission: a `Population<T>[N]` whose `T` is not a
/// model object type refuses `ill_typed`/`type-mismatch`.
///
/// Mutation used (derived by reading, not executed): admitting any resolvable
/// type as the population target (dropping the `ValueType::Reference` match in
/// `population_target`) lets the record through and fails the refusal match.
#[trace("QSpec-TC-198", "QSpec-FR-153-AC-8")]
#[test]
fn a_population_binding_over_a_non_object_target_refuses_type_mismatch() {
    let view = view();
    let selection = view.model_selection();
    let source = format!(
        "language \"ix:native\" edition \"1-draft\";\nprofile v = \"quire.value.complete/v1\";\nmodel M = {:?} version {:?} digest \"sha256-jcs:{}\";\nrecord R {{ flag: Boolean; }}\nfunction F using v(p: Population<R>[3]): Boolean pure {{ true }}\n",
        selection.identity,
        selection.version,
        qsl_semantics::model::key::hex(&selection.digest),
    );
    let parsed = qsl_cst::parse(
        qsl_foundation::SourceIdentity::new("test", "sm1", "fixture", "1"),
        "sm1.native",
        source.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 parses source");
    assert!(parsed.is_admissible(), "S1: {:?}", parsed.diagnostics());
    let unit = qsl_forms::build_unit(&parsed).expect("S2 builds source");
    let model_span = unit.selections().models[0].span;
    let assembled = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        unit,
        vec![SelectedModel {
            alias: "M".to_owned(),
            span: model_span,
            view,
            operation_clauses: Vec::new(),
            type_invariants: Vec::new(),
        }],
        vec![],
    );
    let declarations = assembled.expect("a record target assembles; S3 decides the binding");
    let refusals = declarations
        .check(CheckingLimits::default())
        .expect_err("S3 must refuse a population over a non-object target");
    let [refusal] = refusals.as_slice() else {
        panic!("one refusal: {refusals:?}");
    };
    assert!(
        matches!(
            &refusal.cause,
            CheckCause::IllTyped(IllTypedCause::TypeMismatch)
        ),
        "{refusal:?}"
    );
}
