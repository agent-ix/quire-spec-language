// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-517-AC-20 over R (TC-1312 row 20): the `Healthy` invariant of
//! `examples/checked-handoff/unit-event-position.native` sums its receipts
//! over `M::Total` and is evaluated over receipts of 2 U and 3 U.

use std::collections::BTreeMap;

use ix_trace_rs::trace;
use qsl_eval::value::{CheckedPackageEvaluation, QualifiedName};
use qsl_foundation::SourceIdentity;
use qsl_replay::spine::{
    self, CheckedUnit, DependencyInput, ParseRequest, SpineLimits, SuppliedLibrary,
};
use qsl_semantics::model::object_environment::ObjectEnvironment;
use qsl_semantics::model::observation::{AdmittedObservations, DocumentRef, Observation, SelfKind};
use quire_exact::{
    admit_text, Cancel, FieldValue, Integer, Meter, NodeKey, ObjectId, ObjectReference, Outcome,
    PathSegment, TextPayload, TextProfile, TextType, UniverseId, Value,
};
use quire_semantic_value::object_closure::ObjectClosure;

const DIR: &str = "examples/checked-handoff";
const LIBRARY: &str = "agent-ix/checked-handoff-library";

/// R's event-position unit with a function that forms the receipts 2 U and
/// 3 U, so the test takes the quantities from the real checker.
fn source() -> String {
    let mut text = std::fs::read_to_string(format!("{DIR}/unit-event-position.native"))
        .expect("the event-position unit");
    text.push_str(
        "\nfunction twoAndThree using v(): Sequence<Quantity<U>>[0, 5] pure { sequence[2 U, 3 U] }\n\
         type Count = Int[0, 5];\n\
         function amounts using v(): Sequence<M::Amount>[0, 5] pure { sequence[2 U, 2 U, 3 U] }\n",
    );
    for (index, (_, body, _)) in
        super::bare_recovery_binder::duplicate_query_cases("amounts()", " U", "Count")
            .into_iter()
            .enumerate()
    {
        text.push_str(&format!(
            "function query{index} using v(): Boolean pure {{ {body} }}\n"
        ));
    }
    text
}

/// R's unit, library and model checked together by the production spine.
fn checked() -> CheckedUnit {
    let source = source();
    let library = std::fs::read(format!("{DIR}/library.native")).expect("the library");
    let model = std::fs::read(format!("{DIR}/model.semantic-ir.json")).expect("the model");
    let dependencies = DependencyInput::new([SuppliedLibrary {
        identity: LIBRARY.to_owned(),
        source: SourceIdentity::new("a", "library"),
        path: "library.native".to_owned(),
        bytes: library,
    }])
    .expect("the library is supplied once");
    let packages = qsl_semantics::model::intake::package_input([model.as_slice()])
        .expect("the model is admitted");
    let limits = SpineLimits::default();
    let cancel = Cancel::new();
    let identity = SourceIdentity::new("a", "unit");
    let parsed = spine::parse(
        &ParseRequest {
            source: &identity,
            path: "unit.native",
            bytes: source.as_bytes(),
        },
        limits.source,
        &cancel,
    )
    .expect("R parses")
    .into_value();
    let models = spine::select(&parsed, &packages, limits.model, &cancel)
        .expect("R selects its model")
        .into_value();
    spine::check(
        &parsed,
        "r".to_owned(),
        &models,
        &dependencies,
        limits,
        &cancel,
    )
    .expect("R checks")
    .into_value()
}

/// A `Text[0, 128; nfc]` value holding `payload`.
fn text(payload: &str) -> Value {
    let admitted = admit_text(
        &TextPayload::from_utf8(payload.as_bytes()).expect("valid UTF-8"),
        &TextType::new(0, 128, TextProfile::Nfc).expect("a text type"),
        &mut Meter::new(qsl_semantics::check::SCALAR_LIMITS_UNLIMITED),
    )
    .completed()
    .expect("the text admits");
    Value::Text(admitted)
}

/// `Healthy` over a `Workflow` whose receipts are 2 U and 3 U and whose
/// total is `total` U: the Boolean the checked invariant evaluates to.
fn healthy(unit: &CheckedUnit, total: i64) -> bool {
    let package = unit.package();
    let called = package
        .call(
            &QualifiedName::unqualified("twoAndThree").expect("an identifier"),
            Vec::new(),
            &ObjectEnvironment::default(),
            &mut Meter::new(qsl_semantics::check::SCALAR_LIMITS_UNLIMITED),
        )
        .expect("the receipts are formed");
    let quire_eval::FamilyOutcome::Evaluated(Outcome::Completed(receipts)) = called.outcome else {
        panic!("a completed receipts value, not {:?}", called.outcome);
    };
    let Value::Collection(collection) = &receipts else {
        panic!("a collection of receipts, not {receipts:?}");
    };
    let Value::Quantity(first) = &collection.elements()[0] else {
        panic!("a quantity receipt");
    };
    let types = package.graph().scope().types();
    let workflow = types
        .object_types()
        .find(|declared| declared.name() == "M::Workflow")
        .expect("M::Workflow is declared")
        .key();
    let this = ObjectReference::new(
        UniverseId::new(
            NodeKey::new("fixture", vec![PathSegment::Text("u".to_owned())], 0)
                .expect("a node identity"),
        ),
        workflow,
        ObjectId::new("w1".to_owned()).expect("an object id"),
    );
    let whole = |value: i64| Value::Integer(Integer::from(value));
    let amount = Value::Quantity(quire_exact::Quantity::new(
        quire_exact::Rational::from_integer(Integer::from(total)),
        first.unit().clone(),
    ));
    let closure = ObjectClosure::new(
        types,
        [(
            this.clone(),
            vec![
                ("id", FieldValue::Present(text("w1"))),
                ("ready", FieldValue::Present(Value::Boolean(true))),
                ("receipts", FieldValue::Present(receipts.clone())),
                ("total", FieldValue::Present(amount)),
                ("tally", FieldValue::Present(whole(0))),
                ("wide", FieldValue::Present(whole(0))),
            ],
        )],
        &[],
    )
    .expect("the workflow closes");
    let current = Observation {
        identity: DocumentRef {
            authority: "test".to_owned(),
            identity: "N".to_owned(),
        },
        environment: ObjectEnvironment::new(closure),
        populations: BTreeMap::new(),
        out_of_range: Vec::new(),
    };
    let observations = AdmittedObservations {
        usage: Default::default(),
        clause: package
            .graph()
            .state_clause("Healthy")
            .expect("Healthy is declared")
            .identity(),
        current: Some(current),
        pre: None,
        post: None,
        self_object: this,
        invocation: None,
        self_kind: SelfKind::Object,
        receiver_pre: None,
        receiver_post: None,
        parameters: Vec::new(),
        result: None,
        created: Vec::new(),
        deleted: Vec::new(),
    };
    let evaluation = package
        .evaluate_clause(
            &QualifiedName::unqualified("Healthy").expect("an identifier"),
            &observations,
            &mut Meter::new(qsl_semantics::check::SCALAR_LIMITS_UNLIMITED),
        )
        .expect("the clause evaluates");
    match evaluation.evaluation.outcome {
        quire_eval::FamilyOutcome::Evaluated(Outcome::Completed(Value::Boolean(truth))) => truth,
        other => panic!("a completed Boolean, not {other:?}"),
    }
}

/// FR-517-AC-20 (TC-1312 row 20): over receipts of 2 U and 3 U the sum in
/// `Healthy` is exactly 5 U. The invariant holds when the total is 5 U and
/// fails when it is 4 U, so the sum is neither smaller than 5 U nor larger
/// than 5 U.
#[trace("FR-517-AC-20", "TC-1312")]
#[test]
fn the_receipt_sum_of_healthy_over_r_is_five_u() {
    let unit = checked();
    assert!(healthy(&unit, 5), "2 U and 3 U total 5 U, within 5 U");
    assert!(!healthy(&unit, 4), "2 U and 3 U total 5 U, not within 4 U");
}

/// FR-046-AC-3 (TC-127), the nominal-unit leg on the checked route: over R,
/// the items `[2 U, 2 U, 3 U]` keep the duplicate and the order through
/// `filter` and `map`, and give size 3, the count of 2 U as 2 and the sum
/// 7 U, with the controls a deduplicating query would satisfy false.
#[trace("FR-046-AC-3", "TC-127")]
#[test]
fn unit_queries_over_duplicate_items_evaluate_on_the_checked_route() {
    let unit = checked();
    for (index, (label, _, expected)) in
        super::bare_recovery_binder::duplicate_query_cases("amounts()", " U", "Count")
            .into_iter()
            .enumerate()
    {
        let called = unit
            .package()
            .call(
                &QualifiedName::unqualified(format!("query{index}")).expect("an identifier"),
                Vec::new(),
                &ObjectEnvironment::default(),
                &mut Meter::new(qsl_semantics::check::SCALAR_LIMITS_UNLIMITED),
            )
            .unwrap_or_else(|error| panic!("{label}: the call runs: {error:?}"));
        let quire_eval::FamilyOutcome::Evaluated(Outcome::Completed(Value::Boolean(held))) =
            called.outcome
        else {
            panic!("{label}: a completed Boolean, not {:?}", called.outcome);
        };
        assert_eq!(held, expected, "{label}");
    }
}
