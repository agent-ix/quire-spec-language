// SPDX-License-Identifier: AGPL-3.0-or-later
//! TC-1070's E and L protocols, evaluated through their emitted identities.

use super::*;
use qsl_semantics::model::object_environment::ObjectEnvironment;
use qsl_semantics::model::observation::{DocumentRef, Observation};
use quire_exact::{FieldValue, LimitKind, NodeKey, ObjectId, ObjectReference};
use quire_semantic_value::object_closure::ObjectClosure;
use serde_json::{json, Value as Json};

macro_rules! assert_value_eq {
    ($actual:expr, $expected:expr $(, $message:literal)?) => {{
        let actual = &$actual;
        let expected = &$expected;
        assert_eq!(quire_exact::compare_keys(actual, expected), Some(std::cmp::Ordering::Equal), "{}; actual {actual:?}; expected {expected:?}", concat!($($message)?));
    }};
}

#[path = "../../../../qsl-semantics/tests/support/tc235_k2.rs"]
#[allow(
    dead_code,
    clippy::duplicate_mod,
    reason = "shared semantic-IR fixture constructors"
)]
mod model;

const SOURCE: &str = r#"
language "ix:native" edition "1-draft";
profile P = "quire.protocol.finite-global/v1";
profile T = "quire.temporal.infinite-trace/v1";
model M = "test/orders";
protocol Refunds using P over (view: M::Node)
    on each (opened: M::Tick) when (opened.ready) {
  capture order: M::Total = view.total;
  role Service on M::Node;
  compensate Refund for Main::Applied as (forward: M::Receipt)
      by Service on M::Node::step using T clock "refund-clock" {
    capture target: M::Total = forward.total;
    activate first (trigger: M::Flag)
        when { not trigger.ready and target >= 0 } {
      capture activated: Boolean = trigger.ready;
    }
    within [0,30]; attempts 3 of M::Attempt;
    retry (earlier: M::Attempt, later: M::Attempt) {
      earlier.tally < 3 and later.tally = earlier.tally + 1
      and target >= 0 and not activated
    };
    commit Main::Committed;
    recover (recovered: M::Receipt) { recovered.total = target and not activated };
  }
  compensate Partial for Main::Applied as (partialForward: M::Receipt)
      by Service on M::Node::step using T clock "partial-clock" {
    capture partialTarget: M::Total = partialForward.total;
    activate first (partialTrigger: M::Flag)
        when { not partialTrigger.ready and partialTarget >= 0 } {
      capture partialActivated: Boolean = partialTrigger.ready;
    }
    within [0,30]; attempts 3 of M::Attempt;
    retry (partialEarlier: M::Attempt, partialLater: M::Attempt) {
      partialEarlier.tally < 3 and partialLater.tally = partialEarlier.tally + 1
      and partialTarget >= 0 and not partialActivated
    };
    commit never;
    recover (partialRecovered: M::Receipt) {
      partialRecovered.total = partialTarget and not partialActivated
    };
  }
  run sequence Main {
    attempt Tried by Service on M::Node::step contracts []
        as (attempted: M::Flag) { attempted.ready };
    effect Applied of Main::Tried as (applied: M::Receipt) { order >= 0 };
    commit Committed by Service as (committed: M::Flag) { committed.ready };
  }
  finish Closed as (closed: M::Receipt) { true };
}
protocol Pipeline using P over (view: M::Node) on origin {
  role Seller on M::Node;
  role Buyer on M::Node;
  relationship Owns = M::owns;
  channel Pay from Seller to Buyer carries M::Flag
      ordering unordered delivery at-least-once capacity 2 overflow block;
  run sequence Main {
    send Sent via Pay as (sent: M::Flag) { true };
    receive Got via Pay of Main::Sent as (got: M::Flag)
        related by Owns (view, view) { true };
    choice Decide by Buyer visible (got.ready) {
      case yes when { got.ready } check A using P { true };
      case no when { not got.ready } check B using P { true };
    }
  }
  finish Closed as (closed: M::Flag) { true };
}
"#;

fn document() -> Json {
    use qsl_semantics::model::intake::meaning;
    let bounded = |name: &str, upper: &str| {
        let mut ty = model::declaration(name, "bounded", json!({"scalar": "integer"}));
        ty["constraints"] = Json::Array([("min", "0"), ("max", upper)].into_iter().map(|(keyword, value)| json!({
            "identity": model::id(&format!("{name}/constraints/{keyword}")),
            "keyword": keyword, "operands": {"value": value},
            "appliesTo": "ix://quire/native/Integer", "diagnosticCode": format!("bound.{keyword}"),
            "origin": model::origin(name)
        })).collect());
        ty
    };
    let record = |name: &str, fields: &[(&str, &str)]| {
        model::declaration(
            name,
            "record",
            json!({
                "fields": fields.iter().map(|(field, ty)| model::field(name, field, ty)).collect::<Vec<_>>()
            }),
        )
    };
    let mut node = model::declaration(
        "Node",
        "thing",
        json!({
            "identityFields": [model::id("Node/total")],
            "fields": [model::field("Node", "total", &model::id("Total"))],
            "operations": [model::operation("Node", "step")],
            "relationships": [{
                "identity": model::id("relationship/owns"), "category": "structural", "composite": false,
            "direction": "source-to-target", "origin": {"source": {"sourceIdentity": model::id("spec"), "path": "model.qspec", "startLine": 1, "startColumn": 1}},
                "sourceEnd": {"type": model::id("Node"), "role": "owner", "multiplicity": {"lower": 0, "upper": 1, "ordered": false, "unique": true}},
                "targetEnd": {"type": model::id("Node"), "role": "owned", "multiplicity": {"lower": 0, "upper": 1, "ordered": false, "unique": true}}
            }]
        }),
    );
    node["operations"][0]["returns"] = json!({"typeRef": model::id("Flag"), "multiplicity": {"lower": 1, "upper": 1, "ordered": false, "unique": true}, "nullable": false});
    let mut bounded_kind = model::construct("bounded", meaning::VALUE_TYPE);
    bounded_kind["construct"]["shape"] = json!("record");
    bounded_kind["construct"]["members"] = json!({});
    let mut doc = model::domain_document(
        vec![
            bounded("Total", "1000"),
            bounded("Tally", "3"),
            bounded("Count", "9"),
            node,
            record("Receipt", &[("total", &model::id("Total"))]),
            record("Attempt", &[("tally", &model::id("Tally"))]),
            record("Flag", &[("ready", "ix://quire/native/Boolean")]),
            record(
                "Tick",
                &[
                    ("ready", "ix://quire/native/Boolean"),
                    ("count", &model::id("Count")),
                ],
            ),
        ],
        vec![
            bounded_kind,
            model::construct("thing", meaning::OBJECT_TYPE),
            model::construct("record", meaning::RECORD_VALUE_TYPE),
            model::construct("population", meaning::POPULATION),
        ],
    );
    doc["populations"] = json!([{"identity": model::id("nodes"), "displayName": "nodes", "kind": {"module": model::BUSINESS, "name": "population"}, "members": [model::id("Node")], "extent": "closed", "origin": model::origin("nodes")}]);
    doc
}

fn fixture_with_view() -> (
    CheckedPackage,
    qsl_semantics::model::normalize::EffectiveView,
) {
    fixture_from_source(SOURCE)
}

fn fixture_from_source(
    source: &str,
) -> (
    CheckedPackage,
    qsl_semantics::model::normalize::EffectiveView,
) {
    let bytes = serde_json::to_vec(&document()).unwrap();
    let parsed = qsl_cst::parse(
        qsl_foundation::SourceIdentity::new("protocol", "acceptance", "fixture", "1"),
        "acceptance.native",
        source.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .unwrap();
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let unit = qsl_forms::build_unit(&parsed).unwrap();
    let models = qsl_semantics::model::intake::admit_unit(
        &unit.selections().models,
        &qsl_semantics::model::intake::package_input([bytes.as_slice()]).unwrap(),
        qsl_semantics::model::accounting::ModelNormalizationLimits::default(),
    )
    .unwrap();
    let view = models[0].view.clone();
    (
        CheckedPackage::link(
            PackageDeclarations::assemble(
                parsed.source().reference().clone(),
                "acceptance".to_owned(),
                unit,
                models,
                Vec::new(),
            )
            .unwrap()
            .check(CheckingLimits::default())
            .expect("TC-1070's legitimate protocol package checks"),
        ),
        view,
    )
}

fn fixture() -> CheckedPackage {
    fixture_with_view().0
}

fn int(value: i64) -> Value {
    Value::Integer(value.into())
}

fn record(package: &CheckedPackage, name: &str, fields: Vec<(&str, Value)>) -> Value {
    let types = package.graph().scope().types();
    let key = types
        .composites()
        .find(|ty| ty.name() == format!("M::{name}"))
        .expect("fixture record type")
        .key();
    types
        .record(
            key,
            fields
                .into_iter()
                .map(|(name, value)| (name, FieldValue::Present(value)))
                .collect(),
        )
        .unwrap()
}

fn receipt(package: &CheckedPackage, total: i64) -> Value {
    record(package, "Receipt", vec![("total", int(total))])
}
fn attempt(package: &CheckedPackage, tally: i64) -> Value {
    record(package, "Attempt", vec![("tally", int(tally))])
}
fn flag(package: &CheckedPackage, ready: bool) -> Value {
    record(package, "Flag", vec![("ready", Value::Boolean(ready))])
}

fn binder(package: &CheckedPackage, protocol: &str, name: &str) -> NodeKey {
    package
        .graph()
        .protocol_node_keys(protocol)
        .unwrap()
        .binders
        .iter()
        .find(|binder| binder.name == name)
        .unwrap_or_else(|| panic!("{protocol} binds {name}"))
        .node
        .clone()
}

fn slot(
    package: &CheckedPackage,
    protocol: &str,
    owner: &str,
    member: &str,
    index: Option<usize>,
) -> ProtocolSlot {
    let keys = package.graph().protocol_node_keys(protocol).unwrap();
    let checked = package.graph().protocol(protocol).unwrap();
    let node = if owner == protocol {
        keys.clause.clone().unwrap()
    } else if let Some(position) = checked
        .obligations
        .compensations
        .iter()
        .position(|template| template.name == owner)
    {
        keys.compensations[position].clone()
    } else if let Some((id, name)) = &checked.finish {
        if name == owner {
            keys.nodes[id].clone()
        } else {
            let control = checked
                .controls
                .controls
                .iter()
                .find(|control| control.name == owner)
                .expect("named control");
            keys.nodes[&control.node].clone()
        }
    } else {
        let control = checked
            .controls
            .controls
            .iter()
            .find(|control| control.name == owner)
            .expect("named control");
        keys.nodes[&control.node].clone()
    };
    ProtocolSlot {
        node,
        member: member.to_owned(),
        index,
    }
}

fn supply<'a>(
    package: &CheckedPackage,
    protocol: &str,
    values: Vec<(&str, Value)>,
    observation: Option<&'a Observation>,
) -> ProtocolSupply<'a> {
    ProtocolSupply {
        binders: values
            .into_iter()
            .map(|(name, value)| (binder(package, protocol, name), SuppliedValue::Value(value)))
            .collect(),
        observation,
    }
}

fn completed(result: ProtocolSlotEvaluation) -> Value {
    match result {
        ProtocolSlotEvaluation::Evaluated(quire_eval::Evaluation {
            outcome: FamilyOutcome::Evaluated(quire_exact::Outcome::Completed(value)),
            ..
        }) => value,
        other => panic!("expected completed slot value, got {other:?}"),
    }
}

fn observation(
    package: &CheckedPackage,
    view: &qsl_semantics::model::normalize::EffectiveView,
) -> (Observation, ObjectReference) {
    let key = quire_checked::key::DeclarationKey {
        package: model::PACKAGE.to_owned(),
        node: model::id("Node"),
    };
    let reference = ObjectReference::new(
        view.object_universe_of(&key).unwrap().identity().unwrap(),
        view.type_identities()[&key].clone(),
        ObjectId::new("n5").unwrap(),
    );
    let objects = ObjectClosure::new(
        package.graph().scope().types(),
        [(
            reference.clone(),
            vec![("total", FieldValue::Present(int(5)))],
        )],
        &[],
    )
    .unwrap();
    (
        Observation {
            identity: DocumentRef {
                authority: "test".to_owned(),
                identity: "current".to_owned(),
                revision_namespace: "fixture".to_owned(),
                revision: "1".to_owned(),
            },
            environment: ObjectEnvironment::new(objects).with_references(vec![reference.clone()]),
            populations: [(model::id("nodes"), true)].into_iter().collect(),
            out_of_range: Vec::new(),
        },
        reference,
    )
}

fn assert_reader_admits(package: &CheckedPackage) {
    let emission = qsl_package::emit_checked(package).expect("the complete E/L package emits");
    let bytes = serde_json::to_vec(&document()).unwrap();
    let input = qsl_semantics::model::intake::package_input([bytes.as_slice()]).unwrap();
    let mut admitted = qsl_package::AdmittedPackages::default();
    qsl_package::read_import_view(
        package,
        &emission,
        qsl_semantics::library::LibraryName::new(package.emitted_package_name().unwrap()).unwrap(),
        &input,
        &mut admitted,
        qsl_package::V2ReadLimits::default(),
    )
    .expect(
        "the actual contract reader admits the emitted related records before any slot evaluates",
    );
}

#[test]
#[trace("TC-1070", "FR-390-AC-1")]
fn activation_capture_and_both_related_ends_use_the_supplied_observation() {
    let (package, view) = fixture_with_view();
    assert_reader_admits(&package);
    let (observation, reference) = observation(&package, &view);
    let mut meter = Meter::new(UNLIMITED);
    let tick = record(
        &package,
        "Tick",
        vec![("ready", Value::Boolean(true)), ("count", int(2))],
    );
    assert_value_eq!(
        completed(
            package
                .evaluate_protocol_slot(
                    &slot(&package, "Refunds", "Refunds", "activation.guard", None),
                    &supply(&package, "Refunds", vec![("opened", tick)], None),
                    &mut meter
                )
                .unwrap()
        ),
        Value::Boolean(true)
    );
    assert_value_eq!(
        completed(
            package
                .evaluate_protocol_slot(
                    &slot(&package, "Refunds", "Refunds", "captures", Some(0)),
                    &supply(
                        &package,
                        "Refunds",
                        vec![("view", Value::Reference(reference.clone()))],
                        Some(&observation)
                    ),
                    &mut meter
                )
                .unwrap()
        ),
        int(5)
    );
    for member in ["related.from", "related.to"] {
        let value = completed(
            package
                .evaluate_protocol_slot(
                    &slot(&package, "Pipeline", "Got", member, Some(0)),
                    &supply(
                        &package,
                        "Pipeline",
                        vec![("view", Value::Reference(reference.clone()))],
                        Some(&observation),
                    ),
                    &mut meter,
                )
                .unwrap(),
        );
        assert!(
            matches!(value, Value::Reference(actual) if actual == reference),
            "{member} returns the supplied endpoint"
        );
    }
}

#[test]
#[trace("TC-1070", "FR-390-AC-1", "FR-390-AC-4")]
fn related_indexes_follow_the_emitted_record_order() {
    let source = SOURCE.replace("protocol Pipeline using P over (view: M::Node) on origin {", "protocol Pipeline using P over (view: M::Node) on origin {\n capture peer: M::Node = view;")
        .replace("related by Owns (view, view)", "related by Owns (view, peer) related by Owns (peer, view) related by Owns (peer, view)");
    let (package, view) = fixture_from_source(&source);
    assert_reader_admits(&package);
    let (mut observation, n5) = observation(&package, &view);
    let n9 = ObjectReference::new(
        n5.universe().clone(),
        n5.object_type().clone(),
        ObjectId::new("n9").unwrap(),
    );
    observation.environment = ObjectEnvironment::new(
        ObjectClosure::new(
            package.graph().scope().types(),
            [
                (n5.clone(), vec![("total", FieldValue::Present(int(5)))]),
                (n9.clone(), vec![("total", FieldValue::Present(int(9)))]),
            ],
            &[],
        )
        .unwrap(),
    )
    .with_references(vec![n5.clone(), n9.clone()]);
    for (index, member, name, reference) in [
        (0, "related.from", "peer", &n9),
        (0, "related.to", "view", &n5),
        (1, "related.from", "view", &n5),
        (1, "related.to", "peer", &n9),
    ] {
        let value = completed(
            package
                .evaluate_protocol_slot(
                    &slot(&package, "Pipeline", "Got", member, Some(index)),
                    &supply(
                        &package,
                        "Pipeline",
                        vec![(name, Value::Reference(reference.clone()))],
                        Some(&observation),
                    ),
                    &mut Meter::new(UNLIMITED),
                )
                .unwrap(),
        );
        assert!(
            matches!(value, Value::Reference(actual) if actual == *reference),
            "{member}[{index}] follows the canonical related record"
        );
    }
    assert_refused(
        &package,
        &slot(&package, "Pipeline", "Got", "related.from", Some(2)),
        &supply(&package, "Pipeline", vec![], None),
        CatalogCode::new("missing_declaration", "missing-name"),
        CallAdmissionLocus::Member {
            name: "related.from".to_owned(),
            index: Some(2),
        },
    );
}

const COUNTERS: [LimitKind; 10] = [
    LimitKind::IntegerBits,
    LimitKind::DecimalDigits,
    LimitKind::ScaleExpansion,
    LimitKind::TextInputBytes,
    LimitKind::TextScalars,
    LimitKind::NormalizedScalars,
    LimitKind::UnitEdges,
    LimitKind::ValueOccurrences,
    LimitKind::WorkUnits,
    LimitKind::ResultUnits,
];

fn set_limit(limits: &mut ScalarLimits, kind: LimitKind, bound: u64) {
    match kind {
        LimitKind::IntegerBits => limits.integer_bits = bound,
        LimitKind::DecimalDigits => limits.decimal_digits = bound,
        LimitKind::ScaleExpansion => limits.scale_expansion = bound,
        LimitKind::TextInputBytes => limits.text_input_bytes = bound,
        LimitKind::TextScalars => limits.text_scalars = bound,
        LimitKind::NormalizedScalars => limits.normalized_scalars = bound,
        LimitKind::UnitEdges => limits.unit_edges = bound,
        LimitKind::ValueOccurrences => limits.value_occurrences = bound,
        LimitKind::WorkUnits => limits.work_units = bound,
        LimitKind::ResultUnits => limits.result_units = bound,
    }
}

fn retry_supply(package: &CheckedPackage, first: i64) -> ProtocolSupply<'static> {
    supply(
        package,
        "Refunds",
        vec![
            ("earlier", attempt(package, first)),
            ("later", attempt(package, 1)),
            ("target", int(5)),
            ("activated", Value::Boolean(false)),
        ],
        None,
    )
}

fn assert_incomplete(result: ProtocolSlotEvaluation, counter: LimitKind) {
    match result {
        ProtocolSlotEvaluation::Evaluated(quire_eval::Evaluation {
            outcome: FamilyOutcome::Evaluated(quire_exact::Outcome::Incomplete(record)),
            ..
        }) => assert_eq!(
            record.limit_kind, counter,
            "the denied counter is retained and no completed value is returned"
        ),
        other => panic!("expected {counter:?} exhaustion with no value: {other:?}"),
    }
}

fn measured_boundaries(
    package: &CheckedPackage,
    slot: &ProtocolSlot,
    supply: &ProtocolSupply<'_>,
    expected: Value,
) -> Meter {
    let mut measured = Meter::new(UNLIMITED);
    assert_value_eq!(
        completed(
            package
                .evaluate_protocol_slot(slot, supply, &mut measured)
                .unwrap()
        ),
        expected
    );
    let mut exact_limits = UNLIMITED;
    for counter in COUNTERS {
        set_limit(&mut exact_limits, counter, measured.consumed(counter));
    }
    let mut exact = Meter::new(exact_limits);
    assert_value_eq!(
        completed(
            package
                .evaluate_protocol_slot(slot, supply, &mut exact)
                .unwrap()
        ),
        expected,
        "every counter at its measured usage completes"
    );
    assert_eq!(exact.admitted_charges(), measured.admitted_charges());
    for counter in COUNTERS {
        let used = measured.consumed(counter);
        if used == 0 {
            continue;
        }
        let mut limits = exact_limits;
        set_limit(&mut limits, counter, used - 1);
        let mut short = Meter::new(limits);
        assert_incomplete(
            package
                .evaluate_protocol_slot(slot, supply, &mut short)
                .unwrap(),
            counter,
        );
        assert!(
            short.consumed(counter) < used,
            "the denied charge is not consumed"
        );
    }
    measured
}

#[test]
#[trace("TC-1073", "FR-390-AC-9", "FR-390-AC-11")]
// AC-11's repeated-call and refusal properties only; contract-reader parity
// requires an admitted package with the evaluator's checked-term interface.
fn retry_and_observation_capture_obey_every_measured_counter_boundary() {
    let (package, view) = fixture_with_view();
    let retry = slot(&package, "Refunds", "Refund", "retry.relation", None);
    let values = retry_supply(&package, 0);
    let measured = measured_boundaries(&package, &retry, &values, Value::Boolean(true));
    let expected_counters = vec![
        LimitKind::IntegerBits,
        LimitKind::ValueOccurrences,
        LimitKind::WorkUnits,
        LimitKind::ResultUnits,
    ];
    assert_eq!(COUNTERS.into_iter().filter(|counter| measured.consumed(*counter) > 0).collect::<Vec<_>>(), expected_counters, "T's charged counters are independently required by its arithmetic, comparisons and completed result");
    let mut repeated = Meter::new(UNLIMITED);
    assert_value_eq!(
        completed(
            package
                .evaluate_protocol_slot(&retry, &values, &mut repeated)
                .unwrap()
        ),
        Value::Boolean(true)
    );
    assert_eq!(
        repeated.admitted_charges(),
        measured.admitted_charges(),
        "the same call has the same ordered charges"
    );
    let mut early = Meter::new(UNLIMITED);
    assert_value_eq!(
        completed(
            package
                .evaluate_protocol_slot(&retry, &retry_supply(&package, 3), &mut early)
                .unwrap()
        ),
        Value::Boolean(false)
    );
    assert!(
        early.consumed(LimitKind::WorkUnits) < measured.consumed(LimitKind::WorkUnits),
        "a first-operand refusal skips the remaining retry expression"
    );
    let mut zero = UNLIMITED;
    zero.work_units = 0;
    assert_incomplete(
        package
            .evaluate_protocol_slot(&retry, &values, &mut Meter::new(zero))
            .unwrap(),
        LimitKind::WorkUnits,
    );
    let (observation, reference) = observation(&package, &view);
    let capture = slot(&package, "Refunds", "Refunds", "captures", Some(0));
    let values = supply(
        &package,
        "Refunds",
        vec![("view", Value::Reference(reference))],
        Some(&observation),
    );
    let captured = measured_boundaries(&package, &capture, &values, int(5));
    assert_eq!(
        COUNTERS
            .into_iter()
            .filter(|counter| captured.consumed(*counter) > 0)
            .collect::<Vec<_>>(),
        vec![LimitKind::ValueOccurrences, LimitKind::WorkUnits],
        "C's observation read charges its value occurrence and work"
    );
    assert!(
        captured
            .admitted_charges()
            .iter()
            .any(|charge| charge.point == quire_exact::ChargePoint::ModelNavigate),
        "C actually navigates the observation"
    );
    assert!(
        !captured
            .admitted_charges()
            .iter()
            .any(|charge| charge.point == quire_exact::ChargePoint::ModelDeref),
        "a bare view.total read contains no explicit dereference"
    );
    let mut after_refusal = Meter::new(UNLIMITED);
    let before = after_refusal.clone();
    let mut missing = retry_supply(&package, 0);
    missing
        .binders
        .retain(|(key, _)| *key != binder(&package, "Refunds", "activated"));
    assert!(matches!(
        package.evaluate_protocol_slot(&retry, &missing, &mut after_refusal),
        Err(CallFailure::Admission(_))
    ));
    assert_eq!(after_refusal, before);
    assert_value_eq!(
        completed(
            package
                .evaluate_protocol_slot(&retry, &retry_supply(&package, 0), &mut after_refusal)
                .unwrap()
        ),
        Value::Boolean(true)
    );
    assert_eq!(
        after_refusal.admitted_charges(),
        measured.admitted_charges()
    );
}

#[test]
#[trace("TC-1073", "FR-390-AC-9")]
fn an_explicit_dereference_capture_obeys_the_observation_charge_boundaries() {
    let (package, view) =
        fixture_from_source(&SOURCE.replace("= view.total;", "= deref(view).total;"));
    let (observation, reference) = observation(&package, &view);
    let capture = slot(&package, "Refunds", "Refunds", "captures", Some(0));
    let values = supply(
        &package,
        "Refunds",
        vec![("view", Value::Reference(reference))],
        Some(&observation),
    );
    let measured = measured_boundaries(&package, &capture, &values, int(5));
    let model_points: Vec<_> = measured
        .admitted_charges()
        .iter()
        .filter_map(|charge| {
            matches!(
                charge.point,
                quire_exact::ChargePoint::ModelDeref | quire_exact::ChargePoint::ModelNavigate
            )
            .then_some(charge.point)
        })
        .collect();
    assert_eq!(
        model_points,
        [
            quire_exact::ChargePoint::ModelDeref,
            quire_exact::ChargePoint::ModelNavigate
        ]
    );
    for charge in measured.admitted_charges().iter().filter(|charge| {
        matches!(
            charge.point,
            quire_exact::ChargePoint::ModelDeref | quire_exact::ChargePoint::ModelNavigate
        )
    }) {
        assert_eq!(charge.value_occurrences, Some(1), "the normative model read vector belongs to each charge, including the explicit dereference");
    }
}

#[test]
#[trace("TC-1073", "FR-390-AC-10")]
fn registration_activation_trigger_retry_and_recovery_share_the_sum_in_both_orders() {
    let package = fixture();
    let calls = [
        (
            slot(
                &package,
                "Refunds",
                "Refund",
                "registration_captures",
                Some(0),
            ),
            supply(
                &package,
                "Refunds",
                vec![("forward", receipt(&package, 5))],
                None,
            ),
            int(5),
        ),
        (
            slot(
                &package,
                "Refunds",
                "Refund",
                "activation_captures",
                Some(0),
            ),
            supply(
                &package,
                "Refunds",
                vec![("trigger", flag(&package, true))],
                None,
            ),
            Value::Boolean(true),
        ),
        (
            slot(&package, "Refunds", "Refund", "trigger.guard", None),
            supply(
                &package,
                "Refunds",
                vec![("trigger", flag(&package, false)), ("target", int(5))],
                None,
            ),
            Value::Boolean(true),
        ),
        (
            slot(&package, "Refunds", "Refund", "retry.relation", None),
            retry_supply(&package, 0),
            Value::Boolean(true),
        ),
        (
            slot(&package, "Refunds", "Refund", "recovery.predicate", None),
            supply(
                &package,
                "Refunds",
                vec![
                    ("recovered", receipt(&package, 5)),
                    ("target", int(5)),
                    ("activated", Value::Boolean(false)),
                ],
                None,
            ),
            Value::Boolean(true),
        ),
    ];
    let measured: Vec<_> = calls
        .iter()
        .map(|(slot, supply, expected)| {
            let mut meter = Meter::new(UNLIMITED);
            assert_value_eq!(
                completed(
                    package
                        .evaluate_protocol_slot(slot, supply, &mut meter)
                        .unwrap()
                ),
                *expected
            );
            meter
        })
        .collect();
    let sum: u64 = measured
        .iter()
        .map(|meter| meter.consumed(LimitKind::WorkUnits))
        .sum();
    let mut limits = UNLIMITED;
    limits.work_units = sum;
    let mut forward = Meter::new(limits);
    for (slot, supply, expected) in &calls {
        assert_value_eq!(
            completed(
                package
                    .evaluate_protocol_slot(slot, supply, &mut forward)
                    .unwrap()
            ),
            *expected
        );
    }
    assert_eq!(forward.consumed(LimitKind::WorkUnits), sum);
    let expected_log: Vec<_> = measured
        .iter()
        .flat_map(|meter| meter.admitted_charges().iter().cloned())
        .collect();
    assert_eq!(
        forward.admitted_charges(),
        expected_log,
        "five independent measured logs concatenate without resetting or reevaluating captures"
    );
    let mut reverse = Meter::new(limits);
    for (slot, supply, expected) in calls.iter().rev() {
        assert_value_eq!(
            completed(
                package
                    .evaluate_protocol_slot(slot, supply, &mut reverse)
                    .unwrap()
            ),
            *expected
        );
    }
    for counter in COUNTERS {
        assert_eq!(
            reverse.consumed(counter),
            forward.consumed(counter),
            "order-independent {counter:?} usage"
        );
    }
    let reverse_log: Vec<_> = measured
        .iter()
        .rev()
        .flat_map(|meter| meter.admitted_charges().iter().cloned())
        .collect();
    assert_eq!(reverse.admitted_charges(), reverse_log);
    limits.work_units = sum - 1;
    let mut short = Meter::new(limits);
    for (slot, supply, expected) in &calls[..4] {
        assert_value_eq!(
            completed(
                package
                    .evaluate_protocol_slot(slot, supply, &mut short)
                    .unwrap()
            ),
            *expected
        );
    }
    assert_incomplete(
        package
            .evaluate_protocol_slot(&calls[4].0, &calls[4].1, &mut short)
            .unwrap(),
        LimitKind::WorkUnits,
    );
    assert_eq!(short.consumed(LimitKind::WorkUnits), sum - 1);
    let prefix: Vec<_> = measured[..4]
        .iter()
        .flat_map(|meter| meter.admitted_charges().iter().cloned())
        .collect();
    assert!(short.admitted_charges().starts_with(&prefix));
    assert!(
        measured[4]
            .admitted_charges()
            .starts_with(&short.admitted_charges()[prefix.len()..]),
        "V retains only its charges before denial"
    );
}

#[test]
#[trace("TC-1070", "FR-390-AC-1", "FR-390-AC-2")]
fn both_templates_and_the_remaining_run_slots_evaluate_their_own_expressions() {
    let package = fixture();
    let mut meter = Meter::new(UNLIMITED);
    for (owner, prefix) in [("Refund", ""), ("Partial", "partial")] {
        let name = |stem: &str| {
            if prefix.is_empty() {
                stem.to_owned()
            } else {
                format!("partial{}{}", stem[..1].to_uppercase(), &stem[1..])
            }
        };
        let mut run = |member, index, values: Vec<(&str, Value)>| {
            completed(
                package
                    .evaluate_protocol_slot(
                        &slot(&package, "Refunds", owner, member, index),
                        &supply(&package, "Refunds", values, None),
                        &mut meter,
                    )
                    .unwrap(),
            )
        };
        let forward = name("forward");
        let target = name("target");
        let trigger = name("trigger");
        let activated = name("activated");
        let earlier = name("earlier");
        let later = name("later");
        let recovered = name("recovered");
        let saved_five = run(
            "registration_captures",
            Some(0),
            vec![(&forward, receipt(&package, 5))],
        );
        assert_value_eq!(saved_five, int(5));
        for ready in [false, true] {
            assert_value_eq!(
                run(
                    "trigger.guard",
                    None,
                    vec![(&trigger, flag(&package, ready)), (&target, int(5))]
                ),
                Value::Boolean(!ready)
            );
            assert_value_eq!(
                run(
                    "activation_captures",
                    Some(0),
                    vec![(&trigger, flag(&package, ready))]
                ),
                Value::Boolean(ready)
            );
        }
        for (first, second, active, expected) in [
            (0, 1, false, true),
            (0, 1, true, false),
            (3, 1, false, false),
            (0, 0, false, false),
        ] {
            assert_value_eq!(
                run(
                    "retry.relation",
                    None,
                    vec![
                        (&earlier, attempt(&package, first)),
                        (&later, attempt(&package, second)),
                        (&target, int(5)),
                        (&activated, Value::Boolean(active))
                    ]
                ),
                Value::Boolean(expected)
            );
        }
        for (total, saved, active, expected) in [
            (5, 5, false, true),
            (9, 5, false, false),
            (5, 5, true, false),
            (9, 9, false, true),
        ] {
            assert_value_eq!(
                run(
                    "recovery.predicate",
                    None,
                    vec![
                        (&recovered, receipt(&package, total)),
                        (&target, int(saved)),
                        (&activated, Value::Boolean(active))
                    ]
                ),
                Value::Boolean(expected)
            );
        }
        let current_nine = run(
            "registration_captures",
            Some(0),
            vec![(&forward, receipt(&package, 9))],
        );
        assert_value_eq!(current_nine, int(9));
        assert_value_eq!(
            run(
                "recovery.predicate",
                None,
                vec![
                    (&recovered, receipt(&package, 9)),
                    (&target, saved_five),
                    (&activated, Value::Boolean(false))
                ]
            ),
            Value::Boolean(false),
            "a later registration result does not replace the supplied frozen five"
        );
        assert_value_eq!(
            run(
                "recovery.predicate",
                None,
                vec![
                    (&recovered, receipt(&package, 9)),
                    (&target, current_nine),
                    (&activated, Value::Boolean(false))
                ]
            ),
            Value::Boolean(true),
            "supplying the new captured nine changes the result without recomputation"
        );
    }
    for (owner, values) in [
        ("Applied", vec![("order", int(5))]),
        ("Committed", vec![("committed", flag(&package, true))]),
        ("Closed", vec![]),
    ] {
        assert_value_eq!(
            completed(
                package
                    .evaluate_protocol_slot(
                        &slot(&package, "Refunds", owner, "constraint", None),
                        &supply(&package, "Refunds", values, None),
                        &mut meter
                    )
                    .unwrap()
            ),
            Value::Boolean(true)
        );
    }
    for ready in [true, false] {
        for (member, index, expected) in [
            ("visible", 0, ready),
            // Case indexes follow the emitted UTF-8 label order: no, yes.
            ("cases.guard", 0, !ready),
            ("cases.guard", 1, ready),
        ] {
            assert_value_eq!(
                completed(
                    package
                        .evaluate_protocol_slot(
                            &slot(&package, "Pipeline", "Decide", member, Some(index)),
                            &supply(
                                &package,
                                "Pipeline",
                                vec![("got", flag(&package, ready))],
                                None
                            ),
                            &mut meter
                        )
                        .unwrap()
                ),
                Value::Boolean(expected)
            );
        }
    }
}

fn assert_refused(
    package: &CheckedPackage,
    slot: &ProtocolSlot,
    supply: &ProtocolSupply<'_>,
    code: CatalogCode,
    locus: CallAdmissionLocus,
) {
    let mut meter = Meter::new(UNLIMITED);
    let before = meter.clone();
    let failure = package.evaluate_protocol_slot(slot, supply, &mut meter);
    assert_eq!(meter, before, "admission refusal must charge nothing");
    let Err(CallFailure::Admission(payload)) = failure else {
        panic!("expected typed admission refusal: {failure:?}");
    };
    assert!(
        matches!(payload.as_ref(), CallAdmissionFailure::Refused(record) if record.code == code && record.locus == locus),
        "expected {code:?} at {locus:?}, got {payload:?}"
    );
}

#[test]
#[trace("TC-1071", "FR-390-AC-4")]
fn absent_members_and_each_index_defect_refuse_without_evaluating_a_neighbor() {
    let package = fixture_from_source(&SOURCE.replace(
        "  run sequence Main {",
        "  terminal any;\n  run sequence Main {",
    ))
    .0;
    let empty = supply(&package, "Refunds", vec![], None);
    for (owner, member, index) in [
        ("Refunds", "retry.relation", None),
        ("Refund", "registration_captures", Some(1)),
        ("Refund", "registration_captures", None),
        ("Refund", "retry.relation", Some(0)),
        ("Closed", "condition", None),
    ] {
        assert_refused(
            &package,
            &slot(&package, "Refunds", owner, member, index),
            &empty,
            CatalogCode::new("missing_declaration", "missing-name"),
            CallAdmissionLocus::Member {
                name: member.to_owned(),
                index,
            },
        );
    }
    let terminal = ProtocolSlot {
        node: package
            .graph()
            .protocol_node_keys("Refunds")
            .unwrap()
            .terminal
            .clone()
            .expect("terminal any emits its obligation"),
        member: "condition".to_owned(),
        index: None,
    };
    assert_refused(
        &package,
        &terminal,
        &empty,
        CatalogCode::new("missing_declaration", "missing-name"),
        CallAdmissionLocus::Member {
            name: "condition".to_owned(),
            index: None,
        },
    );
}

#[test]
#[trace("TC-1070", "TC-1071", "FR-390-AC-3", "FR-390-AC-5", "FR-390-AC-6")]
// AC-6 covers record shape and scalar domain here; observation refusals have
// their own cases and are not implied by this test.
fn crossed_template_sources_and_admission_precedence_name_the_exact_binder() {
    let package = fixture();
    let retry = slot(&package, "Refunds", "Refund", "retry.relation", None);
    let invalid = |name: &str| CallAdmissionLocus::Binder(binder(&package, "Refunds", name));
    let mut duplicate = retry_supply(&package, 0);
    duplicate.binders.push((
        binder(&package, "Refunds", "earlier"),
        SuppliedValue::Value(attempt(&package, 0)),
    ));
    duplicate.binders.push((
        binder(&package, "Refunds", "partialTarget"),
        SuppliedValue::Value(int(5)),
    ));
    duplicate
        .binders
        .retain(|(key, _)| *key != binder(&package, "Refunds", "activated"));
    assert_refused(
        &package,
        &retry,
        &duplicate,
        CatalogCode::new("invalid_runtime_input", "duplicate-member"),
        invalid("earlier"),
    );
    for unknown in ["forward", "partialTarget"] {
        let mut values = retry_supply(&package, 0);
        values.binders.push((
            binder(&package, "Refunds", unknown),
            SuppliedValue::Value(if unknown == "forward" {
                receipt(&package, 5)
            } else {
                int(5)
            }),
        ));
        assert_refused(
            &package,
            &retry,
            &values,
            CatalogCode::new("invalid_runtime_input", "unknown-member"),
            invalid(unknown),
        );
    }
    for names in [["activated", "order"], ["order", "activated"]] {
        let values = supply(
            &package,
            "Refunds",
            names
                .into_iter()
                .map(|name| {
                    (
                        name,
                        if name == "activated" {
                            Value::Boolean(false)
                        } else {
                            int(5)
                        },
                    )
                })
                .collect(),
            None,
        );
        assert_refused(
            &package,
            &slot(
                &package,
                "Refunds",
                "Refund",
                "registration_captures",
                Some(0),
            ),
            &values,
            CatalogCode::new("invalid_runtime_input", "unknown-member"),
            invalid("order"),
        );
    }
    let crossed_trigger = supply(
        &package,
        "Refunds",
        vec![
            ("target", int(5)),
            ("partialTrigger", flag(&package, false)),
        ],
        None,
    );
    assert_refused(
        &package,
        &slot(&package, "Refunds", "Refund", "trigger.guard", None),
        &crossed_trigger,
        CatalogCode::new("invalid_runtime_input", "unknown-member"),
        invalid("partialTrigger"),
    );
    for (omitted, first) in [
        (vec!["activated"], "activated"),
        (vec!["activated", "target"], "target"),
    ] {
        let mut values = retry_supply(&package, 0);
        values.binders.retain(|(key, _)| {
            !omitted
                .iter()
                .any(|name| *key == binder(&package, "Refunds", name))
        });
        assert_refused(
            &package,
            &retry,
            &values,
            CatalogCode::new("invalid_runtime_input", "missing-member"),
            invalid(first),
        );
    }
    for (owner, member, index, first) in [
        ("Refund", "retry.relation", None, "target"),
        ("Refund", "trigger.guard", None, "target"),
        ("Refund", "recovery.predicate", None, "target"),
        ("Partial", "retry.relation", None, "partialTarget"),
        ("Partial", "recovery.predicate", None, "partialTarget"),
        ("Refund", "registration_captures", Some(0), "forward"),
        ("Refund", "activation_captures", Some(0), "trigger"),
    ] {
        assert_refused(
            &package,
            &slot(&package, "Refunds", owner, member, index),
            &supply(&package, "Refunds", vec![], None),
            CatalogCode::new("invalid_runtime_input", "missing-member"),
            invalid(first),
        );
    }
    for (name, value, cause) in [
        ("target", Value::Boolean(true), "wrong-value-kind"),
        ("earlier", flag(&package, true), "wrong-value-kind"),
        ("target", int(1001), "invalid-value"),
    ] {
        let mut values = retry_supply(&package, 0);
        let key = binder(&package, "Refunds", name);
        values
            .binders
            .iter_mut()
            .find(|(candidate, _)| *candidate == key)
            .unwrap()
            .1 = SuppliedValue::Value(value);
        assert_refused(
            &package,
            &retry,
            &values,
            CatalogCode::new("invalid_runtime_input", cause),
            invalid(name),
        );
    }
}
