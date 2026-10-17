// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-235 runtime components and M15 context admission/evaluation vectors.
//!
//! The semantic binder owns K2 admission and charge assertions. These rows
//! exercise the evaluator boundary with the same record, event and lifecycle
//! meanings, keeping the runtime oracle independent of the binder's JSON walk.

use std::collections::BTreeMap;

use ix_trace_rs::trace;
use qsl_eval::value::{
    CheckedPackageEvaluation, ClauseSetOutcome, InvocationBinding, ObservationIdentity,
    QualifiedName, RuntimeValuePath, ValuePathStep, ValuePathSubject,
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
    ClauseSelectionInput, ContextSide, DocumentRef, ObservationLimits, OperationFacts, Provisions,
};
use qsl_semantics::model::refusal::{MemberPath, MemberRoot};
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
    lifecycle_document_with(requires, |_| {})
}

/// [`lifecycle_document`] with `extra` applied before the digests are taken.
pub(crate) fn lifecycle_document_with(
    requires: &str,
    extra: impl FnOnce(&mut serde_json::Value),
) -> serde_json::Value {
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
    // FR-106 check 11 refuses a change outside `modifies` as
    // `frame_violation`/`unauthorized-change` (FR-106-AC-5) before any clause
    // is evaluated. `cancelled` is added to the frame so the run reaches its
    // postcondition (QSpec TC-235 M15 differs; details in the private record).
    place["frame"]["modifies"] = serde_json::json!([
        crate::tc235_k2::id("OrderLifecycle/placed"),
        crate::tc235_k2::id("OrderLifecycle/cancelled"),
    ]);
    extra(&mut document);
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
    model_package(document, body, 23)
}

/// The K2-derived `document` normalized after exactly `charges`
/// `normalize.record` charges and checked with `body` as its unit.
pub(crate) fn model_package(
    document: &serde_json::Value,
    body: &str,
    charges: usize,
) -> Result<LifecycleFixture, Vec<qsl_semantics::check::CheckRefusal>> {
    model_package_with(document, body, Some(charges))
}

/// [`model_package`] without a pinned `normalize.record` charge count.
pub(crate) fn model_package_unpinned(
    document: &serde_json::Value,
    body: &str,
) -> Result<LifecycleFixture, Vec<qsl_semantics::check::CheckRefusal>> {
    model_package_with(document, body, None)
}

fn model_package_with(
    document: &serde_json::Value,
    body: &str,
    charges: Option<usize>,
) -> Result<LifecycleFixture, Vec<qsl_semantics::check::CheckRefusal>> {
    let (domain, packages) = read_k2(document);
    let (outcome, meter) = normalize_with_meter(&domain, ModelNormalizationLimits::default());
    let NormalizeOutcome::Completed(view) = outcome else {
        panic!("M15 K2 normalization must admit: {outcome:?}");
    };
    if let Some(charges) = charges {
        assert_eq!(
            meter
                .admitted_charges()
                .iter()
                .filter(|charge| **charge == ChargePoint::NormalizeRecord)
                .count(),
            charges,
            "TC-235 normalize.record charge count"
        );
    }
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
    .expect("M15 model selection admits")
    .with_parsed_operation_clauses(|text| {
        qsl_forms::parse_clause_text(text, qsl_cst::Limits::default())
    })
    .expect("M15 kept operation clauses parse");
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

pub(crate) fn lifecycle_document_ref(label: &str, bytes: &[u8]) -> DocumentRef {
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

pub(crate) fn lifecycle_identity(reference: &DocumentRef) -> serde_json::Value {
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
    let contexts: Vec<_> = contexts
        .iter()
        .map(|(placed, cancelled)| {
            serde_json::json!({
                "machine": lifecycle,
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
        "populations": [],
        "contexts": contexts,
    }))
    .expect("lifecycle snapshot serializes");
    (lifecycle_document_ref(label, &bytes), bytes)
}

fn lifecycle_invocation(
    model_digest: &[u8; 32],
    pre: &DocumentRef,
    post: &DocumentRef,
) -> (DocumentRef, Vec<u8>) {
    lifecycle_invocation_with(model_digest, pre, post, serde_json::json!({}))
}

fn lifecycle_invocation_with(
    model_digest: &[u8; 32],
    pre: &DocumentRef,
    post: &DocumentRef,
    parameters: serde_json::Value,
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
        "self": null,
        "pre": document_ref(pre),
        "post": document_ref(post),
        "parameters": parameters,
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
    lifecycle_selection(
        package,
        packages,
        clause,
        snapshots,
        invocations,
        ClauseSelectionInput::Invocation { invocation },
    )
}

/// `clause` admitted from `input` over the lifecycle fixture's documents.
pub(crate) fn lifecycle_selection(
    package: &CheckedPackage,
    packages: &BTreeMap<[u8; 32], Vec<u8>>,
    clause: &str,
    snapshots: &BTreeMap<[u8; 32], Vec<u8>>,
    invocations: &BTreeMap<[u8; 32], Vec<u8>>,
    input: ClauseSelectionInput,
) -> Result<AdmittedObservations, AdmissionFailure> {
    lifecycle_selection_with(
        package,
        packages,
        clause,
        snapshots,
        invocations,
        input,
        ObservationLimits::default(),
    )
}

/// [`lifecycle_selection`] under `limits`.
pub(crate) fn lifecycle_selection_with(
    package: &CheckedPackage,
    packages: &BTreeMap<[u8; 32], Vec<u8>>,
    clause: &str,
    snapshots: &BTreeMap<[u8; 32], Vec<u8>>,
    invocations: &BTreeMap<[u8; 32], Vec<u8>>,
    input: ClauseSelectionInput,
    limits: ObservationLimits,
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
        operations: package.graph().scope().operations().clone(),
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
            input,
        },
        limits,
    )
}

/// The invariant `Consistent` over one side of `invocation`: FR-106's
/// `Context` selection.
pub(crate) fn lifecycle_context(
    package: &CheckedPackage,
    packages: &BTreeMap<[u8; 32], Vec<u8>>,
    snapshots: &BTreeMap<[u8; 32], Vec<u8>>,
    invocations: &BTreeMap<[u8; 32], Vec<u8>>,
    invocation: &DocumentRef,
    side: qsl_semantics::model::observation::ContextSide,
) -> AdmittedObservations {
    lifecycle_selection(
        package,
        packages,
        "Consistent",
        snapshots,
        invocations,
        ClauseSelectionInput::Context {
            invocation: invocation.clone(),
            side,
        },
    )
    .expect("the invariant's context admits")
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

/// TC-235 M02: the K2 package with `Order` given `quoted` typed `Money` and the
/// invariant `QuotedIsTotal` is admitted after 23 charges, and the admitted
/// clause is `true` on an `Order` whose `quoted` and `total` match in every
/// field and `false` when only the currency differs.
///
/// Mutation used (derived by reading the record equality in the evaluator, not
/// executed): comparing only the first field of a record flips run 2 to `true`;
/// binding `self` to a different object flips both runs.
#[trace("QSpec-TC-235", "QSpec-FR-208-AC-4")]
#[test]
fn tc235_m02_record_values_compare_every_field_at_runtime() {
    let fixture = k2_order_with(
        serde_json::json!([
            crate::tc235_k2::field("Order", "order_id", "ix://quire/native/UUID"),
            crate::tc235_k2::field("Order", "total", &crate::tc235_k2::id("Money")),
            crate::tc235_k2::field("Order", "quoted", &crate::tc235_k2::id("Money")),
        ]),
        serde_json::json!([crate::tc235_k2::clause(
            "Order",
            "QuotedIsTotal",
            "self.quoted = self.total"
        )]),
    );
    let (package, view, _) = model_package(
        &fixture,
        "invariant QuotedIsTotal using v on M::Order at current { self.quoted = self.total }",
        23,
    )
    .expect("M02 checks");

    let types = package.graph().scope().types();
    let declaration = |node: &str| qsl_semantics::model::key::DeclarationKey {
        package: crate::tc235_k2::PACKAGE.to_owned(),
        node: crate::tc235_k2::id(node),
    };
    let money_declaration = types
        .composites()
        .find(|composite| composite.name().ends_with("Money"))
        .expect("the admitted package declares Money");
    let money_value = |amount: i64, currency: &str| {
        let CompositeShape::Record(declared) = money_declaration.shape() else {
            panic!("Money is a record");
        };
        let fields = declared
            .iter()
            .map(|declared| {
                let value = match declared.value_type() {
                    ValueType::Integer => Value::Integer(Integer::from(amount)),
                    ValueType::String => Value::String(currency.to_owned()),
                    other => panic!("unexpected Money field type {other:?}"),
                };
                (declared.name(), FieldValue::Present(value))
            })
            .collect();
        types
            .record(money_declaration.key(), fields)
            .expect("Money value constructs")
    };
    let order_type = view.type_identities()[&declaration("Order")];
    let universe = view
        .object_universe_of(&declaration("Order"))
        .expect("Order has an object universe")
        .identity();
    let o1 = ObjectReference::new(universe, order_type, ObjectId::new("o1").unwrap());

    let truth_for = |quoted: Value| {
        let closure = ObjectClosure::new(
            types,
            [(
                o1.clone(),
                vec![
                    (
                        "order_id",
                        FieldValue::Present(Value::Uuid(
                            quire_exact::Uuid::from_canonical_text(
                                "00000000-0000-4000-8000-000000000001",
                            )
                            .expect("a canonical UUID"),
                        )),
                    ),
                    ("total", FieldValue::Present(money_value(100, "EUR"))),
                    ("quoted", FieldValue::Present(quoted)),
                ],
            )],
            &[],
        )
        .expect("the Order closes over its type");
        let s1 = qsl_semantics::model::observation::Observation {
            identity: DocumentRef {
                authority: "test".to_owned(),
                identity: "s1".to_owned(),
                revision_namespace: "tc235".to_owned(),
                revision: "1".to_owned(),
                digest: [0; 32],
            },
            environment: ObjectEnvironment::new(closure),
            populations: BTreeMap::new(),
            out_of_range: Vec::new(),
        };
        truth(
            &package,
            "QuotedIsTotal",
            &process_observations(&package, "QuotedIsTotal", Some(s1), None, None, o1.clone()),
        )
    };
    assert!(
        truth_for(money_value(100, "EUR")),
        "run 1: equal in every field"
    );
    assert!(
        !truth_for(money_value(100, "USD")),
        "run 2: the currency differs"
    );
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
        invocation.clone(),
    )
    .expect("run 1 postcondition admits");
    let both_contexts = lifecycle_observations(
        &package,
        &packages,
        "ReadsBothContexts",
        &snapshots,
        &invocations,
        post_one
            .invocation
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
    // The model guard is no checked clause, so `NotYetPlaced` can be neither
    // looked up nor evaluated against the run's observations.
    assert!(package.graph().state_clause("NotYetPlaced").is_none());
    let not_yet_placed = |observations: &AdmittedObservations| {
        package.evaluate_clause(
            &QualifiedName::unqualified("NotYetPlaced").unwrap(),
            observations,
            &mut Meter::new(UNLIMITED),
        )
    };
    assert!(not_yet_placed(&pre_one).is_err());
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
            &lifecycle_context(
                &package,
                &packages,
                &snapshots,
                &invocations,
                &invocation,
                ContextSide::Pre,
            ),
        )
        .evaluation
        .outcome,
        qsl_semantics::family::FamilyOutcome::Evaluated(Outcome::Completed(Value::Boolean(true)))
    ));
    assert!(matches!(
        evaluate(
            "Consistent",
            &lifecycle_context(
                &package,
                &packages,
                &snapshots,
                &invocations,
                &invocation,
                ContextSide::Post,
            ),
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
    // `NotYetPlaced` is not evaluated, and no checked clause holds the
    // transitions' `from` and `to`.
    assert!(not_yet_placed(&pre_two).is_err());
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
        invocation.clone(),
    )
    .expect("run 3 postcondition admits");
    assert!(matches!(
        evaluate("PlaceEnsures", &post_three).evaluation.outcome,
        qsl_semantics::family::FamilyOutcome::Evaluated(Outcome::Completed(Value::Boolean(false)))
    ));
    assert!(matches!(
        evaluate(
            "Consistent",
            &lifecycle_context(
                &package,
                &packages,
                &snapshots,
                &invocations,
                &invocation,
                ContextSide::Pre,
            ),
        )
        .evaluation
        .outcome,
        qsl_semantics::family::FamilyOutcome::Evaluated(Outcome::Completed(Value::Boolean(true)))
    ));
    let inconsistent = evaluate(
        "Consistent",
        &lifecycle_context(
            &package,
            &packages,
            &snapshots,
            &invocations,
            &invocation,
            ContextSide::Post,
        ),
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
        record.fields["machine"],
        crate::tc235_k2::id("OrderLifecycle")
    );

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
    assert_eq!(record.fields["entry"], "1");
}

/// M15 run 6 (FR-103-AC-8): a model operation `requires` that reads `pre(e)`
/// refuses `wrong_snapshot`/`forbidden-pre-read`, naming the operation and
/// located at its `Operation.pre` member, with no checked package.
#[trace(
    "QSpec-TC-235",
    "QSpec-FR-208-AC-10",
    "QSpec-FR-208-AC-11",
    "FR-103-AC-8",
    "TC-993"
)]
#[test]
fn tc235_m15_run6_operation_requires_reading_pre_refuses() {
    let pre_read = lifecycle_document("pre(self.placed) = false");
    let refusals = lifecycle_package(
        &pre_read,
        "invariant Consistent using v on M::OrderLifecycle at current { not (self.placed and self.cancelled) }\n\
         pre PlaceRequires using v on M::OrderLifecycle::place { not self.placed }\n\
         post PlaceEnsures using v on M::OrderLifecycle::place { self.placed and not self.cancelled }",
    )
    .expect_err("run 6 refuses the model operation pre-read");
    assert_eq!(refusals.len(), 1, "run 6 has one refusal: {refusals:?}");
    let refusal = &refusals[0];
    assert_eq!(
        refusal.cause.code(),
        qsl_foundation::diagnostic::Code::WrongSnapshot
    );
    assert!(matches!(
        refusal.cause,
        CheckCause::WrongSnapshot(WrongSnapshotCause::ForbiddenPreRead)
    ));
    assert_eq!(
        refusal.operation_node(),
        Some(crate::tc235_k2::id("OrderLifecycle/place").as_str())
    );
    assert_eq!(
        refusal.operation_member(),
        Some(MemberPath::new(MemberRoot::Operation).name("pre").index(0))
    );
}

/// The M15 package with `place`'s kept clauses replaced by `requires` and
/// `ensures`, each entry `(language, text)`.
fn lifecycle_with_contract(
    requires: &[(&str, &str)],
    ensures: &[(&str, &str)],
) -> serde_json::Value {
    let mut document = lifecycle_document("not self.placed");
    let place = document["types"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|ty| ty["identity"] == crate::tc235_k2::id("OrderLifecycle"))
        .expect("K2 OrderLifecycle declaration")["operations"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|operation| operation["identity"] == crate::tc235_k2::id("OrderLifecycle/place"))
        .expect("K2 OrderLifecycle/place operation");
    let clauses = |entries: &[(&str, &str)]| {
        serde_json::Value::Array(
            entries
                .iter()
                .map(|(language, text)| {
                    serde_json::json!({
                        "language": language,
                        "text": text,
                        "origin": crate::tc235_k2::origin("OrderLifecycle/place"),
                    })
                })
                .collect(),
        )
    };
    place["pre"] = clauses(requires);
    place["post"] = clauses(ensures);
    document
}

const LIFECYCLE_UNIT_CLAUSES: &str =
    "invariant Consistent using v on M::OrderLifecycle at current { not (self.placed and self.cancelled) }";

/// FR-103-AC-8: a refusal in the second `requires` and in an `ensures` is
/// located at that clause's own position in its own list.
#[trace("FR-103-AC-8", "TC-993")]
#[test]
fn operation_clause_refusals_are_located_by_kind_and_position() {
    let document = lifecycle_with_contract(
        &[("quire", "not self.placed"), ("quire", "pre(self.placed)")],
        &[("quire", "self.placed")],
    );
    let refusals = lifecycle_package(&document, LIFECYCLE_UNIT_CLAUSES)
        .map(|_| ())
        .expect_err("the second requires reads pre");
    assert_eq!(refusals.len(), 1, "{refusals:?}");
    assert!(matches!(
        refusals[0].cause,
        CheckCause::WrongSnapshot(WrongSnapshotCause::ForbiddenPreRead)
    ));
    assert_eq!(
        refusals[0].operation_member(),
        Some(MemberPath::new(MemberRoot::Operation).name("pre").index(1))
    );

    let document = lifecycle_with_contract(&[], &[("quire", "self.placed"), ("quire", "self")]);
    let refusals = lifecycle_package(&document, LIFECYCLE_UNIT_CLAUSES)
        .map(|_| ())
        .expect_err("the second ensures is not Boolean");
    assert_eq!(refusals.len(), 1, "{refusals:?}");
    assert_eq!(
        refusals[0].operation_node(),
        Some(crate::tc235_k2::id("OrderLifecycle/place").as_str())
    );
    assert_eq!(
        refusals[0].operation_member(),
        Some(MemberPath::new(MemberRoot::Operation).name("post").index(1))
    );
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

/// K2 with `Fulfilment` given `attempts`, the invariant `NonNegativeAttempts`,
/// the operation `restart` with its contract, and the closed population `PF`
/// (TC-235 M16).
fn process_document() -> serde_json::Value {
    use crate::tc235_k2::{clause, field, id, mutate_k2, operation, origin, type_named_mut};
    mutate_k2(|document| {
        let process = type_named_mut(document, "Fulfilment");
        process["fields"].as_array_mut().unwrap().push(field(
            "Fulfilment",
            "attempts",
            "ix://quire/native/Integer",
        ));
        process["clauses"] = serde_json::json!([clause(
            "Fulfilment",
            "NonNegativeAttempts",
            "self.attempts >= 0"
        )]);
        let mut restart = operation("Fulfilment", "restart");
        restart["pre"] = serde_json::json!([{
            "language": "quire",
            "text": "self.attempts < 3",
            "origin": origin("Fulfilment/restart"),
        }]);
        restart["post"] = serde_json::json!([{
            "language": "quire",
            "text": "self.attempts = pre(self.attempts) + 1",
            "origin": origin("Fulfilment/restart"),
        }]);
        restart["frame"]["modifies"] = serde_json::json!([id("Fulfilment/attempts")]);
        process["operations"].as_array_mut().unwrap().push(restart);
        document["populations"] = serde_json::json!([{
            "identity": id("PF"),
            "displayName": id("PF"),
            "kind": {"module": crate::tc235_k2::BUSINESS, "name": "population"},
            "members": [id("Fulfilment")],
            "extent": "closed",
            "origin": origin("PF"),
        }]);
    })
}

/// An observation named `label` holding the `Fulfilment` runs `runs`, each
/// `(object id, attempts)`.
fn process_observation(
    package: &CheckedPackage,
    view: &EffectiveView,
    label: &str,
    runs: &[(&str, i64)],
) -> (
    qsl_semantics::model::observation::Observation,
    Vec<ObjectReference>,
) {
    let fulfilment = view.type_identities()[&qsl_semantics::model::key::DeclarationKey {
        package: crate::tc235_k2::PACKAGE.to_owned(),
        node: crate::tc235_k2::id("Fulfilment"),
    }];
    let universe = view
        .object_universe_of(&qsl_semantics::model::key::DeclarationKey {
            package: crate::tc235_k2::PACKAGE.to_owned(),
            node: crate::tc235_k2::id("Fulfilment"),
        })
        .expect("Fulfilment has an object universe")
        .identity();
    let references: Vec<_> = runs
        .iter()
        .map(|(object, _)| {
            ObjectReference::new(universe, fulfilment, ObjectId::new(*object).unwrap())
        })
        .collect();
    let objects = references
        .iter()
        .zip(runs)
        .map(|(reference, (object, attempts))| {
            let correlation = quire_exact::Uuid::from_canonical_text(&format!(
                "00000000-0000-4000-8000-{:0>12}",
                object.trim_start_matches('r')
            ))
            .expect("a canonical UUID");
            (
                reference.clone(),
                vec![
                    (
                        "correlation_id",
                        FieldValue::Present(Value::Uuid(correlation)),
                    ),
                    (
                        "attempts",
                        FieldValue::Present(Value::Integer(Integer::from(*attempts))),
                    ),
                ],
            )
        });
    let closure = ObjectClosure::new(package.graph().scope().types(), objects, &[])
        .expect("the runs close over the type");
    (
        qsl_semantics::model::observation::Observation {
            identity: DocumentRef {
                authority: "test".to_owned(),
                identity: label.to_owned(),
                revision_namespace: "tc235".to_owned(),
                revision: "1".to_owned(),
                digest: [0; 32],
            },
            environment: ObjectEnvironment::new(closure),
            populations: BTreeMap::from([(crate::tc235_k2::id("PF"), true)]),
            out_of_range: Vec::new(),
        },
        references,
    )
}

fn process_observations(
    package: &CheckedPackage,
    clause: &str,
    current: Option<qsl_semantics::model::observation::Observation>,
    pre: Option<qsl_semantics::model::observation::Observation>,
    post: Option<qsl_semantics::model::observation::Observation>,
    self_object: ObjectReference,
) -> AdmittedObservations {
    AdmittedObservations {
        usage: Default::default(),
        clause: package
            .graph()
            .state_clause(clause)
            .expect("the unit declares the clause")
            .identity(),
        current,
        pre,
        post,
        self_object,
        invocation: None,
        self_kind: qsl_semantics::model::observation::SelfKind::Object,
        receiver_pre: None,
        receiver_post: None,
        parameters: Vec::new(),
        result: None,
        created: Vec::new(),
        deleted: Vec::new(),
    }
}

/// A completed Boolean: the evaluation carries no diagnostic code or cause.
pub(crate) fn truth(
    package: &CheckedPackage,
    clause: &str,
    observations: &AdmittedObservations,
) -> bool {
    let evaluated = package
        .evaluate_clause(
            &QualifiedName::unqualified(clause).unwrap(),
            observations,
            &mut Meter::new(UNLIMITED),
        )
        .expect("admitted observations evaluate");
    match evaluated.evaluation.outcome {
        qsl_semantics::family::FamilyOutcome::Evaluated(Outcome::Completed(Value::Boolean(
            value,
        ))) => value,
        other => panic!("expected a completed Boolean, got {other:?}"),
    }
}

/// TC-235 M16: the process invariant holds per run (`r1` `{attempts: 0}` true,
/// `r2` `{attempts: -1}` a logical violation: a completed `false`, no
/// diagnostic), and `restart` invoked on `r1` from `{attempts: 0}` to
/// `{attempts: 1}` satisfies its precondition and postcondition, while from
/// `{attempts: 3}` to `{attempts: 4}` only the precondition is `false`.
///
/// Mutation used (derived by reading the clause evaluator, not executed):
/// binding `self` to the first run of the population rather than the selected
/// one flips `r2` to `true`; binding `pre(...)` reads to the post observation
/// flips run 2's postcondition to `false`; dropping the precondition's
/// observation selection flips run 3's precondition; reporting no receiver
/// path for an object receiver (or the invocation receiver binding) flips the
/// located-violation assertion. Run 3's one-result outcome is
/// `tc235_m16_run3_a_clause_set_reports_one_result_for_a_false_precondition`.
#[trace(
    "QSpec-TC-235",
    "QSpec-FR-208-AC-6",
    "QSpec-FR-208-AC-10",
    "QSpec-FR-208-AC-11"
)]
#[test]
fn tc235_m16_process_invariant_and_operation_clauses_evaluate_per_run() {
    let (package, view, _) = model_package(
        &process_document(),
        "invariant NonNegativeAttempts using v on M::Fulfilment at current { self.attempts >= 0 }\n\
         pre RestartRequires using v on M::Fulfilment::restart { self.attempts < 3 }\n\
         post RestartEnsures using v on M::Fulfilment::restart { self.attempts = pre(self.attempts) + 1 }",
        25,
    )
    .expect("M16 checks");

    // Run 1: invariant observation s1 holding r1 and r2.
    let (s1, runs) = process_observation(&package, &view, "s1", &[("r1", 0), ("r2", -1)]);
    let invariant = |run: usize| {
        truth(
            &package,
            "NonNegativeAttempts",
            &process_observations(
                &package,
                "NonNegativeAttempts",
                Some(s1.clone()),
                None,
                None,
                runs[run].clone(),
            ),
        )
    };
    assert!(invariant(0), "r1 holds");
    assert!(!invariant(1), "r2 is a logical violation");

    // Run 1's location: the violation is located at the root of the run's
    // declared object identity at `s1`, with no steps -- never the
    // invocation receiver binding, which a run with an object identity
    // does not use (FR-207-AC-8).
    let violation = package
        .evaluate_clause(
            &QualifiedName::unqualified("NonNegativeAttempts").unwrap(),
            &process_observations(
                &package,
                "NonNegativeAttempts",
                Some(s1.clone()),
                None,
                None,
                runs[1].clone(),
            ),
            &mut Meter::new(UNLIMITED),
        )
        .expect("admitted observations evaluate");
    assert_eq!(
        violation.receiver_path(),
        Some(&RuntimeValuePath {
            observation: ObservationIdentity {
                authority: "test".to_owned(),
                identity: "s1".to_owned(),
                revision_namespace: "tc235".to_owned(),
                revision: "1".to_owned(),
            },
            subject: ValuePathSubject::Object(runs[1].clone()),
            steps: Vec::new(),
        })
    );

    // Runs 2 and 3: `restart` on r1.
    let restart = |from: i64, to: i64| {
        let (pre, runs) = process_observation(&package, &view, "p1", &[("r1", from)]);
        let (post, _) = process_observation(&package, &view, "q1", &[("r1", to)]);
        let required = truth(
            &package,
            "RestartRequires",
            &process_observations(
                &package,
                "RestartRequires",
                None,
                Some(pre.clone()),
                None,
                runs[0].clone(),
            ),
        );
        let ensured = truth(
            &package,
            "RestartEnsures",
            &process_observations(
                &package,
                "RestartEnsures",
                None,
                Some(pre),
                Some(post),
                runs[0].clone(),
            ),
        );
        (required, ensured)
    };
    assert_eq!(restart(0, 1), (true, true));
    assert!(!restart(3, 4).0, "attempts 3 does not satisfy attempts < 3");
}

/// TC-235 M16 run 3 (FR-107-AC-13, AC-14): the clause set of `restart` over
/// `r1` from `{attempts: 0}` to `{attempts: 1}` reports both results, each
/// `true`, outcome `True`; from `{attempts: 3}` to `{attempts: 4}` it reports
/// exactly one result, the precondition `RestartRequires` `false`, outcome
/// `False`, and no result for the postcondition `RestartEnsures`.
#[trace("QSpec-TC-235")]
#[test]
fn tc235_m16_run3_a_clause_set_reports_one_result_for_a_false_precondition() {
    let (package, view, _) = model_package(
        &process_document(),
        "invariant NonNegativeAttempts using v on M::Fulfilment at current { self.attempts >= 0 }\n\
         pre RestartRequires using v on M::Fulfilment::restart { self.attempts < 3 }\n\
         post RestartEnsures using v on M::Fulfilment::restart { self.attempts = pre(self.attempts) + 1 }",
        25,
    )
    .expect("M16 checks");
    let set = |from: i64, to: i64| {
        let (pre, runs) = process_observation(&package, &view, "p1", &[("r1", from)]);
        let (post, _) = process_observation(&package, &view, "q1", &[("r1", to)]);
        let required = process_observations(
            &package,
            "RestartRequires",
            None,
            Some(pre.clone()),
            None,
            runs[0].clone(),
        );
        let ensured = process_observations(
            &package,
            "RestartEnsures",
            None,
            Some(pre),
            Some(post),
            runs[0].clone(),
        );
        // Listed postcondition first: the set orders its own entries.
        let clauses = [
            (
                QualifiedName::unqualified("RestartEnsures").unwrap(),
                &ensured,
            ),
            (
                QualifiedName::unqualified("RestartRequires").unwrap(),
                &required,
            ),
        ];
        let evaluation = package
            .evaluate_clause_set(&clauses, UNLIMITED)
            .expect("admitted observations evaluate");
        let entries: Vec<_> = evaluation
            .results
            .iter()
            .map(|entry| {
                let truth = match entry.evaluation.evaluation.outcome {
                    qsl_semantics::family::FamilyOutcome::Evaluated(Outcome::Completed(
                        Value::Boolean(value),
                    )) => value,
                    ref other => panic!("expected a completed Boolean, got {other:?}"),
                };
                (entry.name.to_string(), truth)
            })
            .collect();
        (entries, evaluation.outcome)
    };
    assert_eq!(
        set(0, 1),
        (
            vec![
                ("RestartRequires".to_owned(), true),
                ("RestartEnsures".to_owned(), true)
            ],
            ClauseSetOutcome::True
        )
    );
    assert_eq!(
        set(3, 4),
        (
            vec![("RestartRequires".to_owned(), false)],
            ClauseSetOutcome::False
        )
    );
}

/// The package, its offered package documents, snapshots, invocations and the
/// selected invocation of [`orders_fixture`].
type OrdersFixture = (
    CheckedPackage,
    BTreeMap<[u8; 32], Vec<u8>>,
    BTreeMap<[u8; 32], Vec<u8>>,
    BTreeMap<[u8; 32], Vec<u8>>,
    DocumentRef,
);

/// The M15 lifecycle package with `place(flags: Flags)` and the TC-235 M22
/// clauses, and the invocation `i1` of `place` binding `flags` to
/// `{placed: true}` between `p1` `{placed: false, cancelled: false}` and `q1`
/// `{placed: true, cancelled: true}`.
fn orders_fixture() -> OrdersFixture {
    use crate::tc235_k2::{declaration, field, id, origin};
    let flags_type = id("Flags");
    let document = lifecycle_document_with("not self.placed", |document| {
        document["types"].as_array_mut().unwrap().push(declaration(
            "Flags",
            "money",
            serde_json::json!({
                "fields": [field("Flags", "placed", "ix://quire/native/Boolean")],
                "operations": [],
            }),
        ));
        let lifecycle = crate::tc235_k2::type_named_mut(document, "OrderLifecycle");
        lifecycle["operations"][0]["params"] = serde_json::json!([{
            "identity": id("OrderLifecycle/place/flags"),
            "origin": origin("OrderLifecycle/place/flags"),
            "name": "flags",
            "typeRef": flags_type,
            "presence": "required",
            "nullable": false,
            "defaultKind": "none",
            "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
        }]);
    });
    let (package, _, packages) = model_package(
        &document,
        "invariant Consistent using v on M::OrderLifecycle at current { not (self.placed and self.cancelled) }\n\
         pre PlaceRequires using v on M::OrderLifecycle::place { not self.placed }\n\
         post FlagsPlaced using v on M::OrderLifecycle::place { flags.placed }",
        25,
    )
    .expect("M22 checks");
    let model_digest = packages.keys().next().expect("M22 package");
    let (pre_ref, pre_bytes) = lifecycle_observation(model_digest, "p1", "pre", &[(false, false)]);
    let (post_ref, post_bytes) = lifecycle_observation(model_digest, "q1", "post", &[(true, true)]);
    let (invocation_ref, invocation_bytes) = lifecycle_invocation_with(
        model_digest,
        &pre_ref,
        &post_ref,
        serde_json::json!({"flags": {"record": {
            "type": flags_type,
            "fields": {"placed": {"boolean": true}},
        }}}),
    );
    (
        package,
        packages,
        BTreeMap::from([(pre_ref.digest, pre_bytes), (post_ref.digest, post_bytes)]),
        BTreeMap::from([(invocation_ref.digest, invocation_bytes)]),
        invocation_ref,
    )
}

fn identity_of(label: &str) -> ObservationIdentity {
    ObservationIdentity {
        authority: "test".to_owned(),
        identity: label.to_owned(),
        revision_namespace: "tc235".to_owned(),
        revision: "1".to_owned(),
    }
}

/// A path rooted at `i1`'s `binding` under observation `label`.
fn invocation_path(
    label: &str,
    binding: InvocationBinding,
    steps: Vec<ValuePathStep>,
) -> RuntimeValuePath {
    RuntimeValuePath {
        observation: identity_of(label),
        subject: ValuePathSubject::Invocation {
            invocation: identity_of("i1"),
            binding,
        },
        steps,
    }
}

fn reads_of(evaluation: &qsl_eval::value::ClauseEvaluation) -> Vec<Option<RuntimeValuePath>> {
    evaluation
        .reads()
        .iter()
        .map(|read| read.path.clone())
        .collect()
}

/// TC-235 M22 (TC-1040 steps 6 to 8, FR-107-AC-7 and AC-11): the receiver
/// and parameter reads of the `Orders` lifecycle are located at the
/// invocation's receiver and parameter bindings.
///
/// The invariant's observation is built at the model layer from `i1` and its
/// post context `q1` (FR-107 "The receiver path"). The wire serialization of
/// the two `placed` paths is asserted in qsl-replay
/// (`result::wire::tests`), whose `PathWire` is private to that crate.
#[trace("QSpec-TC-235", "TC-1040", "FR-107-AC-7", "FR-107-AC-11")]
#[test]
fn tc235_m22_receiver_and_parameter_reads_are_located_at_their_bindings() {
    use qsl_semantics::model::observation::SnapshotValue;
    let (package, packages, snapshots, invocations, i1) = orders_fixture();
    let admit = |clause: &str, invocation: DocumentRef| {
        lifecycle_observations(
            &package,
            &packages,
            clause,
            &snapshots,
            &invocations,
            invocation,
        )
        .expect("the observations admit")
    };
    let evaluate = |clause: &str, observations: &AdmittedObservations| {
        package
            .evaluate_clause(
                &QualifiedName::unqualified(clause).unwrap(),
                observations,
                &mut Meter::new(UNLIMITED),
            )
            .expect("admitted observations evaluate")
    };
    let field_step = |name: &str| ValuePathStep::Field(name.to_owned());
    let receiver = InvocationBinding::Receiver;
    let flags = InvocationBinding::Parameter("flags".to_owned());

    // Step 6: `Consistent` at `q1`.
    let post = admit("FlagsPlaced", i1.clone());
    let consistent = evaluate(
        "Consistent",
        &lifecycle_context(
            &package,
            &packages,
            &snapshots,
            &invocations,
            &i1,
            ContextSide::Post,
        ),
    );
    assert!(matches!(
        consistent.evaluation.outcome,
        qsl_semantics::family::FamilyOutcome::Evaluated(Outcome::Completed(Value::Boolean(false)))
    ));
    assert_eq!(
        consistent.receiver_path(),
        Some(&invocation_path("q1", receiver.clone(), vec![]))
    );
    let placed = invocation_path("q1", receiver.clone(), vec![field_step("placed")]);
    assert_eq!(
        reads_of(&consistent),
        vec![
            Some(placed.clone()),
            Some(invocation_path(
                "q1",
                receiver.clone(),
                vec![field_step("cancelled")]
            )),
        ]
    );

    // Step 7: `FlagsPlaced` reads the parameter `flags`.
    let flags_placed = evaluate("FlagsPlaced", &post);
    let parameter = invocation_path("q1", flags, vec![field_step("placed")]);
    assert_eq!(reads_of(&flags_placed), vec![Some(parameter.clone())]);
    assert_ne!(parameter, placed);
    assert!(parameter < placed, "the parameter path orders first");

    // Step 8: `PlaceRequires` over `i1`, then over a pre-call observation.
    let required = admit("PlaceRequires", i1);
    let by_invocation = evaluate("PlaceRequires", &required);
    assert_eq!(
        by_invocation.receiver_path(),
        Some(&invocation_path("p1", receiver.clone(), vec![]))
    );
    assert_eq!(
        reads_of(&by_invocation),
        vec![Some(invocation_path(
            "p1",
            receiver,
            vec![field_step("placed")]
        ))]
    );

    let model_digest = packages.keys().next().expect("M22 package");
    let (pre_ref, pre_bytes) = lifecycle_observation(model_digest, "p1", "pre", &[(false, false)]);
    let mut snapshots = snapshots;
    snapshots.insert(pre_ref.digest, pre_bytes);
    let pre_call = lifecycle_pre_call(
        &package,
        &packages,
        &snapshots,
        "PlaceRequires",
        ClauseSelectionInput::PreCall {
            snapshot: pre_ref,
            self_object: None,
            parameters: BTreeMap::from([(
                quire_exact::Identifier::new("flags").unwrap(),
                SnapshotValue::record(
                    crate::tc235_k2::id("Flags"),
                    vec![("placed".to_owned(), SnapshotValue::boolean(true))],
                ),
            )]),
        },
    );
    let evaluation = evaluate("PlaceRequires", &pre_call);
    assert_eq!(evaluation.receiver_path(), None);
    assert_eq!(reads_of(&evaluation), vec![None]);
}

/// `clause` admitted from a pre-call `input`.
fn lifecycle_pre_call(
    package: &CheckedPackage,
    packages: &BTreeMap<[u8; 32], Vec<u8>>,
    snapshots: &BTreeMap<[u8; 32], Vec<u8>>,
    clause: &str,
    input: ClauseSelectionInput,
) -> AdmittedObservations {
    let checked = package.graph().state_clause(clause).expect("clause");
    let facts = ClauseFacts {
        identity: checked.identity(),
        kind: checked.kind(),
        context: checked.context(),
        operation: checked.operation().map(|operation| OperationFacts {
            declaring: operation.declaring,
            declaration: operation.declaration.clone(),
        }),
        operations: Default::default(),
    };
    admit_observations(
        package.graph().model_selections(),
        package.graph().scope().types(),
        &facts,
        packages,
        ModelNormalizationLimits::default(),
        &Provisions {
            snapshots,
            invocations: &BTreeMap::new(),
        },
        &ClauseSelection {
            name: clause.to_owned(),
            input,
        },
        ObservationLimits::default(),
    )
    .expect("the pre-call observation admits")
}

/// The lifecycle package with `place(flags: Flags)`, the context field
/// `items` (a sequence of `Flags`) and the precondition `ItemsPlaced`, and
/// the snapshots `p1` and `q1` whose context holds two items, `{placed: true}`
/// and `{placed: false}`, plus the invocation `i1`.
fn items_fixture() -> OrdersFixture {
    use crate::tc235_k2::{declaration, field, id, origin};
    let flags_type = id("Flags");
    let document = lifecycle_document_with("not self.placed", |document| {
        document["types"].as_array_mut().unwrap().push(declaration(
            "Flags",
            "money",
            serde_json::json!({
                "fields": [field("Flags", "placed", "ix://quire/native/Boolean")],
                "operations": [],
            }),
        ));
        let lifecycle = crate::tc235_k2::type_named_mut(document, "OrderLifecycle");
        let mut items = field("OrderLifecycle", "items", &flags_type);
        items["multiplicity"] =
            serde_json::json!({"lower": 0, "upper": 3, "ordered": true, "unique": false});
        lifecycle["fields"].as_array_mut().unwrap().push(items);
        lifecycle["operations"][0]["params"] = serde_json::json!([{
            "identity": id("OrderLifecycle/place/flags"),
            "origin": origin("OrderLifecycle/place/flags"),
            "name": "flags",
            "typeRef": flags_type,
            "presence": "required",
            "nullable": false,
            "defaultKind": "none",
            "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
        }]);
    });
    let (package, _, packages) = model_package(
        &document,
        "pre ItemsPlaced using v on M::OrderLifecycle::place { forall(x in self.items: x.placed) }",
        26,
    )
    .expect("the items package checks");
    let model_digest = packages.keys().next().expect("the package");
    let with_items = |label: &str, observation: &str, contexts: &[(bool, bool)]| {
        let (_, bytes) = lifecycle_observation(model_digest, label, observation, contexts);
        let mut document: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        let item = |placed: bool| {
            serde_json::json!({"record": {
                "type": flags_type,
                "fields": {"placed": {"boolean": placed}},
            }})
        };
        document["contexts"][0]["fields"]["items"] =
            serde_json::json!({"sequence": [item(true), item(false)]});
        let bytes = serde_json::to_vec(&document).unwrap();
        (lifecycle_document_ref(label, &bytes), bytes)
    };
    let (pre_ref, pre_bytes) = with_items("p1", "pre", &[(false, false)]);
    let (post_ref, post_bytes) = with_items("q1", "post", &[(true, true)]);
    let (invocation_ref, invocation_bytes) = lifecycle_invocation_with(
        model_digest,
        &pre_ref,
        &post_ref,
        serde_json::json!({"flags": {"record": {
            "type": flags_type,
            "fields": {"placed": {"boolean": true}},
        }}}),
    );
    (
        package,
        packages,
        BTreeMap::from([(pre_ref.digest, pre_bytes), (post_ref.digest, post_bytes)]),
        BTreeMap::from([(invocation_ref.digest, invocation_bytes)]),
        invocation_ref,
    )
}

/// FR-107 "The receiver path" and "Read paths": a state machine's context
/// with no invocation selected (a precondition selected by `PreCall`) has no
/// location, so a read of its collection field and every read of an element
/// of that collection have no path; with an invocation selected the same
/// field read is located at the invocation's receiver binding with a `field`
/// step.
#[trace("TC-1040", "FR-107-AC-7")]
#[test]
fn a_context_collection_read_under_pre_call_has_no_path() {
    use qsl_semantics::model::observation::SnapshotValue;
    let (package, packages, snapshots, invocations, i1) = items_fixture();
    let evaluate = |observations: &AdmittedObservations| {
        package
            .evaluate_clause(
                &QualifiedName::unqualified("ItemsPlaced").unwrap(),
                observations,
                &mut Meter::new(UNLIMITED),
            )
            .expect("admitted observations evaluate")
    };

    let by_invocation = lifecycle_observations(
        &package,
        &packages,
        "ItemsPlaced",
        &snapshots,
        &invocations,
        i1.clone(),
    )
    .expect("the invocation admits");
    let located = evaluate(&by_invocation);
    assert_eq!(
        reads_of(&located)[0],
        Some(invocation_path(
            "p1",
            InvocationBinding::Receiver,
            vec![ValuePathStep::Field("items".to_owned())],
        ))
    );

    let model_digest = packages.keys().next().expect("the package");
    let _ = model_digest;
    let pre_ref = snapshots
        .iter()
        .find(|(_, bytes)| {
            serde_json::from_slice::<serde_json::Value>(bytes).unwrap()["observation"] == "pre"
        })
        .map(|(digest, bytes)| {
            let reference = lifecycle_document_ref("p1", bytes);
            assert_eq!(&reference.digest, digest);
            reference
        })
        .expect("the pre snapshot");
    let by_pre_call = lifecycle_pre_call(
        &package,
        &packages,
        &snapshots,
        "ItemsPlaced",
        ClauseSelectionInput::PreCall {
            snapshot: pre_ref,
            self_object: None,
            parameters: BTreeMap::from([(
                quire_exact::Identifier::new("flags").unwrap(),
                SnapshotValue::record(
                    crate::tc235_k2::id("Flags"),
                    vec![("placed".to_owned(), SnapshotValue::boolean(true))],
                ),
            )]),
        },
    );
    let unlocated = evaluate(&by_pre_call);
    assert_eq!(unlocated.receiver_path(), None);
    // `self.items`, then `x.placed` for the first and the second item (the
    // `forall` stops at the second, which is false).
    assert_eq!(reads_of(&unlocated), vec![None, None, None]);
}
