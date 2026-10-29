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

const UNIT: &str = "language \"ix:native\" edition \"1-draft\";\n\
    profile v = \"quire.value.complete/v1\";\n\
    profile p = \"quire.protocol.finite-global/v1\";\n\
    model M = \"acme/orders\" version \"1.0.0\" digest \
    \"sha256-jcs:876d1b2d9e3f825763f6e301cca5c6d6462f4cc0daacb501f7a68a8b3089e114\";\n\
    protocol P using v over (input: M::Gadget) on each (opened: Boolean) when ((1 + 2 = 3) and opened) {\n\
      role R on M::Gadget;\n\
      run sequence Main { check Ready using p { true }; }\n\
      finish End as (outcome: Boolean) { true };\n\
    }\n";

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
    let parsed = qsl_cst::parse(
        qsl_foundation::SourceIdentity::new("a", "u", "git", "1"),
        "unit.native",
        UNIT.as_bytes(),
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

#[test]
#[trace("TC-1071", "FR-390-AC-3", "FR-390-AC-4", "FR-390-AC-5")]
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

#[trace("TC-1073", "FR-390-AC-9", "FR-390-AC-10")]
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
    package
        .evaluate_protocol_slot(&slot, &supply, &mut measured)
        .expect("the measured call completes");
    let used = measured.consumed(quire_exact::LimitKind::WorkUnits);
    assert!(used > 0, "the arithmetic expression consumed work");

    let mut exact_limits = UNLIMITED;
    exact_limits.work_units = used;
    let mut exact = Meter::new(exact_limits);
    package
        .evaluate_protocol_slot(&slot, &supply, &mut exact)
        .expect("exact measured usage completes");
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
