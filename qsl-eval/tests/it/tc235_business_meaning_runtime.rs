// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-235 runtime components and M15 context admission/evaluation vectors.
//!
//! The semantic binder owns K2 admission and charge assertions. These rows
//! exercise the evaluator boundary with the same record, event and lifecycle
//! meanings, keeping the runtime oracle independent of the binder's JSON walk.

use std::collections::BTreeMap;

use ix_trace_rs::trace;
use qsl_eval::value::{
    CheckedPackageEvaluation, InvocationBinding, ObservationIdentity, QualifiedName,
    RuntimeValuePath, ValuePathSubject,
};
use qsl_forms::{BinaryOperator, ClauseKind, Expression, FunctionDeclaration, TypeForm};
use qsl_foundation::selection::ProfileSelection;
use qsl_foundation::IntakeLimits;
use qsl_package::CheckedPackage;
use qsl_semantics::check::{CheckCause, LockEvidence, PackageDeclarations, WrongSnapshotCause};
use qsl_semantics::model::accounting::{ChargePoint, ModelNormalizationLimits};
use qsl_semantics::model::domain_package::{DomainPackage, DomainPackageRef};
use qsl_semantics::model::intake::{admit_unit, package_input, read_domain_package};
use qsl_semantics::model::key::SHA256_JCS_DIGEST_DOMAIN;
use qsl_semantics::model::normalize::{normalize_with_meter, EffectiveView, NormalizeOutcome};
use qsl_semantics::model::object_environment::ObjectEnvironment;
use qsl_semantics::model::observation::{
    admit_observations, AdmissionFailure, AdmittedObservations, ClauseFacts, ClauseSelection,
    ClauseSelectionInput, DocumentRef, ObservationLimits, OperationFacts, Provisions,
};
use qsl_semantics::value::{CatalogRole, DefinitionLock};
use quire_exact::{admit_text, TextPayload, TextProfile, TextType};
use quire_exact::{
    CardinalityBound, CollectionKind, CollectionType, EffectiveId, FieldValue, Integer, Meter,
    ObjectId, ObjectReference, OptionValue, Outcome, Presence, ScalarLimits, UniverseId, Value,
    ValueType,
};
use quire_semantic_value::checking::CheckingLimits;
use quire_semantic_value::declaration::{
    CompositeDeclaration, CompositeShape, FieldDeclaration, ObjectTypeDeclaration, TypeEnvironment,
};
use quire_semantic_value::object_closure::ObjectClosure;
use sha2::{Digest, Sha256};

const UNLIMITED: ScalarLimits = ScalarLimits {
    integer_bits: u64::MAX,
    decimal_digits: u64::MAX,
    scale_expansion: u64::MAX,
    text_input_bytes: u64::MAX,
    text_scalars: u64::MAX,
    normalized_scalars: u64::MAX,
    unit_edges: u64::MAX,
    value_occurrences: u64::MAX,
    work_units: u64::MAX,
    result_units: u64::MAX,
};

fn key(label: &str) -> quire_exact::NodeKey {
    quire_exact::NodeKey::from_digest(Sha256::digest(label.as_bytes()).into())
}

fn object_type(label: &str) -> EffectiveId {
    EffectiveId::from_digest(Sha256::digest(label.as_bytes()).into())
}

fn k2_order_with(fields: serde_json::Value, clauses: serde_json::Value) -> serde_json::Value {
    let mut document = crate::tc235_k2::k2();
    let order = document["types"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|ty| ty["identity"] == crate::tc235_k2::id("Order"))
        .expect("K2 Order declaration");
    order["fields"] = fields;
    order["clauses"] = clauses;
    document
}

fn lifecycle_document(requires: &str) -> serde_json::Value {
    let mut document = crate::tc235_k2::k2();
    let lifecycle = document["types"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|ty| ty["identity"] == crate::tc235_k2::id("OrderLifecycle"))
        .expect("K2 OrderLifecycle declaration");
    lifecycle["fields"]
        .as_array_mut()
        .unwrap()
        .push(crate::tc235_k2::field(
            "OrderLifecycle",
            "cancelled",
            "ix://quire/native/Boolean",
        ));
    lifecycle["clauses"]
        .as_array_mut()
        .expect("K2 OrderLifecycle clauses")
        .push(crate::tc235_k2::clause(
            "OrderLifecycle",
            "Consistent",
            "not (self.placed and self.cancelled)",
        ));
    let place = lifecycle["operations"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|operation| operation["identity"] == crate::tc235_k2::id("OrderLifecycle/place"))
        .expect("K2 OrderLifecycle/place operation");
    place["pre"] = serde_json::json!([{
        "language": "quire",
        "text": requires,
        "origin": crate::tc235_k2::origin("OrderLifecycle/place"),
    }]);
    place["post"] = serde_json::json!([{
        "language": "quire",
        "text": "self.placed and not self.cancelled",
        "origin": crate::tc235_k2::origin("OrderLifecycle/place"),
    }]);
    place["frame"]["modifies"] = serde_json::json!([crate::tc235_k2::id("OrderLifecycle/placed")]);
    let digest = crate::tc235_k2::jcs_digest(&document["types"]);
    document["source"]["digest"] = serde_json::json!(digest);
    document["package"]["manifestDigest"] = serde_json::json!(digest);
    document["package"]["lockDigest"] = serde_json::json!(digest);
    for construct in document["constructs"].as_array_mut().unwrap() {
        construct["manifestDigest"] = serde_json::json!(digest);
    }
    document
}

fn read_k2(document: &serde_json::Value) -> (DomainPackage, BTreeMap<[u8; 32], Vec<u8>>) {
    let bytes = serde_json::to_vec(document).expect("lifecycle fixture serializes");
    let offered = package_input([bytes.as_slice()]);
    let digest = *offered.keys().next().expect("one lifecycle package");
    let selection = DomainPackageRef {
        identity: crate::tc235_k2::PACKAGE.to_owned(),
        version: "1".to_owned(),
        digest,
    };
    let (selection, admitted) = qsl_semantics::model::intake::admit(
        &selection,
        SHA256_JCS_DIGEST_DOMAIN,
        &offered,
        IntakeLimits::default(),
    )
    .expect("S1 admits lifecycle fixture");
    (
        read_domain_package(selection, &admitted).expect("lifecycle fixture reads"),
        offered,
    )
}

type LifecycleFixture = (CheckedPackage, EffectiveView, BTreeMap<[u8; 32], Vec<u8>>);

fn lifecycle_package(
    document: &serde_json::Value,
    body: &str,
) -> Result<LifecycleFixture, Vec<qsl_semantics::check::CheckRefusal>> {
    let (domain, packages) = read_k2(document);
    let (outcome, meter) = normalize_with_meter(&domain, ModelNormalizationLimits::default());
    let NormalizeOutcome::Completed(view) = outcome else {
        panic!("M15 K2 normalization must admit: {outcome:?}");
    };
    assert_eq!(
        meter
            .admitted_charges()
            .iter()
            .filter(|charge| **charge == ChargePoint::NormalizeRecord)
            .count(),
        23,
        "TC-235 M15 normalize.record charge count"
    );
    let digest = packages.keys().next().expect("lifecycle package digest");
    let unit = format!(
        "language \"ix:native\" edition \"1-draft\";\nprofile v = \"quire.value.complete/v1\";\nmodel M = \"test/orders\" version \"1\" digest \"sha256-jcs:{}\";\n{}\n",
        qsl_semantics::model::key::hex(digest),
        body,
    );
    let parsed = qsl_cst::parse(
        qsl_foundation::SourceIdentity::new("test", "tc235-m15", "fixture", "1"),
        "tc235-m15.native",
        unit.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("M15 source parses");
    assert!(
        parsed.is_admissible(),
        "M15 source diagnostics: {:?}",
        parsed.diagnostics()
    );
    let built = qsl_forms::build_unit(&parsed).expect("M15 source builds");
    let models = admit_unit(
        &built.selections().models,
        &packages,
        ModelNormalizationLimits::default(),
    )
    .expect("M15 model selection admits");
    let declarations = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        built,
        models,
        Vec::new(),
    )
    .expect("M15 source assembles");
    let graph = declarations.check(CheckingLimits::default())?;
    Ok((CheckedPackage::link(graph), view, packages))
}

fn lifecycle_document_ref(label: &str, bytes: &[u8]) -> DocumentRef {
    let document = quire_canonical::read(bytes, u64::MAX).expect("lifecycle document reads");
    let digest = *quire_canonical::sha256(&document, quire_canonical::Limits::new(u64::MAX))
        .expect("lifecycle document hashes")
        .as_bytes();
    DocumentRef {
        authority: "test".to_owned(),
        identity: label.to_owned(),
        revision_namespace: "tc235".to_owned(),
        revision: "1".to_owned(),
        digest,
    }
}

fn lifecycle_identity(reference: &DocumentRef) -> serde_json::Value {
    serde_json::json!({
        "authority": reference.authority,
        "identity": reference.identity,
        "revision_namespace": reference.revision_namespace,
        "revision": reference.revision,
    })
}

fn lifecycle_observation(
    model_digest: &[u8; 32],
    label: &str,
    observation: &str,
    contexts: &[(bool, bool)],
) -> (DocumentRef, Vec<u8>) {
    let lifecycle = crate::tc235_k2::id("OrderLifecycle");
    let objects: Vec<_> = contexts
        .iter()
        .enumerate()
        .map(|(index, (placed, cancelled))| {
            serde_json::json!({
                "key": if index == 0 { "i1".to_owned() } else { format!("i1-{index}") },
                "type": lifecycle,
                "fields": {
                    "placed": {"boolean": placed},
                    "cancelled": {"boolean": cancelled},
                },
            })
        })
        .collect();
    let identity = DocumentRef {
        authority: "test".to_owned(),
        identity: label.to_owned(),
        revision_namespace: "tc235".to_owned(),
        revision: "1".to_owned(),
        digest: [0; 32],
    };
    let bytes = serde_json::to_vec(&serde_json::json!({
        "format": "quire.state.snapshot/v1",
        "identity": lifecycle_identity(&identity),
        "observation": observation,
        "model": {
            "identity": crate::tc235_k2::PACKAGE,
            "version": "1",
            "digest": format!("sha256-jcs:{}", qsl_semantics::model::key::hex(model_digest)),
        },
        "populations": [{
            "population": lifecycle,
            "complete": true,
            "objects": objects,
        }],
    }))
    .expect("lifecycle snapshot serializes");
    (lifecycle_document_ref(label, &bytes), bytes)
}

fn lifecycle_invocation(
    model_digest: &[u8; 32],
    pre: &DocumentRef,
    post: &DocumentRef,
) -> (DocumentRef, Vec<u8>) {
    let identity = DocumentRef {
        authority: "test".to_owned(),
        identity: "i1".to_owned(),
        revision_namespace: "tc235".to_owned(),
        revision: "1".to_owned(),
        digest: [0; 32],
    };
    let document_ref = |reference: &DocumentRef| {
        serde_json::json!({
            "identity": lifecycle_identity(reference),
            "digest": format!("sha256-jcs:{}", qsl_semantics::model::key::hex(&reference.digest)),
        })
    };
    let bytes = serde_json::to_vec(&serde_json::json!({
        "format": "quire.state.invocation/v1",
        "identity": lifecycle_identity(&identity),
        "model": {
            "identity": crate::tc235_k2::PACKAGE,
            "version": "1",
            "digest": format!("sha256-jcs:{}", qsl_semantics::model::key::hex(model_digest)),
        },
        "context": crate::tc235_k2::id("OrderLifecycle"),
        "operation": "place",
        "self": {"population": crate::tc235_k2::id("OrderLifecycle"), "key": "i1"},
        "pre": document_ref(pre),
        "post": document_ref(post),
        "parameters": {},
        "result": null,
        "created": [],
        "deleted": [],
    }))
    .expect("lifecycle invocation serializes");
    (lifecycle_document_ref("i1", &bytes), bytes)
}

fn lifecycle_observations(
    package: &CheckedPackage,
    packages: &BTreeMap<[u8; 32], Vec<u8>>,
    clause: &str,
    snapshots: &BTreeMap<[u8; 32], Vec<u8>>,
    invocations: &BTreeMap<[u8; 32], Vec<u8>>,
    invocation: DocumentRef,
) -> Result<AdmittedObservations, AdmissionFailure> {
    let checked = package.graph().state_clause(clause).expect("M15 clause");
    let facts = ClauseFacts {
        identity: checked.identity(),
        kind: checked.kind(),
        context: checked.context(),
        operation: checked.operation().map(|operation| OperationFacts {
            declaring: operation.declaring,
            declaration: operation.declaration.clone(),
        }),
    };
    admit_observations(
        package.graph().model_selections(),
        package.graph().scope().types(),
        &facts,
        packages,
        ModelNormalizationLimits::default(),
        &Provisions {
            snapshots,
            invocations,
        },
        &ClauseSelection {
            name: clause.to_owned(),
            input: ClauseSelectionInput::Invocation { invocation },
        },
        ObservationLimits::default(),
    )
}

fn lifecycle_invariant_observations(
    package: &CheckedPackage,
    admitted: &AdmittedObservations,
    post: bool,
) -> AdmittedObservations {
    AdmittedObservations {
        usage: admitted.usage,
        clause: package
            .graph()
            .state_clause("Consistent")
            .expect("M15 Consistent clause")
            .identity(),
        current: Some(
            (if post {
                admitted.post.as_ref()
            } else {
                admitted.pre.as_ref()
            })
            .expect("M15 invocation stage")
            .clone(),
        ),
        pre: None,
        post: None,
        self_object: if post {
            admitted.receiver_post.as_ref()
        } else {
            admitted.receiver_pre.as_ref()
        }
        .unwrap_or(&admitted.self_object)
        .clone(),
        receiver_invocation: admitted.receiver_invocation.clone(),
        receiver_pre: admitted.receiver_pre.clone(),
        receiver_post: admitted.receiver_post.clone(),
        parameters: Vec::new(),
        result: None,
        created: Vec::new(),
        deleted: Vec::new(),
    }
}

fn field(name: &str, value_type: ValueType) -> FieldDeclaration {
    FieldDeclaration::new(name, value_type, Presence::Required)
}

fn money_type() -> CompositeDeclaration {
    CompositeDeclaration::new(
        key("Money"),
        "Money",
        CompositeShape::Record(vec![
            field("amount_minor", ValueType::Integer),
            field(
                "currency",
                ValueType::Text(TextType::new(0, 16, TextProfile::BinaryUtf8).unwrap()),
            ),
        ]),
    )
}

fn money(amount: i64, currency: &str) -> Value {
    TypeEnvironment::new([money_type()], [])
        .unwrap()
        .record(
            key("Money"),
            vec![
                (
                    "amount_minor",
                    FieldValue::Present(Value::Integer(Integer::from(amount))),
                ),
                (
                    "currency",
                    FieldValue::Present(Value::Text(
                        admit_text(
                            &TextPayload::from_utf8(currency.as_bytes()).unwrap(),
                            &TextType::new(0, 16, TextProfile::BinaryUtf8).unwrap(),
                            &mut Meter::new(UNLIMITED),
                        )
                        .completed()
                        .unwrap(),
                    )),
                ),
            ],
        )
        .unwrap()
}

fn order_type(fields: Vec<FieldDeclaration>) -> CompositeDeclaration {
    CompositeDeclaration::new(key("Order"), "Order", CompositeShape::Record(fields))
}

fn order_value(fields: Vec<(&str, FieldValue)>, types: &[CompositeDeclaration]) -> Value {
    record_value(key("Order"), fields, types)
}

fn record_value(
    record_key: quire_exact::NodeKey,
    fields: Vec<(&str, FieldValue)>,
    types: &[CompositeDeclaration],
) -> Value {
    TypeEnvironment::new(types.iter().cloned(), [])
        .unwrap()
        .record(record_key, fields)
        .unwrap()
}

fn sequence(element_type: &CollectionType, values: Vec<Value>) -> Value {
    quire_exact::form_collection(element_type, values, &mut Meter::new(UNLIMITED))
        .expect("reachable sequence values admit")
        .completed()
        .unwrap()
}

fn type_form(value_type: &ValueType, name: &str) -> TypeForm {
    match value_type {
        ValueType::Composite(_) => crate::support::type_form::named_type_form(name),
        _ => crate::support::type_form::type_form(value_type),
    }
}

fn package_with_types(
    types: TypeEnvironment,
    functions: Vec<FunctionDeclaration>,
) -> CheckedPackage {
    let function_selections = functions
        .iter()
        .enumerate()
        .map(|(index, _)| {
            (
                index,
                ProfileSelection {
                    alias: "text".to_owned(),
                    identity: DefinitionLock::pinned()
                        .entry(CatalogRole::TextProfile)
                        .identity
                        .to_owned(),
                    span: qsl_foundation::Span { start: 0, end: 0 },
                    identity_span: qsl_foundation::Span { start: 0, end: 0 },
                },
            )
        })
        .collect();
    CheckedPackage::link(
        PackageDeclarations {
            types,
            functions,
            function_selections,
            lock_evidence: LockEvidence::default().with_text_profile(
                DefinitionLock::pinned()
                    .entry(CatalogRole::TextProfile)
                    .reference(),
            ),
            ..PackageDeclarations::new(
                qsl_semantics::check::fixture_source(),
                qsl_foundation::IdentityLimits::default(),
            )
        }
        .check(CheckingLimits::default())
        .unwrap(),
    )
}

fn value_package(functions: Vec<FunctionDeclaration>) -> CheckedPackage {
    package_with_types(TypeEnvironment::new([money_type()], []).unwrap(), functions)
}

fn name(value: &str) -> Expression {
    Expression::name(value.to_owned())
}

fn integer(value: i64) -> Expression {
    Expression::integer(Integer::from(value))
}

fn order_function(name_: &str, body: Expression) -> FunctionDeclaration {
    let order = ValueType::Composite(key("Order"));
    FunctionDeclaration::new(
        name_,
        vec![("self".to_owned(), type_form(&order, "Order"))],
        crate::support::type_form::type_form(&ValueType::Boolean),
        None,
        body,
    )
}

#[trace("QSpec-TC-235", "QSpec-FR-208-AC-4")]
#[test]
fn tc235_m02_record_values_compare_every_field_at_runtime() {
    let fixture = k2_order_with(
        serde_json::json!([
            crate::tc235_k2::field("Order", "total", &crate::tc235_k2::id("Money")),
            crate::tc235_k2::field("Order", "quoted", &crate::tc235_k2::id("Money")),
        ]),
        serde_json::json!([crate::tc235_k2::clause(
            "Order",
            "QuotedIsTotal",
            "self.quoted = self.total"
        )]),
    );
    assert!(fixture["types"][0]["fields"]
        .as_array()
        .unwrap()
        .iter()
        .any(|field| field["name"] == "quoted"));
    assert_eq!(
        fixture["types"][0]["clauses"][0]["clauseId"],
        "QuotedIsTotal"
    );
    let order = order_type(vec![
        field("total", ValueType::Composite(key("Money"))),
        field("quoted", ValueType::Composite(key("Money"))),
    ]);
    let package = package_with_types(
        TypeEnvironment::new([money_type(), order], []).unwrap(),
        vec![order_function(
            "quoted_is_total",
            Expression::binary(
                BinaryOperator::Equal,
                Expression::field(name("self"), "quoted"),
                Expression::field(name("self"), "total"),
            ),
        )],
    );
    let call = |quoted: Value, total: Value| {
        let values = [
            ("total", FieldValue::Present(total)),
            ("quoted", FieldValue::Present(quoted)),
        ];
        let value = order_value(
            values.to_vec(),
            &[
                money_type(),
                order_type(vec![
                    field("total", ValueType::Composite(key("Money"))),
                    field("quoted", ValueType::Composite(key("Money"))),
                ]),
            ],
        );
        package
            .call(
                &QualifiedName::unqualified("quoted_is_total").unwrap(),
                vec![value],
                &ObjectEnvironment::default(),
                &mut Meter::new(UNLIMITED),
            )
            .unwrap()
            .outcome
    };
    assert!(matches!(
        call(money(100, "EUR"), money(100, "EUR")),
        qsl_semantics::family::FamilyOutcome::Evaluated(Outcome::Completed(Value::Boolean(true)))
    ));
    assert!(matches!(
        call(money(100, "USD"), money(100, "EUR")),
        qsl_semantics::family::FamilyOutcome::Evaluated(Outcome::Completed(Value::Boolean(false)))
    ));
}

#[trace("QSpec-TC-235", "QSpec-FR-208-AC-10")]
#[test]
fn money_non_negative_scalar_components() {
    let mut history_field =
        crate::tc235_k2::field("Order", "history", &crate::tc235_k2::id("Money"));
    history_field["presence"] = serde_json::json!("optional");
    history_field["multiplicity"] = serde_json::json!({
        "lower": 0,
        "upper": 3,
        "ordered": true,
        "unique": false
    });
    let mut discount_field =
        crate::tc235_k2::field("Order", "discount", &crate::tc235_k2::id("Money"));
    discount_field["presence"] = serde_json::json!("optional");
    let mut fixture = k2_order_with(
        serde_json::json!([
            crate::tc235_k2::field("Order", "total", &crate::tc235_k2::id("Money")),
            crate::tc235_k2::field("Order", "quoted", &crate::tc235_k2::id("Money")),
            history_field,
            discount_field,
        ]),
        serde_json::json!([]),
    );
    let money_decl = fixture["types"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|ty| ty["identity"] == crate::tc235_k2::id("Money"))
        .expect("K2 Money declaration");
    money_decl["clauses"] = serde_json::json!([crate::tc235_k2::clause(
        "Money",
        "NonNegative",
        "self.amount_minor >= 0"
    )]);
    assert_eq!(fixture["types"][0]["fields"].as_array().unwrap().len(), 4);
    assert_eq!(fixture["types"][0]["fields"][2]["multiplicity"]["upper"], 3);
    assert_eq!(fixture["types"][0]["fields"][3]["presence"], "optional");
    assert_eq!(fixture["types"][2]["clauses"][0]["clauseId"], "NonNegative");
    let history_type = CollectionType::new(
        CollectionKind::Sequence,
        ValueType::Composite(key("Money")),
        Some(CardinalityBound::new(0, 3).unwrap()),
    );
    let observed_order = order_value(
        vec![
            ("total", FieldValue::Present(money(5, "EUR"))),
            ("quoted", FieldValue::Present(money(-5, "EUR"))),
            (
                "history",
                FieldValue::Present(sequence(
                    &history_type,
                    vec![money(1, "EUR"), money(-2, "EUR")],
                )),
            ),
            (
                "discount",
                FieldValue::Present(
                    OptionValue::present(ValueType::Composite(key("Money")), money(-1, "EUR"))
                        .unwrap(),
                ),
            ),
        ],
        &[
            money_type(),
            order_type(vec![
                field("total", ValueType::Composite(key("Money"))),
                field("quoted", ValueType::Composite(key("Money"))),
                field("history", ValueType::collection(history_type.clone())),
                field(
                    "discount",
                    ValueType::option(ValueType::Composite(key("Money"))),
                ),
            ]),
        ],
    );
    assert!(matches!(observed_order, Value::Composite(_)));
    let money_value_type = ValueType::Composite(key("Money"));
    let function = FunctionDeclaration::new(
        "non_negative",
        vec![("self".to_owned(), type_form(&money_value_type, "Money"))],
        crate::support::type_form::type_form(&ValueType::Boolean),
        None,
        Expression::binary(
            BinaryOperator::GreaterOrEqual,
            Expression::field(name("self"), "amount_minor".to_owned()),
            integer(0),
        ),
    );
    let package = value_package(vec![function]);
    let values = [
        money(5, "EUR"),
        money(-5, "EUR"),
        money(1, "EUR"),
        money(-2, "EUR"),
        money(-1, "EUR"),
        money(-7, "EUR"),
    ];
    let results: Vec<_> = values
        .iter()
        .map(|value| {
            package
                .call(
                    &QualifiedName::unqualified("non_negative").unwrap(),
                    vec![value.clone()],
                    &ObjectEnvironment::default(),
                    &mut Meter::new(UNLIMITED),
                )
                .unwrap()
                .outcome
        })
        .collect();
    assert!(matches!(
        results[0],
        qsl_semantics::family::FamilyOutcome::Evaluated(Outcome::Completed(Value::Boolean(true)))
    ));
    assert!(matches!(
        results[1],
        qsl_semantics::family::FamilyOutcome::Evaluated(Outcome::Completed(Value::Boolean(false)))
    ));
    assert!(matches!(
        results[2],
        qsl_semantics::family::FamilyOutcome::Evaluated(Outcome::Completed(Value::Boolean(true)))
    ));
    assert!(results[3..].iter().all(|result| matches!(
        result,
        qsl_semantics::family::FamilyOutcome::Evaluated(Outcome::Completed(Value::Boolean(false)))
    )));
}

#[test]
fn unresolved_object_reference_refuses_at_input_admission() {
    let event = object_type("OrderPlaced");
    let types = TypeEnvironment::new(
        [],
        [ObjectTypeDeclaration::new(
            event,
            "OrderPlaced",
            vec![field("occurred_at", ValueType::Boolean)],
        )],
    )
    .unwrap();
    let package = CheckedPackage::link(
        PackageDeclarations {
            types,
            ..PackageDeclarations::new(
                qsl_semantics::check::fixture_source(),
                qsl_foundation::IdentityLimits::default(),
            )
        }
        .check(CheckingLimits::default())
        .unwrap(),
    );
    let reference = ObjectReference::new(
        UniverseId::from_digest([4; 32]),
        event,
        ObjectId::new("event-absent").unwrap(),
    );
    let checked = package
        .graph()
        .check_clause_expression(
            vec![("self".to_owned(), ValueType::Reference(event))],
            &Expression::binary(
                BinaryOperator::Equal,
                Expression::field(Expression::deref(name("self")), "occurred_at"),
                Expression::field(Expression::deref(name("self")), "occurred_at"),
            ),
            None,
            ClauseKind::Invariant,
            quire_semantic_value::checking::CheckMode::Kernel,
            CheckingLimits::default(),
        )
        .unwrap();
    let evaluation = package
        .evaluate(
            &checked,
            vec![Value::Reference(reference)],
            &ObjectEnvironment::default(),
            &mut Meter::new(UNLIMITED),
        )
        .expect_err("an unresolved object reference is refused at admission");
    assert!(matches!(evaluation, qsl_eval::value::CallFailure::Input(_)));
}

#[trace("QSpec-TC-235", "QSpec-FR-208-AC-10", "QSpec-FR-208-AC-11")]
#[test]
#[ignore = "expected red until PR #781 lands"]
fn tc235_m15_lifecycle_invariant_keeps_receiver_context() {
    let document = lifecycle_document("not self.placed");
    let (package, _, packages) = lifecycle_package(
        &document,
        "invariant Consistent using v on M::OrderLifecycle at current { not (self.placed and self.cancelled) }\n\
         pre PlaceRequires using v on M::OrderLifecycle::place { not self.placed }\n\
         post PlaceEnsures using v on M::OrderLifecycle::place { self.placed and not self.cancelled }\n\
         post ReadsBothContexts using v on M::OrderLifecycle::place { self.placed and not pre(self.placed) }",
    )
    .expect("M15 runs 1-5 check");
    let evaluate = |clause: &str, observations: &AdmittedObservations| {
        package
            .evaluate_clause(
                &QualifiedName::unqualified(clause).unwrap(),
                observations,
                &mut Meter::new(UNLIMITED),
            )
            .expect("admitted lifecycle observations evaluate")
    };

    let run = |pre: &[(bool, bool)], post: &[(bool, bool)]| {
        let model_digest = packages.keys().next().expect("M15 K2 package");
        let (pre_ref, pre_bytes) = lifecycle_observation(model_digest, "p1", "pre", pre);
        let (post_ref, post_bytes) = lifecycle_observation(model_digest, "q1", "post", post);
        let (invocation_ref, invocation_bytes) =
            lifecycle_invocation(model_digest, &pre_ref, &post_ref);
        (
            BTreeMap::from([(pre_ref.digest, pre_bytes), (post_ref.digest, post_bytes)]),
            BTreeMap::from([(invocation_ref.digest, invocation_bytes)]),
            invocation_ref,
        )
    };

    let (snapshots, invocations, invocation) = run(&[(false, false)], &[(true, false)]);
    let pre_one = lifecycle_observations(
        &package,
        &packages,
        "PlaceRequires",
        &snapshots,
        &invocations,
        invocation.clone(),
    )
    .expect("run 1 precondition admits");
    let post_one = lifecycle_observations(
        &package,
        &packages,
        "PlaceEnsures",
        &snapshots,
        &invocations,
        invocation,
    )
    .expect("run 1 postcondition admits");
    let both_contexts = lifecycle_observations(
        &package,
        &packages,
        "ReadsBothContexts",
        &snapshots,
        &invocations,
        post_one
            .receiver_invocation
            .clone()
            .expect("run 1 invocation identity"),
    )
    .expect("postcondition reading both context anchors admits");
    assert!(matches!(
        evaluate("ReadsBothContexts", &both_contexts)
            .evaluation
            .outcome,
        qsl_semantics::family::FamilyOutcome::Evaluated(Outcome::Completed(Value::Boolean(true)))
    ));
    assert!(matches!(
        evaluate("PlaceRequires", &pre_one).evaluation.outcome,
        qsl_semantics::family::FamilyOutcome::Evaluated(Outcome::Completed(Value::Boolean(true)))
    ));
    assert!(matches!(
        evaluate("PlaceEnsures", &post_one).evaluation.outcome,
        qsl_semantics::family::FamilyOutcome::Evaluated(Outcome::Completed(Value::Boolean(true)))
    ));
    assert!(matches!(
        evaluate(
            "Consistent",
            &lifecycle_invariant_observations(&package, &pre_one, false),
        )
        .evaluation
        .outcome,
        qsl_semantics::family::FamilyOutcome::Evaluated(Outcome::Completed(Value::Boolean(true)))
    ));
    assert!(matches!(
        evaluate(
            "Consistent",
            &lifecycle_invariant_observations(&package, &post_one, true),
        )
        .evaluation
        .outcome,
        qsl_semantics::family::FamilyOutcome::Evaluated(Outcome::Completed(Value::Boolean(true)))
    ));

    let (snapshots, invocations, invocation) = run(&[(true, false)], &[(true, false)]);
    let pre_two = lifecycle_observations(
        &package,
        &packages,
        "PlaceRequires",
        &snapshots,
        &invocations,
        invocation,
    )
    .expect("run 2 precondition admits");
    assert!(matches!(
        evaluate("PlaceRequires", &pre_two).evaluation.outcome,
        qsl_semantics::family::FamilyOutcome::Evaluated(Outcome::Completed(Value::Boolean(false)))
    ));

    let (snapshots, invocations, invocation) = run(&[(false, false)], &[(true, true)]);
    let post_three = lifecycle_observations(
        &package,
        &packages,
        "PlaceEnsures",
        &snapshots,
        &invocations,
        invocation,
    )
    .expect("run 3 postcondition admits");
    assert!(matches!(
        evaluate("PlaceEnsures", &post_three).evaluation.outcome,
        qsl_semantics::family::FamilyOutcome::Evaluated(Outcome::Completed(Value::Boolean(false)))
    ));
    assert!(matches!(
        evaluate(
            "Consistent",
            &lifecycle_invariant_observations(&package, &post_three, false),
        )
        .evaluation
        .outcome,
        qsl_semantics::family::FamilyOutcome::Evaluated(Outcome::Completed(Value::Boolean(true)))
    ));
    let inconsistent = evaluate(
        "Consistent",
        &lifecycle_invariant_observations(&package, &post_three, true),
    );
    assert!(matches!(
        inconsistent.evaluation.outcome,
        qsl_semantics::family::FamilyOutcome::Evaluated(Outcome::Completed(Value::Boolean(false)))
    ));
    assert_eq!(
        inconsistent.receiver_path(),
        Some(&RuntimeValuePath {
            observation: ObservationIdentity {
                authority: "test".to_owned(),
                identity: "q1".to_owned(),
                revision_namespace: "tc235".to_owned(),
                revision: "1".to_owned(),
            },
            subject: ValuePathSubject::Invocation {
                invocation: ObservationIdentity {
                    authority: "test".to_owned(),
                    identity: "i1".to_owned(),
                    revision_namespace: "tc235".to_owned(),
                    revision: "1".to_owned(),
                },
                binding: InvocationBinding::Receiver,
            },
            steps: Vec::new(),
        })
    );

    let (snapshots, invocations, invocation) = run(&[], &[(true, false)]);
    let wrong_context = lifecycle_observations(
        &package,
        &packages,
        "PlaceEnsures",
        &snapshots,
        &invocations,
        invocation,
    )
    .expect_err("run 4 has no pre context");
    let AdmissionFailure::Refused(record) = wrong_context else {
        panic!("run 4 refusal: {wrong_context:?}");
    };
    assert_eq!(
        record.code,
        qsl_foundation::diagnostic::Code::InvalidModelBinding
    );
    assert_eq!(record.cause, "wrong-context");
    assert_eq!(record.fields["document"], "p1");
    assert_eq!(
        record.fields["required"],
        format!("({}, i1)", crate::tc235_k2::id("OrderLifecycle"))
    );
    assert_eq!(record.fields["actual"], "none");

    let (snapshots, invocations, invocation) =
        run(&[(false, false)], &[(true, false), (true, false)]);
    let duplicate = lifecycle_observations(
        &package,
        &packages,
        "PlaceEnsures",
        &snapshots,
        &invocations,
        invocation,
    )
    .expect_err("run 5 has two post contexts");
    let AdmissionFailure::Refused(record) = duplicate else {
        panic!("run 5 refusal: {duplicate:?}");
    };
    assert_eq!(
        record.code,
        qsl_foundation::diagnostic::Code::InvalidRuntimeInput
    );
    assert_eq!(record.cause, "duplicate-member");
    assert_eq!(record.fields["document"], "q1");
    assert_eq!(record.fields["path"], "/populations/0/objects/1");

    let pre_read = lifecycle_document("pre(self.placed) = false");
    let refusals = lifecycle_package(
        &pre_read,
        "invariant Consistent using v on M::OrderLifecycle at current { not (self.placed and self.cancelled) }\n\
         pre PlaceRequires using v on M::OrderLifecycle::place { not self.placed }\n\
         post PlaceEnsures using v on M::OrderLifecycle::place { self.placed and not self.cancelled }",
    )
    .expect_err("run 6 refuses the model operation pre-read");
    assert_eq!(refusals.len(), 1, "run 6 has one refusal: {refusals:?}");
    assert_eq!(
        refusals[0].cause.code(),
        qsl_foundation::diagnostic::Code::WrongSnapshot
    );
    assert!(matches!(
        refusals[0].cause,
        CheckCause::WrongSnapshot(WrongSnapshotCause::ForbiddenPreRead)
    ));
    assert!(format!("{:?}", refusals[0]).contains(&crate::tc235_k2::id("OrderLifecycle/place")));
    assert!(format!("{:?}", refusals[0]).contains("Operation.pre"));
}

#[trace("QSpec-TC-235")]
#[test]
fn lifecycle_receiver_and_parameter_boolean_components() {
    let lifecycle = object_type("OrderLifecycle");
    let flags = CompositeDeclaration::new(
        key("Flags"),
        "Flags",
        CompositeShape::Record(vec![field("placed", ValueType::Boolean)]),
    );
    let types = TypeEnvironment::new(
        [flags],
        [ObjectTypeDeclaration::new(
            lifecycle,
            "OrderLifecycle",
            vec![
                field("placed", ValueType::Boolean),
                field("cancelled", ValueType::Boolean),
            ],
        )],
    )
    .unwrap();
    let package = CheckedPackage::link(
        PackageDeclarations {
            types,
            ..PackageDeclarations::new(
                qsl_semantics::check::fixture_source(),
                qsl_foundation::IdentityLimits::default(),
            )
        }
        .check(CheckingLimits::default())
        .unwrap(),
    );
    let receiver = ObjectReference::new(
        UniverseId::from_digest([2; 32]),
        lifecycle,
        ObjectId::new("i1").unwrap(),
    );
    let objects = ObjectEnvironment::new(
        ObjectClosure::new(
            package.graph().scope().types(),
            [(
                receiver.clone(),
                vec![
                    ("placed", FieldValue::Present(Value::Boolean(true))),
                    ("cancelled", FieldValue::Present(Value::Boolean(true))),
                ],
            )],
            &[],
        )
        .unwrap(),
    );
    let checked = package
        .graph()
        .check_clause_expression(
            vec![
                ("self".to_owned(), ValueType::Reference(lifecycle)),
                ("flags".to_owned(), ValueType::Composite(key("Flags"))),
            ],
            &Expression::logical_not(Expression::binary(
                BinaryOperator::And,
                Expression::binary(
                    BinaryOperator::Equal,
                    Expression::field(Expression::deref(name("self")), "placed"),
                    Expression::field(name("flags"), "placed"),
                ),
                Expression::field(Expression::deref(name("self")), "cancelled"),
            )),
            None,
            ClauseKind::Invariant,
            quire_semantic_value::checking::CheckMode::Kernel,
            CheckingLimits::default(),
        )
        .unwrap();
    let flags_true = record_value(
        key("Flags"),
        vec![("placed", FieldValue::Present(Value::Boolean(true)))],
        &[CompositeDeclaration::new(
            key("Flags"),
            "Flags",
            CompositeShape::Record(vec![field("placed", ValueType::Boolean)]),
        )],
    );
    let flags_false = record_value(
        key("Flags"),
        vec![("placed", FieldValue::Present(Value::Boolean(false)))],
        &[CompositeDeclaration::new(
            key("Flags"),
            "Flags",
            CompositeShape::Record(vec![field("placed", ValueType::Boolean)]),
        )],
    );
    let evaluate = |flags: Value| {
        package
            .evaluate(
                &checked,
                vec![Value::Reference(receiver.clone()), flags],
                &objects,
                &mut Meter::new(UNLIMITED),
            )
            .unwrap()
            .outcome
    };
    assert!(matches!(
        evaluate(flags_true),
        qsl_semantics::family::FamilyOutcome::Evaluated(Outcome::Completed(Value::Boolean(false)))
    ));
    assert!(matches!(
        evaluate(flags_false),
        qsl_semantics::family::FamilyOutcome::Evaluated(Outcome::Completed(Value::Boolean(true)))
    ));
}
