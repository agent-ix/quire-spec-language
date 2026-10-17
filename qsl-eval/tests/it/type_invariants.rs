// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-366: the invariant clauses of a model type are classified, parsed and
//! checked (TC-1050), and evaluated once for every value they range over
//! (TC-1051). The reference rows are TC-235 M06 run 4, M13, M14 and M16.

use ix_trace_rs::trace;
use std::collections::BTreeMap;
use std::num::NonZeroU64;

use qsl_eval::value::{
    CheckedPackageEvaluation, InvocationBinding, ObservationIdentity, QualifiedName,
    RuntimeValuePath, ValueInvariantEvaluation, ValueInvariantOutcome, ValuePathStep,
    ValuePathSubject,
};
use qsl_foundation::diagnostic::CatalogCode;
use qsl_package::CheckedPackage;
use qsl_semantics::check::{CheckCause, CheckRefusal, CheckedGraph, PackageDeclarations};
use qsl_semantics::family::FamilyOutcome;
use qsl_semantics::model::accounting::ModelNormalizationLimits;
use qsl_semantics::model::intake::admit_unit;
use qsl_semantics::model::key::DeclarationKey;
use qsl_semantics::model::normalize::{normalize_with_meter, EffectiveView, NormalizeOutcome};
use qsl_semantics::model::observation::{
    admit_observations, AdmittedObservations, AnchorKind, ClauseFacts, ClauseSelection,
    ClauseSelectionInput, DocumentRef, ObservationLimits, OperationFacts, Provisions,
    SelectedAnchor, SelectedObject,
};
use qsl_semantics::model::refusal::{MemberPath, MemberRoot};
use qsl_semantics::model::type_invariant::ClauseRole;
use quire_exact::{
    ChargePoint, FieldValue, InjectedDenial, Meter, Outcome, Timestamp, Uuid, Value as QuireValue,
};
use quire_semantic_value::checking::CheckingLimits;
use serde_json::{json, Value};

use crate::tc235_business_meaning_runtime::{lifecycle_document_ref, lifecycle_identity};
use crate::tc235_k2::{
    clause, declaration, field, id, mutate_k2, operation, origin, type_named_mut, PACKAGE,
};

fn key(node: &str) -> DeclarationKey {
    DeclarationKey {
        package: PACKAGE.to_owned(),
        node: id(node),
    }
}

fn view_of(document: &Value) -> EffectiveView {
    let (domain, _) = crate::tc235_business_meaning_runtime::read_k2(document);
    let (outcome, _) = normalize_with_meter(&domain, ModelNormalizationLimits::default());
    let NormalizeOutcome::Completed(view) = outcome else {
        panic!("the fixture normalizes: {outcome:?}");
    };
    view
}

/// `OrderLifecycle` gets the `quire` clause `Extra` that no transition names.
fn with_extra(document: &mut Value) {
    type_named_mut(document, "OrderLifecycle")["clauses"]
        .as_array_mut()
        .expect("K2 OrderLifecycle clauses")
        .push(clause(
            "OrderLifecycle",
            "Extra",
            "self.placed or not self.placed",
        ));
}

fn lifecycle_state(owner: &str, name: &str) -> Value {
    let node = format!("{owner}/{name}");
    json!({"identity": id(&node), "name": name, "origin": origin(&node)})
}

/// TC-235 M19 run 1: `ExpressLifecycle` specializes `OrderLifecycle` with one
/// transition from `Placed` to its own `Expedited`.
fn with_express_lifecycle(document: &mut Value) {
    let transition = json!({
        "identity": id("ExpressLifecycle/transition0"),
        "from": id("OrderLifecycle/Placed"),
        "to": id("ExpressLifecycle/Expedited"),
        "trigger": id("ExpressLifecycle/expedite"),
        "emits": [],
        "origin": origin("ExpressLifecycle/transition0"),
    });
    document["types"].as_array_mut().unwrap().push(declaration(
        "ExpressLifecycle",
        "lifecycle",
        json!({
            "supertypes": [id("OrderLifecycle")],
            "operations": [operation("ExpressLifecycle", "expedite")],
            "states": [lifecycle_state("ExpressLifecycle", "Expedited")],
            "transitions": [transition],
        }),
    ));
}

/// TC-235 M17 run 1: `TaxedMoney` specializes `Money`.
fn with_taxed_money(document: &mut Value) {
    document["types"].as_array_mut().unwrap().push(declaration(
        "TaxedMoney",
        "money",
        json!({
            "supertypes": [id("Money")],
            "fields": [field("TaxedMoney", "tax_minor", "ix://quire/native/Integer")],
        }),
    ));
}

fn set_clauses(document: &mut Value, owner: &str, clauses: Vec<Value>) {
    type_named_mut(document, owner)["clauses"] = Value::Array(clauses);
}

/// TC-1050 steps 1, 2, 3 and 4: the role of every clause and the effective
/// invariants of every type, equal when the clause records are reordered.
#[trace("TC-1050", "FR-366-AC-1")]
#[test]
fn a_type_level_clause_is_an_invariant_a_guard_or_unevaluated() {
    // Step 1.
    let step_one = mutate_k2(with_extra);
    let roles = view_of(&step_one).type_clause_roles();
    assert_eq!(
        roles.role(&key("OrderLifecycle/Extra")),
        Some(ClauseRole::Invariant)
    );
    assert_eq!(
        roles.role(&key("OrderLifecycle/NotYetPlaced")),
        Some(ClauseRole::Guard)
    );
    assert_eq!(
        roles.effective_invariants(&key("OrderLifecycle")),
        [key("OrderLifecycle/Extra")]
    );

    // Step 2.
    let step_two = mutate_k2(|document| {
        with_extra(document);
        with_express_lifecycle(document);
    });
    let roles = view_of(&step_two).type_clause_roles();
    assert_eq!(
        roles.role(&key("OrderLifecycle/NotYetPlaced")),
        Some(ClauseRole::Guard)
    );
    assert_eq!(
        roles.effective_invariants(&key("ExpressLifecycle")),
        [key("OrderLifecycle/Extra")]
    );

    // Step 3: a clause in another language is kept and never evaluated.
    let in_ocl = mutate_k2(|document| {
        let mut ocl = clause("Money", "NonNegative", "self.amount_minor >= 0");
        ocl["language"] = json!("ocl");
        set_clauses(document, "Money", vec![ocl]);
    });
    let roles = view_of(&in_ocl).type_clause_roles();
    assert_eq!(
        roles.role(&key("Money/NonNegative")),
        Some(ClauseRole::Unevaluated)
    );
    assert!(roles.effective_invariants(&key("Money")).is_empty());
    let in_quire = mutate_k2(|document| {
        set_clauses(
            document,
            "Money",
            vec![clause("Money", "NonNegative", "self.amount_minor >= 0")],
        );
        with_taxed_money(document);
    });
    let roles = view_of(&in_quire).type_clause_roles();
    assert_eq!(
        roles.role(&key("Money/NonNegative")),
        Some(ClauseRole::Invariant)
    );
    for ty in ["Money", "TaxedMoney"] {
        assert_eq!(
            roles.effective_invariants(&key(ty)),
            [key("Money/NonNegative")]
        );
    }

    // Step 4: the classification is a function of the package, not of its
    // clause order.
    let reversed = mutate_k2(|document| {
        with_extra(document);
        type_named_mut(document, "OrderLifecycle")["clauses"]
            .as_array_mut()
            .unwrap()
            .reverse();
    });
    let reversed_roles = view_of(&reversed).type_clause_roles();
    assert_eq!(
        reversed_roles,
        view_of(&step_one).type_clause_roles(),
        "reordered clause records classify equally"
    );
}

/// What compiling a K2-derived document through the driver's steps gives.
enum Compiled {
    Checked(Box<CheckedGraph>),
    Refused(Vec<CheckRefusal>),
    /// The parser refused the text of the clause, at the member.
    Unparsed(DeclarationKey, MemberPath),
}

const UNIT_BODY: &str = "function same using v(a: Boolean): Boolean pure { a }";

type Packages = BTreeMap<[u8; 32], Vec<u8>>;

/// Compile `document` in a `1-draft` unit holding only a function.
fn compile(document: &Value) -> Compiled {
    compile_unit(document, UNIT_BODY).0
}

/// Compile `document` in a `1-draft` unit with `body` as its declarations:
/// select the model, parse its kept operation clauses and its type
/// invariants as the spine driver does, then assemble and check.
fn compile_unit(document: &Value, body: &str) -> (Compiled, Packages) {
    let (_, packages) = crate::tc235_business_meaning_runtime::read_k2(document);
    let digest = packages.keys().next().expect("one package");
    let unit = format!(
        "language \"ix:native\" edition \"1-draft\";\nprofile v = \"quire.value.complete/v1\";\nmodel M = \"test/orders\" version \"1\" digest \"sha256-jcs:{}\";\n{body}\n",
        qsl_semantics::model::key::hex(digest),
    );
    let compiled = compile_parsed(&unit, &packages);
    (compiled, packages)
}

fn compile_parsed(unit: &str, packages: &Packages) -> Compiled {
    let parsed = qsl_cst::parse(
        qsl_foundation::SourceIdentity::new("test", "tc1050", "fixture", "1"),
        "tc1050.native",
        unit.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("the unit parses");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let built = qsl_forms::build_unit(&parsed).expect("the unit builds");
    let models = admit_unit(
        &built.selections().models,
        packages,
        ModelNormalizationLimits::default(),
    )
    .expect("the model selection admits")
    .with_parsed_operation_clauses(|text| {
        qsl_forms::parse_clause_text(text, qsl_cst::Limits::default())
    })
    .expect("the kept operation clauses parse");
    let models = match models.with_parsed_type_invariants(|text| {
        qsl_forms::parse_clause_text(text, qsl_cst::Limits::default())
    }) {
        Ok(models) => models,
        Err(failure) => return Compiled::Unparsed(failure.clause, failure.member),
    };
    let declarations = PackageDeclarations::assemble(
        parsed.source().reference().clone(),
        built,
        models,
        Vec::new(),
    )
    .expect("the unit assembles");
    match declarations.check(CheckingLimits::default()) {
        Ok(graph) => Compiled::Checked(Box::new(graph)),
        Err(refusals) => Compiled::Refused(refusals),
    }
}

fn clauses_member(position: usize) -> MemberPath {
    MemberPath::new(MemberRoot::TypeDefinition)
        .name("clauses")
        .index(position)
}

/// The one refusal of `compiled`, with the code and cause it reports.
fn sole_refusal(compiled: Compiled) -> CheckRefusal {
    let Compiled::Refused(mut refusals) = compiled else {
        panic!("the compile must refuse");
    };
    assert_eq!(refusals.len(), 1, "one refusal, got {refusals:?}");
    refusals.remove(0)
}

fn assert_refused(
    refusal: &CheckRefusal,
    code: &'static str,
    cause: &'static str,
    clause: &str,
    position: usize,
) {
    assert_eq!(
        refusal.cause.catalog_code(),
        Some(CatalogCode::new(code, cause))
    );
    assert_eq!(refusal.type_invariant_clause(), Some(&key(clause)));
    assert_eq!(
        refusal.type_invariant_member(),
        Some(clauses_member(position))
    );
}

/// TC-1050 steps 5 and 6: an invariant that checks, and one that reads `pre`.
#[trace("TC-1050", "FR-366-AC-2")]
#[test]
fn an_invariant_is_checked_and_a_pre_read_refuses_at_its_clause() {
    let money = |text: &str| {
        mutate_k2(|document| {
            set_clauses(
                document,
                "Money",
                vec![clause("Money", "NonNegative", text)],
            );
        })
    };
    let Compiled::Checked(graph) = compile(&money("self.amount_minor >= 0")) else {
        panic!("a Boolean invariant checks");
    };
    let [checked] = graph.type_invariants() else {
        panic!("one checked invariant");
    };
    assert_eq!(checked.clause(), &key("Money/NonNegative"));
    assert_eq!(checked.owner(), &key("Money"));
    assert!(graph.state_clauses().is_empty());

    let refusal = sole_refusal(compile(&money("pre(self.amount_minor) >= 0")));
    assert!(matches!(
        refusal.type_invariant_cause(),
        Some(CheckCause::WrongSnapshot(
            qsl_semantics::check::WrongSnapshotCause::ForbiddenPreRead
        ))
    ));
    assert_refused(
        &refusal,
        "wrong_snapshot",
        "forbidden-pre-read",
        "Money/NonNegative",
        0,
    );

    // Step 6: an event type's invariant.
    let event = |text: &str| {
        mutate_k2(|document| {
            set_clauses(
                document,
                "OrderPlaced",
                vec![clause("OrderPlaced", "Occurred", text)],
            );
        })
    };
    let Compiled::Checked(graph) = compile(&event("self.occurred_at = self.occurred_at")) else {
        panic!("an event invariant checks");
    };
    assert_eq!(
        graph.type_invariants()[0].clause(),
        &key("OrderPlaced/Occurred")
    );
    let refusal = sole_refusal(compile(&event("pre(self.order_id) = self.order_id")));
    assert_refused(
        &refusal,
        "wrong_snapshot",
        "forbidden-pre-read",
        "OrderPlaced/Occurred",
        0,
    );
}

/// TC-1050 steps 7 and 8: a non-Boolean root, a text the parser refuses, a
/// clause in another language and the position of a second clause.
#[trace("TC-1050", "FR-366-AC-3")]
#[test]
fn an_invariant_must_be_boolean_parse_and_name_its_position() {
    let money = |clauses: Vec<Value>| mutate_k2(|document| set_clauses(document, "Money", clauses));
    let named = |text: &str| vec![clause("Money", "NonNegative", text)];

    let refusal = sole_refusal(compile(&money(named("self.amount_minor"))));
    assert_refused(
        &refusal,
        "ill_typed",
        "non-boolean-root",
        "Money/NonNegative",
        0,
    );

    let Compiled::Unparsed(clause_key, member) = compile(&money(named("self.amount_minor >=")))
    else {
        panic!("the parser refuses the text");
    };
    assert_eq!(clause_key, key("Money/NonNegative"));
    assert_eq!(member, clauses_member(0));

    let mut ocl = clause("Money", "NonNegative", "this is not quire (");
    ocl["language"] = json!("ocl");
    let Compiled::Checked(graph) = compile(&money(vec![ocl])) else {
        panic!("an `ocl` text is never parsed");
    };
    assert!(graph.type_invariants().is_empty());

    // Step 8: the second clause of a type is located at position 1.
    let second = compile(&money(vec![
        clause("Money", "Stable", "true"),
        clause("Money", "NonNegative", "pre(self.amount_minor) >= 0"),
    ]));
    let refusal = sole_refusal(second);
    assert_refused(
        &refusal,
        "wrong_snapshot",
        "forbidden-pre-read",
        "Money/NonNegative",
        1,
    );
}

// ---------------------------------------------------------------------------
// TC-1051: evaluation per reachable value
// ---------------------------------------------------------------------------

fn unlimited() -> Meter {
    Meter::new(qsl_semantics::check::SCALAR_LIMITS_UNLIMITED)
}

fn string_json(text: &str) -> Value {
    json!({"kind": "string", "text": text})
}

fn record_json(type_node: &str, fields: Value) -> Value {
    json!({"record": {"type": id(type_node), "fields": fields}})
}

fn money_json(amount: i64) -> Value {
    record_json(
        "Money",
        json!({"amount_minor": {"integer": amount.to_string()}, "currency": string_json("EUR")}),
    )
}

fn uuid_json(last: u8) -> Value {
    json!({"kind": "uuid", "text": format!("00112233-4455-6677-8899-aabbccddee{last:02x}")})
}

fn parameter(owner: &str, operation: &str, name: &str, type_ref: &str) -> Value {
    let node = format!("{owner}/{operation}/{name}");
    json!({
        "identity": id(&node),
        "name": name,
        "origin": origin(&node),
        "typeRef": type_ref,
        "presence": "required",
        "nullable": false,
        "defaultKind": "none",
        "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true},
    })
}

/// TC-1051 fixture **F**: K2 with `Money/NonNegative` (`self.amount_minor >=
/// 0`) and `Order` given the field `quoted: Money`, the bounded sequence
/// `history: Money {0, 3}`, the optional field `discount: Money` and the
/// operation `requote(amount: Money)`, plus the closed population `orders`.
/// `extra` applies further changes.
fn fixture_f(extra: impl FnOnce(&mut Value)) -> Value {
    mutate_k2(|document| {
        set_clauses(
            document,
            "Money",
            vec![clause("Money", "NonNegative", "self.amount_minor >= 0")],
        );
        let order = type_named_mut(document, "Order");
        let fields = order["fields"].as_array_mut().unwrap();
        fields.push(field("Order", "quoted", &id("Money")));
        let mut history = field("Order", "history", &id("Money"));
        history["multiplicity"] = json!({"lower": 0, "upper": 3, "ordered": true, "unique": false});
        fields.push(history);
        let mut discount = field("Order", "discount", &id("Money"));
        discount["presence"] = json!("optional");
        fields.push(discount);
        let mut requote = operation("Order", "requote");
        requote["params"] = json!([parameter("Order", "requote", "amount", &id("Money"))]);
        order["operations"] = json!([requote]);
        document["populations"] = json!([{
            "identity": id("orders"), "displayName": id("orders"),
            "kind": {"module": crate::tc235_k2::BUSINESS, "name": "population"},
            "members": [id("Order")], "extent": "closed",
            "origin": origin("orders"),
        }]);
        extra(document);
    })
}

/// The `Order` object `key` of F: `total` 5, the given `quoted`, `history`
/// and `discount`, and any `extra` fields.
fn order_json(
    key: &str,
    quoted: Value,
    history: &[i64],
    discount: Option<i64>,
    extra: Vec<(&str, Value)>,
) -> Value {
    let mut fields = json!({
        "order_id": uuid_json(1),
        "total": money_json(5),
        "quoted": quoted,
        "history": {"sequence": history.iter().map(|amount| money_json(*amount)).collect::<Vec<_>>()},
        "discount": discount.map_or_else(
            || json!({"absent": {}}),
            |amount| json!({"present": money_json(amount)}),
        ),
    });
    for (name, value) in extra {
        fields[name] = value;
    }
    json!({"key": key, "type": id("Order"), "fields": fields})
}

fn snapshot_doc(
    model: &[u8; 32],
    label: &str,
    observation: &str,
    population: &str,
    objects: &[Value],
) -> (DocumentRef, Vec<u8>) {
    let identity = unsigned_ref(label);
    let mut document = json!({
        "format": "quire.state.snapshot/v1",
        "identity": lifecycle_identity(&identity),
        "observation": observation,
        "model": model_json(model),
        "populations": [{"population": population, "complete": true, "objects": objects}],
    });
    if observation == "current" {
        document["anchor"] = json!({"kind": "handler", "name": "validate"});
    }
    let bytes = serde_json::to_vec(&document).expect("snapshot serializes");
    (lifecycle_document_ref(label, &bytes), bytes)
}

fn unsigned_ref(label: &str) -> DocumentRef {
    DocumentRef {
        authority: "test".to_owned(),
        identity: label.to_owned(),
        revision_namespace: "tc235".to_owned(),
        revision: "1".to_owned(),
        digest: [0; 32],
    }
}

fn model_json(model: &[u8; 32]) -> Value {
    json!({
        "identity": PACKAGE,
        "version": "1",
        "digest": format!("sha256-jcs:{}", qsl_semantics::model::key::hex(model)),
    })
}

#[allow(clippy::too_many_arguments)]
fn invocation_doc(
    model: &[u8; 32],
    label: &str,
    context: &str,
    operation: &str,
    self_population: &str,
    self_key: &str,
    (pre, post): (&DocumentRef, &DocumentRef),
    parameters: Value,
) -> (DocumentRef, Vec<u8>) {
    let reference = |reference: &DocumentRef| {
        json!({
            "identity": lifecycle_identity(reference),
            "digest": format!("sha256-jcs:{}", qsl_semantics::model::key::hex(&reference.digest)),
        })
    };
    let bytes = serde_json::to_vec(&json!({
        "format": "quire.state.invocation/v1",
        "identity": lifecycle_identity(&unsigned_ref(label)),
        "model": model_json(model),
        "context": id(context),
        "operation": operation,
        "self": {"population": self_population, "key": self_key},
        "pre": reference(pre),
        "post": reference(post),
        "parameters": parameters,
        "result": null,
        "created": [],
        "deleted": [],
    }))
    .expect("invocation serializes");
    (lifecycle_document_ref(label, &bytes), bytes)
}

fn clause_facts(package: &CheckedPackage, name: &str) -> ClauseFacts {
    let checked = package
        .graph()
        .state_clause(name)
        .unwrap_or_else(|| panic!("the unit declares {name}"));
    ClauseFacts {
        identity: checked.identity(),
        kind: checked.kind(),
        context: checked.context(),
        operation: checked.operation().map(|operation| OperationFacts {
            declaring: operation.declaring,
            declaration: operation.declaration.clone(),
        }),
    }
}

/// A checked package, its offered package documents and the documents the
/// admission of an observation reads.
struct Documents {
    package: CheckedPackage,
    packages: Packages,
    snapshots: BTreeMap<[u8; 32], Vec<u8>>,
    invocations: BTreeMap<[u8; 32], Vec<u8>>,
}

impl Documents {
    fn model(&self) -> [u8; 32] {
        *self.packages.keys().next().expect("one package")
    }

    fn add_snapshot(&mut self, snapshot: (DocumentRef, Vec<u8>)) -> DocumentRef {
        self.snapshots.insert(snapshot.0.digest, snapshot.1);
        snapshot.0
    }

    fn admit(&self, clause: &str, input: ClauseSelectionInput) -> AdmittedObservations {
        admit_observations(
            self.package.graph().model_selections(),
            self.package.graph().scope().types(),
            &clause_facts(&self.package, clause),
            &self.packages,
            qsl_semantics::model::accounting::ModelNormalizationLimits::default(),
            &Provisions {
                snapshots: &self.snapshots,
                invocations: &self.invocations,
            },
            &ClauseSelection {
                name: clause.to_owned(),
                input,
            },
            ObservationLimits::default(),
        )
        .unwrap_or_else(|failure| panic!("{clause} admits: {failure:?}"))
    }
}

fn documents_with(document: &Value, body: &str) -> Documents {
    let (compiled, packages) = compile_unit(document, body);
    let Compiled::Checked(graph) = compiled else {
        panic!("the fixture compiles");
    };
    Documents {
        package: CheckedPackage::link(*graph),
        packages,
        snapshots: BTreeMap::new(),
        invocations: BTreeMap::new(),
    }
}

/// The observations FR-106's `ValueInvariants { snapshot: s1, invocation }`
/// selection admits: the current snapshot `s1` holding `objects` and the
/// invocation `label` of `operation` on the object `o1` of `context`, whose
/// `pre` snapshot `s2` and `post` snapshot `s3` hold the same `objects`.
///
/// The unit declares an invariant `Any` and a postcondition `Done` so that
/// the two documents are admitted by the selections that exist today.
fn invoked(
    documents: &mut Documents,
    objects: &[Value],
    label: &str,
    operation: &str,
    parameters: Value,
) -> AdmittedObservations {
    let model = documents.model();
    let population = id("orders");
    let s1 = documents.add_snapshot(snapshot_doc(&model, "s1", "current", &population, objects));
    let s2 = documents.add_snapshot(snapshot_doc(&model, "s2", "pre", &population, objects));
    let s3 = documents.add_snapshot(snapshot_doc(&model, "s3", "post", &population, objects));
    let (invocation, bytes) = invocation_doc(
        &model,
        label,
        "Order",
        operation,
        &population,
        "o1",
        (&s2, &s3),
        parameters,
    );
    documents.invocations.insert(invocation.digest, bytes);
    let current = documents.admit(
        "Any",
        ClauseSelectionInput::Current {
            snapshot: s1,
            anchor: SelectedAnchor {
                kind: AnchorKind::Handler,
                name: "validate".to_owned(),
            },
            self_object: SelectedObject {
                population: population.clone(),
                key: "o1".to_owned(),
            },
        },
    );
    let invoked = documents.admit("Done", ClauseSelectionInput::Invocation { invocation });
    AdmittedObservations {
        current: current.current,
        ..invoked
    }
}

fn order_unit(operation: &str) -> String {
    format!(
        "invariant Any using v on M::Order at current {{ true }}\npost Done using v on M::Order::{operation} {{ true }}"
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

fn member(name: &str) -> ValuePathStep {
    ValuePathStep::Member(name.to_owned())
}

/// The path of the object `key` of the current observation `label`.
fn object_path(
    observations: &AdmittedObservations,
    label: &str,
    key: &str,
    steps: Vec<ValuePathStep>,
) -> RuntimeValuePath {
    let current = observations
        .current
        .as_ref()
        .expect("a current observation");
    let reference = current
        .environment
        .references()
        .iter()
        .find(|reference| reference.object().as_str() == key)
        .unwrap_or_else(|| panic!("the observation holds {key}"))
        .clone();
    RuntimeValuePath {
        observation: identity_of(label),
        subject: ValuePathSubject::Object(reference),
        steps,
    }
}

fn parameter_path(observation: &str, invocation: &str, name: &str) -> RuntimeValuePath {
    RuntimeValuePath {
        observation: identity_of(observation),
        subject: ValuePathSubject::Invocation {
            invocation: identity_of(invocation),
            binding: InvocationBinding::Parameter(name.to_owned()),
        },
        steps: Vec::new(),
    }
}

/// Each result as `(clause, path, holds)`, in the order reported.
fn rows(evaluation: &ValueInvariantEvaluation) -> Vec<(String, RuntimeValuePath, bool)> {
    evaluation
        .results
        .iter()
        .map(|result| {
            (
                result.clause.node.clone(),
                result.path.clone(),
                result.holds(),
            )
        })
        .collect()
}

fn non_negative() -> String {
    id("Money/NonNegative")
}

fn evaluate(
    documents: &Documents,
    observations: &AdmittedObservations,
) -> ValueInvariantEvaluation {
    documents
        .package
        .evaluate_value_invariants(observations, &mut unlimited())
        .expect("the evaluation completes")
}

/// The F observation of TC-1051 step 1.
fn step_one(object_fields: impl FnOnce(Value) -> Value) -> (Documents, AdmittedObservations) {
    let mut documents = documents_with(&fixture_f(|_| {}), &order_unit("requote"));
    let mut o1 = order_json("o1", money_json(-5), &[1, -2], Some(-1), vec![]);
    o1["fields"] = object_fields(o1["fields"].take());
    let observations = invoked(
        &mut documents,
        &[o1],
        "i1",
        "requote",
        json!({"amount": money_json(-7)}),
    );
    (documents, observations)
}

/// TC-1051 step 1 (TC-235 M13 run 1): six results of `NonNegative`, one for
/// each `Money` value the observation and the invocation reach, in the FR-207
/// key order of their paths, each false result a Boolean with no code or
/// cause.
#[trace("TC-1051", "FR-366-AC-4")]
#[test]
fn every_reachable_money_value_has_one_result_at_its_path() {
    let (documents, observations) = step_one(|fields| fields);
    let evaluation = evaluate(&documents, &observations);
    let at = |steps| object_path(&observations, "s1", "o1", steps);
    assert_eq!(
        rows(&evaluation),
        [
            (
                non_negative(),
                at(vec![member("discount"), ValuePathStep::OptionValue]),
                false
            ),
            (
                non_negative(),
                at(vec![member("history"), ValuePathStep::Index(0)]),
                true
            ),
            (
                non_negative(),
                at(vec![member("history"), ValuePathStep::Index(1)]),
                false
            ),
            (non_negative(), at(vec![member("quoted")]), false),
            (non_negative(), at(vec![member("total")]), true),
            (non_negative(), parameter_path("s2", "i1", "amount"), false),
        ]
    );
    assert_eq!(evaluation.outcome, ValueInvariantOutcome::False);
    for result in &evaluation.results {
        assert!(
            matches!(
                result.evaluation.outcome,
                FamilyOutcome::Evaluated(Outcome::Completed(QuireValue::Boolean(_)))
            ),
            "a result is a completed Boolean, got {:?}",
            result.evaluation
        );
    }
}

fn reversed(fields: Value) -> Value {
    Value::Object(
        fields
            .as_object()
            .expect("an object's fields")
            .iter()
            .rev()
            .map(|(name, value)| (name.clone(), value.clone()))
            .collect(),
    )
}

fn run(
    documents: &Documents,
    observations: &AdmittedObservations,
) -> (Vec<(String, RuntimeValuePath, bool)>, Vec<ChargePoint>) {
    let mut meter = unlimited();
    let evaluation = documents
        .package
        .evaluate_value_invariants(observations, &mut meter)
        .expect("the evaluation completes");
    (
        rows(&evaluation),
        crate::support::admitted_charge_points(&meter),
    )
}

/// TC-1051 steps 2 and 3: the listing order of an object's fields is no part
/// of a result, two evaluations give equal results and charge logs, and the
/// result for a value does not depend on the rest of the observation.
#[trace("TC-1051", "FR-366-AC-5")]
#[test]
fn a_result_depends_on_its_value_alone() {
    let (documents, observations) = step_one(|fields| fields);
    let (first, charges) = run(&documents, &observations);
    let (again, charges_again) = run(&documents, &observations);
    assert_eq!(first, again);
    assert_eq!(charges, charges_again);

    let (listed_documents, listed) = step_one(reversed);
    assert_eq!(run(&listed_documents, &listed).0, first);

    // `history[0]` changes; the other results stand.
    let mut changed = documents_with(&fixture_f(|_| {}), &order_unit("requote"));
    let o1 = order_json("o1", money_json(-5), &[9, -2], Some(-1), vec![]);
    let observations_changed = invoked(
        &mut changed,
        &[o1],
        "i1",
        "requote",
        json!({"amount": money_json(-7)}),
    );
    let (after, _) = run(&changed, &observations_changed);
    let holds_at = |rows: &[(String, RuntimeValuePath, bool)], steps: &[ValuePathStep]| {
        rows.iter()
            .find(|(_, path, _)| path.steps == steps)
            .map(|(_, _, holds)| *holds)
    };
    assert_eq!(
        holds_at(&after, &[member("quoted")]),
        holds_at(&first, &[member("quoted")])
    );
    assert_eq!(
        holds_at(&after, &[member("history"), ValuePathStep::Index(0)]),
        Some(true)
    );
}

/// TC-1051 step 4 (TC-235 M14 run 1): the invariant of an event type has no
/// result over an observation that holds no value of the type.
#[trace("TC-1051", "FR-366-AC-6")]
#[test]
fn an_event_type_no_value_reaches_has_no_result() {
    let document = fixture_f(|document| {
        set_clauses(
            document,
            "OrderPlaced",
            vec![clause(
                "OrderPlaced",
                "Occurred",
                "self.occurred_at = self.occurred_at",
            )],
        );
        type_named_mut(document, "Money")["clauses"]
            .as_array_mut()
            .unwrap()
            .push(clause("Money", "Stable", "true"));
    });
    let mut documents = documents_with(&document, &order_unit("requote"));
    let o1 = order_json("o1", money_json(2), &[1], None, vec![]);
    let observations = invoked(
        &mut documents,
        &[o1],
        "i1",
        "requote",
        json!({"amount": money_json(7)}),
    );
    let evaluation = evaluate(&documents, &observations);
    assert!(rows(&evaluation)
        .iter()
        .all(|(clause, _, _)| *clause != id("OrderPlaced/Occurred")));
    assert!(
        !evaluation.results.is_empty(),
        "the Money values are checked"
    );
    assert_eq!(evaluation.outcome, ValueInvariantOutcome::True);
}

fn holds_where(
    evaluation: &ValueInvariantEvaluation,
    clause: &str,
    steps: &[ValuePathStep],
) -> Vec<bool> {
    evaluation
        .results
        .iter()
        .filter(|result| result.clause.node == clause && result.path.steps == steps)
        .map(|result| result.holds())
        .collect()
}

/// TC-1051 step 5: a subtype value at a `Money` slot is checked against its
/// effective invariants, and a record nested in a record is located by its
/// field steps.
#[trace("TC-1051", "FR-366-AC-7")]
#[test]
fn a_subtype_value_and_a_nested_record_are_evaluated_at_their_paths() {
    let document = fixture_f(|document| {
        with_taxed_money(document);
        document["types"].as_array_mut().unwrap().push(declaration(
            "Pair",
            "money",
            json!({"fields": [
                field("Pair", "first", &id("Money")),
                field("Pair", "second", &id("Money")),
            ]}),
        ));
        type_named_mut(document, "Order")["fields"]
            .as_array_mut()
            .unwrap()
            .push(field("Order", "pair", &id("Pair")));
    });
    let mut documents = documents_with(&document, &order_unit("requote"));
    let taxed = record_json(
        "TaxedMoney",
        json!({"amount_minor": {"integer": "-5"}, "currency": string_json("EUR"),
               "tax_minor": {"integer": "20"}}),
    );
    let pair = record_json(
        "Pair",
        json!({"first": money_json(-1), "second": money_json(2)}),
    );
    let o1 = order_json("o1", taxed, &[], None, vec![("pair", pair)]);
    let observations = invoked(
        &mut documents,
        &[o1],
        "i1",
        "requote",
        json!({"amount": money_json(7)}),
    );
    let evaluation = evaluate(&documents, &observations);
    let clause = non_negative();
    assert_eq!(
        holds_where(&evaluation, &clause, &[member("quoted")]),
        [false],
        "the TaxedMoney value at quoted is evaluated once"
    );
    assert_eq!(
        holds_where(
            &evaluation,
            &clause,
            &[member("pair"), ValuePathStep::Field("first".to_owned())]
        ),
        [false]
    );
    assert_eq!(
        holds_where(
            &evaluation,
            &clause,
            &[member("pair"), ValuePathStep::Field("second".to_owned())]
        ),
        [true]
    );
}

/// TC-1051 step 6: an event value is evaluated at the parameter binding it is
/// bound to, rooted at the pre snapshot of the invocation.
///
/// Intake refuses an event type as the declared type of an operation
/// parameter and of a field, so no admitted observation holds an event value
/// (carried to QSL-793, FR-366-AC-7). The value is supplied to the admitted
/// observations here, to test the evaluator.
#[trace("TC-1051", "FR-366-AC-7")]
#[test]
fn an_event_value_is_evaluated_at_the_parameter_binding_it_is_bound_to() {
    let document = fixture_f(|document| {
        set_clauses(
            document,
            "OrderPlaced",
            vec![clause("OrderPlaced", "Occurred", "false")],
        );
    });
    let mut documents = documents_with(&document, &order_unit("requote"));
    let o1 = order_json("o1", money_json(2), &[], None, vec![]);
    let mut observations = invoked(
        &mut documents,
        &[o1],
        "i4",
        "requote",
        json!({"amount": money_json(7)}),
    );
    let types = documents.package.graph().scope().types();
    let placed = types
        .composites()
        .find(|declaration| declaration.name().ends_with("OrderPlaced"))
        .expect("the package declares OrderPlaced")
        .key();
    let event = types
        .record(
            placed,
            vec![
                (
                    "occurred_at",
                    FieldValue::Present(QuireValue::Timestamp(
                        Timestamp::from_canonical_text("0").expect("canonical Timestamp"),
                    )),
                ),
                (
                    "order_id",
                    FieldValue::Present(QuireValue::Uuid(
                        Uuid::from_canonical_text("00112233-4455-6677-8899-aabbccddee07")
                            .expect("canonical UUID"),
                    )),
                ),
            ],
        )
        .expect("the event value admits");
    observations.parameters = vec![("placed".to_owned(), event)];
    let evaluation = evaluate(&documents, &observations);
    let occurred: Vec<_> = rows(&evaluation)
        .into_iter()
        .filter(|(clause, _, _)| *clause == id("OrderPlaced/Occurred"))
        .collect();
    assert_eq!(
        occurred,
        [(
            id("OrderPlaced/Occurred"),
            parameter_path("s2", "i4", "placed"),
            false
        )]
    );
}

/// An invariant of a record value type that reads an object, a relationship
/// or a population is refused at check: a record value is evaluated with no
/// object universe (FR-366).
#[trace("TC-1050", "FR-366-AC-2")]
#[test]
fn a_record_invariant_that_reads_an_object_is_refused() {
    let document = fixture_f(|document| {
        let mut buddy = field("Pair", "buddy", &id("Order"));
        buddy["presence"] = json!("optional");
        document["types"].as_array_mut().unwrap().push(declaration(
            "Pair",
            "money",
            json!({"fields": [buddy]}),
        ));
        set_clauses(
            document,
            "Pair",
            vec![clause(
                "Pair",
                "Peer",
                "present(self.buddy) implies value(self.buddy).total.amount_minor >= 0",
            )],
        );
    });
    let (compiled, _) = compile_unit(&document, UNIT_BODY);
    let refusal = sole_refusal(compiled);
    assert_eq!(refusal.type_invariant_clause(), Some(&key("Pair/Peer")));
    assert_eq!(refusal.type_invariant_member(), Some(clauses_member(0)));
    assert!(matches!(
        refusal.type_invariant_cause(),
        Some(CheckCause::ObjectReadInValueInvariant)
    ));
    assert_eq!(
        refusal.cause.catalog_code(),
        Some(CatalogCode::new("unsupported_construct", "expression-form"))
    );
}

/// TC-1051 step 7: a `reference` field is not followed; the referenced
/// object's values are reached once, at the object's own identity.
#[trace("TC-1051", "FR-366-AC-7")]
#[test]
fn a_reference_is_not_followed() {
    let document = fixture_f(|document| {
        let mut buddy = field("Order", "buddy", &id("Order"));
        buddy["presence"] = json!("optional");
        type_named_mut(document, "Order")["fields"]
            .as_array_mut()
            .unwrap()
            .push(buddy);
    });
    let mut documents = documents_with(&document, &order_unit("requote"));
    let o1 = order_json(
        "o1",
        money_json(5),
        &[],
        None,
        vec![(
            "buddy",
            json!({"present": {"reference": {"population": id("orders"), "key": "o2"}}}),
        )],
    );
    let o2 = order_json(
        "o2",
        money_json(-3),
        &[],
        None,
        vec![("buddy", json!({"absent": {}}))],
    );
    let observations = invoked(
        &mut documents,
        &[o1, o2],
        "i1",
        "requote",
        json!({"amount": money_json(7)}),
    );
    let evaluation = evaluate(&documents, &observations);
    let violations: Vec<_> = rows(&evaluation)
        .into_iter()
        .filter(|(_, _, holds)| !holds)
        .collect();
    assert_eq!(
        violations,
        [(
            non_negative(),
            object_path(&observations, "s1", "o2", vec![member("quoted")]),
            false
        )]
    );
    assert!(
        evaluation
            .results
            .iter()
            .all(|result| result.path.steps.first() != Some(&member("buddy"))),
        "nothing is located under the reference field"
    );
}

fn denying(point: ChargePoint, occurrence: u64) -> Meter {
    unlimited().with_injected_denial(InjectedDenial {
        point,
        occurrence: NonZeroU64::new(occurrence).expect("nonzero occurrence"),
    })
}

/// TC-1051 steps 9 and 10: one meter is shared by every value, a denied
/// charge stops the evaluation with the results before it, and an `ocl`
/// clause has no result.
#[trace("TC-1051", "FR-366-AC-8")]
#[test]
fn one_budget_is_shared_and_a_denied_charge_stops_the_evaluation() {
    let (documents, observations) = step_one(|fields| fields);
    // Each value costs three charges, in this order.
    let (_, charges) = run(&documents, &observations);
    let per_value = [
        ChargePoint::OrderingOperands,
        ChargePoint::OrderingArithmetic,
        ChargePoint::OrderingResultRetain,
    ];
    assert_eq!(charges, per_value.repeat(6));
    // The fourth is the `ordering.operands` of the second value.
    let fourth = documents
        .package
        .evaluate_value_invariants(
            &observations,
            &mut denying(ChargePoint::OrderingOperands, 2),
        )
        .expect("the evaluation returns");
    assert_eq!(
        rows(&fourth),
        [(
            non_negative(),
            object_path(
                &observations,
                "s1",
                "o1",
                vec![member("discount"), ValuePathStep::OptionValue]
            ),
            false
        )]
    );
    let ValueInvariantOutcome::Incomplete(incomplete) = &fourth.outcome else {
        panic!("a denied charge is Incomplete, got {:?}", fourth.outcome);
    };
    assert_eq!(incomplete.charge_point, ChargePoint::OrderingOperands);
    assert_eq!(
        fourth.outcome.catalog_code(),
        Some(CatalogCode::new(
            "resource_exhausted",
            "insufficient-next-charge"
        ))
    );
    assert_eq!(evaluation_true_code(&documents, &observations), None);

    let third = documents
        .package
        .evaluate_value_invariants(
            &observations,
            &mut denying(ChargePoint::OrderingResultRetain, 1),
        )
        .expect("the evaluation returns");
    assert!(third.results.is_empty());
    assert!(matches!(
        third.outcome,
        ValueInvariantOutcome::Incomplete(_)
    ));

    // An `ocl` clause is kept and has no result.
    let with_ocl = fixture_f(|document| {
        let mut stable = clause("Money", "Stable", "true");
        stable["language"] = json!("ocl");
        type_named_mut(document, "Money")["clauses"]
            .as_array_mut()
            .unwrap()
            .push(stable);
    });
    let mut documents = documents_with(&with_ocl, &order_unit("requote"));
    let o1 = order_json("o1", money_json(-5), &[1, -2], Some(-1), vec![]);
    let observations = invoked(
        &mut documents,
        &[o1],
        "i1",
        "requote",
        json!({"amount": money_json(-7)}),
    );
    let evaluation = evaluate(&documents, &observations);
    assert_eq!(evaluation.results.len(), 6);
    assert!(evaluation
        .results
        .iter()
        .all(|result| result.clause.node == non_negative()));
}

/// TC-1051 step 11, the process half (TC-235 M16 run 1): the invariant of a
/// process is evaluated once for every run of the observation, and
/// `evaluate_clause` still resolves unit state clauses only.
#[trace("TC-1051", "FR-366-AC-9")]
#[test]
fn a_process_invariant_has_one_result_for_each_run() {
    let document = mutate_k2(|document| {
        let process = type_named_mut(document, "Fulfilment");
        process["fields"].as_array_mut().unwrap().push(field(
            "Fulfilment",
            "attempts",
            "ix://quire/native/Integer",
        ));
        process["clauses"] = json!([clause(
            "Fulfilment",
            "NonNegativeAttempts",
            "self.attempts >= 0"
        )]);
        document["populations"] = json!([{
            "identity": id("runs"), "displayName": id("runs"),
            "kind": {"module": crate::tc235_k2::BUSINESS, "name": "population"},
            "members": [id("Fulfilment")], "extent": "closed",
            "origin": origin("runs"),
        }]);
    });
    let mut documents = documents_with(
        &document,
        "invariant AnyRun using v on M::Fulfilment at current { true }",
    );
    let run_json = |key: &str, attempts: i64| {
        json!({"key": key, "type": id("Fulfilment"), "fields": {
            "correlation_id": uuid_json(attempts.unsigned_abs() as u8 + 1),
            "attempts": {"integer": attempts.to_string()},
        }})
    };
    let model = documents.model();
    let population = id("runs");
    let s1 = documents.add_snapshot(snapshot_doc(
        &model,
        "s1",
        "current",
        &population,
        &[run_json("r1", 0), run_json("r2", -1)],
    ));
    let observations = documents.admit(
        "AnyRun",
        ClauseSelectionInput::Current {
            snapshot: s1,
            anchor: SelectedAnchor {
                kind: AnchorKind::Handler,
                name: "validate".to_owned(),
            },
            self_object: SelectedObject {
                population,
                key: "r1".to_owned(),
            },
        },
    );
    let evaluation = evaluate(&documents, &observations);
    let clause = id("Fulfilment/NonNegativeAttempts");
    assert_eq!(
        rows(&evaluation),
        [
            (
                clause.clone(),
                object_path(&observations, "s1", "r1", vec![]),
                true
            ),
            (
                clause,
                object_path(&observations, "s1", "r2", vec![]),
                false
            ),
        ]
    );
    assert_eq!(evaluation.outcome, ValueInvariantOutcome::False);

    let unknown = documents.package.evaluate_clause(
        &QualifiedName::unqualified("NonNegativeAttempts").unwrap(),
        &observations,
        &mut unlimited(),
    );
    assert!(
        matches!(
            unknown,
            Err(qsl_eval::value::CallFailure::Input(
                quire_semantic_value::call::InputRefusal::UnknownClause(_)
            ))
        ),
        "a type invariant is no unit state clause: {unknown:?}"
    );
}

/// TC-1051 step 11, the state machine half: the invariants of the one
/// machine the invocation names are evaluated over its context in each side
/// of the invocation, each a separate value located at the invocation's
/// receiver binding; a guard has no result.
#[trace("TC-1051", "FR-366-AC-9")]
#[test]
fn a_state_machine_invariant_is_evaluated_over_each_side_of_the_invocation() {
    use crate::tc235_business_meaning_runtime::{
        lifecycle_document_with, lifecycle_invocation_with, lifecycle_observation,
        lifecycle_observations,
    };
    let flags_type = id("Flags");
    let document = lifecycle_document_with("not self.placed", |document| {
        with_extra(document);
        document["types"].as_array_mut().unwrap().push(declaration(
            "Flags",
            "money",
            json!({
                "fields": [field("Flags", "placed", "ix://quire/native/Boolean")],
                "operations": [],
            }),
        ));
        let lifecycle = type_named_mut(document, "OrderLifecycle");
        lifecycle["operations"][0]["params"] =
            json!([parameter("OrderLifecycle", "place", "flags", &flags_type)]);
    });
    let documents = documents_with(
        &document,
        "post FlagsPlaced using v on M::OrderLifecycle::place { flags.placed }",
    );
    let model = documents.model();
    let (p1, p1_bytes) = lifecycle_observation(&model, "p1", "pre", &[(false, false)]);
    let (q1, q1_bytes) = lifecycle_observation(&model, "q1", "post", &[(true, true)]);
    let (i1, i1_bytes) = lifecycle_invocation_with(
        &model,
        &p1,
        &q1,
        json!({"flags": {"record": {"type": flags_type, "fields": {"placed": {"boolean": true}}}}}),
    );
    let admitted = lifecycle_observations(
        &documents.package,
        &documents.packages,
        "FlagsPlaced",
        &BTreeMap::from([(p1.digest, p1_bytes), (q1.digest, q1_bytes)]),
        &BTreeMap::from([(i1.digest, i1_bytes)]),
        i1,
    )
    .expect("the invocation admits");
    let evaluation = evaluate(&documents, &admitted);
    let receiver = |label: &str| RuntimeValuePath {
        observation: identity_of(label),
        subject: ValuePathSubject::Invocation {
            invocation: identity_of("i1"),
            binding: InvocationBinding::Receiver,
        },
        steps: Vec::new(),
    };
    let extra = id("OrderLifecycle/Extra");
    let consistent = id("OrderLifecycle/Consistent");
    assert_eq!(
        rows(&evaluation),
        [
            (consistent.clone(), receiver("p1"), true),
            (extra.clone(), receiver("p1"), true),
            (consistent, receiver("q1"), false),
            (extra, receiver("q1"), true),
        ]
    );
    assert!(
        rows(&evaluation)
            .iter()
            .all(|(clause, _, _)| *clause != id("OrderLifecycle/NotYetPlaced")),
        "a guard has no result"
    );
}

fn evaluation_true_code(
    documents: &Documents,
    observations: &AdmittedObservations,
) -> Option<CatalogCode> {
    let completed = evaluate(documents, observations);
    // The unlimited run of the same observation completes.
    assert_eq!(completed.outcome, ValueInvariantOutcome::False);
    completed.outcome.catalog_code()
}
