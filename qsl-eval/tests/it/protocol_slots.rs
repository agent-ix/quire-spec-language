// SPDX-License-Identifier: AGPL-3.0-or-later
//! FR-390: evaluate a checked expression selected from an emitted protocol node.

use ix_trace_rs::trace;
use qsl_eval::value::{
    CheckedPackageEvaluation, ProtocolSlot, ProtocolSlotEvaluation, ProtocolSupply, SuppliedValue,
};
use qsl_package::CheckedPackage;
use qsl_semantics::check::{NodeTag, PackageDeclarations};
use quire_eval::{CallAdmissionFailure, CallAdmissionLocus, CallFailure, FamilyOutcome};
use quire_exact::{Meter, ScalarLimits, Value};
use quire_semantic_value::catalog::CatalogCode;
use quire_semantic_value::checking::CheckingLimits;

const MODEL: &[u8] = include_bytes!("../../../tests/fixtures/spine-model.semantic-ir.json");

#[path = "../../../qsl-semantics/tests/support/child_composite.rs"]
#[allow(
    clippy::duplicate_mod,
    reason = "the shared domain fixture is used by the QSL package tests too"
)]
mod compensation_model;

/// The unit selecting `MODEL` as `M` by its `sha256-jcs` digest, computed
/// from the fixture.
fn unit() -> String {
    let packages = qsl_semantics::model::intake::package_input([MODEL]);
    let digest = qsl_semantics::model::key::hex(packages.keys().next().expect("one model"));
    format!(
        "language \"ix:native\" edition \"1-draft\";\n\
    profile v = \"quire.value.complete/v1\";\n\
    profile p = \"quire.protocol.finite-global/v1\";\n\
    model M = \"acme/orders\" version \"1.0.0\" digest \
    \"sha256-jcs:{digest}\";\n\
    protocol P using v over (input: M::Gadget) on each (opened: Boolean) when ((1 + 2 = 3) and opened) {{\n\
      role R on M::Gadget;\n\
      run sequence Main {{ check Ready using p {{ true }}; }}\n\
      finish End as (outcome: Boolean) {{ true }};\n\
    }}\n"
    )
}

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

fn package() -> CheckedPackage {
    package_from_source(&unit())
}

fn package_from_source(source: &str) -> CheckedPackage {
    let parsed = qsl_cst::parse(
        qsl_foundation::SourceIdentity::new("a", "u", "git", "1"),
        "unit.native",
        source.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the unit");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let unit = qsl_forms::build_unit(&parsed).expect("S2 builds the unit");
    let models = qsl_semantics::model::intake::admit_unit(
        &unit.selections().models,
        &qsl_semantics::model::intake::package_input([MODEL]),
        qsl_semantics::model::accounting::ModelNormalizationLimits::default(),
    )
    .expect("I1 admits the model");
    CheckedPackage::link(
        PackageDeclarations::assemble(
            parsed.source().reference().clone(),
            unit,
            models,
            Vec::new(),
        )
        .expect("the unit assembles")
        .check(CheckingLimits::default())
        .expect("the package checks"),
    )
}

fn compensation_package() -> CheckedPackage {
    let model = serde_json::to_vec(&compensation_model::document())
        .expect("the compensation model fixture serializes");
    let package_input = qsl_semantics::model::intake::package_input([model.as_slice()]);
    let digest = package_input
        .keys()
        .next()
        .expect("the model fixture has one package");
    let digest = qsl_semantics::model::key::hex(digest);
    let source = format!(
        "language \"ix:native\" edition \"1-draft\";\n\
         profile v = \"quire.value.complete/v1\";\n\
         profile p = \"quire.protocol.finite-global/v1\";\n\
         profile t = \"quire.temporal.infinite-trace/v1\";\n\
         model M = {:?} version \"1\" digest \"sha256-jcs:{}\";\n\
         protocol P using p over (input: M::Ledger) on origin {{\n\
           role R on M::Ledger;\n\
           compensate Refund for Main::Applied as (fwd: M::Money)\n\
             by R on M::Ledger::pay using t clock \"refund-clock\" {{\n\
             capture saved: M::Money = fwd;\n\
             activate first (trigger: M::Money) when {{ trigger.amount_minor < saved.amount_minor }} {{\n\
               capture activated: Boolean = trigger.amount_minor > 0;\n\
             }}\n\
             within [0,10]; attempts 1 of M::Money;\n\
             retry (earlier: M::Money, later: M::Money) {{\n\
               earlier.amount_minor < later.amount_minor and saved.amount_minor >= 0 and not activated\n\
             }};\n\
             commit never;\n\
             recover (recovered: M::Money) {{ recovered.amount_minor = saved.amount_minor and not activated }};\n\
           }}\n\
           run sequence Main {{\n\
             attempt Update by R on M::Ledger::pay contracts [] as (tried: M::Money) {{ true }};\n\
             effect Applied of Update as (applied: M::Money) {{ true }};\n\
           }}\n\
           finish End as (outcome: Boolean) {{ true }};\n\
         }}\n",
        compensation_model::PACKAGE,
        digest
    );
    let parsed = qsl_cst::parse(
        qsl_foundation::SourceIdentity::new("protocol-slots", "unit.native", "git", "1"),
        "unit.native",
        source.as_bytes(),
        qsl_cst::Limits::default(),
    )
    .expect("S1 reads the compensation fixture");
    assert!(parsed.is_admissible(), "{:?}", parsed.diagnostics());
    let unit = qsl_forms::build_unit(&parsed).expect("S2 builds the compensation unit");
    let models = qsl_semantics::model::intake::admit_unit(
        &unit.selections().models,
        &package_input,
        qsl_semantics::model::accounting::ModelNormalizationLimits::default(),
    )
    .expect("I1 admits the compensation model");
    CheckedPackage::link(
        PackageDeclarations::assemble(
            parsed.source().reference().clone(),
            unit,
            models,
            Vec::new(),
        )
        .expect("the compensation unit assembles")
        .check(CheckingLimits::default())
        .expect("the compensation unit checks"),
    )
}

fn money(package: &CheckedPackage, amount: i64) -> Value {
    let types = package.graph().scope().types();
    let declaration = types
        .composites()
        .find(|composite| composite.name() == "M::Money")
        .expect("the selected model declares Money")
        .key();
    types
        .record(
            declaration,
            vec![
                (
                    "amount_minor",
                    quire_exact::FieldValue::Present(Value::Integer(amount.into())),
                ),
                (
                    "currency",
                    quire_exact::FieldValue::Present(Value::Integer(978_i64.into())),
                ),
            ],
        )
        .expect("the money value has every required field")
}

#[test]
#[trace("TC-1071", "FR-390-AC-3", "FR-390-AC-4", "FR-390-AC-5", "FR-049-AC-6")]
#[trace("TC-1072", "FR-390-AC-7")]
fn evaluates_a_protocol_slot_and_refuses_an_unread_binder_before_charging() {
    let package = package();
    let keys = package
        .graph()
        .protocol_node_keys("P")
        .expect("the protocol's emitted identity map");
    let checked_protocol = package.graph().protocol("P").expect("checked protocol");
    let check = checked_protocol
        .controls
        .controls
        .iter()
        .find(|control| control.kind.label() == "check")
        .expect("checked check control");
    let node = *keys.nodes.get(&check.node).expect("emitted check node");
    let slot = ProtocolSlot {
        node,
        member: "condition".to_owned(),
        index: None,
    };

    let mut meter = Meter::new(UNLIMITED);
    let evaluated = package
        .evaluate_protocol_slot(
            &slot,
            &ProtocolSupply {
                binders: Vec::new(),
                observation: None,
            },
            &mut meter,
        )
        .expect("the checked condition evaluates");
    let ProtocolSlotEvaluation::Evaluated(evaluation) = evaluated else {
        panic!("literal condition did not read an unavailable binder: {evaluated:?}");
    };
    assert!(matches!(
        evaluation.outcome,
        FamilyOutcome::Evaluated(quire_exact::Outcome::Completed(Value::Boolean(true)))
    ));

    let unused = keys
        .binders
        .first()
        .expect("the protocol has a declared binder")
        .node;
    let before = meter.clone();
    let failure = package.evaluate_protocol_slot(
        &slot,
        &ProtocolSupply {
            binders: vec![(unused, SuppliedValue::Value(Value::Boolean(true)))],
            observation: None,
        },
        &mut meter,
    );
    let Err(CallFailure::Admission(payload)) = failure else {
        panic!("an unread binder must be a typed admission failure: {failure:?}");
    };
    assert!(matches!(
        payload.as_ref(),
        CallAdmissionFailure::Refused(record)
            if record.code == CatalogCode::new("invalid_runtime_input", "unknown-member")
                && record.locus == CallAdmissionLocus::Binder(unused)
    ));
    assert_eq!(
        meter, before,
        "selection and supply admission charge nothing"
    );

    let duplicate = package.evaluate_protocol_slot(
        &slot,
        &ProtocolSupply {
            binders: vec![
                (unused, SuppliedValue::Value(Value::Boolean(true))),
                (unused, SuppliedValue::Value(Value::Boolean(false))),
            ],
            observation: None,
        },
        &mut meter,
    );
    let Err(CallFailure::Admission(payload)) = duplicate else {
        panic!("a repeated source key must be typed admission: {duplicate:?}");
    };
    assert!(matches!(
        payload.as_ref(),
        CallAdmissionFailure::Refused(record)
            if record.code == CatalogCode::new("invalid_runtime_input", "duplicate-member")
                && record.locus == CallAdmissionLocus::Binder(unused)
    ));

    for (invalid, expected_locus) in [
        (
            ProtocolSlot {
                node: quire_exact::NodeKey::from_digest([0xA5; 32]),
                member: "condition".to_owned(),
                index: None,
            },
            CallAdmissionLocus::Node(quire_exact::NodeKey::from_digest([0xA5; 32])),
        ),
        (
            ProtocolSlot {
                node: package
                    .graph()
                    .semantic_graph()
                    .nodes()
                    .find(|node| node.node_tag() != NodeTag::Protocol)
                    .expect("a non-protocol package node")
                    .key(),
                member: "condition".to_owned(),
                index: None,
            },
            CallAdmissionLocus::Node(
                package
                    .graph()
                    .semantic_graph()
                    .nodes()
                    .find(|node| node.node_tag() != NodeTag::Protocol)
                    .expect("a non-protocol package node")
                    .key(),
            ),
        ),
    ] {
        let result = package.evaluate_protocol_slot(
            &invalid,
            &ProtocolSupply {
                binders: Vec::new(),
                observation: None,
            },
            &mut meter,
        );
        let Err(CallFailure::Admission(payload)) = result else {
            panic!("invalid node selection must refuse: {result:?}");
        };
        assert!(matches!(
            payload.as_ref(),
            CallAdmissionFailure::Refused(record)
                if record.locus == expected_locus
        ));
    }

    let surplus_index = ProtocolSlot {
        node,
        member: "condition".to_owned(),
        index: Some(0),
    };
    let result = package.evaluate_protocol_slot(
        &surplus_index,
        &ProtocolSupply {
            binders: Vec::new(),
            observation: None,
        },
        &mut meter,
    );
    let Err(CallFailure::Admission(payload)) = result else {
        panic!("a single-valued member with an index refuses: {result:?}");
    };
    assert!(matches!(
        payload.as_ref(),
        CallAdmissionFailure::Refused(record)
            if record.code == CatalogCode::new("missing_declaration", "missing-name")
                && record.locus == CallAdmissionLocus::Member {
                    name: "condition".to_owned(), index: Some(0)
                }
    ));

    let activation = ProtocolSlot {
        node: keys.clause.expect("protocol clause key"),
        member: "activation.guard".to_owned(),
        index: None,
    };
    let opened = keys
        .binders
        .iter()
        .find(|binder| binder.name == "opened")
        .expect("activation binder identity")
        .node;
    let unavailable_cause = CatalogCode::new("incomplete_population", "incomplete-scope");
    let mut unavailable_meter = Meter::new(UNLIMITED);
    let unavailable = package
        .evaluate_protocol_slot(
            &activation,
            &ProtocolSupply {
                binders: vec![(opened, SuppliedValue::Unavailable(unavailable_cause))],
                observation: None,
            },
            &mut unavailable_meter,
        )
        .expect("unavailable is retained until its read");
    assert!(matches!(
        unavailable,
        ProtocolSlotEvaluation::Unavailable { binder, cause, .. }
            if binder == opened && cause == unavailable_cause
    ));
    assert!(
        !unavailable_meter.admitted_charges().is_empty(),
        "the arithmetic prefix was charged before the unavailable read"
    );
    let mut activation_meter = Meter::new(UNLIMITED);
    let missing = package.evaluate_protocol_slot(
        &activation,
        &ProtocolSupply {
            binders: Vec::new(),
            observation: None,
        },
        &mut activation_meter,
    );
    let Err(CallFailure::Admission(payload)) = missing else {
        panic!("the activation binder is required: {missing:?}");
    };
    assert!(matches!(
        payload.as_ref(),
        CallAdmissionFailure::Refused(record)
            if record.code == CatalogCode::new("invalid_runtime_input", "missing-member")
                && record.locus == CallAdmissionLocus::Binder(opened)
    ));

    let unknown_before_missing = package.evaluate_protocol_slot(
        &activation,
        &ProtocolSupply {
            binders: vec![(unused, SuppliedValue::Value(Value::Boolean(true)))],
            observation: None,
        },
        &mut activation_meter,
    );
    let Err(CallFailure::Admission(payload)) = unknown_before_missing else {
        panic!("unknown input precedes a missing source: {unknown_before_missing:?}");
    };
    assert!(matches!(
        payload.as_ref(),
        CallAdmissionFailure::Refused(record)
            if record.code == CatalogCode::new("invalid_runtime_input", "unknown-member")
                && record.locus == CallAdmissionLocus::Binder(unused)
    ));

    let wrong_kind = package.evaluate_protocol_slot(
        &activation,
        &ProtocolSupply {
            binders: vec![(opened, SuppliedValue::Value(Value::Integer(1_i64.into())))],
            observation: None,
        },
        &mut activation_meter,
    );
    let Err(CallFailure::Admission(payload)) = wrong_kind else {
        panic!("wrong binder shape must be refused: {wrong_kind:?}");
    };
    assert!(matches!(
        payload.as_ref(),
        CallAdmissionFailure::Refused(record)
            if record.code == CatalogCode::new("invalid_runtime_input", "wrong-value-kind")
                && record.locus == CallAdmissionLocus::Binder(opened)
    ));

    let admitted = package
        .evaluate_protocol_slot(
            &activation,
            &ProtocolSupply {
                binders: vec![(opened, SuppliedValue::Value(Value::Boolean(true)))],
                observation: None,
            },
            &mut activation_meter,
        )
        .expect("the exact activation binder evaluates");
    let ProtocolSlotEvaluation::Evaluated(evaluation) = admitted else {
        panic!("the supplied activation value was unavailable: {admitted:?}");
    };
    assert!(matches!(
        evaluation.outcome,
        FamilyOutcome::Evaluated(quire_exact::Outcome::Completed(Value::Boolean(true)))
    ));
}

#[test]
#[trace("TC-1072", "FR-390-AC-7")]
fn an_unavailable_protocol_binder_is_not_observed_after_a_false_short_circuit() {
    let source = unit().replace("(1 + 2 = 3) and opened", "(1 + 2 = 4) and opened");
    let package = package_from_source(&source);
    let keys = package
        .graph()
        .protocol_node_keys("P")
        .expect("the protocol's emitted identity map");
    let slot = ProtocolSlot {
        node: keys.clause.expect("protocol clause key"),
        member: "activation.guard".to_owned(),
        index: None,
    };
    let opened = keys
        .binders
        .iter()
        .find(|binder| binder.name == "opened")
        .expect("activation binder identity")
        .node;
    let mut meter = Meter::new(UNLIMITED);
    let result = package
        .evaluate_protocol_slot(
            &slot,
            &ProtocolSupply {
                binders: vec![(
                    opened,
                    SuppliedValue::Unavailable(CatalogCode::new(
                        "incomplete_population",
                        "incomplete-scope",
                    )),
                )],
                observation: None,
            },
            &mut meter,
        )
        .expect("the decisive false prefix completes without reading opened");
    assert!(matches!(
        result,
        ProtocolSlotEvaluation::Evaluated(quire_eval::Evaluation {
            outcome: FamilyOutcome::Evaluated(quire_exact::Outcome::Completed(Value::Boolean(
                false
            ))),
            ..
        })
    ));
}

#[test]
// Covers only the capture, trigger, retry and recovery rows of the
// compensation matrix (rows 2-4, 6, 8 and 12); the remaining rows are not
// asserted here, so the whole matrix is not tagged.
#[trace("TC-142", "FR-049-AC-9", "FR-390-AC-1", "FR-390-AC-2")]
fn evaluates_compensation_capture_trigger_retry_and_recovery_from_checked_nodes() {
    let package = compensation_package();
    let keys = package
        .graph()
        .protocol_node_keys("P")
        .expect("the protocol's emitted identity map");
    let binder = |name: &str| {
        keys.binders
            .iter()
            .find(|binder| binder.name == name)
            .unwrap_or_else(|| panic!("the protocol emits binder {name}"))
            .node
    };
    let mut meter = Meter::new(UNLIMITED);
    let saved = match package
        .evaluate_protocol_slot(
            &ProtocolSlot {
                node: keys.compensations[0],
                member: "registration_captures".to_owned(),
                index: Some(0),
            },
            &ProtocolSupply {
                binders: vec![(binder("fwd"), SuppliedValue::Value(money(&package, 5)))],
                observation: None,
            },
            &mut meter,
        )
        .expect("the registration capture initializer evaluates")
    {
        ProtocolSlotEvaluation::Evaluated(quire_eval::Evaluation {
            outcome: FamilyOutcome::Evaluated(quire_exact::Outcome::Completed(value)),
            ..
        }) => value,
        other => panic!("the capture returns its frozen value: {other:?}"),
    };
    assert_eq!(
        quire_exact::compare_keys(&saved, &money(&package, 5)),
        Some(std::cmp::Ordering::Equal)
    );

    let trigger = package
        .evaluate_protocol_slot(
            &ProtocolSlot {
                node: keys.compensations[0],
                member: "trigger.guard".to_owned(),
                index: None,
            },
            &ProtocolSupply {
                binders: vec![
                    (binder("trigger"), SuppliedValue::Value(money(&package, 3))),
                    (binder("saved"), SuppliedValue::Value(saved.clone())),
                ],
                observation: None,
            },
            &mut meter,
        )
        .expect("the trigger guard evaluates over its exact sources");
    assert!(matches!(
        trigger,
        ProtocolSlotEvaluation::Evaluated(quire_eval::Evaluation {
            outcome: FamilyOutcome::Evaluated(quire_exact::Outcome::Completed(Value::Boolean(
                true
            ))),
            ..
        })
    ));

    let activation = package
        .evaluate_protocol_slot(
            &ProtocolSlot {
                node: keys.compensations[0],
                member: "activation_captures".to_owned(),
                index: Some(0),
            },
            &ProtocolSupply {
                binders: vec![(binder("trigger"), SuppliedValue::Value(money(&package, 3)))],
                observation: None,
            },
            &mut meter,
        )
        .expect("the activation capture evaluates");
    assert!(matches!(
        activation,
        ProtocolSlotEvaluation::Evaluated(quire_eval::Evaluation {
            outcome: FamilyOutcome::Evaluated(quire_exact::Outcome::Completed(Value::Boolean(
                true
            ))),
            ..
        })
    ));

    let retry = package
        .evaluate_protocol_slot(
            &ProtocolSlot {
                node: keys.compensations[0],
                member: "retry.relation".to_owned(),
                index: None,
            },
            &ProtocolSupply {
                binders: vec![
                    (binder("earlier"), SuppliedValue::Value(money(&package, 1))),
                    (binder("later"), SuppliedValue::Value(money(&package, 2))),
                    (binder("saved"), SuppliedValue::Value(saved.clone())),
                    (
                        binder("activated"),
                        SuppliedValue::Value(Value::Boolean(false)),
                    ),
                ],
                observation: None,
            },
            &mut meter,
        )
        .expect("the retry relation evaluates from supplied attempt and capture values");
    assert!(matches!(
        retry,
        ProtocolSlotEvaluation::Evaluated(quire_eval::Evaluation {
            outcome: FamilyOutcome::Evaluated(quire_exact::Outcome::Completed(Value::Boolean(
                true
            ))),
            ..
        })
    ));

    let recovery = package
        .evaluate_protocol_slot(
            &ProtocolSlot {
                node: keys.compensations[0],
                member: "recovery.predicate".to_owned(),
                index: None,
            },
            &ProtocolSupply {
                binders: vec![
                    (
                        binder("recovered"),
                        SuppliedValue::Value(money(&package, 5)),
                    ),
                    (binder("saved"), SuppliedValue::Value(saved)),
                    (
                        binder("activated"),
                        SuppliedValue::Value(Value::Boolean(false)),
                    ),
                ],
                observation: None,
            },
            &mut meter,
        )
        .expect("the recovery predicate evaluates from its supplied capture");
    assert!(matches!(
        recovery,
        ProtocolSlotEvaluation::Evaluated(quire_eval::Evaluation {
            outcome: FamilyOutcome::Evaluated(quire_exact::Outcome::Completed(Value::Boolean(
                true
            ))),
            ..
        })
    ));
}

#[trace("TC-1073", "FR-390-AC-9", "FR-390-AC-10", "FR-049-AC-6")]
// Exercises the `work_units` counter only: exact, one-short and zero budgets,
// fresh-meter retry and shared-meter accumulation. Per-counter accounting
// coverage and the deep-stack cases are not asserted here.
#[trace("FR-049-AC-8", "NFR-009-AC-4")]
#[test]
fn protocol_slot_calls_share_meter_usage_and_obey_work_boundaries() {
    let package = package();
    let keys = package
        .graph()
        .protocol_node_keys("P")
        .expect("the protocol's emitted identity map");
    let opened = keys
        .binders
        .iter()
        .find(|binder| binder.name == "opened")
        .expect("activation binder identity")
        .node;
    let slot = ProtocolSlot {
        node: keys.clause.expect("protocol clause key"),
        member: "activation.guard".to_owned(),
        index: None,
    };
    let supply = ProtocolSupply {
        binders: vec![(opened, SuppliedValue::Value(Value::Boolean(true)))],
        observation: None,
    };

    let mut measured = Meter::new(UNLIMITED);
    let measured_result = package
        .evaluate_protocol_slot(&slot, &supply, &mut measured)
        .expect("the measured call completes");
    assert!(matches!(
        measured_result,
        ProtocolSlotEvaluation::Evaluated(quire_eval::Evaluation {
            outcome: FamilyOutcome::Evaluated(quire_exact::Outcome::Completed(Value::Boolean(
                true
            ))),
            ..
        })
    ));
    let used = measured.consumed(quire_exact::LimitKind::WorkUnits);
    assert!(used > 0, "the arithmetic expression consumed work");

    let mut exact_limits = UNLIMITED;
    exact_limits.work_units = used;
    let mut exact = Meter::new(exact_limits);
    let exact_result = package
        .evaluate_protocol_slot(&slot, &supply, &mut exact)
        .expect("exact measured usage completes");
    assert!(matches!(
        exact_result,
        ProtocolSlotEvaluation::Evaluated(quire_eval::Evaluation {
            outcome: FamilyOutcome::Evaluated(quire_exact::Outcome::Completed(Value::Boolean(
                true
            ))),
            ..
        })
    ));
    assert_eq!(exact.consumed(quire_exact::LimitKind::WorkUnits), used);

    let mut short_limits = UNLIMITED;
    short_limits.work_units = used - 1;
    let mut short = Meter::new(short_limits);
    let stopped = package
        .evaluate_protocol_slot(&slot, &supply, &mut short)
        .expect("an exhausted evaluator returns its typed outcome");
    assert!(matches!(
        stopped,
        ProtocolSlotEvaluation::Evaluated(quire_eval::Evaluation {
            outcome: FamilyOutcome::Evaluated(quire_exact::Outcome::Incomplete(ref record)),
            ..
        }) if record.limit_kind == quire_exact::LimitKind::WorkUnits
    ));
    assert_eq!(
        short.consumed(quire_exact::LimitKind::WorkUnits),
        used - 1,
        "the denied next charge does not advance the meter"
    );

    let mut zero_limits = UNLIMITED;
    zero_limits.work_units = 0;
    let mut zero = Meter::new(zero_limits);
    let stopped = package
        .evaluate_protocol_slot(&slot, &supply, &mut zero)
        .expect("zero budget is a typed incomplete outcome");
    assert!(matches!(
        stopped,
        ProtocolSlotEvaluation::Evaluated(quire_eval::Evaluation {
            outcome: FamilyOutcome::Evaluated(quire_exact::Outcome::Incomplete(ref record)),
            ..
        }) if record.limit_kind == quire_exact::LimitKind::WorkUnits
    ));
    assert_eq!(zero.consumed(quire_exact::LimitKind::WorkUnits), 0);

    let mut retry = Meter::new(UNLIMITED);
    let retried = package
        .evaluate_protocol_slot(&slot, &supply, &mut retry)
        .expect("a fresh sufficient meter completes the same immutable call");
    assert!(matches!(
        retried,
        ProtocolSlotEvaluation::Evaluated(quire_eval::Evaluation {
            outcome: FamilyOutcome::Evaluated(quire_exact::Outcome::Completed(Value::Boolean(
                true
            ))),
            ..
        })
    ));
    assert_eq!(
        retry.consumed(quire_exact::LimitKind::WorkUnits),
        used,
        "fresh-meter retry reproduces the measured usage"
    );

    let mut shared = Meter::new(UNLIMITED);
    package
        .evaluate_protocol_slot(&slot, &supply, &mut shared)
        .expect("first shared-metre call completes");
    package
        .evaluate_protocol_slot(&slot, &supply, &mut shared)
        .expect("second shared-metre call completes");
    assert_eq!(
        shared.consumed(quire_exact::LimitKind::WorkUnits),
        used * 2,
        "the caller's one meter accumulates both calls"
    );
}
